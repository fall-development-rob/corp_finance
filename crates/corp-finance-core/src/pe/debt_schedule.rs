use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::*;
use crate::CorpFinanceResult;

/// Amortisation type for a debt tranche
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AmortisationType {
    /// No repayment until maturity (bullet repayment at end)
    Bullet,
    /// Fixed percentage of original principal per year
    StraightLine(Rate),
    /// Custom repayment schedule (one amount per year)
    Custom(Vec<Money>),
    /// Cash sweep — percentage of excess cash used for repayment
    CashSweep(Rate),
}

/// Input for a single debt tranche
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebtTrancheInput {
    pub name: String,
    pub amount: Money,
    pub interest_rate: Rate,
    pub is_floating: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_rate: Option<Rate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spread: Option<Rate>,
    pub amortisation: AmortisationType,
    pub maturity_years: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pik_rate: Option<Rate>,
    pub seniority: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commitment_fee: Option<Rate>,
    pub is_revolver: bool,
}

/// A single period in the debt schedule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebtPeriod {
    pub year: u32,
    pub opening_balance: Money,
    pub interest: Money,
    pub pik_interest: Money,
    pub scheduled_repayment: Money,
    pub closing_balance: Money,
}

/// Output for a single tranche debt schedule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebtScheduleOutput {
    pub tranche_name: String,
    pub periods: Vec<DebtPeriod>,
    pub total_interest_paid: Money,
    pub total_principal_paid: Money,
}

/// Build a year-by-year debt schedule for a single tranche.
pub fn build_debt_schedule(
    input: &DebtTrancheInput,
) -> CorpFinanceResult<ComputationOutput<DebtScheduleOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    if input.amount <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "amount".into(),
            reason: "Debt amount must be positive".into(),
        });
    }
    if input.maturity_years == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "maturity_years".into(),
            reason: "Maturity must be at least 1 year".into(),
        });
    }

    // Determine the effective interest rate
    let effective_rate = if input.is_floating {
        let base = input.base_rate.unwrap_or_else(|| {
            warnings.push("Floating rate tranche missing base_rate; using 0".into());
            Decimal::ZERO
        });
        let spread = input.spread.unwrap_or_else(|| {
            warnings.push("Floating rate tranche missing spread; using interest_rate".into());
            input.interest_rate
        });
        base + spread
    } else {
        input.interest_rate
    };

    let pik_rate = input.pik_rate.unwrap_or(Decimal::ZERO);
    let original_amount = input.amount;

    let mut periods = Vec::with_capacity(input.maturity_years as usize);
    let mut balance = input.amount;
    let mut total_interest_paid = Decimal::ZERO;
    let mut total_principal_paid = Decimal::ZERO;

    for year in 1..=input.maturity_years {
        let opening = balance;

        // PIK interest: capitalised onto the balance
        let pik_interest = opening * pik_rate;

        // Cash interest on opening balance (before PIK addition)
        let interest = opening * effective_rate;
        total_interest_paid += interest;

        // Add PIK to balance before repayment calculation
        balance += pik_interest;

        // Calculate scheduled repayment
        let repayment = match &input.amortisation {
            AmortisationType::Bullet => {
                if year == input.maturity_years {
                    balance
                } else {
                    Decimal::ZERO
                }
            }
            AmortisationType::StraightLine(pct) => {
                let annual = original_amount * pct;
                if year == input.maturity_years {
                    // Final year: repay remaining balance
                    balance
                } else {
                    annual.min(balance)
                }
            }
            AmortisationType::Custom(schedule) => {
                let idx = (year - 1) as usize;
                if idx < schedule.len() {
                    if year == input.maturity_years {
                        balance
                    } else {
                        schedule[idx].min(balance)
                    }
                } else if year == input.maturity_years {
                    balance
                } else {
                    Decimal::ZERO
                }
            }
            AmortisationType::CashSweep(_pct) => {
                // Cash sweep requires external cash flow data;
                // for now treat as bullet with a warning
                if year == input.maturity_years {
                    balance
                } else {
                    warnings.push(format!(
                        "Year {year}: CashSweep amortisation requires external cash flows; treated as bullet"
                    ));
                    Decimal::ZERO
                }
            }
        };

        balance -= repayment;
        total_principal_paid += repayment;

        periods.push(DebtPeriod {
            year,
            opening_balance: opening,
            interest,
            pik_interest,
            scheduled_repayment: repayment,
            closing_balance: balance,
        });
    }

    let output = DebtScheduleOutput {
        tranche_name: input.name.clone(),
        periods,
        total_interest_paid,
        total_principal_paid,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Debt Schedule Builder",
        &serde_json::json!({
            "tranche": input.name,
            "amount": input.amount.to_string(),
            "rate": effective_rate.to_string(),
            "maturity": input.maturity_years,
        }),
        warnings,
        elapsed,
        output,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn bullet_input() -> DebtTrancheInput {
        DebtTrancheInput {
            name: "Senior Term Loan".into(),
            amount: dec!(1000),
            interest_rate: dec!(0.05),
            is_floating: false,
            base_rate: None,
            spread: None,
            amortisation: AmortisationType::Bullet,
            maturity_years: 5,
            pik_rate: None,
            seniority: 1,
            commitment_fee: None,
            is_revolver: false,
        }
    }

    #[test]
    fn test_bullet_schedule() {
        let result = build_debt_schedule(&bullet_input()).unwrap();
        let sched = &result.result;
        assert_eq!(sched.periods.len(), 5);

        // No repayment in years 1-4
        for p in &sched.periods[..4] {
            assert_eq!(p.scheduled_repayment, Decimal::ZERO);
            assert_eq!(p.closing_balance, dec!(1000));
        }

        // Full repayment in year 5
        let last = &sched.periods[4];
        assert_eq!(last.scheduled_repayment, dec!(1000));
        assert_eq!(last.closing_balance, Decimal::ZERO);

        // Total interest = 1000 * 0.05 * 5 = 250
        assert_eq!(sched.total_interest_paid, dec!(250));
        assert_eq!(sched.total_principal_paid, dec!(1000));
    }

    #[test]
    fn test_straight_line_schedule() {
        let mut input = bullet_input();
        input.amortisation = AmortisationType::StraightLine(dec!(0.20)); // 20% per year
        input.maturity_years = 5;

        let result = build_debt_schedule(&input).unwrap();
        let sched = &result.result;

        // Year 1: repay 200 of 1000
        assert_eq!(sched.periods[0].scheduled_repayment, dec!(200));
        assert_eq!(sched.periods[0].closing_balance, dec!(800));

        // Year 2: repay 200
        assert_eq!(sched.periods[1].scheduled_repayment, dec!(200));

        // Total principal = 1000
        assert_eq!(sched.total_principal_paid, dec!(1000));
    }

    #[test]
    fn test_pik_interest() {
        let mut input = bullet_input();
        input.pik_rate = Some(dec!(0.02));
        input.maturity_years = 2;

        let result = build_debt_schedule(&input).unwrap();
        let sched = &result.result;

        // Year 1: PIK = 1000 * 0.02 = 20, balance becomes 1020
        assert_eq!(sched.periods[0].pik_interest, dec!(20));
        assert_eq!(sched.periods[0].closing_balance, dec!(1020));

        // Year 2: opening = 1020, PIK = 1020 * 0.02 = 20.4
        assert_eq!(sched.periods[1].opening_balance, dec!(1020));
        assert_eq!(sched.periods[1].pik_interest, dec!(20.40));
        // Bullet repays full balance: 1020 + 20.4 = 1040.4
        assert_eq!(sched.periods[1].closing_balance, Decimal::ZERO);
    }

    #[test]
    fn test_floating_rate() {
        let mut input = bullet_input();
        input.is_floating = true;
        input.base_rate = Some(dec!(0.03));
        input.spread = Some(dec!(0.02));
        input.maturity_years = 1;

        let result = build_debt_schedule(&input).unwrap();
        let sched = &result.result;

        // Effective rate = 0.03 + 0.02 = 0.05 => interest = 50
        assert_eq!(sched.periods[0].interest, dec!(50));
    }

    #[test]
    fn test_zero_amount_error() {
        let mut input = bullet_input();
        input.amount = Decimal::ZERO;
        assert!(build_debt_schedule(&input).is_err());
    }

    #[test]
    fn test_zero_maturity_error() {
        let mut input = bullet_input();
        input.maturity_years = 0;
        assert!(build_debt_schedule(&input).is_err());
    }
}
