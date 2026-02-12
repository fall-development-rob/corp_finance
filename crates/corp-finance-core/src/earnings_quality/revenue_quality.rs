//! Revenue quality metrics for earnings analysis.
//!
//! Implements:
//! 1. **DSO** -- Days Sales Outstanding (current and prior, plus change).
//! 2. **Deferred Revenue Growth** -- negative growth implies recognition acceleration.
//! 3. **Allowance to Receivables** -- too-low ratio suggests aggressive write-off policy.
//! 4. **Revenue Concentration (HHI)** -- Herfindahl-Hirschman Index of segment revenue.
//! 5. **Composite Quality Score** -- 0-100 with flag-based deductions.
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

/// A single revenue segment for concentration analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentRevenue {
    pub name: String,
    pub revenue: Decimal,
}

/// Financial data required for revenue quality analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevenueQualityInput {
    pub current_receivables: Decimal,
    pub prior_receivables: Decimal,
    pub current_revenue: Decimal,
    pub prior_revenue: Decimal,
    pub current_deferred_revenue: Decimal,
    pub prior_deferred_revenue: Decimal,
    pub allowance_for_doubtful: Decimal,
    pub revenue_segments: Vec<SegmentRevenue>,
}

/// Revenue quality analysis results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevenueQualityOutput {
    /// (current_receivables / current_revenue) * 365.
    pub dso_current: Decimal,
    /// (prior_receivables / prior_revenue) * 365.
    pub dso_prior: Decimal,
    /// dso_current - dso_prior.
    pub dso_change: Decimal,
    /// True if DSO increased by more than 10 days.
    pub dso_flag: bool,
    /// (current_dr - prior_dr) / prior_dr. `None` when prior_dr == 0.
    pub deferred_revenue_growth: Option<Decimal>,
    /// True if deferred revenue growth is negative (recognition acceleration).
    pub deferred_revenue_flag: bool,
    /// allowance_for_doubtful / current_receivables. `None` when receivables == 0.
    pub allowance_to_receivables: Option<Decimal>,
    /// True if allowance ratio < 2%.
    pub allowance_flag: bool,
    /// HHI of revenue segments (0-10000 scale).
    pub revenue_concentration_hhi: Decimal,
    /// True if HHI > 2500 (highly concentrated).
    pub concentration_flag: bool,
    /// Composite quality score (0-100).
    pub quality_score: Decimal,
    /// "High", "Medium", or "Low".
    pub quality_rating: String,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const DAYS_PER_YEAR: Decimal = dec!(365);
const DSO_CHANGE_THRESHOLD: Decimal = dec!(10);
const ALLOWANCE_MIN_RATIO: Decimal = dec!(0.02);
const HHI_CONCENTRATION_THRESHOLD: Decimal = dec!(2500);
const FLAG_DEDUCTION: Decimal = dec!(25);

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Compute revenue quality metrics (DSO, deferred revenue, allowance, HHI).
pub fn calculate_revenue_quality(
    input: &RevenueQualityInput,
) -> CorpFinanceResult<RevenueQualityOutput> {
    // ---- Validation ----
    if input.current_revenue <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "current_revenue".into(),
            reason: "Must be positive".into(),
        });
    }
    if input.prior_revenue <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "prior_revenue".into(),
            reason: "Must be positive".into(),
        });
    }
    if input.current_receivables < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "current_receivables".into(),
            reason: "Cannot be negative".into(),
        });
    }
    if input.prior_receivables < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "prior_receivables".into(),
            reason: "Cannot be negative".into(),
        });
    }

    // ---- DSO ----
    let dso_current = (input.current_receivables / input.current_revenue) * DAYS_PER_YEAR;
    let dso_prior = (input.prior_receivables / input.prior_revenue) * DAYS_PER_YEAR;
    let dso_change = dso_current - dso_prior;
    let dso_flag = dso_change > DSO_CHANGE_THRESHOLD;

    // ---- Deferred Revenue Growth ----
    let (deferred_revenue_growth, deferred_revenue_flag) =
        if input.prior_deferred_revenue != Decimal::ZERO {
            let growth = (input.current_deferred_revenue - input.prior_deferred_revenue)
                / input.prior_deferred_revenue;
            (Some(growth), growth < Decimal::ZERO)
        } else {
            // No prior deferred revenue; flag only if current is also zero (no info)
            (None, false)
        };

    // ---- Allowance to Receivables ----
    let (allowance_to_receivables, allowance_flag) = if input.current_receivables > Decimal::ZERO {
        let ratio = input.allowance_for_doubtful / input.current_receivables;
        (Some(ratio), ratio < ALLOWANCE_MIN_RATIO)
    } else {
        (None, false)
    };

    // ---- Revenue Concentration HHI ----
    let revenue_concentration_hhi = compute_hhi(&input.revenue_segments)?;
    let concentration_flag = revenue_concentration_hhi > HHI_CONCENTRATION_THRESHOLD;

    // ---- Composite Score ----
    let mut score = dec!(100);
    if dso_flag {
        score -= FLAG_DEDUCTION;
    }
    if deferred_revenue_flag {
        score -= FLAG_DEDUCTION;
    }
    if allowance_flag {
        score -= FLAG_DEDUCTION;
    }
    if concentration_flag {
        score -= FLAG_DEDUCTION;
    }
    // Clamp to [0, 100]
    if score < Decimal::ZERO {
        score = Decimal::ZERO;
    }

    let quality_rating = if score >= dec!(75) {
        "High".to_string()
    } else if score >= dec!(50) {
        "Medium".to_string()
    } else {
        "Low".to_string()
    };

    Ok(RevenueQualityOutput {
        dso_current,
        dso_prior,
        dso_change,
        dso_flag,
        deferred_revenue_growth,
        deferred_revenue_flag,
        allowance_to_receivables,
        allowance_flag,
        revenue_concentration_hhi,
        concentration_flag,
        quality_score: score,
        quality_rating,
    })
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Compute the Herfindahl-Hirschman Index for revenue segments.
/// HHI = sum of (market_share_i * 100)^2 where share_i = segment_rev / total_rev.
fn compute_hhi(segments: &[SegmentRevenue]) -> CorpFinanceResult<Decimal> {
    if segments.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one revenue segment required for HHI".into(),
        ));
    }

    let total: Decimal = segments.iter().map(|s| s.revenue).sum();
    if total <= Decimal::ZERO {
        return Err(CorpFinanceError::DivisionByZero {
            context: "total segment revenue for HHI".into(),
        });
    }

    let hhi: Decimal = segments
        .iter()
        .map(|s| {
            let share_pct = (s.revenue / total) * dec!(100);
            share_pct * share_pct
        })
        .sum();

    Ok(hhi)
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn diversified_segments() -> Vec<SegmentRevenue> {
        vec![
            SegmentRevenue {
                name: "Segment A".into(),
                revenue: dec!(250),
            },
            SegmentRevenue {
                name: "Segment B".into(),
                revenue: dec!(250),
            },
            SegmentRevenue {
                name: "Segment C".into(),
                revenue: dec!(250),
            },
            SegmentRevenue {
                name: "Segment D".into(),
                revenue: dec!(250),
            },
        ]
    }

    fn concentrated_segments() -> Vec<SegmentRevenue> {
        vec![
            SegmentRevenue {
                name: "Core".into(),
                revenue: dec!(900),
            },
            SegmentRevenue {
                name: "Other".into(),
                revenue: dec!(100),
            },
        ]
    }

    fn normal_input() -> RevenueQualityInput {
        RevenueQualityInput {
            current_receivables: dec!(100),
            prior_receivables: dec!(95),
            current_revenue: dec!(1000),
            prior_revenue: dec!(950),
            current_deferred_revenue: dec!(50),
            prior_deferred_revenue: dec!(45),
            allowance_for_doubtful: dec!(5),
            revenue_segments: diversified_segments(),
        }
    }

    #[test]
    fn test_normal_company_high_quality() {
        let out = calculate_revenue_quality(&normal_input()).unwrap();
        assert_eq!(out.quality_rating, "High");
        assert!(out.quality_score >= dec!(75));
    }

    #[test]
    fn test_dso_calculation() {
        let out = calculate_revenue_quality(&normal_input()).unwrap();
        // DSO current = (100/1000)*365 = 36.5
        assert_eq!(out.dso_current, dec!(36.5));
    }

    #[test]
    fn test_dso_prior_calculation() {
        let out = calculate_revenue_quality(&normal_input()).unwrap();
        // DSO prior = (95/950)*365 = 36.5
        assert_eq!(out.dso_prior, dec!(36.5));
    }

    #[test]
    fn test_dso_change_stable() {
        let out = calculate_revenue_quality(&normal_input()).unwrap();
        assert_eq!(out.dso_change, Decimal::ZERO);
        assert!(!out.dso_flag);
    }

    #[test]
    fn test_dso_spike_flagged() {
        let mut input = normal_input();
        input.current_receivables = dec!(200); // DSO jumps to 73
        let out = calculate_revenue_quality(&input).unwrap();
        assert!(out.dso_flag);
        assert!(out.dso_change > dec!(10));
    }

    #[test]
    fn test_deferred_revenue_growth() {
        let out = calculate_revenue_quality(&normal_input()).unwrap();
        // growth = (50-45)/45 = 5/45 = 0.1111..
        let growth = out.deferred_revenue_growth.unwrap();
        assert!(growth > dec!(0.11) && growth < dec!(0.12));
        assert!(!out.deferred_revenue_flag);
    }

    #[test]
    fn test_deferred_revenue_decline_flagged() {
        let mut input = normal_input();
        input.current_deferred_revenue = dec!(30); // decline
        let out = calculate_revenue_quality(&input).unwrap();
        assert!(out.deferred_revenue_flag);
    }

    #[test]
    fn test_zero_prior_deferred_revenue() {
        let mut input = normal_input();
        input.prior_deferred_revenue = Decimal::ZERO;
        let out = calculate_revenue_quality(&input).unwrap();
        assert_eq!(out.deferred_revenue_growth, None);
        assert!(!out.deferred_revenue_flag);
    }

    #[test]
    fn test_allowance_ratio() {
        let out = calculate_revenue_quality(&normal_input()).unwrap();
        // ratio = 5/100 = 0.05
        assert_eq!(out.allowance_to_receivables, Some(dec!(0.05)));
        assert!(!out.allowance_flag);
    }

    #[test]
    fn test_low_allowance_flagged() {
        let mut input = normal_input();
        input.allowance_for_doubtful = dec!(1); // 1/100 = 0.01 < 0.02
        let out = calculate_revenue_quality(&input).unwrap();
        assert!(out.allowance_flag);
    }

    #[test]
    fn test_hhi_diversified() {
        let out = calculate_revenue_quality(&normal_input()).unwrap();
        // 4 equal segments: HHI = 4 * (25)^2 = 2500
        assert_eq!(out.revenue_concentration_hhi, dec!(2500));
        // Exactly at threshold, not greater
        assert!(!out.concentration_flag);
    }

    #[test]
    fn test_hhi_concentrated_flagged() {
        let mut input = normal_input();
        input.revenue_segments = concentrated_segments();
        let out = calculate_revenue_quality(&input).unwrap();
        // 90% and 10%: HHI = 8100 + 100 = 8200
        assert!(out.revenue_concentration_hhi > dec!(8000));
        assert!(out.concentration_flag);
    }

    #[test]
    fn test_all_flags_low_quality() {
        let mut input = normal_input();
        input.current_receivables = dec!(200); // DSO spike
        input.current_deferred_revenue = dec!(20); // deferred decline
        input.allowance_for_doubtful = dec!(1); // low allowance
        input.revenue_segments = concentrated_segments(); // concentrated
        let out = calculate_revenue_quality(&input).unwrap();
        assert_eq!(out.quality_score, Decimal::ZERO);
        assert_eq!(out.quality_rating, "Low");
    }

    #[test]
    fn test_zero_current_revenue_rejected() {
        let mut input = normal_input();
        input.current_revenue = Decimal::ZERO;
        assert!(calculate_revenue_quality(&input).is_err());
    }

    #[test]
    fn test_zero_prior_revenue_rejected() {
        let mut input = normal_input();
        input.prior_revenue = Decimal::ZERO;
        assert!(calculate_revenue_quality(&input).is_err());
    }

    #[test]
    fn test_negative_receivables_rejected() {
        let mut input = normal_input();
        input.current_receivables = dec!(-10);
        assert!(calculate_revenue_quality(&input).is_err());
    }

    #[test]
    fn test_empty_segments_rejected() {
        let mut input = normal_input();
        input.revenue_segments = vec![];
        assert!(calculate_revenue_quality(&input).is_err());
    }

    #[test]
    fn test_single_segment_hhi() {
        let mut input = normal_input();
        input.revenue_segments = vec![SegmentRevenue {
            name: "Only".into(),
            revenue: dec!(1000),
        }];
        let out = calculate_revenue_quality(&input).unwrap();
        // single segment: HHI = 10000
        assert_eq!(out.revenue_concentration_hhi, dec!(10000));
        assert!(out.concentration_flag);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let input = normal_input();
        let json = serde_json::to_string(&input).unwrap();
        let deser: RevenueQualityInput = serde_json::from_str(&json).unwrap();
        let out1 = calculate_revenue_quality(&input).unwrap();
        let out2 = calculate_revenue_quality(&deser).unwrap();
        assert_eq!(out1.quality_score, out2.quality_score);
    }

    #[test]
    fn test_output_serialization() {
        let out = calculate_revenue_quality(&normal_input()).unwrap();
        let json = serde_json::to_string(&out).unwrap();
        let deser: RevenueQualityOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(out.quality_score, deser.quality_score);
        assert_eq!(out.quality_rating, deser.quality_rating);
    }
}
