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
pub struct CreditExposure {
    pub name: String,
    pub exposure: Decimal,
    /// Annual probability of default (0 to 1)
    pub probability_of_default: Decimal,
    /// Loss given default (0 to 1)
    pub loss_given_default: Decimal,
    /// Credit rating, e.g. "BBB", "BB"
    pub rating: String,
    pub sector: String,
    pub maturity_years: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioRiskInput {
    pub portfolio_name: String,
    pub exposures: Vec<CreditExposure>,
    /// Asset correlation (typically 0.1-0.3)
    pub default_correlation: Decimal,
    /// e.g. 0.99 for 99% VaR
    pub confidence_level: Decimal,
    /// Typically 1
    pub time_horizon_years: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExposureRisk {
    pub name: String,
    pub exposure: Decimal,
    pub expected_loss: Decimal,
    pub unexpected_loss: Decimal,
    pub risk_contribution: Decimal,
    pub marginal_risk: Decimal,
    pub pct_of_portfolio: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConcentrationMetrics {
    /// Herfindahl by obligor
    pub hhi_name: Decimal,
    /// Herfindahl by sector
    pub hhi_sector: Decimal,
    /// Top 10 exposures as % of total
    pub top_10_pct: Decimal,
    pub largest_exposure_pct: Decimal,
    /// 1 / HHI
    pub effective_number_names: Decimal,
    pub granularity_adjustment: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioRiskOutput {
    pub total_exposure: Decimal,
    pub expected_loss: Decimal,
    pub unexpected_loss: Decimal,
    /// VaR at confidence level
    pub credit_var: Decimal,
    /// credit_var - expected_loss
    pub economic_capital: Decimal,
    pub expected_loss_pct: Decimal,
    pub unexpected_loss_pct: Decimal,
    pub credit_var_pct: Decimal,
    pub exposure_risks: Vec<ExposureRisk>,
    pub concentration: ConcentrationMetrics,
    /// Exposure-weighted average PD
    pub portfolio_pd: Decimal,
    /// Exposure-weighted average LGD
    pub portfolio_lgd: Decimal,
    /// 1 - (portfolio UL / sum of standalone ULs)
    pub diversification_benefit: Decimal,
    pub methodology: String,
    pub assumptions: HashMap<String, String>,
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// Decimal math helpers
// ---------------------------------------------------------------------------

/// Newton's method sqrt: 20 iterations.
fn sqrt_decimal(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    if x == Decimal::ONE {
        return Decimal::ONE;
    }
    let two = dec!(2);
    let mut guess = x / two;
    if x > dec!(100) {
        guess = dec!(10);
    } else if x < dec!(0.01) {
        guess = dec!(0.1);
    }
    for _ in 0..20 {
        guess = (guess + x / guess) / two;
    }
    guess
}

/// Taylor series exp(x) with range reduction.
fn exp_decimal(x: Decimal) -> Decimal {
    let two = dec!(2);
    if x > two || x < -two {
        let half = exp_decimal(x / two);
        return half * half;
    }
    let mut sum = Decimal::ONE;
    let mut term = Decimal::ONE;
    for n in 1u32..=25 {
        term = term * x / Decimal::from(n);
        sum += term;
    }
    sum
}

/// Standard normal PDF.
fn norm_pdf(x: Decimal) -> Decimal {
    let two_pi = dec!(6.283185307179586);
    let exponent = -(x * x) / dec!(2);
    exp_decimal(exponent) / sqrt_decimal(two_pi)
}

/// Standard normal CDF (Abramowitz & Stegun).
fn norm_cdf(x: Decimal) -> Decimal {
    let b1 = dec!(0.319381530);
    let b2 = dec!(-0.356563782);
    let b3 = dec!(1.781477937);
    let b4 = dec!(-1.821255978);
    let b5 = dec!(1.330274429);
    let p = dec!(0.2316419);

    let abs_x = if x < Decimal::ZERO { -x } else { x };
    let t = Decimal::ONE / (Decimal::ONE + p * abs_x);
    let poly = t * (b1 + t * (b2 + t * (b3 + t * (b4 + t * b5))));
    let cdf_pos = Decimal::ONE - norm_pdf(abs_x) * poly;

    if x < Decimal::ZERO {
        Decimal::ONE - cdf_pos
    } else {
        cdf_pos
    }
}

/// Inverse normal CDF (rational approximation, Abramowitz & Stegun 26.2.23).
/// For p in (0, 1), returns x such that Phi(x) = p.
fn inverse_norm(p: Decimal) -> Decimal {
    // For p close to 0 or 1, clamp to avoid divergence
    let p_clamped = if p < dec!(0.0000001) {
        dec!(0.0000001)
    } else if p > dec!(0.9999999) {
        dec!(0.9999999)
    } else {
        p
    };

    // Use symmetry: if p > 0.5, compute for 1-p and negate
    let (p_work, negate) = if p_clamped > dec!(0.5) {
        (Decimal::ONE - p_clamped, true)
    } else {
        (p_clamped, false)
    };

    // Rational approximation for the central region
    // t = sqrt(-2 * ln(p))
    let ln_p = ln_decimal(p_work);
    let t = sqrt_decimal(dec!(-2) * ln_p);

    // Coefficients (Abramowitz & Stegun 26.2.23)
    let c0 = dec!(2.515517);
    let c1 = dec!(0.802853);
    let c2 = dec!(0.010328);
    let d1 = dec!(1.432788);
    let d2 = dec!(0.189269);
    let d3 = dec!(0.001308);

    let numerator = c0 + c1 * t + c2 * t * t;
    let denominator = Decimal::ONE + d1 * t + d2 * t * t + d3 * t * t * t;

    let result = t - numerator / denominator;

    // Result is for the left tail (negative), flip sign for right tail
    if negate {
        result
    } else {
        -result
    }
}

/// Natural log via Newton's method.
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

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_input(input: &PortfolioRiskInput) -> CorpFinanceResult<Vec<String>> {
    let mut warnings = Vec::new();

    if input.exposures.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "at least one exposure is required".into(),
        ));
    }

    if input.default_correlation < Decimal::ZERO || input.default_correlation >= Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "default_correlation".into(),
            reason: "must be in [0, 1)".into(),
        });
    }

    if input.confidence_level <= dec!(0.5) || input.confidence_level >= Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "confidence_level".into(),
            reason: "must be in (0.5, 1)".into(),
        });
    }

    if input.time_horizon_years <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "time_horizon_years".into(),
            reason: "must be positive".into(),
        });
    }

    let mut total_exp = Decimal::ZERO;
    for (i, exp) in input.exposures.iter().enumerate() {
        if exp.exposure <= Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("exposures[{}].exposure", i),
                reason: "must be positive".into(),
            });
        }
        if exp.probability_of_default < Decimal::ZERO || exp.probability_of_default > Decimal::ONE {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("exposures[{}].probability_of_default", i),
                reason: "must be in [0, 1]".into(),
            });
        }
        if exp.loss_given_default < Decimal::ZERO || exp.loss_given_default > Decimal::ONE {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("exposures[{}].loss_given_default", i),
                reason: "must be in [0, 1]".into(),
            });
        }
        total_exp += exp.exposure;
    }

    // Concentration warnings
    for exp in &input.exposures {
        let pct = exp.exposure / total_exp;
        if pct > dec!(0.10) {
            warnings.push(format!(
                "Exposure '{}' is {:.1}% of total portfolio (>10%)",
                exp.name,
                pct * dec!(100)
            ));
        }
    }

    Ok(warnings)
}

// ---------------------------------------------------------------------------
// Core analytics
// ---------------------------------------------------------------------------

/// Compute portfolio credit risk analytics.
pub fn calculate_portfolio_risk(
    input: &PortfolioRiskInput,
) -> CorpFinanceResult<ComputationOutput<PortfolioRiskOutput>> {
    let start = Instant::now();
    let mut warnings = validate_input(input)?;

    let rho = input.default_correlation;
    let n = input.exposures.len();

    // Total exposure
    let total_exposure: Decimal = input.exposures.iter().map(|e| e.exposure).sum();

    // Per-exposure expected and unexpected losses
    let mut el_vec: Vec<Decimal> = Vec::with_capacity(n);
    let mut ul_vec: Vec<Decimal> = Vec::with_capacity(n);
    let mut weights: Vec<Decimal> = Vec::with_capacity(n);

    for exp in &input.exposures {
        let ead = exp.exposure;
        let pd = exp.probability_of_default;
        let lgd = exp.loss_given_default;

        // Expected loss
        let el = pd * lgd * ead;
        el_vec.push(el);

        // Unexpected loss (standalone): UL_i = EAD * LGD * sqrt(PD * (1 - PD))
        let ul = ead * lgd * sqrt_decimal(pd * (Decimal::ONE - pd));
        ul_vec.push(ul);

        weights.push(ead / total_exposure);
    }

    let portfolio_el: Decimal = el_vec.iter().copied().sum();

    // Portfolio UL using correlation: UL = sqrt(sum_i sum_j rho_ij * UL_i * UL_j)
    // where rho_ij = rho for i != j, 1 for i == j
    let mut ul_sq_sum = Decimal::ZERO;
    for i in 0..n {
        for j in 0..n {
            let corr = if i == j { Decimal::ONE } else { rho };
            ul_sq_sum += corr * ul_vec[i] * ul_vec[j];
        }
    }
    let portfolio_ul = sqrt_decimal(ul_sq_sum);

    // Sum of standalone ULs for diversification benefit
    let sum_standalone_ul: Decimal = ul_vec.iter().copied().sum();
    let diversification_benefit = if sum_standalone_ul > Decimal::ZERO {
        Decimal::ONE - portfolio_ul / sum_standalone_ul
    } else {
        Decimal::ZERO
    };

    // Credit VaR (Vasicek / Gaussian copula)
    // Conditional PD: PD_cond = Phi((Phi^-1(PD) + sqrt(rho) * Phi^-1(confidence)) / sqrt(1-rho))
    let sqrt_rho = sqrt_decimal(rho);
    let sqrt_one_minus_rho = sqrt_decimal(Decimal::ONE - rho);
    let z_conf = inverse_norm(input.confidence_level);

    let mut credit_var = Decimal::ZERO;
    let mut pd_cond_vec: Vec<Decimal> = Vec::with_capacity(n);
    for exp in &input.exposures {
        let pd = exp.probability_of_default;
        let lgd = exp.loss_given_default;
        let ead = exp.exposure;

        let pd_cond = if rho == Decimal::ZERO {
            // No correlation: conditional PD = PD
            pd
        } else if pd == Decimal::ZERO {
            Decimal::ZERO
        } else if pd == Decimal::ONE {
            Decimal::ONE
        } else {
            let z_pd = inverse_norm(pd);
            let numerator = z_pd + sqrt_rho * z_conf;
            let arg = numerator / sqrt_one_minus_rho;
            norm_cdf(arg)
        };
        pd_cond_vec.push(pd_cond);
        credit_var += ead * lgd * pd_cond;
    }

    let economic_capital = credit_var - portfolio_el;

    // Marginal risk and risk contribution
    // Marginal risk_i = (rho * UL_i * portfolio_UL + (1-rho) * UL_i^2) / portfolio_UL
    // Risk contribution_i = weight_i * marginal_risk_i
    let mut exposure_risks: Vec<ExposureRisk> = Vec::with_capacity(n);
    let mut total_risk_contribution = Decimal::ZERO;

    for i in 0..n {
        let marginal = if portfolio_ul > Decimal::ZERO {
            (rho * ul_vec[i] * portfolio_ul + (Decimal::ONE - rho) * ul_vec[i] * ul_vec[i])
                / portfolio_ul
        } else {
            Decimal::ZERO
        };

        let risk_contrib = weights[i] * marginal;
        total_risk_contribution += risk_contrib;

        exposure_risks.push(ExposureRisk {
            name: input.exposures[i].name.clone(),
            exposure: input.exposures[i].exposure,
            expected_loss: el_vec[i],
            unexpected_loss: ul_vec[i],
            risk_contribution: risk_contrib,
            marginal_risk: marginal,
            pct_of_portfolio: weights[i] * dec!(100),
        });
    }

    // Concentration metrics
    // HHI by name
    let hhi_name: Decimal = weights.iter().map(|w| *w * *w).sum();

    // HHI by sector
    let mut sector_weights: HashMap<String, Decimal> = HashMap::new();
    for (i, exp) in input.exposures.iter().enumerate() {
        *sector_weights
            .entry(exp.sector.clone())
            .or_insert(Decimal::ZERO) += weights[i];
    }
    let hhi_sector: Decimal = sector_weights.values().map(|w| *w * *w).sum();

    // Top 10 exposure percentage
    let mut sorted_weights = weights.clone();
    sorted_weights.sort_by(|a, b| b.cmp(a));
    let top_10_pct: Decimal = sorted_weights.iter().take(10).copied().sum::<Decimal>() * dec!(100);

    let largest_exposure_pct = sorted_weights.first().copied().unwrap_or(Decimal::ZERO) * dec!(100);

    let effective_number_names = if hhi_name > Decimal::ZERO {
        Decimal::ONE / hhi_name
    } else {
        Decimal::ZERO
    };

    // Granularity adjustment (simplified Gordy): (1 / (2 * n_eff)) * portfolio_UL^2
    let granularity_adjustment = if effective_number_names > Decimal::ZERO {
        portfolio_ul * portfolio_ul / (dec!(2) * effective_number_names)
    } else {
        Decimal::ZERO
    };

    // Warn if concentrated
    if hhi_name > dec!(0.25) {
        warnings.push("Portfolio is highly concentrated (HHI > 0.25)".into());
    }

    // Portfolio-level weighted averages
    let portfolio_pd: Decimal = input
        .exposures
        .iter()
        .zip(weights.iter())
        .map(|(e, w)| e.probability_of_default * *w)
        .sum();

    let portfolio_lgd: Decimal = input
        .exposures
        .iter()
        .zip(weights.iter())
        .map(|(e, w)| e.loss_given_default * *w)
        .sum();

    let el_pct = if total_exposure > Decimal::ZERO {
        portfolio_el / total_exposure * dec!(100)
    } else {
        Decimal::ZERO
    };
    let ul_pct = if total_exposure > Decimal::ZERO {
        portfolio_ul / total_exposure * dec!(100)
    } else {
        Decimal::ZERO
    };
    let var_pct = if total_exposure > Decimal::ZERO {
        credit_var / total_exposure * dec!(100)
    } else {
        Decimal::ZERO
    };

    let mut assumptions = HashMap::new();
    assumptions.insert(
        "model".into(),
        "Vasicek single-factor / Gaussian copula".into(),
    );
    assumptions.insert(
        "default_correlation".into(),
        input.default_correlation.to_string(),
    );
    assumptions.insert(
        "confidence_level".into(),
        input.confidence_level.to_string(),
    );
    assumptions.insert(
        "time_horizon".into(),
        format!("{} year(s)", input.time_horizon_years),
    );

    let output = PortfolioRiskOutput {
        total_exposure,
        expected_loss: portfolio_el,
        unexpected_loss: portfolio_ul,
        credit_var,
        economic_capital,
        expected_loss_pct: el_pct,
        unexpected_loss_pct: ul_pct,
        credit_var_pct: var_pct,
        exposure_risks,
        concentration: ConcentrationMetrics {
            hhi_name,
            hhi_sector,
            top_10_pct,
            largest_exposure_pct,
            effective_number_names,
            granularity_adjustment,
        },
        portfolio_pd,
        portfolio_lgd,
        diversification_benefit,
        methodology: "Vasicek single-factor / Gaussian copula".into(),
        assumptions,
        warnings: warnings.clone(),
    };

    let elapsed = start.elapsed().as_micros() as u64;
    let meta_assumptions = serde_json::json!({
        "model": "Vasicek single-factor / Gaussian copula",
        "correlation": input.default_correlation.to_string(),
        "confidence": input.confidence_level.to_string(),
    });

    Ok(with_metadata(
        "Vasicek single-factor / Gaussian copula",
        &meta_assumptions,
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

    fn approx_eq(a: Decimal, b: Decimal, tol: Decimal) -> bool {
        let diff = a - b;
        let abs_diff = if diff < Decimal::ZERO { -diff } else { diff };
        abs_diff < tol
    }

    fn make_exposure(
        name: &str,
        exposure: Decimal,
        pd: Decimal,
        lgd: Decimal,
        rating: &str,
        sector: &str,
    ) -> CreditExposure {
        CreditExposure {
            name: name.into(),
            exposure,
            probability_of_default: pd,
            loss_given_default: lgd,
            rating: rating.into(),
            sector: sector.into(),
            maturity_years: dec!(5),
        }
    }

    fn single_exposure_input() -> PortfolioRiskInput {
        PortfolioRiskInput {
            portfolio_name: "Test".into(),
            exposures: vec![make_exposure(
                "Obligor A",
                dec!(1000000),
                dec!(0.02),
                dec!(0.45),
                "BBB",
                "Industrials",
            )],
            default_correlation: dec!(0.2),
            confidence_level: dec!(0.99),
            time_horizon_years: dec!(1),
        }
    }

    fn five_exposure_input() -> PortfolioRiskInput {
        PortfolioRiskInput {
            portfolio_name: "Diversified".into(),
            exposures: vec![
                make_exposure("A", dec!(200000), dec!(0.01), dec!(0.40), "A", "Tech"),
                make_exposure(
                    "B",
                    dec!(200000),
                    dec!(0.02),
                    dec!(0.45),
                    "BBB",
                    "Industrials",
                ),
                make_exposure("C", dec!(200000), dec!(0.03), dec!(0.50), "BB", "Energy"),
                make_exposure(
                    "D",
                    dec!(200000),
                    dec!(0.015),
                    dec!(0.40),
                    "BBB",
                    "Healthcare",
                ),
                make_exposure(
                    "E",
                    dec!(200000),
                    dec!(0.025),
                    dec!(0.55),
                    "BB",
                    "Financials",
                ),
            ],
            default_correlation: dec!(0.15),
            confidence_level: dec!(0.99),
            time_horizon_years: dec!(1),
        }
    }

    fn equal_weight_input() -> PortfolioRiskInput {
        PortfolioRiskInput {
            portfolio_name: "Equal Weight".into(),
            exposures: vec![
                make_exposure("A", dec!(100000), dec!(0.02), dec!(0.45), "BBB", "Tech"),
                make_exposure(
                    "B",
                    dec!(100000),
                    dec!(0.02),
                    dec!(0.45),
                    "BBB",
                    "Industrials",
                ),
                make_exposure("C", dec!(100000), dec!(0.02), dec!(0.45), "BBB", "Energy"),
                make_exposure(
                    "D",
                    dec!(100000),
                    dec!(0.02),
                    dec!(0.45),
                    "BBB",
                    "Healthcare",
                ),
            ],
            default_correlation: dec!(0.2),
            confidence_level: dec!(0.99),
            time_horizon_years: dec!(1),
        }
    }

    fn concentrated_input() -> PortfolioRiskInput {
        PortfolioRiskInput {
            portfolio_name: "Concentrated".into(),
            exposures: vec![
                make_exposure("Big", dec!(900000), dec!(0.03), dec!(0.45), "BB", "Energy"),
                make_exposure("Small", dec!(100000), dec!(0.01), dec!(0.40), "A", "Tech"),
            ],
            default_correlation: dec!(0.2),
            confidence_level: dec!(0.99),
            time_horizon_years: dec!(1),
        }
    }

    // -----------------------------------------------------------------------
    // Math helper tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_norm_cdf_zero() {
        assert!(approx_eq(norm_cdf(dec!(0)), dec!(0.5), dec!(0.001)));
    }

    #[test]
    fn test_norm_cdf_tails() {
        assert!(norm_cdf(dec!(5)) > dec!(0.999));
        assert!(norm_cdf(dec!(-5)) < dec!(0.001));
    }

    #[test]
    fn test_inverse_norm_symmetry() {
        // inverse_norm(0.5) should be ~ 0
        assert!(approx_eq(inverse_norm(dec!(0.5)), dec!(0), dec!(0.01)));
    }

    #[test]
    fn test_inverse_norm_99() {
        // Phi^-1(0.99) ~ 2.326
        let z = inverse_norm(dec!(0.99));
        assert!(
            approx_eq(z, dec!(2.326), dec!(0.02)),
            "inverse_norm(0.99) = {} expected ~2.326",
            z
        );
    }

    #[test]
    fn test_inverse_norm_roundtrip() {
        // norm_cdf(inverse_norm(p)) ~ p
        let p = dec!(0.95);
        let z = inverse_norm(p);
        let recovered = norm_cdf(z);
        assert!(
            approx_eq(recovered, p, dec!(0.005)),
            "roundtrip: norm_cdf(inverse_norm(0.95)) = {} expected ~0.95",
            recovered
        );
    }

    // -----------------------------------------------------------------------
    // Single exposure tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_single_exposure_expected_loss() {
        let input = single_exposure_input();
        let result = calculate_portfolio_risk(&input).unwrap();
        let out = &result.result;
        // EL = PD * LGD * EAD = 0.02 * 0.45 * 1_000_000 = 9_000
        assert!(
            approx_eq(out.expected_loss, dec!(9000), dec!(1)),
            "EL = {} expected 9000",
            out.expected_loss
        );
    }

    #[test]
    fn test_single_exposure_total() {
        let input = single_exposure_input();
        let result = calculate_portfolio_risk(&input).unwrap();
        assert_eq!(result.result.total_exposure, dec!(1000000));
    }

    #[test]
    fn test_single_exposure_unexpected_loss() {
        let input = single_exposure_input();
        let result = calculate_portfolio_risk(&input).unwrap();
        let out = &result.result;
        // UL = EAD * LGD * sqrt(PD*(1-PD)) = 1_000_000 * 0.45 * sqrt(0.02*0.98)
        // sqrt(0.0196) ~ 0.14 => UL ~ 63_000
        assert!(
            out.unexpected_loss > dec!(60000) && out.unexpected_loss < dec!(70000),
            "UL = {} expected ~63000",
            out.unexpected_loss
        );
    }

    #[test]
    fn test_single_exposure_var_exceeds_el() {
        let input = single_exposure_input();
        let result = calculate_portfolio_risk(&input).unwrap();
        let out = &result.result;
        assert!(
            out.credit_var > out.expected_loss,
            "VaR {} should exceed EL {}",
            out.credit_var,
            out.expected_loss
        );
    }

    #[test]
    fn test_single_exposure_economic_capital_positive() {
        let input = single_exposure_input();
        let result = calculate_portfolio_risk(&input).unwrap();
        assert!(
            result.result.economic_capital > Decimal::ZERO,
            "Economic capital should be positive"
        );
    }

    // -----------------------------------------------------------------------
    // Five-exposure diversified portfolio
    // -----------------------------------------------------------------------

    #[test]
    fn test_five_exposure_total() {
        let input = five_exposure_input();
        let result = calculate_portfolio_risk(&input).unwrap();
        assert_eq!(result.result.total_exposure, dec!(1000000));
    }

    #[test]
    fn test_five_exposure_el_sum() {
        let input = five_exposure_input();
        let result = calculate_portfolio_risk(&input).unwrap();
        // Sum of individual ELs
        let individual_sum: Decimal = result
            .result
            .exposure_risks
            .iter()
            .map(|r| r.expected_loss)
            .sum();
        assert!(
            approx_eq(individual_sum, result.result.expected_loss, dec!(1)),
            "Sum of individual ELs {} should match portfolio EL {}",
            individual_sum,
            result.result.expected_loss
        );
    }

    #[test]
    fn test_five_exposure_diversification_positive() {
        let input = five_exposure_input();
        let result = calculate_portfolio_risk(&input).unwrap();
        assert!(
            result.result.diversification_benefit > Decimal::ZERO,
            "Diversification benefit {} should be positive",
            result.result.diversification_benefit
        );
    }

    #[test]
    fn test_five_exposure_diversification_less_than_one() {
        let input = five_exposure_input();
        let result = calculate_portfolio_risk(&input).unwrap();
        assert!(
            result.result.diversification_benefit < Decimal::ONE,
            "Diversification benefit {} should be less than 1",
            result.result.diversification_benefit
        );
    }

    #[test]
    fn test_five_exposure_equal_weights() {
        let input = five_exposure_input();
        let result = calculate_portfolio_risk(&input).unwrap();
        for risk in &result.result.exposure_risks {
            assert!(
                approx_eq(risk.pct_of_portfolio, dec!(20), dec!(0.1)),
                "Each exposure should be 20%, got {}",
                risk.pct_of_portfolio
            );
        }
    }

    // -----------------------------------------------------------------------
    // Equal-weight portfolio
    // -----------------------------------------------------------------------

    #[test]
    fn test_equal_weight_hhi() {
        let input = equal_weight_input();
        let result = calculate_portfolio_risk(&input).unwrap();
        // 4 equal weights of 0.25 => HHI = 4 * 0.0625 = 0.25
        assert!(
            approx_eq(
                result.result.concentration.hhi_name,
                dec!(0.25),
                dec!(0.001)
            ),
            "HHI = {} expected 0.25",
            result.result.concentration.hhi_name
        );
    }

    #[test]
    fn test_equal_weight_effective_number() {
        let input = equal_weight_input();
        let result = calculate_portfolio_risk(&input).unwrap();
        // effective_number = 1/0.25 = 4
        assert!(
            approx_eq(
                result.result.concentration.effective_number_names,
                dec!(4),
                dec!(0.1)
            ),
            "Effective number = {} expected 4",
            result.result.concentration.effective_number_names
        );
    }

    // -----------------------------------------------------------------------
    // Concentrated portfolio
    // -----------------------------------------------------------------------

    #[test]
    fn test_concentrated_hhi_high() {
        let input = concentrated_input();
        let result = calculate_portfolio_risk(&input).unwrap();
        // 90% + 10% => HHI = 0.81 + 0.01 = 0.82
        assert!(
            result.result.concentration.hhi_name > dec!(0.80),
            "Concentrated HHI {} should be > 0.80",
            result.result.concentration.hhi_name
        );
    }

    #[test]
    fn test_concentrated_warning() {
        let input = concentrated_input();
        let result = calculate_portfolio_risk(&input).unwrap();
        let has_conc_warning = result
            .result
            .warnings
            .iter()
            .any(|w| w.contains("concentrated"));
        assert!(has_conc_warning, "Should warn about concentrated portfolio");
    }

    #[test]
    fn test_concentrated_largest_pct() {
        let input = concentrated_input();
        let result = calculate_portfolio_risk(&input).unwrap();
        assert!(
            approx_eq(
                result.result.concentration.largest_exposure_pct,
                dec!(90),
                dec!(1)
            ),
            "Largest exposure = {}% expected ~90%",
            result.result.concentration.largest_exposure_pct
        );
    }

    #[test]
    fn test_concentrated_single_name_warning() {
        let input = concentrated_input();
        let result = calculate_portfolio_risk(&input).unwrap();
        let has_name_warning = result
            .result
            .warnings
            .iter()
            .any(|w| w.contains("Big") && w.contains(">10%"));
        assert!(
            has_name_warning,
            "Should warn about 'Big' exceeding 10% of total"
        );
    }

    // -----------------------------------------------------------------------
    // Zero correlation (independent defaults)
    // -----------------------------------------------------------------------

    #[test]
    fn test_zero_correlation_diversification() {
        let mut input = five_exposure_input();
        input.default_correlation = dec!(0);
        let result = calculate_portfolio_risk(&input).unwrap();
        // With zero correlation, diversification benefit should be high
        assert!(
            result.result.diversification_benefit > dec!(0.3),
            "Zero-corr diversification benefit {} should be > 0.3",
            result.result.diversification_benefit
        );
    }

    #[test]
    fn test_zero_correlation_var_equals_el() {
        // With zero correlation, conditional PD = PD, so credit VaR ~ EL
        let mut input = five_exposure_input();
        input.default_correlation = dec!(0);
        let result = calculate_portfolio_risk(&input).unwrap();
        // Credit VaR with rho=0 should just be sum(EAD*LGD*PD) = EL
        assert!(
            approx_eq(
                result.result.credit_var,
                result.result.expected_loss,
                dec!(1)
            ),
            "Zero-corr VaR {} should ~ EL {}",
            result.result.credit_var,
            result.result.expected_loss
        );
    }

    // -----------------------------------------------------------------------
    // High correlation
    // -----------------------------------------------------------------------

    #[test]
    fn test_high_correlation_less_diversification() {
        let mut low_corr = five_exposure_input();
        low_corr.default_correlation = dec!(0.1);
        let mut high_corr = five_exposure_input();
        high_corr.default_correlation = dec!(0.5);

        let low_result = calculate_portfolio_risk(&low_corr).unwrap();
        let high_result = calculate_portfolio_risk(&high_corr).unwrap();

        assert!(
            high_result.result.diversification_benefit < low_result.result.diversification_benefit,
            "High corr div benefit {} should be < low corr {}",
            high_result.result.diversification_benefit,
            low_result.result.diversification_benefit
        );
    }

    #[test]
    fn test_high_correlation_higher_var() {
        let mut low_corr = five_exposure_input();
        low_corr.default_correlation = dec!(0.1);
        let mut high_corr = five_exposure_input();
        high_corr.default_correlation = dec!(0.5);

        let low_result = calculate_portfolio_risk(&low_corr).unwrap();
        let high_result = calculate_portfolio_risk(&high_corr).unwrap();

        assert!(
            high_result.result.credit_var > low_result.result.credit_var,
            "High corr VaR {} should > low corr VaR {}",
            high_result.result.credit_var,
            low_result.result.credit_var
        );
    }

    // -----------------------------------------------------------------------
    // PD edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_very_low_pd() {
        let input = PortfolioRiskInput {
            portfolio_name: "Low PD".into(),
            exposures: vec![make_exposure(
                "IG",
                dec!(1000000),
                dec!(0.001),
                dec!(0.45),
                "AA",
                "Sovereign",
            )],
            default_correlation: dec!(0.15),
            confidence_level: dec!(0.99),
            time_horizon_years: dec!(1),
        };
        let result = calculate_portfolio_risk(&input).unwrap();
        // Very low PD => very low EL
        assert!(
            result.result.expected_loss < dec!(500),
            "Low PD EL {} should be small",
            result.result.expected_loss
        );
    }

    #[test]
    fn test_very_high_pd() {
        let input = PortfolioRiskInput {
            portfolio_name: "High PD".into(),
            exposures: vec![make_exposure(
                "Distressed",
                dec!(1000000),
                dec!(0.20),
                dec!(0.60),
                "CCC",
                "Energy",
            )],
            default_correlation: dec!(0.2),
            confidence_level: dec!(0.99),
            time_horizon_years: dec!(1),
        };
        let result = calculate_portfolio_risk(&input).unwrap();
        // EL = 0.20 * 0.60 * 1M = 120_000
        assert!(
            approx_eq(result.result.expected_loss, dec!(120000), dec!(100)),
            "High PD EL {} expected ~120000",
            result.result.expected_loss
        );
    }

    // -----------------------------------------------------------------------
    // LGD variations
    // -----------------------------------------------------------------------

    #[test]
    fn test_lgd_secured_vs_equity() {
        // Secured (LGD=0.40) should have lower EL than equity (LGD=1.0)
        let secured = PortfolioRiskInput {
            portfolio_name: "Secured".into(),
            exposures: vec![make_exposure(
                "Secured",
                dec!(1000000),
                dec!(0.05),
                dec!(0.40),
                "BBB",
                "Industrials",
            )],
            default_correlation: dec!(0.2),
            confidence_level: dec!(0.99),
            time_horizon_years: dec!(1),
        };
        let equity = PortfolioRiskInput {
            portfolio_name: "Equity".into(),
            exposures: vec![make_exposure(
                "Equity",
                dec!(1000000),
                dec!(0.05),
                dec!(1.0),
                "BBB",
                "Industrials",
            )],
            default_correlation: dec!(0.2),
            confidence_level: dec!(0.99),
            time_horizon_years: dec!(1),
        };
        let sec_result = calculate_portfolio_risk(&secured).unwrap();
        let eq_result = calculate_portfolio_risk(&equity).unwrap();
        assert!(
            sec_result.result.expected_loss < eq_result.result.expected_loss,
            "Secured EL {} should be < equity EL {}",
            sec_result.result.expected_loss,
            eq_result.result.expected_loss
        );
    }

    // -----------------------------------------------------------------------
    // HHI and sector concentration
    // -----------------------------------------------------------------------

    #[test]
    fn test_sector_concentration_single_sector() {
        // All exposures in same sector => HHI_sector = 1.0
        let input = PortfolioRiskInput {
            portfolio_name: "Single sector".into(),
            exposures: vec![
                make_exposure("A", dec!(500000), dec!(0.02), dec!(0.45), "BBB", "Tech"),
                make_exposure("B", dec!(500000), dec!(0.02), dec!(0.45), "BBB", "Tech"),
            ],
            default_correlation: dec!(0.2),
            confidence_level: dec!(0.99),
            time_horizon_years: dec!(1),
        };
        let result = calculate_portfolio_risk(&input).unwrap();
        assert!(
            approx_eq(
                result.result.concentration.hhi_sector,
                dec!(1.0),
                dec!(0.001)
            ),
            "Single sector HHI = {} expected 1.0",
            result.result.concentration.hhi_sector
        );
    }

    #[test]
    fn test_sector_concentration_diverse() {
        let input = five_exposure_input();
        let result = calculate_portfolio_risk(&input).unwrap();
        // 5 different sectors, equal weight => HHI_sector = 5 * 0.04 = 0.20
        assert!(
            approx_eq(
                result.result.concentration.hhi_sector,
                dec!(0.20),
                dec!(0.01)
            ),
            "Diverse sector HHI = {} expected ~0.20",
            result.result.concentration.hhi_sector
        );
    }

    #[test]
    fn test_top_10_pct_with_few_exposures() {
        // Fewer than 10 exposures => top 10 = 100%
        let input = five_exposure_input();
        let result = calculate_portfolio_risk(&input).unwrap();
        assert!(
            approx_eq(result.result.concentration.top_10_pct, dec!(100), dec!(0.1)),
            "Top 10 pct = {} expected 100%",
            result.result.concentration.top_10_pct
        );
    }

    // -----------------------------------------------------------------------
    // Granularity adjustment
    // -----------------------------------------------------------------------

    #[test]
    fn test_granularity_adjustment_positive() {
        let input = five_exposure_input();
        let result = calculate_portfolio_risk(&input).unwrap();
        assert!(
            result.result.concentration.granularity_adjustment > Decimal::ZERO,
            "Granularity adjustment should be positive"
        );
    }

    // -----------------------------------------------------------------------
    // Risk contribution
    // -----------------------------------------------------------------------

    #[test]
    fn test_risk_contributions_all_positive() {
        let input = five_exposure_input();
        let result = calculate_portfolio_risk(&input).unwrap();
        for risk in &result.result.exposure_risks {
            assert!(
                risk.risk_contribution >= Decimal::ZERO,
                "Risk contribution for {} should be non-negative: {}",
                risk.name,
                risk.risk_contribution
            );
        }
    }

    #[test]
    fn test_marginal_risk_ordering() {
        // Exposures with higher PD should generally have higher marginal risk
        // (when LGD and EAD are similar)
        let input = PortfolioRiskInput {
            portfolio_name: "Ordered".into(),
            exposures: vec![
                make_exposure("Low PD", dec!(100000), dec!(0.01), dec!(0.45), "A", "Tech"),
                make_exposure("Med PD", dec!(100000), dec!(0.05), dec!(0.45), "BB", "Tech"),
                make_exposure(
                    "High PD",
                    dec!(100000),
                    dec!(0.10),
                    dec!(0.45),
                    "CCC",
                    "Tech",
                ),
            ],
            default_correlation: dec!(0.2),
            confidence_level: dec!(0.99),
            time_horizon_years: dec!(1),
        };
        let result = calculate_portfolio_risk(&input).unwrap();
        let risks = &result.result.exposure_risks;
        assert!(
            risks[0].marginal_risk < risks[1].marginal_risk,
            "Low PD marginal {} should be < Med PD {}",
            risks[0].marginal_risk,
            risks[1].marginal_risk
        );
        assert!(
            risks[1].marginal_risk < risks[2].marginal_risk,
            "Med PD marginal {} should be < High PD {}",
            risks[1].marginal_risk,
            risks[2].marginal_risk
        );
    }

    // -----------------------------------------------------------------------
    // Validation errors
    // -----------------------------------------------------------------------

    #[test]
    fn test_empty_portfolio_error() {
        let input = PortfolioRiskInput {
            portfolio_name: "Empty".into(),
            exposures: vec![],
            default_correlation: dec!(0.2),
            confidence_level: dec!(0.99),
            time_horizon_years: dec!(1),
        };
        let result = calculate_portfolio_risk(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_negative_exposure_error() {
        let input = PortfolioRiskInput {
            portfolio_name: "Bad".into(),
            exposures: vec![make_exposure(
                "Neg",
                dec!(-1000),
                dec!(0.02),
                dec!(0.45),
                "BBB",
                "Tech",
            )],
            default_correlation: dec!(0.2),
            confidence_level: dec!(0.99),
            time_horizon_years: dec!(1),
        };
        let result = calculate_portfolio_risk(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert!(field.contains("exposure"));
            }
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }

    #[test]
    fn test_pd_above_one_error() {
        let input = PortfolioRiskInput {
            portfolio_name: "Bad PD".into(),
            exposures: vec![make_exposure(
                "Over",
                dec!(1000000),
                dec!(1.5),
                dec!(0.45),
                "BBB",
                "Tech",
            )],
            default_correlation: dec!(0.2),
            confidence_level: dec!(0.99),
            time_horizon_years: dec!(1),
        };
        let result = calculate_portfolio_risk(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_lgd_above_one_error() {
        let input = PortfolioRiskInput {
            portfolio_name: "Bad LGD".into(),
            exposures: vec![make_exposure(
                "Over",
                dec!(1000000),
                dec!(0.02),
                dec!(1.1),
                "BBB",
                "Tech",
            )],
            default_correlation: dec!(0.2),
            confidence_level: dec!(0.99),
            time_horizon_years: dec!(1),
        };
        let result = calculate_portfolio_risk(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_correlation_out_of_range_error() {
        let mut input = single_exposure_input();
        input.default_correlation = dec!(1.0);
        let result = calculate_portfolio_risk(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_correlation_negative_error() {
        let mut input = single_exposure_input();
        input.default_correlation = dec!(-0.1);
        let result = calculate_portfolio_risk(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_confidence_low_error() {
        let mut input = single_exposure_input();
        input.confidence_level = dec!(0.3);
        let result = calculate_portfolio_risk(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_confidence_one_error() {
        let mut input = single_exposure_input();
        input.confidence_level = dec!(1.0);
        let result = calculate_portfolio_risk(&input);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Confidence level impact
    // -----------------------------------------------------------------------

    #[test]
    fn test_99_vs_95_confidence() {
        let mut input_99 = five_exposure_input();
        input_99.confidence_level = dec!(0.99);
        let mut input_95 = five_exposure_input();
        input_95.confidence_level = dec!(0.95);

        let result_99 = calculate_portfolio_risk(&input_99).unwrap();
        let result_95 = calculate_portfolio_risk(&input_95).unwrap();

        assert!(
            result_99.result.credit_var > result_95.result.credit_var,
            "99% VaR {} should exceed 95% VaR {}",
            result_99.result.credit_var,
            result_95.result.credit_var
        );
    }

    // -----------------------------------------------------------------------
    // Metadata
    // -----------------------------------------------------------------------

    #[test]
    fn test_metadata_populated() {
        let input = single_exposure_input();
        let result = calculate_portfolio_risk(&input).unwrap();
        assert!(!result.methodology.is_empty());
        assert!(!result.metadata.version.is_empty());
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
    }

    // -----------------------------------------------------------------------
    // Portfolio-level weighted averages
    // -----------------------------------------------------------------------

    #[test]
    fn test_portfolio_weighted_pd() {
        let input = equal_weight_input();
        let result = calculate_portfolio_risk(&input).unwrap();
        // All PDs are 0.02, so weighted avg should be 0.02
        assert!(
            approx_eq(result.result.portfolio_pd, dec!(0.02), dec!(0.001)),
            "Portfolio PD {} expected 0.02",
            result.result.portfolio_pd
        );
    }

    #[test]
    fn test_portfolio_weighted_lgd() {
        let input = equal_weight_input();
        let result = calculate_portfolio_risk(&input).unwrap();
        assert!(
            approx_eq(result.result.portfolio_lgd, dec!(0.45), dec!(0.001)),
            "Portfolio LGD {} expected 0.45",
            result.result.portfolio_lgd
        );
    }

    // -----------------------------------------------------------------------
    // Percentage outputs
    // -----------------------------------------------------------------------

    #[test]
    fn test_el_pct_consistent() {
        let input = single_exposure_input();
        let result = calculate_portfolio_risk(&input).unwrap();
        let expected_pct = result.result.expected_loss / result.result.total_exposure * dec!(100);
        assert!(
            approx_eq(result.result.expected_loss_pct, expected_pct, dec!(0.01)),
            "EL pct {} should match computed {}",
            result.result.expected_loss_pct,
            expected_pct
        );
    }

    #[test]
    fn test_var_pct_consistent() {
        let input = single_exposure_input();
        let result = calculate_portfolio_risk(&input).unwrap();
        let expected_pct = result.result.credit_var / result.result.total_exposure * dec!(100);
        assert!(
            approx_eq(result.result.credit_var_pct, expected_pct, dec!(0.01)),
            "VaR pct {} should match computed {}",
            result.result.credit_var_pct,
            expected_pct
        );
    }
}
