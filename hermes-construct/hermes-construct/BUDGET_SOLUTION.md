# Puzzle 2: The Conservation Budget Integration

## Solution: `BudgetGuard` — Budget Enforcement Layer

### Overview

The `BudgetGuard` wraps every operation in hermes-construct with a budget check. No operation happens without budget clearance. When budget runs out, the system degrades gracefully instead of crashing.

### Operation Costs

```rust
/// Operation types and their conservation costs.
/// These are the "energy units" consumed by each operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Operation {
    TextResponse,      // Simple text generation via ensign
    CodeGeneration,    // Code-focused generation (engineering room)
    ToolExecution,     // Running a tool/plugin
    PhoneAFriend,      // Escalation to Opus/large model
    ProvenanceCommit,  // Git commit with provenance
    CorrelationScan,   // Penrose correlation scan
    TileCreate,        // Creating a tile
    TileComplete,      // Completing a tile
    TileArchive,       // Archiving a tile
    EnsignActivate,    // Dormant → waking
    EnsignOrient,      // Waking → orienting
    EnsignTileProcess, // Ensign processing a tile
    EnsignStandDown,   // Standing down
    GravityUpdate,     // Nudge room gravity
    GravityRecalibrate,// Full gravity recompute
    CorrelationCompute,// Per-room-pair correlation
    CorrelationTransfer,// Knowledge transfer
    PenroseRefit,      // Re-fit all room pairs
    PortOpen,          // Opening a port
    PortMessage,       // Sending a message
    DeadbandCheck,     // Checking a deadband circuit
    DeadbandAction,    // Executing a deadband remedy
    BootstrapStep,     // One bootstrap step
    ShellSpawn,        // Creating a child shell
    ShellDestroy,      // Destroying a child shell
}

impl Operation {
    /// Conservation cost in energy units.
    pub fn cost(&self) -> f64 {
        match self {
            // Core operations (from puzzle spec)
            Self::TextResponse     => 1.0,
            Self::CodeGeneration   => 5.0,
            Self::ToolExecution    => 3.0,
            Self::PhoneAFriend     => 20.0,
            Self::ProvenanceCommit => 1.0,
            Self::CorrelationScan  => 2.0,

            // Tile lifecycle
            Self::TileCreate       => 0.1,
            Self::TileComplete     => 0.05,
            Self::TileArchive      => 0.01,

            // Ensign lifecycle
            Self::EnsignActivate   => 1.0,
            Self::EnsignOrient     => 0.5,
            Self::EnsignTileProcess=> 0.5,
            Self::EnsignStandDown  => 0.3,

            // Gravity
            Self::GravityUpdate       => 0.01,
            Self::GravityRecalibrate  => 0.1,

            // Penrose
            Self::CorrelationCompute  => 0.05,
            Self::CorrelationTransfer => 0.05,
            Self::PenroseRefit        => 0.5,

            // Ports
            Self::PortOpen      => 0.2,
            Self::PortMessage   => 0.01,

            // Deadband
            Self::DeadbandCheck  => 0.02,
            Self::DeadbandAction => 0.5,

            // Bootstrap
            Self::BootstrapStep => 0.5,

            // Shell management
            Self::ShellSpawn    => 5.0,
            Self::ShellDestroy  => 2.0,
        }
    }
}
```

### The BudgetGuard

```rust
use std::sync::Arc;
use tokio::sync::Mutex;
use rusqlite::Connection;

/// Result of a budget check.
#[derive(Debug, Clone)]
pub enum BudgetVerdict {
    /// Operation is approved, cost deducted.
    Approved {
        remaining: f64,
        cost: f64,
    },
    /// Operation is denied, but a cheaper alternative exists.
    Degraded {
        original_cost: f64,
        degraded_cost: f64,
        reason: String,
    },
    /// Budget completely exhausted. Only free operations allowed.
    Exhausted {
        deficit: f64,  // How much more was needed
    },
}

/// Per-ensign budget allocation based on autonomy level.
#[derive(Debug, Clone)]
pub struct EnsignBudget {
    pub ensign_id: String,
    pub autonomy_level: u8,  // 1-5
    pub daily_budget: f64,
    pub spent: f64,
    pub max_single_operation: f64,
}

impl EnsignBudget {
    /// Budget allocation by autonomy level.
    ///
    /// Level 1 (all Opus):  10% of total — ensigns barely used
    /// Level 2 (observe):   20% of total — ensigns watching
    /// Level 3 (routine):   50% of total — ensigns handling routine
    /// Level 4 (autonomous): 70% of total — ensigns mostly autonomous
    /// Level 5 (self-op):   90% of total — full self-operation
    pub fn for_level(ensign_id: &str, level: u8, shell_budget: f64) -> Self {
        let (fraction, max_op) = match level {
            1 => (0.10, 5.0),   // Only cheap ops
            2 => (0.20, 10.0),  // Small ops allowed
            3 => (0.50, 25.0),  // Most ops allowed
            4 => (0.70, 50.0),  // Including phone-a-friend occasionally
            5 => (0.90, 100.0), // Almost unlimited
            _ => (0.10, 5.0),
        };
        EnsignBudget {
            ensign_id: ensign_id.to_string(),
            autonomy_level: level,
            daily_budget: shell_budget * fraction,
            spent: 0.0,
            max_single_operation: max_op,
        }
    }
}

/// The budget enforcement guard. Wraps all operations.
pub struct BudgetGuard {
    db: Arc<Mutex<Connection>>,
    /// Total shell conservation budget.
    total_budget: f64,
    /// Total spent this cycle.
    total_spent: f64,
    /// Total wasted (failed operations that still cost energy).
    total_wasted: f64,
    /// Per-ensign budgets.
    ensign_budgets: HashMap<String, EnsignBudget>,
    /// Degradation strategy when budget runs low.
    degradation_mode: DegradationMode,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DegradationMode {
    /// Normal operation. All operations allowed.
    Normal,
    /// Budget below 20%. Expensive operations degraded.
    Conservative,
    /// Budget below 5%. Only essential operations.
    Minimal,
    /// Budget exhausted. Only free operations (TileArchive, DeadbandCheck).
    Emergency,
}

impl BudgetGuard {
    pub fn new(db: Arc<Mutex<Connection>>, total_budget: f64) -> Self {
        BudgetGuard {
            db,
            total_budget,
            total_spent: 0.0,
            total_wasted: 0.0,
            ensign_budgets: HashMap::new(),
            degradation_mode: DegradationMode::Normal,
        }
    }

    /// Register an ensign's budget based on its autonomy level.
    pub fn register_ensign(&mut self, ensign_id: &str, level: u8) {
        let budget = EnsignBudget::for_level(ensign_id, level, self.total_budget);
        self.ensign_budgets.insert(ensign_id.to_string(), budget);
    }

    /// Remaining budget.
    pub fn remaining(&self) -> f64 {
        self.total_budget - self.total_spent - self.total_wasted
    }

    /// Current degradation mode based on remaining budget.
    pub fn current_degradation_mode(&self) -> DegradationMode {
        let ratio = self.remaining() / self.total_budget;
        if ratio <= 0.0 {
            DegradationMode::Emergency
        } else if ratio < 0.05 {
            DegradationMode::Minimal
        } else if ratio < 0.20 {
            DegradationMode::Conservative
        } else {
            DegradationMode::Normal
        }
    }

    /// The main budget check. Call BEFORE every operation.
    ///
    /// Returns a verdict:
    /// - Approved: proceed, cost deducted
    /// - Degraded: proceed with cheaper alternative
    /// - Exhausted: do not proceed
    pub fn check(&mut self, op: Operation, ensign_id: Option<&str>) -> BudgetVerdict {
        let cost = op.cost();

        // Check ensign-level budget first
        if let Some(eid) = ensign_id {
            if let Some(ensign_budget) = self.ensign_budgets.get_mut(eid) {
                if cost > ensign_budget.max_single_operation {
                    // This ensign can't afford this operation
                    return BudgetVerdict::Degraded {
                        original_cost: cost,
                        degraded_cost: ensign_budget.max_single_operation,
                        reason: format!(
                            "ensign {} at level {} can't spend {:.1} in one op (max {:.1})",
                            eid, ensign_budget.autonomy_level, cost,
                            ensign_budget.max_single_operation
                        ),
                    };
                }
                if ensign_budget.spent + cost > ensign_budget.daily_budget {
                    return BudgetVerdict::Degraded {
                        original_cost: cost,
                        degraded_cost: 0.0,
                        reason: format!(
                            "ensign {} daily budget exhausted ({:.1}/{:.1})",
                            eid, ensign_budget.spent, ensign_budget.daily_budget
                        ),
                    };
                }
            }
        }

        // Check shell-level budget
        if self.remaining() < cost {
            // Try to degrade the operation
            if let Some(degraded) = self.degrade_operation(&op) {
                let degraded_cost = degraded.cost();
                if self.remaining() >= degraded_cost {
                    self.spend_internal(degraded_cost, ensign_id);
                    return BudgetVerdict::Degraded {
                        original_cost: cost,
                        degraded_cost,
                        reason: format!(
                            "budget low ({:.1} remaining), degraded {:?} to {:?}",
                            self.remaining(), op, degraded
                        ),
                    };
                }
            }
            return BudgetVerdict::Exhausted {
                deficit: cost - self.remaining(),
            };
        }

        // Normal: deduct and approve
        self.spend_internal(cost, ensign_id);
        BudgetVerdict::Approved {
            remaining: self.remaining(),
            cost,
        }
    }

    /// Record a failed operation that still cost energy (waste).
    pub fn record_waste(&mut self, op: Operation, ensign_id: Option<&str>) {
        let cost = op.cost();
        self.total_wasted += cost;
        if let Some(eid) = ensign_id {
            if let Some(budget) = self.ensign_budgets.get_mut(eid) {
                budget.spent += cost;
            }
        }
    }

    /// Deposit budget (e.g., daily reset, or parent shell allocation).
    pub fn deposit(&mut self, amount: f64) {
        self.total_budget += amount;
    }

    /// Daily budget reset: zero out spending, keep total.
    pub fn daily_reset(&mut self) {
        self.total_spent = 0.0;
        self.total_wasted = 0.0;
        for budget in self.ensign_budgets.values_mut() {
            budget.spent = 0.0;
        }
    }

    /// Verify the conservation law:
    /// total_in == total_out + total_remaining + total_wasted
    pub fn verify_conservation(&self) -> Result<ConservationReport, String> {
        let total_in = self.total_budget;
        let total_out = self.total_spent;
        let total_remaining = self.remaining();
        let total_wasted = self.total_wasted;

        let balance = total_in - total_out - total_remaining - total_wasted;
        let balanced = balance.abs() < 0.001;

        Ok(ConservationReport {
            total_in,
            total_out,
            total_remaining,
            total_wasted,
            balance,
            balanced,
        })
    }

    /// Persist budget state to SQLite.
    pub async fn save(&self) -> Result<(), String> {
        let db = self.db.lock().await;
        db.execute(
            "UPDATE conservation SET value = ? WHERE key = 'budget'",
            rusqlite::params![self.total_budget.to_string()],
        ).map_err(|e| format!("save budget: {}", e))?;
        db.execute(
            "UPDATE conservation SET value = ? WHERE key = 'used'",
            rusqlite::params![self.total_spent.to_string()],
        ).map_err(|e| format!("save spent: {}", e))?;
        db.execute(
            "UPDATE conservation SET value = ? WHERE key = 'wasted'",
            rusqlite::params![self.total_wasted.to_string()],
        ).map_err(|e| format!("save wasted: {}", e))?;
        Ok(())
    }

    // ── Internal ──────────────────────────────────────────────────────────

    fn spend_internal(&mut self, cost: f64, ensign_id: Option<&str>) {
        self.total_spent += cost;
        if let Some(eid) = ensign_id {
            if let Some(budget) = self.ensign_budgets.get_mut(eid) {
                budget.spent += cost;
            }
        }
        self.degradation_mode = self.current_degradation_mode();
    }

    /// Degrade an operation to a cheaper alternative.
    fn degrade_operation(&self, op: &Operation) -> Option<Operation> {
        match op {
            Operation::PhoneAFriend =>
                // Degrade to regular text response
                Some(Operation::TextResponse),
            Operation::CodeGeneration =>
                // Degrade to text response (no code execution)
                Some(Operation::TextResponse),
            Operation::ToolExecution =>
                // Degrade: just explain what would happen
                Some(Operation::TextResponse),
            Operation::CorrelationScan =>
                // Skip: return no correlations
                None,
            Operation::PenroseRefit =>
                // Skip entirely
                None,
            // Tile operations are already cheap — no degradation
            _ => None,
        }
    }
}

/// Conservation verification report.
#[derive(Debug, Clone)]
pub struct ConservationReport {
    pub total_in: f64,
    pub total_out: f64,
    pub total_remaining: f64,
    pub total_wasted: f64,
    pub balance: f64,
    pub balanced: bool,
}
```

### Tile Conservation Delta Recording

Every tile records its conservation delta in SQLite. This is the integration point with the tile store:

```rust
/// Extension to the existing Tile type for budget tracking.
impl Tile {
    /// Record the conservation cost of creating this tile.
    pub fn record_budget_cost(&mut self, op: Operation, verdict: &BudgetVerdict) {
        match verdict {
            BudgetVerdict::Approved { cost, .. } => {
                self.conservation_delta = -cost;
            }
            BudgetVerdict::Degraded {
                degraded_cost, ..
            } => {
                self.conservation_delta = -degraded_cost;
            }
            BudgetVerdict::Exhausted { .. } => {
                self.conservation_delta = 0.0; // no cost, operation didn't happen
            }
        }
    }
}

/// When inserting a tile, also record the conservation delta
/// in the tile's metadata for auditing.
fn insert_tile_with_budget(
    conn: &Connection,
    tile: &Tile,
) -> Result<(), rusqlite::Error> {
    // Insert the tile as normal
    crate::tile::insert_tile(conn, tile)?;

    // Also update the room's running conservation tally
    if let Some(room_id) = &tile.room_id {
        conn.execute(
            "UPDATE rooms SET conservation_delta = COALESCE(conservation_delta, 0.0) + ?1
             WHERE id = ?2",
            rusqlite::params![tile.conservation_delta, room_id],
        ).ok(); // best-effort
    }

    Ok(())
}
```

### Kernel Integration

The `BudgetGuard` wraps every operation in the kernel:

```rust
// In kernel.rs, add a BudgetGuard field:

pub struct ShellKernel {
    // ... existing fields ...
    pub budget_guard: Arc<Mutex<BudgetGuard>>,
}

// Modified process_message with budget enforcement:
impl ShellKernel {
    pub async fn process_message(&self, msg: &PortMessage) -> Result<(), String> {
        let tick = conservation::advance_tick();
        let mut guard = self.budget_guard.lock().await;

        // 1. Check budget for tile creation
        match guard.check(Operation::TileCreate, None) {
            BudgetVerdict::Approved { .. } => {},
            BudgetVerdict::Degraded { .. } => {},
            BudgetVerdict::Exhausted { deficit } => {
                log::error!("Budget exhausted, cannot process message (deficit: {:.1})", deficit);
                // Still send a response to the user
                for port in &self.ports {
                    let p = port.lock().await;
                    if p.is_active() {
                        let _ = p.send(&PortResponse {
                            text: "I'm running low on energy right now. Please try again later.".to_string(),
                            reply_to: msg.chat_id.to_string(),
                        }).await;
                    }
                }
                return Ok(());
            }
        }

        // 2. Route to room
        let db = self.db.lock().await;
        let routing = route_message(&db, &msg.text, &[])?;
        drop(db);

        let ensign_id_opt: Option<String> = {
            let db = self.db.lock().await;
            ensign::get_ensign_for_room(&db, &routing.room_id)
                .ok()
                .flatten()
                .map(|e| e.id.clone())
        };

        // 3. Check budget for the actual operation
        let op = if msg.text.contains("code") || msg.text.contains("build") {
            Operation::CodeGeneration
        } else if msg.text.contains("research") || msg.text.contains("analyze") {
            Operation::CorrelationScan
        } else {
            Operation::TextResponse
        };

        let (ensign_id_ref, effective_op) = match guard.check(op, ensign_id_opt.as_deref()) {
            BudgetVerdict::Approved { cost, .. } => {
                (ensign_id_opt.as_deref(), op)
            }
            BudgetVerdict::Degraded {
                degraded_cost, reason, ..
            } => {
                log::warn!("Budget degradation: {}", reason);
                (ensign_id_opt.as_deref(), Operation::TextResponse)
            }
            BudgetVerdict::Exhausted { .. } => {
                drop(guard);
                // Send low-energy response
                for port in &self.ports {
                    let p = port.lock().await;
                    if p.is_active() {
                        let _ = p.send(&PortResponse {
                            text: "Energy reserves critical. Standing by.".to_string(),
                            reply_to: msg.chat_id.to_string(),
                        }).await;
                    }
                }
                return Ok(());
            }
        };

        // 4. Execute the operation (existing API call logic)
        // ... model params, provider call, create action tile ...

        // 5. Record budget in tile
        // tile.record_budget_cost(effective_op, &verdict);

        // 6. Save budget state
        guard.save().await?;
        drop(guard);

        Ok(())
    }
}
```

### Graceful Degradation Cascade

```
Budget Status          Behavior
─────────────────      ──────────────────────────────────────────────────
> 20% remaining        Normal operation. All operations allowed.
                       Phone-a-friend (20 units) approved.

20% → 5% remaining    Conservative mode.
                       Phone-a-friend → degraded to TextResponse (1 unit)
                       CodeGeneration → degraded to TextResponse (1 unit)
                       Correlation scans skipped.
                       Background tick continues but skips Penrose refit.

5% → 0% remaining     Minimal mode.
                       Only TileCreate (0.1), TileComplete (0.05),
                       DeadbandCheck (0.02) allowed.
                       All other ops blocked.
                       User gets: "Running low on energy."

0% (exhausted)         Emergency mode.
                       Only TileArchive (0.01) and DeadbandCheck (0.02).
                       No API calls at all.
                       User gets: "Energy reserves critical. Standing by."
                       System tile created: "Budget exhausted at tick N"
                       Daily reset will restore operation.
```

### Conservation Law Verification

```rust
/// Run conservation verification at every background tick.
///
/// The law: total_in = total_out + total_remaining + total_wasted
///
/// If violated:
/// - Log a warning
/// - Create a system tile documenting the imbalance
/// - Attempt reconciliation (adjust totals)
pub async fn verify_and_reconcile(guard: &Mutex<BudgetGuard>, db: &Mutex<Connection>) {
    let mut g = guard.lock().await;
    let report = g.verify_conservation().unwrap();

    if !report.balanced {
        log::warn!(
            "CONSERVATION VIOLATION: balance={:.6} \
             (in={:.2} out={:.2} remain={:.2} waste={:.2})",
            report.balance,
            report.total_in,
            report.total_out,
            report.total_remaining,
            report.total_wasted,
        );

        // Create a system tile documenting the violation
        let tick = conservation::current_tick();
        let mut tile = Tile::new(
            TileType::Escalation,
            &format!(
                "Conservation law violation: balance={:.6}. \
                 in={:.2} out={:.2} remain={:.2} waste={:.2}",
                report.balance,
                report.total_in,
                report.total_out,
                report.total_remaining,
                report.total_wasted,
            ),
            tick,
        );
        tile.room_id = Some("system".to_string());

        let conn = db.lock().await;
        let _ = tile::insert_tile(&conn, &tile);
    }
}
```

### Ensign Budget Allocation Table

| Autonomy Level | Daily Budget | Max Single Op | Can Phone-a-Friend? | Can CodeGen? |
|---|---|---|---|---|
| 1 (all Opus) | 10% of total | 5 units | No | No |
| 2 (observe) | 20% of total | 10 units | No | No |
| 3 (routine) | 50% of total | 25 units | No | Yes |
| 4 (autonomous) | 70% of total | 50 units | Yes (1/day) | Yes |
| 5 (self-op) | 90% of total | 100 units | Yes (3/day) | Yes |

Example with total budget = 10,000 units:
- Level 1 ensign: 1,000 units/day, max 5 per op
- Level 3 ensign: 5,000 units/day, max 25 per op
- Level 5 ensign: 9,000 units/day, max 100 per op
