//! Aggregation queries for `cfa cost summary`.
//!
//! Backs the `--by surface | tier | tenant | surface_and_tier` flag of the
//! CLI subcommand.  All grouping happens server-side via SQL; the caller
//! supplies a `CostFilter` to scope the window and a `GroupBy` to bucket
//! the rows.

use serde::{Deserialize, Serialize};

use crate::CorpFinanceResult;

use super::ledger::{map_err, CostFilter, CostLedger};

/// What dimension to bucket the summary on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GroupBy {
    /// One row per `Surface` (cli / mcp / skill / plugin).
    Surface,
    /// One row per `TierTag` value (cookbook_* / mcp_* / unknown).
    Tier,
    /// One row per `tenant_id` (NULL collapsed to "default").
    Tenant,
    /// One row per `(surface, tier)` pair.
    SurfaceAndTier,
}

/// Per-bucket aggregate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CostSummaryRow {
    /// String key of the bucket (e.g. `"cli"`, `"mcp_paid_vendor"`,
    /// `"cli|mcp_freemium"`, `"default"`).
    pub key: String,
    /// Number of events in the bucket.
    pub count: u64,
    /// Total cents in the bucket.
    pub cents: i64,
    /// Total input tokens in the bucket.
    pub tokens_in: u64,
    /// Total output tokens in the bucket.
    pub tokens_out: u64,
}

/// Top-level summary returned to `cfa cost summary`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CostSummary {
    pub total_cents: i64,
    pub total_tokens_in: u64,
    pub total_tokens_out: u64,
    pub breakdown: Vec<CostSummaryRow>,
}

/// Run an aggregation query against the ledger.  Sums over all rows matching
/// `filter`, grouped by `group_by`.  Rows are returned ordered by `cents`
/// descending so the highest-cost bucket appears first.
pub fn summary(
    ledger: &CostLedger,
    filter: &CostFilter,
    group_by: GroupBy,
) -> CorpFinanceResult<CostSummary> {
    let (where_clause, params_vec) = build_where_for_summary(filter);

    let group_expr = match group_by {
        GroupBy::Surface => "surface".to_string(),
        GroupBy::Tier => "tier".to_string(),
        GroupBy::Tenant => "COALESCE(tenant_id, 'default')".to_string(),
        GroupBy::SurfaceAndTier => "surface || '|' || tier".to_string(),
    };

    let sql = format!(
        "SELECT {gb} AS bucket, \
                COUNT(*) AS event_count, \
                COALESCE(SUM(cost_cents), 0) AS cents, \
                COALESCE(SUM(tokens_in), 0) AS tokens_in, \
                COALESCE(SUM(tokens_out), 0) AS tokens_out \
         FROM cost_events {wc} \
         GROUP BY {gb} \
         ORDER BY cents DESC, bucket ASC",
        gb = group_expr,
        wc = where_clause,
    );

    let mut stmt = ledger.conn.prepare(&sql).map_err(map_err)?;
    let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec
        .iter()
        .map(|v| v as &dyn rusqlite::ToSql)
        .collect();

    let mut breakdown: Vec<CostSummaryRow> = Vec::new();
    let mut total_cents: i64 = 0;
    let mut total_tokens_in: u64 = 0;
    let mut total_tokens_out: u64 = 0;

    let rows = stmt
        .query_map(params_refs.as_slice(), |r| {
            let key: String = r.get(0)?;
            let count: i64 = r.get(1)?;
            let cents: i64 = r.get(2)?;
            let tokens_in: i64 = r.get(3)?;
            let tokens_out: i64 = r.get(4)?;
            Ok(CostSummaryRow {
                key,
                count: count.max(0) as u64,
                cents,
                tokens_in: tokens_in.max(0) as u64,
                tokens_out: tokens_out.max(0) as u64,
            })
        })
        .map_err(map_err)?;

    for row in rows {
        let row = row.map_err(map_err)?;
        total_cents = total_cents.saturating_add(row.cents);
        total_tokens_in = total_tokens_in.saturating_add(row.tokens_in);
        total_tokens_out = total_tokens_out.saturating_add(row.tokens_out);
        breakdown.push(row);
    }

    Ok(CostSummary {
        total_cents,
        total_tokens_in,
        total_tokens_out,
        breakdown,
    })
}

/// Mirror of `ledger::build_where`, but kept private so the SQL stays in one
/// crate file.  Avoids exposing rusqlite types from `mod ledger`.
fn build_where_for_summary(filter: &CostFilter) -> (String, Vec<rusqlite::types::Value>) {
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
