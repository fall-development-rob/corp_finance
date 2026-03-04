use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{with_metadata, ComputationOutput, Money, Rate};
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Escalation structure governing how base rent grows over the lease term.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EscalationType {
    /// Fixed annual percentage step-up (e.g. 3 % per annum).
    FixedStep { annual_increase_pct: Rate },
    /// CPI-linked bumps: base rent grows at assumed_cpi + spread.
    CpiLinked {
        spread_over_cpi: Rate,
        assumed_cpi: Rate,
    },
    /// Percentage rent: base minimum plus a share of tenant sales above a
    /// breakpoint.
    PercentageRent {
        base_rent_psf: Money,
        pct_of_sales: Rate,
        breakpoint_sales: Money,
    },
    /// Flat rent with no escalation over the term.
    FlatRent,
}

/// A single tenant in a rent roll.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    pub name: String,
    pub suite: String,
    pub leased_sf: Decimal,
    pub base_rent_psf: Money,
    pub lease_start_year: u32,
    pub lease_end_year: u32,
    pub escalation: EscalationType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expense_stop_psf: Option<Money>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ti_allowance_psf: Option<Money>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ti_amortization_years: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub free_rent_months: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub renewal_probability: Option<Rate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub downtime_months: Option<u32>,
    /// Credit quality score 1 (worst) to 10 (best).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credit_score: Option<Decimal>,
    /// Projected annual sales for percentage-rent tenants.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub projected_annual_sales: Option<Money>,
}

/// Weighting methodology for WALT calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WaltWeighting {
    ByBaseRent,
    ByNra,
    ByBoth,
}

// ---------------------------------------------------------------------------
// Inputs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantScheduleInput {
    pub tenant: Tenant,
    pub analysis_start_year: u32,
    pub holding_period_years: u32,
    /// Current operating expenses per SF (for pass-through calculation).
    pub current_opex_psf: Option<Money>,
    /// Annual growth rate of operating expenses.
    pub opex_growth_rate: Option<Rate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseRolloverInput {
    pub tenants: Vec<Tenant>,
    pub holding_period_years: u32,
    pub analysis_start_year: u32,
    /// Total building net rentable area in SF.
    pub total_building_sf: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenewalProbabilityInput {
    pub tenant_name: String,
    pub remaining_term_years: Decimal,
    /// Credit quality score 1-10.
    pub credit_score: Decimal,
    /// Current market vacancy rate (e.g. 0.08 = 8 %).
    pub market_vacancy_rate: Rate,
    /// In-place rent per SF.
    pub in_place_rent_psf: Money,
    /// Market rent per SF.
    pub market_rent_psf: Money,
    /// Leased SF (for vacancy cost).
    pub leased_sf: Decimal,
    /// Expected downtime months if tenant vacates.
    pub downtime_months: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkToMarketInput {
    pub tenants: Vec<MarkToMarketTenant>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkToMarketTenant {
    pub name: String,
    pub leased_sf: Decimal,
    pub in_place_rent_psf: Money,
    pub market_rent_psf: Money,
    pub remaining_lease_years: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaltInput {
    pub tenants: Vec<WaltTenant>,
    pub weighting: WaltWeighting,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaltTenant {
    pub name: String,
    pub leased_sf: Decimal,
    pub annual_base_rent: Money,
    pub remaining_lease_years: Decimal,
}

// ---------------------------------------------------------------------------
// Outputs
// ---------------------------------------------------------------------------

/// Single-year projection row in a tenant schedule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantYearRow {
    pub year: u32,
    pub base_rent_psf: Money,
    pub gross_rent: Money,
    pub free_rent_adjustment: Money,
    pub ti_amortization: Money,
    pub expense_recovery: Money,
    pub net_effective_rent: Money,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantScheduleOutput {
    pub tenant_name: String,
    pub schedule: Vec<TenantYearRow>,
    pub total_gross_rent: Money,
    pub total_net_effective_rent: Money,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RolloverYearRow {
    pub year: u32,
    pub expiring_sf: Decimal,
    pub expiring_rent: Money,
    pub pct_of_nra: Rate,
    pub pct_of_base_rent: Rate,
    pub cumulative_pct_nra: Rate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseRolloverOutput {
    pub annual_rollover: Vec<RolloverYearRow>,
    pub total_leased_sf: Decimal,
    pub total_base_rent: Money,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenewalProbabilityOutput {
    pub tenant_name: String,
    pub renewal_probability: Rate,
    pub vacancy_probability: Rate,
    pub expected_vacancy_cost: Money,
    pub probability_weighted_vacancy_cost: Money,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkToMarketRow {
    pub name: String,
    pub leased_sf: Decimal,
    pub in_place_rent_psf: Money,
    pub market_rent_psf: Money,
    pub delta_psf: Money,
    pub total_annual_delta: Money,
    pub remaining_lease_years: Decimal,
    pub term_weighted_delta: Money,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkToMarketOutput {
    pub tenants: Vec<MarkToMarketRow>,
    pub aggregate_annual_delta: Money,
    pub aggregate_term_weighted_delta: Money,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseTermBucket {
    pub label: String,
    pub sf: Decimal,
    pub pct_of_total: Rate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaltOutput {
    pub walt_years: Decimal,
    pub histogram: Vec<LeaseTermBucket>,
}

// ---------------------------------------------------------------------------
// 1. Tenant Schedule
// ---------------------------------------------------------------------------

/// Projects gross and net effective rent for a single tenant over a holding
/// period, modelling escalation, free rent, TI amortisation, expense stops,
/// and renewal downtime.
pub fn tenant_schedule(
    input: &TenantScheduleInput,
) -> CorpFinanceResult<ComputationOutput<TenantScheduleOutput>> {
    let start = Instant::now();
    let t = &input.tenant;
    let mut warnings: Vec<String> = Vec::new();

    if t.leased_sf <= dec!(0) {
        return Err(CorpFinanceError::InvalidInput {
            field: "leased_sf".into(),
            reason: "must be positive".into(),
        });
    }
    if t.lease_end_year < t.lease_start_year {
        return Err(CorpFinanceError::InvalidInput {
            field: "lease_end_year".into(),
            reason: "must be >= lease_start_year".into(),
        });
    }

    let hold_end = input.analysis_start_year + input.holding_period_years;
    let free_months = t.free_rent_months.unwrap_or(0);
    let ti_psf = t.ti_allowance_psf.unwrap_or(dec!(0));
    let ti_amort_yrs = t.ti_amortization_years.unwrap_or(0);
    let annual_ti_amort = if ti_amort_yrs > 0 {
        ti_psf * t.leased_sf / Decimal::from(ti_amort_yrs)
    } else {
        dec!(0)
    };

    let opex_psf = input.current_opex_psf.unwrap_or(dec!(0));
    let opex_growth = input.opex_growth_rate.unwrap_or(dec!(0));
    let expense_stop = t.expense_stop_psf.unwrap_or(dec!(0));

    let renewal_prob = t.renewal_probability.unwrap_or(dec!(0.70));
    let downtime = t.downtime_months.unwrap_or(6);

    let mut schedule = Vec::new();
    let mut total_gross = dec!(0);
    let mut total_net = dec!(0);
    let mut current_rent_psf = t.base_rent_psf;
    let mut year_index: u32 = 0;
    let mut in_downtime = false;
    let mut downtime_remaining: u32 = 0;

    for yr in input.analysis_start_year..hold_end {
        // Check if we are past the original lease end (renewal logic).
        if yr >= t.lease_end_year && !in_downtime && year_index > 0 {
            let vacancy_factor = dec!(1) - renewal_prob;
            if vacancy_factor > dec!(0) {
                in_downtime = true;
                downtime_remaining = downtime;
                warnings.push(format!(
                    "Year {}: lease expires, {:.0}% renewal probability applied",
                    yr,
                    renewal_prob * dec!(100)
                ));
            }
        }

        if in_downtime && downtime_remaining > 0 {
            let vacant_months = downtime_remaining.min(12);
            let occupied_fraction = dec!(1) - Decimal::from(vacant_months) / dec!(12);
            downtime_remaining = downtime_remaining.saturating_sub(12);
            if downtime_remaining == 0 {
                in_downtime = false;
            }
            let gross = current_rent_psf * t.leased_sf * occupied_fraction;
            let row = TenantYearRow {
                year: yr,
                base_rent_psf: current_rent_psf,
                gross_rent: gross,
                free_rent_adjustment: dec!(0),
                ti_amortization: dec!(0),
                expense_recovery: dec!(0),
                net_effective_rent: gross,
            };
            total_gross += gross;
            total_net += gross;
            schedule.push(row);
            year_index += 1;
            continue;
        }

        // Escalation (skip year 0).
        if year_index > 0 {
            match &t.escalation {
                EscalationType::FixedStep {
                    annual_increase_pct,
                } => {
                    current_rent_psf += current_rent_psf * annual_increase_pct;
                }
                EscalationType::CpiLinked {
                    spread_over_cpi,
                    assumed_cpi,
                } => {
                    let growth = *assumed_cpi + *spread_over_cpi;
                    current_rent_psf += current_rent_psf * growth;
                }
                EscalationType::PercentageRent { .. } | EscalationType::FlatRent => {}
            }
        }

        let mut gross = current_rent_psf * t.leased_sf;

        // Percentage rent overage.
        if let EscalationType::PercentageRent {
            pct_of_sales,
            breakpoint_sales,
            ..
        } = &t.escalation
        {
            if let Some(sales) = t.projected_annual_sales {
                if sales > *breakpoint_sales {
                    gross += (sales - *breakpoint_sales) * pct_of_sales;
                }
            }
        }

        // Free rent adjustment (applies in early months of the lease).
        let elapsed_months = year_index * 12;
        let free_adj = if elapsed_months < free_months {
            let remaining_free = free_months - elapsed_months;
            let months_this_year = remaining_free.min(12);
            gross * Decimal::from(months_this_year) / dec!(12)
        } else {
            dec!(0)
        };

        // Expense recovery (pass-through above stop).
        let mut opex_current = opex_psf;
        for _ in 0..year_index {
            opex_current += opex_current * opex_growth;
        }
        let recovery_psf = if opex_current > expense_stop && expense_stop > dec!(0) {
            opex_current - expense_stop
        } else {
            dec!(0)
        };
        let expense_recovery = recovery_psf * t.leased_sf;

        // TI amortization (landlord cost, reduces net).
        let ti_cost = if year_index < ti_amort_yrs {
            annual_ti_amort
        } else {
            dec!(0)
        };

        let net = gross - free_adj + expense_recovery - ti_cost;

        let row = TenantYearRow {
            year: yr,
            base_rent_psf: current_rent_psf,
            gross_rent: gross,
            free_rent_adjustment: free_adj,
            ti_amortization: ti_cost,
            expense_recovery,
            net_effective_rent: net,
        };
        total_gross += gross;
        total_net += net;
        schedule.push(row);
        year_index += 1;
    }

    let output = TenantScheduleOutput {
        tenant_name: t.name.clone(),
        schedule,
        total_gross_rent: total_gross,
        total_net_effective_rent: total_net,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Rent Roll — Tenant Schedule Projection",
        input,
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// 2. Lease Rollover
// ---------------------------------------------------------------------------

/// Aggregates a tenant list into a property-level lease expiration profile
/// showing annual rollover exposure by NRA and base rent.
pub fn lease_rollover(
    input: &LeaseRolloverInput,
) -> CorpFinanceResult<ComputationOutput<LeaseRolloverOutput>> {
    let start = Instant::now();
    let warnings: Vec<String> = Vec::new();

    if input.tenants.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "tenants list is empty".into(),
        ));
    }

    // RE-CONTRACT-001: sum of leased_sf must not exceed total_building_sf.
    let total_leased: Decimal = input.tenants.iter().map(|t| t.leased_sf).sum();
    if total_leased > input.total_building_sf {
        return Err(CorpFinanceError::InvalidInput {
            field: "tenants".into(),
            reason: format!(
                "sum of leased_sf ({}) exceeds total_building_sf ({})",
                total_leased, input.total_building_sf
            ),
        });
    }

    let total_base_rent: Money = input
        .tenants
        .iter()
        .map(|t| t.base_rent_psf * t.leased_sf)
        .sum();

    let hold_end = input.analysis_start_year + input.holding_period_years;
    let mut annual_rollover = Vec::new();
    let mut cum_pct_nra = dec!(0);

    for yr in input.analysis_start_year..hold_end {
        let mut expiring_sf = dec!(0);
        let mut expiring_rent = dec!(0);

        for t in &input.tenants {
            if t.lease_end_year == yr {
                expiring_sf += t.leased_sf;
                expiring_rent += t.base_rent_psf * t.leased_sf;
            }
        }

        let pct_nra = if input.total_building_sf > dec!(0) {
            expiring_sf / input.total_building_sf
        } else {
            dec!(0)
        };
        let pct_rent = if total_base_rent > dec!(0) {
            expiring_rent / total_base_rent
        } else {
            dec!(0)
        };
        cum_pct_nra += pct_nra;

        annual_rollover.push(RolloverYearRow {
            year: yr,
            expiring_sf,
            expiring_rent,
            pct_of_nra: pct_nra,
            pct_of_base_rent: pct_rent,
            cumulative_pct_nra: cum_pct_nra,
        });
    }

    let output = LeaseRolloverOutput {
        annual_rollover,
        total_leased_sf: total_leased,
        total_base_rent,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Rent Roll — Lease Rollover Profile",
        input,
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// 3. Renewal Probability
// ---------------------------------------------------------------------------

/// Estimates tenant renewal likelihood from credit quality, remaining term,
/// market vacancy, and rent spread.  Outputs probability-weighted vacancy cost.
pub fn renewal_probability(
    input: &RenewalProbabilityInput,
) -> CorpFinanceResult<ComputationOutput<RenewalProbabilityOutput>> {
    let start = Instant::now();
    let warnings: Vec<String> = Vec::new();

    if input.credit_score < dec!(1) || input.credit_score > dec!(10) {
        return Err(CorpFinanceError::InvalidInput {
            field: "credit_score".into(),
            reason: "must be between 1 and 10".into(),
        });
    }

    // Base probability driven by credit score (linear 40 %-95 %).
    let credit_factor = dec!(0.40) + (input.credit_score - dec!(1)) / dec!(9) * dec!(0.55);

    // Term factor: longer remaining term slightly raises renewal odds.
    let term_factor = if input.remaining_term_years >= dec!(5) {
        dec!(1.0)
    } else if input.remaining_term_years >= dec!(2) {
        dec!(0.90) + input.remaining_term_years / dec!(50)
    } else {
        dec!(0.80)
    };

    // Vacancy factor: high market vacancy encourages tenants to stay.
    let vacancy_factor = dec!(1) + input.market_vacancy_rate * dec!(0.5);

    // Rent spread factor: if in-place rent is below market the tenant is more
    // likely to renew; if above market, less likely.
    let spread = if input.market_rent_psf > dec!(0) {
        (input.in_place_rent_psf - input.market_rent_psf) / input.market_rent_psf
    } else {
        dec!(0)
    };
    let spread_factor = dec!(1) - spread * dec!(0.5);

    let mut prob = credit_factor * term_factor * vacancy_factor * spread_factor;
    if prob > dec!(1) {
        prob = dec!(1);
    }
    if prob < dec!(0) {
        prob = dec!(0);
    }

    let vacancy_prob = dec!(1) - prob;
    let annual_rent = input.in_place_rent_psf * input.leased_sf;
    let downtime_fraction = Decimal::from(input.downtime_months) / dec!(12);
    let vacancy_cost = annual_rent * downtime_fraction;
    let pw_vacancy_cost = vacancy_cost * vacancy_prob;

    let output = RenewalProbabilityOutput {
        tenant_name: input.tenant_name.clone(),
        renewal_probability: prob,
        vacancy_probability: vacancy_prob,
        expected_vacancy_cost: vacancy_cost,
        probability_weighted_vacancy_cost: pw_vacancy_cost,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Rent Roll — Renewal Probability Estimation",
        input,
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// 4. Mark to Market
// ---------------------------------------------------------------------------

/// Compares in-place rent/SF vs market rent/SF per tenant and aggregates
/// the mark-to-market delta weighted by remaining lease term.
/// Positive delta = below market (upside); negative = above market (risk).
pub fn mark_to_market(
    input: &MarkToMarketInput,
) -> CorpFinanceResult<ComputationOutput<MarkToMarketOutput>> {
    let start = Instant::now();
    let warnings: Vec<String> = Vec::new();

    if input.tenants.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "tenants list is empty".into(),
        ));
    }

    let mut rows = Vec::new();
    let mut agg_annual = dec!(0);
    let mut agg_tw = dec!(0);

    for t in &input.tenants {
        let delta_psf = t.market_rent_psf - t.in_place_rent_psf;
        let total_annual_delta = delta_psf * t.leased_sf;
        let term_weighted = total_annual_delta * t.remaining_lease_years;

        agg_annual += total_annual_delta;
        agg_tw += term_weighted;

        rows.push(MarkToMarketRow {
            name: t.name.clone(),
            leased_sf: t.leased_sf,
            in_place_rent_psf: t.in_place_rent_psf,
            market_rent_psf: t.market_rent_psf,
            delta_psf,
            total_annual_delta,
            remaining_lease_years: t.remaining_lease_years,
            term_weighted_delta: term_weighted,
        });
    }

    let output = MarkToMarketOutput {
        tenants: rows,
        aggregate_annual_delta: agg_annual,
        aggregate_term_weighted_delta: agg_tw,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Rent Roll — Mark to Market Analysis",
        input,
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// 5. Weighted Average Lease Term (WALT)
// ---------------------------------------------------------------------------

/// Computes WALT weighted by base rent, NRA, or the average of both.
/// Also produces a lease-term distribution histogram.
pub fn weighted_avg_lease_term(
    input: &WaltInput,
) -> CorpFinanceResult<ComputationOutput<WaltOutput>> {
    let start = Instant::now();
    let warnings: Vec<String> = Vec::new();

    if input.tenants.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "tenants list is empty".into(),
        ));
    }

    let walt = match &input.weighting {
        WaltWeighting::ByBaseRent => {
            let total_rent: Money = input.tenants.iter().map(|t| t.annual_base_rent).sum();
            if total_rent <= dec!(0) {
                return Err(CorpFinanceError::DivisionByZero {
                    context: "total annual_base_rent is zero".into(),
                });
            }
            input
                .tenants
                .iter()
                .map(|t| t.annual_base_rent * t.remaining_lease_years)
                .sum::<Decimal>()
                / total_rent
        }
        WaltWeighting::ByNra => {
            let total_sf: Decimal = input.tenants.iter().map(|t| t.leased_sf).sum();
            if total_sf <= dec!(0) {
                return Err(CorpFinanceError::DivisionByZero {
                    context: "total leased_sf is zero".into(),
                });
            }
            input
                .tenants
                .iter()
                .map(|t| t.leased_sf * t.remaining_lease_years)
                .sum::<Decimal>()
                / total_sf
        }
        WaltWeighting::ByBoth => {
            let total_rent: Money = input.tenants.iter().map(|t| t.annual_base_rent).sum();
            let total_sf: Decimal = input.tenants.iter().map(|t| t.leased_sf).sum();
            if total_rent <= dec!(0) || total_sf <= dec!(0) {
                return Err(CorpFinanceError::DivisionByZero {
                    context: "total rent or SF is zero".into(),
                });
            }
            let walt_rent = input
                .tenants
                .iter()
                .map(|t| t.annual_base_rent * t.remaining_lease_years)
                .sum::<Decimal>()
                / total_rent;
            let walt_sf = input
                .tenants
                .iter()
                .map(|t| t.leased_sf * t.remaining_lease_years)
                .sum::<Decimal>()
                / total_sf;
            (walt_rent + walt_sf) / dec!(2)
        }
    };

    // Build histogram buckets.
    let total_sf: Decimal = input.tenants.iter().map(|t| t.leased_sf).sum();
    let buckets: Vec<(&str, Decimal, Decimal)> = vec![
        ("0-1yr", dec!(0), dec!(1)),
        ("1-3yr", dec!(1), dec!(3)),
        ("3-5yr", dec!(3), dec!(5)),
        ("5-10yr", dec!(5), dec!(10)),
        ("10yr+", dec!(10), dec!(999)),
    ];

    let histogram: Vec<LeaseTermBucket> = buckets
        .iter()
        .map(|(label, lo, hi)| {
            let sf: Decimal = input
                .tenants
                .iter()
                .filter(|t| t.remaining_lease_years >= *lo && t.remaining_lease_years < *hi)
                .map(|t| t.leased_sf)
                .sum();
            let pct = if total_sf > dec!(0) {
                sf / total_sf
            } else {
                dec!(0)
            };
            LeaseTermBucket {
                label: label.to_string(),
                sf,
                pct_of_total: pct,
            }
        })
        .collect();

    let output = WaltOutput {
        walt_years: walt,
        histogram,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Rent Roll — Weighted Average Lease Term (WALT)",
        input,
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

    fn sample_tenant() -> Tenant {
        Tenant {
            name: "Acme Corp".into(),
            suite: "100".into(),
            leased_sf: dec!(10000),
            base_rent_psf: dec!(50),
            lease_start_year: 2024,
            lease_end_year: 2029,
            escalation: EscalationType::FixedStep {
                annual_increase_pct: dec!(0.03),
            },
            expense_stop_psf: Some(dec!(15)),
            ti_allowance_psf: Some(dec!(10)),
            ti_amortization_years: Some(5),
            free_rent_months: Some(3),
            renewal_probability: Some(dec!(0.75)),
            downtime_months: Some(6),
            credit_score: Some(dec!(7)),
            projected_annual_sales: None,
        }
    }

    fn flat_tenant(name: &str, sf: Decimal, rent_psf: Decimal, end_yr: u32) -> Tenant {
        Tenant {
            name: name.into(),
            suite: "200".into(),
            leased_sf: sf,
            base_rent_psf: rent_psf,
            lease_start_year: 2024,
            lease_end_year: end_yr,
            escalation: EscalationType::FlatRent,
            expense_stop_psf: None,
            ti_allowance_psf: None,
            ti_amortization_years: None,
            free_rent_months: None,
            renewal_probability: Some(dec!(0.80)),
            downtime_months: Some(3),
            credit_score: Some(dec!(6)),
            projected_annual_sales: None,
        }
    }

    // -- tenant_schedule ----------------------------------------------------

    #[test]
    fn ts_basic_five_year() {
        let input = TenantScheduleInput {
            tenant: sample_tenant(),
            analysis_start_year: 2024,
            holding_period_years: 5,
            current_opex_psf: Some(dec!(18)),
            opex_growth_rate: Some(dec!(0.02)),
        };
        let r = tenant_schedule(&input).unwrap();
        assert_eq!(r.result.schedule.len(), 5);
        assert_eq!(r.result.tenant_name, "Acme Corp");
        assert!(r.result.total_gross_rent > dec!(0));
    }

    #[test]
    fn ts_flat_rent_no_escalation() {
        let t = flat_tenant("Flat Co", dec!(5000), dec!(30), 2030);
        let input = TenantScheduleInput {
            tenant: t,
            analysis_start_year: 2024,
            holding_period_years: 3,
            current_opex_psf: None,
            opex_growth_rate: None,
        };
        let r = tenant_schedule(&input).unwrap();
        for row in &r.result.schedule {
            assert_eq!(row.base_rent_psf, dec!(30));
        }
    }

    #[test]
    fn ts_cpi_linked_escalation() {
        let mut t = sample_tenant();
        t.escalation = EscalationType::CpiLinked {
            spread_over_cpi: dec!(0.01),
            assumed_cpi: dec!(0.025),
        };
        t.free_rent_months = None;
        t.ti_allowance_psf = None;
        let input = TenantScheduleInput {
            tenant: t,
            analysis_start_year: 2024,
            holding_period_years: 3,
            current_opex_psf: None,
            opex_growth_rate: None,
        };
        let r = tenant_schedule(&input).unwrap();
        let s = &r.result.schedule;
        assert!(s[1].base_rent_psf > s[0].base_rent_psf);
        let expected = dec!(50) + dec!(50) * dec!(0.035);
        assert_eq!(s[1].base_rent_psf, expected);
    }

    #[test]
    fn ts_percentage_rent_overage() {
        let t = Tenant {
            name: "Retail Tenant".into(),
            suite: "G01".into(),
            leased_sf: dec!(8000),
            base_rent_psf: dec!(40),
            lease_start_year: 2024,
            lease_end_year: 2029,
            escalation: EscalationType::PercentageRent {
                base_rent_psf: dec!(40),
                pct_of_sales: dec!(0.06),
                breakpoint_sales: dec!(2000000),
            },
            expense_stop_psf: None,
            ti_allowance_psf: None,
            ti_amortization_years: None,
            free_rent_months: None,
            renewal_probability: Some(dec!(0.80)),
            downtime_months: Some(3),
            credit_score: Some(dec!(5)),
            projected_annual_sales: Some(dec!(3000000)),
        };
        let input = TenantScheduleInput {
            tenant: t,
            analysis_start_year: 2024,
            holding_period_years: 2,
            current_opex_psf: None,
            opex_growth_rate: None,
        };
        let r = tenant_schedule(&input).unwrap();
        // Overage = (3M - 2M) * 6% = 60k; base = 40*8000 = 320k => 380k
        assert_eq!(r.result.schedule[0].gross_rent, dec!(380000));
    }

    #[test]
    fn ts_free_rent_deduction() {
        let mut t = flat_tenant("Free Rent Co", dec!(10000), dec!(50), 2030);
        t.free_rent_months = Some(6);
        let input = TenantScheduleInput {
            tenant: t,
            analysis_start_year: 2024,
            holding_period_years: 2,
            current_opex_psf: None,
            opex_growth_rate: None,
        };
        let r = tenant_schedule(&input).unwrap();
        assert_eq!(r.result.schedule[0].free_rent_adjustment, dec!(250000));
    }

    #[test]
    fn ts_expense_recovery() {
        let mut t = flat_tenant("Expense Co", dec!(10000), dec!(40), 2030);
        t.expense_stop_psf = Some(dec!(12));
        let input = TenantScheduleInput {
            tenant: t,
            analysis_start_year: 2024,
            holding_period_years: 2,
            current_opex_psf: Some(dec!(15)),
            opex_growth_rate: Some(dec!(0.02)),
        };
        let r = tenant_schedule(&input).unwrap();
        assert_eq!(r.result.schedule[0].expense_recovery, dec!(30000));
    }

    #[test]
    fn ts_ti_amortization() {
        let mut t = flat_tenant("TI Co", dec!(5000), dec!(40), 2030);
        t.ti_allowance_psf = Some(dec!(20));
        t.ti_amortization_years = Some(3);
        let input = TenantScheduleInput {
            tenant: t,
            analysis_start_year: 2024,
            holding_period_years: 4,
            current_opex_psf: None,
            opex_growth_rate: None,
        };
        let r = tenant_schedule(&input).unwrap();
        let expected_ti = dec!(20) * dec!(5000) / dec!(3);
        assert_eq!(r.result.schedule[0].ti_amortization, expected_ti);
        assert_eq!(r.result.schedule[2].ti_amortization, expected_ti);
        assert_eq!(r.result.schedule[3].ti_amortization, dec!(0));
    }

    #[test]
    fn ts_zero_sf_error() {
        let mut t = sample_tenant();
        t.leased_sf = dec!(0);
        let input = TenantScheduleInput {
            tenant: t,
            analysis_start_year: 2024,
            holding_period_years: 3,
            current_opex_psf: None,
            opex_growth_rate: None,
        };
        assert!(tenant_schedule(&input).is_err());
    }

    #[test]
    fn ts_invalid_dates() {
        let mut t = sample_tenant();
        t.lease_end_year = 2020;
        t.lease_start_year = 2024;
        let input = TenantScheduleInput {
            tenant: t,
            analysis_start_year: 2024,
            holding_period_years: 3,
            current_opex_psf: None,
            opex_growth_rate: None,
        };
        assert!(tenant_schedule(&input).is_err());
    }

    #[test]
    fn ts_net_equals_gross_minus_free_plus_recovery_minus_ti() {
        let mut t = flat_tenant("Check Co", dec!(10000), dec!(50), 2030);
        t.expense_stop_psf = Some(dec!(10));
        t.ti_allowance_psf = Some(dec!(6));
        t.ti_amortization_years = Some(2);
        t.free_rent_months = Some(6);
        let input = TenantScheduleInput {
            tenant: t,
            analysis_start_year: 2024,
            holding_period_years: 1,
            current_opex_psf: Some(dec!(14)),
            opex_growth_rate: None,
        };
        let r = tenant_schedule(&input).unwrap();
        let row = &r.result.schedule[0];
        let expected_net =
            row.gross_rent - row.free_rent_adjustment + row.expense_recovery - row.ti_amortization;
        assert_eq!(row.net_effective_rent, expected_net);
    }

    // -- lease_rollover -----------------------------------------------------

    #[test]
    fn lr_basic_profile() {
        let tenants = vec![
            flat_tenant("A", dec!(5000), dec!(50), 2025),
            flat_tenant("B", dec!(3000), dec!(40), 2026),
            flat_tenant("C", dec!(2000), dec!(60), 2027),
        ];
        let input = LeaseRolloverInput {
            tenants,
            holding_period_years: 5,
            analysis_start_year: 2024,
            total_building_sf: dec!(15000),
        };
        let r = lease_rollover(&input).unwrap();
        assert_eq!(r.result.annual_rollover.len(), 5);
        assert_eq!(r.result.total_leased_sf, dec!(10000));
    }

    #[test]
    fn lr_expiry_concentration() {
        let tenants = vec![
            flat_tenant("A", dec!(5000), dec!(50), 2025),
            flat_tenant("B", dec!(5000), dec!(40), 2025),
        ];
        let input = LeaseRolloverInput {
            tenants,
            holding_period_years: 3,
            analysis_start_year: 2024,
            total_building_sf: dec!(20000),
        };
        let r = lease_rollover(&input).unwrap();
        let yr = r
            .result
            .annual_rollover
            .iter()
            .find(|r| r.year == 2025)
            .unwrap();
        assert_eq!(yr.expiring_sf, dec!(10000));
    }

    #[test]
    fn lr_cumulative_reaches_100pct() {
        let tenants = vec![
            flat_tenant("A", dec!(5000), dec!(50), 2025),
            flat_tenant("B", dec!(5000), dec!(40), 2026),
        ];
        let input = LeaseRolloverInput {
            tenants,
            holding_period_years: 3,
            analysis_start_year: 2024,
            total_building_sf: dec!(10000),
        };
        let r = lease_rollover(&input).unwrap();
        let last = r.result.annual_rollover.last().unwrap();
        assert_eq!(last.cumulative_pct_nra, dec!(1));
    }

    #[test]
    fn lr_contract_001_sf_exceeds_building() {
        let tenants = vec![
            flat_tenant("A", dec!(8000), dec!(50), 2026),
            flat_tenant("B", dec!(8000), dec!(40), 2027),
        ];
        let input = LeaseRolloverInput {
            tenants,
            holding_period_years: 5,
            analysis_start_year: 2024,
            total_building_sf: dec!(10000),
        };
        assert!(lease_rollover(&input).is_err());
    }

    #[test]
    fn lr_empty_tenants() {
        let input = LeaseRolloverInput {
            tenants: vec![],
            holding_period_years: 5,
            analysis_start_year: 2024,
            total_building_sf: dec!(10000),
        };
        assert!(lease_rollover(&input).is_err());
    }

    #[test]
    fn lr_no_expiry_in_period() {
        let tenants = vec![flat_tenant("A", dec!(5000), dec!(50), 2035)];
        let input = LeaseRolloverInput {
            tenants,
            holding_period_years: 3,
            analysis_start_year: 2024,
            total_building_sf: dec!(10000),
        };
        let r = lease_rollover(&input).unwrap();
        for row in &r.result.annual_rollover {
            assert_eq!(row.expiring_sf, dec!(0));
        }
    }

    // -- renewal_probability ------------------------------------------------

    #[test]
    fn rp_high_credit_high_prob() {
        let input = RenewalProbabilityInput {
            tenant_name: "HighCredit".into(),
            remaining_term_years: dec!(3),
            credit_score: dec!(9),
            market_vacancy_rate: dec!(0.05),
            in_place_rent_psf: dec!(45),
            market_rent_psf: dec!(50),
            leased_sf: dec!(10000),
            downtime_months: 6,
        };
        let r = renewal_probability(&input).unwrap();
        assert!(r.result.renewal_probability > dec!(0.80));
    }

    #[test]
    fn rp_low_credit_low_prob() {
        let input = RenewalProbabilityInput {
            tenant_name: "LowCredit".into(),
            remaining_term_years: dec!(1),
            credit_score: dec!(2),
            market_vacancy_rate: dec!(0.03),
            in_place_rent_psf: dec!(55),
            market_rent_psf: dec!(50),
            leased_sf: dec!(5000),
            downtime_months: 9,
        };
        let r = renewal_probability(&input).unwrap();
        assert!(r.result.renewal_probability < dec!(0.60));
    }

    #[test]
    fn rp_below_market_boosts_renewal() {
        let base = RenewalProbabilityInput {
            tenant_name: "BelowMkt".into(),
            remaining_term_years: dec!(3),
            credit_score: dec!(6),
            market_vacancy_rate: dec!(0.05),
            in_place_rent_psf: dec!(40),
            market_rent_psf: dec!(50),
            leased_sf: dec!(10000),
            downtime_months: 6,
        };
        let above = RenewalProbabilityInput {
            in_place_rent_psf: dec!(60),
            ..base.clone()
        };
        let r_below = renewal_probability(&base).unwrap();
        let r_above = renewal_probability(&above).unwrap();
        assert!(r_below.result.renewal_probability > r_above.result.renewal_probability);
    }

    #[test]
    fn rp_vacancy_cost_calculation() {
        let input = RenewalProbabilityInput {
            tenant_name: "VacancyCost".into(),
            remaining_term_years: dec!(2),
            credit_score: dec!(5),
            market_vacancy_rate: dec!(0.08),
            in_place_rent_psf: dec!(50),
            market_rent_psf: dec!(50),
            leased_sf: dec!(10000),
            downtime_months: 6,
        };
        let r = renewal_probability(&input).unwrap();
        let full_cost = dec!(50) * dec!(10000) * dec!(6) / dec!(12);
        assert_eq!(r.result.expected_vacancy_cost, full_cost);
        assert!(r.result.probability_weighted_vacancy_cost <= full_cost);
    }

    #[test]
    fn rp_invalid_credit_score() {
        let input = RenewalProbabilityInput {
            tenant_name: "Bad".into(),
            remaining_term_years: dec!(2),
            credit_score: dec!(0),
            market_vacancy_rate: dec!(0.05),
            in_place_rent_psf: dec!(50),
            market_rent_psf: dec!(50),
            leased_sf: dec!(10000),
            downtime_months: 6,
        };
        assert!(renewal_probability(&input).is_err());
    }

    #[test]
    fn rp_clamped_to_one() {
        let input = RenewalProbabilityInput {
            tenant_name: "MaxProb".into(),
            remaining_term_years: dec!(10),
            credit_score: dec!(10),
            market_vacancy_rate: dec!(0.20),
            in_place_rent_psf: dec!(30),
            market_rent_psf: dec!(60),
            leased_sf: dec!(5000),
            downtime_months: 3,
        };
        let r = renewal_probability(&input).unwrap();
        assert!(r.result.renewal_probability <= dec!(1));
    }

    #[test]
    fn rp_credit_score_11_rejected() {
        let input = RenewalProbabilityInput {
            tenant_name: "Over".into(),
            remaining_term_years: dec!(3),
            credit_score: dec!(11),
            market_vacancy_rate: dec!(0.05),
            in_place_rent_psf: dec!(50),
            market_rent_psf: dec!(50),
            leased_sf: dec!(5000),
            downtime_months: 6,
        };
        assert!(renewal_probability(&input).is_err());
    }

    // -- mark_to_market -----------------------------------------------------

    #[test]
    fn mtm_below_market_upside() {
        let input = MarkToMarketInput {
            tenants: vec![MarkToMarketTenant {
                name: "Below".into(),
                leased_sf: dec!(10000),
                in_place_rent_psf: dec!(40),
                market_rent_psf: dec!(50),
                remaining_lease_years: dec!(3),
            }],
        };
        let r = mark_to_market(&input).unwrap();
        assert_eq!(r.result.aggregate_annual_delta, dec!(100000));
        assert_eq!(r.result.aggregate_term_weighted_delta, dec!(300000));
    }

    #[test]
    fn mtm_above_market_risk() {
        let input = MarkToMarketInput {
            tenants: vec![MarkToMarketTenant {
                name: "Above".into(),
                leased_sf: dec!(5000),
                in_place_rent_psf: dec!(60),
                market_rent_psf: dec!(50),
                remaining_lease_years: dec!(2),
            }],
        };
        let r = mark_to_market(&input).unwrap();
        assert_eq!(r.result.aggregate_annual_delta, dec!(-50000));
    }

    #[test]
    fn mtm_multiple_tenants_net() {
        let input = MarkToMarketInput {
            tenants: vec![
                MarkToMarketTenant {
                    name: "A".into(),
                    leased_sf: dec!(10000),
                    in_place_rent_psf: dec!(40),
                    market_rent_psf: dec!(50),
                    remaining_lease_years: dec!(5),
                },
                MarkToMarketTenant {
                    name: "B".into(),
                    leased_sf: dec!(5000),
                    in_place_rent_psf: dec!(55),
                    market_rent_psf: dec!(50),
                    remaining_lease_years: dec!(3),
                },
            ],
        };
        let r = mark_to_market(&input).unwrap();
        assert_eq!(r.result.aggregate_annual_delta, dec!(75000));
    }

    #[test]
    fn mtm_at_market_zero() {
        let input = MarkToMarketInput {
            tenants: vec![MarkToMarketTenant {
                name: "AtMkt".into(),
                leased_sf: dec!(8000),
                in_place_rent_psf: dec!(50),
                market_rent_psf: dec!(50),
                remaining_lease_years: dec!(4),
            }],
        };
        let r = mark_to_market(&input).unwrap();
        assert_eq!(r.result.aggregate_annual_delta, dec!(0));
    }

    #[test]
    fn mtm_empty_tenants() {
        let input = MarkToMarketInput { tenants: vec![] };
        assert!(mark_to_market(&input).is_err());
    }

    // -- weighted_avg_lease_term --------------------------------------------

    #[test]
    fn walt_by_rent() {
        let input = WaltInput {
            tenants: vec![
                WaltTenant {
                    name: "A".into(),
                    leased_sf: dec!(10000),
                    annual_base_rent: dec!(500000),
                    remaining_lease_years: dec!(5),
                },
                WaltTenant {
                    name: "B".into(),
                    leased_sf: dec!(5000),
                    annual_base_rent: dec!(200000),
                    remaining_lease_years: dec!(2),
                },
            ],
            weighting: WaltWeighting::ByBaseRent,
        };
        let r = weighted_avg_lease_term(&input).unwrap();
        let expected = (dec!(500000) * dec!(5) + dec!(200000) * dec!(2)) / dec!(700000);
        assert_eq!(r.result.walt_years, expected);
    }

    #[test]
    fn walt_by_nra() {
        let input = WaltInput {
            tenants: vec![
                WaltTenant {
                    name: "A".into(),
                    leased_sf: dec!(10000),
                    annual_base_rent: dec!(500000),
                    remaining_lease_years: dec!(5),
                },
                WaltTenant {
                    name: "B".into(),
                    leased_sf: dec!(5000),
                    annual_base_rent: dec!(200000),
                    remaining_lease_years: dec!(2),
                },
            ],
            weighting: WaltWeighting::ByNra,
        };
        let r = weighted_avg_lease_term(&input).unwrap();
        let expected = (dec!(10000) * dec!(5) + dec!(5000) * dec!(2)) / dec!(15000);
        assert_eq!(r.result.walt_years, expected);
    }

    #[test]
    fn walt_by_both_average() {
        let input = WaltInput {
            tenants: vec![
                WaltTenant {
                    name: "A".into(),
                    leased_sf: dec!(10000),
                    annual_base_rent: dec!(500000),
                    remaining_lease_years: dec!(5),
                },
                WaltTenant {
                    name: "B".into(),
                    leased_sf: dec!(5000),
                    annual_base_rent: dec!(200000),
                    remaining_lease_years: dec!(2),
                },
            ],
            weighting: WaltWeighting::ByBoth,
        };
        let r = weighted_avg_lease_term(&input).unwrap();
        let w_rent = (dec!(500000) * dec!(5) + dec!(200000) * dec!(2)) / dec!(700000);
        let w_sf = (dec!(10000) * dec!(5) + dec!(5000) * dec!(2)) / dec!(15000);
        assert_eq!(r.result.walt_years, (w_rent + w_sf) / dec!(2));
    }

    #[test]
    fn walt_histogram_buckets() {
        let input = WaltInput {
            tenants: vec![
                WaltTenant {
                    name: "Short".into(),
                    leased_sf: dec!(2000),
                    annual_base_rent: dec!(100000),
                    remaining_lease_years: dec!(0.5),
                },
                WaltTenant {
                    name: "Mid".into(),
                    leased_sf: dec!(3000),
                    annual_base_rent: dec!(150000),
                    remaining_lease_years: dec!(4),
                },
                WaltTenant {
                    name: "Long".into(),
                    leased_sf: dec!(5000),
                    annual_base_rent: dec!(300000),
                    remaining_lease_years: dec!(12),
                },
            ],
            weighting: WaltWeighting::ByNra,
        };
        let r = weighted_avg_lease_term(&input).unwrap();
        let h = &r.result.histogram;
        assert_eq!(h.len(), 5);
        assert_eq!(h[0].sf, dec!(2000));
        assert_eq!(h[2].sf, dec!(3000));
        assert_eq!(h[4].sf, dec!(5000));
    }

    #[test]
    fn walt_empty_tenants() {
        let input = WaltInput {
            tenants: vec![],
            weighting: WaltWeighting::ByBaseRent,
        };
        assert!(weighted_avg_lease_term(&input).is_err());
    }

    #[test]
    fn walt_zero_rent_error() {
        let input = WaltInput {
            tenants: vec![WaltTenant {
                name: "Zero".into(),
                leased_sf: dec!(1000),
                annual_base_rent: dec!(0),
                remaining_lease_years: dec!(3),
            }],
            weighting: WaltWeighting::ByBaseRent,
        };
        assert!(weighted_avg_lease_term(&input).is_err());
    }

    #[test]
    fn walt_single_tenant_equals_term() {
        let input = WaltInput {
            tenants: vec![WaltTenant {
                name: "Solo".into(),
                leased_sf: dec!(10000),
                annual_base_rent: dec!(500000),
                remaining_lease_years: dec!(7),
            }],
            weighting: WaltWeighting::ByBaseRent,
        };
        let r = weighted_avg_lease_term(&input).unwrap();
        assert_eq!(r.result.walt_years, dec!(7));
    }
}
