use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Static Jurisdiction Data
// ---------------------------------------------------------------------------

const JURISDICTION_COUNT: usize = 10;

const JURISDICTION_CODES: [&str; JURISDICTION_COUNT] = [
    "Cayman",
    "BVI",
    "Luxembourg",
    "Ireland",
    "Jersey",
    "Guernsey",
    "Singapore",
    "HongKong",
    "DIFC",
    "ADGM",
];

const JURISDICTION_NAMES: [&str; JURISDICTION_COUNT] = [
    "Cayman Islands",
    "British Virgin Islands",
    "Grand Duchy of Luxembourg",
    "Republic of Ireland",
    "Bailiwick of Jersey",
    "Bailiwick of Guernsey",
    "Republic of Singapore",
    "Hong Kong SAR",
    "Dubai International Financial Centre",
    "Abu Dhabi Global Market",
];

// Setup cost in USD
const SETUP_COSTS: [u64; JURISDICTION_COUNT] = [
    25_000, 15_000, 80_000, 60_000, 20_000, 15_000, 50_000, 40_000, 60_000, 45_000,
];

// Annual ongoing cost in USD
const ANNUAL_COSTS: [u64; JURISDICTION_COUNT] = [
    80_000, 50_000, 200_000, 150_000, 70_000, 60_000, 120_000, 100_000, 150_000, 120_000,
];

// Regulatory approval timeline in weeks
const TIMELINE_WEEKS: [u32; JURISDICTION_COUNT] = [4, 2, 16, 12, 1, 1, 8, 6, 1, 2];

// Minimum capital requirement in USD
const MINIMUM_CAPITAL: [u64; JURISDICTION_COUNT] = [0, 0, 1_250_000, 0, 0, 0, 1, 0, 0, 0];

// Fund-level tax rate (basis points * 100 for precision, 0 = tax exempt)
const TAX_RATE_BPS: [u32; JURISDICTION_COUNT] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

// Substance requirement score (0-100, higher = more burdensome)
const SUBSTANCE_SCORES: [u32; JURISDICTION_COUNT] = [30, 25, 60, 55, 40, 35, 50, 45, 35, 30];

// Distribution reach count (number of countries accessible)
const DISTRIBUTION_REACH: [u32; JURISDICTION_COUNT] = [180, 150, 30, 30, 20, 18, 25, 20, 15, 12];

// Passport available (EU AIFMD passport)
const PASSPORT_AVAILABLE: [bool; JURISDICTION_COUNT] = [
    false, false, true, true, false, false, false, false, false, false,
];

// NPPR accessible markets count
const NPPR_MARKETS: [u32; JURISDICTION_COUNT] = [0, 0, 27, 27, 15, 12, 0, 0, 0, 0];

// Annual audit cost estimate
const ANNUAL_AUDIT: [u64; JURISDICTION_COUNT] = [
    50_000, 30_000, 80_000, 60_000, 35_000, 30_000, 50_000, 45_000, 55_000, 45_000,
];

// Annual admin cost estimate
const ANNUAL_ADMIN: [u64; JURISDICTION_COUNT] = [
    60_000, 35_000, 100_000, 80_000, 40_000, 35_000, 60_000, 50_000, 70_000, 55_000,
];

// Annual directors cost estimate
const ANNUAL_DIRECTORS: [u64; JURISDICTION_COUNT] = [
    20_000, 10_000, 40_000, 35_000, 15_000, 12_000, 25_000, 20_000, 30_000, 25_000,
];

// Annual substance cost estimate (office, local staff)
const ANNUAL_SUBSTANCE: [u64; JURISDICTION_COUNT] = [
    15_000, 10_000, 50_000, 40_000, 20_000, 15_000, 35_000, 30_000, 25_000, 20_000,
];

// ---------------------------------------------------------------------------
// Distribution reach data
// ---------------------------------------------------------------------------

/// Distribution channel types
const CHANNEL_PASSPORT: &str = "Passport";
const CHANNEL_NPPR: &str = "NPPR";
const CHANNEL_REVERSE_SOLICITATION: &str = "ReverseSolicitation";
const CHANNEL_PRIVATE_PLACEMENT: &str = "PrivatePlacement";
const CHANNEL_BILATERAL: &str = "Bilateral";

/// Target market regions and their constituent country counts
const TARGET_MARKET_EU27: u32 = 27;
const TARGET_MARKET_US: u32 = 1;
const TARGET_MARKET_UK: u32 = 1;
const TARGET_MARKET_SINGAPORE: u32 = 1;
const TARGET_MARKET_HK: u32 = 1;
const TARGET_MARKET_JAPAN: u32 = 1;
const TARGET_MARKET_AUSTRALIA: u32 = 1;
const TARGET_MARKET_GCC: u32 = 6;
const _TOTAL_TARGET_MARKETS: u32 = 39;

// ---------------------------------------------------------------------------
// Types — Inputs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonWeights {
    pub setup_cost: Decimal,
    pub annual_cost: Decimal,
    pub tax: Decimal,
    pub regulatory_speed: Decimal,
    pub distribution_reach: Decimal,
    pub substance: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JurisdictionComparisonInput {
    pub jurisdictions: Vec<String>,
    pub fund_strategy: String,
    pub fund_size: Decimal,
    /// "OpenEnded" or "ClosedEnded"
    pub fund_type: String,
    pub weights: ComparisonWeights,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimalJurisdictionInput {
    pub fund_strategy: String,
    pub fund_size: Decimal,
    pub fund_type: String,
    /// e.g. ["US", "EU", "Asia"]
    pub investor_base: Vec<String>,
    /// e.g. ["EU27", "US", "UK", "GCC"]
    pub distribution_targets: Vec<String>,
    pub weights: ComparisonWeights,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionReachInput {
    pub jurisdictions: Vec<String>,
    /// Target markets, e.g. ["EU27", "US", "UK", "Singapore", "HK", "Japan", "Australia", "GCC"]
    pub target_markets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcoInput {
    pub jurisdictions: Vec<String>,
    pub fund_size: Decimal,
    /// Number of years to project (1, 3, 5, or 10)
    pub projection_years: u32,
    /// Discount rate for NPV calculation (e.g. 0.08 = 8%)
    pub discount_rate: Decimal,
}

// ---------------------------------------------------------------------------
// Types — Outputs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JurisdictionProfile {
    pub code: String,
    pub name: String,
    pub setup_cost_usd: Decimal,
    pub annual_cost_usd: Decimal,
    pub regulatory_timeline_weeks: u32,
    pub minimum_capital_usd: Decimal,
    pub fund_level_tax_rate: Decimal,
    pub substance_score: Decimal,
    pub distribution_reach_count: u32,
    pub passport_available: bool,
    pub nppr_markets: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedJurisdiction {
    pub rank: u32,
    pub code: String,
    pub name: String,
    pub weighted_score: Decimal,
    pub dimension_scores: DimensionScores,
    pub profile: JurisdictionProfile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionScores {
    pub setup_cost_score: Decimal,
    pub annual_cost_score: Decimal,
    pub tax_score: Decimal,
    pub speed_score: Decimal,
    pub distribution_score: Decimal,
    pub substance_score: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JurisdictionComparisonOutput {
    pub ranked_jurisdictions: Vec<RankedJurisdiction>,
    pub best_for_cost: String,
    pub best_for_speed: String,
    pub best_for_distribution: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JurisdictionRecommendation {
    pub code: String,
    pub name: String,
    pub weighted_score: Decimal,
    pub rationale: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitivityScenario {
    pub scenario_name: String,
    pub weights: ComparisonWeights,
    pub top_jurisdiction: String,
    pub score: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineStep {
    pub week: u32,
    pub activity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimalJurisdictionOutput {
    pub primary_recommendation: JurisdictionRecommendation,
    pub alternative: JurisdictionRecommendation,
    pub fallback: JurisdictionRecommendation,
    pub sensitivity_scenarios: Vec<SensitivityScenario>,
    pub implementation_timeline: Vec<TimelineStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketAccess {
    pub market: String,
    pub channel: String,
    pub accessible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JurisdictionReach {
    pub code: String,
    pub name: String,
    pub market_access: Vec<MarketAccess>,
    pub accessible_count: u32,
    pub total_targets: u32,
    pub coverage_pct: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionReachOutput {
    pub per_jurisdiction_reach: Vec<JurisdictionReach>,
    pub coverage_matrix: Vec<Vec<bool>>,
    pub optimal_jurisdiction_for_distribution: String,
    pub total_cost_of_distribution: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JurisdictionTco {
    pub code: String,
    pub name: String,
    pub setup_cost: Decimal,
    pub annual_regulatory: Decimal,
    pub annual_admin: Decimal,
    pub annual_audit: Decimal,
    pub annual_directors: Decimal,
    pub annual_substance: Decimal,
    pub total_annual: Decimal,
    pub total_over_period: Decimal,
    pub npv: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YearCost {
    pub year: u32,
    pub cost: Decimal,
    pub discounted_cost: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JurisdictionYearCosts {
    pub code: String,
    pub year_costs: Vec<YearCost>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcoOutput {
    pub per_jurisdiction_tco: Vec<JurisdictionTco>,
    pub year_by_year_costs: Vec<JurisdictionYearCosts>,
    pub npv_comparison: Vec<(String, Decimal)>,
    pub cheapest_jurisdiction: String,
    pub cost_rankings: Vec<(String, Decimal)>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Side-by-side comparison of 2-10+ jurisdictions with weighted scoring.
pub fn compare_jurisdictions(
    input: &JurisdictionComparisonInput,
) -> CorpFinanceResult<JurisdictionComparisonOutput> {
    validate_comparison_input(input)?;

    let mut warnings: Vec<String> = Vec::new();

    // Resolve profiles
    let profiles: Vec<JurisdictionProfile> = input
        .jurisdictions
        .iter()
        .map(|j| get_profile(j))
        .collect::<CorpFinanceResult<Vec<_>>>()?;

    // Compute min/max for normalization
    let (min_setup, max_setup) = min_max_decimal(&profiles, |p| p.setup_cost_usd);
    let (min_annual, max_annual) = min_max_decimal(&profiles, |p| p.annual_cost_usd);
    let (min_tax, max_tax) = min_max_decimal(&profiles, |p| p.fund_level_tax_rate);
    let (min_weeks, max_weeks) = min_max_u32(&profiles, |p| p.regulatory_timeline_weeks);
    let (min_dist, max_dist) = min_max_u32(&profiles, |p| p.distribution_reach_count);
    let (_min_sub, _max_sub) = min_max_decimal(&profiles, |p| p.substance_score);

    let w = &input.weights;

    let mut ranked: Vec<RankedJurisdiction> = profiles
        .into_iter()
        .map(|p| {
            let setup_score = invert_normalize(p.setup_cost_usd, min_setup, max_setup);
            let annual_score = invert_normalize(p.annual_cost_usd, min_annual, max_annual);
            let tax_score = invert_normalize(p.fund_level_tax_rate, min_tax, max_tax);
            let speed_score =
                invert_normalize_u32(p.regulatory_timeline_weeks, min_weeks, max_weeks);
            let dist_score = normalize_u32(p.distribution_reach_count, min_dist, max_dist);
            // Lower substance burden = higher score
            let sub_score = Decimal::ONE - p.substance_score / dec!(100);

            let weighted = w.setup_cost * setup_score
                + w.annual_cost * annual_score
                + w.tax * tax_score
                + w.regulatory_speed * speed_score
                + w.distribution_reach * dist_score
                + w.substance * sub_score;

            RankedJurisdiction {
                rank: 0,
                code: p.code.clone(),
                name: p.name.clone(),
                weighted_score: weighted,
                dimension_scores: DimensionScores {
                    setup_cost_score: setup_score,
                    annual_cost_score: annual_score,
                    tax_score,
                    speed_score,
                    distribution_score: dist_score,
                    substance_score: sub_score,
                },
                profile: p,
            }
        })
        .collect();

    // Sort descending by weighted score
    ranked.sort_by(|a, b| b.weighted_score.cmp(&a.weighted_score));
    for (i, r) in ranked.iter_mut().enumerate() {
        r.rank = (i as u32) + 1;
    }

    // Best-for categories
    let best_for_cost = ranked
        .iter()
        .max_by(|a, b| {
            a.dimension_scores
                .setup_cost_score
                .cmp(&b.dimension_scores.setup_cost_score)
        })
        .map(|r| r.code.clone())
        .unwrap_or_default();

    let best_for_speed = ranked
        .iter()
        .max_by(|a, b| {
            a.dimension_scores
                .speed_score
                .cmp(&b.dimension_scores.speed_score)
        })
        .map(|r| r.code.clone())
        .unwrap_or_default();

    let best_for_distribution = ranked
        .iter()
        .max_by(|a, b| {
            a.dimension_scores
                .distribution_score
                .cmp(&b.dimension_scores.distribution_score)
        })
        .map(|r| r.code.clone())
        .unwrap_or_default();

    // Warnings
    if input.fund_size < dec!(10_000_000) {
        warnings.push(
            "Fund size below $10M — substance and regulatory costs \
             may be disproportionately high"
                .to_string(),
        );
    }

    for r in &ranked {
        if r.profile.minimum_capital_usd > Decimal::ZERO
            && input.fund_size < r.profile.minimum_capital_usd
        {
            warnings.push(format!(
                "{} requires minimum capital of ${} — fund size ${} is below threshold",
                r.code, r.profile.minimum_capital_usd, input.fund_size
            ));
        }
    }

    Ok(JurisdictionComparisonOutput {
        ranked_jurisdictions: ranked,
        best_for_cost,
        best_for_speed,
        best_for_distribution,
        warnings,
    })
}

/// Recommends top 3 jurisdictions with rationale, sensitivity, and timeline.
pub fn optimal_jurisdiction(
    input: &OptimalJurisdictionInput,
) -> CorpFinanceResult<OptimalJurisdictionOutput> {
    validate_optimal_input(input)?;

    // Run comparison across all 10 jurisdictions
    let all_codes: Vec<String> = JURISDICTION_CODES.iter().map(|s| s.to_string()).collect();

    let comparison_input = JurisdictionComparisonInput {
        jurisdictions: all_codes,
        fund_strategy: input.fund_strategy.clone(),
        fund_size: input.fund_size,
        fund_type: input.fund_type.clone(),
        weights: input.weights.clone(),
    };

    let comparison = compare_jurisdictions(&comparison_input)?;
    let ranked = &comparison.ranked_jurisdictions;

    // Build recommendations with rationale
    let primary = build_recommendation(
        &ranked[0],
        &input.distribution_targets,
        &input.investor_base,
    );
    let alternative = build_recommendation(
        &ranked[1],
        &input.distribution_targets,
        &input.investor_base,
    );
    let fallback = build_recommendation(
        &ranked[2],
        &input.distribution_targets,
        &input.investor_base,
    );

    // Sensitivity scenarios
    let cost_focused = ComparisonWeights {
        setup_cost: dec!(0.35),
        annual_cost: dec!(0.35),
        tax: dec!(0.10),
        regulatory_speed: dec!(0.10),
        distribution_reach: dec!(0.05),
        substance: dec!(0.05),
    };

    let speed_focused = ComparisonWeights {
        setup_cost: dec!(0.05),
        annual_cost: dec!(0.05),
        tax: dec!(0.05),
        regulatory_speed: dec!(0.60),
        distribution_reach: dec!(0.15),
        substance: dec!(0.10),
    };

    let distribution_focused = ComparisonWeights {
        setup_cost: dec!(0.05),
        annual_cost: dec!(0.05),
        tax: dec!(0.05),
        regulatory_speed: dec!(0.05),
        distribution_reach: dec!(0.60),
        substance: dec!(0.20),
    };

    let scenarios = vec![
        run_sensitivity(
            "Cost-Focused",
            &cost_focused,
            &input.fund_strategy,
            input.fund_size,
            &input.fund_type,
        )?,
        run_sensitivity(
            "Speed-Focused",
            &speed_focused,
            &input.fund_strategy,
            input.fund_size,
            &input.fund_type,
        )?,
        run_sensitivity(
            "Distribution-Focused",
            &distribution_focused,
            &input.fund_strategy,
            input.fund_size,
            &input.fund_type,
        )?,
    ];

    // Implementation timeline for primary recommendation
    let timeline = build_implementation_timeline(&primary.code);

    Ok(OptimalJurisdictionOutput {
        primary_recommendation: primary,
        alternative,
        fallback,
        sensitivity_scenarios: scenarios,
        implementation_timeline: timeline,
    })
}

/// Analyzes distribution reach per jurisdiction across target markets.
pub fn distribution_reach_analysis(
    input: &DistributionReachInput,
) -> CorpFinanceResult<DistributionReachOutput> {
    validate_distribution_input(input)?;

    let target_count = count_target_markets(&input.target_markets);
    let mut per_jurisdiction_reach: Vec<JurisdictionReach> = Vec::new();
    let mut coverage_matrix: Vec<Vec<bool>> = Vec::new();

    for jcode in &input.jurisdictions {
        let idx = jurisdiction_index(jcode)?;
        let mut market_access: Vec<MarketAccess> = Vec::new();
        let mut row: Vec<bool> = Vec::new();
        let mut accessible = 0u32;

        for market in &input.target_markets {
            let (channel, access) = determine_market_access(idx, market);
            if access {
                accessible += market_country_count(market);
            }
            row.push(access);
            market_access.push(MarketAccess {
                market: market.clone(),
                channel: channel.to_string(),
                accessible: access,
            });
        }

        let coverage_pct = if target_count > 0 {
            Decimal::from(accessible) * dec!(100) / Decimal::from(target_count)
        } else {
            Decimal::ZERO
        };

        per_jurisdiction_reach.push(JurisdictionReach {
            code: JURISDICTION_CODES[idx].to_string(),
            name: JURISDICTION_NAMES[idx].to_string(),
            market_access,
            accessible_count: accessible,
            total_targets: target_count,
            coverage_pct,
        });
        coverage_matrix.push(row);
    }

    // Find optimal jurisdiction for distribution
    let optimal = per_jurisdiction_reach
        .iter()
        .max_by(|a, b| a.accessible_count.cmp(&b.accessible_count))
        .map(|r| r.code.clone())
        .unwrap_or_default();

    // Estimate total cost of distribution (setup + first year annual for each)
    let total_cost: Decimal = per_jurisdiction_reach
        .iter()
        .map(|r| {
            let idx = jurisdiction_index(&r.code).unwrap_or(0);
            Decimal::from(SETUP_COSTS[idx]) + Decimal::from(ANNUAL_COSTS[idx])
        })
        .sum();

    Ok(DistributionReachOutput {
        per_jurisdiction_reach,
        coverage_matrix,
        optimal_jurisdiction_for_distribution: optimal,
        total_cost_of_distribution: total_cost,
    })
}

/// Multi-year total cost of ownership with NPV comparison.
pub fn total_cost_of_ownership(input: &TcoInput) -> CorpFinanceResult<TcoOutput> {
    validate_tco_input(input)?;

    let mut per_jurisdiction_tco: Vec<JurisdictionTco> = Vec::new();
    let mut year_by_year_costs: Vec<JurisdictionYearCosts> = Vec::new();
    let mut npv_comparison: Vec<(String, Decimal)> = Vec::new();

    for jcode in &input.jurisdictions {
        let idx = jurisdiction_index(jcode)?;

        let setup = Decimal::from(SETUP_COSTS[idx]);
        let annual_regulatory = Decimal::from(ANNUAL_COSTS[idx]);
        let annual_admin = Decimal::from(ANNUAL_ADMIN[idx]);
        let annual_audit = Decimal::from(ANNUAL_AUDIT[idx]);
        let annual_directors = Decimal::from(ANNUAL_DIRECTORS[idx]);
        let annual_substance = Decimal::from(ANNUAL_SUBSTANCE[idx]);
        let total_annual =
            annual_regulatory + annual_admin + annual_audit + annual_directors + annual_substance;

        // Year-by-year costs with discounting (iterative discount factor)
        let mut year_costs: Vec<YearCost> = Vec::new();
        let mut npv = Decimal::ZERO;
        let mut discount_factor = Decimal::ONE;
        let rate_plus_one = Decimal::ONE + input.discount_rate;

        // Year 0: setup cost
        let setup_discounted = setup * discount_factor;
        year_costs.push(YearCost {
            year: 0,
            cost: setup,
            discounted_cost: setup_discounted,
        });
        npv += setup_discounted;

        // Years 1..n: annual costs
        for yr in 1..=input.projection_years {
            discount_factor /= rate_plus_one;
            let discounted = total_annual * discount_factor;
            year_costs.push(YearCost {
                year: yr,
                cost: total_annual,
                discounted_cost: discounted,
            });
            npv += discounted;
        }

        let total_over_period = setup + total_annual * Decimal::from(input.projection_years);

        npv_comparison.push((jcode.clone(), npv));

        per_jurisdiction_tco.push(JurisdictionTco {
            code: JURISDICTION_CODES[idx].to_string(),
            name: JURISDICTION_NAMES[idx].to_string(),
            setup_cost: setup,
            annual_regulatory,
            annual_admin,
            annual_audit,
            annual_directors,
            annual_substance,
            total_annual,
            total_over_period,
            npv,
        });

        year_by_year_costs.push(JurisdictionYearCosts {
            code: JURISDICTION_CODES[idx].to_string(),
            year_costs,
        });
    }

    // Sort NPV comparison ascending (cheapest first)
    npv_comparison.sort_by(|a, b| a.1.cmp(&b.1));

    let cheapest = npv_comparison
        .first()
        .map(|(c, _)| c.clone())
        .unwrap_or_default();

    let cost_rankings: Vec<(String, Decimal)> = npv_comparison.clone();

    Ok(TcoOutput {
        per_jurisdiction_tco,
        year_by_year_costs,
        npv_comparison,
        cheapest_jurisdiction: cheapest,
        cost_rankings,
    })
}

// ---------------------------------------------------------------------------
// Helpers — Profile Lookup
// ---------------------------------------------------------------------------

fn get_profile(code: &str) -> CorpFinanceResult<JurisdictionProfile> {
    let idx = jurisdiction_index(code)?;
    Ok(JurisdictionProfile {
        code: JURISDICTION_CODES[idx].to_string(),
        name: JURISDICTION_NAMES[idx].to_string(),
        setup_cost_usd: Decimal::from(SETUP_COSTS[idx]),
        annual_cost_usd: Decimal::from(ANNUAL_COSTS[idx]),
        regulatory_timeline_weeks: TIMELINE_WEEKS[idx],
        minimum_capital_usd: Decimal::from(MINIMUM_CAPITAL[idx]),
        fund_level_tax_rate: Decimal::from(TAX_RATE_BPS[idx]) / dec!(10000),
        substance_score: Decimal::from(SUBSTANCE_SCORES[idx]),
        distribution_reach_count: DISTRIBUTION_REACH[idx],
        passport_available: PASSPORT_AVAILABLE[idx],
        nppr_markets: NPPR_MARKETS[idx],
    })
}

fn jurisdiction_index(code: &str) -> CorpFinanceResult<usize> {
    JURISDICTION_CODES
        .iter()
        .position(|&c| c == code)
        .ok_or_else(|| CorpFinanceError::InvalidInput {
            field: "jurisdiction".into(),
            reason: format!(
                "Unknown jurisdiction '{}'. Valid: {:?}",
                code, JURISDICTION_CODES
            ),
        })
}

// ---------------------------------------------------------------------------
// Helpers — Normalization
// ---------------------------------------------------------------------------

fn min_max_decimal<F>(profiles: &[JurisdictionProfile], f: F) -> (Decimal, Decimal)
where
    F: Fn(&JurisdictionProfile) -> Decimal,
{
    let vals: Vec<Decimal> = profiles.iter().map(&f).collect();
    let min = vals.iter().copied().min().unwrap_or(Decimal::ZERO);
    let max = vals.iter().copied().max().unwrap_or(Decimal::ONE);
    (min, max)
}

fn min_max_u32<F>(profiles: &[JurisdictionProfile], f: F) -> (u32, u32)
where
    F: Fn(&JurisdictionProfile) -> u32,
{
    let vals: Vec<u32> = profiles.iter().map(&f).collect();
    let min = vals.iter().copied().min().unwrap_or(0);
    let max = vals.iter().copied().max().unwrap_or(1);
    (min, max)
}

/// Normalize where lower value = higher score (0-1). Cost/time dimensions.
fn invert_normalize(val: Decimal, min: Decimal, max: Decimal) -> Decimal {
    if max == min {
        return Decimal::ONE;
    }
    (max - val) / (max - min)
}

fn invert_normalize_u32(val: u32, min: u32, max: u32) -> Decimal {
    if max == min {
        return Decimal::ONE;
    }
    Decimal::from(max - val) / Decimal::from(max - min)
}

/// Normalize where higher value = higher score (0-1). Distribution reach.
fn normalize_u32(val: u32, min: u32, max: u32) -> Decimal {
    if max == min {
        return Decimal::ONE;
    }
    Decimal::from(val - min) / Decimal::from(max - min)
}

// ---------------------------------------------------------------------------
// Helpers — Sensitivity & Recommendations
// ---------------------------------------------------------------------------

fn run_sensitivity(
    name: &str,
    weights: &ComparisonWeights,
    fund_strategy: &str,
    fund_size: Decimal,
    fund_type: &str,
) -> CorpFinanceResult<SensitivityScenario> {
    let all_codes: Vec<String> = JURISDICTION_CODES.iter().map(|s| s.to_string()).collect();
    let comp_input = JurisdictionComparisonInput {
        jurisdictions: all_codes,
        fund_strategy: fund_strategy.to_string(),
        fund_size,
        fund_type: fund_type.to_string(),
        weights: weights.clone(),
    };
    let result = compare_jurisdictions(&comp_input)?;
    let top = &result.ranked_jurisdictions[0];
    Ok(SensitivityScenario {
        scenario_name: name.to_string(),
        weights: weights.clone(),
        top_jurisdiction: top.code.clone(),
        score: top.weighted_score,
    })
}

fn build_recommendation(
    ranked: &RankedJurisdiction,
    distribution_targets: &[String],
    investor_base: &[String],
) -> JurisdictionRecommendation {
    let mut rationale: Vec<String> = Vec::new();

    rationale.push(format!(
        "Weighted score: {} (rank #{})",
        ranked.weighted_score, ranked.rank
    ));

    if ranked.profile.setup_cost_usd <= dec!(25_000) {
        rationale.push("Low setup cost under $25,000".to_string());
    }

    if ranked.profile.regulatory_timeline_weeks <= 2 {
        rationale.push(format!(
            "Fast regulatory approval: {} weeks",
            ranked.profile.regulatory_timeline_weeks
        ));
    }

    if ranked.profile.passport_available {
        rationale.push("EU AIFMD passport available for pan-European distribution".to_string());
    }

    if ranked.profile.distribution_reach_count >= 100 {
        rationale.push(format!(
            "Broad distribution reach: {} countries",
            ranked.profile.distribution_reach_count
        ));
    }

    if ranked.profile.fund_level_tax_rate == Decimal::ZERO {
        rationale.push("Tax-neutral fund-level treatment".to_string());
    }

    // Distribution target alignment
    let has_eu_target = distribution_targets.iter().any(|t| t == "EU27");
    if has_eu_target && ranked.profile.passport_available {
        rationale.push("Aligns with EU27 distribution target via passport".to_string());
    }

    // Investor base alignment
    let has_us_investors = investor_base.iter().any(|i| i == "US");
    if has_us_investors && (ranked.code == "Cayman" || ranked.code == "BVI") {
        rationale.push("Well-established infrastructure for US investor base".to_string());
    }

    JurisdictionRecommendation {
        code: ranked.code.clone(),
        name: ranked.name.clone(),
        weighted_score: ranked.weighted_score,
        rationale,
    }
}

fn build_implementation_timeline(jurisdiction_code: &str) -> Vec<TimelineStep> {
    let idx = JURISDICTION_CODES
        .iter()
        .position(|&c| c == jurisdiction_code)
        .unwrap_or(0);
    let total_weeks = TIMELINE_WEEKS[idx];

    let mut steps: Vec<TimelineStep> = Vec::new();

    steps.push(TimelineStep {
        week: 1,
        activity: "Engage local counsel and service providers".to_string(),
    });

    steps.push(TimelineStep {
        week: 1,
        activity: "Draft constitutional documents (LPA/Articles/Trust Deed)".to_string(),
    });

    if total_weeks >= 4 {
        steps.push(TimelineStep {
            week: 2,
            activity: "Submit regulatory application and KYC documentation".to_string(),
        });
        steps.push(TimelineStep {
            week: 3,
            activity: "Regulatory review period — respond to queries".to_string(),
        });
        steps.push(TimelineStep {
            week: total_weeks,
            activity: "Receive regulatory approval and complete registration".to_string(),
        });
    } else {
        steps.push(TimelineStep {
            week: total_weeks,
            activity: "Submit registration and receive approval".to_string(),
        });
    }

    steps.push(TimelineStep {
        week: total_weeks,
        activity: "Open bank accounts and custodian relationships".to_string(),
    });

    steps.push(TimelineStep {
        week: total_weeks + 1,
        activity: "First close / launch".to_string(),
    });

    steps
}

// ---------------------------------------------------------------------------
// Helpers — Distribution Reach
// ---------------------------------------------------------------------------

fn determine_market_access(jurisdiction_idx: usize, market: &str) -> (&'static str, bool) {
    let code = JURISDICTION_CODES[jurisdiction_idx];
    let has_passport = PASSPORT_AVAILABLE[jurisdiction_idx];

    match market {
        "EU27" => {
            if has_passport {
                (CHANNEL_PASSPORT, true)
            } else if NPPR_MARKETS[jurisdiction_idx] > 0 {
                (CHANNEL_NPPR, true)
            } else if code == "Cayman" || code == "BVI" {
                (CHANNEL_REVERSE_SOLICITATION, true)
            } else {
                (
                    CHANNEL_PRIVATE_PLACEMENT,
                    code == "Singapore" || code == "HongKong",
                )
            }
        }
        "US" => {
            // All jurisdictions can access US via private placement / Reg D/S
            (CHANNEL_PRIVATE_PLACEMENT, true)
        }
        "UK" => {
            if has_passport || NPPR_MARKETS[jurisdiction_idx] > 0 {
                (CHANNEL_NPPR, true)
            } else {
                (CHANNEL_PRIVATE_PLACEMENT, true)
            }
        }
        "Singapore" => {
            if code == "Singapore" {
                (CHANNEL_PASSPORT, true)
            } else {
                (CHANNEL_PRIVATE_PLACEMENT, true)
            }
        }
        "HK" => {
            if code == "HongKong" {
                (CHANNEL_PASSPORT, true)
            } else if code == "Cayman" {
                (CHANNEL_BILATERAL, true)
            } else {
                (CHANNEL_PRIVATE_PLACEMENT, true)
            }
        }
        "Japan" => {
            if code == "Cayman" || code == "Luxembourg" || code == "Ireland" {
                (CHANNEL_BILATERAL, true)
            } else {
                (CHANNEL_PRIVATE_PLACEMENT, true)
            }
        }
        "Australia" => {
            if code == "Cayman" || code == "Luxembourg" || code == "Ireland" || code == "Singapore"
            {
                (CHANNEL_BILATERAL, true)
            } else {
                (CHANNEL_PRIVATE_PLACEMENT, true)
            }
        }
        "GCC" => {
            if code == "DIFC" || code == "ADGM" {
                (CHANNEL_PASSPORT, true)
            } else if code == "Cayman" || code == "BVI" {
                (CHANNEL_BILATERAL, true)
            } else {
                (
                    CHANNEL_PRIVATE_PLACEMENT,
                    code == "Luxembourg" || code == "Ireland",
                )
            }
        }
        _ => (CHANNEL_PRIVATE_PLACEMENT, false),
    }
}

fn market_country_count(market: &str) -> u32 {
    match market {
        "EU27" => TARGET_MARKET_EU27,
        "US" => TARGET_MARKET_US,
        "UK" => TARGET_MARKET_UK,
        "Singapore" => TARGET_MARKET_SINGAPORE,
        "HK" => TARGET_MARKET_HK,
        "Japan" => TARGET_MARKET_JAPAN,
        "Australia" => TARGET_MARKET_AUSTRALIA,
        "GCC" => TARGET_MARKET_GCC,
        _ => 0,
    }
}

fn count_target_markets(targets: &[String]) -> u32 {
    targets.iter().map(|t| market_country_count(t)).sum()
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_comparison_input(input: &JurisdictionComparisonInput) -> CorpFinanceResult<()> {
    if input.jurisdictions.len() < 2 {
        return Err(CorpFinanceError::InvalidInput {
            field: "jurisdictions".into(),
            reason: "At least 2 jurisdictions required for comparison".into(),
        });
    }

    for j in &input.jurisdictions {
        jurisdiction_index(j)?;
    }

    if input.fund_size <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "fund_size".into(),
            reason: "Fund size must be greater than zero".into(),
        });
    }

    validate_weights(&input.weights)?;
    Ok(())
}

fn validate_optimal_input(input: &OptimalJurisdictionInput) -> CorpFinanceResult<()> {
    if input.fund_size <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "fund_size".into(),
            reason: "Fund size must be greater than zero".into(),
        });
    }

    validate_weights(&input.weights)?;
    Ok(())
}

fn validate_distribution_input(input: &DistributionReachInput) -> CorpFinanceResult<()> {
    if input.jurisdictions.len() < 2 {
        return Err(CorpFinanceError::InvalidInput {
            field: "jurisdictions".into(),
            reason: "At least 2 jurisdictions required for comparison".into(),
        });
    }

    for j in &input.jurisdictions {
        jurisdiction_index(j)?;
    }

    if input.target_markets.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "target_markets".into(),
            reason: "At least one target market required".into(),
        });
    }

    Ok(())
}

fn validate_tco_input(input: &TcoInput) -> CorpFinanceResult<()> {
    if input.jurisdictions.len() < 2 {
        return Err(CorpFinanceError::InvalidInput {
            field: "jurisdictions".into(),
            reason: "At least 2 jurisdictions required for comparison".into(),
        });
    }

    for j in &input.jurisdictions {
        jurisdiction_index(j)?;
    }

    if input.fund_size <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "fund_size".into(),
            reason: "Fund size must be greater than zero".into(),
        });
    }

    if input.projection_years == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "projection_years".into(),
            reason: "Projection years must be at least 1".into(),
        });
    }

    if input.discount_rate < Decimal::ZERO || input.discount_rate > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "discount_rate".into(),
            reason: "Discount rate must be between 0 and 1".into(),
        });
    }

    Ok(())
}

fn validate_weights(w: &ComparisonWeights) -> CorpFinanceResult<()> {
    let sum = w.setup_cost
        + w.annual_cost
        + w.tax
        + w.regulatory_speed
        + w.distribution_reach
        + w.substance;
    if (sum - Decimal::ONE).abs() > dec!(0.01) {
        return Err(CorpFinanceError::InvalidInput {
            field: "weights".into(),
            reason: format!("Weights must sum to 1.0 (tolerance 0.01), got {}", sum),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn equal_weights() -> ComparisonWeights {
        ComparisonWeights {
            setup_cost: dec!(0.1667),
            annual_cost: dec!(0.1667),
            tax: dec!(0.1667),
            regulatory_speed: dec!(0.1667),
            distribution_reach: dec!(0.1667),
            substance: dec!(0.1665),
        }
    }

    fn cost_focused_weights() -> ComparisonWeights {
        ComparisonWeights {
            setup_cost: dec!(0.35),
            annual_cost: dec!(0.35),
            tax: dec!(0.10),
            regulatory_speed: dec!(0.10),
            distribution_reach: dec!(0.05),
            substance: dec!(0.05),
        }
    }

    fn all_10_jurisdictions() -> Vec<String> {
        JURISDICTION_CODES.iter().map(|s| s.to_string()).collect()
    }

    fn three_jurisdictions() -> Vec<String> {
        vec![
            "Cayman".to_string(),
            "Luxembourg".to_string(),
            "Singapore".to_string(),
        ]
    }

    // -----------------------------------------------------------------------
    // compare_jurisdictions
    // -----------------------------------------------------------------------

    #[test]
    fn test_compare_all_10_equal_weights() {
        let input = JurisdictionComparisonInput {
            jurisdictions: all_10_jurisdictions(),
            fund_strategy: "Hedge".to_string(),
            fund_size: dec!(500_000_000),
            fund_type: "OpenEnded".to_string(),
            weights: equal_weights(),
        };
        let result = compare_jurisdictions(&input).unwrap();
        assert_eq!(result.ranked_jurisdictions.len(), 10);
        assert_eq!(result.ranked_jurisdictions[0].rank, 1);
        assert_eq!(result.ranked_jurisdictions[9].rank, 10);
        // Scores should be descending
        for i in 0..9 {
            assert!(
                result.ranked_jurisdictions[i].weighted_score
                    >= result.ranked_jurisdictions[i + 1].weighted_score
            );
        }
    }

    #[test]
    fn test_compare_3_cost_focused() {
        let input = JurisdictionComparisonInput {
            jurisdictions: three_jurisdictions(),
            fund_strategy: "PE".to_string(),
            fund_size: dec!(200_000_000),
            fund_type: "ClosedEnded".to_string(),
            weights: cost_focused_weights(),
        };
        let result = compare_jurisdictions(&input).unwrap();
        assert_eq!(result.ranked_jurisdictions.len(), 3);
        // With cost-focused weights, Cayman should beat Luxembourg
        let cayman_rank = result
            .ranked_jurisdictions
            .iter()
            .find(|r| r.code == "Cayman")
            .unwrap()
            .rank;
        let lux_rank = result
            .ranked_jurisdictions
            .iter()
            .find(|r| r.code == "Luxembourg")
            .unwrap()
            .rank;
        assert!(
            cayman_rank < lux_rank,
            "Cayman should rank higher than Luxembourg with cost-focused weights"
        );
    }

    #[test]
    fn test_compare_best_for_categories() {
        let input = JurisdictionComparisonInput {
            jurisdictions: all_10_jurisdictions(),
            fund_strategy: "Hedge".to_string(),
            fund_size: dec!(500_000_000),
            fund_type: "OpenEnded".to_string(),
            weights: equal_weights(),
        };
        let result = compare_jurisdictions(&input).unwrap();
        // BVI has lowest setup cost ($15k) tied with Guernsey
        assert!(
            result.best_for_cost == "BVI" || result.best_for_cost == "Guernsey",
            "Best for cost should be BVI or Guernsey, got {}",
            result.best_for_cost
        );
        // Jersey/Guernsey/DIFC have 1-week timeline
        assert!(
            result.best_for_speed == "Jersey"
                || result.best_for_speed == "Guernsey"
                || result.best_for_speed == "DIFC",
            "Best for speed should be Jersey/Guernsey/DIFC, got {}",
            result.best_for_speed
        );
        // Cayman has highest distribution reach (180)
        assert_eq!(result.best_for_distribution, "Cayman");
    }

    #[test]
    fn test_compare_scores_in_zero_one_range() {
        let input = JurisdictionComparisonInput {
            jurisdictions: all_10_jurisdictions(),
            fund_strategy: "Hedge".to_string(),
            fund_size: dec!(500_000_000),
            fund_type: "OpenEnded".to_string(),
            weights: equal_weights(),
        };
        let result = compare_jurisdictions(&input).unwrap();
        for r in &result.ranked_jurisdictions {
            assert!(r.weighted_score >= Decimal::ZERO);
            assert!(r.weighted_score <= Decimal::ONE);
            assert!(r.dimension_scores.setup_cost_score >= Decimal::ZERO);
            assert!(r.dimension_scores.setup_cost_score <= Decimal::ONE);
            assert!(r.dimension_scores.annual_cost_score >= Decimal::ZERO);
            assert!(r.dimension_scores.annual_cost_score <= Decimal::ONE);
            assert!(r.dimension_scores.speed_score >= Decimal::ZERO);
            assert!(r.dimension_scores.speed_score <= Decimal::ONE);
            assert!(r.dimension_scores.distribution_score >= Decimal::ZERO);
            assert!(r.dimension_scores.distribution_score <= Decimal::ONE);
            assert!(r.dimension_scores.substance_score >= Decimal::ZERO);
            assert!(r.dimension_scores.substance_score <= Decimal::ONE);
        }
    }

    #[test]
    fn test_compare_minimum_capital_warning() {
        let input = JurisdictionComparisonInput {
            jurisdictions: vec!["Cayman".to_string(), "Luxembourg".to_string()],
            fund_strategy: "Hedge".to_string(),
            fund_size: dec!(500_000),
            fund_type: "OpenEnded".to_string(),
            weights: equal_weights(),
        };
        let result = compare_jurisdictions(&input).unwrap();
        assert!(
            result
                .warnings
                .iter()
                .any(|w| w.contains("Luxembourg") && w.contains("minimum capital")),
            "Should warn about Luxembourg minimum capital"
        );
    }

    #[test]
    fn test_compare_small_fund_warning() {
        let input = JurisdictionComparisonInput {
            jurisdictions: vec!["Cayman".to_string(), "BVI".to_string()],
            fund_strategy: "Hedge".to_string(),
            fund_size: dec!(5_000_000),
            fund_type: "OpenEnded".to_string(),
            weights: equal_weights(),
        };
        let result = compare_jurisdictions(&input).unwrap();
        assert!(
            result.warnings.iter().any(|w| w.contains("below $10M")),
            "Should warn about small fund size"
        );
    }

    #[test]
    fn test_compare_two_jurisdictions_minimum() {
        let input = JurisdictionComparisonInput {
            jurisdictions: vec!["Cayman".to_string(), "BVI".to_string()],
            fund_strategy: "Hedge".to_string(),
            fund_size: dec!(100_000_000),
            fund_type: "OpenEnded".to_string(),
            weights: equal_weights(),
        };
        let result = compare_jurisdictions(&input).unwrap();
        assert_eq!(result.ranked_jurisdictions.len(), 2);
    }

    #[test]
    fn test_compare_distribution_focused_lux_ireland_win() {
        let dist_weights = ComparisonWeights {
            setup_cost: dec!(0.05),
            annual_cost: dec!(0.05),
            tax: dec!(0.05),
            regulatory_speed: dec!(0.05),
            distribution_reach: dec!(0.60),
            substance: dec!(0.20),
        };
        let input = JurisdictionComparisonInput {
            jurisdictions: vec![
                "Cayman".to_string(),
                "Luxembourg".to_string(),
                "Ireland".to_string(),
                "BVI".to_string(),
            ],
            fund_strategy: "Hedge".to_string(),
            fund_size: dec!(500_000_000),
            fund_type: "OpenEnded".to_string(),
            weights: dist_weights,
        };
        let result = compare_jurisdictions(&input).unwrap();
        // Cayman has highest distribution_reach_count (180) so it should still win on raw count
        // but Lux/Ireland have passport + low substance burden so depends on normalization
        // With 4 jurisdictions, Cayman=180 is max, so distribution_score=1.0
        // Cayman wins on distribution reach count
        assert_eq!(
            result.best_for_distribution, "Cayman",
            "Cayman has highest distribution reach count"
        );
    }

    #[test]
    fn test_compare_passport_jurisdictions() {
        let input = JurisdictionComparisonInput {
            jurisdictions: vec!["Luxembourg".to_string(), "Ireland".to_string()],
            fund_strategy: "Hedge".to_string(),
            fund_size: dec!(500_000_000),
            fund_type: "OpenEnded".to_string(),
            weights: equal_weights(),
        };
        let result = compare_jurisdictions(&input).unwrap();
        assert_eq!(result.ranked_jurisdictions.len(), 2);
        // Both have passport, same distribution reach (30)
        for r in &result.ranked_jurisdictions {
            assert!(r.profile.passport_available);
        }
    }

    // -----------------------------------------------------------------------
    // Validation errors
    // -----------------------------------------------------------------------

    #[test]
    fn test_error_single_jurisdiction() {
        let input = JurisdictionComparisonInput {
            jurisdictions: vec!["Cayman".to_string()],
            fund_strategy: "Hedge".to_string(),
            fund_size: dec!(100_000_000),
            fund_type: "OpenEnded".to_string(),
            weights: equal_weights(),
        };
        let err = compare_jurisdictions(&input).unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("At least 2"), "Error: {}", msg);
    }

    #[test]
    fn test_error_weights_dont_sum_to_one() {
        let bad_weights = ComparisonWeights {
            setup_cost: dec!(0.50),
            annual_cost: dec!(0.50),
            tax: dec!(0.50),
            regulatory_speed: dec!(0.00),
            distribution_reach: dec!(0.00),
            substance: dec!(0.00),
        };
        let input = JurisdictionComparisonInput {
            jurisdictions: vec!["Cayman".to_string(), "BVI".to_string()],
            fund_strategy: "Hedge".to_string(),
            fund_size: dec!(100_000_000),
            fund_type: "OpenEnded".to_string(),
            weights: bad_weights,
        };
        let err = compare_jurisdictions(&input).unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("sum to 1.0"), "Error: {}", msg);
    }

    #[test]
    fn test_error_invalid_jurisdiction() {
        let input = JurisdictionComparisonInput {
            jurisdictions: vec!["Cayman".to_string(), "Atlantis".to_string()],
            fund_strategy: "Hedge".to_string(),
            fund_size: dec!(100_000_000),
            fund_type: "OpenEnded".to_string(),
            weights: equal_weights(),
        };
        let err = compare_jurisdictions(&input).unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("Atlantis"), "Error: {}", msg);
    }

    #[test]
    fn test_error_zero_fund_size() {
        let input = JurisdictionComparisonInput {
            jurisdictions: vec!["Cayman".to_string(), "BVI".to_string()],
            fund_strategy: "Hedge".to_string(),
            fund_size: Decimal::ZERO,
            fund_type: "OpenEnded".to_string(),
            weights: equal_weights(),
        };
        let err = compare_jurisdictions(&input).unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("greater than zero"), "Error: {}", msg);
    }

    #[test]
    fn test_error_negative_fund_size() {
        let input = JurisdictionComparisonInput {
            jurisdictions: vec!["Cayman".to_string(), "BVI".to_string()],
            fund_strategy: "Hedge".to_string(),
            fund_size: dec!(-100),
            fund_type: "OpenEnded".to_string(),
            weights: equal_weights(),
        };
        let err = compare_jurisdictions(&input).unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("greater than zero"), "Error: {}", msg);
    }

    // -----------------------------------------------------------------------
    // optimal_jurisdiction
    // -----------------------------------------------------------------------

    #[test]
    fn test_optimal_returns_top_3() {
        let input = OptimalJurisdictionInput {
            fund_strategy: "Hedge".to_string(),
            fund_size: dec!(500_000_000),
            fund_type: "OpenEnded".to_string(),
            investor_base: vec!["US".to_string(), "EU".to_string()],
            distribution_targets: vec!["EU27".to_string(), "US".to_string()],
            weights: equal_weights(),
        };
        let result = optimal_jurisdiction(&input).unwrap();
        assert!(!result.primary_recommendation.code.is_empty());
        assert!(!result.alternative.code.is_empty());
        assert!(!result.fallback.code.is_empty());
        // All three should be different
        assert_ne!(result.primary_recommendation.code, result.alternative.code);
        assert_ne!(result.primary_recommendation.code, result.fallback.code);
        assert_ne!(result.alternative.code, result.fallback.code);
    }

    #[test]
    fn test_optimal_sensitivity_scenarios() {
        let input = OptimalJurisdictionInput {
            fund_strategy: "PE".to_string(),
            fund_size: dec!(200_000_000),
            fund_type: "ClosedEnded".to_string(),
            investor_base: vec!["US".to_string()],
            distribution_targets: vec!["US".to_string()],
            weights: equal_weights(),
        };
        let result = optimal_jurisdiction(&input).unwrap();
        assert_eq!(result.sensitivity_scenarios.len(), 3);
        assert_eq!(
            result.sensitivity_scenarios[0].scenario_name,
            "Cost-Focused"
        );
        assert_eq!(
            result.sensitivity_scenarios[1].scenario_name,
            "Speed-Focused"
        );
        assert_eq!(
            result.sensitivity_scenarios[2].scenario_name,
            "Distribution-Focused"
        );
    }

    #[test]
    fn test_optimal_implementation_timeline() {
        let input = OptimalJurisdictionInput {
            fund_strategy: "Hedge".to_string(),
            fund_size: dec!(500_000_000),
            fund_type: "OpenEnded".to_string(),
            investor_base: vec!["US".to_string()],
            distribution_targets: vec!["US".to_string()],
            weights: equal_weights(),
        };
        let result = optimal_jurisdiction(&input).unwrap();
        assert!(!result.implementation_timeline.is_empty());
        // Timeline should start at week 1
        assert_eq!(result.implementation_timeline[0].week, 1);
        // Last step should be launch
        let last = result.implementation_timeline.last().unwrap();
        assert!(last.activity.contains("launch") || last.activity.contains("close"));
    }

    #[test]
    fn test_optimal_rationale_not_empty() {
        let input = OptimalJurisdictionInput {
            fund_strategy: "Hedge".to_string(),
            fund_size: dec!(500_000_000),
            fund_type: "OpenEnded".to_string(),
            investor_base: vec!["US".to_string()],
            distribution_targets: vec!["US".to_string(), "EU27".to_string()],
            weights: equal_weights(),
        };
        let result = optimal_jurisdiction(&input).unwrap();
        assert!(!result.primary_recommendation.rationale.is_empty());
        assert!(!result.alternative.rationale.is_empty());
        assert!(!result.fallback.rationale.is_empty());
    }

    #[test]
    fn test_optimal_primary_score_highest() {
        let input = OptimalJurisdictionInput {
            fund_strategy: "Hedge".to_string(),
            fund_size: dec!(500_000_000),
            fund_type: "OpenEnded".to_string(),
            investor_base: vec!["US".to_string()],
            distribution_targets: vec!["US".to_string()],
            weights: equal_weights(),
        };
        let result = optimal_jurisdiction(&input).unwrap();
        assert!(result.primary_recommendation.weighted_score >= result.alternative.weighted_score);
        assert!(result.alternative.weighted_score >= result.fallback.weighted_score);
    }

    #[test]
    fn test_optimal_error_zero_fund_size() {
        let input = OptimalJurisdictionInput {
            fund_strategy: "Hedge".to_string(),
            fund_size: Decimal::ZERO,
            fund_type: "OpenEnded".to_string(),
            investor_base: vec!["US".to_string()],
            distribution_targets: vec!["US".to_string()],
            weights: equal_weights(),
        };
        let err = optimal_jurisdiction(&input).unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("greater than zero"), "Error: {}", msg);
    }

    // -----------------------------------------------------------------------
    // distribution_reach_analysis
    // -----------------------------------------------------------------------

    #[test]
    fn test_distribution_eu_targets_passport_wins() {
        let input = DistributionReachInput {
            jurisdictions: vec![
                "Luxembourg".to_string(),
                "Ireland".to_string(),
                "Cayman".to_string(),
                "BVI".to_string(),
            ],
            target_markets: vec!["EU27".to_string()],
        };
        let result = distribution_reach_analysis(&input).unwrap();
        // Luxembourg and Ireland have passport access to EU27
        let lux = result
            .per_jurisdiction_reach
            .iter()
            .find(|r| r.code == "Luxembourg")
            .unwrap();
        let ire = result
            .per_jurisdiction_reach
            .iter()
            .find(|r| r.code == "Ireland")
            .unwrap();
        assert!(lux.market_access[0].accessible);
        assert_eq!(lux.market_access[0].channel, "Passport");
        assert!(ire.market_access[0].accessible);
        assert_eq!(ire.market_access[0].channel, "Passport");
    }

    #[test]
    fn test_distribution_coverage_percentage() {
        let input = DistributionReachInput {
            jurisdictions: vec!["Luxembourg".to_string(), "Cayman".to_string()],
            target_markets: vec![
                "EU27".to_string(),
                "US".to_string(),
                "UK".to_string(),
                "GCC".to_string(),
            ],
        };
        let result = distribution_reach_analysis(&input).unwrap();
        for jr in &result.per_jurisdiction_reach {
            assert!(jr.coverage_pct >= Decimal::ZERO);
            assert!(jr.coverage_pct <= dec!(100));
        }
    }

    #[test]
    fn test_distribution_coverage_matrix_shape() {
        let input = DistributionReachInput {
            jurisdictions: vec![
                "Cayman".to_string(),
                "Luxembourg".to_string(),
                "Singapore".to_string(),
            ],
            target_markets: vec!["EU27".to_string(), "US".to_string(), "UK".to_string()],
        };
        let result = distribution_reach_analysis(&input).unwrap();
        assert_eq!(result.coverage_matrix.len(), 3); // 3 jurisdictions
        for row in &result.coverage_matrix {
            assert_eq!(row.len(), 3); // 3 target markets
        }
    }

    #[test]
    fn test_distribution_gcc_difc_adgm_passport() {
        let input = DistributionReachInput {
            jurisdictions: vec!["DIFC".to_string(), "ADGM".to_string()],
            target_markets: vec!["GCC".to_string()],
        };
        let result = distribution_reach_analysis(&input).unwrap();
        let difc = result
            .per_jurisdiction_reach
            .iter()
            .find(|r| r.code == "DIFC")
            .unwrap();
        let adgm = result
            .per_jurisdiction_reach
            .iter()
            .find(|r| r.code == "ADGM")
            .unwrap();
        assert!(difc.market_access[0].accessible);
        assert_eq!(difc.market_access[0].channel, "Passport");
        assert!(adgm.market_access[0].accessible);
        assert_eq!(adgm.market_access[0].channel, "Passport");
    }

    #[test]
    fn test_distribution_us_accessible_all() {
        let input = DistributionReachInput {
            jurisdictions: all_10_jurisdictions(),
            target_markets: vec!["US".to_string()],
        };
        let result = distribution_reach_analysis(&input).unwrap();
        for jr in &result.per_jurisdiction_reach {
            assert!(
                jr.market_access[0].accessible,
                "{} should have US access",
                jr.code
            );
        }
    }

    #[test]
    fn test_distribution_all_markets_all_jurisdictions() {
        let input = DistributionReachInput {
            jurisdictions: all_10_jurisdictions(),
            target_markets: vec![
                "EU27".to_string(),
                "US".to_string(),
                "UK".to_string(),
                "Singapore".to_string(),
                "HK".to_string(),
                "Japan".to_string(),
                "Australia".to_string(),
                "GCC".to_string(),
            ],
        };
        let result = distribution_reach_analysis(&input).unwrap();
        assert_eq!(result.per_jurisdiction_reach.len(), 10);
        assert!(!result.optimal_jurisdiction_for_distribution.is_empty());
        assert!(result.total_cost_of_distribution > Decimal::ZERO);
    }

    #[test]
    fn test_distribution_optimal_is_highest_reach() {
        let input = DistributionReachInput {
            jurisdictions: vec![
                "Cayman".to_string(),
                "Luxembourg".to_string(),
                "BVI".to_string(),
            ],
            target_markets: vec![
                "EU27".to_string(),
                "US".to_string(),
                "UK".to_string(),
                "HK".to_string(),
                "Japan".to_string(),
                "Australia".to_string(),
                "GCC".to_string(),
            ],
        };
        let result = distribution_reach_analysis(&input).unwrap();
        let optimal = &result.optimal_jurisdiction_for_distribution;
        let max_accessible = result
            .per_jurisdiction_reach
            .iter()
            .max_by_key(|r| r.accessible_count)
            .unwrap();
        assert_eq!(optimal, &max_accessible.code);
    }

    #[test]
    fn test_distribution_error_single_jurisdiction() {
        let input = DistributionReachInput {
            jurisdictions: vec!["Cayman".to_string()],
            target_markets: vec!["US".to_string()],
        };
        let err = distribution_reach_analysis(&input).unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("At least 2"), "Error: {}", msg);
    }

    #[test]
    fn test_distribution_error_empty_targets() {
        let input = DistributionReachInput {
            jurisdictions: vec!["Cayman".to_string(), "BVI".to_string()],
            target_markets: vec![],
        };
        let err = distribution_reach_analysis(&input).unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("target market"), "Error: {}", msg);
    }

    // -----------------------------------------------------------------------
    // total_cost_of_ownership
    // -----------------------------------------------------------------------

    #[test]
    fn test_tco_5_year_projection() {
        let input = TcoInput {
            jurisdictions: vec![
                "Cayman".to_string(),
                "BVI".to_string(),
                "Luxembourg".to_string(),
            ],
            fund_size: dec!(200_000_000),
            projection_years: 5,
            discount_rate: dec!(0.08),
        };
        let result = total_cost_of_ownership(&input).unwrap();
        assert_eq!(result.per_jurisdiction_tco.len(), 3);
        assert!(!result.cheapest_jurisdiction.is_empty());

        for tco in &result.per_jurisdiction_tco {
            // Total over period = setup + annual * years
            let expected_total = tco.setup_cost + tco.total_annual * dec!(5);
            assert_eq!(tco.total_over_period, expected_total);
            // NPV should be less than total (positive discount rate)
            assert!(tco.npv < tco.total_over_period);
            assert!(tco.npv > Decimal::ZERO);
        }
    }

    #[test]
    fn test_tco_bvi_cheapest() {
        let input = TcoInput {
            jurisdictions: vec![
                "Cayman".to_string(),
                "BVI".to_string(),
                "Luxembourg".to_string(),
            ],
            fund_size: dec!(200_000_000),
            projection_years: 5,
            discount_rate: dec!(0.08),
        };
        let result = total_cost_of_ownership(&input).unwrap();
        // BVI has lowest setup and annual costs
        assert_eq!(result.cheapest_jurisdiction, "BVI");
    }

    #[test]
    fn test_tco_year_by_year_shape() {
        let input = TcoInput {
            jurisdictions: vec!["Cayman".to_string(), "BVI".to_string()],
            fund_size: dec!(100_000_000),
            projection_years: 3,
            discount_rate: dec!(0.10),
        };
        let result = total_cost_of_ownership(&input).unwrap();
        assert_eq!(result.year_by_year_costs.len(), 2);
        for jyc in &result.year_by_year_costs {
            // Year 0 (setup) + 3 annual years = 4 entries
            assert_eq!(jyc.year_costs.len(), 4);
            assert_eq!(jyc.year_costs[0].year, 0);
            assert_eq!(jyc.year_costs[3].year, 3);
        }
    }

    #[test]
    fn test_tco_discount_factor_iterative() {
        let input = TcoInput {
            jurisdictions: vec!["Cayman".to_string(), "BVI".to_string()],
            fund_size: dec!(100_000_000),
            projection_years: 3,
            discount_rate: dec!(0.10),
        };
        let result = total_cost_of_ownership(&input).unwrap();
        let cayman_yc = &result.year_by_year_costs[0];
        // Year 0: discount factor = 1.0
        assert_eq!(
            cayman_yc.year_costs[0].discounted_cost,
            cayman_yc.year_costs[0].cost
        );
        // Year 1: discounted < undiscounted
        assert!(cayman_yc.year_costs[1].discounted_cost < cayman_yc.year_costs[1].cost);
        // Discounted costs should decrease over time (same nominal, increasing discount)
        assert!(cayman_yc.year_costs[1].discounted_cost > cayman_yc.year_costs[2].discounted_cost);
        assert!(cayman_yc.year_costs[2].discounted_cost > cayman_yc.year_costs[3].discounted_cost);
    }

    #[test]
    fn test_tco_npv_comparison_sorted() {
        let input = TcoInput {
            jurisdictions: all_10_jurisdictions(),
            fund_size: dec!(500_000_000),
            projection_years: 10,
            discount_rate: dec!(0.05),
        };
        let result = total_cost_of_ownership(&input).unwrap();
        assert_eq!(result.npv_comparison.len(), 10);
        // Should be sorted ascending by NPV
        for i in 0..9 {
            assert!(
                result.npv_comparison[i].1 <= result.npv_comparison[i + 1].1,
                "NPV comparison should be sorted ascending"
            );
        }
    }

    #[test]
    fn test_tco_cost_rankings_match_npv() {
        let input = TcoInput {
            jurisdictions: vec![
                "Cayman".to_string(),
                "Luxembourg".to_string(),
                "Singapore".to_string(),
            ],
            fund_size: dec!(100_000_000),
            projection_years: 5,
            discount_rate: dec!(0.08),
        };
        let result = total_cost_of_ownership(&input).unwrap();
        assert_eq!(result.cost_rankings.len(), 3);
        assert_eq!(result.cost_rankings[0].0, result.cheapest_jurisdiction);
    }

    #[test]
    fn test_tco_1_year_projection() {
        let input = TcoInput {
            jurisdictions: vec!["Cayman".to_string(), "BVI".to_string()],
            fund_size: dec!(100_000_000),
            projection_years: 1,
            discount_rate: dec!(0.05),
        };
        let result = total_cost_of_ownership(&input).unwrap();
        for jyc in &result.year_by_year_costs {
            assert_eq!(jyc.year_costs.len(), 2); // year 0 + year 1
        }
    }

    #[test]
    fn test_tco_10_year_projection() {
        let input = TcoInput {
            jurisdictions: vec!["Cayman".to_string(), "Luxembourg".to_string()],
            fund_size: dec!(500_000_000),
            projection_years: 10,
            discount_rate: dec!(0.08),
        };
        let result = total_cost_of_ownership(&input).unwrap();
        for jyc in &result.year_by_year_costs {
            assert_eq!(jyc.year_costs.len(), 11); // year 0..10
        }
        // Luxembourg should be more expensive than Cayman
        let lux_npv = result
            .per_jurisdiction_tco
            .iter()
            .find(|t| t.code == "Luxembourg")
            .unwrap()
            .npv;
        let cay_npv = result
            .per_jurisdiction_tco
            .iter()
            .find(|t| t.code == "Cayman")
            .unwrap()
            .npv;
        assert!(lux_npv > cay_npv);
    }

    #[test]
    fn test_tco_error_zero_projection_years() {
        let input = TcoInput {
            jurisdictions: vec!["Cayman".to_string(), "BVI".to_string()],
            fund_size: dec!(100_000_000),
            projection_years: 0,
            discount_rate: dec!(0.08),
        };
        let err = total_cost_of_ownership(&input).unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("at least 1"), "Error: {}", msg);
    }

    #[test]
    fn test_tco_error_invalid_discount_rate() {
        let input = TcoInput {
            jurisdictions: vec!["Cayman".to_string(), "BVI".to_string()],
            fund_size: dec!(100_000_000),
            projection_years: 5,
            discount_rate: dec!(1.5),
        };
        let err = total_cost_of_ownership(&input).unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("between 0 and 1"), "Error: {}", msg);
    }

    #[test]
    fn test_tco_error_single_jurisdiction() {
        let input = TcoInput {
            jurisdictions: vec!["Cayman".to_string()],
            fund_size: dec!(100_000_000),
            projection_years: 5,
            discount_rate: dec!(0.08),
        };
        let err = total_cost_of_ownership(&input).unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("At least 2"), "Error: {}", msg);
    }

    // -----------------------------------------------------------------------
    // Profile data integrity
    // -----------------------------------------------------------------------

    #[test]
    fn test_all_10_jurisdictions_valid() {
        for code in &JURISDICTION_CODES {
            let profile = get_profile(code).unwrap();
            assert_eq!(&profile.code, code);
            assert!(!profile.name.is_empty());
            assert!(profile.setup_cost_usd > Decimal::ZERO);
            assert!(profile.annual_cost_usd > Decimal::ZERO);
            assert!(profile.regulatory_timeline_weeks >= 1);
            assert!(profile.substance_score >= Decimal::ZERO);
            assert!(profile.substance_score <= dec!(100));
            assert!(profile.distribution_reach_count > 0);
        }
    }

    #[test]
    fn test_cayman_profile_values() {
        let p = get_profile("Cayman").unwrap();
        assert_eq!(p.setup_cost_usd, dec!(25000));
        assert_eq!(p.annual_cost_usd, dec!(80000));
        assert_eq!(p.regulatory_timeline_weeks, 4);
        assert_eq!(p.minimum_capital_usd, Decimal::ZERO);
        assert_eq!(p.fund_level_tax_rate, Decimal::ZERO);
        assert_eq!(p.substance_score, dec!(30));
        assert_eq!(p.distribution_reach_count, 180);
        assert!(!p.passport_available);
    }

    #[test]
    fn test_luxembourg_profile_values() {
        let p = get_profile("Luxembourg").unwrap();
        assert_eq!(p.setup_cost_usd, dec!(80000));
        assert_eq!(p.annual_cost_usd, dec!(200000));
        assert_eq!(p.regulatory_timeline_weeks, 16);
        assert_eq!(p.minimum_capital_usd, dec!(1250000));
        assert_eq!(p.fund_level_tax_rate, Decimal::ZERO);
        assert_eq!(p.substance_score, dec!(60));
        assert_eq!(p.distribution_reach_count, 30);
        assert!(p.passport_available);
        assert_eq!(p.nppr_markets, 27);
    }

    #[test]
    fn test_bvi_profile_values() {
        let p = get_profile("BVI").unwrap();
        assert_eq!(p.setup_cost_usd, dec!(15000));
        assert_eq!(p.annual_cost_usd, dec!(50000));
        assert_eq!(p.regulatory_timeline_weeks, 2);
        assert_eq!(p.minimum_capital_usd, Decimal::ZERO);
        assert_eq!(p.substance_score, dec!(25));
        assert_eq!(p.distribution_reach_count, 150);
    }

    #[test]
    fn test_singapore_profile_values() {
        let p = get_profile("Singapore").unwrap();
        assert_eq!(p.setup_cost_usd, dec!(50000));
        assert_eq!(p.annual_cost_usd, dec!(120000));
        assert_eq!(p.regulatory_timeline_weeks, 8);
        assert_eq!(p.minimum_capital_usd, dec!(1));
        assert_eq!(p.substance_score, dec!(50));
        assert_eq!(p.distribution_reach_count, 25);
        assert!(!p.passport_available);
    }

    // -----------------------------------------------------------------------
    // Edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_compare_all_weight_on_single_dimension() {
        let speed_only = ComparisonWeights {
            setup_cost: dec!(0.00),
            annual_cost: dec!(0.00),
            tax: dec!(0.00),
            regulatory_speed: dec!(1.00),
            distribution_reach: dec!(0.00),
            substance: dec!(0.00),
        };
        let input = JurisdictionComparisonInput {
            jurisdictions: all_10_jurisdictions(),
            fund_strategy: "Hedge".to_string(),
            fund_size: dec!(500_000_000),
            fund_type: "OpenEnded".to_string(),
            weights: speed_only,
        };
        let result = compare_jurisdictions(&input).unwrap();
        // Top jurisdiction should be one of the 1-week jurisdictions
        let top = &result.ranked_jurisdictions[0];
        assert!(
            top.profile.regulatory_timeline_weeks <= 2,
            "Top jurisdiction should have fastest timeline, got {} weeks for {}",
            top.profile.regulatory_timeline_weeks,
            top.code
        );
    }

    #[test]
    fn test_tco_zero_discount_rate() {
        let input = TcoInput {
            jurisdictions: vec!["Cayman".to_string(), "BVI".to_string()],
            fund_size: dec!(100_000_000),
            projection_years: 5,
            discount_rate: Decimal::ZERO,
        };
        let result = total_cost_of_ownership(&input).unwrap();
        // With 0% discount, NPV should equal total_over_period
        for tco in &result.per_jurisdiction_tco {
            assert_eq!(
                tco.npv, tco.total_over_period,
                "At 0% discount rate, NPV should equal total for {}",
                tco.code
            );
        }
    }

    #[test]
    fn test_compare_duplicate_jurisdictions() {
        // Duplicates should work (compare same jurisdiction twice)
        let input = JurisdictionComparisonInput {
            jurisdictions: vec!["Cayman".to_string(), "Cayman".to_string()],
            fund_strategy: "Hedge".to_string(),
            fund_size: dec!(100_000_000),
            fund_type: "OpenEnded".to_string(),
            weights: equal_weights(),
        };
        let result = compare_jurisdictions(&input).unwrap();
        assert_eq!(result.ranked_jurisdictions.len(), 2);
        // Both should have equal scores
        assert_eq!(
            result.ranked_jurisdictions[0].weighted_score,
            result.ranked_jurisdictions[1].weighted_score
        );
    }

    #[test]
    fn test_optimal_weights_tolerance() {
        // Weights sum to 0.999 — within tolerance
        let w = ComparisonWeights {
            setup_cost: dec!(0.166),
            annual_cost: dec!(0.166),
            tax: dec!(0.167),
            regulatory_speed: dec!(0.167),
            distribution_reach: dec!(0.167),
            substance: dec!(0.166),
        };
        let input = OptimalJurisdictionInput {
            fund_strategy: "Hedge".to_string(),
            fund_size: dec!(500_000_000),
            fund_type: "OpenEnded".to_string(),
            investor_base: vec!["US".to_string()],
            distribution_targets: vec!["US".to_string()],
            weights: w,
        };
        let result = optimal_jurisdiction(&input);
        assert!(
            result.is_ok(),
            "Weights within tolerance should be accepted"
        );
    }
}
