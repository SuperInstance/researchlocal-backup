//! VesselState: the one authoritative digital twin (docs/05 §Primitive 2).
//!
//! ## INVARIANTS (non-negotiable)
//!
//! 1. **Pure reducer**: `reduce(state, events) → state`. No I/O, no clocks
//!    other than event timestamps, no randomness. Replayable by axiom A2.
//!
//! 2. **Single source of truth**: This is the ONLY copy of vessel state.
//!    Playbooks, envelope, UI, and black box all read this same snapshot.
//!
//! 3. **Sensor fusion is an implementation detail**: The EKF/UKF choice
//!    (legacy RQ-004) lives INSIDE the reducer — invisible to the bus contract.
//!
//! ## Architecture
//!
//! The reducer pattern ensures that:
//! - Every state transition is traceable to specific events
//! - The system can be replayed from black-box logs
//! - No hidden state or side effects can corrupt the digital twin
//!
//! State updates are grouped by domain:
//! - Navigation: GPS, compass, depth, speed, course
//! - Propulsion: RPM, throttle, rudder position
//! - Environment: wind, sea state classification
//! - Human: autonomy dial, override status
//! - System: degraded mode, tick counter

use crate::bus::events::*;
use crate::bus::Event;
use crate::bus::Lane;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Quality metadata so consumers can reason about staleness instead of
/// trusting silently-bad numbers.
///
/// This is how the system degrades gracefully when sensors fail:
/// - `stale`: hasn't updated recently (don't use it)
/// - `degraded`: updating but with poor quality (use with caution)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SensorHealth {
    /// Last update timestamp (epoch ms)
    pub last_update_ms: u64,
    /// Data is too old to trust (threshold depends on sensor type)
    pub stale: bool,
    /// Sensor is functioning but quality is poor
    pub degraded: bool,
}

impl Default for SensorHealth {
    fn default() -> Self {
        Self {
            last_update_ms: 0,
            stale: true,     // Start stale until proven fresh
            degraded: false,
        }
    }
}

impl SensorHealth {
    /// Create fresh health from current timestamp. TEST/BOOT HELPER ONLY —
    /// never call this from the reducer (wall clock breaks replay purity).
    pub fn fresh_now() -> Self {
        Self {
            last_update_ms: now_ms(),
            stale: false,
            degraded: false,
        }
    }

    /// Update health from an EVENT timestamp and check staleness against
    /// the state's own clock. The reducer calls only this form — event
    /// timestamps are the only time source allowed inside reduce() (A2).
    pub fn update_at(&mut self, event_ts_ms: u64, state_now_ms: u64, age_threshold_ms: u64) {
        self.last_update_ms = event_ts_ms;
        self.stale = state_now_ms.saturating_sub(event_ts_ms) > age_threshold_ms;
    }

    /// Mark sensor as degraded (e.g., high HDOP, erratic compass).
    pub fn mark_degraded(&mut self) {
        self.degraded = true;
    }
}

/// Navigation state: position, motion, heading, depth.
///
/// This is the digital twin of what the navigation instruments report.
/// All fusion and filtering happens here — the bus only sees raw events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavigationState {
    /// Position from GPS
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    /// Speed over ground (knots)
    pub sog_kn: Option<f32>,
    /// Course over ground (degrees 0-360)
    pub cog_deg: Option<f32>,
    /// Magnetic compass heading (degrees 0-360)
    pub heading_deg: Option<f32>,
    /// Compass swing rate (degrees/second) — for damping checks
    pub swing_rate_dps: Option<f32>,
    /// Depth from sounder (meters)
    pub depth_m: Option<f32>,
    /// Bottom hardness (0.0 = mud, 1.0 = rock) from sounder
    pub bottom_hardness: Option<f32>,
    /// Depth trend for shoaling detection (meters per minute, negative = shoaling)
    ///
    /// Computed from depth history. Used by envelope for safety:
    /// "reject if depth_trend < -2.0 m/min" means we're heading toward shallow too fast.
    pub depth_trend_m_per_min: Option<f32>,

    /// Health metadata for each sensor source
    pub gps_health: SensorHealth,
    pub compass_health: SensorHealth,
    pub depth_health: SensorHealth,
}

impl Default for NavigationState {
    fn default() -> Self {
        Self {
            lat: None,
            lon: None,
            sog_kn: None,
            cog_deg: None,
            heading_deg: None,
            swing_rate_dps: None,
            depth_m: None,
            bottom_hardness: None,
            depth_trend_m_per_min: None,
            gps_health: SensorHealth::default(),
            compass_health: SensorHealth::default(),
            depth_health: SensorHealth::default(),
        }
    }
}

/// Propulsion state: engines, throttle, rudder.
///
/// Tracks what the propulsion system is doing, not what we want it to do.
/// (Intent vs. actual is the envelope's job to compare.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropulsionState {
    /// Engine RPM (from N2K PGN 127488 or analog tach)
    pub rpm: Option<u32>,
    /// Throttle position (percent 0-100) as reported by engine
    pub throttle_pct: Option<f32>,
    /// Rudder angle (degrees, negative = port, positive = starboard)
    pub rudder_angle_deg: Option<f32>,
    /// Engine health monitoring
    pub engine_health: SensorHealth,
}

impl Default for PropulsionState {
    fn default() -> Self {
        Self {
            rpm: None,
            throttle_pct: None,
            rudder_angle_deg: None,
            engine_health: SensorHealth::default(),
        }
    }
}

/// Environment state: wind, sea state.
///
/// Used by playbooks for wind-aware trolling and by envelope for safety.
/// Sea state classification enables gain scheduling (legacy RQ-003).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentState {
    /// Apparent wind speed (knots)
    pub wind_speed_kn: Option<f32>,
    /// Apparent wind angle (degrees, 0 = dead ahead)
    pub wind_angle_deg: Option<f32>,
    /// Classified sea state for gain selection
    ///
    /// Derived from wind speed and compass swing statistics.
    /// Calm (<15kn), Moderate (15-25kn), Rough (>25kn).
    pub sea_state: SeaState,
}

impl Default for EnvironmentState {
    fn default() -> Self {
        Self {
            wind_speed_kn: None,
            wind_angle_deg: None,
            sea_state: SeaState::Unknown,
        }
    }
}

/// Sea state classification for gain scheduling.
///
/// Different sea states require different PID parameters for smooth
/// control. This classification selects which parameter set to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SeaState {
    /// <15 knots wind, <1 foot seas — calm conditions
    Calm,
    /// 15-25 knots wind, 1-3 foot seas — moderate conditions
    Moderate,
    /// >25 knots wind, >3 foot seas — rough conditions
    Rough,
    /// Insufficient data to classify
    Unknown,
}

/// Human state: autonomy dial, override status.
///
/// This is how the human expresses intent to the system. The dial
/// is a CEILING, not a floor — the envelope can further reduce
/// autonomy based on conditions (docs/09).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanState {
    /// Current autonomy level (set via dial or escalation)
    pub dial: AutonomyLevel,
    /// Human override is active (jog lever moved)
    ///
    /// When true, all AI intents are rejected and drivers have already
    /// been preempted by hardware veto. This is the ultimate safety.
    pub override_active: bool,
    /// Last time override was detected (for timeout)
    pub last_override_ms: Option<u64>,
}

impl Default for HumanState {
    fn default() -> Self {
        Self {
            dial: AutonomyLevel::Coach, // Restart default (docs/09)
            override_active: false,
            last_override_ms: None,
        }
    }
}

/// The snapshot — the single authoritative vessel state.
///
/// Every tick produces a new snapshot. The `state_hash` chains
/// snapshots so black-box entries can reference an exact world-state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VesselState {
    /// Tick counter (monotonically increasing)
    pub tick: u64,
    /// Snapshot timestamp (epoch ms)
    pub timestamp_ms: u64,
    /// Navigation digital twin
    pub nav: NavigationState,
    /// Propulsion digital twin
    pub propulsion: PropulsionState,
    /// Environment digital twin
    pub environment: EnvironmentState,
    /// Human intent state
    pub human: HumanState,
    /// Degraded mode cause (if any)
    ///
    /// When set, the envelope will not approve any intents beyond
    /// safe maintenance actions. This is how we handle "GPS failed —
    /// proceed at dead slow only" scenarios.
    pub degraded: Option<String>,
    /// Hash of this state for audit trail integrity
    ///
    /// Format: SHA256(state_json). Used by black box to anchor
    /// "what the world looked like when this decision was made."
    pub state_hash: String,
}

impl Default for VesselState {
    fn default() -> Self {
        // NOTE: hash left empty here — computing it via Self::default()
        // recursed infinitely (found in review). Use genesis() instead,
        // which computes the hash exactly once.
        Self {
            tick: 0,
            timestamp_ms: 0,
            nav: NavigationState::default(),
            propulsion: PropulsionState::default(),
            environment: EnvironmentState::default(),
            human: HumanState::default(),
            degraded: None,
            state_hash: String::new(),
        }
    }
}

impl VesselState {
    /// Create the genesis state — all sensors empty, dial at Coach.
    ///
    /// This is the starting point for every boot and every replay.
    /// timestamp_ms is 0 (deterministic): the state's clock only advances
    /// via event timestamps inside reduce(). Wall clock never enters here —
    /// two genesis states must hash identically, or replay anchors drift.
    pub fn genesis() -> Self {
        let state = Self {
            tick: 0,
            timestamp_ms: 0,
            nav: NavigationState::default(),
            propulsion: PropulsionState::default(),
            environment: EnvironmentState::default(),
            human: HumanState::default(),
            degraded: None,
            state_hash: String::new(), // Will compute below
        };

        state.with_computed_hash()
    }

    /// ═══════════════════════════════════════════════════════════════════
    ///  THE REDUCER — pure function, no I/O, fully replayable
    /// ═══════════════════════════════════════════════════════════════════
    ///
    /// This function's purity is what makes replay, simulation, and
    /// codegen testing possible. Guard it with your life.
    ///
    /// Process: events → apply per-kind → update derived → hash → new state
    pub fn reduce(&mut self, events: &[Event]) {
        // Apply each event in order (events are already sorted by timestamp)
        for event in events {
            // Skip narrative lane events — they never touch state
            if event.lane == Lane::Narrative {
                continue;
            }

            // Update tick and timestamp
            self.tick += 1;
            self.timestamp_ms = event.timestamp_ms;

            // Apply event based on kind
            match &event.kind {
                // Sensing events
                EventKind::GpsFix(p) => self.apply_gps(p, event.timestamp_ms),
                EventKind::CompassHeading(p) => self.apply_compass(p, event.timestamp_ms),
                EventKind::DepthSounder(p) => self.apply_depth(p, event.timestamp_ms),
                EventKind::EngineRpm(p) => self.apply_engine(p, event.timestamp_ms),
                EventKind::WindApparent(p) => self.apply_wind(p, event.timestamp_ms),
                EventKind::RudderAngle(p) => self.apply_rudder(p, event.timestamp_ms),

                // Human events
                EventKind::DialSet(p) => {
                    self.human.dial = p.level;
                }
                EventKind::JogLeverMove(_) => {
                    self.human.override_active = true;
                    self.human.last_override_ms = Some(event.timestamp_ms);
                }

                // System events
                EventKind::DegradedMode(p) => {
                    self.degraded = Some(p.cause.clone());
                }

                // Control events never modify state directly
                // (they go through envelope and produce verdicts)
                _ => {
                    // Narrative lane already filtered above
                    // Control kinds don't touch state
                }
            }
        }

        // Update derived quantities (trends, sea state, staleness, hash)
        self.update_derived();
    }

    /// Apply GPS fix to navigation state.
    ///
    /// GPS is authoritative for position and SOG/COG. We don't fuse
    /// this with anything else — it's the ground truth for navigation.
    fn apply_gps(&mut self, payload: &GpsFix, ts: u64) {
        self.nav.lat = Some(payload.lat);
        self.nav.lon = Some(payload.lon);
        self.nav.sog_kn = Some(payload.sog_kn);
        self.nav.cog_deg = Some(payload.cog_deg);

        // Update health from the EVENT timestamp (never wall clock — A2)
        self.nav.gps_health.update_at(ts, self.timestamp_ms, 5000); // 5s staleness threshold

        // HDOP > 5.0 indicates poor geometry — mark degraded
        if payload.hdop > 5.0 {
            self.nav.gps_health.mark_degraded();
        }
    }

    /// Apply compass heading to navigation state.
    ///
    /// Heading is fused from magnetic compass. Gyro fallback would
    /// go here when we have that sensor.
    fn apply_compass(&mut self, payload: &CompassHeading, ts: u64) {
        self.nav.heading_deg = Some(payload.heading_deg);
        self.nav.swing_rate_dps = Some(payload.swing_rate_dps);

        // Update health
        self.nav.compass_health.update_at(ts, self.timestamp_ms, 2000); // 2s staleness threshold

        // Excessive swing indicates compass issues or rough seas
        if payload.swing_rate_dps.abs() > 30.0 {
            self.nav.compass_health.mark_degraded();
        }
    }

    /// Apply depth sounder reading to navigation state.
    ///
    /// Depth is critical for shoaling detection. We maintain a history
    /// to compute depth trend (meters per minute).
    fn apply_depth(&mut self, payload: &DepthSounder, ts: u64) {
        self.nav.depth_m = Some(payload.depth_m);
        self.nav.bottom_hardness = payload.bottom_hardness;

        // Update health
        self.nav.depth_health.update_at(ts, self.timestamp_ms, 3000); // 3s staleness threshold
    }

    /// Apply engine RPM report to propulsion state.
    ///
    /// Comes from N2K PGN 127488 or legacy analog tachometer.
    fn apply_engine(&mut self, payload: &EngineRpm, ts: u64) {
        self.propulsion.rpm = Some(payload.rpm);
        self.propulsion.throttle_pct = Some(payload.throttle_pct);

        // Update health
        self.propulsion.engine_health.update_at(ts, self.timestamp_ms, 1000); // 1s staleness threshold
    }

    /// Apply wind sensor reading to environment state.
    ///
    /// Used for wind-aware trolling and sea state classification.
    fn apply_wind(&mut self, payload: &WindApparent, _ts: u64) {
        self.environment.wind_speed_kn = Some(payload.speed_kn);
        self.environment.wind_angle_deg = Some(payload.angle_deg);

        // Update sea state classification based on wind
        self.environment.sea_state = Self::classify_sea_state(payload.speed_kn);
    }

    /// Apply rudder angle to propulsion state.
    ///
    /// This is feedback from the rudder position sensor, not command.
    /// The envelope compares intent vs. actual.
    fn apply_rudder(&mut self, payload: &RudderAngle, _ts: u64) {
        self.propulsion.rudder_angle_deg = Some(payload.angle_deg);
    }

    /// Update derived quantities: depth trend, sea state, staleness, hash.
    ///
    /// These are computed from the raw state above:
    /// - Depth trend: rate of change (meters per minute)
    /// - Sea state: already updated in apply_wind, but can refine here
    /// - Staleness: age checks for all sensors
    /// - State hash: SHA256 of the entire state
    fn update_derived(&mut self) {
        // Update staleness for all sensors
        self.update_staleness();

        // Compute depth trend from depth history
        // TODO: Maintain depth history buffer for proper trend calculation
        // For now, set to None if we don't have history
        self.nav.depth_trend_m_per_min = None;

        // Update sea state based on combined conditions
        // (Wind + compass swing gives better classification)
        if self.environment.wind_speed_kn.is_some() && self.nav.swing_rate_dps.is_some() {
            self.environment.sea_state = Self::classify_sea_state_combined(
                self.environment.wind_speed_kn.unwrap_or(0.0),
                self.nav.swing_rate_dps.unwrap_or(0.0),
            );
        }

        // Compute final hash
        self.state_hash = Self::hash_self(self);
    }

    /// Update staleness flags for all sensors.
    ///
    /// This is how the system knows when to stop trusting a sensor.
    /// The envelope reads these flags before approving any intent.
    fn update_staleness(&mut self) {
        // The state's OWN clock (last event timestamp) is the reference —
        // never wall clock. This is what makes replay deterministic: the
        // same event stream produces the same staleness flags.
        let now = self.timestamp_ms;

        // Check GPS staleness (5s threshold)
        self.nav.gps_health.stale = now.saturating_sub(self.nav.gps_health.last_update_ms) > 5000;

        // Check compass staleness (2s threshold)
        self.nav.compass_health.stale = now.saturating_sub(self.nav.compass_health.last_update_ms) > 2000;

        // Check depth staleness (3s threshold)
        self.nav.depth_health.stale = now.saturating_sub(self.nav.depth_health.last_update_ms) > 3000;

        // Check engine staleness (1s threshold)
        self.propulsion.engine_health.stale = now.saturating_sub(self.propulsion.engine_health.last_update_ms) > 1000;
    }

    /// Classify sea state from wind speed alone.
    fn classify_sea_state(wind_kn: f32) -> SeaState {
        if wind_kn < 15.0 {
            SeaState::Calm
        } else if wind_kn < 25.0 {
            SeaState::Moderate
        } else {
            SeaState::Rough
        }
    }

    /// Classify sea state from combined wind and motion data.
    ///
    /// High compass swing + moderate wind = rough seas (wave action).
    /// This is more accurate than wind alone.
    fn classify_sea_state_combined(wind_kn: f32, swing_dps: f32) -> SeaState {
        let swing_factor = swing_dps.abs() / 10.0; // Normalize

        if wind_kn < 12.0 && swing_factor < 1.0 {
            SeaState::Calm
        } else if wind_kn < 22.0 && swing_factor < 2.0 {
            SeaState::Moderate
        } else {
            SeaState::Rough
        }
    }

    /// Compute SHA256 hash of this state.
    ///
    /// This is used by the black box to anchor decisions to the exact
    /// world-state that produced them. Critical for replay and audit.
    fn hash_self(state: &VesselState) -> String {
        let json = serde_json::to_string(state)
            .unwrap_or_else(|e| {
                eprintln!("Failed to serialize state for hashing: {}", e);
                "{}".to_string()
            });

        let mut hasher = Sha256::new();
        hasher.update(json.as_bytes());
        let result = hasher.finalize();
        hex::encode(result)
    }

    /// Return a new state with computed hash.
    fn with_computed_hash(mut self) -> Self {
        self.state_hash = Self::hash_self(&self);
        self
    }
}

/// Get current time in milliseconds since epoch.
///
/// Used consistently for all timestamping to ensure monotonic ordering.
fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Frozen, immutable view handed to playbooks.
///
/// Playbooks get a snapshot, never a live reference — this is what
/// keeps them pure. They cannot modify state, only read it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot(pub VesselState);

impl StateSnapshot {
    /// Create a snapshot from the current state.
    pub fn from(state: &VesselState) -> Self {
        Self(state.clone())
    }

    /// Get the inner state (read-only).
    pub fn get(&self) -> &VesselState {
        &self.0
    }

    /// Check if all critical sensors are fresh.
    ///
    /// Returns true if GPS, compass, and depth are all within staleness thresholds.
    pub fn all_sensors_fresh(&self) -> bool {
        !self.0.nav.gps_health.stale
            && !self.0.nav.compass_health.stale
            && !self.0.nav.depth_health.stale
    }

    /// Check if any sensor is degraded.
    ///
    /// Degraded means functioning but with poor quality (e.g., high HDOP).
    pub fn any_sensor_degraded(&self) -> bool {
        self.0.nav.gps_health.degraded
            || self.0.nav.compass_health.degraded
            || self.0.nav.depth_health.degraded
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::{Event, Source, Provenance};
    use crate::bus::Lane;
    use ulid::Ulid;

    fn make_test_event(kind: EventKind) -> Event {
        Event {
            id: Ulid::new(),
            seq: 0,
            timestamp_ms: now_ms(),
            lane: Lane::Telemetry,
            kind,
            source: Source {
                module: "driver.test".to_string(),
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
    fn test_genesis_state() {
        let state = VesselState::genesis();

        assert_eq!(state.tick, 0);
        assert_eq!(state.human.dial, AutonomyLevel::Coach);
        assert!(!state.state_hash.is_empty());
        assert!(state.nav.lat.is_none());
        assert!(state.propulsion.rpm.is_none());
    }

    #[test]
    fn test_gps_reducer() {
        let mut state = VesselState::genesis();

        let event = make_test_event(EventKind::GpsFix(GpsFix {
            lat: 45.5,
            lon: -122.5,
            sog_kn: 2.3,
            cog_deg: 147.0,
            hdop: 1.2,
            sats: 12,
        }));

        state.reduce(&[event]);

        assert_eq!(state.nav.lat, Some(45.5));
        assert_eq!(state.nav.lon, Some(-122.5));
        assert_eq!(state.nav.sog_kn, Some(2.3));
        assert_eq!(state.nav.cog_deg, Some(147.0));
        assert!(!state.nav.gps_health.stale);
        assert!(!state.nav.gps_health.degraded);
    }

    #[test]
    fn test_hdop_degradation() {
        let mut state = VesselState::genesis();

        let event = make_test_event(EventKind::GpsFix(GpsFix {
            lat: 45.5,
            lon: -122.5,
            sog_kn: 2.3,
            cog_deg: 147.0,
            hdop: 6.0, // Poor geometry
            sats: 4,
        }));

        state.reduce(&[event]);

        assert!(state.nav.gps_health.degraded);
    }

    #[test]
    fn test_dial_set() {
        let mut state = VesselState::genesis();

        let event = make_test_event(EventKind::DialSet(DialSet {
            level: AutonomyLevel::Supervise,
            by_whom: "captain".to_string(),
        }));

        state.reduce(&[event]);

        assert_eq!(state.human.dial, AutonomyLevel::Supervise);
    }

    #[test]
    fn test_sea_state_classification() {
        assert_eq!(VesselState::classify_sea_state(10.0), SeaState::Calm);
        assert_eq!(VesselState::classify_sea_state(18.0), SeaState::Moderate);
        assert_eq!(VesselState::classify_sea_state(30.0), SeaState::Rough);
    }

    #[test]
    fn test_state_hash_stability() {
        let state1 = VesselState::genesis();
        let state2 = VesselState::genesis();

        // Same state should produce same hash
        assert_eq!(state1.state_hash, state2.state_hash);
    }

    #[test]
    fn test_snapshot_isolation() {
        let mut state = VesselState::genesis();

        let snapshot = StateSnapshot::from(&state);

        // Modify original state
        state.nav.lat = Some(45.5);

        // Snapshot should be unchanged
        assert!(snapshot.get().nav.lat.is_none());
    }

    #[test]
    fn test_all_sensors_fresh() {
        let mut state = VesselState::genesis();

        // No updates yet — should be stale
        assert!(!StateSnapshot::from(&state).all_sensors_fresh());

        // Apply GPS event
        let gps_event = make_test_event(EventKind::GpsFix(GpsFix {
            lat: 45.5,
            lon: -122.5,
            sog_kn: 2.3,
            cog_deg: 147.0,
            hdop: 1.0,
            sats: 12,
        }));
        state.reduce(&[gps_event]);

        // Still not fresh (missing compass and depth)
        assert!(!StateSnapshot::from(&state).all_sensors_fresh());
    }
}
