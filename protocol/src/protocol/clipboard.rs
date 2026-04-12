//! Barrier Clipboard Protocol Support
//!
//! This module handles clipboard data exchange between client and server.

use super::message::{Message, MessageType, ProtocolResult};
use crate::protocol::message::message_types::*;
use serde::{Deserialize, Serialize};

/// Clipboard data formats supported by Barrier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClipboardFormat {
    /// Plain text
    Text,
    /// HTML content
    Html,
    /// Rich Text Format
    Rtf,
    /// Bitmap image
    Bitmap,
    /// PNG image
    Png,
    /// Unknown or custom format
    Unknown,
}

impl ClipboardFormat {
    /// Parse format from string identifier
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "text" | "utf8_text" | "text/plain" => Self::Text,
            "html" | "text/html" => Self::Html,
            "rtf" | "application/rtf" => Self::Rtf,
            "bmp" | "image/bmp" | "bitmap" => Self::Bitmap,
            "png" | "image/png" => Self::Png,
            _ => Self::Unknown,
        }
    }

    /// Get string identifier for the format
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Text => "UTF8_TEXT",
            Self::Html => "HTML",
            Self::Rtf => "RTF",
            Self::Bitmap => "BMP",
            Self::Png => "PNG",
            Self::Unknown => "UNKNOWN",
        }
    }
}

/// Clipboard data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardData {
    /// Data format
    pub format: ClipboardFormat,
    /// Sequence number for synchronization
    pub sequence_id: u32,
    /// Actual clipboard content
    pub content: Vec<u8>,
}

impl ClipboardData {
    /// Create new clipboard data
    pub fn new(format: ClipboardFormat, content: Vec<u8>, sequence_id: u32) -> Self {
        Self {
            format,
            content,
            sequence_id,
        }
    }

    /// Create text clipboard data
    pub fn text(text: &str, sequence_id: u32) -> Self {
        Self {
            format: ClipboardFormat::Text,
            content: text.as_bytes().to_vec(),
            sequence_id,
        }
    }

    /// Get content as string (for text formats)
    pub fn as_text(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(&self.content)
    }
}

/// Clipboard event types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardEvent {
    /// Clipboard ownership grabbed
    Grab,
    /// Clipboard data received
    Data,
    /// Clipboard cleared
    Clear,
}

/// Create a clipboard grab message
pub fn create_grab_message(format: ClipboardFormat, sequence_id: u32) -> Message {
    let data = format!("{}\n{}", format.as_str(), sequence_id);
    Message::from_string(CLIPBOARD_GRAB, &data)
}

/// Create a clipboard data message
pub fn create_data_message(data: &ClipboardData) -> Message {
    let mut content = Vec::new();

    // Format: format\nsequence_id\ncontent
    content.extend_from_slice(data.format.as_str().as_bytes());
    content.push(b'\n');
    content.extend_from_slice(data.sequence_id.to_string().as_bytes());
    content.push(b'\n');
    content.extend_from_slice(&data.content);

    Message::new(CLIPBOARD, content)
}

/// Create a clipboard clear message
pub fn create_clear_message() -> Message {
    Message::empty(CLIPBOARD_CLEAR)
}

/// Parse clipboard grab message
pub fn parse_grab_message(msg: &Message) -> ProtocolResult<(ClipboardFormat, u32)> {
    let text = msg.as_string()?;
    let parts: Vec<&str> = text.split('\n').collect();

    if parts.len() < 2 {
        return Err(super::message::ProtocolError::InvalidMessageSize {
            expected: 2,
            actual: parts.len(),
        });
    }

    let format = ClipboardFormat::from_str(parts[0]);
    let sequence_id: u32 = parts[1].parse().unwrap_or(0);

    Ok((format, sequence_id))
}

/// Parse clipboard data message
pub fn parse_data_message(msg: &Message) -> ProtocolResult<ClipboardData> {
    let text = std::str::from_utf8(&msg.data)
        .map_err(|e| super::message::ProtocolError::InvalidUtf8Str(e))?;

    // Find the first two newlines to separate header from content
    let mut parts = text.splitn(3, '\n');

    let format_str =
        parts
            .next()
            .ok_or_else(|| super::message::ProtocolError::InvalidMessageSize {
                expected: 3,
                actual: 0,
            })?;

    let sequence_str =
        parts
            .next()
            .ok_or_else(|| super::message::ProtocolError::InvalidMessageSize {
                expected: 3,
                actual: 1,
            })?;

    let content = parts.next().unwrap_or("");

    let format = ClipboardFormat::from_str(format_str);
    let sequence_id: u32 = sequence_str.parse().unwrap_or(0);

    Ok(ClipboardData {
        format,
        sequence_id,
        content: content.as_bytes().to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clipboard_format_parsing() {
        assert_eq!(ClipboardFormat::from_str("text"), ClipboardFormat::Text);
        assert_eq!(
            ClipboardFormat::from_str("UTF8_TEXT"),
            ClipboardFormat::Text
        );
        assert_eq!(
            ClipboardFormat::from_str("text/plain"),
            ClipboardFormat::Text
        );
        assert_eq!(ClipboardFormat::from_str("html"), ClipboardFormat::Html);
        assert_eq!(ClipboardFormat::from_str("RTF"), ClipboardFormat::Rtf);
        assert_eq!(ClipboardFormat::from_str("bmp"), ClipboardFormat::Bitmap);
        assert_eq!(ClipboardFormat::from_str("png"), ClipboardFormat::Png);
        assert_eq!(
            ClipboardFormat::from_str("unknown"),
            ClipboardFormat::Unknown
        );
    }

    #[test]
    fn test_clipboard_data_creation() {
        let data = ClipboardData::text("Hello, World!", 1);
        assert_eq!(data.format, ClipboardFormat::Text);
        assert_eq!(data.sequence_id, 1);
        assert_eq!(data.as_text().unwrap(), "Hello, World!");
    }

    #[test]
    fn test_create_grab_message() {
        let msg = create_grab_message(ClipboardFormat::Text, 42);
        assert_eq!(msg.msg_type, CLIPBOARD_GRAB);

        let (format, seq) = parse_grab_message(&msg).unwrap();
        assert_eq!(format, ClipboardFormat::Text);
        assert_eq!(seq, 42);
    }

    #[test]
    fn test_create_data_message() {
        let data = ClipboardData::text("Test content", 5);
        let msg = create_data_message(&data);
        assert_eq!(msg.msg_type, CLIPBOARD);

        let parsed = parse_data_message(&msg).unwrap();
        assert_eq!(parsed.format, ClipboardFormat::Text);
        assert_eq!(parsed.sequence_id, 5);
        assert_eq!(parsed.as_text().unwrap(), "Test content");
    }
}
