use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EventType {
    SystemStatus,
    AgentMessage,
    ToolExecution,
    KernelCommand,
    UserAction,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Event {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub event_type: EventType,
    pub payload: serde_json::Value,
}

impl Event {
    pub fn new(event_type: EventType, payload: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            event_type,
            payload,
        }
    }
}

pub struct EventBus {
    events: Arc<RwLock<VecDeque<Event>>>,
    capacity: usize,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        Self {
            events: Arc::new(RwLock::new(VecDeque::with_capacity(capacity))),
            capacity,
        }
    }

    /// Ingests a new event into the bus.
    /// If the bus exceeds capacity, the oldest event is evicted (FIFO).
    pub async fn ingest(&self, event: Event) {
        let mut events = self.events.write().await;
        if events.len() >= self.capacity {
            events.pop_front();
        }
        events.push_back(event);
    }

    /// Returns the most recent event.
    pub async fn latest(&self) -> Option<Event> {
        let events = self.events.read().await;
        events.back().cloned()
    }

    /// Query the bus for events matching a specific type.
    pub async fn query(&self, event_type: EventType) -> Vec<Event> {
        let events = self.events.read().await;
        events
            .iter()
            .filter(|e| e.event_type == event_type)
            .cloned()
            .collect()
    }

    /// Returns the current number of events in the bus.
    pub async fn len(&self) -> usize {
        self.events.read().await.len()
    }

    /// Generates a 'chord' (snapshot) of the current state.
    /// For every unique channel (identified by a key in the payload),
    /// it returns the most recent event.
    ///
    /// The payload of the events is expected to have a "channel" field.
    pub async fn chord(&self) -> Vec<Event> {
        let events = self.events.read().await;
        let mut channel_map: std::collections::HashMap<String, Event> = std::collections::HashMap::new();

        for event in events.iter() {
            // Check if the event has a 'channel' key in its payload
            if let Some(channel) = event.payload.get("channel").and_then(|c| c.as_str()) {
                // If we haven't seen this channel, or if this event is newer (though VecDeque is ordered by time),
                // we update the map. Since we iterate from oldest to newest, the last one seen will be the latest.
                channel_map.insert(channel.to_string(), event.clone());
            }
        }

        channel_map.into_values().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_fifo_and_capacity() {
        let bus = EventBus::new(3);
        
        // Ingest 4 events
        for i in 0..4 {
            bus.ingest(Event::new(EventType::SystemStatus, json!({"count": i}))).await;
        }

        assert_eq!(bus.len().await, 3);
        
        // Check if the oldest (0) was evicted and we have (1, 2, 3)
        let latest = bus.latest().await.unwrap();
        assert_eq!(latest.payload["count"], 3);
        
        let all: Vec<Event> = bus.query(EventType::SystemStatus).await;
        assert_eq!(all[0].payload["count"], 1);
        assert_eq!(all[2].payload["count"], 3);
    }

    #[tokio::test]
    async fn test_query_by_type() {
        let bus = EventBus::new(10);
        
        bus.ingest(Event::new(EventType::AgentMessage, json!({"msg": "hi"}))).await;
        bus.ingest(Event::new(EventType::SystemStatus, json!({"status": "ok"}))).await;
        bus.ingest(Event::new(EventType::AgentMessage, json!({"msg": "bye"}))).await;

        let agent_events = bus.query(EventType::AgentMessage).await;
        assert_eq!(agent_events.len(), 2);
        assert_eq!(agent_events[0].payload["msg"], "hi");
        assert_eq!(agent_events[1].payload["msg"], "bye");

        let system_events = bus.query(EventType::SystemStatus).await;
        assert_eq!(system_events.len(), 1);
    }

    #[tokio::test]
    async fn test_latest() {
        let bus = EventBus::new(5);
        bus.ingest(Event::new(EventType::UserAction, json!({"act": "1"}))).await;
        bus.ingest(Event::new(EventType::UserAction, json!({"act": "2"}))).await;

        let latest = bus.latest().await.unwrap();
        assert_eq!(latest.payload["act"], "2");
    }
}
