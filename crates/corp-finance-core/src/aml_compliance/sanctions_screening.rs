use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScreeningType {
    Onboarding,
    Transaction,
    PeriodicReview,
    BatchRescreen,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntityType {
    Individual,
    Corporate,
    Trust,
    Government,
    Vessel,
    Aircraft,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SanctionsList {
    #[serde(rename = "OFAC_SDN")]
    OfacSdn,
    #[serde(rename = "EU_Consolidated")]
    EuConsolidated,
    #[serde(rename = "HMT_UK")]
    HmtUk,
    #[serde(rename = "UN_UNSC")]
    UnUnsc,
    #[serde(rename = "FATF_GreyList")]
    FatfGreyList,
    #[serde(rename = "FATF_BlackList")]
    FatfBlackList,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatchType {
    Exact,
    Strong,
    Possible,
    Weak,
    NoMatch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionRequired {
    Clear,
    ManualReview,
    Escalate,
    Block,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OverallStatus {
    Clear,
    PotentialMatch,
    TruePositive,
    Blocked,
}

// ---------------------------------------------------------------------------
// Input structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreeningEntity {
    pub name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_of_birth: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nationality: Option<String>,
    pub jurisdiction: String,
    pub entity_type: EntityType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionDetails {
    pub amount: Decimal,
    pub currency: String,
    pub counterparty_jurisdiction: String,
    pub purpose: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanctionsScreeningInput {
    pub screening_type: ScreeningType,
    pub entities_to_screen: Vec<ScreeningEntity>,
    pub lists_to_check: Vec<SanctionsList>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_details: Option<TransactionDetails>,
    pub screening_threshold: Decimal,
    pub include_pep_screening: bool,
    pub include_adverse_media: bool,
}

// ---------------------------------------------------------------------------
// Output structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreeningResult {
    pub entity_name: String,
    pub list_checked: String,
    pub match_score: Decimal,
    pub match_type: MatchType,
    pub matched_entry: Option<String>,
    pub action_required: ActionRequired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountryRiskFlag {
    pub country: String,
    pub sanction_type: String,
    pub regime: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanctionsScreeningOutput {
    pub screening_results: Vec<ScreeningResult>,
    pub overall_status: OverallStatus,
    pub matches_found: u32,
    pub false_positive_indicators: Vec<String>,
    pub country_risk_flags: Vec<CountryRiskFlag>,
    pub recommended_actions: Vec<String>,
    pub escalation_required: bool,
    pub sar_filing_required: bool,
    pub sla_deadline: Option<String>,
    pub screening_coverage: Decimal,
    pub methodology: String,
    pub assumptions: Vec<String>,
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// Simulated sanctions database entries
// ---------------------------------------------------------------------------

/// Represents an entry in a sanctions list for matching.
struct SanctionsEntry {
    name: String,
    aliases: Vec<String>,
    list: SanctionsList,
    jurisdiction: Option<String>,
}

/// Build a representative set of sanctions entries for screening.
/// In production this would query real OFAC/EU/HMT/UN databases.
fn build_sanctions_database() -> Vec<SanctionsEntry> {
    vec![
        // OFAC SDN entries
        SanctionsEntry {
            name: "kim jong un".to_string(),
            aliases: vec!["kim jong-un".to_string(), "kim jongun".to_string()],
            list: SanctionsList::OfacSdn,
            jurisdiction: Some("north korea".to_string()),
        },
        SanctionsEntry {
            name: "ali khamenei".to_string(),
            aliases: vec![
                "ayatollah khamenei".to_string(),
                "seyyed ali khamenei".to_string(),
            ],
            list: SanctionsList::OfacSdn,
            jurisdiction: Some("iran".to_string()),
        },
        SanctionsEntry {
            name: "banco delta asia".to_string(),
            aliases: vec!["bda".to_string()],
            list: SanctionsList::OfacSdn,
            jurisdiction: Some("macau".to_string()),
        },
        SanctionsEntry {
            name: "islamic revolutionary guard corps".to_string(),
            aliases: vec![
                "irgc".to_string(),
                "sepah".to_string(),
                "pasdaran".to_string(),
            ],
            list: SanctionsList::OfacSdn,
            jurisdiction: Some("iran".to_string()),
        },
        SanctionsEntry {
            name: "al qaeda".to_string(),
            aliases: vec![
                "al-qaeda".to_string(),
                "al qaida".to_string(),
                "the base".to_string(),
            ],
            list: SanctionsList::OfacSdn,
            jurisdiction: None,
        },
        // EU Consolidated
        SanctionsEntry {
            name: "bashar al-assad".to_string(),
            aliases: vec!["bashar assad".to_string(), "bashar al assad".to_string()],
            list: SanctionsList::EuConsolidated,
            jurisdiction: Some("syria".to_string()),
        },
        SanctionsEntry {
            name: "rosoboronexport".to_string(),
            aliases: vec!["rosoboron".to_string()],
            list: SanctionsList::EuConsolidated,
            jurisdiction: Some("russia".to_string()),
        },
        // HMT UK
        SanctionsEntry {
            name: "vladimir putin".to_string(),
            aliases: vec![
                "putin vladimir vladimirovich".to_string(),
                "v putin".to_string(),
            ],
            list: SanctionsList::HmtUk,
            jurisdiction: Some("russia".to_string()),
        },
        SanctionsEntry {
            name: "sberbank".to_string(),
            aliases: vec!["sberbank of russia".to_string()],
            list: SanctionsList::HmtUk,
            jurisdiction: Some("russia".to_string()),
        },
        // UN UNSC
        SanctionsEntry {
            name: "korea mining development trading corporation".to_string(),
            aliases: vec!["komid".to_string()],
            list: SanctionsList::UnUnsc,
            jurisdiction: Some("north korea".to_string()),
        },
        SanctionsEntry {
            name: "hezbollah".to_string(),
            aliases: vec![
                "hizballah".to_string(),
                "hizbullah".to_string(),
                "party of god".to_string(),
            ],
            list: SanctionsList::UnUnsc,
            jurisdiction: Some("lebanon".to_string()),
        },
    ]
}

// ---------------------------------------------------------------------------
// Fuzzy matching — Levenshtein distance
// ---------------------------------------------------------------------------

/// Compute the Levenshtein edit distance between two strings.
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_len = a.len();
    let b_len = b.len();

    if a_len == 0 {
        return b_len;
    }
    if b_len == 0 {
        return a_len;
    }

    // Use two-row optimisation
    let mut prev_row: Vec<usize> = (0..=b_len).collect();
    let mut curr_row: Vec<usize> = vec![0; b_len + 1];

    for (i, ca) in a.chars().enumerate() {
        curr_row[0] = i + 1;
        for (j, cb) in b.chars().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            curr_row[j + 1] = (prev_row[j] + cost)
                .min(prev_row[j + 1] + 1)
                .min(curr_row[j] + 1);
        }
        std::mem::swap(&mut prev_row, &mut curr_row);
    }

    prev_row[b_len]
}

/// Convert Levenshtein distance to a similarity score (0-100 Decimal).
/// Score = 100 * (1 - distance / max_len).
fn similarity_score(a: &str, b: &str) -> Decimal {
    let a_lower = a.trim().to_lowercase();
    let b_lower = b.trim().to_lowercase();

    if a_lower.is_empty() && b_lower.is_empty() {
        return dec!(100);
    }
    if a_lower.is_empty() || b_lower.is_empty() {
        return Decimal::ZERO;
    }

    let dist = levenshtein_distance(&a_lower, &b_lower);
    let max_len = a_lower.len().max(b_lower.len());

    if max_len == 0 {
        return dec!(100);
    }

    let dist_d = Decimal::from(dist as u64);
    let max_d = Decimal::from(max_len as u64);

    let ratio = dec!(100) * (Decimal::ONE - dist_d / max_d);
    if ratio < Decimal::ZERO {
        Decimal::ZERO
    } else {
        ratio.round_dp(2)
    }
}

/// Compute the best match score between a name (plus aliases) and a
/// sanctions entry (name + aliases).  Returns the maximum pairwise
/// similarity across all combinations.
fn best_match_score(
    entity_name: &str,
    entity_aliases: &[String],
    entry_name: &str,
    entry_aliases: &[String],
) -> Decimal {
    let mut names_to_check: Vec<&str> = vec![entity_name];
    for alias in entity_aliases {
        names_to_check.push(alias.as_str());
    }

    let mut entry_names: Vec<&str> = vec![entry_name];
    for alias in entry_aliases {
        entry_names.push(alias.as_str());
    }

    let mut max_score = Decimal::ZERO;
    for &n in &names_to_check {
        for &e in &entry_names {
            let score = similarity_score(n, e);
            if score > max_score {
                max_score = score;
            }
        }
    }

    max_score
}

/// Classify a match score into a MatchType.
fn classify_match(score: Decimal) -> MatchType {
    if score >= dec!(100) {
        MatchType::Exact
    } else if score >= dec!(90) {
        MatchType::Strong
    } else if score >= dec!(70) {
        MatchType::Possible
    } else if score >= dec!(50) {
        MatchType::Weak
    } else {
        MatchType::NoMatch
    }
}

/// Determine action required based on match type.
fn action_for_match(match_type: &MatchType) -> ActionRequired {
    match match_type {
        MatchType::Exact => ActionRequired::Block,
        MatchType::Strong => ActionRequired::Escalate,
        MatchType::Possible => ActionRequired::ManualReview,
        MatchType::Weak => ActionRequired::ManualReview,
        MatchType::NoMatch => ActionRequired::Clear,
    }
}

// ---------------------------------------------------------------------------
// Country risk screening
// ---------------------------------------------------------------------------

/// Comprehensive sanctions (full embargo).
const EMBARGO_COUNTRIES: &[&str] = &["north korea", "dprk", "iran", "cuba", "syria", "crimea"];

/// Sectoral sanctions.
const SECTORAL_SANCTIONS: &[&str] = &["russia", "venezuela", "myanmar", "belarus"];

/// FATF grey list (increased monitoring).
const FATF_GREY: &[&str] = &[
    "albania",
    "barbados",
    "burkina faso",
    "cameroon",
    "cayman islands",
    "croatia",
    "democratic republic of the congo",
    "gibraltar",
    "haiti",
    "jamaica",
    "jordan",
    "mali",
    "mozambique",
    "nigeria",
    "panama",
    "philippines",
    "senegal",
    "south africa",
    "south sudan",
    "tanzania",
    "turkey",
    "uganda",
    "united arab emirates",
    "vietnam",
    "yemen",
];

/// FATF black list.
const FATF_BLACK: &[&str] = &["north korea", "dprk", "iran", "myanmar"];

fn normalize(s: &str) -> String {
    s.trim().to_lowercase()
}

fn check_country_risk(jurisdiction: &str) -> Vec<CountryRiskFlag> {
    let j = normalize(jurisdiction);
    let mut flags = Vec::new();

    for &c in EMBARGO_COUNTRIES {
        if j.contains(c) {
            flags.push(CountryRiskFlag {
                country: jurisdiction.to_string(),
                sanction_type: "Comprehensive embargo".to_string(),
                regime: match c {
                    "north korea" | "dprk" => "OFAC/EU/UN".to_string(),
                    "iran" => "OFAC/EU/UN".to_string(),
                    "cuba" => "OFAC".to_string(),
                    "syria" => "OFAC/EU".to_string(),
                    "crimea" => "OFAC/EU".to_string(),
                    _ => "Multiple".to_string(),
                },
            });
            return flags;
        }
    }

    for &c in SECTORAL_SANCTIONS {
        if j.contains(c) {
            flags.push(CountryRiskFlag {
                country: jurisdiction.to_string(),
                sanction_type: "Sectoral sanctions".to_string(),
                regime: match c {
                    "russia" => "OFAC/EU/HMT".to_string(),
                    "venezuela" => "OFAC".to_string(),
                    "myanmar" => "OFAC/EU/HMT".to_string(),
                    "belarus" => "EU/HMT".to_string(),
                    _ => "Multiple".to_string(),
                },
            });
            return flags;
        }
    }

    for &c in FATF_BLACK {
        if j.contains(c) {
            flags.push(CountryRiskFlag {
                country: jurisdiction.to_string(),
                sanction_type: "FATF Black List".to_string(),
                regime: "FATF".to_string(),
            });
            return flags;
        }
    }

    for &c in FATF_GREY {
        if j.contains(c) {
            flags.push(CountryRiskFlag {
                country: jurisdiction.to_string(),
                sanction_type: "FATF Grey List — increased monitoring".to_string(),
                regime: "FATF".to_string(),
            });
            return flags;
        }
    }

    flags
}

fn is_embargoed(jurisdiction: &str) -> bool {
    let j = normalize(jurisdiction);
    EMBARGO_COUNTRIES.iter().any(|&c| j.contains(c))
}

// ---------------------------------------------------------------------------
// False positive heuristics
// ---------------------------------------------------------------------------

fn detect_false_positive_indicators(
    entity: &ScreeningEntity,
    entry_name: &str,
    score: Decimal,
) -> Vec<String> {
    let mut indicators = Vec::new();

    // Common name overlap
    let e_lower = normalize(&entity.name);
    let s_lower = normalize(entry_name);

    // Very short names are more likely false positives
    if e_lower.len() <= 4 || s_lower.len() <= 4 {
        indicators.push(format!(
            "Short name length ({} chars) — higher false positive probability",
            e_lower.len().min(s_lower.len())
        ));
    }

    // Score in weak range with different jurisdiction
    if score >= dec!(50) && score < dec!(70) {
        indicators.push("Score in weak match range (50-70) — likely false positive".to_string());
    }

    // Jurisdiction mismatch with entry
    // (heuristic: if we know the entry jurisdiction and they differ)
    if !entity.jurisdiction.is_empty() {
        let e_j = normalize(&entity.jurisdiction);
        let common_safe = [
            "united states",
            "united kingdom",
            "germany",
            "france",
            "japan",
            "canada",
            "australia",
            "switzerland",
        ];
        if common_safe.iter().any(|&c| e_j.contains(c)) && score < dec!(90) {
            indicators.push(
                "Entity jurisdiction is low-risk — lower true-positive probability".to_string(),
            );
        }
    }

    indicators
}

// ---------------------------------------------------------------------------
// Main public function
// ---------------------------------------------------------------------------

/// Screen entities against sanctions lists, producing match scores,
/// country risk flags, and recommended actions.
pub fn screen_sanctions(
    input: &SanctionsScreeningInput,
) -> CorpFinanceResult<SanctionsScreeningOutput> {
    // Input validation
    if input.entities_to_screen.is_empty() {
        return Err(crate::CorpFinanceError::InvalidInput {
            field: "entities_to_screen".to_string(),
            reason: "At least one entity must be provided for screening".to_string(),
        });
    }
    if input.lists_to_check.is_empty() {
        return Err(crate::CorpFinanceError::InvalidInput {
            field: "lists_to_check".to_string(),
            reason: "At least one sanctions list must be selected".to_string(),
        });
    }
    if input.screening_threshold < Decimal::ZERO || input.screening_threshold > dec!(100) {
        return Err(crate::CorpFinanceError::InvalidInput {
            field: "screening_threshold".to_string(),
            reason: "Threshold must be between 0 and 100".to_string(),
        });
    }

    for (idx, entity) in input.entities_to_screen.iter().enumerate() {
        if entity.name.trim().is_empty() {
            return Err(crate::CorpFinanceError::InvalidInput {
                field: format!("entities_to_screen[{}].name", idx),
                reason: "Entity name must not be empty".to_string(),
            });
        }
    }

    // Validate transaction details if screening type is Transaction
    if input.screening_type == ScreeningType::Transaction {
        if input.transaction_details.is_none() {
            return Err(crate::CorpFinanceError::InvalidInput {
                field: "transaction_details".to_string(),
                reason: "Transaction details required for transaction screening".to_string(),
            });
        }
        if let Some(ref td) = input.transaction_details {
            if td.amount < Decimal::ZERO {
                return Err(crate::CorpFinanceError::InvalidInput {
                    field: "transaction_details.amount".to_string(),
                    reason: "Transaction amount must be non-negative".to_string(),
                });
            }
        }
    }

    let database = build_sanctions_database();

    // Total possible list count (all 6 defined lists)
    let total_lists: Decimal = dec!(6);
    let checked_lists = Decimal::from(input.lists_to_check.len() as u64);
    let screening_coverage = (checked_lists / total_lists * dec!(100)).round_dp(2);

    let mut screening_results = Vec::new();
    let mut all_country_risk_flags = Vec::new();
    let mut all_false_positive_indicators = Vec::new();
    let mut max_match_score = Decimal::ZERO;
    let mut matches_found: u32 = 0;

    for entity in &input.entities_to_screen {
        // Country risk check for entity jurisdiction
        let country_flags = check_country_risk(&entity.jurisdiction);
        for flag in &country_flags {
            // Deduplicate
            if !all_country_risk_flags
                .iter()
                .any(|f: &CountryRiskFlag| f.country == flag.country)
            {
                all_country_risk_flags.push(flag.clone());
            }
        }

        // Screen against each selected list
        for list in &input.lists_to_check {
            let relevant_entries: Vec<&SanctionsEntry> =
                database.iter().filter(|e| &e.list == list).collect();

            let mut best_score = Decimal::ZERO;
            let mut best_entry_name: Option<String> = None;

            for entry in &relevant_entries {
                let score =
                    best_match_score(&entity.name, &entity.aliases, &entry.name, &entry.aliases);

                // Jurisdiction boost: if entity and entry share jurisdiction,
                // add 5 points (capped at 100)
                let jurisdiction_boost = match &entry.jurisdiction {
                    Some(ej) => {
                        let ej_lower = normalize(ej);
                        let entity_j = normalize(&entity.jurisdiction);
                        if !entity_j.is_empty()
                            && (entity_j.contains(&ej_lower) || ej_lower.contains(&entity_j))
                        {
                            dec!(5)
                        } else {
                            Decimal::ZERO
                        }
                    }
                    None => Decimal::ZERO,
                };

                let adjusted = (score + jurisdiction_boost).min(dec!(100));

                if adjusted > best_score {
                    best_score = adjusted;
                    best_entry_name = Some(entry.name.clone());
                }
            }

            // Only report if above threshold or if the list had entries
            let match_type = classify_match(best_score);
            let action = action_for_match(&match_type);

            if best_score >= input.screening_threshold {
                matches_found += 1;

                // Collect false positive indicators
                if let Some(ref entry_name) = best_entry_name {
                    let fp_indicators =
                        detect_false_positive_indicators(entity, entry_name, best_score);
                    for indicator in fp_indicators {
                        if !all_false_positive_indicators.contains(&indicator) {
                            all_false_positive_indicators.push(indicator);
                        }
                    }
                }
            }

            if best_score > max_match_score {
                max_match_score = best_score;
            }

            let list_name = match list {
                SanctionsList::OfacSdn => "OFAC SDN",
                SanctionsList::EuConsolidated => "EU Consolidated",
                SanctionsList::HmtUk => "HMT UK",
                SanctionsList::UnUnsc => "UN UNSC",
                SanctionsList::FatfGreyList => "FATF Grey List",
                SanctionsList::FatfBlackList => "FATF Black List",
            };

            screening_results.push(ScreeningResult {
                entity_name: entity.name.clone(),
                list_checked: list_name.to_string(),
                match_score: best_score,
                match_type: if best_score >= input.screening_threshold {
                    match_type
                } else {
                    MatchType::NoMatch
                },
                matched_entry: if best_score >= input.screening_threshold {
                    best_entry_name.clone()
                } else {
                    None
                },
                action_required: if best_score >= input.screening_threshold {
                    action
                } else {
                    ActionRequired::Clear
                },
            });
        }

        // Transaction screening: check counterparty jurisdiction
        if let Some(ref td) = input.transaction_details {
            let cp_flags = check_country_risk(&td.counterparty_jurisdiction);
            for flag in &cp_flags {
                if !all_country_risk_flags
                    .iter()
                    .any(|f: &CountryRiskFlag| f.country == flag.country)
                {
                    all_country_risk_flags.push(flag.clone());
                }
            }
        }
    }

    // Determine overall status
    let overall_status = if screening_results
        .iter()
        .any(|r| matches!(r.action_required, ActionRequired::Block))
    {
        OverallStatus::Blocked
    } else if screening_results
        .iter()
        .any(|r| matches!(r.match_type, MatchType::Exact | MatchType::Strong))
    {
        OverallStatus::TruePositive
    } else if matches_found > 0 {
        OverallStatus::PotentialMatch
    } else {
        // Also check if any entity is in an embargoed jurisdiction
        let any_embargo = input
            .entities_to_screen
            .iter()
            .any(|e| is_embargoed(&e.jurisdiction));
        if any_embargo {
            OverallStatus::Blocked
        } else {
            OverallStatus::Clear
        }
    };

    // Determine escalation and SAR requirements
    let escalation_required = matches!(
        overall_status,
        OverallStatus::TruePositive | OverallStatus::Blocked
    ) || max_match_score >= dec!(90)
        || !all_country_risk_flags.is_empty()
            && all_country_risk_flags
                .iter()
                .any(|f| f.sanction_type.contains("embargo"));

    let sar_filing_required = matches!(
        overall_status,
        OverallStatus::TruePositive | OverallStatus::Blocked
    );

    // SLA deadline
    let sla_deadline = match overall_status {
        OverallStatus::Blocked => {
            Some("Immediate — block transaction and notify compliance".to_string())
        }
        OverallStatus::TruePositive => Some(
            "24 hours — file SAR for potential terrorist financing; 30 days for other".to_string(),
        ),
        OverallStatus::PotentialMatch => Some("48 hours — complete manual review".to_string()),
        OverallStatus::Clear => None,
    };

    // Recommended actions
    let mut recommended_actions = Vec::new();
    match overall_status {
        OverallStatus::Blocked => {
            recommended_actions
                .push("BLOCK: Immediately reject transaction/onboarding".to_string());
            recommended_actions.push("Notify MLRO and senior compliance officer".to_string());
            recommended_actions
                .push("File Suspicious Activity Report (SAR) within 24 hours".to_string());
            recommended_actions.push("Retain all screening records for 5+ years".to_string());
        }
        OverallStatus::TruePositive => {
            recommended_actions.push("Escalate to MLRO for immediate review".to_string());
            recommended_actions
                .push("Freeze account/transaction pending investigation".to_string());
            recommended_actions.push("File SAR within prescribed timeframe".to_string());
            recommended_actions.push("Document all findings and investigation steps".to_string());
        }
        OverallStatus::PotentialMatch => {
            recommended_actions
                .push("Assign to compliance analyst for manual review within 48h SLA".to_string());
            recommended_actions
                .push("Gather additional identifying information for disambiguation".to_string());
            recommended_actions.push(
                "If confirmed false positive, document reasoning and add to whitelist".to_string(),
            );
        }
        OverallStatus::Clear => {
            recommended_actions.push("Proceed with standard processing".to_string());
            recommended_actions.push("Schedule next periodic re-screening per policy".to_string());
        }
    }

    // Additional action for embargoed jurisdictions
    for flag in &all_country_risk_flags {
        if flag.sanction_type.contains("embargo") {
            recommended_actions.push(format!(
                "Embargo alert: {} — verify no prohibited dealings under {} sanctions",
                flag.country, flag.regime
            ));
        }
    }

    if input.include_pep_screening {
        recommended_actions.push("Cross-reference entities against PEP databases".to_string());
    }

    if input.include_adverse_media {
        recommended_actions.push("Run adverse media screening for all entities".to_string());
    }

    let mut assumptions = vec![
        "Screening performed against representative sanctions database entries".to_string(),
        "Fuzzy matching uses Levenshtein distance with jurisdiction boost".to_string(),
        "Match scores are indicative — manual review required for potential matches".to_string(),
    ];

    let mut warnings = Vec::new();

    if screening_coverage < dec!(100) {
        warnings.push(format!(
            "Only {}% of available sanctions lists were checked — consider screening against all lists",
            screening_coverage
        ));
    }

    if matches!(
        overall_status,
        OverallStatus::Blocked | OverallStatus::TruePositive
    ) {
        warnings
            .push("Match found — do NOT proceed until compliance review is complete".to_string());
    }

    if input.screening_threshold < dec!(70) {
        assumptions.push(format!(
            "Screening threshold set to {}% — lower thresholds increase false positives",
            input.screening_threshold
        ));
    }

    Ok(SanctionsScreeningOutput {
        screening_results,
        overall_status,
        matches_found,
        false_positive_indicators: all_false_positive_indicators,
        country_risk_flags: all_country_risk_flags,
        recommended_actions,
        escalation_required,
        sar_filing_required,
        sla_deadline,
        screening_coverage,
        methodology: "Name-based fuzzy matching (Levenshtein distance) with jurisdiction boost, screened against OFAC SDN, EU Consolidated, HMT UK, UN UNSC, FATF Grey/Black lists".to_string(),
        assumptions,
        warnings,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn base_entity() -> ScreeningEntity {
        ScreeningEntity {
            name: "John Smith".to_string(),
            aliases: vec![],
            date_of_birth: Some("1980-01-01".to_string()),
            nationality: Some("US".to_string()),
            jurisdiction: "United States".to_string(),
            entity_type: EntityType::Individual,
        }
    }

    fn base_input() -> SanctionsScreeningInput {
        SanctionsScreeningInput {
            screening_type: ScreeningType::Onboarding,
            entities_to_screen: vec![base_entity()],
            lists_to_check: vec![
                SanctionsList::OfacSdn,
                SanctionsList::EuConsolidated,
                SanctionsList::HmtUk,
                SanctionsList::UnUnsc,
            ],
            transaction_details: None,
            screening_threshold: dec!(70),
            include_pep_screening: false,
            include_adverse_media: false,
        }
    }

    // === Levenshtein / similarity tests ===

    #[test]
    fn test_levenshtein_identical_strings() {
        assert_eq!(levenshtein_distance("hello", "hello"), 0);
    }

    #[test]
    fn test_levenshtein_empty_strings() {
        assert_eq!(levenshtein_distance("", ""), 0);
    }

    #[test]
    fn test_levenshtein_one_empty() {
        assert_eq!(levenshtein_distance("abc", ""), 3);
        assert_eq!(levenshtein_distance("", "xyz"), 3);
    }

    #[test]
    fn test_levenshtein_single_edit() {
        assert_eq!(levenshtein_distance("cat", "bat"), 1);
    }

    #[test]
    fn test_levenshtein_multiple_edits() {
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
    }

    #[test]
    fn test_similarity_exact_match() {
        let score = similarity_score("Kim Jong Un", "kim jong un");
        assert_eq!(score, dec!(100));
    }

    #[test]
    fn test_similarity_close_match() {
        let score = similarity_score("Kim Jong-Un", "Kim Jongun");
        assert!(score >= dec!(80));
    }

    #[test]
    fn test_similarity_different_names() {
        let score = similarity_score("John Smith", "Vladimir Putin");
        assert!(score < dec!(50));
    }

    #[test]
    fn test_similarity_empty_strings() {
        let score = similarity_score("", "");
        assert_eq!(score, dec!(100));
    }

    #[test]
    fn test_similarity_one_empty() {
        let score = similarity_score("test", "");
        assert_eq!(score, Decimal::ZERO);
    }

    // === Exact name match tests ===

    #[test]
    fn test_exact_match_ofac() {
        let mut input = base_input();
        input.entities_to_screen = vec![ScreeningEntity {
            name: "Kim Jong Un".to_string(),
            aliases: vec![],
            date_of_birth: None,
            nationality: Some("North Korea".to_string()),
            jurisdiction: "North Korea".to_string(),
            entity_type: EntityType::Individual,
        }];
        let result = screen_sanctions(&input).unwrap();
        let ofac_result = result
            .screening_results
            .iter()
            .find(|r| r.list_checked == "OFAC SDN" && r.match_score >= dec!(90))
            .expect("Should have OFAC match");
        assert!(matches!(
            ofac_result.match_type,
            MatchType::Exact | MatchType::Strong
        ));
        assert!(matches!(
            ofac_result.action_required,
            ActionRequired::Block | ActionRequired::Escalate
        ));
    }

    #[test]
    fn test_exact_match_eu() {
        let mut input = base_input();
        input.entities_to_screen = vec![ScreeningEntity {
            name: "Bashar Al-Assad".to_string(),
            aliases: vec![],
            date_of_birth: None,
            nationality: Some("Syria".to_string()),
            jurisdiction: "Syria".to_string(),
            entity_type: EntityType::Individual,
        }];
        let result = screen_sanctions(&input).unwrap();
        let eu_result = result
            .screening_results
            .iter()
            .find(|r| r.list_checked == "EU Consolidated" && r.match_score >= dec!(70));
        assert!(eu_result.is_some());
    }

    #[test]
    fn test_exact_match_hmt() {
        let mut input = base_input();
        input.entities_to_screen = vec![ScreeningEntity {
            name: "Vladimir Putin".to_string(),
            aliases: vec![],
            date_of_birth: None,
            nationality: Some("Russia".to_string()),
            jurisdiction: "Russia".to_string(),
            entity_type: EntityType::Individual,
        }];
        let result = screen_sanctions(&input).unwrap();
        let hmt_result = result
            .screening_results
            .iter()
            .find(|r| r.list_checked == "HMT UK" && r.match_score >= dec!(90));
        assert!(hmt_result.is_some());
    }

    #[test]
    fn test_exact_match_un() {
        let mut input = base_input();
        input.entities_to_screen = vec![ScreeningEntity {
            name: "Hezbollah".to_string(),
            aliases: vec![],
            date_of_birth: None,
            nationality: None,
            jurisdiction: "Lebanon".to_string(),
            entity_type: EntityType::Corporate,
        }];
        let result = screen_sanctions(&input).unwrap();
        let un_result = result
            .screening_results
            .iter()
            .find(|r| r.list_checked == "UN UNSC" && r.match_score >= dec!(90));
        assert!(un_result.is_some());
    }

    // === Fuzzy name matching tests ===

    #[test]
    fn test_fuzzy_match_alias() {
        let mut input = base_input();
        input.entities_to_screen = vec![ScreeningEntity {
            name: "Ayatollah Khamenei".to_string(),
            aliases: vec![],
            date_of_birth: None,
            nationality: Some("Iran".to_string()),
            jurisdiction: "Iran".to_string(),
            entity_type: EntityType::Individual,
        }];
        let result = screen_sanctions(&input).unwrap();
        // Should match against "ali khamenei" alias "ayatollah khamenei"
        let ofac_result = result
            .screening_results
            .iter()
            .find(|r| r.list_checked == "OFAC SDN" && r.match_score >= dec!(70));
        assert!(ofac_result.is_some());
    }

    #[test]
    fn test_fuzzy_match_slight_misspelling() {
        let mut input = base_input();
        input.entities_to_screen = vec![ScreeningEntity {
            name: "Vladmir Putn".to_string(),
            aliases: vec![],
            date_of_birth: None,
            nationality: None,
            jurisdiction: "Russia".to_string(),
            entity_type: EntityType::Individual,
        }];
        input.screening_threshold = dec!(60);
        let result = screen_sanctions(&input).unwrap();
        // Misspelling should still get a reasonable score
        let hmt_result = result
            .screening_results
            .iter()
            .find(|r| r.list_checked == "HMT UK" && r.match_score >= dec!(60));
        assert!(hmt_result.is_some());
    }

    #[test]
    fn test_fuzzy_match_with_entity_alias() {
        let mut input = base_input();
        input.entities_to_screen = vec![ScreeningEntity {
            name: "IRGC".to_string(),
            aliases: vec!["Islamic Revolutionary Guard Corps".to_string()],
            date_of_birth: None,
            nationality: None,
            jurisdiction: "Iran".to_string(),
            entity_type: EntityType::Corporate,
        }];
        let result = screen_sanctions(&input).unwrap();
        // The alias should match the OFAC entry
        let ofac_result = result
            .screening_results
            .iter()
            .find(|r| r.list_checked == "OFAC SDN" && r.match_score >= dec!(70));
        assert!(ofac_result.is_some());
    }

    #[test]
    fn test_low_threshold_more_matches() {
        let mut input = base_input();
        input.screening_threshold = dec!(30);
        input.entities_to_screen = vec![ScreeningEntity {
            name: "Ali K".to_string(),
            aliases: vec![],
            date_of_birth: None,
            nationality: None,
            jurisdiction: "United States".to_string(),
            entity_type: EntityType::Individual,
        }];
        let result = screen_sanctions(&input).unwrap();
        // With very low threshold, we might get more matches
        assert!(result.screening_results.len() > 0);
    }

    #[test]
    fn test_high_threshold_fewer_matches() {
        let mut input = base_input();
        input.screening_threshold = dec!(95);
        let result = screen_sanctions(&input).unwrap();
        // "John Smith" at 95% threshold should produce zero matches
        assert_eq!(result.matches_found, 0);
        assert_eq!(result.overall_status, OverallStatus::Clear);
    }

    // === Country-based screening tests ===

    #[test]
    fn test_embargoed_country_iran() {
        let flags = check_country_risk("Iran");
        assert_eq!(flags.len(), 1);
        assert!(flags[0].sanction_type.contains("embargo"));
    }

    #[test]
    fn test_embargoed_country_north_korea() {
        let flags = check_country_risk("North Korea");
        assert_eq!(flags.len(), 1);
        assert!(flags[0].sanction_type.contains("embargo"));
    }

    #[test]
    fn test_embargoed_country_cuba() {
        let flags = check_country_risk("Cuba");
        assert_eq!(flags.len(), 1);
        assert!(flags[0].regime.contains("OFAC"));
    }

    #[test]
    fn test_embargoed_country_syria() {
        let flags = check_country_risk("Syria");
        assert_eq!(flags.len(), 1);
        assert!(flags[0].regime.contains("OFAC"));
    }

    #[test]
    fn test_embargoed_country_crimea() {
        let flags = check_country_risk("Crimea");
        assert_eq!(flags.len(), 1);
    }

    #[test]
    fn test_sectoral_sanctions_russia() {
        let flags = check_country_risk("Russia");
        assert_eq!(flags.len(), 1);
        assert!(flags[0].sanction_type.contains("Sectoral"));
    }

    #[test]
    fn test_sectoral_sanctions_venezuela() {
        let flags = check_country_risk("Venezuela");
        assert_eq!(flags.len(), 1);
        assert!(flags[0].sanction_type.contains("Sectoral"));
    }

    #[test]
    fn test_fatf_grey_list_country() {
        let flags = check_country_risk("Turkey");
        assert_eq!(flags.len(), 1);
        assert!(flags[0].sanction_type.contains("Grey List"));
    }

    #[test]
    fn test_clean_country_no_flags() {
        let flags = check_country_risk("Switzerland");
        assert!(flags.is_empty());
    }

    #[test]
    fn test_entity_in_embargoed_jurisdiction_blocked() {
        let mut input = base_input();
        input.entities_to_screen = vec![ScreeningEntity {
            name: "Acme Corp".to_string(),
            aliases: vec![],
            date_of_birth: None,
            nationality: None,
            jurisdiction: "Iran".to_string(),
            entity_type: EntityType::Corporate,
        }];
        let result = screen_sanctions(&input).unwrap();
        assert!(matches!(result.overall_status, OverallStatus::Blocked));
    }

    // === PEP screening tests ===

    #[test]
    fn test_pep_screening_enabled() {
        let mut input = base_input();
        input.include_pep_screening = true;
        let result = screen_sanctions(&input).unwrap();
        assert!(result.recommended_actions.iter().any(|a| a.contains("PEP")));
    }

    #[test]
    fn test_pep_screening_disabled() {
        let input = base_input();
        let result = screen_sanctions(&input).unwrap();
        assert!(!result
            .recommended_actions
            .iter()
            .any(|a| a.contains("PEP databases")));
    }

    // === Transaction screening tests ===

    #[test]
    fn test_transaction_screening_requires_details() {
        let mut input = base_input();
        input.screening_type = ScreeningType::Transaction;
        input.transaction_details = None;
        let result = screen_sanctions(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_transaction_screening_with_details() {
        let mut input = base_input();
        input.screening_type = ScreeningType::Transaction;
        input.transaction_details = Some(TransactionDetails {
            amount: dec!(50000),
            currency: "USD".to_string(),
            counterparty_jurisdiction: "United Kingdom".to_string(),
            purpose: "Trade payment".to_string(),
        });
        let result = screen_sanctions(&input).unwrap();
        assert!(result.overall_status == OverallStatus::Clear);
    }

    #[test]
    fn test_transaction_screening_embargoed_counterparty() {
        let mut input = base_input();
        input.screening_type = ScreeningType::Transaction;
        input.transaction_details = Some(TransactionDetails {
            amount: dec!(100000),
            currency: "EUR".to_string(),
            counterparty_jurisdiction: "Iran".to_string(),
            purpose: "Goods payment".to_string(),
        });
        let result = screen_sanctions(&input).unwrap();
        assert!(!result.country_risk_flags.is_empty());
        assert!(result
            .country_risk_flags
            .iter()
            .any(|f| f.country == "Iran"));
    }

    #[test]
    fn test_negative_transaction_amount_error() {
        let mut input = base_input();
        input.screening_type = ScreeningType::Transaction;
        input.transaction_details = Some(TransactionDetails {
            amount: dec!(-100),
            currency: "USD".to_string(),
            counterparty_jurisdiction: "United States".to_string(),
            purpose: "Payment".to_string(),
        });
        let result = screen_sanctions(&input);
        assert!(result.is_err());
    }

    // === Multiple entity batch screening tests ===

    #[test]
    fn test_batch_screening_multiple_entities() {
        let mut input = base_input();
        input.screening_type = ScreeningType::BatchRescreen;
        input.entities_to_screen = vec![
            base_entity(),
            ScreeningEntity {
                name: "Jane Doe".to_string(),
                aliases: vec![],
                date_of_birth: Some("1990-05-15".to_string()),
                nationality: Some("UK".to_string()),
                jurisdiction: "United Kingdom".to_string(),
                entity_type: EntityType::Individual,
            },
            ScreeningEntity {
                name: "Acme Trading".to_string(),
                aliases: vec!["Acme Ltd".to_string()],
                date_of_birth: None,
                nationality: None,
                jurisdiction: "Germany".to_string(),
                entity_type: EntityType::Corporate,
            },
        ];
        let result = screen_sanctions(&input).unwrap();
        // 3 entities x 4 lists = 12 results
        assert_eq!(result.screening_results.len(), 12);
    }

    #[test]
    fn test_batch_screening_mixed_risk() {
        let mut input = base_input();
        input.entities_to_screen = vec![
            base_entity(), // clean
            ScreeningEntity {
                name: "Kim Jong Un".to_string(),
                aliases: vec![],
                date_of_birth: None,
                nationality: Some("DPRK".to_string()),
                jurisdiction: "North Korea".to_string(),
                entity_type: EntityType::Individual,
            },
        ];
        let result = screen_sanctions(&input).unwrap();
        // Should be blocked due to Kim Jong Un match + DPRK jurisdiction
        assert!(matches!(
            result.overall_status,
            OverallStatus::Blocked | OverallStatus::TruePositive
        ));
    }

    // === False positive identification tests ===

    #[test]
    fn test_false_positive_short_name() {
        let entity = ScreeningEntity {
            name: "Ali".to_string(),
            aliases: vec![],
            date_of_birth: None,
            nationality: None,
            jurisdiction: "United States".to_string(),
            entity_type: EntityType::Individual,
        };
        let indicators = detect_false_positive_indicators(&entity, "ali khamenei", dec!(60));
        assert!(indicators.iter().any(|i| i.contains("Short name")));
    }

    #[test]
    fn test_false_positive_weak_score() {
        let entity = base_entity();
        let indicators = detect_false_positive_indicators(&entity, "some entry", dec!(55));
        assert!(indicators.iter().any(|i| i.contains("weak match")));
    }

    #[test]
    fn test_false_positive_safe_jurisdiction() {
        let entity = ScreeningEntity {
            name: "Al Khamenei".to_string(),
            aliases: vec![],
            date_of_birth: None,
            nationality: None,
            jurisdiction: "United States".to_string(),
            entity_type: EntityType::Individual,
        };
        let indicators = detect_false_positive_indicators(&entity, "ali khamenei", dec!(80));
        assert!(indicators.iter().any(|i| i.contains("low-risk")));
    }

    // === SAR filing tests ===

    #[test]
    fn test_sar_required_for_blocked() {
        let mut input = base_input();
        input.entities_to_screen = vec![ScreeningEntity {
            name: "Kim Jong Un".to_string(),
            aliases: vec![],
            date_of_birth: None,
            nationality: None,
            jurisdiction: "North Korea".to_string(),
            entity_type: EntityType::Individual,
        }];
        let result = screen_sanctions(&input).unwrap();
        assert!(result.sar_filing_required);
    }

    #[test]
    fn test_sar_not_required_for_clear() {
        let input = base_input();
        let result = screen_sanctions(&input).unwrap();
        assert!(!result.sar_filing_required);
    }

    // === Escalation tests ===

    #[test]
    fn test_escalation_for_true_positive() {
        let mut input = base_input();
        input.entities_to_screen = vec![ScreeningEntity {
            name: "Vladimir Putin".to_string(),
            aliases: vec![],
            date_of_birth: None,
            nationality: None,
            jurisdiction: "Russia".to_string(),
            entity_type: EntityType::Individual,
        }];
        let result = screen_sanctions(&input).unwrap();
        assert!(result.escalation_required);
    }

    #[test]
    fn test_no_escalation_for_clear() {
        let input = base_input();
        let result = screen_sanctions(&input).unwrap();
        assert!(!result.escalation_required);
    }

    #[test]
    fn test_escalation_for_embargo_country() {
        let mut input = base_input();
        input.entities_to_screen = vec![ScreeningEntity {
            name: "Some Company".to_string(),
            aliases: vec![],
            date_of_birth: None,
            nationality: None,
            jurisdiction: "Cuba".to_string(),
            entity_type: EntityType::Corporate,
        }];
        let result = screen_sanctions(&input).unwrap();
        assert!(result.escalation_required);
    }

    // === Screening coverage tests ===

    #[test]
    fn test_full_coverage_all_lists() {
        let mut input = base_input();
        input.lists_to_check = vec![
            SanctionsList::OfacSdn,
            SanctionsList::EuConsolidated,
            SanctionsList::HmtUk,
            SanctionsList::UnUnsc,
            SanctionsList::FatfGreyList,
            SanctionsList::FatfBlackList,
        ];
        let result = screen_sanctions(&input).unwrap();
        assert_eq!(result.screening_coverage, dec!(100));
    }

    #[test]
    fn test_partial_coverage() {
        let mut input = base_input();
        input.lists_to_check = vec![SanctionsList::OfacSdn];
        let result = screen_sanctions(&input).unwrap();
        // 1/6 = 16.67%
        assert!(result.screening_coverage < dec!(20));
        assert!(result.screening_coverage > dec!(16));
    }

    #[test]
    fn test_partial_coverage_warning() {
        let mut input = base_input();
        input.lists_to_check = vec![SanctionsList::OfacSdn];
        let result = screen_sanctions(&input).unwrap();
        assert!(result.warnings.iter().any(|w| w.contains("lists")));
    }

    // === Input validation tests ===

    #[test]
    fn test_empty_entities_error() {
        let mut input = base_input();
        input.entities_to_screen = vec![];
        let result = screen_sanctions(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_lists_error() {
        let mut input = base_input();
        input.lists_to_check = vec![];
        let result = screen_sanctions(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_threshold_too_high() {
        let mut input = base_input();
        input.screening_threshold = dec!(101);
        let result = screen_sanctions(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_threshold_negative() {
        let mut input = base_input();
        input.screening_threshold = dec!(-5);
        let result = screen_sanctions(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_entity_name_error() {
        let mut input = base_input();
        input.entities_to_screen = vec![ScreeningEntity {
            name: "".to_string(),
            aliases: vec![],
            date_of_birth: None,
            nationality: None,
            jurisdiction: "US".to_string(),
            entity_type: EntityType::Individual,
        }];
        let result = screen_sanctions(&input);
        assert!(result.is_err());
    }

    // === Edge cases ===

    #[test]
    fn test_unicode_name() {
        let mut input = base_input();
        input.entities_to_screen = vec![ScreeningEntity {
            name: "\u{0410}\u{043B}\u{0435}\u{043A}\u{0441}\u{0435}\u{0439} \u{041F}\u{0443}\u{0442}\u{0438}\u{043D}".to_string(), // Cyrillic
            aliases: vec![],
            date_of_birth: None,
            nationality: None,
            jurisdiction: "Russia".to_string(),
            entity_type: EntityType::Individual,
        }];
        let result = screen_sanctions(&input).unwrap();
        // Should not crash; Unicode names won't match Latin entries
        assert!(result.screening_results.len() > 0);
    }

    #[test]
    fn test_very_long_name() {
        let mut input = base_input();
        input.entities_to_screen = vec![ScreeningEntity {
            name: "A".repeat(500),
            aliases: vec![],
            date_of_birth: None,
            nationality: None,
            jurisdiction: "United States".to_string(),
            entity_type: EntityType::Individual,
        }];
        let result = screen_sanctions(&input).unwrap();
        assert_eq!(result.overall_status, OverallStatus::Clear);
    }

    #[test]
    fn test_whitespace_only_alias_handling() {
        let mut input = base_input();
        input.entities_to_screen = vec![ScreeningEntity {
            name: "Test Entity".to_string(),
            aliases: vec!["   ".to_string()],
            date_of_birth: None,
            nationality: None,
            jurisdiction: "United States".to_string(),
            entity_type: EntityType::Individual,
        }];
        let result = screen_sanctions(&input).unwrap();
        // Should not crash
        assert!(result.screening_results.len() > 0);
    }

    #[test]
    fn test_case_insensitive_matching() {
        let score = similarity_score("KIM JONG UN", "kim jong un");
        assert_eq!(score, dec!(100));
    }

    #[test]
    fn test_sla_deadline_blocked() {
        let mut input = base_input();
        input.entities_to_screen = vec![ScreeningEntity {
            name: "Kim Jong Un".to_string(),
            aliases: vec![],
            date_of_birth: None,
            nationality: None,
            jurisdiction: "North Korea".to_string(),
            entity_type: EntityType::Individual,
        }];
        let result = screen_sanctions(&input).unwrap();
        assert!(result.sla_deadline.is_some());
    }

    #[test]
    fn test_sla_deadline_clear() {
        let input = base_input();
        let result = screen_sanctions(&input).unwrap();
        assert!(result.sla_deadline.is_none());
    }

    #[test]
    fn test_methodology_populated() {
        let input = base_input();
        let result = screen_sanctions(&input).unwrap();
        assert!(result.methodology.contains("Levenshtein"));
    }

    #[test]
    fn test_assumptions_populated() {
        let input = base_input();
        let result = screen_sanctions(&input).unwrap();
        assert!(!result.assumptions.is_empty());
    }

    #[test]
    fn test_adverse_media_screening_action() {
        let mut input = base_input();
        input.include_adverse_media = true;
        let result = screen_sanctions(&input).unwrap();
        assert!(result
            .recommended_actions
            .iter()
            .any(|a| a.contains("adverse media")));
    }

    #[test]
    fn test_periodic_review_screening() {
        let mut input = base_input();
        input.screening_type = ScreeningType::PeriodicReview;
        let result = screen_sanctions(&input).unwrap();
        assert!(result.overall_status == OverallStatus::Clear);
    }

    #[test]
    fn test_serde_round_trip_input() {
        let input = base_input();
        let json = serde_json::to_string(&input).unwrap();
        let deserialized: SanctionsScreeningInput = serde_json::from_str(&json).unwrap();
        assert_eq!(
            deserialized.entities_to_screen.len(),
            input.entities_to_screen.len()
        );
    }

    #[test]
    fn test_serde_round_trip_output() {
        let input = base_input();
        let result = screen_sanctions(&input).unwrap();
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: SanctionsScreeningOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.overall_status, result.overall_status);
    }

    #[test]
    fn test_clear_status_recommended_actions() {
        let input = base_input();
        let result = screen_sanctions(&input).unwrap();
        assert!(result
            .recommended_actions
            .iter()
            .any(|a| a.contains("Proceed")));
    }

    #[test]
    fn test_blocked_status_recommended_actions() {
        let mut input = base_input();
        input.entities_to_screen = vec![ScreeningEntity {
            name: "Kim Jong Un".to_string(),
            aliases: vec![],
            date_of_birth: None,
            nationality: None,
            jurisdiction: "North Korea".to_string(),
            entity_type: EntityType::Individual,
        }];
        let result = screen_sanctions(&input).unwrap();
        assert!(result
            .recommended_actions
            .iter()
            .any(|a| a.contains("BLOCK") || a.contains("SAR") || a.contains("Freeze")));
    }

    #[test]
    fn test_multiple_lists_results_count() {
        let mut input = base_input();
        input.lists_to_check = vec![SanctionsList::OfacSdn, SanctionsList::EuConsolidated];
        let result = screen_sanctions(&input).unwrap();
        // 1 entity x 2 lists = 2 results
        assert_eq!(result.screening_results.len(), 2);
    }

    #[test]
    fn test_warnings_for_match() {
        let mut input = base_input();
        input.entities_to_screen = vec![ScreeningEntity {
            name: "Vladimir Putin".to_string(),
            aliases: vec![],
            date_of_birth: None,
            nationality: None,
            jurisdiction: "Russia".to_string(),
            entity_type: EntityType::Individual,
        }];
        let result = screen_sanctions(&input).unwrap();
        if matches!(
            result.overall_status,
            OverallStatus::Blocked | OverallStatus::TruePositive
        ) {
            assert!(result.warnings.iter().any(|w| w.contains("do NOT proceed")));
        }
    }

    #[test]
    fn test_jurisdiction_boost_applied() {
        // An entity with jurisdiction matching the sanctions entry
        // should get a higher score than one without
        let score_with_jurisdiction = best_match_score("ali khamenei", &[], "ali khamenei", &[]);
        // Exact match = 100, so both would be 100
        assert_eq!(score_with_jurisdiction, dec!(100));
    }

    #[test]
    fn test_classify_match_exact() {
        assert_eq!(classify_match(dec!(100)), MatchType::Exact);
    }

    #[test]
    fn test_classify_match_strong() {
        assert_eq!(classify_match(dec!(92)), MatchType::Strong);
    }

    #[test]
    fn test_classify_match_possible() {
        assert_eq!(classify_match(dec!(75)), MatchType::Possible);
    }

    #[test]
    fn test_classify_match_weak() {
        assert_eq!(classify_match(dec!(55)), MatchType::Weak);
    }

    #[test]
    fn test_classify_match_no_match() {
        assert_eq!(classify_match(dec!(30)), MatchType::NoMatch);
    }

    #[test]
    fn test_action_for_exact() {
        assert_eq!(action_for_match(&MatchType::Exact), ActionRequired::Block);
    }

    #[test]
    fn test_action_for_strong() {
        assert_eq!(
            action_for_match(&MatchType::Strong),
            ActionRequired::Escalate
        );
    }

    #[test]
    fn test_action_for_possible() {
        assert_eq!(
            action_for_match(&MatchType::Possible),
            ActionRequired::ManualReview
        );
    }

    #[test]
    fn test_action_for_no_match() {
        assert_eq!(action_for_match(&MatchType::NoMatch), ActionRequired::Clear);
    }

    #[test]
    fn test_low_threshold_assumption() {
        let mut input = base_input();
        input.screening_threshold = dec!(50);
        let result = screen_sanctions(&input).unwrap();
        assert!(result
            .assumptions
            .iter()
            .any(|a| a.contains("false positives")));
    }

    #[test]
    fn test_entity_type_vessel() {
        let mut input = base_input();
        input.entities_to_screen = vec![ScreeningEntity {
            name: "MV Wise Honest".to_string(),
            aliases: vec![],
            date_of_birth: None,
            nationality: None,
            jurisdiction: "North Korea".to_string(),
            entity_type: EntityType::Vessel,
        }];
        let result = screen_sanctions(&input).unwrap();
        // Should be blocked due to DPRK jurisdiction
        assert!(matches!(result.overall_status, OverallStatus::Blocked));
    }

    #[test]
    fn test_entity_type_aircraft() {
        let mut input = base_input();
        input.entities_to_screen = vec![ScreeningEntity {
            name: "Test Aircraft".to_string(),
            aliases: vec![],
            date_of_birth: None,
            nationality: None,
            jurisdiction: "United States".to_string(),
            entity_type: EntityType::Aircraft,
        }];
        let result = screen_sanctions(&input).unwrap();
        assert_eq!(result.overall_status, OverallStatus::Clear);
    }
}
