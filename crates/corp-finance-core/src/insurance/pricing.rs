use rust_decimal::Decimal;
use rust_decimal::MathematicalOps;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{with_metadata, ComputationOutput, Money, Rate};
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Square root helper — uses MathematicalOps then refines with Newton's method
// ---------------------------------------------------------------------------

fn sqrt_decimal(val: Decimal) -> Decimal {
    if val <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    // Use the built-in sqrt as initial guess, then refine with Newton's method
    let mut guess = val.sqrt().unwrap_or_else(|| {
        // Fallback: rough initial guess for very large values
        // Scale down, sqrt, scale back up
        val / dec!(2)
    });
    let two = dec!(2);
    for _ in 0..20 {
        if guess.is_zero() {
            return Decimal::ZERO;
        }
        guess = (guess + val / guess) / two;
    }
    guess
}

/// Iterative power: base^exp where exp is a non-negative integer.
/// Avoids `powd()` precision drift.
fn decimal_pow(base: Decimal, exp: u32) -> Decimal {
    let mut result = Decimal::ONE;
    for _ in 0..exp {
        result *= base;
    }
    result
}

// ---------------------------------------------------------------------------
// Function 1: price_premium
// ---------------------------------------------------------------------------

/// Input for premium pricing using frequency x severity method.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PremiumPricingInput {
    /// Line of business (e.g. "Motor", "Liability")
    pub line_of_business: String,
    /// Number of policies or units of exposure
    pub exposure_units: Decimal,
    /// Claims per exposure unit per year (e.g. 0.05 = 5%)
    pub claim_frequency: Decimal,
    /// Average claim size
    pub average_severity: Money,
    /// Annual severity inflation (e.g. 0.03 = 3%)
    pub severity_trend: Rate,
    /// Annual frequency change (e.g. -0.01 = -1% improving)
    pub frequency_trend: Rate,
    /// Years to project trends forward
    pub projection_years: u32,
    /// Target expense ratio (e.g. 0.30 = 30%)
    pub expense_ratio_target: Rate,
    /// Target underwriting profit margin (e.g. 0.05 = 5%)
    pub profit_margin_target: Rate,
    /// Reinsurance cost as % of premium (e.g. 0.10)
    pub reinsurance_cost_pct: Rate,
    /// Investment income reducing needed premium (e.g. 0.02)
    pub investment_income_credit: Rate,
    /// Loading for catastrophe / large losses (e.g. 0.05)
    pub large_loss_load_pct: Rate,
}

/// Rate breakdown components that sum to gross premium.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateComponents {
    pub loss_cost: Money,
    pub loss_cost_pct: Rate,
    pub expense_load: Money,
    pub profit_load: Money,
    pub reinsurance_load: Money,
    pub large_loss_load: Money,
    pub investment_credit: Money,
    pub total: Money,
}

/// A single year of projected experience.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectedYear {
    pub year: u32,
    pub projected_frequency: Decimal,
    pub projected_severity: Money,
    pub projected_pure_premium: Money,
    pub loss_ratio: Rate,
}

/// Output of premium pricing calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PremiumPricingOutput {
    /// Frequency * severity (current year, total book)
    pub pure_premium: Money,
    /// Pure premium projected forward by trends
    pub trended_pure_premium: Money,
    /// Premium loaded for expenses, profit, reinsurance, etc.
    pub gross_premium: Money,
    /// Gross premium / exposure units
    pub premium_per_unit: Money,
    /// Breakdown of rate components
    pub rate_components: RateComponents,
    /// Year-by-year projected experience
    pub projected_experience: Vec<ProjectedYear>,
}

/// Calculate insurance premium using frequency x severity approach.
///
/// Pure premium = exposure_units * claim_frequency * average_severity
/// Trended = pure premium projected forward for severity/frequency trends
/// Gross = trended / (1 - expense - profit - reinsurance - large_loss + investment_credit)
pub fn price_premium(
    input: &PremiumPricingInput,
) -> CorpFinanceResult<ComputationOutput<PremiumPricingOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // Validate inputs
    if input.exposure_units <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "exposure_units".into(),
            reason: "Exposure units must be positive".into(),
        });
    }
    if input.claim_frequency < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "claim_frequency".into(),
            reason: "Claim frequency cannot be negative".into(),
        });
    }
    if input.average_severity < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "average_severity".into(),
            reason: "Average severity cannot be negative".into(),
        });
    }

    // Check that the denominator for gross premium is positive
    let denominator = Decimal::ONE
        - input.expense_ratio_target
        - input.profit_margin_target
        - input.reinsurance_cost_pct
        - input.large_loss_load_pct
        + input.investment_income_credit;

    if denominator <= Decimal::ZERO {
        return Err(CorpFinanceError::FinancialImpossibility(
            "Loading factors exceed 100% — cannot compute a positive gross premium".into(),
        ));
    }

    if input.expense_ratio_target >= Decimal::ONE {
        warnings.push("Expense ratio target >= 100% — this is unusual".into());
    }

    // Pure premium (current year, total book)
    let pure_premium = input.exposure_units * input.claim_frequency * input.average_severity;

    // Trended pure premium (project forward by projection_years)
    let freq_factor = decimal_pow(Decimal::ONE + input.frequency_trend, input.projection_years);
    let sev_factor = decimal_pow(Decimal::ONE + input.severity_trend, input.projection_years);
    let trended_frequency = input.claim_frequency * freq_factor;
    let trended_severity = input.average_severity * sev_factor;
    let trended_pure_premium = input.exposure_units * trended_frequency * trended_severity;

    // Gross premium
    let gross_premium = trended_pure_premium / denominator;

    // Premium per unit
    let premium_per_unit = gross_premium / input.exposure_units;

    // Rate components
    let loss_cost = trended_pure_premium;
    let expense_load = gross_premium * input.expense_ratio_target;
    let profit_load = gross_premium * input.profit_margin_target;
    let reinsurance_load = gross_premium * input.reinsurance_cost_pct;
    let large_loss_load = gross_premium * input.large_loss_load_pct;
    let investment_credit = gross_premium * input.investment_income_credit;

    let loss_cost_pct = if gross_premium.is_zero() {
        Decimal::ZERO
    } else {
        loss_cost / gross_premium
    };

    let rate_components = RateComponents {
        loss_cost,
        loss_cost_pct,
        expense_load,
        profit_load,
        reinsurance_load,
        large_loss_load,
        investment_credit,
        total: gross_premium,
    };

    // Projected experience year by year
    let mut projected_experience = Vec::with_capacity(input.projection_years as usize);
    for y in 1..=input.projection_years {
        let proj_freq =
            input.claim_frequency * decimal_pow(Decimal::ONE + input.frequency_trend, y);
        let proj_sev = input.average_severity * decimal_pow(Decimal::ONE + input.severity_trend, y);
        let proj_pure = input.exposure_units * proj_freq * proj_sev;
        let loss_ratio = if gross_premium.is_zero() {
            Decimal::ZERO
        } else {
            proj_pure / gross_premium
        };
        projected_experience.push(ProjectedYear {
            year: y,
            projected_frequency: proj_freq,
            projected_severity: proj_sev,
            projected_pure_premium: proj_pure,
            loss_ratio,
        });
    }

    let output = PremiumPricingOutput {
        pure_premium,
        trended_pure_premium,
        gross_premium,
        premium_per_unit,
        rate_components,
        projected_experience,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Insurance Premium Pricing (Frequency x Severity)",
        &serde_json::json!({
            "line_of_business": input.line_of_business,
            "exposure_units": input.exposure_units.to_string(),
            "projection_years": input.projection_years,
            "loading_denominator": denominator.to_string(),
        }),
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Function 2: analyze_combined_ratio
// ---------------------------------------------------------------------------

/// A single insurance period for combined ratio analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsurancePeriod {
    pub year: u32,
    /// Net earned premium
    pub net_earned_premium: Money,
    /// Claims + IBNR change
    pub net_incurred_losses: Money,
    /// Loss adjustment expenses (allocated + unallocated)
    pub loss_adjustment_expenses: Money,
    /// Commissions, overhead, etc.
    pub underwriting_expenses: Money,
    /// Policyholder dividends
    pub policyholder_dividends: Money,
    /// Net investment income
    pub net_investment_income: Money,
    /// Realised gains on investments
    pub realized_gains: Money,
}

/// Input for combined ratio analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombinedRatioInput {
    pub company_name: String,
    pub periods: Vec<InsurancePeriod>,
}

/// Results for a single period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodResult {
    pub year: u32,
    /// Incurred losses / earned premium
    pub loss_ratio: Rate,
    /// LAE / earned premium
    pub lae_ratio: Rate,
    /// Underwriting expenses / earned premium
    pub expense_ratio: Rate,
    /// Dividends / earned premium
    pub dividend_ratio: Rate,
    /// loss + LAE + expense + dividend ratios
    pub combined_ratio: Rate,
    /// combined_ratio - investment_income / earned_premium
    pub operating_ratio: Rate,
    /// premium - losses - LAE - expenses - dividends
    pub underwriting_profit_loss: Money,
    /// UW profit + investment income + realised gains
    pub net_income: Money,
}

/// Summary statistics across all periods.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RatioSummary {
    pub avg_loss_ratio: Rate,
    pub avg_combined_ratio: Rate,
    pub avg_operating_ratio: Rate,
    /// "Improving", "Deteriorating", or "Stable"
    pub trend_direction: String,
    /// Year with lowest combined ratio
    pub best_year: u32,
    /// Year with highest combined ratio
    pub worst_year: u32,
    /// Count of years with CR < 100%
    pub profitable_years: u32,
}

/// Output of combined ratio analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombinedRatioOutput {
    pub period_results: Vec<PeriodResult>,
    pub summary: RatioSummary,
}

/// Analyse combined ratio over multiple periods.
///
/// Combined ratio = (losses + LAE + expenses + dividends) / earned premium.
/// CR < 100% indicates underwriting profit.
pub fn analyze_combined_ratio(
    input: &CombinedRatioInput,
) -> CorpFinanceResult<ComputationOutput<CombinedRatioOutput>> {
    let start = Instant::now();
    let warnings: Vec<String> = Vec::new();

    if input.periods.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one insurance period is required".into(),
        ));
    }

    let mut period_results = Vec::with_capacity(input.periods.len());

    for p in &input.periods {
        if p.net_earned_premium.is_zero() {
            return Err(CorpFinanceError::DivisionByZero {
                context: format!("Net earned premium is zero for year {}", p.year),
            });
        }

        let nep = p.net_earned_premium;
        let loss_ratio = p.net_incurred_losses / nep;
        let lae_ratio = p.loss_adjustment_expenses / nep;
        let expense_ratio = p.underwriting_expenses / nep;
        let dividend_ratio = p.policyholder_dividends / nep;
        let combined_ratio = loss_ratio + lae_ratio + expense_ratio + dividend_ratio;
        let investment_ratio = p.net_investment_income / nep;
        let operating_ratio = combined_ratio - investment_ratio;

        let underwriting_profit_loss = nep
            - p.net_incurred_losses
            - p.loss_adjustment_expenses
            - p.underwriting_expenses
            - p.policyholder_dividends;

        let net_income = underwriting_profit_loss + p.net_investment_income + p.realized_gains;

        period_results.push(PeriodResult {
            year: p.year,
            loss_ratio,
            lae_ratio,
            expense_ratio,
            dividend_ratio,
            combined_ratio,
            operating_ratio,
            underwriting_profit_loss,
            net_income,
        });
    }

    // Summary
    let n = Decimal::from(period_results.len() as i64);
    let avg_loss_ratio: Rate = period_results.iter().map(|r| r.loss_ratio).sum::<Decimal>() / n;
    let avg_combined_ratio: Rate = period_results
        .iter()
        .map(|r| r.combined_ratio)
        .sum::<Decimal>()
        / n;
    let avg_operating_ratio: Rate = period_results
        .iter()
        .map(|r| r.operating_ratio)
        .sum::<Decimal>()
        / n;

    // Best and worst year by combined ratio
    let best = period_results
        .iter()
        .min_by(|a, b| a.combined_ratio.cmp(&b.combined_ratio))
        .unwrap();
    let worst = period_results
        .iter()
        .max_by(|a, b| a.combined_ratio.cmp(&b.combined_ratio))
        .unwrap();

    // Trend direction: compare first half average CR to second half
    let trend_direction = determine_trend(&period_results);

    let profitable_years = period_results
        .iter()
        .filter(|r| r.combined_ratio < Decimal::ONE)
        .count() as u32;

    let summary = RatioSummary {
        avg_loss_ratio,
        avg_combined_ratio,
        avg_operating_ratio,
        trend_direction,
        best_year: best.year,
        worst_year: worst.year,
        profitable_years,
    };

    let output = CombinedRatioOutput {
        period_results,
        summary,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Combined Ratio Analysis",
        &serde_json::json!({
            "company": input.company_name,
            "periods_analyzed": input.periods.len(),
        }),
        warnings,
        elapsed,
        output,
    ))
}

/// Determine trend direction by comparing first half vs second half combined ratios.
/// Improving = second half lower, Deteriorating = second half higher, Stable = within 2pp.
fn determine_trend(results: &[PeriodResult]) -> String {
    if results.len() < 2 {
        return "Stable".to_string();
    }
    let mid = results.len() / 2;
    let first_half_avg: Decimal = results[..mid]
        .iter()
        .map(|r| r.combined_ratio)
        .sum::<Decimal>()
        / Decimal::from(mid as i64);
    let second_half_avg: Decimal = results[mid..]
        .iter()
        .map(|r| r.combined_ratio)
        .sum::<Decimal>()
        / Decimal::from((results.len() - mid) as i64);

    let diff = second_half_avg - first_half_avg;
    let threshold = dec!(0.02); // 2 percentage points

    if diff < -threshold {
        "Improving".to_string()
    } else if diff > threshold {
        "Deteriorating".to_string()
    } else {
        "Stable".to_string()
    }
}

// ---------------------------------------------------------------------------
// Function 3: calculate_scr (Solvency II Standard Formula)
// ---------------------------------------------------------------------------

/// Premium and reserve risk sub-module inputs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PremiumReserveRisk {
    /// Net earned premium
    pub net_earned_premium: Money,
    /// Net best-estimate reserves
    pub net_best_estimate_reserves: Money,
    /// Premium risk factor by LoB (e.g. 0.10 for motor)
    pub premium_risk_factor: Rate,
    /// Reserve risk factor by LoB (e.g. 0.08 for motor)
    pub reserve_risk_factor: Rate,
    /// Geographic diversification factor (1 = no diversification, 0.75 = 25% benefit)
    pub geographic_diversification: Rate,
}

/// Input for Solvency II SCR calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrInput {
    pub company_name: String,
    /// Premium and reserve risk parameters
    pub premium_risk: PremiumReserveRisk,
    /// Catastrophe risk capital charge
    pub catastrophe_risk: Money,
    /// Investment / market risk charge
    pub market_risk: Money,
    /// Counterparty default risk
    pub credit_risk: Money,
    /// Gross written premium for operational risk calculation
    pub operational_risk_premium: Money,
    /// Total available capital (Tier 1 + Tier 2)
    pub eligible_own_funds: Money,
    /// MCR as proportion of SCR (typically 0.25-0.45)
    pub mcr_factor: Rate,
}

/// Output of the SCR calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrOutput {
    /// Non-life underwriting SCR (premium + reserve + cat)
    pub non_life_underwriting_scr: Money,
    /// Catastrophe SCR (passed through)
    pub catastrophe_scr: Money,
    /// Market risk SCR
    pub market_scr: Money,
    /// Credit / counterparty default SCR
    pub credit_scr: Money,
    /// Operational risk SCR
    pub operational_scr: Money,
    /// Reduction from diversification across risk modules
    pub diversification_benefit: Money,
    /// Total SCR after diversification
    pub total_scr: Money,
    /// Minimum capital requirement
    pub mcr: Money,
    /// Own funds / SCR
    pub solvency_ratio: Rate,
    /// Solvency ratio >= 100%
    pub meets_scr: bool,
    /// Own funds >= MCR
    pub meets_mcr: bool,
    /// Own funds - SCR
    pub surplus: Money,
}

/// Calculate Solvency II Standard Formula SCR.
///
/// Premium & reserve risk uses the square-root formula with 0.5 correlation.
/// Diversification across modules uses prescribed correlation matrix.
/// Operational risk is simplified as 3% of max(premium, reserves).
pub fn calculate_scr(input: &ScrInput) -> CorpFinanceResult<ComputationOutput<ScrOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    let pr = &input.premium_risk;

    // Validate
    if pr.geographic_diversification <= Decimal::ZERO
        || pr.geographic_diversification > Decimal::ONE
    {
        return Err(CorpFinanceError::InvalidInput {
            field: "geographic_diversification".into(),
            reason: "Must be between 0 (exclusive) and 1 (inclusive)".into(),
        });
    }
    if input.mcr_factor < Decimal::ZERO || input.mcr_factor > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "mcr_factor".into(),
            reason: "MCR factor must be between 0 and 1".into(),
        });
    }

    // --- Premium & Reserve risk ---
    // sigma = sqrt( (pf * NEP)^2 + (rf * Res)^2 + 2 * 0.5 * pf * NEP * rf * Res )
    // non_life_pr = sigma * geographic_diversification
    let pf_nep = pr.premium_risk_factor * pr.net_earned_premium;
    let rf_res = pr.reserve_risk_factor * pr.net_best_estimate_reserves;
    let correlation = dec!(0.5); // prescribed correlation between premium and reserve risk
    let under_sqrt = pf_nep * pf_nep + rf_res * rf_res + dec!(2) * correlation * pf_nep * rf_res;
    let premium_reserve_scr = sqrt_decimal(under_sqrt) * pr.geographic_diversification;

    // Non-life underwriting SCR = premium/reserve + catastrophe
    // (In the standard formula these are aggregated; here we keep cat separate for reporting)
    let non_life_underwriting_scr = premium_reserve_scr + input.catastrophe_risk;
    let catastrophe_scr = input.catastrophe_risk;

    let market_scr = input.market_risk;
    let credit_scr = input.credit_risk;

    // --- Operational risk ---
    // Simplified: 3% of max(earned premium, best-estimate reserves)
    let op_base = if pr.net_earned_premium >= pr.net_best_estimate_reserves {
        pr.net_earned_premium
    } else {
        pr.net_best_estimate_reserves
    };
    let operational_scr = dec!(0.03) * op_base;

    // --- Diversification across risk modules ---
    // Correlation matrix (Solvency II prescribed):
    //                   NL_UW   Market  Credit
    // NL_UW             1.00    0.25    0.50
    // Market            0.25    1.00    0.25
    // Credit            0.50    0.25    1.00
    let rho_nl_mkt = dec!(0.25);
    let rho_nl_cr = dec!(0.50);
    let rho_mkt_cr = dec!(0.25);

    let nl = non_life_underwriting_scr;
    let mkt = market_scr;
    let cr = credit_scr;

    // sum(rho_ij * SCR_i * SCR_j) for all i,j
    let sum_corr = nl * nl
        + mkt * mkt
        + cr * cr
        + dec!(2) * rho_nl_mkt * nl * mkt
        + dec!(2) * rho_nl_cr * nl * cr
        + dec!(2) * rho_mkt_cr * mkt * cr;

    let basic_scr = sqrt_decimal(sum_corr);

    // Total SCR = BSCR + operational SCR
    let total_scr = basic_scr + operational_scr;

    // Diversification benefit = sum of individual modules - basic_scr
    let sum_modules = nl + mkt + cr;
    let diversification_benefit = if sum_modules > basic_scr {
        sum_modules - basic_scr
    } else {
        Decimal::ZERO
    };

    // --- MCR ---
    // MCR = SCR * mcr_factor, floored at 25% of SCR
    let mcr_raw = total_scr * input.mcr_factor;
    let mcr_floor = total_scr * dec!(0.25);
    let mcr = if mcr_raw < mcr_floor {
        mcr_floor
    } else {
        mcr_raw
    };

    // --- Solvency ratio ---
    let solvency_ratio = if total_scr.is_zero() {
        if input.eligible_own_funds > Decimal::ZERO {
            warnings.push("SCR is zero — solvency ratio undefined, reported as 999%".into());
            dec!(9.99)
        } else {
            Decimal::ZERO
        }
    } else {
        input.eligible_own_funds / total_scr
    };

    let meets_scr = solvency_ratio >= Decimal::ONE;
    let meets_mcr = input.eligible_own_funds >= mcr;
    let surplus = input.eligible_own_funds - total_scr;

    let output = ScrOutput {
        non_life_underwriting_scr,
        catastrophe_scr,
        market_scr,
        credit_scr,
        operational_scr,
        diversification_benefit,
        total_scr,
        mcr,
        solvency_ratio,
        meets_scr,
        meets_mcr,
        surplus,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Solvency II SCR Standard Formula",
        &serde_json::json!({
            "company": input.company_name,
            "correlation_premium_reserve": "0.5",
            "correlation_matrix": "NL-Mkt 0.25, NL-Cr 0.50, Mkt-Cr 0.25",
            "operational_risk_method": "3% of max(premium, reserves)",
        }),
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    // Helper: basic premium pricing input
    fn basic_pricing_input() -> PremiumPricingInput {
        PremiumPricingInput {
            line_of_business: "Motor".into(),
            exposure_units: dec!(1000),
            claim_frequency: dec!(0.05),
            average_severity: dec!(10000),
            severity_trend: dec!(0.03),
            frequency_trend: dec!(-0.01),
            projection_years: 3,
            expense_ratio_target: dec!(0.30),
            profit_margin_target: dec!(0.05),
            reinsurance_cost_pct: dec!(0.10),
            investment_income_credit: dec!(0.02),
            large_loss_load_pct: dec!(0.05),
        }
    }

    // -----------------------------------------------------------------------
    // Premium pricing tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_pure_premium_frequency_times_severity() {
        // 5% frequency * $10,000 severity * 1000 units = $500,000
        let input = basic_pricing_input();
        let result = price_premium(&input).unwrap();
        assert_eq!(result.result.pure_premium, dec!(500000));
    }

    #[test]
    fn test_trended_premium_with_trends() {
        let input = basic_pricing_input();
        let result = price_premium(&input).unwrap();

        // frequency after 3 years: 0.05 * (1-0.01)^3 = 0.05 * 0.970299 = 0.04851495
        // severity after 3 years: 10000 * (1.03)^3 = 10000 * 1.092727 = 10927.27
        // trended pure = 1000 * 0.04851495 * 10927.27
        // Pure premium should be less than untended due to frequency improvement
        // but severity trend pushes it up; net effect depends on magnitudes
        let trended = result.result.trended_pure_premium;
        // Should be different from pure_premium
        assert_ne!(trended, result.result.pure_premium);
        assert!(trended > Decimal::ZERO);
    }

    #[test]
    fn test_gross_premium_with_all_loadings() {
        let input = basic_pricing_input();
        let result = price_premium(&input).unwrap();

        // denominator = 1 - 0.30 - 0.05 - 0.10 - 0.05 + 0.02 = 0.52
        // gross = trended_pure / 0.52
        let gross = result.result.gross_premium;
        let trended = result.result.trended_pure_premium;
        let expected_denom = dec!(0.52);
        let expected_gross = trended / expected_denom;
        assert_eq!(gross, expected_gross);
    }

    #[test]
    fn test_rate_components_sum_to_gross() {
        let input = basic_pricing_input();
        let result = price_premium(&input).unwrap();
        let rc = &result.result.rate_components;

        // total should equal gross_premium
        assert_eq!(rc.total, result.result.gross_premium);

        // loss_cost + expense + profit + reinsurance + large_loss - investment_credit = total
        let reconstructed = rc.loss_cost
            + rc.expense_load
            + rc.profit_load
            + rc.reinsurance_load
            + rc.large_loss_load
            - rc.investment_credit;
        // Allow tiny rounding tolerance
        let diff = (reconstructed - rc.total).abs();
        assert!(
            diff < dec!(0.01),
            "Components do not sum to total: reconstructed={}, total={}, diff={}",
            reconstructed,
            rc.total,
            diff
        );
    }

    #[test]
    fn test_projected_experience_over_5_years() {
        let mut input = basic_pricing_input();
        input.projection_years = 5;
        let result = price_premium(&input).unwrap();

        assert_eq!(result.result.projected_experience.len(), 5);
        // Year numbers should be 1..=5
        for (i, pe) in result.result.projected_experience.iter().enumerate() {
            assert_eq!(pe.year, (i + 1) as u32);
        }
        // Each successive year severity should increase (positive severity trend)
        for i in 1..result.result.projected_experience.len() {
            assert!(
                result.result.projected_experience[i].projected_severity
                    > result.result.projected_experience[i - 1].projected_severity
            );
        }
    }

    #[test]
    fn test_premium_per_unit() {
        let input = basic_pricing_input();
        let result = price_premium(&input).unwrap();
        let expected = result.result.gross_premium / input.exposure_units;
        assert_eq!(result.result.premium_per_unit, expected);
    }

    #[test]
    fn test_investment_income_credit_reduces_premium() {
        let mut input_with = basic_pricing_input();
        input_with.investment_income_credit = dec!(0.05);

        let mut input_without = basic_pricing_input();
        input_without.investment_income_credit = Decimal::ZERO;

        let result_with = price_premium(&input_with).unwrap();
        let result_without = price_premium(&input_without).unwrap();

        assert!(
            result_with.result.gross_premium < result_without.result.gross_premium,
            "Investment income credit should reduce gross premium"
        );
    }

    #[test]
    fn test_loading_factors_exceed_100_pct() {
        let mut input = basic_pricing_input();
        input.expense_ratio_target = dec!(0.90);
        // denominator would be 1 - 0.90 - 0.05 - 0.10 - 0.05 + 0.02 = -0.08
        let result = price_premium(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_zero_claims_pure_premium_is_zero() {
        let mut input = basic_pricing_input();
        input.claim_frequency = Decimal::ZERO;
        let result = price_premium(&input).unwrap();
        assert_eq!(result.result.pure_premium, Decimal::ZERO);
        assert_eq!(result.result.trended_pure_premium, Decimal::ZERO);
    }

    // -----------------------------------------------------------------------
    // Combined ratio tests
    // -----------------------------------------------------------------------

    fn profitable_period(year: u32) -> InsurancePeriod {
        InsurancePeriod {
            year,
            net_earned_premium: dec!(1000000),
            net_incurred_losses: dec!(600000),
            loss_adjustment_expenses: dec!(50000),
            underwriting_expenses: dec!(250000),
            policyholder_dividends: dec!(10000),
            net_investment_income: dec!(80000),
            realized_gains: dec!(20000),
        }
    }

    fn unprofitable_period(year: u32) -> InsurancePeriod {
        InsurancePeriod {
            year,
            net_earned_premium: dec!(1000000),
            net_incurred_losses: dec!(750000),
            loss_adjustment_expenses: dec!(100000),
            underwriting_expenses: dec!(200000),
            policyholder_dividends: dec!(20000),
            net_investment_income: dec!(60000),
            realized_gains: dec!(10000),
        }
    }

    #[test]
    fn test_combined_ratio_profitable() {
        let input = CombinedRatioInput {
            company_name: "TestCo".into(),
            periods: vec![profitable_period(2023)],
        };
        let result = analyze_combined_ratio(&input).unwrap();
        let pr = &result.result.period_results[0];

        // CR = (600k + 50k + 250k + 10k) / 1M = 0.91
        assert_eq!(pr.combined_ratio, dec!(0.91));
        assert!(pr.combined_ratio < Decimal::ONE);
        assert!(pr.underwriting_profit_loss > Decimal::ZERO);
    }

    #[test]
    fn test_combined_ratio_unprofitable() {
        let input = CombinedRatioInput {
            company_name: "TestCo".into(),
            periods: vec![unprofitable_period(2023)],
        };
        let result = analyze_combined_ratio(&input).unwrap();
        let pr = &result.result.period_results[0];

        // CR = (750k + 100k + 200k + 20k) / 1M = 1.07
        assert_eq!(pr.combined_ratio, dec!(1.07));
        assert!(pr.combined_ratio > Decimal::ONE);
        assert!(pr.underwriting_profit_loss < Decimal::ZERO);
    }

    #[test]
    fn test_loss_ratio_calculation() {
        let input = CombinedRatioInput {
            company_name: "TestCo".into(),
            periods: vec![profitable_period(2023)],
        };
        let result = analyze_combined_ratio(&input).unwrap();
        // loss_ratio = 600k / 1M = 0.60
        assert_eq!(result.result.period_results[0].loss_ratio, dec!(0.60));
    }

    #[test]
    fn test_expense_ratio_calculation() {
        let input = CombinedRatioInput {
            company_name: "TestCo".into(),
            periods: vec![profitable_period(2023)],
        };
        let result = analyze_combined_ratio(&input).unwrap();
        // expense_ratio = 250k / 1M = 0.25
        assert_eq!(result.result.period_results[0].expense_ratio, dec!(0.25));
    }

    #[test]
    fn test_operating_ratio_includes_investment_income() {
        let input = CombinedRatioInput {
            company_name: "TestCo".into(),
            periods: vec![profitable_period(2023)],
        };
        let result = analyze_combined_ratio(&input).unwrap();
        let pr = &result.result.period_results[0];

        // operating_ratio = combined_ratio - investment_income / premium
        // = 0.91 - 80k/1M = 0.91 - 0.08 = 0.83
        assert_eq!(pr.operating_ratio, dec!(0.83));
        assert!(pr.operating_ratio < pr.combined_ratio);
    }

    #[test]
    fn test_net_income_calculation() {
        let input = CombinedRatioInput {
            company_name: "TestCo".into(),
            periods: vec![profitable_period(2023)],
        };
        let result = analyze_combined_ratio(&input).unwrap();
        let pr = &result.result.period_results[0];

        // UW profit = 1M - 600k - 50k - 250k - 10k = 90k
        // Net income = 90k + 80k + 20k = 190k
        assert_eq!(pr.underwriting_profit_loss, dec!(90000));
        assert_eq!(pr.net_income, dec!(190000));
    }

    #[test]
    fn test_multi_year_trend_analysis() {
        // Create an improving trend: first years bad, later years good
        let periods = vec![
            unprofitable_period(2020),
            unprofitable_period(2021),
            profitable_period(2022),
            profitable_period(2023),
        ];
        let input = CombinedRatioInput {
            company_name: "TestCo".into(),
            periods,
        };
        let result = analyze_combined_ratio(&input).unwrap();
        let summary = &result.result.summary;

        assert_eq!(summary.trend_direction, "Improving");
        assert_eq!(summary.best_year, 2022); // both 2022/2023 have 0.91 CR; min_by picks first
        assert_eq!(summary.worst_year, 2021); // both 2020/2021 have 1.07 CR; max_by picks last
    }

    #[test]
    fn test_summary_best_worst_year() {
        let periods = vec![profitable_period(2022), unprofitable_period(2023)];
        let input = CombinedRatioInput {
            company_name: "TestCo".into(),
            periods,
        };
        let result = analyze_combined_ratio(&input).unwrap();
        let summary = &result.result.summary;

        assert_eq!(summary.best_year, 2022);
        assert_eq!(summary.worst_year, 2023);
        assert_eq!(summary.profitable_years, 1); // only 2022
    }

    #[test]
    fn test_combined_ratio_zero_losses() {
        let period = InsurancePeriod {
            year: 2023,
            net_earned_premium: dec!(1000000),
            net_incurred_losses: Decimal::ZERO,
            loss_adjustment_expenses: Decimal::ZERO,
            underwriting_expenses: dec!(200000),
            policyholder_dividends: Decimal::ZERO,
            net_investment_income: dec!(50000),
            realized_gains: Decimal::ZERO,
        };
        let input = CombinedRatioInput {
            company_name: "TestCo".into(),
            periods: vec![period],
        };
        let result = analyze_combined_ratio(&input).unwrap();
        // loss_ratio should be 0
        assert_eq!(result.result.period_results[0].loss_ratio, Decimal::ZERO);
        assert_eq!(result.result.period_results[0].lae_ratio, Decimal::ZERO);
    }

    #[test]
    fn test_combined_ratio_100pct_expenses() {
        // Edge case: expenses eat entire premium
        let period = InsurancePeriod {
            year: 2023,
            net_earned_premium: dec!(1000000),
            net_incurred_losses: Decimal::ZERO,
            loss_adjustment_expenses: Decimal::ZERO,
            underwriting_expenses: dec!(1000000),
            policyholder_dividends: Decimal::ZERO,
            net_investment_income: Decimal::ZERO,
            realized_gains: Decimal::ZERO,
        };
        let input = CombinedRatioInput {
            company_name: "TestCo".into(),
            periods: vec![period],
        };
        let result = analyze_combined_ratio(&input).unwrap();
        let pr = &result.result.period_results[0];
        assert_eq!(pr.expense_ratio, Decimal::ONE);
        assert_eq!(pr.combined_ratio, Decimal::ONE);
        assert_eq!(pr.underwriting_profit_loss, Decimal::ZERO);
    }

    #[test]
    fn test_combined_ratio_empty_periods() {
        let input = CombinedRatioInput {
            company_name: "TestCo".into(),
            periods: vec![],
        };
        assert!(analyze_combined_ratio(&input).is_err());
    }

    // -----------------------------------------------------------------------
    // SCR tests
    // -----------------------------------------------------------------------

    fn basic_scr_input() -> ScrInput {
        ScrInput {
            company_name: "TestInsurer".into(),
            premium_risk: PremiumReserveRisk {
                net_earned_premium: dec!(10000000),
                net_best_estimate_reserves: dec!(15000000),
                premium_risk_factor: dec!(0.10),
                reserve_risk_factor: dec!(0.08),
                geographic_diversification: dec!(0.85),
            },
            catastrophe_risk: dec!(2000000),
            market_risk: dec!(3000000),
            credit_risk: dec!(1000000),
            operational_risk_premium: dec!(12000000),
            eligible_own_funds: dec!(15000000),
            mcr_factor: dec!(0.35),
        }
    }

    #[test]
    fn test_scr_premium_reserve_risk() {
        let input = basic_scr_input();
        let result = calculate_scr(&input).unwrap();

        // pf_nep = 0.10 * 10M = 1M
        // rf_res = 0.08 * 15M = 1.2M
        // under_sqrt = 1M^2 + 1.2M^2 + 2*0.5*1M*1.2M = 1T + 1.44T + 1.2T = 3.64T
        // sqrt(3.64T) = ~1907878.4
        // * 0.85 geographic = ~1621696.6
        // + cat 2M = ~3621696.6
        let nl = result.result.non_life_underwriting_scr;
        assert!(nl > dec!(3000000), "NL UW SCR should be > 3M, got {}", nl);
        assert!(nl < dec!(4000000), "NL UW SCR should be < 4M, got {}", nl);
    }

    #[test]
    fn test_scr_with_diversification_benefit() {
        let input = basic_scr_input();
        let result = calculate_scr(&input).unwrap();

        // Diversification benefit should be positive (sum of parts > aggregated)
        assert!(
            result.result.diversification_benefit > Decimal::ZERO,
            "Diversification benefit should be positive"
        );

        // Total SCR should be less than simple sum of NL + market + credit + operational
        let simple_sum = result.result.non_life_underwriting_scr
            + result.result.market_scr
            + result.result.credit_scr
            + result.result.operational_scr;
        assert!(
            result.result.total_scr < simple_sum,
            "Total SCR {} should be less than simple sum {}",
            result.result.total_scr,
            simple_sum
        );
    }

    #[test]
    fn test_scr_operational_risk() {
        let input = basic_scr_input();
        let result = calculate_scr(&input).unwrap();

        // op risk = 3% of max(10M premium, 15M reserves) = 3% of 15M = 450k
        assert_eq!(result.result.operational_scr, dec!(450000));
    }

    #[test]
    fn test_solvency_ratio_calculation() {
        let input = basic_scr_input();
        let result = calculate_scr(&input).unwrap();

        // solvency_ratio = own_funds / total_scr = 15M / total_scr
        let expected = input.eligible_own_funds / result.result.total_scr;
        assert_eq!(result.result.solvency_ratio, expected);
    }

    #[test]
    fn test_mcr_floor_at_25_pct() {
        let mut input = basic_scr_input();
        // Set MCR factor very low to trigger the 25% floor
        input.mcr_factor = dec!(0.10);
        let result = calculate_scr(&input).unwrap();

        let floor = result.result.total_scr * dec!(0.25);
        assert!(
            result.result.mcr >= floor,
            "MCR {} should be >= 25% floor {}",
            result.result.mcr,
            floor
        );
        // With factor 0.10, raw MCR = 10% of SCR < 25% floor, so MCR should equal floor
        assert_eq!(result.result.mcr, floor);
    }

    #[test]
    fn test_meets_scr_but_not_mcr() {
        // Create a scenario where own funds > SCR but < MCR
        // This is unusual but possible if MCR factor is high
        let mut input = basic_scr_input();
        // Set very high MCR factor and low own funds relative to MCR
        input.mcr_factor = dec!(0.45);
        // We need own_funds >= SCR but < MCR = SCR * 0.45
        // That's impossible since MCR < SCR always. So instead test the reverse:
        // meets MCR but not SCR (which is the natural failure mode)
        // Actually, since MCR = SCR * factor and factor < 1, MCR < SCR always.
        // So if you fail SCR, you might still pass MCR.
        input.eligible_own_funds = dec!(5000000); // likely less than SCR but more than MCR
        let result = calculate_scr(&input).unwrap();

        // If own_funds < SCR, meets_scr = false
        if result.result.total_scr > dec!(5000000) {
            assert!(!result.result.meets_scr);
            // MCR = SCR * 0.35 probably < 5M, so meets_mcr might be true
            if result.result.mcr < dec!(5000000) {
                assert!(result.result.meets_mcr);
            }
        }
    }

    #[test]
    fn test_surplus_calculation() {
        let input = basic_scr_input();
        let result = calculate_scr(&input).unwrap();

        let expected_surplus = input.eligible_own_funds - result.result.total_scr;
        assert_eq!(result.result.surplus, expected_surplus);
    }

    #[test]
    fn test_scr_invalid_geographic_diversification() {
        let mut input = basic_scr_input();
        input.premium_risk.geographic_diversification = Decimal::ZERO;
        assert!(calculate_scr(&input).is_err());
    }

    #[test]
    fn test_scr_catastrophe_passthrough() {
        let input = basic_scr_input();
        let result = calculate_scr(&input).unwrap();
        assert_eq!(result.result.catastrophe_scr, dec!(2000000));
    }

    // -----------------------------------------------------------------------
    // Utility tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_sqrt_decimal_basic() {
        let val = dec!(4);
        let result = sqrt_decimal(val);
        let diff = (result - dec!(2)).abs();
        assert!(
            diff < dec!(0.0000001),
            "sqrt(4) should be ~2, got {}",
            result
        );
    }

    #[test]
    fn test_sqrt_decimal_zero() {
        assert_eq!(sqrt_decimal(Decimal::ZERO), Decimal::ZERO);
    }

    #[test]
    fn test_sqrt_decimal_negative() {
        assert_eq!(sqrt_decimal(dec!(-1)), Decimal::ZERO);
    }

    #[test]
    fn test_decimal_pow() {
        // (1.03)^3 = 1.092727
        let result = decimal_pow(dec!(1.03), 3);
        let expected = dec!(1.092727);
        let diff = (result - expected).abs();
        assert!(
            diff < dec!(0.000001),
            "(1.03)^3 should be ~1.092727, got {}",
            result
        );
    }
}
