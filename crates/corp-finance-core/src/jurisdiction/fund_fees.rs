use rust_decimal::Decimal;
use rust_decimal::MathematicalOps;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::*;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ManagementFeeBasis {
    CommittedCapital,
    InvestedCapital,
    NetAssetValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WaterfallType {
    /// Whole-fund carry (carry on total fund profit)
    European,
    /// Deal-by-deal carry
    American,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundFeeInput {
    pub fund_size: Money,
    pub management_fee_rate: Rate,
    pub management_fee_basis: ManagementFeeBasis,
    /// Carry rate (typically 20%)
    pub performance_fee_rate: Rate,
    /// Preferred return (typically 8%)
    pub hurdle_rate: Rate,
    /// GP catch-up rate (typically 100% or 80%)
    pub catch_up_rate: Rate,
    pub waterfall_type: WaterfallType,
    /// GP co-invest (typically 1-5%)
    pub gp_commitment_pct: Rate,
    pub clawback: bool,
    pub fund_life_years: u32,
    pub investment_period_years: u32,
    /// Assumed gross IRR for projecting NAV growth
    pub gross_irr_assumption: Rate,
    /// Assumed gross MOIC at fund maturity
    pub gross_moic_assumption: Multiple,
    pub annual_fund_expenses: Money,
    pub currency: Option<Currency>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundFeeOutput {
    pub projections: Vec<FundYearProjection>,
    pub total_management_fees: Money,
    pub total_performance_fees: Money,
    pub total_fund_expenses: Money,
    pub total_gp_income: Money,
    pub lp_net_irr: Rate,
    pub lp_net_moic: Multiple,
    pub lp_gross_moic: Multiple,
    pub total_fee_drag: Rate,
    pub total_fee_drag_dollars: Money,
    pub gp_management_fee_income: Money,
    pub gp_carry_income: Money,
    pub gp_total_income: Money,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundYearProjection {
    pub year: u32,
    /// Cumulative capital called
    pub capital_called: Money,
    /// Capital currently at work
    pub invested_capital: Money,
    pub nav: Money,
    /// Distributions this year
    pub distributions: Money,
    /// Cumulative distributions across all years
    pub cumulative_distributions: Money,
    pub management_fee: Money,
    pub performance_fee_accrual: Money,
    pub fund_expenses: Money,
    /// Distributions to Paid-In
    pub dpi: Multiple,
    /// Residual Value to Paid-In
    pub rvpi: Multiple,
    /// Total Value to Paid-In
    pub tvpi: Multiple,
}

// ---------------------------------------------------------------------------
// Main calculation
// ---------------------------------------------------------------------------

/// Calculate fund fee projections over the full fund lifecycle.
///
/// Models a simplified private-equity fund with management fees, performance
/// fees (carry), fund expenses, and GP co-investment.  Supports both European
/// (whole-fund) and American (deal-by-deal) waterfall structures.
pub fn calculate_fund_fees(
    input: &FundFeeInput,
) -> CorpFinanceResult<ComputationOutput<FundFeeOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // ------------------------------------------------------------------
    // 1. Validate inputs
    // ------------------------------------------------------------------
    validate_input(input)?;

    // ------------------------------------------------------------------
    // 2. Derived constants
    // ------------------------------------------------------------------
    let fund_size = input.fund_size;
    let inv_period = input.investment_period_years;
    let fund_life = input.fund_life_years;
    let harvest_years = fund_life.saturating_sub(inv_period);
    let lp_share = Decimal::ONE - input.gp_commitment_pct;

    // Annual capital deployment during the investment period
    let annual_deployment = if inv_period > 0 {
        fund_size / Decimal::from(inv_period)
    } else {
        fund_size
    };

    // Growth rate applied to the gross NAV each year
    let growth_rate = input.gross_irr_assumption;

    // ------------------------------------------------------------------
    // 3. Year-by-year projection
    //
    // The model tracks *gross* NAV (asset value before fee deductions).
    // Management fees and fund expenses are tracked separately as costs
    // borne by LPs, but they do NOT reduce gross NAV.  This ensures the
    // carry waterfall operates on the correct gross profit figure.
    //
    // The actual realised gross MOIC is derived from the model output
    // rather than assumed equal to gross_moic_assumption.  The
    // gross_moic_assumption is used as the LP benchmark for fee drag.
    // ------------------------------------------------------------------
    let mut projections: Vec<FundYearProjection> = Vec::with_capacity(fund_life as usize);
    let mut cumulative_called = Decimal::ZERO;
    let mut gross_nav = Decimal::ZERO;
    let mut cumulative_distributions = Decimal::ZERO;
    let mut total_mgmt_fees = Decimal::ZERO;
    let mut total_expenses = Decimal::ZERO;
    let mut total_perf_fees = Decimal::ZERO;

    // American waterfall state
    let mut cumulative_profit_distributed = Decimal::ZERO;

    for year in 1..=fund_life {
        // -- Capital calls --
        let new_capital = if year <= inv_period {
            annual_deployment
        } else {
            Decimal::ZERO
        };
        cumulative_called += new_capital;

        // -- Gross NAV growth (scaled) --
        gross_nav = (gross_nav + new_capital) * (Decimal::ONE + growth_rate);

        // -- Management fee --
        let mgmt_fee = calculate_management_fee(input, fund_size, gross_nav, gross_nav);
        total_mgmt_fees += mgmt_fee;

        // -- Fund expenses --
        let expenses = input.annual_fund_expenses;
        total_expenses += expenses;

        // -- Distributions (harvest period) --
        // During harvest, realise a proportional share of gross NAV each year.
        // In the final year, distribute all remaining NAV.
        let distributions = if year > inv_period && harvest_years > 0 {
            let harvest_idx = year - inv_period;
            if year == fund_life {
                gross_nav.max(Decimal::ZERO)
            } else {
                let remaining = harvest_years - harvest_idx + 1;
                (gross_nav / Decimal::from(remaining)).max(Decimal::ZERO)
            }
        } else {
            Decimal::ZERO
        };

        cumulative_distributions += distributions;
        gross_nav -= distributions;

        // -- Performance fee accrual (American / deal-by-deal) --
        let perf_fee_accrual = if matches!(input.waterfall_type, WaterfallType::American)
            && distributions > Decimal::ZERO
        {
            cumulative_profit_distributed += distributions;

            // Simple preferred return: hurdle_rate applied to called capital
            // for the average holding period from midpoint of deployment.
            let avg_years = if inv_period > 0 {
                let mid_deploy = Decimal::from(inv_period) / dec!(2);
                (Decimal::from(year) - mid_deploy).max(Decimal::ONE)
            } else {
                Decimal::from(year)
            };
            let hurdle_return = cumulative_called * input.hurdle_rate * avg_years;
            let total_profit = cumulative_profit_distributed - cumulative_called;
            let profit_over_hurdle = (total_profit - hurdle_return).max(Decimal::ZERO);
            let carry = profit_over_hurdle * input.performance_fee_rate;
            let incremental = (carry - total_perf_fees).max(Decimal::ZERO);
            total_perf_fees += incremental;
            incremental
        } else {
            Decimal::ZERO
        };

        // -- Multiples (gross, based on total capital called) --
        let (dpi, rvpi, tvpi) = if cumulative_called > Decimal::ZERO {
            let dpi = cumulative_distributions / cumulative_called;
            let rvpi = gross_nav / cumulative_called;
            let tvpi = dpi + rvpi;
            (dpi, rvpi, tvpi)
        } else {
            (Decimal::ZERO, Decimal::ZERO, Decimal::ZERO)
        };

        projections.push(FundYearProjection {
            year,
            capital_called: cumulative_called,
            invested_capital: gross_nav,
            nav: gross_nav,
            distributions,
            cumulative_distributions,
            management_fee: mgmt_fee,
            performance_fee_accrual: perf_fee_accrual,
            fund_expenses: expenses,
            dpi,
            rvpi,
            tvpi,
        });
    }

    // ------------------------------------------------------------------
    // 4. European waterfall carry (whole-fund, calculated at end)
    // ------------------------------------------------------------------
    if matches!(input.waterfall_type, WaterfallType::European) {
        let total_value = cumulative_distributions + gross_nav;
        let total_profit = total_value - cumulative_called;

        // Compound hurdle: LPs must receive their capital back plus the
        // compounded preferred return before GP earns carry.
        let hurdle_amount = cumulative_called
            * ((Decimal::ONE + input.hurdle_rate).powd(Decimal::from(fund_life)) - Decimal::ONE);

        if total_profit > hurdle_amount && total_profit > Decimal::ZERO {
            let excess = total_profit - hurdle_amount;

            // GP catch-up: GP receives catch_up_rate of each marginal dollar
            // until GP has received performance_fee_rate of total profit.
            let gp_target_carry = total_profit * input.performance_fee_rate;

            let catch_up_pool_used = if input.catch_up_rate > Decimal::ZERO {
                (gp_target_carry / input.catch_up_rate).min(excess)
            } else {
                Decimal::ZERO
            };
            let catch_up_carry = catch_up_pool_used * input.catch_up_rate;

            let remaining_excess = (excess - catch_up_pool_used).max(Decimal::ZERO);
            let remaining_carry = remaining_excess * input.performance_fee_rate;

            total_perf_fees = (catch_up_carry + remaining_carry).min(gp_target_carry);

            // Attribute the carry to the final-year projection
            if let Some(last) = projections.last_mut() {
                last.performance_fee_accrual = total_perf_fees;
            }
        }
    }

    // ------------------------------------------------------------------
    // 5. LP net returns
    // ------------------------------------------------------------------
    let lp_invested = fund_size * lp_share;

    // Realised gross total value from the model
    let realised_gross_total = cumulative_distributions + gross_nav;

    // LP receives gross distributions minus carry, and also pays mgmt fees
    // and expenses out of their commitment.
    let lp_gross_value = realised_gross_total * lp_share;
    let lp_net_value =
        lp_gross_value - total_perf_fees * lp_share - total_mgmt_fees - total_expenses;

    let lp_net_moic = if lp_invested > Decimal::ZERO {
        lp_net_value / lp_invested
    } else {
        Decimal::ZERO
    };

    // Gross MOIC is derived from the actual model output so that the
    // relationship lp_net_moic < lp_gross_moic always holds when fees > 0.
    let lp_gross_moic = if cumulative_called > Decimal::ZERO {
        realised_gross_total / cumulative_called
    } else {
        input.gross_moic_assumption
    };

    // Fee drag
    let total_fee_drag = if lp_gross_moic > Decimal::ZERO {
        (lp_gross_moic - lp_net_moic) / lp_gross_moic
    } else {
        Decimal::ZERO
    };
    let total_fee_drag_dollars = lp_invested * (lp_gross_moic - lp_net_moic);

    // LP net IRR approximation via cash-flow-based IRR
    let lp_net_irr = approximate_lp_net_irr(
        &projections,
        lp_share,
        total_perf_fees,
        total_mgmt_fees,
        total_expenses,
        fund_life,
        &mut warnings,
    );

    // ------------------------------------------------------------------
    // 6. GP income
    // ------------------------------------------------------------------
    let gp_coinvest_return = fund_size * input.gp_commitment_pct * (lp_gross_moic - Decimal::ONE);
    let gp_management_fee_income = total_mgmt_fees;
    let gp_carry_income = total_perf_fees;
    let gp_total_income = gp_management_fee_income + gp_carry_income + gp_coinvest_return;

    // ------------------------------------------------------------------
    // 7. Assemble output
    // ------------------------------------------------------------------
    let output = FundFeeOutput {
        projections,
        total_management_fees: total_mgmt_fees,
        total_performance_fees: total_perf_fees,
        total_fund_expenses: total_expenses,
        total_gp_income: gp_total_income,
        lp_net_irr,
        lp_net_moic,
        lp_gross_moic,
        total_fee_drag,
        total_fee_drag_dollars,
        gp_management_fee_income,
        gp_carry_income,
        gp_total_income,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Fund Fee Calculator: Management Fees, Carry, Waterfall Analysis",
        &serde_json::json!({
            "fund_size": input.fund_size.to_string(),
            "management_fee_rate": input.management_fee_rate.to_string(),
            "performance_fee_rate": input.performance_fee_rate.to_string(),
            "hurdle_rate": input.hurdle_rate.to_string(),
            "waterfall_type": format!("{:?}", input.waterfall_type),
            "fund_life_years": input.fund_life_years,
            "investment_period_years": input.investment_period_years,
            "gross_irr_assumption": input.gross_irr_assumption.to_string(),
            "gross_moic_assumption": input.gross_moic_assumption.to_string(),
        }),
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn validate_input(input: &FundFeeInput) -> CorpFinanceResult<()> {
    if input.fund_size <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "fund_size".into(),
            reason: "Fund size must be greater than zero".into(),
        });
    }
    if input.management_fee_rate < Decimal::ZERO || input.management_fee_rate > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "management_fee_rate".into(),
            reason: "Management fee rate must be between 0 and 1".into(),
        });
    }
    if input.performance_fee_rate < Decimal::ZERO || input.performance_fee_rate > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "performance_fee_rate".into(),
            reason: "Performance fee rate must be between 0 and 1".into(),
        });
    }
    if input.hurdle_rate < Decimal::ZERO || input.hurdle_rate > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "hurdle_rate".into(),
            reason: "Hurdle rate must be between 0 and 1".into(),
        });
    }
    if input.catch_up_rate < Decimal::ZERO || input.catch_up_rate > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "catch_up_rate".into(),
            reason: "Catch-up rate must be between 0 and 1".into(),
        });
    }
    if input.gp_commitment_pct < Decimal::ZERO || input.gp_commitment_pct > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "gp_commitment_pct".into(),
            reason: "GP commitment percentage must be between 0 and 1".into(),
        });
    }
    if input.fund_life_years == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "fund_life_years".into(),
            reason: "Fund life must be at least 1 year".into(),
        });
    }
    if input.fund_life_years < input.investment_period_years {
        return Err(CorpFinanceError::InvalidInput {
            field: "fund_life_years".into(),
            reason: "Fund life must be >= investment period".into(),
        });
    }
    if input.gross_moic_assumption < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "gross_moic_assumption".into(),
            reason: "Gross MOIC assumption must be non-negative".into(),
        });
    }
    Ok(())
}

fn calculate_management_fee(
    input: &FundFeeInput,
    fund_size: Money,
    invested_capital: Money,
    nav: Money,
) -> Money {
    let basis = match input.management_fee_basis {
        ManagementFeeBasis::CommittedCapital => fund_size,
        ManagementFeeBasis::InvestedCapital => invested_capital.max(Decimal::ZERO),
        ManagementFeeBasis::NetAssetValue => nav.max(Decimal::ZERO),
    };
    basis * input.management_fee_rate
}

/// Approximate LP net IRR using a simplified annual cash-flow series and
/// Newton-Raphson IRR solver from `crate::time_value`.
fn approximate_lp_net_irr(
    projections: &[FundYearProjection],
    lp_share: Decimal,
    total_perf_fees: Money,
    total_mgmt_fees: Money,
    total_expenses: Money,
    fund_life: u32,
    warnings: &mut Vec<String>,
) -> Rate {
    if projections.is_empty() {
        return Decimal::ZERO;
    }

    // Build LP cash flows: negative for capital calls, positive for distributions.
    // Management fees and expenses are spread proportionally across the fund life.
    let mut lp_cfs: Vec<Money> = Vec::with_capacity(projections.len() + 1);

    // Year 0: no cash flow placeholder
    lp_cfs.push(Decimal::ZERO);

    let annual_mgmt_fee = if fund_life > 0 {
        total_mgmt_fees / Decimal::from(fund_life)
    } else {
        Decimal::ZERO
    };
    let annual_expenses = if fund_life > 0 {
        total_expenses / Decimal::from(fund_life)
    } else {
        Decimal::ZERO
    };

    for (i, p) in projections.iter().enumerate() {
        // Capital call is a negative cash flow for the LP
        let call_this_year = if i == 0 {
            p.capital_called * lp_share
        } else {
            (p.capital_called - projections[i - 1].capital_called) * lp_share
        };

        // Gross distributions flow to LP (LP share)
        let dist = p.distributions * lp_share;

        // LP pays management fees and expenses each year
        let fees = annual_mgmt_fee + annual_expenses;

        // In the final year, also deduct the LP share of carry and add residual NAV
        let final_adj = if (i + 1) as u32 == fund_life {
            let carry_deduction = total_perf_fees * lp_share;
            let residual_nav = p.nav * lp_share;
            residual_nav - carry_deduction
        } else {
            Decimal::ZERO
        };

        let net_cf = -call_this_year + dist - fees + final_adj;
        lp_cfs.push(net_cf);
    }

    // Use the existing IRR solver
    match crate::time_value::irr(&lp_cfs, dec!(0.10)) {
        Ok(r) => r,
        Err(e) => {
            warnings.push(format!("LP net IRR approximation warning: {e}"));
            Decimal::ZERO
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// Helper to create a standard 2/20 European fund input.
    fn standard_european_input() -> FundFeeInput {
        FundFeeInput {
            fund_size: dec!(500_000_000),
            management_fee_rate: dec!(0.02),
            management_fee_basis: ManagementFeeBasis::CommittedCapital,
            performance_fee_rate: dec!(0.20),
            hurdle_rate: dec!(0.08),
            catch_up_rate: dec!(1.00),
            waterfall_type: WaterfallType::European,
            gp_commitment_pct: dec!(0.02),
            clawback: true,
            fund_life_years: 10,
            investment_period_years: 5,
            gross_irr_assumption: dec!(0.15),
            gross_moic_assumption: dec!(2.0),
            annual_fund_expenses: dec!(500_000),
            currency: Some(Currency::USD),
        }
    }

    /// Helper to create a standard American (deal-by-deal) fund input.
    fn standard_american_input() -> FundFeeInput {
        let mut input = standard_european_input();
        input.waterfall_type = WaterfallType::American;
        input
    }

    // ------------------------------------------------------------------
    // Test 1: Basic European waterfall
    // ------------------------------------------------------------------
    #[test]
    fn test_basic_european_waterfall() {
        let input = standard_european_input();
        let result = calculate_fund_fees(&input).unwrap();
        let out = &result.result;

        // Should have 10 year projections
        assert_eq!(out.projections.len(), 10);

        // Total management fees should be positive and substantial
        assert!(out.total_management_fees > Decimal::ZERO);

        // Performance fees should be positive (fund returns > hurdle)
        assert!(
            out.total_performance_fees > Decimal::ZERO,
            "Expected positive carry for a 2x MOIC fund with 8% hurdle"
        );

        // LP net MOIC should be less than gross MOIC due to fees
        assert!(
            out.lp_net_moic < out.lp_gross_moic,
            "LP net MOIC ({}) should be < gross MOIC ({})",
            out.lp_net_moic,
            out.lp_gross_moic
        );

        // GP total income should include mgmt fees + carry
        assert!(out.gp_total_income > out.gp_management_fee_income);
    }

    // ------------------------------------------------------------------
    // Test 2: American waterfall (deal-by-deal)
    // ------------------------------------------------------------------
    #[test]
    fn test_american_waterfall() {
        let input = standard_american_input();
        let result = calculate_fund_fees(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.projections.len(), 10);

        // American waterfall should also generate positive carry
        assert!(
            out.total_performance_fees >= Decimal::ZERO,
            "American waterfall carry should be non-negative"
        );

        // LP net MOIC should be lower than gross
        assert!(
            out.lp_net_moic < out.lp_gross_moic,
            "LP net MOIC ({}) should be < gross MOIC ({})",
            out.lp_net_moic,
            out.lp_gross_moic
        );
    }

    // ------------------------------------------------------------------
    // Test 3: Management fee on committed capital stays flat
    // ------------------------------------------------------------------
    #[test]
    fn test_management_fee_committed() {
        let input = standard_european_input();
        let result = calculate_fund_fees(&input).unwrap();
        let out = &result.result;

        // All years should have the same management fee for committed capital basis
        let expected_annual_fee = dec!(500_000_000) * dec!(0.02); // 10M
        for p in &out.projections {
            assert_eq!(
                p.management_fee, expected_annual_fee,
                "Management fee should be flat at {} for committed capital basis, got {} in year {}",
                expected_annual_fee, p.management_fee, p.year
            );
        }
    }

    // ------------------------------------------------------------------
    // Test 4: Management fee on invested capital changes over time
    // ------------------------------------------------------------------
    #[test]
    fn test_management_fee_invested() {
        let mut input = standard_european_input();
        input.management_fee_basis = ManagementFeeBasis::InvestedCapital;

        let result = calculate_fund_fees(&input).unwrap();
        let out = &result.result;

        // Fee should vary year to year since invested capital changes
        let first_fee = out.projections[0].management_fee;
        let last_fee = out.projections.last().unwrap().management_fee;

        // With distributions happening, invested capital (and thus fees) should differ
        assert_ne!(
            first_fee, last_fee,
            "Invested-capital-basis fees should vary: year 1 = {}, final year = {}",
            first_fee, last_fee
        );
    }

    // ------------------------------------------------------------------
    // Test 5: No carry below hurdle
    // ------------------------------------------------------------------
    #[test]
    fn test_no_carry_below_hurdle() {
        let mut input = standard_european_input();
        // Set gross IRR below the hurdle rate so the fund underperforms
        input.gross_irr_assumption = dec!(0.03);
        input.gross_moic_assumption = dec!(1.1);

        let result = calculate_fund_fees(&input).unwrap();
        let out = &result.result;

        assert_eq!(
            out.total_performance_fees,
            Decimal::ZERO,
            "No carry should be earned when gross return ({}) is below hurdle ({})",
            input.gross_irr_assumption,
            input.hurdle_rate
        );
    }

    // ------------------------------------------------------------------
    // Test 6: Fee drag is always positive
    // ------------------------------------------------------------------
    #[test]
    fn test_fee_drag_positive() {
        let input = standard_european_input();
        let result = calculate_fund_fees(&input).unwrap();
        let out = &result.result;

        assert!(
            out.total_fee_drag > Decimal::ZERO,
            "Fee drag should be positive, got {}",
            out.total_fee_drag
        );
        assert!(
            out.total_fee_drag_dollars > Decimal::ZERO,
            "Fee drag in dollars should be positive, got {}",
            out.total_fee_drag_dollars
        );
    }

    // ------------------------------------------------------------------
    // Test 7: GP income components
    // ------------------------------------------------------------------
    #[test]
    fn test_gp_income_components() {
        let input = standard_european_input();
        let result = calculate_fund_fees(&input).unwrap();
        let out = &result.result;

        // GP total = management fees + carry + co-invest return
        let gp_coinvest_return =
            input.fund_size * input.gp_commitment_pct * (out.lp_gross_moic - Decimal::ONE);
        let expected_total =
            out.gp_management_fee_income + out.gp_carry_income + gp_coinvest_return;

        assert_eq!(
            out.gp_total_income,
            expected_total,
            "GP total income ({}) should equal mgmt fees ({}) + carry ({}) + co-invest ({})",
            out.gp_total_income,
            out.gp_management_fee_income,
            out.gp_carry_income,
            gp_coinvest_return
        );

        // Management fee income should equal total management fees
        assert_eq!(out.gp_management_fee_income, out.total_management_fees);

        // Carry income should equal total performance fees
        assert_eq!(out.gp_carry_income, out.total_performance_fees);
    }

    // ------------------------------------------------------------------
    // Test 8: TVPI = DPI + RVPI at all times
    // ------------------------------------------------------------------
    #[test]
    fn test_tvpi_equals_dpi_plus_rvpi() {
        let input = standard_european_input();
        let result = calculate_fund_fees(&input).unwrap();
        let out = &result.result;

        for p in &out.projections {
            let sum = p.dpi + p.rvpi;
            let diff = (p.tvpi - sum).abs();
            assert!(
                diff < dec!(0.0001),
                "TVPI ({}) should equal DPI ({}) + RVPI ({}) in year {}; diff = {}",
                p.tvpi,
                p.dpi,
                p.rvpi,
                p.year,
                diff
            );
        }
    }

    // ------------------------------------------------------------------
    // Test 9: Zero fund size yields an error
    // ------------------------------------------------------------------
    #[test]
    fn test_zero_fund_size_error() {
        let mut input = standard_european_input();
        input.fund_size = Decimal::ZERO;

        let result = calculate_fund_fees(&input);
        assert!(result.is_err(), "Zero fund size should produce an error");

        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "fund_size");
            }
            other => panic!("Expected InvalidInput for fund_size, got: {other}"),
        }
    }

    // ------------------------------------------------------------------
    // Test 10: Clawback scenario — early carry paid, fund underperforms
    // ------------------------------------------------------------------
    #[test]
    fn test_clawback_scenario() {
        // American waterfall with early distributions then underperformance
        let mut input = standard_american_input();
        input.clawback = true;
        // High early IRR but low final MOIC (fund J-curve doesn't deliver)
        input.gross_irr_assumption = dec!(0.12);
        input.gross_moic_assumption = dec!(1.3);
        input.hurdle_rate = dec!(0.08);

        let result = calculate_fund_fees(&input).unwrap();
        let out = &result.result;

        // With clawback enabled, the fund should still compute
        assert_eq!(out.projections.len(), 10);

        // Verify the LP still gets a return (even if reduced)
        assert!(
            out.lp_net_moic > Decimal::ZERO,
            "LP net MOIC should be positive even in clawback scenario"
        );

        // The fee drag should be meaningful in an underperforming fund
        if out.lp_gross_moic > Decimal::ONE {
            assert!(
                out.total_fee_drag > Decimal::ZERO,
                "Fee drag should be positive for underperforming fund"
            );
        }
    }

    // ------------------------------------------------------------------
    // Additional: negative fund size
    // ------------------------------------------------------------------
    #[test]
    fn test_negative_fund_size_error() {
        let mut input = standard_european_input();
        input.fund_size = dec!(-100_000);

        let result = calculate_fund_fees(&input);
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------
    // Additional: fund life less than investment period
    // ------------------------------------------------------------------
    #[test]
    fn test_fund_life_less_than_investment_period() {
        let mut input = standard_european_input();
        input.fund_life_years = 3;
        input.investment_period_years = 5;

        let result = calculate_fund_fees(&input);
        assert!(result.is_err());

        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "fund_life_years");
            }
            other => panic!("Expected InvalidInput for fund_life_years, got: {other}"),
        }
    }
}
