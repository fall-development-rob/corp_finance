//! Payout Sustainability Analysis.
//!
//! Evaluates whether a company's dividend payout is sustainable by examining:
//! 1. **Earnings & FCF payout ratios** — coverage from both an accrual and cash basis.
//! 2. **Post-dividend DSCR** — ability to service debt after dividends.
//! 3. **Capex coverage** — residual cash after dividends vs required capex.
//! 4. **Leverage check** — Net Debt / EBITDA.
//! 5. **Sustainability rating** — Sustainable / Watch / At Risk / Unsustainable.
//!
//! All arithmetic uses `rust_decimal::Decimal`. No `f64`.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Input / Output
// ---------------------------------------------------------------------------

/// Input for payout sustainability analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayoutSustainabilityInput {
    /// Earnings per share.
    pub eps: Decimal,
    /// Dividend per share.
    pub dps: Decimal,
    /// Free cash flow per share.
    pub fcf_per_share: Decimal,
    /// Net debt.
    pub net_debt: Decimal,
    /// EBITDA.
    pub ebitda: Decimal,
    /// Interest expense.
    pub interest_expense: Decimal,
    /// Total dividend outflow.
    pub total_dividends: Decimal,
    /// Minimum required capex.
    pub capex_required: Decimal,
    /// Operating cash flow.
    pub operating_cash_flow: Decimal,
    /// Target payout ratio (optional; defaults to 0.40).
    pub target_payout_ratio: Option<Decimal>,
}

/// Output of payout sustainability analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayoutSustainabilityOutput {
    /// Earnings payout ratio: DPS / EPS.
    pub earnings_payout_ratio: Decimal,
    /// FCF payout ratio: DPS / FCF_per_share.
    pub fcf_payout_ratio: Decimal,
    /// Dividend coverage from earnings: EPS / DPS.
    pub dividend_coverage: Decimal,
    /// Dividend coverage from FCF: FCF / DPS.
    pub fcf_coverage: Decimal,
    /// Post-dividend debt service coverage ratio.
    pub post_dividend_dscr: Decimal,
    /// Capex coverage: (OCF - dividends) / capex.
    pub capex_coverage: Decimal,
    /// Leverage ratio: Net Debt / EBITDA.
    pub leverage_ratio: Decimal,
    /// Overall sustainability flag.
    pub payout_sustainable: bool,
    /// Maximum sustainable DPS given target payout and FCF constraint.
    pub max_sustainable_dps: Decimal,
    /// Qualitative sustainability rating.
    pub sustainability_rating: String,
}

// ---------------------------------------------------------------------------
// Core function
// ---------------------------------------------------------------------------

/// Analyze dividend payout sustainability.
pub fn calculate_payout_sustainability(
    input: &PayoutSustainabilityInput,
) -> CorpFinanceResult<PayoutSustainabilityOutput> {
    validate_input(input)?;

    let target_payout = input.target_payout_ratio.unwrap_or(dec!(0.40));

    // Earnings payout ratio
    let earnings_payout_ratio = if input.eps == Decimal::ZERO {
        if input.dps > Decimal::ZERO {
            dec!(999.99) // Paying dividends with zero earnings
        } else {
            Decimal::ZERO
        }
    } else {
        input.dps / input.eps
    };

    // FCF payout ratio
    let fcf_payout_ratio = if input.fcf_per_share == Decimal::ZERO {
        if input.dps > Decimal::ZERO {
            dec!(999.99)
        } else {
            Decimal::ZERO
        }
    } else {
        input.dps / input.fcf_per_share
    };

    // Dividend coverage (inverse of payout ratio)
    let dividend_coverage = if input.dps == Decimal::ZERO {
        dec!(999.99) // Infinite coverage when no dividends paid
    } else {
        input.eps / input.dps
    };

    let fcf_coverage = if input.dps == Decimal::ZERO {
        dec!(999.99)
    } else {
        input.fcf_per_share / input.dps
    };

    // Post-dividend DSCR
    let post_dividend_dscr = if input.interest_expense == Decimal::ZERO {
        dec!(999.99) // No debt
    } else {
        (input.operating_cash_flow - input.total_dividends) / input.interest_expense
    };

    // Capex coverage
    let capex_coverage = if input.capex_required == Decimal::ZERO {
        dec!(999.99)
    } else {
        (input.operating_cash_flow - input.total_dividends) / input.capex_required
    };

    // Leverage ratio
    let leverage_ratio = if input.ebitda == Decimal::ZERO {
        if input.net_debt > Decimal::ZERO {
            dec!(999.99)
        } else {
            Decimal::ZERO
        }
    } else {
        input.net_debt / input.ebitda
    };

    // Sustainability flag
    let payout_sustainable = dividend_coverage > dec!(1.2)
        && fcf_coverage > Decimal::ONE
        && post_dividend_dscr > dec!(1.5);

    // Max sustainable DPS: min(EPS * target_payout, FCF * 0.80)
    let eps_constrained = if input.eps > Decimal::ZERO {
        input.eps * target_payout
    } else {
        Decimal::ZERO
    };
    let fcf_constrained = if input.fcf_per_share > Decimal::ZERO {
        input.fcf_per_share * dec!(0.80)
    } else {
        Decimal::ZERO
    };
    let max_sustainable_dps = eps_constrained.min(fcf_constrained);

    // Rating classification
    let sustainability_rating =
        classify_sustainability(earnings_payout_ratio, fcf_coverage, leverage_ratio);

    Ok(PayoutSustainabilityOutput {
        earnings_payout_ratio,
        fcf_payout_ratio,
        dividend_coverage,
        fcf_coverage,
        post_dividend_dscr,
        capex_coverage,
        leverage_ratio,
        payout_sustainable,
        max_sustainable_dps,
        sustainability_rating,
    })
}

// ---------------------------------------------------------------------------
// Rating classification
// ---------------------------------------------------------------------------

fn classify_sustainability(
    payout_ratio: Decimal,
    fcf_coverage: Decimal,
    leverage: Decimal,
) -> String {
    if payout_ratio < dec!(0.60) && fcf_coverage > dec!(1.5) && leverage < dec!(2.5) {
        "Sustainable".into()
    } else if payout_ratio < dec!(0.80) && fcf_coverage > Decimal::ONE && leverage < dec!(3.5) {
        "Watch".into()
    } else if payout_ratio < Decimal::ONE && fcf_coverage > dec!(0.5) {
        "At Risk".into()
    } else {
        "Unsustainable".into()
    }
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_input(input: &PayoutSustainabilityInput) -> CorpFinanceResult<()> {
    if input.dps < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "dps".into(),
            reason: "Dividend per share must be non-negative.".into(),
        });
    }
    if input.total_dividends < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "total_dividends".into(),
            reason: "Total dividends must be non-negative.".into(),
        });
    }
    if input.capex_required < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "capex_required".into(),
            reason: "Required capex must be non-negative.".into(),
        });
    }
    if let Some(tp) = input.target_payout_ratio {
        if tp < Decimal::ZERO || tp > Decimal::ONE {
            return Err(CorpFinanceError::InvalidInput {
                field: "target_payout_ratio".into(),
                reason: "Target payout ratio must be between 0 and 1.".into(),
            });
        }
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

    fn approx_eq(a: Decimal, b: Decimal, eps: Decimal) -> bool {
        (a - b).abs() < eps
    }

    fn sustainable_input() -> PayoutSustainabilityInput {
        PayoutSustainabilityInput {
            eps: dec!(5.00),
            dps: dec!(2.00),
            fcf_per_share: dec!(4.50),
            net_debt: dec!(1000),
            ebitda: dec!(800),
            interest_expense: dec!(50),
            total_dividends: dec!(200),
            capex_required: dec!(300),
            operating_cash_flow: dec!(600),
            target_payout_ratio: None,
        }
    }

    fn aggressive_input() -> PayoutSustainabilityInput {
        PayoutSustainabilityInput {
            eps: dec!(3.00),
            dps: dec!(2.80),
            fcf_per_share: dec!(2.50),
            net_debt: dec!(3000),
            ebitda: dec!(800),
            interest_expense: dec!(100),
            total_dividends: dec!(560),
            capex_required: dec!(200),
            operating_cash_flow: dec!(500),
            target_payout_ratio: Some(dec!(0.50)),
        }
    }

    #[test]
    fn test_sustainable_payout() {
        let input = sustainable_input();
        let out = calculate_payout_sustainability(&input).unwrap();
        assert_eq!(out.sustainability_rating, "Sustainable");
        assert!(out.payout_sustainable);
    }

    #[test]
    fn test_earnings_payout_ratio() {
        let input = sustainable_input();
        let out = calculate_payout_sustainability(&input).unwrap();
        // 2.00 / 5.00 = 0.40
        assert!(approx_eq(
            out.earnings_payout_ratio,
            dec!(0.40),
            dec!(0.001)
        ));
    }

    #[test]
    fn test_fcf_payout_ratio() {
        let input = sustainable_input();
        let out = calculate_payout_sustainability(&input).unwrap();
        // 2.00 / 4.50 = 0.4444
        assert!(approx_eq(out.fcf_payout_ratio, dec!(0.4444), dec!(0.01)));
    }

    #[test]
    fn test_dividend_coverage() {
        let input = sustainable_input();
        let out = calculate_payout_sustainability(&input).unwrap();
        // 5.00 / 2.00 = 2.50
        assert!(approx_eq(out.dividend_coverage, dec!(2.50), dec!(0.001)));
    }

    #[test]
    fn test_fcf_coverage() {
        let input = sustainable_input();
        let out = calculate_payout_sustainability(&input).unwrap();
        // 4.50 / 2.00 = 2.25
        assert!(approx_eq(out.fcf_coverage, dec!(2.25), dec!(0.001)));
    }

    #[test]
    fn test_post_dividend_dscr() {
        let input = sustainable_input();
        let out = calculate_payout_sustainability(&input).unwrap();
        // (600 - 200) / 50 = 8.0
        assert!(approx_eq(out.post_dividend_dscr, dec!(8.0), dec!(0.01)));
    }

    #[test]
    fn test_capex_coverage() {
        let input = sustainable_input();
        let out = calculate_payout_sustainability(&input).unwrap();
        // (600 - 200) / 300 = 1.3333
        assert!(approx_eq(out.capex_coverage, dec!(1.3333), dec!(0.01)));
    }

    #[test]
    fn test_leverage_ratio() {
        let input = sustainable_input();
        let out = calculate_payout_sustainability(&input).unwrap();
        // 1000 / 800 = 1.25
        assert!(approx_eq(out.leverage_ratio, dec!(1.25), dec!(0.001)));
    }

    #[test]
    fn test_max_sustainable_dps_default_target() {
        let input = sustainable_input();
        let out = calculate_payout_sustainability(&input).unwrap();
        // eps_constrained = 5.00 * 0.40 = 2.00
        // fcf_constrained = 4.50 * 0.80 = 3.60
        // min = 2.00
        assert!(approx_eq(out.max_sustainable_dps, dec!(2.00), dec!(0.01)));
    }

    #[test]
    fn test_max_sustainable_dps_custom_target() {
        let input = aggressive_input();
        let out = calculate_payout_sustainability(&input).unwrap();
        // eps_constrained = 3.00 * 0.50 = 1.50
        // fcf_constrained = 2.50 * 0.80 = 2.00
        // min = 1.50
        assert!(approx_eq(out.max_sustainable_dps, dec!(1.50), dec!(0.01)));
    }

    #[test]
    fn test_aggressive_payout_at_risk_or_watch() {
        let input = aggressive_input();
        let out = calculate_payout_sustainability(&input).unwrap();
        // payout = 2.80/3.00 = 0.9333, fcf_coverage = 2.50/2.80 = 0.893
        // payout < 1.0 and fcf_coverage > 0.5 => "At Risk"
        assert_eq!(out.sustainability_rating, "At Risk");
    }

    #[test]
    fn test_unsustainable_payout() {
        let input = PayoutSustainabilityInput {
            eps: dec!(2.00),
            dps: dec!(3.00),
            fcf_per_share: dec!(1.50),
            net_debt: dec!(5000),
            ebitda: dec!(500),
            interest_expense: dec!(200),
            total_dividends: dec!(600),
            capex_required: dec!(300),
            operating_cash_flow: dec!(400),
            target_payout_ratio: None,
        };
        let out = calculate_payout_sustainability(&input).unwrap();
        // payout = 3/2 = 1.5, fcf coverage = 1.5/3 = 0.5 => at boundary
        // payout >= 1.0 and fcf_coverage <= 0.5 => "Unsustainable"
        assert_eq!(out.sustainability_rating, "Unsustainable");
    }

    #[test]
    fn test_zero_eps_positive_dps() {
        let input = PayoutSustainabilityInput {
            eps: Decimal::ZERO,
            dps: dec!(1.00),
            fcf_per_share: dec!(2.00),
            net_debt: dec!(100),
            ebitda: dec!(200),
            interest_expense: dec!(10),
            total_dividends: dec!(100),
            capex_required: dec!(50),
            operating_cash_flow: dec!(300),
            target_payout_ratio: None,
        };
        let out = calculate_payout_sustainability(&input).unwrap();
        assert!(approx_eq(
            out.earnings_payout_ratio,
            dec!(999.99),
            dec!(0.01)
        ));
    }

    #[test]
    fn test_zero_dps() {
        let input = PayoutSustainabilityInput {
            eps: dec!(5.00),
            dps: Decimal::ZERO,
            fcf_per_share: dec!(4.00),
            net_debt: dec!(100),
            ebitda: dec!(200),
            interest_expense: dec!(10),
            total_dividends: Decimal::ZERO,
            capex_required: dec!(50),
            operating_cash_flow: dec!(300),
            target_payout_ratio: None,
        };
        let out = calculate_payout_sustainability(&input).unwrap();
        assert_eq!(out.earnings_payout_ratio, Decimal::ZERO);
        assert!(approx_eq(out.dividend_coverage, dec!(999.99), dec!(0.01)));
    }

    #[test]
    fn test_negative_fcf() {
        let input = PayoutSustainabilityInput {
            eps: dec!(3.00),
            dps: dec!(1.50),
            fcf_per_share: dec!(-1.00),
            net_debt: dec!(1000),
            ebitda: dec!(500),
            interest_expense: dec!(50),
            total_dividends: dec!(150),
            capex_required: dec!(100),
            operating_cash_flow: dec!(200),
            target_payout_ratio: None,
        };
        let out = calculate_payout_sustainability(&input).unwrap();
        // FCF coverage = -1.00 / 1.50 < 0 => not sustainable
        assert!(!out.payout_sustainable);
    }

    #[test]
    fn test_high_leverage() {
        let input = PayoutSustainabilityInput {
            eps: dec!(5.00),
            dps: dec!(1.50),
            fcf_per_share: dec!(4.00),
            net_debt: dec!(5000),
            ebitda: dec!(500),
            interest_expense: dec!(200),
            total_dividends: dec!(150),
            capex_required: dec!(100),
            operating_cash_flow: dec!(600),
            target_payout_ratio: None,
        };
        let out = calculate_payout_sustainability(&input).unwrap();
        // leverage = 5000/500 = 10.0 => not "Sustainable" even with low payout
        assert_ne!(out.sustainability_rating, "Sustainable");
    }

    #[test]
    fn test_zero_interest_expense() {
        let input = PayoutSustainabilityInput {
            interest_expense: Decimal::ZERO,
            ..sustainable_input()
        };
        let out = calculate_payout_sustainability(&input).unwrap();
        assert!(approx_eq(out.post_dividend_dscr, dec!(999.99), dec!(0.01)));
    }

    #[test]
    fn test_zero_capex_required() {
        let input = PayoutSustainabilityInput {
            capex_required: Decimal::ZERO,
            ..sustainable_input()
        };
        let out = calculate_payout_sustainability(&input).unwrap();
        assert!(approx_eq(out.capex_coverage, dec!(999.99), dec!(0.01)));
    }

    #[test]
    fn test_reject_negative_dps() {
        let input = PayoutSustainabilityInput {
            dps: dec!(-1),
            ..sustainable_input()
        };
        assert!(calculate_payout_sustainability(&input).is_err());
    }

    #[test]
    fn test_reject_negative_total_dividends() {
        let input = PayoutSustainabilityInput {
            total_dividends: dec!(-100),
            ..sustainable_input()
        };
        assert!(calculate_payout_sustainability(&input).is_err());
    }

    #[test]
    fn test_reject_target_payout_out_of_range() {
        let input = PayoutSustainabilityInput {
            target_payout_ratio: Some(dec!(1.5)),
            ..sustainable_input()
        };
        assert!(calculate_payout_sustainability(&input).is_err());
    }

    #[test]
    fn test_reject_negative_capex() {
        let input = PayoutSustainabilityInput {
            capex_required: dec!(-50),
            ..sustainable_input()
        };
        assert!(calculate_payout_sustainability(&input).is_err());
    }

    #[test]
    fn test_watch_classification() {
        let input = PayoutSustainabilityInput {
            eps: dec!(5.00),
            dps: dec!(3.50),
            fcf_per_share: dec!(4.50),
            net_debt: dec!(2000),
            ebitda: dec!(800),
            interest_expense: dec!(60),
            total_dividends: dec!(350),
            capex_required: dec!(200),
            operating_cash_flow: dec!(600),
            target_payout_ratio: None,
        };
        let out = calculate_payout_sustainability(&input).unwrap();
        // payout = 3.50/5.00 = 0.70 (< 0.80), fcf_coverage = 4.50/3.50 = 1.286 (> 1.0 but < 1.5)
        // leverage = 2000/800 = 2.5 (< 3.5 but >= 2.5, so not Sustainable)
        assert_eq!(out.sustainability_rating, "Watch");
    }

    #[test]
    fn test_serialization_roundtrip() {
        let input = sustainable_input();
        let out = calculate_payout_sustainability(&input).unwrap();
        let json = serde_json::to_string(&out).unwrap();
        let _: PayoutSustainabilityOutput = serde_json::from_str(&json).unwrap();
    }
}
