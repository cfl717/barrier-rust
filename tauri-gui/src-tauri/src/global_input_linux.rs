//! Linux + X11：通过 rdev 监听全局键鼠，在「服务端已选中的远程客户端」激活时转发到协议层。
//! rdev 在 Linux 上使用 XRecord，需 DISPLAY；与 Wayland 会话不兼容。

use crate::BarrierState;
use barrier_protocol::BarrierServer;
use rdev::{Button, EventType, Key};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::runtime;
use tokio::sync::{mpsc, Mutex};

#[derive(Debug, Clone)]
enum GlobalInputEvent {
    MouseMove(i16, i16),
    MouseButton(u8, bool),
    Key(u16, bool),
}

const MOVE_FLUSH_INTERVAL: Duration = Duration::from_millis(8);
const MOVE_BURST_FLUSH: i32 = 96;

pub(crate) fn start_global_input_pipeline(state: Arc<Mutex<BarrierState>>) {
    let (tx, mut rx) = mpsc::unbounded_channel::<GlobalInputEvent>();

    std::thread::spawn(move || {
        let mut last_pos: Option<(f64, f64)> = None;
        let result = rdev::listen(move |event| {
            match event.event_type {
                EventType::MouseMove { x, y } => {
                    if let Some((lx, ly)) = last_pos {
                        let dx = (x - lx).round() as i16;
                        let dy = (y - ly).round() as i16;
                        if dx != 0 || dy != 0 {
                            let _ = tx.send(GlobalInputEvent::MouseMove(dx, dy));
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
            }
        });
        if let Err(err) = result {
            log::error!("Global input listener failed: {:?}", err);
        }
    });

    // `.setup()` 时尚未进入 Tauri 的 Tokio runtime，不能在此处 `tokio::spawn`。
    // 在独立线程自建 Runtime，专门消费 rdev 发来的事件并访问 `BarrierState`。
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

                while let Some(event) = rx.recv().await {
                    let (server_arc, active_client) = {
                        let state_guard = state.lock().await;
                        (state_guard.server.clone(), state_guard.active_client.clone())
                    };
                    let (Some(server_arc), Some(active_client)) = (server_arc, active_client) else {
                        acc_dx = 0;
                        acc_dy = 0;
                        continue;
                    };

                    match event {
                        GlobalInputEvent::MouseMove(dx, dy) => {
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
                                GlobalInputEvent::MouseMove(_, _) => unreachable!(),
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

/// 与 rdev Linux 实现一致：物理键位 → X11 keycode（供客户端 XTestFakeKeyEvent 使用）。
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
