//! Network API
//!
//! Provides network services for applications.

use super::types::{KernelError, KernelResult};

/// Network service for sending and receiving packets
pub struct NetworkService {
    initialized: bool,
}

impl NetworkService {
    /// Create a new network service
    pub fn new() -> KernelResult<Self> {
        Ok(Self { initialized: true })
    }

    /// Poll the network stack (call once per frame)
    pub fn poll(&mut self, timestamp: i64) {
        crate::net::stack::poll(timestamp);
    }

    /// Process incoming packets
    pub fn process_incoming(&mut self) {
        crate::net::protocol::process_incoming();
    }

    /// Broadcast world state to all connected clients
    pub fn broadcast_world_state(&mut self) {
        crate::net::protocol::broadcast_world_state();
    }

    /// Check if network is available
    pub fn is_available(&self) -> bool {
        crate::net::stack::is_initialized()
    }

    /// Get local IP address
    pub fn local_ip(&self) -> Option<[u8; 4]> {
        crate::net::stack::local_ip()
    }
}

impl Default for NetworkService {
    fn default() -> Self {
        Self { initialized: false }
    }
}

/// Network connection mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkMode {
    /// Offline single-player mode
    Offline,
    /// Server mode - host a game
    Server { port: u16 },
    /// Client mode - connect to a server
    Client { server_ip: [u8; 4], port: u16 },
}

impl Default for NetworkMode {
    fn default() -> Self {
        Self::Offline
    }
}
