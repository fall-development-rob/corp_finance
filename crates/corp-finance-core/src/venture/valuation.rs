use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{with_metadata, ComputationOutput, Money};
use crate::CorpFinanceResult;

// ─── Enums ───────────────────────────────────────────────────────────────────

/// Liquidation preference type for a preferred-stock round.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LiqPref {
    /// Investor gets back investment OR converts to common (higher of the two).
    NonParticipating,
    /// Investor gets back investment AND participates in remaining proceeds pro-rata.
    Participating,
    /// Like Participating, but participation is capped at a multiple of the investment.
    CappedParticipating,
}

// ─── Structs ─────────────────────────────────────────────────────────────────

/// An existing shareholder (pre-round).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shareholder {
    pub name: String,
    pub shares: u64,
    pub share_class: String,
}

/// Input for modelling a single funding round.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingRoundInput {
    /// Pre-money valuation of the company.
    pub pre_money_valuation: Money,
    /// Amount the new investor is investing.
    pub investment_amount: Money,
    /// Total shares outstanding before this round.
    pub existing_shares: u64,
    /// List of current shareholders.
    pub existing_shareholders: Vec<Shareholder>,
    /// Target option pool as a percentage of *post-money* fully diluted shares.
    /// E.g., `Some(dec!(0.10))` means 10%. Created pre-money (dilutes founders, not investor).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub option_pool_pct: Option<Decimal>,
    /// Shares already allocated to the option pool (included in `existing_shares`).
    pub option_pool_shares_existing: u64,
    /// Round label, e.g. "Series A".
    pub round_name: String,
    /// Liquidation preference type.
    pub liquidation_preference: LiqPref,
    /// Participation cap as a multiple of investment (e.g. 3.0x). Only meaningful for
    /// `CappedParticipating`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub participation_cap: Option<Decimal>,
}

/// A single row in the cap table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapTableEntry {
    pub name: String,
    pub shares: u64,
    pub ownership_pct: Decimal,
    pub value_at_post_money: Money,
}

/// Output of a single funding-round model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingRoundOutput {
    pub post_money_valuation: Money,
    /// Price per share = pre_money / (existing_shares + option_pool_new_shares).
    pub price_per_share: Money,
    /// Shares issued to the new investor.
    pub new_shares_issued: u64,
    /// New shares added to the option pool in this round.
    pub option_pool_new_shares: u64,
    /// Total shares outstanding after this round (including new option pool shares).
    pub total_shares_post_round: u64,
    /// New investor's ownership percentage post-round.
    pub investor_ownership_pct: Decimal,
    /// How much existing holders were diluted (1 - old_pct_sum / 100%).
    pub founder_dilution_pct: Decimal,
    /// Fully-diluted cap table.
    pub cap_table: Vec<CapTableEntry>,
    /// Total fully diluted shares (all shares + all options).
    pub fully_diluted_shares: u64,
}

// ─── Dilution analysis types ─────────────────────────────────────────────────

/// Specification for a single round in a multi-round dilution analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundSpec {
    pub name: String,
    pub pre_money_valuation: Money,
    pub investment_amount: Money,
    /// Target option pool as % of post-money for this round.
    pub option_pool_pct: Decimal,
}

/// Founder specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FounderSpec {
    pub name: String,
    pub initial_shares: u64,
}

/// Input for multi-round dilution analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DilutionInput {
    /// Rounds in chronological order.
    pub rounds: Vec<RoundSpec>,
    /// Total founder shares at incorporation.
    pub initial_shares: u64,
    /// Founder breakdown.
    pub founders: Vec<FounderSpec>,
}

/// Summary of a single round's outcome within a dilution analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundResult {
    pub round_name: String,
    pub pre_money: Money,
    pub post_money: Money,
    pub price_per_share: Money,
    pub new_shares: u64,
    pub option_pool_increase: u64,
    pub total_shares: u64,
}

/// A point in the founder ownership trajectory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwnershipPoint {
    pub round_name: String,
    pub ownership_pct: Decimal,
    pub value_at_post_money: Money,
}

/// Output of a multi-round dilution analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DilutionOutput {
    pub rounds: Vec<RoundResult>,
    pub final_cap_table: Vec<CapTableEntry>,
    /// One entry per round per founder (flattened: founders outer, rounds inner).
    pub founder_ownership_trajectory: Vec<OwnershipPoint>,
}

// ─── Helper: convert u64 to Decimal ──────────────────────────────────────────

fn dec_from_u64(n: u64) -> Decimal {
    Decimal::from(n)
}

/// Safely convert a non-negative Decimal to u64 by truncating fractional part.
fn to_u64_truncated(d: Decimal) -> u64 {
    // rust_decimal::to_u64 requires the value to be non-negative and have no fractional part
    // We explicitly truncate first.
    let truncated = d.trunc();
    if truncated.is_sign_negative() {
        0
    } else {
        truncated.to_string().parse::<u64>().unwrap_or(0)
    }
}

// ─── Function 1: model_funding_round ─────────────────────────────────────────

/// Model a single VC funding round with option-pool shuffle.
///
/// The option pool is created *pre-money*, diluting existing shareholders but not the
/// incoming investor. This is the standard "option pool shuffle" mechanism.
pub fn model_funding_round(
    input: &FundingRoundInput,
) -> CorpFinanceResult<ComputationOutput<FundingRoundOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // ── Validation ───────────────────────────────────────────────────
    if input.pre_money_valuation <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "pre_money_valuation".into(),
            reason: "Pre-money valuation must be positive".into(),
        });
    }
    if input.investment_amount <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "investment_amount".into(),
            reason: "Investment amount must be positive".into(),
        });
    }
    if input.existing_shares == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "existing_shares".into(),
            reason: "Existing shares must be greater than zero".into(),
        });
    }

    let pool_pct = input.option_pool_pct.unwrap_or(Decimal::ZERO);
    if pool_pct < Decimal::ZERO || pool_pct >= Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "option_pool_pct".into(),
            reason: "Option pool percentage must be in [0, 1)".into(),
        });
    }

    // Validate shareholder shares sum
    let shareholder_shares_sum: u64 = input.existing_shareholders.iter().map(|s| s.shares).sum();
    let unaccounted = input
        .existing_shares
        .saturating_sub(shareholder_shares_sum)
        .saturating_sub(input.option_pool_shares_existing);
    if unaccounted > 0 {
        warnings.push(format!(
            "{} shares not accounted for by named shareholders or existing option pool",
            unaccounted
        ));
    }

    // ── Option pool shuffle ──────────────────────────────────────────
    //
    // The investor wants `pool_pct` of post-money to be the option pool.
    // Post-money shares = existing_shares + new_pool_shares + new_investor_shares
    //
    // Let E = existing_shares, P_new = new pool shares, I = investor shares
    // total_post = E + P_new + I
    // pool_target = (existing_pool + P_new) / total_post = pool_pct
    // price = pre_money / (E + P_new)
    // I = investment / price
    //
    // Solving:
    //   total_post = E + P_new + investment * (E + P_new) / pre_money
    //              = (E + P_new) * (1 + investment / pre_money)
    //              = (E + P_new) * post_money / pre_money
    //
    //   pool_target_shares = pool_pct * total_post
    //                      = pool_pct * (E + P_new) * post_money / pre_money
    //
    //   existing_pool + P_new = pool_pct * (E + P_new) * post_money / pre_money
    //
    // Let S = E + P_new  (pre-money fully-diluted shares after pool expansion)
    //   existing_pool + (S - E) = pool_pct * S * post_money / pre_money
    //
    // post_money = pre_money + investment
    // Let R = post_money / pre_money
    //   existing_pool + S - E = pool_pct * S * R
    //   S - pool_pct * S * R = E - existing_pool
    //   S * (1 - pool_pct * R) = E - existing_pool
    //   S = (E - existing_pool) / (1 - pool_pct * R)
    //
    // But this only works when existing_pool < target. If existing pool already
    // meets or exceeds the target, P_new = 0.

    let post_money = input.pre_money_valuation + input.investment_amount;
    let existing = dec_from_u64(input.existing_shares);
    let existing_pool = dec_from_u64(input.option_pool_shares_existing);

    let option_pool_new_shares: u64;
    let pre_money_fully_diluted: Decimal; // E + P_new

    if pool_pct > Decimal::ZERO {
        let r = post_money / input.pre_money_valuation;
        let denominator = Decimal::ONE - pool_pct * r;

        if denominator <= Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "option_pool_pct".into(),
                reason: format!(
                    "Option pool percentage {} is too large relative to the round economics \
                     (denominator {} <= 0). Reduce pool_pct or increase pre-money.",
                    pool_pct, denominator
                ),
            });
        }

        // S = (E - existing_pool) / denominator
        // But if the company has shares beyond the pool (the normal case), this works.
        // existing_pool is part of E, so (E - existing_pool) = non-pool shares.
        let non_pool_shares = existing - existing_pool;
        let s = non_pool_shares / denominator;

        // New pool shares = S - E (could be negative if pool already sufficient)
        let p_new_decimal = s - existing;
        if p_new_decimal <= Decimal::ZERO {
            option_pool_new_shares = 0;
            pre_money_fully_diluted = existing;
        } else {
            option_pool_new_shares = to_u64_truncated(p_new_decimal);
            pre_money_fully_diluted = existing + dec_from_u64(option_pool_new_shares);
        }
    } else {
        option_pool_new_shares = 0;
        pre_money_fully_diluted = existing;
    }

    // ── Price per share & new investor shares ────────────────────────
    if pre_money_fully_diluted == Decimal::ZERO {
        return Err(CorpFinanceError::DivisionByZero {
            context: "pre-money fully diluted shares is zero".into(),
        });
    }

    let price_per_share = input.pre_money_valuation / pre_money_fully_diluted;
    if price_per_share <= Decimal::ZERO {
        return Err(CorpFinanceError::FinancialImpossibility(
            "Computed price per share is non-positive".into(),
        ));
    }

    let new_shares_decimal = input.investment_amount / price_per_share;
    let new_shares_issued = to_u64_truncated(new_shares_decimal);

    // ── Post-round totals ────────────────────────────────────────────
    let total_shares_post_round =
        input.existing_shares + option_pool_new_shares + new_shares_issued;
    let total_shares_dec = dec_from_u64(total_shares_post_round);

    // Fully diluted = total shares (all options included since existing_shares already
    // includes option_pool_shares_existing, and we added option_pool_new_shares).
    let fully_diluted_shares = total_shares_post_round;

    // ── Ownership percentages ────────────────────────────────────────
    let investor_ownership_pct = dec_from_u64(new_shares_issued) / total_shares_dec;

    // Existing holders' combined ownership before this round was 100%
    // (ignoring that "existing" includes the old option pool for simplicity —
    //  we report the dilution of all pre-round shares as a group).
    let existing_post_pct = dec_from_u64(input.existing_shares) / total_shares_dec;
    let founder_dilution_pct = Decimal::ONE - existing_post_pct;

    // ── Cap table ────────────────────────────────────────────────────
    let hundred = dec!(100);
    let mut cap_table: Vec<CapTableEntry> = Vec::new();

    // Named shareholders
    for sh in &input.existing_shareholders {
        let sh_shares = dec_from_u64(sh.shares);
        let pct = sh_shares / total_shares_dec;
        cap_table.push(CapTableEntry {
            name: sh.name.clone(),
            shares: sh.shares,
            ownership_pct: (pct * hundred).round_dp(4),
            value_at_post_money: (pct * post_money).round_dp(2),
        });
    }

    // Option pool (existing + new)
    let total_pool_shares = input.option_pool_shares_existing + option_pool_new_shares;
    if total_pool_shares > 0 {
        let pool_dec = dec_from_u64(total_pool_shares);
        let pct = pool_dec / total_shares_dec;
        cap_table.push(CapTableEntry {
            name: "Option Pool".into(),
            shares: total_pool_shares,
            ownership_pct: (pct * hundred).round_dp(4),
            value_at_post_money: (pct * post_money).round_dp(2),
        });
    }

    // Unaccounted shares (if any)
    if unaccounted > 0 {
        let un_dec = dec_from_u64(unaccounted);
        let pct = un_dec / total_shares_dec;
        cap_table.push(CapTableEntry {
            name: "Other / Unaccounted".into(),
            shares: unaccounted,
            ownership_pct: (pct * hundred).round_dp(4),
            value_at_post_money: (pct * post_money).round_dp(2),
        });
    }

    // New investor
    {
        let inv_dec = dec_from_u64(new_shares_issued);
        let pct = inv_dec / total_shares_dec;
        cap_table.push(CapTableEntry {
            name: format!("{} Investor", input.round_name),
            shares: new_shares_issued,
            ownership_pct: (pct * hundred).round_dp(4),
            value_at_post_money: (pct * post_money).round_dp(2),
        });
    }

    // ── Assemble output ──────────────────────────────────────────────
    let output = FundingRoundOutput {
        post_money_valuation: post_money,
        price_per_share: price_per_share.round_dp(6),
        new_shares_issued,
        option_pool_new_shares,
        total_shares_post_round,
        investor_ownership_pct: (investor_ownership_pct * hundred).round_dp(4),
        founder_dilution_pct: (founder_dilution_pct * hundred).round_dp(4),
        cap_table,
        fully_diluted_shares,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Venture Capital Funding Round (Option Pool Shuffle)",
        &serde_json::json!({
            "pre_money_valuation": input.pre_money_valuation.to_string(),
            "investment_amount": input.investment_amount.to_string(),
            "existing_shares": input.existing_shares,
            "option_pool_pct": pool_pct.to_string(),
            "option_pool_shares_existing": input.option_pool_shares_existing,
            "round_name": input.round_name,
            "liquidation_preference": format!("{:?}", input.liquidation_preference),
        }),
        warnings,
        elapsed,
        output,
    ))
}

// ─── Function 2: analyze_dilution ────────────────────────────────────────────

/// Analyse dilution across multiple sequential funding rounds.
///
/// Starting from the initial share count and founder allocation, this function
/// models each round sequentially — applying the option pool shuffle, computing
/// new shares issued, and tracking each founder's ownership percentage and value
/// through the trajectory.
pub fn analyze_dilution(
    input: &DilutionInput,
) -> CorpFinanceResult<ComputationOutput<DilutionOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // ── Validation ───────────────────────────────────────────────────
    if input.initial_shares == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "initial_shares".into(),
            reason: "Initial shares must be greater than zero".into(),
        });
    }
    if input.founders.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "founders".into(),
            reason: "At least one founder is required".into(),
        });
    }
    if input.rounds.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "rounds".into(),
            reason: "At least one round is required".into(),
        });
    }

    let founder_shares_sum: u64 = input.founders.iter().map(|f| f.initial_shares).sum();
    if founder_shares_sum > input.initial_shares {
        return Err(CorpFinanceError::InvalidInput {
            field: "founders".into(),
            reason: format!(
                "Founder shares sum ({}) exceeds initial_shares ({})",
                founder_shares_sum, input.initial_shares
            ),
        });
    }

    // ── State tracking ───────────────────────────────────────────────
    // We track all share classes: founders (by name), option pool, and investors per round.
    // Each entry: (name, shares, class)
    struct HolderState {
        name: String,
        shares: u64,
        #[allow(dead_code)]
        class: String,
    }

    let mut holders: Vec<HolderState> = Vec::new();

    // Founders
    for f in &input.founders {
        holders.push(HolderState {
            name: f.name.clone(),
            shares: f.initial_shares,
            class: "Common".into(),
        });
    }

    // If founder shares don't account for all initial shares, track the remainder
    let unallocated = input.initial_shares - founder_shares_sum;
    if unallocated > 0 {
        warnings.push(format!(
            "{} initial shares not allocated to named founders",
            unallocated
        ));
        holders.push(HolderState {
            name: "Other Common".into(),
            shares: unallocated,
            class: "Common".into(),
        });
    }

    let mut total_shares: u64 = input.initial_shares;
    let mut option_pool_shares: u64 = 0;
    let mut round_results: Vec<RoundResult> = Vec::new();
    let mut trajectory: Vec<OwnershipPoint> = Vec::new();

    // ── Process each round ───────────────────────────────────────────
    for round in &input.rounds {
        if round.pre_money_valuation <= Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("{}.pre_money_valuation", round.name),
                reason: "Pre-money valuation must be positive".into(),
            });
        }
        if round.investment_amount <= Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("{}.investment_amount", round.name),
                reason: "Investment amount must be positive".into(),
            });
        }

        let pool_pct = round.option_pool_pct;
        let post_money = round.pre_money_valuation + round.investment_amount;

        // Option pool shuffle (same math as model_funding_round)
        let existing_dec = dec_from_u64(total_shares);
        let existing_pool_dec = dec_from_u64(option_pool_shares);

        let option_pool_increase: u64;
        let pre_money_fd: Decimal;

        if pool_pct > Decimal::ZERO {
            let r = post_money / round.pre_money_valuation;
            let denominator = Decimal::ONE - pool_pct * r;

            if denominator <= Decimal::ZERO {
                return Err(CorpFinanceError::InvalidInput {
                    field: format!("{}.option_pool_pct", round.name),
                    reason: format!(
                        "Option pool {} too large for round economics (denom={})",
                        pool_pct, denominator
                    ),
                });
            }

            let non_pool = existing_dec - existing_pool_dec;
            let s = non_pool / denominator;
            let p_new = s - existing_dec;

            if p_new <= Decimal::ZERO {
                option_pool_increase = 0;
                pre_money_fd = existing_dec;
            } else {
                option_pool_increase = to_u64_truncated(p_new);
                pre_money_fd = existing_dec + dec_from_u64(option_pool_increase);
            }
        } else {
            option_pool_increase = 0;
            pre_money_fd = existing_dec;
        }

        let price_per_share = round.pre_money_valuation / pre_money_fd;
        let new_shares = to_u64_truncated(round.investment_amount / price_per_share);

        // Update state
        total_shares += option_pool_increase + new_shares;
        option_pool_shares += option_pool_increase;

        // Add new investor
        holders.push(HolderState {
            name: format!("{} Investor", round.name),
            shares: new_shares,
            class: round.name.clone(),
        });

        round_results.push(RoundResult {
            round_name: round.name.clone(),
            pre_money: round.pre_money_valuation,
            post_money,
            price_per_share: price_per_share.round_dp(6),
            new_shares,
            option_pool_increase,
            total_shares,
        });

        // Track founder ownership after this round
        let total_dec = dec_from_u64(total_shares);
        let hundred = dec!(100);
        for f in &input.founders {
            let f_shares = holders
                .iter()
                .filter(|h| h.name == f.name)
                .map(|h| h.shares)
                .sum::<u64>();
            let pct = dec_from_u64(f_shares) / total_dec;
            trajectory.push(OwnershipPoint {
                round_name: round.name.clone(),
                ownership_pct: (pct * hundred).round_dp(4),
                value_at_post_money: (pct * post_money).round_dp(2),
            });
        }
    }

    // ── Final cap table ──────────────────────────────────────────────
    let total_dec = dec_from_u64(total_shares);
    let hundred = dec!(100);
    let last_post_money = round_results
        .last()
        .map(|r| r.post_money)
        .unwrap_or(Decimal::ZERO);

    let mut final_cap_table: Vec<CapTableEntry> = Vec::new();

    for h in &holders {
        let pct = dec_from_u64(h.shares) / total_dec;
        final_cap_table.push(CapTableEntry {
            name: h.name.clone(),
            shares: h.shares,
            ownership_pct: (pct * hundred).round_dp(4),
            value_at_post_money: (pct * last_post_money).round_dp(2),
        });
    }

    // Option pool entry
    if option_pool_shares > 0 {
        let pct = dec_from_u64(option_pool_shares) / total_dec;
        final_cap_table.push(CapTableEntry {
            name: "Option Pool".into(),
            shares: option_pool_shares,
            ownership_pct: (pct * hundred).round_dp(4),
            value_at_post_money: (pct * last_post_money).round_dp(2),
        });
    }

    let output = DilutionOutput {
        rounds: round_results,
        final_cap_table,
        founder_ownership_trajectory: trajectory,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Multi-Round Dilution Analysis (Option Pool Shuffle)",
        &serde_json::json!({
            "num_rounds": input.rounds.len(),
            "initial_shares": input.initial_shares,
            "num_founders": input.founders.len(),
            "round_names": input.rounds.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(),
        }),
        warnings,
        elapsed,
        output,
    ))
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// Helper: build a simple funding round input
    fn simple_round_input() -> FundingRoundInput {
        FundingRoundInput {
            pre_money_valuation: dec!(5_000_000),
            investment_amount: dec!(1_000_000),
            existing_shares: 10_000_000,
            existing_shareholders: vec![
                Shareholder {
                    name: "Founder A".into(),
                    shares: 6_000_000,
                    share_class: "Common".into(),
                },
                Shareholder {
                    name: "Founder B".into(),
                    shares: 4_000_000,
                    share_class: "Common".into(),
                },
            ],
            option_pool_pct: None,
            option_pool_shares_existing: 0,
            round_name: "Series A".into(),
            liquidation_preference: LiqPref::NonParticipating,
            participation_cap: None,
        }
    }

    // ── Test 1: Simple funding round — $5M pre, $1M invest => ~16.67% ────

    #[test]
    fn test_simple_round_ownership() {
        let input = simple_round_input();
        let result = model_funding_round(&input).unwrap();
        let out = &result.result;

        // post_money = 5M + 1M = 6M
        assert_eq!(out.post_money_valuation, dec!(6_000_000));

        // price = 5M / 10M shares = 0.50
        assert_eq!(out.price_per_share, dec!(0.500000));

        // new shares = 1M / 0.50 = 2M
        assert_eq!(out.new_shares_issued, 2_000_000);

        // investor ownership = 2M / 12M = 16.6667%
        let expected_pct = dec!(16.6667); // rounded to 4dp
        assert_eq!(out.investor_ownership_pct, expected_pct);
    }

    // ── Test 2: Post-money = pre-money + investment ──────────────────

    #[test]
    fn test_post_money_equals_pre_plus_investment() {
        let input = simple_round_input();
        let result = model_funding_round(&input).unwrap();
        let out = &result.result;

        assert_eq!(
            out.post_money_valuation,
            input.pre_money_valuation + input.investment_amount
        );
    }

    // ── Test 3: Option pool shuffle — 10% pool ──────────────────────

    #[test]
    fn test_option_pool_shuffle_10pct() {
        let mut input = simple_round_input();
        input.option_pool_pct = Some(dec!(0.10));
        input.option_pool_shares_existing = 0;

        let result = model_funding_round(&input).unwrap();
        let out = &result.result;

        // With pool shuffle, more shares in the denominator => lower price per share
        // Without pool: price = 5M / 10M = 0.50
        // With pool: price < 0.50 because new pool shares added pre-money
        assert!(
            out.price_per_share < dec!(0.500000),
            "Price with 10% pool should be less than $0.50, got {}",
            out.price_per_share
        );

        // Option pool should be > 0
        assert!(
            out.option_pool_new_shares > 0,
            "New pool shares should be created"
        );

        // Investor should still get investment/price shares
        let expected_inv_shares = to_u64_truncated(input.investment_amount / out.price_per_share);
        assert_eq!(out.new_shares_issued, expected_inv_shares);
    }

    // ── Test 4: Cap table percentages sum to ~100% ──────────────────

    #[test]
    fn test_cap_table_sums_to_100() {
        let mut input = simple_round_input();
        input.option_pool_pct = Some(dec!(0.10));

        let result = model_funding_round(&input).unwrap();
        let out = &result.result;

        let total_pct: Decimal = out.cap_table.iter().map(|e| e.ownership_pct).sum();
        // Should be very close to 100% (within rounding tolerance)
        let diff = (total_pct - dec!(100)).abs();
        assert!(
            diff < dec!(0.01),
            "Cap table pcts should sum to ~100%, got {} (diff={})",
            total_pct,
            diff
        );
    }

    // ── Test 5: Cap table shares sum to total ────────────────────────

    #[test]
    fn test_cap_table_shares_sum() {
        let mut input = simple_round_input();
        input.option_pool_pct = Some(dec!(0.10));

        let result = model_funding_round(&input).unwrap();
        let out = &result.result;

        let total_shares: u64 = out.cap_table.iter().map(|e| e.shares).sum();
        assert_eq!(
            total_shares, out.total_shares_post_round,
            "Cap table shares should sum to total_shares_post_round"
        );
    }

    // ── Test 6: Zero option pool — no extra shares ──────────────────

    #[test]
    fn test_zero_option_pool() {
        let input = simple_round_input();
        let result = model_funding_round(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.option_pool_new_shares, 0);
        assert_eq!(out.total_shares_post_round, 10_000_000 + 2_000_000);
    }

    // ── Test 7: Large option pool (20%) ─────────────────────────────

    #[test]
    fn test_large_option_pool_20pct() {
        let mut input = simple_round_input();
        input.option_pool_pct = Some(dec!(0.20));

        let result = model_funding_round(&input).unwrap();
        let out = &result.result;

        // Price should be even lower with a 20% pool
        assert!(
            out.price_per_share < dec!(0.500000),
            "Price with 20% pool should be less than $0.50"
        );

        // More pool shares than with 10%
        let mut input_10 = simple_round_input();
        input_10.option_pool_pct = Some(dec!(0.10));
        let result_10 = model_funding_round(&input_10).unwrap();

        assert!(
            out.option_pool_new_shares > result_10.result.option_pool_new_shares,
            "20% pool should create more shares than 10% pool"
        );
    }

    // ── Test 8: Founder dilution is positive ─────────────────────────

    #[test]
    fn test_founder_dilution_positive() {
        let input = simple_round_input();
        let result = model_funding_round(&input).unwrap();
        let out = &result.result;

        assert!(
            out.founder_dilution_pct > Decimal::ZERO,
            "Founder dilution should be positive, got {}",
            out.founder_dilution_pct
        );
    }

    // ── Test 9: Multiple existing shareholders ──────────────────────

    #[test]
    fn test_multiple_shareholders() {
        let mut input = simple_round_input();
        input.existing_shareholders = vec![
            Shareholder {
                name: "Founder A".into(),
                shares: 4_000_000,
                share_class: "Common".into(),
            },
            Shareholder {
                name: "Founder B".into(),
                shares: 3_000_000,
                share_class: "Common".into(),
            },
            Shareholder {
                name: "Angel 1".into(),
                shares: 2_000_000,
                share_class: "Seed".into(),
            },
            Shareholder {
                name: "Angel 2".into(),
                shares: 1_000_000,
                share_class: "Seed".into(),
            },
        ];

        let result = model_funding_round(&input).unwrap();
        let out = &result.result;

        // All 4 shareholders + investor = 5 cap table entries
        assert_eq!(out.cap_table.len(), 5);

        // Founder A should have 4M / 12M = 33.3333%
        let fa = out
            .cap_table
            .iter()
            .find(|e| e.name == "Founder A")
            .unwrap();
        assert_eq!(fa.shares, 4_000_000);
        assert_eq!(fa.ownership_pct, dec!(33.3333));
    }

    // ── Test 10: Validation — zero pre-money ────────────────────────

    #[test]
    fn test_invalid_zero_pre_money() {
        let mut input = simple_round_input();
        input.pre_money_valuation = Decimal::ZERO;

        let result = model_funding_round(&input);
        assert!(result.is_err());
    }

    // ── Test 11: Validation — zero investment ───────────────────────

    #[test]
    fn test_invalid_zero_investment() {
        let mut input = simple_round_input();
        input.investment_amount = Decimal::ZERO;

        let result = model_funding_round(&input);
        assert!(result.is_err());
    }

    // ── Test 12: Validation — zero existing shares ──────────────────

    #[test]
    fn test_invalid_zero_shares() {
        let mut input = simple_round_input();
        input.existing_shares = 0;
        input.existing_shareholders = vec![];

        let result = model_funding_round(&input);
        assert!(result.is_err());
    }

    // ── Test 13: Existing option pool partially allocated ───────────

    #[test]
    fn test_existing_option_pool_partially_allocated() {
        let mut input = simple_round_input();
        // 10M total: 6M founder A, 3M founder B, 1M existing option pool
        input.existing_shareholders = vec![
            Shareholder {
                name: "Founder A".into(),
                shares: 6_000_000,
                share_class: "Common".into(),
            },
            Shareholder {
                name: "Founder B".into(),
                shares: 3_000_000,
                share_class: "Common".into(),
            },
        ];
        input.option_pool_shares_existing = 1_000_000;
        input.option_pool_pct = Some(dec!(0.10));

        let result = model_funding_round(&input).unwrap();
        let out = &result.result;

        // New pool shares should be less than if there were no existing pool
        let mut input_no_existing = input.clone();
        input_no_existing.option_pool_shares_existing = 0;
        let result_no_existing = model_funding_round(&input_no_existing).unwrap();

        assert!(
            out.option_pool_new_shares < result_no_existing.result.option_pool_new_shares,
            "Existing pool should reduce new pool shares needed"
        );
    }

    // ── Test 14: Very small round ───────────────────────────────────

    #[test]
    fn test_very_small_round() {
        let mut input = simple_round_input();
        input.investment_amount = dec!(1_000); // Only $1K on a $5M company

        let result = model_funding_round(&input).unwrap();
        let out = &result.result;

        // Investor should own a tiny fraction
        assert!(
            out.investor_ownership_pct < dec!(0.1),
            "Tiny investment should give tiny ownership, got {}%",
            out.investor_ownership_pct
        );
    }

    // ── Test 15: Very large round ───────────────────────────────────

    #[test]
    fn test_very_large_round() {
        let mut input = simple_round_input();
        input.investment_amount = dec!(50_000_000); // $50M on a $5M company

        let result = model_funding_round(&input).unwrap();
        let out = &result.result;

        // Post-money = $55M
        assert_eq!(out.post_money_valuation, dec!(55_000_000));

        // Investor should own the majority
        assert!(
            out.investor_ownership_pct > dec!(50.0),
            "Large investment should give majority ownership, got {}%",
            out.investor_ownership_pct
        );
    }

    // ── Test 16: Multi-round dilution — Seed + Series A + Series B ──

    #[test]
    fn test_multi_round_dilution() {
        let input = DilutionInput {
            initial_shares: 10_000_000,
            founders: vec![
                FounderSpec {
                    name: "Alice".into(),
                    initial_shares: 6_000_000,
                },
                FounderSpec {
                    name: "Bob".into(),
                    initial_shares: 4_000_000,
                },
            ],
            rounds: vec![
                RoundSpec {
                    name: "Seed".into(),
                    pre_money_valuation: dec!(4_000_000),
                    investment_amount: dec!(1_000_000),
                    option_pool_pct: dec!(0.10),
                },
                RoundSpec {
                    name: "Series A".into(),
                    pre_money_valuation: dec!(20_000_000),
                    investment_amount: dec!(5_000_000),
                    option_pool_pct: dec!(0.10),
                },
                RoundSpec {
                    name: "Series B".into(),
                    pre_money_valuation: dec!(80_000_000),
                    investment_amount: dec!(20_000_000),
                    option_pool_pct: dec!(0.05),
                },
            ],
        };

        let result = analyze_dilution(&input).unwrap();
        let out = &result.result;

        // Should have 3 round results
        assert_eq!(out.rounds.len(), 3);

        // Each round's total shares should increase
        assert!(out.rounds[0].total_shares < out.rounds[1].total_shares);
        assert!(out.rounds[1].total_shares < out.rounds[2].total_shares);

        // Post-money = pre-money + investment for each round
        let investments = [dec!(1_000_000), dec!(5_000_000), dec!(20_000_000)];
        for (r, inv_amount) in out.rounds.iter().zip(investments.iter()) {
            assert_eq!(
                r.post_money,
                r.pre_money + inv_amount,
                "post_money should equal pre_money + investment for round {}",
                r.round_name
            );
        }
    }

    // ── Test 17: Founder ownership decreases each round ──────────────

    #[test]
    fn test_founder_ownership_decreases() {
        let input = DilutionInput {
            initial_shares: 10_000_000,
            founders: vec![FounderSpec {
                name: "Alice".into(),
                initial_shares: 10_000_000,
            }],
            rounds: vec![
                RoundSpec {
                    name: "Seed".into(),
                    pre_money_valuation: dec!(4_000_000),
                    investment_amount: dec!(1_000_000),
                    option_pool_pct: dec!(0.10),
                },
                RoundSpec {
                    name: "Series A".into(),
                    pre_money_valuation: dec!(20_000_000),
                    investment_amount: dec!(5_000_000),
                    option_pool_pct: dec!(0.10),
                },
            ],
        };

        let result = analyze_dilution(&input).unwrap();
        let out = &result.result;

        // Trajectory has one entry per round for Alice
        assert_eq!(out.founder_ownership_trajectory.len(), 2);

        // Ownership should decrease
        let after_seed = out.founder_ownership_trajectory[0].ownership_pct;
        let after_a = out.founder_ownership_trajectory[1].ownership_pct;

        assert!(
            after_seed > after_a,
            "Ownership after Seed ({}) should be > after Series A ({})",
            after_seed,
            after_a
        );

        // Both should be < 100%
        assert!(after_seed < dec!(100));
        assert!(after_a < dec!(100));
    }

    // ── Test 18: Founder value increases despite dilution ────────────

    #[test]
    fn test_founder_value_increases_with_up_rounds() {
        let input = DilutionInput {
            initial_shares: 10_000_000,
            founders: vec![FounderSpec {
                name: "Alice".into(),
                initial_shares: 10_000_000,
            }],
            rounds: vec![
                RoundSpec {
                    name: "Seed".into(),
                    pre_money_valuation: dec!(4_000_000),
                    investment_amount: dec!(1_000_000),
                    option_pool_pct: Decimal::ZERO,
                },
                RoundSpec {
                    name: "Series A".into(),
                    pre_money_valuation: dec!(20_000_000),
                    investment_amount: dec!(5_000_000),
                    option_pool_pct: Decimal::ZERO,
                },
            ],
        };

        let result = analyze_dilution(&input).unwrap();
        let out = &result.result;

        let after_seed_value = out.founder_ownership_trajectory[0].value_at_post_money;
        let after_a_value = out.founder_ownership_trajectory[1].value_at_post_money;

        assert!(
            after_a_value > after_seed_value,
            "Value should increase in up-rounds: after Seed={}, after A={}",
            after_seed_value,
            after_a_value
        );
    }

    // ── Test 19: Final cap table has all holders ─────────────────────

    #[test]
    fn test_final_cap_table_completeness() {
        let input = DilutionInput {
            initial_shares: 10_000_000,
            founders: vec![
                FounderSpec {
                    name: "Alice".into(),
                    initial_shares: 5_000_000,
                },
                FounderSpec {
                    name: "Bob".into(),
                    initial_shares: 5_000_000,
                },
            ],
            rounds: vec![
                RoundSpec {
                    name: "Seed".into(),
                    pre_money_valuation: dec!(5_000_000),
                    investment_amount: dec!(1_000_000),
                    option_pool_pct: dec!(0.10),
                },
                RoundSpec {
                    name: "Series A".into(),
                    pre_money_valuation: dec!(25_000_000),
                    investment_amount: dec!(5_000_000),
                    option_pool_pct: dec!(0.10),
                },
            ],
        };

        let result = analyze_dilution(&input).unwrap();
        let out = &result.result;

        // Should have: Alice, Bob, Seed Investor, Series A Investor, Option Pool
        let names: Vec<&str> = out
            .final_cap_table
            .iter()
            .map(|e| e.name.as_str())
            .collect();
        assert!(names.contains(&"Alice"));
        assert!(names.contains(&"Bob"));
        assert!(names.contains(&"Seed Investor"));
        assert!(names.contains(&"Series A Investor"));
        assert!(names.contains(&"Option Pool"));
    }

    // ── Test 20: Final cap table percentages sum to ~100% ────────────

    #[test]
    fn test_final_cap_table_pcts_sum() {
        let input = DilutionInput {
            initial_shares: 10_000_000,
            founders: vec![FounderSpec {
                name: "Alice".into(),
                initial_shares: 10_000_000,
            }],
            rounds: vec![
                RoundSpec {
                    name: "Seed".into(),
                    pre_money_valuation: dec!(5_000_000),
                    investment_amount: dec!(1_000_000),
                    option_pool_pct: dec!(0.10),
                },
                RoundSpec {
                    name: "Series A".into(),
                    pre_money_valuation: dec!(20_000_000),
                    investment_amount: dec!(5_000_000),
                    option_pool_pct: dec!(0.10),
                },
            ],
        };

        let result = analyze_dilution(&input).unwrap();
        let out = &result.result;

        let total_pct: Decimal = out.final_cap_table.iter().map(|e| e.ownership_pct).sum();
        let diff = (total_pct - dec!(100)).abs();
        assert!(
            diff < dec!(0.1),
            "Final cap table pcts should sum to ~100%, got {} (diff={})",
            total_pct,
            diff
        );
    }

    // ── Test 21: Dilution with no option pool across rounds ─────────

    #[test]
    fn test_dilution_no_option_pool() {
        let input = DilutionInput {
            initial_shares: 10_000_000,
            founders: vec![FounderSpec {
                name: "Alice".into(),
                initial_shares: 10_000_000,
            }],
            rounds: vec![RoundSpec {
                name: "Seed".into(),
                pre_money_valuation: dec!(4_000_000),
                investment_amount: dec!(1_000_000),
                option_pool_pct: Decimal::ZERO,
            }],
        };

        let result = analyze_dilution(&input).unwrap();
        let out = &result.result;

        // No option pool entry in cap table
        let pool_entry = out.final_cap_table.iter().find(|e| e.name == "Option Pool");
        assert!(pool_entry.is_none(), "Should have no option pool entry");

        // Post-money = 5M, Alice has 10M / (10M + 2.5M) = 80%
        // price = 4M / 10M = 0.40, new_shares = 1M / 0.40 = 2500000
        assert_eq!(out.rounds[0].new_shares, 2_500_000);
        assert_eq!(out.rounds[0].total_shares, 12_500_000);

        // Alice ownership = 10M / 12.5M = 80%
        let alice_pct = out.founder_ownership_trajectory[0].ownership_pct;
        assert_eq!(alice_pct, dec!(80.0000));
    }

    // ── Test 22: Validation — empty founders ────────────────────────

    #[test]
    fn test_dilution_invalid_no_founders() {
        let input = DilutionInput {
            initial_shares: 10_000_000,
            founders: vec![],
            rounds: vec![RoundSpec {
                name: "Seed".into(),
                pre_money_valuation: dec!(5_000_000),
                investment_amount: dec!(1_000_000),
                option_pool_pct: Decimal::ZERO,
            }],
        };

        let result = analyze_dilution(&input);
        assert!(result.is_err());
    }

    // ── Test 23: Validation — no rounds ─────────────────────────────

    #[test]
    fn test_dilution_invalid_no_rounds() {
        let input = DilutionInput {
            initial_shares: 10_000_000,
            founders: vec![FounderSpec {
                name: "Alice".into(),
                initial_shares: 10_000_000,
            }],
            rounds: vec![],
        };

        let result = analyze_dilution(&input);
        assert!(result.is_err());
    }

    // ── Test 24: Price per share increases in up-rounds ──────────────

    #[test]
    fn test_price_increases_in_up_rounds() {
        let input = DilutionInput {
            initial_shares: 10_000_000,
            founders: vec![FounderSpec {
                name: "Alice".into(),
                initial_shares: 10_000_000,
            }],
            rounds: vec![
                RoundSpec {
                    name: "Seed".into(),
                    pre_money_valuation: dec!(5_000_000),
                    investment_amount: dec!(1_000_000),
                    option_pool_pct: Decimal::ZERO,
                },
                RoundSpec {
                    name: "Series A".into(),
                    pre_money_valuation: dec!(30_000_000),
                    investment_amount: dec!(10_000_000),
                    option_pool_pct: Decimal::ZERO,
                },
            ],
        };

        let result = analyze_dilution(&input).unwrap();
        let out = &result.result;

        assert!(
            out.rounds[1].price_per_share > out.rounds[0].price_per_share,
            "Series A price ({}) should exceed Seed price ({})",
            out.rounds[1].price_per_share,
            out.rounds[0].price_per_share
        );
    }

    // ── Test 25: Liquidation preference stored correctly ─────────────

    #[test]
    fn test_liquidation_preference_types() {
        let mut input = simple_round_input();

        // Non-participating (default in simple_round_input)
        let result = model_funding_round(&input).unwrap();
        assert!(result.result.post_money_valuation > Decimal::ZERO);

        // Participating
        input.liquidation_preference = LiqPref::Participating;
        let result = model_funding_round(&input).unwrap();
        assert!(result.result.post_money_valuation > Decimal::ZERO);

        // Capped participating
        input.liquidation_preference = LiqPref::CappedParticipating;
        input.participation_cap = Some(dec!(3.0));
        let result = model_funding_round(&input).unwrap();
        assert!(result.result.post_money_valuation > Decimal::ZERO);
    }

    // ── Test 26: Pool shuffle matches known worked example ──────────

    #[test]
    fn test_option_pool_shuffle_worked_example() {
        // Classic example: $8M pre, $2M invest, 10M shares, 15% pool target, 0 existing pool
        // post_money = 10M
        // R = 10M / 8M = 1.25
        // denom = 1 - 0.15 * 1.25 = 1 - 0.1875 = 0.8125
        // S = 10M / 0.8125 = 12,307,692 (truncated)
        // P_new = 12,307,692 - 10,000,000 = 2,307,692
        // price = 8M / 12,307,692 = 0.650000...
        // new_shares = 2M / 0.650000... = 3,076,923 (truncated)
        let input = FundingRoundInput {
            pre_money_valuation: dec!(8_000_000),
            investment_amount: dec!(2_000_000),
            existing_shares: 10_000_000,
            existing_shareholders: vec![Shareholder {
                name: "Founders".into(),
                shares: 10_000_000,
                share_class: "Common".into(),
            }],
            option_pool_pct: Some(dec!(0.15)),
            option_pool_shares_existing: 0,
            round_name: "Series A".into(),
            liquidation_preference: LiqPref::NonParticipating,
            participation_cap: None,
        };

        let result = model_funding_round(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.post_money_valuation, dec!(10_000_000));
        assert_eq!(out.option_pool_new_shares, 2_307_692);

        // Price per share = 8M / (10M + 2,307,692) = 8M / 12,307,692
        let expected_price = dec!(8_000_000) / dec_from_u64(12_307_692);
        assert_eq!(out.price_per_share, expected_price.round_dp(6));

        // Investor gets 2M / price shares
        let expected_inv_shares = to_u64_truncated(dec!(2_000_000) / expected_price);
        assert_eq!(out.new_shares_issued, expected_inv_shares);
    }

    // ── Test 27: Fully diluted shares includes all options ──────────

    #[test]
    fn test_fully_diluted_includes_options() {
        let mut input = simple_round_input();
        input.option_pool_pct = Some(dec!(0.10));
        input.option_pool_shares_existing = 500_000;
        // Adjust shareholders: 6M + 3.5M + 500K existing pool = 10M
        input.existing_shareholders = vec![
            Shareholder {
                name: "Founder A".into(),
                shares: 6_000_000,
                share_class: "Common".into(),
            },
            Shareholder {
                name: "Founder B".into(),
                shares: 3_500_000,
                share_class: "Common".into(),
            },
        ];

        let result = model_funding_round(&input).unwrap();
        let out = &result.result;

        // fully_diluted = total_shares_post_round (all options are in the count)
        assert_eq!(out.fully_diluted_shares, out.total_shares_post_round);
        assert!(out.fully_diluted_shares > input.existing_shares);
    }

    // ── Test 28: Negative pre-money rejected ────────────────────────

    #[test]
    fn test_invalid_negative_pre_money() {
        let mut input = simple_round_input();
        input.pre_money_valuation = dec!(-1_000_000);

        let result = model_funding_round(&input);
        assert!(result.is_err());
    }

    // ── Test 29: Invalid option pool pct >= 1.0 ─────────────────────

    #[test]
    fn test_invalid_pool_pct_too_large() {
        let mut input = simple_round_input();
        input.option_pool_pct = Some(dec!(1.0));

        let result = model_funding_round(&input);
        assert!(result.is_err());
    }

    // ── Test 30: Dilution founder shares exceed initial ─────────────

    #[test]
    fn test_dilution_founder_shares_exceed_initial() {
        let input = DilutionInput {
            initial_shares: 10_000_000,
            founders: vec![
                FounderSpec {
                    name: "Alice".into(),
                    initial_shares: 7_000_000,
                },
                FounderSpec {
                    name: "Bob".into(),
                    initial_shares: 5_000_000,
                },
            ],
            rounds: vec![RoundSpec {
                name: "Seed".into(),
                pre_money_valuation: dec!(5_000_000),
                investment_amount: dec!(1_000_000),
                option_pool_pct: Decimal::ZERO,
            }],
        };

        let result = analyze_dilution(&input);
        assert!(result.is_err());
    }
}
