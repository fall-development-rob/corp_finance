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

/// Whether to compute PBO (projected with salary growth) or ABO (current salaries).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObligationType {
    Pbo,
    Abo,
}

/// An active employee participating in the pension plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Participant {
    pub name: String,
    pub current_age: u32,
    pub retirement_age: u32,
    pub years_of_service: u32,
    pub current_salary: Money,
}

/// A retired participant receiving benefit payments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Retiree {
    pub name: String,
    pub current_age: u32,
    pub life_expectancy: u32,
    pub annual_benefit: Money,
}

/// Plan rules that determine benefit calculations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanProvisions {
    /// Benefit accrual rate per year of service (e.g. 0.015 = 1.5%).
    pub benefit_formula_pct: Rate,
    pub early_retirement_age: u32,
    pub normal_retirement_age: u32,
    pub vesting_years: u32,
    /// Cost-of-living adjustment applied to retiree benefits.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cola_rate: Option<Rate>,
}

/// Regulatory constraints on contributions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributionConstraints {
    /// Must fund to at least this % of PBO (e.g. 0.80 = 80%).
    pub minimum_funding_pct: Rate,
    /// Can deduct contributions up to this % of PBO (e.g. 1.50 = 150%).
    pub maximum_deductible_pct: Rate,
    /// Corridor percentage for amortization (e.g. 0.10 = 10%).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub corridor_pct: Option<Rate>,
}

/// Top-level input for pension funding analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PensionFundingInput {
    pub plan_name: String,
    /// Fair value of plan assets.
    pub plan_assets: Money,
    /// Discount rate for computing present values of obligations.
    pub discount_rate: Rate,
    /// Long-term expected return on plan assets.
    pub expected_return_on_assets: Rate,
    /// Expected annual salary increase rate.
    pub salary_growth_rate: Rate,
    /// General inflation rate.
    pub inflation_rate: Rate,
    /// Whether to report PBO or ABO as the primary obligation.
    pub benefit_obligation_type: ObligationType,
    /// Active employees.
    pub active_participants: Vec<Participant>,
    /// Retirees currently receiving benefits.
    pub retired_participants: Vec<Retiree>,
    /// Plan rules.
    pub plan_provisions: PlanProvisions,
    /// Optional regulatory contribution constraints.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contribution_constraints: Option<ContributionConstraints>,
}

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// Summary statistics for plan participants.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantSummary {
    pub active_count: u32,
    pub retired_count: u32,
    pub total_active_pbo: Money,
    pub total_retiree_pbo: Money,
    pub weighted_avg_age_active: Decimal,
    pub weighted_avg_service: Decimal,
    pub weighted_avg_age_retired: Decimal,
}

/// Liability broken down by age cohort.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohortLiability {
    pub cohort: String,
    pub pbo: Money,
    pub abo: Money,
    pub duration_years: Decimal,
}

/// Complete output of pension funding analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PensionFundingOutput {
    pub plan_name: String,
    /// Projected Benefit Obligation (salary growth included).
    pub projected_benefit_obligation: Money,
    /// Accumulated Benefit Obligation (current salaries only).
    pub accumulated_benefit_obligation: Money,
    pub plan_assets: Money,
    /// Assets minus PBO; positive means overfunded.
    pub funded_status: Money,
    /// Assets / PBO.
    pub funding_ratio: Rate,
    /// max(0, PBO - assets).
    pub unfunded_liability: Money,
    /// Current-year service cost.
    pub service_cost: Money,
    /// Interest cost = discount_rate * PBO.
    pub interest_cost: Money,
    /// Expected return = expected_return_on_assets * plan_assets.
    pub expected_return: Money,
    /// Net Periodic Pension Cost = service + interest - expected return.
    pub net_periodic_pension_cost: Money,
    /// Contribution needed to meet minimum funding requirement.
    pub minimum_required_contribution: Money,
    /// Maximum tax-deductible contribution.
    pub maximum_deductible_contribution: Money,
    pub participant_summary: ParticipantSummary,
    pub liability_by_cohort: Vec<CohortLiability>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Iterative compound: (1 + r)^n using multiplication.
fn compound(rate: Decimal, n: u32) -> Decimal {
    let factor = dec!(1) + rate;
    let mut result = dec!(1);
    for _ in 0..n {
        result *= factor;
    }
    result
}

/// Discount factor: 1 / (1 + r)^n.
fn discount_factor(rate: Decimal, n: u32) -> Decimal {
    let c = compound(rate, n);
    if c == dec!(0) {
        return dec!(0);
    }
    dec!(1) / c
}

/// Present-value annuity factor: sum_{t=1..n} 1/(1+r)^t.
fn annuity_factor(rate: Decimal, n: u32) -> Decimal {
    let mut total = dec!(0);
    let mut df = dec!(1);
    let factor = dec!(1) + rate;
    for _ in 0..n {
        df /= factor;
        total += df;
    }
    total
}

/// Compute COLA-adjusted annuity factor.
/// PV = sum_{t=1..n} (1+cola)^t / (1+r)^t.
fn cola_annuity_factor(rate: Decimal, cola: Decimal, n: u32) -> Decimal {
    let mut total = dec!(0);
    let mut cola_accum = dec!(1);
    let cola_factor = dec!(1) + cola;
    let disc_factor = dec!(1) + rate;
    let mut disc_accum = dec!(1);
    for _ in 0..n {
        cola_accum *= cola_factor;
        disc_accum *= disc_factor;
        total += cola_accum / disc_accum;
    }
    total
}

/// Weighted-average duration of an annuity: sum_{t=1..n} t / (1+r)^t / annuity_factor.
fn annuity_duration(rate: Decimal, n: u32) -> Decimal {
    let af = annuity_factor(rate, n);
    if af == dec!(0) {
        return dec!(0);
    }
    let mut numerator = dec!(0);
    let factor = dec!(1) + rate;
    let mut df = dec!(1);
    for t in 1..=n {
        df /= factor;
        numerator += Decimal::from(t) * df;
    }
    numerator / af
}

/// Assumed life expectancy if not provided: 85 years.
const DEFAULT_LIFE_EXPECTANCY: u32 = 85;

// ---------------------------------------------------------------------------
// Core function
// ---------------------------------------------------------------------------

/// Analyse pension plan funding status, obligations, and required contributions.
pub fn analyze_pension_funding(
    input: &PensionFundingInput,
) -> CorpFinanceResult<ComputationOutput<PensionFundingOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // -- Validation ----------------------------------------------------------
    if input.discount_rate <= dec!(0) {
        return Err(CorpFinanceError::InvalidInput {
            field: "discount_rate".into(),
            reason: "Must be positive".into(),
        });
    }
    if input.plan_assets < dec!(0) {
        return Err(CorpFinanceError::InvalidInput {
            field: "plan_assets".into(),
            reason: "Cannot be negative".into(),
        });
    }
    if input.active_participants.is_empty() && input.retired_participants.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "participants".into(),
            reason: "At least one active participant or retiree is required".into(),
        });
    }
    for p in &input.active_participants {
        if p.retirement_age <= p.current_age {
            return Err(CorpFinanceError::InvalidInput {
                field: "retirement_age".into(),
                reason: format!(
                    "Participant '{}' has retirement_age ({}) <= current_age ({})",
                    p.name, p.retirement_age, p.current_age
                ),
            });
        }
    }

    let provisions = &input.plan_provisions;
    let cola = provisions.cola_rate.unwrap_or(dec!(0));

    // -- Active participant obligations --------------------------------------
    let mut total_active_pbo = dec!(0);
    let mut total_active_abo = dec!(0);
    let mut active_age_sum = dec!(0);
    let mut active_service_sum = dec!(0);
    let mut active_salary_sum = dec!(0);

    // For cohort tracking: buckets by decade
    let mut cohort_pbo: std::collections::BTreeMap<String, Decimal> =
        std::collections::BTreeMap::new();
    let mut cohort_abo: std::collections::BTreeMap<String, Decimal> =
        std::collections::BTreeMap::new();
    let mut cohort_dur_sum: std::collections::BTreeMap<String, Decimal> =
        std::collections::BTreeMap::new();
    let mut cohort_count: std::collections::BTreeMap<String, u32> =
        std::collections::BTreeMap::new();

    for p in &input.active_participants {
        let years_to_retirement = p.retirement_age - p.current_age;
        let total_service_at_retirement = p.years_of_service + years_to_retirement;

        // Projected final salary (PBO)
        let projected_salary =
            p.current_salary * compound(input.salary_growth_rate, years_to_retirement);
        // ABO uses current salary
        let abo_salary = p.current_salary;

        // Annual benefit at retirement
        let pbo_benefit = projected_salary
            * provisions.benefit_formula_pct
            * Decimal::from(total_service_at_retirement);
        let abo_benefit =
            abo_salary * provisions.benefit_formula_pct * Decimal::from(p.years_of_service);

        // Life expectancy at retirement: assume DEFAULT_LIFE_EXPECTANCY
        let payment_years = DEFAULT_LIFE_EXPECTANCY.saturating_sub(p.retirement_age);

        // PV of benefit annuity at retirement
        let af = if cola > dec!(0) {
            cola_annuity_factor(input.discount_rate, cola, payment_years)
        } else {
            annuity_factor(input.discount_rate, payment_years)
        };

        let pv_pbo_at_retirement = pbo_benefit * af;
        let pv_abo_at_retirement = abo_benefit * af;

        // Prorate by fraction of service earned: years_served / total_service
        let fraction = if total_service_at_retirement > 0 {
            Decimal::from(p.years_of_service) / Decimal::from(total_service_at_retirement)
        } else {
            dec!(0)
        };

        // Discount back to today
        let df = discount_factor(input.discount_rate, years_to_retirement);

        let participant_pbo = pv_pbo_at_retirement * fraction * df;
        let participant_abo = pv_abo_at_retirement * df;

        total_active_pbo += participant_pbo;
        total_active_abo += participant_abo;

        active_age_sum += Decimal::from(p.current_age) * p.current_salary;
        active_service_sum += Decimal::from(p.years_of_service) * p.current_salary;
        active_salary_sum += p.current_salary;

        // Cohort by decade
        let decade = (p.current_age / 10) * 10;
        let cohort_key = format!("{}-{}", decade, decade + 9);
        *cohort_pbo.entry(cohort_key.clone()).or_insert(dec!(0)) += participant_pbo;
        *cohort_abo.entry(cohort_key.clone()).or_insert(dec!(0)) += participant_abo;

        // Duration for this participant: years_to_retirement + annuity duration at retirement
        let participant_duration = Decimal::from(years_to_retirement)
            + annuity_duration(input.discount_rate, payment_years);
        *cohort_dur_sum.entry(cohort_key.clone()).or_insert(dec!(0)) += participant_duration;
        *cohort_count.entry(cohort_key).or_insert(0) += 1;
    }

    // -- Retiree obligations -------------------------------------------------
    let mut total_retiree_pbo = dec!(0);
    let mut total_retiree_abo = dec!(0);
    let mut retiree_age_sum = dec!(0);
    let mut retiree_benefit_sum = dec!(0);

    for r in &input.retired_participants {
        let remaining = r.life_expectancy.saturating_sub(r.current_age);

        let pv = if cola > dec!(0) {
            r.annual_benefit * cola_annuity_factor(input.discount_rate, cola, remaining)
        } else {
            r.annual_benefit * annuity_factor(input.discount_rate, remaining)
        };

        total_retiree_pbo += pv;
        total_retiree_abo += pv; // ABO = PBO for retirees (no salary growth)

        retiree_age_sum += Decimal::from(r.current_age) * r.annual_benefit;
        retiree_benefit_sum += r.annual_benefit;

        // Retiree cohort
        let decade = (r.current_age / 10) * 10;
        let cohort_key = format!("{}-{}", decade, decade + 9);
        *cohort_pbo.entry(cohort_key.clone()).or_insert(dec!(0)) += pv;
        *cohort_abo.entry(cohort_key.clone()).or_insert(dec!(0)) += pv;

        let dur = annuity_duration(input.discount_rate, remaining);
        *cohort_dur_sum.entry(cohort_key.clone()).or_insert(dec!(0)) += dur;
        *cohort_count.entry(cohort_key).or_insert(0) += 1;
    }

    // -- Totals --------------------------------------------------------------
    let pbo = total_active_pbo + total_retiree_pbo;
    let abo = total_active_abo + total_retiree_abo;

    let funded_status = input.plan_assets - pbo;
    let funding_ratio = if pbo > dec!(0) {
        input.plan_assets / pbo
    } else {
        dec!(1)
    };
    let unfunded_liability = if pbo > input.plan_assets {
        pbo - input.plan_assets
    } else {
        dec!(0)
    };

    // -- Pension cost components ---------------------------------------------
    // Service cost: approximate as PBO / weighted average remaining service years
    let avg_remaining_service = if !input.active_participants.is_empty() {
        let total_remaining: u32 = input
            .active_participants
            .iter()
            .map(|p| p.retirement_age - p.current_age)
            .sum();
        Decimal::from(total_remaining) / Decimal::from(input.active_participants.len() as u32)
    } else {
        dec!(1)
    };

    let service_cost = if avg_remaining_service > dec!(0) {
        total_active_pbo / avg_remaining_service
    } else {
        dec!(0)
    };

    let interest_cost = input.discount_rate * pbo;
    let expected_return = input.expected_return_on_assets * input.plan_assets;
    let nppc = service_cost + interest_cost - expected_return;

    // -- Contributions -------------------------------------------------------
    let (min_contribution, max_contribution) = match &input.contribution_constraints {
        Some(constraints) => {
            let target_min = constraints.minimum_funding_pct * pbo;
            let min_req = if input.plan_assets < target_min {
                target_min - input.plan_assets
            } else {
                dec!(0)
            };
            let target_max = constraints.maximum_deductible_pct * pbo;
            let max_ded = if target_max > input.plan_assets {
                target_max - input.plan_assets
            } else {
                dec!(0)
            };
            (min_req, max_ded)
        }
        None => (dec!(0), dec!(0)),
    };

    // -- Participant summary -------------------------------------------------
    let weighted_avg_age_active = if active_salary_sum > dec!(0) {
        active_age_sum / active_salary_sum
    } else {
        dec!(0)
    };
    let weighted_avg_service = if active_salary_sum > dec!(0) {
        active_service_sum / active_salary_sum
    } else {
        dec!(0)
    };
    let weighted_avg_age_retired = if retiree_benefit_sum > dec!(0) {
        retiree_age_sum / retiree_benefit_sum
    } else {
        dec!(0)
    };

    let participant_summary = ParticipantSummary {
        active_count: input.active_participants.len() as u32,
        retired_count: input.retired_participants.len() as u32,
        total_active_pbo,
        total_retiree_pbo,
        weighted_avg_age_active,
        weighted_avg_service,
        weighted_avg_age_retired,
    };

    // -- Cohort liabilities --------------------------------------------------
    let liability_by_cohort: Vec<CohortLiability> = cohort_pbo
        .keys()
        .map(|key| {
            let cpbo = *cohort_pbo.get(key).unwrap_or(&dec!(0));
            let cabo = *cohort_abo.get(key).unwrap_or(&dec!(0));
            let dur_sum = *cohort_dur_sum.get(key).unwrap_or(&dec!(0));
            let count = *cohort_count.get(key).unwrap_or(&1);
            let avg_dur = if count > 0 {
                dur_sum / Decimal::from(count)
            } else {
                dec!(0)
            };
            CohortLiability {
                cohort: key.clone(),
                pbo: cpbo,
                abo: cabo,
                duration_years: avg_dur,
            }
        })
        .collect();

    // -- Warnings ------------------------------------------------------------
    if funding_ratio < dec!(0.8) {
        warnings.push(format!(
            "Plan is critically underfunded at {:.1}%",
            funding_ratio * dec!(100)
        ));
    }
    if input.expected_return_on_assets > input.discount_rate + dec!(0.02) {
        warnings.push(
            "Expected return on assets exceeds discount rate by more than 200bp — may be aggressive"
                .into(),
        );
    }

    let output = PensionFundingOutput {
        plan_name: input.plan_name.clone(),
        projected_benefit_obligation: pbo,
        accumulated_benefit_obligation: abo,
        plan_assets: input.plan_assets,
        funded_status,
        funding_ratio,
        unfunded_liability,
        service_cost,
        interest_cost,
        expected_return,
        net_periodic_pension_cost: nppc,
        minimum_required_contribution: min_contribution,
        maximum_deductible_contribution: max_contribution,
        participant_summary,
        liability_by_cohort,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Pension Funding Analysis (PBO/ABO with annuity-factor discounting)",
        &serde_json::json!({
            "discount_rate": input.discount_rate.to_string(),
            "salary_growth_rate": input.salary_growth_rate.to_string(),
            "expected_return_on_assets": input.expected_return_on_assets.to_string(),
            "benefit_formula_pct": input.plan_provisions.benefit_formula_pct.to_string(),
            "obligation_type": format!("{:?}", input.benefit_obligation_type),
            "life_expectancy_assumption": DEFAULT_LIFE_EXPECTANCY,
        }),
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn basic_provisions() -> PlanProvisions {
        PlanProvisions {
            benefit_formula_pct: dec!(0.015),
            early_retirement_age: 55,
            normal_retirement_age: 65,
            vesting_years: 5,
            cola_rate: None,
        }
    }

    fn single_active() -> Vec<Participant> {
        vec![Participant {
            name: "Alice".into(),
            current_age: 45,
            retirement_age: 65,
            years_of_service: 10,
            current_salary: dec!(100000),
        }]
    }

    fn single_retiree() -> Vec<Retiree> {
        vec![Retiree {
            name: "Bob".into(),
            current_age: 67,
            life_expectancy: 85,
            annual_benefit: dec!(30000),
        }]
    }

    fn basic_input() -> PensionFundingInput {
        PensionFundingInput {
            plan_name: "Test Plan".into(),
            plan_assets: dec!(500000),
            discount_rate: dec!(0.05),
            expected_return_on_assets: dec!(0.07),
            salary_growth_rate: dec!(0.03),
            inflation_rate: dec!(0.02),
            benefit_obligation_type: ObligationType::Pbo,
            active_participants: single_active(),
            retired_participants: single_retiree(),
            plan_provisions: basic_provisions(),
            contribution_constraints: None,
        }
    }

    #[test]
    fn test_basic_pbo_positive() {
        let result = analyze_pension_funding(&basic_input()).unwrap();
        assert!(result.result.projected_benefit_obligation > dec!(0));
    }

    #[test]
    fn test_abo_less_than_or_equal_pbo() {
        let result = analyze_pension_funding(&basic_input()).unwrap();
        let r = &result.result;
        assert!(r.accumulated_benefit_obligation <= r.projected_benefit_obligation);
    }

    #[test]
    fn test_funded_status_calculation() {
        let result = analyze_pension_funding(&basic_input()).unwrap();
        let r = &result.result;
        let expected = r.plan_assets - r.projected_benefit_obligation;
        assert_eq!(r.funded_status, expected);
    }

    #[test]
    fn test_funding_ratio() {
        let result = analyze_pension_funding(&basic_input()).unwrap();
        let r = &result.result;
        let expected = r.plan_assets / r.projected_benefit_obligation;
        assert_eq!(r.funding_ratio, expected);
    }

    #[test]
    fn test_unfunded_liability_when_underfunded() {
        let mut input = basic_input();
        input.plan_assets = dec!(10000); // very small
        let result = analyze_pension_funding(&input).unwrap();
        let r = &result.result;
        assert!(r.unfunded_liability > dec!(0));
        assert_eq!(
            r.unfunded_liability,
            r.projected_benefit_obligation - r.plan_assets
        );
    }

    #[test]
    fn test_unfunded_liability_when_overfunded() {
        let mut input = basic_input();
        input.plan_assets = dec!(99999999);
        let result = analyze_pension_funding(&input).unwrap();
        assert_eq!(result.result.unfunded_liability, dec!(0));
    }

    #[test]
    fn test_service_cost_positive() {
        let result = analyze_pension_funding(&basic_input()).unwrap();
        assert!(result.result.service_cost > dec!(0));
    }

    #[test]
    fn test_interest_cost() {
        let input = basic_input();
        let result = analyze_pension_funding(&input).unwrap();
        let r = &result.result;
        let expected = input.discount_rate * r.projected_benefit_obligation;
        assert_eq!(r.interest_cost, expected);
    }

    #[test]
    fn test_expected_return() {
        let input = basic_input();
        let result = analyze_pension_funding(&input).unwrap();
        let r = &result.result;
        let expected = input.expected_return_on_assets * input.plan_assets;
        assert_eq!(r.expected_return, expected);
    }

    #[test]
    fn test_nppc_formula() {
        let result = analyze_pension_funding(&basic_input()).unwrap();
        let r = &result.result;
        let expected = r.service_cost + r.interest_cost - r.expected_return;
        assert_eq!(r.net_periodic_pension_cost, expected);
    }

    #[test]
    fn test_contribution_constraints_underfunded() {
        let mut input = basic_input();
        input.plan_assets = dec!(100000);
        input.contribution_constraints = Some(ContributionConstraints {
            minimum_funding_pct: dec!(0.80),
            maximum_deductible_pct: dec!(1.50),
            corridor_pct: None,
        });
        let result = analyze_pension_funding(&input).unwrap();
        let r = &result.result;
        assert!(r.minimum_required_contribution > dec!(0));
        assert!(r.maximum_deductible_contribution > dec!(0));
    }

    #[test]
    fn test_contribution_constraints_overfunded() {
        let mut input = basic_input();
        input.plan_assets = dec!(99999999);
        input.contribution_constraints = Some(ContributionConstraints {
            minimum_funding_pct: dec!(0.80),
            maximum_deductible_pct: dec!(1.50),
            corridor_pct: None,
        });
        let result = analyze_pension_funding(&input).unwrap();
        let r = &result.result;
        assert_eq!(r.minimum_required_contribution, dec!(0));
    }

    #[test]
    fn test_retiree_only_plan() {
        let mut input = basic_input();
        input.active_participants = vec![];
        let result = analyze_pension_funding(&input).unwrap();
        let r = &result.result;
        assert!(r.projected_benefit_obligation > dec!(0));
        assert_eq!(r.participant_summary.active_count, 0);
        assert_eq!(r.participant_summary.retired_count, 1);
    }

    #[test]
    fn test_active_only_plan() {
        let mut input = basic_input();
        input.retired_participants = vec![];
        let result = analyze_pension_funding(&input).unwrap();
        let r = &result.result;
        assert!(r.projected_benefit_obligation > dec!(0));
        assert_eq!(r.participant_summary.active_count, 1);
        assert_eq!(r.participant_summary.retired_count, 0);
    }

    #[test]
    fn test_cola_increases_obligation() {
        let no_cola_input = basic_input();
        let no_cola = analyze_pension_funding(&no_cola_input).unwrap();

        let mut cola_input = basic_input();
        cola_input.plan_provisions.cola_rate = Some(dec!(0.02));
        let with_cola = analyze_pension_funding(&cola_input).unwrap();

        assert!(
            with_cola.result.projected_benefit_obligation
                > no_cola.result.projected_benefit_obligation
        );
    }

    #[test]
    fn test_higher_discount_rate_lowers_pbo() {
        let low = basic_input();
        let low_result = analyze_pension_funding(&low).unwrap();

        let mut high = basic_input();
        high.discount_rate = dec!(0.08);
        let high_result = analyze_pension_funding(&high).unwrap();

        assert!(
            high_result.result.projected_benefit_obligation
                < low_result.result.projected_benefit_obligation
        );
    }

    #[test]
    fn test_participant_summary_counts() {
        let mut input = basic_input();
        input.active_participants.push(Participant {
            name: "Charlie".into(),
            current_age: 35,
            retirement_age: 65,
            years_of_service: 5,
            current_salary: dec!(80000),
        });
        let result = analyze_pension_funding(&input).unwrap();
        assert_eq!(result.result.participant_summary.active_count, 2);
        assert_eq!(result.result.participant_summary.retired_count, 1);
    }

    #[test]
    fn test_cohort_liabilities_populated() {
        let result = analyze_pension_funding(&basic_input()).unwrap();
        assert!(!result.result.liability_by_cohort.is_empty());
    }

    #[test]
    fn test_cohort_pbo_sums_to_total() {
        let result = analyze_pension_funding(&basic_input()).unwrap();
        let r = &result.result;
        let cohort_sum: Decimal = r.liability_by_cohort.iter().map(|c| c.pbo).sum();
        let diff = (cohort_sum - r.projected_benefit_obligation).abs();
        assert!(diff < dec!(0.01), "Cohort PBO sum should match total PBO");
    }

    #[test]
    fn test_duration_positive() {
        let result = analyze_pension_funding(&basic_input()).unwrap();
        for cohort in &result.result.liability_by_cohort {
            assert!(cohort.duration_years >= dec!(0));
        }
    }

    #[test]
    fn test_validation_negative_discount_rate() {
        let mut input = basic_input();
        input.discount_rate = dec!(-0.01);
        let err = analyze_pension_funding(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => assert_eq!(field, "discount_rate"),
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[test]
    fn test_validation_negative_assets() {
        let mut input = basic_input();
        input.plan_assets = dec!(-1);
        let err = analyze_pension_funding(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => assert_eq!(field, "plan_assets"),
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[test]
    fn test_validation_no_participants() {
        let mut input = basic_input();
        input.active_participants = vec![];
        input.retired_participants = vec![];
        let err = analyze_pension_funding(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => assert_eq!(field, "participants"),
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[test]
    fn test_validation_retirement_age_before_current() {
        let mut input = basic_input();
        input.active_participants[0].retirement_age = 40; // less than current_age 45
        let err = analyze_pension_funding(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => assert_eq!(field, "retirement_age"),
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[test]
    fn test_warning_on_critically_underfunded() {
        let mut input = basic_input();
        input.plan_assets = dec!(10000);
        let result = analyze_pension_funding(&input).unwrap();
        assert!(result.warnings.iter().any(|w| w.contains("underfunded")));
    }
}
