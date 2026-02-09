use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::*;
use crate::CorpFinanceResult;

/// Input for Sources & Uses calculation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourcesUsesInput {
    /// Enterprise value of the target
    pub enterprise_value: Money,
    /// Equity contribution from sponsor
    pub equity_contribution: Money,
    /// Debt tranches: (name, amount)
    pub debt_tranches: Vec<(String, Money)>,
    /// Transaction advisory fees
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_fees: Option<Money>,
    /// Debt financing/arrangement fees
    #[serde(skip_serializing_if = "Option::is_none")]
    pub financing_fees: Option<Money>,
    /// Management equity rollover
    #[serde(skip_serializing_if = "Option::is_none")]
    pub management_rollover: Option<Money>,
}

/// Output for Sources & Uses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourcesUsesOutput {
    /// All sources of funds
    pub sources: Vec<(String, Money)>,
    /// All uses of funds
    pub uses: Vec<(String, Money)>,
    /// Total sources
    pub total_sources: Money,
    /// Total uses
    pub total_uses: Money,
    /// Whether sources equal uses
    pub balanced: bool,
}

/// Build Sources & Uses table for a leveraged transaction.
pub fn build_sources_uses(
    input: &SourcesUsesInput,
) -> CorpFinanceResult<ComputationOutput<SourcesUsesOutput>> {
    let start = Instant::now();
    let warnings: Vec<String> = Vec::new();

    if input.enterprise_value <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "enterprise_value".into(),
            reason: "Enterprise value must be positive".into(),
        });
    }
    if input.equity_contribution < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "equity_contribution".into(),
            reason: "Equity contribution cannot be negative".into(),
        });
    }

    // Build sources
    let mut sources: Vec<(String, Money)> = Vec::new();
    sources.push(("Sponsor Equity".into(), input.equity_contribution));

    for (name, amount) in &input.debt_tranches {
        if *amount < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("debt_tranche:{name}"),
                reason: "Debt tranche amount cannot be negative".into(),
            });
        }
        sources.push((name.clone(), *amount));
    }

    if let Some(rollover) = input.management_rollover {
        if rollover > Decimal::ZERO {
            sources.push(("Management Rollover".into(), rollover));
        }
    }

    // Build uses
    let mut uses: Vec<(String, Money)> = Vec::new();
    uses.push(("Enterprise Value".into(), input.enterprise_value));

    if let Some(fees) = input.transaction_fees {
        if fees > Decimal::ZERO {
            uses.push(("Transaction Fees".into(), fees));
        }
    }

    if let Some(fees) = input.financing_fees {
        if fees > Decimal::ZERO {
            uses.push(("Financing Fees".into(), fees));
        }
    }

    let total_sources: Money = sources.iter().map(|(_, v)| *v).sum();
    let total_uses: Money = uses.iter().map(|(_, v)| *v).sum();
    let balanced = total_sources == total_uses;

    let output = SourcesUsesOutput {
        sources,
        uses,
        total_sources,
        total_uses,
        balanced,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Sources & Uses of Funds",
        &serde_json::json!({
            "enterprise_value": input.enterprise_value.to_string(),
            "equity": input.equity_contribution.to_string(),
        }),
        warnings,
        elapsed,
        output,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_balanced_sources_uses() {
        let input = SourcesUsesInput {
            enterprise_value: dec!(1000),
            equity_contribution: dec!(400),
            debt_tranches: vec![
                ("Senior Debt".into(), dec!(500)),
                ("Mezzanine".into(), dec!(150)),
            ],
            transaction_fees: Some(dec!(30)),
            financing_fees: Some(dec!(20)),
            management_rollover: None,
        };
        let result = build_sources_uses(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.total_sources, dec!(1050));
        assert_eq!(out.total_uses, dec!(1050));
        assert!(out.balanced);
    }

    #[test]
    fn test_unbalanced_sources_uses() {
        let input = SourcesUsesInput {
            enterprise_value: dec!(1000),
            equity_contribution: dec!(400),
            debt_tranches: vec![("Senior Debt".into(), dec!(500))],
            transaction_fees: None,
            financing_fees: None,
            management_rollover: None,
        };
        let result = build_sources_uses(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.total_sources, dec!(900));
        assert_eq!(out.total_uses, dec!(1000));
        assert!(!out.balanced);
    }

    #[test]
    fn test_with_management_rollover() {
        let input = SourcesUsesInput {
            enterprise_value: dec!(1000),
            equity_contribution: dec!(350),
            debt_tranches: vec![("Term Loan".into(), dec!(600))],
            transaction_fees: None,
            financing_fees: None,
            management_rollover: Some(dec!(50)),
        };
        let result = build_sources_uses(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.total_sources, dec!(1000));
        assert!(out.balanced);
        assert!(out.sources.iter().any(|(n, _)| n == "Management Rollover"));
    }

    #[test]
    fn test_zero_ev_error() {
        let input = SourcesUsesInput {
            enterprise_value: dec!(0),
            equity_contribution: dec!(100),
            debt_tranches: vec![],
            transaction_fees: None,
            financing_fees: None,
            management_rollover: None,
        };
        assert!(build_sources_uses(&input).is_err());
    }

    #[test]
    fn test_negative_equity_error() {
        let input = SourcesUsesInput {
            enterprise_value: dec!(1000),
            equity_contribution: dec!(-100),
            debt_tranches: vec![],
            transaction_fees: None,
            financing_fees: None,
            management_rollover: None,
        };
        assert!(build_sources_uses(&input).is_err());
    }

    #[test]
    fn test_sources_labels() {
        let input = SourcesUsesInput {
            enterprise_value: dec!(500),
            equity_contribution: dec!(200),
            debt_tranches: vec![
                ("Revolver".into(), dec!(100)),
                ("Term Loan A".into(), dec!(200)),
            ],
            transaction_fees: None,
            financing_fees: None,
            management_rollover: None,
        };
        let result = build_sources_uses(&input).unwrap();
        let names: Vec<&str> = result
            .result
            .sources
            .iter()
            .map(|(n, _)| n.as_str())
            .collect();
        assert_eq!(names, vec!["Sponsor Equity", "Revolver", "Term Loan A"]);
    }
}
