//! main.rs — Binary entry point
//!
//! Load .env, init SQLite, start Telegram polling, run main loop.

mod conservation;
mod onboarding;
mod deadband;
mod ensign;
mod gravity;
mod kernel;
mod module;
mod penrose;
mod port;
mod room;
mod spectral;
mod tile;

use std::sync::Arc;
use tokio::sync::Mutex;

use kernel::ShellKernel;
use port::PortMessage;

#[tokio::main]
async fn main() {
    // Init logger
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .init();

    log::info!("hermes-construct v0.1 starting...");

    // Load .env (API keys loaded here, never exposed to agent logic)
    if let Err(e) = dotenvy::dotenv() {
        log::warn!("No .env file found: {}", e);
    }

    let db_path = std::env::var("HERMES_DB_PATH").unwrap_or_else(|_| "universe.db".to_string());
    let rooms_dir = std::env::var("HERMES_ROOMS_DIR").unwrap_or_else(|_| "rooms".to_string());
    let ensigns_dir = std::env::var("HERMES_ENSIGNS_DIR").unwrap_or_else(|_| "ensigns".to_string());
    let tick_ms: u64 = std::env::var("HERMES_TICK_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(30_000);

    // Bootstrap the kernel
    let mut kernel = match ShellKernel::bootstrap(&db_path, &rooms_dir, &ensigns_dir).await {
        Ok(k) => k,
        Err(e) => {
            log::error!("Bootstrap failed: {}", e);
            std::process::exit(1);
        }
    };

    kernel.tick_interval_ms = tick_ms;

    // Set up providers (API keys from .env, NEVER accessible to agent logic)
    let deepinfra_key = std::env::var("DEEPINFRA_API_KEY").unwrap_or_default();
    let zai_key = std::env::var("ZAI_API_KEY").unwrap_or_default();

    if !deepinfra_key.is_empty() {
        kernel.add_provider("deepinfra", Box::new(ensign::DeepInfraProvider::new(&deepinfra_key)));
        log::info!("DeepInfra provider registered");
    } else {
        log::warn!("DEEPINFRA_API_KEY not set, DeepInfra provider unavailable");
    }

    if !zai_key.is_empty() {
        kernel.add_provider("z.ai", Box::new(ensign::ZaiProvider::new(&zai_key)));
        log::info!("z.ai provider registered");
    } else {
        log::warn!("ZAI_API_KEY not set, z.ai provider unavailable");
    }

    // Set up Telegram port
    let telegram_token = std::env::var("TELEGRAM_BOT_TOKEN").unwrap_or_default();

    if !telegram_token.is_empty() {
        let telegram_port = Arc::new(Mutex::new(port::TelegramPort::new(&telegram_token)));
        kernel.add_port(telegram_port.clone());

        log::info!("Starting Telegram polling...");

        // Spawn the Telegram long-poll listener
        let tg_port = telegram_port.clone();
        tokio::spawn(async move {
            use teloxide::prelude::*;

            let bot = teloxide::Bot::new(&telegram_token);

            // Clear any pending updates
            let _ = bot.delete_webhook().await;

            let handler = Update::filter_message().branch(
                dptree::endpoint(move |_bot: Bot, msg: Message| {
                    let port = tg_port.clone();
                    async move {
                        if let Some(text) = msg.text() {
                            let chat_id = msg.chat.id.0;
                            let from_user = msg.from.as_ref().map(|u| u.first_name.clone());

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
    } else {
        log::warn!("TELEGRAM_BOT_TOKEN not set, using stdio port");
        let stdio_port = Arc::new(Mutex::new(port::StdioPort::new()));
        kernel.add_port(stdio_port.clone());

        // Feed stdin lines into the stdio port so the loop has local input.
        let stdin_port = stdio_port.clone();
        tokio::spawn(async move {
            use tokio::io::AsyncBufReadExt;
            let reader = tokio::io::BufReader::new(tokio::io::stdin());
            let mut lines = reader.lines();
            log::info!("stdio port ready — type a message and press Enter (Ctrl+C to quit).");
            while let Ok(Some(line)) = lines.next_line().await {
                let text = line.trim().to_string();
                if text.is_empty() {
                    continue;
                }
                let port_msg = PortMessage {
                    id: uuid::Uuid::new_v4().to_string(),
                    text,
                    chat_id: 0,
                    from_user: Some("stdio".to_string()),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                };
                stdin_port.lock().await.push_message(port_msg).await;
            }
        });
    }

    // --- Module system bootstrap ---
    {
        use module::{ModuleRegistry, ModuleContext, StubModule, Capability, AutoLoader};

        let mut registry = ModuleRegistry::new();

        registry.register(Box::new(StubModule::new(
            "crackle-runtime",
            "0.1.0",
            vec![Capability::PatternDetection, Capability::AnomalyDetection],
            vec!["pattern".into(), "crackle".into()],
        )));
        registry.register(Box::new(StubModule::new(
            "conservation-checker",
            "0.1.0",
            vec![Capability::ConservationTracking],
            vec!["conservation".into(), "budget".into()],
        )));
        registry.register(Box::new(StubModule::new(
            "cathedral-probe",
            "0.1.0",
            vec![Capability::TopologyAnalysis, Capability::SpectralAnalysis],
            vec!["topology".into(), "cathedral".into()],
        )));
        registry.register(Box::new(StubModule::new(
            "negative-space-testing",
            "0.1.0",
            vec![Capability::ForbiddenBehavior, Capability::OutputZoning],
            vec!["negative".into(), "forbidden".into()],
        )));
        registry.register(Box::new(StubModule::new(
            "spacemap",
            "0.1.0",
            vec![Capability::AnomalyDetection],
            vec!["map".into(), "space".into()],
        )));

        log::info!("Module registry: {} modules registered", registry.registered_count());

        // Demo: autoload a sample task to verify the pipeline works
        let ctx = ModuleContext::new("bootstrap", 1.0, &[]);
        let al = AutoLoader::new();
        let loaded = al.analyze_and_load("analyze space topology anomalies", &mut registry, &ctx);
        log::info!("Auto-loaded modules for bootstrap task: {:?}", loaded);
    }
    // --- End module system bootstrap ---

    log::info!("Hermes Construct v0.1 running. Ctrl+C to stop.");

    // Run the main loop
    if let Err(e) = kernel.run().await {
        log::error!("Kernel error: {}", e);
        std::process::exit(1);
    }
}
