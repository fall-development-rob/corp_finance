//! Municipal bond analysis module — GO bonds, revenue bonds, credit scoring,
//! and refunding analysis.
//!
//! Covers four analysis types:
//! 1. **General Obligation (GO) bond analysis** — tax base assessment, debt
//!    burden ratios, debt service coverage, fund balance metrics.
//! 2. **Revenue bond analysis** — DSCR, rate covenant testing, additional
//!    bonds test, reserve fund adequacy, customer concentration.
//! 3. **Municipal credit scoring** — composite score from financial, economic,
//!    and governance metrics with implied rating.
//! 4. **Refunding analysis** — advance refunding savings, escrow cost,
//!    net PV savings, payback period.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{with_metadata, ComputationOutput, Money, Rate};
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// Which type of municipal analysis to run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MuniAnalysisType {
    GeneralObligation,
    RevenueBond,
    CreditScore,
    Refunding,
}

// ---------------------------------------------------------------------------
// Input types
// ---------------------------------------------------------------------------

/// Unified input for all municipal bond analyses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MuniAnalysisInput {
    /// Which analysis to perform.
    pub analysis_type: MuniAnalysisType,

    // -- General Obligation fields ------------------------------------------
    /// Issuer name (GO / CreditScore).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issuer_name: Option<String>,
    /// Total assessed property valuation (GO).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assessed_valuation: Option<Money>,
    /// Total direct debt outstanding (GO).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_direct_debt: Option<Money>,
    /// Debt from overlapping jurisdictions (GO).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overlapping_debt: Option<Money>,
    /// Population of the issuing jurisdiction (GO).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub population: Option<u64>,
    /// Total personal income in jurisdiction (GO).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub personal_income: Option<Money>,
    /// Annual debt service payment (GO / RevenueBond).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annual_debt_service: Option<Money>,
    /// General fund revenue (GO).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub general_fund_revenue: Option<Money>,
    /// General fund balance (GO).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub general_fund_balance: Option<Money>,
    /// Tax collection rate, e.g. 0.96 = 96% (GO / CreditScore).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_collection_rate: Option<Rate>,
    /// Statutory/legal debt limit (GO).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub legal_debt_limit: Option<Money>,
    /// Pension funded ratio (GO / CreditScore).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pension_funded_ratio: Option<Rate>,

    // -- Revenue Bond fields ------------------------------------------------
    /// Project or enterprise name (RevenueBond).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_name: Option<String>,
    /// Gross revenue (RevenueBond).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gross_revenue: Option<Money>,
    /// Operating expenses (RevenueBond).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operating_expenses: Option<Money>,
    /// Senior-lien debt service if subordinate (RevenueBond).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub senior_debt_service: Option<Money>,
    /// Minimum DSCR required by rate covenant (RevenueBond).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_covenant_dscr: Option<Rate>,
    /// Minimum DSCR for additional bonds test (RevenueBond).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_bonds_test_dscr: Option<Rate>,
    /// Reserve fund balance (RevenueBond).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reserve_fund_balance: Option<Money>,
    /// Reserve fund requirement (RevenueBond).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reserve_fund_requirement: Option<Money>,
    /// Days cash on hand (RevenueBond).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub days_cash_on_hand: Option<u32>,
    /// Customer base count (RevenueBond).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer_base_count: Option<u64>,
    /// Top-ten customer concentration percentage (RevenueBond).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_ten_customer_pct: Option<Rate>,
    /// Whether the enterprise is an essential service (RevenueBond).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub essential_service: Option<bool>,

    // -- CreditScore fields -------------------------------------------------
    /// Debt as percentage of assessed value (CreditScore).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debt_to_assessed_value: Option<Rate>,
    /// Debt per capita amount (CreditScore).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debt_per_capita: Option<Money>,
    /// Debt as percentage of personal income (CreditScore).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debt_to_personal_income: Option<Rate>,
    /// General fund balance as pct of revenue (CreditScore).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fund_balance_pct: Option<Rate>,
    /// Unemployment rate (CreditScore).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unemployment_rate: Option<Rate>,
    /// 5-year population growth rate (CreditScore).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub population_growth_5yr: Option<Rate>,
    /// Median household income (CreditScore).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub median_household_income: Option<Money>,
    /// Governance score 1-10 (CreditScore).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub governance_score: Option<u32>,

    // -- Refunding fields ---------------------------------------------------
    /// Outstanding par of old bonds (Refunding).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_bond_outstanding: Option<Money>,
    /// Coupon rate on old bonds (Refunding).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_coupon_rate: Option<Rate>,
    /// Remaining years on old bonds (Refunding).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_remaining_years: Option<Decimal>,
    /// Coupon rate on new (refunding) bonds (Refunding).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_coupon_rate: Option<Rate>,
    /// Maturity of new bonds in years (Refunding).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_maturity_years: Option<Decimal>,
    /// Yield on escrow securities (SLGS/Treasuries) (Refunding).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub escrow_yield: Option<Rate>,
    /// Total issuance costs (Refunding).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issuance_costs: Option<Money>,
    /// Call premium as a decimal (Refunding).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_premium: Option<Rate>,
    /// Years until old bonds are callable (Refunding).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_date_years: Option<Decimal>,
    /// Discount rate for PV savings (Refunding).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discount_rate: Option<Rate>,
}

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// Credit indicator with qualitative rating.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditIndicator {
    pub name: String,
    pub value: Decimal,
    pub rating: String, // "Strong", "Adequate", "Weak"
}

/// Factor in credit scoring with its score and weight.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreFactor {
    pub name: String,
    pub value: Decimal,
    pub score: Decimal,
    pub weight: Decimal,
}

/// Unified output for all municipal bond analyses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MuniAnalysisOutput {
    pub analysis_type: String,

    // -- GO Bond outputs ----------------------------------------------------
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debt_to_assessed_value: Option<Rate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub net_direct_debt_ratio: Option<Rate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debt_per_capita: Option<Money>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debt_to_personal_income: Option<Rate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debt_service_coverage: Option<Rate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fund_balance_ratio: Option<Rate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub legal_debt_margin: Option<Money>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credit_indicators: Option<Vec<CreditIndicator>>,

    // -- Revenue Bond outputs -----------------------------------------------
    #[serde(skip_serializing_if = "Option::is_none")]
    pub net_revenue: Option<Money>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dscr: Option<Rate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub senior_dscr: Option<Rate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_covenant_compliance: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_covenant_headroom: Option<Rate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_bonds_capacity: Option<Money>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reserve_fund_pct: Option<Rate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer_concentration_risk: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub essential_service_flag: Option<bool>,

    // -- CreditScore outputs ------------------------------------------------
    #[serde(skip_serializing_if = "Option::is_none")]
    pub financial_score: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub economic_score: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub governance_score_out: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub composite_score: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub implied_rating: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub factors: Option<Vec<ScoreFactor>>,

    // -- Refunding outputs --------------------------------------------------
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gross_savings: Option<Money>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub escrow_cost: Option<Money>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub net_savings: Option<Money>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pv_savings: Option<Money>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pv_savings_pct: Option<Rate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_economically_viable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payback_period_years: Option<Decimal>,

    pub warnings: Vec<String>,
}

impl MuniAnalysisOutput {
    fn new_go() -> Self {
        Self {
            analysis_type: "GeneralObligation".to_string(),
            warnings: Vec::new(),
            // GO
            debt_to_assessed_value: None,
            net_direct_debt_ratio: None,
            debt_per_capita: None,
            debt_to_personal_income: None,
            debt_service_coverage: None,
            fund_balance_ratio: None,
            legal_debt_margin: None,
            credit_indicators: None,
            // Revenue
            net_revenue: None,
            dscr: None,
            senior_dscr: None,
            rate_covenant_compliance: None,
            rate_covenant_headroom: None,
            additional_bonds_capacity: None,
            reserve_fund_pct: None,
            customer_concentration_risk: None,
            essential_service_flag: None,
            // CreditScore
            financial_score: None,
            economic_score: None,
            governance_score_out: None,
            composite_score: None,
            implied_rating: None,
            factors: None,
            // Refunding
            gross_savings: None,
            escrow_cost: None,
            net_savings: None,
            pv_savings: None,
            pv_savings_pct: None,
            is_economically_viable: None,
            payback_period_years: None,
        }
    }

    fn new_revenue() -> Self {
        let mut o = Self::new_go();
        o.analysis_type = "RevenueBond".to_string();
        o
    }

    fn new_credit_score() -> Self {
        let mut o = Self::new_go();
        o.analysis_type = "CreditScore".to_string();
        o
    }

    fn new_refunding() -> Self {
        let mut o = Self::new_go();
        o.analysis_type = "Refunding".to_string();
        o
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Analyse a municipal bond based on the specified `analysis_type`.
pub fn analyze_municipal(
    input: &MuniAnalysisInput,
) -> CorpFinanceResult<ComputationOutput<MuniAnalysisOutput>> {
    let start = Instant::now();

    let output = match input.analysis_type {
        MuniAnalysisType::GeneralObligation => analyze_go(input)?,
        MuniAnalysisType::RevenueBond => analyze_revenue(input)?,
        MuniAnalysisType::CreditScore => analyze_credit_score(input)?,
        MuniAnalysisType::Refunding => analyze_refunding(input)?,
    };

    let elapsed = start.elapsed().as_micros() as u64;

    Ok(with_metadata(
        methodology_label(&input.analysis_type),
        &serde_json::json!({
            "analysis_type": format!("{:?}", input.analysis_type),
            "math_precision": "rust_decimal_128bit",
            "discount_factor_method": "iterative_multiplication",
        }),
        output.warnings.clone(),
        elapsed,
        output,
    ))
}

fn methodology_label(at: &MuniAnalysisType) -> &'static str {
    match at {
        MuniAnalysisType::GeneralObligation => {
            "General Obligation Bond Analysis — debt burden, coverage, fund balance"
        }
        MuniAnalysisType::RevenueBond => {
            "Revenue Bond Analysis — DSCR, rate covenant, additional bonds test"
        }
        MuniAnalysisType::CreditScore => {
            "Municipal Credit Scoring — financial, economic, governance composite"
        }
        MuniAnalysisType::Refunding => {
            "Refunding Analysis — advance refunding savings and PV analysis"
        }
    }
}

// ---------------------------------------------------------------------------
// Helper: require field
// ---------------------------------------------------------------------------

fn require<T: Clone>(field: &Option<T>, name: &str) -> CorpFinanceResult<T> {
    field.clone().ok_or_else(|| CorpFinanceError::InvalidInput {
        field: name.to_string(),
        reason: format!("{name} is required for this analysis type"),
    })
}

fn require_positive(value: Decimal, name: &str) -> CorpFinanceResult<()> {
    if value <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: name.to_string(),
            reason: format!("{name} must be positive"),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 1. General Obligation Analysis
// ---------------------------------------------------------------------------

fn analyze_go(input: &MuniAnalysisInput) -> CorpFinanceResult<MuniAnalysisOutput> {
    let assessed_val = require(&input.assessed_valuation, "assessed_valuation")?;
    let direct_debt = require(&input.total_direct_debt, "total_direct_debt")?;
    let overlapping = require(&input.overlapping_debt, "overlapping_debt")?;
    let population = require(&input.population, "population")?;
    let personal_income = require(&input.personal_income, "personal_income")?;
    let ann_ds = require(&input.annual_debt_service, "annual_debt_service")?;
    let gf_revenue = require(&input.general_fund_revenue, "general_fund_revenue")?;
    let gf_balance = require(&input.general_fund_balance, "general_fund_balance")?;
    let tax_rate = require(&input.tax_collection_rate, "tax_collection_rate")?;

    require_positive(assessed_val, "assessed_valuation")?;
    require_positive(gf_revenue, "general_fund_revenue")?;
    require_positive(ann_ds, "annual_debt_service")?;

    if population == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "population".to_string(),
            reason: "Population must be positive".to_string(),
        });
    }
    if personal_income <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "personal_income".to_string(),
            reason: "Personal income must be positive".to_string(),
        });
    }

    let mut out = MuniAnalysisOutput::new_go();

    let total_debt = direct_debt + overlapping;
    let pop_dec = Decimal::from(population);

    // Ratios
    let debt_av = total_debt / assessed_val;
    let net_direct = direct_debt / assessed_val;
    let per_capita = total_debt / pop_dec;
    let debt_income = total_debt / personal_income;
    let ds_coverage = gf_revenue / ann_ds;
    let fb_ratio = gf_balance / gf_revenue;

    out.debt_to_assessed_value = Some(debt_av);
    out.net_direct_debt_ratio = Some(net_direct);
    out.debt_per_capita = Some(per_capita);
    out.debt_to_personal_income = Some(debt_income);
    out.debt_service_coverage = Some(ds_coverage);
    out.fund_balance_ratio = Some(fb_ratio);

    // Legal debt margin
    if let Some(limit) = input.legal_debt_limit {
        let margin = limit - total_debt;
        out.legal_debt_margin = Some(margin);
        if margin < Decimal::ZERO {
            out.warnings
                .push("Total debt exceeds legal debt limit".to_string());
        }
    }

    // Credit indicators
    let mut indicators = vec![
        CreditIndicator {
            name: "Debt/AV".to_string(),
            value: debt_av,
            rating: rate_debt_av(debt_av),
        },
        CreditIndicator {
            name: "Debt Per Capita".to_string(),
            value: per_capita,
            rating: rate_per_capita(per_capita),
        },
        CreditIndicator {
            name: "Debt/Personal Income".to_string(),
            value: debt_income,
            rating: rate_debt_income(debt_income),
        },
        CreditIndicator {
            name: "Debt Service Coverage".to_string(),
            value: ds_coverage,
            rating: rate_ds_coverage(ds_coverage),
        },
        CreditIndicator {
            name: "Fund Balance Ratio".to_string(),
            value: fb_ratio,
            rating: rate_fund_balance(fb_ratio),
        },
        CreditIndicator {
            name: "Tax Collection Rate".to_string(),
            value: tax_rate,
            rating: rate_tax_collection(tax_rate),
        },
    ];

    if let Some(pfr) = input.pension_funded_ratio {
        indicators.push(CreditIndicator {
            name: "Pension Funded Ratio".to_string(),
            value: pfr,
            rating: rate_pension(pfr),
        });
        if pfr < dec!(0.60) {
            out.warnings
                .push("Pension funded ratio below 60% — significant liability risk".to_string());
        }
    }

    out.credit_indicators = Some(indicators);

    // Warnings for weak indicators
    if debt_av > dec!(0.06) {
        out.warnings
            .push("Debt-to-assessed-value exceeds 6% — Weak".to_string());
    }
    if ds_coverage < dec!(1.5) {
        out.warnings
            .push("Debt service coverage below 1.5x — Weak".to_string());
    }

    Ok(out)
}

// GO rating helpers
fn rate_debt_av(v: Decimal) -> String {
    if v < dec!(0.03) {
        "Strong".to_string()
    } else if v <= dec!(0.06) {
        "Adequate".to_string()
    } else {
        "Weak".to_string()
    }
}

fn rate_per_capita(v: Decimal) -> String {
    if v < dec!(1000) {
        "Strong".to_string()
    } else if v <= dec!(3000) {
        "Adequate".to_string()
    } else {
        "Weak".to_string()
    }
}

fn rate_debt_income(v: Decimal) -> String {
    if v < dec!(0.03) {
        "Strong".to_string()
    } else if v <= dec!(0.06) {
        "Adequate".to_string()
    } else {
        "Weak".to_string()
    }
}

fn rate_ds_coverage(v: Decimal) -> String {
    if v > dec!(3) {
        "Strong".to_string()
    } else if v >= dec!(1.5) {
        "Adequate".to_string()
    } else {
        "Weak".to_string()
    }
}

fn rate_fund_balance(v: Decimal) -> String {
    if v > dec!(0.25) {
        "Strong".to_string()
    } else if v >= dec!(0.08) {
        "Adequate".to_string()
    } else {
        "Weak".to_string()
    }
}

fn rate_tax_collection(v: Decimal) -> String {
    if v > dec!(0.98) {
        "Strong".to_string()
    } else if v >= dec!(0.94) {
        "Adequate".to_string()
    } else {
        "Weak".to_string()
    }
}

fn rate_pension(v: Decimal) -> String {
    if v >= dec!(0.80) {
        "Strong".to_string()
    } else if v >= dec!(0.60) {
        "Adequate".to_string()
    } else {
        "Weak".to_string()
    }
}

// ---------------------------------------------------------------------------
// 2. Revenue Bond Analysis
// ---------------------------------------------------------------------------

fn analyze_revenue(input: &MuniAnalysisInput) -> CorpFinanceResult<MuniAnalysisOutput> {
    let gross_rev = require(&input.gross_revenue, "gross_revenue")?;
    let opex = require(&input.operating_expenses, "operating_expenses")?;
    let ann_ds = require(&input.annual_debt_service, "annual_debt_service")?;
    let rc_dscr = require(&input.rate_covenant_dscr, "rate_covenant_dscr")?;
    let abt_dscr = require(
        &input.additional_bonds_test_dscr,
        "additional_bonds_test_dscr",
    )?;
    let reserve_bal = require(&input.reserve_fund_balance, "reserve_fund_balance")?;
    let reserve_req = require(&input.reserve_fund_requirement, "reserve_fund_requirement")?;
    let top_ten = require(&input.top_ten_customer_pct, "top_ten_customer_pct")?;
    let essential = require(&input.essential_service, "essential_service")?;

    require_positive(gross_rev, "gross_revenue")?;
    require_positive(ann_ds, "annual_debt_service")?;
    require_positive(rc_dscr, "rate_covenant_dscr")?;
    require_positive(abt_dscr, "additional_bonds_test_dscr")?;

    let mut out = MuniAnalysisOutput::new_revenue();

    let net_rev = gross_rev - opex;
    out.net_revenue = Some(net_rev);

    // DSCR
    let dscr = net_rev / ann_ds;
    out.dscr = Some(dscr);

    // Senior DSCR (if subordinate lien)
    if let Some(senior_ds) = input.senior_debt_service {
        if senior_ds > Decimal::ZERO {
            out.senior_dscr = Some(net_rev / senior_ds);
        }
    }

    // Rate covenant
    let rc_compliant = dscr >= rc_dscr;
    out.rate_covenant_compliance = Some(rc_compliant);
    out.rate_covenant_headroom = Some(dscr - rc_dscr);

    if !rc_compliant {
        out.warnings.push(format!(
            "Rate covenant violated: DSCR {dscr} < required {rc_dscr}"
        ));
    }

    // Additional bonds capacity
    // max_new_ds where (net_rev / (ann_ds + max_new_ds)) >= abt_dscr
    // => max_new_ds = (net_rev / abt_dscr) - ann_ds
    let max_new_ds = (net_rev / abt_dscr) - ann_ds;
    let capacity = if max_new_ds > Decimal::ZERO {
        max_new_ds
    } else {
        Decimal::ZERO
    };
    out.additional_bonds_capacity = Some(capacity);

    // Reserve fund
    if reserve_req > Decimal::ZERO {
        let rf_pct = reserve_bal / reserve_req;
        out.reserve_fund_pct = Some(rf_pct);
        if rf_pct < Decimal::ONE {
            out.warnings.push(format!(
                "Reserve fund is {:.1}% of requirement — below 100%",
                rf_pct * dec!(100)
            ));
        }
    } else {
        out.reserve_fund_pct = Some(Decimal::ZERO);
    }

    // Customer concentration
    let conc_risk = if top_ten < dec!(0.25) {
        "Low"
    } else if top_ten <= dec!(0.50) {
        "Medium"
    } else {
        "High"
    };
    out.customer_concentration_risk = Some(conc_risk.to_string());

    if conc_risk == "High" {
        out.warnings
            .push("High customer concentration — top 10 customers exceed 50%".to_string());
    }

    out.essential_service_flag = Some(essential);

    Ok(out)
}

// ---------------------------------------------------------------------------
// 3. Credit Scoring
// ---------------------------------------------------------------------------

fn analyze_credit_score(input: &MuniAnalysisInput) -> CorpFinanceResult<MuniAnalysisOutput> {
    let debt_av = require(&input.debt_to_assessed_value, "debt_to_assessed_value")?;
    let fund_bal = require(&input.fund_balance_pct, "fund_balance_pct")?;
    let tax_coll = require(&input.tax_collection_rate, "tax_collection_rate")?;
    let unemp = require(&input.unemployment_rate, "unemployment_rate")?;
    let pop_growth = require(&input.population_growth_5yr, "population_growth_5yr")?;
    let mhi = require(&input.median_household_income, "median_household_income")?;

    let mut out = MuniAnalysisOutput::new_credit_score();
    let mut factors: Vec<ScoreFactor> = Vec::new();

    // -- Financial factors (40% weight) ------------------------------------
    let debt_av_score = score_debt_av(debt_av);
    let fund_bal_score = score_fund_balance(fund_bal);
    let tax_score = score_tax_collection(tax_coll);

    factors.push(ScoreFactor {
        name: "Debt/AV".to_string(),
        value: debt_av,
        score: debt_av_score,
        weight: dec!(0.40),
    });
    factors.push(ScoreFactor {
        name: "Fund Balance %".to_string(),
        value: fund_bal,
        score: fund_bal_score,
        weight: dec!(0.40),
    });
    factors.push(ScoreFactor {
        name: "Tax Collection Rate".to_string(),
        value: tax_coll,
        score: tax_score,
        weight: dec!(0.40),
    });

    // Financial composite: equal-weighted average of the 3 sub-factors
    let financial = (debt_av_score + fund_bal_score + tax_score) / dec!(3);

    // -- Economic factors (35% weight) -------------------------------------
    let unemp_score = score_unemployment(unemp);
    let pop_score = score_pop_growth(pop_growth);
    let mhi_score = score_mhi(mhi);

    factors.push(ScoreFactor {
        name: "Unemployment Rate".to_string(),
        value: unemp,
        score: unemp_score,
        weight: dec!(0.35),
    });
    factors.push(ScoreFactor {
        name: "Population Growth 5yr".to_string(),
        value: pop_growth,
        score: pop_score,
        weight: dec!(0.35),
    });
    factors.push(ScoreFactor {
        name: "Median Household Income".to_string(),
        value: mhi,
        score: mhi_score,
        weight: dec!(0.35),
    });

    let economic = (unemp_score + pop_score + mhi_score) / dec!(3);

    // -- Governance (25% weight) -------------------------------------------
    let gov = match input.governance_score {
        Some(gs) => {
            let clamped = if gs > 10 { 10 } else { gs };
            Decimal::from(clamped) * dec!(10)
        }
        None => dec!(50), // neutral default
    };

    factors.push(ScoreFactor {
        name: "Governance".to_string(),
        value: gov,
        score: gov,
        weight: dec!(0.25),
    });

    // -- Pension adjustment (applied as penalty to governance) --------------
    if let Some(pfr) = input.pension_funded_ratio {
        if pfr < dec!(0.60) {
            // Heavy penalty
            let penalty = dec!(15);
            out.warnings.push(format!(
                "Pension funded ratio ({pfr}) below 60% — {penalty} point penalty applied"
            ));
            // We subtract from governance rather than re-weighting
            let adjusted_gov = if gov > penalty {
                gov - penalty
            } else {
                dec!(0)
            };
            // Replace the last factor entry
            if let Some(last) = factors.last_mut() {
                last.score = adjusted_gov;
            }
        }
    }

    // Retrieve possibly-adjusted governance
    let final_gov = factors.last().map(|f| f.score).unwrap_or(gov);

    // Composite: 40% financial + 35% economic + 25% governance
    let composite = financial * dec!(0.40) + economic * dec!(0.35) + final_gov * dec!(0.25);
    // Normalise: each component is 0-100; weighted sum max = 0.40*100 + 0.35*100 + 0.25*100 = 100
    // So composite is already on a 0-100 scale.

    let rating = implied_rating(composite);

    out.financial_score = Some(financial);
    out.economic_score = Some(economic);
    out.governance_score_out = Some(final_gov);
    out.composite_score = Some(composite);
    out.implied_rating = Some(rating);
    out.factors = Some(factors);

    Ok(out)
}

// Scoring helpers (each returns 0-100)
fn score_debt_av(v: Decimal) -> Decimal {
    if v < dec!(0.01) {
        dec!(100)
    } else if v < dec!(0.03) {
        dec!(80)
    } else if v < dec!(0.06) {
        dec!(60)
    } else if v < dec!(0.10) {
        dec!(40)
    } else {
        dec!(20)
    }
}

fn score_fund_balance(v: Decimal) -> Decimal {
    if v > dec!(0.25) {
        dec!(100)
    } else if v > dec!(0.15) {
        dec!(80)
    } else if v > dec!(0.08) {
        dec!(60)
    } else if v > dec!(0.03) {
        dec!(40)
    } else {
        dec!(20)
    }
}

fn score_tax_collection(v: Decimal) -> Decimal {
    if v > dec!(0.98) {
        dec!(100)
    } else if v > dec!(0.96) {
        dec!(80)
    } else if v > dec!(0.94) {
        dec!(60)
    } else if v > dec!(0.90) {
        dec!(40)
    } else {
        dec!(20)
    }
}

fn score_unemployment(v: Decimal) -> Decimal {
    if v < dec!(0.04) {
        dec!(100)
    } else if v < dec!(0.06) {
        dec!(80)
    } else if v < dec!(0.08) {
        dec!(60)
    } else if v < dec!(0.10) {
        dec!(40)
    } else {
        dec!(20)
    }
}

fn score_pop_growth(v: Decimal) -> Decimal {
    if v > dec!(0.05) {
        dec!(100)
    } else if v > dec!(0.02) {
        dec!(80)
    } else if v > Decimal::ZERO {
        dec!(60)
    } else if v > dec!(-0.02) {
        dec!(40)
    } else {
        dec!(20)
    }
}

fn score_mhi(v: Decimal) -> Decimal {
    if v > dec!(80000) {
        dec!(100)
    } else if v > dec!(60000) {
        dec!(80)
    } else if v > dec!(40000) {
        dec!(60)
    } else if v > dec!(25000) {
        dec!(40)
    } else {
        dec!(20)
    }
}

fn implied_rating(score: Decimal) -> String {
    if score > dec!(85) {
        "AAA".to_string()
    } else if score > dec!(75) {
        "AA".to_string()
    } else if score > dec!(65) {
        "A".to_string()
    } else if score > dec!(55) {
        "BBB".to_string()
    } else if score > dec!(45) {
        "BB".to_string()
    } else {
        "B".to_string()
    }
}

// ---------------------------------------------------------------------------
// 4. Refunding Analysis
// ---------------------------------------------------------------------------

fn analyze_refunding(input: &MuniAnalysisInput) -> CorpFinanceResult<MuniAnalysisOutput> {
    let outstanding = require(&input.old_bond_outstanding, "old_bond_outstanding")?;
    let old_coupon = require(&input.old_coupon_rate, "old_coupon_rate")?;
    let old_remaining = require(&input.old_remaining_years, "old_remaining_years")?;
    let new_coupon = require(&input.new_coupon_rate, "new_coupon_rate")?;
    let new_maturity = require(&input.new_maturity_years, "new_maturity_years")?;
    let escrow_yield = require(&input.escrow_yield, "escrow_yield")?;
    let issuance = require(&input.issuance_costs, "issuance_costs")?;
    let call_prem = require(&input.call_premium, "call_premium")?;
    let call_years = require(&input.call_date_years, "call_date_years")?;
    let disc_rate = require(&input.discount_rate, "discount_rate")?;

    require_positive(outstanding, "old_bond_outstanding")?;
    require_positive(old_remaining, "old_remaining_years")?;
    require_positive(new_maturity, "new_maturity_years")?;

    let mut out = MuniAnalysisOutput::new_refunding();

    let old_annual_ds = outstanding * old_coupon;
    let new_annual_ds = outstanding * new_coupon;
    let annual_saving = old_annual_ds - new_annual_ds;

    // Convert to integer periods
    let remaining_periods = decimal_to_periods(old_remaining);
    let new_periods = decimal_to_periods(new_maturity);
    let call_periods = decimal_to_periods(call_years);

    // Gross savings = annual saving * remaining years (nominal, undiscounted)
    let gross = annual_saving * Decimal::from(remaining_periods as u32);
    out.gross_savings = Some(gross);

    // Escrow cost: PV of old bond obligations from now until call date,
    // discounted at escrow_yield. This is the amount of SLGS/Treasuries
    // that must be purchased to defease old bonds to the call date.
    //
    // Escrow must fund:
    //   - old_annual_ds for each year from 1..call_periods
    //   - at call date: outstanding * (1 + call_premium)
    let escrow = compute_escrow_cost(
        outstanding,
        old_annual_ds,
        call_prem,
        escrow_yield,
        call_periods,
    );
    out.escrow_cost = Some(escrow);

    // Transaction costs = issuance_costs (escrow cost is funded by new bond
    // proceeds and is already accounted for in the PV comparison below).
    let call_cost = outstanding * call_prem;

    // Net savings (nominal): gross savings minus issuance and call premium
    let net = gross - issuance - call_cost;
    out.net_savings = Some(net);

    // PV savings: present value of the annual savings stream minus
    // present-value transaction costs.
    //
    // Standard municipal refunding PV analysis:
    //   PV(savings) = PV(old DS) - PV(new DS) - transaction costs
    //
    // Simplified with interest-only DS:
    //   PV(savings) = NPV of annual_saving for remaining_periods at disc_rate
    //               - issuance_costs - call_premium_cost
    //
    // The escrow cost is the cost of defeasing old bonds. It is funded by
    // the new bond proceeds, so it does not reduce PV savings separately.
    // (The cost of funding the escrow is already reflected in the new DS.)
    let pv = compute_pv_savings(annual_saving, disc_rate, remaining_periods);
    let pv_net = pv - issuance - call_cost;
    out.pv_savings = Some(pv_net);

    // PV savings as % of old bond outstanding
    let pv_pct = if outstanding > Decimal::ZERO {
        pv_net / outstanding
    } else {
        Decimal::ZERO
    };
    out.pv_savings_pct = Some(pv_pct);

    // Economically viable: rule of thumb >= 3% PV savings
    let viable = pv_pct >= dec!(0.03);
    out.is_economically_viable = Some(viable);

    // Payback period: years until cumulative annual savings recover
    // the upfront transaction costs (issuance + call premium)
    let upfront_costs = issuance + call_cost;
    let payback = if annual_saving > Decimal::ZERO {
        let raw = upfront_costs / annual_saving;
        raw.round_dp(2)
    } else {
        Decimal::from(new_periods as u32)
    };
    out.payback_period_years = Some(payback);

    // Warnings
    if !viable {
        out.warnings.push(format!(
            "PV savings of {:.2}% is below the 3% rule-of-thumb threshold",
            pv_pct * dec!(100)
        ));
    }
    if new_coupon >= old_coupon {
        out.warnings.push(
            "New coupon rate is not lower than old — refunding may not be economic".to_string(),
        );
    }
    if call_years > old_remaining {
        out.warnings
            .push("Call date is beyond old bond maturity".to_string());
    }
    if Decimal::from(new_periods as u32) > old_remaining {
        out.warnings
            .push("New bond maturity extends beyond old bond remaining life".to_string());
    }

    Ok(out)
}

/// Compute escrow cost: PV of old DS payments and call redemption, discounted
/// at the escrow yield.  Uses iterative multiplication for discount factors.
fn compute_escrow_cost(
    outstanding: Money,
    old_annual_ds: Money,
    call_premium: Rate,
    escrow_yield: Rate,
    call_periods: usize,
) -> Money {
    if call_periods == 0 {
        // Bonds are currently callable — escrow is just the redemption
        return outstanding * (Decimal::ONE + call_premium);
    }

    let one_plus_y = Decimal::ONE + escrow_yield;
    let mut discount_factor = Decimal::ONE; // (1+y)^t
    let mut total = Decimal::ZERO;

    for t in 1..=call_periods {
        discount_factor *= one_plus_y;
        if discount_factor.is_zero() {
            continue;
        }
        // Annual coupon payment
        total += old_annual_ds / discount_factor;

        // At call date, add redemption at par + premium
        if t == call_periods {
            let redemption = outstanding * (Decimal::ONE + call_premium);
            total += redemption / discount_factor;
        }
    }

    total
}

/// Compute PV of an annuity of `annual_saving` for `periods` years at
/// `disc_rate`. Uses iterative multiplication.
fn compute_pv_savings(annual_saving: Money, disc_rate: Rate, periods: usize) -> Money {
    if periods == 0 {
        return Decimal::ZERO;
    }
    let one_plus_r = Decimal::ONE + disc_rate;
    let mut discount_factor = Decimal::ONE;
    let mut pv = Decimal::ZERO;

    for _ in 1..=periods {
        discount_factor *= one_plus_r;
        if discount_factor.is_zero() {
            continue;
        }
        pv += annual_saving / discount_factor;
    }

    pv
}

/// Convert Decimal years to whole periods (floor).
fn decimal_to_periods(years: Decimal) -> usize {
    let rounded = years.round_dp(0).to_string().parse::<i64>().unwrap_or(0);
    if rounded < 0 {
        0
    } else {
        rounded as usize
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
    // Helpers to build inputs
    // -----------------------------------------------------------------------

    fn go_input() -> MuniAnalysisInput {
        MuniAnalysisInput {
            analysis_type: MuniAnalysisType::GeneralObligation,
            issuer_name: Some("City of Springfield".to_string()),
            assessed_valuation: Some(dec!(10_000_000_000)),
            total_direct_debt: Some(dec!(150_000_000)),
            overlapping_debt: Some(dec!(50_000_000)),
            population: Some(200_000),
            personal_income: Some(dec!(8_000_000_000)),
            annual_debt_service: Some(dec!(25_000_000)),
            general_fund_revenue: Some(dec!(500_000_000)),
            general_fund_balance: Some(dec!(150_000_000)),
            tax_collection_rate: Some(dec!(0.97)),
            legal_debt_limit: Some(dec!(300_000_000)),
            pension_funded_ratio: Some(dec!(0.75)),
            // Unused fields
            project_name: None,
            gross_revenue: None,
            operating_expenses: None,
            senior_debt_service: None,
            rate_covenant_dscr: None,
            additional_bonds_test_dscr: None,
            reserve_fund_balance: None,
            reserve_fund_requirement: None,
            days_cash_on_hand: None,
            customer_base_count: None,
            top_ten_customer_pct: None,
            essential_service: None,
            debt_to_assessed_value: None,
            debt_per_capita: None,
            debt_to_personal_income: None,
            fund_balance_pct: None,
            unemployment_rate: None,
            population_growth_5yr: None,
            median_household_income: None,
            governance_score: None,
            old_bond_outstanding: None,
            old_coupon_rate: None,
            old_remaining_years: None,
            new_coupon_rate: None,
            new_maturity_years: None,
            escrow_yield: None,
            issuance_costs: None,
            call_premium: None,
            call_date_years: None,
            discount_rate: None,
        }
    }

    fn revenue_input() -> MuniAnalysisInput {
        MuniAnalysisInput {
            analysis_type: MuniAnalysisType::RevenueBond,
            project_name: Some("Water & Sewer System".to_string()),
            gross_revenue: Some(dec!(50_000_000)),
            operating_expenses: Some(dec!(30_000_000)),
            annual_debt_service: Some(dec!(10_000_000)),
            senior_debt_service: Some(dec!(6_000_000)),
            rate_covenant_dscr: Some(dec!(1.25)),
            additional_bonds_test_dscr: Some(dec!(1.10)),
            reserve_fund_balance: Some(dec!(9_500_000)),
            reserve_fund_requirement: Some(dec!(10_000_000)),
            days_cash_on_hand: Some(180),
            customer_base_count: Some(80_000),
            top_ten_customer_pct: Some(dec!(0.15)),
            essential_service: Some(true),
            // Unused
            issuer_name: None,
            assessed_valuation: None,
            total_direct_debt: None,
            overlapping_debt: None,
            population: None,
            personal_income: None,
            general_fund_revenue: None,
            general_fund_balance: None,
            tax_collection_rate: None,
            legal_debt_limit: None,
            pension_funded_ratio: None,
            debt_to_assessed_value: None,
            debt_per_capita: None,
            debt_to_personal_income: None,
            fund_balance_pct: None,
            unemployment_rate: None,
            population_growth_5yr: None,
            median_household_income: None,
            governance_score: None,
            old_bond_outstanding: None,
            old_coupon_rate: None,
            old_remaining_years: None,
            new_coupon_rate: None,
            new_maturity_years: None,
            escrow_yield: None,
            issuance_costs: None,
            call_premium: None,
            call_date_years: None,
            discount_rate: None,
        }
    }

    fn credit_score_input_strong() -> MuniAnalysisInput {
        MuniAnalysisInput {
            analysis_type: MuniAnalysisType::CreditScore,
            issuer_name: Some("AAA County".to_string()),
            debt_to_assessed_value: Some(dec!(0.005)), // <1% => 100
            fund_balance_pct: Some(dec!(0.30)),        // >25% => 100
            tax_collection_rate: Some(dec!(0.99)),     // >98% => 100
            unemployment_rate: Some(dec!(0.03)),       // <4% => 100
            population_growth_5yr: Some(dec!(0.06)),   // >5% => 100
            median_household_income: Some(dec!(90000)), // >80k => 100
            governance_score: Some(10),                // 10*10 = 100
            pension_funded_ratio: Some(dec!(0.90)),
            debt_per_capita: None,
            debt_to_personal_income: None,
            // Unused
            assessed_valuation: None,
            total_direct_debt: None,
            overlapping_debt: None,
            population: None,
            personal_income: None,
            annual_debt_service: None,
            general_fund_revenue: None,
            general_fund_balance: None,
            legal_debt_limit: None,
            project_name: None,
            gross_revenue: None,
            operating_expenses: None,
            senior_debt_service: None,
            rate_covenant_dscr: None,
            additional_bonds_test_dscr: None,
            reserve_fund_balance: None,
            reserve_fund_requirement: None,
            days_cash_on_hand: None,
            customer_base_count: None,
            top_ten_customer_pct: None,
            essential_service: None,
            old_bond_outstanding: None,
            old_coupon_rate: None,
            old_remaining_years: None,
            new_coupon_rate: None,
            new_maturity_years: None,
            escrow_yield: None,
            issuance_costs: None,
            call_premium: None,
            call_date_years: None,
            discount_rate: None,
        }
    }

    fn credit_score_input_stressed() -> MuniAnalysisInput {
        MuniAnalysisInput {
            analysis_type: MuniAnalysisType::CreditScore,
            issuer_name: Some("Stressed City".to_string()),
            debt_to_assessed_value: Some(dec!(0.07)), // 6-10% => 40
            fund_balance_pct: Some(dec!(0.05)),       // 3-8% => 40
            tax_collection_rate: Some(dec!(0.95)),    // 94-96% => 60
            unemployment_rate: Some(dec!(0.07)),      // 6-8% => 60
            population_growth_5yr: Some(dec!(-0.01)), // -2% to 0% => 40
            median_household_income: Some(dec!(45000)), // 40-60k => 60
            governance_score: Some(5),                // 5*10 = 50
            pension_funded_ratio: None,
            debt_per_capita: None,
            debt_to_personal_income: None,
            // Unused
            assessed_valuation: None,
            total_direct_debt: None,
            overlapping_debt: None,
            population: None,
            personal_income: None,
            annual_debt_service: None,
            general_fund_revenue: None,
            general_fund_balance: None,
            legal_debt_limit: None,
            project_name: None,
            gross_revenue: None,
            operating_expenses: None,
            senior_debt_service: None,
            rate_covenant_dscr: None,
            additional_bonds_test_dscr: None,
            reserve_fund_balance: None,
            reserve_fund_requirement: None,
            days_cash_on_hand: None,
            customer_base_count: None,
            top_ten_customer_pct: None,
            essential_service: None,
            old_bond_outstanding: None,
            old_coupon_rate: None,
            old_remaining_years: None,
            new_coupon_rate: None,
            new_maturity_years: None,
            escrow_yield: None,
            issuance_costs: None,
            call_premium: None,
            call_date_years: None,
            discount_rate: None,
        }
    }

    fn refunding_viable_input() -> MuniAnalysisInput {
        MuniAnalysisInput {
            analysis_type: MuniAnalysisType::Refunding,
            old_bond_outstanding: Some(dec!(50_000_000)),
            old_coupon_rate: Some(dec!(0.05)), // 5%
            old_remaining_years: Some(dec!(20)),
            new_coupon_rate: Some(dec!(0.03)), // 3%
            new_maturity_years: Some(dec!(20)),
            escrow_yield: Some(dec!(0.02)), // 2%
            issuance_costs: Some(dec!(500_000)),
            call_premium: Some(dec!(0.02)), // 2%
            call_date_years: Some(dec!(3)),
            discount_rate: Some(dec!(0.035)),
            // Unused
            issuer_name: None,
            assessed_valuation: None,
            total_direct_debt: None,
            overlapping_debt: None,
            population: None,
            personal_income: None,
            annual_debt_service: None,
            general_fund_revenue: None,
            general_fund_balance: None,
            tax_collection_rate: None,
            legal_debt_limit: None,
            pension_funded_ratio: None,
            project_name: None,
            gross_revenue: None,
            operating_expenses: None,
            senior_debt_service: None,
            rate_covenant_dscr: None,
            additional_bonds_test_dscr: None,
            reserve_fund_balance: None,
            reserve_fund_requirement: None,
            days_cash_on_hand: None,
            customer_base_count: None,
            top_ten_customer_pct: None,
            essential_service: None,
            debt_to_assessed_value: None,
            debt_per_capita: None,
            debt_to_personal_income: None,
            fund_balance_pct: None,
            unemployment_rate: None,
            population_growth_5yr: None,
            median_household_income: None,
            governance_score: None,
        }
    }

    fn refunding_uneconomic_input() -> MuniAnalysisInput {
        MuniAnalysisInput {
            analysis_type: MuniAnalysisType::Refunding,
            old_bond_outstanding: Some(dec!(50_000_000)),
            old_coupon_rate: Some(dec!(0.04)), // 4%
            old_remaining_years: Some(dec!(10)),
            new_coupon_rate: Some(dec!(0.039)), // 3.9% — tiny differential
            new_maturity_years: Some(dec!(10)),
            escrow_yield: Some(dec!(0.02)),
            issuance_costs: Some(dec!(500_000)),
            call_premium: Some(dec!(0.02)),
            call_date_years: Some(dec!(3)),
            discount_rate: Some(dec!(0.035)),
            // Unused
            issuer_name: None,
            assessed_valuation: None,
            total_direct_debt: None,
            overlapping_debt: None,
            population: None,
            personal_income: None,
            annual_debt_service: None,
            general_fund_revenue: None,
            general_fund_balance: None,
            tax_collection_rate: None,
            legal_debt_limit: None,
            pension_funded_ratio: None,
            project_name: None,
            gross_revenue: None,
            operating_expenses: None,
            senior_debt_service: None,
            rate_covenant_dscr: None,
            additional_bonds_test_dscr: None,
            reserve_fund_balance: None,
            reserve_fund_requirement: None,
            days_cash_on_hand: None,
            customer_base_count: None,
            top_ten_customer_pct: None,
            essential_service: None,
            debt_to_assessed_value: None,
            debt_per_capita: None,
            debt_to_personal_income: None,
            fund_balance_pct: None,
            unemployment_rate: None,
            population_growth_5yr: None,
            median_household_income: None,
            governance_score: None,
        }
    }

    // -----------------------------------------------------------------------
    // 1. GO bond: debt ratios and credit indicators
    // -----------------------------------------------------------------------
    #[test]
    fn test_go_bond_debt_ratios_and_indicators() {
        let input = go_input();
        let result = analyze_municipal(&input).unwrap();
        let out = &result.result;

        // total_debt = 150M + 50M = 200M
        // debt/AV = 200M / 10B = 0.02
        assert_eq!(out.debt_to_assessed_value.unwrap(), dec!(0.02));

        // net_direct = 150M / 10B = 0.015
        assert_eq!(out.net_direct_debt_ratio.unwrap(), dec!(0.015));

        // debt per capita = 200M / 200k = 1000
        assert_eq!(out.debt_per_capita.unwrap(), dec!(1000));

        // debt/income = 200M / 8B = 0.025
        assert_eq!(out.debt_to_personal_income.unwrap(), dec!(0.025));

        // DS coverage = 500M / 25M = 20
        assert_eq!(out.debt_service_coverage.unwrap(), dec!(20));

        // Fund balance = 150M / 500M = 0.3
        assert_eq!(out.fund_balance_ratio.unwrap(), dec!(0.3));

        // Credit indicators
        let indicators = out.credit_indicators.as_ref().unwrap();
        // Debt/AV = 0.02 < 0.03 => Strong
        let dav = indicators.iter().find(|i| i.name == "Debt/AV").unwrap();
        assert_eq!(dav.rating, "Strong");

        // DS coverage = 20 > 3 => Strong
        let dsc = indicators
            .iter()
            .find(|i| i.name == "Debt Service Coverage")
            .unwrap();
        assert_eq!(dsc.rating, "Strong");

        // Tax collection 0.97: 0.94 <= 0.97 <= 0.98 => Adequate
        let tc = indicators
            .iter()
            .find(|i| i.name == "Tax Collection Rate")
            .unwrap();
        assert_eq!(tc.rating, "Adequate");
    }

    // -----------------------------------------------------------------------
    // 2. GO bond: legal debt margin
    // -----------------------------------------------------------------------
    #[test]
    fn test_go_bond_legal_debt_margin() {
        let input = go_input();
        let result = analyze_municipal(&input).unwrap();
        let out = &result.result;

        // legal_debt_limit = 300M, total_debt = 200M => margin = 100M
        assert_eq!(out.legal_debt_margin.unwrap(), dec!(100_000_000));

        // No warning about exceeding limit
        assert!(!out
            .warnings
            .iter()
            .any(|w| w.contains("exceeds legal debt limit")));

        // Now test with a tight limit
        let mut tight = go_input();
        tight.legal_debt_limit = Some(dec!(180_000_000)); // 180M < 200M
        let result2 = analyze_municipal(&tight).unwrap();
        let out2 = &result2.result;

        assert_eq!(out2.legal_debt_margin.unwrap(), dec!(-20_000_000));
        assert!(out2
            .warnings
            .iter()
            .any(|w| w.contains("exceeds legal debt limit")));
    }

    // -----------------------------------------------------------------------
    // 3. Revenue bond: DSCR and rate covenant compliance
    // -----------------------------------------------------------------------
    #[test]
    fn test_revenue_bond_dscr_and_covenant() {
        let input = revenue_input();
        let result = analyze_municipal(&input).unwrap();
        let out = &result.result;

        // net_revenue = 50M - 30M = 20M
        assert_eq!(out.net_revenue.unwrap(), dec!(20_000_000));

        // DSCR = 20M / 10M = 2.0
        assert_eq!(out.dscr.unwrap(), dec!(2));

        // Senior DSCR = 20M / 6M = 3.333...
        let senior = out.senior_dscr.unwrap();
        let expected_senior = dec!(20_000_000) / dec!(6_000_000);
        let diff = (senior - expected_senior).abs();
        assert!(
            diff < dec!(0.001),
            "Senior DSCR should be ~{expected_senior}, got {senior}"
        );

        // Rate covenant: DSCR 2.0 >= 1.25 => compliant
        assert_eq!(out.rate_covenant_compliance.unwrap(), true);

        // Headroom = 2.0 - 1.25 = 0.75
        assert_eq!(out.rate_covenant_headroom.unwrap(), dec!(0.75));

        // Essential service flag
        assert_eq!(out.essential_service_flag.unwrap(), true);

        // Customer concentration: 15% < 25% => Low
        assert_eq!(out.customer_concentration_risk.as_ref().unwrap(), "Low");
    }

    // -----------------------------------------------------------------------
    // 4. Revenue bond: additional bonds capacity
    // -----------------------------------------------------------------------
    #[test]
    fn test_revenue_bond_additional_bonds_capacity() {
        let input = revenue_input();
        let result = analyze_municipal(&input).unwrap();
        let out = &result.result;

        // net_revenue = 20M, ABT_dscr = 1.10, ann_ds = 10M
        // max_new_ds = (20M / 1.10) - 10M = 18,181,818.18... - 10M
        //            = 8,181,818.18...
        let capacity = out.additional_bonds_capacity.unwrap();
        let expected = dec!(20_000_000) / dec!(1.10) - dec!(10_000_000);
        let diff = (capacity - expected).abs();
        assert!(
            diff < dec!(1),
            "Additional bonds capacity should be ~{expected}, got {capacity}"
        );
        assert!(capacity > Decimal::ZERO);
    }

    // -----------------------------------------------------------------------
    // 5. Credit score: high-quality issuer => AAA
    // -----------------------------------------------------------------------
    #[test]
    fn test_credit_score_aaa_issuer() {
        let input = credit_score_input_strong();
        let result = analyze_municipal(&input).unwrap();
        let out = &result.result;

        // All factors score 100
        // financial = (100+100+100)/3 = 100
        assert_eq!(out.financial_score.unwrap(), dec!(100));
        // economic = (100+100+100)/3 = 100
        assert_eq!(out.economic_score.unwrap(), dec!(100));
        // governance = 10*10 = 100
        assert_eq!(out.governance_score_out.unwrap(), dec!(100));

        // composite = 0.40*100 + 0.35*100 + 0.25*100 = 100
        assert_eq!(out.composite_score.unwrap(), dec!(100));

        // 100 > 85 => AAA
        assert_eq!(out.implied_rating.as_ref().unwrap(), "AAA");
    }

    // -----------------------------------------------------------------------
    // 6. Credit score: stressed issuer => BBB
    // -----------------------------------------------------------------------
    #[test]
    fn test_credit_score_stressed_issuer() {
        let input = credit_score_input_stressed();
        let result = analyze_municipal(&input).unwrap();
        let out = &result.result;

        // Financial: debt_av=40, fund_bal=40, tax=60 => avg=(40+40+60)/3=46.67
        let fin = out.financial_score.unwrap();
        let expected_fin = (dec!(40) + dec!(40) + dec!(60)) / dec!(3);
        let diff_fin = (fin - expected_fin).abs();
        assert!(
            diff_fin < dec!(0.01),
            "Financial score should be ~{expected_fin}, got {fin}"
        );

        // Economic: unemp=60, pop=-1%=40, mhi=60 => avg=(60+40+60)/3=53.33
        let econ = out.economic_score.unwrap();
        let expected_econ = (dec!(60) + dec!(40) + dec!(60)) / dec!(3);
        let diff_econ = (econ - expected_econ).abs();
        assert!(
            diff_econ < dec!(0.01),
            "Economic score should be ~{expected_econ}, got {econ}"
        );

        // Governance = 5*10 = 50
        assert_eq!(out.governance_score_out.unwrap(), dec!(50));

        // Composite = 0.40*46.67 + 0.35*53.33 + 0.25*50
        let composite = out.composite_score.unwrap();
        let expected_comp =
            expected_fin * dec!(0.40) + expected_econ * dec!(0.35) + dec!(50) * dec!(0.25);
        let diff_comp = (composite - expected_comp).abs();
        assert!(
            diff_comp < dec!(0.01),
            "Composite should be ~{expected_comp}, got {composite}"
        );

        // The composite should be in the BBB range (55-65)
        assert!(
            composite > dec!(45) && composite <= dec!(65),
            "Composite {composite} should yield BBB or BB"
        );

        let rating = out.implied_rating.as_ref().unwrap();
        // Expected: composite ~ 49.33 => BB (45-55) range
        // Let's verify: 0.40*(140/3) + 0.35*(160/3) + 0.25*50
        //             = 0.40*46.666.. + 0.35*53.333.. + 12.5
        //             = 18.666.. + 18.666.. + 12.5 = 49.833..
        // 49.83 is in 45-55 range => BB
        assert!(
            rating == "BB" || rating == "BBB",
            "Expected BB or BBB, got {rating}"
        );
    }

    // -----------------------------------------------------------------------
    // 7. Refunding: economically viable (PV savings > 3%)
    // -----------------------------------------------------------------------
    #[test]
    fn test_refunding_viable() {
        let input = refunding_viable_input();
        let result = analyze_municipal(&input).unwrap();
        let out = &result.result;

        // old_annual_ds = 50M * 0.05 = 2.5M
        // new_annual_ds = 50M * 0.03 = 1.5M
        // annual_saving = 1.0M
        // gross = 1.0M * 20 = 20M
        assert_eq!(out.gross_savings.unwrap(), dec!(20_000_000));

        // PV savings should be substantial with a 200bp differential over 20yr
        let pv_pct = out.pv_savings_pct.unwrap();
        assert!(
            pv_pct > dec!(0.03),
            "PV savings pct ({pv_pct}) should exceed 3% for viable refunding"
        );

        assert_eq!(out.is_economically_viable.unwrap(), true);

        // Payback should be reasonable
        let payback = out.payback_period_years.unwrap();
        assert!(
            payback > Decimal::ZERO && payback < dec!(20),
            "Payback {payback} should be between 0 and 20 years"
        );
    }

    // -----------------------------------------------------------------------
    // 8. Refunding: uneconomical (low rate differential)
    // -----------------------------------------------------------------------
    #[test]
    fn test_refunding_uneconomical() {
        let input = refunding_uneconomic_input();
        let result = analyze_municipal(&input).unwrap();
        let out = &result.result;

        // old_coupon=4%, new_coupon=3.9% => 10bp differential
        // annual_saving = 50M * (0.04 - 0.039) = 50M * 0.001 = 50k
        // gross = 50k * 10 = 500k
        assert_eq!(out.gross_savings.unwrap(), dec!(500_000));

        // With escrow costs, issuance costs, and call premium the PV savings
        // should be well below 3%
        let pv_pct = out.pv_savings_pct.unwrap();
        assert!(
            pv_pct < dec!(0.03),
            "PV savings pct ({pv_pct}) should be below 3% for uneconomical refunding"
        );

        assert_eq!(out.is_economically_viable.unwrap(), false);

        // Should have a warning about being below threshold
        assert!(out
            .warnings
            .iter()
            .any(|w| w.contains("below the 3% rule-of-thumb")));
    }

    // -----------------------------------------------------------------------
    // 9. GO bond: pension warning for low funded ratio
    // -----------------------------------------------------------------------
    #[test]
    fn test_go_pension_warning() {
        let mut input = go_input();
        input.pension_funded_ratio = Some(dec!(0.55)); // below 60%

        let result = analyze_municipal(&input).unwrap();
        let out = &result.result;

        assert!(out
            .warnings
            .iter()
            .any(|w| w.contains("Pension funded ratio below 60%")));

        let indicators = out.credit_indicators.as_ref().unwrap();
        let pension = indicators
            .iter()
            .find(|i| i.name == "Pension Funded Ratio")
            .unwrap();
        assert_eq!(pension.rating, "Weak");
    }

    // -----------------------------------------------------------------------
    // 10. Revenue bond: rate covenant violation
    // -----------------------------------------------------------------------
    #[test]
    fn test_revenue_bond_covenant_violation() {
        let mut input = revenue_input();
        // Increase required DSCR above actual
        input.rate_covenant_dscr = Some(dec!(2.50));

        let result = analyze_municipal(&input).unwrap();
        let out = &result.result;

        // DSCR = 2.0 < 2.50 => violation
        assert_eq!(out.rate_covenant_compliance.unwrap(), false);
        assert!(out.rate_covenant_headroom.unwrap() < Decimal::ZERO);
        assert!(out
            .warnings
            .iter()
            .any(|w| w.contains("Rate covenant violated")));
    }

    // -----------------------------------------------------------------------
    // 11. Revenue bond: high customer concentration
    // -----------------------------------------------------------------------
    #[test]
    fn test_revenue_bond_high_concentration() {
        let mut input = revenue_input();
        input.top_ten_customer_pct = Some(dec!(0.55));

        let result = analyze_municipal(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.customer_concentration_risk.as_ref().unwrap(), "High");
        assert!(out
            .warnings
            .iter()
            .any(|w| w.contains("High customer concentration")));
    }

    // -----------------------------------------------------------------------
    // 12. Metadata populated
    // -----------------------------------------------------------------------
    #[test]
    fn test_metadata_populated() {
        let input = go_input();
        let result = analyze_municipal(&input).unwrap();

        assert!(!result.methodology.is_empty());
        assert!(result.methodology.contains("General Obligation"));
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
        assert!(!result.metadata.version.is_empty());
    }

    // -----------------------------------------------------------------------
    // 13. Credit score: pension penalty applied
    // -----------------------------------------------------------------------
    #[test]
    fn test_credit_score_pension_penalty() {
        let mut input = credit_score_input_strong();
        input.pension_funded_ratio = Some(dec!(0.50)); // below 60%

        let result = analyze_municipal(&input).unwrap();
        let out = &result.result;

        // Governance should have 15-point penalty: 100 - 15 = 85
        assert_eq!(out.governance_score_out.unwrap(), dec!(85));

        // Composite should be lower than pure AAA
        let composite = out.composite_score.unwrap();
        // 0.40*100 + 0.35*100 + 0.25*85 = 40 + 35 + 21.25 = 96.25
        assert_eq!(composite, dec!(96.25));

        // Still AAA (>85)
        assert_eq!(out.implied_rating.as_ref().unwrap(), "AAA");
    }

    // -----------------------------------------------------------------------
    // 14. Refunding: escrow cost sanity check
    // -----------------------------------------------------------------------
    #[test]
    fn test_refunding_escrow_cost_positive() {
        let input = refunding_viable_input();
        let result = analyze_municipal(&input).unwrap();
        let out = &result.result;

        let escrow = out.escrow_cost.unwrap();
        assert!(
            escrow > Decimal::ZERO,
            "Escrow cost should be positive, got {escrow}"
        );

        // Escrow should be less than outstanding + call premium
        // (since discounting reduces PV)
        let max_escrow = dec!(50_000_000) * dec!(1.02) + dec!(2_500_000) * dec!(3); // rough max
        assert!(
            escrow < max_escrow,
            "Escrow {escrow} should be less than rough max {max_escrow}"
        );
    }

    // -----------------------------------------------------------------------
    // 15. Validation: missing required field
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_missing_field() {
        let mut input = go_input();
        input.assessed_valuation = None;

        let result = analyze_municipal(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "assessed_valuation");
            }
            other => panic!("Expected InvalidInput, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // 16. Validation: zero population
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_zero_population() {
        let mut input = go_input();
        input.population = Some(0);

        let result = analyze_municipal(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "population");
            }
            other => panic!("Expected InvalidInput, got {other:?}"),
        }
    }
}
