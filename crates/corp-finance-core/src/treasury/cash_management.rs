//! Corporate cash management and liquidity analysis.
//!
//! Provides a 12-month cash position simulation including:
//! - Month-by-month opening/closing balances with operating cash flows
//! - Automatic sweep of surplus cash to a money-market investment account
//! - Revolving credit facility draws when cash falls below the minimum buffer
//! - Interest earned on surplus / interest paid on facility draws
//! - Cash conversion cycle (service-company model: DIO = 0)
//! - Liquidity scoring and actionable recommendations
//!
//! All calculations use `rust_decimal::Decimal` for precision. No `f64`.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::{CorpFinanceError, CorpFinanceResult};

// ---------------------------------------------------------------------------
// Input / Output types
// ---------------------------------------------------------------------------

/// Input for corporate cash management analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashManagementInput {
    /// Current cash balance.
    pub current_cash: Decimal,
    /// Projected monthly operating cash flows (12 months).
    pub operating_cash_flows: Vec<Decimal>,
    /// Minimum required cash balance.
    pub minimum_cash_buffer: Decimal,
    /// Available revolving credit facility size.
    pub credit_facility_size: Decimal,
    /// Annual rate charged on drawn facility.
    pub credit_facility_rate: Decimal,
    /// Annual rate earned on surplus cash (money market).
    pub investment_rate: Decimal,
    /// Annual penalty rate for going below the minimum buffer.
    pub overdraft_rate: Decimal,
    /// Current accounts receivable.
    pub accounts_receivable: Decimal,
    /// Current accounts payable.
    pub accounts_payable: Decimal,
    /// Days sales outstanding.
    pub dso_days: Decimal,
    /// Days payable outstanding.
    pub dpo_days: Decimal,
    /// Annual revenue (for DSO/DPO calculations).
    pub annual_revenue: Decimal,
    /// Cash level above which auto-sweep to investment account.
    pub sweep_threshold: Decimal,
    /// Target cash as a percentage of revenue.
    pub target_cash_ratio: Decimal,
}

/// Output of the cash management analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashManagementOutput {
    /// Projected 12-month cash positions.
    pub monthly_balances: Vec<MonthlyCashBalance>,
    /// Maximum cash balance over the period.
    pub peak_cash: Decimal,
    /// Minimum cash balance over the period.
    pub trough_cash: Decimal,
    /// Average cash balance over the period.
    pub average_cash: Decimal,
    /// Peak credit facility utilisation as a percentage (0-1+).
    pub facility_utilization_peak: Decimal,
    /// Number of months the credit facility is drawn.
    pub months_negative: u32,
    /// Total interest earned from surplus investment.
    pub total_interest_earned: Decimal,
    /// Total interest paid on facility draws.
    pub total_interest_paid: Decimal,
    /// Net interest (earned minus paid).
    pub net_interest: Decimal,
    /// Cash conversion cycle = DSO + DIO(0) - DPO.
    pub cash_conversion_cycle: Decimal,
    /// Free cash flow yield = sum(OCF) / annual_revenue.
    pub free_cash_flow_yield: Decimal,
    /// Liquidity score: "Strong" / "Adequate" / "Tight" / "Critical".
    pub liquidity_score: String,
    /// Actionable recommendations.
    pub recommendations: Vec<String>,
}

/// A single month in the cash projection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthlyCashBalance {
    /// Month number (1-12).
    pub month: u32,
    /// Opening cash balance.
    pub opening: Decimal,
    /// Operating cash flow for the month.
    pub operating_flow: Decimal,
    /// Amount swept to investment (positive = swept out).
    pub sweep_amount: Decimal,
    /// Amount drawn from credit facility (positive = drawn).
    pub facility_draw: Decimal,
    /// Net interest for the month (positive = earned, negative = paid).
    pub interest: Decimal,
    /// Closing cash balance.
    pub closing: Decimal,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Analyse corporate cash management over a 12-month horizon.
///
/// Runs a month-by-month simulation: each month starts with the prior
/// closing balance, adds the operating cash flow, sweeps excess cash to
/// an investment account, draws on the revolving credit facility if below
/// the minimum buffer, and computes interest.
pub fn analyze_cash_management(
    input: &CashManagementInput,
) -> CorpFinanceResult<CashManagementOutput> {
    validate_input(input)?;

    let twelve = Decimal::from(12u32);
    let monthly_investment_rate = input.investment_rate / twelve;
    let monthly_facility_rate = input.credit_facility_rate / twelve;

    let mut monthly_balances: Vec<MonthlyCashBalance> = Vec::with_capacity(12);
    let mut opening = input.current_cash;
    let mut total_interest_earned = Decimal::ZERO;
    let mut total_interest_paid = Decimal::ZERO;
    let mut peak_facility_draw = Decimal::ZERO;
    let mut months_negative: u32 = 0;

    for month_idx in 0..12usize {
        let ocf = if month_idx < input.operating_cash_flows.len() {
            input.operating_cash_flows[month_idx]
        } else {
            Decimal::ZERO
        };

        let after_ocf = opening + ocf;

        let mut sweep_amount = Decimal::ZERO;
        let mut facility_draw = Decimal::ZERO;
        let mut interest = Decimal::ZERO;

        if after_ocf > input.sweep_threshold {
            // Sweep excess to investment
            sweep_amount = after_ocf - input.sweep_threshold;
            let earned = sweep_amount * monthly_investment_rate;
            interest = earned;
            total_interest_earned += earned;
        } else if after_ocf < input.minimum_cash_buffer {
            // Need to draw on revolving facility (each month independent)
            let shortfall = input.minimum_cash_buffer - after_ocf;
            let draw = shortfall.min(input.credit_facility_size);
            if draw > Decimal::ZERO {
                facility_draw = draw;
                let paid = draw * monthly_facility_rate;
                interest = -paid;
                total_interest_paid += paid;
                months_negative += 1;

                if draw > peak_facility_draw {
                    peak_facility_draw = draw;
                }
            }
        }

        let closing = after_ocf - sweep_amount + facility_draw + interest;

        monthly_balances.push(MonthlyCashBalance {
            month: (month_idx as u32) + 1,
            opening,
            operating_flow: ocf,
            sweep_amount,
            facility_draw,
            interest,
            closing,
        });

        opening = closing;
    }

    // Aggregate statistics
    let closings: Vec<Decimal> = monthly_balances.iter().map(|b| b.closing).collect();
    let peak_cash = closings.iter().copied().max().unwrap_or(Decimal::ZERO);
    let trough_cash = closings.iter().copied().min().unwrap_or(Decimal::ZERO);
    let sum_cash: Decimal = closings.iter().copied().sum();
    let average_cash = sum_cash / twelve;

    let facility_utilization_peak = if input.credit_facility_size.is_zero() {
        Decimal::ZERO
    } else {
        peak_facility_draw / input.credit_facility_size
    };

    let net_interest = total_interest_earned - total_interest_paid;

    // CCC = DSO + DIO(0) - DPO  (service company: DIO = 0)
    let cash_conversion_cycle = input.dso_days - input.dpo_days;

    // Free cash flow yield = sum(OCF) / annual_revenue
    let sum_ocf: Decimal = input.operating_cash_flows.iter().copied().sum();
    let free_cash_flow_yield = if input.annual_revenue.is_zero() {
        Decimal::ZERO
    } else {
        sum_ocf / input.annual_revenue
    };

    // Liquidity score
    let liquidity_score = compute_liquidity_score(
        months_negative,
        &input.credit_facility_size,
        peak_facility_draw,
    );

    // Recommendations
    let recommendations = build_recommendations(
        input,
        &liquidity_score,
        months_negative,
        cash_conversion_cycle,
        facility_utilization_peak,
    );

    Ok(CashManagementOutput {
        monthly_balances,
        peak_cash,
        trough_cash,
        average_cash,
        facility_utilization_peak,
        months_negative,
        total_interest_earned,
        total_interest_paid,
        net_interest,
        cash_conversion_cycle,
        free_cash_flow_yield,
        liquidity_score,
        recommendations,
    })
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn validate_input(input: &CashManagementInput) -> CorpFinanceResult<()> {
    if input.operating_cash_flows.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one month of operating cash flows is required.".into(),
        ));
    }
    if input.minimum_cash_buffer < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "minimum_cash_buffer".into(),
            reason: "Minimum cash buffer cannot be negative.".into(),
        });
    }
    if input.credit_facility_size < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "credit_facility_size".into(),
            reason: "Credit facility size cannot be negative.".into(),
        });
    }
    if input.credit_facility_rate < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "credit_facility_rate".into(),
            reason: "Credit facility rate cannot be negative.".into(),
        });
    }
    if input.investment_rate < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "investment_rate".into(),
            reason: "Investment rate cannot be negative.".into(),
        });
    }
    if input.annual_revenue < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "annual_revenue".into(),
            reason: "Annual revenue cannot be negative.".into(),
        });
    }
    if input.sweep_threshold < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "sweep_threshold".into(),
            reason: "Sweep threshold cannot be negative.".into(),
        });
    }
    Ok(())
}

fn compute_liquidity_score(
    months_negative: u32,
    facility_size: &Decimal,
    total_drawn: Decimal,
) -> String {
    let exceeds_facility = total_drawn > *facility_size;
    if months_negative == 0 {
        "Strong".to_string()
    } else if months_negative < 3 && !exceeds_facility {
        "Adequate".to_string()
    } else if months_negative <= 6 && !exceeds_facility {
        "Tight".to_string()
    } else {
        "Critical".to_string()
    }
}

fn build_recommendations(
    input: &CashManagementInput,
    score: &str,
    months_negative: u32,
    ccc: Decimal,
    facility_utilization: Decimal,
) -> Vec<String> {
    let mut recs = Vec::new();

    if score == "Critical" {
        recs.push(
            "Liquidity is critical. Consider raising additional equity or negotiating \
             a larger credit facility immediately."
                .to_string(),
        );
    }

    if score == "Tight" {
        recs.push(
            "Liquidity is tight. Review discretionary spending and accelerate \
             receivables collection to improve cash position."
                .to_string(),
        );
    }

    if months_negative > 0 {
        recs.push(format!(
            "Credit facility is drawn in {} of 12 months. Consider smoothing \
             operating cash flows or building a larger cash buffer.",
            months_negative
        ));
    }

    if ccc > dec!(60) {
        recs.push(format!(
            "Cash conversion cycle is {} days. Reducing DSO or extending \
             DPO would improve working capital efficiency.",
            ccc
        ));
    }

    if facility_utilization > dec!(0.75) {
        recs.push(
            "Peak facility utilisation exceeds 75%. Negotiate headroom or \
             arrange a backup facility."
                .to_string(),
        );
    }

    let target_cash = input.annual_revenue * input.target_cash_ratio;
    if input.current_cash < target_cash {
        recs.push(format!(
            "Current cash ({}) is below the target cash balance ({}). \
             Consider retaining more operating cash flow.",
            input.current_cash, target_cash
        ));
    }

    if input.sweep_threshold <= input.minimum_cash_buffer {
        recs.push(
            "Sweep threshold is at or below the minimum cash buffer. \
             Consider increasing the sweep threshold to avoid unnecessary \
             facility draws."
                .to_string(),
        );
    }

    if recs.is_empty() {
        recs.push(
            "Cash position is healthy. Continue monitoring monthly balances \
             and maintain the current sweep/facility framework."
                .to_string(),
        );
    }

    recs
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    // -- Test helpers --------------------------------------------------------

    fn default_input() -> CashManagementInput {
        CashManagementInput {
            current_cash: dec!(1_000_000),
            operating_cash_flows: vec![
                dec!(100_000),
                dec!(80_000),
                dec!(120_000),
                dec!(90_000),
                dec!(-50_000),
                dec!(-30_000),
                dec!(110_000),
                dec!(95_000),
                dec!(105_000),
                dec!(85_000),
                dec!(115_000),
                dec!(100_000),
            ],
            minimum_cash_buffer: dec!(500_000),
            credit_facility_size: dec!(2_000_000),
            credit_facility_rate: dec!(0.06),
            investment_rate: dec!(0.04),
            overdraft_rate: dec!(0.10),
            accounts_receivable: dec!(300_000),
            accounts_payable: dec!(200_000),
            dso_days: dec!(45),
            dpo_days: dec!(30),
            annual_revenue: dec!(5_000_000),
            sweep_threshold: dec!(1_500_000),
            target_cash_ratio: dec!(0.15),
        }
    }

    fn tight_liquidity_input() -> CashManagementInput {
        CashManagementInput {
            current_cash: dec!(200_000),
            operating_cash_flows: vec![
                dec!(-100_000),
                dec!(-80_000),
                dec!(-120_000),
                dec!(-90_000),
                dec!(-50_000),
                dec!(20_000),
                dec!(10_000),
                dec!(-30_000),
                dec!(5_000),
                dec!(-15_000),
                dec!(30_000),
                dec!(40_000),
            ],
            minimum_cash_buffer: dec!(300_000),
            credit_facility_size: dec!(500_000),
            credit_facility_rate: dec!(0.08),
            investment_rate: dec!(0.03),
            overdraft_rate: dec!(0.12),
            accounts_receivable: dec!(150_000),
            accounts_payable: dec!(100_000),
            dso_days: dec!(60),
            dpo_days: dec!(25),
            annual_revenue: dec!(2_000_000),
            sweep_threshold: dec!(600_000),
            target_cash_ratio: dec!(0.20),
        }
    }

    // -- Validation tests ----------------------------------------------------

    #[test]
    fn test_empty_cash_flows_rejected() {
        let mut input = default_input();
        input.operating_cash_flows = vec![];
        let result = analyze_cash_management(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_negative_minimum_buffer_rejected() {
        let mut input = default_input();
        input.minimum_cash_buffer = dec!(-1);
        let result = analyze_cash_management(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_negative_facility_size_rejected() {
        let mut input = default_input();
        input.credit_facility_size = dec!(-1);
        let result = analyze_cash_management(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_negative_facility_rate_rejected() {
        let mut input = default_input();
        input.credit_facility_rate = dec!(-0.01);
        let result = analyze_cash_management(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_negative_investment_rate_rejected() {
        let mut input = default_input();
        input.investment_rate = dec!(-0.01);
        let result = analyze_cash_management(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_negative_annual_revenue_rejected() {
        let mut input = default_input();
        input.annual_revenue = dec!(-1);
        let result = analyze_cash_management(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_negative_sweep_threshold_rejected() {
        let mut input = default_input();
        input.sweep_threshold = dec!(-1);
        let result = analyze_cash_management(&input);
        assert!(result.is_err());
    }

    // -- Basic output structure tests ----------------------------------------

    #[test]
    fn test_twelve_monthly_balances_returned() {
        let input = default_input();
        let result = analyze_cash_management(&input).unwrap();
        assert_eq!(result.monthly_balances.len(), 12);
    }

    #[test]
    fn test_month_numbers_sequential() {
        let input = default_input();
        let result = analyze_cash_management(&input).unwrap();
        for (i, b) in result.monthly_balances.iter().enumerate() {
            assert_eq!(b.month, (i as u32) + 1);
        }
    }

    #[test]
    fn test_first_month_opening_equals_current_cash() {
        let input = default_input();
        let result = analyze_cash_management(&input).unwrap();
        assert_eq!(result.monthly_balances[0].opening, input.current_cash);
    }

    #[test]
    fn test_closing_equals_next_opening() {
        let input = default_input();
        let result = analyze_cash_management(&input).unwrap();
        for i in 0..11 {
            assert_eq!(
                result.monthly_balances[i].closing,
                result.monthly_balances[i + 1].opening,
                "Month {} closing should equal month {} opening",
                i + 1,
                i + 2
            );
        }
    }

    // -- Cash flow simulation tests ------------------------------------------

    #[test]
    fn test_sweep_when_above_threshold() {
        // First month: 1_000_000 + 100_000 = 1_100_000 < 1_500_000 threshold
        // So no sweep. Let's create a scenario where sweep happens.
        let mut input = default_input();
        input.current_cash = dec!(2_000_000);
        // After first OCF: 2_000_000 + 100_000 = 2_100_000 > 1_500_000
        let result = analyze_cash_management(&input).unwrap();
        let m1 = &result.monthly_balances[0];
        assert_eq!(m1.sweep_amount, dec!(2_100_000) - dec!(1_500_000));
        assert!(m1.sweep_amount > Decimal::ZERO);
    }

    #[test]
    fn test_facility_draw_when_below_minimum() {
        let mut input = default_input();
        input.current_cash = dec!(400_000);
        input.operating_cash_flows = vec![dec!(-200_000); 12];
        // After first OCF: 400_000 - 200_000 = 200_000 < 500_000 minimum
        let result = analyze_cash_management(&input).unwrap();
        let m1 = &result.monthly_balances[0];
        assert!(
            m1.facility_draw > Decimal::ZERO,
            "Should draw on facility when below minimum"
        );
    }

    #[test]
    fn test_no_draw_when_above_minimum() {
        let input = default_input();
        // current_cash=1M, first OCF=100k => 1.1M > 500k minimum, < 1.5M threshold
        let result = analyze_cash_management(&input).unwrap();
        let m1 = &result.monthly_balances[0];
        assert_eq!(m1.facility_draw, Decimal::ZERO);
    }

    #[test]
    fn test_interest_earned_positive_on_sweep() {
        let mut input = default_input();
        input.current_cash = dec!(3_000_000);
        let result = analyze_cash_management(&input).unwrap();
        assert!(
            result.total_interest_earned > Decimal::ZERO,
            "Should earn interest when sweeping surplus"
        );
    }

    #[test]
    fn test_interest_paid_positive_on_facility_draw() {
        let input = tight_liquidity_input();
        let result = analyze_cash_management(&input).unwrap();
        assert!(
            result.total_interest_paid > Decimal::ZERO,
            "Should pay interest on facility draws"
        );
    }

    #[test]
    fn test_net_interest_calculation() {
        let input = default_input();
        let result = analyze_cash_management(&input).unwrap();
        assert_eq!(
            result.net_interest,
            result.total_interest_earned - result.total_interest_paid
        );
    }

    #[test]
    fn test_closing_balance_formula() {
        let input = default_input();
        let result = analyze_cash_management(&input).unwrap();
        for b in &result.monthly_balances {
            let expected =
                b.opening + b.operating_flow - b.sweep_amount + b.facility_draw + b.interest;
            assert_eq!(
                b.closing, expected,
                "Month {}: closing = opening + ocf - sweep + draw + interest",
                b.month
            );
        }
    }

    // -- Aggregate statistic tests -------------------------------------------

    #[test]
    fn test_peak_cash_is_max_closing() {
        let input = default_input();
        let result = analyze_cash_management(&input).unwrap();
        let max_closing = result
            .monthly_balances
            .iter()
            .map(|b| b.closing)
            .max()
            .unwrap();
        assert_eq!(result.peak_cash, max_closing);
    }

    #[test]
    fn test_trough_cash_is_min_closing() {
        let input = default_input();
        let result = analyze_cash_management(&input).unwrap();
        let min_closing = result
            .monthly_balances
            .iter()
            .map(|b| b.closing)
            .min()
            .unwrap();
        assert_eq!(result.trough_cash, min_closing);
    }

    #[test]
    fn test_average_cash_calculation() {
        let input = default_input();
        let result = analyze_cash_management(&input).unwrap();
        let sum: Decimal = result.monthly_balances.iter().map(|b| b.closing).sum();
        let expected_avg = sum / Decimal::from(12u32);
        assert_eq!(result.average_cash, expected_avg);
    }

    // -- CCC and FCF yield tests ---------------------------------------------

    #[test]
    fn test_cash_conversion_cycle() {
        let input = default_input();
        let result = analyze_cash_management(&input).unwrap();
        // CCC = DSO + 0 - DPO = 45 - 30 = 15
        assert_eq!(result.cash_conversion_cycle, dec!(15));
    }

    #[test]
    fn test_ccc_can_be_negative() {
        let mut input = default_input();
        input.dso_days = dec!(20);
        input.dpo_days = dec!(45);
        let result = analyze_cash_management(&input).unwrap();
        assert_eq!(result.cash_conversion_cycle, dec!(-25));
    }

    #[test]
    fn test_free_cash_flow_yield() {
        let input = default_input();
        let result = analyze_cash_management(&input).unwrap();
        let sum_ocf: Decimal = input.operating_cash_flows.iter().copied().sum();
        let expected = sum_ocf / input.annual_revenue;
        assert_eq!(result.free_cash_flow_yield, expected);
    }

    #[test]
    fn test_fcf_yield_zero_revenue() {
        let mut input = default_input();
        input.annual_revenue = Decimal::ZERO;
        let result = analyze_cash_management(&input).unwrap();
        assert_eq!(result.free_cash_flow_yield, Decimal::ZERO);
    }

    // -- Liquidity score tests -----------------------------------------------

    #[test]
    fn test_liquidity_score_strong() {
        // Default input should never need facility
        let input = default_input();
        let result = analyze_cash_management(&input).unwrap();
        assert_eq!(result.liquidity_score, "Strong");
    }

    #[test]
    fn test_liquidity_score_adequate() {
        // Draw facility for 1-2 months only
        let mut input = default_input();
        input.current_cash = dec!(450_000);
        input.operating_cash_flows = vec![
            dec!(-100_000), // month 1: 350k < 500k => draw
            dec!(-50_000),  // month 2: may still need draw
            dec!(300_000),  // month 3: recovers
            dec!(200_000),
            dec!(200_000),
            dec!(200_000),
            dec!(200_000),
            dec!(200_000),
            dec!(200_000),
            dec!(200_000),
            dec!(200_000),
            dec!(200_000),
        ];
        let result = analyze_cash_management(&input).unwrap();
        assert_eq!(result.liquidity_score, "Adequate");
    }

    #[test]
    fn test_liquidity_score_tight() {
        let mut input = default_input();
        input.current_cash = dec!(400_000);
        input.operating_cash_flows = vec![
            dec!(-50_000),
            dec!(-50_000),
            dec!(-50_000),
            dec!(-50_000),
            dec!(-50_000),
            dec!(300_000),
            dec!(300_000),
            dec!(300_000),
            dec!(300_000),
            dec!(300_000),
            dec!(300_000),
            dec!(300_000),
        ];
        let result = analyze_cash_management(&input).unwrap();
        assert!(
            result.liquidity_score == "Tight" || result.liquidity_score == "Critical",
            "Expected Tight or Critical, got {}",
            result.liquidity_score
        );
    }

    #[test]
    fn test_liquidity_score_critical_many_months() {
        let mut input = default_input();
        input.current_cash = dec!(100_000);
        input.operating_cash_flows = vec![dec!(-100_000); 12];
        let result = analyze_cash_management(&input).unwrap();
        assert_eq!(result.liquidity_score, "Critical");
    }

    // -- Facility utilisation tests ------------------------------------------

    #[test]
    fn test_facility_utilization_zero_when_no_draws() {
        let input = default_input();
        let result = analyze_cash_management(&input).unwrap();
        assert_eq!(result.facility_utilization_peak, Decimal::ZERO);
    }

    #[test]
    fn test_facility_utilization_with_zero_facility() {
        let mut input = default_input();
        input.credit_facility_size = Decimal::ZERO;
        let result = analyze_cash_management(&input).unwrap();
        assert_eq!(result.facility_utilization_peak, Decimal::ZERO);
    }

    // -- Recommendations tests -----------------------------------------------

    #[test]
    fn test_recommendations_not_empty() {
        let input = default_input();
        let result = analyze_cash_management(&input).unwrap();
        assert!(!result.recommendations.is_empty());
    }

    #[test]
    fn test_recommendations_mention_facility_when_drawn() {
        let input = tight_liquidity_input();
        let result = analyze_cash_management(&input).unwrap();
        let has_facility_rec = result
            .recommendations
            .iter()
            .any(|r| r.contains("facility") || r.contains("Credit"));
        assert!(
            has_facility_rec,
            "Should have recommendations about facility usage"
        );
    }

    #[test]
    fn test_sweep_threshold_below_buffer_recommendation() {
        let mut input = default_input();
        input.sweep_threshold = dec!(400_000); // below minimum_cash_buffer of 500k
        let result = analyze_cash_management(&input).unwrap();
        let has_sweep_rec = result
            .recommendations
            .iter()
            .any(|r| r.contains("Sweep threshold"));
        assert!(
            has_sweep_rec,
            "Should warn when sweep threshold <= minimum buffer"
        );
    }

    // -- Edge case tests -----------------------------------------------------

    #[test]
    fn test_single_month_cash_flow() {
        let mut input = default_input();
        input.operating_cash_flows = vec![dec!(50_000)];
        let result = analyze_cash_management(&input).unwrap();
        assert_eq!(result.monthly_balances.len(), 12);
        // Months 2-12 should have zero OCF
        for b in &result.monthly_balances[1..] {
            assert_eq!(b.operating_flow, Decimal::ZERO);
        }
    }

    #[test]
    fn test_all_zero_cash_flows() {
        let mut input = default_input();
        input.operating_cash_flows = vec![Decimal::ZERO; 12];
        let result = analyze_cash_management(&input).unwrap();
        // No sweep (1M < 1.5M threshold), no draw (1M > 500k minimum)
        for b in &result.monthly_balances {
            assert_eq!(b.sweep_amount, Decimal::ZERO);
            assert_eq!(b.facility_draw, Decimal::ZERO);
        }
    }

    #[test]
    fn test_large_negative_cash_flow() {
        let mut input = default_input();
        input.operating_cash_flows = vec![dec!(-5_000_000); 12];
        // Should draw heavily on facility
        let result = analyze_cash_management(&input).unwrap();
        assert!(result.months_negative > 0);
        assert!(result.total_interest_paid > Decimal::ZERO);
    }

    #[test]
    fn test_months_negative_count() {
        let input = tight_liquidity_input();
        let result = analyze_cash_management(&input).unwrap();
        // Count months with facility draws
        let draw_months = result
            .monthly_balances
            .iter()
            .filter(|b| b.facility_draw > Decimal::ZERO)
            .count() as u32;
        assert_eq!(result.months_negative, draw_months);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let input = default_input();
        let result = analyze_cash_management(&input).unwrap();
        let json = serde_json::to_string(&result).unwrap();
        let _deserialized: CashManagementOutput = serde_json::from_str(&json).unwrap();
    }
}
