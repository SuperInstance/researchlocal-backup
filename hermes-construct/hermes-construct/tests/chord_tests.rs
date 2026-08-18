use hermes_construct::bus::{Event, EventBus, EventType};
use serde_json::json;

#[tokio::test]
async fn test_chord_snapshot() {
    let bus = EventBus::new(10);

    // Channel A: event 1
    bus.ingest(Event::new(EventType::SystemStatus, json!({"channel": "A", "val": 1}))).await;
    // Channel A: event 2 (should overwrite event 1 in chord)
    bus.ingest(Event::new(EventType::SystemStatus, json!({"channel": "A", "val": 2}))).await;
    
    // Channel B: event 1
    bus.ingest(Event::new(EventType::SystemStatus, json!({"channel": "B", "val": 10}))).await;
    
    // Channel C: no channel key (should be ignored by chord)
    bus.ingest(Event::new(EventType::SystemStatus, json!({"val": 99}))).await;

    let chord = bus.chord().await;

    // Chord should have 2 events: the latest for A and the latest for B
    assert_eq!(chord.len(), 2);

    let mut a_val = 0;
    let mut b_val = 0;

    for event in chord {
        if event.payload["channel"] == "A" {
            a_val = event.payload["val"].as_i64().unwrap();
        } else if event.payload["channel"] == "B" {
            b_val = event.payload["val"].as_i64().unwrap();
        }
    }

    assert_eq!(a_val, 2);
    assert_eq!(b_val, 10);
}
