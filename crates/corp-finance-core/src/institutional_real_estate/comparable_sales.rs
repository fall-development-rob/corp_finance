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

/// Area basis for price-per-SF normalisation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AreaBasis {
    /// Gross building area (exterior walls).
    GrossBuildingArea,
    /// Net rentable area (leasable space).
    NetRentableArea,
    /// Usable area (occupant space, excludes common areas).
    UsableArea,
}

/// Adjustment category following the Appraisal Institute sequence.
/// Transactional adjustments (PropertyRights, FinancingTerms, ConditionsOfSale,
/// MarketConditions) are applied first, then property adjustments (Location,
/// Condition, Size, Age, Amenities).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AdjustmentCategory {
    /// Rights of ownership conveyed.
    PropertyRights,
    /// Financing terms (seller financing, assumed mortgage).
    FinancingTerms,
    /// Conditions of sale (motivation, related parties).
    ConditionsOfSale,
    /// Market conditions / time adjustment.
    MarketConditions,
    /// Location / neighbourhood quality.
    Location,
    /// Physical condition of improvements.
    Condition,
    /// Building size differential.
    Size,
    /// Building age / effective age.
    Age,
    /// Amenities (parking, views, tenant improvements).
    Amenities,
}

impl AdjustmentCategory {
    /// Appraisal Institute ordering: transactional first, then property.
    fn sequence_order(&self) -> u8 {
        match self {
            Self::PropertyRights => 0,
            Self::FinancingTerms => 1,
            Self::ConditionsOfSale => 2,
            Self::MarketConditions => 3,
            Self::Location => 4,
            Self::Condition => 5,
            Self::Size => 6,
            Self::Age => 7,
            Self::Amenities => 8,
        }
    }
}

/// A single adjustment applied to a comparable sale.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Adjustment {
    /// Which category this adjustment falls under.
    pub category: AdjustmentCategory,
    /// Percentage adjustment as a decimal (e.g. 0.05 = +5%).
    /// Positive means subject property is superior; negative means inferior.
    pub pct_adjustment: Rate,
    /// Free-text explanation for the adjustment.
    pub narrative: String,
}

/// A comparable sale transaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comparable {
    /// Property address or identifier.
    pub address: String,
    /// Recorded sale price.
    pub sale_price: Money,
    /// Sale date as YYYY-MM-DD string.
    pub sale_date: String,
    /// Gross building area in square feet.
    pub gross_building_area_sf: Decimal,
    /// Net rentable area in square feet.
    pub net_rentable_area_sf: Option<Decimal>,
    /// Usable area in square feet.
    pub usable_area_sf: Option<Decimal>,
    /// Year the building was constructed.
    pub year_built: u32,
    /// Trailing-twelve-month or annualised NOI.
    pub noi: Option<Money>,
    /// Forward (projected) NOI for cap rate extraction.
    pub forward_noi: Option<Money>,
    /// Current physical occupancy rate (0-1).
    pub occupancy_pct: Option<Rate>,
    /// Property type descriptor.
    pub property_type: String,
    /// Condition rating from 1 (poor) to 5 (excellent).
    pub condition_rating: Option<u32>,
    /// Comparability quality score from 1 (weakest) to 5 (best).
    pub quality_score: Option<u32>,
    /// Distance to subject property in miles (for inverse-distance weighting).
    pub distance_to_subject: Option<Decimal>,
}

/// Reconciliation method selector.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReconciliationMethod {
    /// Simple arithmetic average.
    EqualWeight,
    /// Weight by comparability quality score (1-5).
    QualityScore,
    /// Weight by inverse distance to subject property.
    InverseDistance,
}

// ---------------------------------------------------------------------------
// Input structs
// ---------------------------------------------------------------------------

/// Input for the comparable adjustment grid.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompAdjustmentInput {
    /// Subject property address or identifier.
    pub subject_address: String,
    /// Comparable transactions.
    pub comparables: Vec<Comparable>,
    /// Adjustments per comparable, indexed by position (outer = comp, inner = adjustments).
    pub adjustments: Vec<Vec<Adjustment>>,
    /// Optional subject NOI for implied cap rate calculation.
    pub subject_noi: Option<Money>,
}

/// Input for price-per-SF normalisation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricePerSfInput {
    /// Comparable transactions.
    pub comparables: Vec<Comparable>,
    /// Which area basis to use.
    pub area_basis: AreaBasis,
}

/// Input for cap rate extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapRateExtractionInput {
    /// Comparable transactions (must have NOI populated).
    pub comparables: Vec<Comparable>,
    /// Stabilised vacancy rate to normalise occupancy (e.g. 0.05).
    pub stabilised_vacancy_rate: Rate,
    /// Market expense ratio for standardisation (operating expenses / EGI).
    pub market_expense_ratio: Rate,
    /// CapEx reserve as percentage of EGI.
    pub capex_reserve_pct: Rate,
}

/// Input for reconciliation of adjusted values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationInput {
    /// Adjusted values per comparable (typically adjusted price per SF or total).
    pub adjusted_values: Vec<Money>,
    /// Reconciliation method.
    pub method: ReconciliationMethod,
    /// Quality scores (1-5) per comparable; required for QualityScore method.
    pub quality_scores: Option<Vec<u32>>,
    /// Distances to subject in miles; required for InverseDistance method.
    pub distances: Option<Vec<Decimal>>,
    /// Desired confidence level for the interval (e.g. 0.90).
    pub confidence_level: Option<Rate>,
}

// ---------------------------------------------------------------------------
// Output structs
// ---------------------------------------------------------------------------

/// Per-comparable result from the adjustment grid.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdjustedComp {
    /// Original comparable address.
    pub address: String,
    /// Unadjusted sale price.
    pub unadjusted_price: Money,
    /// Gross cumulative adjustment percentage.
    pub gross_adjustment_pct: Rate,
    /// Net cumulative adjustment percentage.
    pub net_adjustment_pct: Rate,
    /// Adjusted sale price after all adjustments.
    pub adjusted_price: Money,
    /// Adjusted price per SF (GBA basis).
    pub adjusted_price_per_sf: Money,
    /// Implied cap rate at adjusted price (if subject NOI provided).
    pub implied_cap_rate: Option<Rate>,
}

/// Output of the comp adjustment grid.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompAdjustmentOutput {
    /// Subject property identifier.
    pub subject_address: String,
    /// Adjusted results per comparable.
    pub adjusted_comps: Vec<AdjustedComp>,
    /// Average adjusted price.
    pub average_adjusted_price: Money,
    /// Average adjusted price per SF.
    pub average_adjusted_price_per_sf: Money,
}

/// Per-comparable price-per-SF result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompPricePerSf {
    pub address: String,
    pub sale_price: Money,
    pub area_sf: Decimal,
    pub price_per_sf: Money,
    pub area_basis: AreaBasis,
}

/// Output of price-per-SF normalisation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricePerSfOutput {
    pub comps: Vec<CompPricePerSf>,
    pub mean_price_per_sf: Money,
    pub median_price_per_sf: Money,
    pub min_price_per_sf: Money,
    pub max_price_per_sf: Money,
}

/// Per-comparable cap rate extraction result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompCapRate {
    pub address: String,
    /// Going-in cap rate = actual NOI / sale price.
    pub going_in_cap_rate: Option<Rate>,
    /// TTM cap rate from trailing twelve-month NOI.
    pub ttm_cap_rate: Option<Rate>,
    /// Forward cap rate from projected NOI.
    pub forward_cap_rate: Option<Rate>,
    /// Normalised NOI after vacancy/expense/capex adjustments.
    pub normalised_noi: Option<Money>,
}

/// Output of cap rate extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapRateExtractionOutput {
    pub comps: Vec<CompCapRate>,
    pub mean_going_in_cap_rate: Option<Rate>,
    pub mean_forward_cap_rate: Option<Rate>,
    pub cap_rate_range: Option<(Rate, Rate)>,
}

/// Output of reconciliation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationOutput {
    /// Reconciled (weighted) value conclusion.
    pub reconciled_value: Money,
    /// Weight assigned to each comparable.
    pub weights: Vec<Rate>,
    /// Coefficient of variation (std dev / mean).
    pub coefficient_of_variation: Rate,
    /// Lower bound of confidence interval.
    pub confidence_interval_low: Money,
    /// Upper bound of confidence interval.
    pub confidence_interval_high: Money,
    /// Method used.
    pub method: ReconciliationMethod,
}

// ---------------------------------------------------------------------------
// 1. Comparable Adjustment Grid
// ---------------------------------------------------------------------------

/// Apply quantitative adjustments to comparable sales following the Appraisal
/// Institute sequence: transactional adjustments (property rights, financing,
/// conditions of sale, market conditions) applied cumulatively first, then
/// property adjustments (location, condition, size, age, amenities) applied
/// additively to the transaction-adjusted price.
pub fn comp_adjustment_grid(
    input: &CompAdjustmentInput,
) -> CorpFinanceResult<ComputationOutput<CompAdjustmentOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // RE-CONTRACT-007: minimum 3 comparables
    if input.comparables.len() < 3 {
        return Err(CorpFinanceError::InsufficientData(
            "Minimum 3 comparables required for adjustment grid (RE-CONTRACT-007)".into(),
        ));
    }

    if input.adjustments.len() != input.comparables.len() {
        return Err(CorpFinanceError::InvalidInput {
            field: "adjustments".into(),
            reason: "Adjustment vector length must match number of comparables".into(),
        });
    }

    let mut adjusted_comps: Vec<AdjustedComp> = Vec::with_capacity(input.comparables.len());

    for (i, comp) in input.comparables.iter().enumerate() {
        let adjs = &input.adjustments[i];

        // RE-CONTRACT-002: no single adjustment > +/-50%
        for adj in adjs {
            if adj.pct_adjustment.abs() > dec!(0.50) {
                return Err(CorpFinanceError::InvalidInput {
                    field: format!("adjustments[{}].{:?}", i, adj.category),
                    reason: format!(
                        "Single adjustment {:.2}% exceeds +/-50% limit (RE-CONTRACT-002)",
                        adj.pct_adjustment * dec!(100)
                    ),
                });
            }
        }

        // Separate into transactional vs property adjustments, then sort by
        // Appraisal Institute sequence within each group.
        let mut transactional: Vec<&Adjustment> = Vec::new();
        let mut property: Vec<&Adjustment> = Vec::new();
        for adj in adjs {
            if adj.category.sequence_order() <= 3 {
                transactional.push(adj);
            } else {
                property.push(adj);
            }
        }
        transactional.sort_by_key(|a| a.category.sequence_order());
        property.sort_by_key(|a| a.category.sequence_order());

        // Transactional adjustments are applied cumulatively (compounding).
        let mut transaction_adjusted = comp.sale_price;
        for adj in &transactional {
            transaction_adjusted += transaction_adjusted * adj.pct_adjustment;
        }

        // Property adjustments are additive on the transaction-adjusted price.
        let property_net: Decimal = property.iter().map(|a| a.pct_adjustment).sum();
        let adjusted_price = transaction_adjusted * (dec!(1) + property_net);

        // Gross and net adjustment percentages
        let gross_pct: Decimal = adjs.iter().map(|a| a.pct_adjustment.abs()).sum();
        let net_pct: Decimal = adjs.iter().map(|a| a.pct_adjustment).sum();

        if gross_pct > dec!(0.50) {
            warnings.push(format!(
                "Comp '{}': gross adjustment {:.1}% exceeds 50% — reliability may be diminished",
                comp.address,
                gross_pct * dec!(100)
            ));
        }

        let gba = comp.gross_building_area_sf;
        let adjusted_price_per_sf = if gba > Decimal::ZERO {
            adjusted_price / gba
        } else {
            warnings.push(format!(
                "Comp '{}': zero GBA — cannot compute price per SF",
                comp.address
            ));
            Decimal::ZERO
        };

        let implied_cap_rate = input.subject_noi.and_then(|noi| {
            if adjusted_price > Decimal::ZERO {
                Some(noi / adjusted_price)
            } else {
                None
            }
        });

        adjusted_comps.push(AdjustedComp {
            address: comp.address.clone(),
            unadjusted_price: comp.sale_price,
            gross_adjustment_pct: gross_pct,
            net_adjustment_pct: net_pct,
            adjusted_price,
            adjusted_price_per_sf,
            implied_cap_rate,
        });
    }

    let n = Decimal::from(adjusted_comps.len() as u64);
    let total_price: Decimal = adjusted_comps.iter().map(|c| c.adjusted_price).sum();
    let total_ppsf: Decimal = adjusted_comps
        .iter()
        .map(|c| c.adjusted_price_per_sf)
        .sum();

    let output = CompAdjustmentOutput {
        subject_address: input.subject_address.clone(),
        adjusted_comps,
        average_adjusted_price: total_price / n,
        average_adjusted_price_per_sf: total_ppsf / n,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Comparable Sales Adjustment Grid (Appraisal Institute Sequence)",
        input,
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// 2. Price Per SF Normalisation
// ---------------------------------------------------------------------------

/// Normalise sale prices to price-per-SF using the specified area basis
/// (GBA, NRA, or usable area).
pub fn price_per_sf(
    input: &PricePerSfInput,
) -> CorpFinanceResult<ComputationOutput<PricePerSfOutput>> {
    let start = Instant::now();
    let warnings: Vec<String> = Vec::new();

    if input.comparables.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one comparable required for price-per-SF calculation".into(),
        ));
    }

    let mut comps: Vec<CompPricePerSf> = Vec::with_capacity(input.comparables.len());

    for comp in &input.comparables {
        let area = match input.area_basis {
            AreaBasis::GrossBuildingArea => comp.gross_building_area_sf,
            AreaBasis::NetRentableArea => comp.net_rentable_area_sf.ok_or_else(|| {
                CorpFinanceError::InvalidInput {
                    field: format!("comparable '{}' net_rentable_area_sf", comp.address),
                    reason: "NRA not provided but NetRentableArea basis selected".into(),
                }
            })?,
            AreaBasis::UsableArea => comp.usable_area_sf.ok_or_else(|| {
                CorpFinanceError::InvalidInput {
                    field: format!("comparable '{}' usable_area_sf", comp.address),
                    reason: "Usable area not provided but UsableArea basis selected".into(),
                }
            })?,
        };

        if area <= Decimal::ZERO {
            return Err(CorpFinanceError::DivisionByZero {
                context: format!("price_per_sf for comp '{}'", comp.address),
            });
        }

        comps.push(CompPricePerSf {
            address: comp.address.clone(),
            sale_price: comp.sale_price,
            area_sf: area,
            price_per_sf: comp.sale_price / area,
            area_basis: input.area_basis.clone(),
        });
    }

    let mut ppsf_values: Vec<Decimal> = comps.iter().map(|c| c.price_per_sf).collect();
    ppsf_values.sort();

    let n = Decimal::from(ppsf_values.len() as u64);
    let mean = ppsf_values.iter().copied().sum::<Decimal>() / n;
    let min = ppsf_values[0];
    let max = *ppsf_values.last().unwrap();
    let median = compute_median(&ppsf_values);

    let output = PricePerSfOutput {
        comps,
        mean_price_per_sf: mean,
        median_price_per_sf: median,
        min_price_per_sf: min,
        max_price_per_sf: max,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Price Per Square Foot Normalisation",
        input,
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// 3. Cap Rate Extraction
// ---------------------------------------------------------------------------

/// Extract going-in, TTM, and forward cap rates from comparable transactions.
/// Applies NOI normalisation: vacancy adjustment to stabilised rate, expense
/// ratio standardisation, and CapEx reserve deduction.
pub fn cap_rate_extraction(
    input: &CapRateExtractionInput,
) -> CorpFinanceResult<ComputationOutput<CapRateExtractionOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    if input.comparables.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one comparable required for cap rate extraction".into(),
        ));
    }

    // Validate rates
    if input.stabilised_vacancy_rate < Decimal::ZERO || input.stabilised_vacancy_rate >= dec!(1) {
        return Err(CorpFinanceError::InvalidInput {
            field: "stabilised_vacancy_rate".into(),
            reason: "Must be in [0, 1)".into(),
        });
    }
    if input.market_expense_ratio < Decimal::ZERO || input.market_expense_ratio >= dec!(1) {
        return Err(CorpFinanceError::InvalidInput {
            field: "market_expense_ratio".into(),
            reason: "Must be in [0, 1)".into(),
        });
    }

    let mut comps: Vec<CompCapRate> = Vec::with_capacity(input.comparables.len());
    let mut going_in_rates: Vec<Decimal> = Vec::new();
    let mut forward_rates: Vec<Decimal> = Vec::new();

    for comp in &input.comparables {
        if comp.sale_price <= Decimal::ZERO {
            return Err(CorpFinanceError::DivisionByZero {
                context: format!("cap_rate_extraction: sale_price for '{}'", comp.address),
            });
        }

        // Going-in (raw) cap rate from reported NOI
        let going_in = comp.noi.map(|noi| noi / comp.sale_price);

        // TTM cap rate with NOI normalisation
        let (ttm_cap, normalised_noi) = if let Some(noi) = comp.noi {
            let occupancy = comp.occupancy_pct.unwrap_or(dec!(1));
            // Gross potential income implied by actual NOI and occupancy
            // NOI ≈ GPI * occupancy * (1 - expense_ratio) - capex
            // We back into GPI then re-stabilise.
            let actual_egi = if occupancy > Decimal::ZERO {
                noi / (dec!(1) - input.market_expense_ratio) / occupancy
            } else {
                warnings.push(format!(
                    "Comp '{}': zero occupancy — skipping TTM normalisation",
                    comp.address
                ));
                Decimal::ZERO
            };

            if actual_egi > Decimal::ZERO {
                let stabilised_egi = actual_egi * (dec!(1) - input.stabilised_vacancy_rate);
                let stabilised_noi = stabilised_egi * (dec!(1) - input.market_expense_ratio);
                let after_capex = stabilised_noi - (stabilised_egi * input.capex_reserve_pct);
                let cap = after_capex / comp.sale_price;

                // RE-CONTRACT-004: cap rate must be positive and < 1.0
                if cap <= Decimal::ZERO || cap >= dec!(1) {
                    warnings.push(format!(
                        "Comp '{}': normalised cap rate {:.4} outside (0,1) — excluded",
                        comp.address, cap
                    ));
                    (None, Some(after_capex))
                } else {
                    going_in_rates.push(cap);
                    (Some(cap), Some(after_capex))
                }
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        // Forward cap rate from projected NOI
        let fwd_cap = comp.forward_noi.and_then(|fwd| {
            let cap = fwd / comp.sale_price;
            if cap <= Decimal::ZERO || cap >= dec!(1) {
                warnings.push(format!(
                    "Comp '{}': forward cap rate {:.4} outside (0,1) — excluded",
                    comp.address, cap
                ));
                None
            } else {
                forward_rates.push(cap);
                Some(cap)
            }
        });

        comps.push(CompCapRate {
            address: comp.address.clone(),
            going_in_cap_rate: going_in,
            ttm_cap_rate: ttm_cap,
            forward_cap_rate: fwd_cap,
            normalised_noi,
        });
    }

    let mean_going_in = if going_in_rates.is_empty() {
        None
    } else {
        let n = Decimal::from(going_in_rates.len() as u64);
        Some(going_in_rates.iter().copied().sum::<Decimal>() / n)
    };

    let mean_forward = if forward_rates.is_empty() {
        None
    } else {
        let n = Decimal::from(forward_rates.len() as u64);
        Some(forward_rates.iter().copied().sum::<Decimal>() / n)
    };

    let all_rates: Vec<Decimal> = going_in_rates
        .iter()
        .chain(forward_rates.iter())
        .copied()
        .collect();
    let cap_rate_range = if all_rates.is_empty() {
        None
    } else {
        let min = all_rates.iter().copied().fold(Decimal::MAX, Decimal::min);
        let max = all_rates.iter().copied().fold(Decimal::ZERO, Decimal::max);
        Some((min, max))
    };

    let output = CapRateExtractionOutput {
        comps,
        mean_going_in_cap_rate: mean_going_in,
        mean_forward_cap_rate: mean_forward,
        cap_rate_range,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Cap Rate Extraction (Going-In / TTM / Forward)",
        input,
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// 4. Reconciliation
// ---------------------------------------------------------------------------

/// Reconcile multiple adjusted comparable values into a single value conclusion
/// using equal weighting, quality-score weighting, or inverse-distance weighting.
pub fn reconciliation(
    input: &ReconciliationInput,
) -> CorpFinanceResult<ComputationOutput<ReconciliationOutput>> {
    let start = Instant::now();
    let warnings: Vec<String> = Vec::new();

    let n = input.adjusted_values.len();

    // RE-CONTRACT-007: minimum 3 comparables
    if n < 3 {
        return Err(CorpFinanceError::InsufficientData(
            "Minimum 3 adjusted values required for reconciliation (RE-CONTRACT-007)".into(),
        ));
    }

    let weights: Vec<Decimal> = match &input.method {
        ReconciliationMethod::EqualWeight => {
            let w = dec!(1) / Decimal::from(n as u64);
            vec![w; n]
        }
        ReconciliationMethod::QualityScore => {
            let scores = input.quality_scores.as_ref().ok_or_else(|| {
                CorpFinanceError::InvalidInput {
                    field: "quality_scores".into(),
                    reason: "Quality scores required for QualityScore method".into(),
                }
            })?;
            if scores.len() != n {
                return Err(CorpFinanceError::InvalidInput {
                    field: "quality_scores".into(),
                    reason: "Length must match adjusted_values".into(),
                });
            }
            for &s in scores {
                if !(1..=5).contains(&s) {
                    return Err(CorpFinanceError::InvalidInput {
                        field: "quality_scores".into(),
                        reason: format!("Score {} outside 1-5 range", s),
                    });
                }
            }
            let total: Decimal = scores.iter().map(|&s| Decimal::from(s)).sum();
            scores
                .iter()
                .map(|&s| Decimal::from(s) / total)
                .collect()
        }
        ReconciliationMethod::InverseDistance => {
            let distances = input.distances.as_ref().ok_or_else(|| {
                CorpFinanceError::InvalidInput {
                    field: "distances".into(),
                    reason: "Distances required for InverseDistance method".into(),
                }
            })?;
            if distances.len() != n {
                return Err(CorpFinanceError::InvalidInput {
                    field: "distances".into(),
                    reason: "Length must match adjusted_values".into(),
                });
            }
            for d in distances {
                if *d <= Decimal::ZERO {
                    return Err(CorpFinanceError::InvalidInput {
                        field: "distances".into(),
                        reason: "All distances must be positive".into(),
                    });
                }
            }
            let inv_sum: Decimal = distances.iter().map(|d| dec!(1) / *d).sum();
            distances
                .iter()
                .map(|d| (dec!(1) / *d) / inv_sum)
                .collect()
        }
    };

    // Weighted value
    let reconciled_value: Decimal = input
        .adjusted_values
        .iter()
        .zip(weights.iter())
        .map(|(v, w)| *v * *w)
        .sum();

    // Mean and std dev for CV and confidence interval
    let mean = input.adjusted_values.iter().copied().sum::<Decimal>()
        / Decimal::from(n as u64);

    let variance: Decimal = input
        .adjusted_values
        .iter()
        .map(|v| {
            let diff = *v - mean;
            diff * diff
        })
        .sum::<Decimal>()
        / Decimal::from(n as u64);

    let std_dev = decimal_sqrt(variance);

    let cv = if mean > Decimal::ZERO {
        std_dev / mean
    } else {
        Decimal::ZERO
    };

    // Confidence interval using z-score approximation
    let z = match input.confidence_level {
        Some(cl) if cl >= dec!(0.99) => dec!(2.576),
        Some(cl) if cl >= dec!(0.95) => dec!(1.960),
        Some(cl) if cl >= dec!(0.90) => dec!(1.645),
        _ => dec!(1.960), // default 95%
    };
    let se = std_dev / decimal_sqrt(Decimal::from(n as u64));
    let ci_low = reconciled_value - z * se;
    let ci_high = reconciled_value + z * se;

    let output = ReconciliationOutput {
        reconciled_value,
        weights,
        coefficient_of_variation: cv,
        confidence_interval_low: ci_low,
        confidence_interval_high: ci_high,
        method: input.method.clone(),
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Comparable Sales Reconciliation",
        input,
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Newton's method square root for Decimal (20 iterations).
fn decimal_sqrt(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    let mut guess = x / dec!(2);
    if guess == Decimal::ZERO {
        guess = dec!(1);
    }
    for _ in 0..20 {
        guess = (guess + x / guess) / dec!(2);
    }
    guess
}

/// Compute median of a sorted slice.
fn compute_median(sorted: &[Decimal]) -> Decimal {
    let n = sorted.len();
    if n == 0 {
        return Decimal::ZERO;
    }
    if n % 2 == 1 {
        sorted[n / 2]
    } else {
        (sorted[n / 2 - 1] + sorted[n / 2]) / dec!(2)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    // ----- Test helpers -----

    fn make_comp(addr: &str, price: Decimal, gba: Decimal, year: u32) -> Comparable {
        Comparable {
            address: addr.to_string(),
            sale_price: price,
            sale_date: "2024-06-15".to_string(),
            gross_building_area_sf: gba,
            net_rentable_area_sf: Some(gba * dec!(0.85)),
            usable_area_sf: Some(gba * dec!(0.78)),
            year_built: year,
            noi: Some(price * dec!(0.065)),
            forward_noi: Some(price * dec!(0.070)),
            occupancy_pct: Some(dec!(0.93)),
            property_type: "Office".to_string(),
            condition_rating: Some(3),
            quality_score: Some(3),
            distance_to_subject: Some(dec!(2.5)),
        }
    }

    fn three_comps() -> Vec<Comparable> {
        vec![
            make_comp("100 Main St", dec!(5_000_000), dec!(20_000), 2005),
            make_comp("200 Oak Ave", dec!(6_000_000), dec!(22_000), 2010),
            make_comp("300 Elm Blvd", dec!(5_500_000), dec!(21_000), 2008),
        ]
    }

    fn three_adj_sets() -> Vec<Vec<Adjustment>> {
        vec![
            vec![
                Adjustment {
                    category: AdjustmentCategory::MarketConditions,
                    pct_adjustment: dec!(0.03),
                    narrative: "Time adjustment +3%".into(),
                },
                Adjustment {
                    category: AdjustmentCategory::Location,
                    pct_adjustment: dec!(0.05),
                    narrative: "Superior location +5%".into(),
                },
                Adjustment {
                    category: AdjustmentCategory::Condition,
                    pct_adjustment: dec!(-0.02),
                    narrative: "Inferior condition -2%".into(),
                },
            ],
            vec![
                Adjustment {
                    category: AdjustmentCategory::MarketConditions,
                    pct_adjustment: dec!(0.02),
                    narrative: "Time adjustment +2%".into(),
                },
                Adjustment {
                    category: AdjustmentCategory::Size,
                    pct_adjustment: dec!(-0.03),
                    narrative: "Larger building -3%".into(),
                },
            ],
            vec![
                Adjustment {
                    category: AdjustmentCategory::Location,
                    pct_adjustment: dec!(-0.04),
                    narrative: "Inferior location -4%".into(),
                },
                Adjustment {
                    category: AdjustmentCategory::Age,
                    pct_adjustment: dec!(0.02),
                    narrative: "Newer subject +2%".into(),
                },
            ],
        ]
    }

    // ========================================================================
    // comp_adjustment_grid tests
    // ========================================================================

    #[test]
    fn adjustment_grid_basic() {
        let input = CompAdjustmentInput {
            subject_address: "Subject Property".into(),
            comparables: three_comps(),
            adjustments: three_adj_sets(),
            subject_noi: Some(dec!(400_000)),
        };
        let result = comp_adjustment_grid(&input).unwrap();
        assert_eq!(result.result.adjusted_comps.len(), 3);
        for ac in &result.result.adjusted_comps {
            assert!(ac.adjusted_price > Decimal::ZERO);
            assert!(ac.adjusted_price_per_sf > Decimal::ZERO);
            assert!(ac.implied_cap_rate.unwrap() > Decimal::ZERO);
        }
    }

    #[test]
    fn adjustment_grid_net_adjustment_sign() {
        let input = CompAdjustmentInput {
            subject_address: "Subject".into(),
            comparables: three_comps(),
            adjustments: three_adj_sets(),
            subject_noi: None,
        };
        let result = comp_adjustment_grid(&input).unwrap();
        // Comp 0: +3% +5% -2% = +6% net
        assert!(result.result.adjusted_comps[0].net_adjustment_pct > Decimal::ZERO);
        // Comp 2: -4% +2% = -2% net
        assert!(result.result.adjusted_comps[2].net_adjustment_pct < Decimal::ZERO);
    }

    #[test]
    fn adjustment_grid_min_comps_error() {
        let input = CompAdjustmentInput {
            subject_address: "Subject".into(),
            comparables: vec![make_comp("A", dec!(1_000_000), dec!(10_000), 2020)],
            adjustments: vec![vec![]],
            subject_noi: None,
        };
        let err = comp_adjustment_grid(&input).unwrap_err();
        assert!(err.to_string().contains("Minimum 3"));
    }

    #[test]
    fn adjustment_grid_mismatched_lengths() {
        let input = CompAdjustmentInput {
            subject_address: "S".into(),
            comparables: three_comps(),
            adjustments: vec![vec![], vec![]], // only 2
            subject_noi: None,
        };
        assert!(comp_adjustment_grid(&input).is_err());
    }

    #[test]
    fn adjustment_grid_exceeds_50pct() {
        let mut adjs = three_adj_sets();
        adjs[0].push(Adjustment {
            category: AdjustmentCategory::Amenities,
            pct_adjustment: dec!(0.55),
            narrative: "Too large".into(),
        });
        let input = CompAdjustmentInput {
            subject_address: "S".into(),
            comparables: three_comps(),
            adjustments: adjs,
            subject_noi: None,
        };
        let err = comp_adjustment_grid(&input).unwrap_err();
        assert!(err.to_string().contains("50%"));
    }

    #[test]
    fn adjustment_grid_negative_50pct() {
        let mut adjs = three_adj_sets();
        adjs[1] = vec![Adjustment {
            category: AdjustmentCategory::Location,
            pct_adjustment: dec!(-0.51),
            narrative: "Too negative".into(),
        }];
        let input = CompAdjustmentInput {
            subject_address: "S".into(),
            comparables: three_comps(),
            adjustments: adjs,
            subject_noi: None,
        };
        assert!(comp_adjustment_grid(&input).is_err());
    }

    #[test]
    fn adjustment_grid_no_adjustments() {
        let input = CompAdjustmentInput {
            subject_address: "S".into(),
            comparables: three_comps(),
            adjustments: vec![vec![], vec![], vec![]],
            subject_noi: None,
        };
        let result = comp_adjustment_grid(&input).unwrap();
        // With no adjustments, adjusted price should equal unadjusted.
        for ac in &result.result.adjusted_comps {
            assert_eq!(ac.adjusted_price, ac.unadjusted_price);
            assert_eq!(ac.net_adjustment_pct, Decimal::ZERO);
        }
    }

    #[test]
    fn adjustment_grid_transactional_before_property() {
        // Transactional: +10% market conditions, Property: +10% location
        // Compounding transactional: 5M * 1.10 = 5.5M
        // Then additive property: 5.5M * (1 + 0.10) = 6.05M
        let comps = three_comps();
        let adjs = vec![
            vec![
                Adjustment {
                    category: AdjustmentCategory::MarketConditions,
                    pct_adjustment: dec!(0.10),
                    narrative: "Time".into(),
                },
                Adjustment {
                    category: AdjustmentCategory::Location,
                    pct_adjustment: dec!(0.10),
                    narrative: "Location".into(),
                },
            ],
            vec![],
            vec![],
        ];
        let input = CompAdjustmentInput {
            subject_address: "S".into(),
            comparables: comps,
            adjustments: adjs,
            subject_noi: None,
        };
        let result = comp_adjustment_grid(&input).unwrap();
        let adj_price = result.result.adjusted_comps[0].adjusted_price;
        // 5_000_000 * 1.10 * 1.10 = 6_050_000
        assert_eq!(adj_price, dec!(6_050_000));
    }

    #[test]
    fn adjustment_grid_average_values() {
        let input = CompAdjustmentInput {
            subject_address: "S".into(),
            comparables: three_comps(),
            adjustments: vec![vec![], vec![], vec![]],
            subject_noi: None,
        };
        let result = comp_adjustment_grid(&input).unwrap();
        let avg = result.result.average_adjusted_price;
        let expected = (dec!(5_000_000) + dec!(6_000_000) + dec!(5_500_000)) / dec!(3);
        assert_eq!(avg, expected);
    }

    #[test]
    fn adjustment_grid_zero_gba_warning() {
        let mut comps = three_comps();
        comps[0].gross_building_area_sf = Decimal::ZERO;
        let input = CompAdjustmentInput {
            subject_address: "S".into(),
            comparables: comps,
            adjustments: vec![vec![], vec![], vec![]],
            subject_noi: None,
        };
        let result = comp_adjustment_grid(&input).unwrap();
        assert!(result.warnings.iter().any(|w| w.contains("zero GBA")));
    }

    #[test]
    fn adjustment_grid_implied_cap_rate() {
        let input = CompAdjustmentInput {
            subject_address: "S".into(),
            comparables: three_comps(),
            adjustments: vec![vec![], vec![], vec![]],
            subject_noi: Some(dec!(350_000)),
        };
        let result = comp_adjustment_grid(&input).unwrap();
        let cap = result.result.adjusted_comps[0].implied_cap_rate.unwrap();
        assert_eq!(cap, dec!(350_000) / dec!(5_000_000));
    }

    // ========================================================================
    // price_per_sf tests
    // ========================================================================

    #[test]
    fn ppsf_gba_basis() {
        let input = PricePerSfInput {
            comparables: three_comps(),
            area_basis: AreaBasis::GrossBuildingArea,
        };
        let result = price_per_sf(&input).unwrap();
        assert_eq!(result.result.comps.len(), 3);
        // Comp 0: 5M / 20000 = 250
        assert_eq!(result.result.comps[0].price_per_sf, dec!(250));
    }

    #[test]
    fn ppsf_nra_basis() {
        let input = PricePerSfInput {
            comparables: three_comps(),
            area_basis: AreaBasis::NetRentableArea,
        };
        let result = price_per_sf(&input).unwrap();
        // NRA = 20000 * 0.85 = 17000, PPSF = 5M / 17000
        let expected = dec!(5_000_000) / dec!(17_000);
        assert_eq!(result.result.comps[0].price_per_sf, expected);
    }

    #[test]
    fn ppsf_usable_basis() {
        let input = PricePerSfInput {
            comparables: three_comps(),
            area_basis: AreaBasis::UsableArea,
        };
        let result = price_per_sf(&input).unwrap();
        let expected = dec!(5_000_000) / (dec!(20_000) * dec!(0.78));
        assert_eq!(result.result.comps[0].price_per_sf, expected);
    }

    #[test]
    fn ppsf_nra_missing_error() {
        let mut comps = three_comps();
        comps[1].net_rentable_area_sf = None;
        let input = PricePerSfInput {
            comparables: comps,
            area_basis: AreaBasis::NetRentableArea,
        };
        assert!(price_per_sf(&input).is_err());
    }

    #[test]
    fn ppsf_zero_area_error() {
        let mut comps = three_comps();
        comps[0].gross_building_area_sf = Decimal::ZERO;
        let input = PricePerSfInput {
            comparables: comps,
            area_basis: AreaBasis::GrossBuildingArea,
        };
        assert!(price_per_sf(&input).is_err());
    }

    #[test]
    fn ppsf_empty_comps_error() {
        let input = PricePerSfInput {
            comparables: vec![],
            area_basis: AreaBasis::GrossBuildingArea,
        };
        assert!(price_per_sf(&input).is_err());
    }

    #[test]
    fn ppsf_median_odd() {
        let input = PricePerSfInput {
            comparables: three_comps(),
            area_basis: AreaBasis::GrossBuildingArea,
        };
        let result = price_per_sf(&input).unwrap();
        // Prices per SF: 250, 272.727..., 261.904...
        // Sorted: 250, 261.904, 272.727 => median = 261.904...
        let median = result.result.median_price_per_sf;
        assert!(median > dec!(261) && median < dec!(263));
    }

    #[test]
    fn ppsf_min_max() {
        let input = PricePerSfInput {
            comparables: three_comps(),
            area_basis: AreaBasis::GrossBuildingArea,
        };
        let result = price_per_sf(&input).unwrap();
        assert_eq!(result.result.min_price_per_sf, dec!(250));
        assert!(result.result.max_price_per_sf > dec!(272));
    }

    // ========================================================================
    // cap_rate_extraction tests
    // ========================================================================

    #[test]
    fn cap_rate_basic() {
        let input = CapRateExtractionInput {
            comparables: three_comps(),
            stabilised_vacancy_rate: dec!(0.05),
            market_expense_ratio: dec!(0.40),
            capex_reserve_pct: dec!(0.03),
        };
        let result = cap_rate_extraction(&input).unwrap();
        assert_eq!(result.result.comps.len(), 3);
        for comp in &result.result.comps {
            assert!(comp.going_in_cap_rate.is_some());
            assert!(comp.forward_cap_rate.is_some());
        }
    }

    #[test]
    fn cap_rate_going_in_value() {
        let comps = three_comps();
        let input = CapRateExtractionInput {
            comparables: comps.clone(),
            stabilised_vacancy_rate: dec!(0.05),
            market_expense_ratio: dec!(0.40),
            capex_reserve_pct: dec!(0.03),
        };
        let result = cap_rate_extraction(&input).unwrap();
        // Comp 0: NOI = 5M * 0.065 = 325000, going-in = 325000/5M = 0.065
        let gi = result.result.comps[0].going_in_cap_rate.unwrap();
        assert_eq!(gi, dec!(0.065));
    }

    #[test]
    fn cap_rate_forward_value() {
        let comps = three_comps();
        let input = CapRateExtractionInput {
            comparables: comps.clone(),
            stabilised_vacancy_rate: dec!(0.05),
            market_expense_ratio: dec!(0.40),
            capex_reserve_pct: dec!(0.03),
        };
        let result = cap_rate_extraction(&input).unwrap();
        // Comp 0: forward NOI = 5M * 0.070 = 350000, fwd cap = 350000/5M = 0.070
        let fwd = result.result.comps[0].forward_cap_rate.unwrap();
        assert_eq!(fwd, dec!(0.070));
    }

    #[test]
    fn cap_rate_no_noi() {
        let mut comps = three_comps();
        comps[0].noi = None;
        comps[0].forward_noi = None;
        let input = CapRateExtractionInput {
            comparables: comps,
            stabilised_vacancy_rate: dec!(0.05),
            market_expense_ratio: dec!(0.40),
            capex_reserve_pct: dec!(0.03),
        };
        let result = cap_rate_extraction(&input).unwrap();
        assert!(result.result.comps[0].going_in_cap_rate.is_none());
        assert!(result.result.comps[0].forward_cap_rate.is_none());
    }

    #[test]
    fn cap_rate_zero_sale_price_error() {
        let mut comps = three_comps();
        comps[0].sale_price = Decimal::ZERO;
        let input = CapRateExtractionInput {
            comparables: comps,
            stabilised_vacancy_rate: dec!(0.05),
            market_expense_ratio: dec!(0.40),
            capex_reserve_pct: dec!(0.03),
        };
        assert!(cap_rate_extraction(&input).is_err());
    }

    #[test]
    fn cap_rate_invalid_vacancy_error() {
        let input = CapRateExtractionInput {
            comparables: three_comps(),
            stabilised_vacancy_rate: dec!(1.5),
            market_expense_ratio: dec!(0.40),
            capex_reserve_pct: dec!(0.03),
        };
        assert!(cap_rate_extraction(&input).is_err());
    }

    #[test]
    fn cap_rate_empty_comps_error() {
        let input = CapRateExtractionInput {
            comparables: vec![],
            stabilised_vacancy_rate: dec!(0.05),
            market_expense_ratio: dec!(0.40),
            capex_reserve_pct: dec!(0.03),
        };
        assert!(cap_rate_extraction(&input).is_err());
    }

    #[test]
    fn cap_rate_range() {
        let input = CapRateExtractionInput {
            comparables: three_comps(),
            stabilised_vacancy_rate: dec!(0.05),
            market_expense_ratio: dec!(0.40),
            capex_reserve_pct: dec!(0.03),
        };
        let result = cap_rate_extraction(&input).unwrap();
        let (low, high) = result.result.cap_rate_range.unwrap();
        assert!(low > Decimal::ZERO);
        assert!(high >= low);
        assert!(high < dec!(1));
    }

    #[test]
    fn cap_rate_mean_going_in() {
        let input = CapRateExtractionInput {
            comparables: three_comps(),
            stabilised_vacancy_rate: dec!(0.05),
            market_expense_ratio: dec!(0.40),
            capex_reserve_pct: dec!(0.03),
        };
        let result = cap_rate_extraction(&input).unwrap();
        // All comps have same cap rate structure (6.5%), mean should be close
        assert!(result.result.mean_going_in_cap_rate.is_some());
    }

    // ========================================================================
    // reconciliation tests
    // ========================================================================

    #[test]
    fn reconcile_equal_weight() {
        let input = ReconciliationInput {
            adjusted_values: vec![dec!(5_000_000), dec!(5_500_000), dec!(5_200_000)],
            method: ReconciliationMethod::EqualWeight,
            quality_scores: None,
            distances: None,
            confidence_level: Some(dec!(0.95)),
        };
        let result = reconciliation(&input).unwrap();
        let expected = (dec!(5_000_000) + dec!(5_500_000) + dec!(5_200_000)) / dec!(3);
        assert!((result.result.reconciled_value - expected).abs() < dec!(0.01));
        assert_eq!(result.result.weights.len(), 3);
    }

    #[test]
    fn reconcile_quality_score() {
        let input = ReconciliationInput {
            adjusted_values: vec![dec!(5_000_000), dec!(5_500_000), dec!(5_200_000)],
            method: ReconciliationMethod::QualityScore,
            quality_scores: Some(vec![5, 3, 2]),
            distances: None,
            confidence_level: None,
        };
        let result = reconciliation(&input).unwrap();
        // Weights: 5/10, 3/10, 2/10
        let expected = dec!(5_000_000) * dec!(0.5)
            + dec!(5_500_000) * dec!(0.3)
            + dec!(5_200_000) * dec!(0.2);
        assert_eq!(result.result.reconciled_value, expected);
    }

    #[test]
    fn reconcile_inverse_distance() {
        let input = ReconciliationInput {
            adjusted_values: vec![dec!(5_000_000), dec!(5_500_000), dec!(5_200_000)],
            method: ReconciliationMethod::InverseDistance,
            quality_scores: None,
            distances: Some(vec![dec!(1), dec!(2), dec!(4)]),
            confidence_level: None,
        };
        let result = reconciliation(&input).unwrap();
        // inv: 1, 0.5, 0.25 => sum 1.75
        // weights: 1/1.75, 0.5/1.75, 0.25/1.75
        let inv_sum = dec!(1) + dec!(0.5) + dec!(0.25);
        let expected = dec!(5_000_000) / inv_sum
            + dec!(5_500_000) * dec!(0.5) / inv_sum
            + dec!(5_200_000) * dec!(0.25) / inv_sum;
        assert!((result.result.reconciled_value - expected).abs() < dec!(0.01));
    }

    #[test]
    fn reconcile_min_comps_error() {
        let input = ReconciliationInput {
            adjusted_values: vec![dec!(5_000_000), dec!(5_500_000)],
            method: ReconciliationMethod::EqualWeight,
            quality_scores: None,
            distances: None,
            confidence_level: None,
        };
        let err = reconciliation(&input).unwrap_err();
        assert!(err.to_string().contains("Minimum 3"));
    }

    #[test]
    fn reconcile_quality_missing_scores_error() {
        let input = ReconciliationInput {
            adjusted_values: vec![dec!(5_000_000), dec!(5_500_000), dec!(5_200_000)],
            method: ReconciliationMethod::QualityScore,
            quality_scores: None,
            distances: None,
            confidence_level: None,
        };
        assert!(reconciliation(&input).is_err());
    }

    #[test]
    fn reconcile_quality_bad_score_error() {
        let input = ReconciliationInput {
            adjusted_values: vec![dec!(5_000_000), dec!(5_500_000), dec!(5_200_000)],
            method: ReconciliationMethod::QualityScore,
            quality_scores: Some(vec![5, 6, 3]), // 6 out of range
            distances: None,
            confidence_level: None,
        };
        assert!(reconciliation(&input).is_err());
    }

    #[test]
    fn reconcile_distance_missing_error() {
        let input = ReconciliationInput {
            adjusted_values: vec![dec!(5_000_000), dec!(5_500_000), dec!(5_200_000)],
            method: ReconciliationMethod::InverseDistance,
            quality_scores: None,
            distances: None,
            confidence_level: None,
        };
        assert!(reconciliation(&input).is_err());
    }

    #[test]
    fn reconcile_distance_zero_error() {
        let input = ReconciliationInput {
            adjusted_values: vec![dec!(5_000_000), dec!(5_500_000), dec!(5_200_000)],
            method: ReconciliationMethod::InverseDistance,
            quality_scores: None,
            distances: Some(vec![dec!(1), dec!(0), dec!(3)]),
            confidence_level: None,
        };
        assert!(reconciliation(&input).is_err());
    }

    #[test]
    fn reconcile_cv_positive() {
        let input = ReconciliationInput {
            adjusted_values: vec![dec!(5_000_000), dec!(6_000_000), dec!(5_500_000)],
            method: ReconciliationMethod::EqualWeight,
            quality_scores: None,
            distances: None,
            confidence_level: None,
        };
        let result = reconciliation(&input).unwrap();
        assert!(result.result.coefficient_of_variation > Decimal::ZERO);
    }

    #[test]
    fn reconcile_cv_zero_identical() {
        let input = ReconciliationInput {
            adjusted_values: vec![dec!(5_000_000), dec!(5_000_000), dec!(5_000_000)],
            method: ReconciliationMethod::EqualWeight,
            quality_scores: None,
            distances: None,
            confidence_level: None,
        };
        let result = reconciliation(&input).unwrap();
        assert_eq!(result.result.coefficient_of_variation, Decimal::ZERO);
    }

    #[test]
    fn reconcile_ci_narrows_with_more_comps() {
        let input_3 = ReconciliationInput {
            adjusted_values: vec![dec!(5_000_000), dec!(6_000_000), dec!(5_500_000)],
            method: ReconciliationMethod::EqualWeight,
            quality_scores: None,
            distances: None,
            confidence_level: Some(dec!(0.95)),
        };
        let input_5 = ReconciliationInput {
            adjusted_values: vec![
                dec!(5_000_000),
                dec!(6_000_000),
                dec!(5_500_000),
                dec!(5_200_000),
                dec!(5_800_000),
            ],
            method: ReconciliationMethod::EqualWeight,
            quality_scores: None,
            distances: None,
            confidence_level: Some(dec!(0.95)),
        };
        let r3 = reconciliation(&input_3).unwrap();
        let r5 = reconciliation(&input_5).unwrap();
        let width_3 = r3.result.confidence_interval_high - r3.result.confidence_interval_low;
        let width_5 = r5.result.confidence_interval_high - r5.result.confidence_interval_low;
        // More comps should yield tighter interval (lower SE)
        assert!(width_5 < width_3);
    }

    #[test]
    fn reconcile_confidence_90() {
        let input = ReconciliationInput {
            adjusted_values: vec![dec!(5_000_000), dec!(6_000_000), dec!(5_500_000)],
            method: ReconciliationMethod::EqualWeight,
            quality_scores: None,
            distances: None,
            confidence_level: Some(dec!(0.90)),
        };
        let result = reconciliation(&input).unwrap();
        assert!(result.result.confidence_interval_low < result.result.reconciled_value);
        assert!(result.result.confidence_interval_high > result.result.reconciled_value);
    }

    #[test]
    fn reconcile_weights_sum_to_one_equal() {
        let input = ReconciliationInput {
            adjusted_values: vec![dec!(5_000_000), dec!(5_500_000), dec!(5_200_000)],
            method: ReconciliationMethod::EqualWeight,
            quality_scores: None,
            distances: None,
            confidence_level: None,
        };
        let result = reconciliation(&input).unwrap();
        let sum: Decimal = result.result.weights.iter().copied().sum();
        // Allow tiny rounding tolerance
        assert!((sum - dec!(1)).abs() < dec!(0.0001));
    }

    #[test]
    fn reconcile_weights_sum_to_one_quality() {
        let input = ReconciliationInput {
            adjusted_values: vec![dec!(5_000_000), dec!(5_500_000), dec!(5_200_000)],
            method: ReconciliationMethod::QualityScore,
            quality_scores: Some(vec![5, 3, 2]),
            distances: None,
            confidence_level: None,
        };
        let result = reconciliation(&input).unwrap();
        let sum: Decimal = result.result.weights.iter().copied().sum();
        assert_eq!(sum, dec!(1));
    }

    #[test]
    fn reconcile_weights_sum_to_one_distance() {
        let input = ReconciliationInput {
            adjusted_values: vec![dec!(5_000_000), dec!(5_500_000), dec!(5_200_000)],
            method: ReconciliationMethod::InverseDistance,
            quality_scores: None,
            distances: Some(vec![dec!(1), dec!(2), dec!(5)]),
            confidence_level: None,
        };
        let result = reconciliation(&input).unwrap();
        let sum: Decimal = result.result.weights.iter().copied().sum();
        assert!((sum - dec!(1)).abs() < dec!(0.0001));
    }

    // ========================================================================
    // Helper tests
    // ========================================================================

    #[test]
    fn sqrt_known_values() {
        let s4 = decimal_sqrt(dec!(4));
        assert!((s4 - dec!(2)).abs() < dec!(0.0000001));
        let s9 = decimal_sqrt(dec!(9));
        assert!((s9 - dec!(3)).abs() < dec!(0.0000001));
    }

    #[test]
    fn sqrt_zero() {
        assert_eq!(decimal_sqrt(Decimal::ZERO), Decimal::ZERO);
    }

    #[test]
    fn median_odd() {
        assert_eq!(compute_median(&[dec!(1), dec!(3), dec!(5)]), dec!(3));
    }

    #[test]
    fn median_even() {
        assert_eq!(
            compute_median(&[dec!(1), dec!(3), dec!(5), dec!(7)]),
            dec!(4)
        );
    }
}
