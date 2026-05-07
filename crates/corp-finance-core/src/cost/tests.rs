//! Contract tests for the cost ledger.
//!
//! Implements RUF-COST-001..004 plus the two RUF-COST-INV invariants from
//! `docs/contracts/feature_audit_observability.yml`.

use chrono::Utc;
use tempfile::NamedTempFile;
use uuid::Uuid;

use super::budget::{check_threshold, get_budget, set_budget, BudgetFilter, BudgetStatus};
use super::ledger::{CostFilter, CostLedger};
use super::reporter::{summary, GroupBy};
use super::types::{cost_for, BudgetPeriod, CostBudget, CostEvent, Surface, TierTag};

fn make_event(
    surface: Surface,
    surface_event_id: &str,
    model: &str,
    tokens_in: u64,
    tokens_out: u64,
    tier: TierTag,
    tenant_id: Option<&str>,
) -> CostEvent {
    CostEvent {
        event_id: Uuid::now_v7(),
        surface,
        surface_event_id: surface_event_id.to_string(),
        model: model.to_string(),
        tokens_in,
        tokens_out,
        cost_cents: cost_for(model, tokens_in, tokens_out),
        tier,
        tenant_id: tenant_id.map(|s| s.to_string()),
        ts: Utc::now(),
    }
}

fn open_ledger() -> (NamedTempFile, CostLedger) {
    let f = NamedTempFile::new().expect("temp file");
    let ledger = CostLedger::open(f.path()).expect("open ledger");
    (f, ledger)
}

// ---------------------------------------------------------------------------
// RUF-COST-001 — record + retrieve persists every recorded event.
// ---------------------------------------------------------------------------

#[test]
fn ruf_cost_001_record_event_persists() {
    let (_f, ledger) = open_ledger();

    let event = make_event(
        Surface::Cli,
        "workflow.audit",
        "claude-opus-4-7",
        1_000_000,
        500_000,
        TierTag::McpFreeNative,
        Some("acme"),
    );
    ledger.record_event(&event).expect("record");

    let rows = ledger.query(&CostFilter::default()).expect("query");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].surface, Surface::Cli);
    assert_eq!(rows[0].surface_event_id, "workflow.audit");
    assert_eq!(rows[0].model, "claude-opus-4-7");
    assert_eq!(rows[0].tokens_in, 1_000_000);
    assert_eq!(rows[0].tokens_out, 500_000);
    // Opus rate: 1500 in/Mtok, 7500 out/Mtok → 1500 + 7500/2 = 5250 cents.
    assert_eq!(rows[0].cost_cents, 1500 + 3750);
    assert_eq!(rows[0].tier, TierTag::McpFreeNative);
    assert_eq!(rows[0].tenant_id.as_deref(), Some("acme"));
}

// ---------------------------------------------------------------------------
// RUF-COST-001 (continued) — summary aggregates by tier.
// ---------------------------------------------------------------------------

#[test]
fn ruf_cost_001_summary_by_tier_aggregates_per_tier() {
    let (_f, ledger) = open_ledger();

    // 2 CLI subcommands, 2 MCP tools, across 2 tier values.
    let events = [
        make_event(
            Surface::Cli,
            "workflow.audit",
            "claude-opus-4-7",
            500_000,
            500_000,
            TierTag::McpFreeNative,
            None,
        ),
        make_event(
            Surface::Cli,
            "managed-agent.deploy",
            "claude-sonnet-4-6",
            1_000_000,
            1_000_000,
            TierTag::McpFreeNative,
            None,
        ),
        make_event(
            Surface::Mcp,
            "fmp_quote",
            "claude-haiku-4-5",
            2_000_000,
            500_000,
            TierTag::McpFreemium,
            None,
        ),
        make_event(
            Surface::Mcp,
            "lseg_curve",
            "claude-opus-4-7",
            1_000_000,
            1_000_000,
            TierTag::McpPaidVendor,
            None,
        ),
    ];
    for e in &events {
        ledger.record_event(e).unwrap();
    }

    let s = summary(&ledger, &CostFilter::default(), GroupBy::Tier).unwrap();
    // Three distinct tiers represented → three breakdown rows.
    assert_eq!(s.breakdown.len(), 3);
    let keys: Vec<&str> = s.breakdown.iter().map(|r| r.key.as_str()).collect();
    assert!(keys.contains(&"mcp_free_native"));
    assert!(keys.contains(&"mcp_freemium"));
    assert!(keys.contains(&"mcp_paid_vendor"));

    // Total cents must equal sum of breakdown rows.
    let breakdown_total: i64 = s.breakdown.iter().map(|r| r.cents).sum();
    assert_eq!(s.total_cents, breakdown_total);
}

// ---------------------------------------------------------------------------
// RUF-COST-002 — TierTag is derived from CookbookTier / McpServerTier
// (no parallel cost_tier field).  This test exercises the conversion, which
// is the single source of truth.
// ---------------------------------------------------------------------------

#[test]
fn ruf_cost_002_threshold_event_payload_uses_cookbook_tier() {
    use crate::mcp_servers::types::McpServerTier;

    // Tier classification flows from the canonical enums.  Verify the
    // mappings round-trip through the ledger so any reclassification at the
    // call-site is preserved end-to-end.
    let tag_native: TierTag = McpServerTier::FreeNative.into();
    let tag_freemium: TierTag = McpServerTier::Freemium.into();
    let tag_paid: TierTag = McpServerTier::PaidVendor.into();

    let (_f, ledger) = open_ledger();
    for (tag, eid) in [
        (tag_native, "core_only_invoke"),
        (tag_freemium, "fmp_invoke"),
        (tag_paid, "lseg_invoke"),
    ] {
        ledger
            .record_event(&make_event(
                Surface::Mcp,
                eid,
                "claude-opus-4-7",
                1_000,
                1_000,
                tag,
                None,
            ))
            .unwrap();
    }

    // Pull rows back and confirm tier strings line up with TierTag::as_str.
    let rows = ledger.query(&CostFilter::default()).unwrap();
    let tiers: Vec<TierTag> = rows.iter().map(|r| r.tier).collect();
    assert!(tiers.contains(&TierTag::McpFreeNative));
    assert!(tiers.contains(&TierTag::McpFreemium));
    assert!(tiers.contains(&TierTag::McpPaidVendor));

    #[cfg(feature = "managed_agent")]
    {
        use crate::managed_agent::types::CookbookTier;
        assert_eq!(
            TierTag::from(CookbookTier::CoreOnly),
            TierTag::CookbookCoreOnly
        );
        assert_eq!(
            TierTag::from(CookbookTier::PaidVendor),
            TierTag::CookbookPaidVendor
        );
    }
}

// ---------------------------------------------------------------------------
// RUF-COST-003 — budget threshold crossing produces Warn / Exceeded.
// ---------------------------------------------------------------------------

#[test]
fn ruf_cost_003_budget_threshold_crossing_returns_warn_and_exceeded() {
    let (_f, ledger) = open_ledger();

    // Budget: $1.00 (= 100 cents) monthly limit, warn at 80%.
    let budget = CostBudget {
        surface_filter: Some(Surface::Mcp),
        tier_filter: Some(TierTag::McpPaidVendor),
        period: BudgetPeriod::Total,
        limit_cents: 100,
        threshold_pct: 80,
    };
    set_budget(&ledger, &budget).unwrap();

    // 79 cents — Ok.
    ledger
        .record_event(&CostEvent {
            event_id: Uuid::now_v7(),
            surface: Surface::Mcp,
            surface_event_id: "lseg_curve".to_string(),
            model: "claude-opus-4-7".to_string(),
            tokens_in: 0,
            tokens_out: 0,
            cost_cents: 79,
            tier: TierTag::McpPaidVendor,
            tenant_id: None,
            ts: Utc::now(),
        })
        .unwrap();
    let s1 = check_threshold(&ledger, &budget).unwrap();
    assert!(matches!(s1, BudgetStatus::Ok { .. }), "got {:?}", s1);

    // +2 cents → 81 — Warn at 80%.
    ledger
        .record_event(&CostEvent {
            event_id: Uuid::now_v7(),
            surface: Surface::Mcp,
            surface_event_id: "lseg_curve".to_string(),
            model: "claude-opus-4-7".to_string(),
            tokens_in: 0,
            tokens_out: 0,
            cost_cents: 2,
            tier: TierTag::McpPaidVendor,
            tenant_id: None,
            ts: Utc::now(),
        })
        .unwrap();
    let s2 = check_threshold(&ledger, &budget).unwrap();
    match s2 {
        BudgetStatus::Warn {
            used_cents,
            limit_cents,
            pct,
        } => {
            assert_eq!(used_cents, 81);
            assert_eq!(limit_cents, 100);
            assert_eq!(pct, 81);
        }
        other => panic!("expected Warn after crossing 80%, got {:?}", other),
    }

    // +25 cents → 106 — Exceeded.
    ledger
        .record_event(&CostEvent {
            event_id: Uuid::now_v7(),
            surface: Surface::Mcp,
            surface_event_id: "lseg_curve".to_string(),
            model: "claude-opus-4-7".to_string(),
            tokens_in: 0,
            tokens_out: 0,
            cost_cents: 25,
            tier: TierTag::McpPaidVendor,
            tenant_id: None,
            ts: Utc::now(),
        })
        .unwrap();
    let s3 = check_threshold(&ledger, &budget).unwrap();
    match s3 {
        BudgetStatus::Exceeded {
            used_cents,
            limit_cents,
        } => {
            assert_eq!(used_cents, 106);
            assert_eq!(limit_cents, 100);
        }
        other => panic!("expected Exceeded, got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// RUF-COST-004 — free-tier budgets short-circuit to Ok regardless of usage.
// ---------------------------------------------------------------------------

#[test]
fn ruf_cost_004_free_tier_budget_never_exceeds() {
    let (_f, ledger) = open_ledger();

    // Insert a deliberately low limit so the only way to return Ok is the
    // free-tier short-circuit.
    let budget = CostBudget {
        surface_filter: None,
        tier_filter: Some(TierTag::McpFreeNative),
        period: BudgetPeriod::Total,
        limit_cents: 1,
        threshold_pct: 50,
    };
    set_budget(&ledger, &budget).unwrap();

    // Record a $1000 event tagged free-tier.  In a paid budget this would
    // immediately trip Exceeded; in a free-tier budget, it must remain Ok.
    ledger
        .record_event(&CostEvent {
            event_id: Uuid::now_v7(),
            surface: Surface::Mcp,
            surface_event_id: "free_native_invoke".to_string(),
            model: "claude-opus-4-7".to_string(),
            tokens_in: 0,
            tokens_out: 0,
            cost_cents: 100_000,
            tier: TierTag::McpFreeNative,
            tenant_id: None,
            ts: Utc::now(),
        })
        .unwrap();

    let status = check_threshold(&ledger, &budget).unwrap();
    assert!(
        matches!(
            status,
            BudgetStatus::Ok {
                used_cents: 0,
                limit_cents: 1
            }
        ),
        "expected free-tier budget to short-circuit to Ok, got {:?}",
        status
    );
}

// ---------------------------------------------------------------------------
// RUF-COST-INV-001 — budget threshold computation is in-process and fast.
// We measure the round-trip from `record_event` → `check_threshold` and
// require it well below the 1000 ms invariant ceiling.
// ---------------------------------------------------------------------------

#[test]
fn ruf_cost_inv_001_threshold_check_under_1s() {
    use std::time::Instant;

    let (_f, ledger) = open_ledger();
    let budget = CostBudget {
        surface_filter: Some(Surface::Mcp),
        tier_filter: Some(TierTag::McpPaidVendor),
        period: BudgetPeriod::Total,
        limit_cents: 100,
        threshold_pct: 80,
    };
    set_budget(&ledger, &budget).unwrap();

    // Pre-load a few hundred rows so the SUM(...) walks a non-trivial table.
    for i in 0..200_u64 {
        ledger
            .record_event(&CostEvent {
                event_id: Uuid::now_v7(),
                surface: Surface::Mcp,
                surface_event_id: format!("paid_call_{}", i),
                model: "claude-opus-4-7".to_string(),
                tokens_in: 100,
                tokens_out: 100,
                cost_cents: 1,
                tier: TierTag::McpPaidVendor,
                tenant_id: None,
                ts: Utc::now(),
            })
            .unwrap();
    }

    let t0 = Instant::now();
    let _ = check_threshold(&ledger, &budget).unwrap();
    let elapsed = t0.elapsed();
    assert!(
        elapsed.as_millis() < 1000,
        "threshold check took {:?}, exceeds 1000 ms invariant",
        elapsed
    );
}

// ---------------------------------------------------------------------------
// RUF-COST-INV-002 — TierTag is the only translation surface; CookbookTier
// and McpServerTier remain the source of truth.  This test asserts the
// translation is total (every variant maps).
// ---------------------------------------------------------------------------

#[test]
fn ruf_cost_inv_002_tier_translation_is_total() {
    use crate::mcp_servers::types::McpServerTier;

    // Every McpServerTier maps cleanly onto a TierTag (no Unknown fallback).
    for t in [
        McpServerTier::FreeNative,
        McpServerTier::FreePublicWithApiKey,
        McpServerTier::Freemium,
        McpServerTier::PaidVendor,
    ] {
        let tag: TierTag = t.into();
        assert_ne!(tag, TierTag::Unknown);
    }

    #[cfg(feature = "managed_agent")]
    {
        use crate::managed_agent::types::CookbookTier;
        for t in [
            CookbookTier::CoreOnly,
            CookbookTier::Freemium,
            CookbookTier::PaidVendor,
        ] {
            let tag: TierTag = t.into();
            assert_ne!(tag, TierTag::Unknown);
        }
    }
}

// ---------------------------------------------------------------------------
// Bonus: budget round-trip via set_budget / get_budget.
// ---------------------------------------------------------------------------

#[test]
fn budget_round_trips_through_set_then_get() {
    let (_f, ledger) = open_ledger();

    let budget = CostBudget {
        surface_filter: Some(Surface::Mcp),
        tier_filter: Some(TierTag::McpPaidVendor),
        period: BudgetPeriod::Monthly,
        limit_cents: 50_000,
        threshold_pct: 80,
    };
    set_budget(&ledger, &budget).unwrap();

    let fetched = get_budget(
        &ledger,
        &BudgetFilter {
            surface_filter: Some(Surface::Mcp),
            tier_filter: Some(TierTag::McpPaidVendor),
            period: Some(BudgetPeriod::Monthly),
        },
    )
    .unwrap()
    .expect("budget present");
    assert_eq!(fetched, budget);

    // Re-set with a new limit; row should be UPSERTed.
    let updated = CostBudget {
        limit_cents: 60_000,
        threshold_pct: 90,
        ..budget.clone()
    };
    set_budget(&ledger, &updated).unwrap();

    let again = get_budget(
        &ledger,
        &BudgetFilter {
            surface_filter: Some(Surface::Mcp),
            tier_filter: Some(TierTag::McpPaidVendor),
            period: Some(BudgetPeriod::Monthly),
        },
    )
    .unwrap()
    .expect("budget present");
    assert_eq!(again.limit_cents, 60_000);
    assert_eq!(again.threshold_pct, 90);
}

// ---------------------------------------------------------------------------
// Bonus: query filtering by surface and tenant.
// ---------------------------------------------------------------------------

#[test]
fn query_filters_by_surface_and_tenant() {
    let (_f, ledger) = open_ledger();

    let scenarios = [
        (Surface::Cli, "acme", "evt_a"),
        (Surface::Cli, "globex", "evt_b"),
        (Surface::Mcp, "acme", "evt_c"),
        (Surface::Mcp, "acme", "evt_d"),
        (Surface::Plugin, "globex", "evt_e"),
    ];
    for (surface, tenant, eid) in scenarios.iter() {
        ledger
            .record_event(&make_event(
                *surface,
                eid,
                "claude-opus-4-7",
                1_000,
                1_000,
                TierTag::McpFreeNative,
                Some(*tenant),
            ))
            .unwrap();
    }

    let mcp_acme = ledger
        .query(&CostFilter {
            surface: Some(Surface::Mcp),
            tier: None,
            tenant_id: Some("acme".to_string()),
            since: None,
            until: None,
        })
        .unwrap();
    assert_eq!(mcp_acme.len(), 2);
    assert!(mcp_acme.iter().all(|e| e.surface == Surface::Mcp));
    assert!(mcp_acme
        .iter()
        .all(|e| e.tenant_id.as_deref() == Some("acme")));

    let any_globex = ledger
        .query(&CostFilter {
            surface: None,
            tier: None,
            tenant_id: Some("globex".to_string()),
            since: None,
            until: None,
        })
        .unwrap();
    assert_eq!(any_globex.len(), 2);
}
