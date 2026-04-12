//! Linux Platform Implementation
//!
//! This module provides Linux-specific input handling via X11/Wayland.

use super::{PlatformError, PlatformInput};
use std::ffi::CString;
use std::sync::atomic::{AtomicBool, Ordering};

/// Linux input handler
pub struct LinuxInput {
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
        Ok(Self {
            #[cfg(feature = "x11")]
            display: std::ptr::null_mut(),
            #[cfg(feature = "wayland")]
            wayland_connection: None,
            initialized: false,
            keyboard_grabbed: AtomicBool::new(false),
            mouse_grabbed: AtomicBool::new(false),
        })
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
        #[cfg(feature = "x11")]
        {
            use x11::xlib::*;
            use std::ptr;
            
            // Note: This requires running with appropriate permissions
            // In practice, you may need to use evdev or run as root
            unsafe {
                let display = XOpenDisplay(ptr::null());
                if display.is_null() {
                    return Err(PlatformError::DisplayError(
                        "Cannot open X display for keyboard grab".to_string()
                    ));
                }
                
                let root_window = XDefaultRootWindow(display);
                
                // Try to grab keyboard
                let result = XGrabKeyboard(
                    display,
                    root_window,
                    False as _,
                    GrabModeAsync,
                    GrabModeAsync,
                    CurrentTime,
                );
                
                XCloseDisplay(display);
                
                if result == GrabSuccess as i32 {
                    log::info!("Keyboard grabbed successfully");
                    return Ok(());
                } else {
                    return Err(PlatformError::PermissionDenied(
                        format!("Failed to grab keyboard: {}", result)
                    ));
                }
            }
        }
        
        #[cfg(not(feature = "x11"))]
        {
            // Fallback: try evdev (requires root)
            log::warn!("X11 feature not enabled, keyboard capture requires evdev access");
            return Err(PlatformError::PermissionDenied(
                "Keyboard capture requires X11 or root access to /dev/input/event*".to_string()
            ));
        }
    }

    fn inject_keyboard(&self, key_code: u16, pressed: bool) -> Result<(), PlatformError> {
        #[cfg(feature = "x11")]
        {
            use x11::xtest::*;
            use x11::xlib::*;
            use std::ptr;
            
            unsafe {
                let display = XOpenDisplay(ptr::null());
                if display.is_null() {
                    return Err(PlatformError::DisplayError(
                        "Cannot open X display for key injection".to_string()
                    ));
                }
                
                // Use XTest to simulate key press/release
                let keycode = key_code as _;
                XTestFakeKeyEvent(display, keycode, pressed as _, CurrentTime);
                XFlush(display);
                XCloseDisplay(display);
                
                log::debug!("Key {} {}", key_code, if pressed { "pressed" } else { "released" });
                return Ok(());
            }
        }
        
        #[cfg(not(feature = "x11"))]
        {
            log::error!("X11 not available for keyboard injection");
            Err(PlatformError::NotSupported)
        }
    }

    fn capture_mouse(&self) -> Result<(), PlatformError> {
        #[cfg(feature = "x11")]
        {
            use x11::xlib::*;
            use std::ptr;
            
            unsafe {
                let display = XOpenDisplay(ptr::null());
                if display.is_null() {
                    return Err(PlatformError::DisplayError(
                        "Cannot open X display for mouse grab".to_string()
                    ));
                }
                
                let root_window = XDefaultRootWindow(display);
                
                // Grab mouse pointer
                let result = XGrabPointer(
                    display,
                    root_window,
                    False as _,
                    ButtonPressMask | ButtonReleaseMask | PointerMotionMask,
                    GrabModeAsync,
                    GrabModeAsync,
                    0,
                    0,
                    CurrentTime,
                );
                
                XCloseDisplay(display);
                
                if result == GrabSuccess as i32 {
                    log::info!("Mouse grabbed successfully");
                    return Ok(());
                } else {
                    return Err(PlatformError::PermissionDenied(
                        format!("Failed to grab mouse: {}", result)
                    ));
                }
            }
        }
        
        #[cfg(not(feature = "x11"))]
        {
            log::warn!("X11 not available for mouse capture");
            Err(PlatformError::NotSupported)
        }
    }

    fn inject_mouse_move(&self, dx: i16, dy: i16) -> Result<(), PlatformError> {
        #[cfg(feature = "x11")]
        {
            use x11::xtest::*;
            use x11::xlib::*;
            use std::ptr;
            
            unsafe {
                let display = XOpenDisplay(ptr::null());
                if display.is_null() {
                    return Err(PlatformError::DisplayError(
                        "Cannot open X display for mouse movement".to_string()
                    ));
                }
                
                // Use XTest to simulate relative mouse movement
                XTestFakeRelativeMotionEvent(display, dx as _, dy as _, 0, CurrentTime);
                XFlush(display);
                XCloseDisplay(display);
                
                log::debug!("Mouse moved by ({}, {})", dx, dy);
                return Ok(());
            }
        }
        
        #[cfg(not(feature = "x11"))]
        {
            log::error!("X11 not available for mouse movement injection");
            Err(PlatformError::NotSupported)
        }
    }

    fn inject_mouse_button(&self, button: u8, pressed: bool) -> Result<(), PlatformError> {
        #[cfg(feature = "x11")]
        {
            use x11::xtest::*;
            use x11::xlib::*;
            use std::ptr;
            
            // Map Barrier button codes to X11 button numbers
            let x11_button = match button {
                1 => BUTTON_LEFT,
                2 => BUTTON_RIGHT,
                3 => BUTTON_MIDDLE,
                b => {
                    log::warn!("Unknown button code: {}", b);
                    BUTTON_LEFT // Default to left button
                }
            };
            
            unsafe {
                let display = XOpenDisplay(ptr::null());
                if display.is_null() {
                    return Err(PlatformError::DisplayError(
                        "Cannot open X display for mouse button".to_string()
                    ));
                }
                
                XTestFakeButtonEvent(display, x11_button, pressed as _, CurrentTime);
                XFlush(display);
                XCloseDisplay(display);
                
                log::debug!("Mouse button {} {}", button, if pressed { "pressed" } else { "released" });
                return Ok(());
            }
        }
        
        #[cfg(not(feature = "x11"))]
        {
            log::error!("X11 not available for mouse button injection");
            Err(PlatformError::NotSupported)
        }
    }

    fn get_clipboard(&self) -> Result<String, PlatformError> {
        #[cfg(feature = "x11")]
        {
            use x11::xlib::*;
            use std::ptr;
            use std::slice;
            
            unsafe {
                let display = XOpenDisplay(ptr::null());
                if display.is_null() {
                    return Err(PlatformError::DisplayError(
                        "Cannot open X display for clipboard".to_string()
                    ));
                }
                
                let window = XDefaultRootWindow(display);
                
                // Get CLIPBOARD selection
                let clipboard_atom = XInternAtom(display, CString::new("CLIPBOARD")?.as_ptr(), False as _);
                let utf8_atom = XInternAtom(display, CString::new("UTF8_STRING")?.as_ptr(), False as _);
                
                // Convert selection to property
                XConvertSelection(display, clipboard_atom, utf8_atom, clipboard_atom, window, CurrentTime);
                XFlush(display);
                
                // Wait for SelectionNotify event (simplified - in production use proper event loop)
                // For now, we'll use a simpler approach with xclip/wl-clipboard
                
                XCloseDisplay(display);
            }
            
            // Fallback: try using xclip command
            use std::process::Command;
            let output = Command::new("xclip")
                .args(["-selection", "clipboard", "-o"])
                .output();
            
            match output {
                Ok(out) => {
                    if out.status.success() {
                        return Ok(String::from_utf8_lossy(&out.stdout).to_string());
                    }
                }
                Err(_) => {
                    // Try wl-clipboard for Wayland
                    if let Ok(out) = Command::new("wl-paste").output() {
                        if out.status.success() {
                            return Ok(String::from_utf8_lossy(&out.stdout).to_string());
                        }
                    }
                }
            }
            
            Err(PlatformError::NotSupported)
        }
        
        #[cfg(not(feature = "x11"))]
        {
            // Try command-line tools
            use std::process::Command;
            
            if let Ok(out) = Command::new("xclip")
                .args(["-selection", "clipboard", "-o"])
                .output()
            {
                if out.status.success() {
                    return Ok(String::from_utf8_lossy(&out.stdout).to_string());
                }
            }
            
            if let Ok(out) = Command::new("wl-paste").output() {
                if out.status.success() {
                    return Ok(String::from_utf8_lossy(&out.stdout).to_string());
                }
            }
            
            Err(PlatformError::NotSupported)
        }
    }

    fn set_clipboard(&self, content: &str) -> Result<(), PlatformError> {
        #[cfg(feature = "x11")]
        {
            use std::process::Command;
            
            // Try xclip first
            let mut child = Command::new("xclip")
                .args(["-selection", "clipboard", "-i"])
                .stdin(std::process::Stdio::piped())
                .spawn();
            
            if let Ok(mut child) = child {
                if let Some(mut stdin) = child.stdin.take() {
                    use std::io::Write;
                    let _ = stdin.write_all(content.as_bytes());
                }
                let _ = child.wait();
                return Ok(());
            }
            
            // Try wl-clipboard for Wayland
            let mut child = Command::new("wl-copy")
                .stdin(std::process::Stdio::piped())
                .spawn();
            
            if let Ok(mut child) = child {
                if let Some(mut stdin) = child.stdin.take() {
                    use std::io::Write;
                    let _ = stdin.write_all(content.as_bytes());
                }
                let _ = child.wait();
                return Ok(());
            }
            
            Err(PlatformError::NotSupported)
        }
        
        #[cfg(not(feature = "x11"))]
        {
            use std::process::Command;
            
            // Try xclip
            if let Ok(mut child) = Command::new("xclip")
                .args(["-selection", "clipboard", "-i"])
                .stdin(std::process::Stdio::piped())
                .spawn()
            {
                if let Some(mut stdin) = child.stdin.take() {
                    use std::io::Write;
                    let _ = stdin.write_all(content.as_bytes());
                }
                let _ = child.wait();
                return Ok(());
            }
            
            // Try wl-copy
            if let Ok(mut child) = Command::new("wl-copy")
                .stdin(std::process::Stdio::piped())
                .spawn()
            {
                if let Some(mut stdin) = child.stdin.take() {
                    use std::io::Write;
                    let _ = stdin.write_all(content.as_bytes());
                }
                let _ = child.wait();
                return Ok(());
            }
            
            Err(PlatformError::NotSupported)
        }
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
