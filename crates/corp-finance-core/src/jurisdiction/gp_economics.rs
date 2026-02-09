use rust_decimal::Decimal;
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
pub struct GpEconomicsInput {
    pub fund_size: Money,
    pub management_fee_rate: Rate,
    pub carried_interest_rate: Rate,
    pub hurdle_rate: Rate,
    pub gp_commitment_pct: Rate,
    pub fund_life_years: u32,
    pub investment_period_years: u32,
    pub num_investment_professionals: u32,
    pub annual_gp_overhead: Money,
    pub gross_irr_assumption: Rate,
    pub gross_moic_assumption: Multiple,
    /// Years with reduced/no management fee at fund inception
    pub fee_holiday_years: Option<u32>,
    /// Discount on management fee (e.g., for anchor LPs)
    pub fee_discount_rate: Option<Rate>,
    /// Year when successor fund starts charging fees (reduces this fund's fee)
    pub successor_fund_offset: Option<u32>,
    pub currency: Option<Currency>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpEconomicsOutput {
    pub total_management_fee_income: Money,
    pub total_carry_income: Money,
    pub total_gp_coinvest_return: Money,
    pub total_gp_revenue: Money,
    pub annual_management_fee_income: Money,
    pub carry_per_professional: Money,
    pub management_fee_per_professional: Money,
    pub total_per_professional: Money,
    pub breakeven_aum: Money,
    pub breakeven_fund_multiple: Multiple,
    pub gp_commitment_amount: Money,
    pub gp_commitment_return: Money,
    pub projections: Vec<GpYearProjection>,
    pub revenue_mix: RevenueMix,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpYearProjection {
    pub year: u32,
    pub management_fee: Money,
    pub carry_accrual: Money,
    pub coinvest_return: Money,
    pub total_gp_income: Money,
    pub cumulative_gp_income: Money,
    pub overhead: Money,
    pub net_gp_income: Money,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevenueMix {
    pub management_fee_pct: Rate,
    pub carry_pct: Rate,
    pub coinvest_pct: Rate,
}

// ---------------------------------------------------------------------------
// Main calculation
// ---------------------------------------------------------------------------

/// Calculate GP economics including management fee income, carried interest,
/// co-investment returns, breakeven metrics, and per-professional economics.
pub fn calculate_gp_economics(
    input: &GpEconomicsInput,
) -> CorpFinanceResult<ComputationOutput<GpEconomicsOutput>> {
    let start = Instant::now();
    let warnings: Vec<String> = Vec::new();

    // ------------------------------------------------------------------
    // 1. Validate inputs
    // ------------------------------------------------------------------
    validate_input(input)?;

    // ------------------------------------------------------------------
    // 2. Derived constants
    // ------------------------------------------------------------------
    let fund_size = input.fund_size;
    let fund_life = input.fund_life_years;
    let inv_period = input.investment_period_years;
    let realisation_years = fund_life.saturating_sub(inv_period);
    let num_pros = Decimal::from(input.num_investment_professionals);

    let fee_discount = input.fee_discount_rate.unwrap_or(Decimal::ZERO);
    let effective_fee_rate = input.management_fee_rate * (Decimal::ONE - fee_discount);
    let fee_holiday = input.fee_holiday_years.unwrap_or(0);
    let successor_offset = input.successor_fund_offset.unwrap_or(fund_life + 1);

    // GP co-investment
    let gp_commitment_amount = fund_size * input.gp_commitment_pct;
    let gp_commitment_return = gp_commitment_amount * (input.gross_moic_assumption - Decimal::ONE);

    // ------------------------------------------------------------------
    // 3. Management fee projection (year by year)
    // ------------------------------------------------------------------
    // During investment period: fee on full fund_size
    // After investment period: fee on invested capital basis
    //   Assumes ~80% deployed at end of investment period, declining linearly
    //   to 0 by end of fund life.
    let deployed_pct = dec!(0.80);

    let mut total_mgmt_fees = Decimal::ZERO;
    let mut mgmt_fee_by_year: Vec<Money> = Vec::with_capacity(fund_life as usize);

    for year in 1..=fund_life {
        let base_fee = if year <= inv_period {
            // Investment period: fee on committed capital
            fund_size * effective_fee_rate
        } else {
            // Harvest period: fee on declining invested capital
            let harvest_year = year - inv_period;
            let remaining_fraction = if realisation_years > 0 {
                deployed_pct
                    * (Decimal::ONE
                        - Decimal::from(harvest_year) / Decimal::from(realisation_years))
            } else {
                Decimal::ZERO
            };
            let remaining_fraction = remaining_fraction.max(Decimal::ZERO);
            fund_size * remaining_fraction * effective_fee_rate
        };

        // Apply fee holiday (no fee during holiday years)
        let after_holiday = if year <= fee_holiday {
            Decimal::ZERO
        } else {
            base_fee
        };

        // Apply successor fund offset (halve fee when successor fund starts)
        let final_fee = if year >= successor_offset {
            after_holiday * dec!(0.5)
        } else {
            after_holiday
        };

        total_mgmt_fees += final_fee;
        mgmt_fee_by_year.push(final_fee);
    }

    // ------------------------------------------------------------------
    // 4. Carried interest projection
    // ------------------------------------------------------------------
    let total_fund_profit = fund_size * (input.gross_moic_assumption - Decimal::ONE);

    // Compound hurdle over fund life using iterative multiplication
    let mut hurdle_factor = Decimal::ONE;
    for _ in 0..fund_life {
        hurdle_factor *= Decimal::ONE + input.hurdle_rate;
    }
    let hurdle_amount = fund_size * (hurdle_factor - Decimal::ONE);

    let total_carry = if total_fund_profit > hurdle_amount {
        (total_fund_profit - hurdle_amount) * input.carried_interest_rate
    } else {
        Decimal::ZERO
    };

    // Carry is realised proportionally during realisation period
    let carry_per_realisation_year = if realisation_years > 0 {
        total_carry / Decimal::from(realisation_years)
    } else if fund_life > 0 {
        // Edge case: fund_life == investment_period, all carry in final year
        total_carry
    } else {
        Decimal::ZERO
    };

    // Co-invest return is also spread over realisation period
    let coinvest_per_realisation_year = if realisation_years > 0 {
        gp_commitment_return / Decimal::from(realisation_years)
    } else if fund_life > 0 {
        gp_commitment_return
    } else {
        Decimal::ZERO
    };

    // ------------------------------------------------------------------
    // 5. Year-by-year GP projections
    // ------------------------------------------------------------------
    let mut projections: Vec<GpYearProjection> = Vec::with_capacity(fund_life as usize);
    let mut cumulative_gp_income = Decimal::ZERO;

    for year in 1..=fund_life {
        let mgmt_fee = mgmt_fee_by_year[(year - 1) as usize];

        let carry_accrual = if year > inv_period {
            carry_per_realisation_year
        } else {
            Decimal::ZERO
        };

        let coinvest_return = if year > inv_period {
            coinvest_per_realisation_year
        } else {
            Decimal::ZERO
        };

        let total_income = mgmt_fee + carry_accrual + coinvest_return;
        cumulative_gp_income += total_income;
        let overhead = input.annual_gp_overhead;
        let net_income = total_income - overhead;

        projections.push(GpYearProjection {
            year,
            management_fee: mgmt_fee,
            carry_accrual,
            coinvest_return,
            total_gp_income: total_income,
            cumulative_gp_income,
            overhead,
            net_gp_income: net_income,
        });
    }

    // ------------------------------------------------------------------
    // 6. Aggregate metrics
    // ------------------------------------------------------------------
    let total_gp_coinvest_return = gp_commitment_return;
    let total_gp_revenue = total_mgmt_fees + total_carry + total_gp_coinvest_return;

    let annual_management_fee_income = if fund_life > 0 {
        total_mgmt_fees / Decimal::from(fund_life)
    } else {
        Decimal::ZERO
    };

    // Per-professional metrics
    let carry_per_professional = total_carry / num_pros;
    let management_fee_per_professional = total_mgmt_fees / num_pros;
    let total_per_professional = total_gp_revenue / num_pros;

    // Breakeven AUM: minimum fund size so mgmt fees cover overhead
    let breakeven_aum = if input.management_fee_rate > Decimal::ZERO {
        input.annual_gp_overhead / input.management_fee_rate
    } else {
        Decimal::ZERO
    };

    // Breakeven fund multiple: MOIC at which carry kicks in
    // This is (1 + hurdle_rate)^fund_life_years
    let breakeven_fund_multiple = hurdle_factor;

    // Revenue mix
    let revenue_mix = if total_gp_revenue > Decimal::ZERO {
        RevenueMix {
            management_fee_pct: total_mgmt_fees / total_gp_revenue,
            carry_pct: total_carry / total_gp_revenue,
            coinvest_pct: total_gp_coinvest_return / total_gp_revenue,
        }
    } else {
        RevenueMix {
            management_fee_pct: Decimal::ZERO,
            carry_pct: Decimal::ZERO,
            coinvest_pct: Decimal::ZERO,
        }
    };

    // ------------------------------------------------------------------
    // 7. Assemble output
    // ------------------------------------------------------------------
    let output = GpEconomicsOutput {
        total_management_fee_income: total_mgmt_fees,
        total_carry_income: total_carry,
        total_gp_coinvest_return,
        total_gp_revenue,
        annual_management_fee_income,
        carry_per_professional,
        management_fee_per_professional,
        total_per_professional,
        breakeven_aum,
        breakeven_fund_multiple,
        gp_commitment_amount,
        gp_commitment_return,
        projections,
        revenue_mix,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "GP Economics Model: Management Fees, Carry, Co-invest, Per-Professional Analysis",
        &serde_json::json!({
            "fund_size": input.fund_size.to_string(),
            "management_fee_rate": input.management_fee_rate.to_string(),
            "carried_interest_rate": input.carried_interest_rate.to_string(),
            "hurdle_rate": input.hurdle_rate.to_string(),
            "gp_commitment_pct": input.gp_commitment_pct.to_string(),
            "fund_life_years": input.fund_life_years,
            "investment_period_years": input.investment_period_years,
            "num_investment_professionals": input.num_investment_professionals,
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

fn validate_input(input: &GpEconomicsInput) -> CorpFinanceResult<()> {
    if input.fund_size <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "fund_size".into(),
            reason: "Fund size must be greater than zero".into(),
        });
    }
    if input.num_investment_professionals < 1 {
        return Err(CorpFinanceError::InvalidInput {
            field: "num_investment_professionals".into(),
            reason: "Must have at least 1 investment professional".into(),
        });
    }
    if input.management_fee_rate < Decimal::ZERO || input.management_fee_rate > dec!(0.05) {
        return Err(CorpFinanceError::InvalidInput {
            field: "management_fee_rate".into(),
            reason: "Management fee rate must be between 0 and 0.05 (5%)".into(),
        });
    }
    if input.carried_interest_rate < Decimal::ZERO || input.carried_interest_rate > dec!(0.50) {
        return Err(CorpFinanceError::InvalidInput {
            field: "carried_interest_rate".into(),
            reason: "Carried interest rate must be between 0 and 0.50 (50%)".into(),
        });
    }
    if input.fund_life_years < input.investment_period_years {
        return Err(CorpFinanceError::InvalidInput {
            field: "fund_life_years".into(),
            reason: "Fund life must be >= investment period".into(),
        });
    }
    if input.fund_life_years == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "fund_life_years".into(),
            reason: "Fund life must be at least 1 year".into(),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// Standard 2/20 GP economics input for a $500M fund.
    fn standard_input() -> GpEconomicsInput {
        GpEconomicsInput {
            fund_size: dec!(500_000_000),
            management_fee_rate: dec!(0.02),
            carried_interest_rate: dec!(0.20),
            hurdle_rate: dec!(0.08),
            gp_commitment_pct: dec!(0.02),
            fund_life_years: 10,
            investment_period_years: 5,
            num_investment_professionals: 5,
            annual_gp_overhead: dec!(5_000_000),
            gross_irr_assumption: dec!(0.15),
            gross_moic_assumption: dec!(2.5),
            fee_holiday_years: None,
            fee_discount_rate: None,
            successor_fund_offset: None,
            currency: Some(Currency::USD),
        }
    }

    // ------------------------------------------------------------------
    // Test 1: Basic GP economics
    // ------------------------------------------------------------------
    #[test]
    fn test_basic_gp_economics() {
        let input = standard_input();
        let result = calculate_gp_economics(&input).unwrap();
        let out = &result.result;

        // Should have 10 year projections
        assert_eq!(out.projections.len(), 10);

        // Total management fee income should be positive
        assert!(
            out.total_management_fee_income > Decimal::ZERO,
            "Total mgmt fee income should be positive, got {}",
            out.total_management_fee_income
        );

        // Total carry income should be positive (2x MOIC > 8% hurdle)
        assert!(
            out.total_carry_income > Decimal::ZERO,
            "Carry income should be positive for 2x MOIC fund, got {}",
            out.total_carry_income
        );

        // Total GP revenue should be sum of components
        let expected_total =
            out.total_management_fee_income + out.total_carry_income + out.total_gp_coinvest_return;
        assert_eq!(
            out.total_gp_revenue,
            expected_total,
            "Total GP revenue ({}) should equal mgmt ({}) + carry ({}) + coinvest ({})",
            out.total_gp_revenue,
            out.total_management_fee_income,
            out.total_carry_income,
            out.total_gp_coinvest_return
        );
    }

    // ------------------------------------------------------------------
    // Test 2: Breakeven AUM calculation
    // ------------------------------------------------------------------
    #[test]
    fn test_breakeven_aum_calculation() {
        let input = standard_input();
        let result = calculate_gp_economics(&input).unwrap();
        let out = &result.result;

        // breakeven_aum = annual_overhead / management_fee_rate = 5M / 0.02 = 250M
        let expected = dec!(5_000_000) / dec!(0.02);
        assert_eq!(
            out.breakeven_aum, expected,
            "Breakeven AUM should be {}, got {}",
            expected, out.breakeven_aum
        );
    }

    // ------------------------------------------------------------------
    // Test 3: Breakeven fund multiple
    // ------------------------------------------------------------------
    #[test]
    fn test_breakeven_fund_multiple() {
        let input = standard_input();
        let result = calculate_gp_economics(&input).unwrap();
        let out = &result.result;

        // breakeven_fund_multiple = (1 + 0.08)^10
        // Using iterative multiplication for precision
        let mut expected = Decimal::ONE;
        for _ in 0..10 {
            expected *= dec!(1.08);
        }

        let diff = (out.breakeven_fund_multiple - expected).abs();
        assert!(
            diff < dec!(0.0001),
            "Breakeven fund multiple should be ~{}, got {} (diff {})",
            expected,
            out.breakeven_fund_multiple,
            diff
        );

        // Should be approximately 2.1589
        assert!(
            out.breakeven_fund_multiple > dec!(2.15) && out.breakeven_fund_multiple < dec!(2.17),
            "Breakeven multiple should be ~2.159, got {}",
            out.breakeven_fund_multiple
        );
    }

    // ------------------------------------------------------------------
    // Test 4: Carry per professional
    // ------------------------------------------------------------------
    #[test]
    fn test_carry_per_professional() {
        let input = standard_input();
        let result = calculate_gp_economics(&input).unwrap();
        let out = &result.result;

        let expected = out.total_carry_income / Decimal::from(input.num_investment_professionals);
        assert_eq!(
            out.carry_per_professional, expected,
            "Carry per professional should be {}, got {}",
            expected, out.carry_per_professional
        );

        // Mgmt fee per professional
        let expected_mgmt =
            out.total_management_fee_income / Decimal::from(input.num_investment_professionals);
        assert_eq!(
            out.management_fee_per_professional, expected_mgmt,
            "Mgmt fee per professional should be {}, got {}",
            expected_mgmt, out.management_fee_per_professional
        );

        // Total per professional
        let expected_total =
            out.total_gp_revenue / Decimal::from(input.num_investment_professionals);
        assert_eq!(
            out.total_per_professional, expected_total,
            "Total per professional should be {}, got {}",
            expected_total, out.total_per_professional
        );
    }

    // ------------------------------------------------------------------
    // Test 5: No carry below hurdle
    // ------------------------------------------------------------------
    #[test]
    fn test_no_carry_below_hurdle() {
        let mut input = standard_input();
        // Set MOIC so total profit < hurdle amount
        // Hurdle amount ~ fund_size * ((1.08)^10 - 1) ~ 500M * 1.159 ~ 579.5M
        // For carry = 0, need profit < 579.5M, i.e., MOIC < 1 + 1.159 = ~2.159
        // Use MOIC of 1.5 => profit = 250M which is well below hurdle
        input.gross_moic_assumption = dec!(1.5);
        input.gross_irr_assumption = dec!(0.04);

        let result = calculate_gp_economics(&input).unwrap();
        let out = &result.result;

        assert_eq!(
            out.total_carry_income,
            Decimal::ZERO,
            "No carry should be earned when MOIC ({}) produces profit below hurdle",
            input.gross_moic_assumption
        );
    }

    // ------------------------------------------------------------------
    // Test 6: Revenue mix sums to one
    // ------------------------------------------------------------------
    #[test]
    fn test_revenue_mix_sums_to_one() {
        let input = standard_input();
        let result = calculate_gp_economics(&input).unwrap();
        let out = &result.result;

        let mix = &out.revenue_mix;
        let total = mix.management_fee_pct + mix.carry_pct + mix.coinvest_pct;
        let diff = (total - Decimal::ONE).abs();
        assert!(
            diff < dec!(0.0001),
            "Revenue mix should sum to 1.0, got {} (mgmt {}, carry {}, coinvest {})",
            total,
            mix.management_fee_pct,
            mix.carry_pct,
            mix.coinvest_pct
        );
    }

    // ------------------------------------------------------------------
    // Test 7: Fee holiday reduces income
    // ------------------------------------------------------------------
    #[test]
    fn test_fee_holiday_reduces_income() {
        let input_no_holiday = standard_input();
        let result_no_holiday = calculate_gp_economics(&input_no_holiday).unwrap();

        let mut input_with_holiday = standard_input();
        input_with_holiday.fee_holiday_years = Some(2);
        let result_with_holiday = calculate_gp_economics(&input_with_holiday).unwrap();

        assert!(
            result_with_holiday.result.total_management_fee_income
                < result_no_holiday.result.total_management_fee_income,
            "Fee holiday should reduce mgmt fee income: with holiday = {}, without = {}",
            result_with_holiday.result.total_management_fee_income,
            result_no_holiday.result.total_management_fee_income
        );

        // Verify the first 2 years have zero management fees
        assert_eq!(
            result_with_holiday.result.projections[0].management_fee,
            Decimal::ZERO,
            "Year 1 mgmt fee should be 0 during holiday"
        );
        assert_eq!(
            result_with_holiday.result.projections[1].management_fee,
            Decimal::ZERO,
            "Year 2 mgmt fee should be 0 during holiday"
        );
        assert!(
            result_with_holiday.result.projections[2].management_fee > Decimal::ZERO,
            "Year 3 mgmt fee should be positive after holiday"
        );
    }

    // ------------------------------------------------------------------
    // Test 8: Successor fund offset
    // ------------------------------------------------------------------
    #[test]
    fn test_successor_fund_offset() {
        let input_no_successor = standard_input();
        let result_no_successor = calculate_gp_economics(&input_no_successor).unwrap();

        let mut input_with_successor = standard_input();
        input_with_successor.successor_fund_offset = Some(6);
        let result_with_successor = calculate_gp_economics(&input_with_successor).unwrap();

        assert!(
            result_with_successor.result.total_management_fee_income
                < result_no_successor.result.total_management_fee_income,
            "Successor fund offset should reduce mgmt fee income: with = {}, without = {}",
            result_with_successor.result.total_management_fee_income,
            result_no_successor.result.total_management_fee_income
        );

        // Years before offset should be unchanged
        for year_idx in 0..5 {
            assert_eq!(
                result_with_successor.result.projections[year_idx].management_fee,
                result_no_successor.result.projections[year_idx].management_fee,
                "Year {} mgmt fee should be unchanged before successor offset",
                year_idx + 1
            );
        }

        // Years from offset onward should be halved
        let year_6_with = result_with_successor.result.projections[5].management_fee;
        let year_6_without = result_no_successor.result.projections[5].management_fee;
        let diff = (year_6_with - year_6_without * dec!(0.5)).abs();
        assert!(
            diff < dec!(0.01),
            "Year 6 fee with successor should be half: got {}, expected {}",
            year_6_with,
            year_6_without * dec!(0.5)
        );
    }

    // ------------------------------------------------------------------
    // Test 9: GP commitment return
    // ------------------------------------------------------------------
    #[test]
    fn test_gp_commitment_return() {
        let input = standard_input();
        let result = calculate_gp_economics(&input).unwrap();
        let out = &result.result;

        // gp_commitment_amount = 500M * 0.02 = 10M
        let expected_amount = dec!(500_000_000) * dec!(0.02);
        assert_eq!(
            out.gp_commitment_amount, expected_amount,
            "GP commitment amount should be {}, got {}",
            expected_amount, out.gp_commitment_amount
        );

        // gp_commitment_return = 10M * (2.5 - 1.0) = 15M
        let expected_return = expected_amount * (dec!(2.5) - Decimal::ONE);
        assert_eq!(
            out.gp_commitment_return, expected_return,
            "GP commitment return should be {}, got {}",
            expected_return, out.gp_commitment_return
        );

        // total_gp_coinvest_return should equal gp_commitment_return
        assert_eq!(
            out.total_gp_coinvest_return, out.gp_commitment_return,
            "Total coinvest return should equal commitment return"
        );
    }

    // ------------------------------------------------------------------
    // Test 10: Projections sum to total
    // ------------------------------------------------------------------
    #[test]
    fn test_projections_sum_to_total() {
        let input = standard_input();
        let result = calculate_gp_economics(&input).unwrap();
        let out = &result.result;

        let sum_mgmt: Decimal = out.projections.iter().map(|p| p.management_fee).sum();
        let sum_carry: Decimal = out.projections.iter().map(|p| p.carry_accrual).sum();
        let sum_coinvest: Decimal = out.projections.iter().map(|p| p.coinvest_return).sum();

        let mgmt_diff = (sum_mgmt - out.total_management_fee_income).abs();
        assert!(
            mgmt_diff < dec!(0.01),
            "Sum of projection mgmt fees ({}) should equal total ({})",
            sum_mgmt,
            out.total_management_fee_income
        );

        let carry_diff = (sum_carry - out.total_carry_income).abs();
        assert!(
            carry_diff < dec!(0.01),
            "Sum of projection carry ({}) should equal total ({})",
            sum_carry,
            out.total_carry_income
        );

        let coinvest_diff = (sum_coinvest - out.total_gp_coinvest_return).abs();
        assert!(
            coinvest_diff < dec!(0.01),
            "Sum of projection coinvest ({}) should equal total ({})",
            sum_coinvest,
            out.total_gp_coinvest_return
        );

        // Cumulative in last year should equal sum of all total_gp_income
        let sum_total: Decimal = out.projections.iter().map(|p| p.total_gp_income).sum();
        let last = out.projections.last().unwrap();
        let cum_diff = (last.cumulative_gp_income - sum_total).abs();
        assert!(
            cum_diff < dec!(0.01),
            "Last year cumulative ({}) should equal sum of all income ({})",
            last.cumulative_gp_income,
            sum_total
        );
    }

    // ------------------------------------------------------------------
    // Test 11: Zero fund size error
    // ------------------------------------------------------------------
    #[test]
    fn test_zero_fund_size_error() {
        let mut input = standard_input();
        input.fund_size = Decimal::ZERO;

        let result = calculate_gp_economics(&input);
        assert!(result.is_err(), "Zero fund size should produce an error");

        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "fund_size");
            }
            other => panic!("Expected InvalidInput for fund_size, got: {other}"),
        }
    }

    // ------------------------------------------------------------------
    // Test 12: Metadata populated
    // ------------------------------------------------------------------
    #[test]
    fn test_metadata_populated() {
        let input = standard_input();
        let result = calculate_gp_economics(&input).unwrap();

        assert!(!result.methodology.is_empty(), "Methodology should be set");
        assert!(
            result.metadata.precision == "rust_decimal_128bit",
            "Precision should be rust_decimal_128bit"
        );
        assert!(
            !result.metadata.version.is_empty(),
            "Version should be populated"
        );
        // computation_time_us can be 0 on very fast machines, just check it exists
    }

    // ------------------------------------------------------------------
    // Test 13: Fee discount rate
    // ------------------------------------------------------------------
    #[test]
    fn test_fee_discount_rate() {
        let input_no_discount = standard_input();
        let result_no_discount = calculate_gp_economics(&input_no_discount).unwrap();

        let mut input_with_discount = standard_input();
        input_with_discount.fee_discount_rate = Some(dec!(0.25)); // 25% discount
        let result_with_discount = calculate_gp_economics(&input_with_discount).unwrap();

        // With 25% discount, mgmt fees should be 75% of no-discount
        let ratio = result_with_discount.result.total_management_fee_income
            / result_no_discount.result.total_management_fee_income;
        let diff = (ratio - dec!(0.75)).abs();
        assert!(
            diff < dec!(0.0001),
            "25% fee discount should reduce fees to 75%: ratio = {}",
            ratio
        );
    }

    // ------------------------------------------------------------------
    // Test 14: Validation — management fee rate too high
    // ------------------------------------------------------------------
    #[test]
    fn test_management_fee_rate_too_high() {
        let mut input = standard_input();
        input.management_fee_rate = dec!(0.06); // Above 5% max

        let result = calculate_gp_economics(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "management_fee_rate");
            }
            other => panic!("Expected InvalidInput for management_fee_rate, got: {other}"),
        }
    }

    // ------------------------------------------------------------------
    // Test 15: Validation — carried interest rate too high
    // ------------------------------------------------------------------
    #[test]
    fn test_carried_interest_rate_too_high() {
        let mut input = standard_input();
        input.carried_interest_rate = dec!(0.60); // Above 50% max

        let result = calculate_gp_economics(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "carried_interest_rate");
            }
            other => panic!("Expected InvalidInput for carried_interest_rate, got: {other}"),
        }
    }
}
