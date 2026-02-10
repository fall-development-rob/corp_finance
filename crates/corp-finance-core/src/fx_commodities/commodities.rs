use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{with_metadata, ComputationOutput, Money, Rate};
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
    for n in 1..=25u64 {
        term *= reduced / Decimal::from(n);
        sum += term;
    }

    for _ in 0..k {
        sum *= sum;
    }

    sum
}

/// Natural logarithm via Newton's method.
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

/// Iterative multiplication for (base)^n where n is a positive integer.
/// For fractional exponents, falls back to exp(n * ln(base)).
fn decimal_power(base: Decimal, exponent: Decimal) -> Decimal {
    if exponent == Decimal::ZERO {
        return Decimal::ONE;
    }
    if exponent == exponent.trunc() && exponent > Decimal::ZERO {
        let n = exponent.to_string().parse::<u64>().unwrap_or(0);
        if n == 0 {
            return Decimal::ONE;
        }
        let mut result = Decimal::ONE;
        for _ in 0..n {
            result *= base;
        }
        result
    } else {
        exp_decimal(exponent * ln_decimal(base))
    }
}

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// Type of commodity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommodityType {
    /// Oil, natural gas, etc.
    Energy,
    /// Copper, aluminium, etc.
    Metals,
    /// Corn, wheat, soybeans, etc.
    Agriculture,
    /// Gold, silver, platinum, palladium.
    Precious,
}

// ---------------------------------------------------------------------------
// Function 1: price_commodity_forward
// ---------------------------------------------------------------------------

/// Input for pricing a commodity forward.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommodityForwardInput {
    /// Current spot price.
    pub spot_price: Money,
    /// Annualised risk-free rate (decimal).
    pub risk_free_rate: Rate,
    /// Annual storage cost as a percentage of spot (decimal).
    pub storage_cost_rate: Rate,
    /// Annual convenience yield (decimal).
    pub convenience_yield: Rate,
    /// Time to expiry in years.
    pub time_to_expiry: Decimal,
    /// Type of commodity.
    pub commodity_type: CommodityType,
}

/// Output from commodity forward pricing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommodityForwardOutput {
    /// Forward price: F = S * (1 + r + c - y)^T.
    pub forward_price: Money,
    /// Net cost of carry: r + c - y.
    pub cost_of_carry: Decimal,
    /// Basis: F - S.
    pub basis: Money,
    /// Basis percentage: (F - S) / S.
    pub basis_pct: Decimal,
    /// Market structure: "Contango" if F > S, "Backwardation" if F < S, "Flat" otherwise.
    pub contango_backwardation: String,
    /// Implied convenience yield (None at inception; used when re-pricing against market).
    pub implied_convenience_yield: Option<Decimal>,
    /// Approximate annualised roll yield: -(cost_of_carry).
    pub roll_yield: Decimal,
}

/// Price a commodity forward using the discrete cost-of-carry model.
///
/// F = S * (1 + r + c - y)^T
///
/// where r = risk-free rate, c = storage cost rate, y = convenience yield.
/// Uses iterative multiplication for integer T, exp/ln for fractional T.
pub fn price_commodity_forward(
    input: &CommodityForwardInput,
) -> CorpFinanceResult<ComputationOutput<CommodityForwardOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // -- Validation --
    if input.spot_price <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "spot_price".into(),
            reason: "Spot price must be positive".into(),
        });
    }
    if input.storage_cost_rate < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "storage_cost_rate".into(),
            reason: "Storage cost rate must be non-negative".into(),
        });
    }
    if input.convenience_yield < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "convenience_yield".into(),
            reason: "Convenience yield must be non-negative".into(),
        });
    }
    if input.time_to_expiry <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "time_to_expiry".into(),
            reason: "Time to expiry must be positive".into(),
        });
    }

    let s = input.spot_price;
    let r = input.risk_free_rate;
    let c = input.storage_cost_rate;
    let y = input.convenience_yield;
    let t = input.time_to_expiry;

    // -- Net cost of carry --
    let cost_of_carry = r + c - y;

    // -- Forward price: F = S * (1 + r + c - y)^T --
    let base = Decimal::ONE + cost_of_carry;
    if base <= Decimal::ZERO {
        return Err(CorpFinanceError::FinancialImpossibility(format!(
            "Cost of carry base (1 + r + c - y) = {} is non-positive; \
                 cannot compute discrete compounding",
            base
        )));
    }
    let forward_price = s * decimal_power(base, t);

    // -- Basis --
    let basis = forward_price - s;
    let basis_pct = basis / s;

    // -- Contango / Backwardation --
    let contango_backwardation = if basis > Decimal::ZERO {
        "Contango".to_string()
    } else if basis < Decimal::ZERO {
        "Backwardation".to_string()
    } else {
        "Flat".to_string()
    };

    // -- Roll yield approximation --
    // Roll yield is the return from rolling a short-dated future into the next.
    // In contango, roll yield is negative; in backwardation, positive.
    // Approximation: roll_yield ~ -cost_of_carry
    let roll_yield = -cost_of_carry;

    // -- Warnings --
    if y > r {
        warnings.push(format!(
            "Convenience yield ({}) exceeds risk-free rate ({}): strong backwardation signal",
            y, r
        ));
    }

    let annualised_basis = if t > Decimal::ZERO {
        basis_pct / t
    } else {
        Decimal::ZERO
    };
    if annualised_basis.abs() > dec!(0.20) {
        warnings.push(format!(
            "Annualised basis of {:.4} exceeds 20%",
            annualised_basis
        ));
    }

    let output = CommodityForwardOutput {
        forward_price,
        cost_of_carry,
        basis,
        basis_pct,
        contango_backwardation,
        implied_convenience_yield: None,
        roll_yield,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Commodity Forward Pricing via Discrete Cost-of-Carry Model",
        &serde_json::json!({
            "spot_price": s.to_string(),
            "risk_free_rate": r.to_string(),
            "storage_cost_rate": c.to_string(),
            "convenience_yield": y.to_string(),
            "time_to_expiry": t.to_string(),
            "commodity_type": format!("{:?}", input.commodity_type),
        }),
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Function 2: analyze_commodity_curve
// ---------------------------------------------------------------------------

/// A single futures contract in the term structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuturesContract {
    /// Months until expiry.
    pub expiry_months: u32,
    /// Observed futures price.
    pub price: Money,
    /// Open interest (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_interest: Option<u64>,
}

/// Input for commodity term structure analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommodityCurveInput {
    /// Current spot price.
    pub spot_price: Money,
    /// Futures term structure, sorted by expiry.
    pub futures_prices: Vec<FuturesContract>,
    /// Annualised risk-free rate.
    pub risk_free_rate: Rate,
    /// Annual storage cost as a percentage of spot.
    pub storage_cost_rate: Rate,
}

/// A single analysed point on the term structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TermStructurePoint {
    /// Months until expiry.
    pub expiry_months: u32,
    /// Observed futures price.
    pub futures_price: Money,
    /// Basis: futures_price - spot_price.
    pub basis: Money,
    /// Annualised basis percentage: ((F - S) / S) / T.
    pub annualized_basis_pct: Decimal,
    /// Implied convenience yield: y = r + c - ln(F/S) / T.
    pub implied_convenience_yield: Decimal,
}

/// A calendar spread between two futures contracts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarSpread {
    /// Near-month expiry.
    pub near_month: u32,
    /// Far-month expiry.
    pub far_month: u32,
    /// Spread: far_price - near_price.
    pub spread: Money,
    /// Annualised spread percentage.
    pub annualized_spread_pct: Decimal,
}

/// Output from commodity term structure analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommodityCurveOutput {
    /// Analysed points on the term structure.
    pub term_structure: Vec<TermStructurePoint>,
    /// Overall curve shape: "Contango", "Backwardation", or "Mixed".
    pub curve_shape: String,
    /// Implied convenience yields from each futures price.
    pub implied_convenience_yields: Vec<Decimal>,
    /// Calendar spreads between consecutive contracts.
    pub calendar_spreads: Vec<CalendarSpread>,
    /// Average annualised roll yield across consecutive contracts.
    pub avg_roll_yield: Decimal,
}

/// Analyse a commodity futures term structure to extract implied convenience
/// yields, calendar spreads, and determine contango vs backwardation.
///
/// Implied convenience yield from each futures price:
///   y = r + c - ln(F/S) / T
///
/// Calendar spread between consecutive contracts:
///   spread = far_price - near_price
///   annualised spread = (spread / near_price) / (delta_T in years)
///
/// Roll yield approximation for each adjacent pair:
///   roll = (near - far) / near, annualised by dividing by time gap
pub fn analyze_commodity_curve(
    input: &CommodityCurveInput,
) -> CorpFinanceResult<ComputationOutput<CommodityCurveOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // -- Validation --
    if input.spot_price <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "spot_price".into(),
            reason: "Spot price must be positive".into(),
        });
    }
    if input.futures_prices.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one futures contract is required".into(),
        ));
    }
    if input.storage_cost_rate < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "storage_cost_rate".into(),
            reason: "Storage cost rate must be non-negative".into(),
        });
    }

    let twelve = Decimal::from(12);
    let s = input.spot_price;
    let r = input.risk_free_rate;
    let c = input.storage_cost_rate;

    let mut term_structure = Vec::with_capacity(input.futures_prices.len());
    let mut implied_convenience_yields = Vec::with_capacity(input.futures_prices.len());
    let mut contango_count: usize = 0;
    let mut backwardation_count: usize = 0;

    for contract in &input.futures_prices {
        if contract.expiry_months == 0 {
            return Err(CorpFinanceError::InvalidInput {
                field: "expiry_months".into(),
                reason: "Expiry months must be positive".into(),
            });
        }
        if contract.price <= Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "price".into(),
                reason: "Futures price must be positive".into(),
            });
        }

        let t_years = Decimal::from(contract.expiry_months) / twelve;
        let basis = contract.price - s;

        let annualized_basis_pct = if t_years > Decimal::ZERO {
            (basis / s) / t_years
        } else {
            Decimal::ZERO
        };

        // Implied convenience yield: y = r + c - ln(F/S) / T
        let ratio = contract.price / s;
        let implied_cy = if t_years > Decimal::ZERO {
            r + c - ln_decimal(ratio) / t_years
        } else {
            Decimal::ZERO
        };

        implied_convenience_yields.push(implied_cy);

        if basis > Decimal::ZERO {
            contango_count += 1;
        } else if basis < Decimal::ZERO {
            backwardation_count += 1;
        }

        term_structure.push(TermStructurePoint {
            expiry_months: contract.expiry_months,
            futures_price: contract.price,
            basis,
            annualized_basis_pct,
            implied_convenience_yield: implied_cy,
        });
    }

    // -- Calendar spreads --
    let mut calendar_spreads = Vec::new();
    let mut roll_yield_sum = Decimal::ZERO;
    let mut roll_count: u32 = 0;

    for i in 0..term_structure.len().saturating_sub(1) {
        let near = &term_structure[i];
        let far = &term_structure[i + 1];

        let spread = far.futures_price - near.futures_price;
        let delta_months = far.expiry_months.saturating_sub(near.expiry_months);
        let delta_years = Decimal::from(delta_months) / twelve;

        let annualized_spread_pct =
            if delta_years > Decimal::ZERO && near.futures_price > Decimal::ZERO {
                (spread / near.futures_price) / delta_years
            } else {
                Decimal::ZERO
            };

        // Roll yield: (near - far) / near, annualised
        let roll_yield_per_period =
            if near.futures_price > Decimal::ZERO && delta_years > Decimal::ZERO {
                ((near.futures_price - far.futures_price) / near.futures_price) / delta_years
            } else {
                Decimal::ZERO
            };
        roll_yield_sum += roll_yield_per_period;
        roll_count += 1;

        calendar_spreads.push(CalendarSpread {
            near_month: near.expiry_months,
            far_month: far.expiry_months,
            spread,
            annualized_spread_pct,
        });
    }

    let avg_roll_yield = if roll_count > 0 {
        roll_yield_sum / Decimal::from(roll_count)
    } else {
        Decimal::ZERO
    };

    // -- Curve shape --
    let curve_shape = if contango_count > 0 && backwardation_count > 0 {
        "Mixed".to_string()
    } else if contango_count > 0 {
        "Contango".to_string()
    } else if backwardation_count > 0 {
        "Backwardation".to_string()
    } else {
        "Flat".to_string()
    };

    // -- Warnings --
    if contango_count > 0 && backwardation_count > 0 {
        warnings
            .push("Mixed term structure: some contracts in contango, some in backwardation".into());
    }

    for (i, cy) in implied_convenience_yields.iter().enumerate() {
        if *cy > r && *cy > Decimal::ZERO {
            warnings.push(format!(
                "Contract {} ({}m): implied convenience yield {:.4} exceeds risk-free rate {:.4}",
                i + 1,
                input.futures_prices[i].expiry_months,
                cy,
                r
            ));
        }
    }

    let output = CommodityCurveOutput {
        term_structure,
        curve_shape,
        implied_convenience_yields,
        calendar_spreads,
        avg_roll_yield,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Commodity Futures Term Structure Analysis",
        &serde_json::json!({
            "spot_price": s.to_string(),
            "risk_free_rate": r.to_string(),
            "storage_cost_rate": c.to_string(),
            "num_contracts": input.futures_prices.len(),
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

    fn tol() -> Decimal {
        dec!(0.01)
    }

    fn tight_tol() -> Decimal {
        dec!(0.001)
    }

    fn assert_approx(actual: Decimal, expected: Decimal, tolerance: Decimal, label: &str) {
        let diff = (actual - expected).abs();
        assert!(
            diff <= tolerance,
            "{label}: expected ~{expected}, got {actual} (diff={diff}, tol={tolerance})"
        );
    }

    // -----------------------------------------------------------------------
    // 1. Gold forward (low storage, no convenience yield)
    // -----------------------------------------------------------------------
    #[test]
    fn test_gold_forward() {
        // Gold: S=1900, r=5%, c=0.5%, y=0%, T=1
        // F = 1900 * (1 + 0.05 + 0.005 - 0)^1 = 1900 * 1.055 = 2004.50
        let input = CommodityForwardInput {
            spot_price: dec!(1900),
            risk_free_rate: dec!(0.05),
            storage_cost_rate: dec!(0.005),
            convenience_yield: Decimal::ZERO,
            time_to_expiry: Decimal::ONE,
            commodity_type: CommodityType::Precious,
        };
        let result = price_commodity_forward(&input).unwrap();
        let out = &result.result;

        assert_approx(out.forward_price, dec!(2004.50), tol(), "gold forward");
        assert_approx(out.cost_of_carry, dec!(0.055), tight_tol(), "gold carry");
        assert_eq!(out.contango_backwardation, "Contango");
    }

    // -----------------------------------------------------------------------
    // 2. Oil forward (storage + convenience yield)
    // -----------------------------------------------------------------------
    #[test]
    fn test_oil_forward() {
        // Oil: S=80, r=5%, c=3%, y=2%, T=0.5
        // carry = 0.05 + 0.03 - 0.02 = 0.06
        // F = 80 * (1.06)^0.5 ~ 80 * 1.02956 ~ 82.365
        let input = CommodityForwardInput {
            spot_price: dec!(80),
            risk_free_rate: dec!(0.05),
            storage_cost_rate: dec!(0.03),
            convenience_yield: dec!(0.02),
            time_to_expiry: dec!(0.5),
            commodity_type: CommodityType::Energy,
        };
        let result = price_commodity_forward(&input).unwrap();
        let out = &result.result;

        assert_approx(out.cost_of_carry, dec!(0.06), tight_tol(), "oil carry");
        // (1.06)^0.5 ~ 1.02956
        let expected_fwd = dec!(80) * dec!(1.02956);
        assert_approx(out.forward_price, expected_fwd, dec!(0.1), "oil forward");
        assert_eq!(out.contango_backwardation, "Contango");
    }

    // -----------------------------------------------------------------------
    // 3. Backwardation: high convenience yield
    // -----------------------------------------------------------------------
    #[test]
    fn test_backwardation_high_convenience_yield() {
        // S=80, r=3%, c=1%, y=10%, T=1
        // carry = 0.03 + 0.01 - 0.10 = -0.06
        // F = 80 * (1 - 0.06)^1 = 80 * 0.94 = 75.20
        let input = CommodityForwardInput {
            spot_price: dec!(80),
            risk_free_rate: dec!(0.03),
            storage_cost_rate: dec!(0.01),
            convenience_yield: dec!(0.10),
            time_to_expiry: Decimal::ONE,
            commodity_type: CommodityType::Energy,
        };
        let result = price_commodity_forward(&input).unwrap();
        let out = &result.result;

        assert_approx(out.forward_price, dec!(75.20), tol(), "backwardation fwd");
        assert_eq!(out.contango_backwardation, "Backwardation");
        assert!(out.basis < Decimal::ZERO);
    }

    // -----------------------------------------------------------------------
    // 4. Contango detection
    // -----------------------------------------------------------------------
    #[test]
    fn test_contango_detection() {
        let input = CommodityForwardInput {
            spot_price: dec!(100),
            risk_free_rate: dec!(0.08),
            storage_cost_rate: dec!(0.02),
            convenience_yield: dec!(0.01),
            time_to_expiry: Decimal::ONE,
            commodity_type: CommodityType::Metals,
        };
        let result = price_commodity_forward(&input).unwrap();
        assert_eq!(result.result.contango_backwardation, "Contango");
        assert!(result.result.forward_price > dec!(100));
    }

    // -----------------------------------------------------------------------
    // 5. Flat market: carry = 0
    // -----------------------------------------------------------------------
    #[test]
    fn test_flat_market() {
        // carry = 0.05 + 0.01 - 0.06 = 0
        let input = CommodityForwardInput {
            spot_price: dec!(100),
            risk_free_rate: dec!(0.05),
            storage_cost_rate: dec!(0.01),
            convenience_yield: dec!(0.06),
            time_to_expiry: Decimal::ONE,
            commodity_type: CommodityType::Agriculture,
        };
        let result = price_commodity_forward(&input).unwrap();
        let out = &result.result;

        assert_approx(out.forward_price, dec!(100), tol(), "flat market");
        assert_eq!(out.contango_backwardation, "Flat");
    }

    // -----------------------------------------------------------------------
    // 6. Basis percentage
    // -----------------------------------------------------------------------
    #[test]
    fn test_basis_percentage() {
        let input = CommodityForwardInput {
            spot_price: dec!(100),
            risk_free_rate: dec!(0.05),
            storage_cost_rate: dec!(0.01),
            convenience_yield: Decimal::ZERO,
            time_to_expiry: Decimal::ONE,
            commodity_type: CommodityType::Metals,
        };
        let result = price_commodity_forward(&input).unwrap();
        let out = &result.result;

        let expected_basis_pct = (out.forward_price - dec!(100)) / dec!(100);
        assert_approx(out.basis_pct, expected_basis_pct, dec!(0.0001), "basis pct");
    }

    // -----------------------------------------------------------------------
    // 7. Roll yield sign
    // -----------------------------------------------------------------------
    #[test]
    fn test_roll_yield() {
        // In contango, roll yield is negative (cost of rolling)
        let input = CommodityForwardInput {
            spot_price: dec!(100),
            risk_free_rate: dec!(0.05),
            storage_cost_rate: dec!(0.02),
            convenience_yield: Decimal::ZERO,
            time_to_expiry: Decimal::ONE,
            commodity_type: CommodityType::Energy,
        };
        let result = price_commodity_forward(&input).unwrap();
        assert!(
            result.result.roll_yield < Decimal::ZERO,
            "Contango => negative roll yield"
        );

        // In backwardation, roll yield is positive
        let input_back = CommodityForwardInput {
            convenience_yield: dec!(0.15),
            ..input
        };
        let result_back = price_commodity_forward(&input_back).unwrap();
        assert!(
            result_back.result.roll_yield > Decimal::ZERO,
            "Backwardation => positive roll yield"
        );
    }

    // -----------------------------------------------------------------------
    // 8. Term structure: contango curve
    // -----------------------------------------------------------------------
    #[test]
    fn test_term_structure_contango() {
        let input = CommodityCurveInput {
            spot_price: dec!(100),
            futures_prices: vec![
                FuturesContract {
                    expiry_months: 3,
                    price: dec!(101.5),
                    open_interest: Some(50000),
                },
                FuturesContract {
                    expiry_months: 6,
                    price: dec!(103),
                    open_interest: Some(30000),
                },
                FuturesContract {
                    expiry_months: 12,
                    price: dec!(106),
                    open_interest: Some(20000),
                },
            ],
            risk_free_rate: dec!(0.05),
            storage_cost_rate: dec!(0.01),
        };
        let result = analyze_commodity_curve(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.curve_shape, "Contango");
        assert_eq!(out.term_structure.len(), 3);
        for pt in &out.term_structure {
            assert!(pt.basis > Decimal::ZERO);
        }
        // Basis should increase with time
        assert!(out.term_structure[2].basis > out.term_structure[0].basis);
    }

    // -----------------------------------------------------------------------
    // 9. Term structure: backwardation curve
    // -----------------------------------------------------------------------
    #[test]
    fn test_term_structure_backwardation() {
        let input = CommodityCurveInput {
            spot_price: dec!(100),
            futures_prices: vec![
                FuturesContract {
                    expiry_months: 3,
                    price: dec!(98),
                    open_interest: None,
                },
                FuturesContract {
                    expiry_months: 6,
                    price: dec!(96),
                    open_interest: None,
                },
                FuturesContract {
                    expiry_months: 12,
                    price: dec!(93),
                    open_interest: None,
                },
            ],
            risk_free_rate: dec!(0.03),
            storage_cost_rate: dec!(0.01),
        };
        let result = analyze_commodity_curve(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.curve_shape, "Backwardation");
        for pt in &out.term_structure {
            assert!(pt.basis < Decimal::ZERO);
        }
    }

    // -----------------------------------------------------------------------
    // 10. Calendar spreads
    // -----------------------------------------------------------------------
    #[test]
    fn test_calendar_spreads() {
        let input = CommodityCurveInput {
            spot_price: dec!(100),
            futures_prices: vec![
                FuturesContract {
                    expiry_months: 3,
                    price: dec!(102),
                    open_interest: None,
                },
                FuturesContract {
                    expiry_months: 6,
                    price: dec!(104),
                    open_interest: None,
                },
                FuturesContract {
                    expiry_months: 12,
                    price: dec!(108),
                    open_interest: None,
                },
            ],
            risk_free_rate: dec!(0.05),
            storage_cost_rate: dec!(0.01),
        };
        let result = analyze_commodity_curve(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.calendar_spreads.len(), 2);

        // First spread: 6m - 3m
        let sp1 = &out.calendar_spreads[0];
        assert_eq!(sp1.near_month, 3);
        assert_eq!(sp1.far_month, 6);
        assert_approx(sp1.spread, dec!(2), dec!(0.001), "spread 3-6m");

        // Second spread: 12m - 6m
        let sp2 = &out.calendar_spreads[1];
        assert_eq!(sp2.near_month, 6);
        assert_eq!(sp2.far_month, 12);
        assert_approx(sp2.spread, dec!(4), dec!(0.001), "spread 6-12m");
    }

    // -----------------------------------------------------------------------
    // 11. Implied convenience yield extraction
    // -----------------------------------------------------------------------
    #[test]
    fn test_implied_convenience_yield() {
        // S=100, F=100 at T=1, r=5%, c=1%
        // y = r + c - ln(F/S)/T = 0.05 + 0.01 - ln(1.0)/1 = 0.06 - 0 = 0.06
        let input = CommodityCurveInput {
            spot_price: dec!(100),
            futures_prices: vec![FuturesContract {
                expiry_months: 12,
                price: dec!(100),
                open_interest: None,
            }],
            risk_free_rate: dec!(0.05),
            storage_cost_rate: dec!(0.01),
        };
        let result = analyze_commodity_curve(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.implied_convenience_yields.len(), 1);
        assert_approx(
            out.implied_convenience_yields[0],
            dec!(0.06),
            tol(),
            "implied CY",
        );
    }

    // -----------------------------------------------------------------------
    // 12. Average roll yield
    // -----------------------------------------------------------------------
    #[test]
    fn test_avg_roll_yield() {
        let input = CommodityCurveInput {
            spot_price: dec!(100),
            futures_prices: vec![
                FuturesContract {
                    expiry_months: 3,
                    price: dec!(102),
                    open_interest: None,
                },
                FuturesContract {
                    expiry_months: 6,
                    price: dec!(104),
                    open_interest: None,
                },
            ],
            risk_free_rate: dec!(0.05),
            storage_cost_rate: dec!(0.01),
        };
        let result = analyze_commodity_curve(&input).unwrap();
        let out = &result.result;

        // Roll yield: (near - far) / near / delta_T
        // = (102 - 104) / 102 / 0.25 = -0.019608 / 0.25 = -0.078431
        let expected_roll = (dec!(102) - dec!(104)) / dec!(102) / dec!(0.25);
        assert_approx(out.avg_roll_yield, expected_roll, tol(), "avg roll yield");
        assert!(
            out.avg_roll_yield < Decimal::ZERO,
            "Contango => negative roll"
        );
    }

    // -----------------------------------------------------------------------
    // 13. Mixed curve shape
    // -----------------------------------------------------------------------
    #[test]
    fn test_mixed_curve() {
        let input = CommodityCurveInput {
            spot_price: dec!(100),
            futures_prices: vec![
                FuturesContract {
                    expiry_months: 3,
                    price: dec!(101),
                    open_interest: None,
                },
                FuturesContract {
                    expiry_months: 6,
                    price: dec!(99),
                    open_interest: None,
                },
            ],
            risk_free_rate: dec!(0.05),
            storage_cost_rate: dec!(0.01),
        };
        let result = analyze_commodity_curve(&input).unwrap();
        assert_eq!(result.result.curve_shape, "Mixed");
        assert!(result.warnings.iter().any(|w| w.contains("Mixed")));
    }

    // -----------------------------------------------------------------------
    // 14. Validation: spot must be positive
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_spot_positive() {
        let input = CommodityForwardInput {
            spot_price: Decimal::ZERO,
            risk_free_rate: dec!(0.05),
            storage_cost_rate: dec!(0.01),
            convenience_yield: Decimal::ZERO,
            time_to_expiry: Decimal::ONE,
            commodity_type: CommodityType::Energy,
        };
        let err = price_commodity_forward(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "spot_price");
            }
            e => panic!("Expected InvalidInput for spot_price, got {e:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // 15. Validation: storage cost non-negative
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_storage_cost_nonneg() {
        let input = CommodityForwardInput {
            spot_price: dec!(100),
            risk_free_rate: dec!(0.05),
            storage_cost_rate: dec!(-0.01),
            convenience_yield: Decimal::ZERO,
            time_to_expiry: Decimal::ONE,
            commodity_type: CommodityType::Energy,
        };
        let err = price_commodity_forward(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "storage_cost_rate");
            }
            e => panic!("Expected InvalidInput for storage_cost_rate, got {e:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // 16. Validation: convenience yield non-negative
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_convenience_yield_nonneg() {
        let input = CommodityForwardInput {
            spot_price: dec!(100),
            risk_free_rate: dec!(0.05),
            storage_cost_rate: dec!(0.01),
            convenience_yield: dec!(-0.01),
            time_to_expiry: Decimal::ONE,
            commodity_type: CommodityType::Energy,
        };
        let err = price_commodity_forward(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "convenience_yield");
            }
            e => panic!("Expected InvalidInput for convenience_yield, got {e:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // 17. Validation: time must be positive
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_time_positive() {
        let input = CommodityForwardInput {
            spot_price: dec!(100),
            risk_free_rate: dec!(0.05),
            storage_cost_rate: dec!(0.01),
            convenience_yield: Decimal::ZERO,
            time_to_expiry: Decimal::ZERO,
            commodity_type: CommodityType::Energy,
        };
        let err = price_commodity_forward(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "time_to_expiry");
            }
            e => panic!("Expected InvalidInput for time_to_expiry, got {e:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // 18. Curve analysis: empty contracts error
    // -----------------------------------------------------------------------
    #[test]
    fn test_curve_empty_contracts_error() {
        let input = CommodityCurveInput {
            spot_price: dec!(100),
            futures_prices: vec![],
            risk_free_rate: dec!(0.05),
            storage_cost_rate: dec!(0.01),
        };
        let err = analyze_commodity_curve(&input).unwrap_err();
        match err {
            CorpFinanceError::InsufficientData(_) => {}
            e => panic!("Expected InsufficientData, got {e:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // 19. Convenience yield warning
    // -----------------------------------------------------------------------
    #[test]
    fn test_convenience_yield_warning() {
        let input = CommodityForwardInput {
            spot_price: dec!(80),
            risk_free_rate: dec!(0.03),
            storage_cost_rate: dec!(0.01),
            convenience_yield: dec!(0.10),
            time_to_expiry: Decimal::ONE,
            commodity_type: CommodityType::Energy,
        };
        let result = price_commodity_forward(&input).unwrap();
        assert!(
            result
                .warnings
                .iter()
                .any(|w| w.contains("Convenience yield")),
            "Should warn when CY > r_f"
        );
    }

    // -----------------------------------------------------------------------
    // 20. Agriculture forward: multi-year
    // -----------------------------------------------------------------------
    #[test]
    fn test_agriculture_forward_multi_year() {
        // S=500, r=4%, c=2%, y=0%, T=3
        // F = 500 * (1.06)^3 = 500 * 1.191016 = 595.508
        let input = CommodityForwardInput {
            spot_price: dec!(500),
            risk_free_rate: dec!(0.04),
            storage_cost_rate: dec!(0.02),
            convenience_yield: Decimal::ZERO,
            time_to_expiry: dec!(3),
            commodity_type: CommodityType::Agriculture,
        };
        let result = price_commodity_forward(&input).unwrap();
        let out = &result.result;

        // (1.06)^3 = 1.06 * 1.06 * 1.06 = 1.191016
        let expected = dec!(500) * dec!(1.06) * dec!(1.06) * dec!(1.06);
        assert_approx(out.forward_price, expected, tol(), "agri multi-year");
    }

    // -----------------------------------------------------------------------
    // 21. Metadata populated
    // -----------------------------------------------------------------------
    #[test]
    fn test_metadata_populated() {
        let input = CommodityForwardInput {
            spot_price: dec!(100),
            risk_free_rate: dec!(0.05),
            storage_cost_rate: dec!(0.01),
            convenience_yield: Decimal::ZERO,
            time_to_expiry: Decimal::ONE,
            commodity_type: CommodityType::Precious,
        };
        let result = price_commodity_forward(&input).unwrap();

        assert!(result.methodology.contains("Commodity Forward"));
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
        assert!(!result.metadata.version.is_empty());
    }

    // -----------------------------------------------------------------------
    // 22. Curve analysis metadata
    // -----------------------------------------------------------------------
    #[test]
    fn test_curve_metadata() {
        let input = CommodityCurveInput {
            spot_price: dec!(100),
            futures_prices: vec![FuturesContract {
                expiry_months: 6,
                price: dec!(103),
                open_interest: None,
            }],
            risk_free_rate: dec!(0.05),
            storage_cost_rate: dec!(0.01),
        };
        let result = analyze_commodity_curve(&input).unwrap();
        assert!(result.methodology.contains("Term Structure"));
    }

    // -----------------------------------------------------------------------
    // 23. Single contract curve analysis
    // -----------------------------------------------------------------------
    #[test]
    fn test_single_contract_curve() {
        let input = CommodityCurveInput {
            spot_price: dec!(100),
            futures_prices: vec![FuturesContract {
                expiry_months: 6,
                price: dec!(103),
                open_interest: Some(100000),
            }],
            risk_free_rate: dec!(0.05),
            storage_cost_rate: dec!(0.01),
        };
        let result = analyze_commodity_curve(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.term_structure.len(), 1);
        assert!(out.calendar_spreads.is_empty());
        assert_eq!(out.avg_roll_yield, Decimal::ZERO);
        assert_eq!(out.curve_shape, "Contango");
    }
}
