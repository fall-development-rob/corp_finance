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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccountingStandard {
    UsGaap,
    Ifrs,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdjustmentCategory {
    LeaseCapitalisation,
    LifoAdjustment,
    DevelopmentCosts,
    RevaluationStrip,
    ContingencyRecognition,
    PensionNormalisation,
    OtherGaapDifference,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationInput {
    pub source_standard: AccountingStandard,
    pub target_standard: AccountingStandard,
    pub revenue: Money,
    pub ebitda: Money,
    pub ebit: Money,
    pub net_income: Money,
    pub total_assets: Money,
    pub total_debt: Money,
    pub total_equity: Money,
    pub inventory: Money,
    pub ppe_net: Money,
    // Adjustment inputs (all Option)
    pub operating_lease_payments: Option<Money>,
    pub operating_lease_remaining_years: Option<u32>,
    pub lifo_reserve: Option<Money>,
    pub capitalised_dev_costs: Option<Money>,
    pub dev_cost_amortisation: Option<Money>,
    pub revaluation_surplus: Option<Money>,
    pub discount_rate_for_leases: Option<Rate>,
    pub currency: Option<Currency>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationAdjustment {
    pub name: String,
    pub category: AdjustmentCategory,
    pub impact_ebitda: Money,
    pub impact_debt: Money,
    pub impact_assets: Money,
    pub impact_equity: Money,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationOutput {
    pub source_standard: AccountingStandard,
    pub target_standard: AccountingStandard,
    pub adjusted_ebitda: Money,
    pub adjusted_ebit: Money,
    pub adjusted_net_income: Money,
    pub adjusted_total_debt: Money,
    pub adjusted_total_equity: Money,
    pub adjusted_total_assets: Money,
    pub adjustments: Vec<ReconciliationAdjustment>,
    pub materiality_flag: bool,
    pub total_adjustment_magnitude: Money,
}

// ---------------------------------------------------------------------------
// Main calculation
// ---------------------------------------------------------------------------

/// Reconcile financial statements between US GAAP and IFRS.
///
/// Applies standard adjustments for lease capitalisation, LIFO reserves,
/// development cost capitalisation, and revaluation surplus to convert
/// reported financials from one accounting standard to another.
pub fn reconcile_accounting_standards(
    input: &ReconciliationInput,
) -> CorpFinanceResult<ComputationOutput<ReconciliationOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // ------------------------------------------------------------------
    // 1. Validate inputs
    // ------------------------------------------------------------------
    validate_input(input, &mut warnings)?;

    // Same-standard short circuit
    if input.source_standard == input.target_standard {
        warnings.push(
            "Source and target standards are the same; returning unadjusted figures.".to_string(),
        );
        let output = ReconciliationOutput {
            source_standard: input.source_standard.clone(),
            target_standard: input.target_standard.clone(),
            adjusted_ebitda: input.ebitda,
            adjusted_ebit: input.ebit,
            adjusted_net_income: input.net_income,
            adjusted_total_debt: input.total_debt,
            adjusted_total_equity: input.total_equity,
            adjusted_total_assets: input.total_assets,
            adjustments: Vec::new(),
            materiality_flag: false,
            total_adjustment_magnitude: Decimal::ZERO,
        };
        let elapsed = start.elapsed().as_micros() as u64;
        return Ok(with_metadata(
            "GAAP/IFRS Reconciliation (no adjustments — same standard)",
            &build_assumptions(input),
            warnings,
            elapsed,
            output,
        ));
    }

    // ------------------------------------------------------------------
    // 2. Running totals for adjusted figures
    // ------------------------------------------------------------------
    let mut adj_ebitda = input.ebitda;
    let mut adj_ebit = input.ebit;
    let mut adj_net_income = input.net_income;
    let mut adj_debt = input.total_debt;
    let mut adj_equity = input.total_equity;
    let mut adj_assets = input.total_assets;
    let mut adjustments: Vec<ReconciliationAdjustment> = Vec::new();
    let mut total_magnitude = Decimal::ZERO;

    // ------------------------------------------------------------------
    // 3. Lease Capitalisation
    // ------------------------------------------------------------------
    if let Some(lease_adj) = compute_lease_adjustment(input, &mut warnings) {
        adj_ebitda += lease_adj.impact_ebitda;
        adj_debt += lease_adj.impact_debt;
        adj_assets += lease_adj.impact_assets;
        adj_equity += lease_adj.impact_equity;
        // Lease capitalisation impacts EBITDA but not EBIT directly in the
        // simple model (depreciation + interest roughly offsets rent expense
        // at EBIT level). Net income impact is negligible in approximation.
        total_magnitude += lease_adj.impact_assets.abs()
            + lease_adj.impact_debt.abs()
            + lease_adj.impact_ebitda.abs()
            + lease_adj.impact_equity.abs();
        adjustments.push(lease_adj);
    }

    // ------------------------------------------------------------------
    // 4. LIFO Adjustment
    // ------------------------------------------------------------------
    if let Some(lifo_adj) = compute_lifo_adjustment(input) {
        adj_ebitda += lifo_adj.impact_ebitda;
        adj_debt += lifo_adj.impact_debt;
        adj_assets += lifo_adj.impact_assets;
        adj_equity += lifo_adj.impact_equity;
        total_magnitude += lifo_adj.impact_assets.abs()
            + lifo_adj.impact_debt.abs()
            + lifo_adj.impact_ebitda.abs()
            + lifo_adj.impact_equity.abs();
        adjustments.push(lifo_adj);
    }

    // ------------------------------------------------------------------
    // 5. Development Cost Capitalisation
    // ------------------------------------------------------------------
    if let Some(dev_adj) = compute_dev_cost_adjustment(input) {
        adj_ebitda += dev_adj.impact_ebitda;
        adj_ebit += dev_adj.impact_ebitda; // EBIT also changes via dev cost
                                           // More precise EBIT adjustment for dev costs
        adj_ebit += compute_dev_cost_ebit_delta(input);
        adj_ebitda -= dev_adj.impact_ebitda; // undo double-count, re-apply properly below
        let (ebitda_impact, ebit_impact) = compute_dev_cost_pnl_impacts(input);
        adj_ebitda += ebitda_impact;
        // Undo the rough adj_ebit above and apply precise
        adj_ebit -= dev_adj.impact_ebitda;
        adj_ebit -= compute_dev_cost_ebit_delta(input);
        adj_ebit += ebit_impact;
        adj_assets += dev_adj.impact_assets;
        adj_equity += dev_adj.impact_equity;
        adj_net_income += ebit_impact; // Approximate net income impact
        total_magnitude += dev_adj.impact_assets.abs()
            + dev_adj.impact_debt.abs()
            + dev_adj.impact_ebitda.abs()
            + dev_adj.impact_equity.abs();
        adjustments.push(dev_adj);
    }

    // ------------------------------------------------------------------
    // 6. Revaluation Strip
    // ------------------------------------------------------------------
    if let Some(reval_adj) = compute_revaluation_adjustment(input) {
        adj_ebitda += reval_adj.impact_ebitda;
        adj_debt += reval_adj.impact_debt;
        adj_assets += reval_adj.impact_assets;
        adj_equity += reval_adj.impact_equity;
        total_magnitude += reval_adj.impact_assets.abs()
            + reval_adj.impact_debt.abs()
            + reval_adj.impact_ebitda.abs()
            + reval_adj.impact_equity.abs();
        adjustments.push(reval_adj);
    }

    // ------------------------------------------------------------------
    // 7. Materiality check
    // ------------------------------------------------------------------
    let materiality_flag = if input.total_assets > Decimal::ZERO {
        total_magnitude > input.total_assets * dec!(0.02)
    } else {
        false
    };

    // ------------------------------------------------------------------
    // 8. Assemble output
    // ------------------------------------------------------------------
    let output = ReconciliationOutput {
        source_standard: input.source_standard.clone(),
        target_standard: input.target_standard.clone(),
        adjusted_ebitda: adj_ebitda,
        adjusted_ebit: adj_ebit,
        adjusted_net_income: adj_net_income,
        adjusted_total_debt: adj_debt,
        adjusted_total_equity: adj_equity,
        adjusted_total_assets: adj_assets,
        adjustments,
        materiality_flag,
        total_adjustment_magnitude: total_magnitude,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "GAAP/IFRS Reconciliation: Lease, LIFO, Dev Cost, Revaluation Adjustments",
        &build_assumptions(input),
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn validate_input(
    input: &ReconciliationInput,
    _warnings: &mut Vec<String>,
) -> CorpFinanceResult<()> {
    if input.total_assets <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "total_assets".into(),
            reason: "Total assets must be positive".into(),
        });
    }
    if input.revenue < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "revenue".into(),
            reason: "Revenue must be non-negative".into(),
        });
    }
    if input.total_equity <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "total_equity".into(),
            reason: "Total equity must be positive".into(),
        });
    }
    Ok(())
}

fn build_assumptions(input: &ReconciliationInput) -> serde_json::Value {
    serde_json::json!({
        "source_standard": format!("{:?}", input.source_standard),
        "target_standard": format!("{:?}", input.target_standard),
        "revenue": input.revenue.to_string(),
        "ebitda": input.ebitda.to_string(),
        "total_assets": input.total_assets.to_string(),
        "assumed_tax_rate": "0.25",
        "default_lease_discount_rate": "0.05",
    })
}

/// Compute PV of an annuity: payment * ((1 - (1+r)^-n) / r)
/// Uses iterative multiplication to avoid powd precision drift.
fn pv_annuity(payment: Decimal, rate: Decimal, periods: u32) -> Decimal {
    if rate == Decimal::ZERO || periods == 0 {
        return payment * Decimal::from(periods);
    }
    // (1 + r)^n via iterative multiplication
    let one_plus_r = Decimal::ONE + rate;
    let mut compounded = Decimal::ONE;
    for _ in 0..periods {
        compounded *= one_plus_r;
    }
    // PV factor = (1 - 1/(1+r)^n) / r
    let pv_factor = (Decimal::ONE - Decimal::ONE / compounded) / rate;
    payment * pv_factor
}

/// Lease capitalisation adjustment.
///
/// GAAP->IFRS: capitalise operating leases (IFRS 16 requires it).
/// IFRS->GAAP: no adjustment needed (leases already capitalised under IFRS).
fn compute_lease_adjustment(
    input: &ReconciliationInput,
    warnings: &mut Vec<String>,
) -> Option<ReconciliationAdjustment> {
    // Only applies when converting GAAP -> IFRS
    if input.source_standard != AccountingStandard::UsGaap
        || input.target_standard != AccountingStandard::Ifrs
    {
        return None;
    }

    let annual_payment = input.operating_lease_payments?;
    let remaining_years = input.operating_lease_remaining_years?;

    if remaining_years == 0 || annual_payment <= Decimal::ZERO {
        return None;
    }

    let discount_rate = input.discount_rate_for_leases.unwrap_or(dec!(0.05));

    if discount_rate <= Decimal::ZERO {
        warnings.push("Lease discount rate is zero or negative; using 5% default.".to_string());
    }

    let effective_rate = if discount_rate <= Decimal::ZERO {
        dec!(0.05)
    } else {
        discount_rate
    };

    let lease_liability = pv_annuity(annual_payment, effective_rate, remaining_years);

    // ROU asset approximated equal to lease liability at inception
    let rou_asset = lease_liability;

    // EBITDA impact: operating lease payments were an opex deduction.
    // Under IFRS 16, they are replaced by depreciation (below EBITDA)
    // + interest (below EBITDA), so EBITDA increases by the annual payment.
    let ebitda_impact = annual_payment;

    Some(ReconciliationAdjustment {
        name: "Lease Capitalisation (IFRS 16)".to_string(),
        category: AdjustmentCategory::LeaseCapitalisation,
        impact_ebitda: ebitda_impact,
        impact_debt: lease_liability,
        impact_assets: rou_asset,
        impact_equity: Decimal::ZERO, // Assets and liabilities increase equally
        description: format!(
            "Capitalise operating leases: PV of {} annual payments over {} years at {}% = {} lease liability / ROU asset. \
             EBITDA increases by {} (rent replaced by depreciation + interest below EBITDA).",
            annual_payment, remaining_years,
            (effective_rate * dec!(100)).round_dp(1),
            lease_liability.round_dp(0),
            annual_payment,
        ),
    })
}

/// LIFO reserve adjustment.
///
/// GAAP->IFRS: add LIFO reserve to inventory and after-tax portion to equity.
/// IFRS->GAAP: no adjustment (IFRS doesn't allow LIFO, so there's no reserve).
fn compute_lifo_adjustment(input: &ReconciliationInput) -> Option<ReconciliationAdjustment> {
    // Only applies GAAP -> IFRS
    if input.source_standard != AccountingStandard::UsGaap
        || input.target_standard != AccountingStandard::Ifrs
    {
        return None;
    }

    let lifo_reserve = input.lifo_reserve?;
    if lifo_reserve == Decimal::ZERO {
        return None;
    }

    let assumed_tax_rate = dec!(0.25);
    let after_tax_equity_impact = lifo_reserve * (Decimal::ONE - assumed_tax_rate);

    Some(ReconciliationAdjustment {
        name: "LIFO to FIFO Adjustment".to_string(),
        category: AdjustmentCategory::LifoAdjustment,
        impact_ebitda: Decimal::ZERO,
        impact_debt: Decimal::ZERO,
        impact_assets: lifo_reserve,
        impact_equity: after_tax_equity_impact,
        description: format!(
            "Convert LIFO inventory to FIFO: inventory +{}, equity +{} (after-tax at 25% rate).",
            lifo_reserve,
            after_tax_equity_impact.round_dp(2),
        ),
    })
}

/// Development cost capitalisation adjustment.
///
/// GAAP->IFRS: capitalise qualifying development costs (IFRS allows, GAAP expenses).
/// IFRS->GAAP: reverse capitalisation (expense dev costs).
fn compute_dev_cost_adjustment(input: &ReconciliationInput) -> Option<ReconciliationAdjustment> {
    let capitalised = input.capitalised_dev_costs?;
    if capitalised == Decimal::ZERO {
        return None;
    }

    let amortisation = input.dev_cost_amortisation.unwrap_or(Decimal::ZERO);

    match (&input.source_standard, &input.target_standard) {
        (AccountingStandard::UsGaap, AccountingStandard::Ifrs) => {
            // Capitalise: assets increase, EBITDA increases (expense removed),
            // EBIT increases by net of capitalisation minus amortisation
            Some(ReconciliationAdjustment {
                name: "Development Cost Capitalisation (IAS 38)".to_string(),
                category: AdjustmentCategory::DevelopmentCosts,
                impact_ebitda: amortisation, // Placeholder; real P&L impacts computed separately
                impact_debt: Decimal::ZERO,
                impact_assets: capitalised,
                impact_equity: capitalised - amortisation, // Net book value increase
                description: format!(
                    "Capitalise development costs under IAS 38: assets +{}, \
                     EBITDA +{} (expensed R&D now capitalised), \
                     EBIT +{} (capitalised {} less amortisation {}).",
                    capitalised,
                    amortisation,
                    capitalised - amortisation,
                    capitalised,
                    amortisation,
                ),
            })
        }
        (AccountingStandard::Ifrs, AccountingStandard::UsGaap) => {
            // Reverse: assets decrease, EBITDA decreases, EBIT decreases
            Some(ReconciliationAdjustment {
                name: "Reverse Development Cost Capitalisation".to_string(),
                category: AdjustmentCategory::DevelopmentCosts,
                impact_ebitda: -amortisation,
                impact_debt: Decimal::ZERO,
                impact_assets: -capitalised,
                impact_equity: -(capitalised - amortisation),
                description: format!(
                    "Expense development costs under US GAAP: assets -{}, \
                     EBITDA -{} (capitalised R&D expensed), \
                     EBIT -{} (reverse capitalised {} and amortisation {}).",
                    capitalised,
                    amortisation,
                    capitalised - amortisation,
                    capitalised,
                    amortisation,
                ),
            })
        }
        _ => None,
    }
}

/// Compute the EBITDA and EBIT deltas for development cost adjustments.
fn compute_dev_cost_pnl_impacts(input: &ReconciliationInput) -> (Money, Money) {
    let capitalised = input.capitalised_dev_costs.unwrap_or(Decimal::ZERO);
    let amortisation = input.dev_cost_amortisation.unwrap_or(Decimal::ZERO);

    match (&input.source_standard, &input.target_standard) {
        (AccountingStandard::UsGaap, AccountingStandard::Ifrs) => {
            // EBITDA: add back the R&D expense that is now capitalised
            // (in the period, the full dev spend was expensed; now only amortisation hits P&L)
            // EBIT: net impact is capitalised - amortisation (new asset less depreciation)
            (amortisation, capitalised - amortisation)
        }
        (AccountingStandard::Ifrs, AccountingStandard::UsGaap) => {
            (-amortisation, -(capitalised - amortisation))
        }
        _ => (Decimal::ZERO, Decimal::ZERO),
    }
}

/// Helper for the dev cost EBIT delta (used in the main function).
fn compute_dev_cost_ebit_delta(input: &ReconciliationInput) -> Money {
    let capitalised = input.capitalised_dev_costs.unwrap_or(Decimal::ZERO);
    let amortisation = input.dev_cost_amortisation.unwrap_or(Decimal::ZERO);

    match (&input.source_standard, &input.target_standard) {
        (AccountingStandard::UsGaap, AccountingStandard::Ifrs) => capitalised - amortisation,
        (AccountingStandard::Ifrs, AccountingStandard::UsGaap) => -(capitalised - amortisation),
        _ => Decimal::ZERO,
    }
}

/// Revaluation surplus strip.
///
/// IFRS->GAAP: remove revaluation surplus (US GAAP uses historical cost).
/// GAAP->IFRS: no adjustment (revaluation requires independent appraisal).
fn compute_revaluation_adjustment(input: &ReconciliationInput) -> Option<ReconciliationAdjustment> {
    // Only applies IFRS -> GAAP
    if input.source_standard != AccountingStandard::Ifrs
        || input.target_standard != AccountingStandard::UsGaap
    {
        return None;
    }

    let surplus = input.revaluation_surplus?;
    if surplus == Decimal::ZERO {
        return None;
    }

    Some(ReconciliationAdjustment {
        name: "Revaluation Surplus Strip".to_string(),
        category: AdjustmentCategory::RevaluationStrip,
        impact_ebitda: Decimal::ZERO,
        impact_debt: Decimal::ZERO,
        impact_assets: -surplus,
        impact_equity: -surplus,
        description: format!(
            "Strip revaluation surplus for US GAAP historical cost: assets -{}, equity -{}.",
            surplus, surplus,
        ),
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// Helper: standard input for GAAP -> IFRS conversion.
    fn gaap_to_ifrs_input() -> ReconciliationInput {
        ReconciliationInput {
            source_standard: AccountingStandard::UsGaap,
            target_standard: AccountingStandard::Ifrs,
            revenue: dec!(1_000_000),
            ebitda: dec!(200_000),
            ebit: dec!(150_000),
            net_income: dec!(100_000),
            total_assets: dec!(5_000_000),
            total_debt: dec!(2_000_000),
            total_equity: dec!(3_000_000),
            inventory: dec!(500_000),
            ppe_net: dec!(1_500_000),
            operating_lease_payments: Some(dec!(50_000)),
            operating_lease_remaining_years: Some(5),
            lifo_reserve: Some(dec!(100_000)),
            capitalised_dev_costs: Some(dec!(200_000)),
            dev_cost_amortisation: Some(dec!(40_000)),
            revaluation_surplus: None,
            discount_rate_for_leases: Some(dec!(0.05)),
            currency: Some(Currency::USD),
        }
    }

    /// Helper: standard input for IFRS -> GAAP conversion.
    fn ifrs_to_gaap_input() -> ReconciliationInput {
        ReconciliationInput {
            source_standard: AccountingStandard::Ifrs,
            target_standard: AccountingStandard::UsGaap,
            revenue: dec!(1_000_000),
            ebitda: dec!(250_000),
            ebit: dec!(190_000),
            net_income: dec!(120_000),
            total_assets: dec!(5_500_000),
            total_debt: dec!(2_200_000),
            total_equity: dec!(3_300_000),
            inventory: dec!(500_000),
            ppe_net: dec!(1_800_000),
            operating_lease_payments: Some(dec!(50_000)),
            operating_lease_remaining_years: Some(5),
            lifo_reserve: None,
            capitalised_dev_costs: Some(dec!(200_000)),
            dev_cost_amortisation: Some(dec!(40_000)),
            revaluation_surplus: Some(dec!(300_000)),
            discount_rate_for_leases: Some(dec!(0.05)),
            currency: Some(Currency::USD),
        }
    }

    // ------------------------------------------------------------------
    // Test 1: IFRS -> GAAP lease: no change (leases already capitalised)
    // ------------------------------------------------------------------
    #[test]
    fn test_ifrs_to_gaap_lease_no_change() {
        let input = ifrs_to_gaap_input();
        let result = reconcile_accounting_standards(&input).unwrap();
        let out = &result.result;

        // No lease capitalisation adjustment should be present (IFRS already capitalises)
        let lease_adj = out
            .adjustments
            .iter()
            .find(|a| a.category == AdjustmentCategory::LeaseCapitalisation);
        assert!(
            lease_adj.is_none(),
            "IFRS->GAAP should not produce a lease capitalisation adjustment"
        );
    }

    // ------------------------------------------------------------------
    // Test 2: GAAP -> IFRS lease capitalisation
    // ------------------------------------------------------------------
    #[test]
    fn test_gaap_to_ifrs_lease_capitalisation() {
        let mut input = gaap_to_ifrs_input();
        // Remove other adjustments to isolate lease effect
        input.lifo_reserve = None;
        input.capitalised_dev_costs = None;

        let result = reconcile_accounting_standards(&input).unwrap();
        let out = &result.result;

        let lease_adj = out
            .adjustments
            .iter()
            .find(|a| a.category == AdjustmentCategory::LeaseCapitalisation)
            .expect("Should have a lease capitalisation adjustment");

        // PV of 50,000/yr for 5 years at 5%
        // PV = 50000 * ((1 - 1.05^-5) / 0.05) = 50000 * 4.329... ≈ 216,473
        assert!(
            lease_adj.impact_debt > dec!(200_000),
            "Lease liability should be > 200k, got {}",
            lease_adj.impact_debt
        );
        assert!(
            lease_adj.impact_debt < dec!(250_000),
            "Lease liability should be < 250k, got {}",
            lease_adj.impact_debt
        );

        // ROU asset should equal lease liability
        assert_eq!(
            lease_adj.impact_assets, lease_adj.impact_debt,
            "ROU asset should equal lease liability"
        );

        // EBITDA should increase by the annual lease payment
        assert_eq!(
            lease_adj.impact_ebitda,
            dec!(50_000),
            "EBITDA impact should equal annual lease payment"
        );

        // Equity impact is zero (asset and liability increase equally)
        assert_eq!(lease_adj.impact_equity, Decimal::ZERO);

        // Adjusted figures should reflect the changes
        assert!(out.adjusted_ebitda > input.ebitda);
        assert!(out.adjusted_total_debt > input.total_debt);
        assert!(out.adjusted_total_assets > input.total_assets);
    }

    // ------------------------------------------------------------------
    // Test 3: LIFO adjustment GAAP -> IFRS
    // ------------------------------------------------------------------
    #[test]
    fn test_lifo_adjustment_gaap_to_ifrs() {
        let mut input = gaap_to_ifrs_input();
        input.operating_lease_payments = None;
        input.capitalised_dev_costs = None;
        input.lifo_reserve = Some(dec!(100_000));

        let result = reconcile_accounting_standards(&input).unwrap();
        let out = &result.result;

        let lifo_adj = out
            .adjustments
            .iter()
            .find(|a| a.category == AdjustmentCategory::LifoAdjustment)
            .expect("Should have a LIFO adjustment");

        // Assets increase by LIFO reserve
        assert_eq!(lifo_adj.impact_assets, dec!(100_000));

        // Equity increases by after-tax portion (75% of 100k = 75k)
        assert_eq!(lifo_adj.impact_equity, dec!(75_000));

        // No debt or EBITDA impact
        assert_eq!(lifo_adj.impact_debt, Decimal::ZERO);
        assert_eq!(lifo_adj.impact_ebitda, Decimal::ZERO);
    }

    // ------------------------------------------------------------------
    // Test 4: LIFO adjustment IFRS -> GAAP: no change
    // ------------------------------------------------------------------
    #[test]
    fn test_lifo_adjustment_ifrs_to_gaap_no_change() {
        let mut input = ifrs_to_gaap_input();
        input.lifo_reserve = Some(dec!(100_000)); // Even if provided, shouldn't apply

        let result = reconcile_accounting_standards(&input).unwrap();
        let out = &result.result;

        let lifo_adj = out
            .adjustments
            .iter()
            .find(|a| a.category == AdjustmentCategory::LifoAdjustment);
        assert!(
            lifo_adj.is_none(),
            "IFRS->GAAP should not produce a LIFO adjustment"
        );
    }

    // ------------------------------------------------------------------
    // Test 5: Development cost capitalisation GAAP -> IFRS
    // ------------------------------------------------------------------
    #[test]
    fn test_development_cost_capitalisation() {
        let mut input = gaap_to_ifrs_input();
        input.operating_lease_payments = None;
        input.lifo_reserve = None;
        input.capitalised_dev_costs = Some(dec!(200_000));
        input.dev_cost_amortisation = Some(dec!(40_000));

        let result = reconcile_accounting_standards(&input).unwrap();
        let out = &result.result;

        let dev_adj = out
            .adjustments
            .iter()
            .find(|a| a.category == AdjustmentCategory::DevelopmentCosts)
            .expect("Should have a dev cost adjustment");

        // Assets increase by capitalised amount
        assert_eq!(dev_adj.impact_assets, dec!(200_000));

        // Equity increases by net book value (capitalised - amortisation)
        assert_eq!(dev_adj.impact_equity, dec!(160_000));

        // Adjusted assets should be higher
        assert_eq!(
            out.adjusted_total_assets,
            input.total_assets + dec!(200_000)
        );

        // Adjusted EBITDA should be higher (expense removed)
        assert!(
            out.adjusted_ebitda > input.ebitda,
            "EBITDA should increase when dev costs are capitalised"
        );

        // Adjusted EBIT should reflect net capitalisation effect
        assert!(
            out.adjusted_ebit > input.ebit,
            "EBIT should increase by capitalised - amortisation"
        );
    }

    // ------------------------------------------------------------------
    // Test 6: Revaluation strip IFRS -> GAAP
    // ------------------------------------------------------------------
    #[test]
    fn test_revaluation_strip_ifrs_to_gaap() {
        let mut input = ifrs_to_gaap_input();
        input.capitalised_dev_costs = None;
        input.revaluation_surplus = Some(dec!(300_000));

        let result = reconcile_accounting_standards(&input).unwrap();
        let out = &result.result;

        let reval_adj = out
            .adjustments
            .iter()
            .find(|a| a.category == AdjustmentCategory::RevaluationStrip)
            .expect("Should have a revaluation strip adjustment");

        // Assets decrease by surplus
        assert_eq!(reval_adj.impact_assets, dec!(-300_000));

        // Equity decreases by surplus
        assert_eq!(reval_adj.impact_equity, dec!(-300_000));

        // No EBITDA or debt impact
        assert_eq!(reval_adj.impact_ebitda, Decimal::ZERO);
        assert_eq!(reval_adj.impact_debt, Decimal::ZERO);

        // Adjusted totals should reflect
        assert_eq!(
            out.adjusted_total_assets,
            input.total_assets - dec!(300_000)
        );
        assert_eq!(
            out.adjusted_total_equity,
            input.total_equity - dec!(300_000)
        );
    }

    // ------------------------------------------------------------------
    // Test 7: Combined adjustments GAAP -> IFRS
    // ------------------------------------------------------------------
    #[test]
    fn test_combined_adjustments() {
        let input = gaap_to_ifrs_input();
        let result = reconcile_accounting_standards(&input).unwrap();
        let out = &result.result;

        // Should have 3 adjustments: lease, LIFO, dev cost (no revaluation for GAAP->IFRS)
        assert_eq!(
            out.adjustments.len(),
            3,
            "GAAP->IFRS with all inputs should produce 3 adjustments, got {}",
            out.adjustments.len()
        );

        // All adjusted figures should differ from input
        assert_ne!(out.adjusted_ebitda, input.ebitda);
        assert_ne!(out.adjusted_total_debt, input.total_debt);
        assert_ne!(out.adjusted_total_assets, input.total_assets);
        assert_ne!(out.adjusted_total_equity, input.total_equity);

        // Total adjustment magnitude should be positive
        assert!(out.total_adjustment_magnitude > Decimal::ZERO);
    }

    // ------------------------------------------------------------------
    // Test 8: Materiality flag triggered
    // ------------------------------------------------------------------
    #[test]
    fn test_materiality_flag_triggered() {
        let mut input = gaap_to_ifrs_input();
        // Use a small total_assets to ensure >2% threshold is crossed
        input.total_assets = dec!(100_000);
        input.total_equity = dec!(50_000);
        input.lifo_reserve = Some(dec!(50_000)); // 50% of total assets
        input.operating_lease_payments = Some(dec!(20_000));
        input.operating_lease_remaining_years = Some(5);

        let result = reconcile_accounting_standards(&input).unwrap();
        let out = &result.result;

        assert!(
            out.materiality_flag,
            "Materiality flag should be triggered when adjustments > 2% of total assets"
        );
    }

    // ------------------------------------------------------------------
    // Test 9: Materiality flag NOT triggered
    // ------------------------------------------------------------------
    #[test]
    fn test_materiality_flag_not_triggered() {
        let mut input = gaap_to_ifrs_input();
        // Large total assets, tiny adjustments
        input.total_assets = dec!(100_000_000);
        input.total_equity = dec!(80_000_000);
        input.operating_lease_payments = None;
        input.lifo_reserve = Some(dec!(100)); // Tiny LIFO reserve
        input.capitalised_dev_costs = None;

        let result = reconcile_accounting_standards(&input).unwrap();
        let out = &result.result;

        assert!(
            !out.materiality_flag,
            "Materiality flag should NOT be triggered for tiny adjustments relative to assets"
        );
    }

    // ------------------------------------------------------------------
    // Test 10: Same standard warning
    // ------------------------------------------------------------------
    #[test]
    fn test_same_standard_warning() {
        let mut input = gaap_to_ifrs_input();
        input.target_standard = AccountingStandard::UsGaap; // Same as source

        let result = reconcile_accounting_standards(&input).unwrap();
        let out = &result.result;

        // Should return unadjusted figures
        assert_eq!(out.adjusted_ebitda, input.ebitda);
        assert_eq!(out.adjusted_total_assets, input.total_assets);
        assert!(out.adjustments.is_empty());

        // Should have a warning
        assert!(
            result.warnings.iter().any(|w| w.contains("same")),
            "Should warn when source == target standard"
        );
    }

    // ------------------------------------------------------------------
    // Test 11: Zero total assets error
    // ------------------------------------------------------------------
    #[test]
    fn test_zero_total_assets_error() {
        let mut input = gaap_to_ifrs_input();
        input.total_assets = Decimal::ZERO;

        let result = reconcile_accounting_standards(&input);
        assert!(result.is_err(), "Zero total assets should produce an error");

        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "total_assets");
            }
            other => panic!("Expected InvalidInput for total_assets, got: {other}"),
        }
    }

    // ------------------------------------------------------------------
    // Test 12: Metadata is populated
    // ------------------------------------------------------------------
    #[test]
    fn test_metadata_populated() {
        let input = gaap_to_ifrs_input();
        let result = reconcile_accounting_standards(&input).unwrap();

        assert!(
            !result.methodology.is_empty(),
            "Methodology should be populated"
        );
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
        assert!(!result.metadata.version.is_empty());
        // Computation time should be recorded (>= 0)
        // (we can't assert > 0 because on fast machines it could be 0 μs)
    }

    // ------------------------------------------------------------------
    // Test 13: PV annuity helper correctness
    // ------------------------------------------------------------------
    #[test]
    fn test_pv_annuity_calculation() {
        // PV of $1000/yr for 5 years at 10%
        // = 1000 * ((1 - 1.1^-5) / 0.1)
        // = 1000 * 3.79079 = 3790.79
        let pv = pv_annuity(dec!(1000), dec!(0.10), 5);
        let expected = dec!(3790.79);
        let diff = (pv - expected).abs();
        assert!(
            diff < dec!(1.0),
            "PV annuity should be ~3790.79, got {} (diff: {})",
            pv,
            diff
        );
    }

    // ------------------------------------------------------------------
    // Test 14: GAAP -> IFRS with no optional inputs
    // ------------------------------------------------------------------
    #[test]
    fn test_gaap_to_ifrs_no_optional_inputs() {
        let input = ReconciliationInput {
            source_standard: AccountingStandard::UsGaap,
            target_standard: AccountingStandard::Ifrs,
            revenue: dec!(1_000_000),
            ebitda: dec!(200_000),
            ebit: dec!(150_000),
            net_income: dec!(100_000),
            total_assets: dec!(5_000_000),
            total_debt: dec!(2_000_000),
            total_equity: dec!(3_000_000),
            inventory: dec!(500_000),
            ppe_net: dec!(1_500_000),
            operating_lease_payments: None,
            operating_lease_remaining_years: None,
            lifo_reserve: None,
            capitalised_dev_costs: None,
            dev_cost_amortisation: None,
            revaluation_surplus: None,
            discount_rate_for_leases: None,
            currency: None,
        };

        let result = reconcile_accounting_standards(&input).unwrap();
        let out = &result.result;

        // No adjustments should be made
        assert!(out.adjustments.is_empty());
        assert_eq!(out.adjusted_ebitda, input.ebitda);
        assert_eq!(out.adjusted_total_assets, input.total_assets);
        assert!(!out.materiality_flag);
    }

    // ------------------------------------------------------------------
    // Test 15: Reverse direction (IFRS -> GAAP) dev cost reversal
    // ------------------------------------------------------------------
    #[test]
    fn test_ifrs_to_gaap_dev_cost_reversal() {
        let mut input = ifrs_to_gaap_input();
        input.revaluation_surplus = None; // Isolate dev cost effect
        input.capitalised_dev_costs = Some(dec!(200_000));
        input.dev_cost_amortisation = Some(dec!(40_000));

        let result = reconcile_accounting_standards(&input).unwrap();
        let out = &result.result;

        let dev_adj = out
            .adjustments
            .iter()
            .find(|a| a.category == AdjustmentCategory::DevelopmentCosts)
            .expect("Should have a dev cost reversal adjustment");

        // Assets should decrease
        assert_eq!(dev_adj.impact_assets, dec!(-200_000));

        // Equity should decrease by net book value
        assert_eq!(dev_adj.impact_equity, dec!(-160_000));

        // Adjusted assets should be lower
        assert!(out.adjusted_total_assets < input.total_assets);
    }
}
