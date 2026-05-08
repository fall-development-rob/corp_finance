//! Cost-ledger value objects and the static per-million-token model rate
//! table.
//!
//! Money is integer `i64` cents throughout. We never use float for cost
//! arithmetic — even small per-token rounding drift is unacceptable in a
//! ledger that backs analyst budget alerts.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "managed_agent")]
use crate::managed_agent::types::CookbookTier;
use crate::mcp_servers::types::McpServerTier;

// ---------------------------------------------------------------------------
// Surface — re-exported from the shared kernel (Phase 27 cleanup).
// ---------------------------------------------------------------------------

/// A CFA runtime surface.
///
/// Re-exported from the shared kernel `corp_finance_core::surface::Surface`
/// (Phase 27 cleanup). The previous local duplicate was collapsed into the
/// shared kernel; `Surface::as_str` and `Surface::parse` now live on the
/// shared type.
pub use crate::surface::Surface;

// ---------------------------------------------------------------------------
// TierTag — flattened union of CookbookTier and McpServerTier for the ledger.
// ---------------------------------------------------------------------------

/// Tier classification persisted on a `CostEvent`.  Combines `CookbookTier`
/// (deploy-time) and `McpServerTier` (runtime) into a single tag space so the
/// ledger can be aggregated by tier without joining across two enums.
///
/// Per RUF-COST-002 / RUF-COST-INV-002, this enum is **derived** from those
/// two source-of-truth tier enums; no parallel tiering logic lives here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema_gen", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum TierTag {
    /// Cookbook published as `CookbookTier::CoreOnly`.
    CookbookCoreOnly,
    /// Cookbook published as `CookbookTier::Freemium`.
    CookbookFreemium,
    /// Cookbook published as `CookbookTier::PaidVendor`.
    CookbookPaidVendor,
    /// MCP server classified as `McpServerTier::FreeNative`.
    McpFreeNative,
    /// MCP server classified as `McpServerTier::FreePublicWithApiKey`.
    McpFreePublicWithApiKey,
    /// MCP server classified as `McpServerTier::Freemium`.
    McpFreemium,
    /// MCP server classified as `McpServerTier::PaidVendor`.
    McpPaidVendor,
    /// Surface event whose tier could not be determined at record time.
    /// Treated as paid-vendor for budget purposes by convention.
    Unknown,
}

impl TierTag {
    pub fn as_str(&self) -> &'static str {
        match self {
            TierTag::CookbookCoreOnly => "cookbook_core_only",
            TierTag::CookbookFreemium => "cookbook_freemium",
            TierTag::CookbookPaidVendor => "cookbook_paid_vendor",
            TierTag::McpFreeNative => "mcp_free_native",
            TierTag::McpFreePublicWithApiKey => "mcp_free_public_with_api_key",
            TierTag::McpFreemium => "mcp_freemium",
            TierTag::McpPaidVendor => "mcp_paid_vendor",
            TierTag::Unknown => "unknown",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "cookbook_core_only" => Some(TierTag::CookbookCoreOnly),
            "cookbook_freemium" => Some(TierTag::CookbookFreemium),
            "cookbook_paid_vendor" => Some(TierTag::CookbookPaidVendor),
            "mcp_free_native" => Some(TierTag::McpFreeNative),
            "mcp_free_public_with_api_key" => Some(TierTag::McpFreePublicWithApiKey),
            "mcp_freemium" => Some(TierTag::McpFreemium),
            "mcp_paid_vendor" => Some(TierTag::McpPaidVendor),
            "unknown" => Some(TierTag::Unknown),
            _ => None,
        }
    }

    /// True if this tier should never trigger budget thresholds.
    pub fn is_free_tier(&self) -> bool {
        matches!(
            self,
            TierTag::CookbookCoreOnly | TierTag::McpFreeNative | TierTag::McpFreePublicWithApiKey
        )
    }
}

#[cfg(feature = "managed_agent")]
impl From<CookbookTier> for TierTag {
    fn from(t: CookbookTier) -> Self {
        match t {
            CookbookTier::CoreOnly => TierTag::CookbookCoreOnly,
            CookbookTier::Freemium => TierTag::CookbookFreemium,
            CookbookTier::PaidVendor => TierTag::CookbookPaidVendor,
        }
    }
}

impl From<McpServerTier> for TierTag {
    fn from(t: McpServerTier) -> Self {
        match t {
            McpServerTier::FreeNative => TierTag::McpFreeNative,
            McpServerTier::FreePublicWithApiKey => TierTag::McpFreePublicWithApiKey,
            McpServerTier::Freemium => TierTag::McpFreemium,
            McpServerTier::PaidVendor => TierTag::McpPaidVendor,
        }
    }
}

// ---------------------------------------------------------------------------
// CostEvent — a single recorded surface-event cost row.
// ---------------------------------------------------------------------------

/// A single ledger row.  Emitted by the CLI tracing wrapper at subcommand
/// completion or by the MCP server wrapper at tool-handler completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema_gen", derive(schemars::JsonSchema))]
pub struct CostEvent {
    /// Stable identity for this event.
    pub event_id: Uuid,
    /// Which CFA surface produced this event.
    pub surface: Surface,
    /// Surface-event identifier (CLI subcommand name, MCP tool name,
    /// plugin hook id, slash-command id).
    pub surface_event_id: String,
    /// Model identifier (e.g. `claude-opus-4-7`).
    pub model: String,
    /// Input tokens consumed.
    pub tokens_in: u64,
    /// Output tokens consumed.
    pub tokens_out: u64,
    /// Dollarised cost in integer cents (no float, no Decimal).
    pub cost_cents: i64,
    /// Tier tag derived from CookbookTier or McpServerTier at record time.
    pub tier: TierTag,
    /// Optional tenant identifier (single-tenant default; populated by
    /// federation deployments).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
    /// Wall-clock timestamp.
    pub ts: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Budget value objects.
// ---------------------------------------------------------------------------

/// The period a budget covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema_gen", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum BudgetPeriod {
    Daily,
    Weekly,
    Monthly,
    /// All-time cumulative.
    Total,
}

impl BudgetPeriod {
    pub fn as_str(&self) -> &'static str {
        match self {
            BudgetPeriod::Daily => "daily",
            BudgetPeriod::Weekly => "weekly",
            BudgetPeriod::Monthly => "monthly",
            BudgetPeriod::Total => "total",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "daily" => Some(BudgetPeriod::Daily),
            "weekly" => Some(BudgetPeriod::Weekly),
            "monthly" => Some(BudgetPeriod::Monthly),
            "total" => Some(BudgetPeriod::Total),
            _ => None,
        }
    }
}

/// Budget definition.  Keys on `(surface_filter, tier_filter, period)`
/// inside the ledger.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema_gen", derive(schemars::JsonSchema))]
pub struct CostBudget {
    /// Optional surface filter; `None` covers all surfaces.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surface_filter: Option<Surface>,
    /// Optional tier filter; `None` covers all tiers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tier_filter: Option<TierTag>,
    pub period: BudgetPeriod,
    /// Hard cap in integer cents.
    pub limit_cents: i64,
    /// Warning threshold as a percentage (0..=100).  Crossing this fires the
    /// `budget_threshold_crossed` integration event but does not block.
    pub threshold_pct: u8,
}

// ---------------------------------------------------------------------------
// Static rate table — input cents/Mtok, output cents/Mtok.
// ---------------------------------------------------------------------------

/// `(model_id, input_cents_per_mtok, output_cents_per_mtok)` triples.
///
/// Cents-per-million-tokens convention: a row of `("foo", 1500, 7500)` means
/// 1 million input tokens cost $15.00, 1 million output tokens cost $75.00.
pub const MODEL_RATES_PER_MTOK: &[(&str, i64, i64)] = &[
    // Claude Opus 4.7 — premium tier (1M input cost = $15, 1M output cost = $75).
    ("claude-opus-4-7", 1500, 7500),
    // Claude Sonnet 4.6 — mid tier ($3 / $15 per Mtok).
    ("claude-sonnet-4-6", 300, 1500),
    // Claude Haiku 4.5 — cheap tier ($0.80 / $4 per Mtok).
    ("claude-haiku-4-5", 80, 400),
];

/// Compute integer-cent cost for a single surface event.  Unknown models fall
/// back to the highest known rate (Opus) so unbudgeted models are never
/// silently free.
pub fn cost_for(model: &str, tokens_in: u64, tokens_out: u64) -> i64 {
    let (in_rate, out_rate) = MODEL_RATES_PER_MTOK
        .iter()
        .find(|(m, _, _)| *m == model)
        .map(|(_, i, o)| (*i, *o))
        .unwrap_or_else(|| {
            // Conservative default: bill at Opus rate.
            let opus = MODEL_RATES_PER_MTOK
                .iter()
                .find(|(m, _, _)| *m == "claude-opus-4-7")
                .expect("opus row present in MODEL_RATES_PER_MTOK");
            (opus.1, opus.2)
        });

    // cost_cents = tokens * cents_per_mtok / 1_000_000, integer division.
    // Keep all intermediates in i128 to avoid overflow on heavy daily volume.
    let in_cost = (tokens_in as i128) * (in_rate as i128) / 1_000_000_i128;
    let out_cost = (tokens_out as i128) * (out_rate as i128) / 1_000_000_i128;
    let total = in_cost + out_cost;

    // Saturate into i64 — astronomical token counts shouldn't panic the
    // ledger.  Realistic values (< 10^12 tokens) are nowhere near i64::MAX.
    if total > i64::MAX as i128 {
        i64::MAX
    } else if total < i64::MIN as i128 {
        i64::MIN
    } else {
        total as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cost_for_known_model_matches_rate_table() {
        // 1 million input + 1 million output on Opus = 1500 + 7500 = 9000 cents.
        assert_eq!(cost_for("claude-opus-4-7", 1_000_000, 1_000_000), 9000);
        // 1 million input + 1 million output on Sonnet = 300 + 1500 = 1800.
        assert_eq!(cost_for("claude-sonnet-4-6", 1_000_000, 1_000_000), 1800);
        // 1 million input + 1 million output on Haiku = 80 + 400 = 480.
        assert_eq!(cost_for("claude-haiku-4-5", 1_000_000, 1_000_000), 480);
    }

    #[test]
    fn cost_for_unknown_model_falls_back_to_opus() {
        let unknown = cost_for("imaginary-model-2099", 1_000_000, 1_000_000);
        let opus = cost_for("claude-opus-4-7", 1_000_000, 1_000_000);
        assert_eq!(unknown, opus);
    }

    #[test]
    fn cost_for_handles_small_token_counts_with_integer_truncation() {
        // 1000 input tokens at 1500 cents/Mtok = 1.5 cents → truncates to 1.
        assert_eq!(cost_for("claude-opus-4-7", 1000, 0), 1);
        // 100 tokens — sub-cent — truncates to 0.
        assert_eq!(cost_for("claude-opus-4-7", 100, 0), 0);
    }

    #[test]
    fn tier_tag_round_trips_via_str() {
        for tag in [
            TierTag::CookbookCoreOnly,
            TierTag::CookbookFreemium,
            TierTag::CookbookPaidVendor,
            TierTag::McpFreeNative,
            TierTag::McpFreePublicWithApiKey,
            TierTag::McpFreemium,
            TierTag::McpPaidVendor,
            TierTag::Unknown,
        ] {
            assert_eq!(TierTag::parse(tag.as_str()), Some(tag));
        }
    }

    #[test]
    fn surface_round_trips_via_str() {
        for s in [Surface::Cli, Surface::Mcp, Surface::Skill, Surface::Plugin] {
            assert_eq!(Surface::parse(s.as_str()), Some(s));
        }
    }

    #[cfg(feature = "managed_agent")]
    #[test]
    fn tier_tag_from_cookbook_tier_is_correct() {
        assert_eq!(
            TierTag::from(CookbookTier::CoreOnly),
            TierTag::CookbookCoreOnly
        );
        assert_eq!(
            TierTag::from(CookbookTier::Freemium),
            TierTag::CookbookFreemium
        );
        assert_eq!(
            TierTag::from(CookbookTier::PaidVendor),
            TierTag::CookbookPaidVendor
        );
    }

    #[test]
    fn tier_tag_from_mcp_server_tier_is_correct() {
        assert_eq!(
            TierTag::from(McpServerTier::FreeNative),
            TierTag::McpFreeNative
        );
        assert_eq!(
            TierTag::from(McpServerTier::FreePublicWithApiKey),
            TierTag::McpFreePublicWithApiKey
        );
        assert_eq!(TierTag::from(McpServerTier::Freemium), TierTag::McpFreemium);
        assert_eq!(
            TierTag::from(McpServerTier::PaidVendor),
            TierTag::McpPaidVendor
        );
    }

    #[test]
    fn is_free_tier_matches_cookbook_core_only_and_native_mcp_only() {
        assert!(TierTag::CookbookCoreOnly.is_free_tier());
        assert!(TierTag::McpFreeNative.is_free_tier());
        assert!(TierTag::McpFreePublicWithApiKey.is_free_tier());
        assert!(!TierTag::CookbookFreemium.is_free_tier());
        assert!(!TierTag::CookbookPaidVendor.is_free_tier());
        assert!(!TierTag::McpFreemium.is_free_tier());
        assert!(!TierTag::McpPaidVendor.is_free_tier());
    }
}
