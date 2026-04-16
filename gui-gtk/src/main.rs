use adw::prelude::*;
use anyhow::Result;
use async_channel::{unbounded, Sender};
use app_core::{
    AppSettings, AppState, BarrierCore, ClientConfig, ClientRoute, RuntimeCapabilities, ScreenEdge,
    ServerConfig, SwitchClientRequest,
};
use gtk::{glib, Align};
use std::sync::Arc;
use tokio::runtime::Runtime;

#[derive(Clone)]
enum UiMessage {
    Loaded(AppSettings),
    Status(AppState),
    Runtime(RuntimeCapabilities),
    Success(String),
    Error(String),
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let _ = adw::init();
    let runtime = Arc::new(Runtime::new()?);
    let core = Arc::new(BarrierCore::new());
    runtime.block_on(core.initialize_platform_features());
    let _ = runtime.block_on(core.restore_on_startup());

    let app = adw::Application::builder()
        .application_id(if cfg!(target_os = "macos") {
            "org.barrier.native.mac"
        } else {
            "org.barrier.native.gtk"
        })
        .build();

    let runtime_for_activate = runtime.clone();
    let core_for_activate = core.clone();
    app.connect_activate(move |app| {
        build_ui(app, core_for_activate.clone(), runtime_for_activate.clone());
    });
    app.run();
    Ok(())
}

fn build_ui(app: &adw::Application, core: Arc<BarrierCore>, runtime: Arc<Runtime>) {
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title(if cfg!(target_os = "macos") {
            "Barrier"
        } else {
            "Barrier (Ubuntu Native)"
        })
        .default_width(800)
        .default_height(600)
        .build();

    let header_bar = adw::HeaderBar::new();
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.append(&header_bar);

    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_top(16);
    content.set_margin_bottom(16);
    content.set_margin_start(16);
    content.set_margin_end(16);

    let title = gtk::Label::new(Some(if cfg!(target_os = "macos") {
        "Barrier — GTK4 / libadwaita（macOS 上为跨平台壳，非 AppKit）"
    } else {
        "Barrier Ubuntu 原生前端 (GTK4 + libadwaita)"
    }));
    title.add_css_class("title-2");
    title.set_halign(Align::Start);
    content.append(&title);

    let mode_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let mode_label = gtk::Label::new(Some("模式"));
    mode_label.set_width_chars(10);
    mode_label.set_xalign(0.0);
    let mode_combo = gtk::ComboBoxText::new();
    mode_combo.append(Some("server"), "Server");
    mode_combo.append(Some("client"), "Client");
    mode_combo.set_active_id(Some("server"));
    mode_row.append(&mode_label);
    mode_row.append(&mode_combo);
    content.append(&mode_row);

    let server_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let port_entry = gtk::Entry::new();
    port_entry.set_placeholder_text(Some("Server 端口"));
    port_entry.set_text("24800");
    let screen_entry = gtk::Entry::new();
    screen_entry.set_placeholder_text(Some("Server 名称"));
    screen_entry.set_text("server");
    server_box.append(&gtk::Label::new(Some("Server 配置")));
    server_box.append(&port_entry);
    server_box.append(&screen_entry);
    content.append(&server_box);

    let client_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let server_addr_entry = gtk::Entry::new();
    server_addr_entry.set_placeholder_text(Some("Server 地址，如 192.168.1.100:24800"));
    server_addr_entry.set_text("localhost:24800");
    server_addr_entry.set_hexpand(true);
    let client_name_entry = gtk::Entry::new();
    client_name_entry.set_placeholder_text(Some("Client 名称"));
    client_name_entry.set_text("client-1");
    client_box.append(&gtk::Label::new(Some("Client 配置")));
    client_box.append(&server_addr_entry);
    client_box.append(&client_name_entry);
    content.append(&client_box);

    let route_label = gtk::Label::new(Some(
        "客户端映射（每行 name,address,edge；edge 可选 left/right/top/bottom）",
    ));
    route_label.set_halign(Align::Start);
    content.append(&route_label);
    let routes_text = gtk::TextView::new();
    routes_text.set_monospace(true);
    routes_text.set_vexpand(false);
    routes_text.set_hexpand(true);
    routes_text.set_wrap_mode(gtk::WrapMode::WordChar);
    routes_text.buffer().set_text("client-1,192.168.1.101:24800,right");
    let route_scroller = gtk::ScrolledWindow::new();
    route_scroller.set_min_content_height(120);
    route_scroller.set_child(Some(&routes_text));
    content.append(&route_scroller);

    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let start_btn = gtk::Button::with_label("启动");
    let stop_btn = gtk::Button::with_label("停止");
    let save_btn = gtk::Button::with_label("保存配置");
    let switch_btn = gtk::Button::with_label("切换到首个客户端");
    let back_btn = gtk::Button::with_label("切回本机");
    actions.append(&start_btn);
    actions.append(&stop_btn);
    actions.append(&save_btn);
    actions.append(&switch_btn);
    actions.append(&back_btn);
    content.append(&actions);

    let runtime_panel = gtk::Label::new(Some("运行环境: 加载中"));
    runtime_panel.set_halign(Align::Start);
    content.append(&runtime_panel);

    let status_label = gtk::Label::new(Some("状态: 未运行"));
    status_label.set_halign(Align::Start);
    content.append(&status_label);

    let log_label = gtk::Label::new(Some("日志"));
    log_label.set_halign(Align::Start);
    content.append(&log_label);
    let logs_text = gtk::TextView::new();
    logs_text.set_editable(false);
    logs_text.set_monospace(true);
    logs_text.set_vexpand(true);
    let logs_scroller = gtk::ScrolledWindow::new();
    logs_scroller.set_vexpand(true);
    logs_scroller.set_child(Some(&logs_text));
    content.append(&logs_scroller);

    root.append(&content);

    let (ui_tx, ui_rx) = unbounded::<UiMessage>();

    let ui_rx_clone = ui_rx.clone();
    let status_label_ui = status_label.clone();
    let logs_text_ui = logs_text.clone();
    let runtime_panel_ui = runtime_panel.clone();
    let mode_combo_ui = mode_combo.clone();
    let port_entry_ui = port_entry.clone();
    let screen_entry_ui = screen_entry.clone();
    let server_addr_entry_ui = server_addr_entry.clone();
    let client_name_entry_ui = client_name_entry.clone();
    let routes_text_ui = routes_text.clone();

    glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
        while let Ok(msg) = ui_rx_clone.try_recv() {
            match msg {
                UiMessage::Loaded(settings) => {
                    mode_combo_ui.set_active_id(Some(&settings.last_mode));
                    port_entry_ui.set_text(&settings.server_port.to_string());
                    screen_entry_ui.set_text(&settings.server_screen_name);
                    server_addr_entry_ui.set_text(&settings.server_address);
                    client_name_entry_ui.set_text(&settings.client_name);
                    routes_text_ui
                        .buffer()
                        .set_text(&routes_to_text(&settings.client_routes));
                }
                UiMessage::Runtime(caps) => {
                    runtime_panel_ui.set_text(&format!(
                        "运行环境: session={} backend={} x11={} wayland={} global_input={}",
                        caps.session_type,
                        caps.input_backend,
                        caps.has_x11_display,
                        caps.has_wayland_display,
                        caps.global_input_available
                    ));
                }
                UiMessage::Status(status) => {
                    let active_str = status.active_client.clone().unwrap_or_else(|| "无".to_string());
                    let input_hint = if status.mode == "server" && status.is_running {
                        if status.connected_clients > 0 && status.active_client.is_some() {
                            " [输入已转发]"
                        } else if status.connected_clients > 0 {
                            " [客户端已连接，点击「切换到首个客户端」激活]"
                        } else {
                            " [等待客户端连接]"
                        }
                    } else if status.mode == "client" && status.is_running {
                        " [等待 Server 激活]"
                    } else {
                        ""
                    };
                    status_label_ui.set_text(&format!(
                        "状态: {} | 运行={} | 客户端数={} | 激活={}{}",
                        status.mode,
                        if status.is_running { "是" } else { "否" },
                        status.connected_clients,
                        active_str,
                        input_hint
                    ));
                    let log_content = status.log_messages.join("\n");
                    logs_text_ui.buffer().set_text(&log_content);
                }
                UiMessage::Success(message) => {
                    let current = text_view_content(&logs_text_ui);
                    logs_text_ui
                        .buffer()
                        .set_text(&format_log_append(&current, &message));
                }
                UiMessage::Error(message) => {
                    let current = text_view_content(&logs_text_ui);
                    logs_text_ui
                        .buffer()
                        .set_text(&format_log_append(&current, &format!("ERROR: {message}")));
                }
            }
        }
        glib::ControlFlow::Continue
    });

    let bootstrap_core = core.clone();
    let bootstrap_tx = ui_tx.clone();
    runtime.spawn(async move {
        match bootstrap_core.reload_settings().await {
            Ok(settings) => {
                let _ = bootstrap_tx.send(UiMessage::Loaded(settings)).await;
            }
            Err(err) => {
                let _ = bootstrap_tx.send(UiMessage::Error(err)).await;
            }
        }
        let caps = bootstrap_core.get_runtime_capabilities();
        let _ = bootstrap_tx.send(UiMessage::Runtime(caps)).await;
        match bootstrap_core.get_status().await {
            Ok(status) => {
                let _ = bootstrap_tx.send(UiMessage::Status(status)).await;
            }
            Err(err) => {
                let _ = bootstrap_tx.send(UiMessage::Error(err)).await;
            }
        }
    });

    let poll_core = core.clone();
    let poll_rt = runtime.clone();
    let poll_tx = ui_tx.clone();
    glib::timeout_add_seconds_local(2, move || {
        let core = poll_core.clone();
        let tx = poll_tx.clone();
        poll_rt.spawn(async move {
            if let Ok(status) = core.get_status().await {
                let _ = tx.send(UiMessage::Status(status)).await;
            }
        });
        glib::ControlFlow::Continue
    });

    let start_core = core.clone();
    let start_rt = runtime.clone();
    let start_tx = ui_tx.clone();
    let start_mode = mode_combo.clone();
    let start_port = port_entry.clone();
    let start_screen = screen_entry.clone();
    let start_server_addr = server_addr_entry.clone();
    let start_client_name = client_name_entry.clone();
    let start_routes = routes_text.clone();
    start_btn.connect_clicked(move |_| {
        let mode = start_mode
            .active_id()
            .map(|x| x.to_string())
            .unwrap_or_else(|| "server".to_string());
        let server_port = start_port.text().parse::<u16>().unwrap_or(24800);
        let screen_name = start_screen.text().to_string();
        let server_addr = start_server_addr.text().to_string();
        let client_name = start_client_name.text().to_string();
        let routes = parse_routes(start_routes.buffer().text(
            &start_routes.buffer().start_iter(),
            &start_routes.buffer().end_iter(),
            false,
        ));
        let mut settings = AppSettings::default();
        settings.last_mode = mode.clone();
        settings.server_port = server_port;
        settings.server_screen_name = if screen_name.is_empty() {
            "server".to_string()
        } else {
            screen_name.clone()
        };
        settings.server_address = if server_addr.is_empty() {
            "localhost:24800".to_string()
        } else {
            server_addr.clone()
        };
        settings.client_name = if client_name.is_empty() {
            "client-1".to_string()
        } else {
            client_name.clone()
        };
        settings.client_routes = routes.clone();

        let core = start_core.clone();
        let tx = start_tx.clone();
        start_rt.spawn(async move {
            if let Err(err) = core.save_settings(settings).await {
                let _ = tx.send(UiMessage::Error(err)).await;
                return;
            }
            let result = if mode == "client" {
                core.start_client(ClientConfig {
                    server_addr: if server_addr.is_empty() {
                        "localhost:24800".to_string()
                    } else {
                        server_addr
                    },
                    screen_name: if client_name.is_empty() {
                        "client-1".to_string()
                    } else {
                        client_name
                    },
                    auto_reconnect: true,
                    reconnect_interval: 5,
                    enable_clipboard: true,
                    enable_drag_drop: true,
                })
                .await
            } else {
                core.start_server(ServerConfig {
                    screen_name: if screen_name.is_empty() {
                        "server".to_string()
                    } else {
                        screen_name
                    },
                    port: server_port,
                    max_clients: 10,
                    listen_address: Some(format!("0.0.0.0:{server_port}")),
                    enable_clipboard: true,
                    enable_drag_drop: true,
                })
                .await
            };

            match result {
                Ok(()) => {
                    let _ = tx.send(UiMessage::Success("服务启动成功".to_string())).await;
                    if let Ok(status) = core.get_status().await {
                        let _ = tx.send(UiMessage::Status(status)).await;
                    }
                }
                Err(err) => {
                    let _ = tx.send(UiMessage::Error(err)).await;
                }
            }
        });
    });

    let stop_core = core.clone();
    let stop_rt = runtime.clone();
    let stop_tx = ui_tx.clone();
    stop_btn.connect_clicked(move |_| {
        let core = stop_core.clone();
        let tx = stop_tx.clone();
        stop_rt.spawn(async move {
            match core.stop_service().await {
                Ok(()) => {
                    let _ = tx.send(UiMessage::Success("服务已停止".to_string())).await;
                    if let Ok(status) = core.get_status().await {
                        let _ = tx.send(UiMessage::Status(status)).await;
                    }
                }
                Err(err) => {
                    let _ = tx.send(UiMessage::Error(err)).await;
                }
            }
        });
    });

    let save_core = core.clone();
    let save_rt = runtime.clone();
    let save_tx = ui_tx.clone();
    let save_mode = mode_combo.clone();
    let save_port = port_entry.clone();
    let save_screen = screen_entry.clone();
    let save_server_addr = server_addr_entry.clone();
    let save_client_name = client_name_entry.clone();
    let save_routes = routes_text.clone();
    save_btn.connect_clicked(move |_| {
        let mode = save_mode
            .active_id()
            .map(|x| x.to_string())
            .unwrap_or_else(|| "server".to_string());
        let routes = parse_routes(save_routes.buffer().text(
            &save_routes.buffer().start_iter(),
            &save_routes.buffer().end_iter(),
            false,
        ));
        let settings = AppSettings {
            startup_restore: true,
            last_mode: mode,
            server_port: save_port.text().parse::<u16>().unwrap_or(24800),
            server_screen_name: save_screen.text().to_string(),
            server_address: save_server_addr.text().to_string(),
            client_name: save_client_name.text().to_string(),
            pointer_lock_enabled: true,
            client_routes: routes,
        };

        let core = save_core.clone();
        let tx = save_tx.clone();
        save_rt.spawn(async move {
            match core.save_settings(settings).await {
                Ok(()) => {
                    let _ = tx.send(UiMessage::Success("配置已保存".to_string())).await;
                }
                Err(err) => {
                    let _ = tx.send(UiMessage::Error(err)).await;
                }
            }
        });
    });

    let switch_core = core.clone();
    let switch_rt = runtime.clone();
    let switch_tx = ui_tx.clone();
    let switch_routes = routes_text.clone();
    switch_btn.connect_clicked(move |_| {
        let routes = parse_routes(switch_routes.buffer().text(
            &switch_routes.buffer().start_iter(),
            &switch_routes.buffer().end_iter(),
            false,
        ));
        let Some(first) = routes.first() else {
            send_ui_message(&switch_tx, UiMessage::Error("没有可切换的客户端映射".to_string()));
            return;
        };
        let request = SwitchClientRequest {
            edge: first.edge.as_str().to_string(),
            client_name: first.name.clone(),
            client_address: if first.address.is_empty() {
                None
            } else {
                Some(first.address.clone())
            },
            cursor_x: 0,
            cursor_y: 0,
        };
        let core = switch_core.clone();
        let tx = switch_tx.clone();
        switch_rt.spawn(async move {
            match core.switch_client(request).await {
                Ok(msg) => {
                    let _ = tx.send(UiMessage::Success(msg)).await;
                    if let Ok(status) = core.get_status().await {
                        let _ = tx.send(UiMessage::Status(status)).await;
                    }
                }
                Err(err) => {
                    let _ = tx.send(UiMessage::Error(err)).await;
                }
            }
        });
    });

    let back_core = core.clone();
    let back_rt = runtime.clone();
    let back_tx = ui_tx.clone();
    back_btn.connect_clicked(move |_| {
        let core = back_core.clone();
        let tx = back_tx.clone();
        back_rt.spawn(async move {
            match core.switch_back_local("manual".to_string()).await {
                Ok(msg) => {
                    let _ = tx.send(UiMessage::Success(msg)).await;
                    if let Ok(status) = core.get_status().await {
                        let _ = tx.send(UiMessage::Status(status)).await;
                    }
                }
                Err(err) => {
                    let _ = tx.send(UiMessage::Error(err)).await;
                }
            }
        });
    });

    window.set_content(Some(&root));
    window.present();
}

fn parse_routes(raw: glib::GString) -> Vec<ClientRoute> {
    raw.lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return None;
            }
            let parts: Vec<&str> = trimmed.split(',').map(|x| x.trim()).collect();
            if parts.len() < 3 {
                return None;
            }
            let edge = match parts[2] {
                "left" => ScreenEdge::Left,
                "right" => ScreenEdge::Right,
                "top" => ScreenEdge::Top,
                "bottom" => ScreenEdge::Bottom,
                _ => ScreenEdge::Right,
            };
            Some(ClientRoute {
                id: format!("route-{}", parts[0]),
                name: parts[0].to_string(),
                address: parts[1].to_string(),
                edge,
            })
        })
        .collect()
}

fn routes_to_text(routes: &[ClientRoute]) -> String {
    routes
        .iter()
        .map(|route| {
            format!(
                "{},{},{}",
                route.name,
                route.address,
                route.edge.as_str()
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn text_view_content(view: &gtk::TextView) -> String {
    view.buffer()
        .text(&view.buffer().start_iter(), &view.buffer().end_iter(), false)
        .to_string()
}

fn format_log_append(current: &str, next: &str) -> String {
    if current.trim().is_empty() {
        next.to_string()
    } else {
        format!("{current}\n{next}")
    }
}

fn send_ui_message(sender: &Sender<UiMessage>, message: UiMessage) {
    let _ = sender.send_blocking(message);
}
