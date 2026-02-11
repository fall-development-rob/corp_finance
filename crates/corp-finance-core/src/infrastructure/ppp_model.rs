use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::{CorpFinanceError, CorpFinanceResult};

// ---------------------------------------------------------------------------
// Input types
// ---------------------------------------------------------------------------

/// Revenue model for the PPP concession.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RevenueModel {
    /// Government pays fixed availability payments (adjusted for deductions).
    AvailabilityPayment,
    /// Revenue depends on demand (e.g. traffic volume * toll rate).
    DemandBased,
    /// A combination of availability payment and demand-based revenue.
    Mixed,
}

/// Input parameters for the PPP financial model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PppModelInput {
    /// Project name / identifier
    pub project_name: String,
    /// Total construction cost
    pub total_capex: Decimal,
    /// Construction duration in months
    pub construction_period_months: u32,
    /// Total concession period in years (including construction)
    pub concession_years: u32,
    /// Revenue model type
    pub revenue_model: RevenueModel,
    /// Annual availability payment (if availability-based)
    pub annual_availability_payment: Decimal,
    /// Initial traffic volume per year (if demand-based)
    pub initial_traffic_volume: Decimal,
    /// Annual demand growth rate (decimal, e.g. 0.03 = 3%)
    pub traffic_growth_rate: Decimal,
    /// Price per unit of demand (toll rate)
    pub toll_rate: Decimal,
    /// Operating costs as percentage of revenue
    pub opex_pct_revenue: Decimal,
    /// Annual major maintenance reserve as percentage of capex
    pub major_maintenance_reserve_pct: Decimal,
    /// Percentage of capex funded by senior debt
    pub senior_debt_pct: Decimal,
    /// Interest rate on senior debt
    pub senior_debt_rate: Decimal,
    /// Senior debt maturity in years
    pub senior_debt_tenor_years: u32,
    /// Percentage of capex funded by mezzanine debt (can be 0)
    pub mezzanine_debt_pct: Decimal,
    /// Mezzanine debt interest rate
    pub mezzanine_rate: Decimal,
    /// Equity contribution percentage
    pub equity_pct: Decimal,
    /// Corporate tax rate
    pub tax_rate: Decimal,
    /// Discount rate for NPV calculations
    pub discount_rate: Decimal,
    /// Annual inflation rate for escalation
    pub inflation_rate: Decimal,
    /// Expected deductions for underperformance (% of availability payment)
    pub availability_deductions_pct: Decimal,
    /// Major lifecycle costs by year (can be empty; index 0 = year 1 of operations)
    pub lifecycle_cost_schedule: Vec<Decimal>,
}

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// Year-by-year projection for the PPP model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PppYearProjection {
    pub year: u32,
    pub revenue: Decimal,
    pub opex: Decimal,
    pub ebitda: Decimal,
    pub senior_debt_service: Decimal,
    pub mezz_debt_service: Decimal,
    pub tax: Decimal,
    pub equity_cash_flow: Decimal,
    pub dscr: Decimal,
    pub cumulative_equity_cf: Decimal,
}

/// Risk allocation entry for a PPP project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAllocation {
    pub risk: String,
    pub allocated_to: String,
    pub mitigation: String,
}

/// Output of the PPP financial model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PppModelOutput {
    /// Project (unlevered) IRR
    pub project_irr: Decimal,
    /// Levered equity IRR
    pub equity_irr: Decimal,
    /// NPV at the specified discount rate
    pub project_npv: Decimal,
    /// Minimum debt service coverage ratio
    pub senior_dscr_min: Decimal,
    /// Average DSCR across operating years with debt
    pub senior_dscr_avg: Decimal,
    /// Loan life coverage ratio
    pub llcr: Decimal,
    /// Project life coverage ratio
    pub plcr: Decimal,
    /// Simple payback period from start of operations (years)
    pub payback_period_years: Decimal,
    /// Year-by-year projections
    pub annual_projections: Vec<PppYearProjection>,
    /// Total cash returned to equity holders
    pub total_equity_return: Decimal,
    /// Equity multiple (total cash to equity / equity invested)
    pub equity_multiple: Decimal,
    /// Value for money score (NPV savings vs traditional procurement)
    pub vfm_score: Decimal,
    /// Key risk allocations
    pub risk_allocation: Vec<RiskAllocation>,
}

// ---------------------------------------------------------------------------
// Core computation
// ---------------------------------------------------------------------------

/// Build a full PPP (Public-Private Partnership) financial model.
///
/// Models construction and operating phases over the concession period,
/// computes project/equity IRR, NPV, DSCR, LLCR, PLCR, payback, VfM score,
/// and standard risk allocations.
pub fn model_ppp(input: &PppModelInput) -> CorpFinanceResult<PppModelOutput> {
    validate_ppp_input(input)?;

    let construction_years = construction_years_from_months(input.construction_period_months);
    let operating_years = input.concession_years.saturating_sub(construction_years);

    if operating_years == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "concession_years".into(),
            reason: "Concession must extend beyond construction period".into(),
        });
    }

    // Funding breakdown
    let senior_debt = input.total_capex * input.senior_debt_pct;
    let mezz_debt = input.total_capex * input.mezzanine_debt_pct;
    let equity_invested = input.total_capex * input.equity_pct;

    // Senior debt annuity payment (constant annual debt service)
    let annual_senior_ds = compute_annuity_payment(
        senior_debt,
        input.senior_debt_rate,
        input.senior_debt_tenor_years,
    );

    // Mezzanine: interest-only during concession
    let annual_mezz_interest = mezz_debt * input.mezzanine_rate;

    // Major maintenance reserve per year
    let annual_maintenance_reserve = input.total_capex * input.major_maintenance_reserve_pct;

    // Build year-by-year projections
    let total_years = construction_years + operating_years;
    let mut projections: Vec<PppYearProjection> = Vec::with_capacity(total_years as usize);

    // Cash flow arrays for IRR/NPV
    // Project (unlevered) CFs: [-capex spread over construction, then EBITDA - tax per op year]
    let mut project_cfs: Vec<Decimal> = Vec::with_capacity((total_years + 1) as usize);
    // Equity CFs: [-equity at t=0, 0 during construction, then equity_cf per op year]
    let mut equity_cfs: Vec<Decimal> = Vec::with_capacity((total_years + 1) as usize);

    // CFADS for LLCR/PLCR
    let mut cfads_schedule: Vec<Decimal> = Vec::with_capacity(operating_years as usize);
    let mut dscr_values: Vec<Decimal> = Vec::new();
    let mut cumulative_equity_cf = Decimal::ZERO;

    // Capex per construction year (evenly spread)
    let capex_per_year = if construction_years > 0 {
        input.total_capex / Decimal::from(construction_years)
    } else {
        input.total_capex
    };

    // Construction years
    for yr in 1..=construction_years {
        project_cfs.push(-capex_per_year);

        let ecf = if yr == 1 {
            -equity_invested
        } else {
            Decimal::ZERO
        };
        equity_cfs.push(ecf);
        cumulative_equity_cf += ecf;

        projections.push(PppYearProjection {
            year: yr,
            revenue: Decimal::ZERO,
            opex: Decimal::ZERO,
            ebitda: Decimal::ZERO,
            senior_debt_service: Decimal::ZERO,
            mezz_debt_service: Decimal::ZERO,
            tax: Decimal::ZERO,
            equity_cash_flow: ecf,
            dscr: Decimal::ZERO,
            cumulative_equity_cf,
        });
    }

    // If no construction period, equity is invested at year 0 (before year 1)
    if construction_years == 0 {
        // Insert a t=0 cash flow for equity and project
        project_cfs.insert(0, -input.total_capex);
        equity_cfs.insert(0, -equity_invested);
    }

    // Operating years
    let mut inflation_factor = Decimal::ONE;
    let mut traffic = input.initial_traffic_volume;

    for op_yr in 1..=operating_years {
        let yr = construction_years + op_yr;

        if op_yr > 1 {
            inflation_factor *= Decimal::ONE + input.inflation_rate;
            traffic *= Decimal::ONE + input.traffic_growth_rate;
        }

        // Revenue calculation based on model type
        let revenue = match input.revenue_model {
            RevenueModel::AvailabilityPayment => {
                let gross = input.annual_availability_payment * inflation_factor;
                gross * (Decimal::ONE - input.availability_deductions_pct)
            }
            RevenueModel::DemandBased => traffic * input.toll_rate * inflation_factor,
            RevenueModel::Mixed => {
                let avail = input.annual_availability_payment
                    * inflation_factor
                    * (Decimal::ONE - input.availability_deductions_pct);
                let demand = traffic * input.toll_rate * inflation_factor;
                avail + demand
            }
        };

        // Opex (escalated by inflation)
        let opex = revenue * input.opex_pct_revenue + annual_maintenance_reserve * inflation_factor;

        // Lifecycle costs
        let lifecycle_cost = if (op_yr as usize) <= input.lifecycle_cost_schedule.len() {
            input.lifecycle_cost_schedule[(op_yr - 1) as usize]
        } else {
            Decimal::ZERO
        };

        let ebitda = revenue - opex - lifecycle_cost;

        // Tax on positive earnings after debt service (simplified: tax on EBITDA - interest)
        let interest_deduction = senior_debt_interest_component(
            senior_debt,
            input.senior_debt_rate,
            input.senior_debt_tenor_years,
            op_yr,
        ) + annual_mezz_interest;
        let taxable_income = (ebitda - interest_deduction).max(Decimal::ZERO);
        let tax = taxable_income * input.tax_rate;

        // CFADS = EBITDA - tax
        let cfads = ebitda - tax;
        cfads_schedule.push(cfads);

        // Debt service
        let sr_ds = if op_yr <= input.senior_debt_tenor_years {
            annual_senior_ds
        } else {
            Decimal::ZERO
        };
        let mz_ds = if mezz_debt > Decimal::ZERO {
            annual_mezz_interest
        } else {
            Decimal::ZERO
        };

        // DSCR
        let dscr = if sr_ds > Decimal::ZERO {
            cfads / sr_ds
        } else if cfads >= Decimal::ZERO {
            dec!(99)
        } else {
            Decimal::ZERO
        };
        if sr_ds > Decimal::ZERO {
            dscr_values.push(dscr);
        }

        // Equity cash flow
        let equity_cf = cfads - sr_ds - mz_ds;
        cumulative_equity_cf += equity_cf;

        // Project (unlevered) CF = EBITDA - tax
        project_cfs.push(cfads);
        equity_cfs.push(equity_cf);

        projections.push(PppYearProjection {
            year: yr,
            revenue,
            opex: opex + lifecycle_cost,
            ebitda,
            senior_debt_service: sr_ds,
            mezz_debt_service: mz_ds,
            tax,
            equity_cash_flow: equity_cf,
            dscr,
            cumulative_equity_cf,
        });
    }

    // --- Compute metrics ---

    // Project IRR
    let project_irr = compute_irr_nr(&project_cfs, 50);

    // Equity IRR
    let equity_irr = compute_irr_nr(&equity_cfs, 50);

    // Project NPV
    let project_npv = compute_npv_iterative(input.discount_rate, &project_cfs);

    // DSCR metrics
    let senior_dscr_min = dscr_values
        .iter()
        .copied()
        .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or(Decimal::ZERO);

    let senior_dscr_avg = if dscr_values.is_empty() {
        Decimal::ZERO
    } else {
        let sum: Decimal = dscr_values.iter().sum();
        sum / Decimal::from(dscr_values.len() as i64)
    };

    // LLCR: NPV(CFADS over loan life) / senior debt
    let loan_life = input.senior_debt_tenor_years as usize;
    let llcr_cfads: Vec<Decimal> = cfads_schedule.iter().take(loan_life).copied().collect();
    let llcr = compute_coverage_ratio(&llcr_cfads, input.senior_debt_rate, senior_debt);

    // PLCR: NPV(all CFADS) / senior debt
    let plcr = compute_coverage_ratio(&cfads_schedule, input.senior_debt_rate, senior_debt);

    // Payback period (from start of operations)
    let payback_period_years =
        compute_payback_period(equity_invested, &projections, construction_years);

    // Total equity return and multiple
    let total_equity_return: Decimal = projections
        .iter()
        .filter(|p| p.year > construction_years)
        .map(|p| p.equity_cash_flow)
        .sum();

    let equity_multiple = if equity_invested > Decimal::ZERO {
        total_equity_return / equity_invested
    } else {
        Decimal::ZERO
    };

    // VfM score: simplified as NPV savings assuming traditional procurement costs 15-20% more
    // VfM = (NPV_traditional - NPV_ppp) / NPV_traditional
    // Traditional assumed to cost 15% more on capex with same ops profile
    let traditional_premium = dec!(0.15);
    let traditional_capex = input.total_capex * (Decimal::ONE + traditional_premium);
    let mut traditional_cfs = project_cfs.clone();
    if !traditional_cfs.is_empty() {
        // Adjust capex outflows
        let capex_adjustment = (traditional_capex - input.total_capex)
            / Decimal::from(if construction_years > 0 {
                construction_years
            } else {
                1
            });
        for cf in traditional_cfs.iter_mut() {
            if *cf < Decimal::ZERO {
                *cf -= capex_adjustment;
            }
        }
    }
    let traditional_npv = compute_npv_iterative(input.discount_rate, &traditional_cfs);
    let vfm_score = if traditional_npv.abs() > Decimal::ZERO {
        (traditional_npv - project_npv) / traditional_npv.abs()
    } else {
        Decimal::ZERO
    };

    // Risk allocations
    let risk_allocation = build_risk_allocations(&input.revenue_model);

    Ok(PppModelOutput {
        project_irr,
        equity_irr,
        project_npv,
        senior_dscr_min,
        senior_dscr_avg,
        llcr,
        plcr,
        payback_period_years,
        annual_projections: projections,
        total_equity_return,
        equity_multiple,
        vfm_score,
        risk_allocation,
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Convert construction months to years (rounded up).
fn construction_years_from_months(months: u32) -> u32 {
    if months == 0 {
        return 0;
    }
    months.div_ceil(12)
}

/// Validate PPP model input constraints.
fn validate_ppp_input(input: &PppModelInput) -> CorpFinanceResult<()> {
    if input.total_capex <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "total_capex".into(),
            reason: "Total capex must be positive".into(),
        });
    }

    if input.concession_years == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "concession_years".into(),
            reason: "Concession period must be at least 1 year".into(),
        });
    }

    let funding_pct = input.senior_debt_pct + input.mezzanine_debt_pct + input.equity_pct;
    let diff = (funding_pct - Decimal::ONE).abs();
    if diff > dec!(0.01) {
        return Err(CorpFinanceError::InvalidInput {
            field: "senior_debt_pct + mezzanine_debt_pct + equity_pct".into(),
            reason: format!(
                "Funding percentages must sum to 100%, got {}%",
                funding_pct * dec!(100)
            ),
        });
    }

    if input.senior_debt_rate < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "senior_debt_rate".into(),
            reason: "Senior debt rate cannot be negative".into(),
        });
    }

    if input.tax_rate < Decimal::ZERO || input.tax_rate > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "tax_rate".into(),
            reason: "Tax rate must be between 0 and 1".into(),
        });
    }

    if input.discount_rate <= dec!(-1) {
        return Err(CorpFinanceError::InvalidInput {
            field: "discount_rate".into(),
            reason: "Discount rate must be greater than -100%".into(),
        });
    }

    Ok(())
}

/// Compute a constant annuity payment for a loan.
/// PMT = P * r / (1 - (1+r)^-n)  using iterative multiplication.
fn compute_annuity_payment(principal: Decimal, rate: Decimal, periods: u32) -> Decimal {
    if principal <= Decimal::ZERO || periods == 0 {
        return Decimal::ZERO;
    }
    if rate.is_zero() {
        return principal / Decimal::from(periods);
    }

    // (1+r)^n via iterative multiplication
    let one_plus_r = Decimal::ONE + rate;
    let mut compound = Decimal::ONE;
    for _ in 0..periods {
        compound *= one_plus_r;
    }

    if compound.is_zero() {
        return Decimal::ZERO;
    }

    // PMT = P * r * (1+r)^n / ((1+r)^n - 1)
    principal * rate * compound / (compound - Decimal::ONE)
}

/// Approximate the interest component of a level annuity payment for a given year.
/// Interest = outstanding_balance * rate. Outstanding balance decreases each year.
fn senior_debt_interest_component(
    principal: Decimal,
    rate: Decimal,
    tenor: u32,
    operating_year: u32,
) -> Decimal {
    if operating_year > tenor || principal <= Decimal::ZERO || rate.is_zero() {
        return Decimal::ZERO;
    }

    let annuity = compute_annuity_payment(principal, rate, tenor);
    let mut balance = principal;

    for yr in 1..=operating_year {
        let interest = balance * rate;
        if yr == operating_year {
            return interest;
        }
        let principal_repaid = annuity - interest;
        balance -= principal_repaid;
        if balance < Decimal::ZERO {
            balance = Decimal::ZERO;
        }
    }

    Decimal::ZERO
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

/// Compute NPV using iterative discount factors (no powd).
fn compute_npv_iterative(rate: Decimal, cash_flows: &[Decimal]) -> Decimal {
    let mut result = Decimal::ZERO;
    let one_plus_r = Decimal::ONE + rate;
    let mut discount = Decimal::ONE;

    for (t, cf) in cash_flows.iter().enumerate() {
        if t > 0 {
            discount *= one_plus_r;
        }
        if discount.is_zero() {
            break;
        }
        result += cf / discount;
    }

    result
}

/// Compute coverage ratio (LLCR or PLCR).
/// = PV(CFADS at discount_rate) / outstanding_debt
fn compute_coverage_ratio(
    cfads: &[Decimal],
    discount_rate: Decimal,
    outstanding_debt: Decimal,
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

/// Compute simple payback period from start of operations.
fn compute_payback_period(
    equity_invested: Decimal,
    projections: &[PppYearProjection],
    construction_years: u32,
) -> Decimal {
    if equity_invested <= Decimal::ZERO {
        return Decimal::ZERO;
    }

    let mut cumulative = Decimal::ZERO;
    let mut op_year: u32 = 0;

    for proj in projections.iter().filter(|p| p.year > construction_years) {
        let prev = cumulative;
        cumulative += proj.equity_cash_flow;
        op_year += 1;

        if cumulative >= equity_invested {
            let needed = equity_invested - prev;
            let fraction = if proj.equity_cash_flow > Decimal::ZERO {
                needed / proj.equity_cash_flow
            } else {
                Decimal::ZERO
            };
            return Decimal::from(op_year - 1) + fraction;
        }
    }

    dec!(999)
}

/// Build standard risk allocations based on the revenue model.
fn build_risk_allocations(revenue_model: &RevenueModel) -> Vec<RiskAllocation> {
    let demand_allocated = match revenue_model {
        RevenueModel::AvailabilityPayment => "Government",
        RevenueModel::DemandBased => "SPV / Concessionaire",
        RevenueModel::Mixed => "Shared (Government + SPV)",
    };

    vec![
        RiskAllocation {
            risk: "Construction risk".into(),
            allocated_to: "Contractor (EPC)".into(),
            mitigation: "Fixed-price turnkey contract with liquidated damages".into(),
        },
        RiskAllocation {
            risk: "Demand / traffic risk".into(),
            allocated_to: demand_allocated.into(),
            mitigation: "Traffic studies, minimum revenue guarantees, revenue sharing bands".into(),
        },
        RiskAllocation {
            risk: "Availability / performance risk".into(),
            allocated_to: "SPV / Concessionaire".into(),
            mitigation: "Performance monitoring, payment deductions, cure periods".into(),
        },
        RiskAllocation {
            risk: "FX / currency risk".into(),
            allocated_to: "SPV / Hedged".into(),
            mitigation: "Natural hedging, FX swap agreements, indexation clauses".into(),
        },
        RiskAllocation {
            risk: "Political / regulatory risk".into(),
            allocated_to: "Government".into(),
            mitigation:
                "Stabilization clauses, political risk insurance, bilateral investment treaties"
                    .into(),
        },
        RiskAllocation {
            risk: "Force majeure".into(),
            allocated_to: "Shared (Government + SPV)".into(),
            mitigation: "Force majeure provisions, insurance coverage, compensation events".into(),
        },
    ]
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Standard PPP input for a toll road project.
    fn standard_ppp_input() -> PppModelInput {
        PppModelInput {
            project_name: "Highway Concession Alpha".into(),
            total_capex: dec!(500_000_000),
            construction_period_months: 36,
            concession_years: 30,
            revenue_model: RevenueModel::AvailabilityPayment,
            annual_availability_payment: dec!(60_000_000),
            initial_traffic_volume: dec!(10_000_000),
            traffic_growth_rate: dec!(0.03),
            toll_rate: dec!(5),
            opex_pct_revenue: dec!(0.20),
            major_maintenance_reserve_pct: dec!(0.005),
            senior_debt_pct: dec!(0.70),
            senior_debt_rate: dec!(0.05),
            senior_debt_tenor_years: 20,
            mezzanine_debt_pct: dec!(0.10),
            mezzanine_rate: dec!(0.08),
            equity_pct: dec!(0.20),
            tax_rate: dec!(0.25),
            discount_rate: dec!(0.08),
            inflation_rate: dec!(0.02),
            availability_deductions_pct: dec!(0.02),
            lifecycle_cost_schedule: vec![],
        }
    }

    /// Demand-based variant.
    fn demand_based_input() -> PppModelInput {
        let mut input = standard_ppp_input();
        input.revenue_model = RevenueModel::DemandBased;
        input
    }

    /// Mixed revenue variant.
    fn mixed_revenue_input() -> PppModelInput {
        let mut input = standard_ppp_input();
        input.revenue_model = RevenueModel::Mixed;
        input.annual_availability_payment = dec!(30_000_000);
        input
    }

    #[test]
    fn test_basic_availability_ppp() {
        let input = standard_ppp_input();
        let result = model_ppp(&input).unwrap();

        assert!(
            result.project_irr > Decimal::ZERO,
            "Project IRR should be positive: {}",
            result.project_irr
        );
        assert!(
            result.equity_irr > Decimal::ZERO,
            "Equity IRR should be positive: {}",
            result.equity_irr
        );
    }

    #[test]
    fn test_projections_count() {
        let input = standard_ppp_input();
        let result = model_ppp(&input).unwrap();

        // 3 construction years (36 months rounded) + 27 operating = 30 total
        assert_eq!(result.annual_projections.len(), 30);
    }

    #[test]
    fn test_construction_years_zero_revenue() {
        let input = standard_ppp_input();
        let result = model_ppp(&input).unwrap();

        let construction_yrs = construction_years_from_months(input.construction_period_months);
        for proj in result
            .annual_projections
            .iter()
            .take(construction_yrs as usize)
        {
            assert_eq!(
                proj.revenue,
                Decimal::ZERO,
                "Year {} should have zero revenue",
                proj.year
            );
            assert_eq!(proj.ebitda, Decimal::ZERO);
        }
    }

    #[test]
    fn test_operating_years_positive_revenue() {
        let input = standard_ppp_input();
        let result = model_ppp(&input).unwrap();

        let construction_yrs = construction_years_from_months(input.construction_period_months);
        for proj in result
            .annual_projections
            .iter()
            .skip(construction_yrs as usize)
        {
            assert!(
                proj.revenue > Decimal::ZERO,
                "Year {} should have positive revenue, got {}",
                proj.year,
                proj.revenue
            );
        }
    }

    #[test]
    fn test_dscr_positive_during_debt_service() {
        let input = standard_ppp_input();
        let result = model_ppp(&input).unwrap();

        let construction_yrs = construction_years_from_months(input.construction_period_months);
        for proj in result
            .annual_projections
            .iter()
            .skip(construction_yrs as usize)
        {
            if proj.senior_debt_service > Decimal::ZERO {
                assert!(
                    proj.dscr > Decimal::ZERO,
                    "Year {} DSCR should be positive: {}",
                    proj.year,
                    proj.dscr
                );
            }
        }
    }

    #[test]
    fn test_dscr_min_less_than_avg() {
        let input = standard_ppp_input();
        let result = model_ppp(&input).unwrap();

        assert!(
            result.senior_dscr_min <= result.senior_dscr_avg,
            "Min DSCR {} should be <= avg DSCR {}",
            result.senior_dscr_min,
            result.senior_dscr_avg
        );
    }

    #[test]
    fn test_llcr_positive() {
        let input = standard_ppp_input();
        let result = model_ppp(&input).unwrap();

        assert!(
            result.llcr > Decimal::ZERO,
            "LLCR should be positive: {}",
            result.llcr
        );
    }

    #[test]
    fn test_plcr_gte_llcr() {
        let input = standard_ppp_input();
        let result = model_ppp(&input).unwrap();

        assert!(
            result.plcr >= result.llcr,
            "PLCR ({}) should be >= LLCR ({})",
            result.plcr,
            result.llcr
        );
    }

    #[test]
    fn test_equity_multiple_positive() {
        let input = standard_ppp_input();
        let result = model_ppp(&input).unwrap();

        assert!(
            result.equity_multiple > Decimal::ZERO,
            "Equity multiple should be positive: {}",
            result.equity_multiple
        );
    }

    #[test]
    fn test_equity_multiple_gt_one() {
        let input = standard_ppp_input();
        let result = model_ppp(&input).unwrap();

        assert!(
            result.equity_multiple > Decimal::ONE,
            "Equity multiple should be > 1 for a viable project: {}",
            result.equity_multiple
        );
    }

    #[test]
    fn test_payback_within_project_life() {
        let input = standard_ppp_input();
        let result = model_ppp(&input).unwrap();

        let construction_yrs = construction_years_from_months(input.construction_period_months);
        let op_years = input.concession_years - construction_yrs;
        assert!(
            result.payback_period_years <= Decimal::from(op_years)
                || result.payback_period_years == dec!(999),
            "Payback {} should be within operating life {}",
            result.payback_period_years,
            op_years
        );
    }

    #[test]
    fn test_payback_achievable() {
        let input = standard_ppp_input();
        let result = model_ppp(&input).unwrap();

        assert!(
            result.payback_period_years < dec!(999),
            "Payback should be achievable for a standard project"
        );
    }

    #[test]
    fn test_demand_based_revenue() {
        let input = demand_based_input();
        let result = model_ppp(&input).unwrap();

        assert!(result.project_irr > Decimal::ZERO);
        // First operating year revenue: 10M * 5 = 50M
        let construction_yrs = construction_years_from_months(input.construction_period_months);
        let first_op = &result.annual_projections[construction_yrs as usize];
        let expected = dec!(10_000_000) * dec!(5); // traffic * toll
        assert_eq!(first_op.revenue, expected);
    }

    #[test]
    fn test_mixed_revenue_model() {
        let input = mixed_revenue_input();
        let result = model_ppp(&input).unwrap();

        assert!(result.project_irr > Decimal::ZERO);
        // Mixed should combine both streams
        let construction_yrs = construction_years_from_months(input.construction_period_months);
        let first_op = &result.annual_projections[construction_yrs as usize];
        let avail = dec!(30_000_000) * (Decimal::ONE - dec!(0.02));
        let demand = dec!(10_000_000) * dec!(5);
        let expected = avail + demand;
        assert_eq!(first_op.revenue, expected);
    }

    #[test]
    fn test_risk_allocation_availability() {
        let input = standard_ppp_input();
        let result = model_ppp(&input).unwrap();

        assert_eq!(result.risk_allocation.len(), 6);
        let demand_risk = result
            .risk_allocation
            .iter()
            .find(|r| r.risk.contains("Demand"))
            .unwrap();
        assert_eq!(demand_risk.allocated_to, "Government");
    }

    #[test]
    fn test_risk_allocation_demand() {
        let input = demand_based_input();
        let result = model_ppp(&input).unwrap();

        let demand_risk = result
            .risk_allocation
            .iter()
            .find(|r| r.risk.contains("Demand"))
            .unwrap();
        assert_eq!(demand_risk.allocated_to, "SPV / Concessionaire");
    }

    #[test]
    fn test_vfm_score_computed() {
        let input = standard_ppp_input();
        let result = model_ppp(&input).unwrap();

        // VfM score should be non-zero (traditional procurement is assumed more expensive)
        // The sign depends on the NPV but the computation should not be zero
        assert!(
            result.vfm_score != Decimal::ZERO,
            "VfM score should be non-zero"
        );
    }

    #[test]
    fn test_no_mezzanine_debt() {
        let mut input = standard_ppp_input();
        input.mezzanine_debt_pct = Decimal::ZERO;
        input.mezzanine_rate = Decimal::ZERO;
        input.equity_pct = dec!(0.30);

        let result = model_ppp(&input).unwrap();

        let construction_yrs = construction_years_from_months(input.construction_period_months);
        for proj in result
            .annual_projections
            .iter()
            .skip(construction_yrs as usize)
        {
            assert_eq!(
                proj.mezz_debt_service,
                Decimal::ZERO,
                "Year {} should have zero mezz service",
                proj.year
            );
        }
    }

    #[test]
    fn test_validation_zero_capex() {
        let mut input = standard_ppp_input();
        input.total_capex = Decimal::ZERO;

        let result = model_ppp(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "total_capex");
            }
            other => panic!("Expected InvalidInput, got: {other:?}"),
        }
    }

    #[test]
    fn test_validation_zero_concession() {
        let mut input = standard_ppp_input();
        input.concession_years = 0;

        let result = model_ppp(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_funding_mismatch() {
        let mut input = standard_ppp_input();
        input.senior_debt_pct = dec!(0.50);
        input.mezzanine_debt_pct = dec!(0.10);
        input.equity_pct = dec!(0.10); // Sums to 70%, not 100%

        let result = model_ppp(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_negative_rate() {
        let mut input = standard_ppp_input();
        input.senior_debt_rate = dec!(-0.05);

        let result = model_ppp(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_invalid_tax_rate() {
        let mut input = standard_ppp_input();
        input.tax_rate = dec!(1.5);

        let result = model_ppp(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_lifecycle_costs_applied() {
        let mut input = standard_ppp_input();
        // Major lifecycle cost in year 10 of operations
        let mut lifecycle = vec![Decimal::ZERO; 9];
        lifecycle.push(dec!(20_000_000)); // Year 10
        input.lifecycle_cost_schedule = lifecycle;

        let result_with = model_ppp(&input).unwrap();

        input.lifecycle_cost_schedule = vec![];
        let result_without = model_ppp(&input).unwrap();

        // Year 10 of operations (year 13 overall with 3 yr construction)
        let construction_yrs = construction_years_from_months(input.construction_period_months);
        let yr10_idx = (construction_yrs + 10 - 1) as usize;

        let ebitda_with = result_with.annual_projections[yr10_idx].ebitda;
        let ebitda_without = result_without.annual_projections[yr10_idx].ebitda;

        // EBITDA should be lower with lifecycle costs
        assert!(
            ebitda_with < ebitda_without,
            "Lifecycle costs should reduce EBITDA: {} vs {}",
            ebitda_with,
            ebitda_without
        );
    }

    #[test]
    fn test_inflation_escalation() {
        let mut input = standard_ppp_input();
        input.inflation_rate = dec!(0.03);
        let result = model_ppp(&input).unwrap();

        let construction_yrs = construction_years_from_months(input.construction_period_months);
        let ops: Vec<&PppYearProjection> = result
            .annual_projections
            .iter()
            .skip(construction_yrs as usize)
            .collect();

        // Revenue should increase over time due to inflation
        if ops.len() >= 3 {
            assert!(
                ops[2].revenue > ops[0].revenue,
                "Revenue should grow with inflation: yr1={} yr3={}",
                ops[0].revenue,
                ops[2].revenue
            );
        }
    }

    #[test]
    fn test_availability_deductions() {
        let mut input = standard_ppp_input();
        input.availability_deductions_pct = dec!(0.10); // 10% deductions
        let result_high = model_ppp(&input).unwrap();

        input.availability_deductions_pct = Decimal::ZERO;
        let result_zero = model_ppp(&input).unwrap();

        let construction_yrs = construction_years_from_months(input.construction_period_months);
        let rev_high = result_high.annual_projections[construction_yrs as usize].revenue;
        let rev_zero = result_zero.annual_projections[construction_yrs as usize].revenue;

        assert!(
            rev_high < rev_zero,
            "Higher deductions should reduce revenue: {} vs {}",
            rev_high,
            rev_zero
        );
    }

    #[test]
    fn test_cumulative_equity_cf() {
        let input = standard_ppp_input();
        let result = model_ppp(&input).unwrap();

        let mut cumulative = Decimal::ZERO;
        for proj in &result.annual_projections {
            cumulative += proj.equity_cash_flow;
            let diff = (proj.cumulative_equity_cf - cumulative).abs();
            assert!(
                diff < dec!(0.01),
                "Year {}: cumulative {} != expected {}",
                proj.year,
                proj.cumulative_equity_cf,
                cumulative
            );
        }
    }

    #[test]
    fn test_annuity_payment_zero_rate() {
        let pmt = compute_annuity_payment(dec!(1_000_000), Decimal::ZERO, 10);
        assert_eq!(pmt, dec!(100_000));
    }

    #[test]
    fn test_annuity_payment_normal() {
        let pmt = compute_annuity_payment(dec!(1_000_000), dec!(0.05), 10);
        // PMT should be approximately 129,505
        assert!(pmt > dec!(129_000) && pmt < dec!(130_000), "PMT = {}", pmt);
    }

    #[test]
    fn test_construction_months_rounding() {
        assert_eq!(construction_years_from_months(0), 0);
        assert_eq!(construction_years_from_months(1), 1);
        assert_eq!(construction_years_from_months(12), 1);
        assert_eq!(construction_years_from_months(13), 2);
        assert_eq!(construction_years_from_months(24), 2);
        assert_eq!(construction_years_from_months(25), 3);
        assert_eq!(construction_years_from_months(36), 3);
    }

    #[test]
    fn test_no_construction_period() {
        let mut input = standard_ppp_input();
        input.construction_period_months = 0;

        let result = model_ppp(&input).unwrap();

        // All years should be operating
        assert_eq!(
            result.annual_projections.len(),
            input.concession_years as usize
        );
        for proj in &result.annual_projections {
            assert!(proj.revenue > Decimal::ZERO || proj.year == 0);
        }
    }

    #[test]
    fn test_project_npv_sign() {
        let input = standard_ppp_input();
        let result = model_ppp(&input).unwrap();

        // If project IRR > discount rate, NPV should be positive
        if result.project_irr > input.discount_rate {
            assert!(
                result.project_npv > Decimal::ZERO,
                "NPV should be positive when IRR > discount rate"
            );
        }
    }

    #[test]
    fn test_total_equity_return_matches_sum() {
        let input = standard_ppp_input();
        let result = model_ppp(&input).unwrap();

        let construction_yrs = construction_years_from_months(input.construction_period_months);
        let sum: Decimal = result
            .annual_projections
            .iter()
            .filter(|p| p.year > construction_yrs)
            .map(|p| p.equity_cash_flow)
            .sum();

        let diff = (result.total_equity_return - sum).abs();
        assert!(
            diff < dec!(0.01),
            "Total equity return {} should match sum of projections {}",
            result.total_equity_return,
            sum
        );
    }

    #[test]
    fn test_equity_irr_gte_project_irr_with_leverage() {
        let input = standard_ppp_input();
        let result = model_ppp(&input).unwrap();

        // With positive leverage (project return > cost of debt), equity IRR > project IRR
        if result.project_irr > input.senior_debt_rate {
            assert!(
                result.equity_irr >= result.project_irr,
                "Equity IRR ({}) should >= Project IRR ({}) with positive leverage",
                result.equity_irr,
                result.project_irr
            );
        }
    }

    #[test]
    fn test_senior_debt_service_only_during_tenor() {
        let input = standard_ppp_input();
        let result = model_ppp(&input).unwrap();

        let construction_yrs = construction_years_from_months(input.construction_period_months);
        let tenor = input.senior_debt_tenor_years;

        for proj in result
            .annual_projections
            .iter()
            .skip(construction_yrs as usize)
        {
            let op_year = proj.year - construction_yrs;
            if op_year > tenor {
                assert_eq!(
                    proj.senior_debt_service,
                    Decimal::ZERO,
                    "Year {} (op_yr {}): no debt service after tenor",
                    proj.year,
                    op_year
                );
            }
        }
    }

    #[test]
    fn test_concession_shorter_than_construction_fails() {
        let mut input = standard_ppp_input();
        input.construction_period_months = 120; // 10 years
        input.concession_years = 8; // Only 8 years total

        let result = model_ppp(&input);
        assert!(result.is_err());
    }
}
