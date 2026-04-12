//! Barrier Input Capture Library
//!
//! This library provides cross-platform input capture and injection capabilities
//! for the Barrier remote desktop application.
//!
//! # Features
//!
//! - Keyboard event capture and injection
//! - Mouse event capture and injection  
//! - Clipboard synchronization
//! - Cross-platform support (Linux, Windows, macOS)
//!
//! # Example
//!
//! ```rust,no_run
//! use barrier_input_capture::{InputCapture, PlatformError};
//!
//! fn main() -> Result<(), PlatformError> {
//!     let capture = InputCapture::new()?;
//!     capture.initialize()?;
//!     
//!     // Start capturing keyboard events
//!     capture.capture_keyboard()?;
//!     
//!     Ok(())
//! }
//! ```

mod platform;

pub use platform::{get_platform, PlatformError, PlatformInput};

/// Main input capture struct that provides a unified API
pub struct InputCapture {
    platform: Box<dyn PlatformInput>,
}

impl InputCapture {
    /// Create a new InputCapture instance for the current platform
    pub fn new() -> Result<Self, PlatformError> {
        let platform = get_platform()?;
        Ok(Self { platform })
    }

    /// Initialize the input capture system
    pub fn initialize(&self) -> Result<(), PlatformError> {
        self.platform.initialize()
    }

    /// Start capturing keyboard events
    pub fn capture_keyboard(&self) -> Result<(), PlatformError> {
        self.platform.capture_keyboard()
    }

    /// Inject a keyboard event
    pub fn inject_keyboard(&self, key_code: u16, pressed: bool) -> Result<(), PlatformError> {
        self.platform.inject_keyboard(key_code, pressed)
    }

    /// Start capturing mouse events
    pub fn capture_mouse(&self) -> Result<(), PlatformError> {
        self.platform.capture_mouse()
    }

    /// Inject mouse movement
    pub fn inject_mouse_move(&self, dx: i16, dy: i16) -> Result<(), PlatformError> {
        self.platform.inject_mouse_move(dx, dy)
    }

    /// Inject mouse button event
    pub fn inject_mouse_button(&self, button: u8, pressed: bool) -> Result<(), PlatformError> {
        self.platform.inject_mouse_button(button, pressed)
    }

    /// Get clipboard content
    pub fn get_clipboard(&self) -> Result<String, PlatformError> {
        self.platform.get_clipboard()
    }

    /// Set clipboard content
    pub fn set_clipboard(&self, content: &str) -> Result<(), PlatformError> {
        self.platform.set_clipboard(content)
    }

    /// Shutdown the input capture system
    pub fn shutdown(&self) -> Result<(), PlatformError> {
        self.platform.shutdown()
    }
}

/// Keyboard event types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyEvent {
    KeyPress(u16),
    KeyRelease(u16),
}

/// Mouse event types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseEvent {
    Move(i16, i16),
    ButtonPress(u8),
    ButtonRelease(u8),
    Scroll(i16, i16),
}

/// Clipboard event
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipboardEvent {
    ContentChanged(String),
    Requested,
}

/// Combined input event type
#[derive(Debug, Clone)]
pub enum InputEvent {
    Keyboard(KeyEvent),
    Mouse(MouseEvent),
    Clipboard(ClipboardEvent),
}
