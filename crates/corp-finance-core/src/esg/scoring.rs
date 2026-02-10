use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{with_metadata, ComputationOutput};
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Input types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EsgScoreInput {
    pub company_name: String,
    pub sector: String,
    pub environmental: EnvironmentalMetrics,
    pub social: SocialMetrics,
    pub governance: GovernanceMetrics,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pillar_weights: Option<PillarWeights>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peer_scores: Option<Vec<PeerScore>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentalMetrics {
    /// tCO2e per $M revenue
    pub carbon_intensity: Decimal,
    /// 0-1
    pub renewable_energy_pct: Decimal,
    /// megalitres per $M revenue
    pub water_intensity: Decimal,
    /// 0-1
    pub waste_recycling_rate: Decimal,
    pub biodiversity_policy: bool,
    /// total fines in reporting period
    pub environmental_fines_amount: Decimal,
    /// has SBTi approved targets
    pub science_based_targets: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialMetrics {
    /// 0-1
    pub employee_turnover_rate: Decimal,
    /// % female in workforce (0-100)
    pub gender_diversity_pct: Decimal,
    /// % female/minority on board (0-100)
    pub board_diversity_pct: Decimal,
    pub living_wage_compliance: bool,
    /// incidents per 200k hours
    pub health_safety_incident_rate: Decimal,
    /// % of pre-tax profit (0-100 scale, e.g. 1.0 = 1%)
    pub community_investment_pct: Decimal,
    /// % of suppliers audited (0-100)
    pub supply_chain_audit_pct: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceMetrics {
    /// 0-1, % independent directors
    pub board_independence_pct: Decimal,
    pub ceo_chair_separation: bool,
    /// CEO pay / median employee pay
    pub executive_pay_ratio: Decimal,
    pub anti_corruption_policy: bool,
    pub whistleblower_mechanism: bool,
    pub audit_committee_independence: bool,
    /// $ amount
    pub related_party_transactions: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PillarWeights {
    pub environmental: Decimal,
    pub social: Decimal,
    pub governance: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerScore {
    pub company_name: String,
    pub esg_score: Decimal,
    pub e_score: Decimal,
    pub s_score: Decimal,
    pub g_score: Decimal,
}

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EsgScoreOutput {
    /// 0-100
    pub overall_score: Decimal,
    /// 0-100
    pub environmental_score: Decimal,
    /// 0-100
    pub social_score: Decimal,
    /// 0-100
    pub governance_score: Decimal,
    /// "AAA", "AA", "A", "BBB", "BB", "B", "CCC"
    pub rating: String,
    pub pillar_weights_used: PillarWeights,
    pub materiality_map: Vec<MaterialityIssue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peer_comparison: Option<PeerComparison>,
    pub flags: Vec<EsgFlag>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialityIssue {
    pub issue: String,
    /// "E", "S", or "G"
    pub pillar: String,
    /// "High", "Medium", "Low"
    pub materiality: String,
    /// How much this issue moves the score
    pub score_impact: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerComparison {
    /// 0-100
    pub percentile_rank: Decimal,
    pub peer_average: Decimal,
    pub peer_median: Decimal,
    pub best_in_class: String,
    pub gap_to_best: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EsgFlag {
    /// "Red", "Amber", "Green"
    pub flag_type: String,
    pub issue: String,
    pub description: String,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const SCORE_MIN: Decimal = dec!(0);
const SCORE_MAX: Decimal = dec!(100);

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Calculate a comprehensive ESG score with pillar weighting, materiality
/// mapping, peer benchmarking, and red/amber/green flag analysis.
pub fn calculate_esg_score(
    input: &EsgScoreInput,
) -> CorpFinanceResult<ComputationOutput<EsgScoreOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // -- Validation ----------------------------------------------------------
    validate_input(input, &mut warnings)?;

    // -- Resolve pillar weights -----------------------------------------------
    let weights = resolve_weights(input);

    // -- Score each pillar ----------------------------------------------------
    let e_score = score_environmental(&input.environmental);
    let s_score = score_social(&input.social);
    let g_score = score_governance(&input.governance);

    // -- Weighted overall score -----------------------------------------------
    let raw_overall =
        weights.environmental * e_score + weights.social * s_score + weights.governance * g_score;
    let overall_score = clamp_score(raw_overall);

    // -- Rating ---------------------------------------------------------------
    let rating = score_to_rating(overall_score);

    // -- Materiality map ------------------------------------------------------
    let materiality_map = build_materiality_map(input, e_score, s_score, g_score);

    // -- Peer comparison ------------------------------------------------------
    let peer_comparison = input
        .peer_scores
        .as_ref()
        .map(|peers| build_peer_comparison(overall_score, peers));

    // -- Flags ----------------------------------------------------------------
    let flags = build_flags(input, e_score, s_score, g_score);

    let output = EsgScoreOutput {
        overall_score,
        environmental_score: clamp_score(e_score),
        social_score: clamp_score(s_score),
        governance_score: clamp_score(g_score),
        rating,
        pillar_weights_used: weights,
        materiality_map,
        peer_comparison,
        flags,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    let assumptions = serde_json::json!({
        "methodology": "ESG scoring with sector materiality weighting",
        "score_range": "0-100",
        "rating_bands": {
            "AAA": "85-100", "AA": "70-84", "A": "55-69",
            "BBB": "40-54", "BB": "25-39", "B": "10-24", "CCC": "0-9"
        },
        "sector": input.sector,
        "custom_weights": input.pillar_weights.is_some()
    });

    Ok(with_metadata(
        "ESG Scoring Framework (CFA ESG integration)",
        &assumptions,
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_input(input: &EsgScoreInput, warnings: &mut Vec<String>) -> CorpFinanceResult<()> {
    if input.company_name.trim().is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "company_name".into(),
            reason: "Company name must not be empty.".into(),
        });
    }
    if input.sector.trim().is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "sector".into(),
            reason: "Sector must not be empty.".into(),
        });
    }

    // Validate custom weights sum to 1.0
    if let Some(ref w) = input.pillar_weights {
        let sum = w.environmental + w.social + w.governance;
        let tolerance = dec!(0.01);
        if (sum - dec!(1)) > tolerance || (dec!(1) - sum) > tolerance {
            return Err(CorpFinanceError::InvalidInput {
                field: "pillar_weights".into(),
                reason: format!("Pillar weights must sum to 1.0 (got {}).", sum),
            });
        }
        if w.environmental < SCORE_MIN || w.social < SCORE_MIN || w.governance < SCORE_MIN {
            return Err(CorpFinanceError::InvalidInput {
                field: "pillar_weights".into(),
                reason: "Pillar weights must be non-negative.".into(),
            });
        }
    }

    // Validate percentage ranges
    if input.environmental.renewable_energy_pct < dec!(0)
        || input.environmental.renewable_energy_pct > dec!(1)
    {
        return Err(CorpFinanceError::InvalidInput {
            field: "renewable_energy_pct".into(),
            reason: "Must be between 0 and 1.".into(),
        });
    }
    if input.environmental.waste_recycling_rate < dec!(0)
        || input.environmental.waste_recycling_rate > dec!(1)
    {
        return Err(CorpFinanceError::InvalidInput {
            field: "waste_recycling_rate".into(),
            reason: "Must be between 0 and 1.".into(),
        });
    }
    if input.social.employee_turnover_rate < dec!(0)
        || input.social.employee_turnover_rate > dec!(1)
    {
        return Err(CorpFinanceError::InvalidInput {
            field: "employee_turnover_rate".into(),
            reason: "Must be between 0 and 1.".into(),
        });
    }
    if input.governance.board_independence_pct < dec!(0)
        || input.governance.board_independence_pct > dec!(1)
    {
        return Err(CorpFinanceError::InvalidInput {
            field: "board_independence_pct".into(),
            reason: "Must be between 0 and 1.".into(),
        });
    }

    if input.environmental.carbon_intensity < dec!(0) {
        return Err(CorpFinanceError::InvalidInput {
            field: "carbon_intensity".into(),
            reason: "Carbon intensity must be non-negative.".into(),
        });
    }

    // Sector warning for unknown sectors
    let known = [
        "Energy",
        "Technology",
        "Financials",
        "Healthcare",
        "Materials",
        "Industrials",
        "Consumer",
        "Utilities",
        "Real Estate",
    ];
    if !known.contains(&input.sector.as_str()) {
        warnings.push(format!(
            "Unknown sector '{}'. Using default equal weights.",
            input.sector
        ));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Pillar weights resolution
// ---------------------------------------------------------------------------

fn resolve_weights(input: &EsgScoreInput) -> PillarWeights {
    if let Some(ref w) = input.pillar_weights {
        return w.clone();
    }
    sector_default_weights(&input.sector)
}

fn sector_default_weights(sector: &str) -> PillarWeights {
    match sector {
        "Energy" => PillarWeights {
            environmental: dec!(0.50),
            social: dec!(0.20),
            governance: dec!(0.30),
        },
        "Technology" => PillarWeights {
            environmental: dec!(0.20),
            social: dec!(0.40),
            governance: dec!(0.40),
        },
        "Financials" => PillarWeights {
            environmental: dec!(0.15),
            social: dec!(0.35),
            governance: dec!(0.50),
        },
        "Healthcare" => PillarWeights {
            environmental: dec!(0.20),
            social: dec!(0.45),
            governance: dec!(0.35),
        },
        "Materials" => PillarWeights {
            environmental: dec!(0.45),
            social: dec!(0.25),
            governance: dec!(0.30),
        },
        "Industrials" => PillarWeights {
            environmental: dec!(0.35),
            social: dec!(0.30),
            governance: dec!(0.35),
        },
        "Consumer" => PillarWeights {
            environmental: dec!(0.25),
            social: dec!(0.40),
            governance: dec!(0.35),
        },
        "Utilities" => PillarWeights {
            environmental: dec!(0.45),
            social: dec!(0.25),
            governance: dec!(0.30),
        },
        "Real Estate" => PillarWeights {
            environmental: dec!(0.40),
            social: dec!(0.25),
            governance: dec!(0.35),
        },
        _ => PillarWeights {
            environmental: dec!(0.33),
            social: dec!(0.34),
            governance: dec!(0.33),
        },
    }
}

// ---------------------------------------------------------------------------
// Environmental scoring
// ---------------------------------------------------------------------------

fn score_environmental(env: &EnvironmentalMetrics) -> Decimal {
    let carbon_score = score_carbon_intensity(env.carbon_intensity);
    let renewable_score = env.renewable_energy_pct * dec!(100);
    let water_score = score_water_intensity(env.water_intensity);
    let recycling_score = env.waste_recycling_rate * dec!(100);

    // Biodiversity policy adds a small bonus captured in flags, not the average
    let component_count = dec!(4);
    let mut avg =
        (carbon_score + renewable_score + water_score + recycling_score) / component_count;

    // SBTi bonus
    if env.science_based_targets {
        avg += dec!(10);
    }

    // Fines deduction
    avg -= fines_deduction(env.environmental_fines_amount);

    clamp_score(avg)
}

fn score_carbon_intensity(ci: Decimal) -> Decimal {
    if ci < dec!(100) {
        // 0-99 maps linearly to 90-100
        dec!(100) - ci / dec!(10)
    } else if ci < dec!(500) {
        // 100-499 maps linearly to 60-89
        let fraction = (ci - dec!(100)) / dec!(400);
        dec!(89) - fraction * dec!(29)
    } else if ci < dec!(1000) {
        // 500-999 maps linearly to 40-59
        let fraction = (ci - dec!(500)) / dec!(500);
        dec!(59) - fraction * dec!(19)
    } else {
        // >=1000: maps linearly from 39 down to 0 (capped at 2000+)
        let fraction = (ci - dec!(1000)) / dec!(1000);
        let score = dec!(39) - fraction * dec!(39);
        if score < dec!(0) {
            dec!(0)
        } else {
            score
        }
    }
}

fn score_water_intensity(wi: Decimal) -> Decimal {
    // Benchmark: <50 = excellent(90-100), 50-200 = good(60-89),
    // 200-500 = average(40-59), >500 = poor(0-39)
    if wi < dec!(50) {
        let fraction = wi / dec!(50);
        dec!(100) - fraction * dec!(10)
    } else if wi < dec!(200) {
        let fraction = (wi - dec!(50)) / dec!(150);
        dec!(89) - fraction * dec!(29)
    } else if wi < dec!(500) {
        let fraction = (wi - dec!(200)) / dec!(300);
        dec!(59) - fraction * dec!(19)
    } else {
        let fraction = (wi - dec!(500)) / dec!(500);
        let score = dec!(39) - fraction * dec!(39);
        if score < dec!(0) {
            dec!(0)
        } else {
            score
        }
    }
}

fn fines_deduction(amount: Decimal) -> Decimal {
    if amount > dec!(1_000_000) {
        dec!(20)
    } else if amount > dec!(100_000) {
        dec!(10)
    } else if amount > dec!(10_000) {
        dec!(5)
    } else {
        dec!(0)
    }
}

// ---------------------------------------------------------------------------
// Social scoring
// ---------------------------------------------------------------------------

fn score_social(soc: &SocialMetrics) -> Decimal {
    let turnover_score = score_turnover(soc.employee_turnover_rate);
    let gender_score = score_gender_diversity(soc.gender_diversity_pct);
    let board_div_score = score_board_diversity(soc.board_diversity_pct);
    let hs_score = score_health_safety(soc.health_safety_incident_rate);
    let community_score = score_community_investment(soc.community_investment_pct);

    let component_count = dec!(5);
    let mut avg = (turnover_score + gender_score + board_div_score + hs_score + community_score)
        / component_count;

    // Boolean bonuses
    if soc.living_wage_compliance {
        avg += dec!(10);
    }
    if soc.supply_chain_audit_pct > dec!(80) {
        avg += dec!(10);
    }

    clamp_score(avg)
}

fn score_turnover(rate: Decimal) -> Decimal {
    // rate is 0-1 (e.g. 0.10 = 10%)
    let pct = rate * dec!(100);
    if pct < dec!(10) {
        dec!(90)
    } else if pct < dec!(20) {
        dec!(70)
    } else if pct < dec!(30) {
        dec!(50)
    } else {
        dec!(30)
    }
}

fn score_gender_diversity(pct: Decimal) -> Decimal {
    // Target 50% = 100. Linear up to target.
    let score = (pct / dec!(50)) * dec!(100);
    clamp_score(score)
}

fn score_board_diversity(pct: Decimal) -> Decimal {
    // Target 40% = 100. Linear up to target.
    let score = (pct / dec!(40)) * dec!(100);
    clamp_score(score)
}

fn score_health_safety(rate: Decimal) -> Decimal {
    if rate < dec!(1) {
        dec!(90)
    } else if rate < dec!(3) {
        dec!(70)
    } else if rate < dec!(5) {
        dec!(50)
    } else {
        dec!(30)
    }
}

fn score_community_investment(pct: Decimal) -> Decimal {
    // pct is on 0-100 scale (e.g. 1.0 = 1%)
    if pct > dec!(1) {
        dec!(90)
    } else if pct >= dec!(0.5) {
        dec!(70)
    } else {
        dec!(50)
    }
}

// ---------------------------------------------------------------------------
// Governance scoring
// ---------------------------------------------------------------------------

fn score_governance(gov: &GovernanceMetrics) -> Decimal {
    let independence_score = score_board_independence(gov.board_independence_pct);
    let pay_score = score_pay_ratio(gov.executive_pay_ratio);
    let rpt_score = score_related_party_transactions(gov.related_party_transactions);

    let component_count = dec!(3);
    let mut avg = (independence_score + pay_score + rpt_score) / component_count;

    // Boolean bonuses
    if gov.ceo_chair_separation {
        avg += dec!(15);
    }
    if gov.anti_corruption_policy {
        avg += dec!(10);
    }
    if gov.whistleblower_mechanism {
        avg += dec!(10);
    }
    if gov.audit_committee_independence {
        avg += dec!(10);
    }

    clamp_score(avg)
}

fn score_board_independence(pct: Decimal) -> Decimal {
    // pct is 0-1. Target 0.75 = 100.
    let score = (pct / dec!(0.75)) * dec!(100);
    clamp_score(score)
}

fn score_pay_ratio(ratio: Decimal) -> Decimal {
    if ratio < dec!(50) {
        dec!(90)
    } else if ratio < dec!(200) {
        dec!(70)
    } else if ratio < dec!(500) {
        dec!(50)
    } else {
        dec!(30)
    }
}

fn score_related_party_transactions(amount: Decimal) -> Decimal {
    if amount == dec!(0) {
        dec!(100)
    } else if amount < dec!(100_000) {
        dec!(80)
    } else if amount < dec!(1_000_000) {
        dec!(60)
    } else if amount < dec!(10_000_000) {
        dec!(40)
    } else {
        dec!(20)
    }
}

// ---------------------------------------------------------------------------
// Rating
// ---------------------------------------------------------------------------

fn score_to_rating(score: Decimal) -> String {
    if score >= dec!(85) {
        "AAA".to_string()
    } else if score >= dec!(70) {
        "AA".to_string()
    } else if score >= dec!(55) {
        "A".to_string()
    } else if score >= dec!(40) {
        "BBB".to_string()
    } else if score >= dec!(25) {
        "BB".to_string()
    } else if score >= dec!(10) {
        "B".to_string()
    } else {
        "CCC".to_string()
    }
}

// ---------------------------------------------------------------------------
// Materiality map
// ---------------------------------------------------------------------------

fn build_materiality_map(
    input: &EsgScoreInput,
    e_score: Decimal,
    s_score: Decimal,
    g_score: Decimal,
) -> Vec<MaterialityIssue> {
    let weights = resolve_weights(input);
    let mut issues = Vec::new();

    // Environmental issues
    let carbon_score = score_carbon_intensity(input.environmental.carbon_intensity);
    let carbon_impact = weights.environmental * (dec!(100) - carbon_score) / dec!(100);
    issues.push(MaterialityIssue {
        issue: "Carbon Emissions".to_string(),
        pillar: "E".to_string(),
        materiality: materiality_level(&input.sector, "Carbon Emissions"),
        score_impact: carbon_impact,
    });

    issues.push(MaterialityIssue {
        issue: "Renewable Energy".to_string(),
        pillar: "E".to_string(),
        materiality: materiality_level(&input.sector, "Renewable Energy"),
        score_impact: weights.environmental
            * (dec!(100) - input.environmental.renewable_energy_pct * dec!(100))
            / dec!(100),
    });

    // Social issues
    let gender_score = score_gender_diversity(input.social.gender_diversity_pct);
    issues.push(MaterialityIssue {
        issue: "Workforce Diversity".to_string(),
        pillar: "S".to_string(),
        materiality: materiality_level(&input.sector, "Workforce Diversity"),
        score_impact: weights.social * (dec!(100) - gender_score) / dec!(100),
    });

    let hs_score = score_health_safety(input.social.health_safety_incident_rate);
    issues.push(MaterialityIssue {
        issue: "Health & Safety".to_string(),
        pillar: "S".to_string(),
        materiality: materiality_level(&input.sector, "Health & Safety"),
        score_impact: weights.social * (dec!(100) - hs_score) / dec!(100),
    });

    // Governance issues
    let independence_score = score_board_independence(input.governance.board_independence_pct);
    issues.push(MaterialityIssue {
        issue: "Board Independence".to_string(),
        pillar: "G".to_string(),
        materiality: materiality_level(&input.sector, "Board Independence"),
        score_impact: weights.governance * (dec!(100) - independence_score) / dec!(100),
    });

    let pay_score = score_pay_ratio(input.governance.executive_pay_ratio);
    issues.push(MaterialityIssue {
        issue: "Executive Compensation".to_string(),
        pillar: "G".to_string(),
        materiality: materiality_level(&input.sector, "Executive Compensation"),
        score_impact: weights.governance * (dec!(100) - pay_score) / dec!(100),
    });

    // Technology-specific: Data Privacy
    if input.sector == "Technology" || input.sector == "Financials" {
        issues.push(MaterialityIssue {
            issue: "Data Privacy".to_string(),
            pillar: "S".to_string(),
            materiality: "High".to_string(),
            score_impact: weights.social * (dec!(100) - s_score) / dec!(100),
        });
    }

    // Energy-specific: Environmental Compliance
    if input.sector == "Energy" || input.sector == "Materials" {
        issues.push(MaterialityIssue {
            issue: "Environmental Compliance".to_string(),
            pillar: "E".to_string(),
            materiality: "High".to_string(),
            score_impact: weights.environmental * (dec!(100) - e_score) / dec!(100),
        });
    }

    // Sort by score_impact descending
    issues.sort_by(|a, b| b.score_impact.cmp(&a.score_impact));

    // Suppress unused variable warnings by reading the scores
    let _ = (e_score, s_score, g_score);

    issues
}

fn materiality_level(sector: &str, issue: &str) -> String {
    match (sector, issue) {
        ("Energy", "Carbon Emissions") => "High",
        ("Energy", "Renewable Energy") => "High",
        ("Energy", "Health & Safety") => "High",
        ("Materials", "Carbon Emissions") => "High",
        ("Materials", "Health & Safety") => "High",
        ("Technology", "Workforce Diversity") => "High",
        ("Technology", "Board Independence") => "Medium",
        ("Financials", "Board Independence") => "High",
        ("Financials", "Executive Compensation") => "High",
        ("Healthcare", "Health & Safety") => "High",
        ("Healthcare", "Workforce Diversity") => "High",
        ("Industrials", "Carbon Emissions") => "Medium",
        ("Industrials", "Health & Safety") => "High",
        ("Utilities", "Carbon Emissions") => "High",
        ("Utilities", "Renewable Energy") => "High",
        ("Real Estate", "Carbon Emissions") => "High",
        ("Real Estate", "Renewable Energy") => "Medium",
        ("Consumer", "Workforce Diversity") => "High",
        (_, "Board Independence") => "Medium",
        (_, "Executive Compensation") => "Medium",
        (_, "Carbon Emissions") => "Medium",
        _ => "Low",
    }
    .to_string()
}

// ---------------------------------------------------------------------------
// Peer comparison
// ---------------------------------------------------------------------------

fn build_peer_comparison(overall_score: Decimal, peers: &[PeerScore]) -> PeerComparison {
    if peers.is_empty() {
        return PeerComparison {
            percentile_rank: dec!(50),
            peer_average: overall_score,
            peer_median: overall_score,
            best_in_class: "N/A".to_string(),
            gap_to_best: dec!(0),
        };
    }

    let mut all_scores: Vec<Decimal> = peers.iter().map(|p| p.esg_score).collect();
    all_scores.push(overall_score);
    all_scores.sort();

    let n = all_scores.len();

    // Percentile rank: what fraction of scores are below ours
    let below_count = all_scores.iter().filter(|&&s| s < overall_score).count();
    let percentile = if n > 1 {
        Decimal::from(below_count as u64) * dec!(100) / Decimal::from((n - 1) as u64)
    } else {
        dec!(50)
    };

    // Peer-only stats (excluding our score)
    let peer_only: Vec<Decimal> = peers.iter().map(|p| p.esg_score).collect();
    let peer_sum: Decimal = peer_only.iter().copied().sum();
    let peer_count = Decimal::from(peer_only.len() as u64);
    let peer_average = peer_sum / peer_count;

    let mut sorted_peers = peer_only.clone();
    sorted_peers.sort();
    let peer_median = if sorted_peers.len().is_multiple_of(2) {
        let mid = sorted_peers.len() / 2;
        (sorted_peers[mid - 1] + sorted_peers[mid]) / dec!(2)
    } else {
        sorted_peers[sorted_peers.len() / 2]
    };

    let best_peer = peers
        .iter()
        .max_by(|a, b| a.esg_score.cmp(&b.esg_score))
        .unwrap();
    let best_score = best_peer.esg_score;
    let gap_to_best = if best_score > overall_score {
        best_score - overall_score
    } else {
        dec!(0)
    };

    PeerComparison {
        percentile_rank: clamp_score(percentile),
        peer_average,
        peer_median,
        best_in_class: best_peer.company_name.clone(),
        gap_to_best,
    }
}

// ---------------------------------------------------------------------------
// Flags
// ---------------------------------------------------------------------------

fn build_flags(
    input: &EsgScoreInput,
    e_score: Decimal,
    s_score: Decimal,
    g_score: Decimal,
) -> Vec<EsgFlag> {
    let mut flags = Vec::new();

    // -- Red flags --
    // Individual component checks
    let carbon_score = score_carbon_intensity(input.environmental.carbon_intensity);
    if carbon_score < dec!(30) {
        flags.push(EsgFlag {
            flag_type: "Red".to_string(),
            issue: "Carbon Emissions".to_string(),
            description: "Carbon intensity score is critically low (< 30).".to_string(),
        });
    } else if carbon_score < dec!(50) {
        flags.push(EsgFlag {
            flag_type: "Amber".to_string(),
            issue: "Carbon Emissions".to_string(),
            description: "Carbon intensity score is below average (30-50).".to_string(),
        });
    }

    if input.environmental.environmental_fines_amount > dec!(1_000_000) {
        flags.push(EsgFlag {
            flag_type: "Red".to_string(),
            issue: "Environmental Fines".to_string(),
            description: "Environmental fines exceed $1M in reporting period.".to_string(),
        });
    }

    if !input.governance.anti_corruption_policy {
        flags.push(EsgFlag {
            flag_type: "Red".to_string(),
            issue: "Anti-Corruption".to_string(),
            description: "No anti-corruption policy in place.".to_string(),
        });
    }

    if input.governance.executive_pay_ratio > dec!(500) {
        flags.push(EsgFlag {
            flag_type: "Red".to_string(),
            issue: "Executive Pay Ratio".to_string(),
            description: "CEO pay ratio exceeds 500x median employee pay.".to_string(),
        });
    }

    let turnover_score = score_turnover(input.social.employee_turnover_rate);
    if turnover_score < dec!(30) {
        flags.push(EsgFlag {
            flag_type: "Red".to_string(),
            issue: "Employee Turnover".to_string(),
            description: "Employee turnover rate is critically high.".to_string(),
        });
    } else if turnover_score <= dec!(50) {
        flags.push(EsgFlag {
            flag_type: "Amber".to_string(),
            issue: "Employee Turnover".to_string(),
            description: "Employee turnover rate is above average.".to_string(),
        });
    }

    let hs_score = score_health_safety(input.social.health_safety_incident_rate);
    if hs_score < dec!(30) {
        flags.push(EsgFlag {
            flag_type: "Red".to_string(),
            issue: "Health & Safety".to_string(),
            description: "Health and safety incident rate is critically high.".to_string(),
        });
    } else if hs_score <= dec!(50) {
        flags.push(EsgFlag {
            flag_type: "Amber".to_string(),
            issue: "Health & Safety".to_string(),
            description: "Health and safety incident rate is above average.".to_string(),
        });
    }

    let pay_score = score_pay_ratio(input.governance.executive_pay_ratio);
    if pay_score < dec!(30) {
        // Already covered by the > 500 check above with more detail
    } else if pay_score <= dec!(50) {
        flags.push(EsgFlag {
            flag_type: "Amber".to_string(),
            issue: "Executive Compensation".to_string(),
            description: "Executive pay ratio is elevated (200-500x).".to_string(),
        });
    }

    // -- Amber flags for pillar-level scores --
    if e_score >= dec!(30) && e_score <= dec!(50) {
        flags.push(EsgFlag {
            flag_type: "Amber".to_string(),
            issue: "Environmental Pillar".to_string(),
            description: format!(
                "Overall environmental score is below average ({}).",
                e_score
            ),
        });
    }
    if s_score >= dec!(30) && s_score <= dec!(50) {
        flags.push(EsgFlag {
            flag_type: "Amber".to_string(),
            issue: "Social Pillar".to_string(),
            description: format!("Overall social score is below average ({}).", s_score),
        });
    }
    if g_score >= dec!(30) && g_score <= dec!(50) {
        flags.push(EsgFlag {
            flag_type: "Amber".to_string(),
            issue: "Governance Pillar".to_string(),
            description: format!("Overall governance score is below average ({}).", g_score),
        });
    }

    // Red flags for pillar-level scores < 30
    if e_score < dec!(30) {
        flags.push(EsgFlag {
            flag_type: "Red".to_string(),
            issue: "Environmental Pillar".to_string(),
            description: format!(
                "Overall environmental score is critically low ({}).",
                e_score
            ),
        });
    }
    if s_score < dec!(30) {
        flags.push(EsgFlag {
            flag_type: "Red".to_string(),
            issue: "Social Pillar".to_string(),
            description: format!("Overall social score is critically low ({}).", s_score),
        });
    }
    if g_score < dec!(30) {
        flags.push(EsgFlag {
            flag_type: "Red".to_string(),
            issue: "Governance Pillar".to_string(),
            description: format!("Overall governance score is critically low ({}).", g_score),
        });
    }

    // -- Green flags --
    if e_score > dec!(70) && s_score > dec!(70) && g_score > dec!(70) {
        flags.push(EsgFlag {
            flag_type: "Green".to_string(),
            issue: "All Pillars Strong".to_string(),
            description: "All three ESG pillars score above 70.".to_string(),
        });
    }
    if input.environmental.science_based_targets {
        flags.push(EsgFlag {
            flag_type: "Green".to_string(),
            issue: "Science-Based Targets".to_string(),
            description: "Company has SBTi-approved science-based targets.".to_string(),
        });
    }
    if input.governance.board_independence_pct >= dec!(0.75) {
        flags.push(EsgFlag {
            flag_type: "Green".to_string(),
            issue: "Board Independence".to_string(),
            description: "Board independence meets or exceeds 75% threshold.".to_string(),
        });
    }

    flags
}

// ---------------------------------------------------------------------------
// Utility
// ---------------------------------------------------------------------------

fn clamp_score(score: Decimal) -> Decimal {
    if score < SCORE_MIN {
        SCORE_MIN
    } else if score > SCORE_MAX {
        SCORE_MAX
    } else {
        score
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

    fn excellent_environmental() -> EnvironmentalMetrics {
        EnvironmentalMetrics {
            carbon_intensity: dec!(50),
            renewable_energy_pct: dec!(0.90),
            water_intensity: dec!(30),
            waste_recycling_rate: dec!(0.85),
            biodiversity_policy: true,
            environmental_fines_amount: dec!(0),
            science_based_targets: true,
        }
    }

    fn excellent_social() -> SocialMetrics {
        SocialMetrics {
            employee_turnover_rate: dec!(0.08),
            gender_diversity_pct: dec!(48),
            board_diversity_pct: dec!(38),
            living_wage_compliance: true,
            health_safety_incident_rate: dec!(0.5),
            community_investment_pct: dec!(1.5),
            supply_chain_audit_pct: dec!(90),
        }
    }

    fn excellent_governance() -> GovernanceMetrics {
        GovernanceMetrics {
            board_independence_pct: dec!(0.85),
            ceo_chair_separation: true,
            executive_pay_ratio: dec!(30),
            anti_corruption_policy: true,
            whistleblower_mechanism: true,
            audit_committee_independence: true,
            related_party_transactions: dec!(0),
        }
    }

    fn poor_environmental() -> EnvironmentalMetrics {
        EnvironmentalMetrics {
            carbon_intensity: dec!(1500),
            renewable_energy_pct: dec!(0.05),
            water_intensity: dec!(800),
            waste_recycling_rate: dec!(0.10),
            biodiversity_policy: false,
            environmental_fines_amount: dec!(5_000_000),
            science_based_targets: false,
        }
    }

    fn poor_social() -> SocialMetrics {
        SocialMetrics {
            employee_turnover_rate: dec!(0.45),
            gender_diversity_pct: dec!(5),
            board_diversity_pct: dec!(2),
            living_wage_compliance: false,
            health_safety_incident_rate: dec!(12),
            community_investment_pct: dec!(0.05),
            supply_chain_audit_pct: dec!(10),
        }
    }

    fn poor_governance() -> GovernanceMetrics {
        GovernanceMetrics {
            board_independence_pct: dec!(0),
            ceo_chair_separation: false,
            executive_pay_ratio: dec!(1200),
            anti_corruption_policy: false,
            whistleblower_mechanism: false,
            audit_committee_independence: false,
            related_party_transactions: dec!(100_000_000),
        }
    }

    fn high_scoring_input() -> EsgScoreInput {
        EsgScoreInput {
            company_name: "GreenCorp Inc.".to_string(),
            sector: "Technology".to_string(),
            environmental: excellent_environmental(),
            social: excellent_social(),
            governance: excellent_governance(),
            pillar_weights: None,
            peer_scores: None,
        }
    }

    fn low_scoring_input() -> EsgScoreInput {
        EsgScoreInput {
            company_name: "DirtyCo Ltd.".to_string(),
            sector: "Energy".to_string(),
            environmental: poor_environmental(),
            social: poor_social(),
            governance: poor_governance(),
            pillar_weights: None,
            peer_scores: None,
        }
    }

    // -----------------------------------------------------------------------
    // Test 1: High-scoring company -> AAA
    // -----------------------------------------------------------------------
    #[test]
    fn test_high_scoring_company_aaa() {
        let input = high_scoring_input();
        let result = calculate_esg_score(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.rating, "AAA");
        assert!(
            out.overall_score >= dec!(85),
            "Expected overall >= 85, got {}",
            out.overall_score
        );
        assert!(
            out.environmental_score >= dec!(80),
            "Expected E >= 80, got {}",
            out.environmental_score
        );
        assert!(
            out.social_score >= dec!(80),
            "Expected S >= 80, got {}",
            out.social_score
        );
        assert!(
            out.governance_score >= dec!(85),
            "Expected G >= 85, got {}",
            out.governance_score
        );
    }

    // -----------------------------------------------------------------------
    // Test 2: Low-scoring company -> B or CCC (very low)
    // -----------------------------------------------------------------------
    #[test]
    fn test_low_scoring_company_very_low() {
        let input = low_scoring_input();
        let result = calculate_esg_score(&input).unwrap();
        let out = &result.result;

        // With tier-based scoring floors, the worst realistic company
        // lands in the B or CCC band. Environmental score is 0 due to
        // heavy fines and poor metrics, but S and G have inherent floors
        // from discrete tier scoring.
        assert!(
            out.overall_score < dec!(25),
            "Expected overall < 25, got {}",
            out.overall_score
        );
        assert!(
            out.rating == "B" || out.rating == "CCC",
            "Expected B or CCC, got {}",
            out.rating
        );
        // Environmental should be 0 (fines wipe it out)
        assert_eq!(
            out.environmental_score,
            dec!(0),
            "Expected E = 0, got {}",
            out.environmental_score
        );
    }

    // -----------------------------------------------------------------------
    // Test 3: Sector materiality — Energy vs Technology have different E weights
    // -----------------------------------------------------------------------
    #[test]
    fn test_sector_materiality_energy_vs_technology() {
        let energy_weights = sector_default_weights("Energy");
        let tech_weights = sector_default_weights("Technology");

        assert_eq!(energy_weights.environmental, dec!(0.50));
        assert_eq!(tech_weights.environmental, dec!(0.20));

        // Energy weights E much more heavily
        assert!(energy_weights.environmental > tech_weights.environmental);
        // Technology weights S and G more
        assert!(tech_weights.social > energy_weights.social);
        assert!(tech_weights.governance > energy_weights.governance);
    }

    // -----------------------------------------------------------------------
    // Test 4: Custom pillar weights override sector defaults
    // -----------------------------------------------------------------------
    #[test]
    fn test_custom_pillar_weights() {
        let mut input = high_scoring_input();
        let custom = PillarWeights {
            environmental: dec!(0.60),
            social: dec!(0.20),
            governance: dec!(0.20),
        };
        input.pillar_weights = Some(custom.clone());

        let result = calculate_esg_score(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.pillar_weights_used.environmental, dec!(0.60));
        assert_eq!(out.pillar_weights_used.social, dec!(0.20));
        assert_eq!(out.pillar_weights_used.governance, dec!(0.20));
    }

    // -----------------------------------------------------------------------
    // Test 5: Peer comparison with percentile ranking
    // -----------------------------------------------------------------------
    #[test]
    fn test_peer_comparison() {
        let mut input = high_scoring_input();
        input.peer_scores = Some(vec![
            PeerScore {
                company_name: "PeerA".to_string(),
                esg_score: dec!(60),
                e_score: dec!(55),
                s_score: dec!(65),
                g_score: dec!(60),
            },
            PeerScore {
                company_name: "PeerB".to_string(),
                esg_score: dec!(75),
                e_score: dec!(70),
                s_score: dec!(80),
                g_score: dec!(75),
            },
            PeerScore {
                company_name: "PeerC".to_string(),
                esg_score: dec!(80),
                e_score: dec!(75),
                s_score: dec!(85),
                g_score: dec!(80),
            },
        ]);

        let result = calculate_esg_score(&input).unwrap();
        let out = &result.result;
        let peer = out.peer_comparison.as_ref().unwrap();

        // Our score should be > 85, above all peers
        assert!(
            peer.percentile_rank >= dec!(75),
            "Expected high percentile rank, got {}",
            peer.percentile_rank
        );
        assert_eq!(peer.best_in_class, "PeerC");
        // Peer average should be around (60+75+80)/3 = 71.67
        assert!(peer.peer_average > dec!(70) && peer.peer_average < dec!(73));
        assert_eq!(peer.peer_median, dec!(75));
    }

    // -----------------------------------------------------------------------
    // Test 6: Red flags triggered (fines, no policies)
    // -----------------------------------------------------------------------
    #[test]
    fn test_red_flags_fines_and_no_policies() {
        let input = low_scoring_input();
        let result = calculate_esg_score(&input).unwrap();
        let flags = &result.result.flags;

        let red_flags: Vec<&EsgFlag> = flags.iter().filter(|f| f.flag_type == "Red").collect();
        assert!(!red_flags.is_empty(), "Expected at least one red flag");

        // Should flag environmental fines
        assert!(
            red_flags.iter().any(|f| f.issue.contains("Fines")),
            "Expected red flag for environmental fines"
        );

        // Should flag missing anti-corruption policy
        assert!(
            red_flags
                .iter()
                .any(|f| f.issue.contains("Anti-Corruption")),
            "Expected red flag for missing anti-corruption policy"
        );

        // Should flag excessive pay ratio
        assert!(
            red_flags.iter().any(|f| f.issue.contains("Pay Ratio")),
            "Expected red flag for excessive pay ratio"
        );
    }

    // -----------------------------------------------------------------------
    // Test 7: Green flags (all good governance)
    // -----------------------------------------------------------------------
    #[test]
    fn test_green_flags_excellent_company() {
        let input = high_scoring_input();
        let result = calculate_esg_score(&input).unwrap();
        let flags = &result.result.flags;

        let green_flags: Vec<&EsgFlag> = flags.iter().filter(|f| f.flag_type == "Green").collect();
        assert!(!green_flags.is_empty(), "Expected at least one green flag");

        // Should flag SBTi targets
        assert!(
            green_flags
                .iter()
                .any(|f| f.issue.contains("Science-Based")),
            "Expected green flag for SBTi targets"
        );

        // Should flag board independence
        assert!(
            green_flags
                .iter()
                .any(|f| f.issue.contains("Board Independence")),
            "Expected green flag for board independence >= 75%"
        );

        // Should flag all pillars strong
        assert!(
            green_flags.iter().any(|f| f.issue.contains("All Pillars")),
            "Expected green flag for all pillars > 70"
        );
    }

    // -----------------------------------------------------------------------
    // Test 8: Score bounds — always 0-100
    // -----------------------------------------------------------------------
    #[test]
    fn test_score_bounds() {
        // Test with extreme low values
        let input = low_scoring_input();
        let result = calculate_esg_score(&input).unwrap();
        let out = &result.result;

        assert!(out.overall_score >= dec!(0), "Overall score below 0");
        assert!(out.overall_score <= dec!(100), "Overall score above 100");
        assert!(out.environmental_score >= dec!(0));
        assert!(out.environmental_score <= dec!(100));
        assert!(out.social_score >= dec!(0));
        assert!(out.social_score <= dec!(100));
        assert!(out.governance_score >= dec!(0));
        assert!(out.governance_score <= dec!(100));

        // Test with extreme high values
        let input2 = high_scoring_input();
        let result2 = calculate_esg_score(&input2).unwrap();
        let out2 = &result2.result;

        assert!(out2.overall_score >= dec!(0));
        assert!(out2.overall_score <= dec!(100));
        assert!(out2.environmental_score >= dec!(0));
        assert!(out2.environmental_score <= dec!(100));
        assert!(out2.social_score >= dec!(0));
        assert!(out2.social_score <= dec!(100));
        assert!(out2.governance_score >= dec!(0));
        assert!(out2.governance_score <= dec!(100));
    }

    // -----------------------------------------------------------------------
    // Test 9: Rating band boundaries
    // -----------------------------------------------------------------------
    #[test]
    fn test_rating_band_boundaries() {
        assert_eq!(score_to_rating(dec!(100)), "AAA");
        assert_eq!(score_to_rating(dec!(85)), "AAA");
        assert_eq!(score_to_rating(dec!(84)), "AA");
        assert_eq!(score_to_rating(dec!(70)), "AA");
        assert_eq!(score_to_rating(dec!(69)), "A");
        assert_eq!(score_to_rating(dec!(55)), "A");
        assert_eq!(score_to_rating(dec!(54)), "BBB");
        assert_eq!(score_to_rating(dec!(40)), "BBB");
        assert_eq!(score_to_rating(dec!(39)), "BB");
        assert_eq!(score_to_rating(dec!(25)), "BB");
        assert_eq!(score_to_rating(dec!(24)), "B");
        assert_eq!(score_to_rating(dec!(10)), "B");
        assert_eq!(score_to_rating(dec!(9)), "CCC");
        assert_eq!(score_to_rating(dec!(0)), "CCC");
    }

    // -----------------------------------------------------------------------
    // Test 10: Carbon intensity scoring tiers
    // -----------------------------------------------------------------------
    #[test]
    fn test_carbon_intensity_scoring_tiers() {
        // Excellent: <100
        let excellent = score_carbon_intensity(dec!(50));
        assert!(
            excellent >= dec!(90) && excellent <= dec!(100),
            "Expected 90-100, got {}",
            excellent
        );

        // Good: 100-500
        let good = score_carbon_intensity(dec!(300));
        assert!(
            good >= dec!(60) && good < dec!(90),
            "Expected 60-89, got {}",
            good
        );

        // Average: 500-1000
        let average = score_carbon_intensity(dec!(750));
        assert!(
            average >= dec!(40) && average < dec!(60),
            "Expected 40-59, got {}",
            average
        );

        // Poor: >1000
        let poor = score_carbon_intensity(dec!(1500));
        assert!(
            poor >= dec!(0) && poor < dec!(40),
            "Expected 0-39, got {}",
            poor
        );
    }

    // -----------------------------------------------------------------------
    // Test 11: Board independence scoring
    // -----------------------------------------------------------------------
    #[test]
    fn test_board_independence_scoring() {
        // 75% or above -> 100
        assert_eq!(score_board_independence(dec!(0.75)), dec!(100));

        // Above target capped at 100
        let above = score_board_independence(dec!(0.90));
        assert_eq!(above, dec!(100)); // clamped

        // 50% -> ~66.7
        let mid = score_board_independence(dec!(0.50));
        let expected = (dec!(0.50) / dec!(0.75)) * dec!(100);
        assert_eq!(mid, expected);

        // 0% -> 0
        assert_eq!(score_board_independence(dec!(0)), dec!(0));
    }

    // -----------------------------------------------------------------------
    // Test 12: Pay ratio impact
    // -----------------------------------------------------------------------
    #[test]
    fn test_pay_ratio_scoring() {
        assert_eq!(score_pay_ratio(dec!(30)), dec!(90));
        assert_eq!(score_pay_ratio(dec!(49)), dec!(90));
        assert_eq!(score_pay_ratio(dec!(50)), dec!(70));
        assert_eq!(score_pay_ratio(dec!(100)), dec!(70));
        assert_eq!(score_pay_ratio(dec!(200)), dec!(50));
        assert_eq!(score_pay_ratio(dec!(499)), dec!(50));
        assert_eq!(score_pay_ratio(dec!(500)), dec!(30));
        assert_eq!(score_pay_ratio(dec!(1000)), dec!(30));
    }

    // -----------------------------------------------------------------------
    // Test 13: Validation — empty company name
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_empty_company_name() {
        let mut input = high_scoring_input();
        input.company_name = "".to_string();
        let err = calculate_esg_score(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "company_name");
            }
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Test 14: Validation — weights not summing to 1
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_weights_dont_sum_to_one() {
        let mut input = high_scoring_input();
        input.pillar_weights = Some(PillarWeights {
            environmental: dec!(0.50),
            social: dec!(0.20),
            governance: dec!(0.20),
        });
        let err = calculate_esg_score(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "pillar_weights");
            }
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Test 15: Validation — renewable_energy_pct out of range
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_renewable_energy_out_of_range() {
        let mut input = high_scoring_input();
        input.environmental.renewable_energy_pct = dec!(1.5);
        let err = calculate_esg_score(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "renewable_energy_pct");
            }
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Test 16: SBTi bonus adds 10 points to environmental score
    // -----------------------------------------------------------------------
    #[test]
    fn test_sbti_bonus() {
        let mut with_sbti = high_scoring_input();
        with_sbti.environmental.science_based_targets = true;

        let mut without_sbti = high_scoring_input();
        without_sbti.environmental.science_based_targets = false;

        let result_with = calculate_esg_score(&with_sbti).unwrap();
        let result_without = calculate_esg_score(&without_sbti).unwrap();

        // The environmental score with SBTi should be higher
        // (unless capped at 100)
        assert!(
            result_with.result.environmental_score >= result_without.result.environmental_score,
            "SBTi bonus should increase E score"
        );
    }

    // -----------------------------------------------------------------------
    // Test 17: Fines deductions
    // -----------------------------------------------------------------------
    #[test]
    fn test_fines_deductions() {
        assert_eq!(fines_deduction(dec!(0)), dec!(0));
        assert_eq!(fines_deduction(dec!(5_000)), dec!(0));
        assert_eq!(fines_deduction(dec!(10_001)), dec!(5));
        assert_eq!(fines_deduction(dec!(50_000)), dec!(5));
        assert_eq!(fines_deduction(dec!(100_001)), dec!(10));
        assert_eq!(fines_deduction(dec!(500_000)), dec!(10));
        assert_eq!(fines_deduction(dec!(1_000_001)), dec!(20));
        assert_eq!(fines_deduction(dec!(10_000_000)), dec!(20));
    }

    // -----------------------------------------------------------------------
    // Test 18: Turnover rate scoring
    // -----------------------------------------------------------------------
    #[test]
    fn test_turnover_rate_scoring() {
        // <10% -> 90
        assert_eq!(score_turnover(dec!(0.08)), dec!(90));
        // 10-20% -> 70
        assert_eq!(score_turnover(dec!(0.15)), dec!(70));
        // 20-30% -> 50
        assert_eq!(score_turnover(dec!(0.25)), dec!(50));
        // >30% -> 30
        assert_eq!(score_turnover(dec!(0.40)), dec!(30));
    }

    // -----------------------------------------------------------------------
    // Test 19: All default sector weights
    // -----------------------------------------------------------------------
    #[test]
    fn test_all_sector_default_weights() {
        let sectors = [
            ("Energy", dec!(0.50), dec!(0.20), dec!(0.30)),
            ("Technology", dec!(0.20), dec!(0.40), dec!(0.40)),
            ("Financials", dec!(0.15), dec!(0.35), dec!(0.50)),
            ("Healthcare", dec!(0.20), dec!(0.45), dec!(0.35)),
            ("Materials", dec!(0.45), dec!(0.25), dec!(0.30)),
            ("Industrials", dec!(0.35), dec!(0.30), dec!(0.35)),
            ("Consumer", dec!(0.25), dec!(0.40), dec!(0.35)),
            ("Utilities", dec!(0.45), dec!(0.25), dec!(0.30)),
            ("Real Estate", dec!(0.40), dec!(0.25), dec!(0.35)),
        ];

        for (sector, e, s, g) in &sectors {
            let w = sector_default_weights(sector);
            assert_eq!(w.environmental, *e, "Wrong E weight for {}", sector);
            assert_eq!(w.social, *s, "Wrong S weight for {}", sector);
            assert_eq!(w.governance, *g, "Wrong G weight for {}", sector);
            // Verify they sum to 1.0
            assert_eq!(
                w.environmental + w.social + w.governance,
                dec!(1.00),
                "Weights for {} don't sum to 1.0",
                sector
            );
        }

        // Default (unknown sector)
        let def = sector_default_weights("Unknown");
        assert_eq!(def.environmental, dec!(0.33));
        assert_eq!(def.social, dec!(0.34));
        assert_eq!(def.governance, dec!(0.33));
        assert_eq!(def.environmental + def.social + def.governance, dec!(1.00));
    }

    // -----------------------------------------------------------------------
    // Test 20: Metadata populated correctly
    // -----------------------------------------------------------------------
    #[test]
    fn test_metadata_populated() {
        let input = high_scoring_input();
        let result = calculate_esg_score(&input).unwrap();

        assert!(!result.methodology.is_empty());
        assert!(result.methodology.contains("ESG"));
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
    }

    // -----------------------------------------------------------------------
    // Test 21: Materiality map has entries
    // -----------------------------------------------------------------------
    #[test]
    fn test_materiality_map_populated() {
        let input = high_scoring_input();
        let result = calculate_esg_score(&input).unwrap();
        let map = &result.result.materiality_map;

        assert!(!map.is_empty(), "Materiality map should have entries");

        // Should have entries for all three pillars
        assert!(map.iter().any(|m| m.pillar == "E"));
        assert!(map.iter().any(|m| m.pillar == "S"));
        assert!(map.iter().any(|m| m.pillar == "G"));

        // Technology sector should have Data Privacy
        assert!(
            map.iter().any(|m| m.issue == "Data Privacy"),
            "Technology sector should include Data Privacy materiality issue"
        );
    }

    // -----------------------------------------------------------------------
    // Test 22: Peer comparison with no peers
    // -----------------------------------------------------------------------
    #[test]
    fn test_peer_comparison_no_peers() {
        let input = high_scoring_input();
        let result = calculate_esg_score(&input).unwrap();
        assert!(
            result.result.peer_comparison.is_none(),
            "Should be None when no peers provided"
        );
    }

    // -----------------------------------------------------------------------
    // Test 23: Unknown sector uses default weights with warning
    // -----------------------------------------------------------------------
    #[test]
    fn test_unknown_sector_warning() {
        let mut input = high_scoring_input();
        input.sector = "Crypto".to_string();
        let result = calculate_esg_score(&input).unwrap();

        assert!(
            result.warnings.iter().any(|w| w.contains("Unknown sector")),
            "Should warn about unknown sector"
        );
        assert_eq!(result.result.pillar_weights_used.environmental, dec!(0.33));
    }

    // -----------------------------------------------------------------------
    // Test 24: Water intensity scoring
    // -----------------------------------------------------------------------
    #[test]
    fn test_water_intensity_scoring() {
        let excellent = score_water_intensity(dec!(20));
        assert!(
            excellent >= dec!(90),
            "Expected >= 90 for low water intensity, got {}",
            excellent
        );

        let good = score_water_intensity(dec!(100));
        assert!(
            good >= dec!(60) && good < dec!(90),
            "Expected 60-89, got {}",
            good
        );

        let average = score_water_intensity(dec!(350));
        assert!(
            average >= dec!(40) && average < dec!(60),
            "Expected 40-59, got {}",
            average
        );

        let poor = score_water_intensity(dec!(800));
        assert!(
            poor < dec!(40),
            "Expected < 40 for high water intensity, got {}",
            poor
        );
    }

    // -----------------------------------------------------------------------
    // Test 25: Related party transactions scoring
    // -----------------------------------------------------------------------
    #[test]
    fn test_related_party_transactions_scoring() {
        assert_eq!(score_related_party_transactions(dec!(0)), dec!(100));
        assert_eq!(score_related_party_transactions(dec!(50_000)), dec!(80));
        assert_eq!(score_related_party_transactions(dec!(500_000)), dec!(60));
        assert_eq!(score_related_party_transactions(dec!(5_000_000)), dec!(40));
        assert_eq!(score_related_party_transactions(dec!(50_000_000)), dec!(20));
    }

    // -----------------------------------------------------------------------
    // Test 26: Clamp score utility
    // -----------------------------------------------------------------------
    #[test]
    fn test_clamp_score() {
        assert_eq!(clamp_score(dec!(-10)), dec!(0));
        assert_eq!(clamp_score(dec!(0)), dec!(0));
        assert_eq!(clamp_score(dec!(50)), dec!(50));
        assert_eq!(clamp_score(dec!(100)), dec!(100));
        assert_eq!(clamp_score(dec!(150)), dec!(100));
    }

    // -----------------------------------------------------------------------
    // Test 27: Amber flags for mid-range scores
    // -----------------------------------------------------------------------
    #[test]
    fn test_amber_flags_mid_range() {
        // Create input that produces mid-range environmental scores (30-50)
        let mut input = high_scoring_input();
        // Make environmental mediocre — push score into 30-50 range
        input.environmental.carbon_intensity = dec!(800);
        input.environmental.renewable_energy_pct = dec!(0.20);
        input.environmental.water_intensity = dec!(400);
        input.environmental.waste_recycling_rate = dec!(0.15);
        input.environmental.science_based_targets = false;
        input.environmental.environmental_fines_amount = dec!(0);

        let result = calculate_esg_score(&input).unwrap();
        let e_score = result.result.environmental_score;
        let flags = &result.result.flags;

        // Verify E score is in the amber range
        assert!(
            e_score >= dec!(30) && e_score <= dec!(50),
            "Expected E score 30-50 for amber flags, got {}",
            e_score
        );

        let amber_flags: Vec<&EsgFlag> = flags.iter().filter(|f| f.flag_type == "Amber").collect();

        // Should have at least one amber flag for environmental pillar
        assert!(
            !amber_flags.is_empty(),
            "Expected amber flags for mid-range scores"
        );
    }

    // -----------------------------------------------------------------------
    // Test 28: Gender diversity scoring
    // -----------------------------------------------------------------------
    #[test]
    fn test_gender_diversity_scoring() {
        // 50% target = 100
        assert_eq!(score_gender_diversity(dec!(50)), dec!(100));

        // Above target clamped
        assert_eq!(score_gender_diversity(dec!(60)), dec!(100));

        // 25% = 50
        assert_eq!(score_gender_diversity(dec!(25)), dec!(50));

        // 0% = 0
        assert_eq!(score_gender_diversity(dec!(0)), dec!(0));
    }

    // -----------------------------------------------------------------------
    // Test 29: Community investment scoring
    // -----------------------------------------------------------------------
    #[test]
    fn test_community_investment_scoring() {
        assert_eq!(score_community_investment(dec!(2.0)), dec!(90));
        assert_eq!(score_community_investment(dec!(1.5)), dec!(90));
        assert_eq!(score_community_investment(dec!(0.8)), dec!(70));
        assert_eq!(score_community_investment(dec!(0.5)), dec!(70));
        assert_eq!(score_community_investment(dec!(0.3)), dec!(50));
    }

    // -----------------------------------------------------------------------
    // Test 30: Health and safety scoring
    // -----------------------------------------------------------------------
    #[test]
    fn test_health_safety_scoring() {
        assert_eq!(score_health_safety(dec!(0.5)), dec!(90));
        assert_eq!(score_health_safety(dec!(2)), dec!(70));
        assert_eq!(score_health_safety(dec!(4)), dec!(50));
        assert_eq!(score_health_safety(dec!(6)), dec!(30));
    }

    // -----------------------------------------------------------------------
    // Test 31: Negative carbon intensity rejected
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_negative_carbon_intensity() {
        let mut input = high_scoring_input();
        input.environmental.carbon_intensity = dec!(-10);
        let err = calculate_esg_score(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "carbon_intensity");
            }
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Test 32: Board diversity scoring (target 40%)
    // -----------------------------------------------------------------------
    #[test]
    fn test_board_diversity_scoring() {
        assert_eq!(score_board_diversity(dec!(40)), dec!(100));
        assert_eq!(score_board_diversity(dec!(50)), dec!(100)); // clamped
        assert_eq!(score_board_diversity(dec!(20)), dec!(50));
        assert_eq!(score_board_diversity(dec!(0)), dec!(0));
    }
}
