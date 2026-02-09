use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::*;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InvestorType {
    /// US pension, endowment, foundation (501(c)(3))
    TaxExemptUS,
    /// Non-US investor
    ForeignInvestor,
    /// Taxable US entity/individual
    TaxableUS,
    SovereignWealth,
    InsuranceCompany,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InvestmentType {
    DirectEquity,
    DirectDebt,
    LeveragedRealEstate,
    OperatingBusiness,
    /// Master Limited Partnership
    MLP,
    /// Unrelated Debt-Financed Income
    DebtFinancedProperty,
    HedgeFund,
    PrivateEquityFund,
    VentureCapitalFund,
    RealEstateFund,
    FundOfFunds,
    PublicEquity,
    PublicFixedIncome,
    Commodities,
    Derivatives,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RiskLevel {
    None,
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UbtiScreeningInput {
    pub investor_type: InvestorType,
    pub investments: Vec<InvestmentDetail>,
    pub use_blocker: Option<bool>,
    pub currency: Option<Currency>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestmentDetail {
    pub name: String,
    pub investment_type: InvestmentType,
    pub amount: Money,
    pub is_leveraged: bool,
    /// Debt/equity at fund level
    pub leverage_ratio: Option<Decimal>,
    pub has_operating_income: bool,
    /// Percentage of income that is US-source
    pub us_source_income_pct: Option<Rate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UbtiScreeningOutput {
    pub investor_type: InvestorType,
    pub investment_results: Vec<InvestmentScreeningResult>,
    pub total_at_risk: Money,
    pub total_screened: Money,
    pub ubti_risk_pct: Rate,
    pub eci_risk_pct: Rate,
    pub overall_risk: RiskLevel,
    pub recommendations: Vec<String>,
    pub estimated_blocker_cost: Option<Money>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestmentScreeningResult {
    pub name: String,
    pub investment_type: InvestmentType,
    pub amount: Money,
    pub ubti_risk: RiskLevel,
    pub eci_risk: RiskLevel,
    pub ubti_reason: String,
    pub eci_reason: String,
    pub blocker_recommended: bool,
    pub estimated_ubti_amount: Option<Money>,
    pub mitigation_options: Vec<String>,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Default assumed annual return rate for estimating UBTI from leveraged income.
const DEFAULT_RETURN_RATE: Decimal = dec!(0.08);

/// Blocker entity annual cost as a fraction of invested amount (0.15%).
const BLOCKER_ANNUAL_COST_RATE: Decimal = dec!(0.0015);

// ---------------------------------------------------------------------------
// Main calculation
// ---------------------------------------------------------------------------

/// Screen a portfolio of investments for UBTI (Unrelated Business Taxable
/// Income) and ECI (Effectively Connected Income) risk.
///
/// UBTI applies primarily to US tax-exempt investors (pensions, endowments,
/// foundations). ECI applies to foreign investors with US-source income
/// that is "effectively connected" with a US trade or business.
pub fn screen_ubti_eci(
    input: &UbtiScreeningInput,
) -> CorpFinanceResult<ComputationOutput<UbtiScreeningOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // ------------------------------------------------------------------
    // 1. Validate inputs
    // ------------------------------------------------------------------
    validate_input(input)?;

    // ------------------------------------------------------------------
    // 2. Screen each investment
    // ------------------------------------------------------------------
    let mut investment_results: Vec<InvestmentScreeningResult> = Vec::new();
    let mut total_screened = Decimal::ZERO;
    let mut total_at_risk_ubti = Decimal::ZERO;
    let mut total_at_risk_eci = Decimal::ZERO;
    let mut overall_risk = RiskLevel::None;
    let mut total_blocker_basis = Decimal::ZERO;

    for inv in &input.investments {
        total_screened += inv.amount;

        let (ubti_risk, ubti_reason) = assess_ubti_risk(&input.investor_type, inv);
        let (eci_risk, eci_reason) = assess_eci_risk(&input.investor_type, inv);

        // Track amounts at risk
        if matches!(ubti_risk, RiskLevel::Medium | RiskLevel::High) {
            total_at_risk_ubti += inv.amount;
        }
        if matches!(eci_risk, RiskLevel::Medium | RiskLevel::High) {
            total_at_risk_eci += inv.amount;
        }

        // Track highest risk level
        if ubti_risk > overall_risk {
            overall_risk = ubti_risk.clone();
        }
        if eci_risk > overall_risk {
            overall_risk = eci_risk.clone();
        }

        // Blocker recommendation
        let blocker_recommended =
            should_recommend_blocker(&input.investor_type, &ubti_risk, &eci_risk);
        if blocker_recommended {
            total_blocker_basis += inv.amount;
        }

        // Estimated UBTI amount from leverage (UDFI)
        let estimated_ubti_amount = estimate_ubti_from_leverage(inv);

        // Mitigation options
        let mitigation_options = build_mitigation_options(inv, &ubti_risk, &eci_risk);

        investment_results.push(InvestmentScreeningResult {
            name: inv.name.clone(),
            investment_type: inv.investment_type.clone(),
            amount: inv.amount,
            ubti_risk,
            eci_risk,
            ubti_reason,
            eci_reason,
            blocker_recommended,
            estimated_ubti_amount,
            mitigation_options,
        });
    }

    // ------------------------------------------------------------------
    // 3. Aggregate metrics
    // ------------------------------------------------------------------
    let total_at_risk = total_at_risk_ubti.max(total_at_risk_eci);

    let ubti_risk_pct = if total_screened > Decimal::ZERO {
        total_at_risk_ubti / total_screened
    } else {
        Decimal::ZERO
    };

    let eci_risk_pct = if total_screened > Decimal::ZERO {
        total_at_risk_eci / total_screened
    } else {
        Decimal::ZERO
    };

    // ------------------------------------------------------------------
    // 4. Blocker cost estimate
    // ------------------------------------------------------------------
    let estimated_blocker_cost = if total_blocker_basis > Decimal::ZERO {
        Some(total_blocker_basis * BLOCKER_ANNUAL_COST_RATE)
    } else {
        None
    };

    // ------------------------------------------------------------------
    // 5. Portfolio-level recommendations
    // ------------------------------------------------------------------
    let recommendations = build_portfolio_recommendations(
        &input.investor_type,
        &investment_results,
        &overall_risk,
        &mut warnings,
    );

    // ------------------------------------------------------------------
    // 6. Assemble output
    // ------------------------------------------------------------------
    let output = UbtiScreeningOutput {
        investor_type: input.investor_type.clone(),
        investment_results,
        total_at_risk,
        total_screened,
        ubti_risk_pct,
        eci_risk_pct,
        overall_risk,
        recommendations,
        estimated_blocker_cost,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "UBTI/ECI Screening: Tax-Exempt and Foreign Investor Risk Assessment",
        &serde_json::json!({
            "investor_type": format!("{:?}", input.investor_type),
            "num_investments": input.investments.len(),
            "total_screened": total_screened.to_string(),
            "use_blocker": input.use_blocker,
            "default_return_rate": DEFAULT_RETURN_RATE.to_string(),
            "blocker_annual_cost_rate": BLOCKER_ANNUAL_COST_RATE.to_string(),
        }),
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_input(input: &UbtiScreeningInput) -> CorpFinanceResult<()> {
    if input.investments.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one investment is required for UBTI/ECI screening".to_string(),
        ));
    }

    for (i, inv) in input.investments.iter().enumerate() {
        if inv.amount <= Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("investments[{}].amount", i),
                reason: "Investment amount must be positive".to_string(),
            });
        }
        if let Some(ratio) = inv.leverage_ratio {
            if ratio < Decimal::ZERO {
                return Err(CorpFinanceError::InvalidInput {
                    field: format!("investments[{}].leverage_ratio", i),
                    reason: "Leverage ratio must be non-negative".to_string(),
                });
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// UBTI risk assessment
// ---------------------------------------------------------------------------

fn assess_ubti_risk(investor_type: &InvestorType, inv: &InvestmentDetail) -> (RiskLevel, String) {
    // Only TaxExemptUS investors are subject to UBTI
    if !matches!(investor_type, InvestorType::TaxExemptUS) {
        return (
            RiskLevel::None,
            "UBTI applies only to US tax-exempt investors".to_string(),
        );
    }

    use InvestmentType::*;

    match &inv.investment_type {
        // High UBTI risk categories
        OperatingBusiness => (
            RiskLevel::High,
            "Operating business income is directly subject to UBTI for tax-exempt investors"
                .to_string(),
        ),
        MLP => (
            RiskLevel::High,
            "MLPs generate UBTI through pass-through of trade or business income".to_string(),
        ),
        DebtFinancedProperty => (
            RiskLevel::High,
            "Debt-financed property generates UDFI (Unrelated Debt-Financed Income), \
             a form of UBTI"
                .to_string(),
        ),
        LeveragedRealEstate => (
            RiskLevel::High,
            "Leveraged real estate generates UBTI through debt-financed income (UDFI)".to_string(),
        ),

        // Medium UBTI risk — depends on structure
        HedgeFund => {
            if inv.is_leveraged {
                (
                    RiskLevel::Medium,
                    "Leveraged hedge fund positions may generate UBTI through \
                     debt-financed income"
                        .to_string(),
                )
            } else {
                (
                    RiskLevel::Low,
                    "Unleveraged hedge fund typically generates passive income, \
                     low UBTI risk"
                        .to_string(),
                )
            }
        }
        PrivateEquityFund => {
            if inv.has_operating_income {
                (
                    RiskLevel::Medium,
                    "PE fund with operating companies may pass through UBTI from \
                     portfolio company operations"
                        .to_string(),
                )
            } else {
                (
                    RiskLevel::Low,
                    "PE fund without operating income has limited UBTI exposure".to_string(),
                )
            }
        }
        RealEstateFund => {
            if inv.is_leveraged {
                (
                    RiskLevel::Medium,
                    "Leveraged real estate fund generates UDFI on the debt-financed \
                     portion of income"
                        .to_string(),
                )
            } else {
                (
                    RiskLevel::Low,
                    "Unleveraged real estate fund rental income is typically excluded \
                     from UBTI"
                        .to_string(),
                )
            }
        }
        Commodities | Derivatives => {
            if inv.is_leveraged {
                (
                    RiskLevel::Medium,
                    "Leveraged commodities/derivatives positions may generate \
                     debt-financed UBTI"
                        .to_string(),
                )
            } else {
                (
                    RiskLevel::Low,
                    "Unleveraged commodities/derivatives generate passive income".to_string(),
                )
            }
        }

        // Low UBTI risk
        VentureCapitalFund => (
            RiskLevel::Low,
            "VC fund dividends and capital gains are generally excluded from UBTI".to_string(),
        ),
        DirectDebt => {
            if inv.is_leveraged {
                (
                    RiskLevel::Medium,
                    "Leveraged debt investment generates UDFI on the debt-financed portion"
                        .to_string(),
                )
            } else {
                (
                    RiskLevel::Low,
                    "Unleveraged direct debt generates interest income excluded from UBTI"
                        .to_string(),
                )
            }
        }
        DirectEquity => {
            if inv.is_leveraged {
                (
                    RiskLevel::Medium,
                    "Leveraged equity position generates UDFI on the debt-financed portion"
                        .to_string(),
                )
            } else {
                (
                    RiskLevel::None,
                    "Unleveraged direct equity generates passive dividend/capital gain income"
                        .to_string(),
                )
            }
        }
        FundOfFunds => {
            if inv.is_leveraged {
                (
                    RiskLevel::Medium,
                    "Leveraged fund-of-funds may pass through UBTI from underlying funds"
                        .to_string(),
                )
            } else {
                (
                    RiskLevel::Low,
                    "Unleveraged fund-of-funds has limited direct UBTI exposure".to_string(),
                )
            }
        }

        // None — passive investments
        PublicEquity => (
            RiskLevel::Low,
            "Public equity dividends and capital gains are excluded from UBTI".to_string(),
        ),
        PublicFixedIncome => (
            RiskLevel::Low,
            "Public fixed income interest is excluded from UBTI".to_string(),
        ),
    }
}

// ---------------------------------------------------------------------------
// ECI risk assessment
// ---------------------------------------------------------------------------

fn assess_eci_risk(investor_type: &InvestorType, inv: &InvestmentDetail) -> (RiskLevel, String) {
    // ECI applies only to foreign investors
    if !matches!(investor_type, InvestorType::ForeignInvestor) {
        return (
            RiskLevel::None,
            "ECI applies only to non-US (foreign) investors".to_string(),
        );
    }

    let us_pct = inv.us_source_income_pct.unwrap_or(Decimal::ZERO);

    use InvestmentType::*;

    match &inv.investment_type {
        // High ECI risk
        OperatingBusiness => {
            if us_pct > Decimal::ZERO {
                (
                    RiskLevel::High,
                    format!(
                        "US operating business income ({:.0}% US-source) is ECI, subject \
                         to US tax and filing obligations",
                        us_pct * dec!(100)
                    ),
                )
            } else {
                (
                    RiskLevel::None,
                    "Non-US operating business has no ECI exposure".to_string(),
                )
            }
        }
        MLP => (
            RiskLevel::High,
            "MLPs generate ECI for foreign investors through pass-through of \
             US trade or business income"
                .to_string(),
        ),
        LeveragedRealEstate => {
            if us_pct > Decimal::ZERO {
                (
                    RiskLevel::High,
                    "US real estate income is ECI under FIRPTA; leveraged real estate \
                     amplifies exposure"
                        .to_string(),
                )
            } else {
                (
                    RiskLevel::None,
                    "Non-US real estate has no ECI exposure".to_string(),
                )
            }
        }
        DebtFinancedProperty => {
            if us_pct > Decimal::ZERO {
                (
                    RiskLevel::High,
                    "US debt-financed property income is ECI under FIRPTA".to_string(),
                )
            } else {
                (
                    RiskLevel::None,
                    "Non-US debt-financed property has no ECI exposure".to_string(),
                )
            }
        }
        RealEstateFund => {
            if us_pct > Decimal::ZERO {
                (
                    RiskLevel::High,
                    format!(
                        "US real estate fund ({:.0}% US-source) generates ECI under FIRPTA",
                        us_pct * dec!(100)
                    ),
                )
            } else {
                (
                    RiskLevel::None,
                    "Non-US real estate fund has no ECI exposure".to_string(),
                )
            }
        }

        // Medium ECI risk
        PrivateEquityFund => {
            if us_pct > Decimal::ZERO && inv.has_operating_income {
                (
                    RiskLevel::Medium,
                    "PE fund with US operating companies may generate ECI through \
                     pass-through income"
                        .to_string(),
                )
            } else if us_pct > Decimal::ZERO {
                (
                    RiskLevel::Low,
                    "PE fund with US exposure but no operating income has limited \
                     ECI risk"
                        .to_string(),
                )
            } else {
                (
                    RiskLevel::None,
                    "Non-US PE fund has no ECI exposure".to_string(),
                )
            }
        }
        HedgeFund => {
            if us_pct > Decimal::ZERO && inv.is_leveraged {
                (
                    RiskLevel::Medium,
                    "Leveraged hedge fund with US-source income may generate ECI".to_string(),
                )
            } else {
                (
                    RiskLevel::Low,
                    "Hedge fund trading income generally qualifies for the trading \
                     safe harbor (not ECI)"
                        .to_string(),
                )
            }
        }

        // Low ECI risk
        DirectDebt => (
            RiskLevel::Low,
            "Portfolio interest exemption generally shields foreign investors from \
             ECI on US debt"
                .to_string(),
        ),
        PublicEquity => (
            RiskLevel::Low,
            "Public equity capital gains generally not ECI for foreign investors \
             (absent US real property)"
                .to_string(),
        ),
        PublicFixedIncome => (
            RiskLevel::Low,
            "Public fixed income interest generally qualifies for portfolio interest \
             exemption"
                .to_string(),
        ),
        VentureCapitalFund => {
            if us_pct > Decimal::ZERO {
                (
                    RiskLevel::Low,
                    "VC fund capital gains from US companies generally not ECI; \
                     dividends subject to WHT but not ECI"
                        .to_string(),
                )
            } else {
                (
                    RiskLevel::None,
                    "Non-US VC fund has no ECI exposure".to_string(),
                )
            }
        }

        // None — non-US or passive
        DirectEquity => {
            if us_pct > Decimal::ZERO {
                (
                    RiskLevel::Low,
                    "US equity dividends subject to WHT but generally not ECI \
                     (absent US trade or business)"
                        .to_string(),
                )
            } else {
                (
                    RiskLevel::None,
                    "Non-US equity has no ECI exposure".to_string(),
                )
            }
        }
        FundOfFunds => {
            if us_pct > Decimal::ZERO {
                (
                    RiskLevel::Low,
                    "Fund-of-funds with US exposure — ECI risk depends on underlying \
                     fund structures"
                        .to_string(),
                )
            } else {
                (
                    RiskLevel::None,
                    "Non-US fund-of-funds has no ECI exposure".to_string(),
                )
            }
        }
        Commodities | Derivatives => (
            RiskLevel::Low,
            "Commodities/derivatives generally qualify for the trading safe harbor".to_string(),
        ),
    }
}

// ---------------------------------------------------------------------------
// Blocker recommendation
// ---------------------------------------------------------------------------

fn should_recommend_blocker(
    investor_type: &InvestorType,
    ubti_risk: &RiskLevel,
    eci_risk: &RiskLevel,
) -> bool {
    match investor_type {
        InvestorType::TaxExemptUS => {
            matches!(ubti_risk, RiskLevel::High | RiskLevel::Medium)
        }
        InvestorType::ForeignInvestor => {
            matches!(eci_risk, RiskLevel::High | RiskLevel::Medium)
        }
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// UBTI amount estimation (leverage-based UDFI)
// ---------------------------------------------------------------------------

fn estimate_ubti_from_leverage(inv: &InvestmentDetail) -> Option<Money> {
    if !inv.is_leveraged {
        return None;
    }

    let ratio = inv.leverage_ratio.unwrap_or(Decimal::ZERO);
    if ratio <= Decimal::ZERO {
        return None;
    }

    // Debt-financed portion = amount * leverage_ratio / (1 + leverage_ratio)
    let debt_financed_portion = inv.amount * ratio / (Decimal::ONE + ratio);

    // Estimated UBTI = debt-financed portion * assumed return rate
    let estimated = debt_financed_portion * DEFAULT_RETURN_RATE;
    Some(estimated)
}

// ---------------------------------------------------------------------------
// Mitigation options
// ---------------------------------------------------------------------------

fn build_mitigation_options(
    inv: &InvestmentDetail,
    ubti_risk: &RiskLevel,
    eci_risk: &RiskLevel,
) -> Vec<String> {
    let mut options = Vec::new();

    if matches!(ubti_risk, RiskLevel::None) && matches!(eci_risk, RiskLevel::None) {
        return options;
    }

    // Blocker entity is the primary mitigation
    if matches!(ubti_risk, RiskLevel::High | RiskLevel::Medium)
        || matches!(eci_risk, RiskLevel::High | RiskLevel::Medium)
    {
        options.push(
            "Use C-corp blocker entity (domestic or offshore) to convert \
             pass-through income to dividends taxable at entity level"
                .to_string(),
        );
    }

    // Leverage removal
    if inv.is_leveraged {
        options.push(
            "Remove or reduce fund-level leverage to eliminate UDFI \
             (debt-financed income) component"
                .to_string(),
        );
    }

    // MLP-specific
    if matches!(inv.investment_type, InvestmentType::MLP) {
        options.push(
            "Evaluate qualified PTP (Publicly Traded Partnership) exception \
             for MLPs under IRC Section 512(c)"
                .to_string(),
        );
    }

    // Fund-of-funds blocker
    if matches!(
        inv.investment_type,
        InvestmentType::PrivateEquityFund
            | InvestmentType::HedgeFund
            | InvestmentType::RealEstateFund
    ) {
        options
            .push("Invest via fund-of-funds with built-in UBTI/ECI blocker structure".to_string());
    }

    // Fund restructuring
    if matches!(ubti_risk, RiskLevel::Medium) || matches!(eci_risk, RiskLevel::Medium) {
        options.push(
            "Request fund restructuring to segregate UBTI/ECI-generating \
             activities into a blocker vehicle"
                .to_string(),
        );
    }

    options
}

// ---------------------------------------------------------------------------
// Portfolio-level recommendations
// ---------------------------------------------------------------------------

fn build_portfolio_recommendations(
    investor_type: &InvestorType,
    results: &[InvestmentScreeningResult],
    overall_risk: &RiskLevel,
    warnings: &mut Vec<String>,
) -> Vec<String> {
    let mut recs = Vec::new();

    let high_risk_count = results
        .iter()
        .filter(|r| matches!(r.ubti_risk, RiskLevel::High) || matches!(r.eci_risk, RiskLevel::High))
        .count();
    let medium_risk_count = results
        .iter()
        .filter(|r| {
            matches!(r.ubti_risk, RiskLevel::Medium) || matches!(r.eci_risk, RiskLevel::Medium)
        })
        .count();
    let blocker_count = results.iter().filter(|r| r.blocker_recommended).count();

    match investor_type {
        InvestorType::TaxExemptUS => {
            if matches!(overall_risk, RiskLevel::High) {
                recs.push(format!(
                    "Portfolio contains {} high-risk UBTI investments. \
                     A C-corp blocker is strongly recommended to avoid \
                     unrelated business income tax obligations.",
                    high_risk_count
                ));
            }
            if medium_risk_count > 0 {
                recs.push(format!(
                    "{} investments have medium UBTI risk due to leverage \
                     or operating income exposure. Review fund structures \
                     and consider blocker entities.",
                    medium_risk_count
                ));
            }
            if blocker_count > 0 {
                recs.push(format!(
                    "Blocker recommended for {} of {} investments. \
                     Blocker converts UBTI to corporate-level dividends, \
                     eliminating tax-exempt investor filing obligations.",
                    blocker_count,
                    results.len()
                ));
            }
        }
        InvestorType::ForeignInvestor => {
            if matches!(overall_risk, RiskLevel::High) {
                recs.push(format!(
                    "Portfolio contains {} high-risk ECI investments. \
                     A blocker entity is strongly recommended to avoid \
                     US tax filing obligations and ECI tax exposure.",
                    high_risk_count
                ));
            }
            if medium_risk_count > 0 {
                recs.push(format!(
                    "{} investments have medium ECI risk. Review US-source \
                     income allocation and fund structures.",
                    medium_risk_count
                ));
            }
            if blocker_count > 0 {
                recs.push(format!(
                    "Blocker recommended for {} of {} investments to \
                     shield foreign investor from ECI and US filing \
                     requirements.",
                    blocker_count,
                    results.len()
                ));
            }
        }
        InvestorType::TaxableUS => {
            recs.push(
                "As a taxable US investor, UBTI and ECI considerations \
                 do not apply. Standard income tax treatment applies."
                    .to_string(),
            );
        }
        InvestorType::SovereignWealth => {
            // Sovereign wealth funds are not directly subject to UBTI/ECI
            // in the same way, but investments that would be high-risk for
            // other investor types still warrant attention.
            let has_sensitive_investments = results.iter().any(|r| {
                matches!(
                    r.investment_type,
                    InvestmentType::OperatingBusiness
                        | InvestmentType::MLP
                        | InvestmentType::DebtFinancedProperty
                        | InvestmentType::LeveragedRealEstate
                )
            });
            if has_sensitive_investments {
                recs.push(
                    "Sovereign wealth funds may be eligible for sovereign \
                     immunity exemptions under IRC Section 892. Consult \
                     tax counsel to confirm applicability."
                        .to_string(),
                );
                warnings.push(
                    "Sovereign immunity analysis requires case-specific \
                     legal review; this screening provides general guidance only."
                        .to_string(),
                );
            }
        }
        InvestorType::InsuranceCompany => {
            let has_sensitive_investments = results.iter().any(|r| {
                matches!(
                    r.investment_type,
                    InvestmentType::OperatingBusiness
                        | InvestmentType::MLP
                        | InvestmentType::DebtFinancedProperty
                        | InvestmentType::LeveragedRealEstate
                )
            });
            if has_sensitive_investments {
                recs.push(
                    "Insurance companies have special UBTI rules under \
                     IRC Section 512(b)(17). Review whether investments \
                     are within the insurance company's general account \
                     vs. separate account."
                        .to_string(),
                );
            }
        }
    }

    recs
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn make_investment(name: &str, inv_type: InvestmentType, amount: Money) -> InvestmentDetail {
        InvestmentDetail {
            name: name.to_string(),
            investment_type: inv_type,
            amount,
            is_leveraged: false,
            leverage_ratio: None,
            has_operating_income: false,
            us_source_income_pct: None,
        }
    }

    fn make_input(
        investor_type: InvestorType,
        investments: Vec<InvestmentDetail>,
    ) -> UbtiScreeningInput {
        UbtiScreeningInput {
            investor_type,
            investments,
            use_blocker: None,
            currency: Some(Currency::USD),
        }
    }

    // ------------------------------------------------------------------
    // Test 1: Tax-exempt operating business => High UBTI
    // ------------------------------------------------------------------
    #[test]
    fn test_tax_exempt_operating_business_high_ubti() {
        let input = make_input(
            InvestorType::TaxExemptUS,
            vec![make_investment(
                "Acme Operations",
                InvestmentType::OperatingBusiness,
                dec!(10_000_000),
            )],
        );
        let output = screen_ubti_eci(&input).unwrap();
        let r = &output.result;

        assert_eq!(r.investment_results.len(), 1);
        assert_eq!(r.investment_results[0].ubti_risk, RiskLevel::High);
        assert!(r.investment_results[0]
            .ubti_reason
            .contains("Operating business"));
    }

    // ------------------------------------------------------------------
    // Test 2: Tax-exempt public equity => Low UBTI (no concern)
    // ------------------------------------------------------------------
    #[test]
    fn test_tax_exempt_public_equity_no_ubti() {
        let input = make_input(
            InvestorType::TaxExemptUS,
            vec![make_investment(
                "S&P 500 Index",
                InvestmentType::PublicEquity,
                dec!(50_000_000),
            )],
        );
        let output = screen_ubti_eci(&input).unwrap();
        let r = &output.result;

        assert_eq!(r.investment_results[0].ubti_risk, RiskLevel::Low);
        assert_eq!(r.investment_results[0].eci_risk, RiskLevel::None);
        assert!(!r.investment_results[0].blocker_recommended);
    }

    // ------------------------------------------------------------------
    // Test 3: Foreign investor ECI from US business
    // ------------------------------------------------------------------
    #[test]
    fn test_foreign_investor_eci_from_us_business() {
        let mut inv = make_investment(
            "US Widget Co",
            InvestmentType::OperatingBusiness,
            dec!(20_000_000),
        );
        inv.us_source_income_pct = Some(dec!(0.80));

        let input = make_input(InvestorType::ForeignInvestor, vec![inv]);
        let output = screen_ubti_eci(&input).unwrap();
        let r = &output.result;

        assert_eq!(r.investment_results[0].eci_risk, RiskLevel::High);
        assert!(r.investment_results[0].eci_reason.contains("US operating"));
        assert!(r.investment_results[0].blocker_recommended);
    }

    // ------------------------------------------------------------------
    // Test 4: Foreign investor, no ECI for non-US business
    // ------------------------------------------------------------------
    #[test]
    fn test_foreign_investor_no_eci_non_us() {
        let mut inv = make_investment(
            "European Holdings",
            InvestmentType::OperatingBusiness,
            dec!(15_000_000),
        );
        inv.us_source_income_pct = Some(Decimal::ZERO);

        let input = make_input(InvestorType::ForeignInvestor, vec![inv]);
        let output = screen_ubti_eci(&input).unwrap();
        let r = &output.result;

        assert_eq!(r.investment_results[0].eci_risk, RiskLevel::None);
        assert!(!r.investment_results[0].blocker_recommended);
    }

    // ------------------------------------------------------------------
    // Test 5: Taxable US => No UBTI/ECI concern at all
    // ------------------------------------------------------------------
    #[test]
    fn test_taxable_us_no_ubti_concern() {
        let input = make_input(
            InvestorType::TaxableUS,
            vec![
                make_investment(
                    "Operating Biz",
                    InvestmentType::OperatingBusiness,
                    dec!(10_000_000),
                ),
                make_investment("MLP", InvestmentType::MLP, dec!(5_000_000)),
            ],
        );
        let output = screen_ubti_eci(&input).unwrap();
        let r = &output.result;

        for result in &r.investment_results {
            assert_eq!(result.ubti_risk, RiskLevel::None);
            assert_eq!(result.eci_risk, RiskLevel::None);
            assert!(!result.blocker_recommended);
        }
        assert_eq!(r.overall_risk, RiskLevel::None);
    }

    // ------------------------------------------------------------------
    // Test 6: Leveraged investment => UDFI / UBTI
    // ------------------------------------------------------------------
    #[test]
    fn test_leveraged_investment_udfi() {
        let mut inv = make_investment(
            "Leveraged RE Fund",
            InvestmentType::RealEstateFund,
            dec!(25_000_000),
        );
        inv.is_leveraged = true;
        inv.leverage_ratio = Some(dec!(1.5)); // 60% LTV

        let input = make_input(InvestorType::TaxExemptUS, vec![inv]);
        let output = screen_ubti_eci(&input).unwrap();
        let r = &output.result;

        assert_eq!(r.investment_results[0].ubti_risk, RiskLevel::Medium);
        assert!(r.investment_results[0].ubti_reason.contains("UDFI"));

        // Check estimated UBTI amount
        // debt_financed = 25M * 1.5 / 2.5 = 15M
        // estimated UBTI = 15M * 0.08 = 1.2M
        let est = r.investment_results[0].estimated_ubti_amount.unwrap();
        assert_eq!(est, dec!(1_200_000));
    }

    // ------------------------------------------------------------------
    // Test 7: MLP => High UBTI risk for tax-exempt
    // ------------------------------------------------------------------
    #[test]
    fn test_mlp_high_ubti_risk() {
        let input = make_input(
            InvestorType::TaxExemptUS,
            vec![make_investment(
                "Energy MLP",
                InvestmentType::MLP,
                dec!(8_000_000),
            )],
        );
        let output = screen_ubti_eci(&input).unwrap();
        let r = &output.result;

        assert_eq!(r.investment_results[0].ubti_risk, RiskLevel::High);
        assert!(r.investment_results[0].ubti_reason.contains("MLP"));
        assert!(r.investment_results[0].blocker_recommended);

        // MLP should also have PTP mitigation option
        assert!(r.investment_results[0]
            .mitigation_options
            .iter()
            .any(|o| o.contains("PTP")));
    }

    // ------------------------------------------------------------------
    // Test 8: Blocker recommendation for high-risk UBTI
    // ------------------------------------------------------------------
    #[test]
    fn test_blocker_recommendation() {
        let input = make_input(
            InvestorType::TaxExemptUS,
            vec![
                make_investment(
                    "Operating Business",
                    InvestmentType::OperatingBusiness,
                    dec!(10_000_000),
                ),
                make_investment(
                    "Public Bonds",
                    InvestmentType::PublicFixedIncome,
                    dec!(40_000_000),
                ),
            ],
        );
        let output = screen_ubti_eci(&input).unwrap();
        let r = &output.result;

        // Operating business should get blocker recommendation
        assert!(r.investment_results[0].blocker_recommended);
        // Public bonds should NOT
        assert!(!r.investment_results[1].blocker_recommended);

        // Recommendations should mention blocker
        assert!(r.recommendations.iter().any(|rec| rec.contains("blocker")));
    }

    // ------------------------------------------------------------------
    // Test 9: Blocker cost estimate
    // ------------------------------------------------------------------
    #[test]
    fn test_blocker_cost_estimate() {
        let input = UbtiScreeningInput {
            investor_type: InvestorType::TaxExemptUS,
            investments: vec![make_investment(
                "Operating Business",
                InvestmentType::OperatingBusiness,
                dec!(20_000_000),
            )],
            use_blocker: Some(true),
            currency: Some(Currency::USD),
        };
        let output = screen_ubti_eci(&input).unwrap();
        let r = &output.result;

        // Blocker cost = 20M * 0.0015 = 30,000
        let cost = r.estimated_blocker_cost.unwrap();
        assert_eq!(cost, dec!(30_000));
    }

    // ------------------------------------------------------------------
    // Test 10: Portfolio aggregate risk metrics
    // ------------------------------------------------------------------
    #[test]
    fn test_portfolio_aggregate_risk() {
        let mut leveraged_pe = make_investment(
            "PE Fund Alpha",
            InvestmentType::PrivateEquityFund,
            dec!(10_000_000),
        );
        leveraged_pe.has_operating_income = true;

        let input = make_input(
            InvestorType::TaxExemptUS,
            vec![
                make_investment(
                    "Operating Business",
                    InvestmentType::OperatingBusiness,
                    dec!(10_000_000),
                ),
                leveraged_pe,
                make_investment("S&P 500", InvestmentType::PublicEquity, dec!(30_000_000)),
            ],
        );
        let output = screen_ubti_eci(&input).unwrap();
        let r = &output.result;

        // Total screened = 10M + 10M + 30M = 50M
        assert_eq!(r.total_screened, dec!(50_000_000));

        // At-risk UBTI: Operating Business (10M, High) + PE Fund (10M, Medium) = 20M
        assert_eq!(r.total_at_risk, dec!(20_000_000));

        // UBTI risk pct = 20M / 50M = 0.40
        assert_eq!(r.ubti_risk_pct, dec!(0.4));

        // Overall risk should be High (from the operating business)
        assert_eq!(r.overall_risk, RiskLevel::High);
    }

    // ------------------------------------------------------------------
    // Test 11: Mitigation options populated
    // ------------------------------------------------------------------
    #[test]
    fn test_mitigation_options_populated() {
        let mut leveraged_hf = make_investment(
            "Leveraged Hedge Fund",
            InvestmentType::HedgeFund,
            dec!(15_000_000),
        );
        leveraged_hf.is_leveraged = true;
        leveraged_hf.leverage_ratio = Some(dec!(2.0));

        let input = make_input(InvestorType::TaxExemptUS, vec![leveraged_hf]);
        let output = screen_ubti_eci(&input).unwrap();
        let r = &output.result;

        let options = &r.investment_results[0].mitigation_options;
        assert!(
            !options.is_empty(),
            "Mitigation options should be populated"
        );

        // Should include blocker entity option
        assert!(
            options.iter().any(|o| o.contains("blocker")),
            "Should recommend blocker entity"
        );

        // Should include leverage removal option
        assert!(
            options.iter().any(|o| o.contains("leverage")),
            "Should recommend reducing leverage"
        );

        // Should include fund-of-funds option (for hedge fund)
        assert!(
            options.iter().any(|o| o.contains("fund-of-funds")),
            "Should recommend fund-of-funds with blocker"
        );
    }

    // ------------------------------------------------------------------
    // Test 12: Empty investments => error
    // ------------------------------------------------------------------
    #[test]
    fn test_empty_investments_error() {
        let input = make_input(InvestorType::TaxExemptUS, vec![]);
        let result = screen_ubti_eci(&input);

        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InsufficientData(msg) => {
                assert!(msg.contains("At least one investment"));
            }
            other => panic!("Expected InsufficientData, got: {:?}", other),
        }
    }

    // ------------------------------------------------------------------
    // Test 13: Metadata populated
    // ------------------------------------------------------------------
    #[test]
    fn test_metadata_populated() {
        let input = make_input(
            InvestorType::TaxExemptUS,
            vec![make_investment(
                "Test",
                InvestmentType::PublicEquity,
                dec!(1_000_000),
            )],
        );
        let output = screen_ubti_eci(&input).unwrap();

        assert!(!output.methodology.is_empty());
        assert!(output.methodology.contains("UBTI"));
        assert_eq!(output.metadata.precision, "rust_decimal_128bit");
        assert!(!output.metadata.version.is_empty());
    }

    // ------------------------------------------------------------------
    // Test 14: Negative amount => error
    // ------------------------------------------------------------------
    #[test]
    fn test_negative_amount_error() {
        let input = make_input(
            InvestorType::TaxExemptUS,
            vec![make_investment(
                "Bad Amount",
                InvestmentType::PublicEquity,
                dec!(-1_000_000),
            )],
        );
        let result = screen_ubti_eci(&input);

        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert!(field.contains("amount"));
            }
            other => panic!("Expected InvalidInput, got: {:?}", other),
        }
    }

    // ------------------------------------------------------------------
    // Test 15: Negative leverage ratio => error
    // ------------------------------------------------------------------
    #[test]
    fn test_negative_leverage_ratio_error() {
        let mut inv = make_investment("Bad Leverage", InvestmentType::HedgeFund, dec!(1_000_000));
        inv.leverage_ratio = Some(dec!(-0.5));

        let input = make_input(InvestorType::TaxExemptUS, vec![inv]);
        let result = screen_ubti_eci(&input);

        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert!(field.contains("leverage_ratio"));
            }
            other => panic!("Expected InvalidInput, got: {:?}", other),
        }
    }

    // ------------------------------------------------------------------
    // Test 16: Foreign investor MLP => High ECI
    // ------------------------------------------------------------------
    #[test]
    fn test_foreign_investor_mlp_eci() {
        let input = make_input(
            InvestorType::ForeignInvestor,
            vec![make_investment(
                "Energy MLP",
                InvestmentType::MLP,
                dec!(5_000_000),
            )],
        );
        let output = screen_ubti_eci(&input).unwrap();
        let r = &output.result;

        assert_eq!(r.investment_results[0].eci_risk, RiskLevel::High);
        assert_eq!(r.investment_results[0].ubti_risk, RiskLevel::None);
        assert!(r.investment_results[0].blocker_recommended);
    }

    // ------------------------------------------------------------------
    // Test 17: Sovereign wealth special guidance
    // ------------------------------------------------------------------
    #[test]
    fn test_sovereign_wealth_recommendations() {
        let input = make_input(
            InvestorType::SovereignWealth,
            vec![make_investment(
                "US Biz",
                InvestmentType::OperatingBusiness,
                dec!(50_000_000),
            )],
        );
        let output = screen_ubti_eci(&input).unwrap();
        let r = &output.result;

        // Should mention sovereign immunity
        assert!(r
            .recommendations
            .iter()
            .any(|rec| rec.contains("sovereign")));
    }
}
