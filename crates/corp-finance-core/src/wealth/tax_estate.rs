use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{with_metadata, ComputationOutput, Money, Rate};
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Tax-Loss Harvesting Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlhPosition {
    pub ticker: String,
    pub market_value: Money,
    pub cost_basis: Money,
    pub holding_period_days: u32,
    pub unrealized_gain_loss: Money,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlhInput {
    pub portfolio_value: Money,
    pub positions: Vec<TlhPosition>,
    pub short_term_tax_rate: Rate,
    pub long_term_tax_rate: Rate,
    pub annual_capital_gains: Money,
    pub harvest_threshold_pct: Rate,
    pub wash_sale_days: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarvestCandidate {
    pub ticker: String,
    pub unrealized_loss: Money,
    pub loss_pct: Rate,
    pub is_short_term: bool,
    pub tax_savings: Money,
    pub recommended: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxSavings {
    pub gains_offset: Money,
    pub remaining_losses: Money,
    pub short_term_savings: Money,
    pub long_term_savings: Money,
    pub total_immediate_savings: Money,
    pub net_tax_benefit_ratio: Rate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioImpact {
    pub positions_harvested: u32,
    pub cash_raised: Money,
    pub new_cost_basis: Money,
    pub deferred_tax_created: Money,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlhOutput {
    pub harvest_candidates: Vec<HarvestCandidate>,
    pub total_harvestable_losses: Money,
    pub short_term_losses: Money,
    pub long_term_losses: Money,
    pub tax_savings: TaxSavings,
    pub portfolio_impact: PortfolioImpact,
}

// ---------------------------------------------------------------------------
// Estate Planning Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrustType {
    Revocable,
    Irrevocable,
    Grat,
    Ilit,
    Qprt,
    CrummeyTrust,
    CharitableRemainder,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiftPlan {
    pub recipient_name: String,
    pub annual_amount: Money,
    pub is_skip_person: bool,
    pub years_of_gifting: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustPlan {
    pub name: String,
    pub trust_type: TrustType,
    pub funded_amount: Money,
    pub annual_distribution: Money,
    pub expected_return: Rate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstatePlanInput {
    pub total_estate_value: Money,
    pub annual_gifting: Vec<GiftPlan>,
    pub estate_tax_exemption: Money,
    pub gift_tax_annual_exclusion: Money,
    pub estate_tax_rate: Rate,
    pub state_estate_tax_rate: Option<Rate>,
    pub state_exemption: Option<Money>,
    pub gst_tax_rate: Rate,
    pub gst_exemption: Money,
    pub trust_structures: Vec<TrustPlan>,
    pub charitable_bequests: Money,
    pub marital_deduction: Money,
    pub life_insurance_proceeds: Money,
    pub planning_horizon_years: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstateDeductions {
    pub marital_deduction: Money,
    pub charitable_deduction: Money,
    pub trust_deductions: Money,
    pub total_deductions: Money,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiftingAnalysis {
    pub total_gifts: Money,
    pub annual_exclusion_gifts: Money,
    pub taxable_gifts: Money,
    pub gst_gifts: Money,
    pub estate_reduction: Money,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustResult {
    pub name: String,
    pub trust_type: String,
    pub funded_amount: Money,
    pub projected_value: Money,
    pub estate_inclusion: bool,
    pub tax_savings: Money,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstatePlanOutput {
    pub gross_estate: Money,
    pub deductions: EstateDeductions,
    pub taxable_estate: Money,
    pub lifetime_gifts_used: Money,
    pub remaining_exemption: Money,
    pub federal_estate_tax: Money,
    pub state_estate_tax: Money,
    pub gst_tax: Money,
    pub total_taxes: Money,
    pub effective_tax_rate: Rate,
    pub net_to_heirs: Money,
    pub gifting_analysis: GiftingAnalysis,
    pub trust_analysis: Vec<TrustResult>,
    pub planning_strategies: Vec<String>,
}

// ---------------------------------------------------------------------------
// Function 1: Tax-Loss Harvesting Simulation
// ---------------------------------------------------------------------------

/// Simulate tax-loss harvesting across a portfolio of positions.
///
/// Identifies harvest candidates (positions with unrealized losses exceeding the
/// threshold), calculates tax savings from offsetting capital gains, and projects
/// portfolio impact including deferred tax from lower cost basis.
pub fn simulate_tax_loss_harvesting(
    input: &TlhInput,
) -> CorpFinanceResult<ComputationOutput<TlhOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // Validate inputs
    validate_tlh_input(input)?;

    // Identify harvest candidates: positions with unrealized losses
    let mut harvest_candidates: Vec<HarvestCandidate> = Vec::new();
    let mut total_harvestable_losses = Decimal::ZERO;
    let mut short_term_losses = Decimal::ZERO;
    let mut long_term_losses = Decimal::ZERO;

    for pos in &input.positions {
        // Only consider positions with losses (negative unrealized_gain_loss)
        if pos.unrealized_gain_loss >= Decimal::ZERO {
            continue;
        }

        let loss = pos.unrealized_gain_loss.abs();
        let loss_pct = if pos.cost_basis > Decimal::ZERO {
            loss / pos.cost_basis
        } else {
            Decimal::ZERO
        };

        let is_short_term = pos.holding_period_days < 365;
        let applicable_rate = if is_short_term {
            input.short_term_tax_rate
        } else {
            input.long_term_tax_rate
        };

        let candidate_savings = loss * applicable_rate;
        let recommended = loss_pct >= input.harvest_threshold_pct;

        if recommended {
            total_harvestable_losses += loss;
            if is_short_term {
                short_term_losses += loss;
            } else {
                long_term_losses += loss;
            }
        }

        harvest_candidates.push(HarvestCandidate {
            ticker: pos.ticker.clone(),
            unrealized_loss: loss,
            loss_pct,
            is_short_term,
            tax_savings: candidate_savings,
            recommended,
        });
    }

    // Calculate tax savings from offsetting gains
    // ST losses offset gains at ST rate first, then LT gains
    // LT losses offset gains at LT rate first, then ST gains
    let gains = input.annual_capital_gains;

    // ST losses offset up to the gains amount at the ST rate
    let st_offset = short_term_losses.min(gains);
    let st_savings = st_offset * input.short_term_tax_rate;

    // Remaining gains after ST offset
    let remaining_gains_after_st = (gains - st_offset).max(Decimal::ZERO);

    // LT losses offset remaining gains at LT rate
    let lt_offset = long_term_losses.min(remaining_gains_after_st);
    let lt_savings = lt_offset * input.long_term_tax_rate;

    let gains_offset = st_offset + lt_offset;
    let total_immediate_savings = st_savings + lt_savings;

    // Remaining losses carried forward
    let remaining_losses = (total_harvestable_losses - gains_offset).max(Decimal::ZERO);

    if remaining_losses > Decimal::ZERO {
        warnings.push(format!(
            "Excess losses of {} can offset up to $3,000 of ordinary income annually, \
             remainder carries forward to future tax years.",
            remaining_losses
        ));
    }

    let net_tax_benefit_ratio = if total_harvestable_losses > Decimal::ZERO {
        total_immediate_savings / total_harvestable_losses
    } else {
        Decimal::ZERO
    };

    let tax_savings = TaxSavings {
        gains_offset,
        remaining_losses,
        short_term_savings: st_savings,
        long_term_savings: lt_savings,
        total_immediate_savings,
        net_tax_benefit_ratio,
    };

    // Portfolio impact
    let recommended_count = harvest_candidates.iter().filter(|c| c.recommended).count() as u32;

    // Cash raised = market value of harvested (recommended) positions
    // We need to match candidates back to positions by ticker to get market value
    let cash_raised: Money = input
        .positions
        .iter()
        .filter(|p| {
            harvest_candidates
                .iter()
                .any(|c| c.ticker == p.ticker && c.recommended)
        })
        .map(|p| p.market_value)
        .sum();

    // New cost basis if reinvested at current prices equals the cash raised
    // (market value), which is lower than the original cost basis
    let new_cost_basis = cash_raised;

    // Deferred tax: the harvested losses create a lower basis, which means
    // future gains will be larger by that amount. Tax deferred =
    // harvested_losses * applicable blended rate.
    let blended_rate = if total_harvestable_losses > Decimal::ZERO {
        (short_term_losses * input.short_term_tax_rate
            + long_term_losses * input.long_term_tax_rate)
            / total_harvestable_losses
    } else {
        Decimal::ZERO
    };
    let deferred_tax_created = total_harvestable_losses * blended_rate;

    let portfolio_impact = PortfolioImpact {
        positions_harvested: recommended_count,
        cash_raised,
        new_cost_basis,
        deferred_tax_created,
    };

    let output = TlhOutput {
        harvest_candidates,
        total_harvestable_losses,
        short_term_losses,
        long_term_losses,
        tax_savings,
        portfolio_impact,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Tax-Loss Harvesting Simulation: identify harvest candidates, \
         calculate tax savings, and project portfolio impact",
        &serde_json::json!({
            "portfolio_value": input.portfolio_value.to_string(),
            "num_positions": input.positions.len(),
            "short_term_tax_rate": input.short_term_tax_rate.to_string(),
            "long_term_tax_rate": input.long_term_tax_rate.to_string(),
            "annual_capital_gains": input.annual_capital_gains.to_string(),
            "harvest_threshold_pct": input.harvest_threshold_pct.to_string(),
            "wash_sale_days": input.wash_sale_days,
        }),
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Function 2: Estate Planning
// ---------------------------------------------------------------------------

/// Comprehensive estate planning analysis including gift/estate tax,
/// generation-skipping transfer (GST) tax, trust structures, and strategies.
///
/// Calculates gross estate, applicable deductions, taxable estate, federal and
/// state estate taxes, GST tax on skip-person gifts, trust projected values,
/// and generates planning strategy recommendations.
pub fn plan_estate(
    input: &EstatePlanInput,
) -> CorpFinanceResult<ComputationOutput<EstatePlanOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // Validate inputs
    validate_estate_input(input)?;

    // ------------------------------------------------------------------
    // 1. Gifting Analysis
    // ------------------------------------------------------------------
    let mut total_gifts = Decimal::ZERO;
    let mut annual_exclusion_gifts = Decimal::ZERO;
    let mut taxable_gifts = Decimal::ZERO;
    let mut gst_gifts = Decimal::ZERO;

    for gift in &input.annual_gifting {
        let total_gift_amount = gift.annual_amount * Decimal::from(gift.years_of_gifting);
        total_gifts += total_gift_amount;

        // Annual exclusion: up to gift_tax_annual_exclusion per recipient per year
        let exclusion_per_year = gift.annual_amount.min(input.gift_tax_annual_exclusion);
        let total_exclusion = exclusion_per_year * Decimal::from(gift.years_of_gifting);
        annual_exclusion_gifts += total_exclusion;

        // Taxable gifts: amount above annual exclusion (uses lifetime exemption)
        let taxable_per_year =
            (gift.annual_amount - input.gift_tax_annual_exclusion).max(Decimal::ZERO);
        let total_taxable = taxable_per_year * Decimal::from(gift.years_of_gifting);
        taxable_gifts += total_taxable;

        // GST gifts: gifts to skip persons (grandchildren or further)
        if gift.is_skip_person {
            gst_gifts += total_gift_amount;
        }
    }

    // Estate reduction = total gifts removed from estate over the planning horizon
    let estate_reduction = total_gifts;

    let gifting_analysis = GiftingAnalysis {
        total_gifts,
        annual_exclusion_gifts,
        taxable_gifts,
        gst_gifts,
        estate_reduction,
    };

    // ------------------------------------------------------------------
    // 2. Trust Analysis
    // ------------------------------------------------------------------
    let mut trust_results: Vec<TrustResult> = Vec::new();
    let mut irrevocable_trust_total = Decimal::ZERO;

    for trust in &input.trust_structures {
        let estate_inclusion = matches!(trust.trust_type, TrustType::Revocable);

        // Project trust value over planning horizon using iterative multiplication
        let projected_value = project_value(
            trust.funded_amount,
            trust.expected_return,
            input.planning_horizon_years,
        );

        // Irrevocable trusts (and subtypes) remove assets from the estate
        let removes_from_estate = !estate_inclusion;
        if removes_from_estate {
            irrevocable_trust_total += trust.funded_amount;
        }

        // Tax savings = estate tax on the amount removed from estate
        let trust_tax_savings = if removes_from_estate {
            trust.funded_amount * input.estate_tax_rate
        } else {
            Decimal::ZERO
        };

        let type_name = match trust.trust_type {
            TrustType::Revocable => "Revocable",
            TrustType::Irrevocable => "Irrevocable",
            TrustType::Grat => "GRAT",
            TrustType::Ilit => "ILIT",
            TrustType::Qprt => "QPRT",
            TrustType::CrummeyTrust => "Crummey Trust",
            TrustType::CharitableRemainder => "Charitable Remainder Trust",
        };

        trust_results.push(TrustResult {
            name: trust.name.clone(),
            trust_type: type_name.to_string(),
            funded_amount: trust.funded_amount,
            projected_value,
            estate_inclusion,
            tax_savings: trust_tax_savings,
        });
    }

    // ------------------------------------------------------------------
    // 3. Gross Estate
    // ------------------------------------------------------------------
    // Life insurance: included in gross estate unless held in an ILIT
    let has_ilit = input
        .trust_structures
        .iter()
        .any(|t| matches!(t.trust_type, TrustType::Ilit));

    let insurance_in_estate = if has_ilit {
        Decimal::ZERO
    } else {
        input.life_insurance_proceeds
    };

    let gross_estate = input.total_estate_value + insurance_in_estate;

    // ------------------------------------------------------------------
    // 4. Deductions
    // ------------------------------------------------------------------
    let marital_deduction = input.marital_deduction;
    let charitable_deduction = input.charitable_bequests;
    let trust_deductions = irrevocable_trust_total;
    let total_deductions = marital_deduction + charitable_deduction + trust_deductions;

    let deductions = EstateDeductions {
        marital_deduction,
        charitable_deduction,
        trust_deductions,
        total_deductions,
    };

    // ------------------------------------------------------------------
    // 5. Taxable Estate and Federal Estate Tax
    // ------------------------------------------------------------------
    // Reduce gross estate by gifts already removed and deductions
    let estate_after_gifts = (gross_estate - estate_reduction).max(Decimal::ZERO);
    let taxable_estate = (estate_after_gifts - total_deductions).max(Decimal::ZERO);

    // Lifetime gifts that used exemption reduce available exemption
    let lifetime_gifts_used = taxable_gifts;
    let remaining_exemption = (input.estate_tax_exemption - lifetime_gifts_used).max(Decimal::ZERO);

    // Federal estate tax = max(0, (taxable_estate - remaining_exemption)) * rate
    let taxable_above_exemption = (taxable_estate - remaining_exemption).max(Decimal::ZERO);
    let federal_estate_tax = taxable_above_exemption * input.estate_tax_rate;

    // ------------------------------------------------------------------
    // 6. State Estate Tax
    // ------------------------------------------------------------------
    let state_estate_tax = match (input.state_estate_tax_rate, input.state_exemption) {
        (Some(state_rate), Some(state_exempt)) => {
            let state_taxable = (taxable_estate - state_exempt).max(Decimal::ZERO);
            state_taxable * state_rate
        }
        (Some(state_rate), None) => {
            // No separate state exemption — use federal
            let state_taxable = (taxable_estate - remaining_exemption).max(Decimal::ZERO);
            state_taxable * state_rate
        }
        _ => Decimal::ZERO,
    };

    // ------------------------------------------------------------------
    // 7. GST Tax
    // ------------------------------------------------------------------
    // GST tax applies to gifts to skip persons above the GST exemption
    let gst_taxable = (gst_gifts - input.gst_exemption).max(Decimal::ZERO);
    let gst_tax = gst_taxable * input.gst_tax_rate;

    if gst_tax > Decimal::ZERO {
        warnings.push(format!(
            "GST tax of {} triggered on skip-person gifts exceeding the {} exemption.",
            gst_tax, input.gst_exemption
        ));
    }

    // ------------------------------------------------------------------
    // 8. Total Taxes and Net to Heirs
    // ------------------------------------------------------------------
    let total_taxes = federal_estate_tax + state_estate_tax + gst_tax;

    let effective_tax_rate = if gross_estate > Decimal::ZERO {
        total_taxes / gross_estate
    } else {
        Decimal::ZERO
    };

    let net_to_heirs = gross_estate - total_taxes - charitable_deduction;

    // ------------------------------------------------------------------
    // 9. Planning Strategies
    // ------------------------------------------------------------------
    let planning_strategies = generate_planning_strategies(
        input,
        &gifting_analysis,
        &trust_results,
        taxable_estate,
        remaining_exemption,
        federal_estate_tax,
        has_ilit,
    );

    // ------------------------------------------------------------------
    // 10. Assemble Output
    // ------------------------------------------------------------------
    let output = EstatePlanOutput {
        gross_estate,
        deductions,
        taxable_estate,
        lifetime_gifts_used,
        remaining_exemption,
        federal_estate_tax,
        state_estate_tax,
        gst_tax,
        total_taxes,
        effective_tax_rate,
        net_to_heirs,
        gifting_analysis,
        trust_analysis: trust_results,
        planning_strategies,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Estate Planning Analysis: gift/estate tax, GST, trust structures, \
         and planning strategies",
        &serde_json::json!({
            "total_estate_value": input.total_estate_value.to_string(),
            "estate_tax_exemption": input.estate_tax_exemption.to_string(),
            "estate_tax_rate": input.estate_tax_rate.to_string(),
            "gst_tax_rate": input.gst_tax_rate.to_string(),
            "planning_horizon_years": input.planning_horizon_years,
            "num_gift_plans": input.annual_gifting.len(),
            "num_trusts": input.trust_structures.len(),
        }),
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn validate_tlh_input(input: &TlhInput) -> CorpFinanceResult<()> {
    if input.portfolio_value <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "portfolio_value".into(),
            reason: "Portfolio value must be positive".into(),
        });
    }
    if input.short_term_tax_rate < Decimal::ZERO || input.short_term_tax_rate > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "short_term_tax_rate".into(),
            reason: "Short-term tax rate must be between 0 and 1".into(),
        });
    }
    if input.long_term_tax_rate < Decimal::ZERO || input.long_term_tax_rate > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "long_term_tax_rate".into(),
            reason: "Long-term tax rate must be between 0 and 1".into(),
        });
    }
    if input.harvest_threshold_pct < Decimal::ZERO || input.harvest_threshold_pct > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "harvest_threshold_pct".into(),
            reason: "Harvest threshold must be between 0 and 1".into(),
        });
    }
    if input.annual_capital_gains < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "annual_capital_gains".into(),
            reason: "Annual capital gains cannot be negative".into(),
        });
    }
    Ok(())
}

fn validate_estate_input(input: &EstatePlanInput) -> CorpFinanceResult<()> {
    if input.total_estate_value <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "total_estate_value".into(),
            reason: "Total estate value must be positive".into(),
        });
    }
    if input.estate_tax_rate < Decimal::ZERO || input.estate_tax_rate > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "estate_tax_rate".into(),
            reason: "Estate tax rate must be between 0 and 1".into(),
        });
    }
    if input.estate_tax_exemption < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "estate_tax_exemption".into(),
            reason: "Estate tax exemption cannot be negative".into(),
        });
    }
    if input.gst_tax_rate < Decimal::ZERO || input.gst_tax_rate > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "gst_tax_rate".into(),
            reason: "GST tax rate must be between 0 and 1".into(),
        });
    }
    if input.planning_horizon_years == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "planning_horizon_years".into(),
            reason: "Planning horizon must be at least 1 year".into(),
        });
    }
    if let Some(state_rate) = input.state_estate_tax_rate {
        if state_rate < Decimal::ZERO || state_rate > Decimal::ONE {
            return Err(CorpFinanceError::InvalidInput {
                field: "state_estate_tax_rate".into(),
                reason: "State estate tax rate must be between 0 and 1".into(),
            });
        }
    }
    Ok(())
}

/// Project a value forward using iterative multiplication (avoids powd precision drift).
fn project_value(initial: Money, annual_return: Rate, years: u32) -> Money {
    let mut value = initial;
    let growth_factor = Decimal::ONE + annual_return;
    for _ in 0..years {
        value *= growth_factor;
    }
    value
}

/// Generate planning strategy recommendations based on the estate analysis.
fn generate_planning_strategies(
    input: &EstatePlanInput,
    gifting: &GiftingAnalysis,
    trusts: &[TrustResult],
    taxable_estate: Money,
    remaining_exemption: Money,
    federal_tax: Money,
    has_ilit: bool,
) -> Vec<String> {
    let mut strategies: Vec<String> = Vec::new();

    // Strategy: maximize annual exclusion gifting
    if gifting.annual_exclusion_gifts < gifting.total_gifts {
        strategies.push(
            "Maximize annual exclusion gifts to reduce the taxable estate without \
             using lifetime exemption. Consider gifting to multiple recipients \
             within the annual exclusion limit."
                .to_string(),
        );
    }

    // Strategy: use remaining exemption via lifetime gifts
    if remaining_exemption > Decimal::ZERO && taxable_estate > remaining_exemption {
        strategies.push(format!(
            "Remaining lifetime exemption of {} is available. Consider making \
             taxable gifts now to lock in the current exemption amount before \
             potential legislative reductions.",
            remaining_exemption
        ));
    }

    // Strategy: irrevocable trust to remove assets
    let has_irrevocable = trusts.iter().any(|t| !t.estate_inclusion);
    if !has_irrevocable && federal_tax > Decimal::ZERO {
        strategies.push(
            "Consider establishing irrevocable trusts (GRAT, ILIT, or Crummey Trust) \
             to remove appreciating assets from the taxable estate."
                .to_string(),
        );
    }

    // Strategy: ILIT for life insurance
    if input.life_insurance_proceeds > Decimal::ZERO && !has_ilit {
        strategies.push(
            "Life insurance proceeds are included in the gross estate. \
             Transfer the policy to an Irrevocable Life Insurance Trust (ILIT) \
             to exclude proceeds from the estate."
                .to_string(),
        );
    }

    // Strategy: charitable giving
    if input.charitable_bequests == Decimal::ZERO && federal_tax > Decimal::ZERO {
        strategies.push(
            "Charitable bequests reduce the taxable estate dollar-for-dollar. \
             Consider a Charitable Remainder Trust (CRT) for income and estate \
             tax benefits."
                .to_string(),
        );
    }

    // Strategy: marital deduction (portability)
    if input.marital_deduction == Decimal::ZERO && federal_tax > Decimal::ZERO {
        strategies.push(
            "The unlimited marital deduction can defer estate tax until the \
             surviving spouse's death. Consider portability election to transfer \
             unused exemption to the surviving spouse."
                .to_string(),
        );
    }

    // Strategy: GRAT for appreciating assets
    let has_grat = trusts.iter().any(|t| t.trust_type == "GRAT");
    if !has_grat && federal_tax > Decimal::ZERO {
        strategies.push(
            "A Grantor Retained Annuity Trust (GRAT) can transfer asset \
             appreciation to heirs with minimal gift tax, especially effective \
             in low-interest-rate environments."
                .to_string(),
        );
    }

    // Strategy: GST planning
    if gifting.gst_gifts > Decimal::ZERO && gifting.gst_gifts > input.gst_exemption {
        strategies.push(
            "GST gifts exceed the exemption. Consider dynasty trusts or \
             allocating GST exemption to trusts that benefit skip persons \
             to avoid the generation-skipping transfer tax."
                .to_string(),
        );
    }

    // Strategy: state estate tax planning
    if let Some(state_rate) = input.state_estate_tax_rate {
        if state_rate > Decimal::ZERO {
            strategies.push(
                "State estate tax applies. Consider domicile planning or \
                 use of qualified personal residence trusts (QPRTs) to \
                 reduce state-level exposure."
                    .to_string(),
            );
        }
    }

    strategies
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    // ---------------------------------------------------------------
    // TLH Test Helpers
    // ---------------------------------------------------------------

    fn sample_tlh_input() -> TlhInput {
        TlhInput {
            portfolio_value: dec!(1_000_000),
            positions: vec![
                TlhPosition {
                    ticker: "AAPL".into(),
                    market_value: dec!(80_000),
                    cost_basis: dec!(100_000),
                    holding_period_days: 200,
                    unrealized_gain_loss: dec!(-20_000),
                },
                TlhPosition {
                    ticker: "MSFT".into(),
                    market_value: dec!(150_000),
                    cost_basis: dec!(120_000),
                    holding_period_days: 400,
                    unrealized_gain_loss: dec!(30_000),
                },
                TlhPosition {
                    ticker: "GOOG".into(),
                    market_value: dec!(60_000),
                    cost_basis: dec!(100_000),
                    holding_period_days: 500,
                    unrealized_gain_loss: dec!(-40_000),
                },
                TlhPosition {
                    ticker: "AMZN".into(),
                    market_value: dec!(95_000),
                    cost_basis: dec!(100_000),
                    holding_period_days: 100,
                    unrealized_gain_loss: dec!(-5_000),
                },
            ],
            short_term_tax_rate: dec!(0.37),
            long_term_tax_rate: dec!(0.20),
            annual_capital_gains: dec!(50_000),
            harvest_threshold_pct: dec!(0.10),
            wash_sale_days: 30,
        }
    }

    fn sample_estate_input() -> EstatePlanInput {
        EstatePlanInput {
            total_estate_value: dec!(25_000_000),
            annual_gifting: vec![
                GiftPlan {
                    recipient_name: "Child 1".into(),
                    annual_amount: dec!(18_000),
                    is_skip_person: false,
                    years_of_gifting: 10,
                },
                GiftPlan {
                    recipient_name: "Grandchild 1".into(),
                    annual_amount: dec!(50_000),
                    is_skip_person: true,
                    years_of_gifting: 10,
                },
            ],
            estate_tax_exemption: dec!(13_610_000),
            gift_tax_annual_exclusion: dec!(18_000),
            estate_tax_rate: dec!(0.40),
            state_estate_tax_rate: None,
            state_exemption: None,
            gst_tax_rate: dec!(0.40),
            gst_exemption: dec!(13_610_000),
            trust_structures: vec![
                TrustPlan {
                    name: "Family Irrevocable Trust".into(),
                    trust_type: TrustType::Irrevocable,
                    funded_amount: dec!(2_000_000),
                    annual_distribution: dec!(80_000),
                    expected_return: dec!(0.07),
                },
                TrustPlan {
                    name: "Revocable Living Trust".into(),
                    trust_type: TrustType::Revocable,
                    funded_amount: dec!(5_000_000),
                    annual_distribution: dec!(200_000),
                    expected_return: dec!(0.05),
                },
            ],
            charitable_bequests: dec!(1_000_000),
            marital_deduction: dec!(5_000_000),
            life_insurance_proceeds: dec!(3_000_000),
            planning_horizon_years: 20,
        }
    }

    // ---------------------------------------------------------------
    // TLH Tests
    // ---------------------------------------------------------------

    #[test]
    fn test_tlh_identify_candidates_with_losses_above_threshold() {
        let input = sample_tlh_input();
        let result = simulate_tax_loss_harvesting(&input).unwrap();
        let out = &result.result;

        // AAPL: -20k on 100k basis = 20% loss > 10% threshold -> recommended
        // GOOG: -40k on 100k basis = 40% loss > 10% threshold -> recommended
        // AMZN: -5k on 100k basis = 5% loss < 10% threshold -> not recommended
        let recommended: Vec<&HarvestCandidate> = out
            .harvest_candidates
            .iter()
            .filter(|c| c.recommended)
            .collect();
        assert_eq!(recommended.len(), 2, "Should recommend AAPL and GOOG");

        let tickers: Vec<&str> = recommended.iter().map(|c| c.ticker.as_str()).collect();
        assert!(tickers.contains(&"AAPL"));
        assert!(tickers.contains(&"GOOG"));
    }

    #[test]
    fn test_tlh_short_term_vs_long_term_classification() {
        let input = sample_tlh_input();
        let result = simulate_tax_loss_harvesting(&input).unwrap();
        let out = &result.result;

        // AAPL: 200 days -> short-term
        let aapl = out
            .harvest_candidates
            .iter()
            .find(|c| c.ticker == "AAPL")
            .unwrap();
        assert!(aapl.is_short_term, "AAPL (200 days) should be short-term");

        // GOOG: 500 days -> long-term
        let goog = out
            .harvest_candidates
            .iter()
            .find(|c| c.ticker == "GOOG")
            .unwrap();
        assert!(!goog.is_short_term, "GOOG (500 days) should be long-term");
    }

    #[test]
    fn test_tlh_tax_savings_from_offsetting_gains() {
        let input = sample_tlh_input();
        let result = simulate_tax_loss_harvesting(&input).unwrap();
        let out = &result.result;

        // ST losses (AAPL): 20,000
        // LT losses (GOOG): 40,000
        // Annual gains: 50,000
        // ST offset = min(20000, 50000) = 20000; ST savings = 20000 * 0.37 = 7400
        // Remaining gains = 50000 - 20000 = 30000
        // LT offset = min(40000, 30000) = 30000; LT savings = 30000 * 0.20 = 6000
        // Total immediate savings = 13400
        assert_eq!(out.tax_savings.short_term_savings, dec!(7_400));
        assert_eq!(out.tax_savings.long_term_savings, dec!(6_000));
        assert_eq!(out.tax_savings.total_immediate_savings, dec!(13_400));
        assert_eq!(out.tax_savings.gains_offset, dec!(50_000));
    }

    #[test]
    fn test_tlh_no_losses_no_harvest() {
        let input = TlhInput {
            portfolio_value: dec!(500_000),
            positions: vec![
                TlhPosition {
                    ticker: "SPY".into(),
                    market_value: dec!(300_000),
                    cost_basis: dec!(200_000),
                    holding_period_days: 400,
                    unrealized_gain_loss: dec!(100_000),
                },
                TlhPosition {
                    ticker: "QQQ".into(),
                    market_value: dec!(200_000),
                    cost_basis: dec!(150_000),
                    holding_period_days: 600,
                    unrealized_gain_loss: dec!(50_000),
                },
            ],
            short_term_tax_rate: dec!(0.37),
            long_term_tax_rate: dec!(0.20),
            annual_capital_gains: dec!(30_000),
            harvest_threshold_pct: dec!(0.10),
            wash_sale_days: 30,
        };

        let result = simulate_tax_loss_harvesting(&input).unwrap();
        let out = &result.result;

        assert!(
            out.harvest_candidates.is_empty(),
            "No positions with losses should yield no candidates"
        );
        assert_eq!(out.total_harvestable_losses, Decimal::ZERO);
        assert_eq!(out.tax_savings.total_immediate_savings, Decimal::ZERO);
        assert_eq!(out.portfolio_impact.positions_harvested, 0);
    }

    #[test]
    fn test_tlh_losses_exceed_gains_carry_forward() {
        let input = TlhInput {
            portfolio_value: dec!(500_000),
            positions: vec![TlhPosition {
                ticker: "TSLA".into(),
                market_value: dec!(50_000),
                cost_basis: dec!(150_000),
                holding_period_days: 400,
                unrealized_gain_loss: dec!(-100_000),
            }],
            short_term_tax_rate: dec!(0.37),
            long_term_tax_rate: dec!(0.20),
            annual_capital_gains: dec!(20_000),
            harvest_threshold_pct: dec!(0.10),
            wash_sale_days: 30,
        };

        let result = simulate_tax_loss_harvesting(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.total_harvestable_losses, dec!(100_000));
        assert_eq!(out.tax_savings.gains_offset, dec!(20_000));
        assert_eq!(out.tax_savings.remaining_losses, dec!(80_000));

        // Should warn about carry-forward
        assert!(
            result
                .warnings
                .iter()
                .any(|w| w.contains("carries forward")),
            "Should warn about excess losses carrying forward"
        );
    }

    #[test]
    fn test_tlh_portfolio_impact_new_cost_basis() {
        let input = sample_tlh_input();
        let result = simulate_tax_loss_harvesting(&input).unwrap();
        let out = &result.result;

        // Recommended positions: AAPL (MV 80k) and GOOG (MV 60k)
        assert_eq!(out.portfolio_impact.positions_harvested, 2);
        assert_eq!(out.portfolio_impact.cash_raised, dec!(140_000));
        // New cost basis = market value if reinvested
        assert_eq!(out.portfolio_impact.new_cost_basis, dec!(140_000));
    }

    #[test]
    fn test_tlh_deferred_tax_calculation() {
        let input = sample_tlh_input();
        let result = simulate_tax_loss_harvesting(&input).unwrap();
        let out = &result.result;

        // ST losses: 20,000 (AAPL) at 0.37 rate
        // LT losses: 40,000 (GOOG) at 0.20 rate
        // Blended: (20000*0.37 + 40000*0.20) / 60000 = (7400 + 8000) / 60000
        // = 15400 / 60000 = 0.256666...
        // Deferred tax = 60000 * 0.256666... = 15400
        let expected_deferred = dec!(20_000) * dec!(0.37) + dec!(40_000) * dec!(0.20);
        let diff = (out.portfolio_impact.deferred_tax_created - expected_deferred).abs();
        assert!(
            diff < dec!(0.01),
            "Deferred tax {} should be close to {}; diff = {}",
            out.portfolio_impact.deferred_tax_created,
            expected_deferred,
            diff
        );
    }

    // ---------------------------------------------------------------
    // Estate Planning Tests
    // ---------------------------------------------------------------

    #[test]
    fn test_estate_below_exemption_no_tax() {
        let mut input = sample_estate_input();
        input.total_estate_value = dec!(5_000_000);
        input.life_insurance_proceeds = Decimal::ZERO;
        input.annual_gifting = vec![];
        input.trust_structures = vec![];
        input.charitable_bequests = Decimal::ZERO;
        input.marital_deduction = Decimal::ZERO;

        let result = plan_estate(&input).unwrap();
        let out = &result.result;

        assert_eq!(
            out.federal_estate_tax,
            Decimal::ZERO,
            "Estate of $5M should be below $13.61M exemption"
        );
    }

    #[test]
    fn test_estate_above_exemption_tax_at_40_pct() {
        let mut input = sample_estate_input();
        input.total_estate_value = dec!(20_000_000);
        input.life_insurance_proceeds = Decimal::ZERO;
        input.annual_gifting = vec![];
        input.trust_structures = vec![];
        input.charitable_bequests = Decimal::ZERO;
        input.marital_deduction = Decimal::ZERO;

        let result = plan_estate(&input).unwrap();
        let out = &result.result;

        // Taxable estate = 20M, exemption = 13.61M
        // Tax = (20M - 13.61M) * 0.40 = 6.39M * 0.40 = 2,556,000
        let expected_tax = dec!(6_390_000) * dec!(0.40);
        assert_eq!(
            out.federal_estate_tax, expected_tax,
            "Federal estate tax should be 40% of amount above exemption"
        );
        assert!(out.federal_estate_tax > Decimal::ZERO);
    }

    #[test]
    fn test_estate_marital_deduction_reduces_taxable() {
        let mut input = sample_estate_input();
        input.total_estate_value = dec!(30_000_000);
        input.life_insurance_proceeds = Decimal::ZERO;
        input.annual_gifting = vec![];
        input.trust_structures = vec![];
        input.charitable_bequests = Decimal::ZERO;
        input.marital_deduction = dec!(15_000_000);

        let result = plan_estate(&input).unwrap();
        let out = &result.result;

        // Taxable = 30M - 15M marital = 15M
        assert_eq!(out.taxable_estate, dec!(15_000_000));
        // Tax = max(0, 15M - 13.61M) * 0.40 = 1.39M * 0.40 = 556,000
        let expected_tax = dec!(1_390_000) * dec!(0.40);
        assert_eq!(out.federal_estate_tax, expected_tax);
    }

    #[test]
    fn test_estate_charitable_deduction() {
        let mut input = sample_estate_input();
        input.total_estate_value = dec!(20_000_000);
        input.life_insurance_proceeds = Decimal::ZERO;
        input.annual_gifting = vec![];
        input.trust_structures = vec![];
        input.charitable_bequests = dec!(5_000_000);
        input.marital_deduction = Decimal::ZERO;

        let result = plan_estate(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.deductions.charitable_deduction, dec!(5_000_000));
        // Taxable = 20M - 5M charitable = 15M
        assert_eq!(out.taxable_estate, dec!(15_000_000));
    }

    #[test]
    fn test_estate_lifetime_gifts_reduce_exemption() {
        let mut input = sample_estate_input();
        input.total_estate_value = dec!(25_000_000);
        input.life_insurance_proceeds = Decimal::ZERO;
        input.trust_structures = vec![];
        input.charitable_bequests = Decimal::ZERO;
        input.marital_deduction = Decimal::ZERO;
        // Gift of $50k/yr for 10 years to grandchild
        // Annual exclusion = $18k, taxable = $32k/yr, total taxable = $320k
        input.annual_gifting = vec![GiftPlan {
            recipient_name: "Grandchild".into(),
            annual_amount: dec!(50_000),
            is_skip_person: true,
            years_of_gifting: 10,
        }];

        let result = plan_estate(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.lifetime_gifts_used, dec!(320_000));
        let expected_remaining = dec!(13_610_000) - dec!(320_000);
        assert_eq!(out.remaining_exemption, expected_remaining);
    }

    #[test]
    fn test_estate_gst_on_skip_person_gifts() {
        let mut input = sample_estate_input();
        input.total_estate_value = dec!(10_000_000);
        input.life_insurance_proceeds = Decimal::ZERO;
        input.trust_structures = vec![];
        input.charitable_bequests = Decimal::ZERO;
        input.marital_deduction = Decimal::ZERO;
        // Large skip-person gifting that exceeds GST exemption
        input.gst_exemption = dec!(1_000_000); // smaller exemption for testing
        input.annual_gifting = vec![GiftPlan {
            recipient_name: "Grandchild".into(),
            annual_amount: dec!(200_000),
            is_skip_person: true,
            years_of_gifting: 10,
        }];

        let result = plan_estate(&input).unwrap();
        let out = &result.result;

        // Total GST gifts = 200k * 10 = 2M
        assert_eq!(out.gifting_analysis.gst_gifts, dec!(2_000_000));
        // GST tax = max(0, 2M - 1M) * 0.40 = 400,000
        assert_eq!(out.gst_tax, dec!(400_000));
    }

    #[test]
    fn test_estate_annual_exclusion_gifts_not_taxable() {
        let mut input = sample_estate_input();
        input.total_estate_value = dec!(10_000_000);
        input.life_insurance_proceeds = Decimal::ZERO;
        input.trust_structures = vec![];
        input.charitable_bequests = Decimal::ZERO;
        input.marital_deduction = Decimal::ZERO;
        // Gift within annual exclusion
        input.annual_gifting = vec![GiftPlan {
            recipient_name: "Child".into(),
            annual_amount: dec!(18_000),
            is_skip_person: false,
            years_of_gifting: 10,
        }];

        let result = plan_estate(&input).unwrap();
        let out = &result.result;

        // All gifts within exclusion: taxable = 0
        assert_eq!(out.gifting_analysis.taxable_gifts, Decimal::ZERO);
        assert_eq!(out.lifetime_gifts_used, Decimal::ZERO);
        // Full exemption remains
        assert_eq!(out.remaining_exemption, dec!(13_610_000));
    }

    #[test]
    fn test_estate_irrevocable_trust_removes_from_estate() {
        let mut input = sample_estate_input();
        input.total_estate_value = dec!(20_000_000);
        input.life_insurance_proceeds = Decimal::ZERO;
        input.annual_gifting = vec![];
        input.charitable_bequests = Decimal::ZERO;
        input.marital_deduction = Decimal::ZERO;
        input.trust_structures = vec![TrustPlan {
            name: "Family Trust".into(),
            trust_type: TrustType::Irrevocable,
            funded_amount: dec!(3_000_000),
            annual_distribution: dec!(100_000),
            expected_return: dec!(0.06),
        }];

        let result = plan_estate(&input).unwrap();
        let out = &result.result;

        // Irrevocable trust deducts from taxable estate
        assert_eq!(out.deductions.trust_deductions, dec!(3_000_000));
        assert_eq!(out.taxable_estate, dec!(17_000_000));

        // Trust should not be included in estate
        let trust_r = &out.trust_analysis[0];
        assert!(!trust_r.estate_inclusion);
        assert!(trust_r.tax_savings > Decimal::ZERO);
    }

    #[test]
    fn test_estate_revocable_trust_included_in_estate() {
        let mut input = sample_estate_input();
        input.total_estate_value = dec!(20_000_000);
        input.life_insurance_proceeds = Decimal::ZERO;
        input.annual_gifting = vec![];
        input.charitable_bequests = Decimal::ZERO;
        input.marital_deduction = Decimal::ZERO;
        input.trust_structures = vec![TrustPlan {
            name: "Living Trust".into(),
            trust_type: TrustType::Revocable,
            funded_amount: dec!(5_000_000),
            annual_distribution: dec!(200_000),
            expected_return: dec!(0.05),
        }];

        let result = plan_estate(&input).unwrap();
        let out = &result.result;

        // Revocable trust does NOT reduce taxable estate
        assert_eq!(out.deductions.trust_deductions, Decimal::ZERO);
        assert_eq!(out.taxable_estate, dec!(20_000_000));

        let trust_r = &out.trust_analysis[0];
        assert!(trust_r.estate_inclusion);
        assert_eq!(trust_r.tax_savings, Decimal::ZERO);
    }

    #[test]
    fn test_estate_grat_ilit_qprt_trust_types() {
        let mut input = sample_estate_input();
        input.total_estate_value = dec!(30_000_000);
        input.life_insurance_proceeds = dec!(5_000_000);
        input.annual_gifting = vec![];
        input.charitable_bequests = Decimal::ZERO;
        input.marital_deduction = Decimal::ZERO;
        input.trust_structures = vec![
            TrustPlan {
                name: "GRAT".into(),
                trust_type: TrustType::Grat,
                funded_amount: dec!(2_000_000),
                annual_distribution: dec!(300_000),
                expected_return: dec!(0.08),
            },
            TrustPlan {
                name: "ILIT".into(),
                trust_type: TrustType::Ilit,
                funded_amount: dec!(100_000),
                annual_distribution: Decimal::ZERO,
                expected_return: dec!(0.04),
            },
            TrustPlan {
                name: "QPRT".into(),
                trust_type: TrustType::Qprt,
                funded_amount: dec!(1_500_000),
                annual_distribution: Decimal::ZERO,
                expected_return: dec!(0.03),
            },
        ];

        let result = plan_estate(&input).unwrap();
        let out = &result.result;

        // All three are irrevocable subtypes — should exclude from estate
        for t in &out.trust_analysis {
            assert!(
                !t.estate_inclusion,
                "{} should NOT be included in estate",
                t.trust_type
            );
            assert!(t.tax_savings > Decimal::ZERO);
        }

        // ILIT present: life insurance excluded from gross estate
        // Gross estate = 30M (no insurance included)
        assert_eq!(out.gross_estate, dec!(30_000_000));

        // Total trust deductions = 2M + 100k + 1.5M = 3,600,000
        assert_eq!(out.deductions.trust_deductions, dec!(3_600_000));
    }

    #[test]
    fn test_estate_state_estate_tax_calculation() {
        let mut input = sample_estate_input();
        input.total_estate_value = dec!(20_000_000);
        input.life_insurance_proceeds = Decimal::ZERO;
        input.annual_gifting = vec![];
        input.trust_structures = vec![];
        input.charitable_bequests = Decimal::ZERO;
        input.marital_deduction = Decimal::ZERO;
        input.state_estate_tax_rate = Some(dec!(0.16));
        input.state_exemption = Some(dec!(6_110_000));

        let result = plan_estate(&input).unwrap();
        let out = &result.result;

        // State tax = max(0, 20M - 6.11M) * 0.16 = 13.89M * 0.16 = 2,222,400
        let expected_state_tax = dec!(13_890_000) * dec!(0.16);
        assert_eq!(out.state_estate_tax, expected_state_tax);
        assert!(out.state_estate_tax > Decimal::ZERO);
    }

    #[test]
    fn test_estate_effective_tax_rate() {
        let mut input = sample_estate_input();
        input.total_estate_value = dec!(25_000_000);
        input.life_insurance_proceeds = Decimal::ZERO;
        input.annual_gifting = vec![];
        input.trust_structures = vec![];
        input.charitable_bequests = Decimal::ZERO;
        input.marital_deduction = Decimal::ZERO;

        let result = plan_estate(&input).unwrap();
        let out = &result.result;

        // effective_tax_rate = total_taxes / gross_estate
        let expected_rate = out.total_taxes / out.gross_estate;
        assert_eq!(out.effective_tax_rate, expected_rate);
        assert!(
            out.effective_tax_rate > Decimal::ZERO,
            "Effective rate should be positive for taxable estate"
        );
        assert!(
            out.effective_tax_rate < dec!(0.40),
            "Effective rate should be less than the marginal rate"
        );
    }

    #[test]
    fn test_estate_net_to_heirs() {
        let mut input = sample_estate_input();
        input.total_estate_value = dec!(20_000_000);
        input.life_insurance_proceeds = Decimal::ZERO;
        input.annual_gifting = vec![];
        input.trust_structures = vec![];
        input.charitable_bequests = dec!(1_000_000);
        input.marital_deduction = Decimal::ZERO;

        let result = plan_estate(&input).unwrap();
        let out = &result.result;

        // net_to_heirs = gross_estate - total_taxes - charitable
        let expected = out.gross_estate - out.total_taxes - dec!(1_000_000);
        assert_eq!(out.net_to_heirs, expected);
        assert!(out.net_to_heirs > Decimal::ZERO);
        assert!(out.net_to_heirs < out.gross_estate);
    }

    #[test]
    fn test_estate_trust_projected_value_with_growth() {
        let mut input = sample_estate_input();
        input.total_estate_value = dec!(10_000_000);
        input.life_insurance_proceeds = Decimal::ZERO;
        input.annual_gifting = vec![];
        input.charitable_bequests = Decimal::ZERO;
        input.marital_deduction = Decimal::ZERO;
        input.planning_horizon_years = 10;
        input.trust_structures = vec![TrustPlan {
            name: "Growth Trust".into(),
            trust_type: TrustType::Irrevocable,
            funded_amount: dec!(1_000_000),
            annual_distribution: dec!(0),
            expected_return: dec!(0.07),
        }];

        let result = plan_estate(&input).unwrap();
        let out = &result.result;

        let trust_r = &out.trust_analysis[0];

        // Projected value: 1M * (1.07)^10 via iterative multiplication
        let mut expected = dec!(1_000_000);
        for _ in 0..10 {
            expected *= dec!(1.07);
        }
        let diff = (trust_r.projected_value - expected).abs();
        assert!(
            diff < dec!(0.01),
            "Projected value {} should match iterative calc {}; diff = {}",
            trust_r.projected_value,
            expected,
            diff
        );
        assert!(trust_r.projected_value > dec!(1_000_000));
    }

    #[test]
    fn test_estate_planning_strategies_generated() {
        let input = sample_estate_input();
        let result = plan_estate(&input).unwrap();
        let out = &result.result;

        assert!(
            !out.planning_strategies.is_empty(),
            "Should generate at least one planning strategy"
        );
    }

    // ---------------------------------------------------------------
    // Additional Edge Case Tests
    // ---------------------------------------------------------------

    #[test]
    fn test_tlh_validation_negative_portfolio_value() {
        let mut input = sample_tlh_input();
        input.portfolio_value = dec!(-100);

        let result = simulate_tax_loss_harvesting(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "portfolio_value");
            }
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }

    #[test]
    fn test_estate_validation_zero_estate_value() {
        let mut input = sample_estate_input();
        input.total_estate_value = Decimal::ZERO;

        let result = plan_estate(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "total_estate_value");
            }
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }

    #[test]
    fn test_estate_life_insurance_excluded_with_ilit() {
        let mut input = sample_estate_input();
        input.total_estate_value = dec!(20_000_000);
        input.life_insurance_proceeds = dec!(5_000_000);
        input.annual_gifting = vec![];
        input.charitable_bequests = Decimal::ZERO;
        input.marital_deduction = Decimal::ZERO;
        input.trust_structures = vec![TrustPlan {
            name: "ILIT".into(),
            trust_type: TrustType::Ilit,
            funded_amount: dec!(50_000),
            annual_distribution: Decimal::ZERO,
            expected_return: dec!(0.03),
        }];

        let result = plan_estate(&input).unwrap();
        let out = &result.result;

        // With ILIT, insurance should be excluded from gross estate
        assert_eq!(out.gross_estate, dec!(20_000_000));
    }

    #[test]
    fn test_estate_life_insurance_included_without_ilit() {
        let mut input = sample_estate_input();
        input.total_estate_value = dec!(20_000_000);
        input.life_insurance_proceeds = dec!(5_000_000);
        input.annual_gifting = vec![];
        input.trust_structures = vec![];
        input.charitable_bequests = Decimal::ZERO;
        input.marital_deduction = Decimal::ZERO;

        let result = plan_estate(&input).unwrap();
        let out = &result.result;

        // Without ILIT, insurance is included
        assert_eq!(out.gross_estate, dec!(25_000_000));
    }

    #[test]
    fn test_estate_ilit_strategy_recommended_when_no_ilit() {
        let mut input = sample_estate_input();
        input.total_estate_value = dec!(25_000_000);
        input.life_insurance_proceeds = dec!(5_000_000);
        input.annual_gifting = vec![];
        input.trust_structures = vec![];
        input.charitable_bequests = Decimal::ZERO;
        input.marital_deduction = Decimal::ZERO;

        let result = plan_estate(&input).unwrap();
        let out = &result.result;

        assert!(
            out.planning_strategies
                .iter()
                .any(|s| s.contains("ILIT") || s.contains("Irrevocable Life Insurance")),
            "Should recommend ILIT when life insurance is in estate"
        );
    }

    #[test]
    fn test_estate_metadata_populated() {
        let input = sample_estate_input();
        let result = plan_estate(&input).unwrap();

        assert!(!result.methodology.is_empty());
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
        assert!(!result.metadata.version.is_empty());
    }

    #[test]
    fn test_tlh_metadata_populated() {
        let input = sample_tlh_input();
        let result = simulate_tax_loss_harvesting(&input).unwrap();

        assert!(!result.methodology.is_empty());
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
    }
}
