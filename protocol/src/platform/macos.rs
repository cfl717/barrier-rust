//! macOS Platform Implementation
//!
//! This module provides macOS-specific input handling using Quartz Event Services.

use super::{PlatformError, PlatformInput};
use std::ffi::CString;

#[cfg(target_os = "macos")]
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> u8;
    fn CGWarpMouseCursorPosition(newCursorPosition: core_graphics::geometry::CGPoint) -> i32;
}

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

// Convert X11 keycodes (sent by current Linux server path) to macOS CGKeyCode.
fn x11_keycode_to_macos(key_code: u16) -> u16 {
    match key_code {
        9 => 53,   // Escape
        10 => 18,  // 1
        11 => 19,  // 2
        12 => 20,  // 3
        13 => 21,  // 4
        14 => 23,  // 5
        15 => 22,  // 6
        16 => 26,  // 7
        17 => 28,  // 8
        18 => 25,  // 9
        19 => 29,  // 0
        20 => 27,  // -
        21 => 24,  // =
        22 => 51,  // Backspace
        23 => 48,  // Tab
        24 => 12,  // Q
        25 => 13,  // W
        26 => 14,  // E
        27 => 15,  // R
        28 => 17,  // T
        29 => 16,  // Y
        30 => 32,  // U
        31 => 34,  // I
        32 => 31,  // O
        33 => 35,  // P
        34 => 33,  // [
        35 => 30,  // ]
        36 => 36,  // Return
        37 => 59,  // Left Control
        38 => 0,   // A
        39 => 1,   // S
        40 => 2,   // D
        41 => 3,   // F
        42 => 5,   // G
        43 => 4,   // H
        44 => 38,  // J
        45 => 40,  // K
        46 => 37,  // L
        47 => 41,  // ;
        48 => 39,  // '
        49 => 50,  // `
        50 => 56,  // Left Shift
        51 => 42,  // \
        52 => 6,   // Z
        53 => 7,   // X
        54 => 8,   // C
        55 => 9,   // V
        56 => 11,  // B
        57 => 45,  // N
        58 => 46,  // M
        59 => 43,  // ,
        60 => 47,  // .
        61 => 44,  // /
        62 => 60,  // Right Shift
        63 => 67,  // Keypad *
        64 => 58,  // Left Option (Alt)
        65 => 49,  // Space
        66 => 57,  // CapsLock
        67 => 122, // F1
        68 => 120, // F2
        69 => 99,  // F3
        70 => 118, // F4
        71 => 96,  // F5
        72 => 97,  // F6
        73 => 98,  // F7
        74 => 100, // F8
        75 => 101, // F9
        76 => 109, // F10
        77 => 71,  // NumLock/Clear
        79 => 89,  // Keypad 7
        80 => 91,  // Keypad 8
        81 => 92,  // Keypad 9
        82 => 78,  // Keypad -
        83 => 86,  // Keypad 4
        84 => 87,  // Keypad 5
        85 => 88,  // Keypad 6
        86 => 69,  // Keypad +
        87 => 83,  // Keypad 1
        88 => 84,  // Keypad 2
        89 => 85,  // Keypad 3
        90 => 82,  // Keypad 0
        91 => 65,  // Keypad .
        95 => 103, // F11
        96 => 111, // F12
        104 => 76, // Keypad Enter
        105 => 62, // Right Control
        106 => 75, // Keypad /
        108 => 61, // Right Option
        110 => 115, // Home
        111 => 126, // Up
        112 => 116, // PageUp
        113 => 123, // Left
        114 => 124, // Right
        115 => 119, // End
        116 => 125, // Down
        117 => 121, // PageDown
        118 => 114, // Insert
        119 => 117, // ForwardDelete
        127 => 71,  // Pause -> Clear fallback
        133 => 55,  // Left Command (Meta)
        134 => 54,  // Right Command (Meta)
        _ => key_code,
    }
}

impl MacOsInput {
    /// Create a new macOS input handler
    pub fn new() -> Result<Self, PlatformError> {
        Ok(Self { initialized: false })
    }

    #[cfg(target_os = "macos")]
    fn ensure_accessibility_permission() -> Result<(), PlatformError> {
        let trusted = unsafe { AXIsProcessTrusted() != 0 };
        if trusted {
            Ok(())
        } else {
            Err(PlatformError::PermissionDenied(
                "缺少 macOS 无障碍权限（系统设置 -> 隐私与安全性 -> 辅助功能）".to_string(),
            ))
        }
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
        log::info!(
            "Grant permission in System Preferences > Security & Privacy > Privacy > Accessibility"
        );

        // Full implementation would use CGEventTapCreate with kCGSessionEventTap
        // This is complex and requires running in an app bundle or with proper entitlements

        Err(PlatformError::PermissionDenied(
            "Keyboard capture requires Accessibility permissions".to_string(),
        ))
    }

    fn inject_keyboard(&self, key_code: u16, pressed: bool) -> Result<(), PlatformError> {
        // Use CGEvent for keyboard injection
        // This also requires accessibility permissions but is more reliable than capture

        #[cfg(target_os = "macos")]
        {
            Self::ensure_accessibility_permission()?;

            use core_graphics::event::{CGEvent, CGKeyCode};
            use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

            let source =
                CGEventSource::new(CGEventSourceStateID::HIDSystemState).map_err(|_| {
                    PlatformError::InitializationFailed("Failed to create event source".to_string())
                })?;

            let mapped_key = x11_keycode_to_macos(key_code);

            let event = CGEvent::new_keyboard_event(source, mapped_key as CGKeyCode, pressed)
                .map_err(|_| {
                    PlatformError::InitializationFailed(
                        "Failed to create keyboard event".to_string(),
                    )
                })?;

            event.post(core_graphics::event::CGEventTapLocation::HID);

            log::debug!(
                "Key {} -> mac {} {}",
                key_code,
                mapped_key,
                if pressed { "pressed" } else { "released" }
            );
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
            "Mouse capture requires Accessibility permissions".to_string(),
        ))
    }

    fn inject_mouse_move(&self, dx: i16, dy: i16) -> Result<(), PlatformError> {
        #[cfg(target_os = "macos")]
        {
            Self::ensure_accessibility_permission()?;

            use core_graphics::event::CGEvent;
            use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
            use core_graphics::geometry::CGPoint;

            let source =
                CGEventSource::new(CGEventSourceStateID::HIDSystemState).map_err(|_| {
                    PlatformError::InitializationFailed("Failed to create event source".to_string())
                })?;

            // Get current mouse position and add delta
            let dummy_event = CGEvent::new(source.clone()).map_err(|_| {
                PlatformError::InitializationFailed("Failed to create dummy event".to_string())
            })?;
            let current_location = dummy_event.location();
            // Barrier/X11 dy is down-positive; Quartz global coordinates are up-positive.
            let new_location = CGPoint {
                x: current_location.x + dx as f64,
                y: current_location.y - dy as f64,
            };
            let warp_result = unsafe { CGWarpMouseCursorPosition(new_location) };
            if warp_result != 0 {
                return Err(PlatformError::InitializationFailed(format!(
                    "CGWarpMouseCursorPosition failed: {}",
                    warp_result
                )));
            }

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
            Self::ensure_accessibility_permission()?;

            use core_graphics::event::{CGEvent, CGEventType, CGMouseButton};
            use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

            let source =
                CGEventSource::new(CGEventSourceStateID::HIDSystemState).map_err(|_| {
                    PlatformError::InitializationFailed("Failed to create event source".to_string())
                })?;

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

            let dummy_event = CGEvent::new(source.clone()).map_err(|_| {
                PlatformError::InitializationFailed("Failed to create dummy event".to_string())
            })?;
            let location = dummy_event.location();

            let event = CGEvent::new_mouse_event(source, event_type, location, cg_button).map_err(
                |_| {
                    PlatformError::InitializationFailed(
                        "Failed to create mouse button event".to_string(),
                    )
                },
            )?;

            event.post(core_graphics::event::CGEventTapLocation::HID);

            log::debug!(
                "Mouse button {} {}",
                button,
                if pressed { "pressed" } else { "released" }
            );
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

            unsafe {
                let pasteboard_class = Class::get("NSPasteboard").unwrap();
                let pasteboard: *mut Object = msg_send![pasteboard_class, generalPasteboard];

                if pasteboard.is_null() {
                    return Err(PlatformError::InitializationFailed(
                        "Failed to get pasteboard".to_string(),
                    ));
                }

                let string_class = Class::get("NSString").unwrap();
                let mime_type = CString::new("public.utf-8-plain-text").unwrap();
                let ns_string_type: *mut Object =
                    msg_send![string_class, stringWithUTF8String: mime_type.as_ptr()];

                let content: *mut Object = msg_send![pasteboard, stringForType: ns_string_type];

                if !content.is_null() {
                    let utf8_string: *const i8 = msg_send![content, UTF8String];
                    if !utf8_string.is_null() {
                        let rust_string = CString::from_raw(utf8_string as *mut _)
                            .into_string()
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

            unsafe {
                let pasteboard_class = Class::get("NSPasteboard").unwrap();
                let pasteboard: *mut Object = msg_send![pasteboard_class, generalPasteboard];

                if pasteboard.is_null() {
                    return Err(PlatformError::InitializationFailed(
                        "Failed to get pasteboard".to_string(),
                    ));
                }

                // Clear the pasteboard
                let _: () = msg_send![pasteboard, clearContents];

                // Create NSArray with the string type
                let string_class = Class::get("NSString").unwrap();
                let mime_type = CString::new("public.utf-8-plain-text").unwrap();
                let ns_string_type: *mut Object =
                    msg_send![string_class, stringWithUTF8String: mime_type.as_ptr()];

                let array_class = Class::get("NSArray").unwrap();
                let types_array: *mut Object =
                    msg_send![array_class, arrayWithObject: ns_string_type];

                // Write to pasteboard
                let content_cstr = CString::new(content).unwrap();
                let ns_string: *mut Object =
                    msg_send![string_class, stringWithUTF8String: content_cstr.as_ptr()];
                let result: bool = msg_send![pasteboard, writeObjects: &[ns_string]];

                if result {
                    log::debug!("Clipboard set successfully");
                    return Ok(());
                }
            }

            Err(PlatformError::InitializationFailed(
                "Failed to write to clipboard".to_string(),
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
