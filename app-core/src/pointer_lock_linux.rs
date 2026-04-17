//! Linux X11 pointer lock helper.
//! 
//! When enabled, this grabs the server-side pointer and uses an invisible cursor
//! so local pointer feedback no longer distracts during remote-control mode.

use std::ptr;
use x11::xlib;

pub(crate) fn set_pointer_lock(enabled: bool) -> Result<(), String> {
    unsafe {
        let display = xlib::XOpenDisplay(ptr::null());
        if display.is_null() {
            return Err("无法打开 X11 Display（请确认在 X11 会话下运行）".to_string());
        }

        let root = xlib::XDefaultRootWindow(display);

        if enabled {
            let blank_bits = [0u8; 1];
            let blank_pixmap = xlib::XCreateBitmapFromData(
                display,
                root,
                blank_bits.as_ptr() as *const i8,
                1,
                1,
            );
            if blank_pixmap == 0 {
                xlib::XCloseDisplay(display);
                return Err("创建透明光标位图失败".to_string());
            }

            let mut dummy_color: xlib::XColor = std::mem::zeroed();
            let invisible_cursor = xlib::XCreatePixmapCursor(
                display,
                blank_pixmap,
                blank_pixmap,
                &mut dummy_color,
                &mut dummy_color,
                0,
                0,
            );
            xlib::XFreePixmap(display, blank_pixmap);

            if invisible_cursor == 0 {
                xlib::XCloseDisplay(display);
                return Err("创建透明光标失败".to_string());
            }

            xlib::XDefineCursor(display, root, invisible_cursor);

            let event_mask = (xlib::ButtonPressMask
                | xlib::ButtonReleaseMask
                | xlib::PointerMotionMask
                | xlib::EnterWindowMask
                | xlib::LeaveWindowMask) as u32;

            let grab_result = xlib::XGrabPointer(
                display,
                root,
                xlib::False,
                event_mask,
                xlib::GrabModeAsync,
                xlib::GrabModeAsync,
                0,
                invisible_cursor,
                xlib::CurrentTime,
            );

            if grab_result != xlib::GrabSuccess {
                xlib::XUndefineCursor(display, root);
                xlib::XFreeCursor(display, invisible_cursor);
                xlib::XCloseDisplay(display);
                return Err(format!("XGrabPointer 失败，错误码={}", grab_result));
            }

            let screen = xlib::XDefaultScreen(display);
            let center_x = xlib::XDisplayWidth(display, screen) / 2;
            let center_y = xlib::XDisplayHeight(display, screen) / 2;
            xlib::XWarpPointer(display, 0, root, 0, 0, 0, 0, center_x, center_y);

            xlib::XFlush(display);
            xlib::XFreeCursor(display, invisible_cursor);
        } else {
            xlib::XUngrabPointer(display, xlib::CurrentTime);
            xlib::XUndefineCursor(display, root);
            xlib::XFlush(display);
        }

        xlib::XCloseDisplay(display);
    }

    Ok(())
}
