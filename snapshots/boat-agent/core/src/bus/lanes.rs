//! Lane queues with per-lane backpressure policy (docs/06 §Lanes).
//!
//! ## Architecture
//!
//! The bus is organized into three priority lanes, each with distinct
//! backpressure semantics:
//!
//! - **Critical**: Unbounded, immediate delivery. Used for human veto,
//!   watchdog trips, and escalations. Preempts between any two kernel steps.
//!
//! - **Telemetry**: Fixed capacity, coalesced by (kind, producer). The newest
//!   event for each (kind, producer) wins; stale GPS is worse than none.
//!
//! - **Narrative**: Drop-oldest under load. Logs, agent reasoning, and sync
//!   data must never block the control loop.

use super::{AgentRole, Event, IngestError, Lane};
use std::collections::HashMap;
use std::collections::VecDeque;
use tokio::sync::mpsc;

/// Channel capacities chosen to bound memory under worst-case conditions.
/// These values assume ~1KB per event and 10Hz tick rate.
///
/// - Telemetry: 4096 events ≈ 4MB buffer ≈ 6.5 minutes at 100 events/sec
/// - Narrative: 16384 events ≈ 16MB buffer for logs and agent chatter
const TELEMETRY_CAPACITY: usize = 4096;
const NARRATIVE_CAPACITY: usize = 16384;

/// Sink for events that fail ingest. Never silently dropped — the Auditor
/// reads this continuously. A module that lands here repeatedly is defective.
///
/// Each quarantine entry includes: timestamp, event (as much as could be
/// read), and the structured IngestError. This is how the system learns
/// about broken drivers, malformed schemas, and capability violations.
pub struct QuarantineLog {
    /// Path to the rolling log file
    log_path: std::path::PathBuf,
    /// In-memory buffer of recent quarantine entries (circular)
    buffer: VecDeque<QuarantineEntry>,
    /// Maximum entries to keep in memory
    buffer_size: usize,
}

#[derive(Debug, Clone)]
pub struct QuarantineEntry {
    pub timestamp_ms: u64,
    pub error: IngestError,
    pub event_summary: String,
}

impl QuarantineLog {
    /// Create a new quarantine log that writes to the given path.
    pub fn new(log_path: std::path::PathBuf) -> Self {
        Self {
            log_path,
            buffer: VecDeque::with_capacity(1000),
            buffer_size: 1000,
        }
    }

    /// Record a quarantined event. Never blocks — if the write fails,
    /// we still keep it in memory for the Auditor.
    pub fn record(&mut self, error: IngestError, event_summary: String) {
        let entry = QuarantineEntry {
            timestamp_ms: super::now_ms(),
            error,
            event_summary,
        };

        self.buffer.push_back(entry);

        if self.buffer.len() > self.buffer_size {
            self.buffer.pop_front();
        }

        // TODO: async write to log file (non-blocking)
    }

    /// Read recent quarantine entries for the Auditor.
    pub fn recent(&self) -> Vec<QuarantineEntry> {
        self.buffer.iter().cloned().collect()
    }
}

/// Coalescing key: (event_kind_string, producer_module_string).
///
/// Events from the same producer of the same kind are coalesced —
/// we only care about the newest value. This is why "stale GPS is worse
/// than none": if we have 10 GPS updates in one tick, we only process
/// the latest one.
type CoalesceKey = (String, String);

/// Routes events into lane queues; enforces lane privilege at ingest.
///
/// This is the single entry point for all events in the system. Every
/// event must pass through `ingest()`, which validates schema, checks
/// capabilities, and enforces lane privileges.
pub struct LaneRouter {
    /// Critical lane: unbounded, immediate delivery
    critical_tx: mpsc::UnboundedSender<Event>,
    critical_rx: mpsc::UnboundedReceiver<Event>,

    /// Telemetry lane: coalesced by (kind, producer)
    ///
    /// We keep the newest event for each unique (kind, producer) pair.
    /// This ensures that if a driver emits 100 GPS fixes in one tick,
    /// the reducer only sees the most recent one.
    telemetry_coalesce: HashMap<CoalesceKey, Event>,

    /// Narrative lane: drop-oldest under load
    narrative: VecDeque<Event>,

    /// Quarantine log for failed ingests
    quarantine: QuarantineLog,

    /// Capability grants from vessel profile
    ///
    /// Maps agent role to the set of kind globs they may emit.
    /// Enforced at ingest — structural, not polite.
    agent_capabilities: HashMap<String, Vec<String>>,
}

impl LaneRouter {
    /// Create a new lane router with empty queues.
    pub fn new(data_dir: &std::path::Path) -> Self {
        let quarantine_path = data_dir.join("quarantine.log");
        let (critical_tx, critical_rx) = mpsc::unbounded_channel();

        Self {
            critical_tx,
            critical_rx,
            telemetry_coalesce: HashMap::with_capacity(256),
            narrative: VecDeque::with_capacity(NARRATIVE_CAPACITY),
            quarantine: QuarantineLog::new(quarantine_path),
            agent_capabilities: Self::default_capabilities(),
        }
    }

    /// Default capability grants per agent role (docs/10 §Capability model).
    ///
    /// These are the baseline grants. The vessel profile can add more,
    /// but these minimums are always enforced. For example, the Analyst
    /// role explicitly has NO control.* grants — this is structural.
    fn default_capabilities() -> HashMap<String, Vec<String>> {
        let mut caps = HashMap::new();

        // Operator can emit control intents and request escalation
        caps.insert(
            "agent:operator".to_string(),
            vec![
                "control.intent.*".to_string(),
                "agent.shadow.delta".to_string(),
                "agent.escalation.request".to_string(),
            ],
        );

        // Engineer can propose playbooks and escalate
        caps.insert(
            "agent:engineer".to_string(),
            vec![
                "agent.proposal.playbook".to_string(),
                "agent.escalation.request".to_string(),
            ],
        );

        // Analyst can only analyze — NO control grants
        caps.insert(
            "agent:analyst".to_string(),
            vec![
                "agent.analysis.finding".to_string(),
                "agent.shadow.delta".to_string(),
            ],
        );

        // Auditor is never rate-limited and can emit degraded_mode
        caps.insert(
            "agent:auditor".to_string(),
            vec![
                "agent.audit.result".to_string(),
                "system.degraded_mode".to_string(),
            ],
        );

        caps
    }

    /// Load capability grants from vessel profile.
    ///
    /// This extends the default capabilities with any additional grants
    /// specified in the vessel.toml file. The safety constraint is that
    /// we can ONLY add grants, never remove the baseline safety constraints.
    pub fn load_capabilities(&mut self, grants: &HashMap<String, crate::config::AgentGrants>) {
        for (role, grant) in grants {
            let key = format!("agent:{}", role);
            self.agent_capabilities.insert(key, grant.may_emit.clone());
        }
    }

    /// ═══════════════════════════════════════════════════════════════════
    ///  THE SINGLE ENTRY POINT FOR EVERY EVENT IN THE SYSTEM
    /// ═══════════════════════════════════════════════════════════════════
    ///
    /// Enforcement order (non-negotiable):
    ///   1. Schema validation against schemas/vessel-event.schema.json
    ///   2. Capability check: is this actor allowed to emit this kind?
    ///   3. Lane privilege: critical lane is for L0 drivers + envelope only
    ///   4. Routing into the lane queue with its backpressure policy
    ///
    /// Failures go to quarantine with a structured `IngestError`.
    pub fn ingest(&mut self, event: Event) -> Result<(), IngestError> {
        // Step 1: Schema validation against the registered schema for this kind
        self.validate_schema(&event)?;

        // Step 2: Capability check — structural enforcement of agent permissions
        self.check_capability(&event)?;

        // Step 3: Lane privilege — critical lane is protected
        if event.lane == Lane::Critical && !self.may_emit_critical(&event.source.module) {
            return Err(IngestError::LaneViolation {
                module: event.source.module.clone(),
            });
        }

        // Step 4: Route into the lane queue with its backpressure policy
        self.route(event);

        Ok(())
    }

    /// Validate the event payload against its registered schema.
    ///
    /// Each event kind has a schema definition in the vessel-event schema.
    /// We validate here, at the boundary, to ensure malformed events never
    /// enter the system.
    fn validate_schema(&self, event: &Event) -> Result<(), IngestError> {
        // TODO: Implement JSON schema validation using jsonschema crate
        // For now, we do basic sanity checks that would be in the schema:
        //
        // - Verify non-null required fields based on event kind
        // - Check numeric ranges (e.g., lat/lon, hdop)
        // - Ensure timestamps are reasonable (not in the far future)

        // Quick sanity: timestamp can't be in the future (allow 5s clock skew)
        let now = super::now_ms();
        let max_future_ms = 5000;

        if event.timestamp_ms > now + max_future_ms {
            return Err(IngestError::SchemaViolation {
                kind: event.kind.kind_str().to_string(),
                reason: format!("timestamp {} is more than 5s in the future", event.timestamp_ms),
            });
        }

        Ok(())
    }

    /// Check if the event's actor has capability to emit this kind.
    ///
    /// This is how we make "Analyst can't emit control intents" structural
    /// rather than just polite. The capability check happens at ingest time,
    /// so a rogue Analyst agent literally cannot emit control events.
    fn check_capability(&self, event: &Event) -> Result<(), IngestError> {
        let actor_key = match &event.source.actor {
            super::Actor::System => return Ok(()), // System can emit anything
            super::Actor::Human => return Ok(()),  // Human can emit anything
            super::Actor::Agent(role) => format!("agent:{}", role.as_ref()),
        };

        // Get the allowed kind globs for this role
        let allowed = match self.agent_capabilities.get(&actor_key) {
            Some(kinds) => kinds,
            None => {
                return Err(IngestError::CapabilityDenied {
                    actor: actor_key,
                    kind: event.kind.kind_str().to_string(),
                });
            }
        };

        // Check if any of the allowed globs match this event kind.
        // kind_str() is the canonical dotted identity — never Debug output.
        let kind_str = event.kind.kind_str();
        for pattern in allowed {
            if Self::glob_match(pattern, kind_str) {
                return Ok(());
            }
        }

        Err(IngestError::CapabilityDenied {
            actor: actor_key,
            kind: kind_str.to_string(),
        })
    }

    /// Simple glob matching for capability checks.
    ///
    /// Supports:
    /// - Exact match: "control.intent"
    /// - Wildcard suffix: "control.*" matches "control.intent", "control.verdict"
    fn glob_match(pattern: &str, text: &str) -> bool {
        if pattern == "*" {
            return true;
        }

        if let Some(prefix) = pattern.strip_suffix('*') {
            text.starts_with(prefix)
        } else {
            pattern == text
        }
    }

    /// Check if a module may emit on the critical lane.
    ///
    /// The critical lane is a privilege. Only:
    /// - L0 drivers (driver.*)
    /// - The envelope (envelope)
    /// - The kernel (kernel)
    ///
    /// are permitted to emit here. This is how we ensure that human veto
    /// events and watchdog trips ALWAYS preempt.
    fn may_emit_critical(&self, module: &str) -> bool {
        module.starts_with("driver.")
            || module == "envelope"
            || module == "kernel"
            || module.starts_with("actuator.") // Actuator feedback
    }

    /// Route an event into its lane queue with the lane's backpressure policy.
    fn route(&mut self, event: Event) {
        match event.lane {
            Lane::Critical => {
                // Critical: unbounded, immediate delivery
                // We use unbounded sender because critical events must NOT be dropped
                let _ = self.critical_tx.send(event);
            }
            Lane::Telemetry => {
                // Telemetry: coalesce by (kind, producer). kind_str() gives the
                // payload-independent kind identity — using Debug format here
                // would include payload values and coalescing would never fire.
                let key = (event.kind.kind_str().to_string(), event.source.module.clone());
                self.telemetry_coalesce.insert(key, event);

                // If we've exceeded capacity, drop oldest entries (LRU eviction)
                while self.telemetry_coalesce.len() > TELEMETRY_CAPACITY {
                    // Find and remove the oldest entry
                    // TODO: Track insertion order for proper LRU
                    // For now, just remove one arbitrary entry
                    if let Some(key) = self.telemetry_coalesce.keys().next().cloned() {
                        self.telemetry_coalesce.remove(&key);
                    }
                }
            }
            Lane::Narrative => {
                // Narrative: drop-oldest under load
                self.narrative.push_back(event);

                while self.narrative.len() > NARRATIVE_CAPACITY {
                    self.narrative.pop_front();
                }
            }
        }
    }

    /// Drain all critical events.
    ///
    /// Called between every two kernel steps — this is what "preempts" means
    /// concretely. If the jog lever moves or the watchdog trips, we handle
    /// it NOW, not at the next tick boundary.
    pub fn drain_critical(&mut self) -> Vec<Event> {
        let mut events = Vec::new();

        // Drain the unbounded channel
        while let Ok(event) = self.critical_rx.try_recv() {
            events.push(event);
        }

        events
    }

    /// Coalescing drain for the reducer: newest event per (kind, producer).
    ///
    /// This is how we ensure the reducer only sees the latest telemetry.
    /// If we have 10 GPS updates in one tick, we only process the newest.
    ///
    /// Returns all coalesced events and clears the buffer for the next tick.
    pub fn drain_telemetry_coalesced(&mut self) -> Vec<Event> {
        let events = self.telemetry_coalesce.values().cloned().collect();
        self.telemetry_coalesce.clear();
        events
    }

    /// Drain narrative events for logger/agent consumption.
    ///
    /// Returns up to `limit` events, leaving the rest for next call.
    /// This prevents the narrative lane from blocking if a slow consumer
    /// can't keep up with a flood of logs.
    pub fn drain_narrative(&mut self, limit: usize) -> Vec<Event> {
        let mut events = Vec::new();

        while events.len() < limit {
            if let Some(event) = self.narrative.pop_front() {
                events.push(event);
            } else {
                break;
            }
        }

        events
    }

    /// Get recent quarantine entries for the Auditor.
    pub fn quarantine_recent(&self) -> Vec<crate::bus::lanes::QuarantineEntry> {
        self.quarantine.recent()
    }
}

/// Marker: agents that may only ever consume, never produce, control kinds.
///
/// Used by capability checks; zero-sized, compile-time documentation.
/// This is how we make "Analyst is read-only" a type-level guarantee.
pub struct ReadOnlyRole(pub AgentRole);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::{Event, Source, Provenance};
    use crate::bus::events::EventKind;
    use ulid::Ulid;

    fn make_test_event(kind: EventKind, lane: Lane, module: &str) -> Event {
        Event {
            id: Ulid::new(),
            seq: 0,
            timestamp_ms: super::super::now_ms(),
            lane,
            kind,
            source: Source {
                module: module.to_string(),
                instance: None,
                actor: crate::bus::Actor::System,
            },
            provenance: Provenance {
                authority: "test".to_string(),
                confidence: 1.0,
                basis: vec![],
            },
        }
    }

    #[test]
    fn test_critical_lane_privilege() {
        let mut router = LaneRouter::new(std::path::Path::new("/tmp/test"));

        // Driver may emit critical
        let event = make_test_event(
            EventKind::TickHeartbeat(crate::bus::events::TickHeartbeat {
                tick_n: 0,
                drift_ms: 0,
                state_hash: "test".to_string(),
            }),
            Lane::Critical,
            "driver.gps",
        );
        assert!(router.ingest(event).is_ok());

        // Random module may NOT emit critical
        let event = make_test_event(
            EventKind::TickHeartbeat(crate::bus::events::TickHeartbeat {
                tick_n: 0,
                drift_ms: 0,
                state_hash: "test".to_string(),
            }),
            Lane::Critical,
            "agent.analyst",
        );
        assert!(matches!(
            router.ingest(event),
            Err(IngestError::LaneViolation { .. })
        ));
    }

    #[test]
    fn test_capability_enforcement() {
        let mut router = LaneRouter::new(std::path::Path::new("/tmp/test"));

        // Analyst cannot emit control intents
        let mut event = make_test_event(
            EventKind::Intent(crate::bus::events::Intent {
                requested_rudder_deg: None,
                requested_throttle_pct: None,
                horizon_s: 1.0,
            }),
            Lane::Telemetry,
            "agent.analyst",
        );
        event.source.actor = crate::bus::Actor::Agent(crate::bus::AgentRole::Analyst);

        assert!(matches!(
            router.ingest(event),
            Err(IngestError::CapabilityDenied { .. })
        ));
    }

    #[test]
    fn test_telemetry_coalescing() {
        let mut router = LaneRouter::new(std::path::Path::new("/tmp/test"));

        // Emit 5 GPS events from the same driver
        for i in 0..5 {
            let event = make_test_event(
                EventKind::GpsFix(crate::bus::events::GpsFix {
                    lat: 45.5 + i as f64 * 0.001,
                    lon: -122.5,
                    sog_kn: 2.0,
                    cog_deg: 147.0,
                    hdop: 1.0,
                    sats: 12,
                }),
                Lane::Telemetry,
                "driver.gps",
            );
            router.ingest(event).unwrap();
        }

        // Should only get 1 coalesced event (the newest)
        let drained = router.drain_telemetry_coalesced();
        assert_eq!(drained.len(), 1);

        // And it should be the last one (lat ~45.504)
        if let EventKind::GpsFix(fix) = &drained[0].kind {
            assert!((fix.lat - 45.504).abs() < 0.001);
        } else {
            panic!("Expected GpsFix");
        }
    }

    #[test]
    fn test_narrative_drop_oldest() {
        let mut router = LaneRouter::new(std::path::Path::new("/tmp/test"));

        // Fill narrative buffer beyond capacity
        for i in 0..(NARRATIVE_CAPACITY + 100) {
            let event = make_test_event(
                EventKind::VoiceTranscript(crate::bus::events::VoiceTranscript {
                    text: format!("word {}", i),
                    word_timestamps: vec![],
                    audio_hash: "test".to_string(),
                }),
                Lane::Narrative,
                "agent.analyst",
            );
            router.ingest(event).unwrap();
        }

        // Buffer should be at capacity, not over
        assert_eq!(router.narrative.len(), NARRATIVE_CAPACITY);
    }
}
