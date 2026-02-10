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

/// Valuation methodology selector for property analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValuationMethod {
    /// Income capitalisation approach: Value = NOI / cap_rate
    DirectCap,
    /// Discounted cash flow of projected NOI over holding period
    Dcf,
    /// Gross rent multiplier from comparable sales
    GrossRentMultiplier,
    /// Run all applicable methods and compare
    All,
}

/// A comparable property sale used in GRM analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparableSale {
    pub address: String,
    pub sale_price: Money,
    pub gross_rent: Money,
}

/// Input parameters for property valuation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyValuationInput {
    /// Property identifier
    pub property_name: String,
    /// Which valuation method(s) to run
    pub valuation_method: ValuationMethod,
    /// Annual gross potential rental income
    pub gross_potential_rent: Money,
    /// Vacancy and collection loss rate (e.g. 0.05 = 5%)
    pub vacancy_rate: Rate,
    /// Other income (parking, laundry, etc.)
    pub other_income: Money,
    /// Annual operating expenses (taxes, insurance, maintenance, management)
    pub operating_expenses: Money,
    /// Annual capital reserve / replacement allowance
    pub capital_reserves: Money,
    /// Acquisition price (for return calculations)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purchase_price: Option<Money>,
    /// Equity portion of acquisition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub equity_investment: Option<Money>,
    /// Mortgage / loan amount
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loan_amount: Option<Money>,
    /// Annual mortgage interest rate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loan_rate: Option<Rate>,
    /// Mortgage term in years (balloon date)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loan_term_years: Option<u32>,
    /// Amortization period in years (may exceed term for balloon)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loan_amortization_years: Option<u32>,
    /// Market capitalisation rate (for direct cap method)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cap_rate: Option<Rate>,
    /// Annual rent growth rate
    pub market_rent_growth: Rate,
    /// Annual expense growth rate
    pub expense_growth: Rate,
    /// Exit / reversion cap rate at sale
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_cap_rate: Option<Rate>,
    /// Investment holding period in years (for DCF)
    pub holding_period_years: u32,
    /// Discount rate for DCF (WACC or required return)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discount_rate: Option<Rate>,
    /// Comparable sales for GRM analysis
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comparable_sales: Option<Vec<ComparableSale>>,
}

/// Result of direct capitalisation method.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectCapResult {
    /// Indicated value = NOI / cap_rate
    pub value: Money,
    /// Capitalisation rate used
    pub cap_rate: Rate,
    /// Net operating income (year 1)
    pub noi: Money,
    /// Price per unit of NOI (inverse of cap rate)
    pub price_per_unit_noi: Decimal,
}

/// Result of discounted cash flow analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DcfResult {
    /// Property value = PV of cash flows + PV of terminal value
    pub property_value: Money,
    /// Projected NOI for each year of holding period
    pub projected_noi: Vec<Money>,
    /// After-debt cash flows per year (if leveraged)
    pub projected_cash_flows: Vec<Money>,
    /// Terminal / reversion value at exit
    pub terminal_value: Money,
    /// Present value of periodic cash flows
    pub pv_cash_flows: Money,
    /// Present value of terminal value
    pub pv_terminal: Money,
    /// Unlevered IRR
    pub irr: Decimal,
    /// Levered IRR on equity (if financing provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub levered_irr: Option<Decimal>,
}

/// Result of gross rent multiplier analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrmResult {
    /// Indicated value = Gross Rent * avg GRM
    pub value: Money,
    /// Average GRM from comparables
    pub grm: Decimal,
    /// Individual GRMs from each comparable
    pub comparable_grms: Vec<Decimal>,
}

/// Leveraged return metrics when financing is provided.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeveragedReturns {
    /// Loan-to-value ratio
    pub ltv: Decimal,
    /// Debt service coverage ratio (NOI / annual debt service)
    pub dscr: Decimal,
    /// Year 1 cash-on-cash return (year 1 after-debt CF / equity)
    pub cash_on_cash_year1: Decimal,
    /// Equity multiple (total distributions / equity invested)
    pub equity_multiple: Decimal,
    /// Annual debt service (12 * monthly payment)
    pub annual_debt_service: Money,
    /// Monthly mortgage payment
    pub monthly_payment: Money,
}

/// Complete property valuation output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyValuationOutput {
    /// Net operating income (year 1)
    pub noi: Money,
    /// Effective gross income (year 1)
    pub effective_gross_income: Money,
    /// Operating expense ratio (OpEx / EGI)
    pub operating_expense_ratio: Decimal,
    /// Direct capitalisation result (if method selected)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direct_cap_value: Option<DirectCapResult>,
    /// DCF result (if method selected)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dcf_value: Option<DcfResult>,
    /// GRM result (if method selected)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grm_value: Option<GrmResult>,
    /// Leveraged return metrics (if financing provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leveraged_returns: Option<LeveragedReturns>,
    /// Recommended value range (low, high) from methods used
    pub recommended_value_range: (Money, Money),
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Value a property using the selected methodology (direct cap, DCF, GRM, or all).
///
/// Returns a `ComputationOutput<PropertyValuationOutput>` containing detailed
/// valuation results, warnings for unusual metrics, and computation metadata.
pub fn value_property(
    input: &PropertyValuationInput,
) -> CorpFinanceResult<ComputationOutput<PropertyValuationOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // --- Validate common inputs ---
    validate_input(input, &mut warnings)?;

    // --- Core income metrics (year 1) ---
    let vacancy_loss = input.gross_potential_rent * input.vacancy_rate;
    let effective_gross_income = input.gross_potential_rent - vacancy_loss + input.other_income;
    let total_expenses = input.operating_expenses + input.capital_reserves;
    let noi = effective_gross_income - total_expenses;

    let operating_expense_ratio = if effective_gross_income.is_zero() {
        Decimal::ZERO
    } else {
        total_expenses / effective_gross_income
    };

    // --- Leveraged returns (computed once, shared across methods) ---
    let leverage = compute_leveraged_returns(input, noi, &mut warnings)?;

    // --- Run selected methods ---
    let direct_cap_value = match input.valuation_method {
        ValuationMethod::DirectCap | ValuationMethod::All => {
            Some(compute_direct_cap(input, noi, &mut warnings)?)
        }
        _ => None,
    };

    let dcf_value = match input.valuation_method {
        ValuationMethod::Dcf | ValuationMethod::All => {
            Some(compute_dcf(input, noi, leverage.as_ref(), &mut warnings)?)
        }
        _ => None,
    };

    let grm_value = match input.valuation_method {
        ValuationMethod::GrossRentMultiplier | ValuationMethod::All => {
            compute_grm(input, &mut warnings)?
        }
        _ => None,
    };

    // --- Recommended value range ---
    let recommended_value_range = derive_value_range(
        direct_cap_value.as_ref(),
        dcf_value.as_ref(),
        grm_value.as_ref(),
    );

    let output = PropertyValuationOutput {
        noi,
        effective_gross_income,
        operating_expense_ratio,
        direct_cap_value,
        dcf_value,
        grm_value,
        leveraged_returns: leverage,
        recommended_value_range,
    };

    let elapsed = start.elapsed().as_micros() as u64;

    Ok(with_metadata(
        "Real Estate Property Valuation (Income Approach)",
        input,
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_input(
    input: &PropertyValuationInput,
    warnings: &mut Vec<String>,
) -> CorpFinanceResult<()> {
    if input.holding_period_years < 1 {
        return Err(CorpFinanceError::InvalidInput {
            field: "holding_period_years".into(),
            reason: "Holding period must be at least 1 year".into(),
        });
    }

    if input.gross_potential_rent <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "gross_potential_rent".into(),
            reason: "Gross potential rent must be positive".into(),
        });
    }

    if input.vacancy_rate < Decimal::ZERO || input.vacancy_rate >= Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "vacancy_rate".into(),
            reason: "Vacancy rate must be between 0 and 1 (exclusive upper)".into(),
        });
    }

    // Compute NOI to validate it for cap-rate methods
    let vacancy_loss = input.gross_potential_rent * input.vacancy_rate;
    let egi = input.gross_potential_rent - vacancy_loss + input.other_income;
    let total_exp = input.operating_expenses + input.capital_reserves;
    let noi = egi - total_exp;

    match input.valuation_method {
        ValuationMethod::DirectCap | ValuationMethod::All => {
            if noi <= Decimal::ZERO {
                return Err(CorpFinanceError::InvalidInput {
                    field: "noi".into(),
                    reason: "NOI must be positive for capitalisation-based valuation".into(),
                });
            }
            if let Some(cap) = input.cap_rate {
                if cap <= Decimal::ZERO {
                    return Err(CorpFinanceError::InvalidInput {
                        field: "cap_rate".into(),
                        reason: "Cap rate must be positive".into(),
                    });
                }
            }
        }
        _ => {}
    }

    match input.valuation_method {
        ValuationMethod::Dcf | ValuationMethod::All => {
            if input.discount_rate.is_none() {
                return Err(CorpFinanceError::InvalidInput {
                    field: "discount_rate".into(),
                    reason: "Discount rate is required for DCF valuation".into(),
                });
            }
            if let Some(dr) = input.discount_rate {
                if dr <= Decimal::ZERO {
                    return Err(CorpFinanceError::InvalidInput {
                        field: "discount_rate".into(),
                        reason: "Discount rate must be positive".into(),
                    });
                }
            }
            if input.exit_cap_rate.is_none() {
                return Err(CorpFinanceError::InvalidInput {
                    field: "exit_cap_rate".into(),
                    reason: "Exit cap rate is required for DCF terminal value".into(),
                });
            }
            if let Some(ecr) = input.exit_cap_rate {
                if ecr <= Decimal::ZERO {
                    return Err(CorpFinanceError::InvalidInput {
                        field: "exit_cap_rate".into(),
                        reason: "Exit cap rate must be positive".into(),
                    });
                }
            }
        }
        _ => {}
    }

    match input.valuation_method {
        ValuationMethod::GrossRentMultiplier | ValuationMethod::All => {
            if input.comparable_sales.is_none()
                || input.comparable_sales.as_ref().is_none_or(|v| v.is_empty())
            {
                match input.valuation_method {
                    ValuationMethod::GrossRentMultiplier => {
                        return Err(CorpFinanceError::InsufficientData(
                            "At least one comparable sale is required for GRM analysis".into(),
                        ));
                    }
                    ValuationMethod::All => {
                        // For All mode, GRM is optional if no comps
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }

    // --- Warnings for unusual metrics ---
    if let Some(cap) = input.cap_rate {
        if cap < dec!(0.03) {
            warnings.push(format!(
                "Cap rate {cap} is below 3% — unusually low, verify market data"
            ));
        }
        if cap > dec!(0.12) {
            warnings.push(format!(
                "Cap rate {cap} exceeds 12% — unusually high, may indicate elevated risk"
            ));
        }
    }

    if input.vacancy_rate > dec!(0.15) {
        warnings.push(format!(
            "Vacancy rate {:.1}% exceeds 15% — above typical market norms",
            input.vacancy_rate * dec!(100)
        ));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Direct Capitalisation
// ---------------------------------------------------------------------------

fn compute_direct_cap(
    input: &PropertyValuationInput,
    noi: Money,
    warnings: &mut Vec<String>,
) -> CorpFinanceResult<DirectCapResult> {
    let cap_rate = input
        .cap_rate
        .ok_or_else(|| CorpFinanceError::InvalidInput {
            field: "cap_rate".into(),
            reason: "Cap rate is required for direct capitalisation method".into(),
        })?;

    if cap_rate.is_zero() {
        return Err(CorpFinanceError::DivisionByZero {
            context: "direct capitalisation (NOI / cap_rate)".into(),
        });
    }

    let value = noi / cap_rate;
    let price_per_unit_noi = Decimal::ONE / cap_rate;

    if value < Decimal::ZERO {
        warnings.push("Direct cap produces negative value — check NOI and cap rate".into());
    }

    Ok(DirectCapResult {
        value,
        cap_rate,
        noi,
        price_per_unit_noi,
    })
}

// ---------------------------------------------------------------------------
// DCF
// ---------------------------------------------------------------------------

fn compute_dcf(
    input: &PropertyValuationInput,
    _noi_year1: Money,
    leverage: Option<&LeveragedReturns>,
    warnings: &mut Vec<String>,
) -> CorpFinanceResult<DcfResult> {
    let discount_rate = input.discount_rate.unwrap(); // validated
    let exit_cap_rate = input.exit_cap_rate.unwrap(); // validated
    let n = input.holding_period_years as usize;

    // --- Project NOI ---
    let mut projected_noi = Vec::with_capacity(n);
    let mut current_rent = input.gross_potential_rent;
    let mut current_expenses = input.operating_expenses;
    let mut current_cap_reserves = input.capital_reserves;

    for year in 0..n {
        if year > 0 {
            current_rent *= Decimal::ONE + input.market_rent_growth;
            current_expenses *= Decimal::ONE + input.expense_growth;
            current_cap_reserves *= Decimal::ONE + input.expense_growth;
        }

        let vacancy_loss = current_rent * input.vacancy_rate;
        let egi = current_rent - vacancy_loss + input.other_income;
        let year_noi = egi - current_expenses - current_cap_reserves;
        projected_noi.push(year_noi);
    }

    // --- Compute annual debt service for leveraged cash flows ---
    let annual_ds = leverage
        .map(|l| l.annual_debt_service)
        .unwrap_or(Decimal::ZERO);

    // --- Projected after-debt cash flows ---
    let projected_cash_flows: Vec<Money> = projected_noi
        .iter()
        .map(|&noi_yr| noi_yr - annual_ds)
        .collect();

    // --- Terminal value: NOI at exit / exit_cap_rate ---
    // Project one more year of NOI for exit
    let exit_rent = current_rent * (Decimal::ONE + input.market_rent_growth);
    let exit_vacancy = exit_rent * input.vacancy_rate;
    let exit_egi = exit_rent - exit_vacancy + input.other_income;
    let exit_expenses = current_expenses * (Decimal::ONE + input.expense_growth);
    let exit_cap_reserves = current_cap_reserves * (Decimal::ONE + input.expense_growth);
    let exit_noi = exit_egi - exit_expenses - exit_cap_reserves;

    let terminal_value = exit_noi / exit_cap_rate;

    // --- PV of cash flows (unlevered: use NOI) ---
    let mut pv_cash_flows = Decimal::ZERO;
    let mut discount_factor = Decimal::ONE;
    let one_plus_r = Decimal::ONE + discount_rate;

    for year_noi in &projected_noi {
        discount_factor /= one_plus_r;
        pv_cash_flows += *year_noi * discount_factor;
    }

    // --- PV of terminal value (discounted at end of holding period) ---
    // discount_factor is already at 1/(1+r)^n after the loop
    let pv_terminal = terminal_value * discount_factor;

    let property_value = pv_cash_flows + pv_terminal;

    // --- Unlevered IRR ---
    // Cash flows: -purchase_price at t=0, NOI_1..NOI_n, terminal_value at t=n
    let purchase = input.purchase_price.unwrap_or(property_value);

    let mut unlev_cfs = Vec::with_capacity(n + 1);
    unlev_cfs.push(-purchase);
    for (i, noi_yr) in projected_noi.iter().enumerate() {
        if i == n - 1 {
            unlev_cfs.push(*noi_yr + terminal_value);
        } else {
            unlev_cfs.push(*noi_yr);
        }
    }
    let irr = newton_raphson_irr(&unlev_cfs, warnings);

    // --- Levered IRR ---
    let levered_irr = if leverage.is_some() && input.equity_investment.is_some() {
        let equity = input.equity_investment.unwrap();
        let loan_bal = compute_loan_balance_at_year(input, n)?;
        let mut lev_cfs = Vec::with_capacity(n + 1);
        lev_cfs.push(-equity);
        for (i, cf) in projected_cash_flows.iter().enumerate() {
            if i == n - 1 {
                // Sale proceeds minus loan payoff plus last year cash flow
                lev_cfs.push(*cf + terminal_value - loan_bal);
            } else {
                lev_cfs.push(*cf);
            }
        }
        Some(newton_raphson_irr(&lev_cfs, warnings))
    } else {
        None
    };

    if property_value < Decimal::ZERO {
        warnings.push(
            "DCF produces negative property value — review projections and discount rate".into(),
        );
    }

    Ok(DcfResult {
        property_value,
        projected_noi,
        projected_cash_flows,
        terminal_value,
        pv_cash_flows,
        pv_terminal,
        irr,
        levered_irr,
    })
}

// ---------------------------------------------------------------------------
// GRM
// ---------------------------------------------------------------------------

fn compute_grm(
    input: &PropertyValuationInput,
    warnings: &mut Vec<String>,
) -> CorpFinanceResult<Option<GrmResult>> {
    let comps = match &input.comparable_sales {
        Some(v) if !v.is_empty() => v,
        _ => return Ok(None),
    };

    let mut comparable_grms = Vec::with_capacity(comps.len());
    for comp in comps {
        if comp.gross_rent.is_zero() {
            warnings.push(format!(
                "Comparable at {} has zero gross rent — excluded from GRM",
                comp.address
            ));
            continue;
        }
        let grm = comp.sale_price / comp.gross_rent;
        comparable_grms.push(grm);
    }

    if comparable_grms.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "No valid comparables for GRM calculation (all had zero rent)".into(),
        ));
    }

    let sum: Decimal = comparable_grms.iter().copied().sum();
    let count = Decimal::from(comparable_grms.len() as u32);
    let avg_grm = sum / count;

    let value = input.gross_potential_rent * avg_grm;

    Ok(Some(GrmResult {
        value,
        grm: avg_grm,
        comparable_grms,
    }))
}

// ---------------------------------------------------------------------------
// Leveraged Returns
// ---------------------------------------------------------------------------

fn compute_leveraged_returns(
    input: &PropertyValuationInput,
    noi: Money,
    warnings: &mut Vec<String>,
) -> CorpFinanceResult<Option<LeveragedReturns>> {
    let (loan_amount, loan_rate, _loan_term, amort_years, equity, purchase_price) = match (
        input.loan_amount,
        input.loan_rate,
        input.loan_term_years,
        input.loan_amortization_years,
        input.equity_investment,
        input.purchase_price,
    ) {
        (Some(la), Some(lr), Some(lt), Some(ay), Some(eq), Some(pp)) => (la, lr, lt, ay, eq, pp),
        _ => return Ok(None),
    };

    if purchase_price.is_zero() {
        return Err(CorpFinanceError::DivisionByZero {
            context: "LTV calculation (loan / purchase_price)".into(),
        });
    }

    // --- Monthly payment: P * r(1+r)^n / ((1+r)^n - 1) ---
    let monthly_rate = loan_rate / dec!(12);
    let total_months = amort_years * 12;
    let monthly_payment = compute_monthly_payment(loan_amount, monthly_rate, total_months)?;

    let annual_debt_service = monthly_payment * dec!(12);

    // --- LTV ---
    let ltv = loan_amount / purchase_price;

    // --- DSCR ---
    let dscr = if annual_debt_service.is_zero() {
        Decimal::ZERO
    } else {
        noi / annual_debt_service
    };

    // --- Cash-on-cash year 1 ---
    let year1_cf = noi - annual_debt_service;
    let cash_on_cash_year1 = if equity.is_zero() {
        Decimal::ZERO
    } else {
        year1_cf / equity
    };

    // --- Equity multiple (simplified: holding period NOI - DS + terminal) ---
    let n = input.holding_period_years as usize;
    let mut total_cf = Decimal::ZERO;
    let mut current_rent = input.gross_potential_rent;
    let mut current_expenses = input.operating_expenses;
    let mut current_cap_reserves = input.capital_reserves;

    for year in 0..n {
        if year > 0 {
            current_rent *= Decimal::ONE + input.market_rent_growth;
            current_expenses *= Decimal::ONE + input.expense_growth;
            current_cap_reserves *= Decimal::ONE + input.expense_growth;
        }
        let vacancy_loss = current_rent * input.vacancy_rate;
        let egi = current_rent - vacancy_loss + input.other_income;
        let year_noi = egi - current_expenses - current_cap_reserves;
        total_cf += year_noi - annual_debt_service;
    }

    // Add sale proceeds at exit
    if let Some(exit_cap) = input.exit_cap_rate {
        let exit_rent = current_rent * (Decimal::ONE + input.market_rent_growth);
        let exit_vacancy = exit_rent * input.vacancy_rate;
        let exit_egi = exit_rent - exit_vacancy + input.other_income;
        let exit_expenses = current_expenses * (Decimal::ONE + input.expense_growth);
        let exit_cap_reserves = current_cap_reserves * (Decimal::ONE + input.expense_growth);
        let exit_noi = exit_egi - exit_expenses - exit_cap_reserves;
        let terminal = exit_noi / exit_cap;

        let loan_bal = compute_loan_balance_at_year(input, n)?;
        total_cf += terminal - loan_bal;
    }

    let equity_multiple = if equity.is_zero() {
        Decimal::ZERO
    } else {
        total_cf / equity
    };

    // --- Warnings ---
    if dscr < dec!(1.2) && dscr > Decimal::ZERO {
        warnings.push(format!(
            "DSCR of {dscr:.2} is below 1.20x — lender covenant risk"
        ));
    }

    if ltv > dec!(0.80) {
        warnings.push(format!(
            "LTV of {:.1}% exceeds 80% — high leverage",
            ltv * dec!(100)
        ));
    }

    Ok(Some(LeveragedReturns {
        ltv,
        dscr,
        cash_on_cash_year1,
        equity_multiple,
        annual_debt_service,
        monthly_payment,
    }))
}

// ---------------------------------------------------------------------------
// Mortgage helpers
// ---------------------------------------------------------------------------

/// Standard fixed-rate mortgage payment: P * r(1+r)^n / ((1+r)^n - 1)
fn compute_monthly_payment(
    principal: Money,
    monthly_rate: Rate,
    total_months: u32,
) -> CorpFinanceResult<Money> {
    if monthly_rate.is_zero() {
        // Interest-free: straight-line amortisation
        if total_months == 0 {
            return Err(CorpFinanceError::DivisionByZero {
                context: "monthly payment with zero rate and zero months".into(),
            });
        }
        return Ok(principal / Decimal::from(total_months));
    }

    // (1 + r)^n via iterative multiplication
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

/// Compute outstanding loan balance after `years` of payments.
fn compute_loan_balance_at_year(
    input: &PropertyValuationInput,
    years: usize,
) -> CorpFinanceResult<Money> {
    let loan_amount = match input.loan_amount {
        Some(la) => la,
        None => return Ok(Decimal::ZERO),
    };
    let loan_rate = match input.loan_rate {
        Some(lr) => lr,
        None => return Ok(loan_amount), // no rate means no amortisation
    };
    let amort_years = match input.loan_amortization_years {
        Some(ay) => ay,
        None => return Ok(loan_amount),
    };

    let monthly_rate = loan_rate / dec!(12);
    let total_months = amort_years * 12;
    let payments_made = (years as u32) * 12;

    if monthly_rate.is_zero() {
        // Straight-line amortisation
        let paid = loan_amount * Decimal::from(payments_made) / Decimal::from(total_months);
        return Ok(loan_amount - paid);
    }

    let monthly_pmt = compute_monthly_payment(loan_amount, monthly_rate, total_months)?;

    // Track balance through amortisation schedule
    let mut balance = loan_amount;
    for _ in 0..payments_made {
        let interest = balance * monthly_rate;
        let principal_payment = monthly_pmt - interest;
        balance -= principal_payment;
        if balance < Decimal::ZERO {
            balance = Decimal::ZERO;
            break;
        }
    }

    Ok(balance)
}

// ---------------------------------------------------------------------------
// IRR (Newton-Raphson)
// ---------------------------------------------------------------------------

/// Newton-Raphson IRR solver. cash_flows[0] is typically negative (investment).
/// Returns the rate r where NPV(r) = 0.
fn newton_raphson_irr(cash_flows: &[Money], warnings: &mut Vec<String>) -> Decimal {
    let max_iter = 30;
    let epsilon = dec!(0.0000001); // 1e-7
    let mut rate = dec!(0.10); // initial guess

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

/// NPV(r) = sum CF_t / (1+r)^t and its derivative d(NPV)/dr.
fn npv_and_derivative(cash_flows: &[Money], rate: Decimal) -> (Decimal, Decimal) {
    let one_plus_r = Decimal::ONE + rate;
    let mut npv = Decimal::ZERO;
    let mut dnpv = Decimal::ZERO;
    let mut discount = Decimal::ONE; // (1+r)^0 = 1

    for (t, cf) in cash_flows.iter().enumerate() {
        npv += *cf * discount;
        if t > 0 {
            // d/dr of CF_t / (1+r)^t = -t * CF_t / (1+r)^(t+1)
            dnpv += Decimal::from(-(t as i64)) * *cf * discount / one_plus_r;
        }
        discount /= one_plus_r;
    }

    (npv, dnpv)
}

// ---------------------------------------------------------------------------
// Value range
// ---------------------------------------------------------------------------

fn derive_value_range(
    direct_cap: Option<&DirectCapResult>,
    dcf: Option<&DcfResult>,
    grm: Option<&GrmResult>,
) -> (Money, Money) {
    let mut values: Vec<Money> = Vec::new();

    if let Some(dc) = direct_cap {
        values.push(dc.value);
    }
    if let Some(d) = dcf {
        values.push(d.property_value);
    }
    if let Some(g) = grm {
        values.push(g.value);
    }

    if values.is_empty() {
        return (Decimal::ZERO, Decimal::ZERO);
    }

    let min = values.iter().copied().fold(values[0], |a, b| a.min(b));
    let max = values.iter().copied().fold(values[0], |a, b| a.max(b));

    (min, max)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// Standard test property: 10-unit apartment building
    fn sample_input() -> PropertyValuationInput {
        PropertyValuationInput {
            property_name: "Test Apartments".into(),
            valuation_method: ValuationMethod::All,
            gross_potential_rent: dec!(120000),    // $120k annual
            vacancy_rate: dec!(0.05),              // 5%
            other_income: dec!(6000),              // parking, laundry
            operating_expenses: dec!(48000),       // taxes, insurance, maint, mgmt
            capital_reserves: dec!(6000),          // reserves
            purchase_price: Some(dec!(1000000)),   // $1M
            equity_investment: Some(dec!(250000)), // $250k (25% down)
            loan_amount: Some(dec!(750000)),       // $750k mortgage
            loan_rate: Some(dec!(0.065)),          // 6.5%
            loan_term_years: Some(30),
            loan_amortization_years: Some(30),
            cap_rate: Some(dec!(0.06)),       // 6% market cap
            market_rent_growth: dec!(0.03),   // 3% annual
            expense_growth: dec!(0.02),       // 2% annual
            exit_cap_rate: Some(dec!(0.065)), // 6.5% exit cap
            holding_period_years: 5,
            discount_rate: Some(dec!(0.08)), // 8% required return
            comparable_sales: Some(vec![
                ComparableSale {
                    address: "100 Main St".into(),
                    sale_price: dec!(950000),
                    gross_rent: dec!(110000),
                },
                ComparableSale {
                    address: "200 Oak Ave".into(),
                    sale_price: dec!(1050000),
                    gross_rent: dec!(125000),
                },
                ComparableSale {
                    address: "300 Elm Dr".into(),
                    sale_price: dec!(1100000),
                    gross_rent: dec!(130000),
                },
            ]),
        }
    }

    // --- Direct Cap Tests ---

    #[test]
    fn test_direct_cap_valuation() {
        let mut input = sample_input();
        input.valuation_method = ValuationMethod::DirectCap;
        let result = value_property(&input).unwrap();
        let out = &result.result;

        assert!(out.direct_cap_value.is_some());
        let dc = out.direct_cap_value.as_ref().unwrap();

        // NOI = (120000 * 0.95 + 6000) - 48000 - 6000 = 114000 + 6000 - 54000 = 66000
        assert_eq!(out.noi, dec!(66000));
        assert_eq!(dc.noi, dec!(66000));

        // Value = 66000 / 0.06 = 1,100,000
        assert_eq!(dc.value, dec!(1100000));
        assert_eq!(dc.cap_rate, dec!(0.06));
    }

    #[test]
    fn test_noi_calculation() {
        let input = sample_input();
        let result = value_property(&input).unwrap();
        let out = &result.result;

        // EGI = 120000 * (1 - 0.05) + 6000 = 114000 + 6000 = 120000
        assert_eq!(out.effective_gross_income, dec!(120000));

        // NOI = 120000 - 48000 - 6000 = 66000
        assert_eq!(out.noi, dec!(66000));

        // OpEx ratio = 54000 / 120000 = 0.45
        assert_eq!(out.operating_expense_ratio, dec!(0.45));
    }

    #[test]
    fn test_direct_cap_price_per_unit_noi() {
        let mut input = sample_input();
        input.valuation_method = ValuationMethod::DirectCap;
        let result = value_property(&input).unwrap();
        let dc = result.result.direct_cap_value.as_ref().unwrap();

        // 1 / 0.06 = 16.666...
        let expected = Decimal::ONE / dec!(0.06);
        assert_eq!(dc.price_per_unit_noi, expected);
    }

    // --- DCF Tests ---

    #[test]
    fn test_dcf_5_year_hold() {
        let mut input = sample_input();
        input.valuation_method = ValuationMethod::Dcf;
        let result = value_property(&input).unwrap();
        let out = &result.result;

        assert!(out.dcf_value.is_some());
        let dcf = out.dcf_value.as_ref().unwrap();

        // Should have 5 years of projected NOI
        assert_eq!(dcf.projected_noi.len(), 5);

        // Year 1 NOI should match base NOI
        assert_eq!(dcf.projected_noi[0], dec!(66000));

        // Year 2 NOI: rent grows 3%, expenses grow 2%
        // Rent_Y2 = 120000 * 1.03 = 123600
        // Vacancy_Y2 = 123600 * 0.05 = 6180
        // EGI_Y2 = 123600 - 6180 + 6000 = 123420
        // OpEx_Y2 = 48000 * 1.02 = 48960
        // CapRes_Y2 = 6000 * 1.02 = 6120
        // NOI_Y2 = 123420 - 48960 - 6120 = 68340
        assert_eq!(dcf.projected_noi[1], dec!(68340));

        // Property value should be positive
        assert!(dcf.property_value > Decimal::ZERO);

        // Terminal value should be positive
        assert!(dcf.terminal_value > Decimal::ZERO);

        // PV decomposition
        assert!(dcf.pv_cash_flows > Decimal::ZERO);
        assert!(dcf.pv_terminal > Decimal::ZERO);

        // property_value = pv_cash_flows + pv_terminal
        assert_eq!(dcf.property_value, dcf.pv_cash_flows + dcf.pv_terminal);
    }

    #[test]
    fn test_dcf_irr_computation() {
        let mut input = sample_input();
        input.valuation_method = ValuationMethod::Dcf;
        let result = value_property(&input).unwrap();
        let dcf = result.result.dcf_value.as_ref().unwrap();

        // IRR should be a reasonable rate
        assert!(dcf.irr > dec!(-0.5), "IRR too low: {}", dcf.irr);
        assert!(dcf.irr < dec!(1.0), "IRR too high: {}", dcf.irr);
    }

    #[test]
    fn test_dcf_levered_irr() {
        let mut input = sample_input();
        input.valuation_method = ValuationMethod::Dcf;
        let result = value_property(&input).unwrap();
        let dcf = result.result.dcf_value.as_ref().unwrap();

        // With leverage, levered IRR should exist
        assert!(dcf.levered_irr.is_some());
        let lirr = dcf.levered_irr.unwrap();
        assert!(lirr > dec!(-0.5), "Levered IRR too low: {}", lirr);
        assert!(lirr < dec!(2.0), "Levered IRR too high: {}", lirr);
    }

    #[test]
    fn test_dcf_projected_cash_flows_after_debt() {
        let mut input = sample_input();
        input.valuation_method = ValuationMethod::Dcf;
        let result = value_property(&input).unwrap();
        let dcf = result.result.dcf_value.as_ref().unwrap();

        // After-debt CF should be less than NOI (debt service is positive)
        for (i, (noi, cf)) in dcf
            .projected_noi
            .iter()
            .zip(dcf.projected_cash_flows.iter())
            .enumerate()
        {
            assert!(
                cf < noi,
                "Year {} after-debt CF ({}) should be less than NOI ({})",
                i + 1,
                cf,
                noi
            );
        }
    }

    // --- GRM Tests ---

    #[test]
    fn test_grm_from_comps() {
        let mut input = sample_input();
        input.valuation_method = ValuationMethod::GrossRentMultiplier;
        let result = value_property(&input).unwrap();
        let out = &result.result;

        assert!(out.grm_value.is_some());
        let grm = out.grm_value.as_ref().unwrap();

        // GRMs: 950000/110000=8.636..., 1050000/125000=8.4, 1100000/130000=8.461...
        assert_eq!(grm.comparable_grms.len(), 3);
        assert!(grm.grm > dec!(8.0) && grm.grm < dec!(9.0));

        // Value = 120000 * avg_grm
        assert_eq!(grm.value, input.gross_potential_rent * grm.grm);
    }

    #[test]
    fn test_grm_no_comps_error() {
        let mut input = sample_input();
        input.valuation_method = ValuationMethod::GrossRentMultiplier;
        input.comparable_sales = None;
        let result = value_property(&input);
        assert!(result.is_err());
    }

    // --- Leveraged Returns Tests ---

    #[test]
    fn test_leveraged_returns_with_mortgage() {
        let input = sample_input();
        let result = value_property(&input).unwrap();
        let out = &result.result;

        assert!(out.leveraged_returns.is_some());
        let lev = out.leveraged_returns.as_ref().unwrap();

        // LTV = 750000 / 1000000 = 0.75
        assert_eq!(lev.ltv, dec!(0.75));

        // Monthly payment should be positive
        assert!(lev.monthly_payment > Decimal::ZERO);

        // Annual DS = 12 * monthly
        assert_eq!(lev.annual_debt_service, lev.monthly_payment * dec!(12));

        // DSCR = NOI / annual_ds
        let expected_dscr = dec!(66000) / lev.annual_debt_service;
        assert_eq!(lev.dscr, expected_dscr);
    }

    #[test]
    fn test_dscr_calculation() {
        let input = sample_input();
        let result = value_property(&input).unwrap();
        let lev = result.result.leveraged_returns.as_ref().unwrap();

        // DSCR should be reasonable for a 75% LTV property
        assert!(
            lev.dscr > dec!(1.0),
            "DSCR should be above 1.0, got {}",
            lev.dscr
        );
        assert!(
            lev.dscr < dec!(2.0),
            "DSCR should be below 2.0, got {}",
            lev.dscr
        );
    }

    #[test]
    fn test_cash_on_cash_return() {
        let input = sample_input();
        let result = value_property(&input).unwrap();
        let lev = result.result.leveraged_returns.as_ref().unwrap();

        // Cash-on-cash = (NOI - DS) / equity
        let expected = (dec!(66000) - lev.annual_debt_service) / dec!(250000);
        assert_eq!(lev.cash_on_cash_year1, expected);
    }

    // --- All Methods ---

    #[test]
    fn test_all_methods() {
        let input = sample_input();
        let result = value_property(&input).unwrap();
        let out = &result.result;

        // All three methods should produce results
        assert!(out.direct_cap_value.is_some(), "Direct cap missing");
        assert!(out.dcf_value.is_some(), "DCF missing");
        assert!(out.grm_value.is_some(), "GRM missing");

        // Value range should span from min to max
        let (low, high) = out.recommended_value_range;
        assert!(low > Decimal::ZERO);
        assert!(high >= low);
    }

    #[test]
    fn test_value_range_bounds() {
        let input = sample_input();
        let result = value_property(&input).unwrap();
        let out = &result.result;

        let dc_val = out.direct_cap_value.as_ref().unwrap().value;
        let dcf_val = out.dcf_value.as_ref().unwrap().property_value;
        let grm_val = out.grm_value.as_ref().unwrap().value;

        let (low, high) = out.recommended_value_range;
        assert!(low <= dc_val);
        assert!(low <= dcf_val);
        assert!(low <= grm_val);
        assert!(high >= dc_val);
        assert!(high >= dcf_val);
        assert!(high >= grm_val);
    }

    // --- Zero Vacancy ---

    #[test]
    fn test_zero_vacancy() {
        let mut input = sample_input();
        input.valuation_method = ValuationMethod::DirectCap;
        input.vacancy_rate = Decimal::ZERO;

        let result = value_property(&input).unwrap();
        let out = &result.result;

        // EGI = 120000 + 6000 = 126000
        assert_eq!(out.effective_gross_income, dec!(126000));

        // NOI = 126000 - 54000 = 72000
        assert_eq!(out.noi, dec!(72000));
    }

    // --- High Leverage Warning ---

    #[test]
    fn test_high_leverage_warning() {
        let mut input = sample_input();
        input.loan_amount = Some(dec!(850000)); // 85% LTV
        input.equity_investment = Some(dec!(150000));

        let result = value_property(&input).unwrap();
        let ltv_warning = result.warnings.iter().any(|w| w.contains("exceeds 80%"));
        assert!(ltv_warning, "Expected LTV warning for 85% leverage");
    }

    #[test]
    fn test_low_dscr_warning() {
        let mut input = sample_input();
        // Make DSCR low by increasing loan
        input.loan_amount = Some(dec!(900000));
        input.equity_investment = Some(dec!(100000));

        let result = value_property(&input).unwrap();
        let dscr_warning = result.warnings.iter().any(|w| w.contains("DSCR"));
        assert!(dscr_warning, "Expected DSCR warning for high leverage");
    }

    // --- Cap Rate Warnings ---

    #[test]
    fn test_low_cap_rate_warning() {
        let mut input = sample_input();
        input.valuation_method = ValuationMethod::DirectCap;
        input.cap_rate = Some(dec!(0.025)); // 2.5%

        let result = value_property(&input).unwrap();
        let cap_warning = result.warnings.iter().any(|w| w.contains("below 3%"));
        assert!(cap_warning, "Expected low cap rate warning");
    }

    #[test]
    fn test_high_cap_rate_warning() {
        let mut input = sample_input();
        input.valuation_method = ValuationMethod::DirectCap;
        input.cap_rate = Some(dec!(0.15)); // 15%

        let result = value_property(&input).unwrap();
        let cap_warning = result.warnings.iter().any(|w| w.contains("exceeds 12%"));
        assert!(cap_warning, "Expected high cap rate warning");
    }

    // --- Validation Errors ---

    #[test]
    fn test_holding_period_zero_error() {
        let mut input = sample_input();
        input.holding_period_years = 0;

        let result = value_property(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "holding_period_years");
            }
            other => panic!("Expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn test_negative_rent_error() {
        let mut input = sample_input();
        input.gross_potential_rent = dec!(-100);

        let result = value_property(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_zero_cap_rate_error() {
        let mut input = sample_input();
        input.valuation_method = ValuationMethod::DirectCap;
        input.cap_rate = Some(Decimal::ZERO);

        let result = value_property(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_discount_rate_for_dcf() {
        let mut input = sample_input();
        input.valuation_method = ValuationMethod::Dcf;
        input.discount_rate = None;

        let result = value_property(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_exit_cap_for_dcf() {
        let mut input = sample_input();
        input.valuation_method = ValuationMethod::Dcf;
        input.exit_cap_rate = None;

        let result = value_property(&input);
        assert!(result.is_err());
    }

    // --- Methodology metadata ---

    #[test]
    fn test_methodology_string() {
        let input = sample_input();
        let result = value_property(&input).unwrap();
        assert_eq!(
            result.methodology,
            "Real Estate Property Valuation (Income Approach)"
        );
    }

    // --- Mortgage payment sanity ---

    #[test]
    fn test_monthly_payment_sanity() {
        // $750k at 6.5% over 30 years, expected ~$4,740/mo
        let payment = compute_monthly_payment(dec!(750000), dec!(0.065) / dec!(12), 360).unwrap();

        // Should be in the range $4,700 - $4,800
        assert!(
            payment > dec!(4700) && payment < dec!(4800),
            "Monthly payment {} outside expected range",
            payment
        );
    }

    #[test]
    fn test_zero_rate_mortgage() {
        let payment = compute_monthly_payment(dec!(360000), Decimal::ZERO, 360).unwrap();
        // $360k / 360 months = $1000/mo
        assert_eq!(payment, dec!(1000));
    }

    // --- IRR convergence ---

    #[test]
    fn test_irr_simple_case() {
        // Invest 100, receive 110 in 1 year => IRR = 10%
        let cfs = vec![dec!(-100), dec!(110)];
        let mut warnings = Vec::new();
        let irr = newton_raphson_irr(&cfs, &mut warnings);

        let diff = (irr - dec!(0.10)).abs();
        assert!(
            diff < dec!(0.001),
            "Expected IRR ~10%, got {irr} (diff {diff})"
        );
    }

    #[test]
    fn test_irr_multi_period() {
        // Invest 1000, receive 300/year for 5 years => IRR ~15.24%
        let cfs = vec![
            dec!(-1000),
            dec!(300),
            dec!(300),
            dec!(300),
            dec!(300),
            dec!(300),
        ];
        let mut warnings = Vec::new();
        let irr = newton_raphson_irr(&cfs, &mut warnings);

        assert!(
            irr > dec!(0.14) && irr < dec!(0.17),
            "Expected IRR ~15.2%, got {irr}"
        );
    }
}
