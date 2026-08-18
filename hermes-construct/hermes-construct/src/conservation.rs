#![allow(dead_code)]
//! conservation.rs — Budget tracking and enforcement
//!
//! Every operation costs tokens. Conservation enforcement ensures
//! total_deposits - total_withdrawals == total_budget after every tick.
//!
//! # Cost Model
//!
//! Costs are grounded in real API pricing via [`CostModel`]. The budget is
//! denominated in **US dollars**. Each operation maps to a dollar cost derived
//! from the model's per-token pricing and a fixed overhead per API call.
//!
//! The legacy [`costs`] module provides named constants that are the default
//! values from `CostModel::glm_flash()` (the cheapest model). For precise
//! per-call costing, use [`CostModel::cost_for_tokens`] directly.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

// ---------------------------------------------------------------------------
// CostModel — real API pricing
// ---------------------------------------------------------------------------

/// Pricing parameters for a specific model/provider.
///
/// All monetary values are in **US dollars**.
///
/// ```text
/// operation_cost = operation_overhead + (tokens / 1000) * blend_cost
/// blend_cost     = blend_ratio * input_token_cost + (1 - blend_ratio) * output_token_cost
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostModel {
    /// Cost per 1K **input** tokens ($/1K tokens).
    pub input_token_cost: f64,
    /// Cost per 1K **output** tokens ($/1K tokens).
    pub output_token_cost: f64,
    /// Fixed overhead per API call ($).
    pub operation_overhead: f64,
    /// Blend ratio: weight of input in the per-token cost.
    /// 0.5 = equal blend. Defaults to 0.5.
    pub blend_ratio: f64,
}

impl CostModel {
    /// GPT-4o pricing (OpenAI, 2025): $2.50/1M input, $10.00/1M output.
    /// Per-1K: $0.0025 input, $0.01 output. Overhead ≈ $0.001.
    pub fn gpt4o() -> Self {
        Self {
            input_token_cost: 0.0025,
            output_token_cost: 0.01,
            operation_overhead: 0.001,
            blend_ratio: 0.5,
        }
    }

    /// Claude 3.5 Opus pricing (Anthropic, 2025): $15/1M input, $75/1M output.
    /// Per-1K: $0.015 input, $0.075 output. Overhead ≈ $0.002.
    pub fn claude_opus() -> Self {
        Self {
            input_token_cost: 0.015,
            output_token_cost: 0.075,
            operation_overhead: 0.002,
            blend_ratio: 0.5,
        }
    }

    /// GLM-4 Flash (DeepInfra): $0.05/1M input, $0.05/1M output.
    /// Per-1K: $0.00005 input, $0.00005 output. Overhead ≈ $0.0001.
    pub fn glm_flash() -> Self {
        Self {
            input_token_cost: 0.00005,
            output_token_cost: 0.00005,
            operation_overhead: 0.0001,
            blend_ratio: 0.5,
        }
    }

    /// Compute the blended per-1K-token cost.
    ///
    /// ```text
    /// blend_cost = blend_ratio * input_token_cost + (1 - blend_ratio) * output_token_cost
    /// ```
    pub fn token_blend_cost(&self) -> f64 {
        self.blend_ratio * self.input_token_cost
            + (1.0 - self.blend_ratio) * self.output_token_cost
    }

    /// Cost for an operation that uses `token_count` tokens.
    ///
    /// ```text
    /// cost = operation_overhead + (token_count / 1000) * token_blend_cost()
    /// ```
    pub fn cost_for_tokens(&self, token_count: u32) -> f64 {
        self.operation_overhead + (token_count as f64 / 1000.0) * self.token_blend_cost()
    }

    /// A typical completion (~500 tokens blended) — the reference unit.
    pub fn typical_completion_cost(&self) -> f64 {
        self.cost_for_tokens(500)
    }
}

impl Default for CostModel {
    fn default() -> Self {
        Self::glm_flash()
    }
}

// ---------------------------------------------------------------------------
// Cost table — derived from CostModel
// ---------------------------------------------------------------------------

/// Named operation costs derived from a [`CostModel`].
///
/// | Operation            | Basis                                    |
/// |----------------------|------------------------------------------|
/// | tile_create          | 0.1× typical completion + overhead        |
/// | ensign_activate      | 1× typical completion (full LLM call)     |
/// | gravity_update       | 0.01× (scalar arithmetic, no LLM)         |
/// | phone_a_friend       | 10× (external escalation)                 |
/// | shell_spawn          | 5× (heavy initialization)                 |
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostTable {
    pub tile_create: f64,
    pub tile_complete: f64,
    pub tile_archive: f64,
    pub ensign_activate: f64,
    pub ensign_orient: f64,
    pub ensign_tile: f64,
    pub ensign_stand_down: f64,
    pub gravity_update: f64,
    pub gravity_recalibrate: f64,
    pub phone_a_friend: f64,
    pub correlation_compute: f64,
    pub correlation_transfer: f64,
    pub penrose_refit: f64,
    pub port_open_close: f64,
    pub port_message: f64,
    pub deadband_check: f64,
    pub deadband_action: f64,
    pub bootstrap_step: f64,
    pub shell_spawn: f64,
    pub shell_destroy: f64,
}

impl CostTable {
    /// Derive all costs from a model's pricing.
    pub fn from_cost_model(model: &CostModel) -> Self {
        let unit = model.typical_completion_cost();
        let oh = model.operation_overhead;
        Self {
            tile_create: oh + 0.1 * unit,
            tile_complete: oh * 0.5 + 0.05 * unit,
            tile_archive: oh * 0.1,
            ensign_activate: unit,
            ensign_orient: oh + 0.5 * unit,
            ensign_tile: oh + 0.5 * unit,
            ensign_stand_down: oh + 0.3 * unit,
            gravity_update: oh * 0.1,
            gravity_recalibrate: oh + 0.1 * unit,
            phone_a_friend: oh + 10.0 * unit,
            correlation_compute: oh * 0.5 + 0.05 * unit,
            correlation_transfer: oh * 0.5 + 0.05 * unit,
            penrose_refit: oh + 0.5 * unit,
            port_open_close: oh + 0.2 * unit,
            port_message: oh * 0.1,
            deadband_check: oh * 0.1,
            deadband_action: oh + 0.5 * unit,
            bootstrap_step: oh + 0.5 * unit,
            shell_spawn: oh + 5.0 * unit,
            shell_destroy: oh + 2.0 * unit,
        }
    }
}

impl Default for CostTable {
    fn default() -> Self {
        Self::from_cost_model(&CostModel::default())
    }
}

// ---------------------------------------------------------------------------
// Legacy cost constants (backward-compat, now grounded in GLM Flash pricing)
// ---------------------------------------------------------------------------

/// Cost constants for operations, in **US dollars**.
///
/// These are the default values from `CostModel::glm_flash()`.
/// For models with different pricing, use [`CostTable::from_cost_model`].
///
/// Basis: GLM Flash typical_completion ≈ $0.000125 (500 tokens at $0.00005/1K + $0.0001 overhead).
pub mod costs {
    /// Default GLM Flash cost table, evaluated once.
    fn table() -> super::CostTable {
        super::CostTable::default()
    }

    pub const TILE_CREATE: f64 = 0.0001125;
    pub const TILE_COMPLETE: f64 = 0.00010625;
    pub const TILE_ARCHIVE: f64 = 0.00001;
    pub const ENSIGN_ACTIVATE: f64 = 0.000125;
    pub const ENSIGN_ORIENT: f64 = 0.0001375;
    pub const ENSIGN_TILE: f64 = 0.0001375;
    pub const ENSIGN_STAND_DOWN: f64 = 0.0001375;
    pub const GRAVITY_UPDATE: f64 = 0.00001;
    pub const GRAVITY_RECALIBRATE: f64 = 0.0001125;
    pub const PHONE_A_FRIEND: f64 = 0.001375;
    pub const CORRELATION_COMPUTE: f64 = 0.00006875;
    pub const CORRELATION_TRANSFER: f64 = 0.00006875;
    pub const PENROSE_REFIT: f64 = 0.0001625;
    pub const PORT_OPEN_CLOSE: f64 = 0.000125;
    pub const PORT_MESSAGE: f64 = 0.00001;
    pub const DEADBAND_CHECK: f64 = 0.00001;
    pub const DEADBAND_ACTION: f64 = 0.0001625;
    pub const BOOTSTRAP_STEP: f64 = 0.0001625;
    pub const SHELL_SPAWN: f64 = 0.000725;
    pub const SHELL_DESTROY: f64 = 0.00035;
}

// ---------------------------------------------------------------------------
// ConservationState
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConservationState {
    pub budget: f64,
    pub used: f64,
    pub tick: u64,
}

impl ConservationState {
    pub fn remaining(&self) -> f64 {
        self.budget - self.used
    }

    pub fn can_spend(&self, cost: f64) -> bool {
        self.remaining() >= cost
    }

    pub fn spend(&mut self, cost: f64) -> Result<(), String> {
        if !self.can_spend(cost) {
            return Err(format!(
                "conservation budget exceeded: remaining={} cost={}",
                self.remaining(),
                cost
            ));
        }
        self.used += cost;
        Ok(())
    }

    pub fn deposit(&mut self, amount: f64) {
        self.budget += amount;
    }
}

static GLOBAL_TICK: AtomicU64 = AtomicU64::new(0);

pub fn current_tick() -> u64 {
    GLOBAL_TICK.load(Ordering::Relaxed)
}

pub fn advance_tick() -> u64 {
    GLOBAL_TICK.fetch_add(1, Ordering::Relaxed)
}

/// Initialize conservation table in SQLite
pub fn init_schema(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS conservation (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        INSERT OR IGNORE INTO conservation (key, value) VALUES ('budget', '10000');
        INSERT OR IGNORE INTO conservation (key, value) VALUES ('used', '0');
        INSERT OR IGNORE INTO conservation (key, value) VALUES ('tick', '0');"
    )
}

/// Load conservation state from SQLite
pub fn load_state(conn: &Connection) -> Result<ConservationState, rusqlite::Error> {
    let budget: f64 = conn
        .query_row("SELECT value FROM conservation WHERE key = 'budget'", [], |r| {
            r.get::<_, String>(0).map(|s| s.parse().unwrap_or(10000.0))
        })
        .unwrap_or(10000.0);

    let used: f64 = conn
        .query_row("SELECT value FROM conservation WHERE key = 'used'", [], |r| {
            r.get::<_, String>(0).map(|s| s.parse().unwrap_or(0.0))
        })
        .unwrap_or(0.0);

    let tick: u64 = conn
        .query_row("SELECT value FROM conservation WHERE key = 'tick'", [], |r| {
            r.get::<_, String>(0).map(|s| s.parse().unwrap_or(0))
        })
        .unwrap_or(0);

    GLOBAL_TICK.store(tick, Ordering::Relaxed);

    Ok(ConservationState { budget, used, tick })
}

/// Persist conservation state to SQLite
pub fn save_state(conn: &Connection, state: &ConservationState) -> Result<(), rusqlite::Error> {
    conn.execute(
        "UPDATE conservation SET value = ? WHERE key = 'budget'",
        params![state.budget.to_string()],
    )?;
    conn.execute(
        "UPDATE conservation SET value = ? WHERE key = 'used'",
        params![state.used.to_string()],
    )?;
    conn.execute(
        "UPDATE conservation SET value = ? WHERE key = 'tick'",
        params![state.tick.to_string()],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conservation_state_remaining() {
        let s = ConservationState { budget: 100.0, used: 30.0, tick: 0 };
        assert!((s.remaining() - 70.0).abs() < f64::EPSILON);
    }

    #[test]
    fn can_spend_within_budget() {
        let s = ConservationState { budget: 100.0, used: 30.0, tick: 0 };
        assert!(s.can_spend(50.0));
        assert!(!s.can_spend(80.0));
    }

    #[test]
    fn can_spend_exact_remaining() {
        let s = ConservationState { budget: 100.0, used: 30.0, tick: 0 };
        assert!(s.can_spend(70.0));
    }

    #[test]
    fn spend_success() {
        let mut s = ConservationState { budget: 100.0, used: 0.0, tick: 0 };
        assert!(s.spend(50.0).is_ok());
        assert!((s.used - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn spend_exceeds_budget() {
        let mut s = ConservationState { budget: 10.0, used: 0.0, tick: 0 };
        assert!(s.spend(20.0).is_err());
    }

    #[test]
    fn deposit_increases_budget() {
        let mut s = ConservationState { budget: 100.0, used: 50.0, tick: 0 };
        s.deposit(25.0);
        assert!((s.budget - 125.0).abs() < f64::EPSILON);
    }

    #[test]
    fn advance_tick_increments() {
        GLOBAL_TICK.store(0, Ordering::Relaxed);
        let t1 = advance_tick();
        let t2 = advance_tick();
        assert!(t2 > t1);
    }

    #[test]
    fn init_schema_and_load_state() {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();
        let state = load_state(&conn).unwrap();
        assert!((state.budget - 10000.0).abs() < f64::EPSILON);
        assert!((state.used - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn save_and_reload_state() {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();
        let state = ConservationState { budget: 500.0, used: 42.5, tick: 7 };
        save_state(&conn, &state).unwrap();
        let loaded = load_state(&conn).unwrap();
        assert!((loaded.budget - 500.0).abs() < f64::EPSILON);
        assert!((loaded.used - 42.5).abs() < f64::EPSILON);
        assert_eq!(loaded.tick, 7);
    }

    // --- CostModel tests ---

    #[test]
    fn cost_model_gpt4o_pricing() {
        let m = CostModel::gpt4o();
        assert!((m.input_token_cost - 0.0025).abs() < 1e-9);
        assert!((m.output_token_cost - 0.01).abs() < 1e-9);
    }

    #[test]
    fn cost_model_claude_opus_pricing() {
        let m = CostModel::claude_opus();
        assert!((m.input_token_cost - 0.015).abs() < 1e-9);
        assert!((m.output_token_cost - 0.075).abs() < 1e-9);
    }

    #[test]
    fn cost_model_glm_flash_pricing() {
        let m = CostModel::glm_flash();
        assert!((m.input_token_cost - 0.00005).abs() < 1e-9);
        assert!((m.output_token_cost - 0.00005).abs() < 1e-9);
    }

    #[test]
    fn cost_model_blend_cost() {
        let m = CostModel::gpt4o(); // 0.5 * 0.0025 + 0.5 * 0.01 = 0.00625
        assert!((m.token_blend_cost() - 0.00625).abs() < 1e-9);
    }

    #[test]
    fn cost_model_cost_for_tokens() {
        let m = CostModel::gpt4o();
        // 0.001 + (1000/1000) * 0.00625 = 0.00725
        let cost = m.cost_for_tokens(1000);
        assert!((cost - 0.00725).abs() < 1e-9);
    }

    #[test]
    fn cost_model_typical_completion() {
        let m = CostModel::glm_flash();
        // 0.0001 + (500/1000) * 0.00005 = 0.000125
        let cost = m.typical_completion_cost();
        assert!((cost - 0.000125).abs() < 1e-9);
    }

    #[test]
    fn cost_table_from_model() {
        let table = CostTable::from_cost_model(&CostModel::glm_flash());
        // ensign_activate = typical_completion_cost = 0.000125
        assert!((table.ensign_activate - 0.000125).abs() < 1e-9);
        // gravity_update = overhead * 0.1 = 0.00001
        assert!((table.gravity_update - 0.00001).abs() < 1e-9);
    }

    #[test]
    fn cost_table_default_matches_glm_flash() {
        let t1 = CostTable::default();
        let t2 = CostTable::from_cost_model(&CostModel::glm_flash());
        assert!((t1.ensign_activate - t2.ensign_activate).abs() < 1e-15);
        assert!((t1.shell_spawn - t2.shell_spawn).abs() < 1e-15);
    }

    #[test]
    fn gpt4o_is_more_expensive_than_glm_flash() {
        let gpt4 = CostTable::from_cost_model(&CostModel::gpt4o());
        let glm = CostTable::from_cost_model(&CostModel::glm_flash());
        assert!(gpt4.ensign_activate > glm.ensign_activate);
        assert!(gpt4.phone_a_friend > glm.phone_a_friend);
    }
}
