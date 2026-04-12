//! Barrier Protocol Server Example
//! 
//! This example demonstrates how to run a Barrier server.

use barrier_protocol::{BarrierServer, ServerConfig, ServerEvent};
use log::info;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    
    info!("Starting Barrier Server Example");
    
    // Create server configuration
    let config = ServerConfig {
        screen_name: "server".to_string(),
        port: 24800,
        max_clients: 10,
    };
    
    // Create server instance
    let (mut server, mut event_rx) = BarrierServer::new(config);
    
    info!("Server created, starting listener...");
    
    // Spawn event handler
    let event_handle = tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            match event {
                ServerEvent::ClientConnected(client) => {
                    info!("✓ Client connected: {} ({})", client.screen_name, client.address);
                }
                ServerEvent::ClientDisconnected(id) => {
                    info!("✗ Client disconnected: {}", id);
                }
                ServerEvent::HandshakeCompleted { client_id, result } => {
                    info!(
                        "🤝 Handshake completed with client {}: {} {}",
                        client_id, result.app_name, result.version
                    );
                }
                ServerEvent::MessageReceived { client_id, message } => {
                    info!("📨 Message from client {}: {}", client_id, message.msg_type);
                }
                ServerEvent::Error(err) => {
                    log::error!("❌ Server error: {}", err);
                }
            }
        }
    });
    
    // Start server (this will block)
    server.run().await?;
    
    event_handle.await?;
    
    Ok(())
}
