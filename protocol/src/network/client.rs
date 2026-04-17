//! Barrier Client Implementation
//! 
//! This module implements the Barrier client that connects to a server.

use crate::protocol::{client_handshake, HandshakeResult, Message, ProtocolResult};
use crate::protocol::message::message_types;
use crate::protocol::events::{KeyEvent, MouseButtonEvent, MouseMoveEvent};
use crate::network::Connection;
use crate::platform::get_platform;
use log::{error, info, warn};
use tokio::net::TcpStream;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc::UnboundedSender;

/// Client configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClientConfig {
    /// Server address (hostname:port or IP:port)
    #[serde(default = "default_server_addr", alias = "server_address")]
    pub server_addr: String,
    /// Client screen name
    #[serde(default = "default_client_screen_name", alias = "client_name")]
    pub screen_name: String,
    /// Auto-reconnect on disconnect
    #[serde(default = "default_true")]
    pub auto_reconnect: bool,
    /// Reconnect interval in seconds
    #[serde(default = "default_reconnect_interval")]
    pub reconnect_interval: u64,
    /// Enable clipboard sharing
    #[serde(default = "default_true")]
    pub enable_clipboard: bool,
    /// Enable file drag-drop
    #[serde(default = "default_true")]
    pub enable_drag_drop: bool,
}

fn default_server_addr() -> String {
    "localhost:24800".to_string()
}

fn default_client_screen_name() -> String {
    "client".to_string()
}

fn default_reconnect_interval() -> u64 {
    5
}

fn default_true() -> bool {
    true
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            server_addr: default_server_addr(),
            screen_name: default_client_screen_name(),
            auto_reconnect: default_true(),
            reconnect_interval: default_reconnect_interval(),
            enable_clipboard: default_true(),
            enable_drag_drop: default_true(),
        }
    }
}

/// Client state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientState {
    Disconnected,
    Connecting,
    Connected,
    Authenticating,
    Ready,
    Error(String),
}

#[derive(Debug, Clone)]
pub enum ClientRuntimeEvent {
    Log(String),
}

/// Barrier client
pub struct BarrierClient {
    config: ClientConfig,
    state: ClientState,
    connection: Option<Connection>,
    read_half: Option<OwnedReadHalf>,
    write_half: Option<OwnedWriteHalf>,
    handshake_result: Option<HandshakeResult>,
    event_tx: Option<UnboundedSender<ClientRuntimeEvent>>,
}

impl BarrierClient {
    /// Create a new client instance
    pub fn new(config: ClientConfig) -> Self {
        Self {
            config,
            state: ClientState::Disconnected,
            connection: None,
            read_half: None,
            write_half: None,
            handshake_result: None,
            event_tx: None,
        }
    }

    pub fn set_event_sender(&mut self, sender: UnboundedSender<ClientRuntimeEvent>) {
        self.event_tx = Some(sender);
    }

    /// Get current state
    pub fn state(&self) -> ClientState {
        self.state.clone()
    }

    /// Check if connected and ready
    pub fn is_ready(&self) -> bool {
        self.state == ClientState::Ready
    }

    /// Connect to the server
    pub async fn connect(&mut self) -> ProtocolResult<()> {
        self.state = ClientState::Connecting;
        
        info!("Connecting to server at {}", self.config.server_addr);
        self.emit_runtime_log(format!("正在连接服务端 {}", self.config.server_addr));
        
        // Connect to server
        let stream = match TcpStream::connect(&self.config.server_addr).await {
            Ok(stream) => stream,
            Err(e) => {
                self.state = ClientState::Error(format!("Connection failed: {}", e));
                self.emit_runtime_log(format!("连接失败: {}", e));
                return Err(ProtocolError::Io(e));
            }
        };
        
        info!("TCP connection established");
        self.emit_runtime_log("TCP 连接已建立");
        
        // Perform handshake on owned halves, and keep them after handshake.
        self.state = ClientState::Authenticating;
        self.emit_runtime_log("开始握手（CBYQ/QBYC）");
        let (mut read_half, mut write_half) = stream.into_split();
        
        let handshake_result = match client_handshake(
            &mut write_half,
            &mut read_half,
            &self.config.screen_name,
        ).await {
            Ok(result) => result,
            Err(e) => {
                self.state = ClientState::Error(format!("Handshake failed: {}", e));
                self.emit_runtime_log(format!("握手失败: {}", e));
                return Err(e);
            }
        };
        
        info!("Handshake completed: {:?}", handshake_result);
        self.emit_runtime_log(format!(
            "握手成功: app={} screen={}",
            handshake_result.app_name,
            handshake_result
                .screen_name
                .clone()
                .unwrap_or_else(|| "-".to_string())
        ));
        
        // Recreate connection from stream (need to handle this better)
        // Keep halves alive so the TCP session remains connected.
        self.read_half = Some(read_half);
        self.write_half = Some(write_half);
        self.handshake_result = Some(handshake_result);
        self.state = ClientState::Ready;
        
        Ok(())
    }

    /// Disconnect from server
    pub async fn disconnect(&mut self) {
        self.connection = None;
        self.read_half = None;
        self.write_half = None;
        self.handshake_result = None;
        self.state = ClientState::Disconnected;
        info!("Disconnected from server");
    }

    /// Send a message to the server
    pub async fn send(&mut self, message: &Message) -> ProtocolResult<()> {
        if let Some(writer) = &mut self.write_half {
            let bytes = message.serialize()?;
            writer.write_all(&bytes).await?;
            writer.flush().await?;
            Ok(())
        } else {
            Err(ProtocolError::Io(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "Not connected to server",
            )))
        }
    }

    /// Receive a message from the server
    pub async fn receive(&mut self) -> ProtocolResult<Message> {
        if let Some(reader) = &mut self.read_half {
            // Read size (4 bytes)
            let mut size_buf = [0u8; 4];
            reader.read_exact(&mut size_buf).await?;
            let size = u32::from_be_bytes(size_buf) as usize;

            // Read type (4 bytes)
            let mut type_buf = [0u8; 4];
            reader.read_exact(&mut type_buf).await?;

            // Calculate data length
            let data_len = size.saturating_sub(8);
            let mut data = vec![0u8; data_len];
            if data_len > 0 {
                reader.read_exact(&mut data).await?;
            }

            // Read checksum (4 bytes)
            let mut checksum_buf = [0u8; 4];
            reader.read_exact(&mut checksum_buf).await?;
            let stored_checksum = u32::from_be_bytes(checksum_buf);
            let calculated_checksum = data.iter().fold(0u32, |acc, &b| acc ^ (b as u32));
            if stored_checksum != calculated_checksum {
                return Err(ProtocolError::ChecksumMismatch);
            }

            let msg_type = crate::protocol::MessageType::from_bytes(type_buf);
            Ok(Message { msg_type, data })
        } else {
            Err(ProtocolError::Io(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "Not connected to server",
            )))
        }
    }

    /// Get handshake result
    pub fn handshake_result(&self) -> Option<&HandshakeResult> {
        self.handshake_result.as_ref()
    }

    /// Run the client with auto-reconnect
    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        loop {
            match self.connect().await {
                Ok(_) => {
                    info!("Successfully connected to server");
                    self.emit_runtime_log("已连接服务端，等待输入焦点切换消息");
                    let mut input_enabled = false;
                    let mut ignored_input_packets: u64 = 0;
                    let mut injected_packets: u64 = 0;

                    // Basic message processing loop for screen-switch signals.
                    loop {
                        match self.receive().await {
                            Ok(msg) => {
                                if msg.msg_type == message_types::CLIENT_ENTER {
                                    input_enabled = true;
                                    injected_packets = 0;
                                    info!("Received CINN: input focus entered this client");
                                    self.emit_runtime_log("收到 CINN：输入焦点已切换到当前客户端");
                                } else if msg.msg_type == message_types::CLIENT_LEAVE {
                                    input_enabled = false;
                                    info!("Received COUT: input focus left this client");
                                    self.emit_runtime_log("收到 COUT：输入焦点已切回服务端");
                                } else if !input_enabled
                                    && (msg.msg_type == message_types::MOUSE_MOVE
                                        || msg.msg_type == message_types::MOUSE_BUTTON
                                        || msg.msg_type == message_types::KEY_DOWN
                                        || msg.msg_type == message_types::KEY_UP)
                                {
                                    ignored_input_packets += 1;
                                    if ignored_input_packets <= 5 || ignored_input_packets % 100 == 0 {
                                        self.emit_runtime_log(format!(
                                            "未激活状态，忽略输入消息 {}（累计 {}）",
                                            msg.msg_type, ignored_input_packets
                                        ));
                                    }
                                } else if input_enabled && msg.msg_type == message_types::MOUSE_MOVE {
                                    if let Ok(event) = MouseMoveEvent::from_message(&msg) {
                                        if let Err(e) = inject_mouse_move(event.dx, event.dy) {
                                            warn!("Inject mouse move failed: {}", e);
                                            self.emit_runtime_log(format!("鼠标移动注入失败: {}", e));
                                        } else {
                                            injected_packets += 1;
                                            if injected_packets <= 5 || injected_packets % 50 == 0 {
                                                self.emit_runtime_log(format!(
                                                    "已注入鼠标移动（累计 {}）",
                                                    injected_packets
                                                ));
                                            }
                                        }
                                    }
                                } else if input_enabled
                                    && (msg.msg_type == message_types::KEY_DOWN
                                        || msg.msg_type == message_types::KEY_UP)
                                {
                                    if let Ok(event) = KeyEvent::from_message(&msg) {
                                        if let Err(e) = inject_key(event.key_code, event.pressed) {
                                            warn!("Inject key event failed: {}", e);
                                            self.emit_runtime_log(format!("键盘注入失败: {}", e));
                                        } else {
                                            injected_packets += 1;
                                            if injected_packets <= 5 || injected_packets % 50 == 0 {
                                                self.emit_runtime_log(format!(
                                                    "已注入键盘事件（累计 {}）",
                                                    injected_packets
                                                ));
                                            }
                                        }
                                    }
                                } else if input_enabled && msg.msg_type == message_types::MOUSE_BUTTON {
                                    if let Ok(event) = MouseButtonEvent::from_message(&msg) {
                                        let button_code = event.button as u8;
                                        if let Err(e) = inject_mouse_button(button_code, event.pressed) {
                                            warn!("Inject mouse button failed: {}", e);
                                            self.emit_runtime_log(format!("鼠标按键注入失败: {}", e));
                                        } else {
                                            injected_packets += 1;
                                            if injected_packets <= 5 || injected_packets % 50 == 0 {
                                                self.emit_runtime_log(format!(
                                                    "已注入鼠标按键（累计 {}）",
                                                    injected_packets
                                                ));
                                            }
                                        }
                                    }
                                } else {
                                    info!("Received message type: {}", msg.msg_type);
                                }
                            }
                            Err(e) => {
                                warn!("Receive error, disconnecting: {}", e);
                                self.emit_runtime_log(format!("接收消息失败，准备断开并重连: {}", e));
                                self.disconnect().await;
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Connection error: {}", e);
                    self.emit_runtime_log(format!("连接流程异常: {}", e));
                    
                    if !self.config.auto_reconnect {
                        return Err(Box::new(e));
                    }
                    
                    warn!(
                        "Reconnecting in {} seconds...",
                        self.config.reconnect_interval
                    );
                    self.emit_runtime_log(format!(
                        "{} 秒后重连",
                        self.config.reconnect_interval
                    ));
                    tokio::time::sleep(tokio::time::Duration::from_secs(
                        self.config.reconnect_interval,
                    ))
                    .await;
                }
            }
        }
    }

    fn emit_runtime_log<S: Into<String>>(&self, message: S) {
        if let Some(tx) = &self.event_tx {
            let _ = tx.send(ClientRuntimeEvent::Log(message.into()));
        }
    }
}

fn inject_mouse_move(dx: i16, dy: i16) -> Result<(), String> {
    let platform = get_platform().map_err(|e| format!("平台初始化失败: {}", e))?;
    platform
        .inject_mouse_move(dx, dy)
        .map_err(|e| format!("鼠标移动注入失败: {}", e))
}

fn inject_key(key_code: u16, pressed: bool) -> Result<(), String> {
    let platform = get_platform().map_err(|e| format!("平台初始化失败: {}", e))?;
    platform
        .inject_keyboard(key_code, pressed)
        .map_err(|e| format!("键盘注入失败: {}", e))
}

fn inject_mouse_button(button: u8, pressed: bool) -> Result<(), String> {
    let platform = get_platform().map_err(|e| format!("平台初始化失败: {}", e))?;
    platform
        .inject_mouse_button(button, pressed)
        .map_err(|e| format!("鼠标按键注入失败: {}", e))
}

// Re-export ProtocolError for convenience
use crate::protocol::ProtocolError;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_config_default() {
        let config = ClientConfig::default();
        assert_eq!(config.server_addr, "localhost:24800");
        assert_eq!(config.screen_name, "client");
        assert!(config.auto_reconnect);
        assert_eq!(config.reconnect_interval, 5);
    }

    #[tokio::test]
    async fn test_client_creation() {
        let config = ClientConfig::default();
        let client = BarrierClient::new(config);
        
        assert_eq!(client.state(), ClientState::Disconnected);
        assert!(!client.is_ready());
    }

    #[tokio::test]
    async fn test_client_state_transitions() {
        let mut client = BarrierClient::new(ClientConfig::default());
        
        assert_eq!(client.state(), ClientState::Disconnected);
        
        // Note: We can't actually test connect() without a real server
        // This test just verifies state field access works
        client.state = ClientState::Connecting;
        assert_eq!(client.state(), ClientState::Connecting);
    }
}
