#![allow(dead_code)]
//! room.rs — Room loading from JSON files, state management, gravity updates

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::gravity::{self, ModelParams};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RoomType {
    Navigation,
    Engineering,
    Science,
    Security,
    Social,
    Custom(String),
}

impl RoomType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Navigation => "navigation",
            Self::Engineering => "engineering",
            Self::Science => "science",
            Self::Security => "security",
            Self::Social => "social",
            Self::Custom(s) => s,
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "navigation" => Some(Self::Navigation),
            "engineering" => Some(Self::Engineering),
            "science" => Some(Self::Science),
            "security" => Some(Self::Security),
            "social" => Some(Self::Social),
            _ => Some(Self::Custom(s.to_string())),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub id: String,
    pub room_type: RoomType,
    pub gravity: f64,
    pub gravity_confidence: f64,
    pub temperature: f64,
    pub max_tokens: u32,
    pub prompt_style: String,
    pub deadband_tolerance: f64,
    pub ensign_id: Option<String>,
    pub config: Option<serde_json::Value>,
    pub created_tick: u64,
    pub updated_tick: u64,
}

impl Room {
    /// Get the current model params derived from this room's gravity
    pub fn model_params(&self) -> ModelParams {
        gravity::gravity_to_params(self.gravity)
    }

    /// Update gravity with decay toward 0.0
    pub fn decay_gravity(&mut self, decay_rate: f64, tick: u64) {
        self.gravity *= 1.0 - decay_rate;
        // Clamp
        self.gravity = self.gravity.clamp(-1.0, 1.0);
        self.updated_tick = tick;
    }

    /// Nudge gravity based on interaction signal
    pub fn nudge_gravity(&mut self, signal: f64, learning_rate: f64, tick: u64) {
        self.gravity += signal * learning_rate;
        self.gravity = self.gravity.clamp(-1.0, 1.0);
        // Increase confidence slightly
        self.gravity_confidence = (self.gravity_confidence + 0.01).min(1.0);
        self.updated_tick = tick;
    }
}

// ---------------------------------------------------------------------------
// JSON room config (loaded from files)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct RoomConfig {
    pub id: String,
    #[serde(rename = "type")]
    pub room_type: String,
    #[serde(default)]
    pub gravity: f64,
    #[serde(default = "default_confidence")]
    pub gravity_confidence: f64,
    #[serde(default = "default_tolerance")]
    pub deadband_tolerance: f64,
    #[serde(default)]
    pub ensign_id: Option<String>,
    #[serde(default)]
    pub config: Option<serde_json::Value>,
}

fn default_confidence() -> f64 { 0.0 }
fn default_tolerance() -> f64 { 0.1 }

// ---------------------------------------------------------------------------
// SQLite schema + CRUD
// ---------------------------------------------------------------------------

pub fn init_schema(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS rooms (
            id TEXT PRIMARY KEY,
            room_type TEXT NOT NULL,
            gravity REAL DEFAULT 0.0,
            gravity_confidence REAL DEFAULT 0.0,
            temperature REAL DEFAULT 0.7,
            max_tokens INTEGER DEFAULT 2000,
            prompt_style TEXT DEFAULT 'conversational',
            deadband_tolerance REAL DEFAULT 0.1,
            ensign_id TEXT,
            config TEXT,
            created_tick INTEGER NOT NULL,
            updated_tick INTEGER NOT NULL
        );"
    )
}

pub fn upsert_room(conn: &Connection, room: &Room) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO rooms (id, room_type, gravity, gravity_confidence, temperature,
         max_tokens, prompt_style, deadband_tolerance, ensign_id, config,
         created_tick, updated_tick)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
         ON CONFLICT(id) DO UPDATE SET
            gravity = excluded.gravity,
            gravity_confidence = excluded.gravity_confidence,
            temperature = excluded.temperature,
            max_tokens = excluded.max_tokens,
            prompt_style = excluded.prompt_style,
            deadband_tolerance = excluded.deadband_tolerance,
            ensign_id = excluded.ensign_id,
            config = excluded.config,
            updated_tick = excluded.updated_tick",
        params![
            room.id,
            room.room_type.as_str(),
            room.gravity,
            room.gravity_confidence,
            room.temperature,
            room.max_tokens as i64,
            room.prompt_style,
            room.deadband_tolerance,
            room.ensign_id,
            room.config.as_ref().map(|v| v.to_string()),
            room.created_tick as i64,
            room.updated_tick as i64,
        ],
    )?;
    Ok(())
}

pub fn get_room(conn: &Connection, id: &str) -> Result<Option<Room>, rusqlite::Error> {
    let result = conn.query_row(
        "SELECT id, room_type, gravity, gravity_confidence, temperature,
                max_tokens, prompt_style, deadband_tolerance, ensign_id,
                config, created_tick, updated_tick
         FROM rooms WHERE id = ?1",
        params![id],
        |row| Ok(row_to_room(row)),
    );

    match result {
        Ok(room) => Ok(Some(room)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

pub fn get_all_rooms(conn: &Connection) -> Result<Vec<Room>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, room_type, gravity, gravity_confidence, temperature,
                max_tokens, prompt_style, deadband_tolerance, ensign_id,
                config, created_tick, updated_tick
         FROM rooms"
    )?;

    let rooms = stmt.query_map([], |row| Ok(row_to_room(row)))?;
    rooms.collect()
}

/// Load rooms from JSON config files and upsert into SQLite
pub fn load_rooms_from_dir(
    conn: &Connection,
    dir: &str,
    tick: u64,
) -> Result<Vec<Room>, String> {
    let mut rooms = Vec::new();

    if !Path::new(dir).exists() {
        log::warn!("Rooms directory {} does not exist, skipping", dir);
        return Ok(rooms);
    }

    let entries = fs::read_dir(dir).map_err(|e| format!("read rooms dir: {}", e))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e == "json").unwrap_or(false) {
            let content = fs::read_to_string(&path)
                .map_err(|e| format!("read {}: {}", path.display(), e))?;
            let config: RoomConfig = serde_json::from_str(&content)
                .map_err(|e| format!("parse {}: {}", path.display(), e))?;

            let params = gravity::gravity_to_params(config.gravity);

            let room = Room {
                id: config.id.clone(),
                room_type: RoomType::from_str(&config.room_type)
                    .unwrap_or(RoomType::Custom(config.room_type.clone())),
                gravity: config.gravity,
                gravity_confidence: config.gravity_confidence,
                temperature: params.temperature,
                max_tokens: params.max_tokens,
                prompt_style: params.prompt_style,
                deadband_tolerance: config.deadband_tolerance,
                ensign_id: config.ensign_id,
                config: config.config,
                created_tick: tick,
                updated_tick: tick,
            };

            upsert_room(conn, &room)
                .map_err(|e| format!("upsert room {}: {}", room.id, e))?;

            rooms.push(room);
        }
    }

    Ok(rooms)
}

/// Route a message to the best matching room based on gravity and context
///
// Current: keyword-based routing. Simple and deterministic.
// Planned: embedding-based semantic routing using JEPA representations.
// The keyword approach is intentional for v0.1 — it's debuggable and predictable.
pub fn route_to_room(conn: &Connection, message: &str) -> Result<Option<Room>, rusqlite::Error> {
    let rooms = get_all_rooms(conn)?;
    if rooms.is_empty() {
        return Ok(None);
    }

    let msg_lower = message.to_lowercase();

    // Keyword-based routing
    let best = if msg_lower.contains("navigate") || msg_lower.contains("direction") || msg_lower.contains("where") {
        rooms.iter().find(|r| r.room_type == RoomType::Navigation)
    } else if msg_lower.contains("build") || msg_lower.contains("code") || msg_lower.contains("fix") || msg_lower.contains("debug") {
        rooms.iter().find(|r| r.room_type == RoomType::Engineering)
    } else if msg_lower.contains("research") || msg_lower.contains("analyze") || msg_lower.contains("science") {
        rooms.iter().find(|r| r.room_type == RoomType::Science)
    } else if msg_lower.contains("security") || msg_lower.contains("safe") || msg_lower.contains("protect") {
        rooms.iter().find(|r| r.room_type == RoomType::Security)
    } else {
        // Default: social or first room
        rooms.iter().find(|r| r.room_type == RoomType::Social)
            .or_else(|| rooms.first())
    };

    Ok(best.cloned())
}

fn row_to_room(row: &rusqlite::Row<'_>) -> Room {
    let type_str: String = row.get(1).unwrap_or_default();
    let config_str: Option<String> = row.get(9).unwrap_or(None);

    Room {
        id: row.get(0).unwrap_or_default(),
        room_type: RoomType::from_str(&type_str).unwrap_or(RoomType::Custom(type_str)),
        gravity: row.get(2).unwrap_or(0.0),
        gravity_confidence: row.get(3).unwrap_or(0.0),
        temperature: row.get(4).unwrap_or(0.7),
        max_tokens: row.get::<_, i64>(5).unwrap_or(2000) as u32,
        prompt_style: row.get(6).unwrap_or_else(|_| "conversational".to_string()),
        deadband_tolerance: row.get(7).unwrap_or(0.1),
        ensign_id: row.get(8).unwrap_or(None),
        config: config_str.and_then(|s| serde_json::from_str(&s).ok()),
        created_tick: row.get::<_, i64>(10).unwrap_or(0) as u64,
        updated_tick: row.get::<_, i64>(11).unwrap_or(0) as u64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn room_type_roundtrip() {
        for rt in &[RoomType::Navigation, RoomType::Engineering, RoomType::Science, RoomType::Security, RoomType::Social] {
            assert_eq!(RoomType::from_str(rt.as_str()), Some(rt.clone()));
        }
    }

    #[test]
    fn room_type_custom() {
        assert_eq!(RoomType::from_str("kitchen"), Some(RoomType::Custom("kitchen".into())));
        let c = RoomType::Custom("kitchen".into());
        assert_eq!(c.as_str(), "kitchen");
    }

    #[test]
    fn room_model_params() {
        let room = Room {
            id: "r1".into(), room_type: RoomType::Engineering, gravity: -0.8,
            gravity_confidence: 0.5, temperature: 0.3, max_tokens: 500,
            prompt_style: "precise".into(), deadband_tolerance: 0.1, ensign_id: None,
            config: None, created_tick: 0, updated_tick: 0,
        };
        let p = room.model_params();
        assert_eq!(p.prompt_style, "precise");
    }

    #[test]
    fn decay_gravity_toward_zero() {
        let mut room = Room {
            id: "r1".into(), room_type: RoomType::Science, gravity: 0.5,
            gravity_confidence: 0.5, temperature: 0.7, max_tokens: 2000,
            prompt_style: "creative".into(), deadband_tolerance: 0.1, ensign_id: None,
            config: None, created_tick: 0, updated_tick: 0,
        };
        room.decay_gravity(0.1, 10);
        assert!((room.gravity - 0.45).abs() < 1e-9);
        assert_eq!(room.updated_tick, 10);
    }

    #[test]
    fn nudge_gravity_positive() {
        let mut room = Room {
            id: "r1".into(), room_type: RoomType::Social, gravity: 0.0,
            gravity_confidence: 0.5, temperature: 0.7, max_tokens: 2000,
            prompt_style: "creative".into(), deadband_tolerance: 0.1, ensign_id: None,
            config: None, created_tick: 0, updated_tick: 0,
        };
        room.nudge_gravity(0.5, 0.1, 5);
        assert!((room.gravity - 0.05).abs() < 1e-9);
        assert_eq!(room.updated_tick, 5);
    }

    #[test]
    fn nudge_gravity_clamps() {
        let mut room = Room {
            id: "r1".into(), room_type: RoomType::Navigation, gravity: 0.9,
            gravity_confidence: 0.5, temperature: 0.7, max_tokens: 2000,
            prompt_style: "narrative".into(), deadband_tolerance: 0.1, ensign_id: None,
            config: None, created_tick: 0, updated_tick: 0,
        };
        room.nudge_gravity(5.0, 0.5, 1);
        assert!((room.gravity - 1.0).abs() < 1e-9);
    }

    #[test]
    fn upsert_and_get_room() {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();
        let room = Room {
            id: "test-room".into(), room_type: RoomType::Engineering,
            gravity: 0.3, gravity_confidence: 0.8, temperature: 0.7,
            max_tokens: 2000, prompt_style: "creative".into(),
            deadband_tolerance: 0.1, ensign_id: None, config: None,
            created_tick: 1, updated_tick: 1,
        };
        upsert_room(&conn, &room).unwrap();
        let loaded = get_room(&conn, "test-room").unwrap().unwrap();
        assert_eq!(loaded.id, "test-room");
        assert_eq!(loaded.room_type, RoomType::Engineering);
        assert!((loaded.gravity - 0.3).abs() < 1e-9);
    }

    #[test]
    fn get_room_missing() {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();
        assert!(get_room(&conn, "nope").unwrap().is_none());
    }

    #[test]
    fn route_to_room_navigation() {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();
        let room = Room {
            id: "nav1".into(), room_type: RoomType::Navigation, gravity: 0.0,
            gravity_confidence: 0.5, temperature: 0.5, max_tokens: 1000,
            prompt_style: "balanced".into(), deadband_tolerance: 0.1, ensign_id: None,
            config: None, created_tick: 0, updated_tick: 0,
        };
        upsert_room(&conn, &room).unwrap();
        let found = route_to_room(&conn, "where should I navigate?").unwrap().unwrap();
        assert_eq!(found.room_type, RoomType::Navigation);
    }

    #[test]
    fn route_to_room_empty() {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();
        assert!(route_to_room(&conn, "hello").unwrap().is_none());
    }
}
