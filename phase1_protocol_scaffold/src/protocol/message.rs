//! Barrier Protocol Message Types
//! 
//! This module defines all message types used in the Barrier protocol.
//! Messages are 4-character codes transmitted as big-endian u32 values.

use byteorder::{BigEndian, ByteOrder, ReadBytesExt, WriteBytesExt};
use serde::{Deserialize, Serialize};
use std::io::{self, Cursor, Read, Write};
use thiserror::Error;

/// Protocol message type identifier (4 characters)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MessageType(pub [u8; 4]);

impl MessageType {
    /// Create a new message type from a string slice
    pub fn new(s: &str) -> Result<Self, ProtocolError> {
        if s.len() != 4 {
            return Err(ProtocolError::InvalidMessageType(s.to_string()));
        }
        let mut bytes = [0u8; 4];
        bytes.copy_from_slice(s.as_bytes());
        Ok(Self(bytes))
    }

    /// Create from raw bytes
    pub fn from_bytes(bytes: [u8; 4]) -> Self {
        Self(bytes)
    }

    /// Convert to u32 (big-endian)
    pub fn to_u32(&self) -> u32 {
        BigEndian::read_u32(&self.0)
    }

    /// Convert from u32 (big-endian)
    pub fn from_u32(value: u32) -> Self {
        let mut bytes = [0u8; 4];
        BigEndian::write_u32(&mut bytes, value);
        Self(bytes)
    }

    /// Convert to string
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0).unwrap_or("????")
    }
}

impl std::fmt::Display for MessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Common Barrier protocol message types
pub mod message_types {
    use super::MessageType;

    // Handshake messages
    pub const HELLO_SERVER: MessageType = MessageType(*b"CBYQ"); // Client hello to server
    pub const HELLO_CLIENT: MessageType = MessageType(*b"QBYC"); // Server hello to client
    
    // Clipboard messages
    pub const CLIPBOARD: MessageType = MessageType(*b"CLAP");
    pub const CLIPBOARD_GRAB: MessageType = MessageType(*b"CGRP");
    pub const CLIPBOARD_CLEAR: MessageType = MessageType(*b"CCLR");
    
    // Screen information
    pub const SCREEN_SHAPE: MessageType = MessageType(*b"CROP");
    pub const SCREEN_INFO: MessageType = MessageType(*b"CINF");
    
    // Connection management
    pub const CLIENT_ENTER: MessageType = MessageType(*b"CINN");
    pub const CLIENT_LEAVE: MessageType = MessageType(*b"COUT");
    pub const CLIENT_DISCONNECT: MessageType = MessageType(*b"CDIS");
    
    // Input events
    pub const MOUSE_MOVE: MessageType = MessageType(*b"CMOV");
    pub const MOUSE_BUTTON: MessageType = MessageType(*b"CBUT");
    pub const MOUSE_WHEEL: MessageType = MessageType(*b"CWHM");
    pub const KEY_DOWN: MessageType = MessageType(*b"CKDn");
    pub const KEY_UP: MessageType = MessageType(*b"CKUp");
    pub const KEY_REPEAT: MessageType = MessageType(*b"CKRP");
    
    // Options and configuration
    pub const SCREEN_OPTIONS: MessageType = MessageType(*b"CSOP");
    pub const HEARTBEAT: MessageType = MessageType(*b"CHRT");
    pub const NOOP: MessageType = MessageType(*b"CNOP");
    
    // Error and status
    pub const ERROR: MessageType = MessageType(*b"CERR");
    pub const ACK: MessageType = MessageType(*b"CACK");
}

/// Protocol error types
#[derive(Error, Debug)]
pub enum ProtocolError {
    #[error("Invalid message type: {0}")]
    InvalidMessageType(String),
    
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    
    #[error("Invalid message size: expected {expected}, got {actual}")]
    InvalidMessageSize { expected: usize, actual: usize },
    
    #[error("Unknown message type: {0}")]
    UnknownMessageType(u32),
    
    #[error("Protocol version mismatch")]
    VersionMismatch,
    
    #[error("Invalid UTF-8 in message data")]
    InvalidUtf8(#[from] std::string::FromUtf8Error),
    
    #[error("Checksum mismatch")]
    ChecksumMismatch,
}

/// Result type for protocol operations
pub type ProtocolResult<T> = Result<T, ProtocolError>;

/// Barrier protocol message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Message type identifier
    pub msg_type: MessageType,
    
    /// Message payload data
    pub data: Vec<u8>,
}

impl Message {
    /// Create a new message with the given type and data
    pub fn new(msg_type: MessageType, data: Vec<u8>) -> Self {
        Self { msg_type, data }
    }

    /// Create a message with no data
    pub fn empty(msg_type: MessageType) -> Self {
        Self {
            msg_type,
            data: Vec::new(),
        }
    }

    /// Create a message from a string payload
    pub fn from_string(msg_type: MessageType, text: &str) -> Self {
        Self {
            msg_type,
            data: text.as_bytes().to_vec(),
        }
    }

    /// Get the message data as a string
    pub fn as_string(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(&self.data)
    }

    /// Serialize the message to binary format
    /// Format: [size: u32][type: u32][data...][checksum: u32]
    pub fn serialize(&self) -> ProtocolResult<Vec<u8>> {
        let mut buffer = Vec::new();
        
        // Calculate total size (type + data + checksum)
        let total_size = 4 + self.data.len() + 4;
        
        // Write size
        buffer.write_u32::<BigEndian>(total_size as u32)?;
        
        // Write message type
        buffer.write_u32::<BigEndian>(self.msg_type.to_u32())?;
        
        // Write data
        buffer.write_all(&self.data)?;
        
        // Calculate and write checksum (simple XOR of all data bytes)
        let checksum = self.data.iter().fold(0u32, |acc, &b| acc ^ (b as u32));
        buffer.write_u32::<BigEndian>(checksum)?;
        
        Ok(buffer)
    }

    /// Deserialize a message from binary format
    pub fn deserialize(mut reader: impl Read) -> ProtocolResult<Self> {
        // Read size
        let size = reader.read_u32::<BigEndian>()? as usize;
        
        // Read message type
        let type_value = reader.read_u32::<BigEndian>()?;
        let msg_type = MessageType::from_u32(type_value);
        
        // Calculate data length (size - type - checksum)
        let data_len = size.saturating_sub(8);
        
        // Read data
        let mut data = vec![0u8; data_len];
        reader.read_exact(&mut data)?;
        
        // Read and verify checksum
        let stored_checksum = reader.read_u32::<BigEndian>()?;
        let calculated_checksum = data.iter().fold(0u32, |acc, &b| acc ^ (b as u32));
        
        if stored_checksum != calculated_checksum {
            return Err(ProtocolError::ChecksumMismatch);
        }
        
        Ok(Self { msg_type, data })
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> ProtocolResult<Self> {
        Self::deserialize(Cursor::new(bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use message_types::*;

    #[test]
    fn test_message_type_creation() {
        let mt = MessageType::new("CBYQ").unwrap();
        assert_eq!(mt.as_str(), "CBYQ");
        assert_eq!(mt.to_u32(), 0x43425951);
    }

    #[test]
    fn test_message_serialization() {
        let msg = Message::from_string(HELLO_SERVER, "Barrier 2.0");
        let bytes = msg.serialize().unwrap();
        
        assert!(bytes.len() > 8); // At least header + checksum
        
        let deserialized = Message::from_bytes(&bytes).unwrap();
        assert_eq!(deserialized.msg_type, msg.msg_type);
        assert_eq!(deserialized.as_string().unwrap(), "Barrier 2.0");
    }

    #[test]
    fn test_empty_message() {
        let msg = Message::empty(NOOP);
        let bytes = msg.serialize().unwrap();
        
        let deserialized = Message::from_bytes(&bytes).unwrap();
        assert_eq!(deserialized.msg_type, NOOP);
        assert!(deserialized.data.is_empty());
    }
}
