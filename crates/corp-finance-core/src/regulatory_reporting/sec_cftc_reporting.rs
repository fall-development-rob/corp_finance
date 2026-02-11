use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// SEC Form PF filing type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FormPfType {
    Large,
    Small,
    Exempt,
}

/// CFTC CPO-PQR filing type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CpoPqrType {
    Large,
    Small,
    Exempt,
}

/// Filing frequency.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilingFrequency {
    Quarterly,
    Annual,
    Exempt,
}

/// Strategy classification for Form PF.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FormPfStrategy {
    EquityLongShort,
    EventDriven,
    Macro,
    RelativeValue,
    Credit,
    MultiStrategy,
    ManagedFutures,
    Other,
}

// ---------------------------------------------------------------------------
// Input types
// ---------------------------------------------------------------------------

/// Per-fund data for Form PF reporting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormPfFund {
    pub name: String,
    pub nav: Decimal,
    pub gross_assets: Decimal,
    pub strategy: FormPfStrategy,
    pub is_hedge_fund: bool,
    pub is_pe_fund: bool,
    pub is_liquidity_fund: bool,
    pub total_borrowings: Decimal,
    pub secured_borrowings: Decimal,
    pub management_fee_rate: Decimal,
    pub incentive_fee_rate: Decimal,
    pub high_water_mark: bool,
    pub monthly_returns: Vec<Decimal>,
    pub us_investor_pct: Decimal,
    pub institutional_pct: Decimal,
}

/// Counterparty information for Form PF / CPO-PQR.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CounterpartyInfo {
    pub name: String,
    pub exposure: Decimal,
    pub secured_pct: Decimal,
}

/// Input for SEC Form PF and CFTC CPO-PQR report generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecCftcReportingInput {
    pub adviser_name: String,
    pub sec_registered: bool,
    pub nfa_registered: bool,
    pub total_regulatory_aum: Decimal,
    pub fund_count: u32,
    pub funds: Vec<FormPfFund>,
    pub fiscal_year_end: String,
    pub reporting_date: String,
    pub counterparties: Vec<CounterpartyInfo>,
    pub otc_bilateral_pct: Decimal,
    pub otc_cleared_pct: Decimal,
    pub exchange_traded_pct: Decimal,
    pub commodity_pool: bool,
}

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// AUM threshold analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdAnalysis {
    pub is_large_adviser: bool,
    pub is_large_pe: bool,
    pub is_large_liquidity: bool,
    pub hedge_fund_aum: Decimal,
    pub pe_fund_aum: Decimal,
    pub liquidity_fund_aum: Decimal,
}

/// Borrowing analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BorrowingAnalysis {
    pub total_ratio: Decimal,
    pub secured_ratio: Decimal,
    pub unsecured_ratio: Decimal,
    pub total_borrowings: Decimal,
    pub total_nav: Decimal,
}

/// Per-fund performance summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundPerformance {
    pub fund_name: String,
    pub annualized_return: Decimal,
    pub max_drawdown: Decimal,
    pub sharpe_ratio: Decimal,
    pub monthly_count: u32,
    pub borrowing_ratio: Decimal,
}

/// Performance summary for all funds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSummary {
    pub fund_performances: Vec<FundPerformance>,
    pub aggregate_nav: Decimal,
    pub aggregate_gross_assets: Decimal,
}

/// Complete SEC Form PF and CFTC CPO-PQR report output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecCftcReportingOutput {
    pub form_pf_required: bool,
    pub form_pf_filing_type: FormPfType,
    pub form_pf_sections_required: Vec<String>,
    pub filing_frequency: FilingFrequency,
    pub filing_deadline: String,
    pub cpo_pqr_required: bool,
    pub cpo_pqr_type: CpoPqrType,
    pub regulatory_aum_calculation: Decimal,
    pub aum_threshold_analysis: ThresholdAnalysis,
    pub borrowing_analysis: BorrowingAnalysis,
    pub counterparty_concentration: Decimal,
    pub performance_summary: PerformanceSummary,
    pub data_completeness_score: Decimal,
    pub methodology: String,
    pub assumptions: Vec<String>,
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/// Calculate regulatory AUM (gross asset value = sum of absolute long+short).
fn calculate_regulatory_aum(funds: &[FormPfFund]) -> Decimal {
    funds.iter().map(|f| f.gross_assets).sum()
}

/// Determine Form PF filing type based on AUM thresholds.
fn determine_form_pf_type(
    sec_registered: bool,
    total_regulatory_aum: Decimal,
    hedge_fund_aum: Decimal,
    pe_fund_aum: Decimal,
    liquidity_fund_aum: Decimal,
) -> FormPfType {
    if !sec_registered {
        return FormPfType::Exempt;
    }

    let threshold_150m = dec!(150_000_000);
    let threshold_1_5b = dec!(1_500_000_000);

    if total_regulatory_aum < threshold_150m {
        return FormPfType::Exempt;
    }

    // Large if: HF AUM >= $1.5B, or PE AUM >= $2B, or liquidity AUM >= $1B
    if hedge_fund_aum >= threshold_1_5b
        || pe_fund_aum >= dec!(2_000_000_000)
        || liquidity_fund_aum >= dec!(1_000_000_000)
    {
        FormPfType::Large
    } else {
        FormPfType::Small
    }
}

/// Determine required Form PF sections.
fn determine_form_pf_sections(
    filing_type: &FormPfType,
    has_hedge_funds: bool,
    has_pe_funds: bool,
    has_liquidity_funds: bool,
    hedge_fund_aum: Decimal,
    pe_fund_aum: Decimal,
    liquidity_fund_aum: Decimal,
) -> Vec<String> {
    let mut sections = Vec::new();

    if *filing_type == FormPfType::Exempt {
        return sections;
    }

    // Section 1: all filers
    sections.push("Section 1 — Basic Information".to_string());

    // Section 2: large hedge fund advisers ($1.5B+ HF AUM)
    if has_hedge_funds && hedge_fund_aum >= dec!(1_500_000_000) {
        sections.push("Section 2 — Large Hedge Fund Adviser".to_string());
    }

    // Section 3: large liquidity fund advisers ($1B+ liquidity AUM)
    if has_liquidity_funds && liquidity_fund_aum >= dec!(1_000_000_000) {
        sections.push("Section 3 — Large Liquidity Fund Adviser".to_string());
    }

    // Section 4: large PE advisers ($2B+ PE AUM)
    if has_pe_funds && pe_fund_aum >= dec!(2_000_000_000) {
        sections.push("Section 4 — Large Private Equity Adviser".to_string());
    }

    sections
}

/// Determine filing frequency.
fn determine_filing_frequency(filing_type: &FormPfType) -> FilingFrequency {
    match filing_type {
        FormPfType::Large => FilingFrequency::Quarterly,
        FormPfType::Small => FilingFrequency::Annual,
        FormPfType::Exempt => FilingFrequency::Exempt,
    }
}

/// Compute filing deadline from reporting date.
fn compute_filing_deadline(reporting_date: &str, filing_type: &FormPfType) -> String {
    let days = match filing_type {
        FormPfType::Large => 60,
        FormPfType::Small => 120,
        FormPfType::Exempt => 0,
    };

    if *filing_type == FormPfType::Exempt {
        return "N/A".to_string();
    }

    if let Ok(date) = chrono::NaiveDate::parse_from_str(reporting_date, "%Y-%m-%d") {
        let deadline = date + chrono::Duration::days(days);
        deadline.format("%Y-%m-%d").to_string()
    } else {
        format!("{} + {} days", reporting_date, days)
    }
}

/// Determine CFTC CPO-PQR filing type.
fn determine_cpo_pqr_type(
    nfa_registered: bool,
    commodity_pool: bool,
    total_regulatory_aum: Decimal,
    funds: &[FormPfFund],
) -> CpoPqrType {
    if !nfa_registered || !commodity_pool {
        return CpoPqrType::Exempt;
    }

    let threshold_1_5b = dec!(1_500_000_000);
    let threshold_500m = dec!(500_000_000);

    // Large CPO: AUM > $1.5B or any pool > $500M
    if total_regulatory_aum > threshold_1_5b {
        return CpoPqrType::Large;
    }

    let has_large_pool = funds.iter().any(|f| f.gross_assets > threshold_500m);
    if has_large_pool {
        CpoPqrType::Large
    } else {
        CpoPqrType::Small
    }
}

/// Calculate Herfindahl-Hirschman Index for counterparty concentration.
/// HHI = sum of squared market shares (each share as a percentage 0-100).
fn calculate_counterparty_hhi(counterparties: &[CounterpartyInfo]) -> Decimal {
    let total_exposure: Decimal = counterparties.iter().map(|c| c.exposure).sum();
    if total_exposure == dec!(0) {
        return dec!(0);
    }

    counterparties
        .iter()
        .map(|c| {
            let share = c.exposure * dec!(100) / total_exposure;
            share * share
        })
        .sum()
}

/// Calculate annualized return from monthly returns.
fn annualized_return(monthly_returns: &[Decimal]) -> Decimal {
    if monthly_returns.is_empty() {
        return dec!(0);
    }

    // Compound monthly returns: product of (1 + r_i)
    let mut cumulative = dec!(1);
    for r in monthly_returns {
        cumulative *= dec!(1) + *r;
    }

    let n = monthly_returns.len() as u32;
    if n == 0 || cumulative <= dec!(0) {
        return dec!(0);
    }

    // Annualized = (cumulative)^(12/n) - 1
    // Use ln/exp approximation with Taylor series
    let periods = Decimal::from(n);
    let annualization_factor = dec!(12) / periods;

    // ln(cumulative) via Taylor series around 1: ln(1+x) = x - x^2/2 + x^3/3 - ...
    let x = cumulative - dec!(1);
    let ln_cum = if x.abs() < dec!(1) {
        let mut sum = dec!(0);
        let mut term = x;
        for i in 1u32..=30 {
            let sign = if i % 2 == 0 { dec!(-1) } else { dec!(1) };
            sum += sign * term / Decimal::from(i);
            term *= x;
        }
        sum
    } else {
        // For larger values, use iterative Newton's method for ln
        // ln(a) where a > 2: reduce to ln(a/e^k) + k where e^k ~ a
        newton_ln(cumulative)
    };

    // exp(ln_cum * annualization_factor)
    let exponent = ln_cum * annualization_factor;
    taylor_exp(exponent) - dec!(1)
}

/// Taylor series exp(x) with 30 terms.
fn taylor_exp(x: Decimal) -> Decimal {
    let mut sum = dec!(1);
    let mut term = dec!(1);
    for i in 1u32..=30 {
        term *= x / Decimal::from(i);
        sum += term;
        if term.abs() < dec!(0.0000000001) {
            break;
        }
    }
    sum
}

/// Newton's method for ln(x), x > 0.
fn newton_ln(x: Decimal) -> Decimal {
    if x <= dec!(0) {
        return dec!(0);
    }
    // Initial guess
    let mut y = dec!(0);
    // ln(x): solve exp(y) = x via Newton's method
    // y_{n+1} = y_n - (exp(y_n) - x) / exp(y_n) = y_n - 1 + x/exp(y_n)
    for _ in 0..40 {
        let ey = taylor_exp(y);
        if ey == dec!(0) {
            break;
        }
        let delta = dec!(1) - x / ey;
        y -= delta;
        if delta.abs() < dec!(0.0000000001) {
            break;
        }
    }
    y
}

/// Calculate maximum drawdown from monthly returns.
fn max_drawdown(monthly_returns: &[Decimal]) -> Decimal {
    if monthly_returns.is_empty() {
        return dec!(0);
    }

    let mut peak = dec!(1);
    let mut cumulative = dec!(1);
    let mut max_dd = dec!(0);

    for r in monthly_returns {
        cumulative *= dec!(1) + *r;
        if cumulative > peak {
            peak = cumulative;
        }
        if peak > dec!(0) {
            let dd = (peak - cumulative) / peak;
            if dd > max_dd {
                max_dd = dd;
            }
        }
    }

    max_dd
}

/// Calculate Sharpe ratio from monthly returns.
/// Assumes risk-free rate of 0 for simplicity (excess returns = returns).
fn sharpe_ratio(monthly_returns: &[Decimal]) -> Decimal {
    if monthly_returns.len() < 2 {
        return dec!(0);
    }

    let n = Decimal::from(monthly_returns.len() as u32);
    let mean: Decimal = monthly_returns.iter().copied().sum::<Decimal>() / n;

    // Standard deviation
    let variance: Decimal = monthly_returns
        .iter()
        .map(|r| (*r - mean) * (*r - mean))
        .sum::<Decimal>()
        / (n - dec!(1));

    let std_dev = newton_sqrt(variance);

    if std_dev == dec!(0) {
        return dec!(0);
    }

    // Annualize: multiply mean by 12, std_dev by sqrt(12)
    let sqrt_12 = newton_sqrt(dec!(12));
    let annualized_mean = mean * dec!(12);
    let annualized_std = std_dev * sqrt_12;

    if annualized_std == dec!(0) {
        return dec!(0);
    }

    annualized_mean / annualized_std
}

/// Newton's method for square root.
fn newton_sqrt(x: Decimal) -> Decimal {
    if x <= dec!(0) {
        return dec!(0);
    }
    let mut guess = x / dec!(2);
    if guess == dec!(0) {
        guess = dec!(1);
    }
    for _ in 0..30 {
        let next = (guess + x / guess) / dec!(2);
        if (next - guess).abs() < dec!(0.0000000001) {
            return next;
        }
        guess = next;
    }
    guess
}

/// Calculate data completeness score.
fn calculate_completeness_score(input: &SecCftcReportingInput, warnings: &[String]) -> Decimal {
    let mut score = dec!(100);

    // Deduct for missing adviser name
    if input.adviser_name.is_empty() {
        score -= dec!(10);
    }

    // Deduct for missing funds
    if input.funds.is_empty() {
        score -= dec!(20);
    }

    // Deduct for funds without monthly returns
    let funds_without_returns = input
        .funds
        .iter()
        .filter(|f| f.monthly_returns.is_empty())
        .count() as u32;
    score -= Decimal::from(funds_without_returns) * dec!(5);

    // Deduct for missing counterparties
    if input.counterparties.is_empty() {
        score -= dec!(10);
    }

    // Deduct for each warning
    score -= Decimal::from(warnings.len() as u32) * dec!(2);

    // Deduct if trading venue breakdown doesn't sum near 100
    let venue_sum = input.otc_bilateral_pct + input.otc_cleared_pct + input.exchange_traded_pct;
    let hundred = dec!(100);
    if (venue_sum - hundred).abs() > dec!(5) && venue_sum > dec!(0) {
        score -= dec!(5);
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

/// Generate SEC Form PF and CFTC CPO-PQR regulatory reports.
///
/// This function computes:
/// - Form PF filing requirements and type (Large/Small/Exempt)
/// - Required Form PF sections based on fund types and AUM
/// - Filing frequency and deadlines
/// - CFTC CPO-PQR filing requirements
/// - Regulatory AUM calculation
/// - Borrowing analysis
/// - Counterparty concentration (HHI)
/// - Performance summary per fund (annualized return, drawdown, Sharpe)
/// - Data completeness scoring
pub fn generate_sec_cftc_report(
    input: &SecCftcReportingInput,
) -> CorpFinanceResult<SecCftcReportingOutput> {
    let mut warnings: Vec<String> = Vec::new();

    // --- Input validation ---
    if input.adviser_name.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "adviser_name".to_string(),
            reason: "Adviser name must not be empty".to_string(),
        });
    }

    if input.total_regulatory_aum < dec!(0) {
        return Err(CorpFinanceError::InvalidInput {
            field: "total_regulatory_aum".to_string(),
            reason: "Regulatory AUM cannot be negative".to_string(),
        });
    }

    if input.reporting_date.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "reporting_date".to_string(),
            reason: "Reporting date is required".to_string(),
        });
    }

    if chrono::NaiveDate::parse_from_str(&input.reporting_date, "%Y-%m-%d").is_err() {
        return Err(CorpFinanceError::DateError(format!(
            "Invalid reporting_date '{}', expected YYYY-MM-DD",
            input.reporting_date
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
        if fund.gross_assets < dec!(0) {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("funds[{}].gross_assets", i),
                reason: "Fund gross assets cannot be negative".to_string(),
            });
        }
        if fund.total_borrowings < dec!(0) {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("funds[{}].total_borrowings", i),
                reason: "Total borrowings cannot be negative".to_string(),
            });
        }
        if fund.secured_borrowings > fund.total_borrowings {
            warnings.push(format!(
                "Fund '{}': secured borrowings ({}) exceed total borrowings ({})",
                fund.name, fund.secured_borrowings, fund.total_borrowings
            ));
        }
        if fund.management_fee_rate < dec!(0) || fund.management_fee_rate > dec!(1) {
            warnings.push(format!(
                "Fund '{}': management_fee_rate ({}) outside typical 0-100% range",
                fund.name, fund.management_fee_rate
            ));
        }
        if fund.incentive_fee_rate < dec!(0) || fund.incentive_fee_rate > dec!(1) {
            warnings.push(format!(
                "Fund '{}': incentive_fee_rate ({}) outside typical 0-100% range",
                fund.name, fund.incentive_fee_rate
            ));
        }
        if fund.us_investor_pct < dec!(0) || fund.us_investor_pct > dec!(100) {
            warnings.push(format!(
                "Fund '{}': us_investor_pct ({}) outside 0-100 range",
                fund.name, fund.us_investor_pct
            ));
        }
        if fund.institutional_pct < dec!(0) || fund.institutional_pct > dec!(100) {
            warnings.push(format!(
                "Fund '{}': institutional_pct ({}) outside 0-100 range",
                fund.name, fund.institutional_pct
            ));
        }
        if fund.name.is_empty() {
            warnings.push(format!("funds[{}] has an empty name", i));
        }
    }

    // Validate counterparty data
    for (i, cp) in input.counterparties.iter().enumerate() {
        if cp.exposure < dec!(0) {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("counterparties[{}].exposure", i),
                reason: "Counterparty exposure cannot be negative".to_string(),
            });
        }
        if cp.secured_pct < dec!(0) || cp.secured_pct > dec!(100) {
            warnings.push(format!(
                "Counterparty '{}': secured_pct ({}) outside 0-100 range",
                cp.name, cp.secured_pct
            ));
        }
    }

    // Validate trading venue breakdown
    let venue_sum = input.otc_bilateral_pct + input.otc_cleared_pct + input.exchange_traded_pct;
    if venue_sum > dec!(0) && (venue_sum - dec!(100)).abs() > dec!(5) {
        warnings.push(format!(
            "Trading venue breakdown sums to {}%, expected ~100%",
            venue_sum
        ));
    }

    // Validate fund_count matches actual funds
    if input.fund_count != input.funds.len() as u32 {
        warnings.push(format!(
            "fund_count ({}) does not match number of funds provided ({})",
            input.fund_count,
            input.funds.len()
        ));
    }

    // --- Regulatory AUM calculation ---
    let calculated_regulatory_aum = calculate_regulatory_aum(&input.funds);
    if calculated_regulatory_aum > dec!(0)
        && (calculated_regulatory_aum - input.total_regulatory_aum).abs()
            > input.total_regulatory_aum * dec!(0.1)
    {
        warnings.push(format!(
            "Calculated regulatory AUM ({}) differs from reported ({}) by >10%",
            calculated_regulatory_aum, input.total_regulatory_aum
        ));
    }

    // --- AUM by fund type ---
    let hedge_fund_aum: Decimal = input
        .funds
        .iter()
        .filter(|f| f.is_hedge_fund)
        .map(|f| f.gross_assets)
        .sum();
    let pe_fund_aum: Decimal = input
        .funds
        .iter()
        .filter(|f| f.is_pe_fund)
        .map(|f| f.gross_assets)
        .sum();
    let liquidity_fund_aum: Decimal = input
        .funds
        .iter()
        .filter(|f| f.is_liquidity_fund)
        .map(|f| f.gross_assets)
        .sum();

    // --- Determine filing requirements ---
    let form_pf_filing_type = determine_form_pf_type(
        input.sec_registered,
        input.total_regulatory_aum,
        hedge_fund_aum,
        pe_fund_aum,
        liquidity_fund_aum,
    );

    let form_pf_required = form_pf_filing_type != FormPfType::Exempt;

    let has_hedge_funds = input.funds.iter().any(|f| f.is_hedge_fund);
    let has_pe_funds = input.funds.iter().any(|f| f.is_pe_fund);
    let has_liquidity_funds = input.funds.iter().any(|f| f.is_liquidity_fund);

    let form_pf_sections_required = determine_form_pf_sections(
        &form_pf_filing_type,
        has_hedge_funds,
        has_pe_funds,
        has_liquidity_funds,
        hedge_fund_aum,
        pe_fund_aum,
        liquidity_fund_aum,
    );

    let filing_frequency = determine_filing_frequency(&form_pf_filing_type);
    let filing_deadline = compute_filing_deadline(&input.reporting_date, &form_pf_filing_type);

    // --- CFTC CPO-PQR ---
    let cpo_pqr_type = determine_cpo_pqr_type(
        input.nfa_registered,
        input.commodity_pool,
        input.total_regulatory_aum,
        &input.funds,
    );
    let cpo_pqr_required = cpo_pqr_type != CpoPqrType::Exempt;

    // --- Threshold analysis ---
    let threshold_1_5b = dec!(1_500_000_000);
    let threshold_2b = dec!(2_000_000_000);
    let threshold_1b = dec!(1_000_000_000);

    let aum_threshold_analysis = ThresholdAnalysis {
        is_large_adviser: hedge_fund_aum >= threshold_1_5b,
        is_large_pe: pe_fund_aum >= threshold_2b,
        is_large_liquidity: liquidity_fund_aum >= threshold_1b,
        hedge_fund_aum,
        pe_fund_aum,
        liquidity_fund_aum,
    };

    // --- Borrowing analysis ---
    let total_nav: Decimal = input.funds.iter().map(|f| f.nav).sum();
    let total_borrowings: Decimal = input.funds.iter().map(|f| f.total_borrowings).sum();
    let total_secured: Decimal = input.funds.iter().map(|f| f.secured_borrowings).sum();
    let total_unsecured = total_borrowings - total_secured;

    let borrowing_analysis = BorrowingAnalysis {
        total_ratio: if total_nav > dec!(0) {
            total_borrowings / total_nav
        } else {
            dec!(0)
        },
        secured_ratio: if total_nav > dec!(0) {
            total_secured / total_nav
        } else {
            dec!(0)
        },
        unsecured_ratio: if total_nav > dec!(0) {
            total_unsecured / total_nav
        } else {
            dec!(0)
        },
        total_borrowings,
        total_nav,
    };

    // Warn on high leverage
    if borrowing_analysis.total_ratio > dec!(2) {
        warnings.push(format!(
            "Total borrowing ratio ({}) exceeds 2x NAV — high leverage",
            borrowing_analysis.total_ratio
        ));
    }

    // --- Counterparty concentration ---
    let counterparty_concentration = calculate_counterparty_hhi(&input.counterparties);

    // Warn on high concentration (HHI > 2500 is considered highly concentrated)
    if counterparty_concentration > dec!(2500) {
        warnings.push(format!(
            "Counterparty HHI ({}) indicates high concentration (>2500)",
            counterparty_concentration
        ));
    }

    // --- Performance summary ---
    let aggregate_nav: Decimal = input.funds.iter().map(|f| f.nav).sum();
    let aggregate_gross_assets: Decimal = input.funds.iter().map(|f| f.gross_assets).sum();

    let fund_performances: Vec<FundPerformance> = input
        .funds
        .iter()
        .map(|f| {
            let ann_ret = annualized_return(&f.monthly_returns);
            let dd = max_drawdown(&f.monthly_returns);
            let sr = sharpe_ratio(&f.monthly_returns);
            let borrow_ratio = if f.nav > dec!(0) {
                f.total_borrowings / f.nav
            } else {
                dec!(0)
            };
            FundPerformance {
                fund_name: f.name.clone(),
                annualized_return: ann_ret,
                max_drawdown: dd,
                sharpe_ratio: sr,
                monthly_count: f.monthly_returns.len() as u32,
                borrowing_ratio: borrow_ratio,
            }
        })
        .collect();

    let performance_summary = PerformanceSummary {
        fund_performances,
        aggregate_nav,
        aggregate_gross_assets,
    };

    // --- Data completeness score ---
    let data_completeness_score = calculate_completeness_score(input, &warnings);

    // --- Methodology ---
    let methodology = "SEC Form PF filing requirements per SEC Rule 204(b)-1 under \
        the Investment Advisers Act of 1940. CFTC CPO-PQR per NFA Compliance Rule 2-46. \
        Regulatory AUM calculated as gross asset value (sum of absolute long and short positions). \
        Counterparty concentration measured by Herfindahl-Hirschman Index. \
        Performance metrics: annualized return via geometric compounding, \
        max drawdown via peak-to-trough, Sharpe ratio annualized with risk-free rate of 0%."
        .to_string();

    let assumptions = vec![
        "Currency: all values assumed in USD".to_string(),
        "Risk-free rate: 0% for Sharpe ratio calculation".to_string(),
        "Regulatory AUM uses gross asset value (not net)".to_string(),
        "Filing deadlines: Large=60 days, Small=120 days from reporting date".to_string(),
        "Large hedge fund adviser threshold: $1.5B HF AUM".to_string(),
        "Large PE adviser threshold: $2.0B PE AUM".to_string(),
        "Large liquidity fund adviser threshold: $1.0B liquidity AUM".to_string(),
        "Large CPO threshold: $1.5B total AUM or any pool >$500M".to_string(),
    ];

    Ok(SecCftcReportingOutput {
        form_pf_required,
        form_pf_filing_type,
        form_pf_sections_required,
        filing_frequency,
        filing_deadline,
        cpo_pqr_required,
        cpo_pqr_type,
        regulatory_aum_calculation: calculated_regulatory_aum,
        aum_threshold_analysis,
        borrowing_analysis,
        counterparty_concentration,
        performance_summary,
        data_completeness_score,
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

    // Helper: create default valid input
    fn default_input() -> SecCftcReportingInput {
        SecCftcReportingInput {
            adviser_name: "Test Advisers LLC".to_string(),
            sec_registered: true,
            nfa_registered: true,
            total_regulatory_aum: dec!(2_000_000_000),
            fund_count: 2,
            funds: vec![
                FormPfFund {
                    name: "Alpha HF".to_string(),
                    nav: dec!(800_000_000),
                    gross_assets: dec!(1_200_000_000),
                    strategy: FormPfStrategy::EquityLongShort,
                    is_hedge_fund: true,
                    is_pe_fund: false,
                    is_liquidity_fund: false,
                    total_borrowings: dec!(200_000_000),
                    secured_borrowings: dec!(150_000_000),
                    management_fee_rate: dec!(0.02),
                    incentive_fee_rate: dec!(0.20),
                    high_water_mark: true,
                    monthly_returns: vec![
                        dec!(0.02),
                        dec!(-0.01),
                        dec!(0.03),
                        dec!(0.01),
                        dec!(-0.02),
                        dec!(0.04),
                        dec!(0.01),
                        dec!(-0.03),
                        dec!(0.02),
                        dec!(0.01),
                        dec!(0.03),
                        dec!(-0.01),
                    ],
                    us_investor_pct: dec!(70),
                    institutional_pct: dec!(85),
                },
                FormPfFund {
                    name: "Beta PE".to_string(),
                    nav: dec!(600_000_000),
                    gross_assets: dec!(800_000_000),
                    strategy: FormPfStrategy::Other,
                    is_hedge_fund: false,
                    is_pe_fund: true,
                    is_liquidity_fund: false,
                    total_borrowings: dec!(300_000_000),
                    secured_borrowings: dec!(250_000_000),
                    management_fee_rate: dec!(0.02),
                    incentive_fee_rate: dec!(0.20),
                    high_water_mark: false,
                    monthly_returns: vec![
                        dec!(0.01),
                        dec!(0.01),
                        dec!(0.02),
                        dec!(0.01),
                        dec!(0.00),
                        dec!(0.01),
                    ],
                    us_investor_pct: dec!(60),
                    institutional_pct: dec!(90),
                },
            ],
            fiscal_year_end: "12-31".to_string(),
            reporting_date: "2024-12-31".to_string(),
            counterparties: vec![
                CounterpartyInfo {
                    name: "Goldman Sachs".to_string(),
                    exposure: dec!(400_000_000),
                    secured_pct: dec!(80),
                },
                CounterpartyInfo {
                    name: "JP Morgan".to_string(),
                    exposure: dec!(300_000_000),
                    secured_pct: dec!(75),
                },
                CounterpartyInfo {
                    name: "Morgan Stanley".to_string(),
                    exposure: dec!(200_000_000),
                    secured_pct: dec!(70),
                },
            ],
            otc_bilateral_pct: dec!(30),
            otc_cleared_pct: dec!(40),
            exchange_traded_pct: dec!(30),
            commodity_pool: true,
        }
    }

    // --- Form PF type determination tests ---

    #[test]
    fn test_form_pf_large_hedge_fund() {
        let mut input = default_input();
        input.funds[0].gross_assets = dec!(1_600_000_000);
        input.total_regulatory_aum = dec!(2_400_000_000);
        let out = generate_sec_cftc_report(&input).unwrap();
        assert_eq!(out.form_pf_filing_type, FormPfType::Large);
    }

    #[test]
    fn test_form_pf_small_adviser() {
        let mut input = default_input();
        input.total_regulatory_aum = dec!(200_000_000);
        input.funds[0].gross_assets = dec!(120_000_000);
        input.funds[1].gross_assets = dec!(80_000_000);
        let out = generate_sec_cftc_report(&input).unwrap();
        assert_eq!(out.form_pf_filing_type, FormPfType::Small);
    }

    #[test]
    fn test_form_pf_exempt_not_registered() {
        let mut input = default_input();
        input.sec_registered = false;
        let out = generate_sec_cftc_report(&input).unwrap();
        assert_eq!(out.form_pf_filing_type, FormPfType::Exempt);
        assert!(!out.form_pf_required);
    }

    #[test]
    fn test_form_pf_exempt_below_150m() {
        let mut input = default_input();
        input.total_regulatory_aum = dec!(100_000_000);
        let out = generate_sec_cftc_report(&input).unwrap();
        assert_eq!(out.form_pf_filing_type, FormPfType::Exempt);
    }

    #[test]
    fn test_form_pf_at_exact_150m_boundary() {
        let mut input = default_input();
        input.total_regulatory_aum = dec!(150_000_000);
        input.funds[0].gross_assets = dec!(100_000_000);
        input.funds[1].gross_assets = dec!(50_000_000);
        let out = generate_sec_cftc_report(&input).unwrap();
        // At exactly 150M, qualifies as Small
        assert_eq!(out.form_pf_filing_type, FormPfType::Small);
        assert!(out.form_pf_required);
    }

    #[test]
    fn test_form_pf_large_pe_threshold() {
        let mut input = default_input();
        input.funds[1].gross_assets = dec!(2_100_000_000);
        input.total_regulatory_aum = dec!(3_300_000_000);
        let out = generate_sec_cftc_report(&input).unwrap();
        assert_eq!(out.form_pf_filing_type, FormPfType::Large);
        assert!(out.aum_threshold_analysis.is_large_pe);
    }

    #[test]
    fn test_form_pf_large_liquidity_threshold() {
        let mut input = default_input();
        input.funds[0].is_hedge_fund = false;
        input.funds[0].is_liquidity_fund = true;
        input.funds[0].gross_assets = dec!(1_100_000_000);
        let out = generate_sec_cftc_report(&input).unwrap();
        assert_eq!(out.form_pf_filing_type, FormPfType::Large);
        assert!(out.aum_threshold_analysis.is_large_liquidity);
    }

    // --- Form PF sections tests ---

    #[test]
    fn test_form_pf_section_1_always_required() {
        let input = default_input();
        let out = generate_sec_cftc_report(&input).unwrap();
        assert!(out
            .form_pf_sections_required
            .iter()
            .any(|s| s.contains("Section 1")));
    }

    #[test]
    fn test_form_pf_section_2_for_large_hf() {
        let mut input = default_input();
        input.funds[0].gross_assets = dec!(1_600_000_000);
        input.total_regulatory_aum = dec!(2_400_000_000);
        let out = generate_sec_cftc_report(&input).unwrap();
        assert!(out
            .form_pf_sections_required
            .iter()
            .any(|s| s.contains("Section 2")));
    }

    #[test]
    fn test_form_pf_no_section_2_for_small_hf() {
        let mut input = default_input();
        input.total_regulatory_aum = dec!(200_000_000);
        input.funds[0].gross_assets = dec!(120_000_000);
        input.funds[1].gross_assets = dec!(80_000_000);
        let out = generate_sec_cftc_report(&input).unwrap();
        assert!(!out
            .form_pf_sections_required
            .iter()
            .any(|s| s.contains("Section 2")));
    }

    #[test]
    fn test_form_pf_section_4_for_large_pe() {
        let mut input = default_input();
        input.funds[1].gross_assets = dec!(2_100_000_000);
        input.total_regulatory_aum = dec!(3_300_000_000);
        let out = generate_sec_cftc_report(&input).unwrap();
        assert!(out
            .form_pf_sections_required
            .iter()
            .any(|s| s.contains("Section 4")));
    }

    #[test]
    fn test_form_pf_exempt_no_sections() {
        let mut input = default_input();
        input.sec_registered = false;
        let out = generate_sec_cftc_report(&input).unwrap();
        assert!(out.form_pf_sections_required.is_empty());
    }

    // --- Filing frequency tests ---

    #[test]
    fn test_filing_frequency_quarterly_large() {
        let mut input = default_input();
        input.funds[0].gross_assets = dec!(1_600_000_000);
        input.total_regulatory_aum = dec!(2_400_000_000);
        let out = generate_sec_cftc_report(&input).unwrap();
        assert_eq!(out.filing_frequency, FilingFrequency::Quarterly);
    }

    #[test]
    fn test_filing_frequency_annual_small() {
        let mut input = default_input();
        input.total_regulatory_aum = dec!(200_000_000);
        input.funds[0].gross_assets = dec!(120_000_000);
        input.funds[1].gross_assets = dec!(80_000_000);
        let out = generate_sec_cftc_report(&input).unwrap();
        assert_eq!(out.filing_frequency, FilingFrequency::Annual);
    }

    #[test]
    fn test_filing_frequency_exempt() {
        let mut input = default_input();
        input.sec_registered = false;
        let out = generate_sec_cftc_report(&input).unwrap();
        assert_eq!(out.filing_frequency, FilingFrequency::Exempt);
    }

    // --- Filing deadline tests ---

    #[test]
    fn test_filing_deadline_large_60_days() {
        let mut input = default_input();
        input.funds[0].gross_assets = dec!(1_600_000_000);
        input.total_regulatory_aum = dec!(2_400_000_000);
        let out = generate_sec_cftc_report(&input).unwrap();
        // 2024-12-31 + 60 days = 2025-03-01
        assert_eq!(out.filing_deadline, "2025-03-01");
    }

    #[test]
    fn test_filing_deadline_small_120_days() {
        let mut input = default_input();
        input.total_regulatory_aum = dec!(200_000_000);
        input.funds[0].gross_assets = dec!(120_000_000);
        input.funds[1].gross_assets = dec!(80_000_000);
        let out = generate_sec_cftc_report(&input).unwrap();
        // 2024-12-31 + 120 days = 2025-04-30
        assert_eq!(out.filing_deadline, "2025-04-30");
    }

    #[test]
    fn test_filing_deadline_exempt() {
        let mut input = default_input();
        input.sec_registered = false;
        let out = generate_sec_cftc_report(&input).unwrap();
        assert_eq!(out.filing_deadline, "N/A");
    }

    // --- CFTC CPO-PQR tests ---

    #[test]
    fn test_cpo_pqr_large_by_aum() {
        let mut input = default_input();
        input.total_regulatory_aum = dec!(2_000_000_000);
        let out = generate_sec_cftc_report(&input).unwrap();
        assert!(out.cpo_pqr_required);
        assert_eq!(out.cpo_pqr_type, CpoPqrType::Large);
    }

    #[test]
    fn test_cpo_pqr_large_by_pool_size() {
        let mut input = default_input();
        input.total_regulatory_aum = dec!(500_000_000);
        input.funds[0].gross_assets = dec!(600_000_000);
        let out = generate_sec_cftc_report(&input).unwrap();
        assert_eq!(out.cpo_pqr_type, CpoPqrType::Large);
    }

    #[test]
    fn test_cpo_pqr_small() {
        let mut input = default_input();
        input.total_regulatory_aum = dec!(500_000_000);
        input.funds[0].gross_assets = dec!(300_000_000);
        input.funds[1].gross_assets = dec!(200_000_000);
        let out = generate_sec_cftc_report(&input).unwrap();
        assert_eq!(out.cpo_pqr_type, CpoPqrType::Small);
    }

    #[test]
    fn test_cpo_pqr_exempt_not_nfa_registered() {
        let mut input = default_input();
        input.nfa_registered = false;
        let out = generate_sec_cftc_report(&input).unwrap();
        assert_eq!(out.cpo_pqr_type, CpoPqrType::Exempt);
        assert!(!out.cpo_pqr_required);
    }

    #[test]
    fn test_cpo_pqr_exempt_no_commodity_pool() {
        let mut input = default_input();
        input.commodity_pool = false;
        let out = generate_sec_cftc_report(&input).unwrap();
        assert_eq!(out.cpo_pqr_type, CpoPqrType::Exempt);
    }

    // --- Validation tests ---

    #[test]
    fn test_empty_adviser_name_rejected() {
        let mut input = default_input();
        input.adviser_name = "".to_string();
        let err = generate_sec_cftc_report(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "adviser_name");
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[test]
    fn test_negative_aum_rejected() {
        let mut input = default_input();
        input.total_regulatory_aum = dec!(-100);
        let err = generate_sec_cftc_report(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "total_regulatory_aum");
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[test]
    fn test_empty_reporting_date_rejected() {
        let mut input = default_input();
        input.reporting_date = "".to_string();
        let err = generate_sec_cftc_report(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "reporting_date");
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[test]
    fn test_invalid_date_format_rejected() {
        let mut input = default_input();
        input.reporting_date = "12/31/2024".to_string();
        let err = generate_sec_cftc_report(&input).unwrap_err();
        match err {
            CorpFinanceError::DateError(_) => {}
            _ => panic!("Expected DateError"),
        }
    }

    #[test]
    fn test_negative_fund_nav_rejected() {
        let mut input = default_input();
        input.funds[0].nav = dec!(-100);
        let err = generate_sec_cftc_report(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert!(field.contains("nav"));
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[test]
    fn test_negative_gross_assets_rejected() {
        let mut input = default_input();
        input.funds[0].gross_assets = dec!(-100);
        let err = generate_sec_cftc_report(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert!(field.contains("gross_assets"));
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[test]
    fn test_negative_borrowings_rejected() {
        let mut input = default_input();
        input.funds[0].total_borrowings = dec!(-100);
        let err = generate_sec_cftc_report(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert!(field.contains("total_borrowings"));
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[test]
    fn test_negative_counterparty_exposure_rejected() {
        let mut input = default_input();
        input.counterparties[0].exposure = dec!(-100);
        let err = generate_sec_cftc_report(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert!(field.contains("counterparties"));
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    // --- Borrowing analysis tests ---

    #[test]
    fn test_borrowing_ratios() {
        let input = default_input();
        let out = generate_sec_cftc_report(&input).unwrap();
        // Total NAV = 800M + 600M = 1.4B
        // Total borrowings = 200M + 300M = 500M
        // Secured = 150M + 250M = 400M
        // Unsecured = 100M
        let expected_total_ratio = dec!(500_000_000) / dec!(1_400_000_000);
        assert_eq!(out.borrowing_analysis.total_ratio, expected_total_ratio);
        let expected_secured_ratio = dec!(400_000_000) / dec!(1_400_000_000);
        assert_eq!(out.borrowing_analysis.secured_ratio, expected_secured_ratio);
    }

    #[test]
    fn test_borrowing_ratio_zero_nav() {
        let mut input = default_input();
        input.funds[0].nav = dec!(0);
        input.funds[1].nav = dec!(0);
        let out = generate_sec_cftc_report(&input).unwrap();
        assert_eq!(out.borrowing_analysis.total_ratio, dec!(0));
    }

    #[test]
    fn test_high_borrowing_warning() {
        let mut input = default_input();
        // Total NAV = 800M + 600M = 1.4B; need borrowings > 2 * 1.4B = 2.8B
        input.funds[0].total_borrowings = dec!(3_000_000_000);
        input.funds[0].secured_borrowings = dec!(2_000_000_000);
        let out = generate_sec_cftc_report(&input).unwrap();
        assert!(out.warnings.iter().any(|w| w.contains("high leverage")));
    }

    #[test]
    fn test_secured_exceeds_total_warning() {
        let mut input = default_input();
        input.funds[0].secured_borrowings = dec!(300_000_000);
        input.funds[0].total_borrowings = dec!(200_000_000);
        let out = generate_sec_cftc_report(&input).unwrap();
        assert!(out
            .warnings
            .iter()
            .any(|w| w.contains("secured borrowings")));
    }

    // --- Counterparty concentration tests ---

    #[test]
    fn test_counterparty_hhi_calculation() {
        let input = default_input();
        let out = generate_sec_cftc_report(&input).unwrap();
        // Total exposure = 400M + 300M + 200M = 900M
        // Shares: 44.44%, 33.33%, 22.22%
        // HHI = 44.44^2 + 33.33^2 + 22.22^2 = ~3580
        assert!(out.counterparty_concentration > dec!(3000));
    }

    #[test]
    fn test_counterparty_hhi_high_concentration_warning() {
        let input = default_input();
        let out = generate_sec_cftc_report(&input).unwrap();
        assert!(out.warnings.iter().any(|w| w.contains("HHI")));
    }

    #[test]
    fn test_counterparty_hhi_equal_exposure() {
        let cps = vec![
            CounterpartyInfo {
                name: "A".to_string(),
                exposure: dec!(100),
                secured_pct: dec!(50),
            },
            CounterpartyInfo {
                name: "B".to_string(),
                exposure: dec!(100),
                secured_pct: dec!(50),
            },
            CounterpartyInfo {
                name: "C".to_string(),
                exposure: dec!(100),
                secured_pct: dec!(50),
            },
            CounterpartyInfo {
                name: "D".to_string(),
                exposure: dec!(100),
                secured_pct: dec!(50),
            },
        ];
        let hhi = calculate_counterparty_hhi(&cps);
        // 4 equal shares: each 25%, HHI = 4 * 625 = 2500
        assert_eq!(hhi, dec!(2500));
    }

    #[test]
    fn test_counterparty_hhi_single_counterparty() {
        let cps = vec![CounterpartyInfo {
            name: "Only".to_string(),
            exposure: dec!(100),
            secured_pct: dec!(80),
        }];
        let hhi = calculate_counterparty_hhi(&cps);
        // 100% share → HHI = 10000
        assert_eq!(hhi, dec!(10000));
    }

    #[test]
    fn test_counterparty_hhi_empty() {
        let hhi = calculate_counterparty_hhi(&[]);
        assert_eq!(hhi, dec!(0));
    }

    // --- Performance calculation tests ---

    #[test]
    fn test_annualized_return_basic() {
        // 12 months of 1% → cumulative ~12.68%, annualized = same (12 months)
        let returns: Vec<Decimal> = vec![dec!(0.01); 12];
        let ann = annualized_return(&returns);
        // (1.01)^12 - 1 ≈ 0.1268
        let diff = (ann - dec!(0.1268)).abs();
        assert!(diff < dec!(0.01), "ann={}", ann);
    }

    #[test]
    fn test_annualized_return_empty() {
        let ann = annualized_return(&[]);
        assert_eq!(ann, dec!(0));
    }

    #[test]
    fn test_annualized_return_single_month() {
        let returns = vec![dec!(0.05)];
        let ann = annualized_return(&returns);
        // Should annualize a single month's return
        assert!(ann > dec!(0.5), "ann={}", ann); // 5% * ~12 annualized should be big
    }

    #[test]
    fn test_max_drawdown_no_loss() {
        let returns = vec![dec!(0.01), dec!(0.02), dec!(0.01)];
        let dd = max_drawdown(&returns);
        assert_eq!(dd, dec!(0));
    }

    #[test]
    fn test_max_drawdown_basic() {
        // Up 10%, then down 20% → peak=1.1, trough=1.1*0.8=0.88, dd=(1.1-0.88)/1.1=0.2
        let returns = vec![dec!(0.10), dec!(-0.20)];
        let dd = max_drawdown(&returns);
        assert_eq!(dd, dec!(0.2));
    }

    #[test]
    fn test_max_drawdown_empty() {
        let dd = max_drawdown(&[]);
        assert_eq!(dd, dec!(0));
    }

    #[test]
    fn test_sharpe_ratio_zero_returns() {
        let returns = vec![dec!(0), dec!(0), dec!(0)];
        let sr = sharpe_ratio(&returns);
        assert_eq!(sr, dec!(0));
    }

    #[test]
    fn test_sharpe_ratio_single_return() {
        let returns = vec![dec!(0.01)];
        let sr = sharpe_ratio(&returns);
        // Not enough data for std dev (n < 2)
        assert_eq!(sr, dec!(0));
    }

    #[test]
    fn test_sharpe_ratio_positive() {
        // All positive returns with low volatility → positive Sharpe
        let returns = vec![
            dec!(0.01),
            dec!(0.02),
            dec!(0.01),
            dec!(0.02),
            dec!(0.01),
            dec!(0.02),
        ];
        let sr = sharpe_ratio(&returns);
        assert!(sr > dec!(0), "Sharpe={}", sr);
    }

    // --- Regulatory AUM calculation tests ---

    #[test]
    fn test_regulatory_aum_matches_gross() {
        let input = default_input();
        let out = generate_sec_cftc_report(&input).unwrap();
        // Sum of gross_assets: 1.2B + 0.8B = 2.0B
        assert_eq!(out.regulatory_aum_calculation, dec!(2_000_000_000));
    }

    #[test]
    fn test_regulatory_aum_mismatch_warning() {
        let mut input = default_input();
        input.total_regulatory_aum = dec!(5_000_000_000); // Intentional mismatch
        let out = generate_sec_cftc_report(&input).unwrap();
        assert!(out.warnings.iter().any(|w| w.contains("regulatory AUM")));
    }

    // --- Threshold analysis tests ---

    #[test]
    fn test_threshold_analysis_hedge_fund_aum() {
        let input = default_input();
        let out = generate_sec_cftc_report(&input).unwrap();
        // Fund 0 is hedge fund with gross_assets = 1.2B
        assert_eq!(
            out.aum_threshold_analysis.hedge_fund_aum,
            dec!(1_200_000_000)
        );
    }

    #[test]
    fn test_threshold_analysis_pe_aum() {
        let input = default_input();
        let out = generate_sec_cftc_report(&input).unwrap();
        assert_eq!(out.aum_threshold_analysis.pe_fund_aum, dec!(800_000_000));
    }

    #[test]
    fn test_threshold_analysis_not_large_adviser() {
        let input = default_input();
        let out = generate_sec_cftc_report(&input).unwrap();
        // HF AUM = 1.2B, below 1.5B threshold
        assert!(!out.aum_threshold_analysis.is_large_adviser);
    }

    // --- Data completeness score tests ---

    #[test]
    fn test_completeness_score_good_data() {
        let input = default_input();
        let out = generate_sec_cftc_report(&input).unwrap();
        // Should be high with good data
        assert!(out.data_completeness_score > dec!(70));
    }

    #[test]
    fn test_completeness_score_deducts_for_no_returns() {
        let mut input = default_input();
        input.funds[0].monthly_returns = vec![];
        input.funds[1].monthly_returns = vec![];
        let out = generate_sec_cftc_report(&input).unwrap();
        // Deducted 5 per fund without returns = 10 less
        assert!(out.data_completeness_score < dec!(100));
    }

    #[test]
    fn test_completeness_score_deducts_for_no_counterparties() {
        let mut input = default_input();
        input.counterparties = vec![];
        let out = generate_sec_cftc_report(&input).unwrap();
        assert!(out.data_completeness_score < dec!(95));
    }

    #[test]
    fn test_completeness_score_floor_at_zero() {
        let mut input = default_input();
        input.funds = vec![];
        input.counterparties = vec![];
        // This generates warnings too, further lowering score
        let out = generate_sec_cftc_report(&input).unwrap();
        assert!(out.data_completeness_score >= dec!(0));
    }

    // --- Warning tests ---

    #[test]
    fn test_fund_count_mismatch_warning() {
        let mut input = default_input();
        input.fund_count = 5; // Mismatches actual 2 funds
        let out = generate_sec_cftc_report(&input).unwrap();
        assert!(out.warnings.iter().any(|w| w.contains("fund_count")));
    }

    #[test]
    fn test_venue_breakdown_warning() {
        let mut input = default_input();
        input.otc_bilateral_pct = dec!(50);
        input.otc_cleared_pct = dec!(50);
        input.exchange_traded_pct = dec!(50); // sum = 150
        let out = generate_sec_cftc_report(&input).unwrap();
        assert!(out.warnings.iter().any(|w| w.contains("Trading venue")));
    }

    #[test]
    fn test_management_fee_out_of_range_warning() {
        let mut input = default_input();
        input.funds[0].management_fee_rate = dec!(2.5); // > 1 (100%)
        let out = generate_sec_cftc_report(&input).unwrap();
        assert!(out
            .warnings
            .iter()
            .any(|w| w.contains("management_fee_rate")));
    }

    #[test]
    fn test_us_investor_pct_out_of_range_warning() {
        let mut input = default_input();
        input.funds[0].us_investor_pct = dec!(110);
        let out = generate_sec_cftc_report(&input).unwrap();
        assert!(out.warnings.iter().any(|w| w.contains("us_investor_pct")));
    }

    // --- Edge case tests ---

    #[test]
    fn test_no_funds_produces_report() {
        let mut input = default_input();
        input.funds = vec![];
        input.fund_count = 0;
        let out = generate_sec_cftc_report(&input).unwrap();
        assert!(out.performance_summary.fund_performances.is_empty());
    }

    #[test]
    fn test_single_fund() {
        let mut input = default_input();
        input.funds.truncate(1);
        input.fund_count = 1;
        let out = generate_sec_cftc_report(&input).unwrap();
        assert_eq!(out.performance_summary.fund_performances.len(), 1);
    }

    #[test]
    fn test_zero_aum_produces_report() {
        let mut input = default_input();
        input.total_regulatory_aum = dec!(0);
        input.funds = vec![];
        input.fund_count = 0;
        let out = generate_sec_cftc_report(&input).unwrap();
        assert_eq!(out.form_pf_filing_type, FormPfType::Exempt);
    }

    // --- Serialization tests ---

    #[test]
    fn test_output_serializes_to_json() {
        let input = default_input();
        let out = generate_sec_cftc_report(&input).unwrap();
        let json = serde_json::to_string(&out).unwrap();
        let deserialized: SecCftcReportingOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.form_pf_required, out.form_pf_required);
        assert_eq!(deserialized.filing_frequency, out.filing_frequency);
    }

    #[test]
    fn test_input_deserializes_from_json() {
        let input = default_input();
        let json = serde_json::to_string(&input).unwrap();
        let deserialized: SecCftcReportingInput = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.adviser_name, input.adviser_name);
        assert_eq!(
            deserialized.total_regulatory_aum,
            input.total_regulatory_aum
        );
    }

    // --- Methodology and assumptions tests ---

    #[test]
    fn test_methodology_is_populated() {
        let input = default_input();
        let out = generate_sec_cftc_report(&input).unwrap();
        assert!(out.methodology.contains("Form PF"));
        assert!(out.methodology.contains("CPO-PQR"));
    }

    #[test]
    fn test_assumptions_are_populated() {
        let input = default_input();
        let out = generate_sec_cftc_report(&input).unwrap();
        assert!(!out.assumptions.is_empty());
        assert!(out.assumptions.iter().any(|a| a.contains("USD")));
    }

    // --- Performance summary tests ---

    #[test]
    fn test_performance_summary_aggregate_nav() {
        let input = default_input();
        let out = generate_sec_cftc_report(&input).unwrap();
        assert_eq!(out.performance_summary.aggregate_nav, dec!(1_400_000_000));
    }

    #[test]
    fn test_performance_summary_aggregate_gross() {
        let input = default_input();
        let out = generate_sec_cftc_report(&input).unwrap();
        assert_eq!(
            out.performance_summary.aggregate_gross_assets,
            dec!(2_000_000_000)
        );
    }

    #[test]
    fn test_fund_performance_monthly_count() {
        let input = default_input();
        let out = generate_sec_cftc_report(&input).unwrap();
        assert_eq!(
            out.performance_summary.fund_performances[0].monthly_count,
            12
        );
        assert_eq!(
            out.performance_summary.fund_performances[1].monthly_count,
            6
        );
    }

    #[test]
    fn test_fund_borrowing_ratio_in_performance() {
        let input = default_input();
        let out = generate_sec_cftc_report(&input).unwrap();
        // Fund 0: borrowings=200M, NAV=800M → ratio=0.25
        let ratio = out.performance_summary.fund_performances[0].borrowing_ratio;
        assert_eq!(ratio, dec!(0.25));
    }

    // --- Helper function unit tests ---

    #[test]
    fn test_newton_sqrt_basic() {
        let result = newton_sqrt(dec!(4));
        let diff = (result - dec!(2)).abs();
        assert!(diff < dec!(0.0001), "sqrt(4)={}", result);
    }

    #[test]
    fn test_newton_sqrt_zero() {
        assert_eq!(newton_sqrt(dec!(0)), dec!(0));
    }

    #[test]
    fn test_newton_sqrt_negative() {
        assert_eq!(newton_sqrt(dec!(-1)), dec!(0));
    }

    #[test]
    fn test_taylor_exp_zero() {
        let result = taylor_exp(dec!(0));
        assert_eq!(result, dec!(1));
    }

    #[test]
    fn test_taylor_exp_one() {
        let result = taylor_exp(dec!(1));
        let diff = (result - dec!(2.71828)).abs();
        assert!(diff < dec!(0.001), "exp(1)={}", result);
    }

    #[test]
    fn test_newton_ln_one() {
        let result = newton_ln(dec!(1));
        let diff = result.abs();
        assert!(diff < dec!(0.001), "ln(1)={}", result);
    }

    #[test]
    fn test_newton_ln_e() {
        // ln(e) ≈ 1
        let e = taylor_exp(dec!(1));
        let result = newton_ln(e);
        let diff = (result - dec!(1)).abs();
        assert!(diff < dec!(0.001), "ln(e)={}", result);
    }

    #[test]
    fn test_empty_fund_name_warning() {
        let mut input = default_input();
        input.funds[0].name = "".to_string();
        let out = generate_sec_cftc_report(&input).unwrap();
        assert!(out.warnings.iter().any(|w| w.contains("empty name")));
    }
}
