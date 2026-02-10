use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{with_metadata, ComputationOutput};
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Input / Output types
// ---------------------------------------------------------------------------

/// Method for loss reserve estimation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReservingMethod {
    ChainLadder,
    BornhuetterFerguson,
    Both,
}

/// Cumulative claims triangle.
///
/// Each row corresponds to an accident year.  Each column is a development
/// period.  Values are cumulative paid (or incurred) claims.  `None` marks
/// cells that have not yet developed (the lower-right portion of the
/// triangle).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimsTriangle {
    /// Accident years, e.g. [2018, 2019, 2020, 2021, 2022].
    pub accident_years: Vec<u32>,
    /// Development periods in years, e.g. [1, 2, 3, 4, 5].
    pub development_periods: Vec<u32>,
    /// Cumulative claim amounts.  `values[i][j]` is the cumulative claims
    /// for accident year `i` at development period `j`.  `None` for cells
    /// that have not yet emerged.
    pub values: Vec<Vec<Option<Decimal>>>,
}

/// Top-level input for loss reserve estimation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReservingInput {
    /// Line of business, e.g. "Auto Liability".
    pub line_of_business: String,
    /// Cumulative claims triangle.
    pub triangle: ClaimsTriangle,
    /// Reserving method to apply.
    pub method: ReservingMethod,
    /// Earned premium by accident year (required for Bornhuetter-Ferguson).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub earned_premium: Option<Vec<Decimal>>,
    /// A priori expected loss ratio (required for BF, e.g. 0.65 = 65%).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_loss_ratio: Option<Decimal>,
    /// Tail factor for development beyond the last observed column.
    /// Defaults to 1.0 (no tail) when `None`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tail_factor: Option<Decimal>,
    /// Discount rate for present-valuing reserves.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discount_rate: Option<Decimal>,
}

// ---------------------------------------------------------------------------
// Result types
// ---------------------------------------------------------------------------

/// A single age-to-age development factor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevelopmentFactor {
    pub from_period: u32,
    pub to_period: u32,
    /// Weighted average link ratio.
    pub factor: Decimal,
    /// Volume-weighted factor: sum(C_{i,j+1}) / sum(C_{i,j}).
    pub volume_weighted: Decimal,
}

/// Ultimate and IBNR for one accident year.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccidentYearUltimate {
    pub accident_year: u32,
    /// Latest diagonal value (paid to date).
    pub paid_to_date: Decimal,
    /// Developed to ultimate.
    pub ultimate: Decimal,
    /// IBNR = ultimate - paid_to_date.
    pub ibnr: Decimal,
}

/// IBNR for one accident year.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccidentYearIbnr {
    pub accident_year: u32,
    pub ibnr: Decimal,
}

/// Chain-ladder specific results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainLadderResult {
    pub development_factors: Vec<DevelopmentFactor>,
    /// Cumulative development factors to ultimate.
    pub cumulative_factors: Vec<Decimal>,
    /// Fully developed triangle (all cells filled).
    pub completed_triangle: Vec<Vec<Decimal>>,
    pub ultimates: Vec<AccidentYearUltimate>,
    pub ibnr_by_year: Vec<AccidentYearIbnr>,
}

/// Bornhuetter-Ferguson specific results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BfResult {
    pub development_factors: Vec<DevelopmentFactor>,
    pub cumulative_factors: Vec<Decimal>,
    /// Expected ultimates = earned_premium * ELR.
    pub expected_ultimates: Vec<Decimal>,
    pub bf_ultimates: Vec<AccidentYearUltimate>,
    pub bf_ibnr_by_year: Vec<AccidentYearIbnr>,
}

/// Selected reserve for one accident year (possibly present-valued).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccidentYearReserve {
    pub accident_year: u32,
    pub method: String,
    pub reserve: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub present_value_reserve: Option<Decimal>,
}

/// High-level summary statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReserveSummary {
    pub total_ibnr: Decimal,
    pub total_paid: Decimal,
    pub total_ultimate: Decimal,
    /// total_ultimate / total_premium (if premium provided).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overall_loss_ratio: Option<Decimal>,
    /// actual_ultimate / expected_ultimate (>1 means under-reserved a priori).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adequacy_ratio: Option<Decimal>,
    /// Present value of total reserves.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub present_value_total: Option<Decimal>,
}

/// Top-level reserving output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReservingOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain_ladder: Option<ChainLadderResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bornhuetter_ferguson: Option<BfResult>,
    pub selected_reserves: Vec<AccidentYearReserve>,
    pub summary: ReserveSummary,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Estimate loss reserves using chain-ladder and/or Bornhuetter-Ferguson
/// methods.
///
/// The function validates the claims triangle, computes age-to-age
/// development factors, projects ultimate losses, and derives IBNR
/// reserves.  When `method == Both`, immature accident years (<=50%
/// developed) use BF and mature years use chain-ladder.
pub fn estimate_reserves(
    input: &ReservingInput,
) -> CorpFinanceResult<ComputationOutput<ReservingOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // ------------------------------------------------------------------
    // 1. Validate
    // ------------------------------------------------------------------
    validate_input(input)?;

    let tri = &input.triangle;
    let n_years = tri.accident_years.len();
    let n_periods = tri.development_periods.len();
    let tail = input.tail_factor.unwrap_or(Decimal::ONE);

    // ------------------------------------------------------------------
    // 2. Compute development factors (shared by CL and BF)
    // ------------------------------------------------------------------
    let dev_factors = compute_development_factors(tri, n_years, n_periods)?;
    let cum_factors = compute_cumulative_factors(&dev_factors, n_periods, tail);

    // ------------------------------------------------------------------
    // 3. Determine latest diagonal (paid-to-date per accident year)
    // ------------------------------------------------------------------
    let latest_diag = latest_diagonal(tri, n_years, n_periods);

    // ------------------------------------------------------------------
    // 4. Chain-ladder
    // ------------------------------------------------------------------
    let run_cl = matches!(
        input.method,
        ReservingMethod::ChainLadder | ReservingMethod::Both
    );
    let cl_result = if run_cl {
        Some(run_chain_ladder(
            tri,
            &dev_factors,
            &cum_factors,
            &latest_diag,
            n_years,
            n_periods,
        ))
    } else {
        None
    };

    // ------------------------------------------------------------------
    // 5. Bornhuetter-Ferguson
    // ------------------------------------------------------------------
    let run_bf = matches!(
        input.method,
        ReservingMethod::BornhuetterFerguson | ReservingMethod::Both
    );
    let bf_result = if run_bf {
        let premium =
            input
                .earned_premium
                .as_ref()
                .ok_or_else(|| CorpFinanceError::InvalidInput {
                    field: "earned_premium".into(),
                    reason: "Earned premium is required for Bornhuetter-Ferguson".into(),
                })?;
        let elr = input
            .expected_loss_ratio
            .ok_or_else(|| CorpFinanceError::InvalidInput {
                field: "expected_loss_ratio".into(),
                reason: "Expected loss ratio is required for Bornhuetter-Ferguson".into(),
            })?;
        Some(run_bornhuetter_ferguson(
            tri,
            &dev_factors,
            &cum_factors,
            &latest_diag,
            premium,
            elr,
            n_years,
            n_periods,
        ))
    } else {
        None
    };

    // ------------------------------------------------------------------
    // 6. Select reserves per accident year
    // ------------------------------------------------------------------
    let selected = select_reserves(
        &input.method,
        &cl_result,
        &bf_result,
        tri,
        input.discount_rate,
        n_years,
        n_periods,
        &mut warnings,
    );

    // ------------------------------------------------------------------
    // 7. Summary
    // ------------------------------------------------------------------
    let total_ibnr = selected.iter().map(|r| r.reserve).sum::<Decimal>();
    let total_paid = latest_diag.iter().map(|(_, v)| *v).sum::<Decimal>();
    let total_ultimate = total_paid + total_ibnr;

    let total_premium: Option<Decimal> = input
        .earned_premium
        .as_ref()
        .map(|p| p.iter().copied().sum());
    let overall_loss_ratio = total_premium
        .filter(|p| *p > Decimal::ZERO)
        .map(|p| total_ultimate / p);

    let adequacy_ratio = if run_bf {
        let expected_total: Decimal = input
            .earned_premium
            .as_ref()
            .map(|p| {
                p.iter().copied().sum::<Decimal>()
                    * input.expected_loss_ratio.unwrap_or(Decimal::ZERO)
            })
            .unwrap_or(Decimal::ZERO);
        if expected_total > Decimal::ZERO {
            Some(total_ultimate / expected_total)
        } else {
            None
        }
    } else {
        None
    };

    let present_value_total = if input.discount_rate.is_some() {
        Some(
            selected
                .iter()
                .filter_map(|r| r.present_value_reserve)
                .sum(),
        )
    } else {
        None
    };

    let summary = ReserveSummary {
        total_ibnr,
        total_paid,
        total_ultimate,
        overall_loss_ratio,
        adequacy_ratio,
        present_value_total,
    };

    let output = ReservingOutput {
        chain_ladder: cl_result,
        bornhuetter_ferguson: bf_result,
        selected_reserves: selected,
        summary,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    let assumptions = serde_json::json!({
        "line_of_business": input.line_of_business,
        "method": format!("{:?}", input.method),
        "tail_factor": tail.to_string(),
        "discount_rate": input.discount_rate.map(|d| d.to_string()),
        "accident_years": tri.accident_years,
        "development_periods": tri.development_periods,
    });

    Ok(with_metadata(
        "Loss Reserving: Chain-Ladder / Bornhuetter-Ferguson",
        &assumptions,
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_input(input: &ReservingInput) -> CorpFinanceResult<()> {
    let tri = &input.triangle;

    if tri.accident_years.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "accident_years".into(),
            reason: "At least one accident year is required".into(),
        });
    }
    if tri.development_periods.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "development_periods".into(),
            reason: "At least one development period is required".into(),
        });
    }
    if tri.values.len() != tri.accident_years.len() {
        return Err(CorpFinanceError::InvalidInput {
            field: "triangle.values".into(),
            reason: format!(
                "Number of rows ({}) must equal number of accident years ({})",
                tri.values.len(),
                tri.accident_years.len()
            ),
        });
    }
    for (i, row) in tri.values.iter().enumerate() {
        if row.len() != tri.development_periods.len() {
            return Err(CorpFinanceError::InvalidInput {
                field: "triangle.values".into(),
                reason: format!(
                    "Row {} has {} columns but {} development periods declared",
                    i,
                    row.len(),
                    tri.development_periods.len()
                ),
            });
        }
    }

    // Every row must have at least one observed (Some) value
    for (i, row) in tri.values.iter().enumerate() {
        if row.iter().all(|v| v.is_none()) {
            return Err(CorpFinanceError::InvalidInput {
                field: "triangle.values".into(),
                reason: format!(
                    "Accident year {} (row {}) has no observed values",
                    tri.accident_years[i], i
                ),
            });
        }
    }

    // The first row (oldest accident year) must have all values present
    // (it defines the fully developed baseline).
    // Relaxed: at least the first value must be present for every row.
    if tri.values[0][0].is_none() {
        return Err(CorpFinanceError::InvalidInput {
            field: "triangle.values".into(),
            reason: "First cell of the triangle (oldest year, first period) must be present".into(),
        });
    }

    // BF-specific: premium count must match accident years
    if matches!(
        input.method,
        ReservingMethod::BornhuetterFerguson | ReservingMethod::Both
    ) {
        if let Some(ref premium) = input.earned_premium {
            if premium.len() != tri.accident_years.len() {
                return Err(CorpFinanceError::InvalidInput {
                    field: "earned_premium".into(),
                    reason: format!(
                        "Earned premium count ({}) must match accident year count ({})",
                        premium.len(),
                        tri.accident_years.len()
                    ),
                });
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Development factors
// ---------------------------------------------------------------------------

/// Compute age-to-age (link) development factors using volume-weighted
/// averages.
fn compute_development_factors(
    tri: &ClaimsTriangle,
    n_years: usize,
    n_periods: usize,
) -> CorpFinanceResult<Vec<DevelopmentFactor>> {
    let mut factors = Vec::with_capacity(n_periods.saturating_sub(1));

    for j in 0..n_periods.saturating_sub(1) {
        let mut sum_next = Decimal::ZERO;
        let mut sum_curr = Decimal::ZERO;

        for i in 0..n_years {
            if let (Some(curr), Some(next)) = (tri.values[i][j], tri.values[i][j + 1]) {
                sum_curr += curr;
                sum_next += next;
            }
        }

        let volume_weighted = if sum_curr > Decimal::ZERO {
            sum_next / sum_curr
        } else {
            Decimal::ONE
        };

        factors.push(DevelopmentFactor {
            from_period: tri.development_periods[j],
            to_period: tri.development_periods[j + 1],
            factor: volume_weighted,
            volume_weighted,
        });
    }

    Ok(factors)
}

/// Compute cumulative development factors to ultimate.
///
/// CDF_j = product(f_k for k = j..last) * tail_factor
fn compute_cumulative_factors(
    dev_factors: &[DevelopmentFactor],
    n_periods: usize,
    tail: Decimal,
) -> Vec<Decimal> {
    // cum_factors[j] is the CDF from development period j to ultimate.
    // The last period's CDF is just the tail factor.
    let mut cum = vec![Decimal::ZERO; n_periods];

    // CDF for the last column = tail_factor
    cum[n_periods - 1] = tail;

    // Work backwards
    for j in (0..n_periods - 1).rev() {
        cum[j] = dev_factors[j].volume_weighted * cum[j + 1];
    }

    cum
}

/// For each accident year, find the latest observed value (the diagonal).
fn latest_diagonal(tri: &ClaimsTriangle, n_years: usize, n_periods: usize) -> Vec<(u32, Decimal)> {
    let mut diag = Vec::with_capacity(n_years);
    for i in 0..n_years {
        let mut latest_val = Decimal::ZERO;
        for j in (0..n_periods).rev() {
            if let Some(v) = tri.values[i][j] {
                latest_val = v;
                break;
            }
        }
        diag.push((tri.accident_years[i], latest_val));
    }
    diag
}

/// Find the column index of the latest observed value for a given row.
fn latest_col_index(tri: &ClaimsTriangle, row: usize, n_periods: usize) -> usize {
    for j in (0..n_periods).rev() {
        if tri.values[row][j].is_some() {
            return j;
        }
    }
    0
}

// ---------------------------------------------------------------------------
// Chain-Ladder
// ---------------------------------------------------------------------------

fn run_chain_ladder(
    tri: &ClaimsTriangle,
    dev_factors: &[DevelopmentFactor],
    cum_factors: &[Decimal],
    latest_diag: &[(u32, Decimal)],
    n_years: usize,
    n_periods: usize,
) -> ChainLadderResult {
    // Build the completed triangle
    let mut completed: Vec<Vec<Decimal>> = Vec::with_capacity(n_years);
    for i in 0..n_years {
        let mut row = Vec::with_capacity(n_periods);
        // Copy observed values
        for j in 0..n_periods {
            if let Some(v) = tri.values[i][j] {
                row.push(v);
            } else {
                // Project: take the previous column value and multiply by
                // the link ratio
                let prev = row[j - 1];
                let factor = dev_factors[j - 1].volume_weighted;
                row.push(prev * factor);
            }
        }
        completed.push(row);
    }

    // Ultimates
    let mut ultimates = Vec::with_capacity(n_years);
    let mut ibnr_by_year = Vec::with_capacity(n_years);

    #[allow(clippy::needless_range_loop)]
    for i in 0..n_years {
        let last_col = latest_col_index(tri, i, n_periods);
        let paid = latest_diag[i].1;
        let ultimate = paid * cum_factors[last_col];
        let ibnr = ultimate - paid;

        ultimates.push(AccidentYearUltimate {
            accident_year: tri.accident_years[i],
            paid_to_date: paid,
            ultimate,
            ibnr,
        });
        ibnr_by_year.push(AccidentYearIbnr {
            accident_year: tri.accident_years[i],
            ibnr,
        });
    }

    ChainLadderResult {
        development_factors: dev_factors.to_vec(),
        cumulative_factors: cum_factors.to_vec(),
        completed_triangle: completed,
        ultimates,
        ibnr_by_year,
    }
}

// ---------------------------------------------------------------------------
// Bornhuetter-Ferguson
// ---------------------------------------------------------------------------

fn run_bornhuetter_ferguson(
    tri: &ClaimsTriangle,
    dev_factors: &[DevelopmentFactor],
    cum_factors: &[Decimal],
    latest_diag: &[(u32, Decimal)],
    premium: &[Decimal],
    elr: Decimal,
    n_years: usize,
    n_periods: usize,
) -> BfResult {
    let expected_ultimates: Vec<Decimal> = premium.iter().map(|p| *p * elr).collect();

    let mut bf_ultimates = Vec::with_capacity(n_years);
    let mut bf_ibnr_by_year = Vec::with_capacity(n_years);

    for i in 0..n_years {
        let last_col = latest_col_index(tri, i, n_periods);
        let cdf = cum_factors[last_col];
        let paid = latest_diag[i].1;

        // % unreported = 1 - 1/CDF
        let pct_unreported = if cdf > Decimal::ZERO {
            Decimal::ONE - Decimal::ONE / cdf
        } else {
            Decimal::ZERO
        };

        let bf_ibnr = expected_ultimates[i] * pct_unreported;
        let bf_ult = paid + bf_ibnr;

        bf_ultimates.push(AccidentYearUltimate {
            accident_year: tri.accident_years[i],
            paid_to_date: paid,
            ultimate: bf_ult,
            ibnr: bf_ibnr,
        });
        bf_ibnr_by_year.push(AccidentYearIbnr {
            accident_year: tri.accident_years[i],
            ibnr: bf_ibnr,
        });
    }

    BfResult {
        development_factors: dev_factors.to_vec(),
        cumulative_factors: cum_factors.to_vec(),
        expected_ultimates,
        bf_ultimates,
        bf_ibnr_by_year,
    }
}

// ---------------------------------------------------------------------------
// Reserve selection & present-valuing
// ---------------------------------------------------------------------------

fn select_reserves(
    method: &ReservingMethod,
    cl: &Option<ChainLadderResult>,
    bf: &Option<BfResult>,
    tri: &ClaimsTriangle,
    discount_rate: Option<Decimal>,
    n_years: usize,
    n_periods: usize,
    warnings: &mut Vec<String>,
) -> Vec<AccidentYearReserve> {
    let mut reserves = Vec::with_capacity(n_years);

    for i in 0..n_years {
        let last_col = latest_col_index(tri, i, n_periods);

        // Maturity measured as the fraction of development periods
        // completed.  This is the standard actuarial heuristic for
        // choosing between CL and BF: years with fewer than half their
        // development periods observed are considered immature.
        let pct_periods_complete = if n_periods > 0 {
            Decimal::from((last_col + 1) as u32) / Decimal::from(n_periods as u32)
        } else {
            Decimal::ONE
        };

        let (reserve, method_name) = match method {
            ReservingMethod::ChainLadder => {
                let cl_ref = cl.as_ref().unwrap();
                (cl_ref.ibnr_by_year[i].ibnr, "ChainLadder")
            }
            ReservingMethod::BornhuetterFerguson => {
                let bf_ref = bf.as_ref().unwrap();
                (bf_ref.bf_ibnr_by_year[i].ibnr, "BornhuetterFerguson")
            }
            ReservingMethod::Both => {
                let cl_ref = cl.as_ref().unwrap();
                let bf_ref = bf.as_ref().unwrap();
                // Use BF for immature years (<=50% developed), CL for mature
                if pct_periods_complete <= dec!(0.50) {
                    (bf_ref.bf_ibnr_by_year[i].ibnr, "BornhuetterFerguson")
                } else {
                    (cl_ref.ibnr_by_year[i].ibnr, "ChainLadder")
                }
            }
        };

        // Present value discount
        let pv_reserve = discount_rate.map(|rate| {
            let remaining_periods = n_periods.saturating_sub(last_col + 1);
            let avg_years = Decimal::from(remaining_periods as u32) / dec!(2);
            discount_value(reserve, rate, avg_years)
        });

        if reserve < Decimal::ZERO {
            warnings.push(format!(
                "Negative IBNR ({}) for accident year {} — this may indicate \
                 over-reserving or data issues",
                reserve, tri.accident_years[i]
            ));
        }

        reserves.push(AccidentYearReserve {
            accident_year: tri.accident_years[i],
            method: method_name.to_string(),
            reserve,
            present_value_reserve: pv_reserve,
        });
    }

    reserves
}

/// Discount a value by (1 + rate)^years using iterative multiplication.
fn discount_value(value: Decimal, rate: Decimal, years: Decimal) -> Decimal {
    if years <= Decimal::ZERO || rate <= Decimal::ZERO {
        return value;
    }

    // For fractional years we split into integer + fractional parts.
    // Integer part: iterative multiplication. Fractional part: linear
    // interpolation between floor and ceil discount factors.
    let whole = years.trunc();
    let frac = years - whole;
    let whole_u32 = whole.to_string().parse::<u32>().unwrap_or(0);

    let base = Decimal::ONE + rate;

    // Compute base^whole iteratively
    let mut factor_whole = Decimal::ONE;
    for _ in 0..whole_u32 {
        factor_whole *= base;
    }

    // Linear interpolation for fractional year
    let factor = if frac > Decimal::ZERO {
        let factor_next = factor_whole * base;
        factor_whole + (factor_next - factor_whole) * frac
    } else {
        factor_whole
    };

    if factor > Decimal::ZERO {
        value / factor
    } else {
        value
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
    // Test helpers
    // -----------------------------------------------------------------------

    /// Standard 5x5 cumulative paid claims triangle.
    ///
    /// Accident years: 2018-2022
    /// Development periods: 1-5
    ///
    /// ```text
    ///        Dev1      Dev2      Dev3      Dev4      Dev5
    /// 2018   1000      1500      1700      1750      1760
    /// 2019   1100      1650      1870      1920      None
    /// 2020   1200      1800      2040      None      None
    /// 2021   1300      1950      None      None      None
    /// 2022   1400      None      None      None      None
    /// ```
    fn triangle_5x5() -> ClaimsTriangle {
        ClaimsTriangle {
            accident_years: vec![2018, 2019, 2020, 2021, 2022],
            development_periods: vec![1, 2, 3, 4, 5],
            values: vec![
                vec![
                    Some(dec!(1000)),
                    Some(dec!(1500)),
                    Some(dec!(1700)),
                    Some(dec!(1750)),
                    Some(dec!(1760)),
                ],
                vec![
                    Some(dec!(1100)),
                    Some(dec!(1650)),
                    Some(dec!(1870)),
                    Some(dec!(1920)),
                    None,
                ],
                vec![
                    Some(dec!(1200)),
                    Some(dec!(1800)),
                    Some(dec!(2040)),
                    None,
                    None,
                ],
                vec![Some(dec!(1300)), Some(dec!(1950)), None, None, None],
                vec![Some(dec!(1400)), None, None, None, None],
            ],
        }
    }

    /// 3x3 minimal triangle.
    fn triangle_3x3() -> ClaimsTriangle {
        ClaimsTriangle {
            accident_years: vec![2020, 2021, 2022],
            development_periods: vec![1, 2, 3],
            values: vec![
                vec![Some(dec!(500)), Some(dec!(750)), Some(dec!(800))],
                vec![Some(dec!(600)), Some(dec!(900)), None],
                vec![Some(dec!(700)), None, None],
            ],
        }
    }

    /// 4x4 triangle.
    fn triangle_4x4() -> ClaimsTriangle {
        ClaimsTriangle {
            accident_years: vec![2019, 2020, 2021, 2022],
            development_periods: vec![1, 2, 3, 4],
            values: vec![
                vec![
                    Some(dec!(800)),
                    Some(dec!(1200)),
                    Some(dec!(1350)),
                    Some(dec!(1380)),
                ],
                vec![Some(dec!(900)), Some(dec!(1350)), Some(dec!(1520)), None],
                vec![Some(dec!(1000)), Some(dec!(1500)), None, None],
                vec![Some(dec!(1100)), None, None, None],
            ],
        }
    }

    fn standard_cl_input() -> ReservingInput {
        ReservingInput {
            line_of_business: "Auto Liability".to_string(),
            triangle: triangle_5x5(),
            method: ReservingMethod::ChainLadder,
            earned_premium: None,
            expected_loss_ratio: None,
            tail_factor: None,
            discount_rate: None,
        }
    }

    fn standard_bf_input() -> ReservingInput {
        ReservingInput {
            line_of_business: "Workers Comp".to_string(),
            triangle: triangle_5x5(),
            method: ReservingMethod::BornhuetterFerguson,
            earned_premium: Some(vec![
                dec!(2700),
                dec!(2900),
                dec!(3100),
                dec!(3300),
                dec!(3500),
            ]),
            expected_loss_ratio: Some(dec!(0.65)),
            tail_factor: None,
            discount_rate: None,
        }
    }

    fn standard_both_input() -> ReservingInput {
        ReservingInput {
            line_of_business: "General Liability".to_string(),
            triangle: triangle_5x5(),
            method: ReservingMethod::Both,
            earned_premium: Some(vec![
                dec!(2700),
                dec!(2900),
                dec!(3100),
                dec!(3300),
                dec!(3500),
            ]),
            expected_loss_ratio: Some(dec!(0.65)),
            tail_factor: None,
            discount_rate: None,
        }
    }

    // -----------------------------------------------------------------------
    // Test 1: Chain-ladder development factors on 5x5 triangle
    // -----------------------------------------------------------------------
    #[test]
    fn test_cl_development_factors_5x5() {
        let input = standard_cl_input();
        let result = estimate_reserves(&input).unwrap();
        let cl = result.result.chain_ladder.as_ref().unwrap();

        // There should be 4 development factors (periods 1->2, 2->3, 3->4, 4->5)
        assert_eq!(cl.development_factors.len(), 4);

        // Factor 1->2: sum of Dev2 values / sum of Dev1 values
        // (1500+1650+1800+1950) / (1000+1100+1200+1300) = 6900/4600
        let expected_f1 = dec!(6900) / dec!(4600);
        assert_eq!(
            cl.development_factors[0].volume_weighted, expected_f1,
            "Factor 1->2 should be 6900/4600 = {}",
            expected_f1
        );

        // Factor 2->3: (1700+1870+2040) / (1500+1650+1800) = 5610/4950
        let expected_f2 = dec!(5610) / dec!(4950);
        assert_eq!(
            cl.development_factors[1].volume_weighted, expected_f2,
            "Factor 2->3 should be 5610/4950 = {}",
            expected_f2
        );

        // Factor 3->4: (1750+1920) / (1700+1870) = 3670/3570
        let expected_f3 = dec!(3670) / dec!(3570);
        assert_eq!(
            cl.development_factors[2].volume_weighted, expected_f3,
            "Factor 3->4 should be 3670/3570 = {}",
            expected_f3
        );

        // Factor 4->5: 1760/1750
        let expected_f4 = dec!(1760) / dec!(1750);
        assert_eq!(
            cl.development_factors[3].volume_weighted, expected_f4,
            "Factor 4->5 should be 1760/1750 = {}",
            expected_f4
        );
    }

    // -----------------------------------------------------------------------
    // Test 2: Cumulative factors from link ratios
    // -----------------------------------------------------------------------
    #[test]
    fn test_cl_cumulative_factors() {
        let input = standard_cl_input();
        let result = estimate_reserves(&input).unwrap();
        let cl = result.result.chain_ladder.as_ref().unwrap();

        // CDF for the last column (period 5) = tail = 1.0 (no tail specified)
        assert_eq!(cl.cumulative_factors[4], Decimal::ONE);

        // CDF for period 4 = f(4->5) * tail = f4 * 1.0
        let f4 = cl.development_factors[3].volume_weighted;
        assert_eq!(cl.cumulative_factors[3], f4);

        // CDF for period 3 = f(3->4) * CDF(4)
        let f3 = cl.development_factors[2].volume_weighted;
        assert_eq!(cl.cumulative_factors[2], f3 * cl.cumulative_factors[3]);

        // CDF for period 1 = product of all link ratios * tail
        let product = cl
            .development_factors
            .iter()
            .map(|d| d.volume_weighted)
            .fold(Decimal::ONE, |acc, f| acc * f);
        assert_eq!(cl.cumulative_factors[0], product);
    }

    // -----------------------------------------------------------------------
    // Test 3: Ultimates for each accident year
    // -----------------------------------------------------------------------
    #[test]
    fn test_cl_ultimates_5x5() {
        let input = standard_cl_input();
        let result = estimate_reserves(&input).unwrap();
        let cl = result.result.chain_ladder.as_ref().unwrap();

        assert_eq!(cl.ultimates.len(), 5);

        // 2018 (fully developed at col 4, CDF = tail = 1.0 since no tail)
        // Ultimate = 1760 * 1.0 = 1760
        assert_eq!(cl.ultimates[0].paid_to_date, dec!(1760));
        assert_eq!(
            cl.ultimates[0].ultimate,
            dec!(1760) * cl.cumulative_factors[4]
        );

        // 2022 (only col 0, so ultimate = 1400 * CDF[0])
        assert_eq!(cl.ultimates[4].paid_to_date, dec!(1400));
        let expected_ult_2022 = dec!(1400) * cl.cumulative_factors[0];
        assert_eq!(cl.ultimates[4].ultimate, expected_ult_2022);
    }

    // -----------------------------------------------------------------------
    // Test 4: IBNR = ultimate - paid
    // -----------------------------------------------------------------------
    #[test]
    fn test_cl_ibnr_equals_ultimate_minus_paid() {
        let input = standard_cl_input();
        let result = estimate_reserves(&input).unwrap();
        let cl = result.result.chain_ladder.as_ref().unwrap();

        for ult in &cl.ultimates {
            assert_eq!(
                ult.ibnr,
                ult.ultimate - ult.paid_to_date,
                "IBNR for {} should be ultimate ({}) - paid ({})",
                ult.accident_year,
                ult.ultimate,
                ult.paid_to_date
            );
        }
    }

    // -----------------------------------------------------------------------
    // Test 5: Most recent year has highest IBNR
    // -----------------------------------------------------------------------
    #[test]
    fn test_cl_most_recent_year_highest_ibnr() {
        let input = standard_cl_input();
        let result = estimate_reserves(&input).unwrap();
        let cl = result.result.chain_ladder.as_ref().unwrap();

        let max_ibnr = cl.ibnr_by_year.iter().max_by_key(|x| x.ibnr).unwrap();

        assert_eq!(
            max_ibnr.accident_year, 2022,
            "Most recent year (2022) should have the highest IBNR, but {} does",
            max_ibnr.accident_year
        );
    }

    // -----------------------------------------------------------------------
    // Test 6: Oldest year (fully developed) has 0 IBNR (no tail)
    // -----------------------------------------------------------------------
    #[test]
    fn test_cl_oldest_year_zero_ibnr() {
        let input = standard_cl_input();
        let result = estimate_reserves(&input).unwrap();
        let cl = result.result.chain_ladder.as_ref().unwrap();

        // With no tail factor, the oldest year (fully developed) has IBNR = 0
        assert_eq!(
            cl.ibnr_by_year[0].ibnr,
            Decimal::ZERO,
            "Oldest year (2018) should have 0 IBNR with no tail, got {}",
            cl.ibnr_by_year[0].ibnr
        );
    }

    // -----------------------------------------------------------------------
    // Test 7: BF requires premium and ELR
    // -----------------------------------------------------------------------
    #[test]
    fn test_bf_requires_premium() {
        let input = ReservingInput {
            line_of_business: "Test".to_string(),
            triangle: triangle_5x5(),
            method: ReservingMethod::BornhuetterFerguson,
            earned_premium: None,
            expected_loss_ratio: Some(dec!(0.65)),
            tail_factor: None,
            discount_rate: None,
        };
        let result = estimate_reserves(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "earned_premium");
            }
            other => panic!("Expected InvalidInput for earned_premium, got: {other}"),
        }
    }

    // -----------------------------------------------------------------------
    // Test 8: BF ultimates blend a priori with development
    // -----------------------------------------------------------------------
    #[test]
    fn test_bf_ultimates_blend() {
        let input = standard_bf_input();
        let result = estimate_reserves(&input).unwrap();
        let bf = result.result.bornhuetter_ferguson.as_ref().unwrap();

        // BF ultimate should be between paid-to-date and some reasonable upper bound
        for ult in &bf.bf_ultimates {
            assert!(
                ult.ultimate >= ult.paid_to_date,
                "BF ultimate ({}) should be >= paid_to_date ({}) for year {}",
                ult.ultimate,
                ult.paid_to_date,
                ult.accident_year
            );
        }

        // BF IBNR for each year should be non-negative
        for ibnr in &bf.bf_ibnr_by_year {
            assert!(
                ibnr.ibnr >= Decimal::ZERO,
                "BF IBNR ({}) should be >= 0 for year {}",
                ibnr.ibnr,
                ibnr.accident_year
            );
        }
    }

    // -----------------------------------------------------------------------
    // Test 9: BF immature years closer to expected than CL
    // -----------------------------------------------------------------------
    #[test]
    fn test_bf_immature_closer_to_expected() {
        // Run both methods on the same triangle
        let mut input_both = standard_both_input();
        input_both.method = ReservingMethod::Both;
        let result = estimate_reserves(&input_both).unwrap();
        let cl = result.result.chain_ladder.as_ref().unwrap();
        let bf = result.result.bornhuetter_ferguson.as_ref().unwrap();

        // For the most recent year (2022, only 1 period of development =
        // very immature), BF ultimate should be closer to the expected
        // ultimate (premium * ELR) than CL ultimate.
        let expected_ult_2022 = dec!(3500) * dec!(0.65); // 2275
        let cl_ult_2022 = cl.ultimates[4].ultimate;
        let bf_ult_2022 = bf.bf_ultimates[4].ultimate;

        let cl_diff = (cl_ult_2022 - expected_ult_2022).abs();
        let bf_diff = (bf_ult_2022 - expected_ult_2022).abs();

        assert!(
            bf_diff <= cl_diff,
            "BF ultimate for immature year 2022 ({}) should be closer to expected ({}) \
             than CL ({}). BF diff={}, CL diff={}",
            bf_ult_2022,
            expected_ult_2022,
            cl_ult_2022,
            bf_diff,
            cl_diff
        );
    }

    // -----------------------------------------------------------------------
    // Test 10: BF mature years close to CL result
    // -----------------------------------------------------------------------
    #[test]
    fn test_bf_mature_years_close_to_cl() {
        let input = standard_both_input();
        let result = estimate_reserves(&input).unwrap();
        let cl = result.result.chain_ladder.as_ref().unwrap();
        let bf = result.result.bornhuetter_ferguson.as_ref().unwrap();

        // For the oldest year (2018, fully developed with no tail), both
        // methods should produce the same IBNR (0).
        let cl_ibnr_2018 = cl.ibnr_by_year[0].ibnr;
        let bf_ibnr_2018 = bf.bf_ibnr_by_year[0].ibnr;

        // Both should be zero or very close for a fully developed year
        assert!(
            cl_ibnr_2018.abs() < dec!(0.01),
            "CL IBNR for 2018 should be ~0, got {}",
            cl_ibnr_2018
        );
        assert!(
            bf_ibnr_2018.abs() < dec!(0.01),
            "BF IBNR for 2018 should be ~0, got {}",
            bf_ibnr_2018
        );
    }

    // -----------------------------------------------------------------------
    // Test 11: Both method selects BF for immature, CL for mature
    // -----------------------------------------------------------------------
    #[test]
    fn test_both_method_selection() {
        let input = standard_both_input();
        let result = estimate_reserves(&input).unwrap();
        let selected = &result.result.selected_reserves;

        // The most mature year (2018, fully developed) should use CL
        assert_eq!(
            selected[0].method, "ChainLadder",
            "Oldest year (2018) should use ChainLadder, got {}",
            selected[0].method
        );

        // The most recent year (2022, 1 period = very immature) should use BF
        assert_eq!(
            selected[4].method, "BornhuetterFerguson",
            "Most recent year (2022) should use BornhuetterFerguson, got {}",
            selected[4].method
        );
    }

    // -----------------------------------------------------------------------
    // Test 12: Tail factor increases ultimates
    // -----------------------------------------------------------------------
    #[test]
    fn test_tail_factor_increases_ultimates() {
        let mut input_no_tail = standard_cl_input();
        input_no_tail.tail_factor = None;
        let result_no_tail = estimate_reserves(&input_no_tail).unwrap();
        let cl_no_tail = result_no_tail.result.chain_ladder.as_ref().unwrap();

        let mut input_tail = standard_cl_input();
        input_tail.tail_factor = Some(dec!(1.05));
        let result_tail = estimate_reserves(&input_tail).unwrap();
        let cl_tail = result_tail.result.chain_ladder.as_ref().unwrap();

        // With a tail factor > 1, all ultimates should be higher
        for (no_tail, with_tail) in cl_no_tail.ultimates.iter().zip(cl_tail.ultimates.iter()) {
            assert!(
                with_tail.ultimate >= no_tail.ultimate,
                "Tail factor should increase ultimate for year {}: {} vs {}",
                no_tail.accident_year,
                with_tail.ultimate,
                no_tail.ultimate
            );
        }

        // Total IBNR should be higher with tail
        let ibnr_no_tail: Decimal = cl_no_tail.ibnr_by_year.iter().map(|x| x.ibnr).sum();
        let ibnr_tail: Decimal = cl_tail.ibnr_by_year.iter().map(|x| x.ibnr).sum();
        assert!(
            ibnr_tail > ibnr_no_tail,
            "Total IBNR with tail ({}) should exceed without tail ({})",
            ibnr_tail,
            ibnr_no_tail
        );
    }

    // -----------------------------------------------------------------------
    // Test 13: No tail factor (default 1.0)
    // -----------------------------------------------------------------------
    #[test]
    fn test_no_tail_factor_default() {
        let input = standard_cl_input();
        let result = estimate_reserves(&input).unwrap();
        let cl = result.result.chain_ladder.as_ref().unwrap();

        // The last cumulative factor should be exactly 1.0 with no tail
        assert_eq!(
            cl.cumulative_factors[4],
            Decimal::ONE,
            "CDF for last period should be 1.0 with no tail, got {}",
            cl.cumulative_factors[4]
        );
    }

    // -----------------------------------------------------------------------
    // Test 14: Present value discount reduces reserves
    // -----------------------------------------------------------------------
    #[test]
    fn test_present_value_discount_reduces_reserves() {
        let mut input = standard_cl_input();
        input.discount_rate = Some(dec!(0.05));
        let result = estimate_reserves(&input).unwrap();

        for r in &result.result.selected_reserves {
            if r.reserve > Decimal::ZERO {
                let pv = r.present_value_reserve.unwrap();
                assert!(
                    pv <= r.reserve,
                    "PV reserve ({}) should be <= nominal reserve ({}) for year {}",
                    pv,
                    r.reserve,
                    r.accident_year
                );
            }
        }

        // Total PV should be less than or equal to total nominal IBNR
        let total_nominal = result.result.summary.total_ibnr;
        let total_pv = result.result.summary.present_value_total.unwrap();
        assert!(
            total_pv <= total_nominal,
            "PV total ({}) should be <= nominal total ({})",
            total_pv,
            total_nominal
        );
    }

    // -----------------------------------------------------------------------
    // Test 15: Loss ratio calculation
    // -----------------------------------------------------------------------
    #[test]
    fn test_loss_ratio_calculation() {
        let mut input = standard_bf_input();
        // Use CL method but supply premium so loss ratio can be computed
        input.method = ReservingMethod::BornhuetterFerguson;
        let result = estimate_reserves(&input).unwrap();
        let summary = &result.result.summary;

        let lr = summary.overall_loss_ratio.unwrap();
        assert!(
            lr > Decimal::ZERO,
            "Loss ratio should be positive, got {}",
            lr
        );
        // Loss ratio = total_ultimate / total_premium
        let total_premium: Decimal = input.earned_premium.as_ref().unwrap().iter().sum();
        let expected_lr = summary.total_ultimate / total_premium;
        assert_eq!(lr, expected_lr);
    }

    // -----------------------------------------------------------------------
    // Test 16: Adequacy ratio
    // -----------------------------------------------------------------------
    #[test]
    fn test_adequacy_ratio() {
        let input = standard_bf_input();
        let result = estimate_reserves(&input).unwrap();
        let summary = &result.result.summary;

        let ar = summary.adequacy_ratio.unwrap();
        assert!(
            ar > Decimal::ZERO,
            "Adequacy ratio should be positive, got {}",
            ar
        );
        // Adequacy ratio = total_ultimate / total_expected
        let total_premium: Decimal = input.earned_premium.as_ref().unwrap().iter().sum();
        let total_expected = total_premium * input.expected_loss_ratio.unwrap();
        let expected_ar = summary.total_ultimate / total_expected;
        assert_eq!(ar, expected_ar);
    }

    // -----------------------------------------------------------------------
    // Test 17: 3x3 minimal triangle
    // -----------------------------------------------------------------------
    #[test]
    fn test_3x3_minimal_triangle() {
        let input = ReservingInput {
            line_of_business: "Property".to_string(),
            triangle: triangle_3x3(),
            method: ReservingMethod::ChainLadder,
            earned_premium: None,
            expected_loss_ratio: None,
            tail_factor: None,
            discount_rate: None,
        };
        let result = estimate_reserves(&input).unwrap();
        let cl = result.result.chain_ladder.as_ref().unwrap();

        assert_eq!(cl.development_factors.len(), 2);
        assert_eq!(cl.ultimates.len(), 3);

        // Oldest year should have 0 IBNR (no tail)
        assert_eq!(cl.ibnr_by_year[0].ibnr, Decimal::ZERO);

        // Most recent year should have positive IBNR
        assert!(
            cl.ibnr_by_year[2].ibnr > Decimal::ZERO,
            "Most recent year should have positive IBNR"
        );
    }

    // -----------------------------------------------------------------------
    // Test 18: 4x4 triangle
    // -----------------------------------------------------------------------
    #[test]
    fn test_4x4_triangle() {
        let input = ReservingInput {
            line_of_business: "Marine".to_string(),
            triangle: triangle_4x4(),
            method: ReservingMethod::ChainLadder,
            earned_premium: None,
            expected_loss_ratio: None,
            tail_factor: None,
            discount_rate: None,
        };
        let result = estimate_reserves(&input).unwrap();
        let cl = result.result.chain_ladder.as_ref().unwrap();

        assert_eq!(cl.development_factors.len(), 3);
        assert_eq!(cl.ultimates.len(), 4);
        assert_eq!(cl.completed_triangle.len(), 4);

        // All rows of the completed triangle should be fully filled
        for row in &cl.completed_triangle {
            assert_eq!(row.len(), 4);
            for val in row {
                assert!(
                    *val > Decimal::ZERO,
                    "Completed triangle cell should be positive"
                );
            }
        }
    }

    // -----------------------------------------------------------------------
    // Test 19: Triangle with missing values validation
    // -----------------------------------------------------------------------
    #[test]
    fn test_triangle_all_none_row_error() {
        let bad_tri = ClaimsTriangle {
            accident_years: vec![2020, 2021],
            development_periods: vec![1, 2],
            values: vec![
                vec![Some(dec!(100)), Some(dec!(150))],
                vec![None, None], // all None — invalid
            ],
        };
        let input = ReservingInput {
            line_of_business: "Test".to_string(),
            triangle: bad_tri,
            method: ReservingMethod::ChainLadder,
            earned_premium: None,
            expected_loss_ratio: None,
            tail_factor: None,
            discount_rate: None,
        };
        let result = estimate_reserves(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, reason } => {
                assert_eq!(field, "triangle.values");
                assert!(
                    reason.contains("no observed values"),
                    "Error should mention missing values: {}",
                    reason
                );
            }
            other => panic!("Expected InvalidInput, got: {other}"),
        }
    }

    // -----------------------------------------------------------------------
    // Test 20: Edge case — single accident year
    // -----------------------------------------------------------------------
    #[test]
    fn test_single_accident_year() {
        let tri = ClaimsTriangle {
            accident_years: vec![2022],
            development_periods: vec![1, 2, 3],
            values: vec![vec![Some(dec!(1000)), Some(dec!(1500)), Some(dec!(1700))]],
        };
        let input = ReservingInput {
            line_of_business: "Test".to_string(),
            triangle: tri,
            method: ReservingMethod::ChainLadder,
            earned_premium: None,
            expected_loss_ratio: None,
            tail_factor: None,
            discount_rate: None,
        };
        let result = estimate_reserves(&input).unwrap();
        let cl = result.result.chain_ladder.as_ref().unwrap();

        // Single fully-developed year: IBNR = 0 (no tail)
        assert_eq!(cl.ultimates.len(), 1);
        assert_eq!(cl.ultimates[0].ibnr, Decimal::ZERO);
    }

    // -----------------------------------------------------------------------
    // Test 21: Mismatched premium count error
    // -----------------------------------------------------------------------
    #[test]
    fn test_mismatched_premium_count_error() {
        let input = ReservingInput {
            line_of_business: "Test".to_string(),
            triangle: triangle_5x5(),
            method: ReservingMethod::BornhuetterFerguson,
            earned_premium: Some(vec![dec!(1000), dec!(1100), dec!(1200)]), // 3 instead of 5
            expected_loss_ratio: Some(dec!(0.65)),
            tail_factor: None,
            discount_rate: None,
        };
        let result = estimate_reserves(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "earned_premium");
            }
            other => panic!("Expected InvalidInput for earned_premium, got: {other}"),
        }
    }

    // -----------------------------------------------------------------------
    // Test 22: Summary totals
    // -----------------------------------------------------------------------
    #[test]
    fn test_summary_totals() {
        let input = standard_cl_input();
        let result = estimate_reserves(&input).unwrap();
        let cl = result.result.chain_ladder.as_ref().unwrap();
        let summary = &result.result.summary;

        // total_ibnr should equal sum of individual IBNRs
        let sum_ibnr: Decimal = cl.ibnr_by_year.iter().map(|x| x.ibnr).sum();
        assert_eq!(
            summary.total_ibnr, sum_ibnr,
            "Summary total_ibnr ({}) should match sum of year IBNRs ({})",
            summary.total_ibnr, sum_ibnr
        );

        // total_paid should equal sum of latest diagonal
        let sum_paid = dec!(1760) + dec!(1920) + dec!(2040) + dec!(1950) + dec!(1400);
        assert_eq!(
            summary.total_paid, sum_paid,
            "Summary total_paid ({}) should match sum of diagonal ({})",
            summary.total_paid, sum_paid
        );

        // total_ultimate = total_paid + total_ibnr
        assert_eq!(
            summary.total_ultimate,
            summary.total_paid + summary.total_ibnr,
            "total_ultimate ({}) should equal total_paid + total_ibnr ({})",
            summary.total_ultimate,
            summary.total_paid + summary.total_ibnr
        );
    }

    // -----------------------------------------------------------------------
    // Test 23: BF requires ELR
    // -----------------------------------------------------------------------
    #[test]
    fn test_bf_requires_elr() {
        let input = ReservingInput {
            line_of_business: "Test".to_string(),
            triangle: triangle_5x5(),
            method: ReservingMethod::BornhuetterFerguson,
            earned_premium: Some(vec![
                dec!(2700),
                dec!(2900),
                dec!(3100),
                dec!(3300),
                dec!(3500),
            ]),
            expected_loss_ratio: None, // missing
            tail_factor: None,
            discount_rate: None,
        };
        let result = estimate_reserves(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "expected_loss_ratio");
            }
            other => panic!("Expected InvalidInput for expected_loss_ratio, got: {other}"),
        }
    }

    // -----------------------------------------------------------------------
    // Test 24: Metadata is populated
    // -----------------------------------------------------------------------
    #[test]
    fn test_metadata_populated() {
        let input = standard_cl_input();
        let result = estimate_reserves(&input).unwrap();
        assert!(!result.methodology.is_empty());
        assert!(result.methodology.contains("Chain-Ladder"));
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
    }

    // -----------------------------------------------------------------------
    // Test 25: Row/column dimension mismatch error
    // -----------------------------------------------------------------------
    #[test]
    fn test_row_column_mismatch_error() {
        let bad_tri = ClaimsTriangle {
            accident_years: vec![2020, 2021],
            development_periods: vec![1, 2, 3],
            values: vec![
                vec![Some(dec!(100)), Some(dec!(150)), Some(dec!(170))],
                vec![Some(dec!(110)), Some(dec!(165))], // 2 cols instead of 3
            ],
        };
        let input = ReservingInput {
            line_of_business: "Test".to_string(),
            triangle: bad_tri,
            method: ReservingMethod::ChainLadder,
            earned_premium: None,
            expected_loss_ratio: None,
            tail_factor: None,
            discount_rate: None,
        };
        let result = estimate_reserves(&input);
        assert!(result.is_err());
    }
}
