use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{with_metadata, ComputationOutput, Money, Rate};
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Status of an individual HBU test.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HbuTestStatus {
    Passed,
    Failed { reason: String },
    NotEvaluated,
}

/// A potential use case evaluated in the HBU analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PotentialUse {
    /// Use type label, e.g. "Office", "Retail", "Multifamily", "Industrial"
    pub use_type: String,
    /// Maximum buildable square footage under zoning
    pub max_buildable_sf: Decimal,
    /// Estimated stabilised NOI per square foot
    pub estimated_noi_psf: Money,
    /// Market capitalisation rate for this use type
    pub estimated_cap_rate: Rate,
    /// Total development cost per square foot (hard + soft)
    pub development_cost_psf: Money,
    /// Estimated construction timeline in months
    pub construction_months: u32,
}

/// Zoning and land-use constraints for the site.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoningConstraints {
    /// Zoning use class, e.g. "C-3", "R-5", "M-1"
    pub use_class: String,
    /// Uses explicitly permitted under this zoning classification
    pub permitted_uses: Vec<String>,
    /// Maximum floor area ratio
    pub max_far: Decimal,
    /// Maximum building height in feet
    pub max_height_ft: Decimal,
    /// Minimum setback in feet from property line
    pub min_setback_ft: Decimal,
    /// Maximum lot coverage percentage (0.0 to 1.0)
    pub max_lot_coverage_pct: Rate,
}

/// Environmental constraints affecting development.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentalConstraints {
    /// Whether the site contains wetlands
    pub wetlands: bool,
    /// Whether the site is in a FEMA flood zone
    pub flood_zone: bool,
    /// Whether the site is a brownfield requiring remediation
    pub brownfield: bool,
    /// Estimated environmental remediation cost (if brownfield)
    pub remediation_cost: Money,
}

/// Deed restrictions and other private encumbrances.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeedRestrictions {
    /// List of uses prohibited by deed covenants
    pub prohibited_uses: Vec<String>,
    /// Whether there is a historic preservation easement
    pub historic_designation: bool,
    /// Maximum height allowed by deed (may be stricter than zoning)
    pub max_height_override_ft: Option<Decimal>,
}

/// Physical site characteristics relevant to development feasibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteCharacteristics {
    /// Total lot size in square feet
    pub lot_size_sf: Decimal,
    /// Lot shape: "regular" or "irregular"
    pub lot_shape: String,
    /// Topography: "flat", "sloped", or "steep"
    pub topography: String,
    /// Soil conditions: "good", "fair", or "poor"
    pub soil_conditions: String,
    /// Whether all utilities (water, sewer, electric, gas) are available
    pub utilities_available: bool,
    /// Access quality: "good", "fair", or "poor"
    pub access_quality: String,
}

/// A single use evaluated for legal permissibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegalUseResult {
    pub use_type: String,
    pub is_permitted: bool,
    pub constraints: Vec<String>,
}

/// A single use evaluated for physical possibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicalUseResult {
    pub use_type: String,
    pub is_possible: bool,
    pub adjusted_buildable_sf: Decimal,
    pub constraints: Vec<String>,
}

/// A single use evaluated for financial feasibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancialUseResult {
    pub use_type: String,
    pub is_feasible: bool,
    pub development_cost: Money,
    pub stabilised_noi: Money,
    pub capitalised_value: Money,
    pub residual_land_value: Money,
}

/// A single use ranked for maximal productivity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductiveUseResult {
    pub use_type: String,
    pub residual_land_value: Money,
    pub residual_per_sf: Money,
    pub irr: Rate,
    pub equity_multiple: Decimal,
    pub rank: u32,
}

// ---------------------------------------------------------------------------
// Inputs
// ---------------------------------------------------------------------------

/// Full HBU analysis input — orchestrates all 4 tests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HbuAnalysisInput {
    pub site_name: String,
    pub potential_uses: Vec<PotentialUse>,
    pub zoning: ZoningConstraints,
    pub deed_restrictions: DeedRestrictions,
    pub environmental: EnvironmentalConstraints,
    pub site: SiteCharacteristics,
    /// Discount rate used for IRR benchmarking
    pub discount_rate: Rate,
}

/// Input for the legal permissibility test (test 1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegalPermissibleInput {
    pub potential_uses: Vec<PotentialUse>,
    pub zoning: ZoningConstraints,
    pub deed_restrictions: DeedRestrictions,
    pub environmental: EnvironmentalConstraints,
}

/// Input for the physical possibility test (test 2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicallyPossibleInput {
    pub legal_uses: Vec<PotentialUse>,
    pub site: SiteCharacteristics,
    pub environmental: EnvironmentalConstraints,
}

/// Input for the financial feasibility test (test 3).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinanciallyFeasibleInput {
    pub physical_uses: Vec<PotentialUse>,
    pub site: SiteCharacteristics,
}

/// Input for the maximally productive test (test 4).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaximallyProductiveInput {
    pub feasible_uses: Vec<FinancialUseResult>,
    pub potential_uses: Vec<PotentialUse>,
    pub lot_size_sf: Decimal,
    pub discount_rate: Rate,
}

// ---------------------------------------------------------------------------
// Outputs
// ---------------------------------------------------------------------------

/// Output for the legal permissibility test.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegalPermissibleOutput {
    pub results: Vec<LegalUseResult>,
    pub permitted_uses: Vec<String>,
}

/// Output for the physical possibility test.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicallyPossibleOutput {
    pub results: Vec<PhysicalUseResult>,
    pub possible_uses: Vec<String>,
}

/// Output for the financial feasibility test.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinanciallyFeasibleOutput {
    pub results: Vec<FinancialUseResult>,
    pub feasible_uses: Vec<String>,
}

/// Output for the maximally productive test.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaximallyProductiveOutput {
    pub ranked_uses: Vec<ProductiveUseResult>,
    pub highest_and_best_use: String,
    pub highest_residual_per_sf: Money,
}

/// Full HBU analysis output with all four test results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HbuAnalysisOutput {
    pub site_name: String,
    pub legal_test: HbuTestStatus,
    pub physical_test: HbuTestStatus,
    pub financial_test: HbuTestStatus,
    pub maximal_test: HbuTestStatus,
    pub legal_results: Option<LegalPermissibleOutput>,
    pub physical_results: Option<PhysicallyPossibleOutput>,
    pub financial_results: Option<FinanciallyFeasibleOutput>,
    pub maximal_results: Option<MaximallyProductiveOutput>,
    pub conclusion: String,
    pub recommended_use: Option<String>,
}

// ---------------------------------------------------------------------------
// 1. Full HBU Analysis (orchestrator)
// ---------------------------------------------------------------------------

/// Orchestrates the full 4-test HBU framework sequentially.
///
/// RE-CONTRACT-003: Tests run in order — legal, physical, financial, maximal.
/// If any test fails (zero passing uses), subsequent tests are skipped with
/// status `NotEvaluated`.
pub fn hbu_analysis(
    input: &HbuAnalysisInput,
) -> CorpFinanceResult<ComputationOutput<HbuAnalysisOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    if input.potential_uses.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "potential_uses".into(),
            reason: "at least one potential use is required".into(),
        });
    }

    // Validate cap rates (RE-CONTRACT-004)
    for u in &input.potential_uses {
        validate_cap_rate(u.estimated_cap_rate, &u.use_type)?;
    }

    // --- Test 1: Legal Permissibility ---
    let legal_input = LegalPermissibleInput {
        potential_uses: input.potential_uses.clone(),
        zoning: input.zoning.clone(),
        deed_restrictions: input.deed_restrictions.clone(),
        environmental: input.environmental.clone(),
    };
    let legal_output = legal_permissible(&legal_input)?;
    let legal_result = legal_output.result;

    if legal_result.permitted_uses.is_empty() {
        return Ok(with_metadata(
            "Highest and Best Use — 4-Test Framework",
            &serde_json::json!({"site": input.site_name}),
            warnings,
            start.elapsed().as_micros() as u64,
            HbuAnalysisOutput {
                site_name: input.site_name.clone(),
                legal_test: HbuTestStatus::Failed {
                    reason: "No legally permissible uses identified".into(),
                },
                physical_test: HbuTestStatus::NotEvaluated,
                financial_test: HbuTestStatus::NotEvaluated,
                maximal_test: HbuTestStatus::NotEvaluated,
                legal_results: Some(legal_result),
                physical_results: None,
                financial_results: None,
                maximal_results: None,
                conclusion: "HBU analysis terminated: no legally permissible uses".into(),
                recommended_use: None,
            },
        ));
    }

    // Filter potential uses to only legally permitted ones
    let legal_uses: Vec<PotentialUse> = input
        .potential_uses
        .iter()
        .filter(|u| legal_result.permitted_uses.contains(&u.use_type))
        .cloned()
        .collect();

    // --- Test 2: Physical Possibility ---
    let physical_input = PhysicallyPossibleInput {
        legal_uses,
        site: input.site.clone(),
        environmental: input.environmental.clone(),
    };
    let physical_output = physically_possible(&physical_input)?;
    let physical_result = physical_output.result;

    if physical_result.possible_uses.is_empty() {
        return Ok(with_metadata(
            "Highest and Best Use — 4-Test Framework",
            &serde_json::json!({"site": input.site_name}),
            warnings,
            start.elapsed().as_micros() as u64,
            HbuAnalysisOutput {
                site_name: input.site_name.clone(),
                legal_test: HbuTestStatus::Passed,
                physical_test: HbuTestStatus::Failed {
                    reason: "No physically possible uses identified".into(),
                },
                financial_test: HbuTestStatus::NotEvaluated,
                maximal_test: HbuTestStatus::NotEvaluated,
                legal_results: Some(legal_result),
                physical_results: Some(physical_result),
                financial_results: None,
                maximal_results: None,
                conclusion: "HBU analysis terminated: no physically possible uses".into(),
                recommended_use: None,
            },
        ));
    }

    // Build physical uses with adjusted SF
    let physical_uses: Vec<PotentialUse> = input
        .potential_uses
        .iter()
        .filter(|u| physical_result.possible_uses.contains(&u.use_type))
        .cloned()
        .map(|mut u| {
            // Apply adjusted SF from physical analysis
            if let Some(pr) = physical_result
                .results
                .iter()
                .find(|r| r.use_type == u.use_type && r.is_possible)
            {
                u.max_buildable_sf = pr.adjusted_buildable_sf;
            }
            u
        })
        .collect();

    // --- Test 3: Financial Feasibility ---
    let financial_input = FinanciallyFeasibleInput {
        physical_uses,
        site: input.site.clone(),
    };
    let financial_output = financially_feasible(&financial_input)?;
    let financial_result = financial_output.result;

    if financial_result.feasible_uses.is_empty() {
        return Ok(with_metadata(
            "Highest and Best Use — 4-Test Framework",
            &serde_json::json!({"site": input.site_name}),
            warnings,
            start.elapsed().as_micros() as u64,
            HbuAnalysisOutput {
                site_name: input.site_name.clone(),
                legal_test: HbuTestStatus::Passed,
                physical_test: HbuTestStatus::Passed,
                financial_test: HbuTestStatus::Failed {
                    reason: "No financially feasible uses identified".into(),
                },
                maximal_test: HbuTestStatus::NotEvaluated,
                legal_results: Some(legal_result),
                physical_results: Some(physical_result),
                financial_results: Some(financial_result),
                maximal_results: None,
                conclusion: "HBU analysis terminated: no financially feasible uses".into(),
                recommended_use: None,
            },
        ));
    }

    // --- Test 4: Maximally Productive ---
    let feasible_results: Vec<FinancialUseResult> = financial_result
        .results
        .iter()
        .filter(|r| r.is_feasible)
        .cloned()
        .collect();

    let feasible_potential_uses: Vec<PotentialUse> = input
        .potential_uses
        .iter()
        .filter(|u| financial_result.feasible_uses.contains(&u.use_type))
        .cloned()
        .collect();

    let maximal_input = MaximallyProductiveInput {
        feasible_uses: feasible_results,
        potential_uses: feasible_potential_uses,
        lot_size_sf: input.site.lot_size_sf,
        discount_rate: input.discount_rate,
    };
    let maximal_output = maximally_productive(&maximal_input)?;
    let maximal_result = maximal_output.result;

    let recommended = maximal_result.highest_and_best_use.clone();
    let conclusion = format!(
        "Highest and best use is {} with residual land value of ${} per SF",
        recommended, maximal_result.highest_residual_per_sf
    );

    warnings.extend(legal_output.warnings);
    warnings.extend(physical_output.warnings);
    warnings.extend(financial_output.warnings);
    warnings.extend(maximal_output.warnings);

    Ok(with_metadata(
        "Highest and Best Use — 4-Test Framework",
        &serde_json::json!({"site": input.site_name}),
        warnings,
        start.elapsed().as_micros() as u64,
        HbuAnalysisOutput {
            site_name: input.site_name.clone(),
            legal_test: HbuTestStatus::Passed,
            physical_test: HbuTestStatus::Passed,
            financial_test: HbuTestStatus::Passed,
            maximal_test: HbuTestStatus::Passed,
            legal_results: Some(legal_result),
            physical_results: Some(physical_result),
            financial_results: Some(financial_result),
            maximal_results: Some(maximal_result),
            conclusion,
            recommended_use: Some(recommended),
        },
    ))
}

// ---------------------------------------------------------------------------
// 2. Legal Permissibility Test
// ---------------------------------------------------------------------------

/// Evaluate which potential uses are legally permissible given zoning,
/// deed restrictions, and environmental constraints.
pub fn legal_permissible(
    input: &LegalPermissibleInput,
) -> CorpFinanceResult<ComputationOutput<LegalPermissibleOutput>> {
    let start = Instant::now();
    let warnings: Vec<String> = Vec::new();

    let mut results: Vec<LegalUseResult> = Vec::new();

    for potential in &input.potential_uses {
        let mut constraints: Vec<String> = Vec::new();
        let mut is_permitted = true;

        // Check zoning permitted uses
        if !input
            .zoning
            .permitted_uses
            .iter()
            .any(|p| p.eq_ignore_ascii_case(&potential.use_type))
        {
            is_permitted = false;
            constraints.push(format!(
                "Use '{}' not permitted under zoning class '{}'",
                potential.use_type, input.zoning.use_class
            ));
        }

        // Check deed restriction prohibited uses
        if input
            .deed_restrictions
            .prohibited_uses
            .iter()
            .any(|p| p.eq_ignore_ascii_case(&potential.use_type))
        {
            is_permitted = false;
            constraints.push(format!(
                "Use '{}' prohibited by deed restriction",
                potential.use_type
            ));
        }

        // Check historic designation — restricts demolition and new construction
        if input.deed_restrictions.historic_designation {
            constraints.push("Historic designation restricts exterior modifications".into());
            // Historic designation does not necessarily prohibit use, but constrains it
        }

        // Check environmental: wetlands may prohibit certain development
        if input.environmental.wetlands {
            constraints
                .push("Wetlands present — may require Army Corps of Engineers permit".into());
        }

        // Check environmental: flood zone adds regulatory burden
        if input.environmental.flood_zone {
            constraints.push(
                "FEMA flood zone — requires flood-resistant construction and insurance".into(),
            );
        }

        // Check environmental: brownfield requires remediation before development
        if input.environmental.brownfield {
            constraints.push("Brownfield site — environmental remediation required".into());
        }

        // Check height against both zoning and deed restriction overrides
        let effective_max_height = match input.deed_restrictions.max_height_override_ft {
            Some(deed_h) if deed_h < input.zoning.max_height_ft => {
                constraints.push(format!(
                    "Deed restricts height to {} ft (stricter than zoning {} ft)",
                    deed_h, input.zoning.max_height_ft
                ));
                deed_h
            }
            _ => input.zoning.max_height_ft,
        };

        // Estimate required height from buildable SF / lot coverage
        let max_footprint = input.zoning.max_lot_coverage_pct * dec!(10000); // proxy
        if max_footprint > Decimal::ZERO && effective_max_height < dec!(10) {
            constraints.push(format!(
                "Very low height limit ({} ft) severely constrains development",
                effective_max_height
            ));
        }

        results.push(LegalUseResult {
            use_type: potential.use_type.clone(),
            is_permitted,
            constraints,
        });
    }

    let permitted_uses: Vec<String> = results
        .iter()
        .filter(|r| r.is_permitted)
        .map(|r| r.use_type.clone())
        .collect();

    Ok(with_metadata(
        "HBU Test 1 — Legal Permissibility",
        &serde_json::json!({
            "zoning_class": input.zoning.use_class,
            "max_far": input.zoning.max_far.to_string(),
            "max_height_ft": input.zoning.max_height_ft.to_string(),
        }),
        warnings,
        start.elapsed().as_micros() as u64,
        LegalPermissibleOutput {
            results,
            permitted_uses,
        },
    ))
}

// ---------------------------------------------------------------------------
// 3. Physical Possibility Test
// ---------------------------------------------------------------------------

/// Filter legally permissible uses to those physically achievable on the site.
pub fn physically_possible(
    input: &PhysicallyPossibleInput,
) -> CorpFinanceResult<ComputationOutput<PhysicallyPossibleOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    if input.site.lot_size_sf <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "lot_size_sf".into(),
            reason: "lot size must be positive".into(),
        });
    }

    let mut results: Vec<PhysicalUseResult> = Vec::new();

    for potential in &input.legal_uses {
        let mut constraints: Vec<String> = Vec::new();
        let mut is_possible = true;

        // Lot shape adjustment — irregular lots reduce usable area
        let shape_factor = match input.site.lot_shape.to_lowercase().as_str() {
            "regular" => dec!(1.0),
            "irregular" => {
                constraints.push("Irregular lot shape reduces buildable area by ~15%".into());
                dec!(0.85)
            }
            _ => {
                warnings.push(format!(
                    "Unknown lot shape '{}' — treating as irregular",
                    input.site.lot_shape
                ));
                dec!(0.85)
            }
        };

        // Topography adjustment
        let topo_factor = match input.site.topography.to_lowercase().as_str() {
            "flat" => dec!(1.0),
            "sloped" => {
                constraints.push("Sloped topography increases grading costs by ~10%".into());
                dec!(0.95)
            }
            "steep" => {
                constraints.push("Steep topography significantly constrains construction".into());
                dec!(0.80)
            }
            _ => {
                warnings.push(format!(
                    "Unknown topography '{}' — treating as sloped",
                    input.site.topography
                ));
                dec!(0.95)
            }
        };

        // Soil conditions adjustment
        let soil_factor = match input.site.soil_conditions.to_lowercase().as_str() {
            "good" => dec!(1.0),
            "fair" => {
                constraints.push("Fair soil conditions may require enhanced foundations".into());
                dec!(0.95)
            }
            "poor" => {
                constraints.push(
                    "Poor soil conditions require deep foundations — significant cost impact"
                        .into(),
                );
                dec!(0.85)
            }
            _ => {
                warnings.push(format!(
                    "Unknown soil conditions '{}' — treating as fair",
                    input.site.soil_conditions
                ));
                dec!(0.95)
            }
        };

        // Utilities check
        if !input.site.utilities_available {
            constraints.push("Utilities not available — infrastructure extension required".into());
            // Not a hard fail, but costly
        }

        // Access quality
        let access_factor = match input.site.access_quality.to_lowercase().as_str() {
            "good" => dec!(1.0),
            "fair" => {
                constraints.push("Fair access quality may limit some commercial uses".into());
                dec!(0.95)
            }
            "poor" => {
                constraints
                    .push("Poor access significantly constrains development potential".into());
                dec!(0.80)
            }
            _ => dec!(0.95),
        };

        // Wetlands reduce usable area
        let wetland_factor = if input.environmental.wetlands {
            constraints.push("Wetlands reduce developable area by ~25%".into());
            dec!(0.75)
        } else {
            dec!(1.0)
        };

        // Calculate adjusted buildable SF
        let composite_factor =
            shape_factor * topo_factor * soil_factor * access_factor * wetland_factor;

        let adjusted_sf = potential.max_buildable_sf * composite_factor;

        // Minimum viable size check — below 5,000 SF is generally not feasible
        // for institutional development
        let min_viable_sf = dec!(5000);
        if adjusted_sf < min_viable_sf {
            is_possible = false;
            constraints.push(format!(
                "Adjusted buildable area ({} SF) below minimum viable threshold ({} SF)",
                adjusted_sf.round_dp(0),
                min_viable_sf
            ));
        }

        // Steep + poor soil + poor access = not feasible
        if input.site.topography.to_lowercase() == "steep"
            && input.site.soil_conditions.to_lowercase() == "poor"
            && input.site.access_quality.to_lowercase() == "poor"
        {
            is_possible = false;
            constraints.push(
                "Combination of steep topography, poor soil, and poor access renders site undevelopable".into(),
            );
        }

        results.push(PhysicalUseResult {
            use_type: potential.use_type.clone(),
            is_possible,
            adjusted_buildable_sf: adjusted_sf.round_dp(0),
            constraints,
        });
    }

    let possible_uses: Vec<String> = results
        .iter()
        .filter(|r| r.is_possible)
        .map(|r| r.use_type.clone())
        .collect();

    Ok(with_metadata(
        "HBU Test 2 — Physical Possibility",
        &serde_json::json!({
            "lot_size_sf": input.site.lot_size_sf.to_string(),
            "lot_shape": input.site.lot_shape,
            "topography": input.site.topography,
        }),
        warnings,
        start.elapsed().as_micros() as u64,
        PhysicallyPossibleOutput {
            results,
            possible_uses,
        },
    ))
}

// ---------------------------------------------------------------------------
// 4. Financial Feasibility Test
// ---------------------------------------------------------------------------

/// Determine which physically possible uses produce positive residual land value.
///
/// Residual land value = (stabilised NOI / cap_rate) - total development cost
///
/// RE-CONTRACT-004: Cap rate must be positive and < 1.0.
pub fn financially_feasible(
    input: &FinanciallyFeasibleInput,
) -> CorpFinanceResult<ComputationOutput<FinanciallyFeasibleOutput>> {
    let start = Instant::now();
    let warnings: Vec<String> = Vec::new();

    let mut results: Vec<FinancialUseResult> = Vec::new();

    for potential in &input.physical_uses {
        validate_cap_rate(potential.estimated_cap_rate, &potential.use_type)?;

        let buildable_sf = potential.max_buildable_sf;

        // Total development cost = (hard + soft cost per SF) * buildable SF
        // Soft costs typically 20-30% of hard costs — included in development_cost_psf
        let development_cost = potential.development_cost_psf * buildable_sf;

        // Stabilised NOI = NOI per SF * buildable SF
        let stabilised_noi = potential.estimated_noi_psf * buildable_sf;

        // Capitalised value = stabilised NOI / cap rate
        let capitalised_value = stabilised_noi / potential.estimated_cap_rate;

        // Residual land value = capitalised value - development cost
        let residual_land_value = capitalised_value - development_cost;

        let is_feasible = residual_land_value > Decimal::ZERO;

        results.push(FinancialUseResult {
            use_type: potential.use_type.clone(),
            is_feasible,
            development_cost: development_cost.round_dp(2),
            stabilised_noi: stabilised_noi.round_dp(2),
            capitalised_value: capitalised_value.round_dp(2),
            residual_land_value: residual_land_value.round_dp(2),
        });
    }

    let feasible_uses: Vec<String> = results
        .iter()
        .filter(|r| r.is_feasible)
        .map(|r| r.use_type.clone())
        .collect();

    Ok(with_metadata(
        "HBU Test 3 — Financial Feasibility",
        &serde_json::json!({"method": "residual_land_value"}),
        warnings,
        start.elapsed().as_micros() as u64,
        FinanciallyFeasibleOutput {
            results,
            feasible_uses,
        },
    ))
}

// ---------------------------------------------------------------------------
// 5. Maximally Productive Test
// ---------------------------------------------------------------------------

/// Rank feasible uses by residual land value per SF of site area.
/// Also computes IRR and equity multiple for each alternative.
pub fn maximally_productive(
    input: &MaximallyProductiveInput,
) -> CorpFinanceResult<ComputationOutput<MaximallyProductiveOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    if input.feasible_uses.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "feasible_uses".into(),
            reason: "at least one feasible use is required".into(),
        });
    }

    if input.lot_size_sf <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "lot_size_sf".into(),
            reason: "lot size must be positive".into(),
        });
    }

    let mut ranked: Vec<ProductiveUseResult> = Vec::new();

    for feasible in &input.feasible_uses {
        let residual_per_sf = feasible.residual_land_value / input.lot_size_sf;

        // Find matching potential use for construction timeline
        let construction_months = input
            .potential_uses
            .iter()
            .find(|p| p.use_type == feasible.use_type)
            .map(|p| p.construction_months)
            .unwrap_or(24);

        // Build simplified cash flow for IRR:
        // t=0: -(development_cost + land_value)  [land_value ~ residual]
        // t=1..construction_years: 0 (construction period)
        // t=construction_years+1: stabilised value (capitalised_value)
        let total_investment = feasible.development_cost;
        let construction_years = (Decimal::from(construction_months) / dec!(12))
            .round_dp(0)
            .max(Decimal::ONE);
        let construction_periods = construction_years.to_string().parse::<usize>().unwrap_or(2);

        let holding_years = construction_periods + 5; // 5-year hold post-stabilisation
        let mut cash_flows: Vec<Decimal> = Vec::with_capacity(holding_years + 1);

        // t=0: total investment outflow
        cash_flows.push(-total_investment);

        // Construction period: no income
        for _ in 0..construction_periods {
            cash_flows.push(Decimal::ZERO);
        }

        // Operating years: annual NOI
        let annual_noi = feasible.stabilised_noi;
        for _ in 0..5 {
            cash_flows.push(annual_noi);
        }

        // Terminal year: add reversion (capitalised value) to final NOI
        if let Some(last) = cash_flows.last_mut() {
            *last += feasible.capitalised_value;
        }

        let irr = newton_raphson_irr(&cash_flows, &mut warnings);

        // Equity multiple = total inflows / total outflows
        let total_inflows: Decimal = cash_flows.iter().filter(|cf| **cf > Decimal::ZERO).sum();
        let total_outflows: Decimal = cash_flows
            .iter()
            .filter(|cf| **cf < Decimal::ZERO)
            .map(|cf| cf.abs())
            .sum();

        let equity_multiple = if total_outflows > Decimal::ZERO {
            (total_inflows / total_outflows).round_dp(2)
        } else {
            Decimal::ZERO
        };

        ranked.push(ProductiveUseResult {
            use_type: feasible.use_type.clone(),
            residual_land_value: feasible.residual_land_value.round_dp(2),
            residual_per_sf: residual_per_sf.round_dp(2),
            irr: irr.round_dp(4),
            equity_multiple,
            rank: 0, // assigned below
        });
    }

    // Sort by residual per SF descending
    ranked.sort_by(|a, b| b.residual_per_sf.cmp(&a.residual_per_sf));

    // Assign ranks
    for (i, item) in ranked.iter_mut().enumerate() {
        item.rank = (i + 1) as u32;
    }

    let highest_and_best_use = ranked
        .first()
        .map(|r| r.use_type.clone())
        .unwrap_or_default();

    let highest_residual_per_sf = ranked
        .first()
        .map(|r| r.residual_per_sf)
        .unwrap_or(Decimal::ZERO);

    Ok(with_metadata(
        "HBU Test 4 — Maximally Productive",
        &serde_json::json!({
            "lot_size_sf": input.lot_size_sf.to_string(),
            "ranking_metric": "residual_land_value_per_sf",
        }),
        warnings,
        start.elapsed().as_micros() as u64,
        MaximallyProductiveOutput {
            ranked_uses: ranked,
            highest_and_best_use,
            highest_residual_per_sf,
        },
    ))
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Validate cap rate per RE-CONTRACT-004.
fn validate_cap_rate(cap_rate: Rate, use_type: &str) -> CorpFinanceResult<()> {
    if cap_rate <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: format!("estimated_cap_rate ({})", use_type),
            reason: "cap rate must be positive".into(),
        });
    }
    if cap_rate >= Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: format!("estimated_cap_rate ({})", use_type),
            reason: "cap rate must be less than 1.0 (100%)".into(),
        });
    }
    Ok(())
}

/// Newton-Raphson IRR solver.
/// cash_flows[0] is typically negative (investment).
/// Returns the rate r where NPV(r) = 0.
fn newton_raphson_irr(cash_flows: &[Decimal], warnings: &mut Vec<String>) -> Decimal {
    let max_iter: u32 = 30;
    let epsilon = dec!(0.0000001);
    let mut rate = dec!(0.10);

    for _ in 0..max_iter {
        let (npv, dnpv) = npv_and_derivative(cash_flows, rate);

        if dnpv.abs() < dec!(0.000000001) {
            warnings.push("IRR: derivative near zero — result may be imprecise".into());
            break;
        }

        let new_rate = rate - npv / dnpv;

        if (new_rate - rate).abs() < epsilon {
            return new_rate;
        }

        rate = new_rate;

        // Guard against runaway
        if rate < dec!(-0.99) {
            rate = dec!(-0.99);
        }
        if rate > dec!(10.0) {
            rate = dec!(10.0);
        }
    }

    rate
}

/// NPV(r) = sum CF_t / (1+r)^t and its derivative d(NPV)/dr.
fn npv_and_derivative(cash_flows: &[Decimal], rate: Decimal) -> (Decimal, Decimal) {
    let one_plus_r = Decimal::ONE + rate;
    let mut npv = Decimal::ZERO;
    let mut dnpv = Decimal::ZERO;
    let mut discount = Decimal::ONE;

    for (t, cf) in cash_flows.iter().enumerate() {
        npv += *cf * discount;
        if t > 0 {
            dnpv += Decimal::from(-(t as i64)) * *cf * discount / one_plus_r;
        }
        discount /= one_plus_r;
    }

    (npv, dnpv)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- Test helpers ---

    fn sample_zoning() -> ZoningConstraints {
        ZoningConstraints {
            use_class: "C-3".into(),
            permitted_uses: vec!["Office".into(), "Retail".into(), "Multifamily".into()],
            max_far: dec!(4.0),
            max_height_ft: dec!(120),
            min_setback_ft: dec!(10),
            max_lot_coverage_pct: dec!(0.80),
        }
    }

    fn sample_deed() -> DeedRestrictions {
        DeedRestrictions {
            prohibited_uses: vec![],
            historic_designation: false,
            max_height_override_ft: None,
        }
    }

    fn sample_environmental() -> EnvironmentalConstraints {
        EnvironmentalConstraints {
            wetlands: false,
            flood_zone: false,
            brownfield: false,
            remediation_cost: Decimal::ZERO,
        }
    }

    fn sample_site() -> SiteCharacteristics {
        SiteCharacteristics {
            lot_size_sf: dec!(50000),
            lot_shape: "regular".into(),
            topography: "flat".into(),
            soil_conditions: "good".into(),
            utilities_available: true,
            access_quality: "good".into(),
        }
    }

    fn sample_potential_uses() -> Vec<PotentialUse> {
        vec![
            PotentialUse {
                use_type: "Office".into(),
                max_buildable_sf: dec!(200000),
                estimated_noi_psf: dec!(35),
                estimated_cap_rate: dec!(0.06),
                development_cost_psf: dec!(350),
                construction_months: 24,
            },
            PotentialUse {
                use_type: "Retail".into(),
                max_buildable_sf: dec!(100000),
                estimated_noi_psf: dec!(40),
                estimated_cap_rate: dec!(0.065),
                development_cost_psf: dec!(300),
                construction_months: 18,
            },
            PotentialUse {
                use_type: "Multifamily".into(),
                max_buildable_sf: dec!(180000),
                estimated_noi_psf: dec!(25),
                estimated_cap_rate: dec!(0.05),
                development_cost_psf: dec!(280),
                construction_months: 30,
            },
            PotentialUse {
                use_type: "Industrial".into(),
                max_buildable_sf: dec!(250000),
                estimated_noi_psf: dec!(12),
                estimated_cap_rate: dec!(0.07),
                development_cost_psf: dec!(150),
                construction_months: 12,
            },
        ]
    }

    // -----------------------------------------------------------------------
    // Legal Permissibility Tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_legal_all_permitted() {
        let input = LegalPermissibleInput {
            potential_uses: sample_potential_uses(),
            zoning: sample_zoning(),
            deed_restrictions: sample_deed(),
            environmental: sample_environmental(),
        };
        let output = legal_permissible(&input).unwrap();
        let r = &output.result;
        // Office, Retail, Multifamily permitted; Industrial not
        assert_eq!(r.permitted_uses.len(), 3);
        assert!(r.permitted_uses.contains(&"Office".to_string()));
        assert!(r.permitted_uses.contains(&"Retail".to_string()));
        assert!(r.permitted_uses.contains(&"Multifamily".to_string()));
        assert!(!r.permitted_uses.contains(&"Industrial".to_string()));
    }

    #[test]
    fn test_legal_deed_prohibition() {
        let mut deed = sample_deed();
        deed.prohibited_uses = vec!["Retail".into()];
        let input = LegalPermissibleInput {
            potential_uses: sample_potential_uses(),
            zoning: sample_zoning(),
            deed_restrictions: deed,
            environmental: sample_environmental(),
        };
        let output = legal_permissible(&input).unwrap();
        assert!(!output.result.permitted_uses.contains(&"Retail".to_string()));
        assert_eq!(output.result.permitted_uses.len(), 2);
    }

    #[test]
    fn test_legal_historic_designation() {
        let mut deed = sample_deed();
        deed.historic_designation = true;
        let input = LegalPermissibleInput {
            potential_uses: sample_potential_uses(),
            zoning: sample_zoning(),
            deed_restrictions: deed,
            environmental: sample_environmental(),
        };
        let output = legal_permissible(&input).unwrap();
        // Historic designation does not prohibit uses, just constrains
        assert_eq!(output.result.permitted_uses.len(), 3);
        let office = output
            .result
            .results
            .iter()
            .find(|r| r.use_type == "Office")
            .unwrap();
        assert!(office.constraints.iter().any(|c| c.contains("Historic")));
    }

    #[test]
    fn test_legal_environmental_constraints() {
        let env = EnvironmentalConstraints {
            wetlands: true,
            flood_zone: true,
            brownfield: true,
            remediation_cost: dec!(500000),
        };
        let input = LegalPermissibleInput {
            potential_uses: sample_potential_uses(),
            zoning: sample_zoning(),
            deed_restrictions: sample_deed(),
            environmental: env,
        };
        let output = legal_permissible(&input).unwrap();
        let office = output
            .result
            .results
            .iter()
            .find(|r| r.use_type == "Office")
            .unwrap();
        assert!(office.constraints.iter().any(|c| c.contains("Wetlands")));
        assert!(office.constraints.iter().any(|c| c.contains("flood zone")));
        assert!(office.constraints.iter().any(|c| c.contains("Brownfield")));
    }

    #[test]
    fn test_legal_deed_height_override() {
        let mut deed = sample_deed();
        deed.max_height_override_ft = Some(dec!(50));
        let input = LegalPermissibleInput {
            potential_uses: sample_potential_uses(),
            zoning: sample_zoning(),
            deed_restrictions: deed,
            environmental: sample_environmental(),
        };
        let output = legal_permissible(&input).unwrap();
        let office = output
            .result
            .results
            .iter()
            .find(|r| r.use_type == "Office")
            .unwrap();
        assert!(office
            .constraints
            .iter()
            .any(|c| c.contains("Deed restricts height")));
    }

    #[test]
    fn test_legal_no_permitted_uses() {
        let zoning = ZoningConstraints {
            use_class: "P-1".into(),
            permitted_uses: vec!["Park".into()],
            max_far: dec!(0.1),
            max_height_ft: dec!(15),
            min_setback_ft: dec!(50),
            max_lot_coverage_pct: dec!(0.10),
        };
        let input = LegalPermissibleInput {
            potential_uses: sample_potential_uses(),
            zoning,
            deed_restrictions: sample_deed(),
            environmental: sample_environmental(),
        };
        let output = legal_permissible(&input).unwrap();
        assert!(output.result.permitted_uses.is_empty());
    }

    // -----------------------------------------------------------------------
    // Physical Possibility Tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_physical_all_possible() {
        let uses = vec![
            sample_potential_uses()[0].clone(), // Office
            sample_potential_uses()[1].clone(), // Retail
            sample_potential_uses()[2].clone(), // Multifamily
        ];
        let input = PhysicallyPossibleInput {
            legal_uses: uses,
            site: sample_site(),
            environmental: sample_environmental(),
        };
        let output = physically_possible(&input).unwrap();
        assert_eq!(output.result.possible_uses.len(), 3);
    }

    #[test]
    fn test_physical_irregular_lot() {
        let uses = vec![sample_potential_uses()[0].clone()];
        let mut site = sample_site();
        site.lot_shape = "irregular".into();
        let input = PhysicallyPossibleInput {
            legal_uses: uses,
            site,
            environmental: sample_environmental(),
        };
        let output = physically_possible(&input).unwrap();
        let office = &output.result.results[0];
        assert!(office.is_possible);
        // Irregular lot reduces by 15%
        assert_eq!(office.adjusted_buildable_sf, dec!(170000));
    }

    #[test]
    fn test_physical_steep_topography() {
        let uses = vec![sample_potential_uses()[0].clone()];
        let mut site = sample_site();
        site.topography = "steep".into();
        let input = PhysicallyPossibleInput {
            legal_uses: uses,
            site,
            environmental: sample_environmental(),
        };
        let output = physically_possible(&input).unwrap();
        let office = &output.result.results[0];
        // 200000 * 0.80 = 160000
        assert_eq!(office.adjusted_buildable_sf, dec!(160000));
    }

    #[test]
    fn test_physical_poor_soil() {
        let uses = vec![sample_potential_uses()[0].clone()];
        let mut site = sample_site();
        site.soil_conditions = "poor".into();
        let input = PhysicallyPossibleInput {
            legal_uses: uses,
            site,
            environmental: sample_environmental(),
        };
        let output = physically_possible(&input).unwrap();
        let office = &output.result.results[0];
        // 200000 * 0.85 = 170000
        assert_eq!(office.adjusted_buildable_sf, dec!(170000));
    }

    #[test]
    fn test_physical_wetlands_reduce_area() {
        let uses = vec![sample_potential_uses()[0].clone()];
        let env = EnvironmentalConstraints {
            wetlands: true,
            flood_zone: false,
            brownfield: false,
            remediation_cost: Decimal::ZERO,
        };
        let input = PhysicallyPossibleInput {
            legal_uses: uses,
            site: sample_site(),
            environmental: env,
        };
        let output = physically_possible(&input).unwrap();
        let office = &output.result.results[0];
        // 200000 * 0.75 = 150000
        assert_eq!(office.adjusted_buildable_sf, dec!(150000));
    }

    #[test]
    fn test_physical_combined_worst_case() {
        let mut small_use = sample_potential_uses()[0].clone();
        small_use.max_buildable_sf = dec!(10000);
        let uses = vec![small_use];
        let site = SiteCharacteristics {
            lot_size_sf: dec!(5000),
            lot_shape: "irregular".into(),
            topography: "steep".into(),
            soil_conditions: "poor".into(),
            utilities_available: false,
            access_quality: "poor".into(),
        };
        let input = PhysicallyPossibleInput {
            legal_uses: uses,
            site,
            environmental: sample_environmental(),
        };
        let output = physically_possible(&input).unwrap();
        let office = &output.result.results[0];
        // steep + poor soil + poor access = hard fail
        assert!(!office.is_possible);
    }

    #[test]
    fn test_physical_below_minimum_viable() {
        let mut tiny_use = sample_potential_uses()[0].clone();
        tiny_use.max_buildable_sf = dec!(3000); // below 5000 after any factor
        let uses = vec![tiny_use];
        let mut site = sample_site();
        site.lot_shape = "irregular".into(); // 3000 * 0.85 = 2550 < 5000
        let input = PhysicallyPossibleInput {
            legal_uses: uses,
            site,
            environmental: sample_environmental(),
        };
        let output = physically_possible(&input).unwrap();
        assert!(!output.result.results[0].is_possible);
    }

    #[test]
    fn test_physical_zero_lot_size_errors() {
        let uses = vec![sample_potential_uses()[0].clone()];
        let mut site = sample_site();
        site.lot_size_sf = Decimal::ZERO;
        let input = PhysicallyPossibleInput {
            legal_uses: uses,
            site,
            environmental: sample_environmental(),
        };
        assert!(physically_possible(&input).is_err());
    }

    // -----------------------------------------------------------------------
    // Financial Feasibility Tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_financial_positive_residual() {
        // Office: NOI = 35 * 200000 = 7,000,000; Value = 7M/0.06 = 116,666,666.67
        // Cost = 350 * 200000 = 70,000,000; Residual = 46,666,666.67
        let uses = vec![sample_potential_uses()[0].clone()]; // Office
        let input = FinanciallyFeasibleInput {
            physical_uses: uses,
            site: sample_site(),
        };
        let output = financially_feasible(&input).unwrap();
        let r = &output.result.results[0];
        assert!(r.is_feasible);
        assert!(r.residual_land_value > Decimal::ZERO);
        assert_eq!(r.development_cost, dec!(70000000.00));
    }

    #[test]
    fn test_financial_negative_residual() {
        let bad_use = PotentialUse {
            use_type: "BadDeal".into(),
            max_buildable_sf: dec!(100000),
            estimated_noi_psf: dec!(5), // very low NOI
            estimated_cap_rate: dec!(0.08),
            development_cost_psf: dec!(500), // very high cost
            construction_months: 36,
        };
        let input = FinanciallyFeasibleInput {
            physical_uses: vec![bad_use],
            site: sample_site(),
        };
        let output = financially_feasible(&input).unwrap();
        assert!(!output.result.results[0].is_feasible);
        assert!(output.result.results[0].residual_land_value < Decimal::ZERO);
    }

    #[test]
    fn test_financial_cap_rate_zero_errors() {
        let mut bad_use = sample_potential_uses()[0].clone();
        bad_use.estimated_cap_rate = Decimal::ZERO;
        let input = FinanciallyFeasibleInput {
            physical_uses: vec![bad_use],
            site: sample_site(),
        };
        assert!(financially_feasible(&input).is_err());
    }

    #[test]
    fn test_financial_cap_rate_over_one_errors() {
        let mut bad_use = sample_potential_uses()[0].clone();
        bad_use.estimated_cap_rate = dec!(1.5);
        let input = FinanciallyFeasibleInput {
            physical_uses: vec![bad_use],
            site: sample_site(),
        };
        assert!(financially_feasible(&input).is_err());
    }

    #[test]
    fn test_financial_residual_land_value_formula() {
        // Retail: NOI = 40 * 100000 = 4,000,000
        // Value = 4M / 0.065 = 61,538,461.54
        // Cost = 300 * 100000 = 30,000,000
        // Residual = 31,538,461.54
        let uses = vec![sample_potential_uses()[1].clone()]; // Retail
        let input = FinanciallyFeasibleInput {
            physical_uses: uses,
            site: sample_site(),
        };
        let output = financially_feasible(&input).unwrap();
        let r = &output.result.results[0];
        assert!(r.is_feasible);
        // NOI
        assert_eq!(r.stabilised_noi, dec!(4000000.00));
        // Cost
        assert_eq!(r.development_cost, dec!(30000000.00));
        // Capitalised value ~ 61,538,461.54
        let expected_cap_val = (dec!(4000000) / dec!(0.065)).round_dp(2);
        assert_eq!(r.capitalised_value, expected_cap_val);
    }

    #[test]
    fn test_financial_multiple_uses() {
        let uses = vec![
            sample_potential_uses()[0].clone(), // Office
            sample_potential_uses()[1].clone(), // Retail
            sample_potential_uses()[2].clone(), // Multifamily
        ];
        let input = FinanciallyFeasibleInput {
            physical_uses: uses,
            site: sample_site(),
        };
        let output = financially_feasible(&input).unwrap();
        // All three should be feasible with sample data
        assert_eq!(output.result.feasible_uses.len(), 3);
    }

    // -----------------------------------------------------------------------
    // Maximally Productive Tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_maximal_ranking() {
        let feasible = vec![
            FinancialUseResult {
                use_type: "Office".into(),
                is_feasible: true,
                development_cost: dec!(70000000),
                stabilised_noi: dec!(7000000),
                capitalised_value: dec!(116666667),
                residual_land_value: dec!(46666667),
            },
            FinancialUseResult {
                use_type: "Retail".into(),
                is_feasible: true,
                development_cost: dec!(30000000),
                stabilised_noi: dec!(4000000),
                capitalised_value: dec!(61538462),
                residual_land_value: dec!(31538462),
            },
        ];
        let potential = vec![
            sample_potential_uses()[0].clone(),
            sample_potential_uses()[1].clone(),
        ];
        let input = MaximallyProductiveInput {
            feasible_uses: feasible,
            potential_uses: potential,
            lot_size_sf: dec!(50000),
            discount_rate: dec!(0.08),
        };
        let output = maximally_productive(&input).unwrap();
        let r = &output.result;
        // Office: 46666667/50000 = 933.33; Retail: 31538462/50000 = 630.77
        assert_eq!(r.highest_and_best_use, "Office");
        assert_eq!(r.ranked_uses[0].rank, 1);
        assert_eq!(r.ranked_uses[1].rank, 2);
        assert!(r.ranked_uses[0].residual_per_sf > r.ranked_uses[1].residual_per_sf);
    }

    #[test]
    fn test_maximal_irr_positive() {
        let feasible = vec![FinancialUseResult {
            use_type: "Office".into(),
            is_feasible: true,
            development_cost: dec!(70000000),
            stabilised_noi: dec!(7000000),
            capitalised_value: dec!(116666667),
            residual_land_value: dec!(46666667),
        }];
        let potential = vec![sample_potential_uses()[0].clone()];
        let input = MaximallyProductiveInput {
            feasible_uses: feasible,
            potential_uses: potential,
            lot_size_sf: dec!(50000),
            discount_rate: dec!(0.08),
        };
        let output = maximally_productive(&input).unwrap();
        assert!(output.result.ranked_uses[0].irr > Decimal::ZERO);
    }

    #[test]
    fn test_maximal_equity_multiple() {
        let feasible = vec![FinancialUseResult {
            use_type: "Office".into(),
            is_feasible: true,
            development_cost: dec!(70000000),
            stabilised_noi: dec!(7000000),
            capitalised_value: dec!(116666667),
            residual_land_value: dec!(46666667),
        }];
        let potential = vec![sample_potential_uses()[0].clone()];
        let input = MaximallyProductiveInput {
            feasible_uses: feasible,
            potential_uses: potential,
            lot_size_sf: dec!(50000),
            discount_rate: dec!(0.08),
        };
        let output = maximally_productive(&input).unwrap();
        // Equity multiple > 1.0 for a feasible project
        assert!(output.result.ranked_uses[0].equity_multiple > Decimal::ONE);
    }

    #[test]
    fn test_maximal_empty_input_errors() {
        let input = MaximallyProductiveInput {
            feasible_uses: vec![],
            potential_uses: vec![],
            lot_size_sf: dec!(50000),
            discount_rate: dec!(0.08),
        };
        assert!(maximally_productive(&input).is_err());
    }

    #[test]
    fn test_maximal_zero_lot_size_errors() {
        let feasible = vec![FinancialUseResult {
            use_type: "Office".into(),
            is_feasible: true,
            development_cost: dec!(70000000),
            stabilised_noi: dec!(7000000),
            capitalised_value: dec!(116666667),
            residual_land_value: dec!(46666667),
        }];
        let input = MaximallyProductiveInput {
            feasible_uses: feasible,
            potential_uses: vec![sample_potential_uses()[0].clone()],
            lot_size_sf: Decimal::ZERO,
            discount_rate: dec!(0.08),
        };
        assert!(maximally_productive(&input).is_err());
    }

    // -----------------------------------------------------------------------
    // Full HBU Orchestrator Tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_hbu_full_analysis() {
        let input = HbuAnalysisInput {
            site_name: "123 Main St".into(),
            potential_uses: sample_potential_uses(),
            zoning: sample_zoning(),
            deed_restrictions: sample_deed(),
            environmental: sample_environmental(),
            site: sample_site(),
            discount_rate: dec!(0.08),
        };
        let output = hbu_analysis(&input).unwrap();
        let r = &output.result;
        assert!(r.recommended_use.is_some());
        assert!(matches!(r.legal_test, HbuTestStatus::Passed));
        assert!(matches!(r.physical_test, HbuTestStatus::Passed));
        assert!(matches!(r.financial_test, HbuTestStatus::Passed));
        assert!(matches!(r.maximal_test, HbuTestStatus::Passed));
    }

    #[test]
    fn test_hbu_short_circuit_legal() {
        let zoning = ZoningConstraints {
            use_class: "P-1".into(),
            permitted_uses: vec!["Park".into()],
            max_far: dec!(0.1),
            max_height_ft: dec!(15),
            min_setback_ft: dec!(50),
            max_lot_coverage_pct: dec!(0.10),
        };
        let input = HbuAnalysisInput {
            site_name: "Restricted Site".into(),
            potential_uses: sample_potential_uses(),
            zoning,
            deed_restrictions: sample_deed(),
            environmental: sample_environmental(),
            site: sample_site(),
            discount_rate: dec!(0.08),
        };
        let output = hbu_analysis(&input).unwrap();
        let r = &output.result;
        assert!(matches!(r.legal_test, HbuTestStatus::Failed { .. }));
        assert!(matches!(r.physical_test, HbuTestStatus::NotEvaluated));
        assert!(matches!(r.financial_test, HbuTestStatus::NotEvaluated));
        assert!(matches!(r.maximal_test, HbuTestStatus::NotEvaluated));
        assert!(r.recommended_use.is_none());
    }

    #[test]
    fn test_hbu_short_circuit_physical() {
        // Use very small buildable SF so physical test fails
        let tiny_uses = vec![PotentialUse {
            use_type: "Office".into(),
            max_buildable_sf: dec!(100), // will be < 5000 after any factor
            estimated_noi_psf: dec!(35),
            estimated_cap_rate: dec!(0.06),
            development_cost_psf: dec!(350),
            construction_months: 24,
        }];
        let mut site = sample_site();
        site.lot_shape = "irregular".into(); // 100 * 0.85 = 85 < 5000

        let input = HbuAnalysisInput {
            site_name: "Tiny Site".into(),
            potential_uses: tiny_uses,
            zoning: ZoningConstraints {
                use_class: "C-3".into(),
                permitted_uses: vec!["Office".into()],
                max_far: dec!(4.0),
                max_height_ft: dec!(120),
                min_setback_ft: dec!(10),
                max_lot_coverage_pct: dec!(0.80),
            },
            deed_restrictions: sample_deed(),
            environmental: sample_environmental(),
            site,
            discount_rate: dec!(0.08),
        };
        let output = hbu_analysis(&input).unwrap();
        let r = &output.result;
        assert!(matches!(r.legal_test, HbuTestStatus::Passed));
        assert!(matches!(r.physical_test, HbuTestStatus::Failed { .. }));
        assert!(matches!(r.financial_test, HbuTestStatus::NotEvaluated));
    }

    #[test]
    fn test_hbu_short_circuit_financial() {
        // Use low NOI and high cost so financial test fails
        let bad_uses = vec![PotentialUse {
            use_type: "Office".into(),
            max_buildable_sf: dec!(100000),
            estimated_noi_psf: dec!(2), // very low income
            estimated_cap_rate: dec!(0.08),
            development_cost_psf: dec!(600), // very high cost
            construction_months: 24,
        }];
        let input = HbuAnalysisInput {
            site_name: "Bad Economics Site".into(),
            potential_uses: bad_uses,
            zoning: ZoningConstraints {
                use_class: "C-3".into(),
                permitted_uses: vec!["Office".into()],
                max_far: dec!(4.0),
                max_height_ft: dec!(120),
                min_setback_ft: dec!(10),
                max_lot_coverage_pct: dec!(0.80),
            },
            deed_restrictions: sample_deed(),
            environmental: sample_environmental(),
            site: sample_site(),
            discount_rate: dec!(0.08),
        };
        let output = hbu_analysis(&input).unwrap();
        let r = &output.result;
        assert!(matches!(r.legal_test, HbuTestStatus::Passed));
        assert!(matches!(r.physical_test, HbuTestStatus::Passed));
        assert!(matches!(r.financial_test, HbuTestStatus::Failed { .. }));
        assert!(matches!(r.maximal_test, HbuTestStatus::NotEvaluated));
    }

    #[test]
    fn test_hbu_empty_potential_uses_errors() {
        let input = HbuAnalysisInput {
            site_name: "Empty".into(),
            potential_uses: vec![],
            zoning: sample_zoning(),
            deed_restrictions: sample_deed(),
            environmental: sample_environmental(),
            site: sample_site(),
            discount_rate: dec!(0.08),
        };
        assert!(hbu_analysis(&input).is_err());
    }

    #[test]
    fn test_hbu_cap_rate_validation() {
        let bad_uses = vec![PotentialUse {
            use_type: "Office".into(),
            max_buildable_sf: dec!(200000),
            estimated_noi_psf: dec!(35),
            estimated_cap_rate: dec!(-0.05), // negative cap rate
            development_cost_psf: dec!(350),
            construction_months: 24,
        }];
        let input = HbuAnalysisInput {
            site_name: "Bad Cap Rate".into(),
            potential_uses: bad_uses,
            zoning: ZoningConstraints {
                use_class: "C-3".into(),
                permitted_uses: vec!["Office".into()],
                max_far: dec!(4.0),
                max_height_ft: dec!(120),
                min_setback_ft: dec!(10),
                max_lot_coverage_pct: dec!(0.80),
            },
            deed_restrictions: sample_deed(),
            environmental: sample_environmental(),
            site: sample_site(),
            discount_rate: dec!(0.08),
        };
        assert!(hbu_analysis(&input).is_err());
    }

    // -----------------------------------------------------------------------
    // Newton-Raphson IRR Tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_irr_simple_investment() {
        // -100, +110 => IRR = 10%
        let cash_flows = vec![dec!(-100), dec!(110)];
        let mut warnings = Vec::new();
        let irr = newton_raphson_irr(&cash_flows, &mut warnings);
        assert!((irr - dec!(0.10)).abs() < dec!(0.001));
    }

    #[test]
    fn test_irr_multi_period() {
        // -1000, +400, +400, +400 => IRR ~ 9.7%
        let cash_flows = vec![dec!(-1000), dec!(400), dec!(400), dec!(400)];
        let mut warnings = Vec::new();
        let irr = newton_raphson_irr(&cash_flows, &mut warnings);
        assert!(irr > dec!(0.05) && irr < dec!(0.15));
    }

    #[test]
    fn test_irr_zero_npv_at_rate() {
        // Verify NPV is approximately zero at computed IRR
        let cash_flows = vec![dec!(-1000), dec!(300), dec!(400), dec!(500)];
        let mut warnings = Vec::new();
        let irr = newton_raphson_irr(&cash_flows, &mut warnings);
        let (npv, _) = npv_and_derivative(&cash_flows, irr);
        assert!(npv.abs() < dec!(0.01));
    }

    // -----------------------------------------------------------------------
    // Validation Tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_validate_cap_rate_positive() {
        assert!(validate_cap_rate(dec!(0.05), "Test").is_ok());
        assert!(validate_cap_rate(dec!(0.99), "Test").is_ok());
    }

    #[test]
    fn test_validate_cap_rate_zero() {
        assert!(validate_cap_rate(Decimal::ZERO, "Test").is_err());
    }

    #[test]
    fn test_validate_cap_rate_negative() {
        assert!(validate_cap_rate(dec!(-0.05), "Test").is_err());
    }

    #[test]
    fn test_validate_cap_rate_one_or_above() {
        assert!(validate_cap_rate(Decimal::ONE, "Test").is_err());
        assert!(validate_cap_rate(dec!(1.5), "Test").is_err());
    }
}
