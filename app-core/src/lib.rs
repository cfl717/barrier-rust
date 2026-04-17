mod core;
#[cfg(target_os = "linux")]
mod global_input_linux;
#[cfg(target_os = "linux")]
mod pointer_lock_linux;
mod settings;
mod types;

pub use barrier_protocol::{ClientConfig, ServerConfig};
pub use core::BarrierCore;
pub use settings::SettingsStore;
pub use types::{
    AppSettings, AppState, ClientRoute, CoreEvent, RuntimeCapabilities, ScreenEdge,
    SwitchClientRequest,
};
