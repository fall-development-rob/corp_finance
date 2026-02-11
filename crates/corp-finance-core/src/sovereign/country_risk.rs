//! Country risk assessment and sovereign credit scoring module.
//!
//! Provides a multi-factor scoring framework for evaluating sovereign
//! creditworthiness across fiscal, external, monetary, political, and
//! structural dimensions. Maps composite scores to rating equivalents
//! and derives implied default probabilities.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Input parameters for country risk assessment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountryRiskInput {
    /// Country name
    pub country: String,
    /// Real GDP growth rate as decimal (0.03 = 3%)
    pub gdp_growth_rate: Decimal,
    /// CPI inflation rate as decimal (0.05 = 5%)
    pub inflation_rate: Decimal,
    /// Fiscal balance as % of GDP (negative = deficit, e.g. -0.04 = -4%)
    pub fiscal_balance_pct_gdp: Decimal,
    /// Government debt to GDP ratio (0.60 = 60%)
    pub debt_to_gdp: Decimal,
    /// Current account balance as % of GDP
    pub current_account_pct_gdp: Decimal,
    /// FX reserves in months of import cover
    pub fx_reserves_months_imports: Decimal,
    /// Political stability score 0-100
    pub political_stability_score: Decimal,
    /// Rule of law score 0-100
    pub rule_of_law_score: Decimal,
    /// Total external debt to GDP ratio
    pub external_debt_to_gdp: Decimal,
    /// Short-term external debt as % of reserves
    pub short_term_debt_to_reserves: Decimal,
    /// Whether the sovereign has defaulted in the past 20 years
    pub sovereign_default_history: bool,
    /// Percentage of economy using USD (optional, 0-100)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dollarization_pct: Option<Decimal>,
}

/// Component scores for country risk assessment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountryRiskComponents {
    /// Fiscal sustainability score (0-100)
    pub fiscal_score: Decimal,
    /// External vulnerability score (0-100)
    pub external_score: Decimal,
    /// Monetary stability score (0-100)
    pub monetary_score: Decimal,
    /// Political/institutional score (0-100)
    pub political_score: Decimal,
    /// Structural/growth score (0-100)
    pub structural_score: Decimal,
}

/// Output of country risk assessment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountryRiskOutput {
    /// Country risk premium in basis points
    pub country_risk_premium: Decimal,
    /// Composite sovereign credit score (0-100)
    pub sovereign_credit_score: Decimal,
    /// Rating equivalent on AAA to CC scale
    pub sovereign_rating_equivalent: String,
    /// Component scores breakdown
    pub component_scores: CountryRiskComponents,
    /// Risk category label
    pub risk_category: String,
    /// 5-year cumulative default probability
    pub implied_default_probability: Decimal,
    /// Risk assessment notes and recommendations
    pub recommendations: Vec<String>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Assess country risk based on macroeconomic, fiscal, external,
/// political, and structural indicators.
pub fn assess_country_risk(input: &CountryRiskInput) -> CorpFinanceResult<CountryRiskOutput> {
    validate_input(input)?;

    let mut recommendations: Vec<String> = Vec::new();

    // --- Component scores ---
    let fiscal_score = compute_fiscal_score(input, &mut recommendations);
    let external_score = compute_external_score(input, &mut recommendations);
    let monetary_score = compute_monetary_score(input, &mut recommendations);
    let political_score = compute_political_score(input, &mut recommendations);
    let structural_score = compute_structural_score(input, &mut recommendations);

    // --- Composite score (weighted) ---
    // Fiscal: 25%, External: 25%, Monetary: 15%, Political: 20%, Structural: 15%
    let composite = fiscal_score * dec!(0.25)
        + external_score * dec!(0.25)
        + monetary_score * dec!(0.15)
        + political_score * dec!(0.20)
        + structural_score * dec!(0.15);

    // Clamp to 0-100 range
    let composite = clamp(composite, Decimal::ZERO, dec!(100));

    // --- Rating mapping ---
    let rating = score_to_rating(composite);
    let risk_category = score_to_risk_category(composite);

    // --- Country risk premium (bps) ---
    let crp = score_to_crp(composite);

    // --- Implied default probability ---
    let implied_pd = compute_implied_pd(crp);

    let component_scores = CountryRiskComponents {
        fiscal_score,
        external_score,
        monetary_score,
        political_score,
        structural_score,
    };

    Ok(CountryRiskOutput {
        country_risk_premium: crp,
        sovereign_credit_score: composite,
        sovereign_rating_equivalent: rating,
        component_scores,
        risk_category,
        implied_default_probability: implied_pd,
        recommendations,
    })
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_input(input: &CountryRiskInput) -> CorpFinanceResult<()> {
    if input.country.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "country".into(),
            reason: "Country name is required".into(),
        });
    }
    if input.political_stability_score < Decimal::ZERO
        || input.political_stability_score > dec!(100)
    {
        return Err(CorpFinanceError::InvalidInput {
            field: "political_stability_score".into(),
            reason: "Must be between 0 and 100".into(),
        });
    }
    if input.rule_of_law_score < Decimal::ZERO || input.rule_of_law_score > dec!(100) {
        return Err(CorpFinanceError::InvalidInput {
            field: "rule_of_law_score".into(),
            reason: "Must be between 0 and 100".into(),
        });
    }
    if input.debt_to_gdp < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "debt_to_gdp".into(),
            reason: "Debt to GDP ratio cannot be negative".into(),
        });
    }
    if input.fx_reserves_months_imports < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "fx_reserves_months_imports".into(),
            reason: "FX reserves cannot be negative".into(),
        });
    }
    if input.short_term_debt_to_reserves < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "short_term_debt_to_reserves".into(),
            reason: "Short-term debt to reserves ratio cannot be negative".into(),
        });
    }
    if input.external_debt_to_gdp < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "external_debt_to_gdp".into(),
            reason: "External debt to GDP cannot be negative".into(),
        });
    }
    if let Some(dollar_pct) = input.dollarization_pct {
        if dollar_pct < Decimal::ZERO || dollar_pct > dec!(100) {
            return Err(CorpFinanceError::InvalidInput {
                field: "dollarization_pct".into(),
                reason: "Dollarization percentage must be between 0 and 100".into(),
            });
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Component score computations
// ---------------------------------------------------------------------------

/// Fiscal score: based on debt/GDP and fiscal balance.
///
/// Debt/GDP: 30% => 100, 120% => 0 (linear interpolation)
/// Fiscal balance: surplus (>=0) => 100, deficit >= 8% => 0
/// Combined: 50% debt_score + 50% balance_score
fn compute_fiscal_score(input: &CountryRiskInput, recommendations: &mut Vec<String>) -> Decimal {
    // Debt to GDP score: 30% = 100, 120% = 0
    let debt_pct = input.debt_to_gdp * dec!(100); // convert ratio to percentage
    let debt_score = linear_score(debt_pct, dec!(30), dec!(120), true);

    // Fiscal balance score: >= 0 => 100, <= -8% => 0
    let balance_pct = input.fiscal_balance_pct_gdp * dec!(100);
    let balance_score = linear_score(balance_pct, dec!(-8), dec!(0), false);

    let fiscal = debt_score * dec!(0.50) + balance_score * dec!(0.50);

    if input.debt_to_gdp > dec!(0.90) {
        recommendations.push(format!(
            "High debt/GDP ratio ({:.1}%) poses fiscal sustainability risk",
            debt_pct
        ));
    }
    if input.fiscal_balance_pct_gdp < dec!(-0.05) {
        recommendations.push(format!(
            "Large fiscal deficit ({:.1}% of GDP) requires fiscal consolidation",
            balance_pct
        ));
    }

    clamp(fiscal, Decimal::ZERO, dec!(100))
}

/// External score: based on current account, FX reserves, short-term debt/reserves.
///
/// Current account: surplus => 80-100, deficit >5% => 0-20
/// FX reserves: >12 months => 100, <3 months => 0
/// Short-term debt/reserves: <50% => 100, >200% => 0
/// Combined: 30% current_acct + 40% reserves + 30% st_debt
fn compute_external_score(input: &CountryRiskInput, recommendations: &mut Vec<String>) -> Decimal {
    // Current account score: +5% => 100, -10% => 0
    let ca_pct = input.current_account_pct_gdp * dec!(100);
    let ca_score = linear_score(ca_pct, dec!(-10), dec!(5), false);

    // FX reserves: 3 months => 0, 12 months => 100
    let reserves_score = linear_score(input.fx_reserves_months_imports, dec!(3), dec!(12), false);

    // Short-term debt / reserves: 50% => 100, 200% => 0
    let st_debt_pct = input.short_term_debt_to_reserves * dec!(100);
    let st_debt_score = linear_score(st_debt_pct, dec!(50), dec!(200), true);

    let external = ca_score * dec!(0.30) + reserves_score * dec!(0.40) + st_debt_score * dec!(0.30);

    if input.fx_reserves_months_imports < dec!(3) {
        recommendations
            .push("FX reserves below 3 months of imports - critical vulnerability".into());
    }
    if input.short_term_debt_to_reserves > dec!(1.50) {
        recommendations
            .push("Short-term external debt exceeds 150% of reserves - rollover risk".into());
    }
    if input.current_account_pct_gdp < dec!(-0.05) {
        recommendations
            .push("Large current account deficit increases external vulnerability".into());
    }

    clamp(external, Decimal::ZERO, dec!(100))
}

/// Monetary score: based on inflation and dollarization.
///
/// Inflation: <2% => 100, >20% => 0
/// Dollarization penalty: reduces score by up to 20 points
fn compute_monetary_score(input: &CountryRiskInput, recommendations: &mut Vec<String>) -> Decimal {
    // Inflation score: 2% => 100, 20% => 0
    let inflation_pct = input.inflation_rate * dec!(100);
    let inflation_score = linear_score(inflation_pct, dec!(2), dec!(20), true);

    // Dollarization penalty: high dollarization reduces monetary policy effectiveness
    let dollar_penalty = match input.dollarization_pct {
        Some(pct) => {
            // 0% => 0 penalty, 100% => 20 point penalty
            let penalty = pct / dec!(100) * dec!(20);
            if pct > dec!(50) {
                recommendations.push(format!(
                    "High dollarization ({:.0}%) limits monetary policy independence",
                    pct
                ));
            }
            penalty
        }
        None => Decimal::ZERO,
    };

    let monetary = inflation_score - dollar_penalty;

    if input.inflation_rate > dec!(0.10) {
        recommendations.push(format!(
            "High inflation ({:.1}%) erodes purchasing power and investment returns",
            inflation_pct
        ));
    }

    clamp(monetary, Decimal::ZERO, dec!(100))
}

/// Political score: average of political stability and rule of law.
fn compute_political_score(input: &CountryRiskInput, recommendations: &mut Vec<String>) -> Decimal {
    let political = (input.political_stability_score + input.rule_of_law_score) / dec!(2);

    if input.political_stability_score < dec!(30) {
        recommendations
            .push("Low political stability score indicates elevated governance risk".into());
    }
    if input.rule_of_law_score < dec!(30) {
        recommendations
            .push("Weak rule of law increases risk of expropriation or contract breach".into());
    }

    clamp(political, Decimal::ZERO, dec!(100))
}

/// Structural score: based on GDP growth and default history.
///
/// GDP growth: >5% => 100, <-2% => 0
/// Default history: -20 point penalty
fn compute_structural_score(
    input: &CountryRiskInput,
    recommendations: &mut Vec<String>,
) -> Decimal {
    // GDP growth score: -2% => 0, 5% => 100
    let growth_pct = input.gdp_growth_rate * dec!(100);
    let growth_score = linear_score(growth_pct, dec!(-2), dec!(5), false);

    // Default history penalty
    let default_penalty = if input.sovereign_default_history {
        recommendations.push("Sovereign default in past 20 years increases risk perception".into());
        dec!(20)
    } else {
        Decimal::ZERO
    };

    let structural = growth_score - default_penalty;

    if input.gdp_growth_rate < Decimal::ZERO {
        recommendations.push(format!(
            "Negative GDP growth ({:.1}%) indicates recessionary conditions",
            growth_pct
        ));
    }

    clamp(structural, Decimal::ZERO, dec!(100))
}

// ---------------------------------------------------------------------------
// Rating & CRP mapping
// ---------------------------------------------------------------------------

/// Map composite score (0-100) to a rating string.
fn score_to_rating(score: Decimal) -> String {
    if score >= dec!(90) {
        "AAA".into()
    } else if score >= dec!(80) {
        "AA".into()
    } else if score >= dec!(70) {
        "A".into()
    } else if score >= dec!(60) {
        "BBB".into()
    } else if score >= dec!(50) {
        "BB".into()
    } else if score >= dec!(40) {
        "B".into()
    } else if score >= dec!(30) {
        "CCC".into()
    } else {
        "CC".into()
    }
}

/// Map composite score to risk category.
fn score_to_risk_category(score: Decimal) -> String {
    if score >= dec!(70) {
        "Low Risk".into()
    } else if score >= dec!(50) {
        "Moderate Risk".into()
    } else if score >= dec!(30) {
        "High Risk".into()
    } else {
        "Very High Risk".into()
    }
}

/// Map composite score to country risk premium in basis points.
///
/// Uses rating band CRP values with linear interpolation within bands:
/// AAA(90-100)=0, AA(80-90)=50, A(70-80)=100, BBB(60-70)=200,
/// BB(50-60)=350, B(40-50)=500, CCC(30-40)=800, CC(<30)=1200
fn score_to_crp(score: Decimal) -> Decimal {
    let score = clamp(score, Decimal::ZERO, dec!(100));

    // Define breakpoints: (score_threshold, crp_at_threshold)
    // Linear interpolation between adjacent points
    let breakpoints: [(Decimal, Decimal); 9] = [
        (dec!(100), dec!(0)),
        (dec!(90), dec!(0)),
        (dec!(80), dec!(50)),
        (dec!(70), dec!(100)),
        (dec!(60), dec!(200)),
        (dec!(50), dec!(350)),
        (dec!(40), dec!(500)),
        (dec!(30), dec!(800)),
        (dec!(0), dec!(1200)),
    ];

    // Find the bracket
    for i in 0..breakpoints.len() - 1 {
        let (s_high, crp_high) = breakpoints[i];
        let (s_low, crp_low) = breakpoints[i + 1];

        if score >= s_low && score <= s_high {
            if s_high == s_low {
                return crp_high;
            }
            // Linear interpolation: as score decreases from s_high to s_low,
            // CRP increases from crp_high to crp_low
            let fraction = (s_high - score) / (s_high - s_low);
            return crp_high + fraction * (crp_low - crp_high);
        }
    }

    // Fallback for score exactly 0
    dec!(1200)
}

/// Compute implied 5-year cumulative default probability from CRP.
///
/// PD = 1 - (1 - spread / (1 - recovery))^5
/// where spread = CRP / 10000 and recovery = 0.40
fn compute_implied_pd(crp_bps: Decimal) -> Decimal {
    let spread = crp_bps / dec!(10000);
    let recovery = dec!(0.40);
    let lgd = Decimal::ONE - recovery; // 0.60

    if lgd.is_zero() {
        return Decimal::ZERO;
    }

    let annual_pd = spread / lgd;

    // Clamp annual PD to [0, 1]
    let annual_pd = clamp(annual_pd, Decimal::ZERO, Decimal::ONE);

    // 5-year cumulative PD = 1 - (1 - annual_pd)^5
    let survival_1y = Decimal::ONE - annual_pd;
    let mut survival_5y = Decimal::ONE;
    for _ in 0..5 {
        survival_5y *= survival_1y;
    }

    let pd_5y = Decimal::ONE - survival_5y;
    clamp(pd_5y, Decimal::ZERO, Decimal::ONE)
}

// ---------------------------------------------------------------------------
// Utility helpers
// ---------------------------------------------------------------------------

/// Linear interpolation score between two thresholds.
///
/// If `invert` is true, lower values get higher scores (e.g. debt/GDP).
/// If `invert` is false, higher values get higher scores (e.g. reserves).
fn linear_score(value: Decimal, low: Decimal, high: Decimal, invert: bool) -> Decimal {
    if high == low {
        return dec!(50);
    }

    let score = if invert {
        // Low value = high score: debt at 30% => 100, debt at 120% => 0
        if value <= low {
            dec!(100)
        } else if value >= high {
            Decimal::ZERO
        } else {
            dec!(100) * (high - value) / (high - low)
        }
    } else {
        // High value = high score: reserves at 12 months => 100, at 3 months => 0
        if value >= high {
            dec!(100)
        } else if value <= low {
            Decimal::ZERO
        } else {
            dec!(100) * (value - low) / (high - low)
        }
    };

    clamp(score, Decimal::ZERO, dec!(100))
}

/// Clamp a value between min and max.
fn clamp(value: Decimal, min: Decimal, max: Decimal) -> Decimal {
    if value < min {
        min
    } else if value > max {
        max
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

    fn assert_close(actual: Decimal, expected: Decimal, tolerance: Decimal, label: &str) {
        let diff = (actual - expected).abs();
        assert!(
            diff <= tolerance,
            "{label}: expected ~{expected}, got {actual} (diff {diff} > tolerance {tolerance})"
        );
    }

    /// AAA-rated country (Switzerland-like)
    fn aaa_country() -> CountryRiskInput {
        CountryRiskInput {
            country: "Switzerland".into(),
            gdp_growth_rate: dec!(0.02),
            inflation_rate: dec!(0.01),
            fiscal_balance_pct_gdp: dec!(0.01), // 1% surplus
            debt_to_gdp: dec!(0.25),
            current_account_pct_gdp: dec!(0.08),
            fx_reserves_months_imports: dec!(24),
            political_stability_score: dec!(95),
            rule_of_law_score: dec!(95),
            external_debt_to_gdp: dec!(0.20),
            short_term_debt_to_reserves: dec!(0.20),
            sovereign_default_history: false,
            dollarization_pct: None,
        }
    }

    /// Emerging market country (Brazil-like)
    fn emerging_market() -> CountryRiskInput {
        CountryRiskInput {
            country: "Brazil".into(),
            gdp_growth_rate: dec!(0.02),
            inflation_rate: dec!(0.05),
            fiscal_balance_pct_gdp: dec!(-0.04), // 4% deficit
            debt_to_gdp: dec!(0.75),
            current_account_pct_gdp: dec!(-0.03),
            fx_reserves_months_imports: dec!(15),
            political_stability_score: dec!(50),
            rule_of_law_score: dec!(45),
            external_debt_to_gdp: dec!(0.35),
            short_term_debt_to_reserves: dec!(0.60),
            sovereign_default_history: false,
            dollarization_pct: None,
        }
    }

    /// Frontier market country (high risk)
    fn frontier_market() -> CountryRiskInput {
        CountryRiskInput {
            country: "Zambia".into(),
            gdp_growth_rate: dec!(0.01),
            inflation_rate: dec!(0.15),
            fiscal_balance_pct_gdp: dec!(-0.07), // 7% deficit
            debt_to_gdp: dec!(1.10),
            current_account_pct_gdp: dec!(-0.06),
            fx_reserves_months_imports: dec!(2),
            political_stability_score: dec!(30),
            rule_of_law_score: dec!(25),
            external_debt_to_gdp: dec!(0.80),
            short_term_debt_to_reserves: dec!(1.80),
            sovereign_default_history: true,
            dollarization_pct: Some(dec!(30)),
        }
    }

    /// Recently defaulted sovereign
    fn defaulted_sovereign() -> CountryRiskInput {
        CountryRiskInput {
            country: "Argentina".into(),
            gdp_growth_rate: dec!(-0.02),
            inflation_rate: dec!(0.25),
            fiscal_balance_pct_gdp: dec!(-0.06),
            debt_to_gdp: dec!(0.90),
            current_account_pct_gdp: dec!(-0.02),
            fx_reserves_months_imports: dec!(4),
            political_stability_score: dec!(35),
            rule_of_law_score: dec!(30),
            external_debt_to_gdp: dec!(0.60),
            short_term_debt_to_reserves: dec!(1.20),
            sovereign_default_history: true,
            dollarization_pct: Some(dec!(60)),
        }
    }

    // -----------------------------------------------------------------------
    // 1. AAA country gets high score
    // -----------------------------------------------------------------------
    #[test]
    fn test_aaa_country_high_score() {
        let result = assess_country_risk(&aaa_country()).unwrap();

        assert!(
            result.sovereign_credit_score >= dec!(80),
            "AAA country should score >= 80, got {}",
            result.sovereign_credit_score
        );
    }

    // -----------------------------------------------------------------------
    // 2. AAA country gets low CRP
    // -----------------------------------------------------------------------
    #[test]
    fn test_aaa_country_low_crp() {
        let result = assess_country_risk(&aaa_country()).unwrap();

        assert!(
            result.country_risk_premium <= dec!(50),
            "AAA country CRP should be <= 50 bps, got {}",
            result.country_risk_premium
        );
    }

    // -----------------------------------------------------------------------
    // 3. AAA country rating
    // -----------------------------------------------------------------------
    #[test]
    fn test_aaa_country_rating() {
        let result = assess_country_risk(&aaa_country()).unwrap();

        assert!(
            result.sovereign_rating_equivalent == "AAA"
                || result.sovereign_rating_equivalent == "AA",
            "AAA country should get AAA or AA rating, got {}",
            result.sovereign_rating_equivalent
        );
    }

    // -----------------------------------------------------------------------
    // 4. AAA country is low risk
    // -----------------------------------------------------------------------
    #[test]
    fn test_aaa_country_risk_category() {
        let result = assess_country_risk(&aaa_country()).unwrap();

        assert_eq!(
            result.risk_category, "Low Risk",
            "AAA country should be Low Risk"
        );
    }

    // -----------------------------------------------------------------------
    // 5. Emerging market moderate score
    // -----------------------------------------------------------------------
    #[test]
    fn test_emerging_market_moderate_score() {
        let result = assess_country_risk(&emerging_market()).unwrap();

        assert!(
            result.sovereign_credit_score >= dec!(40) && result.sovereign_credit_score <= dec!(70),
            "Emerging market should score 40-70, got {}",
            result.sovereign_credit_score
        );
    }

    // -----------------------------------------------------------------------
    // 6. Emerging market moderate CRP
    // -----------------------------------------------------------------------
    #[test]
    fn test_emerging_market_crp() {
        let result = assess_country_risk(&emerging_market()).unwrap();

        assert!(
            result.country_risk_premium >= dec!(100) && result.country_risk_premium <= dec!(500),
            "Emerging market CRP should be 100-500 bps, got {}",
            result.country_risk_premium
        );
    }

    // -----------------------------------------------------------------------
    // 7. Frontier market high risk
    // -----------------------------------------------------------------------
    #[test]
    fn test_frontier_market_high_risk() {
        let result = assess_country_risk(&frontier_market()).unwrap();

        assert!(
            result.sovereign_credit_score < dec!(40),
            "Frontier market should score < 40, got {}",
            result.sovereign_credit_score
        );
    }

    // -----------------------------------------------------------------------
    // 8. Frontier market high CRP
    // -----------------------------------------------------------------------
    #[test]
    fn test_frontier_market_high_crp() {
        let result = assess_country_risk(&frontier_market()).unwrap();

        assert!(
            result.country_risk_premium >= dec!(500),
            "Frontier market CRP should be >= 500 bps, got {}",
            result.country_risk_premium
        );
    }

    // -----------------------------------------------------------------------
    // 9. Defaulted sovereign low score
    // -----------------------------------------------------------------------
    #[test]
    fn test_defaulted_sovereign_low_score() {
        let result = assess_country_risk(&defaulted_sovereign()).unwrap();

        assert!(
            result.sovereign_credit_score < dec!(40),
            "Defaulted sovereign should score < 40, got {}",
            result.sovereign_credit_score
        );
    }

    // -----------------------------------------------------------------------
    // 10. Defaulted sovereign high CRP
    // -----------------------------------------------------------------------
    #[test]
    fn test_defaulted_sovereign_high_crp() {
        let result = assess_country_risk(&defaulted_sovereign()).unwrap();

        assert!(
            result.country_risk_premium >= dec!(500),
            "Defaulted sovereign CRP should be >= 500 bps, got {}",
            result.country_risk_premium
        );
    }

    // -----------------------------------------------------------------------
    // 11. Defaulted sovereign has default history recommendation
    // -----------------------------------------------------------------------
    #[test]
    fn test_defaulted_sovereign_recommendations() {
        let result = assess_country_risk(&defaulted_sovereign()).unwrap();

        let has_default_note = result.recommendations.iter().any(|r| r.contains("default"));
        assert!(
            has_default_note,
            "Defaulted sovereign should have default-related recommendation"
        );
    }

    // -----------------------------------------------------------------------
    // 12. Score ordering: AAA > EM > Frontier
    // -----------------------------------------------------------------------
    #[test]
    fn test_score_ordering() {
        let aaa = assess_country_risk(&aaa_country()).unwrap();
        let em = assess_country_risk(&emerging_market()).unwrap();
        let frontier = assess_country_risk(&frontier_market()).unwrap();

        assert!(
            aaa.sovereign_credit_score > em.sovereign_credit_score,
            "AAA ({}) > EM ({})",
            aaa.sovereign_credit_score,
            em.sovereign_credit_score
        );
        assert!(
            em.sovereign_credit_score > frontier.sovereign_credit_score,
            "EM ({}) > Frontier ({})",
            em.sovereign_credit_score,
            frontier.sovereign_credit_score
        );
    }

    // -----------------------------------------------------------------------
    // 13. CRP ordering: AAA < EM < Frontier
    // -----------------------------------------------------------------------
    #[test]
    fn test_crp_ordering() {
        let aaa = assess_country_risk(&aaa_country()).unwrap();
        let em = assess_country_risk(&emerging_market()).unwrap();
        let frontier = assess_country_risk(&frontier_market()).unwrap();

        assert!(
            aaa.country_risk_premium < em.country_risk_premium,
            "AAA CRP ({}) < EM CRP ({})",
            aaa.country_risk_premium,
            em.country_risk_premium
        );
        assert!(
            em.country_risk_premium < frontier.country_risk_premium,
            "EM CRP ({}) < Frontier CRP ({})",
            em.country_risk_premium,
            frontier.country_risk_premium
        );
    }

    // -----------------------------------------------------------------------
    // 14. Implied PD ordering: AAA < EM < Frontier
    // -----------------------------------------------------------------------
    #[test]
    fn test_implied_pd_ordering() {
        let aaa = assess_country_risk(&aaa_country()).unwrap();
        let em = assess_country_risk(&emerging_market()).unwrap();
        let frontier = assess_country_risk(&frontier_market()).unwrap();

        assert!(
            aaa.implied_default_probability < em.implied_default_probability,
            "AAA PD ({}) < EM PD ({})",
            aaa.implied_default_probability,
            em.implied_default_probability
        );
        assert!(
            em.implied_default_probability < frontier.implied_default_probability,
            "EM PD ({}) < Frontier PD ({})",
            em.implied_default_probability,
            frontier.implied_default_probability
        );
    }

    // -----------------------------------------------------------------------
    // 15. Fiscal score: low debt gets high score
    // -----------------------------------------------------------------------
    #[test]
    fn test_fiscal_score_low_debt() {
        let result = assess_country_risk(&aaa_country()).unwrap();

        assert!(
            result.component_scores.fiscal_score >= dec!(70),
            "Low-debt country fiscal score should be >= 70, got {}",
            result.component_scores.fiscal_score
        );
    }

    // -----------------------------------------------------------------------
    // 16. Fiscal score: high debt gets low score
    // -----------------------------------------------------------------------
    #[test]
    fn test_fiscal_score_high_debt() {
        let result = assess_country_risk(&frontier_market()).unwrap();

        assert!(
            result.component_scores.fiscal_score < dec!(30),
            "High-debt country fiscal score should be < 30, got {}",
            result.component_scores.fiscal_score
        );
    }

    // -----------------------------------------------------------------------
    // 17. External score: high reserves gets high score
    // -----------------------------------------------------------------------
    #[test]
    fn test_external_score_high_reserves() {
        let result = assess_country_risk(&aaa_country()).unwrap();

        assert!(
            result.component_scores.external_score >= dec!(80),
            "High-reserves country external score should be >= 80, got {}",
            result.component_scores.external_score
        );
    }

    // -----------------------------------------------------------------------
    // 18. External score: low reserves gets low score
    // -----------------------------------------------------------------------
    #[test]
    fn test_external_score_low_reserves() {
        let result = assess_country_risk(&frontier_market()).unwrap();

        assert!(
            result.component_scores.external_score < dec!(30),
            "Low-reserves country external score should be < 30, got {}",
            result.component_scores.external_score
        );
    }

    // -----------------------------------------------------------------------
    // 19. Monetary score: low inflation gets high score
    // -----------------------------------------------------------------------
    #[test]
    fn test_monetary_score_low_inflation() {
        let result = assess_country_risk(&aaa_country()).unwrap();

        assert!(
            result.component_scores.monetary_score >= dec!(90),
            "Low-inflation country monetary score should be >= 90, got {}",
            result.component_scores.monetary_score
        );
    }

    // -----------------------------------------------------------------------
    // 20. Monetary score: high inflation gets low score
    // -----------------------------------------------------------------------
    #[test]
    fn test_monetary_score_high_inflation() {
        let result = assess_country_risk(&defaulted_sovereign()).unwrap();

        assert!(
            result.component_scores.monetary_score < dec!(30),
            "High-inflation country monetary score should be < 30, got {}",
            result.component_scores.monetary_score
        );
    }

    // -----------------------------------------------------------------------
    // 21. Political score: high stability gets high score
    // -----------------------------------------------------------------------
    #[test]
    fn test_political_score_high_stability() {
        let result = assess_country_risk(&aaa_country()).unwrap();

        assert!(
            result.component_scores.political_score >= dec!(90),
            "Stable country political score should be >= 90, got {}",
            result.component_scores.political_score
        );
    }

    // -----------------------------------------------------------------------
    // 22. Political score: low stability gets low score
    // -----------------------------------------------------------------------
    #[test]
    fn test_political_score_low_stability() {
        let result = assess_country_risk(&frontier_market()).unwrap();

        assert!(
            result.component_scores.political_score < dec!(40),
            "Unstable country political score should be < 40, got {}",
            result.component_scores.political_score
        );
    }

    // -----------------------------------------------------------------------
    // 23. Structural score: default history penalizes
    // -----------------------------------------------------------------------
    #[test]
    fn test_structural_score_default_penalty() {
        let mut no_default = emerging_market();
        no_default.sovereign_default_history = false;

        let mut with_default = emerging_market();
        with_default.sovereign_default_history = true;

        let r_no = assess_country_risk(&no_default).unwrap();
        let r_with = assess_country_risk(&with_default).unwrap();

        assert!(
            r_with.component_scores.structural_score < r_no.component_scores.structural_score,
            "Default history should reduce structural score: {} vs {}",
            r_with.component_scores.structural_score,
            r_no.component_scores.structural_score
        );
    }

    // -----------------------------------------------------------------------
    // 24. Structural score: positive growth boosts score
    // -----------------------------------------------------------------------
    #[test]
    fn test_structural_score_positive_growth() {
        let high_growth = CountryRiskInput {
            gdp_growth_rate: dec!(0.06),
            ..aaa_country()
        };
        let result = assess_country_risk(&high_growth).unwrap();

        assert!(
            result.component_scores.structural_score >= dec!(90),
            "High growth structural score should be >= 90, got {}",
            result.component_scores.structural_score
        );
    }

    // -----------------------------------------------------------------------
    // 25. Structural score: negative growth reduces score
    // -----------------------------------------------------------------------
    #[test]
    fn test_structural_score_negative_growth() {
        let negative_growth = CountryRiskInput {
            gdp_growth_rate: dec!(-0.03),
            sovereign_default_history: false,
            ..frontier_market()
        };
        let result = assess_country_risk(&negative_growth).unwrap();

        assert!(
            result.component_scores.structural_score < dec!(20),
            "Negative growth structural score should be < 20, got {}",
            result.component_scores.structural_score
        );
    }

    // -----------------------------------------------------------------------
    // 26. Dollarization penalty applied
    // -----------------------------------------------------------------------
    #[test]
    fn test_dollarization_penalty() {
        let mut no_dollar = emerging_market();
        no_dollar.dollarization_pct = None;

        let mut high_dollar = emerging_market();
        high_dollar.dollarization_pct = Some(dec!(80));

        let r_no = assess_country_risk(&no_dollar).unwrap();
        let r_high = assess_country_risk(&high_dollar).unwrap();

        assert!(
            r_high.component_scores.monetary_score < r_no.component_scores.monetary_score,
            "High dollarization should reduce monetary score: {} vs {}",
            r_high.component_scores.monetary_score,
            r_no.component_scores.monetary_score
        );
    }

    // -----------------------------------------------------------------------
    // 27. Full dollarization (100%) penalty
    // -----------------------------------------------------------------------
    #[test]
    fn test_full_dollarization() {
        let input = CountryRiskInput {
            dollarization_pct: Some(dec!(100)),
            ..emerging_market()
        };
        let result = assess_country_risk(&input).unwrap();

        // With full dollarization, monetary score should be lower
        assert!(
            result.component_scores.monetary_score >= Decimal::ZERO,
            "Monetary score should not go negative, got {}",
            result.component_scores.monetary_score
        );
    }

    // -----------------------------------------------------------------------
    // 28. Rating mapping: score 95 => AAA
    // -----------------------------------------------------------------------
    #[test]
    fn test_rating_mapping_aaa() {
        assert_eq!(score_to_rating(dec!(95)), "AAA");
        assert_eq!(score_to_rating(dec!(90)), "AAA");
    }

    // -----------------------------------------------------------------------
    // 29. Rating mapping: score 85 => AA
    // -----------------------------------------------------------------------
    #[test]
    fn test_rating_mapping_aa() {
        assert_eq!(score_to_rating(dec!(85)), "AA");
    }

    // -----------------------------------------------------------------------
    // 30. Rating mapping: score 75 => A
    // -----------------------------------------------------------------------
    #[test]
    fn test_rating_mapping_a() {
        assert_eq!(score_to_rating(dec!(75)), "A");
    }

    // -----------------------------------------------------------------------
    // 31. Rating mapping: score 65 => BBB
    // -----------------------------------------------------------------------
    #[test]
    fn test_rating_mapping_bbb() {
        assert_eq!(score_to_rating(dec!(65)), "BBB");
    }

    // -----------------------------------------------------------------------
    // 32. Rating mapping: score 55 => BB
    // -----------------------------------------------------------------------
    #[test]
    fn test_rating_mapping_bb() {
        assert_eq!(score_to_rating(dec!(55)), "BB");
    }

    // -----------------------------------------------------------------------
    // 33. Rating mapping: score 45 => B
    // -----------------------------------------------------------------------
    #[test]
    fn test_rating_mapping_b() {
        assert_eq!(score_to_rating(dec!(45)), "B");
    }

    // -----------------------------------------------------------------------
    // 34. Rating mapping: score 35 => CCC
    // -----------------------------------------------------------------------
    #[test]
    fn test_rating_mapping_ccc() {
        assert_eq!(score_to_rating(dec!(35)), "CCC");
    }

    // -----------------------------------------------------------------------
    // 35. Rating mapping: score 20 => CC
    // -----------------------------------------------------------------------
    #[test]
    fn test_rating_mapping_cc() {
        assert_eq!(score_to_rating(dec!(20)), "CC");
    }

    // -----------------------------------------------------------------------
    // 36. CRP for AAA score (90-100) should be ~0
    // -----------------------------------------------------------------------
    #[test]
    fn test_crp_for_aaa_score() {
        let crp = score_to_crp(dec!(95));
        assert_close(crp, Decimal::ZERO, dec!(1), "AAA CRP should be ~0");
    }

    // -----------------------------------------------------------------------
    // 37. CRP interpolation within band
    // -----------------------------------------------------------------------
    #[test]
    fn test_crp_interpolation() {
        // Score 85 is midpoint of AA band (80-90), CRP 0 at 90, 50 at 80
        let crp = score_to_crp(dec!(85));
        assert_close(crp, dec!(25), dec!(1), "Mid-AA band CRP should be ~25");
    }

    // -----------------------------------------------------------------------
    // 38. CRP at band boundary
    // -----------------------------------------------------------------------
    #[test]
    fn test_crp_at_boundary() {
        let crp_80 = score_to_crp(dec!(80));
        assert_close(crp_80, dec!(50), dec!(1), "Score 80 CRP should be 50");

        let crp_70 = score_to_crp(dec!(70));
        assert_close(crp_70, dec!(100), dec!(1), "Score 70 CRP should be 100");

        let crp_60 = score_to_crp(dec!(60));
        assert_close(crp_60, dec!(200), dec!(1), "Score 60 CRP should be 200");
    }

    // -----------------------------------------------------------------------
    // 39. Implied PD: zero CRP => zero PD
    // -----------------------------------------------------------------------
    #[test]
    fn test_implied_pd_zero() {
        let pd = compute_implied_pd(Decimal::ZERO);
        assert_eq!(pd, Decimal::ZERO, "Zero CRP should give zero PD");
    }

    // -----------------------------------------------------------------------
    // 40. Implied PD: reasonable range
    // -----------------------------------------------------------------------
    #[test]
    fn test_implied_pd_range() {
        let pd_low = compute_implied_pd(dec!(50)); // AA
        let pd_mid = compute_implied_pd(dec!(350)); // BB
        let pd_high = compute_implied_pd(dec!(800)); // CCC

        assert!(
            pd_low > Decimal::ZERO && pd_low < dec!(0.10),
            "Low CRP PD should be 0-10%, got {}",
            pd_low
        );
        assert!(
            pd_mid > pd_low,
            "Higher CRP should have higher PD: {} vs {}",
            pd_mid,
            pd_low
        );
        assert!(
            pd_high > pd_mid,
            "Even higher CRP should have even higher PD: {} vs {}",
            pd_high,
            pd_mid
        );
    }

    // -----------------------------------------------------------------------
    // 41. Implied PD monotonically increases with CRP
    // -----------------------------------------------------------------------
    #[test]
    fn test_implied_pd_monotonic() {
        let crps = [
            dec!(0),
            dec!(50),
            dec!(100),
            dec!(200),
            dec!(350),
            dec!(500),
            dec!(800),
            dec!(1200),
        ];
        let pds: Vec<Decimal> = crps.iter().map(|&c| compute_implied_pd(c)).collect();

        for i in 1..pds.len() {
            assert!(
                pds[i] >= pds[i - 1],
                "PD should be monotonically increasing: PD[{}]={} vs PD[{}]={}",
                i - 1,
                pds[i - 1],
                i,
                pds[i]
            );
        }
    }

    // -----------------------------------------------------------------------
    // 42. Empty country name rejected
    // -----------------------------------------------------------------------
    #[test]
    fn test_empty_country_rejected() {
        let input = CountryRiskInput {
            country: "".into(),
            ..aaa_country()
        };
        let err = assess_country_risk(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => assert_eq!(field, "country"),
            other => panic!("Expected InvalidInput for country, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // 43. Political score out of range rejected
    // -----------------------------------------------------------------------
    #[test]
    fn test_political_score_out_of_range() {
        let input = CountryRiskInput {
            political_stability_score: dec!(110),
            ..aaa_country()
        };
        let err = assess_country_risk(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "political_stability_score")
            }
            other => panic!("Expected InvalidInput for political_stability_score, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // 44. Rule of law out of range rejected
    // -----------------------------------------------------------------------
    #[test]
    fn test_rule_of_law_out_of_range() {
        let input = CountryRiskInput {
            rule_of_law_score: dec!(-5),
            ..aaa_country()
        };
        let err = assess_country_risk(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "rule_of_law_score")
            }
            other => panic!("Expected InvalidInput for rule_of_law_score, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // 45. Negative debt to GDP rejected
    // -----------------------------------------------------------------------
    #[test]
    fn test_negative_debt_to_gdp_rejected() {
        let input = CountryRiskInput {
            debt_to_gdp: dec!(-0.10),
            ..aaa_country()
        };
        let err = assess_country_risk(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => assert_eq!(field, "debt_to_gdp"),
            other => panic!("Expected InvalidInput for debt_to_gdp, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // 46. Very high debt to GDP
    // -----------------------------------------------------------------------
    #[test]
    fn test_very_high_debt_to_gdp() {
        let input = CountryRiskInput {
            debt_to_gdp: dec!(2.00), // 200% debt/GDP
            fiscal_balance_pct_gdp: dec!(-0.10),
            ..emerging_market()
        };
        let result = assess_country_risk(&input).unwrap();

        assert!(
            result.component_scores.fiscal_score < dec!(10),
            "200% debt/GDP should give near-zero fiscal score, got {}",
            result.component_scores.fiscal_score
        );
    }

    // -----------------------------------------------------------------------
    // 47. Dollarization percentage out of range rejected
    // -----------------------------------------------------------------------
    #[test]
    fn test_dollarization_out_of_range() {
        let input = CountryRiskInput {
            dollarization_pct: Some(dec!(150)),
            ..emerging_market()
        };
        let err = assess_country_risk(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "dollarization_pct")
            }
            other => panic!("Expected InvalidInput for dollarization_pct, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // 48. Recommendations generated for frontier market
    // -----------------------------------------------------------------------
    #[test]
    fn test_recommendations_generated() {
        let result = assess_country_risk(&frontier_market()).unwrap();

        assert!(
            !result.recommendations.is_empty(),
            "Frontier market should generate recommendations"
        );
        assert!(
            result.recommendations.len() >= 3,
            "Frontier market should have multiple recommendations, got {}",
            result.recommendations.len()
        );
    }

    // -----------------------------------------------------------------------
    // 49. Linear score boundary: at low threshold
    // -----------------------------------------------------------------------
    #[test]
    fn test_linear_score_at_low() {
        // Inverted: value at low threshold => 100
        let score = linear_score(dec!(30), dec!(30), dec!(120), true);
        assert_close(
            score,
            dec!(100),
            dec!(0.01),
            "At low threshold inverted = 100",
        );

        // Normal: value at low threshold => 0
        let score = linear_score(dec!(3), dec!(3), dec!(12), false);
        assert_close(
            score,
            Decimal::ZERO,
            dec!(0.01),
            "At low threshold normal = 0",
        );
    }

    // -----------------------------------------------------------------------
    // 50. Linear score boundary: at high threshold
    // -----------------------------------------------------------------------
    #[test]
    fn test_linear_score_at_high() {
        // Inverted: value at high threshold => 0
        let score = linear_score(dec!(120), dec!(30), dec!(120), true);
        assert_close(
            score,
            Decimal::ZERO,
            dec!(0.01),
            "At high threshold inverted = 0",
        );

        // Normal: value at high threshold => 100
        let score = linear_score(dec!(12), dec!(3), dec!(12), false);
        assert_close(
            score,
            dec!(100),
            dec!(0.01),
            "At high threshold normal = 100",
        );
    }

    // -----------------------------------------------------------------------
    // 51. Linear score: midpoint
    // -----------------------------------------------------------------------
    #[test]
    fn test_linear_score_midpoint() {
        // Inverted midpoint: (30+120)/2 = 75 => 50
        let score = linear_score(dec!(75), dec!(30), dec!(120), true);
        assert_close(score, dec!(50), dec!(0.1), "Midpoint inverted = 50");

        // Normal midpoint: (3+12)/2 = 7.5 => 50
        let score = linear_score(dec!(7.5), dec!(3), dec!(12), false);
        assert_close(score, dec!(50), dec!(0.1), "Midpoint normal = 50");
    }

    // -----------------------------------------------------------------------
    // 52. Negative FX reserves rejected
    // -----------------------------------------------------------------------
    #[test]
    fn test_negative_fx_reserves_rejected() {
        let input = CountryRiskInput {
            fx_reserves_months_imports: dec!(-1),
            ..aaa_country()
        };
        let err = assess_country_risk(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "fx_reserves_months_imports")
            }
            other => panic!("Expected InvalidInput for fx_reserves, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // 53. Composite score clamped to 0-100
    // -----------------------------------------------------------------------
    #[test]
    fn test_composite_score_clamped() {
        // Perfect scores should still be <= 100
        let result = assess_country_risk(&aaa_country()).unwrap();
        assert!(
            result.sovereign_credit_score <= dec!(100),
            "Score should be <= 100, got {}",
            result.sovereign_credit_score
        );
        assert!(
            result.sovereign_credit_score >= Decimal::ZERO,
            "Score should be >= 0, got {}",
            result.sovereign_credit_score
        );
    }
}
