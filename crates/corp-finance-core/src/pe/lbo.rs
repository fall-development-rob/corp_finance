use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::pe::debt_schedule::{self, DebtTrancheInput};
use crate::pe::sources_uses::{self, SourcesUsesInput, SourcesUsesOutput};
use crate::types::*;
use crate::CorpFinanceResult;

/// Input for a full LBO model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LboInput {
    // Entry
    /// Enterprise value at acquisition
    pub entry_ev: Money,
    /// Entry-year EBITDA (LTM or projected)
    pub entry_ebitda: Money,

    // Operating projections (per year vectors, length = exit_year)
    /// Revenue growth rate per year (decimal, e.g. 0.05 = 5%)
    pub revenue_growth: Vec<Rate>,
    /// EBITDA margin per year (decimal, e.g. 0.20 = 20%)
    pub ebitda_margin: Vec<Rate>,
    /// Capital expenditures as percentage of revenue
    pub capex_as_pct_revenue: Rate,
    /// Net working capital change as percentage of revenue
    pub nwc_as_pct_revenue: Rate,
    /// Corporate tax rate
    pub tax_rate: Rate,
    /// Depreciation & amortisation as percentage of revenue
    pub da_as_pct_revenue: Rate,

    // Base revenue (year 0)
    /// Revenue in the base year (year 0)
    pub base_revenue: Money,

    // Debt structure — reuse DebtTrancheInput from debt_schedule.rs
    /// Debt tranches in seniority order
    pub tranches: Vec<DebtTrancheInput>,
    /// Sponsor equity contribution
    pub equity_contribution: Money,
    /// Percentage of excess FCF used for mandatory cash sweep repayment
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cash_sweep_pct: Option<Rate>,

    // Exit
    /// Exit year (e.g. 5 for a 5-year hold)
    pub exit_year: u32,
    /// Exit EV/EBITDA multiple
    pub exit_multiple: Multiple,

    // Fees
    /// Transaction advisory fees
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_fees: Option<Money>,
    /// Debt financing/arrangement fees
    #[serde(skip_serializing_if = "Option::is_none")]
    pub financing_fees: Option<Money>,
    /// Management equity rollover
    #[serde(skip_serializing_if = "Option::is_none")]
    pub management_rollover: Option<Money>,
    /// Currency code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<Currency>,
    /// Minimum cash balance to maintain before optional repayments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum_cash: Option<Money>,
}

/// Full LBO model output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LboOutput {
    /// Year-by-year financial projections
    pub projections: Vec<LboYearProjection>,
    /// Per-tranche debt schedules
    pub debt_schedules: Vec<debt_schedule::DebtScheduleOutput>,
    /// Sources & uses of funds at entry
    pub sources_uses: SourcesUsesOutput,
    /// Exit enterprise value
    pub exit_ev: Money,
    /// Exit equity value (exit EV minus exit net debt)
    pub exit_equity_value: Money,
    /// Net debt at exit
    pub exit_net_debt: Money,
    /// Sponsor IRR
    pub irr: Rate,
    /// Multiple on Invested Capital
    pub moic: Multiple,
    /// Cash-on-cash return
    pub cash_on_cash: Multiple,
    /// Entry leverage (entry net debt / entry EBITDA)
    pub entry_leverage: Multiple,
    /// Exit leverage (exit net debt / exit EBITDA)
    pub exit_leverage: Multiple,
}

/// A single year in the LBO projection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LboYearProjection {
    pub year: u32,
    pub revenue: Money,
    pub ebitda: Money,
    pub ebit: Money,
    pub less_interest: Money,
    pub ebt: Money,
    pub tax: Money,
    pub net_income: Money,
    pub plus_da: Money,
    pub less_capex: Money,
    pub less_nwc_change: Money,
    pub fcf_before_debt_service: Money,
    pub mandatory_repayment: Money,
    pub optional_repayment: Money,
    pub total_debt_outstanding: Money,
    pub net_debt: Money,
    pub cash_balance: Money,
    pub equity_value: Money,
}

/// Helper: get a rate from a vector by index, clamping to the last value if
/// the vector is shorter than the requested index.
fn get_rate(rates: &[Rate], index: usize) -> Rate {
    if rates.is_empty() {
        Decimal::ZERO
    } else if index < rates.len() {
        rates[index]
    } else {
        rates[rates.len() - 1]
    }
}

/// Build a complete LBO model from entry through exit.
///
/// This is the top-level orchestrator that ties together sources & uses,
/// debt schedules, operating projections, and exit/returns calculations.
pub fn build_lbo(input: &LboInput) -> CorpFinanceResult<ComputationOutput<LboOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // ─── Validation ──────────────────────────────────────────────────
    if input.entry_ev <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "entry_ev".into(),
            reason: "Entry enterprise value must be positive".into(),
        });
    }
    if input.entry_ebitda <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "entry_ebitda".into(),
            reason: "Entry EBITDA must be positive".into(),
        });
    }
    if input.exit_year == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "exit_year".into(),
            reason: "Exit year must be at least 1".into(),
        });
    }
    if input.tranches.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "tranches".into(),
            reason: "At least one debt tranche is required".into(),
        });
    }
    if input.equity_contribution <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "equity_contribution".into(),
            reason: "Equity contribution must be positive".into(),
        });
    }
    if input.exit_multiple <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "exit_multiple".into(),
            reason: "Exit multiple must be positive".into(),
        });
    }
    if input.base_revenue <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "base_revenue".into(),
            reason: "Base revenue must be positive".into(),
        });
    }

    // ─── Sources & Uses ──────────────────────────────────────────────
    let debt_tranches_for_su: Vec<(String, Money)> = input
        .tranches
        .iter()
        .map(|t| (t.name.clone(), t.amount))
        .collect();

    let su_input = SourcesUsesInput {
        enterprise_value: input.entry_ev,
        equity_contribution: input.equity_contribution,
        debt_tranches: debt_tranches_for_su,
        transaction_fees: input.transaction_fees,
        financing_fees: input.financing_fees,
        management_rollover: input.management_rollover,
    };
    let su_output = sources_uses::build_sources_uses(&su_input)?;
    if !su_output.result.balanced {
        warnings.push("Sources & Uses are not balanced".into());
    }

    // ─── Debt Schedules ──────────────────────────────────────────────
    // Build each tranche schedule up to at least exit_year periods.
    // The debt_schedule module uses each tranche's own maturity_years.
    let mut debt_schedule_outputs: Vec<debt_schedule::DebtScheduleOutput> = Vec::new();
    for tranche in &input.tranches {
        let ds = debt_schedule::build_debt_schedule(tranche)?;
        debt_schedule_outputs.push(ds.result);
        if !ds.warnings.is_empty() {
            for w in ds.warnings {
                warnings.push(format!("Debt schedule ({}): {w}", tranche.name));
            }
        }
    }

    // ─── Year-by-year projection ─────────────────────────────────────
    let total_initial_debt: Money = input.tranches.iter().map(|t| t.amount).sum();
    let minimum_cash = input.minimum_cash.unwrap_or(Decimal::ZERO);
    let cash_sweep_pct = input.cash_sweep_pct.unwrap_or(Decimal::ZERO);

    let mut projections: Vec<LboYearProjection> = Vec::with_capacity(input.exit_year as usize);
    let mut prev_revenue = input.base_revenue;
    let mut cash_balance = Decimal::ZERO;

    // Track remaining balance per tranche (independent of the static debt schedules)
    let mut tranche_balances: Vec<Money> = input.tranches.iter().map(|t| t.amount).collect();

    // NWC tracking: we need the *change* in NWC
    let base_nwc = input.base_revenue * input.nwc_as_pct_revenue;
    let mut prev_nwc = base_nwc;

    for year in 1..=input.exit_year {
        let yr_idx = (year - 1) as usize;

        // Revenue
        let growth = get_rate(&input.revenue_growth, yr_idx);
        let revenue = prev_revenue * (Decimal::ONE + growth);

        // EBITDA
        let margin = get_rate(&input.ebitda_margin, yr_idx);
        let ebitda = revenue * margin;

        // D&A
        let da = revenue * input.da_as_pct_revenue;

        // EBIT
        let ebit = ebitda - da;

        // Interest: sum across all tranches for this year
        let mut total_interest = Decimal::ZERO;
        for (i, tranche) in input.tranches.iter().enumerate() {
            if yr_idx < debt_schedule_outputs[i].periods.len() {
                total_interest += debt_schedule_outputs[i].periods[yr_idx].interest;
            } else {
                // Tranche matured — if balance remains (shouldn't), charge interest
                if tranche_balances[i] > Decimal::ZERO {
                    let eff_rate = if tranche.is_floating {
                        tranche.base_rate.unwrap_or(Decimal::ZERO)
                            + tranche.spread.unwrap_or(tranche.interest_rate)
                    } else {
                        tranche.interest_rate
                    };
                    total_interest += tranche_balances[i] * eff_rate;
                }
            }
        }

        // EBT, Tax, Net Income
        let ebt = ebit - total_interest;
        let tax = if ebt > Decimal::ZERO {
            ebt * input.tax_rate
        } else {
            Decimal::ZERO
        };
        let net_income = ebt - tax;

        // Free Cash Flow before debt service
        let capex = revenue * input.capex_as_pct_revenue;
        let current_nwc = revenue * input.nwc_as_pct_revenue;
        let nwc_change = current_nwc - prev_nwc;

        let fcf_before_debt_service = net_income + da - capex - nwc_change;

        // Mandatory repayment: sum of scheduled repayments from debt schedules
        let mut mandatory_repayment = Decimal::ZERO;
        for (i, _tranche) in input.tranches.iter().enumerate() {
            if yr_idx < debt_schedule_outputs[i].periods.len() {
                let sched_repay = debt_schedule_outputs[i].periods[yr_idx].scheduled_repayment;
                // Cap at remaining tranche balance
                let actual_repay = sched_repay.min(tranche_balances[i]);
                mandatory_repayment += actual_repay;
                tranche_balances[i] -= actual_repay;
            }
        }

        // Cash available after mandatory repayment
        let fcf_after_mandatory = fcf_before_debt_service - mandatory_repayment;

        // Cash sweep (optional repayment)
        let mut optional_repayment = Decimal::ZERO;
        if cash_sweep_pct > Decimal::ZERO && fcf_after_mandatory > Decimal::ZERO {
            let sweep_amount = fcf_after_mandatory * cash_sweep_pct;
            // Apply cash sweep to tranches in reverse seniority (most junior first)
            // but ensure we don't sweep below minimum cash
            let available_for_sweep =
                if cash_balance + fcf_after_mandatory - sweep_amount >= minimum_cash {
                    sweep_amount
                } else {
                    // Only sweep what keeps us above minimum cash
                    let max_sweep =
                        (cash_balance + fcf_after_mandatory - minimum_cash).max(Decimal::ZERO);
                    max_sweep.min(sweep_amount)
                };

            let total_remaining: Money = tranche_balances.iter().sum();
            let actual_sweep = available_for_sweep.min(total_remaining);

            if actual_sweep > Decimal::ZERO {
                // Apply to the most junior tranche first (highest seniority number)
                let mut remaining_sweep = actual_sweep;
                // Sort tranche indices by seniority descending
                let mut indices: Vec<usize> = (0..input.tranches.len()).collect();
                indices.sort_by(|a, b| {
                    input.tranches[*b]
                        .seniority
                        .cmp(&input.tranches[*a].seniority)
                });

                for idx in indices {
                    if remaining_sweep <= Decimal::ZERO {
                        break;
                    }
                    let paydown = remaining_sweep.min(tranche_balances[idx]);
                    tranche_balances[idx] -= paydown;
                    remaining_sweep -= paydown;
                }
                optional_repayment = actual_sweep - remaining_sweep;
            }
        }

        // Update cash balance
        cash_balance += fcf_after_mandatory - optional_repayment;
        if cash_balance < Decimal::ZERO {
            // Negative cash — flag as warning
            warnings.push(format!(
                "Year {year}: negative cash balance of {cash_balance}"
            ));
        }

        // Total debt outstanding
        let total_debt: Money = tranche_balances.iter().sum();

        // Net debt = total debt - cash
        let net_debt = total_debt - cash_balance.max(Decimal::ZERO);

        // Implied equity value at this point (using current EBITDA and exit multiple
        // is speculative; we use EV minus net debt as a running marker)
        let implied_ev = ebitda * input.exit_multiple;
        let equity_value = implied_ev - net_debt;

        projections.push(LboYearProjection {
            year,
            revenue,
            ebitda,
            ebit,
            less_interest: total_interest,
            ebt,
            tax,
            net_income,
            plus_da: da,
            less_capex: capex,
            less_nwc_change: nwc_change,
            fcf_before_debt_service,
            mandatory_repayment,
            optional_repayment,
            total_debt_outstanding: total_debt,
            net_debt,
            cash_balance,
            equity_value,
        });

        prev_revenue = revenue;
        prev_nwc = current_nwc;
    }

    // ─── Exit ────────────────────────────────────────────────────────
    let last = projections
        .last()
        .expect("projections should be non-empty after loop");
    let exit_ebitda = last.ebitda;
    let exit_ev = exit_ebitda * input.exit_multiple;
    let exit_net_debt = last.net_debt;
    let exit_equity_value = exit_ev - exit_net_debt;

    // ─── Returns ─────────────────────────────────────────────────────
    // Cash flow series: [-equity, 0, 0, ..., exit_equity_value]
    let mut cf_series: Vec<Money> = Vec::with_capacity((input.exit_year + 1) as usize);
    cf_series.push(-input.equity_contribution);
    for i in 0..input.exit_year {
        if i == input.exit_year - 1 {
            cf_series.push(exit_equity_value);
        } else {
            cf_series.push(Decimal::ZERO);
        }
    }

    let irr_val = match crate::time_value::irr(&cf_series, dec!(0.10)) {
        Ok(r) => r,
        Err(e) => {
            warnings.push(format!("IRR calculation warning: {e}"));
            Decimal::ZERO
        }
    };

    // MOIC
    let moic = if input.equity_contribution.is_zero() {
        Decimal::ZERO
    } else {
        exit_equity_value / input.equity_contribution
    };

    // Cash-on-cash (same as MOIC in a simple LBO without interim distributions)
    let cash_on_cash = moic;

    // Leverage multiples
    let entry_net_debt = total_initial_debt; // at entry, cash = 0
    let entry_leverage = if input.entry_ebitda.is_zero() {
        Decimal::ZERO
    } else {
        entry_net_debt / input.entry_ebitda
    };

    let exit_leverage = if exit_ebitda.is_zero() {
        warnings.push("Exit EBITDA is zero; exit leverage undefined".into());
        Decimal::ZERO
    } else {
        exit_net_debt / exit_ebitda
    };

    let output = LboOutput {
        projections,
        debt_schedules: debt_schedule_outputs,
        sources_uses: su_output.result,
        exit_ev,
        exit_equity_value,
        exit_net_debt,
        irr: irr_val,
        moic,
        cash_on_cash,
        entry_leverage,
        exit_leverage,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Leveraged Buyout Model",
        &serde_json::json!({
            "entry_ev": input.entry_ev.to_string(),
            "entry_ebitda": input.entry_ebitda.to_string(),
            "exit_year": input.exit_year,
            "exit_multiple": input.exit_multiple.to_string(),
            "equity_contribution": input.equity_contribution.to_string(),
            "num_tranches": input.tranches.len(),
        }),
        warnings,
        elapsed,
        output,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pe::debt_schedule::{AmortisationType, DebtTrancheInput};
    use rust_decimal_macros::dec;

    /// Helper: build a standard 5-year LBO input for tests
    fn standard_lbo_input() -> LboInput {
        LboInput {
            entry_ev: dec!(1000),
            entry_ebitda: dec!(200),
            revenue_growth: vec![dec!(0.05); 5],
            ebitda_margin: vec![dec!(0.20); 5],
            capex_as_pct_revenue: dec!(0.03),
            nwc_as_pct_revenue: dec!(0.05),
            tax_rate: dec!(0.25),
            da_as_pct_revenue: dec!(0.02),
            base_revenue: dec!(1000),
            tranches: vec![DebtTrancheInput {
                name: "Senior Term Loan".into(),
                amount: dec!(600),
                interest_rate: dec!(0.05),
                is_floating: false,
                base_rate: None,
                spread: None,
                amortisation: AmortisationType::StraightLine(dec!(0.05)),
                maturity_years: 7,
                pik_rate: None,
                seniority: 1,
                commitment_fee: None,
                is_revolver: false,
            }],
            equity_contribution: dec!(400),
            cash_sweep_pct: None,
            exit_year: 5,
            exit_multiple: dec!(6.0),
            transaction_fees: None,
            financing_fees: None,
            management_rollover: None,
            currency: None,
            minimum_cash: None,
        }
    }

    #[test]
    fn test_basic_lbo_returns() {
        let input = standard_lbo_input();
        let result = build_lbo(&input).unwrap();
        let out = &result.result;

        // Should produce 5 years of projections
        assert_eq!(out.projections.len(), 5);

        // IRR should be positive for a reasonable LBO
        assert!(
            out.irr > Decimal::ZERO,
            "IRR should be positive, got {}",
            out.irr
        );

        // MOIC should be > 1 (we made money)
        assert!(
            out.moic > Decimal::ONE,
            "MOIC should be > 1, got {}",
            out.moic
        );

        // Exit EV should be positive
        assert!(out.exit_ev > Decimal::ZERO);
        assert!(out.exit_equity_value > Decimal::ZERO);
    }

    #[test]
    fn test_sources_uses_balanced() {
        let mut input = standard_lbo_input();
        // Make S&U balance: EV = 1000, Equity = 400, Debt = 600 => S = 1000, U = 1000
        input.transaction_fees = None;
        input.financing_fees = None;

        let result = build_lbo(&input).unwrap();
        let su = &result.result.sources_uses;

        assert_eq!(su.total_sources, su.total_uses);
        assert!(su.balanced);
    }

    #[test]
    fn test_debt_paydown() {
        let input = standard_lbo_input();
        let result = build_lbo(&input).unwrap();
        let projs = &result.result.projections;

        // Debt should decrease year-over-year with amortisation
        for i in 1..projs.len() {
            assert!(
                projs[i].total_debt_outstanding <= projs[i - 1].total_debt_outstanding,
                "Debt should decrease: year {} has {} but year {} has {}",
                projs[i].year,
                projs[i].total_debt_outstanding,
                projs[i - 1].year,
                projs[i - 1].total_debt_outstanding
            );
        }

        // First year mandatory repayment should be > 0 for straight-line amort
        assert!(
            projs[0].mandatory_repayment > Decimal::ZERO,
            "Mandatory repayment in year 1 should be positive"
        );
    }

    #[test]
    fn test_cash_sweep() {
        let mut input = standard_lbo_input();
        input.cash_sweep_pct = Some(dec!(0.50)); // 50% cash sweep

        let result = build_lbo(&input).unwrap();
        let projs = &result.result.projections;

        // At least some year should have optional repayment > 0
        let has_optional = projs.iter().any(|p| p.optional_repayment > Decimal::ZERO);
        assert!(
            has_optional,
            "With 50% cash sweep, some optional repayment should occur"
        );
    }

    #[test]
    fn test_exit_equity() {
        let input = standard_lbo_input();
        let result = build_lbo(&input).unwrap();
        let out = &result.result;

        let last_proj = out.projections.last().unwrap();
        let expected_exit_ev = last_proj.ebitda * input.exit_multiple;

        assert_eq!(
            out.exit_ev, expected_exit_ev,
            "Exit EV should equal exit EBITDA * exit multiple"
        );

        // exit_equity = exit_ev - exit_net_debt
        let expected_exit_equity = out.exit_ev - out.exit_net_debt;
        assert_eq!(out.exit_equity_value, expected_exit_equity);
    }

    #[test]
    fn test_leverage_decreases() {
        let input = standard_lbo_input();
        let result = build_lbo(&input).unwrap();
        let out = &result.result;

        assert!(
            out.entry_leverage > out.exit_leverage,
            "Entry leverage ({}) should be greater than exit leverage ({})",
            out.entry_leverage,
            out.exit_leverage
        );
    }

    #[test]
    fn test_invalid_zero_ev() {
        let mut input = standard_lbo_input();
        input.entry_ev = Decimal::ZERO;

        let result = build_lbo(&input);
        assert!(result.is_err(), "Zero entry EV should produce an error");
    }

    #[test]
    fn test_no_tranches_error() {
        let mut input = standard_lbo_input();
        input.tranches = vec![];

        let result = build_lbo(&input);
        assert!(result.is_err(), "Empty tranches should produce an error");
    }

    #[test]
    fn test_revenue_growth_projection() {
        let mut input = standard_lbo_input();
        // 10% growth every year, 20% margin, base_revenue = 1000
        input.revenue_growth = vec![dec!(0.10); 5];
        input.ebitda_margin = vec![dec!(0.20); 5];
        input.base_revenue = dec!(1000);

        let result = build_lbo(&input).unwrap();
        let projs = &result.result.projections;

        // Year 1 revenue = 1000 * 1.10 = 1100
        assert_eq!(projs[0].revenue, dec!(1100));
        // Year 1 EBITDA = 1100 * 0.20 = 220
        assert_eq!(projs[0].ebitda, dec!(220));
        // Year 2 revenue = 1100 * 1.10 = 1210
        assert_eq!(projs[1].revenue, dec!(1210));
    }

    #[test]
    fn test_income_statement_mechanics() {
        let mut input = standard_lbo_input();
        input.revenue_growth = vec![dec!(0.0)]; // zero growth for simplicity
        input.ebitda_margin = vec![dec!(0.20)];
        input.base_revenue = dec!(1000);
        input.da_as_pct_revenue = dec!(0.02);
        input.tax_rate = dec!(0.25);
        input.exit_year = 1;

        // Bullet debt so we can predict interest exactly
        input.tranches = vec![DebtTrancheInput {
            name: "Term Loan".into(),
            amount: dec!(600),
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
        }];

        let result = build_lbo(&input).unwrap();
        let p = &result.result.projections[0];

        // Revenue = 1000 (0% growth on 1000 base)
        assert_eq!(p.revenue, dec!(1000));
        // EBITDA = 1000 * 0.20 = 200
        assert_eq!(p.ebitda, dec!(200));
        // D&A = 1000 * 0.02 = 20
        assert_eq!(p.plus_da, dec!(20));
        // EBIT = 200 - 20 = 180
        assert_eq!(p.ebit, dec!(180));
        // Interest = 600 * 0.05 = 30
        assert_eq!(p.less_interest, dec!(30));
        // EBT = 180 - 30 = 150
        assert_eq!(p.ebt, dec!(150));
        // Tax = 150 * 0.25 = 37.5
        assert_eq!(p.tax, dec!(37.5));
        // Net income = 150 - 37.5 = 112.5
        assert_eq!(p.net_income, dec!(112.5));
    }

    #[test]
    fn test_multi_tranche_lbo() {
        let mut input = standard_lbo_input();
        input.entry_ev = dec!(1000);
        input.equity_contribution = dec!(300);
        input.tranches = vec![
            DebtTrancheInput {
                name: "Senior".into(),
                amount: dec!(500),
                interest_rate: dec!(0.04),
                is_floating: false,
                base_rate: None,
                spread: None,
                amortisation: AmortisationType::StraightLine(dec!(0.10)),
                maturity_years: 7,
                pik_rate: None,
                seniority: 1,
                commitment_fee: None,
                is_revolver: false,
            },
            DebtTrancheInput {
                name: "Mezzanine".into(),
                amount: dec!(200),
                interest_rate: dec!(0.08),
                is_floating: false,
                base_rate: None,
                spread: None,
                amortisation: AmortisationType::Bullet,
                maturity_years: 7,
                pik_rate: None,
                seniority: 2,
                commitment_fee: None,
                is_revolver: false,
            },
        ];

        let result = build_lbo(&input).unwrap();
        let out = &result.result;

        // Two debt schedule outputs
        assert_eq!(out.debt_schedules.len(), 2);
        assert_eq!(out.debt_schedules[0].tranche_name, "Senior");
        assert_eq!(out.debt_schedules[1].tranche_name, "Mezzanine");

        // Entry leverage = 700 / 200 = 3.5
        assert_eq!(out.entry_leverage, dec!(700) / input.entry_ebitda);
    }

    #[test]
    fn test_vector_clamping() {
        let mut input = standard_lbo_input();
        // Only provide 2 years of growth/margin data for a 5-year model
        input.revenue_growth = vec![dec!(0.05), dec!(0.03)];
        input.ebitda_margin = vec![dec!(0.20), dec!(0.22)];
        input.exit_year = 5;

        let result = build_lbo(&input).unwrap();
        let projs = &result.result.projections;

        // Years 3-5 should use the last values (0.03 growth, 0.22 margin)
        // Year 3 revenue = year2_revenue * 1.03
        let year2_rev = projs[1].revenue;
        let year3_rev = projs[2].revenue;
        let expected = year2_rev * (Decimal::ONE + dec!(0.03));
        assert_eq!(year3_rev, expected);

        // Year 3 EBITDA margin should be 0.22
        let year3_margin = projs[2].ebitda / projs[2].revenue;
        assert_eq!(year3_margin, dec!(0.22));
    }
}
