//! Linux platform implementation with pragmatic X11 fallback.
//!
//! The current focus is making client-side injection reliable for
//! keyboard/mouse replay. Global capture is implemented at higher layers.

use super::{PlatformError, PlatformInput};

pub struct LinuxInput;

impl LinuxInput {
    pub fn new() -> Result<Self, PlatformError> {
        Ok(Self)
    }
}

impl PlatformInput for LinuxInput {
    fn initialize(&self) -> Result<(), PlatformError> {
        Ok(())
    }

    fn capture_keyboard(&self) -> Result<(), PlatformError> {
        // Global capture is handled by the app/event bridge, not here yet.
        Ok(())
    }

    fn inject_keyboard(&self, key_code: u16, pressed: bool) -> Result<(), PlatformError> {
        #[cfg(feature = "x11")]
        {
            use std::ptr;
            use x11::xlib::{CurrentTime, XCloseDisplay, XFlush, XOpenDisplay};
            use x11::xtest::XTestFakeKeyEvent;

            unsafe {
                let display = XOpenDisplay(ptr::null());
                if display.is_null() {
                    return Err(PlatformError::DisplayError(
                        "Cannot open X display for key injection".to_string(),
                    ));
                }

                let ok = XTestFakeKeyEvent(display, key_code as u32, pressed as i32, CurrentTime);
                XFlush(display);
                XCloseDisplay(display);

                if ok == 0 {
                    return Err(PlatformError::DisplayError(
                        "XTestFakeKeyEvent failed".to_string(),
                    ));
                }
                Ok(())
            }
        }

        #[cfg(not(feature = "x11"))]
        {
            Err(PlatformError::NotSupported)
        }
    }

    fn capture_mouse(&self) -> Result<(), PlatformError> {
        // Global capture is handled by the app/event bridge, not here yet.
        Ok(())
    }

    fn inject_mouse_move(&self, dx: i16, dy: i16) -> Result<(), PlatformError> {
        #[cfg(feature = "x11")]
        {
            use std::ptr;
            use x11::xlib::{CurrentTime, XCloseDisplay, XFlush, XOpenDisplay};
            use x11::xtest::XTestFakeRelativeMotionEvent;

            unsafe {
                let display = XOpenDisplay(ptr::null());
                if display.is_null() {
                    return Err(PlatformError::DisplayError(
                        "Cannot open X display for mouse movement".to_string(),
                    ));
                }

                let ok =
                    XTestFakeRelativeMotionEvent(display, dx as i32, dy as i32, 0, CurrentTime);
                XFlush(display);
                XCloseDisplay(display);

                if ok == 0 {
                    return Err(PlatformError::DisplayError(
                        "XTestFakeRelativeMotionEvent failed".to_string(),
                    ));
                }
                Ok(())
            }
        }

        #[cfg(not(feature = "x11"))]
        {
            Err(PlatformError::NotSupported)
        }
    }

    fn inject_mouse_button(&self, button: u8, pressed: bool) -> Result<(), PlatformError> {
        #[cfg(feature = "x11")]
        {
            use std::ptr;
            use x11::xlib::{CurrentTime, XCloseDisplay, XFlush, XOpenDisplay};
            use x11::xtest::XTestFakeButtonEvent;

            let x11_button = match button {
                1 => 1,
                2 => 3,
                3 => 2,
                _ => 1,
            };

            unsafe {
                let display = XOpenDisplay(ptr::null());
                if display.is_null() {
                    return Err(PlatformError::DisplayError(
                        "Cannot open X display for mouse button".to_string(),
                    ));
                }

                let ok = XTestFakeButtonEvent(display, x11_button, pressed as i32, CurrentTime);
                XFlush(display);
                XCloseDisplay(display);

                if ok == 0 {
                    return Err(PlatformError::DisplayError(
                        "XTestFakeButtonEvent failed".to_string(),
                    ));
                }
                Ok(())
            }
        }

        #[cfg(not(feature = "x11"))]
        {
            Err(PlatformError::NotSupported)
        }
    }

    fn get_clipboard(&self) -> Result<String, PlatformError> {
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

    fn set_clipboard(&self, content: &str) -> Result<(), PlatformError> {
        use std::io::Write;
        use std::process::{Command, Stdio};

        if let Ok(mut child) = Command::new("xclip")
            .args(["-selection", "clipboard", "-i"])
            .stdin(Stdio::piped())
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(content.as_bytes());
            }
            let _ = child.wait();
            return Ok(());
        }

        if let Ok(mut child) = Command::new("wl-copy").stdin(Stdio::piped()).spawn() {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(content.as_bytes());
            }
            let _ = child.wait();
            return Ok(());
        }

        Err(PlatformError::NotSupported)
    }

    fn shutdown(&self) -> Result<(), PlatformError> {
        Ok(())
    }
}
