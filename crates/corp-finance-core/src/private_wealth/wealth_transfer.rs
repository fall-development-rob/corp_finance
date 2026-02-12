//! Multi-generational wealth transfer planning.
//!
//! Analyses estate tax exposure and compares wealth transfer strategies:
//! - **Annual exclusion gifting** -- annual tax-free gifts to beneficiaries.
//! - **Lifetime exemption** -- estate/gift tax unified credit.
//! - **GRAT** -- grantor retained annuity trust for appreciation shifting.
//! - **IDGT** -- intentional defective grantor trust for income tax leverage.
//! - **GST planning** -- generation-skipping transfer tax mitigation.
//!
//! All arithmetic uses `rust_decimal::Decimal`. No `f64`.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Input / Output
// ---------------------------------------------------------------------------

/// Input for wealth transfer analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WealthTransferInput {
    /// Total estate value.
    pub estate_value: Decimal,
    /// Annual gift exclusion per beneficiary.
    pub annual_exclusion: Decimal,
    /// Lifetime gift/estate tax exemption.
    pub lifetime_exemption: Decimal,
    /// Top estate tax rate (e.g. 0.40).
    pub estate_tax_rate: Decimal,
    /// Generation-skipping transfer tax rate.
    pub gst_tax_rate: Decimal,
    /// Number of beneficiaries.
    pub num_beneficiaries: u32,
    /// Planning horizon in years.
    pub transfer_years: u32,
    /// Expected annual asset growth rate.
    pub asset_growth_rate: Decimal,
    /// Assets in grantor trusts (GRAT/IDGT).
    pub grantor_trust_assets: Decimal,
    /// GRAT annuity as fraction of initial contribution.
    pub grat_annuity_rate: Decimal,
    /// IRS Section 7520 hurdle rate.
    pub section_7520_rate: Decimal,
}

/// A single transfer strategy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferStrategy {
    /// Strategy name.
    pub name: String,
    /// Amount transferred to heirs.
    pub amount_transferred: Decimal,
    /// Tax savings from this strategy.
    pub tax_savings: Decimal,
    /// Description of the strategy.
    pub description: String,
}

/// Output of wealth transfer analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WealthTransferOutput {
    /// Total annual gifts = annual_exclusion * num_beneficiaries per year.
    pub annual_gifts_total: Decimal,
    /// Cumulative annual gifts over transfer_years.
    pub total_annual_gifts: Decimal,
    /// Estate after annual gifting.
    pub remaining_estate: Decimal,
    /// Value passing to heirs via GRAT (excess over 7520 hurdle).
    pub grat_remainder: Decimal,
    /// Estate tax saved via GRAT.
    pub grat_tax_savings: Decimal,
    /// Taxable estate = remaining - exemption.
    pub taxable_estate: Decimal,
    /// Estate tax owed.
    pub estate_tax: Decimal,
    /// Value exposed to GST.
    pub gst_exposure: Decimal,
    /// Effective transfer rate = total to heirs / initial estate.
    pub effective_transfer_rate: Decimal,
    /// Ranked strategies.
    pub strategies: Vec<TransferStrategy>,
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate(input: &WealthTransferInput) -> CorpFinanceResult<()> {
    if input.estate_value <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "estate_value".into(),
            reason: "must be positive".into(),
        });
    }
    if input.annual_exclusion < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "annual_exclusion".into(),
            reason: "cannot be negative".into(),
        });
    }
    if input.lifetime_exemption < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "lifetime_exemption".into(),
            reason: "cannot be negative".into(),
        });
    }
    if input.estate_tax_rate < Decimal::ZERO || input.estate_tax_rate > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "estate_tax_rate".into(),
            reason: "must be between 0 and 1".into(),
        });
    }
    if input.gst_tax_rate < Decimal::ZERO || input.gst_tax_rate > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "gst_tax_rate".into(),
            reason: "must be between 0 and 1".into(),
        });
    }
    if input.num_beneficiaries == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "num_beneficiaries".into(),
            reason: "must be at least 1".into(),
        });
    }
    if input.transfer_years == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "transfer_years".into(),
            reason: "must be at least 1 year".into(),
        });
    }
    if input.grantor_trust_assets < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "grantor_trust_assets".into(),
            reason: "cannot be negative".into(),
        });
    }
    if input.grat_annuity_rate < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "grat_annuity_rate".into(),
            reason: "cannot be negative".into(),
        });
    }
    if input.section_7520_rate < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "section_7520_rate".into(),
            reason: "cannot be negative".into(),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Core function
// ---------------------------------------------------------------------------

/// Analyse multi-generational wealth transfer strategies.
pub fn analyze_wealth_transfer(
    input: &WealthTransferInput,
) -> CorpFinanceResult<WealthTransferOutput> {
    validate(input)?;

    let n_benef = Decimal::from(input.num_beneficiaries);
    let n_years = Decimal::from(input.transfer_years);

    // 1. Annual exclusion gifting
    let annual_gifts_total = input.annual_exclusion * n_benef;
    let total_annual_gifts = annual_gifts_total * n_years;
    let gifts_capped = total_annual_gifts.min(input.estate_value);

    // 2. Estate after annual gifts (with growth on remaining assets)
    let mut remaining = input.estate_value - gifts_capped;
    let growth = Decimal::ONE + input.asset_growth_rate;
    for _ in 0..input.transfer_years {
        remaining *= growth;
    }

    // 3. GRAT calculation
    // Zeroed-out GRAT: annuity PV ~ contribution, gift tax = ~0
    // GRAT grows at asset_growth_rate, pays annuity at grat_annuity_rate
    let grat_contribution = input.grantor_trust_assets;
    let grat_annuity = input.grat_annuity_rate * grat_contribution;

    // Annuity PV at Section 7520 rate
    let mut annuity_pv = Decimal::ZERO;
    let hurdle_div = Decimal::ONE + input.section_7520_rate;
    let mut df = Decimal::ONE;
    for _ in 0..input.transfer_years {
        df /= hurdle_div;
        annuity_pv += grat_annuity * df;
    }

    // FV of GRAT assets at actual growth rate
    let mut grat_fv = grat_contribution;
    for _ in 0..input.transfer_years {
        grat_fv *= growth;
    }
    let total_annuity_paid = grat_annuity * n_years;

    // GRAT remainder = FV - total annuity payments (excess growth over hurdle)
    let grat_remainder = if grat_fv > total_annuity_paid {
        grat_fv - total_annuity_paid
    } else {
        Decimal::ZERO
    };

    let grat_tax_savings = grat_remainder * input.estate_tax_rate;

    // 4. Taxable estate
    // Remove GRAT assets from remaining estate (they are in trust)
    let estate_after_grat = if remaining > grat_contribution {
        remaining - grat_contribution
    } else {
        remaining
    };

    let taxable_estate = if estate_after_grat > input.lifetime_exemption {
        estate_after_grat - input.lifetime_exemption
    } else {
        Decimal::ZERO
    };

    let estate_tax = taxable_estate * input.estate_tax_rate;

    // 5. GST exposure (simplified: skip-generation gifts above GST exemption)
    let gst_exposure = if taxable_estate > Decimal::ZERO {
        taxable_estate
    } else {
        Decimal::ZERO
    };

    // 6. Effective transfer rate
    let total_to_heirs = gifts_capped + grat_remainder + estate_after_grat - estate_tax;
    let effective_transfer_rate = if input.estate_value > Decimal::ZERO {
        total_to_heirs / input.estate_value
    } else {
        Decimal::ZERO
    };

    // 7. Strategies
    let mut strategies = Vec::new();

    strategies.push(TransferStrategy {
        name: "Annual Exclusion Gifting".into(),
        amount_transferred: gifts_capped,
        tax_savings: gifts_capped * input.estate_tax_rate,
        description: format!(
            "Gift ${} per year to {} beneficiaries for {} years",
            input.annual_exclusion, input.num_beneficiaries, input.transfer_years
        ),
    });

    strategies.push(TransferStrategy {
        name: "Zeroed-Out GRAT".into(),
        amount_transferred: grat_remainder,
        tax_savings: grat_tax_savings,
        description: format!(
            "GRAT with {} annuity rate, transferring excess growth over {:.2}% hurdle",
            input.grat_annuity_rate,
            input.section_7520_rate * dec!(100)
        ),
    });

    let idgt_benefit = grat_contribution * input.estate_tax_rate * dec!(0.30);
    strategies.push(TransferStrategy {
        name: "IDGT (Intentional Defective Grantor Trust)".into(),
        amount_transferred: grat_contribution,
        tax_savings: idgt_benefit,
        description: "Grantor pays income tax on trust assets, allowing tax-free growth".into(),
    });

    strategies.push(TransferStrategy {
        name: "Lifetime Exemption".into(),
        amount_transferred: input.lifetime_exemption.min(input.estate_value),
        tax_savings: input.lifetime_exemption.min(input.estate_value) * input.estate_tax_rate,
        description: format!("Use ${} lifetime exemption", input.lifetime_exemption),
    });

    // Sort by tax savings descending
    strategies.sort_by(|a, b| b.tax_savings.cmp(&a.tax_savings));

    Ok(WealthTransferOutput {
        annual_gifts_total,
        total_annual_gifts: gifts_capped,
        remaining_estate: remaining,
        grat_remainder,
        grat_tax_savings,
        taxable_estate,
        estate_tax,
        gst_exposure,
        effective_transfer_rate,
        strategies,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn base_input() -> WealthTransferInput {
        WealthTransferInput {
            estate_value: dec!(50_000_000),
            annual_exclusion: dec!(18_000),
            lifetime_exemption: dec!(13_610_000),
            estate_tax_rate: dec!(0.40),
            gst_tax_rate: dec!(0.40),
            num_beneficiaries: 4,
            transfer_years: 10,
            asset_growth_rate: dec!(0.07),
            grantor_trust_assets: dec!(10_000_000),
            grat_annuity_rate: dec!(0.20),
            section_7520_rate: dec!(0.052),
        }
    }

    #[test]
    fn test_annual_gifts_total() {
        let out = analyze_wealth_transfer(&base_input()).unwrap();
        // 18k * 4 = 72k per year
        assert_eq!(out.annual_gifts_total, dec!(72_000));
    }

    #[test]
    fn test_total_annual_gifts() {
        let out = analyze_wealth_transfer(&base_input()).unwrap();
        // 72k * 10 = 720k
        assert_eq!(out.total_annual_gifts, dec!(720_000));
    }

    #[test]
    fn test_remaining_estate_grows() {
        let out = analyze_wealth_transfer(&base_input()).unwrap();
        // (50M - 720k) grows at 7% for 10 years
        assert!(out.remaining_estate > dec!(50_000_000));
    }

    #[test]
    fn test_grat_remainder_positive() {
        let mut inp = base_input();
        // Use lower annuity rate so total annuity < FV of assets
        // 10M * 0.10 = 1M/yr annuity, total = 10M; FV of 10M at 7% for 10yr ~ 19.67M
        inp.grat_annuity_rate = dec!(0.10);
        let out = analyze_wealth_transfer(&inp).unwrap();
        // Growth rate (7%) > 7520 rate (5.2%), and annuity total < FV
        assert!(out.grat_remainder > Decimal::ZERO);
    }

    #[test]
    fn test_grat_tax_savings() {
        let out = analyze_wealth_transfer(&base_input()).unwrap();
        assert_eq!(out.grat_tax_savings, out.grat_remainder * dec!(0.40));
    }

    #[test]
    fn test_taxable_estate_positive_large_estate() {
        let out = analyze_wealth_transfer(&base_input()).unwrap();
        assert!(out.taxable_estate > Decimal::ZERO);
    }

    #[test]
    fn test_estate_tax_calculation() {
        let out = analyze_wealth_transfer(&base_input()).unwrap();
        assert_eq!(out.estate_tax, out.taxable_estate * dec!(0.40));
    }

    #[test]
    fn test_small_estate_no_tax() {
        let mut inp = base_input();
        inp.estate_value = dec!(5_000_000);
        inp.grantor_trust_assets = Decimal::ZERO;
        let out = analyze_wealth_transfer(&inp).unwrap();
        assert_eq!(out.estate_tax, Decimal::ZERO);
    }

    #[test]
    fn test_exemption_covers_all() {
        let mut inp = base_input();
        inp.estate_value = dec!(10_000_000);
        inp.grantor_trust_assets = Decimal::ZERO;
        inp.asset_growth_rate = Decimal::ZERO;
        let out = analyze_wealth_transfer(&inp).unwrap();
        // 10M - 720k gifts = 9.28M remaining, under 13.61M exemption
        assert_eq!(out.estate_tax, Decimal::ZERO);
    }

    #[test]
    fn test_gst_exposure_mirrors_taxable() {
        let out = analyze_wealth_transfer(&base_input()).unwrap();
        assert_eq!(out.gst_exposure, out.taxable_estate);
    }

    #[test]
    fn test_effective_transfer_rate_range() {
        let out = analyze_wealth_transfer(&base_input()).unwrap();
        assert!(out.effective_transfer_rate > Decimal::ZERO);
    }

    #[test]
    fn test_strategies_count() {
        let out = analyze_wealth_transfer(&base_input()).unwrap();
        assert_eq!(out.strategies.len(), 4);
    }

    #[test]
    fn test_strategies_sorted_by_tax_savings() {
        let out = analyze_wealth_transfer(&base_input()).unwrap();
        for i in 1..out.strategies.len() {
            assert!(out.strategies[i - 1].tax_savings >= out.strategies[i].tax_savings);
        }
    }

    #[test]
    fn test_grat_zero_growth() {
        let mut inp = base_input();
        inp.asset_growth_rate = Decimal::ZERO;
        let out = analyze_wealth_transfer(&inp).unwrap();
        // No excess growth => GRAT remainder = 0
        assert_eq!(out.grat_remainder, Decimal::ZERO);
    }

    #[test]
    fn test_grat_growth_below_hurdle() {
        let mut inp = base_input();
        inp.asset_growth_rate = dec!(0.03); // below 5.2% hurdle
        let out = analyze_wealth_transfer(&inp).unwrap();
        // FV may be less than annuity paid => remainder = 0
        assert_eq!(out.grat_remainder, Decimal::ZERO);
    }

    #[test]
    fn test_many_beneficiaries() {
        let mut inp = base_input();
        inp.num_beneficiaries = 20;
        let out = analyze_wealth_transfer(&inp).unwrap();
        // 18k * 20 * 10 = 3.6M
        assert_eq!(out.annual_gifts_total, dec!(360_000));
    }

    #[test]
    fn test_invalid_estate_value() {
        let mut inp = base_input();
        inp.estate_value = Decimal::ZERO;
        assert!(analyze_wealth_transfer(&inp).is_err());
    }

    #[test]
    fn test_invalid_num_beneficiaries() {
        let mut inp = base_input();
        inp.num_beneficiaries = 0;
        assert!(analyze_wealth_transfer(&inp).is_err());
    }

    #[test]
    fn test_invalid_transfer_years() {
        let mut inp = base_input();
        inp.transfer_years = 0;
        assert!(analyze_wealth_transfer(&inp).is_err());
    }

    #[test]
    fn test_invalid_tax_rate() {
        let mut inp = base_input();
        inp.estate_tax_rate = dec!(1.5);
        assert!(analyze_wealth_transfer(&inp).is_err());
    }

    #[test]
    fn test_one_year_horizon() {
        let mut inp = base_input();
        inp.transfer_years = 1;
        let out = analyze_wealth_transfer(&inp).unwrap();
        assert_eq!(out.annual_gifts_total, dec!(72_000));
        assert_eq!(out.total_annual_gifts, dec!(72_000));
    }

    #[test]
    fn test_no_grantor_trust() {
        let mut inp = base_input();
        inp.grantor_trust_assets = Decimal::ZERO;
        let out = analyze_wealth_transfer(&inp).unwrap();
        assert_eq!(out.grat_remainder, Decimal::ZERO);
        assert_eq!(out.grat_tax_savings, Decimal::ZERO);
    }
}
