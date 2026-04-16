use adw::prelude::*;
use anyhow::Result;
use async_channel::unbounded;
use app_core::{
    AppSettings, AppState, BarrierCore, ClientConfig, ClientRoute, RuntimeCapabilities, ScreenEdge,
    ServerConfig, SwitchClientRequest,
};
use gtk::{glib, Align, Orientation};
use std::cell::RefCell;
use std::rc::Rc;
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
        .application_id("org.barrier.native.gtk")
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
        .title("Barrier")
        .default_width(700)
        .default_height(580)
        .build();

    let header_bar = adw::HeaderBar::new();
    let root = gtk::Box::new(Orientation::Vertical, 0);
    root.append(&header_bar);

    let content = gtk::Box::new(Orientation::Vertical, 12);
    content.set_margin_top(16);
    content.set_margin_bottom(16);
    content.set_margin_start(16);
    content.set_margin_end(16);

    // === 模式选择 ===
    let mode_box = gtk::Box::new(Orientation::Horizontal, 12);
    let server_radio = gtk::CheckButton::with_label("Server（共享本机键鼠）");
    let client_radio = gtk::CheckButton::with_label("Client（接收远程键鼠）");
    client_radio.set_group(Some(&server_radio));
    server_radio.set_active(true);
    mode_box.append(&server_radio);
    mode_box.append(&client_radio);
    content.append(&mode_box);

    // === Server 配置面板 ===
    let server_panel = gtk::Box::new(Orientation::Vertical, 8);
    server_panel.add_css_class("card");
    server_panel.set_margin_top(8);

    let server_title = gtk::Label::new(Some("Server 配置"));
    server_title.add_css_class("title-4");
    server_title.set_halign(Align::Start);
    server_panel.append(&server_title);

    let server_grid = gtk::Grid::new();
    server_grid.set_row_spacing(8);
    server_grid.set_column_spacing(12);

    server_grid.attach(&gtk::Label::new(Some("监听端口:")), 0, 0, 1, 1);
    let port_entry = gtk::Entry::new();
    port_entry.set_text("24800");
    port_entry.set_hexpand(true);
    server_grid.attach(&port_entry, 1, 0, 1, 1);

    server_grid.attach(&gtk::Label::new(Some("屏幕名称:")), 0, 1, 1, 1);
    let screen_entry = gtk::Entry::new();
    screen_entry.set_text("server");
    screen_entry.set_hexpand(true);
    server_grid.attach(&screen_entry, 1, 1, 1, 1);

    server_panel.append(&server_grid);

    // 客户端路由列表
    let routes_title = gtk::Label::new(Some("客户端映射:"));
    routes_title.set_halign(Align::Start);
    routes_title.set_margin_top(8);
    server_panel.append(&routes_title);

    let routes_list = gtk::ListBox::new();
    routes_list.set_selection_mode(gtk::SelectionMode::None);
    routes_list.add_css_class("boxed-list");

    let routes_store: Rc<RefCell<Vec<(gtk::Entry, gtk::Entry, gtk::ComboBoxText)>>> =
        Rc::new(RefCell::new(Vec::new()));

    let routes_list_clone = routes_list.clone();
    let routes_store_clone = routes_store.clone();
    let add_route_row = move |name: &str, address: &str, edge: &str| {
        let row = gtk::ListBoxRow::new();
        let hbox = gtk::Box::new(Orientation::Horizontal, 8);
        hbox.set_margin_top(4);
        hbox.set_margin_bottom(4);
        hbox.set_margin_start(8);
        hbox.set_margin_end(8);

        let name_entry = gtk::Entry::new();
        name_entry.set_placeholder_text(Some("客户端名"));
        name_entry.set_text(name);
        name_entry.set_width_chars(12);

        let addr_entry = gtk::Entry::new();
        addr_entry.set_placeholder_text(Some("地址:端口"));
        addr_entry.set_text(address);
        addr_entry.set_hexpand(true);

        let edge_combo = gtk::ComboBoxText::new();
        edge_combo.append(Some("left"), "← 左");
        edge_combo.append(Some("right"), "→ 右");
        edge_combo.append(Some("top"), "↑ 上");
        edge_combo.append(Some("bottom"), "↓ 下");
        edge_combo.set_active_id(Some(edge));

        hbox.append(&name_entry);
        hbox.append(&addr_entry);
        hbox.append(&edge_combo);

        row.set_child(Some(&hbox));
        routes_list_clone.append(&row);
        routes_store_clone
            .borrow_mut()
            .push((name_entry, addr_entry, edge_combo));
    };

    add_route_row("client-1", "192.168.1.101:24800", "right");

    let routes_scroll = gtk::ScrolledWindow::new();
    routes_scroll.set_min_content_height(100);
    routes_scroll.set_max_content_height(150);
    routes_scroll.set_child(Some(&routes_list));
    server_panel.append(&routes_scroll);

    let routes_btn_box = gtk::Box::new(Orientation::Horizontal, 8);
    let add_route_btn = gtk::Button::with_label("+ 添加客户端");
    routes_btn_box.append(&add_route_btn);
    server_panel.append(&routes_btn_box);

    content.append(&server_panel);

    // === Client 配置面板 ===
    let client_panel = gtk::Box::new(Orientation::Vertical, 8);
    client_panel.add_css_class("card");
    client_panel.set_margin_top(8);
    client_panel.set_visible(false);

    let client_title = gtk::Label::new(Some("Client 配置"));
    client_title.add_css_class("title-4");
    client_title.set_halign(Align::Start);
    client_panel.append(&client_title);

    let client_grid = gtk::Grid::new();
    client_grid.set_row_spacing(8);
    client_grid.set_column_spacing(12);

    client_grid.attach(&gtk::Label::new(Some("Server 地址:")), 0, 0, 1, 1);
    let server_addr_entry = gtk::Entry::new();
    server_addr_entry.set_text("192.168.1.100:24800");
    server_addr_entry.set_hexpand(true);
    server_addr_entry.set_placeholder_text(Some("如 192.168.1.100:24800"));
    client_grid.attach(&server_addr_entry, 1, 0, 1, 1);

    client_grid.attach(&gtk::Label::new(Some("本机名称:")), 0, 1, 1, 1);
    let client_name_entry = gtk::Entry::new();
    client_name_entry.set_text("client-1");
    client_name_entry.set_hexpand(true);
    client_grid.attach(&client_name_entry, 1, 1, 1, 1);

    client_panel.append(&client_grid);
    content.append(&client_panel);

    // === 操作按钮 ===
    let actions = gtk::Box::new(Orientation::Horizontal, 8);
    actions.set_margin_top(12);
    let start_btn = gtk::Button::with_label("启动");
    start_btn.add_css_class("suggested-action");
    let stop_btn = gtk::Button::with_label("停止");
    stop_btn.add_css_class("destructive-action");
    let save_btn = gtk::Button::with_label("保存配置");
    actions.append(&start_btn);
    actions.append(&stop_btn);
    actions.append(&save_btn);
    content.append(&actions);

    // Server 模式下的切换按钮
    let switch_actions = gtk::Box::new(Orientation::Horizontal, 8);
    let switch_btn = gtk::Button::with_label("切换到首个客户端");
    let back_btn = gtk::Button::with_label("切回本机");
    switch_actions.append(&switch_btn);
    switch_actions.append(&back_btn);
    content.append(&switch_actions);

    // === 状态显示 ===
    let status_frame = gtk::Frame::new(Some("状态"));
    status_frame.set_margin_top(12);
    let status_box = gtk::Box::new(Orientation::Vertical, 4);
    status_box.set_margin_top(8);
    status_box.set_margin_bottom(8);
    status_box.set_margin_start(8);
    status_box.set_margin_end(8);

    let runtime_panel = gtk::Label::new(Some("运行环境: 检测中..."));
    runtime_panel.set_halign(Align::Start);
    runtime_panel.set_wrap(true);
    status_box.append(&runtime_panel);

    let status_label = gtk::Label::new(Some("服务状态: 未运行"));
    status_label.set_halign(Align::Start);
    status_box.append(&status_label);

    status_frame.set_child(Some(&status_box));
    content.append(&status_frame);

    // === 日志 ===
    let log_frame = gtk::Frame::new(Some("日志"));
    log_frame.set_vexpand(true);
    let logs_text = gtk::TextView::new();
    logs_text.set_editable(false);
    logs_text.set_monospace(true);
    logs_text.set_wrap_mode(gtk::WrapMode::WordChar);
    let logs_scroller = gtk::ScrolledWindow::new();
    logs_scroller.set_vexpand(true);
    logs_scroller.set_min_content_height(200);
    logs_scroller.set_child(Some(&logs_text));
    log_frame.set_child(Some(&logs_scroller));
    content.append(&log_frame);

    root.append(&content);

    // === 模式切换逻辑 ===
    let server_panel_ref = server_panel.clone();
    let client_panel_ref = client_panel.clone();
    let switch_actions_ref = switch_actions.clone();

    server_radio.connect_toggled({
        let server_panel = server_panel_ref.clone();
        let client_panel = client_panel_ref.clone();
        let switch_actions = switch_actions_ref.clone();
        move |radio| {
            let is_server = radio.is_active();
            server_panel.set_visible(is_server);
            client_panel.set_visible(!is_server);
            switch_actions.set_visible(is_server);
        }
    });

    // === 消息通道 ===
    let (ui_tx, ui_rx) = unbounded::<UiMessage>();

    // 添加路由按钮
    let routes_list_for_add = routes_list.clone();
    let routes_store_for_add = routes_store.clone();
    add_route_btn.connect_clicked(move |_| {
        let row = gtk::ListBoxRow::new();
        let hbox = gtk::Box::new(Orientation::Horizontal, 8);
        hbox.set_margin_top(4);
        hbox.set_margin_bottom(4);
        hbox.set_margin_start(8);
        hbox.set_margin_end(8);

        let name_entry = gtk::Entry::new();
        name_entry.set_placeholder_text(Some("客户端名"));
        name_entry.set_width_chars(12);

        let addr_entry = gtk::Entry::new();
        addr_entry.set_placeholder_text(Some("地址:端口"));
        addr_entry.set_hexpand(true);

        let edge_combo = gtk::ComboBoxText::new();
        edge_combo.append(Some("left"), "← 左");
        edge_combo.append(Some("right"), "→ 右");
        edge_combo.append(Some("top"), "↑ 上");
        edge_combo.append(Some("bottom"), "↓ 下");
        edge_combo.set_active_id(Some("right"));

        hbox.append(&name_entry);
        hbox.append(&addr_entry);
        hbox.append(&edge_combo);

        row.set_child(Some(&hbox));
        routes_list_for_add.append(&row);
        routes_store_for_add
            .borrow_mut()
            .push((name_entry, addr_entry, edge_combo));
    });

    // UI 消息处理
    let ui_rx_clone = ui_rx.clone();
    let status_label_ui = status_label.clone();
    let logs_text_ui = logs_text.clone();
    let runtime_panel_ui = runtime_panel.clone();
    let server_radio_ui = server_radio.clone();
    let port_entry_ui = port_entry.clone();
    let screen_entry_ui = screen_entry.clone();
    let server_addr_entry_ui = server_addr_entry.clone();
    let client_name_entry_ui = client_name_entry.clone();
    let routes_list_ui = routes_list.clone();
    let routes_store_ui = routes_store.clone();
    let server_panel_ui = server_panel_ref.clone();
    let client_panel_ui = client_panel_ref.clone();
    let switch_actions_ui = switch_actions_ref.clone();

    glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
        while let Ok(msg) = ui_rx_clone.try_recv() {
            match msg {
                UiMessage::Loaded(settings) => {
                    let is_server = settings.last_mode == "server";
                    server_radio_ui.set_active(is_server);
                    server_panel_ui.set_visible(is_server);
                    client_panel_ui.set_visible(!is_server);
                    switch_actions_ui.set_visible(is_server);

                    port_entry_ui.set_text(&settings.server_port.to_string());
                    screen_entry_ui.set_text(&settings.server_screen_name);
                    server_addr_entry_ui.set_text(&settings.server_address);
                    client_name_entry_ui.set_text(&settings.client_name);

                    // 清空并重建路由列表
                    while let Some(row) = routes_list_ui.first_child() {
                        routes_list_ui.remove(&row);
                    }
                    routes_store_ui.borrow_mut().clear();

                    for route in &settings.client_routes {
                        let row = gtk::ListBoxRow::new();
                        let hbox = gtk::Box::new(Orientation::Horizontal, 8);
                        hbox.set_margin_top(4);
                        hbox.set_margin_bottom(4);
                        hbox.set_margin_start(8);
                        hbox.set_margin_end(8);

                        let name_entry = gtk::Entry::new();
                        name_entry.set_text(&route.name);
                        name_entry.set_width_chars(12);

                        let addr_entry = gtk::Entry::new();
                        addr_entry.set_text(&route.address);
                        addr_entry.set_hexpand(true);

                        let edge_combo = gtk::ComboBoxText::new();
                        edge_combo.append(Some("left"), "← 左");
                        edge_combo.append(Some("right"), "→ 右");
                        edge_combo.append(Some("top"), "↑ 上");
                        edge_combo.append(Some("bottom"), "↓ 下");
                        edge_combo.set_active_id(Some(route.edge.as_str()));

                        hbox.append(&name_entry);
                        hbox.append(&addr_entry);
                        hbox.append(&edge_combo);
                        row.set_child(Some(&hbox));
                        routes_list_ui.append(&row);
                        routes_store_ui
                            .borrow_mut()
                            .push((name_entry, addr_entry, edge_combo));
                    }
                }
                UiMessage::Runtime(caps) => {
                    let global_str = if caps.global_input_available {
                        "可用"
                    } else {
                        "不可用"
                    };
                    runtime_panel_ui.set_text(&format!(
                        "运行环境: {} | 输入后端: {} | 全局输入: {}",
                        caps.session_type, caps.input_backend, global_str
                    ));
                }
                UiMessage::Status(status) => {
                    let active_str = status
                        .active_client
                        .clone()
                        .unwrap_or_else(|| "无".to_string());
                    let hint = if status.mode == "server" && status.is_running {
                        if status.connected_clients > 0 && status.active_client.is_some() {
                            " ✓ 输入已转发"
                        } else if status.connected_clients > 0 {
                            " ⚠ 点击「切换到首个客户端」激活"
                        } else {
                            " ○ 等待客户端连接"
                        }
                    } else if status.mode == "client" && status.is_running {
                        " ○ 等待 Server 激活"
                    } else {
                        ""
                    };
                    status_label_ui.set_text(&format!(
                        "服务状态: {} | 客户端: {} | 激活: {}{}",
                        if status.is_running { "运行中" } else { "已停止" },
                        status.connected_clients,
                        active_str,
                        hint
                    ));
                    let log_content = status.log_messages.join("\n");
                    let buf = logs_text_ui.buffer();
                    buf.set_text(&log_content);
                    // 自动滚动到底部
                    let end_iter = buf.end_iter();
                    if let Some(mark) = buf.mark("end_mark") {
                        buf.move_mark(&mark, &end_iter);
                    } else {
                        buf.create_mark(Some("end_mark"), &end_iter, false);
                    }
                    logs_text_ui.scroll_to_mark(&buf.mark("end_mark").unwrap(), 0.0, false, 0.0, 1.0);
                }
                UiMessage::Success(message) => {
                    let buf = logs_text_ui.buffer();
                    let mut end = buf.end_iter();
                    buf.insert(&mut end, &format!("\n{}", message));
                }
                UiMessage::Error(message) => {
                    let buf = logs_text_ui.buffer();
                    let mut end = buf.end_iter();
                    buf.insert(&mut end, &format!("\nERROR: {}", message));
                }
            }
        }
        glib::ControlFlow::Continue
    });

    // Bootstrap
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
        if let Ok(status) = bootstrap_core.get_status().await {
            let _ = bootstrap_tx.send(UiMessage::Status(status)).await;
        }
    });

    // 定时轮询状态
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

    // 获取路由数据
    let get_routes = {
        let routes_store = routes_store.clone();
        move || -> Vec<ClientRoute> {
            routes_store
                .borrow()
                .iter()
                .enumerate()
                .filter_map(|(i, (name_e, addr_e, edge_c))| {
                    let name = name_e.text().to_string();
                    let address = addr_e.text().to_string();
                    if name.is_empty() {
                        return None;
                    }
                    let edge = match edge_c.active_id().map(|s| s.to_string()).as_deref() {
                        Some("left") => ScreenEdge::Left,
                        Some("top") => ScreenEdge::Top,
                        Some("bottom") => ScreenEdge::Bottom,
                        _ => ScreenEdge::Right,
                    };
                    Some(ClientRoute {
                        id: format!("route-{}", i),
                        name,
                        address,
                        edge,
                    })
                })
                .collect()
        }
    };

    // 启动按钮
    let start_core = core.clone();
    let start_rt = runtime.clone();
    let start_tx = ui_tx.clone();
    let start_server_radio = server_radio.clone();
    let start_port = port_entry.clone();
    let start_screen = screen_entry.clone();
    let start_server_addr = server_addr_entry.clone();
    let start_client_name = client_name_entry.clone();
    let start_get_routes = get_routes.clone();
    start_btn.connect_clicked(move |_| {
        let is_server = start_server_radio.is_active();
        let server_port = start_port.text().parse::<u16>().unwrap_or(24800);
        let screen_name = start_screen.text().to_string();
        let server_addr = start_server_addr.text().to_string();
        let client_name = start_client_name.text().to_string();
        let routes = start_get_routes();

        let core = start_core.clone();
        let tx = start_tx.clone();
        start_rt.spawn(async move {
            let result = if is_server {
                core.start_server(ServerConfig {
                    screen_name: if screen_name.is_empty() {
                        "server".to_string()
                    } else {
                        screen_name
                    },
                    port: server_port,
                    max_clients: routes.len().max(1),
                    listen_address: Some(format!("0.0.0.0:{}", server_port)),
                    enable_clipboard: true,
                    enable_drag_drop: true,
                })
                .await
            } else {
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
            };
            match result {
                Ok(()) => {
                    let _ = tx.send(UiMessage::Success("服务启动成功".to_string())).await;
                }
                Err(err) => {
                    let _ = tx.send(UiMessage::Error(err)).await;
                }
            }
        });
    });

    // 停止按钮
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
                }
                Err(err) => {
                    let _ = tx.send(UiMessage::Error(err)).await;
                }
            }
        });
    });

    // 保存配置
    let save_core = core.clone();
    let save_rt = runtime.clone();
    let save_tx = ui_tx.clone();
    let save_server_radio = server_radio.clone();
    let save_port = port_entry.clone();
    let save_screen = screen_entry.clone();
    let save_server_addr = server_addr_entry.clone();
    let save_client_name = client_name_entry.clone();
    let save_get_routes = get_routes.clone();
    save_btn.connect_clicked(move |_| {
        let is_server = save_server_radio.is_active();
        let settings = AppSettings {
            startup_restore: true,
            last_mode: if is_server { "server" } else { "client" }.to_string(),
            server_port: save_port.text().parse::<u16>().unwrap_or(24800),
            server_screen_name: save_screen.text().to_string(),
            server_address: save_server_addr.text().to_string(),
            client_name: save_client_name.text().to_string(),
            pointer_lock_enabled: true,
            client_routes: save_get_routes(),
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

    // 切换客户端
    let switch_core = core.clone();
    let switch_rt = runtime.clone();
    let switch_tx = ui_tx.clone();
    let switch_get_routes = get_routes.clone();
    switch_btn.connect_clicked(move |_| {
        let routes = switch_get_routes();
        let Some(first) = routes.first() else {
            let tx = switch_tx.clone();
            glib::spawn_future_local(async move {
                let _ = tx.send(UiMessage::Error("没有配置客户端映射".to_string())).await;
            });
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
                }
                Err(err) => {
                    let _ = tx.send(UiMessage::Error(err)).await;
                }
            }
        });
    });

    // 切回本机
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
