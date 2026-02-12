//! Share Buyback Analysis.
//!
//! Covers:
//! 1. **Shares repurchased** and post-buyback share count.
//! 2. **EPS accretion/dilution** — adjusting for funding source.
//! 3. **Buyback yield** — buyback amount as % of market cap.
//! 4. **Tax efficiency** — buyback vs equivalent dividend distribution.
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

/// Input for share buyback analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuybackInput {
    /// Shares outstanding (millions).
    pub current_shares: Decimal,
    /// Earnings per share.
    pub current_eps: Decimal,
    /// Current share price.
    pub current_price: Decimal,
    /// Total capital deployed for buyback.
    pub buyback_amount: Decimal,
    /// After-tax cost of debt (if debt-funded).
    pub cost_of_debt: Decimal,
    /// Marginal corporate tax rate.
    pub tax_rate: Decimal,
    /// Personal tax rate on dividends.
    pub dividend_tax_rate: Decimal,
    /// Personal tax rate on capital gains.
    pub capital_gains_tax_rate: Decimal,
    /// Funding source: "cash", "debt", or "mixed".
    pub funding_source: String,
}

/// Output of share buyback analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuybackOutput {
    /// Number of shares repurchased (millions).
    pub shares_repurchased: Decimal,
    /// Post-buyback shares outstanding (millions).
    pub new_shares: Decimal,
    /// Pre-buyback EPS.
    pub pre_buyback_eps: Decimal,
    /// Post-buyback EPS.
    pub post_buyback_eps: Decimal,
    /// EPS accretion as a percentage: (post - pre) / pre * 100.
    pub eps_accretion: Decimal,
    /// Whether the buyback is EPS-accretive.
    pub eps_accretive: bool,
    /// Buyback yield: buyback_amount / market_cap.
    pub buyback_yield: Decimal,
    /// Equivalent dividend per share (same total payout).
    pub equivalent_dividend: Decimal,
    /// Tax cost if distributed as dividend.
    pub dividend_tax_cost: Decimal,
    /// Approximate tax cost of buyback (cap gains on assumed 50% gain).
    pub buyback_tax_cost: Decimal,
    /// Tax savings of buyback vs dividend: (dividend_tax - buyback_tax) / dividend_tax.
    pub tax_efficiency: Decimal,
}

// ---------------------------------------------------------------------------
// Core function
// ---------------------------------------------------------------------------

/// Analyze a share buyback: EPS impact, yield, and tax efficiency.
pub fn calculate_buyback(input: &BuybackInput) -> CorpFinanceResult<BuybackOutput> {
    validate_input(input)?;

    let shares = input.current_shares;
    let price = input.current_price;
    let buyback = input.buyback_amount;

    // Shares repurchased
    let shares_repurchased = buyback / price;
    let new_shares = shares - shares_repurchased;

    if new_shares <= Decimal::ZERO {
        return Err(CorpFinanceError::FinancialImpossibility(
            "Buyback amount would repurchase all or more than outstanding shares.".into(),
        ));
    }

    // Total net income (pre-buyback)
    let total_net_income = input.current_eps * shares;

    // Adjust net income for funding source
    let funding = input.funding_source.to_lowercase();
    let adjusted_net_income = match funding.as_str() {
        "debt" => {
            // Interest expense reduces net income (after tax)
            let interest_cost = buyback * input.cost_of_debt * (Decimal::ONE - input.tax_rate);
            total_net_income - interest_cost
        }
        "cash" | "mixed" => {
            // Cash: no earnings impact (ignoring opportunity cost)
            // Mixed: treat same as cash for simplicity
            total_net_income
        }
        _ => {
            return Err(CorpFinanceError::InvalidInput {
                field: "funding_source".into(),
                reason: "Must be 'cash', 'debt', or 'mixed'.".into(),
            });
        }
    };

    // EPS calculations
    let pre_buyback_eps = input.current_eps;
    let post_buyback_eps = adjusted_net_income / new_shares;
    let eps_accretion = if pre_buyback_eps == Decimal::ZERO {
        Decimal::ZERO
    } else {
        (post_buyback_eps - pre_buyback_eps) / pre_buyback_eps.abs() * dec!(100)
    };
    let eps_accretive = post_buyback_eps > pre_buyback_eps;

    // Buyback yield
    let market_cap = price * shares;
    let buyback_yield = if market_cap == Decimal::ZERO {
        Decimal::ZERO
    } else {
        buyback / market_cap
    };

    // Equivalent dividend: same total payout distributed as DPS
    let equivalent_dividend = buyback / shares;

    // Tax comparison
    // Dividend tax: entire distribution taxed at dividend rate
    let dividend_tax_cost = equivalent_dividend * input.dividend_tax_rate * shares;
    // Buyback tax: only the gain portion is taxed; assume 50% of buyback is gain
    let assumed_gain_portion = dec!(0.50);
    let buyback_tax_cost = buyback * assumed_gain_portion * input.capital_gains_tax_rate;

    // Tax efficiency: savings relative to dividend approach
    let tax_efficiency = if dividend_tax_cost == Decimal::ZERO {
        Decimal::ZERO
    } else {
        (dividend_tax_cost - buyback_tax_cost) / dividend_tax_cost * dec!(100)
    };

    Ok(BuybackOutput {
        shares_repurchased,
        new_shares,
        pre_buyback_eps,
        post_buyback_eps,
        eps_accretion,
        eps_accretive,
        buyback_yield,
        equivalent_dividend,
        dividend_tax_cost,
        buyback_tax_cost,
        tax_efficiency,
    })
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_input(input: &BuybackInput) -> CorpFinanceResult<()> {
    if input.current_shares <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "current_shares".into(),
            reason: "Shares outstanding must be positive.".into(),
        });
    }
    if input.current_price <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "current_price".into(),
            reason: "Share price must be positive.".into(),
        });
    }
    if input.buyback_amount <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "buyback_amount".into(),
            reason: "Buyback amount must be positive.".into(),
        });
    }
    if input.tax_rate < Decimal::ZERO || input.tax_rate > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "tax_rate".into(),
            reason: "Tax rate must be between 0 and 1.".into(),
        });
    }
    if input.dividend_tax_rate < Decimal::ZERO || input.dividend_tax_rate > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "dividend_tax_rate".into(),
            reason: "Dividend tax rate must be between 0 and 1.".into(),
        });
    }
    if input.capital_gains_tax_rate < Decimal::ZERO || input.capital_gains_tax_rate > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "capital_gains_tax_rate".into(),
            reason: "Capital gains tax rate must be between 0 and 1.".into(),
        });
    }
    if input.cost_of_debt < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "cost_of_debt".into(),
            reason: "Cost of debt must be non-negative.".into(),
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

    fn approx_eq(a: Decimal, b: Decimal, eps: Decimal) -> bool {
        (a - b).abs() < eps
    }

    fn cash_funded_input() -> BuybackInput {
        BuybackInput {
            current_shares: dec!(100),
            current_eps: dec!(5.00),
            current_price: dec!(50.00),
            buyback_amount: dec!(500),
            cost_of_debt: dec!(0.05),
            tax_rate: dec!(0.25),
            dividend_tax_rate: dec!(0.20),
            capital_gains_tax_rate: dec!(0.15),
            funding_source: "cash".into(),
        }
    }

    fn debt_funded_input() -> BuybackInput {
        BuybackInput {
            funding_source: "debt".into(),
            ..cash_funded_input()
        }
    }

    #[test]
    fn test_shares_repurchased() {
        let input = cash_funded_input();
        let out = calculate_buyback(&input).unwrap();
        // 500 / 50 = 10 shares
        assert!(approx_eq(out.shares_repurchased, dec!(10), dec!(0.001)));
    }

    #[test]
    fn test_new_shares() {
        let input = cash_funded_input();
        let out = calculate_buyback(&input).unwrap();
        // 100 - 10 = 90
        assert!(approx_eq(out.new_shares, dec!(90), dec!(0.001)));
    }

    #[test]
    fn test_cash_funded_eps_accretion() {
        let input = cash_funded_input();
        let out = calculate_buyback(&input).unwrap();
        // Net income = 5 * 100 = 500, post-buyback EPS = 500/90 = 5.5556
        assert!(approx_eq(out.post_buyback_eps, dec!(5.5556), dec!(0.01)));
        assert!(out.eps_accretive);
        assert!(out.eps_accretion > Decimal::ZERO);
    }

    #[test]
    fn test_debt_funded_eps() {
        let input = debt_funded_input();
        let out = calculate_buyback(&input).unwrap();
        // Interest cost = 500 * 0.05 * (1 - 0.25) = 18.75
        // Adjusted NI = 500 - 18.75 = 481.25
        // Post EPS = 481.25 / 90 = 5.3472
        assert!(approx_eq(out.post_buyback_eps, dec!(5.3472), dec!(0.01)));
    }

    #[test]
    fn test_debt_funded_still_accretive() {
        let input = debt_funded_input();
        let out = calculate_buyback(&input).unwrap();
        // 5.3472 > 5.00
        assert!(out.eps_accretive);
    }

    #[test]
    fn test_high_debt_cost_dilutive() {
        let input = BuybackInput {
            cost_of_debt: dec!(0.50),
            funding_source: "debt".into(),
            ..cash_funded_input()
        };
        let out = calculate_buyback(&input).unwrap();
        // Interest = 500 * 0.50 * 0.75 = 187.5
        // Adjusted NI = 500 - 187.5 = 312.5
        // Post EPS = 312.5 / 90 = 3.4722 < 5.00
        assert!(!out.eps_accretive);
        assert!(out.eps_accretion < Decimal::ZERO);
    }

    #[test]
    fn test_buyback_yield() {
        let input = cash_funded_input();
        let out = calculate_buyback(&input).unwrap();
        // market cap = 50 * 100 = 5000
        // yield = 500 / 5000 = 0.10
        assert!(approx_eq(out.buyback_yield, dec!(0.10), dec!(0.001)));
    }

    #[test]
    fn test_equivalent_dividend() {
        let input = cash_funded_input();
        let out = calculate_buyback(&input).unwrap();
        // 500 / 100 = 5.00 per share
        assert!(approx_eq(out.equivalent_dividend, dec!(5.00), dec!(0.001)));
    }

    #[test]
    fn test_dividend_tax_cost() {
        let input = cash_funded_input();
        let out = calculate_buyback(&input).unwrap();
        // 5.00 * 0.20 * 100 = 100
        assert!(approx_eq(out.dividend_tax_cost, dec!(100), dec!(0.01)));
    }

    #[test]
    fn test_buyback_tax_cost() {
        let input = cash_funded_input();
        let out = calculate_buyback(&input).unwrap();
        // 500 * 0.50 * 0.15 = 37.50
        assert!(approx_eq(out.buyback_tax_cost, dec!(37.50), dec!(0.01)));
    }

    #[test]
    fn test_tax_efficiency_positive() {
        let input = cash_funded_input();
        let out = calculate_buyback(&input).unwrap();
        // (100 - 37.50) / 100 * 100 = 62.5%
        assert!(approx_eq(out.tax_efficiency, dec!(62.5), dec!(0.1)));
    }

    #[test]
    fn test_mixed_funding() {
        let input = BuybackInput {
            funding_source: "mixed".into(),
            ..cash_funded_input()
        };
        let out = calculate_buyback(&input).unwrap();
        // Same as cash
        assert!(approx_eq(out.post_buyback_eps, dec!(5.5556), dec!(0.01)));
    }

    #[test]
    fn test_reject_zero_price() {
        let input = BuybackInput {
            current_price: Decimal::ZERO,
            ..cash_funded_input()
        };
        assert!(calculate_buyback(&input).is_err());
    }

    #[test]
    fn test_reject_negative_shares() {
        let input = BuybackInput {
            current_shares: dec!(-10),
            ..cash_funded_input()
        };
        assert!(calculate_buyback(&input).is_err());
    }

    #[test]
    fn test_reject_zero_buyback() {
        let input = BuybackInput {
            buyback_amount: Decimal::ZERO,
            ..cash_funded_input()
        };
        assert!(calculate_buyback(&input).is_err());
    }

    #[test]
    fn test_reject_invalid_funding_source() {
        let input = BuybackInput {
            funding_source: "equity".into(),
            ..cash_funded_input()
        };
        assert!(calculate_buyback(&input).is_err());
    }

    #[test]
    fn test_reject_buyback_exceeds_shares() {
        let input = BuybackInput {
            buyback_amount: dec!(6000),
            // 6000/50 = 120 > 100 shares
            ..cash_funded_input()
        };
        assert!(calculate_buyback(&input).is_err());
    }

    #[test]
    fn test_reject_tax_rate_out_of_range() {
        let input = BuybackInput {
            tax_rate: dec!(1.5),
            ..cash_funded_input()
        };
        assert!(calculate_buyback(&input).is_err());
    }

    #[test]
    fn test_reject_negative_cost_of_debt() {
        let input = BuybackInput {
            cost_of_debt: dec!(-0.05),
            ..cash_funded_input()
        };
        assert!(calculate_buyback(&input).is_err());
    }

    #[test]
    fn test_large_buyback() {
        let input = BuybackInput {
            buyback_amount: dec!(4900),
            ..cash_funded_input()
        };
        let out = calculate_buyback(&input).unwrap();
        // 4900/50 = 98 repurchased, 2 remaining
        assert!(approx_eq(out.new_shares, dec!(2), dec!(0.001)));
        // EPS should be very high
        assert!(out.post_buyback_eps > dec!(100));
    }

    #[test]
    fn test_pre_buyback_eps_unchanged() {
        let input = cash_funded_input();
        let out = calculate_buyback(&input).unwrap();
        assert_eq!(out.pre_buyback_eps, dec!(5.00));
    }

    #[test]
    fn test_serialization_roundtrip() {
        let input = cash_funded_input();
        let out = calculate_buyback(&input).unwrap();
        let json = serde_json::to_string(&out).unwrap();
        let _: BuybackOutput = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_zero_tax_rates_efficiency() {
        let input = BuybackInput {
            dividend_tax_rate: Decimal::ZERO,
            capital_gains_tax_rate: Decimal::ZERO,
            ..cash_funded_input()
        };
        let out = calculate_buyback(&input).unwrap();
        // With zero tax rates, both costs are 0, efficiency = 0
        assert_eq!(out.dividend_tax_cost, Decimal::ZERO);
        assert_eq!(out.buyback_tax_cost, Decimal::ZERO);
        assert_eq!(out.tax_efficiency, Decimal::ZERO);
    }
}
