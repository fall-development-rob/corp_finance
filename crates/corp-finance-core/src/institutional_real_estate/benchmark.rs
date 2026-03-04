use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{with_metadata, ComputationOutput, Money, Rate};
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Helper: Newton's method sqrt (20 iterations)
// ---------------------------------------------------------------------------

fn decimal_sqrt(val: Decimal) -> Decimal {
    if val <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    let mut x = val;
    let two = dec!(2);
    for _ in 0..20 {
        x = (x + val / x) / two;
    }
    x
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A single quarterly return period for NCREIF-style attribution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuarterlyReturn {
    /// Period label, e.g. "2024-Q1"
    pub period: String,
    /// Market value at the start of the quarter
    pub beginning_value: Money,
    /// Market value at the end of the quarter
    pub ending_value: Money,
    /// Net operating income earned during the quarter
    pub noi: Money,
    /// Capital expenditures during the quarter
    pub capex: Money,
}

/// A benchmark return observation for a single period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkReturn {
    /// Period label, e.g. "2024-Q1"
    pub period: String,
    /// Total return for the period
    pub total_return: Rate,
    /// Income component of the return
    pub income_return: Rate,
    /// Appreciation component of the return
    pub appreciation_return: Rate,
}

/// Sector-level weights and returns for Brinson attribution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectorWeight {
    /// Sector name, e.g. "Office", "Retail", "Industrial"
    pub sector: String,
    /// Property/fund weight in this sector
    pub property_weight: Rate,
    /// Benchmark weight in this sector
    pub benchmark_weight: Rate,
    /// Property/fund return in this sector
    pub property_return: Rate,
    /// Benchmark return in this sector
    pub benchmark_return: Rate,
}

/// A single period entry for property index construction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexPeriod {
    /// Period label
    pub period: String,
    /// Appraised / market value at end of period
    pub ending_value: Money,
    /// Net operating income for the period
    pub noi: Money,
    /// Capital expenditures for the period
    pub capex: Money,
}

// ---------------------------------------------------------------------------
// NCREIF Attribution
// ---------------------------------------------------------------------------

/// Input for NCREIF NPI-style return attribution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NcreifAttributionInput {
    /// Quarterly return data
    pub quarterly_returns: Vec<QuarterlyReturn>,
    /// Loan-to-value ratio for leverage adjustment (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ltv: Option<Rate>,
    /// Cost of debt for leverage adjustment (annual, optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_of_debt: Option<Rate>,
}

/// Decomposed return for a single quarter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuarterlyAttribution {
    pub period: String,
    pub income_return: Rate,
    pub appreciation_return: Rate,
    pub total_return: Rate,
}

/// Output from NCREIF attribution analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NcreifAttributionOutput {
    /// Per-quarter decomposition
    pub quarterly_attributions: Vec<QuarterlyAttribution>,
    /// Annual chain-linked total return
    pub annual_total_return: Rate,
    /// Annual chain-linked income return
    pub annual_income_return: Rate,
    /// Annual chain-linked appreciation return
    pub annual_appreciation_return: Rate,
    /// Levered return (if LTV and cost_of_debt provided)
    pub levered_return: Option<Rate>,
}

/// Decompose total return into NCREIF NPI components (income + appreciation),
/// chain-link quarterly returns to annual, and optionally compute levered return.
///
/// RE-CONTRACT-010: Returns decompose into income + appreciation (+ leverage).
pub fn ncreif_attribution(
    input: &NcreifAttributionInput,
) -> CorpFinanceResult<ComputationOutput<NcreifAttributionOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    if input.quarterly_returns.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one quarterly return period is required".into(),
        ));
    }

    // Validate each quarter
    for qr in &input.quarterly_returns {
        if qr.beginning_value <= Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("beginning_value ({})", qr.period),
                reason: "Beginning market value must be positive".into(),
            });
        }
    }

    // Decompose each quarter
    let mut quarterly_attributions = Vec::with_capacity(input.quarterly_returns.len());
    let mut chain_total = Decimal::ONE;
    let mut chain_income = Decimal::ONE;
    let mut chain_appreciation = Decimal::ONE;

    for qr in &input.quarterly_returns {
        let bmv = qr.beginning_value;
        let income_return = qr.noi / bmv;
        let appreciation_return = (qr.ending_value - bmv - qr.capex) / bmv;
        let total_return = income_return + appreciation_return;

        chain_total *= Decimal::ONE + total_return;
        chain_income *= Decimal::ONE + income_return;
        chain_appreciation *= Decimal::ONE + appreciation_return;

        quarterly_attributions.push(QuarterlyAttribution {
            period: qr.period.clone(),
            income_return,
            appreciation_return,
            total_return,
        });
    }

    let annual_total_return = chain_total - Decimal::ONE;
    let annual_income_return = chain_income - Decimal::ONE;
    let annual_appreciation_return = chain_appreciation - Decimal::ONE;

    if input.quarterly_returns.len() != 4 {
        warnings.push(format!(
            "Chain-linking {} quarters (expected 4 for annual return)",
            input.quarterly_returns.len()
        ));
    }

    // Leverage adjustment
    let levered_return = match (input.ltv, input.cost_of_debt) {
        (Some(ltv), Some(cod)) => {
            if ltv >= Decimal::ONE {
                return Err(CorpFinanceError::InvalidInput {
                    field: "ltv".into(),
                    reason: "LTV must be less than 1.0 (100%)".into(),
                });
            }
            let equity_fraction = Decimal::ONE - ltv;
            let levered = annual_total_return + (annual_total_return - cod) * ltv / equity_fraction;
            Some(levered)
        }
        (Some(_), None) | (None, Some(_)) => {
            warnings.push("Both ltv and cost_of_debt are required for leverage adjustment".into());
            None
        }
        _ => None,
    };

    let output = NcreifAttributionOutput {
        quarterly_attributions,
        annual_total_return,
        annual_income_return,
        annual_appreciation_return,
        levered_return,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "NCREIF NPI Return Attribution (income + appreciation decomposition, quarterly chain-linking)",
        &serde_json::json!({
            "quarters": input.quarterly_returns.len(),
            "leverage_adjusted": levered_return.is_some(),
        }),
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// ODCE Comparison
// ---------------------------------------------------------------------------

/// Input for ODCE-style benchmark comparison.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OdceComparisonInput {
    /// Property/fund periodic returns
    pub property_returns: Vec<Rate>,
    /// Benchmark (ODCE index) periodic returns — must match length of property_returns
    pub index_returns: Vec<Rate>,
    /// Sector-level data for Brinson attribution (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sector_weights: Option<Vec<SectorWeight>>,
}

/// Brinson-style real estate sector attribution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectorAttribution {
    pub sector: String,
    pub allocation_effect: Rate,
    pub selection_effect: Rate,
    pub interaction_effect: Rate,
    pub total_effect: Rate,
}

/// Output from ODCE comparison.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OdceComparisonOutput {
    /// Average property return over all periods
    pub avg_property_return: Rate,
    /// Average benchmark return over all periods
    pub avg_index_return: Rate,
    /// Average excess return
    pub avg_excess_return: Rate,
    /// Tracking error (std dev of excess returns)
    pub tracking_error: Rate,
    /// Information ratio = avg_excess / tracking_error
    pub information_ratio: Decimal,
    /// Per-period excess returns
    pub excess_returns: Vec<Rate>,
    /// Sector-level Brinson attribution (if sector data provided)
    pub sector_attribution: Option<Vec<SectorAttribution>>,
}

/// Compare property/fund returns against the ODCE index.
/// Computes excess return, tracking error, information ratio,
/// and optional Brinson-style sector attribution.
pub fn odce_comparison(
    input: &OdceComparisonInput,
) -> CorpFinanceResult<ComputationOutput<OdceComparisonOutput>> {
    let start = Instant::now();
    let warnings: Vec<String> = Vec::new();

    let n = input.property_returns.len();
    if n == 0 {
        return Err(CorpFinanceError::InsufficientData(
            "At least one period of returns is required".into(),
        ));
    }
    if n != input.index_returns.len() {
        return Err(CorpFinanceError::InvalidInput {
            field: "index_returns".into(),
            reason: format!(
                "Property returns has {} periods but index returns has {}",
                n,
                input.index_returns.len()
            ),
        });
    }

    let n_dec = Decimal::from(n as u64);

    // Excess returns
    let excess_returns: Vec<Rate> = input
        .property_returns
        .iter()
        .zip(input.index_returns.iter())
        .map(|(p, b)| *p - *b)
        .collect();

    let sum_excess: Decimal = excess_returns.iter().copied().sum();
    let avg_excess_return = sum_excess / n_dec;

    let avg_property_return: Rate = input.property_returns.iter().copied().sum::<Decimal>() / n_dec;
    let avg_index_return: Rate = input.index_returns.iter().copied().sum::<Decimal>() / n_dec;

    // Tracking error (population std dev of excess returns)
    let tracking_error = if n > 1 {
        let variance: Decimal = excess_returns
            .iter()
            .map(|&er| {
                let diff = er - avg_excess_return;
                diff * diff
            })
            .sum::<Decimal>()
            / Decimal::from((n - 1) as u64);
        decimal_sqrt(variance)
    } else {
        Decimal::ZERO
    };

    // Information ratio
    let information_ratio = if tracking_error > Decimal::ZERO {
        avg_excess_return / tracking_error
    } else {
        Decimal::ZERO
    };

    // Brinson-style sector attribution
    let sector_attribution = input.sector_weights.as_ref().map(|sectors| {
        // Total benchmark return = sum(bw_i * br_i)
        let total_benchmark_return: Decimal = sectors
            .iter()
            .map(|s| s.benchmark_weight * s.benchmark_return)
            .sum();

        sectors
            .iter()
            .map(|s| {
                // Allocation effect: (wp_i - wb_i) * (rb_i - Rb)
                let allocation_effect = (s.property_weight - s.benchmark_weight)
                    * (s.benchmark_return - total_benchmark_return);

                // Selection effect: wb_i * (rp_i - rb_i)
                let selection_effect =
                    s.benchmark_weight * (s.property_return - s.benchmark_return);

                // Interaction effect: (wp_i - wb_i) * (rp_i - rb_i)
                let interaction_effect = (s.property_weight - s.benchmark_weight)
                    * (s.property_return - s.benchmark_return);

                let total_effect = allocation_effect + selection_effect + interaction_effect;

                SectorAttribution {
                    sector: s.sector.clone(),
                    allocation_effect,
                    selection_effect,
                    interaction_effect,
                    total_effect,
                }
            })
            .collect()
    });

    let output = OdceComparisonOutput {
        avg_property_return,
        avg_index_return,
        avg_excess_return,
        tracking_error,
        information_ratio,
        excess_returns,
        sector_attribution,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "ODCE Benchmark Comparison (excess return, tracking error, information ratio, Brinson attribution)",
        &serde_json::json!({
            "periods": n,
            "has_sector_attribution": input.sector_weights.is_some(),
        }),
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Property Index
// ---------------------------------------------------------------------------

/// Input for constructing a property return index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyIndexInput {
    /// Ordered periods (first period's ending_value acts as base for the next period)
    pub periods: Vec<IndexPeriod>,
    /// Starting market value (beginning value for the first period)
    pub initial_value: Money,
}

/// A single period in the computed index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEntry {
    pub period: String,
    pub period_return: Rate,
    pub cumulative_index: Decimal,
}

/// Rolling return statistics over different horizons.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollingStats {
    /// Rolling 1-period returns (all periods)
    pub rolling_1_period: Vec<Rate>,
    /// Rolling 3-period annualized return (if enough data)
    pub rolling_3_period: Option<Vec<Rate>>,
    /// Rolling 5-period annualized return (if enough data)
    pub rolling_5_period: Option<Vec<Rate>>,
}

/// Output from property index construction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyIndexOutput {
    /// Per-period return and cumulative index
    pub index_series: Vec<IndexEntry>,
    /// Total cumulative return over all periods
    pub cumulative_return: Rate,
    /// Annualized volatility of period returns
    pub volatility: Rate,
    /// Maximum drawdown from any peak to subsequent trough
    pub max_drawdown: Rate,
    /// Rolling return statistics
    pub rolling_stats: RollingStats,
}

/// Construct a return index from periodic appraisal values, NOI, and capex.
///
/// period_return = (end_value - begin_value - capex + noi) / begin_value
/// Cumulative index chain-linked from base = 100.
pub fn property_index(
    input: &PropertyIndexInput,
) -> CorpFinanceResult<ComputationOutput<PropertyIndexOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    if input.periods.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one period is required".into(),
        ));
    }
    if input.initial_value <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "initial_value".into(),
            reason: "Initial value must be positive".into(),
        });
    }

    let base_index = dec!(100);
    let mut cumulative = base_index;
    let mut begin_value = input.initial_value;
    let mut period_returns: Vec<Rate> = Vec::with_capacity(input.periods.len());
    let mut index_series: Vec<IndexEntry> = Vec::with_capacity(input.periods.len());

    for ip in &input.periods {
        if begin_value <= Decimal::ZERO {
            return Err(CorpFinanceError::DivisionByZero {
                context: format!("begin_value is zero at period {}", ip.period),
            });
        }
        let period_return = (ip.ending_value - begin_value - ip.capex + ip.noi) / begin_value;
        cumulative *= Decimal::ONE + period_return;
        period_returns.push(period_return);
        index_series.push(IndexEntry {
            period: ip.period.clone(),
            period_return,
            cumulative_index: cumulative,
        });
        begin_value = ip.ending_value;
    }

    let cumulative_return = cumulative / base_index - Decimal::ONE;

    // Volatility (sample std dev of period returns)
    let n = period_returns.len();
    let volatility = if n > 1 {
        let n_dec = Decimal::from(n as u64);
        let mean: Rate = period_returns.iter().copied().sum::<Decimal>() / n_dec;
        let var: Decimal = period_returns
            .iter()
            .map(|&r| {
                let d = r - mean;
                d * d
            })
            .sum::<Decimal>()
            / Decimal::from((n - 1) as u64);
        decimal_sqrt(var)
    } else {
        warnings.push("Cannot compute volatility with fewer than 2 periods".into());
        Decimal::ZERO
    };

    // Maximum drawdown
    let mut peak = base_index;
    let mut max_drawdown = Decimal::ZERO;
    // rebuild cumulative for drawdown (start from base)
    let mut running = base_index;
    for &r in &period_returns {
        running *= Decimal::ONE + r;
        if running > peak {
            peak = running;
        }
        let drawdown = (peak - running) / peak;
        if drawdown > max_drawdown {
            max_drawdown = drawdown;
        }
    }

    // Rolling returns
    let rolling_1_period = period_returns.clone();

    let rolling_3_period = if n >= 3 {
        let mut v = Vec::with_capacity(n - 2);
        for i in 2..n {
            let mut chain = Decimal::ONE;
            for &pr in &period_returns[(i - 2)..=i] {
                chain *= Decimal::ONE + pr;
            }
            v.push(chain - Decimal::ONE);
        }
        Some(v)
    } else {
        None
    };

    let rolling_5_period = if n >= 5 {
        let mut v = Vec::with_capacity(n - 4);
        for i in 4..n {
            let mut chain = Decimal::ONE;
            for &pr in &period_returns[(i - 4)..=i] {
                chain *= Decimal::ONE + pr;
            }
            v.push(chain - Decimal::ONE);
        }
        Some(v)
    } else {
        None
    };

    let rolling_stats = RollingStats {
        rolling_1_period,
        rolling_3_period,
        rolling_5_period,
    };

    let output = PropertyIndexOutput {
        index_series,
        cumulative_return,
        volatility,
        max_drawdown,
        rolling_stats,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Property Return Index (chain-linked, rolling returns, max drawdown)",
        &serde_json::json!({
            "periods": n,
            "base_index": 100,
        }),
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Relative Value
// ---------------------------------------------------------------------------

/// Valuation assessment label.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValueScore {
    /// Price-to-replacement > 1.1x
    Premium,
    /// Price-to-replacement 0.9x to 1.1x
    Fair,
    /// Price-to-replacement < 0.9x
    Discount,
}

/// Input for relative value analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelativeValueInput {
    /// Subject property cap rate
    pub property_cap_rate: Rate,
    /// Benchmark average cap rate for the sector/market
    pub benchmark_cap_rate: Rate,
    /// Risk-free rate (e.g. 10-year Treasury yield)
    pub risk_free_rate: Rate,
    /// Price per square foot of the subject property
    pub price_per_sf: Money,
    /// Estimated replacement cost per square foot
    pub replacement_cost_per_sf: Money,
}

/// Output from relative value analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelativeValueOutput {
    /// Cap rate spread vs benchmark average
    pub cap_rate_spread_to_benchmark: Rate,
    /// Cap rate spread vs risk-free rate
    pub cap_rate_spread_to_risk_free: Rate,
    /// Implied risk premium = property cap rate - risk-free rate
    pub implied_risk_premium: Rate,
    /// Price per SF / Replacement cost per SF
    pub price_to_replacement_ratio: Decimal,
    /// Valuation assessment (Premium / Fair / Discount)
    pub value_score: ValueScore,
    /// Summary narrative
    pub summary: String,
}

/// Compute relative value metrics: cap-rate spreads, implied risk premium,
/// price-to-replacement ratio, and a qualitative valuation score.
pub fn relative_value(
    input: &RelativeValueInput,
) -> CorpFinanceResult<ComputationOutput<RelativeValueOutput>> {
    let start = Instant::now();
    let warnings: Vec<String> = Vec::new();

    if input.replacement_cost_per_sf <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "replacement_cost_per_sf".into(),
            reason: "Replacement cost per SF must be positive".into(),
        });
    }
    if input.price_per_sf < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "price_per_sf".into(),
            reason: "Price per SF must be non-negative".into(),
        });
    }

    let cap_rate_spread_to_benchmark = input.property_cap_rate - input.benchmark_cap_rate;
    let cap_rate_spread_to_risk_free = input.property_cap_rate - input.risk_free_rate;
    let implied_risk_premium = input.property_cap_rate - input.risk_free_rate;

    let price_to_replacement_ratio = input.price_per_sf / input.replacement_cost_per_sf;

    let threshold_premium = dec!(1.1);
    let threshold_discount = dec!(0.9);

    let value_score = if price_to_replacement_ratio > threshold_premium {
        ValueScore::Premium
    } else if price_to_replacement_ratio < threshold_discount {
        ValueScore::Discount
    } else {
        ValueScore::Fair
    };

    let summary = format!(
        "Cap rate {:.2}% vs benchmark {:.2}% (spread {}{:.0}bps). \
         Price/SF ${:.0} vs replacement ${:.0} ({:.2}x). Assessed: {:?}.",
        input.property_cap_rate * dec!(100),
        input.benchmark_cap_rate * dec!(100),
        if cap_rate_spread_to_benchmark >= Decimal::ZERO {
            "+"
        } else {
            ""
        },
        cap_rate_spread_to_benchmark * dec!(10000),
        input.price_per_sf,
        input.replacement_cost_per_sf,
        price_to_replacement_ratio,
        value_score,
    );

    let output = RelativeValueOutput {
        cap_rate_spread_to_benchmark,
        cap_rate_spread_to_risk_free,
        implied_risk_premium,
        price_to_replacement_ratio,
        value_score,
        summary,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Relative Value Analysis (cap rate spreads, risk premium, replacement cost ratio)",
        &serde_json::json!({
            "property_cap_rate": input.property_cap_rate.to_string(),
            "benchmark_cap_rate": input.benchmark_cap_rate.to_string(),
            "risk_free_rate": input.risk_free_rate.to_string(),
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

    // -----------------------------------------------------------------------
    // Helper
    // -----------------------------------------------------------------------

    fn approx_eq(a: Decimal, b: Decimal, tol: Decimal) -> bool {
        let diff = if a > b { a - b } else { b - a };
        diff <= tol
    }

    fn qr(period: &str, bv: Decimal, ev: Decimal, noi: Decimal, capex: Decimal) -> QuarterlyReturn {
        QuarterlyReturn {
            period: period.to_string(),
            beginning_value: bv,
            ending_value: ev,
            noi,
            capex,
        }
    }

    // -----------------------------------------------------------------------
    // ncreif_attribution
    // -----------------------------------------------------------------------

    #[test]
    fn test_ncreif_single_quarter() {
        let input = NcreifAttributionInput {
            quarterly_returns: vec![qr("2024-Q1", dec!(1000), dec!(1020), dec!(15), dec!(5))],
            ltv: None,
            cost_of_debt: None,
        };
        let result = ncreif_attribution(&input).unwrap();
        let out = &result.result;
        // income = 15/1000 = 0.015
        assert_eq!(out.quarterly_attributions[0].income_return, dec!(0.015));
        // appreciation = (1020 - 1000 - 5) / 1000 = 0.015
        assert_eq!(
            out.quarterly_attributions[0].appreciation_return,
            dec!(0.015)
        );
        // total = 0.03
        assert_eq!(out.quarterly_attributions[0].total_return, dec!(0.03));
    }

    #[test]
    fn test_ncreif_return_decomposition_contract() {
        // RE-CONTRACT-010: total = income + appreciation
        let input = NcreifAttributionInput {
            quarterly_returns: vec![
                qr("2024-Q1", dec!(1000), dec!(1030), dec!(12), dec!(3)),
                qr("2024-Q2", dec!(1030), dec!(1050), dec!(13), dec!(4)),
            ],
            ltv: None,
            cost_of_debt: None,
        };
        let result = ncreif_attribution(&input).unwrap();
        for qa in &result.result.quarterly_attributions {
            let sum = qa.income_return + qa.appreciation_return;
            assert!(
                approx_eq(qa.total_return, sum, dec!(0.000001)),
                "Contract violation: total {} != income {} + appreciation {}",
                qa.total_return,
                qa.income_return,
                qa.appreciation_return,
            );
        }
    }

    #[test]
    fn test_ncreif_four_quarter_chain_linking() {
        let quarters = vec![
            qr("2024-Q1", dec!(1000), dec!(1010), dec!(12), dec!(2)),
            qr("2024-Q2", dec!(1010), dec!(1025), dec!(13), dec!(3)),
            qr("2024-Q3", dec!(1025), dec!(1040), dec!(13), dec!(3)),
            qr("2024-Q4", dec!(1040), dec!(1060), dec!(14), dec!(4)),
        ];
        let input = NcreifAttributionInput {
            quarterly_returns: quarters.clone(),
            ltv: None,
            cost_of_debt: None,
        };
        let result = ncreif_attribution(&input).unwrap();

        // Manual chain: (1+r1)(1+r2)(1+r3)(1+r4) - 1
        let mut chain = Decimal::ONE;
        for q in &quarters {
            let r = (q.noi + q.ending_value - q.beginning_value - q.capex) / q.beginning_value;
            chain = chain * (Decimal::ONE + r);
        }
        let expected = chain - Decimal::ONE;
        assert!(approx_eq(
            result.result.annual_total_return,
            expected,
            dec!(0.000001)
        ));
    }

    #[test]
    fn test_ncreif_leverage_adjustment() {
        let input = NcreifAttributionInput {
            quarterly_returns: vec![qr("2024-Q1", dec!(1000), dec!(1020), dec!(15), dec!(5))],
            ltv: Some(dec!(0.60)),
            cost_of_debt: Some(dec!(0.05)),
        };
        let result = ncreif_attribution(&input).unwrap();
        // unlevered = 0.03, cod = 0.05
        // levered = 0.03 + (0.03 - 0.05) * 0.60 / 0.40 = 0.03 + (-0.02)(1.5) = 0.03 - 0.03 = 0.0
        let levered = result.result.levered_return.unwrap();
        assert!(approx_eq(levered, Decimal::ZERO, dec!(0.000001)));
    }

    #[test]
    fn test_ncreif_leverage_positive() {
        let input = NcreifAttributionInput {
            quarterly_returns: vec![qr("2024-Q1", dec!(1000), dec!(1060), dec!(20), dec!(5))],
            ltv: Some(dec!(0.50)),
            cost_of_debt: Some(dec!(0.04)),
        };
        let result = ncreif_attribution(&input).unwrap();
        // unlevered = (20 + 1060 - 1000 - 5)/1000 = 0.075
        // levered = 0.075 + (0.075 - 0.04) * 0.5/0.5 = 0.075 + 0.035 = 0.11
        let levered = result.result.levered_return.unwrap();
        assert!(approx_eq(levered, dec!(0.11), dec!(0.000001)));
    }

    #[test]
    fn test_ncreif_empty_quarters_error() {
        let input = NcreifAttributionInput {
            quarterly_returns: vec![],
            ltv: None,
            cost_of_debt: None,
        };
        assert!(ncreif_attribution(&input).is_err());
    }

    #[test]
    fn test_ncreif_zero_beginning_value_error() {
        let input = NcreifAttributionInput {
            quarterly_returns: vec![qr("2024-Q1", Decimal::ZERO, dec!(100), dec!(5), dec!(1))],
            ltv: None,
            cost_of_debt: None,
        };
        assert!(ncreif_attribution(&input).is_err());
    }

    #[test]
    fn test_ncreif_ltv_at_100_pct_error() {
        let input = NcreifAttributionInput {
            quarterly_returns: vec![qr("2024-Q1", dec!(1000), dec!(1020), dec!(15), dec!(5))],
            ltv: Some(Decimal::ONE),
            cost_of_debt: Some(dec!(0.05)),
        };
        assert!(ncreif_attribution(&input).is_err());
    }

    #[test]
    fn test_ncreif_partial_leverage_warning() {
        let input = NcreifAttributionInput {
            quarterly_returns: vec![qr("2024-Q1", dec!(1000), dec!(1020), dec!(15), dec!(5))],
            ltv: Some(dec!(0.50)),
            cost_of_debt: None,
        };
        let result = ncreif_attribution(&input).unwrap();
        assert!(result.result.levered_return.is_none());
        assert!(!result.warnings.is_empty());
    }

    #[test]
    fn test_ncreif_non_four_quarter_warning() {
        let input = NcreifAttributionInput {
            quarterly_returns: vec![
                qr("2024-Q1", dec!(1000), dec!(1010), dec!(10), dec!(2)),
                qr("2024-Q2", dec!(1010), dec!(1025), dec!(11), dec!(3)),
            ],
            ltv: None,
            cost_of_debt: None,
        };
        let result = ncreif_attribution(&input).unwrap();
        assert!(result.warnings.iter().any(|w| w.contains("2 quarters")));
    }

    // -----------------------------------------------------------------------
    // odce_comparison
    // -----------------------------------------------------------------------

    #[test]
    fn test_odce_basic_excess_return() {
        let input = OdceComparisonInput {
            property_returns: vec![dec!(0.03), dec!(0.04), dec!(0.02), dec!(0.05)],
            index_returns: vec![dec!(0.02), dec!(0.03), dec!(0.025), dec!(0.04)],
            sector_weights: None,
        };
        let result = odce_comparison(&input).unwrap();
        // excess = [0.01, 0.01, -0.005, 0.01]
        assert_eq!(result.result.excess_returns[0], dec!(0.01));
        assert_eq!(result.result.excess_returns[2], dec!(-0.005));
    }

    #[test]
    fn test_odce_avg_excess() {
        let input = OdceComparisonInput {
            property_returns: vec![dec!(0.03), dec!(0.04)],
            index_returns: vec![dec!(0.02), dec!(0.03)],
            sector_weights: None,
        };
        let result = odce_comparison(&input).unwrap();
        // avg excess = (0.01 + 0.01) / 2 = 0.01
        assert_eq!(result.result.avg_excess_return, dec!(0.01));
    }

    #[test]
    fn test_odce_tracking_error() {
        let input = OdceComparisonInput {
            property_returns: vec![dec!(0.05), dec!(0.03), dec!(0.04), dec!(0.02)],
            index_returns: vec![dec!(0.03), dec!(0.03), dec!(0.03), dec!(0.03)],
            sector_weights: None,
        };
        let result = odce_comparison(&input).unwrap();
        // excess = [0.02, 0.00, 0.01, -0.01], mean = 0.005
        // var = ((0.015^2 + 0.005^2 + 0.005^2 + 0.015^2)) / 3
        //     = (0.000225 + 0.000025 + 0.000025 + 0.000225) / 3 = 0.0005 / 3
        assert!(result.result.tracking_error > Decimal::ZERO);
    }

    #[test]
    fn test_odce_information_ratio_sign() {
        let input = OdceComparisonInput {
            property_returns: vec![dec!(0.05), dec!(0.06), dec!(0.04), dec!(0.07)],
            index_returns: vec![dec!(0.03), dec!(0.04), dec!(0.035), dec!(0.05)],
            sector_weights: None,
        };
        let result = odce_comparison(&input).unwrap();
        // Positive outperformance with variation => positive IR
        assert!(result.result.information_ratio > Decimal::ZERO);
        assert!(result.result.tracking_error > Decimal::ZERO);
    }

    #[test]
    fn test_odce_sector_attribution() {
        let sectors = vec![
            SectorWeight {
                sector: "Office".into(),
                property_weight: dec!(0.40),
                benchmark_weight: dec!(0.30),
                property_return: dec!(0.05),
                benchmark_return: dec!(0.04),
            },
            SectorWeight {
                sector: "Industrial".into(),
                property_weight: dec!(0.60),
                benchmark_weight: dec!(0.70),
                property_return: dec!(0.06),
                benchmark_return: dec!(0.05),
            },
        ];
        let input = OdceComparisonInput {
            property_returns: vec![dec!(0.055)],
            index_returns: vec![dec!(0.047)],
            sector_weights: Some(sectors),
        };
        let result = odce_comparison(&input).unwrap();
        let attr = result.result.sector_attribution.as_ref().unwrap();
        assert_eq!(attr.len(), 2);
        // Each sector total_effect = allocation + selection + interaction
        for sa in attr {
            let sum = sa.allocation_effect + sa.selection_effect + sa.interaction_effect;
            assert!(approx_eq(sa.total_effect, sum, dec!(0.000001)));
        }
    }

    #[test]
    fn test_odce_empty_returns_error() {
        let input = OdceComparisonInput {
            property_returns: vec![],
            index_returns: vec![],
            sector_weights: None,
        };
        assert!(odce_comparison(&input).is_err());
    }

    #[test]
    fn test_odce_mismatched_lengths_error() {
        let input = OdceComparisonInput {
            property_returns: vec![dec!(0.03), dec!(0.04)],
            index_returns: vec![dec!(0.02)],
            sector_weights: None,
        };
        assert!(odce_comparison(&input).is_err());
    }

    #[test]
    fn test_odce_single_period_zero_tracking_error() {
        let input = OdceComparisonInput {
            property_returns: vec![dec!(0.05)],
            index_returns: vec![dec!(0.03)],
            sector_weights: None,
        };
        let result = odce_comparison(&input).unwrap();
        // Only one period => tracking error = 0
        assert_eq!(result.result.tracking_error, Decimal::ZERO);
    }

    #[test]
    fn test_odce_zero_excess_returns() {
        let input = OdceComparisonInput {
            property_returns: vec![dec!(0.03), dec!(0.04)],
            index_returns: vec![dec!(0.03), dec!(0.04)],
            sector_weights: None,
        };
        let result = odce_comparison(&input).unwrap();
        assert_eq!(result.result.avg_excess_return, Decimal::ZERO);
        assert_eq!(result.result.information_ratio, Decimal::ZERO);
    }

    // -----------------------------------------------------------------------
    // property_index
    // -----------------------------------------------------------------------

    fn ip(period: &str, ev: Decimal, noi: Decimal, capex: Decimal) -> IndexPeriod {
        IndexPeriod {
            period: period.to_string(),
            ending_value: ev,
            noi,
            capex,
        }
    }

    #[test]
    fn test_property_index_single_period() {
        let input = PropertyIndexInput {
            initial_value: dec!(1000),
            periods: vec![ip("Q1", dec!(1020), dec!(15), dec!(5))],
        };
        let result = property_index(&input).unwrap();
        // return = (1020 - 1000 - 5 + 15) / 1000 = 30/1000 = 0.03
        assert_eq!(result.result.index_series[0].period_return, dec!(0.03));
        // index = 100 * 1.03 = 103
        assert_eq!(result.result.index_series[0].cumulative_index, dec!(103));
    }

    #[test]
    fn test_property_index_cumulative() {
        let input = PropertyIndexInput {
            initial_value: dec!(1000),
            periods: vec![
                ip("Q1", dec!(1020), dec!(15), dec!(5)),
                ip("Q2", dec!(1040), dec!(16), dec!(6)),
            ],
        };
        let result = property_index(&input).unwrap();
        // Q1: return = 0.03, index = 103
        // Q2 begin = 1020, return = (1040-1020-6+16)/1020 = 30/1020
        let q2_ret = dec!(30) / dec!(1020);
        let expected_idx = dec!(103) * (Decimal::ONE + q2_ret);
        assert!(approx_eq(
            result.result.index_series[1].cumulative_index,
            expected_idx,
            dec!(0.001)
        ));
    }

    #[test]
    fn test_property_index_cumulative_return() {
        let input = PropertyIndexInput {
            initial_value: dec!(1000),
            periods: vec![
                ip("Q1", dec!(1030), dec!(12), dec!(2)),
                ip("Q2", dec!(1060), dec!(13), dec!(3)),
            ],
        };
        let result = property_index(&input).unwrap();
        // cumulative_return = final_index / 100 - 1
        let final_idx = result.result.index_series.last().unwrap().cumulative_index;
        let expected = final_idx / dec!(100) - Decimal::ONE;
        assert!(approx_eq(
            result.result.cumulative_return,
            expected,
            dec!(0.000001)
        ));
    }

    #[test]
    fn test_property_index_volatility_positive() {
        let input = PropertyIndexInput {
            initial_value: dec!(1000),
            periods: vec![
                ip("Q1", dec!(1050), dec!(12), dec!(2)),
                ip("Q2", dec!(1030), dec!(10), dec!(5)),
                ip("Q3", dec!(1080), dec!(14), dec!(3)),
            ],
        };
        let result = property_index(&input).unwrap();
        assert!(result.result.volatility > Decimal::ZERO);
    }

    #[test]
    fn test_property_index_max_drawdown() {
        // Deliberate drawdown: Q2 drops below Q1 peak
        let input = PropertyIndexInput {
            initial_value: dec!(1000),
            periods: vec![
                ip("Q1", dec!(1100), dec!(10), dec!(0)), // big gain
                ip("Q2", dec!(1000), dec!(10), dec!(0)), // drop
                ip("Q3", dec!(1050), dec!(10), dec!(0)), // partial recovery
            ],
        };
        let result = property_index(&input).unwrap();
        assert!(result.result.max_drawdown > Decimal::ZERO);
    }

    #[test]
    fn test_property_index_no_drawdown() {
        let input = PropertyIndexInput {
            initial_value: dec!(1000),
            periods: vec![
                ip("Q1", dec!(1010), dec!(10), dec!(0)),
                ip("Q2", dec!(1025), dec!(10), dec!(0)),
                ip("Q3", dec!(1040), dec!(10), dec!(0)),
            ],
        };
        let result = property_index(&input).unwrap();
        // Monotonically increasing => max drawdown = 0
        assert_eq!(result.result.max_drawdown, Decimal::ZERO);
    }

    #[test]
    fn test_property_index_rolling_3() {
        let input = PropertyIndexInput {
            initial_value: dec!(1000),
            periods: vec![
                ip("Q1", dec!(1010), dec!(10), dec!(0)),
                ip("Q2", dec!(1025), dec!(10), dec!(0)),
                ip("Q3", dec!(1040), dec!(10), dec!(0)),
            ],
        };
        let result = property_index(&input).unwrap();
        let rolling3 = result
            .result
            .rolling_stats
            .rolling_3_period
            .as_ref()
            .unwrap();
        assert_eq!(rolling3.len(), 1);
    }

    #[test]
    fn test_property_index_rolling_5_none_if_few_periods() {
        let input = PropertyIndexInput {
            initial_value: dec!(1000),
            periods: vec![
                ip("Q1", dec!(1010), dec!(10), dec!(0)),
                ip("Q2", dec!(1020), dec!(10), dec!(0)),
                ip("Q3", dec!(1030), dec!(10), dec!(0)),
            ],
        };
        let result = property_index(&input).unwrap();
        assert!(result.result.rolling_stats.rolling_5_period.is_none());
    }

    #[test]
    fn test_property_index_empty_error() {
        let input = PropertyIndexInput {
            initial_value: dec!(1000),
            periods: vec![],
        };
        assert!(property_index(&input).is_err());
    }

    #[test]
    fn test_property_index_zero_initial_error() {
        let input = PropertyIndexInput {
            initial_value: Decimal::ZERO,
            periods: vec![ip("Q1", dec!(100), dec!(5), dec!(1))],
        };
        assert!(property_index(&input).is_err());
    }

    // -----------------------------------------------------------------------
    // relative_value
    // -----------------------------------------------------------------------

    #[test]
    fn test_relative_value_basic() {
        let input = RelativeValueInput {
            property_cap_rate: dec!(0.055),
            benchmark_cap_rate: dec!(0.050),
            risk_free_rate: dec!(0.040),
            price_per_sf: dec!(400),
            replacement_cost_per_sf: dec!(380),
        };
        let result = relative_value(&input).unwrap();
        assert_eq!(result.result.cap_rate_spread_to_benchmark, dec!(0.005));
        assert_eq!(result.result.implied_risk_premium, dec!(0.015));
    }

    #[test]
    fn test_relative_value_premium_score() {
        let input = RelativeValueInput {
            property_cap_rate: dec!(0.04),
            benchmark_cap_rate: dec!(0.05),
            risk_free_rate: dec!(0.035),
            price_per_sf: dec!(500),
            replacement_cost_per_sf: dec!(400),
        };
        let result = relative_value(&input).unwrap();
        // 500/400 = 1.25 > 1.1
        assert_eq!(result.result.value_score, ValueScore::Premium);
        assert_eq!(result.result.price_to_replacement_ratio, dec!(1.25));
    }

    #[test]
    fn test_relative_value_discount_score() {
        let input = RelativeValueInput {
            property_cap_rate: dec!(0.07),
            benchmark_cap_rate: dec!(0.05),
            risk_free_rate: dec!(0.04),
            price_per_sf: dec!(300),
            replacement_cost_per_sf: dec!(400),
        };
        let result = relative_value(&input).unwrap();
        // 300/400 = 0.75 < 0.9
        assert_eq!(result.result.value_score, ValueScore::Discount);
    }

    #[test]
    fn test_relative_value_fair_score() {
        let input = RelativeValueInput {
            property_cap_rate: dec!(0.05),
            benchmark_cap_rate: dec!(0.05),
            risk_free_rate: dec!(0.04),
            price_per_sf: dec!(400),
            replacement_cost_per_sf: dec!(400),
        };
        let result = relative_value(&input).unwrap();
        // 400/400 = 1.0, between 0.9 and 1.1
        assert_eq!(result.result.value_score, ValueScore::Fair);
    }

    #[test]
    fn test_relative_value_boundary_premium() {
        // Exactly 1.1x => not premium (must exceed 1.1)
        let input = RelativeValueInput {
            property_cap_rate: dec!(0.05),
            benchmark_cap_rate: dec!(0.05),
            risk_free_rate: dec!(0.04),
            price_per_sf: dec!(440),
            replacement_cost_per_sf: dec!(400),
        };
        let result = relative_value(&input).unwrap();
        // 440/400 = 1.1, not > 1.1
        assert_eq!(result.result.value_score, ValueScore::Fair);
    }

    #[test]
    fn test_relative_value_boundary_discount() {
        // Exactly 0.9x => not discount (must be < 0.9)
        let input = RelativeValueInput {
            property_cap_rate: dec!(0.06),
            benchmark_cap_rate: dec!(0.05),
            risk_free_rate: dec!(0.04),
            price_per_sf: dec!(360),
            replacement_cost_per_sf: dec!(400),
        };
        let result = relative_value(&input).unwrap();
        // 360/400 = 0.9, not < 0.9
        assert_eq!(result.result.value_score, ValueScore::Fair);
    }

    #[test]
    fn test_relative_value_negative_spread() {
        let input = RelativeValueInput {
            property_cap_rate: dec!(0.04),
            benchmark_cap_rate: dec!(0.06),
            risk_free_rate: dec!(0.035),
            price_per_sf: dec!(450),
            replacement_cost_per_sf: dec!(400),
        };
        let result = relative_value(&input).unwrap();
        // -0.02 spread
        assert_eq!(result.result.cap_rate_spread_to_benchmark, dec!(-0.02));
    }

    #[test]
    fn test_relative_value_zero_replacement_cost_error() {
        let input = RelativeValueInput {
            property_cap_rate: dec!(0.05),
            benchmark_cap_rate: dec!(0.05),
            risk_free_rate: dec!(0.04),
            price_per_sf: dec!(400),
            replacement_cost_per_sf: Decimal::ZERO,
        };
        assert!(relative_value(&input).is_err());
    }

    #[test]
    fn test_relative_value_summary_contains_data() {
        let input = RelativeValueInput {
            property_cap_rate: dec!(0.055),
            benchmark_cap_rate: dec!(0.050),
            risk_free_rate: dec!(0.040),
            price_per_sf: dec!(400),
            replacement_cost_per_sf: dec!(380),
        };
        let result = relative_value(&input).unwrap();
        assert!(!result.result.summary.is_empty());
        assert!(result.result.summary.contains("benchmark"));
    }

    // -----------------------------------------------------------------------
    // decimal_sqrt helper
    // -----------------------------------------------------------------------

    #[test]
    fn test_sqrt_zero() {
        assert_eq!(decimal_sqrt(Decimal::ZERO), Decimal::ZERO);
    }

    #[test]
    fn test_sqrt_one() {
        assert!(approx_eq(
            decimal_sqrt(Decimal::ONE),
            Decimal::ONE,
            dec!(0.0000001)
        ));
    }

    #[test]
    fn test_sqrt_four() {
        assert!(approx_eq(decimal_sqrt(dec!(4)), dec!(2), dec!(0.0000001)));
    }

    #[test]
    fn test_sqrt_small_value() {
        // sqrt(0.0001) = 0.01
        assert!(approx_eq(
            decimal_sqrt(dec!(0.0001)),
            dec!(0.01),
            dec!(0.0000001)
        ));
    }

    // -----------------------------------------------------------------------
    // Metadata / methodology
    // -----------------------------------------------------------------------

    #[test]
    fn test_ncreif_methodology_populated() {
        let input = NcreifAttributionInput {
            quarterly_returns: vec![qr("2024-Q1", dec!(1000), dec!(1020), dec!(15), dec!(5))],
            ltv: None,
            cost_of_debt: None,
        };
        let result = ncreif_attribution(&input).unwrap();
        assert!(result.methodology.contains("NCREIF"));
    }

    #[test]
    fn test_odce_methodology_populated() {
        let input = OdceComparisonInput {
            property_returns: vec![dec!(0.03)],
            index_returns: vec![dec!(0.02)],
            sector_weights: None,
        };
        let result = odce_comparison(&input).unwrap();
        assert!(result.methodology.contains("ODCE"));
    }

    #[test]
    fn test_property_index_methodology_populated() {
        let input = PropertyIndexInput {
            initial_value: dec!(1000),
            periods: vec![ip("Q1", dec!(1020), dec!(10), dec!(2))],
        };
        let result = property_index(&input).unwrap();
        assert!(result.methodology.contains("Property Return Index"));
    }

    #[test]
    fn test_relative_value_methodology_populated() {
        let input = RelativeValueInput {
            property_cap_rate: dec!(0.05),
            benchmark_cap_rate: dec!(0.05),
            risk_free_rate: dec!(0.04),
            price_per_sf: dec!(400),
            replacement_cost_per_sf: dec!(400),
        };
        let result = relative_value(&input).unwrap();
        assert!(result.methodology.contains("Relative Value"));
    }
}
