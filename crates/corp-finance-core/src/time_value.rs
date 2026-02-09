use chrono::NaiveDate;
use rust_decimal::Decimal;
use rust_decimal::MathematicalOps;
use rust_decimal_macros::dec;

use crate::error::CorpFinanceError;
use crate::types::{Money, Rate};
use crate::CorpFinanceResult;

const CONVERGENCE_THRESHOLD: Decimal = dec!(0.0000001);
const MAX_IRR_ITERATIONS: u32 = 100;

/// Net Present Value of a series of cash flows
pub fn npv(rate: Rate, cash_flows: &[Money]) -> CorpFinanceResult<Money> {
    if rate <= dec!(-1) {
        return Err(CorpFinanceError::InvalidInput {
            field: "rate".into(),
            reason: "Discount rate must be greater than -100%".into(),
        });
    }

    let mut result = Decimal::ZERO;
    let one_plus_r = Decimal::ONE + rate;
    let mut discount = Decimal::ONE;

    for (t, cf) in cash_flows.iter().enumerate() {
        if t > 0 {
            discount *= one_plus_r;
        }
        if discount.is_zero() {
            return Err(CorpFinanceError::DivisionByZero {
                context: format!("NPV discount factor at period {t}"),
            });
        }
        result += cf / discount;
    }

    Ok(result)
}

/// Internal Rate of Return using Newton-Raphson
pub fn irr(cash_flows: &[Money], guess: Rate) -> CorpFinanceResult<Rate> {
    if cash_flows.len() < 2 {
        return Err(CorpFinanceError::InsufficientData(
            "IRR requires at least 2 cash flows".into(),
        ));
    }

    let mut rate = guess;

    for i in 0..MAX_IRR_ITERATIONS {
        let mut npv_val = Decimal::ZERO;
        let mut dnpv = Decimal::ZERO;
        let one_plus_r = Decimal::ONE + rate;

        for (t, cf) in cash_flows.iter().enumerate() {
            let t_dec = Decimal::from(t as i64);
            let discount = one_plus_r.powd(t_dec);
            if discount.is_zero() {
                continue;
            }
            npv_val += cf / discount;
            if t > 0 {
                dnpv -= t_dec * cf / (one_plus_r.powd(t_dec + Decimal::ONE));
            }
        }

        if npv_val.abs() < CONVERGENCE_THRESHOLD {
            return Ok(rate);
        }

        if dnpv.is_zero() {
            return Err(CorpFinanceError::ConvergenceFailure {
                function: "IRR".into(),
                iterations: i,
                last_delta: npv_val,
            });
        }

        rate -= npv_val / dnpv;

        // Guard against divergence
        if rate < dec!(-0.99) {
            rate = dec!(-0.99);
        } else if rate > dec!(100.0) {
            rate = dec!(100.0);
        }
    }

    Err(CorpFinanceError::ConvergenceFailure {
        function: "IRR".into(),
        iterations: MAX_IRR_ITERATIONS,
        last_delta: npv(rate, cash_flows).unwrap_or(Decimal::MAX),
    })
}

/// Extended IRR for irregular cash flow dates using Newton-Raphson
pub fn xirr(dated_flows: &[(NaiveDate, Money)], guess: Rate) -> CorpFinanceResult<Rate> {
    if dated_flows.len() < 2 {
        return Err(CorpFinanceError::InsufficientData(
            "XIRR requires at least 2 cash flows".into(),
        ));
    }

    let base_date = dated_flows[0].0;
    let mut rate = guess;

    for i in 0..MAX_IRR_ITERATIONS {
        let mut npv_val = Decimal::ZERO;
        let mut dnpv = Decimal::ZERO;

        for (date, amount) in dated_flows {
            let days = (*date - base_date).num_days();
            let years = Decimal::from(days) / dec!(365.25);
            let one_plus_r = Decimal::ONE + rate;

            if one_plus_r <= Decimal::ZERO {
                return Err(CorpFinanceError::ConvergenceFailure {
                    function: "XIRR".into(),
                    iterations: i,
                    last_delta: npv_val,
                });
            }

            let discount = one_plus_r.powd(years);
            if discount.is_zero() {
                continue;
            }

            npv_val += amount / discount;
            dnpv -= years * amount / (one_plus_r * discount);
        }

        if npv_val.abs() < CONVERGENCE_THRESHOLD {
            return Ok(rate);
        }

        if dnpv.is_zero() {
            return Err(CorpFinanceError::ConvergenceFailure {
                function: "XIRR".into(),
                iterations: i,
                last_delta: npv_val,
            });
        }

        rate -= npv_val / dnpv;

        if rate < dec!(-0.99) {
            rate = dec!(-0.99);
        } else if rate > dec!(100.0) {
            rate = dec!(100.0);
        }
    }

    Err(CorpFinanceError::ConvergenceFailure {
        function: "XIRR".into(),
        iterations: MAX_IRR_ITERATIONS,
        last_delta: Decimal::ZERO,
    })
}

/// Present Value
pub fn pv(rate: Rate, nper: u32, pmt: Money, fv: Money) -> CorpFinanceResult<Money> {
    if rate.is_zero() {
        return Ok(-(pmt * Decimal::from(nper) + fv));
    }

    let one_plus_r = Decimal::ONE + rate;
    let factor = one_plus_r.powd(Decimal::from(nper));

    if factor.is_zero() {
        return Err(CorpFinanceError::DivisionByZero {
            context: "PV factor".into(),
        });
    }

    let annuity_factor = (Decimal::ONE - Decimal::ONE / factor) / rate;
    Ok(-(pmt * annuity_factor + fv / factor))
}

/// Future Value
pub fn fv(rate: Rate, nper: u32, pmt: Money, present_value: Money) -> CorpFinanceResult<Money> {
    if rate.is_zero() {
        return Ok(-(present_value + pmt * Decimal::from(nper)));
    }

    let one_plus_r = Decimal::ONE + rate;
    let factor = one_plus_r.powd(Decimal::from(nper));
    let annuity_factor = (factor - Decimal::ONE) / rate;

    Ok(-(present_value * factor + pmt * annuity_factor))
}

/// Payment (PMT)
pub fn pmt(rate: Rate, nper: u32, present_value: Money, future_value: Money) -> CorpFinanceResult<Money> {
    if nper == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "nper".into(),
            reason: "Number of periods must be > 0".into(),
        });
    }

    if rate.is_zero() {
        return Ok(-(present_value + future_value) / Decimal::from(nper));
    }

    let one_plus_r = Decimal::ONE + rate;
    let factor = one_plus_r.powd(Decimal::from(nper));
    let annuity_factor = (factor - Decimal::ONE) / rate;

    if annuity_factor.is_zero() {
        return Err(CorpFinanceError::DivisionByZero {
            context: "PMT annuity factor".into(),
        });
    }

    Ok(-(present_value * factor + future_value) / annuity_factor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_npv_basic() {
        let cfs = vec![dec!(-1000), dec!(300), dec!(400), dec!(500)];
        let result = npv(dec!(0.10), &cfs).unwrap();
        // NPV at 10%: -1000 + 300/1.1 + 400/1.21 + 500/1.331 ≈ -21.04
        assert!((result - dec!(-21.04)).abs() < dec!(1.0));
    }

    #[test]
    fn test_irr_basic() {
        let cfs = vec![dec!(-1000), dec!(400), dec!(400), dec!(400)];
        let result = irr(&cfs, dec!(0.10)).unwrap();
        // IRR should be ~9.7%
        assert!((result - dec!(0.097)).abs() < dec!(0.01));
    }

    #[test]
    fn test_pv_basic() {
        let result = pv(dec!(0.08), 10, dec!(-100), dec!(0)).unwrap();
        // PV of annuity: 100 * (1 - 1/1.08^10) / 0.08 = ~671
        assert!((result - dec!(671)).abs() < dec!(2.0));
    }

    #[test]
    fn test_npv_zero_rate() {
        let cfs = vec![dec!(-100), dec!(50), dec!(50), dec!(50)];
        let result = npv(dec!(0.0), &cfs).unwrap();
        assert_eq!(result, dec!(50));
    }
}
