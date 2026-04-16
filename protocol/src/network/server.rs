//! Barrier Server Implementation
//! 
//! This module implements the Barrier server that accepts client connections.

use crate::protocol::{server_handshake, HandshakeResult, Message, ProtocolResult};
use crate::network::Connection;
use crate::protocol::events::{ClientEnterEvent, KeyEvent, ModifierKeys, MouseButton, MouseButtonEvent, MouseMoveEvent};
use crate::protocol::message::message_types;
use log::{error, info, warn};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::{mpsc, Mutex};

/// Default Barrier server port
pub const BARRIER_PORT: u16 = 24800;

/// Server configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ServerConfig {
    /// Server screen name
    #[serde(default = "default_screen_name")]
    pub screen_name: String,
    /// Port to listen on
    #[serde(default = "default_port")]
    pub port: u16,
    /// Maximum number of connected clients
    #[serde(default = "default_max_clients")]
    pub max_clients: usize,
    /// Listen address (for frontend compatibility, maps to port)
    #[serde(default, alias = "address", skip_serializing_if = "Option::is_none")]
    pub listen_address: Option<String>,
    /// Enable clipboard sharing
    #[serde(default = "default_true")]
    pub enable_clipboard: bool,
    /// Enable file drag-drop
    #[serde(default = "default_true")]
    pub enable_drag_drop: bool,
}

fn default_screen_name() -> String {
    "server".to_string()
}

fn default_port() -> u16 {
    BARRIER_PORT
}

fn default_max_clients() -> usize {
    10
}

fn default_true() -> bool {
    true
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            screen_name: default_screen_name(),
            port: default_port(),
            max_clients: default_max_clients(),
            listen_address: None,
            enable_clipboard: default_true(),
            enable_drag_drop: default_true(),
        }
    }
}

/// Connected client information
#[derive(Debug, Clone)]
pub struct ClientInfo {
    /// Connection ID
    pub id: u64,
    /// Screen name
    pub screen_name: String,
    /// Client address
    pub address: String,
}

/// Barrier server state
pub struct BarrierServer {
    config: ServerConfig,
    clients: Arc<Mutex<HashMap<String, ClientSession>>>,
    next_client_id: u64,
    event_tx: mpsc::Sender<ServerEvent>,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

/// 独立的客户端计数器，可在不锁 BarrierServer 的情况下查询
#[derive(Clone)]
pub struct ClientsHandle {
    clients: Arc<Mutex<HashMap<String, ClientSession>>>,
}

impl ClientsHandle {
    pub async fn count(&self) -> usize {
        self.clients.lock().await.len()
    }

    pub async fn names(&self) -> Vec<String> {
        self.clients.lock().await.keys().cloned().collect()
    }
}

struct ClientSession {
    info: ClientInfo,
    writer: Arc<Mutex<OwnedWriteHalf>>,
}

/// Server events
#[derive(Debug, Clone)]
pub enum ServerEvent {
    /// Client connected
    ClientConnected(ClientInfo),
    /// Client disconnected
    ClientDisconnected(u64),
    /// Message received from client
    MessageReceived { client_id: u64, message: Message },
    /// Handshake completed
    HandshakeCompleted { client_id: u64, result: HandshakeResult },
    /// Error occurred
    Error(String),
}

impl BarrierServer {
    /// Create a new server instance
    pub fn new(mut config: ServerConfig) -> (Self, mpsc::Receiver<ServerEvent>) {
        // Parse listen_address if provided to extract port
        if let Some(ref addr) = config.listen_address {
            if let Some(port_str) = addr.split(':').last() {
                if let Ok(port) = port_str.parse::<u16>() {
                    config.port = port;
                }
            }
        }
        
        let (tx, rx) = mpsc::channel(100);
        let clients = Arc::new(Mutex::new(HashMap::new()));
        
        (
            Self {
                config,
                clients,
                next_client_id: 1,
                event_tx: tx,
                shutdown_tx: None,
            },
            rx,
        )
    }

    /// 停止服务器
    pub fn shutdown(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }

    /// 获取独立的客户端句柄，可在不锁 server 的情况下查询客户端数量
    pub fn clients_handle(&self) -> ClientsHandle {
        ClientsHandle {
            clients: Arc::clone(&self.clients),
        }
    }

    /// Start the server and accept connections
    /// 注意：此方法会启动后台任务来接受连接，然后立即返回
    /// 这样 BarrierServer 不会被长期锁定，其他方法（如 send_enter）可以正常调用
    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let addr = format!("0.0.0.0:{}", self.config.port);
        let listener = TcpListener::bind(&addr).await?;
        
        info!("Barrier server listening on {}", addr);
        
        let clients = Arc::clone(&self.clients);
        let event_tx = self.event_tx.clone();
        let config = self.config.clone();
        
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        self.shutdown_tx = Some(shutdown_tx);
        
        // 在后台任务中运行 accept 循环，这样 run() 可以立即返回
        tokio::spawn(async move {
            let mut next_client_id: u64 = 1;
            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => {
                        info!("Server shutdown requested");
                        break;
                    }
                    result = listener.accept() => {
                        match result {
                            Ok((stream, peer_addr)) => {
                                info!("New connection from {}", peer_addr);
                                
                                let client_id = next_client_id;
                                next_client_id += 1;
                                
                                let connection = Connection::new(stream, client_id);
                                let clients = Arc::clone(&clients);
                                let event_tx = event_tx.clone();
                                let config = config.clone();
                                
                                tokio::spawn(async move {
                                    if let Err(e) = handle_client(connection, peer_addr.to_string(), client_id, clients, event_tx, config).await {
                                        error!("Error handling client {}: {}", client_id, e);
                                    }
                                });
                            }
                            Err(e) => {
                                error!("Accept error: {}", e);
                                break;
                            }
                        }
                    }
                }
            }
            info!("Server stopped");
        });
        
        Ok(())
    }

    /// Get list of connected clients
    pub async fn get_clients(&self) -> Vec<ClientInfo> {
        self.clients
            .lock()
            .await
            .values()
            .map(|session| session.info.clone())
            .collect()
    }

    /// Get client count
    pub async fn client_count(&self) -> usize {
        self.clients.lock().await.len()
    }

    /// Send CLIENT_ENTER to one client.
    pub async fn send_enter(&self, screen_name: &str, x: i16, y: i16) -> Result<(), String> {
        let writer = self.get_client_writer(screen_name).await?;
        let enter_msg = ClientEnterEvent::new(1, x, y, 0).to_message();
        let enter_bytes = enter_msg
            .serialize()
            .map_err(|e| format!("序列化 CINN 失败: {}", e))?;
        send_serialized_messages(&writer, &[enter_bytes]).await
    }

    /// Send CLIENT_LEAVE to one client.
    pub async fn send_leave(&self, screen_name: &str) -> Result<(), String> {
        let writer = self.get_client_writer(screen_name).await?;
        let leave_msg = Message::empty(message_types::CLIENT_LEAVE);
        let leave_bytes = leave_msg
            .serialize()
            .map_err(|e| format!("序列化 COUT 失败: {}", e))?;
        send_serialized_messages(&writer, &[leave_bytes]).await
    }

    pub async fn relay_mouse_move(
        &self,
        screen_name: &str,
        dx: i16,
        dy: i16,
    ) -> Result<(), String> {
        let writer = self.get_client_writer(screen_name).await?;
        let msg = MouseMoveEvent::new(dx, dy, ModifierKeys::NONE).to_message();
        let bytes = msg
            .serialize()
            .map_err(|e| format!("序列化 CMOV 失败: {}", e))?;
        send_serialized_messages(&writer, &[bytes]).await
    }

    pub async fn relay_key_event(
        &self,
        screen_name: &str,
        key_code: u16,
        pressed: bool,
    ) -> Result<(), String> {
        let writer = self.get_client_writer(screen_name).await?;
        let event = KeyEvent::new(key_code, pressed, ModifierKeys::NONE);
        let msg = if pressed {
            event.to_key_down_message()
        } else {
            event.to_key_up_message()
        };
        let bytes = msg
            .serialize()
            .map_err(|e| format!("序列化键盘事件失败: {}", e))?;
        send_serialized_messages(&writer, &[bytes]).await
    }

    pub async fn relay_mouse_button(
        &self,
        screen_name: &str,
        button: u8,
        pressed: bool,
    ) -> Result<(), String> {
        let writer = self.get_client_writer(screen_name).await?;
        let mapped_button = match button {
            1 => MouseButton::Left,
            2 => MouseButton::Right,
            3 => MouseButton::Middle,
            _ => MouseButton::None,
        };
        let msg = MouseButtonEvent::new(mapped_button, pressed, ModifierKeys::NONE).to_message();
        let bytes = msg
            .serialize()
            .map_err(|e| format!("序列化鼠标按键事件失败: {}", e))?;
        send_serialized_messages(&writer, &[bytes]).await
    }

    async fn get_client_writer(
        &self,
        screen_name: &str,
    ) -> Result<Arc<Mutex<OwnedWriteHalf>>, String> {
        let clients = self.clients.lock().await;
        clients
            .get(screen_name)
            .map(|session| Arc::clone(&session.writer))
            .ok_or_else(|| format!("客户端 `{}` 未连接", screen_name))
    }
}

/// Handle a single client connection
async fn handle_client(
    connection: Connection,
    peer_addr: String,
    client_id: u64,
    clients: Arc<Mutex<HashMap<String, ClientSession>>>,
    event_tx: mpsc::Sender<ServerEvent>,
    _config: ServerConfig,
) -> ProtocolResult<()> {
    let (mut read_half, mut write_half) = connection.split();
    
    // Perform handshake
    let handshake_result = match server_handshake(&mut write_half, &mut read_half).await {
        Ok(result) => {
            info!("Handshake completed with client {}: {:?}", client_id, result);
            
            // Notify about successful handshake
            let _ = event_tx
                .send(ServerEvent::HandshakeCompleted {
                    client_id,
                    result: result.clone(),
                })
                .await;
            
            result
        }
        Err(e) => {
            warn!("Handshake failed with client {}: {}", client_id, e);
            return Err(e);
        }
    };
    
    // Register client
    let client_info = ClientInfo {
        id: client_id,
        screen_name: handshake_result.screen_name.unwrap_or_else(|| format!("client-{}", client_id)),
        address: peer_addr,
    };
    
    let screen_name = client_info.screen_name.clone();
    let writer = Arc::new(Mutex::new(write_half));
    clients.lock().await.insert(
        screen_name.clone(),
        ClientSession {
            info: client_info.clone(),
            writer,
        },
    );
    
    // Notify about new client
    let _ = event_tx.send(ServerEvent::ClientConnected(client_info)).await;
    
    info!("Client {} registered", client_id);

    wait_for_disconnect(&mut read_half).await;
    clients.lock().await.remove(&screen_name);
    let _ = event_tx.send(ServerEvent::ClientDisconnected(client_id)).await;
    info!("Client {} disconnected", client_id);

    Ok(())
}

async fn wait_for_disconnect(read_half: &mut OwnedReadHalf) {
    let mut buf = [0u8; 1024];
    loop {
        match read_half.read(&mut buf).await {
            Ok(0) => break,
            Ok(_) => continue,
            Err(_) => break,
        }
    }
}

async fn send_serialized_messages(
    writer: &Arc<Mutex<OwnedWriteHalf>>,
    payloads: &[Vec<u8>],
) -> Result<(), String> {
    let mut writer = writer.lock().await;
    for payload in payloads {
        writer
            .write_all(payload)
            .await
            .map_err(|e| format!("发送消息失败: {}", e))?;
    }
    writer
        .flush()
        .await
        .map_err(|e| format!("刷新消息失败: {}", e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_server_config_default() {
        let config = ServerConfig::default();
        assert_eq!(config.port, BARRIER_PORT);
        assert_eq!(config.screen_name, "server");
        assert_eq!(config.max_clients, 10);
    }

    #[tokio::test]
    async fn test_server_creation() {
        let config = ServerConfig::default();
        let (server, _rx) = BarrierServer::new(config);
        
        assert_eq!(server.client_count().await, 0);
    }
}
