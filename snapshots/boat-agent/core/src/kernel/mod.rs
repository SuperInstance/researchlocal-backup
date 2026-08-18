//! The kernel: tick, schedule, supervise, record (docs/05 §The kernel).
//!
//! ## Architecture
//!
//! The kernel is the heart of the vessel intelligence system. It's deliberately
//! boring — contains NO policy about trolling, fishing, or navigation. It knows
//! only what "safe" means, not what "optimal" means.
//!
//! ## Responsibilities
//!
//! 1. **Tick Scheduling**: Run the control loop at 10Hz (100ms intervals)
//! 2. **Event Routing**: Drain and dispatch events from the bus lanes
//! 3. **State Management**: Maintain the authoritative digital twin
//! 4. **Control Arbitration**: Pass intents through the safety envelope
//! 5. **Black Box Recording**: Log every actuation tick with full context
//! 6. **Supervision**: Monitor and restart child processes (drivers, sidecars)
//! 7. **Critical Preemption**: Handle jog lever and watchdog events immediately
//!
//! ## Tick Sequence
//!
//! One tick (100ms) performs:
//!
//! 0. Drain CRITICAL lane → apply immediately (jog lever, watchdog)
//! 1. Drain telemetry (coalesced) → state.reduce()
//! 2. Get mission targets (L3) — from memory/missions config
//! 3. Playbooks.evaluate(snapshot, targets) — non-blocking with timeout
//! 4. Envelope.arbitrate(intent, state) — the one door
//! 5. Actuator flush via envelope-owned drivers
//! 6. Blackbox.append(snapshot_hash, intent, verdict, dial, actor)
//! 7. Drain critical again before sleeping to next tick
//!
//! Between ANY two steps, critical events preempt.
//!
//! ## Design Philosophy
//!
//! - **Deterministic**: Same inputs → same outputs (replayable)
//! - **Transparent**: Everything goes through the bus and black box
//! - **Fail-Safe**: Watchdog timeout → safe state, buzzer sounds
//! - **Human-Centered**: Jog lever = absolute veto, always respected

use crate::blackbox::{BlackBox, BlackBoxEntry};
use crate::bus::{Event, LaneRouter};
use crate::bus::events::{Intent, Verdict, AutonomyLevel, VerdictOutcome};
use crate::config::VesselProfile;
use crate::envelope::SafetyEnvelope;
use crate::state::{VesselState, StateSnapshot};
use anyhow::Result;
use std::time::{Duration, Instant};
use std::path::PathBuf;
use tracing::{info, warn, error, debug};

/// Default control rate. Playbooks evaluate per-tick, non-blocking.
const TICK_HZ: u32 = 10;
const TICK_INTERVAL_MS: u64 = 1000 / TICK_HZ as u64;

/// Maximum time for playbook evaluation before timeout.
///
/// Playbooks that take too long are aborted to maintain the 10Hz tick rate.
const PLAYBOOK_TIMEOUT_MS: u64 = 80;

/// Configuration for the kernel boot process.
pub struct KernelConfig {
    /// Vessel profile from vessel.toml
    pub profile: VesselProfile,
    /// Data directory for logs and state
    pub data_dir: PathBuf,
}

/// The kernel — heart of the vessel intelligence system.
///
/// This struct owns all core subsystems and orchestrates their interaction.
pub struct Kernel {
    /// Vessel profile (limits, drivers, agents, missions)
    profile: VesselProfile,
    /// Event bus router
    router: LaneRouter,
    /// Authoritative digital twin
    state: VesselState,
    /// Safety envelope — owns all actuation
    envelope: SafetyEnvelope,
    /// Black box recorder
    blackbox: BlackBox,
    /// Tick counter for diagnostics
    tick_count: u64,
    /// Last tick time for drift tracking
    last_tick_time: Instant,
    /// Whether the kernel is running
    is_running: bool,
    /// Data directory path
    data_dir: PathBuf,
}

impl Kernel {
    /// Bootstrap the kernel with all subsystems.
    ///
    /// Boot order matters:
    ///   1. Envelope FIRST (safe state asserted before anything moves)
    ///   2. Black box + state
    ///   3. Router (drivers begin emitting)
    ///   4. Ready to start main loop
    ///
    /// Dial ALWAYS restarts at Coach (docs/09): autonomy is re-earned each
    /// power-up, never silently resumed.
    pub fn bootstrap(config: KernelConfig) -> Result<Self> {
        info!("kernel: bootstrapping vessel intelligence system");

        // Step 1: Create envelope FIRST (safe state before anything else)
        let envelope = SafetyEnvelope::new(config.profile.envelope.clone());
        info!("kernel: safety envelope armed");

        // Step 2: Initialize black box and state
        let blackbox_path = config.data_dir.join("blackbox.log");
        let mut blackbox = BlackBox::open(blackbox_path)?;
        info!("kernel: black box opened at {}", blackbox.path().display());

        let state = VesselState::genesis();
        info!("kernel: genesis state created (dial=Coach, tick=0)");

        // Step 3: Initialize router
        let mut router = LaneRouter::new(&config.data_dir);
        router.load_capabilities(&config.profile.agents);
        info!("kernel: event router initialized");

        // Step 4: Record bootstrap to black box
        let bootstrap_entry = BlackBoxEntry {
            timestamp_epoch_ms: now_ms(),
            tick: 0,
            state_hash: state.state_hash.clone(),
            dial_level: state.human.dial as u8,
            actor: "kernel".to_string(),
            intent_json: None,
            verdict_json: None,
            human_override_detected: false,
        };
        blackbox.append(&bootstrap_entry)?;
        info!("kernel: bootstrap recorded to black box");

        Ok(Self {
            profile: config.profile,
            router,
            state,
            envelope,
            blackbox,
            tick_count: 0,
            last_tick_time: Instant::now(),
            is_running: false,
            data_dir: config.data_dir,
        })
    }

    /// Start the kernel main loop.
    ///
    /// This runs until `shutdown()` is called or a fatal error occurs.
    pub async fn run(&mut self) -> Result<()> {
        info!("kernel: starting main loop at {}Hz", TICK_HZ);
        self.is_running = true;
        self.last_tick_time = Instant::now();

        while self.is_running {
            // Measure tick start for drift tracking
            let tick_start = Instant::now();

            // Run one tick
            if let Err(e) = self.run_one_tick().await {
                error!("kernel: tick {} failed: {}", self.tick_count, e);

                // Record failure to black box
                self.record_tick_failure(&e.to_string()).await;

                // Continue running — don't let one tick failure stop the system
                // The envelope will reject unsafe commands if state is degraded
            }

            // Update tick counter
            self.tick_count += 1;

            // Check watchdog — if tick took too long, we're in trouble
            let tick_duration = tick_start.elapsed();
            if tick_duration > Duration::from_millis(TICK_INTERVAL_MS) {
                warn!("kernel: tick {} took {}ms (exceeds {}ms budget)",
                    self.tick_count,
                    tick_duration.as_millis(),
                    TICK_INTERVAL_MS
                );

                // Record missed heartbeat
                self.envelope.record_missed_heartbeat();
            }

            // Sleep to next tick boundary (account for drift)
            let drift = tick_duration.as_millis() as i64 - TICK_INTERVAL_MS as i64;
            let sleep_time = if drift < 0 {
                Duration::from_millis(TICK_INTERVAL_MS)
            } else {
                Duration::from_millis(0) // Already late, don't sleep
            };

            if sleep_time > Duration::ZERO {
                tokio::time::sleep(sleep_time).await;
            }

            // Track drift for diagnostics
            let actual_interval = tick_start.elapsed();
            debug!("kernel: tick {} completed in {}ms",
                self.tick_count,
                actual_interval.as_millis()
            );
        }

        info!("kernel: main loop stopped (tick_count={})", self.tick_count);
        Ok(())
    }

    /// Run one tick of the control loop.
    ///
    /// This is the core tick sequence — everything that happens in 100ms.
    async fn run_one_tick(&mut self) -> Result<()> {
        // Step 0: Drain CRITICAL lane — immediate processing
        self.handle_critical_events();

        // Step 1: Drain telemetry — coalesced by producer
        let telemetry_events = self.router.drain_telemetry_coalesced();
        if !telemetry_events.is_empty() {
            self.state.reduce(&telemetry_events);
            debug!("kernel: reduced {} telemetry events", telemetry_events.len());
        }

        // Step 2: Get mission targets (L3) — for now, use default
        // TODO: Load from mission config based on current active mission
        let targets = self.get_mission_targets();

        // Step 3: Playbooks evaluate — produce intents
        // TODO: Integrate playbook host
        let intent = self.evaluate_playbooks(&targets).await;

        // Step 4: Envelope arbitration — the one door
        let verdict = if let Some(ref intent) = intent {
            let arbitration = self.envelope.arbitrate(intent, &self.state);
            // Intent carries no id (payload-only type); the kernel assigns
            // the correlation id when creating the verdict.
            Some(self.envelope.to_verdict(ulid::Ulid::new(), arbitration))
        } else {
            None
        };

        // Step 5: Actuator flush (if approved or clamped to a safe command)
        if let Some(ref verdict) = verdict {
            if matches!(verdict.outcome, VerdictOutcome::Approved | VerdictOutcome::Clamped) {
                if let Some(ref command) = verdict.final_command {
                    self.flush_actuators(command).await?;
                    // Feed the rate limiter — without this, envelope rate
                    // limits are dead code. Uses the STATE's clock (A2).
                    self.envelope.update_command_memory(
                        command.requested_rudder_deg,
                        command.requested_throttle_pct,
                        self.state.timestamp_ms,
                    );
                }
            }
        }

        // Step 6: Black box recording
        self.record_tick_to_blackbox(intent.as_ref(), verdict.as_ref()).await?;

        // Step 7: Envelope heartbeat
        self.envelope.heartbeat(self.state.tick);

        // Step 8: Drain critical again before sleep
        self.handle_critical_events();

        Ok(())
    }

    /// Handle critical events — immediate processing.
    ///
    /// These events preempt between any two kernel steps.
    fn handle_critical_events(&mut self) {
        let critical_events = self.router.drain_critical();

        for event in critical_events {
            match &event.kind {
                crate::bus::events::EventKind::JogLeverMove(_) => {
                    info!("kernel: jog lever moved — human override active");

                    // Override flag set in state
                    self.state.human.override_active = true;
                    self.state.human.last_override_ms = Some(now_ms());

                    // Drivers already preempted by hardware (docs/09 §Veto)
                    // Current playbook demoted to shadow

                    // TODO: Demote active playbook to shadow
                    warn!("kernel: human override — playbook demoted to shadow");
                }
                crate::bus::events::EventKind::DialSet(dial) => {
                    info!("kernel: autonomy dial set to {:?}", dial.level);

                    // Update state
                    self.state.human.dial = dial.level;
                }
                crate::bus::events::EventKind::EscalationAnswer(answer) => {
                    info!("kernel: escalation answered: option={}", answer.option_id);

                    // TODO: Handle escalation response
                    // This would update the active playbook or mission
                }
                crate::bus::events::EventKind::WatchdogTrip(trip) => {
                    error!("kernel: watchdog trip — {} missed heartbeats", trip.missed_heartbeats);

                    // Record degraded mode
                    self.state.degraded = Some(format!(
                        "watchdog trip: {} missed heartbeats",
                        trip.missed_heartbeats
                    ));

                    // Envelope will reject all intents until cleared
                }
                crate::bus::events::EventKind::DegradedMode(mode) => {
                    warn!("kernel: degraded mode: {}", mode.cause);

                    self.state.degraded = Some(mode.cause.clone());
                }
                _ => {
                    // Reduce any state-affecting events
                    self.state.reduce(&[event]);
                }
            }
        }
    }

    /// Get mission targets from active mission.
    ///
    /// L3 mission targets are what the vessel is trying to achieve:
    /// - Transit: target_track_deg = 147.0
    /// - Trolling: target_sog_kn = 2.3
    /// - Anchor Watch: position holding
    fn get_mission_targets(&self) -> serde_json::Value {
        // TODO: Load from mission config
        // For now, return empty (no active mission)
        serde_json::json!({})
    }

    /// Evaluate playbooks to produce intents.
    ///
    /// This calls the playbook host with the current snapshot and targets.
    /// Playbooks run with a timeout — if they exceed it, we hold the last
    /// safe command and count the miss.
    async fn evaluate_playbooks(&mut self, _targets: &serde_json::Value) -> Option<Intent> {
        // TODO: Integrate playbook host
        // For now, return None (no intent from playbooks)

        // When playbook host is integrated:
        // 1. Create StateSnapshot from current state
        // 2. Call playbook_host.evaluate(snapshot, targets)
        // 3. Apply timeout using tokio::time::timeout
        // 4. Return intent or None on timeout

        None
    }

    /// Flush approved commands to actuators.
    ///
    /// This is the ONLY place where actuator writes happen. The envelope
    /// owns all drivers — they live here in kernel but are controlled
    /// exclusively through the envelope.
    async fn flush_actuators(&mut self, command: &Intent) -> Result<()> {
        // TODO: Integrate actuator drivers
        // For now, just log

        if command.requested_rudder_deg.is_some() || command.requested_throttle_pct.is_some() {
            info!("kernel: flushing actuators: rudder={:?}, throttle={:?}",
                command.requested_rudder_deg,
                command.requested_throttle_pct
            );
        }

        // When drivers are integrated:
        // 1. Get driver references from envelope
        // 2. Send NMEA sentences or actuator commands
        // 3. Verify writes completed

        Ok(())
    }

    /// Record tick to black box.
    ///
    /// Every actuation tick records: state hash, intent, verdict, dial, actor.
    async fn record_tick_to_blackbox(
        &mut self,
        intent: Option<&Intent>,
        verdict: Option<&Verdict>,
    ) -> Result<()> {
        let entry = BlackBoxEntry {
            timestamp_epoch_ms: now_ms(),
            tick: self.state.tick,
            state_hash: self.state.state_hash.clone(),
            dial_level: self.state.human.dial as u8,
            // Intent payloads carry no provenance (that lives on bus Events).
            // Until the playbook host lands, actuating intents are kernel-sourced.
            actor: intent
                .map(|_| "playbook:pending-integration".to_string())
                .unwrap_or_else(|| "none".to_string()),
            intent_json: intent.map(|i| serde_json::to_string(i).unwrap_or_default()),
            verdict_json: verdict.map(|v| serde_json::to_string(v).unwrap_or_default()),
            human_override_detected: self.state.human.override_active,
        };

        self.blackbox.append(&entry)?;

        Ok(())
    }

    /// Record tick failure to black box.
    ///
    /// This ensures that even error conditions are logged for audit.
    async fn record_tick_failure(&mut self, error: &str) {
        let entry = BlackBoxEntry {
            timestamp_epoch_ms: now_ms(),
            tick: self.state.tick,
            state_hash: self.state.state_hash.clone(),
            dial_level: self.state.human.dial as u8,
            actor: "kernel".to_string(),
            intent_json: None,
            verdict_json: Some(serde_json::json!({
                "outcome": "tick_failed",
                "error": error
            }).to_string()),
            human_override_detected: false,
        };

        if let Err(e) = self.blackbox.append(&entry) {
            error!("kernel: failed to record tick failure: {}", e);
        }
    }

    /// Stop the kernel main loop.
    pub fn shutdown(&mut self) {
        info!("kernel: shutdown requested");
        self.is_running = false;
    }

    /// Get the current state snapshot.
    pub fn state_snapshot(&self) -> StateSnapshot {
        StateSnapshot::from(&self.state)
    }

    /// Get tick count for diagnostics.
    pub fn tick_count(&self) -> u64 {
        self.tick_count
    }

    /// Get effective autonomy level.
    pub fn effective_autonomy(&self) -> AutonomyLevel {
        self.envelope.effective_autonomy(&self.state)
    }

    /// Get consecutive rejection count.
    pub fn consecutive_rejections(&self) -> u32 {
        self.envelope.consecutive_rejections()
    }

    /// Get missed heartbeat count.
    pub fn missed_heartbeats(&self) -> u32 {
        self.envelope.missed_heartbeats()
    }
}

/// Emit an event onto the bus (used by modules holding a router handle).
///
/// This is a convenience function for modules that don't have direct
/// access to the router (e.g., drivers, agents).
pub fn emit(_router: &mut LaneRouter, _event: Event) {
    // TODO: Implement router.emit() method
    // This would be used by drivers and agents to emit events
    todo!("router.emit() implementation")
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
    use crate::config::{VesselInfo, EnvelopeLimits};
    use std::collections::HashMap;

    fn make_test_profile() -> VesselProfile {
        VesselProfile {
            vessel: VesselInfo {
                id: "test_vessel".to_string(),
                name: "Test Vessel".to_string(),
                vessel_type: "troller".to_string(),
                data_dir: "./data".to_string(),
            },
            envelope: EnvelopeLimits {
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
            },
            drivers: HashMap::new(),
            agents: HashMap::new(),
            missions: HashMap::new(),
            cloud: None,
        }
    }

    /// Bootstrap into a temp dir — never touch fixed paths in tests.
    fn boot_test_kernel() -> (tempfile::TempDir, Kernel) {
        let dir = tempfile::TempDir::new().unwrap();
        let config = KernelConfig {
            profile: make_test_profile(),
            data_dir: dir.path().to_path_buf(),
        };
        let kernel = Kernel::bootstrap(config).unwrap();
        (dir, kernel)
    }

    #[test]
    fn test_kernel_bootstrap() {
        let (_dir, kernel) = boot_test_kernel();

        assert_eq!(kernel.tick_count(), 0);
        assert_eq!(kernel.state.tick, 0);
        assert_eq!(kernel.state.human.dial, AutonomyLevel::Coach);
        assert!(!kernel.is_running);
    }

    #[test]
    fn test_effective_autonomy() {
        let (_dir, kernel) = boot_test_kernel();

        // At startup, should be Coach (default)
        assert_eq!(kernel.effective_autonomy(), AutonomyLevel::Coach);
    }

    #[test]
    fn test_shutdown() {
        let (_dir, mut kernel) = boot_test_kernel();
        kernel.shutdown();

        assert!(!kernel.is_running);
    }
}
