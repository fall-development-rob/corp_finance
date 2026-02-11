use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::{CorpFinanceError, CorpFinanceResult};

use super::us_funds::InvestorType;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UkEuFundInput {
    pub fund_name: String,
    /// "UKLP", "UKLLP", "OEIC", "ACS", "SICAV", "FCP", "KG"
    pub structure_type: String,
    /// "UK", "France", "Germany", "Netherlands"
    pub domicile: String,
    pub fund_size: Decimal,
    pub management_fee_rate: Decimal,
    pub carried_interest_rate: Decimal,
    pub preferred_return: Decimal,
    pub fund_term_years: u32,
    pub investor_types: Vec<InvestorType>,
    pub expected_annual_return: Decimal,
    pub aifmd_compliant: bool,
    pub ucits_compliant: bool,
    /// Local VAT on management fees (e.g. 0.20 for UK)
    pub vat_rate: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundEconomicsEu {
    pub management_fees_annual: Decimal,
    pub carried_interest_potential: Decimal,
    pub gp_return: Decimal,
    pub net_return_to_lps: Decimal,
    pub total_fund_expenses: Decimal,
    pub vat_cost_annual: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceTestEu {
    pub test_name: String,
    pub required_threshold: String,
    pub assumed_value: String,
    pub passes: bool,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxAnalysisEu {
    pub carried_interest_tax_rate: Decimal,
    pub fund_level_tax: Decimal,
    pub reporting_fund_status: String,
    pub structure_specific_tests: Vec<ComplianceTestEu>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AifmdAnalysis {
    pub passport_eligible: bool,
    pub leverage_ratio_commitment: Decimal,
    pub leverage_ratio_gross: Decimal,
    pub depositary_required: bool,
    pub aifm_capital_required: Decimal,
    pub remuneration_compliant: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VatAnalysis {
    pub management_fee_vat_exempt: bool,
    pub irrecoverable_vat: Decimal,
    pub effective_cost_increase: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketingAnalysis {
    pub passport_jurisdictions: Vec<String>,
    pub nppr_jurisdictions: Vec<String>,
    pub reverse_solicitation_risk: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UkEuFundOutput {
    pub structure_type: String,
    pub domicile: String,
    pub fund_economics: FundEconomicsEu,
    pub tax_analysis: TaxAnalysisEu,
    pub aifmd_analysis: AifmdAnalysis,
    pub vat_analysis: VatAnalysis,
    pub marketing_analysis: MarketingAnalysis,
    pub recommendations: Vec<String>,
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

const VALID_STRUCTURES: &[&str] = &["UKLP", "UKLLP", "OEIC", "ACS", "SICAV", "FCP", "KG"];
const VALID_DOMICILES: &[&str] = &["UK", "France", "Germany", "Netherlands"];
const VALID_CATEGORIES: &[&str] = &["TaxExempt", "Taxable", "Foreign", "ERISA"];

fn validate_input(input: &UkEuFundInput) -> CorpFinanceResult<()> {
    if input.fund_size <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "fund_size".into(),
            reason: "must be positive".into(),
        });
    }
    if input.management_fee_rate < Decimal::ZERO || input.management_fee_rate > dec!(0.10) {
        return Err(CorpFinanceError::InvalidInput {
            field: "management_fee_rate".into(),
            reason: "must be in [0, 0.10]".into(),
        });
    }
    if input.carried_interest_rate < Decimal::ZERO || input.carried_interest_rate > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "carried_interest_rate".into(),
            reason: "must be in [0, 1]".into(),
        });
    }
    if input.preferred_return < Decimal::ZERO || input.preferred_return > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "preferred_return".into(),
            reason: "must be in [0, 1]".into(),
        });
    }
    if input.fund_term_years == 0 || input.fund_term_years > 30 {
        return Err(CorpFinanceError::InvalidInput {
            field: "fund_term_years".into(),
            reason: "must be in [1, 30]".into(),
        });
    }
    if !VALID_STRUCTURES.contains(&input.structure_type.as_str()) {
        return Err(CorpFinanceError::InvalidInput {
            field: "structure_type".into(),
            reason: format!("must be one of: {}", VALID_STRUCTURES.join(", ")),
        });
    }
    if !VALID_DOMICILES.contains(&input.domicile.as_str()) {
        return Err(CorpFinanceError::InvalidInput {
            field: "domicile".into(),
            reason: format!("must be one of: {}", VALID_DOMICILES.join(", ")),
        });
    }
    if input.vat_rate < Decimal::ZERO || input.vat_rate > dec!(0.30) {
        return Err(CorpFinanceError::InvalidInput {
            field: "vat_rate".into(),
            reason: "must be in [0, 0.30]".into(),
        });
    }
    if input.expected_annual_return < dec!(-1) || input.expected_annual_return > dec!(1) {
        return Err(CorpFinanceError::InvalidInput {
            field: "expected_annual_return".into(),
            reason: "must be in [-1, 1]".into(),
        });
    }
    if input.investor_types.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "investor_types".into(),
            reason: "must have at least one investor type".into(),
        });
    }
    let mut alloc_sum = Decimal::ZERO;
    for inv in &input.investor_types {
        if !VALID_CATEGORIES.contains(&inv.category.as_str()) {
            return Err(CorpFinanceError::InvalidInput {
                field: "investor_types.category".into(),
                reason: format!(
                    "'{}' is not valid; must be one of: {}",
                    inv.category,
                    VALID_CATEGORIES.join(", ")
                ),
            });
        }
        if inv.allocation_pct < Decimal::ZERO || inv.allocation_pct > Decimal::ONE {
            return Err(CorpFinanceError::InvalidInput {
                field: "investor_types.allocation_pct".into(),
                reason: "must be in [0, 1]".into(),
            });
        }
        alloc_sum += inv.allocation_pct;
    }
    let alloc_diff = if alloc_sum > Decimal::ONE {
        alloc_sum - Decimal::ONE
    } else {
        Decimal::ONE - alloc_sum
    };
    if alloc_diff > dec!(0.01) {
        return Err(CorpFinanceError::InvalidInput {
            field: "investor_types.allocation_pct".into(),
            reason: format!("allocations sum to {} but must sum to ~1.0", alloc_sum),
        });
    }

    // Structure/domicile compatibility
    let st = input.structure_type.as_str();
    let dom = input.domicile.as_str();
    match (st, dom) {
        ("UKLP" | "UKLLP" | "OEIC" | "ACS", "UK") => {}
        ("SICAV", "France" | "Netherlands") => {}
        ("FCP", "France") => {}
        ("KG", "Germany") => {}
        ("SICAV", "UK" | "Germany") | ("FCP", "UK" | "Germany" | "Netherlands") => {
            return Err(CorpFinanceError::InvalidInput {
                field: "structure_type/domicile".into(),
                reason: format!("structure '{}' is not available in domicile '{}'", st, dom),
            });
        }
        ("UKLP" | "UKLLP" | "OEIC" | "ACS", _) => {
            return Err(CorpFinanceError::InvalidInput {
                field: "structure_type/domicile".into(),
                reason: format!(
                    "UK structure '{}' requires domicile 'UK', got '{}'",
                    st, dom
                ),
            });
        }
        ("KG", _) => {
            return Err(CorpFinanceError::InvalidInput {
                field: "structure_type/domicile".into(),
                reason: format!("KG structure requires domicile 'Germany', got '{}'", dom),
            });
        }
        _ => {}
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Iterative compound: (1 + r)^n using multiplication.
fn compound(rate: Decimal, years: u32) -> Decimal {
    let mut result = Decimal::ONE;
    let factor = Decimal::ONE + rate;
    for _ in 0..years {
        result *= factor;
    }
    result
}

fn compute_fund_economics(input: &UkEuFundInput) -> FundEconomicsEu {
    let mgmt_fee_annual = input.fund_size * input.management_fee_rate;
    let term = Decimal::from(input.fund_term_years);

    // VAT on management fees (depends on structure)
    let vat_exempt = is_management_fee_vat_exempt(input);
    let vat_cost_annual = if vat_exempt {
        Decimal::ZERO
    } else {
        mgmt_fee_annual * input.vat_rate
    };

    // Total fund value at end
    let total_value =
        input.fund_size * compound(input.expected_annual_return, input.fund_term_years);
    let total_profit = if total_value > input.fund_size {
        total_value - input.fund_size
    } else {
        Decimal::ZERO
    };

    // Preferred return hurdle
    let hurdle_value = input.fund_size * compound(input.preferred_return, input.fund_term_years);
    let excess_above_hurdle = if total_value > hurdle_value {
        total_value - hurdle_value
    } else {
        Decimal::ZERO
    };

    let carried_interest = excess_above_hurdle * input.carried_interest_rate;

    // GP return (assume 2% GP commitment for EU/UK standard)
    let gp_commitment_pct = dec!(0.02);
    let gp_commitment = input.fund_size * gp_commitment_pct;
    let gp_return = gp_commitment * compound(input.expected_annual_return, input.fund_term_years)
        - gp_commitment;

    // Fund expenses (0.5% admin + management fees + VAT)
    let annual_expenses = input.fund_size * dec!(0.005);
    let total_expenses = (annual_expenses + mgmt_fee_annual + vat_cost_annual) * term;

    // Net return to LPs
    let lp_capital = input.fund_size * (Decimal::ONE - gp_commitment_pct);
    let lp_share_of_profit = total_profit - carried_interest - gp_return;
    let net_return_to_lps = if lp_capital > Decimal::ZERO {
        lp_share_of_profit / lp_capital
    } else {
        Decimal::ZERO
    };

    FundEconomicsEu {
        management_fees_annual: mgmt_fee_annual,
        carried_interest_potential: carried_interest,
        gp_return,
        net_return_to_lps,
        total_fund_expenses: total_expenses,
        vat_cost_annual,
    }
}

fn is_management_fee_vat_exempt(input: &UkEuFundInput) -> bool {
    // UCITS funds: management fees exempt from VAT in most EU jurisdictions
    if input.ucits_compliant {
        return true;
    }
    // AIF: VAT treatment varies by jurisdiction
    match (input.structure_type.as_str(), input.domicile.as_str()) {
        ("OEIC" | "ACS", "UK") => true, // UK: exempt for authorized funds
        ("SICAV", "France" | "Netherlands") => true, // Generally exempt
        ("FCP", "France") => true,      // Transparent, exempt
        ("KG", "Germany") => false,     // Management fees generally subject to VAT
        ("UKLP" | "UKLLP", "UK") => false, // LP management fees not exempt
        _ => false,
    }
}

fn compute_tax_analysis(input: &UkEuFundInput) -> TaxAnalysisEu {
    let st = input.structure_type.as_str();
    let dom = input.domicile.as_str();

    // Carried interest tax rate
    let carry_tax_rate = match (st, dom) {
        ("UKLP" | "UKLLP", "UK") => dec!(0.28), // UK CGT 28% for carried interest (if > 3yr hold)
        ("OEIC" | "ACS", "UK") => dec!(0.20),   // Corp tax on fund / CGT for individuals
        ("SICAV", _) => dec!(0.0),              // Luxembourg SICAV: no income/capital gains tax
        ("FCP", "France") => dec!(0.30),        // Flat tax (PFU) 30% for French investors
        ("KG", "Germany") => dec!(0.2638),      // Abgeltungsteuer 26.375% + solidarity surcharge
        _ => dec!(0.25),
    };

    // Fund-level tax
    let fund_level_tax = match (st, dom) {
        ("UKLP" | "UKLLP", "UK") => Decimal::ZERO, // Transparent
        ("OEIC", "UK") => Decimal::ZERO,           // Exempt from CGT
        ("ACS", "UK") => Decimal::ZERO,            // Co-ownership scheme, transparent
        ("SICAV", _) => dec!(0.0005) * input.fund_size, // Taxe d'abonnement: 5bps for institutional
        ("FCP", "France") => Decimal::ZERO,        // Transparent
        ("KG", "Germany") => Decimal::ZERO,        // Transparent for income tax
        _ => Decimal::ZERO,
    };

    // Reporting fund status
    let reporting_status = match (st, dom) {
        ("OEIC" | "ACS", "UK") => "Authorized UK fund — automatic reporting fund status".into(),
        ("UKLP" | "UKLLP", "UK") => "Transparent — no reporting fund status needed".into(),
        ("SICAV", _) => "Must elect UK reporting fund status for UK investors".into(),
        ("FCP", "France") => "Transparent — reporting per investor domicile rules".into(),
        ("KG", "Germany") => "Transparent — reporting per investor domicile rules".into(),
        _ => "Reporting fund status depends on investor domicile".into(),
    };

    let tests = compute_compliance_tests_eu(input);

    TaxAnalysisEu {
        carried_interest_tax_rate: carry_tax_rate,
        fund_level_tax,
        reporting_fund_status: reporting_status,
        structure_specific_tests: tests,
    }
}

fn compute_compliance_tests_eu(input: &UkEuFundInput) -> Vec<ComplianceTestEu> {
    let st = input.structure_type.as_str();
    let mut tests = Vec::new();

    match st {
        "UKLP" | "UKLLP" => {
            tests.push(ComplianceTestEu {
                test_name: "Carried Interest 3-Year Hold".into(),
                required_threshold: ">=3 year average holding period".into(),
                assumed_value: format!("{} years fund term", input.fund_term_years),
                passes: input.fund_term_years >= 3,
                description: "Carried interest qualifies for 28% CGT rate only if average holding period >= 3 years".into(),
            });
            tests.push(ComplianceTestEu {
                test_name: "Income-Linked Carry Test".into(),
                required_threshold: "Carry linked to fund performance".into(),
                assumed_value: "Assumed compliant".into(),
                passes: true,
                description: "Carried interest must be genuinely linked to fund performance, not disguised management fees".into(),
            });
        }
        "OEIC" => {
            tests.push(ComplianceTestEu {
                test_name: "FCA Authorization".into(),
                required_threshold: "FCA authorized".into(),
                assumed_value: "Authorized".into(),
                passes: true,
                description: "OEIC must be authorized by the FCA".into(),
            });
            tests.push(ComplianceTestEu {
                test_name: "COLL Compliance".into(),
                required_threshold: "FCA COLL sourcebook compliant".into(),
                assumed_value: "Compliant".into(),
                passes: true,
                description: "Must comply with FCA Collective Investment Schemes sourcebook".into(),
            });
        }
        "ACS" => {
            tests.push(ComplianceTestEu {
                test_name: "Co-Ownership Structure".into(),
                required_threshold: "Valid co-ownership deed".into(),
                assumed_value: "In place".into(),
                passes: true,
                description: "ACS must operate as a contractual co-ownership scheme".into(),
            });
            tests.push(ComplianceTestEu {
                test_name: "Tax Transparency".into(),
                required_threshold: "Transparent for income tax".into(),
                assumed_value: "Transparent".into(),
                passes: true,
                description: "ACS is treated as transparent for income tax purposes".into(),
            });
        }
        "SICAV" => {
            tests.push(ComplianceTestEu {
                test_name: "Taxe d'Abonnement".into(),
                required_threshold: "5bps for institutional, 1bp for money market".into(),
                assumed_value: "5bps institutional rate".into(),
                passes: true,
                description:
                    "Luxembourg subscription tax — 0.05% per annum for institutional shares".into(),
            });
            tests.push(ComplianceTestEu {
                test_name: "CSSF Registration".into(),
                required_threshold: "CSSF registered and supervised".into(),
                assumed_value: "Registered".into(),
                passes: true,
                description: "Must be registered with Luxembourg CSSF".into(),
            });
        }
        "FCP" => {
            tests.push(ComplianceTestEu {
                test_name: "AMF Authorization".into(),
                required_threshold: "AMF authorized".into(),
                assumed_value: "Authorized".into(),
                passes: true,
                description: "French FCP must be authorized by the AMF".into(),
            });
            tests.push(ComplianceTestEu {
                test_name: "Tax Transparency".into(),
                required_threshold: "Transparent for French tax".into(),
                assumed_value: "Transparent".into(),
                passes: true,
                description: "FCP is a contractual fund, transparent for French tax purposes"
                    .into(),
            });
        }
        "KG" => {
            tests.push(ComplianceTestEu {
                test_name: "Trade Tax Exemption".into(),
                required_threshold: "Pure asset management (Vermoegensverwaltung)".into(),
                assumed_value: "Asset management assumed".into(),
                passes: true,
                description: "KG may be subject to Gewerbesteuer (trade tax) if deemed a commercial enterprise".into(),
            });
            tests.push(ComplianceTestEu {
                test_name: "BaFin Registration".into(),
                required_threshold: "BaFin registered if AIFMD applies".into(),
                assumed_value: if input.aifmd_compliant {
                    "Registered"
                } else {
                    "Not registered"
                }
                .into(),
                passes: input.aifmd_compliant,
                description:
                    "German KG fund must be registered with BaFin under KAGB if AIFMD applies"
                        .into(),
            });
        }
        _ => {}
    }

    tests
}

fn compute_aifmd_analysis(input: &UkEuFundInput) -> AifmdAnalysis {
    // AIFM capital requirements:
    // Base: EUR 125,000
    // Additional: 0.02% of AUM above EUR 250,000,000
    // Maximum total: EUR 10,000,000
    let base_capital = dec!(125_000);
    let aum_threshold = dec!(250_000_000);
    let additional = if input.fund_size > aum_threshold {
        (input.fund_size - aum_threshold) * dec!(0.0002)
    } else {
        Decimal::ZERO
    };
    let aifm_capital = (base_capital + additional).min(dec!(10_000_000));

    // Passport eligibility
    let passport_eligible = input.aifmd_compliant
        && matches!(
            input.domicile.as_str(),
            "UK" | "France" | "Germany" | "Netherlands"
        );

    // Leverage limits (commitment method typically 2x, gross typically 3x for unleveraged)
    let leverage_commitment = dec!(2.0);
    let leverage_gross = dec!(3.0);

    AifmdAnalysis {
        passport_eligible,
        leverage_ratio_commitment: leverage_commitment,
        leverage_ratio_gross: leverage_gross,
        depositary_required: input.aifmd_compliant,
        aifm_capital_required: aifm_capital,
        remuneration_compliant: input.aifmd_compliant,
    }
}

fn compute_vat_analysis(input: &UkEuFundInput) -> VatAnalysis {
    let vat_exempt = is_management_fee_vat_exempt(input);
    let mgmt_fee = input.fund_size * input.management_fee_rate;

    let irrecoverable_vat = if vat_exempt {
        // Even exempt funds have some irrecoverable input VAT on costs
        // Estimate 10-20% of fund admin costs subject to VAT
        let admin_costs = input.fund_size * dec!(0.005);
        admin_costs * input.vat_rate * dec!(0.15) // 15% of admin costs have non-recoverable VAT
    } else {
        // Full VAT on management fees is a cost (not recoverable by fund)
        mgmt_fee * input.vat_rate
    };

    let effective_cost_increase = if mgmt_fee > Decimal::ZERO {
        irrecoverable_vat / mgmt_fee
    } else {
        Decimal::ZERO
    };

    VatAnalysis {
        management_fee_vat_exempt: vat_exempt,
        irrecoverable_vat,
        effective_cost_increase,
    }
}

fn compute_marketing_analysis(input: &UkEuFundInput) -> MarketingAnalysis {
    let dom = input.domicile.as_str();

    // AIFMD passport: can market to professional investors in all EU/EEA states
    let passport_jurisdictions = if input.aifmd_compliant {
        match dom {
            "UK" => vec![
                // Post-Brexit, UK AIFM has UK domestic passport only
                "United Kingdom".into(),
            ],
            _ => vec![
                "France".into(),
                "Germany".into(),
                "Netherlands".into(),
                "Luxembourg".into(),
                "Ireland".into(),
                "Italy".into(),
                "Spain".into(),
                "Belgium".into(),
                "Austria".into(),
                "Sweden".into(),
                "Denmark".into(),
                "Finland".into(),
                "Norway".into(),
            ],
        }
    } else {
        vec![]
    };

    // National private placement regimes
    let nppr_jurisdictions = if !input.aifmd_compliant {
        match dom {
            "UK" => vec!["United Kingdom (Section 272 FSMA)".into()],
            _ => vec!["Local NPPR where available".into()],
        }
    } else {
        // AIFMD-compliant funds may still use NPPR for non-passport jurisdictions
        match dom {
            "UK" => vec!["EU (via NPPR post-Brexit)".into()],
            _ => vec![
                "United Kingdom (via NPPR post-Brexit)".into(),
                "Switzerland (via NPPR)".into(),
            ],
        }
    };

    let reverse_solicitation_risk = if input.aifmd_compliant {
        "Low — AIFMD passport provides compliant marketing framework".into()
    } else {
        "High — reliance on reverse solicitation carries regulatory risk".into()
    };

    MarketingAnalysis {
        passport_jurisdictions,
        nppr_jurisdictions,
        reverse_solicitation_risk,
    }
}

fn generate_recommendations(
    input: &UkEuFundInput,
    tax: &TaxAnalysisEu,
    vat: &VatAnalysis,
) -> Vec<String> {
    let mut recs = Vec::new();
    let st = input.structure_type.as_str();

    match st {
        "UKLP" | "UKLLP" => {
            recs.push(
                "Ensure average holding period >= 3 years for 28% CGT rate on carried interest"
                    .into(),
            );
            recs.push("Document income-linked carry test compliance for HMRC".into());
        }
        "OEIC" => {
            recs.push("Consider sub-fund structure for investor segregation".into());
        }
        "ACS" => {
            recs.push(
                "ACS provides tax transparency — ideal for institutional investors seeking treaty access".into(),
            );
        }
        "SICAV" => {
            recs.push("Consider Part II vs SIF vs RAIF for regulatory flexibility".into());
            recs.push("Elect UK reporting fund status if marketing to UK investors".into());
        }
        "FCP" => {
            recs.push("Ensure AMF compliance for French distribution".into());
        }
        "KG" => {
            recs.push("Maintain pure asset management classification to avoid trade tax".into());
            recs.push("Register with BaFin under KAGB for AIFMD compliance".into());
        }
        _ => {}
    }

    if !input.aifmd_compliant {
        recs.push("Consider obtaining AIFMD authorization for EU marketing passport".into());
    }

    if !vat.management_fee_vat_exempt {
        recs.push(
            "Management fees are subject to VAT — consider restructuring to achieve exemption"
                .into(),
        );
    }

    if input.ucits_compliant {
        recs.push("UCITS status provides broad retail distribution access across EU".into());
    }

    if tax.carried_interest_tax_rate > dec!(0.25) {
        recs.push(format!(
            "Carried interest tax rate is {:.1}% — consider structuring to maximize capital gains treatment",
            tax.carried_interest_tax_rate * dec!(100)
        ));
    }

    recs
}

fn generate_warnings(
    input: &UkEuFundInput,
    tax: &TaxAnalysisEu,
    aifmd: &AifmdAnalysis,
) -> Vec<String> {
    let mut warns = Vec::new();

    for test in &tax.structure_specific_tests {
        if !test.passes {
            warns.push(format!("FAILED: {} — {}", test.test_name, test.description));
        }
    }

    if input.domicile == "UK" && input.aifmd_compliant {
        warns.push(
            "Post-Brexit: UK AIFMD passport is domestic only; EU distribution requires NPPR or reverse solicitation".into(),
        );
    }

    if !input.aifmd_compliant && input.domicile != "UK" {
        warns.push("Non-AIFMD compliant EU fund — limited to national private placement".into());
    }

    if aifmd.aifm_capital_required > dec!(5_000_000) {
        warns.push(format!(
            "AIFM capital requirement is high: EUR {}",
            aifmd.aifm_capital_required
        ));
    }

    if tax.fund_level_tax > Decimal::ZERO {
        warns.push(format!(
            "Fund-level tax applies: EUR {} per annum (taxe d'abonnement / equivalent)",
            tax.fund_level_tax
        ));
    }

    let st = input.structure_type.as_str();
    if (st == "UKLP" || st == "UKLLP") && input.fund_term_years < 3 {
        warns.push("Fund term < 3 years — carried interest may not qualify for CGT rate".into());
    }

    if st == "KG" && !input.aifmd_compliant {
        warns.push("German KG without AIFMD registration may face BaFin enforcement action".into());
    }

    warns
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Analyze a UK or EU onshore fund structure, producing economics, tax,
/// AIFMD, VAT, and marketing analysis.
pub fn analyze_uk_eu_fund(input: &UkEuFundInput) -> CorpFinanceResult<UkEuFundOutput> {
    validate_input(input)?;

    let fund_economics = compute_fund_economics(input);
    let tax_analysis = compute_tax_analysis(input);
    let aifmd_analysis = compute_aifmd_analysis(input);
    let vat_analysis = compute_vat_analysis(input);
    let marketing_analysis = compute_marketing_analysis(input);
    let recommendations = generate_recommendations(input, &tax_analysis, &vat_analysis);
    let warnings = generate_warnings(input, &tax_analysis, &aifmd_analysis);

    Ok(UkEuFundOutput {
        structure_type: input.structure_type.clone(),
        domicile: input.domicile.clone(),
        fund_economics,
        tax_analysis,
        aifmd_analysis,
        vat_analysis,
        marketing_analysis,
        recommendations,
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

    fn approx_eq(a: Decimal, b: Decimal, tol: Decimal) -> bool {
        let diff = if a > b { a - b } else { b - a };
        diff < tol
    }

    fn uk_lp_input() -> UkEuFundInput {
        UkEuFundInput {
            fund_name: "UK PE Fund I".into(),
            structure_type: "UKLP".into(),
            domicile: "UK".into(),
            fund_size: dec!(200_000_000),
            management_fee_rate: dec!(0.02),
            carried_interest_rate: dec!(0.20),
            preferred_return: dec!(0.08),
            fund_term_years: 10,
            investor_types: vec![
                InvestorType {
                    category: "Taxable".into(),
                    allocation_pct: dec!(0.60),
                },
                InvestorType {
                    category: "TaxExempt".into(),
                    allocation_pct: dec!(0.40),
                },
            ],
            expected_annual_return: dec!(0.15),
            aifmd_compliant: true,
            ucits_compliant: false,
            vat_rate: dec!(0.20),
        }
    }

    fn uk_llp_input() -> UkEuFundInput {
        UkEuFundInput {
            fund_name: "UK LLP Fund".into(),
            structure_type: "UKLLP".into(),
            domicile: "UK".into(),
            fund_size: dec!(150_000_000),
            management_fee_rate: dec!(0.015),
            carried_interest_rate: dec!(0.20),
            preferred_return: dec!(0.08),
            fund_term_years: 8,
            investor_types: vec![InvestorType {
                category: "Taxable".into(),
                allocation_pct: dec!(1.0),
            }],
            expected_annual_return: dec!(0.12),
            aifmd_compliant: true,
            ucits_compliant: false,
            vat_rate: dec!(0.20),
        }
    }

    fn oeic_input() -> UkEuFundInput {
        UkEuFundInput {
            fund_name: "UK OEIC".into(),
            structure_type: "OEIC".into(),
            domicile: "UK".into(),
            fund_size: dec!(500_000_000),
            management_fee_rate: dec!(0.01),
            carried_interest_rate: dec!(0.0),
            preferred_return: dec!(0.0),
            fund_term_years: 5,
            investor_types: vec![InvestorType {
                category: "Taxable".into(),
                allocation_pct: dec!(1.0),
            }],
            expected_annual_return: dec!(0.08),
            aifmd_compliant: true,
            ucits_compliant: true,
            vat_rate: dec!(0.20),
        }
    }

    fn acs_input() -> UkEuFundInput {
        UkEuFundInput {
            fund_name: "UK ACS".into(),
            structure_type: "ACS".into(),
            domicile: "UK".into(),
            fund_size: dec!(300_000_000),
            management_fee_rate: dec!(0.005),
            carried_interest_rate: dec!(0.0),
            preferred_return: dec!(0.0),
            fund_term_years: 5,
            investor_types: vec![InvestorType {
                category: "TaxExempt".into(),
                allocation_pct: dec!(1.0),
            }],
            expected_annual_return: dec!(0.07),
            aifmd_compliant: true,
            ucits_compliant: false,
            vat_rate: dec!(0.20),
        }
    }

    fn sicav_input() -> UkEuFundInput {
        UkEuFundInput {
            fund_name: "Lux SICAV".into(),
            structure_type: "SICAV".into(),
            domicile: "France".into(),
            fund_size: dec!(1_000_000_000),
            management_fee_rate: dec!(0.015),
            carried_interest_rate: dec!(0.20),
            preferred_return: dec!(0.06),
            fund_term_years: 10,
            investor_types: vec![
                InvestorType {
                    category: "Taxable".into(),
                    allocation_pct: dec!(0.50),
                },
                InvestorType {
                    category: "Foreign".into(),
                    allocation_pct: dec!(0.50),
                },
            ],
            expected_annual_return: dec!(0.10),
            aifmd_compliant: true,
            ucits_compliant: false,
            vat_rate: dec!(0.17),
        }
    }

    fn fcp_input() -> UkEuFundInput {
        UkEuFundInput {
            fund_name: "French FCP".into(),
            structure_type: "FCP".into(),
            domicile: "France".into(),
            fund_size: dec!(100_000_000),
            management_fee_rate: dec!(0.02),
            carried_interest_rate: dec!(0.20),
            preferred_return: dec!(0.08),
            fund_term_years: 7,
            investor_types: vec![InvestorType {
                category: "Taxable".into(),
                allocation_pct: dec!(1.0),
            }],
            expected_annual_return: dec!(0.12),
            aifmd_compliant: true,
            ucits_compliant: false,
            vat_rate: dec!(0.20),
        }
    }

    fn kg_input() -> UkEuFundInput {
        UkEuFundInput {
            fund_name: "German KG Fund".into(),
            structure_type: "KG".into(),
            domicile: "Germany".into(),
            fund_size: dec!(250_000_000),
            management_fee_rate: dec!(0.02),
            carried_interest_rate: dec!(0.20),
            preferred_return: dec!(0.08),
            fund_term_years: 10,
            investor_types: vec![InvestorType {
                category: "Taxable".into(),
                allocation_pct: dec!(1.0),
            }],
            expected_annual_return: dec!(0.12),
            aifmd_compliant: true,
            ucits_compliant: false,
            vat_rate: dec!(0.19),
        }
    }

    // -----------------------------------------------------------------------
    // Basic functionality tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_uk_lp_basic() {
        let input = uk_lp_input();
        let result = analyze_uk_eu_fund(&input).unwrap();
        assert_eq!(result.structure_type, "UKLP");
        assert_eq!(result.domicile, "UK");
    }

    #[test]
    fn test_management_fee_calculation() {
        let input = uk_lp_input();
        let result = analyze_uk_eu_fund(&input).unwrap();
        // 200M * 2% = 4M
        assert_eq!(
            result.fund_economics.management_fees_annual,
            dec!(4_000_000)
        );
    }

    #[test]
    fn test_fund_expenses_positive() {
        let input = uk_lp_input();
        let result = analyze_uk_eu_fund(&input).unwrap();
        assert!(result.fund_economics.total_fund_expenses > Decimal::ZERO);
    }

    #[test]
    fn test_carried_interest_positive_when_above_hurdle() {
        let input = uk_lp_input();
        let result = analyze_uk_eu_fund(&input).unwrap();
        assert!(
            result.fund_economics.carried_interest_potential > Decimal::ZERO,
            "Carry should be positive when 15% return > 8% hurdle"
        );
    }

    #[test]
    fn test_carried_interest_zero_when_below_hurdle() {
        let mut input = uk_lp_input();
        input.expected_annual_return = dec!(0.05);
        let result = analyze_uk_eu_fund(&input).unwrap();
        assert_eq!(
            result.fund_economics.carried_interest_potential,
            Decimal::ZERO
        );
    }

    // -----------------------------------------------------------------------
    // Tax analysis tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_uk_lp_carry_tax_rate() {
        let input = uk_lp_input();
        let result = analyze_uk_eu_fund(&input).unwrap();
        assert_eq!(result.tax_analysis.carried_interest_tax_rate, dec!(0.28));
    }

    #[test]
    fn test_uk_lp_transparent_no_fund_tax() {
        let input = uk_lp_input();
        let result = analyze_uk_eu_fund(&input).unwrap();
        assert_eq!(result.tax_analysis.fund_level_tax, Decimal::ZERO);
    }

    #[test]
    fn test_sicav_subscription_tax() {
        let input = sicav_input();
        let result = analyze_uk_eu_fund(&input).unwrap();
        // 1B * 5bps = 500,000
        assert!(
            approx_eq(result.tax_analysis.fund_level_tax, dec!(500_000), dec!(1)),
            "SICAV subscription tax = {}, expected 500,000",
            result.tax_analysis.fund_level_tax
        );
    }

    #[test]
    fn test_sicav_zero_carry_tax() {
        let input = sicav_input();
        let result = analyze_uk_eu_fund(&input).unwrap();
        assert_eq!(result.tax_analysis.carried_interest_tax_rate, Decimal::ZERO);
    }

    #[test]
    fn test_fcp_transparent() {
        let input = fcp_input();
        let result = analyze_uk_eu_fund(&input).unwrap();
        assert_eq!(result.tax_analysis.fund_level_tax, Decimal::ZERO);
    }

    #[test]
    fn test_fcp_flat_tax_rate() {
        let input = fcp_input();
        let result = analyze_uk_eu_fund(&input).unwrap();
        assert_eq!(result.tax_analysis.carried_interest_tax_rate, dec!(0.30));
    }

    #[test]
    fn test_kg_transparent() {
        let input = kg_input();
        let result = analyze_uk_eu_fund(&input).unwrap();
        assert_eq!(result.tax_analysis.fund_level_tax, Decimal::ZERO);
    }

    #[test]
    fn test_uk_lp_3_year_hold_test() {
        let input = uk_lp_input();
        let result = analyze_uk_eu_fund(&input).unwrap();
        let has_hold_test = result
            .tax_analysis
            .structure_specific_tests
            .iter()
            .any(|t| t.test_name.contains("3-Year"));
        assert!(has_hold_test, "UK LP should have 3-year hold test");
    }

    #[test]
    fn test_uk_lp_short_term_fails_hold_test() {
        let mut input = uk_lp_input();
        input.fund_term_years = 2;
        let result = analyze_uk_eu_fund(&input).unwrap();
        let hold_test = result
            .tax_analysis
            .structure_specific_tests
            .iter()
            .find(|t| t.test_name.contains("3-Year"));
        assert!(hold_test.is_some());
        assert!(!hold_test.unwrap().passes);
    }

    #[test]
    fn test_oeic_fca_authorization_test() {
        let input = oeic_input();
        let result = analyze_uk_eu_fund(&input).unwrap();
        let has_fca = result
            .tax_analysis
            .structure_specific_tests
            .iter()
            .any(|t| t.test_name.contains("FCA"));
        assert!(has_fca);
    }

    #[test]
    fn test_acs_transparency_test() {
        let input = acs_input();
        let result = analyze_uk_eu_fund(&input).unwrap();
        let has_transparency = result
            .tax_analysis
            .structure_specific_tests
            .iter()
            .any(|t| t.test_name.contains("Transparency") || t.test_name.contains("Co-Ownership"));
        assert!(has_transparency);
    }

    // -----------------------------------------------------------------------
    // AIFMD analysis tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_aifmd_passport_eligible() {
        let input = uk_lp_input();
        let result = analyze_uk_eu_fund(&input).unwrap();
        assert!(result.aifmd_analysis.passport_eligible);
    }

    #[test]
    fn test_aifmd_not_compliant_no_passport() {
        let mut input = uk_lp_input();
        input.aifmd_compliant = false;
        let result = analyze_uk_eu_fund(&input).unwrap();
        assert!(!result.aifmd_analysis.passport_eligible);
    }

    #[test]
    fn test_aifmd_capital_base() {
        // Fund size < 250M: capital = base 125,000
        let mut input = uk_lp_input();
        input.fund_size = dec!(100_000_000);
        let result = analyze_uk_eu_fund(&input).unwrap();
        assert_eq!(result.aifmd_analysis.aifm_capital_required, dec!(125_000));
    }

    #[test]
    fn test_aifmd_capital_above_threshold() {
        // Fund size 1B: 125k + (1B - 250M) * 0.02% = 125k + 150k = 275k
        let input = sicav_input();
        let result = analyze_uk_eu_fund(&input).unwrap();
        assert!(
            approx_eq(
                result.aifmd_analysis.aifm_capital_required,
                dec!(275_000),
                dec!(100)
            ),
            "AIFM capital = {}, expected ~275,000",
            result.aifmd_analysis.aifm_capital_required
        );
    }

    #[test]
    fn test_aifmd_capital_capped() {
        // Very large fund: capital capped at 10M
        let mut input = sicav_input();
        input.fund_size = dec!(100_000_000_000); // 100B
        let result = analyze_uk_eu_fund(&input).unwrap();
        assert_eq!(
            result.aifmd_analysis.aifm_capital_required,
            dec!(10_000_000)
        );
    }

    #[test]
    fn test_depositary_required_when_aifmd() {
        let input = uk_lp_input();
        let result = analyze_uk_eu_fund(&input).unwrap();
        assert!(result.aifmd_analysis.depositary_required);
    }

    #[test]
    fn test_depositary_not_required_when_not_aifmd() {
        let mut input = uk_lp_input();
        input.aifmd_compliant = false;
        let result = analyze_uk_eu_fund(&input).unwrap();
        assert!(!result.aifmd_analysis.depositary_required);
    }

    #[test]
    fn test_leverage_ratios_present() {
        let input = uk_lp_input();
        let result = analyze_uk_eu_fund(&input).unwrap();
        assert!(result.aifmd_analysis.leverage_ratio_commitment > Decimal::ZERO);
        assert!(result.aifmd_analysis.leverage_ratio_gross > Decimal::ZERO);
    }

    // -----------------------------------------------------------------------
    // VAT analysis tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_uk_lp_vat_not_exempt() {
        let input = uk_lp_input();
        let result = analyze_uk_eu_fund(&input).unwrap();
        assert!(!result.vat_analysis.management_fee_vat_exempt);
    }

    #[test]
    fn test_uk_lp_vat_cost_positive() {
        let input = uk_lp_input();
        let result = analyze_uk_eu_fund(&input).unwrap();
        assert!(result.fund_economics.vat_cost_annual > Decimal::ZERO);
    }

    #[test]
    fn test_oeic_vat_exempt() {
        let input = oeic_input();
        let result = analyze_uk_eu_fund(&input).unwrap();
        assert!(result.vat_analysis.management_fee_vat_exempt);
    }

    #[test]
    fn test_ucits_vat_exempt() {
        let mut input = uk_lp_input();
        input.ucits_compliant = true;
        let result = analyze_uk_eu_fund(&input).unwrap();
        assert!(result.vat_analysis.management_fee_vat_exempt);
    }

    #[test]
    fn test_oeic_vat_cost_zero() {
        let input = oeic_input();
        let result = analyze_uk_eu_fund(&input).unwrap();
        assert_eq!(result.fund_economics.vat_cost_annual, Decimal::ZERO);
    }

    #[test]
    fn test_kg_vat_not_exempt() {
        let input = kg_input();
        let result = analyze_uk_eu_fund(&input).unwrap();
        assert!(!result.vat_analysis.management_fee_vat_exempt);
    }

    #[test]
    fn test_sicav_vat_exempt() {
        let input = sicav_input();
        let result = analyze_uk_eu_fund(&input).unwrap();
        assert!(result.vat_analysis.management_fee_vat_exempt);
    }

    #[test]
    fn test_irrecoverable_vat_positive_even_when_exempt() {
        let input = oeic_input();
        let result = analyze_uk_eu_fund(&input).unwrap();
        // Even exempt funds have some irrecoverable input VAT
        assert!(result.vat_analysis.irrecoverable_vat > Decimal::ZERO);
    }

    // -----------------------------------------------------------------------
    // Marketing analysis tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_uk_aifmd_passport_domestic_only() {
        let input = uk_lp_input();
        let result = analyze_uk_eu_fund(&input).unwrap();
        assert_eq!(
            result.marketing_analysis.passport_jurisdictions,
            vec!["United Kingdom"]
        );
    }

    #[test]
    fn test_eu_aifmd_passport_multiple() {
        let input = sicav_input();
        let result = analyze_uk_eu_fund(&input).unwrap();
        assert!(
            result.marketing_analysis.passport_jurisdictions.len() > 5,
            "EU AIFMD passport should cover many jurisdictions"
        );
    }

    #[test]
    fn test_non_aifmd_no_passport() {
        let mut input = uk_lp_input();
        input.aifmd_compliant = false;
        let result = analyze_uk_eu_fund(&input).unwrap();
        assert!(result.marketing_analysis.passport_jurisdictions.is_empty());
    }

    #[test]
    fn test_reverse_solicitation_risk_high_without_aifmd() {
        let mut input = uk_lp_input();
        input.aifmd_compliant = false;
        let result = analyze_uk_eu_fund(&input).unwrap();
        assert!(result
            .marketing_analysis
            .reverse_solicitation_risk
            .contains("High"));
    }

    #[test]
    fn test_reverse_solicitation_risk_low_with_aifmd() {
        let input = uk_lp_input();
        let result = analyze_uk_eu_fund(&input).unwrap();
        assert!(result
            .marketing_analysis
            .reverse_solicitation_risk
            .contains("Low"));
    }

    // -----------------------------------------------------------------------
    // Recommendations and warnings tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_recommendations_not_empty() {
        let input = uk_lp_input();
        let result = analyze_uk_eu_fund(&input).unwrap();
        assert!(!result.recommendations.is_empty());
    }

    #[test]
    fn test_uk_post_brexit_warning() {
        let input = uk_lp_input();
        let result = analyze_uk_eu_fund(&input).unwrap();
        assert!(
            result.warnings.iter().any(|w| w.contains("Brexit")),
            "UK AIFMD fund should have post-Brexit warning"
        );
    }

    #[test]
    fn test_short_term_uk_lp_warning() {
        let mut input = uk_lp_input();
        input.fund_term_years = 2;
        let result = analyze_uk_eu_fund(&input).unwrap();
        assert!(
            result
                .warnings
                .iter()
                .any(|w| w.contains("CGT") || w.contains("3 years")),
            "Short-term UK LP should warn about CGT rate qualification"
        );
    }

    #[test]
    fn test_non_aifmd_eu_warning() {
        let mut input = fcp_input();
        input.aifmd_compliant = false;
        let result = analyze_uk_eu_fund(&input).unwrap();
        assert!(
            result
                .warnings
                .iter()
                .any(|w| w.contains("AIFMD") || w.contains("private placement")),
            "Non-AIFMD EU fund should have marketing limitation warning"
        );
    }

    #[test]
    fn test_kg_non_aifmd_warning() {
        let mut input = kg_input();
        input.aifmd_compliant = false;
        let result = analyze_uk_eu_fund(&input).unwrap();
        assert!(
            result.warnings.iter().any(|w| w.contains("BaFin")),
            "Non-AIFMD KG should warn about BaFin enforcement"
        );
    }

    // -----------------------------------------------------------------------
    // Validation error tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_invalid_fund_size() {
        let mut input = uk_lp_input();
        input.fund_size = dec!(-100);
        assert!(analyze_uk_eu_fund(&input).is_err());
    }

    #[test]
    fn test_invalid_structure_type() {
        let mut input = uk_lp_input();
        input.structure_type = "BadType".into();
        assert!(analyze_uk_eu_fund(&input).is_err());
    }

    #[test]
    fn test_invalid_domicile() {
        let mut input = uk_lp_input();
        input.domicile = "Bermuda".into();
        assert!(analyze_uk_eu_fund(&input).is_err());
    }

    #[test]
    fn test_structure_domicile_mismatch_uk_in_france() {
        let mut input = uk_lp_input();
        input.domicile = "France".into();
        assert!(analyze_uk_eu_fund(&input).is_err());
    }

    #[test]
    fn test_structure_domicile_mismatch_kg_in_uk() {
        let mut input = kg_input();
        input.domicile = "UK".into();
        assert!(analyze_uk_eu_fund(&input).is_err());
    }

    #[test]
    fn test_invalid_vat_rate() {
        let mut input = uk_lp_input();
        input.vat_rate = dec!(0.50);
        assert!(analyze_uk_eu_fund(&input).is_err());
    }

    #[test]
    fn test_allocation_not_sum_to_one() {
        let mut input = uk_lp_input();
        input.investor_types = vec![InvestorType {
            category: "Taxable".into(),
            allocation_pct: dec!(0.50),
        }];
        assert!(analyze_uk_eu_fund(&input).is_err());
    }

    #[test]
    fn test_empty_investor_types() {
        let mut input = uk_lp_input();
        input.investor_types = vec![];
        assert!(analyze_uk_eu_fund(&input).is_err());
    }

    #[test]
    fn test_zero_fund_term() {
        let mut input = uk_lp_input();
        input.fund_term_years = 0;
        assert!(analyze_uk_eu_fund(&input).is_err());
    }

    #[test]
    fn test_invalid_management_fee() {
        let mut input = uk_lp_input();
        input.management_fee_rate = dec!(0.15);
        assert!(analyze_uk_eu_fund(&input).is_err());
    }

    // -----------------------------------------------------------------------
    // All structure types produce valid output
    // -----------------------------------------------------------------------

    #[test]
    fn test_all_structures_produce_output() {
        let inputs: Vec<UkEuFundInput> = vec![
            uk_lp_input(),
            uk_llp_input(),
            oeic_input(),
            acs_input(),
            sicav_input(),
            fcp_input(),
            kg_input(),
        ];
        for input in &inputs {
            let result = analyze_uk_eu_fund(input);
            assert!(
                result.is_ok(),
                "Structure '{}' in '{}' should produce valid output, got: {:?}",
                input.structure_type,
                input.domicile,
                result.err()
            );
        }
    }

    #[test]
    fn test_net_return_positive_when_above_hurdle() {
        let input = uk_lp_input();
        let result = analyze_uk_eu_fund(&input).unwrap();
        assert!(
            result.fund_economics.net_return_to_lps > Decimal::ZERO,
            "LP net return should be positive with 15% return > 8% hurdle"
        );
    }

    // -----------------------------------------------------------------------
    // Compound helper test
    // -----------------------------------------------------------------------

    #[test]
    fn test_compound_basic() {
        let result = compound(dec!(0.10), 3);
        assert!(
            approx_eq(result, dec!(1.331), dec!(0.001)),
            "compound(0.10, 3) = {} expected ~1.331",
            result
        );
    }

    #[test]
    fn test_compound_zero_rate() {
        let result = compound(Decimal::ZERO, 10);
        assert_eq!(result, Decimal::ONE);
    }

    // -----------------------------------------------------------------------
    // SICAV Netherlands domicile
    // -----------------------------------------------------------------------

    #[test]
    fn test_sicav_netherlands() {
        let mut input = sicav_input();
        input.domicile = "Netherlands".into();
        let result = analyze_uk_eu_fund(&input).unwrap();
        assert_eq!(result.domicile, "Netherlands");
        assert_eq!(result.structure_type, "SICAV");
    }

    #[test]
    fn test_fcp_tax_transparency_test() {
        let input = fcp_input();
        let result = analyze_uk_eu_fund(&input).unwrap();
        let has_transparency = result
            .tax_analysis
            .structure_specific_tests
            .iter()
            .any(|t| t.test_name.contains("Transparency") || t.test_name.contains("AMF"));
        assert!(has_transparency);
    }

    #[test]
    fn test_kg_trade_tax_test() {
        let input = kg_input();
        let result = analyze_uk_eu_fund(&input).unwrap();
        let has_trade_tax = result
            .tax_analysis
            .structure_specific_tests
            .iter()
            .any(|t| t.test_name.contains("Trade Tax"));
        assert!(has_trade_tax, "KG should have trade tax exemption test");
    }

    #[test]
    fn test_reporting_fund_status_populated() {
        let input = uk_lp_input();
        let result = analyze_uk_eu_fund(&input).unwrap();
        assert!(!result.tax_analysis.reporting_fund_status.is_empty());
    }
}
