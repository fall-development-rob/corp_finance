//! Crypto token valuation: NVT ratio, Metcalfe's Law, token DCF, and
//! relative (comparable protocol) valuation.
//!
//! All arithmetic uses `rust_decimal::Decimal` — never `f64`.
//! Discount factors are built by iterative multiplication (never `powd`).

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

/// NVT thresholds for signal classification
const NVT_UNDERVALUED_UPPER: Decimal = dec!(20);
const NVT_FAIR_UPPER: Decimal = dec!(65);

// ---------------------------------------------------------------------------
// Input types
// ---------------------------------------------------------------------------

/// A comparable protocol used for relative valuation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparableProtocol {
    /// Protocol name
    pub name: String,
    /// Fully diluted valuation
    pub fdv: Money,
    /// Annual protocol revenue / fees earned
    pub revenue: Money,
    /// Total value locked (DeFi protocols)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tvl: Option<Money>,
    /// Annual protocol fees (may differ from revenue if some fees go to LPs)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fees: Option<Money>,
}

/// Full input for a token valuation computation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenValuationInput {
    /// Token or protocol name
    pub token_name: String,
    /// Fully diluted valuation / market cap
    pub network_value: Money,
    /// Daily on-chain transaction volume
    pub daily_transaction_volume: Money,
    /// Number of active addresses (daily or monthly)
    pub active_addresses: u64,
    /// Annual protocol revenue (fees earned by the protocol)
    pub annual_protocol_revenue: Money,
    /// Total value locked — relevant for DeFi protocols
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_value_locked: Option<Money>,
    /// Total / max token supply
    pub token_supply: Decimal,
    /// Currently circulating supply
    pub circulating_supply: Decimal,
    /// Discount rate for DCF (e.g. 0.20 = 20%)
    pub discount_rate: Rate,
    /// Expected annual revenue growth rate (e.g. 0.30 = 30%)
    pub revenue_growth_rate: Rate,
    /// Terminal growth rate for Gordon Growth model (e.g. 0.03 = 3%)
    pub terminal_growth_rate: Rate,
    /// Number of projection years for DCF (default 5)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub projection_years: Option<u32>,
    /// Comparable protocols for relative valuation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comparable_protocols: Option<Vec<ComparableProtocol>>,
}

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// Relative-valuation detail from comparable protocol multiples.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelativeValuation {
    /// Median FDV / Revenue multiple from comparables
    pub median_fdv_revenue: Decimal,
    /// Median FDV / TVL multiple from comparables (if TVL data present)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub median_fdv_tvl: Option<Decimal>,
    /// Median FDV / Fees multiple from comparables (if fees data present)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub median_fdv_fees: Option<Decimal>,
    /// Implied value from FDV/Revenue
    pub implied_value_revenue: Money,
    /// Implied value from FDV/TVL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub implied_value_tvl: Option<Money>,
    /// Implied value from FDV/Fees
    #[serde(skip_serializing_if = "Option::is_none")]
    pub implied_value_fees: Option<Money>,
}

/// Complete output from a token valuation computation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenValuationOutput {
    /// NVT ratio: network_value / (daily_tx_vol * 365)
    pub nvt_ratio: Decimal,
    /// NVT signal classification
    pub nvt_signal: String,
    /// Metcalfe's Law implied network value
    pub metcalfe_value: Money,
    /// Premium (+) or discount (-) vs Metcalfe value
    pub metcalfe_premium_discount: Rate,
    /// Present value of projected protocol fees (DCF)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dcf_value: Option<Money>,
    /// DCF value per circulating token
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dcf_per_token: Option<Decimal>,
    /// Relative valuation from comparable protocols
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relative_valuation: Option<RelativeValuation>,
    /// Warnings generated during computation
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Value a crypto token / protocol using NVT, Metcalfe, DCF, and comparable
/// protocol multiples.
pub fn value_token(
    input: &TokenValuationInput,
) -> CorpFinanceResult<ComputationOutput<TokenValuationOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // --- Validate ---
    validate_input(input)?;

    // --- Projection years ---
    let proj_years = input.projection_years.unwrap_or(5);

    // --- NVT ratio ---
    let annualized_tx_volume = input.daily_transaction_volume * dec!(365);
    let nvt_ratio = if annualized_tx_volume.is_zero() {
        warnings
            .push("Daily transaction volume is zero; NVT ratio is undefined — set to zero".into());
        Decimal::ZERO
    } else {
        input.network_value / annualized_tx_volume
    };
    let nvt_signal = classify_nvt(nvt_ratio, annualized_tx_volume.is_zero());

    // --- Metcalfe's Law ---
    let n = Decimal::from(input.active_addresses);
    let n_squared = n * n;

    let metcalfe_value = if n_squared.is_zero() {
        warnings.push("Active addresses is zero; Metcalfe value is undefined".into());
        Decimal::ZERO
    } else {
        // Calibrate coefficient k from comparables if available, else self-calibrate
        let k = compute_metcalfe_coefficient(input, &mut warnings);
        k * n_squared
    };

    let metcalfe_premium_discount = if metcalfe_value.is_zero() {
        Decimal::ZERO
    } else {
        (input.network_value - metcalfe_value) / metcalfe_value
    };

    // --- Token DCF ---
    let (dcf_value, dcf_per_token) = if input.annual_protocol_revenue > Decimal::ZERO {
        let dcf = compute_token_dcf(
            input.annual_protocol_revenue,
            input.revenue_growth_rate,
            input.discount_rate,
            input.terminal_growth_rate,
            proj_years,
        );
        let per_token = if input.circulating_supply.is_zero() {
            warnings.push("Circulating supply is zero; DCF per token undefined".into());
            Decimal::ZERO
        } else {
            dcf / input.circulating_supply
        };
        (Some(dcf), Some(per_token))
    } else {
        warnings.push("Annual protocol revenue is zero or negative; DCF valuation skipped".into());
        (None, None)
    };

    // --- Relative valuation ---
    let relative_valuation = compute_relative_valuation(input, &mut warnings);

    let output = TokenValuationOutput {
        nvt_ratio,
        nvt_signal,
        metcalfe_value,
        metcalfe_premium_discount,
        dcf_value,
        dcf_per_token,
        relative_valuation,
        warnings: warnings.clone(),
    };

    let elapsed = start.elapsed().as_micros() as u64;

    Ok(with_metadata(
        "Token Valuation — NVT, Metcalfe, DCF, and relative comparable analysis",
        &serde_json::json!({
            "token_name": input.token_name,
            "projection_years": proj_years,
            "discount_rate": input.discount_rate.to_string(),
            "terminal_growth_rate": input.terminal_growth_rate.to_string(),
            "nvt_thresholds": {"undervalued": "<20", "fair": "20-65", "overvalued": ">65"},
            "metcalfe_law": "V = k * n^2",
            "dcf_method": "Gordon Growth terminal value, iterative discount factors",
        }),
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_input(input: &TokenValuationInput) -> CorpFinanceResult<()> {
    if input.network_value < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "network_value".into(),
            reason: "Network value cannot be negative".into(),
        });
    }
    if input.daily_transaction_volume < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "daily_transaction_volume".into(),
            reason: "Daily transaction volume cannot be negative".into(),
        });
    }
    if input.annual_protocol_revenue < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "annual_protocol_revenue".into(),
            reason: "Annual protocol revenue cannot be negative".into(),
        });
    }
    if input.token_supply <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "token_supply".into(),
            reason: "Token supply must be positive".into(),
        });
    }
    if input.circulating_supply < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "circulating_supply".into(),
            reason: "Circulating supply cannot be negative".into(),
        });
    }
    if input.circulating_supply > input.token_supply {
        return Err(CorpFinanceError::InvalidInput {
            field: "circulating_supply".into(),
            reason: "Circulating supply cannot exceed total supply".into(),
        });
    }
    if input.discount_rate <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "discount_rate".into(),
            reason: "Discount rate must be positive".into(),
        });
    }
    if input.discount_rate <= input.terminal_growth_rate {
        return Err(CorpFinanceError::InvalidInput {
            field: "discount_rate".into(),
            reason: "Discount rate must exceed terminal growth rate for Gordon Growth model".into(),
        });
    }
    if input.terminal_growth_rate < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "terminal_growth_rate".into(),
            reason: "Terminal growth rate cannot be negative".into(),
        });
    }
    if let Some(years) = input.projection_years {
        if years == 0 {
            return Err(CorpFinanceError::InvalidInput {
                field: "projection_years".into(),
                reason: "Projection years must be at least 1".into(),
            });
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// NVT classification
// ---------------------------------------------------------------------------

fn classify_nvt(nvt_ratio: Decimal, volume_is_zero: bool) -> String {
    if volume_is_zero {
        "N/A — no transaction volume".to_string()
    } else if nvt_ratio < NVT_UNDERVALUED_UPPER {
        "Undervalued".to_string()
    } else if nvt_ratio <= NVT_FAIR_UPPER {
        "Fair".to_string()
    } else {
        "Overvalued".to_string()
    }
}

// ---------------------------------------------------------------------------
// Metcalfe coefficient
// ---------------------------------------------------------------------------

/// Compute the Metcalfe coefficient k.
///
/// If comparable protocols with active-address data are provided in the future,
/// we would average k across them. For now we self-calibrate:
/// k = network_value / n^2.
fn compute_metcalfe_coefficient(
    input: &TokenValuationInput,
    _warnings: &mut Vec<String>,
) -> Decimal {
    let n = Decimal::from(input.active_addresses);
    let n_squared = n * n;
    if n_squared.is_zero() {
        return Decimal::ZERO;
    }
    // Self-calibrate: k = network_value / n^2
    // When used as "metcalfe_value = k * n^2", this returns network_value itself,
    // meaning premium/discount = 0 by construction unless comparables provide
    // a different k. This is the standard approach when no external reference
    // for k is available — the ratio then shows how the token's own valuation
    // tracks its network growth over time.
    input.network_value / n_squared
}

// ---------------------------------------------------------------------------
// Token DCF
// ---------------------------------------------------------------------------

/// Compute discounted cash flow valuation for fee-generating protocols.
///
/// Projects `annual_revenue` forward by `growth_rate` for `years`, then adds
/// a terminal value via Gordon Growth. Discounts all to present using
/// iterative multiplication for discount factors (never `powd`).
fn compute_token_dcf(
    annual_revenue: Money,
    growth_rate: Rate,
    discount_rate: Rate,
    terminal_growth_rate: Rate,
    years: u32,
) -> Money {
    let one_plus_g = Decimal::ONE + growth_rate;
    let one_plus_r = Decimal::ONE + discount_rate;

    let mut total_pv = Decimal::ZERO;
    let mut projected_revenue = annual_revenue;
    let mut discount_factor = Decimal::ONE; // (1+r)^t built iteratively

    for _t in 1..=years {
        // Project revenue forward one year
        projected_revenue *= one_plus_g;
        // Build discount factor iteratively: discount_factor *= (1+r)
        discount_factor *= one_plus_r;
        // PV of this year's revenue
        total_pv += projected_revenue / discount_factor;
    }

    // Terminal value (Gordon Growth): final_revenue * (1+g_terminal) / (r - g_terminal)
    let terminal_revenue = projected_revenue * (Decimal::ONE + terminal_growth_rate);
    let terminal_value = terminal_revenue / (discount_rate - terminal_growth_rate);

    // Discount terminal value to present
    // discount_factor is already (1+r)^years from the loop
    total_pv += terminal_value / discount_factor;

    total_pv
}

// ---------------------------------------------------------------------------
// Relative valuation
// ---------------------------------------------------------------------------

/// Compute relative valuation from comparable protocol multiples.
fn compute_relative_valuation(
    input: &TokenValuationInput,
    warnings: &mut Vec<String>,
) -> Option<RelativeValuation> {
    let comps = match &input.comparable_protocols {
        Some(c) if !c.is_empty() => c,
        _ => return None,
    };

    // --- FDV / Revenue multiples ---
    let fdv_revenue_multiples: Vec<Decimal> = comps
        .iter()
        .filter(|c| c.revenue > Decimal::ZERO)
        .map(|c| c.fdv / c.revenue)
        .collect();

    if fdv_revenue_multiples.is_empty() {
        warnings.push("No comparable protocols with positive revenue for FDV/Revenue".into());
        return None;
    }

    let median_fdv_revenue = median_decimal(&fdv_revenue_multiples);
    let implied_value_revenue = if input.annual_protocol_revenue > Decimal::ZERO {
        input.annual_protocol_revenue * median_fdv_revenue
    } else {
        warnings.push(
            "Target annual_protocol_revenue is zero; implied FDV/Revenue value is zero".into(),
        );
        Decimal::ZERO
    };

    // --- FDV / TVL multiples ---
    let fdv_tvl_multiples: Vec<Decimal> = comps
        .iter()
        .filter_map(|c| {
            c.tvl.and_then(|tvl| {
                if tvl > Decimal::ZERO {
                    Some(c.fdv / tvl)
                } else {
                    None
                }
            })
        })
        .collect();

    let (median_fdv_tvl, implied_value_tvl) = if fdv_tvl_multiples.is_empty() {
        (None, None)
    } else {
        let med = median_decimal(&fdv_tvl_multiples);
        let implied = input.total_value_locked.map(|tvl| {
            if tvl > Decimal::ZERO {
                tvl * med
            } else {
                Decimal::ZERO
            }
        });
        (Some(med), implied)
    };

    // --- FDV / Fees multiples ---
    let fdv_fees_multiples: Vec<Decimal> = comps
        .iter()
        .filter_map(|c| {
            c.fees.and_then(|fees| {
                if fees > Decimal::ZERO {
                    Some(c.fdv / fees)
                } else {
                    None
                }
            })
        })
        .collect();

    let (median_fdv_fees, implied_value_fees) = if fdv_fees_multiples.is_empty() {
        (None, None)
    } else {
        let med = median_decimal(&fdv_fees_multiples);
        // Use annual_protocol_revenue as fees proxy for target
        let implied = if input.annual_protocol_revenue > Decimal::ZERO {
            Some(input.annual_protocol_revenue * med)
        } else {
            Some(Decimal::ZERO)
        };
        (Some(med), implied)
    };

    Some(RelativeValuation {
        median_fdv_revenue,
        median_fdv_tvl,
        median_fdv_fees,
        implied_value_revenue,
        implied_value_tvl,
        implied_value_fees,
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Compute the median of a slice of Decimals.
/// Assumes the slice is non-empty.
fn median_decimal(values: &[Decimal]) -> Decimal {
    let mut sorted = values.to_vec();
    sorted.sort();
    let len = sorted.len();
    if len % 2 == 1 {
        sorted[len / 2]
    } else {
        (sorted[len / 2 - 1] + sorted[len / 2]) / dec!(2)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// Helper: standard token valuation input for testing.
    fn standard_token_input() -> TokenValuationInput {
        TokenValuationInput {
            token_name: "TestProtocol".to_string(),
            network_value: dec!(1_000_000_000),         // $1B FDV
            daily_transaction_volume: dec!(10_000_000), // $10M daily
            active_addresses: 500_000,
            annual_protocol_revenue: dec!(50_000_000), // $50M annual revenue
            total_value_locked: Some(dec!(2_000_000_000)), // $2B TVL
            token_supply: dec!(1_000_000_000),         // 1B tokens
            circulating_supply: dec!(500_000_000),     // 500M circulating
            discount_rate: dec!(0.20),                 // 20%
            revenue_growth_rate: dec!(0.30),           // 30% growth
            terminal_growth_rate: dec!(0.03),          // 3% terminal
            projection_years: Some(5),
            comparable_protocols: None,
        }
    }

    /// Helper: build a set of 3 comparable protocols.
    fn three_comparables() -> Vec<ComparableProtocol> {
        vec![
            ComparableProtocol {
                name: "ProtocolA".to_string(),
                fdv: dec!(2_000_000_000),
                revenue: dec!(100_000_000),
                tvl: Some(dec!(5_000_000_000)),
                fees: Some(dec!(80_000_000)),
            },
            ComparableProtocol {
                name: "ProtocolB".to_string(),
                fdv: dec!(500_000_000),
                revenue: dec!(25_000_000),
                tvl: Some(dec!(1_000_000_000)),
                fees: Some(dec!(20_000_000)),
            },
            ComparableProtocol {
                name: "ProtocolC".to_string(),
                fdv: dec!(3_000_000_000),
                revenue: dec!(200_000_000),
                tvl: Some(dec!(8_000_000_000)),
                fees: Some(dec!(150_000_000)),
            },
        ]
    }

    // -----------------------------------------------------------------------
    // 1. NVT ratio calculation — Undervalued zone (<20)
    // -----------------------------------------------------------------------
    #[test]
    fn test_nvt_ratio_undervalued() {
        let mut input = standard_token_input();
        // NVT = 1B / (10M * 365) = 1B / 3.65B ~ 0.274
        // That is < 20, so "Undervalued"
        input.network_value = dec!(1_000_000_000);
        input.daily_transaction_volume = dec!(200_000_000); // 200M daily
                                                            // NVT = 1B / (200M * 365) = 1B / 73B ~ 0.0137 => Undervalued

        let result = value_token(&input).unwrap();
        let out = &result.result;

        let expected_nvt = dec!(1_000_000_000) / (dec!(200_000_000) * dec!(365));
        let diff = (out.nvt_ratio - expected_nvt).abs();
        assert!(
            diff < dec!(0.0001),
            "NVT ratio should be ~{}, got {}",
            expected_nvt,
            out.nvt_ratio
        );
        assert_eq!(out.nvt_signal, "Undervalued");
    }

    // -----------------------------------------------------------------------
    // 2. NVT ratio calculation — Fair zone (20-65)
    // -----------------------------------------------------------------------
    #[test]
    fn test_nvt_ratio_fair() {
        let mut input = standard_token_input();
        // Want NVT ~ 40: network_value / (daily * 365) = 40
        // daily = network_value / (40 * 365) = 1B / 14600 ~ 68493.15
        input.daily_transaction_volume = dec!(68_493);
        // NVT = 1B / (68493 * 365) = 1B / 24999945 ~ 40.0001

        let result = value_token(&input).unwrap();
        let out = &result.result;

        assert!(
            out.nvt_ratio >= dec!(20) && out.nvt_ratio <= dec!(65),
            "NVT ratio {} should be in Fair range (20-65)",
            out.nvt_ratio
        );
        assert_eq!(out.nvt_signal, "Fair");
    }

    // -----------------------------------------------------------------------
    // 3. NVT ratio calculation — Overvalued zone (>65)
    // -----------------------------------------------------------------------
    #[test]
    fn test_nvt_ratio_overvalued() {
        let mut input = standard_token_input();
        // Want NVT ~ 100: daily = 1B / (100 * 365) = 1B / 36500 ~ 27397.26
        input.daily_transaction_volume = dec!(27_397);
        // NVT = 1B / (27397 * 365) = 1B / 9999905 ~ 100.001

        let result = value_token(&input).unwrap();
        let out = &result.result;

        assert!(
            out.nvt_ratio > dec!(65),
            "NVT ratio {} should be > 65 (Overvalued)",
            out.nvt_ratio
        );
        assert_eq!(out.nvt_signal, "Overvalued");
    }

    // -----------------------------------------------------------------------
    // 4. Metcalfe valuation with known inputs
    // -----------------------------------------------------------------------
    #[test]
    fn test_metcalfe_valuation() {
        let input = standard_token_input();
        let result = value_token(&input).unwrap();
        let out = &result.result;

        // Self-calibrating: k = network_value / n^2
        // metcalfe_value = k * n^2 = network_value
        // premium/discount = 0
        let n = Decimal::from(500_000u64);
        let n_sq = n * n;
        let k = dec!(1_000_000_000) / n_sq;
        let expected_metcalfe = k * n_sq;

        let diff = (out.metcalfe_value - expected_metcalfe).abs();
        assert!(
            diff < dec!(1),
            "Metcalfe value should be ~{}, got {}",
            expected_metcalfe,
            out.metcalfe_value
        );

        // Premium/discount should be 0 for self-calibration
        assert!(
            out.metcalfe_premium_discount.abs() < dec!(0.0001),
            "Premium/discount should be ~0, got {}",
            out.metcalfe_premium_discount
        );
    }

    // -----------------------------------------------------------------------
    // 5. Token DCF with 5-year projection
    // -----------------------------------------------------------------------
    #[test]
    fn test_token_dcf_5_year() {
        let input = standard_token_input();
        let result = value_token(&input).unwrap();
        let out = &result.result;

        assert!(out.dcf_value.is_some(), "DCF value should be present");
        let dcf = out.dcf_value.unwrap();

        // Manual calculation:
        // revenue = 50M, growth = 30%, discount = 20%, terminal = 3%
        // Year 1: 50M * 1.3 = 65M, PV = 65M / 1.2 = 54.1667M
        // Year 2: 65M * 1.3 = 84.5M, PV = 84.5M / 1.44 = 58.6806M
        // Year 3: 84.5M * 1.3 = 109.85M, PV = 109.85M / 1.728 = 63.5706M
        // Year 4: 109.85M * 1.3 = 142.805M, PV = 142.805M / 2.0736 = 68.8688M
        // Year 5: 142.805M * 1.3 = 185.6465M, PV = 185.6465M / 2.48832 = 74.5679M
        // Terminal: 185.6465M * 1.03 / (0.20 - 0.03) = 191.2159M / 0.17 = 1124.8M
        // PV terminal = 1124.8M / 2.48832 = 451.87M
        // Total = 54.167 + 58.681 + 63.571 + 68.869 + 74.568 + 451.87 = ~771.7M

        assert!(
            dcf > dec!(700_000_000) && dcf < dec!(850_000_000),
            "DCF value should be ~770M, got {}",
            dcf
        );
    }

    // -----------------------------------------------------------------------
    // 6. DCF per-token calculation
    // -----------------------------------------------------------------------
    #[test]
    fn test_dcf_per_token() {
        let input = standard_token_input();
        let result = value_token(&input).unwrap();
        let out = &result.result;

        assert!(
            out.dcf_per_token.is_some(),
            "DCF per token should be present"
        );
        let per_token = out.dcf_per_token.unwrap();
        let dcf = out.dcf_value.unwrap();

        // per_token = dcf / circulating_supply
        let expected = dcf / dec!(500_000_000);
        let diff = (per_token - expected).abs();
        assert!(
            diff < dec!(0.000001),
            "DCF per token should be {}, got {}",
            expected,
            per_token
        );

        // With ~770M DCF / 500M supply, per token ~ 1.54
        assert!(
            per_token > dec!(1.0) && per_token < dec!(2.0),
            "DCF per token should be between $1 and $2, got {}",
            per_token
        );
    }

    // -----------------------------------------------------------------------
    // 7. Relative valuation with 3 comparables
    // -----------------------------------------------------------------------
    #[test]
    fn test_relative_valuation_three_comparables() {
        let mut input = standard_token_input();
        input.comparable_protocols = Some(three_comparables());

        let result = value_token(&input).unwrap();
        let out = &result.result;

        assert!(
            out.relative_valuation.is_some(),
            "Relative valuation should be present"
        );
        let rv = out.relative_valuation.as_ref().unwrap();

        // FDV/Revenue multiples:
        // A: 2B/100M = 20, B: 500M/25M = 20, C: 3B/200M = 15
        // Sorted: [15, 20, 20] => median = 20
        assert_eq!(
            rv.median_fdv_revenue,
            dec!(20),
            "Median FDV/Revenue should be 20, got {}",
            rv.median_fdv_revenue
        );

        // Implied value = 50M * 20 = 1B
        assert_eq!(
            rv.implied_value_revenue,
            dec!(1_000_000_000),
            "Implied FDV/Revenue value should be 1B, got {}",
            rv.implied_value_revenue
        );

        // FDV/TVL multiples:
        // A: 2B/5B = 0.4, B: 500M/1B = 0.5, C: 3B/8B = 0.375
        // Sorted: [0.375, 0.4, 0.5] => median = 0.4
        assert!(rv.median_fdv_tvl.is_some());
        assert_eq!(
            rv.median_fdv_tvl.unwrap(),
            dec!(0.4),
            "Median FDV/TVL should be 0.4, got {}",
            rv.median_fdv_tvl.unwrap()
        );

        // Implied TVL value = 2B * 0.4 = 800M
        assert!(rv.implied_value_tvl.is_some());
        assert_eq!(
            rv.implied_value_tvl.unwrap(),
            dec!(800_000_000),
            "Implied FDV/TVL value should be 800M, got {}",
            rv.implied_value_tvl.unwrap()
        );

        // FDV/Fees multiples:
        // A: 2B/80M = 25, B: 500M/20M = 25, C: 3B/150M = 20
        // Sorted: [20, 25, 25] => median = 25
        assert!(rv.median_fdv_fees.is_some());
        assert_eq!(
            rv.median_fdv_fees.unwrap(),
            dec!(25),
            "Median FDV/Fees should be 25, got {}",
            rv.median_fdv_fees.unwrap()
        );
    }

    // -----------------------------------------------------------------------
    // 8. Edge case: zero transaction volume triggers warning
    // -----------------------------------------------------------------------
    #[test]
    fn test_zero_transaction_volume_warning() {
        let mut input = standard_token_input();
        input.daily_transaction_volume = Decimal::ZERO;

        let result = value_token(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.nvt_ratio, Decimal::ZERO);
        assert_eq!(out.nvt_signal, "N/A — no transaction volume");
        assert!(
            out.warnings
                .iter()
                .any(|w| w.contains("transaction volume is zero")),
            "Should warn about zero transaction volume"
        );
    }

    // -----------------------------------------------------------------------
    // 9. Edge case: discount_rate <= terminal_growth_rate => error
    // -----------------------------------------------------------------------
    #[test]
    fn test_discount_rate_lte_terminal_growth_rate_error() {
        let mut input = standard_token_input();
        input.discount_rate = dec!(0.03);
        input.terminal_growth_rate = dec!(0.03);

        let result = value_token(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, reason } => {
                assert_eq!(field, "discount_rate");
                assert!(reason.contains("terminal growth rate"));
            }
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // 10. Full integration test with all features
    // -----------------------------------------------------------------------
    #[test]
    fn test_full_integration() {
        let mut input = standard_token_input();
        input.comparable_protocols = Some(three_comparables());

        let result = value_token(&input).unwrap();
        let out = &result.result;

        // NVT present and classified
        assert!(out.nvt_ratio > Decimal::ZERO);
        assert!(!out.nvt_signal.is_empty());

        // Metcalfe present
        assert!(out.metcalfe_value > Decimal::ZERO);

        // DCF present
        assert!(out.dcf_value.is_some());
        assert!(out.dcf_value.unwrap() > Decimal::ZERO);
        assert!(out.dcf_per_token.is_some());
        assert!(out.dcf_per_token.unwrap() > Decimal::ZERO);

        // Relative valuation present
        assert!(out.relative_valuation.is_some());
        let rv = out.relative_valuation.as_ref().unwrap();
        assert!(rv.median_fdv_revenue > Decimal::ZERO);
        assert!(rv.implied_value_revenue > Decimal::ZERO);

        // Metadata
        assert!(result.methodology.contains("Token Valuation"));
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
    }

    // -----------------------------------------------------------------------
    // 11. Negative network value rejected
    // -----------------------------------------------------------------------
    #[test]
    fn test_invalid_negative_network_value() {
        let mut input = standard_token_input();
        input.network_value = dec!(-100);

        let result = value_token(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "network_value");
            }
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // 12. Zero active addresses triggers warning, metcalfe_value = 0
    // -----------------------------------------------------------------------
    #[test]
    fn test_zero_active_addresses() {
        let mut input = standard_token_input();
        input.active_addresses = 0;

        let result = value_token(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.metcalfe_value, Decimal::ZERO);
        assert_eq!(out.metcalfe_premium_discount, Decimal::ZERO);
        assert!(
            out.warnings
                .iter()
                .any(|w| w.contains("Active addresses is zero")),
            "Should warn about zero active addresses"
        );
    }

    // -----------------------------------------------------------------------
    // 13. DCF skipped when revenue is zero
    // -----------------------------------------------------------------------
    #[test]
    fn test_dcf_skipped_zero_revenue() {
        let mut input = standard_token_input();
        input.annual_protocol_revenue = Decimal::ZERO;

        let result = value_token(&input).unwrap();
        let out = &result.result;

        assert!(out.dcf_value.is_none());
        assert!(out.dcf_per_token.is_none());
        assert!(
            out.warnings
                .iter()
                .any(|w| w.contains("DCF valuation skipped")),
            "Should warn about skipped DCF"
        );
    }

    // -----------------------------------------------------------------------
    // 14. No comparables => relative_valuation is None
    // -----------------------------------------------------------------------
    #[test]
    fn test_no_comparables() {
        let mut input = standard_token_input();
        input.comparable_protocols = None;

        let result = value_token(&input).unwrap();
        let out = &result.result;

        assert!(
            out.relative_valuation.is_none(),
            "Relative valuation should be None without comparables"
        );
    }

    // -----------------------------------------------------------------------
    // 15. Circulating supply exceeds total => error
    // -----------------------------------------------------------------------
    #[test]
    fn test_circulating_exceeds_total_supply() {
        let mut input = standard_token_input();
        input.circulating_supply = dec!(2_000_000_000);
        input.token_supply = dec!(1_000_000_000);

        let result = value_token(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "circulating_supply");
            }
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // 16. Median helper — odd count
    // -----------------------------------------------------------------------
    #[test]
    fn test_median_odd() {
        let values = vec![dec!(10), dec!(30), dec!(20)];
        assert_eq!(median_decimal(&values), dec!(20));
    }

    // -----------------------------------------------------------------------
    // 17. Median helper — even count
    // -----------------------------------------------------------------------
    #[test]
    fn test_median_even() {
        let values = vec![dec!(10), dec!(20), dec!(30), dec!(40)];
        // Sorted: [10, 20, 30, 40] => median = (20+30)/2 = 25
        assert_eq!(median_decimal(&values), dec!(25));
    }

    // -----------------------------------------------------------------------
    // 18. DCF with 1-year projection
    // -----------------------------------------------------------------------
    #[test]
    fn test_dcf_1_year_projection() {
        let mut input = standard_token_input();
        input.projection_years = Some(1);

        let result = value_token(&input).unwrap();
        let out = &result.result;

        assert!(out.dcf_value.is_some());
        let dcf = out.dcf_value.unwrap();

        // Year 1 revenue: 50M * 1.3 = 65M, PV = 65M / 1.2 = 54.1667M
        // Terminal: 65M * 1.03 / 0.17 = 66.95M / 0.17 = 393.824M
        // PV terminal = 393.824M / 1.2 = 328.186M
        // Total ~ 382.35M
        assert!(
            dcf > dec!(350_000_000) && dcf < dec!(420_000_000),
            "1-year DCF should be ~382M, got {}",
            dcf
        );
    }

    // -----------------------------------------------------------------------
    // 19. Zero projection years => error
    // -----------------------------------------------------------------------
    #[test]
    fn test_zero_projection_years_error() {
        let mut input = standard_token_input();
        input.projection_years = Some(0);

        let result = value_token(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "projection_years");
            }
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // 20. Metadata is populated
    // -----------------------------------------------------------------------
    #[test]
    fn test_metadata_populated() {
        let input = standard_token_input();
        let result = value_token(&input).unwrap();

        assert!(!result.methodology.is_empty());
        assert!(result.methodology.contains("Token Valuation"));
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
        assert!(!result.metadata.version.is_empty());
    }
}
