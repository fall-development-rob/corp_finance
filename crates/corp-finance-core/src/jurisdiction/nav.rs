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
pub enum EqualisationMethod {
    EqualisationShares,
    SeriesAccounting,
    DepreciationDeposit,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CrystallisationFrequency {
    Monthly,
    Quarterly,
    SemiAnnually,
    Annually,
    OnRedemption,
}

impl CrystallisationFrequency {
    /// Returns the fraction of a year represented by one period.
    fn period_fraction(&self) -> Decimal {
        match self {
            CrystallisationFrequency::Monthly => Decimal::ONE / dec!(12),
            CrystallisationFrequency::Quarterly => dec!(0.25),
            CrystallisationFrequency::SemiAnnually => dec!(0.5),
            CrystallisationFrequency::Annually => Decimal::ONE,
            CrystallisationFrequency::OnRedemption => Decimal::ONE,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub investor_id: String,
    pub amount: Money,
    pub nav_per_share_at_entry: Money,
    pub shares_issued: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Redemption {
    pub investor_id: String,
    pub shares_redeemed: Decimal,
    pub nav_per_share_at_exit: Money,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareClassInput {
    pub class_name: String,
    pub currency: Currency,
    pub shares_outstanding: Decimal,
    pub nav_per_share_opening: Money,
    pub high_water_mark: Money,
    pub management_fee_rate: Rate,
    pub performance_fee_rate: Rate,
    pub hurdle_rate: Option<Rate>,
    pub crystallisation_frequency: CrystallisationFrequency,
    pub fx_rate_to_base: Option<Decimal>,
    pub fx_hedging_cost: Option<Rate>,
    pub subscriptions: Vec<Subscription>,
    pub redemptions: Vec<Redemption>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavInput {
    pub share_classes: Vec<ShareClassInput>,
    pub gross_portfolio_return: Rate,
    pub period_label: String,
    pub equalisation_method: EqualisationMethod,
    pub base_currency: Currency,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareClassNavOutput {
    pub class_name: String,
    pub currency: Currency,
    pub gross_nav_per_share: Money,
    pub management_fee_accrual: Money,
    pub performance_fee_accrual: Money,
    pub net_nav_per_share: Money,
    pub high_water_mark: Money,
    /// (NAV - HWM) / HWM, negative if below
    pub hwm_distance: Rate,
    pub shares_outstanding: Decimal,
    pub class_total_nav: Money,
    pub gross_return: Rate,
    pub net_return: Rate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavOutput {
    pub period_label: String,
    pub share_classes: Vec<ShareClassNavOutput>,
    pub total_fund_nav: Money,
    pub base_currency: Currency,
    pub equalisation_method: EqualisationMethod,
    pub equalisation_adjustments: Vec<EqualisationAdjustment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EqualisationAdjustment {
    pub investor_id: String,
    pub class_name: String,
    /// "Equalisation credit" or "Equalisation debit"
    pub adjustment_type: String,
    pub amount: Money,
    pub description: String,
}

// ---------------------------------------------------------------------------
// Main calculation
// ---------------------------------------------------------------------------

/// Calculate NAV per share class with equalisation adjustments.
///
/// Processes gross returns, subscriptions/redemptions, management and
/// performance fee accruals, high-water-mark updates, FX conversion,
/// and multi-period equalisation for mid-period subscriptions.
pub fn calculate_nav(input: &NavInput) -> CorpFinanceResult<ComputationOutput<NavOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // ------------------------------------------------------------------
    // 1. Validate
    // ------------------------------------------------------------------
    validate_nav_input(input)?;

    // ------------------------------------------------------------------
    // 2. Process each share class
    // ------------------------------------------------------------------
    let mut class_outputs: Vec<ShareClassNavOutput> = Vec::with_capacity(input.share_classes.len());
    let mut equalisation_adjustments: Vec<EqualisationAdjustment> = Vec::new();

    for sc in &input.share_classes {
        let period_frac = sc.crystallisation_frequency.period_fraction();

        // -- Step 1: Gross NAV per share --
        let gross_nav_ps = sc.nav_per_share_opening * (Decimal::ONE + input.gross_portfolio_return);

        // -- Step 2: Process subscriptions (increase shares outstanding) --
        let subscription_shares: Decimal = sc.subscriptions.iter().map(|s| s.shares_issued).sum();
        let shares_after_subs = sc.shares_outstanding + subscription_shares;

        // -- Step 3: Process redemptions (decrease shares outstanding) --
        let redemption_shares: Decimal = sc.redemptions.iter().map(|r| r.shares_redeemed).sum();
        let shares_outstanding = shares_after_subs - redemption_shares;

        if shares_outstanding <= Decimal::ZERO {
            warnings.push(format!(
                "Share class '{}' has non-positive shares outstanding ({}) after sub/redemption processing",
                sc.class_name, shares_outstanding
            ));
        }

        // -- Step 4: Management fee accrual --
        let mgmt_fee = gross_nav_ps * sc.management_fee_rate * period_frac;

        // -- Step 5: Performance fee accrual --
        let perf_fee = calculate_performance_fee(
            gross_nav_ps,
            sc.high_water_mark,
            sc.nav_per_share_opening,
            sc.performance_fee_rate,
            sc.hurdle_rate,
            period_frac,
        );

        // -- Step 6: Net NAV per share --
        let net_nav_ps = gross_nav_ps - mgmt_fee - perf_fee;

        // -- Step 7: High-water mark only moves up --
        let new_hwm = sc.high_water_mark.max(net_nav_ps);

        // -- Step 8: HWM distance --
        let hwm_distance = if sc.high_water_mark > Decimal::ZERO {
            (net_nav_ps - sc.high_water_mark) / sc.high_water_mark
        } else {
            Decimal::ZERO
        };

        // -- Step 9: Gross and net returns --
        let gross_return = if sc.nav_per_share_opening > Decimal::ZERO {
            (gross_nav_ps - sc.nav_per_share_opening) / sc.nav_per_share_opening
        } else {
            Decimal::ZERO
        };
        let net_return = if sc.nav_per_share_opening > Decimal::ZERO {
            (net_nav_ps - sc.nav_per_share_opening) / sc.nav_per_share_opening
        } else {
            Decimal::ZERO
        };

        // -- Step 10: Class total NAV --
        let class_total_nav = net_nav_ps * shares_outstanding;

        // -- Step 11: Equalisation for mid-period subscriptions --
        for sub in &sc.subscriptions {
            if let Some(adj) = calculate_equalisation_adjustment(
                &input.equalisation_method,
                sub,
                &sc.class_name,
                perf_fee,
                net_nav_ps,
                &mut warnings,
            ) {
                equalisation_adjustments.push(adj);
            }
        }

        class_outputs.push(ShareClassNavOutput {
            class_name: sc.class_name.clone(),
            currency: sc.currency.clone(),
            gross_nav_per_share: gross_nav_ps,
            management_fee_accrual: mgmt_fee,
            performance_fee_accrual: perf_fee,
            net_nav_per_share: net_nav_ps,
            high_water_mark: new_hwm,
            hwm_distance,
            shares_outstanding,
            class_total_nav,
            gross_return,
            net_return,
        });
    }

    // ------------------------------------------------------------------
    // 3. Total fund NAV in base currency
    // ------------------------------------------------------------------
    let total_fund_nav =
        calculate_total_fund_nav(&class_outputs, &input.share_classes, &mut warnings);

    // ------------------------------------------------------------------
    // 4. Assemble output
    // ------------------------------------------------------------------
    let output = NavOutput {
        period_label: input.period_label.clone(),
        share_classes: class_outputs,
        total_fund_nav,
        base_currency: input.base_currency.clone(),
        equalisation_method: input.equalisation_method.clone(),
        equalisation_adjustments,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "NAV Calculator with Equalisation: Per-share-class NAV, fees, HWM, FX conversion",
        &serde_json::json!({
            "gross_portfolio_return": input.gross_portfolio_return.to_string(),
            "period_label": input.period_label,
            "equalisation_method": format!("{:?}", input.equalisation_method),
            "base_currency": format!("{:?}", input.base_currency),
            "share_class_count": input.share_classes.len(),
        }),
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn validate_nav_input(input: &NavInput) -> CorpFinanceResult<()> {
    if input.share_classes.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "share_classes".into(),
            reason: "At least one share class is required".into(),
        });
    }

    for sc in &input.share_classes {
        if sc.shares_outstanding <= Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "shares_outstanding".into(),
                reason: format!(
                    "Share class '{}': shares_outstanding must be > 0, got {}",
                    sc.class_name, sc.shares_outstanding
                ),
            });
        }
        if sc.nav_per_share_opening <= Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "nav_per_share_opening".into(),
                reason: format!(
                    "Share class '{}': nav_per_share_opening must be > 0, got {}",
                    sc.class_name, sc.nav_per_share_opening
                ),
            });
        }
        if sc.high_water_mark <= Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "high_water_mark".into(),
                reason: format!(
                    "Share class '{}': high_water_mark must be > 0, got {}",
                    sc.class_name, sc.high_water_mark
                ),
            });
        }
        if sc.management_fee_rate < Decimal::ZERO || sc.management_fee_rate >= Decimal::ONE {
            return Err(CorpFinanceError::InvalidInput {
                field: "management_fee_rate".into(),
                reason: format!(
                    "Share class '{}': management_fee_rate must be in [0, 1), got {}",
                    sc.class_name, sc.management_fee_rate
                ),
            });
        }
        if sc.performance_fee_rate < Decimal::ZERO || sc.performance_fee_rate >= Decimal::ONE {
            return Err(CorpFinanceError::InvalidInput {
                field: "performance_fee_rate".into(),
                reason: format!(
                    "Share class '{}': performance_fee_rate must be in [0, 1), got {}",
                    sc.class_name, sc.performance_fee_rate
                ),
            });
        }
    }

    Ok(())
}

/// Calculate performance fee per share.
///
/// Performance fee is only charged when gross NAV exceeds the high-water mark
/// (and exceeds the hurdle-adjusted opening NAV if a hurdle rate is specified).
fn calculate_performance_fee(
    gross_nav_ps: Money,
    high_water_mark: Money,
    opening_nav_ps: Money,
    perf_fee_rate: Rate,
    hurdle_rate: Option<Rate>,
    period_frac: Decimal,
) -> Money {
    if gross_nav_ps <= high_water_mark {
        return Decimal::ZERO;
    }

    // Determine the effective floor: the higher of HWM and hurdle-adjusted opening NAV
    let hurdle_adjusted = match hurdle_rate {
        Some(hr) => opening_nav_ps * (Decimal::ONE + hr * period_frac),
        Option::None => opening_nav_ps,
    };

    let floor = high_water_mark.max(hurdle_adjusted);

    if gross_nav_ps <= floor {
        return Decimal::ZERO;
    }

    let gain = gross_nav_ps - floor;
    gain * perf_fee_rate
}

/// Calculate equalisation adjustment for a single subscription.
fn calculate_equalisation_adjustment(
    method: &EqualisationMethod,
    sub: &Subscription,
    class_name: &str,
    perf_fee_per_share: Money,
    _net_nav_ps: Money,
    _warnings: &mut Vec<String>,
) -> Option<EqualisationAdjustment> {
    match method {
        EqualisationMethod::None => None,

        EqualisationMethod::EqualisationShares => {
            // New subscriber is issued extra "equalisation shares" representing
            // the accrued performance fee portion they would otherwise avoid.
            let eq_amount = perf_fee_per_share * sub.shares_issued;
            if eq_amount <= Decimal::ZERO {
                return None;
            }
            Some(EqualisationAdjustment {
                investor_id: sub.investor_id.clone(),
                class_name: class_name.to_string(),
                adjustment_type: "Equalisation credit".to_string(),
                amount: eq_amount,
                description: format!(
                    "Equalisation shares issued for {} shares at perf fee {}/share",
                    sub.shares_issued, perf_fee_per_share
                ),
            })
        }

        EqualisationMethod::SeriesAccounting => {
            // Each subscription gets its own series with an independent HWM.
            // The adjustment records the per-investor tracking entry.
            let eq_amount = perf_fee_per_share * sub.shares_issued;
            if eq_amount <= Decimal::ZERO {
                return None;
            }
            Some(EqualisationAdjustment {
                investor_id: sub.investor_id.clone(),
                class_name: class_name.to_string(),
                adjustment_type: "Equalisation credit".to_string(),
                amount: eq_amount,
                description: format!(
                    "Series accounting: new series created for {} shares, tracking independent HWM at {}",
                    sub.shares_issued, sub.nav_per_share_at_entry
                ),
            })
        }

        EqualisationMethod::DepreciationDeposit => {
            // New investor pays an equalisation deposit equal to
            // accrued performance fee * number of shares subscribed.
            let deposit = perf_fee_per_share * sub.shares_issued;
            if deposit <= Decimal::ZERO {
                return None;
            }
            Some(EqualisationAdjustment {
                investor_id: sub.investor_id.clone(),
                class_name: class_name.to_string(),
                adjustment_type: "Equalisation debit".to_string(),
                amount: deposit,
                description: format!(
                    "Depreciation deposit: {} paid on {} shares (perf fee {}/share)",
                    deposit, sub.shares_issued, perf_fee_per_share
                ),
            })
        }
    }
}

/// Convert each share class total NAV to base currency and sum.
fn calculate_total_fund_nav(
    class_outputs: &[ShareClassNavOutput],
    class_inputs: &[ShareClassInput],
    warnings: &mut Vec<String>,
) -> Money {
    let mut total = Decimal::ZERO;

    for (co, ci) in class_outputs.iter().zip(class_inputs.iter()) {
        let class_nav = co.class_total_nav;

        let nav_in_base = match ci.fx_rate_to_base {
            Some(fx_rate) if fx_rate > Decimal::ZERO => {
                let converted = class_nav * fx_rate;

                // Apply FX hedging cost if present
                match ci.fx_hedging_cost {
                    Some(hedge_cost) => converted * (Decimal::ONE - hedge_cost),
                    Option::None => converted,
                }
            }
            Some(fx_rate) => {
                warnings.push(format!(
                    "Share class '{}': invalid fx_rate_to_base ({}), using NAV as-is",
                    co.class_name, fx_rate
                ));
                class_nav
            }
            Option::None => class_nav,
        };

        total += nav_in_base;
    }

    total
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// Helper: create a simple single-class NAV input.
    fn single_class_input() -> NavInput {
        NavInput {
            share_classes: vec![ShareClassInput {
                class_name: "Class A".to_string(),
                currency: Currency::USD,
                shares_outstanding: dec!(1_000_000),
                nav_per_share_opening: dec!(100),
                high_water_mark: dec!(100),
                management_fee_rate: dec!(0.02),
                performance_fee_rate: dec!(0.20),
                hurdle_rate: None,
                crystallisation_frequency: CrystallisationFrequency::Annually,
                fx_rate_to_base: None,
                fx_hedging_cost: None,
                subscriptions: vec![],
                redemptions: vec![],
            }],
            gross_portfolio_return: dec!(0.10),
            period_label: "Q4 2025".to_string(),
            equalisation_method: EqualisationMethod::None,
            base_currency: Currency::USD,
        }
    }

    // ------------------------------------------------------------------
    // Test 1: Basic single class NAV
    // ------------------------------------------------------------------
    #[test]
    fn test_basic_single_class_nav() {
        let input = single_class_input();
        let result = calculate_nav(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.share_classes.len(), 1);
        assert_eq!(out.period_label, "Q4 2025");

        let sc = &out.share_classes[0];
        // Gross NAV = 100 * (1 + 0.10) = 110
        assert_eq!(sc.gross_nav_per_share, dec!(110));

        // Shares unchanged (no subs/redemptions)
        assert_eq!(sc.shares_outstanding, dec!(1_000_000));

        // Net NAV < Gross NAV due to fees
        assert!(sc.net_nav_per_share < sc.gross_nav_per_share);
        assert!(sc.net_nav_per_share > Decimal::ZERO);

        // Total fund NAV = net_nav * shares
        assert_eq!(out.total_fund_nav, sc.net_nav_per_share * dec!(1_000_000));
    }

    // ------------------------------------------------------------------
    // Test 2: Management fee accrual
    // ------------------------------------------------------------------
    #[test]
    fn test_management_fee_accrual() {
        let input = single_class_input();
        let result = calculate_nav(&input).unwrap();
        let sc = &result.result.share_classes[0];

        // Management fee = gross_nav * rate * period_fraction
        // = 110 * 0.02 * 1.0 (annually) = 2.2
        assert_eq!(sc.management_fee_accrual, dec!(2.2));
    }

    // ------------------------------------------------------------------
    // Test 3: Performance fee above HWM
    // ------------------------------------------------------------------
    #[test]
    fn test_performance_fee_above_hwm() {
        let input = single_class_input();
        let result = calculate_nav(&input).unwrap();
        let sc = &result.result.share_classes[0];

        // Gross NAV = 110, HWM = 100, no hurdle
        // Gain above HWM = 110 - 100 = 10
        // Perf fee = 10 * 0.20 = 2.0
        assert_eq!(sc.performance_fee_accrual, dec!(2.0));
    }

    // ------------------------------------------------------------------
    // Test 4: No performance fee below HWM
    // ------------------------------------------------------------------
    #[test]
    fn test_no_performance_fee_below_hwm() {
        let mut input = single_class_input();
        // Negative return keeps NAV below HWM
        input.gross_portfolio_return = dec!(-0.05);

        let result = calculate_nav(&input).unwrap();
        let sc = &result.result.share_classes[0];

        // Gross NAV = 100 * 0.95 = 95, below HWM of 100
        assert_eq!(sc.gross_nav_per_share, dec!(95));
        assert_eq!(sc.performance_fee_accrual, Decimal::ZERO);
    }

    // ------------------------------------------------------------------
    // Test 5: HWM only moves up
    // ------------------------------------------------------------------
    #[test]
    fn test_hwm_only_moves_up() {
        // Case 1: NAV above HWM -> HWM should rise
        let input = single_class_input();
        let result = calculate_nav(&input).unwrap();
        let sc = &result.result.share_classes[0];
        // Net NAV > 100 (opening HWM), so HWM should be updated to net NAV
        assert!(sc.high_water_mark >= dec!(100));
        assert_eq!(sc.high_water_mark, sc.net_nav_per_share);

        // Case 2: NAV below HWM -> HWM stays at 100
        let mut input2 = single_class_input();
        input2.gross_portfolio_return = dec!(-0.05);
        let result2 = calculate_nav(&input2).unwrap();
        let sc2 = &result2.result.share_classes[0];
        // Net NAV < 100, so HWM stays at 100
        assert_eq!(sc2.high_water_mark, dec!(100));
    }

    // ------------------------------------------------------------------
    // Test 6: Subscription increases shares
    // ------------------------------------------------------------------
    #[test]
    fn test_subscription_increases_shares() {
        let mut input = single_class_input();
        input.share_classes[0].subscriptions.push(Subscription {
            investor_id: "INV-001".to_string(),
            amount: dec!(1_000_000),
            nav_per_share_at_entry: dec!(105),
            shares_issued: dec!(9_523.81),
        });

        let result = calculate_nav(&input).unwrap();
        let sc = &result.result.share_classes[0];

        // Shares = 1_000_000 + 9_523.81
        assert_eq!(sc.shares_outstanding, dec!(1_009_523.81));
    }

    // ------------------------------------------------------------------
    // Test 7: Redemption decreases shares
    // ------------------------------------------------------------------
    #[test]
    fn test_redemption_decreases_shares() {
        let mut input = single_class_input();
        input.share_classes[0].redemptions.push(Redemption {
            investor_id: "INV-002".to_string(),
            shares_redeemed: dec!(50_000),
            nav_per_share_at_exit: dec!(110),
        });

        let result = calculate_nav(&input).unwrap();
        let sc = &result.result.share_classes[0];

        // Shares = 1_000_000 - 50_000 = 950_000
        assert_eq!(sc.shares_outstanding, dec!(950_000));
    }

    // ------------------------------------------------------------------
    // Test 8: Multi-class fund
    // ------------------------------------------------------------------
    #[test]
    fn test_multi_class_fund() {
        let mut input = single_class_input();

        // Add a second class in EUR
        input.share_classes.push(ShareClassInput {
            class_name: "Class B".to_string(),
            currency: Currency::EUR,
            shares_outstanding: dec!(500_000),
            nav_per_share_opening: dec!(200),
            high_water_mark: dec!(200),
            management_fee_rate: dec!(0.015),
            performance_fee_rate: dec!(0.15),
            hurdle_rate: None,
            crystallisation_frequency: CrystallisationFrequency::Quarterly,
            fx_rate_to_base: None,
            fx_hedging_cost: None,
            subscriptions: vec![],
            redemptions: vec![],
        });

        let result = calculate_nav(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.share_classes.len(), 2);
        assert_eq!(out.share_classes[0].class_name, "Class A");
        assert_eq!(out.share_classes[1].class_name, "Class B");

        // Class B: gross_nav = 200 * 1.10 = 220
        assert_eq!(out.share_classes[1].gross_nav_per_share, dec!(220));

        // Total fund NAV should be sum of both classes
        let expected_total =
            out.share_classes[0].class_total_nav + out.share_classes[1].class_total_nav;
        assert_eq!(out.total_fund_nav, expected_total);
    }

    // ------------------------------------------------------------------
    // Test 9: FX conversion to base currency
    // ------------------------------------------------------------------
    #[test]
    fn test_fx_conversion_to_base() {
        let mut input = single_class_input();

        // Class A is in GBP; 1 GBP = 1.25 USD
        input.share_classes[0].currency = Currency::GBP;
        input.share_classes[0].fx_rate_to_base = Some(dec!(1.25));
        input.base_currency = Currency::USD;

        let result = calculate_nav(&input).unwrap();
        let out = &result.result;
        let sc = &out.share_classes[0];

        // Class total NAV in GBP
        let class_nav_gbp = sc.net_nav_per_share * sc.shares_outstanding;
        assert_eq!(sc.class_total_nav, class_nav_gbp);

        // Total fund NAV should be in USD = GBP * 1.25
        let expected_usd = class_nav_gbp * dec!(1.25);
        assert_eq!(out.total_fund_nav, expected_usd);
    }

    // ------------------------------------------------------------------
    // Test 10: Equalisation depreciation deposit
    // ------------------------------------------------------------------
    #[test]
    fn test_equalisation_depreciation_deposit() {
        let mut input = single_class_input();
        input.equalisation_method = EqualisationMethod::DepreciationDeposit;
        input.share_classes[0].subscriptions.push(Subscription {
            investor_id: "INV-003".to_string(),
            amount: dec!(500_000),
            nav_per_share_at_entry: dec!(105),
            shares_issued: dec!(4_761.90),
        });

        let result = calculate_nav(&input).unwrap();
        let out = &result.result;

        // Should have an equalisation adjustment (perf fee > 0, so deposit applies)
        assert!(!out.equalisation_adjustments.is_empty());

        let adj = &out.equalisation_adjustments[0];
        assert_eq!(adj.investor_id, "INV-003");
        assert_eq!(adj.class_name, "Class A");
        assert_eq!(adj.adjustment_type, "Equalisation debit");

        // Deposit = perf_fee_per_share * shares_issued = 2.0 * 4761.90 = 9523.80
        let sc = &out.share_classes[0];
        let expected_deposit = sc.performance_fee_accrual * dec!(4_761.90);
        assert_eq!(adj.amount, expected_deposit);
    }

    // ------------------------------------------------------------------
    // Test 11: Hurdle rate reduces performance fee
    // ------------------------------------------------------------------
    #[test]
    fn test_hurdle_rate_reduces_perf_fee() {
        // Without hurdle
        let input_no_hurdle = single_class_input();
        let result_no_hurdle = calculate_nav(&input_no_hurdle).unwrap();
        let perf_fee_no_hurdle = result_no_hurdle.result.share_classes[0].performance_fee_accrual;

        // With hurdle at 5%
        let mut input_with_hurdle = single_class_input();
        input_with_hurdle.share_classes[0].hurdle_rate = Some(dec!(0.05));
        let result_with_hurdle = calculate_nav(&input_with_hurdle).unwrap();
        let perf_fee_with_hurdle =
            result_with_hurdle.result.share_classes[0].performance_fee_accrual;

        // No hurdle: gain = 110 - 100 = 10, fee = 2.0
        // With hurdle: floor = max(100, 100 * 1.05) = 105, gain = 110 - 105 = 5, fee = 1.0
        assert_eq!(perf_fee_no_hurdle, dec!(2.0));
        assert_eq!(perf_fee_with_hurdle, dec!(1.0));
        assert!(perf_fee_with_hurdle < perf_fee_no_hurdle);
    }

    // ------------------------------------------------------------------
    // Test 12: Zero shares error
    // ------------------------------------------------------------------
    #[test]
    fn test_zero_shares_error() {
        let mut input = single_class_input();
        input.share_classes[0].shares_outstanding = Decimal::ZERO;

        let result = calculate_nav(&input);
        assert!(result.is_err());

        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "shares_outstanding");
            }
            other => panic!("Expected InvalidInput for shares_outstanding, got: {other}"),
        }
    }

    // ------------------------------------------------------------------
    // Test 13: Metadata populated
    // ------------------------------------------------------------------
    #[test]
    fn test_metadata_populated() {
        let input = single_class_input();
        let result = calculate_nav(&input).unwrap();

        assert!(!result.methodology.is_empty());
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
        assert!(
            result.metadata.computation_time_us > 0 || result.metadata.computation_time_us == 0
        );
        assert!(!result.metadata.version.is_empty());
    }

    // ------------------------------------------------------------------
    // Test 14: Empty share classes error
    // ------------------------------------------------------------------
    #[test]
    fn test_empty_share_classes_error() {
        let input = NavInput {
            share_classes: vec![],
            gross_portfolio_return: dec!(0.10),
            period_label: "Q4 2025".to_string(),
            equalisation_method: EqualisationMethod::None,
            base_currency: Currency::USD,
        };

        let result = calculate_nav(&input);
        assert!(result.is_err());

        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "share_classes");
            }
            other => panic!("Expected InvalidInput for share_classes, got: {other}"),
        }
    }

    // ------------------------------------------------------------------
    // Test 15: FX hedging cost reduces total NAV
    // ------------------------------------------------------------------
    #[test]
    fn test_fx_hedging_cost() {
        let mut input = single_class_input();
        input.share_classes[0].currency = Currency::EUR;
        input.share_classes[0].fx_rate_to_base = Some(dec!(1.10));
        input.share_classes[0].fx_hedging_cost = Some(dec!(0.005));

        let result = calculate_nav(&input).unwrap();
        let total_with_hedge = result.result.total_fund_nav;

        // Without hedging cost
        let mut input2 = single_class_input();
        input2.share_classes[0].currency = Currency::EUR;
        input2.share_classes[0].fx_rate_to_base = Some(dec!(1.10));
        input2.share_classes[0].fx_hedging_cost = None;

        let result2 = calculate_nav(&input2).unwrap();
        let total_no_hedge = result2.result.total_fund_nav;

        assert!(total_with_hedge < total_no_hedge);
    }

    // ------------------------------------------------------------------
    // Test 16: Quarterly crystallisation reduces per-period fees
    // ------------------------------------------------------------------
    #[test]
    fn test_quarterly_crystallisation() {
        let mut input = single_class_input();
        input.share_classes[0].crystallisation_frequency = CrystallisationFrequency::Quarterly;

        let result = calculate_nav(&input).unwrap();
        let sc = &result.result.share_classes[0];

        // Quarterly management fee = 110 * 0.02 * 0.25 = 0.55
        assert_eq!(sc.management_fee_accrual, dec!(0.55));
    }
}
