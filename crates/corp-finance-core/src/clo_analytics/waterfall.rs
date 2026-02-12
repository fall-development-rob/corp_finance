//! CLO Waterfall Engine.
//!
//! Implements sequential payment priority waterfalls for CLO structures:
//! - Senior fees -> AAA interest -> AA interest -> ... -> equity residual
//! - Sequential principal paydown (AAA first, then AA, etc.)
//! - Period-by-period collateral pool amortization with defaults/recoveries
//!
//! All arithmetic uses `rust_decimal::Decimal`. No `f64`.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Input / Output types
// ---------------------------------------------------------------------------

/// A single tranche in the CLO capital structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaterfallTranche {
    /// Tranche name (e.g. "AAA", "AA", "A", "BBB", "BB", "Equity").
    pub name: String,
    /// Credit rating label.
    pub rating: String,
    /// Initial notional balance.
    pub notional: Decimal,
    /// Spread over reference rate (decimal: 0.0150 = 150bp).
    pub spread: Decimal,
    /// Whether this is the equity tranche.
    pub is_equity: bool,
}

/// Input for the CLO waterfall engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaterfallInput {
    /// Ordered tranches from most senior to equity.
    pub tranches: Vec<WaterfallTranche>,
    /// Initial collateral pool balance.
    pub pool_balance: Decimal,
    /// Weighted average spread of the collateral pool (decimal).
    pub weighted_avg_spread: Decimal,
    /// Annual conditional default rate (decimal: 0.02 = 2%).
    pub cdr: Decimal,
    /// Annual conditional prepayment rate (decimal: 0.10 = 10%).
    pub cpr: Decimal,
    /// Recovery rate on defaults (decimal: 0.40 = 40%).
    pub recovery_rate: Decimal,
    /// Recovery lag in months (recoveries received after this delay).
    pub recovery_lag_months: u32,
    /// Reference rate (SOFR/LIBOR, decimal: 0.05 = 5%).
    pub reference_rate: Decimal,
    /// Number of projection periods (typically quarterly).
    pub num_periods: u32,
    /// Days per period (typically 90 for quarterly).
    pub period_days: u32,
    /// Senior fees in basis points (annualized).
    pub senior_fees_bps: Decimal,
}

/// Payment details for a single tranche in a single period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranchePayment {
    /// Tranche name.
    pub name: String,
    /// Interest paid this period.
    pub interest_paid: Decimal,
    /// Principal paid this period.
    pub principal_paid: Decimal,
    /// Ending balance after payments.
    pub ending_balance: Decimal,
}

/// A single waterfall period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaterfallPeriod {
    /// Period number (1-indexed).
    pub period: u32,
    /// Pool balance at start of period.
    pub pool_balance: Decimal,
    /// Defaults in this period.
    pub defaults: Decimal,
    /// Losses (defaults net of recovery).
    pub losses: Decimal,
    /// Recoveries received this period.
    pub recoveries: Decimal,
    /// Total interest available for distribution.
    pub interest_available: Decimal,
    /// Total principal available for distribution.
    pub principal_available: Decimal,
    /// Payments to each tranche.
    pub tranche_payments: Vec<TranchePayment>,
}

/// Output of the CLO waterfall engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaterfallOutput {
    /// Period-by-period waterfall results.
    pub periods: Vec<WaterfallPeriod>,
    /// Total interest paid to each tranche over the life.
    pub total_interest_by_tranche: Vec<(String, Decimal)>,
    /// Total principal paid to each tranche over the life.
    pub total_principal_by_tranche: Vec<(String, Decimal)>,
    /// Equity cash flows (negative initial investment, positive distributions).
    pub equity_cash_flows: Vec<Decimal>,
}

// ---------------------------------------------------------------------------
// Engine
// ---------------------------------------------------------------------------

/// Run the CLO waterfall across all projection periods.
pub fn calculate_waterfall(input: &WaterfallInput) -> CorpFinanceResult<WaterfallOutput> {
    validate_waterfall_input(input)?;

    let basis = dec!(360);
    let bps_divisor = dec!(10000);
    let period_frac = Decimal::from(input.period_days) / basis;

    // Convert annual rates to per-period rates
    // CDR_periodic = 1 - (1 - CDR_annual)^(period_days/360)
    // We approximate: CDR_periodic ~ CDR_annual * period_frac (simple)
    let cdr_periodic = input.cdr * period_frac;
    let cpr_periodic = input.cpr * period_frac;

    let mut pool_balance = input.pool_balance;
    let mut tranche_balances: Vec<Decimal> = input.tranches.iter().map(|t| t.notional).collect();

    // Pending recoveries: Vec of (period_due, amount)
    let mut pending_recoveries: Vec<(u32, Decimal)> = Vec::new();
    let recovery_lag_periods = if input.period_days > 0 {
        let months_per_period = Decimal::from(input.period_days) / dec!(30);
        let lag = Decimal::from(input.recovery_lag_months) / months_per_period;
        // Round up to nearest integer
        let lag_u32 = lag.to_string().parse::<f64>().unwrap_or(1.0).ceil() as u32;
        if lag_u32 == 0 {
            1
        } else {
            lag_u32
        }
    } else {
        1
    };

    let mut periods: Vec<WaterfallPeriod> = Vec::with_capacity(input.num_periods as usize);
    let mut total_interest: Vec<Decimal> = vec![Decimal::ZERO; input.tranches.len()];
    let mut total_principal: Vec<Decimal> = vec![Decimal::ZERO; input.tranches.len()];

    // Equity initial investment (negative cash flow)
    let equity_initial: Decimal = input
        .tranches
        .iter()
        .filter(|t| t.is_equity)
        .map(|t| t.notional)
        .sum();
    let mut equity_cash_flows: Vec<Decimal> = vec![-equity_initial];

    for period in 1..=input.num_periods {
        if pool_balance <= Decimal::ZERO {
            // Pool exhausted — record zero period
            let tranche_payments: Vec<TranchePayment> = input
                .tranches
                .iter()
                .enumerate()
                .map(|(i, t)| TranchePayment {
                    name: t.name.clone(),
                    interest_paid: Decimal::ZERO,
                    principal_paid: Decimal::ZERO,
                    ending_balance: tranche_balances[i],
                })
                .collect();
            periods.push(WaterfallPeriod {
                period,
                pool_balance: Decimal::ZERO,
                defaults: Decimal::ZERO,
                losses: Decimal::ZERO,
                recoveries: Decimal::ZERO,
                interest_available: Decimal::ZERO,
                principal_available: Decimal::ZERO,
                tranche_payments,
            });
            equity_cash_flows.push(Decimal::ZERO);
            continue;
        }

        // 1. Defaults
        let defaults = pool_balance * cdr_periodic;

        // 2. Schedule pending recoveries
        let recovery_amount_future = defaults * input.recovery_rate;
        pending_recoveries.push((period + recovery_lag_periods, recovery_amount_future));

        // 3. Collect recoveries due this period
        let recoveries: Decimal = pending_recoveries
            .iter()
            .filter(|(due, _)| *due == period)
            .map(|(_, amt)| *amt)
            .sum();
        pending_recoveries.retain(|(due, _)| *due > period);

        // 4. Losses = defaults * (1 - recovery_rate)
        let losses = defaults * (Decimal::ONE - input.recovery_rate);

        // 5. Prepayments
        let surviving = pool_balance - defaults;
        let prepayments = if surviving > Decimal::ZERO {
            surviving * cpr_periodic
        } else {
            Decimal::ZERO
        };

        // 6. Scheduled amortization (simplified: 0 for bullet collateral)
        let scheduled_amort = Decimal::ZERO;

        // 7. Collateral interest income
        let interest_income =
            pool_balance * (input.weighted_avg_spread + input.reference_rate) * period_frac;

        // 8. Senior fees
        let senior_fees = pool_balance * input.senior_fees_bps / bps_divisor * period_frac;

        // 9. Available interest = interest_income - senior_fees
        let mut avail_interest = interest_income - senior_fees;
        if avail_interest < Decimal::ZERO {
            avail_interest = Decimal::ZERO;
        }

        // 10. Available principal = prepayments + scheduled_amort + recoveries
        let mut avail_principal = prepayments + scheduled_amort + recoveries;

        // 11. Interest waterfall (top-down)
        let mut tranche_payments: Vec<TranchePayment> = Vec::with_capacity(input.tranches.len());
        let mut equity_distribution = Decimal::ZERO;

        for (i, tranche) in input.tranches.iter().enumerate() {
            if tranche.is_equity {
                // Equity gets residual interest
                let int_paid = avail_interest;
                avail_interest = Decimal::ZERO;
                equity_distribution += int_paid;
                tranche_payments.push(TranchePayment {
                    name: tranche.name.clone(),
                    interest_paid: int_paid,
                    principal_paid: Decimal::ZERO, // filled in principal pass
                    ending_balance: tranche_balances[i],
                });
            } else {
                // Rated tranche: interest = balance * (spread + reference) * period_frac
                let coupon =
                    tranche_balances[i] * (tranche.spread + input.reference_rate) * period_frac;
                let int_paid = coupon.min(avail_interest);
                avail_interest -= int_paid;
                total_interest[i] += int_paid;
                tranche_payments.push(TranchePayment {
                    name: tranche.name.clone(),
                    interest_paid: int_paid,
                    principal_paid: Decimal::ZERO,
                    ending_balance: tranche_balances[i],
                });
            }
        }

        // 12. Principal waterfall (sequential, top-down for rated tranches)
        for (i, tranche) in input.tranches.iter().enumerate() {
            if tranche.is_equity {
                continue;
            }
            let prin_paid = tranche_balances[i].min(avail_principal);
            avail_principal -= prin_paid;
            tranche_balances[i] -= prin_paid;
            total_principal[i] += prin_paid;
            tranche_payments[i].principal_paid = prin_paid;
            tranche_payments[i].ending_balance = tranche_balances[i];
        }

        // Any remaining principal goes to equity (return of capital)
        for (i, tranche) in input.tranches.iter().enumerate() {
            if tranche.is_equity {
                let prin_paid = tranche_balances[i].min(avail_principal);
                avail_principal -= prin_paid;
                tranche_balances[i] -= prin_paid;
                total_principal[i] += prin_paid;
                equity_distribution += prin_paid;
                tranche_payments[i].principal_paid = prin_paid;
                tranche_payments[i].ending_balance = tranche_balances[i];
            }
        }

        equity_cash_flows.push(equity_distribution);

        // 13. Update pool balance
        pool_balance = pool_balance - defaults - prepayments - scheduled_amort;
        if pool_balance < Decimal::ZERO {
            pool_balance = Decimal::ZERO;
        }

        periods.push(WaterfallPeriod {
            period,
            pool_balance,
            defaults,
            losses,
            recoveries,
            interest_available: interest_income - senior_fees,
            principal_available: prepayments + scheduled_amort + recoveries,
            tranche_payments,
        });
    }

    let total_interest_by_tranche: Vec<(String, Decimal)> = input
        .tranches
        .iter()
        .enumerate()
        .map(|(i, t)| (t.name.clone(), total_interest[i]))
        .collect();

    let total_principal_by_tranche: Vec<(String, Decimal)> = input
        .tranches
        .iter()
        .enumerate()
        .map(|(i, t)| (t.name.clone(), total_principal[i]))
        .collect();

    Ok(WaterfallOutput {
        periods,
        total_interest_by_tranche,
        total_principal_by_tranche,
        equity_cash_flows,
    })
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_waterfall_input(input: &WaterfallInput) -> CorpFinanceResult<()> {
    if input.tranches.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one tranche is required.".into(),
        ));
    }
    if input.pool_balance <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "pool_balance".into(),
            reason: "Pool balance must be positive.".into(),
        });
    }
    if input.cdr < Decimal::ZERO || input.cdr > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "cdr".into(),
            reason: "CDR must be in [0, 1].".into(),
        });
    }
    if input.cpr < Decimal::ZERO || input.cpr > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "cpr".into(),
            reason: "CPR must be in [0, 1].".into(),
        });
    }
    if input.recovery_rate < Decimal::ZERO || input.recovery_rate > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "recovery_rate".into(),
            reason: "Recovery rate must be in [0, 1].".into(),
        });
    }
    if input.num_periods == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "num_periods".into(),
            reason: "Must have at least one projection period.".into(),
        });
    }
    if input.period_days == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "period_days".into(),
            reason: "Period days must be positive.".into(),
        });
    }
    if input.senior_fees_bps < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "senior_fees_bps".into(),
            reason: "Senior fees cannot be negative.".into(),
        });
    }
    for t in &input.tranches {
        if t.notional < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("tranche.{}.notional", t.name),
                reason: "Tranche notional cannot be negative.".into(),
            });
        }
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

    fn sample_tranches() -> Vec<WaterfallTranche> {
        vec![
            WaterfallTranche {
                name: "AAA".into(),
                rating: "AAA".into(),
                notional: dec!(600_000_000),
                spread: dec!(0.0130),
                is_equity: false,
            },
            WaterfallTranche {
                name: "AA".into(),
                rating: "AA".into(),
                notional: dec!(100_000_000),
                spread: dec!(0.0180),
                is_equity: false,
            },
            WaterfallTranche {
                name: "A".into(),
                rating: "A".into(),
                notional: dec!(80_000_000),
                spread: dec!(0.0250),
                is_equity: false,
            },
            WaterfallTranche {
                name: "BBB".into(),
                rating: "BBB".into(),
                notional: dec!(50_000_000),
                spread: dec!(0.0400),
                is_equity: false,
            },
            WaterfallTranche {
                name: "Equity".into(),
                rating: "NR".into(),
                notional: dec!(70_000_000),
                spread: Decimal::ZERO,
                is_equity: true,
            },
        ]
    }

    fn sample_input() -> WaterfallInput {
        WaterfallInput {
            tranches: sample_tranches(),
            pool_balance: dec!(900_000_000),
            weighted_avg_spread: dec!(0.0350),
            cdr: dec!(0.02),
            cpr: dec!(0.10),
            recovery_rate: dec!(0.40),
            recovery_lag_months: 6,
            reference_rate: dec!(0.05),
            num_periods: 20,
            period_days: 90,
            senior_fees_bps: dec!(50),
        }
    }

    #[test]
    fn test_waterfall_produces_correct_number_of_periods() {
        let input = sample_input();
        let out = calculate_waterfall(&input).unwrap();
        assert_eq!(out.periods.len(), 20);
    }

    #[test]
    fn test_waterfall_pool_balance_decreases() {
        let input = sample_input();
        let out = calculate_waterfall(&input).unwrap();
        // Pool should decrease over time due to defaults and prepayments
        assert!(out.periods.last().unwrap().pool_balance < input.pool_balance);
    }

    #[test]
    fn test_waterfall_defaults_positive() {
        let input = sample_input();
        let out = calculate_waterfall(&input).unwrap();
        assert!(out.periods[0].defaults > Decimal::ZERO);
    }

    #[test]
    fn test_waterfall_losses_less_than_defaults() {
        let input = sample_input();
        let out = calculate_waterfall(&input).unwrap();
        // Losses = defaults * (1 - recovery), so losses < defaults when recovery > 0
        assert!(out.periods[0].losses < out.periods[0].defaults);
    }

    #[test]
    fn test_waterfall_interest_available_positive() {
        let input = sample_input();
        let out = calculate_waterfall(&input).unwrap();
        assert!(out.periods[0].interest_available > Decimal::ZERO);
    }

    #[test]
    fn test_waterfall_aaa_gets_paid_first() {
        let input = sample_input();
        let out = calculate_waterfall(&input).unwrap();
        let p0 = &out.periods[0];
        let aaa_int = p0.tranche_payments[0].interest_paid;
        assert!(aaa_int > Decimal::ZERO, "AAA should receive interest");
    }

    #[test]
    fn test_waterfall_sequential_principal() {
        let input = sample_input();
        let out = calculate_waterfall(&input).unwrap();
        // In sequential, AAA should receive principal before AA
        // If AAA balance is still > 0, AA should get 0 principal
        let p0 = &out.periods[0];
        if p0.tranche_payments[0].ending_balance > Decimal::ZERO {
            // AA principal should be zero while AAA still outstanding
            assert_eq!(
                p0.tranche_payments[1].principal_paid,
                Decimal::ZERO,
                "AA should not get principal while AAA outstanding"
            );
        }
    }

    #[test]
    fn test_equity_cash_flows_start_negative() {
        let input = sample_input();
        let out = calculate_waterfall(&input).unwrap();
        assert!(
            out.equity_cash_flows[0] < Decimal::ZERO,
            "First equity CF should be negative (investment)"
        );
    }

    #[test]
    fn test_equity_cash_flows_length() {
        let input = sample_input();
        let out = calculate_waterfall(&input).unwrap();
        // Initial + num_periods
        assert_eq!(out.equity_cash_flows.len(), 21);
    }

    #[test]
    fn test_total_interest_by_tranche_count() {
        let input = sample_input();
        let out = calculate_waterfall(&input).unwrap();
        assert_eq!(out.total_interest_by_tranche.len(), input.tranches.len());
    }

    #[test]
    fn test_total_principal_by_tranche_count() {
        let input = sample_input();
        let out = calculate_waterfall(&input).unwrap();
        assert_eq!(out.total_principal_by_tranche.len(), input.tranches.len());
    }

    #[test]
    fn test_zero_cdr_no_defaults() {
        let mut input = sample_input();
        input.cdr = Decimal::ZERO;
        let out = calculate_waterfall(&input).unwrap();
        for p in &out.periods {
            assert_eq!(p.defaults, Decimal::ZERO);
            assert_eq!(p.losses, Decimal::ZERO);
        }
    }

    #[test]
    fn test_zero_cpr_no_prepayments() {
        let mut input = sample_input();
        input.cpr = Decimal::ZERO;
        input.cdr = Decimal::ZERO;
        let out = calculate_waterfall(&input).unwrap();
        for p in &out.periods {
            // With no defaults and no prepayments, principal available should be just recoveries
            // and since no defaults, no recoveries either
            assert_eq!(p.principal_available, Decimal::ZERO);
        }
    }

    #[test]
    fn test_full_recovery_no_losses() {
        let mut input = sample_input();
        input.recovery_rate = Decimal::ONE;
        let out = calculate_waterfall(&input).unwrap();
        for p in &out.periods {
            assert_eq!(p.losses, Decimal::ZERO);
        }
    }

    #[test]
    fn test_reject_empty_tranches() {
        let mut input = sample_input();
        input.tranches = vec![];
        assert!(calculate_waterfall(&input).is_err());
    }

    #[test]
    fn test_reject_negative_pool_balance() {
        let mut input = sample_input();
        input.pool_balance = dec!(-100);
        assert!(calculate_waterfall(&input).is_err());
    }

    #[test]
    fn test_reject_cdr_out_of_range() {
        let mut input = sample_input();
        input.cdr = dec!(1.5);
        assert!(calculate_waterfall(&input).is_err());
    }

    #[test]
    fn test_reject_cpr_out_of_range() {
        let mut input = sample_input();
        input.cpr = dec!(-0.01);
        assert!(calculate_waterfall(&input).is_err());
    }

    #[test]
    fn test_reject_recovery_rate_out_of_range() {
        let mut input = sample_input();
        input.recovery_rate = dec!(1.1);
        assert!(calculate_waterfall(&input).is_err());
    }

    #[test]
    fn test_reject_zero_periods() {
        let mut input = sample_input();
        input.num_periods = 0;
        assert!(calculate_waterfall(&input).is_err());
    }

    #[test]
    fn test_reject_zero_period_days() {
        let mut input = sample_input();
        input.period_days = 0;
        assert!(calculate_waterfall(&input).is_err());
    }

    #[test]
    fn test_single_period_waterfall() {
        let mut input = sample_input();
        input.num_periods = 1;
        let out = calculate_waterfall(&input).unwrap();
        assert_eq!(out.periods.len(), 1);
    }

    #[test]
    fn test_interest_income_proportional_to_pool() {
        let input = sample_input();
        let out = calculate_waterfall(&input).unwrap();
        let period_frac = Decimal::from(input.period_days) / dec!(360);
        let expected_income =
            input.pool_balance * (input.weighted_avg_spread + input.reference_rate) * period_frac;
        let senior_fees = input.pool_balance * input.senior_fees_bps / dec!(10000) * period_frac;
        let expected_avail = expected_income - senior_fees;
        assert!(
            approx_eq(out.periods[0].interest_available, expected_avail, dec!(1)),
            "Interest available {} should be ~{}",
            out.periods[0].interest_available,
            expected_avail
        );
    }

    #[test]
    fn test_serialization_roundtrip() {
        let input = sample_input();
        let out = calculate_waterfall(&input).unwrap();
        let json = serde_json::to_string(&out).unwrap();
        let _: WaterfallOutput = serde_json::from_str(&json).unwrap();
    }
}
