//! kernel.rs — ShellKernel tying it all together
//!
//! The main tick loop, message routing, and conservation budget management.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::conservation::{self, ConservationState, costs};
use crate::deadband;
use crate::ensign::{self, CompletionRequest, Provider};
use crate::gravity;
use crate::penrose;
use crate::port::{Port, PortMessage, PortResponse};
use crate::room;
use crate::tile::{self, Tile, TileType};

/// Helper to convert rusqlite errors to String
fn sql_err(e: rusqlite::Error) -> String {
    format!("{}", e)
}

// ---------------------------------------------------------------------------
// Conservation degradation
// ---------------------------------------------------------------------------

/// Below this much remaining budget we throttle params and switch to the
/// cheapest provider. Above it, full gravity-derived params are used.
const BUDGET_SOFT_FLOOR: f64 = 100.0;
/// Below this we stop calling providers entirely and refuse gracefully.
const BUDGET_HARD_FLOOR: f64 = 10.0;

/// How the kernel handles a message given the remaining conservation budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DegradeMode {
    /// Plenty of budget — honor the room's gravity-derived params.
    Full,
    /// Low budget — clamp tokens, force precise/cheap, prefer cheapest provider.
    Throttled,
    /// Out of budget — no API call; refuse honestly and escalate the tile.
    Floor,
}

// ---------------------------------------------------------------------------
// KernelMetrics — observability (Samira's Prometheus request)
// ---------------------------------------------------------------------------

/// Snapshot of kernel state for metrics / monitoring.
///
/// All fields are computed on demand from live state. Can be serialized to
/// JSON for Prometheus exposition or any other metrics backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelMetrics {
    /// Total tiles processed across all rooms.
    pub tiles_processed: u64,
    /// Remaining conservation budget in dollars.
    pub conservation_remaining: f64,
    /// Total conservation budget in dollars.
    pub conservation_budget: f64,
    /// Conservation used in dollars.
    pub conservation_used: f64,
    /// Number of active rooms.
    pub rooms_active: u64,
    /// Number of loaded modules.
    pub modules_loaded: u64,
    /// Average algebraic connectivity (Fiedler value) across the room graph.
    /// 0.0 if no correlations have been computed yet.
    pub avg_fiedler: f64,
    /// Current kernel tick.
    pub tick: u64,
    /// Number of registered providers.
    pub providers_count: usize,
    /// Number of open ports.
    pub ports_count: usize,
}

// ---------------------------------------------------------------------------
// Provenance — chain-of-custody for every decision (recorded, no new file)
// ---------------------------------------------------------------------------

/// One provenance entry: who produced a tile, with what model/params, at what
/// cost, and which decision the kernel made (normal / throttled / floor-refusal).
struct ProvenanceEntry {
    id: String,
    tile_id: String,
    ensign_id: String,
    model: String,
    provider: String,
    prompt_style: String,
    temperature: f64,
    tokens_used: u32,
    conservation_cost: f64,
    room_id: String,
    tick: u64,
    parent_provenance: Option<String>,
    decision: String,
    is_valid: bool,
}

fn record_provenance(conn: &Connection, p: &ProvenanceEntry) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO provenance
            (id, tile_id, ensign_id, model, provider, prompt_style, temperature,
             tokens_used, conservation_cost, room_id, tick, parent_provenance,
             decision, is_valid)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        rusqlite::params![
            p.id,
            p.tile_id,
            p.ensign_id,
            p.model,
            p.provider,
            p.prompt_style,
            p.temperature,
            p.tokens_used as i64,
            p.conservation_cost,
            p.room_id,
            p.tick as i64,
            p.parent_provenance,
            p.decision,
            p.is_valid,
        ],
    )?;
    Ok(())
}

// ---------------------------------------------------------------------------
// ShellKernel
// ---------------------------------------------------------------------------

pub struct ShellKernel {
    pub db: Arc<Mutex<Connection>>,
    pub providers: Vec<(String, Box<dyn Provider>)>,
    pub ports: Vec<Arc<Mutex<dyn Port>>>,
    pub conservation: Arc<Mutex<ConservationState>>,
    pub gravity_history: Arc<Mutex<HashMap<String, Vec<f64>>>>,
    pub tick_interval_ms: u64,
}

impl ShellKernel {
    /// Bootstrap: init all SQLite schemas, load rooms/ensigns
    pub async fn bootstrap(
        db_path: &str,
        rooms_dir: &str,
        ensigns_dir: &str,
    ) -> Result<Self, String> {
        // Open SQLite with WAL mode
        let conn = Connection::open(db_path)
            .map_err(|e| format!("open db: {}", e))?;

        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;")
            .map_err(|e| format!("set WAL mode: {}", e))?;

        // Init all schemas
        conservation::init_schema(&conn).map_err(sql_err)?;
        tile::init_schema(&conn).map_err(sql_err)?;
        room::init_schema(&conn).map_err(sql_err)?;
        ensign::init_schema(&conn).map_err(sql_err)?;
        penrose::init_schema(&conn).map_err(sql_err)?;
        deadband::init_schema(&conn).map_err(sql_err)?;

        // Provenance: chain-of-custody for every decision the kernel makes.
        // (Lives here as wiring rather than a separate module/file.)
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS provenance (
                id TEXT PRIMARY KEY,
                tile_id TEXT NOT NULL,
                ensign_id TEXT,
                model TEXT NOT NULL,
                provider TEXT NOT NULL,
                prompt_style TEXT,
                temperature REAL,
                tokens_used INTEGER,
                conservation_cost REAL,
                room_id TEXT,
                tick INTEGER,
                parent_provenance TEXT,
                decision TEXT,
                is_valid BOOLEAN DEFAULT 1
            );
            CREATE INDEX IF NOT EXISTS idx_prov_tile ON provenance(tile_id);
            CREATE INDEX IF NOT EXISTS idx_prov_room ON provenance(room_id);",
        )
        .map_err(|e| format!("provenance init: {}", e))?;

        // Shell metadata
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS shell_meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            INSERT OR IGNORE INTO shell_meta (key, value) VALUES ('shell_id', 'hermes-construct');
            INSERT OR IGNORE INTO shell_meta (key, value) VALUES ('shell_type', 'hermes');
            INSERT OR IGNORE INTO shell_meta (key, value) VALUES ('autonomy_level', '1');
            INSERT OR IGNORE INTO shell_meta (key, value) VALUES ('conservation_budget', '10000');"
        ).map_err(|e| format!("shell_meta init: {}", e))?;

        let tick = conservation::current_tick();

        // Load rooms from JSON
        let rooms = room::load_rooms_from_dir(&conn, rooms_dir, tick)?;
        log::info!("Loaded {} rooms from {}", rooms.len(), rooms_dir);

        // Load ensigns from JSON
        let mut ensigns = ensign::load_ensigns_from_dir(&conn, ensigns_dir)?;
        log::info!("Loaded {} ensigns from {}", ensigns.len(), ensigns_dir);

        // Bind ensigns to their rooms. Rooms reference an ensign via `ensign_id`,
        // but ensigns load without a `room_id`, so get_ensign_for_room() (which
        // queries ensigns.room_id) would never match. Back-fill the link here so
        // the ensign lifecycle can actually resolve and fire per room.
        for r in &rooms {
            if let Some(eid) = &r.ensign_id {
                if let Some(e) = ensigns.iter_mut().find(|e| &e.id == eid) {
                    e.room_id = Some(r.id.clone());
                    ensign::upsert_ensign(&conn, e).map_err(sql_err)?;
                    log::info!("Bound ensign {} to room {}", e.id, r.id);
                }
            }
        }

        // Load conservation state
        let mut cons_state = conservation::load_state(&conn).map_err(sql_err)?;
        cons_state.tick = tick;

        // Initialize gravity history
        let mut gravity_history = HashMap::new();
        for r in &rooms {
            gravity_history.entry(r.id.clone())
                .or_insert_with(|| vec![r.gravity]);
        }

        Ok(Self {
            db: Arc::new(Mutex::new(conn)),
            providers: Vec::new(),
            ports: Vec::new(),
            conservation: Arc::new(Mutex::new(cons_state)),
            gravity_history: Arc::new(Mutex::new(gravity_history)),
            tick_interval_ms: 30_000,
        })
    }

    /// Register a provider
    pub fn add_provider(&mut self, name: &str, provider: Box<dyn Provider>) {
        self.providers.push((name.to_string(), provider));
    }

    /// Register a port
    pub fn add_port(&mut self, port: Arc<Mutex<dyn Port>>) {
        self.ports.push(port);
    }

    /// Pick the cheapest available provider for degraded operation.
    /// DeepInfra (Seed-mini class) is treated as the low-cost default;
    /// otherwise fall back to whatever provider is registered first.
    fn cheapest_provider(&self) -> Option<String> {
        if self.providers.iter().any(|(n, _)| n == "deepinfra") {
            Some("deepinfra".to_string())
        } else {
            self.providers.first().map(|(n, _)| n.clone())
        }
    }

    /// Process an incoming message through the full pipeline.
    ///
    /// Steps: observe -> route -> drive ensign lifecycle -> derive (and possibly
    /// degrade) params -> provide-or-refuse -> tiles -> gravity -> provenance ->
    /// reply -> persist. Low budget degrades behavior; it never aborts the loop.
    pub async fn process_message(&self, msg: &PortMessage) -> Result<(), String> {
        let tick = conservation::advance_tick();

        // 0. Decide degradation mode from remaining budget, up front.
        let remaining = { self.conservation.lock().await.remaining() };
        let mode = if remaining <= BUDGET_HARD_FLOOR {
            DegradeMode::Floor
        } else if remaining <= BUDGET_SOFT_FLOOR {
            DegradeMode::Throttled
        } else {
            DegradeMode::Full
        };
        if mode != DegradeMode::Full {
            log::warn!(
                "conservation low (remaining={:.2}) — degrading to {:?}",
                remaining, mode
            );
        }

        // 1. Observation tile. Spends are best-effort: at the floor we still
        //    record what happened rather than aborting the message.
        let mut obs_tile = Tile::new(TileType::Observation, &msg.text, tick);
        obs_tile.conservation_delta = costs::TILE_CREATE;
        {
            let mut cons = self.conservation.lock().await;
            let _ = cons.spend(costs::TILE_CREATE);
            cons.tick = tick;
        }

        // 2. Route to room
        let room = {
            let db = self.db.lock().await;
            room::route_to_room(&db, &msg.text).map_err(sql_err)?
        };
        let room_id = room.as_ref().map(|r| r.id.clone())
            .unwrap_or_else(|| "default".to_string());
        obs_tile.room_id = Some(room_id.clone());

        // 3. Get the room's ensign and drive its lifecycle (wake -> handle this
        //    tick). A Dormant ensign is woken, oriented and brought to yellow
        //    alert so it can_handle() the message immediately. At the floor we
        //    leave the ensign asleep — there is no budget to spin it up.
        let mut ensign_info = {
            let db = self.db.lock().await;
            ensign::get_ensign_for_room(&db, &room_id).map_err(sql_err)?
        };
        if mode != DegradeMode::Floor {
            if let Some(ref mut e) = ensign_info {
                if !e.can_handle() {
                    e.wake();
                    e.orient();
                    e.go_yellow();
                    e.record_call(costs::ENSIGN_ACTIVATE);
                    {
                        let mut cons = self.conservation.lock().await;
                        let _ = cons.spend(costs::ENSIGN_ACTIVATE);
                    }
                    let db = self.db.lock().await;
                    let _ = ensign::upsert_ensign(&db, e);
                }
            }
        }

        // 4. Derive model params from room gravity, then apply degradation.
        let mut model_params = room.as_ref()
            .map(|r| r.model_params())
            .unwrap_or_else(|| gravity::gravity_to_params(0.0));

        let mut model_name = ensign_info.as_ref()
            .map(|e| e.model_name.clone())
            .unwrap_or_else(|| "seed-2.0-mini".to_string());
        let mut provider_name = ensign_info.as_ref()
            .map(|e| e.provider.clone())
            .unwrap_or_else(|| "deepinfra".to_string());

        if mode == DegradeMode::Throttled {
            // Clamp tokens, force precise/cheap sampling, prefer cheapest provider.
            model_params.max_tokens = model_params.max_tokens.min(256);
            model_params.temperature = 0.3;
            model_params.top_p = 0.9;
            model_params.prompt_style = "precise".to_string();
            if let Some(cheap) = self.cheapest_provider() {
                if cheap != provider_name {
                    provider_name = cheap;
                    model_name = "seed-2.0-mini".to_string();
                }
            }
        }

        let system_prompt = gravity::style_to_system_prompt(&model_params.prompt_style);

        // 5. Produce a response: refuse at the floor, otherwise call the provider.
        let (response_text, tokens_used, model_used, decision): (String, u32, String, &str) =
            if mode == DegradeMode::Floor {
                (
                    "I'm running low on energy budget and need to pause new work. \
                     Please try again once the budget is replenished.".to_string(),
                    0,
                    "none".to_string(),
                    "floor-refusal",
                )
            } else {
                let completion_result = {
                    let provider = self.providers.iter()
                        .find(|(n, _)| n == &provider_name)
                        .map(|(_, p)| p.as_ref());
                    match provider {
                        Some(p) => {
                            let request = CompletionRequest {
                                prompt: msg.text.clone(),
                                model: model_name.clone(),
                                params: model_params.clone(),
                                system_prompt: Some(system_prompt),
                            };
                            p.complete(&request).await
                        }
                        None => Err(format!("no provider '{}' available", provider_name)),
                    }
                };
                match completion_result {
                    Ok(resp) => (
                        resp.text.clone(),
                        resp.tokens_used,
                        resp.model.clone(),
                        if mode == DegradeMode::Throttled { "throttled" } else { "normal" },
                    ),
                    Err(e) => {
                        log::error!("completion error: {}", e);
                        (
                            "I encountered an error processing your request.".to_string(),
                            0,
                            "none".to_string(),
                            "error",
                        )
                    }
                }
            };

        // 6. Action tile.
        let mut action_tile = Tile::new(TileType::Action, &response_text, tick);
        action_tile.room_id = Some(room_id.clone());
        action_tile.parent_id = Some(obs_tile.id.clone());
        action_tile.model_used = Some(model_used.clone());
        action_tile.tokens_used = tokens_used;
        action_tile.ensign_id = ensign_info.as_ref().map(|e| e.id.clone());
        let action_cost = costs::TILE_CREATE + costs::ENSIGN_TILE;
        action_tile.conservation_delta = action_cost;
        if mode == DegradeMode::Floor {
            action_tile.escalate("conservation budget floor reached", tick);
        }
        {
            let mut cons = self.conservation.lock().await;
            let _ = cons.spend(action_cost);
        }

        // 7. Charge the ensign for a real provider hit, and persist its energy.
        if tokens_used > 0 {
            if let Some(ref mut e) = ensign_info {
                e.record_call(costs::ENSIGN_TILE);
                let db = self.db.lock().await;
                let _ = ensign::upsert_ensign(&db, e);
            }
        }

        // 8. Update room gravity from the interaction signal.
        if let Some(ref room) = room {
            let signal = if tokens_used > 0 { 0.05 } else { -0.05 };
            let mut updated_room = room.clone();
            updated_room.nudge_gravity(signal, 0.1, tick);
            {
                let db = self.db.lock().await;
                let _ = room::upsert_room(&db, &updated_room).map_err(sql_err);
            }
            let mut gh = self.gravity_history.lock().await;
            gh.entry(room.id.clone())
                .or_default()
                .push(updated_room.gravity);
        }

        // 9. Persist tiles + a provenance entry for the decision made.
        {
            let db = self.db.lock().await;
            let _ = tile::insert_tile(&db, &obs_tile);
            // A floor-refusal tile is already escalated; don't overwrite that
            // status with Complete.
            if mode != DegradeMode::Floor {
                action_tile.complete(tick);
            }
            let _ = tile::insert_tile(&db, &action_tile);

            let prov = ProvenanceEntry {
                id: uuid::Uuid::new_v4().to_string(),
                tile_id: action_tile.id.clone(),
                ensign_id: ensign_info.as_ref().map(|e| e.id.clone())
                    .unwrap_or_else(|| "unassigned".to_string()),
                model: model_used.clone(),
                provider: if tokens_used > 0 { provider_name.clone() } else { "none".to_string() },
                prompt_style: model_params.prompt_style.clone(),
                temperature: model_params.temperature,
                tokens_used,
                conservation_cost: action_cost,
                room_id: room_id.clone(),
                tick,
                parent_provenance: Some(obs_tile.id.clone()),
                decision: decision.to_string(),
                is_valid: true,
            };
            if let Err(e) = record_provenance(&db, &prov) {
                log::error!("provenance record error: {}", e);
            }
        }

        // 10. Send response over active ports.
        for port in &self.ports {
            let p = port.lock().await;
            if p.is_active() {
                let response = PortResponse {
                    text: response_text.clone(),
                    reply_to: msg.chat_id.to_string(),
                };
                let _ = p.send(&response).await;
            }
        }

        // 11. Persist conservation state.
        {
            let db = self.db.lock().await;
            let cons = self.conservation.lock().await;
            let _ = conservation::save_state(&db, &cons);
        }

        Ok(())
    }

    /// Background tick: decay gravities, scan correlations, check deadbands
    pub async fn background_tick(&self) -> Result<(), String> {
        let tick = conservation::advance_tick();

        let db = self.db.lock().await;

        // Decay room gravities
        let rooms = room::get_all_rooms(&db)
            .map_err(|e| format!("get rooms: {}", e))?;

        for mut r in rooms {
            r.decay_gravity(0.01, tick);
            room::upsert_room(&db, &r).map_err(sql_err)?;

            // Track gravity
            let mut gh = self.gravity_history.lock().await;
            gh.entry(r.id.clone()).or_default().push(r.gravity);
        }

        // Scan correlations (only if we have enough history)
        let gh = self.gravity_history.lock().await;
        let histories: HashMap<String, Vec<f64>> = gh.clone();
        drop(gh);

        if !histories.is_empty() {
            let corrs = penrose::scan_correlations(&db, &histories, tick)
                .map_err(sql_err)?;
            if !corrs.is_empty() {
                log::info!("Detected {} correlations", corrs.len());
            }
        }

        // Check deadbands
        let circuits = deadband::run_checks(&db, tick)
            .map_err(sql_err)?;
        for circuit in &circuits {
            if circuit.is_breached {
                log::warn!(
                    "Deadband breach: {} in room {}",
                    circuit.name, circuit.room_id
                );
            }
        }

        // Update conservation
        {
            let mut cons = self.conservation.lock().await;
            let _ = cons.spend(costs::GRAVITY_UPDATE + costs::CORRELATION_COMPUTE + costs::DEADBAND_CHECK);
            cons.tick = tick;
            let _ = conservation::save_state(&db, &cons);
        }

        Ok(())
    }

    /// Drain each port once and process any pending messages, then yield
    /// briefly. The short sleep keeps the loop calm (no busy-spin) and lets the
    /// Ctrl+C branch in `run()` win the `select!` instead of being starved.
    async fn poll_ports(&self) {
        for port in &self.ports {
            let p = port.lock().await;
            if let Some(msg) = p.receive().await {
                drop(p);
                if let Err(e) = self.process_message(&msg).await {
                    log::error!("message processing error: {}", e);
                }
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    /// Collect current kernel metrics for observability.
    ///
    /// Can be wired to Prometheus (`kernel_tiles_processed`, etc.) or any
    /// other metrics backend.
    #[allow(dead_code)]
    pub async fn metrics(&self) -> KernelMetrics {
        let db = self.db.lock().await;
        let cons = self.conservation.lock().await;

        let tiles_processed = tile::query_tiles(&db, None, None, None, usize::MAX)
            .map(|t| t.len() as u64)
            .unwrap_or(0);

        let rooms_active = room::get_all_rooms(&db)
            .map(|r| r.len() as u64)
            .unwrap_or(0);

        // Average Fiedler value = average absolute correlation coefficient
        // across all detected correlations. Higher = more connected rooms.
        let correlations = penrose::get_all_correlations(&db).unwrap_or_default();
        let avg_fiedler = if correlations.is_empty() {
            0.0
        } else {
            correlations.iter().map(|c| c.correlation.abs()).sum::<f64>() / correlations.len() as f64
        };

        KernelMetrics {
            tiles_processed,
            conservation_remaining: cons.remaining(),
            conservation_budget: cons.budget,
            conservation_used: cons.used,
            rooms_active,
            modules_loaded: 0, // TODO: wire to module loader
            avg_fiedler,
            tick: cons.tick,
            providers_count: self.providers.len(),
            ports_count: self.ports.len(),
        }
    }

    /// Graceful shutdown: stand down every ensign, persist conservation state,
    /// and checkpoint the WAL so SQLite closes cleanly on drop.
    pub async fn shutdown(&self) -> Result<(), String> {
        log::info!("Standing down ensigns and saving state...");
        let db = self.db.lock().await;

        match ensign::get_all_ensigns(&db) {
            Ok(ensigns) => {
                for mut e in ensigns {
                    e.stand_down();
                    let _ = ensign::upsert_ensign(&db, &e);
                }
            }
            Err(e) => log::error!("stand-down: could not load ensigns: {}", e),
        }

        {
            let cons = self.conservation.lock().await;
            if let Err(e) = conservation::save_state(&db, &cons) {
                log::error!("shutdown: save conservation: {}", e);
            }
            log::info!(
                "Final conservation: used={:.2} / budget={:.2} ({:.2} remaining)",
                cons.used, cons.budget, cons.remaining()
            );
        }

        // Fold the WAL back into the main db file so nothing is lost on close.
        if let Err(e) = db.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);") {
            log::error!("shutdown: wal checkpoint: {}", e);
        }

        log::info!("Shutdown complete.");
        Ok(())
    }

    /// Run the main event loop until Ctrl+C, then shut down gracefully.
    pub async fn run(&self) -> Result<(), String> {
        log::info!("Hermes Construct kernel starting...");

        let mut tick_interval = tokio::time::interval(
            std::time::Duration::from_millis(self.tick_interval_ms)
        );

        loop {
            tokio::select! {
                // Graceful shutdown on Ctrl+C.
                _ = tokio::signal::ctrl_c() => {
                    log::info!("Ctrl+C received — shutting down gracefully...");
                    break;
                }

                // Background tick: gravity decay, correlations, deadbands.
                _ = tick_interval.tick() => {
                    if let Err(e) = self.background_tick().await {
                        log::error!("background tick error: {}", e);
                    }
                }

                // Poll ports for incoming messages.
                _ = self.poll_ports() => {}
            }
        }

        self.shutdown().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ensign::{CompletionRequest, CompletionResponse, Provider};
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicU32, Ordering as AtomicOrdering};

    struct MockProvider {
        call_count: AtomicU32,
    }

    #[async_trait]
    impl Provider for MockProvider {
        async fn complete(&self, req: &CompletionRequest) -> Result<CompletionResponse, String> {
            self.call_count.fetch_add(1, AtomicOrdering::Relaxed);
            Ok(CompletionResponse {
                text: format!("echo: {}", req.prompt),
                model: req.model.clone(),
                tokens_used: 10,
                provider: "mock".into(),
            })
        }
        fn name(&self) -> &str { "mock" }
    }

    async fn make_kernel() -> ShellKernel {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = OFF;").unwrap();

        conservation::init_schema(&conn).unwrap();
        tile::init_schema(&conn).unwrap();
        room::init_schema(&conn).unwrap();
        ensign::init_schema(&conn).unwrap();
        penrose::init_schema(&conn).unwrap();
        deadband::init_schema(&conn).unwrap();

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS provenance (
                id TEXT PRIMARY KEY, tile_id TEXT NOT NULL, ensign_id TEXT,
                model TEXT NOT NULL, provider TEXT NOT NULL, prompt_style TEXT,
                temperature REAL, tokens_used INTEGER, conservation_cost REAL,
                room_id TEXT, tick INTEGER, parent_provenance TEXT,
                decision TEXT, is_valid BOOLEAN DEFAULT 1
            );
            CREATE INDEX IF NOT EXISTS idx_prov_tile ON provenance(tile_id);
            CREATE INDEX IF NOT EXISTS idx_prov_room ON provenance(room_id);")
        .unwrap();

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS shell_meta (
                key TEXT PRIMARY KEY, value TEXT NOT NULL
            );")
        .unwrap();

        let cons_state = conservation::load_state(&conn).unwrap();

        ShellKernel {
            db: Arc::new(Mutex::new(conn)),
            providers: vec![("deepinfra".into(), Box::new(MockProvider { call_count: AtomicU32::new(0) }))],
            ports: vec![],
            conservation: Arc::new(Mutex::new(cons_state)),
            gravity_history: Arc::new(Mutex::new(HashMap::new())),
            tick_interval_ms: 30_000,
        }
    }

    fn test_message(text: &str) -> PortMessage {
        PortMessage {
            id: uuid::Uuid::new_v4().to_string(),
            text: text.to_string(),
            chat_id: 42,
            from_user: Some("tester".into()),
            timestamp: 0,
        }
    }

    #[tokio::test]
    async fn process_message_creates_tiles() {
        let kernel = make_kernel().await;
        let msg = test_message("build a feature");
        kernel.process_message(&msg).await.unwrap();

        let db = kernel.db.lock().await;
        let tiles = tile::query_tiles(&db, None, None, None, 10).unwrap();
        assert!(tiles.len() >= 2, "expected >= 2 tiles, got {}", tiles.len());

        let obs = tiles.iter().find(|t| t.tile_type == TileType::Observation).unwrap();
        assert_eq!(obs.content, "build a feature");
    }

    #[tokio::test]
    async fn process_message_uses_provider() {
        let kernel = make_kernel().await;
        let msg = test_message("debug this");
        kernel.process_message(&msg).await.unwrap();

        let db = kernel.db.lock().await;
        let actions = tile::query_tiles(&db, None, Some(&TileType::Action), None, 10).unwrap();
        assert!(!actions.is_empty(), "no action tiles found");
        let action = &actions[0];
        assert!(action.content.starts_with("echo: debug this"));
        assert_eq!(action.tokens_used, 10);
    }

    #[tokio::test]
    async fn process_message_floor_refuses() {
        use crate::tile::TileStatus;
        let kernel = make_kernel().await;
        {
            let mut cons = kernel.conservation.lock().await;
            cons.used = cons.budget - 5.0;
        }
        let msg = test_message("help");
        kernel.process_message(&msg).await.unwrap();

        let db = kernel.db.lock().await;
        let actions = tile::query_tiles(&db, None, Some(&TileType::Action), None, 10).unwrap();
        assert!(!actions.is_empty(), "no action tiles found");
        assert!(actions[0].content.contains("low on energy"));
        assert_eq!(actions[0].status, TileStatus::Escalated);
    }

    #[tokio::test]
    async fn process_message_persists_conservation() {
        let kernel = make_kernel().await;
        let msg = test_message("hello");
        kernel.process_message(&msg).await.unwrap();

        let db = kernel.db.lock().await;
        let state = conservation::load_state(&db).unwrap();
        assert!(state.used > 0.0);
    }

    #[tokio::test]
    async fn cheapest_provider_prefers_deepinfra() {
        let kernel = make_kernel().await;
        assert_eq!(kernel.cheapest_provider(), Some("deepinfra".to_string()));
    }

    #[tokio::test]
    async fn cheapest_provider_empty() {
        let mut kernel = make_kernel().await;
        kernel.providers.clear();
        assert_eq!(kernel.cheapest_provider(), None);
    }

    #[tokio::test]
    async fn metrics_returns_snapshot() {
        let kernel = make_kernel().await;
        let m = kernel.metrics().await;
        assert_eq!(m.providers_count, 1);
        assert_eq!(m.ports_count, 0);
        assert!((m.conservation_budget - 10000.0).abs() < f64::EPSILON);
        assert!((m.conservation_used - 0.0).abs() < f64::EPSILON);
        assert_eq!(m.tiles_processed, 0);
        assert_eq!(m.rooms_active, 0);
    }

    #[tokio::test]
    async fn metrics_after_message() {
        let kernel = make_kernel().await;
        let msg = test_message("test metrics");
        kernel.process_message(&msg).await.unwrap();

        let m = kernel.metrics().await;
        assert!(m.conservation_used > 0.0);
    }
}
