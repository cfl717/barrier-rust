use crate::settings::SettingsStore;
use crate::types::{AppSettings, AppState, CoreEvent, RuntimeCapabilities, SwitchClientRequest};
use barrier_protocol::{BarrierClient, BarrierServer, ClientConfig, ClientsHandle, ServerConfig};
use std::env;
use std::io::ErrorKind;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, Mutex};
use tokio::task::JoinHandle;
use tokio::time::sleep;

#[cfg(target_os = "linux")]
use crate::global_input_linux::start_global_input_pipeline;

#[cfg(not(target_os = "linux"))]
fn start_global_input_pipeline(_state: Arc<Mutex<BarrierRuntimeState>>) {}

pub struct BarrierCore {
    state: Arc<Mutex<BarrierRuntimeState>>,
    event_tx: broadcast::Sender<CoreEvent>,
    settings_store: SettingsStore,
    global_input_started: AtomicBool,
}

pub(crate) struct BarrierRuntimeState {
    pub(crate) server: Option<Arc<Mutex<BarrierServer>>>,
    /// 独立的客户端句柄，可在不锁 server 的情况下查询
    clients_handle: Option<ClientsHandle>,
    client: Option<Arc<Mutex<BarrierClient>>>,
    server_task: Option<JoinHandle<()>>,
    client_task: Option<JoinHandle<()>>,
    mode: Option<String>,
    server_address: String,
    last_error: Option<String>,
    log_messages: Vec<String>,
    pub(crate) active_client: Option<String>,
    settings: AppSettings,
}

impl BarrierCore {
    pub fn new() -> Self {
        Self::with_settings_store(SettingsStore::default())
    }

    pub fn with_settings_store(settings_store: SettingsStore) -> Self {
        let settings = settings_store.load().unwrap_or_default();
        let (event_tx, _) = broadcast::channel(128);
        Self {
            state: Arc::new(Mutex::new(BarrierRuntimeState {
                server: None,
                clients_handle: None,
                client: None,
                server_task: None,
                client_task: None,
                mode: None,
                server_address: format!("localhost:{}", settings.server_port),
                last_error: None,
                log_messages: vec!["应用启动".to_string()],
                active_client: None,
                settings,
            })),
            event_tx,
            settings_store,
            global_input_started: AtomicBool::new(false),
        }
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<CoreEvent> {
        self.event_tx.subscribe()
    }

    pub async fn initialize_platform_features(&self) {
        if self
            .global_input_started
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            start_global_input_pipeline(self.state.clone());
        }
    }

    pub async fn get_settings(&self) -> AppSettings {
        let guard = self.state.lock().await;
        guard.settings.clone()
    }

    pub async fn reload_settings(&self) -> Result<AppSettings, String> {
        let settings = self.settings_store.load()?;
        {
            let mut guard = self.state.lock().await;
            guard.settings = settings.clone();
            append_log(&mut guard, format!("配置已加载: {}", self.settings_store.path().display()));
        }
        let _ = self.event_tx.send(CoreEvent::Settings(settings.clone()));
        self.broadcast_status().await;
        Ok(settings)
    }

    pub async fn save_settings(&self, settings: AppSettings) -> Result<(), String> {
        self.settings_store.save(&settings)?;
        {
            let mut guard = self.state.lock().await;
            guard.settings = settings.clone();
            append_log(&mut guard, format!("配置已保存: {}", self.settings_store.path().display()));
        }
        let _ = self.event_tx.send(CoreEvent::Settings(settings));
        self.broadcast_status().await;
        Ok(())
    }

    pub async fn restore_on_startup(&self) -> Result<(), String> {
        let settings = self.get_settings().await;
        if !settings.startup_restore {
            return Ok(());
        }
        match settings.last_mode.as_str() {
            "server" => self
                .start_server(ServerConfig {
                    screen_name: settings.server_screen_name,
                    port: settings.server_port,
                    max_clients: 10,
                    listen_address: Some(format!("0.0.0.0:{}", settings.server_port)),
                    enable_clipboard: true,
                    enable_drag_drop: true,
                })
                .await,
            "client" => self
                .start_client(ClientConfig {
                    server_addr: settings.server_address,
                    screen_name: settings.client_name,
                    auto_reconnect: true,
                    reconnect_interval: 5,
                    enable_clipboard: true,
                    enable_drag_drop: true,
                })
                .await,
            _ => Ok(()),
        }
    }

    pub fn get_runtime_capabilities(&self) -> RuntimeCapabilities {
        detect_runtime_capabilities()
    }

    pub async fn start_server(&self, config: ServerConfig) -> Result<(), String> {
        let state_handle = self.state.clone();
        let event_tx = self.event_tx.clone();
        let mut guard = self.state.lock().await;

        if let Some(task) = guard.client_task.take() {
            abort_task_and_wait(task).await;
        }
        guard.client = None;

        if let Some(task) = guard.server_task.take() {
            abort_task_and_wait(task).await;
        }

        let port = parse_port_from_server_config(&config);
        let bind_addr = format!("0.0.0.0:{port}");
        ensure_bind_address_available(&bind_addr).await?;

        let (server, _event_rx) = BarrierServer::new(config);
        let clients_handle = server.clients_handle();
        let server_arc = Arc::new(Mutex::new(server));
        guard.server = Some(server_arc.clone());
        guard.clients_handle = Some(clients_handle);
        guard.mode = Some("server".to_string());
        guard.active_client = None;
        guard.server_address = bind_addr.clone();
        guard.last_error = None;
        guard.settings.last_mode = "server".to_string();
        guard.settings.server_port = port;
        append_log(&mut guard, format!("Server starting on {}", bind_addr));
        if let Err(err) = self.settings_store.save(&guard.settings) {
            append_log(&mut guard, format!("配置保存失败: {}", err));
        }

        let task = tokio::spawn(async move {
            // 注意：不能长期持有 server_arc 的锁，否则会阻塞 get_status 等查询
            // 这里只在启动时短暂获取锁来调用 run()，run() 内部会自行管理
            let run_result = {
                let mut server = server_arc.lock().await;
                server.run().await
            };
            if let Err(e) = run_result {
                log::error!("Server error: {}", e);
                let mut state = state_handle.lock().await;
                state.last_error = Some(format!("Server error: {}", e));
                append_log(&mut state, format!("Server error: {}", e));
                let _ = event_tx.send(CoreEvent::Error(format!("Server error: {}", e)));
                let snapshot = state.to_app_state(0);
                let _ = event_tx.send(CoreEvent::Status(snapshot));
            }
        });
        guard.server_task = Some(task);
        append_log(&mut guard, "Server 正在监听，等待客户端连接…".to_string());
        append_log(&mut guard, "提示：客户端连接后，点击「切换到首个客户端」可激活输入转发".to_string());

        drop(guard);
        self.broadcast_status().await;
        Ok(())
    }

    pub async fn start_client(&self, config: ClientConfig) -> Result<(), String> {
        let state_handle = self.state.clone();
        let event_tx = self.event_tx.clone();
        let mut guard = self.state.lock().await;

        if let Some(task) = guard.server_task.take() {
            abort_task_and_wait(task).await;
        }
        guard.server = None;

        if let Some(task) = guard.client_task.take() {
            abort_task_and_wait(task).await;
        }

        let target_server = config.server_addr.clone();
        TcpStream::connect(&target_server)
            .await
            .map_err(|e| format!("无法连接服务端（{}）: {}", target_server, e))?;

        let client = BarrierClient::new(config.clone());
        let client_arc = Arc::new(Mutex::new(client));
        guard.client = Some(client_arc.clone());
        guard.mode = Some("client".to_string());
        guard.server_address = target_server;
        guard.active_client = None;
        guard.last_error = None;
        guard.settings.last_mode = "client".to_string();
        guard.settings.server_address = config.server_addr;
        guard.settings.client_name = config.screen_name;
        let starting_msg = format!("Client starting -> {}", guard.server_address);
        append_log(&mut guard, starting_msg);
        if let Err(err) = self.settings_store.save(&guard.settings) {
            append_log(&mut guard, format!("配置保存失败: {}", err));
        }

        let task = tokio::spawn(async move {
            let mut client = client_arc.lock().await;
            if let Err(e) = client.run().await {
                log::error!("Client error: {}", e);
                let mut state = state_handle.lock().await;
                state.last_error = Some(format!("Client error: {}", e));
                append_log(&mut state, format!("Client error: {}", e));
                let _ = event_tx.send(CoreEvent::Error(format!("Client error: {}", e)));
                let snapshot = state.to_app_state(0);
                let _ = event_tx.send(CoreEvent::Status(snapshot));
            }
        });
        guard.client_task = Some(task);
        append_log(&mut guard, "Client 已连接，等待 Server 激活输入…".to_string());

        drop(guard);
        self.broadcast_status().await;
        Ok(())
    }

    pub async fn stop_service(&self) -> Result<(), String> {
        let mut guard = self.state.lock().await;

        if let Some(task) = guard.server_task.take() {
            abort_task_and_wait(task).await;
        }
        if let Some(task) = guard.client_task.take() {
            abort_task_and_wait(task).await;
        }

        guard.server = None;
        guard.clients_handle = None;
        guard.client = None;
        guard.mode = None;
        guard.active_client = None;
        guard.last_error = None;
        append_log(&mut guard, "Service stopped".to_string());

        drop(guard);
        self.broadcast_status().await;
        Ok(())
    }

    pub async fn switch_client(&self, request: SwitchClientRequest) -> Result<String, String> {
        let (server_arc, previous_active_client) = {
            let state_guard = self.state.lock().await;
            (
                state_guard
                    .server
                    .clone()
                    .ok_or_else(|| "服务端未运行，无法切换客户端".to_string())?,
                state_guard.active_client.clone(),
            )
        };

        let requested_name = request.client_name.trim().to_string();
        if requested_name.is_empty() {
            return Err("客户端名称不能为空".to_string());
        }

        let mut switched_to = requested_name.clone();
        {
            let server = server_arc.lock().await;
            let connected_clients = server.get_clients().await;
            let connected_names: Vec<String> = connected_clients
                .iter()
                .map(|client| client.screen_name.clone())
                .collect();

            if !connected_names.iter().any(|name| name == &requested_name) {
                if connected_names.len() == 1 {
                    switched_to = connected_names[0].clone();
                } else if connected_names.is_empty() {
                    return Err("当前没有已连接客户端，请先启动并连接客户端".to_string());
                } else {
                    return Err(format!(
                        "目标客户端 `{}` 未连接。当前已连接客户端：{}",
                        requested_name,
                        connected_names.join(", ")
                    ));
                }
            }

            if let Some(previous) = previous_active_client.as_ref() {
                if previous != &switched_to {
                    let _ = server.send_leave(previous).await;
                }
            }
            server
                .send_enter(&switched_to, request.cursor_x, request.cursor_y)
                .await?;
        }

        let message = format!(
            "Edge switch [{}] -> {} ({})",
            request.edge,
            switched_to,
            request
                .client_address
                .clone()
                .unwrap_or_else(|| "未配置地址".to_string())
        );

        let mut state_guard = self.state.lock().await;
        state_guard.active_client = Some(switched_to);
        state_guard.last_error = None;
        append_log(&mut state_guard, message.clone());

        drop(state_guard);
        self.broadcast_status().await;
        Ok(message)
    }

    pub async fn switch_back_local(&self, edge: String) -> Result<String, String> {
        let (server_arc, current_active) = {
            let state_guard = self.state.lock().await;
            (
                state_guard
                    .server
                    .clone()
                    .ok_or_else(|| "服务端未运行，无法切回本机".to_string())?,
                state_guard.active_client.clone(),
            )
        };

        let active = current_active.ok_or_else(|| "当前没有激活客户端".to_string())?;
        {
            let server = server_arc.lock().await;
            server.send_leave(&active).await?;
        }

        let message = format!("Edge return [{}] -> local ({})", edge, active);
        let mut state_guard = self.state.lock().await;
        state_guard.active_client = None;
        append_log(&mut state_guard, message.clone());

        drop(state_guard);
        self.broadcast_status().await;
        Ok(message)
    }

    pub async fn relay_mouse_move(&self, dx: i16, dy: i16) -> Result<(), String> {
        if dx == 0 && dy == 0 {
            return Ok(());
        }
        let (server_arc, client_name) = {
            let state_guard = self.state.lock().await;
            (
                state_guard
                    .server
                    .clone()
                    .ok_or_else(|| "服务端未运行".to_string())?,
                state_guard
                    .active_client
                    .clone()
                    .ok_or_else(|| "当前没有激活客户端".to_string())?,
            )
        };
        let server = server_arc.lock().await;
        server.relay_mouse_move(&client_name, dx, dy).await
    }

    pub async fn relay_key_event(&self, key_code: u16, pressed: bool) -> Result<(), String> {
        let (server_arc, client_name) = {
            let state_guard = self.state.lock().await;
            (
                state_guard
                    .server
                    .clone()
                    .ok_or_else(|| "服务端未运行".to_string())?,
                state_guard
                    .active_client
                    .clone()
                    .ok_or_else(|| "当前没有激活客户端".to_string())?,
            )
        };
        let server = server_arc.lock().await;
        server.relay_key_event(&client_name, key_code, pressed).await
    }

    pub async fn relay_mouse_button(&self, button: u8, pressed: bool) -> Result<(), String> {
        let (server_arc, client_name) = {
            let state_guard = self.state.lock().await;
            (
                state_guard
                    .server
                    .clone()
                    .ok_or_else(|| "服务端未运行".to_string())?,
                state_guard
                    .active_client
                    .clone()
                    .ok_or_else(|| "当前没有激活客户端".to_string())?,
            )
        };
        let server = server_arc.lock().await;
        server.relay_mouse_button(&client_name, button, pressed).await
    }

    pub async fn get_status(&self) -> Result<AppState, String> {
        let (server_opt, clients_handle, client_opt, server_address, active_client, log_messages) = {
            let state_guard = self.state.lock().await;
            (
                state_guard.server.is_some(),
                state_guard.clients_handle.clone(),
                state_guard.client.is_some(),
                state_guard.server_address.clone(),
                state_guard.active_client.clone(),
                state_guard.log_messages.clone(),
            )
        };

        // 使用独立的 clients_handle 查询，避免锁 server
        let connected_clients = if let Some(handle) = &clients_handle {
            handle.count().await as u32
        } else {
            0
        };

        let status = if server_opt {
            AppState {
                mode: "server".to_string(),
                is_running: true,
                connected_clients,
                server_address,
                active_client,
                log_messages,
            }
        } else if client_opt {
            AppState {
                mode: "client".to_string(),
                is_running: true,
                connected_clients: 0,
                server_address,
                active_client: None,
                log_messages,
            }
        } else {
            AppState {
                mode: "idle".to_string(),
                is_running: false,
                connected_clients: 0,
                server_address,
                active_client: None,
                log_messages,
            }
        };
        Ok(status)
    }

    async fn broadcast_status(&self) {
        if let Ok(status) = self.get_status().await {
            let _ = self.event_tx.send(CoreEvent::Status(status));
        }
    }
}

impl BarrierRuntimeState {
    fn to_app_state(&self, connected_clients: u32) -> AppState {
        if self.server.is_some() {
            AppState {
                mode: "server".to_string(),
                is_running: true,
                connected_clients,
                server_address: self.server_address.clone(),
                active_client: self.active_client.clone(),
                log_messages: self.log_messages.clone(),
            }
        } else if self.client.is_some() {
            AppState {
                mode: "client".to_string(),
                is_running: true,
                connected_clients: 0,
                server_address: self.server_address.clone(),
                active_client: None,
                log_messages: self.log_messages.clone(),
            }
        } else {
            AppState::default()
        }
    }
}

fn append_log(state: &mut BarrierRuntimeState, message: String) {
    state.log_messages.push(message);
    if state.log_messages.len() > 200 {
        let overflow = state.log_messages.len() - 200;
        state.log_messages.drain(0..overflow);
    }
}

fn parse_port_from_server_config(config: &ServerConfig) -> u16 {
    if let Some(addr) = &config.listen_address {
        if let Some(port_str) = addr.split(':').next_back() {
            if let Ok(port) = port_str.parse::<u16>() {
                return port;
            }
        }
    }
    config.port
}

async fn abort_task_and_wait(task: JoinHandle<()>) {
    task.abort();
    if let Err(join_err) = task.await {
        if !join_err.is_cancelled() {
            log::warn!("后台任务停止时出现异常: {}", join_err);
        }
    }
}

async fn ensure_bind_address_available(bind_addr: &str) -> Result<(), String> {
    const MAX_BIND_RETRIES: usize = 6;
    const RETRY_DELAY: Duration = Duration::from_millis(120);

    for attempt in 0..MAX_BIND_RETRIES {
        match TcpListener::bind(bind_addr).await {
            Ok(listener) => {
                drop(listener);
                return Ok(());
            }
            Err(err) if err.kind() == ErrorKind::AddrInUse && attempt + 1 < MAX_BIND_RETRIES => {
                sleep(RETRY_DELAY).await;
            }
            Err(err) if err.kind() == ErrorKind::AddrInUse => {
                return Err(format!(
                    "无法启动服务端（{}）: 地址已在使用。请关闭占用该端口的进程，或换一个监听端口后重试。",
                    bind_addr
                ));
            }
            Err(err) => return Err(format!("无法启动服务端（{}）: {}", bind_addr, err)),
        }
    }

    Err(format!("无法启动服务端（{}）: 端口检查失败", bind_addr))
}

fn detect_runtime_capabilities() -> RuntimeCapabilities {
    let session_type = env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "unknown".to_string());
    let has_x11_display = env::var("DISPLAY").is_ok();
    let has_wayland_display = env::var("WAYLAND_DISPLAY").is_ok();
    detect_runtime_capabilities_from_flags(session_type, has_x11_display, has_wayland_display)
}

fn detect_runtime_capabilities_from_flags(
    session_type: String,
    has_x11_display: bool,
    has_wayland_display: bool,
) -> RuntimeCapabilities {
    let mut recommendations = Vec::new();
    let mut blocking_issues = Vec::new();
    let input_backend;

    if has_x11_display {
        input_backend = "x11".to_string();
        recommendations.push("检测到 X11 环境，可优先使用 X11 输入链路。".to_string());
        if cfg!(target_os = "linux") {
            recommendations.push(
                "本应用在 Linux 上可使用后端全局键鼠监听（rdev）转发到远程客户端。".to_string(),
            );
        }
    } else if has_wayland_display {
        input_backend = "wayland-limited".to_string();
        recommendations.push(
            "检测到 Wayland 环境：全局键鼠捕获/注入能力受 compositor 安全策略限制。".to_string(),
        );
        recommendations.push("如需完整 Barrier 行为，建议切换到 X11 会话，或后续接入 portal/libei。".to_string());
    } else {
        input_backend = "none".to_string();
        blocking_issues.push("未检测到 DISPLAY/WAYLAND_DISPLAY，GUI 输入能力不可用。".to_string());
        recommendations.push("请在桌面图形会话中运行，或检查远程会话环境变量。".to_string());
    }

    RuntimeCapabilities {
        session_type,
        has_x11_display,
        has_wayland_display,
        input_backend,
        global_input_available: cfg!(target_os = "linux") && has_x11_display,
        recommendations,
        blocking_issues,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ScreenEdge;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_settings_roundtrip() {
        let dir = tempdir().expect("tempdir");
        let store = SettingsStore::new(dir.path().join("settings.toml"));
        let core = BarrierCore::with_settings_store(store.clone());
        let mut settings = core.get_settings().await;
        settings.client_name = "test-client".to_string();
        settings.client_routes[0].edge = ScreenEdge::Left;
        core.save_settings(settings.clone()).await.expect("save settings");

        let loaded = store.load().expect("load settings");
        assert_eq!(loaded.client_name, "test-client");
        assert_eq!(loaded.client_routes[0].edge.as_str(), "left");
    }

    #[tokio::test]
    async fn test_default_status_not_running() {
        let dir = tempdir().expect("tempdir");
        let store = SettingsStore::new(dir.path().join("settings.toml"));
        let core = BarrierCore::with_settings_store(store);
        let status = core.get_status().await.expect("status");
        assert!(!status.is_running);
    }

    #[tokio::test]
    async fn test_switch_client_requires_server() {
        let dir = tempdir().expect("tempdir");
        let store = SettingsStore::new(dir.path().join("settings.toml"));
        let core = BarrierCore::with_settings_store(store);
        let err = core
            .switch_client(SwitchClientRequest {
                edge: "left".to_string(),
                client_name: "client-1".to_string(),
                client_address: None,
                cursor_x: 0,
                cursor_y: 0,
            })
            .await
            .expect_err("must fail");
        assert!(err.contains("服务端未运行"));
    }

    #[test]
    fn test_runtime_capability_flags() {
        let x11_caps = detect_runtime_capabilities_from_flags("x11".to_string(), true, false);
        assert_eq!(x11_caps.input_backend, "x11");
        assert!(x11_caps.recommendations.iter().any(|x| x.contains("X11")));

        let none_caps = detect_runtime_capabilities_from_flags("unknown".to_string(), false, false);
        assert_eq!(none_caps.input_backend, "none");
        assert!(!none_caps.blocking_issues.is_empty());
    }
}
