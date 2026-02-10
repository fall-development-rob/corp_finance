use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{with_metadata, ComputationOutput, Money, Rate};
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Types — Carbon Footprint
// ---------------------------------------------------------------------------

/// GHG Protocol Scope 3 category breakdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scope3Category {
    /// Category number 1-15 per GHG Protocol
    pub category: u32,
    /// Human-readable name, e.g. "Purchased Goods", "Business Travel"
    pub name: String,
    /// Emissions in tCO2e
    pub emissions: Decimal,
}

/// Input for carbon footprint analysis (Scope 1/2/3), carbon pricing, and
/// target gap analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarbonFootprintInput {
    pub company_name: String,
    /// Annual revenue (for intensity calculations)
    pub revenue: Money,
    /// Scope 1 direct emissions (tCO2e)
    pub scope1_emissions: Decimal,
    /// Scope 2 location-based emissions from purchased electricity (tCO2e)
    pub scope2_emissions: Decimal,
    /// Optional market-based Scope 2 (accounting for renewable energy credits)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope2_market_based: Option<Decimal>,
    /// Scope 3 breakdown by GHG Protocol category
    pub scope3_categories: Vec<Scope3Category>,
    /// Carbon price in $/tCO2e (e.g. 50.0 for carbon tax / ETS price)
    pub carbon_price: Decimal,
    /// Target reduction as a decimal (e.g. 0.42 for 42% by 2030)
    pub reduction_target_pct: Decimal,
    /// Total emissions in the baseline year
    pub baseline_year_emissions: Decimal,
    /// Target year (e.g. 2030)
    pub target_year: u32,
}

/// Gap analysis comparing current emissions against a reduction target.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetGapAnalysis {
    pub baseline_emissions: Decimal,
    pub current_emissions: Decimal,
    /// baseline * (1 - reduction_target)
    pub target_emissions: Decimal,
    /// current - target (positive means still need to reduce)
    pub required_reduction: Decimal,
    pub on_track: bool,
    /// Required reduction per year to hit the target
    pub annual_reduction_needed: Decimal,
    /// Percentage of total reduction already achieved (0-100)
    pub pct_achieved: Decimal,
}

/// Output of carbon footprint analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarbonFootprintOutput {
    pub total_scope1_2: Decimal,
    pub total_scope1_2_3: Decimal,
    /// tCO2e per $M revenue
    pub carbon_intensity_revenue: Decimal,
    /// Difference between location-based and market-based Scope 2
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope2_method_delta: Option<Decimal>,
    pub scope3_total: Decimal,
    pub scope3_largest_category: String,
    /// Scope 3 as percentage of total (0-100)
    pub scope3_as_pct_of_total: Decimal,
    /// Total emissions * carbon_price
    pub carbon_cost_annual: Money,
    /// carbon_cost / revenue
    pub carbon_cost_intensity: Decimal,
    pub target_gap_analysis: TargetGapAnalysis,
    /// Implied temperature alignment (1.5 to 4.0+)
    pub temperature_alignment: Decimal,
}

// ---------------------------------------------------------------------------
// Types — Green Bond
// ---------------------------------------------------------------------------

/// Framework under which the green bond is issued.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GreenBondFramework {
    Icma,
    Cbi,
    EuTaxonomy,
}

/// A project funded by green bond proceeds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GreenProject {
    pub project_name: String,
    /// Amount allocated from bond proceeds
    pub allocation: Money,
    /// Category: "Renewable Energy", "Energy Efficiency", "Clean Transport",
    /// "Green Buildings", "Water", "Waste", "Biodiversity"
    pub category: String,
    /// Expected annual CO2 avoided (tCO2e)
    pub expected_co2_avoided: Decimal,
}

/// Input for green bond premium (greenium) analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GreenBondInput {
    pub bond_name: String,
    pub face_value: Money,
    pub coupon_rate: Rate,
    pub maturity_years: Decimal,
    /// Yield of the green bond
    pub green_bond_yield: Rate,
    /// Yield of a comparable conventional bond
    pub conventional_yield: Rate,
    /// Projects funded by proceeds
    pub use_of_proceeds: Vec<GreenProject>,
    /// Applicable green bond framework
    pub framework: GreenBondFramework,
}

/// Allocation breakdown by project category.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryAllocation {
    pub category: String,
    pub amount: Money,
    pub pct_of_total: Decimal,
}

/// Output of green bond analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GreenBondOutput {
    /// Greenium in basis points: (conventional - green) * 10000
    pub greenium_bps: Decimal,
    /// Present value of lower coupon savings (simplified)
    pub greenium_cost: Money,
    /// Total lifetime CO2 impact: sum(expected_co2_avoided) * maturity
    pub total_co2_impact: Decimal,
    /// greenium_cost / total_co2_impact
    pub cost_per_tonne_avoided: Decimal,
    pub allocation_by_category: Vec<CategoryAllocation>,
    /// Percentage of proceeds in eligible green categories (0-100)
    pub alignment_score: Decimal,
}

// ---------------------------------------------------------------------------
// Types — Sustainability-Linked Loan (SLL)
// ---------------------------------------------------------------------------

/// Whether lower or higher values are better for this KPI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TargetDirection {
    Lower,
    Higher,
}

/// A single sustainability performance target (SPT).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SustainabilityTarget {
    pub kpi_name: String,
    pub baseline_value: Decimal,
    pub target_value: Decimal,
    pub current_value: Decimal,
    /// Basis-point margin reduction if target is met (e.g. 10 = 10bps)
    pub margin_adjustment_bps: Decimal,
    pub direction: TargetDirection,
}

/// Input for SLL covenant testing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SllInput {
    pub loan_name: String,
    pub facility_amount: Money,
    /// Base margin in basis points (e.g. 200 = 200bps)
    pub base_margin_bps: Decimal,
    /// Sustainability performance targets
    pub spts: Vec<SustainabilityTarget>,
}

/// Result for a single SPT.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetResult {
    pub kpi_name: String,
    pub baseline: Decimal,
    pub target: Decimal,
    pub current: Decimal,
    /// Progress toward target (0-100)
    pub progress_pct: Decimal,
    pub met: bool,
    /// Margin impact in bps (negative = reduction)
    pub margin_impact_bps: Decimal,
}

/// Output of SLL covenant testing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SllOutput {
    pub base_margin_bps: Decimal,
    pub adjusted_margin_bps: Decimal,
    pub total_adjustment_bps: Decimal,
    /// facility_amount * total_adjustment / 10000
    pub annual_savings: Money,
    pub target_results: Vec<TargetResult>,
}

// ---------------------------------------------------------------------------
// Eligible green bond categories for alignment scoring
// ---------------------------------------------------------------------------

const ELIGIBLE_CATEGORIES: &[&str] = &[
    "Renewable Energy",
    "Energy Efficiency",
    "Clean Transport",
    "Green Buildings",
    "Water",
    "Waste",
    "Biodiversity",
];

// ---------------------------------------------------------------------------
// Public API — Carbon Footprint
// ---------------------------------------------------------------------------

/// Analyse an organisation's carbon footprint across Scope 1, 2, and 3,
/// compute carbon cost exposure, target gap analysis, and implied
/// temperature alignment.
pub fn analyze_carbon_footprint(
    input: &CarbonFootprintInput,
) -> CorpFinanceResult<ComputationOutput<CarbonFootprintOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // -- Validation --
    validate_carbon_input(input)?;

    if input.scope3_categories.is_empty() {
        warnings.push(
            "No Scope 3 data provided; total footprint will understate actual emissions.".into(),
        );
    }

    // -- Scope totals --
    let scope3_total: Decimal = input.scope3_categories.iter().map(|c| c.emissions).sum();

    let total_scope1_2 = input.scope1_emissions + input.scope2_emissions;
    let total_scope1_2_3 = total_scope1_2 + scope3_total;

    // -- Scope 2 market-based delta --
    let scope2_method_delta = input
        .scope2_market_based
        .map(|mb| input.scope2_emissions - mb);

    // -- Scope 3 largest category --
    let scope3_largest_category = input
        .scope3_categories
        .iter()
        .max_by(|a, b| a.emissions.cmp(&b.emissions))
        .map(|c| c.name.clone())
        .unwrap_or_else(|| "N/A".to_string());

    // -- Scope 3 as % of total --
    let scope3_as_pct_of_total = if total_scope1_2_3 > Decimal::ZERO {
        (scope3_total / total_scope1_2_3) * dec!(100)
    } else {
        Decimal::ZERO
    };

    // -- Carbon intensity: tCO2e per $M revenue --
    let revenue_in_millions = input.revenue / dec!(1_000_000);
    let carbon_intensity_revenue = if revenue_in_millions > Decimal::ZERO {
        total_scope1_2_3 / revenue_in_millions
    } else {
        warnings.push("Revenue is zero; carbon intensity cannot be calculated.".into());
        Decimal::ZERO
    };

    // -- Carbon cost --
    let carbon_cost_annual = total_scope1_2_3 * input.carbon_price;
    let carbon_cost_intensity = if input.revenue > Decimal::ZERO {
        carbon_cost_annual / input.revenue
    } else {
        Decimal::ZERO
    };

    // -- Target gap analysis --
    let target_emissions =
        input.baseline_year_emissions * (Decimal::ONE - input.reduction_target_pct);
    let current_emissions = total_scope1_2_3;
    let required_reduction = current_emissions - target_emissions;
    let on_track = current_emissions <= target_emissions;

    // Total reduction needed from baseline
    let total_reduction_from_baseline = input.baseline_year_emissions - target_emissions;

    // Reduction achieved so far
    let reduction_achieved = input.baseline_year_emissions - current_emissions;

    let pct_achieved = if total_reduction_from_baseline > Decimal::ZERO {
        (reduction_achieved / total_reduction_from_baseline * dec!(100)).max(Decimal::ZERO)
    } else if on_track {
        dec!(100)
    } else {
        Decimal::ZERO
    };

    // Years remaining to target (simplified: current year assumed from context)
    // Use a default of target_year - 2024 if positive, else 1
    let years_remaining = if input.target_year > 2024 {
        Decimal::from(input.target_year - 2024)
    } else {
        Decimal::ONE
    };

    let annual_reduction_needed = if required_reduction > Decimal::ZERO {
        required_reduction / years_remaining
    } else {
        Decimal::ZERO
    };

    let target_gap_analysis = TargetGapAnalysis {
        baseline_emissions: input.baseline_year_emissions,
        current_emissions,
        target_emissions,
        required_reduction: required_reduction.max(Decimal::ZERO),
        on_track,
        annual_reduction_needed,
        pct_achieved,
    };

    // -- Temperature alignment (simplified intensity model) --
    let temperature_alignment = implied_temperature(carbon_intensity_revenue);

    let output = CarbonFootprintOutput {
        total_scope1_2,
        total_scope1_2_3,
        carbon_intensity_revenue,
        scope2_method_delta,
        scope3_total,
        scope3_largest_category,
        scope3_as_pct_of_total,
        carbon_cost_annual,
        carbon_cost_intensity,
        target_gap_analysis,
        temperature_alignment,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    let assumptions = serde_json::json!({
        "ghg_protocol": "Corporate Standard + Scope 3",
        "scope2_method": "location-based (market-based optional)",
        "temperature_model": "simplified carbon-intensity proxy",
        "carbon_price_unit": "$/tCO2e",
        "intensity_unit": "tCO2e per $M revenue",
        "current_year_assumed": 2024
    });

    Ok(with_metadata(
        "Carbon Footprint Analysis (GHG Protocol / TCFD methodology)",
        &assumptions,
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Public API — Green Bond
// ---------------------------------------------------------------------------

/// Analyse a green bond's greenium, carbon impact, allocation, and alignment.
pub fn analyze_green_bond(
    input: &GreenBondInput,
) -> CorpFinanceResult<ComputationOutput<GreenBondOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // -- Validation --
    validate_green_bond_input(input)?;

    // -- Greenium --
    let greenium_bps = (input.conventional_yield - input.green_bond_yield) * dec!(10_000);

    if greenium_bps < Decimal::ZERO {
        warnings.push(
            "Negative greenium: the green bond yields more than the conventional bond.".into(),
        );
    }

    // Simplified PV of coupon savings over the bond's life
    // greenium_bps / 10000 * face_value * maturity_years
    let greenium_cost = (greenium_bps / dec!(10_000)) * input.face_value * input.maturity_years;

    // -- CO2 impact --
    let annual_co2_avoided: Decimal = input
        .use_of_proceeds
        .iter()
        .map(|p| p.expected_co2_avoided)
        .sum();
    let total_co2_impact = annual_co2_avoided * input.maturity_years;

    let cost_per_tonne_avoided = if total_co2_impact > Decimal::ZERO {
        greenium_cost / total_co2_impact
    } else {
        warnings.push("Total CO2 impact is zero; cost per tonne cannot be calculated.".into());
        Decimal::ZERO
    };

    // -- Allocation by category --
    let total_proceeds: Decimal = input.use_of_proceeds.iter().map(|p| p.allocation).sum();

    let mut category_map: Vec<(String, Decimal)> = Vec::new();
    for project in &input.use_of_proceeds {
        if let Some(entry) = category_map
            .iter_mut()
            .find(|(c, _)| *c == project.category)
        {
            entry.1 += project.allocation;
        } else {
            category_map.push((project.category.clone(), project.allocation));
        }
    }

    let allocation_by_category: Vec<CategoryAllocation> = category_map
        .into_iter()
        .map(|(category, amount)| {
            let pct_of_total = if total_proceeds > Decimal::ZERO {
                (amount / total_proceeds) * dec!(100)
            } else {
                Decimal::ZERO
            };
            CategoryAllocation {
                category,
                amount,
                pct_of_total,
            }
        })
        .collect();

    // -- Alignment score: % of proceeds in eligible categories --
    let eligible_amount: Decimal = input
        .use_of_proceeds
        .iter()
        .filter(|p| {
            ELIGIBLE_CATEGORIES
                .iter()
                .any(|e| e.eq_ignore_ascii_case(&p.category))
        })
        .map(|p| p.allocation)
        .sum();

    let alignment_score = if total_proceeds > Decimal::ZERO {
        (eligible_amount / total_proceeds) * dec!(100)
    } else {
        Decimal::ZERO
    };

    let output = GreenBondOutput {
        greenium_bps,
        greenium_cost,
        total_co2_impact,
        cost_per_tonne_avoided,
        allocation_by_category,
        alignment_score,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    let assumptions = serde_json::json!({
        "framework": format!("{:?}", input.framework),
        "greenium_formula": "(conventional_yield - green_yield) * 10000 bps",
        "pv_savings": "simplified: greenium_rate * face_value * maturity",
        "eligible_categories": ELIGIBLE_CATEGORIES,
    });

    Ok(with_metadata(
        "Green Bond Analysis (ICMA Green Bond Principles)",
        &assumptions,
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Public API — Sustainability-Linked Loan
// ---------------------------------------------------------------------------

/// Test sustainability performance targets for a sustainability-linked loan
/// and compute the adjusted margin.
pub fn test_sll_covenants(input: &SllInput) -> CorpFinanceResult<ComputationOutput<SllOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // -- Validation --
    validate_sll_input(input)?;

    if input.spts.is_empty() {
        warnings.push("No sustainability performance targets provided.".into());
    }

    // -- Evaluate each SPT --
    let mut total_adjustment_bps = Decimal::ZERO;
    let mut target_results: Vec<TargetResult> = Vec::with_capacity(input.spts.len());

    for spt in &input.spts {
        let (met, progress_pct) = evaluate_spt(spt);

        let margin_impact_bps = if met {
            -spt.margin_adjustment_bps
        } else {
            Decimal::ZERO
        };

        if met {
            total_adjustment_bps -= spt.margin_adjustment_bps;
        }

        target_results.push(TargetResult {
            kpi_name: spt.kpi_name.clone(),
            baseline: spt.baseline_value,
            target: spt.target_value,
            current: spt.current_value,
            progress_pct,
            met,
            margin_impact_bps,
        });
    }

    let adjusted_margin_bps = input.base_margin_bps + total_adjustment_bps;
    let annual_savings = input.facility_amount * (-total_adjustment_bps) / dec!(10_000);

    let output = SllOutput {
        base_margin_bps: input.base_margin_bps,
        adjusted_margin_bps,
        total_adjustment_bps,
        annual_savings,
        target_results,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    let assumptions = serde_json::json!({
        "methodology": "Sustainability-Linked Loan Principles (SLLP)",
        "margin_adjustment": "applied only when SPT is fully met",
        "progress_formula_lower": "(baseline - current) / (baseline - target) * 100",
        "progress_formula_higher": "(current - baseline) / (target - baseline) * 100",
    });

    Ok(with_metadata(
        "SLL Covenant Testing (Sustainability-Linked Loan Principles)",
        &assumptions,
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn validate_carbon_input(input: &CarbonFootprintInput) -> CorpFinanceResult<()> {
    if input.revenue <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "revenue".into(),
            reason: "Revenue must be positive.".into(),
        });
    }
    if input.scope1_emissions < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "scope1_emissions".into(),
            reason: "Scope 1 emissions cannot be negative.".into(),
        });
    }
    if input.scope2_emissions < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "scope2_emissions".into(),
            reason: "Scope 2 emissions cannot be negative.".into(),
        });
    }
    if input.carbon_price < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "carbon_price".into(),
            reason: "Carbon price cannot be negative.".into(),
        });
    }
    if input.reduction_target_pct < Decimal::ZERO || input.reduction_target_pct > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "reduction_target_pct".into(),
            reason: "Reduction target must be between 0 and 1.".into(),
        });
    }
    if input.baseline_year_emissions < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "baseline_year_emissions".into(),
            reason: "Baseline year emissions cannot be negative.".into(),
        });
    }
    for cat in &input.scope3_categories {
        if cat.emissions < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("scope3_categories[{}]", cat.category),
                reason: "Scope 3 category emissions cannot be negative.".into(),
            });
        }
        if cat.category == 0 || cat.category > 15 {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("scope3_categories[{}]", cat.category),
                reason: "Scope 3 category must be 1-15 per GHG Protocol.".into(),
            });
        }
    }
    Ok(())
}

fn validate_green_bond_input(input: &GreenBondInput) -> CorpFinanceResult<()> {
    if input.face_value <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "face_value".into(),
            reason: "Face value must be positive.".into(),
        });
    }
    if input.maturity_years <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "maturity_years".into(),
            reason: "Maturity must be positive.".into(),
        });
    }
    if input.use_of_proceeds.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one green project must be specified in use_of_proceeds.".into(),
        ));
    }
    for project in &input.use_of_proceeds {
        if project.allocation < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("use_of_proceeds[{}].allocation", project.project_name),
                reason: "Project allocation cannot be negative.".into(),
            });
        }
    }
    Ok(())
}

fn validate_sll_input(input: &SllInput) -> CorpFinanceResult<()> {
    if input.facility_amount <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "facility_amount".into(),
            reason: "Facility amount must be positive.".into(),
        });
    }
    if input.base_margin_bps < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "base_margin_bps".into(),
            reason: "Base margin cannot be negative.".into(),
        });
    }
    for spt in &input.spts {
        if spt.margin_adjustment_bps < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("spts[{}].margin_adjustment_bps", spt.kpi_name),
                reason: "Margin adjustment cannot be negative.".into(),
            });
        }
    }
    Ok(())
}

/// Evaluate a single sustainability performance target.
/// Returns (met, progress_pct).
fn evaluate_spt(spt: &SustainabilityTarget) -> (bool, Decimal) {
    match spt.direction {
        TargetDirection::Lower => {
            let met = spt.current_value <= spt.target_value;
            let denominator = spt.baseline_value - spt.target_value;
            let progress_pct = if denominator > Decimal::ZERO {
                let raw = (spt.baseline_value - spt.current_value) / denominator * dec!(100);
                raw.max(Decimal::ZERO).min(dec!(100))
            } else if met {
                dec!(100)
            } else {
                Decimal::ZERO
            };
            (met, progress_pct)
        }
        TargetDirection::Higher => {
            let met = spt.current_value >= spt.target_value;
            let denominator = spt.target_value - spt.baseline_value;
            let progress_pct = if denominator > Decimal::ZERO {
                let raw = (spt.current_value - spt.baseline_value) / denominator * dec!(100);
                raw.max(Decimal::ZERO).min(dec!(100))
            } else if met {
                dec!(100)
            } else {
                Decimal::ZERO
            };
            (met, progress_pct)
        }
    }
}

/// Simplified temperature alignment model based on carbon intensity
/// (tCO2e per $M revenue).
fn implied_temperature(intensity: Decimal) -> Decimal {
    if intensity < dec!(100) {
        dec!(1.5)
    } else if intensity < dec!(300) {
        dec!(2.0)
    } else if intensity < dec!(600) {
        dec!(2.5)
    } else if intensity < dec!(1000) {
        dec!(3.0)
    } else {
        dec!(4.0)
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

    fn sample_carbon_input() -> CarbonFootprintInput {
        CarbonFootprintInput {
            company_name: "TestCorp".into(),
            revenue: dec!(500_000_000),
            scope1_emissions: dec!(10_000),
            scope2_emissions: dec!(15_000),
            scope2_market_based: Some(dec!(8_000)),
            scope3_categories: vec![
                Scope3Category {
                    category: 1,
                    name: "Purchased Goods".into(),
                    emissions: dec!(50_000),
                },
                Scope3Category {
                    category: 6,
                    name: "Business Travel".into(),
                    emissions: dec!(5_000),
                },
                Scope3Category {
                    category: 11,
                    name: "Use of Sold Products".into(),
                    emissions: dec!(20_000),
                },
            ],
            carbon_price: dec!(50),
            reduction_target_pct: dec!(0.42),
            baseline_year_emissions: dec!(120_000),
            target_year: 2030,
        }
    }

    fn sample_green_bond_input() -> GreenBondInput {
        GreenBondInput {
            bond_name: "TestCorp Green Bond 2025".into(),
            face_value: dec!(500_000_000),
            coupon_rate: dec!(0.035),
            maturity_years: dec!(10),
            green_bond_yield: dec!(0.0340),
            conventional_yield: dec!(0.0345),
            use_of_proceeds: vec![
                GreenProject {
                    project_name: "Solar Farm".into(),
                    allocation: dec!(200_000_000),
                    category: "Renewable Energy".into(),
                    expected_co2_avoided: dec!(80_000),
                },
                GreenProject {
                    project_name: "Building Retrofit".into(),
                    allocation: dec!(150_000_000),
                    category: "Energy Efficiency".into(),
                    expected_co2_avoided: dec!(30_000),
                },
                GreenProject {
                    project_name: "EV Fleet".into(),
                    allocation: dec!(100_000_000),
                    category: "Clean Transport".into(),
                    expected_co2_avoided: dec!(15_000),
                },
                GreenProject {
                    project_name: "General Purpose".into(),
                    allocation: dec!(50_000_000),
                    category: "General Corporate".into(),
                    expected_co2_avoided: dec!(0),
                },
            ],
            framework: GreenBondFramework::Icma,
        }
    }

    fn sample_sll_input() -> SllInput {
        SllInput {
            loan_name: "TestCorp SLL 2025".into(),
            facility_amount: dec!(200_000_000),
            base_margin_bps: dec!(200),
            spts: vec![
                SustainabilityTarget {
                    kpi_name: "Scope 1+2 Emissions Intensity".into(),
                    baseline_value: dec!(100),
                    target_value: dec!(70),
                    current_value: dec!(65),
                    margin_adjustment_bps: dec!(10),
                    direction: TargetDirection::Lower,
                },
                SustainabilityTarget {
                    kpi_name: "Renewable Energy %".into(),
                    baseline_value: dec!(30),
                    target_value: dec!(60),
                    current_value: dec!(55),
                    margin_adjustment_bps: dec!(5),
                    direction: TargetDirection::Higher,
                },
                SustainabilityTarget {
                    kpi_name: "Water Intensity".into(),
                    baseline_value: dec!(50),
                    target_value: dec!(35),
                    current_value: dec!(40),
                    margin_adjustment_bps: dec!(5),
                    direction: TargetDirection::Lower,
                },
            ],
        }
    }

    // -----------------------------------------------------------------------
    // Carbon Footprint Tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_scope1_2_total() {
        let input = sample_carbon_input();
        let result = analyze_carbon_footprint(&input).unwrap();
        let out = &result.result;
        // Scope 1 (10000) + Scope 2 (15000) = 25000
        assert_eq!(out.total_scope1_2, dec!(25_000));
    }

    #[test]
    fn test_scope1_2_3_total() {
        let input = sample_carbon_input();
        let result = analyze_carbon_footprint(&input).unwrap();
        let out = &result.result;
        // 25000 + 50000 + 5000 + 20000 = 100000
        assert_eq!(out.total_scope1_2_3, dec!(100_000));
    }

    #[test]
    fn test_scope3_as_pct_of_total() {
        let input = sample_carbon_input();
        let result = analyze_carbon_footprint(&input).unwrap();
        let out = &result.result;
        // Scope 3 total = 75000, full total = 100000 => 75%
        assert_eq!(out.scope3_total, dec!(75_000));
        assert_eq!(out.scope3_as_pct_of_total, dec!(75));
    }

    #[test]
    fn test_scope3_largest_category() {
        let input = sample_carbon_input();
        let result = analyze_carbon_footprint(&input).unwrap();
        let out = &result.result;
        assert_eq!(out.scope3_largest_category, "Purchased Goods");
    }

    #[test]
    fn test_carbon_cost_calculation() {
        let input = sample_carbon_input();
        let result = analyze_carbon_footprint(&input).unwrap();
        let out = &result.result;
        // 100000 tCO2e * $50/tCO2e = $5,000,000
        assert_eq!(out.carbon_cost_annual, dec!(5_000_000));
    }

    #[test]
    fn test_carbon_cost_intensity() {
        let input = sample_carbon_input();
        let result = analyze_carbon_footprint(&input).unwrap();
        let out = &result.result;
        // $5,000,000 / $500,000,000 = 0.01
        assert_eq!(out.carbon_cost_intensity, dec!(0.01));
    }

    #[test]
    fn test_carbon_intensity_per_revenue() {
        let input = sample_carbon_input();
        let result = analyze_carbon_footprint(&input).unwrap();
        let out = &result.result;
        // 100000 tCO2e / ($500M / $1M) = 100000 / 500 = 200
        assert_eq!(out.carbon_intensity_revenue, dec!(200));
    }

    #[test]
    fn test_target_gap_off_track() {
        let input = sample_carbon_input();
        let result = analyze_carbon_footprint(&input).unwrap();
        let gap = &result.result.target_gap_analysis;
        // target = 120000 * (1 - 0.42) = 69600
        // current = 100000, so off track (100000 > 69600)
        assert_eq!(gap.target_emissions, dec!(69_600));
        assert!(!gap.on_track);
        // required_reduction = 100000 - 69600 = 30400
        assert_eq!(gap.required_reduction, dec!(30_400));
    }

    #[test]
    fn test_target_gap_on_track() {
        let mut input = sample_carbon_input();
        // Lower emissions to be below target
        input.scope3_categories = vec![Scope3Category {
            category: 1,
            name: "Purchased Goods".into(),
            emissions: dec!(10_000),
        }];
        // total = 10000 + 15000 + 10000 = 35000
        // target = 120000 * (1 - 0.42) = 69600
        // 35000 < 69600 => on track
        let result = analyze_carbon_footprint(&input).unwrap();
        let gap = &result.result.target_gap_analysis;
        assert!(gap.on_track);
        assert_eq!(gap.required_reduction, Decimal::ZERO);
    }

    #[test]
    fn test_target_pct_achieved() {
        let input = sample_carbon_input();
        let result = analyze_carbon_footprint(&input).unwrap();
        let gap = &result.result.target_gap_analysis;
        // total_reduction_from_baseline = 120000 - 69600 = 50400
        // reduction_achieved = 120000 - 100000 = 20000
        // pct_achieved = 20000 / 50400 * 100 = 39.68...
        let expected = dec!(20_000) / dec!(50_400) * dec!(100);
        assert_eq!(gap.pct_achieved, expected);
    }

    #[test]
    fn test_temperature_alignment_2c() {
        let input = sample_carbon_input();
        let result = analyze_carbon_footprint(&input).unwrap();
        let out = &result.result;
        // intensity = 200 => 100-300 range => 2.0C
        assert_eq!(out.temperature_alignment, dec!(2.0));
    }

    #[test]
    fn test_temperature_alignment_1_5c() {
        let mut input = sample_carbon_input();
        // Revenue very high => low intensity
        input.revenue = dec!(10_000_000_000); // $10B
                                              // intensity = 100000 / 10000 = 10 => <100 => 1.5C
        let result = analyze_carbon_footprint(&input).unwrap();
        assert_eq!(result.result.temperature_alignment, dec!(1.5));
    }

    #[test]
    fn test_temperature_alignment_4c() {
        let mut input = sample_carbon_input();
        // Very low revenue => high intensity
        input.revenue = dec!(50_000_000); // $50M
                                          // intensity = 100000 / 50 = 2000 => >1000 => 4.0C
        let result = analyze_carbon_footprint(&input).unwrap();
        assert_eq!(result.result.temperature_alignment, dec!(4.0));
    }

    #[test]
    fn test_scope2_market_based_delta() {
        let input = sample_carbon_input();
        let result = analyze_carbon_footprint(&input).unwrap();
        let out = &result.result;
        // delta = location (15000) - market (8000) = 7000
        assert_eq!(out.scope2_method_delta, Some(dec!(7_000)));
    }

    #[test]
    fn test_scope2_no_market_based() {
        let mut input = sample_carbon_input();
        input.scope2_market_based = None;
        let result = analyze_carbon_footprint(&input).unwrap();
        assert!(result.result.scope2_method_delta.is_none());
    }

    #[test]
    fn test_carbon_invalid_revenue() {
        let mut input = sample_carbon_input();
        input.revenue = dec!(-100);
        let err = analyze_carbon_footprint(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "revenue");
            }
            other => panic!("Expected InvalidInput for revenue, got: {other:?}"),
        }
    }

    #[test]
    fn test_carbon_invalid_scope3_category() {
        let mut input = sample_carbon_input();
        input.scope3_categories.push(Scope3Category {
            category: 16,
            name: "Invalid".into(),
            emissions: dec!(100),
        });
        let err = analyze_carbon_footprint(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert!(field.contains("scope3_categories"));
            }
            other => panic!("Expected InvalidInput, got: {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Green Bond Tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_greenium_calculation() {
        let input = sample_green_bond_input();
        let result = analyze_green_bond(&input).unwrap();
        let out = &result.result;
        // greenium = (0.0345 - 0.0340) * 10000 = 5 bps
        assert_eq!(out.greenium_bps, dec!(5));
    }

    #[test]
    fn test_greenium_cost() {
        let input = sample_green_bond_input();
        let result = analyze_green_bond(&input).unwrap();
        let out = &result.result;
        // cost = 5/10000 * 500_000_000 * 10 = 2,500,000
        assert_eq!(out.greenium_cost, dec!(2_500_000));
    }

    #[test]
    fn test_total_co2_impact() {
        let input = sample_green_bond_input();
        let result = analyze_green_bond(&input).unwrap();
        let out = &result.result;
        // annual = 80000 + 30000 + 15000 + 0 = 125000
        // total = 125000 * 10 = 1_250_000
        assert_eq!(out.total_co2_impact, dec!(1_250_000));
    }

    #[test]
    fn test_cost_per_tonne_avoided() {
        let input = sample_green_bond_input();
        let result = analyze_green_bond(&input).unwrap();
        let out = &result.result;
        // 2_500_000 / 1_250_000 = 2.0
        assert_eq!(out.cost_per_tonne_avoided, dec!(2));
    }

    #[test]
    fn test_category_allocation_percentages() {
        let input = sample_green_bond_input();
        let result = analyze_green_bond(&input).unwrap();
        let out = &result.result;
        // Total proceeds = 200M + 150M + 100M + 50M = 500M
        // Renewable: 200/500 = 40%
        let renewable = out
            .allocation_by_category
            .iter()
            .find(|c| c.category == "Renewable Energy")
            .unwrap();
        assert_eq!(renewable.pct_of_total, dec!(40));
        assert_eq!(renewable.amount, dec!(200_000_000));

        // Energy Efficiency: 150/500 = 30%
        let efficiency = out
            .allocation_by_category
            .iter()
            .find(|c| c.category == "Energy Efficiency")
            .unwrap();
        assert_eq!(efficiency.pct_of_total, dec!(30));
    }

    #[test]
    fn test_alignment_score() {
        let input = sample_green_bond_input();
        let result = analyze_green_bond(&input).unwrap();
        let out = &result.result;
        // Eligible: 200M + 150M + 100M = 450M out of 500M = 90%
        // "General Corporate" is not an eligible category
        assert_eq!(out.alignment_score, dec!(90));
    }

    #[test]
    fn test_green_bond_invalid_face_value() {
        let mut input = sample_green_bond_input();
        input.face_value = Decimal::ZERO;
        let err = analyze_green_bond(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "face_value");
            }
            other => panic!("Expected InvalidInput, got: {other:?}"),
        }
    }

    #[test]
    fn test_green_bond_no_projects() {
        let mut input = sample_green_bond_input();
        input.use_of_proceeds.clear();
        let err = analyze_green_bond(&input).unwrap_err();
        match err {
            CorpFinanceError::InsufficientData(_) => {}
            other => panic!("Expected InsufficientData, got: {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // SLL Tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_sll_all_targets_met() {
        // In sample, emissions target (65 <= 70) met, water (40 > 35) not met,
        // renewable (55 < 60) not met. Adjust so all are met.
        let mut input = sample_sll_input();
        // Make renewable target met
        input.spts[1].current_value = dec!(65);
        // Make water target met
        input.spts[2].current_value = dec!(30);

        let result = test_sll_covenants(&input).unwrap();
        let out = &result.result;

        // All targets met: total adjustment = -(10 + 5 + 5) = -20
        assert_eq!(out.total_adjustment_bps, dec!(-20));
        assert_eq!(out.adjusted_margin_bps, dec!(180));
        // annual savings = 200M * 20 / 10000 = 400,000
        assert_eq!(out.annual_savings, dec!(400_000));

        for tr in &out.target_results {
            assert!(tr.met, "Expected target '{}' to be met", tr.kpi_name);
        }
    }

    #[test]
    fn test_sll_partial_targets_met() {
        let input = sample_sll_input();
        let result = test_sll_covenants(&input).unwrap();
        let out = &result.result;

        // Emissions: 65 <= 70 => met (10bps)
        // Renewable: 55 < 60 => not met
        // Water: 40 > 35 => not met
        assert_eq!(out.total_adjustment_bps, dec!(-10));
        assert_eq!(out.adjusted_margin_bps, dec!(190));
        // savings = 200M * 10 / 10000 = 200,000
        assert_eq!(out.annual_savings, dec!(200_000));

        let emissions = out
            .target_results
            .iter()
            .find(|t| t.kpi_name == "Scope 1+2 Emissions Intensity")
            .unwrap();
        assert!(emissions.met);
        assert_eq!(emissions.margin_impact_bps, dec!(-10));

        let renewable = out
            .target_results
            .iter()
            .find(|t| t.kpi_name == "Renewable Energy %")
            .unwrap();
        assert!(!renewable.met);
        assert_eq!(renewable.margin_impact_bps, Decimal::ZERO);
    }

    #[test]
    fn test_sll_no_targets_met() {
        let mut input = sample_sll_input();
        // Emissions: make current worse than target
        input.spts[0].current_value = dec!(90);
        // Renewable: still below target
        input.spts[1].current_value = dec!(35);
        // Water: still above target
        input.spts[2].current_value = dec!(48);

        let result = test_sll_covenants(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.total_adjustment_bps, Decimal::ZERO);
        assert_eq!(out.adjusted_margin_bps, dec!(200));
        assert_eq!(out.annual_savings, Decimal::ZERO);

        for tr in &out.target_results {
            assert!(!tr.met, "Expected target '{}' to not be met", tr.kpi_name);
        }
    }

    #[test]
    fn test_sll_progress_lower_direction() {
        let input = sample_sll_input();
        let result = test_sll_covenants(&input).unwrap();
        let out = &result.result;

        // Emissions: baseline=100, target=70, current=65
        // progress = (100-65)/(100-70)*100 = 35/30*100 = 116.67 => capped at 100
        let emissions = out
            .target_results
            .iter()
            .find(|t| t.kpi_name == "Scope 1+2 Emissions Intensity")
            .unwrap();
        assert_eq!(emissions.progress_pct, dec!(100));

        // Water: baseline=50, target=35, current=40
        // progress = (50-40)/(50-35)*100 = 10/15*100 = 66.666...
        let water = out
            .target_results
            .iter()
            .find(|t| t.kpi_name == "Water Intensity")
            .unwrap();
        let expected_water = dec!(10) / dec!(15) * dec!(100);
        assert_eq!(water.progress_pct, expected_water);
    }

    #[test]
    fn test_sll_progress_higher_direction() {
        let input = sample_sll_input();
        let result = test_sll_covenants(&input).unwrap();
        let out = &result.result;

        // Renewable: baseline=30, target=60, current=55
        // progress = (55-30)/(60-30)*100 = 25/30*100 = 83.333...
        let renewable = out
            .target_results
            .iter()
            .find(|t| t.kpi_name == "Renewable Energy %")
            .unwrap();
        let expected = dec!(25) / dec!(30) * dec!(100);
        assert_eq!(renewable.progress_pct, expected);
    }

    #[test]
    fn test_sll_invalid_facility_amount() {
        let mut input = sample_sll_input();
        input.facility_amount = Decimal::ZERO;
        let err = test_sll_covenants(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "facility_amount");
            }
            other => panic!("Expected InvalidInput, got: {other:?}"),
        }
    }

    #[test]
    fn test_sll_empty_spts_warning() {
        let mut input = sample_sll_input();
        input.spts.clear();
        let result = test_sll_covenants(&input).unwrap();
        assert!(result
            .warnings
            .iter()
            .any(|w| w.contains("No sustainability")));
        assert_eq!(result.result.total_adjustment_bps, Decimal::ZERO);
    }

    // -----------------------------------------------------------------------
    // Metadata tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_carbon_metadata_populated() {
        let input = sample_carbon_input();
        let result = analyze_carbon_footprint(&input).unwrap();
        assert!(!result.methodology.is_empty());
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
    }

    #[test]
    fn test_green_bond_metadata_populated() {
        let input = sample_green_bond_input();
        let result = analyze_green_bond(&input).unwrap();
        assert!(!result.methodology.is_empty());
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
    }

    #[test]
    fn test_sll_metadata_populated() {
        let input = sample_sll_input();
        let result = test_sll_covenants(&input).unwrap();
        assert!(!result.methodology.is_empty());
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
    }
}
