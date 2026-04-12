//! Windows Platform Implementation (stub)
//!
//! This module provides Windows-specific input handling.

use super::{PlatformError, PlatformInput};

/// Windows input handler
pub struct WindowsInput {
    initialized: bool,
}

impl WindowsInput {
    /// Create a new Windows input handler
    pub fn new() -> Result<Self, PlatformError> {
        Ok(Self { initialized: false })
    }
}

impl PlatformInput for WindowsInput {
    fn initialize() -> Result<(), PlatformError> {
        log::info!("Windows input system initialized (stub)");
        Ok(())
    }

    fn capture_keyboard(&self) -> Result<(), PlatformError> {
        Err(PlatformError::NotSupported)
    }

    fn inject_keyboard(&self, _key_code: u16, _pressed: bool) -> Result<(), PlatformError> {
        Err(PlatformError::NotSupported)
    }

    fn capture_mouse(&self) -> Result<(), PlatformError> {
        Err(PlatformError::NotSupported)
    }

    fn inject_mouse_move(&self, _dx: i16, _dy: i16) -> Result<(), PlatformError> {
        Err(PlatformError::NotSupported)
    }

    fn inject_mouse_button(&self, _button: u8, _pressed: bool) -> Result<(), PlatformError> {
        Err(PlatformError::NotSupported)
    }

    fn get_clipboard(&self) -> Result<String, PlatformError> {
        Err(PlatformError::NotSupported)
    }

    fn set_clipboard(&self, _content: &str) -> Result<(), PlatformError> {
        Err(PlatformError::NotSupported)
    }

    fn shutdown(&self) -> Result<(), PlatformError> {
        log::info!("Windows input system shutdown (stub)");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_windows_input_creation() {
        let input = WindowsInput::new();
        assert!(input.is_ok());
    }
}
