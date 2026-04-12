//! macOS Platform Implementation
//!
//! This module provides macOS-specific input handling using Quartz Event Services.

use super::{PlatformError, PlatformInput};
use std::ffi::CString;

/// macOS input handler
pub struct MacOsInput {
    initialized: bool,
}

// Map Barrier button codes to CGMouseButton
fn barrier_button_to_cg(button: u8) -> i32 {
    match button {
        1 => 0, // kCGMouseButtonLeft
        2 => 2, // kCGMouseButtonRight  
        3 => 1, // kCGMouseButtonCenter
        _ => 0,
    }
}

impl MacOsInput {
    /// Create a new macOS input handler
    pub fn new() -> Result<Self, PlatformError> {
        Ok(Self { initialized: false })
    }
}

impl PlatformInput for MacOsInput {
    fn initialize(&self) -> Result<(), PlatformError> {
        log::info!("macOS input system initialized");
        
        // Note: On macOS, accessibility permissions are required for input capture/injection
        // The user must grant permission in System Preferences > Security & Privacy > Privacy > Accessibility
        
        Ok(())
    }

    fn capture_keyboard(&self) -> Result<(), PlatformError> {
        // On macOS, we need to use CGEventTap or IOKit for keyboard capture
        // This requires accessibility permissions
        
        log::warn!("Keyboard capture on macOS requires Accessibility permissions");
        log::info!("Grant permission in System Preferences > Security & Privacy > Privacy > Accessibility");
        
        // Full implementation would use CGEventTapCreate with kCGSessionEventTap
        // This is complex and requires running in an app bundle or with proper entitlements
        
        Err(PlatformError::PermissionDenied(
            "Keyboard capture requires Accessibility permissions".to_string()
        ))
    }

    fn inject_keyboard(&self, key_code: u16, pressed: bool) -> Result<(), PlatformError> {
        // Use CGEvent for keyboard injection
        // This also requires accessibility permissions but is more reliable than capture
        
        #[cfg(target_os = "macos")]
        {
            use core_graphics::event::{CGEvent, CGEventType, CGKeyCode};
            use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
            
            let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
                .map_err(|_| PlatformError::InitializationFailed(
                    "Failed to create event source".to_string()
                ))?;
            
            let event_type = if pressed {
                CGEventType::KeyDown
            } else {
                CGEventType::KeyUp
            };
            
            let event = CGEvent::new_keyboard_event(source, key_code as CGKeyCode, pressed)
                .map_err(|_| PlatformError::InitializationFailed(
                    "Failed to create keyboard event".to_string()
                ))?;
            
            event.post(core_graphics::event::CGEventTapLocation::HID);
            
            log::debug!("Key {} {}", key_code, if pressed { "pressed" } else { "released" });
            return Ok(());
        }
        
        #[cfg(not(target_os = "macos"))]
        {
            Err(PlatformError::NotSupported)
        }
    }

    fn capture_mouse(&self) -> Result<(), PlatformError> {
        log::warn!("Mouse capture on macOS requires Accessibility permissions");
        log::info!("Use CGEventTap for mouse event capture");
        
        Err(PlatformError::PermissionDenied(
            "Mouse capture requires Accessibility permissions".to_string()
        ))
    }

    fn inject_mouse_move(&self, dx: i16, dy: i16) -> Result<(), PlatformError> {
        #[cfg(target_os = "macos")]
        {
            use core_graphics::event::{CGEvent, CGEventType};
            use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
            use core_graphics::geometry::CGPoint;
            
            let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
                .map_err(|_| PlatformError::InitializationFailed(
                    "Failed to create event source".to_string()
                ))?;
            
            // Get current mouse position and add delta
            let current_location = CGEvent::get_location();
            let new_location = CGPoint {
                x: current_location.x + dx as f64,
                y: current_location.y + dy as f64,
            };
            
            let event = CGEvent::new_mouse_event(
                source,
                CGEventType::MouseMoved,
                new_location,
                core_graphics::event::CGMouseButton::Left,
            )
            .map_err(|_| PlatformError::InitializationFailed(
                "Failed to create mouse move event".to_string()
            ))?;
            
            event.post(core_graphics::event::CGEventTapLocation::HID);
            
            log::debug!("Mouse moved by ({}, {})", dx, dy);
            return Ok(());
        }
        
        #[cfg(not(target_os = "macos"))]
        {
            Err(PlatformError::NotSupported)
        }
    }

    fn inject_mouse_button(&self, button: u8, pressed: bool) -> Result<(), PlatformError> {
        #[cfg(target_os = "macos")]
        {
            use core_graphics::event::{CGEvent, CGEventType, CGMouseButton};
            use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
            
            let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
                .map_err(|_| PlatformError::InitializationFailed(
                    "Failed to create event source".to_string()
                ))?;
            
            let cg_button = match barrier_button_to_cg(button) {
                0 => CGMouseButton::Left,
                1 => CGMouseButton::Center,
                2 => CGMouseButton::Right,
                _ => CGMouseButton::Left,
            };
            
            let event_type = if pressed {
                match cg_button {
                    CGMouseButton::Left => CGEventType::LeftMouseDown,
                    CGMouseButton::Center => CGEventType::OtherMouseDown,
                    CGMouseButton::Right => CGEventType::RightMouseDown,
                }
            } else {
                match cg_button {
                    CGMouseButton::Left => CGEventType::LeftMouseUp,
                    CGMouseButton::Center => CGEventType::OtherMouseUp,
                    CGMouseButton::Right => CGEventType::RightMouseUp,
                }
            };
            
            let location = CGEvent::get_location();
            
            let event = CGEvent::new_mouse_event(
                source,
                event_type,
                location,
                cg_button,
            )
            .map_err(|_| PlatformError::InitializationFailed(
                "Failed to create mouse button event".to_string()
            ))?;
            
            event.post(core_graphics::event::CGEventTapLocation::HID);
            
            log::debug!("Mouse button {} {}", button, if pressed { "pressed" } else { "released" });
            return Ok(());
        }
        
        #[cfg(not(target_os = "macos"))]
        {
            Err(PlatformError::NotSupported)
        }
    }

    fn get_clipboard(&self) -> Result<String, PlatformError> {
        #[cfg(target_os = "macos")]
        {
            use objc::runtime::{Class, Object};
            use objc::{msg_send, sel, sel_impl};
            use objc_foundation::{INSString, NSObject, NSString};
            
            unsafe {
                let pasteboard_class = Class::get("NSPasteboard").unwrap();
                let pasteboard: *mut Object = msg_send![pasteboard_class, generalPasteboard];
                
                if pasteboard.is_null() {
                    return Err(PlatformError::InitializationFailed(
                        "Failed to get pasteboard".to_string()
                    ));
                }
                
                let string_class = Class::get("NSString").unwrap();
                let ns_string_type: *mut Object = msg_send![string_class, stringWithString: NSString::from_str("public.utf-8-plain-text")];
                
                let content: *mut Object = msg_send![pasteboard, stringForType: ns_string_type];
                
                if !content.is_null() {
                    let utf8_string: *const i8 = msg_send![content, UTF8String];
                    if !utf8_string.is_null() {
                        let rust_string = CString::from_raw(utf8_string as *mut _).into_string()
                            .map_err(|_| PlatformError::NotSupported)?;
                        return Ok(rust_string);
                    }
                }
            }
            
            Err(PlatformError::NotSupported)
        }
        
        #[cfg(not(target_os = "macos"))]
        {
            Err(PlatformError::NotSupported)
        }
    }

    fn set_clipboard(&self, content: &str) -> Result<(), PlatformError> {
        #[cfg(target_os = "macos")]
        {
            use objc::runtime::{Class, Object};
            use objc::{msg_send, sel, sel_impl};
            use objc_foundation::NSString;
            
            unsafe {
                let pasteboard_class = Class::get("NSPasteboard").unwrap();
                let pasteboard: *mut Object = msg_send![pasteboard_class, generalPasteboard];
                
                if pasteboard.is_null() {
                    return Err(PlatformError::InitializationFailed(
                        "Failed to get pasteboard".to_string()
                    ));
                }
                
                // Clear the pasteboard
                let _: () = msg_send![pasteboard, clearContents];
                
                // Create NSArray with the string type
                let string_class = Class::get("NSString").unwrap();
                let ns_string_type: *mut Object = msg_send![string_class, stringWithString: NSString::from_str("public.utf-8-plain-text")];
                
                let array_class = Class::get("NSArray").unwrap();
                let types_array: *mut Object = msg_send![array_class, arrayWithObject: ns_string_type];
                
                // Write to pasteboard
                let ns_string = NSString::from_str(content);
                let result: bool = msg_send![pasteboard, writeObjects: &[ns_string]];
                
                if result {
                    log::debug!("Clipboard set successfully");
                    return Ok(());
                }
            }
            
            Err(PlatformError::InitializationFailed(
                "Failed to write to clipboard".to_string()
            ))
        }
        
        #[cfg(not(target_os = "macos"))]
        {
            Err(PlatformError::NotSupported)
        }
    }

    fn shutdown(&self) -> Result<(), PlatformError> {
        log::info!("macOS input system shutdown");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macos_input_creation() {
        let input = MacOsInput::new();
        assert!(input.is_ok());
    }

    #[test]
    fn test_platform_trait_methods_exist() {
        let input = MacOsInput::new().unwrap();
        let _ = input.initialize();
    }
}
