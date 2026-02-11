use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::{CorpFinanceError, CorpFinanceResult};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestorType {
    pub category: String,
    pub allocation_pct: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsFundInput {
    pub fund_name: String,
    /// "DelawareLP", "LLC", "REIT", "MLP", "BDC", "QOZ"
    pub structure_type: String,
    pub fund_size: Decimal,
    pub management_fee_rate: Decimal,
    pub carried_interest_rate: Decimal,
    pub preferred_return: Decimal,
    pub gp_commitment_pct: Decimal,
    pub fund_term_years: u32,
    /// "Delaware", "California", etc.
    pub state_of_formation: String,
    /// category in ["TaxExempt", "Taxable", "Foreign", "ERISA"]
    pub investor_types: Vec<InvestorType>,
    pub expected_annual_return: Decimal,
    /// "Quarterly", "Annual", "AtRealization"
    pub distribution_frequency: String,
    /// e.g. ["Section754", "QEF", "PFIC"]
    pub tax_elections: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundEconomics {
    pub management_fees_annual: Decimal,
    pub carried_interest_potential: Decimal,
    pub gp_return: Decimal,
    pub net_return_to_lps: Decimal,
    pub total_fund_expenses: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceTest {
    pub test_name: String,
    pub required_threshold: String,
    pub assumed_value: String,
    pub passes: bool,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxAnalysis {
    pub effective_tax_rate_taxable: Decimal,
    pub ubti_risk_score: Decimal,
    pub eci_risk_score: Decimal,
    pub pass_through_benefits: Vec<String>,
    pub structure_specific_tests: Vec<ComplianceTest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErisaAnalysis {
    pub plan_asset_risk: String,
    pub vcoc_eligible: bool,
    pub reoc_eligible: bool,
    pub blocker_recommended: bool,
    pub benefit_plan_investor_pct: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateAnalysis {
    pub formation_cost: Decimal,
    pub annual_cost: Decimal,
    pub franchise_tax: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestorSuitability {
    pub category: String,
    pub suitable: bool,
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsFundOutput {
    pub structure_type: String,
    pub fund_economics: FundEconomics,
    pub tax_analysis: TaxAnalysis,
    pub erisa_analysis: ErisaAnalysis,
    pub state_analysis: StateAnalysis,
    pub investor_suitability: Vec<InvestorSuitability>,
    pub recommendations: Vec<String>,
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

const VALID_STRUCTURES: &[&str] = &["DelawareLP", "LLC", "REIT", "MLP", "BDC", "QOZ"];
const VALID_CATEGORIES: &[&str] = &["TaxExempt", "Taxable", "Foreign", "ERISA"];
const VALID_DIST_FREQ: &[&str] = &["Quarterly", "Annual", "AtRealization"];

fn validate_input(input: &UsFundInput) -> CorpFinanceResult<()> {
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
    if input.gp_commitment_pct < Decimal::ZERO || input.gp_commitment_pct > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "gp_commitment_pct".into(),
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
    if !VALID_DIST_FREQ.contains(&input.distribution_frequency.as_str()) {
        return Err(CorpFinanceError::InvalidInput {
            field: "distribution_frequency".into(),
            reason: format!("must be one of: {}", VALID_DIST_FREQ.join(", ")),
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
    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Iterative compound: (1 + r)^n using multiplication to avoid powd drift.
fn compound(rate: Decimal, years: u32) -> Decimal {
    let mut result = Decimal::ONE;
    let factor = Decimal::ONE + rate;
    for _ in 0..years {
        result *= factor;
    }
    result
}

fn compute_fund_economics(input: &UsFundInput) -> FundEconomics {
    let mgmt_fee_annual = input.fund_size * input.management_fee_rate;
    let term = Decimal::from(input.fund_term_years);

    // Total fund value at end (simple compound)
    let total_value =
        input.fund_size * compound(input.expected_annual_return, input.fund_term_years);
    let total_profit = if total_value > input.fund_size {
        total_value - input.fund_size
    } else {
        Decimal::ZERO
    };

    // Preferred return hurdle over fund life
    let hurdle_value = input.fund_size * compound(input.preferred_return, input.fund_term_years);
    let excess_above_hurdle = if total_value > hurdle_value {
        total_value - hurdle_value
    } else {
        Decimal::ZERO
    };

    // Carried interest (European waterfall: carry only on excess above hurdle)
    let carried_interest = excess_above_hurdle * input.carried_interest_rate;

    // GP commitment return
    let gp_commitment = input.fund_size * input.gp_commitment_pct;
    let gp_return = gp_commitment * compound(input.expected_annual_return, input.fund_term_years)
        - gp_commitment;

    // Fund expenses (estimated: 0.5% of AUM annually for admin/legal/audit)
    let annual_expenses = input.fund_size * dec!(0.005);
    let total_expenses = annual_expenses * term + mgmt_fee_annual * term;

    // Net return to LPs
    let lp_capital = input.fund_size * (Decimal::ONE - input.gp_commitment_pct);
    let lp_share_of_profit = total_profit - carried_interest - gp_return;
    let net_return_to_lps = if lp_capital > Decimal::ZERO {
        lp_share_of_profit / lp_capital
    } else {
        Decimal::ZERO
    };

    FundEconomics {
        management_fees_annual: mgmt_fee_annual,
        carried_interest_potential: carried_interest,
        gp_return,
        net_return_to_lps,
        total_fund_expenses: total_expenses,
    }
}

fn compute_tax_analysis(input: &UsFundInput) -> TaxAnalysis {
    let st = input.structure_type.as_str();

    // Effective tax rate for taxable investors (federal estimates)
    let effective_tax_rate = match st {
        "DelawareLP" | "LLC" | "FCP" => dec!(0.238), // pass-through, top LTCG 23.8%
        "REIT" => dec!(0.37),                        // ordinary income rate on distributions
        "MLP" => dec!(0.238),                        // LTCG on unit sales
        "BDC" => dec!(0.37),                         // dividends taxed as ordinary income
        "QOZ" => dec!(0.0),                          // 10-year hold = step-up to FMV
        _ => dec!(0.37),
    };

    // UBTI risk (0=none, 1=high)
    let ubti_risk = match st {
        "DelawareLP" => dec!(0.3), // debt-financed income risk
        "LLC" => dec!(0.3),
        "REIT" => dec!(0.1), // generally no UBTI if properly structured
        "MLP" => dec!(0.9),  // MLPs generate UBTI for tax-exempt investors
        "BDC" => dec!(0.2),
        "QOZ" => dec!(0.2),
        _ => dec!(0.5),
    };

    // ECI risk for foreign investors
    let eci_risk = match st {
        "DelawareLP" => dec!(0.7), // trade or business income passes through
        "LLC" => dec!(0.7),
        "REIT" => dec!(0.2), // FIRPTA may apply to real estate
        "MLP" => dec!(0.9),  // ECI for all foreign investors
        "BDC" => dec!(0.3),
        "QOZ" => dec!(0.5),
        _ => dec!(0.5),
    };

    // Pass-through benefits
    let pass_through_benefits = match st {
        "DelawareLP" => vec![
            "Flow-through of capital gains".into(),
            "Section 199A deduction potential (20% QBI)".into(),
            "K-1 reporting to investors".into(),
            "No entity-level tax".into(),
        ],
        "LLC" => vec![
            "Check-the-box flexibility".into(),
            "Flow-through taxation".into(),
            "Limited liability protection".into(),
            "K-1 reporting to members".into(),
        ],
        "REIT" => vec![
            "Dividends-paid deduction eliminates corporate tax".into(),
            "Qualified REIT dividend (199A) — 20% deduction".into(),
        ],
        "MLP" => vec![
            "Tax-deferred distributions (return of capital)".into(),
            "Depreciation pass-through".into(),
            "Section 754 step-up available".into(),
        ],
        "BDC" => vec![
            "Pass-through via RIC structure".into(),
            "Dividends-paid deduction".into(),
        ],
        "QOZ" => vec![
            "Deferral of capital gains until 2026".into(),
            "10-year hold: step-up to FMV (no tax on appreciation)".into(),
            "Substantial improvement benefit".into(),
        ],
        _ => vec![],
    };

    // Structure-specific compliance tests
    let tests = compute_compliance_tests(input);

    TaxAnalysis {
        effective_tax_rate_taxable: effective_tax_rate,
        ubti_risk_score: ubti_risk,
        eci_risk_score: eci_risk,
        pass_through_benefits,
        structure_specific_tests: tests,
    }
}

fn compute_compliance_tests(input: &UsFundInput) -> Vec<ComplianceTest> {
    let st = input.structure_type.as_str();
    let mut tests = Vec::new();

    match st {
        "REIT" => {
            tests.push(ComplianceTest {
                test_name: "Distribution Requirement".into(),
                required_threshold: ">=90% of taxable income".into(),
                assumed_value: "90% assumed".into(),
                passes: true,
                description: "REIT must distribute at least 90% of taxable income".into(),
            });
            tests.push(ComplianceTest {
                test_name: "75% Asset Test".into(),
                required_threshold: ">=75% real estate assets".into(),
                assumed_value: "75% assumed".into(),
                passes: true,
                description:
                    "At least 75% of assets must be real estate, cash, or government securities"
                        .into(),
            });
            tests.push(ComplianceTest {
                test_name: "75% Income Test".into(),
                required_threshold: ">=75% from real estate".into(),
                assumed_value: "75% assumed".into(),
                passes: true,
                description:
                    "At least 75% of gross income from rents, mortgages, or real property sales"
                        .into(),
            });
            tests.push(ComplianceTest {
                test_name: "95% Income Test".into(),
                required_threshold: ">=95% passive income".into(),
                assumed_value: "95% assumed".into(),
                passes: true,
                description: "At least 95% of gross income from passive sources".into(),
            });
            tests.push(ComplianceTest {
                test_name: "TRS Limit".into(),
                required_threshold: "<=25% of assets in TRS".into(),
                assumed_value: "0% assumed".into(),
                passes: true,
                description: "Taxable REIT subsidiary cannot exceed 25% of total assets".into(),
            });
        }
        "MLP" => {
            tests.push(ComplianceTest {
                test_name: "Qualifying Income Test".into(),
                required_threshold: ">=90% qualifying income".into(),
                assumed_value: "90% assumed".into(),
                passes: true,
                description: "At least 90% of income must be qualifying (natural resources, real estate, etc.)".into(),
            });
            tests.push(ComplianceTest {
                test_name: "IDR Tier 1 (25/15/50 split)".into(),
                required_threshold: "IDR tiers properly structured".into(),
                assumed_value: "Standard tiers assumed".into(),
                passes: true,
                description: "Incentive distribution rights at 25%/15%/50% split levels".into(),
            });
            tests.push(ComplianceTest {
                test_name: "PTP Status".into(),
                required_threshold: "Publicly traded partnership rules".into(),
                assumed_value: "Compliant".into(),
                passes: true,
                description: "Must meet qualifying income test to avoid corporate taxation as PTP"
                    .into(),
            });
        }
        "BDC" => {
            tests.push(ComplianceTest {
                test_name: "Distribution Requirement".into(),
                required_threshold: ">=90% of investment income".into(),
                assumed_value: "90% assumed".into(),
                passes: true,
                description: "BDC must distribute at least 90% of net investment income".into(),
            });
            tests.push(ComplianceTest {
                test_name: "70% Qualifying Asset Test".into(),
                required_threshold: ">=70% qualifying assets".into(),
                assumed_value: "70% assumed".into(),
                passes: true,
                description:
                    "At least 70% of assets in qualifying investments (private/thinly traded)"
                        .into(),
            });
            tests.push(ComplianceTest {
                test_name: "Leverage Limit".into(),
                required_threshold: "<=2:1 debt-to-equity".into(),
                assumed_value: "1.5:1 assumed".into(),
                passes: true,
                description: "Debt-to-equity cannot exceed 2:1 (post-2018 SBCAA)".into(),
            });
        }
        "QOZ" => {
            tests.push(ComplianceTest {
                test_name: "90% QOZP Test".into(),
                required_threshold: ">=90% in qualified opportunity zone property".into(),
                assumed_value: "90% assumed".into(),
                passes: true,
                description: "At least 90% of assets must be qualified opportunity zone property"
                    .into(),
            });
            tests.push(ComplianceTest {
                test_name: "Substantial Improvement".into(),
                required_threshold: "Basis doubled within 30 months".into(),
                assumed_value: "Improvement planned".into(),
                passes: true,
                description:
                    "For existing buildings, basis must be doubled in improvements within 30 months"
                        .into(),
            });
            tests.push(ComplianceTest {
                test_name: "10-Year Hold Period".into(),
                required_threshold: ">=10 years for step-up to FMV".into(),
                assumed_value: format!("{} years", input.fund_term_years),
                passes: input.fund_term_years >= 10,
                description: "Must hold for at least 10 years to receive step-up to fair market value".into(),
            });
        }
        "DelawareLP" => {
            tests.push(ComplianceTest {
                test_name: "LP Agreement".into(),
                required_threshold: "Valid limited partnership agreement".into(),
                assumed_value: "In place".into(),
                passes: true,
                description: "Must have valid LP agreement filed with Delaware Secretary of State"
                    .into(),
            });
            if input.tax_elections.contains(&"Section754".to_string()) {
                tests.push(ComplianceTest {
                    test_name: "Section 754 Election".into(),
                    required_threshold: "Election filed with IRS".into(),
                    assumed_value: "Elected".into(),
                    passes: true,
                    description: "Section 754 election allows step-up in basis on transfer of partnership interests".into(),
                });
            }
        }
        "LLC" => {
            tests.push(ComplianceTest {
                test_name: "Check-the-Box Election".into(),
                required_threshold: "Form 8832 filed".into(),
                assumed_value: "Partnership classification".into(),
                passes: true,
                description: "LLC must elect partnership or disregarded entity classification for pass-through".into(),
            });
        }
        _ => {}
    }

    tests
}

fn compute_erisa_analysis(input: &UsFundInput) -> ErisaAnalysis {
    // Calculate benefit plan investor percentage
    let erisa_pct: Decimal = input
        .investor_types
        .iter()
        .filter(|i| i.category == "ERISA" || i.category == "TaxExempt")
        .map(|i| i.allocation_pct)
        .sum();

    // 25% threshold: if benefit plan investors >= 25%, fund assets = plan assets
    let plan_asset_risk = if erisa_pct >= dec!(0.25) {
        "High — exceeds 25% plan asset threshold".to_string()
    } else if erisa_pct >= dec!(0.15) {
        "Moderate — approaching 25% threshold".to_string()
    } else {
        "Low — below 25% threshold".to_string()
    };

    // VCOC: venture capital operating company exemption (>50% in operating companies + management rights)
    let vcoc_eligible = matches!(input.structure_type.as_str(), "DelawareLP" | "LLC");

    // REOC: real estate operating company (>50% in real estate with active management)
    let reoc_eligible = matches!(input.structure_type.as_str(), "REIT" | "QOZ");

    // Blocker recommended if ERISA investors and high UBTI risk structure
    let blocker_recommended =
        erisa_pct > Decimal::ZERO && matches!(input.structure_type.as_str(), "MLP");

    ErisaAnalysis {
        plan_asset_risk,
        vcoc_eligible,
        reoc_eligible,
        blocker_recommended,
        benefit_plan_investor_pct: erisa_pct,
    }
}

fn compute_state_analysis(input: &UsFundInput) -> StateAnalysis {
    let state = input.state_of_formation.as_str();

    let (formation_cost, annual_cost, franchise_tax) = match state {
        "Delaware" => (dec!(200), dec!(300), dec!(300)),
        "California" => (dec!(70), dec!(800), dec!(800)),
        "New York" => (dec!(200), dec!(4500), dec!(25)),
        "Texas" => (dec!(300), dec!(0), dec!(0)),
        "Nevada" => (dec!(75), dec!(350), dec!(0)),
        "Wyoming" => (dec!(100), dec!(60), dec!(0)),
        _ => (dec!(150), dec!(500), dec!(500)),
    };

    StateAnalysis {
        formation_cost,
        annual_cost,
        franchise_tax,
    }
}

fn compute_investor_suitability(input: &UsFundInput) -> Vec<InvestorSuitability> {
    let st = input.structure_type.as_str();

    input
        .investor_types
        .iter()
        .map(|inv| {
            let cat = inv.category.as_str();
            let mut issues = Vec::new();
            let mut suitable = true;

            match (cat, st) {
                ("TaxExempt", "MLP") => {
                    issues
                        .push("UBTI risk: MLP income is unrelated business taxable income".into());
                    issues.push("Consider blocker corporation to shield UBTI".into());
                    suitable = false;
                }
                ("TaxExempt", "DelawareLP") | ("TaxExempt", "LLC") => {
                    if input.tax_elections.iter().any(|e| e == "PFIC") {
                        issues.push("PFIC election may generate UBTI".into());
                    }
                    issues.push("Debt-financed income may generate UBTI".into());
                }
                ("TaxExempt", _) => {}
                ("Foreign", "MLP") => {
                    issues.push(
                        "ECI exposure: foreign investors subject to US tax on MLP income".into(),
                    );
                    issues.push("Withholding at 37% on ECI distributions".into());
                    issues.push("US tax return filing required".into());
                    suitable = false;
                }
                ("Foreign", "DelawareLP") | ("Foreign", "LLC") => {
                    issues.push("ECI risk: partnership trade or business income is ECI".into());
                    issues.push("FIRPTA may apply to real property dispositions".into());
                    issues.push("Consider blocker to convert ECI to portfolio income".into());
                }
                ("Foreign", "REIT") => {
                    issues.push(
                        "FIRPTA withholding on REIT distributions from US real property".into(),
                    );
                }
                ("Foreign", _) => {}
                ("ERISA", "MLP") => {
                    issues.push("UBTI risk makes MLP unsuitable for ERISA plans".into());
                    issues.push("Blocker entity required".into());
                    suitable = false;
                }
                ("ERISA", _) => {
                    let erisa_pct: Decimal = input
                        .investor_types
                        .iter()
                        .filter(|i| i.category == "ERISA" || i.category == "TaxExempt")
                        .map(|i| i.allocation_pct)
                        .sum();
                    if erisa_pct >= dec!(0.25) {
                        issues
                            .push("Plan asset risk: benefit plan investors >= 25% of fund".into());
                        issues
                            .push("VCOC or REOC exemption needed to avoid plan asset rules".into());
                    }
                }
                ("Taxable", _) => {
                    // Generally suitable
                }
                _ => {}
            }

            InvestorSuitability {
                category: inv.category.clone(),
                suitable,
                issues,
            }
        })
        .collect()
}

fn generate_recommendations(
    input: &UsFundInput,
    tax: &TaxAnalysis,
    erisa: &ErisaAnalysis,
) -> Vec<String> {
    let mut recs = Vec::new();
    let st = input.structure_type.as_str();

    match st {
        "DelawareLP" => {
            if !input.tax_elections.contains(&"Section754".to_string()) {
                recs.push(
                    "Consider Section 754 election for basis step-up on secondary transfers".into(),
                );
            }
            recs.push(
                "Ensure GP maintains at least 1% ownership for tax partnership validity".into(),
            );
        }
        "LLC" => {
            recs.push("File Form 8832 to elect partnership classification if multi-member".into());
            recs.push("Consider series LLC structure for asset isolation".into());
        }
        "REIT" => {
            recs.push("Monitor 75%/95% income tests and 75% asset test quarterly".into());
            recs.push(
                "Consider TRS for non-qualifying activities (capped at 25% of assets)".into(),
            );
        }
        "MLP" => {
            recs.push("Monitor qualifying income test (90%) quarterly".into());
            recs.push("Consider blocker entity for tax-exempt and foreign investors".into());
        }
        "BDC" => {
            recs.push("Monitor 2:1 leverage limit and 70% qualifying asset test".into());
            recs.push("Consider spillover dividend for excess undistributed income".into());
        }
        "QOZ" => {
            if input.fund_term_years < 10 {
                recs.push("Extend fund term to at least 10 years for full step-up benefit".into());
            }
            recs.push("Ensure 90% QOZP test compliance at semi-annual testing dates".into());
            recs.push("Plan substantial improvement within 30 months of acquisition".into());
        }
        _ => {}
    }

    if erisa.benefit_plan_investor_pct > dec!(0.15) && erisa.benefit_plan_investor_pct < dec!(0.25)
    {
        recs.push("Approaching 25% benefit plan investor threshold — monitor closely".into());
    }
    if erisa.benefit_plan_investor_pct >= dec!(0.25) && !erisa.vcoc_eligible && !erisa.reoc_eligible
    {
        recs.push("Obtain VCOC or REOC exemption to avoid plan asset rules".into());
    }

    if tax.ubti_risk_score > dec!(0.5) {
        recs.push("High UBTI risk — consider blocker corporation for tax-exempt investors".into());
    }
    if tax.eci_risk_score > dec!(0.5) {
        recs.push("High ECI risk — consider offshore blocker for foreign investors".into());
    }

    recs
}

fn generate_warnings(input: &UsFundInput, tax: &TaxAnalysis, erisa: &ErisaAnalysis) -> Vec<String> {
    let mut warns = Vec::new();

    if erisa.benefit_plan_investor_pct >= dec!(0.25) {
        warns.push("ERISA plan asset rules triggered — benefit plan investors >= 25%".into());
    }

    if input.structure_type == "QOZ" && input.fund_term_years < 10 {
        warns.push(
            "QOZ fund term < 10 years — investors will not receive full step-up benefit".into(),
        );
    }

    for test in &tax.structure_specific_tests {
        if !test.passes {
            warns.push(format!("FAILED: {} — {}", test.test_name, test.description));
        }
    }

    let has_foreign = input.investor_types.iter().any(|i| i.category == "Foreign");
    if has_foreign && tax.eci_risk_score > dec!(0.5) {
        warns.push("Foreign investors face significant ECI exposure in this structure".into());
    }

    let has_tax_exempt = input
        .investor_types
        .iter()
        .any(|i| i.category == "TaxExempt");
    if has_tax_exempt && tax.ubti_risk_score > dec!(0.5) {
        warns.push("Tax-exempt investors face significant UBTI exposure".into());
    }

    warns
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Analyze a US onshore fund structure, producing economics, tax, ERISA,
/// state, and investor suitability analysis.
pub fn analyze_us_fund_structure(input: &UsFundInput) -> CorpFinanceResult<UsFundOutput> {
    validate_input(input)?;

    let fund_economics = compute_fund_economics(input);
    let tax_analysis = compute_tax_analysis(input);
    let erisa_analysis = compute_erisa_analysis(input);
    let state_analysis = compute_state_analysis(input);
    let investor_suitability = compute_investor_suitability(input);
    let recommendations = generate_recommendations(input, &tax_analysis, &erisa_analysis);
    let warnings = generate_warnings(input, &tax_analysis, &erisa_analysis);

    Ok(UsFundOutput {
        structure_type: input.structure_type.clone(),
        fund_economics,
        tax_analysis,
        erisa_analysis,
        state_analysis,
        investor_suitability,
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

    fn default_input() -> UsFundInput {
        UsFundInput {
            fund_name: "Test Fund I".into(),
            structure_type: "DelawareLP".into(),
            fund_size: dec!(100_000_000),
            management_fee_rate: dec!(0.02),
            carried_interest_rate: dec!(0.20),
            preferred_return: dec!(0.08),
            gp_commitment_pct: dec!(0.02),
            fund_term_years: 10,
            state_of_formation: "Delaware".into(),
            investor_types: vec![
                InvestorType {
                    category: "Taxable".into(),
                    allocation_pct: dec!(0.60),
                },
                InvestorType {
                    category: "TaxExempt".into(),
                    allocation_pct: dec!(0.25),
                },
                InvestorType {
                    category: "Foreign".into(),
                    allocation_pct: dec!(0.15),
                },
            ],
            expected_annual_return: dec!(0.15),
            distribution_frequency: "Quarterly".into(),
            tax_elections: vec!["Section754".into()],
        }
    }

    fn reit_input() -> UsFundInput {
        UsFundInput {
            fund_name: "REIT Fund".into(),
            structure_type: "REIT".into(),
            fund_size: dec!(500_000_000),
            management_fee_rate: dec!(0.01),
            carried_interest_rate: dec!(0.15),
            preferred_return: dec!(0.06),
            gp_commitment_pct: dec!(0.01),
            fund_term_years: 7,
            state_of_formation: "Delaware".into(),
            investor_types: vec![
                InvestorType {
                    category: "Taxable".into(),
                    allocation_pct: dec!(0.70),
                },
                InvestorType {
                    category: "TaxExempt".into(),
                    allocation_pct: dec!(0.30),
                },
            ],
            expected_annual_return: dec!(0.10),
            distribution_frequency: "Quarterly".into(),
            tax_elections: vec![],
        }
    }

    fn mlp_input() -> UsFundInput {
        UsFundInput {
            fund_name: "Energy MLP".into(),
            structure_type: "MLP".into(),
            fund_size: dec!(200_000_000),
            management_fee_rate: dec!(0.02),
            carried_interest_rate: dec!(0.20),
            preferred_return: dec!(0.08),
            gp_commitment_pct: dec!(0.02),
            fund_term_years: 10,
            state_of_formation: "Delaware".into(),
            investor_types: vec![
                InvestorType {
                    category: "Taxable".into(),
                    allocation_pct: dec!(0.50),
                },
                InvestorType {
                    category: "TaxExempt".into(),
                    allocation_pct: dec!(0.20),
                },
                InvestorType {
                    category: "Foreign".into(),
                    allocation_pct: dec!(0.30),
                },
            ],
            expected_annual_return: dec!(0.12),
            distribution_frequency: "Quarterly".into(),
            tax_elections: vec![],
        }
    }

    fn bdc_input() -> UsFundInput {
        UsFundInput {
            fund_name: "BDC Fund".into(),
            structure_type: "BDC".into(),
            fund_size: dec!(300_000_000),
            management_fee_rate: dec!(0.015),
            carried_interest_rate: dec!(0.175),
            preferred_return: dec!(0.07),
            gp_commitment_pct: dec!(0.03),
            fund_term_years: 8,
            state_of_formation: "Delaware".into(),
            investor_types: vec![InvestorType {
                category: "Taxable".into(),
                allocation_pct: dec!(1.0),
            }],
            expected_annual_return: dec!(0.10),
            distribution_frequency: "Quarterly".into(),
            tax_elections: vec![],
        }
    }

    fn qoz_input() -> UsFundInput {
        UsFundInput {
            fund_name: "QOZ Fund".into(),
            structure_type: "QOZ".into(),
            fund_size: dec!(50_000_000),
            management_fee_rate: dec!(0.015),
            carried_interest_rate: dec!(0.20),
            preferred_return: dec!(0.08),
            gp_commitment_pct: dec!(0.05),
            fund_term_years: 12,
            state_of_formation: "Delaware".into(),
            investor_types: vec![InvestorType {
                category: "Taxable".into(),
                allocation_pct: dec!(1.0),
            }],
            expected_annual_return: dec!(0.12),
            distribution_frequency: "AtRealization".into(),
            tax_elections: vec![],
        }
    }

    fn llc_input() -> UsFundInput {
        UsFundInput {
            fund_name: "LLC Fund".into(),
            structure_type: "LLC".into(),
            fund_size: dec!(75_000_000),
            management_fee_rate: dec!(0.02),
            carried_interest_rate: dec!(0.20),
            preferred_return: dec!(0.08),
            gp_commitment_pct: dec!(0.02),
            fund_term_years: 10,
            state_of_formation: "California".into(),
            investor_types: vec![
                InvestorType {
                    category: "Taxable".into(),
                    allocation_pct: dec!(0.80),
                },
                InvestorType {
                    category: "ERISA".into(),
                    allocation_pct: dec!(0.20),
                },
            ],
            expected_annual_return: dec!(0.14),
            distribution_frequency: "Annual".into(),
            tax_elections: vec![],
        }
    }

    // -----------------------------------------------------------------------
    // Basic functionality tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_delaware_lp_basic() {
        let input = default_input();
        let result = analyze_us_fund_structure(&input).unwrap();
        assert_eq!(result.structure_type, "DelawareLP");
    }

    #[test]
    fn test_management_fee_calculation() {
        let input = default_input();
        let result = analyze_us_fund_structure(&input).unwrap();
        // 100M * 2% = 2M
        assert_eq!(
            result.fund_economics.management_fees_annual,
            dec!(2_000_000)
        );
    }

    #[test]
    fn test_fund_expenses_positive() {
        let input = default_input();
        let result = analyze_us_fund_structure(&input).unwrap();
        assert!(result.fund_economics.total_fund_expenses > Decimal::ZERO);
    }

    #[test]
    fn test_carried_interest_positive_with_positive_return() {
        let input = default_input();
        let result = analyze_us_fund_structure(&input).unwrap();
        // 15% return > 8% hurdle => carry should be positive
        assert!(
            result.fund_economics.carried_interest_potential > Decimal::ZERO,
            "Carried interest should be positive when return > hurdle"
        );
    }

    #[test]
    fn test_carried_interest_zero_when_below_hurdle() {
        let mut input = default_input();
        input.expected_annual_return = dec!(0.05); // below 8% hurdle
        let result = analyze_us_fund_structure(&input).unwrap();
        assert_eq!(
            result.fund_economics.carried_interest_potential,
            Decimal::ZERO,
            "Carry should be zero when return < hurdle"
        );
    }

    #[test]
    fn test_gp_return_positive() {
        let input = default_input();
        let result = analyze_us_fund_structure(&input).unwrap();
        assert!(result.fund_economics.gp_return > Decimal::ZERO);
    }

    // -----------------------------------------------------------------------
    // Tax analysis tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_delaware_lp_pass_through() {
        let input = default_input();
        let result = analyze_us_fund_structure(&input).unwrap();
        assert!(
            result
                .tax_analysis
                .pass_through_benefits
                .iter()
                .any(|b| b.contains("Flow-through")),
            "Delaware LP should have flow-through benefits"
        );
    }

    #[test]
    fn test_delaware_lp_effective_tax_rate() {
        let input = default_input();
        let result = analyze_us_fund_structure(&input).unwrap();
        assert_eq!(result.tax_analysis.effective_tax_rate_taxable, dec!(0.238));
    }

    #[test]
    fn test_reit_distribution_test() {
        let input = reit_input();
        let result = analyze_us_fund_structure(&input).unwrap();
        let has_dist_test = result
            .tax_analysis
            .structure_specific_tests
            .iter()
            .any(|t| t.test_name.contains("Distribution"));
        assert!(
            has_dist_test,
            "REIT should have distribution requirement test"
        );
    }

    #[test]
    fn test_reit_asset_tests() {
        let input = reit_input();
        let result = analyze_us_fund_structure(&input).unwrap();
        let test_names: Vec<&str> = result
            .tax_analysis
            .structure_specific_tests
            .iter()
            .map(|t| t.test_name.as_str())
            .collect();
        assert!(test_names.contains(&"75% Asset Test"));
        assert!(test_names.contains(&"75% Income Test"));
        assert!(test_names.contains(&"95% Income Test"));
        assert!(test_names.contains(&"TRS Limit"));
    }

    #[test]
    fn test_mlp_qualifying_income_test() {
        let input = mlp_input();
        let result = analyze_us_fund_structure(&input).unwrap();
        let has_qi = result
            .tax_analysis
            .structure_specific_tests
            .iter()
            .any(|t| t.test_name.contains("Qualifying Income"));
        assert!(has_qi, "MLP should have qualifying income test");
    }

    #[test]
    fn test_mlp_high_ubti_risk() {
        let input = mlp_input();
        let result = analyze_us_fund_structure(&input).unwrap();
        assert!(
            result.tax_analysis.ubti_risk_score >= dec!(0.9),
            "MLP UBTI risk {} should be >= 0.9",
            result.tax_analysis.ubti_risk_score
        );
    }

    #[test]
    fn test_mlp_high_eci_risk() {
        let input = mlp_input();
        let result = analyze_us_fund_structure(&input).unwrap();
        assert!(
            result.tax_analysis.eci_risk_score >= dec!(0.9),
            "MLP ECI risk {} should be >= 0.9",
            result.tax_analysis.eci_risk_score
        );
    }

    #[test]
    fn test_bdc_compliance_tests() {
        let input = bdc_input();
        let result = analyze_us_fund_structure(&input).unwrap();
        let test_names: Vec<&str> = result
            .tax_analysis
            .structure_specific_tests
            .iter()
            .map(|t| t.test_name.as_str())
            .collect();
        assert!(test_names.contains(&"Distribution Requirement"));
        assert!(test_names.contains(&"70% Qualifying Asset Test"));
        assert!(test_names.contains(&"Leverage Limit"));
    }

    #[test]
    fn test_qoz_step_up_benefit() {
        let input = qoz_input();
        let result = analyze_us_fund_structure(&input).unwrap();
        assert_eq!(
            result.tax_analysis.effective_tax_rate_taxable,
            dec!(0.0),
            "QOZ 10-year hold should have 0% effective tax"
        );
    }

    #[test]
    fn test_qoz_compliance_tests_pass_with_long_term() {
        let input = qoz_input(); // 12-year term
        let result = analyze_us_fund_structure(&input).unwrap();
        let hold_test = result
            .tax_analysis
            .structure_specific_tests
            .iter()
            .find(|t| t.test_name.contains("10-Year"));
        assert!(hold_test.is_some());
        assert!(
            hold_test.unwrap().passes,
            "12-year term should pass 10-year test"
        );
    }

    #[test]
    fn test_qoz_compliance_tests_fail_with_short_term() {
        let mut input = qoz_input();
        input.fund_term_years = 7;
        let result = analyze_us_fund_structure(&input).unwrap();
        let hold_test = result
            .tax_analysis
            .structure_specific_tests
            .iter()
            .find(|t| t.test_name.contains("10-Year"));
        assert!(hold_test.is_some());
        assert!(
            !hold_test.unwrap().passes,
            "7-year term should fail 10-year test"
        );
    }

    #[test]
    fn test_llc_check_the_box() {
        let input = llc_input();
        let result = analyze_us_fund_structure(&input).unwrap();
        let has_ctb = result
            .tax_analysis
            .structure_specific_tests
            .iter()
            .any(|t| t.test_name.contains("Check-the-Box"));
        assert!(has_ctb, "LLC should have check-the-box test");
    }

    #[test]
    fn test_section_754_election_test() {
        let input = default_input(); // has Section754 in tax_elections
        let result = analyze_us_fund_structure(&input).unwrap();
        let has_754 = result
            .tax_analysis
            .structure_specific_tests
            .iter()
            .any(|t| t.test_name.contains("Section 754"));
        assert!(has_754, "Should have Section 754 test when elected");
    }

    // -----------------------------------------------------------------------
    // ERISA analysis tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_erisa_below_threshold() {
        let input = default_input(); // TaxExempt 25% (just at threshold)
        let result = analyze_us_fund_structure(&input).unwrap();
        // TaxExempt 25% >= 25%
        assert!(
            result.erisa_analysis.plan_asset_risk.contains("High"),
            "25% TaxExempt should trigger high plan asset risk"
        );
    }

    #[test]
    fn test_erisa_low_risk() {
        let mut input = default_input();
        input.investor_types = vec![
            InvestorType {
                category: "Taxable".into(),
                allocation_pct: dec!(0.90),
            },
            InvestorType {
                category: "TaxExempt".into(),
                allocation_pct: dec!(0.10),
            },
        ];
        let result = analyze_us_fund_structure(&input).unwrap();
        assert!(
            result.erisa_analysis.plan_asset_risk.contains("Low"),
            "10% TaxExempt should be low risk"
        );
    }

    #[test]
    fn test_erisa_moderate_risk() {
        let mut input = default_input();
        input.investor_types = vec![
            InvestorType {
                category: "Taxable".into(),
                allocation_pct: dec!(0.80),
            },
            InvestorType {
                category: "TaxExempt".into(),
                allocation_pct: dec!(0.20),
            },
        ];
        let result = analyze_us_fund_structure(&input).unwrap();
        assert!(
            result.erisa_analysis.plan_asset_risk.contains("Moderate"),
            "20% TaxExempt should be moderate risk"
        );
    }

    #[test]
    fn test_vcoc_eligible_for_lp() {
        let input = default_input();
        let result = analyze_us_fund_structure(&input).unwrap();
        assert!(result.erisa_analysis.vcoc_eligible);
    }

    #[test]
    fn test_reoc_eligible_for_reit() {
        let input = reit_input();
        let result = analyze_us_fund_structure(&input).unwrap();
        assert!(result.erisa_analysis.reoc_eligible);
    }

    #[test]
    fn test_blocker_recommended_for_mlp() {
        let mut input = mlp_input();
        input.investor_types = vec![
            InvestorType {
                category: "Taxable".into(),
                allocation_pct: dec!(0.50),
            },
            InvestorType {
                category: "ERISA".into(),
                allocation_pct: dec!(0.50),
            },
        ];
        let result = analyze_us_fund_structure(&input).unwrap();
        assert!(
            result.erisa_analysis.blocker_recommended,
            "MLP with ERISA investors should recommend blocker"
        );
    }

    // -----------------------------------------------------------------------
    // State analysis tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_delaware_formation_cost() {
        let input = default_input();
        let result = analyze_us_fund_structure(&input).unwrap();
        assert_eq!(result.state_analysis.formation_cost, dec!(200));
    }

    #[test]
    fn test_california_higher_annual_cost() {
        let input = llc_input(); // California
        let result = analyze_us_fund_structure(&input).unwrap();
        assert_eq!(result.state_analysis.annual_cost, dec!(800));
    }

    #[test]
    fn test_texas_no_franchise_tax() {
        let mut input = default_input();
        input.state_of_formation = "Texas".into();
        let result = analyze_us_fund_structure(&input).unwrap();
        assert_eq!(result.state_analysis.franchise_tax, Decimal::ZERO);
    }

    // -----------------------------------------------------------------------
    // Investor suitability tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_taxable_always_suitable() {
        let input = default_input();
        let result = analyze_us_fund_structure(&input).unwrap();
        let taxable = result
            .investor_suitability
            .iter()
            .find(|s| s.category == "Taxable");
        assert!(taxable.is_some());
        assert!(taxable.unwrap().suitable);
    }

    #[test]
    fn test_tax_exempt_mlp_unsuitable() {
        let input = mlp_input();
        let result = analyze_us_fund_structure(&input).unwrap();
        let te = result
            .investor_suitability
            .iter()
            .find(|s| s.category == "TaxExempt");
        assert!(te.is_some());
        assert!(
            !te.unwrap().suitable,
            "TaxExempt investors in MLP should be unsuitable"
        );
    }

    #[test]
    fn test_foreign_mlp_unsuitable() {
        let input = mlp_input();
        let result = analyze_us_fund_structure(&input).unwrap();
        let foreign = result
            .investor_suitability
            .iter()
            .find(|s| s.category == "Foreign");
        assert!(foreign.is_some());
        assert!(
            !foreign.unwrap().suitable,
            "Foreign investors in MLP should be unsuitable"
        );
    }

    #[test]
    fn test_foreign_has_eci_warning_in_lp() {
        let input = default_input();
        let result = analyze_us_fund_structure(&input).unwrap();
        let foreign = result
            .investor_suitability
            .iter()
            .find(|s| s.category == "Foreign");
        assert!(foreign.is_some());
        assert!(
            foreign.unwrap().issues.iter().any(|i| i.contains("ECI")),
            "Foreign investors in LP should have ECI warning"
        );
    }

    // -----------------------------------------------------------------------
    // Recommendations and warnings tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_recommendations_not_empty() {
        let input = default_input();
        let result = analyze_us_fund_structure(&input).unwrap();
        assert!(!result.recommendations.is_empty());
    }

    #[test]
    fn test_qoz_short_term_warning() {
        let mut input = qoz_input();
        input.fund_term_years = 7;
        let result = analyze_us_fund_structure(&input).unwrap();
        assert!(
            result.warnings.iter().any(|w| w.contains("10 years")),
            "Should warn about short QOZ term"
        );
    }

    #[test]
    fn test_mlp_eci_warning_for_foreign() {
        let input = mlp_input();
        let result = analyze_us_fund_structure(&input).unwrap();
        assert!(
            result.warnings.iter().any(|w| w.contains("ECI")),
            "MLP with foreign investors should have ECI warning"
        );
    }

    #[test]
    fn test_mlp_ubti_warning_for_tax_exempt() {
        let input = mlp_input();
        let result = analyze_us_fund_structure(&input).unwrap();
        assert!(
            result.warnings.iter().any(|w| w.contains("UBTI")),
            "MLP with tax-exempt investors should have UBTI warning"
        );
    }

    // -----------------------------------------------------------------------
    // Validation error tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_invalid_fund_size() {
        let mut input = default_input();
        input.fund_size = dec!(-100);
        let result = analyze_us_fund_structure(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_management_fee() {
        let mut input = default_input();
        input.management_fee_rate = dec!(0.15);
        let result = analyze_us_fund_structure(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_structure_type() {
        let mut input = default_input();
        input.structure_type = "InvalidType".into();
        let result = analyze_us_fund_structure(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_investor_category() {
        let mut input = default_input();
        input.investor_types = vec![InvestorType {
            category: "BadCategory".into(),
            allocation_pct: dec!(1.0),
        }];
        let result = analyze_us_fund_structure(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_allocation_pct_not_sum_to_one() {
        let mut input = default_input();
        input.investor_types = vec![
            InvestorType {
                category: "Taxable".into(),
                allocation_pct: dec!(0.30),
            },
            InvestorType {
                category: "TaxExempt".into(),
                allocation_pct: dec!(0.30),
            },
        ];
        let result = analyze_us_fund_structure(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_zero_fund_term() {
        let mut input = default_input();
        input.fund_term_years = 0;
        let result = analyze_us_fund_structure(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_distribution_frequency() {
        let mut input = default_input();
        input.distribution_frequency = "Monthly".into();
        let result = analyze_us_fund_structure(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_investor_types() {
        let mut input = default_input();
        input.investor_types = vec![];
        let result = analyze_us_fund_structure(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_carried_interest_rate_out_of_range() {
        let mut input = default_input();
        input.carried_interest_rate = dec!(1.5);
        let result = analyze_us_fund_structure(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_preferred_return_out_of_range() {
        let mut input = default_input();
        input.preferred_return = dec!(-0.1);
        let result = analyze_us_fund_structure(&input);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Compound helper
    // -----------------------------------------------------------------------

    #[test]
    fn test_compound_basic() {
        // (1.10)^3 = 1.331
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
    // Multi-structure tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_all_structure_types_produce_output() {
        for st in VALID_STRUCTURES {
            let mut input = default_input();
            input.structure_type = st.to_string();
            if *st == "QOZ" {
                // QOZ needs single taxable investor for simplicity
                input.investor_types = vec![InvestorType {
                    category: "Taxable".into(),
                    allocation_pct: dec!(1.0),
                }];
            }
            let result = analyze_us_fund_structure(&input);
            assert!(
                result.is_ok(),
                "Structure '{}' should produce valid output, got: {:?}",
                st,
                result.err()
            );
            assert_eq!(result.unwrap().structure_type, *st);
        }
    }

    #[test]
    fn test_erisa_investor_plan_asset() {
        let mut input = default_input();
        input.investor_types = vec![
            InvestorType {
                category: "Taxable".into(),
                allocation_pct: dec!(0.50),
            },
            InvestorType {
                category: "ERISA".into(),
                allocation_pct: dec!(0.50),
            },
        ];
        let result = analyze_us_fund_structure(&input).unwrap();
        assert!(
            result.erisa_analysis.benefit_plan_investor_pct >= dec!(0.50),
            "Benefit plan investor pct should be at least 50%"
        );
        assert!(result.erisa_analysis.plan_asset_risk.contains("High"));
    }

    #[test]
    fn test_net_return_positive_when_fund_returns_well() {
        let input = default_input();
        let result = analyze_us_fund_structure(&input).unwrap();
        assert!(
            result.fund_economics.net_return_to_lps > Decimal::ZERO,
            "LP net return should be positive with 15% annual return"
        );
    }
}
