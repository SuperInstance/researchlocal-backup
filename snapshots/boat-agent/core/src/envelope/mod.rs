//! ═══════════════════════════════════════════════════════════════════
//!  THE SAFETY ENVELOPE — AGENT READ-ONLY (prime directive 1)
//!
//!  The vessel's reflexes. Every physical output in the system passes
//!  through `arbitrate()` and every actuator write is performed by this
//!  module. There is no second door (docs/05 law 4).
//!
//!  This code trusts NOTHING above it: playbooks are assumed adversarial,
//!  agents are assumed fallible, networks are assumed absent. It trusts
//!  only: vessel.toml limits, hardware watchdog heartbeats, and the
//!  human's hands.
//!
//!  Any agent that believes this module needs to change: STOP.
//!  Emit an escalation. A human reviews envelope changes. No exceptions.
//! ═══════════════════════════════════════════════════════════════════

use crate::bus::events::{AutonomyLevel, Intent, Verdict, VerdictOutcome};
use crate::config::EnvelopeLimits;
use crate::state::VesselState;
use serde::{Deserialize, Serialize};

/// Rate-limit memory. Kept minimal and explicit.
///
/// The envelope remembers recent actuation to enforce rate limits and
/// detect patterns like oscillation or runaway commands.
#[derive(Debug, Default)]
pub struct EnvelopeMemory {
    /// Last rudder command (degrees) — for rate limiting
    pub last_rudder_deg: Option<f32>,
    /// Last throttle command (percent) — for rate limiting
    pub last_throttle_pct: Option<f32>,
    /// Last command timestamp (epoch ms) — for cooldown enforcement
    pub last_command_ms: u64,
    /// Consecutive rejections — for detecting failing playbooks
    pub consecutive_rejections: u32,
    /// Missed kernel heartbeats — watchdog counter
    pub missed_kernel_heartbeats: u32,
}

/// The safety envelope — the one door to physical actuation.
///
/// This module is the ONLY part of the system that can command actuators.
/// All intents from playbooks must pass through `arbitrate()` first.
pub struct SafetyEnvelope {
    /// Safety limits from vessel.toml — never hardcoded per boat
    limits: EnvelopeLimits,
    /// Rate limit and pattern detection memory
    memory: EnvelopeMemory,
}

/// The result of envelope arbitration.
///
/// Every intent becomes one of these outcomes. The verdict is emitted
/// to the bus and logged to the black box for audit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Arbitration {
    /// Intent is safe to execute as requested
    Approved(Intent),
    /// Intent is partially safe — clamp to safe limits and execute
    Clamped { command: Intent, reason: String },
    /// Intent is unsafe — do not execute
    Rejected(String),
}

impl SafetyEnvelope {
    /// Create a new safety envelope with the given limits.
    pub fn new(limits: EnvelopeLimits) -> Self {
        Self {
            limits,
            memory: EnvelopeMemory::default(),
        }
    }

    /// ═══════════════════════════════════════════════════════════════════
    ///  THE ONE DOOR — called once per tick per intent
    /// ═══════════════════════════════════════════════════════════════════
    ///
    /// Check order is deliberate — cheapest and most absolute first.
    /// Each check is a pure function of (intent, state, limits, memory).
    /// No I/O, no network calls, fully replayable.
    ///
    /// Checks (in order):
    ///   1. HUMAN VETO       — override_active → reject everything
    ///   2. WATCHDOG         — kernel heartbeat stale → safe state
    ///   3. DIAL CEILING      — dial < required → downgrade to advisory/reject
    ///   4. SENSOR SANITY     — actuating on stale sensors → reject
    ///   5. HARD BOUNDS       — rudder ±limit, throttle range, RPM redline
    ///   6. RATE LIMITS       — per-tick deltas, cooldowns
    ///   7. CONTEXT GUARDS    — min speed for autopilot, shoaling trend
    pub fn arbitrate(&mut self, intent: &Intent, state: &VesselState) -> Arbitration {
        // Check 1: HUMAN VETO — absolute preemption
        if state.human.override_active {
            return self.reject("human override active — all intents rejected until jog lever quiet");
        }

        // Check 2: WATCHDOG — kernel heartbeat stale
        if self.memory.missed_kernel_heartbeats >= self.limits.watchdog_miss_threshold {
            return self.reject_owned(format!(
                "watchdog tripped: missed {} heartbeats — safe state held",
                self.memory.missed_kernel_heartbeats
            ));
        }

        // Check 3: DIAL CEILING — effective autonomy = min(dial, conditions allow)
        let effective_dial = self.effective_autonomy(state);

        // Log = observation only. Coach = advisory only: intents may be
        // displayed to the human, but NOTHING actuates. The kernel surfaces
        // advisory intents as shadow deltas; the envelope's actuation answer
        // at these levels is always Rejected. (docs/09)
        if effective_dial == AutonomyLevel::Log {
            return self.reject("autonomy dial at Log — observation only, no actuation");
        }
        if effective_dial == AutonomyLevel::Coach {
            return self.reject(
                "autonomy dial at Coach — advisory only, no actuation (intent logged as shadow delta)"
            );
        }
        // TODO(docs/09): Supervise should also require standing consent per
        // action class. Consent machinery lands with the mission layer;
        // until then Supervise actuates like Autopilot within these checks.

        // Check 4: SENSOR SANITY — don't actuate on bad data
        if let Some(reason) = self.check_sensor_sanity(state, intent) {
            return self.reject_owned(reason);
        }

        // Check 5: HARD BOUNDS — clamp, then KEEP CHECKING. Clamping bounds
        // magnitude; it says nothing about rate or context. A clamped
        // command must still pass checks 6–7 like any other.
        let (effective, clamp_reason) = match self.clamp_hard_bounds(intent) {
            Some(clamped) => (
                clamped,
                Some("intent exceeds vessel hard limits — clamped to safe values".to_string()),
            ),
            None => (intent.clone(), None),
        };

        // Check 6: RATE LIMITS — applied to the (possibly clamped) command.
        // Time source: the state's own timestamp, never wall clock (A2).
        if let Some(reason) = self.check_rate_limits(&effective, state.timestamp_ms) {
            return self.reject_owned(reason);
        }

        // Check 7: CONTEXT GUARDS — situation-specific safety
        if let Some(reason) = self.check_context_guards(state, &effective) {
            return self.reject_owned(reason);
        }

        // All checks passed
        self.memory.consecutive_rejections = 0;
        match clamp_reason {
            Some(reason) => Arbitration::Clamped {
                command: effective,
                reason,
            },
            None => Arbitration::Approved(effective),
        }
    }

    /// Rejection helper: uniform consecutive-rejection accounting.
    /// Every rejection path goes through here — no exceptions.
    fn reject(&mut self, reason: &str) -> Arbitration {
        self.memory.consecutive_rejections += 1;
        Arbitration::Rejected(reason.to_string())
    }

    fn reject_owned(&mut self, reason: String) -> Arbitration {
        self.memory.consecutive_rejections += 1;
        Arbitration::Rejected(reason)
    }

    /// Effective autonomy = min(dial, what current conditions allow).
    ///
    /// Sensor degradation or missed heartbeats automatically lower
    /// the autonomy ceiling. The dial sets the maximum, conditions
    /// may require it to be lower (docs/09).
    pub fn effective_autonomy(&self, state: &VesselState) -> AutonomyLevel {
        let dial = state.human.dial;

        // If degraded or watchdog issues, cap at Supervise
        if state.degraded.is_some() || self.memory.missed_kernel_heartbeats > 0 {
            return dial.min(AutonomyLevel::Supervise);
        }

        // If critical sensors are stale, cap at Supervise
        if state.nav.gps_health.stale || state.nav.compass_health.stale {
            return dial.min(AutonomyLevel::Supervise);
        }

        dial
    }

    /// Check sensor sanity — don't actuate on stale or degraded data.
    fn check_sensor_sanity(&self, state: &VesselState, intent: &Intent) -> Option<String> {
        // Steering requires fresh GPS and compass
        if intent.requested_rudder_deg.is_some() {
            if state.nav.gps_health.stale {
                return Some("GPS stale — cannot steer without position reference".to_string());
            }
            if state.nav.compass_health.stale {
                return Some("Compass stale — cannot steer without heading reference".to_string());
            }
        }

        // Throttle commands require fresh engine data
        if intent.requested_throttle_pct.is_some() {
            if state.propulsion.engine_health.stale {
                return Some("Engine data stale — cannot adjust throttle without RPM reference".to_string());
            }
        }

        None
    }

    /// Clamp to hard bounds from vessel.toml — absolute limits.
    /// Returns Some(clamped) only if something actually changed.
    fn clamp_hard_bounds(&self, intent: &Intent) -> Option<Intent> {
        let mut clamped = intent.clone();

        // Hard rudder limit (±max_rudder_deg)
        if let Some(rudder) = intent.requested_rudder_deg {
            let max = self.limits.max_rudder_deg;
            if rudder.abs() > max {
                clamped.requested_rudder_deg = Some(rudder.clamp(-max, max));
            }
        }

        // Hard throttle range (0-100%, with trolling ceiling)
        if let Some(throttle) = intent.requested_throttle_pct {
            // Clamp to 0-100 first
            let clamped_throttle = throttle.clamp(0.0, 100.0);

            // If in trolling mode (we detect this via RPM or context),
            // enforce the trolling ceiling
            if self.is_trolling_mode() && clamped_throttle > self.limits.trolling_throttle_ceiling_pct {
                clamped.requested_throttle_pct = Some(self.limits.trolling_throttle_ceiling_pct);
            } else {
                clamped.requested_throttle_pct = Some(clamped_throttle);
            }
        }

        // Return clamped intent if anything changed
        if clamped.requested_rudder_deg != intent.requested_rudder_deg
            || clamped.requested_throttle_pct != intent.requested_throttle_pct
        {
            Some(clamped)
        } else {
            None
        }
    }

    /// Check rate limits — prevent oscillation and runaway commands.
    ///
    /// `now_ms` is the STATE's timestamp (last event time), passed in by the
    /// caller — this function must stay pure for replay (A2).
    fn check_rate_limits(&mut self, intent: &Intent, now_ms: u64) -> Option<String> {
        let time_since_last_command = now_ms.saturating_sub(self.memory.last_command_ms);

        // Minimum interval between commands (from vessel.toml)
        let min_interval = self.limits.min_command_interval_ms;

        if self.memory.last_command_ms > 0 && time_since_last_command < min_interval {
            return Some(format!(
                "rate limit: only {}ms since last command (min {}ms)",
                time_since_last_command, min_interval
            ));
        }

        // Check rudder rate limit (max degrees per command, from vessel.toml)
        if let (Some(last), Some(requested)) = (
            self.memory.last_rudder_deg,
            intent.requested_rudder_deg
        ) {
            let delta = (requested - last).abs();
            let max_delta = self.limits.max_rudder_step_deg;

            if delta > max_delta {
                return Some(format!(
                    "rudder rate limit: {:.1}° change exceeds max {:.1}° per command",
                    delta, max_delta
                ));
            }
        }

        // Check throttle rate limit (from vessel.toml)
        if let (Some(last), Some(requested)) = (
            self.memory.last_throttle_pct,
            intent.requested_throttle_pct
        ) {
            let delta = (requested - last).abs();
            let max_delta = self.limits.max_throttle_step_pct;

            if delta > max_delta {
                return Some(format!(
                    "throttle rate limit: {:.1}% change exceeds max {:.1}% per command",
                    delta, max_delta
                ));
            }
        }

        None
    }

    /// Check context guards — situation-specific safety rules.
    fn check_context_guards(&self, state: &VesselState, intent: &Intent) -> Option<String> {
        // Guard: minimum speed for autopilot engagement
        if intent.requested_rudder_deg.is_some() {
            if let Some(sog) = state.nav.sog_kn {
                if sog < self.limits.min_autopilot_speed_kn {
                    return Some(format!(
                        "autopilot speed guard: vessel speed {:.1}kn below minimum {:.1}kn — \
                         rudder commands rejected (no steerage at low speed)",
                        sog, self.limits.min_autopilot_speed_kn
                    ));
                }
            }
        }

        // Guard: RPM redline protection
        if let Some(requested_throttle) = intent.requested_throttle_pct {
            if let Some(current_rpm) = state.propulsion.rpm {
                // Estimate RPM resulting from this throttle (rough approximation)
                // If it would exceed max_trolling_rpm, reject
                if requested_throttle > 50.0 && current_rpm > self.limits.max_trolling_rpm * 80 / 100 {
                    return Some(format!(
                        "RPM redline guard: current RPM {} near max {} — throttle increase rejected",
                        current_rpm, self.limits.max_trolling_rpm
                    ));
                }
            }
        }

        // Guard: shoaling detection (depth trend)
        if let Some(trend) = state.nav.depth_trend_m_per_min {
            if trend < -self.limits.shoaling_reject_m_per_min {
                return Some(format!(
                    "shoaling guard: depth trend {:.1}m/min indicates rapid shoaling — \
                     all propulsion commands rejected (vessel heading toward shallow water)",
                    trend
                ));
            }
        }

        // Guard: excessive compass swing (rough seas or compass failure)
        if let (Some(swing), Some(rudder_cmd)) = (
            state.nav.swing_rate_dps,
            intent.requested_rudder_deg
        ) {
            if swing.abs() > self.limits.max_swing_dps && rudder_cmd.abs() > 5.0 {
                return Some(format!(
                    "compass swing guard: swing rate {:.1}°/sec exceeds threshold — \
                     rudder commands rejected (vessel in rough seas or compass failing)",
                    swing
                ));
            }
        }

        None
    }

    /// Detect if we're in trolling mode (low speed, fishing operation).
    ///
    /// Used to apply trolling-specific limits like throttle ceiling.
    fn is_trolling_mode(&self) -> bool {
        // TODO: Proper trolling mode detection from state
        // For now, assume trolling if RPM is low and we're not in transit
        // This would be determined by mission context
        false
    }

    /// Kernel heartbeat, once per tick.
    ///
    /// Missing ticks are counted by the CALLER's absence — the watchdog
    /// hardware relay is armed here. If the kernel stops calling this,
    /// the relay opens and the buzzer sounds.
    pub fn heartbeat(&mut self, tick: u64) {
        // Reset missed heartbeat counter on successful heartbeat
        self.memory.missed_kernel_heartbeats = 0;
        let _ = tick;
    }

    /// Increment missed heartbeat counter (called by watchdog).
    pub fn record_missed_heartbeat(&mut self) {
        self.memory.missed_kernel_heartbeats += 1;
    }

    /// Convert an arbitration into the bus-visible verdict event.
    pub fn to_verdict(&self, intent_id: ulid::Ulid, arb: Arbitration) -> Verdict {
        match arb {
            Arbitration::Approved(cmd) => Verdict {
                intent_id,
                outcome: VerdictOutcome::Approved,
                reason: String::new(),
                final_command: Some(cmd),
            },
            Arbitration::Clamped { command, reason } => Verdict {
                intent_id,
                outcome: VerdictOutcome::Clamped,
                reason,
                final_command: Some(command),
            },
            Arbitration::Rejected(reason) => Verdict {
                intent_id,
                outcome: VerdictOutcome::Rejected,
                reason,
                final_command: None,
            },
        }
    }

    /// Update memory with the last command (for rate limiting).
    ///
    /// `ts_ms` is the STATE's timestamp at command time — passed in by the
    /// kernel so rate limiting stays deterministic under replay (A2).
    pub fn update_command_memory(&mut self, rudder: Option<f32>, throttle: Option<f32>, ts_ms: u64) {
        self.memory.last_rudder_deg = rudder;
        self.memory.last_throttle_pct = throttle;
        self.memory.last_command_ms = ts_ms;
    }

    /// Get consecutive rejection count (for detecting failing playbooks).
    pub fn consecutive_rejections(&self) -> u32 {
        self.memory.consecutive_rejections
    }

    /// Get missed heartbeat count (for watchdog status).
    pub fn missed_heartbeats(&self) -> u32 {
        self.memory.missed_kernel_heartbeats
    }
}

/// Get current time in milliseconds since epoch.
fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::events::Intent;
    use ulid::Ulid;

    fn make_test_limits() -> EnvelopeLimits {
        EnvelopeLimits {
            max_rudder_deg: 15.0,
            min_autopilot_speed_kn: 2.0,
            max_trolling_rpm: 1800,
            max_throttle_step_pct: 15.0,
            trolling_throttle_ceiling_pct: 40.0,
            watchdog_heartbeat_ms: 300,
            watchdog_miss_threshold: 3,
            shoaling_reject_m_per_min: 2.0,
            min_command_interval_ms: 200,
            max_rudder_step_deg: 15.0,
            max_swing_dps: 20.0,
        }
    }

    fn make_test_state() -> VesselState {
        use crate::state::{NavigationState, PropulsionState, EnvironmentState, HumanState, SensorHealth};

        let mut nav = NavigationState::default();
        nav.lat = Some(45.5);
        nav.lon = Some(-122.5);
        nav.sog_kn = Some(3.0); // Above min speed
        nav.heading_deg = Some(147.0);
        nav.gps_health = SensorHealth::fresh_now();
        nav.compass_health = SensorHealth::fresh_now();

        let mut propulsion = PropulsionState::default();
        propulsion.rpm = Some(1200);
        propulsion.engine_health = SensorHealth::fresh_now();

        let mut human = HumanState::default();
        human.dial = AutonomyLevel::Autopilot; // Coach/Log reject actuation — tests need actuation level

        VesselState {
            tick: 1,
            timestamp_ms: now_ms(),
            nav,
            propulsion,
            environment: EnvironmentState::default(),
            human,
            degraded: None,
            state_hash: String::new(),
        }
    }

    #[test]
    fn test_human_override_rejects_all() {
        let mut envelope = SafetyEnvelope::new(make_test_limits());
        let mut state = make_test_state();
        state.human.override_active = true;

        let intent = Intent {
            requested_rudder_deg: Some(5.0),
            requested_throttle_pct: None,
            horizon_s: 1.0,
        };

        let arb = envelope.arbitrate(&intent, &state);

        assert!(matches!(arb, Arbitration::Rejected(_)));
        if let Arbitration::Rejected(reason) = arb {
            assert!(reason.contains("human override"));
        }
    }

    #[test]
    fn test_watchdog_trip_rejects_all() {
        let mut envelope = SafetyEnvelope::new(make_test_limits());
        envelope.memory.missed_kernel_heartbeats = 5;

        let state = make_test_state();
        let intent = Intent {
            requested_rudder_deg: Some(5.0),
            requested_throttle_pct: None,
            horizon_s: 1.0,
        };

        let arb = envelope.arbitrate(&intent, &state);

        assert!(matches!(arb, Arbitration::Rejected(_)));
        if let Arbitration::Rejected(reason) = arb {
            assert!(reason.contains("watchdog"));
        }
    }

    #[test]
    fn test_stale_gps_rejects_steering() {
        let mut envelope = SafetyEnvelope::new(make_test_limits());
        let mut state = make_test_state();
        state.nav.gps_health.stale = true;

        let intent = Intent {
            requested_rudder_deg: Some(5.0),
            requested_throttle_pct: None,
            horizon_s: 1.0,
        };

        let arb = envelope.arbitrate(&intent, &state);

        assert!(matches!(arb, Arbitration::Rejected(_)));
        if let Arbitration::Rejected(reason) = arb {
            assert!(reason.contains("GPS stale"));
        }
    }

    #[test]
    fn test_min_speed_guard() {
        let mut envelope = SafetyEnvelope::new(make_test_limits());
        let mut state = make_test_state();
        state.nav.sog_kn = Some(1.5); // Below min speed

        let intent = Intent {
            requested_rudder_deg: Some(5.0),
            requested_throttle_pct: None,
            horizon_s: 1.0,
        };

        let arb = envelope.arbitrate(&intent, &state);

        assert!(matches!(arb, Arbitration::Rejected(_)));
        if let Arbitration::Rejected(reason) = arb {
            assert!(reason.contains("below minimum"));
        }
    }

    #[test]
    fn test_rudder_clamp() {
        let mut envelope = SafetyEnvelope::new(make_test_limits());
        let state = make_test_state();

        let intent = Intent {
            requested_rudder_deg: Some(25.0), // Exceeds max of 15
            requested_throttle_pct: None,
            horizon_s: 1.0,
        };

        let arb = envelope.arbitrate(&intent, &state);

        assert!(matches!(arb, Arbitration::Clamped { .. }));
        if let Arbitration::Clamped { command, .. } = arb {
            assert_eq!(command.requested_rudder_deg, Some(15.0));
        }
    }

    #[test]
    fn test_throttle_rate_limit() {
        let mut envelope = SafetyEnvelope::new(make_test_limits());
        let state = make_test_state();

        // Set up last command
        envelope.memory.last_throttle_pct = Some(20.0);
        envelope.memory.last_command_ms = now_ms();

        // Try to change by 20% (exceeds max of 15%)
        let intent = Intent {
            requested_rudder_deg: None,
            requested_throttle_pct: Some(40.0),
            horizon_s: 1.0,
        };

        let arb = envelope.arbitrate(&intent, &state);

        assert!(matches!(arb, Arbitration::Rejected(_)));
        if let Arbitration::Rejected(reason) = arb {
            assert!(reason.contains("rate limit"));
        }
    }

    #[test]
    fn test_safe_intent_approved() {
        let mut envelope = SafetyEnvelope::new(make_test_limits());
        envelope.memory.last_command_ms = now_ms() - 1000; // 1s ago

        let state = make_test_state();

        let intent = Intent {
            requested_rudder_deg: Some(5.0),
            requested_throttle_pct: Some(25.0),
            horizon_s: 1.0,
        };

        let arb = envelope.arbitrate(&intent, &state);

        assert!(matches!(arb, Arbitration::Approved(_)));
    }

    #[test]
    fn test_effective_autonomy_degraded() {
        let mut envelope = SafetyEnvelope::new(make_test_limits());
        let mut state = make_test_state();
        state.degraded = Some("GPS failed".to_string());

        // Dial at Autopilot, but degraded should cap at Supervise
        state.human.dial = AutonomyLevel::Autopilot;

        let effective = envelope.effective_autonomy(&state);
        assert_eq!(effective, AutonomyLevel::Supervise);
    }

    #[test]
    fn test_to_verdict() {
        let mut envelope = SafetyEnvelope::new(make_test_limits());

        let intent_id = Ulid::new();
        let intent = Intent {
            requested_rudder_deg: Some(5.0),
            requested_throttle_pct: None,
            horizon_s: 1.0,
        };

        let verdict = envelope.to_verdict(intent_id, Arbitration::Approved(intent.clone()));

        assert_eq!(verdict.intent_id, intent_id);
        assert_eq!(verdict.outcome, VerdictOutcome::Approved);
        assert!(verdict.final_command.is_some());
        assert!(verdict.reason.is_empty());
    }

    /// REGRESSION: dial at Coach must NEVER actuate (docs/09). An earlier
    /// implementation noted "treat Coach as suggest only" in a comment but
    /// fell through to approval — dial 1 behaved like dial 3. This test is
    /// the tripwire.
    #[test]
    fn test_coach_dial_never_actuates() {
        let mut envelope = SafetyEnvelope::new(make_test_limits());
        let mut state = make_test_state();
        state.human.dial = AutonomyLevel::Coach;

        let intent = Intent {
            requested_rudder_deg: Some(5.0),
            requested_throttle_pct: Some(25.0),
            horizon_s: 1.0,
        };

        let arb = envelope.arbitrate(&intent, &state);
        assert!(matches!(arb, Arbitration::Rejected(_)));
        if let Arbitration::Rejected(reason) = arb {
            assert!(reason.contains("Coach"));
        }
    }

    /// REGRESSION: a clamped command must still pass rate limits and
    /// context guards. Clamping bounds magnitude only.
    #[test]
    fn test_clamped_command_still_checked() {
        let mut envelope = SafetyEnvelope::new(make_test_limits());
        let mut state = make_test_state();
        state.nav.sog_kn = Some(1.0); // below min autopilot speed

        // Rudder 25° exceeds hard bound (clamped to 15°) AND the vessel is
        // too slow for steering — the clamped command must still be rejected
        // by the context guard, not approved as Clamped.
        let intent = Intent {
            requested_rudder_deg: Some(25.0),
            requested_throttle_pct: None,
            horizon_s: 1.0,
        };

        let arb = envelope.arbitrate(&intent, &state);
        assert!(matches!(arb, Arbitration::Rejected(_)));
    }
}
