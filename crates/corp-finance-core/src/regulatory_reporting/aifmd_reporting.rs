use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// AIFMD reporting frequency based on AUM thresholds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReportingFrequency {
    Quarterly,
    SemiAnnual,
    Annual,
    Exempt,
}

/// AIFMD strategy classification for Annex IV.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AifmdStrategy {
    Equity,
    FixedIncome,
    EventDriven,
    Credit,
    Macro,
    RelativeValue,
    ManagedFutures,
    MultiStrategy,
    Other,
}

// ---------------------------------------------------------------------------
// Input types
// ---------------------------------------------------------------------------

/// Fund-level information for AIFMD Annex IV reporting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundInfo {
    pub name: String,
    pub nav: Decimal,
    pub strategy: AifmdStrategy,
    pub domicile: String,
    pub leverage_gross: Decimal,
    pub leverage_commitment: Decimal,
    pub investor_count: u32,
    pub largest_investor_pct: Decimal,
    pub redemption_frequency: String,
    pub notice_period_days: u32,
    pub has_gates: bool,
    pub has_lockup: bool,
    pub lockup_months: u32,
    pub side_pocket_pct: Decimal,
}

/// Counterparty exposure entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CounterpartyExposure {
    pub name: String,
    pub exposure_pct: Decimal,
}

/// Liquidity profile — percentage of NAV redeemable within each bucket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityProfile {
    pub pct_1d: Decimal,
    pub pct_2_7d: Decimal,
    pub pct_8_30d: Decimal,
    pub pct_31_90d: Decimal,
    pub pct_91_180d: Decimal,
    pub pct_181_365d: Decimal,
    pub pct_over_365d: Decimal,
}

/// Market exposure entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketExposure {
    pub market: String,
    pub pct: Decimal,
}

/// Input for AIFMD Annex IV report generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AifmdReportingInput {
    pub aifm_name: String,
    pub aifm_jurisdiction: String,
    pub total_aum: Decimal,
    pub funds: Vec<FundInfo>,
    pub reporting_period_end: String,
    pub long_exposures: Decimal,
    pub short_exposures: Decimal,
    pub top_counterparties: Vec<CounterpartyExposure>,
    pub liquidity_profile: LiquidityProfile,
    pub principal_markets: Vec<MarketExposure>,
    pub stress_equity_impact: Decimal,
    pub stress_rates_impact: Decimal,
    pub stress_fx_impact: Decimal,
    pub stress_credit_impact: Decimal,
}

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// Strategy breakdown in the AIFM-level report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyBreakdown {
    pub strategy: AifmdStrategy,
    pub fund_count: u32,
    pub total_nav: Decimal,
    pub pct_of_aum: Decimal,
}

/// AIFM-level report data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AifmReport {
    pub total_aum: Decimal,
    pub strategy_breakdown: Vec<StrategyBreakdown>,
    pub leverage_gross: Decimal,
    pub leverage_commitment: Decimal,
    pub top_markets: Vec<MarketExposure>,
    pub liquidity_profile: LiquidityProfile,
}

/// AIF-level (per-fund) report data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AifReport {
    pub fund_name: String,
    pub nav: Decimal,
    pub strategy: AifmdStrategy,
    pub domicile: String,
    pub leverage_gross: Decimal,
    pub leverage_commitment: Decimal,
    pub investor_concentration: Decimal,
    pub investor_count: u32,
    pub redemption_frequency: String,
    pub notice_period_days: u32,
    pub has_gates: bool,
    pub has_lockup: bool,
    pub lockup_months: u32,
    pub side_pocket_pct: Decimal,
    pub top_counterparties: Vec<CounterpartyExposure>,
    pub enhanced_reporting_required: bool,
}

/// Stress test summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressTestSummary {
    pub equity_shock_pct: Decimal,
    pub equity_impact: Decimal,
    pub rates_shock_bps: Decimal,
    pub rates_impact: Decimal,
    pub fx_shock_pct: Decimal,
    pub fx_impact: Decimal,
    pub credit_shock_bps: Decimal,
    pub credit_impact: Decimal,
    pub worst_case_impact: Decimal,
    pub combined_impact: Decimal,
}

/// Complete AIFMD Annex IV report output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AifmdReportingOutput {
    pub reporting_frequency: ReportingFrequency,
    pub aifm_report: AifmReport,
    pub fund_reports: Vec<AifReport>,
    pub leverage_flags: Vec<String>,
    pub stress_test_summary: StressTestSummary,
    pub data_quality_issues: Vec<String>,
    pub filing_deadline: String,
    pub compliance_score: Decimal,
    pub methodology: String,
    pub assumptions: Vec<String>,
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/// Determine reporting frequency from AUM in EUR.
fn determine_reporting_frequency(total_aum: Decimal) -> ReportingFrequency {
    let one_billion = dec!(1_000_000_000);
    let one_hundred_million = dec!(100_000_000);

    if total_aum >= one_billion {
        ReportingFrequency::Quarterly
    } else if total_aum >= one_hundred_million {
        // Both €100M-€500M and €500M-€1B bands require semi-annual reporting
        ReportingFrequency::SemiAnnual
    } else {
        ReportingFrequency::Annual
    }
}

/// Compute the filing deadline based on reporting period end and frequency.
fn compute_filing_deadline(period_end: &str, freq: &ReportingFrequency) -> String {
    // AIFMD filing deadlines:
    //   Quarterly: 1 month after quarter end
    //   Semi-annual: 2 months after period end
    //   Annual: 4 months after fiscal year end
    //   Exempt: N/A
    let days_offset = match freq {
        ReportingFrequency::Quarterly => 30,
        ReportingFrequency::SemiAnnual => 60,
        ReportingFrequency::Annual => 120,
        ReportingFrequency::Exempt => 0,
    };

    if *freq == ReportingFrequency::Exempt {
        return "N/A".to_string();
    }

    if let Ok(date) = chrono::NaiveDate::parse_from_str(period_end, "%Y-%m-%d") {
        let deadline = date + chrono::Duration::days(days_offset);
        deadline.format("%Y-%m-%d").to_string()
    } else {
        format!("{} + {} days", period_end, days_offset)
    }
}

/// Validate liquidity profile sums to approximately 100%.
fn validate_liquidity_profile(lp: &LiquidityProfile) -> Vec<String> {
    let mut issues = Vec::new();
    let sum = lp.pct_1d
        + lp.pct_2_7d
        + lp.pct_8_30d
        + lp.pct_31_90d
        + lp.pct_91_180d
        + lp.pct_181_365d
        + lp.pct_over_365d;

    let hundred = dec!(100);
    let tolerance = dec!(1);
    if (sum - hundred).abs() > tolerance {
        issues.push(format!(
            "Liquidity profile sums to {}%, expected ~100%",
            sum
        ));
    }

    // Check for negative buckets
    let buckets = [
        ("pct_1d", lp.pct_1d),
        ("pct_2_7d", lp.pct_2_7d),
        ("pct_8_30d", lp.pct_8_30d),
        ("pct_31_90d", lp.pct_31_90d),
        ("pct_91_180d", lp.pct_91_180d),
        ("pct_181_365d", lp.pct_181_365d),
        ("pct_over_365d", lp.pct_over_365d),
    ];
    for (name, val) in &buckets {
        if *val < dec!(0) {
            issues.push(format!("Liquidity bucket {} is negative: {}", name, val));
        }
    }
    issues
}

/// Build the strategy breakdown from fund list.
fn build_strategy_breakdown(funds: &[FundInfo], total_aum: Decimal) -> Vec<StrategyBreakdown> {
    use std::collections::HashMap;

    let mut map: HashMap<String, (AifmdStrategy, u32, Decimal)> = HashMap::new();

    for f in funds {
        let key = format!("{:?}", f.strategy);
        let entry = map
            .entry(key)
            .or_insert_with(|| (f.strategy.clone(), 0, dec!(0)));
        entry.1 += 1;
        entry.2 += f.nav;
    }

    let mut result: Vec<StrategyBreakdown> = map
        .into_values()
        .map(|(strategy, fund_count, total_nav)| {
            let pct = if total_aum > dec!(0) {
                total_nav * dec!(100) / total_aum
            } else {
                dec!(0)
            };
            StrategyBreakdown {
                strategy,
                fund_count,
                total_nav,
                pct_of_aum: pct,
            }
        })
        .collect();

    result.sort_by(|a, b| b.total_nav.cmp(&a.total_nav));
    result
}

/// Compute aggregate leverage (gross method) from long and short exposures.
fn compute_gross_leverage(long: Decimal, short: Decimal, nav: Decimal) -> Decimal {
    if nav == dec!(0) {
        return dec!(0);
    }
    (long.abs() + short.abs()) / nav
}

/// Compute aggregate leverage (commitment method) — applies netting.
/// For simplicity, commitment = gross * 0.8 as a conservative estimate.
fn compute_commitment_leverage(long: Decimal, short: Decimal, nav: Decimal) -> Decimal {
    if nav == dec!(0) {
        return dec!(0);
    }
    let gross = long.abs() + short.abs();
    // Commitment method allows netting of hedged positions: ~80% of gross
    let netting_factor = dec!(0.8);
    (gross * netting_factor) / nav
}

/// Calculate compliance score based on data quality.
fn calculate_compliance_score(
    input: &AifmdReportingInput,
    data_quality_issues: &[String],
) -> Decimal {
    let mut score = dec!(100);

    // Deduct for data quality issues
    let issue_deduction = dec!(5);
    let issue_count = Decimal::from(data_quality_issues.len() as u32);
    score -= issue_deduction * issue_count;

    // Deduct if fewer than 5 counterparties reported
    if input.top_counterparties.len() < 5 {
        score -= dec!(5);
    }

    // Deduct if fewer than 5 principal markets
    if input.principal_markets.len() < 5 {
        score -= dec!(3);
    }

    // Deduct if no funds provided
    if input.funds.is_empty() {
        score -= dec!(20);
    }

    // Deduct if missing stress test data
    if input.stress_equity_impact == dec!(0)
        && input.stress_rates_impact == dec!(0)
        && input.stress_fx_impact == dec!(0)
        && input.stress_credit_impact == dec!(0)
    {
        score -= dec!(10);
    }

    // Floor at 0
    if score < dec!(0) {
        score = dec!(0);
    }

    score
}

// ---------------------------------------------------------------------------
// Main function
// ---------------------------------------------------------------------------

/// Generate an AIFMD Annex IV report for the given AIFM and its funds.
///
/// This function computes:
/// - Reporting frequency based on AUM thresholds
/// - AIFM-level report with strategy breakdown, leverage, liquidity
/// - Per-fund (AIF-level) reports
/// - Leverage flags for >3x commitment leverage
/// - Stress test summary
/// - Data quality checks and compliance scoring
pub fn generate_aifmd_report(
    input: &AifmdReportingInput,
) -> CorpFinanceResult<AifmdReportingOutput> {
    let mut warnings: Vec<String> = Vec::new();
    let mut data_quality_issues: Vec<String> = Vec::new();

    // --- Input validation ---
    if input.aifm_name.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "aifm_name".to_string(),
            reason: "AIFM name must not be empty".to_string(),
        });
    }

    if input.total_aum < dec!(0) {
        return Err(CorpFinanceError::InvalidInput {
            field: "total_aum".to_string(),
            reason: "Total AUM cannot be negative".to_string(),
        });
    }

    if input.reporting_period_end.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "reporting_period_end".to_string(),
            reason: "Reporting period end date is required".to_string(),
        });
    }

    // Validate date format
    if chrono::NaiveDate::parse_from_str(&input.reporting_period_end, "%Y-%m-%d").is_err() {
        return Err(CorpFinanceError::DateError(format!(
            "Invalid reporting_period_end '{}', expected YYYY-MM-DD",
            input.reporting_period_end
        )));
    }

    // Validate fund data
    for (i, fund) in input.funds.iter().enumerate() {
        if fund.nav < dec!(0) {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("funds[{}].nav", i),
                reason: "Fund NAV cannot be negative".to_string(),
            });
        }
        if fund.largest_investor_pct < dec!(0) || fund.largest_investor_pct > dec!(100) {
            data_quality_issues.push(format!(
                "Fund '{}' largest_investor_pct ({}) outside 0-100 range",
                fund.name, fund.largest_investor_pct
            ));
        }
        if fund.side_pocket_pct < dec!(0) || fund.side_pocket_pct > dec!(100) {
            data_quality_issues.push(format!(
                "Fund '{}' side_pocket_pct ({}) outside 0-100 range",
                fund.name, fund.side_pocket_pct
            ));
        }
        if fund.leverage_gross < dec!(0) {
            data_quality_issues.push(format!(
                "Fund '{}' has negative gross leverage: {}",
                fund.name, fund.leverage_gross
            ));
        }
        if fund.name.is_empty() {
            data_quality_issues.push(format!("funds[{}] has an empty name", i));
        }
    }

    // Validate liquidity profile
    let lp_issues = validate_liquidity_profile(&input.liquidity_profile);
    data_quality_issues.extend(lp_issues);

    // Validate counterparty exposures
    let cp_sum: Decimal = input
        .top_counterparties
        .iter()
        .map(|c| c.exposure_pct)
        .sum();
    if cp_sum > dec!(100) {
        data_quality_issues.push(format!(
            "Total counterparty exposure percentages sum to {} (>100%)",
            cp_sum
        ));
    }

    // Validate market exposures
    let mkt_sum: Decimal = input.principal_markets.iter().map(|m| m.pct).sum();
    if mkt_sum > dec!(100) {
        data_quality_issues.push(format!(
            "Total market exposure percentages sum to {} (>100%)",
            mkt_sum
        ));
    }

    // --- Determine reporting frequency ---
    let reporting_frequency = determine_reporting_frequency(input.total_aum);

    // --- Filing deadline ---
    let filing_deadline =
        compute_filing_deadline(&input.reporting_period_end, &reporting_frequency);

    // --- Aggregate NAV from funds ---
    let aggregate_nav: Decimal = input.funds.iter().map(|f| f.nav).sum();

    if aggregate_nav > dec!(0)
        && (aggregate_nav - input.total_aum).abs() > input.total_aum * dec!(0.1)
    {
        warnings.push(format!(
            "Sum of fund NAVs ({}) differs from total_aum ({}) by >10%",
            aggregate_nav, input.total_aum
        ));
    }

    // --- Strategy breakdown ---
    let strategy_breakdown = build_strategy_breakdown(&input.funds, input.total_aum);

    // --- Leverage at AIFM level ---
    let aifm_leverage_gross =
        compute_gross_leverage(input.long_exposures, input.short_exposures, aggregate_nav);
    let aifm_leverage_commitment =
        compute_commitment_leverage(input.long_exposures, input.short_exposures, aggregate_nav);

    // --- Top markets (top 5) ---
    let mut top_markets = input.principal_markets.clone();
    top_markets.sort_by(|a, b| b.pct.cmp(&a.pct));
    top_markets.truncate(5);

    // --- AIFM report ---
    let aifm_report = AifmReport {
        total_aum: input.total_aum,
        strategy_breakdown,
        leverage_gross: aifm_leverage_gross,
        leverage_commitment: aifm_leverage_commitment,
        top_markets,
        liquidity_profile: input.liquidity_profile.clone(),
    };

    // --- Fund-level reports ---
    let mut leverage_flags: Vec<String> = Vec::new();
    let commitment_threshold = dec!(3);

    // Top counterparties limited to 5
    let mut top_cp = input.top_counterparties.clone();
    top_cp.sort_by(|a, b| b.exposure_pct.cmp(&a.exposure_pct));
    top_cp.truncate(5);

    let fund_reports: Vec<AifReport> = input
        .funds
        .iter()
        .map(|f| {
            let enhanced = f.leverage_commitment > commitment_threshold;
            if enhanced {
                leverage_flags.push(format!(
                    "Fund '{}' commitment leverage ({}) exceeds 3x threshold — enhanced reporting required",
                    f.name, f.leverage_commitment
                ));
            }
            AifReport {
                fund_name: f.name.clone(),
                nav: f.nav,
                strategy: f.strategy.clone(),
                domicile: f.domicile.clone(),
                leverage_gross: f.leverage_gross,
                leverage_commitment: f.leverage_commitment,
                investor_concentration: f.largest_investor_pct,
                investor_count: f.investor_count,
                redemption_frequency: f.redemption_frequency.clone(),
                notice_period_days: f.notice_period_days,
                has_gates: f.has_gates,
                has_lockup: f.has_lockup,
                lockup_months: f.lockup_months,
                side_pocket_pct: f.side_pocket_pct,
                top_counterparties: top_cp.clone(),
                enhanced_reporting_required: enhanced,
            }
        })
        .collect();

    // Check AIFM-level leverage too
    if aifm_leverage_commitment > commitment_threshold {
        leverage_flags.push(format!(
            "AIFM-level commitment leverage ({}) exceeds 3x threshold",
            aifm_leverage_commitment
        ));
    }

    // --- Stress test summary ---
    let stress_test_summary = StressTestSummary {
        equity_shock_pct: dec!(-30),
        equity_impact: input.stress_equity_impact,
        rates_shock_bps: dec!(250),
        rates_impact: input.stress_rates_impact,
        fx_shock_pct: dec!(-20),
        fx_impact: input.stress_fx_impact,
        credit_shock_bps: dec!(400),
        credit_impact: input.stress_credit_impact,
        worst_case_impact: *[
            input.stress_equity_impact,
            input.stress_rates_impact,
            input.stress_fx_impact,
            input.stress_credit_impact,
        ]
        .iter()
        .min()
        .unwrap_or(&dec!(0)),
        combined_impact: input.stress_equity_impact
            + input.stress_rates_impact
            + input.stress_fx_impact
            + input.stress_credit_impact,
    };

    // Warn if combined stress exceeds -50% NAV
    if stress_test_summary.combined_impact < dec!(-50) {
        warnings.push(format!(
            "Combined stress impact ({}) exceeds -50% of NAV — review risk management",
            stress_test_summary.combined_impact
        ));
    }

    // --- Compliance score ---
    let compliance_score = calculate_compliance_score(input, &data_quality_issues);

    // --- Methodology ---
    let methodology = "AIFMD Annex IV reporting per Directive 2011/61/EU and \
        Commission Delegated Regulation (EU) No 231/2013. Leverage calculated using \
        both gross and commitment methods per ESMA guidelines (ESMA/2014/869). \
        Stress tests apply standard shocks: equity -30%, rates +250bps, FX -20%, \
        credit spreads +400bps."
        .to_string();

    let assumptions = vec![
        "Currency: all values assumed in EUR unless otherwise stated".to_string(),
        "Commitment method netting factor: 80% of gross (conservative estimate)".to_string(),
        "Liquidity profile based on reported redemption terms, not market conditions".to_string(),
        "Stress impacts are user-provided point estimates, not distribution-based".to_string(),
        "Filing deadlines: quarterly=30d, semi-annual=60d, annual=120d from period end".to_string(),
    ];

    Ok(AifmdReportingOutput {
        reporting_frequency,
        aifm_report,
        fund_reports,
        leverage_flags,
        stress_test_summary,
        data_quality_issues,
        filing_deadline,
        compliance_score,
        methodology,
        assumptions,
        warnings,
    })
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: create a default valid input
    fn default_input() -> AifmdReportingInput {
        AifmdReportingInput {
            aifm_name: "Test AIFM".to_string(),
            aifm_jurisdiction: "Luxembourg".to_string(),
            total_aum: dec!(2_000_000_000),
            funds: vec![
                FundInfo {
                    name: "Fund Alpha".to_string(),
                    nav: dec!(1_200_000_000),
                    strategy: AifmdStrategy::Equity,
                    domicile: "Ireland".to_string(),
                    leverage_gross: dec!(2.0),
                    leverage_commitment: dec!(1.5),
                    investor_count: 50,
                    largest_investor_pct: dec!(25),
                    redemption_frequency: "Monthly".to_string(),
                    notice_period_days: 30,
                    has_gates: false,
                    has_lockup: false,
                    lockup_months: 0,
                    side_pocket_pct: dec!(0),
                },
                FundInfo {
                    name: "Fund Beta".to_string(),
                    nav: dec!(800_000_000),
                    strategy: AifmdStrategy::Credit,
                    domicile: "Luxembourg".to_string(),
                    leverage_gross: dec!(1.8),
                    leverage_commitment: dec!(1.3),
                    investor_count: 30,
                    largest_investor_pct: dec!(40),
                    redemption_frequency: "Quarterly".to_string(),
                    notice_period_days: 45,
                    has_gates: true,
                    has_lockup: true,
                    lockup_months: 12,
                    side_pocket_pct: dec!(5),
                },
            ],
            reporting_period_end: "2024-12-31".to_string(),
            long_exposures: dec!(3_000_000_000),
            short_exposures: dec!(500_000_000),
            top_counterparties: vec![
                CounterpartyExposure {
                    name: "Goldman Sachs".to_string(),
                    exposure_pct: dec!(20),
                },
                CounterpartyExposure {
                    name: "JP Morgan".to_string(),
                    exposure_pct: dec!(15),
                },
                CounterpartyExposure {
                    name: "Morgan Stanley".to_string(),
                    exposure_pct: dec!(10),
                },
            ],
            liquidity_profile: LiquidityProfile {
                pct_1d: dec!(10),
                pct_2_7d: dec!(15),
                pct_8_30d: dec!(25),
                pct_31_90d: dec!(20),
                pct_91_180d: dec!(15),
                pct_181_365d: dec!(10),
                pct_over_365d: dec!(5),
            },
            principal_markets: vec![
                MarketExposure {
                    market: "US Equities".to_string(),
                    pct: dec!(40),
                },
                MarketExposure {
                    market: "EU Corporate Bonds".to_string(),
                    pct: dec!(25),
                },
                MarketExposure {
                    market: "UK Equities".to_string(),
                    pct: dec!(15),
                },
            ],
            stress_equity_impact: dec!(-12),
            stress_rates_impact: dec!(-5),
            stress_fx_impact: dec!(-3),
            stress_credit_impact: dec!(-8),
        }
    }

    // --- Reporting frequency tests ---

    #[test]
    fn test_quarterly_frequency_above_1b() {
        let input = default_input();
        let out = generate_aifmd_report(&input).unwrap();
        assert_eq!(out.reporting_frequency, ReportingFrequency::Quarterly);
    }

    #[test]
    fn test_semiannual_frequency_500m_to_1b() {
        let mut input = default_input();
        input.total_aum = dec!(750_000_000);
        let out = generate_aifmd_report(&input).unwrap();
        assert_eq!(out.reporting_frequency, ReportingFrequency::SemiAnnual);
    }

    #[test]
    fn test_semiannual_frequency_100m_to_500m() {
        let mut input = default_input();
        input.total_aum = dec!(200_000_000);
        let out = generate_aifmd_report(&input).unwrap();
        assert_eq!(out.reporting_frequency, ReportingFrequency::SemiAnnual);
    }

    #[test]
    fn test_annual_frequency_below_100m() {
        let mut input = default_input();
        input.total_aum = dec!(50_000_000);
        let out = generate_aifmd_report(&input).unwrap();
        assert_eq!(out.reporting_frequency, ReportingFrequency::Annual);
    }

    #[test]
    fn test_frequency_at_exact_1b_boundary() {
        let mut input = default_input();
        input.total_aum = dec!(1_000_000_000);
        let out = generate_aifmd_report(&input).unwrap();
        assert_eq!(out.reporting_frequency, ReportingFrequency::Quarterly);
    }

    #[test]
    fn test_frequency_at_exact_500m_boundary() {
        let mut input = default_input();
        input.total_aum = dec!(500_000_000);
        let out = generate_aifmd_report(&input).unwrap();
        assert_eq!(out.reporting_frequency, ReportingFrequency::SemiAnnual);
    }

    #[test]
    fn test_frequency_at_exact_100m_boundary() {
        let mut input = default_input();
        input.total_aum = dec!(100_000_000);
        let out = generate_aifmd_report(&input).unwrap();
        assert_eq!(out.reporting_frequency, ReportingFrequency::SemiAnnual);
    }

    #[test]
    fn test_frequency_just_below_100m() {
        let mut input = default_input();
        input.total_aum = dec!(99_999_999);
        let out = generate_aifmd_report(&input).unwrap();
        assert_eq!(out.reporting_frequency, ReportingFrequency::Annual);
    }

    // --- Validation tests ---

    #[test]
    fn test_empty_aifm_name_rejected() {
        let mut input = default_input();
        input.aifm_name = "".to_string();
        let err = generate_aifmd_report(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "aifm_name");
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[test]
    fn test_negative_aum_rejected() {
        let mut input = default_input();
        input.total_aum = dec!(-100);
        let err = generate_aifmd_report(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "total_aum");
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[test]
    fn test_empty_reporting_period_rejected() {
        let mut input = default_input();
        input.reporting_period_end = "".to_string();
        let err = generate_aifmd_report(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "reporting_period_end");
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[test]
    fn test_invalid_date_format_rejected() {
        let mut input = default_input();
        input.reporting_period_end = "31/12/2024".to_string();
        let err = generate_aifmd_report(&input).unwrap_err();
        match err {
            CorpFinanceError::DateError(_) => {}
            _ => panic!("Expected DateError"),
        }
    }

    #[test]
    fn test_negative_fund_nav_rejected() {
        let mut input = default_input();
        input.funds[0].nav = dec!(-100);
        let err = generate_aifmd_report(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert!(field.contains("nav"));
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    // --- AIFM report tests ---

    #[test]
    fn test_aifm_report_total_aum() {
        let input = default_input();
        let out = generate_aifmd_report(&input).unwrap();
        assert_eq!(out.aifm_report.total_aum, dec!(2_000_000_000));
    }

    #[test]
    fn test_aifm_report_strategy_breakdown_count() {
        let input = default_input();
        let out = generate_aifmd_report(&input).unwrap();
        // 2 funds with 2 different strategies
        assert_eq!(out.aifm_report.strategy_breakdown.len(), 2);
    }

    #[test]
    fn test_strategy_breakdown_pct_sums_correctly() {
        let input = default_input();
        let out = generate_aifmd_report(&input).unwrap();
        let total_pct: Decimal = out
            .aifm_report
            .strategy_breakdown
            .iter()
            .map(|s| s.pct_of_aum)
            .sum();
        assert_eq!(total_pct, dec!(100));
    }

    #[test]
    fn test_aifm_leverage_gross_calculation() {
        let input = default_input();
        let out = generate_aifmd_report(&input).unwrap();
        // NAV = 1.2B + 0.8B = 2B; long=3B, short=0.5B; gross = 3.5B / 2B = 1.75
        assert_eq!(out.aifm_report.leverage_gross, dec!(1.75));
    }

    #[test]
    fn test_aifm_leverage_commitment_calculation() {
        let input = default_input();
        let out = generate_aifmd_report(&input).unwrap();
        // Commitment = gross * 0.8 = 3.5B * 0.8 / 2B = 1.4
        assert_eq!(out.aifm_report.leverage_commitment, dec!(1.4));
    }

    #[test]
    fn test_top_markets_limited_to_5() {
        let mut input = default_input();
        for i in 0..10 {
            input.principal_markets.push(MarketExposure {
                market: format!("Market {}", i),
                pct: dec!(2),
            });
        }
        let out = generate_aifmd_report(&input).unwrap();
        assert!(out.aifm_report.top_markets.len() <= 5);
    }

    #[test]
    fn test_top_markets_sorted_by_pct() {
        let input = default_input();
        let out = generate_aifmd_report(&input).unwrap();
        for i in 1..out.aifm_report.top_markets.len() {
            assert!(out.aifm_report.top_markets[i - 1].pct >= out.aifm_report.top_markets[i].pct);
        }
    }

    #[test]
    fn test_liquidity_profile_passthrough() {
        let input = default_input();
        let out = generate_aifmd_report(&input).unwrap();
        assert_eq!(out.aifm_report.liquidity_profile.pct_1d, dec!(10));
        assert_eq!(out.aifm_report.liquidity_profile.pct_over_365d, dec!(5));
    }

    // --- Fund-level report tests ---

    #[test]
    fn test_fund_reports_count_matches() {
        let input = default_input();
        let out = generate_aifmd_report(&input).unwrap();
        assert_eq!(out.fund_reports.len(), 2);
    }

    #[test]
    fn test_fund_report_names() {
        let input = default_input();
        let out = generate_aifmd_report(&input).unwrap();
        assert_eq!(out.fund_reports[0].fund_name, "Fund Alpha");
        assert_eq!(out.fund_reports[1].fund_name, "Fund Beta");
    }

    #[test]
    fn test_fund_report_investor_concentration() {
        let input = default_input();
        let out = generate_aifmd_report(&input).unwrap();
        assert_eq!(out.fund_reports[0].investor_concentration, dec!(25));
        assert_eq!(out.fund_reports[1].investor_concentration, dec!(40));
    }

    #[test]
    fn test_fund_report_redemption_details() {
        let input = default_input();
        let out = generate_aifmd_report(&input).unwrap();
        assert_eq!(out.fund_reports[0].redemption_frequency, "Monthly");
        assert_eq!(out.fund_reports[0].notice_period_days, 30);
        assert!(!out.fund_reports[0].has_gates);
        assert!(out.fund_reports[1].has_gates);
    }

    #[test]
    fn test_fund_report_lockup_details() {
        let input = default_input();
        let out = generate_aifmd_report(&input).unwrap();
        assert!(!out.fund_reports[0].has_lockup);
        assert_eq!(out.fund_reports[0].lockup_months, 0);
        assert!(out.fund_reports[1].has_lockup);
        assert_eq!(out.fund_reports[1].lockup_months, 12);
    }

    #[test]
    fn test_fund_report_side_pocket() {
        let input = default_input();
        let out = generate_aifmd_report(&input).unwrap();
        assert_eq!(out.fund_reports[0].side_pocket_pct, dec!(0));
        assert_eq!(out.fund_reports[1].side_pocket_pct, dec!(5));
    }

    #[test]
    fn test_fund_report_counterparties_limited_to_5() {
        let mut input = default_input();
        for i in 0..10 {
            input.top_counterparties.push(CounterpartyExposure {
                name: format!("CP {}", i),
                exposure_pct: dec!(3),
            });
        }
        let out = generate_aifmd_report(&input).unwrap();
        for fr in &out.fund_reports {
            assert!(fr.top_counterparties.len() <= 5);
        }
    }

    // --- Leverage flag tests ---

    #[test]
    fn test_no_leverage_flags_below_threshold() {
        let input = default_input();
        let out = generate_aifmd_report(&input).unwrap();
        // Default funds have commitment leverage 1.5 and 1.3, both below 3x
        // AIFM-level is 1.4, also below 3x
        assert!(out.leverage_flags.is_empty());
    }

    #[test]
    fn test_leverage_flag_triggered_at_fund_level() {
        let mut input = default_input();
        input.funds[0].leverage_commitment = dec!(3.5);
        let out = generate_aifmd_report(&input).unwrap();
        assert!(!out.leverage_flags.is_empty());
        assert!(out.leverage_flags[0].contains("Fund Alpha"));
        assert!(out.leverage_flags[0].contains("3.5"));
    }

    #[test]
    fn test_leverage_flag_at_exact_3x() {
        let mut input = default_input();
        input.funds[0].leverage_commitment = dec!(3);
        let out = generate_aifmd_report(&input).unwrap();
        // Exactly 3x should not trigger (threshold is >3x)
        let fund_flags: Vec<&String> = out
            .leverage_flags
            .iter()
            .filter(|f| f.contains("Fund Alpha"))
            .collect();
        assert!(fund_flags.is_empty());
    }

    #[test]
    fn test_enhanced_reporting_flag_on_fund() {
        let mut input = default_input();
        input.funds[0].leverage_commitment = dec!(4.0);
        let out = generate_aifmd_report(&input).unwrap();
        assert!(out.fund_reports[0].enhanced_reporting_required);
        assert!(!out.fund_reports[1].enhanced_reporting_required);
    }

    #[test]
    fn test_aifm_level_leverage_flag() {
        let mut input = default_input();
        // Make aggregate leverage exceed 3x: long=10B, short=3B, NAV=2B → commitment = 13*0.8/2 = 5.2
        input.long_exposures = dec!(10_000_000_000);
        input.short_exposures = dec!(3_000_000_000);
        let out = generate_aifmd_report(&input).unwrap();
        let aifm_flags: Vec<&String> = out
            .leverage_flags
            .iter()
            .filter(|f| f.contains("AIFM-level"))
            .collect();
        assert!(!aifm_flags.is_empty());
    }

    // --- Stress test tests ---

    #[test]
    fn test_stress_test_shocks() {
        let input = default_input();
        let out = generate_aifmd_report(&input).unwrap();
        assert_eq!(out.stress_test_summary.equity_shock_pct, dec!(-30));
        assert_eq!(out.stress_test_summary.rates_shock_bps, dec!(250));
        assert_eq!(out.stress_test_summary.fx_shock_pct, dec!(-20));
        assert_eq!(out.stress_test_summary.credit_shock_bps, dec!(400));
    }

    #[test]
    fn test_stress_test_impacts() {
        let input = default_input();
        let out = generate_aifmd_report(&input).unwrap();
        assert_eq!(out.stress_test_summary.equity_impact, dec!(-12));
        assert_eq!(out.stress_test_summary.rates_impact, dec!(-5));
        assert_eq!(out.stress_test_summary.fx_impact, dec!(-3));
        assert_eq!(out.stress_test_summary.credit_impact, dec!(-8));
    }

    #[test]
    fn test_stress_test_worst_case() {
        let input = default_input();
        let out = generate_aifmd_report(&input).unwrap();
        // Worst case is -12 (equity)
        assert_eq!(out.stress_test_summary.worst_case_impact, dec!(-12));
    }

    #[test]
    fn test_stress_test_combined() {
        let input = default_input();
        let out = generate_aifmd_report(&input).unwrap();
        // Combined: -12 + -5 + -3 + -8 = -28
        assert_eq!(out.stress_test_summary.combined_impact, dec!(-28));
    }

    #[test]
    fn test_stress_warning_when_combined_exceeds_50() {
        let mut input = default_input();
        input.stress_equity_impact = dec!(-25);
        input.stress_rates_impact = dec!(-15);
        input.stress_fx_impact = dec!(-10);
        input.stress_credit_impact = dec!(-5);
        // Combined = -55
        let out = generate_aifmd_report(&input).unwrap();
        assert!(out.warnings.iter().any(|w| w.contains("-50%")));
    }

    // --- Data quality and compliance tests ---

    #[test]
    fn test_liquidity_profile_not_summing_to_100() {
        let mut input = default_input();
        input.liquidity_profile.pct_over_365d = dec!(20); // sum becomes 115
        let out = generate_aifmd_report(&input).unwrap();
        assert!(out
            .data_quality_issues
            .iter()
            .any(|d| d.contains("Liquidity profile")));
    }

    #[test]
    fn test_counterparty_exposure_exceeds_100() {
        let mut input = default_input();
        input.top_counterparties = vec![
            CounterpartyExposure {
                name: "CP1".to_string(),
                exposure_pct: dec!(60),
            },
            CounterpartyExposure {
                name: "CP2".to_string(),
                exposure_pct: dec!(50),
            },
        ];
        let out = generate_aifmd_report(&input).unwrap();
        assert!(out
            .data_quality_issues
            .iter()
            .any(|d| d.contains("counterparty")));
    }

    #[test]
    fn test_market_exposure_exceeds_100() {
        let mut input = default_input();
        input.principal_markets = vec![
            MarketExposure {
                market: "M1".to_string(),
                pct: dec!(80),
            },
            MarketExposure {
                market: "M2".to_string(),
                pct: dec!(30),
            },
        ];
        let out = generate_aifmd_report(&input).unwrap();
        assert!(out
            .data_quality_issues
            .iter()
            .any(|d| d.contains("market exposure")));
    }

    #[test]
    fn test_compliance_score_perfect_data() {
        let mut input = default_input();
        // Add 5 counterparties and 5 markets for perfect score
        input.top_counterparties = (0..5)
            .map(|i| CounterpartyExposure {
                name: format!("CP {}", i),
                exposure_pct: dec!(10),
            })
            .collect();
        input.principal_markets = (0..5)
            .map(|i| MarketExposure {
                market: format!("Market {}", i),
                pct: dec!(10),
            })
            .collect();
        let out = generate_aifmd_report(&input).unwrap();
        assert_eq!(out.compliance_score, dec!(100));
    }

    #[test]
    fn test_compliance_score_deducts_for_few_counterparties() {
        let mut input = default_input();
        input.top_counterparties = vec![CounterpartyExposure {
            name: "Only One".to_string(),
            exposure_pct: dec!(20),
        }];
        input.principal_markets = (0..5)
            .map(|i| MarketExposure {
                market: format!("Market {}", i),
                pct: dec!(10),
            })
            .collect();
        let out = generate_aifmd_report(&input).unwrap();
        assert!(out.compliance_score < dec!(100));
    }

    #[test]
    fn test_compliance_score_deducts_for_no_funds() {
        let mut input = default_input();
        input.funds = vec![];
        input.top_counterparties = (0..5)
            .map(|i| CounterpartyExposure {
                name: format!("CP {}", i),
                exposure_pct: dec!(10),
            })
            .collect();
        input.principal_markets = (0..5)
            .map(|i| MarketExposure {
                market: format!("Market {}", i),
                pct: dec!(10),
            })
            .collect();
        let out = generate_aifmd_report(&input).unwrap();
        assert!(out.compliance_score <= dec!(80));
    }

    #[test]
    fn test_compliance_score_deducts_for_zero_stress() {
        let mut input = default_input();
        input.stress_equity_impact = dec!(0);
        input.stress_rates_impact = dec!(0);
        input.stress_fx_impact = dec!(0);
        input.stress_credit_impact = dec!(0);
        input.top_counterparties = (0..5)
            .map(|i| CounterpartyExposure {
                name: format!("CP {}", i),
                exposure_pct: dec!(10),
            })
            .collect();
        input.principal_markets = (0..5)
            .map(|i| MarketExposure {
                market: format!("Market {}", i),
                pct: dec!(10),
            })
            .collect();
        let out = generate_aifmd_report(&input).unwrap();
        assert!(out.compliance_score <= dec!(90));
    }

    #[test]
    fn test_compliance_score_floor_at_zero() {
        let mut input = default_input();
        // Create many data quality issues
        input.funds = vec![];
        input.top_counterparties = vec![];
        input.principal_markets = vec![];
        input.stress_equity_impact = dec!(0);
        input.stress_rates_impact = dec!(0);
        input.stress_fx_impact = dec!(0);
        input.stress_credit_impact = dec!(0);
        input.liquidity_profile.pct_over_365d = dec!(80); // bad sum
        input.liquidity_profile.pct_1d = dec!(-5); // negative bucket
        let out = generate_aifmd_report(&input).unwrap();
        assert!(out.compliance_score >= dec!(0));
    }

    // --- Filing deadline tests ---

    #[test]
    fn test_filing_deadline_quarterly() {
        let input = default_input(); // AUM > 1B → quarterly
        let out = generate_aifmd_report(&input).unwrap();
        // 2024-12-31 + 30 days = 2025-01-30
        assert_eq!(out.filing_deadline, "2025-01-30");
    }

    #[test]
    fn test_filing_deadline_semiannual() {
        let mut input = default_input();
        input.total_aum = dec!(750_000_000);
        let out = generate_aifmd_report(&input).unwrap();
        // 2024-12-31 + 60 days = 2025-03-01 (2025 is not a leap year)
        assert_eq!(out.filing_deadline, "2025-03-01");
    }

    #[test]
    fn test_filing_deadline_annual() {
        let mut input = default_input();
        input.total_aum = dec!(50_000_000);
        let out = generate_aifmd_report(&input).unwrap();
        // 2024-12-31 + 120 days = 2025-04-30
        assert_eq!(out.filing_deadline, "2025-04-30");
    }

    // --- Warning and edge case tests ---

    #[test]
    fn test_nav_aum_mismatch_warning() {
        let mut input = default_input();
        // Make fund NAVs sum to 500M while AUM is 2B — large mismatch
        input.funds[0].nav = dec!(300_000_000);
        input.funds[1].nav = dec!(200_000_000);
        let out = generate_aifmd_report(&input).unwrap();
        assert!(out
            .warnings
            .iter()
            .any(|w| w.contains("differs from total_aum")));
    }

    #[test]
    fn test_no_warning_when_nav_matches_aum() {
        let input = default_input();
        let out = generate_aifmd_report(&input).unwrap();
        // NAV sum = 2B, AUM = 2B → no mismatch warning
        assert!(!out
            .warnings
            .iter()
            .any(|w| w.contains("differs from total_aum")));
    }

    #[test]
    fn test_zero_aum_produces_report() {
        let mut input = default_input();
        input.total_aum = dec!(0);
        input.funds = vec![];
        input.long_exposures = dec!(0);
        input.short_exposures = dec!(0);
        let out = generate_aifmd_report(&input).unwrap();
        assert_eq!(out.reporting_frequency, ReportingFrequency::Annual);
    }

    #[test]
    fn test_single_fund() {
        let mut input = default_input();
        input.funds.truncate(1);
        let out = generate_aifmd_report(&input).unwrap();
        assert_eq!(out.fund_reports.len(), 1);
        assert_eq!(out.aifm_report.strategy_breakdown.len(), 1);
    }

    #[test]
    fn test_many_funds_same_strategy() {
        let mut input = default_input();
        input.funds = (0..5)
            .map(|i| FundInfo {
                name: format!("Fund {}", i),
                nav: dec!(200_000_000),
                strategy: AifmdStrategy::Equity,
                domicile: "Ireland".to_string(),
                leverage_gross: dec!(1.5),
                leverage_commitment: dec!(1.2),
                investor_count: 10,
                largest_investor_pct: dec!(30),
                redemption_frequency: "Monthly".to_string(),
                notice_period_days: 30,
                has_gates: false,
                has_lockup: false,
                lockup_months: 0,
                side_pocket_pct: dec!(0),
            })
            .collect();
        let out = generate_aifmd_report(&input).unwrap();
        assert_eq!(out.fund_reports.len(), 5);
        assert_eq!(out.aifm_report.strategy_breakdown.len(), 1);
        assert_eq!(out.aifm_report.strategy_breakdown[0].fund_count, 5);
    }

    #[test]
    fn test_methodology_is_populated() {
        let input = default_input();
        let out = generate_aifmd_report(&input).unwrap();
        assert!(out.methodology.contains("AIFMD"));
        assert!(out.methodology.contains("Annex IV"));
    }

    #[test]
    fn test_assumptions_are_populated() {
        let input = default_input();
        let out = generate_aifmd_report(&input).unwrap();
        assert!(!out.assumptions.is_empty());
    }

    #[test]
    fn test_data_quality_negative_liquidity_bucket() {
        let mut input = default_input();
        input.liquidity_profile.pct_1d = dec!(-5);
        // Adjust another to keep sum near 100
        input.liquidity_profile.pct_8_30d = dec!(30);
        let out = generate_aifmd_report(&input).unwrap();
        assert!(out
            .data_quality_issues
            .iter()
            .any(|d| d.contains("negative")));
    }

    #[test]
    fn test_data_quality_investor_pct_out_of_range() {
        let mut input = default_input();
        input.funds[0].largest_investor_pct = dec!(110);
        let out = generate_aifmd_report(&input).unwrap();
        assert!(out
            .data_quality_issues
            .iter()
            .any(|d| d.contains("largest_investor_pct")));
    }

    #[test]
    fn test_data_quality_side_pocket_out_of_range() {
        let mut input = default_input();
        input.funds[0].side_pocket_pct = dec!(-1);
        let out = generate_aifmd_report(&input).unwrap();
        assert!(out
            .data_quality_issues
            .iter()
            .any(|d| d.contains("side_pocket_pct")));
    }

    #[test]
    fn test_data_quality_negative_gross_leverage() {
        let mut input = default_input();
        input.funds[0].leverage_gross = dec!(-0.5);
        let out = generate_aifmd_report(&input).unwrap();
        assert!(out
            .data_quality_issues
            .iter()
            .any(|d| d.contains("negative gross leverage")));
    }

    // --- Serialization round-trip test ---

    #[test]
    fn test_output_serializes_to_json() {
        let input = default_input();
        let out = generate_aifmd_report(&input).unwrap();
        let json = serde_json::to_string(&out).unwrap();
        let deserialized: AifmdReportingOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.reporting_frequency, out.reporting_frequency);
        assert_eq!(deserialized.compliance_score, out.compliance_score);
    }

    #[test]
    fn test_input_deserializes_from_json() {
        let input = default_input();
        let json = serde_json::to_string(&input).unwrap();
        let deserialized: AifmdReportingInput = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.aifm_name, input.aifm_name);
        assert_eq!(deserialized.total_aum, input.total_aum);
    }

    // --- Leverage helper unit tests ---

    #[test]
    fn test_gross_leverage_with_zero_nav() {
        let result = compute_gross_leverage(dec!(100), dec!(50), dec!(0));
        assert_eq!(result, dec!(0));
    }

    #[test]
    fn test_gross_leverage_calculation() {
        // long=300, short=100, nav=200 → (300+100)/200 = 2.0
        let result = compute_gross_leverage(dec!(300), dec!(100), dec!(200));
        assert_eq!(result, dec!(2));
    }

    #[test]
    fn test_commitment_leverage_with_zero_nav() {
        let result = compute_commitment_leverage(dec!(100), dec!(50), dec!(0));
        assert_eq!(result, dec!(0));
    }

    #[test]
    fn test_commitment_leverage_calculation() {
        // long=300, short=100, nav=200 → (400*0.8)/200 = 1.6
        let result = compute_commitment_leverage(dec!(300), dec!(100), dec!(200));
        assert_eq!(result, dec!(1.6));
    }

    // --- Reporting frequency helper unit tests ---

    #[test]
    fn test_determine_reporting_frequency_zero_aum() {
        assert_eq!(
            determine_reporting_frequency(dec!(0)),
            ReportingFrequency::Annual
        );
    }

    #[test]
    fn test_determine_reporting_frequency_large_aum() {
        assert_eq!(
            determine_reporting_frequency(dec!(5_000_000_000)),
            ReportingFrequency::Quarterly
        );
    }

    // --- Empty fund name data quality ---

    #[test]
    fn test_empty_fund_name_flagged() {
        let mut input = default_input();
        input.funds[0].name = "".to_string();
        let out = generate_aifmd_report(&input).unwrap();
        assert!(out
            .data_quality_issues
            .iter()
            .any(|d| d.contains("empty name")));
    }
}
