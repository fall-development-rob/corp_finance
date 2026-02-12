//! Net Interest Margin (NIM) analysis.
//!
//! Covers:
//! 1. **NIM calculation** -- net interest income / earning assets.
//! 2. **Weighted-average asset yield and liability cost**.
//! 3. **Rate/volume/mix variance decomposition**.
//! 4. **Interest rate sensitivity gap analysis** (RSA - RSL).
//!
//! All arithmetic uses `rust_decimal::Decimal`. No `f64`.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Input / Output
// ---------------------------------------------------------------------------

/// A single item in the asset mix breakdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetMixItem {
    pub name: String,
    pub balance: Decimal,
    pub yield_rate: Decimal,
}

/// A single item in the liability mix breakdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiabilityMixItem {
    pub name: String,
    pub balance: Decimal,
    pub cost_rate: Decimal,
}

/// Input for NIM analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NimAnalysisInput {
    /// Total interest income for the period.
    pub interest_income: Decimal,
    /// Total interest expense for the period.
    pub interest_expense: Decimal,
    /// Total earning assets (average balance).
    pub earning_assets: Decimal,
    /// Breakdown of earning assets by category.
    pub asset_mix: Vec<AssetMixItem>,
    /// Breakdown of interest-bearing liabilities by category.
    pub liability_mix: Vec<LiabilityMixItem>,
    /// Prior period interest income.
    pub prior_interest_income: Decimal,
    /// Prior period interest expense.
    pub prior_interest_expense: Decimal,
    /// Prior period earning assets.
    pub prior_earning_assets: Decimal,
    /// Rate-sensitive assets for gap analysis.
    pub rate_sensitive_assets: Decimal,
    /// Rate-sensitive liabilities for gap analysis.
    pub rate_sensitive_liabilities: Decimal,
}

/// Rate/volume/mix variance decomposition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateVolumeVariance {
    /// Change in NII attributable to changes in rates.
    pub rate_effect: Decimal,
    /// Change in NII attributable to changes in volume.
    pub volume_effect: Decimal,
    /// Residual (mix/interaction) effect.
    pub mix_effect: Decimal,
}

/// Output of NIM analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NimAnalysisOutput {
    /// Net interest margin (as decimal, e.g. 0.03 = 3%).
    pub nim: Decimal,
    /// Net interest income = interest_income - interest_expense.
    pub net_interest_income: Decimal,
    /// Spread = weighted avg asset yield - weighted avg liability cost.
    pub spread: Decimal,
    /// Weighted average asset yield.
    pub asset_yield: Decimal,
    /// Weighted average liability cost.
    pub liability_cost: Decimal,
    /// Rate/volume/mix variance decomposition.
    pub rate_volume_variance: RateVolumeVariance,
    /// Interest rate sensitivity gap (RSA - RSL).
    pub interest_sensitivity_gap: Decimal,
    /// Gap ratio = gap / earning_assets.
    pub gap_ratio: Decimal,
}

// ---------------------------------------------------------------------------
// Core function
// ---------------------------------------------------------------------------

/// Perform NIM analysis including spread, rate/volume variance, and gap analysis.
pub fn analyze_nim(input: &NimAnalysisInput) -> CorpFinanceResult<NimAnalysisOutput> {
    validate_nim_input(input)?;

    // Net interest income
    let nii = input.interest_income - input.interest_expense;

    // NIM
    let nim = nii / input.earning_assets;

    // Weighted average asset yield
    let asset_yield = weighted_average_yield(&input.asset_mix)?;

    // Weighted average liability cost
    let liability_cost = weighted_average_cost(&input.liability_mix)?;

    // Spread
    let spread = asset_yield - liability_cost;

    // Rate/volume/mix variance
    let prior_nii = input.prior_interest_income - input.prior_interest_expense;

    let rate_volume_variance = if input.prior_earning_assets == Decimal::ZERO {
        // No prior period data -- cannot decompose
        RateVolumeVariance {
            rate_effect: Decimal::ZERO,
            volume_effect: Decimal::ZERO,
            mix_effect: Decimal::ZERO,
        }
    } else {
        let prior_nim = prior_nii / input.prior_earning_assets;
        let total_change = nii - prior_nii;

        // Volume effect: change in earning assets times prior NIM
        let volume_effect = (input.earning_assets - input.prior_earning_assets) * prior_nim;

        // Rate effect: prior earning assets times change in NIM
        let rate_effect = input.prior_earning_assets * (nim - prior_nim);

        // Mix effect: residual
        let mix_effect = total_change - volume_effect - rate_effect;

        RateVolumeVariance {
            rate_effect,
            volume_effect,
            mix_effect,
        }
    };

    // Gap analysis
    let interest_sensitivity_gap = input.rate_sensitive_assets - input.rate_sensitive_liabilities;
    let gap_ratio = interest_sensitivity_gap / input.earning_assets;

    Ok(NimAnalysisOutput {
        nim,
        net_interest_income: nii,
        spread,
        asset_yield,
        liability_cost,
        rate_volume_variance,
        interest_sensitivity_gap,
        gap_ratio,
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn weighted_average_yield(items: &[AssetMixItem]) -> CorpFinanceResult<Decimal> {
    if items.is_empty() {
        return Ok(Decimal::ZERO);
    }
    let total_balance: Decimal = items.iter().map(|a| a.balance).sum();
    if total_balance == Decimal::ZERO {
        return Err(CorpFinanceError::DivisionByZero {
            context: "asset_mix total balance is zero".into(),
        });
    }
    let weighted_sum: Decimal = items.iter().map(|a| a.balance * a.yield_rate).sum();
    Ok(weighted_sum / total_balance)
}

fn weighted_average_cost(items: &[LiabilityMixItem]) -> CorpFinanceResult<Decimal> {
    if items.is_empty() {
        return Ok(Decimal::ZERO);
    }
    let total_balance: Decimal = items.iter().map(|l| l.balance).sum();
    if total_balance == Decimal::ZERO {
        return Err(CorpFinanceError::DivisionByZero {
            context: "liability_mix total balance is zero".into(),
        });
    }
    let weighted_sum: Decimal = items.iter().map(|l| l.balance * l.cost_rate).sum();
    Ok(weighted_sum / total_balance)
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_nim_input(input: &NimAnalysisInput) -> CorpFinanceResult<()> {
    if input.earning_assets <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "earning_assets".into(),
            reason: "Earning assets must be positive.".into(),
        });
    }
    if input.interest_income < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "interest_income".into(),
            reason: "Interest income cannot be negative.".into(),
        });
    }
    if input.interest_expense < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "interest_expense".into(),
            reason: "Interest expense cannot be negative.".into(),
        });
    }
    for item in &input.asset_mix {
        if item.balance < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "asset_mix.balance".into(),
                reason: format!("Asset '{}' has negative balance.", item.name),
            });
        }
    }
    for item in &input.liability_mix {
        if item.balance < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "liability_mix.balance".into(),
                reason: format!("Liability '{}' has negative balance.", item.name),
            });
        }
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

    fn approx_eq(a: Decimal, b: Decimal, eps: Decimal) -> bool {
        (a - b).abs() < eps
    }

    fn sample_assets() -> Vec<AssetMixItem> {
        vec![
            AssetMixItem {
                name: "Commercial Loans".into(),
                balance: dec!(500_000_000),
                yield_rate: dec!(0.055),
            },
            AssetMixItem {
                name: "Mortgage Loans".into(),
                balance: dec!(300_000_000),
                yield_rate: dec!(0.04),
            },
            AssetMixItem {
                name: "Securities".into(),
                balance: dec!(200_000_000),
                yield_rate: dec!(0.03),
            },
        ]
    }

    fn sample_liabilities() -> Vec<LiabilityMixItem> {
        vec![
            LiabilityMixItem {
                name: "Demand Deposits".into(),
                balance: dec!(400_000_000),
                cost_rate: dec!(0.005),
            },
            LiabilityMixItem {
                name: "Time Deposits".into(),
                balance: dec!(300_000_000),
                cost_rate: dec!(0.025),
            },
            LiabilityMixItem {
                name: "Wholesale Funding".into(),
                balance: dec!(200_000_000),
                cost_rate: dec!(0.035),
            },
        ]
    }

    fn base_input() -> NimAnalysisInput {
        NimAnalysisInput {
            interest_income: dec!(45_000_000),
            interest_expense: dec!(17_000_000),
            earning_assets: dec!(1_000_000_000),
            asset_mix: sample_assets(),
            liability_mix: sample_liabilities(),
            prior_interest_income: dec!(42_000_000),
            prior_interest_expense: dec!(15_000_000),
            prior_earning_assets: dec!(950_000_000),
            rate_sensitive_assets: dec!(600_000_000),
            rate_sensitive_liabilities: dec!(500_000_000),
        }
    }

    #[test]
    fn test_basic_nim_calculation() {
        let input = base_input();
        let out = analyze_nim(&input).unwrap();
        // NIM = (45M - 17M) / 1B = 0.028
        assert_eq!(out.nim, dec!(0.028));
        assert_eq!(out.net_interest_income, dec!(28_000_000));
    }

    #[test]
    fn test_nim_as_percentage() {
        let input = base_input();
        let out = analyze_nim(&input).unwrap();
        // 2.8% NIM is typical for a bank
        let nim_pct = out.nim * dec!(100);
        assert!(nim_pct > dec!(2) && nim_pct < dec!(5));
    }

    #[test]
    fn test_weighted_avg_asset_yield() {
        let input = base_input();
        let out = analyze_nim(&input).unwrap();
        // 500M*5.5% + 300M*4% + 200M*3% = 27.5M+12M+6M = 45.5M
        // Total assets = 1B -> yield = 0.0455
        let expected = dec!(0.0455);
        assert!(approx_eq(out.asset_yield, expected, dec!(0.0001)));
    }

    #[test]
    fn test_weighted_avg_liability_cost() {
        let input = base_input();
        let out = analyze_nim(&input).unwrap();
        // 400M*0.5% + 300M*2.5% + 200M*3.5% = 2M+7.5M+7M = 16.5M
        // Total liabilities = 900M -> cost = 0.01833...
        let expected = dec!(16_500_000) / dec!(900_000_000);
        assert!(approx_eq(out.liability_cost, expected, dec!(0.00001)));
    }

    #[test]
    fn test_spread_positive() {
        let input = base_input();
        let out = analyze_nim(&input).unwrap();
        assert!(
            out.spread > Decimal::ZERO,
            "Spread should be positive for a profitable bank"
        );
    }

    #[test]
    fn test_spread_equals_yield_minus_cost() {
        let input = base_input();
        let out = analyze_nim(&input).unwrap();
        let expected_spread = out.asset_yield - out.liability_cost;
        assert!(approx_eq(out.spread, expected_spread, dec!(0.000001)));
    }

    #[test]
    fn test_rate_volume_variance_sums_to_total_change() {
        let input = base_input();
        let out = analyze_nim(&input).unwrap();
        let prior_nii = dec!(42_000_000) - dec!(15_000_000); // 27M
        let total_change = out.net_interest_income - prior_nii;
        let variance_sum = out.rate_volume_variance.rate_effect
            + out.rate_volume_variance.volume_effect
            + out.rate_volume_variance.mix_effect;
        assert!(
            approx_eq(variance_sum, total_change, dec!(0.01)),
            "Rate+Volume+Mix should sum to total NII change. Got {}, expected {}",
            variance_sum,
            total_change
        );
    }

    #[test]
    fn test_volume_effect_positive_when_assets_grow() {
        let input = base_input();
        let out = analyze_nim(&input).unwrap();
        // Earning assets grew from 950M to 1B, prior NII was positive
        assert!(
            out.rate_volume_variance.volume_effect > Decimal::ZERO,
            "Volume effect should be positive when assets grow"
        );
    }

    #[test]
    fn test_gap_analysis() {
        let input = base_input();
        let out = analyze_nim(&input).unwrap();
        // Gap = 600M - 500M = 100M
        assert_eq!(out.interest_sensitivity_gap, dec!(100_000_000));
    }

    #[test]
    fn test_gap_ratio() {
        let input = base_input();
        let out = analyze_nim(&input).unwrap();
        // Gap ratio = 100M / 1B = 0.1
        assert_eq!(out.gap_ratio, dec!(0.1));
    }

    #[test]
    fn test_negative_gap() {
        let mut input = base_input();
        input.rate_sensitive_assets = dec!(400_000_000);
        input.rate_sensitive_liabilities = dec!(600_000_000);
        let out = analyze_nim(&input).unwrap();
        assert_eq!(out.interest_sensitivity_gap, dec!(-200_000_000));
        assert!(out.gap_ratio < Decimal::ZERO);
    }

    #[test]
    fn test_zero_earning_assets_rejected() {
        let mut input = base_input();
        input.earning_assets = Decimal::ZERO;
        assert!(analyze_nim(&input).is_err());
    }

    #[test]
    fn test_negative_earning_assets_rejected() {
        let mut input = base_input();
        input.earning_assets = dec!(-100);
        assert!(analyze_nim(&input).is_err());
    }

    #[test]
    fn test_negative_interest_income_rejected() {
        let mut input = base_input();
        input.interest_income = dec!(-1);
        assert!(analyze_nim(&input).is_err());
    }

    #[test]
    fn test_negative_asset_balance_rejected() {
        let mut input = base_input();
        input.asset_mix[0].balance = dec!(-100);
        assert!(analyze_nim(&input).is_err());
    }

    #[test]
    fn test_negative_liability_balance_rejected() {
        let mut input = base_input();
        input.liability_mix[0].balance = dec!(-100);
        assert!(analyze_nim(&input).is_err());
    }

    #[test]
    fn test_empty_asset_mix_yields_zero() {
        let mut input = base_input();
        input.asset_mix = vec![];
        let out = analyze_nim(&input).unwrap();
        assert_eq!(out.asset_yield, Decimal::ZERO);
    }

    #[test]
    fn test_empty_liability_mix_yields_zero() {
        let mut input = base_input();
        input.liability_mix = vec![];
        let out = analyze_nim(&input).unwrap();
        assert_eq!(out.liability_cost, Decimal::ZERO);
    }

    #[test]
    fn test_rising_rate_scenario() {
        // Higher current rates, same volume
        let input = NimAnalysisInput {
            interest_income: dec!(55_000_000),
            interest_expense: dec!(25_000_000),
            earning_assets: dec!(1_000_000_000),
            asset_mix: vec![AssetMixItem {
                name: "Loans".into(),
                balance: dec!(1_000_000_000),
                yield_rate: dec!(0.055),
            }],
            liability_mix: vec![LiabilityMixItem {
                name: "Deposits".into(),
                balance: dec!(800_000_000),
                cost_rate: dec!(0.03125),
            }],
            prior_interest_income: dec!(45_000_000),
            prior_interest_expense: dec!(17_000_000),
            prior_earning_assets: dec!(1_000_000_000),
            rate_sensitive_assets: dec!(700_000_000),
            rate_sensitive_liabilities: dec!(500_000_000),
        };
        let out = analyze_nim(&input).unwrap();
        // NIM = (55M-25M)/1B = 0.03
        assert_eq!(out.nim, dec!(0.03));
        // Volume effect should be 0 (same earning assets)
        assert_eq!(out.rate_volume_variance.volume_effect, Decimal::ZERO);
        // Rate effect should capture the improvement
        assert!(out.rate_volume_variance.rate_effect > Decimal::ZERO);
    }

    #[test]
    fn test_prior_earning_assets_zero_gives_zero_variance() {
        let mut input = base_input();
        input.prior_earning_assets = Decimal::ZERO;
        let out = analyze_nim(&input).unwrap();
        assert_eq!(out.rate_volume_variance.rate_effect, Decimal::ZERO);
        assert_eq!(out.rate_volume_variance.volume_effect, Decimal::ZERO);
        assert_eq!(out.rate_volume_variance.mix_effect, Decimal::ZERO);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let input = base_input();
        let out = analyze_nim(&input).unwrap();
        let json = serde_json::to_string(&out).unwrap();
        let _: NimAnalysisOutput = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_typical_community_bank() {
        let input = NimAnalysisInput {
            interest_income: dec!(12_000_000),
            interest_expense: dec!(3_000_000),
            earning_assets: dec!(300_000_000),
            asset_mix: vec![
                AssetMixItem {
                    name: "CRE Loans".into(),
                    balance: dec!(150_000_000),
                    yield_rate: dec!(0.05),
                },
                AssetMixItem {
                    name: "C&I Loans".into(),
                    balance: dec!(100_000_000),
                    yield_rate: dec!(0.045),
                },
                AssetMixItem {
                    name: "Treasuries".into(),
                    balance: dec!(50_000_000),
                    yield_rate: dec!(0.025),
                },
            ],
            liability_mix: vec![
                LiabilityMixItem {
                    name: "Core Deposits".into(),
                    balance: dec!(200_000_000),
                    cost_rate: dec!(0.008),
                },
                LiabilityMixItem {
                    name: "CDs".into(),
                    balance: dec!(50_000_000),
                    cost_rate: dec!(0.03),
                },
            ],
            prior_interest_income: dec!(11_500_000),
            prior_interest_expense: dec!(2_800_000),
            prior_earning_assets: dec!(290_000_000),
            rate_sensitive_assets: dec!(180_000_000),
            rate_sensitive_liabilities: dec!(120_000_000),
        };
        let out = analyze_nim(&input).unwrap();
        // NIM = 9M / 300M = 0.03
        assert_eq!(out.nim, dec!(0.03));
        assert!(out.spread > Decimal::ZERO);
        assert_eq!(out.interest_sensitivity_gap, dec!(60_000_000));
    }
}
