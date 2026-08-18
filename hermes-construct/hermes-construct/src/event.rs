use serde::{Deserialize, Serialize};
use crate::clock::FabricTimestamp;
use std::collections::HashMap;
use uuid::Uuid;

/// Represents a discrete event within the system.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Event {
    pub id: Uuid,
    pub timestamp: FabricTimestamp,
    pub source: String,
    pub kind: EventKind,
    pub payload: EventPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EventKind {
    System,
    Agent,
    Tile,
    Room,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EventPayload {
    None,
    Message(String),
    StateChange {
        key: String,
        old: String,
        new: String,
    },
    Error(String),
}

/// An EventBus responsible for routing events to interested subscribers.
pub struct EventBus {
    subscribers: HashMap<EventKind, Vec<Box<dyn Fn(&Event) + Send + Sync>>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            subscribers: HashMap::new(),
        }
    }

    /// Registers a callback for a specific event kind.
    pub fn subscribe<F>(&mut self, kind: EventKind, callback: F)
    where
        F: Fn(&Event) + Send + Sync + 'static,
    {
        self.subscribers
            .entry(kind)
            .or_insert_with(Vec::new)
            .push(Box::new(callback));
    }

    /// Emits an event to all subscribers of the matching kinds.
    pub fn emit(&self, event: Event) {
        if let Some(callbacks) = self.subscribers.get(&event.kind) {
            for callback in callbacks {
                callback(&event);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_event_creation() {
        let event = Event {
            id: Uuid::new_v4(),
            timestamp: FabricTimestamp::now(),
            source: "test_source".to_string(),
            kind: EventKind::System,
            payload: EventPayload::None,
        };
        assert_eq!(event.source, "test_source");
    }

    #[test]
    fn test_event_bus_subscription() {
        let mut bus = EventBus::new();
        let received = Arc::new(Mutex::new(false));
        let received_clone = Arc::clone(&received);

        bus.subscribe(EventKind::System, move |e| {
            let mut r = received_clone.lock().unwrap();
            *r = true;
            println!("Received event: {:?}", e.id);
        });

        let event = Event {
            id: Uuid::new_v4(),
            timestamp: FabricTimestamp::now(),
            source: "test".to_string(),
            kind: EventKind::System,
            payload: EventPayload::None,
        };

        bus.emit(event);
        assert!(*received.lock().unwrap());
    }
}
