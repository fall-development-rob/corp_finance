//! Monetary policy analysis tools.
//!
//! Provides Taylor Rule rate prescription, Phillips Curve inflation dynamics,
//! Okun's Law output-gap estimation, and composite policy assessment including
//! recession-risk scoring, inflation-trend detection, and narrative
//! recommendations.
//!
//! All calculations use `rust_decimal::Decimal` for precision. No `f64`.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{CorpFinanceError, CorpFinanceResult};

// ---------------------------------------------------------------------------
// Input / Output types
// ---------------------------------------------------------------------------

/// Input for monetary policy analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonetaryPolicyInput {
    /// Actual inflation rate (e.g., 0.035 = 3.5%).
    pub current_inflation: Decimal,
    /// Central bank target inflation (e.g., 0.02).
    pub target_inflation: Decimal,
    /// Real GDP growth rate.
    pub current_gdp_growth: Decimal,
    /// Estimated potential GDP growth rate.
    pub potential_gdp_growth: Decimal,
    /// Actual unemployment rate.
    pub current_unemployment: Decimal,
    /// NAIRU estimate.
    pub natural_unemployment: Decimal,
    /// Current central bank policy rate.
    pub current_policy_rate: Decimal,
    /// Neutral real interest rate (r*).
    pub neutral_real_rate: Decimal,
    /// Taylor rule weight on inflation gap (default 0.5).
    pub inflation_weight: Decimal,
    /// Taylor rule weight on output gap (default 0.5).
    pub output_weight: Decimal,
    /// Historical inflation observations for trend analysis (optional).
    pub historical_inflation: Vec<Decimal>,
    /// Historical unemployment observations for Phillips curve (optional).
    pub historical_unemployment: Vec<Decimal>,
}

/// Taylor Rule result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaylorRuleResult {
    /// Prescribed policy rate.
    pub prescribed_rate: Decimal,
    /// Neutral rate = r* + target_inflation.
    pub neutral_rate: Decimal,
    /// Inflation gap = current_inflation - target_inflation.
    pub inflation_gap: Decimal,
    /// Output gap = current_gdp_growth - potential_gdp_growth.
    pub output_gap: Decimal,
    /// Inflation weight x inflation gap.
    pub inflation_component: Decimal,
    /// Output weight x output gap.
    pub output_component: Decimal,
    /// Rate deviation = current_policy_rate - prescribed_rate.
    pub rate_deviation: Decimal,
    /// "Accommodative", "Neutral", or "Restrictive".
    pub policy_stance: String,
}

/// Phillips Curve result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhillipsCurveResult {
    /// Unemployment gap = current - natural.
    pub unemployment_gap: Decimal,
    /// Implied change in inflation from the gap.
    pub implied_inflation_change: Decimal,
    /// Estimated Phillips curve slope.
    pub phillips_coefficient: Decimal,
    /// NAIRU estimate used.
    pub nairu_estimate: Decimal,
    /// Output cost per 1% disinflation.
    pub sacrifice_ratio: Decimal,
}

/// Okun's Law result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OkunResult {
    /// Output gap as a percentage.
    pub output_gap_pct: Decimal,
    /// Unemployment gap = current - natural.
    pub unemployment_gap: Decimal,
    /// Okun coefficient (typically -2 to -3).
    pub okun_coefficient: Decimal,
    /// Implied GDP loss from excess unemployment.
    pub implied_gdp_loss: Decimal,
}

/// Full monetary policy analysis output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonetaryPolicyOutput {
    /// Taylor Rule analysis.
    pub taylor_rule: TaylorRuleResult,
    /// Phillips Curve analysis.
    pub phillips_curve: PhillipsCurveResult,
    /// Okun's Law analysis.
    pub okun_law: OkunResult,
    /// "Rising", "Falling", or "Stable".
    pub inflation_trend: String,
    /// "Low", "Moderate", "High", or "Elevated".
    pub recession_risk: String,
    /// Narrative policy recommendation.
    pub policy_recommendation: String,
    /// Methodology description.
    pub methodology: String,
    /// Key assumptions.
    pub assumptions: HashMap<String, String>,
    /// Warnings about unusual inputs.
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Analyse monetary policy conditions using the Taylor Rule, Phillips Curve,
/// and Okun's Law frameworks.
pub fn analyze_monetary_policy(
    input: &MonetaryPolicyInput,
) -> CorpFinanceResult<MonetaryPolicyOutput> {
    let warnings = validate_input(input)?;

    let taylor = compute_taylor_rule(input);
    let phillips = compute_phillips_curve(input);
    let okun = compute_okun_law(input);
    let inflation_trend = compute_inflation_trend(&input.historical_inflation);
    let recession_risk = compute_recession_risk(input, &taylor, &okun);
    let policy_recommendation =
        build_recommendation(&taylor, &phillips, &inflation_trend, &recession_risk);

    let mut assumptions = HashMap::new();
    assumptions.insert(
        "taylor_rule".into(),
        format!(
            "Weights: inflation={}, output={}",
            input.inflation_weight, input.output_weight
        ),
    );
    assumptions.insert(
        "okun_coefficient".into(),
        format!("{}", okun.okun_coefficient),
    );
    assumptions.insert("nairu".into(), format!("{}", input.natural_unemployment));
    assumptions.insert("r_star".into(), format!("{}", input.neutral_real_rate));

    Ok(MonetaryPolicyOutput {
        taylor_rule: taylor,
        phillips_curve: phillips,
        okun_law: okun,
        inflation_trend,
        recession_risk,
        policy_recommendation,
        methodology: "Taylor Rule (1993) with Phillips Curve and Okun's Law supplements. \
                       Inflation trend via OLS slope on historical data. Recession risk \
                       scored on rate inversion, unemployment, and output gap signals."
            .into(),
        assumptions,
        warnings,
    })
}

// ---------------------------------------------------------------------------
// Internal: Taylor Rule
// ---------------------------------------------------------------------------

fn compute_taylor_rule(input: &MonetaryPolicyInput) -> TaylorRuleResult {
    let neutral_rate = input.neutral_real_rate + input.target_inflation;
    let inflation_gap = input.current_inflation - input.target_inflation;
    let output_gap = input.current_gdp_growth - input.potential_gdp_growth;

    let inflation_component = input.inflation_weight * inflation_gap;
    let output_component = input.output_weight * output_gap;

    let prescribed_rate = neutral_rate + inflation_component + output_component;
    let rate_deviation = input.current_policy_rate - prescribed_rate;

    let policy_stance = if rate_deviation > dec!(0.005) {
        "Restrictive".to_string()
    } else if rate_deviation < dec!(-0.005) {
        "Accommodative".to_string()
    } else {
        "Neutral".to_string()
    };

    TaylorRuleResult {
        prescribed_rate,
        neutral_rate,
        inflation_gap,
        output_gap,
        inflation_component,
        output_component,
        rate_deviation,
        policy_stance,
    }
}

// ---------------------------------------------------------------------------
// Internal: Phillips Curve
// ---------------------------------------------------------------------------

fn compute_phillips_curve(input: &MonetaryPolicyInput) -> PhillipsCurveResult {
    let unemployment_gap = input.current_unemployment - input.natural_unemployment;
    let nairu_estimate = input.natural_unemployment;

    // Estimate Phillips coefficient from historical data if available,
    // otherwise use default -0.5.
    let phillips_coefficient =
        estimate_phillips_coefficient(&input.historical_inflation, &input.historical_unemployment);

    // Implied inflation change = -coefficient * unemployment_gap
    // With a negative coefficient (e.g. -0.5) and the spec's formula:
    // pi_change = -beta * (u - u*) where beta is the (negative) slope.
    let implied_inflation_change = -phillips_coefficient * unemployment_gap;

    // Sacrifice ratio = 1 / |coefficient|
    let abs_coeff = if phillips_coefficient < Decimal::ZERO {
        -phillips_coefficient
    } else {
        phillips_coefficient
    };
    let sacrifice_ratio = if abs_coeff.is_zero() {
        Decimal::ZERO
    } else {
        Decimal::ONE / abs_coeff
    };

    PhillipsCurveResult {
        unemployment_gap,
        implied_inflation_change,
        phillips_coefficient,
        nairu_estimate,
        sacrifice_ratio,
    }
}

/// Estimate the Phillips coefficient via simple OLS on (unemployment, inflation)
/// pairs.  Falls back to -0.5 when fewer than 3 paired observations exist.
fn estimate_phillips_coefficient(
    inflation_history: &[Decimal],
    unemployment_history: &[Decimal],
) -> Decimal {
    let n = inflation_history.len().min(unemployment_history.len());
    if n < 3 {
        return dec!(-0.5);
    }

    // Simple OLS: slope = (n*sum(xy) - sum(x)*sum(y)) / (n*sum(x^2) - (sum(x))^2)
    // x = unemployment, y = inflation
    let n_dec = Decimal::from(n as u32);
    let mut sum_x = Decimal::ZERO;
    let mut sum_y = Decimal::ZERO;
    let mut sum_xy = Decimal::ZERO;
    let mut sum_x2 = Decimal::ZERO;

    for i in 0..n {
        let x = unemployment_history[i];
        let y = inflation_history[i];
        sum_x += x;
        sum_y += y;
        sum_xy += x * y;
        sum_x2 += x * x;
    }

    let denom = n_dec * sum_x2 - sum_x * sum_x;
    if denom.is_zero() {
        return dec!(-0.5);
    }

    (n_dec * sum_xy - sum_x * sum_y) / denom
}

// ---------------------------------------------------------------------------
// Internal: Okun's Law
// ---------------------------------------------------------------------------

fn compute_okun_law(input: &MonetaryPolicyInput) -> OkunResult {
    let okun_coefficient = dec!(-2);
    let unemployment_gap = input.current_unemployment - input.natural_unemployment;

    // Output gap = -kappa * (u - u*) where kappa is the Okun coefficient (-2).
    // -(-2) * gap = 2 * gap.
    let output_gap_pct = -okun_coefficient * unemployment_gap;

    // Implied GDP loss = kappa * unemployment_gap * potential_gdp_growth
    let implied_gdp_loss = okun_coefficient * unemployment_gap * input.potential_gdp_growth;

    OkunResult {
        output_gap_pct,
        unemployment_gap,
        okun_coefficient,
        implied_gdp_loss,
    }
}

// ---------------------------------------------------------------------------
// Internal: inflation trend
// ---------------------------------------------------------------------------

fn compute_inflation_trend(history: &[Decimal]) -> String {
    if history.len() < 3 {
        return "Stable".to_string();
    }

    // Simple OLS slope on (t, inflation)
    let n = history.len();
    let n_dec = Decimal::from(n as u32);
    let mut sum_t = Decimal::ZERO;
    let mut sum_y = Decimal::ZERO;
    let mut sum_ty = Decimal::ZERO;
    let mut sum_t2 = Decimal::ZERO;

    for (i, &val) in history.iter().enumerate() {
        let t = Decimal::from(i as u32);
        sum_t += t;
        sum_y += val;
        sum_ty += t * val;
        sum_t2 += t * t;
    }

    let denom = n_dec * sum_t2 - sum_t * sum_t;
    if denom.is_zero() {
        return "Stable".to_string();
    }

    let slope = (n_dec * sum_ty - sum_t * sum_y) / denom;

    if slope > dec!(0.002) {
        "Rising".to_string()
    } else if slope < dec!(-0.002) {
        "Falling".to_string()
    } else {
        "Stable".to_string()
    }
}

// ---------------------------------------------------------------------------
// Internal: recession risk
// ---------------------------------------------------------------------------

fn compute_recession_risk(
    input: &MonetaryPolicyInput,
    taylor: &TaylorRuleResult,
    okun: &OkunResult,
) -> String {
    let mut score = Decimal::ZERO;

    // Signal 1: policy rate > neutral rate by > 100bps (inverted yield curve proxy)
    let neutral = input.neutral_real_rate + input.target_inflation;
    if input.current_policy_rate > neutral + dec!(0.01) {
        score += Decimal::ONE;
    }

    // Signal 2: unemployment above NAIRU
    if input.current_unemployment > input.natural_unemployment + dec!(0.005) {
        score += Decimal::ONE;
    }

    // Signal 3: negative output gap (from Okun's Law)
    if okun.output_gap_pct < dec!(-0.005) {
        score += Decimal::ONE;
    }

    // Signal 4: Taylor rule suggests rate is too restrictive by >200bps
    if taylor.rate_deviation > dec!(0.02) {
        score += Decimal::ONE;
    }

    if score >= dec!(3) {
        "Elevated".to_string()
    } else if score >= dec!(2) {
        "High".to_string()
    } else if score >= dec!(1) {
        "Moderate".to_string()
    } else {
        "Low".to_string()
    }
}

// ---------------------------------------------------------------------------
// Internal: policy recommendation
// ---------------------------------------------------------------------------

fn build_recommendation(
    taylor: &TaylorRuleResult,
    phillips: &PhillipsCurveResult,
    inflation_trend: &str,
    recession_risk: &str,
) -> String {
    let mut parts: Vec<String> = Vec::new();

    // Rate prescription
    parts.push(format!(
        "Taylor Rule prescribes a rate of {:.4}% (current: {:.4}%, deviation: {:.4}%).",
        taylor.prescribed_rate * dec!(100),
        (taylor.prescribed_rate + taylor.rate_deviation) * dec!(100),
        taylor.rate_deviation * dec!(100),
    ));

    // Stance
    match taylor.policy_stance.as_str() {
        "Accommodative" => {
            parts.push(
                "Current stance is accommodative relative to the rule. \
                 Consider tightening if inflation pressures persist."
                    .into(),
            );
        }
        "Restrictive" => {
            parts.push(
                "Current stance is restrictive relative to the rule. \
                 Consider easing if growth weakens or inflation moderates."
                    .into(),
            );
        }
        _ => {
            parts.push("Current stance is broadly neutral relative to the Taylor Rule.".into());
        }
    }

    // Inflation dynamics
    if phillips.unemployment_gap < Decimal::ZERO {
        parts.push(
            "Labour market is tight (unemployment below NAIRU), \
             suggesting upward inflation pressure."
                .into(),
        );
    } else if phillips.unemployment_gap > dec!(0.01) {
        parts.push(
            "Labour market slack is significant, which should help contain inflation.".into(),
        );
    }

    if inflation_trend == "Rising" {
        parts.push("Inflation is on a rising trend.".into());
    } else if inflation_trend == "Falling" {
        parts.push("Inflation is on a falling trend.".into());
    }

    // Recession risk
    match recession_risk {
        "High" | "Elevated" => {
            parts.push(format!(
                "Recession risk is {}. Prioritise growth support.",
                recession_risk.to_lowercase()
            ));
        }
        "Moderate" => {
            parts.push("Recession risk is moderate. Monitor closely.".into());
        }
        _ => {}
    }

    parts.join(" ")
}

// ---------------------------------------------------------------------------
// Internal: validation
// ---------------------------------------------------------------------------

fn validate_input(input: &MonetaryPolicyInput) -> CorpFinanceResult<Vec<String>> {
    let mut warnings = Vec::new();

    // Hard errors
    if input.current_unemployment < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "current_unemployment".into(),
            reason: "Unemployment rate cannot be negative.".into(),
        });
    }
    if input.natural_unemployment < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "natural_unemployment".into(),
            reason: "Natural unemployment rate cannot be negative.".into(),
        });
    }
    if input.current_unemployment > dec!(0.5) {
        return Err(CorpFinanceError::InvalidInput {
            field: "current_unemployment".into(),
            reason: "Unemployment rate exceeds 50%, which is unrealistic.".into(),
        });
    }
    if input.inflation_weight < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "inflation_weight".into(),
            reason: "Taylor rule weight on inflation must be non-negative.".into(),
        });
    }
    if input.output_weight < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "output_weight".into(),
            reason: "Taylor rule weight on output must be non-negative.".into(),
        });
    }

    // Soft warnings
    if input.current_inflation < dec!(-0.05) || input.current_inflation > dec!(0.30) {
        warnings.push(format!(
            "Inflation rate {} is outside typical range [-5%, 30%].",
            input.current_inflation
        ));
    }
    if input.target_inflation < dec!(0.01) || input.target_inflation > dec!(0.04) {
        warnings.push(format!(
            "Target inflation {} is outside typical range [1%, 4%].",
            input.target_inflation
        ));
    }

    Ok(warnings)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    // -- Test helpers --------------------------------------------------------

    /// Standard US-like scenario: 2% target, 3.5% actual inflation,
    /// 3.8% unemployment (below 4.5% NAIRU).
    fn us_standard_input() -> MonetaryPolicyInput {
        MonetaryPolicyInput {
            current_inflation: dec!(0.035),
            target_inflation: dec!(0.02),
            current_gdp_growth: dec!(0.025),
            potential_gdp_growth: dec!(0.02),
            current_unemployment: dec!(0.038),
            natural_unemployment: dec!(0.045),
            current_policy_rate: dec!(0.0525),
            neutral_real_rate: dec!(0.005),
            inflation_weight: dec!(0.5),
            output_weight: dec!(0.5),
            historical_inflation: vec![
                dec!(0.025),
                dec!(0.028),
                dec!(0.030),
                dec!(0.032),
                dec!(0.035),
            ],
            historical_unemployment: vec![
                dec!(0.042),
                dec!(0.041),
                dec!(0.040),
                dec!(0.039),
                dec!(0.038),
            ],
        }
    }

    /// Recession scenario: negative output gap, rising unemployment.
    fn recession_input() -> MonetaryPolicyInput {
        MonetaryPolicyInput {
            current_inflation: dec!(0.015),
            target_inflation: dec!(0.02),
            current_gdp_growth: dec!(-0.01),
            potential_gdp_growth: dec!(0.02),
            current_unemployment: dec!(0.07),
            natural_unemployment: dec!(0.045),
            current_policy_rate: dec!(0.04),
            neutral_real_rate: dec!(0.005),
            inflation_weight: dec!(0.5),
            output_weight: dec!(0.5),
            historical_inflation: vec![
                dec!(0.030),
                dec!(0.025),
                dec!(0.020),
                dec!(0.018),
                dec!(0.015),
            ],
            historical_unemployment: vec![
                dec!(0.050),
                dec!(0.055),
                dec!(0.060),
                dec!(0.065),
                dec!(0.070),
            ],
        }
    }

    // -- Taylor Rule tests ---------------------------------------------------

    #[test]
    fn test_taylor_neutral_rate() {
        let input = us_standard_input();
        let result = analyze_monetary_policy(&input).unwrap();
        // neutral = r* + pi* = 0.005 + 0.02 = 0.025
        assert_eq!(result.taylor_rule.neutral_rate, dec!(0.025));
    }

    #[test]
    fn test_taylor_inflation_gap() {
        let input = us_standard_input();
        let result = analyze_monetary_policy(&input).unwrap();
        assert_eq!(result.taylor_rule.inflation_gap, dec!(0.015));
    }

    #[test]
    fn test_taylor_output_gap() {
        let input = us_standard_input();
        let result = analyze_monetary_policy(&input).unwrap();
        assert_eq!(result.taylor_rule.output_gap, dec!(0.005));
    }

    #[test]
    fn test_taylor_inflation_component() {
        let input = us_standard_input();
        let result = analyze_monetary_policy(&input).unwrap();
        // 0.5 * 0.015 = 0.0075
        assert_eq!(result.taylor_rule.inflation_component, dec!(0.0075));
    }

    #[test]
    fn test_taylor_output_component() {
        let input = us_standard_input();
        let result = analyze_monetary_policy(&input).unwrap();
        // 0.5 * 0.005 = 0.0025
        assert_eq!(result.taylor_rule.output_component, dec!(0.0025));
    }

    #[test]
    fn test_taylor_prescribed_rate_standard() {
        let input = us_standard_input();
        let result = analyze_monetary_policy(&input).unwrap();
        // prescribed = 0.025 + 0.0075 + 0.0025 = 0.035
        assert_eq!(result.taylor_rule.prescribed_rate, dec!(0.035));
    }

    #[test]
    fn test_taylor_rate_deviation() {
        let input = us_standard_input();
        let result = analyze_monetary_policy(&input).unwrap();
        // deviation = 0.0525 - 0.035 = 0.0175
        assert_eq!(result.taylor_rule.rate_deviation, dec!(0.0175));
    }

    #[test]
    fn test_taylor_zero_output_gap() {
        let mut input = us_standard_input();
        input.current_gdp_growth = input.potential_gdp_growth;
        let result = analyze_monetary_policy(&input).unwrap();
        assert_eq!(result.taylor_rule.output_gap, Decimal::ZERO);
        assert_eq!(result.taylor_rule.output_component, Decimal::ZERO);
    }

    #[test]
    fn test_taylor_large_inflation_gap() {
        let mut input = us_standard_input();
        input.current_inflation = dec!(0.10);
        let result = analyze_monetary_policy(&input).unwrap();
        assert_eq!(result.taylor_rule.inflation_gap, dec!(0.08));
        assert_eq!(result.taylor_rule.inflation_component, dec!(0.04));
    }

    #[test]
    fn test_taylor_negative_output_gap_recession() {
        let input = recession_input();
        let result = analyze_monetary_policy(&input).unwrap();
        // output gap = -0.01 - 0.02 = -0.03
        assert_eq!(result.taylor_rule.output_gap, dec!(-0.03));
    }

    #[test]
    fn test_taylor_equal_weights() {
        let input = us_standard_input();
        let result = analyze_monetary_policy(&input).unwrap();
        assert_eq!(
            result.taylor_rule.inflation_component,
            dec!(0.5) * result.taylor_rule.inflation_gap
        );
        assert_eq!(
            result.taylor_rule.output_component,
            dec!(0.5) * result.taylor_rule.output_gap
        );
    }

    #[test]
    fn test_taylor_asymmetric_weights_inflation_hawk() {
        let mut input = us_standard_input();
        input.inflation_weight = dec!(1.5);
        input.output_weight = dec!(0.5);
        let result = analyze_monetary_policy(&input).unwrap();
        // inflation component = 1.5 * 0.015 = 0.0225
        assert_eq!(result.taylor_rule.inflation_component, dec!(0.0225));
        // output component = 0.5 * 0.005 = 0.0025
        assert_eq!(result.taylor_rule.output_component, dec!(0.0025));
        // prescribed = 0.025 + 0.0225 + 0.0025 = 0.05
        assert_eq!(result.taylor_rule.prescribed_rate, dec!(0.05));
    }

    // -- Policy stance tests -------------------------------------------------

    #[test]
    fn test_stance_restrictive() {
        let input = us_standard_input();
        let result = analyze_monetary_policy(&input).unwrap();
        assert_eq!(result.taylor_rule.policy_stance, "Restrictive");
    }

    #[test]
    fn test_stance_accommodative() {
        let mut input = us_standard_input();
        input.current_policy_rate = dec!(0.01);
        let result = analyze_monetary_policy(&input).unwrap();
        assert_eq!(result.taylor_rule.policy_stance, "Accommodative");
    }

    #[test]
    fn test_stance_neutral() {
        let mut input = us_standard_input();
        // prescribed = 0.035
        input.current_policy_rate = dec!(0.035);
        let result = analyze_monetary_policy(&input).unwrap();
        assert_eq!(result.taylor_rule.policy_stance, "Neutral");
    }

    #[test]
    fn test_stance_neutral_borderline() {
        let mut input = us_standard_input();
        // prescribed = 0.035; 0.035 + 0.004 = 0.039 (within 0.005 threshold)
        input.current_policy_rate = dec!(0.039);
        let result = analyze_monetary_policy(&input).unwrap();
        assert_eq!(result.taylor_rule.policy_stance, "Neutral");
    }

    // -- Phillips Curve tests ------------------------------------------------

    #[test]
    fn test_phillips_unemployment_gap() {
        let input = us_standard_input();
        let result = analyze_monetary_policy(&input).unwrap();
        // gap = 0.038 - 0.045 = -0.007
        assert_eq!(result.phillips_curve.unemployment_gap, dec!(-0.007));
    }

    #[test]
    fn test_phillips_with_historical_data() {
        let input = us_standard_input();
        let result = analyze_monetary_policy(&input).unwrap();
        // With 5 data points, should estimate coefficient from data
        assert!(result.phillips_curve.phillips_coefficient < Decimal::ZERO);
    }

    #[test]
    fn test_phillips_without_data_uses_default() {
        let mut input = us_standard_input();
        input.historical_inflation = vec![];
        input.historical_unemployment = vec![];
        let result = analyze_monetary_policy(&input).unwrap();
        assert_eq!(result.phillips_curve.phillips_coefficient, dec!(-0.5));
    }

    #[test]
    fn test_phillips_insufficient_data_uses_default() {
        let mut input = us_standard_input();
        input.historical_inflation = vec![dec!(0.02), dec!(0.03)];
        input.historical_unemployment = vec![dec!(0.04), dec!(0.05)];
        let result = analyze_monetary_policy(&input).unwrap();
        assert_eq!(result.phillips_curve.phillips_coefficient, dec!(-0.5));
    }

    #[test]
    fn test_phillips_implied_change_tight_labour_default() {
        // Default coeff = -0.5, gap = 0.038 - 0.045 = -0.007
        // implied = -(-0.5) * (-0.007) = 0.5 * (-0.007) = -0.0035
        let mut input = us_standard_input();
        input.historical_inflation = vec![];
        input.historical_unemployment = vec![];
        let result = analyze_monetary_policy(&input).unwrap();
        assert_eq!(
            result.phillips_curve.implied_inflation_change,
            dec!(-0.0035)
        );
    }

    #[test]
    fn test_phillips_implied_change_slack_labour() {
        // Recession: u=0.07, u*=0.045, gap=0.025
        // implied = -(-0.5) * 0.025 = 0.5 * 0.025 = 0.0125
        let mut input = recession_input();
        input.historical_inflation = vec![];
        input.historical_unemployment = vec![];
        let result = analyze_monetary_policy(&input).unwrap();
        assert_eq!(result.phillips_curve.unemployment_gap, dec!(0.025));
        assert_eq!(result.phillips_curve.implied_inflation_change, dec!(0.0125));
    }

    #[test]
    fn test_phillips_sacrifice_ratio_default() {
        let mut input = us_standard_input();
        input.historical_inflation = vec![];
        input.historical_unemployment = vec![];
        let result = analyze_monetary_policy(&input).unwrap();
        // sacrifice_ratio = 1 / |coeff| = 1 / 0.5 = 2
        assert_eq!(result.phillips_curve.sacrifice_ratio, dec!(2));
    }

    #[test]
    fn test_phillips_nairu_matches_input() {
        let input = us_standard_input();
        let result = analyze_monetary_policy(&input).unwrap();
        assert_eq!(
            result.phillips_curve.nairu_estimate,
            input.natural_unemployment
        );
    }

    // -- Okun's Law tests ----------------------------------------------------

    #[test]
    fn test_okun_unemployment_gap() {
        let input = us_standard_input();
        let result = analyze_monetary_policy(&input).unwrap();
        assert_eq!(result.okun_law.unemployment_gap, dec!(-0.007));
    }

    #[test]
    fn test_okun_coefficient() {
        let input = us_standard_input();
        let result = analyze_monetary_policy(&input).unwrap();
        assert_eq!(result.okun_law.okun_coefficient, dec!(-2));
    }

    #[test]
    fn test_okun_output_gap_tight_market() {
        let input = us_standard_input();
        let result = analyze_monetary_policy(&input).unwrap();
        // output_gap = -(-2) * (-0.007) = 2 * (-0.007) = -0.014
        assert_eq!(result.okun_law.output_gap_pct, dec!(-0.014));
    }

    #[test]
    fn test_okun_output_gap_high_unemployment() {
        let input = recession_input();
        let result = analyze_monetary_policy(&input).unwrap();
        // gap = 0.07 - 0.045 = 0.025
        // output_gap = -(-2) * 0.025 = 2 * 0.025 = 0.05
        assert_eq!(result.okun_law.output_gap_pct, dec!(0.050));
    }

    #[test]
    fn test_okun_zero_unemployment_gap() {
        let mut input = us_standard_input();
        input.current_unemployment = input.natural_unemployment;
        let result = analyze_monetary_policy(&input).unwrap();
        assert_eq!(result.okun_law.unemployment_gap, Decimal::ZERO);
        assert_eq!(result.okun_law.output_gap_pct, Decimal::ZERO);
        assert_eq!(result.okun_law.implied_gdp_loss, Decimal::ZERO);
    }

    #[test]
    fn test_okun_implied_gdp_loss() {
        let input = us_standard_input();
        let result = analyze_monetary_policy(&input).unwrap();
        // loss = (-2) * (-0.007) * 0.02 = 0.00028
        let expected = dec!(-2) * dec!(-0.007) * dec!(0.02);
        assert_eq!(result.okun_law.implied_gdp_loss, expected);
    }

    // -- Recession risk tests ------------------------------------------------

    #[test]
    fn test_recession_risk_low() {
        let mut input = us_standard_input();
        input.current_policy_rate = dec!(0.025); // at neutral
        input.current_unemployment = dec!(0.045); // exactly at NAIRU, no signals
        let result = analyze_monetary_policy(&input).unwrap();
        assert_eq!(result.recession_risk, "Low");
    }

    #[test]
    fn test_recession_risk_elevated() {
        let mut input = recession_input();
        input.current_policy_rate = dec!(0.06);
        let result = analyze_monetary_policy(&input).unwrap();
        assert!(
            result.recession_risk == "Elevated" || result.recession_risk == "High",
            "Expected Elevated or High, got {}",
            result.recession_risk
        );
    }

    #[test]
    fn test_recession_risk_moderate() {
        // Exactly one signal: policy rate above neutral by >100bps,
        // but unemployment at NAIRU and no negative output gap.
        let mut input = us_standard_input();
        input.current_policy_rate = dec!(0.05); // > neutral 0.025 + 0.01
        input.current_unemployment = dec!(0.045); // at NAIRU, no signal 2 or 3
        let result = analyze_monetary_policy(&input).unwrap();
        assert_eq!(result.recession_risk, "Moderate");
    }

    // -- Inflation trend tests -----------------------------------------------

    #[test]
    fn test_inflation_trend_rising() {
        let input = us_standard_input();
        let result = analyze_monetary_policy(&input).unwrap();
        assert_eq!(result.inflation_trend, "Rising");
    }

    #[test]
    fn test_inflation_trend_falling() {
        let input = recession_input();
        let result = analyze_monetary_policy(&input).unwrap();
        assert_eq!(result.inflation_trend, "Falling");
    }

    #[test]
    fn test_inflation_trend_stable() {
        let mut input = us_standard_input();
        input.historical_inflation = vec![dec!(0.02), dec!(0.02), dec!(0.02), dec!(0.02)];
        let result = analyze_monetary_policy(&input).unwrap();
        assert_eq!(result.inflation_trend, "Stable");
    }

    #[test]
    fn test_inflation_trend_insufficient_data() {
        let mut input = us_standard_input();
        input.historical_inflation = vec![dec!(0.02), dec!(0.03)];
        let result = analyze_monetary_policy(&input).unwrap();
        assert_eq!(result.inflation_trend, "Stable");
    }

    // -- Edge case tests -----------------------------------------------------

    #[test]
    fn test_deflation_scenario() {
        let mut input = us_standard_input();
        input.current_inflation = dec!(-0.01);
        let result = analyze_monetary_policy(&input).unwrap();
        assert_eq!(result.taylor_rule.inflation_gap, dec!(-0.03));
        assert!(result.taylor_rule.prescribed_rate < result.taylor_rule.neutral_rate);
    }

    #[test]
    fn test_hyperinflation_proxy() {
        let mut input = us_standard_input();
        input.current_inflation = dec!(0.20);
        let result = analyze_monetary_policy(&input).unwrap();
        assert_eq!(result.taylor_rule.inflation_gap, dec!(0.18));
    }

    #[test]
    fn test_rate_deviation_positive_means_tight() {
        let input = us_standard_input();
        let result = analyze_monetary_policy(&input).unwrap();
        assert!(result.taylor_rule.rate_deviation > Decimal::ZERO);
        assert_eq!(result.taylor_rule.policy_stance, "Restrictive");
    }

    #[test]
    fn test_rate_deviation_negative_means_loose() {
        let mut input = us_standard_input();
        input.current_policy_rate = dec!(0.01);
        let result = analyze_monetary_policy(&input).unwrap();
        assert!(result.taylor_rule.rate_deviation < Decimal::ZERO);
        assert_eq!(result.taylor_rule.policy_stance, "Accommodative");
    }

    // -- Validation tests ----------------------------------------------------

    #[test]
    fn test_validation_negative_unemployment() {
        let mut input = us_standard_input();
        input.current_unemployment = dec!(-0.01);
        assert!(analyze_monetary_policy(&input).is_err());
    }

    #[test]
    fn test_validation_negative_natural_unemployment() {
        let mut input = us_standard_input();
        input.natural_unemployment = dec!(-0.01);
        assert!(analyze_monetary_policy(&input).is_err());
    }

    #[test]
    fn test_validation_unemployment_over_50pct() {
        let mut input = us_standard_input();
        input.current_unemployment = dec!(0.55);
        assert!(analyze_monetary_policy(&input).is_err());
    }

    #[test]
    fn test_validation_negative_inflation_weight() {
        let mut input = us_standard_input();
        input.inflation_weight = dec!(-0.1);
        assert!(analyze_monetary_policy(&input).is_err());
    }

    #[test]
    fn test_validation_negative_output_weight() {
        let mut input = us_standard_input();
        input.output_weight = dec!(-0.1);
        assert!(analyze_monetary_policy(&input).is_err());
    }

    #[test]
    fn test_warning_unusual_target_inflation() {
        let mut input = us_standard_input();
        input.target_inflation = dec!(0.10);
        let result = analyze_monetary_policy(&input).unwrap();
        assert!(result
            .warnings
            .iter()
            .any(|w| w.contains("Target inflation")));
    }

    #[test]
    fn test_warning_extreme_inflation() {
        let mut input = us_standard_input();
        input.current_inflation = dec!(0.35);
        let result = analyze_monetary_policy(&input).unwrap();
        assert!(result.warnings.iter().any(|w| w.contains("Inflation rate")));
    }

    // -- Output structure tests ----------------------------------------------

    #[test]
    fn test_methodology_not_empty() {
        let input = us_standard_input();
        let result = analyze_monetary_policy(&input).unwrap();
        assert!(!result.methodology.is_empty());
    }

    #[test]
    fn test_assumptions_contain_key_fields() {
        let input = us_standard_input();
        let result = analyze_monetary_policy(&input).unwrap();
        assert!(result.assumptions.contains_key("taylor_rule"));
        assert!(result.assumptions.contains_key("okun_coefficient"));
        assert!(result.assumptions.contains_key("nairu"));
        assert!(result.assumptions.contains_key("r_star"));
    }

    #[test]
    fn test_policy_recommendation_not_empty() {
        let input = us_standard_input();
        let result = analyze_monetary_policy(&input).unwrap();
        assert!(!result.policy_recommendation.is_empty());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let input = us_standard_input();
        let result = analyze_monetary_policy(&input).unwrap();
        let json = serde_json::to_string(&result).unwrap();
        let _deserialized: MonetaryPolicyOutput = serde_json::from_str(&json).unwrap();
    }

    // -- Phillips coefficient regression tests --------------------------------

    #[test]
    fn test_phillips_coefficient_regression_negative_slope() {
        let inflation = vec![dec!(0.05), dec!(0.04), dec!(0.03), dec!(0.02), dec!(0.01)];
        let unemployment = vec![dec!(0.03), dec!(0.04), dec!(0.05), dec!(0.06), dec!(0.07)];
        let coeff = estimate_phillips_coefficient(&inflation, &unemployment);
        assert_eq!(coeff, dec!(-1));
    }

    #[test]
    fn test_phillips_coefficient_perfect_inverse() {
        let unemployment = vec![dec!(0.02), dec!(0.04), dec!(0.06), dec!(0.08)];
        let inflation = vec![dec!(0.08), dec!(0.06), dec!(0.04), dec!(0.02)];
        let coeff = estimate_phillips_coefficient(&inflation, &unemployment);
        assert_eq!(coeff, dec!(-1));
    }

    // -- Comprehensive scenario test ------------------------------------------

    #[test]
    fn test_full_recession_scenario() {
        let input = recession_input();
        let result = analyze_monetary_policy(&input).unwrap();

        // prescribed = 0.025 + 0.5*(-0.005) + 0.5*(-0.03) = 0.025 - 0.0025 - 0.015 = 0.0075
        assert_eq!(result.taylor_rule.prescribed_rate, dec!(0.0075));

        // Current rate 4% is way above prescribed 0.75%
        assert!(result.taylor_rule.rate_deviation > Decimal::ZERO);
        assert_eq!(result.taylor_rule.policy_stance, "Restrictive");

        // Okun: unemployment gap = 0.07 - 0.045 = 0.025
        assert_eq!(result.okun_law.unemployment_gap, dec!(0.025));

        assert_eq!(result.inflation_trend, "Falling");

        assert!(
            result.recession_risk == "High" || result.recession_risk == "Elevated",
            "Expected High or Elevated, got {}",
            result.recession_risk
        );
    }
}
