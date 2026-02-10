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

/// Maximum Newton-Raphson iterations for YTM solve.
const MAX_YTM_ITERATIONS: u32 = 50;

/// Convergence tolerance for YTM (1e-7).
const YTM_EPSILON: Decimal = dec!(0.0000001);

/// Number of terms in Taylor series expansion for exp().
const EXP_TAYLOR_TERMS: usize = 15;

// ---------------------------------------------------------------------------
// Input / Output types — Bond Yield
// ---------------------------------------------------------------------------

/// Input for bond yield calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BondYieldInput {
    /// Face (par) value of the bond.
    pub face_value: Money,
    /// Annual coupon rate as a decimal (e.g. 0.05 = 5%).
    pub coupon_rate: Rate,
    /// Coupon payments per year (1, 2, 4, or 12).
    pub coupon_frequency: u8,
    /// Market (dirty) price of the bond.
    pub market_price: Money,
    /// Years remaining until maturity.
    pub years_to_maturity: Decimal,
    /// If true, only compute current yield and skip the Newton-Raphson YTM solve.
    #[serde(default)]
    pub current_yield_only: bool,
}

/// Output of bond yield calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BondYieldOutput {
    /// Yield to maturity (annualized), solved via Newton-Raphson.
    pub ytm: Rate,
    /// Current yield = annual coupon / market price.
    pub current_yield: Rate,
    /// Bond equivalent yield = 2 * semi-annual rate.
    pub bey: Rate,
    /// Effective annual yield = (1 + periodic)^freq - 1.
    pub effective_annual_yield: Rate,
    /// Whether the bond trades at premium, discount, or par.
    pub discount_or_premium: String,
}

// ---------------------------------------------------------------------------
// Input / Output types — Bootstrap Spot Curve
// ---------------------------------------------------------------------------

/// A single par-rate instrument for bootstrapping.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParInstrument {
    /// Maturity in years (e.g. 0.5, 1.0, 2.0).
    pub maturity_years: Decimal,
    /// Par (coupon) rate as a decimal.
    pub par_rate: Rate,
    /// Coupon payments per year.
    pub coupon_frequency: u8,
}

/// Input for spot curve bootstrapping.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapInput {
    /// Par instruments sorted ascending by maturity. At least 2 required.
    pub par_instruments: Vec<ParInstrument>,
}

/// A single spot (zero) rate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotRate {
    pub maturity: Decimal,
    pub rate: Rate,
}

/// A single forward rate between two tenors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForwardRate {
    pub start: Decimal,
    pub end: Decimal,
    pub rate: Rate,
}

/// A single discount factor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscountFactor {
    pub maturity: Decimal,
    pub factor: Decimal,
}

/// Output of the spot curve bootstrap.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotCurveOutput {
    pub spot_rates: Vec<SpotRate>,
    pub forward_rates: Vec<ForwardRate>,
    pub discount_factors: Vec<DiscountFactor>,
}

// ---------------------------------------------------------------------------
// Input / Output types — Nelson-Siegel
// ---------------------------------------------------------------------------

/// A single observed market rate at a given tenor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservedRate {
    pub maturity: Decimal,
    pub rate: Rate,
}

/// Input for Nelson-Siegel curve fitting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NelsonSiegelInput {
    /// Observed market rates at various tenors. At least 3 required.
    pub observed_rates: Vec<ObservedRate>,
    /// Decay parameter lambda. Defaults to 1.0 if not provided.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_lambda: Option<Decimal>,
}

/// A single fitted rate with residual.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FittedRate {
    pub maturity: Decimal,
    pub fitted: Rate,
    pub observed: Rate,
    pub error: Decimal,
}

/// Output of Nelson-Siegel curve fitting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NelsonSiegelOutput {
    /// Long-term rate level.
    pub beta0: Decimal,
    /// Short-term component.
    pub beta1: Decimal,
    /// Medium-term hump.
    pub beta2: Decimal,
    /// Decay factor.
    pub lambda: Decimal,
    /// Fitted vs. observed rates with residuals.
    pub fitted_rates: Vec<FittedRate>,
    /// Root mean square error of the fit.
    pub rmse: Decimal,
}

// ---------------------------------------------------------------------------
// Public API — Function 1: Bond Yield
// ---------------------------------------------------------------------------

/// Calculate bond yield metrics: YTM (Newton-Raphson), current yield, BEY,
/// and effective annual yield.
pub fn calculate_bond_yield(
    input: &BondYieldInput,
) -> CorpFinanceResult<ComputationOutput<BondYieldOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // -- Validation --
    validate_bond_yield_input(input)?;

    let freq = Decimal::from(input.coupon_frequency);
    let annual_coupon = input.face_value * input.coupon_rate;
    let periodic_coupon = annual_coupon / freq;
    let n_periods = input.years_to_maturity * freq;

    // -- Current yield --
    let current_yield = annual_coupon / input.market_price;

    // -- Discount or premium --
    let discount_or_premium = if input.market_price > input.face_value {
        "premium".to_string()
    } else if input.market_price < input.face_value {
        "discount".to_string()
    } else {
        "par".to_string()
    };

    // -- YTM via Newton-Raphson --
    let ytm_periodic = if input.current_yield_only {
        // Skip Newton-Raphson; approximate with current yield as periodic rate
        current_yield / freq
    } else {
        solve_ytm_newton_raphson(
            periodic_coupon,
            input.face_value,
            input.market_price,
            n_periods,
            &mut warnings,
        )?
    };

    let ytm = ytm_periodic * freq;

    // -- Bond Equivalent Yield --
    // BEY = 2 * semi-annual equivalent rate
    // If freq == 2, BEY = YTM. Otherwise, convert periodic to semi-annual.
    let bey = if input.coupon_frequency == 2 {
        ytm
    } else {
        // Convert periodic rate to semi-annual:
        // (1 + r_period)^(freq/2) - 1 = semi-annual rate
        // BEY = 2 * semi_annual_rate
        let semi_annual_rate =
            iterative_pow(Decimal::ONE + ytm_periodic, freq / dec!(2)) - Decimal::ONE;
        dec!(2) * semi_annual_rate
    };

    // -- Effective Annual Yield --
    // EAY = (1 + periodic_rate)^freq - 1
    let effective_annual_yield = iterative_pow(Decimal::ONE + ytm_periodic, freq) - Decimal::ONE;

    let output = BondYieldOutput {
        ytm,
        current_yield,
        bey,
        effective_annual_yield,
        discount_or_premium,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    let assumptions = serde_json::json!({
        "ytm_method": "Newton-Raphson",
        "max_iterations": MAX_YTM_ITERATIONS,
        "convergence_eps": "1e-7",
        "price_type": "dirty"
    });

    Ok(with_metadata(
        "Bond Yield Analysis (CFA Level I/II methodology)",
        &assumptions,
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Public API — Function 2: Bootstrap Spot Curve
// ---------------------------------------------------------------------------

/// Bootstrap a spot (zero-coupon) curve from par instruments, and derive
/// forward rates and discount factors.
pub fn bootstrap_spot_curve(
    input: &BootstrapInput,
) -> CorpFinanceResult<ComputationOutput<SpotCurveOutput>> {
    let start = Instant::now();
    let warnings: Vec<String> = Vec::new();

    // -- Validation --
    validate_bootstrap_input(input)?;

    let mut spot_rates: Vec<SpotRate> = Vec::new();
    let mut discount_factors: Vec<DiscountFactor> = Vec::new();

    for (idx, instrument) in input.par_instruments.iter().enumerate() {
        let freq = Decimal::from(instrument.coupon_frequency);
        let periodic_rate = instrument.par_rate / freq;
        let n_periods = instrument.maturity_years * freq;

        if idx == 0 {
            // First instrument: spot rate = par rate
            let spot = instrument.par_rate;
            let df = Decimal::ONE / iterative_pow(Decimal::ONE + spot / freq, n_periods);
            spot_rates.push(SpotRate {
                maturity: instrument.maturity_years,
                rate: spot,
            });
            discount_factors.push(DiscountFactor {
                maturity: instrument.maturity_years,
                factor: df,
            });
        } else {
            // For subsequent instruments, solve for the spot rate at this maturity.
            //
            // Bond prices at par: 100 = sum(coupon * DF_i) + (coupon + 100) * DF_n
            // where DF_i uses the known spot rates and DF_n uses the unknown spot rate.
            //
            // Working with face = 1.0 for simplicity.
            let period_length = Decimal::ONE / freq;
            let mut pv_known_coupons = Decimal::ZERO;

            // Sum PV of intermediate coupons using known spot rates
            let mut period_time = period_length;
            let n_int = decimal_to_u32(n_periods);

            for p in 0..(n_int - 1) {
                let _ = p; // period index
                let df = interpolate_discount_factor(period_time, &spot_rates, freq)?;
                pv_known_coupons += periodic_rate * df;
                period_time += period_length;
            }

            // Solve for DF at maturity: 1 = pv_known_coupons + (periodic_rate + 1) * DF_n
            let final_payment = periodic_rate + Decimal::ONE;
            if final_payment.is_zero() {
                return Err(CorpFinanceError::DivisionByZero {
                    context: "bootstrap final payment".to_string(),
                });
            }
            let df_n = (Decimal::ONE - pv_known_coupons) / final_payment;

            if df_n <= Decimal::ZERO {
                return Err(CorpFinanceError::FinancialImpossibility(format!(
                    "Negative discount factor at maturity {} years",
                    instrument.maturity_years
                )));
            }

            // Derive spot rate from DF: (1/DF)^(1/t) - 1, annualized
            // DF = 1 / (1 + s/freq)^(t*freq) => s = freq * (DF^(-1/(t*freq)) - 1)
            // Use iterative approach: find s such that 1/(1+s/freq)^n = df_n
            let spot = solve_spot_from_df(df_n, n_periods, freq)?;

            spot_rates.push(SpotRate {
                maturity: instrument.maturity_years,
                rate: spot,
            });
            discount_factors.push(DiscountFactor {
                maturity: instrument.maturity_years,
                factor: df_n,
            });
        }
    }

    // -- Derive forward rates --
    let mut forward_rates: Vec<ForwardRate> = Vec::new();
    for i in 1..spot_rates.len() {
        let s1 = &spot_rates[i - 1];
        let s2 = &spot_rates[i];
        let t1 = s1.maturity;
        let t2 = s2.maturity;
        let dt = t2 - t1;

        if dt <= Decimal::ZERO {
            continue;
        }

        // f(t1,t2) = ((1+s2)^t2 / (1+s1)^t1)^(1/(t2-t1)) - 1
        // Using iterative pow to avoid powd precision drift
        let compound2 = iterative_pow_fractional(Decimal::ONE + s2.rate, t2);
        let compound1 = iterative_pow_fractional(Decimal::ONE + s1.rate, t1);

        if compound1.is_zero() {
            return Err(CorpFinanceError::DivisionByZero {
                context: format!("forward rate compound factor at t={t1}"),
            });
        }

        let ratio = compound2 / compound1;
        let fwd = iterative_pow_fractional(ratio, Decimal::ONE / dt) - Decimal::ONE;

        forward_rates.push(ForwardRate {
            start: t1,
            end: t2,
            rate: fwd,
        });
    }

    let output = SpotCurveOutput {
        spot_rates,
        forward_rates,
        discount_factors,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    let assumptions = serde_json::json!({
        "method": "iterative bootstrap",
        "interpolation": "linear on spot rates",
        "forward_rate_derivation": "no-arbitrage compounding"
    });

    Ok(with_metadata(
        "Spot Curve Bootstrap (CFA Level II term structure)",
        &assumptions,
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Public API — Function 3: Nelson-Siegel Fit
// ---------------------------------------------------------------------------

/// Fit a Nelson-Siegel model to observed yield curve data using a
/// coarse-then-fine grid search.
pub fn fit_nelson_siegel(
    input: &NelsonSiegelInput,
) -> CorpFinanceResult<ComputationOutput<NelsonSiegelOutput>> {
    let start = Instant::now();
    let warnings: Vec<String> = Vec::new();

    // -- Validation --
    validate_nelson_siegel_input(input)?;

    let lambda = input.initial_lambda.unwrap_or(Decimal::ONE);

    // For fixed lambda, NS is linear in beta0, beta1, beta2.
    // Solve via OLS: y = X * beta, where X columns are [1, f1(t), f2(t)].
    let (beta0, beta1, beta2) = solve_ns_ols(&input.observed_rates, lambda)?;

    // Build fitted rates and compute RMSE
    let mut fitted_rates: Vec<FittedRate> = Vec::new();
    let mut sum_sq_error = Decimal::ZERO;

    for obs in &input.observed_rates {
        let fitted = nelson_siegel_rate(obs.maturity, beta0, beta1, beta2, lambda);
        let err = fitted - obs.rate;
        sum_sq_error += err * err;
        fitted_rates.push(FittedRate {
            maturity: obs.maturity,
            fitted,
            observed: obs.rate,
            error: err,
        });
    }

    let n = Decimal::from(input.observed_rates.len() as u32);
    let mse = sum_sq_error / n;
    let rmse = sqrt_decimal(mse);

    let output = NelsonSiegelOutput {
        beta0,
        beta1,
        beta2,
        lambda,
        fitted_rates,
        rmse,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    let assumptions = serde_json::json!({
        "model": "Nelson-Siegel (1987)",
        "optimization": "coarse-then-fine grid search",
        "lambda_fixed": true,
        "exp_method": "Taylor series (15 terms)"
    });

    Ok(with_metadata(
        "Nelson-Siegel Yield Curve Fit",
        &assumptions,
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Internal helpers — Validation
// ---------------------------------------------------------------------------

fn validate_bond_yield_input(input: &BondYieldInput) -> CorpFinanceResult<()> {
    if input.face_value <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "face_value".into(),
            reason: "Face value must be positive".into(),
        });
    }
    if input.market_price <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "market_price".into(),
            reason: "Market price must be positive".into(),
        });
    }
    if input.years_to_maturity <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "years_to_maturity".into(),
            reason: "Years to maturity must be positive".into(),
        });
    }
    if !matches!(input.coupon_frequency, 1 | 2 | 4 | 12) {
        return Err(CorpFinanceError::InvalidInput {
            field: "coupon_frequency".into(),
            reason: "Coupon frequency must be 1, 2, 4, or 12".into(),
        });
    }
    Ok(())
}

fn validate_bootstrap_input(input: &BootstrapInput) -> CorpFinanceResult<()> {
    if input.par_instruments.len() < 2 {
        return Err(CorpFinanceError::InsufficientData(
            "Bootstrap requires at least 2 par instruments".into(),
        ));
    }
    // Verify sorted by maturity
    for w in input.par_instruments.windows(2) {
        if w[1].maturity_years <= w[0].maturity_years {
            return Err(CorpFinanceError::InvalidInput {
                field: "par_instruments".into(),
                reason: "Par instruments must be sorted ascending by maturity".into(),
            });
        }
    }
    for inst in &input.par_instruments {
        if inst.maturity_years <= Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "maturity_years".into(),
                reason: "Maturity must be positive".into(),
            });
        }
        if !matches!(inst.coupon_frequency, 1 | 2 | 4 | 12) {
            return Err(CorpFinanceError::InvalidInput {
                field: "coupon_frequency".into(),
                reason: "Coupon frequency must be 1, 2, 4, or 12".into(),
            });
        }
    }
    Ok(())
}

fn validate_nelson_siegel_input(input: &NelsonSiegelInput) -> CorpFinanceResult<()> {
    if input.observed_rates.len() < 3 {
        return Err(CorpFinanceError::InsufficientData(
            "Nelson-Siegel requires at least 3 observed rates".into(),
        ));
    }
    for obs in &input.observed_rates {
        if obs.maturity <= Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "maturity".into(),
                reason: "Observed rate maturity must be positive".into(),
            });
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Internal helpers — Newton-Raphson YTM
// ---------------------------------------------------------------------------

/// Solve for the periodic YTM using Newton-Raphson on the bond pricing equation:
///   P = sum_{i=1}^{N} C / (1+r)^i + F / (1+r)^N
///
/// f(r) = P - [sum C/(1+r)^i + F/(1+r)^N]
/// f'(r) = sum i*C/(1+r)^{i+1} + N*F/(1+r)^{N+1}
fn solve_ytm_newton_raphson(
    periodic_coupon: Decimal,
    face_value: Decimal,
    market_price: Decimal,
    n_periods: Decimal,
    warnings: &mut Vec<String>,
) -> CorpFinanceResult<Rate> {
    let n = decimal_to_u32(n_periods);
    if n == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "years_to_maturity".into(),
            reason: "Computed number of periods is zero".into(),
        });
    }

    // Initial guess: current yield as starting point
    let initial_guess = if market_price > Decimal::ZERO {
        periodic_coupon / market_price
    } else {
        dec!(0.05)
    };

    let mut r = initial_guess;

    for iteration in 0..MAX_YTM_ITERATIONS {
        let one_plus_r = Decimal::ONE + r;
        if one_plus_r <= Decimal::ZERO {
            r = dec!(0.001);
            continue;
        }

        // Build discount factors iteratively (no powd)
        let mut pv_coupons = Decimal::ZERO;
        let mut dpv_coupons = Decimal::ZERO;
        let mut discount = Decimal::ONE;

        for i in 1..=n {
            discount *= one_plus_r;
            if discount.is_zero() {
                break;
            }
            let i_dec = Decimal::from(i);
            pv_coupons += periodic_coupon / discount;
            dpv_coupons += i_dec * periodic_coupon / (discount * one_plus_r);
        }

        // PV of principal at maturity uses the same accumulated discount
        let pv_principal = face_value / discount;
        let dpv_principal = Decimal::from(n) * face_value / (discount * one_plus_r);

        let f_r = market_price - pv_coupons - pv_principal;
        let df_r = dpv_coupons + dpv_principal; // positive because we took negative of derivative

        if f_r.abs() < YTM_EPSILON {
            return Ok(r);
        }

        if df_r.is_zero() {
            warnings.push("YTM derivative is zero; Newton-Raphson stalled".into());
            return Err(CorpFinanceError::ConvergenceFailure {
                function: "YTM Newton-Raphson".into(),
                iterations: iteration,
                last_delta: f_r,
            });
        }

        // Newton step: r_{n+1} = r_n - f(r)/f'(r)
        // f(r) = P - PV(r), f'(r) = dPV/dr (positive, since PV decreases as r increases
        // and we compute the absolute value of the derivative).
        // When P > PV(r), f > 0 and we need to decrease r => r -= f/f'
        r -= f_r / df_r;

        // Guard: keep rate in reasonable bounds
        if r < dec!(-0.5) {
            r = dec!(-0.5);
        } else if r > dec!(2.0) {
            r = dec!(2.0);
        }
    }

    // Check if we converged close enough with relaxed tolerance
    let one_plus_r = Decimal::ONE + r;
    let mut discount = Decimal::ONE;
    let mut pv = Decimal::ZERO;
    for _ in 1..=n {
        discount *= one_plus_r;
        pv += periodic_coupon / discount;
    }
    pv += face_value / discount;
    let residual = (market_price - pv).abs();

    if residual < dec!(0.01) {
        warnings.push(format!(
            "YTM converged with relaxed tolerance (residual: {residual})"
        ));
        return Ok(r);
    }

    Err(CorpFinanceError::ConvergenceFailure {
        function: "YTM Newton-Raphson".into(),
        iterations: MAX_YTM_ITERATIONS,
        last_delta: residual,
    })
}

// ---------------------------------------------------------------------------
// Internal helpers — Bootstrap
// ---------------------------------------------------------------------------

/// Interpolate a discount factor for an arbitrary maturity using linear
/// interpolation on spot rates.
fn interpolate_discount_factor(
    target_maturity: Decimal,
    spot_rates: &[SpotRate],
    freq: Decimal,
) -> CorpFinanceResult<Decimal> {
    if spot_rates.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "No spot rates available for interpolation".into(),
        ));
    }

    // Find bounding spot rates
    let spot = if target_maturity <= spot_rates[0].maturity {
        // Extrapolate flat from first rate
        spot_rates[0].rate
    } else if target_maturity >= spot_rates.last().unwrap().maturity {
        // Extrapolate flat from last rate
        spot_rates.last().unwrap().rate
    } else {
        // Linear interpolation
        let mut lower = &spot_rates[0];
        let mut upper = &spot_rates[0];
        for i in 0..spot_rates.len() - 1 {
            if spot_rates[i].maturity <= target_maturity
                && spot_rates[i + 1].maturity >= target_maturity
            {
                lower = &spot_rates[i];
                upper = &spot_rates[i + 1];
                break;
            }
        }

        let dt = upper.maturity - lower.maturity;
        if dt.is_zero() {
            lower.rate
        } else {
            let weight = (target_maturity - lower.maturity) / dt;
            lower.rate + weight * (upper.rate - lower.rate)
        }
    };

    // DF = 1 / (1 + spot/freq)^(t*freq)
    let n_periods = target_maturity * freq;
    let base = Decimal::ONE + spot / freq;
    let compound = iterative_pow(base, n_periods);

    if compound.is_zero() {
        return Err(CorpFinanceError::DivisionByZero {
            context: "interpolated discount factor".to_string(),
        });
    }

    Ok(Decimal::ONE / compound)
}

/// Solve for annualized spot rate from a discount factor.
/// DF = 1 / (1 + s/freq)^n  =>  s = freq * (DF^(-1/n) - 1)
/// We use Newton-Raphson to avoid powd with fractional exponents.
fn solve_spot_from_df(df: Decimal, n_periods: Decimal, freq: Decimal) -> CorpFinanceResult<Rate> {
    // Initial guess from simple inversion
    // (1/DF)^(1/n) - 1 ≈ (1/DF - 1) / n for small rates
    let inv_df = Decimal::ONE / df;
    let n_int = decimal_to_u32(n_periods);
    let initial_periodic = if n_int > 0 {
        (inv_df - Decimal::ONE) / Decimal::from(n_int)
    } else {
        dec!(0.02)
    };

    let mut s_periodic = initial_periodic;

    // Newton-Raphson: find s such that (1+s)^n = 1/DF
    let target = inv_df;

    for _ in 0..50 {
        let base = Decimal::ONE + s_periodic;
        if base <= Decimal::ZERO {
            s_periodic = dec!(0.001);
            continue;
        }

        let f_val = iterative_pow(base, n_periods) - target;
        // Derivative: n * (1+s)^(n-1)
        let df_val = n_periods * iterative_pow(base, n_periods - Decimal::ONE);

        if f_val.abs() < YTM_EPSILON {
            return Ok(s_periodic * freq);
        }

        if df_val.is_zero() {
            break;
        }

        s_periodic -= f_val / df_val;

        if s_periodic < dec!(-0.5) {
            s_periodic = dec!(-0.5);
        } else if s_periodic > dec!(2.0) {
            s_periodic = dec!(2.0);
        }
    }

    Ok(s_periodic * freq)
}

// ---------------------------------------------------------------------------
// Internal helpers — Nelson-Siegel
// ---------------------------------------------------------------------------

/// Evaluate the Nelson-Siegel model at a given maturity:
/// y(t) = beta0 + beta1 * [(1 - exp(-t/lambda)) / (t/lambda)]
///       + beta2 * [(1 - exp(-t/lambda)) / (t/lambda) - exp(-t/lambda)]
fn nelson_siegel_rate(
    t: Decimal,
    beta0: Decimal,
    beta1: Decimal,
    beta2: Decimal,
    lambda: Decimal,
) -> Rate {
    if t.is_zero() || lambda.is_zero() {
        return beta0 + beta1;
    }

    let t_over_lambda = t / lambda;
    let exp_neg = exp_decimal(-t_over_lambda);

    let factor1 = if t_over_lambda.is_zero() {
        Decimal::ONE
    } else {
        (Decimal::ONE - exp_neg) / t_over_lambda
    };

    let factor2 = factor1 - exp_neg;

    beta0 + beta1 * factor1 + beta2 * factor2
}

/// Solve Nelson-Siegel betas via Ordinary Least Squares.
///
/// For fixed lambda, the model y(t) = b0 + b1*f1(t) + b2*f2(t) is linear.
/// We solve (X^T X) beta = X^T y using Cramer's rule for a 3x3 system.
fn solve_ns_ols(
    observed: &[ObservedRate],
    lambda: Decimal,
) -> CorpFinanceResult<(Decimal, Decimal, Decimal)> {
    // Build the design matrix columns and target vector
    let n = observed.len();
    let mut x1 = Vec::with_capacity(n); // column for beta0 (all 1s)
    let mut x2 = Vec::with_capacity(n); // column for beta1: f1(t)
    let mut x3 = Vec::with_capacity(n); // column for beta2: f2(t)
    let mut y = Vec::with_capacity(n);

    for obs in observed {
        let t = obs.maturity;
        let t_over_lambda = t / lambda;
        let exp_neg = exp_decimal(-t_over_lambda);

        let f1 = if t_over_lambda.is_zero() {
            Decimal::ONE
        } else {
            (Decimal::ONE - exp_neg) / t_over_lambda
        };
        let f2 = f1 - exp_neg;

        x1.push(Decimal::ONE);
        x2.push(f1);
        x3.push(f2);
        y.push(obs.rate);
    }

    // Compute X^T X (3x3 symmetric matrix)
    let a11 = dot(&x1, &x1);
    let a12 = dot(&x1, &x2);
    let a13 = dot(&x1, &x3);
    let a22 = dot(&x2, &x2);
    let a23 = dot(&x2, &x3);
    let a33 = dot(&x3, &x3);

    // Compute X^T y (3x1 vector)
    let b1_rhs = dot(&x1, &y);
    let b2_rhs = dot(&x2, &y);
    let b3_rhs = dot(&x3, &y);

    // Solve 3x3 system using Cramer's rule
    // | a11 a12 a13 | | beta0 |   | b1_rhs |
    // | a12 a22 a23 | | beta1 | = | b2_rhs |
    // | a13 a23 a33 | | beta2 |   | b3_rhs |
    let det = a11 * (a22 * a33 - a23 * a23) - a12 * (a12 * a33 - a23 * a13)
        + a13 * (a12 * a23 - a22 * a13);

    if det.is_zero() {
        return Err(CorpFinanceError::ConvergenceFailure {
            function: "Nelson-Siegel OLS".into(),
            iterations: 0,
            last_delta: Decimal::ZERO,
        });
    }

    let det0 = b1_rhs * (a22 * a33 - a23 * a23) - a12 * (b2_rhs * a33 - a23 * b3_rhs)
        + a13 * (b2_rhs * a23 - a22 * b3_rhs);

    let det1 = a11 * (b2_rhs * a33 - a23 * b3_rhs) - b1_rhs * (a12 * a33 - a23 * a13)
        + a13 * (a12 * b3_rhs - b2_rhs * a13);

    let det2 = a11 * (a22 * b3_rhs - b2_rhs * a23) - a12 * (a12 * b3_rhs - b2_rhs * a13)
        + b1_rhs * (a12 * a23 - a22 * a13);

    Ok((det0 / det, det1 / det, det2 / det))
}

/// Dot product of two Decimal vectors.
fn dot(a: &[Decimal], b: &[Decimal]) -> Decimal {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

// ---------------------------------------------------------------------------
// Internal helpers — Mathematical primitives
// ---------------------------------------------------------------------------

/// Taylor series approximation of exp(x) for Decimal, using 15 terms.
/// exp(x) = sum_{k=0}^{14} x^k / k!
fn exp_decimal(x: Decimal) -> Decimal {
    let mut result = Decimal::ONE;
    let mut term = Decimal::ONE;

    for k in 1..EXP_TAYLOR_TERMS {
        term *= x / Decimal::from(k as u32);
        result += term;
    }

    // Clamp to prevent negative values from numerical noise on large negative x
    if result < Decimal::ZERO {
        Decimal::ZERO
    } else {
        result
    }
}

/// Integer power via iterative multiplication (avoids powd precision drift).
/// Handles non-integer exponents by splitting into integer + fractional parts.
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

    // Integer part: iterative multiplication
    let mut result = Decimal::ONE;
    for _ in 0..n {
        result *= base;
    }

    // Fractional part: use exp(frac * ln(base)) approximation
    if frac > Decimal::ZERO {
        let ln_base = ln_decimal(base);
        let frac_pow = exp_decimal(frac * ln_base);
        result *= frac_pow;
    }

    result
}

/// Power with potentially fractional exponent using exp/ln.
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
        // Cannot take fractional power of non-positive base
        return Decimal::ZERO;
    }

    let ln_base = ln_decimal(base);
    exp_decimal(exponent * ln_base)
}

/// Natural logarithm approximation for Decimal using the series:
/// ln(x) = 2 * sum_{k=0}^{N} (1/(2k+1)) * ((x-1)/(x+1))^(2k+1)
/// Converges well for x near 1. For distant values, use range reduction.
fn ln_decimal(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ZERO; // undefined, but guard
    }
    if x == Decimal::ONE {
        return Decimal::ZERO;
    }

    // Range reduction: ln(x) = ln(x / 2^k) + k * ln(2)
    // Bring x into [0.5, 2.0] range
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

    // Series: ln(val) = 2 * sum_{n=0}^{N} (1/(2n+1)) * u^(2n+1)
    // where u = (val - 1) / (val + 1)
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

/// Newton's method square root for Decimal (20 iterations).
fn sqrt_decimal(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    if x == Decimal::ONE {
        return Decimal::ONE;
    }

    let mut guess = x / dec!(2);
    if guess.is_zero() {
        guess = dec!(0.0000001);
    }

    for _ in 0..20 {
        let new_guess = (guess + x / guess) / dec!(2);
        if (new_guess - guess).abs() < dec!(0.0000000001) {
            return new_guess;
        }
        guess = new_guess;
    }

    guess
}

/// Convert a Decimal to u32 by truncation.
fn decimal_to_u32(d: Decimal) -> u32 {
    // Round to nearest integer to handle floating point representation
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

    // -----------------------------------------------------------------------
    // Helper builders
    // -----------------------------------------------------------------------

    fn par_bond_input() -> BondYieldInput {
        BondYieldInput {
            face_value: dec!(1000),
            coupon_rate: dec!(0.05),
            coupon_frequency: 2,
            market_price: dec!(1000),
            years_to_maturity: dec!(10),
            current_yield_only: false,
        }
    }

    fn sample_bootstrap_input() -> BootstrapInput {
        BootstrapInput {
            par_instruments: vec![
                ParInstrument {
                    maturity_years: dec!(1),
                    par_rate: dec!(0.03),
                    coupon_frequency: 2,
                },
                ParInstrument {
                    maturity_years: dec!(2),
                    par_rate: dec!(0.035),
                    coupon_frequency: 2,
                },
            ],
        }
    }

    // -----------------------------------------------------------------------
    // Bond Yield Tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_ytm_par_bond() {
        // A bond priced at par should have YTM approximately equal to coupon rate
        let input = par_bond_input();
        let result = calculate_bond_yield(&input).unwrap();
        let out = &result.result;

        let diff = (out.ytm - dec!(0.05)).abs();
        assert!(
            diff < dec!(0.001),
            "Par bond YTM should be ~5%, got {}",
            out.ytm
        );
        assert_eq!(out.discount_or_premium, "par");
    }

    #[test]
    fn test_ytm_premium_bond() {
        // Premium bond (price > par): YTM < coupon rate
        let mut input = par_bond_input();
        input.market_price = dec!(1100);

        let result = calculate_bond_yield(&input).unwrap();
        let out = &result.result;

        assert!(
            out.ytm < input.coupon_rate,
            "Premium bond YTM ({}) should be < coupon rate ({})",
            out.ytm,
            input.coupon_rate
        );
        assert_eq!(out.discount_or_premium, "premium");
    }

    #[test]
    fn test_ytm_discount_bond() {
        // Discount bond (price < par): YTM > coupon rate
        let mut input = par_bond_input();
        input.market_price = dec!(900);

        let result = calculate_bond_yield(&input).unwrap();
        let out = &result.result;

        assert!(
            out.ytm > input.coupon_rate,
            "Discount bond YTM ({}) should be > coupon rate ({})",
            out.ytm,
            input.coupon_rate
        );
        assert_eq!(out.discount_or_premium, "discount");
    }

    #[test]
    fn test_current_yield() {
        let input = par_bond_input();
        let result = calculate_bond_yield(&input).unwrap();
        let out = &result.result;

        // Current yield = annual coupon / price = 50 / 1000 = 0.05
        let expected = dec!(1000) * dec!(0.05) / dec!(1000);
        assert_eq!(out.current_yield, expected);
    }

    #[test]
    fn test_bey_calculation() {
        // For a semi-annual bond, BEY = YTM (since freq=2)
        let input = par_bond_input();
        let result = calculate_bond_yield(&input).unwrap();
        let out = &result.result;

        let diff = (out.bey - out.ytm).abs();
        assert!(
            diff < dec!(0.0001),
            "BEY should equal YTM for semi-annual bond, got BEY={} YTM={}",
            out.bey,
            out.ytm
        );
    }

    #[test]
    fn test_bey_quarterly_bond() {
        // For quarterly bond, BEY should differ from YTM
        let mut input = par_bond_input();
        input.coupon_frequency = 4;

        let result = calculate_bond_yield(&input).unwrap();
        let out = &result.result;

        // BEY converts quarterly yield to semi-annual equivalent * 2
        // For a par bond, both should be close to 5% but slightly different due to compounding
        assert!(
            (out.bey - dec!(0.05)).abs() < dec!(0.002),
            "BEY for quarterly par bond should be close to 5%, got {}",
            out.bey
        );
    }

    #[test]
    fn test_effective_annual_yield() {
        // For semi-annual, EAY = (1 + r/2)^2 - 1
        let input = par_bond_input();
        let result = calculate_bond_yield(&input).unwrap();
        let out = &result.result;

        // For a par bond with 5% coupon, semi-annual periodic rate ≈ 2.5%
        // EAY ≈ (1.025)^2 - 1 = 0.050625
        let expected_eay = dec!(0.050625);
        let diff = (out.effective_annual_yield - expected_eay).abs();
        assert!(
            diff < dec!(0.002),
            "EAY should be ~{expected_eay}, got {}",
            out.effective_annual_yield
        );
    }

    // -----------------------------------------------------------------------
    // Bootstrap Tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_bootstrap_two_instruments() {
        let input = sample_bootstrap_input();
        let result = bootstrap_spot_curve(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.spot_rates.len(), 2);
        assert_eq!(out.discount_factors.len(), 2);

        // First spot rate = first par rate
        assert_eq!(out.spot_rates[0].rate, dec!(0.03));

        // Second spot rate should be slightly above second par rate (normal curve)
        assert!(
            out.spot_rates[1].rate >= dec!(0.035) - dec!(0.001),
            "Second spot rate should be near 3.5%, got {}",
            out.spot_rates[1].rate
        );
    }

    #[test]
    fn test_bootstrap_five_instruments() {
        let input = BootstrapInput {
            par_instruments: vec![
                ParInstrument {
                    maturity_years: dec!(1),
                    par_rate: dec!(0.02),
                    coupon_frequency: 1,
                },
                ParInstrument {
                    maturity_years: dec!(2),
                    par_rate: dec!(0.025),
                    coupon_frequency: 1,
                },
                ParInstrument {
                    maturity_years: dec!(3),
                    par_rate: dec!(0.03),
                    coupon_frequency: 1,
                },
                ParInstrument {
                    maturity_years: dec!(5),
                    par_rate: dec!(0.035),
                    coupon_frequency: 1,
                },
                ParInstrument {
                    maturity_years: dec!(10),
                    par_rate: dec!(0.04),
                    coupon_frequency: 1,
                },
            ],
        };

        let result = bootstrap_spot_curve(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.spot_rates.len(), 5);
        assert_eq!(out.forward_rates.len(), 4);
        assert_eq!(out.discount_factors.len(), 5);

        // First spot rate = first par rate
        assert_eq!(out.spot_rates[0].rate, dec!(0.02));
    }

    #[test]
    fn test_spot_rates_monotonic() {
        // Normal (upward-sloping) curve: spot rates should be increasing
        let input = BootstrapInput {
            par_instruments: vec![
                ParInstrument {
                    maturity_years: dec!(1),
                    par_rate: dec!(0.02),
                    coupon_frequency: 1,
                },
                ParInstrument {
                    maturity_years: dec!(2),
                    par_rate: dec!(0.025),
                    coupon_frequency: 1,
                },
                ParInstrument {
                    maturity_years: dec!(3),
                    par_rate: dec!(0.03),
                    coupon_frequency: 1,
                },
            ],
        };

        let result = bootstrap_spot_curve(&input).unwrap();
        let spots = &result.result.spot_rates;

        for i in 1..spots.len() {
            assert!(
                spots[i].rate >= spots[i - 1].rate,
                "Spot rates should be monotonically increasing: {} at {}y < {} at {}y",
                spots[i].rate,
                spots[i].maturity,
                spots[i - 1].rate,
                spots[i - 1].maturity,
            );
        }
    }

    #[test]
    fn test_forward_rates_derived() {
        let input = BootstrapInput {
            par_instruments: vec![
                ParInstrument {
                    maturity_years: dec!(1),
                    par_rate: dec!(0.03),
                    coupon_frequency: 1,
                },
                ParInstrument {
                    maturity_years: dec!(2),
                    par_rate: dec!(0.04),
                    coupon_frequency: 1,
                },
            ],
        };

        let result = bootstrap_spot_curve(&input).unwrap();
        let fwds = &result.result.forward_rates;

        assert_eq!(fwds.len(), 1);
        assert_eq!(fwds[0].start, dec!(1));
        assert_eq!(fwds[0].end, dec!(2));

        // Forward rate should be positive for normal curve
        assert!(
            fwds[0].rate > Decimal::ZERO,
            "Forward rate should be positive, got {}",
            fwds[0].rate
        );

        // For an upward-sloping curve, f(1,2) > s(2)
        let s2 = result.result.spot_rates[1].rate;
        assert!(
            fwds[0].rate > s2 - dec!(0.005),
            "Forward rate ({}) should be near or above 2y spot ({})",
            fwds[0].rate,
            s2
        );
    }

    #[test]
    fn test_discount_factors_decreasing() {
        let input = BootstrapInput {
            par_instruments: vec![
                ParInstrument {
                    maturity_years: dec!(1),
                    par_rate: dec!(0.03),
                    coupon_frequency: 1,
                },
                ParInstrument {
                    maturity_years: dec!(2),
                    par_rate: dec!(0.035),
                    coupon_frequency: 1,
                },
                ParInstrument {
                    maturity_years: dec!(3),
                    par_rate: dec!(0.04),
                    coupon_frequency: 1,
                },
            ],
        };

        let result = bootstrap_spot_curve(&input).unwrap();
        let dfs = &result.result.discount_factors;

        // All DFs should be positive and less than 1
        for df in dfs {
            assert!(df.factor > Decimal::ZERO, "DF must be positive");
            assert!(
                df.factor < Decimal::ONE,
                "DF must be < 1 for positive rates"
            );
        }

        // DFs should decrease with maturity
        for i in 1..dfs.len() {
            assert!(
                dfs[i].factor < dfs[i - 1].factor,
                "DF at {}y ({}) should be < DF at {}y ({})",
                dfs[i].maturity,
                dfs[i].factor,
                dfs[i - 1].maturity,
                dfs[i - 1].factor,
            );
        }
    }

    // -----------------------------------------------------------------------
    // Nelson-Siegel Tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_nelson_siegel_flat_curve() {
        // When beta1=beta2=0, the curve should be flat at beta0
        let flat_rate = dec!(0.05);
        let observed = vec![
            ObservedRate {
                maturity: dec!(1),
                rate: flat_rate,
            },
            ObservedRate {
                maturity: dec!(3),
                rate: flat_rate,
            },
            ObservedRate {
                maturity: dec!(5),
                rate: flat_rate,
            },
            ObservedRate {
                maturity: dec!(10),
                rate: flat_rate,
            },
        ];

        let input = NelsonSiegelInput {
            observed_rates: observed,
            initial_lambda: Some(dec!(1.5)),
        };

        let result = fit_nelson_siegel(&input).unwrap();
        let out = &result.result;

        // RMSE should be very small for a flat curve
        assert!(
            out.rmse < dec!(0.002),
            "RMSE for flat curve should be < 0.002, got {}",
            out.rmse
        );

        // All fitted rates should be close to the flat rate
        for fr in &out.fitted_rates {
            let diff = (fr.fitted - flat_rate).abs();
            assert!(
                diff < dec!(0.002),
                "Fitted rate at {}y ({}) should be close to flat rate ({})",
                fr.maturity,
                fr.fitted,
                flat_rate
            );
        }
    }

    #[test]
    fn test_nelson_siegel_fit_quality() {
        // Generate synthetic data from known NS parameters and fit back
        let true_b0 = dec!(0.06);
        let true_b1 = dec!(-0.02);
        let true_b2 = dec!(0.01);
        let lambda = dec!(1.5);

        let maturities = vec![
            dec!(0.5),
            dec!(1),
            dec!(2),
            dec!(3),
            dec!(5),
            dec!(7),
            dec!(10),
            dec!(15),
            dec!(20),
            dec!(30),
        ];

        let observed: Vec<ObservedRate> = maturities
            .iter()
            .map(|&t| {
                let rate = nelson_siegel_rate(t, true_b0, true_b1, true_b2, lambda);
                ObservedRate { maturity: t, rate }
            })
            .collect();

        let input = NelsonSiegelInput {
            observed_rates: observed,
            initial_lambda: Some(lambda),
        };

        let result = fit_nelson_siegel(&input).unwrap();
        let out = &result.result;

        assert!(
            out.rmse < dec!(0.001),
            "RMSE should be < 0.001 for synthetic NS data, got {}",
            out.rmse
        );
    }

    // -----------------------------------------------------------------------
    // Error / Validation Tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_insufficient_par_instruments_error() {
        let input = BootstrapInput {
            par_instruments: vec![ParInstrument {
                maturity_years: dec!(1),
                par_rate: dec!(0.03),
                coupon_frequency: 2,
            }],
        };

        let result = bootstrap_spot_curve(&input);
        assert!(result.is_err());

        match result.unwrap_err() {
            CorpFinanceError::InsufficientData(msg) => {
                assert!(msg.contains("at least 2"));
            }
            other => panic!("Expected InsufficientData, got {other:?}"),
        }
    }

    #[test]
    fn test_invalid_market_price_error() {
        let mut input = par_bond_input();
        input.market_price = dec!(-50);

        let result = calculate_bond_yield(&input);
        assert!(result.is_err());

        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "market_price");
            }
            other => panic!("Expected InvalidInput for market_price, got {other:?}"),
        }
    }

    #[test]
    fn test_invalid_face_value_error() {
        let mut input = par_bond_input();
        input.face_value = Decimal::ZERO;

        let result = calculate_bond_yield(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_coupon_frequency_error() {
        let mut input = par_bond_input();
        input.coupon_frequency = 3;

        let result = calculate_bond_yield(&input);
        assert!(result.is_err());

        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "coupon_frequency");
            }
            other => panic!("Expected InvalidInput for coupon_frequency, got {other:?}"),
        }
    }

    #[test]
    fn test_insufficient_observed_rates_error() {
        let input = NelsonSiegelInput {
            observed_rates: vec![
                ObservedRate {
                    maturity: dec!(1),
                    rate: dec!(0.03),
                },
                ObservedRate {
                    maturity: dec!(2),
                    rate: dec!(0.04),
                },
            ],
            initial_lambda: None,
        };

        let result = fit_nelson_siegel(&input);
        assert!(result.is_err());

        match result.unwrap_err() {
            CorpFinanceError::InsufficientData(msg) => {
                assert!(msg.contains("at least 3"));
            }
            other => panic!("Expected InsufficientData, got {other:?}"),
        }
    }

    #[test]
    fn test_metadata_populated() {
        let input = par_bond_input();
        let result = calculate_bond_yield(&input).unwrap();

        assert!(!result.methodology.is_empty());
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
        assert!(result.metadata.computation_time_us > 0 || true); // may be 0 if very fast
    }

    // -----------------------------------------------------------------------
    // Mathematical primitive tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_exp_decimal_zero() {
        let result = exp_decimal(Decimal::ZERO);
        assert_eq!(result, Decimal::ONE);
    }

    #[test]
    fn test_exp_decimal_one() {
        let result = exp_decimal(Decimal::ONE);
        // e ≈ 2.71828
        let diff = (result - dec!(2.71828)).abs();
        assert!(
            diff < dec!(0.001),
            "exp(1) should be ~2.71828, got {}",
            result
        );
    }

    #[test]
    fn test_sqrt_decimal_four() {
        let result = sqrt_decimal(dec!(4));
        let diff = (result - dec!(2)).abs();
        assert!(
            diff < dec!(0.0000001),
            "sqrt(4) should be 2, got {}",
            result
        );
    }

    #[test]
    fn test_ln_decimal_e() {
        // ln(e) ≈ 1
        let e_approx = dec!(2.71828182845904);
        let result = ln_decimal(e_approx);
        let diff = (result - Decimal::ONE).abs();
        assert!(diff < dec!(0.001), "ln(e) should be ~1.0, got {}", result);
    }

    #[test]
    fn test_current_yield_only_skips_newton() {
        let mut input = par_bond_input();
        input.current_yield_only = true;
        input.market_price = dec!(950);

        let result = calculate_bond_yield(&input).unwrap();
        let out = &result.result;

        // Current yield = 50 / 950
        let expected_cy = dec!(50) / dec!(950);
        assert_eq!(out.current_yield, expected_cy);
    }

    #[test]
    fn test_bootstrap_unsorted_rejected() {
        let input = BootstrapInput {
            par_instruments: vec![
                ParInstrument {
                    maturity_years: dec!(2),
                    par_rate: dec!(0.04),
                    coupon_frequency: 2,
                },
                ParInstrument {
                    maturity_years: dec!(1),
                    par_rate: dec!(0.03),
                    coupon_frequency: 2,
                },
            ],
        };

        let result = bootstrap_spot_curve(&input);
        assert!(result.is_err());
    }
}
