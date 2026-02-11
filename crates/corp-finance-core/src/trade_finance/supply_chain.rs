//! Supply chain finance -- reverse factoring, dynamic discounting,
//! forfaiting, and export credit analysis.
//!
//! All calculations use a 360-day year convention (standard in trade finance).
//! Monetary values and rates are in `rust_decimal::Decimal` throughout.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{with_metadata, ComputationOutput, Money, Rate};
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Trade finance day-count basis.
const DAYS_IN_YEAR: Decimal = dec!(360);
/// Basis points divisor.
const BPS: Decimal = dec!(10000);

// ---------------------------------------------------------------------------
// Input types
// ---------------------------------------------------------------------------

/// The type of supply chain finance analysis to perform.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ScfType {
    ReverseFactoring,
    DynamicDiscounting,
    Forfaiting,
    ExportCredit,
}

/// Repayment structure for export credit facilities.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RepaymentType {
    EqualPrincipal,
    Annuity,
}

/// Full input for a supply chain finance computation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupplyChainFinanceInput {
    /// Which SCF analysis to run.
    pub analysis_type: ScfType,

    // -- Reverse Factoring fields --
    /// Invoice face value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoice_amount: Option<Money>,
    /// Original buyer payment terms in days (e.g. 90).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_terms_days: Option<u32>,
    /// Days after invoice date when supplier receives early payment (e.g. 10).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub early_payment_days: Option<u32>,
    /// Supplier's own cost of financing (annualized, decimal).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supplier_cost_of_funds: Option<Rate>,
    /// Buyer credit spread over base rate in basis points.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buyer_credit_spread_bps: Option<Decimal>,
    /// Base interest rate (e.g. SOFR), decimal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_rate: Option<Rate>,
    /// Platform / intermediary fee in basis points (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform_fee_bps: Option<Decimal>,

    // -- Dynamic Discounting fields --
    /// Standard payment term in days.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub standard_payment_days: Option<u32>,
    /// Actual early payment day (measured from invoice date).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub early_payment_day: Option<u32>,
    /// Discount rate per day early, in basis points.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discount_rate_per_day_bps: Option<Decimal>,
    /// Buyer's opportunity cost / WACC (decimal).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buyer_opportunity_cost: Option<Rate>,

    // -- Forfaiting fields --
    /// Face value of the trade receivable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receivable_amount: Option<Money>,
    /// Maturity in days from present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maturity_days: Option<u32>,
    /// Forfaiter's all-in discount rate (annualized, decimal).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discount_rate: Option<Rate>,
    /// Commitment fee in basis points (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commitment_fee_bps: Option<Decimal>,
    /// Whether the bill is avalised (bank-guaranteed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_avalised: Option<bool>,
    /// Credit rating of the avalising bank (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avalising_bank_rating: Option<String>,
    /// Grace days added to maturity (default 3).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grace_days: Option<u32>,

    // -- Export Credit fields --
    /// Total contract value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contract_value: Option<Money>,
    /// Percentage covered by ECA (decimal, e.g. 0.85 = 85%).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eca_covered_pct: Option<Rate>,
    /// OECD CIRR rate for the currency (decimal).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cirr_rate: Option<Rate>,
    /// Commercial market rate for uncovered portion (decimal).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commercial_rate: Option<Rate>,
    /// ECA exposure / premium fee (decimal, e.g. 0.02 = 2%).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eca_premium_pct: Option<Rate>,
    /// Tenor in whole years.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenor_years: Option<u32>,
    /// Repayment structure: EqualPrincipal or Annuity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repayment_type: Option<RepaymentType>,
    /// Down payment percentage (decimal, OECD consensus typically 0.15).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub down_payment_pct: Option<Rate>,
}

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// Complete output from a supply chain finance computation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupplyChainFinanceOutput {
    /// Which analysis was performed.
    pub analysis_type: String,

    // -- Reverse Factoring outputs --
    /// Discount deducted for early payment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discount_amount: Option<Money>,
    /// Amount the supplier actually receives.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supplier_proceeds: Option<Money>,
    /// Annualized effective cost to the supplier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supplier_effective_rate: Option<Rate>,
    /// Savings vs. supplier's own financing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supplier_savings: Option<Money>,
    /// Days of payable outstanding maintained by buyer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buyer_dpo_extension: Option<u32>,
    /// Annualized yield for the funder.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub funder_return: Option<Rate>,

    // -- Dynamic Discounting outputs --
    /// Total discount percentage offered.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discount_pct: Option<Rate>,
    // discount_amount is shared with reverse factoring
    // supplier_proceeds is shared with reverse factoring
    /// Buyer's annualized return on the early payment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buyer_annualized_return: Option<Rate>,
    /// NPV of the discount vs. buyer's opportunity cost.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buyer_npv: Option<Money>,

    // -- Forfaiting outputs --
    /// Discount on the receivable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discount: Option<Money>,
    /// Gross proceeds (receivable minus discount).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proceeds: Option<Money>,
    /// Commitment fee amount (if applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commitment_fee: Option<Money>,
    /// Net proceeds after discount and commitment fee.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub net_proceeds: Option<Money>,
    /// All-in yield to the forfaiter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective_yield: Option<Rate>,
    /// Effective financing cost to the exporter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exporter_effective_cost: Option<Rate>,

    // -- Export Credit outputs --
    /// Amount covered by the ECA.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eca_covered_amount: Option<Money>,
    /// Amount financed on commercial terms.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commercial_amount: Option<Money>,
    /// Total ECA premium.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eca_premium: Option<Money>,
    /// Weighted average blended rate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blended_rate: Option<Rate>,
    /// Total interest cost over the tenor.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_interest_cost: Option<Money>,
    /// Down payment amount.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub down_payment: Option<Money>,
    /// Financed amount (contract value minus down payment).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub financed_amount: Option<Money>,
    /// First-year (or level) annual debt service.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annual_debt_service: Option<Money>,

    /// Warnings and informational messages.
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Analyse a supply chain finance structure.
///
/// Dispatches to the appropriate sub-routine based on `input.analysis_type`.
pub fn analyze_supply_chain_finance(
    input: &SupplyChainFinanceInput,
) -> CorpFinanceResult<ComputationOutput<SupplyChainFinanceOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    let output = match input.analysis_type {
        ScfType::ReverseFactoring => analyze_reverse_factoring(input, &mut warnings)?,
        ScfType::DynamicDiscounting => analyze_dynamic_discounting(input, &mut warnings)?,
        ScfType::Forfaiting => analyze_forfaiting(input, &mut warnings)?,
        ScfType::ExportCredit => analyze_export_credit(input, &mut warnings)?,
    };

    let elapsed = start.elapsed().as_micros() as u64;

    let methodology = match input.analysis_type {
        ScfType::ReverseFactoring => {
            "Reverse Factoring -- buyer-led payables finance, 360-day basis"
        }
        ScfType::DynamicDiscounting => {
            "Dynamic Discounting -- sliding scale early payment, 360-day basis"
        }
        ScfType::Forfaiting => {
            "Forfaiting -- without-recourse discount of trade receivables, 360-day basis"
        }
        ScfType::ExportCredit => {
            "Export Credit -- ECA-backed blended financing with CIRR, 360-day basis"
        }
    };

    Ok(with_metadata(
        methodology,
        &serde_json::json!({
            "day_count": "360",
            "analysis_type": format!("{:?}", input.analysis_type),
        }),
        output.warnings.clone(),
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Reverse Factoring
// ---------------------------------------------------------------------------

fn analyze_reverse_factoring(
    input: &SupplyChainFinanceInput,
    warnings: &mut Vec<String>,
) -> CorpFinanceResult<SupplyChainFinanceOutput> {
    // --- Extract required fields ---
    let invoice = required_field(input.invoice_amount, "invoice_amount")?;
    let payment_terms = required_field(input.payment_terms_days, "payment_terms_days")?;
    let early_days = required_field(input.early_payment_days, "early_payment_days")?;
    let supplier_cof = required_field(input.supplier_cost_of_funds, "supplier_cost_of_funds")?;
    let buyer_spread_bps =
        required_field(input.buyer_credit_spread_bps, "buyer_credit_spread_bps")?;
    let base_rate = required_field(input.base_rate, "base_rate")?;

    // --- Validate ---
    if invoice <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "invoice_amount".into(),
            reason: "Invoice amount must be positive".into(),
        });
    }
    if early_days >= payment_terms {
        return Err(CorpFinanceError::InvalidInput {
            field: "early_payment_days".into(),
            reason: "Early payment days must be less than payment terms days".into(),
        });
    }

    let days_accelerated = payment_terms - early_days;
    let days_dec = Decimal::from(days_accelerated);

    // Discount rate for the accelerated period
    let program_rate = base_rate + buyer_spread_bps / BPS;
    let discount_rate = program_rate * days_dec / DAYS_IN_YEAR;
    let discount_amount = invoice * discount_rate;

    // Platform fee
    let platform_fee_amount = match input.platform_fee_bps {
        Some(fee_bps) if fee_bps > Decimal::ZERO => {
            invoice * fee_bps / BPS * days_dec / DAYS_IN_YEAR
        }
        _ => Decimal::ZERO,
    };

    let supplier_proceeds = invoice - discount_amount - platform_fee_amount;

    // Supplier effective annualized rate
    let supplier_effective_rate = if supplier_proceeds > Decimal::ZERO {
        ((invoice - supplier_proceeds) / supplier_proceeds) * (DAYS_IN_YEAR / days_dec)
    } else {
        warnings.push("Supplier proceeds are zero or negative".into());
        Decimal::ZERO
    };

    // Supplier's alternative cost (financing the receivable themselves)
    let supplier_alt_cost = invoice * supplier_cof * days_dec / DAYS_IN_YEAR;
    let supplier_savings = supplier_alt_cost - discount_amount - platform_fee_amount;

    if supplier_savings < Decimal::ZERO {
        warnings.push(
            "Supplier savings negative: program cost exceeds supplier's own financing cost".into(),
        );
    }

    // Funder return (annualized)
    let funder_return = if supplier_proceeds > Decimal::ZERO {
        (discount_amount / supplier_proceeds) * (DAYS_IN_YEAR / days_dec)
    } else {
        Decimal::ZERO
    };

    Ok(SupplyChainFinanceOutput {
        analysis_type: "ReverseFactoring".to_string(),
        discount_amount: Some(discount_amount),
        supplier_proceeds: Some(supplier_proceeds),
        supplier_effective_rate: Some(supplier_effective_rate),
        supplier_savings: Some(supplier_savings),
        buyer_dpo_extension: Some(payment_terms),
        funder_return: Some(funder_return),
        // Not applicable for this analysis type
        discount_pct: None,
        buyer_annualized_return: None,
        buyer_npv: None,
        discount: None,
        proceeds: None,
        commitment_fee: None,
        net_proceeds: None,
        effective_yield: None,
        exporter_effective_cost: None,
        eca_covered_amount: None,
        commercial_amount: None,
        eca_premium: None,
        blended_rate: None,
        total_interest_cost: None,
        down_payment: None,
        financed_amount: None,
        annual_debt_service: None,
        warnings: warnings.clone(),
    })
}

// ---------------------------------------------------------------------------
// Dynamic Discounting
// ---------------------------------------------------------------------------

fn analyze_dynamic_discounting(
    input: &SupplyChainFinanceInput,
    warnings: &mut Vec<String>,
) -> CorpFinanceResult<SupplyChainFinanceOutput> {
    let invoice = required_field(input.invoice_amount, "invoice_amount")?;
    let std_days = required_field(input.standard_payment_days, "standard_payment_days")?;
    let early_day = required_field(input.early_payment_day, "early_payment_day")?;
    let disc_per_day_bps =
        required_field(input.discount_rate_per_day_bps, "discount_rate_per_day_bps")?;
    let buyer_opp_cost = required_field(input.buyer_opportunity_cost, "buyer_opportunity_cost")?;

    if invoice <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "invoice_amount".into(),
            reason: "Invoice amount must be positive".into(),
        });
    }
    if early_day >= std_days {
        return Err(CorpFinanceError::InvalidInput {
            field: "early_payment_day".into(),
            reason: "Early payment day must be less than standard payment days".into(),
        });
    }

    let days_early = std_days - early_day;
    let days_early_dec = Decimal::from(days_early);

    // Discount percentage = days_early * discount_rate_per_day_bps / 10000
    let discount_pct = days_early_dec * disc_per_day_bps / BPS;
    let discount_amount = invoice * discount_pct;
    let supplier_proceeds = invoice - discount_amount;

    // Buyer annualized return = (discount_pct / (1 - discount_pct)) * (360 / days_early)
    let buyer_annualized_return = if discount_pct < Decimal::ONE {
        (discount_pct / (Decimal::ONE - discount_pct)) * (DAYS_IN_YEAR / days_early_dec)
    } else {
        warnings.push("Discount percentage >= 100%, annualized return undefined".into());
        Decimal::ZERO
    };

    // Buyer NPV = discount_amount - (invoice * buyer_opportunity_cost * days_early / 360)
    let opp_cost_amount = invoice * buyer_opp_cost * days_early_dec / DAYS_IN_YEAR;
    let buyer_npv = discount_amount - opp_cost_amount;

    if buyer_npv < Decimal::ZERO {
        warnings.push("Buyer NPV is negative: opportunity cost exceeds discount benefit".into());
    }

    Ok(SupplyChainFinanceOutput {
        analysis_type: "DynamicDiscounting".to_string(),
        discount_pct: Some(discount_pct),
        discount_amount: Some(discount_amount),
        supplier_proceeds: Some(supplier_proceeds),
        buyer_annualized_return: Some(buyer_annualized_return),
        buyer_npv: Some(buyer_npv),
        // Not applicable
        supplier_effective_rate: None,
        supplier_savings: None,
        buyer_dpo_extension: None,
        funder_return: None,
        discount: None,
        proceeds: None,
        commitment_fee: None,
        net_proceeds: None,
        effective_yield: None,
        exporter_effective_cost: None,
        eca_covered_amount: None,
        commercial_amount: None,
        eca_premium: None,
        blended_rate: None,
        total_interest_cost: None,
        down_payment: None,
        financed_amount: None,
        annual_debt_service: None,
        warnings: warnings.clone(),
    })
}

// ---------------------------------------------------------------------------
// Forfaiting
// ---------------------------------------------------------------------------

fn analyze_forfaiting(
    input: &SupplyChainFinanceInput,
    warnings: &mut Vec<String>,
) -> CorpFinanceResult<SupplyChainFinanceOutput> {
    let receivable = required_field(input.receivable_amount, "receivable_amount")?;
    let maturity_days = required_field(input.maturity_days, "maturity_days")?;
    let discount_rate = required_field(input.discount_rate, "discount_rate")?;

    if receivable <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "receivable_amount".into(),
            reason: "Receivable amount must be positive".into(),
        });
    }
    if maturity_days == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "maturity_days".into(),
            reason: "Maturity days must be positive".into(),
        });
    }

    let grace = input.grace_days.unwrap_or(3);
    let total_days = maturity_days + grace;
    let total_days_dec = Decimal::from(total_days);

    let is_avalised = input.is_avalised.unwrap_or(false);
    if is_avalised {
        if let Some(ref rating) = input.avalising_bank_rating {
            warnings.push(format!("Avalised bill, bank rating: {}", rating));
        } else {
            warnings.push("Avalised bill, bank rating not provided".into());
        }
    } else {
        warnings.push("Bill is not avalised -- higher credit risk to forfaiter".into());
    }

    // Discount = receivable * discount_rate * total_days / 360
    let discount = receivable * discount_rate * total_days_dec / DAYS_IN_YEAR;
    let proceeds = receivable - discount;

    // Commitment fee
    let commitment_fee = input
        .commitment_fee_bps
        .map(|fee_bps| receivable * fee_bps / BPS * total_days_dec / DAYS_IN_YEAR);
    let commitment_fee_amount = commitment_fee.unwrap_or(Decimal::ZERO);

    let net_proceeds = receivable - discount - commitment_fee_amount;

    if net_proceeds <= Decimal::ZERO {
        return Err(CorpFinanceError::FinancialImpossibility(
            "Net proceeds are zero or negative -- discount exceeds receivable value".into(),
        ));
    }

    // Effective yield = ((receivable / net_proceeds) - 1) * (360 / total_days)
    let effective_yield =
        (receivable / net_proceeds - Decimal::ONE) * (DAYS_IN_YEAR / total_days_dec);

    // Exporter effective cost = same as effective yield (cost to the exporter)
    let exporter_effective_cost = effective_yield;

    Ok(SupplyChainFinanceOutput {
        analysis_type: "Forfaiting".to_string(),
        discount: Some(discount),
        proceeds: Some(proceeds),
        commitment_fee,
        net_proceeds: Some(net_proceeds),
        effective_yield: Some(effective_yield),
        exporter_effective_cost: Some(exporter_effective_cost),
        // Not applicable
        discount_amount: None,
        supplier_proceeds: None,
        supplier_effective_rate: None,
        supplier_savings: None,
        buyer_dpo_extension: None,
        funder_return: None,
        discount_pct: None,
        buyer_annualized_return: None,
        buyer_npv: None,
        eca_covered_amount: None,
        commercial_amount: None,
        eca_premium: None,
        blended_rate: None,
        total_interest_cost: None,
        down_payment: None,
        financed_amount: None,
        annual_debt_service: None,
        warnings: warnings.clone(),
    })
}

// ---------------------------------------------------------------------------
// Export Credit
// ---------------------------------------------------------------------------

fn analyze_export_credit(
    input: &SupplyChainFinanceInput,
    warnings: &mut Vec<String>,
) -> CorpFinanceResult<SupplyChainFinanceOutput> {
    let contract_value = required_field(input.contract_value, "contract_value")?;
    let eca_pct = required_field(input.eca_covered_pct, "eca_covered_pct")?;
    let cirr = required_field(input.cirr_rate, "cirr_rate")?;
    let comm_rate = required_field(input.commercial_rate, "commercial_rate")?;
    let eca_premium_pct = required_field(input.eca_premium_pct, "eca_premium_pct")?;
    let tenor = required_field(input.tenor_years, "tenor_years")?;
    let repay_type = required_field(input.repayment_type.clone(), "repayment_type")?;
    let dp_pct = required_field(input.down_payment_pct, "down_payment_pct")?;

    if contract_value <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "contract_value".into(),
            reason: "Contract value must be positive".into(),
        });
    }
    if tenor == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "tenor_years".into(),
            reason: "Tenor must be at least 1 year".into(),
        });
    }
    if eca_pct < Decimal::ZERO || eca_pct > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "eca_covered_pct".into(),
            reason: "ECA covered percentage must be between 0 and 1".into(),
        });
    }
    if dp_pct < Decimal::ZERO || dp_pct >= Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "down_payment_pct".into(),
            reason: "Down payment percentage must be >= 0 and < 1".into(),
        });
    }

    let down_payment = contract_value * dp_pct;
    let financed = contract_value - down_payment;
    let eca_covered = financed * eca_pct;
    let commercial = financed - eca_covered;

    // Blended rate = weighted average of CIRR (on ECA portion) and commercial rate
    let blended_rate = if financed > Decimal::ZERO {
        (eca_covered * cirr + commercial * comm_rate) / financed
    } else {
        warnings.push("Financed amount is zero".into());
        Decimal::ZERO
    };

    // ECA premium (one-time, on ECA-covered amount)
    let eca_premium = eca_covered * eca_premium_pct;

    // Annual debt service & total interest cost depend on repayment type
    let tenor_dec = Decimal::from(tenor);
    let (annual_ds, total_interest) = match repay_type {
        RepaymentType::EqualPrincipal => {
            compute_equal_principal_debt_service(financed, blended_rate, tenor, warnings)
        }
        RepaymentType::Annuity => {
            compute_annuity_debt_service(financed, blended_rate, tenor, warnings)
        }
    };

    // OECD consensus: typical down payment is 15%
    if dp_pct < dec!(0.15) {
        warnings.push(format!(
            "Down payment {:.1}% is below OECD Consensus minimum of 15%",
            dp_pct * dec!(100)
        ));
    }

    if tenor > 10 {
        warnings.push(format!(
            "Tenor of {} years exceeds typical ECA maximum of 10 years",
            tenor
        ));
    }

    // Verify total interest is sensible
    let _ = tenor_dec; // used for warnings above

    Ok(SupplyChainFinanceOutput {
        analysis_type: "ExportCredit".to_string(),
        eca_covered_amount: Some(eca_covered),
        commercial_amount: Some(commercial),
        eca_premium: Some(eca_premium),
        blended_rate: Some(blended_rate),
        total_interest_cost: Some(total_interest),
        down_payment: Some(down_payment),
        financed_amount: Some(financed),
        annual_debt_service: Some(annual_ds),
        // Not applicable
        discount_amount: None,
        supplier_proceeds: None,
        supplier_effective_rate: None,
        supplier_savings: None,
        buyer_dpo_extension: None,
        funder_return: None,
        discount_pct: None,
        buyer_annualized_return: None,
        buyer_npv: None,
        discount: None,
        proceeds: None,
        commitment_fee: None,
        net_proceeds: None,
        effective_yield: None,
        exporter_effective_cost: None,
        warnings: warnings.clone(),
    })
}

// ---------------------------------------------------------------------------
// Export Credit helpers
// ---------------------------------------------------------------------------

/// Equal principal repayment: each period pays principal/n + outstanding*rate.
/// Returns (first_year_debt_service, total_interest_over_tenor).
fn compute_equal_principal_debt_service(
    principal: Money,
    rate: Rate,
    tenor: u32,
    _warnings: &mut Vec<String>,
) -> (Money, Money) {
    let n = Decimal::from(tenor);
    let annual_principal = principal / n;

    let mut outstanding = principal;
    let mut total_interest = Decimal::ZERO;
    let mut first_year_ds = Decimal::ZERO;

    for year in 1..=tenor {
        let interest = outstanding * rate;
        total_interest += interest;
        let ds = annual_principal + interest;
        if year == 1 {
            first_year_ds = ds;
        }
        outstanding -= annual_principal;
    }

    (first_year_ds, total_interest)
}

/// Annuity repayment: level payment via standard annuity formula.
/// PMT = P * r / (1 - (1+r)^{-n}), using iterative multiplication.
/// Returns (annual_payment, total_interest_over_tenor).
fn compute_annuity_debt_service(
    principal: Money,
    rate: Rate,
    tenor: u32,
    warnings: &mut Vec<String>,
) -> (Money, Money) {
    if rate.is_zero() {
        // Zero rate: just divide principal equally
        let pmt = principal / Decimal::from(tenor);
        return (pmt, Decimal::ZERO);
    }

    // Compute (1+r)^n via iterative multiplication
    let one_plus_r = Decimal::ONE + rate;
    let mut compound = Decimal::ONE;
    for _ in 0..tenor {
        compound *= one_plus_r;
    }

    // PMT = P * r * (1+r)^n / ((1+r)^n - 1)
    let denominator = compound - Decimal::ONE;
    if denominator.is_zero() {
        warnings.push("Annuity denominator is zero, falling back to equal principal".into());
        let pmt = principal / Decimal::from(tenor);
        return (pmt, Decimal::ZERO);
    }

    let pmt = principal * rate * compound / denominator;
    let total_payments = pmt * Decimal::from(tenor);
    let total_interest = total_payments - principal;

    (pmt, total_interest)
}

// ---------------------------------------------------------------------------
// Utility
// ---------------------------------------------------------------------------

/// Extract a required field from an Option, returning InvalidInput on None.
fn required_field<T>(opt: Option<T>, field_name: &str) -> CorpFinanceResult<T> {
    opt.ok_or_else(|| CorpFinanceError::InvalidInput {
        field: field_name.to_string(),
        reason: format!("{} is required for this analysis type", field_name),
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    // --- Helpers ---

    fn reverse_factoring_input() -> SupplyChainFinanceInput {
        SupplyChainFinanceInput {
            analysis_type: ScfType::ReverseFactoring,
            invoice_amount: Some(dec!(1_000_000)),
            payment_terms_days: Some(90),
            early_payment_days: Some(10),
            supplier_cost_of_funds: Some(dec!(0.08)),
            buyer_credit_spread_bps: Some(dec!(100)), // 1% spread
            base_rate: Some(dec!(0.05)),              // 5% base
            platform_fee_bps: None,
            // Not used
            standard_payment_days: None,
            early_payment_day: None,
            discount_rate_per_day_bps: None,
            buyer_opportunity_cost: None,
            receivable_amount: None,
            maturity_days: None,
            discount_rate: None,
            commitment_fee_bps: None,
            is_avalised: None,
            avalising_bank_rating: None,
            grace_days: None,
            contract_value: None,
            eca_covered_pct: None,
            cirr_rate: None,
            commercial_rate: None,
            eca_premium_pct: None,
            tenor_years: None,
            repayment_type: None,
            down_payment_pct: None,
        }
    }

    fn dynamic_discounting_input() -> SupplyChainFinanceInput {
        SupplyChainFinanceInput {
            analysis_type: ScfType::DynamicDiscounting,
            invoice_amount: Some(dec!(500_000)),
            standard_payment_days: Some(60),
            early_payment_day: Some(10),
            discount_rate_per_day_bps: Some(dec!(5)), // 5 bps/day
            buyer_opportunity_cost: Some(dec!(0.10)), // 10% WACC
            // Not used
            payment_terms_days: None,
            early_payment_days: None,
            supplier_cost_of_funds: None,
            buyer_credit_spread_bps: None,
            base_rate: None,
            platform_fee_bps: None,
            receivable_amount: None,
            maturity_days: None,
            discount_rate: None,
            commitment_fee_bps: None,
            is_avalised: None,
            avalising_bank_rating: None,
            grace_days: None,
            contract_value: None,
            eca_covered_pct: None,
            cirr_rate: None,
            commercial_rate: None,
            eca_premium_pct: None,
            tenor_years: None,
            repayment_type: None,
            down_payment_pct: None,
        }
    }

    fn forfaiting_input() -> SupplyChainFinanceInput {
        SupplyChainFinanceInput {
            analysis_type: ScfType::Forfaiting,
            receivable_amount: Some(dec!(2_000_000)),
            maturity_days: Some(180),
            discount_rate: Some(dec!(0.06)),
            commitment_fee_bps: Some(dec!(50)), // 50 bps
            is_avalised: Some(true),
            avalising_bank_rating: Some("AA".to_string()),
            grace_days: Some(3),
            // Not used
            invoice_amount: None,
            payment_terms_days: None,
            early_payment_days: None,
            supplier_cost_of_funds: None,
            buyer_credit_spread_bps: None,
            base_rate: None,
            platform_fee_bps: None,
            standard_payment_days: None,
            early_payment_day: None,
            discount_rate_per_day_bps: None,
            buyer_opportunity_cost: None,
            contract_value: None,
            eca_covered_pct: None,
            cirr_rate: None,
            commercial_rate: None,
            eca_premium_pct: None,
            tenor_years: None,
            repayment_type: None,
            down_payment_pct: None,
        }
    }

    fn export_credit_input() -> SupplyChainFinanceInput {
        SupplyChainFinanceInput {
            analysis_type: ScfType::ExportCredit,
            contract_value: Some(dec!(50_000_000)),
            eca_covered_pct: Some(dec!(0.85)),
            cirr_rate: Some(dec!(0.03)),
            commercial_rate: Some(dec!(0.06)),
            eca_premium_pct: Some(dec!(0.02)),
            tenor_years: Some(7),
            repayment_type: Some(RepaymentType::EqualPrincipal),
            down_payment_pct: Some(dec!(0.15)),
            // Not used
            invoice_amount: None,
            payment_terms_days: None,
            early_payment_days: None,
            supplier_cost_of_funds: None,
            buyer_credit_spread_bps: None,
            base_rate: None,
            platform_fee_bps: None,
            standard_payment_days: None,
            early_payment_day: None,
            discount_rate_per_day_bps: None,
            buyer_opportunity_cost: None,
            receivable_amount: None,
            maturity_days: None,
            discount_rate: None,
            commitment_fee_bps: None,
            is_avalised: None,
            avalising_bank_rating: None,
            grace_days: None,
        }
    }

    // -----------------------------------------------------------------------
    // 1. Reverse factoring: discount and proceeds
    // -----------------------------------------------------------------------
    #[test]
    fn test_reverse_factoring_discount_and_proceeds() {
        let input = reverse_factoring_input();
        let result = analyze_supply_chain_finance(&input).unwrap();
        let out = &result.result;

        // Program rate = 0.05 + 100/10000 = 0.06
        // Days accelerated = 90 - 10 = 80
        // Discount rate = 0.06 * 80/360 = 0.013333...
        // Discount amount = 1,000,000 * 0.013333... = 13,333.33...
        let expected_discount = dec!(1_000_000) * dec!(0.06) * dec!(80) / dec!(360);
        let discount = out.discount_amount.unwrap();
        let diff = (discount - expected_discount).abs();
        assert!(
            diff < dec!(0.01),
            "Discount should be ~{}, got {}",
            expected_discount,
            discount
        );

        let proceeds = out.supplier_proceeds.unwrap();
        let expected_proceeds = dec!(1_000_000) - expected_discount;
        let diff2 = (proceeds - expected_proceeds).abs();
        assert!(
            diff2 < dec!(0.01),
            "Proceeds should be ~{}, got {}",
            expected_proceeds,
            proceeds
        );
    }

    // -----------------------------------------------------------------------
    // 2. Supplier savings vs own cost of funds
    // -----------------------------------------------------------------------
    #[test]
    fn test_reverse_factoring_supplier_savings() {
        let input = reverse_factoring_input();
        let result = analyze_supply_chain_finance(&input).unwrap();
        let out = &result.result;

        // Supplier alt cost = 1,000,000 * 0.08 * 80/360 = 17,777.78
        // Discount = 1,000,000 * 0.06 * 80/360 = 13,333.33
        // Savings = 17,777.78 - 13,333.33 = 4,444.44
        let expected_alt_cost = dec!(1_000_000) * dec!(0.08) * dec!(80) / dec!(360);
        let expected_discount = dec!(1_000_000) * dec!(0.06) * dec!(80) / dec!(360);
        let expected_savings = expected_alt_cost - expected_discount;

        let savings = out.supplier_savings.unwrap();
        let diff = (savings - expected_savings).abs();
        assert!(
            diff < dec!(0.01),
            "Supplier savings should be ~{}, got {}",
            expected_savings,
            savings
        );
        assert!(
            savings > Decimal::ZERO,
            "Savings should be positive when program rate < supplier cost of funds"
        );
    }

    // -----------------------------------------------------------------------
    // 3. Dynamic discounting buyer return
    // -----------------------------------------------------------------------
    #[test]
    fn test_dynamic_discounting_buyer_return() {
        let input = dynamic_discounting_input();
        let result = analyze_supply_chain_finance(&input).unwrap();
        let out = &result.result;

        // Days early = 60 - 10 = 50
        // Discount % = 50 * 5 / 10000 = 0.025 (2.5%)
        let expected_disc_pct = dec!(50) * dec!(5) / dec!(10000);
        assert_eq!(
            out.discount_pct.unwrap(),
            expected_disc_pct,
            "Discount pct should be {}",
            expected_disc_pct
        );

        // Buyer annualized return = (0.025 / 0.975) * (360/50)
        let expected_return =
            (expected_disc_pct / (Decimal::ONE - expected_disc_pct)) * (dec!(360) / dec!(50));
        let actual_return = out.buyer_annualized_return.unwrap();
        let diff = (actual_return - expected_return).abs();
        assert!(
            diff < dec!(0.0001),
            "Buyer return should be ~{}, got {}",
            expected_return,
            actual_return
        );

        // Return should be significantly above 10% (buyer's WACC), confirming value
        assert!(
            actual_return > dec!(0.10),
            "Buyer return ({}) should exceed WACC (10%)",
            actual_return
        );
    }

    // -----------------------------------------------------------------------
    // 4. Forfaiting discount with grace days
    // -----------------------------------------------------------------------
    #[test]
    fn test_forfaiting_discount_with_grace_days() {
        let input = forfaiting_input();
        let result = analyze_supply_chain_finance(&input).unwrap();
        let out = &result.result;

        // Total days = 180 + 3 = 183
        // Discount = 2,000,000 * 0.06 * 183/360
        let expected_discount = dec!(2_000_000) * dec!(0.06) * dec!(183) / dec!(360);
        let discount = out.discount.unwrap();
        let diff = (discount - expected_discount).abs();
        assert!(
            diff < dec!(0.01),
            "Forfaiting discount should be ~{}, got {}",
            expected_discount,
            discount
        );

        // Proceeds = 2M - discount
        let expected_proceeds = dec!(2_000_000) - expected_discount;
        let proceeds = out.proceeds.unwrap();
        let diff2 = (proceeds - expected_proceeds).abs();
        assert!(
            diff2 < dec!(0.01),
            "Forfaiting proceeds should be ~{}, got {}",
            expected_proceeds,
            proceeds
        );
    }

    // -----------------------------------------------------------------------
    // 5. Forfaiting with avalised bill flag
    // -----------------------------------------------------------------------
    #[test]
    fn test_forfaiting_avalised_bill() {
        let input = forfaiting_input();
        let result = analyze_supply_chain_finance(&input).unwrap();

        // Check warnings contain avalised info
        let has_aval_warning = result
            .result
            .warnings
            .iter()
            .any(|w| w.contains("Avalised") || w.contains("avalised"));
        assert!(
            has_aval_warning || result.warnings.iter().any(|w| w.contains("Avalised")),
            "Should have a warning about avalised bill status"
        );

        // Non-avalised should warn about higher risk
        let mut non_aval_input = forfaiting_input();
        non_aval_input.is_avalised = Some(false);
        let result2 = analyze_supply_chain_finance(&non_aval_input).unwrap();
        let has_risk_warning = result2
            .result
            .warnings
            .iter()
            .any(|w| w.contains("not avalised"));
        assert!(
            has_risk_warning,
            "Non-avalised bill should warn about higher credit risk"
        );
    }

    // -----------------------------------------------------------------------
    // 6. Export credit blended rate (85% ECA cover)
    // -----------------------------------------------------------------------
    #[test]
    fn test_export_credit_blended_rate() {
        let input = export_credit_input();
        let result = analyze_supply_chain_finance(&input).unwrap();
        let out = &result.result;

        // Down payment = 50M * 0.15 = 7.5M
        // Financed = 50M - 7.5M = 42.5M
        // ECA covered = 42.5M * 0.85 = 36.125M
        // Commercial = 42.5M - 36.125M = 6.375M
        // Blended = (36.125M * 0.03 + 6.375M * 0.06) / 42.5M
        //         = (1,083,750 + 382,500) / 42,500,000
        //         = 1,466,250 / 42,500,000
        //         = 0.0345

        let financed = dec!(42_500_000);
        let eca_cov = financed * dec!(0.85);
        let comm = financed - eca_cov;
        let expected_blended = (eca_cov * dec!(0.03) + comm * dec!(0.06)) / financed;

        let blended = out.blended_rate.unwrap();
        let diff = (blended - expected_blended).abs();
        assert!(
            diff < dec!(0.0001),
            "Blended rate should be ~{}, got {}",
            expected_blended,
            blended
        );

        // Should be between CIRR and commercial rate
        assert!(blended > dec!(0.03) && blended < dec!(0.06));

        // Check ECA amounts
        assert_eq!(out.eca_covered_amount.unwrap(), eca_cov);
        assert_eq!(out.commercial_amount.unwrap(), comm);
        assert_eq!(out.down_payment.unwrap(), dec!(7_500_000));
        assert_eq!(out.financed_amount.unwrap(), financed);
    }

    // -----------------------------------------------------------------------
    // 7. Export credit annual debt service (equal principal)
    // -----------------------------------------------------------------------
    #[test]
    fn test_export_credit_equal_principal_debt_service() {
        let input = export_credit_input();
        let result = analyze_supply_chain_finance(&input).unwrap();
        let out = &result.result;

        // Financed = 42.5M, blended_rate ~ 0.0345, tenor = 7
        // Year 1: principal/7 + 42.5M * rate
        let financed = dec!(42_500_000);
        let rate = out.blended_rate.unwrap();
        let annual_principal = financed / dec!(7);
        let year1_interest = financed * rate;
        let expected_ds = annual_principal + year1_interest;

        let ds = out.annual_debt_service.unwrap();
        let diff = (ds - expected_ds).abs();
        assert!(
            diff < dec!(1), // within $1
            "Year 1 debt service should be ~{}, got {}",
            expected_ds,
            ds
        );

        // Total interest should be positive and less than financed amount
        let total_int = out.total_interest_cost.unwrap();
        assert!(total_int > Decimal::ZERO);
        assert!(total_int < financed);
    }

    // -----------------------------------------------------------------------
    // 8. Edge case: early_payment_days >= payment_terms_days -> error
    // -----------------------------------------------------------------------
    #[test]
    fn test_reverse_factoring_early_days_gte_payment_terms() {
        let mut input = reverse_factoring_input();
        input.early_payment_days = Some(90); // equal to payment terms

        let result = analyze_supply_chain_finance(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "early_payment_days");
            }
            other => panic!("Expected InvalidInput, got {:?}", other),
        }

        // Also test > case
        let mut input2 = reverse_factoring_input();
        input2.early_payment_days = Some(100);
        let result2 = analyze_supply_chain_finance(&input2);
        assert!(result2.is_err());
    }

    // -----------------------------------------------------------------------
    // 9. Reverse factoring with platform fee
    // -----------------------------------------------------------------------
    #[test]
    fn test_reverse_factoring_with_platform_fee() {
        let mut input = reverse_factoring_input();
        input.platform_fee_bps = Some(dec!(25)); // 25 bps

        let result = analyze_supply_chain_finance(&input).unwrap();
        let out = &result.result;

        // Platform fee = 1,000,000 * 25/10000 * 80/360
        let expected_platform_fee = dec!(1_000_000) * dec!(25) / dec!(10000) * dec!(80) / dec!(360);
        let discount = out.discount_amount.unwrap();
        let proceeds = out.supplier_proceeds.unwrap();

        // Proceeds should be invoice - discount - platform_fee
        let expected_proceeds = dec!(1_000_000) - discount - expected_platform_fee;
        let diff = (proceeds - expected_proceeds).abs();
        assert!(
            diff < dec!(0.01),
            "Proceeds with platform fee should be ~{}, got {}",
            expected_proceeds,
            proceeds
        );
    }

    // -----------------------------------------------------------------------
    // 10. Forfaiting effective yield
    // -----------------------------------------------------------------------
    #[test]
    fn test_forfaiting_effective_yield() {
        let input = forfaiting_input();
        let result = analyze_supply_chain_finance(&input).unwrap();
        let out = &result.result;

        let net = out.net_proceeds.unwrap();
        let total_days_dec = dec!(183);
        // effective_yield = ((2M / net) - 1) * (360 / 183)
        let expected = (dec!(2_000_000) / net - Decimal::ONE) * (dec!(360) / total_days_dec);
        let ey = out.effective_yield.unwrap();
        let diff = (ey - expected).abs();
        assert!(
            diff < dec!(0.0001),
            "Effective yield should be ~{}, got {}",
            expected,
            ey
        );
        // Yield should be higher than the stated discount rate (due to commitment fee)
        assert!(ey > dec!(0.06), "Effective yield should exceed stated 6%");
    }

    // -----------------------------------------------------------------------
    // 11. Export credit annuity repayment
    // -----------------------------------------------------------------------
    #[test]
    fn test_export_credit_annuity_repayment() {
        let mut input = export_credit_input();
        input.repayment_type = Some(RepaymentType::Annuity);

        let result = analyze_supply_chain_finance(&input).unwrap();
        let out = &result.result;

        let financed = out.financed_amount.unwrap();
        let pmt = out.annual_debt_service.unwrap();

        // Annuity payment should be level and positive
        assert!(pmt > Decimal::ZERO);

        // Total payments = pmt * 7 should exceed principal (due to interest)
        let total_payments = pmt * dec!(7);
        assert!(
            total_payments > financed,
            "Total annuity payments ({}) should exceed principal ({})",
            total_payments,
            financed
        );

        // Total interest should equal total payments - principal
        let total_int = out.total_interest_cost.unwrap();
        let diff = (total_int - (total_payments - financed)).abs();
        assert!(
            diff < dec!(1),
            "Total interest should be ~{}, got {}",
            total_payments - financed,
            total_int
        );
    }

    // -----------------------------------------------------------------------
    // 12. Dynamic discounting: buyer NPV
    // -----------------------------------------------------------------------
    #[test]
    fn test_dynamic_discounting_buyer_npv() {
        let input = dynamic_discounting_input();
        let result = analyze_supply_chain_finance(&input).unwrap();
        let out = &result.result;

        // Discount amount = 500,000 * 0.025 = 12,500
        // Opp cost = 500,000 * 0.10 * 50/360 = 6,944.44
        // NPV = 12,500 - 6,944.44 = 5,555.56
        let disc_amt = out.discount_amount.unwrap();
        let expected_disc = dec!(500_000) * dec!(0.025);
        assert_eq!(disc_amt, expected_disc);

        let opp_cost = dec!(500_000) * dec!(0.10) * dec!(50) / dec!(360);
        let expected_npv = expected_disc - opp_cost;
        let npv = out.buyer_npv.unwrap();
        let diff = (npv - expected_npv).abs();
        assert!(
            diff < dec!(0.01),
            "Buyer NPV should be ~{}, got {}",
            expected_npv,
            npv
        );
        assert!(npv > Decimal::ZERO, "Buyer NPV should be positive");
    }

    // -----------------------------------------------------------------------
    // 13. Funder return calculation
    // -----------------------------------------------------------------------
    #[test]
    fn test_reverse_factoring_funder_return() {
        let input = reverse_factoring_input();
        let result = analyze_supply_chain_finance(&input).unwrap();
        let out = &result.result;

        let discount = out.discount_amount.unwrap();
        let proceeds = out.supplier_proceeds.unwrap();

        // funder_return = (discount / proceeds) * (360 / 80)
        let expected_return = (discount / proceeds) * (dec!(360) / dec!(80));
        let actual = out.funder_return.unwrap();
        let diff = (actual - expected_return).abs();
        assert!(
            diff < dec!(0.0001),
            "Funder return should be ~{}, got {}",
            expected_return,
            actual
        );
    }

    // -----------------------------------------------------------------------
    // 14. Export credit ECA premium
    // -----------------------------------------------------------------------
    #[test]
    fn test_export_credit_eca_premium() {
        let input = export_credit_input();
        let result = analyze_supply_chain_finance(&input).unwrap();
        let out = &result.result;

        // ECA premium = 36,125,000 * 0.02 = 722,500
        let eca_covered = out.eca_covered_amount.unwrap();
        let expected_premium = eca_covered * dec!(0.02);
        let premium = out.eca_premium.unwrap();
        assert_eq!(
            premium, expected_premium,
            "ECA premium should be {}, got {}",
            expected_premium, premium
        );
    }

    // -----------------------------------------------------------------------
    // 15. Validation: negative invoice amount
    // -----------------------------------------------------------------------
    #[test]
    fn test_negative_invoice_amount() {
        let mut input = reverse_factoring_input();
        input.invoice_amount = Some(dec!(-100));

        let result = analyze_supply_chain_finance(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "invoice_amount");
            }
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // 16. Validation: missing required field
    // -----------------------------------------------------------------------
    #[test]
    fn test_missing_required_field() {
        let mut input = reverse_factoring_input();
        input.base_rate = None;

        let result = analyze_supply_chain_finance(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "base_rate");
            }
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // 17. Metadata is populated
    // -----------------------------------------------------------------------
    #[test]
    fn test_metadata_populated() {
        let input = reverse_factoring_input();
        let result = analyze_supply_chain_finance(&input).unwrap();

        assert!(result.methodology.contains("Reverse Factoring"));
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
        assert!(!result.metadata.version.is_empty());
    }

    // -----------------------------------------------------------------------
    // 18. Forfaiting without commitment fee
    // -----------------------------------------------------------------------
    #[test]
    fn test_forfaiting_no_commitment_fee() {
        let mut input = forfaiting_input();
        input.commitment_fee_bps = None;

        let result = analyze_supply_chain_finance(&input).unwrap();
        let out = &result.result;

        assert!(out.commitment_fee.is_none());
        // net_proceeds = proceeds (no fee deducted)
        assert_eq!(out.net_proceeds.unwrap(), out.proceeds.unwrap());
    }

    // -----------------------------------------------------------------------
    // 19. Export credit down payment warning
    // -----------------------------------------------------------------------
    #[test]
    fn test_export_credit_low_down_payment_warning() {
        let mut input = export_credit_input();
        input.down_payment_pct = Some(dec!(0.10)); // 10% < 15% OECD minimum

        let result = analyze_supply_chain_finance(&input).unwrap();
        let has_dp_warning = result
            .result
            .warnings
            .iter()
            .any(|w| w.contains("OECD Consensus"));
        assert!(has_dp_warning, "Should warn about below-OECD down payment");
    }

    // -----------------------------------------------------------------------
    // 20. Dynamic discounting edge: early_payment_day >= standard
    // -----------------------------------------------------------------------
    #[test]
    fn test_dynamic_discounting_early_day_gte_standard() {
        let mut input = dynamic_discounting_input();
        input.early_payment_day = Some(60); // equal to standard

        let result = analyze_supply_chain_finance(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "early_payment_day");
            }
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }
}
