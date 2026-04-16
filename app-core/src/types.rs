use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppState {
    pub mode: String,
    pub is_running: bool,
    pub connected_clients: u32,
    pub server_address: String,
    pub active_client: Option<String>,
    pub log_messages: Vec<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            mode: "server".to_string(),
            is_running: false,
            connected_clients: 0,
            server_address: String::new(),
            active_client: None,
            log_messages: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeCapabilities {
    pub session_type: String,
    pub has_x11_display: bool,
    pub has_wayland_display: bool,
    pub input_backend: String,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScreenEdge {
    Left,
    Right,
    Top,
    Bottom,
}

impl Default for ScreenEdge {
    fn default() -> Self {
        Self::Right
    }
}

impl ScreenEdge {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
            Self::Top => "top",
            Self::Bottom => "bottom",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ClientRoute {
    pub id: String,
    pub name: String,
    pub address: String,
    #[serde(default)]
    pub edge: ScreenEdge,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub startup_restore: bool,
    pub last_mode: String,
    pub server_port: u16,
    pub server_screen_name: String,
    pub server_address: String,
    pub client_name: String,
    pub pointer_lock_enabled: bool,
    pub client_routes: Vec<ClientRoute>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            startup_restore: true,
            last_mode: "server".to_string(),
            server_port: 24800,
            server_screen_name: "server".to_string(),
            server_address: "localhost:24800".to_string(),
            client_name: "client-1".to_string(),
            pointer_lock_enabled: true,
            client_routes: vec![ClientRoute {
                id: "default-client-route".to_string(),
                name: "client-1".to_string(),
                address: "192.168.1.101:24800".to_string(),
                edge: ScreenEdge::Right,
            }],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", content = "payload", rename_all = "snake_case")]
pub enum CoreEvent {
    Status(AppState),
    Log(String),
    Error(String),
    Settings(AppSettings),
}
