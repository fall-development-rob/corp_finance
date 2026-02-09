use clap::Args;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::input;

/// Arguments for Sharpe ratio calculation
#[derive(Args)]
pub struct SharpeArgs {
    /// Path to JSON/CSV file with return data
    #[arg(long)]
    pub input: Option<String>,

    /// Comma-separated periodic returns (e.g. "0.05,0.02,-0.01,0.03")
    #[arg(long, value_delimiter = ',', allow_hyphen_values = true)]
    pub returns: Option<Vec<Decimal>>,

    /// Risk-free rate (annualised)
    #[arg(long, default_value = "0.042")]
    pub risk_free_rate: Decimal,

    /// Return frequency for annualisation: daily, weekly, monthly, quarterly, annual
    #[arg(long, default_value = "monthly")]
    pub frequency: String,
}

/// Arguments for portfolio risk metrics
#[derive(Args)]
pub struct RiskArgs {
    /// Path to JSON/CSV file with return data
    #[arg(long)]
    pub input: Option<String>,

    /// Comma-separated periodic returns
    #[arg(long, value_delimiter = ',', allow_hyphen_values = true)]
    pub returns: Option<Vec<Decimal>>,

    /// Confidence level for VaR/CVaR (e.g. 0.95 for 95%)
    #[arg(long, default_value = "0.95")]
    pub confidence: Decimal,

    /// Portfolio value for monetary VaR
    #[arg(long)]
    pub portfolio_value: Option<Decimal>,
}

/// Arguments for Kelly criterion position sizing
#[derive(Args)]
pub struct KellyArgs {
    /// Probability of a winning trade (0 to 1)
    #[arg(long)]
    pub win_prob: Decimal,

    /// Win/loss ratio (average win / average loss)
    #[arg(long)]
    pub win_loss_ratio: Decimal,

    /// Kelly fraction (0 to 1, typically 0.5 for half-Kelly)
    #[arg(long, default_value = "0.5")]
    pub fraction: Decimal,

    /// Portfolio value for monetary sizing
    #[arg(long)]
    pub portfolio_value: Option<Decimal>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SharpeOutput {
    sharpe_ratio: Decimal,
    mean_return: Decimal,
    std_dev: Decimal,
    annualised_return: Decimal,
    annualised_std_dev: Decimal,
    risk_free_rate: Decimal,
    num_periods: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct RiskOutput {
    var_pct: Decimal,
    cvar_pct: Decimal,
    var_monetary: Option<Decimal>,
    cvar_monetary: Option<Decimal>,
    confidence: Decimal,
    max_drawdown: Decimal,
    num_periods: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct KellyOutput {
    full_kelly: Decimal,
    fractional_kelly: Decimal,
    fraction_used: Decimal,
    position_size: Option<Decimal>,
    edge: Decimal,
}

fn annualisation_factor(frequency: &str) -> Result<Decimal, Box<dyn std::error::Error>> {
    match frequency.to_lowercase().as_str() {
        "daily" => Ok(dec!(252)),
        "weekly" => Ok(dec!(52)),
        "monthly" => Ok(dec!(12)),
        "quarterly" => Ok(dec!(4)),
        "annual" | "annually" => Ok(dec!(1)),
        _ => Err(format!(
            "Unknown frequency '{}'. Use: daily, weekly, monthly, quarterly, annual",
            frequency
        ).into()),
    }
}

fn mean(values: &[Decimal]) -> Decimal {
    if values.is_empty() {
        return Decimal::ZERO;
    }
    let sum: Decimal = values.iter().sum();
    sum / Decimal::from(values.len() as i64)
}

fn std_dev(values: &[Decimal], avg: Decimal) -> Decimal {
    if values.len() < 2 {
        return Decimal::ZERO;
    }
    let variance: Decimal = values
        .iter()
        .map(|v| {
            let diff = *v - avg;
            diff * diff
        })
        .sum::<Decimal>()
        / Decimal::from((values.len() - 1) as i64);

    // Newton's method for square root
    sqrt_decimal(variance)
}

fn sqrt_decimal(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    let mut guess = x / dec!(2);
    if guess.is_zero() {
        guess = dec!(0.0001);
    }
    for _ in 0..50 {
        let next = (guess + x / guess) / dec!(2);
        if (next - guess).abs() < dec!(0.0000000001) {
            return next;
        }
        guess = next;
    }
    guess
}

fn get_returns(
    input_path: &Option<String>,
    cli_returns: &Option<Vec<Decimal>>,
) -> Result<Vec<Decimal>, Box<dyn std::error::Error>> {
    if let Some(ref path) = input_path {
        let data: Value = input::file::read_json_value(path)?;
        if let Some(arr) = data.as_array() {
            let returns: Vec<Decimal> = arr
                .iter()
                .map(|v| {
                    if let Some(s) = v.as_str() {
                        s.parse::<Decimal>()
                    } else if let Some(n) = v.as_f64() {
                        Ok(Decimal::try_from(n).unwrap_or_default())
                    } else {
                        Err(rust_decimal::Error::from(rust_decimal::Error::Underflow))
                    }
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(returns)
        } else if let Some(obj) = data.as_object() {
            if let Some(arr) = obj.get("returns").and_then(|v| v.as_array()) {
                let returns: Vec<Decimal> = arr
                    .iter()
                    .filter_map(|v| {
                        v.as_str()
                            .and_then(|s| s.parse::<Decimal>().ok())
                            .or_else(|| v.as_f64().map(|n| Decimal::try_from(n).unwrap_or_default()))
                    })
                    .collect();
                Ok(returns)
            } else {
                Err("JSON object must contain a 'returns' array".into())
            }
        } else {
            Err("Expected a JSON array of returns or object with 'returns' key".into())
        }
    } else if let Some(ref rets) = cli_returns {
        Ok(rets.clone())
    } else if let Some(data) = input::stdin::read_stdin()? {
        let parsed: Vec<Decimal> = serde_json::from_value(data)?;
        Ok(parsed)
    } else {
        Err("Provide --returns or --input file or pipe JSON via stdin".into())
    }
}

pub fn run_sharpe(args: SharpeArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let returns = get_returns(&args.input, &args.returns)?;
    if returns.len() < 2 {
        return Err("Sharpe ratio requires at least 2 return observations".into());
    }

    let factor = annualisation_factor(&args.frequency)?;
    let avg = mean(&returns);
    let sd = std_dev(&returns, avg);

    let annualised_return = avg * factor;
    let annualised_sd = sd * sqrt_decimal(factor);
    let rf = args.risk_free_rate;

    let sharpe = if annualised_sd.is_zero() {
        Decimal::ZERO
    } else {
        (annualised_return - rf) / annualised_sd
    };

    let output = SharpeOutput {
        sharpe_ratio: sharpe,
        mean_return: avg,
        std_dev: sd,
        annualised_return,
        annualised_std_dev: annualised_sd,
        risk_free_rate: rf,
        num_periods: returns.len(),
    };

    Ok(serde_json::to_value(output)?)
}

pub fn run_risk(args: RiskArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let mut returns = get_returns(&args.input, &args.returns)?;
    if returns.len() < 2 {
        return Err("Risk metrics require at least 2 return observations".into());
    }

    // Sort ascending for percentile calculations
    returns.sort();

    let n = returns.len();
    let confidence = args.confidence;

    // Historical VaR: percentile at (1 - confidence)
    let var_index = ((dec!(1) - confidence) * Decimal::from(n as i64))
        .floor()
        .to_string()
        .parse::<usize>()
        .unwrap_or(0);
    let var_index = var_index.min(n - 1);

    let var_pct = -returns[var_index]; // VaR is positive (loss)

    // CVaR: average of returns at or below VaR threshold
    let tail = &returns[..=var_index];
    let cvar_pct = if tail.is_empty() {
        var_pct
    } else {
        -mean(tail)
    };

    // Maximum drawdown
    let mut peak = Decimal::ZERO;
    let mut max_dd = Decimal::ZERO;
    let mut cumulative = Decimal::ZERO;
    for r in &returns {
        cumulative += r;
        if cumulative > peak {
            peak = cumulative;
        }
        let drawdown = peak - cumulative;
        if drawdown > max_dd {
            max_dd = drawdown;
        }
    }

    let output = RiskOutput {
        var_pct,
        cvar_pct,
        var_monetary: args.portfolio_value.map(|pv| var_pct * pv),
        cvar_monetary: args.portfolio_value.map(|pv| cvar_pct * pv),
        confidence,
        max_drawdown: max_dd,
        num_periods: n,
    };

    Ok(serde_json::to_value(output)?)
}

pub fn run_kelly(args: KellyArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let p = args.win_prob;
    let wl = args.win_loss_ratio;
    let frac = args.fraction;

    if p <= Decimal::ZERO || p >= Decimal::ONE {
        return Err("--win-prob must be between 0 and 1 (exclusive)".into());
    }
    if wl <= Decimal::ZERO {
        return Err("--win-loss-ratio must be positive".into());
    }
    if frac <= Decimal::ZERO || frac > Decimal::ONE {
        return Err("--fraction must be between 0 (exclusive) and 1 (inclusive)".into());
    }

    // Kelly formula: f* = p - (1-p)/b where b = win/loss ratio
    let q = Decimal::ONE - p;
    let full_kelly = p - q / wl;

    let fractional_kelly = full_kelly * frac;

    let edge = p * wl - q;

    let output = KellyOutput {
        full_kelly,
        fractional_kelly,
        fraction_used: frac,
        position_size: args.portfolio_value.map(|pv| {
            if fractional_kelly > Decimal::ZERO {
                fractional_kelly * pv
            } else {
                Decimal::ZERO
            }
        }),
        edge,
    };

    Ok(serde_json::to_value(output)?)
}
