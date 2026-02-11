use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{with_metadata, ComputationOutput};
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RatedExposure {
    pub name: String,
    pub rating: String,
    pub exposure: Decimal,
    pub maturity_years: Decimal,
    pub coupon_rate: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionMatrix {
    /// Rating labels, e.g. ["AAA", "AA", "A", "BBB", "BB", "B", "CCC", "D"]
    pub ratings: Vec<String>,
    /// Row i = from rating[i], column j = to rating[j]; each row sums to 1
    pub probabilities: Vec<Vec<Decimal>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RatingSpread {
    pub rating: String,
    pub spread_bps: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationInput {
    pub initial_ratings: Vec<RatedExposure>,
    pub transition_matrix: TransitionMatrix,
    pub time_horizon_years: u32,
    /// Spread curve for mark-to-market
    pub spread_curve: Vec<RatingSpread>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationResult {
    pub name: String,
    pub current_rating: String,
    pub expected_rating_distribution: Vec<(String, Decimal)>,
    pub upgrade_probability: Decimal,
    pub downgrade_probability: Decimal,
    pub default_probability: Decimal,
    pub stable_probability: Decimal,
    pub expected_value_change_pct: Decimal,
    pub credit_var_contribution: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixQuality {
    /// All rows sum to 1 and all probabilities >= 0
    pub is_valid_stochastic: bool,
    /// Higher ratings have lower default probability
    pub is_monotone: bool,
    /// Should be "D" (default)
    pub absorbing_state: String,
    /// max |row_sum - 1|
    pub max_row_deviation: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationOutput {
    pub portfolio_results: Vec<MigrationResult>,
    pub portfolio_expected_migration_loss: Decimal,
    pub portfolio_migration_var: Decimal,
    /// Cumulative default probability by rating
    pub multi_year_default_prob: Vec<(String, Decimal)>,
    pub matrix_quality: MatrixQuality,
    pub methodology: String,
    pub assumptions: HashMap<String, String>,
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// Matrix operations (all Decimal)
// ---------------------------------------------------------------------------

/// Multiply two square matrices of Decimals.
fn matrix_multiply(a: &[Vec<Decimal>], b: &[Vec<Decimal>]) -> Vec<Vec<Decimal>> {
    let n = a.len();
    let mut result = vec![vec![Decimal::ZERO; n]; n];
    for i in 0..n {
        for j in 0..n {
            let mut s = Decimal::ZERO;
            for k in 0..n {
                s += a[i][k] * b[k][j];
            }
            result[i][j] = s;
        }
    }
    result
}

/// Raise a square matrix to integer power via repeated squaring.
fn matrix_power(m: &[Vec<Decimal>], exp: u32) -> Vec<Vec<Decimal>> {
    let n = m.len();
    if exp == 0 {
        // Return identity
        let mut id = vec![vec![Decimal::ZERO; n]; n];
        for (i, row) in id.iter_mut().enumerate() {
            row[i] = Decimal::ONE;
        }
        return id;
    }
    if exp == 1 {
        return m.to_vec();
    }

    let mut base = m.to_vec();
    let mut result: Option<Vec<Vec<Decimal>>> = None;
    let mut e = exp;

    while e > 0 {
        if e & 1 == 1 {
            result = Some(match result {
                None => base.clone(),
                Some(ref r) => matrix_multiply(r, &base),
            });
        }
        base = matrix_multiply(&base, &base);
        e >>= 1;
    }

    result.unwrap_or_else(|| {
        let mut id = vec![vec![Decimal::ZERO; n]; n];
        for (i, row) in id.iter_mut().enumerate() {
            row[i] = Decimal::ONE;
        }
        id
    })
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_input(input: &MigrationInput) -> CorpFinanceResult<Vec<String>> {
    let warnings = Vec::new();
    let matrix = &input.transition_matrix;
    let n = matrix.ratings.len();

    // Matrix must be square
    if matrix.probabilities.len() != n {
        return Err(CorpFinanceError::InvalidInput {
            field: "transition_matrix.probabilities".into(),
            reason: format!(
                "matrix has {} rows but {} ratings",
                matrix.probabilities.len(),
                n
            ),
        });
    }
    for (i, row) in matrix.probabilities.iter().enumerate() {
        if row.len() != n {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("transition_matrix.probabilities[{}]", i),
                reason: format!("row has {} columns but {} ratings", row.len(), n),
            });
        }
    }

    // At least one exposure
    if input.initial_ratings.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "at least one rated exposure is required".into(),
        ));
    }

    // Exposure ratings must exist in matrix
    for exp in &input.initial_ratings {
        if !matrix.ratings.contains(&exp.rating) {
            return Err(CorpFinanceError::InvalidInput {
                field: "initial_ratings".into(),
                reason: format!(
                    "exposure '{}' has rating '{}' not in transition matrix",
                    exp.name, exp.rating
                ),
            });
        }
    }

    // Spread curve ratings must exist in matrix
    for sp in &input.spread_curve {
        if !matrix.ratings.contains(&sp.rating) {
            return Err(CorpFinanceError::InvalidInput {
                field: "spread_curve".into(),
                reason: format!("spread rating '{}' not in transition matrix", sp.rating),
            });
        }
    }

    // Time horizon
    if input.time_horizon_years == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "time_horizon_years".into(),
            reason: "must be at least 1".into(),
        });
    }

    Ok(warnings)
}

fn assess_matrix_quality(matrix: &TransitionMatrix) -> (MatrixQuality, Vec<String>) {
    let mut warnings = Vec::new();
    let n = matrix.ratings.len();
    let mut max_row_deviation = Decimal::ZERO;
    let mut is_valid_stochastic = true;

    // Check row sums and non-negativity
    for (i, row) in matrix.probabilities.iter().enumerate() {
        let row_sum: Decimal = row.iter().copied().sum();
        let dev = (row_sum - Decimal::ONE).abs();
        if dev > max_row_deviation {
            max_row_deviation = dev;
        }
        if dev > dec!(0.001) {
            is_valid_stochastic = false;
            warnings.push(format!(
                "Row '{}' sums to {} (deviation {})",
                matrix.ratings[i], row_sum, dev
            ));
        }
        for (j, &p) in row.iter().enumerate() {
            if p < Decimal::ZERO {
                is_valid_stochastic = false;
                warnings.push(format!(
                    "Negative probability at [{},{}]: {}",
                    matrix.ratings[i], matrix.ratings[j], p
                ));
            }
        }
    }

    // Find default column (last rating assumed "D")
    let default_idx = n - 1;
    let absorbing_state = matrix.ratings[default_idx].clone();

    // Check absorbing: default row should be [0, ..., 0, 1]
    if n > 0 {
        let d_row = &matrix.probabilities[default_idx];
        let self_prob = d_row[default_idx];
        if (self_prob - Decimal::ONE).abs() > dec!(0.001) {
            warnings.push(format!(
                "Default state '{}' is not absorbing (self-transition = {})",
                absorbing_state, self_prob
            ));
        }
    }

    // Monotonicity: for each row, default probability should increase as rating worsens
    let mut is_monotone = true;
    if n >= 2 {
        for i in 0..(n - 2) {
            let pd_i = matrix.probabilities[i][default_idx];
            let pd_next = matrix.probabilities[i + 1][default_idx];
            if pd_next < pd_i - dec!(0.0001) {
                is_monotone = false;
                warnings.push(format!(
                    "Non-monotone: '{}' default prob {} > '{}' default prob {}",
                    matrix.ratings[i],
                    pd_i,
                    matrix.ratings[i + 1],
                    pd_next
                ));
            }
        }
    }

    (
        MatrixQuality {
            is_valid_stochastic,
            is_monotone,
            absorbing_state,
            max_row_deviation,
        },
        warnings,
    )
}

// ---------------------------------------------------------------------------
// Core analytics
// ---------------------------------------------------------------------------

/// Compute rating migration analytics for a portfolio.
pub fn calculate_migration(
    input: &MigrationInput,
) -> CorpFinanceResult<ComputationOutput<MigrationOutput>> {
    let start = Instant::now();
    let mut warnings = validate_input(input)?;

    let matrix = &input.transition_matrix;
    let n = matrix.ratings.len();
    let default_idx = n - 1;

    // Assess matrix quality
    let (quality, quality_warnings) = assess_matrix_quality(matrix);
    warnings.extend(quality_warnings);

    if !quality.is_monotone {
        warnings.push("Transition matrix is not monotone".into());
    }

    // Build spread lookup
    let spread_map: HashMap<String, Decimal> = input
        .spread_curve
        .iter()
        .map(|s| (s.rating.clone(), s.spread_bps))
        .collect();

    // Multi-year matrix power
    let multi_year_matrix = matrix_power(&matrix.probabilities, input.time_horizon_years);

    // Cumulative default probabilities by rating (from multi-year matrix)
    let multi_year_default_prob: Vec<(String, Decimal)> = matrix
        .ratings
        .iter()
        .zip(multi_year_matrix.iter())
        .map(|(rating, row)| (rating.clone(), row[default_idx]))
        .collect();

    // Per-exposure migration analysis
    let mut portfolio_results: Vec<MigrationResult> = Vec::new();
    let mut portfolio_expected_loss = Decimal::ZERO;
    let mut portfolio_var_sum = Decimal::ZERO;

    for exp in &input.initial_ratings {
        // Find row index for this exposure's current rating
        let row_idx = matrix
            .ratings
            .iter()
            .position(|r| r == &exp.rating)
            .unwrap(); // validated above

        // Rating distribution after time_horizon
        let row = &multi_year_matrix[row_idx];
        let expected_distribution: Vec<(String, Decimal)> = matrix
            .ratings
            .iter()
            .zip(row.iter())
            .map(|(r, p)| (r.clone(), *p))
            .collect();

        // Upgrade/downgrade/stable/default probabilities
        let stable_prob = row[row_idx];
        let default_prob = row[default_idx];
        let mut upgrade_prob = Decimal::ZERO;
        let mut downgrade_prob = Decimal::ZERO;
        for (j, &prob) in row.iter().enumerate() {
            if j < row_idx {
                upgrade_prob += prob;
            } else if j > row_idx && j != default_idx {
                downgrade_prob += prob;
            }
        }
        // Default is also a downgrade for non-default ratings
        if row_idx != default_idx {
            downgrade_prob += default_prob;
        }

        // Mark-to-market value change
        let current_spread = spread_map
            .get(&exp.rating)
            .copied()
            .unwrap_or(Decimal::ZERO);

        // Approximate duration: (1 - 1/(1+y)^n) / y where y = coupon_rate + spread/10000
        let y = exp.coupon_rate + current_spread / dec!(10000);
        let duration = if y > Decimal::ZERO && exp.maturity_years > Decimal::ZERO {
            // Macaulay duration approximation
            let mat_int = decimal_to_u32(exp.maturity_years);
            if mat_int > 0 {
                let denom = iterative_pow(Decimal::ONE + y, mat_int);
                if denom > Decimal::ZERO {
                    (Decimal::ONE - Decimal::ONE / denom) / y
                } else {
                    exp.maturity_years / dec!(2)
                }
            } else {
                exp.maturity_years / dec!(2)
            }
        } else {
            exp.maturity_years / dec!(2)
        };

        // Expected value change and VaR contribution
        let mut expected_value_change = Decimal::ZERO;
        let mut loss_prob_pairs: Vec<(Decimal, Decimal)> = Vec::with_capacity(n);

        for (j, (rating, &prob)) in matrix.ratings.iter().zip(row.iter()).enumerate() {
            let new_spread = spread_map.get(rating).copied().unwrap_or(Decimal::ZERO);

            let value_change_pct = if j == default_idx {
                // Default: lose (1 - recovery), simplified as -100%
                dec!(-100)
            } else {
                // value_change_pct ~ -duration * (new_spread - current_spread) / 10000 * 100
                -duration * (new_spread - current_spread) / dec!(10000) * dec!(100)
            };

            expected_value_change += prob * value_change_pct;
            loss_prob_pairs.push((value_change_pct, prob));
        }

        // Credit VaR contribution: worst-case loss at ~99% confidence
        // Sort by loss (ascending = worst first)
        loss_prob_pairs.sort_by(|a, b| a.0.cmp(&b.0));
        let mut cum_prob = Decimal::ZERO;
        let mut var_contribution = Decimal::ZERO;
        for (loss, prob) in &loss_prob_pairs {
            cum_prob += prob;
            if cum_prob >= dec!(0.01) {
                // 99% confidence => the loss at 1% cumulative tail
                var_contribution = -(*loss) * exp.exposure / dec!(100);
                break;
            }
        }
        // If we never reached 1% (very unlikely), use worst loss
        if cum_prob < dec!(0.01) && !loss_prob_pairs.is_empty() {
            let worst = loss_prob_pairs[0].0;
            var_contribution = -worst * exp.exposure / dec!(100);
        }

        portfolio_expected_loss += exp.exposure * expected_value_change / dec!(100);
        portfolio_var_sum += var_contribution;

        portfolio_results.push(MigrationResult {
            name: exp.name.clone(),
            current_rating: exp.rating.clone(),
            expected_rating_distribution: expected_distribution,
            upgrade_probability: upgrade_prob,
            downgrade_probability: downgrade_prob,
            default_probability: default_prob,
            stable_probability: stable_prob,
            expected_value_change_pct: expected_value_change,
            credit_var_contribution: var_contribution,
        });
    }

    let mut assumptions = HashMap::new();
    assumptions.insert("model".into(), "Rating migration / Gaussian copula".into());
    assumptions.insert(
        "time_horizon".into(),
        format!("{} year(s)", input.time_horizon_years),
    );
    assumptions.insert("matrix_size".into(), format!("{} ratings", n));

    let output = MigrationOutput {
        portfolio_results,
        portfolio_expected_migration_loss: portfolio_expected_loss,
        portfolio_migration_var: portfolio_var_sum,
        multi_year_default_prob,
        matrix_quality: quality,
        methodology: "Rating migration / Markov chain".into(),
        assumptions,
        warnings: warnings.clone(),
    };

    let elapsed = start.elapsed().as_micros() as u64;
    let meta_assumptions = serde_json::json!({
        "model": "Rating migration / Markov chain",
        "time_horizon_years": input.time_horizon_years,
        "matrix_ratings": input.transition_matrix.ratings,
    });

    Ok(with_metadata(
        "Rating migration / Markov chain",
        &meta_assumptions,
        warnings,
        elapsed,
        output,
    ))
}

/// Convert Decimal to u32 (truncating fractional).
fn decimal_to_u32(d: Decimal) -> u32 {
    let s = d.to_string();
    // Parse integer part
    if let Some(dot_idx) = s.find('.') {
        s[..dot_idx].parse().unwrap_or(0)
    } else {
        s.parse().unwrap_or(0)
    }
}

/// Integer power via iterative multiplication.
fn iterative_pow(base: Decimal, exp: u32) -> Decimal {
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

    /// Standard S&P-like 8-rating transition matrix (1-year, approximate).
    fn sp_transition_matrix() -> TransitionMatrix {
        TransitionMatrix {
            ratings: vec![
                "AAA".into(),
                "AA".into(),
                "A".into(),
                "BBB".into(),
                "BB".into(),
                "B".into(),
                "CCC".into(),
                "D".into(),
            ],
            probabilities: vec![
                // AAA
                vec![
                    dec!(0.9081),
                    dec!(0.0833),
                    dec!(0.0068),
                    dec!(0.0006),
                    dec!(0.0012),
                    dec!(0.0000),
                    dec!(0.0000),
                    dec!(0.0000),
                ],
                // AA
                vec![
                    dec!(0.0070),
                    dec!(0.9065),
                    dec!(0.0779),
                    dec!(0.0064),
                    dec!(0.0006),
                    dec!(0.0014),
                    dec!(0.0002),
                    dec!(0.0000),
                ],
                // A
                vec![
                    dec!(0.0009),
                    dec!(0.0227),
                    dec!(0.9105),
                    dec!(0.0552),
                    dec!(0.0074),
                    dec!(0.0026),
                    dec!(0.0001),
                    dec!(0.0006),
                ],
                // BBB
                vec![
                    dec!(0.0002),
                    dec!(0.0033),
                    dec!(0.0595),
                    dec!(0.8693),
                    dec!(0.0530),
                    dec!(0.0117),
                    dec!(0.0012),
                    dec!(0.0018),
                ],
                // BB
                vec![
                    dec!(0.0003),
                    dec!(0.0014),
                    dec!(0.0067),
                    dec!(0.0773),
                    dec!(0.8053),
                    dec!(0.0884),
                    dec!(0.0100),
                    dec!(0.0106),
                ],
                // B
                vec![
                    dec!(0.0000),
                    dec!(0.0011),
                    dec!(0.0024),
                    dec!(0.0043),
                    dec!(0.0648),
                    dec!(0.8346),
                    dec!(0.0407),
                    dec!(0.0521),
                ],
                // CCC
                vec![
                    dec!(0.0022),
                    dec!(0.0000),
                    dec!(0.0022),
                    dec!(0.0130),
                    dec!(0.0238),
                    dec!(0.1124),
                    dec!(0.6486),
                    dec!(0.1978),
                ],
                // D (absorbing)
                vec![
                    dec!(0.0000),
                    dec!(0.0000),
                    dec!(0.0000),
                    dec!(0.0000),
                    dec!(0.0000),
                    dec!(0.0000),
                    dec!(0.0000),
                    dec!(1.0000),
                ],
            ],
        }
    }

    fn standard_spread_curve() -> Vec<RatingSpread> {
        vec![
            RatingSpread {
                rating: "AAA".into(),
                spread_bps: dec!(20),
            },
            RatingSpread {
                rating: "AA".into(),
                spread_bps: dec!(40),
            },
            RatingSpread {
                rating: "A".into(),
                spread_bps: dec!(70),
            },
            RatingSpread {
                rating: "BBB".into(),
                spread_bps: dec!(120),
            },
            RatingSpread {
                rating: "BB".into(),
                spread_bps: dec!(250),
            },
            RatingSpread {
                rating: "B".into(),
                spread_bps: dec!(450),
            },
            RatingSpread {
                rating: "CCC".into(),
                spread_bps: dec!(800),
            },
            RatingSpread {
                rating: "D".into(),
                spread_bps: dec!(2000),
            },
        ]
    }

    fn single_bbb_input() -> MigrationInput {
        MigrationInput {
            initial_ratings: vec![RatedExposure {
                name: "Corp A".into(),
                rating: "BBB".into(),
                exposure: dec!(1000000),
                maturity_years: dec!(5),
                coupon_rate: dec!(0.05),
            }],
            transition_matrix: sp_transition_matrix(),
            time_horizon_years: 1,
            spread_curve: standard_spread_curve(),
        }
    }

    // -----------------------------------------------------------------------
    // Matrix operation tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_matrix_power_identity() {
        // M^0 = I
        let m = sp_transition_matrix();
        let result = matrix_power(&m.probabilities, 0);
        for i in 0..m.ratings.len() {
            for j in 0..m.ratings.len() {
                let expected = if i == j { Decimal::ONE } else { Decimal::ZERO };
                assert!(
                    approx_eq(result[i][j], expected, dec!(0.0001)),
                    "Identity check failed at [{},{}]: {}",
                    i,
                    j,
                    result[i][j]
                );
            }
        }
    }

    #[test]
    fn test_matrix_power_one() {
        // M^1 = M
        let m = sp_transition_matrix();
        let result = matrix_power(&m.probabilities, 1);
        for i in 0..m.ratings.len() {
            for j in 0..m.ratings.len() {
                assert!(
                    approx_eq(result[i][j], m.probabilities[i][j], dec!(0.0001)),
                    "M^1 check failed at [{},{}]",
                    i,
                    j
                );
            }
        }
    }

    #[test]
    fn test_matrix_power_two_row_sums() {
        // M^2 rows should still sum to ~1
        let m = sp_transition_matrix();
        let result = matrix_power(&m.probabilities, 2);
        for (i, row) in result.iter().enumerate() {
            let row_sum: Decimal = row.iter().copied().sum();
            assert!(
                approx_eq(row_sum, Decimal::ONE, dec!(0.01)),
                "M^2 row {} sums to {}",
                i,
                row_sum
            );
        }
    }

    #[test]
    fn test_matrix_power_five_cumulative_default() {
        // 5-year cumulative default for BBB should be higher than 1-year
        let m = sp_transition_matrix();
        let one_year = matrix_power(&m.probabilities, 1);
        let five_year = matrix_power(&m.probabilities, 5);
        let bbb_idx = 3; // BBB
        let d_idx = 7; // D
        assert!(
            five_year[bbb_idx][d_idx] > one_year[bbb_idx][d_idx],
            "5-year default {} should exceed 1-year {}",
            five_year[bbb_idx][d_idx],
            one_year[bbb_idx][d_idx]
        );
    }

    // -----------------------------------------------------------------------
    // Single BBB exposure tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_bbb_migration_stable_dominant() {
        let result = calculate_migration(&single_bbb_input()).unwrap();
        let bbb = &result.result.portfolio_results[0];
        // BBB stable probability should be the largest
        assert!(
            bbb.stable_probability > dec!(0.80),
            "BBB stable prob {} should be > 0.80",
            bbb.stable_probability
        );
    }

    #[test]
    fn test_bbb_upgrade_probability() {
        let result = calculate_migration(&single_bbb_input()).unwrap();
        let bbb = &result.result.portfolio_results[0];
        // BBB can upgrade to AAA, AA, A
        assert!(
            bbb.upgrade_probability > Decimal::ZERO,
            "Upgrade probability should be positive"
        );
        assert!(
            bbb.upgrade_probability < dec!(0.10),
            "Upgrade probability {} should be < 10%",
            bbb.upgrade_probability
        );
    }

    #[test]
    fn test_bbb_downgrade_includes_default() {
        let result = calculate_migration(&single_bbb_input()).unwrap();
        let bbb = &result.result.portfolio_results[0];
        // Downgrade includes BB, B, CCC, D
        assert!(
            bbb.downgrade_probability > bbb.default_probability,
            "Downgrade prob {} should exceed default prob {}",
            bbb.downgrade_probability,
            bbb.default_probability
        );
    }

    #[test]
    fn test_bbb_probabilities_sum_to_one() {
        let result = calculate_migration(&single_bbb_input()).unwrap();
        let bbb = &result.result.portfolio_results[0];
        let total = bbb.upgrade_probability + bbb.stable_probability + bbb.downgrade_probability;
        // Note: downgrade includes default, so total = upgrade + stable + downgrade
        // But default is counted inside downgrade, so we need upgrade + stable + downgrade_excl_default + default
        // Actually our definition: downgrade includes default, so total should ~ 1
        assert!(
            approx_eq(total, Decimal::ONE, dec!(0.01)),
            "Probabilities sum to {} expected ~1",
            total
        );
    }

    #[test]
    fn test_bbb_default_probability() {
        let result = calculate_migration(&single_bbb_input()).unwrap();
        let bbb = &result.result.portfolio_results[0];
        // 1-year BBB default ~ 0.18%
        assert!(
            approx_eq(bbb.default_probability, dec!(0.0018), dec!(0.001)),
            "BBB default prob {} expected ~0.0018",
            bbb.default_probability
        );
    }

    #[test]
    fn test_bbb_distribution_length() {
        let result = calculate_migration(&single_bbb_input()).unwrap();
        let bbb = &result.result.portfolio_results[0];
        assert_eq!(bbb.expected_rating_distribution.len(), 8);
    }

    // -----------------------------------------------------------------------
    // Multi-year cumulative default
    // -----------------------------------------------------------------------

    #[test]
    fn test_multi_year_default_ordering() {
        let mut input = single_bbb_input();
        input.time_horizon_years = 5;
        let result = calculate_migration(&input).unwrap();
        // Default probabilities should increase by rating (AAA < AA < ... < CCC)
        let defaults = &result.result.multi_year_default_prob;
        for i in 0..(defaults.len() - 2) {
            assert!(
                defaults[i].1 <= defaults[i + 1].1 + dec!(0.01),
                "{} default {} should be <= {} default {}",
                defaults[i].0,
                defaults[i].1,
                defaults[i + 1].0,
                defaults[i + 1].1
            );
        }
    }

    #[test]
    fn test_multi_year_default_increases_with_horizon() {
        let mut input1 = single_bbb_input();
        input1.time_horizon_years = 1;
        let mut input5 = single_bbb_input();
        input5.time_horizon_years = 5;

        let result1 = calculate_migration(&input1).unwrap();
        let result5 = calculate_migration(&input5).unwrap();

        // Find BBB default prob in each
        let bbb1 = result1
            .result
            .multi_year_default_prob
            .iter()
            .find(|(r, _)| r == "BBB")
            .unwrap()
            .1;
        let bbb5 = result5
            .result
            .multi_year_default_prob
            .iter()
            .find(|(r, _)| r == "BBB")
            .unwrap()
            .1;

        assert!(
            bbb5 > bbb1,
            "5-year BBB default {} should exceed 1-year {}",
            bbb5,
            bbb1
        );
    }

    #[test]
    fn test_default_state_absorbing() {
        let input = single_bbb_input();
        let result = calculate_migration(&input).unwrap();
        // D cumulative default should be 1.0
        let d_default = result
            .result
            .multi_year_default_prob
            .iter()
            .find(|(r, _)| r == "D")
            .unwrap()
            .1;
        assert!(
            approx_eq(d_default, Decimal::ONE, dec!(0.001)),
            "D default prob {} should be 1.0",
            d_default
        );
    }

    // -----------------------------------------------------------------------
    // Matrix quality checks
    // -----------------------------------------------------------------------

    #[test]
    fn test_sp_matrix_valid_stochastic() {
        let input = single_bbb_input();
        let result = calculate_migration(&input).unwrap();
        assert!(
            result.result.matrix_quality.is_valid_stochastic,
            "S&P matrix should be valid stochastic"
        );
    }

    #[test]
    fn test_sp_matrix_monotone() {
        let input = single_bbb_input();
        let result = calculate_migration(&input).unwrap();
        assert!(
            result.result.matrix_quality.is_monotone,
            "S&P matrix should be monotone"
        );
    }

    #[test]
    fn test_sp_matrix_absorbing_state() {
        let input = single_bbb_input();
        let result = calculate_migration(&input).unwrap();
        assert_eq!(result.result.matrix_quality.absorbing_state, "D");
    }

    #[test]
    fn test_sp_matrix_row_deviation_small() {
        let input = single_bbb_input();
        let result = calculate_migration(&input).unwrap();
        assert!(
            result.result.matrix_quality.max_row_deviation < dec!(0.001),
            "Max row deviation {} should be < 0.001",
            result.result.matrix_quality.max_row_deviation
        );
    }

    #[test]
    fn test_non_stochastic_matrix_detection() {
        let mut input = single_bbb_input();
        // Make a row that doesn't sum to 1
        input.transition_matrix.probabilities[0] = vec![
            dec!(0.5),
            dec!(0.1),
            dec!(0.1),
            dec!(0.1),
            dec!(0.1),
            dec!(0.0),
            dec!(0.0),
            dec!(0.0),
        ]; // sums to 0.9
        let result = calculate_migration(&input).unwrap();
        assert!(
            !result.result.matrix_quality.is_valid_stochastic,
            "Should detect non-stochastic matrix"
        );
    }

    // -----------------------------------------------------------------------
    // Identity matrix (no migration)
    // -----------------------------------------------------------------------

    #[test]
    fn test_identity_matrix_no_migration() {
        let mut input = single_bbb_input();
        let n = input.transition_matrix.ratings.len();
        // Set to identity: no transitions
        let mut id = vec![vec![Decimal::ZERO; n]; n];
        for i in 0..n {
            id[i][i] = Decimal::ONE;
        }
        input.transition_matrix.probabilities = id;

        let result = calculate_migration(&input).unwrap();
        let bbb = &result.result.portfolio_results[0];
        assert!(
            approx_eq(bbb.stable_probability, Decimal::ONE, dec!(0.001)),
            "Identity matrix: stable prob {} should be 1.0",
            bbb.stable_probability
        );
        assert!(
            approx_eq(bbb.upgrade_probability, Decimal::ZERO, dec!(0.001)),
            "Identity matrix: upgrade prob should be 0"
        );
        assert!(
            approx_eq(bbb.default_probability, Decimal::ZERO, dec!(0.001)),
            "Identity matrix: default prob should be 0"
        );
    }

    // -----------------------------------------------------------------------
    // High-yield dominated portfolio
    // -----------------------------------------------------------------------

    #[test]
    fn test_high_yield_portfolio() {
        let input = MigrationInput {
            initial_ratings: vec![
                RatedExposure {
                    name: "HY1".into(),
                    rating: "BB".into(),
                    exposure: dec!(500000),
                    maturity_years: dec!(3),
                    coupon_rate: dec!(0.07),
                },
                RatedExposure {
                    name: "HY2".into(),
                    rating: "B".into(),
                    exposure: dec!(300000),
                    maturity_years: dec!(4),
                    coupon_rate: dec!(0.09),
                },
                RatedExposure {
                    name: "HY3".into(),
                    rating: "CCC".into(),
                    exposure: dec!(200000),
                    maturity_years: dec!(2),
                    coupon_rate: dec!(0.12),
                },
            ],
            transition_matrix: sp_transition_matrix(),
            time_horizon_years: 1,
            spread_curve: standard_spread_curve(),
        };
        let result = calculate_migration(&input).unwrap();
        // CCC should have highest default prob
        let ccc = result
            .result
            .portfolio_results
            .iter()
            .find(|r| r.current_rating == "CCC")
            .unwrap();
        let bb = result
            .result
            .portfolio_results
            .iter()
            .find(|r| r.current_rating == "BB")
            .unwrap();
        assert!(
            ccc.default_probability > bb.default_probability,
            "CCC default {} should exceed BB default {}",
            ccc.default_probability,
            bb.default_probability
        );
    }

    // -----------------------------------------------------------------------
    // Investment-grade portfolio
    // -----------------------------------------------------------------------

    #[test]
    fn test_ig_portfolio_low_default() {
        let input = MigrationInput {
            initial_ratings: vec![
                RatedExposure {
                    name: "IG1".into(),
                    rating: "AAA".into(),
                    exposure: dec!(500000),
                    maturity_years: dec!(7),
                    coupon_rate: dec!(0.03),
                },
                RatedExposure {
                    name: "IG2".into(),
                    rating: "AA".into(),
                    exposure: dec!(500000),
                    maturity_years: dec!(5),
                    coupon_rate: dec!(0.035),
                },
            ],
            transition_matrix: sp_transition_matrix(),
            time_horizon_years: 1,
            spread_curve: standard_spread_curve(),
        };
        let result = calculate_migration(&input).unwrap();
        // Both should have near-zero default probability
        for r in &result.result.portfolio_results {
            assert!(
                r.default_probability < dec!(0.001),
                "{} default prob {} should be < 0.1%",
                r.name,
                r.default_probability
            );
        }
    }

    // -----------------------------------------------------------------------
    // Mark-to-market value change
    // -----------------------------------------------------------------------

    #[test]
    fn test_bbb_expected_value_change_small() {
        let result = calculate_migration(&single_bbb_input()).unwrap();
        let bbb = &result.result.portfolio_results[0];
        // Expected value change should be small and likely negative (asymmetric risk)
        assert!(
            bbb.expected_value_change_pct.abs() < dec!(5),
            "Expected value change {} should be small",
            bbb.expected_value_change_pct
        );
    }

    #[test]
    fn test_migration_var_positive() {
        let result = calculate_migration(&single_bbb_input()).unwrap();
        assert!(
            result.result.portfolio_migration_var >= Decimal::ZERO,
            "Migration VaR {} should be non-negative",
            result.result.portfolio_migration_var
        );
    }

    // -----------------------------------------------------------------------
    // Portfolio-level migration loss
    // -----------------------------------------------------------------------

    #[test]
    fn test_portfolio_migration_loss_finite() {
        let input = MigrationInput {
            initial_ratings: vec![
                RatedExposure {
                    name: "A".into(),
                    rating: "A".into(),
                    exposure: dec!(500000),
                    maturity_years: dec!(5),
                    coupon_rate: dec!(0.04),
                },
                RatedExposure {
                    name: "B".into(),
                    rating: "BBB".into(),
                    exposure: dec!(500000),
                    maturity_years: dec!(5),
                    coupon_rate: dec!(0.05),
                },
            ],
            transition_matrix: sp_transition_matrix(),
            time_horizon_years: 1,
            spread_curve: standard_spread_curve(),
        };
        let result = calculate_migration(&input).unwrap();
        // Expected migration loss should be finite
        assert!(
            result.result.portfolio_expected_migration_loss.abs() < dec!(10000000),
            "Migration loss {} should be finite",
            result.result.portfolio_expected_migration_loss
        );
    }

    // -----------------------------------------------------------------------
    // Validation tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_non_square_matrix_error() {
        let mut input = single_bbb_input();
        input.transition_matrix.probabilities.pop(); // Remove last row
        let result = calculate_migration(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_unknown_rating_error() {
        let mut input = single_bbb_input();
        input.initial_ratings[0].rating = "XYZ".into();
        let result = calculate_migration(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_exposures_error() {
        let mut input = single_bbb_input();
        input.initial_ratings.clear();
        let result = calculate_migration(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_zero_horizon_error() {
        let mut input = single_bbb_input();
        input.time_horizon_years = 0;
        let result = calculate_migration(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_bad_spread_rating_error() {
        let mut input = single_bbb_input();
        input.spread_curve.push(RatingSpread {
            rating: "INVALID".into(),
            spread_bps: dec!(100),
        });
        let result = calculate_migration(&input);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // All-default row test
    // -----------------------------------------------------------------------

    #[test]
    fn test_all_default_row() {
        let input = single_bbb_input();
        let n = input.transition_matrix.ratings.len();
        let d_idx = n - 1;
        // Verify that default row is absorbing
        let d_row = &input.transition_matrix.probabilities[d_idx];
        assert!(
            approx_eq(d_row[d_idx], Decimal::ONE, dec!(0.001)),
            "Default row self-transition should be 1.0"
        );
    }

    // -----------------------------------------------------------------------
    // Migration VaR ordering
    // -----------------------------------------------------------------------

    #[test]
    fn test_migration_var_hy_exceeds_ig() {
        let ig_input = MigrationInput {
            initial_ratings: vec![RatedExposure {
                name: "IG".into(),
                rating: "AA".into(),
                exposure: dec!(1000000),
                maturity_years: dec!(5),
                coupon_rate: dec!(0.04),
            }],
            transition_matrix: sp_transition_matrix(),
            time_horizon_years: 1,
            spread_curve: standard_spread_curve(),
        };
        let hy_input = MigrationInput {
            initial_ratings: vec![RatedExposure {
                name: "HY".into(),
                rating: "B".into(),
                exposure: dec!(1000000),
                maturity_years: dec!(5),
                coupon_rate: dec!(0.08),
            }],
            transition_matrix: sp_transition_matrix(),
            time_horizon_years: 1,
            spread_curve: standard_spread_curve(),
        };

        let ig_result = calculate_migration(&ig_input).unwrap();
        let hy_result = calculate_migration(&hy_input).unwrap();

        assert!(
            hy_result.result.portfolio_migration_var >= ig_result.result.portfolio_migration_var,
            "HY VaR {} should >= IG VaR {}",
            hy_result.result.portfolio_migration_var,
            ig_result.result.portfolio_migration_var
        );
    }

    // -----------------------------------------------------------------------
    // Metadata
    // -----------------------------------------------------------------------

    #[test]
    fn test_metadata_populated() {
        let result = calculate_migration(&single_bbb_input()).unwrap();
        assert!(!result.methodology.is_empty());
        assert!(!result.metadata.version.is_empty());
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
    }
}
