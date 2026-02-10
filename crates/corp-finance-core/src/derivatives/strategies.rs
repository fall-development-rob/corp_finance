use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::*;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StrategyType {
    LongCall,
    LongPut,
    CoveredCall,
    ProtectivePut,
    BullCallSpread,
    BearPutSpread,
    LongStraddle,
    LongStrangle,
    IronCondor,
    ButterflySpread,
    Collar,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LegType {
    Call,
    Put,
    Stock,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LegPosition {
    Long,
    Short,
}

// ---------------------------------------------------------------------------
// Input types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyLeg {
    pub leg_type: LegType,
    pub position: LegPosition,
    pub strike: Option<Money>,
    pub premium: Money,
    pub quantity: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyInput {
    pub strategy_type: StrategyType,
    pub underlying_price: Money,
    pub legs: Vec<StrategyLeg>,
    pub price_range: Option<(Money, Money)>,
    pub price_steps: Option<u32>,
}

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayoffPoint {
    pub underlying_price: Money,
    pub payoff: Money,
    pub per_leg: Vec<Money>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyCharacteristics {
    pub direction: String,
    pub profit_type: String,
    pub loss_type: String,
    pub requires_margin: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyOutput {
    pub strategy_name: String,
    pub net_premium: Money,
    pub max_profit: Option<Money>,
    pub max_loss: Option<Money>,
    pub breakeven_points: Vec<Money>,
    pub payoff_table: Vec<PayoffPoint>,
    pub risk_reward_ratio: Option<Decimal>,
    pub profit_probability_estimate: Option<Rate>,
    pub strategy_characteristics: StrategyCharacteristics,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Compute a single leg's payoff at expiry given underlying price S.
fn leg_payoff(leg: &StrategyLeg, s: Decimal) -> Decimal {
    let raw = match leg.leg_type {
        LegType::Call => {
            let k = leg.strike.unwrap_or(Decimal::ZERO);
            let intrinsic = if s > k { s - k } else { Decimal::ZERO };
            match leg.position {
                LegPosition::Long => intrinsic - leg.premium,
                LegPosition::Short => leg.premium - intrinsic,
            }
        }
        LegType::Put => {
            let k = leg.strike.unwrap_or(Decimal::ZERO);
            let intrinsic = if k > s { k - s } else { Decimal::ZERO };
            match leg.position {
                LegPosition::Long => intrinsic - leg.premium,
                LegPosition::Short => leg.premium - intrinsic,
            }
        }
        LegType::Stock => match leg.position {
            LegPosition::Long => s - leg.premium,
            LegPosition::Short => leg.premium - s,
        },
    };
    raw * leg.quantity
}

/// Compute total strategy payoff at expiry given underlying price S.
fn strategy_payoff(legs: &[StrategyLeg], s: Decimal) -> (Decimal, Vec<Decimal>) {
    let per_leg: Vec<Decimal> = legs.iter().map(|leg| leg_payoff(leg, s)).collect();
    let total: Decimal = per_leg.iter().copied().sum();
    (total, per_leg)
}

/// Build the payoff table across the price range.
fn build_payoff_table(
    legs: &[StrategyLeg],
    low: Decimal,
    high: Decimal,
    steps: u32,
) -> Vec<PayoffPoint> {
    let mut table = Vec::with_capacity(steps as usize + 1);
    let range = high - low;
    let step_size = if steps > 0 {
        range / Decimal::from(steps)
    } else {
        range
    };

    for i in 0..=steps {
        let price = low + step_size * Decimal::from(i);
        let (payoff, per_leg) = strategy_payoff(legs, price);
        table.push(PayoffPoint {
            underlying_price: price,
            payoff,
            per_leg,
        });
    }
    table
}

/// Find breakeven points by linear interpolation between sign changes in the
/// payoff table. We also check for exact zero-crossings.
fn find_breakevens(table: &[PayoffPoint]) -> Vec<Money> {
    let mut breakevens = Vec::new();

    for i in 0..table.len() {
        // Exact zero
        if table[i].payoff == Decimal::ZERO {
            let price = table[i].underlying_price;
            if !breakevens.contains(&price) {
                breakevens.push(price);
            }
            continue;
        }

        if i == 0 {
            continue;
        }

        let prev = &table[i - 1];
        let curr = &table[i];

        // Sign change: one positive, one negative (skip if either is zero --
        // handled above).
        let prev_sign = prev.payoff > Decimal::ZERO;
        let curr_sign = curr.payoff > Decimal::ZERO;
        if prev_sign != curr_sign && prev.payoff != Decimal::ZERO && curr.payoff != Decimal::ZERO {
            // Linear interpolation: find S where payoff = 0
            // payoff(prev) + (payoff(curr) - payoff(prev)) * t = 0
            // t = -payoff(prev) / (payoff(curr) - payoff(prev))
            let denom = curr.payoff - prev.payoff;
            if denom != Decimal::ZERO {
                let t = -prev.payoff / denom;
                let be =
                    prev.underlying_price + t * (curr.underlying_price - prev.underlying_price);
                if !breakevens.contains(&be) {
                    breakevens.push(be);
                }
            }
        }
    }

    breakevens.sort();
    breakevens
}

/// Determine max profit and max loss from the payoff table.
/// Returns (max_profit, max_loss) where None means unlimited.
///
/// Strategy: find the maximum and minimum payoffs in the table. If the
/// extreme values are at the endpoints AND the payoff is still trending
/// in that direction (monotonically increasing/decreasing at the edge),
/// mark as unlimited.
fn find_max_profit_loss(table: &[PayoffPoint]) -> (Option<Money>, Option<Money>) {
    if table.is_empty() {
        return (None, None);
    }

    let max_payoff = table.iter().map(|p| p.payoff).max().unwrap();
    let min_payoff = table.iter().map(|p| p.payoff).min().unwrap();

    let n = table.len();

    // Check if profit is unlimited: max is at the last point AND is still
    // increasing (or at first point and still decreasing to the left).
    let profit_unlimited = if n >= 2 {
        let at_right_end =
            table[n - 1].payoff == max_payoff && table[n - 1].payoff > table[n - 2].payoff;
        let at_left_end = table[0].payoff == max_payoff && table[0].payoff > table[1].payoff;
        at_right_end || at_left_end
    } else {
        false
    };

    // Check if loss is unlimited: min is at an endpoint AND still decreasing.
    let loss_unlimited = if n >= 2 {
        let at_right_end =
            table[n - 1].payoff == min_payoff && table[n - 1].payoff < table[n - 2].payoff;
        let at_left_end = table[0].payoff == min_payoff && table[0].payoff < table[1].payoff;
        at_right_end || at_left_end
    } else {
        false
    };

    let max_profit = if profit_unlimited {
        None
    } else {
        Some(max_payoff)
    };

    // max_loss is the absolute value of the minimum payoff (losses are negative payoffs)
    let max_loss = if loss_unlimited {
        None
    } else {
        Some(min_payoff.abs())
    };

    (max_profit, max_loss)
}

/// Compute net premium: sum of all premiums paid minus premiums received.
/// Long positions pay premium (cost), short positions receive premium (credit).
fn compute_net_premium(legs: &[StrategyLeg]) -> Money {
    let mut net = Decimal::ZERO;
    for leg in legs {
        let cost = leg.premium * leg.quantity;
        match leg.position {
            LegPosition::Long => {
                // For stock legs, premium is purchase price -- not an option premium cost
                // in the net premium sense. Only count option legs.
                match leg.leg_type {
                    LegType::Call | LegType::Put => net += cost,
                    LegType::Stock => {} // stock cost not counted as option premium
                }
            }
            LegPosition::Short => {
                match leg.leg_type {
                    LegType::Call | LegType::Put => net -= cost,
                    LegType::Stock => {} // stock proceeds not counted as option premium
                }
            }
        }
    }
    net
}

/// Determine strategy characteristics from the strategy type.
fn determine_characteristics(strategy_type: &StrategyType) -> StrategyCharacteristics {
    match strategy_type {
        StrategyType::LongCall => StrategyCharacteristics {
            direction: "bullish".to_string(),
            profit_type: "unlimited".to_string(),
            loss_type: "limited".to_string(),
            requires_margin: false,
        },
        StrategyType::LongPut => StrategyCharacteristics {
            direction: "bearish".to_string(),
            profit_type: "unlimited".to_string(),
            loss_type: "limited".to_string(),
            requires_margin: false,
        },
        StrategyType::CoveredCall => StrategyCharacteristics {
            direction: "bullish".to_string(),
            profit_type: "limited".to_string(),
            loss_type: "limited".to_string(),
            requires_margin: false,
        },
        StrategyType::ProtectivePut => StrategyCharacteristics {
            direction: "bullish".to_string(),
            profit_type: "unlimited".to_string(),
            loss_type: "limited".to_string(),
            requires_margin: false,
        },
        StrategyType::BullCallSpread => StrategyCharacteristics {
            direction: "bullish".to_string(),
            profit_type: "limited".to_string(),
            loss_type: "limited".to_string(),
            requires_margin: false,
        },
        StrategyType::BearPutSpread => StrategyCharacteristics {
            direction: "bearish".to_string(),
            profit_type: "limited".to_string(),
            loss_type: "limited".to_string(),
            requires_margin: false,
        },
        StrategyType::LongStraddle => StrategyCharacteristics {
            direction: "volatile".to_string(),
            profit_type: "unlimited".to_string(),
            loss_type: "limited".to_string(),
            requires_margin: false,
        },
        StrategyType::LongStrangle => StrategyCharacteristics {
            direction: "volatile".to_string(),
            profit_type: "unlimited".to_string(),
            loss_type: "limited".to_string(),
            requires_margin: false,
        },
        StrategyType::IronCondor => StrategyCharacteristics {
            direction: "neutral".to_string(),
            profit_type: "limited".to_string(),
            loss_type: "limited".to_string(),
            requires_margin: true,
        },
        StrategyType::ButterflySpread => StrategyCharacteristics {
            direction: "neutral".to_string(),
            profit_type: "limited".to_string(),
            loss_type: "limited".to_string(),
            requires_margin: false,
        },
        StrategyType::Collar => StrategyCharacteristics {
            direction: "bullish".to_string(),
            profit_type: "limited".to_string(),
            loss_type: "limited".to_string(),
            requires_margin: false,
        },
        StrategyType::Custom => StrategyCharacteristics {
            direction: "neutral".to_string(),
            profit_type: "unknown".to_string(),
            loss_type: "unknown".to_string(),
            requires_margin: false,
        },
    }
}

/// Human-readable name for a strategy type.
fn strategy_name(strategy_type: &StrategyType) -> String {
    match strategy_type {
        StrategyType::LongCall => "Long Call".to_string(),
        StrategyType::LongPut => "Long Put".to_string(),
        StrategyType::CoveredCall => "Covered Call".to_string(),
        StrategyType::ProtectivePut => "Protective Put".to_string(),
        StrategyType::BullCallSpread => "Bull Call Spread".to_string(),
        StrategyType::BearPutSpread => "Bear Put Spread".to_string(),
        StrategyType::LongStraddle => "Long Straddle".to_string(),
        StrategyType::LongStrangle => "Long Strangle".to_string(),
        StrategyType::IronCondor => "Iron Condor".to_string(),
        StrategyType::ButterflySpread => "Butterfly Spread".to_string(),
        StrategyType::Collar => "Collar".to_string(),
        StrategyType::Custom => "Custom Strategy".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_input(input: &StrategyInput) -> CorpFinanceResult<()> {
    if input.legs.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "legs".into(),
            reason: "Strategy must have at least one leg".into(),
        });
    }

    if input.underlying_price <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "underlying_price".into(),
            reason: "Underlying price must be positive".into(),
        });
    }

    for (i, leg) in input.legs.iter().enumerate() {
        if leg.quantity <= Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("legs[{}].quantity", i),
                reason: "Quantity must be positive".into(),
            });
        }

        match leg.leg_type {
            LegType::Call | LegType::Put => match leg.strike {
                Some(k) if k <= Decimal::ZERO => {
                    return Err(CorpFinanceError::InvalidInput {
                        field: format!("legs[{}].strike", i),
                        reason: "Strike price must be positive for options".into(),
                    });
                }
                None => {
                    return Err(CorpFinanceError::InvalidInput {
                        field: format!("legs[{}].strike", i),
                        reason: "Strike price is required for Call/Put legs".into(),
                    });
                }
                _ => {}
            },
            LegType::Stock => {}
        }
    }

    // Validate leg counts for known strategy types
    validate_strategy_legs(input)?;

    Ok(())
}

fn validate_strategy_legs(input: &StrategyInput) -> CorpFinanceResult<()> {
    let call_count = input
        .legs
        .iter()
        .filter(|l| matches!(l.leg_type, LegType::Call))
        .count();
    let put_count = input
        .legs
        .iter()
        .filter(|l| matches!(l.leg_type, LegType::Put))
        .count();
    let stock_count = input
        .legs
        .iter()
        .filter(|l| matches!(l.leg_type, LegType::Stock))
        .count();

    match input.strategy_type {
        StrategyType::LongCall => {
            if call_count != 1 || put_count != 0 || stock_count != 0 {
                return Err(CorpFinanceError::InvalidInput {
                    field: "legs".into(),
                    reason: "Long Call requires exactly 1 call leg".into(),
                });
            }
        }
        StrategyType::LongPut => {
            if put_count != 1 || call_count != 0 || stock_count != 0 {
                return Err(CorpFinanceError::InvalidInput {
                    field: "legs".into(),
                    reason: "Long Put requires exactly 1 put leg".into(),
                });
            }
        }
        StrategyType::CoveredCall => {
            if stock_count != 1 || call_count != 1 {
                return Err(CorpFinanceError::InvalidInput {
                    field: "legs".into(),
                    reason: "Covered Call requires 1 stock leg and 1 call leg".into(),
                });
            }
        }
        StrategyType::ProtectivePut => {
            if stock_count != 1 || put_count != 1 {
                return Err(CorpFinanceError::InvalidInput {
                    field: "legs".into(),
                    reason: "Protective Put requires 1 stock leg and 1 put leg".into(),
                });
            }
        }
        StrategyType::BullCallSpread => {
            if call_count != 2 {
                return Err(CorpFinanceError::InvalidInput {
                    field: "legs".into(),
                    reason: "Bull Call Spread requires exactly 2 call legs".into(),
                });
            }
        }
        StrategyType::BearPutSpread => {
            if put_count != 2 {
                return Err(CorpFinanceError::InvalidInput {
                    field: "legs".into(),
                    reason: "Bear Put Spread requires exactly 2 put legs".into(),
                });
            }
        }
        StrategyType::LongStraddle => {
            if call_count != 1 || put_count != 1 {
                return Err(CorpFinanceError::InvalidInput {
                    field: "legs".into(),
                    reason: "Long Straddle requires 1 call and 1 put leg".into(),
                });
            }
        }
        StrategyType::LongStrangle => {
            if call_count != 1 || put_count != 1 {
                return Err(CorpFinanceError::InvalidInput {
                    field: "legs".into(),
                    reason: "Long Strangle requires 1 call and 1 put leg".into(),
                });
            }
        }
        StrategyType::IronCondor => {
            if call_count != 2 || put_count != 2 {
                return Err(CorpFinanceError::InvalidInput {
                    field: "legs".into(),
                    reason: "Iron Condor requires 2 call legs and 2 put legs".into(),
                });
            }
        }
        StrategyType::ButterflySpread => {
            if call_count != 3 && put_count != 3 {
                return Err(CorpFinanceError::InvalidInput {
                    field: "legs".into(),
                    reason: "Butterfly Spread requires 3 call or 3 put legs".into(),
                });
            }
        }
        StrategyType::Collar => {
            if stock_count != 1 || put_count != 1 || call_count != 1 {
                return Err(CorpFinanceError::InvalidInput {
                    field: "legs".into(),
                    reason: "Collar requires 1 stock, 1 put, and 1 call leg".into(),
                });
            }
        }
        StrategyType::Custom => {}
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Main function
// ---------------------------------------------------------------------------

pub fn analyze_strategy(
    input: &StrategyInput,
) -> CorpFinanceResult<ComputationOutput<StrategyOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    validate_input(input)?;

    // Determine price range (default: spot +/- 30%)
    let thirty_pct = input.underlying_price * Decimal::new(30, 2); // 0.30
    let (low, high) = input.price_range.unwrap_or((
        input.underlying_price - thirty_pct,
        input.underlying_price + thirty_pct,
    ));

    // Clamp low to zero (prices cannot be negative)
    let low = if low < Decimal::ZERO {
        Decimal::ZERO
    } else {
        low
    };

    if low >= high {
        return Err(CorpFinanceError::InvalidInput {
            field: "price_range".into(),
            reason: "Low price must be less than high price".into(),
        });
    }

    let steps = input.price_steps.unwrap_or(21);
    if steps == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "price_steps".into(),
            reason: "Must have at least 1 price step".into(),
        });
    }

    // Build payoff table
    let payoff_table = build_payoff_table(&input.legs, low, high, steps);

    // Find breakevens
    let breakeven_points = find_breakevens(&payoff_table);

    // Max profit/loss
    let (max_profit, max_loss) = find_max_profit_loss(&payoff_table);

    // Risk-reward ratio
    let risk_reward_ratio = match (max_profit, max_loss) {
        (Some(mp), Some(ml)) if ml > Decimal::ZERO => Some(mp / ml),
        _ => None,
    };

    // Net premium
    let net_premium = compute_net_premium(&input.legs);

    // Profit probability estimate: fraction of payoff table points that are profitable
    let profitable_count = payoff_table
        .iter()
        .filter(|p| p.payoff > Decimal::ZERO)
        .count();
    let total_points = payoff_table.len();
    let profit_probability_estimate = if total_points > 0 {
        Some(Decimal::from(profitable_count as u32) / Decimal::from(total_points as u32))
    } else {
        None
    };

    // Strategy characteristics
    let strategy_characteristics = determine_characteristics(&input.strategy_type);

    // Warn if max profit/loss is unlimited
    if max_profit.is_none() {
        warnings.push("Profit potential is theoretically unlimited".to_string());
    }
    if max_loss.is_none() {
        warnings.push("Loss potential is theoretically unlimited".to_string());
    }

    let output = StrategyOutput {
        strategy_name: strategy_name(&input.strategy_type),
        net_premium,
        max_profit,
        max_loss,
        breakeven_points,
        payoff_table,
        risk_reward_ratio,
        profit_probability_estimate,
        strategy_characteristics,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Option Strategy Analysis — Expiry Payoff Profile",
        &serde_json::json!({
            "strategy_type": strategy_name(&input.strategy_type),
            "underlying_price": input.underlying_price.to_string(),
            "num_legs": input.legs.len(),
            "price_range": format!("{} - {}", low, high),
            "price_steps": steps,
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

    // -----------------------------------------------------------------------
    // 1. Long call payoff
    // -----------------------------------------------------------------------
    #[test]
    fn test_long_call_payoff() {
        let input = StrategyInput {
            strategy_type: StrategyType::LongCall,
            underlying_price: dec!(100),
            legs: vec![StrategyLeg {
                leg_type: LegType::Call,
                position: LegPosition::Long,
                strike: Some(dec!(100)),
                premium: dec!(5),
                quantity: dec!(1),
            }],
            price_range: Some((dec!(80), dec!(120))),
            price_steps: Some(40),
        };

        let result = analyze_strategy(&input).unwrap();
        let table = &result.result.payoff_table;

        // At S=80 (well below strike): payoff = max(0, 80-100) - 5 = -5
        assert_eq!(table[0].payoff, dec!(-5));

        // At S=120 (well above strike): payoff = max(0, 120-100) - 5 = 15
        assert_eq!(table[40].payoff, dec!(15));

        // At S=100 (at strike): payoff = max(0, 0) - 5 = -5
        let at_strike = table
            .iter()
            .find(|p| p.underlying_price == dec!(100))
            .unwrap();
        assert_eq!(at_strike.payoff, dec!(-5));

        assert_eq!(result.result.strategy_characteristics.direction, "bullish");
    }

    // -----------------------------------------------------------------------
    // 2. Long put payoff
    // -----------------------------------------------------------------------
    #[test]
    fn test_long_put_payoff() {
        let input = StrategyInput {
            strategy_type: StrategyType::LongPut,
            underlying_price: dec!(100),
            legs: vec![StrategyLeg {
                leg_type: LegType::Put,
                position: LegPosition::Long,
                strike: Some(dec!(100)),
                premium: dec!(5),
                quantity: dec!(1),
            }],
            price_range: Some((dec!(80), dec!(120))),
            price_steps: Some(40),
        };

        let result = analyze_strategy(&input).unwrap();
        let table = &result.result.payoff_table;

        // At S=80: payoff = max(0, 100-80) - 5 = 15
        assert_eq!(table[0].payoff, dec!(15));

        // At S=120: payoff = max(0, 100-120) - 5 = -5
        assert_eq!(table[40].payoff, dec!(-5));

        assert_eq!(result.result.strategy_characteristics.direction, "bearish");
    }

    // -----------------------------------------------------------------------
    // 3. Covered call (long stock + short call)
    // -----------------------------------------------------------------------
    #[test]
    fn test_covered_call() {
        // Buy stock at 100, sell call at strike 105 for premium 3
        let input = StrategyInput {
            strategy_type: StrategyType::CoveredCall,
            underlying_price: dec!(100),
            legs: vec![
                StrategyLeg {
                    leg_type: LegType::Stock,
                    position: LegPosition::Long,
                    strike: None,
                    premium: dec!(100), // purchase price
                    quantity: dec!(1),
                },
                StrategyLeg {
                    leg_type: LegType::Call,
                    position: LegPosition::Short,
                    strike: Some(dec!(105)),
                    premium: dec!(3),
                    quantity: dec!(1),
                },
            ],
            price_range: Some((dec!(80), dec!(120))),
            price_steps: Some(40),
        };

        let result = analyze_strategy(&input).unwrap();
        let table = &result.result.payoff_table;

        // At S=120:
        // Stock: 120-100 = 20
        // Short call: 3 - max(0, 120-105) = 3-15 = -12
        // Total: 20-12 = 8 (max profit = stock gain to strike + premium = 5+3 = 8)
        assert_eq!(table[40].payoff, dec!(8));

        // At S=80:
        // Stock: 80-100 = -20
        // Short call: 3 - max(0, 80-105) = 3
        // Total: -20+3 = -17
        assert_eq!(table[0].payoff, dec!(-17));

        assert_eq!(
            result.result.strategy_characteristics.profit_type,
            "limited"
        );
    }

    // -----------------------------------------------------------------------
    // 4. Protective put (long stock + long put)
    // -----------------------------------------------------------------------
    #[test]
    fn test_protective_put() {
        // Buy stock at 100, buy put at strike 95 for premium 2
        let input = StrategyInput {
            strategy_type: StrategyType::ProtectivePut,
            underlying_price: dec!(100),
            legs: vec![
                StrategyLeg {
                    leg_type: LegType::Stock,
                    position: LegPosition::Long,
                    strike: None,
                    premium: dec!(100),
                    quantity: dec!(1),
                },
                StrategyLeg {
                    leg_type: LegType::Put,
                    position: LegPosition::Long,
                    strike: Some(dec!(95)),
                    premium: dec!(2),
                    quantity: dec!(1),
                },
            ],
            price_range: Some((dec!(80), dec!(120))),
            price_steps: Some(40),
        };

        let result = analyze_strategy(&input).unwrap();
        let table = &result.result.payoff_table;

        // At S=80:
        // Stock: 80-100 = -20
        // Long put: max(0, 95-80) - 2 = 15-2 = 13
        // Total: -20+13 = -7 (max loss is capped)
        assert_eq!(table[0].payoff, dec!(-7));

        // At S=120:
        // Stock: 120-100 = 20
        // Long put: max(0, 95-120) - 2 = -2
        // Total: 20-2 = 18
        assert_eq!(table[40].payoff, dec!(18));

        assert_eq!(result.result.strategy_characteristics.direction, "bullish");
    }

    // -----------------------------------------------------------------------
    // 5. Bull call spread (long lower call + short higher call)
    // -----------------------------------------------------------------------
    #[test]
    fn test_bull_call_spread() {
        // Buy call at K=95 for 7, sell call at K=105 for 2
        let input = StrategyInput {
            strategy_type: StrategyType::BullCallSpread,
            underlying_price: dec!(100),
            legs: vec![
                StrategyLeg {
                    leg_type: LegType::Call,
                    position: LegPosition::Long,
                    strike: Some(dec!(95)),
                    premium: dec!(7),
                    quantity: dec!(1),
                },
                StrategyLeg {
                    leg_type: LegType::Call,
                    position: LegPosition::Short,
                    strike: Some(dec!(105)),
                    premium: dec!(2),
                    quantity: dec!(1),
                },
            ],
            price_range: Some((dec!(80), dec!(120))),
            price_steps: Some(40),
        };

        let result = analyze_strategy(&input).unwrap();
        let table = &result.result.payoff_table;

        // At S=80 (below both strikes):
        // Long call: max(0, 80-95) - 7 = -7
        // Short call: 2 - max(0, 80-105) = 2
        // Total: -7+2 = -5 (max loss = net debit)
        assert_eq!(table[0].payoff, dec!(-5));

        // At S=120 (above both strikes):
        // Long call: max(0, 120-95) - 7 = 25-7 = 18
        // Short call: 2 - max(0, 120-105) = 2-15 = -13
        // Total: 18-13 = 5 (max profit = spread width - net debit = 10-5 = 5)
        assert_eq!(table[40].payoff, dec!(5));

        assert_eq!(
            result.result.strategy_characteristics.profit_type,
            "limited"
        );
        assert_eq!(result.result.strategy_characteristics.loss_type, "limited");
    }

    // -----------------------------------------------------------------------
    // 6. Bear put spread (long higher put + short lower put)
    // -----------------------------------------------------------------------
    #[test]
    fn test_bear_put_spread() {
        // Buy put at K=105 for 7, sell put at K=95 for 2
        let input = StrategyInput {
            strategy_type: StrategyType::BearPutSpread,
            underlying_price: dec!(100),
            legs: vec![
                StrategyLeg {
                    leg_type: LegType::Put,
                    position: LegPosition::Long,
                    strike: Some(dec!(105)),
                    premium: dec!(7),
                    quantity: dec!(1),
                },
                StrategyLeg {
                    leg_type: LegType::Put,
                    position: LegPosition::Short,
                    strike: Some(dec!(95)),
                    premium: dec!(2),
                    quantity: dec!(1),
                },
            ],
            price_range: Some((dec!(80), dec!(120))),
            price_steps: Some(40),
        };

        let result = analyze_strategy(&input).unwrap();
        let table = &result.result.payoff_table;

        // At S=80 (below both strikes):
        // Long put: max(0, 105-80) - 7 = 25-7 = 18
        // Short put: 2 - max(0, 95-80) = 2-15 = -13
        // Total: 18-13 = 5 (max profit = spread width - net debit = 10-5 = 5)
        assert_eq!(table[0].payoff, dec!(5));

        // At S=120 (above both strikes):
        // Long put: max(0, 105-120) - 7 = -7
        // Short put: 2 - max(0, 95-120) = 2
        // Total: -7+2 = -5 (max loss = net debit)
        assert_eq!(table[40].payoff, dec!(-5));

        assert_eq!(result.result.strategy_characteristics.direction, "bearish");
    }

    // -----------------------------------------------------------------------
    // 7. Long straddle (long call + long put, same strike)
    // -----------------------------------------------------------------------
    #[test]
    fn test_long_straddle() {
        // Buy call and put at K=100, both for 5 each
        let input = StrategyInput {
            strategy_type: StrategyType::LongStraddle,
            underlying_price: dec!(100),
            legs: vec![
                StrategyLeg {
                    leg_type: LegType::Call,
                    position: LegPosition::Long,
                    strike: Some(dec!(100)),
                    premium: dec!(5),
                    quantity: dec!(1),
                },
                StrategyLeg {
                    leg_type: LegType::Put,
                    position: LegPosition::Long,
                    strike: Some(dec!(100)),
                    premium: dec!(5),
                    quantity: dec!(1),
                },
            ],
            price_range: Some((dec!(80), dec!(120))),
            price_steps: Some(40),
        };

        let result = analyze_strategy(&input).unwrap();
        let table = &result.result.payoff_table;

        // At S=100 (at strike): both options expire worthless
        // Call: 0-5 = -5, Put: 0-5 = -5, Total: -10 (max loss)
        let at_strike = table
            .iter()
            .find(|p| p.underlying_price == dec!(100))
            .unwrap();
        assert_eq!(at_strike.payoff, dec!(-10));

        // At S=80: Call: -5, Put: 20-5=15, Total: 10
        assert_eq!(table[0].payoff, dec!(10));

        // At S=120: Call: 20-5=15, Put: -5, Total: 10
        assert_eq!(table[40].payoff, dec!(10));

        assert_eq!(result.result.strategy_characteristics.direction, "volatile");
    }

    // -----------------------------------------------------------------------
    // 8. Long strangle (long OTM call + long OTM put)
    // -----------------------------------------------------------------------
    #[test]
    fn test_long_strangle() {
        // Buy call at K=110 for 2, buy put at K=90 for 2
        let input = StrategyInput {
            strategy_type: StrategyType::LongStrangle,
            underlying_price: dec!(100),
            legs: vec![
                StrategyLeg {
                    leg_type: LegType::Call,
                    position: LegPosition::Long,
                    strike: Some(dec!(110)),
                    premium: dec!(2),
                    quantity: dec!(1),
                },
                StrategyLeg {
                    leg_type: LegType::Put,
                    position: LegPosition::Long,
                    strike: Some(dec!(90)),
                    premium: dec!(2),
                    quantity: dec!(1),
                },
            ],
            price_range: Some((dec!(70), dec!(130))),
            price_steps: Some(60),
        };

        let result = analyze_strategy(&input).unwrap();
        let table = &result.result.payoff_table;

        // At S=100: both OTM, max loss = total premium = -4
        let at_100 = table
            .iter()
            .find(|p| p.underlying_price == dec!(100))
            .unwrap();
        assert_eq!(at_100.payoff, dec!(-4));

        // At S=70:
        // Call: max(0, 70-110) - 2 = -2
        // Put: max(0, 90-70) - 2 = 20-2 = 18
        // Total: 16
        assert_eq!(table[0].payoff, dec!(16));

        // At S=130:
        // Call: max(0, 130-110) - 2 = 20-2 = 18
        // Put: max(0, 90-130) - 2 = -2
        // Total: 16
        assert_eq!(table[60].payoff, dec!(16));
    }

    // -----------------------------------------------------------------------
    // 9. Iron condor (short strangle + long wider strangle)
    // -----------------------------------------------------------------------
    #[test]
    fn test_iron_condor() {
        // Sell put K=90 for 3, sell call K=110 for 3
        // Buy put K=85 for 1, buy call K=115 for 1
        // Net credit = (3+3) - (1+1) = 4
        let input = StrategyInput {
            strategy_type: StrategyType::IronCondor,
            underlying_price: dec!(100),
            legs: vec![
                StrategyLeg {
                    leg_type: LegType::Put,
                    position: LegPosition::Short,
                    strike: Some(dec!(90)),
                    premium: dec!(3),
                    quantity: dec!(1),
                },
                StrategyLeg {
                    leg_type: LegType::Call,
                    position: LegPosition::Short,
                    strike: Some(dec!(110)),
                    premium: dec!(3),
                    quantity: dec!(1),
                },
                StrategyLeg {
                    leg_type: LegType::Put,
                    position: LegPosition::Long,
                    strike: Some(dec!(85)),
                    premium: dec!(1),
                    quantity: dec!(1),
                },
                StrategyLeg {
                    leg_type: LegType::Call,
                    position: LegPosition::Long,
                    strike: Some(dec!(115)),
                    premium: dec!(1),
                    quantity: dec!(1),
                },
            ],
            price_range: Some((dec!(75), dec!(125))),
            price_steps: Some(50),
        };

        let result = analyze_strategy(&input).unwrap();
        let table = &result.result.payoff_table;

        // At S=100 (between short strikes): max profit = net credit = 4
        let at_100 = table
            .iter()
            .find(|p| p.underlying_price == dec!(100))
            .unwrap();
        assert_eq!(at_100.payoff, dec!(4));

        // At S=75 (below all strikes):
        // Short put 90: 3 - (90-75) = 3-15 = -12
        // Short call 110: 3
        // Long put 85: (85-75) - 1 = 10-1 = 9
        // Long call 115: -1
        // Total: -12+3+9-1 = -1
        assert_eq!(table[0].payoff, dec!(-1));

        assert_eq!(result.result.strategy_characteristics.direction, "neutral");
    }

    // -----------------------------------------------------------------------
    // 10. Butterfly spread (call butterfly)
    // -----------------------------------------------------------------------
    #[test]
    fn test_butterfly_spread() {
        // Buy 1 call at K=95 for 8, sell 2 calls at K=100 for 5 each,
        // buy 1 call at K=105 for 2
        // Net debit = 8 - 2*5 + 2 = 0 (cost neutral in this example)
        let input = StrategyInput {
            strategy_type: StrategyType::ButterflySpread,
            underlying_price: dec!(100),
            legs: vec![
                StrategyLeg {
                    leg_type: LegType::Call,
                    position: LegPosition::Long,
                    strike: Some(dec!(95)),
                    premium: dec!(8),
                    quantity: dec!(1),
                },
                StrategyLeg {
                    leg_type: LegType::Call,
                    position: LegPosition::Short,
                    strike: Some(dec!(100)),
                    premium: dec!(5),
                    quantity: dec!(2),
                },
                StrategyLeg {
                    leg_type: LegType::Call,
                    position: LegPosition::Long,
                    strike: Some(dec!(105)),
                    premium: dec!(2),
                    quantity: dec!(1),
                },
            ],
            price_range: Some((dec!(85), dec!(115))),
            price_steps: Some(30),
        };

        let result = analyze_strategy(&input).unwrap();
        let table = &result.result.payoff_table;

        // At S=100 (center strike):
        // Long 95 call: (100-95)-8 = -3
        // Short 100 call (x2): 2*(5-0) = 10
        // Long 105 call: 0-2 = -2
        // Total: -3+10-2 = 5 (max profit)
        let at_100 = table
            .iter()
            .find(|p| p.underlying_price == dec!(100))
            .unwrap();
        assert_eq!(at_100.payoff, dec!(5));

        // At S=85 (below all strikes): all calls OTM
        // Long 95: -8, Short 100 (x2): 2*5=10, Long 105: -2
        // Total: -8+10-2 = 0
        assert_eq!(table[0].payoff, dec!(0));

        // At S=115 (above all strikes):
        // Long 95: (115-95)-8=12, Short 100 (x2): 2*(5-(115-100))=2*(5-15)=-20
        // Long 105: (115-105)-2=8
        // Total: 12-20+8 = 0
        assert_eq!(table[30].payoff, dec!(0));

        assert_eq!(result.result.strategy_characteristics.direction, "neutral");
    }

    // -----------------------------------------------------------------------
    // 11. Collar (long stock + long put + short call)
    // -----------------------------------------------------------------------
    #[test]
    fn test_collar() {
        // Buy stock at 100, buy put K=95 for 3, sell call K=105 for 3
        // Zero-cost collar (premiums offset)
        let input = StrategyInput {
            strategy_type: StrategyType::Collar,
            underlying_price: dec!(100),
            legs: vec![
                StrategyLeg {
                    leg_type: LegType::Stock,
                    position: LegPosition::Long,
                    strike: None,
                    premium: dec!(100),
                    quantity: dec!(1),
                },
                StrategyLeg {
                    leg_type: LegType::Put,
                    position: LegPosition::Long,
                    strike: Some(dec!(95)),
                    premium: dec!(3),
                    quantity: dec!(1),
                },
                StrategyLeg {
                    leg_type: LegType::Call,
                    position: LegPosition::Short,
                    strike: Some(dec!(105)),
                    premium: dec!(3),
                    quantity: dec!(1),
                },
            ],
            price_range: Some((dec!(80), dec!(120))),
            price_steps: Some(40),
        };

        let result = analyze_strategy(&input).unwrap();
        let table = &result.result.payoff_table;

        // At S=80:
        // Stock: 80-100 = -20
        // Long put: (95-80)-3 = 12
        // Short call: 3-0 = 3
        // Total: -20+12+3 = -5
        assert_eq!(table[0].payoff, dec!(-5));

        // At S=120:
        // Stock: 120-100 = 20
        // Long put: 0-3 = -3
        // Short call: 3-(120-105) = 3-15 = -12
        // Total: 20-3-12 = 5
        assert_eq!(table[40].payoff, dec!(5));

        // Net premium should be zero (3 paid - 3 received = 0)
        assert_eq!(result.result.net_premium, dec!(0));
    }

    // -----------------------------------------------------------------------
    // 12. Breakeven long call (strike + premium)
    // -----------------------------------------------------------------------
    #[test]
    fn test_breakeven_long_call() {
        let input = StrategyInput {
            strategy_type: StrategyType::LongCall,
            underlying_price: dec!(100),
            legs: vec![StrategyLeg {
                leg_type: LegType::Call,
                position: LegPosition::Long,
                strike: Some(dec!(100)),
                premium: dec!(5),
                quantity: dec!(1),
            }],
            price_range: Some((dec!(90), dec!(120))),
            price_steps: Some(300), // fine resolution for accurate interpolation
        };

        let result = analyze_strategy(&input).unwrap();
        let breakevens = &result.result.breakeven_points;

        // Breakeven = strike + premium = 105
        assert_eq!(breakevens.len(), 1);
        assert_eq!(breakevens[0], dec!(105));
    }

    // -----------------------------------------------------------------------
    // 13. Breakeven straddle (2 breakevens)
    // -----------------------------------------------------------------------
    #[test]
    fn test_breakeven_straddle() {
        // K=100, premium=5 each => total premium = 10
        // Breakevens at 90 and 110
        let input = StrategyInput {
            strategy_type: StrategyType::LongStraddle,
            underlying_price: dec!(100),
            legs: vec![
                StrategyLeg {
                    leg_type: LegType::Call,
                    position: LegPosition::Long,
                    strike: Some(dec!(100)),
                    premium: dec!(5),
                    quantity: dec!(1),
                },
                StrategyLeg {
                    leg_type: LegType::Put,
                    position: LegPosition::Long,
                    strike: Some(dec!(100)),
                    premium: dec!(5),
                    quantity: dec!(1),
                },
            ],
            price_range: Some((dec!(80), dec!(120))),
            price_steps: Some(400), // fine resolution
        };

        let result = analyze_strategy(&input).unwrap();
        let breakevens = &result.result.breakeven_points;

        assert_eq!(breakevens.len(), 2);
        // Lower breakeven: K - total_premium = 100 - 10 = 90
        assert_eq!(breakevens[0], dec!(90));
        // Upper breakeven: K + total_premium = 100 + 10 = 110
        assert_eq!(breakevens[1], dec!(110));
    }

    // -----------------------------------------------------------------------
    // 14. Max profit limited (spread)
    // -----------------------------------------------------------------------
    #[test]
    fn test_max_profit_limited() {
        // Bull call spread: buy K=95 for 7, sell K=105 for 2 => net debit 5
        // Max profit = spread width - net debit = 10-5 = 5
        let input = StrategyInput {
            strategy_type: StrategyType::BullCallSpread,
            underlying_price: dec!(100),
            legs: vec![
                StrategyLeg {
                    leg_type: LegType::Call,
                    position: LegPosition::Long,
                    strike: Some(dec!(95)),
                    premium: dec!(7),
                    quantity: dec!(1),
                },
                StrategyLeg {
                    leg_type: LegType::Call,
                    position: LegPosition::Short,
                    strike: Some(dec!(105)),
                    premium: dec!(2),
                    quantity: dec!(1),
                },
            ],
            price_range: Some((dec!(80), dec!(120))),
            price_steps: Some(40),
        };

        let result = analyze_strategy(&input).unwrap();
        // Max profit should be limited (Some value)
        assert!(result.result.max_profit.is_some());
        assert_eq!(result.result.max_profit.unwrap(), dec!(5));
    }

    // -----------------------------------------------------------------------
    // 15. Max loss limited (spread)
    // -----------------------------------------------------------------------
    #[test]
    fn test_max_loss_limited() {
        // Same bull call spread: max loss = net debit = 5
        let input = StrategyInput {
            strategy_type: StrategyType::BullCallSpread,
            underlying_price: dec!(100),
            legs: vec![
                StrategyLeg {
                    leg_type: LegType::Call,
                    position: LegPosition::Long,
                    strike: Some(dec!(95)),
                    premium: dec!(7),
                    quantity: dec!(1),
                },
                StrategyLeg {
                    leg_type: LegType::Call,
                    position: LegPosition::Short,
                    strike: Some(dec!(105)),
                    premium: dec!(2),
                    quantity: dec!(1),
                },
            ],
            price_range: Some((dec!(80), dec!(120))),
            price_steps: Some(40),
        };

        let result = analyze_strategy(&input).unwrap();
        // Max loss should be limited (Some value)
        assert!(result.result.max_loss.is_some());
        assert_eq!(result.result.max_loss.unwrap(), dec!(5));
    }

    // -----------------------------------------------------------------------
    // 16. Net premium credit (short strategy)
    // -----------------------------------------------------------------------
    #[test]
    fn test_net_premium_credit() {
        // Iron condor: sell put 90@3, sell call 110@3, buy put 85@1, buy call 115@1
        // Net premium = (1+1) - (3+3) = -4 (credit of 4)
        let input = StrategyInput {
            strategy_type: StrategyType::IronCondor,
            underlying_price: dec!(100),
            legs: vec![
                StrategyLeg {
                    leg_type: LegType::Put,
                    position: LegPosition::Short,
                    strike: Some(dec!(90)),
                    premium: dec!(3),
                    quantity: dec!(1),
                },
                StrategyLeg {
                    leg_type: LegType::Call,
                    position: LegPosition::Short,
                    strike: Some(dec!(110)),
                    premium: dec!(3),
                    quantity: dec!(1),
                },
                StrategyLeg {
                    leg_type: LegType::Put,
                    position: LegPosition::Long,
                    strike: Some(dec!(85)),
                    premium: dec!(1),
                    quantity: dec!(1),
                },
                StrategyLeg {
                    leg_type: LegType::Call,
                    position: LegPosition::Long,
                    strike: Some(dec!(115)),
                    premium: dec!(1),
                    quantity: dec!(1),
                },
            ],
            price_range: Some((dec!(75), dec!(125))),
            price_steps: Some(50),
        };

        let result = analyze_strategy(&input).unwrap();
        // Net premium negative = credit received
        assert_eq!(result.result.net_premium, dec!(-4));
    }

    // -----------------------------------------------------------------------
    // 17. Empty legs error
    // -----------------------------------------------------------------------
    #[test]
    fn test_empty_legs_error() {
        let input = StrategyInput {
            strategy_type: StrategyType::Custom,
            underlying_price: dec!(100),
            legs: vec![],
            price_range: None,
            price_steps: None,
        };

        let err = analyze_strategy(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "legs");
            }
            _ => panic!("Expected InvalidInput error for empty legs"),
        }
    }

    // -----------------------------------------------------------------------
    // 18. Metadata populated
    // -----------------------------------------------------------------------
    #[test]
    fn test_metadata_populated() {
        let input = StrategyInput {
            strategy_type: StrategyType::LongCall,
            underlying_price: dec!(100),
            legs: vec![StrategyLeg {
                leg_type: LegType::Call,
                position: LegPosition::Long,
                strike: Some(dec!(100)),
                premium: dec!(5),
                quantity: dec!(1),
            }],
            price_range: None,
            price_steps: None,
        };

        let result = analyze_strategy(&input).unwrap();

        assert!(!result.methodology.is_empty());
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
        assert!(!result.metadata.version.is_empty());
        // Assumptions should contain strategy info
        let assumptions = result.assumptions.as_object().unwrap();
        assert!(assumptions.contains_key("strategy_type"));
        assert!(assumptions.contains_key("underlying_price"));
        assert!(assumptions.contains_key("num_legs"));
    }

    // -----------------------------------------------------------------------
    // 19. Quantity multiplier
    // -----------------------------------------------------------------------
    #[test]
    fn test_quantity_multiplier() {
        // 10 contracts of a long call
        let input = StrategyInput {
            strategy_type: StrategyType::LongCall,
            underlying_price: dec!(100),
            legs: vec![StrategyLeg {
                leg_type: LegType::Call,
                position: LegPosition::Long,
                strike: Some(dec!(100)),
                premium: dec!(5),
                quantity: dec!(10),
            }],
            price_range: Some((dec!(80), dec!(120))),
            price_steps: Some(40),
        };

        let result = analyze_strategy(&input).unwrap();
        let table = &result.result.payoff_table;

        // At S=120: (max(0, 120-100) - 5) * 10 = 15*10 = 150
        assert_eq!(table[40].payoff, dec!(150));

        // At S=80: (max(0, 80-100) - 5) * 10 = -5*10 = -50
        assert_eq!(table[0].payoff, dec!(-50));
    }

    // -----------------------------------------------------------------------
    // 20. Invalid strike price error
    // -----------------------------------------------------------------------
    #[test]
    fn test_invalid_strike_error() {
        let input = StrategyInput {
            strategy_type: StrategyType::LongCall,
            underlying_price: dec!(100),
            legs: vec![StrategyLeg {
                leg_type: LegType::Call,
                position: LegPosition::Long,
                strike: Some(dec!(-10)),
                premium: dec!(5),
                quantity: dec!(1),
            }],
            price_range: None,
            price_steps: None,
        };

        let err = analyze_strategy(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert!(field.contains("strike"));
            }
            _ => panic!("Expected InvalidInput error for negative strike"),
        }
    }

    // -----------------------------------------------------------------------
    // 21. Risk reward ratio
    // -----------------------------------------------------------------------
    #[test]
    fn test_risk_reward_ratio() {
        // Bull call spread: max profit=5, max loss=5, ratio=1.0
        let input = StrategyInput {
            strategy_type: StrategyType::BullCallSpread,
            underlying_price: dec!(100),
            legs: vec![
                StrategyLeg {
                    leg_type: LegType::Call,
                    position: LegPosition::Long,
                    strike: Some(dec!(95)),
                    premium: dec!(7),
                    quantity: dec!(1),
                },
                StrategyLeg {
                    leg_type: LegType::Call,
                    position: LegPosition::Short,
                    strike: Some(dec!(105)),
                    premium: dec!(2),
                    quantity: dec!(1),
                },
            ],
            price_range: Some((dec!(80), dec!(120))),
            price_steps: Some(40),
        };

        let result = analyze_strategy(&input).unwrap();
        let rr = result.result.risk_reward_ratio.unwrap();
        assert_eq!(rr, dec!(1));
    }

    // -----------------------------------------------------------------------
    // 22. Profit probability estimate
    // -----------------------------------------------------------------------
    #[test]
    fn test_profit_probability_estimate() {
        let input = StrategyInput {
            strategy_type: StrategyType::LongCall,
            underlying_price: dec!(100),
            legs: vec![StrategyLeg {
                leg_type: LegType::Call,
                position: LegPosition::Long,
                strike: Some(dec!(100)),
                premium: dec!(5),
                quantity: dec!(1),
            }],
            price_range: Some((dec!(80), dec!(120))),
            price_steps: Some(40),
        };

        let result = analyze_strategy(&input).unwrap();
        let prob = result.result.profit_probability_estimate.unwrap();

        // Breakeven at 105; price range 80-120. Points above 105 are profitable.
        // With 40 steps across 40-point range, each step is 1.
        // Profitable: 106,107,...,120 = 15 points out of 41 total
        assert!(prob > Decimal::ZERO);
        assert!(prob < Decimal::ONE);
    }

    // -----------------------------------------------------------------------
    // 23. Default price range and steps
    // -----------------------------------------------------------------------
    #[test]
    fn test_default_price_range() {
        let input = StrategyInput {
            strategy_type: StrategyType::LongCall,
            underlying_price: dec!(100),
            legs: vec![StrategyLeg {
                leg_type: LegType::Call,
                position: LegPosition::Long,
                strike: Some(dec!(100)),
                premium: dec!(5),
                quantity: dec!(1),
            }],
            price_range: None,
            price_steps: None,
        };

        let result = analyze_strategy(&input).unwrap();
        let table = &result.result.payoff_table;

        // Default: spot +/- 30% => 70 to 130, 21 steps => 22 points
        assert_eq!(table.len(), 22);
        assert_eq!(table[0].underlying_price, dec!(70));
        assert_eq!(table[21].underlying_price, dec!(130));
    }

    // -----------------------------------------------------------------------
    // 24. Wrong leg count for strategy type
    // -----------------------------------------------------------------------
    #[test]
    fn test_wrong_leg_count_bull_call_spread() {
        // BullCallSpread needs 2 call legs, giving it 1
        let input = StrategyInput {
            strategy_type: StrategyType::BullCallSpread,
            underlying_price: dec!(100),
            legs: vec![StrategyLeg {
                leg_type: LegType::Call,
                position: LegPosition::Long,
                strike: Some(dec!(95)),
                premium: dec!(7),
                quantity: dec!(1),
            }],
            price_range: None,
            price_steps: None,
        };

        let err = analyze_strategy(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "legs");
            }
            _ => panic!("Expected InvalidInput for wrong leg count"),
        }
    }
}
