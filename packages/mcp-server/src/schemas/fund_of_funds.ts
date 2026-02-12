import { z } from "zod";

export const JCurveSchema = z.object({
  total_commitment: z.coerce.number().describe("Total fund commitment"),
  drawdown_schedule: z.array(z.coerce.number()).describe("Drawdown rates by year as decimals"),
  distribution_schedule: z.array(z.coerce.number()).describe("Distribution rates by year as decimals"),
  fund_life_years: z.coerce.number().int().describe("Fund life in years"),
  growth_rate: z.coerce.number().describe("Expected portfolio growth rate"),
  management_fee_pct: z.coerce.number().describe("Annual management fee as decimal"),
  carry_pct: z.coerce.number().describe("Carried interest as decimal (e.g. 0.20)"),
  preferred_return: z.coerce.number().describe("Preferred return hurdle as decimal"),
  public_index_returns: z.array(z.coerce.number()).describe("Public market index returns by year for PME"),
});

export const CommitmentPacingSchema = z.object({
  existing_funds: z.array(z.object({
    vintage: z.coerce.number().int().describe("Vintage year"),
    commitment: z.coerce.number().describe("Total commitment"),
    unfunded: z.coerce.number().describe("Remaining unfunded commitment"),
    nav: z.coerce.number().describe("Current NAV"),
    drawdown_rate: z.coerce.number().describe("Expected annual drawdown rate"),
    distribution_rate: z.coerce.number().describe("Expected annual distribution rate"),
  })).describe("Existing fund commitments"),
  target_allocation_pct: z.coerce.number().describe("Target allocation to PE as decimal"),
  total_portfolio_value: z.coerce.number().describe("Total portfolio value"),
  planning_years: z.coerce.number().int().describe("Number of years to project"),
  new_commitment_per_year: z.coerce.number().describe("Planned new commitment per year"),
  drawdown_curve: z.array(z.coerce.number()).describe("Expected drawdown curve by fund age"),
  distribution_curve: z.array(z.coerce.number()).describe("Expected distribution curve by fund age"),
});

export const ManagerSelectionSchema = z.object({
  manager_name: z.string().describe("Manager or GP name"),
  funds: z.array(z.object({
    name: z.string().describe("Fund name"),
    vintage: z.coerce.number().int().describe("Vintage year"),
    irr: z.coerce.number().describe("Net IRR as decimal"),
    tvpi: z.coerce.number().describe("Total value to paid-in multiple"),
    dpi: z.coerce.number().describe("Distributions to paid-in multiple"),
    pme: z.coerce.number().describe("Public market equivalent"),
  })).describe("Historical fund performance"),
  qualitative_scores: z.array(z.object({
    factor: z.string().describe("Qualitative factor name"),
    weight: z.coerce.number().describe("Factor weight"),
    score: z.coerce.number().describe("Factor score (1-10)"),
  })).describe("Qualitative assessment scores"),
  benchmark_quartiles: z.array(z.object({
    metric: z.string().describe("Performance metric name"),
    q1: z.coerce.number().describe("First quartile threshold"),
    median: z.coerce.number().describe("Median value"),
    q3: z.coerce.number().describe("Third quartile threshold"),
  })).describe("Benchmark quartile data"),
});

export const SecondariesPricingSchema = z.object({
  fund_nav: z.coerce.number().describe("Current fund NAV"),
  unfunded_commitment: z.coerce.number().describe("Remaining unfunded commitment"),
  remaining_life_years: z.coerce.number().describe("Expected remaining fund life in years"),
  expected_distribution_rate: z.coerce.number().describe("Expected annual distribution rate"),
  expected_growth_rate: z.coerce.number().describe("Expected NAV growth rate"),
  discount_rate: z.coerce.number().describe("Discount rate for cash flow PV"),
  management_fee_pct: z.coerce.number().describe("Annual management fee as decimal"),
  carry_pct: z.coerce.number().describe("Carried interest as decimal"),
});

export const FofPortfolioSchema = z.object({
  funds: z.array(z.object({
    name: z.string().describe("Fund name"),
    strategy: z.string().describe("Strategy (e.g. Buyout, Venture, Growth)"),
    vintage: z.coerce.number().int().describe("Vintage year"),
    geography: z.string().describe("Geographic focus"),
    commitment: z.coerce.number().describe("Total commitment"),
    nav: z.coerce.number().describe("Current NAV"),
    irr: z.coerce.number().describe("Net IRR as decimal"),
    tvpi: z.coerce.number().describe("TVPI multiple"),
  })).describe("Fund of funds portfolio holdings"),
  strategy_correlations: z.array(z.array(z.coerce.number())).optional().describe("Strategy return correlation matrix"),
  max_strategy_pct: z.coerce.number().describe("Maximum allocation to any single strategy"),
  max_vintage_pct: z.coerce.number().describe("Maximum allocation to any single vintage"),
  max_geography_pct: z.coerce.number().describe("Maximum allocation to any single geography"),
});
