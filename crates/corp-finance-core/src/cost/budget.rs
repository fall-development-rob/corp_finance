//! Budget management and threshold checking for the cost ledger.
//!
//! Budgets key on `(surface_filter, tier_filter, period)`.  A budget with
//! both `surface_filter = None` and `tier_filter = None` is the global
//! catch-all.  The threshold checker walks the relevant period window,
//! sums matching `cost_events`, and returns `Ok` / `Warn` / `Exceeded`.
//!
//! Per RUF-COST-004, free-tier surface events (`CookbookCoreOnly`,
//! `McpFreeNative`, `McpFreePublicWithApiKey`) have no monthly limit and no
//! warn thresholds — `check_threshold` returns `BudgetStatus::Ok { 0, ... }`
//! immediately for those tier filters.

use chrono::{Datelike, Duration, TimeZone, Utc, Weekday};
use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::CorpFinanceResult;

use super::ledger::{fetch_budget_row, map_err, CostFilter, CostLedger};
use super::types::{BudgetPeriod, CostBudget, Surface, TierTag};

/// Filter used by `get_budget()` — matches the budget primary key.
#[derive(Debug, Clone, Default)]
pub struct BudgetFilter {
    pub surface_filter: Option<Surface>,
    pub tier_filter: Option<TierTag>,
    pub period: Option<BudgetPeriod>,
}

/// Result of `check_threshold()`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum BudgetStatus {
    /// Within both warn and hard limits.
    Ok { used_cents: i64, limit_cents: i64 },
    /// Crossed the warn threshold but not the hard limit.
    Warn {
        used_cents: i64,
        limit_cents: i64,
        pct: u8,
    },
    /// Crossed the hard limit.
    Exceeded { used_cents: i64, limit_cents: i64 },
}

/// Insert-or-update a budget row.
pub fn set_budget(ledger: &CostLedger, budget: &CostBudget) -> CorpFinanceResult<()> {
    let surface_str = budget.surface_filter.map(|s| s.as_str().to_string());
    let tier_str = budget.tier_filter.map(|t| t.as_str().to_string());
    let period_str = budget.period.as_str();

    ledger
        .conn
        .execute(
            "INSERT INTO cost_budgets \
             (surface_filter, tier_filter, period, limit_cents, threshold_pct) \
             VALUES (?1, ?2, ?3, ?4, ?5) \
             ON CONFLICT(surface_filter, tier_filter, period) DO UPDATE SET \
                 limit_cents = excluded.limit_cents, \
                 threshold_pct = excluded.threshold_pct",
            params![
                surface_str,
                tier_str,
                period_str,
                budget.limit_cents,
                budget.threshold_pct as i64,
            ],
        )
        .map_err(map_err)?;
    Ok(())
}

/// Look up the budget row matching `filter`.  Returns `None` if no budget is
/// registered for that key.  When `filter.period` is `None`, defaults to
/// `BudgetPeriod::Monthly`.
pub fn get_budget(
    ledger: &CostLedger,
    filter: &BudgetFilter,
) -> CorpFinanceResult<Option<CostBudget>> {
    let period = filter.period.unwrap_or(BudgetPeriod::Monthly);
    let row = fetch_budget_row(
        &ledger.conn,
        filter.surface_filter,
        filter.tier_filter,
        period.as_str(),
    )?;

    Ok(row.map(|(limit_cents, threshold_pct)| CostBudget {
        surface_filter: filter.surface_filter,
        tier_filter: filter.tier_filter,
        period,
        limit_cents,
        threshold_pct,
    }))
}

/// Sum the relevant `cost_events` window and classify the budget status.
///
/// Free-tier budgets short-circuit to `BudgetStatus::Ok { 0, .. }`
/// regardless of usage (per RUF-COST-004 / RUF-COST-INV-002 — free-tier
/// events never trigger threshold events).
pub fn check_threshold(
    ledger: &CostLedger,
    budget: &CostBudget,
) -> CorpFinanceResult<BudgetStatus> {
    if let Some(t) = budget.tier_filter {
        if t.is_free_tier() {
            return Ok(BudgetStatus::Ok {
                used_cents: 0,
                limit_cents: budget.limit_cents,
            });
        }
    }

    let (since, until) = period_window(budget.period);
    let filter = CostFilter {
        surface: budget.surface_filter,
        tier: budget.tier_filter,
        tenant_id: None,
        since,
        until,
    };
    let used_cents = ledger.sum_cents(&filter)?;

    classify(used_cents, budget.limit_cents, budget.threshold_pct)
}

fn classify(
    used_cents: i64,
    limit_cents: i64,
    threshold_pct: u8,
) -> CorpFinanceResult<BudgetStatus> {
    if limit_cents <= 0 {
        // Treat zero/negative limits as "no cap" and never exceed.
        return Ok(BudgetStatus::Ok {
            used_cents,
            limit_cents,
        });
    }
    if used_cents >= limit_cents {
        return Ok(BudgetStatus::Exceeded {
            used_cents,
            limit_cents,
        });
    }
    let pct_used = ((used_cents as i128 * 100) / limit_cents as i128) as i64;
    if pct_used >= threshold_pct as i64 {
        Ok(BudgetStatus::Warn {
            used_cents,
            limit_cents,
            pct: pct_used.clamp(0, 100) as u8,
        })
    } else {
        Ok(BudgetStatus::Ok {
            used_cents,
            limit_cents,
        })
    }
}

/// Compute the (`since`, `until`) window for the named period.  `until` is
/// always `None` (means "now or later"); `since` is the start of the period.
fn period_window(
    period: BudgetPeriod,
) -> (Option<chrono::DateTime<Utc>>, Option<chrono::DateTime<Utc>>) {
    let now = Utc::now();
    let since = match period {
        BudgetPeriod::Total => return (None, None),
        BudgetPeriod::Daily => Utc
            .with_ymd_and_hms(now.year(), now.month(), now.day(), 0, 0, 0)
            .single(),
        BudgetPeriod::Weekly => {
            // ISO week: Monday-anchored.
            let weekday = now.weekday();
            let days_from_monday = match weekday {
                Weekday::Mon => 0,
                Weekday::Tue => 1,
                Weekday::Wed => 2,
                Weekday::Thu => 3,
                Weekday::Fri => 4,
                Weekday::Sat => 5,
                Weekday::Sun => 6,
            };
            let week_start = now.date_naive() - Duration::days(days_from_monday);
            Utc.with_ymd_and_hms(
                week_start.year(),
                week_start.month(),
                week_start.day(),
                0,
                0,
                0,
            )
            .single()
        }
        BudgetPeriod::Monthly => Utc
            .with_ymd_and_hms(now.year(), now.month(), 1, 0, 0, 0)
            .single(),
    };
    (since, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_ok_below_threshold() {
        let s = classify(40, 100, 80).unwrap();
        assert!(matches!(
            s,
            BudgetStatus::Ok {
                used_cents: 40,
                limit_cents: 100
            }
        ));
    }

    #[test]
    fn classify_warn_at_threshold() {
        let s = classify(80, 100, 80).unwrap();
        match s {
            BudgetStatus::Warn {
                used_cents,
                limit_cents,
                pct,
            } => {
                assert_eq!(used_cents, 80);
                assert_eq!(limit_cents, 100);
                assert_eq!(pct, 80);
            }
            other => panic!("expected Warn, got {:?}", other),
        }
    }

    #[test]
    fn classify_exceeded_at_or_above_limit() {
        let s = classify(105, 100, 80).unwrap();
        assert!(matches!(s, BudgetStatus::Exceeded { .. }));
        let s2 = classify(100, 100, 80).unwrap();
        assert!(matches!(s2, BudgetStatus::Exceeded { .. }));
    }

    #[test]
    fn classify_zero_limit_is_uncapped() {
        let s = classify(1_000_000, 0, 80).unwrap();
        assert!(matches!(s, BudgetStatus::Ok { .. }));
    }
}
