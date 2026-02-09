use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::*;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// How the acquirer pays for the target.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsiderationType {
    AllCash,
    AllStock,
    /// Mixed consideration: `cash_pct` is the fraction paid in cash (0..=1).
    /// The remainder (1 - cash_pct) is paid in stock.
    Mixed {
        cash_pct: Rate,
    },
}

/// Inputs for an accretion / dilution merger analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergerInput {
    // --- Acquirer ---
    pub acquirer_name: String,
    pub acquirer_net_income: Money,
    pub acquirer_shares_outstanding: Decimal,
    pub acquirer_share_price: Money,
    pub acquirer_tax_rate: Rate,

    // --- Target ---
    pub target_name: String,
    pub target_net_income: Money,
    pub target_shares_outstanding: Decimal,
    pub target_share_price: Money,

    // --- Deal terms ---
    pub offer_price_per_share: Money,
    pub consideration: ConsiderationType,

    // --- Synergies ---
    /// Pre-tax revenue synergies expected (annual run-rate).
    pub revenue_synergies: Option<Money>,
    /// Pre-tax cost synergies expected (annual run-rate).
    pub cost_synergies: Option<Money>,
    /// Fraction of synergies realised in year 1 (0..=1).
    pub synergy_phase_in_pct: Option<Rate>,
    /// One-time integration / restructuring costs.
    pub integration_costs: Option<Money>,

    // --- Financing (cash portion) ---
    /// Interest rate on new debt raised to fund the cash component.
    pub debt_financing_rate: Option<Rate>,
    /// Rate earned on cash balances that are foregone when paying cash.
    pub foregone_interest_rate: Option<Rate>,

    // --- Optional adjustments ---
    /// Annual goodwill amortisation charge (non-cash, pre-tax).
    pub goodwill_amortisation: Option<Money>,
    /// One-time transaction / advisory fees.
    pub transaction_fees: Option<Money>,
}

/// Results of the accretion / dilution analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergerOutput {
    // --- Deal metrics ---
    /// Total deal value (offer price * target shares).
    pub deal_value: Money,
    /// Offer premium as a decimal (e.g. 0.25 = 25%).
    pub premium_pct: Rate,
    /// Absolute premium per share.
    pub premium_amount: Money,

    // --- EPS analysis ---
    /// Acquirer standalone EPS before the deal.
    pub acquirer_eps_standalone: Money,
    /// Pro-forma EPS after the deal.
    pub pro_forma_eps: Money,
    /// Absolute change in EPS.
    pub eps_accretion_dilution: Money,
    /// Percentage change in EPS (decimal).
    pub eps_accretion_dilution_pct: Rate,
    /// `true` when pro-forma EPS >= standalone EPS.
    pub is_accretive: bool,

    // --- Share metrics ---
    /// Exchange ratio (offer price / acquirer share price). Applicable to stock deals.
    pub exchange_ratio: Option<Decimal>,
    /// New shares issued to target shareholders.
    pub new_shares_issued: Option<Decimal>,
    /// Pro-forma total shares outstanding.
    pub pro_forma_shares: Decimal,

    // --- Pro-forma financials ---
    /// Pro-forma net income after all adjustments.
    pub pro_forma_net_income: Money,
    /// Combined net income before synergies / adjustments.
    pub combined_net_income_pre_synergies: Money,
    /// Net synergy contribution to earnings (after tax, net of costs).
    pub synergy_impact: Money,
    /// After-tax financing cost for the cash component.
    pub financing_cost: Money,

    // --- Breakeven ---
    /// Pre-tax synergies required for EPS-neutral deal.
    pub breakeven_synergies: Money,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Perform an accretion / dilution analysis for a proposed merger.
///
/// Returns a `ComputationOutput<MergerOutput>` wrapped in the standard
/// result envelope with methodology, assumptions, warnings and metadata.
pub fn analyze_merger(input: &MergerInput) -> CorpFinanceResult<ComputationOutput<MergerOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // ------------------------------------------------------------------
    // 1. Validate inputs
    // ------------------------------------------------------------------
    validate_input(input)?;

    let zero = Decimal::ZERO;

    // ------------------------------------------------------------------
    // 2. Deal value & premium
    // ------------------------------------------------------------------
    let deal_value = input.offer_price_per_share * input.target_shares_outstanding;
    let premium_amount = input.offer_price_per_share - input.target_share_price;
    let premium_pct = premium_amount / input.target_share_price;

    if premium_pct < zero {
        warnings.push("Offer price is below current target share price (negative premium)".into());
    }

    // ------------------------------------------------------------------
    // 3. Acquirer standalone EPS
    // ------------------------------------------------------------------
    let acquirer_eps_standalone = input.acquirer_net_income / input.acquirer_shares_outstanding;

    // ------------------------------------------------------------------
    // 4. Consideration structure -> financing cost & new shares
    // ------------------------------------------------------------------
    let (financing_cost, new_shares_issued, exchange_ratio) =
        compute_consideration(input, deal_value, &mut warnings);

    // ------------------------------------------------------------------
    // 5. Pro-forma shares
    // ------------------------------------------------------------------
    let pro_forma_shares = input.acquirer_shares_outstanding + new_shares_issued.unwrap_or(zero);

    // ------------------------------------------------------------------
    // 6. Combined net income (pre-synergy, pre-adjustments)
    // ------------------------------------------------------------------
    let combined_net_income_pre_synergies = input.acquirer_net_income + input.target_net_income;

    // ------------------------------------------------------------------
    // 7. Synergy impact
    // ------------------------------------------------------------------
    let synergy_impact = compute_synergy_impact(input, &mut warnings);

    // ------------------------------------------------------------------
    // 8. Pro-forma net income
    // ------------------------------------------------------------------
    let pro_forma_net_income = combined_net_income_pre_synergies - financing_cost + synergy_impact;

    // ------------------------------------------------------------------
    // 9. Pro-forma EPS & accretion / dilution
    // ------------------------------------------------------------------
    let pro_forma_eps = pro_forma_net_income / pro_forma_shares;

    let eps_accretion_dilution = pro_forma_eps - acquirer_eps_standalone;
    let eps_accretion_dilution_pct = if acquirer_eps_standalone != zero {
        eps_accretion_dilution / acquirer_eps_standalone
    } else {
        zero
    };
    let is_accretive = eps_accretion_dilution >= zero;

    // ------------------------------------------------------------------
    // 10. Breakeven synergies
    // ------------------------------------------------------------------
    let breakeven_synergies = compute_breakeven_synergies(
        input,
        combined_net_income_pre_synergies,
        financing_cost,
        acquirer_eps_standalone,
        pro_forma_shares,
    );

    // ------------------------------------------------------------------
    // Build output
    // ------------------------------------------------------------------
    let output = MergerOutput {
        deal_value,
        premium_pct,
        premium_amount,
        acquirer_eps_standalone,
        pro_forma_eps,
        eps_accretion_dilution,
        eps_accretion_dilution_pct,
        is_accretive,
        exchange_ratio,
        new_shares_issued,
        pro_forma_shares,
        pro_forma_net_income,
        combined_net_income_pre_synergies,
        synergy_impact,
        financing_cost,
        breakeven_synergies,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "M&A Accretion/Dilution Analysis",
        &serde_json::json!({
            "acquirer": input.acquirer_name,
            "target": input.target_name,
            "consideration": format!("{:?}", input.consideration),
            "offer_price": input.offer_price_per_share.to_string(),
        }),
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Validate all required numeric constraints on the input.
fn validate_input(input: &MergerInput) -> CorpFinanceResult<()> {
    if input.acquirer_shares_outstanding <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "acquirer_shares_outstanding".into(),
            reason: "Acquirer shares outstanding must be positive".into(),
        });
    }
    if input.target_shares_outstanding <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "target_shares_outstanding".into(),
            reason: "Target shares outstanding must be positive".into(),
        });
    }
    if input.acquirer_share_price <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "acquirer_share_price".into(),
            reason: "Acquirer share price must be positive".into(),
        });
    }
    if input.target_share_price <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "target_share_price".into(),
            reason: "Target share price must be positive".into(),
        });
    }
    if input.offer_price_per_share <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "offer_price_per_share".into(),
            reason: "Offer price per share must be positive".into(),
        });
    }
    if input.acquirer_tax_rate < Decimal::ZERO || input.acquirer_tax_rate > dec!(1) {
        return Err(CorpFinanceError::InvalidInput {
            field: "acquirer_tax_rate".into(),
            reason: "Tax rate must be between 0 and 1".into(),
        });
    }

    // Validate mixed consideration cash_pct
    if let ConsiderationType::Mixed { cash_pct } = &input.consideration {
        if *cash_pct < Decimal::ZERO || *cash_pct > dec!(1) {
            return Err(CorpFinanceError::InvalidInput {
                field: "consideration.cash_pct".into(),
                reason: "Cash percentage must be between 0 and 1".into(),
            });
        }
    }

    Ok(())
}

/// Determine the after-tax financing cost, number of new shares issued, and
/// exchange ratio based on the consideration type.
fn compute_consideration(
    input: &MergerInput,
    deal_value: Money,
    warnings: &mut Vec<String>,
) -> (Money, Option<Decimal>, Option<Decimal>) {
    let one = dec!(1);
    let zero = Decimal::ZERO;
    let after_tax_multiplier = one - input.acquirer_tax_rate;

    match &input.consideration {
        ConsiderationType::AllCash => {
            let debt_cost = input
                .debt_financing_rate
                .map(|r| deal_value * r * after_tax_multiplier)
                .unwrap_or(zero);
            let foregone_cost = input
                .foregone_interest_rate
                .map(|r| deal_value * r * after_tax_multiplier)
                .unwrap_or(zero);

            if input.debt_financing_rate.is_none() && input.foregone_interest_rate.is_none() {
                warnings.push(
                    "All-cash deal with no financing rate specified; financing cost is zero".into(),
                );
            }

            let financing_cost = debt_cost + foregone_cost;
            (financing_cost, None, None)
        }
        ConsiderationType::AllStock => {
            let exchange_ratio = input.offer_price_per_share / input.acquirer_share_price;
            let new_shares = input.target_shares_outstanding * exchange_ratio;
            (zero, Some(new_shares), Some(exchange_ratio))
        }
        ConsiderationType::Mixed { cash_pct } => {
            let cash_portion = deal_value * *cash_pct;

            // Cash component financing cost
            let debt_cost = input
                .debt_financing_rate
                .map(|r| cash_portion * r * after_tax_multiplier)
                .unwrap_or(zero);
            let foregone_cost = input
                .foregone_interest_rate
                .map(|r| cash_portion * r * after_tax_multiplier)
                .unwrap_or(zero);
            let financing_cost = debt_cost + foregone_cost;

            // Stock component
            let exchange_ratio = input.offer_price_per_share / input.acquirer_share_price;
            let stock_pct = one - *cash_pct;
            let new_shares = input.target_shares_outstanding * exchange_ratio * stock_pct;

            (financing_cost, Some(new_shares), Some(exchange_ratio))
        }
    }
}

/// Calculate the net after-tax synergy impact on earnings.
///
/// Synergy impact = (cost_synergies + revenue_synergies) * phase_in_pct
///                  * (1 - tax_rate) - integration_costs
///                  - goodwill_amortisation - transaction_fees
fn compute_synergy_impact(input: &MergerInput, warnings: &mut Vec<String>) -> Money {
    let one = dec!(1);
    let zero = Decimal::ZERO;

    let gross_synergies =
        input.cost_synergies.unwrap_or(zero) + input.revenue_synergies.unwrap_or(zero);

    let phase_in = input.synergy_phase_in_pct.unwrap_or(one);

    let after_tax_synergies = gross_synergies * phase_in * (one - input.acquirer_tax_rate);

    let integration = input.integration_costs.unwrap_or(zero);
    let goodwill = input.goodwill_amortisation.unwrap_or(zero);
    let fees = input.transaction_fees.unwrap_or(zero);

    if gross_synergies == zero && (integration > zero || goodwill > zero || fees > zero) {
        warnings.push("No synergies specified but integration costs / fees are present".into());
    }

    after_tax_synergies - integration - goodwill - fees
}

/// Compute the pre-tax synergy amount that would make the deal EPS-neutral.
///
/// At breakeven: pro_forma_EPS = standalone_EPS
///
/// standalone_EPS = acquirer_NI / acquirer_shares
/// pro_forma_EPS  = (combined_NI - financing_cost + S_at * (1-t) * phase_in
///                   - integration - goodwill - fees) / pro_forma_shares
///
/// Setting pro_forma_EPS = standalone_EPS and solving for S (gross synergies):
///
/// S = [ standalone_EPS * pro_forma_shares - combined_NI + financing_cost
///       + integration + goodwill + fees ] / [ phase_in * (1 - tax_rate) ]
fn compute_breakeven_synergies(
    input: &MergerInput,
    combined_ni: Money,
    financing_cost: Money,
    standalone_eps: Money,
    pro_forma_shares: Decimal,
) -> Money {
    let one = dec!(1);
    let zero = Decimal::ZERO;

    let phase_in = input.synergy_phase_in_pct.unwrap_or(one);
    let after_tax_multiplier = (one - input.acquirer_tax_rate) * phase_in;

    // If the multiplier is zero we cannot solve; return zero.
    if after_tax_multiplier == zero {
        return zero;
    }

    let integration = input.integration_costs.unwrap_or(zero);
    let goodwill = input.goodwill_amortisation.unwrap_or(zero);
    let fees = input.transaction_fees.unwrap_or(zero);

    let target_ni = standalone_eps * pro_forma_shares;
    let numerator = target_ni - combined_ni + financing_cost + integration + goodwill + fees;

    let breakeven = numerator / after_tax_multiplier;

    // Breakeven synergies cannot be negative (negative means the deal is
    // already accretive without synergies).
    if breakeven < zero {
        zero
    } else {
        breakeven
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// Helper: build a base MergerInput with sensible defaults.
    fn base_input() -> MergerInput {
        MergerInput {
            acquirer_name: "AcquirerCo".into(),
            acquirer_net_income: dec!(500),
            acquirer_shares_outstanding: dec!(100),
            acquirer_share_price: dec!(50),
            acquirer_tax_rate: dec!(0.25),

            target_name: "TargetCo".into(),
            target_net_income: dec!(100),
            target_shares_outstanding: dec!(50),
            target_share_price: dec!(20),

            offer_price_per_share: dec!(25),
            consideration: ConsiderationType::AllCash,

            revenue_synergies: None,
            cost_synergies: None,
            synergy_phase_in_pct: None,
            integration_costs: None,

            debt_financing_rate: Some(dec!(0.05)),
            foregone_interest_rate: None,

            goodwill_amortisation: None,
            transaction_fees: None,
        }
    }

    // -----------------------------------------------------------------------
    // 1. All-cash accretive deal
    // -----------------------------------------------------------------------
    #[test]
    fn test_all_cash_accretive() {
        // Target earnings yield > after-tax cost of debt
        // Target NI / deal_value = 100 / 1250 = 8.0%
        // After-tax debt cost = 5% * (1 - 0.25) = 3.75%
        // 8% > 3.75% => accretive
        let input = base_input();
        let result = analyze_merger(&input).unwrap();
        let out = &result.result;

        // Deal value = 25 * 50 = 1250
        assert_eq!(out.deal_value, dec!(1250));

        // Financing cost = 1250 * 0.05 * 0.75 = 46.875
        assert_eq!(out.financing_cost, dec!(46.875));

        // Combined NI = 500 + 100 = 600
        assert_eq!(out.combined_net_income_pre_synergies, dec!(600));

        // Pro-forma NI = 600 - 46.875 + 0 = 553.125
        assert_eq!(out.pro_forma_net_income, dec!(553.125));

        // No new shares => pro-forma shares = 100
        assert_eq!(out.pro_forma_shares, dec!(100));
        assert!(out.new_shares_issued.is_none());

        // Standalone EPS = 500/100 = 5.00
        assert_eq!(out.acquirer_eps_standalone, dec!(5));

        // Pro-forma EPS = 553.125 / 100 = 5.53125
        assert_eq!(out.pro_forma_eps, dec!(5.53125));

        assert!(out.is_accretive);
        assert!(out.eps_accretion_dilution > Decimal::ZERO);
    }

    // -----------------------------------------------------------------------
    // 2. All-stock dilutive deal (target P/E > acquirer P/E)
    // -----------------------------------------------------------------------
    #[test]
    fn test_all_stock_dilutive() {
        // Acquirer P/E = 50 / (500/100) = 50/5 = 10x
        // Target P/E at offer = 25 / (100/50) = 25/2 = 12.5x
        // Target P/E > Acquirer P/E => dilutive in all-stock
        let mut input = base_input();
        input.consideration = ConsiderationType::AllStock;

        let result = analyze_merger(&input).unwrap();
        let out = &result.result;

        // Exchange ratio = 25 / 50 = 0.5
        assert_eq!(out.exchange_ratio.unwrap(), dec!(0.5));

        // New shares = 50 * 0.5 = 25
        assert_eq!(out.new_shares_issued.unwrap(), dec!(25));

        // Pro-forma shares = 100 + 25 = 125
        assert_eq!(out.pro_forma_shares, dec!(125));

        // Financing cost = 0 (all stock)
        assert_eq!(out.financing_cost, Decimal::ZERO);

        // Pro-forma NI = 600 + 0 = 600
        assert_eq!(out.pro_forma_net_income, dec!(600));

        // Pro-forma EPS = 600 / 125 = 4.80
        assert_eq!(out.pro_forma_eps, dec!(4.8));

        // Standalone EPS = 5.00 => dilutive
        assert!(!out.is_accretive);
        assert_eq!(out.eps_accretion_dilution, dec!(-0.2));
    }

    // -----------------------------------------------------------------------
    // 3. All-stock accretive deal (target P/E < acquirer P/E)
    // -----------------------------------------------------------------------
    #[test]
    fn test_all_stock_accretive() {
        // Make acquirer P/E high and target cheap
        // Acquirer P/E = 80 / (500/100) = 80/5 = 16x
        // Target P/E at offer = 25 / (100/50) = 12.5x
        // 12.5x < 16x => accretive
        let mut input = base_input();
        input.acquirer_share_price = dec!(80);
        input.consideration = ConsiderationType::AllStock;

        let result = analyze_merger(&input).unwrap();
        let out = &result.result;

        // Exchange ratio = 25/80 = 0.3125
        assert_eq!(out.exchange_ratio.unwrap(), dec!(0.3125));

        // New shares = 50 * 0.3125 = 15.625
        assert_eq!(out.new_shares_issued.unwrap(), dec!(15.625));

        // Pro-forma shares = 100 + 15.625 = 115.625
        assert_eq!(out.pro_forma_shares, dec!(115.625));

        // Pro-forma EPS = 600 / 115.625 ~ 5.189...
        // Standalone EPS = 5.00 => accretive
        assert!(out.is_accretive);
        assert!(out.pro_forma_eps > dec!(5));
    }

    // -----------------------------------------------------------------------
    // 4. Mixed consideration (50/50 cash/stock)
    // -----------------------------------------------------------------------
    #[test]
    fn test_mixed_consideration() {
        let mut input = base_input();
        input.consideration = ConsiderationType::Mixed {
            cash_pct: dec!(0.5),
        };
        input.debt_financing_rate = Some(dec!(0.05));

        let result = analyze_merger(&input).unwrap();
        let out = &result.result;

        // Deal value = 1250
        assert_eq!(out.deal_value, dec!(1250));

        // Cash portion = 625, stock portion = 625
        // Financing cost (cash) = 625 * 0.05 * 0.75 = 23.4375
        assert_eq!(out.financing_cost, dec!(23.4375));

        // Exchange ratio = 25/50 = 0.5
        assert_eq!(out.exchange_ratio.unwrap(), dec!(0.5));

        // New shares = 50 * 0.5 * 0.5 (stock_pct) = 12.5
        assert_eq!(out.new_shares_issued.unwrap(), dec!(12.5));

        // Pro-forma shares = 100 + 12.5 = 112.5
        assert_eq!(out.pro_forma_shares, dec!(112.5));

        // Pro-forma NI = 600 - 23.4375 = 576.5625
        assert_eq!(out.pro_forma_net_income, dec!(576.5625));

        // Pro-forma EPS = 576.5625 / 112.5 = 5.125
        assert_eq!(out.pro_forma_eps, dec!(5.125));

        assert!(out.is_accretive);
    }

    // -----------------------------------------------------------------------
    // 5. Synergies make a dilutive deal accretive
    // -----------------------------------------------------------------------
    #[test]
    fn test_synergies_make_accretive() {
        // Start with the dilutive all-stock case
        let mut input = base_input();
        input.consideration = ConsiderationType::AllStock;

        // Confirm it is dilutive without synergies
        let result_no_syn = analyze_merger(&input).unwrap();
        assert!(!result_no_syn.result.is_accretive);

        // Add cost synergies large enough to flip it
        // Need to add > 0.2 EPS * 125 shares = 25 NI after tax
        // 25 / 0.75 = 33.33 pre-tax synergies
        // Use 50 to be safely accretive
        input.cost_synergies = Some(dec!(50));
        input.synergy_phase_in_pct = Some(dec!(1));

        let result = analyze_merger(&input).unwrap();
        let out = &result.result;

        // Synergy impact = 50 * 1.0 * 0.75 = 37.5
        assert_eq!(out.synergy_impact, dec!(37.5));

        // Pro-forma NI = 600 + 37.5 = 637.5
        assert_eq!(out.pro_forma_net_income, dec!(637.5));

        // Pro-forma EPS = 637.5 / 125 = 5.10
        assert_eq!(out.pro_forma_eps, dec!(5.1));

        assert!(out.is_accretive);
    }

    // -----------------------------------------------------------------------
    // 6. Premium calculation
    // -----------------------------------------------------------------------
    #[test]
    fn test_premium_calculation() {
        let input = base_input();
        let result = analyze_merger(&input).unwrap();
        let out = &result.result;

        // Premium = (25 - 20) / 20 = 0.25 (25%)
        assert_eq!(out.premium_pct, dec!(0.25));
        assert_eq!(out.premium_amount, dec!(5));
    }

    // -----------------------------------------------------------------------
    // 7. Exchange ratio & shares issued
    // -----------------------------------------------------------------------
    #[test]
    fn test_exchange_ratio() {
        let mut input = base_input();
        input.consideration = ConsiderationType::AllStock;
        input.offer_price_per_share = dec!(30);
        input.acquirer_share_price = dec!(60);

        let result = analyze_merger(&input).unwrap();
        let out = &result.result;

        // Exchange ratio = 30/60 = 0.5
        assert_eq!(out.exchange_ratio.unwrap(), dec!(0.5));

        // New shares = 50 * 0.5 = 25
        assert_eq!(out.new_shares_issued.unwrap(), dec!(25));

        // Pro-forma shares = 100 + 25 = 125
        assert_eq!(out.pro_forma_shares, dec!(125));
    }

    // -----------------------------------------------------------------------
    // 8. Breakeven synergies
    // -----------------------------------------------------------------------
    #[test]
    fn test_breakeven_synergies() {
        // Use the dilutive all-stock scenario
        let mut input = base_input();
        input.consideration = ConsiderationType::AllStock;

        let result = analyze_merger(&input).unwrap();
        let out = &result.result;

        // The deal is dilutive; breakeven synergies should be > 0
        assert!(out.breakeven_synergies > Decimal::ZERO);

        // Verify: applying the breakeven synergies should make EPS ~= standalone
        let mut verify_input = input.clone();
        verify_input.cost_synergies = Some(out.breakeven_synergies);
        verify_input.synergy_phase_in_pct = Some(dec!(1));

        let verify_result = analyze_merger(&verify_input).unwrap();
        let eps_diff = (verify_result.result.pro_forma_eps
            - verify_result.result.acquirer_eps_standalone)
            .abs();

        // Should be essentially zero (within rounding tolerance)
        assert!(
            eps_diff < dec!(0.0001),
            "Breakeven synergies did not produce EPS-neutral result; diff = {eps_diff}"
        );
    }

    // -----------------------------------------------------------------------
    // 9. Zero shares error
    // -----------------------------------------------------------------------
    #[test]
    fn test_zero_shares_error() {
        let mut input = base_input();
        input.acquirer_shares_outstanding = Decimal::ZERO;

        let result = analyze_merger(&input);
        assert!(result.is_err());

        let err = result.unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "acquirer_shares_outstanding");
            }
            other => panic!("Expected InvalidInput error, got: {other}"),
        }

        // Also test target zero shares
        let mut input2 = base_input();
        input2.target_shares_outstanding = Decimal::ZERO;
        assert!(analyze_merger(&input2).is_err());
    }

    // -----------------------------------------------------------------------
    // 10. Foregone interest cost adds to financing cost
    // -----------------------------------------------------------------------
    #[test]
    fn test_foregone_interest() {
        let mut input = base_input();
        input.debt_financing_rate = Some(dec!(0.05));
        input.foregone_interest_rate = Some(dec!(0.02));

        let result = analyze_merger(&input).unwrap();
        let out = &result.result;

        // Financing cost = 1250 * 0.05 * 0.75 + 1250 * 0.02 * 0.75
        //                = 46.875 + 18.75 = 65.625
        assert_eq!(out.financing_cost, dec!(65.625));
    }

    // -----------------------------------------------------------------------
    // 11. Integration costs, goodwill, fees reduce synergy impact
    // -----------------------------------------------------------------------
    #[test]
    fn test_adjustments_reduce_earnings() {
        let mut input = base_input();
        input.cost_synergies = Some(dec!(100));
        input.synergy_phase_in_pct = Some(dec!(1));
        input.integration_costs = Some(dec!(10));
        input.goodwill_amortisation = Some(dec!(5));
        input.transaction_fees = Some(dec!(3));

        let result = analyze_merger(&input).unwrap();
        let out = &result.result;

        // Synergy impact = 100 * 1.0 * 0.75 - 10 - 5 - 3 = 75 - 18 = 57
        assert_eq!(out.synergy_impact, dec!(57));
    }

    // -----------------------------------------------------------------------
    // 12. Methodology string
    // -----------------------------------------------------------------------
    #[test]
    fn test_methodology_string() {
        let input = base_input();
        let result = analyze_merger(&input).unwrap();
        assert_eq!(result.methodology, "M&A Accretion/Dilution Analysis");
    }
}
