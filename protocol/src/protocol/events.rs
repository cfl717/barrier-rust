//! Barrier Input Events Protocol Support
//! 
//! This module handles mouse and keyboard event serialization.

use super::message::{Message, MessageType, ProtocolResult};
use crate::protocol::message::message_types::*;
use byteorder::{BigEndian, WriteBytesExt};

/// Mouse button identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    None = 0,
    Left = 1,
    Right = 2,
    Middle = 3,
}

impl MouseButton {
    pub fn from_code(code: u8) -> Self {
        match code {
            1 => Self::Left,
            2 => Self::Right,
            3 => Self::Middle,
            _ => Self::None,
        }
    }
}

/// Keyboard modifier keys
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModifierKeys(pub u16);

impl ModifierKeys {
    pub const NONE: Self = Self(0);
    pub const SHIFT: Self = Self(0x0001);
    pub const CONTROL: Self = Self(0x0002);
    pub const ALT: Self = Self(0x0004);
    pub const META: Self = Self(0x0008);
    pub const CAPS_LOCK: Self = Self(0x0010);
    pub const NUM_LOCK: Self = Self(0x0020);
    pub const SCROLL_LOCK: Self = Self(0x0040);
    pub const RIGHT_ALT: Self = Self(0x0080);
    pub const RIGHT_CONTROL: Self = Self(0x0100);
    pub const RIGHT_META: Self = Self(0x0200);

    pub fn is_shift(&self) -> bool {
        self.0 & Self::SHIFT.0 != 0
    }

    pub fn is_control(&self) -> bool {
        self.0 & Self::CONTROL.0 != 0
    }

    pub fn is_alt(&self) -> bool {
        self.0 & (Self::ALT.0 | Self::RIGHT_ALT.0) != 0
    }

    pub fn is_meta(&self) -> bool {
        self.0 & (Self::META.0 | Self::RIGHT_META.0) != 0
    }
}

/// Mouse move event
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MouseMoveEvent {
    /// Relative X movement (signed 16-bit)
    pub dx: i16,
    /// Relative Y movement (signed 16-bit)
    pub dy: i16,
    /// Current modifier keys state
    pub modifiers: ModifierKeys,
}

impl MouseMoveEvent {
    pub fn new(dx: i16, dy: i16, modifiers: ModifierKeys) -> Self {
        Self {
            dx,
            dy,
            modifiers,
        }
    }

    /// Serialize to message
    pub fn to_message(&self) -> Message {
        let mut data = Vec::with_capacity(6);
        let _ = data.write_i16::<BigEndian>(self.dx);
        let _ = data.write_i16::<BigEndian>(self.dy);
        let _ = data.write_u16::<BigEndian>(self.modifiers.0);
        Message::new(MOUSE_MOVE, data)
    }

    /// Parse from message
    pub fn from_message(msg: &Message) -> ProtocolResult<Self> {
        if msg.data.len() < 6 {
            return Err(super::message::ProtocolError::InvalidMessageSize {
                expected: 6,
                actual: msg.data.len(),
            });
        }

        use byteorder::ReadBytesExt;
        use std::io::Cursor;

        let mut cursor = Cursor::new(&msg.data);
        let dx = cursor.read_i16::<BigEndian>()?;
        let dy = cursor.read_i16::<BigEndian>()?;
        let modifiers = ModifierKeys(cursor.read_u16::<BigEndian>()?);

        Ok(Self {
            dx,
            dy,
            modifiers,
        })
    }
}

/// Mouse button event
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MouseButtonEvent {
    /// Button identifier
    pub button: MouseButton,
    /// True if pressed, false if released
    pub pressed: bool,
    /// Current modifier keys state
    pub modifiers: ModifierKeys,
}

impl MouseButtonEvent {
    pub fn new(button: MouseButton, pressed: bool, modifiers: ModifierKeys) -> Self {
        Self {
            button,
            pressed,
            modifiers,
        }
    }

    /// Serialize to message
    pub fn to_message(&self) -> Message {
        let mut data = Vec::with_capacity(4);
        let _ = data.write_u8(self.button as u8);
        let _ = data.write_u8(if self.pressed { 1 } else { 0 });
        let _ = data.write_u16::<BigEndian>(self.modifiers.0);
        Message::new(MOUSE_BUTTON, data)
    }

    /// Parse from message
    pub fn from_message(msg: &Message) -> ProtocolResult<Self> {
        if msg.data.len() < 4 {
            return Err(super::message::ProtocolError::InvalidMessageSize {
                expected: 4,
                actual: msg.data.len(),
            });
        }

        let button = MouseButton::from_code(msg.data[0]);
        let pressed = msg.data[1] != 0;
        let modifiers = ModifierKeys(u16::from_be_bytes([msg.data[2], msg.data[3]]));

        Ok(Self {
            button,
            pressed,
            modifiers,
        })
    }
}

/// Keyboard key event
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeyEvent {
    /// Key code (platform-specific scan code or virtual key code)
    pub key_code: u16,
    /// Event type: true for key down, false for key up
    pub pressed: bool,
    /// Current modifier keys state
    pub modifiers: ModifierKeys,
    /// Optional repeat count for key repeat events
    pub repeat_count: u16,
}

impl KeyEvent {
    pub fn new(key_code: u16, pressed: bool, modifiers: ModifierKeys) -> Self {
        Self {
            key_code,
            pressed,
            modifiers,
            repeat_count: 0,
        }
    }

    pub fn with_repeat(mut self, count: u16) -> Self {
        self.repeat_count = count;
        self
    }

    /// Serialize to key down message
    pub fn to_key_down_message(&self) -> Message {
        let mut data = Vec::with_capacity(6);
        let _ = data.write_u16::<BigEndian>(self.key_code);
        let _ = data.write_u16::<BigEndian>(self.modifiers.0);
        Message::new(KEY_DOWN, data)
    }

    /// Serialize to key up message
    pub fn to_key_up_message(&self) -> Message {
        let mut data = Vec::with_capacity(6);
        let _ = data.write_u16::<BigEndian>(self.key_code);
        let _ = data.write_u16::<BigEndian>(self.modifiers.0);
        Message::new(KEY_UP, data)
    }

    /// Parse key event from message
    pub fn from_message(msg: &Message) -> ProtocolResult<Self> {
        if msg.data.len() < 4 {
            return Err(super::message::ProtocolError::InvalidMessageSize {
                expected: 4,
                actual: msg.data.len(),
            });
        }

        let key_code = u16::from_be_bytes([msg.data[0], msg.data[1]]);
        let modifiers = ModifierKeys(u16::from_be_bytes([msg.data[2], msg.data[3]]));
        let pressed = msg.msg_type == KEY_DOWN || msg.msg_type == KEY_REPEAT;

        Ok(Self {
            key_code,
            pressed,
            modifiers,
            repeat_count: 0,
        })
    }
}

/// Screen position for enter/leave events
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenPosition {
    pub x: i16,
    pub y: i16,
}

impl ScreenPosition {
    pub fn new(x: i16, y: i16) -> Self {
        Self { x, y }
    }
}

/// Client enter screen event
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClientEnterEvent {
    /// Screen ID being entered
    pub screen_id: u8,
    /// Entry position
    pub position: ScreenPosition,
    /// Sequence number
    pub sequence: u16,
}

impl ClientEnterEvent {
    pub fn new(screen_id: u8, x: i16, y: i16, sequence: u16) -> Self {
        Self {
            screen_id,
            position: ScreenPosition::new(x, y),
            sequence,
        }
    }

    /// Serialize to message
    pub fn to_message(&self) -> Message {
        let mut data = Vec::with_capacity(6);
        let _ = data.write_u8(self.screen_id);
        let _ = data.write_i16::<BigEndian>(self.position.x);
        let _ = data.write_i16::<BigEndian>(self.position.y);
        // Sequence is optional, not included in basic format
        Message::new(CLIENT_ENTER, data)
    }

    /// Parse from message
    pub fn from_message(msg: &Message) -> ProtocolResult<Self> {
        if msg.data.len() < 5 {
            return Err(super::message::ProtocolError::InvalidMessageSize {
                expected: 5,
                actual: msg.data.len(),
            });
        }

        use byteorder::ReadBytesExt;
        use std::io::Cursor;

        let mut cursor = Cursor::new(&msg.data);
        let screen_id = cursor.read_u8()?;
        let x = cursor.read_i16::<BigEndian>()?;
        let y = cursor.read_i16::<BigEndian>()?;

        Ok(Self {
            screen_id,
            position: ScreenPosition::new(x, y),
            sequence: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mouse_move_serialization() {
        let event = MouseMoveEvent::new(10, -5, ModifierKeys::CONTROL);
        let msg = event.to_message();
        
        assert_eq!(msg.msg_type, MOUSE_MOVE);
        
        let parsed = MouseMoveEvent::from_message(&msg).unwrap();
        assert_eq!(parsed.dx, 10);
        assert_eq!(parsed.dy, -5);
        assert!(parsed.modifiers.is_control());
    }

    #[test]
    fn test_mouse_button_serialization() {
        let event = MouseButtonEvent::new(MouseButton::Left, true, ModifierKeys::NONE);
        let msg = event.to_message();
        
        assert_eq!(msg.msg_type, MOUSE_BUTTON);
        
        let parsed = MouseButtonEvent::from_message(&msg).unwrap();
        assert_eq!(parsed.button, MouseButton::Left);
        assert!(parsed.pressed);
    }

    #[test]
    fn test_key_event_serialization() {
        let event = KeyEvent::new(65, true, ModifierKeys::SHIFT);
        let msg = event.to_key_down_message();
        
        assert_eq!(msg.msg_type, KEY_DOWN);
        
        let parsed = KeyEvent::from_message(&msg).unwrap();
        assert_eq!(parsed.key_code, 65);
        assert!(parsed.pressed);
        assert!(parsed.modifiers.is_shift());
    }

    #[test]
    fn test_client_enter_serialization() {
        let event = ClientEnterEvent::new(1, 100, 200, 0);
        let msg = event.to_message();
        
        assert_eq!(msg.msg_type, CLIENT_ENTER);
        
        let parsed = ClientEnterEvent::from_message(&msg).unwrap();
        assert_eq!(parsed.screen_id, 1);
        assert_eq!(parsed.position.x, 100);
        assert_eq!(parsed.position.y, 200);
    }

    #[test]
    fn test_modifier_keys() {
        let mods = ModifierKeys(ModifierKeys::SHIFT.0 | ModifierKeys::CONTROL.0);
        assert!(mods.is_shift());
        assert!(mods.is_control());
        assert!(!mods.is_alt());
    }
}
