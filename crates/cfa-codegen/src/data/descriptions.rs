//! Hand-written rich descriptions for the most-used MCP tools.
//!
//! Mirrors `DESCRIPTION_OVERRIDES` in `scripts/gen-mcp-tools.py`.
//! Every other tool falls back to "<section>: <humanised name>." derived
//! from the NAPI section header.

/// Returns `Some(description)` if the tool has an override, else `None`.
///
/// Pattern note: kept as a function rather than a static map because there
/// are only 11 entries; the linear scan is faster than a hash lookup at
/// this size and avoids the lazy-static / once_cell dance.
pub fn lookup(name: &str) -> Option<&'static str> {
    Some(match name {
        "calculate_wacc" => {
            "Weighted Average Cost of Capital via CAPM. Inputs: risk_free_rate, \
equity_risk_premium, beta, cost_of_debt, tax_rate, debt_weight, \
equity_weight. Optional: size_premium, country_risk_premium, \
specific_risk_premium, unlevered_beta, target_debt_equity (Hamada). \
Returns wacc, cost_of_equity, after_tax_cost_of_debt, levered_beta."
        }
        "build_dcf" => {
            "Discounted Cash Flow valuation. Inputs: base_revenue, currency, \
revenue_growth_rates[], ebitda_margin, capex_as_pct_revenue, \
nwc_as_pct_revenue, tax_rate, wacc, terminal_method (GordonGrowth | \
ExitMultiple), terminal_growth_rate, shares_outstanding, net_debt. \
Returns enterprise_value, equity_value, per-share value, year-by-year \
projections."
        }
        "comps_analysis" => {
            "Trading comparables across peer set. Computes EV/EBITDA, EV/Revenue, \
P/E, P/B, PEG mean/median/high/low and derives implied target \
valuations."
        }
        "credit_metrics" => {
            "Standard credit metrics: leverage (Debt/EBITDA, Net Debt/EBITDA), \
coverage (interest, EBIT, DSCR), liquidity (current, quick), and \
synthetic rating (AAA→D)."
        }
        "debt_capacity" => {
            "Maximum sustainable debt at target rating given EBITDA, interest rate, \
tax rate, and rating-grade leverage thresholds."
        }
        "covenant_compliance" => {
            "Maintenance covenant headroom test: leverage, coverage, and DSCR vs. \
covenant levels, with EBITDA cushion calculation."
        }
        "altman_zscore" => {
            "Altman Z-Score distress prediction (manufacturing/private/non-mfg \
models). Returns score and zone (Safe / Grey / Distress)."
        }
        "build_lbo" => {
            "Full LBO model with sources & uses, pro-forma capital structure, \
debt schedule, equity returns, and IRR/MOIC waterfall."
        }
        "calculate_waterfall" => {
            "European or American waterfall: return-of-capital → preferred return \
→ catch-up → carried interest. Per-LP and GP cash splits."
        }
        "price_bond" => {
            "Bond pricing from yield: clean/dirty price, accrued interest, \
DV01. Supports semi-annual / annual / monthly compounding."
        }
        "price_option" => {
            "Option pricing: Black-Scholes (European), CRR binomial (American). \
Returns price + Greeks (delta, gamma, theta, vega, rho)."
        }
        _ => return None,
    })
}

/// Tools to skip when emitting MCP registrations (wrapper-shaped, takes
/// extra args beyond a single Input struct).
pub const SKIP: &[&str] = &["scenario_analysis"];

/// Friendly aliases preserved from cfa-core v0.1 so existing skills/commands
/// keep working. Each entry registers an additional tool name pointing to
/// the same WASM export. Order is preserved (LinkedHashMap-like).
pub const LEGACY_ALIASES: &[(&str, &str)] = &[
    ("wacc_calculator", "calculate_wacc"),
    ("dcf_model", "build_dcf"),
];
