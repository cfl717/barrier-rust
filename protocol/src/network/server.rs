//! Barrier Server Implementation
//! 
//! This module implements the Barrier server that accepts client connections.

use crate::protocol::{server_handshake, HandshakeResult, Message, ProtocolResult};
use crate::network::Connection;
use crate::protocol::events::ClientEnterEvent;
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
        
        (
            Self {
                config,
                clients: Arc::new(Mutex::new(HashMap::new())),
                next_client_id: 1,
                event_tx: tx,
            },
            rx,
        )
    }

    /// Start the server and accept connections
    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let addr = format!("0.0.0.0:{}", self.config.port);
        let listener = TcpListener::bind(&addr).await?;
        
        info!("Barrier server listening on {}", addr);
        
        loop {
            let (stream, peer_addr) = listener.accept().await?;
            info!("New connection from {}", peer_addr);
            
            let client_id = self.next_client_id;
            self.next_client_id += 1;
            
            let connection = Connection::new(stream, client_id);
            let clients = Arc::clone(&self.clients);
            let event_tx = self.event_tx.clone();
            let config = self.config.clone();
            
            tokio::spawn(async move {
                if let Err(e) = handle_client(connection, peer_addr.to_string(), client_id, clients, event_tx, config).await {
                    error!("Error handling client {}: {}", client_id, e);
                }
            });
        }
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

    /// Switch input focus to a specific client by screen name.
    pub async fn switch_to_client(&self, screen_name: &str) -> Result<(), String> {
        let writer = {
            let clients = self.clients.lock().await;
            clients
                .get(screen_name)
                .map(|session| Arc::clone(&session.writer))
                .ok_or_else(|| format!("客户端 `{}` 未连接", screen_name))?
        };

        let leave_msg = Message::empty(message_types::CLIENT_LEAVE);
        let enter_msg = ClientEnterEvent::new(1, 0, 0, 0).to_message();

        let leave_bytes = leave_msg
            .serialize()
            .map_err(|e| format!("序列化 COUT 失败: {}", e))?;
        let enter_bytes = enter_msg
            .serialize()
            .map_err(|e| format!("序列化 CINN 失败: {}", e))?;

        let mut writer = writer.lock().await;
        writer
            .write_all(&leave_bytes)
            .await
            .map_err(|e| format!("发送 COUT 失败: {}", e))?;
        writer
            .write_all(&enter_bytes)
            .await
            .map_err(|e| format!("发送 CINN 失败: {}", e))?;
        writer
            .flush()
            .await
            .map_err(|e| format!("刷新切换消息失败: {}", e))?;

        Ok(())
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
