//! The realtime channel, the Pusher protocol 7 on ws.eve-telescope.com:
//! the websocket at `/app/<key>` (`socket`), the private channel
//! signatures of `/broadcasting/auth` (`auth`) and the events the API
//! publishes (`events`). Every desktop client since v0.2.1 connects with
//! the same key and expects these frames byte for byte
//! (tests/integration/pusher_protocol.rs).
//!
//! Delivery: one broadcast bus per process; each connection forwards the
//! frames of the channels it subscribed to. The api runs as one
//! container, so no cross-process fan-out is needed.

pub mod auth;
pub mod events;
pub mod socket;

use std::sync::Arc;

use tokio::sync::broadcast;

/// Frames waiting for a slow connection before it misses some.
const BUS_CAPACITY: usize = 1024;

/// One frame for every subscriber of a channel.
#[derive(Debug, Clone)]
pub struct Published {
    pub channel: String,
    pub frame: String,
}

#[derive(Clone)]
pub struct Hub {
    bus: broadcast::Sender<Arc<Published>>,
}

impl Default for Hub {
    fn default() -> Self {
        Self::new()
    }
}

impl Hub {
    pub fn new() -> Self {
        let (bus, _) = broadcast::channel(BUS_CAPACITY);
        Self { bus }
    }

    /// Sends a frame to the channel's subscribers; nobody listening is
    /// fine.
    pub fn publish(&self, channel: String, frame: String) {
        let _ = self.bus.send(Arc::new(Published { channel, frame }));
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Arc<Published>> {
        self.bus.subscribe()
    }
}
