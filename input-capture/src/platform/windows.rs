//! Windows Platform Implementation
//!
//! This module provides Windows-specific input handling using Win32 APIs.

use super::{PlatformError, PlatformInput};
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::GetDC;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

/// Windows input handler
pub struct WindowsInput {
    initialized: bool,
}

// Map Barrier button codes to Windows mouse button codes
fn barrier_button_to_windows(button: u8) -> VIRTUAL_KEY {
    match button {
        1 => VK_LBUTTON,
        2 => VK_RBUTTON,
        3 => VK_MBUTTON,
        _ => VK_LBUTTON, // Default to left button
    }
}

impl WindowsInput {
    /// Create a new Windows input handler
    pub fn new() -> Result<Self, PlatformError> {
        Ok(Self { initialized: false })
    }
    
    /// Convert a virtual key code to Windows scan code
    fn get_scan_code(vk_code: u16) -> u16 {
        unsafe {
            MapVirtualKeyA(vk_code as u32, MAPVK_VK_TO_VSC) as u16
        }
    }
}

impl PlatformInput for WindowsInput {
    fn initialize(&self) -> Result<(), PlatformError> {
        log::info!("Windows input system initialized");
        Ok(())
    }

    fn capture_keyboard(&self) -> Result<(), PlatformError> {
        // On Windows, we can use RegisterRawInputDevices or SetWindowsHookEx
        // For simplicity, we'll note that full keyboard capture requires
        // running with appropriate privileges or as a service
        
        log::warn!("Keyboard capture on Windows requires elevated privileges or a low-level hook");
        log::info!("For production use, implement RegisterRawInputDevices or SetWindowsHookEx with WH_KEYBOARD_LL");
        
        // Note: Full implementation would require:
        // 1. Setting up a raw input device registration
        // 2. Or installing a low-level keyboard hook (requires dll injection or service)
        // 3. Running the application with appropriate permissions
        
        Err(PlatformError::PermissionDenied(
            "Keyboard capture requires elevated privileges or system service".to_string()
        ))
    }

    fn inject_keyboard(&self, key_code: u16, pressed: bool) -> Result<(), PlatformError> {
        unsafe {
            // Use SendInput for reliable keyboard injection
            let scan_code = Self::get_scan_code(key_code);
            
            let mut flags = KEYEVENTF_SCANCODE;
            if !pressed {
                flags |= KEYEVENTF_KEYUP;
            }
            
            let inputs = [INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUTUNION {
                    ki: KEYBDINPUT {
                        wVk: VIRTUAL_KEY(0), // Using scan code, not virtual key
                        wScan: scan_code,
                        dwFlags: flags,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            }];
            
            let result = SendInput(&inputs, std::mem::size_of::<INPUT>() as u32);
            
            if result == 1 {
                log::debug!("Key {} {}", key_code, if pressed { "pressed" } else { "released" });
                Ok(())
            } else {
                Err(PlatformError::InitializationFailed(
                    "SendInput failed".to_string()
                ))
            }
        }
    }

    fn capture_mouse(&self) -> Result<(), PlatformError> {
        // Similar to keyboard, mouse capture requires appropriate setup
        log::warn!("Mouse capture on Windows requires elevated privileges or a low-level hook");
        log::info!("For production use, implement RegisterRawInputDevices or SetWindowsHookEx with WH_MOUSE_LL");
        
        Err(PlatformError::PermissionDenied(
            "Mouse capture requires elevated privileges or system service".to_string()
        ))
    }

    fn inject_mouse_move(&self, dx: i16, dy: i16) -> Result<(), PlatformError> {
        unsafe {
            // Use SendInput for mouse movement
            let inputs = [INPUT {
                r#type: INPUT_MOUSE,
                Anonymous: INPUTUNION {
                    mi: MOUSEINPUT {
                        dx: dx as i32,
                        dy: dy as i32,
                        mouseData: 0,
                        dwFlags: MOUSEEVENTF_MOVE,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            }];
            
            let result = SendInput(&inputs, std::mem::size_of::<INPUT>() as u32);
            
            if result == 1 {
                log::debug!("Mouse moved by ({}, {})", dx, dy);
                Ok(())
            } else {
                Err(PlatformError::InitializationFailed(
                    "SendInput failed for mouse move".to_string()
                ))
            }
        }
    }

    fn inject_mouse_button(&self, button: u8, pressed: bool) -> Result<(), PlatformError> {
        unsafe {
            let vk_button = barrier_button_to_windows(button);
            
            let flags = match vk_button {
                VK_LBUTTON => {
                    if pressed { MOUSEEVENTF_LEFTDOWN } else { MOUSEEVENTF_LEFTUP }
                },
                VK_RBUTTON => {
                    if pressed { MOUSEEVENTF_RIGHTDOWN } else { MOUSEEVENTF_RIGHTUP }
                },
                VK_MBUTTON => {
                    if pressed { MOUSEEVENTF_MIDDLEDOWN } else { MOUSEEVENTF_MIDDLEUP }
                },
                _ => return Err(PlatformError::NotSupported),
            };
            
            let inputs = [INPUT {
                r#type: INPUT_MOUSE,
                Anonymous: INPUTUNION {
                    mi: MOUSEINPUT {
                        dx: 0,
                        dy: 0,
                        mouseData: 0,
                        dwFlags: flags,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            }];
            
            let result = SendInput(&inputs, std::mem::size_of::<INPUT>() as u32);
            
            if result == 1 {
                log::debug!("Mouse button {} {}", button, if pressed { "pressed" } else { "released" });
                Ok(())
            } else {
                Err(PlatformError::InitializationFailed(
                    "SendInput failed for mouse button".to_string()
                ))
            }
        }
    }

    fn get_clipboard(&self) -> Result<String, PlatformError> {
        use windows::Win32::Foundation::HANDLE;
        use windows::Win32::System::DataExchange::*;
        use windows::Win32::System::Memory::*;
        use std::ptr;
        
        unsafe {
            if !OpenClipboard(HWND(0)) {
                return Err(PlatformError::InitializationFailed(
                    "Failed to open clipboard".to_string()
                ));
            }
            
            // Try UTF-8 text first (CF_UNICODETEXT)
            let handle = GetClipboardData(CF_UNICODETEXT.0 as u32);
            
            if handle.is_ok() {
                let hglobal = handle.unwrap();
                let data_ptr = GlobalLock(hglobal);
                
                if !data_ptr.is_null() {
                    // Convert wide string to Rust String
                    let wide_str = data_ptr as *const u16;
                    let mut len = 0;
                    while *wide_str.add(len) != 0 {
                        len += 1;
                    }
                    
                    let wide_slice = std::slice::from_raw_parts(wide_str, len);
                    let result = String::from_utf16_lossy(wide_slice);
                    
                    GlobalUnlock(hglobal);
                    CloseClipboard();
                    
                    return Ok(result);
                }
            }
            
            // Fallback: try ANSI text (CF_TEXT)
            let handle = GetClipboardData(CF_TEXT.0 as u32);
            
            if handle.is_ok() {
                let hglobal = handle.unwrap();
                let data_ptr = GlobalLock(hglobal);
                
                if !data_ptr.is_null() {
                    let c_str = data_ptr as *const i8;
                    let bytes = std::ffi::CStr::from_ptr(c_str).to_bytes();
                    let result = String::from_utf8_lossy(bytes).to_string();
                    
                    GlobalUnlock(hglobal);
                    CloseClipboard();
                    
                    return Ok(result);
                }
            }
            
            CloseClipboard();
            Err(PlatformError::NotSupported)
        }
    }

    fn set_clipboard(&self, content: &str) -> Result<(), PlatformError> {
        use windows::Win32::Foundation::HANDLE;
        use windows::Win32::System::DataExchange::*;
        use windows::Win32::System::Memory::*;
        use std::ptr;
        
        unsafe {
            if !OpenClipboard(HWND(0)) {
                return Err(PlatformError::InitializationFailed(
                    "Failed to open clipboard".to_string()
                ));
            }
            
            EmptyClipboard();
            
            // Convert to wide string (UTF-16)
            let wide: Vec<u16> = OsStr::new(content)
                .encode_wide()
                .chain(Some(0)) // Null terminator
                .collect();
            
            let size = wide.len() * 2; // Each u16 is 2 bytes
            
            let hglobal = GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, size);
            
            if hglobal.is_err() {
                CloseClipboard();
                return Err(PlatformError::InitializationFailed(
                    "Failed to allocate global memory".to_string()
                ));
            }
            
            let hglobal = hglobal.unwrap();
            let ptr = GlobalLock(hglobal);
            
            if ptr.is_null() {
                CloseClipboard();
                return Err(PlatformError::InitializationFailed(
                    "Failed to lock global memory".to_string()
                ));
            }
            
            // Copy wide string to clipboard memory
            std::ptr::copy_nonoverlapping(
                wide.as_ptr(),
                ptr as *mut u16,
                wide.len(),
            );
            
            GlobalUnlock(hglobal);
            
            // Set clipboard data
            let result = SetClipboardData(CF_UNICODETEXT.0 as u32, Some(hglobal));
            
            CloseClipboard();
            
            if result.is_ok() {
                log::debug!("Clipboard set successfully");
                Ok(())
            } else {
                Err(PlatformError::InitializationFailed(
                    "Failed to set clipboard data".to_string()
                ))
            }
        }
    }

    fn shutdown(&self) -> Result<(), PlatformError> {
        log::info!("Windows input system shutdown");
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

    #[test]
    fn test_platform_trait_methods_exist() {
        let input = WindowsInput::new().unwrap();
        let _ = input.initialize();
    }
}
