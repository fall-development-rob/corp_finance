use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Input Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestedParty {
    pub name: String,
    pub jurisdiction: String,
    pub function: String,
    pub operating_revenue: Decimal,
    pub operating_costs: Decimal,
    pub operating_profit: Decimal,
    pub assets: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comparable {
    pub name: String,
    pub net_margin: Decimal,
    pub berry_ratio: Option<Decimal>,
    pub return_on_assets: Option<Decimal>,
    pub gross_margin: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CfcParams {
    pub parent_jurisdiction: String,
    pub subsidiary_jurisdiction: String,
    pub subsidiary_income: Decimal,
    pub subsidiary_tax_paid: Decimal,
    pub passive_income_pct: Decimal,
    pub ownership_pct: Decimal,
    pub de_minimis_threshold: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntercompanyInput {
    pub transaction_name: String,
    pub pricing_method: String,
    pub tested_party: TestedParty,
    pub comparables: Vec<Comparable>,
    pub transaction_value: Decimal,
    pub cfc_analysis: Option<CfcParams>,
    pub gaar_jurisdiction: Option<String>,
}

// ---------------------------------------------------------------------------
// Output Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestedPartyResult {
    pub operating_margin: Decimal,
    pub berry_ratio: Decimal,
    pub return_on_assets: Decimal,
    pub gross_margin: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparableRange {
    pub count: u32,
    pub p25: Decimal,
    pub median: Decimal,
    pub p75: Decimal,
    pub mean: Decimal,
    pub std_dev: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArmLengthResult {
    pub within_range: bool,
    pub tested_value: Decimal,
    pub range_low: Decimal,
    pub range_high: Decimal,
    pub adjustment_needed: Decimal,
    pub adjustment_direction: String,
    pub confidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingDetail {
    pub method_description: String,
    pub arm_length_price: Decimal,
    pub current_price: Decimal,
    pub deviation_pct: Decimal,
    pub tax_impact_estimate: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CfcResult {
    pub cfc_regime: String,
    pub inclusion_required: bool,
    pub inclusion_amount: Decimal,
    pub effective_tax_rate: Decimal,
    pub tax_liability: Decimal,
    pub exemptions_available: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GaarResult {
    pub risk_level: String,
    pub main_purpose_risk: bool,
    pub substance_adequate: bool,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntercompanyOutput {
    pub method_used: String,
    pub tested_party_result: TestedPartyResult,
    pub comparable_range: ComparableRange,
    pub arm_length_assessment: ArmLengthResult,
    pub pricing_analysis: PricingDetail,
    pub cfc_analysis: Option<CfcResult>,
    pub gaar_assessment: Option<GaarResult>,
    pub documentation_requirements: Vec<String>,
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

const VALID_METHODS: &[&str] = &["CUP", "RPM", "CPLM", "TNMM", "ProfitSplit"];

fn validate_intercompany_input(input: &IntercompanyInput) -> CorpFinanceResult<()> {
    if !VALID_METHODS.contains(&input.pricing_method.as_str()) {
        return Err(CorpFinanceError::InvalidInput {
            field: "pricing_method".into(),
            reason: format!(
                "Invalid method '{}'. Valid: {:?}",
                input.pricing_method, VALID_METHODS
            ),
        });
    }

    if input.transaction_value < dec!(0) {
        return Err(CorpFinanceError::InvalidInput {
            field: "transaction_value".into(),
            reason: "Must be non-negative".into(),
        });
    }

    if input.tested_party.operating_revenue < dec!(0) {
        return Err(CorpFinanceError::InvalidInput {
            field: "tested_party.operating_revenue".into(),
            reason: "Must be non-negative".into(),
        });
    }

    if input.tested_party.operating_costs < dec!(0) {
        return Err(CorpFinanceError::InvalidInput {
            field: "tested_party.operating_costs".into(),
            reason: "Must be non-negative".into(),
        });
    }

    if input.tested_party.assets < dec!(0) {
        return Err(CorpFinanceError::InvalidInput {
            field: "tested_party.assets".into(),
            reason: "Must be non-negative".into(),
        });
    }

    if input.comparables.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one comparable is required".to_string(),
        ));
    }

    if let Some(ref cfc) = input.cfc_analysis {
        if cfc.subsidiary_income < dec!(0) {
            return Err(CorpFinanceError::InvalidInput {
                field: "cfc_analysis.subsidiary_income".into(),
                reason: "Must be non-negative".into(),
            });
        }
        if cfc.subsidiary_tax_paid < dec!(0) {
            return Err(CorpFinanceError::InvalidInput {
                field: "cfc_analysis.subsidiary_tax_paid".into(),
                reason: "Must be non-negative".into(),
            });
        }
        if cfc.passive_income_pct < dec!(0) || cfc.passive_income_pct > dec!(100) {
            return Err(CorpFinanceError::InvalidInput {
                field: "cfc_analysis.passive_income_pct".into(),
                reason: "Must be between 0 and 100".into(),
            });
        }
        if cfc.ownership_pct < dec!(0) || cfc.ownership_pct > dec!(100) {
            return Err(CorpFinanceError::InvalidInput {
                field: "cfc_analysis.ownership_pct".into(),
                reason: "Must be between 0 and 100".into(),
            });
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Statistical Helpers
// ---------------------------------------------------------------------------

/// Newton's method square root (20 iterations) for Decimal
fn decimal_sqrt(val: Decimal) -> Decimal {
    if val <= dec!(0) {
        return dec!(0);
    }
    let mut guess = val / dec!(2);
    if guess == dec!(0) {
        guess = dec!(1);
    }
    for _ in 0..20 {
        guess = (guess + val / guess) / dec!(2);
    }
    guess
}

/// Compute percentile using linear interpolation
fn percentile(sorted: &[Decimal], p: Decimal) -> Decimal {
    if sorted.is_empty() {
        return dec!(0);
    }
    if sorted.len() == 1 {
        return sorted[0];
    }
    let n = Decimal::from(sorted.len() as u32);
    let rank = p / dec!(100) * (n - dec!(1));

    // Integer and fractional parts
    let lower_idx = rank
        .floor()
        .to_string()
        .parse::<usize>()
        .unwrap_or(0)
        .min(sorted.len() - 1);
    let upper_idx = (lower_idx + 1).min(sorted.len() - 1);
    let fraction = rank - Decimal::from(lower_idx as u32);

    sorted[lower_idx] + fraction * (sorted[upper_idx] - sorted[lower_idx])
}

fn compute_comparable_range(comparables: &[Comparable]) -> ComparableRange {
    let mut margins: Vec<Decimal> = comparables.iter().map(|c| c.net_margin).collect();
    margins.sort();

    let count = margins.len() as u32;
    let sum: Decimal = margins.iter().copied().sum();
    let mean = if count > 0 {
        sum / Decimal::from(count)
    } else {
        dec!(0)
    };

    let p25 = percentile(&margins, dec!(25));
    let median = percentile(&margins, dec!(50));
    let p75 = percentile(&margins, dec!(75));

    // Standard deviation
    let variance = if count > 1 {
        let sum_sq: Decimal = margins.iter().map(|m| (*m - mean) * (*m - mean)).sum();
        sum_sq / Decimal::from(count - 1)
    } else {
        dec!(0)
    };
    let std_dev = decimal_sqrt(variance);

    ComparableRange {
        count,
        p25,
        median,
        p75,
        mean,
        std_dev,
    }
}

fn compute_tested_party_result(tp: &TestedParty) -> TestedPartyResult {
    let operating_margin = if tp.operating_revenue > dec!(0) {
        tp.operating_profit / tp.operating_revenue
    } else {
        dec!(0)
    };

    let gross_profit = tp.operating_revenue - tp.operating_costs + tp.operating_profit;
    let gross_margin = if tp.operating_revenue > dec!(0) {
        gross_profit / tp.operating_revenue
    } else {
        dec!(0)
    };

    // Berry ratio = gross profit / operating expenses
    let opex = tp.operating_costs;
    let berry_ratio = if opex > dec!(0) {
        gross_profit / opex
    } else {
        dec!(0)
    };

    let return_on_assets = if tp.assets > dec!(0) {
        tp.operating_profit / tp.assets
    } else {
        dec!(0)
    };

    TestedPartyResult {
        operating_margin,
        berry_ratio,
        return_on_assets,
        gross_margin,
    }
}

// ---------------------------------------------------------------------------
// Pricing Method Implementations
// ---------------------------------------------------------------------------

fn method_description(method: &str) -> String {
    match method {
        "CUP" => "Comparable Uncontrolled Price — direct comparison of controlled transaction price to uncontrolled comparable".into(),
        "RPM" => "Resale Price Method — resale price less appropriate gross margin from comparables".into(),
        "CPLM" => "Cost Plus Method — costs incurred plus appropriate markup from comparables".into(),
        "TNMM" => "Transactional Net Margin Method — net profit margin compared against comparable set PLI".into(),
        "ProfitSplit" => "Profit Split Method — residual allocation after routine returns".into(),
        _ => format!("Unknown method: {}", method),
    }
}

/// Compute the arm's length price based on the chosen method
fn compute_arm_length_price(
    method: &str,
    tp: &TestedParty,
    range: &ComparableRange,
    transaction_value: Decimal,
) -> Decimal {
    match method {
        "CUP" => {
            // Direct comparison: the median comparable price is the benchmark
            // For CUP, the arm's length price IS the transaction value adjusted
            // by the median margin differential. Use median net margin as
            // adjustment factor.
            if tp.operating_revenue > dec!(0) {
                transaction_value * (dec!(1) + range.median)
                    / (dec!(1) + tp.operating_profit / tp.operating_revenue)
            } else {
                transaction_value
            }
        }
        "RPM" => {
            // Resale Price Method: arm's length = resale price * (1 - median gross margin)
            // Use comparable median as the appropriate gross margin
            transaction_value * (dec!(1) - range.median)
        }
        "CPLM" => {
            // Cost Plus: arm's length = cost * (1 + median markup)
            // cost = operating_costs
            tp.operating_costs * (dec!(1) + range.median)
        }
        "TNMM" => {
            // TNMM: expected profit = revenue * median net margin
            // arm's length price = costs + expected profit
            if tp.operating_revenue > dec!(0) {
                let expected_profit = tp.operating_revenue * range.median;
                tp.operating_costs + expected_profit
            } else {
                transaction_value
            }
        }
        "ProfitSplit" => {
            // Residual profit split: allocate routine return (median margin),
            // then split residual. Simplified: arm's length share is
            // costs * (1 + median) for the routine portion.
            tp.operating_costs * (dec!(1) + range.median)
        }
        _ => transaction_value,
    }
}

// ---------------------------------------------------------------------------
// CFC Analysis
// ---------------------------------------------------------------------------

fn statutory_rate_for_cfc(jurisdiction: &str) -> Decimal {
    match jurisdiction {
        "US" => dec!(0.21),
        "UK" => dec!(0.25),
        "Germany" => dec!(0.2983),
        "France" => dec!(0.2571),
        "Ireland" => dec!(0.15),
        "Netherlands" => dec!(0.2569),
        "Luxembourg" => dec!(0.2494),
        "Switzerland" => dec!(0.1470),
        "Singapore" => dec!(0.17),
        "HongKong" => dec!(0.165),
        "Japan" => dec!(0.3062),
        "Australia" => dec!(0.30),
        "Canada" => dec!(0.265),
        "Cayman" | "BVI" | "Jersey" | "Guernsey" | "Bermuda" => dec!(0),
        _ => dec!(0.25),
    }
}

fn analyze_cfc(cfc: &CfcParams) -> CfcResult {
    let etr = if cfc.subsidiary_income > dec!(0) {
        cfc.subsidiary_tax_paid / cfc.subsidiary_income
    } else {
        dec!(0)
    };

    let parent_rate = statutory_rate_for_cfc(&cfc.parent_jurisdiction);
    let sub_rate = statutory_rate_for_cfc(&cfc.subsidiary_jurisdiction);

    // Determine CFC regime based on parent jurisdiction
    match cfc.parent_jurisdiction.as_str() {
        "US" => analyze_us_cfc(cfc, etr, parent_rate),
        "UK" => analyze_uk_cfc(cfc, etr, parent_rate, sub_rate),
        _ => analyze_eu_atad_cfc(cfc, etr, parent_rate, sub_rate),
    }
}

fn analyze_us_cfc(cfc: &CfcParams, etr: Decimal, parent_rate: Decimal) -> CfcResult {
    let mut exemptions = Vec::new();
    let mut inclusion_required = false;
    let mut inclusion_amount = dec!(0);

    // Subpart F: passive income inclusion
    let subpart_f_triggered =
        cfc.passive_income_pct > cfc.de_minimis_threshold && cfc.ownership_pct >= dec!(10);

    if subpart_f_triggered {
        let passive_income = cfc.subsidiary_income * cfc.passive_income_pct / dec!(100);
        inclusion_amount += passive_income * cfc.ownership_pct / dec!(100);
        inclusion_required = true;
    } else {
        if cfc.passive_income_pct <= cfc.de_minimis_threshold {
            exemptions
                .push("Subpart F de minimis exception — passive income below threshold".into());
        }
        if cfc.ownership_pct < dec!(10) {
            exemptions.push("Below 10% ownership threshold for CFC status".into());
        }
    }

    // GILTI: effective rate test (ETR < 90% of US rate)
    let gilti_threshold = parent_rate * dec!(0.90);
    let gilti_triggered = etr < gilti_threshold && cfc.ownership_pct >= dec!(10);

    if gilti_triggered {
        // GILTI inclusion = tested income - NDTIR (10% of QBAI, simplified)
        let tested_income = cfc.subsidiary_income * cfc.ownership_pct / dec!(100);
        // Simplified QBAI = 10% of income as deemed tangible return
        let qbai_return = tested_income * dec!(0.10);
        let gilti_amount = if tested_income > qbai_return {
            tested_income - qbai_return
        } else {
            dec!(0)
        };
        // Only add GILTI if not already covered by Subpart F
        if !subpart_f_triggered {
            inclusion_amount += gilti_amount;
            inclusion_required = true;
        }
        // GILTI 50% deduction (Section 250) + FTC (80% of foreign taxes)
        exemptions.push("GILTI Section 250 deduction (50%) may reduce effective rate".into());
    }

    // Tax liability at parent rate on inclusion
    let mut tax_liability = inclusion_amount * parent_rate;
    // Credit for foreign taxes paid (proportional)
    let ftc = if cfc.subsidiary_income > dec!(0) {
        cfc.subsidiary_tax_paid * inclusion_amount / cfc.subsidiary_income
    } else {
        dec!(0)
    };
    tax_liability = if tax_liability > ftc {
        tax_liability - ftc
    } else {
        dec!(0)
    };

    CfcResult {
        cfc_regime: "US Subpart F / GILTI".into(),
        inclusion_required,
        inclusion_amount,
        effective_tax_rate: etr,
        tax_liability,
        exemptions_available: exemptions,
    }
}

fn analyze_uk_cfc(
    cfc: &CfcParams,
    etr: Decimal,
    parent_rate: Decimal,
    sub_rate: Decimal,
) -> CfcResult {
    let mut exemptions = Vec::new();
    let mut inclusion_required = false;
    let mut inclusion_amount = dec!(0);
    let mut tax_liability = dec!(0);

    // UK CFC: gateway tests
    // (1) Low tax test: ETR < 75% of UK rate
    let low_tax_threshold = parent_rate * dec!(0.75);
    let low_tax = etr < low_tax_threshold;

    // (2) Significant people functions test (simplified: if passive > 50%)
    let spf_risk = cfc.passive_income_pct > dec!(50);

    if low_tax && spf_risk && cfc.ownership_pct >= dec!(25) {
        inclusion_required = true;
        inclusion_amount = cfc.subsidiary_income * cfc.ownership_pct / dec!(100);
        tax_liability = inclusion_amount * parent_rate;
        // Credit
        let ftc = if cfc.subsidiary_income > dec!(0) {
            cfc.subsidiary_tax_paid * inclusion_amount / cfc.subsidiary_income
        } else {
            dec!(0)
        };
        tax_liability = if tax_liability > ftc {
            tax_liability - ftc
        } else {
            dec!(0)
        };
    } else {
        if !low_tax {
            exemptions.push("Exempt — subsidiary ETR passes UK low-tax gateway test".into());
        }
        if !spf_risk {
            exemptions.push(
                "Exempt — significant people functions located in subsidiary jurisdiction".into(),
            );
        }
        if cfc.ownership_pct < dec!(25) {
            exemptions.push("Below 25% ownership threshold for UK CFC rules".into());
        }
    }

    // Excluded territories exemption
    if sub_rate >= parent_rate {
        exemptions.push("Excluded territories exemption — subsidiary rate >= parent rate".into());
    }

    CfcResult {
        cfc_regime: "UK CFC Rules (TIOPA 2010)".into(),
        inclusion_required,
        inclusion_amount,
        effective_tax_rate: etr,
        tax_liability,
        exemptions_available: exemptions,
    }
}

fn analyze_eu_atad_cfc(
    cfc: &CfcParams,
    etr: Decimal,
    parent_rate: Decimal,
    _sub_rate: Decimal,
) -> CfcResult {
    let mut exemptions = Vec::new();
    let mut inclusion_required = false;
    let mut inclusion_amount = dec!(0);
    let mut tax_liability = dec!(0);

    // EU ATAD CFC rules
    // (1) Control test: ownership > 50% (direct/indirect)
    let control = cfc.ownership_pct > dec!(50);

    // (2) Low tax test: ETR < 50% of parent jurisdiction rate
    let low_tax_threshold = parent_rate * dec!(0.50);
    let low_tax = etr < low_tax_threshold;

    if control && low_tax {
        // Include non-distributed passive income
        let passive_income = cfc.subsidiary_income * cfc.passive_income_pct / dec!(100);
        inclusion_amount = passive_income * cfc.ownership_pct / dec!(100);
        inclusion_required = true;

        tax_liability = inclusion_amount * parent_rate;
        let ftc = if cfc.subsidiary_income > dec!(0) {
            cfc.subsidiary_tax_paid * inclusion_amount / cfc.subsidiary_income
        } else {
            dec!(0)
        };
        tax_liability = if tax_liability > ftc {
            tax_liability - ftc
        } else {
            dec!(0)
        };
    } else {
        if !control {
            exemptions.push("No CFC control — ownership below 50%".into());
        }
        if !low_tax {
            exemptions.push("Exempt — ETR exceeds 50% of parent jurisdiction rate".into());
        }
    }

    // Substance exception
    if cfc.passive_income_pct < dec!(33) {
        exemptions.push(
            "Potential substance exception — majority of income is active/operational".into(),
        );
    }

    CfcResult {
        cfc_regime: format!("EU ATAD CFC (parent: {})", cfc.parent_jurisdiction),
        inclusion_required,
        inclusion_amount,
        effective_tax_rate: etr,
        tax_liability,
        exemptions_available: exemptions,
    }
}

// ---------------------------------------------------------------------------
// GAAR Assessment
// ---------------------------------------------------------------------------

fn assess_gaar(
    jurisdiction: &str,
    tp: &TestedParty,
    arm_length_result: &ArmLengthResult,
) -> GaarResult {
    let mut recommendations = Vec::new();
    let mut main_purpose_risk = false;
    let mut substance_adequate = true;

    // Main purpose test: if the entity has minimal economic substance
    // but large transaction volumes, flag risk
    let margin = if tp.operating_revenue > dec!(0) {
        tp.operating_profit / tp.operating_revenue
    } else {
        dec!(0)
    };

    if margin < dec!(0.01) && tp.operating_revenue > dec!(10000000) {
        main_purpose_risk = true;
        recommendations.push(
            "Very low margin on large revenue — may suggest arrangement lacks economic substance"
                .into(),
        );
    }

    // Economic substance
    if tp.assets < tp.operating_revenue * dec!(0.05) {
        substance_adequate = false;
        recommendations.push(
            "Assets are less than 5% of revenue — may indicate insufficient economic substance"
                .into(),
        );
    }

    // If outside arm's length range, flag
    if !arm_length_result.within_range {
        main_purpose_risk = true;
        recommendations.push(
            "Pricing is outside arm's length range — consider adjustment to mitigate GAAR risk"
                .into(),
        );
    }

    // Jurisdiction-specific guidance
    match jurisdiction {
        "UK" => {
            recommendations.push(
                "UK GAAR (FA 2013): ensure arrangement is not 'abusive' under the double reasonableness test".into(),
            );
        }
        "Germany" => {
            recommendations.push(
                "German AO Section 42: arrangement must have economic purpose beyond tax benefit"
                    .into(),
            );
        }
        "France" => {
            recommendations.push(
                "French abus de droit (LPF L64): ensure arrangement is not solely motivated by fiscal purpose".into(),
            );
        }
        "Australia" => {
            recommendations.push(
                "Part IVA ITAA 1936: dominant purpose test — ensure commercial rationale".into(),
            );
        }
        "Canada" => {
            recommendations.push(
                "GAAR (ITA Section 245): misuse/abuse test — document business purpose".into(),
            );
        }
        _ => {
            recommendations.push(format!(
                "Review {} local anti-avoidance provisions and document business purpose",
                jurisdiction,
            ));
        }
    }

    let risk_level = if main_purpose_risk && !substance_adequate {
        "High".into()
    } else if main_purpose_risk || !substance_adequate {
        "Medium".into()
    } else {
        "Low".into()
    };

    GaarResult {
        risk_level,
        main_purpose_risk,
        substance_adequate,
        recommendations,
    }
}

// ---------------------------------------------------------------------------
// Documentation Requirements
// ---------------------------------------------------------------------------

fn determine_documentation(method: &str, has_cfc: bool, jurisdiction: &str) -> Vec<String> {
    let mut docs = vec![
        "Master File (OECD BEPS Action 13) — group-wide overview of MNE business operations".into(),
        "Local File — detailed transfer pricing analysis for local entity".into(),
        format!(
            "Functional analysis (FAR) for tested party in {}",
            jurisdiction
        ),
        format!(
            "Benchmarking study using {} method with comparable set",
            method
        ),
    ];

    match method {
        "CUP" => {
            docs.push("Internal/external CUP documentation with comparability adjustments".into());
        }
        "RPM" => {
            docs.push("Resale price documentation with gross margin analysis".into());
        }
        "CPLM" => {
            docs.push("Cost base documentation and markup analysis".into());
        }
        "TNMM" => {
            docs.push("PLI selection rationale and comparable search methodology".into());
            docs.push("Database search documentation (e.g., Orbis/Compustat)".into());
        }
        "ProfitSplit" => {
            docs.push("Profit split allocation key documentation".into());
            docs.push("Contribution analysis for each party to the transaction".into());
        }
        _ => {}
    }

    if has_cfc {
        docs.push("CFC analysis documentation and income characterization".into());
    }

    docs.push("Intercompany agreement(s) governing the transaction".into());
    docs.push("Annual update of benchmarking study (recommended every 3 years minimum)".into());

    docs
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Analyze intercompany transfer pricing using the specified method, with
/// optional CFC analysis and GAAR assessment.
pub fn analyze_intercompany(input: &IntercompanyInput) -> CorpFinanceResult<IntercompanyOutput> {
    validate_intercompany_input(input)?;

    let mut warnings: Vec<String> = Vec::new();

    // -----------------------------------------------------------------------
    // Tested Party Result (PLI computation)
    // -----------------------------------------------------------------------
    let tested_party_result = compute_tested_party_result(&input.tested_party);

    // -----------------------------------------------------------------------
    // Comparable Range (IQR)
    // -----------------------------------------------------------------------
    let comparable_range = compute_comparable_range(&input.comparables);

    if comparable_range.count < 5 {
        warnings.push(format!(
            "Only {} comparables — OECD recommends a minimum of 5-10 for robust IQR",
            comparable_range.count,
        ));
    }

    // -----------------------------------------------------------------------
    // Arm's Length Assessment
    // -----------------------------------------------------------------------
    let tested_value = tested_party_result.operating_margin;
    let within_range = tested_value >= comparable_range.p25 && tested_value <= comparable_range.p75;

    let adjustment_needed = if !within_range {
        comparable_range.median - tested_value
    } else {
        dec!(0)
    };

    let adjustment_direction = if adjustment_needed > dec!(0) {
        "Increase".into()
    } else if adjustment_needed < dec!(0) {
        "Decrease".into()
    } else {
        "None".into()
    };

    // Confidence based on comparable count and spread
    let confidence = if comparable_range.count >= 10 && comparable_range.std_dev < dec!(0.05) {
        "High".into()
    } else if comparable_range.count >= 5 {
        "Medium".into()
    } else {
        "Low".into()
    };

    if !within_range {
        warnings.push(format!(
            "Tested party margin ({}) is outside IQR [{}, {}] — adjustment to median ({}) recommended",
            tested_value, comparable_range.p25, comparable_range.p75, comparable_range.median,
        ));
    }

    let arm_length_assessment = ArmLengthResult {
        within_range,
        tested_value,
        range_low: comparable_range.p25,
        range_high: comparable_range.p75,
        adjustment_needed,
        adjustment_direction,
        confidence,
    };

    // -----------------------------------------------------------------------
    // Pricing Analysis
    // -----------------------------------------------------------------------
    let arm_length_price = compute_arm_length_price(
        &input.pricing_method,
        &input.tested_party,
        &comparable_range,
        input.transaction_value,
    );

    let deviation_pct = if arm_length_price > dec!(0) {
        (input.transaction_value - arm_length_price) / arm_length_price * dec!(100)
    } else {
        dec!(0)
    };

    // Tax impact estimate: adjustment * statutory rate
    let rate = statutory_rate_for_cfc(&input.tested_party.jurisdiction);
    let price_diff = input.transaction_value - arm_length_price;
    let tax_impact = price_diff.abs() * rate;

    let pricing_analysis = PricingDetail {
        method_description: method_description(&input.pricing_method),
        arm_length_price,
        current_price: input.transaction_value,
        deviation_pct,
        tax_impact_estimate: tax_impact,
    };

    // -----------------------------------------------------------------------
    // CFC Analysis
    // -----------------------------------------------------------------------
    let cfc_analysis = input.cfc_analysis.as_ref().map(analyze_cfc);

    // -----------------------------------------------------------------------
    // GAAR Assessment
    // -----------------------------------------------------------------------
    let gaar_assessment = input
        .gaar_jurisdiction
        .as_ref()
        .map(|jur| assess_gaar(jur, &input.tested_party, &arm_length_assessment));

    // -----------------------------------------------------------------------
    // Documentation Requirements
    // -----------------------------------------------------------------------
    let documentation_requirements = determine_documentation(
        &input.pricing_method,
        input.cfc_analysis.is_some(),
        &input.tested_party.jurisdiction,
    );

    Ok(IntercompanyOutput {
        method_used: input.pricing_method.clone(),
        tested_party_result,
        comparable_range,
        arm_length_assessment,
        pricing_analysis,
        cfc_analysis,
        gaar_assessment,
        documentation_requirements,
        warnings,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn make_comparable(name: &str, margin: Decimal) -> Comparable {
        Comparable {
            name: name.into(),
            net_margin: margin,
            berry_ratio: None,
            return_on_assets: None,
            gross_margin: None,
        }
    }

    fn basic_comparables() -> Vec<Comparable> {
        vec![
            make_comparable("Comp A", dec!(0.04)),
            make_comparable("Comp B", dec!(0.06)),
            make_comparable("Comp C", dec!(0.08)),
            make_comparable("Comp D", dec!(0.05)),
            make_comparable("Comp E", dec!(0.07)),
            make_comparable("Comp F", dec!(0.09)),
            make_comparable("Comp G", dec!(0.03)),
        ]
    }

    fn basic_tested_party() -> TestedParty {
        TestedParty {
            name: "Test Sub".into(),
            jurisdiction: "Ireland".into(),
            function: "DistributionCE".into(),
            operating_revenue: dec!(100000000),
            operating_costs: dec!(94000000),
            operating_profit: dec!(6000000),
            assets: dec!(50000000),
        }
    }

    fn basic_input() -> IntercompanyInput {
        IntercompanyInput {
            transaction_name: "Distribution Agreement".into(),
            pricing_method: "TNMM".into(),
            tested_party: basic_tested_party(),
            comparables: basic_comparables(),
            transaction_value: dec!(80000000),
            cfc_analysis: None,
            gaar_jurisdiction: None,
        }
    }

    fn input_with_cfc() -> IntercompanyInput {
        let mut input = basic_input();
        input.cfc_analysis = Some(CfcParams {
            parent_jurisdiction: "US".into(),
            subsidiary_jurisdiction: "Ireland".into(),
            subsidiary_income: dec!(20000000),
            subsidiary_tax_paid: dec!(2500000),
            passive_income_pct: dec!(30),
            ownership_pct: dec!(100),
            de_minimis_threshold: dec!(5),
        });
        input
    }

    // --- Validation Tests ---

    #[test]
    fn test_invalid_method_rejected() {
        let mut input = basic_input();
        input.pricing_method = "INVALID".into();
        assert!(analyze_intercompany(&input).is_err());
    }

    #[test]
    fn test_negative_transaction_value_rejected() {
        let mut input = basic_input();
        input.transaction_value = dec!(-1);
        assert!(analyze_intercompany(&input).is_err());
    }

    #[test]
    fn test_negative_revenue_rejected() {
        let mut input = basic_input();
        input.tested_party.operating_revenue = dec!(-1);
        assert!(analyze_intercompany(&input).is_err());
    }

    #[test]
    fn test_negative_costs_rejected() {
        let mut input = basic_input();
        input.tested_party.operating_costs = dec!(-1);
        assert!(analyze_intercompany(&input).is_err());
    }

    #[test]
    fn test_negative_assets_rejected() {
        let mut input = basic_input();
        input.tested_party.assets = dec!(-1);
        assert!(analyze_intercompany(&input).is_err());
    }

    #[test]
    fn test_empty_comparables_rejected() {
        let mut input = basic_input();
        input.comparables.clear();
        assert!(analyze_intercompany(&input).is_err());
    }

    #[test]
    fn test_negative_cfc_income_rejected() {
        let mut input = input_with_cfc();
        input.cfc_analysis.as_mut().unwrap().subsidiary_income = dec!(-1);
        assert!(analyze_intercompany(&input).is_err());
    }

    #[test]
    fn test_negative_cfc_tax_rejected() {
        let mut input = input_with_cfc();
        input.cfc_analysis.as_mut().unwrap().subsidiary_tax_paid = dec!(-1);
        assert!(analyze_intercompany(&input).is_err());
    }

    #[test]
    fn test_passive_income_pct_over_100_rejected() {
        let mut input = input_with_cfc();
        input.cfc_analysis.as_mut().unwrap().passive_income_pct = dec!(101);
        assert!(analyze_intercompany(&input).is_err());
    }

    #[test]
    fn test_ownership_pct_over_100_rejected() {
        let mut input = input_with_cfc();
        input.cfc_analysis.as_mut().unwrap().ownership_pct = dec!(101);
        assert!(analyze_intercompany(&input).is_err());
    }

    // --- Tested Party Result Tests ---

    #[test]
    fn test_operating_margin_calculation() {
        let input = basic_input();
        let output = analyze_intercompany(&input).unwrap();
        // 6M / 100M = 0.06
        assert_eq!(output.tested_party_result.operating_margin, dec!(0.06));
    }

    #[test]
    fn test_return_on_assets_calculation() {
        let input = basic_input();
        let output = analyze_intercompany(&input).unwrap();
        // 6M / 50M = 0.12
        assert_eq!(output.tested_party_result.return_on_assets, dec!(0.12));
    }

    #[test]
    fn test_zero_revenue_margin() {
        let mut input = basic_input();
        input.tested_party.operating_revenue = dec!(0);
        input.tested_party.operating_profit = dec!(0);
        let output = analyze_intercompany(&input).unwrap();
        assert_eq!(output.tested_party_result.operating_margin, dec!(0));
    }

    #[test]
    fn test_zero_assets_roa() {
        let mut input = basic_input();
        input.tested_party.assets = dec!(0);
        let output = analyze_intercompany(&input).unwrap();
        assert_eq!(output.tested_party_result.return_on_assets, dec!(0));
    }

    // --- Comparable Range Tests ---

    #[test]
    fn test_comparable_count() {
        let input = basic_input();
        let output = analyze_intercompany(&input).unwrap();
        assert_eq!(output.comparable_range.count, 7);
    }

    #[test]
    fn test_comparable_median() {
        let input = basic_input();
        let output = analyze_intercompany(&input).unwrap();
        // Sorted margins: 0.03, 0.04, 0.05, 0.06, 0.07, 0.08, 0.09
        // Median (P50) = 0.06
        assert_eq!(output.comparable_range.median, dec!(0.06));
    }

    #[test]
    fn test_comparable_mean() {
        let input = basic_input();
        let output = analyze_intercompany(&input).unwrap();
        // Mean = (0.03+0.04+0.05+0.06+0.07+0.08+0.09)/7 = 0.42/7 = 0.06
        assert_eq!(output.comparable_range.mean, dec!(0.06));
    }

    #[test]
    fn test_comparable_p25_p75() {
        let input = basic_input();
        let output = analyze_intercompany(&input).unwrap();
        // With 7 values sorted: 0.03, 0.04, 0.05, 0.06, 0.07, 0.08, 0.09
        // P25: rank = 0.25 * 6 = 1.5 => 0.04 + 0.5*(0.05-0.04) = 0.045
        assert_eq!(output.comparable_range.p25, dec!(0.045));
        // P75: rank = 0.75 * 6 = 4.5 => 0.07 + 0.5*(0.08-0.07) = 0.075
        assert_eq!(output.comparable_range.p75, dec!(0.075));
    }

    #[test]
    fn test_comparable_std_dev_positive() {
        let input = basic_input();
        let output = analyze_intercompany(&input).unwrap();
        assert!(output.comparable_range.std_dev > dec!(0));
    }

    #[test]
    fn test_single_comparable_std_dev_zero() {
        let mut input = basic_input();
        input.comparables = vec![make_comparable("Only", dec!(0.05))];
        let output = analyze_intercompany(&input).unwrap();
        assert_eq!(output.comparable_range.std_dev, dec!(0));
    }

    // --- Arm's Length Assessment Tests ---

    #[test]
    fn test_within_arm_length_range() {
        let input = basic_input();
        let output = analyze_intercompany(&input).unwrap();
        // Tested margin 0.06, P25=0.045, P75=0.075 => within range
        assert!(output.arm_length_assessment.within_range);
        assert_eq!(output.arm_length_assessment.adjustment_needed, dec!(0));
    }

    #[test]
    fn test_outside_arm_length_range() {
        let mut input = basic_input();
        // Set tested party to very low margin
        input.tested_party.operating_profit = dec!(1000000); // 1% margin
        let output = analyze_intercompany(&input).unwrap();
        // 1% margin < P25 (4.5%) => outside range
        assert!(!output.arm_length_assessment.within_range);
        assert!(output.arm_length_assessment.adjustment_needed > dec!(0));
        assert_eq!(
            output.arm_length_assessment.adjustment_direction,
            "Increase"
        );
    }

    #[test]
    fn test_above_arm_length_range() {
        let mut input = basic_input();
        input.tested_party.operating_profit = dec!(15000000); // 15% margin
        let output = analyze_intercompany(&input).unwrap();
        assert!(!output.arm_length_assessment.within_range);
        assert_eq!(
            output.arm_length_assessment.adjustment_direction,
            "Decrease"
        );
    }

    #[test]
    fn test_confidence_low_few_comparables() {
        let mut input = basic_input();
        input.comparables = vec![
            make_comparable("A", dec!(0.05)),
            make_comparable("B", dec!(0.07)),
        ];
        let output = analyze_intercompany(&input).unwrap();
        assert_eq!(output.arm_length_assessment.confidence, "Low");
    }

    #[test]
    fn test_confidence_medium_adequate_comparables() {
        let input = basic_input();
        let output = analyze_intercompany(&input).unwrap();
        // 7 comparables with moderate spread => Medium
        assert_eq!(output.arm_length_assessment.confidence, "Medium");
    }

    // --- Pricing Method Tests ---

    #[test]
    fn test_tnmm_method_used() {
        let input = basic_input();
        let output = analyze_intercompany(&input).unwrap();
        assert_eq!(output.method_used, "TNMM");
    }

    #[test]
    fn test_cup_method() {
        let mut input = basic_input();
        input.pricing_method = "CUP".into();
        let output = analyze_intercompany(&input).unwrap();
        assert_eq!(output.method_used, "CUP");
        assert!(output
            .pricing_analysis
            .method_description
            .contains("Comparable Uncontrolled Price"));
    }

    #[test]
    fn test_rpm_method() {
        let mut input = basic_input();
        input.pricing_method = "RPM".into();
        let output = analyze_intercompany(&input).unwrap();
        assert_eq!(output.method_used, "RPM");
        // RPM: arm_length = tx_value * (1 - median_margin)
        // = 80M * (1 - 0.06) = 80M * 0.94 = 75.2M
        assert_eq!(output.pricing_analysis.arm_length_price, dec!(75200000));
    }

    #[test]
    fn test_cplm_method() {
        let mut input = basic_input();
        input.pricing_method = "CPLM".into();
        let output = analyze_intercompany(&input).unwrap();
        assert_eq!(output.method_used, "CPLM");
        // CPLM: arm_length = costs * (1 + median)
        // = 94M * (1 + 0.06) = 94M * 1.06 = 99.64M
        assert_eq!(output.pricing_analysis.arm_length_price, dec!(99640000));
    }

    #[test]
    fn test_profit_split_method() {
        let mut input = basic_input();
        input.pricing_method = "ProfitSplit".into();
        let output = analyze_intercompany(&input).unwrap();
        assert_eq!(output.method_used, "ProfitSplit");
        assert!(output
            .pricing_analysis
            .method_description
            .contains("Profit Split"));
    }

    #[test]
    fn test_deviation_pct_computed() {
        let mut input = basic_input();
        input.pricing_method = "RPM".into();
        let output = analyze_intercompany(&input).unwrap();
        // current 80M, arm's length 75.2M
        // deviation = (80 - 75.2) / 75.2 * 100 ~ 6.38%
        assert!(output.pricing_analysis.deviation_pct > dec!(6));
        assert!(output.pricing_analysis.deviation_pct < dec!(7));
    }

    #[test]
    fn test_tax_impact_estimate() {
        let mut input = basic_input();
        input.pricing_method = "RPM".into();
        let output = analyze_intercompany(&input).unwrap();
        // Ireland rate = 15%, diff = |80M - 75.2M| = 4.8M, impact = 4.8M * 0.15 = 0.72M
        assert_eq!(output.pricing_analysis.tax_impact_estimate, dec!(720000));
    }

    // --- CFC Analysis Tests ---

    #[test]
    fn test_cfc_none_when_not_provided() {
        let input = basic_input();
        let output = analyze_intercompany(&input).unwrap();
        assert!(output.cfc_analysis.is_none());
    }

    #[test]
    fn test_us_cfc_subpart_f_triggered() {
        let input = input_with_cfc();
        let output = analyze_intercompany(&input).unwrap();
        let cfc = output.cfc_analysis.unwrap();
        assert_eq!(cfc.cfc_regime, "US Subpart F / GILTI");
        // 30% passive > 5% threshold, 100% ownership > 10%
        assert!(cfc.inclusion_required);
        assert!(cfc.inclusion_amount > dec!(0));
    }

    #[test]
    fn test_us_cfc_etr() {
        let input = input_with_cfc();
        let output = analyze_intercompany(&input).unwrap();
        let cfc = output.cfc_analysis.unwrap();
        // 2.5M / 20M = 0.125
        assert_eq!(cfc.effective_tax_rate, dec!(0.125));
    }

    #[test]
    fn test_us_cfc_de_minimis_exception() {
        let mut input = input_with_cfc();
        // Set passive below threshold
        input.cfc_analysis.as_mut().unwrap().passive_income_pct = dec!(3);
        let output = analyze_intercompany(&input).unwrap();
        let cfc = output.cfc_analysis.unwrap();
        // Subpart F not triggered due to de minimis, but GILTI may still apply
        let has_de_minimis = cfc
            .exemptions_available
            .iter()
            .any(|e| e.contains("de minimis"));
        assert!(has_de_minimis);
    }

    #[test]
    fn test_us_cfc_below_ownership_threshold() {
        let mut input = input_with_cfc();
        input.cfc_analysis.as_mut().unwrap().ownership_pct = dec!(5);
        let output = analyze_intercompany(&input).unwrap();
        let cfc = output.cfc_analysis.unwrap();
        assert!(!cfc.inclusion_required);
    }

    #[test]
    fn test_uk_cfc_analysis() {
        let mut input = basic_input();
        input.cfc_analysis = Some(CfcParams {
            parent_jurisdiction: "UK".into(),
            subsidiary_jurisdiction: "Cayman".into(),
            subsidiary_income: dec!(10000000),
            subsidiary_tax_paid: dec!(0),
            passive_income_pct: dec!(60),
            ownership_pct: dec!(100),
            de_minimis_threshold: dec!(5),
        });
        let output = analyze_intercompany(&input).unwrap();
        let cfc = output.cfc_analysis.unwrap();
        assert!(cfc.cfc_regime.contains("UK"));
        // 0% ETR < 75% of 25% = 18.75% => low tax triggered
        // 60% passive > 50% => SPF risk
        // 100% ownership > 25%
        assert!(cfc.inclusion_required);
    }

    #[test]
    fn test_eu_atad_cfc_analysis() {
        let mut input = basic_input();
        input.cfc_analysis = Some(CfcParams {
            parent_jurisdiction: "Germany".into(),
            subsidiary_jurisdiction: "Cayman".into(),
            subsidiary_income: dec!(5000000),
            subsidiary_tax_paid: dec!(0),
            passive_income_pct: dec!(80),
            ownership_pct: dec!(75),
            de_minimis_threshold: dec!(5),
        });
        let output = analyze_intercompany(&input).unwrap();
        let cfc = output.cfc_analysis.unwrap();
        assert!(cfc.cfc_regime.contains("EU ATAD"));
        // 0% ETR < 50% of 29.83% => low tax
        // 75% ownership > 50% => control
        assert!(cfc.inclusion_required);
    }

    #[test]
    fn test_eu_atad_no_control() {
        let mut input = basic_input();
        input.cfc_analysis = Some(CfcParams {
            parent_jurisdiction: "France".into(),
            subsidiary_jurisdiction: "Ireland".into(),
            subsidiary_income: dec!(10000000),
            subsidiary_tax_paid: dec!(1500000),
            passive_income_pct: dec!(40),
            ownership_pct: dec!(30),
            de_minimis_threshold: dec!(5),
        });
        let output = analyze_intercompany(&input).unwrap();
        let cfc = output.cfc_analysis.unwrap();
        // 30% ownership < 50% => no control
        assert!(!cfc.inclusion_required);
        let has_no_control = cfc
            .exemptions_available
            .iter()
            .any(|e| e.contains("below 50%"));
        assert!(has_no_control);
    }

    #[test]
    fn test_cfc_tax_liability_nonneg() {
        let input = input_with_cfc();
        let output = analyze_intercompany(&input).unwrap();
        let cfc = output.cfc_analysis.unwrap();
        assert!(cfc.tax_liability >= dec!(0));
    }

    // --- GAAR Tests ---

    #[test]
    fn test_gaar_none_when_not_provided() {
        let input = basic_input();
        let output = analyze_intercompany(&input).unwrap();
        assert!(output.gaar_assessment.is_none());
    }

    #[test]
    fn test_gaar_uk_assessment() {
        let mut input = basic_input();
        input.gaar_jurisdiction = Some("UK".into());
        let output = analyze_intercompany(&input).unwrap();
        let gaar = output.gaar_assessment.unwrap();
        assert!(!gaar.recommendations.is_empty());
        let has_uk_ref = gaar.recommendations.iter().any(|r| r.contains("UK GAAR"));
        assert!(has_uk_ref);
    }

    #[test]
    fn test_gaar_germany_assessment() {
        let mut input = basic_input();
        input.gaar_jurisdiction = Some("Germany".into());
        let output = analyze_intercompany(&input).unwrap();
        let gaar = output.gaar_assessment.unwrap();
        let has_de_ref = gaar.recommendations.iter().any(|r| r.contains("German AO"));
        assert!(has_de_ref);
    }

    #[test]
    fn test_gaar_low_risk_with_substance() {
        let mut input = basic_input();
        input.gaar_jurisdiction = Some("US".into());
        let output = analyze_intercompany(&input).unwrap();
        let gaar = output.gaar_assessment.unwrap();
        // Within arm's length, adequate margin and assets
        assert_eq!(gaar.risk_level, "Low");
        assert!(gaar.substance_adequate);
    }

    #[test]
    fn test_gaar_elevated_risk_outside_range() {
        let mut input = basic_input();
        input.tested_party.operating_profit = dec!(500000); // 0.5% margin
        input.gaar_jurisdiction = Some("UK".into());
        let output = analyze_intercompany(&input).unwrap();
        let gaar = output.gaar_assessment.unwrap();
        // Low margin on 100M revenue => main purpose risk
        assert!(gaar.main_purpose_risk);
    }

    #[test]
    fn test_gaar_low_assets_substance() {
        let mut input = basic_input();
        input.tested_party.assets = dec!(1000000); // 1% of revenue
        input.gaar_jurisdiction = Some("Australia".into());
        let output = analyze_intercompany(&input).unwrap();
        let gaar = output.gaar_assessment.unwrap();
        assert!(!gaar.substance_adequate);
    }

    // --- Documentation Requirements Tests ---

    #[test]
    fn test_documentation_includes_master_file() {
        let input = basic_input();
        let output = analyze_intercompany(&input).unwrap();
        let has_master = output
            .documentation_requirements
            .iter()
            .any(|d| d.contains("Master File"));
        assert!(has_master);
    }

    #[test]
    fn test_documentation_includes_local_file() {
        let input = basic_input();
        let output = analyze_intercompany(&input).unwrap();
        let has_local = output
            .documentation_requirements
            .iter()
            .any(|d| d.contains("Local File"));
        assert!(has_local);
    }

    #[test]
    fn test_documentation_tnmm_specific() {
        let input = basic_input();
        let output = analyze_intercompany(&input).unwrap();
        let has_pli = output
            .documentation_requirements
            .iter()
            .any(|d| d.contains("PLI"));
        assert!(has_pli);
    }

    #[test]
    fn test_documentation_includes_cfc_when_present() {
        let input = input_with_cfc();
        let output = analyze_intercompany(&input).unwrap();
        let has_cfc_doc = output
            .documentation_requirements
            .iter()
            .any(|d| d.contains("CFC"));
        assert!(has_cfc_doc);
    }

    // --- Warning Tests ---

    #[test]
    fn test_few_comparables_warning() {
        let mut input = basic_input();
        input.comparables = vec![
            make_comparable("A", dec!(0.05)),
            make_comparable("B", dec!(0.07)),
        ];
        let output = analyze_intercompany(&input).unwrap();
        let has_warning = output.warnings.iter().any(|w| w.contains("comparables"));
        assert!(has_warning);
    }

    #[test]
    fn test_outside_range_warning() {
        let mut input = basic_input();
        input.tested_party.operating_profit = dec!(1000000); // 1%
        let output = analyze_intercompany(&input).unwrap();
        let has_warning = output.warnings.iter().any(|w| w.contains("outside IQR"));
        assert!(has_warning);
    }

    // --- Edge Cases ---

    #[test]
    fn test_all_methods_valid() {
        for method in VALID_METHODS {
            let mut input = basic_input();
            input.pricing_method = method.to_string();
            let result = analyze_intercompany(&input);
            assert!(result.is_ok(), "Method {} should be valid", method);
        }
    }

    #[test]
    fn test_zero_transaction_value() {
        let mut input = basic_input();
        input.transaction_value = dec!(0);
        let output = analyze_intercompany(&input).unwrap();
        assert_eq!(output.pricing_analysis.current_price, dec!(0));
    }

    #[test]
    fn test_single_comparable() {
        let mut input = basic_input();
        input.comparables = vec![make_comparable("Only", dec!(0.05))];
        let output = analyze_intercompany(&input).unwrap();
        assert_eq!(output.comparable_range.median, dec!(0.05));
        assert_eq!(output.comparable_range.p25, dec!(0.05));
        assert_eq!(output.comparable_range.p75, dec!(0.05));
    }

    // --- Statistical Helper Tests ---

    #[test]
    fn test_decimal_sqrt_perfect_square() {
        let result = decimal_sqrt(dec!(4));
        // Should be very close to 2
        assert!(result > dec!(1.999999));
        assert!(result < dec!(2.000001));
    }

    #[test]
    fn test_decimal_sqrt_zero() {
        assert_eq!(decimal_sqrt(dec!(0)), dec!(0));
    }

    #[test]
    fn test_decimal_sqrt_negative() {
        assert_eq!(decimal_sqrt(dec!(-1)), dec!(0));
    }

    #[test]
    fn test_percentile_empty() {
        assert_eq!(percentile(&[], dec!(50)), dec!(0));
    }

    #[test]
    fn test_percentile_single() {
        assert_eq!(percentile(&[dec!(5)], dec!(50)), dec!(5));
    }

    #[test]
    fn test_percentile_even_count() {
        let sorted = vec![dec!(1), dec!(2), dec!(3), dec!(4)];
        let median = percentile(&sorted, dec!(50));
        // rank = 0.5 * 3 = 1.5 => 2 + 0.5 * (3-2) = 2.5
        assert_eq!(median, dec!(2.5));
    }

    #[test]
    fn test_berry_ratio_computation() {
        let tp = TestedParty {
            name: "Test".into(),
            jurisdiction: "US".into(),
            function: "ServicesCE".into(),
            operating_revenue: dec!(10000000),
            operating_costs: dec!(8000000),
            operating_profit: dec!(2000000),
            assets: dec!(5000000),
        };
        let result = compute_tested_party_result(&tp);
        // gross_profit = 10M - 8M + 2M = 4M; berry = 4M / 8M = 0.5
        assert_eq!(result.berry_ratio, dec!(0.5));
    }

    #[test]
    fn test_zero_costs_berry_ratio() {
        let tp = TestedParty {
            name: "Test".into(),
            jurisdiction: "US".into(),
            function: "Holding".into(),
            operating_revenue: dec!(10000000),
            operating_costs: dec!(0),
            operating_profit: dec!(10000000),
            assets: dec!(50000000),
        };
        let result = compute_tested_party_result(&tp);
        assert_eq!(result.berry_ratio, dec!(0));
    }
}
