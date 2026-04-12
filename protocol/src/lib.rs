//! Barrier Protocol Implementation in Rust
//! 
//! This crate implements the Barrier network protocol for sharing
//! mouse and keyboard between multiple computers.
//! 
//! # Architecture
//! 
//! ```text
//! ┌─────────────────┐     TCP/IP      ┌─────────────────┐
//! │     Server      │ ◄─────────────► │     Client      │
//! │  (Primary PC)   │                 │ (Secondary PC)  │
//! └────────┬────────┘                 └────────┬────────┘
//!          │                                   │
//!   ┌──────▼──────┐                     ┌──────▼──────┐
//!   │  Protocol   │                     │  Protocol   │
//!   │   Layer     │                     │   Layer     │
//!   └──────┬──────┘                     └──────┬──────┘
//!          │                                   │
//!   ┌──────▼──────┐                     ┌──────▼──────┐
//!   │   Input     │                     │   Input     │
//!   │  Capture    │                     │  Injection  │
//!   └─────────────┘                     └─────────────┘
//! ```
//! 
//! # Protocol Overview
//! 
//! Barrier uses a custom binary protocol over TCP port 24800.
//! All multi-byte integers are transmitted in big-endian format.
//! 
//! ## Message Format
//! 
//! ```text
//! ┌─────────────┬─────────────┬─────────────┬─────────────┐
//! │  Size (4B)  │ Type (4B)   │  Data...    │  Checksum   │
//! └─────────────┴─────────────┴─────────────┴─────────────┘
//! ```
//! 
//! ## Key Message Types
//! 
//! - `CBYQ` / `QBYC`: Hello handshake (server/client)
//! - `CLAP`: Clipboard data
//! - `CROP`: Screen shape information
//! - `CINN`: Client enter screen
//! - `COUT`: Client leave screen
//! - `CMOV`: Mouse move
//! - `CBUT`: Mouse button
//! - `CKEY`: Keyboard key
//! - `CSOP`: Screen options

pub mod protocol;
pub mod network;
pub mod platform;

pub use protocol::*;
pub use network::*;
