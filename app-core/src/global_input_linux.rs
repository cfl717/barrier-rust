//! Linux + X11：通过 rdev 监听全局键鼠，在「服务端已选中的远程客户端」激活时转发到协议层。
//! rdev 在 Linux 上使用 XRecord，需 DISPLAY；与 Wayland 会话不兼容。
//! 
//! 支持功能：
//! - 边缘自动切换：鼠标到达屏幕边缘时自动切换到对应客户端
//! - 输入转发：激活客户端后将键鼠事件转发到远程

use crate::core::BarrierRuntimeState;
use barrier_protocol::BarrierServer;
use rdev::{Button, EventType, Key};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::runtime;
use tokio::sync::{mpsc, Mutex};

#[derive(Debug, Clone)]
enum GlobalInputEvent {
    MouseMove(i16, i16, f64, f64), // dx, dy, abs_x, abs_y
    MouseButton(u8, bool),
    Key(u16, bool),
}

const MOVE_FLUSH_INTERVAL: Duration = Duration::from_millis(8);
const MOVE_BURST_FLUSH: i32 = 96;
const EDGE_THRESHOLD: f64 = 2.0; // 距离边缘多少像素触发切换

pub(crate) fn start_global_input_pipeline(state: Arc<Mutex<BarrierRuntimeState>>) {
    let (tx, mut rx) = mpsc::unbounded_channel::<GlobalInputEvent>();

    std::thread::spawn(move || {
        let mut last_pos: Option<(f64, f64)> = None;
        let result = rdev::listen(move |event| match event.event_type {
            EventType::MouseMove { x, y } => {
                if let Some((lx, ly)) = last_pos {
                    let dx = (x - lx).round() as i16;
                    let dy = (y - ly).round() as i16;
                    if dx != 0 || dy != 0 {
                        let _ = tx.send(GlobalInputEvent::MouseMove(dx, dy, x, y));
                    }
                }
                last_pos = Some((x, y));
            }
            EventType::ButtonPress(button) => {
                if let Some(mapped) = map_button(button) {
                    let _ = tx.send(GlobalInputEvent::MouseButton(mapped, true));
                }
            }
            EventType::ButtonRelease(button) => {
                if let Some(mapped) = map_button(button) {
                    let _ = tx.send(GlobalInputEvent::MouseButton(mapped, false));
                }
            }
            EventType::KeyPress(key) => {
                if let Some(code) = rdev_key_to_x11_keycode(key) {
                    let _ = tx.send(GlobalInputEvent::Key(code, true));
                }
            }
            EventType::KeyRelease(key) => {
                if let Some(code) = rdev_key_to_x11_keycode(key) {
                    let _ = tx.send(GlobalInputEvent::Key(code, false));
                }
            }
            _ => {}
        });
        if let Err(err) = result {
            log::error!("Global input listener failed: {:?}", err);
        }
    });

    if let Err(err) = std::thread::Builder::new()
        .name("barrier-global-input-relay".into())
        .spawn(move || {
            let rt = match runtime::Builder::new_multi_thread()
                .enable_all()
                .thread_name("barrier-global-input-worker")
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    log::error!("global input: 无法创建 Tokio runtime: {}", e);
                    return;
                }
            };
            rt.block_on(async move {
                let mut acc_dx: i32 = 0;
                let mut acc_dy: i32 = 0;
                let mut last_flush = Instant::now();

                async fn flush_pending_move(
                    server_arc: &Arc<Mutex<BarrierServer>>,
                    active_client: &str,
                    acc_dx: &mut i32,
                    acc_dy: &mut i32,
                    last_flush: &mut Instant,
                ) {
                    if *acc_dx == 0 && *acc_dy == 0 {
                        return;
                    }
                    let server = server_arc.lock().await;
                    if let Err(err) = server
                        .relay_mouse_move(active_client, *acc_dx as i16, *acc_dy as i16)
                        .await
                    {
                        log::warn!("Global input relay (move) failed: {}", err);
                    }
                    *acc_dx = 0;
                    *acc_dy = 0;
                    *last_flush = Instant::now();
                }

                // 获取屏幕尺寸用于边缘检测
                let screen_size = get_screen_size();
                log::info!("Screen size for edge detection: {:?}", screen_size);

                while let Some(event) = rx.recv().await {
                    let (server_arc, active_client, client_routes) = {
                        let state_guard = state.lock().await;
                        (
                            state_guard.server.clone(),
                            state_guard.active_client.clone(),
                            state_guard.settings.client_routes.clone(),
                        )
                    };

                    // 如果没有 server 运行，跳过
                    let Some(server_arc) = server_arc else {
                        acc_dx = 0;
                        acc_dy = 0;
                        continue;
                    };

                    // 处理边缘检测和自动切换
                    if let GlobalInputEvent::MouseMove(_, _, abs_x, abs_y) = &event {
                        if active_client.is_none() {
                            // 当前在本机，检测是否到达边缘
                            if let Some((width, height)) = screen_size {
                                let edge = detect_edge(*abs_x, *abs_y, width, height);
                                if let Some(edge_name) = edge {
                                    // 查找对应边缘的客户端
                                    if let Some(route) = client_routes.iter().find(|r| r.edge.as_str() == edge_name) {
                                        log::info!("Edge detected: {} -> switching to {}", edge_name, route.name);
                                        // 触发切换
                                        let mut state_guard = state.lock().await;
                                        state_guard.active_client = Some(route.name.clone());
                                        // 发送 ENTER
                                        let server = server_arc.lock().await;
                                        if let Err(e) = server.send_enter(&route.name, 0, 0).await {
                                            log::warn!("Failed to send enter on edge switch: {}", e);
                                        }
                                        drop(server);
                                        crate::core::append_log(&mut state_guard, format!("边缘切换: {} -> {}", edge_name, route.name));
                                    }
                                }
                            }
                        }
                    }

                    // 如果没有激活的客户端，不转发输入
                    let Some(active_client) = active_client else {
                        acc_dx = 0;
                        acc_dy = 0;
                        continue;
                    };

                    match event {
                        GlobalInputEvent::MouseMove(dx, dy, _, _) => {
                            acc_dx += dx as i32;
                            acc_dy += dy as i32;
                            let burst =
                                acc_dx.abs() > MOVE_BURST_FLUSH || acc_dy.abs() > MOVE_BURST_FLUSH;
                            if burst || last_flush.elapsed() >= MOVE_FLUSH_INTERVAL {
                                flush_pending_move(
                                    &server_arc,
                                    &active_client,
                                    &mut acc_dx,
                                    &mut acc_dy,
                                    &mut last_flush,
                                )
                                .await;
                            }
                        }
                        other => {
                            flush_pending_move(
                                &server_arc,
                                &active_client,
                                &mut acc_dx,
                                &mut acc_dy,
                                &mut last_flush,
                            )
                            .await;

                            let server = server_arc.lock().await;
                            let relay_result = match other {
                                GlobalInputEvent::MouseButton(button, pressed) => {
                                    server.relay_mouse_button(&active_client, button, pressed).await
                                }
                                GlobalInputEvent::Key(key_code, pressed) => {
                                    server.relay_key_event(&active_client, key_code, pressed).await
                                }
                                GlobalInputEvent::MouseMove(_, _, _, _) => unreachable!(),
                            };
                            if let Err(err) = relay_result {
                                log::warn!("Global input relay failed: {}", err);
                            }
                        }
                    }
                }
            });
        })
    {
        log::error!("global input: 无法启动转发线程: {}", err);
    }
}

fn map_button(button: Button) -> Option<u8> {
    match button {
        Button::Left => Some(1),
        Button::Right => Some(2),
        Button::Middle => Some(3),
        _ => None,
    }
}

/// 获取屏幕尺寸
fn get_screen_size() -> Option<(f64, f64)> {
    #[cfg(target_os = "linux")]
    {
        use std::ptr;
        use x11::xlib::{XCloseDisplay, XDefaultScreen, XDisplayHeight, XDisplayWidth, XOpenDisplay};
        
        unsafe {
            let display = XOpenDisplay(ptr::null());
            if display.is_null() {
                return None;
            }
            let screen = XDefaultScreen(display);
            let width = XDisplayWidth(display, screen);
            let height = XDisplayHeight(display, screen);
            XCloseDisplay(display);
            Some((width as f64, height as f64))
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

/// 检测鼠标是否到达屏幕边缘，返回边缘方向
fn detect_edge(x: f64, y: f64, screen_width: f64, screen_height: f64) -> Option<&'static str> {
    if x <= EDGE_THRESHOLD {
        Some("left")
    } else if x >= screen_width - EDGE_THRESHOLD {
        Some("right")
    } else if y <= EDGE_THRESHOLD {
        Some("top")
    } else if y >= screen_height - EDGE_THRESHOLD {
        Some("bottom")
    } else {
        None
    }
}

#[rustfmt::skip]
fn rdev_key_to_x11_keycode(key: Key) -> Option<u16> {
    match key {
        Key::Unknown(code) => (code <= u32::from(u16::MAX)).then(|| code as u16),
        Key::MetaRight => Some(134),
        Key::Alt => Some(64),
        Key::AltGr => Some(108),
        Key::Backspace => Some(22),
        Key::CapsLock => Some(66),
        Key::ControlLeft => Some(37),
        Key::ControlRight => Some(105),
        Key::Delete => Some(119),
        Key::DownArrow => Some(116),
        Key::End => Some(115),
        Key::Escape => Some(9),
        Key::F1 => Some(67),
        Key::F10 => Some(76),
        Key::F11 => Some(95),
        Key::F12 => Some(96),
        Key::F2 => Some(68),
        Key::F3 => Some(69),
        Key::F4 => Some(70),
        Key::F5 => Some(71),
        Key::F6 => Some(72),
        Key::F7 => Some(73),
        Key::F8 => Some(74),
        Key::F9 => Some(75),
        Key::Home => Some(110),
        Key::LeftArrow => Some(113),
        Key::MetaLeft => Some(133),
        Key::PageDown => Some(117),
        Key::PageUp => Some(112),
        Key::Return => Some(36),
        Key::RightArrow => Some(114),
        Key::ShiftLeft => Some(50),
        Key::ShiftRight => Some(62),
        Key::Space => Some(65),
        Key::Tab => Some(23),
        Key::UpArrow => Some(111),
        Key::PrintScreen => Some(107),
        Key::ScrollLock => Some(78),
        Key::Pause => Some(127),
        Key::NumLock => Some(77),
        Key::BackQuote => Some(49),
        Key::Num1 => Some(10),
        Key::Num2 => Some(11),
        Key::Num3 => Some(12),
        Key::Num4 => Some(13),
        Key::Num5 => Some(14),
        Key::Num6 => Some(15),
        Key::Num7 => Some(16),
        Key::Num8 => Some(17),
        Key::Num9 => Some(18),
        Key::Num0 => Some(19),
        Key::Minus => Some(20),
        Key::Equal => Some(21),
        Key::KeyQ => Some(24),
        Key::KeyW => Some(25),
        Key::KeyE => Some(26),
        Key::KeyR => Some(27),
        Key::KeyT => Some(28),
        Key::KeyY => Some(29),
        Key::KeyU => Some(30),
        Key::KeyI => Some(31),
        Key::KeyO => Some(32),
        Key::KeyP => Some(33),
        Key::LeftBracket => Some(34),
        Key::RightBracket => Some(35),
        Key::KeyA => Some(38),
        Key::KeyS => Some(39),
        Key::KeyD => Some(40),
        Key::KeyF => Some(41),
        Key::KeyG => Some(42),
        Key::KeyH => Some(43),
        Key::KeyJ => Some(44),
        Key::KeyK => Some(45),
        Key::KeyL => Some(46),
        Key::SemiColon => Some(47),
        Key::Quote => Some(48),
        Key::BackSlash => Some(51),
        Key::IntlBackslash => Some(94),
        Key::KeyZ => Some(52),
        Key::KeyX => Some(53),
        Key::KeyC => Some(54),
        Key::KeyV => Some(55),
        Key::KeyB => Some(56),
        Key::KeyN => Some(57),
        Key::KeyM => Some(58),
        Key::Comma => Some(59),
        Key::Dot => Some(60),
        Key::Slash => Some(61),
        Key::Insert => Some(118),
        Key::KpReturn => Some(104),
        Key::KpMinus => Some(82),
        Key::KpPlus => Some(86),
        Key::KpMultiply => Some(63),
        Key::KpDivide => Some(106),
        Key::Kp0 => Some(90),
        Key::Kp1 => Some(87),
        Key::Kp2 => Some(88),
        Key::Kp3 => Some(89),
        Key::Kp4 => Some(83),
        Key::Kp5 => Some(84),
        Key::Kp6 => Some(85),
        Key::Kp7 => Some(79),
        Key::Kp8 => Some(80),
        Key::Kp9 => Some(81),
        Key::KpDelete => Some(91),
        Key::Function => None,
    }
}
