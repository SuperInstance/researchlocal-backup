#![allow(dead_code)]
//! tile.rs — Tile types + SQLite CRUD
//!
//! The tile is the fundamental unit of work. Everything is a tile.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TileType {
    Observation,
    Action,
    Thought,
    Delegation,
    Escalation,
    Artifact,
}

impl TileType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Observation => "observation",
            Self::Action => "action",
            Self::Thought => "thought",
            Self::Delegation => "delegation",
            Self::Escalation => "escalation",
            Self::Artifact => "artifact",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "observation" => Some(Self::Observation),
            "action" => Some(Self::Action),
            "thought" => Some(Self::Thought),
            "delegation" => Some(Self::Delegation),
            "escalation" => Some(Self::Escalation),
            "artifact" => Some(Self::Artifact),
            _ => None,
        }
    }
}

impl std::fmt::Display for TileType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TileStatus {
    Active,
    Complete,
    Deadband,
    Escalated,
    Archived,
}

impl TileStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Active => "active",
            Self::Complete => "complete",
            Self::Deadband => "deadband",
            Self::Escalated => "escalated",
            Self::Archived => "archived",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "active" => Some(Self::Active),
            "complete" => Some(Self::Complete),
            "deadband" => Some(Self::Deadband),
            "escalated" => Some(Self::Escalated),
            "archived" => Some(Self::Archived),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tile {
    pub id: String,
    pub room_id: Option<String>,
    pub tile_type: TileType,
    pub parent_id: Option<String>,
    pub status: TileStatus,
    pub content: String,
    pub deadband_lower: Option<f64>,
    pub deadband_upper: Option<f64>,
    pub deadband_current: Option<f64>,
    pub ensign_id: Option<String>,
    pub model_used: Option<String>,
    pub tokens_used: u32,
    pub conservation_delta: f64,
    pub metadata: Option<serde_json::Value>,
    pub created_tick: u64,
    pub updated_tick: u64,
}

impl Tile {
    pub fn new(tile_type: TileType, content: &str, tick: u64) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            room_id: None,
            tile_type,
            parent_id: None,
            status: TileStatus::Active,
            content: content.to_string(),
            deadband_lower: None,
            deadband_upper: None,
            deadband_current: None,
            ensign_id: None,
            model_used: None,
            tokens_used: 0,
            conservation_delta: 0.0,
            metadata: None,
            created_tick: tick,
            updated_tick: tick,
        }
    }

    pub fn with_room(mut self, room_id: &str) -> Self {
        self.room_id = Some(room_id.to_string());
        self
    }

    pub fn with_parent(mut self, parent_id: &str) -> Self {
        self.parent_id = Some(parent_id.to_string());
        self
    }

    pub fn with_ensign(mut self, ensign_id: &str) -> Self {
        self.ensign_id = Some(ensign_id.to_string());
        self
    }

    pub fn complete(&mut self, tick: u64) {
        self.status = TileStatus::Complete;
        self.updated_tick = tick;
    }

    pub fn escalate(&mut self, reason: &str, tick: u64) {
        self.status = TileStatus::Escalated;
        self.updated_tick = tick;
        let mut meta = self.metadata.clone().unwrap_or(serde_json::Value::Null);
        if let serde_json::Value::Null = meta {
            meta = serde_json::json!({});
        }
        if let Some(obj) = meta.as_object_mut() {
            obj.insert(
                "escalation_reason".to_string(),
                serde_json::Value::String(reason.to_string()),
            );
        }
        self.metadata = Some(meta);
    }
}

// ---------------------------------------------------------------------------
// SQLite schema + CRUD
// ---------------------------------------------------------------------------

pub fn init_schema(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS tiles (
            id TEXT PRIMARY KEY,
            room_id TEXT,
            tile_type TEXT NOT NULL,
            parent_id TEXT,
            status TEXT DEFAULT 'active',
            content TEXT NOT NULL,
            deadband_lower REAL,
            deadband_upper REAL,
            deadband_current REAL,
            ensign_id TEXT,
            model_used TEXT,
            tokens_used INTEGER DEFAULT 0,
            conservation_delta REAL DEFAULT 0.0,
            metadata TEXT,
            created_tick INTEGER NOT NULL,
            updated_tick INTEGER NOT NULL,
            FOREIGN KEY (parent_id) REFERENCES tiles(id),
            FOREIGN KEY (room_id) REFERENCES rooms(id)
        );
        CREATE INDEX IF NOT EXISTS idx_tiles_room ON tiles(room_id);
        CREATE INDEX IF NOT EXISTS idx_tiles_type ON tiles(tile_type);
        CREATE INDEX IF NOT EXISTS idx_tiles_status ON tiles(status);
        CREATE INDEX IF NOT EXISTS idx_tiles_tick ON tiles(created_tick);"
    )
}

pub fn insert_tile(conn: &Connection, tile: &Tile) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO tiles (id, room_id, tile_type, parent_id, status, content,
         deadband_lower, deadband_upper, deadband_current, ensign_id, model_used,
         tokens_used, conservation_delta, metadata, created_tick, updated_tick)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
        params![
            tile.id,
            tile.room_id,
            tile.tile_type.as_str(),
            tile.parent_id,
            tile.status.as_str(),
            tile.content,
            tile.deadband_lower,
            tile.deadband_upper,
            tile.deadband_current,
            tile.ensign_id,
            tile.model_used,
            tile.tokens_used as i64,
            tile.conservation_delta,
            tile.metadata.as_ref().map(|v| v.to_string()),
            tile.created_tick as i64,
            tile.updated_tick as i64,
        ],
    )?;
    Ok(())
}

pub fn get_tile(conn: &Connection, id: &str) -> Result<Option<Tile>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, room_id, tile_type, parent_id, status, content,
                deadband_lower, deadband_upper, deadband_current,
                ensign_id, model_used, tokens_used, conservation_delta,
                metadata, created_tick, updated_tick
         FROM tiles WHERE id = ?1"
    )?;

    let result = stmt.query_row(params![id], |row| {
        Ok(row_to_tile(row))
    });

    match result {
        Ok(tile) => Ok(Some(tile)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

pub fn query_tiles(
    conn: &Connection,
    room_id: Option<&str>,
    tile_type: Option<&TileType>,
    status: Option<&TileStatus>,
    limit: usize,
) -> Result<Vec<Tile>, rusqlite::Error> {
    let mut sql = String::from(
        "SELECT id, room_id, tile_type, parent_id, status, content,
                deadband_lower, deadband_upper, deadband_current,
                ensign_id, model_used, tokens_used, conservation_delta,
                metadata, created_tick, updated_tick
         FROM tiles WHERE 1=1"
    );
    let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    let mut param_idx = 1;

    if let Some(rid) = room_id {
        sql.push_str(&format!(" AND room_id = ?{}", param_idx));
        param_values.push(Box::new(rid.to_string()));
        param_idx += 1;
    }
    if let Some(tt) = tile_type {
        sql.push_str(&format!(" AND tile_type = ?{}", param_idx));
        param_values.push(Box::new(tt.as_str().to_string()));
        param_idx += 1;
    }
    if let Some(st) = status {
        sql.push_str(&format!(" AND status = ?{}", param_idx));
        param_values.push(Box::new(st.as_str().to_string()));
    }

    sql.push_str(&format!(" ORDER BY created_tick DESC LIMIT {}", limit));

    let mut stmt = conn.prepare(&sql)?;
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = param_values.iter().map(|p| p.as_ref()).collect();

    let tiles = stmt.query_map(param_refs.as_slice(), |row| {
        Ok(row_to_tile(row))
    })?;

    tiles.collect()
}

pub fn update_tile_status(
    conn: &Connection,
    id: &str,
    status: &TileStatus,
    tick: u64,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "UPDATE tiles SET status = ?1, updated_tick = ?2 WHERE id = ?3",
        params![status.as_str(), tick as i64, id],
    )?;
    Ok(())
}

fn row_to_tile(row: &rusqlite::Row<'_>) -> Tile {
    let type_str: String = row.get(2).unwrap_or_default();
    let status_str: String = row.get(4).unwrap_or_default();
    let meta_str: Option<String> = row.get(13).unwrap_or(None);

    Tile {
        id: row.get(0).unwrap_or_default(),
        room_id: row.get(1).unwrap_or(None),
        tile_type: TileType::from_str(&type_str).unwrap_or(TileType::Observation),
        parent_id: row.get(3).unwrap_or(None),
        status: TileStatus::from_str(&status_str).unwrap_or(TileStatus::Active),
        content: row.get(5).unwrap_or_default(),
        deadband_lower: row.get(6).unwrap_or(None),
        deadband_upper: row.get(7).unwrap_or(None),
        deadband_current: row.get(8).unwrap_or(None),
        ensign_id: row.get(9).unwrap_or(None),
        model_used: row.get(10).unwrap_or(None),
        tokens_used: row.get::<_, i64>(11).unwrap_or(0) as u32,
        conservation_delta: row.get(12).unwrap_or(0.0),
        metadata: meta_str.and_then(|s| serde_json::from_str(&s).ok()),
        created_tick: row.get::<_, i64>(14).unwrap_or(0) as u64,
        updated_tick: row.get::<_, i64>(15).unwrap_or(0) as u64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tile_type_roundtrip() {
        for tt in &[TileType::Observation, TileType::Action, TileType::Thought, TileType::Delegation, TileType::Escalation, TileType::Artifact] {
            assert_eq!(TileType::from_str(tt.as_str()), Some(tt.clone()));
        }
    }

    #[test]
    fn tile_status_roundtrip() {
        for ts in &[TileStatus::Active, TileStatus::Complete, TileStatus::Deadband, TileStatus::Escalated, TileStatus::Archived] {
            assert_eq!(TileStatus::from_str(ts.as_str()), Some(ts.clone()));
        }
    }

    #[test]
    fn tile_type_display() {
        assert_eq!(format!("{}", TileType::Action), "action");
    }

    #[test]
    fn tile_new_has_uuid() {
        let t = Tile::new(TileType::Observation, "hello", 0);
        assert!(!t.id.is_empty());
        assert_eq!(t.tile_type, TileType::Observation);
        assert_eq!(t.content, "hello");
        assert_eq!(t.status, TileStatus::Active);
        assert!(t.room_id.is_none());
        assert!(t.parent_id.is_none());
    }

    #[test]
    fn tile_builder_pattern() {
        let t = Tile::new(TileType::Action, "test", 0)
            .with_room("r1")
            .with_parent("p1")
            .with_ensign("e1");
        assert_eq!(t.room_id.as_deref(), Some("r1"));
        assert_eq!(t.parent_id.as_deref(), Some("p1"));
        assert_eq!(t.ensign_id.as_deref(), Some("e1"));
    }

    #[test]
    fn tile_complete() {
        let mut t = Tile::new(TileType::Action, "done", 0);
        t.complete(10);
        assert_eq!(t.status, TileStatus::Complete);
        assert_eq!(t.updated_tick, 10);
    }

    #[test]
    fn tile_escalate() {
        let mut t = Tile::new(TileType::Escalation, "bad", 0);
        t.escalate("something went wrong", 5);
        assert_eq!(t.status, TileStatus::Escalated);
        assert_eq!(t.updated_tick, 5);
        let meta = t.metadata.unwrap();
        assert_eq!(meta["escalation_reason"], "something went wrong");
    }

    #[test]
    fn insert_and_get_tile() {
        let conn = Connection::open_in_memory().unwrap();
        crate::room::init_schema(&conn).unwrap();
        init_schema(&conn).unwrap();
        let t = Tile::new(TileType::Observation, "test obs", 1);
        insert_tile(&conn, &t).unwrap();
        let loaded = get_tile(&conn, &t.id).unwrap().unwrap();
        assert_eq!(loaded.id, t.id);
        assert_eq!(loaded.content, "test obs");
        assert_eq!(loaded.tile_type, TileType::Observation);
    }

    #[test]
    fn get_tile_missing() {
        let conn = Connection::open_in_memory().unwrap();
        crate::room::init_schema(&conn).unwrap();
        init_schema(&conn).unwrap();
        assert!(get_tile(&conn, "nonexistent").unwrap().is_none());
    }

    #[test]
    fn query_tiles_by_type() {
        let conn = Connection::open_in_memory().unwrap();
        crate::room::init_schema(&conn).unwrap();
        init_schema(&conn).unwrap();
        let t1 = Tile::new(TileType::Observation, "obs", 1);
        let t2 = Tile::new(TileType::Action, "act", 2);
        insert_tile(&conn, &t1).unwrap();
        insert_tile(&conn, &t2).unwrap();
        let obs = query_tiles(&conn, None, Some(&TileType::Observation), None, 10).unwrap();
        assert_eq!(obs.len(), 1);
        assert_eq!(obs[0].tile_type, TileType::Observation);
    }

    #[test]
    fn update_status_in_db() {
        let conn = Connection::open_in_memory().unwrap();
        crate::room::init_schema(&conn).unwrap();
        init_schema(&conn).unwrap();
        let t = Tile::new(TileType::Action, "act", 1);
        insert_tile(&conn, &t).unwrap();
        super::update_tile_status(&conn, &t.id, &TileStatus::Complete, 5).unwrap();
        let loaded = get_tile(&conn, &t.id).unwrap().unwrap();
        assert_eq!(loaded.status, TileStatus::Complete);
        assert_eq!(loaded.updated_tick, 5);
    }
}
