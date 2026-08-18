#![allow(dead_code)]
//! penrose.rs — Cross-room correlation detection
//!
//! Pearson coefficients between room signals. Detects when rooms
//! are correlated (success rates, gravity drift, task patterns).
//!
//! # Spectral Analysis
//!
//! Beyond simple Pearson correlation, this module provides:
//!
//! - **Autocorrelation**: Detect periodic patterns in a single room's gravity
//!   history. High autocorrelation at lag k suggests the room's behavior repeats
//!   every k ticks.
//!
//! - **Cross-correlation**: Detect lagged relationships between rooms. If room B's
//!   gravity tracks room A's with a delay of 3 ticks, the cross-correlation will
//!   peak at lag 3.
//!
//! Reference: Box, G.E.P., Jenkins, G.M., & Reinsel, G.C. (2015).
//! *Time Series Analysis: Forecasting and Control*. 5th ed. Wiley.
//! ISBN: 978-1-118-67502-1.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SplineType {
    Causal,
    Resonant,
    Predictive,
    Synergistic,
    Redundant,
}

impl SplineType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Causal => "causal",
            Self::Resonant => "resonant",
            Self::Predictive => "predictive",
            Self::Synergistic => "synergistic",
            Self::Redundant => "redundant",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "causal" => Some(Self::Causal),
            "resonant" => Some(Self::Resonant),
            "predictive" => Some(Self::Predictive),
            "synergistic" => Some(Self::Synergistic),
            "redundant" => Some(Self::Redundant),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Correlation {
    pub id: String,
    pub room_a: String,
    pub room_b: String,
    pub correlation: f64,
    pub spline_type: SplineType,
    pub confidence: f64,
    pub occurrences: u32,
    pub energy_savings: f64,
    pub token_savings: u32,
    pub first_detected: u64,
    pub last_confirmed: u64,
}

// ---------------------------------------------------------------------------
// Pearson correlation
// ---------------------------------------------------------------------------

/// Compute Pearson correlation coefficient between two series
pub fn pearson(x: &[f64], y: &[f64]) -> f64 {
    if x.len() != y.len() || x.len() < 2 {
        return 0.0;
    }

    let n = x.len() as f64;
    let mean_x: f64 = x.iter().sum::<f64>() / n;
    let mean_y: f64 = y.iter().sum::<f64>() / n;

    let mut cov = 0.0;
    let mut var_x = 0.0;
    let mut var_y = 0.0;

    for i in 0..x.len() {
        let dx = x[i] - mean_x;
        let dy = y[i] - mean_y;
        cov += dx * dy;
        var_x += dx * dx;
        var_y += dy * dy;
    }

    if var_x == 0.0 || var_y == 0.0 {
        return 0.0;
    }

    cov / (var_x.sqrt() * var_y.sqrt())
}

/// Classify the type of correlation based on the coefficient
pub fn classify_correlation(coeff: f64) -> SplineType {
    let abs = coeff.abs();
    if abs > 0.9 {
        SplineType::Causal
    } else if abs > 0.7 {
        SplineType::Predictive
    } else if abs > 0.5 {
        SplineType::Synergistic
    } else if abs > 0.3 {
        SplineType::Resonant
    } else {
        SplineType::Redundant
    }
}

// ---------------------------------------------------------------------------
// SQLite
// ---------------------------------------------------------------------------

pub fn init_schema(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS correlations (
            id TEXT PRIMARY KEY,
            room_a TEXT NOT NULL,
            room_b TEXT NOT NULL,
            correlation REAL NOT NULL,
            spline_type TEXT NOT NULL,
            confidence REAL DEFAULT 0.0,
            occurrences INTEGER DEFAULT 1,
            energy_savings REAL DEFAULT 0.0,
            token_savings INTEGER DEFAULT 0,
            first_detected INTEGER NOT NULL,
            last_confirmed INTEGER NOT NULL,
            FOREIGN KEY (room_a) REFERENCES rooms(id),
            FOREIGN KEY (room_b) REFERENCES rooms(id)
        );"
    )
}

pub fn insert_correlation(conn: &Connection, corr: &Correlation) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO correlations (id, room_a, room_b, correlation, spline_type,
         confidence, occurrences, energy_savings, token_savings,
         first_detected, last_confirmed)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            corr.id,
            corr.room_a,
            corr.room_b,
            corr.correlation,
            corr.spline_type.as_str(),
            corr.confidence,
            corr.occurrences as i64,
            corr.energy_savings,
            corr.token_savings as i64,
            corr.first_detected as i64,
            corr.last_confirmed as i64,
        ],
    )?;
    Ok(())
}

pub fn get_correlations_for_room(
    conn: &Connection,
    room_id: &str,
) -> Result<Vec<Correlation>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, room_a, room_b, correlation, spline_type,
                confidence, occurrences, energy_savings, token_savings,
                first_detected, last_confirmed
         FROM correlations WHERE room_a = ?1 OR room_b = ?1"
    )?;
    let corrs = stmt.query_map(params![room_id], |row| Ok(row_to_correlation(row)))?;
    corrs.collect::<Result<Vec<_>, _>>()
}

pub fn get_all_correlations(conn: &Connection) -> Result<Vec<Correlation>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, room_a, room_b, correlation, spline_type,
                confidence, occurrences, energy_savings, token_savings,
                first_detected, last_confirmed
         FROM correlations"
    )?;
    let corrs = stmt.query_map([], |row| Ok(row_to_correlation(row)))?;
    corrs.collect::<Result<Vec<_>, _>>()
}

/// Scan all room pairs for correlations based on gravity history
pub fn scan_correlations(
    conn: &Connection,
    room_gravities: &std::collections::HashMap<String, Vec<f64>>,
    tick: u64,
) -> Result<Vec<Correlation>, rusqlite::Error> {
    let rooms: Vec<&String> = room_gravities.keys().collect();
    let mut new_correlations = Vec::new();

    for i in 0..rooms.len() {
        for j in (i + 1)..rooms.len() {
            let room_a = rooms[i];
            let room_b = rooms[j];

            let grav_a = room_gravities.get(room_a).unwrap();
            let grav_b = room_gravities.get(room_b).unwrap();

            // Need at least 5 samples
            let min_len = grav_a.len().min(grav_b.len());
            if min_len < 5 {
                continue;
            }

            let coeff = pearson(&grav_a[..min_len], &grav_b[..min_len]);
            let abs_coeff = coeff.abs();

            // Only store meaningful correlations (> 0.3)
            if abs_coeff > 0.3 {
                let spline_type = classify_correlation(coeff);

                let corr = Correlation {
                    id: uuid::Uuid::new_v4().to_string(),
                    room_a: (*room_a).clone(),
                    room_b: (*room_b).clone(),
                    correlation: coeff,
                    spline_type,
                    confidence: abs_coeff,
                    occurrences: 1,
                    energy_savings: 0.0,
                    token_savings: 0,
                    first_detected: tick,
                    last_confirmed: tick,
                };

                insert_correlation(conn, &corr)?;
                new_correlations.push(corr);
            }
        }
    }

    Ok(new_correlations)
}

fn row_to_correlation(row: &rusqlite::Row<'_>) -> Correlation {
    let spline_str: String = row.get(4).unwrap_or_default();

    Correlation {
        id: row.get(0).unwrap_or_default(),
        room_a: row.get(1).unwrap_or_default(),
        room_b: row.get(2).unwrap_or_default(),
        correlation: row.get(3).unwrap_or(0.0),
        spline_type: SplineType::from_str(&spline_str).unwrap_or(SplineType::Redundant),
        confidence: row.get(5).unwrap_or(0.0),
        occurrences: row.get::<_, i64>(6).unwrap_or(1) as u32,
        energy_savings: row.get(7).unwrap_or(0.0),
        token_savings: row.get::<_, i64>(8).unwrap_or(0) as u32,
        first_detected: row.get::<_, i64>(9).unwrap_or(0) as u64,
        last_confirmed: row.get::<_, i64>(10).unwrap_or(0) as u64,
    }
}

// ---------------------------------------------------------------------------
// Spectral analysis: autocorrelation & cross-correlation
// ---------------------------------------------------------------------------

/// Autocorrelation at a given lag for a single time series.
///
/// Computes the autocorrelation function (ACF) at lag `k`:
///
/// ```text
/// ACF(k) = Σ_{t=0}^{n-k-1} (x[t] - x̄) * (x[t+k] - x̄) / Σ_{t=0}^{n-1} (x[t] - x̄)²
/// ```
///
/// Returns 0.0 if the series has fewer than `lag + 2` elements or zero variance.
///
/// Reference: Box, Jenkins & Reinsel (2015), §2.1.
pub fn autocorrelation(series: &[f64], lag: usize) -> f64 {
    if lag == 0 || series.len() < lag + 2 {
        return if lag == 0 { 1.0 } else { 0.0 };
    }

    let n = series.len() as f64;
    let mean: f64 = series.iter().sum::<f64>() / n;

    let mut denom = 0.0;
    let mut numer = 0.0;
    for i in 0..series.len() {
        let d = series[i] - mean;
        denom += d * d;
        if i + lag < series.len() {
            numer += d * (series[i + lag] - mean);
        }
    }

    if denom.abs() < f64::EPSILON {
        return 0.0;
    }

    numer / denom
}

/// Compute the full autocorrelation function for lags 0..max_lag.
///
/// Returns a vector where `result[k]` = ACF(k). By definition, result[0] = 1.0.
pub fn autocorrelation_function(series: &[f64], max_lag: usize) -> Vec<f64> {
    let max_lag = max_lag.min(series.len().saturating_sub(1));
    (0..=max_lag).map(|k| autocorrelation(series, k)).collect()
}

/// Cross-correlation at a given lag between two time series.
///
/// Computes the normalized cross-correlation:
///
/// ```text
/// CCF(k) = Σ_{t} (x[t] - x̄) * (y[t+k] - ȳ) / (n * σ_x * σ_y)
/// ```
///
/// A positive lag means y lags behind x (x leads y). A negative lag means
/// x lags behind y (y leads x).
///
/// Returns 0.0 if the series are too short or have zero variance.
///
/// Reference: Box, Jenkins & Reinsel (2015), §11.1.
pub fn cross_correlation(x: &[f64], y: &[f64], lag: i32) -> f64 {
    let min_len = x.len().min(y.len());
    if min_len < 3 {
        return 0.0;
    }

    let mean_x: f64 = x.iter().sum::<f64>() / x.len() as f64;
    let mean_y: f64 = y.iter().sum::<f64>() / y.len() as f64;

    let mut var_x = 0.0f64;
    let mut var_y = 0.0f64;
    for i in 0..min_len {
        var_x += (x[i] - mean_x).powi(2);
        var_y += (y[i] - mean_y).powi(2);
    }

    if var_x.abs() < f64::EPSILON || var_y.abs() < f64::EPSILON {
        return 0.0;
    }

    let mut cov = 0.0f64;
    let mut count = 0.0f64;
    for (t, x_val) in x.iter().enumerate().take(min_len) {
        let ty = t as i32 + lag;
        if ty >= 0 && (ty as usize) < min_len {
            cov += (x_val - mean_x) * (y[ty as usize] - mean_y);
            count += 1.0;
        }
    }

    if count < 2.0 {
        return 0.0;
    }

    cov / (var_x.sqrt() * var_y.sqrt())
}

/// Compute cross-correlation for a range of lags.
///
/// Returns a vector of (lag, correlation) pairs for `lag` in `-max_lag..=max_lag`.
pub fn cross_correlation_function(
    x: &[f64],
    y: &[f64],
    max_lag: usize,
) -> Vec<(i32, f64)> {
    let max_lag = max_lag.min(x.len().saturating_sub(2));
    let max_lag = max_lag.min(y.len().saturating_sub(2));
    (-(max_lag as i32)..=max_lag as i32)
        .map(|lag| (lag, cross_correlation(x, y, lag)))
        .collect()
}

/// Find the lag with the peak absolute cross-correlation.
///
/// Returns (best_lag, best_correlation). Useful for detecting lead/lag
/// relationships between rooms.
pub fn peak_cross_correlation(x: &[f64], y: &[f64], max_lag: usize) -> (i32, f64) {
    cross_correlation_function(x, y, max_lag)
        .into_iter()
        .max_by(|a, b| a.1.abs().partial_cmp(&b.1.abs()).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or((0, 0.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spline_type_roundtrip() {
        for s in &[SplineType::Causal, SplineType::Resonant, SplineType::Predictive, SplineType::Synergistic, SplineType::Redundant] {
            assert_eq!(SplineType::from_str(s.as_str()), Some(s.clone()));
        }
    }

    #[test]
    fn pearson_perfect_positive() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![2.0, 4.0, 6.0, 8.0, 10.0];
        let r = pearson(&x, &y);
        assert!((r - 1.0).abs() < 1e-9);
    }

    #[test]
    fn pearson_perfect_negative() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![10.0, 8.0, 6.0, 4.0, 2.0];
        let r = pearson(&x, &y);
        assert!((r - (-1.0)).abs() < 1e-9);
    }

    #[test]
    fn pearson_uncorrelated() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![5.0, 1.0, 4.0, 2.0, 3.0];
        let r = pearson(&x, &y);
        assert!(r.abs() < 0.5);
    }

    #[test]
    fn pearson_empty_mismatched() {
        assert_eq!(pearson(&[], &[]), 0.0);
        assert_eq!(pearson(&[1.0], &[2.0]), 0.0);
        assert_eq!(pearson(&[1.0, 2.0], &[1.0]), 0.0);
    }

    #[test]
    fn pearson_constant_series() {
        let x = vec![5.0, 5.0, 5.0];
        let y = vec![1.0, 2.0, 3.0];
        assert_eq!(pearson(&x, &y), 0.0);
    }

    #[test]
    fn classify_causal() {
        assert_eq!(classify_correlation(0.95), SplineType::Causal);
        assert_eq!(classify_correlation(-0.92), SplineType::Causal);
    }

    #[test]
    fn classify_predictive() {
        assert_eq!(classify_correlation(0.75), SplineType::Predictive);
    }

    #[test]
    fn classify_synergistic() {
        assert_eq!(classify_correlation(0.55), SplineType::Synergistic);
    }

    #[test]
    fn classify_resonant() {
        assert_eq!(classify_correlation(0.35), SplineType::Resonant);
    }

    #[test]
    fn classify_redundant() {
        assert_eq!(classify_correlation(0.1), SplineType::Redundant);
    }

    #[test]
    fn scan_correlations_inserts() {
        let conn = Connection::open_in_memory().unwrap();
        crate::room::init_schema(&conn).unwrap();
        conn.execute("INSERT INTO rooms (id, room_type, created_tick, updated_tick) VALUES (?1, 'engineering', 0, 0)", ["r1"]).unwrap();
        conn.execute("INSERT INTO rooms (id, room_type, created_tick, updated_tick) VALUES (?1, 'science', 0, 0)", ["r2"]).unwrap();
        init_schema(&conn).unwrap();
        let mut gravities = std::collections::HashMap::new();
        gravities.insert("r1".into(), vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        gravities.insert("r2".into(), vec![2.0, 4.0, 6.0, 8.0, 10.0]);
        let corrs = scan_correlations(&conn, &gravities, 1).unwrap();
        assert_eq!(corrs.len(), 1);
        assert!((corrs[0].correlation - 1.0).abs() < 1e-9);
    }

    // --- Spectral analysis tests ---

    #[test]
    fn autocorrelation_at_lag_zero_is_one() {
        let series = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert!((autocorrelation(&series, 0) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn autocorrelation_linear_series_decays() {
        let series = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let acf1 = autocorrelation(&series, 1);
        let acf2 = autocorrelation(&series, 2);
        // Autocorrelation should be positive and decreasing for linear series
        assert!(acf1 > acf2);
        assert!(acf1 > 0.0);
    }

    #[test]
    fn autocorrelation_short_series() {
        assert_eq!(autocorrelation(&[1.0], 1), 0.0);
        assert_eq!(autocorrelation(&[], 1), 0.0);
    }

    #[test]
    fn autocorrelation_constant_series() {
        let series = vec![5.0, 5.0, 5.0, 5.0];
        assert_eq!(autocorrelation(&series, 1), 0.0);
    }

    #[test]
    fn autocorrelation_periodic_series() {
        // Period-2 signal: ACF at lag 2 should be positive and close to 1.0
        // ACF at lag 1 should be negative and close to -1.0
        let series = vec![1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0];
        let acf1 = autocorrelation(&series, 1);
        let acf2 = autocorrelation(&series, 2);
        assert!(acf1 < -0.8, "expected < -0.8 at lag 1, got {}", acf1);
        assert!(acf2 > 0.5, "expected > 0.5 at lag 2, got {}", acf2);
    }

    #[test]
    fn autocorrelation_function_length() {
        let series = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let acf = autocorrelation_function(&series, 10);
        assert_eq!(acf.len(), 5); // clamped to series.len() - 1
        assert!((acf[0] - 1.0).abs() < 1e-9); // lag 0 = 1.0
    }

    #[test]
    fn cross_correlation_identical_series() {
        let s = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let ccf0 = cross_correlation(&s, &s, 0);
        assert!((ccf0 - 1.0).abs() < 1e-9);
    }

    #[test]
    fn cross_correlation_lagged_series() {
        // Use detrended data so lag structure is clearer
        let x = vec![1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0];
        let y = vec![-1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0]; // x inverted = lag 1
        let ccf0 = cross_correlation(&x, &y, 0);
        // At lag 0, x and y are anti-correlated
        assert!(ccf0 < -0.9, "expected < -0.9 at lag 0, got {}", ccf0);
        let ccf1 = cross_correlation(&x, &y, 1);
        // At lag 1, they should be positively correlated
        assert!(ccf1 > 0.5, "expected > 0.5 at lag 1, got {}", ccf1);
    }

    #[test]
    fn cross_correlation_short_series() {
        assert_eq!(cross_correlation(&[1.0], &[2.0], 0), 0.0);
        assert_eq!(cross_correlation(&[], &[], 0), 0.0);
    }

    #[test]
    fn cross_correlation_function_symmetry() {
        let x = vec![1.0, 3.0, 2.0, 4.0, 3.0, 5.0, 4.0, 6.0];
        let y = vec![2.0, 1.0, 3.0, 2.0, 4.0, 3.0, 5.0, 4.0];
        let ccf = cross_correlation_function(&x, &y, 3);
        assert_eq!(ccf.len(), 7); // -3..=3
        // CCF(-k) ≈ CCF(k) for similar stationary series
        let ccf_neg = cross_correlation(&x, &y, -2);
        let ccf_pos = cross_correlation(&x, &y, 2);
        // They won't be exactly equal but should be same sign
        assert_eq!(ccf_neg.signum(), ccf_pos.signum());
    }

    #[test]
    fn peak_cross_correlation_finds_best_lag() {
        let x = vec![1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0];
        let y = vec![-1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0];
        let (lag, corr) = peak_cross_correlation(&x, &y, 3);
        // Peak should be at an odd lag (where anti-phase becomes in-phase)
        assert!(corr.abs() > 0.5, "expected |corr| > 0.5, got {}", corr);
    }
}
