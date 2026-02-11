use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Decimal math helpers (pure Decimal, no f64)
// ---------------------------------------------------------------------------

/// Taylor series exp(x) with range reduction for |x| > 2.
fn exp_decimal(x: Decimal) -> Decimal {
    let two = Decimal::from(2);

    let mut k: u32 = 0;
    let mut reduced = x;
    while reduced.abs() > two {
        reduced /= two;
        k += 1;
    }

    let mut sum = Decimal::ONE;
    let mut term = Decimal::ONE;
    for n in 1..=30u64 {
        term *= reduced / Decimal::from(n);
        sum += term;
    }

    for _ in 0..k {
        sum *= sum;
    }

    sum
}

/// Natural logarithm via Newton's method (20 iterations).
fn ln_decimal(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    if x == Decimal::ONE {
        return Decimal::ZERO;
    }

    let mut guess = Decimal::ZERO;
    let mut temp = x;
    let two = Decimal::from(2);
    let ln2_approx = dec!(0.6931471805599453);

    if temp > Decimal::ONE {
        while temp > two {
            temp /= two;
            guess += ln2_approx;
        }
    } else {
        while temp < Decimal::ONE {
            temp *= two;
            guess -= ln2_approx;
        }
    }

    for _ in 0..20 {
        let ey = exp_decimal(guess);
        if ey.is_zero() {
            break;
        }
        guess = guess - Decimal::ONE + x / ey;
    }

    guess
}

/// Newton's method square root (20 iterations).
#[cfg(test)]
fn sqrt_decimal(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    if x == Decimal::ONE {
        return Decimal::ONE;
    }
    let two = Decimal::from(2);
    let mut guess = x / two;
    for _ in 0..20 {
        if guess.is_zero() {
            break;
        }
        guess = (guess + x / guess) / two;
    }
    guess
}

// ---------------------------------------------------------------------------
// Structs
// ---------------------------------------------------------------------------

/// A single futures price on the term structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuturesPrice {
    /// Months until delivery.
    pub month: u32,
    /// Observed futures price.
    pub price: Decimal,
    /// Open interest (optional).
    pub open_interest: Option<Decimal>,
}

/// A monthly seasonal demand factor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeasonalFactor {
    /// Calendar month (1-12).
    pub month: u32,
    /// Demand multiplier relative to average (1.0 = average).
    pub factor: Decimal,
}

/// Input for storage economics analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageEconomicsInput {
    /// Current spot price.
    pub spot_price: Decimal,
    /// Futures term structure.
    pub futures_prices: Vec<FuturesPrice>,
    /// Monthly storage cost per unit.
    pub storage_cost_per_unit_month: Decimal,
    /// Annual financing rate (cost of capital for inventory).
    pub financing_rate: Decimal,
    /// Insurance cost as percentage of value per year (optional).
    pub insurance_cost_pct: Option<Decimal>,
    /// Per-unit handling cost (loading + unloading) (optional).
    pub handling_cost: Option<Decimal>,
    /// Maximum storage capacity in units (optional).
    pub max_storage_capacity: Option<Decimal>,
    /// Current inventory in units (optional).
    pub current_inventory: Option<Decimal>,
    /// Maximum injection rate: units per month in (optional).
    pub injection_rate: Option<Decimal>,
    /// Maximum withdrawal rate: units per month out (optional).
    pub withdrawal_rate: Option<Decimal>,
    /// Monthly seasonal demand factors (optional).
    pub seasonal_factors: Option<Vec<SeasonalFactor>>,
    /// Name of the commodity.
    pub commodity_name: String,
}

/// Implied convenience yield at a given tenor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvenienceYield {
    /// Months to delivery.
    pub months: u32,
    /// Annualised convenience yield.
    pub annualized_yield: Decimal,
    /// Source description.
    pub implied_from: String,
}

/// A seasonal buy-low / sell-high opportunity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeasonalOpportunity {
    /// Month to buy (lowest demand factor).
    pub buy_month: u32,
    /// Month to sell (highest demand factor).
    pub sell_month: u32,
    /// Expected profit per unit.
    pub expected_profit: Decimal,
    /// Confidence label.
    pub confidence: String,
}

/// Economics at a specific tenor (months).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenorEconomics {
    /// Months to delivery.
    pub months: u32,
    /// Futures price at this tenor.
    pub futures_price: Decimal,
    /// Total cost of carry from spot to this tenor.
    pub carry_cost: Decimal,
    /// Net profit from cash-and-carry: futures - spot - carry.
    pub net_profit: Decimal,
    /// Annualised return on the trade.
    pub annualized_return: Decimal,
}

/// Output from storage economics analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageEconomicsOutput {
    /// Overall market structure: "Contango", "Backwardation", or "Mixed".
    pub market_structure: String,
    /// Implied convenience yields at each tenor.
    pub implied_convenience_yields: Vec<ConvenienceYield>,
    /// Total storage arbitrage profit from optimal cash-and-carry.
    pub storage_arbitrage_profit: Decimal,
    /// Optimal number of months to hold commodity in storage.
    pub optimal_storage_months: u32,
    /// Total cost of carry for the optimal period.
    pub total_carry_cost: Decimal,
    /// Theoretical maximum contango (full carry spread).
    pub full_carry_spread: Decimal,
    /// How much of the theoretical full carry is captured.
    pub carry_pct_of_theoretical: Decimal,
    /// Seasonal buy-sell opportunity (if seasonal factors provided).
    pub seasonal_opportunity: Option<SeasonalOpportunity>,
    /// Inventory recommendation: "Build", "Draw", or "Hold".
    pub inventory_recommendation: String,
    /// Detailed economics at each futures tenor.
    pub economics_by_tenor: Vec<TenorEconomics>,
}

// ---------------------------------------------------------------------------
// Core function
// ---------------------------------------------------------------------------

/// Analyse storage economics for a commodity.
///
/// Determines whether it is profitable to buy at spot, store, and sell at a
/// futures price (cash-and-carry arbitrage). Also extracts implied convenience
/// yields, identifies optimal storage duration, and provides seasonal analysis.
///
/// # Cost of Carry
///
/// Full carry = spot * (financing + insurance) * T/12 + storage * T + handling * 2
///
/// # Convenience Yield
///
/// Solved from F = S * exp((r + storage_annual - c) * T):
///   c = r + storage_annual - ln(F/S) / T
///
/// # Storage Arbitrage
///
/// Profit = futures_price - spot_price - carry_cost
pub fn analyze_storage_economics(
    input: &StorageEconomicsInput,
) -> CorpFinanceResult<StorageEconomicsOutput> {
    // -- Validation --
    if input.spot_price <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "spot_price".into(),
            reason: "Spot price must be positive".into(),
        });
    }
    if input.futures_prices.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one futures price is required".into(),
        ));
    }
    if input.storage_cost_per_unit_month < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "storage_cost_per_unit_month".into(),
            reason: "Storage cost must be non-negative".into(),
        });
    }

    for fp in &input.futures_prices {
        if fp.month == 0 {
            return Err(CorpFinanceError::InvalidInput {
                field: "futures_prices.month".into(),
                reason: "Futures month must be positive".into(),
            });
        }
        if fp.price <= Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "futures_prices.price".into(),
                reason: "Futures price must be positive".into(),
            });
        }
    }

    let s = input.spot_price;
    let r = input.financing_rate;
    let insurance = input.insurance_cost_pct.unwrap_or(Decimal::ZERO);
    let handling = input.handling_cost.unwrap_or(Decimal::ZERO);
    let twelve = Decimal::from(12);

    // Annual storage as a rate (for convenience yield calculation)
    let storage_annual = if s > Decimal::ZERO {
        input.storage_cost_per_unit_month * twelve / s
    } else {
        Decimal::ZERO
    };

    // -- Market structure --
    let mut contango_count: usize = 0;
    let mut backwardation_count: usize = 0;
    for fp in &input.futures_prices {
        if fp.price > s {
            contango_count += 1;
        } else if fp.price < s {
            backwardation_count += 1;
        }
    }
    let market_structure = if contango_count > 0 && backwardation_count > 0 {
        "Mixed".to_string()
    } else if contango_count > 0 {
        "Contango".to_string()
    } else if backwardation_count > 0 {
        "Backwardation".to_string()
    } else {
        "Contango".to_string() // flat = contango (at carry)
    };

    // -- Compute economics by tenor --
    let mut economics_by_tenor: Vec<TenorEconomics> = Vec::new();
    let mut implied_convenience_yields: Vec<ConvenienceYield> = Vec::new();
    let mut best_profit = Decimal::MIN;
    let mut best_months: u32 = 0;
    let mut best_carry: Decimal = Decimal::ZERO;

    // Sort futures by month for processing
    let mut sorted_futures = input.futures_prices.clone();
    sorted_futures.sort_by_key(|f| f.month);

    for fp in &sorted_futures {
        let t_months = Decimal::from(fp.month);
        let t_years = t_months / twelve;

        // Full carry cost = financing + insurance (on spot) + storage + handling
        // financing + insurance cost: spot * (r + insurance) * T/12
        let finance_cost = s * (r + insurance) * t_years;
        let storage_cost = input.storage_cost_per_unit_month * t_months;
        let handling_total = handling * Decimal::from(2); // in and out
        let carry_cost = finance_cost + storage_cost + handling_total;

        // Net profit from cash-and-carry
        let net_profit = fp.price - s - carry_cost;

        // Annualised return
        let capital = s + carry_cost;
        let annualized_return = if capital > Decimal::ZERO && t_years > Decimal::ZERO {
            (net_profit / capital) * (twelve / t_months)
        } else {
            Decimal::ZERO
        };

        economics_by_tenor.push(TenorEconomics {
            months: fp.month,
            futures_price: fp.price,
            carry_cost,
            net_profit,
            annualized_return,
        });

        // Track best tenor
        if net_profit > best_profit {
            best_profit = net_profit;
            best_months = fp.month;
            best_carry = carry_cost;
        }

        // Implied convenience yield:
        // F = S * exp((r + storage_annual - c) * T)
        // ln(F/S) = (r + storage_annual - c) * T
        // c = r + storage_annual - ln(F/S) / T
        if t_years > Decimal::ZERO && s > Decimal::ZERO {
            let ratio = fp.price / s;
            let ln_ratio = ln_decimal(ratio);
            let cy = r + storage_annual - ln_ratio / t_years;

            implied_convenience_yields.push(ConvenienceYield {
                months: fp.month,
                annualized_yield: cy,
                implied_from: format!("{} month futures at {}", fp.month, fp.price),
            });
        }
    }

    // -- Optimal storage --
    let optimal_storage_months = best_months;
    let total_carry_cost = best_carry;
    let storage_arbitrage_profit = if best_profit > Decimal::ZERO {
        best_profit
    } else {
        Decimal::ZERO
    };

    // -- Full carry spread: theoretical max contango --
    // Use the longest tenor for the full carry reference
    let max_tenor_months = sorted_futures.last().map(|f| f.month).unwrap_or(1);
    let max_t_months = Decimal::from(max_tenor_months);
    let max_t_years = max_t_months / twelve;
    let full_carry = s * (r + insurance) * max_t_years
        + input.storage_cost_per_unit_month * max_t_months
        + handling * Decimal::from(2);

    let full_carry_spread = full_carry;

    // Actual spread at the longest tenor
    let actual_spread = sorted_futures
        .last()
        .map(|f| f.price - s)
        .unwrap_or(Decimal::ZERO);

    let carry_pct_of_theoretical = if full_carry_spread > Decimal::ZERO {
        actual_spread / full_carry_spread
    } else if actual_spread > Decimal::ZERO {
        Decimal::ONE
    } else {
        Decimal::ZERO
    };

    // -- Seasonal opportunity --
    let seasonal_opportunity = compute_seasonal_opportunity(input, s);

    // -- Inventory recommendation --
    // If profitable storage arbitrage exists (contango > carry) => "Build"
    // If backwardation => "Draw"
    // Otherwise => "Hold"
    let inventory_recommendation = if storage_arbitrage_profit > Decimal::ZERO {
        "Build".to_string()
    } else if market_structure == "Backwardation" {
        "Draw".to_string()
    } else {
        "Hold".to_string()
    };

    Ok(StorageEconomicsOutput {
        market_structure,
        implied_convenience_yields,
        storage_arbitrage_profit,
        optimal_storage_months,
        total_carry_cost,
        full_carry_spread,
        carry_pct_of_theoretical,
        seasonal_opportunity,
        inventory_recommendation,
        economics_by_tenor,
    })
}

/// Compute seasonal buy-sell opportunity if seasonal factors are provided.
fn compute_seasonal_opportunity(
    input: &StorageEconomicsInput,
    spot: Decimal,
) -> Option<SeasonalOpportunity> {
    let factors = match &input.seasonal_factors {
        Some(f) if f.len() >= 2 => f,
        _ => return None,
    };

    // Find month with lowest factor (buy) and highest factor (sell)
    let mut min_factor = Decimal::MAX;
    let mut min_month: u32 = 0;
    let mut max_factor = Decimal::MIN;
    let mut max_month: u32 = 0;

    for sf in factors {
        if sf.factor < min_factor {
            min_factor = sf.factor;
            min_month = sf.month;
        }
        if sf.factor > max_factor {
            max_factor = sf.factor;
            max_month = sf.month;
        }
    }

    // Storage duration between buy and sell months
    let storage_months = if max_month > min_month {
        max_month - min_month
    } else {
        max_month + 12 - min_month
    };

    // Carry cost over the storage period
    let t_months = Decimal::from(storage_months);
    let twelve = Decimal::from(12);
    let t_years = t_months / twelve;
    let insurance = input.insurance_cost_pct.unwrap_or(Decimal::ZERO);
    let handling = input.handling_cost.unwrap_or(Decimal::ZERO);
    let carry = spot * (input.financing_rate + insurance) * t_years
        + input.storage_cost_per_unit_month * t_months
        + handling * Decimal::from(2);

    // Expected profit = spot * (high_factor - low_factor) - carry
    let price_diff = spot * (max_factor - min_factor);
    let expected_profit = price_diff - carry;

    // Confidence based on factor spread
    let factor_spread = max_factor - min_factor;
    let confidence = if factor_spread > dec!(0.3) {
        "High"
    } else if factor_spread > dec!(0.15) {
        "Medium"
    } else {
        "Low"
    };

    Some(SeasonalOpportunity {
        buy_month: min_month,
        sell_month: max_month,
        expected_profit,
        confidence: confidence.to_string(),
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn tol() -> Decimal {
        dec!(0.01)
    }

    fn wide_tol() -> Decimal {
        dec!(0.5)
    }

    fn assert_approx(actual: Decimal, expected: Decimal, tolerance: Decimal, label: &str) {
        let diff = (actual - expected).abs();
        assert!(
            diff <= tolerance,
            "{label}: expected ~{expected}, got {actual} (diff={diff}, tol={tolerance})"
        );
    }

    // Helper: basic contango market
    fn contango_input() -> StorageEconomicsInput {
        StorageEconomicsInput {
            spot_price: dec!(80),
            futures_prices: vec![
                FuturesPrice {
                    month: 3,
                    price: dec!(82),
                    open_interest: Some(dec!(50000)),
                },
                FuturesPrice {
                    month: 6,
                    price: dec!(84),
                    open_interest: Some(dec!(30000)),
                },
                FuturesPrice {
                    month: 12,
                    price: dec!(88),
                    open_interest: Some(dec!(20000)),
                },
            ],
            storage_cost_per_unit_month: dec!(0.50),
            financing_rate: dec!(0.05),
            insurance_cost_pct: Some(dec!(0.005)),
            handling_cost: Some(dec!(0.25)),
            max_storage_capacity: Some(dec!(100000)),
            current_inventory: Some(dec!(50000)),
            injection_rate: Some(dec!(10000)),
            withdrawal_rate: Some(dec!(15000)),
            seasonal_factors: None,
            commodity_name: "WTI Crude Oil".into(),
        }
    }

    // Helper: backwardation market
    fn backwardation_input() -> StorageEconomicsInput {
        StorageEconomicsInput {
            spot_price: dec!(90),
            futures_prices: vec![
                FuturesPrice {
                    month: 3,
                    price: dec!(88),
                    open_interest: None,
                },
                FuturesPrice {
                    month: 6,
                    price: dec!(86),
                    open_interest: None,
                },
                FuturesPrice {
                    month: 12,
                    price: dec!(83),
                    open_interest: None,
                },
            ],
            storage_cost_per_unit_month: dec!(0.50),
            financing_rate: dec!(0.05),
            insurance_cost_pct: None,
            handling_cost: None,
            max_storage_capacity: None,
            current_inventory: None,
            injection_rate: None,
            withdrawal_rate: None,
            seasonal_factors: None,
            commodity_name: "Brent Crude".into(),
        }
    }

    // -----------------------------------------------------------------------
    // 1. Contango market structure detection
    // -----------------------------------------------------------------------
    #[test]
    fn test_contango_market_structure() {
        let input = contango_input();
        let result = analyze_storage_economics(&input).unwrap();
        assert_eq!(result.market_structure, "Contango");
    }

    // -----------------------------------------------------------------------
    // 2. Backwardation market structure detection
    // -----------------------------------------------------------------------
    #[test]
    fn test_backwardation_market_structure() {
        let input = backwardation_input();
        let result = analyze_storage_economics(&input).unwrap();
        assert_eq!(result.market_structure, "Backwardation");
    }

    // -----------------------------------------------------------------------
    // 3. Mixed market structure
    // -----------------------------------------------------------------------
    #[test]
    fn test_mixed_market_structure() {
        let input = StorageEconomicsInput {
            spot_price: dec!(80),
            futures_prices: vec![
                FuturesPrice {
                    month: 3,
                    price: dec!(82),
                    open_interest: None,
                },
                FuturesPrice {
                    month: 6,
                    price: dec!(78),
                    open_interest: None,
                },
            ],
            storage_cost_per_unit_month: dec!(0.50),
            financing_rate: dec!(0.05),
            insurance_cost_pct: None,
            handling_cost: None,
            max_storage_capacity: None,
            current_inventory: None,
            injection_rate: None,
            withdrawal_rate: None,
            seasonal_factors: None,
            commodity_name: "Test Oil".into(),
        };
        let result = analyze_storage_economics(&input).unwrap();
        assert_eq!(result.market_structure, "Mixed");
    }

    // -----------------------------------------------------------------------
    // 4. Carry cost calculation
    // -----------------------------------------------------------------------
    #[test]
    fn test_carry_cost_calculation() {
        let input = contango_input();
        let result = analyze_storage_economics(&input).unwrap();

        // 3-month carry:
        // finance = 80 * (0.05 + 0.005) * 3/12 = 80 * 0.055 * 0.25 = 1.10
        // storage = 0.50 * 3 = 1.50
        // handling = 0.25 * 2 = 0.50
        // total = 1.10 + 1.50 + 0.50 = 3.10
        let tenor3 = result
            .economics_by_tenor
            .iter()
            .find(|t| t.months == 3)
            .unwrap();
        assert_approx(tenor3.carry_cost, dec!(3.10), tol(), "3m carry");
    }

    // -----------------------------------------------------------------------
    // 5. Storage arbitrage profit (contango)
    // -----------------------------------------------------------------------
    #[test]
    fn test_storage_arbitrage_profit_contango() {
        let input = contango_input();
        let result = analyze_storage_economics(&input).unwrap();

        // At 3 months: profit = 82 - 80 - 3.10 = -1.10 (negative)
        // At 6 months: carry = 80*0.055*0.5 + 0.50*6 + 0.50 = 2.20 + 3.00 + 0.50 = 5.70
        //   profit = 84 - 80 - 5.70 = -1.70
        // At 12 months: carry = 80*0.055*1 + 0.50*12 + 0.50 = 4.40 + 6.00 + 0.50 = 10.90
        //   profit = 88 - 80 - 10.90 = -2.90
        // All negative => storage_arbitrage_profit = 0 (no profitable arbitrage)
        // The best (least negative) = 3m at -1.10
        assert_approx(
            result.storage_arbitrage_profit,
            Decimal::ZERO,
            tol(),
            "no arb profit",
        );
    }

    // -----------------------------------------------------------------------
    // 6. Profitable storage arbitrage
    // -----------------------------------------------------------------------
    #[test]
    fn test_profitable_storage_arbitrage() {
        // Wide contango that exceeds carry costs
        let input = StorageEconomicsInput {
            spot_price: dec!(80),
            futures_prices: vec![FuturesPrice {
                month: 6,
                price: dec!(90),
                open_interest: None,
            }],
            storage_cost_per_unit_month: dec!(0.20),
            financing_rate: dec!(0.03),
            insurance_cost_pct: None,
            handling_cost: None,
            max_storage_capacity: None,
            current_inventory: None,
            injection_rate: None,
            withdrawal_rate: None,
            seasonal_factors: None,
            commodity_name: "Cheap Oil".into(),
        };
        let result = analyze_storage_economics(&input).unwrap();

        // carry = 80 * 0.03 * 0.5 + 0.20 * 6 = 1.20 + 1.20 = 2.40
        // profit = 90 - 80 - 2.40 = 7.60
        assert_approx(
            result.storage_arbitrage_profit,
            dec!(7.60),
            tol(),
            "profitable arb",
        );
        assert_eq!(result.optimal_storage_months, 6);
        assert_eq!(result.inventory_recommendation, "Build");
    }

    // -----------------------------------------------------------------------
    // 7. Optimal storage months
    // -----------------------------------------------------------------------
    #[test]
    fn test_optimal_storage_months() {
        // 3 month is most profitable
        let input = StorageEconomicsInput {
            spot_price: dec!(80),
            futures_prices: vec![
                FuturesPrice {
                    month: 3,
                    price: dec!(86),
                    open_interest: None,
                },
                FuturesPrice {
                    month: 6,
                    price: dec!(85),
                    open_interest: None,
                },
                FuturesPrice {
                    month: 12,
                    price: dec!(84),
                    open_interest: None,
                },
            ],
            storage_cost_per_unit_month: dec!(0.20),
            financing_rate: dec!(0.03),
            insurance_cost_pct: None,
            handling_cost: None,
            max_storage_capacity: None,
            current_inventory: None,
            injection_rate: None,
            withdrawal_rate: None,
            seasonal_factors: None,
            commodity_name: "Oil".into(),
        };
        let result = analyze_storage_economics(&input).unwrap();

        // 3m: carry = 80*0.03*0.25 + 0.20*3 = 0.60 + 0.60 = 1.20, profit = 86-80-1.20 = 4.80
        // 6m: carry = 80*0.03*0.5 + 0.20*6 = 1.20 + 1.20 = 2.40, profit = 85-80-2.40 = 2.60
        // 12m: carry = 80*0.03*1 + 0.20*12 = 2.40+2.40 = 4.80, profit = 84-80-4.80 = -0.80
        assert_eq!(result.optimal_storage_months, 3);
        assert_approx(
            result.storage_arbitrage_profit,
            dec!(4.80),
            tol(),
            "optimal profit",
        );
    }

    // -----------------------------------------------------------------------
    // 8. Full carry spread
    // -----------------------------------------------------------------------
    #[test]
    fn test_full_carry_spread() {
        let input = contango_input();
        let result = analyze_storage_economics(&input).unwrap();

        // Full carry at 12 months (longest tenor):
        // finance = 80 * (0.05 + 0.005) * 1 = 4.40
        // storage = 0.50 * 12 = 6.00
        // handling = 0.25 * 2 = 0.50
        // total = 10.90
        assert_approx(result.full_carry_spread, dec!(10.90), tol(), "full carry");
    }

    // -----------------------------------------------------------------------
    // 9. Carry percentage of theoretical
    // -----------------------------------------------------------------------
    #[test]
    fn test_carry_pct_of_theoretical() {
        let input = contango_input();
        let result = analyze_storage_economics(&input).unwrap();

        // Actual spread at 12m = 88 - 80 = 8
        // Full carry = 10.90
        // pct = 8 / 10.90 ~ 0.7339
        let expected = dec!(8) / dec!(10.90);
        assert_approx(
            result.carry_pct_of_theoretical,
            expected,
            tol(),
            "carry pct",
        );
    }

    // -----------------------------------------------------------------------
    // 10. Implied convenience yield: contango
    // -----------------------------------------------------------------------
    #[test]
    fn test_implied_convenience_yield_contango() {
        let input = contango_input();
        let result = analyze_storage_economics(&input).unwrap();

        // Convenience yields should be present for each tenor
        assert_eq!(result.implied_convenience_yields.len(), 3);

        // In contango, convenience yield should be positive but moderate
        // c = r + storage_annual - ln(F/S)/T
        // For 12m: storage_annual = 0.50 * 12 / 80 = 0.075
        // ln(88/80) = ln(1.1) ~ 0.09531
        // c = 0.05 + 0.075 - 0.09531 = 0.02969
        let cy_12m = result
            .implied_convenience_yields
            .iter()
            .find(|c| c.months == 12)
            .unwrap();
        assert_approx(
            cy_12m.annualized_yield,
            dec!(0.02969),
            dec!(0.01),
            "CY 12m contango",
        );
    }

    // -----------------------------------------------------------------------
    // 11. Implied convenience yield: backwardation
    // -----------------------------------------------------------------------
    #[test]
    fn test_implied_convenience_yield_backwardation() {
        let input = backwardation_input();
        let result = analyze_storage_economics(&input).unwrap();

        // In backwardation, convenience yield should be higher
        // storage_annual = 0.50 * 12 / 90 = 0.06667
        // For 12m: ln(83/90) = ln(0.9222) ~ -0.08101
        // c = 0.05 + 0.06667 - (-0.08101) = 0.19768
        let cy_12m = result
            .implied_convenience_yields
            .iter()
            .find(|c| c.months == 12)
            .unwrap();
        assert!(
            cy_12m.annualized_yield > dec!(0.1),
            "CY in backwardation should be high, got {}",
            cy_12m.annualized_yield
        );
    }

    // -----------------------------------------------------------------------
    // 12. Backwardation: no storage profit
    // -----------------------------------------------------------------------
    #[test]
    fn test_backwardation_no_storage_profit() {
        let input = backwardation_input();
        let result = analyze_storage_economics(&input).unwrap();

        assert_approx(
            result.storage_arbitrage_profit,
            Decimal::ZERO,
            tol(),
            "no arb in backwardation",
        );
    }

    // -----------------------------------------------------------------------
    // 13. Inventory recommendation: Build
    // -----------------------------------------------------------------------
    #[test]
    fn test_inventory_recommendation_build() {
        let input = StorageEconomicsInput {
            spot_price: dec!(80),
            futures_prices: vec![FuturesPrice {
                month: 6,
                price: dec!(90),
                open_interest: None,
            }],
            storage_cost_per_unit_month: dec!(0.10),
            financing_rate: dec!(0.02),
            insurance_cost_pct: None,
            handling_cost: None,
            max_storage_capacity: None,
            current_inventory: None,
            injection_rate: None,
            withdrawal_rate: None,
            seasonal_factors: None,
            commodity_name: "Oil".into(),
        };
        let result = analyze_storage_economics(&input).unwrap();
        assert_eq!(result.inventory_recommendation, "Build");
    }

    // -----------------------------------------------------------------------
    // 14. Inventory recommendation: Draw
    // -----------------------------------------------------------------------
    #[test]
    fn test_inventory_recommendation_draw() {
        let input = backwardation_input();
        let result = analyze_storage_economics(&input).unwrap();
        assert_eq!(result.inventory_recommendation, "Draw");
    }

    // -----------------------------------------------------------------------
    // 15. Inventory recommendation: Hold
    // -----------------------------------------------------------------------
    #[test]
    fn test_inventory_recommendation_hold() {
        // Slight contango but not enough to cover costs
        let input = contango_input();
        let result = analyze_storage_economics(&input).unwrap();
        // All tenors negative profit => no arb, but market is contango => "Hold"
        assert_eq!(result.inventory_recommendation, "Hold");
    }

    // -----------------------------------------------------------------------
    // 16. Seasonal opportunity
    // -----------------------------------------------------------------------
    #[test]
    fn test_seasonal_opportunity() {
        let mut input = contango_input();
        input.seasonal_factors = Some(vec![
            SeasonalFactor {
                month: 4,
                factor: dec!(0.8),
            }, // buy: low demand
            SeasonalFactor {
                month: 7,
                factor: dec!(1.0),
            },
            SeasonalFactor {
                month: 1,
                factor: dec!(1.3),
            }, // sell: high demand
        ]);
        let result = analyze_storage_economics(&input).unwrap();

        assert!(result.seasonal_opportunity.is_some());
        let opp = result.seasonal_opportunity.unwrap();
        assert_eq!(opp.buy_month, 4);
        assert_eq!(opp.sell_month, 1);
    }

    // -----------------------------------------------------------------------
    // 17. Seasonal opportunity profit calculation
    // -----------------------------------------------------------------------
    #[test]
    fn test_seasonal_profit_calculation() {
        let input = StorageEconomicsInput {
            spot_price: dec!(100),
            futures_prices: vec![FuturesPrice {
                month: 6,
                price: dec!(105),
                open_interest: None,
            }],
            storage_cost_per_unit_month: dec!(0.50),
            financing_rate: dec!(0.05),
            insurance_cost_pct: None,
            handling_cost: None,
            max_storage_capacity: None,
            current_inventory: None,
            injection_rate: None,
            withdrawal_rate: None,
            seasonal_factors: Some(vec![
                SeasonalFactor {
                    month: 6,
                    factor: dec!(0.9),
                }, // buy
                SeasonalFactor {
                    month: 12,
                    factor: dec!(1.2),
                }, // sell
            ]),
            commodity_name: "Gas".into(),
        };
        let result = analyze_storage_economics(&input).unwrap();

        let opp = result.seasonal_opportunity.unwrap();
        // price_diff = 100 * (1.2 - 0.9) = 30
        // storage months = 12 - 6 = 6
        // carry = 100 * 0.05 * 0.5 + 0.50 * 6 = 2.50 + 3.00 = 5.50
        // profit = 30 - 5.50 = 24.50
        assert_approx(
            opp.expected_profit,
            dec!(24.50),
            wide_tol(),
            "seasonal profit",
        );
    }

    // -----------------------------------------------------------------------
    // 18. No seasonal factors: seasonal opportunity is None
    // -----------------------------------------------------------------------
    #[test]
    fn test_no_seasonal_factors() {
        let input = contango_input();
        let result = analyze_storage_economics(&input).unwrap();
        assert!(result.seasonal_opportunity.is_none());
    }

    // -----------------------------------------------------------------------
    // 19. Economics by tenor count
    // -----------------------------------------------------------------------
    #[test]
    fn test_economics_by_tenor_count() {
        let input = contango_input();
        let result = analyze_storage_economics(&input).unwrap();
        assert_eq!(result.economics_by_tenor.len(), 3);
    }

    // -----------------------------------------------------------------------
    // 20. Annualised return calculation
    // -----------------------------------------------------------------------
    #[test]
    fn test_annualized_return() {
        let input = StorageEconomicsInput {
            spot_price: dec!(80),
            futures_prices: vec![FuturesPrice {
                month: 6,
                price: dec!(90),
                open_interest: None,
            }],
            storage_cost_per_unit_month: dec!(0.20),
            financing_rate: dec!(0.03),
            insurance_cost_pct: None,
            handling_cost: None,
            max_storage_capacity: None,
            current_inventory: None,
            injection_rate: None,
            withdrawal_rate: None,
            seasonal_factors: None,
            commodity_name: "Oil".into(),
        };
        let result = analyze_storage_economics(&input).unwrap();
        let tenor = &result.economics_by_tenor[0];

        // carry = 80*0.03*0.5 + 0.20*6 = 1.20 + 1.20 = 2.40
        // profit = 90 - 80 - 2.40 = 7.60
        // capital = 80 + 2.40 = 82.40
        // annualised = (7.60 / 82.40) * (12/6) = 0.09223 * 2 = 0.18447
        let expected_return = (dec!(7.60) / dec!(82.40)) * dec!(2);
        assert_approx(
            tenor.annualized_return,
            expected_return,
            tol(),
            "annualised return",
        );
    }

    // -----------------------------------------------------------------------
    // 21. Single futures price
    // -----------------------------------------------------------------------
    #[test]
    fn test_single_futures_price() {
        let input = StorageEconomicsInput {
            spot_price: dec!(80),
            futures_prices: vec![FuturesPrice {
                month: 3,
                price: dec!(82),
                open_interest: None,
            }],
            storage_cost_per_unit_month: dec!(0.50),
            financing_rate: dec!(0.05),
            insurance_cost_pct: None,
            handling_cost: None,
            max_storage_capacity: None,
            current_inventory: None,
            injection_rate: None,
            withdrawal_rate: None,
            seasonal_factors: None,
            commodity_name: "Oil".into(),
        };
        let result = analyze_storage_economics(&input).unwrap();

        assert_eq!(result.economics_by_tenor.len(), 1);
        assert_eq!(result.implied_convenience_yields.len(), 1);
        assert_eq!(result.optimal_storage_months, 3);
    }

    // -----------------------------------------------------------------------
    // 22. Zero storage cost
    // -----------------------------------------------------------------------
    #[test]
    fn test_zero_storage_cost() {
        let input = StorageEconomicsInput {
            spot_price: dec!(100),
            futures_prices: vec![FuturesPrice {
                month: 6,
                price: dec!(105),
                open_interest: None,
            }],
            storage_cost_per_unit_month: Decimal::ZERO,
            financing_rate: dec!(0.05),
            insurance_cost_pct: None,
            handling_cost: None,
            max_storage_capacity: None,
            current_inventory: None,
            injection_rate: None,
            withdrawal_rate: None,
            seasonal_factors: None,
            commodity_name: "Gold".into(),
        };
        let result = analyze_storage_economics(&input).unwrap();

        // carry = 100 * 0.05 * 0.5 = 2.50 (financing only)
        // profit = 105 - 100 - 2.50 = 2.50
        let tenor = &result.economics_by_tenor[0];
        assert_approx(tenor.carry_cost, dec!(2.50), tol(), "zero storage carry");
        assert_approx(tenor.net_profit, dec!(2.50), tol(), "zero storage profit");
    }

    // -----------------------------------------------------------------------
    // 23. Handling cost included
    // -----------------------------------------------------------------------
    #[test]
    fn test_handling_cost_included() {
        let input = StorageEconomicsInput {
            spot_price: dec!(100),
            futures_prices: vec![FuturesPrice {
                month: 6,
                price: dec!(110),
                open_interest: None,
            }],
            storage_cost_per_unit_month: dec!(0.50),
            financing_rate: dec!(0.05),
            insurance_cost_pct: None,
            handling_cost: Some(dec!(1.0)), // $1 per unit in + out
            max_storage_capacity: None,
            current_inventory: None,
            injection_rate: None,
            withdrawal_rate: None,
            seasonal_factors: None,
            commodity_name: "Oil".into(),
        };
        let result = analyze_storage_economics(&input).unwrap();

        // carry = 100*0.05*0.5 + 0.50*6 + 1.0*2 = 2.50 + 3.00 + 2.00 = 7.50
        let tenor = &result.economics_by_tenor[0];
        assert_approx(tenor.carry_cost, dec!(7.50), tol(), "with handling");
    }

    // -----------------------------------------------------------------------
    // 24. Insurance cost included
    // -----------------------------------------------------------------------
    #[test]
    fn test_insurance_cost_included() {
        let input = StorageEconomicsInput {
            spot_price: dec!(100),
            futures_prices: vec![FuturesPrice {
                month: 12,
                price: dec!(110),
                open_interest: None,
            }],
            storage_cost_per_unit_month: dec!(0.50),
            financing_rate: dec!(0.05),
            insurance_cost_pct: Some(dec!(0.01)), // 1% per year
            handling_cost: None,
            max_storage_capacity: None,
            current_inventory: None,
            injection_rate: None,
            withdrawal_rate: None,
            seasonal_factors: None,
            commodity_name: "Oil".into(),
        };
        let result = analyze_storage_economics(&input).unwrap();

        // carry = 100*(0.05+0.01)*1 + 0.50*12 = 6.00 + 6.00 = 12.00
        let tenor = &result.economics_by_tenor[0];
        assert_approx(tenor.carry_cost, dec!(12.00), tol(), "with insurance");
    }

    // -----------------------------------------------------------------------
    // 25. Validation: spot price must be positive
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_spot_positive() {
        let mut input = contango_input();
        input.spot_price = Decimal::ZERO;
        let err = analyze_storage_economics(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "spot_price");
            }
            e => panic!("Expected InvalidInput, got {e:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // 26. Validation: empty futures
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_empty_futures() {
        let mut input = contango_input();
        input.futures_prices = vec![];
        let err = analyze_storage_economics(&input).unwrap_err();
        match err {
            CorpFinanceError::InsufficientData(_) => {}
            e => panic!("Expected InsufficientData, got {e:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // 27. Validation: negative storage cost
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_negative_storage_cost() {
        let mut input = contango_input();
        input.storage_cost_per_unit_month = dec!(-1);
        let err = analyze_storage_economics(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "storage_cost_per_unit_month");
            }
            e => panic!("Expected InvalidInput, got {e:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // 28. Validation: futures month must be positive
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_futures_month_zero() {
        let mut input = contango_input();
        input.futures_prices = vec![FuturesPrice {
            month: 0,
            price: dec!(82),
            open_interest: None,
        }];
        let err = analyze_storage_economics(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "futures_prices.month");
            }
            e => panic!("Expected InvalidInput, got {e:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // 29. Validation: futures price must be positive
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_futures_price_positive() {
        let mut input = contango_input();
        input.futures_prices = vec![FuturesPrice {
            month: 3,
            price: Decimal::ZERO,
            open_interest: None,
        }];
        let err = analyze_storage_economics(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "futures_prices.price");
            }
            e => panic!("Expected InvalidInput, got {e:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // 30. Seasonal confidence: High
    // -----------------------------------------------------------------------
    #[test]
    fn test_seasonal_confidence_high() {
        let mut input = contango_input();
        input.seasonal_factors = Some(vec![
            SeasonalFactor {
                month: 4,
                factor: dec!(0.7),
            },
            SeasonalFactor {
                month: 1,
                factor: dec!(1.4),
            },
        ]);
        let result = analyze_storage_economics(&input).unwrap();
        let opp = result.seasonal_opportunity.unwrap();
        // spread = 1.4 - 0.7 = 0.7 > 0.3 => "High"
        assert_eq!(opp.confidence, "High");
    }

    // -----------------------------------------------------------------------
    // 31. Seasonal confidence: Medium
    // -----------------------------------------------------------------------
    #[test]
    fn test_seasonal_confidence_medium() {
        let mut input = contango_input();
        input.seasonal_factors = Some(vec![
            SeasonalFactor {
                month: 4,
                factor: dec!(0.9),
            },
            SeasonalFactor {
                month: 1,
                factor: dec!(1.1),
            },
        ]);
        let result = analyze_storage_economics(&input).unwrap();
        let opp = result.seasonal_opportunity.unwrap();
        // spread = 1.1 - 0.9 = 0.2, between 0.15 and 0.3 => "Medium"
        assert_eq!(opp.confidence, "Medium");
    }

    // -----------------------------------------------------------------------
    // 32. Seasonal confidence: Low
    // -----------------------------------------------------------------------
    #[test]
    fn test_seasonal_confidence_low() {
        let mut input = contango_input();
        input.seasonal_factors = Some(vec![
            SeasonalFactor {
                month: 4,
                factor: dec!(0.95),
            },
            SeasonalFactor {
                month: 1,
                factor: dec!(1.05),
            },
        ]);
        let result = analyze_storage_economics(&input).unwrap();
        let opp = result.seasonal_opportunity.unwrap();
        // spread = 1.05 - 0.95 = 0.10 < 0.15 => "Low"
        assert_eq!(opp.confidence, "Low");
    }

    // -----------------------------------------------------------------------
    // 33. Futures sorted by month internally
    // -----------------------------------------------------------------------
    #[test]
    fn test_futures_sorted_by_month() {
        // Pass in reverse order
        let input = StorageEconomicsInput {
            spot_price: dec!(80),
            futures_prices: vec![
                FuturesPrice {
                    month: 12,
                    price: dec!(88),
                    open_interest: None,
                },
                FuturesPrice {
                    month: 3,
                    price: dec!(82),
                    open_interest: None,
                },
                FuturesPrice {
                    month: 6,
                    price: dec!(84),
                    open_interest: None,
                },
            ],
            storage_cost_per_unit_month: dec!(0.50),
            financing_rate: dec!(0.05),
            insurance_cost_pct: None,
            handling_cost: None,
            max_storage_capacity: None,
            current_inventory: None,
            injection_rate: None,
            withdrawal_rate: None,
            seasonal_factors: None,
            commodity_name: "Oil".into(),
        };
        let result = analyze_storage_economics(&input).unwrap();

        // Economics should be in sorted order
        assert_eq!(result.economics_by_tenor[0].months, 3);
        assert_eq!(result.economics_by_tenor[1].months, 6);
        assert_eq!(result.economics_by_tenor[2].months, 12);
    }

    // -----------------------------------------------------------------------
    // 34. Convenience yield structure is populated
    // -----------------------------------------------------------------------
    #[test]
    fn test_convenience_yield_structure() {
        let input = contango_input();
        let result = analyze_storage_economics(&input).unwrap();

        for cy in &result.implied_convenience_yields {
            assert!(cy.months > 0);
            assert!(!cy.implied_from.is_empty());
        }
    }

    // -----------------------------------------------------------------------
    // 35. Seasonal wrap-around (buy in November, sell in March)
    // -----------------------------------------------------------------------
    #[test]
    fn test_seasonal_wrap_around() {
        let mut input = contango_input();
        input.seasonal_factors = Some(vec![
            SeasonalFactor {
                month: 11,
                factor: dec!(0.8),
            }, // buy
            SeasonalFactor {
                month: 3,
                factor: dec!(1.3),
            }, // sell
        ]);
        let result = analyze_storage_economics(&input).unwrap();
        let opp = result.seasonal_opportunity.unwrap();
        assert_eq!(opp.buy_month, 11);
        assert_eq!(opp.sell_month, 3);
        // Storage months: 3 + 12 - 11 = 4 months
    }

    // -----------------------------------------------------------------------
    // 36. Net profit sign by tenor
    // -----------------------------------------------------------------------
    #[test]
    fn test_net_profit_sign() {
        let input = backwardation_input();
        let result = analyze_storage_economics(&input).unwrap();

        // All tenors should have negative profit in backwardation
        for tenor in &result.economics_by_tenor {
            assert!(
                tenor.net_profit < Decimal::ZERO,
                "Tenor {}m should be negative, got {}",
                tenor.months,
                tenor.net_profit
            );
        }
    }

    // -----------------------------------------------------------------------
    // 37. Carry cost increases with tenor
    // -----------------------------------------------------------------------
    #[test]
    fn test_carry_cost_increases_with_tenor() {
        let input = contango_input();
        let result = analyze_storage_economics(&input).unwrap();

        for i in 1..result.economics_by_tenor.len() {
            assert!(
                result.economics_by_tenor[i].carry_cost
                    > result.economics_by_tenor[i - 1].carry_cost,
                "Carry cost should increase with tenor"
            );
        }
    }

    // -----------------------------------------------------------------------
    // 38. Large contango: carry_pct > 1 (super contango)
    // -----------------------------------------------------------------------
    #[test]
    fn test_super_contango() {
        let input = StorageEconomicsInput {
            spot_price: dec!(20),
            futures_prices: vec![FuturesPrice {
                month: 12,
                price: dec!(35),
                open_interest: None,
            }],
            storage_cost_per_unit_month: dec!(0.10),
            financing_rate: dec!(0.02),
            insurance_cost_pct: None,
            handling_cost: None,
            max_storage_capacity: None,
            current_inventory: None,
            injection_rate: None,
            withdrawal_rate: None,
            seasonal_factors: None,
            commodity_name: "Super Contango Oil".into(),
        };
        let result = analyze_storage_economics(&input).unwrap();

        // actual spread = 35 - 20 = 15
        // full carry = 20*0.02*1 + 0.10*12 = 0.40 + 1.20 = 1.60
        // carry_pct = 15 / 1.60 = 9.375 (super contango!)
        assert!(
            result.carry_pct_of_theoretical > Decimal::ONE,
            "Super contango: pct should exceed 1.0, got {}",
            result.carry_pct_of_theoretical
        );
        assert_eq!(result.inventory_recommendation, "Build");
    }

    // -----------------------------------------------------------------------
    // 39. Convenience yield with spot equals futures
    // -----------------------------------------------------------------------
    #[test]
    fn test_convenience_yield_flat() {
        let input = StorageEconomicsInput {
            spot_price: dec!(100),
            futures_prices: vec![FuturesPrice {
                month: 12,
                price: dec!(100),
                open_interest: None,
            }],
            storage_cost_per_unit_month: dec!(0.50),
            financing_rate: dec!(0.05),
            insurance_cost_pct: None,
            handling_cost: None,
            max_storage_capacity: None,
            current_inventory: None,
            injection_rate: None,
            withdrawal_rate: None,
            seasonal_factors: None,
            commodity_name: "Flat Market".into(),
        };
        let result = analyze_storage_economics(&input).unwrap();

        // F = S => ln(F/S) = 0
        // c = r + storage_annual - 0 = 0.05 + 0.06 = 0.11
        // storage_annual = 0.50 * 12 / 100 = 0.06
        let cy = &result.implied_convenience_yields[0];
        assert_approx(cy.annualized_yield, dec!(0.11), dec!(0.01), "flat CY");
    }

    // -----------------------------------------------------------------------
    // 40. All optional fields None
    // -----------------------------------------------------------------------
    #[test]
    fn test_minimal_input() {
        let input = StorageEconomicsInput {
            spot_price: dec!(100),
            futures_prices: vec![FuturesPrice {
                month: 6,
                price: dec!(103),
                open_interest: None,
            }],
            storage_cost_per_unit_month: dec!(0.50),
            financing_rate: dec!(0.05),
            insurance_cost_pct: None,
            handling_cost: None,
            max_storage_capacity: None,
            current_inventory: None,
            injection_rate: None,
            withdrawal_rate: None,
            seasonal_factors: None,
            commodity_name: "Minimal".into(),
        };
        let result = analyze_storage_economics(&input).unwrap();

        assert_eq!(result.economics_by_tenor.len(), 1);
        assert!(result.seasonal_opportunity.is_none());
        assert!(!result.market_structure.is_empty());
        assert!(!result.inventory_recommendation.is_empty());
    }

    // -----------------------------------------------------------------------
    // 41. Sqrt decimal helper
    // -----------------------------------------------------------------------
    #[test]
    fn test_sqrt_decimal_helper() {
        assert_approx(sqrt_decimal(dec!(4)), dec!(2), dec!(0.0001), "sqrt(4)");
        assert_approx(sqrt_decimal(dec!(9)), dec!(3), dec!(0.0001), "sqrt(9)");
        assert_approx(sqrt_decimal(dec!(2)), dec!(1.41421), dec!(0.001), "sqrt(2)");
        assert_eq!(sqrt_decimal(Decimal::ZERO), Decimal::ZERO);
        assert_eq!(sqrt_decimal(Decimal::ONE), Decimal::ONE);
    }

    // -----------------------------------------------------------------------
    // 42. Exp/ln round trip
    // -----------------------------------------------------------------------
    #[test]
    fn test_exp_ln_round_trip() {
        let x = dec!(1.5);
        let result = ln_decimal(exp_decimal(x));
        assert_approx(result, x, dec!(0.001), "exp/ln round trip");
    }
}
