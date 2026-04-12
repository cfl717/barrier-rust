//! Barrier Network Connection
//! 
//! This module handles individual client connections.

use crate::protocol::{HandshakeResult, Message, ProtocolResult};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;

/// Active network connection with a client or server
pub struct Connection {
    /// TCP stream
    stream: TcpStream,
    /// Handshake result (after successful handshake)
    handshake_result: Option<HandshakeResult>,
    /// Connection ID for logging
    id: u64,
}

impl Connection {
    /// Create a new connection from a TCP stream
    pub fn new(stream: TcpStream, id: u64) -> Self {
        Self {
            stream,
            handshake_result: None,
            id,
        }
    }

    /// Get the connection ID
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Check if handshake is complete
    pub fn is_authenticated(&self) -> bool {
        self.handshake_result.is_some()
    }

    /// Get handshake result
    pub fn handshake_result(&self) -> Option<&HandshakeResult> {
        self.handshake_result.as_ref()
    }

    /// Set handshake result after successful handshake
    pub fn set_handshake_result(&mut self, result: HandshakeResult) {
        self.handshake_result = Some(result);
    }

    /// Split the connection into read and write halves
    pub fn split(self) -> (tokio::io::ReadHalf<TcpStream>, tokio::io::WriteHalf<TcpStream>) {
        self.stream.into_split()
    }

    /// Get mutable reference to the stream
    pub fn stream_mut(&mut self) -> &mut TcpStream {
        &mut self.stream
    }

    /// Send a message over the connection
    pub async fn send(&mut self, message: &Message) -> ProtocolResult<()> {
        use tokio::io::AsyncWriteExt;
        
        let bytes = message.serialize()?;
        self.stream.write_all(&bytes).await?;
        self.stream.flush().await?;
        Ok(())
    }

    /// Receive a message from the connection
    pub async fn receive(&mut self) -> ProtocolResult<Message> {
        use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
        
        let mut reader = BufReader::new(&mut self.stream);
        
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
            return Err(crate::protocol::ProtocolError::ChecksumMismatch);
        }
        
        let msg_type = crate::protocol::MessageType::from_bytes(type_buf);
        
        Ok(Message { msg_type, data })
    }

    /// Close the connection
    pub async fn close(self) -> std::io::Result<()> {
        // Connection is dropped, TCP stream will be closed
        Ok(())
    }
}

impl std::fmt::Debug for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Connection")
            .field("id", &self.id)
            .field("authenticated", &self.is_authenticated())
            .finish()
    }
}
