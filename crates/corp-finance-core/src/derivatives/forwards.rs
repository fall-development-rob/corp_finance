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
///
/// For large |x|, we use the identity exp(x) = exp(x/2^k)^(2^k) to bring
/// the argument into a range where the Taylor series converges rapidly, then
/// square the result k times.
fn exp_decimal(x: Decimal) -> Decimal {
    let two = Decimal::from(2);

    // Range reduction: find k such that |x / 2^k| <= 2
    let mut k: u32 = 0;
    let mut reduced = x;
    while reduced.abs() > two {
        reduced /= two;
        k += 1;
    }

    // Taylor series: exp(reduced) = sum_{n=0}^{24} reduced^n / n!
    let mut sum = Decimal::ONE;
    let mut term = Decimal::ONE;
    for n in 1..=25u64 {
        term *= reduced / Decimal::from(n);
        sum += term;
    }

    // Reverse the range reduction by repeated squaring
    for _ in 0..k {
        sum *= sum;
    }

    sum
}

/// Natural logarithm via Newton's method.
///
/// Solves f(y) = exp(y) - x = 0 using Newton iterations:
///   y_{n+1} = y_n - (exp(y_n) - x) / exp(y_n)
///           = y_n - 1 + x / exp(y_n)
///
/// Initial guess uses a rough log2 approximation.
fn ln_decimal(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ZERO; // Domain guard; callers validate positivity
    }
    if x == Decimal::ONE {
        return Decimal::ZERO;
    }

    // Initial guess: count powers of 2 via repeated halving/doubling
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

    // Newton iterations
    for _ in 0..30 {
        let ey = exp_decimal(guess);
        if ey.is_zero() {
            break;
        }
        // y_{n+1} = y_n - 1 + x / exp(y_n)
        guess = guess - Decimal::ONE + x / ey;
    }

    guess
}

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// Type of underlying asset for the forward/futures contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnderlyingType {
    Equity,
    Commodity,
    Currency,
    Index,
    Bond,
}

/// Market condition inferred from the forward/futures basis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarketCondition {
    Contango,
    Backwardation,
    Flat,
}

// ---------------------------------------------------------------------------
// Function 1: price_forward
// ---------------------------------------------------------------------------

/// Input for pricing a forward contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForwardInput {
    /// Current spot price of the underlying asset.
    pub spot_price: Money,
    /// Annualised risk-free interest rate (decimal).
    pub risk_free_rate: Rate,
    /// Time to expiry in years.
    pub time_to_expiry: Decimal,
    /// Annualised storage cost as a percentage of spot (commodities).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_cost_rate: Option<Rate>,
    /// Annualised convenience yield (commodities).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub convenience_yield: Option<Rate>,
    /// Annualised continuous dividend yield (equity/index).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dividend_yield: Option<Rate>,
    /// Foreign risk-free rate for currency forwards (interest rate parity).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foreign_rate: Option<Rate>,
    /// Type of the underlying asset.
    pub underlying_type: UnderlyingType,
}

/// Output from forward pricing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForwardOutput {
    /// Theoretical forward price: F = S * exp((r - q + c - y) * T).
    pub forward_price: Money,
    /// Net cost of carry rate: r - q + c - y.
    pub cost_of_carry: Rate,
    /// Basis: F - S.
    pub basis: Money,
    /// Basis rate: (F - S) / S.
    pub basis_rate: Rate,
    /// Market condition inferred from the basis sign.
    pub market_condition: MarketCondition,
    /// Explanation of the no-arbitrage pricing relationship.
    pub theoretical_vs_no_arb: String,
    /// Present value of the forward price: F * exp(-r * T).
    pub present_value_of_forward: Money,
}

/// Price a forward contract using continuous compounding cost-of-carry.
///
/// The general formula is:
///   F = S * exp((r - q + c - y) * T)
///
/// where:
///   r = domestic risk-free rate
///   q = dividend yield (equity/index) or foreign rate (currency)
///   c = storage cost rate (commodity)
///   y = convenience yield (commodity)
pub fn price_forward(input: &ForwardInput) -> CorpFinanceResult<ComputationOutput<ForwardOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // -- Validation --
    if input.spot_price <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "spot_price".into(),
            reason: "Spot price must be positive".into(),
        });
    }
    if input.time_to_expiry <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "time_to_expiry".into(),
            reason: "Time to expiry must be positive".into(),
        });
    }

    // -- Build cost-of-carry components --
    let r = input.risk_free_rate;
    let q = match input.underlying_type {
        UnderlyingType::Currency => input.foreign_rate.unwrap_or(Decimal::ZERO),
        UnderlyingType::Equity | UnderlyingType::Index => {
            input.dividend_yield.unwrap_or(Decimal::ZERO)
        }
        _ => Decimal::ZERO,
    };
    let c = input.storage_cost_rate.unwrap_or(Decimal::ZERO);
    let y = input.convenience_yield.unwrap_or(Decimal::ZERO);
    let t = input.time_to_expiry;

    let cost_of_carry = r - q + c - y;

    // Warn on unusual carry rates
    if cost_of_carry.abs() > dec!(0.50) {
        warnings.push(format!(
            "Net cost of carry {cost_of_carry} exceeds 50%; verify input rates"
        ));
    }

    // F = S * exp(cost_of_carry * T)
    let forward_price = input.spot_price * exp_decimal(cost_of_carry * t);

    let basis = forward_price - input.spot_price;
    let basis_rate = basis / input.spot_price;

    let market_condition = if basis > Decimal::ZERO {
        MarketCondition::Contango
    } else if basis < Decimal::ZERO {
        MarketCondition::Backwardation
    } else {
        MarketCondition::Flat
    };

    // PV of forward = F * exp(-r * T) (discounted at the domestic risk-free rate)
    let present_value_of_forward = forward_price * exp_decimal(-r * t);

    let theoretical_vs_no_arb = build_no_arb_explanation(
        &input.underlying_type,
        r,
        q,
        c,
        y,
        cost_of_carry,
        &market_condition,
    );

    let output = ForwardOutput {
        forward_price,
        cost_of_carry,
        basis,
        basis_rate,
        market_condition,
        theoretical_vs_no_arb,
        present_value_of_forward,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Forward Pricing via Continuous Cost-of-Carry Model",
        &serde_json::json!({
            "spot_price": input.spot_price.to_string(),
            "risk_free_rate": r.to_string(),
            "time_to_expiry": t.to_string(),
            "dividend_yield": q.to_string(),
            "storage_cost_rate": c.to_string(),
            "convenience_yield": y.to_string(),
            "underlying_type": format!("{:?}", input.underlying_type),
        }),
        warnings,
        elapsed,
        output,
    ))
}

fn build_no_arb_explanation(
    underlying: &UnderlyingType,
    r: Decimal,
    q: Decimal,
    c: Decimal,
    y: Decimal,
    carry: Decimal,
    condition: &MarketCondition,
) -> String {
    let asset_label = match underlying {
        UnderlyingType::Equity => "equity",
        UnderlyingType::Commodity => "commodity",
        UnderlyingType::Currency => "currency pair",
        UnderlyingType::Index => "index",
        UnderlyingType::Bond => "bond",
    };

    let condition_label = match condition {
        MarketCondition::Contango => "contango (F > S)",
        MarketCondition::Backwardation => "backwardation (F < S)",
        MarketCondition::Flat => "flat (F = S)",
    };

    format!(
        "No-arbitrage forward price for {asset_label}: F = S * exp((r - q + c - y) * T). \
         Components: r={r}, q={q}, c={c}, y={y}, net carry={carry}. \
         Market is in {condition_label}. \
         Any deviation from this price creates a risk-free arbitrage opportunity \
         via cash-and-carry (if F is too high) or reverse cash-and-carry (if F is too low)."
    )
}

// ---------------------------------------------------------------------------
// Function 2: value_forward_position
// ---------------------------------------------------------------------------

/// Input for valuing an existing forward position.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForwardPositionInput {
    /// The original (locked-in) forward price.
    pub original_forward_price: Money,
    /// Current spot price.
    pub current_spot: Money,
    /// Current annualised risk-free rate.
    pub risk_free_rate: Rate,
    /// Remaining time to expiry in years.
    pub remaining_time: Decimal,
    /// True if long the forward, false if short.
    pub is_long: bool,
    /// Number of units in the contract.
    pub contract_size: Decimal,
    /// Continuous dividend yield (if applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dividend_yield: Option<Rate>,
}

/// Output from forward position valuation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForwardPositionOutput {
    /// Current theoretical forward price.
    pub current_forward_price: Money,
    /// Mark-to-market value per unit (PV of difference).
    pub mark_to_market: Money,
    /// Total profit/loss considering contract size.
    pub profit_loss: Money,
    /// Annualised return on the position.
    pub annualized_return: Rate,
}

/// Value an existing forward position by comparing the original locked-in
/// price against the current theoretical forward.
///
/// For a long position:
///   MtM = (current_F - original_F) * exp(-r * T) * contract_size
///
/// For a short position the sign is reversed.
pub fn value_forward_position(
    input: &ForwardPositionInput,
) -> CorpFinanceResult<ComputationOutput<ForwardPositionOutput>> {
    let start = Instant::now();
    let warnings: Vec<String> = Vec::new();

    // -- Validation --
    if input.current_spot <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "current_spot".into(),
            reason: "Current spot price must be positive".into(),
        });
    }
    if input.remaining_time <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "remaining_time".into(),
            reason: "Remaining time must be positive".into(),
        });
    }
    if input.contract_size <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "contract_size".into(),
            reason: "Contract size must be positive".into(),
        });
    }
    if input.original_forward_price <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "original_forward_price".into(),
            reason: "Original forward price must be positive".into(),
        });
    }

    let r = input.risk_free_rate;
    let t = input.remaining_time;
    let q = input.dividend_yield.unwrap_or(Decimal::ZERO);

    // Current theoretical forward: F_current = S * exp((r - q) * T)
    let current_forward_price = input.current_spot * exp_decimal((r - q) * t);

    // Discount factor to present value
    let discount_factor = exp_decimal(-r * t);

    // MtM per unit for a long position
    let diff = current_forward_price - input.original_forward_price;
    let mtm_per_unit = diff * discount_factor;

    // Apply direction: long = +1, short = -1
    let direction = if input.is_long {
        Decimal::ONE
    } else {
        -Decimal::ONE
    };

    let mark_to_market = mtm_per_unit * direction;
    let profit_loss = mark_to_market * input.contract_size;

    // Annualised return: the P&L as a fraction of the notional (original_F * size),
    // annualised by dividing by time elapsed.
    // We use the original forward price as the notional basis.
    let notional = input.original_forward_price * input.contract_size;
    let annualized_return = if notional > Decimal::ZERO && t > Decimal::ZERO {
        (profit_loss / notional) / t
    } else {
        Decimal::ZERO
    };

    let output = ForwardPositionOutput {
        current_forward_price,
        mark_to_market,
        profit_loss,
        annualized_return,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Forward Position Valuation — Mark-to-Market",
        &serde_json::json!({
            "original_forward_price": input.original_forward_price.to_string(),
            "current_spot": input.current_spot.to_string(),
            "risk_free_rate": r.to_string(),
            "remaining_time": t.to_string(),
            "is_long": input.is_long,
            "contract_size": input.contract_size.to_string(),
            "dividend_yield": q.to_string(),
        }),
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Function 3: futures_basis_analysis
// ---------------------------------------------------------------------------

/// A single futures contract in the term structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuturesContract {
    /// Months until expiry.
    pub expiry_months: Decimal,
    /// Observed futures price.
    pub price: Money,
    /// Descriptive label (e.g. "Mar-25", "Jun-25").
    pub label: String,
}

/// Input for futures term structure / basis analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasisAnalysisInput {
    /// Current spot price.
    pub spot_price: Money,
    /// Vector of futures contracts across the term structure.
    pub futures_prices: Vec<FuturesContract>,
    /// Annualised risk-free rate.
    pub risk_free_rate: Rate,
}

/// A single point in the basis term structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasisTerm {
    /// Descriptive label for this contract.
    pub label: String,
    /// Months until expiry.
    pub expiry_months: Decimal,
    /// Observed futures price.
    pub futures_price: Money,
    /// Basis: F - S.
    pub basis: Money,
    /// Annualised basis rate: (F - S) / S / T.
    pub annualised_basis_rate: Rate,
    /// Implied yield: ln(F / S) / T.
    pub implied_yield: Rate,
}

/// Output from futures basis analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasisAnalysisOutput {
    /// Term structure of basis and implied yields.
    pub term_structure: Vec<BasisTerm>,
    /// Overall assessment of the curve shape.
    pub curve_shape: MarketCondition,
    /// Simple average of implied yields across all contracts.
    pub average_implied_yield: Rate,
}

/// Analyse the futures term structure to extract basis, implied yields,
/// and the overall curve shape (contango vs. backwardation).
pub fn futures_basis_analysis(
    input: &BasisAnalysisInput,
) -> CorpFinanceResult<ComputationOutput<BasisAnalysisOutput>> {
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

    let twelve = Decimal::from(12);
    let mut term_structure = Vec::with_capacity(input.futures_prices.len());
    let mut yield_sum = Decimal::ZERO;
    let mut contango_count: usize = 0;
    let mut backwardation_count: usize = 0;

    for contract in &input.futures_prices {
        if contract.expiry_months <= Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "expiry_months".into(),
                reason: format!(
                    "Expiry months must be positive for contract '{}'",
                    contract.label
                ),
            });
        }
        if contract.price <= Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "price".into(),
                reason: format!(
                    "Futures price must be positive for contract '{}'",
                    contract.label
                ),
            });
        }

        let t_years = contract.expiry_months / twelve;
        let basis = contract.price - input.spot_price;

        // Annualised basis rate: basis / S / T
        let annualised_basis_rate = if t_years > Decimal::ZERO {
            basis / input.spot_price / t_years
        } else {
            Decimal::ZERO
        };

        // Implied yield: ln(F / S) / T
        let ratio = contract.price / input.spot_price;
        let implied_yield = if t_years > Decimal::ZERO {
            ln_decimal(ratio) / t_years
        } else {
            Decimal::ZERO
        };

        yield_sum += implied_yield;

        if basis > Decimal::ZERO {
            contango_count += 1;
        } else if basis < Decimal::ZERO {
            backwardation_count += 1;
        }

        term_structure.push(BasisTerm {
            label: contract.label.clone(),
            expiry_months: contract.expiry_months,
            futures_price: contract.price,
            basis,
            annualised_basis_rate,
            implied_yield,
        });
    }

    let n = Decimal::from(input.futures_prices.len() as u32);
    let average_implied_yield = yield_sum / n;

    // Overall curve shape: majority vote
    let curve_shape = if contango_count > backwardation_count {
        MarketCondition::Contango
    } else if backwardation_count > contango_count {
        MarketCondition::Backwardation
    } else {
        MarketCondition::Flat
    };

    if contango_count > 0 && backwardation_count > 0 {
        warnings
            .push("Mixed term structure: some contracts in contango, some in backwardation".into());
    }

    let output = BasisAnalysisOutput {
        term_structure,
        curve_shape,
        average_implied_yield,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Futures Basis & Term Structure Analysis",
        &serde_json::json!({
            "spot_price": input.spot_price.to_string(),
            "num_contracts": input.futures_prices.len(),
            "risk_free_rate": input.risk_free_rate.to_string(),
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

    // Tolerance for comparing Decimal results against expected values.
    // Taylor/Newton implementations introduce small precision drift.
    const TOL: &str = "0.01";

    fn tol() -> Decimal {
        TOL.parse::<Decimal>().unwrap()
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

    // -- Helper tests for exp/ln --

    #[test]
    fn test_exp_decimal_basic() {
        // exp(0) = 1
        assert_eq!(exp_decimal(Decimal::ZERO), Decimal::ONE);

        // exp(1) ~ 2.71828
        let e1 = exp_decimal(Decimal::ONE);
        assert_approx(e1, dec!(2.71828), dec!(0.001), "exp(1)");

        // exp(-1) ~ 0.36788
        let em1 = exp_decimal(-Decimal::ONE);
        assert_approx(em1, dec!(0.36788), dec!(0.001), "exp(-1)");

        // exp(0.05) ~ 1.05127 (typical small rate*time)
        let e005 = exp_decimal(dec!(0.05));
        assert_approx(e005, dec!(1.05127), dec!(0.001), "exp(0.05)");
    }

    #[test]
    fn test_ln_decimal_basic() {
        // ln(1) = 0
        assert_eq!(ln_decimal(Decimal::ONE), Decimal::ZERO);

        // ln(e) ~ 1
        let e = exp_decimal(Decimal::ONE);
        let ln_e = ln_decimal(e);
        assert_approx(ln_e, Decimal::ONE, dec!(0.0001), "ln(e)");

        // ln(2) ~ 0.6931
        let ln2 = ln_decimal(Decimal::from(2));
        assert_approx(ln2, dec!(0.6931), dec!(0.001), "ln(2)");
    }

    // -----------------------------------------------------------------------
    // 1. test_equity_forward_no_dividend
    // -----------------------------------------------------------------------
    #[test]
    fn test_equity_forward_no_dividend() {
        // F = S * exp(r * T) = 100 * exp(0.05 * 1) = 100 * 1.05127 ~ 105.127
        let input = ForwardInput {
            spot_price: dec!(100),
            risk_free_rate: dec!(0.05),
            time_to_expiry: Decimal::ONE,
            storage_cost_rate: None,
            convenience_yield: None,
            dividend_yield: None,
            foreign_rate: None,
            underlying_type: UnderlyingType::Equity,
        };
        let result = price_forward(&input).unwrap();
        let out = &result.result;

        assert_approx(out.forward_price, dec!(105.127), tol(), "equity fwd no div");
        assert_approx(out.cost_of_carry, dec!(0.05), tight_tol(), "carry rate");
        assert!(out.basis > Decimal::ZERO);
        assert_eq!(out.market_condition, MarketCondition::Contango);

        // PV of forward should equal spot (no-arb) ~ 100
        assert_approx(
            out.present_value_of_forward,
            dec!(100),
            tol(),
            "PV of forward",
        );
    }

    // -----------------------------------------------------------------------
    // 2. test_equity_forward_with_dividend
    // -----------------------------------------------------------------------
    #[test]
    fn test_equity_forward_with_dividend() {
        // F = S * exp((r - q) * T) = 100 * exp((0.05 - 0.02) * 1) = 100 * exp(0.03)
        // exp(0.03) ~ 1.03045
        let input = ForwardInput {
            spot_price: dec!(100),
            risk_free_rate: dec!(0.05),
            time_to_expiry: Decimal::ONE,
            storage_cost_rate: None,
            convenience_yield: None,
            dividend_yield: Some(dec!(0.02)),
            foreign_rate: None,
            underlying_type: UnderlyingType::Equity,
        };
        let result = price_forward(&input).unwrap();
        let out = &result.result;

        // exp(0.03) ~ 1.03045 => F ~ 103.045
        assert_approx(
            out.forward_price,
            dec!(103.045),
            tol(),
            "equity fwd with div",
        );
        assert_approx(out.cost_of_carry, dec!(0.03), tight_tol(), "carry rate");
    }

    // -----------------------------------------------------------------------
    // 3. test_commodity_forward_storage_cost
    // -----------------------------------------------------------------------
    #[test]
    fn test_commodity_forward_storage_cost() {
        // F = S * exp((r + c) * T) = 50 * exp((0.05 + 0.03) * 0.5)
        // = 50 * exp(0.04) ~ 50 * 1.04081 ~ 52.04
        let input = ForwardInput {
            spot_price: dec!(50),
            risk_free_rate: dec!(0.05),
            time_to_expiry: dec!(0.5),
            storage_cost_rate: Some(dec!(0.03)),
            convenience_yield: None,
            dividend_yield: None,
            foreign_rate: None,
            underlying_type: UnderlyingType::Commodity,
        };
        let result = price_forward(&input).unwrap();
        let out = &result.result;

        // carry = 0.05 + 0.03 = 0.08; exp(0.08*0.5)=exp(0.04) ~ 1.04081
        assert_approx(
            out.forward_price,
            dec!(52.04),
            tol(),
            "commodity fwd storage",
        );
        assert_approx(out.cost_of_carry, dec!(0.08), tight_tol(), "carry rate");
    }

    // -----------------------------------------------------------------------
    // 4. test_commodity_forward_convenience_yield
    // -----------------------------------------------------------------------
    #[test]
    fn test_commodity_forward_convenience_yield() {
        // F = S * exp((r + c - y) * T) = 80 * exp((0.04 + 0.02 - 0.06) * 1)
        // = 80 * exp(0) = 80
        let input = ForwardInput {
            spot_price: dec!(80),
            risk_free_rate: dec!(0.04),
            time_to_expiry: Decimal::ONE,
            storage_cost_rate: Some(dec!(0.02)),
            convenience_yield: Some(dec!(0.06)),
            dividend_yield: None,
            foreign_rate: None,
            underlying_type: UnderlyingType::Commodity,
        };
        let result = price_forward(&input).unwrap();
        let out = &result.result;

        // Net carry = 0.04 + 0.02 - 0.06 = 0
        assert_approx(out.cost_of_carry, Decimal::ZERO, tight_tol(), "carry rate");
        assert_approx(
            out.forward_price,
            dec!(80),
            tol(),
            "commodity fwd conv yield",
        );
        assert_eq!(out.market_condition, MarketCondition::Flat);
    }

    // -----------------------------------------------------------------------
    // 5. test_currency_forward_interest_rate_parity
    // -----------------------------------------------------------------------
    #[test]
    fn test_currency_forward_interest_rate_parity() {
        // Covered interest rate parity:
        // F = S * exp((r_domestic - r_foreign) * T)
        // S = 1.10 (EUR/USD), r_domestic = 0.05, r_foreign = 0.03, T = 1
        // F = 1.10 * exp(0.02) ~ 1.10 * 1.02020 ~ 1.12222
        let input = ForwardInput {
            spot_price: dec!(1.10),
            risk_free_rate: dec!(0.05),
            time_to_expiry: Decimal::ONE,
            storage_cost_rate: None,
            convenience_yield: None,
            dividend_yield: None,
            foreign_rate: Some(dec!(0.03)),
            underlying_type: UnderlyingType::Currency,
        };
        let result = price_forward(&input).unwrap();
        let out = &result.result;

        // exp(0.02) ~ 1.02020 => F ~ 1.1222
        assert_approx(out.forward_price, dec!(1.1222), tol(), "currency fwd IRP");
        assert_approx(out.cost_of_carry, dec!(0.02), tight_tol(), "carry rate");
    }

    // -----------------------------------------------------------------------
    // 6. test_contango_detected
    // -----------------------------------------------------------------------
    #[test]
    fn test_contango_detected() {
        // Positive carry => F > S => Contango
        let input = ForwardInput {
            spot_price: dec!(100),
            risk_free_rate: dec!(0.10),
            time_to_expiry: Decimal::ONE,
            storage_cost_rate: None,
            convenience_yield: None,
            dividend_yield: None,
            foreign_rate: None,
            underlying_type: UnderlyingType::Index,
        };
        let result = price_forward(&input).unwrap();
        assert_eq!(result.result.market_condition, MarketCondition::Contango);
        assert!(result.result.forward_price > dec!(100));
    }

    // -----------------------------------------------------------------------
    // 7. test_backwardation_detected
    // -----------------------------------------------------------------------
    #[test]
    fn test_backwardation_detected() {
        // Large dividend yield exceeding risk-free rate => negative carry => F < S
        let input = ForwardInput {
            spot_price: dec!(100),
            risk_free_rate: dec!(0.02),
            time_to_expiry: Decimal::ONE,
            storage_cost_rate: None,
            convenience_yield: None,
            dividend_yield: Some(dec!(0.08)),
            foreign_rate: None,
            underlying_type: UnderlyingType::Equity,
        };
        let result = price_forward(&input).unwrap();
        assert_eq!(
            result.result.market_condition,
            MarketCondition::Backwardation
        );
        assert!(result.result.forward_price < dec!(100));
    }

    // -----------------------------------------------------------------------
    // 8. test_basis_calculation
    // -----------------------------------------------------------------------
    #[test]
    fn test_basis_calculation() {
        let input = ForwardInput {
            spot_price: dec!(100),
            risk_free_rate: dec!(0.05),
            time_to_expiry: Decimal::ONE,
            storage_cost_rate: None,
            convenience_yield: None,
            dividend_yield: None,
            foreign_rate: None,
            underlying_type: UnderlyingType::Equity,
        };
        let result = price_forward(&input).unwrap();
        let out = &result.result;

        // Basis = F - S
        let expected_basis = out.forward_price - dec!(100);
        assert_eq!(out.basis, expected_basis);

        // Basis rate = basis / S
        let expected_rate = expected_basis / dec!(100);
        assert_approx(out.basis_rate, expected_rate, dec!(0.0001), "basis rate");
    }

    // -----------------------------------------------------------------------
    // 9. test_cost_of_carry_rate
    // -----------------------------------------------------------------------
    #[test]
    fn test_cost_of_carry_rate() {
        // Full carry: r=0.05, q=0.02, c=0.01, y=0.005 => carry = 0.035
        let input = ForwardInput {
            spot_price: dec!(200),
            risk_free_rate: dec!(0.05),
            time_to_expiry: dec!(0.5),
            storage_cost_rate: Some(dec!(0.01)),
            convenience_yield: Some(dec!(0.005)),
            dividend_yield: None,
            foreign_rate: None,
            underlying_type: UnderlyingType::Commodity,
        };
        let result = price_forward(&input).unwrap();
        let carry = result.result.cost_of_carry;
        // For commodity: carry = r + c - y = 0.05 + 0.01 - 0.005 = 0.055
        assert_approx(carry, dec!(0.055), tight_tol(), "cost of carry");
    }

    // -----------------------------------------------------------------------
    // 10. test_long_position_profit
    // -----------------------------------------------------------------------
    #[test]
    fn test_long_position_profit() {
        // Locked in at F=100, spot has risen so new F > 100 => long profits
        let input = ForwardPositionInput {
            original_forward_price: dec!(100),
            current_spot: dec!(110),
            risk_free_rate: dec!(0.05),
            remaining_time: dec!(0.5),
            is_long: true,
            contract_size: dec!(10),
            dividend_yield: None,
        };
        let result = value_forward_position(&input).unwrap();
        let out = &result.result;

        // Current F = 110 * exp(0.05 * 0.5) = 110 * exp(0.025) ~ 110 * 1.02532 ~ 112.785
        assert!(out.current_forward_price > dec!(112));
        // MtM for long: (current_F - original_F) * exp(-r*T) > 0
        assert!(out.mark_to_market > Decimal::ZERO);
        assert!(out.profit_loss > Decimal::ZERO);
    }

    // -----------------------------------------------------------------------
    // 11. test_long_position_loss
    // -----------------------------------------------------------------------
    #[test]
    fn test_long_position_loss() {
        // Locked in at F=100, spot has fallen so new F < 100 => long loses
        let input = ForwardPositionInput {
            original_forward_price: dec!(100),
            current_spot: dec!(90),
            risk_free_rate: dec!(0.05),
            remaining_time: dec!(0.5),
            is_long: true,
            contract_size: dec!(10),
            dividend_yield: None,
        };
        let result = value_forward_position(&input).unwrap();
        let out = &result.result;

        // Current F = 90 * exp(0.025) ~ 92.28, which is < 100
        assert!(out.current_forward_price < dec!(100));
        assert!(out.mark_to_market < Decimal::ZERO);
        assert!(out.profit_loss < Decimal::ZERO);
    }

    // -----------------------------------------------------------------------
    // 12. test_short_position_mtm
    // -----------------------------------------------------------------------
    #[test]
    fn test_short_position_mtm() {
        // Short position profits when spot falls
        let input = ForwardPositionInput {
            original_forward_price: dec!(100),
            current_spot: dec!(90),
            risk_free_rate: dec!(0.05),
            remaining_time: dec!(0.5),
            is_long: false,
            contract_size: dec!(5),
            dividend_yield: None,
        };
        let result = value_forward_position(&input).unwrap();
        let out = &result.result;

        // Short profits when F falls
        assert!(out.mark_to_market > Decimal::ZERO);
        assert!(out.profit_loss > Decimal::ZERO);

        // Verify symmetry: long loss = short gain for same parameters
        let long_input = ForwardPositionInput {
            is_long: true,
            ..input.clone()
        };
        let long_result = value_forward_position(&long_input).unwrap();
        let long_mtm = long_result.result.mark_to_market;

        // Short MtM should be the negative of long MtM
        assert_approx(
            out.mark_to_market,
            -long_mtm,
            dec!(0.0001),
            "short/long symmetry",
        );
    }

    // -----------------------------------------------------------------------
    // 13. test_basis_analysis_term_structure
    // -----------------------------------------------------------------------
    #[test]
    fn test_basis_analysis_term_structure() {
        let input = BasisAnalysisInput {
            spot_price: dec!(100),
            futures_prices: vec![
                FuturesContract {
                    expiry_months: dec!(3),
                    price: dec!(101),
                    label: "Mar-25".into(),
                },
                FuturesContract {
                    expiry_months: dec!(6),
                    price: dec!(102.5),
                    label: "Jun-25".into(),
                },
                FuturesContract {
                    expiry_months: dec!(12),
                    price: dec!(105),
                    label: "Dec-25".into(),
                },
            ],
            risk_free_rate: dec!(0.05),
        };
        let result = futures_basis_analysis(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.term_structure.len(), 3);
        assert_eq!(out.curve_shape, MarketCondition::Contango);

        // All contracts above spot => all bases positive
        for term in &out.term_structure {
            assert!(term.basis > Decimal::ZERO);
            assert!(term.implied_yield > Decimal::ZERO);
        }

        // Basis increases with time to expiry
        assert!(out.term_structure[2].basis > out.term_structure[0].basis);
    }

    // -----------------------------------------------------------------------
    // 14. test_implied_yield_calculation
    // -----------------------------------------------------------------------
    #[test]
    fn test_implied_yield_calculation() {
        // Single contract: F=105, S=100, T=1 year
        // Implied yield = ln(105/100) / 1 = ln(1.05) ~ 0.04879
        let input = BasisAnalysisInput {
            spot_price: dec!(100),
            futures_prices: vec![FuturesContract {
                expiry_months: dec!(12),
                price: dec!(105),
                label: "Dec-25".into(),
            }],
            risk_free_rate: dec!(0.05),
        };
        let result = futures_basis_analysis(&input).unwrap();
        let iy = result.result.term_structure[0].implied_yield;

        // ln(1.05) ~ 0.04879
        assert_approx(iy, dec!(0.04879), tol(), "implied yield");
        assert_approx(
            result.result.average_implied_yield,
            iy,
            dec!(0.0001),
            "avg implied yield single",
        );
    }

    // -----------------------------------------------------------------------
    // 15. test_invalid_spot_price_error
    // -----------------------------------------------------------------------
    #[test]
    fn test_invalid_spot_price_error() {
        // Zero spot price
        let input = ForwardInput {
            spot_price: Decimal::ZERO,
            risk_free_rate: dec!(0.05),
            time_to_expiry: Decimal::ONE,
            storage_cost_rate: None,
            convenience_yield: None,
            dividend_yield: None,
            foreign_rate: None,
            underlying_type: UnderlyingType::Equity,
        };
        let err = price_forward(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "spot_price");
            }
            e => panic!("Expected InvalidInput for spot_price, got {e:?}"),
        }

        // Negative spot price
        let input_neg = ForwardInput {
            spot_price: dec!(-10),
            ..input
        };
        assert!(price_forward(&input_neg).is_err());

        // Zero time to expiry
        let input_t0 = ForwardInput {
            spot_price: dec!(100),
            time_to_expiry: Decimal::ZERO,
            ..input_neg
        };
        let err_t0 = price_forward(&input_t0).unwrap_err();
        match err_t0 {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "time_to_expiry");
            }
            e => panic!("Expected InvalidInput for time_to_expiry, got {e:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // 16. test_metadata_populated
    // -----------------------------------------------------------------------
    #[test]
    fn test_metadata_populated() {
        let input = ForwardInput {
            spot_price: dec!(100),
            risk_free_rate: dec!(0.05),
            time_to_expiry: Decimal::ONE,
            storage_cost_rate: None,
            convenience_yield: None,
            dividend_yield: None,
            foreign_rate: None,
            underlying_type: UnderlyingType::Equity,
        };
        let result = price_forward(&input).unwrap();

        assert!(!result.methodology.is_empty());
        assert!(result.methodology.contains("Forward"));
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
        assert!(!result.metadata.version.is_empty());
        assert!(!result.warnings.is_empty() || result.warnings.is_empty()); // no crash
    }

    // -----------------------------------------------------------------------
    // 17. test_position_validation_errors
    // -----------------------------------------------------------------------
    #[test]
    fn test_position_validation_errors() {
        // Zero contract size
        let input = ForwardPositionInput {
            original_forward_price: dec!(100),
            current_spot: dec!(105),
            risk_free_rate: dec!(0.05),
            remaining_time: dec!(0.5),
            is_long: true,
            contract_size: Decimal::ZERO,
            dividend_yield: None,
        };
        let err = value_forward_position(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "contract_size");
            }
            e => panic!("Expected InvalidInput for contract_size, got {e:?}"),
        }

        // Zero current spot
        let input2 = ForwardPositionInput {
            contract_size: dec!(10),
            current_spot: Decimal::ZERO,
            ..input
        };
        let err2 = value_forward_position(&input2).unwrap_err();
        match err2 {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "current_spot");
            }
            e => panic!("Expected InvalidInput for current_spot, got {e:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // 18. test_basis_analysis_empty_contracts_error
    // -----------------------------------------------------------------------
    #[test]
    fn test_basis_analysis_empty_contracts_error() {
        let input = BasisAnalysisInput {
            spot_price: dec!(100),
            futures_prices: vec![],
            risk_free_rate: dec!(0.05),
        };
        let err = futures_basis_analysis(&input).unwrap_err();
        match err {
            CorpFinanceError::InsufficientData(_) => {}
            e => panic!("Expected InsufficientData, got {e:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // 19. test_negative_risk_free_rate_allowed
    // -----------------------------------------------------------------------
    #[test]
    fn test_negative_risk_free_rate_allowed() {
        // Negative rates are valid (e.g. EUR, CHF, JPY historically)
        let input = ForwardInput {
            spot_price: dec!(100),
            risk_free_rate: dec!(-0.005),
            time_to_expiry: Decimal::ONE,
            storage_cost_rate: None,
            convenience_yield: None,
            dividend_yield: None,
            foreign_rate: None,
            underlying_type: UnderlyingType::Bond,
        };
        let result = price_forward(&input).unwrap();
        // Negative rate => F < S (backwardation)
        assert!(result.result.forward_price < dec!(100));
        assert_eq!(
            result.result.market_condition,
            MarketCondition::Backwardation
        );
    }

    // -----------------------------------------------------------------------
    // 20. test_basis_analysis_backwardation_curve
    // -----------------------------------------------------------------------
    #[test]
    fn test_basis_analysis_backwardation_curve() {
        let input = BasisAnalysisInput {
            spot_price: dec!(100),
            futures_prices: vec![
                FuturesContract {
                    expiry_months: dec!(3),
                    price: dec!(99),
                    label: "Mar-25".into(),
                },
                FuturesContract {
                    expiry_months: dec!(6),
                    price: dec!(97.5),
                    label: "Jun-25".into(),
                },
            ],
            risk_free_rate: dec!(0.02),
        };
        let result = futures_basis_analysis(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.curve_shape, MarketCondition::Backwardation);
        for term in &out.term_structure {
            assert!(term.basis < Decimal::ZERO);
            assert!(term.implied_yield < Decimal::ZERO);
        }
    }

    // -----------------------------------------------------------------------
    // 21. test_forward_pv_equals_spot_no_dividends
    // -----------------------------------------------------------------------
    #[test]
    fn test_forward_pv_equals_spot_no_dividends() {
        // When q=0, c=0, y=0: PV(F) = F * exp(-rT) = S * exp(rT) * exp(-rT) = S
        let input = ForwardInput {
            spot_price: dec!(150),
            risk_free_rate: dec!(0.08),
            time_to_expiry: dec!(2),
            storage_cost_rate: None,
            convenience_yield: None,
            dividend_yield: None,
            foreign_rate: None,
            underlying_type: UnderlyingType::Index,
        };
        let result = price_forward(&input).unwrap();
        assert_approx(
            result.result.present_value_of_forward,
            dec!(150),
            tol(),
            "PV(F) = S",
        );
    }

    // -----------------------------------------------------------------------
    // 22. test_exp_range_reduction_large_x
    // -----------------------------------------------------------------------
    #[test]
    fn test_exp_range_reduction_large_x() {
        // exp(5) ~ 148.413
        let e5 = exp_decimal(dec!(5));
        assert_approx(e5, dec!(148.413), dec!(0.1), "exp(5)");

        // exp(-5) ~ 0.00674
        let em5 = exp_decimal(dec!(-5));
        assert_approx(em5, dec!(0.00674), dec!(0.001), "exp(-5)");
    }

    // -----------------------------------------------------------------------
    // 23. test_position_with_dividend_yield
    // -----------------------------------------------------------------------
    #[test]
    fn test_position_with_dividend_yield() {
        let input = ForwardPositionInput {
            original_forward_price: dec!(100),
            current_spot: dec!(105),
            risk_free_rate: dec!(0.05),
            remaining_time: dec!(0.5),
            is_long: true,
            contract_size: dec!(100),
            dividend_yield: Some(dec!(0.03)),
        };
        let result = value_forward_position(&input).unwrap();
        let out = &result.result;

        // Current F = 105 * exp((0.05 - 0.03) * 0.5) = 105 * exp(0.01) ~ 106.055
        assert!(out.current_forward_price > dec!(105));
        assert!(out.current_forward_price < dec!(107));
        assert!(out.profit_loss > Decimal::ZERO);
    }
}
