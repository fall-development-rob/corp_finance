//! Total Shareholder Return (TSR) Decomposition.
//!
//! Decomposes TSR into:
//! 1. **Capital gain** — price appreciation.
//! 2. **Dividend yield** — income from dividends.
//! 3. **Buyback contribution** — value returned via share repurchases.
//! 4. **Annualized return** — using Newton's method for nth root.
//!
//! All arithmetic uses `rust_decimal::Decimal`. No `f64`.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Newton's method nth root: solve x^n = target.
/// Returns x such that x^n ~ target.
fn newton_nth_root(target: Decimal, n: Decimal, iterations: u32) -> Decimal {
    if target <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    if n == Decimal::ONE {
        return target;
    }

    // Initial guess: start at target^(1/n) ~ 1 + (target-1)/n for values near 1
    let mut x = Decimal::ONE + (target - Decimal::ONE) / n;
    if x <= Decimal::ZERO {
        x = dec!(0.5);
    }

    let n_minus_1 = n - Decimal::ONE;

    for _ in 0..iterations {
        // x_k^(n-1) via iterative multiplication
        let x_pow_n_minus_1 = iterative_pow(x, n_minus_1);
        if x_pow_n_minus_1 == Decimal::ZERO {
            break;
        }
        let x_pow_n = x_pow_n_minus_1 * x;

        // Newton step: x_{k+1} = x_k - (x_k^n - target) / (n * x_k^(n-1))
        let numerator = x_pow_n - target;
        let denominator = n * x_pow_n_minus_1;
        if denominator == Decimal::ZERO {
            break;
        }
        x -= numerator / denominator;
        if x <= Decimal::ZERO {
            x = dec!(0.001);
        }
    }
    x
}

/// Iterative exponentiation for integer part + fractional approximation.
/// For non-integer exponents, uses: x^n = x^floor(n) * x^frac(n),
/// where x^frac(n) ~ 1 + frac(n) * ln(x) (first-order Taylor).
fn iterative_pow(base: Decimal, exp: Decimal) -> Decimal {
    if base == Decimal::ZERO {
        return Decimal::ZERO;
    }
    if exp == Decimal::ZERO {
        return Decimal::ONE;
    }

    let is_negative = exp < Decimal::ZERO;
    let abs_exp = exp.abs();

    // Split into integer and fractional parts
    let int_part = abs_exp.trunc();
    let frac_part = abs_exp.fract();

    // Integer power via repeated multiplication
    let mut result = Decimal::ONE;
    let n_int = int_part.to_string().parse::<u64>().unwrap_or(0);
    for _ in 0..n_int.min(200) {
        result *= base;
    }

    // Fractional approximation: base^frac ~ 1 + frac * ln(base)
    if frac_part > Decimal::ZERO {
        let ln_base = decimal_ln(base);
        let frac_pow = Decimal::ONE + frac_part * ln_base;
        result *= frac_pow;
    }

    if is_negative {
        if result == Decimal::ZERO {
            Decimal::ZERO
        } else {
            Decimal::ONE / result
        }
    } else {
        result
    }
}

/// Natural logarithm via Taylor series. ln(x) for x > 0.
fn decimal_ln(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    let ln2 = dec!(0.6931471805599453);
    let mut val = x;
    let mut adjust = Decimal::ZERO;
    while val > dec!(2.0) {
        val /= dec!(2);
        adjust += ln2;
    }
    while val < dec!(0.5) {
        val *= dec!(2);
        adjust -= ln2;
    }
    let z = (val - Decimal::ONE) / (val + Decimal::ONE);
    let z2 = z * z;
    let mut term = z;
    let mut sum = z;
    for k in 1u32..40 {
        term *= z2;
        let denom = Decimal::from(2 * k + 1);
        sum += term / denom;
    }
    dec!(2) * sum + adjust
}

// ---------------------------------------------------------------------------
// Input / Output
// ---------------------------------------------------------------------------

/// Input for total shareholder return decomposition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TotalShareholderReturnInput {
    /// Share price at the beginning of the period.
    pub beginning_price: Decimal,
    /// Share price at the end of the period.
    pub ending_price: Decimal,
    /// Total dividends received per share during the period.
    pub dividends_received: Decimal,
    /// Buyback yield during the period (as a decimal, e.g., 0.02 = 2%).
    pub buyback_yield: Decimal,
    /// Shares outstanding at beginning.
    pub shares_beginning: Decimal,
    /// Shares outstanding at end.
    pub shares_ending: Decimal,
    /// Holding period in years.
    pub holding_period_years: Decimal,
}

/// Output of total shareholder return decomposition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TotalShareholderReturnOutput {
    /// Total return over the period.
    pub total_return: Decimal,
    /// Capital gain component.
    pub capital_gain: Decimal,
    /// Capital gain as % of total return.
    pub capital_gain_pct: Decimal,
    /// Dividend yield component.
    pub dividend_yield: Decimal,
    /// Dividend yield as % of total return.
    pub dividend_yield_pct: Decimal,
    /// Buyback contribution component.
    pub buyback_contribution: Decimal,
    /// Buyback as % of total return.
    pub buyback_pct: Decimal,
    /// Annualized return (CAGR).
    pub annualized_return: Decimal,
    /// Price return component (same as capital gain).
    pub price_return: Decimal,
    /// Income return: dividend + buyback.
    pub income_return: Decimal,
}

// ---------------------------------------------------------------------------
// Core function
// ---------------------------------------------------------------------------

/// Calculate and decompose total shareholder return.
pub fn calculate_total_shareholder_return(
    input: &TotalShareholderReturnInput,
) -> CorpFinanceResult<TotalShareholderReturnOutput> {
    validate_input(input)?;

    let bp = input.beginning_price;
    let ep = input.ending_price;
    let divs = input.dividends_received;
    let bb_yield = input.buyback_yield;

    // Component returns
    let capital_gain = (ep - bp) / bp;
    let dividend_yield = divs / bp;
    let buyback_contribution = bb_yield;

    // Total return
    let total_return = capital_gain + dividend_yield + buyback_contribution;

    // Decomposition percentages (as % of total return)
    let (capital_gain_pct, dividend_yield_pct, buyback_pct) = if total_return == Decimal::ZERO {
        (Decimal::ZERO, Decimal::ZERO, Decimal::ZERO)
    } else {
        let cg_pct = capital_gain / total_return * dec!(100);
        let dy_pct = dividend_yield / total_return * dec!(100);
        let bb_pct = buyback_contribution / total_return * dec!(100);
        (cg_pct, dy_pct, bb_pct)
    };

    // Annualized return via Newton nth root
    // (1 + total_return)^(1/years) - 1
    let years = input.holding_period_years;
    let annualized_return = if years <= Decimal::ONE {
        total_return
    } else {
        let growth_factor = Decimal::ONE + total_return;
        if growth_factor <= Decimal::ZERO {
            // Total loss exceeding 100%, annualization not meaningful
            dec!(-1)
        } else {
            newton_nth_root(growth_factor, years, 40) - Decimal::ONE
        }
    };

    // Price vs income decomposition
    let price_return = capital_gain;
    let income_return = dividend_yield + buyback_contribution;

    Ok(TotalShareholderReturnOutput {
        total_return,
        capital_gain,
        capital_gain_pct,
        dividend_yield,
        dividend_yield_pct,
        buyback_contribution,
        buyback_pct,
        annualized_return,
        price_return,
        income_return,
    })
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_input(input: &TotalShareholderReturnInput) -> CorpFinanceResult<()> {
    if input.beginning_price <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "beginning_price".into(),
            reason: "Beginning price must be positive.".into(),
        });
    }
    if input.ending_price < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "ending_price".into(),
            reason: "Ending price must be non-negative.".into(),
        });
    }
    if input.dividends_received < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "dividends_received".into(),
            reason: "Dividends received must be non-negative.".into(),
        });
    }
    if input.shares_beginning <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "shares_beginning".into(),
            reason: "Beginning shares must be positive.".into(),
        });
    }
    if input.shares_ending <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "shares_ending".into(),
            reason: "Ending shares must be positive.".into(),
        });
    }
    if input.holding_period_years <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "holding_period_years".into(),
            reason: "Holding period must be positive.".into(),
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

    fn one_year_input() -> TotalShareholderReturnInput {
        TotalShareholderReturnInput {
            beginning_price: dec!(100),
            ending_price: dec!(110),
            dividends_received: dec!(3),
            buyback_yield: dec!(0.02),
            shares_beginning: dec!(1000),
            shares_ending: dec!(980),
            holding_period_years: Decimal::ONE,
        }
    }

    fn multi_year_input() -> TotalShareholderReturnInput {
        TotalShareholderReturnInput {
            beginning_price: dec!(50),
            ending_price: dec!(75),
            dividends_received: dec!(5),
            buyback_yield: dec!(0.03),
            shares_beginning: dec!(500),
            shares_ending: dec!(470),
            holding_period_years: dec!(3),
        }
    }

    #[test]
    fn test_basic_total_return() {
        let input = one_year_input();
        let out = calculate_total_shareholder_return(&input).unwrap();
        // capital gain = (110-100)/100 = 0.10
        // div yield = 3/100 = 0.03
        // buyback = 0.02
        // total = 0.15
        assert!(approx_eq(out.total_return, dec!(0.15), dec!(0.001)));
    }

    #[test]
    fn test_capital_gain() {
        let input = one_year_input();
        let out = calculate_total_shareholder_return(&input).unwrap();
        assert!(approx_eq(out.capital_gain, dec!(0.10), dec!(0.001)));
    }

    #[test]
    fn test_dividend_yield() {
        let input = one_year_input();
        let out = calculate_total_shareholder_return(&input).unwrap();
        assert!(approx_eq(out.dividend_yield, dec!(0.03), dec!(0.001)));
    }

    #[test]
    fn test_buyback_contribution() {
        let input = one_year_input();
        let out = calculate_total_shareholder_return(&input).unwrap();
        assert_eq!(out.buyback_contribution, dec!(0.02));
    }

    #[test]
    fn test_decomposition_sums_to_total() {
        let input = one_year_input();
        let out = calculate_total_shareholder_return(&input).unwrap();
        let sum = out.capital_gain + out.dividend_yield + out.buyback_contribution;
        assert!(approx_eq(sum, out.total_return, dec!(0.0001)));
    }

    #[test]
    fn test_decomposition_pcts_sum_to_100() {
        let input = one_year_input();
        let out = calculate_total_shareholder_return(&input).unwrap();
        let total_pct = out.capital_gain_pct + out.dividend_yield_pct + out.buyback_pct;
        assert!(approx_eq(total_pct, dec!(100), dec!(0.1)));
    }

    #[test]
    fn test_annualized_one_year() {
        let input = one_year_input();
        let out = calculate_total_shareholder_return(&input).unwrap();
        // 1-year: annualized = total
        assert!(approx_eq(
            out.annualized_return,
            out.total_return,
            dec!(0.001)
        ));
    }

    #[test]
    fn test_annualized_multi_year() {
        let input = multi_year_input();
        let out = calculate_total_shareholder_return(&input).unwrap();
        // total = (75-50)/50 + 5/50 + 0.03 = 0.50 + 0.10 + 0.03 = 0.63
        // annualized = (1.63)^(1/3) - 1 ~ 0.1761
        assert!(approx_eq(out.annualized_return, dec!(0.176), dec!(0.01)));
    }

    #[test]
    fn test_price_return_equals_capital_gain() {
        let input = one_year_input();
        let out = calculate_total_shareholder_return(&input).unwrap();
        assert_eq!(out.price_return, out.capital_gain);
    }

    #[test]
    fn test_income_return() {
        let input = one_year_input();
        let out = calculate_total_shareholder_return(&input).unwrap();
        let expected = out.dividend_yield + out.buyback_contribution;
        assert!(approx_eq(out.income_return, expected, dec!(0.0001)));
    }

    #[test]
    fn test_dividend_heavy_return() {
        let input = TotalShareholderReturnInput {
            beginning_price: dec!(100),
            ending_price: dec!(98),
            dividends_received: dec!(12),
            buyback_yield: Decimal::ZERO,
            shares_beginning: dec!(1000),
            shares_ending: dec!(1000),
            holding_period_years: Decimal::ONE,
        };
        let out = calculate_total_shareholder_return(&input).unwrap();
        // capital = -0.02, div = 0.12, total = 0.10
        assert!(out.dividend_yield_pct > dec!(50));
    }

    #[test]
    fn test_buyback_heavy_return() {
        let input = TotalShareholderReturnInput {
            beginning_price: dec!(100),
            ending_price: dec!(101),
            dividends_received: dec!(1),
            buyback_yield: dec!(0.08),
            shares_beginning: dec!(1000),
            shares_ending: dec!(920),
            holding_period_years: Decimal::ONE,
        };
        let out = calculate_total_shareholder_return(&input).unwrap();
        // total = 0.01 + 0.01 + 0.08 = 0.10
        assert!(out.buyback_pct > dec!(50));
    }

    #[test]
    fn test_capital_gain_dominated() {
        let input = TotalShareholderReturnInput {
            beginning_price: dec!(50),
            ending_price: dec!(100),
            dividends_received: dec!(1),
            buyback_yield: dec!(0.01),
            shares_beginning: dec!(1000),
            shares_ending: dec!(990),
            holding_period_years: dec!(2),
        };
        let out = calculate_total_shareholder_return(&input).unwrap();
        // capital = 1.0, div = 0.02, bb = 0.01 => cap gain dominates
        assert!(out.capital_gain_pct > dec!(90));
    }

    #[test]
    fn test_zero_total_return() {
        let input = TotalShareholderReturnInput {
            beginning_price: dec!(100),
            ending_price: dec!(100),
            dividends_received: Decimal::ZERO,
            buyback_yield: Decimal::ZERO,
            shares_beginning: dec!(1000),
            shares_ending: dec!(1000),
            holding_period_years: Decimal::ONE,
        };
        let out = calculate_total_shareholder_return(&input).unwrap();
        assert_eq!(out.total_return, Decimal::ZERO);
        assert_eq!(out.capital_gain_pct, Decimal::ZERO);
    }

    #[test]
    fn test_negative_return() {
        let input = TotalShareholderReturnInput {
            beginning_price: dec!(100),
            ending_price: dec!(80),
            dividends_received: dec!(2),
            buyback_yield: Decimal::ZERO,
            shares_beginning: dec!(1000),
            shares_ending: dec!(1000),
            holding_period_years: Decimal::ONE,
        };
        let out = calculate_total_shareholder_return(&input).unwrap();
        // total = -0.20 + 0.02 = -0.18
        assert!(out.total_return < Decimal::ZERO);
    }

    #[test]
    fn test_reject_zero_beginning_price() {
        let input = TotalShareholderReturnInput {
            beginning_price: Decimal::ZERO,
            ..one_year_input()
        };
        assert!(calculate_total_shareholder_return(&input).is_err());
    }

    #[test]
    fn test_reject_negative_ending_price() {
        let input = TotalShareholderReturnInput {
            ending_price: dec!(-10),
            ..one_year_input()
        };
        assert!(calculate_total_shareholder_return(&input).is_err());
    }

    #[test]
    fn test_reject_negative_dividends() {
        let input = TotalShareholderReturnInput {
            dividends_received: dec!(-5),
            ..one_year_input()
        };
        assert!(calculate_total_shareholder_return(&input).is_err());
    }

    #[test]
    fn test_reject_zero_holding_period() {
        let input = TotalShareholderReturnInput {
            holding_period_years: Decimal::ZERO,
            ..one_year_input()
        };
        assert!(calculate_total_shareholder_return(&input).is_err());
    }

    #[test]
    fn test_reject_zero_shares_beginning() {
        let input = TotalShareholderReturnInput {
            shares_beginning: Decimal::ZERO,
            ..one_year_input()
        };
        assert!(calculate_total_shareholder_return(&input).is_err());
    }

    #[test]
    fn test_reject_zero_shares_ending() {
        let input = TotalShareholderReturnInput {
            shares_ending: Decimal::ZERO,
            ..one_year_input()
        };
        assert!(calculate_total_shareholder_return(&input).is_err());
    }

    #[test]
    fn test_annualized_five_year() {
        // 50% total return over 5 years
        let input = TotalShareholderReturnInput {
            beginning_price: dec!(100),
            ending_price: dec!(150),
            dividends_received: Decimal::ZERO,
            buyback_yield: Decimal::ZERO,
            shares_beginning: dec!(1000),
            shares_ending: dec!(1000),
            holding_period_years: dec!(5),
        };
        let out = calculate_total_shareholder_return(&input).unwrap();
        // (1.50)^(1/5) - 1 ~ 0.08447
        assert!(approx_eq(out.annualized_return, dec!(0.0845), dec!(0.005)));
    }

    #[test]
    fn test_newton_nth_root_square() {
        // sqrt(4) = 2
        let result = newton_nth_root(dec!(4), dec!(2), 40);
        assert!(approx_eq(result, dec!(2), dec!(0.001)));
    }

    #[test]
    fn test_newton_nth_root_cube() {
        // cbrt(8) = 2
        let result = newton_nth_root(dec!(8), dec!(3), 40);
        assert!(approx_eq(result, dec!(2), dec!(0.001)));
    }

    #[test]
    fn test_serialization_roundtrip() {
        let input = one_year_input();
        let out = calculate_total_shareholder_return(&input).unwrap();
        let json = serde_json::to_string(&out).unwrap();
        let _: TotalShareholderReturnOutput = serde_json::from_str(&json).unwrap();
    }
}
