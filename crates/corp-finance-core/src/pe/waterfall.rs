use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::*;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Input types
// ---------------------------------------------------------------------------

/// Input for a PE fund cash-flow waterfall distribution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaterfallInput {
    /// Total exit proceeds available for distribution
    pub total_proceeds: Money,
    /// Total capital invested by the fund
    pub total_invested: Money,
    /// Ordered waterfall tiers (executed top-to-bottom)
    pub tiers: Vec<WaterfallTier>,
    /// GP commitment as a fraction of fund (typically 0.01 - 0.05)
    pub gp_commitment_pct: Rate,
}

/// A single tier in the distribution waterfall.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaterfallTier {
    /// Human-readable tier name
    pub name: String,
    /// Distribution logic for this tier
    pub tier_type: WaterfallTierType,
}

/// Distribution mechanics for a waterfall tier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WaterfallTierType {
    /// Return of contributed capital pro-rata GP/LP
    ReturnOfCapital,
    /// Preferred return (hurdle) on invested capital
    PreferredReturn { rate: Rate },
    /// GP catch-up until GP carry equals target share of total profit
    CatchUp { gp_share: Rate },
    /// Carried interest split of remaining proceeds
    CarriedInterest { gp_share: Rate },
    /// Residual split (same mechanics as CarriedInterest)
    Residual { gp_share: Rate },
}

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// Full waterfall distribution result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaterfallOutput {
    /// Per-tier breakdown
    pub tiers: Vec<WaterfallTierResult>,
    /// Total distributions to the GP
    pub total_to_gp: Money,
    /// Total distributions to LPs
    pub total_to_lp: Money,
    /// GP share of total proceeds (decimal)
    pub gp_pct_of_total: Rate,
    /// LP share of total proceeds (decimal)
    pub lp_pct_of_total: Rate,
    /// GP carry (excludes GP co-invest return of capital and preferred)
    pub gp_carry: Money,
    /// GP return attributable to its co-investment
    pub gp_co_invest_return: Money,
}

/// Result for a single waterfall tier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaterfallTierResult {
    /// Tier name (copied from input)
    pub tier_name: String,
    /// Total amount distributed in this tier
    pub amount: Money,
    /// Amount allocated to the GP
    pub to_gp: Money,
    /// Amount allocated to LPs
    pub to_lp: Money,
    /// Proceeds remaining after this tier
    pub remaining: Money,
}

// ---------------------------------------------------------------------------
// Calculation
// ---------------------------------------------------------------------------

/// Calculate a PE fund cash-flow waterfall (European-style).
///
/// Distributes `total_proceeds` through an ordered set of tiers (return of
/// capital, preferred return, GP catch-up, carried interest, residual) and
/// tracks GP vs LP allocations throughout.
pub fn calculate_waterfall(
    input: &WaterfallInput,
) -> CorpFinanceResult<ComputationOutput<WaterfallOutput>> {
    let start = Instant::now();
    let warnings: Vec<String> = Vec::new();

    // --- Validation ---
    if input.total_proceeds < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "total_proceeds".into(),
            reason: "Total proceeds cannot be negative".into(),
        });
    }
    if input.total_invested <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "total_invested".into(),
            reason: "Total invested must be positive".into(),
        });
    }
    if input.gp_commitment_pct < Decimal::ZERO || input.gp_commitment_pct > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "gp_commitment_pct".into(),
            reason: "GP commitment percentage must be between 0 and 1".into(),
        });
    }
    if input.tiers.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "tiers".into(),
            reason: "At least one waterfall tier is required".into(),
        });
    }

    let gp_pct = input.gp_commitment_pct;

    let mut remaining = input.total_proceeds;
    let mut tier_results: Vec<WaterfallTierResult> = Vec::new();

    // Accumulators for GP co-invest amounts (return of capital + preferred)
    let mut gp_co_invest_roc = Decimal::ZERO;
    let mut gp_co_invest_pref = Decimal::ZERO;

    // Track cumulative LP preferred (needed for catch-up target calculation)
    let mut cumulative_lp_preferred = Decimal::ZERO;

    // Determine the carry rate from the first CarriedInterest or Residual tier.
    // This is needed to compute the catch-up target.
    let carry_rate = input
        .tiers
        .iter()
        .find_map(|t| match &t.tier_type {
            WaterfallTierType::CarriedInterest { gp_share } => Some(*gp_share),
            WaterfallTierType::Residual { gp_share } => Some(*gp_share),
            _ => None,
        })
        .unwrap_or(dec!(0.20));

    for tier in &input.tiers {
        let tier_result = match &tier.tier_type {
            WaterfallTierType::ReturnOfCapital => {
                let distributable = remaining.min(input.total_invested);
                let to_gp = distributable * gp_pct;
                let to_lp = distributable - to_gp;
                remaining -= distributable;
                gp_co_invest_roc = to_gp;
                WaterfallTierResult {
                    tier_name: tier.name.clone(),
                    amount: distributable,
                    to_gp,
                    to_lp,
                    remaining,
                }
            }
            WaterfallTierType::PreferredReturn { rate } => {
                let preferred_total = input.total_invested * *rate;
                let distributable = remaining.min(preferred_total);
                let to_gp = distributable * gp_pct;
                let to_lp = distributable - to_gp;
                remaining -= distributable;
                gp_co_invest_pref = to_gp;
                cumulative_lp_preferred = to_lp;
                WaterfallTierResult {
                    tier_name: tier.name.clone(),
                    amount: distributable,
                    to_gp,
                    to_lp,
                    remaining,
                }
            }
            WaterfallTierType::CatchUp { gp_share } => {
                // The GP catch-up target: GP should receive
                // carry_rate / (1 - carry_rate) * LP_preferred
                // so that total profit split reaches the target ratio.
                let target_catchup = if carry_rate < Decimal::ONE {
                    (carry_rate / (Decimal::ONE - carry_rate)) * cumulative_lp_preferred
                } else {
                    remaining // 100% carry => catch up takes everything
                };

                let distributable = remaining.min(target_catchup).max(Decimal::ZERO);
                let to_gp = distributable * *gp_share;
                let to_lp = distributable - to_gp;
                remaining -= distributable;
                WaterfallTierResult {
                    tier_name: tier.name.clone(),
                    amount: distributable,
                    to_gp,
                    to_lp,
                    remaining,
                }
            }
            WaterfallTierType::CarriedInterest { gp_share }
            | WaterfallTierType::Residual { gp_share } => {
                let distributable = remaining;
                let to_gp = distributable * *gp_share;
                let to_lp = distributable - to_gp;
                remaining = Decimal::ZERO;
                WaterfallTierResult {
                    tier_name: tier.name.clone(),
                    amount: distributable,
                    to_gp,
                    to_lp,
                    remaining,
                }
            }
        };

        tier_results.push(tier_result);
    }

    // Aggregate totals
    let total_to_gp: Money = tier_results.iter().map(|t| t.to_gp).sum();
    let total_to_lp: Money = tier_results.iter().map(|t| t.to_lp).sum();

    let (gp_pct_of_total, lp_pct_of_total) = if input.total_proceeds.is_zero() {
        (Decimal::ZERO, Decimal::ZERO)
    } else {
        (
            total_to_gp / input.total_proceeds,
            total_to_lp / input.total_proceeds,
        )
    };

    // GP carry = total GP distributions minus the GP's pro-rata share of
    // return of capital and preferred return
    let gp_co_invest_return = gp_co_invest_roc + gp_co_invest_pref;
    let gp_carry = total_to_gp - gp_co_invest_return;

    let output = WaterfallOutput {
        tiers: tier_results,
        total_to_gp,
        total_to_lp,
        gp_pct_of_total,
        lp_pct_of_total,
        gp_carry,
        gp_co_invest_return,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "PE Cash-Flow Waterfall (European)",
        &serde_json::json!({
            "total_proceeds": input.total_proceeds.to_string(),
            "total_invested": input.total_invested.to_string(),
            "gp_commitment_pct": input.gp_commitment_pct.to_string(),
            "num_tiers": input.tiers.len(),
        }),
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// Helper: standard European waterfall (8% pref, 100% catch-up, 80/20)
    fn european_waterfall(
        total_proceeds: Money,
        total_invested: Money,
        gp_commitment_pct: Rate,
    ) -> WaterfallInput {
        WaterfallInput {
            total_proceeds,
            total_invested,
            tiers: vec![
                WaterfallTier {
                    name: "Return of Capital".into(),
                    tier_type: WaterfallTierType::ReturnOfCapital,
                },
                WaterfallTier {
                    name: "Preferred Return".into(),
                    tier_type: WaterfallTierType::PreferredReturn { rate: dec!(0.08) },
                },
                WaterfallTier {
                    name: "GP Catch-Up".into(),
                    tier_type: WaterfallTierType::CatchUp {
                        gp_share: dec!(1.0),
                    },
                },
                WaterfallTier {
                    name: "Carried Interest".into(),
                    tier_type: WaterfallTierType::CarriedInterest {
                        gp_share: dec!(0.20),
                    },
                },
            ],
            gp_commitment_pct,
        }
    }

    #[test]
    fn test_basic_european_waterfall() {
        // Fund: $100M invested, $200M proceeds, GP commits 2%
        let input = european_waterfall(dec!(200), dec!(100), dec!(0.02));
        let result = calculate_waterfall(&input).unwrap();
        let out = &result.result;

        // Tier 0: Return of Capital = $100M
        assert_eq!(out.tiers[0].amount, dec!(100));
        assert_eq!(out.tiers[0].to_gp, dec!(2)); // 2% of $100M
        assert_eq!(out.tiers[0].to_lp, dec!(98));
        assert_eq!(out.tiers[0].remaining, dec!(100));

        // Tier 1: Preferred Return = 8% * $100M = $8M
        assert_eq!(out.tiers[1].amount, dec!(8));
        // GP gets 2% of $8M = $0.16
        assert_eq!(out.tiers[1].to_gp, dec!(0.16));
        assert_eq!(out.tiers[1].to_lp, dec!(7.84));
        assert_eq!(out.tiers[1].remaining, dec!(92));

        // Tier 2: GP Catch-Up
        // LP preferred = $7.84. Catch-up target = 0.20 / 0.80 * 7.84 = $1.96
        let expected_catchup = dec!(0.20) / dec!(0.80) * dec!(7.84);
        assert_eq!(out.tiers[2].amount, expected_catchup);
        // 100% to GP
        assert_eq!(out.tiers[2].to_gp, expected_catchup);
        assert_eq!(out.tiers[2].to_lp, dec!(0));

        // Tier 3: Carried Interest on remaining
        let remaining_after_catchup = dec!(92) - expected_catchup;
        assert_eq!(out.tiers[3].amount, remaining_after_catchup);
        // 20% to GP, 80% to LP
        assert_eq!(out.tiers[3].to_gp, remaining_after_catchup * dec!(0.20));
        assert_eq!(
            out.tiers[3].to_lp,
            remaining_after_catchup - remaining_after_catchup * dec!(0.20)
        );

        // Totals
        assert_eq!(out.total_to_gp + out.total_to_lp, dec!(200));
        assert!(out.gp_carry > Decimal::ZERO);
        // GP carry should not include the co-invest return
        assert_eq!(out.gp_co_invest_return, dec!(2) + dec!(0.16));
        assert_eq!(out.gp_carry, out.total_to_gp - out.gp_co_invest_return);
    }

    #[test]
    fn test_return_of_capital_only() {
        // Proceeds less than invested -- only partial return of capital
        let input = european_waterfall(dec!(60), dec!(100), dec!(0.02));
        let result = calculate_waterfall(&input).unwrap();
        let out = &result.result;

        // Only $60M returned (< $100M invested)
        assert_eq!(out.tiers[0].amount, dec!(60));
        assert_eq!(out.tiers[0].to_gp, dec!(1.2)); // 2% of $60
        assert_eq!(out.tiers[0].to_lp, dec!(58.8));

        // All subsequent tiers should be zero
        for tier in &out.tiers[1..] {
            assert_eq!(tier.amount, Decimal::ZERO);
        }

        assert_eq!(out.total_to_gp + out.total_to_lp, dec!(60));
        // No carry when capital not fully returned
        assert_eq!(out.gp_carry, Decimal::ZERO);
    }

    #[test]
    fn test_no_carry_below_hurdle() {
        // Proceeds = invested + partial preferred (no catch-up or carry)
        // $100M invested, $105M proceeds => only $5M of $8M pref is paid
        let input = european_waterfall(dec!(105), dec!(100), dec!(0.02));
        let result = calculate_waterfall(&input).unwrap();
        let out = &result.result;

        // ROC fully covered
        assert_eq!(out.tiers[0].amount, dec!(100));
        // Preferred: only $5M distributed (< $8M target)
        assert_eq!(out.tiers[1].amount, dec!(5));
        // Catch-up and carry are zero
        assert_eq!(out.tiers[2].amount, Decimal::ZERO);
        assert_eq!(out.tiers[3].amount, Decimal::ZERO);

        // GP carry should be zero since only co-invest portions were received
        assert_eq!(out.gp_carry, Decimal::ZERO);
    }

    #[test]
    fn test_full_catch_up() {
        // Enough proceeds to fully catch up but not much beyond
        // $100M invested, $110M proceeds, 2% GP commitment
        // ROC = $100M. Remaining = $10M.
        // Preferred = $8M. Remaining = $2M.
        // LP pref = $8M * 0.98 = $7.84
        // Catch-up target = 0.20/0.80 * 7.84 = $1.96
        // Catch-up distributable = min($2, $1.96) = $1.96 (all to GP)
        // Carried Interest on remaining $0.04
        let input = european_waterfall(dec!(110), dec!(100), dec!(0.02));
        let result = calculate_waterfall(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.tiers[0].amount, dec!(100));
        assert_eq!(out.tiers[1].amount, dec!(8));
        assert_eq!(out.tiers[1].to_lp, dec!(7.84));

        let expected_catchup = dec!(0.25) * dec!(7.84); // 0.20/0.80 = 0.25
        assert_eq!(out.tiers[2].amount, expected_catchup);
        assert_eq!(out.tiers[2].to_gp, expected_catchup);

        let carry_remaining = dec!(2) - expected_catchup;
        assert_eq!(out.tiers[3].amount, carry_remaining);

        // Total distributions equal proceeds
        assert_eq!(out.total_to_gp + out.total_to_lp, dec!(110));
    }

    #[test]
    fn test_gp_commitment_allocation() {
        // Verify GP gets pro-rata share of return of capital
        let input = european_waterfall(dec!(200), dec!(100), dec!(0.05));
        let result = calculate_waterfall(&input).unwrap();
        let out = &result.result;

        // ROC: GP should get 5% of $100M = $5M
        assert_eq!(out.tiers[0].to_gp, dec!(5));
        assert_eq!(out.tiers[0].to_lp, dec!(95));

        // Preferred: GP should get 5% of $8M = $0.40
        assert_eq!(out.tiers[1].to_gp, dec!(0.40));
        assert_eq!(out.tiers[1].to_lp, dec!(7.60));

        // GP co-invest return = $5 + $0.40 = $5.40
        assert_eq!(out.gp_co_invest_return, dec!(5.40));
    }

    #[test]
    fn test_zero_proceeds() {
        let input = european_waterfall(dec!(0), dec!(100), dec!(0.02));
        let result = calculate_waterfall(&input).unwrap();
        let out = &result.result;

        for tier in &out.tiers {
            assert_eq!(tier.amount, Decimal::ZERO);
            assert_eq!(tier.to_gp, Decimal::ZERO);
            assert_eq!(tier.to_lp, Decimal::ZERO);
        }

        assert_eq!(out.total_to_gp, Decimal::ZERO);
        assert_eq!(out.total_to_lp, Decimal::ZERO);
        assert_eq!(out.gp_pct_of_total, Decimal::ZERO);
        assert_eq!(out.lp_pct_of_total, Decimal::ZERO);
        assert_eq!(out.gp_carry, Decimal::ZERO);
    }

    #[test]
    fn test_high_return_scenario() {
        // 3x MOIC: $100M invested, $300M proceeds, 2% GP commitment
        let input = european_waterfall(dec!(300), dec!(100), dec!(0.02));
        let result = calculate_waterfall(&input).unwrap();
        let out = &result.result;

        // ROC: $100M
        assert_eq!(out.tiers[0].amount, dec!(100));
        // Preferred: $8M
        assert_eq!(out.tiers[1].amount, dec!(8));

        // After ROC + pref: $192M remaining
        // LP pref = $7.84. Catch-up = 0.25 * 7.84 = $1.96
        let catchup = dec!(0.25) * dec!(7.84);
        assert_eq!(out.tiers[2].amount, catchup);

        // Carried interest on remaining $192 - $1.96 = $190.04
        let carry_base = dec!(192) - catchup;
        assert_eq!(out.tiers[3].amount, carry_base);
        assert_eq!(out.tiers[3].to_gp, carry_base * dec!(0.20));
        assert_eq!(out.tiers[3].to_lp, carry_base - carry_base * dec!(0.20));

        // Total = $300M
        assert_eq!(out.total_to_gp + out.total_to_lp, dec!(300));

        // GP carry should be substantial at 3x
        assert!(out.gp_carry > dec!(35)); // roughly ~$40M carry
    }

    #[test]
    fn test_no_preferred_return() {
        // Waterfall with only ROC and 80/20 carry split (no pref, no catch-up)
        let input = WaterfallInput {
            total_proceeds: dec!(200),
            total_invested: dec!(100),
            tiers: vec![
                WaterfallTier {
                    name: "Return of Capital".into(),
                    tier_type: WaterfallTierType::ReturnOfCapital,
                },
                WaterfallTier {
                    name: "Profit Split".into(),
                    tier_type: WaterfallTierType::CarriedInterest {
                        gp_share: dec!(0.20),
                    },
                },
            ],
            gp_commitment_pct: dec!(0.02),
        };
        let result = calculate_waterfall(&input).unwrap();
        let out = &result.result;

        // ROC: $100M
        assert_eq!(out.tiers[0].amount, dec!(100));
        // Carry on $100M profit
        assert_eq!(out.tiers[1].amount, dec!(100));
        assert_eq!(out.tiers[1].to_gp, dec!(20));
        assert_eq!(out.tiers[1].to_lp, dec!(80));

        // GP carry = total GP - co-invest return
        // co-invest return = 2% * $100M ROC = $2
        assert_eq!(out.gp_co_invest_return, dec!(2));
        assert_eq!(out.gp_carry, out.total_to_gp - dec!(2));
        assert_eq!(out.total_to_gp + out.total_to_lp, dec!(200));
    }

    #[test]
    fn test_invalid_negative_proceeds() {
        let input = european_waterfall(dec!(-50), dec!(100), dec!(0.02));
        let result = calculate_waterfall(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "total_proceeds");
            }
            other => panic!("Expected InvalidInput, got: {other:?}"),
        }
    }

    #[test]
    fn test_invalid_zero_invested() {
        let input = WaterfallInput {
            total_proceeds: dec!(100),
            total_invested: dec!(0),
            tiers: vec![WaterfallTier {
                name: "ROC".into(),
                tier_type: WaterfallTierType::ReturnOfCapital,
            }],
            gp_commitment_pct: dec!(0.02),
        };
        let result = calculate_waterfall(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_gp_commitment_pct() {
        let input = WaterfallInput {
            total_proceeds: dec!(100),
            total_invested: dec!(50),
            tiers: vec![WaterfallTier {
                name: "ROC".into(),
                tier_type: WaterfallTierType::ReturnOfCapital,
            }],
            gp_commitment_pct: dec!(1.5), // > 1, invalid
        };
        let result = calculate_waterfall(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_residual_tier() {
        // Verify Residual tier behaves like CarriedInterest
        let input = WaterfallInput {
            total_proceeds: dec!(150),
            total_invested: dec!(100),
            tiers: vec![
                WaterfallTier {
                    name: "Return of Capital".into(),
                    tier_type: WaterfallTierType::ReturnOfCapital,
                },
                WaterfallTier {
                    name: "Residual Split".into(),
                    tier_type: WaterfallTierType::Residual {
                        gp_share: dec!(0.30),
                    },
                },
            ],
            gp_commitment_pct: dec!(0.01),
        };
        let result = calculate_waterfall(&input).unwrap();
        let out = &result.result;

        // ROC = $100M, Residual = $50M
        assert_eq!(out.tiers[1].amount, dec!(50));
        assert_eq!(out.tiers[1].to_gp, dec!(15)); // 30% of $50
        assert_eq!(out.tiers[1].to_lp, dec!(35)); // 70% of $50
        assert_eq!(out.total_to_gp + out.total_to_lp, dec!(150));
    }

    #[test]
    fn test_pct_of_total() {
        let input = european_waterfall(dec!(200), dec!(100), dec!(0.02));
        let result = calculate_waterfall(&input).unwrap();
        let out = &result.result;

        // GP% + LP% should equal 1
        let sum = out.gp_pct_of_total + out.lp_pct_of_total;
        assert_eq!(sum, Decimal::ONE);
    }
}
