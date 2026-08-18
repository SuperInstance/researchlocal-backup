#![allow(dead_code)]
//! deadband.rs — Deadband monitoring and trend detection
//!
//! A deadband monitors a value within [lower, upper] bounds.
//! Trends: stable, drifting, oscillating, diverging.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Trend {
    Stable,
    Drifting,
    Oscillating,
    Diverging,
}

impl Trend {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Stable => "stable",
            Self::Drifting => "drifting",
            Self::Oscillating => "oscillating",
            Self::Diverging => "diverging",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "stable" => Some(Self::Stable),
            "drifting" => Some(Self::Drifting),
            "oscillating" => Some(Self::Oscillating),
            "diverging" => Some(Self::Diverging),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadbandState {
    pub lower: f64,
    pub upper: f64,
    pub current: f64,
    pub trend: Trend,
    pub consecutive_breaches: u32,
}

impl DeadbandState {
    pub fn new(lower: f64, upper: f64, current: f64) -> Self {
        Self {
            lower,
            upper,
            current,
            trend: Trend::Stable,
            consecutive_breaches: 0,
        }
    }

    /// Check if current value is within the deadband
    pub fn is_in_band(&self) -> bool {
        self.current >= self.lower && self.current <= self.upper
    }

    /// Update the current value and detect trend
    pub fn update(&mut self, new_value: f64) -> &Trend {
        let delta = new_value - self.current;
        let old_in_band = self.is_in_band();
        self.current = new_value;

        if self.is_in_band() {
            self.consecutive_breaches = 0;
            self.trend = Trend::Stable;
        } else {
            self.consecutive_breaches += 1;

            // Detect trend
            if self.consecutive_breaches > 10 {
                self.trend = Trend::Diverging;
            } else if self.consecutive_breaches > 5 {
                self.trend = Trend::Oscillating;
            } else {
                self.trend = Trend::Drifting;
            }
        }

        if !old_in_band && !self.is_in_band() && delta.abs() > (self.upper - self.lower) {
            self.trend = Trend::Diverging;
        }

        &self.trend
    }
}

/// Detect trend from a series of values
pub fn detect_trend(values: &[f64]) -> Trend {
    if values.len() < 3 {
        return Trend::Stable;
    }

    let deltas: Vec<f64> = values.windows(2).map(|w| w[1] - w[0]).collect();

    // Check for oscillation: alternating signs
    let sign_changes: usize = deltas.windows(2)
        .filter(|w| w[0].signum() != w[1].signum())
        .count();

    if sign_changes as f64 / (deltas.len() as f64 - 1.0) > 0.6 {
        return Trend::Oscillating;
    }

    // Check for divergence: increasing magnitude
    let increasing_magnitude: usize = deltas.windows(2)
        .filter(|w| w[1].abs() > w[0].abs())
        .count();

    if increasing_magnitude as f64 / (deltas.len() as f64 - 1.0) > 0.7 {
        return Trend::Diverging;
    }

    // Check for drift: consistent direction
    let mean_delta: f64 = deltas.iter().sum::<f64>() / deltas.len() as f64;
    if mean_delta.abs() > 0.01 {
        return Trend::Drifting;
    }

    Trend::Stable
}

// ---------------------------------------------------------------------------
// SQLite
// ---------------------------------------------------------------------------

pub fn init_schema(conn: &Connection) -> Result<(), rusqlite::Error> {
    // Deadband state is stored as part of tile metadata
    // We store deadband circuit configurations here
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS deadband_circuits (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            room_id TEXT NOT NULL,
            monitored_quantity TEXT NOT NULL,
            setpoint REAL NOT NULL,
            tolerance REAL NOT NULL,
            action TEXT NOT NULL,
            ensign_id TEXT,
            check_interval INTEGER DEFAULT 30,
            automation_level INTEGER DEFAULT 1,
            last_value REAL,
            consecutive_breaches INTEGER DEFAULT 0,
            is_breached BOOLEAN DEFAULT 0,
            created_tick INTEGER NOT NULL,
            updated_tick INTEGER NOT NULL,
            FOREIGN KEY (room_id) REFERENCES rooms(id)
        );"
    )
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadbandCircuit {
    pub id: String,
    pub name: String,
    pub room_id: String,
    pub monitored_quantity: String,
    pub setpoint: f64,
    pub tolerance: f64,
    pub action: String,
    pub ensign_id: Option<String>,
    pub check_interval: u64,
    pub automation_level: u32,
    pub last_value: Option<f64>,
    pub consecutive_breaches: u32,
    pub is_breached: bool,
    pub created_tick: u64,
    pub updated_tick: u64,
}

impl DeadbandCircuit {
    /// Check if current value is within the relative deadband.
    ///
    /// Uses **relative** tolerance: `|current - setpoint| / |setpoint| < tolerance`.
    /// This correctly handles large setpoint values (e.g. setpoint=10000 with
    /// tolerance=0.05 means ±500, not ±0.05).
    ///
    /// Edge case: when setpoint is zero, falls back to absolute comparison
    /// (|current| < tolerance) to avoid division by zero.
    pub fn check(&mut self, current_value: f64) -> bool {
        self.last_value = Some(current_value);

        let in_band = if self.setpoint.abs() < f64::EPSILON {
            // When setpoint ≈ 0, use absolute tolerance
            current_value.abs() < self.tolerance
        } else {
            // Relative tolerance: |current - setpoint| / |setpoint| < tolerance
            ((current_value - self.setpoint).abs() / self.setpoint.abs()) < self.tolerance
        };

        if !in_band {
            self.consecutive_breaches += 1;
            self.is_breached = true;
        } else {
            self.consecutive_breaches = 0;
            self.is_breached = false;
        }

        !in_band
    }
}

/// Run deadband checks for all circuits
pub fn run_checks(conn: &Connection, _tick: u64) -> Result<Vec<DeadbandCircuit>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, name, room_id, monitored_quantity, setpoint, tolerance,
                action, ensign_id, check_interval, automation_level,
                last_value, consecutive_breaches, is_breached,
                created_tick, updated_tick
         FROM deadband_circuits"
    )?;

    let circuits: Vec<DeadbandCircuit> = stmt.query_map([], |row| {
        Ok(DeadbandCircuit {
            id: row.get(0)?,
            name: row.get(1)?,
            room_id: row.get(2)?,
            monitored_quantity: row.get(3)?,
            setpoint: row.get(4)?,
            tolerance: row.get(5)?,
            action: row.get(6)?,
            ensign_id: row.get(7)?,
            check_interval: row.get::<_, i64>(8)? as u64,
            automation_level: row.get::<_, i64>(9)? as u32,
            last_value: row.get(10)?,
            consecutive_breaches: row.get::<_, i64>(11)? as u32,
            is_breached: row.get::<_, bool>(12)?,
            created_tick: row.get::<_, i64>(13)? as u64,
            updated_tick: row.get::<_, i64>(14)? as u64,
        })
    })?.filter_map(|r| r.ok()).collect();

    Ok(circuits)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trend_roundtrip() {
        for t in &[Trend::Stable, Trend::Drifting, Trend::Oscillating, Trend::Diverging] {
            assert_eq!(Trend::from_str(t.as_str()), Some(t.clone()));
        }
    }

    #[test]
    fn trend_from_invalid() {
        assert_eq!(Trend::from_str("unknown"), None);
    }

    #[test]
    fn deadband_in_band() {
        let db = DeadbandState::new(0.0, 10.0, 5.0);
        assert!(db.is_in_band());
    }

    #[test]
    fn deadband_below_band() {
        let db = DeadbandState::new(0.0, 10.0, -1.0);
        assert!(!db.is_in_band());
    }

    #[test]
    fn deadband_at_boundary() {
        let db = DeadbandState::new(0.0, 10.0, 0.0);
        assert!(db.is_in_band());
        let db2 = DeadbandState::new(0.0, 10.0, 10.0);
        assert!(db2.is_in_band());
    }

    #[test]
    fn update_stays_in_band_is_stable() {
        let mut db = DeadbandState::new(0.0, 10.0, 5.0);
        let trend = db.update(6.0);
        assert_eq!(*trend, Trend::Stable);
        assert_eq!(db.consecutive_breaches, 0);
    }

    #[test]
    fn update_drifts_then_oscillates() {
        let mut db = DeadbandState::new(0.0, 10.0, 5.0);
        for _ in 0..3 { db.update(15.0); }
        assert_eq!(db.trend, Trend::Drifting);
        for _ in 0..4 { db.update(15.0); }
        assert_eq!(db.trend, Trend::Oscillating);
    }

    #[test]
    fn update_diverges_after_many_breaches() {
        let mut db = DeadbandState::new(0.0, 10.0, 5.0);
        for _ in 0..12 { db.update(15.0); }
        assert_eq!(db.trend, Trend::Diverging);
    }

    #[test]
    fn detect_trend_oscillating() {
        let vals = vec![1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0];
        assert_eq!(detect_trend(&vals), Trend::Oscillating);
    }

    #[test]
    fn detect_trend_stable() {
        let vals = vec![5.0, 5.0, 5.0, 5.0, 5.0];
        assert_eq!(detect_trend(&vals), Trend::Stable);
    }

    #[test]
    fn detect_trend_short_series() {
        assert_eq!(detect_trend(&[1.0]), Trend::Stable);
        assert_eq!(detect_trend(&[]), Trend::Stable);
    }

    #[test]
    fn circuit_check_in_band() {
        let mut c = DeadbandCircuit {
            id: "c1".into(), name: "test".into(), room_id: "r1".into(),
            monitored_quantity: "temp".into(), setpoint: 50.0, tolerance: 0.1,
            action: "alert".into(), ensign_id: None, check_interval: 30,
            automation_level: 1, last_value: None, consecutive_breaches: 0,
            is_breached: false, created_tick: 0, updated_tick: 0,
        };
        // 48.0 is within 10% of 50.0 (|48-50|/|50| = 0.04 < 0.1)
        assert!(!c.check(48.0)); // in band, not breached
        assert!(!c.is_breached);
    }

    #[test]
    fn circuit_check_out_of_band() {
        let mut c = DeadbandCircuit {
            id: "c1".into(), name: "test".into(), room_id: "r1".into(),
            monitored_quantity: "temp".into(), setpoint: 50.0, tolerance: 0.1,
            action: "alert".into(), ensign_id: None, check_interval: 30,
            automation_level: 1, last_value: None, consecutive_breaches: 0,
            is_breached: false, created_tick: 0, updated_tick: 0,
        };
        // 60.0 is outside 10% of 50.0 (|60-50|/|50| = 0.2 > 0.1)
        assert!(c.check(60.0)); // breach
        assert!(c.is_breached);
        assert_eq!(c.consecutive_breaches, 1);
    }

    #[test]
    fn circuit_check_relative_tolerance_large_values() {
        let mut c = DeadbandCircuit {
            id: "c1".into(), name: "test".into(), room_id: "r1".into(),
            monitored_quantity: "budget".into(), setpoint: 10000.0, tolerance: 0.05,
            action: "alert".into(), ensign_id: None, check_interval: 30,
            automation_level: 1, last_value: None, consecutive_breaches: 0,
            is_breached: false, created_tick: 0, updated_tick: 0,
        };
        // 9600 is within 5% of 10000 (|9600-10000|/10000 = 0.04 < 0.05)
        assert!(!c.check(9600.0));
        // 9000 is outside 5% (|9000-10000|/10000 = 0.10 > 0.05)
        assert!(c.check(9000.0));
    }

    #[test]
    fn circuit_check_zero_setpoint_uses_absolute() {
        let mut c = DeadbandCircuit {
            id: "c1".into(), name: "test".into(), room_id: "r1".into(),
            monitored_quantity: "error".into(), setpoint: 0.0, tolerance: 0.5,
            action: "alert".into(), ensign_id: None, check_interval: 30,
            automation_level: 1, last_value: None, consecutive_breaches: 0,
            is_breached: false, created_tick: 0, updated_tick: 0,
        };
        // |0.3| < 0.5 → in band
        assert!(!c.check(0.3));
        // |0.6| >= 0.5 → out of band
        assert!(c.check(0.6));
    }
}
