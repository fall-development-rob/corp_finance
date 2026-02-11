use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::{CorpFinanceError, CorpFinanceResult};

// ---------------------------------------------------------------------------
// Input types
// ---------------------------------------------------------------------------

/// Input parameters for concession valuation and analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConcessionInput {
    /// Concession name / identifier
    pub concession_name: String,
    /// Years remaining on the concession
    pub remaining_years: u32,
    /// Current annual revenue
    pub current_annual_revenue: Decimal,
    /// Annual revenue growth rate (decimal, e.g. 0.03 = 3%)
    pub revenue_growth_rate: Decimal,
    /// Opex as percentage of revenue
    pub opex_margin: Decimal,
    /// Annual maintenance capex
    pub capex_maintenance: Decimal,
    /// Cost of handback condition compliance
    pub handback_cost: Decimal,
    /// Years before end when handback capex starts
    pub handback_years_before_end: u32,
    /// WACC for NPV calculations
    pub discount_rate: Decimal,
    /// Terminal value approach: "None", "Reversion", "Extension"
    pub terminal_value_approach: String,
    /// Reversion value (if terminal approach is "Reversion")
    pub reversion_value: Decimal,
    /// Probability of extension (decimal, e.g. 0.50 = 50%)
    pub extension_probability: Decimal,
    /// Additional years if extension granted
    pub extension_years: u32,
    /// Outstanding debt balance
    pub outstanding_debt: Decimal,
    /// Debt interest rate
    pub debt_rate: Decimal,
    /// Annual debt service (principal + interest)
    pub annual_debt_service: Decimal,
    /// Corporate tax rate
    pub tax_rate: Decimal,
    /// Additional discount for regulatory risk
    pub regulatory_risk_premium: Decimal,
}

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// Year-by-year cash flow projection for the concession.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConcessionYear {
    pub year: u32,
    pub revenue: Decimal,
    pub opex: Decimal,
    pub maintenance_capex: Decimal,
    pub handback_capex: Decimal,
    pub ebitda: Decimal,
    pub debt_service: Decimal,
    pub tax: Decimal,
    pub fcf_to_equity: Decimal,
    pub cumulative_fcf: Decimal,
}

/// Summary of debt service coverage ratios.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageSummary {
    pub min_dscr: Decimal,
    pub avg_dscr: Decimal,
    pub max_dscr: Decimal,
}

/// Key financial metrics for comparability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConcessionMetrics {
    /// EV / EBITDA multiple
    pub ev_to_ebitda: Decimal,
    /// EV / Revenue multiple
    pub ev_to_revenue: Decimal,
    /// Year 1 FCF / EV
    pub fcf_yield: Decimal,
    /// Outstanding debt / EBITDA
    pub debt_to_ebitda: Decimal,
}

/// Complete output of the concession valuation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConcessionOutput {
    /// Enterprise value (NPV of all cash flows)
    pub enterprise_value: Decimal,
    /// Equity value (EV minus debt)
    pub equity_value: Decimal,
    /// EV per remaining year
    pub ev_per_year: Decimal,
    /// Implied yield (year 1 FCF / EV)
    pub implied_yield: Decimal,
    /// Annual cash flow projections
    pub fcf_projections: Vec<ConcessionYear>,
    /// IRR on equity investment at current valuation
    pub irr: Decimal,
    /// Years to recover equity from FCF
    pub payback_years: Decimal,
    /// Coverage ratio summary
    pub coverage_ratios: CoverageSummary,
    /// Value of the extension option
    pub extension_option_value: Decimal,
    /// Total value (EV + extension option)
    pub total_value_with_option: Decimal,
    /// PV of handback obligations
    pub handback_cost_pv: Decimal,
    /// Impact of regulatory risk premium on value
    pub regulatory_risk_adjustment: Decimal,
    /// Comparable financial metrics
    pub comparable_metrics: ConcessionMetrics,
}

// ---------------------------------------------------------------------------
// Core computation
// ---------------------------------------------------------------------------

/// Value a concession and produce detailed cash flow projections and metrics.
///
/// Projects year-by-year revenue, opex, handback costs, computes FCF to equity,
/// NPV at adjusted discount rate, extension option value, IRR, DSCR, and
/// comparable metrics.
pub fn value_concession(input: &ConcessionInput) -> CorpFinanceResult<ConcessionOutput> {
    validate_concession_input(input)?;

    let adjusted_rate = input.discount_rate + input.regulatory_risk_premium;
    let base_rate = input.discount_rate;

    // Determine when handback capex begins
    let handback_start_year = if input.handback_years_before_end > 0
        && input.remaining_years > input.handback_years_before_end
    {
        input.remaining_years - input.handback_years_before_end + 1
    } else if input.handback_years_before_end > 0 {
        // Handback period exceeds remaining years: start from year 1
        1
    } else {
        // No handback
        input.remaining_years + 1
    };

    // Spread handback cost evenly over the handback years
    let handback_years_count = if input.handback_years_before_end > 0 {
        input.handback_years_before_end.min(input.remaining_years)
    } else {
        0
    };
    let annual_handback = if handback_years_count > 0 && input.handback_cost > Decimal::ZERO {
        input.handback_cost / Decimal::from(handback_years_count)
    } else {
        Decimal::ZERO
    };

    // Build year-by-year projections
    let mut projections: Vec<ConcessionYear> = Vec::with_capacity(input.remaining_years as usize);
    let mut dscr_values: Vec<Decimal> = Vec::new();
    let mut cumulative_fcf = Decimal::ZERO;

    // Cash flow array for enterprise value NPV (unlevered FCF)
    let mut unlevered_cfs: Vec<Decimal> = Vec::with_capacity(input.remaining_years as usize);
    // Cash flow array for equity IRR
    let mut equity_cfs: Vec<Decimal> = Vec::with_capacity((input.remaining_years + 1) as usize);

    let mut current_revenue = input.current_annual_revenue;

    for yr in 1..=input.remaining_years {
        // Revenue growth (year 1 uses current, subsequent grow)
        if yr > 1 {
            current_revenue *= Decimal::ONE + input.revenue_growth_rate;
        }

        let revenue = current_revenue;
        let opex = revenue * input.opex_margin;
        let ebitda = revenue - opex;

        let maintenance_capex = input.capex_maintenance;

        // Handback capex
        let handback_capex = if yr >= handback_start_year && handback_years_count > 0 {
            annual_handback
        } else {
            Decimal::ZERO
        };

        // Debt service
        let debt_service = input.annual_debt_service;

        // Tax: on EBITDA less interest component (simplified: assume interest = debt * rate)
        let interest_expense = input.outstanding_debt * input.debt_rate;
        let taxable_income =
            (ebitda - interest_expense - maintenance_capex - handback_capex).max(Decimal::ZERO);
        let tax = taxable_income * input.tax_rate;

        // FCF to equity = EBITDA - debt_service - tax - maintenance_capex - handback_capex
        let fcf_to_equity = ebitda - debt_service - tax - maintenance_capex - handback_capex;
        cumulative_fcf += fcf_to_equity;

        // Unlevered FCF = EBITDA - tax_on_ebitda - maintenance_capex - handback_capex
        // (for enterprise value, we use pre-debt cash flows)
        let ebitda_tax =
            (ebitda - maintenance_capex - handback_capex).max(Decimal::ZERO) * input.tax_rate;
        let unlevered_fcf = ebitda - ebitda_tax - maintenance_capex - handback_capex;
        unlevered_cfs.push(unlevered_fcf);

        // DSCR
        let dscr = if debt_service > Decimal::ZERO {
            ebitda / debt_service
        } else if ebitda >= Decimal::ZERO {
            dec!(99)
        } else {
            Decimal::ZERO
        };
        dscr_values.push(dscr);

        equity_cfs.push(fcf_to_equity);

        projections.push(ConcessionYear {
            year: yr,
            revenue,
            opex,
            maintenance_capex,
            handback_capex,
            ebitda,
            debt_service,
            tax,
            fcf_to_equity,
            cumulative_fcf,
        });
    }

    // --- Enterprise value: NPV of unlevered FCFs ---
    let ev_at_adjusted = compute_npv_iterative(adjusted_rate, &unlevered_cfs);

    // Also compute at base rate (without regulatory premium) for regulatory adjustment
    let ev_at_base = compute_npv_iterative(base_rate, &unlevered_cfs);
    let regulatory_risk_adjustment = ev_at_base - ev_at_adjusted;

    // Terminal / reversion value
    let terminal_value = match input.terminal_value_approach.as_str() {
        "Reversion" => {
            // PV of reversion value at end of concession
            let mut discount = Decimal::ONE;
            let one_plus_r = Decimal::ONE + adjusted_rate;
            for _ in 0..input.remaining_years {
                discount *= one_plus_r;
            }
            if discount.is_zero() {
                Decimal::ZERO
            } else {
                input.reversion_value / discount
            }
        }
        _ => Decimal::ZERO, // "None" or "Extension" (handled separately)
    };

    let enterprise_value = ev_at_adjusted + terminal_value;

    // Equity value = EV - outstanding debt
    let equity_value = enterprise_value - input.outstanding_debt;

    // EV per remaining year
    let ev_per_year = if input.remaining_years > 0 {
        enterprise_value / Decimal::from(input.remaining_years)
    } else {
        Decimal::ZERO
    };

    // Implied yield: year 1 FCF / EV
    let year1_fcf = if !projections.is_empty() {
        projections[0].fcf_to_equity
    } else {
        Decimal::ZERO
    };
    let implied_yield = if enterprise_value > Decimal::ZERO {
        year1_fcf / enterprise_value
    } else {
        Decimal::ZERO
    };

    // Equity IRR: [-equity_value at t=0, then FCF to equity each year]
    let mut irr_cfs: Vec<Decimal> = Vec::with_capacity((input.remaining_years + 1) as usize);
    irr_cfs.push(-equity_value);
    for cf in &equity_cfs {
        irr_cfs.push(*cf);
    }
    let irr = compute_irr_nr(&irr_cfs, 50);

    // Payback years
    let payback_years = compute_payback(equity_value, &equity_cfs);

    // Coverage ratios
    let min_dscr = dscr_values
        .iter()
        .copied()
        .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or(Decimal::ZERO);

    let max_dscr = dscr_values
        .iter()
        .copied()
        .filter(|d| *d < dec!(99))
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or(Decimal::ZERO);

    let avg_dscr = if dscr_values.is_empty() {
        Decimal::ZERO
    } else {
        let realistic: Vec<Decimal> = dscr_values
            .iter()
            .copied()
            .filter(|d| *d < dec!(99))
            .collect();
        if realistic.is_empty() {
            dec!(99)
        } else {
            let sum: Decimal = realistic.iter().sum();
            sum / Decimal::from(realistic.len() as i64)
        }
    };

    let coverage_ratios = CoverageSummary {
        min_dscr,
        avg_dscr,
        max_dscr,
    };

    // Extension option value
    let extension_option_value = compute_extension_option(input, &projections, adjusted_rate);

    let total_value_with_option = enterprise_value + extension_option_value;

    // Handback cost PV
    let handback_cost_pv = compute_handback_pv(
        annual_handback,
        handback_start_year,
        input.remaining_years,
        adjusted_rate,
    );

    // Comparable metrics
    let year1_ebitda = if !projections.is_empty() {
        projections[0].ebitda
    } else {
        Decimal::ZERO
    };
    let year1_revenue = if !projections.is_empty() {
        projections[0].revenue
    } else {
        Decimal::ZERO
    };
    let year1_unlevered_fcf = if !unlevered_cfs.is_empty() {
        unlevered_cfs[0]
    } else {
        Decimal::ZERO
    };

    let comparable_metrics = ConcessionMetrics {
        ev_to_ebitda: if year1_ebitda > Decimal::ZERO {
            enterprise_value / year1_ebitda
        } else {
            Decimal::ZERO
        },
        ev_to_revenue: if year1_revenue > Decimal::ZERO {
            enterprise_value / year1_revenue
        } else {
            Decimal::ZERO
        },
        fcf_yield: if enterprise_value > Decimal::ZERO {
            year1_unlevered_fcf / enterprise_value
        } else {
            Decimal::ZERO
        },
        debt_to_ebitda: if year1_ebitda > Decimal::ZERO {
            input.outstanding_debt / year1_ebitda
        } else {
            Decimal::ZERO
        },
    };

    Ok(ConcessionOutput {
        enterprise_value,
        equity_value,
        ev_per_year,
        implied_yield,
        fcf_projections: projections,
        irr,
        payback_years,
        coverage_ratios,
        extension_option_value,
        total_value_with_option,
        handback_cost_pv,
        regulatory_risk_adjustment,
        comparable_metrics,
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Validate concession input constraints.
fn validate_concession_input(input: &ConcessionInput) -> CorpFinanceResult<()> {
    if input.remaining_years == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "remaining_years".into(),
            reason: "Remaining concession years must be at least 1".into(),
        });
    }

    if input.current_annual_revenue < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "current_annual_revenue".into(),
            reason: "Current annual revenue cannot be negative".into(),
        });
    }

    if input.discount_rate <= dec!(-1) {
        return Err(CorpFinanceError::InvalidInput {
            field: "discount_rate".into(),
            reason: "Discount rate must be greater than -100%".into(),
        });
    }

    if input.tax_rate < Decimal::ZERO || input.tax_rate > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "tax_rate".into(),
            reason: "Tax rate must be between 0 and 1".into(),
        });
    }

    if input.opex_margin < Decimal::ZERO || input.opex_margin > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "opex_margin".into(),
            reason: "Opex margin must be between 0 and 1".into(),
        });
    }

    if input.regulatory_risk_premium < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "regulatory_risk_premium".into(),
            reason: "Regulatory risk premium cannot be negative".into(),
        });
    }

    let valid_approaches = ["None", "Reversion", "Extension"];
    if !valid_approaches.contains(&input.terminal_value_approach.as_str()) {
        return Err(CorpFinanceError::InvalidInput {
            field: "terminal_value_approach".into(),
            reason: format!(
                "Terminal value approach must be one of: {:?}",
                valid_approaches
            ),
        });
    }

    Ok(())
}

/// Compute NPV using iterative discount factors (no powd).
fn compute_npv_iterative(rate: Decimal, cash_flows: &[Decimal]) -> Decimal {
    let mut result = Decimal::ZERO;
    let one_plus_r = Decimal::ONE + rate;
    let mut discount = Decimal::ONE;

    for cf in cash_flows.iter() {
        discount *= one_plus_r;
        if discount.is_zero() {
            break;
        }
        result += cf / discount;
    }

    result
}

/// Compute IRR using Newton-Raphson with iterative discount factors.
fn compute_irr_nr(cash_flows: &[Decimal], max_iter: u32) -> Decimal {
    if cash_flows.len() < 2 {
        return Decimal::ZERO;
    }

    let epsilon = dec!(0.0000001);
    let mut rate = dec!(0.10);

    for _ in 0..max_iter {
        let mut npv_val = Decimal::ZERO;
        let mut dnpv = Decimal::ZERO;
        let one_plus_r = Decimal::ONE + rate;

        let mut discount = Decimal::ONE;
        for (t, cf) in cash_flows.iter().enumerate() {
            if t > 0 {
                discount *= one_plus_r;
            }
            if discount.is_zero() {
                break;
            }
            npv_val += cf / discount;
            if t > 0 {
                let t_dec = Decimal::from(t as i64);
                dnpv -= t_dec * cf / (discount * one_plus_r);
            }
        }

        if npv_val.abs() < epsilon {
            return rate;
        }

        if dnpv.is_zero() {
            break;
        }

        rate -= npv_val / dnpv;

        if rate < dec!(-0.99) {
            rate = dec!(-0.99);
        } else if rate > dec!(100.0) {
            rate = dec!(100.0);
        }
    }

    // Fallback: try crate-level IRR
    match crate::time_value::irr(cash_flows, dec!(0.10)) {
        Ok(r) => r,
        Err(_) => Decimal::ZERO,
    }
}

/// Compute payback period from a series of cash flows.
fn compute_payback(investment: Decimal, cash_flows: &[Decimal]) -> Decimal {
    if investment <= Decimal::ZERO {
        return Decimal::ZERO;
    }

    let mut cumulative = Decimal::ZERO;
    for (i, cf) in cash_flows.iter().enumerate() {
        let prev = cumulative;
        cumulative += cf;

        if cumulative >= investment {
            let needed = investment - prev;
            let fraction = if *cf > Decimal::ZERO {
                needed / cf
            } else {
                Decimal::ZERO
            };
            return Decimal::from(i as i64) + fraction;
        }
    }

    dec!(999)
}

/// Compute the value of an extension option.
///
/// Extension value = probability * NPV(FCF during extension years) discounted
/// back to present from end of base concession.
fn compute_extension_option(
    input: &ConcessionInput,
    projections: &[ConcessionYear],
    adjusted_rate: Decimal,
) -> Decimal {
    if input.terminal_value_approach != "Extension"
        || input.extension_probability <= Decimal::ZERO
        || input.extension_years == 0
    {
        return Decimal::ZERO;
    }

    // Project FCF during extension years starting from last year's revenue
    let last_revenue = if let Some(last) = projections.last() {
        last.revenue
    } else {
        return Decimal::ZERO;
    };

    let one_plus_r = Decimal::ONE + adjusted_rate;
    let mut extension_pv = Decimal::ZERO;
    let mut current_rev = last_revenue;

    // Discount from end of base concession to present
    let mut base_discount = Decimal::ONE;
    for _ in 0..input.remaining_years {
        base_discount *= one_plus_r;
    }

    // Extension period cash flows
    let mut ext_discount = base_discount;
    for _ in 1..=input.extension_years {
        current_rev *= Decimal::ONE + input.revenue_growth_rate;
        let opex = current_rev * input.opex_margin;
        let ebitda = current_rev - opex;
        let maintenance = input.capex_maintenance;
        let taxable = (ebitda - maintenance).max(Decimal::ZERO);
        let tax = taxable * input.tax_rate;
        let fcf = ebitda - tax - maintenance;

        ext_discount *= one_plus_r;
        if ext_discount.is_zero() {
            break;
        }
        extension_pv += fcf / ext_discount;
    }

    input.extension_probability * extension_pv
}

/// Compute PV of handback obligations.
fn compute_handback_pv(
    annual_handback: Decimal,
    handback_start_year: u32,
    remaining_years: u32,
    rate: Decimal,
) -> Decimal {
    if annual_handback <= Decimal::ZERO {
        return Decimal::ZERO;
    }

    let one_plus_r = Decimal::ONE + rate;
    let mut pv = Decimal::ZERO;
    let mut discount = Decimal::ONE;

    for yr in 1..=remaining_years {
        discount *= one_plus_r;
        if discount.is_zero() {
            break;
        }
        if yr >= handback_start_year {
            pv += annual_handback / discount;
        }
    }

    pv
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Standard concession input for a toll road.
    fn standard_concession_input() -> ConcessionInput {
        ConcessionInput {
            concession_name: "Toll Road Beta".into(),
            remaining_years: 20,
            current_annual_revenue: dec!(50_000_000),
            revenue_growth_rate: dec!(0.03),
            opex_margin: dec!(0.30),
            capex_maintenance: dec!(3_000_000),
            handback_cost: dec!(15_000_000),
            handback_years_before_end: 3,
            discount_rate: dec!(0.08),
            terminal_value_approach: "None".into(),
            reversion_value: Decimal::ZERO,
            extension_probability: Decimal::ZERO,
            extension_years: 0,
            outstanding_debt: dec!(100_000_000),
            debt_rate: dec!(0.05),
            annual_debt_service: dec!(12_000_000),
            tax_rate: dec!(0.25),
            regulatory_risk_premium: dec!(0.01),
        }
    }

    fn extension_input() -> ConcessionInput {
        let mut input = standard_concession_input();
        input.terminal_value_approach = "Extension".into();
        input.extension_probability = dec!(0.50);
        input.extension_years = 10;
        input
    }

    fn reversion_input() -> ConcessionInput {
        let mut input = standard_concession_input();
        input.terminal_value_approach = "Reversion".into();
        input.reversion_value = dec!(50_000_000);
        input
    }

    #[test]
    fn test_basic_concession_valuation() {
        let input = standard_concession_input();
        let result = value_concession(&input).unwrap();

        assert!(
            result.enterprise_value > Decimal::ZERO,
            "Enterprise value should be positive: {}",
            result.enterprise_value
        );
    }

    #[test]
    fn test_equity_value() {
        let input = standard_concession_input();
        let result = value_concession(&input).unwrap();

        let expected_equity = result.enterprise_value - input.outstanding_debt;
        let diff = (result.equity_value - expected_equity).abs();
        assert!(
            diff < dec!(0.01),
            "Equity value {} should equal EV {} - debt {}",
            result.equity_value,
            result.enterprise_value,
            input.outstanding_debt
        );
    }

    #[test]
    fn test_projections_count() {
        let input = standard_concession_input();
        let result = value_concession(&input).unwrap();

        assert_eq!(result.fcf_projections.len(), input.remaining_years as usize);
    }

    #[test]
    fn test_revenue_growth() {
        let input = standard_concession_input();
        let result = value_concession(&input).unwrap();

        // Year 1 revenue should equal current_annual_revenue
        assert_eq!(result.fcf_projections[0].revenue, dec!(50_000_000));

        // Year 2 should grow
        let expected_yr2 = dec!(50_000_000) * (Decimal::ONE + dec!(0.03));
        assert_eq!(result.fcf_projections[1].revenue, expected_yr2);
    }

    #[test]
    fn test_opex_calculation() {
        let input = standard_concession_input();
        let result = value_concession(&input).unwrap();

        let yr1 = &result.fcf_projections[0];
        let expected_opex = yr1.revenue * dec!(0.30);
        assert_eq!(yr1.opex, expected_opex);
    }

    #[test]
    fn test_ebitda_equals_revenue_minus_opex() {
        let input = standard_concession_input();
        let result = value_concession(&input).unwrap();

        for proj in &result.fcf_projections {
            let expected = proj.revenue - proj.opex;
            let diff = (proj.ebitda - expected).abs();
            assert!(
                diff < dec!(0.01),
                "Year {}: EBITDA {} should equal revenue {} - opex {}",
                proj.year,
                proj.ebitda,
                proj.revenue,
                proj.opex
            );
        }
    }

    #[test]
    fn test_handback_costs_applied() {
        let input = standard_concession_input();
        let result = value_concession(&input).unwrap();

        // Handback starts 3 years before end (year 18, 19, 20)
        for proj in &result.fcf_projections {
            if proj.year >= 18 {
                assert!(
                    proj.handback_capex > Decimal::ZERO,
                    "Year {}: handback capex should be positive",
                    proj.year
                );
            } else {
                assert_eq!(
                    proj.handback_capex,
                    Decimal::ZERO,
                    "Year {}: no handback capex expected",
                    proj.year
                );
            }
        }
    }

    #[test]
    fn test_handback_cost_spread() {
        let input = standard_concession_input();
        let result = value_concession(&input).unwrap();

        let annual_handback = dec!(15_000_000) / dec!(3);
        for proj in result.fcf_projections.iter().filter(|p| p.year >= 18) {
            let diff = (proj.handback_capex - annual_handback).abs();
            assert!(
                diff < dec!(0.01),
                "Year {}: handback {} should equal {}",
                proj.year,
                proj.handback_capex,
                annual_handback
            );
        }
    }

    #[test]
    fn test_dscr_positive() {
        let input = standard_concession_input();
        let result = value_concession(&input).unwrap();

        assert!(
            result.coverage_ratios.min_dscr > Decimal::ZERO,
            "Min DSCR should be positive: {}",
            result.coverage_ratios.min_dscr
        );
    }

    #[test]
    fn test_dscr_min_lte_avg_lte_max() {
        let input = standard_concession_input();
        let result = value_concession(&input).unwrap();

        assert!(
            result.coverage_ratios.min_dscr <= result.coverage_ratios.avg_dscr,
            "Min DSCR ({}) should be <= avg ({})",
            result.coverage_ratios.min_dscr,
            result.coverage_ratios.avg_dscr
        );
        assert!(
            result.coverage_ratios.avg_dscr <= result.coverage_ratios.max_dscr,
            "Avg DSCR ({}) should be <= max ({})",
            result.coverage_ratios.avg_dscr,
            result.coverage_ratios.max_dscr
        );
    }

    #[test]
    fn test_ev_per_year() {
        let input = standard_concession_input();
        let result = value_concession(&input).unwrap();

        let expected = result.enterprise_value / Decimal::from(input.remaining_years);
        let diff = (result.ev_per_year - expected).abs();
        assert!(
            diff < dec!(0.01),
            "EV per year {} should equal EV {} / years {}",
            result.ev_per_year,
            result.enterprise_value,
            input.remaining_years
        );
    }

    #[test]
    fn test_implied_yield() {
        let input = standard_concession_input();
        let result = value_concession(&input).unwrap();

        let yr1_fcf = result.fcf_projections[0].fcf_to_equity;
        let expected = yr1_fcf / result.enterprise_value;
        let diff = (result.implied_yield - expected).abs();
        assert!(
            diff < dec!(0.0001),
            "Implied yield {} should equal yr1 FCF {} / EV {}",
            result.implied_yield,
            yr1_fcf,
            result.enterprise_value
        );
    }

    #[test]
    fn test_irr_positive_for_viable_concession() {
        let input = standard_concession_input();
        let result = value_concession(&input).unwrap();

        assert!(
            result.irr > Decimal::ZERO,
            "IRR should be positive: {}",
            result.irr
        );
    }

    #[test]
    fn test_payback_within_concession() {
        let input = standard_concession_input();
        let result = value_concession(&input).unwrap();

        assert!(
            result.payback_years <= Decimal::from(input.remaining_years)
                || result.payback_years == dec!(999),
            "Payback {} should be within remaining years {}",
            result.payback_years,
            input.remaining_years
        );
    }

    #[test]
    fn test_extension_option_value_none() {
        let input = standard_concession_input();
        let result = value_concession(&input).unwrap();

        assert_eq!(
            result.extension_option_value,
            Decimal::ZERO,
            "Extension value should be zero with 'None' approach"
        );
    }

    #[test]
    fn test_extension_option_value_positive() {
        let input = extension_input();
        let result = value_concession(&input).unwrap();

        assert!(
            result.extension_option_value > Decimal::ZERO,
            "Extension value should be positive: {}",
            result.extension_option_value
        );
    }

    #[test]
    fn test_total_value_with_option() {
        let input = extension_input();
        let result = value_concession(&input).unwrap();

        let expected = result.enterprise_value + result.extension_option_value;
        let diff = (result.total_value_with_option - expected).abs();
        assert!(
            diff < dec!(0.01),
            "Total value {} should equal EV {} + option {}",
            result.total_value_with_option,
            result.enterprise_value,
            result.extension_option_value
        );
    }

    #[test]
    fn test_reversion_value_increases_ev() {
        let base_input = standard_concession_input();
        let base_result = value_concession(&base_input).unwrap();

        let rev_input = reversion_input();
        let rev_result = value_concession(&rev_input).unwrap();

        assert!(
            rev_result.enterprise_value > base_result.enterprise_value,
            "Reversion should increase EV: {} vs {}",
            rev_result.enterprise_value,
            base_result.enterprise_value
        );
    }

    #[test]
    fn test_handback_cost_pv_positive() {
        let input = standard_concession_input();
        let result = value_concession(&input).unwrap();

        assert!(
            result.handback_cost_pv > Decimal::ZERO,
            "Handback cost PV should be positive: {}",
            result.handback_cost_pv
        );
    }

    #[test]
    fn test_regulatory_risk_adjustment_positive() {
        let input = standard_concession_input();
        let result = value_concession(&input).unwrap();

        assert!(
            result.regulatory_risk_adjustment > Decimal::ZERO,
            "Regulatory risk adjustment should be positive: {}",
            result.regulatory_risk_adjustment
        );
    }

    #[test]
    fn test_zero_regulatory_premium() {
        let mut input = standard_concession_input();
        input.regulatory_risk_premium = Decimal::ZERO;

        let result = value_concession(&input).unwrap();

        let diff = result.regulatory_risk_adjustment.abs();
        assert!(
            diff < dec!(0.01),
            "With zero premium, regulatory adjustment should be ~0: {}",
            result.regulatory_risk_adjustment
        );
    }

    #[test]
    fn test_comparable_metrics_positive() {
        let input = standard_concession_input();
        let result = value_concession(&input).unwrap();

        assert!(
            result.comparable_metrics.ev_to_ebitda > Decimal::ZERO,
            "EV/EBITDA should be positive"
        );
        assert!(
            result.comparable_metrics.ev_to_revenue > Decimal::ZERO,
            "EV/Revenue should be positive"
        );
        assert!(
            result.comparable_metrics.debt_to_ebitda > Decimal::ZERO,
            "Debt/EBITDA should be positive"
        );
    }

    #[test]
    fn test_cumulative_fcf() {
        let input = standard_concession_input();
        let result = value_concession(&input).unwrap();

        let mut cumulative = Decimal::ZERO;
        for proj in &result.fcf_projections {
            cumulative += proj.fcf_to_equity;
            let diff = (proj.cumulative_fcf - cumulative).abs();
            assert!(
                diff < dec!(0.01),
                "Year {}: cumulative {} != expected {}",
                proj.year,
                proj.cumulative_fcf,
                cumulative
            );
        }
    }

    #[test]
    fn test_validation_zero_remaining_years() {
        let mut input = standard_concession_input();
        input.remaining_years = 0;

        let result = value_concession(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_negative_revenue() {
        let mut input = standard_concession_input();
        input.current_annual_revenue = dec!(-1);

        let result = value_concession(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_invalid_tax_rate() {
        let mut input = standard_concession_input();
        input.tax_rate = dec!(1.5);

        let result = value_concession(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_invalid_opex_margin() {
        let mut input = standard_concession_input();
        input.opex_margin = dec!(1.5);

        let result = value_concession(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_negative_regulatory_premium() {
        let mut input = standard_concession_input();
        input.regulatory_risk_premium = dec!(-0.01);

        let result = value_concession(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_invalid_terminal_approach() {
        let mut input = standard_concession_input();
        input.terminal_value_approach = "Invalid".into();

        let result = value_concession(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_no_handback_cost() {
        let mut input = standard_concession_input();
        input.handback_cost = Decimal::ZERO;
        input.handback_years_before_end = 0;

        let result = value_concession(&input).unwrap();

        for proj in &result.fcf_projections {
            assert_eq!(
                proj.handback_capex,
                Decimal::ZERO,
                "Year {}: no handback expected",
                proj.year
            );
        }
        assert_eq!(result.handback_cost_pv, Decimal::ZERO);
    }

    #[test]
    fn test_no_debt() {
        let mut input = standard_concession_input();
        input.outstanding_debt = Decimal::ZERO;
        input.debt_rate = Decimal::ZERO;
        input.annual_debt_service = Decimal::ZERO;

        let result = value_concession(&input).unwrap();

        assert_eq!(result.equity_value, result.enterprise_value);
        // All DSCRs should be 99 (infinite coverage)
        assert_eq!(result.coverage_ratios.min_dscr, dec!(99));
    }

    #[test]
    fn test_higher_growth_increases_ev() {
        let mut input_low = standard_concession_input();
        input_low.revenue_growth_rate = dec!(0.01);

        let mut input_high = standard_concession_input();
        input_high.revenue_growth_rate = dec!(0.05);

        let result_low = value_concession(&input_low).unwrap();
        let result_high = value_concession(&input_high).unwrap();

        assert!(
            result_high.enterprise_value > result_low.enterprise_value,
            "Higher growth should increase EV: {} vs {}",
            result_high.enterprise_value,
            result_low.enterprise_value
        );
    }

    #[test]
    fn test_higher_discount_rate_decreases_ev() {
        let mut input_low = standard_concession_input();
        input_low.discount_rate = dec!(0.06);

        let mut input_high = standard_concession_input();
        input_high.discount_rate = dec!(0.12);

        let result_low = value_concession(&input_low).unwrap();
        let result_high = value_concession(&input_high).unwrap();

        assert!(
            result_low.enterprise_value > result_high.enterprise_value,
            "Lower discount rate should produce higher EV: {} vs {}",
            result_low.enterprise_value,
            result_high.enterprise_value
        );
    }

    #[test]
    fn test_maintenance_capex_in_projections() {
        let input = standard_concession_input();
        let result = value_concession(&input).unwrap();

        for proj in &result.fcf_projections {
            assert_eq!(
                proj.maintenance_capex,
                dec!(3_000_000),
                "Year {}: maintenance capex should be constant",
                proj.year
            );
        }
    }
}
