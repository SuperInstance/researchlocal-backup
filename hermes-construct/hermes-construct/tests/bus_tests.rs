use hermes_construct::bus::{Event, EventBus, EventType};
use serde_json::json;

#[tokio::test]
async fn test_fifo_and_capacity() {
    let bus = EventBus::new(3);
    
    // Ingest 4 events
    for i in 0..4 {
        bus.ingest(Event::new(EventType::SystemStatus, json!({ "count": i }))).await;
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
    
    bus.ingest(Event::new(EventType::AgentMessage, json!({ "msg": "hi" }))).await;
    bus.ingest(Event::new(EventType::SystemStatus, json!({ "status": "ok" }))).await;
    bus.ingest(Event::new(EventType::AgentMessage, json!({ "msg": "bye" }))).await;

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
    bus.ingest(Event::new(EventType::UserAction, json!({ "act": "1" }))).await;
    bus.ingest(Event::new(EventType::UserAction, json!({ "act": "2" }))).await;

    let latest = bus.latest().await.unwrap();
    assert_eq!(latest.payload["act"], "2");
}
