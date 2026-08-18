#![allow(dead_code)]
//! ensign.rs — Ensign lifecycle and provider abstraction
//!
//! An Ensign is a small model (Seed-mini, GLM-flash) that maintains a room.
//! Lifecycle: Dormant → Waking → Orienting → YellowAlert → (handling) → StandingDown
//!
//! Provider trait abstracts over DeepInfra, z.ai, etc.

use async_trait::async_trait;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::gravity::ModelParams;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EnsignStatus {
    Dormant,
    Waking,
    Orienting,
    GreenAlert,
    YellowAlert,
    RedAlert,
    StandingDown,
    Escalated,
}

impl EnsignStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Dormant => "dormant",
            Self::Waking => "waking",
            Self::Orienting => "orienting",
            Self::GreenAlert => "green_alert",
            Self::YellowAlert => "yellow_alert",
            Self::RedAlert => "red_alert",
            Self::StandingDown => "standing_down",
            Self::Escalated => "escalated",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "dormant" => Some(Self::Dormant),
            "waking" => Some(Self::Waking),
            "orienting" => Some(Self::Orienting),
            "green_alert" => Some(Self::GreenAlert),
            "yellow_alert" => Some(Self::YellowAlert),
            "red_alert" => Some(Self::RedAlert),
            "standing_down" => Some(Self::StandingDown),
            "escalated" => Some(Self::Escalated),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AlertLevel {
    Green,
    Yellow,
    Red,
}

impl AlertLevel {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Green => "green",
            Self::Yellow => "yellow",
            Self::Red => "red",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ensign {
    pub id: String,
    pub model_type: String,
    pub model_name: String,
    pub provider: String,
    pub room_id: Option<String>,
    pub status: EnsignStatus,
    pub alert_level: AlertLevel,
    pub energy_budget: f64,
    pub energy_used: f64,
    pub call_count: u32,
    pub config: Option<serde_json::Value>,
}

impl Ensign {
    pub fn new(id: &str, model_name: &str, provider: &str) -> Self {
        Self {
            id: id.to_string(),
            model_type: "remote_light".to_string(),
            model_name: model_name.to_string(),
            provider: provider.to_string(),
            room_id: None,
            status: EnsignStatus::Dormant,
            alert_level: AlertLevel::Green,
            energy_budget: 100.0,
            energy_used: 0.0,
            call_count: 0,
            config: None,
        }
    }

    pub fn wake(&mut self) {
        if self.status == EnsignStatus::Dormant {
            self.status = EnsignStatus::Waking;
        }
    }

    pub fn orient(&mut self) {
        if self.status == EnsignStatus::Waking {
            self.status = EnsignStatus::Orienting;
        }
    }

    pub fn go_yellow(&mut self) {
        self.status = EnsignStatus::YellowAlert;
        self.alert_level = AlertLevel::Yellow;
    }

    pub fn go_red(&mut self) {
        self.status = EnsignStatus::RedAlert;
        self.alert_level = AlertLevel::Red;
    }

    pub fn stand_down(&mut self) {
        self.status = EnsignStatus::StandingDown;
        self.alert_level = AlertLevel::Green;
    }

    pub fn can_handle(&self) -> bool {
        matches!(self.status, EnsignStatus::YellowAlert | EnsignStatus::GreenAlert)
    }

    pub fn record_call(&mut self, energy_cost: f64) {
        self.call_count += 1;
        self.energy_used += energy_cost;
    }
}

// ---------------------------------------------------------------------------
// Provider trait — abstracts over API backends
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct CompletionRequest {
    pub prompt: String,
    pub model: String,
    pub params: ModelParams,
    pub system_prompt: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CompletionResponse {
    pub text: String,
    pub model: String,
    pub tokens_used: u32,
    pub provider: String,
}

#[async_trait]
pub trait Provider: Send + Sync {
    /// Complete a prompt using the given model parameters
    async fn complete(&self, request: &CompletionRequest) -> Result<CompletionResponse, String>;

    /// Provider name
    fn name(&self) -> &str;
}

// ---------------------------------------------------------------------------
// DeepInfra provider
// TODO: Wire to real HTTP client with retry, timeout, and streaming support.
// ---------------------------------------------------------------------------

pub struct DeepInfraProvider {
    client: reqwest::Client,
    api_key: String,
    base_url: String,
}

impl DeepInfraProvider {
    pub fn new(api_key: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: api_key.to_string(),
            base_url: "https://api.deepinfra.com/v1/openai/chat/completions".to_string(),
        }
    }
}

#[async_trait]
impl Provider for DeepInfraProvider {
    async fn complete(&self, request: &CompletionRequest) -> Result<CompletionResponse, String> {
        let messages = build_messages(&request.system_prompt, &request.prompt);

        let body = serde_json::json!({
            "model": request.model,
            "messages": messages,
            "temperature": request.params.temperature,
            "max_tokens": request.params.max_tokens,
            "top_p": request.params.top_p,
        });

        let resp = self
            .client
            .post(&self.base_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("deepinfra request error: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("deepinfra error {}: {}", status, text));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("deepinfra parse error: {}", e))?;

        let text = json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let tokens = json["usage"]["total_tokens"]
            .as_u64()
            .unwrap_or(0) as u32;

        Ok(CompletionResponse {
            text,
            model: request.model.clone(),
            tokens_used: tokens,
            provider: "deepinfra".to_string(),
        })
    }

    fn name(&self) -> &str {
        "deepinfra"
    }
}

// ---------------------------------------------------------------------------
// z.ai provider
// TODO: Wire to real HTTP client with retry, timeout, and streaming support.
// ---------------------------------------------------------------------------

pub struct ZaiProvider {
    client: reqwest::Client,
    api_key: String,
    base_url: String,
}

impl ZaiProvider {
    pub fn new(api_key: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: api_key.to_string(),
            base_url: "https://api.zai.chat/v1/chat/completions".to_string(),
        }
    }
}

#[async_trait]
impl Provider for ZaiProvider {
    async fn complete(&self, request: &CompletionRequest) -> Result<CompletionResponse, String> {
        let messages = build_messages(&request.system_prompt, &request.prompt);

        let body = serde_json::json!({
            "model": request.model,
            "messages": messages,
            "temperature": request.params.temperature,
            "max_tokens": request.params.max_tokens,
            "top_p": request.params.top_p,
        });

        let resp = self
            .client
            .post(&self.base_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("z.ai request error: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("z.ai error {}: {}", status, text));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("z.ai parse error: {}", e))?;

        let text = json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let tokens = json["usage"]["total_tokens"]
            .as_u64()
            .unwrap_or(0) as u32;

        Ok(CompletionResponse {
            text,
            model: request.model.clone(),
            tokens_used: tokens,
            provider: "z.ai".to_string(),
        })
    }

    fn name(&self) -> &str {
        "z.ai"
    }
}

// ---------------------------------------------------------------------------
// Provider resolution
// ---------------------------------------------------------------------------

fn build_messages(system_prompt: &Option<String>, user_prompt: &str) -> Vec<serde_json::Value> {
    let mut messages = Vec::new();
    if let Some(sys) = system_prompt {
        messages.push(serde_json::json!({
            "role": "system",
            "content": sys
        }));
    }
    messages.push(serde_json::json!({
        "role": "user",
        "content": user_prompt
    }));
    messages
}

/// Resolve a provider by name
pub fn get_provider<'a>(
    name: &str,
    providers: &'a [(String, Box<dyn Provider>)],
) -> Option<&'a dyn Provider> {
    providers.iter().find(|(n, _)| n == name).map(|(_, p)| p.as_ref())
}

// ---------------------------------------------------------------------------
// Ensign config from JSON
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct EnsignConfig {
    pub id: String,
    pub model: String,
    pub provider: String,
}

// ---------------------------------------------------------------------------
// SQLite
// ---------------------------------------------------------------------------

pub fn init_schema(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS ensigns (
            id TEXT PRIMARY KEY,
            model_type TEXT NOT NULL,
            model_name TEXT NOT NULL,
            provider TEXT NOT NULL,
            room_id TEXT,
            status TEXT DEFAULT 'dormant',
            alert_level TEXT DEFAULT 'green',
            energy_budget REAL DEFAULT 100.0,
            energy_used REAL DEFAULT 0.0,
            call_count INTEGER DEFAULT 0,
            config TEXT,
            FOREIGN KEY (room_id) REFERENCES rooms(id)
        );"
    )
}

pub fn upsert_ensign(conn: &Connection, ensign: &Ensign) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO ensigns (id, model_type, model_name, provider, room_id, status,
         alert_level, energy_budget, energy_used, call_count, config)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
         ON CONFLICT(id) DO UPDATE SET
            status = excluded.status,
            alert_level = excluded.alert_level,
            energy_used = excluded.energy_used,
            call_count = excluded.call_count,
            room_id = excluded.room_id",
        params![
            ensign.id,
            ensign.model_type,
            ensign.model_name,
            ensign.provider,
            ensign.room_id,
            ensign.status.as_str(),
            ensign.alert_level.as_str(),
            ensign.energy_budget,
            ensign.energy_used,
            ensign.call_count as i64,
            ensign.config.as_ref().map(|v| v.to_string()),
        ],
    )?;
    Ok(())
}

pub fn get_ensign_for_room(conn: &Connection, room_id: &str) -> Result<Option<Ensign>, rusqlite::Error> {
    let result = conn.query_row(
        "SELECT id, model_type, model_name, provider, room_id, status,
                alert_level, energy_budget, energy_used, call_count, config
         FROM ensigns WHERE room_id = ?1",
        params![room_id],
        |row| Ok(row_to_ensign(row)),
    );

    match result {
        Ok(e) => Ok(Some(e)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

pub fn get_all_ensigns(conn: &Connection) -> Result<Vec<Ensign>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, model_type, model_name, provider, room_id, status,
                alert_level, energy_budget, energy_used, call_count, config
         FROM ensigns"
    )?;
    let ensigns = stmt.query_map([], |row| Ok(row_to_ensign(row)))?;
    ensigns.collect()
}

/// Load ensigns from JSON config files
pub fn load_ensigns_from_dir(
    conn: &Connection,
    dir: &str,
) -> Result<Vec<Ensign>, String> {
    let mut ensigns = Vec::new();

    if !std::path::Path::new(dir).exists() {
        log::warn!("Ensigns directory {} does not exist, skipping", dir);
        return Ok(ensigns);
    }

    let entries = std::fs::read_dir(dir).map_err(|e| format!("read ensigns dir: {}", e))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e == "json").unwrap_or(false) {
            let content = std::fs::read_to_string(&path)
                .map_err(|e| format!("read {}: {}", path.display(), e))?;
            let config: EnsignConfig = serde_json::from_str(&content)
                .map_err(|e| format!("parse {}: {}", path.display(), e))?;

            let ensign = Ensign::new(&config.id, &config.model, &config.provider);
            upsert_ensign(conn, &ensign)
                .map_err(|e| format!("upsert ensign {}: {}", ensign.id, e))?;

            ensigns.push(ensign);
        }
    }

    Ok(ensigns)
}

fn row_to_ensign(row: &rusqlite::Row<'_>) -> Ensign {
    let status_str: String = row.get(5).unwrap_or_default();
    let alert_str: String = row.get(6).unwrap_or_default();
    let config_str: Option<String> = row.get(10).unwrap_or(None);

    Ensign {
        id: row.get(0).unwrap_or_default(),
        model_type: row.get(1).unwrap_or_else(|_| "remote_light".to_string()),
        model_name: row.get(2).unwrap_or_default(),
        provider: row.get(3).unwrap_or_default(),
        room_id: row.get(4).unwrap_or(None),
        status: EnsignStatus::from_str(&status_str).unwrap_or(EnsignStatus::Dormant),
        alert_level: match alert_str.as_str() {
            "yellow" => AlertLevel::Yellow,
            "red" => AlertLevel::Red,
            _ => AlertLevel::Green,
        },
        energy_budget: row.get(7).unwrap_or(100.0),
        energy_used: row.get(8).unwrap_or(0.0),
        call_count: row.get::<_, i64>(9).unwrap_or(0) as u32,
        config: config_str.and_then(|s| serde_json::from_str(&s).ok()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensign_status_roundtrip() {
        for s in &[
            EnsignStatus::Dormant, EnsignStatus::Waking, EnsignStatus::Orienting,
            EnsignStatus::GreenAlert, EnsignStatus::YellowAlert, EnsignStatus::RedAlert,
            EnsignStatus::StandingDown, EnsignStatus::Escalated,
        ] {
            assert_eq!(EnsignStatus::from_str(s.as_str()), Some(s.clone()));
        }
    }

    #[test]
    fn ensign_status_invalid() {
        assert_eq!(EnsignStatus::from_str("nonexistent"), None);
    }

    #[test]
    fn ensign_lifecycle_wake_orient_yellow() {
        let mut e = Ensign::new("e1", "seed-mini", "deepinfra");
        assert_eq!(e.status, EnsignStatus::Dormant);
        assert!(!e.can_handle());
        e.wake();
        assert_eq!(e.status, EnsignStatus::Waking);
        e.orient();
        assert_eq!(e.status, EnsignStatus::Orienting);
        assert!(!e.can_handle());
        e.go_yellow();
        assert_eq!(e.status, EnsignStatus::YellowAlert);
        assert!(e.can_handle());
    }

    #[test]
    fn ensign_go_red_and_stand_down() {
        let mut e = Ensign::new("e2", "glm-flash", "z.ai");
        e.go_red();
        assert_eq!(e.status, EnsignStatus::RedAlert);
        assert_eq!(e.alert_level, AlertLevel::Red);
        e.stand_down();
        assert_eq!(e.status, EnsignStatus::StandingDown);
        assert_eq!(e.alert_level, AlertLevel::Green);
    }

    #[test]
    fn ensign_record_call() {
        let mut e = Ensign::new("e3", "model", "prov");
        e.record_call(5.0);
        e.record_call(3.0);
        assert_eq!(e.call_count, 2);
        assert!((e.energy_used - 8.0).abs() < f64::EPSILON);
    }

    #[test]
    fn ensign_wake_only_from_dormant() {
        let mut e = Ensign::new("e4", "m", "p");
        e.wake();
        e.wake(); // already Waking, no change
        assert_eq!(e.status, EnsignStatus::Waking);
    }

    #[test]
    fn build_messages_with_system_prompt() {
        let msgs = build_messages(&Some("sys".into()), "user");
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0]["role"], "system");
        assert_eq!(msgs[1]["role"], "user");
    }

    #[test]
    fn build_messages_without_system_prompt() {
        let msgs = build_messages(&None, "user");
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0]["role"], "user");
    }

    #[test]
    fn get_provider_finds_matching() {
        struct DummyProvider;
        #[async_trait]
        impl Provider for DummyProvider {
            async fn complete(&self, _req: &CompletionRequest) -> Result<CompletionResponse, String> {
                Ok(CompletionResponse { text: "".into(), model: "".into(), tokens_used: 0, provider: "dummy".into() })
            }
            fn name(&self) -> &str { "dummy" }
        }
        let providers: Vec<(String, Box<dyn Provider>)> = vec![("deepinfra".into(), Box::new(DummyProvider))];
        let p = get_provider("deepinfra", &providers);
        assert!(p.is_some());
        assert_eq!(p.unwrap().name(), "dummy");
    }

    #[test]
    fn get_provider_missing() {
        let providers: Vec<(String, Box<dyn Provider>)> = vec![];
        assert!(get_provider("none", &providers).is_none());
    }

    #[test]
    fn sqlite_upsert_and_get() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::room::init_schema(&conn).unwrap();
        init_schema(&conn).unwrap();
        let e = Ensign::new("ensign-1", "seed-mini", "deepinfra");
        upsert_ensign(&conn, &e).unwrap();
        let loaded = get_all_ensigns(&conn).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, "ensign-1");
        assert_eq!(loaded[0].status, EnsignStatus::Dormant);
    }
}
