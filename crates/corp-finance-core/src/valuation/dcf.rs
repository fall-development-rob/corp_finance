use rust_decimal::Decimal;
use rust_decimal::MathematicalOps;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{
    with_metadata, ComputationOutput, Currency, Money, Multiple, ProjectionPeriod, Rate,
};
use crate::CorpFinanceResult;

use super::wacc::{calculate_wacc, WaccInput};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Method for computing terminal value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TerminalMethod {
    /// Gordon growth model: TV = FCFF_terminal * (1+g) / (WACC - g)
    GordonGrowth,
    /// Exit multiple: TV = EBITDA_terminal * exit_multiple
    ExitMultiple,
    /// Compute both and report; uses Gordon as primary
    Both,
}

/// Input parameters for a Discounted Cash Flow valuation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DcfInput {
    /// Base (Year 0) revenue
    pub base_revenue: Money,
    /// Year-by-year revenue growth rates; length determines explicit forecast period
    /// unless `forecast_years` overrides it.
    pub revenue_growth_rates: Vec<Rate>,
    /// EBITDA margin as a fraction of revenue
    pub ebitda_margin: Rate,
    /// EBIT margin (if provided, used instead of deriving EBIT from EBITDA - D&A)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ebit_margin: Option<Rate>,
    /// Depreciation & amortisation as a percentage of revenue
    #[serde(skip_serializing_if = "Option::is_none")]
    pub da_as_pct_revenue: Option<Rate>,
    /// Capital expenditure as a percentage of revenue
    pub capex_as_pct_revenue: Rate,
    /// Change in net working capital as a percentage of revenue
    pub nwc_as_pct_revenue: Rate,
    /// Marginal tax rate on operating income
    pub tax_rate: Rate,
    /// Weighted average cost of capital (discount rate)
    pub wacc: Rate,
    /// If provided, WACC is computed from this input (overrides `wacc` field)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wacc_input: Option<WaccInput>,
    /// Terminal value method
    pub terminal_method: TerminalMethod,
    /// Terminal / perpetuity growth rate (required for GordonGrowth / Both)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal_growth_rate: Option<Rate>,
    /// Exit EBITDA multiple (required for ExitMultiple / Both)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal_exit_multiple: Option<Multiple>,
    /// Reporting currency
    pub currency: Currency,
    /// Number of explicit forecast years (default: length of growth_rates or 10)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub forecast_years: Option<u32>,
    /// Use mid-year convention for discounting (default: true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mid_year_convention: Option<bool>,
    /// Net debt for equity bridge (debt minus cash)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub net_debt: Option<Money>,
    /// Minority interest to subtract in equity bridge
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minority_interest: Option<Money>,
    /// Diluted shares outstanding for per-share value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shares_outstanding: Option<Decimal>,
}

/// Projection for a single year of the DCF model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DcfYearProjection {
    pub period: ProjectionPeriod,
    pub revenue: Money,
    pub ebitda: Money,
    pub ebit: Money,
    pub nopat: Money,
    pub plus_da: Money,
    pub less_capex: Money,
    pub less_nwc_change: Money,
    pub fcff: Money,
    pub discount_factor: Rate,
    pub pv_fcff: Money,
}

/// Output of the DCF valuation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DcfOutput {
    /// Year-by-year projections
    pub projections: Vec<DcfYearProjection>,
    /// Terminal value via Gordon growth (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal_value_gordon: Option<Money>,
    /// Terminal value via exit multiple (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal_value_exit: Option<Money>,
    /// Terminal value used in the primary valuation
    pub terminal_value_used: Money,
    /// Sum of present values of explicit-period FCFFs
    pub pv_of_fcff: Money,
    /// Present value of terminal value
    pub pv_of_terminal: Money,
    /// Enterprise value = PV(FCFFs) + PV(TV)
    pub enterprise_value: Money,
    /// Equity value = EV - net_debt - minority_interest (if bridge data provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub equity_value: Option<Money>,
    /// Per-share equity value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub equity_value_per_share: Option<Money>,
    /// Implied EV/EBITDA exit multiple from the terminal value used
    pub implied_exit_multiple: Multiple,
    /// Terminal value as a percentage of enterprise value
    pub terminal_value_pct: Rate,
    /// WACC used in the calculation
    pub wacc_used: Rate,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Run a 2-stage FCFF DCF valuation.
pub fn calculate_dcf(input: &DcfInput) -> CorpFinanceResult<ComputationOutput<DcfOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // --- Resolve WACC ---
    let wacc = resolve_wacc(input, &mut warnings)?;

    // --- Validate ---
    validate_dcf_input(input, wacc)?;

    let mid_year = input.mid_year_convention.unwrap_or(true);
    let n_years = resolve_forecast_years(input);

    // --- Project cash flows ---
    let projections = build_projections(input, n_years, wacc, mid_year)?;

    let pv_of_fcff: Money = projections.iter().map(|p| p.pv_fcff).sum();
    let last = projections.last().ok_or_else(|| {
        CorpFinanceError::InsufficientData("No projection years generated".into())
    })?;

    // --- Terminal value ---
    let (tv_gordon, tv_exit, tv_used) = compute_terminal_values(input, last, wacc, &mut warnings)?;

    // --- Discount TV to present ---
    let tv_discount_period = Decimal::from(n_years);
    let tv_discount_factor = Decimal::ONE / (Decimal::ONE + wacc).powd(tv_discount_period);
    let pv_of_terminal = tv_used * tv_discount_factor;

    // --- Enterprise value ---
    let enterprise_value = pv_of_fcff + pv_of_terminal;

    // --- Terminal value percentage warning ---
    let tv_pct = if enterprise_value.is_zero() {
        Decimal::ZERO
    } else {
        pv_of_terminal / enterprise_value
    };
    if tv_pct > dec!(0.75) {
        warnings.push(format!(
            "Terminal value represents {:.1}% of enterprise value; consider extending the explicit forecast period",
            tv_pct * dec!(100)
        ));
    }

    // --- Implied exit multiple ---
    let implied_exit_multiple = if last.ebitda.is_zero() {
        Decimal::ZERO
    } else {
        tv_used / last.ebitda
    };

    // --- Equity bridge ---
    let (equity_value, equity_value_per_share) = compute_equity_bridge(input, enterprise_value)?;

    let output = DcfOutput {
        projections,
        terminal_value_gordon: tv_gordon,
        terminal_value_exit: tv_exit,
        terminal_value_used: tv_used,
        pv_of_fcff,
        pv_of_terminal,
        enterprise_value,
        equity_value,
        equity_value_per_share,
        implied_exit_multiple,
        terminal_value_pct: tv_pct,
        wacc_used: wacc,
    };

    let elapsed = start.elapsed().as_micros() as u64;

    Ok(with_metadata(
        "2-Stage FCFF DCF (WACC-based)",
        input,
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn resolve_wacc(input: &DcfInput, warnings: &mut Vec<String>) -> CorpFinanceResult<Rate> {
    if let Some(ref wacc_input) = input.wacc_input {
        let wacc_out = calculate_wacc(wacc_input)?;
        for w in &wacc_out.warnings {
            warnings.push(format!("[WACC] {w}"));
        }
        Ok(wacc_out.result.wacc)
    } else {
        Ok(input.wacc)
    }
}

fn validate_dcf_input(input: &DcfInput, wacc: Rate) -> CorpFinanceResult<()> {
    if wacc <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "wacc".into(),
            reason: "WACC must be positive".into(),
        });
    }
    if input.base_revenue <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "base_revenue".into(),
            reason: "Base revenue must be positive".into(),
        });
    }
    if input.ebitda_margin <= Decimal::ZERO || input.ebitda_margin >= Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "ebitda_margin".into(),
            reason: "EBITDA margin must be between 0 and 1 (exclusive)".into(),
        });
    }
    if input.tax_rate < Decimal::ZERO || input.tax_rate > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "tax_rate".into(),
            reason: "Tax rate must be between 0 and 1".into(),
        });
    }

    // Terminal growth must be less than WACC (Gordon growth model constraint)
    if let Some(g) = input.terminal_growth_rate {
        if g >= wacc {
            return Err(CorpFinanceError::FinancialImpossibility(format!(
                "Terminal growth rate ({g}) must be less than WACC ({wacc}) for the Gordon growth model"
            )));
        }
    }

    // Validate terminal method has required inputs
    match input.terminal_method {
        TerminalMethod::GordonGrowth => {
            if input.terminal_growth_rate.is_none() {
                return Err(CorpFinanceError::InvalidInput {
                    field: "terminal_growth_rate".into(),
                    reason: "Required for GordonGrowth terminal method".into(),
                });
            }
        }
        TerminalMethod::ExitMultiple => {
            if input.terminal_exit_multiple.is_none() {
                return Err(CorpFinanceError::InvalidInput {
                    field: "terminal_exit_multiple".into(),
                    reason: "Required for ExitMultiple terminal method".into(),
                });
            }
        }
        TerminalMethod::Both => {
            if input.terminal_growth_rate.is_none() {
                return Err(CorpFinanceError::InvalidInput {
                    field: "terminal_growth_rate".into(),
                    reason: "Required for Both terminal method".into(),
                });
            }
            if input.terminal_exit_multiple.is_none() {
                return Err(CorpFinanceError::InvalidInput {
                    field: "terminal_exit_multiple".into(),
                    reason: "Required for Both terminal method".into(),
                });
            }
        }
    }

    Ok(())
}

fn resolve_forecast_years(input: &DcfInput) -> u32 {
    input.forecast_years.unwrap_or_else(|| {
        let n = input.revenue_growth_rates.len() as u32;
        if n > 0 {
            n
        } else {
            10
        }
    })
}

fn build_projections(
    input: &DcfInput,
    n_years: u32,
    wacc: Rate,
    mid_year: bool,
) -> CorpFinanceResult<Vec<DcfYearProjection>> {
    let mut projections = Vec::with_capacity(n_years as usize);
    let mut prev_revenue = input.base_revenue;
    let mut prev_nwc = input.base_revenue * input.nwc_as_pct_revenue;

    for year_idx in 0..n_years {
        let year_num = year_idx + 1;
        let growth = growth_rate_for_year(input, year_idx);
        let revenue = prev_revenue * (Decimal::ONE + growth);
        let ebitda = revenue * input.ebitda_margin;

        // EBIT: use explicit margin if provided, otherwise derive from EBITDA - D&A
        let da = revenue * input.da_as_pct_revenue.unwrap_or(Decimal::ZERO);
        let ebit = if let Some(ebit_margin) = input.ebit_margin {
            revenue * ebit_margin
        } else {
            ebitda - da
        };

        let nopat = ebit * (Decimal::ONE - input.tax_rate);
        let capex = revenue * input.capex_as_pct_revenue;
        let current_nwc = revenue * input.nwc_as_pct_revenue;
        let nwc_change = current_nwc - prev_nwc;

        // FCFF = NOPAT + D&A - CapEx - Delta NWC
        let plus_da = da;
        let fcff = nopat + plus_da - capex - nwc_change;

        // Discount factor
        let discount_period = if mid_year {
            Decimal::from(year_num) - dec!(0.5)
        } else {
            Decimal::from(year_num)
        };
        let discount_factor = Decimal::ONE / (Decimal::ONE + wacc).powd(discount_period);
        let pv_fcff = fcff * discount_factor;

        projections.push(DcfYearProjection {
            period: ProjectionPeriod {
                year: year_num as i32,
                label: format!("Year {year_num}"),
                is_terminal: false,
            },
            revenue,
            ebitda,
            ebit,
            nopat,
            plus_da,
            less_capex: capex,
            less_nwc_change: nwc_change,
            fcff,
            discount_factor,
            pv_fcff,
        });

        prev_revenue = revenue;
        prev_nwc = current_nwc;
    }

    Ok(projections)
}

/// Get the growth rate for a given year index. If `revenue_growth_rates` is shorter
/// than the forecast period, the last rate is carried forward.
fn growth_rate_for_year(input: &DcfInput, year_idx: u32) -> Rate {
    let idx = year_idx as usize;
    if idx < input.revenue_growth_rates.len() {
        input.revenue_growth_rates[idx]
    } else if let Some(&last) = input.revenue_growth_rates.last() {
        last
    } else {
        Decimal::ZERO
    }
}

fn compute_terminal_values(
    input: &DcfInput,
    last_year: &DcfYearProjection,
    wacc: Rate,
    warnings: &mut Vec<String>,
) -> CorpFinanceResult<(Option<Money>, Option<Money>, Money)> {
    let tv_gordon = match input.terminal_method {
        TerminalMethod::GordonGrowth | TerminalMethod::Both => {
            let g = input.terminal_growth_rate.unwrap(); // validated above
            let denom = wacc - g;
            if denom <= Decimal::ZERO {
                return Err(CorpFinanceError::FinancialImpossibility(
                    "WACC must exceed terminal growth rate".into(),
                ));
            }
            let tv = last_year.fcff * (Decimal::ONE + g) / denom;
            Some(tv)
        }
        TerminalMethod::ExitMultiple => None,
    };

    let tv_exit = match input.terminal_method {
        TerminalMethod::ExitMultiple | TerminalMethod::Both => {
            let multiple = input.terminal_exit_multiple.unwrap(); // validated above
            let tv = last_year.ebitda * multiple;
            Some(tv)
        }
        TerminalMethod::GordonGrowth => None,
    };

    // Determine which TV to use
    let tv_used = match input.terminal_method {
        TerminalMethod::GordonGrowth => tv_gordon.unwrap(),
        TerminalMethod::ExitMultiple => tv_exit.unwrap(),
        TerminalMethod::Both => {
            let g = tv_gordon.unwrap();
            let e = tv_exit.unwrap();
            if g > Decimal::ZERO && e > Decimal::ZERO {
                let diff_pct = ((g - e) / g).abs();
                if diff_pct > dec!(0.25) {
                    warnings.push(format!(
                        "Gordon TV ({g}) and Exit Multiple TV ({e}) differ by {:.1}%; review assumptions",
                        diff_pct * dec!(100)
                    ));
                }
            }
            // Use Gordon as primary when Both
            g
        }
    };

    Ok((tv_gordon, tv_exit, tv_used))
}

fn compute_equity_bridge(
    input: &DcfInput,
    enterprise_value: Money,
) -> CorpFinanceResult<(Option<Money>, Option<Money>)> {
    let equity_value = match (input.net_debt, input.minority_interest) {
        (Some(nd), Some(mi)) => Some(enterprise_value - nd - mi),
        (Some(nd), None) => Some(enterprise_value - nd),
        (None, Some(mi)) => Some(enterprise_value - mi),
        (None, None) => None,
    };

    let equity_per_share = match (equity_value, input.shares_outstanding) {
        (Some(ev), Some(shares)) if shares > Decimal::ZERO => Some(ev / shares),
        _ => None,
    };

    Ok((equity_value, equity_per_share))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn sample_dcf_input() -> DcfInput {
        DcfInput {
            base_revenue: dec!(1000),
            revenue_growth_rates: vec![
                dec!(0.10),
                dec!(0.09),
                dec!(0.08),
                dec!(0.07),
                dec!(0.06),
                dec!(0.05),
                dec!(0.05),
                dec!(0.04),
                dec!(0.04),
                dec!(0.03),
            ],
            ebitda_margin: dec!(0.25),
            ebit_margin: None,
            da_as_pct_revenue: Some(dec!(0.03)),
            capex_as_pct_revenue: dec!(0.05),
            nwc_as_pct_revenue: dec!(0.10),
            tax_rate: dec!(0.25),
            wacc: dec!(0.10),
            wacc_input: None,
            terminal_method: TerminalMethod::GordonGrowth,
            terminal_growth_rate: Some(dec!(0.025)),
            terminal_exit_multiple: None,
            currency: Currency::USD,
            forecast_years: None,
            mid_year_convention: Some(true),
            net_debt: Some(dec!(200)),
            minority_interest: None,
            shares_outstanding: Some(dec!(100)),
        }
    }

    #[test]
    fn test_basic_dcf() {
        let input = sample_dcf_input();
        let result = calculate_dcf(&input).unwrap();
        let out = &result.result;

        // Should have 10 projection years
        assert_eq!(out.projections.len(), 10);

        // Year 1 revenue = 1000 * 1.10 = 1100
        assert_eq!(out.projections[0].revenue, dec!(1100));

        // Enterprise value should be positive and reasonable
        assert!(out.enterprise_value > Decimal::ZERO);

        // Equity value should be EV - net_debt
        assert!(out.equity_value.is_some());
        let eq = out.equity_value.unwrap();
        assert_eq!(eq, out.enterprise_value - dec!(200));

        // Per-share value
        assert!(out.equity_value_per_share.is_some());
        let eps = out.equity_value_per_share.unwrap();
        assert_eq!(eps, eq / dec!(100));

        // WACC used
        assert_eq!(out.wacc_used, dec!(0.10));
    }

    #[test]
    fn test_dcf_year1_fcff() {
        let input = sample_dcf_input();
        let result = calculate_dcf(&input).unwrap();
        let y1 = &result.result.projections[0];

        // Revenue = 1100
        assert_eq!(y1.revenue, dec!(1100));
        // EBITDA = 1100 * 0.25 = 275
        assert_eq!(y1.ebitda, dec!(275));
        // D&A = 1100 * 0.03 = 33
        assert_eq!(y1.plus_da, dec!(33));
        // EBIT = 275 - 33 = 242
        assert_eq!(y1.ebit, dec!(242));
        // NOPAT = 242 * (1 - 0.25) = 181.5
        assert_eq!(y1.nopat, dec!(181.5));
        // CapEx = 1100 * 0.05 = 55
        assert_eq!(y1.less_capex, dec!(55));
        // NWC change = 1100*0.10 - 1000*0.10 = 110 - 100 = 10
        assert_eq!(y1.less_nwc_change, dec!(10));
        // FCFF = 181.5 + 33 - 55 - 10 = 149.5
        assert_eq!(y1.fcff, dec!(149.5));
    }

    #[test]
    fn test_dcf_exit_multiple() {
        let mut input = sample_dcf_input();
        input.terminal_method = TerminalMethod::ExitMultiple;
        input.terminal_growth_rate = None;
        input.terminal_exit_multiple = Some(dec!(10));

        let result = calculate_dcf(&input).unwrap();
        let out = &result.result;

        assert!(out.terminal_value_exit.is_some());
        assert!(out.terminal_value_gordon.is_none());
        assert!(out.enterprise_value > Decimal::ZERO);

        // TV = terminal EBITDA * 10x
        let last_ebitda = out.projections.last().unwrap().ebitda;
        assert_eq!(out.terminal_value_exit.unwrap(), last_ebitda * dec!(10));
    }

    #[test]
    fn test_dcf_both_terminal_methods() {
        let mut input = sample_dcf_input();
        input.terminal_method = TerminalMethod::Both;
        input.terminal_exit_multiple = Some(dec!(10));

        let result = calculate_dcf(&input).unwrap();
        let out = &result.result;

        assert!(out.terminal_value_gordon.is_some());
        assert!(out.terminal_value_exit.is_some());
        // Primary uses Gordon
        assert_eq!(out.terminal_value_used, out.terminal_value_gordon.unwrap());
    }

    #[test]
    fn test_dcf_terminal_growth_exceeds_wacc() {
        let mut input = sample_dcf_input();
        input.terminal_growth_rate = Some(dec!(0.12)); // > WACC of 10%

        let result = calculate_dcf(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_dcf_zero_wacc_rejected() {
        let mut input = sample_dcf_input();
        input.wacc = Decimal::ZERO;

        let result = calculate_dcf(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_dcf_negative_revenue_rejected() {
        let mut input = sample_dcf_input();
        input.base_revenue = dec!(-100);

        let result = calculate_dcf(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_dcf_with_wacc_input() {
        let mut input = sample_dcf_input();
        input.wacc_input = Some(WaccInput {
            risk_free_rate: dec!(0.042),
            equity_risk_premium: dec!(0.055),
            beta: dec!(1.10),
            cost_of_debt: dec!(0.055),
            tax_rate: dec!(0.21),
            debt_weight: dec!(0.30),
            equity_weight: dec!(0.70),
            size_premium: None,
            country_risk_premium: None,
            specific_risk_premium: None,
            unlevered_beta: None,
            target_debt_equity: None,
        });
        // terminal_growth must be < the computed WACC (~8.5%)
        input.terminal_growth_rate = Some(dec!(0.025));

        let result = calculate_dcf(&input).unwrap();
        let out = &result.result;

        // WACC should come from the WACC module, not the flat 0.10
        assert!(out.wacc_used > dec!(0.07) && out.wacc_used < dec!(0.10));
    }

    #[test]
    fn test_dcf_mid_year_convention_off() {
        let mut input = sample_dcf_input();
        input.mid_year_convention = Some(false);

        let result_no_mid = calculate_dcf(&input).unwrap();

        input.mid_year_convention = Some(true);
        let result_mid = calculate_dcf(&input).unwrap();

        // Mid-year convention should give higher EV (less discounting)
        assert!(
            result_mid.result.enterprise_value > result_no_mid.result.enterprise_value,
            "Mid-year EV ({}) should exceed end-of-year EV ({})",
            result_mid.result.enterprise_value,
            result_no_mid.result.enterprise_value,
        );
    }

    #[test]
    fn test_dcf_growth_rate_carry_forward() {
        let mut input = sample_dcf_input();
        input.revenue_growth_rates = vec![dec!(0.08), dec!(0.06)];
        input.forecast_years = Some(5);

        let result = calculate_dcf(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.projections.len(), 5);
        // Years 3-5 should use last rate (6%)
        let y2_rev = out.projections[1].revenue;
        let y3_rev = out.projections[2].revenue;
        let growth_y3 = (y3_rev - y2_rev) / y2_rev;
        assert!(
            (growth_y3 - dec!(0.06)).abs() < dec!(0.001),
            "Year 3 growth should be 6% (carried forward), got {growth_y3}"
        );
    }

    #[test]
    fn test_dcf_no_equity_bridge() {
        let mut input = sample_dcf_input();
        input.net_debt = None;
        input.minority_interest = None;
        input.shares_outstanding = None;

        let result = calculate_dcf(&input).unwrap();
        assert!(result.result.equity_value.is_none());
        assert!(result.result.equity_value_per_share.is_none());
    }

    #[test]
    fn test_dcf_methodology() {
        let input = sample_dcf_input();
        let result = calculate_dcf(&input).unwrap();
        assert_eq!(result.methodology, "2-Stage FCFF DCF (WACC-based)");
    }

    #[test]
    fn test_dcf_tv_percentage() {
        let input = sample_dcf_input();
        let result = calculate_dcf(&input).unwrap();
        let out = &result.result;

        // TV% should be between 0 and 1
        assert!(out.terminal_value_pct >= Decimal::ZERO);
        assert!(out.terminal_value_pct <= Decimal::ONE);
    }
}
