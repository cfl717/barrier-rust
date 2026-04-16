use app_core::{AppSettings, BarrierCore, SettingsStore};
use barrier_protocol::ServerConfig;
use std::net::TcpListener;
use tempfile::tempdir;

fn pick_free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind random port");
    let port = listener.local_addr().expect("local addr").port();
    drop(listener);
    port
}

#[tokio::test]
async fn server_start_stop_updates_status() {
    let dir = tempdir().expect("tempdir");
    let store = SettingsStore::new(dir.path().join("settings.toml"));
    let core = BarrierCore::with_settings_store(store);

    let port = pick_free_port();
    core.start_server(ServerConfig {
        screen_name: "server".to_string(),
        port,
        max_clients: 10,
        listen_address: Some(format!("0.0.0.0:{port}")),
        enable_clipboard: true,
        enable_drag_drop: true,
    })
    .await
    .expect("start server");

    let running = core.get_status().await.expect("status after start");
    assert!(running.is_running);
    assert_eq!(running.mode, "server");

    core.stop_service().await.expect("stop service");
    let stopped = core.get_status().await.expect("status after stop");
    assert!(!stopped.is_running);
}

#[tokio::test]
async fn restore_on_startup_obeys_settings() {
    let dir = tempdir().expect("tempdir");
    let store = SettingsStore::new(dir.path().join("settings.toml"));
    let port = pick_free_port();
    store
        .save(&AppSettings {
            startup_restore: true,
            last_mode: "server".to_string(),
            server_port: port,
            server_screen_name: "restore-server".to_string(),
            server_address: format!("localhost:{port}"),
            client_name: "client-1".to_string(),
            pointer_lock_enabled: true,
            client_routes: vec![],
        })
        .expect("save settings");

    let core = BarrierCore::with_settings_store(store);
    core.restore_on_startup().await.expect("restore startup");
    let status = core.get_status().await.expect("status");
    assert!(status.is_running);
    assert_eq!(status.mode, "server");
    core.stop_service().await.expect("stop service");
}
