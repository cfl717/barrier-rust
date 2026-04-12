//! Linux Platform Implementation
//!
//! This module provides Linux-specific input handling via X11/Wayland.

use super::{PlatformError, PlatformInput};
use std::cell::RefCell;
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

    // Wayland backend (wrapped in RefCell for interior mutability)
    #[cfg(feature = "wayland")]
    wayland_backend: RefCell<Option<wayland_backend::WaylandBackend>>,

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
            wayland_backend: RefCell::new(None),
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
        use x11rb::connection::Connection;
        use x11rb::errors::ConnectionError;
        use x11rb::protocol::xproto::*;

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
                    Err(PlatformError::PermissionDenied(format!(
                        "Failed to grab keyboard: {:?}",
                        reply.status()
                    )))
                }
            }
            Err(e) => Err(PlatformError::DisplayError(format!(
                "X11 keyboard grab error: {}",
                e
            ))),
        }
    }

    /// X11 keyboard injection implementation
    #[cfg(feature = "x11")]
    fn inject_keyboard_x11(&self, key_code: u16, pressed: bool) -> Result<(), PlatformError> {
        use x11rb::connection::Connection;
        use x11rb::protocol::xtest::*;

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

        log::debug!(
            "X11 key {} {}",
            key_code,
            if pressed { "pressed" } else { "released" }
        );
        Ok(())
    }

    /// X11 mouse capture implementation
    #[cfg(feature = "x11")]
    fn capture_mouse_x11(&self) -> Result<(), PlatformError> {
        use x11rb::connection::Connection;
        use x11rb::protocol::xproto::*;

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
                    Err(PlatformError::PermissionDenied(format!(
                        "Failed to grab mouse: {:?}",
                        reply.status()
                    )))
                }
            }
            Err(e) => Err(PlatformError::DisplayError(format!(
                "X11 mouse grab error: {}",
                e
            ))),
        }
    }

    /// X11 mouse movement injection implementation
    #[cfg(feature = "x11")]
    fn inject_mouse_move_x11(&self, dx: i16, dy: i16) -> Result<(), PlatformError> {
        use x11rb::connection::Connection;
        use x11rb::protocol::xtest::*;

        let (conn, _) = x11rb::connect(None)
            .map_err(|e| PlatformError::DisplayError(format!("Failed to connect to X11: {}", e)))?;

        conn.fake_relative_motion_input(
            dx as _, dy as _, 0, // time offset
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
        use x11rb::connection::Connection;
        use x11rb::protocol::xtest::*;

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
            pressed, x11_button, 0, // time offset
            0, // device id
        )
        .map_err(|e| PlatformError::DisplayError(format!("X11 button injection failed: {}", e)))?;

        conn.flush()
            .map_err(|e| PlatformError::DisplayError(format!("X11 flush failed: {}", e)))?;

        log::debug!(
            "X11 mouse button {} {}",
            button,
            if pressed { "pressed" } else { "released" }
        );
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
    fn inject_keyboard_x11_fallback(
        &self,
        key_code: u16,
        pressed: bool,
    ) -> Result<(), PlatformError> {
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
    fn inject_mouse_button_x11_fallback(
        &self,
        button: u8,
        pressed: bool,
    ) -> Result<(), PlatformError> {
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
                    "Failed to open X11 display".to_string(),
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
        // Initialize based on display server
        match self.display_server {
            #[cfg(feature = "wayland")]
            DisplayServer::Wayland => {
                let mut backend = wayland_backend::WaylandBackend::new()?;
                // Create virtual devices for injection
                backend.create_virtual_keyboard()?;
                backend.create_virtual_pointer()?;

                // Store backend in RefCell
                self.wayland_backend.borrow_mut().replace(backend);

                log::info!("Wayland input system initialized");
            }
            #[cfg(feature = "x11")]
            DisplayServer::X11 => {
                log::info!("X11 input system initialized");
            }
            DisplayServer::Unknown => {
                return Err(PlatformError::DisplayError(
                    "Unknown display server".to_string(),
                ));
            }
        }
        Ok(())
    }

    fn capture_keyboard(&self) -> Result<(), PlatformError> {
        #[cfg(feature = "wayland")]
        if self.is_wayland() {
            if let Some(backend) = self.wayland_backend.borrow_mut().as_mut() {
                return backend.capture_keyboard();
            } else {
                return Err(PlatformError::NotInitialized);
            }
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
            if let Some(backend) = self.wayland_backend.borrow_mut().as_mut() {
                // Convert u16 key code to u32 (Wayland uses Linux key codes)
                return backend.inject_key(key_code as u32, pressed);
            } else {
                return Err(PlatformError::NotInitialized);
            }
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
            if let Some(backend) = self.wayland_backend.borrow_mut().as_mut() {
                return backend.capture_mouse();
            } else {
                return Err(PlatformError::NotInitialized);
            }
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
            if let Some(backend) = self.wayland_backend.borrow_mut().as_mut() {
                // Convert i16 delta to f64 relative movement
                // Using a scaling factor: 0.001 per unit seems reasonable
                let scale = 0.001;
                return backend.inject_mouse_move_relative(dx as f64 * scale, dy as f64 * scale);
            } else {
                return Err(PlatformError::NotInitialized);
            }
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
            if let Some(backend) = self.wayland_backend.borrow_mut().as_mut() {
                // Map Barrier button codes to Linux button codes
                // Barrier: 1=left, 2=right, 3=middle
                // Linux: 0x110=left, 0x111=right, 0x112=middle
                let linux_button = match button {
                    1 => 0x110, // BTN_LEFT
                    2 => 0x111, // BTN_RIGHT
                    3 => 0x112, // BTN_MIDDLE
                    _ => {
                        log::warn!("Unknown button code: {}, using left button", button);
                        0x110
                    }
                };
                return backend.inject_mouse_button(linux_button, pressed);
            } else {
                return Err(PlatformError::NotInitialized);
            }
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
            // Try native Wayland clipboard first
            if let Some(backend) = self.wayland_backend.borrow().as_ref() {
                match backend.get_clipboard() {
                    Ok(content) => return Ok(content),
                    Err(e) => log::debug!(
                        "Wayland native clipboard failed: {}, falling back to wl-paste",
                        e
                    ),
                }
            }

            // Fallback to external tool
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
        use std::io::Write;
        use std::process::Command;

        // Try Wayland first if detected
        #[cfg(feature = "wayland")]
        if self.is_wayland() {
            // Try native Wayland clipboard first
            if let Some(backend) = self.wayland_backend.borrow().as_ref() {
                match backend.set_clipboard(content) {
                    Ok(()) => return Ok(()),
                    Err(e) => log::debug!(
                        "Wayland native clipboard failed: {}, falling back to wl-copy",
                        e
                    ),
                }
            }

            // Fallback to external tool
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

#[cfg(feature = "wayland")]
mod wayland_backend {
    use super::PlatformError;
    use wayland_client::{
        protocol::{wl_compositor, wl_display, wl_registry, wl_seat},
        Connection, Dispatch, EventQueue, Proxy, QueueHandle,
    };
    use wayland_protocols_misc::zwp_virtual_keyboard_v1::client::zwp_virtual_keyboard_manager_v1;
    use wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_manager_v1;
    use wayland_protocols_wlr::input_inhibitor::v1::client::zwlr_input_inhibitor_manager_v1;
    use wayland_protocols_wlr::virtual_pointer::v1::client::zwlr_virtual_pointer_manager_v1;

    /// Wayland backend implementation
    pub struct WaylandBackend {
        connection: Option<Connection>,
        event_queue: Option<EventQueue<Self>>,
        queue_handle: QueueHandle<Self>,

        // Global objects
        compositor: Option<wl_compositor::WlCompositor>,
        seat: Option<wl_seat::WlSeat>,
        input_inhibitor_manager:
            Option<zwlr_input_inhibitor_manager_v1::ZwlrInputInhibitorManagerV1>,
        virtual_keyboard_manager:
            Option<zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1>,
        virtual_pointer_manager:
            Option<zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1>,
        data_control_manager: Option<zwlr_data_control_manager_v1::ZwlrDataControlManagerV1>,

        // Active objects
        active_inhibitor: Option<zwlr_input_inhibitor_manager_v1::ZwlrInputInhibitorV1>,
        virtual_keyboard: Option<zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardV1>,
        virtual_pointer: Option<zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerV1>,

        // Mouse position tracking (for relative movement)
        mouse_x: f64,
        mouse_y: f64,
    }

    impl WaylandBackend {
        /// Create a new Wayland backend
        pub fn new() -> Result<Self, PlatformError> {
            let connection = Connection::connect_to_env().map_err(|e| {
                PlatformError::DisplayError(format!("Failed to connect to Wayland: {}", e))
            })?;

            let display = connection.display();
            let mut event_queue = connection.new_event_queue();
            let queue_handle = event_queue.handle();

            let mut backend = Self {
                connection: Some(connection),
                event_queue: Some(event_queue),
                queue_handle,
                compositor: None,
                seat: None,
                input_inhibitor_manager: None,
                virtual_keyboard_manager: None,
                virtual_pointer_manager: None,
                data_control_manager: None,
                active_inhibitor: None,
                virtual_keyboard: None,
                virtual_pointer: None,
                mouse_x: 0.0,
                mouse_y: 0.0,
            };

            // Setup registry and bind globals
            backend.setup_registry()?;

            // Roundtrip to get initial globals
            backend.roundtrip()?;

            Ok(backend)
        }

        fn setup_registry(&mut self) -> Result<(), PlatformError> {
            let display = self.connection.as_ref().unwrap().display();
            let registry = display.get_registry(&self.queue_handle, ());

            // We'll implement registry handling in Dispatch trait
            Ok(())
        }

        fn roundtrip(&mut self) -> Result<(), PlatformError> {
            self.event_queue
                .as_mut()
                .ok_or(PlatformError::NotInitialized)?
                .roundtrip()
                .map_err(|e| {
                    PlatformError::DisplayError(format!("Wayland roundtrip failed: {}", e))
                })
        }

        /// Capture keyboard input using input inhibitor
        pub fn capture_keyboard(&mut self) -> Result<(), PlatformError> {
            let inhibitor_manager = self.input_inhibitor_manager.as_ref().ok_or_else(|| {
                PlatformError::NotSupportedWithMessage(
                    "wlr-input-inhibitor protocol not supported".into(),
                )
            })?;

            let inhibitor = inhibitor_manager.get_inhibitor(&self.queue_handle, ());
            self.active_inhibitor = Some(inhibitor);

            // Activate by sending roundtrip
            self.roundtrip()?;

            Ok(())
        }

        /// Release keyboard capture
        pub fn release_keyboard_capture(&mut self) {
            self.active_inhibitor.take();
        }

        /// Capture mouse input using input inhibitor
        pub fn capture_mouse(&mut self) -> Result<(), PlatformError> {
            // Input inhibitor captures both keyboard and mouse
            // So we can just use the same inhibitor
            self.capture_keyboard()
        }

        /// Release mouse capture
        pub fn release_mouse_capture(&mut self) {
            // Same inhibitor is used for both
            self.release_keyboard_capture()
        }

        /// Create virtual keyboard for input injection
        pub fn create_virtual_keyboard(&mut self) -> Result<(), PlatformError> {
            let keyboard_manager = self.virtual_keyboard_manager.as_ref().ok_or_else(|| {
                PlatformError::NotSupportedWithMessage(
                    "virtual-keyboard protocol not supported".into(),
                )
            })?;

            let seat = self.seat.as_ref().ok_or(PlatformError::NotInitialized)?;

            let keyboard = keyboard_manager.create_virtual_keyboard(seat, &self.queue_handle, ());
            self.virtual_keyboard = Some(keyboard);

            Ok(())
        }

        /// Inject a key event
        pub fn inject_key(&mut self, key_code: u32, pressed: bool) -> Result<(), PlatformError> {
            if let Some(kbd) = &self.virtual_keyboard {
                let state = if pressed { 1 } else { 0 };
                // Note: time parameter is 0 for synthetic events
                kbd.key(0, key_code, state);
                kbd.commit();
                Ok(())
            } else {
                Err(PlatformError::NotInitialized)
            }
        }

        /// Create virtual pointer for mouse injection
        pub fn create_virtual_pointer(&mut self) -> Result<(), PlatformError> {
            let pointer_manager = self.virtual_pointer_manager.as_ref().ok_or_else(|| {
                PlatformError::NotSupportedWithMessage(
                    "wlr-virtual-pointer protocol not supported".into(),
                )
            })?;

            let seat = self.seat.as_ref().ok_or(PlatformError::NotInitialized)?;

            let pointer = pointer_manager.create_virtual_pointer(seat, &self.queue_handle, ());
            self.virtual_pointer = Some(pointer);

            Ok(())
        }

        /// Inject absolute mouse movement
        pub fn inject_mouse_move_absolute(&mut self, x: f64, y: f64) -> Result<(), PlatformError> {
            if let Some(ptr) = &self.virtual_pointer {
                // Normalize coordinates (assuming 0-1.0 range)
                ptr.motion_absolute(0, x, y, 0, 0);
                ptr.frame();
                self.mouse_x = x;
                self.mouse_y = y;
                Ok(())
            } else {
                Err(PlatformError::NotInitialized)
            }
        }

        /// Inject relative mouse movement
        pub fn inject_mouse_move_relative(
            &mut self,
            dx: f64,
            dy: f64,
        ) -> Result<(), PlatformError> {
            // Update position (clamp to 0-1 range)
            self.mouse_x = (self.mouse_x + dx).max(0.0).min(1.0);
            self.mouse_y = (self.mouse_y + dy).max(0.0).min(1.0);

            if let Some(ptr) = &self.virtual_pointer {
                ptr.motion_absolute(0, self.mouse_x, self.mouse_y, 0, 0);
                ptr.frame();
                Ok(())
            } else {
                Err(PlatformError::NotInitialized)
            }
        }

        /// Inject mouse button
        pub fn inject_mouse_button(
            &mut self,
            button: u32,
            pressed: bool,
        ) -> Result<(), PlatformError> {
            if let Some(ptr) = &self.virtual_pointer {
                let state = if pressed { 1 } else { 0 };
                ptr.button(0, button, state);
                ptr.frame();
                Ok(())
            } else {
                Err(PlatformError::NotInitialized)
            }
        }

        /// Get clipboard content
        pub fn get_clipboard(&self) -> Result<String, PlatformError> {
            let manager = self.data_control_manager.as_ref().ok_or_else(|| {
                PlatformError::NotSupportedWithMessage(
                    "wlr-data-control protocol not supported".into(),
                )
            })?;

            // TODO: Implement clipboard reading
            // This requires creating a data device, making selection, and handling events
            log::warn!("Wayland native clipboard get not yet implemented");
            Err(PlatformError::NotSupportedWithMessage(
                "Clipboard get not implemented".into(),
            ))
        }

        /// Set clipboard content
        pub fn set_clipboard(&self, content: &str) -> Result<(), PlatformError> {
            let manager = self.data_control_manager.as_ref().ok_or_else(|| {
                PlatformError::NotSupportedWithMessage(
                    "wlr-data-control protocol not supported".into(),
                )
            })?;

            // TODO: Implement clipboard setting
            // This requires creating a data source and offering it
            log::warn!("Wayland native clipboard set not yet implemented");
            Err(PlatformError::NotSupportedWithMessage(
                "Clipboard set not implemented".into(),
            ))
        }
    }

    impl Dispatch<wl_registry::WlRegistry, ()> for WaylandBackend {
        fn event(
            state: &mut Self,
            _proxy: &wl_registry::WlRegistry,
            event: wl_registry::Event,
            _data: &(),
            _conn: &Connection,
            _qhandle: &QueueHandle<Self>,
        ) {
            match event {
                wl_registry::Event::Global {
                    name,
                    interface,
                    version,
                } => {
                    log::debug!("Wayland global: {} {} {}", name, interface, version);

                    match interface.as_str() {
                        "wl_compositor" => {
                            let compositor = _proxy.bind::<wl_compositor::WlCompositor>(
                                name,
                                version,
                                _qhandle,
                                (),
                            );
                            state.compositor = Some(compositor);
                        }
                        "wl_seat" => {
                            let seat = _proxy.bind::<wl_seat::WlSeat>(name, version, _qhandle, ());
                            state.seat = Some(seat);
                        }
                        "zwlr_input_inhibitor_manager_v1" => {
                            let manager = _proxy.bind::<zwlr_input_inhibitor_manager_v1::ZwlrInputInhibitorManagerV1>(
                                name, version, _qhandle, ());
                            state.input_inhibitor_manager = Some(manager);
                        }
                        "zwp_virtual_keyboard_manager_v1" => {
                            let manager = _proxy.bind::<zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1>(
                                name, version, _qhandle, ());
                            state.virtual_keyboard_manager = Some(manager);
                        }
                        "zwlr_virtual_pointer_manager_v1" => {
                            let manager = _proxy.bind::<zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1>(
                                name, version, _qhandle, ());
                            state.virtual_pointer_manager = Some(manager);
                        }
                        "zwlr_data_control_manager_v1" => {
                            let manager = _proxy
                                .bind::<zwlr_data_control_manager_v1::ZwlrDataControlManagerV1>(
                                name,
                                version,
                                _qhandle,
                                (),
                            );
                            state.data_control_manager = Some(manager);
                        }
                        _ => {}
                    }
                }
                wl_registry::Event::GlobalRemove { name: _ } => {}
            }
        }
    }

    // Implement other dispatch traits as needed
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
