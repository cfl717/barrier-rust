//! Barrier Platform Abstraction Layer
//!
//! This module provides platform-specific implementations for input capture and injection.

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "macos")]
pub mod macos;

/// Platform trait for input operations
pub trait PlatformInput {
    /// Initialize the platform input system
    fn initialize(&self) -> Result<(), PlatformError>;

    /// Capture keyboard events
    fn capture_keyboard(&self) -> Result<(), PlatformError>;

    /// Inject keyboard events
    fn inject_keyboard(&self, key_code: u16, pressed: bool) -> Result<(), PlatformError>;

    /// Capture mouse events
    fn capture_mouse(&self) -> Result<(), PlatformError>;

    /// Inject mouse movement
    fn inject_mouse_move(&self, dx: i16, dy: i16) -> Result<(), PlatformError>;

    /// Inject mouse button
    fn inject_mouse_button(&self, button: u8, pressed: bool) -> Result<(), PlatformError>;

    /// Get clipboard content
    fn get_clipboard(&self) -> Result<String, PlatformError>;

    /// Set clipboard content
    fn set_clipboard(&self, content: &str) -> Result<(), PlatformError>;

    /// Shutdown the platform input system
    fn shutdown(&self) -> Result<(), PlatformError>;
}

/// Platform error types
#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    #[error("Platform not supported")]
    NotSupported,

    #[error("Initialization failed: {0}")]
    InitializationFailed(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Display server error: {0}")]
    DisplayError(String),

    #[error("Not initialized")]
    NotInitialized,

    #[error("Not supported: {0}")]
    NotSupportedWithMessage(String),
}

/// Get the appropriate platform implementation
pub fn get_platform() -> Result<Box<dyn PlatformInput>, PlatformError> {
    #[cfg(target_os = "linux")]
    {
        linux::LinuxInput::new().map(|p| Box::new(p) as Box<dyn PlatformInput>)
    }

    #[cfg(target_os = "windows")]
    {
        windows::WindowsInput::new().map(|p| Box::new(p) as Box<dyn PlatformInput>)
    }

    #[cfg(target_os = "macos")]
    {
        macos::MacOsInput::new().map(|p| Box::new(p) as Box<dyn PlatformInput>)
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    {
        Err(PlatformError::NotSupported)
    }
}
