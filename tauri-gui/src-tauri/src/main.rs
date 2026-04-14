// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Arc;
use tauri::Manager;
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;
use tokio::sync::Mutex;
use barrier_protocol::{BarrierServer, BarrierClient, ServerConfig, ClientConfig};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppState {
    pub mode: String, // "server" or "client"
    pub is_running: bool,
    pub connected_clients: u32,
    pub server_address: String,
    pub log_messages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeCapabilities {
    pub session_type: String,
    pub has_x11_display: bool,
    pub has_wayland_display: bool,
    pub input_backend: String,
    /// Linux + DISPLAY：后端可用 rdev 做全局键鼠转发（与网页窗口无关）。
    pub global_input_available: bool,
    pub recommendations: Vec<String>,
    pub blocking_issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchClientRequest {
    pub edge: String,
    pub client_name: String,
    pub client_address: Option<String>,
    pub cursor_x: i16,
    pub cursor_y: i16,
}

impl Default for AppState {
    fn default() -> Self {
        AppState {
            mode: "server".to_string(),
            is_running: false,
            connected_clients: 0,
            server_address: String::new(),
            log_messages: Vec::new(),
        }
    }
}

pub(crate) struct BarrierState {
    server: Option<Arc<Mutex<BarrierServer>>>,
    client: Option<Arc<Mutex<BarrierClient>>>,
    server_task: Option<JoinHandle<()>>,
    client_task: Option<JoinHandle<()>>,
    mode: Option<String>,
    server_address: String,
    last_error: Option<String>,
    log_messages: Vec<String>,
    active_client: Option<String>,
}

#[cfg(target_os = "linux")]
mod global_input_linux;

#[cfg(target_os = "linux")]
use global_input_linux::start_global_input_pipeline;

#[cfg(not(target_os = "linux"))]
fn start_global_input_pipeline(_state: Arc<Mutex<BarrierState>>) {}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn get_runtime_capabilities() -> RuntimeCapabilities {
    detect_runtime_capabilities()
}

#[tauri::command]
async fn start_server(state: tauri::State<'_, Arc<Mutex<BarrierState>>>, config: ServerConfig) -> Result<(), String> {
    let state_handle = state.inner().clone();
    let mut state_guard = state.lock().await;

    if let Some(task) = state_guard.client_task.take() {
        task.abort();
    }
    state_guard.client = None;

    if let Some(task) = state_guard.server_task.take() {
        task.abort();
    }

    let port = parse_port_from_server_config(&config);
    let bind_addr = format!("0.0.0.0:{port}");
    let listener = TcpListener::bind(&bind_addr)
        .await
        .map_err(|e| format!("无法启动服务端（{}）: {}", bind_addr, e))?;
    drop(listener);

    let (server, _event_rx) = BarrierServer::new(config);
    let server_arc = Arc::new(Mutex::new(server));
    state_guard.server = Some(server_arc.clone());
    state_guard.mode = Some("server".to_string());
    state_guard.active_client = None;
    state_guard.server_address = bind_addr.clone();
    state_guard.last_error = None;
    append_log(&mut state_guard, format!("Server starting on {}", bind_addr));

    // Start server in background
    let task = tokio::spawn(async move {
        let mut server = server_arc.lock().await;
        if let Err(e) = server.run().await {
            log::error!("Server error: {}", e);
            let mut state = state_handle.lock().await;
            state.last_error = Some(format!("Server error: {}", e));
            append_log(&mut state, format!("Server error: {}", e));
        }
    });
    state_guard.server_task = Some(task);

    Ok(())
}

#[tauri::command]
async fn start_client(state: tauri::State<'_, Arc<Mutex<BarrierState>>>, config: ClientConfig) -> Result<(), String> {
    let state_handle = state.inner().clone();
    let mut state_guard = state.lock().await;

    if let Some(task) = state_guard.server_task.take() {
        task.abort();
    }
    state_guard.server = None;

    if let Some(task) = state_guard.client_task.take() {
        task.abort();
    }

    let target_server = config.server_addr.clone();
    TcpStream::connect(&target_server)
        .await
        .map_err(|e| format!("无法连接服务端（{}）: {}", target_server, e))?;

    let client = BarrierClient::new(config);
    let client_arc = Arc::new(Mutex::new(client));
    state_guard.client = Some(client_arc.clone());
    state_guard.mode = Some("client".to_string());
    state_guard.server_address = target_server;
    let current_server_address = state_guard.server_address.clone();
    state_guard.last_error = None;
    append_log(&mut state_guard, format!("Client starting -> {}", current_server_address));

    // Start client in background
    let task = tokio::spawn(async move {
        let mut client = client_arc.lock().await;
        if let Err(e) = client.run().await {
            log::error!("Client error: {}", e);
            let mut state = state_handle.lock().await;
            state.last_error = Some(format!("Client error: {}", e));
            append_log(&mut state, format!("Client error: {}", e));
        }
    });
    state_guard.client_task = Some(task);

    Ok(())
}

#[tauri::command]
async fn stop_service(state: tauri::State<'_, Arc<Mutex<BarrierState>>>) -> Result<(), String> {
    let mut state_guard = state.lock().await;

    if let Some(task) = state_guard.server_task.take() {
        task.abort();
    }
    if let Some(task) = state_guard.client_task.take() {
        task.abort();
    }

    state_guard.server = None;
    state_guard.client = None;
    state_guard.mode = None;
    state_guard.active_client = None;
    state_guard.last_error = None;
    append_log(&mut state_guard, "Service stopped".to_string());

    Ok(())
}

#[tauri::command]
async fn switch_client(
    state: tauri::State<'_, Arc<Mutex<BarrierState>>>,
    request: SwitchClientRequest,
) -> Result<String, String> {
    let (server_arc, previous_active_client) = {
        let state_guard = state.lock().await;
        (
            state_guard
                .server
                .clone()
                .ok_or_else(|| "服务端未运行，无法切换客户端".to_string())?,
            state_guard.active_client.clone(),
        )
    };

    {
        let server = server_arc.lock().await;
        if let Some(previous) = previous_active_client.as_ref() {
            if previous != &request.client_name {
                let _ = server.send_leave(previous).await;
            }
        }
        server
            .send_enter(&request.client_name, request.cursor_x, request.cursor_y)
            .await?;
    }

    let message = format!(
        "Edge switch [{}] -> {} ({})",
        request.edge,
        request.client_name,
        request
            .client_address
            .clone()
            .unwrap_or_else(|| "未配置地址".to_string())
    );

    let mut state_guard = state.lock().await;
    state_guard.active_client = Some(request.client_name.clone());
    state_guard.last_error = None;
    append_log(&mut state_guard, message.clone());
    Ok(message)
}

#[tauri::command]
async fn switch_back_local(
    state: tauri::State<'_, Arc<Mutex<BarrierState>>>,
    edge: String,
) -> Result<String, String> {
    let (server_arc, current_active) = {
        let state_guard = state.lock().await;
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
    let mut state_guard = state.lock().await;
    state_guard.active_client = None;
    append_log(&mut state_guard, message.clone());
    Ok(message)
}

#[tauri::command]
async fn relay_mouse_move(
    state: tauri::State<'_, Arc<Mutex<BarrierState>>>,
    dx: i16,
    dy: i16,
) -> Result<(), String> {
    if dx == 0 && dy == 0 {
        return Ok(());
    }

    let (server_arc, client_name) = {
        let state_guard = state.lock().await;
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

#[tauri::command]
async fn relay_key_event(
    state: tauri::State<'_, Arc<Mutex<BarrierState>>>,
    key_code: u16,
    pressed: bool,
) -> Result<(), String> {
    let (server_arc, client_name) = {
        let state_guard = state.lock().await;
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

#[tauri::command]
async fn relay_mouse_button(
    state: tauri::State<'_, Arc<Mutex<BarrierState>>>,
    button: u8,
    pressed: bool,
) -> Result<(), String> {
    let (server_arc, client_name) = {
        let state_guard = state.lock().await;
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

#[tauri::command]
async fn get_status(state: tauri::State<'_, Arc<Mutex<BarrierState>>>) -> Result<AppState, String> {
    let (server_opt, client_opt, server_address, log_messages) = {
        let state_guard = state.lock().await;
        (
            state_guard.server.clone(),
            state_guard.client.clone(),
            state_guard.server_address.clone(),
            state_guard.log_messages.clone(),
        )
    };

    let connected_clients = if let Some(server) = &server_opt {
        server.lock().await.client_count().await as u32
    } else {
        0
    };

    let status = if server_opt.is_some() {
        AppState {
            mode: "server".to_string(),
            is_running: true,
            connected_clients,
            server_address,
            log_messages,
        }
    } else if client_opt.is_some() {
        AppState {
            mode: "client".to_string(),
            is_running: true,
            connected_clients: 0,
            server_address,
            log_messages,
        }
    } else {
        AppState::default()
    };
    
    Ok(status)
}

fn append_log(state: &mut BarrierState, message: String) {
    state.log_messages.push(message);
    if state.log_messages.len() > 200 {
        let overflow = state.log_messages.len() - 200;
        state.log_messages.drain(0..overflow);
    }
}

fn parse_port_from_server_config(config: &ServerConfig) -> u16 {
    if let Some(addr) = &config.listen_address {
        if let Some(port_str) = addr.split(':').last() {
            if let Ok(port) = port_str.parse::<u16>() {
                return port;
            }
        }
    }
    config.port
}

fn detect_runtime_capabilities() -> RuntimeCapabilities {
    let session_type = env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "unknown".to_string());
    let has_x11_display = env::var("DISPLAY").is_ok();
    let has_wayland_display = env::var("WAYLAND_DISPLAY").is_ok();

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
        recommendations.push("检测到 Wayland 环境：全局键鼠捕获/注入能力受 compositor 安全策略限制。".to_string());
        recommendations.push("如需完整 Barrier 行为，建议切换到 X11 会话，或后续接入 portal/libei。".to_string());
    } else {
        input_backend = "none".to_string();
        blocking_issues.push("未检测到 DISPLAY/WAYLAND_DISPLAY，GUI 输入能力不可用。".to_string());
        recommendations.push("请在桌面图形会话中运行，或检查远程会话环境变量。".to_string());
    }

    let global_input_available = cfg!(target_os = "linux") && has_x11_display;

    RuntimeCapabilities {
        session_type,
        has_x11_display,
        has_wayland_display,
        input_backend,
        global_input_available,
        recommendations,
        blocking_issues,
    }
}

fn main() {
    std::panic::set_hook(Box::new(|panic_info| {
        let backtrace = std::backtrace::Backtrace::force_capture();
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/barrier-tauri-crash.log")
        {
            let _ = writeln!(file, "panic: {}", panic_info);
            let _ = writeln!(file, "backtrace:\n{}", backtrace);
            let _ = writeln!(file, "----------------------------------------");
        }
    }));

    env_logger::init();
    
    tauri::Builder::default()
        .manage(Arc::new(Mutex::new(BarrierState {
            server: None,
            client: None,
            server_task: None,
            client_task: None,
            mode: None,
            server_address: "localhost:24800".to_string(),
            last_error: None,
            log_messages: vec!["应用启动".to_string()],
            active_client: None,
        })))
        .setup(|app| {
            let state = app.state::<Arc<Mutex<BarrierState>>>().inner().clone();
            start_global_input_pipeline(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            get_runtime_capabilities,
            start_server,
            start_client,
            switch_client,
            switch_back_local,
            relay_mouse_move,
            relay_key_event,
            relay_mouse_button,
            stop_service,
            get_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
