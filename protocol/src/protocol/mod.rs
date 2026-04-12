//! Barrier Protocol Module
//! 
//! This module defines the core protocol structures and message types
//! used in Barrier communication.

pub mod message;
pub mod handshake;
pub mod clipboard;
pub mod events;

pub use message::*;
pub use handshake::*;
pub use clipboard::*;
pub use events::*;
