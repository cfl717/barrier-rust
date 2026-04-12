// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::sync::Arc;
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

struct BarrierState {
    server: Option<Arc<Mutex<BarrierServer>>>,
    client: Option<Arc<Mutex<BarrierClient>>>,
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn start_server(state: tauri::State<'_, Arc<Mutex<BarrierState>>>, config: ServerConfig) -> Result<(), String> {
    let mut state_guard = state.lock().await;
    
    match BarrierServer::new(config).await {
        Ok(server) => {
            let server_arc = Arc::new(Mutex::new(server));
            state_guard.server = Some(server_arc.clone());
            
            // Start server in background
            tokio::spawn(async move {
                let mut server = server_arc.lock().await;
                if let Err(e) = server.run().await {
                    log::error!("Server error: {}", e);
                }
            });
            
            Ok(())
        }
        Err(e) => Err(format!("Failed to start server: {}", e)),
    }
}

#[tauri::command]
async fn start_client(state: tauri::State<'_, Arc<Mutex<BarrierState>>>, config: ClientConfig) -> Result<(), String> {
    let mut state_guard = state.lock().await;
    
    match BarrierClient::new(config).await {
        Ok(client) => {
            let client_arc = Arc::new(Mutex::new(client));
            state_guard.client = Some(client_arc.clone());
            
            // Start client in background
            tokio::spawn(async move {
                let mut client = client_arc.lock().await;
                if let Err(e) = client.run().await {
                    log::error!("Client error: {}", e);
                }
            });
            
            Ok(())
        }
        Err(e) => Err(format!("Failed to start client: {}", e)),
    }
}

#[tauri::command]
async fn stop_service(state: tauri::State<'_, Arc<Mutex<BarrierState>>>) -> Result<(), String> {
    let mut state_guard = state.lock().await;
    
    state_guard.server = None;
    state_guard.client = None;
    
    Ok(())
}

#[tauri::command]
async fn get_status(state: tauri::State<'_, Arc<Mutex<BarrierState>>>) -> Result<AppState, String> {
    let state_guard = state.lock().await;
    
    let status = if state_guard.server.is_some() {
        AppState {
            mode: "server".to_string(),
            is_running: true,
            connected_clients: 0, // TODO: Get actual count
            server_address: "0.0.0.0:24800".to_string(),
            log_messages: vec!["Server is running".to_string()],
        }
    } else if state_guard.client.is_some() {
        AppState {
            mode: "client".to_string(),
            is_running: true,
            connected_clients: 0,
            server_address: "localhost:24800".to_string(),
            log_messages: vec!["Client is running".to_string()],
        }
    } else {
        AppState::default()
    };
    
    Ok(status)
}

fn main() {
    env_logger::init();
    
    tauri::Builder::default()
        .manage(Arc::new(Mutex::new(BarrierState {
            server: None,
            client: None,
        })))
        .invoke_handler(tauri::generate_handler![
            greet,
            start_server,
            start_client,
            stop_service,
            get_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
