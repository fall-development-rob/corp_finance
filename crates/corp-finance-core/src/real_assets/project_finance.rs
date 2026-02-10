use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{with_metadata, ComputationOutput, Money, Rate};
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Input types
// ---------------------------------------------------------------------------

/// Revenue assumptions for the operating period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevenueAssumptions {
    /// Year 1 operating revenue
    pub base_revenue: Money,
    /// Annual revenue growth rate (decimal, e.g. 0.03 = 3%)
    pub revenue_growth: Rate,
    /// Capacity factor for power/infra projects (e.g. 0.85). Applied as a
    /// multiplier on base_revenue if provided.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capacity_factor: Option<Decimal>,
    /// Percentage of revenue under long-term offtake contract vs merchant
    pub offtake_pct: Decimal,
}

/// Operating expenditure assumptions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpExAssumptions {
    /// Annual fixed operating costs
    pub fixed_opex: Money,
    /// Variable OpEx as percentage of revenue
    pub variable_opex_pct: Rate,
    /// Annual OpEx escalation rate
    pub opex_escalation: Rate,
    /// Annual reserve for major maintenance
    pub major_maintenance_reserve: Money,
}

/// How project debt principal is repaid.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DebtSculpting {
    /// Equal annual principal repayment over the tenor
    LevelRepayment,
    /// Principal sized each year to maintain target DSCR
    Sculpted,
    /// Interest only with full principal at maturity
    BulletMaturity,
}

/// Project-level debt assumptions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectDebt {
    /// Senior secured debt amount
    pub senior_debt: Money,
    /// Annual interest rate on senior debt
    pub senior_rate: Rate,
    /// Repayment period in years (post-construction)
    pub senior_tenor_years: u32,
    /// Debt sculpting / repayment profile
    pub sculpting: DebtSculpting,
    /// Target DSCR when using sculpted repayment (e.g. 1.3)
    pub target_dscr: Decimal,
    /// Months of debt service to hold in reserve account
    pub dsra_months: u32,
    /// Optional subordinated / mezzanine tranche
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subordinated_debt: Option<Money>,
    /// Interest rate on subordinated debt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_rate: Option<Rate>,
}

/// Top-level input for the project finance model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectFinanceInput {
    /// Project name / identifier
    pub project_name: String,
    /// Total development and construction cost
    pub total_project_cost: Money,
    /// Years of construction (no revenue generated)
    pub construction_period_years: u32,
    /// Years of operation post-construction
    pub operating_period_years: u32,
    /// Revenue assumptions for the operating phase
    pub revenue_assumptions: RevenueAssumptions,
    /// Operating cost assumptions
    pub operating_assumptions: OpExAssumptions,
    /// Debt structure and repayment assumptions
    pub debt_assumptions: ProjectDebt,
    /// Sponsor equity contribution
    pub equity_contribution: Money,
    /// Discount rate used for project NPV calculation
    pub discount_rate: Rate,
    /// Corporate / project tax rate
    pub tax_rate: Rate,
    /// Straight-line depreciation period in years
    pub depreciation_years: u32,
}

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// Year-by-year financial projection for the project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectionYear {
    /// Year number (1-based, spanning construction + operating)
    pub year: i32,
    /// "Construction" or "Operating"
    pub phase: String,
    /// Gross revenue for the year
    pub revenue: Money,
    /// Total operating expenditure
    pub opex: Money,
    /// EBITDA = revenue - opex
    pub ebitda: Money,
    /// Straight-line depreciation charge
    pub depreciation: Money,
    /// EBIT = EBITDA - depreciation
    pub ebit: Money,
    /// Tax payable (zero if EBIT <= 0)
    pub tax: Money,
    /// Net income = EBIT - tax
    pub net_income: Money,
    /// Cash flow available for debt service (CFADS)
    pub cash_flow_available_for_debt_service: Money,
    /// Total senior debt service (principal + interest)
    pub senior_debt_service: Money,
    /// Debt Service Coverage Ratio (CFADS / senior_debt_service)
    pub dscr: Decimal,
    /// Cash flow remaining for equity after all debt service
    pub cash_flow_to_equity: Money,
    /// Outstanding senior debt balance at year end
    pub outstanding_debt: Money,
}

/// Summary debt coverage and leverage metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebtMetrics {
    /// Minimum DSCR across all operating years
    pub min_dscr: Decimal,
    /// Average DSCR across all operating years
    pub avg_dscr: Decimal,
    /// Loan Life Coverage Ratio (NPV of CFADS over loan life / debt outstanding)
    pub llcr: Decimal,
    /// Project Life Coverage Ratio (NPV of CFADS over project life / debt outstanding)
    pub plcr: Decimal,
    /// Maximum leverage = senior_debt / total_project_cost
    pub max_leverage: Decimal,
    /// Required DSRA balance
    pub dsra_balance: Money,
}

/// Distribution waterfall for a single year.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaterfallYear {
    /// Year number
    pub year: i32,
    /// Cash flow available for debt service
    pub cfads: Money,
    /// Senior debt service paid
    pub senior_debt_service: Money,
    /// Subordinated debt service paid
    pub sub_debt_service: Money,
    /// Contribution to / release from DSRA
    pub dsra_contribution: Money,
    /// Residual distribution to equity holders
    pub equity_distribution: Money,
}

/// Complete project finance model output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectFinanceOutput {
    /// Unlevered project IRR (on pre-financing cash flows)
    pub project_irr: Decimal,
    /// Levered equity IRR (on equity cash flows)
    pub equity_irr: Decimal,
    /// Project NPV at the specified discount rate
    pub project_npv: Money,
    /// Equity multiple = total distributions / equity contribution
    pub equity_multiple: Decimal,
    /// Years to recover the initial equity investment
    pub payback_period_years: Decimal,
    /// Year-by-year income statement and cash flow projections
    pub projections: Vec<ProjectionYear>,
    /// Summary debt coverage metrics
    pub debt_metrics: DebtMetrics,
    /// Annual distribution waterfall
    pub distribution_waterfall: Vec<WaterfallYear>,
}

// ---------------------------------------------------------------------------
// Core computation
// ---------------------------------------------------------------------------

/// Build a full project finance model for infrastructure / project finance
/// transactions.
///
/// Models construction and operating phases, debt sculpting (level, sculpted,
/// or bullet), distribution waterfall, and computes project/equity IRR, NPV,
/// DSCR, LLCR, PLCR, and payback period.
pub fn model_project_finance(
    input: &ProjectFinanceInput,
) -> CorpFinanceResult<ComputationOutput<ProjectFinanceOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // ── Validation ───────────────────────────────────────────────────
    validate_input(input)?;

    let total_years = input.construction_period_years + input.operating_period_years;
    let debt = &input.debt_assumptions;
    let rev = &input.revenue_assumptions;
    let opex_a = &input.operating_assumptions;

    // Effective base revenue (apply capacity factor if present)
    let effective_base_revenue = match rev.capacity_factor {
        Some(cf) => rev.base_revenue * cf,
        None => rev.base_revenue,
    };

    // Annual depreciation (straight-line over depreciation_years on total cost)
    let annual_depreciation = if input.depreciation_years > 0 {
        input.total_project_cost / Decimal::from(input.depreciation_years)
    } else {
        Decimal::ZERO
    };

    // Senior debt level repayment amount (used for LevelRepayment sculpting)
    let level_principal = if debt.senior_tenor_years > 0 {
        debt.senior_debt / Decimal::from(debt.senior_tenor_years)
    } else {
        Decimal::ZERO
    };

    // Sub debt annual interest
    let sub_debt_amount = debt.subordinated_debt.unwrap_or(Decimal::ZERO);
    let sub_rate = debt.sub_rate.unwrap_or(Decimal::ZERO);

    // ── Phase 1: Build raw CFADS schedule ────────────────────────────
    // We need CFADS before we can sculpt debt, so first pass computes
    // CFADS ignoring debt service, then we layer in debt service.

    let mut projections: Vec<ProjectionYear> = Vec::with_capacity(total_years as usize);
    let mut cfads_schedule: Vec<Money> = Vec::with_capacity(total_years as usize);

    let mut current_revenue = effective_base_revenue;
    let mut current_fixed_opex = opex_a.fixed_opex;

    // Capitalized interest during construction
    let mut capitalized_interest = Decimal::ZERO;
    let construction_draw_per_year = if input.construction_period_years > 0 {
        input.total_project_cost / Decimal::from(input.construction_period_years)
    } else {
        Decimal::ZERO
    };
    let mut drawn_debt = Decimal::ZERO;

    for yr in 1..=total_years {
        let is_construction = yr <= input.construction_period_years;
        let phase = if is_construction {
            "Construction".to_string()
        } else {
            "Operating".to_string()
        };

        if is_construction {
            // During construction: no revenue, capitalize interest on drawn debt
            let draw_this_year =
                construction_draw_per_year.min(input.total_project_cost - drawn_debt);
            // Interest on average balance during this year
            let avg_balance = drawn_debt + draw_this_year / dec!(2);
            let interest_this_year = avg_balance * debt.senior_rate;
            capitalized_interest += interest_this_year;
            drawn_debt += draw_this_year;

            let proj = ProjectionYear {
                year: yr as i32,
                phase,
                revenue: Decimal::ZERO,
                opex: Decimal::ZERO,
                ebitda: Decimal::ZERO,
                depreciation: Decimal::ZERO,
                ebit: Decimal::ZERO,
                tax: Decimal::ZERO,
                net_income: Decimal::ZERO,
                cash_flow_available_for_debt_service: Decimal::ZERO,
                senior_debt_service: Decimal::ZERO,
                dscr: Decimal::ZERO,
                cash_flow_to_equity: Decimal::ZERO,
                outstanding_debt: drawn_debt,
            };
            cfads_schedule.push(Decimal::ZERO);
            projections.push(proj);
        } else {
            let op_year = yr - input.construction_period_years;

            // Revenue: year 1 uses base, subsequent years grow
            let revenue = if op_year == 1 {
                current_revenue
            } else {
                current_revenue *= Decimal::ONE + rev.revenue_growth;
                current_revenue
            };

            // OpEx: fixed (escalating) + variable (% of revenue) + maintenance
            if op_year > 1 {
                current_fixed_opex *= Decimal::ONE + opex_a.opex_escalation;
            }
            let variable_opex = revenue * opex_a.variable_opex_pct;
            let total_opex = current_fixed_opex + variable_opex + opex_a.major_maintenance_reserve;

            let ebitda = revenue - total_opex;

            // Depreciation: apply only while within depreciation life
            let depreciation = if op_year <= input.depreciation_years {
                annual_depreciation
            } else {
                Decimal::ZERO
            };

            let ebit = ebitda - depreciation;
            let tax = if ebit > Decimal::ZERO {
                ebit * input.tax_rate
            } else {
                Decimal::ZERO
            };
            let net_income = ebit - tax;

            // CFADS = EBITDA - tax - major maintenance capex
            // (maintenance reserve is already in opex, so CFADS = EBITDA - tax)
            let cfads = ebitda - tax;

            cfads_schedule.push(cfads);

            let proj = ProjectionYear {
                year: yr as i32,
                phase,
                revenue,
                opex: total_opex,
                ebitda,
                depreciation,
                ebit,
                tax,
                net_income,
                cash_flow_available_for_debt_service: cfads,
                // Debt service fields filled in Phase 2
                senior_debt_service: Decimal::ZERO,
                dscr: Decimal::ZERO,
                cash_flow_to_equity: Decimal::ZERO,
                outstanding_debt: Decimal::ZERO,
            };
            projections.push(proj);
        }
    }

    // ── Phase 2: Debt service schedule ───────────────────────────────
    // Operating-period CFADS are stored in cfads_schedule at indices
    // [construction_period_years .. total_years-1].
    // The first operating-year CFADS is at cfads_schedule[construction_period_years].

    let construction_n = input.construction_period_years as usize;
    let operating_cfads: Vec<Money> = cfads_schedule[construction_n..].to_vec();

    // Outstanding senior debt at start of operations = senior_debt + capitalized interest
    let initial_outstanding = debt.senior_debt + capitalized_interest;

    // Compute principal schedule per operating year
    let mut principal_schedule: Vec<Money> =
        vec![Decimal::ZERO; input.operating_period_years as usize];
    let mut interest_schedule: Vec<Money> =
        vec![Decimal::ZERO; input.operating_period_years as usize];

    // First pass: compute interest and principal for each operating year
    match debt.sculpting {
        DebtSculpting::LevelRepayment => {
            let mut bal = initial_outstanding;
            for i in 0..input.operating_period_years as usize {
                let interest = bal * debt.senior_rate;
                let principal = if i < debt.senior_tenor_years as usize {
                    level_principal.min(bal)
                } else {
                    Decimal::ZERO
                };
                interest_schedule[i] = interest;
                principal_schedule[i] = principal;
                bal -= principal;
            }
        }
        DebtSculpting::Sculpted => {
            // Sculpted: principal_t = CFADS_t / target_dscr - interest_t
            // We iterate: interest depends on outstanding, principal depends
            // on CFADS and interest.
            let mut bal = initial_outstanding;
            let tenor = debt.senior_tenor_years as usize;
            for i in 0..input.operating_period_years as usize {
                let interest = bal * debt.senior_rate;
                interest_schedule[i] = interest;

                if i < tenor && bal > Decimal::ZERO && debt.target_dscr > Decimal::ZERO {
                    // total_ds = CFADS / target_dscr
                    // principal = total_ds - interest
                    let total_ds = operating_cfads[i] / debt.target_dscr;
                    let principal = (total_ds - interest).max(Decimal::ZERO).min(bal);
                    principal_schedule[i] = principal;
                    bal -= principal;
                } else {
                    principal_schedule[i] = Decimal::ZERO;
                }
            }
        }
        DebtSculpting::BulletMaturity => {
            let mut bal = initial_outstanding;
            let tenor = debt.senior_tenor_years as usize;
            for i in 0..input.operating_period_years as usize {
                let interest = bal * debt.senior_rate;
                interest_schedule[i] = interest;
                if i == tenor.saturating_sub(1) && tenor > 0 {
                    // Full principal at maturity
                    principal_schedule[i] = bal;
                    bal = Decimal::ZERO;
                }
            }
        }
    }

    // ── Phase 3: Waterfall and final projections ─────────────────────
    let mut waterfall: Vec<WaterfallYear> = Vec::with_capacity(total_years as usize);
    let mut equity_distributions: Vec<Money> = Vec::new();
    let mut outstanding = initial_outstanding;
    let sub_outstanding = sub_debt_amount;
    let mut dsra_balance = Decimal::ZERO;

    // Track DSCRs for metrics
    let mut dscr_values: Vec<Decimal> = Vec::new();

    for yr in 1..=total_years {
        let idx = (yr - 1) as usize;
        let is_construction = yr <= input.construction_period_years;

        if is_construction {
            // Construction year waterfall: no CFADS, no service
            waterfall.push(WaterfallYear {
                year: yr as i32,
                cfads: Decimal::ZERO,
                senior_debt_service: Decimal::ZERO,
                sub_debt_service: Decimal::ZERO,
                dsra_contribution: Decimal::ZERO,
                equity_distribution: Decimal::ZERO,
            });
            equity_distributions.push(Decimal::ZERO);
        } else {
            let op_idx = (yr - input.construction_period_years - 1) as usize;
            let cfads = operating_cfads[op_idx];
            let _interest = interest_schedule[op_idx];
            let principal = principal_schedule[op_idx];

            // Recompute interest on actual outstanding balance
            let actual_interest = outstanding * debt.senior_rate;
            let actual_principal = principal.min(outstanding);
            let senior_ds = actual_interest + actual_principal;

            // DSCR
            let dscr = if senior_ds > Decimal::ZERO {
                cfads / senior_ds
            } else if cfads >= Decimal::ZERO {
                dec!(99.0) // effectively infinite coverage
            } else {
                Decimal::ZERO
            };
            dscr_values.push(dscr);

            outstanding -= actual_principal;

            // Sub debt service (interest only, no amortization assumed)
            let sub_interest = sub_outstanding * sub_rate;

            // DSRA target: dsra_months / 12 * next period's debt service
            let dsra_target = if op_idx + 1 < input.operating_period_years as usize {
                let next_interest = if outstanding > Decimal::ZERO {
                    outstanding * debt.senior_rate
                } else {
                    Decimal::ZERO
                };
                let next_principal = if op_idx + 1 < principal_schedule.len() {
                    principal_schedule[op_idx + 1].min(outstanding)
                } else {
                    Decimal::ZERO
                };
                let next_ds = next_interest + next_principal;
                Decimal::from(debt.dsra_months) / dec!(12) * next_ds
            } else {
                // Last year: DSRA released
                Decimal::ZERO
            };

            let dsra_contribution = dsra_target - dsra_balance;
            dsra_balance = dsra_target;

            // Equity distribution = CFADS - senior DS - sub DS - DSRA contribution
            let equity_dist =
                cfads - senior_ds - sub_interest - dsra_contribution.max(Decimal::ZERO);
            let equity_dist = equity_dist.max(Decimal::ZERO);

            // If DSRA is releasing (negative contribution), add to equity
            let equity_dist = if dsra_contribution < Decimal::ZERO {
                equity_dist + (-dsra_contribution)
            } else {
                equity_dist
            };

            equity_distributions.push(equity_dist);

            // Update projection row
            projections[idx].senior_debt_service = senior_ds;
            projections[idx].dscr = dscr;
            projections[idx].cash_flow_to_equity = equity_dist;
            projections[idx].outstanding_debt = outstanding;

            waterfall.push(WaterfallYear {
                year: yr as i32,
                cfads,
                senior_debt_service: senior_ds,
                sub_debt_service: sub_interest,
                dsra_contribution,
                equity_distribution: equity_dist,
            });
        }
    }

    // ── Phase 4: IRR, NPV, metrics ──────────────────────────────────

    // Project IRR: unlevered cash flows
    // [-total_project_cost at t=0, CFADS during operating years]
    let mut project_cfs: Vec<Money> = Vec::with_capacity((total_years + 1) as usize);
    project_cfs.push(-input.total_project_cost);
    // Construction years: zero cash flow
    for _ in 0..input.construction_period_years {
        project_cfs.push(Decimal::ZERO);
    }
    // Operating years: CFADS
    for cfads in &operating_cfads {
        project_cfs.push(*cfads);
    }

    let project_irr = compute_irr(&project_cfs, &mut warnings, "Project IRR");

    // Equity IRR: levered cash flows
    // [-equity at t=0, zero during construction, equity distributions during ops]
    let mut equity_cfs: Vec<Money> = Vec::with_capacity((total_years + 1) as usize);
    equity_cfs.push(-input.equity_contribution);
    for dist in &equity_distributions {
        equity_cfs.push(*dist);
    }

    let equity_irr = compute_irr(&equity_cfs, &mut warnings, "Equity IRR");

    // Project NPV using iterative discount factors
    let project_npv = compute_npv(input.discount_rate, &project_cfs)?;

    // Equity multiple
    let total_equity_distributions: Money = equity_distributions.iter().sum();
    let equity_multiple = if input.equity_contribution > Decimal::ZERO {
        total_equity_distributions / input.equity_contribution
    } else {
        Decimal::ZERO
    };

    // Payback period
    let payback_period_years = compute_payback(
        input.equity_contribution,
        &equity_distributions,
        input.construction_period_years,
    );

    // Debt metrics
    let min_dscr = dscr_values
        .iter()
        .copied()
        .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or(Decimal::ZERO);

    let avg_dscr = if dscr_values.is_empty() {
        Decimal::ZERO
    } else {
        let sum: Decimal = dscr_values.iter().sum();
        sum / Decimal::from(dscr_values.len() as i64)
    };

    // LLCR: NPV of CFADS over loan life / initial outstanding debt
    let loan_life = debt.senior_tenor_years as usize;
    let llcr_cfads: Vec<Money> = operating_cfads.iter().take(loan_life).copied().collect();
    let llcr = compute_coverage_ratio(&llcr_cfads, debt.senior_rate, initial_outstanding);

    // PLCR: NPV of all CFADS over project life / initial outstanding debt
    let plcr = compute_coverage_ratio(&operating_cfads, debt.senior_rate, initial_outstanding);

    let max_leverage = if input.total_project_cost > Decimal::ZERO {
        debt.senior_debt / input.total_project_cost
    } else {
        Decimal::ZERO
    };

    // DSRA balance: required amount (based on first year's debt service)
    let dsra_required = if !interest_schedule.is_empty() && !principal_schedule.is_empty() {
        let first_ds = interest_schedule[0] + principal_schedule[0];
        Decimal::from(debt.dsra_months) / dec!(12) * first_ds
    } else {
        Decimal::ZERO
    };

    let debt_metrics = DebtMetrics {
        min_dscr,
        avg_dscr,
        llcr,
        plcr,
        max_leverage,
        dsra_balance: dsra_required,
    };

    // ── Warnings ─────────────────────────────────────────────────────
    if min_dscr < dec!(1.2) && min_dscr > Decimal::ZERO {
        warnings.push(format!(
            "Minimum DSCR of {min_dscr} is below 1.2x — lender covenant risk"
        ));
    }
    if equity_irr < dec!(0.08) && equity_irr > Decimal::ZERO {
        warnings.push(format!(
            "Equity IRR of {equity_irr} is below 8% — may not meet investor hurdle"
        ));
    }
    if llcr < dec!(1.1) && llcr > Decimal::ZERO {
        warnings.push(format!(
            "LLCR of {llcr} is below 1.1x — debt serviceability concern"
        ));
    }
    let project_life =
        Decimal::from(input.construction_period_years + input.operating_period_years);
    if payback_period_years > project_life * dec!(0.70) && payback_period_years < dec!(999) {
        warnings.push(format!(
            "Payback period of {payback_period_years} years exceeds 70% of project life"
        ));
    }

    let output = ProjectFinanceOutput {
        project_irr,
        equity_irr,
        project_npv,
        equity_multiple,
        payback_period_years,
        projections,
        debt_metrics,
        distribution_waterfall: waterfall,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Project Finance Model (Infrastructure)",
        &serde_json::json!({
            "project_name": input.project_name,
            "total_project_cost": input.total_project_cost.to_string(),
            "construction_years": input.construction_period_years,
            "operating_years": input.operating_period_years,
            "senior_debt": debt.senior_debt.to_string(),
            "equity_contribution": input.equity_contribution.to_string(),
            "sculpting": format!("{:?}", debt.sculpting),
            "discount_rate": input.discount_rate.to_string(),
        }),
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Validate all input constraints.
fn validate_input(input: &ProjectFinanceInput) -> CorpFinanceResult<()> {
    if input.total_project_cost <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "total_project_cost".into(),
            reason: "Total project cost must be positive".into(),
        });
    }

    let total_funding = input.equity_contribution
        + input.debt_assumptions.senior_debt
        + input
            .debt_assumptions
            .subordinated_debt
            .unwrap_or(Decimal::ZERO);
    if total_funding < input.total_project_cost {
        return Err(CorpFinanceError::InvalidInput {
            field: "equity_contribution + debt".into(),
            reason: format!(
                "Total funding ({total_funding}) is less than project cost ({})",
                input.total_project_cost
            ),
        });
    }

    if input.operating_period_years < 1 {
        return Err(CorpFinanceError::InvalidInput {
            field: "operating_period_years".into(),
            reason: "Operating period must be at least 1 year".into(),
        });
    }

    if input.debt_assumptions.target_dscr < Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "target_dscr".into(),
            reason: "Target DSCR must be >= 1.0".into(),
        });
    }

    if input.equity_contribution < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "equity_contribution".into(),
            reason: "Equity contribution cannot be negative".into(),
        });
    }

    Ok(())
}

/// Compute IRR using Newton-Raphson (30 iterations, epsilon 1e-7).
/// Falls back to zero with a warning if convergence fails.
fn compute_irr(cash_flows: &[Money], warnings: &mut Vec<String>, label: &str) -> Decimal {
    if cash_flows.len() < 2 {
        warnings.push(format!("{label}: insufficient cash flows"));
        return Decimal::ZERO;
    }

    let epsilon = dec!(0.0000001);
    let max_iter = 30;
    let mut rate = dec!(0.10); // initial guess

    for _i in 0..max_iter {
        let mut npv_val = Decimal::ZERO;
        let mut dnpv = Decimal::ZERO;
        let one_plus_r = Decimal::ONE + rate;

        // Use iterative discount factor to avoid powd
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

        // Guard against divergence
        if rate < dec!(-0.99) {
            rate = dec!(-0.99);
        } else if rate > dec!(100.0) {
            rate = dec!(100.0);
        }
    }

    // If we did not converge, try the crate-level irr as fallback
    match crate::time_value::irr(cash_flows, dec!(0.10)) {
        Ok(r) => r,
        Err(e) => {
            warnings.push(format!("{label} did not converge: {e}"));
            Decimal::ZERO
        }
    }
}

/// Compute NPV using iterative discount factors (no powd).
fn compute_npv(rate: Rate, cash_flows: &[Money]) -> CorpFinanceResult<Money> {
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

/// Compute a coverage ratio (LLCR or PLCR).
/// = PV(CFADS at discount_rate) / outstanding_debt
fn compute_coverage_ratio(
    cfads: &[Money],
    discount_rate: Rate,
    outstanding_debt: Money,
) -> Decimal {
    if outstanding_debt <= Decimal::ZERO || cfads.is_empty() {
        return Decimal::ZERO;
    }

    let one_plus_r = Decimal::ONE + discount_rate;
    let mut pv = Decimal::ZERO;
    let mut discount = Decimal::ONE;

    for (t, cf) in cfads.iter().enumerate() {
        if t > 0 {
            discount *= one_plus_r;
        }
        if discount.is_zero() {
            break;
        }
        pv += cf / discount;
    }

    if outstanding_debt.is_zero() {
        Decimal::ZERO
    } else {
        pv / outstanding_debt
    }
}

/// Compute payback period: the year at which cumulative equity distributions
/// equal or exceed the equity contribution. Returns 999 if never recovered.
fn compute_payback(
    equity_contribution: Money,
    equity_distributions: &[Money],
    _construction_years: u32,
) -> Decimal {
    if equity_contribution <= Decimal::ZERO {
        return Decimal::ZERO;
    }

    let mut cumulative = Decimal::ZERO;
    for (i, dist) in equity_distributions.iter().enumerate() {
        let prev_cumulative = cumulative;
        cumulative += dist;
        if cumulative >= equity_contribution {
            // Interpolate within the year
            let year = (i + 1) as i64; // 1-based year in total project life
            let needed = equity_contribution - prev_cumulative;
            let fraction = if *dist > Decimal::ZERO {
                needed / dist
            } else {
                Decimal::ZERO
            };
            return Decimal::from(year - 1) + fraction;
        }
    }

    // Never recovered
    dec!(999)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// Helper: standard infrastructure project input (e.g. solar farm).
    fn standard_project_input() -> ProjectFinanceInput {
        ProjectFinanceInput {
            project_name: "Solar Farm Alpha".into(),
            total_project_cost: dec!(100_000_000),
            construction_period_years: 2,
            operating_period_years: 20,
            revenue_assumptions: RevenueAssumptions {
                base_revenue: dec!(15_000_000),
                revenue_growth: dec!(0.02),
                capacity_factor: Some(dec!(0.85)),
                offtake_pct: dec!(0.90),
            },
            operating_assumptions: OpExAssumptions {
                fixed_opex: dec!(2_000_000),
                variable_opex_pct: dec!(0.05),
                opex_escalation: dec!(0.02),
                major_maintenance_reserve: dec!(500_000),
            },
            debt_assumptions: ProjectDebt {
                senior_debt: dec!(70_000_000),
                senior_rate: dec!(0.05),
                senior_tenor_years: 15,
                sculpting: DebtSculpting::LevelRepayment,
                target_dscr: dec!(1.3),
                dsra_months: 6,
                subordinated_debt: None,
                sub_rate: None,
            },
            equity_contribution: dec!(30_000_000),
            discount_rate: dec!(0.08),
            tax_rate: dec!(0.25),
            depreciation_years: 20,
        }
    }

    #[test]
    fn test_basic_infrastructure_project() {
        let input = standard_project_input();
        let result = model_project_finance(&input).unwrap();
        let out = &result.result;

        // Should have 22 years total (2 construction + 20 operating)
        assert_eq!(out.projections.len(), 22);

        // Project IRR should be positive for a viable project
        assert!(
            out.project_irr > Decimal::ZERO,
            "Project IRR should be positive, got {}",
            out.project_irr
        );

        // Equity IRR should be positive
        assert!(
            out.equity_irr > Decimal::ZERO,
            "Equity IRR should be positive, got {}",
            out.equity_irr
        );

        // Equity multiple should be > 1
        assert!(
            out.equity_multiple > Decimal::ONE,
            "Equity multiple should be > 1, got {}",
            out.equity_multiple
        );
    }

    #[test]
    fn test_sculpted_debt_sizing() {
        let mut input = standard_project_input();
        input.debt_assumptions.sculpting = DebtSculpting::Sculpted;
        input.debt_assumptions.target_dscr = dec!(1.30);

        let result = model_project_finance(&input).unwrap();
        let out = &result.result;

        // With sculpted debt, DSCR should be close to target for most years
        let operating_projs: Vec<&ProjectionYear> = out
            .projections
            .iter()
            .filter(|p| p.phase == "Operating" && p.senior_debt_service > Decimal::ZERO)
            .collect();

        for proj in &operating_projs {
            // DSCR should be approximately target_dscr (within tolerance)
            let diff = (proj.dscr - dec!(1.30)).abs();
            assert!(
                diff < dec!(0.5),
                "Year {}: DSCR {} too far from target 1.3",
                proj.year,
                proj.dscr
            );
        }
    }

    #[test]
    fn test_level_repayment() {
        let mut input = standard_project_input();
        input.debt_assumptions.sculpting = DebtSculpting::LevelRepayment;
        input.debt_assumptions.senior_tenor_years = 15;

        let result = model_project_finance(&input).unwrap();
        let out = &result.result;

        // Debt should decrease over operating years
        let mut prev_debt = Decimal::MAX;
        for proj in out.projections.iter().filter(|p| p.phase == "Operating") {
            if proj.senior_debt_service > Decimal::ZERO {
                assert!(
                    proj.outstanding_debt <= prev_debt,
                    "Year {}: debt {} should be <= previous {}",
                    proj.year,
                    proj.outstanding_debt,
                    prev_debt
                );
                prev_debt = proj.outstanding_debt;
            }
        }
    }

    #[test]
    fn test_bullet_maturity() {
        let mut input = standard_project_input();
        input.debt_assumptions.sculpting = DebtSculpting::BulletMaturity;
        input.debt_assumptions.senior_tenor_years = 15;

        let result = model_project_finance(&input).unwrap();
        let out = &result.result;

        // Before maturity year, debt should stay roughly constant (interest only)
        let operating_projs: Vec<&ProjectionYear> = out
            .projections
            .iter()
            .filter(|p| p.phase == "Operating")
            .collect();

        // First operating year should have high outstanding debt
        assert!(
            operating_projs[0].outstanding_debt > Decimal::ZERO,
            "Outstanding debt should be positive in first operating year"
        );

        // Before tenor end, debt should be approximately the same
        // (only interest is paid, no principal reduction except at maturity)
        if operating_projs.len() > 2 {
            let _first_debt = operating_projs[0].outstanding_debt;
            let _mid_debt = operating_projs[operating_projs.len() / 2].outstanding_debt;
            // Bullet: debt stays the same until maturity
            // Check the year before maturity
            if operating_projs.len() > 14 {
                let before_maturity = operating_projs[13].outstanding_debt;
                assert!(
                    before_maturity > Decimal::ZERO,
                    "Debt before maturity should still be outstanding"
                );
            }
        }
    }

    #[test]
    fn test_construction_and_operating_phases() {
        let input = standard_project_input();
        let result = model_project_finance(&input).unwrap();
        let out = &result.result;

        let construction: Vec<&ProjectionYear> = out
            .projections
            .iter()
            .filter(|p| p.phase == "Construction")
            .collect();
        let operating: Vec<&ProjectionYear> = out
            .projections
            .iter()
            .filter(|p| p.phase == "Operating")
            .collect();

        assert_eq!(construction.len(), 2);
        assert_eq!(operating.len(), 20);

        // Construction years should have zero revenue
        for proj in &construction {
            assert_eq!(
                proj.revenue,
                Decimal::ZERO,
                "Year {}: construction should have zero revenue",
                proj.year
            );
        }

        // Operating years should have positive revenue
        for proj in &operating {
            assert!(
                proj.revenue > Decimal::ZERO,
                "Year {}: operating should have positive revenue",
                proj.year
            );
        }
    }

    #[test]
    fn test_dscr_computation() {
        let input = standard_project_input();
        let result = model_project_finance(&input).unwrap();
        let out = &result.result;

        // DSCR should be CFADS / senior_debt_service for each operating year
        for proj in out.projections.iter().filter(|p| p.phase == "Operating") {
            if proj.senior_debt_service > Decimal::ZERO {
                let expected_dscr =
                    proj.cash_flow_available_for_debt_service / proj.senior_debt_service;
                let diff = (proj.dscr - expected_dscr).abs();
                assert!(
                    diff < dec!(0.01),
                    "Year {}: DSCR {} != expected {}",
                    proj.year,
                    proj.dscr,
                    expected_dscr
                );
            }
        }

        // Debt metrics min_dscr should match the minimum projection DSCR
        let min_proj_dscr = out
            .projections
            .iter()
            .filter(|p| p.phase == "Operating" && p.senior_debt_service > Decimal::ZERO)
            .map(|p| p.dscr)
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(Decimal::ZERO);

        let diff = (out.debt_metrics.min_dscr - min_proj_dscr).abs();
        assert!(
            diff < dec!(0.01),
            "min_dscr {} != min projection DSCR {}",
            out.debt_metrics.min_dscr,
            min_proj_dscr
        );
    }

    #[test]
    fn test_llcr_plcr() {
        let input = standard_project_input();
        let result = model_project_finance(&input).unwrap();
        let out = &result.result;

        // LLCR should be positive for a viable project
        assert!(
            out.debt_metrics.llcr > Decimal::ZERO,
            "LLCR should be positive, got {}",
            out.debt_metrics.llcr
        );

        // PLCR should be >= LLCR (project life >= loan life)
        assert!(
            out.debt_metrics.plcr >= out.debt_metrics.llcr,
            "PLCR ({}) should be >= LLCR ({})",
            out.debt_metrics.plcr,
            out.debt_metrics.llcr
        );

        // Both should be > 1 for a healthy project
        assert!(out.debt_metrics.llcr > Decimal::ONE, "LLCR should be > 1.0");
        assert!(out.debt_metrics.plcr > Decimal::ONE, "PLCR should be > 1.0");
    }

    #[test]
    fn test_equity_irr() {
        let input = standard_project_input();
        let result = model_project_finance(&input).unwrap();
        let out = &result.result;

        // Equity IRR should typically be higher than project IRR due to leverage
        // (assuming project is profitable and debt is cheaper than project return)
        assert!(
            out.equity_irr >= out.project_irr || out.equity_irr > Decimal::ZERO,
            "Equity IRR ({}) should generally be >= Project IRR ({})",
            out.equity_irr,
            out.project_irr
        );
    }

    #[test]
    fn test_project_npv() {
        let input = standard_project_input();
        let result = model_project_finance(&input).unwrap();
        let out = &result.result;

        // At the project IRR, NPV should be approximately zero
        // At a lower discount rate than IRR, NPV should be positive
        if out.project_irr > input.discount_rate {
            assert!(
                out.project_npv > Decimal::ZERO,
                "NPV should be positive when discount rate < IRR"
            );
        }
    }

    #[test]
    fn test_subordinated_debt_layer() {
        let mut input = standard_project_input();
        input.debt_assumptions.subordinated_debt = Some(dec!(10_000_000));
        input.debt_assumptions.sub_rate = Some(dec!(0.08));
        // Adjust equity down to keep total funding = cost
        input.equity_contribution = dec!(20_000_000);

        let result = model_project_finance(&input).unwrap();
        let out = &result.result;

        // Waterfall should show sub debt service in operating years
        let has_sub_service = out
            .distribution_waterfall
            .iter()
            .any(|w| w.sub_debt_service > Decimal::ZERO);
        assert!(
            has_sub_service,
            "Waterfall should include subordinated debt service"
        );

        // Sub debt service should be 10M * 8% = 800K per year
        let first_op_waterfall = out
            .distribution_waterfall
            .iter()
            .find(|w| w.sub_debt_service > Decimal::ZERO)
            .unwrap();
        let expected_sub_interest = dec!(10_000_000) * dec!(0.08);
        assert_eq!(
            first_op_waterfall.sub_debt_service, expected_sub_interest,
            "Sub debt service should be {}",
            expected_sub_interest
        );
    }

    #[test]
    fn test_zero_revenue_construction_years() {
        let mut input = standard_project_input();
        input.construction_period_years = 3;

        let result = model_project_finance(&input).unwrap();
        let out = &result.result;

        // First 3 years should be construction with zero revenue
        for i in 0..3 {
            assert_eq!(out.projections[i].phase, "Construction");
            assert_eq!(out.projections[i].revenue, Decimal::ZERO);
            assert_eq!(out.projections[i].ebitda, Decimal::ZERO);
            assert_eq!(out.projections[i].tax, Decimal::ZERO);
        }

        // Year 4 should be first operating year with revenue
        assert_eq!(out.projections[3].phase, "Operating");
        assert!(out.projections[3].revenue > Decimal::ZERO);
    }

    #[test]
    fn test_payback_calculation() {
        let input = standard_project_input();
        let result = model_project_finance(&input).unwrap();
        let out = &result.result;

        // Payback should be within the project life
        let total_life =
            Decimal::from(input.construction_period_years + input.operating_period_years);
        assert!(
            out.payback_period_years <= total_life || out.payback_period_years == dec!(999),
            "Payback {} should be within project life {} or 999",
            out.payback_period_years,
            total_life
        );

        // For a viable project, payback should be < 999
        assert!(
            out.payback_period_years < dec!(999),
            "Payback should be achievable for a standard project"
        );
    }

    #[test]
    fn test_validation_zero_project_cost() {
        let mut input = standard_project_input();
        input.total_project_cost = Decimal::ZERO;

        let result = model_project_finance(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "total_project_cost");
            }
            other => panic!("Expected InvalidInput, got: {other:?}"),
        }
    }

    #[test]
    fn test_validation_insufficient_funding() {
        let mut input = standard_project_input();
        input.equity_contribution = dec!(1_000); // Way too low
        input.debt_assumptions.senior_debt = dec!(1_000);

        let result = model_project_finance(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_zero_operating_period() {
        let mut input = standard_project_input();
        input.operating_period_years = 0;

        let result = model_project_finance(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_dscr_below_one() {
        let mut input = standard_project_input();
        input.debt_assumptions.target_dscr = dec!(0.5);

        let result = model_project_finance(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_waterfall_consistency() {
        let input = standard_project_input();
        let result = model_project_finance(&input).unwrap();
        let out = &result.result;

        // Verify waterfall has entries for every year
        assert_eq!(
            out.distribution_waterfall.len(),
            (input.construction_period_years + input.operating_period_years) as usize
        );

        // Verify equity distribution is never negative
        for wf in &out.distribution_waterfall {
            assert!(
                wf.equity_distribution >= Decimal::ZERO,
                "Year {}: equity distribution should be non-negative",
                wf.year
            );
        }

        // In surplus years (CFADS > senior + sub debt service), equity > 0
        let has_equity_dist = out
            .distribution_waterfall
            .iter()
            .any(|w| w.equity_distribution > Decimal::ZERO);
        assert!(
            has_equity_dist,
            "A viable project should have at least one year with equity distribution"
        );

        // Construction years should have zero distributions
        for wf in out
            .distribution_waterfall
            .iter()
            .take(input.construction_period_years as usize)
        {
            assert_eq!(wf.cfads, Decimal::ZERO);
            assert_eq!(wf.senior_debt_service, Decimal::ZERO);
            assert_eq!(wf.equity_distribution, Decimal::ZERO);
        }
    }

    #[test]
    fn test_max_leverage() {
        let input = standard_project_input();
        let result = model_project_finance(&input).unwrap();
        let out = &result.result;

        let expected = dec!(70_000_000) / dec!(100_000_000);
        assert_eq!(
            out.debt_metrics.max_leverage, expected,
            "Max leverage should be senior_debt / total_project_cost"
        );
    }

    #[test]
    fn test_revenue_growth_applied() {
        let mut input = standard_project_input();
        input.revenue_assumptions.revenue_growth = dec!(0.05);
        input.revenue_assumptions.capacity_factor = None;
        input.revenue_assumptions.base_revenue = dec!(10_000_000);
        input.construction_period_years = 0;
        input.operating_period_years = 3;
        input.debt_assumptions.senior_debt = dec!(70_000_000);
        input.debt_assumptions.senior_tenor_years = 3;
        input.equity_contribution = dec!(30_000_000);

        let result = model_project_finance(&input).unwrap();
        let out = &result.result;

        // Year 1: base revenue
        assert_eq!(out.projections[0].revenue, dec!(10_000_000));
        // Year 2: base * 1.05
        assert_eq!(out.projections[1].revenue, dec!(10_500_000));
        // Year 3: base * 1.05^2
        assert_eq!(out.projections[2].revenue, dec!(11_025_000));
    }

    #[test]
    fn test_no_construction_period() {
        let mut input = standard_project_input();
        input.construction_period_years = 0;

        let result = model_project_finance(&input).unwrap();
        let out = &result.result;

        // All years should be operating
        assert_eq!(out.projections.len(), 20);
        for proj in &out.projections {
            assert_eq!(proj.phase, "Operating");
        }
    }

    #[test]
    fn test_dsra_balance_reported() {
        let input = standard_project_input();
        let result = model_project_finance(&input).unwrap();
        let out = &result.result;

        // DSRA balance should be > 0 when dsra_months > 0
        assert!(
            out.debt_metrics.dsra_balance > Decimal::ZERO,
            "DSRA balance should be positive with 6 months coverage"
        );
    }

    #[test]
    fn test_capacity_factor_applied() {
        let mut input = standard_project_input();
        input.revenue_assumptions.capacity_factor = Some(dec!(1.0));
        input.construction_period_years = 0;
        let _base = input.revenue_assumptions.base_revenue;

        let result_full = model_project_finance(&input).unwrap();

        input.revenue_assumptions.capacity_factor = Some(dec!(0.50));
        let result_half = model_project_finance(&input).unwrap();

        // Revenue with 50% capacity should be half of 100% capacity
        let rev_full = result_full.result.projections[0].revenue;
        let rev_half = result_half.result.projections[0].revenue;
        assert_eq!(
            rev_half,
            rev_full / dec!(2),
            "50% capacity factor should halve revenue"
        );
    }
}
