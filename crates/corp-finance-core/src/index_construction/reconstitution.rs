//! Index Reconstitution Analysis.
//!
//! Covers:
//! 1. **Additions** -- candidates meeting criteria above buffer threshold
//! 2. **Deletions** -- current members failing criteria below buffer threshold
//! 3. **Buffer Zone** -- members near thresholds to prevent excessive turnover
//! 4. **Turnover** -- reconstitution-driven turnover measurement
//! 5. **Impact Analysis** -- estimated buy/sell pressure from recon changes
//!
//! All arithmetic uses `rust_decimal::Decimal`. No `f64`.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// An index member (current or candidate).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexMember {
    pub ticker: String,
    pub market_cap: Decimal,
    pub meets_criteria: bool,
    pub float_pct: Decimal,
    pub avg_volume: Decimal,
}

/// A reconstitution action (addition or deletion).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconAction {
    pub ticker: String,
    pub market_cap: Decimal,
    pub reason: String,
}

/// Impact metrics from reconstitution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconImpact {
    pub estimated_buy_pressure: Decimal,
    pub estimated_sell_pressure: Decimal,
    pub net_flow: Decimal,
}

// ---------------------------------------------------------------------------
// Input / Output
// ---------------------------------------------------------------------------

/// Input for reconstitution analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconstitutionInput {
    pub current_members: Vec<IndexMember>,
    pub candidates: Vec<IndexMember>,
    pub min_market_cap: Decimal,
    pub min_float_pct: Decimal,
    pub min_volume: Decimal,
    pub max_members: u32,
    /// Buffer zone around threshold (e.g. 0.10 = 10%).
    pub buffer_zone_pct: Decimal,
}

/// Output of the reconstitution analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconstitutionOutput {
    pub additions: Vec<ReconAction>,
    pub deletions: Vec<ReconAction>,
    pub retained: Vec<String>,
    pub turnover: Decimal,
    pub new_member_count: u32,
    pub avg_market_cap_before: Decimal,
    pub avg_market_cap_after: Decimal,
    pub buffer_zone_members: Vec<String>,
    pub reconstitution_impact: ReconImpact,
}

// ---------------------------------------------------------------------------
// Calculation
// ---------------------------------------------------------------------------

/// Perform index reconstitution analysis.
pub fn calculate_reconstitution(
    input: &ReconstitutionInput,
) -> CorpFinanceResult<ReconstitutionOutput> {
    validate_reconstitution_input(input)?;

    let lower_threshold = input.min_market_cap * (Decimal::ONE - input.buffer_zone_pct);
    let upper_threshold = input.min_market_cap * (Decimal::ONE + input.buffer_zone_pct);

    // Determine deletions: current members that fail criteria and are below buffer
    let mut deletions: Vec<ReconAction> = Vec::new();
    let mut retained: Vec<String> = Vec::new();
    let mut buffer_zone_members: Vec<String> = Vec::new();

    for m in &input.current_members {
        let in_buffer = m.market_cap >= lower_threshold && m.market_cap <= upper_threshold;

        if in_buffer {
            buffer_zone_members.push(m.ticker.clone());
        }

        if !m.meets_criteria && m.market_cap < lower_threshold {
            deletions.push(ReconAction {
                ticker: m.ticker.clone(),
                market_cap: m.market_cap,
                reason: format!(
                    "Fails criteria, market cap {} below buffer threshold {}",
                    m.market_cap, lower_threshold
                ),
            });
        } else {
            retained.push(m.ticker.clone());
        }
    }

    // Available slots after deletions
    let retained_count = retained.len() as u32;
    let available_slots = input.max_members.saturating_sub(retained_count);

    // Determine additions: candidates that meet criteria, above upper threshold, and pass filters
    let mut qualified_candidates: Vec<&IndexMember> = input
        .candidates
        .iter()
        .filter(|c| {
            c.meets_criteria
                && c.market_cap > upper_threshold
                && c.float_pct >= input.min_float_pct
                && c.avg_volume >= input.min_volume
        })
        .collect();

    // Sort by market cap descending
    qualified_candidates.sort_by(|a, b| b.market_cap.cmp(&a.market_cap));

    // Take up to available slots
    let additions: Vec<ReconAction> = qualified_candidates
        .iter()
        .take(available_slots as usize)
        .map(|c| ReconAction {
            ticker: c.ticker.clone(),
            market_cap: c.market_cap,
            reason: format!(
                "Meets criteria, market cap {} above buffer threshold {}",
                c.market_cap, upper_threshold
            ),
        })
        .collect();

    // Turnover
    let current_count = input.current_members.len() as u32;
    let changes = (additions.len() + deletions.len()) as u64;
    let turnover = if current_count == 0 {
        Decimal::ZERO
    } else {
        Decimal::from(changes) / (dec!(2) * Decimal::from(current_count as u64))
    };

    // New member count
    let new_member_count = retained_count + additions.len() as u32;

    // Average market cap before
    let avg_market_cap_before = if input.current_members.is_empty() {
        Decimal::ZERO
    } else {
        let total: Decimal = input.current_members.iter().map(|m| m.market_cap).sum();
        total / Decimal::from(input.current_members.len() as u64)
    };

    // Average market cap after: retained members + additions
    let retained_mc: Decimal = input
        .current_members
        .iter()
        .filter(|m| retained.contains(&m.ticker))
        .map(|m| m.market_cap)
        .sum();
    let additions_mc: Decimal = additions.iter().map(|a| a.market_cap).sum();
    let avg_market_cap_after = if new_member_count == 0 {
        Decimal::ZERO
    } else {
        (retained_mc + additions_mc) / Decimal::from(new_member_count as u64)
    };

    // Impact
    let estimated_buy_pressure: Decimal = additions.iter().map(|a| a.market_cap).sum();
    let estimated_sell_pressure: Decimal = deletions.iter().map(|d| d.market_cap).sum();
    let net_flow = estimated_buy_pressure - estimated_sell_pressure;

    Ok(ReconstitutionOutput {
        additions,
        deletions,
        retained,
        turnover,
        new_member_count,
        avg_market_cap_before,
        avg_market_cap_after,
        buffer_zone_members,
        reconstitution_impact: ReconImpact {
            estimated_buy_pressure,
            estimated_sell_pressure,
            net_flow,
        },
    })
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_reconstitution_input(input: &ReconstitutionInput) -> CorpFinanceResult<()> {
    if input.current_members.is_empty() && input.candidates.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one current member or candidate is required".into(),
        ));
    }
    if input.min_market_cap < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "min_market_cap".into(),
            reason: "Minimum market cap must be non-negative".into(),
        });
    }
    if input.min_float_pct < Decimal::ZERO || input.min_float_pct > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "min_float_pct".into(),
            reason: "Minimum float percentage must be between 0 and 1".into(),
        });
    }
    if input.buffer_zone_pct < Decimal::ZERO || input.buffer_zone_pct > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "buffer_zone_pct".into(),
            reason: "Buffer zone must be between 0 and 1".into(),
        });
    }
    if input.max_members == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "max_members".into(),
            reason: "Max members must be positive".into(),
        });
    }
    if input.min_volume < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "min_volume".into(),
            reason: "Minimum volume must be non-negative".into(),
        });
    }
    for m in input.current_members.iter().chain(input.candidates.iter()) {
        if m.market_cap < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "market_cap".into(),
                reason: format!("Negative market cap for {}", m.ticker),
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

    fn make_member(ticker: &str, mc: Decimal, meets: bool) -> IndexMember {
        IndexMember {
            ticker: ticker.into(),
            market_cap: mc,
            meets_criteria: meets,
            float_pct: dec!(0.50),
            avg_volume: dec!(500_000),
        }
    }

    fn make_base_input() -> ReconstitutionInput {
        ReconstitutionInput {
            current_members: vec![
                make_member("A", dec!(5000), true),
                make_member("B", dec!(3000), true),
                make_member("C", dec!(2000), true),
                make_member("D", dec!(800), false), // fails criteria, below threshold
                make_member("E", dec!(1200), true),
            ],
            candidates: vec![
                make_member("X", dec!(4000), true),
                make_member("Y", dec!(2500), true),
                make_member("Z", dec!(500), true), // below threshold
            ],
            min_market_cap: dec!(1000),
            min_float_pct: dec!(0.25),
            min_volume: dec!(100_000),
            max_members: 5,
            buffer_zone_pct: dec!(0.10),
        }
    }

    // --- No changes scenario ---
    #[test]
    fn test_no_changes_all_qualify() {
        let mut input = make_base_input();
        // All current members pass, no slots for additions
        input.current_members = vec![
            make_member("A", dec!(5000), true),
            make_member("B", dec!(3000), true),
            make_member("C", dec!(2000), true),
            make_member("D", dec!(1500), true),
            make_member("E", dec!(1200), true),
        ];
        let out = calculate_reconstitution(&input).unwrap();
        assert_eq!(out.deletions.len(), 0);
        assert_eq!(out.additions.len(), 0);
        assert_eq!(out.retained.len(), 5);
    }

    // --- Deletions ---
    #[test]
    fn test_deletion_below_buffer() {
        let input = make_base_input();
        let out = calculate_reconstitution(&input).unwrap();
        // D: mc=800, fails criteria, 800 < 1000*(1-0.10)=900 -> deleted
        let deleted_tickers: Vec<&str> = out.deletions.iter().map(|d| d.ticker.as_str()).collect();
        assert!(deleted_tickers.contains(&"D"));
    }

    #[test]
    fn test_deletion_reason() {
        let input = make_base_input();
        let out = calculate_reconstitution(&input).unwrap();
        assert!(!out.deletions.is_empty());
        assert!(out.deletions[0].reason.contains("Fails criteria"));
    }

    // --- Additions ---
    #[test]
    fn test_additions_fill_slots() {
        let input = make_base_input();
        let out = calculate_reconstitution(&input).unwrap();
        // D deleted (1 slot), X qualifies (mc=4000 > 1100 upper threshold), Y qualifies (2500 > 1100)
        // But only 1 slot available (5 retained - 1 deletion = 4, max 5, so 1 slot)
        assert!(!out.additions.is_empty());
    }

    #[test]
    fn test_additions_sorted_by_market_cap() {
        let mut input = make_base_input();
        input.max_members = 10; // lots of room
        let out = calculate_reconstitution(&input).unwrap();
        // X (4000) should be before Y (2500) in additions
        if out.additions.len() >= 2 {
            assert!(out.additions[0].market_cap >= out.additions[1].market_cap);
        }
    }

    #[test]
    fn test_additions_only() {
        let mut input = make_base_input();
        input.current_members = vec![make_member("A", dec!(5000), true)];
        input.max_members = 5;
        let out = calculate_reconstitution(&input).unwrap();
        assert!(out.deletions.is_empty());
        assert!(!out.additions.is_empty());
    }

    #[test]
    fn test_deletions_only() {
        let mut input = make_base_input();
        input.candidates = vec![]; // no candidates
        let out = calculate_reconstitution(&input).unwrap();
        assert!(!out.deletions.is_empty());
        assert!(out.additions.is_empty());
    }

    // --- Buffer zone ---
    #[test]
    fn test_buffer_zone_prevents_churn() {
        let mut input = make_base_input();
        // D has mc=950, fails criteria but within buffer (900-1100)
        input.current_members[3] = make_member("D", dec!(950), false);
        let out = calculate_reconstitution(&input).unwrap();
        // D should NOT be deleted (within buffer zone)
        let deleted_tickers: Vec<&str> = out.deletions.iter().map(|d| d.ticker.as_str()).collect();
        assert!(!deleted_tickers.contains(&"D"));
    }

    #[test]
    fn test_buffer_zone_members_identified() {
        let mut input = make_base_input();
        // E has mc=1050, within buffer zone (900-1100)
        input.current_members[4] = make_member("E", dec!(1050), true);
        let out = calculate_reconstitution(&input).unwrap();
        assert!(out.buffer_zone_members.contains(&"E".to_string()));
    }

    // --- Market cap sorting ---
    #[test]
    fn test_candidates_below_threshold_excluded() {
        let input = make_base_input();
        let out = calculate_reconstitution(&input).unwrap();
        // Z: mc=500 < 1100 upper threshold -> not added
        let added_tickers: Vec<&str> = out.additions.iter().map(|a| a.ticker.as_str()).collect();
        assert!(!added_tickers.contains(&"Z"));
    }

    // --- Volume filter ---
    #[test]
    fn test_volume_filter() {
        let mut input = make_base_input();
        input.candidates[0].avg_volume = dec!(50_000); // below min 100k
        input.max_members = 10;
        let out = calculate_reconstitution(&input).unwrap();
        let added_tickers: Vec<&str> = out.additions.iter().map(|a| a.ticker.as_str()).collect();
        assert!(!added_tickers.contains(&"X"));
    }

    // --- Float filter ---
    #[test]
    fn test_float_filter() {
        let mut input = make_base_input();
        input.candidates[0].float_pct = dec!(0.10); // below min 0.25
        input.max_members = 10;
        let out = calculate_reconstitution(&input).unwrap();
        let added_tickers: Vec<&str> = out.additions.iter().map(|a| a.ticker.as_str()).collect();
        assert!(!added_tickers.contains(&"X"));
    }

    // --- Max members respected ---
    #[test]
    fn test_max_members_respected() {
        let input = make_base_input();
        let out = calculate_reconstitution(&input).unwrap();
        assert!(out.new_member_count <= input.max_members);
    }

    // --- Turnover ---
    #[test]
    fn test_turnover_calculation() {
        let input = make_base_input();
        let out = calculate_reconstitution(&input).unwrap();
        // turnover = (additions + deletions) / (2 * current)
        let expected = Decimal::from((out.additions.len() + out.deletions.len()) as u64)
            / (dec!(2) * Decimal::from(input.current_members.len() as u64));
        assert!(approx_eq(out.turnover, expected, dec!(0.001)));
    }

    #[test]
    fn test_turnover_zero_no_changes() {
        let mut input = make_base_input();
        input.current_members = vec![
            make_member("A", dec!(5000), true),
            make_member("B", dec!(3000), true),
            make_member("C", dec!(2000), true),
            make_member("D", dec!(1500), true),
            make_member("E", dec!(1200), true),
        ];
        let out = calculate_reconstitution(&input).unwrap();
        assert_eq!(out.turnover, Decimal::ZERO);
    }

    // --- Average market cap ---
    #[test]
    fn test_avg_market_cap_before() {
        let input = make_base_input();
        let out = calculate_reconstitution(&input).unwrap();
        // (5000+3000+2000+800+1200)/5 = 2400
        assert!(approx_eq(out.avg_market_cap_before, dec!(2400), dec!(1)));
    }

    #[test]
    fn test_avg_market_cap_after_changes() {
        let input = make_base_input();
        let out = calculate_reconstitution(&input).unwrap();
        // After reconstitution, should differ from before
        assert!(out.avg_market_cap_after > Decimal::ZERO);
    }

    // --- Impact ---
    #[test]
    fn test_buy_pressure() {
        let input = make_base_input();
        let out = calculate_reconstitution(&input).unwrap();
        let add_mc: Decimal = out.additions.iter().map(|a| a.market_cap).sum();
        assert_eq!(out.reconstitution_impact.estimated_buy_pressure, add_mc);
    }

    #[test]
    fn test_sell_pressure() {
        let input = make_base_input();
        let out = calculate_reconstitution(&input).unwrap();
        let del_mc: Decimal = out.deletions.iter().map(|d| d.market_cap).sum();
        assert_eq!(out.reconstitution_impact.estimated_sell_pressure, del_mc);
    }

    #[test]
    fn test_net_flow() {
        let input = make_base_input();
        let out = calculate_reconstitution(&input).unwrap();
        let expected = out.reconstitution_impact.estimated_buy_pressure
            - out.reconstitution_impact.estimated_sell_pressure;
        assert_eq!(out.reconstitution_impact.net_flow, expected);
    }

    // --- Validation ---
    #[test]
    fn test_reject_all_empty() {
        let input = ReconstitutionInput {
            current_members: vec![],
            candidates: vec![],
            min_market_cap: dec!(1000),
            min_float_pct: dec!(0.25),
            min_volume: dec!(100_000),
            max_members: 5,
            buffer_zone_pct: dec!(0.10),
        };
        assert!(calculate_reconstitution(&input).is_err());
    }

    #[test]
    fn test_reject_negative_min_market_cap() {
        let mut input = make_base_input();
        input.min_market_cap = dec!(-100);
        assert!(calculate_reconstitution(&input).is_err());
    }

    #[test]
    fn test_reject_invalid_float_pct() {
        let mut input = make_base_input();
        input.min_float_pct = dec!(1.5);
        assert!(calculate_reconstitution(&input).is_err());
    }

    #[test]
    fn test_reject_zero_max_members() {
        let mut input = make_base_input();
        input.max_members = 0;
        assert!(calculate_reconstitution(&input).is_err());
    }

    #[test]
    fn test_reject_invalid_buffer() {
        let mut input = make_base_input();
        input.buffer_zone_pct = dec!(1.5);
        assert!(calculate_reconstitution(&input).is_err());
    }

    #[test]
    fn test_reject_negative_member_market_cap() {
        let mut input = make_base_input();
        input.current_members[0].market_cap = dec!(-100);
        assert!(calculate_reconstitution(&input).is_err());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let input = make_base_input();
        let out = calculate_reconstitution(&input).unwrap();
        let json = serde_json::to_string(&out).unwrap();
        let _: ReconstitutionOutput = serde_json::from_str(&json).unwrap();
    }
}
