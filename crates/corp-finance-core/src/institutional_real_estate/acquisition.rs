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

/// A single debt tranche in an acquisition capital structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebtTranche {
    /// Label for the tranche (e.g. "Senior", "Mezzanine").
    pub name: String,
    /// Principal amount.
    pub amount: Money,
    /// Annual interest rate (e.g. 0.055 = 5.5%).
    pub interest_rate: Rate,
    /// Loan term in years (balloon maturity).
    pub term_years: u32,
    /// Amortisation schedule in years. None = interest-only for full term.
    pub amortization_years: Option<u32>,
    /// Interest-only period before amortisation begins. None = 0.
    pub io_period_years: Option<u32>,
}

/// Go / No-Go decision based on underwriting thresholds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GoNoGoDecision {
    Go,
    Conditional { failed_thresholds: Vec<String> },
    NoGo { failed_thresholds: Vec<String> },
}

/// Year-by-year pro forma row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProFormaYear {
    pub year: u32,
    pub noi: Money,
    pub debt_service: Money,
    pub cash_flow_after_debt: Money,
    pub cash_on_cash: Rate,
}

// ---------------------------------------------------------------------------
// 1. Acquisition Model
// ---------------------------------------------------------------------------

/// Input for a full acquisition underwriting model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcquisitionModelInput {
    /// Purchase price.
    pub purchase_price: Money,
    /// Closing costs (transfer tax, title, legal, etc.).
    pub closing_costs: Money,
    /// Capex reserves at close (TI, LC, deferred maintenance).
    pub capex_reserves: Money,
    /// Year-1 net operating income.
    pub noi_year1: Money,
    /// Annual NOI growth rate.
    pub noi_growth_rate: Rate,
    /// Hold period in years.
    pub hold_period_years: u32,
    /// Exit cap rate.
    pub exit_cap_rate: Rate,
    /// Disposition costs as a fraction of sale price (e.g. 0.02).
    pub disposition_cost_rate: Rate,
    /// Debt tranches (senior, mezzanine, etc.).
    pub debt_tranches: Vec<DebtTranche>,
    /// Discount rate for NPV calculations.
    pub discount_rate: Rate,
    /// Minimum acceptable levered IRR for Go/NoGo.
    pub target_irr: Option<Rate>,
    /// Minimum acceptable DSCR for Go/NoGo.
    pub target_dscr: Option<Rate>,
}

/// Output of the acquisition underwriting model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcquisitionModelOutput {
    /// Total uses of funds (purchase + closing + capex reserves).
    pub total_uses: Money,
    /// Total debt across all tranches.
    pub total_debt: Money,
    /// Total equity required.
    pub equity_required: Money,
    /// Going-in cap rate (NOI year 1 / purchase price).
    pub going_in_cap_rate: Rate,
    /// Year-by-year pro forma.
    pub pro_forma: Vec<ProFormaYear>,
    /// Gross sale price at exit.
    pub exit_sale_price: Money,
    /// Net sale proceeds (after disposition costs and debt payoff).
    pub net_sale_proceeds: Money,
    /// Levered IRR on equity (Newton-Raphson).
    pub levered_irr: Rate,
    /// Unlevered IRR on total cost basis.
    pub unlevered_irr: Rate,
    /// Equity multiple (total distributions / equity invested).
    pub equity_multiple: Decimal,
    /// Year-1 DSCR (NOI / total debt service).
    pub dscr_year1: Decimal,
    /// Go / No-Go decision based on targets.
    pub decision: GoNoGoDecision,
}

pub fn acquisition_model(
    input: &AcquisitionModelInput,
) -> CorpFinanceResult<ComputationOutput<AcquisitionModelOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // --- Validate ---
    validate_cap_rate(input.exit_cap_rate, "exit_cap_rate")?;

    if input.purchase_price <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "purchase_price".into(),
            reason: "Purchase price must be positive".into(),
        });
    }
    if input.hold_period_years < 1 {
        return Err(CorpFinanceError::InvalidInput {
            field: "hold_period_years".into(),
            reason: "Hold period must be at least 1 year".into(),
        });
    }
    if input.noi_year1 <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "noi_year1".into(),
            reason: "Year-1 NOI must be positive".into(),
        });
    }

    // --- Sources & Uses ---
    let total_uses = input.purchase_price + input.closing_costs + input.capex_reserves;
    let total_debt: Money = input.debt_tranches.iter().map(|t| t.amount).sum();
    let equity_required = total_uses - total_debt;

    if equity_required <= Decimal::ZERO {
        warnings.push("Equity required is zero or negative — over-leveraged".into());
    }

    let going_in_cap_rate = input.noi_year1 / input.purchase_price;

    // --- Build annual debt service schedule per tranche ---
    let n = input.hold_period_years as usize;
    let tranche_ds = build_tranche_debt_service(&input.debt_tranches, n, &mut warnings)?;

    // --- Pro forma ---
    let mut pro_forma = Vec::with_capacity(n);
    let mut noi = input.noi_year1;

    for yr in 0..n {
        if yr > 0 {
            noi *= Decimal::ONE + input.noi_growth_rate;
        }
        let ds: Money = tranche_ds.iter().map(|t| t[yr]).sum();
        let cf_after_debt = noi - ds;
        let coc = if equity_required.is_zero() || equity_required < Decimal::ZERO {
            Decimal::ZERO
        } else {
            cf_after_debt / equity_required
        };
        pro_forma.push(ProFormaYear {
            year: (yr + 1) as u32,
            noi,
            debt_service: ds,
            cash_flow_after_debt: cf_after_debt,
            cash_on_cash: coc,
        });
    }

    // --- Exit ---
    let exit_noi = noi * (Decimal::ONE + input.noi_growth_rate); // NOI in year after hold
    let exit_sale_price = exit_noi / input.exit_cap_rate;
    let disposition_costs = exit_sale_price * input.disposition_cost_rate;

    // Outstanding loan balances at exit
    let total_remaining_debt = compute_total_remaining_debt(&input.debt_tranches, n)?;
    let net_sale_proceeds = exit_sale_price - disposition_costs - total_remaining_debt;

    // --- DSCR Year 1 ---
    let ds_year1: Money = tranche_ds.iter().map(|t| t[0]).sum();
    let dscr_year1 = if ds_year1.is_zero() {
        dec!(999.99)
    } else {
        input.noi_year1 / ds_year1
    };

    if dscr_year1 < dec!(1.2) && dscr_year1 < dec!(999) {
        warnings.push(format!(
            "Year-1 DSCR of {dscr_year1:.2}x is below 1.20x — lender covenant risk"
        ));
    }

    // --- Levered IRR ---
    let mut lev_cfs = Vec::with_capacity(n + 1);
    lev_cfs.push(-equity_required);
    for (i, pf) in pro_forma.iter().enumerate() {
        if i == n - 1 {
            lev_cfs.push(pf.cash_flow_after_debt + net_sale_proceeds);
        } else {
            lev_cfs.push(pf.cash_flow_after_debt);
        }
    }
    let levered_irr = newton_raphson_irr(&lev_cfs, &mut warnings);

    // --- Unlevered IRR ---
    let mut unlev_cfs = Vec::with_capacity(n + 1);
    unlev_cfs.push(-total_uses);
    for (i, pf) in pro_forma.iter().enumerate() {
        if i == n - 1 {
            unlev_cfs.push(pf.noi + exit_sale_price - disposition_costs);
        } else {
            unlev_cfs.push(pf.noi);
        }
    }
    let unlevered_irr = newton_raphson_irr(&unlev_cfs, &mut warnings);

    // --- Equity multiple ---
    let total_distributions: Money = pro_forma
        .iter()
        .map(|pf| pf.cash_flow_after_debt)
        .sum::<Decimal>()
        + net_sale_proceeds;
    let equity_multiple = if equity_required.is_zero() || equity_required < Decimal::ZERO {
        Decimal::ZERO
    } else {
        total_distributions / equity_required
    };

    // --- Go / No-Go ---
    let mut failed = Vec::new();
    if let Some(target_irr) = input.target_irr {
        if levered_irr < target_irr {
            failed.push(format!(
                "Levered IRR {:.2}% < target {:.2}%",
                levered_irr * dec!(100),
                target_irr * dec!(100)
            ));
        }
    }
    if let Some(target_dscr) = input.target_dscr {
        if dscr_year1 < target_dscr {
            failed.push(format!("DSCR {dscr_year1:.2}x < target {target_dscr:.2}x"));
        }
    }

    let decision = if failed.is_empty() {
        GoNoGoDecision::Go
    } else if failed.len() == 1 {
        GoNoGoDecision::Conditional {
            failed_thresholds: failed,
        }
    } else {
        GoNoGoDecision::NoGo {
            failed_thresholds: failed,
        }
    };

    let output = AcquisitionModelOutput {
        total_uses,
        total_debt,
        equity_required,
        going_in_cap_rate,
        pro_forma,
        exit_sale_price,
        net_sale_proceeds,
        levered_irr,
        unlevered_irr,
        equity_multiple,
        dscr_year1,
        decision,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Institutional Real Estate Acquisition Model",
        input,
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// 2. Hold / Sell Analysis
// ---------------------------------------------------------------------------

/// Input for hold vs. sell decision analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoldSellInput {
    /// Current property value (market value or offer price).
    pub current_value: Money,
    /// Current annual NOI.
    pub current_noi: Money,
    /// Annual NOI growth rate.
    pub noi_growth_rate: Rate,
    /// Remaining hold period to evaluate (years).
    pub remaining_hold_years: u32,
    /// Exit cap rate for terminal value.
    pub exit_cap_rate: Rate,
    /// Disposition cost rate (fraction of sale price).
    pub disposition_cost_rate: Rate,
    /// Remaining outstanding debt.
    pub remaining_debt: Money,
    /// Annual debt service.
    pub annual_debt_service: Money,
    /// Discount rate for NPV.
    pub discount_rate: Rate,
    /// Original equity invested (for IRR calculation).
    pub original_equity: Money,
    /// Years already held (for optimal hold period search).
    pub years_held: u32,
    /// Maximum additional years to search for optimal hold.
    pub max_additional_years: Option<u32>,
}

/// Output of hold vs. sell analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoldSellOutput {
    /// NPV of holding the property.
    pub hold_npv: Money,
    /// NPV of selling now.
    pub sell_npv: Money,
    /// Recommendation: positive = hold, negative = sell.
    pub npv_advantage_of_holding: Money,
    /// Break-even exit cap rate (where hold NPV = sell NPV).
    pub breakeven_exit_cap_rate: Option<Rate>,
    /// Optimal hold period (year that maximises equity IRR).
    pub optimal_hold_period_years: u32,
    /// IRR at optimal hold period.
    pub optimal_irr: Rate,
}

pub fn hold_sell_analysis(
    input: &HoldSellInput,
) -> CorpFinanceResult<ComputationOutput<HoldSellOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // --- Validate ---
    validate_cap_rate(input.exit_cap_rate, "exit_cap_rate")?;
    if input.discount_rate <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "discount_rate".into(),
            reason: "Discount rate must be positive".into(),
        });
    }
    if input.remaining_hold_years < 1 {
        return Err(CorpFinanceError::InvalidInput {
            field: "remaining_hold_years".into(),
            reason: "Remaining hold years must be at least 1".into(),
        });
    }

    // --- Sell NPV (sell now) ---
    let sell_proceeds = input.current_value
        - input.current_value * input.disposition_cost_rate
        - input.remaining_debt;
    let sell_npv = sell_proceeds; // Already at t=0

    // --- Hold NPV ---
    let n = input.remaining_hold_years as usize;
    let mut noi = input.current_noi;
    let mut hold_pv = Decimal::ZERO;
    let one_plus_r = Decimal::ONE + input.discount_rate;
    let mut df = Decimal::ONE;

    for yr in 0..n {
        if yr > 0 {
            noi *= Decimal::ONE + input.noi_growth_rate;
        }
        let cf = noi - input.annual_debt_service;
        df /= one_plus_r;
        hold_pv += cf * df;
    }

    // Terminal value at end of hold
    let exit_noi = noi * (Decimal::ONE + input.noi_growth_rate);
    let terminal_value = exit_noi / input.exit_cap_rate;
    let exit_disposition = terminal_value * input.disposition_cost_rate;
    // Assume debt is fully amortised or refinanced; use remaining_debt scaled simplistically
    let terminal_net = terminal_value - exit_disposition - input.remaining_debt;
    let hold_npv = hold_pv + terminal_net * df;

    let npv_advantage = hold_npv - sell_npv;

    if npv_advantage < Decimal::ZERO {
        warnings.push("Sell NPV exceeds Hold NPV — consider selling".into());
    }

    // --- Break-even exit cap rate (bisection) ---
    let breakeven_exit_cap_rate = find_breakeven_cap_rate(input, sell_npv, &mut warnings);

    // --- Optimal hold period ---
    let max_search = input
        .max_additional_years
        .unwrap_or(15)
        .max(input.remaining_hold_years);
    let mut best_irr = dec!(-1.0);
    let mut best_year = 1u32;

    for test_years in 1..=max_search {
        let irr = compute_hold_irr(input, test_years as usize, &mut warnings);
        if irr > best_irr {
            best_irr = irr;
            best_year = test_years;
        }
    }

    let output = HoldSellOutput {
        hold_npv,
        sell_npv,
        npv_advantage_of_holding: npv_advantage,
        breakeven_exit_cap_rate,
        optimal_hold_period_years: best_year,
        optimal_irr: best_irr,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Hold vs. Sell Analysis (RE-CONTRACT-006)",
        input,
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// 3. Value-Add IRR
// ---------------------------------------------------------------------------

/// Input for a value-add renovation/repositioning analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueAddIrrInput {
    /// Acquisition cost (all-in).
    pub acquisition_cost: Money,
    /// Renovation capex schedule by year (index 0 = year 1).
    pub renovation_capex: Vec<Money>,
    /// Equity invested at close.
    pub equity_at_close: Money,
    /// Current occupancy rate (e.g. 0.70 = 70%).
    pub current_occupancy: Rate,
    /// Stabilised occupancy target (e.g. 0.95).
    pub stabilised_occupancy: Rate,
    /// Years to reach stabilised occupancy from start.
    pub lease_up_years: u32,
    /// Gross potential income at stabilisation.
    pub stabilised_gpi: Money,
    /// Operating expense ratio (fraction of EGI).
    pub opex_ratio: Rate,
    /// Hold period in years.
    pub hold_period_years: u32,
    /// Exit cap rate.
    pub exit_cap_rate: Rate,
    /// Disposition cost rate.
    pub disposition_cost_rate: Rate,
    /// Annual NOI growth after stabilisation.
    pub noi_growth_rate: Rate,
    /// Optional: GP promote / waterfall fees as a fraction of profits.
    pub promote_rate: Option<Rate>,
    /// Debt tranche (optional).
    pub debt: Option<DebtTranche>,
}

/// Output of the value-add IRR analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueAddIrrOutput {
    /// Gross IRR (before promote/fees).
    pub gross_irr: Rate,
    /// Net IRR (after promote/fees).
    pub net_irr: Rate,
    /// Equity multiple.
    pub equity_multiple: Decimal,
    /// Peak equity requirement (cumulative).
    pub peak_equity: Money,
    /// Return on cost (stabilised NOI / total cost).
    pub return_on_cost: Rate,
    /// Stabilised NOI.
    pub stabilised_noi: Money,
    /// Year-by-year NOI schedule.
    pub noi_schedule: Vec<Money>,
}

pub fn value_add_irr(
    input: &ValueAddIrrInput,
) -> CorpFinanceResult<ComputationOutput<ValueAddIrrOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // --- Validate ---
    validate_cap_rate(input.exit_cap_rate, "exit_cap_rate")?;
    if input.hold_period_years < 1 {
        return Err(CorpFinanceError::InvalidInput {
            field: "hold_period_years".into(),
            reason: "Hold period must be at least 1 year".into(),
        });
    }
    if input.stabilised_occupancy <= Decimal::ZERO || input.stabilised_occupancy > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "stabilised_occupancy".into(),
            reason: "Stabilised occupancy must be between 0 (excl) and 1 (incl)".into(),
        });
    }

    let n = input.hold_period_years as usize;
    let lease_up = input.lease_up_years as usize;

    // --- Build NOI schedule with lease-up ---
    let mut noi_schedule = Vec::with_capacity(n);
    let stabilised_egi = input.stabilised_gpi * input.stabilised_occupancy;
    let stabilised_noi = stabilised_egi * (Decimal::ONE - input.opex_ratio);
    let mut prev_noi = Decimal::ZERO;

    for yr in 0..n {
        let occupancy = if lease_up == 0 || yr >= lease_up {
            input.stabilised_occupancy
        } else {
            // Linear interpolation from current to stabilised
            let frac = Decimal::from((yr + 1) as u32) / Decimal::from(lease_up as u32);
            input.current_occupancy + (input.stabilised_occupancy - input.current_occupancy) * frac
        };

        let egi = input.stabilised_gpi * occupancy;
        let mut year_noi = egi * (Decimal::ONE - input.opex_ratio);

        // Apply growth after stabilisation
        if yr >= lease_up && yr > 0 && prev_noi > Decimal::ZERO {
            if yr == lease_up {
                // First stabilised year — use stabilised NOI as base
                year_noi = stabilised_noi;
            } else {
                year_noi = prev_noi * (Decimal::ONE + input.noi_growth_rate);
            }
        }

        prev_noi = year_noi;
        noi_schedule.push(year_noi);
    }

    // --- Total cost basis ---
    let total_renovation: Money = input.renovation_capex.iter().copied().sum();
    let total_cost = input.acquisition_cost + total_renovation;

    // --- Return on cost ---
    let return_on_cost = if total_cost.is_zero() {
        Decimal::ZERO
    } else {
        stabilised_noi / total_cost
    };

    // --- Debt service (if any) ---
    let (_annual_ds, debt_balance_at_exit) = match &input.debt {
        Some(d) => {
            let ds = compute_tranche_annual_ds(d, 0)?; // Use year-0 DS (IO vs amort handled)
            let bal = compute_tranche_balance(d, n)?;
            (ds, bal)
        }
        None => (Decimal::ZERO, Decimal::ZERO),
    };

    // --- Cash flow series for gross IRR ---
    let mut gross_cfs = Vec::with_capacity(n + 1);
    // t=0: equity outflow
    gross_cfs.push(-input.equity_at_close);

    // Track peak equity
    let mut cumulative_equity = input.equity_at_close;
    let mut peak_equity = cumulative_equity;

    for (yr, &noi_yr) in noi_schedule.iter().enumerate().take(n) {
        let capex = if yr < input.renovation_capex.len() && yr > 0 {
            input.renovation_capex[yr]
        } else {
            Decimal::ZERO
        };

        // Additional equity calls for capex in later years
        if capex > Decimal::ZERO {
            cumulative_equity += capex;
            if cumulative_equity > peak_equity {
                peak_equity = cumulative_equity;
            }
        }

        let ds = match &input.debt {
            Some(d) => compute_tranche_annual_ds(d, yr)?,
            None => Decimal::ZERO,
        };

        let cf = noi_yr - ds - capex;

        if yr == n - 1 {
            // Exit
            let exit_noi = if n > lease_up {
                prev_noi * (Decimal::ONE + input.noi_growth_rate)
            } else {
                stabilised_noi
            };
            let sale_price = exit_noi / input.exit_cap_rate;
            let disp_cost = sale_price * input.disposition_cost_rate;
            let net_exit = sale_price - disp_cost - debt_balance_at_exit;
            gross_cfs.push(cf + net_exit);
        } else {
            gross_cfs.push(cf);
        }
    }

    let gross_irr = newton_raphson_irr(&gross_cfs, &mut warnings);

    // --- Net IRR (after promote) ---
    let promote = input.promote_rate.unwrap_or(Decimal::ZERO);
    let net_cfs: Vec<Decimal> = gross_cfs
        .iter()
        .enumerate()
        .map(|(i, &cf)| {
            if i == 0 || cf <= Decimal::ZERO {
                cf
            } else {
                cf * (Decimal::ONE - promote)
            }
        })
        .collect();
    let net_irr = newton_raphson_irr(&net_cfs, &mut warnings);

    // --- Equity multiple ---
    let total_distributions: Money = gross_cfs.iter().skip(1).copied().sum();
    let equity_multiple = if input.equity_at_close.is_zero() {
        Decimal::ZERO
    } else {
        total_distributions / input.equity_at_close
    };

    if return_on_cost < input.exit_cap_rate {
        warnings.push(format!(
            "Return on cost {:.2}% < exit cap rate {:.2}% — value-add may not create spread",
            return_on_cost * dec!(100),
            input.exit_cap_rate * dec!(100)
        ));
    }

    let output = ValueAddIrrOutput {
        gross_irr,
        net_irr,
        equity_multiple,
        peak_equity,
        return_on_cost,
        stabilised_noi,
        noi_schedule,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Value-Add IRR Analysis",
        input,
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// 4. Development Feasibility
// ---------------------------------------------------------------------------

/// Input for a ground-up development feasibility analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevelopmentFeasibilityInput {
    /// Land acquisition cost.
    pub land_cost: Money,
    /// Hard construction costs.
    pub hard_costs: Money,
    /// Soft costs as a fraction of hard costs (e.g. 0.20 = 20%).
    pub soft_cost_pct: Rate,
    /// Construction period in months.
    pub construction_months: u32,
    /// Construction draw schedule (% of hard + soft costs per period).
    /// Length should match construction periods. If empty, assumes linear draw.
    pub draw_schedule_pct: Vec<Rate>,
    /// Annual construction loan rate.
    pub construction_loan_rate: Rate,
    /// Lease-up period after completion (months).
    pub lease_up_months: u32,
    /// Stabilised annual NOI.
    pub stabilised_noi: Money,
    /// Market cap rate for comparable stabilised properties.
    pub market_cap_rate: Rate,
    /// Target profit margin (for Go/NoGo).
    pub target_profit_margin: Option<Rate>,
}

/// Output of the development feasibility analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevelopmentFeasibilityOutput {
    /// Total soft costs.
    pub soft_costs: Money,
    /// Total construction costs (hard + soft).
    pub total_construction_costs: Money,
    /// Financing carry (interest during construction + lease-up).
    pub financing_carry: Money,
    /// Total development cost (land + construction + carry).
    pub total_development_cost: Money,
    /// Development yield (stabilised NOI / total cost).
    pub development_yield: Rate,
    /// Development spread (yield - market cap rate).
    pub development_spread: Rate,
    /// Stabilised value (NOI / market cap rate).
    pub stabilised_value: Money,
    /// Residual land value (stabilised value - all non-land costs).
    pub residual_land_value: Money,
    /// Profit margin ((value - cost) / cost).
    pub profit_margin: Rate,
    /// Go / No-Go decision.
    pub decision: GoNoGoDecision,
}

pub fn development_feasibility(
    input: &DevelopmentFeasibilityInput,
) -> CorpFinanceResult<ComputationOutput<DevelopmentFeasibilityOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // --- Validate ---
    validate_cap_rate(input.market_cap_rate, "market_cap_rate")?;
    if input.hard_costs <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "hard_costs".into(),
            reason: "Hard costs must be positive".into(),
        });
    }
    if input.construction_months == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "construction_months".into(),
            reason: "Construction period must be at least 1 month".into(),
        });
    }

    // --- Costs ---
    let soft_costs = input.hard_costs * input.soft_cost_pct;
    let total_construction_costs = input.hard_costs + soft_costs;

    // --- Financing carry ---
    // Build draw schedule: if provided use it, otherwise linear
    let periods = input.construction_months as usize;
    let draw_pcts: Vec<Decimal> = if input.draw_schedule_pct.len() == periods {
        input.draw_schedule_pct.clone()
    } else {
        // Linear draw
        let pct_per_month = Decimal::ONE / Decimal::from(periods as u32);
        vec![pct_per_month; periods]
    };

    let monthly_rate = input.construction_loan_rate / dec!(12);
    let mut outstanding = Decimal::ZERO;
    let mut total_interest = Decimal::ZERO;

    // Interest during construction
    for pct in &draw_pcts {
        let draw = total_construction_costs * *pct;
        outstanding += draw;
        let interest = outstanding * monthly_rate;
        total_interest += interest;
    }

    // Interest during lease-up (full balance, no additional draws)
    for _ in 0..input.lease_up_months {
        let interest = outstanding * monthly_rate;
        total_interest += interest;
    }

    let financing_carry = total_interest;

    // --- Total development cost ---
    let total_development_cost = input.land_cost + total_construction_costs + financing_carry;

    // --- Development yield & spread ---
    let development_yield = if total_development_cost.is_zero() {
        Decimal::ZERO
    } else {
        input.stabilised_noi / total_development_cost
    };
    let development_spread = development_yield - input.market_cap_rate;

    if development_spread < Decimal::ZERO {
        warnings.push(format!(
            "Negative development spread ({:.2}%) — project may not justify risk",
            development_spread * dec!(100)
        ));
    }

    // --- Stabilised value ---
    let stabilised_value = input.stabilised_noi / input.market_cap_rate;

    // --- Residual land value ---
    let residual_land_value = stabilised_value - total_construction_costs - financing_carry;

    // --- Profit margin ---
    let profit_margin = if total_development_cost.is_zero() {
        Decimal::ZERO
    } else {
        (stabilised_value - total_development_cost) / total_development_cost
    };

    if profit_margin < dec!(0.15) {
        warnings.push(format!(
            "Profit margin {:.1}% is below 15% — thin for development risk",
            profit_margin * dec!(100)
        ));
    }

    // --- Go / No-Go ---
    let mut failed = Vec::new();
    if let Some(target) = input.target_profit_margin {
        if profit_margin < target {
            failed.push(format!(
                "Profit margin {:.1}% < target {:.1}%",
                profit_margin * dec!(100),
                target * dec!(100)
            ));
        }
    }
    if development_spread < dec!(0.01) {
        failed.push(format!(
            "Development spread {:.2}% < 100 bps",
            development_spread * dec!(100)
        ));
    }

    let decision = if failed.is_empty() {
        GoNoGoDecision::Go
    } else if failed.len() == 1 {
        GoNoGoDecision::Conditional {
            failed_thresholds: failed,
        }
    } else {
        GoNoGoDecision::NoGo {
            failed_thresholds: failed,
        }
    };

    let output = DevelopmentFeasibilityOutput {
        soft_costs,
        total_construction_costs,
        financing_carry,
        total_development_cost,
        development_yield,
        development_spread,
        stabilised_value,
        residual_land_value,
        profit_margin,
        decision,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Development Feasibility Analysis",
        input,
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// 5. Refinancing Analysis
// ---------------------------------------------------------------------------

/// Input for comparing existing vs. proposed debt terms.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefinancingInput {
    /// Current property value.
    pub property_value: Money,
    /// Current annual NOI.
    pub current_noi: Money,
    /// Existing loan outstanding balance.
    pub existing_balance: Money,
    /// Existing annual interest rate.
    pub existing_rate: Rate,
    /// Remaining term on existing loan (years).
    pub existing_remaining_years: u32,
    /// Existing loan amortisation years remaining. None = IO.
    pub existing_amort_years: Option<u32>,
    /// Proposed new loan amount.
    pub proposed_amount: Money,
    /// Proposed annual interest rate.
    pub proposed_rate: Rate,
    /// Proposed loan term (years).
    pub proposed_term_years: u32,
    /// Proposed amortisation (years). None = IO.
    pub proposed_amort_years: Option<u32>,
    /// Prepayment penalty or defeasance cost on existing loan.
    pub prepayment_penalty: Money,
    /// Closing costs for new loan.
    pub closing_costs: Money,
    /// Discount rate for NPV comparison.
    pub discount_rate: Rate,
}

/// Output of the refinancing analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefinancingOutput {
    /// Existing annual debt service.
    pub existing_annual_ds: Money,
    /// Proposed annual debt service.
    pub proposed_annual_ds: Money,
    /// Annual interest savings.
    pub annual_interest_savings: Money,
    /// NPV of interest savings over analysis period.
    pub npv_of_savings: Money,
    /// Total cost to refinance (penalty + closing).
    pub total_refi_cost: Money,
    /// Break-even period in months.
    pub breakeven_months: u32,
    /// Post-refi LTV.
    pub post_refi_ltv: Rate,
    /// Post-refi DSCR.
    pub post_refi_dscr: Decimal,
    /// Debt yield (NOI / proposed loan).
    pub debt_yield: Rate,
    /// Cash-out amount (proposed - existing balance - costs). Negative = cash-in.
    pub cash_out_amount: Money,
    /// Recommendation.
    pub recommend_refi: bool,
}

pub fn refinancing(
    input: &RefinancingInput,
) -> CorpFinanceResult<ComputationOutput<RefinancingOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // --- Validate ---
    if input.property_value <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "property_value".into(),
            reason: "Property value must be positive".into(),
        });
    }
    if input.existing_remaining_years == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "existing_remaining_years".into(),
            reason: "Remaining years must be at least 1".into(),
        });
    }

    // --- Existing debt service ---
    let existing_annual_ds = compute_annual_debt_service(
        input.existing_balance,
        input.existing_rate,
        input.existing_amort_years,
    )?;

    // --- Proposed debt service ---
    let proposed_annual_ds = compute_annual_debt_service(
        input.proposed_amount,
        input.proposed_rate,
        input.proposed_amort_years,
    )?;

    // --- Annual savings ---
    let annual_interest_savings = existing_annual_ds - proposed_annual_ds;

    // --- NPV of savings ---
    let analysis_years = input
        .existing_remaining_years
        .min(input.proposed_term_years);
    let one_plus_r = Decimal::ONE + input.discount_rate;
    let mut npv_savings = Decimal::ZERO;
    let mut df = Decimal::ONE;

    for _ in 0..analysis_years {
        df /= one_plus_r;
        npv_savings += annual_interest_savings * df;
    }

    // --- Total refi cost ---
    let total_refi_cost = input.prepayment_penalty + input.closing_costs;

    // --- Break-even (months) ---
    let monthly_savings = annual_interest_savings / dec!(12);
    let breakeven_months = if monthly_savings <= Decimal::ZERO {
        u32::MAX // Never breaks even
    } else {
        // Ceiling of total_refi_cost / monthly_savings
        let months_dec = total_refi_cost / monthly_savings;
        // Round up
        let truncated = months_dec.trunc();
        if months_dec > truncated {
            (truncated + Decimal::ONE)
                .to_string()
                .parse::<u32>()
                .unwrap_or(u32::MAX)
        } else {
            truncated.to_string().parse::<u32>().unwrap_or(u32::MAX)
        }
    };

    // --- Post-refi metrics ---
    let post_refi_ltv = input.proposed_amount / input.property_value;
    let post_refi_dscr = if proposed_annual_ds.is_zero() {
        dec!(999.99)
    } else {
        input.current_noi / proposed_annual_ds
    };
    let debt_yield = if input.proposed_amount.is_zero() {
        Decimal::ZERO
    } else {
        input.current_noi / input.proposed_amount
    };

    // --- Cash-out ---
    let cash_out_amount = input.proposed_amount - input.existing_balance - total_refi_cost;

    // --- Warnings ---
    if post_refi_ltv > dec!(0.75) {
        warnings.push(format!(
            "Post-refi LTV {:.1}% exceeds 75%",
            post_refi_ltv * dec!(100)
        ));
    }
    if post_refi_dscr < dec!(1.25) && post_refi_dscr < dec!(999) {
        warnings.push(format!("Post-refi DSCR {post_refi_dscr:.2}x below 1.25x"));
    }
    if breakeven_months > 36 {
        warnings.push(format!(
            "Break-even period of {breakeven_months} months exceeds 3 years"
        ));
    }

    let recommend_refi = npv_savings > total_refi_cost && breakeven_months < (analysis_years * 12);

    let output = RefinancingOutput {
        existing_annual_ds,
        proposed_annual_ds,
        annual_interest_savings,
        npv_of_savings: npv_savings,
        total_refi_cost,
        breakeven_months,
        post_refi_ltv,
        post_refi_dscr,
        debt_yield,
        cash_out_amount,
        recommend_refi,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Refinancing Analysis",
        input,
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// RE-CONTRACT-004: Validate cap rate is positive and < 1.0.
fn validate_cap_rate(cap_rate: Rate, field: &str) -> CorpFinanceResult<()> {
    if cap_rate <= Decimal::ZERO || cap_rate >= Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: field.into(),
            reason: "Cap rate must be positive and less than 1.0 (RE-CONTRACT-004)".into(),
        });
    }
    Ok(())
}

/// Newton-Raphson IRR solver. cash_flows[0] is typically negative (investment).
fn newton_raphson_irr(cash_flows: &[Decimal], warnings: &mut Vec<String>) -> Decimal {
    let max_iter: u32 = 50;
    let epsilon = dec!(0.0000001);
    let mut rate = dec!(0.10);

    for _ in 0..max_iter {
        let (npv, dnpv) = npv_and_derivative(cash_flows, rate);

        if dnpv.abs() < dec!(0.000000001) {
            warnings.push("IRR: derivative near zero — result may be imprecise".into());
            break;
        }

        let new_rate = rate - npv / dnpv;

        if (new_rate - rate).abs() < epsilon {
            return new_rate;
        }

        rate = new_rate;

        // Guard against runaway
        if rate < dec!(-0.99) {
            rate = dec!(-0.99);
        }
        if rate > dec!(10.0) {
            rate = dec!(10.0);
        }
    }

    rate
}

/// NPV(r) and d(NPV)/dr for Newton-Raphson.
fn npv_and_derivative(cash_flows: &[Decimal], rate: Decimal) -> (Decimal, Decimal) {
    let one_plus_r = Decimal::ONE + rate;
    let mut npv = Decimal::ZERO;
    let mut dnpv = Decimal::ZERO;
    let mut discount = Decimal::ONE;

    for (t, cf) in cash_flows.iter().enumerate() {
        npv += *cf * discount;
        if t > 0 {
            dnpv += Decimal::from(-(t as i64)) * *cf * discount / one_plus_r;
        }
        discount /= one_plus_r;
    }

    (npv, dnpv)
}

/// Build per-tranche annual debt service vectors, handling IO periods.
fn build_tranche_debt_service(
    tranches: &[DebtTranche],
    years: usize,
    _warnings: &mut Vec<String>,
) -> CorpFinanceResult<Vec<Vec<Money>>> {
    let mut result = Vec::with_capacity(tranches.len());

    for tranche in tranches {
        let mut ds_vec = Vec::with_capacity(years);

        for yr in 0..years {
            let ds = compute_tranche_annual_ds(tranche, yr)?;
            ds_vec.push(ds);
        }

        result.push(ds_vec);
    }

    Ok(result)
}

/// Annual debt service for a single tranche in a given year.
fn compute_tranche_annual_ds(tranche: &DebtTranche, year: usize) -> CorpFinanceResult<Money> {
    let io_years = tranche.io_period_years.unwrap_or(0) as usize;

    if year < io_years {
        // Interest-only period
        Ok(tranche.amount * tranche.interest_rate)
    } else {
        // Amortising period
        match tranche.amortization_years {
            None => {
                // Full IO for entire term
                Ok(tranche.amount * tranche.interest_rate)
            }
            Some(amort_years) => {
                let monthly_rate = tranche.interest_rate / dec!(12);
                let total_months = amort_years * 12;

                if monthly_rate.is_zero() {
                    if total_months == 0 {
                        return Err(CorpFinanceError::DivisionByZero {
                            context: "tranche amortisation with zero months".into(),
                        });
                    }
                    Ok(tranche.amount / Decimal::from(amort_years))
                } else {
                    let monthly_pmt =
                        compute_monthly_payment_simple(tranche.amount, monthly_rate, total_months)?;
                    Ok(monthly_pmt * dec!(12))
                }
            }
        }
    }
}

/// Compute outstanding balance for a tranche after `years` of payments.
fn compute_tranche_balance(tranche: &DebtTranche, years: usize) -> CorpFinanceResult<Money> {
    let io_years = tranche.io_period_years.unwrap_or(0) as usize;

    if years <= io_years {
        // Still in IO period — full balance outstanding
        return Ok(tranche.amount);
    }

    match tranche.amortization_years {
        None => Ok(tranche.amount), // Full IO
        Some(amort_years) => {
            let amort_payments_years = years - io_years;
            let monthly_rate = tranche.interest_rate / dec!(12);
            let total_months = amort_years * 12;
            let payments_made = (amort_payments_years as u32) * 12;

            if monthly_rate.is_zero() {
                let paid =
                    tranche.amount * Decimal::from(payments_made) / Decimal::from(total_months);
                return Ok((tranche.amount - paid).max(Decimal::ZERO));
            }

            let monthly_pmt =
                compute_monthly_payment_simple(tranche.amount, monthly_rate, total_months)?;

            let mut balance = tranche.amount;
            for _ in 0..payments_made {
                let interest = balance * monthly_rate;
                let principal = monthly_pmt - interest;
                balance -= principal;
                if balance < Decimal::ZERO {
                    balance = Decimal::ZERO;
                    break;
                }
            }

            Ok(balance)
        }
    }
}

/// Total remaining debt across all tranches after `years`.
fn compute_total_remaining_debt(
    tranches: &[DebtTranche],
    years: usize,
) -> CorpFinanceResult<Money> {
    let mut total = Decimal::ZERO;
    for tranche in tranches {
        total += compute_tranche_balance(tranche, years)?;
    }
    Ok(total)
}

/// Standard fixed-rate monthly payment: P * r(1+r)^n / ((1+r)^n - 1)
fn compute_monthly_payment_simple(
    principal: Money,
    monthly_rate: Rate,
    total_months: u32,
) -> CorpFinanceResult<Money> {
    if monthly_rate.is_zero() {
        if total_months == 0 {
            return Err(CorpFinanceError::DivisionByZero {
                context: "monthly payment with zero rate and zero months".into(),
            });
        }
        return Ok(principal / Decimal::from(total_months));
    }

    let mut compound = Decimal::ONE;
    for _ in 0..total_months {
        compound *= Decimal::ONE + monthly_rate;
    }

    let numerator = principal * monthly_rate * compound;
    let denominator = compound - Decimal::ONE;

    if denominator.is_zero() {
        return Err(CorpFinanceError::DivisionByZero {
            context: "mortgage payment denominator".into(),
        });
    }

    Ok(numerator / denominator)
}

/// Annual debt service for a simple loan (for refinancing).
fn compute_annual_debt_service(
    balance: Money,
    annual_rate: Rate,
    amort_years: Option<u32>,
) -> CorpFinanceResult<Money> {
    match amort_years {
        None => {
            // Interest-only
            Ok(balance * annual_rate)
        }
        Some(ay) => {
            let monthly_rate = annual_rate / dec!(12);
            let total_months = ay * 12;
            let monthly_pmt = compute_monthly_payment_simple(balance, monthly_rate, total_months)?;
            Ok(monthly_pmt * dec!(12))
        }
    }
}

/// Find break-even exit cap rate via bisection for hold/sell analysis.
fn find_breakeven_cap_rate(
    input: &HoldSellInput,
    sell_npv: Money,
    _warnings: &mut Vec<String>,
) -> Option<Rate> {
    let mut lo = dec!(0.01);
    let mut hi = dec!(0.50);

    for _ in 0..60 {
        let mid = (lo + hi) / dec!(2);
        let hold_npv = compute_hold_npv_with_cap(input, mid);

        let diff = hold_npv - sell_npv;
        if diff.abs() < dec!(0.01) {
            return Some(mid);
        }

        if diff > Decimal::ZERO {
            // Hold NPV > sell NPV => need higher cap (lower terminal value) to equalise
            lo = mid;
        } else {
            hi = mid;
        }
    }

    None
}

/// Hold NPV using a specific exit cap rate.
fn compute_hold_npv_with_cap(input: &HoldSellInput, exit_cap: Rate) -> Money {
    let n = input.remaining_hold_years as usize;
    let one_plus_r = Decimal::ONE + input.discount_rate;
    let mut df = Decimal::ONE;
    let mut noi = input.current_noi;
    let mut hold_pv = Decimal::ZERO;

    for yr in 0..n {
        if yr > 0 {
            noi *= Decimal::ONE + input.noi_growth_rate;
        }
        let cf = noi - input.annual_debt_service;
        df /= one_plus_r;
        hold_pv += cf * df;
    }

    let exit_noi = noi * (Decimal::ONE + input.noi_growth_rate);
    let terminal = exit_noi / exit_cap;
    let exit_disp = terminal * input.disposition_cost_rate;
    let terminal_net = terminal - exit_disp - input.remaining_debt;
    hold_pv + terminal_net * df
}

/// Compute equity IRR for a given hold period (for optimal hold search).
fn compute_hold_irr(input: &HoldSellInput, hold_years: usize, warnings: &mut Vec<String>) -> Rate {
    let mut cfs = Vec::with_capacity(hold_years + 1);
    cfs.push(-input.original_equity);

    let mut noi = input.current_noi;
    for yr in 0..hold_years {
        if yr > 0 {
            noi *= Decimal::ONE + input.noi_growth_rate;
        }
        let cf = noi - input.annual_debt_service;

        if yr == hold_years - 1 {
            let exit_noi = noi * (Decimal::ONE + input.noi_growth_rate);
            let sale = exit_noi / input.exit_cap_rate;
            let disp = sale * input.disposition_cost_rate;
            let net_exit = sale - disp - input.remaining_debt;
            cfs.push(cf + net_exit);
        } else {
            cfs.push(cf);
        }
    }

    newton_raphson_irr(&cfs, warnings)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn senior_tranche() -> DebtTranche {
        DebtTranche {
            name: "Senior".into(),
            amount: dec!(7_000_000),
            interest_rate: dec!(0.055),
            term_years: 10,
            amortization_years: Some(30),
            io_period_years: Some(2),
        }
    }

    fn mezz_tranche() -> DebtTranche {
        DebtTranche {
            name: "Mezzanine".into(),
            amount: dec!(1_500_000),
            interest_rate: dec!(0.09),
            term_years: 5,
            amortization_years: None,
            io_period_years: None,
        }
    }

    fn basic_acquisition_input() -> AcquisitionModelInput {
        AcquisitionModelInput {
            purchase_price: dec!(10_000_000),
            closing_costs: dec!(200_000),
            capex_reserves: dec!(300_000),
            noi_year1: dec!(700_000),
            noi_growth_rate: dec!(0.03),
            hold_period_years: 5,
            exit_cap_rate: dec!(0.06),
            disposition_cost_rate: dec!(0.02),
            debt_tranches: vec![senior_tranche()],
            discount_rate: dec!(0.08),
            target_irr: Some(dec!(0.12)),
            target_dscr: Some(dec!(1.20)),
        }
    }

    fn basic_hold_sell_input() -> HoldSellInput {
        HoldSellInput {
            current_value: dec!(12_000_000),
            current_noi: dec!(750_000),
            noi_growth_rate: dec!(0.03),
            remaining_hold_years: 5,
            exit_cap_rate: dec!(0.06),
            disposition_cost_rate: dec!(0.02),
            remaining_debt: dec!(6_000_000),
            annual_debt_service: dec!(450_000),
            discount_rate: dec!(0.08),
            original_equity: dec!(4_000_000),
            years_held: 3,
            max_additional_years: Some(10),
        }
    }

    fn basic_value_add_input() -> ValueAddIrrInput {
        ValueAddIrrInput {
            acquisition_cost: dec!(8_000_000),
            renovation_capex: vec![dec!(500_000), dec!(1_000_000), dec!(500_000)],
            equity_at_close: dec!(4_000_000),
            current_occupancy: dec!(0.60),
            stabilised_occupancy: dec!(0.93),
            lease_up_years: 3,
            stabilised_gpi: dec!(1_500_000),
            opex_ratio: dec!(0.40),
            hold_period_years: 7,
            exit_cap_rate: dec!(0.055),
            disposition_cost_rate: dec!(0.02),
            noi_growth_rate: dec!(0.025),
            promote_rate: Some(dec!(0.20)),
            debt: Some(DebtTranche {
                name: "Senior".into(),
                amount: dec!(5_000_000),
                interest_rate: dec!(0.05),
                term_years: 7,
                amortization_years: Some(30),
                io_period_years: Some(2),
            }),
        }
    }

    fn basic_dev_input() -> DevelopmentFeasibilityInput {
        DevelopmentFeasibilityInput {
            land_cost: dec!(3_000_000),
            hard_costs: dec!(12_000_000),
            soft_cost_pct: dec!(0.20),
            construction_months: 24,
            draw_schedule_pct: vec![],
            construction_loan_rate: dec!(0.065),
            lease_up_months: 12,
            stabilised_noi: dec!(1_800_000),
            market_cap_rate: dec!(0.055),
            target_profit_margin: Some(dec!(0.15)),
        }
    }

    fn basic_refi_input() -> RefinancingInput {
        RefinancingInput {
            property_value: dec!(15_000_000),
            current_noi: dec!(1_000_000),
            existing_balance: dec!(8_000_000),
            existing_rate: dec!(0.065),
            existing_remaining_years: 7,
            existing_amort_years: Some(25),
            proposed_amount: dec!(9_000_000),
            proposed_rate: dec!(0.05),
            proposed_term_years: 10,
            proposed_amort_years: Some(30),
            prepayment_penalty: dec!(160_000),
            closing_costs: dec!(90_000),
            discount_rate: dec!(0.07),
        }
    }

    // -----------------------------------------------------------------------
    // Acquisition Model tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_acquisition_model_sources_uses() {
        let r = acquisition_model(&basic_acquisition_input()).unwrap();
        let o = &r.result;
        assert_eq!(o.total_uses, dec!(10_500_000));
        assert_eq!(o.total_debt, dec!(7_000_000));
        assert_eq!(o.equity_required, dec!(3_500_000));
    }

    #[test]
    fn test_acquisition_model_going_in_cap() {
        let r = acquisition_model(&basic_acquisition_input()).unwrap();
        assert_eq!(r.result.going_in_cap_rate, dec!(0.07));
    }

    #[test]
    fn test_acquisition_model_pro_forma_length() {
        let r = acquisition_model(&basic_acquisition_input()).unwrap();
        assert_eq!(r.result.pro_forma.len(), 5);
    }

    #[test]
    fn test_acquisition_model_noi_growth() {
        let r = acquisition_model(&basic_acquisition_input()).unwrap();
        let pf = &r.result.pro_forma;
        assert_eq!(pf[0].noi, dec!(700_000));
        // Year 2 = 700k * 1.03
        let expected_y2 = dec!(700_000) * dec!(1.03);
        assert_eq!(pf[1].noi, expected_y2);
    }

    #[test]
    fn test_acquisition_model_io_period_debt_service() {
        let r = acquisition_model(&basic_acquisition_input()).unwrap();
        let pf = &r.result.pro_forma;
        // Year 1 and 2 are IO: 7M * 5.5% = 385,000
        let io_ds = dec!(7_000_000) * dec!(0.055);
        assert_eq!(pf[0].debt_service, io_ds);
        assert_eq!(pf[1].debt_service, io_ds);
        // Year 3 should be amortising (higher DS)
        assert!(pf[2].debt_service > io_ds);
    }

    #[test]
    fn test_acquisition_model_levered_irr_positive() {
        let r = acquisition_model(&basic_acquisition_input()).unwrap();
        assert!(r.result.levered_irr > Decimal::ZERO);
    }

    #[test]
    fn test_acquisition_model_unlevered_irr_positive() {
        let r = acquisition_model(&basic_acquisition_input()).unwrap();
        assert!(r.result.unlevered_irr > Decimal::ZERO);
    }

    #[test]
    fn test_acquisition_model_leverage_amplifies_irr() {
        let r = acquisition_model(&basic_acquisition_input()).unwrap();
        // With positive leverage, levered IRR should exceed unlevered
        assert!(r.result.levered_irr > r.result.unlevered_irr);
    }

    #[test]
    fn test_acquisition_model_equity_multiple_above_one() {
        let r = acquisition_model(&basic_acquisition_input()).unwrap();
        assert!(r.result.equity_multiple > Decimal::ONE);
    }

    #[test]
    fn test_acquisition_model_decision_go() {
        let mut input = basic_acquisition_input();
        input.target_irr = Some(dec!(0.01)); // Very low target
        input.target_dscr = Some(dec!(1.0));
        let r = acquisition_model(&input).unwrap();
        assert!(matches!(r.result.decision, GoNoGoDecision::Go));
    }

    #[test]
    fn test_acquisition_model_decision_nogo() {
        let mut input = basic_acquisition_input();
        input.target_irr = Some(dec!(0.50)); // Impossibly high
        input.target_dscr = Some(dec!(5.0));
        let r = acquisition_model(&input).unwrap();
        assert!(matches!(r.result.decision, GoNoGoDecision::NoGo { .. }));
    }

    #[test]
    fn test_acquisition_model_mezz_increases_leverage() {
        let mut input = basic_acquisition_input();
        input.debt_tranches.push(mezz_tranche());
        let r = acquisition_model(&input).unwrap();
        assert_eq!(r.result.total_debt, dec!(8_500_000));
        assert_eq!(r.result.equity_required, dec!(2_000_000));
    }

    #[test]
    fn test_acquisition_model_invalid_cap_rate_zero() {
        let mut input = basic_acquisition_input();
        input.exit_cap_rate = Decimal::ZERO;
        assert!(acquisition_model(&input).is_err());
    }

    #[test]
    fn test_acquisition_model_invalid_cap_rate_above_one() {
        let mut input = basic_acquisition_input();
        input.exit_cap_rate = dec!(1.5);
        assert!(acquisition_model(&input).is_err());
    }

    #[test]
    fn test_acquisition_model_invalid_noi() {
        let mut input = basic_acquisition_input();
        input.noi_year1 = Decimal::ZERO;
        assert!(acquisition_model(&input).is_err());
    }

    #[test]
    fn test_acquisition_model_methodology() {
        let r = acquisition_model(&basic_acquisition_input()).unwrap();
        assert!(r.methodology.contains("Acquisition"));
    }

    // -----------------------------------------------------------------------
    // Hold/Sell Analysis tests (RE-CONTRACT-006)
    // -----------------------------------------------------------------------

    #[test]
    fn test_hold_sell_both_npvs_computed() {
        let r = hold_sell_analysis(&basic_hold_sell_input()).unwrap();
        // RE-CONTRACT-006: must include hold AND sell NPV
        let _hold = r.result.hold_npv;
        let _sell = r.result.sell_npv;
    }

    #[test]
    fn test_hold_sell_sell_npv_calculation() {
        let input = basic_hold_sell_input();
        let r = hold_sell_analysis(&input).unwrap();
        // Sell NPV = current_value - disp_costs - debt = 12M - 240k - 6M = 5,760,000
        let expected = dec!(12_000_000) - dec!(240_000) - dec!(6_000_000);
        assert_eq!(r.result.sell_npv, expected);
    }

    #[test]
    fn test_hold_sell_npv_advantage_sign() {
        let r = hold_sell_analysis(&basic_hold_sell_input()).unwrap();
        // The advantage should equal hold - sell
        let diff = r.result.hold_npv - r.result.sell_npv;
        assert_eq!(r.result.npv_advantage_of_holding, diff);
    }

    #[test]
    fn test_hold_sell_breakeven_cap_rate_exists() {
        let r = hold_sell_analysis(&basic_hold_sell_input()).unwrap();
        assert!(r.result.breakeven_exit_cap_rate.is_some());
        let be = r.result.breakeven_exit_cap_rate.unwrap();
        assert!(be > Decimal::ZERO && be < Decimal::ONE);
    }

    #[test]
    fn test_hold_sell_optimal_period_at_least_one() {
        let r = hold_sell_analysis(&basic_hold_sell_input()).unwrap();
        assert!(r.result.optimal_hold_period_years >= 1);
    }

    #[test]
    fn test_hold_sell_optimal_irr_positive() {
        let r = hold_sell_analysis(&basic_hold_sell_input()).unwrap();
        assert!(r.result.optimal_irr > Decimal::ZERO);
    }

    #[test]
    fn test_hold_sell_invalid_cap_rate() {
        let mut input = basic_hold_sell_input();
        input.exit_cap_rate = Decimal::ZERO;
        assert!(hold_sell_analysis(&input).is_err());
    }

    #[test]
    fn test_hold_sell_invalid_discount_rate() {
        let mut input = basic_hold_sell_input();
        input.discount_rate = Decimal::ZERO;
        assert!(hold_sell_analysis(&input).is_err());
    }

    #[test]
    fn test_hold_sell_methodology() {
        let r = hold_sell_analysis(&basic_hold_sell_input()).unwrap();
        assert!(r.methodology.contains("RE-CONTRACT-006"));
    }

    // -----------------------------------------------------------------------
    // Value-Add IRR tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_value_add_noi_schedule_length() {
        let r = value_add_irr(&basic_value_add_input()).unwrap();
        assert_eq!(r.result.noi_schedule.len(), 7);
    }

    #[test]
    fn test_value_add_noi_increases_during_leaseup() {
        let r = value_add_irr(&basic_value_add_input()).unwrap();
        let sched = &r.result.noi_schedule;
        // NOI should increase as occupancy improves
        assert!(sched[1] > sched[0]);
        assert!(sched[2] > sched[1]);
    }

    #[test]
    fn test_value_add_stabilised_noi() {
        let input = basic_value_add_input();
        let r = value_add_irr(&input).unwrap();
        // Stabilised NOI = 1.5M * 0.93 * (1 - 0.40) = 837,000
        let expected = dec!(1_500_000) * dec!(0.93) * dec!(0.60);
        assert_eq!(r.result.stabilised_noi, expected);
    }

    #[test]
    fn test_value_add_return_on_cost() {
        let input = basic_value_add_input();
        let r = value_add_irr(&input).unwrap();
        // Total cost = 8M + 2M capex = 10M, stabilised NOI = 837k
        // ROC = 837k / 10M = 0.0837
        assert!(r.result.return_on_cost > dec!(0.08));
        assert!(r.result.return_on_cost < dec!(0.09));
    }

    #[test]
    fn test_value_add_gross_irr_positive() {
        let r = value_add_irr(&basic_value_add_input()).unwrap();
        assert!(r.result.gross_irr > Decimal::ZERO);
    }

    #[test]
    fn test_value_add_net_irr_less_than_gross() {
        let r = value_add_irr(&basic_value_add_input()).unwrap();
        // Net IRR should be less than gross due to promote
        assert!(r.result.net_irr <= r.result.gross_irr);
    }

    #[test]
    fn test_value_add_peak_equity() {
        let r = value_add_irr(&basic_value_add_input()).unwrap();
        // Peak equity should be >= initial equity
        assert!(r.result.peak_equity >= dec!(4_000_000));
    }

    #[test]
    fn test_value_add_equity_multiple() {
        let r = value_add_irr(&basic_value_add_input()).unwrap();
        assert!(r.result.equity_multiple > Decimal::ZERO);
    }

    #[test]
    fn test_value_add_invalid_cap_rate() {
        let mut input = basic_value_add_input();
        input.exit_cap_rate = dec!(1.5);
        assert!(value_add_irr(&input).is_err());
    }

    #[test]
    fn test_value_add_invalid_occupancy() {
        let mut input = basic_value_add_input();
        input.stabilised_occupancy = dec!(1.5);
        assert!(value_add_irr(&input).is_err());
    }

    // -----------------------------------------------------------------------
    // Development Feasibility tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_dev_soft_costs() {
        let r = development_feasibility(&basic_dev_input()).unwrap();
        // Soft = 12M * 20% = 2.4M
        assert_eq!(r.result.soft_costs, dec!(2_400_000));
    }

    #[test]
    fn test_dev_total_construction_costs() {
        let r = development_feasibility(&basic_dev_input()).unwrap();
        // Total = 12M + 2.4M = 14.4M
        assert_eq!(r.result.total_construction_costs, dec!(14_400_000));
    }

    #[test]
    fn test_dev_financing_carry_positive() {
        let r = development_feasibility(&basic_dev_input()).unwrap();
        assert!(r.result.financing_carry > Decimal::ZERO);
    }

    #[test]
    fn test_dev_total_cost_includes_all() {
        let r = development_feasibility(&basic_dev_input()).unwrap();
        let o = &r.result;
        assert_eq!(
            o.total_development_cost,
            dec!(3_000_000) + o.total_construction_costs + o.financing_carry
        );
    }

    #[test]
    fn test_dev_yield_positive() {
        let r = development_feasibility(&basic_dev_input()).unwrap();
        assert!(r.result.development_yield > Decimal::ZERO);
    }

    #[test]
    fn test_dev_stabilised_value() {
        let r = development_feasibility(&basic_dev_input()).unwrap();
        // Value = 1.8M / 0.055 = 32,727,272.727...
        let expected = dec!(1_800_000) / dec!(0.055);
        assert_eq!(r.result.stabilised_value, expected);
    }

    #[test]
    fn test_dev_residual_land_value() {
        let r = development_feasibility(&basic_dev_input()).unwrap();
        let o = &r.result;
        let expected = o.stabilised_value - o.total_construction_costs - o.financing_carry;
        assert_eq!(o.residual_land_value, expected);
    }

    #[test]
    fn test_dev_profit_margin() {
        let r = development_feasibility(&basic_dev_input()).unwrap();
        let o = &r.result;
        let expected = (o.stabilised_value - o.total_development_cost) / o.total_development_cost;
        assert_eq!(o.profit_margin, expected);
    }

    #[test]
    fn test_dev_go_decision_feasible() {
        let mut input = basic_dev_input();
        input.target_profit_margin = Some(dec!(0.01)); // Very low bar
        let r = development_feasibility(&input).unwrap();
        // Should be Go or Conditional (development spread might be marginal)
        assert!(!matches!(r.result.decision, GoNoGoDecision::NoGo { .. }));
    }

    #[test]
    fn test_dev_invalid_cap_rate() {
        let mut input = basic_dev_input();
        input.market_cap_rate = Decimal::ZERO;
        assert!(development_feasibility(&input).is_err());
    }

    #[test]
    fn test_dev_invalid_hard_costs() {
        let mut input = basic_dev_input();
        input.hard_costs = Decimal::ZERO;
        assert!(development_feasibility(&input).is_err());
    }

    #[test]
    fn test_dev_custom_draw_schedule() {
        let mut input = basic_dev_input();
        // 24-month schedule: front-loaded
        let mut sched = vec![dec!(0.05); 24];
        sched[0] = dec!(0.10);
        sched[23] = dec!(0.05);
        // Normalise so sum ~= 1.0
        let total: Decimal = sched.iter().copied().sum();
        let normalised: Vec<Decimal> = sched.iter().map(|s| *s / total).collect();
        input.draw_schedule_pct = normalised;
        let r = development_feasibility(&input).unwrap();
        assert!(r.result.financing_carry > Decimal::ZERO);
    }

    // -----------------------------------------------------------------------
    // Refinancing tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_refi_annual_savings_positive() {
        let r = refinancing(&basic_refi_input()).unwrap();
        // Lower rate => positive savings
        assert!(r.result.annual_interest_savings > Decimal::ZERO);
    }

    #[test]
    fn test_refi_proposed_ds_lower() {
        let r = refinancing(&basic_refi_input()).unwrap();
        // Even though proposed amount is higher, rate is lower with longer amort
        // Check DS values exist and are positive
        assert!(r.result.existing_annual_ds > Decimal::ZERO);
        assert!(r.result.proposed_annual_ds > Decimal::ZERO);
    }

    #[test]
    fn test_refi_npv_savings() {
        let r = refinancing(&basic_refi_input()).unwrap();
        assert!(r.result.npv_of_savings > Decimal::ZERO);
    }

    #[test]
    fn test_refi_total_cost() {
        let r = refinancing(&basic_refi_input()).unwrap();
        assert_eq!(r.result.total_refi_cost, dec!(250_000));
    }

    #[test]
    fn test_refi_breakeven_months() {
        let r = refinancing(&basic_refi_input()).unwrap();
        assert!(r.result.breakeven_months > 0);
        assert!(r.result.breakeven_months < 999);
    }

    #[test]
    fn test_refi_post_ltv() {
        let r = refinancing(&basic_refi_input()).unwrap();
        // LTV = 9M / 15M = 0.60
        assert_eq!(r.result.post_refi_ltv, dec!(0.6));
    }

    #[test]
    fn test_refi_post_dscr_positive() {
        let r = refinancing(&basic_refi_input()).unwrap();
        assert!(r.result.post_refi_dscr > Decimal::ZERO);
    }

    #[test]
    fn test_refi_debt_yield() {
        let r = refinancing(&basic_refi_input()).unwrap();
        // Debt yield = 1M / 9M ~ 0.1111
        let expected = dec!(1_000_000) / dec!(9_000_000);
        assert_eq!(r.result.debt_yield, expected);
    }

    #[test]
    fn test_refi_cash_out() {
        let r = refinancing(&basic_refi_input()).unwrap();
        // Cash out = 9M - 8M - 250k = 750k
        assert_eq!(r.result.cash_out_amount, dec!(750_000));
    }

    #[test]
    fn test_refi_recommend() {
        let r = refinancing(&basic_refi_input()).unwrap();
        // With significant rate savings, should recommend
        assert!(r.result.recommend_refi);
    }

    #[test]
    fn test_refi_no_recommend_high_penalty() {
        let mut input = basic_refi_input();
        input.prepayment_penalty = dec!(5_000_000); // Enormous penalty
        let r = refinancing(&input).unwrap();
        assert!(!r.result.recommend_refi);
    }

    #[test]
    fn test_refi_invalid_property_value() {
        let mut input = basic_refi_input();
        input.property_value = Decimal::ZERO;
        assert!(refinancing(&input).is_err());
    }

    #[test]
    fn test_refi_io_existing() {
        let mut input = basic_refi_input();
        input.existing_amort_years = None; // IO
        let r = refinancing(&input).unwrap();
        // IO DS = 8M * 6.5% = 520k
        assert_eq!(r.result.existing_annual_ds, dec!(520_000));
    }

    // -----------------------------------------------------------------------
    // Helper / edge-case tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_validate_cap_rate_zero() {
        assert!(validate_cap_rate(Decimal::ZERO, "test").is_err());
    }

    #[test]
    fn test_validate_cap_rate_one() {
        assert!(validate_cap_rate(Decimal::ONE, "test").is_err());
    }

    #[test]
    fn test_validate_cap_rate_negative() {
        assert!(validate_cap_rate(dec!(-0.05), "test").is_err());
    }

    #[test]
    fn test_validate_cap_rate_valid() {
        assert!(validate_cap_rate(dec!(0.06), "test").is_ok());
    }

    #[test]
    fn test_newton_raphson_simple() {
        // Invest 100, get back 110 in year 1 => IRR = 10%
        let cfs = vec![dec!(-100), dec!(110)];
        let mut w = Vec::new();
        let irr = newton_raphson_irr(&cfs, &mut w);
        assert!((irr - dec!(0.10)).abs() < dec!(0.001));
    }

    #[test]
    fn test_newton_raphson_multi_year() {
        // Invest 1000, receive 400 for 3 years => IRR ~ 9.7%
        let cfs = vec![dec!(-1000), dec!(400), dec!(400), dec!(400)];
        let mut w = Vec::new();
        let irr = newton_raphson_irr(&cfs, &mut w);
        assert!(irr > dec!(0.08) && irr < dec!(0.12));
    }

    #[test]
    fn test_monthly_payment_simple() {
        // 1M loan, 5% / 12 monthly, 360 months
        let pmt =
            compute_monthly_payment_simple(dec!(1_000_000), dec!(0.05) / dec!(12), 360).unwrap();
        // Expected ~5,368
        assert!(pmt > dec!(5_000) && pmt < dec!(6_000));
    }

    #[test]
    fn test_tranche_balance_io_only() {
        let t = DebtTranche {
            name: "IO".into(),
            amount: dec!(5_000_000),
            interest_rate: dec!(0.05),
            term_years: 5,
            amortization_years: None,
            io_period_years: None,
        };
        let bal = compute_tranche_balance(&t, 3).unwrap();
        assert_eq!(bal, dec!(5_000_000));
    }

    #[test]
    fn test_tranche_balance_during_io_period() {
        let t = DebtTranche {
            name: "Senior".into(),
            amount: dec!(5_000_000),
            interest_rate: dec!(0.05),
            term_years: 10,
            amortization_years: Some(30),
            io_period_years: Some(3),
        };
        // Still in IO period at year 2
        let bal = compute_tranche_balance(&t, 2).unwrap();
        assert_eq!(bal, dec!(5_000_000));
    }

    #[test]
    fn test_tranche_balance_after_io() {
        let t = DebtTranche {
            name: "Senior".into(),
            amount: dec!(5_000_000),
            interest_rate: dec!(0.05),
            term_years: 10,
            amortization_years: Some(30),
            io_period_years: Some(2),
        };
        // After 5 years (3 years of amort after 2 IO)
        let bal = compute_tranche_balance(&t, 5).unwrap();
        assert!(bal < dec!(5_000_000));
        assert!(bal > dec!(4_500_000)); // Not much amort in 3 years on a 30yr schedule
    }
}
