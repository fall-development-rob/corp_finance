//! Per-domain feature gate mapping for the WASM emitter.
//!
//! `corp-finance-core` exposes one Cargo feature per domain (`valuation`,
//! `credit`, `fixed_income`, …). The WASM bindings auto-generated from
//! `packages/bindings/src/lib.rs` are grouped under `// ---- Section ----`
//! comment headers whose names broadly match those features. Wrapping each
//! `wasm_tool!` invocation in `#[cfg(feature = "<domain>")]` lets downstream
//! consumers build a slim WASM artifact with only the domains they need.
//!
//! Two facts make this almost trivial in practice:
//!
//! 1. Every NAPI binding's function path begins with the domain module:
//!    `corp_finance_core::<domain>::<sub_module>::<fn_name>`. The first
//!    segment after `corp_finance_core::` is always the Cargo feature name.
//!    `feature_for_tool()` exploits this.
//!
//! 2. The section comment header above each binding is *almost* the same
//!    string in title case (`"Valuation"`, `"Fixed Income"`, …). Most
//!    sections snake-case-and-strip-suffix to the feature name, but a few
//!    use a shorter alias (`"Private Equity"` → `pe`,
//!    `"M&A"` → `ma`, `"Behavioral Finance"` → `behavioral`, …). The
//!    `SECTION_FEATURE_MAP` table below documents every such divergence.
//!
//! The emitter prefers (1) for correctness and uses (2) only as a sanity
//! check during `cargo test` — if the two ever disagree the test fails.

/// Manually curated `Section title → Cargo feature` lookup.
///
/// Used purely as a documentation/audit aid and as a test oracle. The
/// runtime emitter derives the feature from the function path so a missing
/// entry here is a test failure, not a runtime bug.
///
/// Section titles are matched after stripping any ` — Phase NN` suffix and
/// trimming whitespace. So `"Credit"` and `"Credit — Phase 2"` both map
/// to `credit`.
pub const SECTION_FEATURE_MAP: &[(&str, &str)] = &[
    // Direct snake_case (section title lower-cased and spaces → underscores).
    ("Valuation", "valuation"),
    ("Credit", "credit"),
    ("Portfolio", "portfolio"),
    ("Scenarios", "scenarios"),
    ("Derivatives", "derivatives"),
    ("Restructuring", "restructuring"),
    ("ESG", "esg"),
    ("Regulatory", "regulatory"),
    ("Insurance", "insurance"),
    ("Compliance", "compliance"),
    ("Sovereign", "sovereign"),
    ("Treasury", "treasury"),
    ("Infrastructure", "infrastructure"),
    ("Securitization", "securitization"),
    ("Workflows", "workflows"),
    ("Fixed Income", "fixed_income"),
    ("Real Assets", "real_assets"),
    ("Institutional Real Estate", "institutional_real_estate"),
    ("Monte Carlo", "monte_carlo"),
    ("Quant Risk", "quant_risk"),
    ("Private Credit", "private_credit"),
    ("Structured Products", "structured_products"),
    ("Trade Finance", "trade_finance"),
    ("Credit Derivatives", "credit_derivatives"),
    ("Lease Accounting", "lease_accounting"),
    ("Real Options", "real_options"),
    ("Equity Research", "equity_research"),
    ("Commodity Trading", "commodity_trading"),
    ("Quant Strategies", "quant_strategies"),
    ("Performance Attribution", "performance_attribution"),
    ("Credit Portfolio", "credit_portfolio"),
    ("Macro Economics", "macro_economics"),
    ("Onshore Structures", "onshore_structures"),
    ("Offshore Structures", "offshore_structures"),
    ("Offshore Structures Expansion", "offshore_structures"),
    ("Transfer Pricing", "transfer_pricing"),
    ("Tax Treaty", "tax_treaty"),
    ("Substance Requirements", "substance_requirements"),
    ("Regulatory Reporting", "regulatory_reporting"),
    ("AML Compliance", "aml_compliance"),
    ("Volatility Surface", "volatility_surface"),
    ("Portfolio Optimization", "portfolio_optimization"),
    ("Risk Budgeting", "risk_budgeting"),
    ("Market Microstructure", "market_microstructure"),
    ("Interest Rate Models", "interest_rate_models"),
    ("Mortgage Analytics", "mortgage_analytics"),
    ("Repo Financing", "repo_financing"),
    ("Credit Scoring", "credit_scoring"),
    ("Capital Allocation", "capital_allocation"),
    ("CLO Analytics", "clo_analytics"),
    ("Fund of Funds", "fund_of_funds"),
    ("Earnings Quality", "earnings_quality"),
    ("Bank Analytics", "bank_analytics"),
    ("Dividend Policy", "dividend_policy"),
    ("Carbon Markets", "carbon_markets"),
    ("Private Wealth", "private_wealth"),
    ("Emerging Markets", "emerging_markets"),
    ("Index Construction", "index_construction"),
    ("Financial Forensics", "financial_forensics"),
    // Section names that diverge from the snake_case rule. The Cargo
    // feature is shorter or differently named than the section title.
    ("Private Equity", "pe"),
    ("M&A", "ma"),
    ("Jurisdiction / Fund", "jurisdiction"),
    ("Three-Statement Model", "three_statement"),
    ("FX & Commodities", "fx_commodities"),
    ("Venture Capital", "venture"),
    ("FP&A (Financial Planning & Analysis)", "fpa"),
    ("Wealth Management", "wealth"),
    ("Crypto / Digital Assets", "crypto"),
    ("Municipal Bonds", "municipal"),
    ("Convertible Bonds", "convertibles"),
    ("Pension & LDI", "pension"),
    ("Behavioral Finance", "behavioral"),
    ("FATCA/CRS", "fatca_crs"),
    ("Inflation-Linked", "inflation_linked"),
];

/// Strip a trailing `— Phase NN` (or `- Phase NN`) suffix from a section
/// title, returning the canonical key used in `SECTION_FEATURE_MAP`.
pub fn normalize_section_title(title: &str) -> &str {
    // Match either em-dash or ASCII hyphen, surrounded by whitespace, then
    // `Phase`. Done by hand to avoid pulling regex into the data module.
    if let Some(idx) = title.find(" — Phase ").or_else(|| title.find(" - Phase ")) {
        title[..idx].trim()
    } else {
        title.trim()
    }
}

/// Look up a Cargo feature by section title (suffix-tolerant).
pub fn feature_for_section(title: &str) -> Option<&'static str> {
    let key = normalize_section_title(title);
    SECTION_FEATURE_MAP
        .iter()
        .find(|(s, _)| *s == key)
        .map(|(_, f)| *f)
}

/// Derive the Cargo feature gate for a tool from its fully-qualified
/// function path (e.g. `corp_finance_core::valuation::wacc::calculate_wacc`
/// → `valuation`).
///
/// This is the source of truth used by the WASM emitter. It does not
/// depend on the section header above the binding, so reordering or
/// retitling sections cannot break the build.
pub fn feature_for_tool(fn_path: &str) -> Option<&'static str> {
    let prefix = "corp_finance_core::";
    let rest = fn_path.strip_prefix(prefix)?;
    let domain = rest.split("::").next()?;
    // Validate against the canonical feature list so a typo in the NAPI
    // file or a new module added without a matching feature surfaces
    // immediately during codegen.
    if KNOWN_FEATURES.contains(&domain) {
        Some(static_feature_name(domain))
    } else {
        None
    }
}

/// All Cargo features defined on `corp-finance-core` that gate at least
/// one WASM-exported tool. Kept in sync with
/// `crates/corp-finance-core/Cargo.toml`. Pure-utility features that
/// gate no tools (`monte_carlo`'s underlying `scenarios`) are still
/// listed so the cross-check passes for empty sections.
pub const KNOWN_FEATURES: &[&str] = &[
    "valuation",
    "credit",
    "pe",
    "ma",
    "portfolio",
    "fixed_income",
    "three_statement",
    "jurisdiction",
    "derivatives",
    "quant_risk",
    "restructuring",
    "real_assets",
    "fx_commodities",
    "securitization",
    "venture",
    "esg",
    "regulatory",
    "insurance",
    "private_credit",
    "fpa",
    "wealth",
    "crypto",
    "trade_finance",
    "structured_products",
    "municipal",
    "credit_derivatives",
    "convertibles",
    "lease_accounting",
    "pension",
    "sovereign",
    "real_options",
    "equity_research",
    "commodity_trading",
    "quant_strategies",
    "treasury",
    "infrastructure",
    "behavioral",
    "performance_attribution",
    "credit_portfolio",
    "macro_economics",
    "compliance",
    "onshore_structures",
    "offshore_structures",
    "transfer_pricing",
    "tax_treaty",
    "fatca_crs",
    "substance_requirements",
    "regulatory_reporting",
    "aml_compliance",
    "volatility_surface",
    "portfolio_optimization",
    "risk_budgeting",
    "market_microstructure",
    "interest_rate_models",
    "mortgage_analytics",
    "inflation_linked",
    "repo_financing",
    "capital_allocation",
    "credit_scoring",
    "clo_analytics",
    "fund_of_funds",
    "earnings_quality",
    "dividend_policy",
    "carbon_markets",
    "bank_analytics",
    "private_wealth",
    "emerging_markets",
    "index_construction",
    "financial_forensics",
    "workflows",
    "institutional_real_estate",
    "scenarios",
    "monte_carlo",
];

/// Return the `'static`-lifetime canonical name for a feature so the
/// emitter can embed it directly in `#[cfg(feature = "...")]` strings.
fn static_feature_name(domain: &str) -> &'static str {
    // Linear scan: KNOWN_FEATURES is a `&'static [&'static str]`, so the
    // matching entry already has the right lifetime. 73 entries is small
    // enough that a HashMap would be slower (cache miss + hash) than a
    // straight scan with early-out string equality.
    for &f in KNOWN_FEATURES {
        if f == domain {
            return f;
        }
    }
    // `feature_for_tool` already filters by KNOWN_FEATURES, so this is
    // unreachable in practice; panic loudly if invariants ever change.
    panic!("static_feature_name called with non-canonical domain {domain:?}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_strips_phase_suffix() {
        assert_eq!(normalize_section_title("Credit"), "Credit");
        assert_eq!(normalize_section_title("Credit — Phase 2"), "Credit");
        assert_eq!(
            normalize_section_title("Crypto / Digital Assets — Phase 8"),
            "Crypto / Digital Assets"
        );
        assert_eq!(normalize_section_title("M&A"), "M&A");
    }

    #[test]
    fn feature_for_section_resolves_known_titles() {
        assert_eq!(feature_for_section("Valuation"), Some("valuation"));
        assert_eq!(feature_for_section("Credit — Phase 2"), Some("credit"));
        assert_eq!(feature_for_section("M&A"), Some("ma"));
        assert_eq!(feature_for_section("FX & Commodities"), Some("fx_commodities"));
        assert_eq!(
            feature_for_section("Offshore Structures Expansion — Phase 23"),
            Some("offshore_structures")
        );
    }

    #[test]
    fn feature_for_tool_extracts_first_segment() {
        assert_eq!(
            feature_for_tool("corp_finance_core::valuation::wacc::calculate_wacc"),
            Some("valuation")
        );
        assert_eq!(
            feature_for_tool("corp_finance_core::pe::lbo::build_lbo"),
            Some("pe")
        );
        assert_eq!(
            feature_for_tool("corp_finance_core::offshore_structures::cayman::analyze_cayman_structure"),
            Some("offshore_structures")
        );
    }

    #[test]
    fn feature_for_tool_rejects_unknown_module() {
        assert_eq!(
            feature_for_tool("corp_finance_core::not_a_real_module::foo::bar"),
            None
        );
        assert_eq!(feature_for_tool("std::ops::Add"), None);
    }

    #[test]
    fn known_features_includes_every_section_target() {
        for (title, feature) in SECTION_FEATURE_MAP {
            assert!(
                KNOWN_FEATURES.contains(feature),
                "section {title:?} maps to feature {feature:?} which is not in KNOWN_FEATURES"
            );
        }
    }
}
