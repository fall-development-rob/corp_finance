//! Asset-Liability Management (ALM): gap analysis (repricing/maturity),
//! NII simulation, EVE sensitivity, and duration gap metrics.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{with_metadata, ComputationOutput, Money, Rate};
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// When a position can reprice.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RepricingBucket {
    Overnight,
    UpTo1M,
    M1to3,
    M3to6,
    M6to12,
    Y1to2,
    Y2to3,
    Y3to5,
    Y5to10,
    Over10Y,
    NonSensitive,
}

/// When a position matures (same bucket labels as repricing).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MaturityBucket {
    Overnight,
    UpTo1M,
    M1to3,
    M3to6,
    M6to12,
    Y1to2,
    Y2to3,
    Y3to5,
    Y5to10,
    Over10Y,
    NonSensitive,
}

/// Whether a position's rate is fixed or floating.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RateType {
    Fixed,
    Floating,
}

// ---------------------------------------------------------------------------
// Input types
// ---------------------------------------------------------------------------

/// A single balance-sheet or off-balance-sheet position.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlmPosition {
    pub name: String,
    pub balance: Money,
    pub rate: Rate,
    pub repricing_bucket: RepricingBucket,
    pub maturity_bucket: MaturityBucket,
    pub rate_type: RateType,
    /// For floating-rate positions, how much of the market rate change passes
    /// through. 1.0 = perfect pass-through, 0.5 = 50%.
    pub rate_sensitivity: Decimal,
}

/// A rate change applied to a specific repricing bucket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketShift {
    pub bucket: RepricingBucket,
    pub shift_bps: i32,
}

/// A full interest-rate scenario (parallel or non-parallel).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateScenario {
    pub name: String,
    pub shifts: Vec<BucketShift>,
}

/// Top-level input for the ALM analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlmInput {
    pub institution_name: String,
    pub assets: Vec<AlmPosition>,
    pub liabilities: Vec<AlmPosition>,
    #[serde(default)]
    pub off_balance_sheet: Vec<AlmPosition>,
    pub rate_scenarios: Vec<RateScenario>,
    pub current_nii: Money,
}

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapBucket {
    pub bucket: String,
    pub assets: Money,
    pub liabilities: Money,
    pub off_balance_sheet: Money,
    pub gap: Money,
    pub gap_ratio: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CumulativeGap {
    pub bucket: String,
    pub cumulative_gap: Money,
    pub cumulative_gap_ratio: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapAnalysis {
    pub buckets: Vec<GapBucket>,
    pub cumulative_gap: Vec<CumulativeGap>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NiiScenario {
    pub scenario_name: String,
    pub baseline_nii: Money,
    pub projected_nii: Money,
    pub nii_change: Money,
    pub nii_change_pct: Decimal,
    pub at_risk: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EveScenario {
    pub scenario_name: String,
    pub baseline_eve: Money,
    pub stressed_eve: Money,
    pub eve_change: Money,
    pub eve_change_pct: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DurationGap {
    pub asset_duration: Decimal,
    pub liability_duration: Decimal,
    pub duration_gap: Decimal,
    pub interpretation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlmSummary {
    pub total_assets: Money,
    pub total_liabilities: Money,
    pub net_position: Money,
    pub largest_repricing_gap_bucket: String,
    pub worst_nii_scenario: String,
    pub worst_eve_scenario: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlmOutput {
    pub repricing_gap: GapAnalysis,
    pub maturity_gap: GapAnalysis,
    pub nii_sensitivity: Vec<NiiScenario>,
    pub eve_sensitivity: Vec<EveScenario>,
    pub duration_gap: DurationGap,
    pub summary: AlmSummary,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Ordered list of all repricing buckets (including NonSensitive).
const BUCKET_ORDER: &[RepricingBucket] = &[
    RepricingBucket::Overnight,
    RepricingBucket::UpTo1M,
    RepricingBucket::M1to3,
    RepricingBucket::M3to6,
    RepricingBucket::M6to12,
    RepricingBucket::Y1to2,
    RepricingBucket::Y2to3,
    RepricingBucket::Y3to5,
    RepricingBucket::Y5to10,
    RepricingBucket::Over10Y,
    RepricingBucket::NonSensitive,
];

/// NII at-risk threshold: a decline of more than 15% is flagged.
const NII_AT_RISK_THRESHOLD: Decimal = dec!(-0.15);

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn bucket_label(b: &RepricingBucket) -> &'static str {
    match b {
        RepricingBucket::Overnight => "Overnight",
        RepricingBucket::UpTo1M => "Up to 1M",
        RepricingBucket::M1to3 => "1M-3M",
        RepricingBucket::M3to6 => "3M-6M",
        RepricingBucket::M6to12 => "6M-12M",
        RepricingBucket::Y1to2 => "1Y-2Y",
        RepricingBucket::Y2to3 => "2Y-3Y",
        RepricingBucket::Y3to5 => "3Y-5Y",
        RepricingBucket::Y5to10 => "5Y-10Y",
        RepricingBucket::Over10Y => "Over 10Y",
        RepricingBucket::NonSensitive => "Non-Sensitive",
    }
}

fn maturity_to_repricing(m: &MaturityBucket) -> RepricingBucket {
    match m {
        MaturityBucket::Overnight => RepricingBucket::Overnight,
        MaturityBucket::UpTo1M => RepricingBucket::UpTo1M,
        MaturityBucket::M1to3 => RepricingBucket::M1to3,
        MaturityBucket::M3to6 => RepricingBucket::M3to6,
        MaturityBucket::M6to12 => RepricingBucket::M6to12,
        MaturityBucket::Y1to2 => RepricingBucket::Y1to2,
        MaturityBucket::Y2to3 => RepricingBucket::Y2to3,
        MaturityBucket::Y3to5 => RepricingBucket::Y3to5,
        MaturityBucket::Y5to10 => RepricingBucket::Y5to10,
        MaturityBucket::Over10Y => RepricingBucket::Over10Y,
        MaturityBucket::NonSensitive => RepricingBucket::NonSensitive,
    }
}

/// Midpoint in years for each bucket -- used for EVE PV calculations and
/// as a proxy for modified duration.
fn midpoint_years(b: &RepricingBucket) -> Decimal {
    match b {
        RepricingBucket::Overnight => dec!(0),
        RepricingBucket::UpTo1M => dec!(0.042),
        RepricingBucket::M1to3 => dec!(0.167),
        RepricingBucket::M3to6 => dec!(0.375),
        RepricingBucket::M6to12 => dec!(0.75),
        RepricingBucket::Y1to2 => dec!(1.5),
        RepricingBucket::Y2to3 => dec!(2.5),
        RepricingBucket::Y3to5 => dec!(4),
        RepricingBucket::Y5to10 => dec!(7.5),
        RepricingBucket::Over10Y => dec!(15),
        RepricingBucket::NonSensitive => dec!(0),
    }
}

/// NII time weight: the fraction of the next 12 months during which a rate
/// change in this bucket would affect NII.
fn nii_time_weight(b: &RepricingBucket) -> Decimal {
    match b {
        RepricingBucket::Overnight => dec!(1.0),
        RepricingBucket::UpTo1M => Decimal::from(11) / Decimal::from(12),
        RepricingBucket::M1to3 => Decimal::from(9) / Decimal::from(12),
        RepricingBucket::M3to6 => Decimal::from(6) / Decimal::from(12),
        RepricingBucket::M6to12 => Decimal::from(3) / Decimal::from(12),
        // Buckets beyond 1 year do not reprice within the 12-month horizon
        _ => dec!(0),
    }
}

/// Natural logarithm via the series
///   ln(x) = 2 * sum_{k=0..N} (1/(2k+1)) * u^(2k+1)
/// where u = (x-1)/(x+1).
fn decimal_ln(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    let u = (x - Decimal::ONE) / (x + Decimal::ONE);
    let u_sq = u * u;
    let mut term = u;
    let mut sum = u;
    for k in 1..30u32 {
        term *= u_sq;
        let denom = Decimal::from(2 * k + 1);
        sum += term / denom;
    }
    sum * Decimal::from(2)
}

/// Exponential via Taylor series: exp(x) = sum_{k=0..N} x^k / k!
fn decimal_exp(x: Decimal) -> Decimal {
    let mut term = Decimal::ONE;
    let mut sum = Decimal::ONE;
    for k in 1..30u32 {
        term *= x / Decimal::from(k);
        sum += term;
        if term.abs() < dec!(0.0000000001) {
            break;
        }
    }
    sum
}

/// Compute base^t using exp(t * ln(base)).
fn decimal_pow(base: Decimal, t: Decimal) -> Decimal {
    if t == Decimal::ZERO {
        return Decimal::ONE;
    }
    if base <= Decimal::ZERO {
        return dec!(0.000001);
    }
    if base == Decimal::ONE {
        return Decimal::ONE;
    }
    decimal_exp(t * decimal_ln(base))
}

/// Present value: balance / (1 + rate)^midpoint_years
fn position_pv(balance: Money, rate: Rate, bucket: &RepricingBucket) -> Money {
    let t = midpoint_years(bucket);
    if t == Decimal::ZERO {
        return balance;
    }
    let discount = decimal_pow(Decimal::ONE + rate, t);
    if discount == Decimal::ZERO {
        return balance;
    }
    balance / discount
}

/// Look up the shift (as a decimal) for a given bucket in a scenario.
fn shift_for_bucket(scenario: &RateScenario, bucket: &RepricingBucket) -> Decimal {
    for bs in &scenario.shifts {
        if &bs.bucket == bucket {
            return Decimal::from(bs.shift_bps) / dec!(10000);
        }
    }
    Decimal::ZERO
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_input(input: &AlmInput) -> CorpFinanceResult<()> {
    if input.assets.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "assets".to_string(),
            reason: "At least one asset position is required".to_string(),
        });
    }
    if input.liabilities.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "liabilities".to_string(),
            reason: "At least one liability position is required".to_string(),
        });
    }
    if input.rate_scenarios.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "rate_scenarios".to_string(),
            reason: "At least one rate scenario is required".to_string(),
        });
    }
    for pos in input
        .assets
        .iter()
        .chain(input.liabilities.iter())
        .chain(input.off_balance_sheet.iter())
    {
        if pos.balance < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("position.balance ({})", pos.name),
                reason: "Balance must be non-negative".to_string(),
            });
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Gap analysis (repricing and maturity)
// ---------------------------------------------------------------------------

fn compute_gap_analysis(
    assets: &[AlmPosition],
    liabilities: &[AlmPosition],
    obs: &[AlmPosition],
    total_assets: Money,
    use_repricing: bool,
) -> GapAnalysis {
    let mut buckets_out: Vec<GapBucket> = Vec::new();

    for b in BUCKET_ORDER.iter() {
        let label = bucket_label(b).to_string();

        let asset_sum: Money = assets
            .iter()
            .filter(|p| {
                if use_repricing {
                    &p.repricing_bucket == b
                } else {
                    maturity_to_repricing(&p.maturity_bucket) == *b
                }
            })
            .map(|p| p.balance)
            .sum();

        let liab_sum: Money = liabilities
            .iter()
            .filter(|p| {
                if use_repricing {
                    &p.repricing_bucket == b
                } else {
                    maturity_to_repricing(&p.maturity_bucket) == *b
                }
            })
            .map(|p| p.balance)
            .sum();

        let obs_sum: Money = obs
            .iter()
            .filter(|p| {
                if use_repricing {
                    &p.repricing_bucket == b
                } else {
                    maturity_to_repricing(&p.maturity_bucket) == *b
                }
            })
            .map(|p| p.balance)
            .sum();

        let gap = asset_sum - liab_sum + obs_sum;
        let gap_ratio = if total_assets > Decimal::ZERO {
            gap / total_assets
        } else {
            Decimal::ZERO
        };

        buckets_out.push(GapBucket {
            bucket: label,
            assets: asset_sum,
            liabilities: liab_sum,
            off_balance_sheet: obs_sum,
            gap,
            gap_ratio,
        });
    }

    let mut running = Decimal::ZERO;
    let mut cumulative: Vec<CumulativeGap> = Vec::new();
    for gb in &buckets_out {
        running += gb.gap;
        let ratio = if total_assets > Decimal::ZERO {
            running / total_assets
        } else {
            Decimal::ZERO
        };
        cumulative.push(CumulativeGap {
            bucket: gb.bucket.clone(),
            cumulative_gap: running,
            cumulative_gap_ratio: ratio,
        });
    }

    GapAnalysis {
        buckets: buckets_out,
        cumulative_gap: cumulative,
    }
}

// ---------------------------------------------------------------------------
// NII sensitivity
// ---------------------------------------------------------------------------

fn compute_nii_sensitivity(
    assets: &[AlmPosition],
    liabilities: &[AlmPosition],
    obs: &[AlmPosition],
    scenarios: &[RateScenario],
    baseline_nii: Money,
) -> Vec<NiiScenario> {
    scenarios
        .iter()
        .map(|scenario| {
            let mut total_impact = Decimal::ZERO;

            // Assets and OBS: floating positions gain NII when rates rise
            for pos in assets.iter().chain(obs.iter()) {
                if pos.rate_type == RateType::Floating {
                    let shift = shift_for_bucket(scenario, &pos.repricing_bucket);
                    let tw = nii_time_weight(&pos.repricing_bucket);
                    total_impact += pos.balance * shift * pos.rate_sensitivity * tw;
                }
            }

            // Liabilities: floating positions increase expense when rates rise
            for pos in liabilities.iter() {
                if pos.rate_type == RateType::Floating {
                    let shift = shift_for_bucket(scenario, &pos.repricing_bucket);
                    let tw = nii_time_weight(&pos.repricing_bucket);
                    total_impact -= pos.balance * shift * pos.rate_sensitivity * tw;
                }
            }

            let projected = baseline_nii + total_impact;
            let change_pct = if baseline_nii != Decimal::ZERO {
                total_impact / baseline_nii
            } else {
                Decimal::ZERO
            };

            NiiScenario {
                scenario_name: scenario.name.clone(),
                baseline_nii,
                projected_nii: projected,
                nii_change: total_impact,
                nii_change_pct: change_pct,
                at_risk: change_pct < NII_AT_RISK_THRESHOLD,
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// EVE sensitivity
// ---------------------------------------------------------------------------

fn compute_eve_sensitivity(
    assets: &[AlmPosition],
    liabilities: &[AlmPosition],
    obs: &[AlmPosition],
    scenarios: &[RateScenario],
) -> Vec<EveScenario> {
    let base_pv_assets: Money = assets
        .iter()
        .chain(obs.iter())
        .map(|p| position_pv(p.balance, p.rate, &p.repricing_bucket))
        .sum();

    let base_pv_liabilities: Money = liabilities
        .iter()
        .map(|p| position_pv(p.balance, p.rate, &p.repricing_bucket))
        .sum();

    let baseline_eve = base_pv_assets - base_pv_liabilities;

    scenarios
        .iter()
        .map(|scenario| {
            let stressed_pv_assets: Money = assets
                .iter()
                .chain(obs.iter())
                .map(|p| {
                    let shift = shift_for_bucket(scenario, &p.repricing_bucket);
                    position_pv(p.balance, p.rate + shift, &p.repricing_bucket)
                })
                .sum();

            let stressed_pv_liabilities: Money = liabilities
                .iter()
                .map(|p| {
                    let shift = shift_for_bucket(scenario, &p.repricing_bucket);
                    position_pv(p.balance, p.rate + shift, &p.repricing_bucket)
                })
                .sum();

            let stressed_eve = stressed_pv_assets - stressed_pv_liabilities;
            let eve_change = stressed_eve - baseline_eve;
            let eve_change_pct = if baseline_eve != Decimal::ZERO {
                eve_change / baseline_eve
            } else {
                Decimal::ZERO
            };

            EveScenario {
                scenario_name: scenario.name.clone(),
                baseline_eve,
                stressed_eve,
                eve_change,
                eve_change_pct,
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Duration gap
// ---------------------------------------------------------------------------

fn compute_duration_gap(
    assets: &[AlmPosition],
    liabilities: &[AlmPosition],
    total_assets: Money,
    total_liabilities: Money,
) -> DurationGap {
    let asset_duration = if total_assets > Decimal::ZERO {
        assets
            .iter()
            .map(|p| p.balance * midpoint_years(&p.repricing_bucket))
            .sum::<Decimal>()
            / total_assets
    } else {
        Decimal::ZERO
    };

    let liability_duration = if total_liabilities > Decimal::ZERO {
        liabilities
            .iter()
            .map(|p| p.balance * midpoint_years(&p.repricing_bucket))
            .sum::<Decimal>()
            / total_liabilities
    } else {
        Decimal::ZERO
    };

    let leverage_adj = if total_assets > Decimal::ZERO {
        (total_liabilities / total_assets) * liability_duration
    } else {
        Decimal::ZERO
    };

    let gap = asset_duration - leverage_adj;
    let interpretation = if gap > Decimal::ZERO {
        "Asset sensitive".to_string()
    } else if gap < Decimal::ZERO {
        "Liability sensitive".to_string()
    } else {
        "Neutral".to_string()
    };

    DurationGap {
        asset_duration,
        liability_duration,
        duration_gap: gap,
        interpretation,
    }
}

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

/// Perform a comprehensive Asset-Liability Management analysis.
///
/// Computes repricing and maturity gap analysis, NII sensitivity under
/// multiple rate scenarios, Economic Value of Equity (EVE) sensitivity,
/// and duration gap metrics.
pub fn analyze_alm(input: &AlmInput) -> CorpFinanceResult<ComputationOutput<AlmOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    validate_input(input)?;

    let total_assets: Money = input.assets.iter().map(|p| p.balance).sum();
    let total_liabilities: Money = input.liabilities.iter().map(|p| p.balance).sum();
    let net_position = total_assets - total_liabilities;

    if total_assets < total_liabilities {
        warnings.push("Total liabilities exceed total assets (negative equity).".to_string());
    }

    let repricing_gap = compute_gap_analysis(
        &input.assets,
        &input.liabilities,
        &input.off_balance_sheet,
        total_assets,
        true,
    );
    let maturity_gap = compute_gap_analysis(
        &input.assets,
        &input.liabilities,
        &input.off_balance_sheet,
        total_assets,
        false,
    );

    let nii_sensitivity = compute_nii_sensitivity(
        &input.assets,
        &input.liabilities,
        &input.off_balance_sheet,
        &input.rate_scenarios,
        input.current_nii,
    );

    let eve_sensitivity = compute_eve_sensitivity(
        &input.assets,
        &input.liabilities,
        &input.off_balance_sheet,
        &input.rate_scenarios,
    );

    let duration_gap = compute_duration_gap(
        &input.assets,
        &input.liabilities,
        total_assets,
        total_liabilities,
    );

    let largest_repricing_gap_bucket = repricing_gap
        .buckets
        .iter()
        .max_by_key(|gb| gb.gap.abs())
        .map(|gb| gb.bucket.clone())
        .unwrap_or_default();

    let worst_nii_scenario = nii_sensitivity
        .iter()
        .min_by(|a, b| a.nii_change.partial_cmp(&b.nii_change).unwrap())
        .map(|s| s.scenario_name.clone())
        .unwrap_or_default();

    let worst_eve_scenario = eve_sensitivity
        .iter()
        .min_by(|a, b| a.eve_change.partial_cmp(&b.eve_change).unwrap())
        .map(|s| s.scenario_name.clone())
        .unwrap_or_default();

    let summary = AlmSummary {
        total_assets,
        total_liabilities,
        net_position,
        largest_repricing_gap_bucket,
        worst_nii_scenario,
        worst_eve_scenario,
    };

    let output = AlmOutput {
        repricing_gap,
        maturity_gap,
        nii_sensitivity,
        eve_sensitivity,
        duration_gap,
        summary,
    };

    let elapsed = start.elapsed().as_micros() as u64;

    Ok(with_metadata(
        "ALM Gap Analysis, NII Sensitivity, EVE Sensitivity, Duration Gap",
        &serde_json::json!({
            "institution": input.institution_name,
            "nii_at_risk_threshold": NII_AT_RISK_THRESHOLD.to_string(),
            "eve_discount_method": "exp(t * ln(1+r)) Taylor series",
            "duration_proxy": "bucket midpoint years",
            "nii_time_weight": "fraction of 12-month horizon affected"
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

    fn make_position(
        name: &str,
        balance: Decimal,
        rate: Decimal,
        repricing: RepricingBucket,
        maturity: MaturityBucket,
        rate_type: RateType,
        sensitivity: Decimal,
    ) -> AlmPosition {
        AlmPosition {
            name: name.to_string(),
            balance,
            rate,
            repricing_bucket: repricing,
            maturity_bucket: maturity,
            rate_type,
            rate_sensitivity: sensitivity,
        }
    }

    fn parallel_shift(name: &str, bps: i32) -> RateScenario {
        RateScenario {
            name: name.to_string(),
            shifts: BUCKET_ORDER
                .iter()
                .filter(|b| **b != RepricingBucket::NonSensitive)
                .map(|b| BucketShift {
                    bucket: b.clone(),
                    shift_bps: bps,
                })
                .collect(),
        }
    }

    fn simple_input() -> AlmInput {
        AlmInput {
            institution_name: "Test Bank".to_string(),
            assets: vec![
                make_position(
                    "Floating Loans",
                    dec!(500_000),
                    dec!(0.05),
                    RepricingBucket::M3to6,
                    MaturityBucket::Y3to5,
                    RateType::Floating,
                    dec!(1.0),
                ),
                make_position(
                    "Fixed Bonds",
                    dec!(300_000),
                    dec!(0.04),
                    RepricingBucket::Y3to5,
                    MaturityBucket::Y3to5,
                    RateType::Fixed,
                    dec!(0.0),
                ),
            ],
            liabilities: vec![
                make_position(
                    "Demand Deposits",
                    dec!(400_000),
                    dec!(0.01),
                    RepricingBucket::Overnight,
                    MaturityBucket::Overnight,
                    RateType::Floating,
                    dec!(0.5),
                ),
                make_position(
                    "Term Deposits",
                    dec!(200_000),
                    dec!(0.03),
                    RepricingBucket::M6to12,
                    MaturityBucket::Y1to2,
                    RateType::Fixed,
                    dec!(0.0),
                ),
            ],
            off_balance_sheet: vec![],
            rate_scenarios: vec![
                parallel_shift("+100bps", 100),
                parallel_shift("-100bps", -100),
            ],
            current_nii: dec!(25_000),
        }
    }

    // 1. Simple positive repricing gap
    #[test]
    fn test_positive_repricing_gap_in_short_bucket() {
        let input = AlmInput {
            institution_name: "Gap Bank".to_string(),
            assets: vec![make_position(
                "Short Loans",
                dec!(100_000),
                dec!(0.05),
                RepricingBucket::UpTo1M,
                MaturityBucket::UpTo1M,
                RateType::Floating,
                dec!(1.0),
            )],
            liabilities: vec![make_position(
                "Long Deposits",
                dec!(80_000),
                dec!(0.02),
                RepricingBucket::Y1to2,
                MaturityBucket::Y1to2,
                RateType::Fixed,
                dec!(0.0),
            )],
            off_balance_sheet: vec![],
            rate_scenarios: vec![parallel_shift("+100bps", 100)],
            current_nii: dec!(3_000),
        };
        let result = analyze_alm(&input).unwrap();
        let up_to_1m = result
            .result
            .repricing_gap
            .buckets
            .iter()
            .find(|b| b.bucket == "Up to 1M")
            .unwrap();
        assert_eq!(up_to_1m.assets, dec!(100_000));
        assert_eq!(up_to_1m.liabilities, dec!(0));
        assert_eq!(up_to_1m.gap, dec!(100_000));
    }

    // 2. Cumulative gap calculation
    #[test]
    fn test_cumulative_gap() {
        let input = simple_input();
        let result = analyze_alm(&input).unwrap();
        let cum = &result.result.repricing_gap.cumulative_gap;
        let mut running = Decimal::ZERO;
        for (i, cg) in cum.iter().enumerate() {
            running += result.result.repricing_gap.buckets[i].gap;
            assert_eq!(
                cg.cumulative_gap, running,
                "Cumulative gap mismatch at bucket {}",
                cg.bucket
            );
        }
    }

    // 3. NII sensitivity +100bps parallel shift (asset sensitive)
    #[test]
    fn test_nii_sensitivity_positive_shift() {
        let input = simple_input();
        let result = analyze_alm(&input).unwrap();
        let nii_up = result
            .result
            .nii_sensitivity
            .iter()
            .find(|s| s.scenario_name == "+100bps")
            .unwrap();
        // Floating asset (500k, M3to6, beta=1): 500k * 0.01 * 1.0 * 0.5 = 2500
        // Floating liability (400k, O/N, beta=0.5): 400k * 0.01 * 0.5 * 1.0 = 2000
        // Net = 2500 - 2000 = +500
        assert_eq!(nii_up.nii_change, dec!(500));
        assert_eq!(nii_up.projected_nii, dec!(25_500));
    }

    // 4. NII sensitivity -100bps parallel shift
    #[test]
    fn test_nii_sensitivity_negative_shift() {
        let input = simple_input();
        let result = analyze_alm(&input).unwrap();
        let nii_down = result
            .result
            .nii_sensitivity
            .iter()
            .find(|s| s.scenario_name == "-100bps")
            .unwrap();
        assert_eq!(nii_down.nii_change, dec!(-500));
        assert_eq!(nii_down.projected_nii, dec!(24_500));
    }

    // 5. NII floating rate pass-through beta=0.5
    #[test]
    fn test_nii_partial_pass_through() {
        let input = AlmInput {
            institution_name: "Beta Bank".to_string(),
            assets: vec![make_position(
                "Partial Float",
                dec!(100_000),
                dec!(0.04),
                RepricingBucket::Overnight,
                MaturityBucket::Y1to2,
                RateType::Floating,
                dec!(0.5),
            )],
            liabilities: vec![make_position(
                "Fixed Dep",
                dec!(80_000),
                dec!(0.02),
                RepricingBucket::Y5to10,
                MaturityBucket::Y5to10,
                RateType::Fixed,
                dec!(0.0),
            )],
            off_balance_sheet: vec![],
            rate_scenarios: vec![parallel_shift("+200bps", 200)],
            current_nii: dec!(2_000),
        };
        let result = analyze_alm(&input).unwrap();
        let nii = &result.result.nii_sensitivity[0];
        // 100k * 0.02 * 0.5 * 1.0 = 1000
        assert_eq!(nii.nii_change, dec!(1000));
    }

    // 6. EVE baseline calculation
    #[test]
    fn test_eve_baseline() {
        let input = simple_input();
        let result = analyze_alm(&input).unwrap();
        let eve = &result.result.eve_sensitivity[0];
        assert!(
            eve.baseline_eve > Decimal::ZERO,
            "Baseline EVE should be positive"
        );
    }

    // 7. EVE stressed scenario
    #[test]
    fn test_eve_stressed_scenario() {
        let input = simple_input();
        let result = analyze_alm(&input).unwrap();
        let eve_up = result
            .result
            .eve_sensitivity
            .iter()
            .find(|s| s.scenario_name == "+100bps")
            .unwrap();
        assert_ne!(eve_up.eve_change, Decimal::ZERO);
    }

    // 8. Duration gap: asset duration > liability duration
    #[test]
    fn test_duration_gap_asset_sensitive() {
        let input = AlmInput {
            institution_name: "LongAsset Bank".to_string(),
            assets: vec![make_position(
                "Long Bonds",
                dec!(100_000),
                dec!(0.04),
                RepricingBucket::Y5to10,
                MaturityBucket::Y5to10,
                RateType::Fixed,
                dec!(0.0),
            )],
            liabilities: vec![make_position(
                "Short Deposits",
                dec!(80_000),
                dec!(0.01),
                RepricingBucket::Overnight,
                MaturityBucket::Overnight,
                RateType::Floating,
                dec!(1.0),
            )],
            off_balance_sheet: vec![],
            rate_scenarios: vec![parallel_shift("+100bps", 100)],
            current_nii: dec!(3_000),
        };
        let result = analyze_alm(&input).unwrap();
        let dg = &result.result.duration_gap;
        assert_eq!(dg.asset_duration, dec!(7.5));
        assert_eq!(dg.liability_duration, dec!(0));
        assert!(dg.duration_gap > Decimal::ZERO);
        assert_eq!(dg.interpretation, "Asset sensitive");
    }

    // 9. Duration gap: liability duration > asset duration
    #[test]
    fn test_duration_gap_liability_sensitive() {
        let input = AlmInput {
            institution_name: "ShortAsset Bank".to_string(),
            assets: vec![make_position(
                "O/N Loans",
                dec!(100_000),
                dec!(0.03),
                RepricingBucket::Overnight,
                MaturityBucket::Overnight,
                RateType::Floating,
                dec!(1.0),
            )],
            liabilities: vec![make_position(
                "Long Bonds Issued",
                dec!(100_000),
                dec!(0.05),
                RepricingBucket::Y5to10,
                MaturityBucket::Y5to10,
                RateType::Fixed,
                dec!(0.0),
            )],
            off_balance_sheet: vec![],
            rate_scenarios: vec![parallel_shift("+100bps", 100)],
            current_nii: dec!(0),
        };
        let result = analyze_alm(&input).unwrap();
        let dg = &result.result.duration_gap;
        assert_eq!(dg.asset_duration, dec!(0));
        assert_eq!(dg.liability_duration, dec!(7.5));
        assert!(dg.duration_gap < Decimal::ZERO);
        assert_eq!(dg.interpretation, "Liability sensitive");
    }

    // 10. Non-parallel shift (steepener)
    #[test]
    fn test_non_parallel_steepener() {
        let steepener = RateScenario {
            name: "Steepener".to_string(),
            shifts: vec![
                BucketShift {
                    bucket: RepricingBucket::Overnight,
                    shift_bps: 200,
                },
                BucketShift {
                    bucket: RepricingBucket::UpTo1M,
                    shift_bps: 175,
                },
                BucketShift {
                    bucket: RepricingBucket::M1to3,
                    shift_bps: 150,
                },
                BucketShift {
                    bucket: RepricingBucket::M3to6,
                    shift_bps: 100,
                },
                BucketShift {
                    bucket: RepricingBucket::M6to12,
                    shift_bps: 50,
                },
                BucketShift {
                    bucket: RepricingBucket::Y1to2,
                    shift_bps: 25,
                },
                BucketShift {
                    bucket: RepricingBucket::Y2to3,
                    shift_bps: 10,
                },
                BucketShift {
                    bucket: RepricingBucket::Y3to5,
                    shift_bps: 0,
                },
                BucketShift {
                    bucket: RepricingBucket::Y5to10,
                    shift_bps: -10,
                },
                BucketShift {
                    bucket: RepricingBucket::Over10Y,
                    shift_bps: -25,
                },
            ],
        };
        let input = AlmInput {
            institution_name: "Steep Bank".to_string(),
            assets: vec![make_position(
                "Short Float",
                dec!(100_000),
                dec!(0.03),
                RepricingBucket::M3to6,
                MaturityBucket::Y1to2,
                RateType::Floating,
                dec!(1.0),
            )],
            liabilities: vec![make_position(
                "O/N Deposit",
                dec!(80_000),
                dec!(0.01),
                RepricingBucket::Overnight,
                MaturityBucket::Overnight,
                RateType::Floating,
                dec!(1.0),
            )],
            off_balance_sheet: vec![],
            rate_scenarios: vec![steepener],
            current_nii: dec!(2_000),
        };
        let result = analyze_alm(&input).unwrap();
        let nii = &result.result.nii_sensitivity[0];
        // Asset (M3to6, +100bps): 100k * 0.01 * 1.0 * 0.5 = 500
        // Liability (O/N, +200bps): 80k * 0.02 * 1.0 * 1.0 = 1600
        // Net = 500 - 1600 = -1100
        assert_eq!(nii.nii_change, dec!(-1100));
    }

    // 11. Flattener scenario
    #[test]
    fn test_flattener_scenario() {
        let flattener = RateScenario {
            name: "Flattener".to_string(),
            shifts: vec![
                BucketShift {
                    bucket: RepricingBucket::Overnight,
                    shift_bps: -50,
                },
                BucketShift {
                    bucket: RepricingBucket::M3to6,
                    shift_bps: 0,
                },
                BucketShift {
                    bucket: RepricingBucket::Y5to10,
                    shift_bps: 100,
                },
            ],
        };
        let input = AlmInput {
            institution_name: "Flat Bank".to_string(),
            assets: vec![make_position(
                "O/N Loans",
                dec!(100_000),
                dec!(0.03),
                RepricingBucket::Overnight,
                MaturityBucket::M6to12,
                RateType::Floating,
                dec!(1.0),
            )],
            liabilities: vec![make_position(
                "Fixed Dep",
                dec!(80_000),
                dec!(0.02),
                RepricingBucket::Y5to10,
                MaturityBucket::Y5to10,
                RateType::Fixed,
                dec!(0.0),
            )],
            off_balance_sheet: vec![],
            rate_scenarios: vec![flattener],
            current_nii: dec!(1_000),
        };
        let result = analyze_alm(&input).unwrap();
        let nii = &result.result.nii_sensitivity[0];
        // Asset (O/N, -50bps): 100k * -0.005 * 1.0 * 1.0 = -500
        assert_eq!(nii.nii_change, dec!(-500));
    }

    // 12. Off-balance sheet positions in gap
    #[test]
    fn test_off_balance_sheet_in_gap() {
        let input = AlmInput {
            institution_name: "OBS Bank".to_string(),
            assets: vec![make_position(
                "Loans",
                dec!(100_000),
                dec!(0.04),
                RepricingBucket::M1to3,
                MaturityBucket::Y1to2,
                RateType::Floating,
                dec!(1.0),
            )],
            liabilities: vec![make_position(
                "Deposits",
                dec!(80_000),
                dec!(0.01),
                RepricingBucket::M1to3,
                MaturityBucket::M6to12,
                RateType::Floating,
                dec!(0.5),
            )],
            off_balance_sheet: vec![make_position(
                "IRS Receive",
                dec!(50_000),
                dec!(0.03),
                RepricingBucket::M1to3,
                MaturityBucket::Y2to3,
                RateType::Floating,
                dec!(1.0),
            )],
            rate_scenarios: vec![parallel_shift("+100bps", 100)],
            current_nii: dec!(3_000),
        };
        let result = analyze_alm(&input).unwrap();
        let gap = result
            .result
            .repricing_gap
            .buckets
            .iter()
            .find(|b| b.bucket == "1M-3M")
            .unwrap();
        assert_eq!(gap.gap, dec!(70_000));
        assert_eq!(gap.off_balance_sheet, dec!(50_000));
    }

    // 13. Fixed-rate positions do not reprice for NII
    #[test]
    fn test_fixed_rate_no_nii_impact() {
        let input = AlmInput {
            institution_name: "Fixed Bank".to_string(),
            assets: vec![make_position(
                "Fixed Loans",
                dec!(500_000),
                dec!(0.06),
                RepricingBucket::Y3to5,
                MaturityBucket::Y3to5,
                RateType::Fixed,
                dec!(0.0),
            )],
            liabilities: vec![make_position(
                "Fixed Deposits",
                dec!(400_000),
                dec!(0.03),
                RepricingBucket::Y3to5,
                MaturityBucket::Y3to5,
                RateType::Fixed,
                dec!(0.0),
            )],
            off_balance_sheet: vec![],
            rate_scenarios: vec![parallel_shift("+300bps", 300)],
            current_nii: dec!(15_000),
        };
        let result = analyze_alm(&input).unwrap();
        let nii = &result.result.nii_sensitivity[0];
        assert_eq!(nii.nii_change, dec!(0));
        assert_eq!(nii.projected_nii, dec!(15_000));
    }

    // 14. All positions in the same bucket
    #[test]
    fn test_all_positions_same_bucket() {
        let input = AlmInput {
            institution_name: "Single Bucket Bank".to_string(),
            assets: vec![make_position(
                "Loans",
                dec!(200_000),
                dec!(0.05),
                RepricingBucket::M6to12,
                MaturityBucket::M6to12,
                RateType::Floating,
                dec!(1.0),
            )],
            liabilities: vec![make_position(
                "Deposits",
                dec!(150_000),
                dec!(0.02),
                RepricingBucket::M6to12,
                MaturityBucket::M6to12,
                RateType::Floating,
                dec!(1.0),
            )],
            off_balance_sheet: vec![],
            rate_scenarios: vec![parallel_shift("+100bps", 100)],
            current_nii: dec!(6_000),
        };
        let result = analyze_alm(&input).unwrap();
        let gap = result
            .result
            .repricing_gap
            .buckets
            .iter()
            .find(|b| b.bucket == "6M-12M")
            .unwrap();
        assert_eq!(gap.gap, dec!(50_000));

        let nii = &result.result.nii_sensitivity[0];
        // tw for M6to12 = 3/12 = 0.25
        // Asset: 200k * 0.01 * 1.0 * 0.25 = 500
        // Liab:  150k * 0.01 * 1.0 * 0.25 = 375
        // Net = 125
        assert_eq!(nii.nii_change, dec!(125));
    }

    // 15. Summary picks worst NII scenario
    #[test]
    fn test_summary_worst_nii_scenario() {
        let input = simple_input();
        let result = analyze_alm(&input).unwrap();
        assert_eq!(result.result.summary.worst_nii_scenario, "-100bps");
    }

    // 16. Summary picks worst EVE scenario
    #[test]
    fn test_summary_worst_eve_scenario() {
        let input = simple_input();
        let result = analyze_alm(&input).unwrap();
        assert!(
            result.result.summary.worst_eve_scenario == "+100bps"
                || result.result.summary.worst_eve_scenario == "-100bps"
        );
    }

    // 17. Zero-rate positions
    #[test]
    fn test_zero_rate_positions() {
        let input = AlmInput {
            institution_name: "ZeroRate Bank".to_string(),
            assets: vec![make_position(
                "Cash",
                dec!(50_000),
                dec!(0.0),
                RepricingBucket::Overnight,
                MaturityBucket::Overnight,
                RateType::Floating,
                dec!(1.0),
            )],
            liabilities: vec![make_position(
                "Non-Interest Deposits",
                dec!(40_000),
                dec!(0.0),
                RepricingBucket::NonSensitive,
                MaturityBucket::NonSensitive,
                RateType::Fixed,
                dec!(0.0),
            )],
            off_balance_sheet: vec![],
            rate_scenarios: vec![parallel_shift("+100bps", 100)],
            current_nii: dec!(0),
        };
        let result = analyze_alm(&input).unwrap();
        let nii = &result.result.nii_sensitivity[0];
        // Cash O/N floating: 50k * 0.01 * 1.0 * 1.0 = 500
        assert_eq!(nii.nii_change, dec!(500));
    }

    // 18. Multiple scenarios compared
    #[test]
    fn test_multiple_scenarios() {
        let input = AlmInput {
            institution_name: "Multi Bank".to_string(),
            assets: vec![make_position(
                "Float Loans",
                dec!(100_000),
                dec!(0.04),
                RepricingBucket::M1to3,
                MaturityBucket::Y2to3,
                RateType::Floating,
                dec!(1.0),
            )],
            liabilities: vec![make_position(
                "Fixed Dep",
                dec!(80_000),
                dec!(0.02),
                RepricingBucket::Y3to5,
                MaturityBucket::Y3to5,
                RateType::Fixed,
                dec!(0.0),
            )],
            off_balance_sheet: vec![],
            rate_scenarios: vec![
                parallel_shift("+100bps", 100),
                parallel_shift("+200bps", 200),
                parallel_shift("-100bps", -100),
            ],
            current_nii: dec!(2_000),
        };
        let result = analyze_alm(&input).unwrap();
        assert_eq!(result.result.nii_sensitivity.len(), 3);
        assert_eq!(result.result.eve_sensitivity.len(), 3);

        let nii_100 = result
            .result
            .nii_sensitivity
            .iter()
            .find(|s| s.scenario_name == "+100bps")
            .unwrap();
        let nii_200 = result
            .result
            .nii_sensitivity
            .iter()
            .find(|s| s.scenario_name == "+200bps")
            .unwrap();
        assert_eq!(nii_200.nii_change, nii_100.nii_change * dec!(2));
    }

    // 19. NII at-risk flag
    #[test]
    fn test_nii_at_risk_flag() {
        let input = AlmInput {
            institution_name: "AtRisk Bank".to_string(),
            assets: vec![make_position(
                "Fixed Loans",
                dec!(100_000),
                dec!(0.05),
                RepricingBucket::Y5to10,
                MaturityBucket::Y5to10,
                RateType::Fixed,
                dec!(0.0),
            )],
            liabilities: vec![make_position(
                "Float Deposits",
                dec!(100_000),
                dec!(0.01),
                RepricingBucket::Overnight,
                MaturityBucket::Overnight,
                RateType::Floating,
                dec!(1.0),
            )],
            off_balance_sheet: vec![],
            rate_scenarios: vec![parallel_shift("+400bps", 400)],
            current_nii: dec!(4_000),
        };
        let result = analyze_alm(&input).unwrap();
        let nii = &result.result.nii_sensitivity[0];
        // Liab: 100k * 0.04 * 1.0 * 1.0 = -4000, on baseline 4000 = -100%
        assert_eq!(nii.nii_change, dec!(-4_000));
        assert!(nii.at_risk, "Should be flagged as at-risk");
    }

    // 20. Gap ratio relative to total assets
    #[test]
    fn test_gap_ratio() {
        let input = AlmInput {
            institution_name: "Ratio Bank".to_string(),
            assets: vec![make_position(
                "Loans",
                dec!(200_000),
                dec!(0.04),
                RepricingBucket::M3to6,
                MaturityBucket::Y1to2,
                RateType::Floating,
                dec!(1.0),
            )],
            liabilities: vec![make_position(
                "Deposits",
                dec!(100_000),
                dec!(0.02),
                RepricingBucket::M3to6,
                MaturityBucket::M6to12,
                RateType::Floating,
                dec!(0.5),
            )],
            off_balance_sheet: vec![],
            rate_scenarios: vec![parallel_shift("+100bps", 100)],
            current_nii: dec!(4_000),
        };
        let result = analyze_alm(&input).unwrap();
        let gap = result
            .result
            .repricing_gap
            .buckets
            .iter()
            .find(|b| b.bucket == "3M-6M")
            .unwrap();
        assert_eq!(gap.gap, dec!(100_000));
        assert_eq!(gap.gap_ratio, dec!(0.5));
    }

    // 21. Validation: empty assets
    #[test]
    fn test_validation_empty_assets() {
        let input = AlmInput {
            institution_name: "Empty Bank".to_string(),
            assets: vec![],
            liabilities: vec![make_position(
                "Dep",
                dec!(100),
                dec!(0.01),
                RepricingBucket::Overnight,
                MaturityBucket::Overnight,
                RateType::Fixed,
                dec!(0.0),
            )],
            off_balance_sheet: vec![],
            rate_scenarios: vec![parallel_shift("+100bps", 100)],
            current_nii: dec!(0),
        };
        let err = analyze_alm(&input).unwrap_err();
        assert!(
            err.to_string().contains("assets"),
            "Error should mention assets"
        );
    }

    // 22. Validation: empty liabilities
    #[test]
    fn test_validation_empty_liabilities() {
        let input = AlmInput {
            institution_name: "NoLiab Bank".to_string(),
            assets: vec![make_position(
                "Loan",
                dec!(100),
                dec!(0.04),
                RepricingBucket::M1to3,
                MaturityBucket::M1to3,
                RateType::Fixed,
                dec!(0.0),
            )],
            liabilities: vec![],
            off_balance_sheet: vec![],
            rate_scenarios: vec![parallel_shift("+100bps", 100)],
            current_nii: dec!(0),
        };
        let err = analyze_alm(&input).unwrap_err();
        assert!(err.to_string().contains("liabilities"));
    }

    // 23. Validation: empty scenarios
    #[test]
    fn test_validation_empty_scenarios() {
        let input = AlmInput {
            institution_name: "NoScenario Bank".to_string(),
            assets: vec![make_position(
                "Loan",
                dec!(100),
                dec!(0.04),
                RepricingBucket::M1to3,
                MaturityBucket::M1to3,
                RateType::Fixed,
                dec!(0.0),
            )],
            liabilities: vec![make_position(
                "Dep",
                dec!(80),
                dec!(0.01),
                RepricingBucket::Overnight,
                MaturityBucket::Overnight,
                RateType::Fixed,
                dec!(0.0),
            )],
            off_balance_sheet: vec![],
            rate_scenarios: vec![],
            current_nii: dec!(0),
        };
        let err = analyze_alm(&input).unwrap_err();
        assert!(err.to_string().contains("rate_scenarios"));
    }

    // 24. Maturity gap uses maturity bucket, not repricing
    #[test]
    fn test_maturity_gap_uses_maturity_bucket() {
        let input = AlmInput {
            institution_name: "Maturity Bank".to_string(),
            assets: vec![make_position(
                "Loan",
                dec!(100_000),
                dec!(0.05),
                RepricingBucket::M3to6,
                MaturityBucket::Y3to5,
                RateType::Floating,
                dec!(1.0),
            )],
            liabilities: vec![make_position(
                "Dep",
                dec!(80_000),
                dec!(0.02),
                RepricingBucket::Overnight,
                MaturityBucket::Y1to2,
                RateType::Floating,
                dec!(0.5),
            )],
            off_balance_sheet: vec![],
            rate_scenarios: vec![parallel_shift("+100bps", 100)],
            current_nii: dec!(3_000),
        };
        let result = analyze_alm(&input).unwrap();

        // Repricing gap: asset in M3to6
        let rp = result
            .result
            .repricing_gap
            .buckets
            .iter()
            .find(|b| b.bucket == "3M-6M")
            .unwrap();
        assert_eq!(rp.assets, dec!(100_000));
        assert_eq!(rp.liabilities, dec!(0));

        // Maturity gap: asset in Y3to5
        let mat_a = result
            .result
            .maturity_gap
            .buckets
            .iter()
            .find(|b| b.bucket == "3Y-5Y")
            .unwrap();
        assert_eq!(mat_a.assets, dec!(100_000));
        assert_eq!(mat_a.liabilities, dec!(0));

        // Maturity gap: liability in Y1to2
        let mat_l = result
            .result
            .maturity_gap
            .buckets
            .iter()
            .find(|b| b.bucket == "1Y-2Y")
            .unwrap();
        assert_eq!(mat_l.assets, dec!(0));
        assert_eq!(mat_l.liabilities, dec!(80_000));
    }

    // 25. Summary totals
    #[test]
    fn test_summary_totals() {
        let input = simple_input();
        let result = analyze_alm(&input).unwrap();
        let s = &result.result.summary;
        assert_eq!(s.total_assets, dec!(800_000));
        assert_eq!(s.total_liabilities, dec!(600_000));
        assert_eq!(s.net_position, dec!(200_000));
    }

    // 26. Negative balance validation
    #[test]
    fn test_validation_negative_balance() {
        let input = AlmInput {
            institution_name: "Negative Bank".to_string(),
            assets: vec![make_position(
                "Bad Loan",
                dec!(-100),
                dec!(0.04),
                RepricingBucket::M1to3,
                MaturityBucket::M1to3,
                RateType::Fixed,
                dec!(0.0),
            )],
            liabilities: vec![make_position(
                "Dep",
                dec!(80),
                dec!(0.01),
                RepricingBucket::Overnight,
                MaturityBucket::Overnight,
                RateType::Fixed,
                dec!(0.0),
            )],
            off_balance_sheet: vec![],
            rate_scenarios: vec![parallel_shift("+100bps", 100)],
            current_nii: dec!(0),
        };
        let err = analyze_alm(&input).unwrap_err();
        assert!(err.to_string().contains("Balance"));
    }

    // 27. EVE: rates down increases PV of long-duration assets
    #[test]
    fn test_eve_rates_down_increases_long_asset_pv() {
        let input = AlmInput {
            institution_name: "LongAsset EVE Bank".to_string(),
            assets: vec![make_position(
                "Long Bond",
                dec!(100_000),
                dec!(0.05),
                RepricingBucket::Over10Y,
                MaturityBucket::Over10Y,
                RateType::Fixed,
                dec!(0.0),
            )],
            liabilities: vec![make_position(
                "O/N Deposit",
                dec!(80_000),
                dec!(0.01),
                RepricingBucket::Overnight,
                MaturityBucket::Overnight,
                RateType::Floating,
                dec!(1.0),
            )],
            off_balance_sheet: vec![],
            rate_scenarios: vec![parallel_shift("-200bps", -200)],
            current_nii: dec!(4_000),
        };
        let result = analyze_alm(&input).unwrap();
        let eve = &result.result.eve_sensitivity[0];
        assert!(
            eve.eve_change > Decimal::ZERO,
            "EVE should increase when rates fall and assets have longer duration"
        );
    }
}
