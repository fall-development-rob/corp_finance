import { z } from "zod";

export const HModelDdmSchema = z.object({
  d0: z.coerce.number().describe("Current annual dividend per share"),
  r: z.coerce.number().describe("Required rate of return as decimal"),
  g_short: z.coerce.number().describe("Short-term (initial) growth rate as decimal"),
  g_long: z.coerce.number().describe("Long-term (terminal) growth rate as decimal"),
  half_life: z.coerce.number().describe("Half-life of growth decline in years"),
});

export const MultistageDdmSchema = z.object({
  d0: z.coerce.number().describe("Current annual dividend per share"),
  r: z.coerce.number().describe("Required rate of return as decimal"),
  stages: z.array(z.object({
    years: z.coerce.number().int().describe("Number of years in this growth stage"),
    growth_rate: z.coerce.number().describe("Growth rate for this stage as decimal"),
  })).describe("Growth stages with duration and rate"),
  terminal_growth: z.coerce.number().describe("Terminal perpetuity growth rate as decimal"),
});

export const BuybackAnalysisSchema = z.object({
  current_shares: z.coerce.number().describe("Current shares outstanding"),
  current_eps: z.coerce.number().describe("Current earnings per share"),
  current_price: z.coerce.number().describe("Current share price"),
  buyback_amount: z.coerce.number().describe("Total buyback amount"),
  cost_of_debt: z.coerce.number().describe("Cost of debt if debt-funded as decimal"),
  tax_rate: z.coerce.number().describe("Corporate tax rate as decimal"),
  dividend_tax_rate: z.coerce.number().describe("Dividend tax rate for shareholder as decimal"),
  capital_gains_tax_rate: z.coerce.number().describe("Capital gains tax rate for shareholder as decimal"),
  funding_source: z.string().describe("Funding source: 'cash', 'debt', or 'mixed'"),
});

export const PayoutSustainabilitySchema = z.object({
  eps: z.coerce.number().describe("Earnings per share"),
  dps: z.coerce.number().describe("Dividends per share"),
  fcf_per_share: z.coerce.number().describe("Free cash flow per share"),
  net_debt: z.coerce.number().describe("Net debt (total debt minus cash)"),
  ebitda: z.coerce.number().describe("EBITDA"),
  interest_expense: z.coerce.number().describe("Interest expense"),
  total_dividends: z.coerce.number().describe("Total dividend payments"),
  capex_required: z.coerce.number().describe("Required capital expenditure"),
  operating_cash_flow: z.coerce.number().describe("Operating cash flow"),
  target_payout_ratio: z.coerce.number().optional().describe("Target payout ratio as decimal (optional)"),
});

export const TotalShareholderReturnSchema = z.object({
  beginning_price: z.coerce.number().describe("Share price at beginning of period"),
  ending_price: z.coerce.number().describe("Share price at end of period"),
  dividends_received: z.coerce.number().describe("Total dividends received during period"),
  buyback_yield: z.coerce.number().describe("Buyback yield as decimal"),
  shares_beginning: z.coerce.number().describe("Shares outstanding at beginning"),
  shares_ending: z.coerce.number().describe("Shares outstanding at end"),
  holding_period_years: z.coerce.number().describe("Holding period in years"),
});
