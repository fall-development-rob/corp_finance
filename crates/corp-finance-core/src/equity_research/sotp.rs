use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{with_metadata, ComputationOutput};
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Types — Sum-of-the-Parts Valuation
// ---------------------------------------------------------------------------

/// Valuation methodology applied to a segment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ValuationMethod {
    /// Enterprise-Value / EBITDA
    EvEbitda,
    /// Price / Earnings
    PeRatio,
    /// Enterprise-Value / Revenue
    EvRevenue,
    /// Enterprise-Value / EBIT
    EvEbit,
    /// Simplified DCF (perpetuity growth)
    Dcf,
    /// Net-Asset-Value based (e.g. real estate, investment holding)
    NavBased,
}

/// One business segment for SOTP analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentInput {
    /// Segment or subsidiary name
    pub name: String,
    /// Segment revenue
    pub revenue: Decimal,
    /// Segment EBITDA
    pub ebitda: Decimal,
    /// Segment EBIT
    pub ebit: Decimal,
    /// Segment net income (required for PeRatio method)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub net_income: Option<Decimal>,
    /// Segment net assets (required for NavBased method)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assets: Option<Decimal>,
    /// Valuation method to apply
    pub method: ValuationMethod,
    /// Applied multiple (or proxy WACC for Dcf, NAV multiplier for NavBased)
    pub multiple: Decimal,
    /// Comparable company multiple range (low, high)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comparable_range: Option<(Decimal, Decimal)>,
    /// Expected segment revenue growth rate (decimal)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub growth_rate: Option<Decimal>,
    /// EBITDA or profit margin (decimal)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub margin: Option<Decimal>,
}

/// Top-level SOTP input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SotpInput {
    /// Company name
    pub company_name: String,
    /// Business segments to value
    pub segments: Vec<SegmentInput>,
    /// Total debt minus cash
    pub net_debt: Decimal,
    /// Diluted shares outstanding
    pub shares_outstanding: Decimal,
    /// Optional conglomerate / holding company discount (e.g. 0.15 = 15%)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub holding_company_discount: Option<Decimal>,
    /// Minority interest value to subtract
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minority_interests: Option<Decimal>,
    /// Value of unconsolidated equity stakes to add
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unconsolidated_investments: Option<Decimal>,
}

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// Valuation detail for a single segment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentValuation {
    pub name: String,
    pub method: String,
    pub enterprise_value: Decimal,
    pub pct_of_total: Decimal,
    pub implied_ev_ebitda: Decimal,
    pub value_range: Option<(Decimal, Decimal)>,
}

/// Implied multiples for the consolidated entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpliedMultiples {
    pub ev_ebitda: Decimal,
    pub ev_revenue: Decimal,
    pub ev_ebit: Decimal,
    pub pe_ratio: Option<Decimal>,
}

/// Range data for a single segment in the football field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentRange {
    pub name: String,
    pub low: Decimal,
    pub base: Decimal,
    pub high: Decimal,
}

/// Football-field visualisation data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FootballField {
    pub low_value_per_share: Decimal,
    pub base_value_per_share: Decimal,
    pub high_value_per_share: Decimal,
    pub segment_ranges: Vec<SegmentRange>,
}

/// Full SOTP output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SotpOutput {
    pub total_enterprise_value: Decimal,
    pub equity_value: Decimal,
    pub equity_value_per_share: Decimal,
    pub segment_values: Vec<SegmentValuation>,
    pub implied_multiple: ImpliedMultiples,
    pub football_field: FootballField,
    pub conglomerate_discount_applied: Decimal,
    pub sum_check: Decimal,
}

// ---------------------------------------------------------------------------
// Core calculation
// ---------------------------------------------------------------------------

/// Calculate Sum-of-the-Parts valuation.
pub fn calculate_sotp(input: &SotpInput) -> CorpFinanceResult<ComputationOutput<SotpOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // --- Validate inputs ---
    if input.segments.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "SOTP requires at least one segment".into(),
        ));
    }
    if input.shares_outstanding <= dec!(0) {
        return Err(CorpFinanceError::InvalidInput {
            field: "shares_outstanding".into(),
            reason: "must be positive".into(),
        });
    }

    // --- Value each segment ---
    let mut segment_vals: Vec<SegmentValuation> = Vec::new();
    let mut total_ev = dec!(0);
    let mut total_low = dec!(0);
    let mut total_high = dec!(0);
    let mut segment_ranges: Vec<SegmentRange> = Vec::new();

    let mut consolidated_ebitda = dec!(0);
    let mut consolidated_revenue = dec!(0);
    let mut consolidated_ebit = dec!(0);
    let mut consolidated_net_income = dec!(0);
    let mut has_any_net_income = false;

    for seg in &input.segments {
        // Accumulate consolidated metrics
        consolidated_ebitda += seg.ebitda;
        consolidated_revenue += seg.revenue;
        consolidated_ebit += seg.ebit;
        if let Some(ni) = seg.net_income {
            consolidated_net_income += ni;
            has_any_net_income = true;
        }

        let ev = compute_segment_ev(seg, &mut warnings)?;
        total_ev += ev;

        // Compute range
        let (low_ev, high_ev) = compute_segment_range(seg, ev, &mut warnings)?;
        total_low += low_ev;
        total_high += high_ev;

        // Implied EV/EBITDA for this segment
        let implied_ev_ebitda = if seg.ebitda > dec!(0) {
            ev / seg.ebitda
        } else {
            dec!(0)
        };

        let value_range = Some((low_ev, high_ev));

        segment_ranges.push(SegmentRange {
            name: seg.name.clone(),
            low: low_ev,
            base: ev,
            high: high_ev,
        });

        segment_vals.push(SegmentValuation {
            name: seg.name.clone(),
            method: format!("{:?}", seg.method),
            enterprise_value: ev,
            pct_of_total: dec!(0), // filled after totals known
            implied_ev_ebitda,
            value_range,
        });
    }

    // Fill pct_of_total
    if total_ev > dec!(0) {
        for sv in &mut segment_vals {
            sv.pct_of_total = sv.enterprise_value / total_ev * dec!(100);
        }
    }

    // --- Apply holding-company discount ---
    let discount_rate = input.holding_company_discount.unwrap_or(dec!(0));
    if discount_rate < dec!(0) || discount_rate >= dec!(1) {
        return Err(CorpFinanceError::InvalidInput {
            field: "holding_company_discount".into(),
            reason: "must be between 0 and 1 (exclusive)".into(),
        });
    }
    let discount_factor = dec!(1) - discount_rate;
    let discounted_ev = total_ev * discount_factor;
    let discounted_low = total_low * discount_factor;
    let discounted_high = total_high * discount_factor;
    let conglomerate_discount_applied = total_ev - discounted_ev;

    // --- Bridge to equity value ---
    let minority = input.minority_interests.unwrap_or(dec!(0));
    let unconsol = input.unconsolidated_investments.unwrap_or(dec!(0));

    let equity_value = discounted_ev - input.net_debt + unconsol - minority;
    let equity_value_per_share = equity_value / input.shares_outstanding;

    let low_equity = discounted_low - input.net_debt + unconsol - minority;
    let high_equity = discounted_high - input.net_debt + unconsol - minority;
    let low_per_share = low_equity / input.shares_outstanding;
    let high_per_share = high_equity / input.shares_outstanding;

    // --- Implied multiples for consolidated entity ---
    let implied_ev_ebitda = if consolidated_ebitda > dec!(0) {
        discounted_ev / consolidated_ebitda
    } else {
        warnings.push("Consolidated EBITDA is zero; implied EV/EBITDA not meaningful".into());
        dec!(0)
    };
    let implied_ev_revenue = if consolidated_revenue > dec!(0) {
        discounted_ev / consolidated_revenue
    } else {
        warnings.push("Consolidated revenue is zero; implied EV/Revenue not meaningful".into());
        dec!(0)
    };
    let implied_ev_ebit = if consolidated_ebit > dec!(0) {
        discounted_ev / consolidated_ebit
    } else {
        warnings.push("Consolidated EBIT is zero; implied EV/EBIT not meaningful".into());
        dec!(0)
    };
    let implied_pe = if has_any_net_income && consolidated_net_income > dec!(0) {
        Some(equity_value / consolidated_net_income)
    } else {
        None
    };

    let implied_multiple = ImpliedMultiples {
        ev_ebitda: implied_ev_ebitda,
        ev_revenue: implied_ev_revenue,
        ev_ebit: implied_ev_ebit,
        pe_ratio: implied_pe,
    };

    // --- Football field ---
    let football_field = FootballField {
        low_value_per_share: low_per_share,
        base_value_per_share: equity_value_per_share,
        high_value_per_share: high_per_share,
        segment_ranges,
    };

    let output = SotpOutput {
        total_enterprise_value: discounted_ev,
        equity_value,
        equity_value_per_share,
        segment_values: segment_vals,
        implied_multiple,
        football_field,
        conglomerate_discount_applied,
        sum_check: discounted_ev,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Sum-of-the-Parts Valuation",
        &serde_json::json!({
            "segments": input.segments.len(),
            "holding_company_discount": discount_rate,
            "net_debt": input.net_debt.to_string(),
        }),
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Compute the enterprise value for a single segment based on its method.
fn compute_segment_ev(
    seg: &SegmentInput,
    warnings: &mut Vec<String>,
) -> CorpFinanceResult<Decimal> {
    match seg.method {
        ValuationMethod::EvEbitda => Ok(seg.ebitda * seg.multiple),
        ValuationMethod::EvRevenue => Ok(seg.revenue * seg.multiple),
        ValuationMethod::EvEbit => Ok(seg.ebit * seg.multiple),
        ValuationMethod::PeRatio => {
            let ni = seg
                .net_income
                .ok_or_else(|| CorpFinanceError::InvalidInput {
                    field: format!("segments[{}].net_income", seg.name),
                    reason: "net_income required for PeRatio method".into(),
                })?;
            if ni <= dec!(0) {
                return Err(CorpFinanceError::InvalidInput {
                    field: format!("segments[{}].net_income", seg.name),
                    reason: "net_income must be positive for PeRatio method".into(),
                });
            }
            Ok(ni * seg.multiple)
        }
        ValuationMethod::Dcf => {
            // Simplified perpetuity growth model: EBITDA * (1 + g) / (WACC - g)
            // `multiple` is proxy WACC, `growth_rate` is g
            let g = seg.growth_rate.unwrap_or(dec!(0));
            let wacc = seg.multiple; // proxy WACC stored in multiple field
            if wacc <= g {
                return Err(CorpFinanceError::FinancialImpossibility(format!(
                    "Segment '{}': WACC ({}) must exceed growth rate ({}) for DCF perpetuity",
                    seg.name, wacc, g
                )));
            }
            let numerator = seg.ebitda * (dec!(1) + g);
            let denominator = wacc - g;
            Ok(numerator / denominator)
        }
        ValuationMethod::NavBased => {
            let assets = seg.assets.ok_or_else(|| CorpFinanceError::InvalidInput {
                field: format!("segments[{}].assets", seg.name),
                reason: "assets required for NavBased method".into(),
            })?;
            if assets < dec!(0) {
                warnings.push(format!(
                    "Segment '{}': negative asset base {}",
                    seg.name, assets
                ));
            }
            // multiple acts as NAV premium/discount multiplier (1.0 = at NAV)
            Ok(assets * seg.multiple)
        }
    }
}

/// Compute the low and high range for a segment.
/// Uses comparable_range if available, otherwise +/-20% of the base multiple.
fn compute_segment_range(
    seg: &SegmentInput,
    base_ev: Decimal,
    _warnings: &mut Vec<String>,
) -> CorpFinanceResult<(Decimal, Decimal)> {
    if let Some((low_mult, high_mult)) = seg.comparable_range {
        let base_mult = seg.multiple;
        if base_mult == dec!(0) {
            return Ok((base_ev, base_ev));
        }
        let low_ev = base_ev / base_mult * low_mult;
        let high_ev = base_ev / base_mult * high_mult;
        Ok((low_ev, high_ev))
    } else {
        // Default: +/-20%
        let low_ev = base_ev * dec!(0.80);
        let high_ev = base_ev * dec!(1.20);
        Ok((low_ev, high_ev))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn tech_company_input() -> SotpInput {
        SotpInput {
            company_name: "TechCorp".into(),
            segments: vec![
                SegmentInput {
                    name: "Cloud Services".into(),
                    revenue: dec!(5000),
                    ebitda: dec!(1500),
                    ebit: dec!(1200),
                    net_income: Some(dec!(900)),
                    assets: None,
                    method: ValuationMethod::EvEbitda,
                    multiple: dec!(15),
                    comparable_range: Some((dec!(12), dec!(18))),
                    growth_rate: Some(dec!(0.20)),
                    margin: Some(dec!(0.30)),
                },
                SegmentInput {
                    name: "Enterprise Software".into(),
                    revenue: dec!(3000),
                    ebitda: dec!(900),
                    ebit: dec!(750),
                    net_income: Some(dec!(550)),
                    assets: None,
                    method: ValuationMethod::EvRevenue,
                    multiple: dec!(8),
                    comparable_range: Some((dec!(6), dec!(10))),
                    growth_rate: Some(dec!(0.12)),
                    margin: Some(dec!(0.30)),
                },
                SegmentInput {
                    name: "Hardware".into(),
                    revenue: dec!(2000),
                    ebitda: dec!(400),
                    ebit: dec!(300),
                    net_income: Some(dec!(200)),
                    assets: None,
                    method: ValuationMethod::EvEbit,
                    multiple: dec!(10),
                    comparable_range: None,
                    growth_rate: Some(dec!(0.03)),
                    margin: Some(dec!(0.20)),
                },
            ],
            net_debt: dec!(5000),
            shares_outstanding: dec!(100),
            holding_company_discount: None,
            minority_interests: None,
            unconsolidated_investments: None,
        }
    }

    #[test]
    fn test_sotp_tech_company_basic() {
        let input = tech_company_input();
        let result = calculate_sotp(&input).unwrap();
        let out = &result.result;

        // Cloud: 1500 * 15 = 22500
        // Enterprise: 3000 * 8 = 24000
        // Hardware: 300 * 10 = 3000
        let expected_ev = dec!(22500) + dec!(24000) + dec!(3000);
        assert_eq!(out.total_enterprise_value, expected_ev);
        assert_eq!(out.equity_value, expected_ev - dec!(5000));
        assert_eq!(
            out.equity_value_per_share,
            (expected_ev - dec!(5000)) / dec!(100)
        );
        assert_eq!(out.segment_values.len(), 3);
    }

    #[test]
    fn test_sotp_segment_pct_of_total() {
        let input = tech_company_input();
        let result = calculate_sotp(&input).unwrap();
        let out = &result.result;

        let total_pct: Decimal = out.segment_values.iter().map(|s| s.pct_of_total).sum();
        // Should sum to ~100%
        assert!((total_pct - dec!(100)).abs() < dec!(0.01));
    }

    #[test]
    fn test_sotp_with_conglomerate_discount() {
        let mut input = tech_company_input();
        input.holding_company_discount = Some(dec!(0.15));
        let result = calculate_sotp(&input).unwrap();
        let out = &result.result;

        let undiscounted_ev = dec!(22500) + dec!(24000) + dec!(3000); // 49500
        let expected_ev = undiscounted_ev * dec!(0.85);
        assert_eq!(out.total_enterprise_value, expected_ev);
        assert_eq!(
            out.conglomerate_discount_applied,
            undiscounted_ev - expected_ev
        );
    }

    #[test]
    fn test_sotp_with_minority_and_unconsolidated() {
        let mut input = tech_company_input();
        input.minority_interests = Some(dec!(500));
        input.unconsolidated_investments = Some(dec!(1000));
        let result = calculate_sotp(&input).unwrap();
        let out = &result.result;

        let expected_ev = dec!(49500);
        let expected_equity = expected_ev - dec!(5000) + dec!(1000) - dec!(500);
        assert_eq!(out.equity_value, expected_equity);
    }

    #[test]
    fn test_sotp_single_segment() {
        let input = SotpInput {
            company_name: "SimpleCo".into(),
            segments: vec![SegmentInput {
                name: "Core".into(),
                revenue: dec!(1000),
                ebitda: dec!(300),
                ebit: dec!(250),
                net_income: Some(dec!(180)),
                assets: None,
                method: ValuationMethod::EvEbitda,
                multiple: dec!(10),
                comparable_range: None,
                growth_rate: None,
                margin: None,
            }],
            net_debt: dec!(500),
            shares_outstanding: dec!(50),
            holding_company_discount: None,
            minority_interests: None,
            unconsolidated_investments: None,
        };
        let result = calculate_sotp(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.total_enterprise_value, dec!(3000));
        assert_eq!(out.equity_value, dec!(2500));
        assert_eq!(out.equity_value_per_share, dec!(50));
        assert_eq!(out.segment_values[0].pct_of_total, dec!(100));
    }

    #[test]
    fn test_sotp_pe_ratio_method() {
        let input = SotpInput {
            company_name: "PECo".into(),
            segments: vec![SegmentInput {
                name: "Retail".into(),
                revenue: dec!(2000),
                ebitda: dec!(400),
                ebit: dec!(350),
                net_income: Some(dec!(250)),
                assets: None,
                method: ValuationMethod::PeRatio,
                multiple: dec!(20),
                comparable_range: None,
                growth_rate: None,
                margin: None,
            }],
            net_debt: dec!(0),
            shares_outstanding: dec!(100),
            holding_company_discount: None,
            minority_interests: None,
            unconsolidated_investments: None,
        };
        let result = calculate_sotp(&input).unwrap();
        let out = &result.result;
        // 250 * 20 = 5000
        assert_eq!(out.total_enterprise_value, dec!(5000));
    }

    #[test]
    fn test_sotp_pe_ratio_missing_net_income() {
        let input = SotpInput {
            company_name: "BadCo".into(),
            segments: vec![SegmentInput {
                name: "Seg".into(),
                revenue: dec!(1000),
                ebitda: dec!(200),
                ebit: dec!(150),
                net_income: None,
                assets: None,
                method: ValuationMethod::PeRatio,
                multiple: dec!(15),
                comparable_range: None,
                growth_rate: None,
                margin: None,
            }],
            net_debt: dec!(0),
            shares_outstanding: dec!(10),
            holding_company_discount: None,
            minority_interests: None,
            unconsolidated_investments: None,
        };
        let result = calculate_sotp(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_sotp_pe_ratio_negative_net_income() {
        let input = SotpInput {
            company_name: "LossCo".into(),
            segments: vec![SegmentInput {
                name: "Seg".into(),
                revenue: dec!(1000),
                ebitda: dec!(200),
                ebit: dec!(150),
                net_income: Some(dec!(-50)),
                assets: None,
                method: ValuationMethod::PeRatio,
                multiple: dec!(15),
                comparable_range: None,
                growth_rate: None,
                margin: None,
            }],
            net_debt: dec!(0),
            shares_outstanding: dec!(10),
            holding_company_discount: None,
            minority_interests: None,
            unconsolidated_investments: None,
        };
        let result = calculate_sotp(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_sotp_nav_based_method() {
        let input = SotpInput {
            company_name: "REITCo".into(),
            segments: vec![SegmentInput {
                name: "Property Portfolio".into(),
                revenue: dec!(500),
                ebitda: dec!(300),
                ebit: dec!(280),
                net_income: Some(dec!(200)),
                assets: Some(dec!(10000)),
                method: ValuationMethod::NavBased,
                multiple: dec!(1.10), // 10% premium to NAV
                comparable_range: Some((dec!(0.90), dec!(1.20))),
                growth_rate: None,
                margin: None,
            }],
            net_debt: dec!(3000),
            shares_outstanding: dec!(200),
            holding_company_discount: None,
            minority_interests: None,
            unconsolidated_investments: None,
        };
        let result = calculate_sotp(&input).unwrap();
        let out = &result.result;
        // 10000 * 1.10 = 11000
        assert_eq!(out.total_enterprise_value, dec!(11000));
        assert_eq!(out.equity_value, dec!(8000)); // 11000 - 3000
    }

    #[test]
    fn test_sotp_nav_based_missing_assets() {
        let input = SotpInput {
            company_name: "BadREIT".into(),
            segments: vec![SegmentInput {
                name: "Seg".into(),
                revenue: dec!(500),
                ebitda: dec!(300),
                ebit: dec!(280),
                net_income: None,
                assets: None,
                method: ValuationMethod::NavBased,
                multiple: dec!(1.0),
                comparable_range: None,
                growth_rate: None,
                margin: None,
            }],
            net_debt: dec!(0),
            shares_outstanding: dec!(10),
            holding_company_discount: None,
            minority_interests: None,
            unconsolidated_investments: None,
        };
        let result = calculate_sotp(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_sotp_dcf_method() {
        let input = SotpInput {
            company_name: "GrowthCo".into(),
            segments: vec![SegmentInput {
                name: "High Growth".into(),
                revenue: dec!(1000),
                ebitda: dec!(200),
                ebit: dec!(180),
                net_income: Some(dec!(120)),
                assets: None,
                method: ValuationMethod::Dcf,
                multiple: dec!(0.10), // WACC = 10%
                comparable_range: None,
                growth_rate: Some(dec!(0.03)), // g = 3%
                margin: None,
            }],
            net_debt: dec!(0),
            shares_outstanding: dec!(50),
            holding_company_discount: None,
            minority_interests: None,
            unconsolidated_investments: None,
        };
        let result = calculate_sotp(&input).unwrap();
        let out = &result.result;
        // 200 * 1.03 / (0.10 - 0.03) = 206 / 0.07 = 2942.857...
        let expected = dec!(200) * dec!(1.03) / dec!(0.07);
        assert_eq!(out.total_enterprise_value, expected);
    }

    #[test]
    fn test_sotp_dcf_wacc_equals_growth() {
        let input = SotpInput {
            company_name: "BadDCF".into(),
            segments: vec![SegmentInput {
                name: "Seg".into(),
                revenue: dec!(1000),
                ebitda: dec!(200),
                ebit: dec!(180),
                net_income: None,
                assets: None,
                method: ValuationMethod::Dcf,
                multiple: dec!(0.05), // WACC = 5%
                comparable_range: None,
                growth_rate: Some(dec!(0.05)), // g = 5% => WACC == g
                margin: None,
            }],
            net_debt: dec!(0),
            shares_outstanding: dec!(10),
            holding_company_discount: None,
            minority_interests: None,
            unconsolidated_investments: None,
        };
        let result = calculate_sotp(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_sotp_ev_revenue_method() {
        let input = SotpInput {
            company_name: "SaaSCo".into(),
            segments: vec![SegmentInput {
                name: "SaaS".into(),
                revenue: dec!(800),
                ebitda: dec!(100),
                ebit: dec!(80),
                net_income: None,
                assets: None,
                method: ValuationMethod::EvRevenue,
                multiple: dec!(12),
                comparable_range: None,
                growth_rate: None,
                margin: None,
            }],
            net_debt: dec!(200),
            shares_outstanding: dec!(40),
            holding_company_discount: None,
            minority_interests: None,
            unconsolidated_investments: None,
        };
        let result = calculate_sotp(&input).unwrap();
        let out = &result.result;
        // 800 * 12 = 9600
        assert_eq!(out.total_enterprise_value, dec!(9600));
        assert_eq!(out.equity_value, dec!(9400)); // 9600 - 200
    }

    #[test]
    fn test_sotp_all_methods_combined() {
        let input = SotpInput {
            company_name: "DiversifiedCo".into(),
            segments: vec![
                SegmentInput {
                    name: "A".into(),
                    revenue: dec!(1000),
                    ebitda: dec!(300),
                    ebit: dec!(250),
                    net_income: Some(dec!(180)),
                    assets: None,
                    method: ValuationMethod::EvEbitda,
                    multiple: dec!(10),
                    comparable_range: None,
                    growth_rate: None,
                    margin: None,
                },
                SegmentInput {
                    name: "B".into(),
                    revenue: dec!(500),
                    ebitda: dec!(100),
                    ebit: dec!(80),
                    net_income: Some(dec!(60)),
                    assets: None,
                    method: ValuationMethod::PeRatio,
                    multiple: dec!(15),
                    comparable_range: None,
                    growth_rate: None,
                    margin: None,
                },
                SegmentInput {
                    name: "C".into(),
                    revenue: dec!(2000),
                    ebitda: dec!(50),
                    ebit: dec!(30),
                    net_income: None,
                    assets: None,
                    method: ValuationMethod::EvRevenue,
                    multiple: dec!(5),
                    comparable_range: None,
                    growth_rate: None,
                    margin: None,
                },
                SegmentInput {
                    name: "D".into(),
                    revenue: dec!(800),
                    ebitda: dec!(200),
                    ebit: dec!(170),
                    net_income: None,
                    assets: None,
                    method: ValuationMethod::EvEbit,
                    multiple: dec!(12),
                    comparable_range: None,
                    growth_rate: None,
                    margin: None,
                },
                SegmentInput {
                    name: "E".into(),
                    revenue: dec!(400),
                    ebitda: dec!(100),
                    ebit: dec!(90),
                    net_income: None,
                    assets: Some(dec!(5000)),
                    method: ValuationMethod::NavBased,
                    multiple: dec!(1.05),
                    comparable_range: None,
                    growth_rate: None,
                    margin: None,
                },
            ],
            net_debt: dec!(2000),
            shares_outstanding: dec!(200),
            holding_company_discount: None,
            minority_interests: None,
            unconsolidated_investments: None,
        };
        let result = calculate_sotp(&input).unwrap();
        let out = &result.result;

        // A: 300*10=3000, B: 60*15=900, C: 2000*5=10000, D: 170*12=2040, E: 5000*1.05=5250
        let expected = dec!(3000) + dec!(900) + dec!(10000) + dec!(2040) + dec!(5250);
        assert_eq!(out.total_enterprise_value, expected);
        assert_eq!(out.segment_values.len(), 5);
    }

    #[test]
    fn test_sotp_football_field_with_comparable_range() {
        let input = SotpInput {
            company_name: "RangeCo".into(),
            segments: vec![SegmentInput {
                name: "Main".into(),
                revenue: dec!(1000),
                ebitda: dec!(500),
                ebit: dec!(400),
                net_income: None,
                assets: None,
                method: ValuationMethod::EvEbitda,
                multiple: dec!(10),
                comparable_range: Some((dec!(8), dec!(14))),
                growth_rate: None,
                margin: None,
            }],
            net_debt: dec!(1000),
            shares_outstanding: dec!(100),
            holding_company_discount: None,
            minority_interests: None,
            unconsolidated_investments: None,
        };
        let result = calculate_sotp(&input).unwrap();
        let ff = &result.result.football_field;

        // Base: 500*10=5000, equity=4000, per_share=40
        // Low:  5000/10*8=4000, equity=3000, per_share=30
        // High: 5000/10*14=7000, equity=6000, per_share=60
        assert_eq!(ff.base_value_per_share, dec!(40));
        assert_eq!(ff.low_value_per_share, dec!(30));
        assert_eq!(ff.high_value_per_share, dec!(60));
    }

    #[test]
    fn test_sotp_football_field_default_range() {
        let input = SotpInput {
            company_name: "DefaultRange".into(),
            segments: vec![SegmentInput {
                name: "Main".into(),
                revenue: dec!(1000),
                ebitda: dec!(400),
                ebit: dec!(350),
                net_income: None,
                assets: None,
                method: ValuationMethod::EvEbitda,
                multiple: dec!(10),
                comparable_range: None,
                growth_rate: None,
                margin: None,
            }],
            net_debt: dec!(0),
            shares_outstanding: dec!(100),
            holding_company_discount: None,
            minority_interests: None,
            unconsolidated_investments: None,
        };
        let result = calculate_sotp(&input).unwrap();
        let ff = &result.result.football_field;

        // Base: 4000, per_share=40
        // Low:  4000*0.80=3200, per_share=32
        // High: 4000*1.20=4800, per_share=48
        assert_eq!(ff.base_value_per_share, dec!(40));
        assert_eq!(ff.low_value_per_share, dec!(32));
        assert_eq!(ff.high_value_per_share, dec!(48));
    }

    #[test]
    fn test_sotp_implied_multiples() {
        let input = tech_company_input();
        let result = calculate_sotp(&input).unwrap();
        let im = &result.result.implied_multiple;

        let ev = dec!(49500);
        let ebitda = dec!(1500) + dec!(900) + dec!(400); // 2800
        let revenue = dec!(5000) + dec!(3000) + dec!(2000); // 10000
        let ebit = dec!(1200) + dec!(750) + dec!(300); // 2250

        assert_eq!(im.ev_ebitda, ev / ebitda);
        assert_eq!(im.ev_revenue, ev / revenue);
        assert_eq!(im.ev_ebit, ev / ebit);

        let ni = dec!(900) + dec!(550) + dec!(200); // 1650
        let equity = ev - dec!(5000); // 44500
        assert_eq!(im.pe_ratio, Some(equity / ni));
    }

    #[test]
    fn test_sotp_zero_shares_error() {
        let mut input = tech_company_input();
        input.shares_outstanding = dec!(0);
        let result = calculate_sotp(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_sotp_negative_shares_error() {
        let mut input = tech_company_input();
        input.shares_outstanding = dec!(-10);
        let result = calculate_sotp(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_sotp_empty_segments_error() {
        let input = SotpInput {
            company_name: "Empty".into(),
            segments: vec![],
            net_debt: dec!(0),
            shares_outstanding: dec!(100),
            holding_company_discount: None,
            minority_interests: None,
            unconsolidated_investments: None,
        };
        let result = calculate_sotp(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_sotp_invalid_discount_negative() {
        let mut input = tech_company_input();
        input.holding_company_discount = Some(dec!(-0.1));
        let result = calculate_sotp(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_sotp_invalid_discount_ge_one() {
        let mut input = tech_company_input();
        input.holding_company_discount = Some(dec!(1.0));
        let result = calculate_sotp(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_sotp_zero_net_debt() {
        let mut input = tech_company_input();
        input.net_debt = dec!(0);
        let result = calculate_sotp(&input).unwrap();
        let out = &result.result;
        assert_eq!(out.equity_value, out.total_enterprise_value);
    }

    #[test]
    fn test_sotp_negative_net_debt_net_cash() {
        let mut input = tech_company_input();
        input.net_debt = dec!(-2000); // net cash position
        let result = calculate_sotp(&input).unwrap();
        let out = &result.result;
        // equity = EV - (-2000) = EV + 2000
        assert_eq!(out.equity_value, out.total_enterprise_value + dec!(2000));
    }

    #[test]
    fn test_sotp_sum_check() {
        let input = tech_company_input();
        let result = calculate_sotp(&input).unwrap();
        let out = &result.result;
        assert_eq!(out.sum_check, out.total_enterprise_value);
    }

    #[test]
    fn test_sotp_methodology_in_metadata() {
        let input = tech_company_input();
        let result = calculate_sotp(&input).unwrap();
        assert_eq!(result.methodology, "Sum-of-the-Parts Valuation");
    }

    #[test]
    fn test_sotp_segment_method_labels() {
        let input = tech_company_input();
        let result = calculate_sotp(&input).unwrap();
        let out = &result.result;
        assert_eq!(out.segment_values[0].method, "EvEbitda");
        assert_eq!(out.segment_values[1].method, "EvRevenue");
        assert_eq!(out.segment_values[2].method, "EvEbit");
    }

    #[test]
    fn test_sotp_conglomerate_with_all_adjustments() {
        let mut input = tech_company_input();
        input.holding_company_discount = Some(dec!(0.10));
        input.minority_interests = Some(dec!(300));
        input.unconsolidated_investments = Some(dec!(700));
        let result = calculate_sotp(&input).unwrap();
        let out = &result.result;

        let raw_ev = dec!(49500);
        let disc_ev = raw_ev * dec!(0.90); // 44550
        let equity = disc_ev - dec!(5000) + dec!(700) - dec!(300); // 39950
        assert_eq!(out.total_enterprise_value, disc_ev);
        assert_eq!(out.equity_value, equity);
        assert_eq!(out.equity_value_per_share, equity / dec!(100));
    }

    #[test]
    fn test_sotp_zero_ebitda_implied_multiple() {
        let input = SotpInput {
            company_name: "ZeroEBITDA".into(),
            segments: vec![SegmentInput {
                name: "Seg".into(),
                revenue: dec!(1000),
                ebitda: dec!(0),
                ebit: dec!(0),
                net_income: None,
                assets: None,
                method: ValuationMethod::EvRevenue,
                multiple: dec!(5),
                comparable_range: None,
                growth_rate: None,
                margin: None,
            }],
            net_debt: dec!(0),
            shares_outstanding: dec!(10),
            holding_company_discount: None,
            minority_interests: None,
            unconsolidated_investments: None,
        };
        let result = calculate_sotp(&input).unwrap();
        let im = &result.result.implied_multiple;
        // EV/EBITDA should be 0 since consolidated EBITDA is 0
        assert_eq!(im.ev_ebitda, dec!(0));
        assert!(result.warnings.iter().any(|w| w.contains("EBITDA is zero")));
    }

    #[test]
    fn test_sotp_no_net_income_no_pe() {
        let input = SotpInput {
            company_name: "NoPE".into(),
            segments: vec![SegmentInput {
                name: "Seg".into(),
                revenue: dec!(1000),
                ebitda: dec!(300),
                ebit: dec!(250),
                net_income: None,
                assets: None,
                method: ValuationMethod::EvEbitda,
                multiple: dec!(10),
                comparable_range: None,
                growth_rate: None,
                margin: None,
            }],
            net_debt: dec!(0),
            shares_outstanding: dec!(10),
            holding_company_discount: None,
            minority_interests: None,
            unconsolidated_investments: None,
        };
        let result = calculate_sotp(&input).unwrap();
        assert!(result.result.implied_multiple.pe_ratio.is_none());
    }
}
