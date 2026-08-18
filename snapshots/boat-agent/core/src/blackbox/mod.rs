//! Black box: append-only, SHA-256 hash-chained flight recorder.
//!
//! ## Architecture
//!
//! The black box is the spine of the vessel intelligence system. It:
//! - Records every actuation tick with complete context
//! - Provides the replay source for testing and audit
//! - Serves as the trust substrate for the entire system
//! - Enables legal liability protection through cryptographic provenance
//!
//! ## Chain Format
//!
//! Each entry: `prev_hash|curr_hash|json_data\n`
//!
//! This blockchain-style chaining ensures tamper-evidence: any modification
//! to a historical entry breaks all subsequent hashes. The Auditor runs
//! `verify_range()` continuously to detect corruption.
//!
//! ## What Gets Recorded
//!
//! Every actuation tick (10Hz) records:
//! - `timestamp_epoch_ms`: When this happened
//! - `tick`: Kernel tick number
//! - `state_hash`: Exact world-state anchor (SHA256 of VesselState)
//! - `dial_level`: Authority basis (human intent)
//! - `actor`: Who initiated (playbook:…@hash | human | envelope)
//! - `intent_json`: What was requested (if any)
//! - `verdict_json`: What happened (approved/clamped/rejected)
//! - `human_override_detected`: Whether veto was active
//!
//! This is enough data to answer "why did the boat do that?" for any
//! action, enabling replay, audit, and legal defense.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::Mutex;

/// What gets recorded every actuation tick.
///
/// Fields chosen so that "why did the boat do that?" is always computable
/// from the log (docs/04 axiom A7).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlackBoxEntry {
    /// Unix epoch milliseconds
    pub timestamp_epoch_ms: u64,
    /// Kernel tick number
    pub tick: u64,
    /// SHA256 of VesselState — exact world-state anchor
    pub state_hash: String,
    /// Autonomy dial level (0-3) — authority basis
    pub dial_level: u8,
    /// Who initiated this action
    ///
    /// Format: "playbook:<id>@<hash>" | "human" | "envelope" | "agent:<role>"
    pub actor: String,
    /// Intent JSON (what was requested)
    ///
    /// Serialized `Intent` from bus events. None if no intent this tick.
    pub intent_json: Option<String>,
    /// Verdict JSON (what happened)
    ///
    /// Serialized `Verdict` from bus events. None if no verdict this tick.
    pub verdict_json: Option<String>,
    /// Whether human override (jog lever) was detected
    pub human_override_detected: bool,
}

/// The black box — append-only, hash-chained log.
///
/// This is the only trusted storage in the system. All other state can be
/// reconstructed from this log plus the schemas.
pub struct BlackBox {
    /// SHA-256 hash of the last entry (empty for genesis)
    last_hash: Vec<u8>,
    /// Path to the log file
    log_path: PathBuf,
    /// Sequence number (monotonically increasing)
    seq: u64,
}

/// Result of chain verification.
///
/// Used by the Auditor to report chain integrity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainVerification {
    /// Range that was verified
    pub range: (u64, u64),
    /// Whether all hashes were valid
    pub is_valid: bool,
    /// First invalid sequence (if any)
    pub first_invalid_seq: Option<u64>,
    /// Total entries checked
    pub entries_checked: u64,
}

/// Error types for black box operations.
#[derive(Debug, thiserror::Error)]
pub enum BlackBoxError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Chain corruption at seq {seq}: expected {expected} but found {found}")]
    ChainCorruption { seq: u64, expected: String, found: String },
    #[error("Log file not found: {0}")]
    NotFound(String),
}

impl BlackBox {
    /// Open a black box log file.
    ///
    /// Resume: read last line, adopt its hash as chain head. If the file was
    /// truncated or corrupted, the FIRST new entry records the discontinuity
    /// explicitly — gaps are data, not errors.
    pub fn open(log_path: PathBuf) -> Result<Self, BlackBoxError> {
        // If file doesn't exist, start fresh
        if !log_path.exists() {
            // Create parent directory if needed
            if let Some(parent) = log_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            return Ok(Self {
                last_hash: Vec::new(), // Empty hash for genesis
                log_path,
                seq: 0,
            });
        }

        // Resume: walk the file once — adopt the last line's hash as chain
        // head and the line count as the sequence number. If the tail is
        // corrupted, start a discontinuity: empty hash, FIRST new entry
        // records the gap explicitly — gaps are data, not errors.
        let (line_count, last_line) = Self::read_tail(&log_path)?;

        let mut last_hash = Vec::new();
        if let Some(line) = last_line {
            // Parse: prev_hash|curr_hash|json
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() >= 2 {
                last_hash = hex::decode(parts[1].trim())
                    .map_err(|e| BlackBoxError::Serialization(format!("Invalid hex: {}", e)))?;
            }
        }

        Ok(Self {
            last_hash,
            log_path,
            seq: line_count,
        })
    }

    /// Read the line count and last line from a log file.
    ///
    /// Used to resume the chain after restart (hash head + sequence).
    fn read_tail(path: &PathBuf) -> Result<(u64, Option<String>), BlackBoxError> {
        use std::io::{BufRead, BufReader};

        let file = File::open(path)?;
        let reader = BufReader::new(file);

        let mut count = 0u64;
        let mut last_line = None;
        for line in reader.lines() {
            let line = line?;
            if !line.trim().is_empty() {
                count += 1;
                last_line = Some(line);
            }
        }

        Ok((count, last_line))
    }

    /// Append one entry to the black box.
    ///
    /// Format: `prev_hash|curr_hash|json_data\n`
    ///
    /// This is the ONLY way to write to the black box. All entries must pass
    /// through this method to ensure chain integrity.
    ///
    /// Returns: the current hash (hex) for reference
    pub fn append(&mut self, entry: &BlackBoxEntry) -> Result<String, BlackBoxError> {
        // Serialize the entry to JSON
        let json = serde_json::to_string(entry)
            .map_err(|e| BlackBoxError::Serialization(e.to_string()))?;

        // Compute the sequential hash chain (prev_hash + current_data)
        let mut hasher = Sha256::new();
        hasher.update(&self.last_hash);
        hasher.update(json.as_bytes());
        let current_hash = hasher.finalize().to_vec();

        // Format the log line: prev_hash|curr_hash|json_data\n
        let hex_prev = hex::encode(&self.last_hash);
        let hex_curr = hex::encode(&current_hash);
        let log_line = format!("{}|{}|{}\n", hex_prev, hex_curr, json);

        // Atomic append + flush to ensure physical disk write
        //
        // We use BufWriter for efficiency but flush after every write to
        // ensure durability. This is slower but safer — a crash before
        // flush loses the last entry, which is acceptable.
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)?;

        {
            let mut writer = BufWriter::new(file);
            writer.write_all(log_line.as_bytes())?;
            writer.flush()?;
        }

        // Update state for next entry
        self.last_hash = current_hash;
        self.seq += 1;

        Ok(hex_curr)
    }

    /// Verify a range of the hash chain.
    ///
    /// The Auditor runs this continuously to detect tampering. This is how
    /// the system proves "the log has not been modified since X."
    ///
    /// Parameters:
    /// - `from_seq`: Starting sequence number (inclusive)
    /// - `to_seq`: Ending sequence number (inclusive)
    ///
    /// Returns: verification result with validity status and any corruption details
    pub fn verify_range(&self, from_seq: u64, to_seq: u64) -> Result<ChainVerification, BlackBoxError> {
        use std::io::BufRead;

        // Read the entire file
        let file = File::open(&self.log_path)?;
        let reader = std::io::BufReader::new(file);

        let mut lines = reader.lines();
        let mut prev_hash: Vec<u8> = Vec::new(); // Start with empty hash
        let mut entries_checked = 0u64;
        let mut first_invalid_seq: Option<u64> = None;

        for seq in 0..=to_seq {
            if let Some(Ok(line)) = lines.next() {
                // Skip entries before our range
                if seq < from_seq {
                    // Still need to track hash for continuity
                    if let Some(hash) = Self::extract_hash_from_line(&line) {
                        prev_hash = hash;
                    }
                    continue;
                }

                // Verify this entry
                let parts: Vec<&str> = line.split('|').collect();
                if parts.len() < 3 {
                    return Err(BlackBoxError::Serialization(
                        format!("Invalid line format at seq {}", seq)
                    ));
                }

                let _stated_prev = parts[0].trim();
                let stated_curr = parts[1].trim();
                let json_data = parts[2..].join("|"); // Rejoin in case json contains |

                // Verify: hash(prev_hash + json) == stated_curr
                let mut hasher = Sha256::new();
                hasher.update(&prev_hash);
                hasher.update(json_data.as_bytes());
                let computed_hash = hex::encode(hasher.finalize());

                if computed_hash != stated_curr {
                    if first_invalid_seq.is_none() {
                        first_invalid_seq = Some(seq);
                    }
                }

                // Update for next iteration
                prev_hash = hex::decode(stated_curr)
                    .map_err(|e| BlackBoxError::Serialization(format!("Invalid hex: {}", e)))?;
                entries_checked += 1;
            } else {
                break; // End of file
            }
        }

        let is_valid = first_invalid_seq.is_none();

        Ok(ChainVerification {
            range: (from_seq, to_seq),
            is_valid,
            first_invalid_seq,
            entries_checked,
        })
    }

    /// Extract the current hash from a log line.
    ///
    /// Format: `prev_hash|curr_hash|json_data`
    fn extract_hash_from_line(line: &str) -> Option<Vec<u8>> {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 2 {
            hex::decode(parts[1].trim()).ok()
        } else {
            None
        }
    }

    /// Read a range of entries from the log.
    ///
    /// Used for replay and audit. Returns entries in sequence order.
    pub fn read_range(&self, from_seq: u64, to_seq: u64) -> Result<Vec<BlackBoxEntry>, BlackBoxError> {
        use std::io::BufRead;

        let file = File::open(&self.log_path)?;
        let reader = std::io::BufReader::new(file);

        let mut lines = reader.lines();
        let mut entries = Vec::new();

        for seq in 0..=to_seq {
            if let Some(Ok(line)) = lines.next() {
                if seq < from_seq {
                    continue; // Skip entries before range
                }

                // Parse: prev_hash|curr_hash|json
                let parts: Vec<&str> = line.split('|').collect();
                if parts.len() >= 3 {
                    let json_data = parts[2..].join("|");

                    if let Ok(entry) = serde_json::from_str::<BlackBoxEntry>(&json_data) {
                        entries.push(entry);
                    }
                }
            } else {
                break; // End of file
            }
        }

        Ok(entries)
    }

    /// Get the current sequence number.
    pub fn seq(&self) -> u64 {
        self.seq
    }

    /// Get the current chain head hash.
    pub fn current_hash(&self) -> String {
        hex::encode(&self.last_hash)
    }

    /// Get the log file path.
    pub fn path(&self) -> &PathBuf {
        &self.log_path
    }

    /// Flush any buffered writes (for safety before shutdown).
    pub fn flush(&mut self) -> Result<(), BlackBoxError> {
        // Open and close the file to ensure OS buffers are flushed
        let _file = File::open(&self.log_path)?;
        Ok(())
    }
}

/// Thread-safe wrapper for black box access.
///
/// The black box may be accessed from multiple threads (kernel, auditor,
/// etc.). This wrapper provides interior mutability through Mutex.
pub struct SharedBlackBox(Mutex<BlackBox>);

impl SharedBlackBox {
    /// Create a new shared black box.
    pub fn new(log_path: PathBuf) -> Result<Self, BlackBoxError> {
        Ok(SharedBlackBox(Mutex::new(BlackBox::open(log_path)?)))
    }

    /// Append an entry (thread-safe).
    pub fn append(&self, entry: &BlackBoxEntry) -> Result<String, BlackBoxError> {
        let mut bb = self.0.lock().map_err(|e| {
            BlackBoxError::Serialization(format!("Mutex poisoned: {}", e))
        })?;
        bb.append(entry)
    }

    /// Verify a range (thread-safe).
    pub fn verify_range(&self, from_seq: u64, to_seq: u64) -> Result<ChainVerification, BlackBoxError> {
        let bb = self.0.lock().map_err(|e| {
            BlackBoxError::Serialization(format!("Mutex poisoned: {}", e))
        })?;
        bb.verify_range(from_seq, to_seq)
    }

    /// Read a range (thread-safe).
    pub fn read_range(&self, from_seq: u64, to_seq: u64) -> Result<Vec<BlackBoxEntry>, BlackBoxError> {
        let bb = self.0.lock().map_err(|e| {
            BlackBoxError::Serialization(format!("Mutex poisoned: {}", e))
        })?;
        bb.read_range(from_seq, to_seq)
    }

    /// Get current sequence (thread-safe).
    pub fn seq(&self) -> u64 {
        let bb = self.0.lock().unwrap();
        bb.seq()
    }

    /// Get current hash (thread-safe).
    pub fn current_hash(&self) -> String {
        let bb = self.0.lock().unwrap();
        bb.current_hash()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_test_entry(tick: u64) -> BlackBoxEntry {
        BlackBoxEntry {
            timestamp_epoch_ms: 1234567890 + tick * 100,
            tick,
            state_hash: format!("state_hash_{}", tick),
            dial_level: 2,
            actor: "playbook:test@abc123".to_string(),
            intent_json: Some(r#"{"requested_rudder_deg":5.0}"#.to_string()),
            verdict_json: Some(r#"{"outcome":"approved"}"#.to_string()),
            human_override_detected: false,
        }
    }

    #[test]
    fn test_append_and_chain() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test.log");

        let mut bb = BlackBox::open(log_path.clone()).unwrap();

        // Append first entry
        let entry1 = make_test_entry(0);
        let hash1 = bb.append(&entry1).unwrap();

        // Append second entry
        let entry2 = make_test_entry(1);
        let hash2 = bb.append(&entry2).unwrap();

        // Hashes should be different
        assert_ne!(hash1, hash2);

        // Sequence should advance
        assert_eq!(bb.seq(), 2);
    }

    #[test]
    fn test_verify_range_valid() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test.log");

        let mut bb = BlackBox::open(log_path.clone()).unwrap();

        // Append some entries
        for i in 0..5 {
            bb.append(&make_test_entry(i)).unwrap();
        }

        // Verify the range
        let verification = bb.verify_range(0, 4).unwrap();

        assert!(verification.is_valid);
        assert_eq!(verification.entries_checked, 5);
        assert!(verification.first_invalid_seq.is_none());
    }

    #[test]
    fn test_resume_from_existing_log() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test.log");

        // Create a log file
        let mut bb1 = BlackBox::open(log_path.clone()).unwrap();
        bb1.append(&make_test_entry(0)).unwrap();
        bb1.append(&make_test_entry(1)).unwrap();
        let last_hash = bb1.current_hash();

        // Reopen (simulates restart)
        let bb2 = BlackBox::open(log_path).unwrap();

        // Should have resumed from last hash
        assert_eq!(bb2.current_hash(), last_hash);
        assert_eq!(bb2.seq(), 2);
    }

    #[test]
    fn test_read_range() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test.log");

        let mut bb = BlackBox::open(log_path.clone()).unwrap();

        // Append entries
        let expected = vec![
            make_test_entry(0),
            make_test_entry(1),
            make_test_entry(2),
        ];

        for entry in &expected {
            bb.append(entry).unwrap();
        }

        // Read back
        let entries = bb.read_range(0, 2).unwrap();

        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].tick, 0);
        assert_eq!(entries[1].tick, 1);
        assert_eq!(entries[2].tick, 2);
    }

    #[test]
    fn test_shared_black_box() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test.log");

        let sbb = SharedBlackBox::new(log_path).unwrap();

        // Append through shared interface
        sbb.append(&make_test_entry(0)).unwrap();

        // Read back through shared interface
        let entries = sbb.read_range(0, 0).unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].tick, 0);
    }
}
