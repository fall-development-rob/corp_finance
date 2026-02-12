//! Philanthropic vehicle comparison analysis.
//!
//! Evaluates five charitable giving vehicles:
//! 1. **Donor-Advised Fund (DAF)** -- immediate deduction, flexible granting.
//! 2. **Charitable Remainder Trust (CRT)** -- income to donor, remainder to charity.
//! 3. **Charitable Lead Trust (CLT)** -- income to charity, remainder to heirs.
//! 4. **Private Foundation** -- perpetual control, excise tax.
//! 5. **Direct Gift** -- simple outright donation.
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

/// Input for philanthropic vehicle comparison.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhilanthropicInput {
    /// Amount being donated.
    pub donation_amount: Decimal,
    /// Donor adjusted gross income.
    pub donor_income: Decimal,
    /// Marginal income tax rate.
    pub donor_tax_rate: Decimal,
    /// Fair market value of appreciated asset (if donating assets).
    pub appreciated_asset_fmv: Decimal,
    /// Cost basis of appreciated asset.
    pub appreciated_asset_basis: Decimal,
    /// Annual payout rate for CRT/CLT (e.g. 0.05 = 5%).
    pub payout_rate: Decimal,
    /// Trust term in years.
    pub trust_term_years: u32,
    /// IRS Section 7520 discount rate.
    pub discount_rate: Decimal,
    /// Donor age (for life-based trusts).
    pub donor_age: u32,
}

/// A single vehicle comparison result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VehicleComparison {
    /// Vehicle name.
    pub name: String,
    /// Estimated income tax deduction.
    pub tax_deduction: Decimal,
    /// Annual income stream to donor (0 for non-income vehicles).
    pub income_stream: Decimal,
    /// Estate reduction amount.
    pub estate_reduction: Decimal,
    /// Flexibility score (0-10).
    pub flexibility: Decimal,
    /// Complexity score (1-10, higher = more complex).
    pub complexity_score: Decimal,
}

/// Output of philanthropic vehicle analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhilanthropicOutput {
    /// Vehicle comparisons.
    pub vehicles: Vec<VehicleComparison>,
    /// Capital gains avoided by donating appreciated asset.
    pub capital_gains_avoided: Decimal,
    /// Recommended vehicle name.
    pub recommended_vehicle: String,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Present value of annuity: pmt * sum_{t=1}^{n} 1/(1+r)^t
fn pv_annuity(pmt: Decimal, rate: Decimal, periods: u32) -> Decimal {
    let mut pv = Decimal::ZERO;
    let mut df = Decimal::ONE;
    let divisor = Decimal::ONE + rate;
    for _ in 0..periods {
        df /= divisor;
        pv += pmt * df;
    }
    pv
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate(input: &PhilanthropicInput) -> CorpFinanceResult<()> {
    if input.donation_amount <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "donation_amount".into(),
            reason: "must be positive".into(),
        });
    }
    if input.donor_income <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "donor_income".into(),
            reason: "must be positive".into(),
        });
    }
    if input.donor_tax_rate < Decimal::ZERO || input.donor_tax_rate > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "donor_tax_rate".into(),
            reason: "must be between 0 and 1".into(),
        });
    }
    if input.appreciated_asset_fmv < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "appreciated_asset_fmv".into(),
            reason: "cannot be negative".into(),
        });
    }
    if input.appreciated_asset_basis < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "appreciated_asset_basis".into(),
            reason: "cannot be negative".into(),
        });
    }
    if input.payout_rate <= Decimal::ZERO || input.payout_rate > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "payout_rate".into(),
            reason: "must be between 0 (exclusive) and 1".into(),
        });
    }
    if input.trust_term_years == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "trust_term_years".into(),
            reason: "must be at least 1 year".into(),
        });
    }
    if input.discount_rate < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "discount_rate".into(),
            reason: "cannot be negative".into(),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Core function
// ---------------------------------------------------------------------------

/// Compare philanthropic vehicles for a donation.
pub fn compare_philanthropic_vehicles(
    input: &PhilanthropicInput,
) -> CorpFinanceResult<PhilanthropicOutput> {
    validate(input)?;

    let donation = input.donation_amount;
    let agi = input.donor_income;
    let fmv = input.appreciated_asset_fmv;
    let basis = input.appreciated_asset_basis;
    let gain_avoided = if fmv > basis {
        fmv - basis
    } else {
        Decimal::ZERO
    };

    // Effective term for life-based trusts
    let life_expectancy = if input.donor_age < 85 {
        85 - input.donor_age
    } else {
        1
    };
    let effective_term = if input.trust_term_years > 0 {
        input.trust_term_years
    } else {
        life_expectancy
    };

    let mut vehicles: Vec<VehicleComparison> = Vec::with_capacity(5);

    // 1. Donor-Advised Fund
    // Cash: up to 60% AGI; Appreciated: up to 30% AGI
    let daf_limit_cash = agi * dec!(0.60);
    let daf_limit_appreciated = agi * dec!(0.30);
    let daf_deduction = if fmv > Decimal::ZERO {
        fmv.min(daf_limit_appreciated)
    } else {
        donation.min(daf_limit_cash)
    };
    vehicles.push(VehicleComparison {
        name: "Donor-Advised Fund".into(),
        tax_deduction: daf_deduction,
        income_stream: Decimal::ZERO,
        estate_reduction: donation,
        flexibility: dec!(9),
        complexity_score: dec!(2),
    });

    // 2. Charitable Remainder Trust
    let crt_annual_payout = donation * input.payout_rate;
    let annuity_pv = pv_annuity(crt_annual_payout, input.discount_rate, effective_term);
    let crt_deduction = if donation > annuity_pv {
        donation - annuity_pv
    } else {
        Decimal::ZERO
    };
    vehicles.push(VehicleComparison {
        name: "Charitable Remainder Trust".into(),
        tax_deduction: crt_deduction,
        income_stream: crt_annual_payout,
        estate_reduction: donation,
        flexibility: dec!(5),
        complexity_score: dec!(7),
    });

    // 3. Charitable Lead Trust
    let clt_annual_payout = donation * input.payout_rate;
    let clt_pv = pv_annuity(clt_annual_payout, input.discount_rate, effective_term);
    vehicles.push(VehicleComparison {
        name: "Charitable Lead Trust".into(),
        tax_deduction: clt_pv, // estate/gift tax deduction for charitable lead interest
        income_stream: Decimal::ZERO, // income goes to charity, not donor
        estate_reduction: clt_pv,
        flexibility: dec!(4),
        complexity_score: dec!(8),
    });

    // 4. Private Foundation
    // Cash: 30% AGI, Appreciated: 20% AGI
    let pf_limit_cash = agi * dec!(0.30);
    let pf_limit_appreciated = agi * dec!(0.20);
    let pf_deduction = if fmv > Decimal::ZERO {
        fmv.min(pf_limit_appreciated)
    } else {
        donation.min(pf_limit_cash)
    };
    vehicles.push(VehicleComparison {
        name: "Private Foundation".into(),
        tax_deduction: pf_deduction,
        income_stream: Decimal::ZERO,
        estate_reduction: donation,
        flexibility: dec!(8),
        complexity_score: dec!(9),
    });

    // 5. Direct Gift
    let direct_limit = agi * dec!(0.60);
    let direct_deduction = if fmv > Decimal::ZERO {
        fmv.min(agi * dec!(0.30))
    } else {
        donation.min(direct_limit)
    };
    vehicles.push(VehicleComparison {
        name: "Direct Gift".into(),
        tax_deduction: direct_deduction,
        income_stream: Decimal::ZERO,
        estate_reduction: donation,
        flexibility: dec!(3),
        complexity_score: dec!(1),
    });

    // Recommend: highest deduction-to-complexity ratio
    let recommended = vehicles
        .iter()
        .max_by_key(|v| {
            if v.complexity_score > Decimal::ZERO {
                v.tax_deduction * dec!(10) / v.complexity_score + v.income_stream * dec!(5)
            } else {
                v.tax_deduction * dec!(10)
            }
        })
        .map(|v| v.name.clone())
        .unwrap_or_else(|| "Direct Gift".into());

    Ok(PhilanthropicOutput {
        vehicles,
        capital_gains_avoided: gain_avoided,
        recommended_vehicle: recommended,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn base_input() -> PhilanthropicInput {
        PhilanthropicInput {
            donation_amount: dec!(500_000),
            donor_income: dec!(1_000_000),
            donor_tax_rate: dec!(0.37),
            appreciated_asset_fmv: dec!(500_000),
            appreciated_asset_basis: dec!(100_000),
            payout_rate: dec!(0.05),
            trust_term_years: 20,
            discount_rate: dec!(0.052),
            donor_age: 60,
        }
    }

    #[test]
    fn test_five_vehicles_returned() {
        let out = compare_philanthropic_vehicles(&base_input()).unwrap();
        assert_eq!(out.vehicles.len(), 5);
    }

    #[test]
    fn test_capital_gains_avoided() {
        let out = compare_philanthropic_vehicles(&base_input()).unwrap();
        assert_eq!(out.capital_gains_avoided, dec!(400_000));
    }

    #[test]
    fn test_daf_deduction_agi_limit() {
        let out = compare_philanthropic_vehicles(&base_input()).unwrap();
        let daf = out
            .vehicles
            .iter()
            .find(|v| v.name == "Donor-Advised Fund")
            .unwrap();
        // 30% AGI limit for appreciated = 300k, FMV = 500k => capped at 300k
        assert_eq!(daf.tax_deduction, dec!(300_000));
    }

    #[test]
    fn test_daf_cash_limit() {
        let mut inp = base_input();
        inp.appreciated_asset_fmv = Decimal::ZERO;
        let out = compare_philanthropic_vehicles(&inp).unwrap();
        let daf = out
            .vehicles
            .iter()
            .find(|v| v.name == "Donor-Advised Fund")
            .unwrap();
        // 60% AGI = 600k, donation 500k => 500k
        assert_eq!(daf.tax_deduction, dec!(500_000));
    }

    #[test]
    fn test_crt_has_income_stream() {
        let out = compare_philanthropic_vehicles(&base_input()).unwrap();
        let crt = out
            .vehicles
            .iter()
            .find(|v| v.name == "Charitable Remainder Trust")
            .unwrap();
        assert_eq!(crt.income_stream, dec!(25_000)); // 500k * 5%
    }

    #[test]
    fn test_crt_deduction_positive() {
        let out = compare_philanthropic_vehicles(&base_input()).unwrap();
        let crt = out
            .vehicles
            .iter()
            .find(|v| v.name == "Charitable Remainder Trust")
            .unwrap();
        assert!(crt.tax_deduction > Decimal::ZERO);
        assert!(crt.tax_deduction < dec!(500_000));
    }

    #[test]
    fn test_clt_no_income_to_donor() {
        let out = compare_philanthropic_vehicles(&base_input()).unwrap();
        let clt = out
            .vehicles
            .iter()
            .find(|v| v.name == "Charitable Lead Trust")
            .unwrap();
        assert_eq!(clt.income_stream, Decimal::ZERO);
    }

    #[test]
    fn test_clt_estate_reduction() {
        let out = compare_philanthropic_vehicles(&base_input()).unwrap();
        let clt = out
            .vehicles
            .iter()
            .find(|v| v.name == "Charitable Lead Trust")
            .unwrap();
        assert!(clt.estate_reduction > Decimal::ZERO);
    }

    #[test]
    fn test_private_foundation_lower_limit() {
        let out = compare_philanthropic_vehicles(&base_input()).unwrap();
        let pf = out
            .vehicles
            .iter()
            .find(|v| v.name == "Private Foundation")
            .unwrap();
        // 20% AGI for appreciated = 200k
        assert_eq!(pf.tax_deduction, dec!(200_000));
    }

    #[test]
    fn test_direct_gift_simplicity() {
        let out = compare_philanthropic_vehicles(&base_input()).unwrap();
        let dg = out
            .vehicles
            .iter()
            .find(|v| v.name == "Direct Gift")
            .unwrap();
        assert_eq!(dg.complexity_score, dec!(1));
    }

    #[test]
    fn test_recommended_vehicle_is_valid() {
        let out = compare_philanthropic_vehicles(&base_input()).unwrap();
        let names: Vec<&str> = out.vehicles.iter().map(|v| v.name.as_str()).collect();
        assert!(names.contains(&out.recommended_vehicle.as_str()));
    }

    #[test]
    fn test_large_donation() {
        let mut inp = base_input();
        inp.donation_amount = dec!(10_000_000);
        inp.appreciated_asset_fmv = dec!(10_000_000);
        let out = compare_philanthropic_vehicles(&inp).unwrap();
        // DAF capped at 30% AGI = 300k for appreciated
        let daf = out
            .vehicles
            .iter()
            .find(|v| v.name == "Donor-Advised Fund")
            .unwrap();
        assert_eq!(daf.tax_deduction, dec!(300_000));
    }

    #[test]
    fn test_no_appreciated_asset() {
        let mut inp = base_input();
        inp.appreciated_asset_fmv = Decimal::ZERO;
        inp.appreciated_asset_basis = Decimal::ZERO;
        let out = compare_philanthropic_vehicles(&inp).unwrap();
        assert_eq!(out.capital_gains_avoided, Decimal::ZERO);
    }

    #[test]
    fn test_young_donor_longer_life() {
        let mut inp = base_input();
        inp.donor_age = 30;
        // With 55-year life expectancy vs 25-year term, term dominates
        let out = compare_philanthropic_vehicles(&inp).unwrap();
        let crt = out
            .vehicles
            .iter()
            .find(|v| v.name == "Charitable Remainder Trust")
            .unwrap();
        assert!(crt.tax_deduction > Decimal::ZERO);
    }

    #[test]
    fn test_old_donor_short_life() {
        let mut inp = base_input();
        inp.donor_age = 84;
        let out = compare_philanthropic_vehicles(&inp).unwrap();
        let crt = out
            .vehicles
            .iter()
            .find(|v| v.name == "Charitable Remainder Trust")
            .unwrap();
        assert!(crt.tax_deduction > Decimal::ZERO);
    }

    #[test]
    fn test_invalid_donation_amount() {
        let mut inp = base_input();
        inp.donation_amount = Decimal::ZERO;
        assert!(compare_philanthropic_vehicles(&inp).is_err());
    }

    #[test]
    fn test_invalid_donor_income() {
        let mut inp = base_input();
        inp.donor_income = dec!(-100);
        assert!(compare_philanthropic_vehicles(&inp).is_err());
    }

    #[test]
    fn test_invalid_payout_rate() {
        let mut inp = base_input();
        inp.payout_rate = dec!(1.5);
        assert!(compare_philanthropic_vehicles(&inp).is_err());
    }

    #[test]
    fn test_invalid_trust_term() {
        let mut inp = base_input();
        inp.trust_term_years = 0;
        assert!(compare_philanthropic_vehicles(&inp).is_err());
    }

    #[test]
    fn test_pv_annuity_helper() {
        // 1000/yr for 10 years at 5%
        let pv = pv_annuity(dec!(1000), dec!(0.05), 10);
        // Should be ~7721.73
        assert!(pv > dec!(7700) && pv < dec!(7750));
    }

    #[test]
    fn test_crt_deduction_decreases_with_higher_payout() {
        let inp1 = base_input();
        let mut inp2 = base_input();
        inp2.payout_rate = dec!(0.08);
        let out1 = compare_philanthropic_vehicles(&inp1).unwrap();
        let out2 = compare_philanthropic_vehicles(&inp2).unwrap();
        let crt1 = out1
            .vehicles
            .iter()
            .find(|v| v.name == "Charitable Remainder Trust")
            .unwrap();
        let crt2 = out2
            .vehicles
            .iter()
            .find(|v| v.name == "Charitable Remainder Trust")
            .unwrap();
        // Higher payout => more retained => lower deduction
        assert!(crt2.tax_deduction < crt1.tax_deduction);
    }

    #[test]
    fn test_all_estate_reductions_equal_donation() {
        let out = compare_philanthropic_vehicles(&base_input()).unwrap();
        for v in &out.vehicles {
            if v.name != "Charitable Lead Trust" {
                assert_eq!(v.estate_reduction, dec!(500_000));
            }
        }
    }
}
