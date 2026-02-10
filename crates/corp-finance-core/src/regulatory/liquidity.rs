use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{with_metadata, ComputationOutput, Money, Rate};
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// LCR Input Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LcrInput {
    pub institution_name: String,
    pub hqla: HqlaPortfolio,
    pub cash_outflows: Vec<CashOutflow>,
    pub cash_inflows: Vec<CashInflow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HqlaPortfolio {
    /// Level 1: cash, central bank reserves, govt bonds
    pub level1_assets: Vec<HqlaAsset>,
    /// Level 2A: GSE bonds, 20% RW corporate bonds
    pub level2a_assets: Vec<HqlaAsset>,
    /// Level 2B: lower quality (RMBS, corporate bonds, equities)
    pub level2b_assets: Vec<HqlaAsset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HqlaAsset {
    pub name: String,
    pub market_value: Money,
    /// Override haircut; if None, standard haircut is used (L1=0%, L2A=15%, L2B=50%)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub haircut: Option<Rate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashOutflow {
    pub category: OutflowCategory,
    pub amount: Money,
    /// Override run-off rate; if None, standard rate is used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_off_rate: Option<Rate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutflowCategory {
    /// 5% run-off
    RetailStableDeposits,
    /// 10% run-off
    RetailLessStable,
    /// 25% run-off
    UnsecuredWholesaleOperational,
    /// 40% run-off
    UnsecuredWholesaleNonOperational,
    /// 100% run-off
    UnsecuredWholesaleFinancial,
    /// 0% run-off
    SecuredFundingCentral,
    /// 0% run-off
    SecuredFundingLevel1,
    /// 15% run-off
    SecuredFundingLevel2A,
    /// 100% run-off
    SecuredFundingOther,
    /// 10% run-off (blended)
    CreditFacilities,
    /// 100% run-off
    LiquidityFacilities,
    /// 100% run-off
    Other,
}

impl OutflowCategory {
    /// Standard Basel III run-off rate for each outflow category.
    fn standard_rate(&self) -> Rate {
        match self {
            Self::RetailStableDeposits => dec!(0.05),
            Self::RetailLessStable => dec!(0.10),
            Self::UnsecuredWholesaleOperational => dec!(0.25),
            Self::UnsecuredWholesaleNonOperational => dec!(0.40),
            Self::UnsecuredWholesaleFinancial => dec!(1.00),
            Self::SecuredFundingCentral => dec!(0.00),
            Self::SecuredFundingLevel1 => dec!(0.00),
            Self::SecuredFundingLevel2A => dec!(0.15),
            Self::SecuredFundingOther => dec!(1.00),
            Self::CreditFacilities => dec!(0.10),
            Self::LiquidityFacilities => dec!(1.00),
            Self::Other => dec!(1.00),
        }
    }
}

impl std::fmt::Display for OutflowCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RetailStableDeposits => write!(f, "Retail Stable Deposits"),
            Self::RetailLessStable => write!(f, "Retail Less Stable Deposits"),
            Self::UnsecuredWholesaleOperational => {
                write!(f, "Unsecured Wholesale Operational")
            }
            Self::UnsecuredWholesaleNonOperational => {
                write!(f, "Unsecured Wholesale Non-Operational")
            }
            Self::UnsecuredWholesaleFinancial => {
                write!(f, "Unsecured Wholesale Financial")
            }
            Self::SecuredFundingCentral => write!(f, "Secured Funding (Central Bank)"),
            Self::SecuredFundingLevel1 => write!(f, "Secured Funding (Level 1)"),
            Self::SecuredFundingLevel2A => write!(f, "Secured Funding (Level 2A)"),
            Self::SecuredFundingOther => write!(f, "Secured Funding (Other)"),
            Self::CreditFacilities => write!(f, "Credit Facilities"),
            Self::LiquidityFacilities => write!(f, "Liquidity Facilities"),
            Self::Other => write!(f, "Other Outflows"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashInflow {
    pub category: InflowCategory,
    pub amount: Money,
    /// Override inflow rate; if None, standard rate is used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inflow_rate: Option<Rate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InflowCategory {
    /// 50% inflow
    RetailLoans,
    /// 50% inflow
    WholesaleNonFinancial,
    /// 100% inflow
    WholesaleFinancial,
    /// 0% inflow (collateral returned)
    SecuredLendingLevel1,
    /// 15% inflow
    SecuredLendingLevel2A,
    /// 100% inflow
    SecuredLendingOther,
    /// 50% inflow
    Other,
}

impl InflowCategory {
    /// Standard Basel III inflow rate for each inflow category.
    fn standard_rate(&self) -> Rate {
        match self {
            Self::RetailLoans => dec!(0.50),
            Self::WholesaleNonFinancial => dec!(0.50),
            Self::WholesaleFinancial => dec!(1.00),
            Self::SecuredLendingLevel1 => dec!(0.00),
            Self::SecuredLendingLevel2A => dec!(0.15),
            Self::SecuredLendingOther => dec!(1.00),
            Self::Other => dec!(0.50),
        }
    }
}

impl std::fmt::Display for InflowCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RetailLoans => write!(f, "Retail Loans"),
            Self::WholesaleNonFinancial => write!(f, "Wholesale Non-Financial"),
            Self::WholesaleFinancial => write!(f, "Wholesale Financial"),
            Self::SecuredLendingLevel1 => write!(f, "Secured Lending (Level 1)"),
            Self::SecuredLendingLevel2A => write!(f, "Secured Lending (Level 2A)"),
            Self::SecuredLendingOther => write!(f, "Secured Lending (Other)"),
            Self::Other => write!(f, "Other Inflows"),
        }
    }
}

// ---------------------------------------------------------------------------
// LCR Output Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LcrOutput {
    pub total_hqla: Money,
    pub hqla_breakdown: HqlaBreakdown,
    pub total_outflows: Money,
    pub total_inflows: Money,
    pub net_outflows: Money,
    pub lcr_ratio: Rate,
    pub meets_requirement: bool,
    pub surplus_deficit: Money,
    pub outflow_details: Vec<FlowDetail>,
    pub inflow_details: Vec<FlowDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HqlaBreakdown {
    pub level1: Money,
    /// After 15% haircut (or custom)
    pub level2a: Money,
    /// After 25-50% haircut (or custom)
    pub level2b: Money,
    /// Whether the Level 2 total cap (40% of HQLA) was applied
    pub level2_cap_applied: bool,
    /// Whether the Level 2B cap (15% of HQLA) was applied
    pub level2b_cap_applied: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowDetail {
    pub category: String,
    pub gross_amount: Money,
    pub rate: Rate,
    pub weighted_amount: Money,
}

// ---------------------------------------------------------------------------
// NSFR Input Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NsfrInput {
    pub institution_name: String,
    pub available_funding: Vec<FundingSource>,
    pub required_funding: Vec<FundingRequirement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingSource {
    pub category: AsfCategory,
    pub amount: Money,
    /// Override ASF factor; if None, standard factor is used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asf_factor: Option<Rate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AsfCategory {
    /// 100% ASF
    RegulatoryCapital,
    /// 95% ASF
    StableRetailDeposits,
    /// 90% ASF
    LessStableRetailDeposits,
    /// 100% ASF
    WholesaleFundingGt1Y,
    /// 50% ASF
    WholesaleFunding6mTo1Y,
    /// 0% ASF
    WholesaleFundingLt6M,
    /// 0% ASF
    Other,
}

impl AsfCategory {
    /// Standard Basel III ASF factor.
    fn standard_factor(&self) -> Rate {
        match self {
            Self::RegulatoryCapital => dec!(1.00),
            Self::StableRetailDeposits => dec!(0.95),
            Self::LessStableRetailDeposits => dec!(0.90),
            Self::WholesaleFundingGt1Y => dec!(1.00),
            Self::WholesaleFunding6mTo1Y => dec!(0.50),
            Self::WholesaleFundingLt6M => dec!(0.00),
            Self::Other => dec!(0.00),
        }
    }
}

impl std::fmt::Display for AsfCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RegulatoryCapital => write!(f, "Regulatory Capital"),
            Self::StableRetailDeposits => write!(f, "Stable Retail Deposits"),
            Self::LessStableRetailDeposits => {
                write!(f, "Less Stable Retail Deposits")
            }
            Self::WholesaleFundingGt1Y => write!(f, "Wholesale Funding >1Y"),
            Self::WholesaleFunding6mTo1Y => write!(f, "Wholesale Funding 6M-1Y"),
            Self::WholesaleFundingLt6M => write!(f, "Wholesale Funding <6M"),
            Self::Other => write!(f, "Other"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingRequirement {
    pub category: RsfCategory,
    pub amount: Money,
    /// Override RSF factor; if None, standard factor is used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rsf_factor: Option<Rate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RsfCategory {
    /// 0% RSF
    Cash,
    /// 0% RSF
    CentralBankReserves,
    /// 5% RSF
    Level1Hqla,
    /// 15% RSF
    Level2aHqla,
    /// 50% RSF
    Level2bHqla,
    /// 10% RSF
    LoansToFILt6M,
    /// 50% RSF
    LoansToFI6mTo1Y,
    /// 65% RSF
    ResidentialMortgages,
    /// 85% RSF
    RetailLoans,
    /// 85% RSF
    CorporateLoansGt1Y,
    /// 100% RSF
    NonPerformingLoans,
    /// 100% RSF
    FixedAssets,
    /// 100% RSF
    Other,
}

impl RsfCategory {
    /// Standard Basel III RSF factor.
    fn standard_factor(&self) -> Rate {
        match self {
            Self::Cash => dec!(0.00),
            Self::CentralBankReserves => dec!(0.00),
            Self::Level1Hqla => dec!(0.05),
            Self::Level2aHqla => dec!(0.15),
            Self::Level2bHqla => dec!(0.50),
            Self::LoansToFILt6M => dec!(0.10),
            Self::LoansToFI6mTo1Y => dec!(0.50),
            Self::ResidentialMortgages => dec!(0.65),
            Self::RetailLoans => dec!(0.85),
            Self::CorporateLoansGt1Y => dec!(0.85),
            Self::NonPerformingLoans => dec!(1.00),
            Self::FixedAssets => dec!(1.00),
            Self::Other => dec!(1.00),
        }
    }
}

impl std::fmt::Display for RsfCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cash => write!(f, "Cash"),
            Self::CentralBankReserves => write!(f, "Central Bank Reserves"),
            Self::Level1Hqla => write!(f, "Level 1 HQLA"),
            Self::Level2aHqla => write!(f, "Level 2A HQLA"),
            Self::Level2bHqla => write!(f, "Level 2B HQLA"),
            Self::LoansToFILt6M => write!(f, "Loans to FI <6M"),
            Self::LoansToFI6mTo1Y => write!(f, "Loans to FI 6M-1Y"),
            Self::ResidentialMortgages => write!(f, "Residential Mortgages"),
            Self::RetailLoans => write!(f, "Retail Loans"),
            Self::CorporateLoansGt1Y => write!(f, "Corporate Loans >1Y"),
            Self::NonPerformingLoans => write!(f, "Non-Performing Loans"),
            Self::FixedAssets => write!(f, "Fixed Assets"),
            Self::Other => write!(f, "Other"),
        }
    }
}

// ---------------------------------------------------------------------------
// NSFR Output Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NsfrOutput {
    pub available_stable_funding: Money,
    pub required_stable_funding: Money,
    pub nsfr_ratio: Rate,
    pub meets_requirement: bool,
    pub surplus_deficit: Money,
    pub asf_details: Vec<FundingDetail>,
    pub rsf_details: Vec<FundingDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingDetail {
    pub category: String,
    pub amount: Money,
    pub factor: Rate,
    pub weighted_amount: Money,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const DEFAULT_HAIRCUT_L1: Decimal = dec!(0.00);
const DEFAULT_HAIRCUT_L2A: Decimal = dec!(0.15);
const DEFAULT_HAIRCUT_L2B: Decimal = dec!(0.50);

/// Level 2 cap: 40% of adjusted total HQLA
const LEVEL2_CAP_RATIO: Decimal = dec!(0.40);
/// Level 2B cap: 15% of adjusted total HQLA
const LEVEL2B_CAP_RATIO: Decimal = dec!(0.15);

/// Inflow cap: 75% of total outflows
const INFLOW_CAP_RATIO: Decimal = dec!(0.75);

/// Minimum LCR requirement: 100%
const LCR_MIN_REQUIREMENT: Decimal = dec!(1.00);

/// Minimum NSFR requirement: 100%
const NSFR_MIN_REQUIREMENT: Decimal = dec!(1.00);

// ---------------------------------------------------------------------------
// LCR Calculation
// ---------------------------------------------------------------------------

/// Calculate the Basel III Liquidity Coverage Ratio (LCR).
///
/// LCR = HQLA / Net Cash Outflows >= 100%
///
/// HQLA is adjusted for haircuts and subject to level caps.
/// Net cash outflows = total outflows - min(total inflows, 0.75 * total outflows).
pub fn calculate_lcr(input: &LcrInput) -> CorpFinanceResult<ComputationOutput<LcrOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    validate_lcr_input(input, &mut warnings)?;

    // -- Calculate raw HQLA by level (after per-asset haircuts) --
    let raw_l1 = sum_hqla_level(&input.hqla.level1_assets, DEFAULT_HAIRCUT_L1);
    let raw_l2a = sum_hqla_level(&input.hqla.level2a_assets, DEFAULT_HAIRCUT_L2A);
    let raw_l2b = sum_hqla_level(&input.hqla.level2b_assets, DEFAULT_HAIRCUT_L2B);

    // -- Apply HQLA composition caps --
    let (adj_l1, adj_l2a, adj_l2b, l2_cap_applied, l2b_cap_applied) =
        apply_hqla_caps(raw_l1, raw_l2a, raw_l2b, &mut warnings);

    let total_hqla = adj_l1 + adj_l2a + adj_l2b;

    // -- Calculate weighted outflows --
    let mut outflow_details = Vec::with_capacity(input.cash_outflows.len());
    let mut total_outflows = Decimal::ZERO;

    for outflow in &input.cash_outflows {
        let rate = outflow
            .run_off_rate
            .unwrap_or_else(|| outflow.category.standard_rate());
        let weighted = outflow.amount * rate;
        total_outflows += weighted;
        outflow_details.push(FlowDetail {
            category: outflow.category.to_string(),
            gross_amount: outflow.amount,
            rate,
            weighted_amount: weighted,
        });
    }

    // -- Calculate weighted inflows --
    let mut inflow_details = Vec::with_capacity(input.cash_inflows.len());
    let mut raw_inflows = Decimal::ZERO;

    for inflow in &input.cash_inflows {
        let rate = inflow
            .inflow_rate
            .unwrap_or_else(|| inflow.category.standard_rate());
        let weighted = inflow.amount * rate;
        raw_inflows += weighted;
        inflow_details.push(FlowDetail {
            category: inflow.category.to_string(),
            gross_amount: inflow.amount,
            rate,
            weighted_amount: weighted,
        });
    }

    // -- Cap inflows at 75% of outflows --
    let inflow_cap = total_outflows * INFLOW_CAP_RATIO;
    let total_inflows = if raw_inflows > inflow_cap {
        warnings.push(format!(
            "Inflows capped at 75% of outflows: raw {} -> capped {}",
            raw_inflows, inflow_cap
        ));
        inflow_cap
    } else {
        raw_inflows
    };

    // -- Net outflows (floor at 1 to prevent division by zero) --
    let net_outflows_raw = total_outflows - total_inflows;
    let net_outflows = if net_outflows_raw <= Decimal::ZERO {
        warnings.push("Net outflows non-positive; floored to 1 for ratio calculation.".to_string());
        Decimal::ONE
    } else {
        net_outflows_raw
    };

    // -- LCR ratio --
    let lcr_ratio = total_hqla / net_outflows;
    let meets_requirement = lcr_ratio >= LCR_MIN_REQUIREMENT;
    let surplus_deficit = total_hqla - net_outflows;

    let output = LcrOutput {
        total_hqla,
        hqla_breakdown: HqlaBreakdown {
            level1: adj_l1,
            level2a: adj_l2a,
            level2b: adj_l2b,
            level2_cap_applied: l2_cap_applied,
            level2b_cap_applied: l2b_cap_applied,
        },
        total_outflows,
        total_inflows,
        net_outflows,
        lcr_ratio,
        meets_requirement,
        surplus_deficit,
        outflow_details,
        inflow_details,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    let assumptions = serde_json::json!({
        "framework": "Basel III LCR",
        "formula": "LCR = HQLA / max(outflows - min(inflows, 0.75 * outflows), 1)",
        "hqla_haircuts": {
            "level1": "0%",
            "level2a": "15%",
            "level2b": "50% (default)"
        },
        "caps": {
            "level2_total": "40% of adjusted HQLA",
            "level2b": "15% of adjusted HQLA",
            "inflows": "75% of total outflows"
        },
        "minimum_requirement": "100%"
    });

    Ok(with_metadata(
        "Basel III Liquidity Coverage Ratio (LCR)",
        &assumptions,
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// NSFR Calculation
// ---------------------------------------------------------------------------

/// Calculate the Basel III Net Stable Funding Ratio (NSFR).
///
/// NSFR = Available Stable Funding (ASF) / Required Stable Funding (RSF) >= 100%
pub fn calculate_nsfr(input: &NsfrInput) -> CorpFinanceResult<ComputationOutput<NsfrOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    validate_nsfr_input(input, &mut warnings)?;

    // -- Calculate ASF --
    let mut asf_details = Vec::with_capacity(input.available_funding.len());
    let mut total_asf = Decimal::ZERO;

    for source in &input.available_funding {
        let factor = source
            .asf_factor
            .unwrap_or_else(|| source.category.standard_factor());
        let weighted = source.amount * factor;
        total_asf += weighted;
        asf_details.push(FundingDetail {
            category: source.category.to_string(),
            amount: source.amount,
            factor,
            weighted_amount: weighted,
        });
    }

    // -- Calculate RSF --
    let mut rsf_details = Vec::with_capacity(input.required_funding.len());
    let mut total_rsf = Decimal::ZERO;

    for requirement in &input.required_funding {
        let factor = requirement
            .rsf_factor
            .unwrap_or_else(|| requirement.category.standard_factor());
        let weighted = requirement.amount * factor;
        total_rsf += weighted;
        rsf_details.push(FundingDetail {
            category: requirement.category.to_string(),
            amount: requirement.amount,
            factor,
            weighted_amount: weighted,
        });
    }

    // -- NSFR ratio (floor RSF at 1 to prevent division by zero) --
    let rsf_denominator = if total_rsf <= Decimal::ZERO {
        warnings
            .push("Required stable funding is non-positive; floored to 1 for ratio.".to_string());
        Decimal::ONE
    } else {
        total_rsf
    };

    let nsfr_ratio = total_asf / rsf_denominator;
    let meets_requirement = nsfr_ratio >= NSFR_MIN_REQUIREMENT;
    let surplus_deficit = total_asf - total_rsf;

    let output = NsfrOutput {
        available_stable_funding: total_asf,
        required_stable_funding: total_rsf,
        nsfr_ratio,
        meets_requirement,
        surplus_deficit,
        asf_details,
        rsf_details,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    let assumptions = serde_json::json!({
        "framework": "Basel III NSFR",
        "formula": "NSFR = ASF / RSF >= 100%",
        "asf_factors": {
            "regulatory_capital": "100%",
            "stable_retail": "95%",
            "less_stable_retail": "90%",
            "wholesale_gt_1y": "100%",
            "wholesale_6m_1y": "50%",
            "wholesale_lt_6m": "0%"
        },
        "minimum_requirement": "100%"
    });

    Ok(with_metadata(
        "Basel III Net Stable Funding Ratio (NSFR)",
        &assumptions,
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Sum HQLA assets for a given level, applying per-asset haircuts.
fn sum_hqla_level(assets: &[HqlaAsset], default_haircut: Rate) -> Money {
    assets
        .iter()
        .map(|a| {
            let haircut = a.haircut.unwrap_or(default_haircut);
            a.market_value * (Decimal::ONE - haircut)
        })
        .sum()
}

/// Apply Basel III HQLA composition caps iteratively:
///
/// 1. Level 2B <= 15% of adjusted total HQLA
/// 2. Level 2 (2A + 2B) <= 40% of adjusted total HQLA
///
/// Returns (adj_l1, adj_l2a, adj_l2b, l2_cap_applied, l2b_cap_applied).
fn apply_hqla_caps(
    l1: Money,
    l2a: Money,
    l2b: Money,
    warnings: &mut Vec<String>,
) -> (Money, Money, Money, bool, bool) {
    let mut adj_l2b = l2b;
    let mut adj_l2a = l2a;
    let mut l2b_cap_applied = false;
    let mut l2_cap_applied = false;

    // Cap Level 2B at 15% of total HQLA.
    // L2B <= 15% * (L1 + L2A + L2B)
    // => L2B * (1 - 0.15) <= 0.15 * (L1 + L2A)
    // => L2B <= (L1 + L2A) * 0.15 / 0.85
    let l2b_max = (l1 + adj_l2a) * LEVEL2B_CAP_RATIO / (Decimal::ONE - LEVEL2B_CAP_RATIO);
    if adj_l2b > l2b_max {
        warnings.push(format!(
            "Level 2B HQLA capped: {} -> {} (15% of adjusted HQLA)",
            adj_l2b, l2b_max
        ));
        adj_l2b = l2b_max;
        l2b_cap_applied = true;
    }

    // Cap total Level 2 (L2A + L2B) at 40% of total HQLA.
    // (L2A + L2B) <= 40% * (L1 + L2A + L2B)
    // => (L2A + L2B) * 0.60 <= 0.40 * L1
    // => (L2A + L2B) <= L1 * 0.40 / 0.60
    let l2_total = adj_l2a + adj_l2b;
    let l2_max = l1 * LEVEL2_CAP_RATIO / (Decimal::ONE - LEVEL2_CAP_RATIO);
    if l2_total > l2_max {
        let excess = l2_total - l2_max;
        if excess <= adj_l2a {
            adj_l2a -= excess;
        } else {
            adj_l2a = Decimal::ZERO;
            adj_l2b = l2_max;
        }
        warnings.push(format!(
            "Level 2 HQLA capped: total was {}, max {} (40% of adjusted HQLA)",
            l2_total, l2_max
        ));
        l2_cap_applied = true;
    }

    (l1, adj_l2a, adj_l2b, l2_cap_applied, l2b_cap_applied)
}

fn validate_lcr_input(input: &LcrInput, warnings: &mut Vec<String>) -> CorpFinanceResult<()> {
    if input.institution_name.trim().is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "institution_name".into(),
            reason: "Institution name must not be empty.".into(),
        });
    }

    for asset in input
        .hqla
        .level1_assets
        .iter()
        .chain(input.hqla.level2a_assets.iter())
        .chain(input.hqla.level2b_assets.iter())
    {
        if asset.market_value < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "hqla.market_value".into(),
                reason: format!(
                    "HQLA asset '{}' has negative market value: {}",
                    asset.name, asset.market_value
                ),
            });
        }
    }

    for outflow in &input.cash_outflows {
        if outflow.amount < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "cash_outflows.amount".into(),
                reason: format!(
                    "Outflow category '{}' has negative amount: {}",
                    outflow.category, outflow.amount
                ),
            });
        }
    }

    for inflow in &input.cash_inflows {
        if inflow.amount < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "cash_inflows.amount".into(),
                reason: format!(
                    "Inflow category '{}' has negative amount: {}",
                    inflow.category, inflow.amount
                ),
            });
        }
    }

    if input.cash_outflows.is_empty() {
        warnings.push("No cash outflows provided; net outflows will be floored.".to_string());
    }

    Ok(())
}

fn validate_nsfr_input(input: &NsfrInput, warnings: &mut Vec<String>) -> CorpFinanceResult<()> {
    if input.institution_name.trim().is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "institution_name".into(),
            reason: "Institution name must not be empty.".into(),
        });
    }

    for source in &input.available_funding {
        if source.amount < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "available_funding.amount".into(),
                reason: format!(
                    "Funding source '{}' has negative amount: {}",
                    source.category, source.amount
                ),
            });
        }
    }

    for req in &input.required_funding {
        if req.amount < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "required_funding.amount".into(),
                reason: format!(
                    "Funding requirement '{}' has negative amount: {}",
                    req.category, req.amount
                ),
            });
        }
    }

    if input.available_funding.is_empty() {
        warnings.push("No available funding sources provided.".to_string());
    }
    if input.required_funding.is_empty() {
        warnings.push("No required funding items provided.".to_string());
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

    // -- Helper builders ---------------------------------------------------

    fn simple_lcr_input() -> LcrInput {
        LcrInput {
            institution_name: "Test Bank".to_string(),
            hqla: HqlaPortfolio {
                level1_assets: vec![HqlaAsset {
                    name: "Cash".to_string(),
                    market_value: dec!(500),
                    haircut: None,
                }],
                level2a_assets: vec![],
                level2b_assets: vec![],
            },
            cash_outflows: vec![CashOutflow {
                category: OutflowCategory::RetailStableDeposits,
                amount: dec!(2000),
                run_off_rate: None,
            }],
            cash_inflows: vec![CashInflow {
                category: InflowCategory::RetailLoans,
                amount: dec!(100),
                inflow_rate: None,
            }],
        }
    }

    fn multi_level_lcr_input() -> LcrInput {
        LcrInput {
            institution_name: "Multi-Level Bank".to_string(),
            hqla: HqlaPortfolio {
                level1_assets: vec![
                    HqlaAsset {
                        name: "Cash".to_string(),
                        market_value: dec!(300),
                        haircut: None,
                    },
                    HqlaAsset {
                        name: "Govt Bonds".to_string(),
                        market_value: dec!(200),
                        haircut: None,
                    },
                ],
                level2a_assets: vec![HqlaAsset {
                    name: "GSE Bonds".to_string(),
                    market_value: dec!(100),
                    haircut: None,
                }],
                level2b_assets: vec![HqlaAsset {
                    name: "Corporate Bonds".to_string(),
                    market_value: dec!(60),
                    haircut: None,
                }],
            },
            cash_outflows: vec![
                CashOutflow {
                    category: OutflowCategory::RetailStableDeposits,
                    amount: dec!(1000),
                    run_off_rate: None,
                },
                CashOutflow {
                    category: OutflowCategory::UnsecuredWholesaleFinancial,
                    amount: dec!(200),
                    run_off_rate: None,
                },
            ],
            cash_inflows: vec![CashInflow {
                category: InflowCategory::RetailLoans,
                amount: dec!(100),
                inflow_rate: None,
            }],
        }
    }

    fn simple_nsfr_input() -> NsfrInput {
        NsfrInput {
            institution_name: "Test Bank".to_string(),
            available_funding: vec![
                FundingSource {
                    category: AsfCategory::RegulatoryCapital,
                    amount: dec!(500),
                    asf_factor: None,
                },
                FundingSource {
                    category: AsfCategory::StableRetailDeposits,
                    amount: dec!(1000),
                    asf_factor: None,
                },
            ],
            required_funding: vec![
                FundingRequirement {
                    category: RsfCategory::ResidentialMortgages,
                    amount: dec!(800),
                    rsf_factor: None,
                },
                FundingRequirement {
                    category: RsfCategory::Cash,
                    amount: dec!(200),
                    rsf_factor: None,
                },
            ],
        }
    }

    // -- LCR Tests ---------------------------------------------------------

    #[test]
    fn test_lcr_simple_level1_only() {
        let input = simple_lcr_input();
        let result = calculate_lcr(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.total_hqla, dec!(500));
        assert_eq!(out.hqla_breakdown.level1, dec!(500));
        assert_eq!(out.hqla_breakdown.level2a, Decimal::ZERO);
        assert_eq!(out.hqla_breakdown.level2b, Decimal::ZERO);

        // Outflows: 2000 * 0.05 = 100
        assert_eq!(out.total_outflows, dec!(100));
        // Inflows: 100 * 0.50 = 50, cap = 75, uncapped
        assert_eq!(out.total_inflows, dec!(50));
        // Net = 100 - 50 = 50
        assert_eq!(out.net_outflows, dec!(50));
        // LCR = 500 / 50 = 10.0
        assert_eq!(out.lcr_ratio, dec!(10));
        assert!(out.meets_requirement);
        assert_eq!(out.surplus_deficit, dec!(450));
    }

    #[test]
    fn test_lcr_all_three_hqla_levels() {
        let input = multi_level_lcr_input();
        let result = calculate_lcr(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.hqla_breakdown.level1, dec!(500));
        // 100 * 0.85 = 85
        assert_eq!(out.hqla_breakdown.level2a, dec!(85));
        // 60 * 0.50 = 30
        assert_eq!(out.hqla_breakdown.level2b, dec!(30));
        assert_eq!(out.total_hqla, dec!(615));
    }

    #[test]
    fn test_lcr_level2_cap_kicks_in() {
        // L1=100, L2A raw = 200*0.85 = 170, L2B = 0
        // L2_max = 100 * 0.40/0.60 = 66.666...
        let input = LcrInput {
            institution_name: "Cap Test Bank".to_string(),
            hqla: HqlaPortfolio {
                level1_assets: vec![HqlaAsset {
                    name: "Cash".to_string(),
                    market_value: dec!(100),
                    haircut: None,
                }],
                level2a_assets: vec![HqlaAsset {
                    name: "GSE Bonds".to_string(),
                    market_value: dec!(200),
                    haircut: None,
                }],
                level2b_assets: vec![],
            },
            cash_outflows: vec![CashOutflow {
                category: OutflowCategory::RetailStableDeposits,
                amount: dec!(1000),
                run_off_rate: None,
            }],
            cash_inflows: vec![],
        };

        let result = calculate_lcr(&input).unwrap();
        let out = &result.result;

        assert!(out.hqla_breakdown.level2_cap_applied);
        // After cap, L2 should be exactly 40% of total HQLA
        let total =
            out.hqla_breakdown.level1 + out.hqla_breakdown.level2a + out.hqla_breakdown.level2b;
        assert_eq!(out.total_hqla, total);
        let l2_ratio = out.hqla_breakdown.level2a / total;
        // Allow tiny precision tolerance
        assert!(
            l2_ratio <= dec!(0.40) + dec!(0.0001),
            "L2A ratio {} should be <= 40%",
            l2_ratio
        );
    }

    #[test]
    fn test_lcr_level2b_cap_kicks_in() {
        // L1=500, L2A=0, L2B raw = 400*0.50 = 200
        // L2B_max = 500 * 0.15/0.85 = 88.235...
        let input = LcrInput {
            institution_name: "L2B Cap Bank".to_string(),
            hqla: HqlaPortfolio {
                level1_assets: vec![HqlaAsset {
                    name: "Cash".to_string(),
                    market_value: dec!(500),
                    haircut: None,
                }],
                level2a_assets: vec![],
                level2b_assets: vec![HqlaAsset {
                    name: "RMBS".to_string(),
                    market_value: dec!(400),
                    haircut: None,
                }],
            },
            cash_outflows: vec![CashOutflow {
                category: OutflowCategory::RetailStableDeposits,
                amount: dec!(2000),
                run_off_rate: None,
            }],
            cash_inflows: vec![],
        };

        let result = calculate_lcr(&input).unwrap();
        let out = &result.result;

        assert!(out.hqla_breakdown.level2b_cap_applied);
        let l2b_max = dec!(500) * dec!(0.15) / dec!(0.85);
        assert_eq!(out.hqla_breakdown.level2b, l2b_max);
    }

    #[test]
    fn test_lcr_inflow_cap_at_75_percent() {
        let input = LcrInput {
            institution_name: "Inflow Cap Bank".to_string(),
            hqla: HqlaPortfolio {
                level1_assets: vec![HqlaAsset {
                    name: "Cash".to_string(),
                    market_value: dec!(500),
                    haircut: None,
                }],
                level2a_assets: vec![],
                level2b_assets: vec![],
            },
            cash_outflows: vec![CashOutflow {
                category: OutflowCategory::UnsecuredWholesaleFinancial,
                amount: dec!(1000),
                run_off_rate: None,
            }],
            cash_inflows: vec![CashInflow {
                category: InflowCategory::WholesaleFinancial,
                amount: dec!(2000),
                inflow_rate: None,
            }],
        };

        let result = calculate_lcr(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.total_outflows, dec!(1000));
        assert_eq!(out.total_inflows, dec!(750));
        assert_eq!(out.net_outflows, dec!(250));
        assert_eq!(out.lcr_ratio, dec!(2));
    }

    #[test]
    fn test_lcr_meets_requirement() {
        let input = LcrInput {
            institution_name: "Healthy Bank".to_string(),
            hqla: HqlaPortfolio {
                level1_assets: vec![HqlaAsset {
                    name: "Cash".to_string(),
                    market_value: dec!(200),
                    haircut: None,
                }],
                level2a_assets: vec![],
                level2b_assets: vec![],
            },
            cash_outflows: vec![CashOutflow {
                category: OutflowCategory::UnsecuredWholesaleFinancial,
                amount: dec!(100),
                run_off_rate: None,
            }],
            cash_inflows: vec![],
        };

        let result = calculate_lcr(&input).unwrap();
        assert!(result.result.meets_requirement);
        assert!(result.result.lcr_ratio >= dec!(1));
    }

    #[test]
    fn test_lcr_fails_requirement() {
        let input = LcrInput {
            institution_name: "Struggling Bank".to_string(),
            hqla: HqlaPortfolio {
                level1_assets: vec![HqlaAsset {
                    name: "Cash".to_string(),
                    market_value: dec!(50),
                    haircut: None,
                }],
                level2a_assets: vec![],
                level2b_assets: vec![],
            },
            cash_outflows: vec![CashOutflow {
                category: OutflowCategory::UnsecuredWholesaleFinancial,
                amount: dec!(200),
                run_off_rate: None,
            }],
            cash_inflows: vec![],
        };

        let result = calculate_lcr(&input).unwrap();
        let out = &result.result;

        assert!(!out.meets_requirement);
        assert_eq!(out.lcr_ratio, dec!(0.25));
        assert_eq!(out.surplus_deficit, dec!(-150));
    }

    #[test]
    fn test_lcr_outflow_retail_stable_rate() {
        let input = LcrInput {
            institution_name: "Rate Test".to_string(),
            hqla: HqlaPortfolio {
                level1_assets: vec![HqlaAsset {
                    name: "Cash".to_string(),
                    market_value: dec!(100),
                    haircut: None,
                }],
                level2a_assets: vec![],
                level2b_assets: vec![],
            },
            cash_outflows: vec![CashOutflow {
                category: OutflowCategory::RetailStableDeposits,
                amount: dec!(1000),
                run_off_rate: None,
            }],
            cash_inflows: vec![],
        };

        let result = calculate_lcr(&input).unwrap();
        assert_eq!(result.result.total_outflows, dec!(50));
        assert_eq!(result.result.outflow_details[0].rate, dec!(0.05));
    }

    #[test]
    fn test_lcr_outflow_wholesale_financial_rate() {
        let input = LcrInput {
            institution_name: "Rate Test".to_string(),
            hqla: HqlaPortfolio {
                level1_assets: vec![HqlaAsset {
                    name: "Cash".to_string(),
                    market_value: dec!(100),
                    haircut: None,
                }],
                level2a_assets: vec![],
                level2b_assets: vec![],
            },
            cash_outflows: vec![CashOutflow {
                category: OutflowCategory::UnsecuredWholesaleFinancial,
                amount: dec!(500),
                run_off_rate: None,
            }],
            cash_inflows: vec![],
        };

        let result = calculate_lcr(&input).unwrap();
        assert_eq!(result.result.total_outflows, dec!(500));
        assert_eq!(result.result.outflow_details[0].rate, dec!(1.00));
    }

    #[test]
    fn test_lcr_zero_outflows_edge_case() {
        let input = LcrInput {
            institution_name: "No Outflows Bank".to_string(),
            hqla: HqlaPortfolio {
                level1_assets: vec![HqlaAsset {
                    name: "Cash".to_string(),
                    market_value: dec!(500),
                    haircut: None,
                }],
                level2a_assets: vec![],
                level2b_assets: vec![],
            },
            cash_outflows: vec![],
            cash_inflows: vec![],
        };

        let result = calculate_lcr(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.net_outflows, Decimal::ONE);
        assert_eq!(out.lcr_ratio, dec!(500));
        assert!(out.meets_requirement);
    }

    #[test]
    fn test_lcr_custom_override_rates() {
        let input = LcrInput {
            institution_name: "Override Bank".to_string(),
            hqla: HqlaPortfolio {
                level1_assets: vec![HqlaAsset {
                    name: "Cash".to_string(),
                    market_value: dec!(100),
                    haircut: Some(dec!(0.02)),
                }],
                level2a_assets: vec![],
                level2b_assets: vec![],
            },
            cash_outflows: vec![CashOutflow {
                category: OutflowCategory::RetailStableDeposits,
                amount: dec!(1000),
                run_off_rate: Some(dec!(0.08)),
            }],
            cash_inflows: vec![CashInflow {
                category: InflowCategory::RetailLoans,
                amount: dec!(200),
                inflow_rate: Some(dec!(0.60)),
            }],
        };

        let result = calculate_lcr(&input).unwrap();
        let out = &result.result;

        // HQLA: 100 * (1 - 0.02) = 98
        assert_eq!(out.total_hqla, dec!(98));
        // Outflows: 1000 * 0.08 = 80
        assert_eq!(out.total_outflows, dec!(80));
        assert_eq!(out.outflow_details[0].rate, dec!(0.08));
        // Inflows: 200 * 0.60 = 120, cap = 0.75 * 80 = 60 -> capped
        assert_eq!(out.total_inflows, dec!(60));
        assert_eq!(out.inflow_details[0].rate, dec!(0.60));
    }

    #[test]
    fn test_lcr_surplus_deficit_calculation() {
        let input = LcrInput {
            institution_name: "Surplus Bank".to_string(),
            hqla: HqlaPortfolio {
                level1_assets: vec![HqlaAsset {
                    name: "Cash".to_string(),
                    market_value: dec!(300),
                    haircut: None,
                }],
                level2a_assets: vec![],
                level2b_assets: vec![],
            },
            cash_outflows: vec![CashOutflow {
                category: OutflowCategory::UnsecuredWholesaleNonOperational,
                amount: dec!(500),
                run_off_rate: None,
            }],
            cash_inflows: vec![],
        };

        let result = calculate_lcr(&input).unwrap();
        let out = &result.result;

        // 500 * 0.40 = 200
        assert_eq!(out.net_outflows, dec!(200));
        assert_eq!(out.surplus_deficit, dec!(100));
    }

    #[test]
    fn test_lcr_multiple_outflow_categories() {
        let input = LcrInput {
            institution_name: "Mixed Outflows Bank".to_string(),
            hqla: HqlaPortfolio {
                level1_assets: vec![HqlaAsset {
                    name: "Cash".to_string(),
                    market_value: dec!(1000),
                    haircut: None,
                }],
                level2a_assets: vec![],
                level2b_assets: vec![],
            },
            cash_outflows: vec![
                CashOutflow {
                    category: OutflowCategory::RetailStableDeposits,
                    amount: dec!(2000),
                    run_off_rate: None,
                },
                CashOutflow {
                    category: OutflowCategory::RetailLessStable,
                    amount: dec!(1000),
                    run_off_rate: None,
                },
                CashOutflow {
                    category: OutflowCategory::UnsecuredWholesaleOperational,
                    amount: dec!(500),
                    run_off_rate: None,
                },
                CashOutflow {
                    category: OutflowCategory::LiquidityFacilities,
                    amount: dec!(100),
                    run_off_rate: None,
                },
            ],
            cash_inflows: vec![],
        };

        let result = calculate_lcr(&input).unwrap();
        let out = &result.result;

        // 2000*0.05 + 1000*0.10 + 500*0.25 + 100*1.00 = 100+100+125+100 = 425
        assert_eq!(out.total_outflows, dec!(425));
        assert_eq!(out.outflow_details.len(), 4);
    }

    #[test]
    fn test_lcr_negative_market_value_rejected() {
        let input = LcrInput {
            institution_name: "Bad Asset Bank".to_string(),
            hqla: HqlaPortfolio {
                level1_assets: vec![HqlaAsset {
                    name: "Negative Cash".to_string(),
                    market_value: dec!(-100),
                    haircut: None,
                }],
                level2a_assets: vec![],
                level2b_assets: vec![],
            },
            cash_outflows: vec![],
            cash_inflows: vec![],
        };

        let err = calculate_lcr(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "hqla.market_value");
            }
            other => panic!("Expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn test_lcr_empty_institution_name_rejected() {
        let mut input = simple_lcr_input();
        input.institution_name = "".to_string();

        let err = calculate_lcr(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "institution_name");
            }
            other => panic!("Expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn test_lcr_metadata_populated() {
        let input = simple_lcr_input();
        let result = calculate_lcr(&input).unwrap();

        assert!(result.methodology.contains("LCR"));
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
    }

    #[test]
    fn test_lcr_both_caps_applied() {
        // L1=100, L2A raw=300*0.85=255, L2B raw=200*0.50=100
        // L2B_max = (100+255)*0.15/0.85 = 62.647...
        // After L2B cap: L2B = 62.647
        // L2_total = 255+62.647 = 317.647 but L2_max = 100*0.40/0.60 = 66.666
        let input = LcrInput {
            institution_name: "Double Cap Bank".to_string(),
            hqla: HqlaPortfolio {
                level1_assets: vec![HqlaAsset {
                    name: "Cash".to_string(),
                    market_value: dec!(100),
                    haircut: None,
                }],
                level2a_assets: vec![HqlaAsset {
                    name: "GSE Bonds".to_string(),
                    market_value: dec!(300),
                    haircut: None,
                }],
                level2b_assets: vec![HqlaAsset {
                    name: "RMBS".to_string(),
                    market_value: dec!(200),
                    haircut: None,
                }],
            },
            cash_outflows: vec![CashOutflow {
                category: OutflowCategory::RetailStableDeposits,
                amount: dec!(5000),
                run_off_rate: None,
            }],
            cash_inflows: vec![],
        };

        let result = calculate_lcr(&input).unwrap();
        let out = &result.result;

        assert!(out.hqla_breakdown.level2b_cap_applied);
        assert!(out.hqla_breakdown.level2_cap_applied);

        let total =
            out.hqla_breakdown.level1 + out.hqla_breakdown.level2a + out.hqla_breakdown.level2b;
        let l2_share = (out.hqla_breakdown.level2a + out.hqla_breakdown.level2b) / total;
        assert!(
            l2_share <= dec!(0.40) + dec!(0.0001),
            "L2 share {} should be <= 40%",
            l2_share
        );
    }

    #[test]
    fn test_lcr_secured_funding_rates() {
        let input = LcrInput {
            institution_name: "Secured Bank".to_string(),
            hqla: HqlaPortfolio {
                level1_assets: vec![HqlaAsset {
                    name: "Cash".to_string(),
                    market_value: dec!(1000),
                    haircut: None,
                }],
                level2a_assets: vec![],
                level2b_assets: vec![],
            },
            cash_outflows: vec![
                CashOutflow {
                    category: OutflowCategory::SecuredFundingCentral,
                    amount: dec!(500),
                    run_off_rate: None,
                },
                CashOutflow {
                    category: OutflowCategory::SecuredFundingLevel1,
                    amount: dec!(300),
                    run_off_rate: None,
                },
                CashOutflow {
                    category: OutflowCategory::SecuredFundingLevel2A,
                    amount: dec!(200),
                    run_off_rate: None,
                },
                CashOutflow {
                    category: OutflowCategory::SecuredFundingOther,
                    amount: dec!(100),
                    run_off_rate: None,
                },
            ],
            cash_inflows: vec![],
        };

        let result = calculate_lcr(&input).unwrap();
        let out = &result.result;

        // 500*0 + 300*0 + 200*0.15 + 100*1.00 = 0+0+30+100 = 130
        assert_eq!(out.total_outflows, dec!(130));
    }

    #[test]
    fn test_lcr_hqla_custom_haircut_per_asset() {
        let input = LcrInput {
            institution_name: "Custom Haircut Bank".to_string(),
            hqla: HqlaPortfolio {
                level1_assets: vec![HqlaAsset {
                    name: "Govt Bonds".to_string(),
                    market_value: dec!(1000),
                    haircut: Some(dec!(0.05)),
                }],
                level2a_assets: vec![HqlaAsset {
                    name: "Agency Bonds".to_string(),
                    market_value: dec!(500),
                    haircut: Some(dec!(0.20)),
                }],
                level2b_assets: vec![
                    HqlaAsset {
                        name: "RMBS".to_string(),
                        market_value: dec!(200),
                        haircut: Some(dec!(0.25)),
                    },
                    HqlaAsset {
                        name: "Corp Bonds".to_string(),
                        market_value: dec!(100),
                        haircut: Some(dec!(0.50)),
                    },
                ],
            },
            cash_outflows: vec![CashOutflow {
                category: OutflowCategory::RetailStableDeposits,
                amount: dec!(10000),
                run_off_rate: None,
            }],
            cash_inflows: vec![],
        };

        let result = calculate_lcr(&input).unwrap();
        let out = &result.result;

        // L1: 1000*(1-0.05) = 950
        assert_eq!(out.hqla_breakdown.level1, dec!(950));
        // L2A: 500*(1-0.20) = 400
        assert_eq!(out.hqla_breakdown.level2a, dec!(400));
        // L2B: 200*(1-0.25) + 100*(1-0.50) = 150+50 = 200
        // L2B cap: (950+400)*0.15/0.85 = 238.235... -> 200 < 238.24 => no cap
        assert!(!out.hqla_breakdown.level2b_cap_applied);
        assert_eq!(out.hqla_breakdown.level2b, dec!(200));
    }

    #[test]
    fn test_lcr_inflow_categories() {
        let input = LcrInput {
            institution_name: "Inflow Test Bank".to_string(),
            hqla: HqlaPortfolio {
                level1_assets: vec![HqlaAsset {
                    name: "Cash".to_string(),
                    market_value: dec!(1000),
                    haircut: None,
                }],
                level2a_assets: vec![],
                level2b_assets: vec![],
            },
            cash_outflows: vec![CashOutflow {
                category: OutflowCategory::UnsecuredWholesaleFinancial,
                amount: dec!(10000),
                run_off_rate: None,
            }],
            cash_inflows: vec![
                CashInflow {
                    category: InflowCategory::SecuredLendingLevel1,
                    amount: dec!(500),
                    inflow_rate: None,
                },
                CashInflow {
                    category: InflowCategory::SecuredLendingLevel2A,
                    amount: dec!(300),
                    inflow_rate: None,
                },
                CashInflow {
                    category: InflowCategory::SecuredLendingOther,
                    amount: dec!(200),
                    inflow_rate: None,
                },
                CashInflow {
                    category: InflowCategory::WholesaleFinancial,
                    amount: dec!(100),
                    inflow_rate: None,
                },
            ],
        };

        let result = calculate_lcr(&input).unwrap();
        let out = &result.result;

        // 500*0 + 300*0.15 + 200*1.0 + 100*1.0 = 0+45+200+100 = 345
        assert_eq!(out.total_inflows, dec!(345));
        assert_eq!(out.inflow_details[0].rate, dec!(0.00));
        assert_eq!(out.inflow_details[0].weighted_amount, Decimal::ZERO);
    }

    // -- NSFR Tests --------------------------------------------------------

    #[test]
    fn test_nsfr_simple_case() {
        let input = simple_nsfr_input();
        let result = calculate_nsfr(&input).unwrap();
        let out = &result.result;

        // ASF: 500*1.00 + 1000*0.95 = 1450
        assert_eq!(out.available_stable_funding, dec!(1450));
        // RSF: 800*0.65 + 200*0.00 = 520
        assert_eq!(out.required_stable_funding, dec!(520));
        let expected_ratio = dec!(1450) / dec!(520);
        assert_eq!(out.nsfr_ratio, expected_ratio);
        assert!(out.meets_requirement);
    }

    #[test]
    fn test_nsfr_mixed_funding_sources() {
        let input = NsfrInput {
            institution_name: "Mixed Bank".to_string(),
            available_funding: vec![
                FundingSource {
                    category: AsfCategory::RegulatoryCapital,
                    amount: dec!(200),
                    asf_factor: None,
                },
                FundingSource {
                    category: AsfCategory::LessStableRetailDeposits,
                    amount: dec!(500),
                    asf_factor: None,
                },
                FundingSource {
                    category: AsfCategory::WholesaleFunding6mTo1Y,
                    amount: dec!(300),
                    asf_factor: None,
                },
                FundingSource {
                    category: AsfCategory::WholesaleFundingLt6M,
                    amount: dec!(400),
                    asf_factor: None,
                },
            ],
            required_funding: vec![FundingRequirement {
                category: RsfCategory::RetailLoans,
                amount: dec!(1000),
                rsf_factor: None,
            }],
        };

        let result = calculate_nsfr(&input).unwrap();
        let out = &result.result;

        // ASF: 200*1.00 + 500*0.90 + 300*0.50 + 400*0.00 = 800
        assert_eq!(out.available_stable_funding, dec!(800));
        // RSF: 1000*0.85 = 850
        assert_eq!(out.required_stable_funding, dec!(850));
        assert!(!out.meets_requirement);
    }

    #[test]
    fn test_nsfr_meets_requirement() {
        let input = simple_nsfr_input();
        let result = calculate_nsfr(&input).unwrap();

        assert!(result.result.meets_requirement);
        assert!(result.result.nsfr_ratio >= dec!(1));
    }

    #[test]
    fn test_nsfr_fails_requirement() {
        let input = NsfrInput {
            institution_name: "Weak Bank".to_string(),
            available_funding: vec![FundingSource {
                category: AsfCategory::WholesaleFundingLt6M,
                amount: dec!(1000),
                asf_factor: None,
            }],
            required_funding: vec![FundingRequirement {
                category: RsfCategory::FixedAssets,
                amount: dec!(500),
                rsf_factor: None,
            }],
        };

        let result = calculate_nsfr(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.available_stable_funding, Decimal::ZERO);
        assert_eq!(out.required_stable_funding, dec!(500));
        assert!(!out.meets_requirement);
        assert_eq!(out.nsfr_ratio, Decimal::ZERO);
        assert_eq!(out.surplus_deficit, dec!(-500));
    }

    #[test]
    fn test_nsfr_asf_factor_application() {
        let input = NsfrInput {
            institution_name: "Factor Test".to_string(),
            available_funding: vec![
                FundingSource {
                    category: AsfCategory::RegulatoryCapital,
                    amount: dec!(100),
                    asf_factor: None,
                },
                FundingSource {
                    category: AsfCategory::StableRetailDeposits,
                    amount: dec!(200),
                    asf_factor: None,
                },
            ],
            required_funding: vec![FundingRequirement {
                category: RsfCategory::Cash,
                amount: dec!(100),
                rsf_factor: None,
            }],
        };

        let result = calculate_nsfr(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.asf_details.len(), 2);
        assert_eq!(out.asf_details[0].factor, dec!(1.00));
        assert_eq!(out.asf_details[0].weighted_amount, dec!(100));
        assert_eq!(out.asf_details[1].factor, dec!(0.95));
        assert_eq!(out.asf_details[1].weighted_amount, dec!(190));
    }

    #[test]
    fn test_nsfr_rsf_factor_application() {
        let input = NsfrInput {
            institution_name: "RSF Test".to_string(),
            available_funding: vec![FundingSource {
                category: AsfCategory::RegulatoryCapital,
                amount: dec!(10000),
                asf_factor: None,
            }],
            required_funding: vec![
                FundingRequirement {
                    category: RsfCategory::Cash,
                    amount: dec!(100),
                    rsf_factor: None,
                },
                FundingRequirement {
                    category: RsfCategory::Level1Hqla,
                    amount: dec!(200),
                    rsf_factor: None,
                },
                FundingRequirement {
                    category: RsfCategory::ResidentialMortgages,
                    amount: dec!(500),
                    rsf_factor: None,
                },
                FundingRequirement {
                    category: RsfCategory::NonPerformingLoans,
                    amount: dec!(50),
                    rsf_factor: None,
                },
            ],
        };

        let result = calculate_nsfr(&input).unwrap();
        let out = &result.result;

        // RSF: 0 + 10 + 325 + 50 = 385
        assert_eq!(out.required_stable_funding, dec!(385));
        assert_eq!(out.rsf_details[0].weighted_amount, Decimal::ZERO);
        assert_eq!(out.rsf_details[1].weighted_amount, dec!(10));
        assert_eq!(out.rsf_details[2].weighted_amount, dec!(325));
        assert_eq!(out.rsf_details[3].weighted_amount, dec!(50));
    }

    #[test]
    fn test_nsfr_zero_rsf_edge_case() {
        let input = NsfrInput {
            institution_name: "All Cash Bank".to_string(),
            available_funding: vec![FundingSource {
                category: AsfCategory::RegulatoryCapital,
                amount: dec!(1000),
                asf_factor: None,
            }],
            required_funding: vec![FundingRequirement {
                category: RsfCategory::Cash,
                amount: dec!(500),
                rsf_factor: None,
            }],
        };

        let result = calculate_nsfr(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.required_stable_funding, Decimal::ZERO);
        assert_eq!(out.nsfr_ratio, dec!(1000));
        assert!(out.meets_requirement);
    }

    #[test]
    fn test_nsfr_custom_override_factors() {
        let input = NsfrInput {
            institution_name: "Override Bank".to_string(),
            available_funding: vec![FundingSource {
                category: AsfCategory::StableRetailDeposits,
                amount: dec!(1000),
                asf_factor: Some(dec!(0.80)),
            }],
            required_funding: vec![FundingRequirement {
                category: RsfCategory::ResidentialMortgages,
                amount: dec!(500),
                rsf_factor: Some(dec!(0.75)),
            }],
        };

        let result = calculate_nsfr(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.available_stable_funding, dec!(800));
        assert_eq!(out.asf_details[0].factor, dec!(0.80));
        assert_eq!(out.required_stable_funding, dec!(375));
        assert_eq!(out.rsf_details[0].factor, dec!(0.75));
    }

    #[test]
    fn test_nsfr_surplus_deficit() {
        let input = simple_nsfr_input();
        let result = calculate_nsfr(&input).unwrap();
        let out = &result.result;

        assert_eq!(
            out.surplus_deficit,
            out.available_stable_funding - out.required_stable_funding
        );
        assert!(out.surplus_deficit > Decimal::ZERO);
    }

    #[test]
    fn test_nsfr_empty_institution_name_rejected() {
        let mut input = simple_nsfr_input();
        input.institution_name = "  ".to_string();

        let err = calculate_nsfr(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "institution_name");
            }
            other => panic!("Expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn test_nsfr_metadata_populated() {
        let input = simple_nsfr_input();
        let result = calculate_nsfr(&input).unwrap();

        assert!(result.methodology.contains("NSFR"));
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
    }

    #[test]
    fn test_nsfr_all_rsf_categories() {
        let input = NsfrInput {
            institution_name: "Full RSF Bank".to_string(),
            available_funding: vec![FundingSource {
                category: AsfCategory::RegulatoryCapital,
                amount: dec!(10000),
                asf_factor: None,
            }],
            required_funding: vec![
                FundingRequirement {
                    category: RsfCategory::Cash,
                    amount: dec!(100),
                    rsf_factor: None,
                },
                FundingRequirement {
                    category: RsfCategory::CentralBankReserves,
                    amount: dec!(100),
                    rsf_factor: None,
                },
                FundingRequirement {
                    category: RsfCategory::Level1Hqla,
                    amount: dec!(100),
                    rsf_factor: None,
                },
                FundingRequirement {
                    category: RsfCategory::Level2aHqla,
                    amount: dec!(100),
                    rsf_factor: None,
                },
                FundingRequirement {
                    category: RsfCategory::Level2bHqla,
                    amount: dec!(100),
                    rsf_factor: None,
                },
                FundingRequirement {
                    category: RsfCategory::LoansToFILt6M,
                    amount: dec!(100),
                    rsf_factor: None,
                },
                FundingRequirement {
                    category: RsfCategory::LoansToFI6mTo1Y,
                    amount: dec!(100),
                    rsf_factor: None,
                },
                FundingRequirement {
                    category: RsfCategory::ResidentialMortgages,
                    amount: dec!(100),
                    rsf_factor: None,
                },
                FundingRequirement {
                    category: RsfCategory::RetailLoans,
                    amount: dec!(100),
                    rsf_factor: None,
                },
                FundingRequirement {
                    category: RsfCategory::CorporateLoansGt1Y,
                    amount: dec!(100),
                    rsf_factor: None,
                },
                FundingRequirement {
                    category: RsfCategory::NonPerformingLoans,
                    amount: dec!(100),
                    rsf_factor: None,
                },
                FundingRequirement {
                    category: RsfCategory::FixedAssets,
                    amount: dec!(100),
                    rsf_factor: None,
                },
                FundingRequirement {
                    category: RsfCategory::Other,
                    amount: dec!(100),
                    rsf_factor: None,
                },
            ],
        };

        let result = calculate_nsfr(&input).unwrap();
        let out = &result.result;

        // RSF: 0+0+5+15+50+10+50+65+85+85+100+100+100 = 665
        assert_eq!(out.required_stable_funding, dec!(665));
        assert_eq!(out.rsf_details.len(), 13);
    }
}
