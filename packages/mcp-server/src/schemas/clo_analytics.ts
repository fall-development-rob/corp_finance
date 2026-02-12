import { z } from "zod";

export const CloWaterfallSchema = z.object({
  tranches: z.array(z.object({
    name: z.string().describe("Tranche name (e.g. AAA, AA, Equity)"),
    rating: z.string().optional().describe("Credit rating"),
    notional: z.coerce.number().describe("Tranche notional amount"),
    spread: z.coerce.number().describe("Tranche spread in bps"),
    is_equity: z.boolean().describe("Whether this is the equity tranche"),
  })).describe("CLO tranche structure"),
  pool_balance: z.coerce.number().describe("Total collateral pool balance"),
  weighted_avg_spread: z.coerce.number().describe("Weighted average spread of assets in bps"),
  cdr: z.coerce.number().describe("Constant default rate as decimal"),
  cpr: z.coerce.number().describe("Constant prepayment rate as decimal"),
  recovery_rate: z.coerce.number().describe("Recovery rate on defaults as decimal"),
  recovery_lag_months: z.coerce.number().int().describe("Recovery lag in months"),
  reference_rate: z.coerce.number().describe("Reference rate (e.g. SOFR) as decimal"),
  num_periods: z.coerce.number().int().describe("Number of periods to project"),
  period_days: z.coerce.number().int().describe("Days per period (e.g. 90 for quarterly)"),
  senior_fees_bps: z.coerce.number().describe("Senior fees in basis points"),
});

export const CloCoverageTestsSchema = z.object({
  tranches: z.array(z.object({
    name: z.string().describe("Tranche name"),
    notional: z.coerce.number().describe("Tranche notional"),
    spread: z.coerce.number().describe("Tranche spread in bps"),
    oc_trigger: z.coerce.number().describe("OC test trigger level"),
    ic_trigger: z.coerce.number().describe("IC test trigger level"),
  })).describe("Tranches with coverage test triggers"),
  pool_par: z.coerce.number().describe("Current pool par balance"),
  defaulted_par: z.coerce.number().describe("Defaulted assets par balance"),
  interest_income: z.coerce.number().describe("Periodic interest income from pool"),
  senior_fees: z.coerce.number().describe("Senior fees amount"),
  reference_rate: z.coerce.number().describe("Reference rate as decimal"),
});

export const CloReinvestmentSchema = z.object({
  assets: z.array(z.object({
    name: z.string().describe("Asset identifier"),
    notional: z.coerce.number().describe("Asset notional"),
    rating: z.string().describe("Asset rating (e.g. B, BB)"),
    spread: z.coerce.number().describe("Asset spread in bps"),
    remaining_life: z.coerce.number().describe("Remaining life in years"),
    industry: z.string().describe("Industry classification"),
  })).describe("Current portfolio assets"),
  target_par: z.coerce.number().describe("Target par balance to maintain"),
  max_warf: z.coerce.number().describe("Maximum weighted average rating factor"),
  min_wals: z.coerce.number().describe("Minimum weighted average life (short)"),
  max_wal: z.coerce.number().describe("Maximum weighted average life"),
  min_diversity_score: z.coerce.number().describe("Minimum diversity score"),
});

export const CloTrancheAnalyticsSchema = z.object({
  tranche_name: z.string().describe("Tranche identifier"),
  cash_flows: z.array(z.object({
    period: z.coerce.number().int().describe("Period number"),
    interest: z.coerce.number().describe("Interest payment"),
    principal: z.coerce.number().describe("Principal payment"),
  })).describe("Projected tranche cash flows"),
  initial_investment: z.coerce.number().describe("Initial investment amount"),
  price: z.coerce.number().describe("Purchase price as fraction of par"),
  call_date_period: z.coerce.number().int().optional().describe("Optional call date period for yield-to-call"),
  reference_rate: z.coerce.number().describe("Reference rate as decimal"),
});

export const CloScenarioSchema = z.object({
  tranches: z.array(z.object({
    name: z.string().describe("Tranche name"),
    notional: z.coerce.number().describe("Tranche notional"),
    spread: z.coerce.number().describe("Tranche spread in bps"),
    is_equity: z.boolean().describe("Whether this is equity tranche"),
  })).describe("CLO tranche structure"),
  pool_balance: z.coerce.number().describe("Total pool balance"),
  weighted_avg_spread: z.coerce.number().describe("Weighted average spread in bps"),
  reference_rate: z.coerce.number().describe("Reference rate as decimal"),
  scenarios: z.array(z.object({
    name: z.string().describe("Scenario name"),
    cdr: z.coerce.number().describe("Default rate for scenario"),
    cpr: z.coerce.number().describe("Prepayment rate for scenario"),
    recovery: z.coerce.number().describe("Recovery rate for scenario"),
    probability: z.coerce.number().describe("Scenario probability weight"),
  })).describe("Stress scenarios to evaluate"),
  num_periods: z.coerce.number().int().describe("Number of periods to project"),
});
