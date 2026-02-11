//! ASC 842 / IFRS 16 lease classification and measurement.
//!
//! Classifies leases as finance or operating under ASC 842, computes
//! right-of-use (ROU) assets and lease liabilities at inception, and
//! generates month-by-month amortization schedules using the effective
//! interest method (finance leases) or straight-line recognition
//! (operating leases under ASC 842).

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::types::{Money, Rate};
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const NEWTON_ITERATIONS: u32 = 30;
const FINANCE_LEASE_TERM_THRESHOLD: Decimal = dec!(0.75);
const FINANCE_LEASE_PV_THRESHOLD: Decimal = dec!(0.90);

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// Accounting standard for lease classification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LeaseStandard {
    Asc842,
    Ifrs16,
}

// ---------------------------------------------------------------------------
// Input types
// ---------------------------------------------------------------------------

/// Full input for lease classification and measurement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseInput {
    /// Description of the lease
    pub lease_description: String,
    /// Accounting standard to apply
    pub standard: LeaseStandard,
    /// Total lease term in months
    pub lease_term_months: u32,
    /// Base monthly payment
    pub monthly_payment: Money,
    /// Annual payment escalation (e.g. 0.03 = 3%)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annual_escalation: Option<Rate>,
    /// Lessee's incremental borrowing rate (annual)
    pub incremental_borrowing_rate: Rate,
    /// Rate implicit in the lease (if known, used instead of IBR)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub implicit_rate: Option<Rate>,
    /// Fair market value of the underlying asset
    pub fair_value_of_asset: Money,
    /// Economic useful life in months
    pub useful_life_months: u32,
    /// Guaranteed residual value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub residual_value_guaranteed: Option<Money>,
    /// Unguaranteed residual value (lessor only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub residual_value_unguaranteed: Option<Money>,
    /// Purchase option price
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purchase_option_price: Option<Money>,
    /// Whether exercise of purchase option is reasonably certain
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purchase_option_reasonably_certain: Option<bool>,
    /// Early termination penalty
    #[serde(skip_serializing_if = "Option::is_none")]
    pub termination_penalty: Option<Money>,
    /// Initial direct costs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_direct_costs: Option<Money>,
    /// Lease incentives received from lessor
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lease_incentives_received: Option<Money>,
    /// Prepaid lease payments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prepaid_lease_payments: Option<Money>,
    /// Does ownership transfer at end of lease?
    pub transfer_of_ownership: bool,
    /// Is the asset specialized with no alternative use?
    pub specialized_asset: bool,
}

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// Complete output from lease classification and measurement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseOutput {
    /// Lease description
    pub lease_description: String,
    /// Standard applied
    pub standard: String,
    /// Classification: "Finance" or "Operating"
    pub classification: String,
    /// Which classification tests triggered
    pub classification_criteria: Vec<ClassificationCriterion>,
    /// Right-of-use asset at inception
    pub initial_rou_asset: Money,
    /// PV of lease payments at inception
    pub initial_lease_liability: Money,
    /// Undiscounted total of all lease payments
    pub total_lease_payments: Money,
    /// Total interest expense over lease term
    pub total_interest_expense: Money,
    /// Total ROU depreciation over lease term
    pub total_depreciation: Money,
    /// Month-by-month amortization schedule
    pub amortization_schedule: Vec<LeaseAmortizationRow>,
    /// Weighted average lease term in years
    pub weighted_average_lease_term: Decimal,
    /// PV / FMV percentage (for 90% test)
    pub present_value_to_fair_value_pct: Rate,
}

/// A single classification test result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationCriterion {
    /// Name of the test
    pub test_name: String,
    /// Whether the test triggered finance classification
    pub test_result: bool,
    /// Descriptive detail
    pub detail: String,
}

/// A single row in the lease amortization schedule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseAmortizationRow {
    /// Month number (1-indexed)
    pub month: u32,
    /// Lease liability at start of month
    pub beginning_liability: Money,
    /// Payment made this month
    pub payment: Money,
    /// Interest expense for the month
    pub interest_expense: Money,
    /// Principal reduction for the month
    pub principal_reduction: Money,
    /// Lease liability at end of month
    pub ending_liability: Money,
    /// ROU asset balance at end of month
    pub rou_asset: Money,
    /// Depreciation expense for the month
    pub depreciation: Money,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Classify a lease under ASC 842 or IFRS 16 and compute ROU asset,
/// lease liability, and month-by-month amortization schedule.
pub fn classify_lease(input: &LeaseInput) -> CorpFinanceResult<LeaseOutput> {
    validate_input(input)?;

    let annual_rate = input
        .implicit_rate
        .unwrap_or(input.incremental_borrowing_rate);
    let monthly_rate = annual_to_monthly_rate(annual_rate);

    // Build the payment schedule with escalation
    let payments = build_payment_schedule(input);
    let total_lease_payments: Money = payments.iter().copied().sum();

    // PV of lease payments
    let mut pv_of_payments = Decimal::ZERO;
    let mut discount_factor = Decimal::ONE;
    let one_plus_r = Decimal::ONE + monthly_rate;
    for payment in &payments {
        discount_factor *= one_plus_r;
        if !discount_factor.is_zero() {
            pv_of_payments += *payment / discount_factor;
        }
    }

    // Add PV of purchase option if reasonably certain
    let purchase_option_rc = input.purchase_option_reasonably_certain.unwrap_or(false);
    if purchase_option_rc {
        if let Some(pop) = input.purchase_option_price {
            // discount_factor is already at (1+r)^n after the loop
            discount_factor *= one_plus_r;
            if !discount_factor.is_zero() {
                pv_of_payments += pop / discount_factor;
            }
        }
    }

    // Add PV of guaranteed residual value
    if let Some(grv) = input.residual_value_guaranteed {
        // Reset discount factor for end of lease term
        let mut df_grv = Decimal::ONE;
        for _ in 0..input.lease_term_months {
            df_grv *= one_plus_r;
        }
        if !df_grv.is_zero() {
            pv_of_payments += grv / df_grv;
        }
    }

    let lease_liability = pv_of_payments;

    // Initial ROU asset
    let idc = input.initial_direct_costs.unwrap_or(Decimal::ZERO);
    let incentives = input.lease_incentives_received.unwrap_or(Decimal::ZERO);
    let prepaid = input.prepaid_lease_payments.unwrap_or(Decimal::ZERO);
    let initial_rou_asset = lease_liability + idc + prepaid - incentives;

    // Classification criteria (5 tests)
    let criteria = run_classification_tests(input, lease_liability);
    let is_finance = criteria.iter().any(|c| c.test_result);

    let classification = match input.standard {
        // IFRS 16: all leases are ROU for lessees — always "Finance"
        LeaseStandard::Ifrs16 => "Finance",
        LeaseStandard::Asc842 => {
            if is_finance {
                "Finance"
            } else {
                "Operating"
            }
        }
    };

    // PV / FMV percentage
    let pv_to_fv_pct = if input.fair_value_of_asset.is_zero() {
        Decimal::ZERO
    } else {
        lease_liability / input.fair_value_of_asset
    };

    // Depreciation period: shorter of lease term and useful life,
    // unless ownership transfers (then useful life)
    let dep_months = if input.transfer_of_ownership || purchase_option_rc {
        input.useful_life_months
    } else {
        input.lease_term_months.min(input.useful_life_months)
    };

    // Build amortization schedule
    let is_operating_asc842 =
        input.standard == LeaseStandard::Asc842 && classification == "Operating";
    let schedule = build_amortization_schedule(
        &payments,
        lease_liability,
        initial_rou_asset,
        monthly_rate,
        dep_months,
        is_operating_asc842,
    );

    let total_interest_expense: Money = schedule.iter().map(|r| r.interest_expense).sum();
    let total_depreciation: Money = schedule.iter().map(|r| r.depreciation).sum();

    let weighted_average_lease_term = Decimal::from(input.lease_term_months) / dec!(12);

    Ok(LeaseOutput {
        lease_description: input.lease_description.clone(),
        standard: match input.standard {
            LeaseStandard::Asc842 => "ASC 842".to_string(),
            LeaseStandard::Ifrs16 => "IFRS 16".to_string(),
        },
        classification: classification.to_string(),
        classification_criteria: criteria,
        initial_rou_asset,
        initial_lease_liability: lease_liability,
        total_lease_payments,
        total_interest_expense,
        total_depreciation,
        amortization_schedule: schedule,
        weighted_average_lease_term,
        present_value_to_fair_value_pct: pv_to_fv_pct,
    })
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_input(input: &LeaseInput) -> CorpFinanceResult<()> {
    if input.lease_term_months == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "lease_term_months".into(),
            reason: "Lease term must be greater than zero".into(),
        });
    }
    if input.monthly_payment <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "monthly_payment".into(),
            reason: "Monthly payment must be positive".into(),
        });
    }
    if input.incremental_borrowing_rate <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "incremental_borrowing_rate".into(),
            reason: "Incremental borrowing rate must be positive".into(),
        });
    }
    if input.fair_value_of_asset <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "fair_value_of_asset".into(),
            reason: "Fair value of asset must be positive".into(),
        });
    }
    if input.useful_life_months == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "useful_life_months".into(),
            reason: "Useful life must be greater than zero".into(),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Classification tests
// ---------------------------------------------------------------------------

fn run_classification_tests(
    input: &LeaseInput,
    pv_of_payments: Money,
) -> Vec<ClassificationCriterion> {
    let mut criteria = Vec::with_capacity(5);

    // Test 1: Transfer of ownership
    criteria.push(ClassificationCriterion {
        test_name: "Transfer of Ownership".to_string(),
        test_result: input.transfer_of_ownership,
        detail: if input.transfer_of_ownership {
            "Ownership transfers to lessee at end of lease term".to_string()
        } else {
            "No transfer of ownership at lease end".to_string()
        },
    });

    // Test 2: Purchase option reasonably certain
    let po_rc = input.purchase_option_reasonably_certain.unwrap_or(false)
        && input.purchase_option_price.is_some();
    criteria.push(ClassificationCriterion {
        test_name: "Purchase Option Reasonably Certain".to_string(),
        test_result: po_rc,
        detail: if po_rc {
            format!(
                "Purchase option of {} is reasonably certain to be exercised",
                input.purchase_option_price.unwrap_or(Decimal::ZERO)
            )
        } else {
            "No purchase option reasonably certain to be exercised".to_string()
        },
    });

    // Test 3: Lease term >= 75% of useful life
    let term_ratio = if input.useful_life_months == 0 {
        Decimal::ZERO
    } else {
        Decimal::from(input.lease_term_months) / Decimal::from(input.useful_life_months)
    };
    let term_test = term_ratio >= FINANCE_LEASE_TERM_THRESHOLD;
    criteria.push(ClassificationCriterion {
        test_name: "Lease Term >= 75% of Useful Life".to_string(),
        test_result: term_test,
        detail: format!(
            "Lease term {}/{} months = {:.1}% of useful life (threshold: 75%)",
            input.lease_term_months,
            input.useful_life_months,
            term_ratio * dec!(100)
        ),
    });

    // Test 4: PV of payments >= 90% of FMV
    let pv_ratio = if input.fair_value_of_asset.is_zero() {
        Decimal::ZERO
    } else {
        pv_of_payments / input.fair_value_of_asset
    };
    let pv_test = pv_ratio >= FINANCE_LEASE_PV_THRESHOLD;
    criteria.push(ClassificationCriterion {
        test_name: "PV of Payments >= 90% of Fair Value".to_string(),
        test_result: pv_test,
        detail: format!(
            "PV of payments {} / FMV {} = {:.1}% (threshold: 90%)",
            pv_of_payments,
            input.fair_value_of_asset,
            pv_ratio * dec!(100)
        ),
    });

    // Test 5: Specialized asset
    criteria.push(ClassificationCriterion {
        test_name: "Specialized Asset with No Alternative Use".to_string(),
        test_result: input.specialized_asset,
        detail: if input.specialized_asset {
            "Asset is specialized with no alternative use to the lessor".to_string()
        } else {
            "Asset is not specialized; has alternative uses".to_string()
        },
    });

    criteria
}

// ---------------------------------------------------------------------------
// Payment schedule
// ---------------------------------------------------------------------------

fn build_payment_schedule(input: &LeaseInput) -> Vec<Money> {
    let escalation = input.annual_escalation.unwrap_or(Decimal::ZERO);
    let mut payments = Vec::with_capacity(input.lease_term_months as usize);

    for m in 0..input.lease_term_months {
        let year = m / 12; // 0-based year index
                           // Payment in year Y = base * (1 + escalation)^Y using iterative multiplication
        let mut escalation_factor = Decimal::ONE;
        for _ in 0..year {
            escalation_factor *= Decimal::ONE + escalation;
        }
        payments.push(input.monthly_payment * escalation_factor);
    }

    payments
}

// ---------------------------------------------------------------------------
// Amortization schedule
// ---------------------------------------------------------------------------

fn build_amortization_schedule(
    payments: &[Money],
    initial_liability: Money,
    initial_rou: Money,
    monthly_rate: Rate,
    depreciation_months: u32,
    is_operating_asc842: bool,
) -> Vec<LeaseAmortizationRow> {
    let n = payments.len();
    let mut schedule = Vec::with_capacity(n);

    let monthly_depreciation = if depreciation_months == 0 {
        Decimal::ZERO
    } else {
        initial_rou / Decimal::from(depreciation_months)
    };

    let mut liability = initial_liability;
    let mut rou = initial_rou;

    if is_operating_asc842 {
        // Operating lease under ASC 842: single straight-line lease cost.
        // Total cost = total payments, spread evenly.
        let total_payments: Money = payments.iter().copied().sum();
        let straight_line_cost = if n == 0 {
            Decimal::ZERO
        } else {
            total_payments / Decimal::from(n as u32)
        };

        for (i, &payment) in payments.iter().enumerate() {
            let month = (i + 1) as u32;
            let beg_liability = liability;

            // Interest accrual on liability
            let interest = beg_liability * monthly_rate;
            let principal = payment - interest;
            liability = beg_liability + interest - payment;

            // For operating leases, the "depreciation" is the plug to reach
            // straight-line total cost: depreciation = straight_line_cost - interest
            let dep = if month <= depreciation_months {
                straight_line_cost - interest
            } else {
                Decimal::ZERO
            };

            // ROU asset decreases by the depreciation amount
            rou -= dep;
            // Prevent tiny negative from rounding
            if rou < Decimal::ZERO {
                rou = Decimal::ZERO;
            }

            // Prevent tiny negative liability from rounding on last period
            if liability < Decimal::ZERO && liability > dec!(-0.01) {
                liability = Decimal::ZERO;
            }

            schedule.push(LeaseAmortizationRow {
                month,
                beginning_liability: beg_liability,
                payment,
                interest_expense: interest,
                principal_reduction: principal,
                ending_liability: liability,
                rou_asset: rou,
                depreciation: dep,
            });
        }
    } else {
        // Finance lease: effective interest method for liability,
        // straight-line depreciation for ROU
        for (i, &payment) in payments.iter().enumerate() {
            let month = (i + 1) as u32;
            let beg_liability = liability;

            let interest = beg_liability * monthly_rate;
            let principal = payment - interest;
            liability = beg_liability + interest - payment;

            let dep = if month <= depreciation_months {
                monthly_depreciation
            } else {
                Decimal::ZERO
            };
            rou -= dep;
            if rou < Decimal::ZERO {
                rou = Decimal::ZERO;
            }

            // Prevent tiny negative liability from rounding on last period
            if liability < Decimal::ZERO && liability > dec!(-0.01) {
                liability = Decimal::ZERO;
            }

            schedule.push(LeaseAmortizationRow {
                month,
                beginning_liability: beg_liability,
                payment,
                interest_expense: interest,
                principal_reduction: principal,
                ending_liability: liability,
                rou_asset: rou,
                depreciation: dep,
            });
        }
    }

    schedule
}

// ---------------------------------------------------------------------------
// Math helpers
// ---------------------------------------------------------------------------

/// Convert annual rate to monthly rate using Newton's method for the 12th root.
/// monthly_rate = (1 + annual_rate)^(1/12) - 1
fn annual_to_monthly_rate(annual_rate: Rate) -> Rate {
    let a = Decimal::ONE + annual_rate;
    nth_root(a, 12) - Decimal::ONE
}

/// Newton's method for the nth root of A.
/// x_{k+1} = ((n-1)*x_k + A / x_k^(n-1)) / n
fn nth_root(a: Decimal, n: u32) -> Decimal {
    if a <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    if a == Decimal::ONE {
        return Decimal::ONE;
    }
    let n_dec = Decimal::from(n);
    let n_minus_1 = n_dec - Decimal::ONE;

    // Initial guess: start near 1 for rates close to 1
    let mut x = a;
    // Better initial guess for values close to 1
    if a > dec!(0.5) && a < dec!(2.0) {
        x = Decimal::ONE + (a - Decimal::ONE) / n_dec;
    }

    for _ in 0..NEWTON_ITERATIONS {
        // Compute x^(n-1) iteratively
        let mut x_pow = Decimal::ONE;
        for _ in 0..(n - 1) {
            x_pow *= x;
        }
        if x_pow.is_zero() {
            break;
        }
        let x_new = (n_minus_1 * x + a / x_pow) / n_dec;
        if (x_new - x).abs() < dec!(0.0000000000001) {
            return x_new;
        }
        x = x_new;
    }
    x
}

/// Compute PV of a payment stream at a given monthly rate using iterative
/// discount factors.
pub(crate) fn pv_of_payment_stream(payments: &[Money], monthly_rate: Rate) -> Money {
    let mut pv = Decimal::ZERO;
    let mut discount_factor = Decimal::ONE;
    let one_plus_r = Decimal::ONE + monthly_rate;
    for payment in payments {
        discount_factor *= one_plus_r;
        if !discount_factor.is_zero() {
            pv += *payment / discount_factor;
        }
    }
    pv
}

/// Build a payment schedule from basic lease parameters (reused by sale_leaseback).
pub(crate) fn build_payment_schedule_from_params(
    lease_term_months: u32,
    monthly_payment: Money,
    annual_escalation: Option<Rate>,
) -> Vec<Money> {
    let escalation = annual_escalation.unwrap_or(Decimal::ZERO);
    let mut payments = Vec::with_capacity(lease_term_months as usize);
    for m in 0..lease_term_months {
        let year = m / 12;
        let mut escalation_factor = Decimal::ONE;
        for _ in 0..year {
            escalation_factor *= Decimal::ONE + escalation;
        }
        payments.push(monthly_payment * escalation_factor);
    }
    payments
}

/// Convert annual rate to monthly — public within crate for sale_leaseback.
pub(crate) fn annual_to_monthly(annual_rate: Rate) -> Rate {
    annual_to_monthly_rate(annual_rate)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// Helper: standard office lease for testing
    fn standard_office_lease() -> LeaseInput {
        LeaseInput {
            lease_description: "Office Lease - 123 Main St".to_string(),
            standard: LeaseStandard::Asc842,
            lease_term_months: 60,
            monthly_payment: dec!(10000),
            annual_escalation: None,
            incremental_borrowing_rate: dec!(0.06),
            implicit_rate: None,
            fair_value_of_asset: dec!(1000000),
            useful_life_months: 240,
            residual_value_guaranteed: None,
            residual_value_unguaranteed: None,
            purchase_option_price: None,
            purchase_option_reasonably_certain: None,
            termination_penalty: None,
            initial_direct_costs: None,
            lease_incentives_received: None,
            prepaid_lease_payments: None,
            transfer_of_ownership: false,
            specialized_asset: false,
        }
    }

    /// Helper: finance lease that triggers via the 75% useful life test
    fn finance_lease_long_term() -> LeaseInput {
        LeaseInput {
            lease_description: "Equipment Lease - Long Term".to_string(),
            standard: LeaseStandard::Asc842,
            lease_term_months: 96, // 8 years
            monthly_payment: dec!(5000),
            annual_escalation: None,
            incremental_borrowing_rate: dec!(0.05),
            implicit_rate: None,
            fair_value_of_asset: dec!(400000),
            useful_life_months: 120, // 10 years => 96/120 = 80% >= 75%
            residual_value_guaranteed: None,
            residual_value_unguaranteed: None,
            purchase_option_price: None,
            purchase_option_reasonably_certain: None,
            termination_penalty: None,
            initial_direct_costs: None,
            lease_incentives_received: None,
            prepaid_lease_payments: None,
            transfer_of_ownership: false,
            specialized_asset: false,
        }
    }

    // -----------------------------------------------------------------------
    // 1. Operating lease classification (no finance tests triggered)
    // -----------------------------------------------------------------------
    #[test]
    fn test_operating_lease_classification() {
        let input = standard_office_lease();
        let result = classify_lease(&input).unwrap();

        assert_eq!(result.classification, "Operating");
        assert_eq!(result.standard, "ASC 842");
        // No criteria should be triggered
        assert!(
            !result.classification_criteria.iter().any(|c| c.test_result),
            "No finance criteria should trigger for a short-term office lease"
        );
    }

    // -----------------------------------------------------------------------
    // 2. Finance lease via transfer of ownership
    // -----------------------------------------------------------------------
    #[test]
    fn test_finance_lease_transfer_of_ownership() {
        let mut input = standard_office_lease();
        input.transfer_of_ownership = true;

        let result = classify_lease(&input).unwrap();
        assert_eq!(result.classification, "Finance");

        let ownership_test = result
            .classification_criteria
            .iter()
            .find(|c| c.test_name == "Transfer of Ownership")
            .unwrap();
        assert!(ownership_test.test_result);
    }

    // -----------------------------------------------------------------------
    // 3. Finance lease via purchase option reasonably certain
    // -----------------------------------------------------------------------
    #[test]
    fn test_finance_lease_purchase_option() {
        let mut input = standard_office_lease();
        input.purchase_option_price = Some(dec!(1));
        input.purchase_option_reasonably_certain = Some(true);

        let result = classify_lease(&input).unwrap();
        assert_eq!(result.classification, "Finance");

        let po_test = result
            .classification_criteria
            .iter()
            .find(|c| c.test_name == "Purchase Option Reasonably Certain")
            .unwrap();
        assert!(po_test.test_result);
    }

    // -----------------------------------------------------------------------
    // 4. Finance lease via 75% useful life test
    // -----------------------------------------------------------------------
    #[test]
    fn test_finance_lease_75_percent_useful_life() {
        let input = finance_lease_long_term();
        let result = classify_lease(&input).unwrap();

        assert_eq!(result.classification, "Finance");

        let term_test = result
            .classification_criteria
            .iter()
            .find(|c| c.test_name == "Lease Term >= 75% of Useful Life")
            .unwrap();
        assert!(term_test.test_result);
    }

    // -----------------------------------------------------------------------
    // 5. Finance lease via 90% PV test
    // -----------------------------------------------------------------------
    #[test]
    fn test_finance_lease_90_percent_pv() {
        // Set fair value low so PV of payments >= 90%
        let mut input = standard_office_lease();
        input.fair_value_of_asset = dec!(500000);
        // 60 months * 10000 = 600k total, PV will be close to 500k+
        // With 6% IBR, PV ~= 517k => 517k/500k > 90%
        let result = classify_lease(&input).unwrap();

        assert_eq!(result.classification, "Finance");

        let pv_test = result
            .classification_criteria
            .iter()
            .find(|c| c.test_name.contains("90%"))
            .unwrap();
        assert!(pv_test.test_result);
        assert!(result.present_value_to_fair_value_pct >= dec!(0.90));
    }

    // -----------------------------------------------------------------------
    // 6. Finance lease via specialized asset
    // -----------------------------------------------------------------------
    #[test]
    fn test_finance_lease_specialized_asset() {
        let mut input = standard_office_lease();
        input.specialized_asset = true;

        let result = classify_lease(&input).unwrap();
        assert_eq!(result.classification, "Finance");

        let spec_test = result
            .classification_criteria
            .iter()
            .find(|c| c.test_name.contains("Specialized"))
            .unwrap();
        assert!(spec_test.test_result);
    }

    // -----------------------------------------------------------------------
    // 7. IFRS 16 always classifies as Finance for lessee
    // -----------------------------------------------------------------------
    #[test]
    fn test_ifrs16_always_finance() {
        let mut input = standard_office_lease();
        input.standard = LeaseStandard::Ifrs16;

        let result = classify_lease(&input).unwrap();
        assert_eq!(result.classification, "Finance");
        assert_eq!(result.standard, "IFRS 16");
    }

    // -----------------------------------------------------------------------
    // 8. IFRS 16 with no finance criteria still classifies as Finance
    // -----------------------------------------------------------------------
    #[test]
    fn test_ifrs16_operating_scenario_still_finance() {
        let mut input = standard_office_lease();
        input.standard = LeaseStandard::Ifrs16;
        // This would be operating under ASC 842, but IFRS 16 = all Finance
        let result = classify_lease(&input).unwrap();
        assert_eq!(result.classification, "Finance");
    }

    // -----------------------------------------------------------------------
    // 9. Payment escalation
    // -----------------------------------------------------------------------
    #[test]
    fn test_escalating_payments() {
        let mut input = standard_office_lease();
        input.annual_escalation = Some(dec!(0.03)); // 3% annual escalation

        let result = classify_lease(&input).unwrap();

        // Total payments should exceed non-escalating scenario
        let flat_total = dec!(10000) * dec!(60);
        assert!(
            result.total_lease_payments > flat_total,
            "Escalating payments total {} should exceed flat total {}",
            result.total_lease_payments,
            flat_total
        );

        // Check that later payments are higher
        let sched = &result.amortization_schedule;
        let first_payment = sched[0].payment;
        let last_payment = sched[sched.len() - 1].payment;
        assert!(
            last_payment > first_payment,
            "Last payment {} should exceed first payment {} with escalation",
            last_payment,
            first_payment
        );
    }

    // -----------------------------------------------------------------------
    // 10. Guaranteed residual value increases PV
    // -----------------------------------------------------------------------
    #[test]
    fn test_guaranteed_residual_value() {
        let input_no_grv = standard_office_lease();
        let result_no_grv = classify_lease(&input_no_grv).unwrap();

        let mut input_grv = standard_office_lease();
        input_grv.residual_value_guaranteed = Some(dec!(50000));
        let result_grv = classify_lease(&input_grv).unwrap();

        assert!(
            result_grv.initial_lease_liability > result_no_grv.initial_lease_liability,
            "Lease liability with GRV {} should exceed without GRV {}",
            result_grv.initial_lease_liability,
            result_no_grv.initial_lease_liability
        );
    }

    // -----------------------------------------------------------------------
    // 11. Initial direct costs increase ROU
    // -----------------------------------------------------------------------
    #[test]
    fn test_initial_direct_costs() {
        let input_no_idc = standard_office_lease();
        let result_no_idc = classify_lease(&input_no_idc).unwrap();

        let mut input_idc = standard_office_lease();
        input_idc.initial_direct_costs = Some(dec!(15000));
        let result_idc = classify_lease(&input_idc).unwrap();

        let diff = result_idc.initial_rou_asset - result_no_idc.initial_rou_asset;
        assert_eq!(
            diff,
            dec!(15000),
            "ROU should increase by IDC amount, got diff {}",
            diff
        );
    }

    // -----------------------------------------------------------------------
    // 12. Lease incentives decrease ROU
    // -----------------------------------------------------------------------
    #[test]
    fn test_lease_incentives() {
        let input_no_inc = standard_office_lease();
        let result_no_inc = classify_lease(&input_no_inc).unwrap();

        let mut input_inc = standard_office_lease();
        input_inc.lease_incentives_received = Some(dec!(20000));
        let result_inc = classify_lease(&input_inc).unwrap();

        let diff = result_no_inc.initial_rou_asset - result_inc.initial_rou_asset;
        assert_eq!(
            diff,
            dec!(20000),
            "ROU should decrease by incentive amount, got diff {}",
            diff
        );
    }

    // -----------------------------------------------------------------------
    // 13. Prepaid payments increase ROU
    // -----------------------------------------------------------------------
    #[test]
    fn test_prepaid_lease_payments() {
        let input_no_pp = standard_office_lease();
        let result_no_pp = classify_lease(&input_no_pp).unwrap();

        let mut input_pp = standard_office_lease();
        input_pp.prepaid_lease_payments = Some(dec!(10000));
        let result_pp = classify_lease(&input_pp).unwrap();

        let diff = result_pp.initial_rou_asset - result_no_pp.initial_rou_asset;
        assert_eq!(
            diff,
            dec!(10000),
            "ROU should increase by prepaid amount, got diff {}",
            diff
        );
    }

    // -----------------------------------------------------------------------
    // 14. Amortization schedule has correct length
    // -----------------------------------------------------------------------
    #[test]
    fn test_amortization_schedule_length() {
        let input = standard_office_lease();
        let result = classify_lease(&input).unwrap();

        assert_eq!(
            result.amortization_schedule.len(),
            60,
            "Schedule should have 60 monthly rows"
        );
    }

    // -----------------------------------------------------------------------
    // 15. Amortization schedule: first row begins with full liability
    // -----------------------------------------------------------------------
    #[test]
    fn test_amortization_first_row() {
        let input = standard_office_lease();
        let result = classify_lease(&input).unwrap();

        let first = &result.amortization_schedule[0];
        assert_eq!(first.month, 1);
        let diff = (first.beginning_liability - result.initial_lease_liability).abs();
        assert!(
            diff < dec!(0.01),
            "First row liability {} should match initial {}",
            first.beginning_liability,
            result.initial_lease_liability
        );
    }

    // -----------------------------------------------------------------------
    // 16. Amortization: ending liability near zero at end
    // -----------------------------------------------------------------------
    #[test]
    fn test_amortization_ending_liability_near_zero() {
        // Use finance lease with no extras
        let input = finance_lease_long_term();
        let result = classify_lease(&input).unwrap();

        let last = result.amortization_schedule.last().unwrap();
        assert!(
            last.ending_liability.abs() < dec!(1.0),
            "Ending liability should be near zero, got {}",
            last.ending_liability
        );
    }

    // -----------------------------------------------------------------------
    // 17. Implicit rate used when provided
    // -----------------------------------------------------------------------
    #[test]
    fn test_implicit_rate_used() {
        let mut input_ibr = standard_office_lease();
        input_ibr.implicit_rate = None;
        let result_ibr = classify_lease(&input_ibr).unwrap();

        let mut input_imp = standard_office_lease();
        input_imp.implicit_rate = Some(dec!(0.04)); // lower rate => higher PV
        let result_imp = classify_lease(&input_imp).unwrap();

        assert!(
            result_imp.initial_lease_liability > result_ibr.initial_lease_liability,
            "Lower implicit rate should produce higher PV: {} vs {}",
            result_imp.initial_lease_liability,
            result_ibr.initial_lease_liability
        );
    }

    // -----------------------------------------------------------------------
    // 18. Weighted average lease term
    // -----------------------------------------------------------------------
    #[test]
    fn test_weighted_average_lease_term() {
        let input = standard_office_lease();
        let result = classify_lease(&input).unwrap();

        assert_eq!(
            result.weighted_average_lease_term,
            dec!(5),
            "60 months = 5 years, got {}",
            result.weighted_average_lease_term
        );
    }

    // -----------------------------------------------------------------------
    // 19. Validation: zero lease term
    // -----------------------------------------------------------------------
    #[test]
    fn test_invalid_zero_lease_term() {
        let mut input = standard_office_lease();
        input.lease_term_months = 0;

        let result = classify_lease(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "lease_term_months");
            }
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // 20. Validation: zero monthly payment
    // -----------------------------------------------------------------------
    #[test]
    fn test_invalid_zero_monthly_payment() {
        let mut input = standard_office_lease();
        input.monthly_payment = Decimal::ZERO;

        let result = classify_lease(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "monthly_payment");
            }
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // 21. Validation: zero IBR
    // -----------------------------------------------------------------------
    #[test]
    fn test_invalid_zero_ibr() {
        let mut input = standard_office_lease();
        input.incremental_borrowing_rate = Decimal::ZERO;

        let result = classify_lease(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "incremental_borrowing_rate");
            }
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // 22. Validation: zero fair value
    // -----------------------------------------------------------------------
    #[test]
    fn test_invalid_zero_fair_value() {
        let mut input = standard_office_lease();
        input.fair_value_of_asset = Decimal::ZERO;

        let result = classify_lease(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "fair_value_of_asset");
            }
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // 23. Validation: zero useful life
    // -----------------------------------------------------------------------
    #[test]
    fn test_invalid_zero_useful_life() {
        let mut input = standard_office_lease();
        input.useful_life_months = 0;

        let result = classify_lease(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "useful_life_months");
            }
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // 24. Finance lease: depreciation over shorter of term/useful life
    // -----------------------------------------------------------------------
    #[test]
    fn test_depreciation_shorter_period() {
        // Lease term 60 months, useful life 240 months => dep over 60 months
        let mut input = standard_office_lease();
        input.transfer_of_ownership = true; // make it finance
        let result = classify_lease(&input).unwrap();

        // With transfer of ownership, depreciation period = useful life (240)
        // so monthly dep = ROU / 240
        let expected_monthly_dep = result.initial_rou_asset / dec!(240);
        let first_dep = result.amortization_schedule[0].depreciation;
        let diff = (first_dep - expected_monthly_dep).abs();
        assert!(
            diff < dec!(0.01),
            "Monthly depreciation should be ~{}, got {}",
            expected_monthly_dep,
            first_dep
        );
    }

    // -----------------------------------------------------------------------
    // 25. Operating lease: straight-line cost (ASC 842)
    // -----------------------------------------------------------------------
    #[test]
    fn test_operating_lease_straight_line() {
        let input = standard_office_lease();
        let result = classify_lease(&input).unwrap();
        assert_eq!(result.classification, "Operating");

        // For operating lease, total expense per month = straight-line total cost
        let sched = &result.amortization_schedule;
        let first_total = sched[0].interest_expense + sched[0].depreciation;
        let mid_total = sched[30].interest_expense + sched[30].depreciation;

        // Should be approximately equal (straight-line)
        let diff = (first_total - mid_total).abs();
        assert!(
            diff < dec!(1.0),
            "Operating lease cost should be approximately straight-line: first={}, mid={}",
            first_total,
            mid_total
        );
    }

    // -----------------------------------------------------------------------
    // 26. PV to fair value percentage
    // -----------------------------------------------------------------------
    #[test]
    fn test_pv_to_fair_value_pct() {
        let input = standard_office_lease();
        let result = classify_lease(&input).unwrap();

        // PV should be positive and less than 1 for this scenario
        assert!(
            result.present_value_to_fair_value_pct > Decimal::ZERO,
            "PV/FMV should be positive"
        );
        assert!(
            result.present_value_to_fair_value_pct < Decimal::ONE,
            "PV/FMV should be less than 1 for this operating lease, got {}",
            result.present_value_to_fair_value_pct
        );
    }

    // -----------------------------------------------------------------------
    // 27. Total interest + total depreciation approximately = total payments
    //     minus initial liability for finance lease (since interest adds up to
    //     total payments - principal, and depreciation = ROU)
    // -----------------------------------------------------------------------
    #[test]
    fn test_interest_plus_principal_equals_total_payments() {
        let input = finance_lease_long_term();
        let result = classify_lease(&input).unwrap();

        // Total interest + total principal reduction = total payments
        let total_interest: Decimal = result
            .amortization_schedule
            .iter()
            .map(|r| r.interest_expense)
            .sum();
        let total_principal: Decimal = result
            .amortization_schedule
            .iter()
            .map(|r| r.principal_reduction)
            .sum();

        let sum = total_interest + total_principal;
        let diff = (sum - result.total_lease_payments).abs();
        assert!(
            diff < dec!(1.0),
            "Interest ({}) + principal ({}) = {} should equal total payments ({})",
            total_interest,
            total_principal,
            sum,
            result.total_lease_payments
        );
    }

    // -----------------------------------------------------------------------
    // 28. Multiple finance criteria can trigger simultaneously
    // -----------------------------------------------------------------------
    #[test]
    fn test_multiple_criteria_triggered() {
        let mut input = standard_office_lease();
        input.transfer_of_ownership = true;
        input.specialized_asset = true;

        let result = classify_lease(&input).unwrap();
        assert_eq!(result.classification, "Finance");

        let triggered: Vec<_> = result
            .classification_criteria
            .iter()
            .filter(|c| c.test_result)
            .collect();
        assert!(
            triggered.len() >= 2,
            "At least 2 criteria should trigger, got {}",
            triggered.len()
        );
    }

    // -----------------------------------------------------------------------
    // 29. Purchase option not reasonably certain does NOT trigger
    // -----------------------------------------------------------------------
    #[test]
    fn test_purchase_option_not_reasonably_certain() {
        let mut input = standard_office_lease();
        input.purchase_option_price = Some(dec!(50000));
        input.purchase_option_reasonably_certain = Some(false);

        let result = classify_lease(&input).unwrap();

        let po_test = result
            .classification_criteria
            .iter()
            .find(|c| c.test_name.contains("Purchase Option"))
            .unwrap();
        assert!(
            !po_test.test_result,
            "Purchase option not reasonably certain should not trigger"
        );
    }

    // -----------------------------------------------------------------------
    // 30. Nth root helper: 12th root of 1.06 should give ~0.00487 monthly rate
    // -----------------------------------------------------------------------
    #[test]
    fn test_nth_root_precision() {
        let annual = dec!(0.06);
        let monthly = annual_to_monthly_rate(annual);

        // (1.06)^(1/12) - 1 ~ 0.004867...
        assert!(
            monthly > dec!(0.00486) && monthly < dec!(0.00488),
            "Monthly rate for 6% annual should be ~0.00487, got {}",
            monthly
        );

        // Verify round-trip: (1+monthly)^12 should be close to 1.06
        let mut compounded = Decimal::ONE;
        for _ in 0..12 {
            compounded *= Decimal::ONE + monthly;
        }
        let diff = (compounded - dec!(1.06)).abs();
        assert!(
            diff < dec!(0.000001),
            "Round-trip (1+m)^12 should be ~1.06, got {}",
            compounded
        );
    }
}
