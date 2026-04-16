//! Barrier Protocol Handshake Implementation
//! 
//! This module handles the initial handshake between client and server.

use super::message::{Message, MessageType, ProtocolResult};
use crate::protocol::message::message_types::*;

/// Barrier protocol version
pub const BARRIER_VERSION: &str = "Barrier 2.0";

/// Handshake state machine
#[derive(Debug, Clone, PartialEq)]
pub enum HandshakeState {
    /// Initial state, waiting to start handshake
    Init,
    /// Client has sent hello to server
    HelloSent,
    /// Server has responded with hello
    HelloReceived,
    /// Handshake completed successfully
    Completed,
    /// Handshake failed
    Failed(String),
}

/// Handshake result containing client/server info
#[derive(Debug, Clone)]
pub struct HandshakeResult {
    /// Remote application name (e.g., "Barrier")
    pub app_name: String,
    /// Protocol version string
    pub version: String,
    /// Optional screen name
    pub screen_name: Option<String>,
}

/// Perform client-side handshake
pub async fn client_handshake<W, R>(
    writer: &mut W,
    reader: &mut R,
    screen_name: &str,
) -> ProtocolResult<HandshakeResult>
where
    W: tokio::io::AsyncWrite + Unpin,
    R: tokio::io::AsyncRead + Unpin,
{
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    
    let mut reader = BufReader::new(reader);
    
    // Send client hello: CBYQ + version + screen_name
    let hello_data = format!("{}\n{}\n", BARRIER_VERSION, screen_name);
    let hello_msg = Message::from_string(HELLO_SERVER, &hello_data);
    let hello_bytes = hello_msg.serialize()?;
    tokio::io::AsyncWriteExt::write_all(writer, &hello_bytes).await?;
    tokio::io::AsyncWriteExt::flush(writer).await?;
    
    log::info!("Sent client hello: {} {}", BARRIER_VERSION, screen_name);
    
    // Read server response
    let response_msg = read_message(&mut reader).await?;
    
    if response_msg.msg_type != HELLO_CLIENT {
        return Err(super::message::ProtocolError::UnknownMessageType(
            response_msg.msg_type.to_u32(),
        ));
    }
    
    // Parse server hello response
    let response_text = response_msg.as_string()?;
    let parts: Vec<&str> = response_text.trim().split('\n').collect();
    
    if parts.len() < 2 {
        return Err(super::message::ProtocolError::InvalidMessageSize {
            expected: 2,
            actual: parts.len(),
        });
    }
    
    // parts[0] = Barrier version, parts[1] = server screen_name
    let result = HandshakeResult {
        app_name: parts[0].to_string(),
        version: parts[0].to_string(),
        screen_name: parts.get(1).map(|s| s.to_string()),
    };
    
    log::info!(
        "Handshake completed: {} {:?}",
        result.app_name,
        result.screen_name
    );
    
    Ok(result)
}

/// Perform server-side handshake
pub async fn server_handshake<W, R>(
    writer: &mut W,
    reader: &mut R,
) -> ProtocolResult<HandshakeResult>
where
    W: tokio::io::AsyncWrite + Unpin,
    R: tokio::io::AsyncRead + Unpin,
{
    use tokio::io::{AsyncBufReadExt, BufReader};
    
    let mut reader = BufReader::new(reader);
    
    // Wait for client hello
    let hello_msg = read_message(&mut reader).await?;
    
    if hello_msg.msg_type != HELLO_SERVER {
        return Err(super::message::ProtocolError::UnknownMessageType(
            hello_msg.msg_type.to_u32(),
        ));
    }
    
    // Parse client hello
    let hello_text = hello_msg.as_string()?;
    let parts: Vec<&str> = hello_text.trim().split('\n').collect();
    
    if parts.len() < 2 {
        return Err(super::message::ProtocolError::InvalidMessageSize {
            expected: 2,
            actual: parts.len(),
        });
    }
    
    // parts[0] = Barrier version (e.g. "Barrier 1.0")
    // parts[1] = screen_name
    let client_result = HandshakeResult {
        app_name: parts[0].to_string(),
        version: parts[0].to_string(),
        screen_name: parts.get(1).map(|s| s.to_string()),
    };
    
    log::info!(
        "Received client hello: {} {} {:?}",
        client_result.app_name,
        client_result.version,
        client_result.screen_name
    );
    
    // Send server hello response: QBYC + version + server_screen_name
    let response_data = format!("{}\n{}\n", BARRIER_VERSION, "server");
    let response_msg = Message::from_string(HELLO_CLIENT, &response_data);
    let response_bytes = response_msg.serialize()?;
    tokio::io::AsyncWriteExt::write_all(writer, &response_bytes).await?;
    tokio::io::AsyncWriteExt::flush(writer).await?;
    
    log::info!("Sent server hello: {}", BARRIER_VERSION);
    
    Ok(client_result)
}

/// Read a complete message from the stream
async fn read_message<R>(
    reader: &mut R,
) -> ProtocolResult<Message>
where
    R: tokio::io::AsyncBufRead + Unpin,
{
    use tokio::io::AsyncReadExt;
    
    // Read size (4 bytes)
    let mut size_buf = [0u8; 4];
    reader.read_exact(&mut size_buf).await?;
    let size = u32::from_be_bytes(size_buf) as usize;
    
    // Read type (4 bytes)
    let mut type_buf = [0u8; 4];
    reader.read_exact(&mut type_buf).await?;
    
    // Calculate data length
    let data_len = size.saturating_sub(8);
    
    // Read data
    let mut data = vec![0u8; data_len];
    if data_len > 0 {
        reader.read_exact(&mut data).await?;
    }
    
    // Read checksum (4 bytes)
    let mut checksum_buf = [0u8; 4];
    reader.read_exact(&mut checksum_buf).await?;
    let stored_checksum = u32::from_be_bytes(checksum_buf);
    
    // Verify checksum
    let calculated_checksum = data.iter().fold(0u32, |acc, &b| acc ^ (b as u32));
    if stored_checksum != calculated_checksum {
        return Err(super::message::ProtocolError::ChecksumMismatch);
    }
    
    let msg_type = MessageType::from_bytes(type_buf);
    
    Ok(Message { msg_type, data })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncRead, AsyncWrite};
    
    #[tokio::test]
    async fn test_handshake_constants() {
        assert_eq!(BARRIER_VERSION, "Barrier 2.0");
    }
    
    #[tokio::test]
    async fn test_handshake_state_transitions() {
        let mut state = HandshakeState::Init;
        assert_eq!(state, HandshakeState::Init);
        
        state = HandshakeState::HelloSent;
        assert_eq!(state, HandshakeState::HelloSent);
        
        state = HandshakeState::HelloReceived;
        assert_eq!(state, HandshakeState::HelloReceived);
        
        state = HandshakeState::Completed;
        assert_eq!(state, HandshakeState::Completed);
    }
}
