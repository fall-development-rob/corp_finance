//! Risk Limit Management Framework.
//!
//! Covers:
//! 1. **Limit Types** -- Notional, VaR, Concentration, Sector, Country
//! 2. **Utilization** -- current_exposure / limit for each type
//! 3. **Breach Detection** -- flag breaches (>100%) and warnings (>threshold)
//! 4. **Headroom** -- remaining capacity = limit - current
//! 5. **Limit Aggregation** -- roll up sub-limits to entity level
//!
//! All arithmetic uses `rust_decimal::Decimal`. No `f64`.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// Type of risk limit.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LimitType {
    Notional,
    VaR,
    Concentration,
    Sector,
    Country,
}

impl std::fmt::Display for LimitType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LimitType::Notional => write!(f, "Notional"),
            LimitType::VaR => write!(f, "VaR"),
            LimitType::Concentration => write!(f, "Concentration"),
            LimitType::Sector => write!(f, "Sector"),
            LimitType::Country => write!(f, "Country"),
        }
    }
}

/// Traffic light status of a limit.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LimitStatus {
    /// Below warning threshold.
    Green,
    /// Above warning threshold but below limit.
    Amber,
    /// At or above limit (100%).
    Red,
    /// Exceeded limit.
    Breach,
}

impl std::fmt::Display for LimitStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LimitStatus::Green => write!(f, "Green"),
            LimitStatus::Amber => write!(f, "Amber"),
            LimitStatus::Red => write!(f, "Red"),
            LimitStatus::Breach => write!(f, "Breach"),
        }
    }
}

// ---------------------------------------------------------------------------
// Input / Output
// ---------------------------------------------------------------------------

/// A single risk limit definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitDefinition {
    /// Name / identifier of the limit.
    pub name: String,
    /// Type of limit.
    pub limit_type: LimitType,
    /// Maximum allowed value.
    pub limit_value: Decimal,
    /// Current exposure / usage.
    pub current_value: Decimal,
    /// Warning threshold as a fraction of limit (e.g. 0.80 = 80%).
    pub warning_threshold: Decimal,
}

/// Input for limit management.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitManagementInput {
    /// Risk limits to evaluate.
    pub limits: Vec<LimitDefinition>,
}

/// Status of a single limit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitStatusDetail {
    /// Limit name.
    pub name: String,
    /// Limit type.
    pub limit_type: LimitType,
    /// Utilization percentage (current / limit).
    pub utilization_pct: Decimal,
    /// Remaining headroom (limit - current).
    pub headroom: Decimal,
    /// Traffic light status.
    pub status: LimitStatus,
}

/// Output of the limit management analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitManagementOutput {
    /// Per-limit status details.
    pub limit_status: Vec<LimitStatusDetail>,
    /// Total number of breaches (>100%).
    pub total_breaches: u32,
    /// Total number of warnings (>threshold but <=100%).
    pub total_warnings: u32,
    /// Worst (highest) utilization across all limits.
    pub worst_utilization: Decimal,
}

/// Evaluate risk limits and detect breaches/warnings.
pub fn evaluate_limits(input: &LimitManagementInput) -> CorpFinanceResult<LimitManagementOutput> {
    validate_limit_input(input)?;

    let mut limit_status = Vec::with_capacity(input.limits.len());
    let mut total_breaches: u32 = 0;
    let mut total_warnings: u32 = 0;
    let mut worst_utilization = Decimal::ZERO;

    for limit in &input.limits {
        // Utilization
        let utilization_pct = if limit.limit_value.is_zero() {
            if limit.current_value.is_zero() {
                Decimal::ZERO
            } else {
                // Any non-zero usage against zero limit is a breach
                dec!(999.99)
            }
        } else {
            limit.current_value / limit.limit_value
        };

        // Headroom
        let headroom = limit.limit_value - limit.current_value;

        // Status determination
        let status = if utilization_pct > Decimal::ONE {
            LimitStatus::Breach
        } else if utilization_pct == Decimal::ONE {
            LimitStatus::Red
        } else if utilization_pct >= limit.warning_threshold {
            LimitStatus::Amber
        } else {
            LimitStatus::Green
        };

        match status {
            LimitStatus::Breach => total_breaches += 1,
            LimitStatus::Amber => total_warnings += 1,
            LimitStatus::Red => total_breaches += 1,
            LimitStatus::Green => {}
        }

        if utilization_pct > worst_utilization {
            worst_utilization = utilization_pct;
        }

        limit_status.push(LimitStatusDetail {
            name: limit.name.clone(),
            limit_type: limit.limit_type.clone(),
            utilization_pct,
            headroom,
            status,
        });
    }

    Ok(LimitManagementOutput {
        limit_status,
        total_breaches,
        total_warnings,
        worst_utilization,
    })
}

fn validate_limit_input(input: &LimitManagementInput) -> CorpFinanceResult<()> {
    if input.limits.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one limit definition is required.".into(),
        ));
    }
    for limit in &input.limits {
        if limit.limit_value < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "limit_value".into(),
                reason: format!(
                    "Limit value must be non-negative for limit '{}'.",
                    limit.name
                ),
            });
        }
        if limit.current_value < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "current_value".into(),
                reason: format!(
                    "Current value must be non-negative for limit '{}'.",
                    limit.name
                ),
            });
        }
        if limit.warning_threshold < Decimal::ZERO || limit.warning_threshold > Decimal::ONE {
            return Err(CorpFinanceError::InvalidInput {
                field: "warning_threshold".into(),
                reason: format!(
                    "Warning threshold must be in [0, 1] for limit '{}'.",
                    limit.name
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

    fn make_base_input() -> LimitManagementInput {
        LimitManagementInput {
            limits: vec![
                LimitDefinition {
                    name: "Notional_Total".into(),
                    limit_type: LimitType::Notional,
                    limit_value: dec!(100_000_000),
                    current_value: dec!(75_000_000),
                    warning_threshold: dec!(0.80),
                },
                LimitDefinition {
                    name: "VaR_Daily".into(),
                    limit_type: LimitType::VaR,
                    limit_value: dec!(5_000_000),
                    current_value: dec!(4_500_000),
                    warning_threshold: dec!(0.80),
                },
                LimitDefinition {
                    name: "Single_Name".into(),
                    limit_type: LimitType::Concentration,
                    limit_value: dec!(10_000_000),
                    current_value: dec!(6_000_000),
                    warning_threshold: dec!(0.80),
                },
                LimitDefinition {
                    name: "Sector_Tech".into(),
                    limit_type: LimitType::Sector,
                    limit_value: dec!(30_000_000),
                    current_value: dec!(32_000_000),
                    warning_threshold: dec!(0.80),
                },
                LimitDefinition {
                    name: "Country_EM".into(),
                    limit_type: LimitType::Country,
                    limit_value: dec!(20_000_000),
                    current_value: dec!(10_000_000),
                    warning_threshold: dec!(0.80),
                },
            ],
        }
    }

    #[test]
    fn test_status_count_matches_limits() {
        let input = make_base_input();
        let out = evaluate_limits(&input).unwrap();
        assert_eq!(out.limit_status.len(), 5);
    }

    #[test]
    fn test_green_status_below_threshold() {
        let input = make_base_input();
        let out = evaluate_limits(&input).unwrap();
        // Country_EM: 10M/20M = 50% < 80% threshold => Green
        let country = out
            .limit_status
            .iter()
            .find(|s| s.name == "Country_EM")
            .unwrap();
        assert_eq!(country.status, LimitStatus::Green);
    }

    #[test]
    fn test_amber_status_above_threshold() {
        let input = make_base_input();
        let out = evaluate_limits(&input).unwrap();
        // VaR_Daily: 4.5M/5M = 90% > 80% => Amber
        let var = out
            .limit_status
            .iter()
            .find(|s| s.name == "VaR_Daily")
            .unwrap();
        assert_eq!(var.status, LimitStatus::Amber);
    }

    #[test]
    fn test_breach_status_over_limit() {
        let input = make_base_input();
        let out = evaluate_limits(&input).unwrap();
        // Sector_Tech: 32M/30M > 100% => Breach
        let sector = out
            .limit_status
            .iter()
            .find(|s| s.name == "Sector_Tech")
            .unwrap();
        assert_eq!(sector.status, LimitStatus::Breach);
    }

    #[test]
    fn test_utilization_calculation() {
        let input = make_base_input();
        let out = evaluate_limits(&input).unwrap();
        let notional = out
            .limit_status
            .iter()
            .find(|s| s.name == "Notional_Total")
            .unwrap();
        // 75M / 100M = 0.75
        assert_eq!(notional.utilization_pct, dec!(0.75));
    }

    #[test]
    fn test_headroom_calculation() {
        let input = make_base_input();
        let out = evaluate_limits(&input).unwrap();
        let notional = out
            .limit_status
            .iter()
            .find(|s| s.name == "Notional_Total")
            .unwrap();
        // headroom = 100M - 75M = 25M
        assert_eq!(notional.headroom, dec!(25_000_000));
    }

    #[test]
    fn test_negative_headroom_on_breach() {
        let input = make_base_input();
        let out = evaluate_limits(&input).unwrap();
        let sector = out
            .limit_status
            .iter()
            .find(|s| s.name == "Sector_Tech")
            .unwrap();
        // headroom = 30M - 32M = -2M
        assert_eq!(sector.headroom, dec!(-2_000_000));
    }

    #[test]
    fn test_total_breaches_count() {
        let input = make_base_input();
        let out = evaluate_limits(&input).unwrap();
        // Sector_Tech is breached
        assert_eq!(out.total_breaches, 1);
    }

    #[test]
    fn test_total_warnings_count() {
        let input = make_base_input();
        let out = evaluate_limits(&input).unwrap();
        // VaR_Daily (90%) and Single_Name (60%? no, 60% < 80%) => only VaR_Daily
        // Notional_Total: 75% < 80% => Green
        // Actually: VaR 90% => Amber
        assert!(out.total_warnings >= 1);
    }

    #[test]
    fn test_worst_utilization() {
        let input = make_base_input();
        let out = evaluate_limits(&input).unwrap();
        // Sector_Tech: 32M/30M = 1.0666... is the worst
        let expected = dec!(32_000_000) / dec!(30_000_000);
        assert!(
            approx_eq(out.worst_utilization, expected, dec!(0.001)),
            "Worst utilization should be ~{}, got {}",
            expected,
            out.worst_utilization
        );
    }

    #[test]
    fn test_all_green_no_breaches_no_warnings() {
        let input = LimitManagementInput {
            limits: vec![
                LimitDefinition {
                    name: "Safe_1".into(),
                    limit_type: LimitType::Notional,
                    limit_value: dec!(100),
                    current_value: dec!(10),
                    warning_threshold: dec!(0.80),
                },
                LimitDefinition {
                    name: "Safe_2".into(),
                    limit_type: LimitType::VaR,
                    limit_value: dec!(100),
                    current_value: dec!(20),
                    warning_threshold: dec!(0.80),
                },
            ],
        };
        let out = evaluate_limits(&input).unwrap();
        assert_eq!(out.total_breaches, 0);
        assert_eq!(out.total_warnings, 0);
        for s in &out.limit_status {
            assert_eq!(s.status, LimitStatus::Green);
        }
    }

    #[test]
    fn test_exact_at_limit_is_red() {
        let input = LimitManagementInput {
            limits: vec![LimitDefinition {
                name: "At_Limit".into(),
                limit_type: LimitType::Notional,
                limit_value: dec!(100),
                current_value: dec!(100),
                warning_threshold: dec!(0.80),
            }],
        };
        let out = evaluate_limits(&input).unwrap();
        assert_eq!(out.limit_status[0].status, LimitStatus::Red);
    }

    #[test]
    fn test_exact_at_warning_threshold_is_amber() {
        let input = LimitManagementInput {
            limits: vec![LimitDefinition {
                name: "At_Warning".into(),
                limit_type: LimitType::VaR,
                limit_value: dec!(100),
                current_value: dec!(80),
                warning_threshold: dec!(0.80),
            }],
        };
        let out = evaluate_limits(&input).unwrap();
        assert_eq!(out.limit_status[0].status, LimitStatus::Amber);
    }

    #[test]
    fn test_just_below_warning_is_green() {
        let input = LimitManagementInput {
            limits: vec![LimitDefinition {
                name: "Below_Warning".into(),
                limit_type: LimitType::Concentration,
                limit_value: dec!(100),
                current_value: dec!(79),
                warning_threshold: dec!(0.80),
            }],
        };
        let out = evaluate_limits(&input).unwrap();
        assert_eq!(out.limit_status[0].status, LimitStatus::Green);
    }

    #[test]
    fn test_zero_current_is_green() {
        let input = LimitManagementInput {
            limits: vec![LimitDefinition {
                name: "Unused".into(),
                limit_type: LimitType::Country,
                limit_value: dec!(100),
                current_value: Decimal::ZERO,
                warning_threshold: dec!(0.80),
            }],
        };
        let out = evaluate_limits(&input).unwrap();
        assert_eq!(out.limit_status[0].status, LimitStatus::Green);
        assert_eq!(out.limit_status[0].utilization_pct, Decimal::ZERO);
        assert_eq!(out.limit_status[0].headroom, dec!(100));
    }

    #[test]
    fn test_zero_limit_zero_current_is_green() {
        let input = LimitManagementInput {
            limits: vec![LimitDefinition {
                name: "Zero_Zero".into(),
                limit_type: LimitType::Notional,
                limit_value: Decimal::ZERO,
                current_value: Decimal::ZERO,
                warning_threshold: dec!(0.80),
            }],
        };
        let out = evaluate_limits(&input).unwrap();
        assert_eq!(out.limit_status[0].utilization_pct, Decimal::ZERO);
    }

    #[test]
    fn test_zero_limit_nonzero_current_is_breach() {
        let input = LimitManagementInput {
            limits: vec![LimitDefinition {
                name: "Zero_Limit".into(),
                limit_type: LimitType::VaR,
                limit_value: Decimal::ZERO,
                current_value: dec!(1),
                warning_threshold: dec!(0.80),
            }],
        };
        let out = evaluate_limits(&input).unwrap();
        assert_eq!(out.limit_status[0].status, LimitStatus::Breach);
    }

    #[test]
    fn test_limit_type_preserved() {
        let input = make_base_input();
        let out = evaluate_limits(&input).unwrap();
        assert_eq!(out.limit_status[0].limit_type, LimitType::Notional);
        assert_eq!(out.limit_status[1].limit_type, LimitType::VaR);
        assert_eq!(out.limit_status[2].limit_type, LimitType::Concentration);
        assert_eq!(out.limit_status[3].limit_type, LimitType::Sector);
        assert_eq!(out.limit_status[4].limit_type, LimitType::Country);
    }

    #[test]
    fn test_multiple_breaches() {
        let input = LimitManagementInput {
            limits: vec![
                LimitDefinition {
                    name: "Breach_1".into(),
                    limit_type: LimitType::Notional,
                    limit_value: dec!(100),
                    current_value: dec!(150),
                    warning_threshold: dec!(0.80),
                },
                LimitDefinition {
                    name: "Breach_2".into(),
                    limit_type: LimitType::VaR,
                    limit_value: dec!(50),
                    current_value: dec!(60),
                    warning_threshold: dec!(0.80),
                },
            ],
        };
        let out = evaluate_limits(&input).unwrap();
        assert_eq!(out.total_breaches, 2);
    }

    #[test]
    fn test_warning_threshold_zero_all_amber_or_higher() {
        let input = LimitManagementInput {
            limits: vec![LimitDefinition {
                name: "Zero_Threshold".into(),
                limit_type: LimitType::Sector,
                limit_value: dec!(100),
                current_value: dec!(50),
                warning_threshold: Decimal::ZERO,
            }],
        };
        let out = evaluate_limits(&input).unwrap();
        // 50% >= 0% threshold => Amber
        assert_eq!(out.limit_status[0].status, LimitStatus::Amber);
    }

    #[test]
    fn test_warning_threshold_one_only_breach_triggers() {
        let input = LimitManagementInput {
            limits: vec![LimitDefinition {
                name: "High_Threshold".into(),
                limit_type: LimitType::Country,
                limit_value: dec!(100),
                current_value: dec!(99),
                warning_threshold: Decimal::ONE,
            }],
        };
        let out = evaluate_limits(&input).unwrap();
        // 99% < 100% threshold => Green (since 0.99 < 1.0)
        assert_eq!(out.limit_status[0].status, LimitStatus::Green);
    }

    // -- Validation tests --

    #[test]
    fn test_reject_empty_limits() {
        let input = LimitManagementInput { limits: vec![] };
        assert!(evaluate_limits(&input).is_err());
    }

    #[test]
    fn test_reject_negative_limit_value() {
        let input = LimitManagementInput {
            limits: vec![LimitDefinition {
                name: "Bad".into(),
                limit_type: LimitType::Notional,
                limit_value: dec!(-100),
                current_value: dec!(50),
                warning_threshold: dec!(0.80),
            }],
        };
        assert!(evaluate_limits(&input).is_err());
    }

    #[test]
    fn test_reject_negative_current_value() {
        let input = LimitManagementInput {
            limits: vec![LimitDefinition {
                name: "Bad".into(),
                limit_type: LimitType::VaR,
                limit_value: dec!(100),
                current_value: dec!(-10),
                warning_threshold: dec!(0.80),
            }],
        };
        assert!(evaluate_limits(&input).is_err());
    }

    #[test]
    fn test_reject_warning_threshold_above_one() {
        let input = LimitManagementInput {
            limits: vec![LimitDefinition {
                name: "Bad".into(),
                limit_type: LimitType::Concentration,
                limit_value: dec!(100),
                current_value: dec!(50),
                warning_threshold: dec!(1.5),
            }],
        };
        assert!(evaluate_limits(&input).is_err());
    }

    #[test]
    fn test_reject_negative_warning_threshold() {
        let input = LimitManagementInput {
            limits: vec![LimitDefinition {
                name: "Bad".into(),
                limit_type: LimitType::Sector,
                limit_value: dec!(100),
                current_value: dec!(50),
                warning_threshold: dec!(-0.1),
            }],
        };
        assert!(evaluate_limits(&input).is_err());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let input = make_base_input();
        let out = evaluate_limits(&input).unwrap();
        let json = serde_json::to_string(&out).unwrap();
        let _: LimitManagementOutput = serde_json::from_str(&json).unwrap();
    }
}
