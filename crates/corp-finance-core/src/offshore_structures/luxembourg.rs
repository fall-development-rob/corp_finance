use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LuxFundInput {
    pub fund_name: String,
    /// "SICAV_SIF", "SICAV_RAIF", "SCSp", "ICAV", "QIAIF", "Section110"
    pub structure_type: String,
    /// "Luxembourg" or "Ireland"
    pub domicile: String,
    pub fund_size: Decimal,
    pub management_fee_rate: Decimal,
    pub carried_interest_rate: Decimal,
    /// None for open-ended
    pub fund_term_years: Option<u32>,
    /// e.g. ["EU_Institutional", "US_TaxExempt", "Asian_SWF"]
    pub target_investor_base: Vec<String>,
    pub aifmd_full_scope: bool,
    pub ucits_compliant: bool,
    /// Institutional SIF/RAIF can be exempt from subscription tax
    pub subscription_tax_exempt: bool,
    pub management_company_location: String,
}

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructureComparison {
    pub regulatory_approval_needed: bool,
    pub approval_timeline_weeks: u32,
    pub minimum_investment: Decimal,
    pub diversification_required: bool,
    pub suitable_for: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreatyBenefit {
    pub jurisdiction: String,
    pub wht_reduction_from: Decimal,
    pub wht_reduction_to: Decimal,
    pub treaty_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LuxTaxAnalysis {
    pub subscription_tax_rate: Decimal,
    pub subscription_tax_annual: Decimal,
    pub fund_level_income_tax: Decimal,
    pub distribution_wht_rate: Decimal,
    pub treaty_benefits: Vec<TreatyBenefit>,
    pub effective_tax_drag: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AifmdAnalysis {
    pub full_scope_required: bool,
    pub aifm_capital_required: Decimal,
    pub passport_jurisdictions: Vec<String>,
    pub depositary_required: bool,
    pub risk_management_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UcitsAnalysis {
    pub eligible: bool,
    pub diversification_compliant: bool,
    pub leverage_compliant: bool,
    pub kid_required: bool,
    pub distribution_countries: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationalCosts {
    pub central_admin_annual: Decimal,
    pub transfer_agent_annual: Decimal,
    pub depositary_annual: Decimal,
    pub regulatory_fees_annual: Decimal,
    pub legal_ongoing: Decimal,
    pub total_annual_cost: Decimal,
    pub total_expense_ratio: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestorAccess {
    pub eu_passport: bool,
    pub us_access_method: String,
    pub asia_access: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LuxFundOutput {
    pub structure_type: String,
    pub domicile: String,
    pub structure_comparison: StructureComparison,
    pub tax_analysis: LuxTaxAnalysis,
    pub aifmd: AifmdAnalysis,
    pub ucits: Option<UcitsAnalysis>,
    pub operational: OperationalCosts,
    pub investor_access: InvestorAccess,
    pub recommendations: Vec<String>,
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub fn analyze_lux_structure(input: &LuxFundInput) -> CorpFinanceResult<LuxFundOutput> {
    validate_input(input)?;

    let mut warnings: Vec<String> = Vec::new();
    let mut recommendations: Vec<String> = Vec::new();

    // ------------------------------------------------------------------
    // 1. Structure Comparison
    // ------------------------------------------------------------------
    let structure_comparison = build_structure_comparison(
        &input.structure_type,
        &input.domicile,
        &mut recommendations,
        &mut warnings,
    )?;

    // ------------------------------------------------------------------
    // 2. Tax Analysis
    // ------------------------------------------------------------------
    let tax_analysis = build_tax_analysis(input, &mut recommendations, &mut warnings);

    // ------------------------------------------------------------------
    // 3. AIFMD Passport Analysis
    // ------------------------------------------------------------------
    let aifmd = build_aifmd_analysis(input, &mut recommendations, &mut warnings);

    // ------------------------------------------------------------------
    // 4. UCITS Analysis (if applicable)
    // ------------------------------------------------------------------
    let ucits = if input.ucits_compliant {
        Some(build_ucits_analysis(
            input,
            &mut recommendations,
            &mut warnings,
        ))
    } else {
        None
    };

    // ------------------------------------------------------------------
    // 5. Operational Setup & Costs
    // ------------------------------------------------------------------
    let operational = build_operational_costs(input, &aifmd, &mut recommendations);

    // ------------------------------------------------------------------
    // 6. Investor Access
    // ------------------------------------------------------------------
    let investor_access = build_investor_access(input, &aifmd, &ucits, &mut recommendations);

    // ------------------------------------------------------------------
    // 7. Final recommendations
    // ------------------------------------------------------------------
    if operational.total_expense_ratio > dec!(0.01) {
        warnings.push(format!(
            "Total expense ratio of {:.2}% is above the typical 1% threshold \
             for institutional funds",
            operational.total_expense_ratio * dec!(100)
        ));
    }

    if input.ucits_compliant && input.aifmd_full_scope {
        recommendations.push(
            "Fund is both UCITS and AIFMD compliant — consider whether \
             dual compliance is necessary or if one regime is sufficient"
                .to_string(),
        );
    }

    if input.domicile == "Luxembourg"
        && input
            .target_investor_base
            .contains(&"US_TaxExempt".to_string())
    {
        recommendations.push(
            "For US tax-exempt investors, consider a parallel Cayman \
             or Delaware blocker structure alongside Luxembourg vehicle"
                .to_string(),
        );
    }

    Ok(LuxFundOutput {
        structure_type: input.structure_type.clone(),
        domicile: input.domicile.clone(),
        structure_comparison,
        tax_analysis,
        aifmd,
        ucits,
        operational,
        investor_access,
        recommendations,
        warnings,
    })
}

// ---------------------------------------------------------------------------
// Helpers — Structure Comparison
// ---------------------------------------------------------------------------

fn build_structure_comparison(
    structure_type: &str,
    domicile: &str,
    recommendations: &mut Vec<String>,
    warnings: &mut Vec<String>,
) -> CorpFinanceResult<StructureComparison> {
    let (reg_approval, timeline_weeks, min_invest, diversification, suitable) = match structure_type
    {
        "SICAV_SIF" => {
            if domicile != "Luxembourg" {
                warnings.push(
                    "SICAV-SIF is a Luxembourg-specific structure; \
                         domicile should be Luxembourg"
                        .to_string(),
                );
            }
            (
                true,
                52u32,
                dec!(1_250_000),
                true,
                vec![
                    "PE".to_string(),
                    "RealEstate".to_string(),
                    "Infrastructure".to_string(),
                    "Hedge".to_string(),
                    "Credit".to_string(),
                ],
            )
        }
        "SICAV_RAIF" => {
            if domicile != "Luxembourg" {
                warnings.push(
                    "SICAV-RAIF is a Luxembourg-specific structure; \
                         domicile should be Luxembourg"
                        .to_string(),
                );
            }
            recommendations.push(
                "RAIF advantage: no CSSF approval needed — regulated \
                     through the AIFM; faster launch (2-4 weeks)"
                    .to_string(),
            );
            (
                false,
                4,
                dec!(1_250_000),
                true,
                vec![
                    "PE".to_string(),
                    "RealEstate".to_string(),
                    "Infrastructure".to_string(),
                    "Hedge".to_string(),
                    "Credit".to_string(),
                    "FundOfFunds".to_string(),
                ],
            )
        }
        "SCSp" => {
            if domicile != "Luxembourg" {
                warnings.push(
                    "SCSp is a Luxembourg-specific partnership form; \
                         domicile should be Luxembourg"
                        .to_string(),
                );
            }
            recommendations.push(
                "SCSp is tax transparent — no entity-level tax. \
                     Ideal for PE/VC where investors want pass-through"
                    .to_string(),
            );
            (
                false,
                2,
                Decimal::ZERO,
                false,
                vec![
                    "PE".to_string(),
                    "VC".to_string(),
                    "RealEstate".to_string(),
                    "Infrastructure".to_string(),
                ],
            )
        }
        "ICAV" => {
            if domicile != "Ireland" {
                warnings.push(
                    "ICAV is an Irish-specific vehicle; domicile \
                         should be Ireland"
                        .to_string(),
                );
            }
            recommendations.push(
                "ICAV advantage: no AGM requirement, check-the-box \
                     election for US tax purposes"
                    .to_string(),
            );
            (
                true,
                12,
                Decimal::ZERO,
                true,
                vec![
                    "Hedge".to_string(),
                    "PE".to_string(),
                    "Credit".to_string(),
                    "FundOfFunds".to_string(),
                ],
            )
        }
        "QIAIF" => {
            if domicile != "Ireland" {
                warnings.push(
                    "QIAIF is an Irish-specific structure; domicile \
                         should be Ireland"
                        .to_string(),
                );
            }
            (
                true,
                4,
                dec!(100_000),
                false,
                vec![
                    "Hedge".to_string(),
                    "PE".to_string(),
                    "RealEstate".to_string(),
                    "Credit".to_string(),
                    "Infrastructure".to_string(),
                ],
            )
        }
        "Section110" => {
            if domicile != "Ireland" {
                warnings.push(
                    "Section 110 is an Irish-specific SPV structure; \
                         domicile should be Ireland"
                        .to_string(),
                );
            }
            recommendations.push(
                "Section 110 is designed for securitization/SPV — \
                     not a traditional fund structure"
                    .to_string(),
            );
            (
                false,
                2,
                Decimal::ZERO,
                false,
                vec!["Securitization".to_string(), "Credit".to_string()],
            )
        }
        other => {
            return Err(CorpFinanceError::InvalidInput {
                field: "structure_type".into(),
                reason: format!(
                    "Unknown structure type '{}'. Expected one of: \
                         SICAV_SIF, SICAV_RAIF, SCSp, ICAV, QIAIF, Section110",
                    other
                ),
            });
        }
    };

    Ok(StructureComparison {
        regulatory_approval_needed: reg_approval,
        approval_timeline_weeks: timeline_weeks,
        minimum_investment: min_invest,
        diversification_required: diversification,
        suitable_for: suitable,
    })
}

// ---------------------------------------------------------------------------
// Helpers — Tax Analysis
// ---------------------------------------------------------------------------

fn build_tax_analysis(
    input: &LuxFundInput,
    recommendations: &mut Vec<String>,
    warnings: &mut Vec<String>,
) -> LuxTaxAnalysis {
    let (sub_tax_rate, fund_income_tax, dist_wht_rate) = match input.domicile.as_str() {
        "Luxembourg" => {
            let sub_rate = calculate_lux_subscription_tax(
                &input.structure_type,
                input.subscription_tax_exempt,
            );

            // Luxembourg funds: no income tax at fund level for SIF/RAIF/SCSp
            let income_tax = match input.structure_type.as_str() {
                "SCSp" => Decimal::ZERO,
                "SICAV_SIF" | "SICAV_RAIF" => Decimal::ZERO,
                _ => Decimal::ZERO,
            };

            // Luxembourg has no withholding tax on distributions to non-residents
            let wht = Decimal::ZERO;

            if sub_rate > Decimal::ZERO {
                recommendations.push(format!(
                    "Subscription tax (taxe d'abonnement) applies at {:.2}% \
                     per annum on NAV",
                    sub_rate * dec!(100)
                ));
            }

            (sub_rate, income_tax, wht)
        }
        "Ireland" => {
            // Ireland: 0% tax at fund level for qualifying investment funds
            let income_tax = Decimal::ZERO;

            // Irish funds: 25% exit tax on distributions to Irish residents
            // but 0% for non-resident investors (with proper declarations)
            let wht = if input
                .target_investor_base
                .iter()
                .all(|inv| inv != "Irish_Resident")
            {
                Decimal::ZERO
            } else {
                dec!(0.25)
            };

            if wht > Decimal::ZERO {
                warnings.push(
                    "Irish exit tax of 25% applies to distributions to \
                     Irish-resident investors; non-residents are exempt \
                     with proper declarations"
                        .to_string(),
                );
            }

            (Decimal::ZERO, income_tax, wht)
        }
        _ => (Decimal::ZERO, Decimal::ZERO, Decimal::ZERO),
    };

    let sub_tax_annual = input.fund_size * sub_tax_rate;

    // Treaty benefits
    let treaty_benefits = build_treaty_benefits(
        &input.domicile,
        &input.target_investor_base,
        recommendations,
    );

    // Effective tax drag = subscription tax + income tax effect on fund returns
    // For most Luxembourg/Ireland structures, the main drag is subscription tax
    let effective_tax_drag = sub_tax_rate + fund_income_tax;

    if effective_tax_drag == Decimal::ZERO {
        recommendations.push(format!(
            "{} {} structure is effectively tax-neutral at the fund level",
            input.domicile, input.structure_type
        ));
    }

    LuxTaxAnalysis {
        subscription_tax_rate: sub_tax_rate,
        subscription_tax_annual: sub_tax_annual,
        fund_level_income_tax: fund_income_tax,
        distribution_wht_rate: dist_wht_rate,
        treaty_benefits,
        effective_tax_drag,
    }
}

/// Calculate Luxembourg subscription tax (taxe d'abonnement)
fn calculate_lux_subscription_tax(structure_type: &str, subscription_tax_exempt: bool) -> Decimal {
    match structure_type {
        // SCSp is a partnership — no subscription tax
        "SCSp" => Decimal::ZERO,
        // SIF/RAIF: 1bp for institutional, 5bp standard
        "SICAV_SIF" | "SICAV_RAIF" => {
            if subscription_tax_exempt {
                Decimal::ZERO
            } else {
                dec!(0.0001) // 1bp institutional rate
            }
        }
        _ => dec!(0.0005), // 5bp standard rate
    }
}

fn build_treaty_benefits(
    domicile: &str,
    target_investors: &[String],
    recommendations: &mut Vec<String>,
) -> Vec<TreatyBenefit> {
    let mut benefits = Vec::new();

    match domicile {
        "Luxembourg" => {
            // Luxembourg has 80+ DTTs
            benefits.push(TreatyBenefit {
                jurisdiction: "United States".to_string(),
                wht_reduction_from: dec!(0.30),
                wht_reduction_to: dec!(0.15),
                treaty_type: "Luxembourg-US DTT".to_string(),
            });
            benefits.push(TreatyBenefit {
                jurisdiction: "Germany".to_string(),
                wht_reduction_from: dec!(0.2638),
                wht_reduction_to: dec!(0.05),
                treaty_type: "EU Parent-Subsidiary Directive".to_string(),
            });
            benefits.push(TreatyBenefit {
                jurisdiction: "United Kingdom".to_string(),
                wht_reduction_from: dec!(0.20),
                wht_reduction_to: Decimal::ZERO,
                treaty_type: "Luxembourg-UK DTT".to_string(),
            });
            benefits.push(TreatyBenefit {
                jurisdiction: "China".to_string(),
                wht_reduction_from: dec!(0.10),
                wht_reduction_to: dec!(0.05),
                treaty_type: "Luxembourg-China DTT".to_string(),
            });

            if target_investors.iter().any(|i| i.contains("EU")) {
                recommendations.push(
                    "Luxembourg's 80+ DTTs and EU Parent-Subsidiary \
                     Directive provide favorable tax treaty access \
                     for EU investors"
                        .to_string(),
                );
            }
        }
        "Ireland" => {
            // Ireland has 70+ DTTs
            benefits.push(TreatyBenefit {
                jurisdiction: "United States".to_string(),
                wht_reduction_from: dec!(0.30),
                wht_reduction_to: dec!(0.15),
                treaty_type: "Ireland-US DTT".to_string(),
            });
            benefits.push(TreatyBenefit {
                jurisdiction: "Germany".to_string(),
                wht_reduction_from: dec!(0.2638),
                wht_reduction_to: dec!(0.05),
                treaty_type: "EU Parent-Subsidiary Directive".to_string(),
            });
            benefits.push(TreatyBenefit {
                jurisdiction: "United Kingdom".to_string(),
                wht_reduction_from: dec!(0.20),
                wht_reduction_to: Decimal::ZERO,
                treaty_type: "Ireland-UK DTT".to_string(),
            });

            if target_investors.iter().any(|i| i.contains("US")) {
                recommendations.push(
                    "Ireland's DTT with the US and check-the-box \
                     election makes ICAV/QIAIF attractive for \
                     US investors"
                        .to_string(),
                );
            }
        }
        _ => {}
    }

    benefits
}

// ---------------------------------------------------------------------------
// Helpers — AIFMD
// ---------------------------------------------------------------------------

fn build_aifmd_analysis(
    input: &LuxFundInput,
    recommendations: &mut Vec<String>,
    warnings: &mut Vec<String>,
) -> AifmdAnalysis {
    let full_scope = input.aifmd_full_scope;

    // AIFM capital requirement:
    // base = EUR 125,000
    // + 0.02% of AUM over EUR 250M (capped at EUR 10M total)
    let aifm_capital = if full_scope {
        let base = dec!(125_000);
        let excess_aum = (input.fund_size - dec!(250_000_000)).max(Decimal::ZERO);
        let additional = excess_aum * dec!(0.0002);
        let total = base + additional;
        total.min(dec!(10_000_000))
    } else {
        Decimal::ZERO
    };

    // EU/EEA passport jurisdictions (all 27 EU + 3 EEA)
    let passport_jurisdictions = if full_scope {
        vec![
            "Austria".to_string(),
            "Belgium".to_string(),
            "Bulgaria".to_string(),
            "Croatia".to_string(),
            "Cyprus".to_string(),
            "Czech Republic".to_string(),
            "Denmark".to_string(),
            "Estonia".to_string(),
            "Finland".to_string(),
            "France".to_string(),
            "Germany".to_string(),
            "Greece".to_string(),
            "Hungary".to_string(),
            "Ireland".to_string(),
            "Italy".to_string(),
            "Latvia".to_string(),
            "Lithuania".to_string(),
            "Luxembourg".to_string(),
            "Malta".to_string(),
            "Netherlands".to_string(),
            "Poland".to_string(),
            "Portugal".to_string(),
            "Romania".to_string(),
            "Slovakia".to_string(),
            "Slovenia".to_string(),
            "Spain".to_string(),
            "Sweden".to_string(),
            "Iceland".to_string(),
            "Liechtenstein".to_string(),
            "Norway".to_string(),
        ]
    } else {
        recommendations.push(
            "Without AIFMD full-scope authorization, marketing is \
             limited to reverse solicitation or national private \
             placement regimes"
                .to_string(),
        );
        vec![]
    };

    let depositary_required = full_scope;
    let risk_management_required = full_scope;

    if full_scope {
        recommendations.push(
            "AIFMD requires: depositary appointment, risk management \
             function (may be delegated), liquidity management policy, \
             and remuneration policy"
                .to_string(),
        );

        if input.fund_size > dec!(500_000_000) {
            warnings.push(format!(
                "AIFM capital requirement: EUR {:.0} (base EUR 125k + \
                 0.02% of AUM over EUR 250M, capped at EUR 10M)",
                aifm_capital
            ));
        }
    }

    // SCSp under EUR 500M may be exempt from full-scope AIFMD
    if input.structure_type == "SCSp" && input.fund_size < dec!(500_000_000) && full_scope {
        recommendations.push(
            "SCSp under EUR 500M may qualify for AIFMD registration \
             (lighter regime) rather than full authorization"
                .to_string(),
        );
    }

    AifmdAnalysis {
        full_scope_required: full_scope,
        aifm_capital_required: aifm_capital,
        passport_jurisdictions,
        depositary_required,
        risk_management_required,
    }
}

// ---------------------------------------------------------------------------
// Helpers — UCITS
// ---------------------------------------------------------------------------

fn build_ucits_analysis(
    input: &LuxFundInput,
    recommendations: &mut Vec<String>,
    warnings: &mut Vec<String>,
) -> UcitsAnalysis {
    // UCITS eligibility check
    let eligible = matches!(
        input.structure_type.as_str(),
        "SICAV_SIF" | "SICAV_RAIF" | "ICAV"
    );

    if !eligible {
        warnings.push(format!(
            "{} structure is not eligible for UCITS; \
             UCITS requires SICAV or ICAV-type vehicle",
            input.structure_type
        ));
    }

    // 5/10/40 diversification rule
    let diversification_compliant = eligible;

    // Global exposure leverage limit: <= 2x NAV
    let leverage_compliant = eligible;

    // KID/KIID required for all UCITS
    let kid_required = eligible;

    if eligible {
        recommendations.push(
            "UCITS provides passport to distribute to retail investors \
             across all 27 EU member states + EEA"
                .to_string(),
        );
        recommendations.push(
            "UCITS 5/10/40 rule: max 5% in any single issuer, 10% with \
             permission, total of 10%+ exposures cannot exceed 40%"
                .to_string(),
        );
    }

    // Distribution countries = all EU/EEA for UCITS
    let distribution_countries = if eligible {
        vec![
            "All EU Member States".to_string(),
            "EEA (Iceland, Liechtenstein, Norway)".to_string(),
            "UK (with FCA recognition)".to_string(),
            "Singapore (MAS recognized)".to_string(),
            "Hong Kong (SFC recognized)".to_string(),
            "Switzerland (FINMA recognized)".to_string(),
        ]
    } else {
        vec![]
    };

    UcitsAnalysis {
        eligible,
        diversification_compliant,
        leverage_compliant,
        kid_required,
        distribution_countries,
    }
}

// ---------------------------------------------------------------------------
// Helpers — Operational Costs
// ---------------------------------------------------------------------------

fn build_operational_costs(
    input: &LuxFundInput,
    aifmd: &AifmdAnalysis,
    recommendations: &mut Vec<String>,
) -> OperationalCosts {
    let (central_admin, transfer_agent, depositary, reg_fees, legal) = match input.domicile.as_str()
    {
        "Luxembourg" => {
            let admin = estimate_lux_admin_cost(input.fund_size);
            let ta = estimate_transfer_agent_cost(input.fund_size);
            let dep = if aifmd.depositary_required {
                estimate_depositary_cost(input.fund_size)
            } else {
                Decimal::ZERO
            };
            // CSSF annual fee
            let reg = match input.structure_type.as_str() {
                "SICAV_SIF" => dec!(4_000),
                "SICAV_RAIF" => dec!(3_500),
                "SCSp" => dec!(2_500),
                _ => dec!(3_000),
            };
            let legal = estimate_lux_legal_cost(input.fund_size);
            (admin, ta, dep, reg, legal)
        }
        "Ireland" => {
            let admin = estimate_ireland_admin_cost(input.fund_size);
            let ta = estimate_transfer_agent_cost(input.fund_size);
            let dep = if aifmd.depositary_required {
                estimate_depositary_cost(input.fund_size)
            } else {
                Decimal::ZERO
            };
            // Central Bank of Ireland annual fee
            let reg = match input.structure_type.as_str() {
                "ICAV" => dec!(3_800),
                "QIAIF" => dec!(3_200),
                "Section110" => dec!(1_500),
                _ => dec!(3_000),
            };
            let legal = estimate_ireland_legal_cost(input.fund_size);
            (admin, ta, dep, reg, legal)
        }
        _ => (
            Decimal::ZERO,
            Decimal::ZERO,
            Decimal::ZERO,
            Decimal::ZERO,
            Decimal::ZERO,
        ),
    };

    let total = central_admin + transfer_agent + depositary + reg_fees + legal;
    let ter = if input.fund_size > Decimal::ZERO {
        total / input.fund_size
    } else {
        Decimal::ZERO
    };

    if depositary == Decimal::ZERO && aifmd.full_scope_required {
        recommendations.push(
            "Depositary is required under AIFMD — ensure appointment \
             before launch"
                .to_string(),
        );
    }

    OperationalCosts {
        central_admin_annual: central_admin,
        transfer_agent_annual: transfer_agent,
        depositary_annual: depositary,
        regulatory_fees_annual: reg_fees,
        legal_ongoing: legal,
        total_annual_cost: total,
        total_expense_ratio: ter,
    }
}

// ---------------------------------------------------------------------------
// Helpers — Investor Access
// ---------------------------------------------------------------------------

fn build_investor_access(
    input: &LuxFundInput,
    aifmd: &AifmdAnalysis,
    ucits: &Option<UcitsAnalysis>,
    recommendations: &mut Vec<String>,
) -> InvestorAccess {
    let eu_passport = aifmd.full_scope_required || ucits.as_ref().is_some_and(|u| u.eligible);

    let us_access_method = if input.target_investor_base.iter().any(|i| i.contains("US")) {
        if input.domicile == "Ireland" && matches!(input.structure_type.as_str(), "ICAV" | "QIAIF")
        {
            "Check-the-box election + Reg D/Reg S private placement".to_string()
        } else {
            "Reg D/Reg S private placement (no US public offering)".to_string()
        }
    } else {
        "Not targeting US investors".to_string()
    };

    let mut asia_access = Vec::new();
    if input
        .target_investor_base
        .iter()
        .any(|i| i.contains("Asian") || i.contains("SWF"))
    {
        if ucits.as_ref().is_some_and(|u| u.eligible) {
            asia_access.push("Singapore: MAS recognized UCITS scheme".to_string());
            asia_access.push("Hong Kong: SFC recognized UCITS scheme".to_string());
        } else {
            asia_access.push("Singapore: Restricted scheme (accredited investors)".to_string());
            asia_access.push("Hong Kong: Professional investor exemption".to_string());
        }
        asia_access.push("Japan: QII exemption".to_string());
    }

    if eu_passport {
        recommendations.push(
            "EU passport enables marketing to professional investors \
             across all 27 EU member states + EEA without additional \
             registration"
                .to_string(),
        );
    }

    InvestorAccess {
        eu_passport,
        us_access_method,
        asia_access,
    }
}

// ---------------------------------------------------------------------------
// Helpers — Cost Estimation
// ---------------------------------------------------------------------------

fn estimate_lux_admin_cost(fund_size: Decimal) -> Decimal {
    if fund_size >= dec!(1_000_000_000) {
        dec!(300_000)
    } else if fund_size >= dec!(500_000_000) {
        dec!(200_000)
    } else if fund_size >= dec!(100_000_000) {
        dec!(125_000)
    } else {
        dec!(80_000)
    }
}

fn estimate_ireland_admin_cost(fund_size: Decimal) -> Decimal {
    if fund_size >= dec!(1_000_000_000) {
        dec!(275_000)
    } else if fund_size >= dec!(500_000_000) {
        dec!(180_000)
    } else if fund_size >= dec!(100_000_000) {
        dec!(110_000)
    } else {
        dec!(70_000)
    }
}

fn estimate_transfer_agent_cost(fund_size: Decimal) -> Decimal {
    if fund_size >= dec!(1_000_000_000) {
        dec!(100_000)
    } else if fund_size >= dec!(500_000_000) {
        dec!(75_000)
    } else if fund_size >= dec!(100_000_000) {
        dec!(50_000)
    } else {
        dec!(35_000)
    }
}

fn estimate_depositary_cost(fund_size: Decimal) -> Decimal {
    // Depositary fees: typically 1-5bps of AUM, minimum ~EUR 50k
    let bps_fee = fund_size * dec!(0.0002); // 2bps
    bps_fee.max(dec!(50_000)).min(dec!(500_000))
}

fn estimate_lux_legal_cost(fund_size: Decimal) -> Decimal {
    if fund_size >= dec!(500_000_000) {
        dec!(100_000)
    } else if fund_size >= dec!(100_000_000) {
        dec!(65_000)
    } else {
        dec!(40_000)
    }
}

fn estimate_ireland_legal_cost(fund_size: Decimal) -> Decimal {
    if fund_size >= dec!(500_000_000) {
        dec!(90_000)
    } else if fund_size >= dec!(100_000_000) {
        dec!(55_000)
    } else {
        dec!(35_000)
    }
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_input(input: &LuxFundInput) -> CorpFinanceResult<()> {
    if input.fund_name.trim().is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "fund_name".into(),
            reason: "Fund name cannot be empty".into(),
        });
    }

    let valid_structures = [
        "SICAV_SIF",
        "SICAV_RAIF",
        "SCSp",
        "ICAV",
        "QIAIF",
        "Section110",
    ];
    if !valid_structures.contains(&input.structure_type.as_str()) {
        return Err(CorpFinanceError::InvalidInput {
            field: "structure_type".into(),
            reason: format!(
                "Unknown structure type '{}'. Valid: {:?}",
                input.structure_type, valid_structures
            ),
        });
    }

    let valid_domiciles = ["Luxembourg", "Ireland"];
    if !valid_domiciles.contains(&input.domicile.as_str()) {
        return Err(CorpFinanceError::InvalidInput {
            field: "domicile".into(),
            reason: format!(
                "Unknown domicile '{}'. Valid: {:?}",
                input.domicile, valid_domiciles
            ),
        });
    }

    if input.fund_size <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "fund_size".into(),
            reason: "Fund size must be greater than zero".into(),
        });
    }

    if input.management_fee_rate < Decimal::ZERO || input.management_fee_rate > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "management_fee_rate".into(),
            reason: "Management fee rate must be between 0 and 1".into(),
        });
    }

    if input.carried_interest_rate < Decimal::ZERO || input.carried_interest_rate > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "carried_interest_rate".into(),
            reason: "Carried interest rate must be between 0 and 1".into(),
        });
    }

    // Validate structure-domicile compatibility
    match input.structure_type.as_str() {
        "SICAV_SIF" | "SICAV_RAIF" | "SCSp" => {
            if input.domicile != "Luxembourg" {
                return Err(CorpFinanceError::InvalidInput {
                    field: "domicile".into(),
                    reason: format!("{} requires Luxembourg domicile", input.structure_type),
                });
            }
        }
        "ICAV" | "QIAIF" | "Section110" => {
            if input.domicile != "Ireland" {
                return Err(CorpFinanceError::InvalidInput {
                    field: "domicile".into(),
                    reason: format!("{} requires Ireland domicile", input.structure_type),
                });
            }
        }
        _ => {}
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

    fn lux_sif_input() -> LuxFundInput {
        LuxFundInput {
            fund_name: "European Growth SIF".to_string(),
            structure_type: "SICAV_SIF".to_string(),
            domicile: "Luxembourg".to_string(),
            fund_size: dec!(500_000_000),
            management_fee_rate: dec!(0.015),
            carried_interest_rate: dec!(0.20),
            fund_term_years: Some(10),
            target_investor_base: vec!["EU_Institutional".to_string()],
            aifmd_full_scope: true,
            ucits_compliant: false,
            subscription_tax_exempt: false,
            management_company_location: "Luxembourg".to_string(),
        }
    }

    fn lux_raif_input() -> LuxFundInput {
        let mut input = lux_sif_input();
        input.fund_name = "Fast Launch RAIF".to_string();
        input.structure_type = "SICAV_RAIF".to_string();
        input
    }

    fn lux_scsp_input() -> LuxFundInput {
        LuxFundInput {
            fund_name: "Lux PE SCSp".to_string(),
            structure_type: "SCSp".to_string(),
            domicile: "Luxembourg".to_string(),
            fund_size: dec!(300_000_000),
            management_fee_rate: dec!(0.02),
            carried_interest_rate: dec!(0.20),
            fund_term_years: Some(12),
            target_investor_base: vec!["EU_Institutional".to_string(), "US_TaxExempt".to_string()],
            aifmd_full_scope: false,
            ucits_compliant: false,
            subscription_tax_exempt: true,
            management_company_location: "Luxembourg".to_string(),
        }
    }

    fn ireland_icav_input() -> LuxFundInput {
        LuxFundInput {
            fund_name: "Dublin ICAV Fund".to_string(),
            structure_type: "ICAV".to_string(),
            domicile: "Ireland".to_string(),
            fund_size: dec!(750_000_000),
            management_fee_rate: dec!(0.01),
            carried_interest_rate: dec!(0.15),
            fund_term_years: None,
            target_investor_base: vec![
                "EU_Institutional".to_string(),
                "US_TaxExempt".to_string(),
                "Asian_SWF".to_string(),
            ],
            aifmd_full_scope: true,
            ucits_compliant: true,
            subscription_tax_exempt: true,
            management_company_location: "Dublin".to_string(),
        }
    }

    fn ireland_qiaif_input() -> LuxFundInput {
        LuxFundInput {
            fund_name: "Dublin QIAIF".to_string(),
            structure_type: "QIAIF".to_string(),
            domicile: "Ireland".to_string(),
            fund_size: dec!(200_000_000),
            management_fee_rate: dec!(0.02),
            carried_interest_rate: dec!(0.20),
            fund_term_years: Some(7),
            target_investor_base: vec!["EU_Institutional".to_string()],
            aifmd_full_scope: true,
            ucits_compliant: false,
            subscription_tax_exempt: true,
            management_company_location: "Dublin".to_string(),
        }
    }

    fn section110_input() -> LuxFundInput {
        LuxFundInput {
            fund_name: "Irish SPV Section 110".to_string(),
            structure_type: "Section110".to_string(),
            domicile: "Ireland".to_string(),
            fund_size: dec!(100_000_000),
            management_fee_rate: dec!(0.005),
            carried_interest_rate: Decimal::ZERO,
            fund_term_years: Some(5),
            target_investor_base: vec!["EU_Institutional".to_string()],
            aifmd_full_scope: false,
            ucits_compliant: false,
            subscription_tax_exempt: true,
            management_company_location: "Dublin".to_string(),
        }
    }

    // ------------------------------------------------------------------
    // 1. Basic SICAV-SIF
    // ------------------------------------------------------------------
    #[test]
    fn test_basic_sicav_sif() {
        let input = lux_sif_input();
        let result = analyze_lux_structure(&input).unwrap();

        assert_eq!(result.structure_type, "SICAV_SIF");
        assert_eq!(result.domicile, "Luxembourg");
        assert!(result.structure_comparison.regulatory_approval_needed);
        assert_eq!(result.structure_comparison.approval_timeline_weeks, 52);
    }

    // ------------------------------------------------------------------
    // 2. SICAV-RAIF — faster launch
    // ------------------------------------------------------------------
    #[test]
    fn test_sicav_raif_faster_launch() {
        let input = lux_raif_input();
        let result = analyze_lux_structure(&input).unwrap();

        assert_eq!(result.structure_type, "SICAV_RAIF");
        assert!(!result.structure_comparison.regulatory_approval_needed);
        assert_eq!(result.structure_comparison.approval_timeline_weeks, 4);
    }

    // ------------------------------------------------------------------
    // 3. SCSp — tax transparent partnership
    // ------------------------------------------------------------------
    #[test]
    fn test_scsp_tax_transparent() {
        let input = lux_scsp_input();
        let result = analyze_lux_structure(&input).unwrap();

        assert_eq!(result.structure_type, "SCSp");
        assert_eq!(result.tax_analysis.subscription_tax_rate, Decimal::ZERO);
        assert_eq!(result.tax_analysis.subscription_tax_annual, Decimal::ZERO);
    }

    // ------------------------------------------------------------------
    // 4. Subscription tax — SIF non-exempt
    // ------------------------------------------------------------------
    #[test]
    fn test_subscription_tax_sif_non_exempt() {
        let input = lux_sif_input();
        let result = analyze_lux_structure(&input).unwrap();

        // 1bp for institutional SIF
        assert_eq!(result.tax_analysis.subscription_tax_rate, dec!(0.0001));
        let expected = dec!(500_000_000) * dec!(0.0001);
        assert_eq!(result.tax_analysis.subscription_tax_annual, expected);
    }

    // ------------------------------------------------------------------
    // 5. Subscription tax — exempt
    // ------------------------------------------------------------------
    #[test]
    fn test_subscription_tax_exempt() {
        let mut input = lux_sif_input();
        input.subscription_tax_exempt = true;
        let result = analyze_lux_structure(&input).unwrap();

        assert_eq!(result.tax_analysis.subscription_tax_rate, Decimal::ZERO);
    }

    // ------------------------------------------------------------------
    // 6. Ireland ICAV
    // ------------------------------------------------------------------
    #[test]
    fn test_ireland_icav() {
        let input = ireland_icav_input();
        let result = analyze_lux_structure(&input).unwrap();

        assert_eq!(result.structure_type, "ICAV");
        assert_eq!(result.domicile, "Ireland");
        assert!(result.structure_comparison.regulatory_approval_needed);
    }

    // ------------------------------------------------------------------
    // 7. QIAIF — minimum investment
    // ------------------------------------------------------------------
    #[test]
    fn test_qiaif_minimum_investment() {
        let input = ireland_qiaif_input();
        let result = analyze_lux_structure(&input).unwrap();

        assert_eq!(
            result.structure_comparison.minimum_investment,
            dec!(100_000)
        );
    }

    // ------------------------------------------------------------------
    // 8. Section 110 — SPV
    // ------------------------------------------------------------------
    #[test]
    fn test_section110_spv() {
        let input = section110_input();
        let result = analyze_lux_structure(&input).unwrap();

        assert_eq!(result.structure_type, "Section110");
        assert!(!result.structure_comparison.regulatory_approval_needed);
        assert!(result
            .structure_comparison
            .suitable_for
            .contains(&"Securitization".to_string()));
    }

    // ------------------------------------------------------------------
    // 9. AIFMD capital — small fund
    // ------------------------------------------------------------------
    #[test]
    fn test_aifmd_capital_small_fund() {
        let mut input = lux_sif_input();
        input.fund_size = dec!(100_000_000);
        let result = analyze_lux_structure(&input).unwrap();

        // Under EUR 250M, only base capital of EUR 125k
        assert_eq!(result.aifmd.aifm_capital_required, dec!(125_000));
    }

    // ------------------------------------------------------------------
    // 10. AIFMD capital — large fund
    // ------------------------------------------------------------------
    #[test]
    fn test_aifmd_capital_large_fund() {
        let input = lux_sif_input();
        let result = analyze_lux_structure(&input).unwrap();

        // EUR 500M: base 125k + 0.02% of (500M - 250M) = 125k + 50k = 175k
        assert_eq!(result.aifmd.aifm_capital_required, dec!(175_000));
    }

    // ------------------------------------------------------------------
    // 11. AIFMD capital — cap at EUR 10M
    // ------------------------------------------------------------------
    #[test]
    fn test_aifmd_capital_capped() {
        let mut input = lux_sif_input();
        input.fund_size = dec!(100_000_000_000); // 100B
        let result = analyze_lux_structure(&input).unwrap();

        assert_eq!(result.aifmd.aifm_capital_required, dec!(10_000_000));
    }

    // ------------------------------------------------------------------
    // 12. AIFMD passport — full scope
    // ------------------------------------------------------------------
    #[test]
    fn test_aifmd_passport_full_scope() {
        let input = lux_sif_input();
        let result = analyze_lux_structure(&input).unwrap();

        assert!(result.aifmd.full_scope_required);
        assert_eq!(result.aifmd.passport_jurisdictions.len(), 30);
        assert!(result.aifmd.depositary_required);
        assert!(result.aifmd.risk_management_required);
    }

    // ------------------------------------------------------------------
    // 13. AIFMD — no passport without full scope
    // ------------------------------------------------------------------
    #[test]
    fn test_aifmd_no_passport_without_full_scope() {
        let input = lux_scsp_input();
        let result = analyze_lux_structure(&input).unwrap();

        assert!(!result.aifmd.full_scope_required);
        assert!(result.aifmd.passport_jurisdictions.is_empty());
        assert_eq!(result.aifmd.aifm_capital_required, Decimal::ZERO);
    }

    // ------------------------------------------------------------------
    // 14. UCITS analysis — ICAV eligible
    // ------------------------------------------------------------------
    #[test]
    fn test_ucits_eligible_icav() {
        let input = ireland_icav_input();
        let result = analyze_lux_structure(&input).unwrap();

        assert!(result.ucits.is_some());
        let ucits = result.ucits.unwrap();
        assert!(ucits.eligible);
        assert!(ucits.kid_required);
        assert!(!ucits.distribution_countries.is_empty());
    }

    // ------------------------------------------------------------------
    // 15. UCITS analysis — not compliant
    // ------------------------------------------------------------------
    #[test]
    fn test_ucits_not_compliant() {
        let input = lux_scsp_input();
        let result = analyze_lux_structure(&input).unwrap();

        assert!(result.ucits.is_none());
    }

    // ------------------------------------------------------------------
    // 16. Operational costs — Luxembourg
    // ------------------------------------------------------------------
    #[test]
    fn test_operational_costs_lux() {
        let input = lux_sif_input();
        let result = analyze_lux_structure(&input).unwrap();

        assert!(result.operational.central_admin_annual > Decimal::ZERO);
        assert!(result.operational.depositary_annual > Decimal::ZERO);
        assert!(result.operational.total_annual_cost > Decimal::ZERO);
        assert!(result.operational.total_expense_ratio > Decimal::ZERO);
    }

    // ------------------------------------------------------------------
    // 17. Operational costs — Ireland
    // ------------------------------------------------------------------
    #[test]
    fn test_operational_costs_ireland() {
        let input = ireland_icav_input();
        let result = analyze_lux_structure(&input).unwrap();

        assert!(result.operational.central_admin_annual > Decimal::ZERO);
        assert!(result.operational.regulatory_fees_annual > Decimal::ZERO);
    }

    // ------------------------------------------------------------------
    // 18. TER is reasonable
    // ------------------------------------------------------------------
    #[test]
    fn test_ter_reasonable() {
        let input = lux_sif_input();
        let result = analyze_lux_structure(&input).unwrap();

        assert!(
            result.operational.total_expense_ratio < dec!(0.05),
            "TER should be less than 5%"
        );
    }

    // ------------------------------------------------------------------
    // 19. Investor access — EU passport with AIFMD
    // ------------------------------------------------------------------
    #[test]
    fn test_investor_access_eu_passport() {
        let input = lux_sif_input();
        let result = analyze_lux_structure(&input).unwrap();

        assert!(result.investor_access.eu_passport);
    }

    // ------------------------------------------------------------------
    // 20. Investor access — no EU passport without AIFMD/UCITS
    // ------------------------------------------------------------------
    #[test]
    fn test_investor_access_no_passport() {
        let input = lux_scsp_input();
        let result = analyze_lux_structure(&input).unwrap();

        assert!(!result.investor_access.eu_passport);
    }

    // ------------------------------------------------------------------
    // 21. US access method — ICAV check-the-box
    // ------------------------------------------------------------------
    #[test]
    fn test_us_access_icav() {
        let input = ireland_icav_input();
        let result = analyze_lux_structure(&input).unwrap();

        assert!(result
            .investor_access
            .us_access_method
            .contains("Check-the-box"));
    }

    // ------------------------------------------------------------------
    // 22. US access method — Lux private placement
    // ------------------------------------------------------------------
    #[test]
    fn test_us_access_lux() {
        let mut input = lux_sif_input();
        input.target_investor_base.push("US_TaxExempt".to_string());
        let result = analyze_lux_structure(&input).unwrap();

        assert!(result.investor_access.us_access_method.contains("Reg D"));
    }

    // ------------------------------------------------------------------
    // 23. Asia access — UCITS recognized
    // ------------------------------------------------------------------
    #[test]
    fn test_asia_access_ucits() {
        let input = ireland_icav_input();
        let result = analyze_lux_structure(&input).unwrap();

        assert!(!result.investor_access.asia_access.is_empty());
        assert!(result
            .investor_access
            .asia_access
            .iter()
            .any(|a| a.contains("Singapore")));
    }

    // ------------------------------------------------------------------
    // 24. Treaty benefits — Luxembourg
    // ------------------------------------------------------------------
    #[test]
    fn test_treaty_benefits_lux() {
        let input = lux_sif_input();
        let result = analyze_lux_structure(&input).unwrap();

        assert!(!result.tax_analysis.treaty_benefits.is_empty());
        assert!(result
            .tax_analysis
            .treaty_benefits
            .iter()
            .any(|t| t.jurisdiction == "United States"));
    }

    // ------------------------------------------------------------------
    // 25. Treaty benefits — Ireland
    // ------------------------------------------------------------------
    #[test]
    fn test_treaty_benefits_ireland() {
        let input = ireland_icav_input();
        let result = analyze_lux_structure(&input).unwrap();

        assert!(!result.tax_analysis.treaty_benefits.is_empty());
        assert!(result
            .tax_analysis
            .treaty_benefits
            .iter()
            .any(|t| t.jurisdiction == "United Kingdom"));
    }

    // ------------------------------------------------------------------
    // 26. Ireland — no subscription tax
    // ------------------------------------------------------------------
    #[test]
    fn test_ireland_no_subscription_tax() {
        let input = ireland_icav_input();
        let result = analyze_lux_structure(&input).unwrap();

        assert_eq!(result.tax_analysis.subscription_tax_rate, Decimal::ZERO);
    }

    // ------------------------------------------------------------------
    // 27. Effective tax drag
    // ------------------------------------------------------------------
    #[test]
    fn test_effective_tax_drag() {
        let input = lux_sif_input();
        let result = analyze_lux_structure(&input).unwrap();

        // SIF non-exempt: sub tax = 1bp
        assert_eq!(result.tax_analysis.effective_tax_drag, dec!(0.0001));
    }

    // ------------------------------------------------------------------
    // 28. Validation — empty fund name
    // ------------------------------------------------------------------
    #[test]
    fn test_validation_empty_fund_name() {
        let mut input = lux_sif_input();
        input.fund_name = "".to_string();
        let result = analyze_lux_structure(&input);
        assert!(result.is_err());

        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "fund_name");
            }
            other => panic!("Expected InvalidInput, got: {other}"),
        }
    }

    // ------------------------------------------------------------------
    // 29. Validation — invalid structure type
    // ------------------------------------------------------------------
    #[test]
    fn test_validation_invalid_structure() {
        let mut input = lux_sif_input();
        input.structure_type = "SICAV_UNKNOWN".to_string();
        let result = analyze_lux_structure(&input);
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------
    // 30. Validation — invalid domicile
    // ------------------------------------------------------------------
    #[test]
    fn test_validation_invalid_domicile() {
        let mut input = lux_sif_input();
        input.domicile = "Jersey".to_string();
        let result = analyze_lux_structure(&input);
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------
    // 31. Validation — zero fund size
    // ------------------------------------------------------------------
    #[test]
    fn test_validation_zero_fund_size() {
        let mut input = lux_sif_input();
        input.fund_size = Decimal::ZERO;
        let result = analyze_lux_structure(&input);
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------
    // 32. Validation — negative fund size
    // ------------------------------------------------------------------
    #[test]
    fn test_validation_negative_fund_size() {
        let mut input = lux_sif_input();
        input.fund_size = dec!(-100);
        let result = analyze_lux_structure(&input);
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------
    // 33. Validation — management fee out of range
    // ------------------------------------------------------------------
    #[test]
    fn test_validation_mgmt_fee_out_of_range() {
        let mut input = lux_sif_input();
        input.management_fee_rate = dec!(1.5);
        let result = analyze_lux_structure(&input);
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------
    // 34. Validation — negative carried interest
    // ------------------------------------------------------------------
    #[test]
    fn test_validation_negative_carried_interest() {
        let mut input = lux_sif_input();
        input.carried_interest_rate = dec!(-0.10);
        let result = analyze_lux_structure(&input);
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------
    // 35. Validation — SIF must be Luxembourg
    // ------------------------------------------------------------------
    #[test]
    fn test_validation_sif_wrong_domicile() {
        let mut input = lux_sif_input();
        input.domicile = "Ireland".to_string();
        let result = analyze_lux_structure(&input);
        assert!(result.is_err());

        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "domicile");
            }
            other => panic!("Expected InvalidInput for domicile, got: {other}"),
        }
    }

    // ------------------------------------------------------------------
    // 36. Validation — ICAV must be Ireland
    // ------------------------------------------------------------------
    #[test]
    fn test_validation_icav_wrong_domicile() {
        let mut input = ireland_icav_input();
        input.domicile = "Luxembourg".to_string();
        let result = analyze_lux_structure(&input);
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------
    // 37. RAIF same tax as SIF
    // ------------------------------------------------------------------
    #[test]
    fn test_raif_same_tax_as_sif() {
        let sif = lux_sif_input();
        let raif = lux_raif_input();
        let sif_result = analyze_lux_structure(&sif).unwrap();
        let raif_result = analyze_lux_structure(&raif).unwrap();

        assert_eq!(
            sif_result.tax_analysis.subscription_tax_rate,
            raif_result.tax_analysis.subscription_tax_rate
        );
    }

    // ------------------------------------------------------------------
    // 38. SIF minimum investment
    // ------------------------------------------------------------------
    #[test]
    fn test_sif_minimum_investment() {
        let input = lux_sif_input();
        let result = analyze_lux_structure(&input).unwrap();

        assert_eq!(
            result.structure_comparison.minimum_investment,
            dec!(1_250_000)
        );
    }

    // ------------------------------------------------------------------
    // 39. SCSp no minimum investment
    // ------------------------------------------------------------------
    #[test]
    fn test_scsp_no_minimum() {
        let input = lux_scsp_input();
        let result = analyze_lux_structure(&input).unwrap();

        assert_eq!(
            result.structure_comparison.minimum_investment,
            Decimal::ZERO
        );
    }

    // ------------------------------------------------------------------
    // 40. SCSp no diversification required
    // ------------------------------------------------------------------
    #[test]
    fn test_scsp_no_diversification() {
        let input = lux_scsp_input();
        let result = analyze_lux_structure(&input).unwrap();

        assert!(!result.structure_comparison.diversification_required);
    }

    // ------------------------------------------------------------------
    // 41. Depositary cost with AIFMD
    // ------------------------------------------------------------------
    #[test]
    fn test_depositary_with_aifmd() {
        let input = lux_sif_input();
        let result = analyze_lux_structure(&input).unwrap();

        assert!(result.operational.depositary_annual > Decimal::ZERO);
    }

    // ------------------------------------------------------------------
    // 42. No depositary without AIFMD
    // ------------------------------------------------------------------
    #[test]
    fn test_no_depositary_without_aifmd() {
        let input = lux_scsp_input();
        let result = analyze_lux_structure(&input).unwrap();

        assert_eq!(result.operational.depositary_annual, Decimal::ZERO);
    }

    // ------------------------------------------------------------------
    // 43. Operational total = sum of components
    // ------------------------------------------------------------------
    #[test]
    fn test_operational_total_sum() {
        let input = lux_sif_input();
        let result = analyze_lux_structure(&input).unwrap();

        let expected = result.operational.central_admin_annual
            + result.operational.transfer_agent_annual
            + result.operational.depositary_annual
            + result.operational.regulatory_fees_annual
            + result.operational.legal_ongoing;

        assert_eq!(result.operational.total_annual_cost, expected);
    }

    // ------------------------------------------------------------------
    // 44. TER matches total / fund_size
    // ------------------------------------------------------------------
    #[test]
    fn test_ter_matches_calculation() {
        let input = lux_sif_input();
        let result = analyze_lux_structure(&input).unwrap();

        let expected_ter = result.operational.total_annual_cost / dec!(500_000_000);
        assert_eq!(result.operational.total_expense_ratio, expected_ter);
    }

    // ------------------------------------------------------------------
    // 45. Output serialization round-trip
    // ------------------------------------------------------------------
    #[test]
    fn test_output_serialization() {
        let input = lux_sif_input();
        let result = analyze_lux_structure(&input).unwrap();

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: LuxFundOutput = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.structure_type, result.structure_type);
        assert_eq!(deserialized.domicile, result.domicile);
        assert_eq!(
            deserialized.tax_analysis.subscription_tax_rate,
            result.tax_analysis.subscription_tax_rate
        );
    }

    // ------------------------------------------------------------------
    // 46. Ireland fund level income tax is zero
    // ------------------------------------------------------------------
    #[test]
    fn test_ireland_fund_level_tax_zero() {
        let input = ireland_icav_input();
        let result = analyze_lux_structure(&input).unwrap();

        assert_eq!(result.tax_analysis.fund_level_income_tax, Decimal::ZERO);
    }

    // ------------------------------------------------------------------
    // 47. Luxembourg fund level income tax is zero
    // ------------------------------------------------------------------
    #[test]
    fn test_lux_fund_level_tax_zero() {
        let input = lux_sif_input();
        let result = analyze_lux_structure(&input).unwrap();

        assert_eq!(result.tax_analysis.fund_level_income_tax, Decimal::ZERO);
    }

    // ------------------------------------------------------------------
    // 48. UCITS not available for SCSp
    // ------------------------------------------------------------------
    #[test]
    fn test_ucits_not_for_scsp() {
        let mut input = lux_scsp_input();
        input.ucits_compliant = true;
        let result = analyze_lux_structure(&input).unwrap();

        let ucits = result.ucits.unwrap();
        assert!(!ucits.eligible);
    }

    // ------------------------------------------------------------------
    // 49. Small fund lower costs than large
    // ------------------------------------------------------------------
    #[test]
    fn test_small_fund_lower_costs() {
        let mut small = lux_sif_input();
        small.fund_size = dec!(50_000_000);
        let small_result = analyze_lux_structure(&small).unwrap();

        let large = lux_sif_input();
        let large_result = analyze_lux_structure(&large).unwrap();

        assert!(
            small_result.operational.total_annual_cost < large_result.operational.total_annual_cost,
            "Small fund should have lower total costs"
        );
    }

    // ------------------------------------------------------------------
    // 50. QIAIF no diversification requirement
    // ------------------------------------------------------------------
    #[test]
    fn test_qiaif_no_diversification() {
        let input = ireland_qiaif_input();
        let result = analyze_lux_structure(&input).unwrap();

        assert!(!result.structure_comparison.diversification_required);
    }
}
