//! Letter of Credit pricing, risk assessment, standby LC / bank guarantee
//! pricing, and banker's acceptance discounting.
//!
//! Covers:
//! 1. **LC fee calculation** -- issuance, confirmation, advising, negotiation,
//!    amendment fees and all-in cost (annualized).
//! 2. **Risk assessment** -- country risk, bank risk, tenor risk, documentary
//!    risk on a 1-10 scale with weighted overall score.
//! 3. **Standby LC / bank guarantee** -- commitment fees, utilization.
//! 4. **BA discounting** -- discount yield and proceeds for usance / deferred
//!    payment LCs.
//!
//! All arithmetic uses `rust_decimal::Decimal`. Day-count convention is
//! Actual/360 (money-market standard for trade finance).

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

/// Basis-point divisor.
const BPS: Decimal = dec!(10000);

/// Money-market day-count denominator (Actual/360).
const DAY_COUNT_BASE: Decimal = dec!(360);

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// Type of letter of credit.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LcType {
    /// Standard commercial (import/export) LC.
    Commercial,
    /// Standby LC / bank guarantee.
    Standby,
    /// Revolving LC -- amount reinstates after each drawing.
    Revolving,
    /// Back-to-back LC (intermediary structure).
    BackToBack,
    /// Transferable LC.
    Transferable,
}

impl std::fmt::Display for LcType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LcType::Commercial => write!(f, "Commercial"),
            LcType::Standby => write!(f, "Standby"),
            LcType::Revolving => write!(f, "Revolving"),
            LcType::BackToBack => write!(f, "Back-to-Back"),
            LcType::Transferable => write!(f, "Transferable"),
        }
    }
}

// ---------------------------------------------------------------------------
// Input
// ---------------------------------------------------------------------------

/// Full input for letter-of-credit pricing and risk assessment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LetterOfCreditInput {
    /// Type of LC.
    pub lc_type: LcType,
    /// Face (nominal) amount of the LC.
    pub face_amount: Money,
    /// ISO currency code (e.g. "USD").
    pub currency: String,
    /// Validity / tenor in calendar days.
    pub tenor_days: u32,
    /// Credit rating of the issuing bank (AAA through B).
    pub issuing_bank_rating: String,
    /// Credit rating of the confirming bank, if confirmed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirming_bank_rating: Option<String>,
    /// ISO 3166-1 alpha-2 country code of the applicant (importer).
    pub applicant_country: String,
    /// ISO 3166-1 alpha-2 country code of the beneficiary (exporter).
    pub beneficiary_country: String,
    /// Issuance fee in basis points (annual basis, prorated over tenor).
    pub issuance_fee_bps: Decimal,
    /// Confirmation fee in basis points (annual basis, prorated over tenor).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirmation_fee_bps: Option<Decimal>,
    /// Flat advising fee.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub advising_fee_flat: Option<Money>,
    /// Negotiation fee in basis points (per drawing).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub negotiation_fee_bps: Option<Decimal>,
    /// Flat amendment fee (per amendment).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amendment_fee_flat: Option<Money>,
    /// Expected number of drawings under the LC.
    pub drawing_count: u32,
    /// Whether the LC is confirmed by a second bank.
    pub is_confirmed: bool,
    /// `true` = sight LC (payment at presentation), `false` = usance /
    /// deferred payment.
    pub is_at_sight: bool,
    /// Number of days for deferred payment (usance LCs only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deferred_payment_days: Option<u32>,
    /// Discount rate for banker's acceptance discounting.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discount_rate: Option<Rate>,
    /// Brief description of the underlying goods (informational).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub goods_description: Option<String>,
}

// ---------------------------------------------------------------------------
// Output
// ---------------------------------------------------------------------------

/// Risk assessment on a 1-10 scale for each dimension.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    /// Country / sovereign risk (1 = low, 10 = sanctioned / CCC).
    pub country_risk: Decimal,
    /// Issuing-bank credit risk.
    pub bank_risk: Decimal,
    /// Tenor / maturity risk.
    pub tenor_risk: Decimal,
    /// Documentary / payment-type risk.
    pub documentary_risk: Decimal,
    /// Weighted overall risk score.
    pub overall: Decimal,
}

/// Complete output from LC pricing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LetterOfCreditOutput {
    /// LC type (display string).
    pub lc_type: String,
    /// Face amount of the LC.
    pub face_amount: Money,
    /// Issuance fee = face x (bps/10000) x (tenor/360).
    pub issuance_fee: Money,
    /// Confirmation fee (if confirmed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirmation_fee: Option<Money>,
    /// Advising fee (flat).
    pub advising_fee: Money,
    /// Negotiation fee (if applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub negotiation_fee: Option<Money>,
    /// Total amendment fees.
    pub amendment_fees: Money,
    /// Sum of all fees.
    pub total_fees: Money,
    /// All-in cost as an annualized percentage of face amount.
    pub all_in_cost_pct: Rate,
    /// Multi-dimensional risk assessment.
    pub risk_score: RiskAssessment,
    /// Discounted proceeds for usance / deferred payment LCs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discount_proceeds: Option<Money>,
    /// Discount yield for BA discounting.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discount_yield: Option<Rate>,
    /// Effective financing cost (all-in cost including discount).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective_financing_cost: Option<Rate>,
    /// Advisory warnings (e.g. high-risk country, long tenor).
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Price a letter of credit: compute fees, risk assessment, and (for usance
/// LCs) banker's acceptance discount proceeds / yield.
pub fn price_letter_of_credit(
    input: &LetterOfCreditInput,
) -> CorpFinanceResult<ComputationOutput<LetterOfCreditOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // --- Validate ---------------------------------------------------------
    validate_input(input)?;

    let face = input.face_amount;
    let tenor = Decimal::from(input.tenor_days);

    // --- Fee calculations -------------------------------------------------

    // Issuance fee: face x (bps / 10_000) x (tenor / 360)
    let issuance_fee = face * (input.issuance_fee_bps / BPS) * (tenor / DAY_COUNT_BASE);

    // Confirmation fee (only if confirmed and bps provided)
    let confirmation_fee = if input.is_confirmed {
        let bps = input.confirmation_fee_bps.unwrap_or(Decimal::ZERO);
        if bps.is_zero() {
            warnings.push(
                "LC is confirmed but no confirmation_fee_bps provided; \
                 defaulting to zero"
                    .into(),
            );
        }
        Some(face * (bps / BPS) * (tenor / DAY_COUNT_BASE))
    } else {
        None
    };

    // Advising fee (flat, default to zero)
    let advising_fee = input.advising_fee_flat.unwrap_or(Decimal::ZERO);

    // Negotiation fee: face x (bps / 10_000) per drawing
    let negotiation_fee = input.negotiation_fee_bps.map(|bps| {
        let per_drawing = face * (bps / BPS);
        let drawings = Decimal::from(input.drawing_count.max(1));
        per_drawing * drawings
    });

    // Amendment fees (flat x 1 occurrence -- assume one amendment expected)
    let amendment_fees = input.amendment_fee_flat.unwrap_or(Decimal::ZERO);

    // Total fees
    let total_fees = issuance_fee
        + confirmation_fee.unwrap_or(Decimal::ZERO)
        + advising_fee
        + negotiation_fee.unwrap_or(Decimal::ZERO)
        + amendment_fees;

    // All-in cost annualized: (total_fees / face) x (360 / tenor)
    let all_in_cost_pct = if face.is_zero() || tenor.is_zero() {
        Decimal::ZERO
    } else {
        (total_fees / face) * (DAY_COUNT_BASE / tenor)
    };

    // --- Risk assessment --------------------------------------------------
    let risk_score = assess_risk(input, &mut warnings);

    // --- BA discounting (usance / deferred LCs only) ----------------------
    let (discount_proceeds, discount_yield, effective_financing_cost) =
        compute_discount(input, all_in_cost_pct, &mut warnings);

    // --- Warnings for elevated risk ---------------------------------------
    if risk_score.overall > dec!(7) {
        warnings.push(format!(
            "Overall risk score is elevated ({}/10). Consider additional \
             credit enhancement or insurance.",
            risk_score.overall.round_dp(1)
        ));
    }
    if input.tenor_days > 360 {
        warnings.push("Tenor exceeds 360 days; extended-tenor surcharges may apply.".into());
    }

    // --- Assemble output --------------------------------------------------
    let output = LetterOfCreditOutput {
        lc_type: input.lc_type.to_string(),
        face_amount: face,
        issuance_fee,
        confirmation_fee,
        advising_fee,
        negotiation_fee,
        amendment_fees,
        total_fees,
        all_in_cost_pct,
        risk_score,
        discount_proceeds,
        discount_yield,
        effective_financing_cost,
        warnings: warnings.clone(),
    };

    let elapsed = start.elapsed().as_micros() as u64;

    Ok(with_metadata(
        "Letter of Credit Pricing — fee calculation, risk assessment, BA discounting",
        &serde_json::json!({
            "lc_type": input.lc_type.to_string(),
            "day_count": "Actual/360",
            "risk_weights": {
                "country": "30%",
                "bank": "30%",
                "tenor": "20%",
                "documentary": "20%"
            },
            "discount_method": "simple discount (face x rate x days/360)"
        }),
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_input(input: &LetterOfCreditInput) -> CorpFinanceResult<()> {
    if input.face_amount <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "face_amount".into(),
            reason: "Face amount must be positive".into(),
        });
    }
    if input.tenor_days == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "tenor_days".into(),
            reason: "Tenor must be at least 1 day".into(),
        });
    }
    if input.issuance_fee_bps < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "issuance_fee_bps".into(),
            reason: "Issuance fee cannot be negative".into(),
        });
    }
    if let Some(bps) = input.confirmation_fee_bps {
        if bps < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "confirmation_fee_bps".into(),
                reason: "Confirmation fee cannot be negative".into(),
            });
        }
    }
    if let Some(bps) = input.negotiation_fee_bps {
        if bps < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "negotiation_fee_bps".into(),
                reason: "Negotiation fee cannot be negative".into(),
            });
        }
    }
    if let Some(flat) = input.advising_fee_flat {
        if flat < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "advising_fee_flat".into(),
                reason: "Advising fee cannot be negative".into(),
            });
        }
    }
    if let Some(flat) = input.amendment_fee_flat {
        if flat < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "amendment_fee_flat".into(),
                reason: "Amendment fee cannot be negative".into(),
            });
        }
    }
    if !input.is_at_sight
        && (input.deferred_payment_days.is_none() || input.deferred_payment_days == Some(0))
    {
        return Err(CorpFinanceError::InvalidInput {
            field: "deferred_payment_days".into(),
            reason: "Usance/deferred LC requires deferred_payment_days > 0".into(),
        });
    }
    validate_rating(&input.issuing_bank_rating, "issuing_bank_rating")?;
    if let Some(ref rating) = input.confirming_bank_rating {
        validate_rating(rating, "confirming_bank_rating")?;
    }
    Ok(())
}

/// Ensure a rating string is one of the accepted values.
fn validate_rating(rating: &str, field: &str) -> CorpFinanceResult<()> {
    let valid = [
        "AAA", "AA+", "AA", "AA-", "A+", "A", "A-", "BBB+", "BBB", "BBB-", "BB+", "BB", "BB-",
        "B+", "B", "B-", "CCC", "CC", "C", "D",
    ];
    if !valid.contains(&rating) {
        return Err(CorpFinanceError::InvalidInput {
            field: field.into(),
            reason: format!(
                "Invalid rating '{}'. Expected one of: {}",
                rating,
                valid.join(", ")
            ),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Risk assessment
// ---------------------------------------------------------------------------

/// Compute a multi-dimensional risk score (each 1-10).
fn assess_risk(input: &LetterOfCreditInput, warnings: &mut Vec<String>) -> RiskAssessment {
    let country_risk = country_risk_score(&input.applicant_country, &input.beneficiary_country);
    let bank_risk = bank_risk_score(&input.issuing_bank_rating);
    let tenor_risk = tenor_risk_score(input.tenor_days);
    let documentary_risk = documentary_risk_score(input.is_at_sight, input.deferred_payment_days);

    // Weighted overall: country 30%, bank 30%, tenor 20%, documentary 20%
    let overall = country_risk * dec!(0.30)
        + bank_risk * dec!(0.30)
        + tenor_risk * dec!(0.20)
        + documentary_risk * dec!(0.20);

    if country_risk >= dec!(8) {
        warnings.push(format!(
            "High country risk ({}/10) for applicant '{}' / beneficiary '{}'.",
            country_risk, input.applicant_country, input.beneficiary_country
        ));
    }
    if bank_risk >= dec!(7) {
        warnings.push(format!(
            "Elevated bank risk ({}/10) for issuing bank rating '{}'.",
            bank_risk, input.issuing_bank_rating
        ));
    }

    RiskAssessment {
        country_risk,
        bank_risk,
        tenor_risk,
        documentary_risk,
        overall,
    }
}

/// Map the max country risk of applicant and beneficiary.
fn country_risk_score(applicant: &str, beneficiary: &str) -> Decimal {
    let app = single_country_risk(applicant);
    let ben = single_country_risk(beneficiary);
    // Take the higher risk of the two counterparties
    if app > ben {
        app
    } else {
        ben
    }
}

/// Map a single ISO-2 country code to a 1-10 risk score.
fn single_country_risk(code: &str) -> Decimal {
    match code.to_uppercase().as_str() {
        // Tier 1 -- developed, low-risk (1-2)
        "US" | "GB" | "DE" | "FR" | "JP" | "CH" | "CA" | "AU" | "NL" | "SE" | "NO" | "DK"
        | "FI" | "SG" | "NZ" | "AT" | "BE" | "LU" | "IE" => dec!(1),

        // Tier 2 -- investment-grade emerging (3-4)
        "KR" | "TW" | "CZ" | "PL" | "CL" | "AE" | "QA" | "KW" | "IL" | "ES" | "IT" | "PT"
        | "HK" | "SA" => dec!(3),

        // Tier 3 -- mid-risk emerging (5-6)
        "CN" | "IN" | "BR" | "MX" | "TH" | "MY" | "ID" | "PH" | "CO" | "PE" | "ZA" | "TR"
        | "RO" | "BG" | "VN" => dec!(5),

        // Tier 4 -- high-risk (7-8)
        "NG" | "EG" | "PK" | "BD" | "KE" | "GH" | "TZ" | "ET" | "UA" | "AR" | "LK" | "MM" => {
            dec!(7)
        }

        // Tier 5 -- sanctioned / CCC (9-10)
        "IR" | "KP" | "SY" | "CU" | "VE" | "RU" | "BY" | "SD" | "SO" | "YE" | "LY" | "AF" => {
            dec!(9)
        }

        // Unknown / unrecognized -- default to elevated
        _ => dec!(6),
    }
}

/// Map issuing-bank credit rating to a 1-10 risk score.
fn bank_risk_score(rating: &str) -> Decimal {
    match rating {
        "AAA" => dec!(1),
        "AA+" | "AA" | "AA-" => dec!(2),
        "A+" | "A" | "A-" => dec!(3),
        "BBB+" | "BBB" | "BBB-" => dec!(4),
        "BB+" | "BB" | "BB-" => dec!(6),
        "B+" | "B" | "B-" => dec!(8),
        "CCC" | "CC" | "C" | "D" => dec!(10),
        _ => dec!(5),
    }
}

/// Map tenor (days) to a 1-10 risk score.
fn tenor_risk_score(days: u32) -> Decimal {
    match days {
        0..=89 => dec!(2),
        90..=180 => dec!(4),
        181..=360 => dec!(6),
        _ => dec!(8),
    }
}

/// Map documentary / payment type to a 1-10 risk score.
fn documentary_risk_score(is_at_sight: bool, deferred_days: Option<u32>) -> Decimal {
    if is_at_sight {
        dec!(2)
    } else {
        match deferred_days.unwrap_or(0) {
            0..=90 => dec!(5),
            91..=180 => dec!(6),
            _ => dec!(7),
        }
    }
}

// ---------------------------------------------------------------------------
// BA discounting
// ---------------------------------------------------------------------------

/// Compute discount proceeds and yield for usance / deferred LCs with a
/// discount rate.
fn compute_discount(
    input: &LetterOfCreditInput,
    all_in_cost_pct: Rate,
    warnings: &mut Vec<String>,
) -> (Option<Money>, Option<Rate>, Option<Rate>) {
    // Only applicable for non-sight LCs with a discount rate
    if input.is_at_sight {
        return (None, None, None);
    }

    let deferred_days = match input.deferred_payment_days {
        Some(d) if d > 0 => d,
        _ => return (None, None, None),
    };

    let rate = match input.discount_rate {
        Some(r) => r,
        None => return (None, None, None),
    };

    if rate < Decimal::ZERO {
        warnings.push("Discount rate is negative; proceeding with calculation.".into());
    }

    let days = Decimal::from(deferred_days);
    let face = input.face_amount;

    // Simple discount: Discount = face x rate x (days / 360)
    let discount_amount = face * rate * (days / DAY_COUNT_BASE);
    let proceeds = face - discount_amount;

    // Discount yield: (face / proceeds - 1) x (360 / days)
    let discount_yield = if proceeds.is_zero() || proceeds < Decimal::ZERO {
        warnings.push("Discount proceeds are zero or negative; yield undefined.".into());
        Decimal::ZERO
    } else {
        (face / proceeds - Decimal::ONE) * (DAY_COUNT_BASE / days)
    };

    // Effective financing cost = all-in LC cost + discount yield (annualized)
    let effective_cost = all_in_cost_pct + discount_yield;

    (Some(proceeds), Some(discount_yield), Some(effective_cost))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    // --- Helper: standard sight commercial LC input -----------------------
    fn sight_commercial_lc() -> LetterOfCreditInput {
        LetterOfCreditInput {
            lc_type: LcType::Commercial,
            face_amount: dec!(1_000_000),
            currency: "USD".into(),
            tenor_days: 180,
            issuing_bank_rating: "A".into(),
            confirming_bank_rating: None,
            applicant_country: "US".into(),
            beneficiary_country: "DE".into(),
            issuance_fee_bps: dec!(100), // 100 bps annualized
            confirmation_fee_bps: None,
            advising_fee_flat: Some(dec!(500)),
            negotiation_fee_bps: Some(dec!(25)),
            amendment_fee_flat: Some(dec!(250)),
            drawing_count: 1,
            is_confirmed: false,
            is_at_sight: true,
            deferred_payment_days: None,
            discount_rate: None,
            goods_description: Some("Machinery parts".into()),
        }
    }

    // --- Helper: confirmed usance LC ------------------------------------
    fn confirmed_usance_lc() -> LetterOfCreditInput {
        LetterOfCreditInput {
            lc_type: LcType::Commercial,
            face_amount: dec!(2_000_000),
            currency: "USD".into(),
            tenor_days: 365,
            issuing_bank_rating: "BBB".into(),
            confirming_bank_rating: Some("AA".into()),
            applicant_country: "TR".into(),
            beneficiary_country: "DE".into(),
            issuance_fee_bps: dec!(150),
            confirmation_fee_bps: Some(dec!(75)),
            advising_fee_flat: Some(dec!(750)),
            negotiation_fee_bps: Some(dec!(20)),
            amendment_fee_flat: Some(dec!(500)),
            drawing_count: 2,
            is_confirmed: true,
            is_at_sight: false,
            deferred_payment_days: Some(90),
            discount_rate: Some(dec!(0.05)),
            goods_description: Some("Textiles".into()),
        }
    }

    // -------------------------------------------------------------------
    // 1. Sight commercial LC fee calculation
    // -------------------------------------------------------------------
    #[test]
    fn test_sight_commercial_lc_fees() {
        let input = sight_commercial_lc();
        let result = price_letter_of_credit(&input).unwrap();
        let out = &result.result;

        // Issuance fee = 1_000_000 * (100/10000) * (180/360)
        //              = 1_000_000 * 0.01 * 0.5 = 5_000
        assert_eq!(out.issuance_fee, dec!(5000));

        // No confirmation fee
        assert!(out.confirmation_fee.is_none());

        // Advising fee = 500
        assert_eq!(out.advising_fee, dec!(500));

        // Negotiation fee = 1_000_000 * (25/10000) * 1 drawing = 2_500
        assert_eq!(out.negotiation_fee, Some(dec!(2500)));

        // Amendment fees = 250
        assert_eq!(out.amendment_fees, dec!(250));

        // Total = 5000 + 500 + 2500 + 250 = 8250
        assert_eq!(out.total_fees, dec!(8250));
    }

    // -------------------------------------------------------------------
    // 2. Confirmed usance LC with confirmation fee
    // -------------------------------------------------------------------
    #[test]
    fn test_confirmed_usance_lc_with_confirmation_fee() {
        let input = confirmed_usance_lc();
        let result = price_letter_of_credit(&input).unwrap();
        let out = &result.result;

        // Issuance fee = 2_000_000 * (150/10000) * (365/360)
        //   = 2_000_000 * 0.015 * 1.01388...
        //   = 30_000 * (365/360)
        //   = 30_416.6666...
        let expected_issuance =
            dec!(2_000_000) * (dec!(150) / dec!(10000)) * (dec!(365) / dec!(360));
        let diff = (out.issuance_fee - expected_issuance).abs();
        assert!(
            diff < dec!(0.01),
            "Issuance fee expected ~{}, got {}",
            expected_issuance,
            out.issuance_fee
        );

        // Confirmation fee = 2_000_000 * (75/10000) * (365/360)
        let expected_conf = dec!(2_000_000) * (dec!(75) / dec!(10000)) * (dec!(365) / dec!(360));
        let conf = out.confirmation_fee.unwrap();
        let diff = (conf - expected_conf).abs();
        assert!(
            diff < dec!(0.01),
            "Confirmation fee expected ~{}, got {}",
            expected_conf,
            conf
        );
    }

    // -------------------------------------------------------------------
    // 3. BA discount proceeds calculation
    // -------------------------------------------------------------------
    #[test]
    fn test_ba_discount_proceeds() {
        let input = confirmed_usance_lc();
        let result = price_letter_of_credit(&input).unwrap();
        let out = &result.result;

        // Discount = 2_000_000 * 0.05 * (90/360) = 2_000_000 * 0.0125 = 25_000
        // Proceeds = 2_000_000 - 25_000 = 1_975_000
        let expected_proceeds = dec!(1_975_000);
        let proceeds = out.discount_proceeds.unwrap();
        assert_eq!(
            proceeds, expected_proceeds,
            "Discount proceeds expected {}, got {}",
            expected_proceeds, proceeds
        );
    }

    // -------------------------------------------------------------------
    // 4. All-in cost annualization
    // -------------------------------------------------------------------
    #[test]
    fn test_all_in_cost_annualization() {
        let input = sight_commercial_lc();
        let result = price_letter_of_credit(&input).unwrap();
        let out = &result.result;

        // Total fees = 8250 (from test 1)
        // All-in = (8250 / 1_000_000) * (360 / 180)
        //        = 0.00825 * 2 = 0.0165 = 1.65%
        let expected = dec!(0.0165);
        let diff = (out.all_in_cost_pct - expected).abs();
        assert!(
            diff < dec!(0.0001),
            "All-in cost expected ~{}, got {}",
            expected,
            out.all_in_cost_pct
        );
    }

    // -------------------------------------------------------------------
    // 5. Risk scoring: high-risk country
    // -------------------------------------------------------------------
    #[test]
    fn test_risk_scoring_high_risk_country() {
        let mut input = sight_commercial_lc();
        input.applicant_country = "IR".into(); // Iran -- sanctioned
        input.beneficiary_country = "DE".into();

        let result = price_letter_of_credit(&input).unwrap();
        let risk = &result.result.risk_score;

        // Country risk for IR = 9
        assert_eq!(risk.country_risk, dec!(9));
        // Overall should be elevated
        assert!(
            risk.overall >= dec!(4),
            "Overall risk should be >= 4 for sanctioned-country LC, got {}",
            risk.overall
        );
    }

    // -------------------------------------------------------------------
    // 6. Risk scoring: low-risk sovereign (developed countries)
    // -------------------------------------------------------------------
    #[test]
    fn test_risk_scoring_low_risk_sovereign() {
        let mut input = sight_commercial_lc();
        input.applicant_country = "US".into();
        input.beneficiary_country = "GB".into();
        input.issuing_bank_rating = "AAA".into();

        let result = price_letter_of_credit(&input).unwrap();
        let risk = &result.result.risk_score;

        // Country risk = 1 (US/GB both tier 1)
        assert_eq!(risk.country_risk, dec!(1));
        // Bank risk = 1 (AAA)
        assert_eq!(risk.bank_risk, dec!(1));
        // Overall should be low
        assert!(
            risk.overall <= dec!(3),
            "Overall risk for US/GB AAA should be <= 3, got {}",
            risk.overall
        );
    }

    // -------------------------------------------------------------------
    // 7. Revolving LC with multiple drawings
    // -------------------------------------------------------------------
    #[test]
    fn test_revolving_lc_multiple_drawings() {
        let mut input = sight_commercial_lc();
        input.lc_type = LcType::Revolving;
        input.drawing_count = 4;
        input.negotiation_fee_bps = Some(dec!(25));

        let result = price_letter_of_credit(&input).unwrap();
        let out = &result.result;

        // Negotiation fee = 1_000_000 * (25/10000) * 4 = 2500 * 4 = 10_000
        assert_eq!(out.negotiation_fee, Some(dec!(10000)));
        assert_eq!(out.lc_type, "Revolving");

        // Total = issuance(5000) + advising(500) + negotiation(10000) + amendment(250) = 15750
        assert_eq!(out.total_fees, dec!(15750));
    }

    // -------------------------------------------------------------------
    // 8. Standby LC commitment fee
    // -------------------------------------------------------------------
    #[test]
    fn test_standby_lc_commitment_fee() {
        let input = LetterOfCreditInput {
            lc_type: LcType::Standby,
            face_amount: dec!(5_000_000),
            currency: "USD".into(),
            tenor_days: 365,
            issuing_bank_rating: "AA".into(),
            confirming_bank_rating: None,
            applicant_country: "US".into(),
            beneficiary_country: "US".into(),
            issuance_fee_bps: dec!(50), // standby typically lower issuance
            confirmation_fee_bps: None,
            advising_fee_flat: Some(dec!(1000)),
            negotiation_fee_bps: None,
            amendment_fee_flat: None,
            drawing_count: 0,
            is_confirmed: false,
            is_at_sight: true,
            deferred_payment_days: None,
            discount_rate: None,
            goods_description: None,
        };

        let result = price_letter_of_credit(&input).unwrap();
        let out = &result.result;

        // Issuance fee = 5_000_000 * (50/10000) * (365/360)
        //   = 5_000_000 * 0.005 * 1.01388...
        //   = 25_000 * (365/360)
        let expected_issuance =
            dec!(5_000_000) * (dec!(50) / dec!(10000)) * (dec!(365) / dec!(360));
        let diff = (out.issuance_fee - expected_issuance).abs();
        assert!(
            diff < dec!(0.01),
            "Standby issuance fee expected ~{}, got {}",
            expected_issuance,
            out.issuance_fee
        );

        assert_eq!(out.lc_type, "Standby");
        assert!(out.confirmation_fee.is_none());
        assert!(out.discount_proceeds.is_none());
    }

    // -------------------------------------------------------------------
    // 9. Discount yield calculation
    // -------------------------------------------------------------------
    #[test]
    fn test_discount_yield() {
        let input = confirmed_usance_lc();
        let result = price_letter_of_credit(&input).unwrap();
        let out = &result.result;

        // Proceeds = 1_975_000 (from test 3)
        // Yield = (2_000_000 / 1_975_000 - 1) * (360 / 90)
        //       = (1.01265... - 1) * 4
        //       = 0.01265... * 4 = 0.05063...
        let expected_yield =
            (dec!(2_000_000) / dec!(1_975_000) - Decimal::ONE) * (dec!(360) / dec!(90));
        let dy = out.discount_yield.unwrap();
        let diff = (dy - expected_yield).abs();
        assert!(
            diff < dec!(0.0001),
            "Discount yield expected ~{}, got {}",
            expected_yield,
            dy
        );
    }

    // -------------------------------------------------------------------
    // 10. Effective financing cost (includes LC fees + discount)
    // -------------------------------------------------------------------
    #[test]
    fn test_effective_financing_cost() {
        let input = confirmed_usance_lc();
        let result = price_letter_of_credit(&input).unwrap();
        let out = &result.result;

        // Effective = all_in_cost_pct + discount_yield
        let efc = out.effective_financing_cost.unwrap();
        let expected = out.all_in_cost_pct + out.discount_yield.unwrap();
        let diff = (efc - expected).abs();
        assert!(
            diff < dec!(0.0001),
            "Effective financing cost expected {}, got {}",
            expected,
            efc
        );
        // Should be greater than either component alone
        assert!(efc > out.all_in_cost_pct);
    }

    // -------------------------------------------------------------------
    // 11. Sight LC has no discount
    // -------------------------------------------------------------------
    #[test]
    fn test_sight_lc_no_discount() {
        let input = sight_commercial_lc();
        let result = price_letter_of_credit(&input).unwrap();
        let out = &result.result;

        assert!(out.discount_proceeds.is_none());
        assert!(out.discount_yield.is_none());
        assert!(out.effective_financing_cost.is_none());
    }

    // -------------------------------------------------------------------
    // 12. Validation: zero face amount rejected
    // -------------------------------------------------------------------
    #[test]
    fn test_invalid_zero_face_amount() {
        let mut input = sight_commercial_lc();
        input.face_amount = Decimal::ZERO;

        let result = price_letter_of_credit(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "face_amount");
            }
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }

    // -------------------------------------------------------------------
    // 13. Validation: zero tenor rejected
    // -------------------------------------------------------------------
    #[test]
    fn test_invalid_zero_tenor() {
        let mut input = sight_commercial_lc();
        input.tenor_days = 0;

        let result = price_letter_of_credit(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "tenor_days");
            }
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }

    // -------------------------------------------------------------------
    // 14. Validation: invalid bank rating rejected
    // -------------------------------------------------------------------
    #[test]
    fn test_invalid_bank_rating() {
        let mut input = sight_commercial_lc();
        input.issuing_bank_rating = "AAAA".into();

        let result = price_letter_of_credit(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "issuing_bank_rating");
            }
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }

    // -------------------------------------------------------------------
    // 15. Validation: usance LC without deferred days rejected
    // -------------------------------------------------------------------
    #[test]
    fn test_invalid_usance_no_deferred_days() {
        let mut input = sight_commercial_lc();
        input.is_at_sight = false;
        input.deferred_payment_days = None;

        let result = price_letter_of_credit(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "deferred_payment_days");
            }
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }

    // -------------------------------------------------------------------
    // 16. Tenor risk scoring boundaries
    // -------------------------------------------------------------------
    #[test]
    fn test_tenor_risk_boundaries() {
        // < 90 days
        assert_eq!(tenor_risk_score(30), dec!(2));
        assert_eq!(tenor_risk_score(89), dec!(2));
        // 90-180 days
        assert_eq!(tenor_risk_score(90), dec!(4));
        assert_eq!(tenor_risk_score(180), dec!(4));
        // 181-360 days
        assert_eq!(tenor_risk_score(181), dec!(6));
        assert_eq!(tenor_risk_score(360), dec!(6));
        // > 360 days
        assert_eq!(tenor_risk_score(361), dec!(8));
        assert_eq!(tenor_risk_score(720), dec!(8));
    }

    // -------------------------------------------------------------------
    // 17. Bank risk scoring
    // -------------------------------------------------------------------
    #[test]
    fn test_bank_risk_scoring() {
        assert_eq!(bank_risk_score("AAA"), dec!(1));
        assert_eq!(bank_risk_score("AA"), dec!(2));
        assert_eq!(bank_risk_score("A"), dec!(3));
        assert_eq!(bank_risk_score("BBB"), dec!(4));
        assert_eq!(bank_risk_score("BB"), dec!(6));
        assert_eq!(bank_risk_score("B"), dec!(8));
        assert_eq!(bank_risk_score("CCC"), dec!(10));
    }

    // -------------------------------------------------------------------
    // 18. Overall risk weighted average
    // -------------------------------------------------------------------
    #[test]
    fn test_overall_risk_weighted_average() {
        let input = sight_commercial_lc();
        let result = price_letter_of_credit(&input).unwrap();
        let risk = &result.result.risk_score;

        // US/DE = country 1, A bank = 3, 180d = 4, sight = 2
        // overall = 1*0.3 + 3*0.3 + 4*0.2 + 2*0.2
        //         = 0.3 + 0.9 + 0.8 + 0.4 = 2.4
        let expected = dec!(1) * dec!(0.30)
            + dec!(3) * dec!(0.30)
            + dec!(4) * dec!(0.20)
            + dec!(2) * dec!(0.20);
        assert_eq!(
            risk.overall, expected,
            "Overall risk expected {}, got {}",
            expected, risk.overall
        );
    }

    // -------------------------------------------------------------------
    // 19. Back-to-back LC type display
    // -------------------------------------------------------------------
    #[test]
    fn test_back_to_back_lc_type() {
        let mut input = sight_commercial_lc();
        input.lc_type = LcType::BackToBack;

        let result = price_letter_of_credit(&input).unwrap();
        assert_eq!(result.result.lc_type, "Back-to-Back");
    }

    // -------------------------------------------------------------------
    // 20. Transferable LC type display
    // -------------------------------------------------------------------
    #[test]
    fn test_transferable_lc_type() {
        let mut input = sight_commercial_lc();
        input.lc_type = LcType::Transferable;

        let result = price_letter_of_credit(&input).unwrap();
        assert_eq!(result.result.lc_type, "Transferable");
    }

    // -------------------------------------------------------------------
    // 21. Confirmed LC without confirmation bps triggers warning
    // -------------------------------------------------------------------
    #[test]
    fn test_confirmed_lc_no_bps_warns() {
        let mut input = sight_commercial_lc();
        input.is_confirmed = true;
        input.confirmation_fee_bps = None;

        let result = price_letter_of_credit(&input).unwrap();
        let out = &result.result;

        // Confirmation fee should be Some(0)
        assert_eq!(out.confirmation_fee, Some(Decimal::ZERO));
        // Should have warning about missing bps
        assert!(
            result
                .warnings
                .iter()
                .any(|w| w.contains("confirmation_fee_bps")),
            "Expected warning about missing confirmation fee bps"
        );
    }

    // -------------------------------------------------------------------
    // 22. Metadata populated correctly
    // -------------------------------------------------------------------
    #[test]
    fn test_metadata_populated() {
        let input = sight_commercial_lc();
        let result = price_letter_of_credit(&input).unwrap();

        assert!(result.methodology.contains("Letter of Credit"));
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
        assert!(!result.metadata.version.is_empty());
    }

    // -------------------------------------------------------------------
    // 23. Long tenor triggers warning
    // -------------------------------------------------------------------
    #[test]
    fn test_long_tenor_warning() {
        let mut input = sight_commercial_lc();
        input.tenor_days = 540;

        let result = price_letter_of_credit(&input).unwrap();
        assert!(
            result.warnings.iter().any(|w| w.contains("360 days")),
            "Expected warning about tenor exceeding 360 days"
        );
    }

    // -------------------------------------------------------------------
    // 24. No negotiation fee when bps absent
    // -------------------------------------------------------------------
    #[test]
    fn test_no_negotiation_fee() {
        let mut input = sight_commercial_lc();
        input.negotiation_fee_bps = None;

        let result = price_letter_of_credit(&input).unwrap();
        assert!(result.result.negotiation_fee.is_none());
    }

    // -------------------------------------------------------------------
    // 25. Usance LC without discount rate -- no discount output
    // -------------------------------------------------------------------
    #[test]
    fn test_usance_no_discount_rate() {
        let mut input = confirmed_usance_lc();
        input.discount_rate = None;

        let result = price_letter_of_credit(&input).unwrap();
        let out = &result.result;

        assert!(out.discount_proceeds.is_none());
        assert!(out.discount_yield.is_none());
        assert!(out.effective_financing_cost.is_none());
    }
}
