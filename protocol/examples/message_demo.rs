//! Barrier Protocol Message Serialization Example
//! 
//! This example demonstrates message serialization and deserialization.

use barrier_protocol::{Message, MessageType, message_types};
use log::info;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    
    info!("Barrier Protocol Message Example");
    info!("================================");
    
    // Create a hello message
    let hello_msg = Message::from_string(message_types::HELLO_SERVER, "Barrier 2.0\nclient-pc");
    info!("Created HELLO_SERVER message: {}", hello_msg.msg_type);
    
    // Serialize the message
    let bytes = hello_msg.serialize()?;
    info!("Serialized to {} bytes: {:02X?}", bytes.len(), bytes);
    
    // Deserialize the message
    let parsed_msg = Message::from_bytes(&bytes)?;
    info!("Deserialized message: {}", parsed_msg.msg_type);
    info!("Message content: {}", parsed_msg.as_string()?);
    
    // Verify round-trip
    assert_eq!(hello_msg.msg_type, parsed_msg.msg_type);
    assert_eq!(hello_msg.as_string()?, parsed_msg.as_string()?);
    info!("✓ Round-trip verification successful!");
    
    // Create a mouse move event
    use barrier_protocol::MouseMoveEvent;
    let mouse_event = barrier_protocol::MouseMoveEvent::new(
        10,
        -5,
        barrier_protocol::ModifierKeys::CONTROL,
    );
    let mouse_msg = mouse_event.to_message();
    info!("\nCreated MOUSE_MOVE message: {}", mouse_msg.msg_type);
    
    let mouse_bytes = mouse_msg.serialize()?;
    info!("Serialized to {} bytes", mouse_bytes.len());
    
    let parsed_mouse = Message::from_bytes(&mouse_bytes)?;
    let parsed_mouse_event = MouseMoveEvent::from_message(&parsed_mouse)?;
    info!(
        "Parsed mouse event: dx={}, dy={}, modifiers={:04X}",
        parsed_mouse_event.dx,
        parsed_mouse_event.dy,
        parsed_mouse_event.modifiers.0
    );
    
    // Create clipboard data
    use barrier_protocol::{ClipboardData, ClipboardFormat, create_data_message, parse_data_message};
    let clipboard = ClipboardData::text("Hello from Rust!", 1);
    let clip_msg = create_data_message(&clipboard);
    info!("\nCreated CLIPBOARD message: {}", clip_msg.msg_type);
    
    let clip_bytes = clip_msg.serialize()?;
    info!("Serialized to {} bytes", clip_bytes.len());
    
    let parsed_clip = Message::from_bytes(&clip_bytes)?;
    let parsed_clipboard = parse_data_message(&parsed_clip)?;
    info!(
        "Parsed clipboard: format={:?}, sequence={}, content=\"{}\"",
        parsed_clipboard.format,
        parsed_clipboard.sequence_id,
        parsed_clipboard.as_text()?
    );
    
    info!("\n✓ All examples completed successfully!");
    
    Ok(())
}
