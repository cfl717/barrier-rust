//! Linux Platform Implementation
//!
//! This module provides Linux-specific input handling via X11/Wayland.

use super::{PlatformError, PlatformInput};
use std::ffi::CString;
use std::sync::atomic::{AtomicBool, Ordering};

/// Display server type detection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayServer {
    X11,
    Wayland,
    Unknown,
}

impl DisplayServer {
    /// Detect the current display server at runtime
    pub fn detect() -> Self {
        // Check for Wayland first (higher priority)
        if std::env::var("WAYLAND_DISPLAY").is_ok() {
            log::info!("Detected Wayland display server");
            return DisplayServer::Wayland;
        }
        
        // Fallback to X11
        if std::env::var("DISPLAY").is_ok() {
            log::info!("Detected X11 display server");
            return DisplayServer::X11;
        }
        
        log::warn!("No display server detected");
        DisplayServer::Unknown
    }
}

/// Linux input handler
pub struct LinuxInput {
    // Display server type (detected at runtime)
    display_server: DisplayServer,
    
    // X11 display connection
    #[cfg(feature = "x11")]
    display: *mut std::os::raw::c_void,
    
    // Wayland connection (alternative)
    #[cfg(feature = "wayland")]
    wayland_connection: Option<*mut std::os::raw::c_void>,
    
    initialized: bool,
    keyboard_grabbed: AtomicBool,
    mouse_grabbed: AtomicBool,
}

// X11 constants
const KEY_REPEAT: i32 = 0;
const BUTTON_LEFT: u32 = 1;
const BUTTON_RIGHT: u32 = 3;
const BUTTON_MIDDLE: u32 = 2;

impl LinuxInput {
    /// Create a new Linux input handler
    pub fn new() -> Result<Self, PlatformError> {
        let display_server = DisplayServer::detect();
        
        Ok(Self {
            display_server,
            #[cfg(feature = "x11")]
            display: std::ptr::null_mut(),
            #[cfg(feature = "wayland")]
            wayland_connection: None,
            initialized: false,
            keyboard_grabbed: AtomicBool::new(false),
            mouse_grabbed: AtomicBool::new(false),
        })
    }
    
    /// Check if running under Wayland
    #[cfg(feature = "wayland")]
    fn is_wayland(&self) -> bool {
        self.display_server == DisplayServer::Wayland
    }
    
    /// Check if running under X11
    #[cfg(feature = "x11")]
    fn is_x11(&self) -> bool {
        self.display_server == DisplayServer::X11
    }
    
    // ==================== X11 Implementation Methods ====================
    
    /// X11 keyboard capture implementation
    #[cfg(feature = "x11")]
    fn capture_keyboard_x11(&self) -> Result<(), PlatformError> {
        use x11rb::protocol::xproto::*;
        use x11rb::connection::Connection;
        use x11rb::errors::ConnectionError;
        
        // Open X11 connection
        let (conn, screen_num) = x11rb::connect(None)
            .map_err(|e| PlatformError::DisplayError(format!("Failed to connect to X11: {}", e)))?;
        
        let root_window = conn.setup().roots[screen_num].root;
        
        // Try to grab keyboard
        let result = conn.grab_keyboard(
            false, // owner_events
            root_window,
            x11rb::CURRENT_TIME,
            GrabMode::ASYNC,
            GrabMode::ASYNC,
        );
        
        match result {
            Ok(reply) => {
                if reply.status() == GrabStatus::SUCCESS {
                    log::info!("X11 keyboard grabbed successfully");
                    Ok(())
                } else {
                    Err(PlatformError::PermissionDenied(
                        format!("Failed to grab keyboard: {:?}", reply.status())
                    ))
                }
            }
            Err(e) => Err(PlatformError::DisplayError(
                format!("X11 keyboard grab error: {}", e)
            )),
        }
    }
    
    /// X11 keyboard injection implementation
    #[cfg(feature = "x11")]
    fn inject_keyboard_x11(&self, key_code: u16, pressed: bool) -> Result<(), PlatformError> {
        use x11rb::protocol::xtest::*;
        use x11rb::connection::Connection;
        
        let (conn, _) = x11rb::connect(None)
            .map_err(|e| PlatformError::DisplayError(format!("Failed to connect to X11: {}", e)))?;
        
        // Use XTest extension to simulate key event
        conn.fake_key_input(
            pressed,
            key_code as _,
            0, // time offset
            0, // device id
        )
        .map_err(|e| PlatformError::DisplayError(format!("X11 key injection failed: {}", e)))?;
        
        conn.flush()
            .map_err(|e| PlatformError::DisplayError(format!("X11 flush failed: {}", e)))?;
        
        log::debug!("X11 key {} {}", key_code, if pressed { "pressed" } else { "released" });
        Ok(())
    }
    
    /// X11 mouse capture implementation
    #[cfg(feature = "x11")]
    fn capture_mouse_x11(&self) -> Result<(), PlatformError> {
        use x11rb::protocol::xproto::*;
        use x11rb::connection::Connection;
        
        let (conn, screen_num) = x11rb::connect(None)
            .map_err(|e| PlatformError::DisplayError(format!("Failed to connect to X11: {}", e)))?;
        
        let root_window = conn.setup().roots[screen_num].root;
        
        // Grab pointer
        let result = conn.grab_pointer(
            false, // owner_events
            root_window,
            EventMask::BUTTON_PRESS | EventMask::BUTTON_RELEASE | EventMask::POINTER_MOTION,
            GrabMode::ASYNC,
            GrabMode::ASYNC,
            0, // confine_to
            0, // cursor
            x11rb::CURRENT_TIME,
        );
        
        match result {
            Ok(reply) => {
                if reply.status() == GrabStatus::SUCCESS {
                    log::info!("X11 mouse grabbed successfully");
                    Ok(())
                } else {
                    Err(PlatformError::PermissionDenied(
                        format!("Failed to grab mouse: {:?}", reply.status())
                    ))
                }
            }
            Err(e) => Err(PlatformError::DisplayError(
                format!("X11 mouse grab error: {}", e)
            )),
        }
    }
    
    /// X11 mouse movement injection implementation
    #[cfg(feature = "x11")]
    fn inject_mouse_move_x11(&self, dx: i16, dy: i16) -> Result<(), PlatformError> {
        use x11rb::protocol::xtest::*;
        use x11rb::connection::Connection;
        
        let (conn, _) = x11rb::connect(None)
            .map_err(|e| PlatformError::DisplayError(format!("Failed to connect to X11: {}", e)))?;
        
        conn.fake_relative_motion_input(
            dx as _,
            dy as _,
            0, // time offset
            0, // device id
        )
        .map_err(|e| PlatformError::DisplayError(format!("X11 mouse move failed: {}", e)))?;
        
        conn.flush()
            .map_err(|e| PlatformError::DisplayError(format!("X11 flush failed: {}", e)))?;
        
        log::debug!("X11 mouse moved by ({}, {})", dx, dy);
        Ok(())
    }
    
    /// X11 mouse button injection implementation
    #[cfg(feature = "x11")]
    fn inject_mouse_button_x11(&self, button: u8, pressed: bool) -> Result<(), PlatformError> {
        use x11rb::protocol::xtest::*;
        use x11rb::connection::Connection;
        
        // Map Barrier button codes to X11 button numbers
        let x11_button = match button {
            1 => 1, // Left
            2 => 3, // Right
            3 => 2, // Middle
            b => {
                log::warn!("Unknown button code: {}, defaulting to left", b);
                1
            }
        };
        
        let (conn, _) = x11rb::connect(None)
            .map_err(|e| PlatformError::DisplayError(format!("Failed to connect to X11: {}", e)))?;
        
        conn.fake_button_input(
            pressed,
            x11_button,
            0, // time offset
            0, // device id
        )
        .map_err(|e| PlatformError::DisplayError(format!("X11 button injection failed: {}", e)))?;
        
        conn.flush()
            .map_err(|e| PlatformError::DisplayError(format!("X11 flush failed: {}", e)))?;
        
        log::debug!("X11 mouse button {} {}", button, if pressed { "pressed" } else { "released" });
        Ok(())
    }
    
    // ==================== Wayland Fallback Methods (Phase 1) ====================
    
    /// Wayland keyboard capture fallback (Phase 1: uses X11 compatibility)
    #[cfg(feature = "wayland")]
    fn capture_keyboard_x11_fallback(&self) -> Result<(), PlatformError> {
        #[cfg(feature = "x11")]
        {
            log::info!("Wayland falling back to X11 for keyboard capture");
            return self.capture_keyboard_x11();
        }
        
        #[cfg(not(feature = "x11"))]
        {
            log::warn!("Wayland without X11 fallback - using external tools");
            // Note: Full Wayland native support will be implemented in Phase 2
            Err(PlatformError::NotSupported)
        }
    }
    
    /// Wayland keyboard injection fallback (Phase 1)
    #[cfg(feature = "wayland")]
    fn inject_keyboard_x11_fallback(&self, key_code: u16, pressed: bool) -> Result<(), PlatformError> {
        #[cfg(feature = "x11")]
        {
            return self.inject_keyboard_x11(key_code, pressed);
        }
        
        #[cfg(not(feature = "x11"))]
        {
            Err(PlatformError::NotSupported)
        }
    }
    
    /// Wayland mouse capture fallback (Phase 1)
    #[cfg(feature = "wayland")]
    fn capture_mouse_x11_fallback(&self) -> Result<(), PlatformError> {
        #[cfg(feature = "x11")]
        {
            return self.capture_mouse_x11();
        }
        
        #[cfg(not(feature = "x11"))]
        {
            Err(PlatformError::NotSupported)
        }
    }
    
    /// Wayland mouse movement injection fallback (Phase 1)
    #[cfg(feature = "wayland")]
    fn inject_mouse_move_x11_fallback(&self, dx: i16, dy: i16) -> Result<(), PlatformError> {
        #[cfg(feature = "x11")]
        {
            return self.inject_mouse_move_x11(dx, dy);
        }
        
        #[cfg(not(feature = "x11"))]
        {
            Err(PlatformError::NotSupported)
        }
    }
    
    /// Wayland mouse button injection fallback (Phase 1)
    #[cfg(feature = "wayland")]
    fn inject_mouse_button_x11_fallback(&self, button: u8, pressed: bool) -> Result<(), PlatformError> {
        #[cfg(feature = "x11")]
        {
            return self.inject_mouse_button_x11(button, pressed);
        }
        
        #[cfg(not(feature = "x11"))]
        {
            Err(PlatformError::NotSupported)
        }
    }
    
    /// Open X11 display connection
    #[cfg(feature = "x11")]
    fn open_x11_display(&mut self) -> Result<(), PlatformError> {
        use x11::xlib::*;
        
        unsafe {
            XInitThreads();
            self.display = XOpenDisplay(std::ptr::null());
            
            if self.display.is_null() {
                return Err(PlatformError::DisplayError(
                    "Failed to open X11 display".to_string()
                ));
            }
        }
        
        log::info!("X11 display opened successfully");
        Ok(())
    }
    
    /// Close X11 display connection
    #[cfg(feature = "x11")]
    fn close_x11_display(&mut self) {
        use x11::xlib::*;
        
        unsafe {
            if !self.display.is_null() {
                XCloseDisplay(self.display as *mut _);
                self.display = std::ptr::null_mut();
                log::info!("X11 display closed");
            }
        }
    }
}

impl PlatformInput for LinuxInput {
    fn initialize(&self) -> Result<(), PlatformError> {
        // For now, we'll use a simple approach without persistent connections
        // In production, you'd want to maintain X11/Wayland connections
        log::info!("Linux input system initialized");
        Ok(())
    }

    fn capture_keyboard(&self) -> Result<(), PlatformError> {
        #[cfg(feature = "wayland")]
        if self.is_wayland() {
            log::warn!("Wayland keyboard capture not yet implemented (Phase 1)");
            log::info!("Falling back to external tools or X11 compatibility layer");
            // Phase 1: Use existing X11 fallback or external tools
            // Full Wayland native support will be implemented in Phase 2
            return self.capture_keyboard_x11_fallback();
        }
        
        #[cfg(feature = "x11")]
        if self.is_x11() {
            return self.capture_keyboard_x11();
        }
        
        Err(PlatformError::NotSupported)
    }

    fn inject_keyboard(&self, key_code: u16, pressed: bool) -> Result<(), PlatformError> {
        #[cfg(feature = "wayland")]
        if self.is_wayland() {
            log::warn!("Wayland keyboard injection not yet implemented (Phase 1)");
            // Phase 1: Fallback to X11 compatibility or external tools
            return self.inject_keyboard_x11_fallback(key_code, pressed);
        }
        
        #[cfg(feature = "x11")]
        if self.is_x11() {
            return self.inject_keyboard_x11(key_code, pressed);
        }
        
        Err(PlatformError::NotSupported)
    }

    fn capture_mouse(&self) -> Result<(), PlatformError> {
        #[cfg(feature = "wayland")]
        if self.is_wayland() {
            log::warn!("Wayland mouse capture not yet implemented (Phase 1)");
            // Phase 1: Fallback
            return self.capture_mouse_x11_fallback();
        }
        
        #[cfg(feature = "x11")]
        if self.is_x11() {
            return self.capture_mouse_x11();
        }
        
        Err(PlatformError::NotSupported)
    }

    fn inject_mouse_move(&self, dx: i16, dy: i16) -> Result<(), PlatformError> {
        #[cfg(feature = "wayland")]
        if self.is_wayland() {
            log::warn!("Wayland mouse movement injection not yet implemented (Phase 1)");
            return self.inject_mouse_move_x11_fallback(dx, dy);
        }
        
        #[cfg(feature = "x11")]
        if self.is_x11() {
            return self.inject_mouse_move_x11(dx, dy);
        }
        
        Err(PlatformError::NotSupported)
    }

    fn inject_mouse_button(&self, button: u8, pressed: bool) -> Result<(), PlatformError> {
        #[cfg(feature = "wayland")]
        if self.is_wayland() {
            log::warn!("Wayland mouse button injection not yet implemented (Phase 1)");
            return self.inject_mouse_button_x11_fallback(button, pressed);
        }
        
        #[cfg(feature = "x11")]
        if self.is_x11() {
            return self.inject_mouse_button_x11(button, pressed);
        }
        
        Err(PlatformError::NotSupported)
    }

    fn get_clipboard(&self) -> Result<String, PlatformError> {
        use std::process::Command;
        
        // Try Wayland first if detected
        #[cfg(feature = "wayland")]
        if self.is_wayland() {
            log::debug!("Using wl-paste for Wayland clipboard");
            if let Ok(out) = Command::new("wl-paste").output() {
                if out.status.success() {
                    return Ok(String::from_utf8_lossy(&out.stdout).to_string());
                }
            }
            // Fallback to X11 if wl-paste fails
            #[cfg(feature = "x11")]
            {
                log::debug!("wl-paste failed, trying xclip");
            }
        }
        
        // Try X11 clipboard tools
        #[cfg(feature = "x11")]
        {
            log::debug!("Using xclip for X11 clipboard");
            if let Ok(out) = Command::new("xclip")
                .args(["-selection", "clipboard", "-o"])
                .output()
            {
                if out.status.success() {
                    return Ok(String::from_utf8_lossy(&out.stdout).to_string());
                }
            }
        }
        
        // If all methods fail
        Err(PlatformError::NotSupported)
    }

    fn set_clipboard(&self, content: &str) -> Result<(), PlatformError> {
        use std::process::Command;
        use std::io::Write;
        
        // Try Wayland first if detected
        #[cfg(feature = "wayland")]
        if self.is_wayland() {
            log::debug!("Using wl-copy for Wayland clipboard");
            let mut child = Command::new("wl-copy")
                .stdin(std::process::Stdio::piped())
                .spawn();
            
            if let Ok(mut child) = child {
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(content.as_bytes());
                }
                let _ = child.wait();
                return Ok(());
            }
            // Fallback to X11 if wl-copy fails
            #[cfg(feature = "x11")]
            {
                log::debug!("wl-copy failed, trying xclip");
            }
        }
        
        // Try X11 clipboard tools
        #[cfg(feature = "x11")]
        {
            log::debug!("Using xclip for X11 clipboard");
            let mut child = Command::new("xclip")
                .args(["-selection", "clipboard", "-i"])
                .stdin(std::process::Stdio::piped())
                .spawn();
            
            if let Ok(mut child) = child {
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(content.as_bytes());
                }
                let _ = child.wait();
                return Ok(());
            }
        }
        
        Err(PlatformError::NotSupported)
    }

    fn shutdown(&self) -> Result<(), PlatformError> {
        log::info!("Linux input system shutdown");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linux_input_creation() {
        let input = LinuxInput::new();
        assert!(input.is_ok());
    }

    #[test]
    fn test_platform_trait_methods_exist() {
        let input = LinuxInput::new().unwrap();
        let _ = input.initialize();
    }
}
