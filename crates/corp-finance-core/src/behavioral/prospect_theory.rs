use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::{CorpFinanceError, CorpFinanceResult};

// ---------------------------------------------------------------------------
// Helper math functions (Decimal-only, no f64)
// ---------------------------------------------------------------------------

/// Taylor series exp with range reduction: exp(x) = exp(x/2)^2 for |x|>2
fn exp_decimal(x: Decimal) -> Decimal {
    let two = dec!(2);
    if x > two || x < -two {
        let half = exp_decimal(x / two);
        return half * half;
    }
    let mut sum = Decimal::ONE;
    let mut term = Decimal::ONE;
    for n in 1u32..=30 {
        term = term * x / Decimal::from(n);
        sum += term;
    }
    sum
}

/// Newton's method ln: find y such that exp(y) = x, 30 iterations
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

    for _ in 0..30 {
        let ey = exp_decimal(y);
        if ey == Decimal::ZERO {
            break;
        }
        y = y - Decimal::ONE + x / ey;
    }
    y
}

/// Decimal power: x^a = exp(a * ln(x)), x must be positive
fn pow_decimal(base: Decimal, exponent: Decimal) -> Decimal {
    if base <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    if exponent == Decimal::ZERO {
        return Decimal::ONE;
    }
    if exponent == Decimal::ONE {
        return base;
    }
    exp_decimal(exponent * ln_decimal(base))
}

// ---------------------------------------------------------------------------
// Input types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Outcome {
    pub description: String,
    pub value: Decimal,
    pub probability: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProspectTheoryInput {
    pub outcomes: Vec<Outcome>,
    pub reference_point: Decimal,
    pub current_value: Decimal,
    pub loss_aversion_lambda: Decimal,
    pub alpha: Decimal,
    pub beta_param: Decimal,
    pub gamma: Decimal,
    pub delta_param: Decimal,
    pub holding_period_months: u32,
    pub annual_return_history: Vec<Decimal>,
}

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutcomeAnalysis {
    pub description: String,
    pub value: Decimal,
    pub probability: Decimal,
    pub gain_or_loss: Decimal,
    pub value_function: Decimal,
    pub decision_weight: Decimal,
    pub weighted_value: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbabilityWeight {
    pub actual: Decimal,
    pub decision_weight: Decimal,
    pub distortion: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MentalAccounting {
    pub strong_gain_zone: Decimal,
    pub weak_gain_zone: Decimal,
    pub weak_loss_zone: Decimal,
    pub strong_loss_zone: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProspectTheoryOutput {
    pub prospect_value: Decimal,
    pub expected_value: Decimal,
    pub certainty_equivalent: Decimal,
    pub risk_premium: Decimal,
    pub gain_loss_ratio: Decimal,
    pub outcome_analysis: Vec<OutcomeAnalysis>,
    pub disposition_effect_score: Decimal,
    pub framing_bias_score: Decimal,
    pub loss_aversion_impact: Decimal,
    pub probability_distortion: Vec<ProbabilityWeight>,
    pub behavioral_recommendation: String,
    pub mental_accounting_zones: MentalAccounting,
}

// ---------------------------------------------------------------------------
// Core implementation
// ---------------------------------------------------------------------------

/// Kahneman-Tversky value function: v(x) = x^alpha for gains,
/// v(x) = -lambda * |x|^beta for losses (relative to reference_point)
fn value_function(
    gain_or_loss: Decimal,
    alpha: Decimal,
    beta_param: Decimal,
    lambda: Decimal,
) -> Decimal {
    if gain_or_loss >= Decimal::ZERO {
        pow_decimal(gain_or_loss, alpha)
    } else {
        let abs_loss = gain_or_loss.abs();
        -lambda * pow_decimal(abs_loss, beta_param)
    }
}

/// Probability weighting for gains: w+(p) = p^gamma / (p^gamma + (1-p)^gamma)^(1/gamma)
fn weight_gain(p: Decimal, gamma: Decimal) -> Decimal {
    if p <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    if p >= Decimal::ONE {
        return Decimal::ONE;
    }
    let p_g = pow_decimal(p, gamma);
    let one_minus_p_g = pow_decimal(Decimal::ONE - p, gamma);
    let denom_base = p_g + one_minus_p_g;
    if denom_base <= Decimal::ZERO {
        return p;
    }
    let denom = pow_decimal(denom_base, Decimal::ONE / gamma);
    if denom == Decimal::ZERO {
        return p;
    }
    p_g / denom
}

/// Probability weighting for losses: w-(p) = p^delta / (p^delta + (1-p)^delta)^(1/delta)
fn weight_loss(p: Decimal, delta: Decimal) -> Decimal {
    if p <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    if p >= Decimal::ONE {
        return Decimal::ONE;
    }
    let p_d = pow_decimal(p, delta);
    let one_minus_p_d = pow_decimal(Decimal::ONE - p, delta);
    let denom_base = p_d + one_minus_p_d;
    if denom_base <= Decimal::ZERO {
        return p;
    }
    let denom = pow_decimal(denom_base, Decimal::ONE / delta);
    if denom == Decimal::ZERO {
        return p;
    }
    p_d / denom
}

/// Inverse value function: given v, find x such that v(x) = v
/// For gains: x = v^(1/alpha)
/// For losses: x = -(|v|/lambda)^(1/beta)
fn inverse_value_function(
    v: Decimal,
    alpha: Decimal,
    beta_param: Decimal,
    lambda: Decimal,
) -> Decimal {
    if v >= Decimal::ZERO {
        pow_decimal(v, Decimal::ONE / alpha)
    } else {
        let abs_v = v.abs();
        if lambda == Decimal::ZERO {
            return Decimal::ZERO;
        }
        let inner = abs_v / lambda;
        -pow_decimal(inner, Decimal::ONE / beta_param)
    }
}

/// Compute certainty equivalent via bisection search
fn compute_certainty_equivalent(
    prospect_value: Decimal,
    alpha: Decimal,
    beta_param: Decimal,
    lambda: Decimal,
) -> Decimal {
    // Use inverse value function for the direct analytic solution
    inverse_value_function(prospect_value, alpha, beta_param, lambda)
}

/// Compute disposition effect score from return history and current position
fn compute_disposition_score(
    current_value: Decimal,
    reference_point: Decimal,
    annual_return_history: &[Decimal],
) -> Decimal {
    if annual_return_history.is_empty() {
        return dec!(50);
    }

    let in_gain = current_value > reference_point;
    let mut disposition_signals = Decimal::ZERO;
    let count = Decimal::from(annual_return_history.len() as u32);

    for ret in annual_return_history {
        if in_gain && *ret > Decimal::ZERO {
            // In gain territory with positive returns: tendency to sell winners
            disposition_signals += Decimal::ONE;
        } else if !in_gain && *ret < Decimal::ZERO {
            // In loss territory with negative returns: tendency to hold losers
            disposition_signals += Decimal::ONE;
        }
    }

    // Score 0-100: higher means stronger disposition effect
    let ratio = disposition_signals / count;
    let score = ratio * dec!(100);
    if score > dec!(100) {
        dec!(100)
    } else if score < Decimal::ZERO {
        Decimal::ZERO
    } else {
        score
    }
}

/// Compute framing bias score by comparing prospect values at shifted reference points
fn compute_framing_bias(
    outcomes: &[Outcome],
    reference_point: Decimal,
    alpha: Decimal,
    beta_param: Decimal,
    gamma: Decimal,
    delta_param: Decimal,
    lambda: Decimal,
) -> Decimal {
    // Test sensitivity to +/-10% shift in reference point
    let shift = reference_point * dec!(0.10);
    if shift == Decimal::ZERO {
        return dec!(50);
    }

    let pv_base = compute_prospect_value(
        outcomes,
        reference_point,
        alpha,
        beta_param,
        gamma,
        delta_param,
        lambda,
    );
    let pv_up = compute_prospect_value(
        outcomes,
        reference_point + shift,
        alpha,
        beta_param,
        gamma,
        delta_param,
        lambda,
    );
    let pv_down = compute_prospect_value(
        outcomes,
        reference_point - shift,
        alpha,
        beta_param,
        gamma,
        delta_param,
        lambda,
    );

    // Higher sensitivity to framing = bigger difference
    let range = (pv_up - pv_down).abs();
    let base_abs = if pv_base.abs() > Decimal::ZERO {
        pv_base.abs()
    } else {
        Decimal::ONE
    };

    let sensitivity = range / base_abs * dec!(50);
    if sensitivity > dec!(100) {
        dec!(100)
    } else {
        sensitivity
    }
}

/// Compute raw prospect value for a set of outcomes
fn compute_prospect_value(
    outcomes: &[Outcome],
    reference_point: Decimal,
    alpha: Decimal,
    beta_param: Decimal,
    gamma: Decimal,
    delta_param: Decimal,
    lambda: Decimal,
) -> Decimal {
    let mut total = Decimal::ZERO;
    for o in outcomes {
        let gain_or_loss = o.value - reference_point;
        let vf = value_function(gain_or_loss, alpha, beta_param, lambda);
        let dw = if gain_or_loss >= Decimal::ZERO {
            weight_gain(o.probability, gamma)
        } else {
            weight_loss(o.probability, delta_param)
        };
        total += dw * vf;
    }
    total
}

/// Classify current position into mental accounting zones
fn compute_mental_accounting(current_value: Decimal, reference_point: Decimal) -> MentalAccounting {
    let diff = current_value - reference_point;
    let abs_ref = if reference_point.abs() > Decimal::ZERO {
        reference_point.abs()
    } else {
        Decimal::ONE
    };
    let pct_diff = diff / abs_ref * dec!(100);

    // Thresholds: strong >10%, weak 0-10%
    let strong_gain = if pct_diff > dec!(10) {
        pct_diff
    } else {
        Decimal::ZERO
    };
    let weak_gain = if pct_diff > Decimal::ZERO && pct_diff <= dec!(10) {
        pct_diff
    } else {
        Decimal::ZERO
    };
    let weak_loss = if pct_diff < Decimal::ZERO && pct_diff >= dec!(-10) {
        pct_diff.abs()
    } else {
        Decimal::ZERO
    };
    let strong_loss = if pct_diff < dec!(-10) {
        pct_diff.abs()
    } else {
        Decimal::ZERO
    };

    MentalAccounting {
        strong_gain_zone: strong_gain,
        weak_gain_zone: weak_gain,
        weak_loss_zone: weak_loss,
        strong_loss_zone: strong_loss,
    }
}

/// Generate behavioral recommendation based on analysis
fn generate_recommendation(
    disposition_score: Decimal,
    framing_score: Decimal,
    loss_aversion_impact: Decimal,
    in_gain_territory: bool,
    prospect_value: Decimal,
    expected_value: Decimal,
) -> String {
    let mut advice = Vec::new();

    if disposition_score > dec!(70) {
        if in_gain_territory {
            advice.push(
                "High disposition effect detected: you may be inclined to sell winners too early. \
                 Consider holding for long-term gains."
                    .to_string(),
            );
        } else {
            advice.push(
                "High disposition effect detected: you may be holding losers too long. \
                 Consider reviewing your exit criteria objectively."
                    .to_string(),
            );
        }
    }

    if framing_score > dec!(60) {
        advice.push(
            "Significant framing bias: your valuation is sensitive to the reference point chosen. \
             Try evaluating the investment on its own merits rather than relative to your purchase price."
                .to_string(),
        );
    }

    if loss_aversion_impact.abs() > dec!(20) {
        advice.push(
            "Strong loss aversion impact: losses are weighted much more heavily than equivalent \
             gains. Consider whether your risk assessment is proportionate."
                .to_string(),
        );
    }

    if prospect_value < Decimal::ZERO && expected_value > Decimal::ZERO {
        advice.push(
            "Behavioral bias is making a positive-EV investment appear negative. \
             The investment has positive expected value but feels negative due to loss aversion."
                .to_string(),
        );
    }

    if advice.is_empty() {
        "Biases are within normal range. Decision-making appears relatively rational for this \
         investment."
            .to_string()
    } else {
        advice.join(" ")
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub fn analyze_prospect_theory(
    input: &ProspectTheoryInput,
) -> CorpFinanceResult<ProspectTheoryOutput> {
    // Validation
    if input.outcomes.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "outcomes".to_string(),
            reason: "At least one outcome is required".to_string(),
        });
    }

    let prob_sum: Decimal = input.outcomes.iter().map(|o| o.probability).sum();
    let prob_diff = (prob_sum - Decimal::ONE).abs();
    if prob_diff > dec!(0.01) {
        return Err(CorpFinanceError::InvalidInput {
            field: "outcomes.probability".to_string(),
            reason: format!("Probabilities must sum to 1.0 (got {})", prob_sum),
        });
    }

    for o in &input.outcomes {
        if o.probability < Decimal::ZERO || o.probability > Decimal::ONE {
            return Err(CorpFinanceError::InvalidInput {
                field: "outcomes.probability".to_string(),
                reason: format!("Probability must be between 0 and 1, got {}", o.probability),
            });
        }
    }

    if input.alpha <= Decimal::ZERO || input.alpha > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "alpha".to_string(),
            reason: "Alpha must be in (0, 1]".to_string(),
        });
    }

    if input.beta_param <= Decimal::ZERO || input.beta_param > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "beta_param".to_string(),
            reason: "Beta must be in (0, 1]".to_string(),
        });
    }

    if input.loss_aversion_lambda <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "loss_aversion_lambda".to_string(),
            reason: "Loss aversion lambda must be positive".to_string(),
        });
    }

    // Compute per-outcome analysis
    let mut outcome_analysis = Vec::new();
    let mut expected_value = Decimal::ZERO;
    let mut weighted_gains = Decimal::ZERO;
    let mut weighted_losses = Decimal::ZERO;
    let mut probability_distortion = Vec::new();

    for o in &input.outcomes {
        let gain_or_loss = o.value - input.reference_point;
        let vf = value_function(
            gain_or_loss,
            input.alpha,
            input.beta_param,
            input.loss_aversion_lambda,
        );

        let dw = if gain_or_loss >= Decimal::ZERO {
            weight_gain(o.probability, input.gamma)
        } else {
            weight_loss(o.probability, input.delta_param)
        };

        let weighted_value = dw * vf;
        expected_value += o.probability * o.value;

        if gain_or_loss >= Decimal::ZERO {
            weighted_gains += weighted_value;
        } else {
            weighted_losses += weighted_value.abs();
        }

        outcome_analysis.push(OutcomeAnalysis {
            description: o.description.clone(),
            value: o.value,
            probability: o.probability,
            gain_or_loss,
            value_function: vf,
            decision_weight: dw,
            weighted_value,
        });

        probability_distortion.push(ProbabilityWeight {
            actual: o.probability,
            decision_weight: dw,
            distortion: dw - o.probability,
        });
    }

    let prospect_value: Decimal = outcome_analysis.iter().map(|oa| oa.weighted_value).sum();

    // Certainty equivalent
    let certainty_equivalent = compute_certainty_equivalent(
        prospect_value,
        input.alpha,
        input.beta_param,
        input.loss_aversion_lambda,
    );

    let risk_premium = expected_value - certainty_equivalent;

    // Gain/loss ratio
    let gain_loss_ratio = if weighted_losses > Decimal::ZERO {
        weighted_gains / weighted_losses
    } else if weighted_gains > Decimal::ZERO {
        dec!(999)
    } else {
        Decimal::ONE
    };

    // Disposition effect
    let disposition_effect_score = compute_disposition_score(
        input.current_value,
        input.reference_point,
        &input.annual_return_history,
    );

    // Framing bias
    let framing_bias_score = compute_framing_bias(
        &input.outcomes,
        input.reference_point,
        input.alpha,
        input.beta_param,
        input.gamma,
        input.delta_param,
        input.loss_aversion_lambda,
    );

    // Loss aversion impact: how much the PV differs from a lambda=1 scenario
    let pv_neutral = compute_prospect_value(
        &input.outcomes,
        input.reference_point,
        input.alpha,
        input.beta_param,
        input.gamma,
        input.delta_param,
        Decimal::ONE,
    );
    let loss_aversion_impact = if pv_neutral.abs() > Decimal::ZERO {
        (prospect_value - pv_neutral) / pv_neutral.abs() * dec!(100)
    } else {
        Decimal::ZERO
    };

    let in_gain = input.current_value > input.reference_point;

    let mental_accounting_zones =
        compute_mental_accounting(input.current_value, input.reference_point);

    let behavioral_recommendation = generate_recommendation(
        disposition_effect_score,
        framing_bias_score,
        loss_aversion_impact,
        in_gain,
        prospect_value,
        expected_value,
    );

    Ok(ProspectTheoryOutput {
        prospect_value,
        expected_value,
        certainty_equivalent,
        risk_premium,
        gain_loss_ratio,
        outcome_analysis,
        disposition_effect_score,
        framing_bias_score,
        loss_aversion_impact,
        probability_distortion,
        behavioral_recommendation,
        mental_accounting_zones,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn default_input() -> ProspectTheoryInput {
        ProspectTheoryInput {
            outcomes: vec![
                Outcome {
                    description: "Bull case".to_string(),
                    value: dec!(120),
                    probability: dec!(0.4),
                },
                Outcome {
                    description: "Base case".to_string(),
                    value: dec!(100),
                    probability: dec!(0.35),
                },
                Outcome {
                    description: "Bear case".to_string(),
                    value: dec!(80),
                    probability: dec!(0.25),
                },
            ],
            reference_point: dec!(100),
            current_value: dec!(105),
            loss_aversion_lambda: dec!(2.25),
            alpha: dec!(0.88),
            beta_param: dec!(0.88),
            gamma: dec!(0.61),
            delta_param: dec!(0.69),
            holding_period_months: 12,
            annual_return_history: vec![
                dec!(0.08),
                dec!(0.12),
                dec!(-0.05),
                dec!(0.15),
                dec!(0.03),
            ],
        }
    }

    #[test]
    fn test_basic_prospect_theory() {
        let input = default_input();
        let result = analyze_prospect_theory(&input).unwrap();

        // EV should be 0.4*120 + 0.35*100 + 0.25*80 = 48+35+20 = 103
        assert_eq!(result.expected_value, dec!(103));
        assert!(result.outcome_analysis.len() == 3);
    }

    #[test]
    fn test_prospect_value_is_computed() {
        let input = default_input();
        let result = analyze_prospect_theory(&input).unwrap();

        // Prospect value should be negative relative to EV due to loss aversion
        // (losses weighted more heavily)
        assert!(result.prospect_value != Decimal::ZERO);
    }

    #[test]
    fn test_certainty_equivalent() {
        let input = default_input();
        let result = analyze_prospect_theory(&input).unwrap();

        // CE should differ from EV
        assert!(result.certainty_equivalent != result.expected_value);
    }

    #[test]
    fn test_risk_premium_positive_for_mixed_outcomes() {
        let input = default_input();
        let result = analyze_prospect_theory(&input).unwrap();

        // With loss aversion, risk premium should be positive (EV > CE)
        assert!(
            result.risk_premium > Decimal::ZERO,
            "Risk premium should be positive with loss aversion, got {}",
            result.risk_premium
        );
    }

    #[test]
    fn test_gain_loss_ratio() {
        let input = default_input();
        let result = analyze_prospect_theory(&input).unwrap();

        assert!(result.gain_loss_ratio > Decimal::ZERO);
    }

    #[test]
    fn test_all_gains_scenario() {
        let input = ProspectTheoryInput {
            outcomes: vec![
                Outcome {
                    description: "Good".to_string(),
                    value: dec!(120),
                    probability: dec!(0.6),
                },
                Outcome {
                    description: "Great".to_string(),
                    value: dec!(150),
                    probability: dec!(0.4),
                },
            ],
            reference_point: dec!(100),
            current_value: dec!(110),
            loss_aversion_lambda: dec!(2.25),
            alpha: dec!(0.88),
            beta_param: dec!(0.88),
            gamma: dec!(0.61),
            delta_param: dec!(0.69),
            holding_period_months: 6,
            annual_return_history: vec![dec!(0.10), dec!(0.15)],
        };
        let result = analyze_prospect_theory(&input).unwrap();

        // All gains scenario: prospect value should be positive
        assert!(result.prospect_value > Decimal::ZERO);
        // No losses, so gain_loss_ratio should be high
        assert!(result.gain_loss_ratio > dec!(100));
    }

    #[test]
    fn test_all_losses_scenario() {
        let input = ProspectTheoryInput {
            outcomes: vec![
                Outcome {
                    description: "Bad".to_string(),
                    value: dec!(80),
                    probability: dec!(0.5),
                },
                Outcome {
                    description: "Worse".to_string(),
                    value: dec!(60),
                    probability: dec!(0.5),
                },
            ],
            reference_point: dec!(100),
            current_value: dec!(85),
            loss_aversion_lambda: dec!(2.25),
            alpha: dec!(0.88),
            beta_param: dec!(0.88),
            gamma: dec!(0.61),
            delta_param: dec!(0.69),
            holding_period_months: 12,
            annual_return_history: vec![dec!(-0.10), dec!(-0.05)],
        };
        let result = analyze_prospect_theory(&input).unwrap();

        // All losses: prospect value should be negative
        assert!(result.prospect_value < Decimal::ZERO);
    }

    #[test]
    fn test_symmetric_outcomes() {
        let input = ProspectTheoryInput {
            outcomes: vec![
                Outcome {
                    description: "Up".to_string(),
                    value: dec!(120),
                    probability: dec!(0.5),
                },
                Outcome {
                    description: "Down".to_string(),
                    value: dec!(80),
                    probability: dec!(0.5),
                },
            ],
            reference_point: dec!(100),
            current_value: dec!(100),
            loss_aversion_lambda: dec!(2.25),
            alpha: dec!(0.88),
            beta_param: dec!(0.88),
            gamma: dec!(0.61),
            delta_param: dec!(0.69),
            holding_period_months: 12,
            annual_return_history: vec![],
        };
        let result = analyze_prospect_theory(&input).unwrap();

        // Symmetric payoff but loss aversion makes PV negative
        assert!(
            result.prospect_value < Decimal::ZERO,
            "Symmetric gamble with loss aversion should have negative PV, got {}",
            result.prospect_value
        );
        // EV should be exactly 100
        assert_eq!(result.expected_value, dec!(100));
    }

    #[test]
    fn test_single_outcome_certain() {
        let input = ProspectTheoryInput {
            outcomes: vec![Outcome {
                description: "Certain gain".to_string(),
                value: dec!(110),
                probability: dec!(1),
            }],
            reference_point: dec!(100),
            current_value: dec!(105),
            loss_aversion_lambda: dec!(2.25),
            alpha: dec!(0.88),
            beta_param: dec!(0.88),
            gamma: dec!(0.61),
            delta_param: dec!(0.69),
            holding_period_months: 12,
            annual_return_history: vec![],
        };
        let result = analyze_prospect_theory(&input).unwrap();

        // Certain outcome: decision weight for p=1 should be 1
        assert_eq!(result.outcome_analysis[0].probability, dec!(1));
        // EV = 110
        assert_eq!(result.expected_value, dec!(110));
    }

    #[test]
    fn test_probability_distortion_small_probs() {
        // Small probabilities should be overweighted
        let input = ProspectTheoryInput {
            outcomes: vec![
                Outcome {
                    description: "Unlikely big gain".to_string(),
                    value: dec!(1000),
                    probability: dec!(0.05),
                },
                Outcome {
                    description: "Likely small loss".to_string(),
                    value: dec!(95),
                    probability: dec!(0.95),
                },
            ],
            reference_point: dec!(100),
            current_value: dec!(100),
            loss_aversion_lambda: dec!(2.25),
            alpha: dec!(0.88),
            beta_param: dec!(0.88),
            gamma: dec!(0.61),
            delta_param: dec!(0.69),
            holding_period_months: 12,
            annual_return_history: vec![],
        };
        let result = analyze_prospect_theory(&input).unwrap();

        // The 5% probability outcome should have decision weight > 5%
        let small_prob_dw = result.probability_distortion[0].decision_weight;
        assert!(
            small_prob_dw > dec!(0.05),
            "Small probability should be overweighted, got {}",
            small_prob_dw
        );
    }

    #[test]
    fn test_probability_distortion_large_probs() {
        // Large probabilities should be underweighted
        let input = ProspectTheoryInput {
            outcomes: vec![
                Outcome {
                    description: "Very likely small gain".to_string(),
                    value: dec!(105),
                    probability: dec!(0.95),
                },
                Outcome {
                    description: "Unlikely big loss".to_string(),
                    value: dec!(50),
                    probability: dec!(0.05),
                },
            ],
            reference_point: dec!(100),
            current_value: dec!(100),
            loss_aversion_lambda: dec!(2.25),
            alpha: dec!(0.88),
            beta_param: dec!(0.88),
            gamma: dec!(0.61),
            delta_param: dec!(0.69),
            holding_period_months: 12,
            annual_return_history: vec![],
        };
        let result = analyze_prospect_theory(&input).unwrap();

        // The 95% probability gain should have decision weight < 95%
        let large_prob_dw = result.probability_distortion[0].decision_weight;
        assert!(
            large_prob_dw < dec!(0.95),
            "Large probability should be underweighted, got {}",
            large_prob_dw
        );
    }

    #[test]
    fn test_disposition_effect_in_gain() {
        let input = ProspectTheoryInput {
            outcomes: vec![
                Outcome {
                    description: "Up".to_string(),
                    value: dec!(120),
                    probability: dec!(0.5),
                },
                Outcome {
                    description: "Down".to_string(),
                    value: dec!(80),
                    probability: dec!(0.5),
                },
            ],
            reference_point: dec!(100),
            current_value: dec!(115), // In gain territory
            loss_aversion_lambda: dec!(2.25),
            alpha: dec!(0.88),
            beta_param: dec!(0.88),
            gamma: dec!(0.61),
            delta_param: dec!(0.69),
            holding_period_months: 12,
            // All positive returns while in gain territory = high disposition
            annual_return_history: vec![dec!(0.08), dec!(0.12), dec!(0.05), dec!(0.10), dec!(0.15)],
        };
        let result = analyze_prospect_theory(&input).unwrap();

        // All positive returns while in gain territory → disposition = 100
        assert_eq!(result.disposition_effect_score, dec!(100));
    }

    #[test]
    fn test_disposition_effect_in_loss() {
        let input = ProspectTheoryInput {
            outcomes: vec![
                Outcome {
                    description: "Up".to_string(),
                    value: dec!(120),
                    probability: dec!(0.5),
                },
                Outcome {
                    description: "Down".to_string(),
                    value: dec!(80),
                    probability: dec!(0.5),
                },
            ],
            reference_point: dec!(100),
            current_value: dec!(85), // In loss territory
            loss_aversion_lambda: dec!(2.25),
            alpha: dec!(0.88),
            beta_param: dec!(0.88),
            gamma: dec!(0.61),
            delta_param: dec!(0.69),
            holding_period_months: 12,
            // All negative returns while in loss territory = high disposition
            annual_return_history: vec![
                dec!(-0.08),
                dec!(-0.12),
                dec!(-0.05),
                dec!(-0.10),
                dec!(-0.15),
            ],
        };
        let result = analyze_prospect_theory(&input).unwrap();

        assert_eq!(result.disposition_effect_score, dec!(100));
    }

    #[test]
    fn test_disposition_effect_no_history() {
        let mut input = default_input();
        input.annual_return_history = vec![];
        let result = analyze_prospect_theory(&input).unwrap();

        // Default score with no history
        assert_eq!(result.disposition_effect_score, dec!(50));
    }

    #[test]
    fn test_mental_accounting_strong_gain() {
        let mut input = default_input();
        input.current_value = dec!(115); // 15% above reference
        let result = analyze_prospect_theory(&input).unwrap();

        assert!(result.mental_accounting_zones.strong_gain_zone > Decimal::ZERO);
        assert_eq!(result.mental_accounting_zones.weak_gain_zone, Decimal::ZERO);
        assert_eq!(result.mental_accounting_zones.weak_loss_zone, Decimal::ZERO);
        assert_eq!(
            result.mental_accounting_zones.strong_loss_zone,
            Decimal::ZERO
        );
    }

    #[test]
    fn test_mental_accounting_weak_gain() {
        let mut input = default_input();
        input.current_value = dec!(105); // 5% above reference
        let result = analyze_prospect_theory(&input).unwrap();

        assert_eq!(
            result.mental_accounting_zones.strong_gain_zone,
            Decimal::ZERO
        );
        assert!(result.mental_accounting_zones.weak_gain_zone > Decimal::ZERO);
    }

    #[test]
    fn test_mental_accounting_weak_loss() {
        let mut input = default_input();
        input.current_value = dec!(95); // 5% below reference
        let result = analyze_prospect_theory(&input).unwrap();

        assert!(result.mental_accounting_zones.weak_loss_zone > Decimal::ZERO);
        assert_eq!(
            result.mental_accounting_zones.strong_loss_zone,
            Decimal::ZERO
        );
    }

    #[test]
    fn test_mental_accounting_strong_loss() {
        let mut input = default_input();
        input.current_value = dec!(85); // 15% below reference
        let result = analyze_prospect_theory(&input).unwrap();

        assert!(result.mental_accounting_zones.strong_loss_zone > Decimal::ZERO);
        assert_eq!(result.mental_accounting_zones.weak_loss_zone, Decimal::ZERO);
    }

    #[test]
    fn test_loss_aversion_impact() {
        let input = default_input();
        let result = analyze_prospect_theory(&input).unwrap();

        // With lambda=2.25, loss aversion should have a significant impact
        assert!(result.loss_aversion_impact != Decimal::ZERO);
    }

    #[test]
    fn test_lambda_one_no_loss_aversion() {
        let mut input = default_input();
        input.loss_aversion_lambda = Decimal::ONE;
        let result = analyze_prospect_theory(&input).unwrap();

        // When lambda=1, loss aversion impact should be zero
        assert_eq!(result.loss_aversion_impact, Decimal::ZERO);
    }

    #[test]
    fn test_high_loss_aversion() {
        let mut input = default_input();
        input.loss_aversion_lambda = dec!(5.0);
        let result_high = analyze_prospect_theory(&input).unwrap();

        input.loss_aversion_lambda = dec!(2.25);
        let result_normal = analyze_prospect_theory(&input).unwrap();

        // Higher lambda should result in lower (more negative) prospect value
        assert!(
            result_high.prospect_value < result_normal.prospect_value,
            "Higher lambda should reduce prospect value"
        );
    }

    #[test]
    fn test_framing_bias_score() {
        let input = default_input();
        let result = analyze_prospect_theory(&input).unwrap();

        assert!(result.framing_bias_score >= Decimal::ZERO);
        assert!(result.framing_bias_score <= dec!(100));
    }

    #[test]
    fn test_behavioral_recommendation_not_empty() {
        let input = default_input();
        let result = analyze_prospect_theory(&input).unwrap();

        assert!(!result.behavioral_recommendation.is_empty());
    }

    #[test]
    fn test_outcome_analysis_count() {
        let input = default_input();
        let result = analyze_prospect_theory(&input).unwrap();

        assert_eq!(result.outcome_analysis.len(), input.outcomes.len());
        assert_eq!(result.probability_distortion.len(), input.outcomes.len());
    }

    #[test]
    fn test_invalid_empty_outcomes() {
        let mut input = default_input();
        input.outcomes = vec![];
        assert!(analyze_prospect_theory(&input).is_err());
    }

    #[test]
    fn test_invalid_probability_sum() {
        let mut input = default_input();
        input.outcomes = vec![
            Outcome {
                description: "A".to_string(),
                value: dec!(100),
                probability: dec!(0.3),
            },
            Outcome {
                description: "B".to_string(),
                value: dec!(100),
                probability: dec!(0.3),
            },
        ];
        assert!(analyze_prospect_theory(&input).is_err());
    }

    #[test]
    fn test_invalid_negative_probability() {
        let mut input = default_input();
        input.outcomes[0].probability = dec!(-0.1);
        assert!(analyze_prospect_theory(&input).is_err());
    }

    #[test]
    fn test_invalid_alpha_zero() {
        let mut input = default_input();
        input.alpha = Decimal::ZERO;
        assert!(analyze_prospect_theory(&input).is_err());
    }

    #[test]
    fn test_invalid_alpha_above_one() {
        let mut input = default_input();
        input.alpha = dec!(1.5);
        assert!(analyze_prospect_theory(&input).is_err());
    }

    #[test]
    fn test_invalid_beta_zero() {
        let mut input = default_input();
        input.beta_param = Decimal::ZERO;
        assert!(analyze_prospect_theory(&input).is_err());
    }

    #[test]
    fn test_invalid_lambda_negative() {
        let mut input = default_input();
        input.loss_aversion_lambda = dec!(-1);
        assert!(analyze_prospect_theory(&input).is_err());
    }

    #[test]
    fn test_value_function_gains() {
        // v(10) with alpha=0.88 => 10^0.88
        let vf = value_function(dec!(10), dec!(0.88), dec!(0.88), dec!(2.25));
        assert!(vf > Decimal::ZERO);
        assert!(vf < dec!(10)); // Concavity: x^0.88 < x for x > 1
    }

    #[test]
    fn test_value_function_losses() {
        // v(-10) with beta=0.88, lambda=2.25 => -2.25 * 10^0.88
        let vf = value_function(dec!(-10), dec!(0.88), dec!(0.88), dec!(2.25));
        assert!(vf < Decimal::ZERO);
        // Loss should be more painful: |v(-10)| > v(10) due to lambda > 1
        let vf_gain = value_function(dec!(10), dec!(0.88), dec!(0.88), dec!(2.25));
        assert!(vf.abs() > vf_gain);
    }

    #[test]
    fn test_value_function_zero() {
        let vf = value_function(Decimal::ZERO, dec!(0.88), dec!(0.88), dec!(2.25));
        assert_eq!(vf, Decimal::ZERO);
    }

    #[test]
    fn test_weight_gain_boundary_zero() {
        let w = weight_gain(Decimal::ZERO, dec!(0.61));
        assert_eq!(w, Decimal::ZERO);
    }

    #[test]
    fn test_weight_gain_boundary_one() {
        let w = weight_gain(Decimal::ONE, dec!(0.61));
        assert_eq!(w, Decimal::ONE);
    }

    #[test]
    fn test_weight_loss_boundary_zero() {
        let w = weight_loss(Decimal::ZERO, dec!(0.69));
        assert_eq!(w, Decimal::ZERO);
    }

    #[test]
    fn test_weight_loss_boundary_one() {
        let w = weight_loss(Decimal::ONE, dec!(0.69));
        assert_eq!(w, Decimal::ONE);
    }

    #[test]
    fn test_many_outcomes() {
        let outcomes: Vec<Outcome> = (0..10)
            .map(|i| Outcome {
                description: format!("Outcome {}", i),
                value: dec!(80) + Decimal::from(i as u32) * dec!(5),
                probability: dec!(0.1),
            })
            .collect();
        let input = ProspectTheoryInput {
            outcomes,
            reference_point: dec!(100),
            current_value: dec!(100),
            loss_aversion_lambda: dec!(2.25),
            alpha: dec!(0.88),
            beta_param: dec!(0.88),
            gamma: dec!(0.61),
            delta_param: dec!(0.69),
            holding_period_months: 12,
            annual_return_history: vec![],
        };
        let result = analyze_prospect_theory(&input).unwrap();
        assert_eq!(result.outcome_analysis.len(), 10);
    }

    #[test]
    fn test_extreme_gain() {
        let input = ProspectTheoryInput {
            outcomes: vec![Outcome {
                description: "Extreme gain".to_string(),
                value: dec!(10000),
                probability: dec!(1.0),
            }],
            reference_point: dec!(100),
            current_value: dec!(100),
            loss_aversion_lambda: dec!(2.25),
            alpha: dec!(0.88),
            beta_param: dec!(0.88),
            gamma: dec!(0.61),
            delta_param: dec!(0.69),
            holding_period_months: 12,
            annual_return_history: vec![],
        };
        let result = analyze_prospect_theory(&input).unwrap();
        assert!(result.prospect_value > Decimal::ZERO);
    }

    #[test]
    fn test_extreme_loss() {
        let input = ProspectTheoryInput {
            outcomes: vec![Outcome {
                description: "Extreme loss".to_string(),
                value: dec!(1),
                probability: dec!(1.0),
            }],
            reference_point: dec!(100),
            current_value: dec!(50),
            loss_aversion_lambda: dec!(2.25),
            alpha: dec!(0.88),
            beta_param: dec!(0.88),
            gamma: dec!(0.61),
            delta_param: dec!(0.69),
            holding_period_months: 12,
            annual_return_history: vec![dec!(-0.20), dec!(-0.30)],
        };
        let result = analyze_prospect_theory(&input).unwrap();
        assert!(result.prospect_value < Decimal::ZERO);
    }

    #[test]
    fn test_exp_decimal_basic() {
        let result = exp_decimal(Decimal::ZERO);
        assert_eq!(result, Decimal::ONE);
    }

    #[test]
    fn test_ln_decimal_basic() {
        let result = ln_decimal(Decimal::ONE);
        assert_eq!(result, Decimal::ZERO);
    }

    #[test]
    fn test_pow_decimal_identity() {
        let result = pow_decimal(dec!(5), Decimal::ONE);
        assert_eq!(result, dec!(5));
    }

    #[test]
    fn test_pow_decimal_zero_exponent() {
        let result = pow_decimal(dec!(5), Decimal::ZERO);
        assert_eq!(result, Decimal::ONE);
    }
}
