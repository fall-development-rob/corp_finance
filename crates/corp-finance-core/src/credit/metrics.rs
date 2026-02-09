use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::{types::*, CorpFinanceError, CorpFinanceResult};

/// Maximum coverage cap when interest expense is zero or near-zero.
const COVERAGE_CAP: Decimal = dec!(999);

// ---------------------------------------------------------------------------
// Input / Output types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditMetricsInput {
    pub revenue: Money,
    pub ebitda: Money,
    pub ebit: Money,
    pub interest_expense: Money,
    pub depreciation_amortisation: Money,
    pub total_debt: Money,
    pub cash: Money,
    pub total_assets: Money,
    pub current_assets: Money,
    pub current_liabilities: Money,
    pub total_equity: Money,
    pub retained_earnings: Money,
    pub working_capital: Money,
    pub operating_cash_flow: Money,
    pub capex: Money,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub funds_from_operations: Option<Money>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lease_payments: Option<Money>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred_dividends: Option<Money>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub market_cap: Option<Money>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditMetricsOutput {
    // Leverage
    pub net_debt: Money,
    pub net_debt_to_ebitda: Multiple,
    pub total_debt_to_ebitda: Multiple,
    pub debt_to_equity: Multiple,
    pub debt_to_assets: Rate,
    pub net_debt_to_ev: Option<Rate>,
    // Coverage
    pub interest_coverage: Multiple,
    pub ebit_coverage: Multiple,
    pub fixed_charge_coverage: Option<Multiple>,
    pub dscr: Multiple,
    // Cash flow
    pub ffo_to_debt: Option<Rate>,
    pub ocf_to_debt: Rate,
    pub fcf_to_debt: Rate,
    pub fcf: Money,
    pub cash_conversion: Rate,
    // Liquidity
    pub current_ratio: Multiple,
    pub quick_ratio: Multiple,
    pub cash_to_debt: Rate,
    // Rating
    pub implied_rating: CreditRating,
    pub rating_rationale: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CreditRating {
    AAA,
    #[serde(rename = "AA+")]
    AAp,
    AA,
    #[serde(rename = "AA-")]
    AAm,
    #[serde(rename = "A+")]
    Ap,
    A,
    #[serde(rename = "A-")]
    Am,
    #[serde(rename = "BBB+")]
    BBBp,
    BBB,
    #[serde(rename = "BBB-")]
    BBBm,
    #[serde(rename = "BB+")]
    BBp,
    BB,
    #[serde(rename = "BB-")]
    BBm,
    #[serde(rename = "B+")]
    Bp,
    B,
    #[serde(rename = "B-")]
    Bm,
    #[serde(rename = "CCC+")]
    CCCp,
    CCC,
    #[serde(rename = "CCC-")]
    CCCm,
    CC,
    C,
    D,
}

impl std::fmt::Display for CreditRating {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::AAA => "AAA",
            Self::AAp => "AA+",
            Self::AA => "AA",
            Self::AAm => "AA-",
            Self::Ap => "A+",
            Self::A => "A",
            Self::Am => "A-",
            Self::BBBp => "BBB+",
            Self::BBB => "BBB",
            Self::BBBm => "BBB-",
            Self::BBp => "BB+",
            Self::BB => "BB",
            Self::BBm => "BB-",
            Self::Bp => "B+",
            Self::B => "B",
            Self::Bm => "B-",
            Self::CCCp => "CCC+",
            Self::CCC => "CCC",
            Self::CCCm => "CCC-",
            Self::CC => "CC",
            Self::C => "C",
            Self::D => "D",
        };
        write!(f, "{}", s)
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Compute a full suite of credit metrics from financial statement data.
pub fn calculate_credit_metrics(
    input: &CreditMetricsInput,
) -> CorpFinanceResult<ComputationOutput<CreditMetricsOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // -- Validation ----------------------------------------------------------
    validate_input(input)?;

    if input.ebitda <= Decimal::ZERO {
        warnings.push("EBITDA is non-positive; leverage ratios may be meaningless.".into());
    }

    // -- Leverage -------------------------------------------------------------
    let net_debt = input.total_debt - input.cash;

    let net_debt_to_ebitda = safe_divide(net_debt, input.ebitda, "net_debt / EBITDA")?;
    let total_debt_to_ebitda = safe_divide(input.total_debt, input.ebitda, "total_debt / EBITDA")?;
    let debt_to_equity = safe_divide(input.total_debt, input.total_equity, "debt / equity")?;
    let debt_to_assets = safe_divide(input.total_debt, input.total_assets, "debt / assets")?;

    let net_debt_to_ev = input.market_cap.and_then(|mc| {
        let ev = mc + net_debt;
        if ev.is_zero() {
            None
        } else {
            Some(net_debt / ev)
        }
    });

    // -- Coverage -------------------------------------------------------------
    let interest_coverage = if input.interest_expense.is_zero() {
        warnings.push("Interest expense is zero; coverage capped at 999x.".into());
        COVERAGE_CAP
    } else {
        safe_divide(input.ebitda, input.interest_expense, "EBITDA / interest")?
    };

    let ebit_coverage = if input.interest_expense.is_zero() {
        COVERAGE_CAP
    } else {
        safe_divide(input.ebit, input.interest_expense, "EBIT / interest")?
    };

    let fixed_charge_coverage = match input.lease_payments {
        Some(lease) => {
            let total_charges =
                input.interest_expense + lease + input.preferred_dividends.unwrap_or(Decimal::ZERO);
            if total_charges.is_zero() {
                warnings.push("Total fixed charges are zero; fixed-charge coverage capped.".into());
                Some(COVERAGE_CAP)
            } else {
                let numerator = input.ebitda + lease; // EBITDAR proxy
                Some(safe_divide(
                    numerator,
                    total_charges,
                    "fixed charge coverage",
                )?)
            }
        }
        None => None,
    };

    // DSCR: (EBITDA - capex) / interest. If interest is zero, cap.
    let dscr = if input.interest_expense.is_zero() {
        COVERAGE_CAP
    } else {
        safe_divide(input.ebitda - input.capex, input.interest_expense, "DSCR")?
    };

    // -- Cash-flow metrics ----------------------------------------------------
    let fcf = input.operating_cash_flow - input.capex;

    let ffo_to_debt = match input.funds_from_operations {
        Some(ffo) => Some(safe_divide(ffo, input.total_debt, "FFO / debt")?),
        None => None,
    };

    let ocf_to_debt = safe_divide(input.operating_cash_flow, input.total_debt, "OCF / debt")?;
    let fcf_to_debt = safe_divide(fcf, input.total_debt, "FCF / debt")?;

    let cash_conversion = if input.ebitda.is_zero() {
        Decimal::ZERO
    } else {
        safe_divide(input.operating_cash_flow, input.ebitda, "cash conversion")?
    };

    // -- Liquidity ------------------------------------------------------------
    let current_ratio = safe_divide(
        input.current_assets,
        input.current_liabilities,
        "current ratio",
    )?;

    // Quick ratio: (current_assets - inventories). We approximate inventory as
    // current_assets - cash - receivables. Without explicit inventory, use
    // cash + receivables ≈ cash portion of current assets as proxy; simplify to
    // (current_assets - (current_assets - cash)) = cash / current_liabilities
    // However, a more standard proxy is (current_assets - inventory). Since we
    // don't have inventory, we use current_assets directly minus a conservative
    // haircut. For simplicity here, use cash / current_liabilities as a floor.
    let quick_ratio = safe_divide(input.cash, input.current_liabilities, "quick ratio")?;

    let cash_to_debt = safe_divide(input.cash, input.total_debt, "cash / debt")?;

    // -- Synthetic Rating -----------------------------------------------------
    let (implied_rating, rating_rationale) = derive_synthetic_rating(
        interest_coverage,
        net_debt_to_ebitda,
        debt_to_equity,
        fcf_to_debt,
        current_ratio,
    );

    let output = CreditMetricsOutput {
        net_debt,
        net_debt_to_ebitda,
        total_debt_to_ebitda,
        debt_to_equity,
        debt_to_assets,
        net_debt_to_ev,
        interest_coverage,
        ebit_coverage,
        fixed_charge_coverage,
        dscr,
        ffo_to_debt,
        ocf_to_debt,
        fcf_to_debt,
        fcf,
        cash_conversion,
        current_ratio,
        quick_ratio,
        cash_to_debt,
        implied_rating,
        rating_rationale,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    let assumptions = serde_json::json!({
        "dscr_uses_interest_only": true,
        "quick_ratio_proxy": "cash / current_liabilities",
        "coverage_cap": "999x when interest is zero"
    });

    Ok(with_metadata(
        "Credit Metrics (CFA Level II methodology)",
        &assumptions,
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn validate_input(input: &CreditMetricsInput) -> CorpFinanceResult<()> {
    if input.revenue <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "revenue".into(),
            reason: "Revenue must be positive.".into(),
        });
    }
    if input.total_assets <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "total_assets".into(),
            reason: "Total assets must be positive.".into(),
        });
    }
    if input.total_debt < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "total_debt".into(),
            reason: "Total debt cannot be negative.".into(),
        });
    }
    if input.current_liabilities <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "current_liabilities".into(),
            reason: "Current liabilities must be positive.".into(),
        });
    }
    Ok(())
}

fn safe_divide(
    numerator: Decimal,
    denominator: Decimal,
    context: &str,
) -> CorpFinanceResult<Decimal> {
    if denominator.is_zero() {
        return Err(CorpFinanceError::DivisionByZero {
            context: context.to_string(),
        });
    }
    Ok(numerator / denominator)
}

/// Derive a synthetic credit rating from key ratios.
///
/// The mapping follows a simplified Damodaran-style grid keyed primarily on
/// interest coverage and net-debt/EBITDA, with secondary signals from D/E,
/// FCF/debt, and current ratio.
fn derive_synthetic_rating(
    coverage: Multiple,
    leverage: Multiple,
    debt_to_equity: Multiple,
    fcf_to_debt: Rate,
    current_ratio: Multiple,
) -> (CreditRating, Vec<String>) {
    let mut rationale = Vec::new();

    // Primary grid: coverage x leverage
    let base_rating = match (coverage, leverage) {
        (c, l) if c > dec!(8.0) && l < dec!(1.0) => {
            rationale.push(format!(
                "Coverage {c}x > 8.0 and leverage {l}x < 1.0 => AAA zone"
            ));
            CreditRating::AAA
        }
        (c, l) if c > dec!(6.0) && l < dec!(2.0) => {
            rationale.push(format!(
                "Coverage {c}x > 6.0 and leverage {l}x < 2.0 => AA zone"
            ));
            CreditRating::AA
        }
        (c, l) if c > dec!(5.0) && l < dec!(2.5) => {
            rationale.push(format!(
                "Coverage {c}x > 5.0 and leverage {l}x < 2.5 => A zone"
            ));
            CreditRating::A
        }
        (c, l) if c > dec!(4.0) && l < dec!(3.5) => {
            rationale.push(format!(
                "Coverage {c}x > 4.0 and leverage {l}x < 3.5 => BBB zone"
            ));
            CreditRating::BBB
        }
        (c, l) if c > dec!(3.0) && l < dec!(4.5) => {
            rationale.push(format!(
                "Coverage {c}x > 3.0 and leverage {l}x < 4.5 => BB zone"
            ));
            CreditRating::BB
        }
        (c, l) if c > dec!(2.0) && l < dec!(5.5) => {
            rationale.push(format!(
                "Coverage {c}x > 2.0 and leverage {l}x < 5.5 => B zone"
            ));
            CreditRating::B
        }
        (c, l) if c > dec!(1.0) && l < dec!(7.0) => {
            rationale.push(format!(
                "Coverage {c}x > 1.0 and leverage {l}x < 7.0 => CCC zone"
            ));
            CreditRating::CCC
        }
        (c, _) if c > dec!(0.5) => {
            rationale.push(format!("Coverage {c}x > 0.5 but high leverage => CC zone"));
            CreditRating::CC
        }
        (c, _) if c > Decimal::ZERO => {
            rationale.push(format!("Marginal coverage {c}x => C zone"));
            CreditRating::C
        }
        _ => {
            rationale.push("Zero or negative coverage => D (default)".into());
            CreditRating::D
        }
    };

    // Secondary modifiers (informational; do not notch for simplicity)
    if debt_to_equity > dec!(3.0) {
        rationale.push(format!(
            "Elevated D/E of {debt_to_equity}x (>3.0) is a negative signal."
        ));
    }
    if fcf_to_debt < dec!(0.05) {
        rationale.push(format!(
            "Low FCF/Debt of {fcf_to_debt} (<5%) limits financial flexibility."
        ));
    }
    if current_ratio < dec!(1.0) {
        rationale.push(format!(
            "Current ratio {current_ratio}x (<1.0) signals liquidity risk."
        ));
    }

    (base_rating, rationale)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn sample_input() -> CreditMetricsInput {
        CreditMetricsInput {
            revenue: dec!(1_000_000),
            ebitda: dec!(200_000),
            ebit: dec!(150_000),
            interest_expense: dec!(25_000),
            depreciation_amortisation: dec!(50_000),
            total_debt: dec!(500_000),
            cash: dec!(80_000),
            total_assets: dec!(1_200_000),
            current_assets: dec!(300_000),
            current_liabilities: dec!(150_000),
            total_equity: dec!(400_000),
            retained_earnings: dec!(200_000),
            working_capital: dec!(150_000),
            operating_cash_flow: dec!(180_000),
            capex: dec!(60_000),
            funds_from_operations: Some(dec!(170_000)),
            lease_payments: Some(dec!(10_000)),
            preferred_dividends: None,
            market_cap: Some(dec!(800_000)),
        }
    }

    #[test]
    fn test_basic_credit_metrics() {
        let input = sample_input();
        let result = calculate_credit_metrics(&input).unwrap();
        let m = &result.result;

        // Net debt = 500k - 80k = 420k
        assert_eq!(m.net_debt, dec!(420_000));

        // Net debt / EBITDA = 420k / 200k = 2.1
        assert_eq!(m.net_debt_to_ebitda, dec!(2.1));

        // Total debt / EBITDA = 500k / 200k = 2.5
        assert_eq!(m.total_debt_to_ebitda, dec!(2.5));

        // Debt / equity = 500k / 400k = 1.25
        assert_eq!(m.debt_to_equity, dec!(1.25));

        // Interest coverage = 200k / 25k = 8
        assert_eq!(m.interest_coverage, dec!(8));

        // EBIT coverage = 150k / 25k = 6
        assert_eq!(m.ebit_coverage, dec!(6));

        // FCF = 180k - 60k = 120k
        assert_eq!(m.fcf, dec!(120_000));

        // Current ratio = 300k / 150k = 2
        assert_eq!(m.current_ratio, dec!(2));
    }

    #[test]
    fn test_dscr_calculation() {
        let input = sample_input();
        let result = calculate_credit_metrics(&input).unwrap();
        // DSCR = (200k - 60k) / 25k = 140k / 25k = 5.6
        assert_eq!(result.result.dscr, dec!(5.6));
    }

    #[test]
    fn test_ffo_to_debt() {
        let input = sample_input();
        let result = calculate_credit_metrics(&input).unwrap();
        // FFO/debt = 170k / 500k = 0.34
        assert_eq!(result.result.ffo_to_debt, Some(dec!(0.34)));
    }

    #[test]
    fn test_fixed_charge_coverage() {
        let input = sample_input();
        let result = calculate_credit_metrics(&input).unwrap();
        // total_charges = 25k + 10k = 35k
        // numerator = EBITDA + lease = 200k + 10k = 210k
        // FCC = 210k / 35k = 6
        assert_eq!(result.result.fixed_charge_coverage, Some(dec!(6)));
    }

    #[test]
    fn test_net_debt_to_ev() {
        let input = sample_input();
        let result = calculate_credit_metrics(&input).unwrap();
        // EV = 800k + 420k = 1_220k
        // net_debt/EV = 420k / 1_220k ≈ 0.3442622...
        let nd_ev = result.result.net_debt_to_ev.unwrap();
        // Check to 4 decimal places
        let expected = dec!(420_000) / dec!(1_220_000);
        assert_eq!(nd_ev, expected);
    }

    #[test]
    fn test_synthetic_rating_strong_company() {
        // High coverage, low leverage => AAA
        let input = CreditMetricsInput {
            revenue: dec!(2_000_000),
            ebitda: dec!(500_000),
            ebit: dec!(450_000),
            interest_expense: dec!(50_000),
            depreciation_amortisation: dec!(50_000),
            total_debt: dec!(300_000),
            cash: dec!(200_000),
            total_assets: dec!(2_000_000),
            current_assets: dec!(500_000),
            current_liabilities: dec!(200_000),
            total_equity: dec!(1_000_000),
            retained_earnings: dec!(600_000),
            working_capital: dec!(300_000),
            operating_cash_flow: dec!(400_000),
            capex: dec!(100_000),
            funds_from_operations: None,
            lease_payments: None,
            preferred_dividends: None,
            market_cap: None,
        };
        let result = calculate_credit_metrics(&input).unwrap();
        // coverage = 500k/50k = 10, leverage = 100k/500k = 0.2 => AAA
        assert_eq!(result.result.implied_rating, CreditRating::AAA);
    }

    #[test]
    fn test_zero_interest_coverage_cap() {
        let mut input = sample_input();
        input.interest_expense = Decimal::ZERO;
        let result = calculate_credit_metrics(&input).unwrap();
        assert_eq!(result.result.interest_coverage, dec!(999));
        assert_eq!(result.result.ebit_coverage, dec!(999));
        assert!(result.warnings.iter().any(|w| w.contains("zero")));
    }

    #[test]
    fn test_invalid_revenue_rejected() {
        let mut input = sample_input();
        input.revenue = dec!(-100);
        let err = calculate_credit_metrics(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => assert_eq!(field, "revenue"),
            other => panic!("Expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn test_zero_total_debt_division() {
        let mut input = sample_input();
        input.total_debt = Decimal::ZERO;
        let err = calculate_credit_metrics(&input).unwrap_err();
        match err {
            CorpFinanceError::DivisionByZero { .. } => {} // expected
            other => panic!("Expected DivisionByZero, got {other:?}"),
        }
    }

    #[test]
    fn test_metadata_populated() {
        let input = sample_input();
        let result = calculate_credit_metrics(&input).unwrap();
        assert!(!result.methodology.is_empty());
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
    }
}
