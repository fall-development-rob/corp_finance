use rust_decimal::Decimal;
use rust_decimal::MathematicalOps;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::*;
use crate::CorpFinanceResult;

// Re-use ReturnFrequency from sibling module
use super::returns::ReturnFrequency;

/// Input for portfolio risk metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskMetricsInput {
    /// Periodic returns (as decimals)
    pub returns: Vec<Decimal>,
    /// Observation frequency
    pub frequency: ReturnFrequency,
    /// Confidence level (e.g. 0.95 or 0.99)
    pub confidence_level: Rate,
    /// Portfolio value for absolute VaR
    #[serde(skip_serializing_if = "Option::is_none")]
    pub portfolio_value: Option<Money>,
}

/// Output of portfolio risk metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskMetricsOutput {
    /// Parametric VaR (as a positive loss number, relative to portfolio)
    pub var_parametric: Decimal,
    /// Historical VaR (as a positive loss number)
    pub var_historical: Decimal,
    /// Conditional VaR / Expected Shortfall
    pub cvar: Decimal,
    /// Maximum drawdown
    pub max_drawdown: Rate,
    /// Duration of max drawdown in periods
    pub max_drawdown_duration: u32,
    /// Skewness of returns
    pub skewness: Decimal,
    /// Excess kurtosis of returns
    pub kurtosis: Decimal,
    /// Annualised volatility
    pub annualised_volatility: Rate,
}

/// Calculate portfolio risk metrics (VaR, CVaR, drawdown, higher moments).
pub fn calculate_risk_metrics(
    input: &RiskMetricsInput,
) -> CorpFinanceResult<ComputationOutput<RiskMetricsOutput>> {
    let start = Instant::now();
    let warnings: Vec<String> = Vec::new();

    let n = input.returns.len();
    if n < 3 {
        return Err(CorpFinanceError::InsufficientData(
            "At least 3 return observations required for risk metrics".into(),
        ));
    }

    if input.confidence_level <= Decimal::ZERO || input.confidence_level >= Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "confidence_level".into(),
            reason: "Confidence level must be between 0 and 1 (exclusive)".into(),
        });
    }

    let n_dec = Decimal::from(n as i64);
    let periods = input.frequency.periods_per_year();

    // Mean and std dev
    let mean: Decimal = input.returns.iter().sum::<Decimal>() / n_dec;
    let variance = {
        let sum_sq: Decimal = input.returns.iter().map(|r| (r - mean) * (r - mean)).sum();
        sum_sq / Decimal::from((n - 1) as i64)
    };
    let std_dev = sqrt_decimal(variance);
    let annualised_volatility = std_dev * sqrt_decimal(periods);

    // Z-score for confidence level
    let z_score = z_score_for_confidence(input.confidence_level);

    // Parametric VaR = -(mean - z * std_dev) => positive loss
    let var_parametric = -(mean - z_score * std_dev);

    // Historical VaR: sort returns ascending, take percentile
    let mut sorted = input.returns.clone();
    sorted.sort();
    let var_index = ((Decimal::ONE - input.confidence_level) * n_dec)
        .floor()
        .to_string()
        .parse::<usize>()
        .unwrap_or(0);
    let var_index = var_index.min(n - 1);
    let var_historical = -(sorted[var_index]);

    // CVaR = average of returns at or below the VaR threshold
    let threshold = sorted[var_index];
    let tail: Vec<&Decimal> = sorted.iter().filter(|r| **r <= threshold).collect();
    let cvar = if tail.is_empty() {
        var_historical
    } else {
        let tail_sum: Decimal = tail.iter().copied().sum();
        -(tail_sum / Decimal::from(tail.len() as i64))
    };

    // Max drawdown and duration
    let (max_drawdown, max_drawdown_duration) = max_drawdown_with_duration(&input.returns);

    // Skewness: E[(X-mu)^3] / sigma^3 * n / ((n-1)(n-2))
    let skewness = if n < 3 || std_dev.is_zero() {
        Decimal::ZERO
    } else {
        let m3: Decimal = input.returns.iter().map(|r| (r - mean).powd(dec!(3))).sum();
        let adjustment = n_dec / (Decimal::from((n - 1) as i64) * Decimal::from((n - 2) as i64));
        let sigma3 = std_dev * std_dev * std_dev;
        if sigma3.is_zero() {
            Decimal::ZERO
        } else {
            adjustment * m3 / sigma3
        }
    };

    // Excess kurtosis: E[(X-mu)^4] / sigma^4 - 3 (with sample adjustment)
    let kurtosis = if n < 4 || std_dev.is_zero() {
        Decimal::ZERO
    } else {
        let m4: Decimal = input.returns.iter().map(|r| (r - mean).powd(dec!(4))).sum();
        let sigma4 = variance * variance;
        if sigma4.is_zero() {
            Decimal::ZERO
        } else {
            let n1 = Decimal::from((n - 1) as i64);
            let n2 = Decimal::from((n - 2) as i64);
            let n3 = Decimal::from((n - 3) as i64);
            let factor1 = n_dec * (n_dec + Decimal::ONE) / (n1 * n2 * n3);
            let factor2 = dec!(3) * n1 * n1 / (n2 * n3);
            factor1 * (m4 / sigma4) * n_dec - factor2
        }
    };

    let output = RiskMetricsOutput {
        var_parametric,
        var_historical,
        cvar,
        max_drawdown,
        max_drawdown_duration,
        skewness,
        kurtosis,
        annualised_volatility,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Portfolio Risk Metrics (VaR, CVaR, Drawdown, Skewness, Kurtosis)",
        &serde_json::json!({
            "observations": n,
            "confidence_level": input.confidence_level.to_string(),
            "frequency": format!("{:?}", input.frequency),
        }),
        warnings,
        elapsed,
        output,
    ))
}

/// Maximum drawdown and its duration (in periods).
fn max_drawdown_with_duration(returns: &[Decimal]) -> (Rate, u32) {
    let mut cumulative = Decimal::ONE;
    let mut peak = Decimal::ONE;
    let mut max_dd = Decimal::ZERO;
    let mut peak_idx: usize = 0;
    let mut max_dd_start: usize = 0;
    let mut max_dd_end: usize = 0;

    for (i, r) in returns.iter().enumerate() {
        cumulative *= Decimal::ONE + r;
        if cumulative > peak {
            peak = cumulative;
            peak_idx = i;
        }
        if !peak.is_zero() {
            let dd = (peak - cumulative) / peak;
            if dd > max_dd {
                max_dd = dd;
                max_dd_start = peak_idx;
                max_dd_end = i;
            }
        }
    }

    let duration = if max_dd_end >= max_dd_start {
        (max_dd_end - max_dd_start) as u32
    } else {
        0
    };

    (max_dd, duration)
}

/// Approximate z-score for common confidence levels.
/// Uses a lookup for standard values and linear interpolation otherwise.
fn z_score_for_confidence(confidence: Decimal) -> Decimal {
    // Common z-scores
    if confidence == dec!(0.90) {
        return dec!(1.282);
    }
    if confidence == dec!(0.95) {
        return dec!(1.645);
    }
    if confidence == dec!(0.975) {
        return dec!(1.960);
    }
    if confidence == dec!(0.99) {
        return dec!(2.326);
    }
    if confidence == dec!(0.995) {
        return dec!(2.576);
    }

    // Simple approximation for other values:
    // Beasley-Springer-Moro approximation simplified
    // For values between 0.9 and 0.99, linearly interpolate
    if confidence >= dec!(0.95) && confidence <= dec!(0.99) {
        let t = (confidence - dec!(0.95)) / dec!(0.04);
        return dec!(1.645) + t * (dec!(2.326) - dec!(1.645));
    }
    if confidence >= dec!(0.90) && confidence < dec!(0.95) {
        let t = (confidence - dec!(0.90)) / dec!(0.05);
        return dec!(1.282) + t * (dec!(1.645) - dec!(1.282));
    }

    // Fallback: use 1.645 (95%)
    dec!(1.645)
}

fn sqrt_decimal(val: Decimal) -> Decimal {
    if val <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    val.sqrt().unwrap_or(Decimal::ZERO)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn sample_returns() -> Vec<Decimal> {
        vec![
            dec!(0.05),
            dec!(-0.02),
            dec!(0.03),
            dec!(0.01),
            dec!(-0.01),
            dec!(0.04),
            dec!(0.02),
            dec!(-0.03),
            dec!(0.06),
            dec!(0.01),
            dec!(-0.02),
            dec!(0.03),
        ]
    }

    #[test]
    fn test_basic_risk_metrics() {
        let input = RiskMetricsInput {
            returns: sample_returns(),
            frequency: ReturnFrequency::Monthly,
            confidence_level: dec!(0.95),
            portfolio_value: None,
        };
        let result = calculate_risk_metrics(&input).unwrap();
        let out = &result.result;

        assert!(out.var_parametric > Decimal::ZERO);
        assert!(out.var_historical > Decimal::ZERO);
        assert!(out.cvar >= out.var_historical);
        assert!(out.annualised_volatility > Decimal::ZERO);
    }

    #[test]
    fn test_cvar_gte_var() {
        let input = RiskMetricsInput {
            returns: sample_returns(),
            frequency: ReturnFrequency::Monthly,
            confidence_level: dec!(0.95),
            portfolio_value: None,
        };
        let result = calculate_risk_metrics(&input).unwrap();
        assert!(result.result.cvar >= result.result.var_historical);
    }

    #[test]
    fn test_max_drawdown() {
        let returns = vec![dec!(0.10), dec!(-0.20), dec!(0.05), dec!(-0.15)];
        let input = RiskMetricsInput {
            returns,
            frequency: ReturnFrequency::Monthly,
            confidence_level: dec!(0.95),
            portfolio_value: None,
        };
        let result = calculate_risk_metrics(&input).unwrap();
        assert!(result.result.max_drawdown > dec!(0.20));
    }

    #[test]
    fn test_higher_confidence_higher_var() {
        let rets = sample_returns();
        let input95 = RiskMetricsInput {
            returns: rets.clone(),
            frequency: ReturnFrequency::Monthly,
            confidence_level: dec!(0.95),
            portfolio_value: None,
        };
        let input99 = RiskMetricsInput {
            returns: rets,
            frequency: ReturnFrequency::Monthly,
            confidence_level: dec!(0.99),
            portfolio_value: None,
        };
        let r95 = calculate_risk_metrics(&input95).unwrap();
        let r99 = calculate_risk_metrics(&input99).unwrap();
        assert!(r99.result.var_parametric > r95.result.var_parametric);
    }

    #[test]
    fn test_insufficient_data() {
        let input = RiskMetricsInput {
            returns: vec![dec!(0.05), dec!(0.03)],
            frequency: ReturnFrequency::Monthly,
            confidence_level: dec!(0.95),
            portfolio_value: None,
        };
        assert!(calculate_risk_metrics(&input).is_err());
    }

    #[test]
    fn test_invalid_confidence() {
        let input = RiskMetricsInput {
            returns: sample_returns(),
            frequency: ReturnFrequency::Monthly,
            confidence_level: dec!(1.5),
            portfolio_value: None,
        };
        assert!(calculate_risk_metrics(&input).is_err());
    }

    #[test]
    fn test_z_scores() {
        assert_eq!(z_score_for_confidence(dec!(0.95)), dec!(1.645));
        assert_eq!(z_score_for_confidence(dec!(0.99)), dec!(2.326));
    }
}
