use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{with_metadata, ComputationOutput};
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Convertible Note
// ---------------------------------------------------------------------------

/// What triggers the convertible note conversion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConversionTrigger {
    /// A qualifying equity round meets or exceeds `qualified_financing_amount`.
    QualifiedFinancing,
    /// The note has reached its maturity date.
    Maturity,
    /// The company is being acquired or merged.
    ChangeOfControl,
}

/// Input for convertible-note conversion mechanics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvertibleNoteInput {
    /// Note face value.
    pub principal: Decimal,
    /// Annual interest rate (e.g. 0.05 = 5%).
    pub interest_rate: Decimal,
    /// Note term in months.
    pub term_months: u32,
    /// Months elapsed since issuance (for accrued interest).
    pub elapsed_months: u32,
    /// Conversion discount (e.g. 0.20 = 20%).
    pub discount_rate: Decimal,
    /// Maximum pre-money valuation for conversion (investor-friendly).
    pub valuation_cap: Option<Decimal>,
    /// Size of the qualifying equity round.
    pub qualified_financing_amount: Decimal,
    /// Pre-money valuation of the qualifying round.
    pub qualified_financing_pre_money: Decimal,
    /// Shares outstanding before the round.
    pub pre_money_shares: u64,
    /// What triggers conversion.
    pub conversion_trigger: ConversionTrigger,
}

/// Output of convertible-note conversion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvertibleNoteOutput {
    /// Interest accrued over elapsed_months.
    pub accrued_interest: Decimal,
    /// Principal + accrued interest.
    pub total_conversion_amount: Decimal,
    /// Qualified-round price * (1 - discount).
    pub effective_price_discount: Decimal,
    /// Valuation cap / pre_money_shares (if cap provided).
    pub effective_price_cap: Option<Decimal>,
    /// The conversion price used (most favourable to the note-holder).
    pub conversion_price: Decimal,
    /// Shares issued on conversion (floor).
    pub shares_issued: u64,
    /// conversion_price * pre_money_shares.
    pub effective_valuation: Decimal,
    /// Savings vs converting at the round price.
    pub discount_savings: Decimal,
    /// shares_issued / (pre_money_shares + new_round_shares + shares_issued).
    pub ownership_pct: Decimal,
}

/// Convert a convertible note into equity at the most favourable price for
/// the note-holder, applying discount and/or valuation cap.
pub fn convert_note(
    input: &ConvertibleNoteInput,
) -> CorpFinanceResult<ComputationOutput<ConvertibleNoteOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // -- Validate inputs --
    if input.principal.is_zero() || input.principal.is_sign_negative() {
        return Err(CorpFinanceError::InvalidInput {
            field: "principal".into(),
            reason: "Principal must be positive".into(),
        });
    }
    if input.pre_money_shares == 0 {
        return Err(CorpFinanceError::DivisionByZero {
            context: "pre_money_shares cannot be zero".into(),
        });
    }
    if input.qualified_financing_pre_money.is_zero()
        || input.qualified_financing_pre_money.is_sign_negative()
    {
        return Err(CorpFinanceError::InvalidInput {
            field: "qualified_financing_pre_money".into(),
            reason: "Pre-money valuation of qualifying round must be positive".into(),
        });
    }
    if input.discount_rate < Decimal::ZERO || input.discount_rate >= dec!(1) {
        return Err(CorpFinanceError::InvalidInput {
            field: "discount_rate".into(),
            reason: "Discount rate must be in [0, 1)".into(),
        });
    }
    if input.elapsed_months > input.term_months {
        warnings.push(format!(
            "elapsed_months ({}) exceeds term_months ({}); note may be past maturity",
            input.elapsed_months, input.term_months
        ));
    }

    let shares_dec = Decimal::from(input.pre_money_shares);

    // -- Accrued interest (simple interest) --
    let accrued_interest =
        input.principal * input.interest_rate * Decimal::from(input.elapsed_months) / dec!(12);
    let total_conversion_amount = input.principal + accrued_interest;

    // -- Price at round --
    let price_at_round = input.qualified_financing_pre_money / shares_dec;

    // -- Discount price --
    let effective_price_discount = price_at_round * (dec!(1) - input.discount_rate);

    // -- Cap price --
    let effective_price_cap = input.valuation_cap.map(|cap| cap / shares_dec);

    // -- Conversion price: minimum of applicable prices (most favourable to holder) --
    let conversion_price = match effective_price_cap {
        Some(cap_price) => {
            // Both discount and cap apply; pick the lower (better for investor)
            if cap_price < effective_price_discount {
                cap_price
            } else {
                effective_price_discount
            }
        }
        None => {
            // Only discount applies
            if input.discount_rate > Decimal::ZERO {
                effective_price_discount
            } else {
                // No cap, no discount -> convert at round price
                warnings
                    .push("No valuation cap and zero discount; converting at round price".into());
                price_at_round
            }
        }
    };

    if conversion_price.is_zero() || conversion_price.is_sign_negative() {
        return Err(CorpFinanceError::DivisionByZero {
            context: "conversion_price must be positive".into(),
        });
    }

    // -- Shares issued (floor) --
    let shares_issued_dec = (total_conversion_amount / conversion_price).floor();
    let shares_issued = decimal_to_u64(shares_issued_dec)?;

    // -- Effective valuation --
    let effective_valuation = conversion_price * shares_dec;

    // -- Discount savings --
    let shares_at_round_price = if price_at_round.is_zero() {
        Decimal::ZERO
    } else {
        (total_conversion_amount / price_at_round).floor()
    };
    let discount_savings = (shares_issued_dec - shares_at_round_price) * price_at_round;

    // -- Ownership percentage --
    // New-round shares from the equity financing
    let new_round_shares = if price_at_round.is_zero() {
        Decimal::ZERO
    } else {
        (input.qualified_financing_amount / price_at_round).floor()
    };
    let total_post_shares = shares_dec + new_round_shares + shares_issued_dec;
    let ownership_pct = if total_post_shares.is_zero() {
        Decimal::ZERO
    } else {
        shares_issued_dec / total_post_shares
    };

    let output = ConvertibleNoteOutput {
        accrued_interest,
        total_conversion_amount,
        effective_price_discount,
        effective_price_cap,
        conversion_price,
        shares_issued,
        effective_valuation,
        discount_savings,
        ownership_pct,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Convertible Note Conversion",
        &serde_json::json!({
            "principal": input.principal.to_string(),
            "interest_rate": input.interest_rate.to_string(),
            "discount_rate": input.discount_rate.to_string(),
            "valuation_cap": input.valuation_cap.map(|c| c.to_string()),
            "conversion_trigger": format!("{:?}", input.conversion_trigger),
        }),
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// SAFE (Simple Agreement for Future Equity)
// ---------------------------------------------------------------------------

/// Pre-money vs post-money SAFE type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SafeType {
    /// Traditional pre-money SAFE.
    PreMoney,
    /// YC-style post-money SAFE (cap IS the post-money valuation).
    PostMoney,
}

/// Input for SAFE conversion mechanics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafeInput {
    /// Amount invested via the SAFE.
    pub investment_amount: Decimal,
    /// Pre-money valuation cap (for pre-money SAFE) or post-money cap (for post-money).
    pub valuation_cap: Option<Decimal>,
    /// Discount rate (e.g. 0.20 = 20%).
    pub discount_rate: Option<Decimal>,
    /// Type of SAFE.
    pub safe_type: SafeType,
    /// Pre-money valuation of the qualifying round.
    pub qualified_financing_pre_money: Decimal,
    /// Amount of the qualifying round.
    pub qualified_financing_amount: Decimal,
    /// Shares outstanding before the SAFE conversion (company capitalisation).
    pub pre_money_shares: u64,
    /// Most Favoured Nation provision.
    pub mfn: bool,
}

/// Output of SAFE conversion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafeOutput {
    /// Price per share used for conversion.
    pub conversion_price: Decimal,
    /// Shares issued on conversion (floor).
    pub shares_issued: u64,
    /// Effective pre-money valuation implied by conversion price.
    pub effective_valuation: Decimal,
    /// Ownership percentage after conversion.
    pub ownership_pct: Decimal,
    /// Price derived from cap (if applicable).
    pub price_via_cap: Option<Decimal>,
    /// Price derived from discount (if applicable).
    pub price_via_discount: Option<Decimal>,
    /// Which method drove the conversion price.
    pub method_used: String,
}

/// Convert a SAFE into equity at a qualifying financing event.
pub fn convert_safe(input: &SafeInput) -> CorpFinanceResult<ComputationOutput<SafeOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // -- Validate --
    if input.investment_amount.is_zero() || input.investment_amount.is_sign_negative() {
        return Err(CorpFinanceError::InvalidInput {
            field: "investment_amount".into(),
            reason: "Investment amount must be positive".into(),
        });
    }
    if input.pre_money_shares == 0 {
        return Err(CorpFinanceError::DivisionByZero {
            context: "pre_money_shares cannot be zero".into(),
        });
    }
    if input.qualified_financing_pre_money.is_zero()
        || input.qualified_financing_pre_money.is_sign_negative()
    {
        return Err(CorpFinanceError::InvalidInput {
            field: "qualified_financing_pre_money".into(),
            reason: "Pre-money valuation of qualifying round must be positive".into(),
        });
    }
    if let Some(dr) = input.discount_rate {
        if dr < Decimal::ZERO || dr >= dec!(1) {
            return Err(CorpFinanceError::InvalidInput {
                field: "discount_rate".into(),
                reason: "Discount rate must be in [0, 1)".into(),
            });
        }
    }

    let shares_dec = Decimal::from(input.pre_money_shares);

    match input.safe_type {
        SafeType::PreMoney => convert_safe_pre_money(input, shares_dec, &mut warnings, start),
        SafeType::PostMoney => convert_safe_post_money(input, shares_dec, &mut warnings, start),
    }
}

/// Pre-money SAFE conversion: works like a convertible note without interest.
fn convert_safe_pre_money(
    input: &SafeInput,
    shares_dec: Decimal,
    warnings: &mut Vec<String>,
    start: Instant,
) -> CorpFinanceResult<ComputationOutput<SafeOutput>> {
    let price_at_round = input.qualified_financing_pre_money / shares_dec;

    // Price via cap
    let price_via_cap = input.valuation_cap.map(|cap| cap / shares_dec);

    // Price via discount
    let price_via_discount = input
        .discount_rate
        .map(|dr| price_at_round * (dec!(1) - dr));

    // Determine conversion price and method
    let (conversion_price, method_used) = match (price_via_cap, price_via_discount) {
        (Some(cap_p), Some(disc_p)) => {
            if cap_p <= disc_p {
                (cap_p, "cap")
            } else {
                (disc_p, "discount")
            }
        }
        (Some(cap_p), None) => (cap_p, "cap"),
        (None, Some(disc_p)) => (disc_p, "discount"),
        (None, None) => {
            warnings.push("No valuation cap or discount; converting at round price".into());
            (price_at_round, "round_price")
        }
    };

    if conversion_price.is_zero() || conversion_price.is_sign_negative() {
        return Err(CorpFinanceError::DivisionByZero {
            context: "conversion_price must be positive".into(),
        });
    }

    let shares_issued_dec = (input.investment_amount / conversion_price).floor();
    let shares_issued = decimal_to_u64(shares_issued_dec)?;

    let effective_valuation = conversion_price * shares_dec;

    // Ownership: shares_issued / (pre_money_shares + new_round_shares + shares_issued)
    let new_round_shares = if price_at_round.is_zero() {
        Decimal::ZERO
    } else {
        (input.qualified_financing_amount / price_at_round).floor()
    };
    let total_post = shares_dec + new_round_shares + shares_issued_dec;
    let ownership_pct = if total_post.is_zero() {
        Decimal::ZERO
    } else {
        shares_issued_dec / total_post
    };

    if input.mfn {
        warnings
            .push("MFN provision active: holder may elect better terms from later SAFEs".into());
    }

    let output = SafeOutput {
        conversion_price,
        shares_issued,
        effective_valuation,
        ownership_pct,
        price_via_cap,
        price_via_discount,
        method_used: method_used.to_string(),
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "SAFE Conversion (Pre-Money)",
        &serde_json::json!({
            "investment_amount": input.investment_amount.to_string(),
            "valuation_cap": input.valuation_cap.map(|c| c.to_string()),
            "discount_rate": input.discount_rate.map(|d| d.to_string()),
            "mfn": input.mfn,
        }),
        warnings.clone(),
        elapsed,
        output,
    ))
}

/// Post-money SAFE conversion (YC standard):
/// ownership_pct = investment_amount / valuation_cap, calculated before the new round.
fn convert_safe_post_money(
    input: &SafeInput,
    shares_dec: Decimal,
    warnings: &mut Vec<String>,
    start: Instant,
) -> CorpFinanceResult<ComputationOutput<SafeOutput>> {
    let price_at_round = input.qualified_financing_pre_money / shares_dec;

    // Post-money cap: the cap IS the post-money valuation
    let (conversion_price, method_used, price_via_cap, price_via_discount) = if let Some(cap) =
        input.valuation_cap
    {
        if cap.is_zero() || cap.is_sign_negative() {
            return Err(CorpFinanceError::InvalidInput {
                field: "valuation_cap".into(),
                reason: "Post-money cap must be positive".into(),
            });
        }
        // ownership = investment / cap (post-money)
        // shares = pre_money_shares * ownership / (1 - ownership)
        // Effective per-share price via cap:
        //   price_via_cap = cap / (pre_money_shares + shares_from_safe)
        //   But it is simpler to derive from ownership.
        // For reporting, the effective price_via_cap is analogous to pre-money cap / shares:
        //   We use the implicit pre-money implied by the post-money cap.
        let implied_pre_money = cap - input.investment_amount;
        let pvc = if shares_dec.is_zero() {
            Decimal::ZERO
        } else {
            implied_pre_money / shares_dec
        };

        // Also compute discount price if discount_rate is provided
        let pvd = input
            .discount_rate
            .map(|dr| price_at_round * (dec!(1) - dr));

        // For post-money SAFEs, the cap-based approach is the standard method.
        // If a discount is also provided, use whichever is lower.
        let (cp, method) = match pvd {
            Some(disc_p) if disc_p < pvc => (disc_p, "discount"),
            _ => (pvc, "cap"),
        };

        (cp, method, Some(pvc), pvd)
    } else if let Some(dr) = input.discount_rate {
        // Post-money SAFE with only a discount (unusual but possible)
        let disc_p = price_at_round * (dec!(1) - dr);
        (disc_p, "discount", None, Some(disc_p))
    } else {
        warnings.push("Post-money SAFE without cap or discount; converting at round price".into());
        (price_at_round, "round_price", None, None)
    };

    if conversion_price.is_zero() || conversion_price.is_sign_negative() {
        return Err(CorpFinanceError::DivisionByZero {
            context: "conversion_price must be positive".into(),
        });
    }

    // For post-money SAFEs with a cap, ownership is deterministic:
    //   ownership = investment / cap
    //   shares = pre_money_shares * ownership / (1 - ownership)
    let (shares_issued, ownership_pct) =
        if let (Some(cap), true) = (input.valuation_cap, method_used == "cap") {
            let ownership = input.investment_amount / cap;
            let denom = dec!(1) - ownership;
            if denom.is_zero() || denom.is_sign_negative() {
                return Err(CorpFinanceError::FinancialImpossibility(
                    "Investment equals or exceeds post-money cap".into(),
                ));
            }
            let shares_dec_issued = (shares_dec * ownership / denom).floor();
            let si = decimal_to_u64(shares_dec_issued)?;

            // Ownership of total company (including new round shares)
            let new_round_shares = if price_at_round.is_zero() {
                Decimal::ZERO
            } else {
                (input.qualified_financing_amount / price_at_round).floor()
            };
            let total_post = shares_dec + new_round_shares + shares_dec_issued;
            let own = if total_post.is_zero() {
                Decimal::ZERO
            } else {
                shares_dec_issued / total_post
            };
            (si, own)
        } else {
            // Discount-only or round-price path
            let shares_issued_dec = (input.investment_amount / conversion_price).floor();
            let si = decimal_to_u64(shares_issued_dec)?;

            let new_round_shares = if price_at_round.is_zero() {
                Decimal::ZERO
            } else {
                (input.qualified_financing_amount / price_at_round).floor()
            };
            let total_post = shares_dec + new_round_shares + shares_issued_dec;
            let own = if total_post.is_zero() {
                Decimal::ZERO
            } else {
                shares_issued_dec / total_post
            };
            (si, own)
        };

    let effective_valuation = conversion_price * shares_dec;

    if input.mfn {
        warnings
            .push("MFN provision active: holder may elect better terms from later SAFEs".into());
    }

    let output = SafeOutput {
        conversion_price,
        shares_issued,
        effective_valuation,
        ownership_pct,
        price_via_cap,
        price_via_discount,
        method_used: method_used.to_string(),
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "SAFE Conversion (Post-Money / YC Standard)",
        &serde_json::json!({
            "investment_amount": input.investment_amount.to_string(),
            "valuation_cap": input.valuation_cap.map(|c| c.to_string()),
            "discount_rate": input.discount_rate.map(|d| d.to_string()),
            "safe_type": "PostMoney",
            "mfn": input.mfn,
        }),
        warnings.clone(),
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Safely convert a non-negative Decimal to u64.
fn decimal_to_u64(val: Decimal) -> CorpFinanceResult<u64> {
    if val.is_sign_negative() {
        return Err(CorpFinanceError::FinancialImpossibility(
            "Cannot convert negative value to share count".into(),
        ));
    }
    let truncated = val.floor();
    // rust_decimal can represent values up to ~79 digits; u64 max is ~1.8e19.
    // For share counts this is always safe.
    truncated
        .to_string()
        .parse::<u64>()
        .map_err(|e| CorpFinanceError::FinancialImpossibility(format!("Share count overflow: {e}")))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    // -- Helper to build a basic convertible note input --
    fn base_note_input() -> ConvertibleNoteInput {
        ConvertibleNoteInput {
            principal: dec!(100_000),
            interest_rate: dec!(0.05),
            term_months: 24,
            elapsed_months: 12,
            discount_rate: dec!(0.20),
            valuation_cap: Some(dec!(5_000_000)),
            qualified_financing_amount: dec!(2_000_000),
            qualified_financing_pre_money: dec!(8_000_000),
            pre_money_shares: 10_000_000,
            conversion_trigger: ConversionTrigger::QualifiedFinancing,
        }
    }

    // -- Helper to build a basic pre-money SAFE input --
    fn base_safe_pre_money() -> SafeInput {
        SafeInput {
            investment_amount: dec!(500_000),
            valuation_cap: Some(dec!(5_000_000)),
            discount_rate: Some(dec!(0.20)),
            safe_type: SafeType::PreMoney,
            qualified_financing_pre_money: dec!(10_000_000),
            qualified_financing_amount: dec!(3_000_000),
            pre_money_shares: 10_000_000,
            mfn: false,
        }
    }

    // -- Helper to build a basic post-money SAFE input --
    fn base_safe_post_money() -> SafeInput {
        SafeInput {
            investment_amount: dec!(500_000),
            valuation_cap: Some(dec!(10_000_000)),
            discount_rate: None,
            safe_type: SafeType::PostMoney,
            qualified_financing_pre_money: dec!(15_000_000),
            qualified_financing_amount: dec!(5_000_000),
            pre_money_shares: 10_000_000,
            mfn: false,
        }
    }

    // -----------------------------------------------------------------------
    // Convertible Note Tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_note_discount_only() {
        let mut input = base_note_input();
        input.valuation_cap = None; // no cap
        input.discount_rate = dec!(0.20);
        input.elapsed_months = 0; // no accrued interest

        let result = convert_note(&input).unwrap();
        let out = &result.result;

        // price_at_round = 8_000_000 / 10_000_000 = 0.80
        // discount_price = 0.80 * 0.80 = 0.64
        assert_eq!(out.conversion_price, dec!(0.64));
        assert_eq!(out.effective_price_cap, None);
        assert_eq!(out.accrued_interest, Decimal::ZERO);
        // shares = 100_000 / 0.64 = 156_250
        assert_eq!(out.shares_issued, 156_250);
    }

    #[test]
    fn test_note_cap_only() {
        let mut input = base_note_input();
        input.discount_rate = dec!(0.0); // no discount
        input.valuation_cap = Some(dec!(4_000_000));
        input.elapsed_months = 0;

        let result = convert_note(&input).unwrap();
        let out = &result.result;

        // cap_price = 4_000_000 / 10_000_000 = 0.40
        // discount_price = 0.80 * 1.0 = 0.80  (no discount)
        // conversion at cap price since it is lower
        assert_eq!(out.conversion_price, dec!(0.40));
        assert_eq!(out.effective_price_cap, Some(dec!(0.40)));
        // shares = 100_000 / 0.40 = 250_000
        assert_eq!(out.shares_issued, 250_000);
    }

    #[test]
    fn test_note_cap_and_discount_cap_wins() {
        let mut input = base_note_input();
        input.discount_rate = dec!(0.20);
        input.valuation_cap = Some(dec!(4_000_000));
        input.elapsed_months = 0;

        let result = convert_note(&input).unwrap();
        let out = &result.result;

        // cap_price = 4_000_000 / 10_000_000 = 0.40
        // discount_price = 0.80 * 0.80 = 0.64
        // cap wins (lower)
        assert_eq!(out.conversion_price, dec!(0.40));
    }

    #[test]
    fn test_note_cap_and_discount_discount_wins() {
        let mut input = base_note_input();
        input.discount_rate = dec!(0.20);
        input.valuation_cap = Some(dec!(7_000_000));
        input.elapsed_months = 0;

        let result = convert_note(&input).unwrap();
        let out = &result.result;

        // cap_price = 7_000_000 / 10_000_000 = 0.70
        // discount_price = 0.80 * 0.80 = 0.64
        // discount wins (lower)
        assert_eq!(out.conversion_price, dec!(0.64));
    }

    #[test]
    fn test_note_accrued_interest() {
        let mut input = base_note_input();
        input.elapsed_months = 18;

        let result = convert_note(&input).unwrap();
        let out = &result.result;

        // accrued = 100_000 * 0.05 * 18 / 12 = 7_500
        assert_eq!(out.accrued_interest, dec!(7500));
        assert_eq!(out.total_conversion_amount, dec!(107500));
    }

    #[test]
    fn test_note_maturity_conversion() {
        let mut input = base_note_input();
        input.conversion_trigger = ConversionTrigger::Maturity;
        input.elapsed_months = 24;

        let result = convert_note(&input).unwrap();
        let out = &result.result;

        // accrued = 100_000 * 0.05 * 24 / 12 = 10_000
        assert_eq!(out.accrued_interest, dec!(10000));
        assert_eq!(out.total_conversion_amount, dec!(110000));
    }

    #[test]
    fn test_note_change_of_control() {
        let mut input = base_note_input();
        input.conversion_trigger = ConversionTrigger::ChangeOfControl;

        let result = convert_note(&input).unwrap();
        // Should still compute correctly regardless of trigger type
        assert!(result.result.shares_issued > 0);
    }

    #[test]
    fn test_note_zero_principal_error() {
        let mut input = base_note_input();
        input.principal = Decimal::ZERO;
        assert!(convert_note(&input).is_err());
    }

    #[test]
    fn test_note_zero_shares_error() {
        let mut input = base_note_input();
        input.pre_money_shares = 0;
        assert!(convert_note(&input).is_err());
    }

    #[test]
    fn test_note_discount_savings() {
        let mut input = base_note_input();
        input.valuation_cap = None;
        input.discount_rate = dec!(0.20);
        input.elapsed_months = 0;

        let result = convert_note(&input).unwrap();
        let out = &result.result;

        // At round price: 100_000 / 0.80 = 125_000 shares
        // At discount:    100_000 / 0.64 = 156_250 shares
        // Savings = (156_250 - 125_000) * 0.80 = 25_000
        assert_eq!(out.discount_savings, dec!(25000.00));
    }

    // -----------------------------------------------------------------------
    // SAFE Pre-Money Tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_safe_pre_money_cap_only() {
        let mut input = base_safe_pre_money();
        input.discount_rate = None; // no discount

        let result = convert_safe(&input).unwrap();
        let out = &result.result;

        // price_at_round = 10_000_000 / 10_000_000 = 1.00
        // price_via_cap = 5_000_000 / 10_000_000 = 0.50
        assert_eq!(out.price_via_cap, Some(dec!(0.50)));
        assert_eq!(out.price_via_discount, None);
        assert_eq!(out.conversion_price, dec!(0.50));
        assert_eq!(out.method_used, "cap");
        // shares = 500_000 / 0.50 = 1_000_000
        assert_eq!(out.shares_issued, 1_000_000);
    }

    #[test]
    fn test_safe_pre_money_discount_only() {
        let mut input = base_safe_pre_money();
        input.valuation_cap = None;

        let result = convert_safe(&input).unwrap();
        let out = &result.result;

        // price_at_round = 1.00
        // price_via_discount = 1.00 * 0.80 = 0.80
        assert_eq!(out.price_via_discount, Some(dec!(0.80)));
        assert_eq!(out.price_via_cap, None);
        assert_eq!(out.conversion_price, dec!(0.80));
        assert_eq!(out.method_used, "discount");
        // shares = 500_000 / 0.80 = 625_000
        assert_eq!(out.shares_issued, 625_000);
    }

    #[test]
    fn test_safe_pre_money_cap_and_discount() {
        let input = base_safe_pre_money();

        let result = convert_safe(&input).unwrap();
        let out = &result.result;

        // price_via_cap = 0.50
        // price_via_discount = 0.80
        // cap wins (lower price, better for investor)
        assert_eq!(out.conversion_price, dec!(0.50));
        assert_eq!(out.method_used, "cap");
    }

    #[test]
    fn test_safe_pre_money_discount_wins_over_cap() {
        let mut input = base_safe_pre_money();
        input.valuation_cap = Some(dec!(9_000_000)); // cap price = 0.90
        input.discount_rate = Some(dec!(0.20)); // discount price = 0.80

        let result = convert_safe(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.conversion_price, dec!(0.80));
        assert_eq!(out.method_used, "discount");
    }

    // -----------------------------------------------------------------------
    // SAFE Post-Money Tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_safe_post_money_standard_yc() {
        let input = base_safe_post_money();

        let result = convert_safe(&input).unwrap();
        let out = &result.result;

        // Post-money cap = 10_000_000
        // Ownership (pre-round) = 500_000 / 10_000_000 = 5%
        // shares = 10_000_000 * 0.05 / 0.95 = 526_315 (floor)
        assert_eq!(out.method_used, "cap");
        assert_eq!(out.shares_issued, 526_315);
    }

    #[test]
    fn test_safe_post_money_ownership_equals_investment_over_cap() {
        let mut input = base_safe_post_money();
        input.investment_amount = dec!(1_000_000);
        input.valuation_cap = Some(dec!(10_000_000));

        let result = convert_safe(&input).unwrap();
        let out = &result.result;

        // ownership (pre-round) = 1_000_000 / 10_000_000 = 10%
        // shares = 10_000_000 * 0.10 / 0.90 = 1_111_111 (floor)
        assert_eq!(out.shares_issued, 1_111_111);
    }

    #[test]
    fn test_safe_post_money_small_investment() {
        let mut input = base_safe_post_money();
        input.investment_amount = dec!(100_000);
        input.valuation_cap = Some(dec!(10_000_000));

        let result = convert_safe(&input).unwrap();
        let out = &result.result;

        // ownership = 100_000 / 10_000_000 = 1%
        // shares = 10_000_000 * 0.01 / 0.99 = 101_010 (floor)
        assert_eq!(out.shares_issued, 101_010);
    }

    // -----------------------------------------------------------------------
    // Edge Cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_cap_exactly_equals_round_valuation() {
        let mut input = base_note_input();
        input.valuation_cap = Some(dec!(8_000_000)); // same as pre-money
        input.discount_rate = dec!(0.20);
        input.elapsed_months = 0;

        let result = convert_note(&input).unwrap();
        let out = &result.result;

        // cap_price = 8_000_000 / 10_000_000 = 0.80
        // discount_price = 0.80 * 0.80 = 0.64
        // discount wins
        assert_eq!(out.conversion_price, dec!(0.64));
    }

    #[test]
    fn test_no_cap_no_discount_converts_at_round_price() {
        let mut input = base_safe_pre_money();
        input.valuation_cap = None;
        input.discount_rate = None;

        let result = convert_safe(&input).unwrap();
        let out = &result.result;

        // price = 10_000_000 / 10_000_000 = 1.00
        assert_eq!(out.conversion_price, dec!(1.00));
        assert_eq!(out.method_used, "round_price");
        // shares = 500_000 / 1.00 = 500_000
        assert_eq!(out.shares_issued, 500_000);
    }

    #[test]
    fn test_note_no_cap_no_discount_at_round_price() {
        let mut input = base_note_input();
        input.valuation_cap = None;
        input.discount_rate = dec!(0.0);
        input.elapsed_months = 0;

        let result = convert_note(&input).unwrap();
        let out = &result.result;

        // price = 0.80 (round price)
        assert_eq!(out.conversion_price, dec!(0.80));
        // shares = 100_000 / 0.80 = 125_000
        assert_eq!(out.shares_issued, 125_000);
    }

    #[test]
    fn test_ownership_percentage_accuracy() {
        let mut input = base_note_input();
        input.valuation_cap = None;
        input.discount_rate = dec!(0.20);
        input.elapsed_months = 0;

        let result = convert_note(&input).unwrap();
        let out = &result.result;

        // shares_issued = 156_250
        // new_round_shares = 2_000_000 / 0.80 = 2_500_000
        // total = 10_000_000 + 2_500_000 + 156_250 = 12_656_250
        // ownership = 156_250 / 12_656_250
        let expected_ownership = dec!(156250) / dec!(12656250);
        let diff = (out.ownership_pct - expected_ownership).abs();
        assert!(diff < dec!(0.000001), "ownership diff: {diff}");
    }

    #[test]
    fn test_multiple_safes_independent() {
        // Two identical SAFE conversions should produce identical results
        let input = base_safe_pre_money();

        let result1 = convert_safe(&input).unwrap();
        let result2 = convert_safe(&input).unwrap();

        assert_eq!(result1.result.shares_issued, result2.result.shares_issued);
        assert_eq!(
            result1.result.conversion_price,
            result2.result.conversion_price
        );
        assert_eq!(result1.result.ownership_pct, result2.result.ownership_pct);
    }

    #[test]
    fn test_safe_mfn_flag_warning() {
        let mut input = base_safe_pre_money();
        input.mfn = true;

        let result = convert_safe(&input).unwrap();
        assert!(result.warnings.iter().any(|w| w.contains("MFN provision")));
    }

    #[test]
    fn test_note_elapsed_exceeds_term_warning() {
        let mut input = base_note_input();
        input.elapsed_months = 30;
        input.term_months = 24;

        let result = convert_note(&input).unwrap();
        assert!(result.warnings.iter().any(|w| w.contains("past maturity")));
    }

    #[test]
    fn test_safe_invalid_investment_amount() {
        let mut input = base_safe_pre_money();
        input.investment_amount = dec!(-100);
        assert!(convert_safe(&input).is_err());
    }

    #[test]
    fn test_note_invalid_discount_rate() {
        let mut input = base_note_input();
        input.discount_rate = dec!(1.5);
        assert!(convert_note(&input).is_err());
    }

    #[test]
    fn test_safe_post_money_effective_valuation() {
        let input = base_safe_post_money();

        let result = convert_safe(&input).unwrap();
        let out = &result.result;

        // price_via_cap = (10_000_000 - 500_000) / 10_000_000 = 0.95
        // effective_valuation = 0.95 * 10_000_000 = 9_500_000
        assert_eq!(out.effective_valuation, dec!(9_500_000));
        assert_eq!(out.price_via_cap, Some(dec!(0.95)));
    }

    #[test]
    fn test_note_large_accrued_interest() {
        let mut input = base_note_input();
        input.interest_rate = dec!(0.12);
        input.elapsed_months = 24;

        let result = convert_note(&input).unwrap();
        let out = &result.result;

        // accrued = 100_000 * 0.12 * 24 / 12 = 24_000
        assert_eq!(out.accrued_interest, dec!(24000));
        assert_eq!(out.total_conversion_amount, dec!(124000));
    }
}
