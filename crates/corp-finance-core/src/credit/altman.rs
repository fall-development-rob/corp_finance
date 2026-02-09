use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::{types::*, CorpFinanceError, CorpFinanceResult};

// ---------------------------------------------------------------------------
// Input / Output types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AltmanInput {
    // Required for all variants
    pub working_capital: Money,
    pub total_assets: Money,
    pub retained_earnings: Money,
    pub ebit: Money,
    pub revenue: Money,
    pub total_liabilities: Money,
    // Required for original Z (public companies)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub market_cap: Option<Money>,
    // Required for Z' and Z'' (private companies)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub book_equity: Option<Money>,
    // Flag to select which model(s) to compute
    pub is_public: bool,
    pub is_manufacturing: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ZScoreZone {
    Safe,
    Grey,
    Distress,
}

impl std::fmt::Display for ZScoreZone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Safe => write!(f, "Safe"),
            Self::Grey => write!(f, "Grey Zone"),
            Self::Distress => write!(f, "Distress"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZScoreResult {
    pub model: String,
    pub score: Decimal,
    pub zone: ZScoreZone,
    pub components: Vec<ZScoreComponent>,
    /// (distress_upper_bound, safe_lower_bound)
    pub zone_thresholds: (Decimal, Decimal),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZScoreComponent {
    pub name: String,
    pub ratio: Decimal,
    pub coefficient: Decimal,
    pub weighted_value: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AltmanOutput {
    pub scores: Vec<ZScoreResult>,
    pub primary_score: Decimal,
    pub primary_zone: ZScoreZone,
    pub primary_model: String,
    /// Rough probability-of-default estimate mapped from the zone.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub probability_of_default_estimate: Option<Rate>,
}

// ---------------------------------------------------------------------------
// Coefficients
// ---------------------------------------------------------------------------

// Original Z-Score (public manufacturing)
const Z_COEFF_X1: Decimal = dec!(1.2);
const Z_COEFF_X2: Decimal = dec!(1.4);
const Z_COEFF_X3: Decimal = dec!(3.3);
const Z_COEFF_X4: Decimal = dec!(0.6);
const Z_COEFF_X5: Decimal = dec!(1.0);

// Z'-Score (private companies)
const ZP_COEFF_X1: Decimal = dec!(0.717);
const ZP_COEFF_X2: Decimal = dec!(0.847);
const ZP_COEFF_X3: Decimal = dec!(3.107);
const ZP_COEFF_X4: Decimal = dec!(0.420);
const ZP_COEFF_X5: Decimal = dec!(0.998);

// Z''-Score (non-manufacturing / emerging markets)
const ZPP_COEFF_X1: Decimal = dec!(6.56);
const ZPP_COEFF_X2: Decimal = dec!(3.26);
const ZPP_COEFF_X3: Decimal = dec!(6.72);
const ZPP_COEFF_X4: Decimal = dec!(1.05);

// Zone thresholds
const Z_SAFE: Decimal = dec!(2.99);
const Z_DISTRESS: Decimal = dec!(1.81);

const ZP_SAFE: Decimal = dec!(2.90);
const ZP_DISTRESS: Decimal = dec!(1.23);

const ZPP_SAFE: Decimal = dec!(2.60);
const ZPP_DISTRESS: Decimal = dec!(1.10);

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Compute the Altman Z-Score(s) for bankruptcy prediction.
///
/// Depending on the `is_public` and `is_manufacturing` flags the function
/// computes up to three model variants (original Z, Z', Z'') and selects
/// the most appropriate one as the primary model.
pub fn calculate_altman_zscore(
    input: &AltmanInput,
) -> CorpFinanceResult<ComputationOutput<AltmanOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // -- Validation ----------------------------------------------------------
    validate_input(input)?;

    // -- Component ratios ----------------------------------------------------
    let x1 = safe_divide(
        input.working_capital,
        input.total_assets,
        "X1: WC / Total Assets",
    )?;
    let x2 = safe_divide(
        input.retained_earnings,
        input.total_assets,
        "X2: Retained Earnings / Total Assets",
    )?;
    let x3 = safe_divide(input.ebit, input.total_assets, "X3: EBIT / Total Assets")?;
    let x5 = safe_divide(
        input.revenue,
        input.total_assets,
        "X5: Revenue / Total Assets",
    )?;

    let mut scores: Vec<ZScoreResult> = Vec::new();

    // -- Original Z-Score (public manufacturing) -----------------------------
    if input.is_public {
        match input.market_cap {
            Some(mc) => {
                let x4 = safe_divide(
                    mc,
                    input.total_liabilities,
                    "X4: Market Cap / Total Liabilities",
                )?;

                let components = vec![
                    build_component("X1: Working Capital / Total Assets", x1, Z_COEFF_X1),
                    build_component("X2: Retained Earnings / Total Assets", x2, Z_COEFF_X2),
                    build_component("X3: EBIT / Total Assets", x3, Z_COEFF_X3),
                    build_component("X4: Market Cap / Total Liabilities", x4, Z_COEFF_X4),
                    build_component("X5: Revenue / Total Assets", x5, Z_COEFF_X5),
                ];

                let score = Z_COEFF_X1 * x1
                    + Z_COEFF_X2 * x2
                    + Z_COEFF_X3 * x3
                    + Z_COEFF_X4 * x4
                    + Z_COEFF_X5 * x5;

                let zone = classify_zone(score, Z_DISTRESS, Z_SAFE);

                scores.push(ZScoreResult {
                    model: "Original Z-Score".to_string(),
                    score,
                    zone,
                    components,
                    zone_thresholds: (Z_DISTRESS, Z_SAFE),
                });
            }
            None => {
                warnings.push(
                    "market_cap is required for the original Z-Score but was not provided."
                        .to_string(),
                );
            }
        }
    }

    // -- Z'-Score (private companies) ----------------------------------------
    if let Some(be) = input.book_equity {
        let x4_prime = safe_divide(
            be,
            input.total_liabilities,
            "X4': Book Equity / Total Liabilities",
        )?;

        let components = vec![
            build_component("X1: Working Capital / Total Assets", x1, ZP_COEFF_X1),
            build_component("X2: Retained Earnings / Total Assets", x2, ZP_COEFF_X2),
            build_component("X3: EBIT / Total Assets", x3, ZP_COEFF_X3),
            build_component(
                "X4': Book Equity / Total Liabilities",
                x4_prime,
                ZP_COEFF_X4,
            ),
            build_component("X5: Revenue / Total Assets", x5, ZP_COEFF_X5),
        ];

        let score = ZP_COEFF_X1 * x1
            + ZP_COEFF_X2 * x2
            + ZP_COEFF_X3 * x3
            + ZP_COEFF_X4 * x4_prime
            + ZP_COEFF_X5 * x5;

        let zone = classify_zone(score, ZP_DISTRESS, ZP_SAFE);

        scores.push(ZScoreResult {
            model: "Z'-Score (Private)".to_string(),
            score,
            zone,
            components,
            zone_thresholds: (ZP_DISTRESS, ZP_SAFE),
        });
    }

    // -- Z''-Score (non-manufacturing / emerging markets) --------------------
    if !input.is_manufacturing {
        if let Some(be) = input.book_equity {
            let x4_prime = safe_divide(
                be,
                input.total_liabilities,
                "X4': Book Equity / Total Liabilities",
            )?;

            let components = vec![
                build_component("X1: Working Capital / Total Assets", x1, ZPP_COEFF_X1),
                build_component("X2: Retained Earnings / Total Assets", x2, ZPP_COEFF_X2),
                build_component("X3: EBIT / Total Assets", x3, ZPP_COEFF_X3),
                build_component(
                    "X4': Book Equity / Total Liabilities",
                    x4_prime,
                    ZPP_COEFF_X4,
                ),
            ];

            let score =
                ZPP_COEFF_X1 * x1 + ZPP_COEFF_X2 * x2 + ZPP_COEFF_X3 * x3 + ZPP_COEFF_X4 * x4_prime;

            let zone = classify_zone(score, ZPP_DISTRESS, ZPP_SAFE);

            scores.push(ZScoreResult {
                model: "Z''-Score (Non-Manufacturing)".to_string(),
                score,
                zone,
                components,
                zone_thresholds: (ZPP_DISTRESS, ZPP_SAFE),
            });
        } else {
            warnings.push("book_equity is required for Z'' but was not provided.".to_string());
        }
    }

    // -- Primary model selection ---------------------------------------------
    if scores.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "No Z-Score variant could be computed. Provide market_cap (public) or \
             book_equity (private)."
                .to_string(),
        ));
    }

    let primary = select_primary(&scores, input);

    let pd_estimate = estimate_pd(&primary.zone);

    let output = AltmanOutput {
        primary_score: primary.score,
        primary_zone: primary.zone.clone(),
        primary_model: primary.model.clone(),
        probability_of_default_estimate: Some(pd_estimate),
        scores,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    let assumptions = serde_json::json!({
        "methodology": "Altman Z-Score bankruptcy prediction",
        "original_z": "Z = 1.2*X1 + 1.4*X2 + 3.3*X3 + 0.6*X4 + 1.0*X5",
        "z_prime": "Z' = 0.717*X1 + 0.847*X2 + 3.107*X3 + 0.420*X4' + 0.998*X5",
        "z_double_prime": "Z'' = 6.56*X1 + 3.26*X2 + 6.72*X3 + 1.05*X4'",
        "pd_mapping": "rough heuristic, not calibrated"
    });

    Ok(with_metadata(
        "Altman Z-Score (CFA Level II credit analysis)",
        &assumptions,
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn validate_input(input: &AltmanInput) -> CorpFinanceResult<()> {
    if input.total_assets <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "total_assets".into(),
            reason: "Total assets must be positive.".into(),
        });
    }
    if input.total_liabilities <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "total_liabilities".into(),
            reason: "Total liabilities must be positive.".into(),
        });
    }
    Ok(())
}

fn safe_divide(
    numerator: Decimal,
    denominator: Decimal,
    context: &str,
) -> CorpFinanceResult<Decimal> {
    if denominator.is_zero() {
        return Err(CorpFinanceError::DivisionByZero {
            context: context.to_string(),
        });
    }
    Ok(numerator / denominator)
}

fn classify_zone(score: Decimal, distress_upper: Decimal, safe_lower: Decimal) -> ZScoreZone {
    if score > safe_lower {
        ZScoreZone::Safe
    } else if score < distress_upper {
        ZScoreZone::Distress
    } else {
        ZScoreZone::Grey
    }
}

fn build_component(name: &str, ratio: Decimal, coefficient: Decimal) -> ZScoreComponent {
    ZScoreComponent {
        name: name.to_string(),
        ratio,
        coefficient,
        weighted_value: coefficient * ratio,
    }
}

/// Select the most appropriate model as primary based on the company flags.
///
/// - Public companies: original Z-Score
/// - Private manufacturing: Z'-Score
/// - Private non-manufacturing: Z''-Score
fn select_primary<'a>(scores: &'a [ZScoreResult], input: &AltmanInput) -> &'a ZScoreResult {
    let target = if input.is_public {
        "Original Z-Score"
    } else if input.is_manufacturing {
        "Z'-Score (Private)"
    } else {
        "Z''-Score (Non-Manufacturing)"
    };

    scores
        .iter()
        .find(|s| s.model == target)
        .unwrap_or(&scores[0])
}

/// Map Z-Score zone to a rough probability of default estimate.
///
/// These are heuristic midpoints, not calibrated default probabilities.
fn estimate_pd(zone: &ZScoreZone) -> Rate {
    match zone {
        ZScoreZone::Safe => dec!(0.03),     // ~3%
        ZScoreZone::Grey => dec!(0.22),     // ~22%
        ZScoreZone::Distress => dec!(0.65), // ~65%
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// A strong public manufacturing company with healthy financials.
    fn strong_public_input() -> AltmanInput {
        AltmanInput {
            working_capital: dec!(500_000),
            total_assets: dec!(2_000_000),
            retained_earnings: dec!(600_000),
            ebit: dec!(400_000),
            revenue: dec!(3_000_000),
            total_liabilities: dec!(800_000),
            market_cap: Some(dec!(2_500_000)),
            book_equity: Some(dec!(1_200_000)),
            is_public: true,
            is_manufacturing: true,
        }
    }

    /// A distressed company with poor financials.
    fn distressed_input() -> AltmanInput {
        AltmanInput {
            working_capital: dec!(-100_000),
            total_assets: dec!(1_000_000),
            retained_earnings: dec!(-200_000),
            ebit: dec!(10_000),
            revenue: dec!(400_000),
            total_liabilities: dec!(900_000),
            market_cap: Some(dec!(50_000)),
            book_equity: Some(dec!(100_000)),
            is_public: true,
            is_manufacturing: true,
        }
    }

    /// A marginal company in the grey zone.
    fn grey_zone_input() -> AltmanInput {
        // Target: 1.81 <= Z <= 2.99
        // X1 = 150k/1M = 0.15, X2 = 150k/1M = 0.15, X3 = 100k/1M = 0.1,
        // X4 = 500k/600k = 0.8333, X5 = 900k/1M = 0.9
        // Z = 1.2(0.15) + 1.4(0.15) + 3.3(0.1) + 0.6(0.8333) + 1.0(0.9)
        //   = 0.18 + 0.21 + 0.33 + 0.5 + 0.9 = 2.12
        AltmanInput {
            working_capital: dec!(150_000),
            total_assets: dec!(1_000_000),
            retained_earnings: dec!(150_000),
            ebit: dec!(100_000),
            revenue: dec!(900_000),
            total_liabilities: dec!(600_000),
            market_cap: Some(dec!(500_000)),
            book_equity: Some(dec!(400_000)),
            is_public: true,
            is_manufacturing: true,
        }
    }

    #[test]
    fn test_safe_zone_public() {
        let input = strong_public_input();
        let result = calculate_altman_zscore(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.primary_model, "Original Z-Score");
        assert_eq!(out.primary_zone, ZScoreZone::Safe);
        assert!(
            out.primary_score > dec!(2.99),
            "Expected Z > 2.99, got {}",
            out.primary_score
        );
    }

    #[test]
    fn test_distress_zone() {
        let input = distressed_input();
        let result = calculate_altman_zscore(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.primary_zone, ZScoreZone::Distress);
        assert!(
            out.primary_score < dec!(1.81),
            "Expected Z < 1.81, got {}",
            out.primary_score
        );
    }

    #[test]
    fn test_grey_zone() {
        let input = grey_zone_input();
        let result = calculate_altman_zscore(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.primary_zone, ZScoreZone::Grey);
        assert!(
            out.primary_score >= dec!(1.81) && out.primary_score <= dec!(2.99),
            "Expected 1.81 <= Z <= 2.99, got {}",
            out.primary_score
        );
    }

    #[test]
    fn test_private_company_zprime() {
        let input = AltmanInput {
            working_capital: dec!(300_000),
            total_assets: dec!(1_500_000),
            retained_earnings: dec!(400_000),
            ebit: dec!(250_000),
            revenue: dec!(2_000_000),
            total_liabilities: dec!(700_000),
            market_cap: None,
            book_equity: Some(dec!(800_000)),
            is_public: false,
            is_manufacturing: true,
        };
        let result = calculate_altman_zscore(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.primary_model, "Z'-Score (Private)");
        // Verify only Z' is computed (no original Z, no Z'')
        assert_eq!(out.scores.len(), 1);
        assert_eq!(out.scores[0].model, "Z'-Score (Private)");
    }

    #[test]
    fn test_non_manufacturing_zdouble_prime() {
        let input = AltmanInput {
            working_capital: dec!(200_000),
            total_assets: dec!(1_000_000),
            retained_earnings: dec!(300_000),
            ebit: dec!(150_000),
            revenue: dec!(1_200_000),
            total_liabilities: dec!(500_000),
            market_cap: None,
            book_equity: Some(dec!(500_000)),
            is_public: false,
            is_manufacturing: false,
        };
        let result = calculate_altman_zscore(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.primary_model, "Z''-Score (Non-Manufacturing)");
        // Both Z' and Z'' should be computed for private non-manufacturing
        assert_eq!(out.scores.len(), 2);

        // Z'' excludes X5 (revenue/total_assets)
        let zpp = out
            .scores
            .iter()
            .find(|s| s.model.contains("Non-Manufacturing"))
            .unwrap();
        assert_eq!(
            zpp.components.len(),
            4,
            "Z'' should have 4 components (no X5)"
        );
    }

    #[test]
    fn test_component_calculation() {
        let input = AltmanInput {
            working_capital: dec!(200_000),
            total_assets: dec!(1_000_000),
            retained_earnings: dec!(300_000),
            ebit: dec!(100_000),
            revenue: dec!(1_500_000),
            total_liabilities: dec!(500_000),
            market_cap: Some(dec!(1_000_000)),
            book_equity: Some(dec!(500_000)),
            is_public: true,
            is_manufacturing: true,
        };
        let result = calculate_altman_zscore(&input).unwrap();
        let out = &result.result;

        let z_original = out
            .scores
            .iter()
            .find(|s| s.model == "Original Z-Score")
            .unwrap();

        // X1 = 200k / 1M = 0.2
        assert_eq!(z_original.components[0].ratio, dec!(0.2));
        // X2 = 300k / 1M = 0.3
        assert_eq!(z_original.components[1].ratio, dec!(0.3));
        // X3 = 100k / 1M = 0.1
        assert_eq!(z_original.components[2].ratio, dec!(0.1));
        // X4 = 1M / 500k = 2.0
        assert_eq!(z_original.components[3].ratio, dec!(2));
        // X5 = 1.5M / 1M = 1.5
        assert_eq!(z_original.components[4].ratio, dec!(1.5));
    }

    #[test]
    fn test_coefficients_correct() {
        let input = AltmanInput {
            working_capital: dec!(200_000),
            total_assets: dec!(1_000_000),
            retained_earnings: dec!(300_000),
            ebit: dec!(100_000),
            revenue: dec!(1_500_000),
            total_liabilities: dec!(500_000),
            market_cap: Some(dec!(1_000_000)),
            book_equity: Some(dec!(500_000)),
            is_public: true,
            is_manufacturing: true,
        };
        let result = calculate_altman_zscore(&input).unwrap();
        let out = &result.result;

        let z_original = out
            .scores
            .iter()
            .find(|s| s.model == "Original Z-Score")
            .unwrap();

        // Verify weighted values = coefficient * ratio
        // X1: 1.2 * 0.2 = 0.24
        assert_eq!(z_original.components[0].coefficient, dec!(1.2));
        assert_eq!(z_original.components[0].weighted_value, dec!(0.24));
        // X2: 1.4 * 0.3 = 0.42
        assert_eq!(z_original.components[1].coefficient, dec!(1.4));
        assert_eq!(z_original.components[1].weighted_value, dec!(0.42));
        // X3: 3.3 * 0.1 = 0.33
        assert_eq!(z_original.components[2].coefficient, dec!(3.3));
        assert_eq!(z_original.components[2].weighted_value, dec!(0.33));
        // X4: 0.6 * 2.0 = 1.2
        assert_eq!(z_original.components[3].coefficient, dec!(0.6));
        assert_eq!(z_original.components[3].weighted_value, dec!(1.2));
        // X5: 1.0 * 1.5 = 1.5
        assert_eq!(z_original.components[4].coefficient, dec!(1.0));
        assert_eq!(z_original.components[4].weighted_value, dec!(1.5));

        // Total Z = 0.24 + 0.42 + 0.33 + 1.2 + 1.5 = 3.69
        let expected_z = dec!(0.24) + dec!(0.42) + dec!(0.33) + dec!(1.2) + dec!(1.5);
        assert_eq!(z_original.score, expected_z);
        assert_eq!(z_original.score, dec!(3.69));
    }

    #[test]
    fn test_zero_total_assets_error() {
        let input = AltmanInput {
            working_capital: dec!(100_000),
            total_assets: Decimal::ZERO,
            retained_earnings: dec!(50_000),
            ebit: dec!(20_000),
            revenue: dec!(500_000),
            total_liabilities: dec!(300_000),
            market_cap: Some(dec!(400_000)),
            book_equity: None,
            is_public: true,
            is_manufacturing: true,
        };
        let err = calculate_altman_zscore(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "total_assets");
            }
            other => panic!("Expected InvalidInput for total_assets, got {other:?}"),
        }
    }

    #[test]
    fn test_all_three_models() {
        // Public manufacturing with both market_cap and book_equity gets all 3
        let input = strong_public_input();
        let result = calculate_altman_zscore(&input).unwrap();
        let out = &result.result;

        // Should NOT have Z'' because is_manufacturing = true
        assert_eq!(out.scores.len(), 2);
        assert!(out.scores.iter().any(|s| s.model == "Original Z-Score"));
        assert!(out.scores.iter().any(|s| s.model == "Z'-Score (Private)"));

        // Now test with is_manufacturing = false to get all 3
        let mut input_nm = strong_public_input();
        input_nm.is_manufacturing = false;
        let result_nm = calculate_altman_zscore(&input_nm).unwrap();
        let out_nm = &result_nm.result;

        assert_eq!(out_nm.scores.len(), 3);
        assert!(out_nm.scores.iter().any(|s| s.model == "Original Z-Score"));
        assert!(out_nm
            .scores
            .iter()
            .any(|s| s.model == "Z'-Score (Private)"));
        assert!(out_nm
            .scores
            .iter()
            .any(|s| s.model.contains("Non-Manufacturing")));
    }

    #[test]
    fn test_zero_total_liabilities_error() {
        let input = AltmanInput {
            working_capital: dec!(100_000),
            total_assets: dec!(1_000_000),
            retained_earnings: dec!(50_000),
            ebit: dec!(20_000),
            revenue: dec!(500_000),
            total_liabilities: Decimal::ZERO,
            market_cap: Some(dec!(400_000)),
            book_equity: None,
            is_public: true,
            is_manufacturing: true,
        };
        let err = calculate_altman_zscore(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "total_liabilities");
            }
            other => panic!("Expected InvalidInput for total_liabilities, got {other:?}"),
        }
    }

    #[test]
    fn test_pd_estimate_zones() {
        // Safe zone PD
        assert_eq!(estimate_pd(&ZScoreZone::Safe), dec!(0.03));
        // Grey zone PD
        assert_eq!(estimate_pd(&ZScoreZone::Grey), dec!(0.22));
        // Distress zone PD
        assert_eq!(estimate_pd(&ZScoreZone::Distress), dec!(0.65));
    }

    #[test]
    fn test_metadata_populated() {
        let input = strong_public_input();
        let result = calculate_altman_zscore(&input).unwrap();
        assert!(!result.methodology.is_empty());
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
        assert!(result.methodology.contains("Altman"));
    }

    #[test]
    fn test_negative_working_capital() {
        // A company can have negative working capital; the model should handle it
        let mut input = strong_public_input();
        input.working_capital = dec!(-100_000);
        let result = calculate_altman_zscore(&input).unwrap();
        let out = &result.result;

        // X1 will be negative, dragging the score down
        let z_original = out
            .scores
            .iter()
            .find(|s| s.model == "Original Z-Score")
            .unwrap();
        assert!(
            z_original.components[0].ratio < Decimal::ZERO,
            "X1 should be negative for negative working capital"
        );
    }

    #[test]
    fn test_missing_market_cap_for_public() {
        // Public company without market_cap should produce a warning and fall back
        let input = AltmanInput {
            working_capital: dec!(200_000),
            total_assets: dec!(1_000_000),
            retained_earnings: dec!(300_000),
            ebit: dec!(100_000),
            revenue: dec!(1_500_000),
            total_liabilities: dec!(500_000),
            market_cap: None,
            book_equity: Some(dec!(500_000)),
            is_public: true,
            is_manufacturing: true,
        };
        let result = calculate_altman_zscore(&input).unwrap();
        assert!(
            result.warnings.iter().any(|w| w.contains("market_cap")),
            "Should warn about missing market_cap"
        );
        // Should still compute Z' as fallback
        assert!(!result.result.scores.is_empty());
    }
}
