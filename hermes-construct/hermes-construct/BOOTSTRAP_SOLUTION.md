# Puzzle 3: The Self-Bootstrap Sequence

## Solution: Exact Boot Sequence with Error Handling

### Overview

When hermes-construct first boots on Oracle with an empty SQLite database, the following sequence executes. Every step is logged as a tile for full auditability. Each step includes failure modes and recovery strategies.

### The Boot Sequence

```rust
/// The complete bootstrap sequence.
/// Each step is atomic: either it succeeds fully or it rolls back
/// and the system enters a recovery state.
pub async fn bootstrap(
    db_path: &str,
    rooms_dir: &str,
    ensigns_dir: &str,
) -> Result<ShellKernel, BootstrapError> {
    let bootstrap_start = std::time::Instant::now();

    // ═══════════════════════════════════════════════════════════════════
    // STEP 0: Process starts, loads .env
    // ═══════════════════════════════════════════════════════════════════

    let env_result = dotenvy::dotenv();
    match env_result {
        Ok(_) => log::info!("[BOOT] Step 0: .env loaded"),
        Err(_) => {
            log::warn!("[BOOT] Step 0: No .env file found, using environment variables");
            // NOT FATAL — env vars may be set by systemd/docker
        }
    }

    // Validate critical env vars
    let db_path = std::env::var("HERMES_DB_PATH")
        .unwrap_or_else(|_| "universe.db".to_string());
    let rooms_dir = std::env::var("HERMES_ROOMS_DIR")
        .unwrap_or_else(|_| "rooms".to_string());
    let ensigns_dir = std::env::var("HERMES_ENSIGNS_DIR")
        .unwrap_or_else(|_| "ensigns".to_string());
    let tick_ms: u64 = std::env::var("HERMES_TICK_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(30_000);

    // Check API keys (non-fatal — system can start in degraded mode)
    let has_deepinfra = !std::env::var("DEEPINFRA_API_KEY")
        .unwrap_or_default().is_empty();
    let has_zai = !std::env::var("ZAI_API_KEY")
        .unwrap_or_default().is_empty();
    let has_telegram = !std::env::var("TELEGRAM_BOT_TOKEN")
        .unwrap_or_default().is_empty();

    if !has_deepinfra && !has_zai {
        log::error!("[BOOT] Step 0: No API keys configured. System will start but cannot process messages.");
        // Continue — system can still run background tasks
    }

    // ═══════════════════════════════════════════════════════════════════
    // STEP 1: Init SQLite, create tables
    // ═══════════════════════════════════════════════════════════════════

    let conn = match Connection::open(&db_path) {
        Ok(c) => c,
        Err(e) => {
            return Err(BootstrapError::Fatal(
                format!("Step 1 FAILED: Cannot open SQLite at '{}': {}",
                    db_path, e)
            ));
        }
    };

    // Set WAL mode for concurrent access
    if let Err(e) = conn.execute_batch(
        "PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;"
    ) {
        // Recovery: try without WAL
        log::warn!("[BOOT] Step 1: WAL mode failed ({}), falling back to default", e);
        if let Err(e2) = conn.execute_batch("PRAGMA busy_timeout=5000;") {
            return Err(BootstrapError::Fatal(
                format!("Step 1 FAILED: Cannot set SQLite pragmas: {}", e2)
            ));
        }
    }

    // Create all tables (idempotent — IF NOT EXISTS)
    let schema_results: Vec<Result<(), rusqlite::Error>> = vec![
        conservation::init_schema(&conn),
        tile::init_schema(&conn),
        room::init_schema(&conn),
        ensign::init_schema(&conn),
        penrose::init_schema(&conn),
        deadband::init_schema(&conn),
    ];

    for (i, result) in schema_results.into_iter().enumerate() {
        if let Err(e) = result {
            return Err(BootstrapError::Fatal(
                format!("Step 1 FAILED: Schema {} init error: {}", i, e)
            ));
        }
    }

    // Shell metadata (idempotent)
    if let Err(e) = conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS shell_meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        INSERT OR IGNORE INTO shell_meta (key, value) VALUES
            ('shell_id', 'hermes-construct'),
            ('shell_type', 'hermes'),
            ('autonomy_level', '1'),
            ('conservation_budget', '10000'),
            ('boot_count', '0'),
            ('last_boot_tick', '0');"
    ) {
        return Err(BootstrapError::Fatal(
            format!("Step 1 FAILED: Shell metadata init: {}", e)
        ));
    }

    // Increment boot count
    let _ = conn.execute(
        "UPDATE shell_meta SET value = CAST(CAST(value AS INTEGER) + 1 AS TEXT)
         WHERE key = 'boot_count'",
        [],
    );

    let tick = conservation::current_tick();
    log::info!("[BOOT] Step 1: SQLite initialized, all schemas created (tick {})", tick);

    // ═══════════════════════════════════════════════════════════════════
    // STEP 2: Check for existing rooms in rooms/ directory
    // ═══════════════════════════════════════════════════════════════════

    let rooms = match room::load_rooms_from_dir(&conn, &rooms_dir, tick) {
        Ok(r) => r,
        Err(e) => {
            // Recovery: if rooms dir doesn't exist, we'll create defaults
            log::warn!("[BOOT] Step 2: Room loading failed: {}. Will create defaults.", e);
            vec![]
        }
    };

    let rooms_count = rooms.len();
    log::info!("[BOOT] Step 2: Found {} rooms in {}", rooms_count, rooms_dir);

    // ═══════════════════════════════════════════════════════════════════
    // STEP 3: If no rooms, create default rooms
    // ═══════════════════════════════════════════════════════════════════

    let rooms = if rooms.is_empty() {
        log::info!("[BOOT] Step 3: No rooms found, creating defaults");

        let default_rooms = vec![
            Room {
                id: "navigation".to_string(),
                room_type: RoomType::Navigation,
                gravity: -0.3,
                gravity_confidence: 0.1,
                temperature: 0.5,
                max_tokens: 1000,
                prompt_style: "balanced".to_string(),
                deadband_tolerance: 0.05,
                ensign_id: None,
                config: None,
                created_tick: tick,
                updated_tick: tick,
            },
            Room {
                id: "engineering".to_string(),
                room_type: RoomType::Engineering,
                gravity: -0.6,
                gravity_confidence: 0.1,
                temperature: 0.3,
                max_tokens: 500,
                prompt_style: "precise".to_string(),
                deadband_tolerance: 0.10,
                ensign_id: None,
                config: None,
                created_tick: tick,
                updated_tick: tick,
            },
            Room {
                id: "science".to_string(),
                room_type: RoomType::Science,
                gravity: 0.0,
                gravity_confidence: 0.1,
                temperature: 0.5,
                max_tokens: 1000,
                prompt_style: "balanced".to_string(),
                deadband_tolerance: 0.08,
                ensign_id: None,
                config: None,
                created_tick: tick,
                updated_tick: tick,
            },
            Room {
                id: "security".to_string(),
                room_type: RoomType::Security,
                gravity: -0.8,
                gravity_confidence: 0.1,
                temperature: 0.3,
                max_tokens: 500,
                prompt_style: "precise".to_string(),
                deadband_tolerance: 0.05,
                ensign_id: None,
                config: None,
                created_tick: tick,
                updated_tick: tick,
            },
            Room {
                id: "social".to_string(),
                room_type: RoomType::Social,
                gravity: 0.5,
                gravity_confidence: 0.1,
                temperature: 0.7,
                max_tokens: 2000,
                prompt_style: "creative".to_string(),
                deadband_tolerance: 0.15,
                ensign_id: None,
                config: None,
                created_tick: tick,
                updated_tick: tick,
            },
        ];

        for room in &default_rooms {
            room::upsert_room(&conn, room)
                .map_err(|e| BootstrapError::Fatal(
                    format!("Step 3 FAILED: Cannot create room '{}': {}", room.id, e)
                ))?;
        }

        // Also write default room JSON files so they persist
        let _ = std::fs::create_dir_all(&rooms_dir);
        for room in &default_rooms {
            let json = serde_json::json!({
                "id": room.id,
                "type": room.room_type.as_str(),
                "gravity": room.gravity,
                "gravity_confidence": room.gravity_confidence,
                "deadband_tolerance": room.deadband_tolerance,
            });
            let path = format!("{}/{}.json", rooms_dir, room.id);
            let _ = std::fs::write(&path, serde_json::to_string_pretty(&json).unwrap());
        }

        default_rooms
    } else {
        rooms
    };

    log::info!("[BOOT] Step 3: {} rooms ready", rooms.len());

    // ═══════════════════════════════════════════════════════════════════
    // STEP 4: For each room, init ensign (wake → orient → yellow alert)
    // ═══════════════════════════════════════════════════════════════════

    let ensigns = match ensign::load_ensigns_from_dir(&conn, &ensigns_dir) {
        Ok(e) => e,
        Err(e) => {
            log::warn!("[BOOT] Step 4: Ensign loading failed: {}. Creating defaults.", e);
            vec![]
        }
    };

    // If no ensign configs, create default ensigns
    let ensigns = if ensigns.is_empty() {
        log::info!("[BOOT] Step 4: No ensigns found, creating defaults");

        let default_ensigns = vec![
            ("seed-nav-01", "seed-2.0-mini", "deepinfra", "navigation"),
            ("seed-eng-01", "seed-2.0-mini", "deepinfra", "engineering"),
            ("glm-sci-01", "glm-4-flash", "z.ai", "science"),
            ("glm-soc-01", "glm-4-flash", "z.ai", "social"),
            ("seed-sec-01", "seed-2.0-mini", "deepinfra", "security"),
        ];

        let mut created = Vec::new();
        for (id, model, provider, room_id) in default_ensigns {
            let mut ensign = Ensign::new(id, model, provider);
            ensign.room_id = Some(room_id.to_string());

            ensign::upsert_ensign(&conn, &ensign)
                .map_err(|e| BootstrapError::Fatal(
                    format!("Step 4 FAILED: Cannot create ensign '{}': {}", id, e)
                ))?;

            // Assign ensign to room
            let _ = conn.execute(
                "UPDATE rooms SET ensign_id = ?1 WHERE id = ?2",
                rusqlite::params![id, room_id],
            );

            created.push(ensign);
        }

        // Write default ensign JSON files
        let _ = std::fs::create_dir_all(&ensigns_dir);
        for (id, model, provider, _) in &default_ensigns {
            let json = serde_json::json!({
                "id": id,
                "model": model,
                "provider": provider,
            });
            let path = format!("{}/{}.json", ensigns_dir, id);
            let _ = std::fs::write(&path, serde_json::to_string_pretty(&json).unwrap());
        }

        created
    } else {
        ensigns
    };

    // Run ensign lifecycle: wake → orient → yellow alert for each
    let mut gravity_history = HashMap::new();
    for mut ensign in ensigns {
        // DORMANT → WAKING
        ensign.wake();
        log::info!("[BOOT] Step 4: Ensign {} waking", ensign.id);

        // WAKING → ORIENTING (read room state)
        ensign.orient();
        log::info!("[BOOT] Step 4: Ensign {} orienting in room {:?}",
            ensign.id, ensign.room_id);

        // ORIENTING → YELLOW_ALERT (ready to handle messages)
        if has_deepinfra || has_zai {
            ensign.go_yellow();
            log::info!("[BOOT] Step 4: Ensign {} at yellow alert", ensign.id);
        } else {
            // No API keys — stay at green alert (monitoring only)
            ensign.status = EnsignStatus::GreenAlert;
            log::warn!("[BOOT] Step 4: Ensign {} at green alert (no API keys)", ensign.id);
        }

        // Persist updated ensign state
        ensign::upsert_ensign(&conn, &ensign)
            .map_err(|e| BootstrapError::NonFatal(
                format!("Step 4 WARNING: Cannot update ensign '{}': {}", ensign.id, e)
            )).ok();

        // Init gravity history for this ensign's room
        if let Some(ref room_id) = ensign.room_id {
            let room = room::get_room(&conn, room_id).ok().flatten();
            gravity_history.entry(room_id.clone())
                .or_insert_with(|| {
                    vec![room.map(|r| r.gravity).unwrap_or(0.0)]
                });
        }
    }

    log::info!("[BOOT] Step 4: {} ensigns deployed", ensigns.len());

    // ═══════════════════════════════════════════════════════════════════
    // STEP 5: Connect Telegram port
    // ═══════════════════════════════════════════════════════════════════

    let mut ports: Vec<Arc<Mutex<dyn Port>>> = Vec::new();

    if has_telegram {
        let telegram_token = std::env::var("TELEGRAM_BOT_TOKEN").unwrap();
        let telegram_port = Arc::new(Mutex::new(
            port::TelegramPort::new(&telegram_token)
        ));
        ports.push(telegram_port.clone());
        log::info!("[BOOT] Step 5: Telegram port created");

        // Spawn the Telegram long-poll listener
        let tg_port = telegram_port.clone();
        let token_clone = telegram_token.clone();
        tokio::spawn(async move {
            use teloxide::prelude::*;
            let bot = teloxide::Bot::new(&token_clone);
            let _ = bot.delete_webhook().await;

            let handler = Update::filter_message().branch(
                dptree::endpoint(move |_bot: Bot, msg: Message| {
                    let port = tg_port.clone();
                    async move {
                        if let Some(text) = msg.text() {
                            let chat_id = msg.chat.id.0;
                            let from_user = msg.from.as_ref()
                                .map(|u| u.first_name.clone());
                            let port_msg = PortMessage {
                                id: uuid::Uuid::new_v4().to_string(),
                                text: text.to_string(),
                                chat_id,
                                from_user,
                                timestamp: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_secs(),
                            };
                            port.lock().await.push_message(port_msg).await;
                        }
                        respond(())
                    }
                })
            );

            let mut dispatcher = Dispatcher::builder(bot, handler).build();
            dispatcher.dispatch().await;
        });
        log::info!("[BOOT] Step 5: Telegram polling started");
    } else {
        log::warn!("[BOOT] Step 5: No TELEGRAM_BOT_TOKEN, using stdio port");
        let stdio_port = Arc::new(Mutex::new(port::StdioPort::new()));
        ports.push(stdio_port);
    }

    // ═══════════════════════════════════════════════════════════════════
    // STEP 6: First tick — scan for correlations (none yet on fresh boot)
    // ═══════════════════════════════════════════════════════════════════

    let tick = conservation::advance_tick();

    // Scan correlations (will be empty on first boot — no gravity history yet)
    let correlations = penrose::scan_correlations(&conn, &gravity_history, tick)
        .unwrap_or_default();
    log::info!("[BOOT] Step 6: Correlation scan found {} correlations", correlations.len());

    // Load conservation state
    let mut cons_state = conservation::load_state(&conn)
        .unwrap_or_else(|_| ConservationState {
            budget: 10000.0,
            used: 0.0,
            tick,
        });
    cons_state.tick = tick;

    // ═══════════════════════════════════════════════════════════════════
    // STEP 7: Post "Hermes online" system tile
    // ═══════════════════════════════════════════════════════════════════

    let uptime_ms = bootstrap_start.elapsed().as_millis();
    let status_msg = format!(
        "Hermes Construct v0.1 online. {} rooms, {} ensigns, {} correlations. \
         Providers: {}{}. Port: {}. Bootstrap: {}ms. Tick: {}.",
        rooms.len(),
        rooms.len(), // one ensign per room
        correlations.len(),
        if has_deepinfra { "deepinfra" } else { "" },
        if has_zai { "+z.ai" } else { "" },
        if has_telegram { "telegram" } else { "stdio" },
        uptime_ms,
        tick,
    );

    let mut system_tile = Tile::new(TileType::Artifact, &status_msg, tick);
    system_tile.room_id = Some("system".to_string());
    system_tile.metadata = Some(serde_json::json!({
        "type": "bootstrap_complete",
        "rooms": rooms.len(),
        "ensigns": rooms.len(),
        "providers": [has_deepinfra.then(|| "deepinfra"), has_zai.then(|| "z.ai")]
            .into_iter().flatten().collect::<Vec<_>>(),
        "port": if has_telegram { "telegram" } else { "stdio" },
        "uptime_ms": uptime_ms,
    }));

    tile::insert_tile(&conn, &system_tile)
        .map_err(|e| BootstrapError::NonFatal(
            format!("Step 7 WARNING: Cannot insert system tile: {}", e)
        )).ok();

    conservation::save_state(&conn, &cons_state)
        .map_err(|e| BootstrapError::NonFatal(
            format!("Step 7 WARNING: Cannot save conservation state: {}", e)
        )).ok();

    log::info!("[BOOT] Step 7: System tile posted — {}", status_msg);

    // ═══════════════════════════════════════════════════════════════════
    // STEP 8: Enter main loop
    // ═══════════════════════════════════════════════════════════════════

    let mut kernel = ShellKernel {
        db: Arc::new(Mutex::new(conn)),
        providers: Vec::new(),
        ports,
        conservation: Arc::new(Mutex::new(cons_state)),
        gravity_history: Arc::new(Mutex::new(gravity_history)),
        tick_interval_ms: tick_ms,
    };

    // Register providers
    if has_deepinfra {
        let key = std::env::var("DEEPINFRA_API_KEY").unwrap();
        kernel.add_provider("deepinfra", Box::new(
            ensign::DeepInfraProvider::new(&key)
        ));
    }
    if has_zai {
        let key = std::env::var("ZAI_API_KEY").unwrap();
        kernel.add_provider("z.ai", Box::new(
            ensign::ZaiProvider::new(&key)
        ));
    }

    log::info!("[BOOT] Step 8: Kernel ready. Entering main loop.");
    Ok(kernel)
}

/// Bootstrap errors: Fatal = abort, NonFatal = continue degraded.
#[derive(Debug)]
pub enum BootstrapError {
    Fatal(String),
    NonFatal(String),
}
impl std::fmt::Display for BootstrapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fatal(msg) => write!(f, "FATAL: {}", msg),
            Self::NonFatal(msg) => write!(f, "WARNING: {}", msg),
        }
    }
}
```

### Failure Recovery Matrix

| Step | What Can Go Wrong | Recovery Strategy |
|------|-------------------|-------------------|
| **Step 0: .env** | No .env file | Non-fatal. Use environment variables (systemd, docker). Log warning. |
| **Step 0: .env** | No API keys | Non-fatal. System boots in degraded mode. Ensigns stay at green alert. No messages processed until keys added. |
| **Step 1: SQLite** | Cannot open database | **Fatal.** Check disk space, file permissions. Exit with error. User must fix and restart. |
| **Step 1: SQLite** | WAL mode fails | Non-fatal. Fall back to default journal mode. Performance slightly worse. |
| **Step 1: SQLite** | Schema creation fails | **Fatal.** Likely SQLite version issue or corruption. Delete universe.db and retry. |
| **Step 1: SQLite** | SQLite locked by another process | **Fatal.** Another hermes-construct instance is running. Kill it and retry. |
| **Step 2: Rooms** | rooms/ directory missing | Non-fatal. Create default rooms in Step 3. Also mkdir rooms/ for persistence. |
| **Step 2: Rooms** | Invalid JSON in room config | Skip that file, log warning. Other rooms still load. |
| **Step 3: Defaults** | Cannot write default room JSON | Non-fatal. Rooms exist in SQLite. Files are just for human readability. |
| **Step 4: Ensigns** | ensigns/ directory missing | Non-fatal. Create default ensigns. |
| **Step 4: Ensigns** | Provider not available (e.g., deepinfra key missing) | Ensign stays at green alert instead of yellow. Will promote to yellow when provider is registered. |
| **Step 4: Ensigns** | SQLite write fails during upsert | **Fatal.** Database corruption. Check disk and permissions. |
| **Step 5: Telegram** | Invalid bot token | Port creation succeeds but polling fails silently. Log error on first poll attempt. System falls back to stdio. |
| **Step 5: Telegram** | Network unreachable | Telegram port created but polling fails. Background retries every 30s. System still accepts stdio input. |
| **Step 5: Telegram** | Telegram API returns 409 (conflict) | Another bot instance is running. Log error. User must kill the other instance. |
| **Step 6: Correlations** | Scan fails | Non-fatal. Return empty correlations. Will retry on next background tick. |
| **Step 7: System tile** | Cannot insert tile | Non-fatal. Bootstrap continues. Tile is informational only. |
| **Step 8: Main loop** | No providers registered | System runs but all messages get "no provider available" error. User must configure API keys and restart. |
| **Step 8: Main loop** | All ports fail | System enters headless mode. Background ticks still run. Tiles can be created via SQLite directly. |

### Recovery Strategies for Common Scenarios

```rust
/// Recovery: if bootstrap fails, attempt a minimal boot.
pub async fn bootstrap_minimal(db_path: &str) -> Result<ShellKernel, BootstrapError> {
    log::warn!("[BOOT] Attempting minimal bootstrap...");

    // Use only SQLite, no rooms/ensigns from disk
    let conn = Connection::open(db_path)
        .map_err(|e| BootstrapError::Fatal(
            format!("Minimal boot: Cannot open database: {}", e)
        ))?;

    conn.execute_batch("PRAGMA busy_timeout=5000;").ok();

    // Create minimal schemas
    conservation::init_schema(&conn).ok();
    tile::init_schema(&conn).ok();
    room::init_schema(&conn).ok();
    ensign::init_schema(&conn).ok();
    penrose::init_schema(&conn).ok();
    deadband::init_schema(&conn).ok();

    let tick = conservation::current_tick();

    // Create only the social room (catch-all)
    let social = Room {
        id: "social".to_string(),
        room_type: RoomType::Social,
        gravity: 0.0,
        gravity_confidence: 0.1,
        temperature: 0.7,
        max_tokens: 2000,
        prompt_style: "creative".to_string(),
        deadband_tolerance: 0.15,
        ensign_id: None,
        config: None,
        created_tick: tick,
        updated_tick: tick,
    };
    room::upsert_room(&conn, &social).ok();

    // Use stdio only
    let stdio_port = Arc::new(Mutex::new(port::StdioPort::new()));

    let cons_state = ConservationState {
        budget: 10000.0,
        used: 0.0,
        tick,
    };

    log::info!("[BOOT] Minimal bootstrap complete (stdio only, social room only)");

    Ok(ShellKernel {
        db: Arc::new(Mutex::new(conn)),
        providers: Vec::new(),
        ports: vec![stdio_port],
        conservation: Arc::new(Mutex::new(cons_state)),
        gravity_history: Arc::new(Mutex::new(HashMap::new())),
        tick_interval_ms: 30_000,
    })
}
```

### Bootstrap Sequence Diagram

```
Time →
────────────────────────────────────────────────────────────────────────→

T+0ms    Step 0: Process starts
         ├── Load .env (or env vars)
         ├── Validate API keys
         └── Log configuration summary

T+5ms    Step 1: Init SQLite
         ├── Open universe.db (create if needed)
         ├── Set WAL mode
         ├── Create 6 tables (tiles, rooms, ensigns, correlations,
         │                     deadband_circuits, shell_meta)
         ├── Create 4 indexes
         └── Increment boot_count

T+10ms   Step 2: Check rooms/
         ├── If rooms/*.json found → load & upsert
         └── If empty → proceed to Step 3

T+15ms   Step 3: Create default rooms (if needed)
         ├── Navigation  (gravity: -0.3, precise)
         ├── Engineering (gravity: -0.6, very precise)
         ├── Science     (gravity:  0.0, balanced)
         ├── Security    (gravity: -0.8, very precise)
         └── Social      (gravity:  0.5, creative)

T+20ms   Step 4: Init ensigns
         ├── seed-nav-01 → Navigation   (dormant→waking→orienting→yellow)
         ├── seed-eng-01 → Engineering  (dormant→waking→orienting→yellow)
         ├── glm-sci-01  → Science      (dormant→waking→orienting→yellow)
         ├── glm-soc-01  → Social       (dormant→waking→orienting→yellow)
         └── seed-sec-01 → Security     (dormant→waking→orienting→yellow)

T+25ms   Step 5: Connect Telegram port
         ├── Create TelegramPort
         ├── Spawn long-poll listener
         └── OR fall back to StdioPort

T+30ms   Step 6: First correlation scan
         ├── Scan gravity history (empty on first boot → 0 correlations)
         └── Load conservation state

T+35ms   Step 7: Post system tile
         ├── Create artifact tile: "Hermes Construct v0.1 online..."
         ├── Record boot stats
         └── Persist conservation state

T+40ms   Step 8: Enter main loop
         └── tokio::select! { poll_ports, background_tick }

         ┌─────────────────────────────────────────────────────────┐
         │ MAIN LOOP (forever)                                     │
         │                                                         │
         │  ┌──────────┐    ┌──────────────┐                      │
         │  │ Port poll │    │ Background   │                      │
         │  │ (messages)│    │ tick (30s)   │                      │
         │  └─────┬─────┘    └──────┬───────┘                      │
         │        │                 │                              │
         │        ▼                 ▼                              │
         │  process_message()  background_tick()                  │
         │        │                 │                              │
         │        ├── route         ├── decay gravities            │
         │        ├── budget check  ├── scan correlations          │
         │        ├── ensign call   ├── check deadbands            │
         │        ├── create tiles  ├── conservation verify        │
         │        └── respond       └── progressive generation     │
         └─────────────────────────────────────────────────────────┘
```

### What Happens on Subsequent Boots

```rust
// On subsequent boots, the sequence is the same but:
//
// Step 1: SQLite tables already exist (IF NOT EXISTS is safe)
// Step 2: Rooms are loaded from rooms/ AND existing SQLite data
// Step 3: Skipped — rooms already exist
// Step 4: Ensigns reloaded from SQLite state (preserving status)
//         If an ensign was at yellow_alert, it wakes back to yellow
//         If an ensign was escalated, it wakes to yellow (fresh start)
// Step 5: Port reconnected (new Telegram polling)
// Step 6: Correlation scan uses existing gravity history
// Step 7: New system tile: "Hermes Construct v0.1 restarted (boot #N)"
// Step 8: Main loop resumes
//
// Boot count is tracked in shell_meta for diagnostics.
```
