use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{with_metadata, ComputationOutput, Money, Rate};
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NoteType {
    CapitalProtected,
    YieldEnhancement,
    Participation,
    CreditLinked,
}

impl std::fmt::Display for NoteType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NoteType::CapitalProtected => write!(f, "Capital Protected Note"),
            NoteType::YieldEnhancement => write!(f, "Yield Enhancement (Reverse Convertible)"),
            NoteType::Participation => write!(f, "Participation Note"),
            NoteType::CreditLinked => write!(f, "Credit-Linked Note"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredNoteInput {
    pub note_type: NoteType,
    pub notional: Money,
    pub maturity_years: Decimal,
    pub risk_free_rate: Rate,
    pub underlying_price: Money,
    pub underlying_volatility: Rate,

    // Capital Protected fields
    #[serde(default)]
    pub protection_level: Option<Rate>,
    #[serde(default)]
    pub participation_rate: Option<Rate>,
    #[serde(default)]
    pub cap_level: Option<Rate>,

    // Yield Enhancement fields
    #[serde(default)]
    pub barrier_level: Option<Rate>,
    #[serde(default)]
    pub coupon_rate: Option<Rate>,
    #[serde(default)]
    pub strike_pct: Option<Rate>,

    // Participation fields — reuses participation_rate, cap_level
    #[serde(default)]
    pub floor_level: Option<Rate>,

    // Credit-Linked fields
    #[serde(default)]
    pub reference_entity: Option<String>,
    #[serde(default)]
    pub credit_spread_bps: Option<Decimal>,
    #[serde(default)]
    pub recovery_rate: Option<Rate>,
    #[serde(default)]
    pub default_probability: Option<Rate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayoffScenario {
    pub underlying_return: Rate,
    pub note_return: Rate,
    pub note_payout: Money,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredNoteOutput {
    pub note_type: String,
    pub notional: Money,
    pub issue_price: Money,
    pub zero_coupon_bond_value: Money,
    pub option_component_value: Money,
    pub participation_rate: Rate,
    pub max_return: Option<Rate>,
    pub max_loss: Rate,
    pub breakeven_underlying_move: Rate,
    pub expected_return: Rate,
    pub coupon_yield: Option<Rate>,
    pub credit_risk_premium: Option<Money>,
    pub fair_spread: Option<Decimal>,
    pub payoff_scenarios: Vec<PayoffScenario>,
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// Decimal math helpers (no f64, no MathematicalOps)
// Same patterns as derivatives/options.rs
// ---------------------------------------------------------------------------

/// Taylor series exp(x) with range reduction for |x| > 2.
fn exp_decimal(x: Decimal) -> Decimal {
    let two = dec!(2);
    if x > two || x < -two {
        let half = exp_decimal(x / two);
        return half * half;
    }
    let mut sum = Decimal::ONE;
    let mut term = Decimal::ONE;
    for n in 1u32..=25 {
        term = term * x / Decimal::from(n);
        sum += term;
    }
    sum
}

/// Newton's method sqrt: 25 iterations.
fn sqrt_decimal(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    if x == Decimal::ONE {
        return Decimal::ONE;
    }
    let two = dec!(2);
    let mut guess = x / two;
    if x > dec!(100) {
        guess = dec!(10);
    } else if x < dec!(0.01) {
        guess = dec!(0.1);
    }
    for _ in 0..25 {
        guess = (guess + x / guess) / two;
    }
    guess
}

/// Natural log via Newton's method: 30 iterations.
fn ln_decimal(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return dec!(-999);
    }
    if x == Decimal::ONE {
        return Decimal::ZERO;
    }
    let mut y = if x > dec!(0.5) && x < dec!(2) {
        x - Decimal::ONE
    } else {
        let mut approx = Decimal::ZERO;
        let mut v = x;
        let e_approx = dec!(2.718281828459045);
        if x > Decimal::ONE {
            while v > e_approx {
                v /= e_approx;
                approx += Decimal::ONE;
            }
            approx + (v - Decimal::ONE)
        } else {
            while v < Decimal::ONE / e_approx {
                v *= e_approx;
                approx -= Decimal::ONE;
            }
            approx + (v - Decimal::ONE)
        }
    };
    for _ in 0..30 {
        let ey = exp_decimal(y);
        if ey == Decimal::ZERO {
            break;
        }
        y = y - Decimal::ONE + x / ey;
    }
    y
}

/// Standard normal PDF: phi(x) = exp(-x^2/2) / sqrt(2*pi)
fn norm_pdf(x: Decimal) -> Decimal {
    let two_pi = dec!(6.283185307179586);
    let exponent = -(x * x) / dec!(2);
    exp_decimal(exponent) / sqrt_decimal(two_pi)
}

/// Standard normal CDF using Abramowitz & Stegun approximation.
fn norm_cdf(x: Decimal) -> Decimal {
    let b1 = dec!(0.319381530);
    let b2 = dec!(-0.356563782);
    let b3 = dec!(1.781477937);
    let b4 = dec!(-1.821255978);
    let b5 = dec!(1.330274429);
    let p = dec!(0.2316419);

    let abs_x = if x < Decimal::ZERO { -x } else { x };
    let t = Decimal::ONE / (Decimal::ONE + p * abs_x);
    let poly = t * (b1 + t * (b2 + t * (b3 + t * (b4 + t * b5))));
    let cdf_pos = Decimal::ONE - norm_pdf(abs_x) * poly;

    if x < Decimal::ZERO {
        Decimal::ONE - cdf_pos
    } else {
        cdf_pos
    }
}

/// Iterative multiplication for (1+r)^T where T is a positive integer.
/// For fractional T, uses exp(T * ln(1+r)).
fn compound_factor(rate: Decimal, years: Decimal) -> Decimal {
    // Check if years is a whole number
    let rounded = years.round_dp(0);
    if (years - rounded).abs() < dec!(0.0001) && rounded >= Decimal::ZERO {
        let n = rounded.to_string().parse::<u32>().unwrap_or(0);
        let base = Decimal::ONE + rate;
        pow_decimal(base, n)
    } else {
        // Fractional years: use exp/ln
        exp_decimal(years * ln_decimal(Decimal::ONE + rate))
    }
}

/// Integer power via exponentiation by squaring.
fn pow_decimal(base: Decimal, exp: u32) -> Decimal {
    if exp == 0 {
        return Decimal::ONE;
    }
    let mut result = Decimal::ONE;
    let mut b = base;
    let mut e = exp;
    while e > 0 {
        if e & 1 == 1 {
            result *= b;
        }
        b *= b;
        e >>= 1;
    }
    result
}

// ---------------------------------------------------------------------------
// Black-Scholes call/put pricing (European)
// ---------------------------------------------------------------------------

fn bs_call_price(s: Decimal, k: Decimal, t: Decimal, r: Decimal, sigma: Decimal) -> Decimal {
    if t <= Decimal::ZERO || sigma <= Decimal::ZERO {
        return (s - k).max(Decimal::ZERO);
    }
    let sqrt_t = sqrt_decimal(t);
    let sigma_sqrt_t = sigma * sqrt_t;
    if sigma_sqrt_t == Decimal::ZERO {
        return (s - k * exp_decimal(-r * t)).max(Decimal::ZERO);
    }
    let d1 = (ln_decimal(s / k) + (r + sigma * sigma / dec!(2)) * t) / sigma_sqrt_t;
    let d2 = d1 - sigma_sqrt_t;
    s * norm_cdf(d1) - k * exp_decimal(-r * t) * norm_cdf(d2)
}

fn bs_put_price(s: Decimal, k: Decimal, t: Decimal, r: Decimal, sigma: Decimal) -> Decimal {
    if t <= Decimal::ZERO || sigma <= Decimal::ZERO {
        return (k - s).max(Decimal::ZERO);
    }
    let sqrt_t = sqrt_decimal(t);
    let sigma_sqrt_t = sigma * sqrt_t;
    if sigma_sqrt_t == Decimal::ZERO {
        return (k * exp_decimal(-r * t) - s).max(Decimal::ZERO);
    }
    let d1 = (ln_decimal(s / k) + (r + sigma * sigma / dec!(2)) * t) / sigma_sqrt_t;
    let d2 = d1 - sigma_sqrt_t;
    k * exp_decimal(-r * t) * norm_cdf(-d2) - s * norm_cdf(-d1)
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_common(input: &StructuredNoteInput) -> CorpFinanceResult<()> {
    if input.notional <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "notional".into(),
            reason: "must be positive".into(),
        });
    }
    if input.maturity_years <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "maturity_years".into(),
            reason: "must be positive".into(),
        });
    }
    if input.underlying_price <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "underlying_price".into(),
            reason: "must be positive".into(),
        });
    }
    if input.underlying_volatility <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "underlying_volatility".into(),
            reason: "must be positive".into(),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Standard payoff scenario generation
// ---------------------------------------------------------------------------

fn standard_scenario_returns() -> Vec<Rate> {
    vec![
        dec!(-0.50),
        dec!(-0.25),
        dec!(0.00),
        dec!(0.10),
        dec!(0.25),
        dec!(0.50),
        dec!(1.00),
    ]
}

// ---------------------------------------------------------------------------
// Capital Protected Note
// ---------------------------------------------------------------------------

fn price_capital_protected(input: &StructuredNoteInput) -> CorpFinanceResult<StructuredNoteOutput> {
    let notional = input.notional;
    let t = input.maturity_years;
    let r = input.risk_free_rate;
    let sigma = input.underlying_volatility;
    let s = input.underlying_price;

    let protection = input.protection_level.unwrap_or(Decimal::ONE);
    if protection <= Decimal::ZERO || protection > Decimal::ONE + dec!(0.0001) {
        return Err(CorpFinanceError::InvalidInput {
            field: "protection_level".into(),
            reason: "must be between 0 (exclusive) and 1.0 (inclusive)".into(),
        });
    }

    // ZCB value: PV of protected principal
    let discount_factor = compound_factor(r, t);
    let zcb_value = notional * protection / discount_factor;

    // Option budget: what remains to buy upside
    let option_budget = notional - zcb_value;

    // ATM call option value (per unit of underlying, then scale to notional)
    // Call on notional's worth of underlying: strike = S (at-the-money)
    let atm_call_per_unit = bs_call_price(s, s, t, r, sigma);

    // Number of units the notional buys
    let units = notional / s;

    // Total ATM call cost for full participation
    let full_call_cost = atm_call_per_unit * units;

    let mut warnings = Vec::new();

    // If cap level is set, compute call spread
    let (participation, option_value, max_return) = if let Some(cap) = input.cap_level {
        if cap <= Decimal::ONE {
            return Err(CorpFinanceError::InvalidInput {
                field: "cap_level".into(),
                reason: "must be greater than 1.0 (e.g., 1.3 = 130%)".into(),
            });
        }
        let cap_strike = s * cap;
        let cap_call_per_unit = bs_call_price(s, cap_strike, t, r, sigma);
        // Call spread value per unit = atm_call - cap_call
        let spread_per_unit = atm_call_per_unit - cap_call_per_unit;
        let spread_total = spread_per_unit * units;

        if spread_total <= Decimal::ZERO {
            warnings.push("Call spread value is zero or negative; cap may be too low".into());
            (Decimal::ONE, option_budget, Some(cap - Decimal::ONE))
        } else {
            // participation = option_budget / spread_total
            let part = if let Some(p) = input.participation_rate {
                // User-specified participation: check if affordable
                let cost = p * spread_total;
                if cost > option_budget + dec!(0.01) * notional {
                    warnings.push(format!(
                        "Requested participation {p} costs {cost} but budget is {option_budget}; \
                         note would trade above par"
                    ));
                }
                p
            } else {
                option_budget / spread_total
            };
            let opt_val = part * spread_total;
            (part, opt_val, Some(part * (cap - Decimal::ONE)))
        }
    } else {
        // No cap: full call participation
        let part = if let Some(p) = input.participation_rate {
            let cost = p * full_call_cost;
            if cost > option_budget + dec!(0.01) * notional {
                warnings.push(format!(
                    "Requested participation {p} costs {cost} but budget is {option_budget}; \
                     note would trade above par"
                ));
            }
            p
        } else if full_call_cost > Decimal::ZERO {
            option_budget / full_call_cost
        } else {
            warnings.push("ATM call value is zero; cannot derive participation".into());
            Decimal::ZERO
        };
        let opt_val = part * full_call_cost;
        (part, opt_val, None)
    };

    let issue_price = zcb_value + option_value;
    let max_loss = Decimal::ONE - protection;
    // Breakeven: underlying must rise enough so participation * move = 0 (for par return)
    // Since principal is protected, breakeven = 0% underlying move (get principal back)
    let breakeven = Decimal::ZERO;

    // Expected return: probability-weighted using simple normal model
    // E[max(S_T/S_0 - 1, 0)] for a lognormal underlying
    // Approximate: call_value / (S * e^(-rT)) gives risk-neutral expected upside
    let expected_upside = if s > Decimal::ZERO {
        participation * atm_call_per_unit * units / notional
    } else {
        Decimal::ZERO
    };
    // Net expected return = protection - 1 + expected_upside (relative to par)
    let expected_return = protection - Decimal::ONE + expected_upside;

    // Payoff scenarios
    let scenarios = standard_scenario_returns();
    let payoff_scenarios = scenarios
        .iter()
        .map(|&underlying_ret| {
            let note_ret = if underlying_ret <= Decimal::ZERO {
                // Downside: protected at protection_level
                protection - Decimal::ONE
            } else {
                // Upside: protection floor + participation * upside (possibly capped)
                if let Some(cap) = input.cap_level {
                    let capped_ret = underlying_ret.min(cap - Decimal::ONE);
                    participation * capped_ret
                } else {
                    participation * underlying_ret
                }
            };
            // Minimum note return is protection - 1 (e.g., -10% for 90% protection)
            let final_ret = note_ret.max(protection - Decimal::ONE);
            PayoffScenario {
                underlying_return: underlying_ret,
                note_return: final_ret,
                note_payout: notional * (Decimal::ONE + final_ret),
            }
        })
        .collect();

    Ok(StructuredNoteOutput {
        note_type: NoteType::CapitalProtected.to_string(),
        notional,
        issue_price,
        zero_coupon_bond_value: zcb_value,
        option_component_value: option_value,
        participation_rate: participation,
        max_return,
        max_loss,
        breakeven_underlying_move: breakeven,
        expected_return,
        coupon_yield: None,
        credit_risk_premium: None,
        fair_spread: None,
        payoff_scenarios,
        warnings,
    })
}

// ---------------------------------------------------------------------------
// Yield Enhancement (Reverse Convertible)
// ---------------------------------------------------------------------------

fn price_yield_enhancement(input: &StructuredNoteInput) -> CorpFinanceResult<StructuredNoteOutput> {
    let notional = input.notional;
    let t = input.maturity_years;
    let r = input.risk_free_rate;
    let sigma = input.underlying_volatility;
    let s = input.underlying_price;

    let barrier = input.barrier_level.ok_or(CorpFinanceError::InvalidInput {
        field: "barrier_level".into(),
        reason: "required for YieldEnhancement notes".into(),
    })?;
    let coupon = input.coupon_rate.ok_or(CorpFinanceError::InvalidInput {
        field: "coupon_rate".into(),
        reason: "required for YieldEnhancement notes".into(),
    })?;
    let strike_pct = input.strike_pct.unwrap_or(Decimal::ONE);

    if barrier <= Decimal::ZERO || barrier >= strike_pct {
        return Err(CorpFinanceError::InvalidInput {
            field: "barrier_level".into(),
            reason: "must be positive and below strike_pct".into(),
        });
    }

    let mut warnings = Vec::new();

    let strike = s * strike_pct;

    // Investor sells a down-and-in put.
    // Simplified barrier put approximation:
    // P_barrier ~ P_vanilla * (barrier / strike_pct)^alpha
    // alpha = 2 * r / sigma^2 + 1
    // This is a common analytical approximation for down-and-in puts.
    let vanilla_put = bs_put_price(s, strike, t, r, sigma);

    let alpha = if sigma > Decimal::ZERO {
        dec!(2) * r / (sigma * sigma) + Decimal::ONE
    } else {
        Decimal::ONE
    };

    // (barrier_level / strike_pct)^alpha approximation via exp(alpha * ln(ratio))
    let ratio = barrier / strike_pct;
    let barrier_adjustment = if ratio > Decimal::ZERO && ratio < Decimal::ONE {
        exp_decimal(alpha * ln_decimal(ratio))
    } else {
        Decimal::ONE
    };

    let barrier_put_value = vanilla_put * barrier_adjustment;

    // The put value per unit of notional
    let put_per_notional = barrier_put_value / s;

    // Fair coupon = risk_free + put_value / notional (annualized)
    let fair_coupon = if t > Decimal::ZERO {
        r + put_per_notional / t
    } else {
        r
    };

    if coupon < fair_coupon - dec!(0.001) {
        warnings.push(format!(
            "Offered coupon {coupon} is below fair coupon {fair_coupon}; \
             note would trade below par"
        ));
    }

    // ZCB component (for full principal)
    let discount_factor = compound_factor(r, t);
    let zcb_value = notional / discount_factor;

    // Option component: embedded short put value (negative = premium received)
    let option_value = barrier_put_value * notional / s;

    // Issue price: par minus excess coupon cost, but at fair value
    // At fair pricing: issue_price = ZCB + coupon_pv - put_value
    let coupon_pv = notional * coupon * t / discount_factor;
    let issue_price = zcb_value + coupon_pv - option_value;

    // Max loss: if underlying goes to barrier or below
    // Loss = (strike - barrier * S / S) / strike_pct = (strike_pct - barrier) / strike_pct
    // Actually if underlying drops to zero: loss = 100% - coupon
    // But typically max loss scenario is barrier breach:
    // Max loss = 1 - barrier/strike_pct + coupon*T (coupon offsets)
    let max_loss_without_coupon = Decimal::ONE - barrier / strike_pct;
    let max_loss = (max_loss_without_coupon - coupon * t).max(Decimal::ZERO);

    // Breakeven: underlying can drop by coupon * T before loss (if barrier breached)
    let breakeven = -(coupon * t);

    // Expected return using probability of barrier breach
    // P(S_T < barrier * S) under risk-neutral measure
    let barrier_price = s * barrier;
    let d2_barrier = if sigma > Decimal::ZERO {
        let sqrt_t = sqrt_decimal(t);
        (ln_decimal(s / barrier_price) + (r - sigma * sigma / dec!(2)) * t) / (sigma * sqrt_t)
    } else {
        dec!(10) // Large positive = no breach probability
    };
    let prob_breach = norm_cdf(-d2_barrier);
    // Expected return: (1 - p_breach) * coupon - p_breach * avg_loss_given_breach
    let avg_loss_given_breach = (Decimal::ONE - barrier) / dec!(2); // rough average
    let expected_return =
        (Decimal::ONE - prob_breach) * coupon * t - prob_breach * avg_loss_given_breach;

    // Payoff scenarios
    let scenarios = standard_scenario_returns();
    let payoff_scenarios = scenarios
        .iter()
        .map(|&underlying_ret| {
            let final_price_ratio = Decimal::ONE + underlying_ret;
            let note_ret = if final_price_ratio >= barrier {
                // No barrier breach: full principal + coupon
                coupon * t
            } else {
                // Barrier breached: receive shares worth final_price_ratio / strike_pct
                // plus coupon
                let share_value = final_price_ratio / strike_pct;
                share_value - Decimal::ONE + coupon * t
            };
            PayoffScenario {
                underlying_return: underlying_ret,
                note_return: note_ret,
                note_payout: notional * (Decimal::ONE + note_ret),
            }
        })
        .collect();

    Ok(StructuredNoteOutput {
        note_type: NoteType::YieldEnhancement.to_string(),
        notional,
        issue_price,
        zero_coupon_bond_value: zcb_value,
        option_component_value: option_value,
        participation_rate: Decimal::ONE,
        max_return: Some(coupon * t),
        max_loss,
        breakeven_underlying_move: breakeven,
        expected_return,
        coupon_yield: Some(coupon),
        credit_risk_premium: None,
        fair_spread: None,
        payoff_scenarios,
        warnings,
    })
}

// ---------------------------------------------------------------------------
// Participation Note
// ---------------------------------------------------------------------------

fn price_participation(input: &StructuredNoteInput) -> CorpFinanceResult<StructuredNoteOutput> {
    let notional = input.notional;
    let t = input.maturity_years;
    let r = input.risk_free_rate;
    let sigma = input.underlying_volatility;
    let s = input.underlying_price;

    let participation = input
        .participation_rate
        .ok_or(CorpFinanceError::InvalidInput {
            field: "participation_rate".into(),
            reason: "required for Participation notes".into(),
        })?;

    if participation <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "participation_rate".into(),
            reason: "must be positive".into(),
        });
    }

    let mut warnings = Vec::new();

    let discount_factor = compound_factor(r, t);
    let zcb_value = notional / discount_factor;

    let units = notional / s;
    let atm_call = bs_call_price(s, s, t, r, sigma);

    // If capped, use call spread
    let (option_value, max_return) = if let Some(cap) = input.cap_level {
        if cap <= Decimal::ONE {
            return Err(CorpFinanceError::InvalidInput {
                field: "cap_level".into(),
                reason: "must be greater than 1.0".into(),
            });
        }
        let cap_strike = s * cap;
        let cap_call = bs_call_price(s, cap_strike, t, r, sigma);
        let spread_per_unit = atm_call - cap_call;
        let opt_val = participation * spread_per_unit * units;
        let max_ret = participation * (cap - Decimal::ONE);
        (opt_val, Some(max_ret))
    } else {
        let opt_val = participation * atm_call * units;
        (opt_val, None)
    };

    // Floor level: if provided, investor buys a put at floor
    let floor_cost = if let Some(floor) = input.floor_level {
        if floor >= Decimal::ONE {
            warnings.push(
                "floor_level >= 1.0 means full downside protection; this is expensive".into(),
            );
        }
        let floor_strike = s * floor;
        let put_val = bs_put_price(s, floor_strike, t, r, sigma);
        put_val * units
    } else {
        Decimal::ZERO
    };

    let issue_price = zcb_value + option_value + floor_cost;

    // Funding cost for leveraged participation (>1x)
    if participation > Decimal::ONE {
        let excess_cost = option_value - (atm_call * units); // cost above 1x
        if excess_cost > Decimal::ZERO {
            warnings.push(format!(
                "Leveraged participation ({participation}x) adds funding cost of \
                 approximately {excess_cost}"
            ));
        }
    }

    // Max loss: without floor, can lose 100% of principal
    let max_loss = if let Some(floor) = input.floor_level {
        (Decimal::ONE - floor).max(Decimal::ZERO)
    } else {
        Decimal::ONE
    };

    // Breakeven: underlying must return 0% for par (no upside, no downside at par)
    let breakeven = Decimal::ZERO;

    // Expected return approximation
    let expected_upside = if s > Decimal::ZERO {
        participation * atm_call * units / notional
    } else {
        Decimal::ZERO
    };
    // For a participation note without floor, expected downside also exists
    let expected_return = expected_upside - (Decimal::ONE - Decimal::ONE / discount_factor);

    // Payoff scenarios
    let scenarios = standard_scenario_returns();
    let payoff_scenarios = scenarios
        .iter()
        .map(|&underlying_ret| {
            let note_ret = if underlying_ret > Decimal::ZERO {
                if let Some(cap) = input.cap_level {
                    let capped = underlying_ret.min(cap - Decimal::ONE);
                    participation * capped
                } else {
                    participation * underlying_ret
                }
            } else {
                // Downside: 1:1 loss unless floor
                if let Some(floor) = input.floor_level {
                    underlying_ret.max(floor - Decimal::ONE)
                } else {
                    underlying_ret
                }
            };
            PayoffScenario {
                underlying_return: underlying_ret,
                note_return: note_ret,
                note_payout: notional * (Decimal::ONE + note_ret),
            }
        })
        .collect();

    Ok(StructuredNoteOutput {
        note_type: NoteType::Participation.to_string(),
        notional,
        issue_price,
        zero_coupon_bond_value: zcb_value,
        option_component_value: option_value,
        participation_rate: participation,
        max_return,
        max_loss,
        breakeven_underlying_move: breakeven,
        expected_return,
        coupon_yield: None,
        credit_risk_premium: None,
        fair_spread: None,
        payoff_scenarios,
        warnings,
    })
}

// ---------------------------------------------------------------------------
// Credit-Linked Note
// ---------------------------------------------------------------------------

fn price_credit_linked(input: &StructuredNoteInput) -> CorpFinanceResult<StructuredNoteOutput> {
    let notional = input.notional;
    let t = input.maturity_years;
    let r = input.risk_free_rate;

    let _ref_entity = input.reference_entity.as_deref().unwrap_or("Unknown");
    let spread_bps = input
        .credit_spread_bps
        .ok_or(CorpFinanceError::InvalidInput {
            field: "credit_spread_bps".into(),
            reason: "required for CreditLinked notes".into(),
        })?;
    let recovery = input.recovery_rate.ok_or(CorpFinanceError::InvalidInput {
        field: "recovery_rate".into(),
        reason: "required for CreditLinked notes".into(),
    })?;
    let default_prob = input
        .default_probability
        .ok_or(CorpFinanceError::InvalidInput {
            field: "default_probability".into(),
            reason: "required for CreditLinked notes".into(),
        })?;
    let coupon = input.coupon_rate.ok_or(CorpFinanceError::InvalidInput {
        field: "coupon_rate".into(),
        reason: "required for CreditLinked notes".into(),
    })?;

    if recovery < Decimal::ZERO || recovery > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "recovery_rate".into(),
            reason: "must be between 0 and 1".into(),
        });
    }
    if default_prob < Decimal::ZERO || default_prob > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "default_probability".into(),
            reason: "must be between 0 and 1".into(),
        });
    }

    let mut warnings = Vec::new();

    let discount_factor = compound_factor(r, t);

    // ZCB value (risk-free principal)
    let zcb_value = notional / discount_factor;

    // CDS-like default leg: expected loss = PD * LGD * notional * T (simplified)
    // For multi-year: cumulative PD ~ 1 - (1 - annual_pd)^T
    let survival_factor = compound_factor(-default_prob, t); // (1 - pd)^T
    let cum_default_prob = Decimal::ONE - survival_factor;
    let lgd = Decimal::ONE - recovery;
    let expected_default_loss = cum_default_prob * lgd * notional;

    // PV of expected default loss
    // Approximate: discount at mid-life
    let mid_discount = compound_factor(r, t / dec!(2));
    let pv_default_loss = expected_default_loss / mid_discount;

    // Fair spread: annual spread that compensates for default risk
    // fair_spread_annual = default_prob * LGD
    let fair_spread_annual = default_prob * lgd;
    let fair_spread_bps = fair_spread_annual * dec!(10000);

    // Credit risk premium: the PV of expected default losses
    let credit_risk_premium = pv_default_loss;

    // Option component: embedded CDS protection sold
    let option_value = pv_default_loss;

    // CLN coupon yield = risk_free + credit_spread
    let total_coupon = r + spread_bps / dec!(10000);
    let coupon_yield = coupon;

    // Issue price: PV of coupons + PV of principal - expected default loss
    // PV of coupons: sum of coupon * notional / (1+r)^i for i=1..T
    let mut coupon_pv = Decimal::ZERO;
    let t_int = t.round_dp(0).to_string().parse::<u32>().unwrap_or(1);
    for i in 1..=t_int {
        let df = compound_factor(r, Decimal::from(i));
        coupon_pv += notional * total_coupon / df;
    }
    let issue_price = zcb_value + coupon_pv - pv_default_loss;

    if spread_bps < fair_spread_bps - dec!(10) {
        warnings.push(format!(
            "Credit spread ({spread_bps} bps) is below fair spread ({fair_spread_bps} bps); \
             investor is undercompensated for default risk"
        ));
    }

    // Max loss: default with low recovery
    let max_loss = lgd;

    // Breakeven: the spread earned over the period vs. loss given default
    // breakeven = 0 (break-even on underlying credit, not equity move)
    let breakeven = Decimal::ZERO;

    // Expected return: (1 - cum_pd) * total_coupon * T + cum_pd * (recovery - 1 + coupon * T)
    let expected_return = (Decimal::ONE - cum_default_prob) * total_coupon * t
        + cum_default_prob * (recovery - Decimal::ONE + total_coupon * t);

    // Payoff scenarios for CLN: based on credit events
    // We model scenarios as: no default, default at various recovery levels
    let payoff_scenarios = vec![
        PayoffScenario {
            underlying_return: Decimal::ZERO, // No default
            note_return: total_coupon * t,
            note_payout: notional * (Decimal::ONE + total_coupon * t),
        },
        PayoffScenario {
            underlying_return: dec!(-0.20), // Mild stress
            note_return: total_coupon * t,  // Still no default
            note_payout: notional * (Decimal::ONE + total_coupon * t),
        },
        PayoffScenario {
            underlying_return: dec!(-0.50), // Default, high recovery
            note_return: recovery + dec!(0.10) - Decimal::ONE + total_coupon * t,
            note_payout: notional * (recovery + dec!(0.10) + total_coupon * t),
        },
        PayoffScenario {
            underlying_return: dec!(-0.75), // Default, expected recovery
            note_return: recovery - Decimal::ONE + total_coupon * t,
            note_payout: notional * (recovery + total_coupon * t),
        },
        PayoffScenario {
            underlying_return: dec!(-1.00), // Default, low recovery
            note_return: (recovery - dec!(0.10)).max(Decimal::ZERO) - Decimal::ONE
                + total_coupon * t,
            note_payout: notional * ((recovery - dec!(0.10)).max(Decimal::ZERO) + total_coupon * t),
        },
    ];

    Ok(StructuredNoteOutput {
        note_type: NoteType::CreditLinked.to_string(),
        notional,
        issue_price,
        zero_coupon_bond_value: zcb_value,
        option_component_value: option_value,
        participation_rate: Decimal::ONE,
        max_return: Some(total_coupon * t),
        max_loss,
        breakeven_underlying_move: breakeven,
        expected_return,
        coupon_yield: Some(coupon_yield),
        credit_risk_premium: Some(credit_risk_premium),
        fair_spread: Some(fair_spread_bps),
        payoff_scenarios,
        warnings,
    })
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub fn price_structured_note(
    input: &StructuredNoteInput,
) -> CorpFinanceResult<ComputationOutput<StructuredNoteOutput>> {
    let start = Instant::now();
    validate_common(input)?;

    let output = match input.note_type {
        NoteType::CapitalProtected => price_capital_protected(input)?,
        NoteType::YieldEnhancement => price_yield_enhancement(input)?,
        NoteType::Participation => price_participation(input)?,
        NoteType::CreditLinked => price_credit_linked(input)?,
    };

    let methodology = match input.note_type {
        NoteType::CapitalProtected => "Zero-coupon bond + Black-Scholes call option decomposition",
        NoteType::YieldEnhancement => {
            "Reverse convertible: short down-and-in put with barrier approximation"
        }
        NoteType::Participation => "Participation: leveraged call/call-spread with optional floor",
        NoteType::CreditLinked => {
            "Credit-linked note: risk-free bond minus embedded CDS protection leg"
        }
    };

    let assumptions = serde_json::json!({
        "note_type": input.note_type.to_string(),
        "notional": input.notional.to_string(),
        "maturity_years": input.maturity_years.to_string(),
        "risk_free_rate": input.risk_free_rate.to_string(),
        "underlying_price": input.underlying_price.to_string(),
        "underlying_volatility": input.underlying_volatility.to_string(),
        "option_model": "Black-Scholes (Abramowitz & Stegun norm_cdf)",
        "compounding": "iterative multiplication for integer years, exp/ln for fractional",
    });

    let elapsed = start.elapsed().as_micros() as u64;
    let all_warnings = output.warnings.clone();

    Ok(with_metadata(
        methodology,
        &assumptions,
        all_warnings,
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

    fn approx_eq(a: Decimal, b: Decimal, tol: Decimal) -> bool {
        let diff = a - b;
        let abs_diff = if diff < Decimal::ZERO { -diff } else { diff };
        abs_diff < tol
    }

    fn base_input(note_type: NoteType) -> StructuredNoteInput {
        StructuredNoteInput {
            note_type,
            notional: dec!(1000000),
            maturity_years: dec!(3),
            risk_free_rate: dec!(0.04),
            underlying_price: dec!(100),
            underlying_volatility: dec!(0.25),
            protection_level: None,
            participation_rate: None,
            cap_level: None,
            barrier_level: None,
            coupon_rate: None,
            strike_pct: None,
            floor_level: None,
            reference_entity: None,
            credit_spread_bps: None,
            recovery_rate: None,
            default_probability: None,
        }
    }

    // -----------------------------------------------------------------------
    // 1. Capital protected note: 100% protection, participation derivation
    // -----------------------------------------------------------------------
    #[test]
    fn test_capital_protected_full_protection() {
        let mut input = base_input(NoteType::CapitalProtected);
        input.protection_level = Some(dec!(1.0));

        let result = price_structured_note(&input).unwrap();
        let out = &result.result;

        // With 100% protection, ZCB = 1,000,000 / (1.04)^3
        // Option budget = 1,000,000 - ZCB > 0
        assert!(
            out.zero_coupon_bond_value < out.notional,
            "ZCB {} should be less than notional {}",
            out.zero_coupon_bond_value,
            out.notional
        );
        assert!(
            out.option_component_value > Decimal::ZERO,
            "Option budget should be positive"
        );
        assert!(
            out.participation_rate > Decimal::ZERO,
            "Derived participation should be positive"
        );
        assert_eq!(
            out.max_loss,
            Decimal::ZERO,
            "100% protection means 0 max loss"
        );
        assert_eq!(
            out.breakeven_underlying_move,
            Decimal::ZERO,
            "Breakeven should be 0% for capital protected"
        );
        // Participation should be meaningful (not near zero)
        assert!(
            out.participation_rate > dec!(0.3),
            "Participation {} seems too low for 3Y 4% rates",
            out.participation_rate
        );
    }

    // -----------------------------------------------------------------------
    // 2. Capital protected note: 90% protection => higher participation
    // -----------------------------------------------------------------------
    #[test]
    fn test_capital_protected_90pct_higher_participation() {
        let mut input_100 = base_input(NoteType::CapitalProtected);
        input_100.protection_level = Some(dec!(1.0));
        let result_100 = price_structured_note(&input_100).unwrap();

        let mut input_90 = base_input(NoteType::CapitalProtected);
        input_90.protection_level = Some(dec!(0.90));
        let result_90 = price_structured_note(&input_90).unwrap();

        assert!(
            result_90.result.participation_rate > result_100.result.participation_rate,
            "90% protection ({}) should have higher participation than 100% ({})",
            result_90.result.participation_rate,
            result_100.result.participation_rate
        );
        // 90% protection means max loss = 10%
        assert!(
            approx_eq(result_90.result.max_loss, dec!(0.10), dec!(0.001)),
            "Max loss should be 10% for 90% protection, got {}",
            result_90.result.max_loss
        );
    }

    // -----------------------------------------------------------------------
    // 3. Yield enhancement: barrier not breached => full notional + coupon
    // -----------------------------------------------------------------------
    #[test]
    fn test_yield_enhancement_no_breach() {
        let mut input = base_input(NoteType::YieldEnhancement);
        input.barrier_level = Some(dec!(0.70));
        input.coupon_rate = Some(dec!(0.08));
        input.strike_pct = Some(dec!(1.0));

        let result = price_structured_note(&input).unwrap();
        let out = &result.result;

        // Check that coupon yield is set
        assert_eq!(out.coupon_yield, Some(dec!(0.08)));

        // Find the 0% underlying return scenario (no move, no breach)
        let no_move = out
            .payoff_scenarios
            .iter()
            .find(|s| s.underlying_return == Decimal::ZERO)
            .expect("Should have 0% scenario");

        // Return should be coupon * T = 0.08 * 3 = 0.24
        let expected_coupon_return = dec!(0.08) * dec!(3);
        assert!(
            approx_eq(no_move.note_return, expected_coupon_return, dec!(0.001)),
            "No-breach return {} should be coupon*T = {}",
            no_move.note_return,
            expected_coupon_return
        );
        // Payout = notional * (1 + 0.24) = 1,240,000
        assert!(
            approx_eq(no_move.note_payout, dec!(1240000), dec!(100)),
            "No-breach payout {} should be ~1,240,000",
            no_move.note_payout
        );
    }

    // -----------------------------------------------------------------------
    // 4. Yield enhancement: barrier breached => loss scenario
    // -----------------------------------------------------------------------
    #[test]
    fn test_yield_enhancement_breach() {
        let mut input = base_input(NoteType::YieldEnhancement);
        input.barrier_level = Some(dec!(0.70));
        input.coupon_rate = Some(dec!(0.08));
        input.strike_pct = Some(dec!(1.0));

        let result = price_structured_note(&input).unwrap();
        let out = &result.result;

        // -50% scenario: final_price_ratio = 0.50 < barrier 0.70 => breach
        let breach_scenario = out
            .payoff_scenarios
            .iter()
            .find(|s| s.underlying_return == dec!(-0.50))
            .expect("Should have -50% scenario");

        // Return = (0.50/1.0) - 1 + 0.08*3 = -0.50 + 0.24 = -0.26
        let expected_ret = dec!(0.50) / dec!(1.0) - Decimal::ONE + dec!(0.08) * dec!(3);
        assert!(
            approx_eq(breach_scenario.note_return, expected_ret, dec!(0.001)),
            "Breach return {} should be {}",
            breach_scenario.note_return,
            expected_ret
        );
        // Should be a loss
        assert!(
            breach_scenario.note_return < Decimal::ZERO,
            "Breach scenario should result in a loss"
        );
    }

    // -----------------------------------------------------------------------
    // 5. Participation note: 150% participation, capped at 130%
    // -----------------------------------------------------------------------
    #[test]
    fn test_participation_leveraged_capped() {
        let mut input = base_input(NoteType::Participation);
        input.participation_rate = Some(dec!(1.50));
        input.cap_level = Some(dec!(1.30));

        let result = price_structured_note(&input).unwrap();
        let out = &result.result;

        assert!(
            approx_eq(out.participation_rate, dec!(1.50), dec!(0.001)),
            "Participation should be 1.50x"
        );

        // Max return = 1.5 * (1.30 - 1.0) = 0.45
        assert_eq!(out.max_return, Some(dec!(0.45)));

        // Check +50% scenario: min(0.50, 0.30) * 1.5 = 0.30 * 1.5 = 0.45
        let up_50 = out
            .payoff_scenarios
            .iter()
            .find(|s| s.underlying_return == dec!(0.50))
            .expect("Should have +50% scenario");
        assert!(
            approx_eq(up_50.note_return, dec!(0.45), dec!(0.001)),
            "+50% should be capped at 0.45, got {}",
            up_50.note_return
        );

        // Check +10% scenario: 0.10 * 1.5 = 0.15
        let up_10 = out
            .payoff_scenarios
            .iter()
            .find(|s| s.underlying_return == dec!(0.10))
            .expect("Should have +10% scenario");
        assert!(
            approx_eq(up_10.note_return, dec!(0.15), dec!(0.001)),
            "+10% should be 0.15, got {}",
            up_10.note_return
        );

        // Downside: -25% => -25% (no floor)
        let down_25 = out
            .payoff_scenarios
            .iter()
            .find(|s| s.underlying_return == dec!(-0.25))
            .expect("Should have -25% scenario");
        assert!(
            approx_eq(down_25.note_return, dec!(-0.25), dec!(0.001)),
            "-25% should pass through, got {}",
            down_25.note_return
        );
    }

    // -----------------------------------------------------------------------
    // 6. CLN: fair spread from default probability
    // -----------------------------------------------------------------------
    #[test]
    fn test_cln_fair_spread() {
        let mut input = base_input(NoteType::CreditLinked);
        input.reference_entity = Some("Acme Corp".into());
        input.credit_spread_bps = Some(dec!(200));
        input.recovery_rate = Some(dec!(0.40));
        input.default_probability = Some(dec!(0.02));
        input.coupon_rate = Some(dec!(0.06));

        let result = price_structured_note(&input).unwrap();
        let out = &result.result;

        // Fair spread = PD * LGD * 10000 = 0.02 * 0.60 * 10000 = 120 bps
        let expected_fair_spread = dec!(0.02) * dec!(0.60) * dec!(10000);
        assert!(
            approx_eq(out.fair_spread.unwrap(), expected_fair_spread, dec!(1)),
            "Fair spread {} should be ~{} bps",
            out.fair_spread.unwrap(),
            expected_fair_spread
        );
    }

    // -----------------------------------------------------------------------
    // 7. CLN: expected default loss calculation
    // -----------------------------------------------------------------------
    #[test]
    fn test_cln_expected_default_loss() {
        let mut input = base_input(NoteType::CreditLinked);
        input.reference_entity = Some("Beta Inc".into());
        input.credit_spread_bps = Some(dec!(300));
        input.recovery_rate = Some(dec!(0.40));
        input.default_probability = Some(dec!(0.03));
        input.coupon_rate = Some(dec!(0.07));

        let result = price_structured_note(&input).unwrap();
        let out = &result.result;

        // Cumulative PD over 3 years ~ 1 - (1 - 0.03)^3 = 1 - 0.97^3 ~ 0.0873
        let cum_pd = Decimal::ONE - pow_decimal(dec!(0.97), 3);
        let lgd = dec!(0.60);
        let expected_loss = cum_pd * lgd * dec!(1000000);

        // Credit risk premium should be close to PV of expected loss
        assert!(
            out.credit_risk_premium.is_some(),
            "Credit risk premium should be set"
        );
        let premium = out.credit_risk_premium.unwrap();
        // Allow reasonable tolerance since we discount at mid-life
        assert!(
            approx_eq(premium, expected_loss, expected_loss * dec!(0.15)),
            "Credit risk premium {} should be near expected loss {}",
            premium,
            expected_loss
        );
        assert!(premium > Decimal::ZERO, "Premium should be positive");
    }

    // -----------------------------------------------------------------------
    // 8. Payoff scenario table: 7 scenarios generated
    // -----------------------------------------------------------------------
    #[test]
    fn test_payoff_scenarios_count() {
        let mut input = base_input(NoteType::CapitalProtected);
        input.protection_level = Some(dec!(1.0));

        let result = price_structured_note(&input).unwrap();
        let scenarios = &result.result.payoff_scenarios;

        assert_eq!(
            scenarios.len(),
            7,
            "Should generate 7 standard payoff scenarios, got {}",
            scenarios.len()
        );

        // Verify scenario returns are ordered
        let returns: Vec<Decimal> = scenarios.iter().map(|s| s.underlying_return).collect();
        assert_eq!(
            returns,
            vec![
                dec!(-0.50),
                dec!(-0.25),
                dec!(0.00),
                dec!(0.10),
                dec!(0.25),
                dec!(0.50),
                dec!(1.00),
            ]
        );

        // All payouts should be positive for 100% capital protected note
        for s in scenarios {
            assert!(
                s.note_payout > Decimal::ZERO,
                "Payout should be positive for capital protected note, got {} at {}% return",
                s.note_payout,
                s.underlying_return * dec!(100)
            );
        }
    }

    // -----------------------------------------------------------------------
    // 9. Capital protected with cap
    // -----------------------------------------------------------------------
    #[test]
    fn test_capital_protected_with_cap() {
        let mut input = base_input(NoteType::CapitalProtected);
        input.protection_level = Some(dec!(1.0));
        input.cap_level = Some(dec!(1.30));

        let result = price_structured_note(&input).unwrap();
        let out = &result.result;

        // Max return should be participation * 30%
        assert!(out.max_return.is_some());
        let max_ret = out.max_return.unwrap();
        let expected_max = out.participation_rate * dec!(0.30);
        assert!(
            approx_eq(max_ret, expected_max, dec!(0.001)),
            "Max return {} should be participation({}) * 30% = {}",
            max_ret,
            out.participation_rate,
            expected_max
        );

        // Participation with cap should be higher than without cap
        let mut input_no_cap = base_input(NoteType::CapitalProtected);
        input_no_cap.protection_level = Some(dec!(1.0));
        let result_no_cap = price_structured_note(&input_no_cap).unwrap();

        assert!(
            out.participation_rate > result_no_cap.result.participation_rate,
            "Capped participation {} should exceed uncapped {}",
            out.participation_rate,
            result_no_cap.result.participation_rate
        );
    }

    // -----------------------------------------------------------------------
    // 10. Validation: negative notional
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_negative_notional() {
        let mut input = base_input(NoteType::CapitalProtected);
        input.notional = dec!(-100);
        input.protection_level = Some(dec!(1.0));

        let result = price_structured_note(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "notional");
            }
            other => panic!("Expected InvalidInput, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // 11. Metadata is populated
    // -----------------------------------------------------------------------
    #[test]
    fn test_metadata_populated() {
        let mut input = base_input(NoteType::CapitalProtected);
        input.protection_level = Some(dec!(1.0));

        let result = price_structured_note(&input).unwrap();
        assert!(!result.methodology.is_empty());
        assert!(!result.metadata.version.is_empty());
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
    }

    // -----------------------------------------------------------------------
    // 12. Participation note with floor
    // -----------------------------------------------------------------------
    #[test]
    fn test_participation_with_floor() {
        let mut input = base_input(NoteType::Participation);
        input.participation_rate = Some(dec!(1.0));
        input.floor_level = Some(dec!(0.90));

        let result = price_structured_note(&input).unwrap();
        let out = &result.result;

        // Max loss should be 10% (1 - 0.90)
        assert!(
            approx_eq(out.max_loss, dec!(0.10), dec!(0.001)),
            "Max loss with 90% floor should be 10%, got {}",
            out.max_loss
        );

        // -50% scenario should be floored at -10%
        let down_50 = out
            .payoff_scenarios
            .iter()
            .find(|s| s.underlying_return == dec!(-0.50))
            .expect("Should have -50% scenario");
        assert!(
            approx_eq(down_50.note_return, dec!(-0.10), dec!(0.001)),
            "Floor should limit loss to -10%, got {}",
            down_50.note_return
        );
    }
}
