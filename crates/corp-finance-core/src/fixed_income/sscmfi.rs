use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{with_metadata, ComputationOutput, Money, Rate};
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const MAX_ITERATIONS: u32 = 100;
const EPSILON: Decimal = dec!(0.0000001);
const HUNDRED: Decimal = dec!(100);
const EXP_TAYLOR_TERMS: usize = 15;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SscmfiSecurityType {
    Treasury,
    Agency,
    Corporate,
    Municipal,
    CD,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SscmfiPaymentType {
    Periodic,
    Discount,
    IAM,
    Stepped,
    Multistep,
    PIK,
    PartPIK,
}

impl Default for SscmfiPaymentType {
    fn default() -> Self {
        Self::Periodic
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SscmfiDayCount {
    SSCM30_360,
    ActualActual,
    Actual360,
    Actual365,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SscmfiEomRule {
    Adjust,
    NoAdjust,
}

impl Default for SscmfiEomRule {
    fn default() -> Self {
        Self::Adjust
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SscmfiFrequency {
    Annual,
    Semiannual,
    Quarterly,
    Monthly,
}

impl SscmfiFrequency {
    pub fn to_u8(&self) -> u8 {
        match self {
            Self::Annual => 1,
            Self::Semiannual => 2,
            Self::Quarterly => 4,
            Self::Monthly => 12,
        }
    }
}

// ---------------------------------------------------------------------------
// Input / Output types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallRedemption {
    pub date: String,
    pub price: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepSchedule {
    pub date: String,
    pub coupon_rate: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SscmfiBondInput {
    pub security_type: SscmfiSecurityType,
    #[serde(default)]
    pub payment_type: SscmfiPaymentType,
    pub maturity_date: String,
    pub coupon_rate: Decimal,
    pub given_type: String,
    pub given_value: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settlement_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redemption_value: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub day_count: Option<SscmfiDayCount>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eom_rule: Option<SscmfiEomRule>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency: Option<SscmfiFrequency>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_schedule: Option<Vec<CallRedemption>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_schedule: Option<Vec<StepSchedule>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pik_rate: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cash_rate: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calc_analytics: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calc_cashflows: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SscmfiBondOutput {
    pub price: Money,
    pub yield_value: Rate,
    pub accrued_interest: Money,
    pub trading_price: Money,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub analytics: Option<SscmfiAnalytics>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cashflow_schedule: Option<Vec<SscmfiCashflow>>,
    pub redemption_info: SscmfiRedemptionInfo,
    pub conventions: SscmfiConventions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SscmfiAnalytics {
    pub macaulay_duration: Decimal,
    pub modified_duration: Decimal,
    pub convexity: Decimal,
    pub pv01: Money,
    pub yv32: Rate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SscmfiCashflow {
    pub period: u32,
    pub amount: Money,
    pub cashflow_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SscmfiRedemptionInfo {
    pub redemption_type: String,
    pub redemption_date: String,
    pub redemption_price: Money,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worst_yield: Option<Rate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SscmfiConventions {
    pub day_count: String,
    pub frequency: String,
    pub eom_rule: String,
    pub settlement_date: String,
}

// ---------------------------------------------------------------------------
// Internal working struct — resolved inputs
// ---------------------------------------------------------------------------

struct ResolvedInput {
    coupon_decimal: Decimal,
    yield_decimal: Decimal,
    price: Decimal,
    given_is_price: bool,
    redemption: Decimal,
    freq: u8,
    freq_dec: Decimal,
    n_periods: u32,
    years_to_maturity: Decimal,
    day_count: SscmfiDayCount,
    eom_rule: SscmfiEomRule,
    frequency: SscmfiFrequency,
    calc_analytics: bool,
    calc_cashflows: bool,
    settlement_date: String,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub fn calculate_sscmfi_bond(
    input: &SscmfiBondInput,
) -> CorpFinanceResult<ComputationOutput<SscmfiBondOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    validate_input(input)?;

    let resolved = resolve_input(input, &mut warnings)?;

    let output = match input.payment_type {
        SscmfiPaymentType::Periodic => calculate_periodic(input, &resolved, &mut warnings)?,
        SscmfiPaymentType::Discount => calculate_discount(input, &resolved, &mut warnings)?,
        SscmfiPaymentType::IAM => calculate_iam(input, &resolved, &mut warnings)?,
        SscmfiPaymentType::Stepped => calculate_stepped(input, &resolved, &mut warnings)?,
        SscmfiPaymentType::Multistep => calculate_multistep(input, &resolved, &mut warnings)?,
        SscmfiPaymentType::PIK => calculate_pik(input, &resolved, &mut warnings)?,
        SscmfiPaymentType::PartPIK => calculate_partpik(input, &resolved, &mut warnings)?,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    let assumptions = serde_json::json!({
        "payment_type": format!("{:?}", input.payment_type),
        "security_type": format!("{:?}", input.security_type),
        "day_count": format!("{:?}", resolved.day_count),
        "frequency": format!("{:?}", resolved.frequency),
        "yield_method": "Newton-Raphson",
        "precision": "rust_decimal_128bit",
        "par_basis": 100,
    });

    Ok(with_metadata(
        "SSCMFI Bond Math (native Rust, 128-bit decimal)",
        &assumptions,
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_input(input: &SscmfiBondInput) -> CorpFinanceResult<()> {
    if input.given_type != "Price" && input.given_type != "Yield" {
        return Err(CorpFinanceError::InvalidInput {
            field: "given_type".into(),
            reason: "Must be 'Price' or 'Yield'".into(),
        });
    }

    if input.given_type == "Price" && input.given_value <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "given_value".into(),
            reason: "Price must be positive".into(),
        });
    }

    if input.payment_type == SscmfiPaymentType::Stepped
        || input.payment_type == SscmfiPaymentType::Multistep
    {
        if input.step_schedule.is_none() || input.step_schedule.as_ref().unwrap().is_empty() {
            return Err(CorpFinanceError::InvalidInput {
                field: "step_schedule".into(),
                reason: "Stepped/Multistep payment type requires step_schedule".into(),
            });
        }
    }

    if input.payment_type == SscmfiPaymentType::PIK && input.pik_rate.is_none() {
        return Err(CorpFinanceError::InvalidInput {
            field: "pik_rate".into(),
            reason: "PIK payment type requires pik_rate".into(),
        });
    }

    if input.payment_type == SscmfiPaymentType::PartPIK {
        if input.pik_rate.is_none() || input.cash_rate.is_none() {
            return Err(CorpFinanceError::InvalidInput {
                field: "pik_rate/cash_rate".into(),
                reason: "PartPIK payment type requires both pik_rate and cash_rate".into(),
            });
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Input resolution — apply security type defaults
// ---------------------------------------------------------------------------

fn resolve_input(
    input: &SscmfiBondInput,
    warnings: &mut Vec<String>,
) -> CorpFinanceResult<ResolvedInput> {
    let (default_dc, default_freq) = match input.security_type {
        SscmfiSecurityType::Treasury => (SscmfiDayCount::ActualActual, SscmfiFrequency::Semiannual),
        SscmfiSecurityType::Agency => (SscmfiDayCount::SSCM30_360, SscmfiFrequency::Semiannual),
        SscmfiSecurityType::Corporate => (SscmfiDayCount::SSCM30_360, SscmfiFrequency::Semiannual),
        SscmfiSecurityType::Municipal => (SscmfiDayCount::SSCM30_360, SscmfiFrequency::Semiannual),
        SscmfiSecurityType::CD => (SscmfiDayCount::Actual360, SscmfiFrequency::Monthly),
    };

    let day_count = input.day_count.clone().unwrap_or(default_dc);
    let frequency = input.frequency.clone().unwrap_or(default_freq);
    let eom_rule = input.eom_rule.clone().unwrap_or_default();
    let redemption = input.redemption_value.unwrap_or(HUNDRED);
    let calc_analytics = input.calc_analytics.unwrap_or(true);
    let calc_cashflows = input.calc_cashflows.unwrap_or(false);

    let settlement_date = input
        .settlement_date
        .clone()
        .unwrap_or_else(|| "02/22/2026".to_string());

    // Convert coupon rate from percentage to decimal
    let coupon_decimal = input.coupon_rate / HUNDRED;

    let freq = frequency.to_u8();
    let freq_dec = Decimal::from(freq);

    // Estimate years to maturity from dates (simplified: parse year difference)
    let years_to_maturity = estimate_years(&settlement_date, &input.maturity_date)?;

    if years_to_maturity <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "maturity_date".into(),
            reason: "Maturity date must be after settlement date".into(),
        });
    }

    let n_periods = decimal_to_u32(years_to_maturity * freq_dec);
    if n_periods == 0 {
        warnings.push("Very short-dated bond: computing as single period".into());
    }
    let n_periods = n_periods.max(1);

    let given_is_price = input.given_type == "Price";
    let (price, yield_decimal) = if given_is_price {
        (input.given_value, Decimal::ZERO)
    } else {
        (Decimal::ZERO, input.given_value / HUNDRED)
    };

    Ok(ResolvedInput {
        coupon_decimal,
        yield_decimal,
        price,
        given_is_price,
        redemption,
        freq,
        freq_dec,
        n_periods,
        years_to_maturity,
        day_count,
        eom_rule,
        frequency,
        calc_analytics,
        calc_cashflows,
        settlement_date,
    })
}

// ---------------------------------------------------------------------------
// Payment Type Calculators
// ---------------------------------------------------------------------------

fn calculate_periodic(
    input: &SscmfiBondInput,
    r: &ResolvedInput,
    warnings: &mut Vec<String>,
) -> CorpFinanceResult<SscmfiBondOutput> {
    let periodic_coupon = r.redemption * r.coupon_decimal / r.freq_dec;

    let (clean_price, yield_pct) = if r.given_is_price {
        let y = solve_yield(periodic_coupon, r.redemption, r.price, r.n_periods, r.freq_dec, warnings)?;
        (r.price, y * r.freq_dec * HUNDRED)
    } else {
        let p = price_from_yield_periodic(periodic_coupon, r.redemption, r.yield_decimal / r.freq_dec, r.n_periods);
        (p, input.given_value)
    };

    let accrued = compute_accrued(r.coupon_decimal, r.redemption, &r.day_count, r.freq);
    let dirty_price = clean_price + accrued;

    // Yield-to-worst for callable bonds
    let yield_decimal = yield_pct / HUNDRED;
    let (redemption_info, worst_yield) = compute_callable_analytics(
        input, r, periodic_coupon, clean_price, yield_pct, warnings,
    )?;

    let analytics = if r.calc_analytics {
        Some(compute_analytics_from_yield(
            periodic_coupon, r.redemption, yield_decimal / r.freq_dec, r.n_periods, r.freq_dec, clean_price,
        ))
    } else {
        None
    };

    let cashflows = if r.calc_cashflows {
        Some(generate_cashflows(periodic_coupon, r.redemption, r.n_periods))
    } else {
        None
    };

    Ok(SscmfiBondOutput {
        price: clean_price,
        yield_value: worst_yield.unwrap_or(yield_pct),
        accrued_interest: accrued,
        trading_price: dirty_price,
        analytics,
        cashflow_schedule: cashflows,
        redemption_info,
        conventions: build_conventions(r),
    })
}

fn calculate_discount(
    input: &SscmfiBondInput,
    r: &ResolvedInput,
    warnings: &mut Vec<String>,
) -> CorpFinanceResult<SscmfiBondOutput> {
    // Zero coupon: Price = Redemption / (1 + y/freq)^n
    let (clean_price, yield_pct) = if r.given_is_price {
        // Solve: price = redemption / (1 + y)^t
        let y = solve_discount_yield(r.redemption, r.price, r.years_to_maturity, warnings)?;
        (r.price, y * HUNDRED)
    } else {
        let y_periodic = r.yield_decimal / r.freq_dec;
        let p = r.redemption / iterative_pow(Decimal::ONE + y_periodic, Decimal::from(r.n_periods));
        (p, input.given_value)
    };

    let accrued = Decimal::ZERO; // Zero coupon has no accrued

    let analytics = if r.calc_analytics {
        let yield_decimal = yield_pct / HUNDRED;
        // For zero coupon: Macaulay duration = years to maturity
        let mac_dur = r.years_to_maturity;
        let mod_dur = mac_dur / (Decimal::ONE + yield_decimal / r.freq_dec);
        let convexity = mac_dur * (mac_dur + Decimal::ONE / r.freq_dec)
            / iterative_pow(Decimal::ONE + yield_decimal / r.freq_dec, dec!(2));
        let pv01 = mod_dur * clean_price / dec!(10000);
        let yv32 = if mod_dur > Decimal::ZERO {
            Decimal::ONE / (dec!(32) * mod_dur * clean_price / HUNDRED)
        } else {
            Decimal::ZERO
        };
        Some(SscmfiAnalytics { macaulay_duration: mac_dur, modified_duration: mod_dur, convexity, pv01, yv32 })
    } else {
        None
    };

    Ok(SscmfiBondOutput {
        price: clean_price,
        yield_value: yield_pct,
        accrued_interest: accrued,
        trading_price: clean_price,
        analytics,
        cashflow_schedule: if r.calc_cashflows {
            Some(vec![SscmfiCashflow {
                period: r.n_periods,
                amount: r.redemption,
                cashflow_type: "Redemption".into(),
            }])
        } else {
            None
        },
        redemption_info: SscmfiRedemptionInfo {
            redemption_type: "Maturity".into(),
            redemption_date: input.maturity_date.clone(),
            redemption_price: r.redemption,
            worst_yield: None,
        },
        conventions: build_conventions(r),
    })
}

fn calculate_iam(
    input: &SscmfiBondInput,
    r: &ResolvedInput,
    warnings: &mut Vec<String>,
) -> CorpFinanceResult<SscmfiBondOutput> {
    // IAM: all interest paid at maturity
    // Total interest = face * coupon_rate * years
    let total_interest = r.redemption * r.coupon_decimal * r.years_to_maturity;
    let maturity_payment = r.redemption + total_interest;

    let (clean_price, yield_pct) = if r.given_is_price {
        let y = solve_discount_yield(maturity_payment, r.price, r.years_to_maturity, warnings)?;
        (r.price, y * HUNDRED)
    } else {
        let p = maturity_payment / iterative_pow(Decimal::ONE + r.yield_decimal, r.years_to_maturity);
        (p, input.given_value)
    };

    // Accrued interest for IAM: simple interest accrued from issue
    let accrued = r.redemption * r.coupon_decimal * estimate_accrued_fraction(r);

    let analytics = if r.calc_analytics {
        let yield_decimal = yield_pct / HUNDRED;
        let mac_dur = r.years_to_maturity;
        let mod_dur = mac_dur / (Decimal::ONE + yield_decimal);
        let convexity = mac_dur * (mac_dur + Decimal::ONE)
            / iterative_pow(Decimal::ONE + yield_decimal, dec!(2));
        let pv01 = mod_dur * clean_price / dec!(10000);
        let yv32 = if mod_dur > Decimal::ZERO {
            Decimal::ONE / (dec!(32) * mod_dur * clean_price / HUNDRED)
        } else {
            Decimal::ZERO
        };
        Some(SscmfiAnalytics { macaulay_duration: mac_dur, modified_duration: mod_dur, convexity, pv01, yv32 })
    } else {
        None
    };

    Ok(SscmfiBondOutput {
        price: clean_price,
        yield_value: yield_pct,
        accrued_interest: accrued,
        trading_price: clean_price + accrued,
        analytics,
        cashflow_schedule: if r.calc_cashflows {
            Some(vec![SscmfiCashflow {
                period: 1,
                amount: maturity_payment,
                cashflow_type: "Interest + Redemption".into(),
            }])
        } else {
            None
        },
        redemption_info: SscmfiRedemptionInfo {
            redemption_type: "Maturity".into(),
            redemption_date: input.maturity_date.clone(),
            redemption_price: r.redemption,
            worst_yield: None,
        },
        conventions: build_conventions(r),
    })
}

fn calculate_stepped(
    input: &SscmfiBondInput,
    r: &ResolvedInput,
    warnings: &mut Vec<String>,
) -> CorpFinanceResult<SscmfiBondOutput> {
    let schedule = input.step_schedule.as_ref().unwrap();
    let step = &schedule[0];
    let step_rate = step.coupon_rate / HUNDRED;

    // Estimate periods before and after step
    let step_years = estimate_years(&r.settlement_date, &step.date).unwrap_or(r.years_to_maturity / dec!(2));
    let periods_before = (step_years * r.freq_dec).round().to_string().parse::<u32>().unwrap_or(r.n_periods / 2).max(1);
    let periods_after = r.n_periods.saturating_sub(periods_before).max(1);

    let coupon_before = r.redemption * r.coupon_decimal / r.freq_dec;
    let coupon_after = r.redemption * step_rate / r.freq_dec;

    let (clean_price, yield_pct) = if r.given_is_price {
        let y = solve_stepped_yield(
            coupon_before, coupon_after, r.redemption, r.price,
            periods_before, periods_after, r.freq_dec, warnings,
        )?;
        (r.price, y * r.freq_dec * HUNDRED)
    } else {
        let y_periodic = r.yield_decimal / r.freq_dec;
        let p = price_stepped(coupon_before, coupon_after, r.redemption, y_periodic, periods_before, periods_after);
        (p, input.given_value)
    };

    let accrued = compute_accrued(r.coupon_decimal, r.redemption, &r.day_count, r.freq);

    let analytics = if r.calc_analytics {
        let yield_decimal = yield_pct / HUNDRED;
        Some(compute_analytics_stepped(
            coupon_before, coupon_after, r.redemption,
            yield_decimal / r.freq_dec, periods_before, periods_after,
            r.freq_dec, clean_price,
        ))
    } else {
        None
    };

    Ok(SscmfiBondOutput {
        price: clean_price,
        yield_value: yield_pct,
        accrued_interest: accrued,
        trading_price: clean_price + accrued,
        analytics,
        cashflow_schedule: None,
        redemption_info: SscmfiRedemptionInfo {
            redemption_type: "Maturity".into(),
            redemption_date: input.maturity_date.clone(),
            redemption_price: r.redemption,
            worst_yield: None,
        },
        conventions: build_conventions(r),
    })
}

fn calculate_multistep(
    input: &SscmfiBondInput,
    r: &ResolvedInput,
    warnings: &mut Vec<String>,
) -> CorpFinanceResult<SscmfiBondOutput> {
    // For multistep, build segments from step schedule
    let schedule = input.step_schedule.as_ref().unwrap();
    let mut segments: Vec<(u32, Decimal)> = Vec::new();

    let mut prev_date = r.settlement_date.clone();
    let mut prev_rate = r.coupon_decimal;
    let mut total_periods = 0u32;

    for step in schedule {
        let seg_years = estimate_years(&prev_date, &step.date).unwrap_or(dec!(1));
        let seg_periods = (seg_years * r.freq_dec).round().to_string().parse::<u32>().unwrap_or(1).max(1);
        let coupon = r.redemption * prev_rate / r.freq_dec;
        segments.push((seg_periods, coupon));
        total_periods += seg_periods;
        prev_rate = step.coupon_rate / HUNDRED;
        prev_date = step.date.clone();
    }

    // Final segment to maturity
    let remaining = if total_periods < r.n_periods { r.n_periods - total_periods } else { 1 };
    let final_coupon = r.redemption * prev_rate / r.freq_dec;
    segments.push((remaining, final_coupon));

    let (clean_price, yield_pct) = if r.given_is_price {
        let y = solve_multistep_yield(&segments, r.redemption, r.price, r.freq_dec, warnings)?;
        (r.price, y * r.freq_dec * HUNDRED)
    } else {
        let y_periodic = r.yield_decimal / r.freq_dec;
        let p = price_multistep(&segments, r.redemption, y_periodic);
        (p, input.given_value)
    };

    let accrued = compute_accrued(r.coupon_decimal, r.redemption, &r.day_count, r.freq);

    Ok(SscmfiBondOutput {
        price: clean_price,
        yield_value: yield_pct,
        accrued_interest: accrued,
        trading_price: clean_price + accrued,
        analytics: None,
        cashflow_schedule: None,
        redemption_info: SscmfiRedemptionInfo {
            redemption_type: "Maturity".into(),
            redemption_date: input.maturity_date.clone(),
            redemption_price: r.redemption,
            worst_yield: None,
        },
        conventions: build_conventions(r),
    })
}

fn calculate_pik(
    input: &SscmfiBondInput,
    r: &ResolvedInput,
    warnings: &mut Vec<String>,
) -> CorpFinanceResult<SscmfiBondOutput> {
    // PIK: interest accrues to principal each period
    let pik_decimal = input.pik_rate.unwrap_or(input.coupon_rate) / HUNDRED;
    let periodic_pik = pik_decimal / r.freq_dec;

    // Accreted face at maturity = redemption * (1 + pik_rate/freq)^n
    let accreted_face = r.redemption * iterative_pow(Decimal::ONE + periodic_pik, Decimal::from(r.n_periods));

    let (clean_price, yield_pct) = if r.given_is_price {
        let y = solve_discount_yield(accreted_face, r.price, r.years_to_maturity, warnings)?;
        (r.price, y * HUNDRED)
    } else {
        let p = accreted_face / iterative_pow(Decimal::ONE + r.yield_decimal, r.years_to_maturity);
        (p, input.given_value)
    };

    let analytics = if r.calc_analytics {
        let yield_decimal = yield_pct / HUNDRED;
        let mac_dur = r.years_to_maturity;
        let mod_dur = mac_dur / (Decimal::ONE + yield_decimal / r.freq_dec);
        let convexity = mac_dur * (mac_dur + Decimal::ONE / r.freq_dec)
            / iterative_pow(Decimal::ONE + yield_decimal / r.freq_dec, dec!(2));
        let pv01 = mod_dur * clean_price / dec!(10000);
        let yv32 = if mod_dur > Decimal::ZERO {
            Decimal::ONE / (dec!(32) * mod_dur * clean_price / HUNDRED)
        } else {
            Decimal::ZERO
        };
        Some(SscmfiAnalytics { macaulay_duration: mac_dur, modified_duration: mod_dur, convexity, pv01, yv32 })
    } else {
        None
    };

    Ok(SscmfiBondOutput {
        price: clean_price,
        yield_value: yield_pct,
        accrued_interest: Decimal::ZERO,
        trading_price: clean_price,
        analytics,
        cashflow_schedule: if r.calc_cashflows {
            Some(vec![SscmfiCashflow {
                period: r.n_periods,
                amount: accreted_face,
                cashflow_type: "Accreted Redemption (PIK)".into(),
            }])
        } else {
            None
        },
        redemption_info: SscmfiRedemptionInfo {
            redemption_type: "Maturity".into(),
            redemption_date: input.maturity_date.clone(),
            redemption_price: accreted_face,
            worst_yield: None,
        },
        conventions: build_conventions(r),
    })
}

fn calculate_partpik(
    input: &SscmfiBondInput,
    r: &ResolvedInput,
    warnings: &mut Vec<String>,
) -> CorpFinanceResult<SscmfiBondOutput> {
    // PartPIK: cash coupon paid periodically, PIK accretes to principal
    let cash_decimal = input.cash_rate.unwrap() / HUNDRED;
    let pik_decimal = input.pik_rate.unwrap() / HUNDRED;
    let periodic_cash = r.redemption * cash_decimal / r.freq_dec;
    let periodic_pik = pik_decimal / r.freq_dec;

    // Accreted face = redemption * (1 + pik/freq)^n
    let accreted_face = r.redemption * iterative_pow(Decimal::ONE + periodic_pik, Decimal::from(r.n_periods));

    let (clean_price, yield_pct) = if r.given_is_price {
        let y = solve_partpik_yield(
            periodic_cash, periodic_pik, r.redemption, r.price,
            r.n_periods, r.freq_dec, warnings,
        )?;
        (r.price, y * r.freq_dec * HUNDRED)
    } else {
        let y_periodic = r.yield_decimal / r.freq_dec;
        let p = price_partpik(periodic_cash, periodic_pik, r.redemption, y_periodic, r.n_periods);
        (p, input.given_value)
    };

    let accrued = compute_accrued(cash_decimal, r.redemption, &r.day_count, r.freq);

    Ok(SscmfiBondOutput {
        price: clean_price,
        yield_value: yield_pct,
        accrued_interest: accrued,
        trading_price: clean_price + accrued,
        analytics: None,
        cashflow_schedule: None,
        redemption_info: SscmfiRedemptionInfo {
            redemption_type: "Maturity".into(),
            redemption_date: input.maturity_date.clone(),
            redemption_price: accreted_face,
            worst_yield: None,
        },
        conventions: build_conventions(r),
    })
}

// ---------------------------------------------------------------------------
// Pricing helpers
// ---------------------------------------------------------------------------

fn price_from_yield_periodic(
    coupon: Decimal,
    redemption: Decimal,
    y_periodic: Decimal,
    n: u32,
) -> Decimal {
    if n == 0 {
        return redemption + coupon;
    }
    let one_plus_y = Decimal::ONE + y_periodic;
    let mut pv_coupons = Decimal::ZERO;
    let mut discount = Decimal::ONE;

    for _ in 1..=n {
        discount *= one_plus_y;
        if discount.is_zero() {
            break;
        }
        pv_coupons += coupon / discount;
    }

    pv_coupons + redemption / discount
}

fn price_stepped(
    coupon_before: Decimal,
    coupon_after: Decimal,
    redemption: Decimal,
    y_periodic: Decimal,
    periods_before: u32,
    periods_after: u32,
) -> Decimal {
    let one_plus_y = Decimal::ONE + y_periodic;
    let mut pv = Decimal::ZERO;
    let mut discount = Decimal::ONE;

    for _ in 1..=periods_before {
        discount *= one_plus_y;
        pv += coupon_before / discount;
    }
    for _ in 1..=periods_after {
        discount *= one_plus_y;
        pv += coupon_after / discount;
    }
    pv + redemption / discount
}

fn price_multistep(
    segments: &[(u32, Decimal)],
    redemption: Decimal,
    y_periodic: Decimal,
) -> Decimal {
    let one_plus_y = Decimal::ONE + y_periodic;
    let mut pv = Decimal::ZERO;
    let mut discount = Decimal::ONE;

    for (periods, coupon) in segments {
        for _ in 0..*periods {
            discount *= one_plus_y;
            pv += coupon / discount;
        }
    }
    pv + redemption / discount
}

fn price_partpik(
    cash_coupon: Decimal,
    periodic_pik: Decimal,
    redemption: Decimal,
    y_periodic: Decimal,
    n: u32,
) -> Decimal {
    let one_plus_y = Decimal::ONE + y_periodic;
    let mut pv = Decimal::ZERO;
    let mut discount = Decimal::ONE;
    let mut accreted = redemption;

    for _ in 1..=n {
        discount *= one_plus_y;
        pv += cash_coupon / discount;
        accreted *= Decimal::ONE + periodic_pik;
    }
    pv + accreted / discount
}

// ---------------------------------------------------------------------------
// Yield solvers
// ---------------------------------------------------------------------------

fn solve_yield(
    coupon: Decimal,
    redemption: Decimal,
    target_price: Decimal,
    n: u32,
    _freq: Decimal,
    warnings: &mut Vec<String>,
) -> CorpFinanceResult<Decimal> {
    // Initial guess from current yield
    let mut y = if target_price > Decimal::ZERO {
        coupon / target_price
    } else {
        dec!(0.025)
    };

    for _iteration in 0..MAX_ITERATIONS {
        let one_plus_y = Decimal::ONE + y;
        if one_plus_y <= Decimal::ZERO {
            y = dec!(0.001);
            continue;
        }

        let mut pv = Decimal::ZERO;
        let mut dpv = Decimal::ZERO;
        let mut discount = Decimal::ONE;

        for i in 1..=n {
            discount *= one_plus_y;
            if discount.is_zero() {
                break;
            }
            let i_dec = Decimal::from(i);
            pv += coupon / discount;
            dpv += i_dec * coupon / (discount * one_plus_y);
        }

        pv += redemption / discount;
        dpv += Decimal::from(n) * redemption / (discount * one_plus_y);

        let f = target_price - pv;

        if f.abs() < EPSILON {
            return Ok(y);
        }

        if dpv.is_zero() {
            warnings.push("Yield solver: derivative is zero".into());
            break;
        }

        y -= f / dpv;

        if y < dec!(-0.5) {
            y = dec!(-0.5);
        } else if y > dec!(5.0) {
            y = dec!(5.0);
        }
    }

    // Check relaxed convergence
    let pv_check = price_from_yield_periodic(coupon, redemption, y, n);
    let residual = (target_price - pv_check).abs();
    if residual < dec!(0.01) {
        warnings.push(format!("Yield converged with relaxed tolerance (residual: {residual})"));
        return Ok(y);
    }

    Err(CorpFinanceError::ConvergenceFailure {
        function: "SSCMFI yield solver".into(),
        iterations: MAX_ITERATIONS,
        last_delta: residual,
    })
}

fn solve_discount_yield(
    maturity_value: Decimal,
    price: Decimal,
    years: Decimal,
    warnings: &mut Vec<String>,
) -> CorpFinanceResult<Decimal> {
    // For discount/IAM/PIK: Price = FV / (1+y)^t
    // y = (FV/P)^(1/t) - 1
    if price <= Decimal::ZERO || years <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "price/years".into(),
            reason: "Price and years must be positive".into(),
        });
    }

    let ratio = maturity_value / price;

    // Newton-Raphson: find y such that (1+y)^t = ratio
    let mut y = (ratio - Decimal::ONE) / years; // linear approximation

    for _ in 0..MAX_ITERATIONS {
        let base = Decimal::ONE + y;
        if base <= Decimal::ZERO {
            y = dec!(0.001);
            continue;
        }

        let f = iterative_pow_fractional(base, years) - ratio;
        let df = years * iterative_pow_fractional(base, years - Decimal::ONE);

        if f.abs() < EPSILON {
            return Ok(y);
        }
        if df.is_zero() {
            break;
        }

        y -= f / df;

        if y < dec!(-0.5) {
            y = dec!(-0.5);
        } else if y > dec!(5.0) {
            y = dec!(5.0);
        }
    }

    warnings.push("Discount yield solver used relaxed convergence".into());
    Ok(y)
}

fn solve_stepped_yield(
    coupon_before: Decimal,
    coupon_after: Decimal,
    redemption: Decimal,
    target_price: Decimal,
    periods_before: u32,
    periods_after: u32,
    _freq: Decimal,
    warnings: &mut Vec<String>,
) -> CorpFinanceResult<Decimal> {
    let mut y = dec!(0.025);

    for _ in 0..MAX_ITERATIONS {
        let p = price_stepped(coupon_before, coupon_after, redemption, y, periods_before, periods_after);
        let p_up = price_stepped(coupon_before, coupon_after, redemption, y + EPSILON, periods_before, periods_after);

        let f = target_price - p;
        let df = -(p_up - p) / EPSILON;

        if f.abs() < EPSILON {
            return Ok(y);
        }
        if df.abs() < dec!(0.00000001) {
            break;
        }

        y += f / df;
        if y < dec!(-0.5) { y = dec!(-0.5); }
        if y > dec!(5.0) { y = dec!(5.0); }
    }

    warnings.push("Stepped yield solver used relaxed convergence".into());
    Ok(y)
}

fn solve_multistep_yield(
    segments: &[(u32, Decimal)],
    redemption: Decimal,
    target_price: Decimal,
    _freq: Decimal,
    warnings: &mut Vec<String>,
) -> CorpFinanceResult<Decimal> {
    let mut y = dec!(0.025);

    for _ in 0..MAX_ITERATIONS {
        let p = price_multistep(segments, redemption, y);
        let p_up = price_multistep(segments, redemption, y + EPSILON);

        let f = target_price - p;
        let df = -(p_up - p) / EPSILON;

        if f.abs() < EPSILON {
            return Ok(y);
        }
        if df.abs() < dec!(0.00000001) {
            break;
        }

        y += f / df;
        if y < dec!(-0.5) { y = dec!(-0.5); }
        if y > dec!(5.0) { y = dec!(5.0); }
    }

    warnings.push("Multistep yield solver used relaxed convergence".into());
    Ok(y)
}

fn solve_partpik_yield(
    cash_coupon: Decimal,
    periodic_pik: Decimal,
    redemption: Decimal,
    target_price: Decimal,
    n: u32,
    _freq: Decimal,
    warnings: &mut Vec<String>,
) -> CorpFinanceResult<Decimal> {
    let mut y = dec!(0.04);

    for _ in 0..MAX_ITERATIONS {
        let p = price_partpik(cash_coupon, periodic_pik, redemption, y, n);
        let p_up = price_partpik(cash_coupon, periodic_pik, redemption, y + EPSILON, n);

        let f = target_price - p;
        let df = -(p_up - p) / EPSILON;

        if f.abs() < EPSILON {
            return Ok(y);
        }
        if df.abs() < dec!(0.00000001) {
            break;
        }

        y += f / df;
        if y < dec!(-0.5) { y = dec!(-0.5); }
        if y > dec!(5.0) { y = dec!(5.0); }
    }

    warnings.push("PartPIK yield solver used relaxed convergence".into());
    Ok(y)
}

// ---------------------------------------------------------------------------
// Callable bond analytics
// ---------------------------------------------------------------------------

fn compute_callable_analytics(
    input: &SscmfiBondInput,
    r: &ResolvedInput,
    coupon: Decimal,
    clean_price: Decimal,
    yield_to_maturity_pct: Decimal,
    warnings: &mut Vec<String>,
) -> CorpFinanceResult<(SscmfiRedemptionInfo, Option<Decimal>)> {
    let call_schedule = match &input.call_schedule {
        Some(cs) if !cs.is_empty() => cs,
        _ => {
            return Ok((
                SscmfiRedemptionInfo {
                    redemption_type: "Maturity".into(),
                    redemption_date: input.maturity_date.clone(),
                    redemption_price: r.redemption,
                    worst_yield: None,
                },
                None,
            ))
        }
    };

    let mut worst_yield = yield_to_maturity_pct;
    let mut worst_date = input.maturity_date.clone();
    let mut worst_price = r.redemption;
    let mut worst_type = "Maturity".to_string();

    for call in call_schedule {
        let call_years = estimate_years(&r.settlement_date, &call.date).unwrap_or(Decimal::ZERO);
        if call_years <= Decimal::ZERO {
            continue;
        }
        let call_periods = (call_years * r.freq_dec).round().to_string().parse::<u32>().unwrap_or(0);
        if call_periods == 0 {
            continue;
        }

        // Solve YTC: find y such that price = PV(coupons to call) + PV(call_price)
        match solve_yield(coupon, call.price, clean_price, call_periods, r.freq_dec, warnings) {
            Ok(y_periodic) => {
                let ytc = y_periodic * r.freq_dec * HUNDRED;
                if ytc < worst_yield {
                    worst_yield = ytc;
                    worst_date = call.date.clone();
                    worst_price = call.price;
                    worst_type = "Call".to_string();
                }
            }
            Err(_) => {
                warnings.push(format!("Could not solve yield-to-call for date {}", call.date));
            }
        }
    }

    Ok((
        SscmfiRedemptionInfo {
            redemption_type: worst_type,
            redemption_date: worst_date,
            redemption_price: worst_price,
            worst_yield: Some(worst_yield),
        },
        Some(worst_yield),
    ))
}

// ---------------------------------------------------------------------------
// Analytics computation
// ---------------------------------------------------------------------------

fn compute_analytics_from_yield(
    coupon: Decimal,
    redemption: Decimal,
    y_periodic: Decimal,
    n: u32,
    freq: Decimal,
    clean_price: Decimal,
) -> SscmfiAnalytics {
    let one_plus_y = Decimal::ONE + y_periodic;
    if one_plus_y <= Decimal::ZERO || clean_price <= Decimal::ZERO {
        return SscmfiAnalytics {
            macaulay_duration: Decimal::ZERO,
            modified_duration: Decimal::ZERO,
            convexity: Decimal::ZERO,
            pv01: Decimal::ZERO,
            yv32: Decimal::ZERO,
        };
    }

    let mut mac_num = Decimal::ZERO;
    let mut conv_num = Decimal::ZERO;
    let mut price_sum = Decimal::ZERO;
    let mut discount = Decimal::ONE;

    for i in 1..=n {
        discount *= one_plus_y;
        if discount.is_zero() {
            break;
        }
        let i_dec = Decimal::from(i);
        let t = i_dec / freq;
        let cf = if i == n { coupon + redemption } else { coupon };
        let pv_cf = cf / discount;

        price_sum += pv_cf;
        mac_num += t * pv_cf;
        conv_num += (t * t + t / freq) * pv_cf;
    }

    let mac_dur = if price_sum > Decimal::ZERO {
        mac_num / price_sum
    } else {
        Decimal::ZERO
    };

    let mod_dur = mac_dur / one_plus_y;

    let convexity = if price_sum > Decimal::ZERO {
        conv_num / (price_sum * one_plus_y * one_plus_y)
    } else {
        Decimal::ZERO
    };

    // PV01: price change per 1 basis point yield change
    let pv01 = mod_dur * clean_price / dec!(10000);

    // YV32: yield change per 1/32nd price change
    let yv32 = if mod_dur * clean_price > Decimal::ZERO {
        (Decimal::ONE / dec!(32)) / (mod_dur * clean_price / HUNDRED)
    } else {
        Decimal::ZERO
    };

    SscmfiAnalytics {
        macaulay_duration: mac_dur,
        modified_duration: mod_dur,
        convexity,
        pv01,
        yv32,
    }
}

fn compute_analytics_stepped(
    coupon_before: Decimal,
    coupon_after: Decimal,
    redemption: Decimal,
    y_periodic: Decimal,
    periods_before: u32,
    periods_after: u32,
    freq: Decimal,
    clean_price: Decimal,
) -> SscmfiAnalytics {
    let one_plus_y = Decimal::ONE + y_periodic;
    let total = periods_before + periods_after;
    let mut mac_num = Decimal::ZERO;
    let mut price_sum = Decimal::ZERO;
    let mut discount = Decimal::ONE;

    for i in 1..=total {
        discount *= one_plus_y;
        let i_dec = Decimal::from(i);
        let t = i_dec / freq;
        let c = if i <= periods_before { coupon_before } else { coupon_after };
        let cf = if i == total { c + redemption } else { c };
        let pv_cf = cf / discount;
        price_sum += pv_cf;
        mac_num += t * pv_cf;
    }

    let mac_dur = if price_sum > Decimal::ZERO { mac_num / price_sum } else { Decimal::ZERO };
    let mod_dur = mac_dur / one_plus_y;
    let pv01 = mod_dur * clean_price / dec!(10000);
    let yv32 = if mod_dur * clean_price > Decimal::ZERO {
        (Decimal::ONE / dec!(32)) / (mod_dur * clean_price / HUNDRED)
    } else {
        Decimal::ZERO
    };

    SscmfiAnalytics {
        macaulay_duration: mac_dur,
        modified_duration: mod_dur,
        convexity: Decimal::ZERO, // simplified for stepped
        pv01,
        yv32,
    }
}

// ---------------------------------------------------------------------------
// Accrued interest
// ---------------------------------------------------------------------------

fn compute_accrued(
    coupon_decimal: Decimal,
    face: Decimal,
    _day_count: &SscmfiDayCount,
    freq: u8,
) -> Decimal {
    // Simplified: assume mid-period accrual (0.5 of periodic coupon)
    // In production, this would parse actual settlement/coupon dates
    let annual_coupon = face * coupon_decimal;
    let periodic_coupon = annual_coupon / Decimal::from(freq);
    let accrual_fraction = dec!(0.5); // mid-period default
    periodic_coupon * accrual_fraction
}

fn estimate_accrued_fraction(_r: &ResolvedInput) -> Decimal {
    // For IAM: fraction of a year from issue to settlement
    // Simplified to half a year
    dec!(0.5)
}

// ---------------------------------------------------------------------------
// Cashflow generation
// ---------------------------------------------------------------------------

fn generate_cashflows(
    coupon: Decimal,
    redemption: Decimal,
    n: u32,
) -> Vec<SscmfiCashflow> {
    let mut cfs = Vec::with_capacity(n as usize);
    for i in 1..=n {
        if i < n {
            cfs.push(SscmfiCashflow {
                period: i,
                amount: coupon,
                cashflow_type: "Coupon".into(),
            });
        } else {
            cfs.push(SscmfiCashflow {
                period: i,
                amount: coupon + redemption,
                cashflow_type: "Coupon + Redemption".into(),
            });
        }
    }
    cfs
}

// ---------------------------------------------------------------------------
// Helpers — conventions builder
// ---------------------------------------------------------------------------

fn build_conventions(r: &ResolvedInput) -> SscmfiConventions {
    SscmfiConventions {
        day_count: format!("{:?}", r.day_count),
        frequency: format!("{:?}", r.frequency),
        eom_rule: format!("{:?}", r.eom_rule),
        settlement_date: r.settlement_date.clone(),
    }
}

// ---------------------------------------------------------------------------
// Helpers — date parsing (simplified)
// ---------------------------------------------------------------------------

fn estimate_years(from_date: &str, to_date: &str) -> CorpFinanceResult<Decimal> {
    let from = parse_date_parts(from_date)?;
    let to = parse_date_parts(to_date)?;

    // Calculate approximate years
    let from_days = from.0 * dec!(365.25) + from.1 * dec!(30.4375) + from.2;
    let to_days = to.0 * dec!(365.25) + to.1 * dec!(30.4375) + to.2;
    let diff_days = to_days - from_days;
    Ok(diff_days / dec!(365.25))
}

fn parse_date_parts(date_str: &str) -> CorpFinanceResult<(Decimal, Decimal, Decimal)> {
    // Parse MM/DD/YYYY
    let parts: Vec<&str> = date_str.split('/').collect();
    if parts.len() != 3 {
        return Err(CorpFinanceError::InvalidInput {
            field: "date".into(),
            reason: format!("Date must be in MM/DD/YYYY format, got '{date_str}'"),
        });
    }

    let month: u32 = parts[0].parse().map_err(|_| CorpFinanceError::InvalidInput {
        field: "date".into(),
        reason: format!("Invalid month in date '{date_str}'"),
    })?;
    let day: u32 = parts[1].parse().map_err(|_| CorpFinanceError::InvalidInput {
        field: "date".into(),
        reason: format!("Invalid day in date '{date_str}'"),
    })?;
    let year: u32 = parts[2].parse().map_err(|_| CorpFinanceError::InvalidInput {
        field: "date".into(),
        reason: format!("Invalid year in date '{date_str}'"),
    })?;

    if month == 0 || month > 12 || day == 0 || day > 31 {
        return Err(CorpFinanceError::InvalidInput {
            field: "date".into(),
            reason: format!("Invalid date values in '{date_str}'"),
        });
    }

    Ok((
        Decimal::from(year),
        Decimal::from(month),
        Decimal::from(day),
    ))
}

// ---------------------------------------------------------------------------
// Mathematical helpers (same patterns as yields.rs — no powd)
// ---------------------------------------------------------------------------

fn iterative_pow(base: Decimal, exponent: Decimal) -> Decimal {
    if exponent.is_zero() {
        return Decimal::ONE;
    }
    if base.is_zero() {
        return Decimal::ZERO;
    }
    if base == Decimal::ONE {
        return Decimal::ONE;
    }

    let n = decimal_to_u32(exponent);
    let frac = exponent - Decimal::from(n);

    let mut result = Decimal::ONE;
    for _ in 0..n {
        result *= base;
    }

    if frac > Decimal::ZERO {
        let ln_base = ln_decimal(base);
        let frac_pow = exp_decimal(frac * ln_base);
        result *= frac_pow;
    }

    result
}

fn iterative_pow_fractional(base: Decimal, exponent: Decimal) -> Decimal {
    if exponent.is_zero() {
        return Decimal::ONE;
    }
    if base.is_zero() {
        return Decimal::ZERO;
    }
    if base == Decimal::ONE {
        return Decimal::ONE;
    }
    if base <= Decimal::ZERO {
        return Decimal::ZERO;
    }

    let ln_base = ln_decimal(base);
    exp_decimal(exponent * ln_base)
}

fn exp_decimal(x: Decimal) -> Decimal {
    let mut result = Decimal::ONE;
    let mut term = Decimal::ONE;

    for k in 1..EXP_TAYLOR_TERMS {
        term *= x / Decimal::from(k as u32);
        result += term;
    }

    if result < Decimal::ZERO {
        Decimal::ZERO
    } else {
        result
    }
}

fn ln_decimal(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    if x == Decimal::ONE {
        return Decimal::ZERO;
    }

    let ln2 = dec!(0.6931471805599453);
    let mut val = x;
    let mut k: i32 = 0;

    while val > dec!(2.0) {
        val /= dec!(2);
        k += 1;
    }
    while val < dec!(0.5) {
        val *= dec!(2);
        k -= 1;
    }

    let u = (val - Decimal::ONE) / (val + Decimal::ONE);
    let u_sq = u * u;
    let mut term = u;
    let mut result = u;

    for n in 1..20u32 {
        term *= u_sq;
        let coeff = Decimal::ONE / Decimal::from(2 * n + 1);
        result += coeff * term;
    }
    result *= dec!(2);

    result + Decimal::from(k) * ln2
}

fn decimal_to_u32(d: Decimal) -> u32 {
    let rounded = d.round();
    if rounded < Decimal::ZERO {
        0
    } else {
        rounded.to_string().parse::<u32>().unwrap_or(0)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn treasury_bond_yield_input() -> SscmfiBondInput {
        SscmfiBondInput {
            security_type: SscmfiSecurityType::Treasury,
            payment_type: SscmfiPaymentType::Periodic,
            maturity_date: "06/15/2036".to_string(),
            coupon_rate: dec!(5.0),
            given_type: "Yield".to_string(),
            given_value: dec!(5.0),
            settlement_date: Some("02/22/2026".to_string()),
            redemption_value: None,
            day_count: None,
            eom_rule: None,
            frequency: None,
            call_schedule: None,
            step_schedule: None,
            pik_rate: None,
            cash_rate: None,
            calc_analytics: Some(true),
            calc_cashflows: Some(false),
        }
    }

    fn corporate_bond_input() -> SscmfiBondInput {
        SscmfiBondInput {
            security_type: SscmfiSecurityType::Corporate,
            payment_type: SscmfiPaymentType::Periodic,
            maturity_date: "03/01/2030".to_string(),
            coupon_rate: dec!(6.0),
            given_type: "Yield".to_string(),
            given_value: dec!(6.5),
            settlement_date: Some("02/22/2026".to_string()),
            redemption_value: None,
            day_count: None,
            eom_rule: None,
            frequency: None,
            call_schedule: None,
            step_schedule: None,
            pik_rate: None,
            cash_rate: None,
            calc_analytics: Some(true),
            calc_cashflows: Some(false),
        }
    }

    // -- Test 1: Treasury par bond (price ~100 when coupon == yield)
    #[test]
    fn test_treasury_par_bond() {
        let input = treasury_bond_yield_input();
        let result = calculate_sscmfi_bond(&input).unwrap();
        let out = &result.result;

        let diff = (out.price - HUNDRED).abs();
        assert!(
            diff < dec!(2.0),
            "Treasury par bond price should be near 100, got {}",
            out.price
        );
    }

    // -- Test 2: Corporate bond price-to-yield
    #[test]
    fn test_corporate_price_to_yield() {
        // First get price from yield
        let input = corporate_bond_input();
        let result = calculate_sscmfi_bond(&input).unwrap();
        let price = result.result.price;

        // Now solve yield from that price
        let mut reverse = corporate_bond_input();
        reverse.given_type = "Price".to_string();
        reverse.given_value = price;

        let result2 = calculate_sscmfi_bond(&reverse).unwrap();
        let yield_back = result2.result.yield_value;

        let diff = (yield_back - dec!(6.5)).abs();
        assert!(
            diff < dec!(0.1),
            "Round-trip yield should be ~6.5%, got {}",
            yield_back
        );
    }

    // -- Test 3: Corporate bond yield-to-price
    #[test]
    fn test_corporate_yield_to_price() {
        let input = corporate_bond_input();
        let result = calculate_sscmfi_bond(&input).unwrap();
        let out = &result.result;

        // When yield > coupon, price should be < 100 (discount)
        assert!(
            out.price < HUNDRED,
            "Discount bond (yield > coupon) price should be < 100, got {}",
            out.price
        );
    }

    // -- Test 4: Callable municipal bond yield-to-worst
    #[test]
    fn test_callable_municipal_ytw() {
        let input = SscmfiBondInput {
            security_type: SscmfiSecurityType::Municipal,
            payment_type: SscmfiPaymentType::Periodic,
            maturity_date: "01/01/2035".to_string(),
            coupon_rate: dec!(4.0),
            given_type: "Price".to_string(),
            given_value: dec!(102.5),
            settlement_date: Some("02/22/2026".to_string()),
            redemption_value: None,
            day_count: None,
            eom_rule: None,
            frequency: None,
            call_schedule: Some(vec![
                CallRedemption { date: "01/01/2028".to_string(), price: dec!(102) },
                CallRedemption { date: "01/01/2030".to_string(), price: dec!(101) },
                CallRedemption { date: "01/01/2032".to_string(), price: dec!(100) },
            ]),
            step_schedule: None,
            pik_rate: None,
            cash_rate: None,
            calc_analytics: Some(true),
            calc_cashflows: Some(false),
        };

        let result = calculate_sscmfi_bond(&input).unwrap();
        let out = &result.result;

        // Yield-to-worst should exist for callable bonds
        assert!(
            out.redemption_info.worst_yield.is_some(),
            "Callable bond should have worst_yield"
        );
    }

    // -- Test 5: Zero coupon discount security
    #[test]
    fn test_zero_coupon_discount() {
        let input = SscmfiBondInput {
            security_type: SscmfiSecurityType::Treasury,
            payment_type: SscmfiPaymentType::Discount,
            maturity_date: "03/15/2028".to_string(),
            coupon_rate: dec!(0),
            given_type: "Yield".to_string(),
            given_value: dec!(4.75),
            settlement_date: Some("02/22/2026".to_string()),
            redemption_value: None,
            day_count: None,
            eom_rule: None,
            frequency: None,
            call_schedule: None,
            step_schedule: None,
            pik_rate: None,
            cash_rate: None,
            calc_analytics: Some(true),
            calc_cashflows: Some(true),
        };

        let result = calculate_sscmfi_bond(&input).unwrap();
        let out = &result.result;

        assert!(out.price > Decimal::ZERO, "Discount price must be positive");
        assert!(out.price < HUNDRED, "Discount price must be < par");
        assert_eq!(out.accrued_interest, Decimal::ZERO, "Zero coupon has no accrued");
    }

    // -- Test 6: IAM bond pricing
    #[test]
    fn test_iam_bond() {
        let input = SscmfiBondInput {
            security_type: SscmfiSecurityType::CD,
            payment_type: SscmfiPaymentType::IAM,
            maturity_date: "02/22/2027".to_string(),
            coupon_rate: dec!(5.0),
            given_type: "Yield".to_string(),
            given_value: dec!(5.5),
            settlement_date: Some("02/22/2026".to_string()),
            redemption_value: None,
            day_count: None,
            eom_rule: None,
            frequency: None,
            call_schedule: None,
            step_schedule: None,
            pik_rate: None,
            cash_rate: None,
            calc_analytics: Some(true),
            calc_cashflows: Some(true),
        };

        let result = calculate_sscmfi_bond(&input).unwrap();
        let out = &result.result;

        assert!(out.price > Decimal::ZERO, "IAM price must be positive");
        assert!(
            out.cashflow_schedule.as_ref().unwrap().len() == 1,
            "IAM should have single cashflow at maturity"
        );
    }

    // -- Test 7: Stepped coupon bond
    #[test]
    fn test_stepped_coupon() {
        let input = SscmfiBondInput {
            security_type: SscmfiSecurityType::Corporate,
            payment_type: SscmfiPaymentType::Stepped,
            maturity_date: "09/01/2030".to_string(),
            coupon_rate: dec!(3.5),
            given_type: "Yield".to_string(),
            given_value: dec!(5.0),
            settlement_date: Some("02/22/2026".to_string()),
            redemption_value: None,
            day_count: None,
            eom_rule: None,
            frequency: None,
            call_schedule: None,
            step_schedule: Some(vec![
                StepSchedule { date: "09/01/2028".to_string(), coupon_rate: dec!(5.0) },
            ]),
            pik_rate: None,
            cash_rate: None,
            calc_analytics: Some(true),
            calc_cashflows: Some(false),
        };

        let result = calculate_sscmfi_bond(&input).unwrap();
        let out = &result.result;

        assert!(out.price > Decimal::ZERO, "Stepped bond price must be positive");
    }

    // -- Test 8: PIK bond pricing
    #[test]
    fn test_pik_bond() {
        let input = SscmfiBondInput {
            security_type: SscmfiSecurityType::Corporate,
            payment_type: SscmfiPaymentType::PIK,
            maturity_date: "12/01/2029".to_string(),
            coupon_rate: dec!(8.0),
            given_type: "Yield".to_string(),
            given_value: dec!(9.5),
            settlement_date: Some("02/22/2026".to_string()),
            redemption_value: None,
            day_count: None,
            eom_rule: None,
            frequency: None,
            call_schedule: None,
            step_schedule: None,
            pik_rate: Some(dec!(8.0)),
            cash_rate: None,
            calc_analytics: Some(true),
            calc_cashflows: Some(true),
        };

        let result = calculate_sscmfi_bond(&input).unwrap();
        let out = &result.result;

        assert!(out.price > Decimal::ZERO, "PIK bond price must be positive");
        assert_eq!(out.accrued_interest, Decimal::ZERO, "PIK has no cash accrued");
        // Redemption should be > par due to PIK accretion
        assert!(
            out.redemption_info.redemption_price > HUNDRED,
            "PIK redemption should be > par, got {}",
            out.redemption_info.redemption_price
        );
    }

    // -- Test 9: Duration/convexity analytics
    #[test]
    fn test_analytics_duration_convexity() {
        let input = treasury_bond_yield_input();
        let result = calculate_sscmfi_bond(&input).unwrap();
        let analytics = result.result.analytics.unwrap();

        assert!(
            analytics.macaulay_duration > Decimal::ZERO,
            "Macaulay duration must be positive"
        );
        assert!(
            analytics.modified_duration > Decimal::ZERO,
            "Modified duration must be positive"
        );
        assert!(
            analytics.macaulay_duration > analytics.modified_duration,
            "Macaulay duration should be > modified duration"
        );
        assert!(
            analytics.convexity > Decimal::ZERO,
            "Convexity must be positive"
        );
    }

    // -- Test 10: PV01 calculation
    #[test]
    fn test_pv01() {
        let input = treasury_bond_yield_input();
        let result = calculate_sscmfi_bond(&input).unwrap();
        let analytics = result.result.analytics.unwrap();

        assert!(
            analytics.pv01 > Decimal::ZERO,
            "PV01 must be positive, got {}",
            analytics.pv01
        );
        // PV01 for a 10-year 5% bond should be roughly 0.07-0.09
        assert!(
            analytics.pv01 < dec!(0.15),
            "PV01 seems too large: {}",
            analytics.pv01
        );
    }

    // -- Test 11: YV32 calculation
    #[test]
    fn test_yv32() {
        let input = treasury_bond_yield_input();
        let result = calculate_sscmfi_bond(&input).unwrap();
        let analytics = result.result.analytics.unwrap();

        assert!(
            analytics.yv32 > Decimal::ZERO,
            "YV32 must be positive, got {}",
            analytics.yv32
        );
    }

    // -- Test 12: Accrued interest
    #[test]
    fn test_accrued_interest() {
        let input = corporate_bond_input();
        let result = calculate_sscmfi_bond(&input).unwrap();
        let out = &result.result;

        assert!(
            out.accrued_interest > Decimal::ZERO,
            "Accrued interest should be positive for coupon-bearing bond"
        );
        // For 6% semi-annual, periodic coupon = 3, half period accrued ~ 1.5
        assert!(
            out.accrued_interest < dec!(5.0),
            "Accrued interest seems too large: {}",
            out.accrued_interest
        );
    }

    // -- Test 13: Dirty price = clean + accrued
    #[test]
    fn test_dirty_price() {
        let input = corporate_bond_input();
        let result = calculate_sscmfi_bond(&input).unwrap();
        let out = &result.result;

        let expected_dirty = out.price + out.accrued_interest;
        let diff = (out.trading_price - expected_dirty).abs();
        assert!(
            diff < dec!(0.0001),
            "Trading price should be clean + accrued: {} vs {}",
            out.trading_price,
            expected_dirty
        );
    }

    // -- Test 14: Security type defaults
    #[test]
    fn test_security_type_defaults() {
        // Treasury should use ActualActual
        let input = treasury_bond_yield_input();
        let result = calculate_sscmfi_bond(&input).unwrap();
        assert!(result.result.conventions.day_count.contains("ActualActual"));
        assert!(result.result.conventions.frequency.contains("Semiannual"));

        // CD should use Actual360 / Monthly
        let mut cd_input = treasury_bond_yield_input();
        cd_input.security_type = SscmfiSecurityType::CD;
        let result2 = calculate_sscmfi_bond(&cd_input).unwrap();
        assert!(result2.result.conventions.day_count.contains("Actual360"));
        assert!(result2.result.conventions.frequency.contains("Monthly"));
    }

    // -- Test 15: Invalid date format
    #[test]
    fn test_invalid_date_error() {
        let mut input = treasury_bond_yield_input();
        input.maturity_date = "2030-06-15".to_string(); // Wrong format

        let result = calculate_sscmfi_bond(&input);
        assert!(result.is_err(), "Should reject non-MM/DD/YYYY date");
    }

    // -- Test 16: Settlement after maturity
    #[test]
    fn test_settlement_after_maturity() {
        let mut input = treasury_bond_yield_input();
        input.settlement_date = Some("01/01/2040".to_string());

        let result = calculate_sscmfi_bond(&input);
        assert!(result.is_err(), "Should reject settlement after maturity");
    }

    // -- Test 17: Cashflow schedule generation
    #[test]
    fn test_cashflow_schedule() {
        let mut input = treasury_bond_yield_input();
        input.calc_cashflows = Some(true);

        let result = calculate_sscmfi_bond(&input).unwrap();
        let cfs = result.result.cashflow_schedule.unwrap();

        assert!(!cfs.is_empty(), "Cashflow schedule should not be empty");

        // Last cashflow should include redemption
        let last = cfs.last().unwrap();
        assert!(
            last.cashflow_type.contains("Redemption"),
            "Last cashflow should include redemption"
        );
        assert!(
            last.amount > dec!(100),
            "Last cashflow should be > par (coupon + redemption)"
        );
    }

    // -- Test 18: Part-PIK bond
    #[test]
    fn test_partpik_bond() {
        let input = SscmfiBondInput {
            security_type: SscmfiSecurityType::Corporate,
            payment_type: SscmfiPaymentType::PartPIK,
            maturity_date: "06/15/2028".to_string(),
            coupon_rate: dec!(10.0),
            given_type: "Yield".to_string(),
            given_value: dec!(11.0),
            settlement_date: Some("02/22/2026".to_string()),
            redemption_value: None,
            day_count: None,
            eom_rule: None,
            frequency: None,
            call_schedule: None,
            step_schedule: None,
            pik_rate: Some(dec!(4.0)),
            cash_rate: Some(dec!(6.0)),
            calc_analytics: Some(false),
            calc_cashflows: Some(false),
        };

        let result = calculate_sscmfi_bond(&input).unwrap();
        let out = &result.result;

        assert!(out.price > Decimal::ZERO, "PartPIK bond price must be positive");
        // Redemption should be > par due to PIK accretion
        assert!(
            out.redemption_info.redemption_price > HUNDRED,
            "PartPIK redemption should be > par"
        );
    }

    // -- Test 19: Invalid given_type
    #[test]
    fn test_invalid_given_type() {
        let mut input = treasury_bond_yield_input();
        input.given_type = "Spread".to_string();

        let result = calculate_sscmfi_bond(&input);
        assert!(result.is_err(), "Should reject invalid given_type");
    }

    // -- Test 20: Metadata populated
    #[test]
    fn test_metadata_populated() {
        let input = treasury_bond_yield_input();
        let result = calculate_sscmfi_bond(&input).unwrap();

        assert!(!result.methodology.is_empty());
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
    }

    // -- Test 21: Multistep coupon bond
    #[test]
    fn test_multistep_coupon() {
        let input = SscmfiBondInput {
            security_type: SscmfiSecurityType::Corporate,
            payment_type: SscmfiPaymentType::Multistep,
            maturity_date: "01/01/2032".to_string(),
            coupon_rate: dec!(3.0),
            given_type: "Yield".to_string(),
            given_value: dec!(5.0),
            settlement_date: Some("02/22/2026".to_string()),
            redemption_value: None,
            day_count: None,
            eom_rule: None,
            frequency: None,
            call_schedule: None,
            step_schedule: Some(vec![
                StepSchedule { date: "01/01/2028".to_string(), coupon_rate: dec!(4.0) },
                StepSchedule { date: "01/01/2030".to_string(), coupon_rate: dec!(5.0) },
            ]),
            pik_rate: None,
            cash_rate: None,
            calc_analytics: Some(false),
            calc_cashflows: Some(false),
        };

        let result = calculate_sscmfi_bond(&input).unwrap();
        assert!(result.result.price > Decimal::ZERO, "Multistep bond price must be positive");
    }
}
