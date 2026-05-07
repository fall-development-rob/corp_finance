//! Behavioral trust scoring per peer (ADR-019 §"Behavioral Trust
//! Scoring").
//!
//! Composite formula (FED-INV-004):
//!
//! ```text
//! composite = 0.4 * success_rate
//!           + 0.2 * uptime
//!           + 0.2 * (1.0 - threat_score)
//!           + 0.2 * integrity_score
//! ```
//!
//! Note the asymmetric input: `threat_score` is itself in [0, 1] but
//! contributes inversely to trust (higher threat -> lower trust).
//!
//! ## Persistence
//!
//! v1 exposes `save_to(conn, score)` / `load_from(conn, peer_id)` which
//! consume a `rusqlite::Connection` provided by the caller. The schema is
//! a single `trust_scores` table keyed on `peer_id`. v2 will move to the
//! cost-ledger SQLite store; the table layout is kept compatible.
//!
//! ## Upgrade / downgrade
//!
//! - [`upgrade_eligibility`] gates trust upgrades on `>=10` interactions
//!   AND `composite >= 0.7` (RUF-FED-005).
//! - [`instant_downgrade_on_threat`] zeroes the threat-related component
//!   immediately on `Critical` severity (no smoothing window).

use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use super::types::TrustScore;

// ---------------------------------------------------------------------------
// Composition
// ---------------------------------------------------------------------------

/// Compute a composite trust score from the four behavioral inputs.
///
/// All inputs are clamped to `[0.0, 1.0]` before composition. The
/// resulting `TrustScore.composite` is guaranteed to be in `[0.0, 1.0]`.
pub fn compute_trust_score(
    success_rate: f32,
    uptime: f32,
    threat_score: f32,
    integrity_score: f32,
) -> TrustScore {
    let s = success_rate.clamp(0.0, 1.0);
    let u = uptime.clamp(0.0, 1.0);
    let t = threat_score.clamp(0.0, 1.0);
    let i = integrity_score.clamp(0.0, 1.0);

    let composite = 0.4 * s + 0.2 * u + 0.2 * (1.0 - t) + 0.2 * i;
    let composite = composite.clamp(0.0, 1.0);

    TrustScore {
        peer_id: String::new(),
        success_rate: s,
        uptime: u,
        threat_score: t,
        integrity_score: i,
        composite,
        last_updated: Utc::now(),
    }
}

// ---------------------------------------------------------------------------
// Upgrade / downgrade gates
// ---------------------------------------------------------------------------

/// One historical interaction with a peer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InteractionRecord {
    pub peer_id: String,
    pub ts: chrono::DateTime<Utc>,
    pub success: bool,
}

/// Threat severity ladder. `Critical` triggers instant downgrade.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThreatSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Eligibility check for trust-tier upgrade.
///
/// Per RUF-FED-005: requires at least 10 interaction records on the
/// history AND a composite score of at least 0.7.
pub fn upgrade_eligibility(history: &[InteractionRecord], score: f32) -> bool {
    history.len() >= 10 && score >= 0.7
}

/// Instant trust downgrade on threat detection.
///
/// `Critical` severity zeroes the success and integrity contributions and
/// pushes `threat_score` to 1.0, recomputing the composite. `High`
/// severity halves success / integrity. Lower severities are no-ops at
/// this layer (the threat-detection pipeline elsewhere accumulates them
/// into `threat_score` over time).
pub fn instant_downgrade_on_threat(score: &mut TrustScore, threat_severity: ThreatSeverity) {
    match threat_severity {
        ThreatSeverity::Critical => {
            score.success_rate = 0.0;
            score.integrity_score = 0.0;
            score.threat_score = 1.0;
        }
        ThreatSeverity::High => {
            score.success_rate *= 0.5;
            score.integrity_score *= 0.5;
            score.threat_score = (score.threat_score + 0.5).clamp(0.0, 1.0);
        }
        ThreatSeverity::Medium | ThreatSeverity::Low => {
            // No-op at this layer.
            return;
        }
    }
    let composite = 0.4 * score.success_rate
        + 0.2 * score.uptime
        + 0.2 * (1.0 - score.threat_score)
        + 0.2 * score.integrity_score;
    score.composite = composite.clamp(0.0, 1.0);
    score.last_updated = Utc::now();
}

// ---------------------------------------------------------------------------
// SQLite persistence helpers
// ---------------------------------------------------------------------------

/// Initialise the `trust_scores` table on `conn`. Idempotent.
pub fn ensure_schema(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS trust_scores (
            peer_id TEXT PRIMARY KEY,
            success_rate REAL NOT NULL,
            uptime REAL NOT NULL,
            threat_score REAL NOT NULL,
            integrity_score REAL NOT NULL,
            composite REAL NOT NULL,
            last_updated TEXT NOT NULL
        )",
        [],
    )?;
    Ok(())
}

/// Persist (or upsert) a trust score row.
pub fn save_to(conn: &Connection, score: &TrustScore) -> rusqlite::Result<()> {
    ensure_schema(conn)?;
    conn.execute(
        "INSERT INTO trust_scores (peer_id, success_rate, uptime, threat_score, integrity_score, composite, last_updated)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(peer_id) DO UPDATE SET
            success_rate = excluded.success_rate,
            uptime = excluded.uptime,
            threat_score = excluded.threat_score,
            integrity_score = excluded.integrity_score,
            composite = excluded.composite,
            last_updated = excluded.last_updated",
        params![
            score.peer_id,
            score.success_rate as f64,
            score.uptime as f64,
            score.threat_score as f64,
            score.integrity_score as f64,
            score.composite as f64,
            score.last_updated.to_rfc3339(),
        ],
    )?;
    Ok(())
}

/// Load a trust score row by `peer_id`. Returns `Ok(None)` if no row.
pub fn load_from(conn: &Connection, peer_id: &str) -> rusqlite::Result<Option<TrustScore>> {
    ensure_schema(conn)?;
    let mut stmt = conn.prepare(
        "SELECT peer_id, success_rate, uptime, threat_score, integrity_score, composite, last_updated
         FROM trust_scores WHERE peer_id = ?1",
    )?;
    let mut rows = stmt.query(params![peer_id])?;
    if let Some(row) = rows.next()? {
        let last_updated_s: String = row.get(6)?;
        let last_updated = chrono::DateTime::parse_from_rfc3339(&last_updated_s)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        Ok(Some(TrustScore {
            peer_id: row.get(0)?,
            success_rate: row.get::<_, f64>(1)? as f32,
            uptime: row.get::<_, f64>(2)? as f32,
            threat_score: row.get::<_, f64>(3)? as f32,
            integrity_score: row.get::<_, f64>(4)? as f32,
            composite: row.get::<_, f64>(5)? as f32,
            last_updated,
        }))
    } else {
        Ok(None)
    }
}
