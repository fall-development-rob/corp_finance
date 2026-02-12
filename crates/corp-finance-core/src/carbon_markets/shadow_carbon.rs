//! Shadow carbon pricing analytics.
//!
//! Covers:
//! 1. **NPV without carbon** -- standard discounted cash flow.
//! 2. **NPV with carbon** -- DCF internalising carbon cost (with escalation).
//! 3. **Carbon impact** -- difference between the two NPVs.
//! 4. **Marginal abatement cost** -- carbon impact per tonne of emissions.
//! 5. **Breakeven carbon price** -- Newton-Raphson solve for NPV = 0.
//! 6. **Portfolio ranking** -- projects ranked by carbon-adjusted NPV.
//!
//! All arithmetic uses `rust_decimal::Decimal`. No `f64`.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Input / Output
// ---------------------------------------------------------------------------

/// A single project for shadow carbon analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarbonProject {
    /// Project name.
    pub name: String,
    /// Capital expenditure (up-front cost).
    pub capex: Decimal,
    /// Annual operating cash flows by year.
    pub annual_cash_flows: Vec<Decimal>,
    /// Annual CO2e emissions by year (tonnes).
    pub annual_emissions: Vec<Decimal>,
    /// Project life in years.
    pub project_life: u32,
}

/// Input for shadow carbon pricing analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowCarbonInput {
    /// Projects to evaluate.
    pub projects: Vec<CarbonProject>,
    /// Internal carbon price ($/tCO2e).
    pub carbon_price: Decimal,
    /// Discount rate.
    pub discount_rate: Decimal,
    /// Annual escalation of the carbon price (e.g. 0.05 for 5%/yr).
    pub carbon_price_escalation: Decimal,
}

/// Per-project result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectCarbonResult {
    /// Project name.
    pub name: String,
    /// Standard NPV (no carbon cost).
    pub npv_without_carbon: Decimal,
    /// NPV with carbon cost internalised.
    pub npv_with_carbon: Decimal,
    /// Difference (npv_with_carbon - npv_without_carbon), always <= 0 for positive emissions.
    pub carbon_impact: Decimal,
    /// Total lifetime emissions.
    pub total_emissions: Decimal,
    /// Total discounted carbon cost.
    pub total_carbon_cost: Decimal,
    /// Total carbon cost / capex.
    pub carbon_cost_per_unit_capex: Decimal,
    /// Carbon impact / total emissions.
    pub marginal_abatement_cost: Decimal,
    /// Carbon price at which NPV with carbon = 0 (Newton-Raphson).
    pub breakeven_carbon_price: Decimal,
}

/// Aggregate shadow carbon output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowCarbonOutput {
    /// Per-project results.
    pub project_results: Vec<ProjectCarbonResult>,
    /// Projects ranked by NPV_with_carbon (best first).
    pub ranking: Vec<String>,
    /// Total portfolio emissions across all projects.
    pub total_portfolio_emissions: Decimal,
    /// Total portfolio carbon cost.
    pub total_portfolio_carbon_cost: Decimal,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Compute NPV with a given carbon price. Returns (npv, total_carbon_cost).
fn npv_with_carbon_price(
    project: &CarbonProject,
    carbon_price: Decimal,
    discount_rate: Decimal,
    escalation: Decimal,
) -> (Decimal, Decimal) {
    let mut npv = -project.capex;
    let mut total_carbon_cost = Decimal::ZERO;
    let mut discount_factor = Decimal::ONE;
    let years = project.project_life as usize;

    for t in 0..years {
        discount_factor /= Decimal::ONE + discount_rate;
        let cf = if t < project.annual_cash_flows.len() {
            project.annual_cash_flows[t]
        } else {
            Decimal::ZERO
        };
        let em = if t < project.annual_emissions.len() {
            project.annual_emissions[t]
        } else {
            Decimal::ZERO
        };
        // Escalated carbon price for year t+1 (year index is 1-based for escalation)
        let escalation_factor = iterative_power(Decimal::ONE + escalation, (t + 1) as u32);
        let carbon_cost_t = em * carbon_price * escalation_factor;
        total_carbon_cost += carbon_cost_t * discount_factor;
        npv += (cf - carbon_cost_t) * discount_factor;
    }
    (npv, total_carbon_cost)
}

/// Compute standard NPV without carbon cost.
fn npv_without_carbon(project: &CarbonProject, discount_rate: Decimal) -> Decimal {
    let mut npv = -project.capex;
    let mut discount_factor = Decimal::ONE;
    let years = project.project_life as usize;

    for t in 0..years {
        discount_factor /= Decimal::ONE + discount_rate;
        let cf = if t < project.annual_cash_flows.len() {
            project.annual_cash_flows[t]
        } else {
            Decimal::ZERO
        };
        npv += cf * discount_factor;
    }
    npv
}

/// Iterative power: base^exp using repeated multiplication.
fn iterative_power(base: Decimal, exp: u32) -> Decimal {
    let mut result = Decimal::ONE;
    for _ in 0..exp {
        result *= base;
    }
    result
}

/// Newton-Raphson to find breakeven carbon price where NPV_with_carbon = 0.
/// f(p) = NPV_with_carbon(p)
/// f'(p) = -sum(emissions_t * escalation_factor_t * discount_factor_t)
fn breakeven_newton(
    project: &CarbonProject,
    discount_rate: Decimal,
    escalation: Decimal,
    npv_no_carbon: Decimal,
) -> Decimal {
    // If NPV without carbon is already <= 0, breakeven is 0 (or negative).
    if npv_no_carbon <= Decimal::ZERO {
        return Decimal::ZERO;
    }

    // Check if there are any emissions at all
    let total_em: Decimal = project.annual_emissions.iter().copied().sum();
    if total_em <= Decimal::ZERO {
        // No emissions means carbon price has no effect; breakeven is infinite.
        // Return a sentinel large value.
        return dec!(999999999);
    }

    // Pre-compute sensitivity: df/dp = -sum(emissions_t * esc_t * disc_t)
    let years = project.project_life as usize;
    let mut sensitivity = Decimal::ZERO;
    let mut discount_factor = Decimal::ONE;
    for t in 0..years {
        discount_factor /= Decimal::ONE + discount_rate;
        let em = if t < project.annual_emissions.len() {
            project.annual_emissions[t]
        } else {
            Decimal::ZERO
        };
        let esc = iterative_power(Decimal::ONE + escalation, (t + 1) as u32);
        sensitivity += em * esc * discount_factor;
    }

    if sensitivity <= Decimal::ZERO {
        return dec!(999999999);
    }

    // Newton-Raphson: p_{k+1} = p_k - f(p_k) / f'(p_k)
    // f(p) = npv_no_carbon - p * sensitivity  (linear in p for our cost-of-carry model)
    // So breakeven = npv_no_carbon / sensitivity
    // But let's use iterative Newton for generality/spec compliance.
    let mut p = npv_no_carbon / sensitivity; // good initial guess
    for _ in 0..30 {
        let (f_p, _) = npv_with_carbon_price(project, p, discount_rate, escalation);
        // f'(p) = -sensitivity (constant w.r.t. p)
        let fp_deriv = -sensitivity;
        if fp_deriv == Decimal::ZERO {
            break;
        }
        let step = f_p / fp_deriv;
        p -= step;
        // Clamp to non-negative
        if p < Decimal::ZERO {
            p = Decimal::ZERO;
        }
    }
    p
}

// ---------------------------------------------------------------------------
// Core calculation
// ---------------------------------------------------------------------------

/// Compute shadow carbon pricing analysis for a portfolio of projects.
pub fn calculate_shadow_carbon(input: &ShadowCarbonInput) -> CorpFinanceResult<ShadowCarbonOutput> {
    // --- Validation ---
    if input.projects.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one project is required".into(),
        ));
    }
    if input.carbon_price < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "carbon_price".into(),
            reason: "Carbon price cannot be negative".into(),
        });
    }
    if input.discount_rate < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "discount_rate".into(),
            reason: "Discount rate cannot be negative".into(),
        });
    }
    for (i, proj) in input.projects.iter().enumerate() {
        if proj.capex < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("projects[{}].capex", i),
                reason: "Capex cannot be negative".into(),
            });
        }
        if proj.project_life == 0 {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("projects[{}].project_life", i),
                reason: "Project life must be at least 1 year".into(),
            });
        }
    }

    // --- Per-project calculations ---
    let mut project_results = Vec::with_capacity(input.projects.len());
    let mut total_portfolio_emissions = Decimal::ZERO;
    let mut total_portfolio_carbon_cost = Decimal::ZERO;

    for proj in &input.projects {
        let npv_no = npv_without_carbon(proj, input.discount_rate);
        let (npv_with, carbon_cost) = npv_with_carbon_price(
            proj,
            input.carbon_price,
            input.discount_rate,
            input.carbon_price_escalation,
        );
        let carbon_impact = npv_with - npv_no;
        let total_emissions: Decimal = proj.annual_emissions.iter().copied().sum();

        let carbon_cost_per_unit_capex = if proj.capex > Decimal::ZERO {
            carbon_cost / proj.capex
        } else {
            Decimal::ZERO
        };

        let marginal_abatement_cost = if total_emissions > Decimal::ZERO {
            carbon_impact / total_emissions
        } else {
            Decimal::ZERO
        };

        let breakeven = breakeven_newton(
            proj,
            input.discount_rate,
            input.carbon_price_escalation,
            npv_no,
        );

        total_portfolio_emissions += total_emissions;
        total_portfolio_carbon_cost += carbon_cost;

        project_results.push(ProjectCarbonResult {
            name: proj.name.clone(),
            npv_without_carbon: npv_no,
            npv_with_carbon: npv_with,
            carbon_impact,
            total_emissions,
            total_carbon_cost: carbon_cost,
            carbon_cost_per_unit_capex,
            marginal_abatement_cost,
            breakeven_carbon_price: breakeven,
        });
    }

    // --- Ranking by NPV_with_carbon (descending) ---
    let mut ranked: Vec<(usize, Decimal)> = project_results
        .iter()
        .enumerate()
        .map(|(i, r)| (i, r.npv_with_carbon))
        .collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1));
    let ranking: Vec<String> = ranked
        .iter()
        .map(|(i, _)| project_results[*i].name.clone())
        .collect();

    Ok(ShadowCarbonOutput {
        project_results,
        ranking,
        total_portfolio_emissions,
        total_portfolio_carbon_cost,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn simple_project() -> CarbonProject {
        CarbonProject {
            name: "Solar Farm".into(),
            capex: dec!(1000000),
            annual_cash_flows: vec![dec!(300000); 5],
            annual_emissions: vec![dec!(1000); 5],
            project_life: 5,
        }
    }

    fn dirty_project() -> CarbonProject {
        CarbonProject {
            name: "Coal Plant".into(),
            capex: dec!(500000),
            annual_cash_flows: vec![dec!(200000); 5],
            annual_emissions: vec![dec!(50000); 5],
            project_life: 5,
        }
    }

    fn base_input() -> ShadowCarbonInput {
        ShadowCarbonInput {
            projects: vec![simple_project()],
            carbon_price: dec!(50),
            discount_rate: dec!(0.10),
            carbon_price_escalation: dec!(0.05),
        }
    }

    #[test]
    fn test_npv_without_carbon_basic() {
        let input = base_input();
        let out = calculate_shadow_carbon(&input).unwrap();
        let result = &out.project_results[0];
        // NPV = -1000000 + 300000 * (PV annuity 5yr 10%)
        // disc factors: 0.9091, 0.8264, 0.7513, 0.6830, 0.6209 -> sum ~ 3.7908
        // NPV ~ -1000000 + 300000 * 3.7908 ~ 137240
        assert!(result.npv_without_carbon > dec!(137000));
        assert!(result.npv_without_carbon < dec!(138000));
    }

    #[test]
    fn test_npv_with_carbon_less_than_without() {
        let input = base_input();
        let out = calculate_shadow_carbon(&input).unwrap();
        let result = &out.project_results[0];
        assert!(result.npv_with_carbon < result.npv_without_carbon);
    }

    #[test]
    fn test_carbon_impact_negative() {
        let input = base_input();
        let out = calculate_shadow_carbon(&input).unwrap();
        let result = &out.project_results[0];
        assert!(result.carbon_impact < Decimal::ZERO);
    }

    #[test]
    fn test_total_emissions() {
        let input = base_input();
        let out = calculate_shadow_carbon(&input).unwrap();
        // 1000 * 5 = 5000
        assert_eq!(out.project_results[0].total_emissions, dec!(5000));
    }

    #[test]
    fn test_total_carbon_cost_positive() {
        let input = base_input();
        let out = calculate_shadow_carbon(&input).unwrap();
        assert!(out.project_results[0].total_carbon_cost > Decimal::ZERO);
    }

    #[test]
    fn test_carbon_cost_per_unit_capex() {
        let input = base_input();
        let out = calculate_shadow_carbon(&input).unwrap();
        let result = &out.project_results[0];
        let expected = result.total_carbon_cost / dec!(1000000);
        assert_eq!(result.carbon_cost_per_unit_capex, expected);
    }

    #[test]
    fn test_breakeven_convergence() {
        let input = base_input();
        let out = calculate_shadow_carbon(&input).unwrap();
        let result = &out.project_results[0];
        // Breakeven should be positive and finite for a project with positive NPV and emissions
        assert!(result.breakeven_carbon_price > Decimal::ZERO);
        assert!(result.breakeven_carbon_price < dec!(999999999));
    }

    #[test]
    fn test_breakeven_at_breakeven_price_npv_near_zero() {
        let input = base_input();
        let out = calculate_shadow_carbon(&input).unwrap();
        let be = out.project_results[0].breakeven_carbon_price;
        // Verify NPV at breakeven is near zero
        let proj = &input.projects[0];
        let (npv_at_be, _) =
            npv_with_carbon_price(proj, be, input.discount_rate, input.carbon_price_escalation);
        assert!(npv_at_be.abs() < dec!(1)); // within 1 dollar of zero
    }

    #[test]
    fn test_zero_emissions_no_carbon_impact() {
        let mut input = base_input();
        input.projects[0].annual_emissions = vec![Decimal::ZERO; 5];
        let out = calculate_shadow_carbon(&input).unwrap();
        let result = &out.project_results[0];
        assert_eq!(result.carbon_impact, Decimal::ZERO);
        assert_eq!(result.total_carbon_cost, Decimal::ZERO);
        assert_eq!(result.npv_with_carbon, result.npv_without_carbon);
    }

    #[test]
    fn test_zero_carbon_price_no_impact() {
        let mut input = base_input();
        input.carbon_price = Decimal::ZERO;
        let out = calculate_shadow_carbon(&input).unwrap();
        let result = &out.project_results[0];
        assert_eq!(result.carbon_impact, Decimal::ZERO);
    }

    #[test]
    fn test_high_carbon_price_negative_npv() {
        let mut input = base_input();
        input.carbon_price = dec!(5000);
        let out = calculate_shadow_carbon(&input).unwrap();
        let result = &out.project_results[0];
        assert!(result.npv_with_carbon < Decimal::ZERO);
    }

    #[test]
    fn test_escalation_increases_cost() {
        let mut input_no_esc = base_input();
        input_no_esc.carbon_price_escalation = Decimal::ZERO;
        let out_no = calculate_shadow_carbon(&input_no_esc).unwrap();

        let mut input_esc = base_input();
        input_esc.carbon_price_escalation = dec!(0.10);
        let out_esc = calculate_shadow_carbon(&input_esc).unwrap();

        assert!(
            out_esc.project_results[0].total_carbon_cost
                > out_no.project_results[0].total_carbon_cost
        );
    }

    #[test]
    fn test_multiple_projects_ranking() {
        let mut input = base_input();
        input.projects.push(dirty_project());
        let out = calculate_shadow_carbon(&input).unwrap();
        // Solar should rank higher (lower emissions)
        assert_eq!(out.ranking[0], "Solar Farm");
        assert_eq!(out.ranking[1], "Coal Plant");
    }

    #[test]
    fn test_portfolio_totals() {
        let mut input = base_input();
        input.projects.push(dirty_project());
        let out = calculate_shadow_carbon(&input).unwrap();
        // Solar: 5000, Coal: 250000
        assert_eq!(out.total_portfolio_emissions, dec!(255000));
        let sum_cost: Decimal = out
            .project_results
            .iter()
            .map(|r| r.total_carbon_cost)
            .sum();
        assert_eq!(out.total_portfolio_carbon_cost, sum_cost);
    }

    #[test]
    fn test_empty_projects_rejected() {
        let input = ShadowCarbonInput {
            projects: vec![],
            carbon_price: dec!(50),
            discount_rate: dec!(0.10),
            carbon_price_escalation: dec!(0.05),
        };
        let result = calculate_shadow_carbon(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_negative_carbon_price_rejected() {
        let mut input = base_input();
        input.carbon_price = dec!(-10);
        let result = calculate_shadow_carbon(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_negative_discount_rate_rejected() {
        let mut input = base_input();
        input.discount_rate = dec!(-0.05);
        let result = calculate_shadow_carbon(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_negative_capex_rejected() {
        let mut input = base_input();
        input.projects[0].capex = dec!(-100);
        let result = calculate_shadow_carbon(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_zero_project_life_rejected() {
        let mut input = base_input();
        input.projects[0].project_life = 0;
        let result = calculate_shadow_carbon(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_negative_npv_without_carbon_breakeven_zero() {
        let mut input = base_input();
        input.projects[0].capex = dec!(9999999);
        input.projects[0].annual_cash_flows = vec![dec!(100); 5];
        let out = calculate_shadow_carbon(&input).unwrap();
        // NPV without carbon is negative, so breakeven = 0
        assert_eq!(out.project_results[0].breakeven_carbon_price, Decimal::ZERO);
    }

    #[test]
    fn test_marginal_abatement_cost() {
        let input = base_input();
        let out = calculate_shadow_carbon(&input).unwrap();
        let result = &out.project_results[0];
        let expected = result.carbon_impact / result.total_emissions;
        assert_eq!(result.marginal_abatement_cost, expected);
    }

    #[test]
    fn test_zero_capex_carbon_cost_per_unit() {
        let mut input = base_input();
        input.projects[0].capex = Decimal::ZERO;
        let out = calculate_shadow_carbon(&input).unwrap();
        assert_eq!(
            out.project_results[0].carbon_cost_per_unit_capex,
            Decimal::ZERO
        );
    }

    #[test]
    fn test_single_year_project() {
        let mut input = base_input();
        input.projects[0].project_life = 1;
        input.projects[0].annual_cash_flows = vec![dec!(1200000)];
        input.projects[0].annual_emissions = vec![dec!(5000)];
        let out = calculate_shadow_carbon(&input).unwrap();
        let result = &out.project_results[0];
        // NPV = -1000000 + 1200000/1.1 = -1000000 + 1090909.09...
        assert!(result.npv_without_carbon > dec!(90909));
        assert!(result.npv_without_carbon < dec!(90910));
    }
}
