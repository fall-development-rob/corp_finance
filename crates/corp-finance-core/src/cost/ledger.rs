//! `rusqlite`-backed cost ledger.
//!
//! Schema lives at the top of [`CostLedger::open`] and is auto-migrated on
//! every open.  Tables:
//!
//! - `cost_events` — one row per recorded surface event.
//! - `cost_budgets` — one row per `(surface_filter, tier_filter, period)`
//!   budget definition.

use std::path::Path;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension, Row};
use uuid::Uuid;

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

use super::types::{CostEvent, Surface, TierTag};

/// SQL filter for `query()` and `summary()`.  All fields optional.
#[derive(Debug, Clone, Default)]
pub struct CostFilter {
    pub surface: Option<Surface>,
    pub tier: Option<TierTag>,
    pub tenant_id: Option<String>,
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
}

/// SQLite-backed cost ledger.
pub struct CostLedger {
    pub(crate) conn: Connection,
}

impl CostLedger {
    /// Open (or create) the ledger at `path`.  Auto-migrates the schema.
    pub fn open(path: &Path) -> CorpFinanceResult<Self> {
        let conn = Connection::open(path).map_err(map_err)?;
        Self::init_schema(&conn)?;
        Ok(Self { conn })
    }

    /// Open an in-memory ledger.  Useful for unit tests that don't want to
    /// touch disk.
    #[allow(dead_code)]
    pub fn open_in_memory() -> CorpFinanceResult<Self> {
        let conn = Connection::open_in_memory().map_err(map_err)?;
        Self::init_schema(&conn)?;
        Ok(Self { conn })
    }

    fn init_schema(conn: &Connection) -> CorpFinanceResult<()> {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS cost_events (
                event_id          TEXT PRIMARY KEY,
                surface           TEXT NOT NULL,
                surface_event_id  TEXT NOT NULL,
                model             TEXT NOT NULL,
                tokens_in         INTEGER NOT NULL,
                tokens_out        INTEGER NOT NULL,
                cost_cents        INTEGER NOT NULL,
                tier              TEXT NOT NULL,
                tenant_id         TEXT,
                ts                TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_cost_events_surface
                ON cost_events(surface);
            CREATE INDEX IF NOT EXISTS idx_cost_events_tier
                ON cost_events(tier);
            CREATE INDEX IF NOT EXISTS idx_cost_events_ts
                ON cost_events(ts);
            CREATE INDEX IF NOT EXISTS idx_cost_events_tenant
                ON cost_events(tenant_id);

            CREATE TABLE IF NOT EXISTS cost_budgets (
                surface_filter    TEXT,
                tier_filter       TEXT,
                period            TEXT NOT NULL,
                limit_cents       INTEGER NOT NULL,
                threshold_pct     INTEGER NOT NULL,
                PRIMARY KEY (surface_filter, tier_filter, period)
            );
            "#,
        )
        .map_err(map_err)?;
        Ok(())
    }

    /// Persist a `CostEvent`.
    pub fn record_event(&self, event: &CostEvent) -> CorpFinanceResult<()> {
        self.conn
            .execute(
                r#"
                INSERT INTO cost_events (
                    event_id, surface, surface_event_id, model,
                    tokens_in, tokens_out, cost_cents, tier, tenant_id, ts
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                "#,
                params![
                    event.event_id.to_string(),
                    event.surface.as_str(),
                    event.surface_event_id,
                    event.model,
                    event.tokens_in as i64,
                    event.tokens_out as i64,
                    event.cost_cents,
                    event.tier.as_str(),
                    event.tenant_id,
                    event.ts.to_rfc3339(),
                ],
            )
            .map_err(map_err)?;
        Ok(())
    }

    /// Query events with optional filtering.  Returns rows ordered by `ts`
    /// ascending.
    pub fn query(&self, filter: &CostFilter) -> CorpFinanceResult<Vec<CostEvent>> {
        let (where_clause, params_vec) = build_where(filter);
        let sql = format!(
            "SELECT event_id, surface, surface_event_id, model, tokens_in, \
             tokens_out, cost_cents, tier, tenant_id, ts \
             FROM cost_events {} ORDER BY ts ASC",
            where_clause
        );

        let mut stmt = self.conn.prepare(&sql).map_err(map_err)?;
        let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec
            .iter()
            .map(|v| v as &dyn rusqlite::ToSql)
            .collect();

        let rows = stmt
            .query_map(params_refs.as_slice(), row_to_event)
            .map_err(map_err)?;

        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(map_err)?);
        }
        Ok(out)
    }

    /// Total cents in the ledger matching `filter`.  Used by the budget
    /// threshold checker.
    pub(crate) fn sum_cents(&self, filter: &CostFilter) -> CorpFinanceResult<i64> {
        let (where_clause, params_vec) = build_where(filter);
        let sql = format!(
            "SELECT COALESCE(SUM(cost_cents), 0) FROM cost_events {}",
            where_clause
        );
        let mut stmt = self.conn.prepare(&sql).map_err(map_err)?;
        let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec
            .iter()
            .map(|v| v as &dyn rusqlite::ToSql)
            .collect();
        let total: i64 = stmt
            .query_row(params_refs.as_slice(), |r| r.get(0))
            .map_err(map_err)?;
        Ok(total)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Translate a `rusqlite::Error` into our domain error.
pub(crate) fn map_err(e: rusqlite::Error) -> CorpFinanceError {
    CorpFinanceError::SerializationError(format!("cost ledger sqlite error: {}", e))
}

/// Build a parameterised WHERE clause for `cost_events` from a `CostFilter`.
/// Params are returned as boxed `rusqlite::types::Value` so the caller can
/// rebind them to `&dyn ToSql`.
fn build_where(filter: &CostFilter) -> (String, Vec<rusqlite::types::Value>) {
    use rusqlite::types::Value;
    let mut clauses: Vec<&'static str> = Vec::new();
    let mut params_out: Vec<Value> = Vec::new();

    if let Some(s) = filter.surface {
        clauses.push("surface = ?");
        params_out.push(Value::Text(s.as_str().to_string()));
    }
    if let Some(t) = filter.tier {
        clauses.push("tier = ?");
        params_out.push(Value::Text(t.as_str().to_string()));
    }
    if let Some(ref tenant) = filter.tenant_id {
        clauses.push("tenant_id = ?");
        params_out.push(Value::Text(tenant.clone()));
    }
    if let Some(since) = filter.since {
        clauses.push("ts >= ?");
        params_out.push(Value::Text(since.to_rfc3339()));
    }
    if let Some(until) = filter.until {
        clauses.push("ts <= ?");
        params_out.push(Value::Text(until.to_rfc3339()));
    }

    let where_clause = if clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", clauses.join(" AND "))
    };

    (where_clause, params_out)
}

fn row_to_event(row: &Row<'_>) -> rusqlite::Result<CostEvent> {
    let event_id_str: String = row.get(0)?;
    let surface_str: String = row.get(1)?;
    let surface_event_id: String = row.get(2)?;
    let model: String = row.get(3)?;
    let tokens_in: i64 = row.get(4)?;
    let tokens_out: i64 = row.get(5)?;
    let cost_cents: i64 = row.get(6)?;
    let tier_str: String = row.get(7)?;
    let tenant_id: Option<String> = row.get(8)?;
    let ts_str: String = row.get(9)?;

    let event_id = Uuid::from_str(&event_id_str).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })?;
    let surface = Surface::parse(&surface_str).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            1,
            rusqlite::types::Type::Text,
            format!("unknown surface: {}", surface_str).into(),
        )
    })?;
    let tier = TierTag::parse(&tier_str).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            7,
            rusqlite::types::Type::Text,
            format!("unknown tier: {}", tier_str).into(),
        )
    })?;
    let ts = DateTime::parse_from_rfc3339(&ts_str)
        .map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(9, rusqlite::types::Type::Text, Box::new(e))
        })?
        .with_timezone(&Utc);

    Ok(CostEvent {
        event_id,
        surface,
        surface_event_id,
        model,
        tokens_in: tokens_in as u64,
        tokens_out: tokens_out as u64,
        cost_cents,
        tier,
        tenant_id,
        ts,
    })
}

/// Optional helper for budget persistence.  Looks up a single row by key.
pub(crate) fn fetch_budget_row(
    conn: &Connection,
    surface: Option<Surface>,
    tier: Option<TierTag>,
    period: &str,
) -> CorpFinanceResult<Option<(i64, u8)>> {
    let surface_str = surface.map(|s| s.as_str().to_string());
    let tier_str = tier.map(|t| t.as_str().to_string());

    let mut stmt = conn
        .prepare(
            "SELECT limit_cents, threshold_pct FROM cost_budgets \
             WHERE COALESCE(surface_filter, '') = COALESCE(?1, '') \
               AND COALESCE(tier_filter, '') = COALESCE(?2, '') \
               AND period = ?3",
        )
        .map_err(map_err)?;

    let row = stmt
        .query_row(
            params![surface_str, tier_str, period],
            |r| -> rusqlite::Result<(i64, i64)> { Ok((r.get(0)?, r.get(1)?)) },
        )
        .optional()
        .map_err(map_err)?;

    Ok(row.map(|(l, p)| (l, p as u8)))
}
