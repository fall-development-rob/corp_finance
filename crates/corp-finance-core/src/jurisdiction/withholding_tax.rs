use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::*;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Jurisdiction {
    US,
    UK,
    Cayman,
    Ireland,
    Luxembourg,
    Jersey,
    Guernsey,
    BVI,
    Germany,
    France,
    Netherlands,
    Switzerland,
    Singapore,
    HongKong,
    Japan,
    Australia,
    Canada,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IncomeType {
    Dividend,
    Interest,
    Royalty,
    RentalIncome,
    CapitalGain,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhtInput {
    pub source_jurisdiction: Jurisdiction,
    pub investor_jurisdiction: Jurisdiction,
    pub fund_jurisdiction: Option<Jurisdiction>,
    pub income_type: IncomeType,
    pub gross_income: Money,
    pub is_tax_exempt_investor: bool,
    pub currency: Option<Currency>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhtOutput {
    pub statutory_rate: Rate,
    pub treaty_rate: Option<Rate>,
    pub effective_rate: Rate,
    pub withholding_amount: Money,
    pub net_income: Money,
    pub treaty_name: Option<String>,
    pub notes: Vec<String>,
    pub blocker_recommendation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioWhtInput {
    pub holdings: Vec<WhtInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioWhtOutput {
    pub total_gross_income: Money,
    pub total_wht: Money,
    pub total_net_income: Money,
    pub effective_wht_rate: Rate,
    pub wht_drag_on_return: Rate,
    pub per_holding: Vec<WhtOutput>,
    pub optimisation_suggestions: Vec<String>,
}

// ---------------------------------------------------------------------------
// Statutory rate lookup
// ---------------------------------------------------------------------------

/// Returns the statutory WHT rate for a given source jurisdiction and income type.
fn statutory_rate(source: &Jurisdiction, income_type: &IncomeType) -> Rate {
    use IncomeType::*;
    use Jurisdiction::*;

    match (source, income_type) {
        // US
        (US, Dividend) => dec!(0.30),
        (US, Interest) => dec!(0.30),
        (US, Royalty) => dec!(0.30),
        (US, CapitalGain) => dec!(0.0),
        (US, RentalIncome) => dec!(0.30),

        // UK
        (UK, Dividend) => dec!(0.0),
        (UK, Interest) => dec!(0.20),
        (UK, Royalty) => dec!(0.20),
        (UK, CapitalGain) => dec!(0.0),
        (UK, RentalIncome) => dec!(0.20),

        // Germany (incl. solidarity surcharge)
        (Germany, Dividend) => dec!(0.26375),
        (Germany, Interest) => dec!(0.0),
        (Germany, Royalty) => dec!(0.15),
        (Germany, CapitalGain) => dec!(0.0),
        (Germany, RentalIncome) => dec!(0.15),

        // France
        (France, Dividend) => dec!(0.30),
        (France, Interest) => dec!(0.0),
        (France, Royalty) => dec!(0.3333),
        (France, CapitalGain) => dec!(0.0),
        (France, RentalIncome) => dec!(0.30),

        // Ireland
        (Ireland, Dividend) => dec!(0.25),
        (Ireland, Interest) => dec!(0.20),
        (Ireland, Royalty) => dec!(0.20),
        (Ireland, CapitalGain) => dec!(0.0),
        (Ireland, RentalIncome) => dec!(0.20),

        // Switzerland
        (Switzerland, Dividend) => dec!(0.35),
        (Switzerland, Interest) => dec!(0.0),
        (Switzerland, Royalty) => dec!(0.0),
        (Switzerland, CapitalGain) => dec!(0.0),
        (Switzerland, RentalIncome) => dec!(0.0),

        // Japan
        (Japan, Dividend) => dec!(0.2042),
        (Japan, Interest) => dec!(0.15315),
        (Japan, Royalty) => dec!(0.2042),
        (Japan, CapitalGain) => dec!(0.0),
        (Japan, RentalIncome) => dec!(0.2042),

        // Australia
        (Australia, Dividend) => dec!(0.30),
        (Australia, Interest) => dec!(0.10),
        (Australia, Royalty) => dec!(0.30),
        (Australia, CapitalGain) => dec!(0.0),
        (Australia, RentalIncome) => dec!(0.30),

        // Canada
        (Canada, Dividend) => dec!(0.25),
        (Canada, Interest) => dec!(0.25),
        (Canada, Royalty) => dec!(0.25),
        (Canada, CapitalGain) => dec!(0.0),
        (Canada, RentalIncome) => dec!(0.25),

        // Singapore
        (Singapore, Dividend) => dec!(0.0),
        (Singapore, Interest) => dec!(0.15),
        (Singapore, Royalty) => dec!(0.10),
        (Singapore, CapitalGain) => dec!(0.0),
        (Singapore, RentalIncome) => dec!(0.15),

        // Hong Kong
        (HongKong, Dividend) => dec!(0.0),
        (HongKong, Interest) => dec!(0.0),
        (HongKong, Royalty) => dec!(0.0495),
        (HongKong, CapitalGain) => dec!(0.0),
        (HongKong, RentalIncome) => dec!(0.0),

        // Luxembourg
        (Luxembourg, Dividend) => dec!(0.15),
        (Luxembourg, Interest) => dec!(0.0),
        (Luxembourg, Royalty) => dec!(0.0),
        (Luxembourg, CapitalGain) => dec!(0.0),
        (Luxembourg, RentalIncome) => dec!(0.0),

        // Tax-neutral jurisdictions
        (Cayman, _) | (BVI, _) | (Jersey, _) | (Guernsey, _) => dec!(0.0),

        // Netherlands — no WHT on interest/royalties post-2021 conditional WHT
        (Netherlands, Dividend) => dec!(0.15),
        (Netherlands, Interest) => dec!(0.0),
        (Netherlands, Royalty) => dec!(0.0),
        (Netherlands, CapitalGain) => dec!(0.0),
        (Netherlands, RentalIncome) => dec!(0.0),

        // Other / unknown — conservative assumption 0%
        (Other(_), _) => dec!(0.0),
    }
}

// ---------------------------------------------------------------------------
// Treaty rate lookup
// ---------------------------------------------------------------------------

/// Returns the treaty rate for a pair of jurisdictions and income type, if a
/// treaty exists. Uses an ordered pair lookup (symmetric).
fn treaty_rate(
    source: &Jurisdiction,
    investor: &Jurisdiction,
    income_type: &IncomeType,
) -> Option<(Rate, String)> {
    use IncomeType::*;
    use Jurisdiction::*;

    // Helper: normalize pair so we can look up in one direction
    let pair = (source, investor);
    let (rate, treaty_name) = match pair {
        // US treaties
        (US, UK) | (UK, US) => match income_type {
            Dividend => (dec!(0.15), "US-UK Double Taxation Convention"),
            Interest => (dec!(0.0), "US-UK Double Taxation Convention"),
            _ => return None,
        },
        (US, Ireland) | (Ireland, US) => match income_type {
            Dividend => (dec!(0.15), "US-Ireland Income Tax Treaty"),
            Interest => (dec!(0.0), "US-Ireland Income Tax Treaty"),
            _ => return None,
        },
        (US, Luxembourg) | (Luxembourg, US) => match income_type {
            Dividend => (dec!(0.15), "US-Luxembourg Income Tax Treaty"),
            Interest => (dec!(0.0), "US-Luxembourg Income Tax Treaty"),
            _ => return None,
        },
        (US, Switzerland) | (Switzerland, US) => match income_type {
            Dividend => (dec!(0.15), "US-Switzerland Income Tax Treaty"),
            Interest => (dec!(0.0), "US-Switzerland Income Tax Treaty"),
            _ => return None,
        },
        (US, Canada) | (Canada, US) => match income_type {
            Dividend => (dec!(0.15), "US-Canada Income Tax Treaty"),
            Interest => (dec!(0.0), "US-Canada Income Tax Treaty"),
            _ => return None,
        },
        (US, Germany) | (Germany, US) => match income_type {
            Dividend => (dec!(0.15), "US-Germany Income Tax Treaty"),
            Interest => (dec!(0.0), "US-Germany Income Tax Treaty"),
            _ => return None,
        },
        (US, Japan) | (Japan, US) => match income_type {
            Dividend => (dec!(0.10), "US-Japan Income Tax Treaty"),
            Interest => (dec!(0.10), "US-Japan Income Tax Treaty"),
            _ => return None,
        },
        (US, Australia) | (Australia, US) => match income_type {
            Dividend => (dec!(0.15), "US-Australia Income Tax Treaty"),
            Interest => (dec!(0.10), "US-Australia Income Tax Treaty"),
            _ => return None,
        },
        // UK treaties
        (UK, Germany) | (Germany, UK) => match income_type {
            Dividend => (dec!(0.10), "UK-Germany Double Taxation Convention"),
            Interest => (dec!(0.0), "UK-Germany Double Taxation Convention"),
            _ => return None,
        },
        (UK, France) | (France, UK) => match income_type {
            Dividend => (dec!(0.15), "UK-France Double Taxation Convention"),
            Interest => (dec!(0.0), "UK-France Double Taxation Convention"),
            _ => return None,
        },
        _ => return None,
    };

    Some((rate, treaty_name.to_string()))
}

// ---------------------------------------------------------------------------
// Tax-neutral jurisdiction helpers
// ---------------------------------------------------------------------------

fn is_tax_neutral(j: &Jurisdiction) -> bool {
    matches!(
        j,
        Jurisdiction::Cayman | Jurisdiction::BVI | Jurisdiction::Jersey | Jurisdiction::Guernsey
    )
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Calculate withholding tax for a single income stream.
pub fn calculate_withholding_tax(
    input: &WhtInput,
) -> CorpFinanceResult<ComputationOutput<WhtOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();
    let mut notes: Vec<String> = Vec::new();

    // Validation
    if input.gross_income <= dec!(0) {
        return Err(CorpFinanceError::InvalidInput {
            field: "gross_income".to_string(),
            reason: "Gross income must be positive".to_string(),
        });
    }

    if input.source_jurisdiction == input.investor_jurisdiction {
        warnings.push(
            "Source and investor jurisdictions are the same — this is domestic income, \
             not subject to withholding tax in most cases."
                .to_string(),
        );
    }

    // Statutory rate
    let stat_rate = statutory_rate(&input.source_jurisdiction, &input.income_type);

    // Treaty rate
    let treaty_lookup = treaty_rate(
        &input.source_jurisdiction,
        &input.investor_jurisdiction,
        &input.income_type,
    );

    let (treaty_rate_val, treaty_name) = match &treaty_lookup {
        Some((r, name)) => (Some(*r), Some(name.clone())),
        None => (None, None),
    };

    // Effective rate = min(statutory, treaty) where treaty exists
    let effective_rate = match treaty_rate_val {
        Some(tr) if tr < stat_rate => {
            notes.push(format!(
                "Treaty rate ({:.2}%) applied, lower than statutory rate ({:.2}%)",
                tr * dec!(100),
                stat_rate * dec!(100),
            ));
            tr
        }
        Some(_) => stat_rate,
        None => stat_rate,
    };

    // Tax-exempt investor handling
    if input.is_tax_exempt_investor {
        notes.push(
            "Investor is tax-exempt (e.g., pension fund, sovereign). Reduced rates \
             may be available depending on jurisdiction-specific exemptions. \
             Consult local tax counsel."
                .to_string(),
        );
    }

    // Blocker recommendation
    let blocker_recommendation = if input.source_jurisdiction == Jurisdiction::US
        && is_tax_neutral(&input.investor_jurisdiction)
    {
        Some(
            "Consider using a US blocker corporation to avoid ECI/UBTI exposure. \
                 A blocker converts pass-through income to corporate dividends, potentially \
                 subject to treaty-reduced WHT if structured through a treaty jurisdiction."
                .to_string(),
        )
    } else {
        None
    };

    let withholding_amount = input.gross_income * effective_rate;
    let net_income = input.gross_income - withholding_amount;

    let result = WhtOutput {
        statutory_rate: stat_rate,
        treaty_rate: treaty_rate_val,
        effective_rate,
        withholding_amount,
        net_income,
        treaty_name,
        notes,
        blocker_recommendation,
    };

    let assumptions = serde_json::json!({
        "source_jurisdiction": input.source_jurisdiction,
        "investor_jurisdiction": input.investor_jurisdiction,
        "income_type": input.income_type,
        "gross_income": input.gross_income.to_string(),
        "is_tax_exempt_investor": input.is_tax_exempt_investor,
    });

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Withholding tax calculation using statutory and treaty rate lookups",
        &assumptions,
        warnings,
        elapsed,
        result,
    ))
}

/// Calculate withholding tax for a portfolio of holdings and provide optimisation
/// suggestions.
pub fn calculate_portfolio_wht(
    input: &PortfolioWhtInput,
) -> CorpFinanceResult<ComputationOutput<PortfolioWhtOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    if input.holdings.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "Portfolio must contain at least one holding".to_string(),
        ));
    }

    let mut per_holding: Vec<WhtOutput> = Vec::new();
    let mut total_gross = dec!(0);
    let mut total_wht = dec!(0);

    for (i, holding) in input.holdings.iter().enumerate() {
        match calculate_withholding_tax(holding) {
            Ok(output) => {
                // Carry per-holding warnings up to portfolio level
                for w in &output.warnings {
                    warnings.push(format!("Holding {}: {}", i + 1, w));
                }
                total_gross += holding.gross_income;
                total_wht += output.result.withholding_amount;
                per_holding.push(output.result);
            }
            Err(e) => {
                return Err(e);
            }
        }
    }

    let total_net = total_gross - total_wht;

    let effective_wht_rate = if total_gross > dec!(0) {
        total_wht / total_gross
    } else {
        dec!(0)
    };

    // WHT drag on return is the same as effective rate for pre-tax returns
    let wht_drag_on_return = effective_wht_rate;

    // Optimisation suggestions
    let mut optimisation_suggestions: Vec<String> = Vec::new();

    // Check if any US-sourced holdings could benefit from treaty restructuring
    let has_us_sourced = input.holdings.iter().any(|h| {
        h.source_jurisdiction == Jurisdiction::US && is_tax_neutral(&h.investor_jurisdiction)
    });
    if has_us_sourced {
        optimisation_suggestions.push(
            "Consider Luxembourg or Ireland fund vehicle to benefit from US-LUX or US-IE \
             treaty (reduces US dividend WHT from 30% to 15%)."
                .to_string(),
        );
    }

    // Check for high Swiss WHT
    let has_swiss_dividends = input.holdings.iter().any(|h| {
        h.source_jurisdiction == Jurisdiction::Switzerland
            && matches!(h.income_type, IncomeType::Dividend)
    });
    if has_swiss_dividends {
        optimisation_suggestions.push(
            "Swiss dividend WHT is 35%. Consider treaty jurisdictions (e.g., UK at 15% \
             under UK-Switzerland treaty) to reduce effective rate."
                .to_string(),
        );
    }

    // Check for German dividend WHT
    let has_german_dividends = input.holdings.iter().any(|h| {
        h.source_jurisdiction == Jurisdiction::Germany
            && matches!(h.income_type, IncomeType::Dividend)
    });
    if has_german_dividends {
        optimisation_suggestions.push(
            "German dividend WHT is 26.375% (incl. solidarity surcharge). UK investors \
             benefit from UK-Germany treaty reducing rate to 10%."
                .to_string(),
        );
    }

    let result = PortfolioWhtOutput {
        total_gross_income: total_gross,
        total_wht,
        total_net_income: total_net,
        effective_wht_rate,
        wht_drag_on_return,
        per_holding,
        optimisation_suggestions,
    };

    let assumptions = serde_json::json!({
        "num_holdings": input.holdings.len(),
        "total_gross_income": total_gross.to_string(),
    });

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Portfolio withholding tax analysis with optimisation suggestions",
        &assumptions,
        warnings,
        elapsed,
        result,
    ))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn simple_input(
        source: Jurisdiction,
        investor: Jurisdiction,
        income_type: IncomeType,
        gross_income: Money,
    ) -> WhtInput {
        WhtInput {
            source_jurisdiction: source,
            investor_jurisdiction: investor,
            fund_jurisdiction: None,
            income_type,
            gross_income,
            is_tax_exempt_investor: false,
            currency: None,
        }
    }

    #[test]
    fn test_us_dividend_to_uk_investor() {
        let input = simple_input(
            Jurisdiction::US,
            Jurisdiction::UK,
            IncomeType::Dividend,
            dec!(1_000_000),
        );
        let output = calculate_withholding_tax(&input).unwrap();
        let r = &output.result;

        assert_eq!(r.statutory_rate, dec!(0.30));
        assert_eq!(r.treaty_rate, Some(dec!(0.15)));
        assert_eq!(r.effective_rate, dec!(0.15));
        assert_eq!(r.withholding_amount, dec!(150_000));
        assert_eq!(r.net_income, dec!(850_000));
        assert!(r.treaty_name.as_ref().unwrap().contains("US-UK"));
    }

    #[test]
    fn test_us_interest_to_ireland() {
        let input = simple_input(
            Jurisdiction::US,
            Jurisdiction::Ireland,
            IncomeType::Interest,
            dec!(500_000),
        );
        let output = calculate_withholding_tax(&input).unwrap();
        let r = &output.result;

        assert_eq!(r.statutory_rate, dec!(0.30));
        assert_eq!(r.treaty_rate, Some(dec!(0.0)));
        assert_eq!(r.effective_rate, dec!(0.0));
        assert_eq!(r.withholding_amount, dec!(0));
        assert_eq!(r.net_income, dec!(500_000));
    }

    #[test]
    fn test_cayman_no_wht() {
        let input = simple_input(
            Jurisdiction::Cayman,
            Jurisdiction::US,
            IncomeType::Dividend,
            dec!(1_000_000),
        );
        let output = calculate_withholding_tax(&input).unwrap();
        let r = &output.result;

        assert_eq!(r.statutory_rate, dec!(0.0));
        assert_eq!(r.effective_rate, dec!(0.0));
        assert_eq!(r.withholding_amount, dec!(0));
        assert_eq!(r.net_income, dec!(1_000_000));
    }

    #[test]
    fn test_uk_dividend_no_wht() {
        let input = simple_input(
            Jurisdiction::UK,
            Jurisdiction::US,
            IncomeType::Dividend,
            dec!(750_000),
        );
        let output = calculate_withholding_tax(&input).unwrap();
        let r = &output.result;

        assert_eq!(r.statutory_rate, dec!(0.0));
        assert_eq!(r.effective_rate, dec!(0.0));
        assert_eq!(r.net_income, dec!(750_000));
    }

    #[test]
    fn test_switzerland_dividend_high_statutory() {
        let input = simple_input(
            Jurisdiction::Switzerland,
            Jurisdiction::Cayman,
            IncomeType::Dividend,
            dec!(1_000_000),
        );
        let output = calculate_withholding_tax(&input).unwrap();
        let r = &output.result;

        assert_eq!(r.statutory_rate, dec!(0.35));
        assert_eq!(r.treaty_rate, None);
        assert_eq!(r.effective_rate, dec!(0.35));
        assert_eq!(r.withholding_amount, dec!(350_000));
        assert_eq!(r.net_income, dec!(650_000));
    }

    #[test]
    fn test_us_dividend_to_cayman_no_treaty() {
        let input = simple_input(
            Jurisdiction::US,
            Jurisdiction::Cayman,
            IncomeType::Dividend,
            dec!(1_000_000),
        );
        let output = calculate_withholding_tax(&input).unwrap();
        let r = &output.result;

        assert_eq!(r.statutory_rate, dec!(0.30));
        assert_eq!(r.treaty_rate, None);
        assert_eq!(r.effective_rate, dec!(0.30));
        assert_eq!(r.withholding_amount, dec!(300_000));
    }

    #[test]
    fn test_tax_exempt_investor_note() {
        let input = WhtInput {
            source_jurisdiction: Jurisdiction::US,
            investor_jurisdiction: Jurisdiction::UK,
            fund_jurisdiction: None,
            income_type: IncomeType::Dividend,
            gross_income: dec!(1_000_000),
            is_tax_exempt_investor: true,
            currency: None,
        };
        let output = calculate_withholding_tax(&input).unwrap();
        let r = &output.result;

        assert!(r.notes.iter().any(|n| n.contains("tax-exempt")));
    }

    #[test]
    fn test_blocker_recommendation_us_to_cayman() {
        let input = simple_input(
            Jurisdiction::US,
            Jurisdiction::Cayman,
            IncomeType::Dividend,
            dec!(1_000_000),
        );
        let output = calculate_withholding_tax(&input).unwrap();
        let r = &output.result;

        assert!(r.blocker_recommendation.is_some());
        assert!(r
            .blocker_recommendation
            .as_ref()
            .unwrap()
            .contains("blocker corporation"));
    }

    #[test]
    fn test_domestic_income_warning() {
        let input = simple_input(
            Jurisdiction::US,
            Jurisdiction::US,
            IncomeType::Dividend,
            dec!(1_000_000),
        );
        let output = calculate_withholding_tax(&input).unwrap();

        assert!(output
            .warnings
            .iter()
            .any(|w| w.contains("domestic income")));
    }

    #[test]
    fn test_portfolio_wht_calculation() {
        let portfolio = PortfolioWhtInput {
            holdings: vec![
                simple_input(
                    Jurisdiction::US,
                    Jurisdiction::UK,
                    IncomeType::Dividend,
                    dec!(1_000_000),
                ),
                simple_input(
                    Jurisdiction::Cayman,
                    Jurisdiction::UK,
                    IncomeType::Dividend,
                    dec!(500_000),
                ),
            ],
        };
        let output = calculate_portfolio_wht(&portfolio).unwrap();
        let r = &output.result;

        assert_eq!(r.total_gross_income, dec!(1_500_000));
        // US holding: 15% treaty rate = 150,000 WHT; Cayman: 0
        assert_eq!(r.total_wht, dec!(150_000));
        assert_eq!(r.total_net_income, dec!(1_350_000));
        assert_eq!(r.per_holding.len(), 2);
    }

    #[test]
    fn test_portfolio_effective_rate() {
        let portfolio = PortfolioWhtInput {
            holdings: vec![
                simple_input(
                    Jurisdiction::US,
                    Jurisdiction::UK,
                    IncomeType::Dividend,
                    dec!(1_000_000),
                ),
                simple_input(
                    Jurisdiction::Cayman,
                    Jurisdiction::UK,
                    IncomeType::Dividend,
                    dec!(1_000_000),
                ),
            ],
        };
        let output = calculate_portfolio_wht(&portfolio).unwrap();
        let r = &output.result;

        // 150k WHT on 2M gross = 7.5% blended
        assert_eq!(r.effective_wht_rate, dec!(0.075));
    }

    #[test]
    fn test_zero_gross_income_error() {
        let input = simple_input(
            Jurisdiction::US,
            Jurisdiction::UK,
            IncomeType::Dividend,
            dec!(0),
        );
        let result = calculate_withholding_tax(&input);

        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "gross_income");
            }
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }

    #[test]
    fn test_capital_gains_typically_zero() {
        // Capital gains WHT is 0% in most jurisdictions
        for source in [
            Jurisdiction::US,
            Jurisdiction::UK,
            Jurisdiction::Germany,
            Jurisdiction::France,
            Jurisdiction::Japan,
            Jurisdiction::Cayman,
        ] {
            let input = simple_input(
                source.clone(),
                Jurisdiction::UK,
                IncomeType::CapitalGain,
                dec!(1_000_000),
            );
            let output = calculate_withholding_tax(&input).unwrap();
            assert_eq!(
                output.result.effective_rate,
                dec!(0.0),
                "Expected 0% capital gains WHT for {:?}",
                source
            );
        }
    }

    #[test]
    fn test_metadata_populated() {
        let input = simple_input(
            Jurisdiction::US,
            Jurisdiction::UK,
            IncomeType::Dividend,
            dec!(1_000_000),
        );
        let output = calculate_withholding_tax(&input).unwrap();

        assert!(!output.methodology.is_empty());
        assert_eq!(output.metadata.precision, "rust_decimal_128bit");
        assert!(!output.metadata.version.is_empty());
    }

    #[test]
    fn test_us_japan_treaty_rates() {
        let input = simple_input(
            Jurisdiction::US,
            Jurisdiction::Japan,
            IncomeType::Dividend,
            dec!(1_000_000),
        );
        let output = calculate_withholding_tax(&input).unwrap();
        let r = &output.result;

        assert_eq!(r.treaty_rate, Some(dec!(0.10)));
        assert_eq!(r.effective_rate, dec!(0.10));
        assert_eq!(r.withholding_amount, dec!(100_000));
    }

    #[test]
    fn test_portfolio_optimisation_suggestions() {
        let portfolio = PortfolioWhtInput {
            holdings: vec![
                simple_input(
                    Jurisdiction::US,
                    Jurisdiction::Cayman,
                    IncomeType::Dividend,
                    dec!(1_000_000),
                ),
                simple_input(
                    Jurisdiction::Switzerland,
                    Jurisdiction::Cayman,
                    IncomeType::Dividend,
                    dec!(500_000),
                ),
            ],
        };
        let output = calculate_portfolio_wht(&portfolio).unwrap();
        let r = &output.result;

        // Should suggest Luxembourg vehicle for US-sourced
        assert!(r
            .optimisation_suggestions
            .iter()
            .any(|s| s.contains("Luxembourg")));
        // Should suggest treaty for Swiss dividends
        assert!(r
            .optimisation_suggestions
            .iter()
            .any(|s| s.contains("Swiss dividend")));
    }
}
