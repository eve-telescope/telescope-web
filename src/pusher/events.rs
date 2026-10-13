//! The broadcast events (app/Events): named by their PHP classes, which
//! laravel-echo's `listen('IntelEntryCreated')` expects, with the event's
//! public properties as a JSON string in `data`. Published right after
//! the change is stored, the sender's own sockets included.

use serde_json::json;

use super::Hub;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    IntelEntryCreated { network_id: i64, entry_id: i64 },
    IntelEntryUpdated { network_id: i64, entry_id: i64 },
    IntelEntryDeleted { network_id: i64, entry_id: i64 },
    ScanShared { network_id: i64, scan_id: i64 },
}

/// The private channel of a network.
pub fn channel(network_id: i64) -> String {
    format!("private-intel-network.{network_id}")
}

impl Event {
    fn network_id(&self) -> i64 {
        match *self {
            Event::IntelEntryCreated { network_id, .. }
            | Event::IntelEntryUpdated { network_id, .. }
            | Event::IntelEntryDeleted { network_id, .. }
            | Event::ScanShared { network_id, .. } => network_id,
        }
    }

    /// The event's name and its data, as Laravel's broadcaster sent them.
    fn name_and_data(&self) -> (&'static str, String) {
        match *self {
            Event::IntelEntryCreated {
                network_id,
                entry_id,
            } => (
                "App\\Events\\IntelEntryCreated",
                json!({ "network_id": network_id, "entry_id": entry_id }).to_string(),
            ),
            Event::IntelEntryUpdated {
                network_id,
                entry_id,
            } => (
                "App\\Events\\IntelEntryUpdated",
                json!({ "network_id": network_id, "entry_id": entry_id }).to_string(),
            ),
            Event::IntelEntryDeleted {
                network_id,
                entry_id,
            } => (
                "App\\Events\\IntelEntryDeleted",
                json!({ "network_id": network_id, "entry_id": entry_id }).to_string(),
            ),
            Event::ScanShared {
                network_id,
                scan_id,
            } => (
                "App\\Events\\ScanShared",
                json!({ "network_id": network_id, "scan_id": scan_id }).to_string(),
            ),
        }
    }

    /// The frame a subscriber receives.
    pub fn frame(&self) -> String {
        let (name, data) = self.name_and_data();
        json!({ "event": name, "data": data, "channel": channel(self.network_id()) }).to_string()
    }
}

impl Hub {
    pub fn broadcast(&self, event: Event) {
        self.publish(channel(event.network_id()), event.frame());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_like_reverb() {
        assert_eq!(
            Event::IntelEntryCreated {
                network_id: 1,
                entry_id: 7
            }
            .frame(),
            r#"{"event":"App\\Events\\IntelEntryCreated","data":"{\"network_id\":1,\"entry_id\":7}","channel":"private-intel-network.1"}"#
        );
        assert_eq!(
            Event::ScanShared {
                network_id: 2,
                scan_id: 28
            }
            .frame(),
            r#"{"event":"App\\Events\\ScanShared","data":"{\"network_id\":2,\"scan_id\":28}","channel":"private-intel-network.2"}"#
        );
    }
}
