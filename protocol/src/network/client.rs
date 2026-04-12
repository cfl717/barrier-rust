//! Barrier Client Implementation
//! 
//! This module implements the Barrier client that connects to a server.

use crate::protocol::{client_handshake, HandshakeResult, Message, ProtocolResult};
use crate::network::Connection;
use log::{error, info, warn};
use tokio::net::TcpStream;

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

/// Barrier client
pub struct BarrierClient {
    config: ClientConfig,
    state: ClientState,
    connection: Option<Connection>,
    handshake_result: Option<HandshakeResult>,
}

impl BarrierClient {
    /// Create a new client instance
    pub fn new(config: ClientConfig) -> Self {
        Self {
            config,
            state: ClientState::Disconnected,
            connection: None,
            handshake_result: None,
        }
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
        
        // Connect to server
        let stream = match TcpStream::connect(&self.config.server_addr).await {
            Ok(stream) => stream,
            Err(e) => {
                self.state = ClientState::Error(format!("Connection failed: {}", e));
                return Err(ProtocolError::Io(e));
            }
        };
        
        info!("TCP connection established");
        
        // Create connection wrapper
        let mut connection = Connection::new(stream, 0);
        
        // Perform handshake
        self.state = ClientState::Authenticating;
        
        let (mut read_half, mut write_half) = connection.split();
        
        let handshake_result = match client_handshake(
            &mut write_half,
            &mut read_half,
            &self.config.screen_name,
        ).await {
            Ok(result) => result,
            Err(e) => {
                self.state = ClientState::Error(format!("Handshake failed: {}", e));
                return Err(e);
            }
        };
        
        info!("Handshake completed: {:?}", handshake_result);
        
        // Recreate connection from stream (need to handle this better)
        // For now, we'll just store the handshake result
        self.handshake_result = Some(handshake_result);
        self.state = ClientState::Ready;
        
        Ok(())
    }

    /// Disconnect from server
    pub async fn disconnect(&mut self) {
        self.connection = None;
        self.handshake_result = None;
        self.state = ClientState::Disconnected;
        info!("Disconnected from server");
    }

    /// Send a message to the server
    pub async fn send(&mut self, message: &Message) -> ProtocolResult<()> {
        if let Some(conn) = &mut self.connection {
            conn.send(message).await
        } else {
            Err(ProtocolError::Io(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "Not connected to server",
            )))
        }
    }

    /// Receive a message from the server
    pub async fn receive(&mut self) -> ProtocolResult<Message> {
        if let Some(conn) = &mut self.connection {
            conn.receive().await
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
                    
                    // TODO: Main message processing loop would go here
                    // For now, we just stay connected
                    
                    // Keep connection alive
                    tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
                }
                Err(e) => {
                    error!("Connection error: {}", e);
                    
                    if !self.config.auto_reconnect {
                        return Err(Box::new(e));
                    }
                    
                    warn!(
                        "Reconnecting in {} seconds...",
                        self.config.reconnect_interval
                    );
                    tokio::time::sleep(tokio::time::Duration::from_secs(
                        self.config.reconnect_interval,
                    ))
                    .await;
                }
            }
        }
    }
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
