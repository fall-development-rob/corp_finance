//! Sale-leaseback analysis under ASC 842 / IFRS 16.
//!
//! Determines whether a sale-leaseback transaction qualifies as a sale (ASC 606
//! criteria), computes gain/loss recognition (including partial recognition for
//! the retained right), deferred gains for above-FMV transactions, and
//! failed-sale financing obligation treatment.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::types::{Money, Rate};
use crate::CorpFinanceResult;

use super::classification::{
    annual_to_monthly, build_payment_schedule_from_params, pv_of_payment_stream, LeaseStandard,
};

// ---------------------------------------------------------------------------
// Input types
// ---------------------------------------------------------------------------

/// Full input for a sale-leaseback analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaleLeasebackInput {
    /// Description of the transaction
    pub description: String,
    /// Accounting standard to apply
    pub standard: LeaseStandard,
    /// Book value of asset on seller-lessee's books
    pub asset_carrying_value: Money,
    /// Price buyer-lessor pays
    pub sale_price: Money,
    /// Fair market value of the underlying asset
    pub fair_value: Money,
    /// Leaseback term in months
    pub lease_term_months: u32,
    /// Monthly leaseback payment
    pub monthly_lease_payment: Money,
    /// Annual payment escalation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annual_escalation: Option<Rate>,
    /// Lessee's incremental borrowing rate (annual)
    pub incremental_borrowing_rate: Rate,
    /// Remaining useful life of the asset in months
    pub useful_life_remaining_months: u32,
    /// Whether the transfer qualifies as a sale under ASC 606
    pub qualifies_as_sale: bool,
}

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// Complete output from a sale-leaseback analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaleLeasebackOutput {
    /// Transaction description
    pub description: String,
    /// Whether the transaction qualifies as a sale
    pub qualifies_as_sale: bool,
    /// Accounting for a qualifying sale
    pub sale_accounting: SaleAccounting,
    /// Accounting for a failed sale (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed_sale_accounting: Option<FailedSaleAccounting>,
    /// Classification of the leaseback
    pub leaseback_classification: String,
    /// ROU asset from the leaseback
    pub leaseback_rou_asset: Money,
    /// Lease liability from the leaseback
    pub leaseback_lease_liability: Money,
    /// Net cash impact: sale proceeds minus total undiscounted lease payments
    pub net_cash_impact: Money,
    /// Gain recognized at inception
    pub gain_on_sale: Money,
    /// Gain deferred (financing component)
    pub deferred_gain: Money,
    /// Net P&L impact at inception
    pub total_pnl_impact: Money,
}

/// Accounting details for a qualifying sale.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaleAccounting {
    /// Proceeds received from the sale
    pub sale_proceeds: Money,
    /// Carrying value derecognized
    pub carrying_value: Money,
    /// Total gain or loss = sale_price - carrying_value
    pub total_gain_loss: Money,
    /// Gain recognized immediately
    pub recognized_gain: Money,
    /// Gain deferred (financing or retained right)
    pub deferred_gain: Money,
    /// PV of leaseback / fair_value
    pub retained_right_ratio: Rate,
}

/// Accounting details for a failed sale.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedSaleAccounting {
    /// Amount of the financing obligation
    pub financing_obligation: Money,
    /// Whether the asset remains on the seller-lessee's books
    pub asset_remains_on_books: bool,
    /// Reason the sale failed
    pub reason: String,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Analyze a sale-leaseback transaction under ASC 842 / IFRS 16.
pub fn analyze_sale_leaseback(
    input: &SaleLeasebackInput,
) -> CorpFinanceResult<SaleLeasebackOutput> {
    validate_input(input)?;

    let monthly_rate = annual_to_monthly(input.incremental_borrowing_rate);

    // Build leaseback payment schedule
    let payments = build_payment_schedule_from_params(
        input.lease_term_months,
        input.monthly_lease_payment,
        input.annual_escalation,
    );
    let total_undiscounted: Money = payments.iter().copied().sum();
    let pv_leaseback = pv_of_payment_stream(&payments, monthly_rate);

    // Leaseback classification using the 5-test framework
    let leaseback_classification = classify_leaseback(input, pv_leaseback);

    let net_cash_impact = input.sale_price - total_undiscounted;

    if input.qualifies_as_sale {
        // Qualifying sale
        let total_gain_loss = input.sale_price - input.asset_carrying_value;
        let retained_right_ratio = if input.fair_value.is_zero() {
            Decimal::ZERO
        } else {
            pv_leaseback / input.fair_value
        };

        let (recognized_gain, deferred_gain, rou_asset) = if input.sale_price == input.fair_value {
            // At fair value: recognize full gain, ROU = retained right portion of carrying value
            let recognized = total_gain_loss * (Decimal::ONE - retained_right_ratio);
            let deferred = total_gain_loss - recognized;
            let rou = retained_right_ratio * input.asset_carrying_value;
            (recognized, deferred, rou)
        } else if input.sale_price > input.fair_value {
            // Above fair value: excess is additional financing (deferred)
            let excess = input.sale_price - input.fair_value;
            // Gain based on fair value
            let gain_at_fmv = input.fair_value - input.asset_carrying_value;
            let recognized = if gain_at_fmv > Decimal::ZERO {
                gain_at_fmv * (Decimal::ONE - retained_right_ratio)
            } else {
                gain_at_fmv // losses recognized in full
            };
            let deferred = total_gain_loss - recognized;
            // ROU = PV of leaseback - excess (above-market portion adds to liability, not ROU)
            let rou = pv_leaseback;
            let _ = excess; // excess accounted for in deferred_gain
            (recognized, deferred, rou)
        } else {
            // Below fair value: difference is prepaid rent, adjusts ROU
            let shortfall = input.fair_value - input.sale_price;
            let recognized = total_gain_loss * (Decimal::ONE - retained_right_ratio);
            let deferred = total_gain_loss - recognized;
            // ROU includes the prepaid rent adjustment
            let rou = pv_leaseback + shortfall;
            (recognized, deferred, rou)
        };

        let sale_accounting = SaleAccounting {
            sale_proceeds: input.sale_price,
            carrying_value: input.asset_carrying_value,
            total_gain_loss,
            recognized_gain,
            deferred_gain,
            retained_right_ratio,
        };

        let total_pnl_impact = recognized_gain;

        Ok(SaleLeasebackOutput {
            description: input.description.clone(),
            qualifies_as_sale: true,
            sale_accounting,
            failed_sale_accounting: None,
            leaseback_classification,
            leaseback_rou_asset: rou_asset,
            leaseback_lease_liability: pv_leaseback,
            net_cash_impact,
            gain_on_sale: recognized_gain,
            deferred_gain,
            total_pnl_impact,
        })
    } else {
        // Failed sale: treat as financing
        let sale_accounting = SaleAccounting {
            sale_proceeds: input.sale_price,
            carrying_value: input.asset_carrying_value,
            total_gain_loss: Decimal::ZERO,
            recognized_gain: Decimal::ZERO,
            deferred_gain: Decimal::ZERO,
            retained_right_ratio: Decimal::ONE,
        };

        let failed_sale = FailedSaleAccounting {
            financing_obligation: input.sale_price,
            asset_remains_on_books: true,
            reason: "Transfer does not qualify as a sale under ASC 606 / IFRS 15 criteria"
                .to_string(),
        };

        Ok(SaleLeasebackOutput {
            description: input.description.clone(),
            qualifies_as_sale: false,
            sale_accounting,
            failed_sale_accounting: Some(failed_sale),
            leaseback_classification,
            leaseback_rou_asset: Decimal::ZERO,
            leaseback_lease_liability: input.sale_price, // financing obligation
            net_cash_impact,
            gain_on_sale: Decimal::ZERO,
            deferred_gain: Decimal::ZERO,
            total_pnl_impact: Decimal::ZERO,
        })
    }
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_input(input: &SaleLeasebackInput) -> CorpFinanceResult<()> {
    if input.asset_carrying_value < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "asset_carrying_value".into(),
            reason: "Asset carrying value cannot be negative".into(),
        });
    }
    if input.sale_price <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "sale_price".into(),
            reason: "Sale price must be positive".into(),
        });
    }
    if input.fair_value <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "fair_value".into(),
            reason: "Fair value must be positive".into(),
        });
    }
    if input.lease_term_months == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "lease_term_months".into(),
            reason: "Lease term must be greater than zero".into(),
        });
    }
    if input.monthly_lease_payment <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "monthly_lease_payment".into(),
            reason: "Monthly lease payment must be positive".into(),
        });
    }
    if input.incremental_borrowing_rate <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "incremental_borrowing_rate".into(),
            reason: "Incremental borrowing rate must be positive".into(),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Leaseback classification
// ---------------------------------------------------------------------------

/// Simplified 5-test classification for the leaseback portion.
fn classify_leaseback(input: &SaleLeasebackInput, pv_leaseback: Money) -> String {
    // For IFRS 16, all leases are effectively finance for lessee
    if input.standard == LeaseStandard::Ifrs16 {
        return "Finance".to_string();
    }

    // Test 1: ownership transfer — in a sale-leaseback, ownership does NOT
    // transfer back (seller-lessee sold it), so this is always false.
    // Test 2: purchase option — not modeled in this input; false.
    // Test 3: lease term >= 75% useful life
    let term_test = if input.useful_life_remaining_months == 0 {
        false
    } else {
        let ratio = Decimal::from(input.lease_term_months)
            / Decimal::from(input.useful_life_remaining_months);
        ratio >= dec!(0.75)
    };

    // Test 4: PV >= 90% of FMV
    let pv_test = if input.fair_value.is_zero() {
        false
    } else {
        pv_leaseback / input.fair_value >= dec!(0.90)
    };

    // Test 5: specialized asset — not modeled separately for leaseback

    if term_test || pv_test {
        "Finance".to_string()
    } else {
        "Operating".to_string()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// Helper: standard sale-leaseback at FMV
    fn standard_slb() -> SaleLeasebackInput {
        SaleLeasebackInput {
            description: "Office Building Sale-Leaseback".to_string(),
            standard: LeaseStandard::Asc842,
            asset_carrying_value: dec!(800000),
            sale_price: dec!(1000000),
            fair_value: dec!(1000000),
            lease_term_months: 60,
            monthly_lease_payment: dec!(12000),
            annual_escalation: None,
            incremental_borrowing_rate: dec!(0.06),
            useful_life_remaining_months: 240,
            qualifies_as_sale: true,
        }
    }

    // -----------------------------------------------------------------------
    // 1. Qualifying sale at FMV: gain recognition
    // -----------------------------------------------------------------------
    #[test]
    fn test_qualifying_sale_at_fmv() {
        let input = standard_slb();
        let result = analyze_sale_leaseback(&input).unwrap();

        assert!(result.qualifies_as_sale);
        assert!(result.failed_sale_accounting.is_none());

        // Total gain = 1M - 800k = 200k
        let sa = &result.sale_accounting;
        assert_eq!(sa.total_gain_loss, dec!(200000));

        // Recognized gain = total_gain * (1 - retained_right_ratio)
        assert!(
            result.gain_on_sale > Decimal::ZERO,
            "Should recognize partial gain at FMV"
        );
        assert!(
            result.gain_on_sale <= dec!(200000),
            "Recognized gain should not exceed total gain"
        );
    }

    // -----------------------------------------------------------------------
    // 2. Retained right ratio is PV / FMV
    // -----------------------------------------------------------------------
    #[test]
    fn test_retained_right_ratio() {
        let input = standard_slb();
        let result = analyze_sale_leaseback(&input).unwrap();

        let rr = result.sale_accounting.retained_right_ratio;
        // PV of 60 payments of 12k at 6% should be a meaningful fraction of 1M
        assert!(
            rr > dec!(0.5) && rr < dec!(0.9),
            "Retained right ratio should be between 0.5 and 0.9, got {}",
            rr
        );
    }

    // -----------------------------------------------------------------------
    // 3. Deferred gain + recognized gain = total gain
    // -----------------------------------------------------------------------
    #[test]
    fn test_gain_decomposition() {
        let input = standard_slb();
        let result = analyze_sale_leaseback(&input).unwrap();

        let sa = &result.sale_accounting;
        let sum = sa.recognized_gain + sa.deferred_gain;
        let diff = (sum - sa.total_gain_loss).abs();
        assert!(
            diff < dec!(0.01),
            "Recognized ({}) + deferred ({}) = {} should equal total gain ({})",
            sa.recognized_gain,
            sa.deferred_gain,
            sum,
            sa.total_gain_loss
        );
    }

    // -----------------------------------------------------------------------
    // 4. Above-FMV sale: financing element
    // -----------------------------------------------------------------------
    #[test]
    fn test_above_fmv_sale() {
        let mut input = standard_slb();
        input.sale_price = dec!(1200000); // 200k above FMV of 1M

        let result = analyze_sale_leaseback(&input).unwrap();

        // Deferred gain should include the excess
        assert!(
            result.deferred_gain > Decimal::ZERO,
            "Above-FMV sale should have deferred gain, got {}",
            result.deferred_gain
        );

        // The recognized gain should be less than the total gain
        let sa = &result.sale_accounting;
        assert!(
            sa.recognized_gain < sa.total_gain_loss,
            "Recognized gain ({}) should be less than total gain ({})",
            sa.recognized_gain,
            sa.total_gain_loss
        );
    }

    // -----------------------------------------------------------------------
    // 5. Below-FMV sale: prepaid rent
    // -----------------------------------------------------------------------
    #[test]
    fn test_below_fmv_sale() {
        let mut input = standard_slb();
        input.sale_price = dec!(900000); // 100k below FMV
        input.asset_carrying_value = dec!(700000);

        let result = analyze_sale_leaseback(&input).unwrap();

        // ROU should include prepaid rent adjustment
        assert!(
            result.leaseback_rou_asset > result.leaseback_lease_liability,
            "Below-FMV ROU ({}) should exceed lease liability ({}) due to prepaid rent",
            result.leaseback_rou_asset,
            result.leaseback_lease_liability
        );
    }

    // -----------------------------------------------------------------------
    // 6. Failed sale: financing treatment
    // -----------------------------------------------------------------------
    #[test]
    fn test_failed_sale() {
        let mut input = standard_slb();
        input.qualifies_as_sale = false;

        let result = analyze_sale_leaseback(&input).unwrap();

        assert!(!result.qualifies_as_sale);
        assert!(result.failed_sale_accounting.is_some());

        let fsa = result.failed_sale_accounting.as_ref().unwrap();
        assert!(fsa.asset_remains_on_books);
        assert_eq!(fsa.financing_obligation, dec!(1000000));

        // No gain recognized
        assert_eq!(result.gain_on_sale, Decimal::ZERO);
        assert_eq!(result.deferred_gain, Decimal::ZERO);
        assert_eq!(result.total_pnl_impact, Decimal::ZERO);
    }

    // -----------------------------------------------------------------------
    // 7. Leaseback classification: operating (short term vs useful life)
    // -----------------------------------------------------------------------
    #[test]
    fn test_leaseback_operating_classification() {
        let input = standard_slb();
        let result = analyze_sale_leaseback(&input).unwrap();

        // 60 months / 240 months = 25% < 75% => Operating (assuming PV < 90%)
        assert_eq!(
            result.leaseback_classification, "Operating",
            "Short leaseback should be classified Operating"
        );
    }

    // -----------------------------------------------------------------------
    // 8. Leaseback classification: finance (long term)
    // -----------------------------------------------------------------------
    #[test]
    fn test_leaseback_finance_classification() {
        let mut input = standard_slb();
        input.lease_term_months = 192; // 16 years out of 20 = 80% >= 75%
        input.useful_life_remaining_months = 240;

        let result = analyze_sale_leaseback(&input).unwrap();
        assert_eq!(
            result.leaseback_classification, "Finance",
            "Long leaseback should be classified Finance"
        );
    }

    // -----------------------------------------------------------------------
    // 9. Net cash impact
    // -----------------------------------------------------------------------
    #[test]
    fn test_net_cash_impact() {
        let input = standard_slb();
        let result = analyze_sale_leaseback(&input).unwrap();

        // Sale price = 1M, total undiscounted payments = 60 * 12k = 720k
        // Net cash = 1M - 720k = 280k
        let expected = dec!(1000000) - dec!(720000);
        assert_eq!(
            result.net_cash_impact, expected,
            "Net cash impact should be {}, got {}",
            expected, result.net_cash_impact
        );
    }

    // -----------------------------------------------------------------------
    // 10. Lease liability equals PV of leaseback payments
    // -----------------------------------------------------------------------
    #[test]
    fn test_lease_liability_equals_pv() {
        let input = standard_slb();
        let result = analyze_sale_leaseback(&input).unwrap();

        // Lease liability should be PV of 60 payments of 12k at 6%
        assert!(
            result.leaseback_lease_liability > Decimal::ZERO,
            "Lease liability should be positive"
        );
        assert!(
            result.leaseback_lease_liability < dec!(720000),
            "PV should be less than undiscounted total"
        );
    }

    // -----------------------------------------------------------------------
    // 11. Sale at a loss: full loss recognized
    // -----------------------------------------------------------------------
    #[test]
    fn test_sale_at_loss() {
        let mut input = standard_slb();
        input.asset_carrying_value = dec!(1200000); // CV > sale price
        input.sale_price = dec!(1000000);
        input.fair_value = dec!(1000000);

        let result = analyze_sale_leaseback(&input).unwrap();

        let sa = &result.sale_accounting;
        // Total loss = 1M - 1.2M = -200k
        assert_eq!(sa.total_gain_loss, dec!(-200000));
        // Recognized gain is negative (loss)
        assert!(
            sa.recognized_gain < Decimal::ZERO,
            "Should recognize a loss"
        );
    }

    // -----------------------------------------------------------------------
    // 12. IFRS 16 leaseback always Finance
    // -----------------------------------------------------------------------
    #[test]
    fn test_ifrs16_leaseback_classification() {
        let mut input = standard_slb();
        input.standard = LeaseStandard::Ifrs16;

        let result = analyze_sale_leaseback(&input).unwrap();
        assert_eq!(result.leaseback_classification, "Finance");
    }

    // -----------------------------------------------------------------------
    // 13. Escalating leaseback payments
    // -----------------------------------------------------------------------
    #[test]
    fn test_escalating_leaseback() {
        let mut input_flat = standard_slb();
        input_flat.annual_escalation = None;
        let result_flat = analyze_sale_leaseback(&input_flat).unwrap();

        let mut input_esc = standard_slb();
        input_esc.annual_escalation = Some(dec!(0.03));
        let result_esc = analyze_sale_leaseback(&input_esc).unwrap();

        // Escalating payments should increase lease liability
        assert!(
            result_esc.leaseback_lease_liability > result_flat.leaseback_lease_liability,
            "Escalating payments should increase PV: {} vs {}",
            result_esc.leaseback_lease_liability,
            result_flat.leaseback_lease_liability
        );
    }

    // -----------------------------------------------------------------------
    // 14. Validation: negative carrying value
    // -----------------------------------------------------------------------
    #[test]
    fn test_invalid_negative_carrying_value() {
        let mut input = standard_slb();
        input.asset_carrying_value = dec!(-100);

        let result = analyze_sale_leaseback(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "asset_carrying_value");
            }
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // 15. Validation: zero sale price
    // -----------------------------------------------------------------------
    #[test]
    fn test_invalid_zero_sale_price() {
        let mut input = standard_slb();
        input.sale_price = Decimal::ZERO;

        let result = analyze_sale_leaseback(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "sale_price");
            }
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // 16. Validation: zero fair value
    // -----------------------------------------------------------------------
    #[test]
    fn test_invalid_zero_fair_value() {
        let mut input = standard_slb();
        input.fair_value = Decimal::ZERO;

        let result = analyze_sale_leaseback(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "fair_value");
            }
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // 17. Validation: zero lease term
    // -----------------------------------------------------------------------
    #[test]
    fn test_invalid_zero_lease_term() {
        let mut input = standard_slb();
        input.lease_term_months = 0;

        let result = analyze_sale_leaseback(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "lease_term_months");
            }
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // 18. Total PnL impact equals recognized gain for qualifying sale
    // -----------------------------------------------------------------------
    #[test]
    fn test_total_pnl_equals_recognized_gain() {
        let input = standard_slb();
        let result = analyze_sale_leaseback(&input).unwrap();

        assert_eq!(
            result.total_pnl_impact, result.gain_on_sale,
            "Total P&L impact should equal recognized gain"
        );
    }

    // -----------------------------------------------------------------------
    // 19. ROU asset for at-FMV sale = retained_right * carrying_value
    // -----------------------------------------------------------------------
    #[test]
    fn test_rou_at_fmv_calculation() {
        let input = standard_slb();
        let result = analyze_sale_leaseback(&input).unwrap();

        let rr = result.sale_accounting.retained_right_ratio;
        let expected_rou = rr * input.asset_carrying_value;
        let diff = (result.leaseback_rou_asset - expected_rou).abs();
        assert!(
            diff < dec!(0.01),
            "ROU at FMV should be retained_right * CV: expected {}, got {}",
            expected_rou,
            result.leaseback_rou_asset
        );
    }

    // -----------------------------------------------------------------------
    // 20. Failed sale: lease liability equals sale price (financing)
    // -----------------------------------------------------------------------
    #[test]
    fn test_failed_sale_liability_equals_price() {
        let mut input = standard_slb();
        input.qualifies_as_sale = false;

        let result = analyze_sale_leaseback(&input).unwrap();
        assert_eq!(
            result.leaseback_lease_liability, input.sale_price,
            "Failed sale liability should equal sale price"
        );
    }
}
