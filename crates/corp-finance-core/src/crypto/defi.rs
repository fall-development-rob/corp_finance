//! DeFi yield farming, impermanent loss, staking economics, and liquidity
//! pool analysis.
//!
//! Provides institutional-grade analytics for decentralized finance protocols:
//! - **Yield farming**: APR-to-APY conversion with configurable compounding,
//!   gas cost netting, and multi-token reward stacking.
//! - **Impermanent loss**: Constant-product AMM (Uniswap-style x*y=k) IL
//!   calculation with fee income offset comparison.
//! - **Staking economics**: Validator rewards net of commission and
//!   probabilistic slashing risk.
//! - **Liquidity pool**: Pool share, fee revenue projection, and
//!   IL-adjusted net returns.
//!
//! All calculations use `rust_decimal::Decimal` for precision. Compound
//! interest is computed via iterative multiplication (never `powd()`), and
//! square roots use Newton's method (20 iterations).

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

const NEWTON_SQRT_ITERATIONS: u32 = 20;
const DAYS_PER_YEAR: Decimal = dec!(365);

// ---------------------------------------------------------------------------
// Input types
// ---------------------------------------------------------------------------

/// The type of DeFi analysis to perform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DefiAnalysisType {
    YieldFarm,
    ImpermanentLoss,
    Staking,
    LiquidityPool,
}

/// Full input for a DeFi yield / staking / LP analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefiYieldInput {
    /// Protocol or pool name (informational).
    pub protocol_name: String,
    /// Which analysis to run.
    pub analysis_type: DefiAnalysisType,

    // -- Yield Farm fields --------------------------------------------------
    /// Base APR (annualized rate, e.g. 0.12 = 12%).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_apr: Option<Rate>,
    /// Additional token incentive APR on top of base_apr.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reward_apr: Option<Rate>,
    /// How many times per year rewards compound (365 = daily).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compounding_frequency: Option<u32>,
    /// Gas cost per compound transaction (in USD or base currency).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_cost_per_compound: Option<Money>,
    /// Principal deposited into the farm.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub principal: Option<Money>,
    /// How long the position is held (days).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub holding_period_days: Option<u32>,

    // -- Impermanent Loss fields --------------------------------------------
    /// Initial price of token A (in quote currency).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_price_a: Option<Money>,
    /// Initial price of token B (in quote currency).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_price_b: Option<Money>,
    /// Final price of token A.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_price_a: Option<Money>,
    /// Final price of token B.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_price_b: Option<Money>,
    /// Total value deposited into the pool at inception.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_deposit_value: Option<Money>,
    /// Trading fee APR earned by the pool.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pool_fee_apr: Option<Rate>,

    // -- Staking fields -----------------------------------------------------
    /// Amount staked (in native token value).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub staked_amount: Option<Money>,
    /// Annual reward rate for the validator/network.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annual_reward_rate: Option<Rate>,
    /// Validator commission rate (e.g. 0.10 = 10% of rewards taken).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validator_commission: Option<Rate>,
    /// Annual probability of a slashing event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slashing_probability: Option<Rate>,
    /// Fraction of stake lost if slashed (e.g. 0.05 = 5%).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slashing_penalty: Option<Rate>,
    /// Unbonding period in days.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unbonding_period_days: Option<u32>,
    /// Whether staking rewards are auto-compounded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compounding: Option<bool>,

    // -- Liquidity Pool fields ----------------------------------------------
    /// Total value locked in the pool.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pool_tvl: Option<Money>,
    /// User's deposit into the pool.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_deposit: Option<Money>,
    /// Average daily trading volume through the pool.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub daily_volume: Option<Money>,
    /// Pool swap fee rate (e.g. 0.003 = 0.3% Uniswap-style).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pool_fee_rate: Option<Rate>,
    /// Weight of token A in the pool (0.5 = 50/50).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_a_weight: Option<Rate>,
    /// Expected price change percentage for IL estimation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_change_pct: Option<Rate>,
}

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// Output from a DeFi analysis computation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefiYieldOutput {
    /// Which analysis was performed.
    pub analysis_type: String,
    /// Effective APY after compounding.
    pub effective_apy: Rate,
    /// APY after gas costs are deducted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub net_apy: Option<Rate>,
    /// Total dollar return over the holding period.
    pub total_return: Money,
    /// Impermanent loss as a percentage of hold value (negative = loss).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub impermanent_loss_pct: Option<Rate>,
    /// Impermanent loss in dollar terms.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub impermanent_loss_amount: Option<Money>,
    /// Whether pool fees exceed IL or vice-versa.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub il_vs_fees: Option<String>,
    /// Staking yield after validator commission.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub staking_effective_yield: Option<Rate>,
    /// Expected annual staking reward in currency terms.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub staking_expected_annual_reward: Option<Money>,
    /// User share of the pool (user_deposit / pool_tvl).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pool_share: Option<Rate>,
    /// Projected annual fee income from the pool.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pool_fee_income: Option<Money>,
    /// Yield adjusted for expected slashing cost.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk_adjusted_apy: Option<Rate>,
    /// Warnings and informational notes.
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Analyse a DeFi position: yield farm, impermanent loss, staking economics,
/// or liquidity pool metrics.
pub fn analyze_defi(
    input: &DefiYieldInput,
) -> CorpFinanceResult<ComputationOutput<DefiYieldOutput>> {
    let start = Instant::now();

    let output = match &input.analysis_type {
        DefiAnalysisType::YieldFarm => analyze_yield_farm(input)?,
        DefiAnalysisType::ImpermanentLoss => analyze_impermanent_loss(input)?,
        DefiAnalysisType::Staking => analyze_staking(input)?,
        DefiAnalysisType::LiquidityPool => analyze_liquidity_pool(input)?,
    };

    let elapsed = start.elapsed().as_micros() as u64;

    Ok(with_metadata(
        &format!(
            "DeFi Analysis — {:?} for {}",
            input.analysis_type, input.protocol_name
        ),
        &serde_json::json!({
            "protocol": input.protocol_name,
            "analysis_type": format!("{:?}", input.analysis_type),
            "math": "rust_decimal (iterative compound, Newton sqrt)",
        }),
        output.warnings.clone(),
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Yield Farm
// ---------------------------------------------------------------------------

fn analyze_yield_farm(input: &DefiYieldInput) -> CorpFinanceResult<DefiYieldOutput> {
    let base_apr = require_field(input.base_apr, "base_apr")?;
    let compounding_frequency =
        require_field(input.compounding_frequency, "compounding_frequency")?;
    let principal = require_field(input.principal, "principal")?;
    let holding_period_days = require_field(input.holding_period_days, "holding_period_days")?;

    let mut warnings: Vec<String> = Vec::new();

    // Validate
    if compounding_frequency == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "compounding_frequency".into(),
            reason: "Compounding frequency must be > 0".into(),
        });
    }
    if principal <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "principal".into(),
            reason: "Principal must be positive".into(),
        });
    }
    if base_apr < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "base_apr".into(),
            reason: "Base APR cannot be negative".into(),
        });
    }

    // Total APR = base + reward (if any)
    let total_apr = base_apr + input.reward_apr.unwrap_or(Decimal::ZERO);

    if total_apr > dec!(10) {
        warnings.push("APR exceeds 1000% — verify this is correct".into());
    }

    // APY = (1 + APR/n)^n - 1  via iterative multiplication
    let n = compounding_frequency;
    let period_rate = total_apr / Decimal::from(n);
    let effective_apy = compound_iterative(period_rate, n) - Decimal::ONE;

    // Net APY: subtract annualized gas costs
    let annualized_gas = match input.gas_cost_per_compound {
        Some(gas) => gas * Decimal::from(n),
        None => Decimal::ZERO,
    };
    let gas_drag = if principal > Decimal::ZERO {
        annualized_gas / principal
    } else {
        Decimal::ZERO
    };
    let net_apy = effective_apy - gas_drag;

    if net_apy < Decimal::ZERO {
        warnings.push("Gas costs exceed yield — position is net negative".into());
    }

    // Total return over holding period
    let holding_fraction = Decimal::from(holding_period_days) / DAYS_PER_YEAR;
    let total_return = principal * net_apy * holding_fraction;

    Ok(DefiYieldOutput {
        analysis_type: "YieldFarm".to_string(),
        effective_apy,
        net_apy: Some(net_apy),
        total_return,
        impermanent_loss_pct: None,
        impermanent_loss_amount: None,
        il_vs_fees: None,
        staking_effective_yield: None,
        staking_expected_annual_reward: None,
        pool_share: None,
        pool_fee_income: None,
        risk_adjusted_apy: None,
        warnings,
    })
}

// ---------------------------------------------------------------------------
// Impermanent Loss
// ---------------------------------------------------------------------------

fn analyze_impermanent_loss(input: &DefiYieldInput) -> CorpFinanceResult<DefiYieldOutput> {
    let initial_price_a = require_field(input.initial_price_a, "initial_price_a")?;
    let initial_price_b = require_field(input.initial_price_b, "initial_price_b")?;
    let final_price_a = require_field(input.final_price_a, "final_price_a")?;
    let final_price_b = require_field(input.final_price_b, "final_price_b")?;
    let initial_deposit_value =
        require_field(input.initial_deposit_value, "initial_deposit_value")?;

    let mut warnings: Vec<String> = Vec::new();

    // Validate prices are positive
    if initial_price_a <= Decimal::ZERO || initial_price_b <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "initial_price".into(),
            reason: "Initial prices must be positive".into(),
        });
    }
    if final_price_a <= Decimal::ZERO || final_price_b <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "final_price".into(),
            reason: "Final prices must be positive".into(),
        });
    }
    if initial_deposit_value <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "initial_deposit_value".into(),
            reason: "Initial deposit value must be positive".into(),
        });
    }

    // Price ratio: how much token A appreciated relative to token B.
    // For a 50/50 constant-product AMM, we track the relative price change.
    // price_ratio = (final_a / initial_a) / (final_b / initial_b)
    let ratio_a = final_price_a / initial_price_a;
    let ratio_b = final_price_b / initial_price_b;
    let price_ratio = ratio_a / ratio_b;

    // IL = 2 * sqrt(price_ratio) / (1 + price_ratio) - 1
    let sqrt_ratio = newton_sqrt(price_ratio)?;
    let il_pct = dec!(2) * sqrt_ratio / (Decimal::ONE + price_ratio) - Decimal::ONE;

    // IL is always <= 0 (it is a loss relative to holding)
    let il_amount = initial_deposit_value * il_pct.abs();

    // Value if held (no LP):
    // Half the deposit was token A, half was token B.
    // hold_value = initial_deposit_value/2 * ratio_a + initial_deposit_value/2 * ratio_b
    let hold_value =
        initial_deposit_value / dec!(2) * ratio_a + initial_deposit_value / dec!(2) * ratio_b;

    // LP value = hold_value * (2 * sqrt(price_ratio) / (1 + price_ratio))
    let lp_value = hold_value + hold_value * il_pct;
    let _ = lp_value; // informational; IL amount is the key output

    // Fee income over holding period (if provided)
    let holding_days = input.holding_period_days.unwrap_or(365);
    let holding_fraction = Decimal::from(holding_days) / DAYS_PER_YEAR;
    let fee_income = match input.pool_fee_apr {
        Some(fee_apr) => initial_deposit_value * fee_apr * holding_fraction,
        None => Decimal::ZERO,
    };

    let il_vs_fees = if fee_income > Decimal::ZERO {
        if fee_income >= il_amount {
            Some("Fees exceed IL".to_string())
        } else {
            Some("IL exceeds fees".to_string())
        }
    } else {
        None
    };

    // Effective APY: net of IL + fees, annualized
    let net_gain = fee_income - il_amount;
    let effective_apy = if initial_deposit_value > Decimal::ZERO && holding_fraction > Decimal::ZERO
    {
        net_gain / initial_deposit_value / holding_fraction
    } else {
        Decimal::ZERO
    };

    if il_pct.abs() > dec!(0.10) {
        warnings.push(format!(
            "Impermanent loss is significant: {:.2}%",
            il_pct * dec!(100)
        ));
    }

    Ok(DefiYieldOutput {
        analysis_type: "ImpermanentLoss".to_string(),
        effective_apy,
        net_apy: None,
        total_return: net_gain,
        impermanent_loss_pct: Some(il_pct),
        impermanent_loss_amount: Some(il_amount),
        il_vs_fees,
        staking_effective_yield: None,
        staking_expected_annual_reward: None,
        pool_share: None,
        pool_fee_income: Some(fee_income),
        risk_adjusted_apy: None,
        warnings,
    })
}

// ---------------------------------------------------------------------------
// Staking Economics
// ---------------------------------------------------------------------------

fn analyze_staking(input: &DefiYieldInput) -> CorpFinanceResult<DefiYieldOutput> {
    let staked_amount = require_field(input.staked_amount, "staked_amount")?;
    let annual_reward_rate = require_field(input.annual_reward_rate, "annual_reward_rate")?;
    let validator_commission = require_field(input.validator_commission, "validator_commission")?;
    let slashing_probability = require_field(input.slashing_probability, "slashing_probability")?;
    let slashing_penalty = require_field(input.slashing_penalty, "slashing_penalty")?;
    let _unbonding_period_days =
        require_field(input.unbonding_period_days, "unbonding_period_days")?;
    let compounding = input.compounding.unwrap_or(false);

    let mut warnings: Vec<String> = Vec::new();

    // Validate
    if staked_amount <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "staked_amount".into(),
            reason: "Staked amount must be positive".into(),
        });
    }
    if annual_reward_rate < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "annual_reward_rate".into(),
            reason: "Annual reward rate cannot be negative".into(),
        });
    }
    if validator_commission < Decimal::ZERO || validator_commission > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "validator_commission".into(),
            reason: "Validator commission must be between 0 and 1".into(),
        });
    }
    if slashing_probability < Decimal::ZERO || slashing_probability > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "slashing_probability".into(),
            reason: "Slashing probability must be between 0 and 1".into(),
        });
    }
    if slashing_penalty < Decimal::ZERO || slashing_penalty > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "slashing_penalty".into(),
            reason: "Slashing penalty must be between 0 and 1".into(),
        });
    }

    // Net yield after commission
    let net_yield = annual_reward_rate * (Decimal::ONE - validator_commission);

    // Effective APY if compounding daily
    let effective_apy = if compounding {
        // (1 + net_yield/365)^365 - 1 via iterative multiplication
        let daily_rate = net_yield / DAYS_PER_YEAR;
        compound_iterative(daily_rate, 365) - Decimal::ONE
    } else {
        net_yield
    };

    // Expected slashing cost = probability * penalty (as a rate on the stake)
    let expected_slashing_cost = slashing_probability * slashing_penalty;

    // Risk-adjusted yield
    let risk_adjusted_apy = effective_apy - expected_slashing_cost;

    // Expected annual reward (before slashing risk)
    let expected_annual_reward = staked_amount * effective_apy;

    // Total return = risk-adjusted over 1 year
    let total_return = staked_amount * risk_adjusted_apy;

    if risk_adjusted_apy < Decimal::ZERO {
        warnings.push("Risk-adjusted yield is negative — slashing risk exceeds rewards".into());
    }

    if slashing_probability > dec!(0.05) {
        warnings.push("Slashing probability exceeds 5% — high-risk validator".into());
    }

    if let Some(unbonding) = input.unbonding_period_days {
        if unbonding > 28 {
            warnings.push(format!(
                "Unbonding period is {} days — capital locked for extended period",
                unbonding
            ));
        }
    }

    Ok(DefiYieldOutput {
        analysis_type: "Staking".to_string(),
        effective_apy,
        net_apy: None,
        total_return,
        impermanent_loss_pct: None,
        impermanent_loss_amount: None,
        il_vs_fees: None,
        staking_effective_yield: Some(net_yield),
        staking_expected_annual_reward: Some(expected_annual_reward),
        pool_share: None,
        pool_fee_income: None,
        risk_adjusted_apy: Some(risk_adjusted_apy),
        warnings,
    })
}

// ---------------------------------------------------------------------------
// Liquidity Pool
// ---------------------------------------------------------------------------

fn analyze_liquidity_pool(input: &DefiYieldInput) -> CorpFinanceResult<DefiYieldOutput> {
    let pool_tvl = require_field(input.pool_tvl, "pool_tvl")?;
    let user_deposit = require_field(input.user_deposit, "user_deposit")?;
    let daily_volume = require_field(input.daily_volume, "daily_volume")?;
    let pool_fee_rate = require_field(input.pool_fee_rate, "pool_fee_rate")?;
    let _token_a_weight = require_field(input.token_a_weight, "token_a_weight")?;
    let price_change_pct = require_field(input.price_change_pct, "price_change_pct")?;

    let mut warnings: Vec<String> = Vec::new();

    // Validate
    if pool_tvl <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "pool_tvl".into(),
            reason: "Pool TVL must be positive".into(),
        });
    }
    if user_deposit <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "user_deposit".into(),
            reason: "User deposit must be positive".into(),
        });
    }
    if user_deposit > pool_tvl {
        warnings.push("User deposit exceeds pool TVL — check inputs".into());
    }
    if pool_fee_rate < Decimal::ZERO || pool_fee_rate > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "pool_fee_rate".into(),
            reason: "Pool fee rate must be between 0 and 1".into(),
        });
    }

    // Pool share
    let pool_share = user_deposit / pool_tvl;

    // Daily fee income: daily_volume * pool_fee_rate * pool_share
    let daily_fee_income = daily_volume * pool_fee_rate * pool_share;
    let annual_fee_income = daily_fee_income * DAYS_PER_YEAR;

    // Fee APY
    let fee_apy = if user_deposit > Decimal::ZERO {
        annual_fee_income / user_deposit
    } else {
        Decimal::ZERO
    };

    // IL from expected price change
    // price_ratio = (1 + price_change_pct) (token A changes, token B stable)
    let price_ratio = Decimal::ONE + price_change_pct;

    let (il_pct, il_amount) = if price_ratio > Decimal::ZERO {
        let sqrt_ratio = newton_sqrt(price_ratio)?;
        let il = dec!(2) * sqrt_ratio / (Decimal::ONE + price_ratio) - Decimal::ONE;
        (il, user_deposit * il.abs())
    } else {
        warnings.push("Price ratio non-positive after change — IL undefined".into());
        (Decimal::ZERO, Decimal::ZERO)
    };

    // IL vs fees comparison
    let il_vs_fees = if annual_fee_income >= il_amount {
        Some("Fees exceed IL".to_string())
    } else {
        Some("IL exceeds fees".to_string())
    };

    // Net effective APY = fee APY - annualized IL percentage
    let effective_apy = fee_apy + il_pct; // il_pct is negative or zero

    // Total return over 1 year
    let total_return = annual_fee_income - il_amount;

    if effective_apy < Decimal::ZERO {
        warnings.push("Net APY is negative — impermanent loss exceeds fee income".into());
    }

    Ok(DefiYieldOutput {
        analysis_type: "LiquidityPool".to_string(),
        effective_apy,
        net_apy: None,
        total_return,
        impermanent_loss_pct: Some(il_pct),
        impermanent_loss_amount: Some(il_amount),
        il_vs_fees,
        staking_effective_yield: None,
        staking_expected_annual_reward: None,
        pool_share: Some(pool_share),
        pool_fee_income: Some(annual_fee_income),
        risk_adjusted_apy: None,
        warnings,
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Compute (1 + r)^n via iterative multiplication (avoids Decimal::powd drift).
fn compound_iterative(rate: Decimal, n: u32) -> Decimal {
    let mut result = Decimal::ONE;
    let factor = Decimal::ONE + rate;
    for _ in 0..n {
        result *= factor;
    }
    result
}

/// Newton's method square root for Decimal values. 20 iterations.
fn newton_sqrt(value: Decimal) -> CorpFinanceResult<Decimal> {
    if value < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "sqrt_input".into(),
            reason: "Cannot take square root of a negative number".into(),
        });
    }
    if value.is_zero() {
        return Ok(Decimal::ZERO);
    }
    if value == Decimal::ONE {
        return Ok(Decimal::ONE);
    }

    // Initial guess: value / 2 for values > 1, otherwise value itself
    let mut guess = if value > Decimal::ONE {
        value / dec!(2)
    } else {
        value
    };

    for _ in 0..NEWTON_SQRT_ITERATIONS {
        if guess.is_zero() {
            break;
        }
        guess = (guess + value / guess) / dec!(2);
    }

    Ok(guess)
}

/// Extract a required field from an Option, returning an InvalidInput error
/// if the field is None.
fn require_field<T>(opt: Option<T>, field_name: &str) -> CorpFinanceResult<T> {
    opt.ok_or_else(|| CorpFinanceError::InvalidInput {
        field: field_name.to_string(),
        reason: format!("{} is required for this analysis type", field_name),
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    // -----------------------------------------------------------------------
    // Helpers for building test inputs
    // -----------------------------------------------------------------------

    fn yield_farm_input() -> DefiYieldInput {
        DefiYieldInput {
            protocol_name: "TestFarm".to_string(),
            analysis_type: DefiAnalysisType::YieldFarm,
            base_apr: Some(dec!(0.12)),
            reward_apr: None,
            compounding_frequency: Some(365),
            gas_cost_per_compound: None,
            principal: Some(dec!(10000)),
            holding_period_days: Some(365),
            initial_price_a: None,
            initial_price_b: None,
            final_price_a: None,
            final_price_b: None,
            initial_deposit_value: None,
            pool_fee_apr: None,
            staked_amount: None,
            annual_reward_rate: None,
            validator_commission: None,
            slashing_probability: None,
            slashing_penalty: None,
            unbonding_period_days: None,
            compounding: None,
            pool_tvl: None,
            user_deposit: None,
            daily_volume: None,
            pool_fee_rate: None,
            token_a_weight: None,
            price_change_pct: None,
        }
    }

    fn il_input() -> DefiYieldInput {
        DefiYieldInput {
            protocol_name: "UniswapV2".to_string(),
            analysis_type: DefiAnalysisType::ImpermanentLoss,
            base_apr: None,
            reward_apr: None,
            compounding_frequency: None,
            gas_cost_per_compound: None,
            principal: None,
            holding_period_days: Some(365),
            initial_price_a: Some(dec!(1000)),
            initial_price_b: Some(dec!(1)),
            final_price_a: Some(dec!(2000)),
            final_price_b: Some(dec!(1)),
            initial_deposit_value: Some(dec!(10000)),
            pool_fee_apr: None,
            staked_amount: None,
            annual_reward_rate: None,
            validator_commission: None,
            slashing_probability: None,
            slashing_penalty: None,
            unbonding_period_days: None,
            compounding: None,
            pool_tvl: None,
            user_deposit: None,
            daily_volume: None,
            pool_fee_rate: None,
            token_a_weight: None,
            price_change_pct: None,
        }
    }

    fn staking_input() -> DefiYieldInput {
        DefiYieldInput {
            protocol_name: "EthStaking".to_string(),
            analysis_type: DefiAnalysisType::Staking,
            base_apr: None,
            reward_apr: None,
            compounding_frequency: None,
            gas_cost_per_compound: None,
            principal: None,
            holding_period_days: None,
            initial_price_a: None,
            initial_price_b: None,
            final_price_a: None,
            final_price_b: None,
            initial_deposit_value: None,
            pool_fee_apr: None,
            staked_amount: Some(dec!(32)),
            annual_reward_rate: Some(dec!(0.05)),
            validator_commission: Some(dec!(0.10)),
            slashing_probability: Some(dec!(0.01)),
            slashing_penalty: Some(dec!(0.05)),
            unbonding_period_days: Some(14),
            compounding: Some(false),
            pool_tvl: None,
            user_deposit: None,
            daily_volume: None,
            pool_fee_rate: None,
            token_a_weight: None,
            price_change_pct: None,
        }
    }

    fn lp_input() -> DefiYieldInput {
        DefiYieldInput {
            protocol_name: "UniswapV3".to_string(),
            analysis_type: DefiAnalysisType::LiquidityPool,
            base_apr: None,
            reward_apr: None,
            compounding_frequency: None,
            gas_cost_per_compound: None,
            principal: None,
            holding_period_days: None,
            initial_price_a: None,
            initial_price_b: None,
            final_price_a: None,
            final_price_b: None,
            initial_deposit_value: None,
            pool_fee_apr: None,
            staked_amount: None,
            annual_reward_rate: None,
            validator_commission: None,
            slashing_probability: None,
            slashing_penalty: None,
            unbonding_period_days: None,
            compounding: None,
            pool_tvl: Some(dec!(10_000_000)),
            user_deposit: Some(dec!(100_000)),
            daily_volume: Some(dec!(5_000_000)),
            pool_fee_rate: Some(dec!(0.003)),
            token_a_weight: Some(dec!(0.5)),
            price_change_pct: Some(dec!(0.50)),
        }
    }

    // -----------------------------------------------------------------------
    // 1. APR to APY conversion (daily compounding, known result)
    // -----------------------------------------------------------------------
    #[test]
    fn test_apr_to_apy_daily_compounding() {
        let input = yield_farm_input();
        let result = analyze_defi(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.analysis_type, "YieldFarm");

        // APY = (1 + 0.12/365)^365 - 1 ≈ 0.12747 (12.75%)
        // Allow tolerance for Decimal iterative precision
        assert!(
            out.effective_apy > dec!(0.1274) && out.effective_apy < dec!(0.1276),
            "Daily compound APY of 12% APR should be ~12.75%, got {}",
            out.effective_apy
        );
    }

    // -----------------------------------------------------------------------
    // 2. Net APY after gas costs
    // -----------------------------------------------------------------------
    #[test]
    fn test_net_apy_after_gas_costs() {
        let mut input = yield_farm_input();
        input.gas_cost_per_compound = Some(dec!(0.50)); // $0.50 per compound
        input.principal = Some(dec!(1000)); // small principal to make gas significant

        let result = analyze_defi(&input).unwrap();
        let out = &result.result;

        // Annualized gas = 0.50 * 365 = 182.50
        // Gas drag = 182.50 / 1000 = 0.1825 (18.25%)
        // APY ≈ 12.75%, so net ≈ 12.75% - 18.25% = -5.50%
        let net = out.net_apy.unwrap();
        assert!(
            net < Decimal::ZERO,
            "With high gas on small principal, net APY should be negative, got {}",
            net
        );

        // Check warning about net negative
        assert!(
            out.warnings.iter().any(|w| w.contains("net negative")),
            "Should warn about negative net yield"
        );
    }

    // -----------------------------------------------------------------------
    // 3. Impermanent loss: 2x price increase → IL ≈ -5.72%
    // -----------------------------------------------------------------------
    #[test]
    fn test_impermanent_loss_2x_price_increase() {
        let input = il_input(); // token A: 1000 -> 2000, token B: 1 -> 1
        let result = analyze_defi(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.analysis_type, "ImpermanentLoss");

        // price_ratio = 2.0
        // IL = 2*sqrt(2)/(1+2) - 1 = 2*1.41421/3 - 1 = 0.94281 - 1 = -0.05719
        let il = out.impermanent_loss_pct.unwrap();
        assert!(
            il < dec!(-0.056) && il > dec!(-0.058),
            "IL for 2x price change should be ~-5.72%, got {}",
            il
        );

        // IL amount = 10000 * 0.0572 ≈ 572
        let il_amt = out.impermanent_loss_amount.unwrap();
        assert!(
            il_amt > dec!(560) && il_amt < dec!(580),
            "IL amount should be ~572, got {}",
            il_amt
        );
    }

    // -----------------------------------------------------------------------
    // 4. Impermanent loss: symmetric price change (both tokens move equally)
    // -----------------------------------------------------------------------
    #[test]
    fn test_impermanent_loss_symmetric_change() {
        let mut input = il_input();
        // Both tokens double: ratio stays 1:1, so IL should be zero
        input.initial_price_a = Some(dec!(100));
        input.initial_price_b = Some(dec!(100));
        input.final_price_a = Some(dec!(200));
        input.final_price_b = Some(dec!(200));

        let result = analyze_defi(&input).unwrap();
        let out = &result.result;

        let il = out.impermanent_loss_pct.unwrap();
        // When both tokens change by same factor, price_ratio = 1, IL = 0
        assert!(
            il.abs() < dec!(0.0001),
            "Symmetric price change should yield ~0 IL, got {}",
            il
        );
    }

    // -----------------------------------------------------------------------
    // 5. IL vs fees comparison
    // -----------------------------------------------------------------------
    #[test]
    fn test_il_vs_fees_comparison() {
        // Case 1: Fees exceed IL
        let mut input = il_input();
        input.pool_fee_apr = Some(dec!(0.20)); // 20% fee APR — very high
        input.holding_period_days = Some(365);

        let result = analyze_defi(&input).unwrap();
        let out = &result.result;

        // IL ≈ 572 on 10000 deposit
        // Fee income = 10000 * 0.20 * 1 = 2000
        // 2000 > 572, so fees exceed IL
        assert_eq!(
            out.il_vs_fees.as_deref(),
            Some("Fees exceed IL"),
            "With 20% fee APR, fees should exceed IL"
        );

        // Case 2: IL exceeds fees
        let mut input2 = il_input();
        input2.pool_fee_apr = Some(dec!(0.01)); // 1% fee APR — low
        input2.holding_period_days = Some(365);

        let result2 = analyze_defi(&input2).unwrap();
        let out2 = &result2.result;

        // Fee income = 10000 * 0.01 * 1 = 100
        // 100 < 572, so IL exceeds fees
        assert_eq!(
            out2.il_vs_fees.as_deref(),
            Some("IL exceeds fees"),
            "With 1% fee APR, IL should exceed fees"
        );
    }

    // -----------------------------------------------------------------------
    // 6. Staking yield after commission
    // -----------------------------------------------------------------------
    #[test]
    fn test_staking_yield_after_commission() {
        let input = staking_input();
        let result = analyze_defi(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.analysis_type, "Staking");

        // Net yield = 0.05 * (1 - 0.10) = 0.045
        let staking_yield = out.staking_effective_yield.unwrap();
        assert_eq!(
            staking_yield,
            dec!(0.045),
            "Staking yield after 10% commission on 5% rate should be 4.5%, got {}",
            staking_yield
        );

        // Without compounding, effective_apy = net_yield = 0.045
        assert_eq!(
            out.effective_apy,
            dec!(0.045),
            "Without compounding, APY = net yield"
        );
    }

    // -----------------------------------------------------------------------
    // 7. Staking risk-adjusted yield with slashing
    // -----------------------------------------------------------------------
    #[test]
    fn test_staking_risk_adjusted_yield() {
        let input = staking_input();
        let result = analyze_defi(&input).unwrap();
        let out = &result.result;

        // Net yield = 0.045
        // Expected slashing = 0.01 * 0.05 = 0.0005
        // Risk-adjusted = 0.045 - 0.0005 = 0.0445
        let risk_adj = out.risk_adjusted_apy.unwrap();
        assert_eq!(
            risk_adj,
            dec!(0.0445),
            "Risk-adjusted yield should be 4.45%, got {}",
            risk_adj
        );

        // Total return = 32 * 0.0445 = 1.424
        let expected_return = dec!(32) * dec!(0.0445);
        assert_eq!(
            out.total_return, expected_return,
            "Total return should be {}, got {}",
            expected_return, out.total_return
        );
    }

    // -----------------------------------------------------------------------
    // 8. Liquidity pool share and fee income calculation
    // -----------------------------------------------------------------------
    #[test]
    fn test_liquidity_pool_share_and_fees() {
        let input = lp_input();
        let result = analyze_defi(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.analysis_type, "LiquidityPool");

        // Pool share = 100_000 / 10_000_000 = 0.01
        let share = out.pool_share.unwrap();
        assert_eq!(share, dec!(0.01), "Pool share should be 1%, got {}", share);

        // Daily fee income = 5_000_000 * 0.003 * 0.01 = 150
        // Annual fee income = 150 * 365 = 54_750
        let fee_income = out.pool_fee_income.unwrap();
        assert_eq!(
            fee_income,
            dec!(54750),
            "Annual fee income should be 54750, got {}",
            fee_income
        );
    }

    // -----------------------------------------------------------------------
    // 9. Yield farm with reward APR stacking
    // -----------------------------------------------------------------------
    #[test]
    fn test_yield_farm_with_reward_apr() {
        let mut input = yield_farm_input();
        input.base_apr = Some(dec!(0.08)); // 8% base
        input.reward_apr = Some(dec!(0.04)); // 4% reward token
        input.compounding_frequency = Some(1); // annual compound

        let result = analyze_defi(&input).unwrap();
        let out = &result.result;

        // Total APR = 12%, annual compound => APY = 12% (no compounding benefit)
        assert_eq!(
            out.effective_apy,
            dec!(0.12),
            "With annual compounding, APY should equal APR of 12%, got {}",
            out.effective_apy
        );
    }

    // -----------------------------------------------------------------------
    // 10. Staking with compounding
    // -----------------------------------------------------------------------
    #[test]
    fn test_staking_with_compounding() {
        let mut input = staking_input();
        input.compounding = Some(true);

        let result = analyze_defi(&input).unwrap();
        let out = &result.result;

        // Net yield = 0.045
        // Compounded daily: (1 + 0.045/365)^365 - 1 ≈ 0.04603
        assert!(
            out.effective_apy > dec!(0.0460) && out.effective_apy < dec!(0.0461),
            "Compounded staking APY should be ~4.60%, got {}",
            out.effective_apy
        );

        // Should still be greater than simple yield
        let simple_yield = out.staking_effective_yield.unwrap();
        assert!(
            out.effective_apy > simple_yield,
            "Compounded APY ({}) should exceed simple yield ({})",
            out.effective_apy,
            simple_yield
        );
    }

    // -----------------------------------------------------------------------
    // 11. Liquidity pool IL calculation
    // -----------------------------------------------------------------------
    #[test]
    fn test_liquidity_pool_il_from_price_change() {
        let input = lp_input(); // price_change_pct = 0.50 (50% increase)
        let result = analyze_defi(&input).unwrap();
        let out = &result.result;

        // price_ratio = 1.5
        // IL = 2*sqrt(1.5)/(1+1.5) - 1 = 2*1.22474/2.5 - 1 = 0.97980 - 1 = -0.02020
        let il = out.impermanent_loss_pct.unwrap();
        assert!(
            il < dec!(-0.019) && il > dec!(-0.021),
            "IL for 50% price change should be ~-2.02%, got {}",
            il
        );
    }

    // -----------------------------------------------------------------------
    // 12. Yield farm total return calculation
    // -----------------------------------------------------------------------
    #[test]
    fn test_yield_farm_total_return() {
        let mut input = yield_farm_input();
        input.holding_period_days = Some(182); // ~6 months

        let result = analyze_defi(&input).unwrap();
        let out = &result.result;

        // total_return = principal * net_apy * (182/365)
        // net_apy ≈ 0.12749 (no gas), holding_fraction ≈ 0.4986
        let holding_fraction = dec!(182) / dec!(365);
        let expected_approx = dec!(10000) * out.effective_apy * holding_fraction;
        let diff = (out.total_return - expected_approx).abs();
        assert!(
            diff < dec!(0.01),
            "Total return should be ~{}, got {}",
            expected_approx,
            out.total_return
        );
    }

    // -----------------------------------------------------------------------
    // 13. Validation: zero compounding frequency rejected
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_zero_compounding_frequency() {
        let mut input = yield_farm_input();
        input.compounding_frequency = Some(0);

        let result = analyze_defi(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "compounding_frequency");
            }
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // 14. Validation: negative principal rejected
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_negative_principal() {
        let mut input = yield_farm_input();
        input.principal = Some(dec!(-1000));

        let result = analyze_defi(&input);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // 15. Validation: missing required field
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_missing_staking_field() {
        let mut input = staking_input();
        input.staked_amount = None; // required

        let result = analyze_defi(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "staked_amount");
            }
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // 16. IL: no price change => zero IL
    // -----------------------------------------------------------------------
    #[test]
    fn test_il_no_price_change() {
        let mut input = il_input();
        input.final_price_a = input.initial_price_a; // no change
        input.final_price_b = input.initial_price_b;

        let result = analyze_defi(&input).unwrap();
        let out = &result.result;

        let il = out.impermanent_loss_pct.unwrap();
        assert!(
            il.abs() < dec!(0.0001),
            "No price change should yield ~0 IL, got {}",
            il
        );
    }

    // -----------------------------------------------------------------------
    // 17. Newton sqrt helper: known values
    // -----------------------------------------------------------------------
    #[test]
    fn test_newton_sqrt_known_values() {
        // sqrt(4) = 2
        let s4 = newton_sqrt(dec!(4)).unwrap();
        let diff4 = (s4 - dec!(2)).abs();
        assert!(diff4 < dec!(0.0000001), "sqrt(4) should be 2, got {}", s4);

        // sqrt(2) ≈ 1.41421356
        let s2 = newton_sqrt(dec!(2)).unwrap();
        let diff2 = (s2 - dec!(1.41421356)).abs();
        assert!(
            diff2 < dec!(0.00001),
            "sqrt(2) should be ~1.41421, got {}",
            s2
        );

        // sqrt(1) = 1
        let s1 = newton_sqrt(dec!(1)).unwrap();
        assert_eq!(s1, dec!(1));

        // sqrt(0) = 0
        let s0 = newton_sqrt(dec!(0)).unwrap();
        assert_eq!(s0, dec!(0));
    }

    // -----------------------------------------------------------------------
    // 18. Metadata is populated correctly
    // -----------------------------------------------------------------------
    #[test]
    fn test_metadata_populated() {
        let input = yield_farm_input();
        let result = analyze_defi(&input).unwrap();

        assert!(!result.methodology.is_empty());
        assert!(result.methodology.contains("DeFi"));
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
        assert!(!result.metadata.version.is_empty());
    }

    // -----------------------------------------------------------------------
    // 19. High APR warning
    // -----------------------------------------------------------------------
    #[test]
    fn test_high_apr_warning() {
        let mut input = yield_farm_input();
        input.base_apr = Some(dec!(15.0)); // 1500% APR

        let result = analyze_defi(&input).unwrap();
        assert!(
            result.result.warnings.iter().any(|w| w.contains("1000%")),
            "Should warn about APR exceeding 1000%"
        );
    }

    // -----------------------------------------------------------------------
    // 20. Liquidity pool: IL exceeds fees => negative total return
    // -----------------------------------------------------------------------
    #[test]
    fn test_lp_il_exceeds_fees_negative_return() {
        let mut input = lp_input();
        input.price_change_pct = Some(dec!(3.0)); // 300% price increase => severe IL
        input.daily_volume = Some(dec!(100)); // very low volume => low fees

        let result = analyze_defi(&input).unwrap();
        let out = &result.result;

        assert!(
            out.total_return < Decimal::ZERO,
            "With severe IL and low fees, total return should be negative, got {}",
            out.total_return
        );

        assert_eq!(out.il_vs_fees.as_deref(), Some("IL exceeds fees"),);
    }
}
