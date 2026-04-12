//! Barrier Protocol Client Example
//! 
//! This example demonstrates how to run a Barrier client.

use barrier_protocol::{BarrierClient, ClientConfig, ClientState};
use log::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    
    info!("Starting Barrier Client Example");
    
    // Create client configuration
    let config = ClientConfig {
        server_addr: "localhost:24800".to_string(),
        screen_name: "client-pc".to_string(),
        auto_reconnect: true,
        reconnect_interval: 5,
    };
    
    // Create client instance
    let mut client = BarrierClient::new(config);
    
    info!("Client created, connecting to server...");
    
    // Run client with auto-reconnect
    client.run().await?;
    
    Ok(())
}
