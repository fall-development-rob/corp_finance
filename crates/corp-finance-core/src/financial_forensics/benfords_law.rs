//! Benford's Law digit analysis for financial fraud detection.
//!
//! Tests whether a dataset of financial figures follows the expected
//! digit-frequency distribution predicted by Benford's Law.
//! Uses a chi-squared goodness-of-fit test.
//!
//! All arithmetic uses `rust_decimal::Decimal`. No `f64`.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn decimal_ln(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    let ln2 = dec!(0.6931471805599453);
    let mut val = x;
    let mut adjust = Decimal::ZERO;
    while val > dec!(2.0) {
        val /= dec!(2);
        adjust += ln2;
    }
    while val < dec!(0.5) {
        val *= dec!(2);
        adjust -= ln2;
    }
    let z = (val - Decimal::ONE) / (val + Decimal::ONE);
    let z2 = z * z;
    let mut term = z;
    let mut sum = z;
    for k in 1u32..40 {
        term *= z2;
        sum += term / Decimal::from(2 * k + 1);
    }
    dec!(2) * sum + adjust
}

fn decimal_log10(x: Decimal) -> Decimal {
    decimal_ln(x) / dec!(2.302585092994046)
}

fn decimal_sqrt(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    let mut g = if x > Decimal::ONE {
        x / dec!(2)
    } else {
        Decimal::ONE
    };
    for _ in 0..30 {
        g = (g + x / g) / dec!(2);
    }
    g
}

fn first_digit(val: Decimal) -> Option<u32> {
    if val.abs() == Decimal::ZERO {
        return None;
    }
    for ch in val.abs().to_string().chars() {
        if ch.is_ascii_digit() && ch != '0' {
            return ch.to_digit(10);
        }
    }
    None
}

fn second_digit(val: Decimal) -> Option<u32> {
    if val.abs() == Decimal::ZERO {
        return None;
    }
    let mut found_first = false;
    for ch in val.abs().to_string().chars() {
        if ch == '.' {
            continue;
        }
        if !found_first {
            if ch.is_ascii_digit() && ch != '0' {
                found_first = true;
            }
        } else if ch.is_ascii_digit() {
            return ch.to_digit(10);
        }
    }
    if found_first {
        Some(0)
    } else {
        None
    }
}

fn first_two_digits(val: Decimal) -> Option<u32> {
    Some(first_digit(val)? * 10 + second_digit(val)?)
}

// ---------------------------------------------------------------------------
// Input / Output
// ---------------------------------------------------------------------------

/// Input for Benford's Law analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenfordsLawInput {
    pub data_points: Vec<Decimal>,
    pub test_type: String,
    pub significance_level: Decimal,
}

/// Frequency result for a single digit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigitFrequency {
    pub digit: u32,
    pub observed_count: u32,
    pub observed_pct: Decimal,
    pub expected_pct: Decimal,
    pub deviation: Decimal,
}

/// Output of Benford's Law analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenfordsLawOutput {
    pub digit_frequencies: Vec<DigitFrequency>,
    pub chi_squared: Decimal,
    pub degrees_of_freedom: u32,
    pub critical_value: Decimal,
    pub conforms: bool,
    pub max_deviation_digit: u32,
    pub max_deviation: Decimal,
    pub mean_absolute_deviation: Decimal,
    pub suspect_digits: Vec<u32>,
}

// ---------------------------------------------------------------------------
// Expected frequencies
// ---------------------------------------------------------------------------

fn benford_first_digit(d: u32) -> Decimal {
    decimal_log10(Decimal::ONE + Decimal::ONE / Decimal::from(d))
}

fn benford_second_digit(d: u32) -> Decimal {
    (1u32..=9)
        .map(|k| decimal_log10(Decimal::ONE + Decimal::ONE / Decimal::from(10 * k + d)))
        .sum()
}

fn benford_first_two_digits(dd: u32) -> Decimal {
    decimal_log10(Decimal::ONE + Decimal::ONE / Decimal::from(dd))
}

fn chi_squared_critical(df: u32, alpha: Decimal) -> Decimal {
    match df {
        8 if alpha <= dec!(0.01) => dec!(20.090),
        8 if alpha <= dec!(0.05) => dec!(15.507),
        8 if alpha <= dec!(0.10) => dec!(13.362),
        8 => dec!(11.030),
        9 if alpha <= dec!(0.01) => dec!(21.666),
        9 if alpha <= dec!(0.05) => dec!(16.919),
        9 if alpha <= dec!(0.10) => dec!(14.684),
        9 => dec!(12.242),
        89 if alpha <= dec!(0.01) => dec!(122.942),
        89 if alpha <= dec!(0.05) => dec!(112.022),
        89 if alpha <= dec!(0.10) => dec!(106.469),
        89 => dec!(100.535),
        _ => {
            // Wilson-Hilferty approximation
            let z = if alpha <= dec!(0.01) {
                dec!(2.326)
            } else if alpha <= dec!(0.05) {
                dec!(1.645)
            } else if alpha <= dec!(0.10) {
                dec!(1.282)
            } else {
                dec!(1.036)
            };
            let df_d = Decimal::from(df);
            let frac = dec!(2) / (dec!(9) * df_d);
            let base = Decimal::ONE - frac + z * decimal_sqrt(frac);
            df_d * base * base * base
        }
    }
}

// ---------------------------------------------------------------------------
// Core function
// ---------------------------------------------------------------------------

/// Perform Benford's Law digit analysis on financial data.
pub fn analyze_benfords_law(input: &BenfordsLawInput) -> CorpFinanceResult<BenfordsLawOutput> {
    if input.data_points.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one data point is required.".into(),
        ));
    }
    if input.significance_level <= Decimal::ZERO || input.significance_level >= Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "significance_level".into(),
            reason: "Must be between 0 and 1 exclusive.".into(),
        });
    }
    match input.test_type.as_str() {
        "first_digit" => run_analysis(input, 1, 9, benford_first_digit, |v| first_digit(*v)),
        "second_digit" => run_analysis(input, 0, 9, benford_second_digit, |v| second_digit(*v)),
        "first_two_digits" => run_analysis(input, 10, 99, benford_first_two_digits, |v| {
            first_two_digits(*v)
        }),
        _ => Err(CorpFinanceError::InvalidInput {
            field: "test_type".into(),
            reason: "Must be 'first_digit', 'second_digit', or 'first_two_digits'.".into(),
        }),
    }
}

/// Shared analysis logic for all digit test types.
fn run_analysis(
    input: &BenfordsLawInput,
    start: u32,
    end: u32,
    expected_fn: fn(u32) -> Decimal,
    extract_fn: impl Fn(&Decimal) -> Option<u32>,
) -> CorpFinanceResult<BenfordsLawOutput> {
    let digits: Vec<u32> = input.data_points.iter().filter_map(&extract_fn).collect();
    let n = Decimal::from(digits.len() as u64);
    if n == Decimal::ZERO {
        return Err(CorpFinanceError::InsufficientData(
            "No valid extractable digits.".into(),
        ));
    }
    let hundred = dec!(100);
    let num_digits = end - start + 1;
    let mut frequencies = Vec::with_capacity(num_digits as usize);
    let (mut chi_sq, mut max_dev, mut total_abs_dev) =
        (Decimal::ZERO, Decimal::ZERO, Decimal::ZERO);
    let mut max_dev_digit = start;
    let mut suspect_digits = Vec::new();

    for d in start..=end {
        let count = digits.iter().filter(|&&x| x == d).count() as u32;
        let observed_pct = Decimal::from(count) / n * hundred;
        let expected_pct = expected_fn(d) * hundred;
        let deviation = observed_pct - expected_pct;
        let expected_count = expected_fn(d) * n;
        if expected_count > Decimal::ZERO {
            let diff = Decimal::from(count) - expected_count;
            chi_sq += diff * diff / expected_count;
        }
        let p = expected_fn(d);
        let std_dev = decimal_sqrt(p * (Decimal::ONE - p) / n) * hundred;
        if deviation.abs() > dec!(2) * std_dev && std_dev > Decimal::ZERO {
            suspect_digits.push(d);
        }
        if deviation.abs() > max_dev {
            max_dev = deviation.abs();
            max_dev_digit = d;
        }
        total_abs_dev += deviation.abs();
        frequencies.push(DigitFrequency {
            digit: d,
            observed_count: count,
            observed_pct,
            expected_pct,
            deviation,
        });
    }
    let df = num_digits - 1;
    let critical = chi_squared_critical(df, input.significance_level);
    Ok(BenfordsLawOutput {
        digit_frequencies: frequencies,
        chi_squared: chi_sq,
        degrees_of_freedom: df,
        critical_value: critical,
        conforms: chi_sq < critical,
        max_deviation_digit: max_dev_digit,
        max_deviation: max_dev,
        mean_absolute_deviation: total_abs_dev / Decimal::from(num_digits),
        suspect_digits,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn approx_eq(a: Decimal, b: Decimal, eps: Decimal) -> bool {
        (a - b).abs() < eps
    }

    fn benford_data() -> Vec<Decimal> {
        let mut data = Vec::new();
        let mut v = dec!(2);
        for _ in 0..60 {
            data.push(v);
            v *= dec!(2);
        }
        v = dec!(3);
        for _ in 0..38 {
            data.push(v);
            v *= dec!(3);
        }
        let (mut a, mut b) = (Decimal::ONE, Decimal::ONE);
        for _ in 0..60 {
            data.push(a);
            let c = a + b;
            a = b;
            b = c;
        }
        for i in 1u32..=50 {
            data.push(Decimal::from(i) * dec!(137));
        }
        data
    }

    fn uniform_data() -> Vec<Decimal> {
        (1u32..=9)
            .flat_map(|d| std::iter::repeat(Decimal::from(d) * dec!(100)).take(50))
            .collect()
    }

    fn fd_input(data: Vec<Decimal>) -> BenfordsLawInput {
        BenfordsLawInput {
            data_points: data,
            test_type: "first_digit".into(),
            significance_level: dec!(0.05),
        }
    }

    #[test]
    fn test_benford_data_conforms() {
        let out = analyze_benfords_law(&fd_input(benford_data())).unwrap();
        assert!(out.conforms, "chi2={}", out.chi_squared);
    }

    #[test]
    fn test_uniform_does_not_conform() {
        assert!(
            !analyze_benfords_law(&fd_input(uniform_data()))
                .unwrap()
                .conforms
        );
    }

    #[test]
    fn test_first_digit_nine_freqs() {
        assert_eq!(
            analyze_benfords_law(&fd_input(benford_data()))
                .unwrap()
                .digit_frequencies
                .len(),
            9
        );
    }

    #[test]
    fn test_first_digit_df_8() {
        assert_eq!(
            analyze_benfords_law(&fd_input(benford_data()))
                .unwrap()
                .degrees_of_freedom,
            8
        );
    }

    #[test]
    fn test_benford_p1() {
        assert!(approx_eq(
            benford_first_digit(1),
            dec!(0.30103),
            dec!(0.001)
        ));
    }

    #[test]
    fn test_benford_p9() {
        assert!(approx_eq(
            benford_first_digit(9),
            dec!(0.04576),
            dec!(0.001)
        ));
    }

    #[test]
    fn test_first_digit_sums_one() {
        let t: Decimal = (1u32..=9).map(benford_first_digit).sum();
        assert!(approx_eq(t, Decimal::ONE, dec!(0.001)));
    }

    #[test]
    fn test_second_digit_sums_one() {
        let t: Decimal = (0u32..=9).map(benford_second_digit).sum();
        assert!(approx_eq(t, Decimal::ONE, dec!(0.001)));
    }

    #[test]
    fn test_second_digit_df_9() {
        let inp = BenfordsLawInput {
            data_points: benford_data(),
            test_type: "second_digit".into(),
            significance_level: dec!(0.05),
        };
        assert_eq!(analyze_benfords_law(&inp).unwrap().degrees_of_freedom, 9);
    }

    #[test]
    fn test_second_digit_ten_freqs() {
        let inp = BenfordsLawInput {
            data_points: benford_data(),
            test_type: "second_digit".into(),
            significance_level: dec!(0.05),
        };
        assert_eq!(
            analyze_benfords_law(&inp).unwrap().digit_frequencies.len(),
            10
        );
    }

    #[test]
    fn test_chi_squared_non_negative() {
        assert!(
            analyze_benfords_law(&fd_input(benford_data()))
                .unwrap()
                .chi_squared
                >= Decimal::ZERO
        );
    }

    #[test]
    fn test_cv_df8_a005() {
        assert!(approx_eq(
            chi_squared_critical(8, dec!(0.05)),
            dec!(15.507),
            dec!(0.001)
        ));
    }

    #[test]
    fn test_cv_df9_a001() {
        assert!(approx_eq(
            chi_squared_critical(9, dec!(0.01)),
            dec!(21.666),
            dec!(0.001)
        ));
    }

    #[test]
    fn test_max_dev_digit() {
        assert_eq!(
            analyze_benfords_law(&fd_input(uniform_data()))
                .unwrap()
                .max_deviation_digit,
            1
        );
    }

    #[test]
    fn test_mad_positive() {
        assert!(
            analyze_benfords_law(&fd_input(uniform_data()))
                .unwrap()
                .mean_absolute_deviation
                > Decimal::ZERO
        );
    }

    #[test]
    fn test_suspect_digits() {
        assert!(!analyze_benfords_law(&fd_input(uniform_data()))
            .unwrap()
            .suspect_digits
            .is_empty());
    }

    #[test]
    fn test_empty_data() {
        let inp = BenfordsLawInput {
            data_points: vec![],
            test_type: "first_digit".into(),
            significance_level: dec!(0.05),
        };
        assert!(analyze_benfords_law(&inp).is_err());
    }

    #[test]
    fn test_alpha_zero() {
        let inp = BenfordsLawInput {
            data_points: vec![dec!(100)],
            test_type: "first_digit".into(),
            significance_level: Decimal::ZERO,
        };
        assert!(analyze_benfords_law(&inp).is_err());
    }

    #[test]
    fn test_alpha_one() {
        let inp = BenfordsLawInput {
            data_points: vec![dec!(100)],
            test_type: "first_digit".into(),
            significance_level: Decimal::ONE,
        };
        assert!(analyze_benfords_law(&inp).is_err());
    }

    #[test]
    fn test_bad_type() {
        let inp = BenfordsLawInput {
            data_points: vec![dec!(100)],
            test_type: "third_digit".into(),
            significance_level: dec!(0.05),
        };
        assert!(analyze_benfords_law(&inp).is_err());
    }

    #[test]
    fn test_single_point() {
        let out = analyze_benfords_law(&fd_input(vec![dec!(123)])).unwrap();
        assert_eq!(out.digit_frequencies[0].observed_count, 1);
        assert_eq!(out.digit_frequencies[0].digit, 1);
    }

    #[test]
    fn test_first_two_digits_df_89() {
        let inp = BenfordsLawInput {
            data_points: benford_data(),
            test_type: "first_two_digits".into(),
            significance_level: dec!(0.05),
        };
        let out = analyze_benfords_law(&inp).unwrap();
        assert_eq!(out.degrees_of_freedom, 89);
        assert_eq!(out.digit_frequencies.len(), 90);
    }

    #[test]
    fn test_first_two_sums_one() {
        let t: Decimal = (10u32..=99).map(benford_first_two_digits).sum();
        assert!(approx_eq(t, Decimal::ONE, dec!(0.001)));
    }

    #[test]
    fn test_counts_sum() {
        let data = benford_data();
        let exp = data.iter().filter(|v| first_digit(**v).is_some()).count() as u32;
        let out = analyze_benfords_law(&fd_input(data)).unwrap();
        let total: u32 = out.digit_frequencies.iter().map(|f| f.observed_count).sum();
        assert_eq!(total, exp);
    }

    #[test]
    fn test_negatives() {
        let data = vec![dec!(-123), dec!(-456), dec!(-789), dec!(-234), dec!(-567)];
        let out = analyze_benfords_law(&fd_input(data)).unwrap();
        let total: u32 = out.digit_frequencies.iter().map(|f| f.observed_count).sum();
        assert_eq!(total, 5);
    }

    #[test]
    fn test_serde() {
        let out = analyze_benfords_law(&fd_input(benford_data())).unwrap();
        let j = serde_json::to_string(&out).unwrap();
        let _: BenfordsLawOutput = serde_json::from_str(&j).unwrap();
    }

    #[test]
    fn test_log10_ten() {
        assert!(approx_eq(
            decimal_log10(dec!(10)),
            Decimal::ONE,
            dec!(0.001)
        ));
    }
}
