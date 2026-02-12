use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{with_metadata, ComputationOutput};
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Decimal math helpers
// ---------------------------------------------------------------------------

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
    for _ in 0..20 {
        guess = (guess + x / guess) / two;
    }
    guess
}

fn exp_decimal(x: Decimal) -> Decimal {
    let two = dec!(2);
    if x > two || x < -two {
        let half = exp_decimal(x / two);
        return half * half;
    }
    let mut sum = Decimal::ONE;
    let mut term = Decimal::ONE;
    for n in 1u32..=40 {
        term = term * x / Decimal::from(n);
        sum += term;
    }
    sum
}

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
    for _ in 0..40 {
        let ey = exp_decimal(y);
        if ey == Decimal::ZERO {
            break;
        }
        y = y - Decimal::ONE + x / ey;
    }
    y
}

#[allow(dead_code)]
fn abs_decimal(x: Decimal) -> Decimal {
    if x < Decimal::ZERO {
        -x
    } else {
        x
    }
}

/// sinh(x) = (exp(x) - exp(-x)) / 2
fn sinh_decimal(x: Decimal) -> Decimal {
    let ex = exp_decimal(x);
    let emx = exp_decimal(-x);
    (ex - emx) / dec!(2)
}

/// cosh(x) = (exp(x) + exp(-x)) / 2
#[allow(dead_code)]
fn cosh_decimal(x: Decimal) -> Decimal {
    let ex = exp_decimal(x);
    let emx = exp_decimal(-x);
    (ex + emx) / dec!(2)
}

/// acosh(x) = ln(x + sqrt(x^2 - 1)), x >= 1
fn acosh_decimal(x: Decimal) -> Decimal {
    if x < Decimal::ONE {
        return Decimal::ZERO;
    }
    ln_decimal(x + sqrt_decimal(x * x - Decimal::ONE))
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Order direction.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OrderSide {
    Buy,
    Sell,
}

/// Execution strategy selector.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExecutionStrategy {
    /// Time-Weighted Average Price
    TWAP,
    /// Volume-Weighted Average Price
    VWAP,
    /// Implementation Shortfall (Almgren-Chriss)
    IS,
    /// Percentage of Volume
    POV,
}

/// Market parameters for execution modelling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketParameters {
    pub current_price: Decimal,
    pub daily_volume: Decimal,
    /// Annualized volatility
    pub daily_volatility: Decimal,
    /// Bid-ask spread in price units
    pub bid_ask_spread: Decimal,
    /// Intraday volume distribution (sums to 1). If None, use U-shaped default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume_profile: Option<Vec<Decimal>>,
    /// Temporary impact coefficient (eta)
    pub temporary_impact: Decimal,
    /// Permanent impact coefficient (gamma)
    pub permanent_impact: Decimal,
}

/// Constraints on the execution schedule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConstraints {
    /// Maximum percentage of market volume per slice
    pub max_participation_rate: Decimal,
    /// Minimum quantity per slice
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_slice_size: Option<Decimal>,
    /// Maximum quantity per slice
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_slice_size: Option<Decimal>,
    /// Slice-index ranges where trading is not allowed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_trade_periods: Option<Vec<(u32, u32)>>,
}

/// Input for optimal execution calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimalExecutionInput {
    pub security_name: String,
    /// Total shares/units to execute
    pub order_size: Decimal,
    pub side: OrderSide,
    pub execution_strategy: ExecutionStrategy,
    pub market_params: MarketParameters,
    /// Total execution window in hours
    pub time_horizon: Decimal,
    /// Number of time slices
    pub num_slices: u32,
    /// 0 (patient) to 1 (urgent) -- affects risk aversion
    pub urgency: Decimal,
    pub constraints: ExecutionConstraints,
}

/// A single slice in the execution schedule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSlice {
    pub slice_index: u32,
    /// Hours from start
    pub time_start: Decimal,
    /// Hours from end of slice
    pub time_end: Decimal,
    /// Shares to trade in this slice
    pub quantity: Decimal,
    /// Percentage of total order
    pub pct_of_total: Decimal,
    /// Cumulative percentage executed
    pub cumulative_pct: Decimal,
    /// Expected price for this slice
    pub expected_price: Decimal,
    /// Expected market volume in this slice
    pub expected_market_volume: Decimal,
    /// Slice participation rate
    pub participation_rate: Decimal,
}

/// Execution cost breakdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionCost {
    /// Half-spread * total quantity
    pub spread_cost: Decimal,
    /// Transient market impact cost
    pub temporary_impact: Decimal,
    /// Permanent market impact cost
    pub permanent_impact: Decimal,
    /// Volatility-driven timing risk (std dev)
    pub timing_risk: Decimal,
    /// Total expected cost (spread + temp + perm)
    pub total_expected_cost: Decimal,
    /// Total cost in basis points
    pub total_cost_bps: Decimal,
    /// Cost of delayed execution
    pub opportunity_cost: Decimal,
}

/// Execution risk measures.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRisk {
    pub variance_of_cost: Decimal,
    pub std_dev_of_cost: Decimal,
    /// 95% VaR of implementation shortfall
    pub var_95: Decimal,
    /// 5th percentile cost (best case)
    pub best_case_cost: Decimal,
    /// 95th percentile cost (worst case)
    pub worst_case_cost: Decimal,
}

/// A point on the execution efficient frontier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostRiskPoint {
    pub urgency: Decimal,
    pub expected_cost: Decimal,
    pub risk: Decimal,
}

/// Benchmark comparison against an alternative strategy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkComparison {
    pub strategy: String,
    pub expected_cost_bps: Decimal,
    pub risk_bps: Decimal,
    /// Cost/risk trade-off (execution Sharpe)
    pub sharpe_of_execution: Decimal,
}

/// Full output of the optimal execution calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimalExecutionOutput {
    pub strategy: String,
    /// Optimal trade schedule
    pub schedule: Vec<ExecutionSlice>,
    /// Total execution cost estimate
    pub expected_cost: ExecutionCost,
    /// Execution risk measures
    pub risk_metrics: ExecutionRisk,
    /// Average percentage of market volume
    pub participation_rate: Decimal,
    /// Hours to complete
    pub estimated_duration: Decimal,
    /// Efficient frontier (cost vs risk)
    pub efficient_frontier: Vec<CostRiskPoint>,
    /// Comparison with other strategies
    pub benchmark_comparison: Vec<BenchmarkComparison>,
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Generate a default U-shaped intraday volume profile.
fn default_volume_profile(n: u32) -> Vec<Decimal> {
    let pi = dec!(3.141592653589793);
    let two = dec!(2);
    let mut profile = Vec::with_capacity(n as usize);
    let mut sum = Decimal::ZERO;
    for j in 0..n {
        // v_j = 1 + 0.5 * cos(pi * (2j+1) / (2N))
        let jd = Decimal::from(j);
        let nd = Decimal::from(n);
        let arg = pi * (two * jd + Decimal::ONE) / (two * nd);
        let cos_val = cos_decimal(arg);
        let v = Decimal::ONE + dec!(0.5) * cos_val;
        profile.push(v);
        sum += v;
    }
    // Normalize to sum to 1
    if sum > Decimal::ZERO {
        for v in profile.iter_mut() {
            *v /= sum;
        }
    }
    profile
}

/// cos(x) via Taylor series: cos(x) = sum_{n=0}^{20} (-1)^n x^{2n} / (2n)!
fn cos_decimal(x: Decimal) -> Decimal {
    // Range reduction: cos is periodic with period 2*pi
    let pi = dec!(3.141592653589793);
    let two_pi = dec!(2) * pi;
    let mut xr = x;
    // Reduce to [0, 2*pi)
    if xr > two_pi || xr < -two_pi {
        let n_periods = (xr / two_pi).floor();
        xr -= n_periods * two_pi;
    }
    let mut sum = Decimal::ONE;
    let mut term = Decimal::ONE;
    for n in 1u32..=20 {
        term = term * (-xr * xr) / (Decimal::from(2 * n - 1) * Decimal::from(2 * n));
        sum += term;
    }
    sum
}

/// Compute the volume profile to use: provided or default.
fn get_volume_profile(params: &MarketParameters, n: u32) -> Vec<Decimal> {
    if let Some(ref vp) = params.volume_profile {
        if vp.len() == n as usize {
            return vp.clone();
        }
    }
    default_volume_profile(n)
}

/// Check if a slice index falls in a no-trade period.
fn is_no_trade(constraints: &ExecutionConstraints, idx: u32) -> bool {
    if let Some(ref periods) = constraints.no_trade_periods {
        for (start, end) in periods {
            if idx >= *start && idx <= *end {
                return true;
            }
        }
    }
    false
}

/// Apply constraints to a raw schedule: enforce no-trade, min/max sizes, participation cap.
/// Returns adjusted quantities (same length as input).
fn apply_constraints(
    raw_quantities: &[Decimal],
    total_qty: Decimal,
    constraints: &ExecutionConstraints,
    expected_volumes: &[Decimal],
) -> Vec<Decimal> {
    let _n = raw_quantities.len();
    let mut adjusted = raw_quantities.to_vec();

    // Zero out no-trade periods
    for (i, q) in adjusted.iter_mut().enumerate() {
        if is_no_trade(constraints, i as u32) {
            *q = Decimal::ZERO;
        }
    }

    // Cap by participation rate
    for (i, q) in adjusted.iter_mut().enumerate() {
        let max_by_participation = constraints.max_participation_rate * expected_volumes[i];
        if *q > max_by_participation && max_by_participation > Decimal::ZERO {
            *q = max_by_participation;
        }
    }

    // Apply min/max slice size
    if let Some(min_size) = constraints.min_slice_size {
        for q in adjusted.iter_mut() {
            if *q > Decimal::ZERO && *q < min_size {
                *q = min_size;
            }
        }
    }
    if let Some(max_size) = constraints.max_slice_size {
        for q in adjusted.iter_mut() {
            if *q > max_size {
                *q = max_size;
            }
        }
    }

    // Rescale to match total_qty
    let current_sum: Decimal = adjusted.iter().copied().sum();
    if current_sum > Decimal::ZERO && current_sum != total_qty {
        let scale = total_qty / current_sum;
        for q in adjusted.iter_mut() {
            *q *= scale;
        }
    }

    adjusted
}

/// Compute execution costs for a given schedule.
fn compute_costs(
    quantities: &[Decimal],
    params: &MarketParameters,
    total_qty: Decimal,
    tau: Decimal,
    sigma: Decimal,
) -> (ExecutionCost, ExecutionRisk) {
    let eta = params.temporary_impact;
    let gamma = params.permanent_impact;
    let n = quantities.len();

    // Spread cost = 0.5 * bid_ask_spread * Q
    let spread_cost = dec!(0.5) * params.bid_ask_spread * total_qty;

    // Temporary impact = eta * sum( (x_j / tau)^2 * tau ) = eta * sum(x_j^2 / tau)
    let mut temp_impact = Decimal::ZERO;
    for q in quantities {
        if tau > Decimal::ZERO {
            temp_impact += eta * (*q) * (*q) / tau;
        }
    }

    // Permanent impact = gamma * Q * sum(x_j) -- cumulative effect
    // Simplified: permanent impact accumulates as sum of x_j weighted by remaining
    let mut perm_impact = Decimal::ZERO;
    let mut cumulative = Decimal::ZERO;
    for q in quantities {
        cumulative += *q;
        perm_impact += gamma * (*q) * cumulative;
    }
    // Normalize: divide by total_qty if > 0
    if total_qty > Decimal::ZERO {
        perm_impact /= total_qty;
    }

    let total_expected_cost = spread_cost + temp_impact + perm_impact;
    let notional = params.current_price * total_qty;
    let total_cost_bps = if notional > Decimal::ZERO {
        total_expected_cost / notional * dec!(10000)
    } else {
        Decimal::ZERO
    };

    // Timing risk = sigma * sqrt(sum(n_j^2 * tau))
    // n_j = remaining shares at time j
    let mut remaining = total_qty;
    let mut var_sum = Decimal::ZERO;
    for q in quantities {
        var_sum += remaining * remaining * tau;
        remaining -= *q;
    }
    let timing_risk = sigma * sqrt_decimal(var_sum);

    // Opportunity cost: cost if we delay by full horizon
    // Approximation: sigma * sqrt(T) * Q
    let total_time = tau * Decimal::from(n as u32);
    let opportunity_cost = sigma * sqrt_decimal(total_time) * total_qty * dec!(0.5);

    // Risk metrics
    let variance_of_cost = sigma * sigma * var_sum;
    let std_dev_of_cost = sqrt_decimal(variance_of_cost);
    // VaR 95% ~ expected + 1.645 * std_dev
    let z_95 = dec!(1.645);
    let var_95 = total_expected_cost + z_95 * std_dev_of_cost;
    let best_case = total_expected_cost - z_95 * std_dev_of_cost;
    let worst_case = var_95;

    let cost = ExecutionCost {
        spread_cost,
        temporary_impact: temp_impact,
        permanent_impact: perm_impact,
        timing_risk,
        total_expected_cost,
        total_cost_bps,
        opportunity_cost,
    };

    let risk = ExecutionRisk {
        variance_of_cost,
        std_dev_of_cost,
        var_95,
        best_case_cost: best_case,
        worst_case_cost: worst_case,
    };

    (cost, risk)
}

/// Compute IS (Almgren-Chriss) optimal quantities for a given urgency.
fn almgren_chriss_quantities(
    total_qty: Decimal,
    n: u32,
    tau: Decimal,
    sigma: Decimal,
    eta: Decimal,
    urgency_val: Decimal,
) -> Vec<Decimal> {
    // Risk aversion: lambda = urgency * 1e-6
    let lambda = urgency_val * dec!(0.000001);

    // kappa_tilde^2 = (lambda * sigma^2) / (eta * (tau/T)^2)
    // T = N * tau
    let total_time = Decimal::from(n) * tau;
    let tau_over_t = if total_time > Decimal::ZERO {
        tau / total_time
    } else {
        Decimal::ONE
    };
    let kappa_tilde_sq = if eta > Decimal::ZERO && tau_over_t > Decimal::ZERO {
        lambda * sigma * sigma / (eta * tau_over_t * tau_over_t)
    } else {
        Decimal::ZERO
    };

    // kappa = acosh(1 + kappa_tilde^2 / 2)
    let kappa = acosh_decimal(Decimal::ONE + kappa_tilde_sq / dec!(2));

    // Optimal trajectory: n_j = Q * sinh(kappa*(N-j)) / sinh(kappa*N)
    let kn = kappa * Decimal::from(n);
    let sinh_kn = sinh_decimal(kn);

    let mut trajectory = Vec::with_capacity((n + 1) as usize);
    for j in 0..=n {
        if sinh_kn == Decimal::ZERO {
            // Flat trajectory (TWAP-like)
            let remaining_frac = Decimal::from(n - j) / Decimal::from(n);
            trajectory.push(total_qty * remaining_frac);
        } else {
            let arg = kappa * Decimal::from(n - j);
            let n_j = total_qty * sinh_decimal(arg) / sinh_kn;
            trajectory.push(n_j);
        }
    }

    // Trade quantity per slice: x_j = n_{j-1} - n_j
    let mut quantities = Vec::with_capacity(n as usize);
    for j in 1..=n {
        let x_j = trajectory[j as usize - 1] - trajectory[j as usize];
        quantities.push(if x_j > Decimal::ZERO {
            x_j
        } else {
            Decimal::ZERO
        });
    }

    quantities
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Compute optimal execution schedule for a large order.
pub fn optimize_execution(
    input: &OptimalExecutionInput,
) -> CorpFinanceResult<ComputationOutput<OptimalExecutionOutput>> {
    let start = Instant::now();
    let warnings: Vec<String> = Vec::new();

    // -- Validation --
    if input.order_size <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "order_size".into(),
            reason: "must be positive".into(),
        });
    }
    if input.num_slices == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "num_slices".into(),
            reason: "must be at least 1".into(),
        });
    }
    if input.time_horizon <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "time_horizon".into(),
            reason: "must be positive".into(),
        });
    }
    if input.urgency < Decimal::ZERO || input.urgency > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "urgency".into(),
            reason: "must be between 0 and 1".into(),
        });
    }
    if input.market_params.current_price <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "current_price".into(),
            reason: "must be positive".into(),
        });
    }
    if input.market_params.daily_volume <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "daily_volume".into(),
            reason: "must be positive".into(),
        });
    }
    if input.constraints.max_participation_rate <= Decimal::ZERO
        || input.constraints.max_participation_rate > Decimal::ONE
    {
        return Err(CorpFinanceError::InvalidInput {
            field: "max_participation_rate".into(),
            reason: "must be between 0 (exclusive) and 1 (inclusive)".into(),
        });
    }

    let n = input.num_slices;
    let total_qty = input.order_size;
    let tau = input.time_horizon / Decimal::from(n);
    let sigma = input.market_params.daily_volatility;
    let params = &input.market_params;
    let volume_profile = get_volume_profile(params, n);

    // Expected volume per slice = daily_volume * profile_weight
    let expected_volumes: Vec<Decimal> = volume_profile
        .iter()
        .map(|v| params.daily_volume * *v)
        .collect();

    // -----------------------------------------------------------------------
    // Generate raw quantities based on strategy
    // -----------------------------------------------------------------------
    let raw_quantities: Vec<Decimal> = match input.execution_strategy {
        ExecutionStrategy::TWAP => {
            let q_per_slice = total_qty / Decimal::from(n);
            vec![q_per_slice; n as usize]
        }
        ExecutionStrategy::VWAP => volume_profile.iter().map(|v| total_qty * *v).collect(),
        ExecutionStrategy::IS => almgren_chriss_quantities(
            total_qty,
            n,
            tau,
            sigma,
            params.temporary_impact,
            input.urgency,
        ),
        ExecutionStrategy::POV => {
            // Trade at a fixed participation rate, capped by remaining
            let target_rate = input.urgency.max(dec!(0.01)); // use urgency as target pov
            let mut quantities = Vec::with_capacity(n as usize);
            let mut remaining = total_qty;
            for ev in &expected_volumes {
                let q = (target_rate * *ev).min(remaining);
                let q = if q < Decimal::ZERO { Decimal::ZERO } else { q };
                quantities.push(q);
                remaining -= q;
                if remaining <= Decimal::ZERO {
                    break;
                }
            }
            // Pad remaining slices with zero
            while quantities.len() < n as usize {
                quantities.push(Decimal::ZERO);
            }
            // If not fully filled, add remainder to last active slice
            let filled: Decimal = quantities.iter().copied().sum();
            if filled < total_qty {
                let remainder = total_qty - filled;
                // Add to the last non-zero slice, or the last slice
                let last_idx = quantities.len() - 1;
                quantities[last_idx] += remainder;
            }
            quantities
        }
    };

    // Apply constraints
    let adjusted_quantities = apply_constraints(
        &raw_quantities,
        total_qty,
        &input.constraints,
        &expected_volumes,
    );

    // -----------------------------------------------------------------------
    // Build execution schedule
    // -----------------------------------------------------------------------
    let mut schedule = Vec::with_capacity(n as usize);
    let mut cumulative = Decimal::ZERO;
    let mut total_participation = Decimal::ZERO;
    let mut active_slices = 0u32;

    for j in 0..n {
        let idx = j as usize;
        let qty = adjusted_quantities[idx];
        cumulative += qty;
        let pct = if total_qty > Decimal::ZERO {
            qty / total_qty * dec!(100)
        } else {
            Decimal::ZERO
        };
        let cum_pct = if total_qty > Decimal::ZERO {
            cumulative / total_qty * dec!(100)
        } else {
            Decimal::ZERO
        };
        let time_start = Decimal::from(j) * tau;
        let time_end = Decimal::from(j + 1) * tau;

        // Expected price: current_price + permanent impact up to this point
        let cum_impact = params.permanent_impact * cumulative;
        let sign = match input.side {
            OrderSide::Buy => Decimal::ONE,
            OrderSide::Sell => -Decimal::ONE,
        };
        let expected_price = params.current_price + sign * cum_impact;

        let ev = expected_volumes[idx];
        let pr = if ev > Decimal::ZERO {
            qty / ev
        } else {
            Decimal::ZERO
        };

        if qty > Decimal::ZERO {
            total_participation += pr;
            active_slices += 1;
        }

        schedule.push(ExecutionSlice {
            slice_index: j,
            time_start,
            time_end,
            quantity: qty,
            pct_of_total: pct,
            cumulative_pct: cum_pct,
            expected_price,
            expected_market_volume: ev,
            participation_rate: pr,
        });
    }

    let avg_participation = if active_slices > 0 {
        total_participation / Decimal::from(active_slices)
    } else {
        Decimal::ZERO
    };

    // -----------------------------------------------------------------------
    // Compute costs and risks
    // -----------------------------------------------------------------------
    let (expected_cost, risk_metrics) =
        compute_costs(&adjusted_quantities, params, total_qty, tau, sigma);

    // -----------------------------------------------------------------------
    // Efficient frontier: vary urgency from 0.01 to 1.0
    // -----------------------------------------------------------------------
    let frontier_points = 10u32;
    let mut efficient_frontier = Vec::with_capacity(frontier_points as usize);
    for i in 1..=frontier_points {
        let u = Decimal::from(i) / Decimal::from(frontier_points);
        let is_qtys =
            almgren_chriss_quantities(total_qty, n, tau, sigma, params.temporary_impact, u);
        let constrained =
            apply_constraints(&is_qtys, total_qty, &input.constraints, &expected_volumes);
        let (cost, risk) = compute_costs(&constrained, params, total_qty, tau, sigma);
        efficient_frontier.push(CostRiskPoint {
            urgency: u,
            expected_cost: cost.total_cost_bps,
            risk: risk.std_dev_of_cost,
        });
    }

    // -----------------------------------------------------------------------
    // Benchmark comparison: compute all 4 strategies
    // -----------------------------------------------------------------------
    let strategies = [
        ("TWAP", ExecutionStrategy::TWAP),
        ("VWAP", ExecutionStrategy::VWAP),
        ("IS", ExecutionStrategy::IS),
        ("POV", ExecutionStrategy::POV),
    ];

    let notional = params.current_price * total_qty;
    let mut benchmark_comparison = Vec::with_capacity(4);

    for (name, strat) in &strategies {
        let qtys: Vec<Decimal> = match strat {
            ExecutionStrategy::TWAP => {
                let q_each = total_qty / Decimal::from(n);
                vec![q_each; n as usize]
            }
            ExecutionStrategy::VWAP => volume_profile.iter().map(|v| total_qty * *v).collect(),
            ExecutionStrategy::IS => almgren_chriss_quantities(
                total_qty,
                n,
                tau,
                sigma,
                params.temporary_impact,
                input.urgency,
            ),
            ExecutionStrategy::POV => {
                let target = input.urgency.max(dec!(0.01));
                let mut qs = Vec::with_capacity(n as usize);
                let mut rem = total_qty;
                for ev in &expected_volumes {
                    let q = (target * *ev).min(rem);
                    qs.push(if q > Decimal::ZERO { q } else { Decimal::ZERO });
                    rem -= q;
                    if rem <= Decimal::ZERO {
                        break;
                    }
                }
                while qs.len() < n as usize {
                    qs.push(Decimal::ZERO);
                }
                let filled: Decimal = qs.iter().copied().sum();
                if filled < total_qty {
                    let last = qs.len() - 1;
                    qs[last] += total_qty - filled;
                }
                qs
            }
        };

        let constrained =
            apply_constraints(&qtys, total_qty, &input.constraints, &expected_volumes);
        let (c, r) = compute_costs(&constrained, params, total_qty, tau, sigma);

        let cost_bps = if notional > Decimal::ZERO {
            c.total_expected_cost / notional * dec!(10000)
        } else {
            Decimal::ZERO
        };
        let risk_bps = if notional > Decimal::ZERO {
            r.std_dev_of_cost / notional * dec!(10000)
        } else {
            Decimal::ZERO
        };
        let sharpe = if risk_bps > Decimal::ZERO {
            cost_bps / risk_bps
        } else {
            Decimal::ZERO
        };

        benchmark_comparison.push(BenchmarkComparison {
            strategy: name.to_string(),
            expected_cost_bps: cost_bps,
            risk_bps,
            sharpe_of_execution: sharpe,
        });
    }

    // -----------------------------------------------------------------------
    // Build output
    // -----------------------------------------------------------------------
    let output = OptimalExecutionOutput {
        strategy: format!("{:?}", input.execution_strategy),
        schedule,
        expected_cost,
        risk_metrics,
        participation_rate: avg_participation,
        estimated_duration: input.time_horizon,
        efficient_frontier,
        benchmark_comparison,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Optimal Execution (Almgren-Chriss 2001)",
        &serde_json::json!({
            "strategy": format!("{:?}", input.execution_strategy),
            "order_size": input.order_size.to_string(),
            "num_slices": input.num_slices,
            "urgency": input.urgency.to_string(),
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

    fn default_params() -> MarketParameters {
        MarketParameters {
            current_price: dec!(50.00),
            daily_volume: dec!(1000000),
            daily_volatility: dec!(0.02),
            bid_ask_spread: dec!(0.05),
            volume_profile: None,
            temporary_impact: dec!(0.001),
            permanent_impact: dec!(0.0001),
        }
    }

    fn default_constraints() -> ExecutionConstraints {
        ExecutionConstraints {
            max_participation_rate: dec!(0.25),
            min_slice_size: None,
            max_slice_size: None,
            no_trade_periods: None,
        }
    }

    fn basic_input() -> OptimalExecutionInput {
        OptimalExecutionInput {
            security_name: "TEST".into(),
            order_size: dec!(10000),
            side: OrderSide::Buy,
            execution_strategy: ExecutionStrategy::TWAP,
            market_params: default_params(),
            time_horizon: dec!(6.5),
            num_slices: 10,
            urgency: dec!(0.5),
            constraints: default_constraints(),
        }
    }

    // --- Validation tests ---

    #[test]
    fn test_zero_order_size() {
        let mut input = basic_input();
        input.order_size = Decimal::ZERO;
        assert!(optimize_execution(&input).is_err());
    }

    #[test]
    fn test_negative_order_size() {
        let mut input = basic_input();
        input.order_size = dec!(-100);
        assert!(optimize_execution(&input).is_err());
    }

    #[test]
    fn test_zero_slices() {
        let mut input = basic_input();
        input.num_slices = 0;
        assert!(optimize_execution(&input).is_err());
    }

    #[test]
    fn test_zero_time_horizon() {
        let mut input = basic_input();
        input.time_horizon = Decimal::ZERO;
        assert!(optimize_execution(&input).is_err());
    }

    #[test]
    fn test_negative_time_horizon() {
        let mut input = basic_input();
        input.time_horizon = dec!(-1);
        assert!(optimize_execution(&input).is_err());
    }

    #[test]
    fn test_urgency_out_of_range_high() {
        let mut input = basic_input();
        input.urgency = dec!(1.5);
        assert!(optimize_execution(&input).is_err());
    }

    #[test]
    fn test_urgency_out_of_range_low() {
        let mut input = basic_input();
        input.urgency = dec!(-0.1);
        assert!(optimize_execution(&input).is_err());
    }

    #[test]
    fn test_zero_price() {
        let mut input = basic_input();
        input.market_params.current_price = Decimal::ZERO;
        assert!(optimize_execution(&input).is_err());
    }

    #[test]
    fn test_zero_daily_volume() {
        let mut input = basic_input();
        input.market_params.daily_volume = Decimal::ZERO;
        assert!(optimize_execution(&input).is_err());
    }

    #[test]
    fn test_zero_participation_rate() {
        let mut input = basic_input();
        input.constraints.max_participation_rate = Decimal::ZERO;
        assert!(optimize_execution(&input).is_err());
    }

    #[test]
    fn test_participation_rate_over_one() {
        let mut input = basic_input();
        input.constraints.max_participation_rate = dec!(1.5);
        assert!(optimize_execution(&input).is_err());
    }

    // --- TWAP tests ---

    #[test]
    fn test_twap_equal_slices() {
        let input = basic_input();
        let result = optimize_execution(&input).unwrap();
        let schedule = &result.result.schedule;
        assert_eq!(schedule.len(), 10);
        // All quantities should be approximately equal
        let expected = dec!(10000) / dec!(10);
        for s in schedule {
            let diff = abs_decimal(s.quantity - expected);
            // Allow for constraint adjustments
            assert!(
                diff < dec!(1000),
                "slice {} qty {} too far from {}",
                s.slice_index,
                s.quantity,
                expected
            );
        }
    }

    #[test]
    fn test_twap_sums_to_total() {
        let input = basic_input();
        let result = optimize_execution(&input).unwrap();
        let total: Decimal = result.result.schedule.iter().map(|s| s.quantity).sum();
        let diff = abs_decimal(total - dec!(10000));
        assert!(diff < dec!(1), "TWAP total {} should be ~10000", total);
    }

    #[test]
    fn test_twap_cumulative_pct() {
        let input = basic_input();
        let result = optimize_execution(&input).unwrap();
        let last = result.result.schedule.last().unwrap();
        let diff = abs_decimal(last.cumulative_pct - dec!(100));
        assert!(diff < dec!(1));
    }

    // --- VWAP tests ---

    #[test]
    fn test_vwap_sums_to_total() {
        let mut input = basic_input();
        input.execution_strategy = ExecutionStrategy::VWAP;
        let result = optimize_execution(&input).unwrap();
        let total: Decimal = result.result.schedule.iter().map(|s| s.quantity).sum();
        let diff = abs_decimal(total - dec!(10000));
        assert!(diff < dec!(1));
    }

    #[test]
    fn test_vwap_proportional_to_volume() {
        let mut input = basic_input();
        input.execution_strategy = ExecutionStrategy::VWAP;
        input.market_params.volume_profile = Some(vec![
            dec!(0.2),
            dec!(0.1),
            dec!(0.1),
            dec!(0.1),
            dec!(0.1),
            dec!(0.1),
            dec!(0.1),
            dec!(0.05),
            dec!(0.05),
            dec!(0.1),
        ]);
        let result = optimize_execution(&input).unwrap();
        // First slice should have the most volume
        let first = result.result.schedule[0].quantity;
        let second = result.result.schedule[1].quantity;
        assert!(
            first > second,
            "first slice ({}) should be > second ({})",
            first,
            second
        );
    }

    #[test]
    fn test_vwap_default_u_shape() {
        let mut input = basic_input();
        input.execution_strategy = ExecutionStrategy::VWAP;
        let result = optimize_execution(&input).unwrap();
        // With U-shaped profile, first and last slices should have more volume
        let first = result.result.schedule[0].quantity;
        let mid = result.result.schedule[4].quantity;
        assert!(
            first > mid,
            "U-shape: first {} should be > mid {}",
            first,
            mid
        );
    }

    // --- IS (Almgren-Chriss) tests ---

    #[test]
    fn test_is_sums_to_total() {
        let mut input = basic_input();
        input.execution_strategy = ExecutionStrategy::IS;
        let result = optimize_execution(&input).unwrap();
        let total: Decimal = result.result.schedule.iter().map(|s| s.quantity).sum();
        let diff = abs_decimal(total - dec!(10000));
        assert!(diff < dec!(1));
    }

    #[test]
    fn test_is_urgent_front_loaded() {
        let mut input = basic_input();
        input.execution_strategy = ExecutionStrategy::IS;
        input.urgency = dec!(0.99);
        let result = optimize_execution(&input).unwrap();
        // With high urgency, first slices should be larger
        let first = result.result.schedule[0].quantity;
        let last = result.result.schedule.last().unwrap().quantity;
        assert!(
            first > last,
            "urgent IS: first {} should be > last {}",
            first,
            last
        );
    }

    #[test]
    fn test_is_patient_more_uniform() {
        let mut input = basic_input();
        input.execution_strategy = ExecutionStrategy::IS;
        input.urgency = dec!(0.01);
        let result = optimize_execution(&input).unwrap();
        // With low urgency, distribution should be more uniform
        let first = result.result.schedule[0].quantity;
        let last = result.result.schedule.last().unwrap().quantity;
        let ratio = if last > Decimal::ZERO {
            first / last
        } else {
            dec!(999)
        };
        // Should be more balanced than urgent case
        assert!(
            ratio < dec!(10),
            "patient IS ratio {} should be < 10",
            ratio
        );
    }

    #[test]
    fn test_is_schedule_count() {
        let mut input = basic_input();
        input.execution_strategy = ExecutionStrategy::IS;
        let result = optimize_execution(&input).unwrap();
        assert_eq!(result.result.schedule.len(), 10);
    }

    // --- POV tests ---

    #[test]
    fn test_pov_sums_to_total() {
        let mut input = basic_input();
        input.execution_strategy = ExecutionStrategy::POV;
        let result = optimize_execution(&input).unwrap();
        let total: Decimal = result.result.schedule.iter().map(|s| s.quantity).sum();
        let diff = abs_decimal(total - dec!(10000));
        assert!(diff < dec!(1));
    }

    #[test]
    fn test_pov_respects_urgency_as_rate() {
        let mut input = basic_input();
        input.execution_strategy = ExecutionStrategy::POV;
        input.urgency = dec!(0.5);
        let result = optimize_execution(&input).unwrap();
        assert!(result.result.participation_rate > Decimal::ZERO);
    }

    // --- Constraint tests ---

    #[test]
    fn test_no_trade_period_zeroes_slices() {
        let mut input = basic_input();
        input.constraints.no_trade_periods = Some(vec![(2, 4)]);
        let result = optimize_execution(&input).unwrap();
        // Total should still be ~10000 (redistributed)
        let total: Decimal = result.result.schedule.iter().map(|s| s.quantity).sum();
        let diff = abs_decimal(total - dec!(10000));
        assert!(diff < dec!(1));
    }

    #[test]
    fn test_max_slice_size_constraint() {
        let mut input = basic_input();
        input.constraints.max_slice_size = Some(dec!(2000));
        let result = optimize_execution(&input).unwrap();
        // After constraint + rescaling, totals still match
        let total: Decimal = result.result.schedule.iter().map(|s| s.quantity).sum();
        let diff = abs_decimal(total - dec!(10000));
        assert!(diff < dec!(1));
    }

    // --- Cost tests ---

    #[test]
    fn test_spread_cost_positive() {
        let input = basic_input();
        let result = optimize_execution(&input).unwrap();
        assert!(result.result.expected_cost.spread_cost > Decimal::ZERO);
    }

    #[test]
    fn test_spread_cost_formula() {
        let input = basic_input();
        let result = optimize_execution(&input).unwrap();
        let expected = dec!(0.5) * dec!(0.05) * dec!(10000);
        let diff = abs_decimal(result.result.expected_cost.spread_cost - expected);
        assert!(
            diff < dec!(1),
            "spread cost {} should be ~{}",
            result.result.expected_cost.spread_cost,
            expected
        );
    }

    #[test]
    fn test_total_cost_positive() {
        let input = basic_input();
        let result = optimize_execution(&input).unwrap();
        assert!(result.result.expected_cost.total_expected_cost > Decimal::ZERO);
    }

    #[test]
    fn test_total_cost_bps_positive() {
        let input = basic_input();
        let result = optimize_execution(&input).unwrap();
        assert!(result.result.expected_cost.total_cost_bps > Decimal::ZERO);
    }

    #[test]
    fn test_temporary_impact_non_negative() {
        let input = basic_input();
        let result = optimize_execution(&input).unwrap();
        assert!(result.result.expected_cost.temporary_impact >= Decimal::ZERO);
    }

    #[test]
    fn test_permanent_impact_non_negative() {
        let input = basic_input();
        let result = optimize_execution(&input).unwrap();
        assert!(result.result.expected_cost.permanent_impact >= Decimal::ZERO);
    }

    // --- Risk tests ---

    #[test]
    fn test_var_95_greater_than_expected() {
        let input = basic_input();
        let result = optimize_execution(&input).unwrap();
        assert!(
            result.result.risk_metrics.var_95 >= result.result.expected_cost.total_expected_cost
        );
    }

    #[test]
    fn test_worst_case_greater_than_best_case() {
        let input = basic_input();
        let result = optimize_execution(&input).unwrap();
        assert!(
            result.result.risk_metrics.worst_case_cost >= result.result.risk_metrics.best_case_cost
        );
    }

    #[test]
    fn test_std_dev_non_negative() {
        let input = basic_input();
        let result = optimize_execution(&input).unwrap();
        assert!(result.result.risk_metrics.std_dev_of_cost >= Decimal::ZERO);
    }

    // --- Efficient frontier tests ---

    #[test]
    fn test_frontier_has_points() {
        let input = basic_input();
        let result = optimize_execution(&input).unwrap();
        assert_eq!(result.result.efficient_frontier.len(), 10);
    }

    #[test]
    fn test_frontier_urgency_increases() {
        let input = basic_input();
        let result = optimize_execution(&input).unwrap();
        let ef = &result.result.efficient_frontier;
        for i in 1..ef.len() {
            assert!(ef[i].urgency > ef[i - 1].urgency);
        }
    }

    // --- Benchmark comparison tests ---

    #[test]
    fn test_benchmark_has_four_strategies() {
        let input = basic_input();
        let result = optimize_execution(&input).unwrap();
        assert_eq!(result.result.benchmark_comparison.len(), 4);
    }

    #[test]
    fn test_benchmark_strategy_names() {
        let input = basic_input();
        let result = optimize_execution(&input).unwrap();
        let names: Vec<&str> = result
            .result
            .benchmark_comparison
            .iter()
            .map(|b| b.strategy.as_str())
            .collect();
        assert!(names.contains(&"TWAP"));
        assert!(names.contains(&"VWAP"));
        assert!(names.contains(&"IS"));
        assert!(names.contains(&"POV"));
    }

    #[test]
    fn test_benchmark_costs_positive() {
        let input = basic_input();
        let result = optimize_execution(&input).unwrap();
        for b in &result.result.benchmark_comparison {
            assert!(
                b.expected_cost_bps >= Decimal::ZERO,
                "strategy {} cost should be >= 0",
                b.strategy
            );
        }
    }

    // --- Sell side tests ---

    #[test]
    fn test_sell_side_works() {
        let mut input = basic_input();
        input.side = OrderSide::Sell;
        let result = optimize_execution(&input).unwrap();
        assert!(result.result.expected_cost.total_expected_cost > Decimal::ZERO);
    }

    // --- Metadata tests ---

    #[test]
    fn test_methodology_label() {
        let input = basic_input();
        let result = optimize_execution(&input).unwrap();
        assert!(result.methodology.contains("Almgren-Chriss"));
    }

    #[test]
    fn test_strategy_label() {
        let input = basic_input();
        let result = optimize_execution(&input).unwrap();
        assert_eq!(result.result.strategy, "TWAP");
    }

    #[test]
    fn test_estimated_duration() {
        let input = basic_input();
        let result = optimize_execution(&input).unwrap();
        assert_eq!(result.result.estimated_duration, dec!(6.5));
    }

    // --- Math helper tests ---

    #[test]
    fn test_sinh_zero() {
        let s = sinh_decimal(Decimal::ZERO);
        assert!(abs_decimal(s) < dec!(0.0001));
    }

    #[test]
    fn test_cosh_zero() {
        let c = cosh_decimal(Decimal::ZERO);
        let diff = abs_decimal(c - Decimal::ONE);
        assert!(diff < dec!(0.0001));
    }

    #[test]
    fn test_sinh_cosh_identity() {
        // cosh^2(x) - sinh^2(x) = 1
        let x = dec!(1.5);
        let s = sinh_decimal(x);
        let c = cosh_decimal(x);
        let diff = abs_decimal(c * c - s * s - Decimal::ONE);
        assert!(diff < dec!(0.01), "identity failed: diff = {}", diff);
    }

    #[test]
    fn test_acosh_one() {
        let a = acosh_decimal(Decimal::ONE);
        assert!(abs_decimal(a) < dec!(0.0001));
    }

    #[test]
    fn test_acosh_roundtrip() {
        let x = dec!(2);
        let a = acosh_decimal(x);
        let c = cosh_decimal(a);
        let diff = abs_decimal(c - x);
        assert!(
            diff < dec!(0.01),
            "acosh roundtrip failed: cosh(acosh(2)) = {}",
            c
        );
    }

    #[test]
    fn test_cos_zero() {
        let c = cos_decimal(Decimal::ZERO);
        let diff = abs_decimal(c - Decimal::ONE);
        assert!(diff < dec!(0.0001));
    }

    #[test]
    fn test_cos_pi() {
        let pi = dec!(3.141592653589793);
        let c = cos_decimal(pi);
        let diff = abs_decimal(c + Decimal::ONE);
        assert!(diff < dec!(0.01));
    }

    // --- Serialization ---

    #[test]
    fn test_output_serializable() {
        let input = basic_input();
        let result = optimize_execution(&input).unwrap();
        let json = serde_json::to_string(&result).unwrap();
        assert!(!json.is_empty());
    }

    // --- Single slice edge case ---

    #[test]
    fn test_single_slice() {
        let mut input = basic_input();
        input.num_slices = 1;
        let result = optimize_execution(&input).unwrap();
        assert_eq!(result.result.schedule.len(), 1);
        let diff = abs_decimal(result.result.schedule[0].quantity - dec!(10000));
        assert!(diff < dec!(1));
    }

    // --- Large order stress test ---

    #[test]
    fn test_large_order() {
        let mut input = basic_input();
        input.order_size = dec!(500000);
        input.num_slices = 50;
        let result = optimize_execution(&input).unwrap();
        let total: Decimal = result.result.schedule.iter().map(|s| s.quantity).sum();
        let diff = abs_decimal(total - dec!(500000));
        assert!(diff < dec!(10));
    }

    #[test]
    fn test_all_strategies_produce_output() {
        for strat in &[
            ExecutionStrategy::TWAP,
            ExecutionStrategy::VWAP,
            ExecutionStrategy::IS,
            ExecutionStrategy::POV,
        ] {
            let mut input = basic_input();
            input.execution_strategy = strat.clone();
            let result = optimize_execution(&input);
            assert!(result.is_ok(), "strategy {:?} failed", strat);
        }
    }
}
