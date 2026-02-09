use rust_decimal::Decimal;
use rust_decimal::MathematicalOps;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{with_metadata, ComputationOutput, Currency, Money, Multiple, Rate};
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Financial metrics for a company (target or comparable).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyMetrics {
    /// Enterprise value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enterprise_value: Option<Money>,
    /// Market capitalisation (equity value)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub market_cap: Option<Money>,
    /// Total revenue / sales
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revenue: Option<Money>,
    /// EBITDA
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ebitda: Option<Money>,
    /// EBIT / operating income
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ebit: Option<Money>,
    /// Net income
    #[serde(skip_serializing_if = "Option::is_none")]
    pub net_income: Option<Money>,
    /// Book value of equity
    #[serde(skip_serializing_if = "Option::is_none")]
    pub book_value: Option<Money>,
    /// Earnings per share
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eps: Option<Decimal>,
    /// Expected EPS growth rate (for PEG ratio)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eps_growth_rate: Option<Rate>,
    /// Share price
    #[serde(skip_serializing_if = "Option::is_none")]
    pub share_price: Option<Money>,
}

/// A comparable company with its financial metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparableCompany {
    /// Company name or ticker
    pub name: String,
    /// Financial metrics
    pub metrics: CompanyMetrics,
    /// Include in the analysis (allows easy toggling)
    pub include: bool,
}

/// Types of valuation multiples to compute.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MultipleType {
    EvEbitda,
    EvRevenue,
    EvEbit,
    PriceEarnings,
    PriceBook,
    Peg,
}

impl std::fmt::Display for MultipleType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MultipleType::EvEbitda => write!(f, "EV/EBITDA"),
            MultipleType::EvRevenue => write!(f, "EV/Revenue"),
            MultipleType::EvEbit => write!(f, "EV/EBIT"),
            MultipleType::PriceEarnings => write!(f, "P/E"),
            MultipleType::PriceBook => write!(f, "P/B"),
            MultipleType::Peg => write!(f, "PEG"),
        }
    }
}

/// Input for a trading comparables analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompsInput {
    /// Target company name
    pub target_name: String,
    /// Target company metrics
    pub target_metrics: CompanyMetrics,
    /// List of comparable companies
    pub comparables: Vec<ComparableCompany>,
    /// Which multiples to compute
    pub multiples: Vec<MultipleType>,
    /// Reporting currency
    pub currency: Currency,
}

/// Descriptive statistics for a single multiple across the comp set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultipleStatistics {
    pub multiple_type: MultipleType,
    pub values: Vec<(String, Multiple)>,
    pub mean: Multiple,
    pub median: Multiple,
    pub high: Multiple,
    pub low: Multiple,
    pub std_dev: Multiple,
    pub count: usize,
}

/// An implied valuation for the target from one multiple.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpliedValuation {
    pub multiple_type: MultipleType,
    /// Implied value using median multiple
    pub implied_at_median: Money,
    /// Implied value using mean multiple
    pub implied_at_mean: Money,
    /// Implied value using low multiple
    pub implied_at_low: Money,
    /// Implied value using high multiple
    pub implied_at_high: Money,
    /// The target metric used as the base
    pub target_metric_value: Money,
}

/// Output of a trading comparables analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompsOutput {
    /// Statistics for each requested multiple
    pub multiple_statistics: Vec<MultipleStatistics>,
    /// Implied valuations of the target company
    pub implied_valuations: Vec<ImpliedValuation>,
    /// Number of comparable companies included
    pub companies_included: usize,
    /// Number of comparable companies excluded
    pub companies_excluded: usize,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Run a trading comparables analysis.
pub fn calculate_comps(
    input: &CompsInput,
) -> CorpFinanceResult<ComputationOutput<CompsOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // --- Validate ---
    let included: Vec<&ComparableCompany> =
        input.comparables.iter().filter(|c| c.include).collect();
    let excluded_count = input.comparables.len() - included.len();

    if included.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "No comparable companies included in the analysis".into(),
        ));
    }
    if input.multiples.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "multiples".into(),
            reason: "At least one multiple type must be specified".into(),
        });
    }
    if included.len() < 3 {
        warnings.push(format!(
            "Only {} comparables included; consider adding more for statistical significance",
            included.len()
        ));
    }

    // --- Compute multiples and statistics ---
    let mut multiple_statistics: Vec<MultipleStatistics> = Vec::new();
    let mut implied_valuations: Vec<ImpliedValuation> = Vec::new();

    for mult_type in &input.multiples {
        let values = compute_multiples_for_type(mult_type, &included, &mut warnings);

        if values.is_empty() {
            warnings.push(format!(
                "No comparable companies had sufficient data for {mult_type}"
            ));
            continue;
        }

        let stats = compute_statistics(mult_type.clone(), values);

        // Compute implied valuation for target
        if let Some(implied) =
            compute_implied_valuation(mult_type, &stats, &input.target_metrics, &mut warnings)
        {
            implied_valuations.push(implied);
        }

        multiple_statistics.push(stats);
    }

    if multiple_statistics.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "Could not compute any multiples from the comparable set".into(),
        ));
    }

    let output = CompsOutput {
        multiple_statistics,
        implied_valuations,
        companies_included: included.len(),
        companies_excluded: excluded_count,
    };

    let elapsed = start.elapsed().as_micros() as u64;

    Ok(with_metadata(
        "Trading Comparables Analysis",
        input,
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Extract the multiple value for each comparable that has the required data.
fn compute_multiples_for_type(
    mult_type: &MultipleType,
    companies: &[&ComparableCompany],
    warnings: &mut Vec<String>,
) -> Vec<(String, Multiple)> {
    let mut values = Vec::new();

    for comp in companies {
        let m = &comp.metrics;
        let result = match mult_type {
            MultipleType::EvEbitda => match (m.enterprise_value, m.ebitda) {
                (Some(ev), Some(ebitda)) if ebitda > Decimal::ZERO => Some(ev / ebitda),
                _ => None,
            },
            MultipleType::EvRevenue => match (m.enterprise_value, m.revenue) {
                (Some(ev), Some(rev)) if rev > Decimal::ZERO => Some(ev / rev),
                _ => None,
            },
            MultipleType::EvEbit => match (m.enterprise_value, m.ebit) {
                (Some(ev), Some(ebit)) if ebit > Decimal::ZERO => Some(ev / ebit),
                _ => None,
            },
            MultipleType::PriceEarnings => match (m.market_cap, m.net_income) {
                (Some(mc), Some(ni)) if ni > Decimal::ZERO => Some(mc / ni),
                _ => None,
            },
            MultipleType::PriceBook => match (m.market_cap, m.book_value) {
                (Some(mc), Some(bv)) if bv > Decimal::ZERO => Some(mc / bv),
                _ => None,
            },
            MultipleType::Peg => {
                // PEG = (P/E) / (EPS growth * 100)
                match (m.market_cap, m.net_income, m.eps_growth_rate) {
                    (Some(mc), Some(ni), Some(g)) if ni > Decimal::ZERO && g > Decimal::ZERO => {
                        let pe = mc / ni;
                        let growth_pct = g * dec!(100);
                        if growth_pct > Decimal::ZERO {
                            Some(pe / growth_pct)
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            }
        };

        match result {
            Some(v) => values.push((comp.name.clone(), v)),
            None => {
                warnings.push(format!(
                    "{}: insufficient data for {mult_type}",
                    comp.name
                ));
            }
        }
    }

    values
}

fn compute_statistics(
    multiple_type: MultipleType,
    values: Vec<(String, Multiple)>,
) -> MultipleStatistics {
    let count = values.len();
    let mut sorted_vals: Vec<Multiple> = values.iter().map(|(_, v)| *v).collect();
    sorted_vals.sort();

    let sum: Decimal = sorted_vals.iter().copied().sum();
    let mean = sum / Decimal::from(count as i64);

    let median = if count % 2 == 0 {
        let mid = count / 2;
        (sorted_vals[mid - 1] + sorted_vals[mid]) / dec!(2)
    } else {
        sorted_vals[count / 2]
    };

    let high = sorted_vals[count - 1];
    let low = sorted_vals[0];

    // Standard deviation
    let std_dev = if count > 1 {
        let variance: Decimal = sorted_vals
            .iter()
            .map(|v| {
                let diff = *v - mean;
                diff * diff
            })
            .sum::<Decimal>()
            / Decimal::from((count - 1) as i64); // sample std dev
        sqrt_decimal(variance)
    } else {
        Decimal::ZERO
    };

    MultipleStatistics {
        multiple_type,
        values,
        mean,
        median,
        high,
        low,
        std_dev,
        count,
    }
}

/// Compute implied valuation for the target using the given statistics.
fn compute_implied_valuation(
    mult_type: &MultipleType,
    stats: &MultipleStatistics,
    target: &CompanyMetrics,
    warnings: &mut Vec<String>,
) -> Option<ImpliedValuation> {
    // Determine which target metric to multiply
    let base_value = match mult_type {
        MultipleType::EvEbitda => target.ebitda,
        MultipleType::EvRevenue => target.revenue,
        MultipleType::EvEbit => target.ebit,
        MultipleType::PriceEarnings => target.net_income,
        MultipleType::PriceBook => target.book_value,
        MultipleType::Peg => {
            // For PEG, implied value = PEG * growth * earnings
            match (target.net_income, target.eps_growth_rate) {
                (Some(ni), Some(g)) if g > Decimal::ZERO => {
                    let growth_pct = g * dec!(100);
                    return Some(ImpliedValuation {
                        multiple_type: mult_type.clone(),
                        implied_at_median: ni * stats.median * growth_pct,
                        implied_at_mean: ni * stats.mean * growth_pct,
                        implied_at_low: ni * stats.low * growth_pct,
                        implied_at_high: ni * stats.high * growth_pct,
                        target_metric_value: ni,
                    });
                }
                _ => {
                    warnings.push(format!(
                        "Target missing net_income or eps_growth_rate for PEG implied valuation"
                    ));
                    return None;
                }
            }
        }
    };

    match base_value {
        Some(val) if val > Decimal::ZERO => Some(ImpliedValuation {
            multiple_type: mult_type.clone(),
            implied_at_median: val * stats.median,
            implied_at_mean: val * stats.mean,
            implied_at_low: val * stats.low,
            implied_at_high: val * stats.high,
            target_metric_value: val,
        }),
        _ => {
            warnings.push(format!(
                "Target missing required metric for {mult_type} implied valuation"
            ));
            None
        }
    }
}

/// Integer square root approximation for Decimal via Newton's method.
fn sqrt_decimal(value: Decimal) -> Decimal {
    if value <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    // Use rust_decimal's built-in sqrt
    value.sqrt().unwrap_or(Decimal::ZERO)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn sample_comps_input() -> CompsInput {
        CompsInput {
            target_name: "TargetCo".into(),
            target_metrics: CompanyMetrics {
                enterprise_value: None,
                market_cap: None,
                revenue: Some(dec!(500)),
                ebitda: Some(dec!(125)),
                ebit: Some(dec!(100)),
                net_income: Some(dec!(75)),
                book_value: Some(dec!(300)),
                eps: Some(dec!(2.50)),
                eps_growth_rate: Some(dec!(0.15)),
                share_price: Some(dec!(40)),
            },
            comparables: vec![
                ComparableCompany {
                    name: "CompA".into(),
                    metrics: CompanyMetrics {
                        enterprise_value: Some(dec!(2000)),
                        market_cap: Some(dec!(1600)),
                        revenue: Some(dec!(800)),
                        ebitda: Some(dec!(200)),
                        ebit: Some(dec!(160)),
                        net_income: Some(dec!(120)),
                        book_value: Some(dec!(500)),
                        eps: Some(dec!(3.00)),
                        eps_growth_rate: Some(dec!(0.12)),
                        share_price: Some(dec!(50)),
                    },
                    include: true,
                },
                ComparableCompany {
                    name: "CompB".into(),
                    metrics: CompanyMetrics {
                        enterprise_value: Some(dec!(3000)),
                        market_cap: Some(dec!(2500)),
                        revenue: Some(dec!(1200)),
                        ebitda: Some(dec!(360)),
                        ebit: Some(dec!(300)),
                        net_income: Some(dec!(200)),
                        book_value: Some(dec!(800)),
                        eps: Some(dec!(4.00)),
                        eps_growth_rate: Some(dec!(0.20)),
                        share_price: Some(dec!(60)),
                    },
                    include: true,
                },
                ComparableCompany {
                    name: "CompC".into(),
                    metrics: CompanyMetrics {
                        enterprise_value: Some(dec!(1500)),
                        market_cap: Some(dec!(1200)),
                        revenue: Some(dec!(600)),
                        ebitda: Some(dec!(150)),
                        ebit: Some(dec!(120)),
                        net_income: Some(dec!(90)),
                        book_value: Some(dec!(400)),
                        eps: Some(dec!(2.00)),
                        eps_growth_rate: Some(dec!(0.10)),
                        share_price: Some(dec!(35)),
                    },
                    include: true,
                },
                ComparableCompany {
                    name: "CompD_excluded".into(),
                    metrics: CompanyMetrics {
                        enterprise_value: Some(dec!(5000)),
                        market_cap: Some(dec!(4000)),
                        revenue: Some(dec!(2000)),
                        ebitda: Some(dec!(400)),
                        ebit: Some(dec!(350)),
                        net_income: Some(dec!(250)),
                        book_value: Some(dec!(1000)),
                        eps: Some(dec!(5.00)),
                        eps_growth_rate: Some(dec!(0.08)),
                        share_price: Some(dec!(80)),
                    },
                    include: false,
                },
            ],
            multiples: vec![
                MultipleType::EvEbitda,
                MultipleType::EvRevenue,
                MultipleType::PriceEarnings,
                MultipleType::PriceBook,
            ],
            currency: Currency::USD,
        }
    }

    #[test]
    fn test_basic_comps() {
        let input = sample_comps_input();
        let result = calculate_comps(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.companies_included, 3);
        assert_eq!(out.companies_excluded, 1);
        assert_eq!(out.multiple_statistics.len(), 4);
    }

    #[test]
    fn test_ev_ebitda_multiples() {
        let input = sample_comps_input();
        let result = calculate_comps(&input).unwrap();
        let out = &result.result;

        let ev_ebitda = out
            .multiple_statistics
            .iter()
            .find(|s| s.multiple_type == MultipleType::EvEbitda)
            .unwrap();

        // CompA: 2000/200 = 10x, CompB: 3000/360 = 8.33x, CompC: 1500/150 = 10x
        assert_eq!(ev_ebitda.count, 3);

        // Sorted: ~8.33, 10, 10
        assert!(
            (ev_ebitda.low - dec!(8.33)).abs() < dec!(0.01),
            "Low: expected ~8.33, got {}",
            ev_ebitda.low
        );

        // Median should be 10
        assert!(
            (ev_ebitda.median - dec!(10)).abs() < dec!(0.01),
            "Median: expected ~10, got {}",
            ev_ebitda.median
        );
    }

    #[test]
    fn test_implied_valuation_ev_ebitda() {
        let input = sample_comps_input();
        let result = calculate_comps(&input).unwrap();
        let out = &result.result;

        let implied = out
            .implied_valuations
            .iter()
            .find(|v| v.multiple_type == MultipleType::EvEbitda)
            .unwrap();

        // Target EBITDA = 125
        assert_eq!(implied.target_metric_value, dec!(125));

        // At median 10x: 125 * 10 = 1250
        let ev_ebitda_stats = out
            .multiple_statistics
            .iter()
            .find(|s| s.multiple_type == MultipleType::EvEbitda)
            .unwrap();
        let expected_median = dec!(125) * ev_ebitda_stats.median;
        assert_eq!(implied.implied_at_median, expected_median);
    }

    #[test]
    fn test_excluded_company_not_in_stats() {
        let input = sample_comps_input();
        let result = calculate_comps(&input).unwrap();
        let out = &result.result;

        for stats in &out.multiple_statistics {
            assert!(
                !stats.values.iter().any(|(name, _)| name == "CompD_excluded"),
                "Excluded company should not appear in {}: {:?}",
                stats.multiple_type,
                stats.values
            );
        }
    }

    #[test]
    fn test_missing_data_handled() {
        let mut input = sample_comps_input();
        // Remove EBITDA from CompA
        input.comparables[0].metrics.ebitda = None;

        let result = calculate_comps(&input).unwrap();
        let ev_ebitda = result
            .result
            .multiple_statistics
            .iter()
            .find(|s| s.multiple_type == MultipleType::EvEbitda)
            .unwrap();

        // Only CompB and CompC should contribute
        assert_eq!(ev_ebitda.count, 2);
        assert!(result
            .warnings
            .iter()
            .any(|w| w.contains("CompA") && w.contains("EV/EBITDA")));
    }

    #[test]
    fn test_no_comparables_error() {
        let mut input = sample_comps_input();
        for comp in &mut input.comparables {
            comp.include = false;
        }

        let result = calculate_comps(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_no_multiples_error() {
        let mut input = sample_comps_input();
        input.multiples.clear();

        let result = calculate_comps(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_negative_ebitda_excluded() {
        let mut input = sample_comps_input();
        input.comparables[0].metrics.ebitda = Some(dec!(-50));

        let result = calculate_comps(&input).unwrap();
        let ev_ebitda = result
            .result
            .multiple_statistics
            .iter()
            .find(|s| s.multiple_type == MultipleType::EvEbitda)
            .unwrap();

        // CompA should be excluded due to negative EBITDA
        assert_eq!(ev_ebitda.count, 2);
    }

    #[test]
    fn test_statistics_single_company() {
        let mut input = sample_comps_input();
        input.comparables[1].include = false;
        input.comparables[2].include = false;

        let result = calculate_comps(&input).unwrap();
        let ev_ebitda = result
            .result
            .multiple_statistics
            .iter()
            .find(|s| s.multiple_type == MultipleType::EvEbitda)
            .unwrap();

        assert_eq!(ev_ebitda.count, 1);
        assert_eq!(ev_ebitda.mean, ev_ebitda.median);
        assert_eq!(ev_ebitda.std_dev, Decimal::ZERO);
    }

    #[test]
    fn test_peg_multiple() {
        let mut input = sample_comps_input();
        input.multiples = vec![MultipleType::Peg];

        let result = calculate_comps(&input).unwrap();
        let out = &result.result;

        // Should have PEG statistics
        let peg = out
            .multiple_statistics
            .iter()
            .find(|s| s.multiple_type == MultipleType::Peg)
            .unwrap();

        // CompA: P/E = 1600/120 = 13.33, growth = 12% => PEG = 13.33/12 = 1.11
        // CompB: P/E = 2500/200 = 12.5, growth = 20% => PEG = 12.5/20 = 0.625
        // CompC: P/E = 1200/90 = 13.33, growth = 10% => PEG = 13.33/10 = 1.33
        assert_eq!(peg.count, 3);
        assert!(peg.low < peg.median);
        assert!(peg.median <= peg.high);
    }

    #[test]
    fn test_price_book_multiple() {
        let mut input = sample_comps_input();
        input.multiples = vec![MultipleType::PriceBook];

        let result = calculate_comps(&input).unwrap();
        let pb = result
            .result
            .multiple_statistics
            .iter()
            .find(|s| s.multiple_type == MultipleType::PriceBook)
            .unwrap();

        // CompA: 1600/500=3.2, CompB: 2500/800=3.125, CompC: 1200/400=3.0
        assert_eq!(pb.count, 3);
        assert!(
            (pb.low - dec!(3.0)).abs() < dec!(0.01),
            "P/B low: expected 3.0, got {}",
            pb.low
        );
    }

    #[test]
    fn test_methodology_string() {
        let input = sample_comps_input();
        let result = calculate_comps(&input).unwrap();
        assert_eq!(result.methodology, "Trading Comparables Analysis");
    }

    #[test]
    fn test_few_comparables_warning() {
        let mut input = sample_comps_input();
        input.comparables[1].include = false;
        input.comparables[2].include = false;
        // Only CompA included (1 company)

        let result = calculate_comps(&input).unwrap();
        assert!(result
            .warnings
            .iter()
            .any(|w| w.contains("comparables included")));
    }
}
