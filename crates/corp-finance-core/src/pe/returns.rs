use chrono::NaiveDate;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::*;
use crate::CorpFinanceResult;

/// Input for PE returns calculation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReturnsInput {
    /// Periodic cash flows for IRR calculation (index 0 = initial investment, negative)
    pub cash_flows: Vec<Money>,
    /// Dated cash flows for XIRR calculation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dated_cash_flows: Option<Vec<CashFlow>>,
    /// Equity invested at entry
    pub entry_equity: Money,
    /// Equity received at exit
    pub exit_equity: Money,
    /// Holding period in years (for periodic IRR)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub holding_period_years: Option<Years>,
    /// Entry and exit dates (for XIRR and date-based holding period)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dates: Option<(NaiveDate, NaiveDate)>,
}

/// Output of PE returns calculation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReturnsOutput {
    /// Internal Rate of Return (periodic)
    pub irr: Option<Rate>,
    /// Extended IRR (date-based)
    pub xirr: Option<Rate>,
    /// Multiple on Invested Capital
    pub moic: Multiple,
    /// Cash-on-Cash return
    pub cash_on_cash: Multiple,
    /// Total equity invested
    pub total_invested: Money,
    /// Total equity returned
    pub total_returned: Money,
    /// Holding period in years
    pub holding_period: Years,
}

/// Calculate PE fund returns: IRR, XIRR, MOIC, Cash-on-Cash.
pub fn calculate_returns(
    input: &ReturnsInput,
) -> CorpFinanceResult<ComputationOutput<ReturnsOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // Validate inputs
    if input.entry_equity.is_zero() {
        return Err(CorpFinanceError::DivisionByZero {
            context: "entry_equity cannot be zero for MOIC/Cash-on-Cash".into(),
        });
    }
    if input.entry_equity < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "entry_equity".into(),
            reason: "Entry equity must be positive".into(),
        });
    }

    // Derive total invested and total returned from cash flows if available,
    // otherwise use entry/exit equity directly
    let (total_invested, total_returned) = if input.cash_flows.is_empty() {
        (input.entry_equity, input.exit_equity)
    } else {
        let invested = input
            .cash_flows
            .iter()
            .filter(|cf| cf.is_sign_negative())
            .map(|cf| cf.abs())
            .sum::<Decimal>();
        let returned = input
            .cash_flows
            .iter()
            .filter(|cf| cf.is_sign_positive())
            .sum::<Decimal>();
        (invested, returned)
    };

    if total_invested.is_zero() {
        return Err(CorpFinanceError::DivisionByZero {
            context: "total_invested is zero".into(),
        });
    }

    // MOIC
    let moic = total_returned / total_invested;

    // Cash-on-Cash (same as MOIC when no leverage adjustments)
    let cash_on_cash = input.exit_equity / input.entry_equity;

    // Holding period
    let holding_period = if let Some(hp) = input.holding_period_years {
        hp
    } else if let Some((entry, exit)) = input.dates {
        let days = (exit - entry).num_days();
        if days <= 0 {
            return Err(CorpFinanceError::InvalidInput {
                field: "dates".into(),
                reason: "Exit date must be after entry date".into(),
            });
        }
        Decimal::from(days) / dec!(365.25)
    } else {
        // Infer from cash_flows length (assume annual periods)
        let n = input.cash_flows.len();
        if n > 1 {
            Decimal::from((n - 1) as i64)
        } else {
            warnings.push("No holding period or dates provided; defaulting to 0".into());
            Decimal::ZERO
        }
    };

    // IRR from periodic cash flows
    let irr_result = if input.cash_flows.len() >= 2 {
        match crate::time_value::irr(&input.cash_flows, dec!(0.10)) {
            Ok(r) => Some(r),
            Err(e) => {
                warnings.push(format!("IRR calculation warning: {e}"));
                None
            }
        }
    } else {
        None
    };

    // XIRR from dated cash flows
    let xirr_result = if let Some(ref dated) = input.dated_cash_flows {
        if dated.len() >= 2 {
            let flows: Vec<(NaiveDate, Money)> =
                dated.iter().map(|cf| (cf.date, cf.amount)).collect();
            match crate::time_value::xirr(&flows, dec!(0.10)) {
                Ok(r) => Some(r),
                Err(e) => {
                    warnings.push(format!("XIRR calculation warning: {e}"));
                    None
                }
            }
        } else {
            warnings.push("XIRR requires at least 2 dated cash flows".into());
            None
        }
    } else {
        None
    };

    let output = ReturnsOutput {
        irr: irr_result,
        xirr: xirr_result,
        moic,
        cash_on_cash,
        total_invested,
        total_returned,
        holding_period,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "PE Returns: IRR, XIRR, MOIC, Cash-on-Cash",
        &serde_json::json!({
            "entry_equity": input.entry_equity.to_string(),
            "exit_equity": input.exit_equity.to_string(),
        }),
        warnings,
        elapsed,
        output,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    #[test]
    fn test_basic_moic() {
        let input = ReturnsInput {
            cash_flows: vec![dec!(-100), dec!(50), dec!(50), dec!(150)],
            dated_cash_flows: None,
            entry_equity: dec!(100),
            exit_equity: dec!(250),
            holding_period_years: Some(dec!(3)),
            dates: None,
        };
        let result = calculate_returns(&input).unwrap();
        // MOIC = (50+50+150) / 100 = 2.5
        assert_eq!(result.result.moic, dec!(2.5));
    }

    #[test]
    fn test_cash_on_cash() {
        let input = ReturnsInput {
            cash_flows: vec![],
            dated_cash_flows: None,
            entry_equity: dec!(200),
            exit_equity: dec!(500),
            holding_period_years: Some(dec!(4)),
            dates: None,
        };
        let result = calculate_returns(&input).unwrap();
        assert_eq!(result.result.cash_on_cash, dec!(2.5));
    }

    #[test]
    fn test_irr_calculation() {
        let input = ReturnsInput {
            cash_flows: vec![dec!(-1000), dec!(400), dec!(400), dec!(400)],
            dated_cash_flows: None,
            entry_equity: dec!(1000),
            exit_equity: dec!(400),
            holding_period_years: Some(dec!(3)),
            dates: None,
        };
        let result = calculate_returns(&input).unwrap();
        let irr_val = result.result.irr.unwrap();
        // IRR ~9.7%
        assert!((irr_val - dec!(0.097)).abs() < dec!(0.01));
    }

    #[test]
    fn test_xirr_calculation() {
        let d0 = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        let d1 = NaiveDate::from_ymd_opt(2021, 1, 1).unwrap();
        let d2 = NaiveDate::from_ymd_opt(2022, 1, 1).unwrap();
        let d3 = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();

        let input = ReturnsInput {
            cash_flows: vec![],
            dated_cash_flows: Some(vec![
                CashFlow {
                    date: d0,
                    amount: dec!(-1000),
                    label: None,
                },
                CashFlow {
                    date: d1,
                    amount: dec!(400),
                    label: None,
                },
                CashFlow {
                    date: d2,
                    amount: dec!(400),
                    label: None,
                },
                CashFlow {
                    date: d3,
                    amount: dec!(400),
                    label: None,
                },
            ]),
            entry_equity: dec!(1000),
            exit_equity: dec!(400),
            holding_period_years: None,
            dates: Some((d0, d3)),
        };
        let result = calculate_returns(&input).unwrap();
        assert!(result.result.xirr.is_some());
    }

    #[test]
    fn test_holding_period_from_dates() {
        let d0 = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        let d1 = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let input = ReturnsInput {
            cash_flows: vec![],
            dated_cash_flows: None,
            entry_equity: dec!(100),
            exit_equity: dec!(200),
            holding_period_years: None,
            dates: Some((d0, d1)),
        };
        let result = calculate_returns(&input).unwrap();
        // ~5 years
        assert!((result.result.holding_period - dec!(5)).abs() < dec!(0.1));
    }

    #[test]
    fn test_zero_entry_equity_error() {
        let input = ReturnsInput {
            cash_flows: vec![],
            dated_cash_flows: None,
            entry_equity: dec!(0),
            exit_equity: dec!(100),
            holding_period_years: Some(dec!(1)),
            dates: None,
        };
        assert!(calculate_returns(&input).is_err());
    }
}
