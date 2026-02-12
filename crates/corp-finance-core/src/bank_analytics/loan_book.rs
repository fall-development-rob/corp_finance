//! Loan book analytics.
//!
//! Covers:
//! 1. **Portfolio summary** -- total balance, performing %, NPL ratio.
//! 2. **Coverage ratio** -- total provisions / NPL balance.
//! 3. **Weighted averages** -- interest rate, maturity.
//! 4. **Concentration risk** -- sector/geography HHI, risk classification.
//! 5. **Vintage analysis** -- maturity bucket grouping.
//! 6. **Status breakdown** -- by loan classification.
//!
//! All arithmetic uses `rust_decimal::Decimal`. No `f64`.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Input / Output
// ---------------------------------------------------------------------------

/// A single loan in the book.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoanDetail {
    pub id: String,
    pub balance: Decimal,
    pub sector: String,
    pub geography: String,
    /// Status: "performing", "watchlist", "substandard", "doubtful", "loss".
    pub status: String,
    pub provision: Decimal,
    pub interest_rate: Decimal,
    pub maturity_years: Decimal,
}

/// A concentration item (sector or geography).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConcentrationItem {
    pub name: String,
    pub balance: Decimal,
    pub pct: Decimal,
    pub count: u64,
}

/// A vintage bucket summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VintageBucket {
    pub bucket: String,
    pub count: u64,
    pub balance: Decimal,
    pub pct: Decimal,
}

/// A status breakdown item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusItem {
    pub status: String,
    pub count: u64,
    pub balance: Decimal,
    pub pct: Decimal,
}

/// Input for loan book analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoanBookInput {
    pub loans: Vec<LoanDetail>,
}

/// Output of loan book analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoanBookOutput {
    pub total_portfolio: Decimal,
    pub performing_pct: Decimal,
    pub npl_ratio: Decimal,
    pub coverage_ratio: Decimal,
    pub weighted_avg_rate: Decimal,
    pub weighted_avg_maturity: Decimal,
    pub sector_concentration: Vec<ConcentrationItem>,
    pub geography_concentration: Vec<ConcentrationItem>,
    pub sector_hhi: Decimal,
    pub geography_hhi: Decimal,
    pub concentration_risk: String,
    pub vintage_summary: Vec<VintageBucket>,
    pub status_breakdown: Vec<StatusItem>,
}

// ---------------------------------------------------------------------------
// Core function
// ---------------------------------------------------------------------------

/// Analyze a loan book for portfolio metrics, concentration, and vintage.
pub fn analyze_loan_book(input: &LoanBookInput) -> CorpFinanceResult<LoanBookOutput> {
    validate_loan_book_input(input)?;

    let total_portfolio: Decimal = input.loans.iter().map(|l| l.balance).sum();

    if total_portfolio == Decimal::ZERO {
        return Err(CorpFinanceError::DivisionByZero {
            context: "Total portfolio balance is zero.".into(),
        });
    }

    // Performing = performing + watchlist
    let performing_balance: Decimal = input
        .loans
        .iter()
        .filter(|l| l.status == "performing" || l.status == "watchlist")
        .map(|l| l.balance)
        .sum();
    let performing_pct = performing_balance / total_portfolio;

    // NPL = substandard + doubtful + loss
    let npl_balance: Decimal = input
        .loans
        .iter()
        .filter(|l| l.status == "substandard" || l.status == "doubtful" || l.status == "loss")
        .map(|l| l.balance)
        .sum();
    let npl_ratio = npl_balance / total_portfolio;

    // Coverage ratio = total provisions / NPL balance
    let total_provisions: Decimal = input.loans.iter().map(|l| l.provision).sum();
    let coverage_ratio = if npl_balance > Decimal::ZERO {
        total_provisions / npl_balance
    } else {
        Decimal::ZERO
    };

    // Weighted average rate
    let weighted_rate_sum: Decimal = input
        .loans
        .iter()
        .map(|l| l.balance * l.interest_rate)
        .sum();
    let weighted_avg_rate = weighted_rate_sum / total_portfolio;

    // Weighted average maturity
    let weighted_maturity_sum: Decimal = input
        .loans
        .iter()
        .map(|l| l.balance * l.maturity_years)
        .sum();
    let weighted_avg_maturity = weighted_maturity_sum / total_portfolio;

    // Sector concentration
    let sector_concentration =
        build_concentration(&input.loans, total_portfolio, |l| l.sector.clone());
    let sector_hhi = calculate_hhi(&sector_concentration);

    // Geography concentration
    let geography_concentration =
        build_concentration(&input.loans, total_portfolio, |l| l.geography.clone());
    let geography_hhi = calculate_hhi(&geography_concentration);

    // Concentration risk: use max of sector and geography HHI
    let max_hhi = if sector_hhi > geography_hhi {
        sector_hhi
    } else {
        geography_hhi
    };
    let concentration_risk = if max_hhi < dec!(1500) {
        "Low".to_string()
    } else if max_hhi < dec!(2500) {
        "Moderate".to_string()
    } else {
        "High".to_string()
    };

    // Vintage summary by maturity buckets
    let vintage_summary = build_vintage_summary(&input.loans, total_portfolio);

    // Status breakdown
    let status_breakdown = build_status_breakdown(&input.loans, total_portfolio);

    Ok(LoanBookOutput {
        total_portfolio,
        performing_pct,
        npl_ratio,
        coverage_ratio,
        weighted_avg_rate,
        weighted_avg_maturity,
        sector_concentration,
        geography_concentration,
        sector_hhi,
        geography_hhi,
        concentration_risk,
        vintage_summary,
        status_breakdown,
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn build_concentration<F>(loans: &[LoanDetail], total: Decimal, key_fn: F) -> Vec<ConcentrationItem>
where
    F: Fn(&LoanDetail) -> String,
{
    let mut map: BTreeMap<String, (Decimal, u64)> = BTreeMap::new();
    for loan in loans {
        let key = key_fn(loan);
        let entry = map.entry(key).or_insert((Decimal::ZERO, 0));
        entry.0 += loan.balance;
        entry.1 += 1;
    }
    let mut items: Vec<ConcentrationItem> = map
        .into_iter()
        .map(|(name, (balance, count))| ConcentrationItem {
            name,
            balance,
            pct: balance / total,
            count,
        })
        .collect();
    // Sort by balance descending
    items.sort_by(|a, b| b.balance.cmp(&a.balance));
    items
}

fn calculate_hhi(items: &[ConcentrationItem]) -> Decimal {
    // HHI = sum of (share in percentage points)^2
    // Share = pct * 100, then square
    items
        .iter()
        .map(|item| {
            let share_pct = item.pct * dec!(100);
            share_pct * share_pct
        })
        .sum()
}

fn maturity_bucket(years: Decimal) -> String {
    if years <= Decimal::ONE {
        "0-1yr".to_string()
    } else if years <= dec!(3) {
        "1-3yr".to_string()
    } else if years <= dec!(5) {
        "3-5yr".to_string()
    } else {
        "5yr+".to_string()
    }
}

fn build_vintage_summary(loans: &[LoanDetail], total: Decimal) -> Vec<VintageBucket> {
    let buckets_order = ["0-1yr", "1-3yr", "3-5yr", "5yr+"];
    let mut map: BTreeMap<String, (u64, Decimal)> = BTreeMap::new();
    for bucket in &buckets_order {
        map.insert(bucket.to_string(), (0, Decimal::ZERO));
    }
    for loan in loans {
        let bucket = maturity_bucket(loan.maturity_years);
        let entry = map.entry(bucket).or_insert((0, Decimal::ZERO));
        entry.0 += 1;
        entry.1 += loan.balance;
    }
    buckets_order
        .iter()
        .map(|bucket_name| {
            let (count, balance) = map.get(*bucket_name).copied().unwrap_or((0, Decimal::ZERO));
            VintageBucket {
                bucket: bucket_name.to_string(),
                count,
                balance,
                pct: if total > Decimal::ZERO {
                    balance / total
                } else {
                    Decimal::ZERO
                },
            }
        })
        .collect()
}

fn build_status_breakdown(loans: &[LoanDetail], total: Decimal) -> Vec<StatusItem> {
    let status_order = ["performing", "watchlist", "substandard", "doubtful", "loss"];
    let mut map: BTreeMap<String, (u64, Decimal)> = BTreeMap::new();
    for status in &status_order {
        map.insert(status.to_string(), (0, Decimal::ZERO));
    }
    for loan in loans {
        let entry = map.entry(loan.status.clone()).or_insert((0, Decimal::ZERO));
        entry.0 += 1;
        entry.1 += loan.balance;
    }
    status_order
        .iter()
        .map(|status_name| {
            let (count, balance) = map.get(*status_name).copied().unwrap_or((0, Decimal::ZERO));
            StatusItem {
                status: status_name.to_string(),
                count,
                balance,
                pct: if total > Decimal::ZERO {
                    balance / total
                } else {
                    Decimal::ZERO
                },
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_loan_book_input(input: &LoanBookInput) -> CorpFinanceResult<()> {
    if input.loans.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "Loan book must contain at least one loan.".into(),
        ));
    }
    let valid_statuses = ["performing", "watchlist", "substandard", "doubtful", "loss"];
    for loan in &input.loans {
        if loan.balance < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "balance".into(),
                reason: format!("Loan '{}' has negative balance.", loan.id),
            });
        }
        if loan.provision < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "provision".into(),
                reason: format!("Loan '{}' has negative provision.", loan.id),
            });
        }
        if loan.maturity_years < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "maturity_years".into(),
                reason: format!("Loan '{}' has negative maturity.", loan.id),
            });
        }
        if !valid_statuses.contains(&loan.status.as_str()) {
            return Err(CorpFinanceError::InvalidInput {
                field: "status".into(),
                reason: format!(
                    "Loan '{}' has invalid status '{}'. Valid: {:?}.",
                    loan.id, loan.status, valid_statuses
                ),
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

    fn diversified_book() -> LoanBookInput {
        LoanBookInput {
            loans: vec![
                LoanDetail {
                    id: "L001".into(),
                    balance: dec!(10_000_000),
                    sector: "CRE".into(),
                    geography: "Northeast".into(),
                    status: "performing".into(),
                    provision: dec!(100_000),
                    interest_rate: dec!(0.055),
                    maturity_years: dec!(7),
                },
                LoanDetail {
                    id: "L002".into(),
                    balance: dec!(8_000_000),
                    sector: "C&I".into(),
                    geography: "Southeast".into(),
                    status: "performing".into(),
                    provision: dec!(80_000),
                    interest_rate: dec!(0.06),
                    maturity_years: dec!(3),
                },
                LoanDetail {
                    id: "L003".into(),
                    balance: dec!(6_000_000),
                    sector: "Residential".into(),
                    geography: "West".into(),
                    status: "performing".into(),
                    provision: dec!(60_000),
                    interest_rate: dec!(0.045),
                    maturity_years: dec!(15),
                },
                LoanDetail {
                    id: "L004".into(),
                    balance: dec!(4_000_000),
                    sector: "Consumer".into(),
                    geography: "Midwest".into(),
                    status: "watchlist".into(),
                    provision: dec!(200_000),
                    interest_rate: dec!(0.07),
                    maturity_years: dec!(2),
                },
                LoanDetail {
                    id: "L005".into(),
                    balance: dec!(2_000_000),
                    sector: "CRE".into(),
                    geography: "Northeast".into(),
                    status: "substandard".into(),
                    provision: dec!(500_000),
                    interest_rate: dec!(0.065),
                    maturity_years: dec!(4),
                },
            ],
        }
    }

    fn concentrated_book() -> LoanBookInput {
        LoanBookInput {
            loans: vec![
                LoanDetail {
                    id: "L001".into(),
                    balance: dec!(80_000_000),
                    sector: "CRE".into(),
                    geography: "NYC".into(),
                    status: "performing".into(),
                    provision: dec!(800_000),
                    interest_rate: dec!(0.05),
                    maturity_years: dec!(5),
                },
                LoanDetail {
                    id: "L002".into(),
                    balance: dec!(15_000_000),
                    sector: "CRE".into(),
                    geography: "NYC".into(),
                    status: "performing".into(),
                    provision: dec!(150_000),
                    interest_rate: dec!(0.055),
                    maturity_years: dec!(7),
                },
                LoanDetail {
                    id: "L003".into(),
                    balance: dec!(5_000_000),
                    sector: "C&I".into(),
                    geography: "Boston".into(),
                    status: "substandard".into(),
                    provision: dec!(2_000_000),
                    interest_rate: dec!(0.06),
                    maturity_years: dec!(3),
                },
            ],
        }
    }

    #[test]
    fn test_total_portfolio() {
        let input = diversified_book();
        let out = analyze_loan_book(&input).unwrap();
        assert_eq!(out.total_portfolio, dec!(30_000_000));
    }

    #[test]
    fn test_performing_pct() {
        let input = diversified_book();
        let out = analyze_loan_book(&input).unwrap();
        // Performing + watchlist = 10M + 8M + 6M + 4M = 28M / 30M
        let expected = dec!(28_000_000) / dec!(30_000_000);
        assert!(approx_eq(out.performing_pct, expected, dec!(0.0001)));
    }

    #[test]
    fn test_npl_ratio() {
        let input = diversified_book();
        let out = analyze_loan_book(&input).unwrap();
        // NPL = 2M (substandard) / 30M
        let expected = dec!(2_000_000) / dec!(30_000_000);
        assert!(approx_eq(out.npl_ratio, expected, dec!(0.0001)));
    }

    #[test]
    fn test_coverage_ratio() {
        let input = diversified_book();
        let out = analyze_loan_book(&input).unwrap();
        // Total provisions = 940,000; NPL balance = 2,000,000
        // Coverage = 940,000 / 2,000,000 = 0.47
        assert!(approx_eq(out.coverage_ratio, dec!(0.47), dec!(0.001)));
    }

    #[test]
    fn test_weighted_avg_rate() {
        let input = diversified_book();
        let out = analyze_loan_book(&input).unwrap();
        // (10M*5.5% + 8M*6% + 6M*4.5% + 4M*7% + 2M*6.5%) / 30M
        let weighted =
            dec!(550_000) + dec!(480_000) + dec!(270_000) + dec!(280_000) + dec!(130_000);
        let expected = weighted / dec!(30_000_000);
        assert!(approx_eq(out.weighted_avg_rate, expected, dec!(0.0001)));
    }

    #[test]
    fn test_weighted_avg_maturity() {
        let input = diversified_book();
        let out = analyze_loan_book(&input).unwrap();
        // (10M*7 + 8M*3 + 6M*15 + 4M*2 + 2M*4) / 30M
        let weighted = dec!(70_000_000)
            + dec!(24_000_000)
            + dec!(90_000_000)
            + dec!(8_000_000)
            + dec!(8_000_000);
        let expected = weighted / dec!(30_000_000);
        assert!(approx_eq(out.weighted_avg_maturity, expected, dec!(0.01)));
    }

    #[test]
    fn test_sector_concentration_count() {
        let input = diversified_book();
        let out = analyze_loan_book(&input).unwrap();
        // Sectors: CRE (2 loans), C&I (1), Residential (1), Consumer (1)
        assert_eq!(out.sector_concentration.len(), 4);
    }

    #[test]
    fn test_sector_hhi_diversified() {
        let input = diversified_book();
        let out = analyze_loan_book(&input).unwrap();
        // Shares: CRE=40%, C&I=26.67%, Res=20%, Consumer=13.33%
        // HHI = 1600 + 711 + 400 + 178 = ~2889
        assert!(
            out.sector_hhi > dec!(1500),
            "HHI should reflect some concentration"
        );
    }

    #[test]
    fn test_geography_concentration() {
        let input = diversified_book();
        let out = analyze_loan_book(&input).unwrap();
        assert_eq!(out.geography_concentration.len(), 4);
    }

    #[test]
    fn test_hhi_concentrated_book() {
        let input = concentrated_book();
        let out = analyze_loan_book(&input).unwrap();
        // CRE = 95%, C&I = 5% => HHI = 9025 + 25 = 9050
        assert!(out.sector_hhi > dec!(8000));
        assert_eq!(out.concentration_risk, "High");
    }

    #[test]
    fn test_low_concentration_risk() {
        // 10 equal sectors
        let mut loans = Vec::new();
        for i in 0..10 {
            loans.push(LoanDetail {
                id: format!("L{:03}", i),
                balance: dec!(1_000_000),
                sector: format!("Sector{}", i),
                geography: format!("Geo{}", i),
                status: "performing".into(),
                provision: dec!(10_000),
                interest_rate: dec!(0.05),
                maturity_years: dec!(3),
            });
        }
        let input = LoanBookInput { loans };
        let out = analyze_loan_book(&input).unwrap();
        // HHI = 10 * (10)^2 = 1000
        assert_eq!(out.sector_hhi, dec!(1000));
        assert_eq!(out.concentration_risk, "Low");
    }

    #[test]
    fn test_vintage_buckets() {
        let input = diversified_book();
        let out = analyze_loan_book(&input).unwrap();
        assert_eq!(out.vintage_summary.len(), 4);
        assert_eq!(out.vintage_summary[0].bucket, "0-1yr");
        assert_eq!(out.vintage_summary[1].bucket, "1-3yr");
        assert_eq!(out.vintage_summary[2].bucket, "3-5yr");
        assert_eq!(out.vintage_summary[3].bucket, "5yr+");
    }

    #[test]
    fn test_vintage_bucket_assignment() {
        let input = diversified_book();
        let out = analyze_loan_book(&input).unwrap();
        // 2yr -> 1-3yr, 3yr -> 1-3yr, 4yr -> 3-5yr, 7yr -> 5yr+, 15yr -> 5yr+
        let bucket_1_3 = out
            .vintage_summary
            .iter()
            .find(|b| b.bucket == "1-3yr")
            .unwrap();
        // L002 (3yr, 8M) + L004 (2yr, 4M) = 12M
        assert_eq!(bucket_1_3.balance, dec!(12_000_000));
        assert_eq!(bucket_1_3.count, 2);
    }

    #[test]
    fn test_status_breakdown() {
        let input = diversified_book();
        let out = analyze_loan_book(&input).unwrap();
        assert_eq!(out.status_breakdown.len(), 5);
        let performing = out
            .status_breakdown
            .iter()
            .find(|s| s.status == "performing")
            .unwrap();
        assert_eq!(performing.count, 3);
        assert_eq!(performing.balance, dec!(24_000_000));
    }

    #[test]
    fn test_all_performing() {
        let input = LoanBookInput {
            loans: vec![LoanDetail {
                id: "L001".into(),
                balance: dec!(10_000_000),
                sector: "CRE".into(),
                geography: "NYC".into(),
                status: "performing".into(),
                provision: dec!(50_000),
                interest_rate: dec!(0.05),
                maturity_years: dec!(5),
            }],
        };
        let out = analyze_loan_book(&input).unwrap();
        assert_eq!(out.performing_pct, Decimal::ONE);
        assert_eq!(out.npl_ratio, Decimal::ZERO);
        assert_eq!(out.coverage_ratio, Decimal::ZERO); // no NPL
    }

    #[test]
    fn test_all_npl() {
        let input = LoanBookInput {
            loans: vec![
                LoanDetail {
                    id: "L001".into(),
                    balance: dec!(5_000_000),
                    sector: "CRE".into(),
                    geography: "NYC".into(),
                    status: "substandard".into(),
                    provision: dec!(1_000_000),
                    interest_rate: dec!(0.06),
                    maturity_years: dec!(3),
                },
                LoanDetail {
                    id: "L002".into(),
                    balance: dec!(3_000_000),
                    sector: "C&I".into(),
                    geography: "LA".into(),
                    status: "doubtful".into(),
                    provision: dec!(2_000_000),
                    interest_rate: dec!(0.07),
                    maturity_years: dec!(2),
                },
                LoanDetail {
                    id: "L003".into(),
                    balance: dec!(2_000_000),
                    sector: "Consumer".into(),
                    geography: "Chicago".into(),
                    status: "loss".into(),
                    provision: dec!(2_000_000),
                    interest_rate: dec!(0.08),
                    maturity_years: dec!(1),
                },
            ],
        };
        let out = analyze_loan_book(&input).unwrap();
        assert_eq!(out.performing_pct, Decimal::ZERO);
        assert_eq!(out.npl_ratio, Decimal::ONE);
        // Coverage = 5M provisions / 10M NPL = 0.5
        assert_eq!(out.coverage_ratio, dec!(0.5));
    }

    #[test]
    fn test_hhi_single_sector() {
        let input = LoanBookInput {
            loans: vec![LoanDetail {
                id: "L001".into(),
                balance: dec!(10_000_000),
                sector: "CRE".into(),
                geography: "NYC".into(),
                status: "performing".into(),
                provision: dec!(50_000),
                interest_rate: dec!(0.05),
                maturity_years: dec!(5),
            }],
        };
        let out = analyze_loan_book(&input).unwrap();
        // Single sector = 100% share => HHI = 10000
        assert_eq!(out.sector_hhi, dec!(10000));
        assert_eq!(out.concentration_risk, "High");
    }

    #[test]
    fn test_reject_empty_loans() {
        let input = LoanBookInput { loans: vec![] };
        assert!(analyze_loan_book(&input).is_err());
    }

    #[test]
    fn test_reject_negative_balance() {
        let input = LoanBookInput {
            loans: vec![LoanDetail {
                id: "L001".into(),
                balance: dec!(-100),
                sector: "CRE".into(),
                geography: "NYC".into(),
                status: "performing".into(),
                provision: Decimal::ZERO,
                interest_rate: dec!(0.05),
                maturity_years: dec!(5),
            }],
        };
        assert!(analyze_loan_book(&input).is_err());
    }

    #[test]
    fn test_reject_invalid_status() {
        let input = LoanBookInput {
            loans: vec![LoanDetail {
                id: "L001".into(),
                balance: dec!(1_000_000),
                sector: "CRE".into(),
                geography: "NYC".into(),
                status: "unknown".into(),
                provision: Decimal::ZERO,
                interest_rate: dec!(0.05),
                maturity_years: dec!(5),
            }],
        };
        assert!(analyze_loan_book(&input).is_err());
    }

    #[test]
    fn test_reject_negative_provision() {
        let input = LoanBookInput {
            loans: vec![LoanDetail {
                id: "L001".into(),
                balance: dec!(1_000_000),
                sector: "CRE".into(),
                geography: "NYC".into(),
                status: "performing".into(),
                provision: dec!(-100),
                interest_rate: dec!(0.05),
                maturity_years: dec!(5),
            }],
        };
        assert!(analyze_loan_book(&input).is_err());
    }

    #[test]
    fn test_reject_negative_maturity() {
        let input = LoanBookInput {
            loans: vec![LoanDetail {
                id: "L001".into(),
                balance: dec!(1_000_000),
                sector: "CRE".into(),
                geography: "NYC".into(),
                status: "performing".into(),
                provision: Decimal::ZERO,
                interest_rate: dec!(0.05),
                maturity_years: dec!(-1),
            }],
        };
        assert!(analyze_loan_book(&input).is_err());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let input = diversified_book();
        let out = analyze_loan_book(&input).unwrap();
        let json = serde_json::to_string(&out).unwrap();
        let _: LoanBookOutput = serde_json::from_str(&json).unwrap();
    }
}
