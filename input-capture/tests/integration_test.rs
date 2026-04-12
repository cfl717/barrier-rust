//! Integration tests for barrier-input-capture
//!
//! These tests verify the functionality of the input capture system.

#[cfg(test)]
mod tests {
    use barrier_input_capture::{InputCapture, PlatformError};

    #[test]
    #[ignore = "Requires platform-specific setup"]
    fn test_create_input_capture() {
        let result = InputCapture::new();
        assert!(result.is_ok() || matches!(result, Err(PlatformError::NotSupported)));
    }

    #[test]
    #[ignore = "Requires active display server"]
    fn test_initialize() {
        let capture = match InputCapture::new() {
            Ok(c) => c,
            Err(_) => return, // Skip if platform not supported
        };

        let result = capture.initialize();
        assert!(result.is_ok());
    }

    #[test]
    #[ignore = "Requires keyboard permissions"]
    fn test_keyboard_injection() {
        let capture = match InputCapture::new() {
            Ok(c) => c,
            Err(_) => return,
        };

        // Test injecting a key press (key code 65 = 'A')
        let result = capture.inject_keyboard(65, true);
        assert!(result.is_ok());

        // Test injecting a key release
        let result = capture.inject_keyboard(65, false);
        assert!(result.is_ok());
    }

    #[test]
    #[ignore = "Requires mouse permissions"]
    fn test_mouse_injection() {
        let capture = match InputCapture::new() {
            Ok(c) => c,
            Err(_) => return,
        };

        // Test mouse movement
        let result = capture.inject_mouse_move(10, 10);
        assert!(result.is_ok());

        // Test mouse button press (button 1 = left click)
        let result = capture.inject_mouse_button(1, true);
        assert!(result.is_ok());

        // Test mouse button release
        let result = capture.inject_mouse_button(1, false);
        assert!(result.is_ok());
    }

    #[test]
    #[ignore = "Requires clipboard access"]
    fn test_clipboard() {
        let capture = match InputCapture::new() {
            Ok(c) => c,
            Err(_) => return,
        };

        let test_content = "Test clipboard content";
        
        // Set clipboard
        let result = capture.set_clipboard(test_content);
        assert!(result.is_ok());

        // Get clipboard
        let result = capture.get_clipboard();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), test_content);
    }
}

/// Mock implementation for testing on unsupported platforms
#[cfg(feature = "test-mock")]
pub mod mock {
    use barrier_input_capture::{PlatformError, PlatformInput};

    pub struct MockInput;

    impl MockInput {
        pub fn new() -> Self {
            Self
        }
    }

    impl PlatformInput for MockInput {
        fn initialize(&self) -> Result<(), PlatformError> {
            Ok(())
        }

        fn capture_keyboard(&self) -> Result<(), PlatformError> {
            Ok(())
        }

        fn inject_keyboard(&self, _key_code: u16, _pressed: bool) -> Result<(), PlatformError> {
            Ok(())
        }

        fn capture_mouse(&self) -> Result<(), PlatformError> {
            Ok(())
        }

        fn inject_mouse_move(&self, _dx: i16, _dy: i16) -> Result<(), PlatformError> {
            Ok(())
        }

        fn inject_mouse_button(&self, _button: u8, _pressed: bool) -> Result<(), PlatformError> {
            Ok(())
        }

        fn get_clipboard(&self) -> Result<String, PlatformError> {
            Ok(String::new())
        }

        fn set_clipboard(&self, _content: &str) -> Result<(), PlatformError> {
            Ok(())
        }

        fn shutdown(&self) -> Result<(), PlatformError> {
            Ok(())
        }
    }

    #[test]
    fn test_mock_implementation() {
        let mock = MockInput::new();
        assert!(mock.initialize().is_ok());
        assert!(mock.capture_keyboard().is_ok());
        assert!(mock.capture_mouse().is_ok());
        assert!(mock.inject_keyboard(65, true).is_ok());
        assert!(mock.inject_mouse_move(10, 10).is_ok());
        assert!(mock.inject_mouse_button(1, true).is_ok());
        assert!(mock.get_clipboard().is_ok());
        assert!(mock.set_clipboard("test").is_ok());
        assert!(mock.shutdown().is_ok());
    }
}
