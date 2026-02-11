import { z } from "zod";

export const MonetaryPolicySchema = z.object({
  current_inflation: z.coerce.number().describe("Current inflation rate (decimal, e.g. 0.03 for 3%)"),
  target_inflation: z.coerce.number().describe("Central bank inflation target (decimal, e.g. 0.02 for 2%)"),
  current_gdp_growth: z.coerce.number().describe("Current real GDP growth rate (decimal)"),
  potential_gdp_growth: z.coerce.number().describe("Potential (long-run) GDP growth rate (decimal)"),
  current_unemployment: z.coerce.number().describe("Current unemployment rate (decimal, e.g. 0.04 for 4%)"),
  natural_unemployment: z.coerce.number().describe("Natural (NAIRU) unemployment rate (decimal)"),
  current_policy_rate: z.coerce.number().describe("Current central bank policy rate (decimal)"),
  neutral_real_rate: z.coerce.number().describe("Neutral real interest rate (r-star) (decimal)"),
  inflation_weight: z.coerce.number().describe("Taylor rule weight on inflation gap (e.g. 0.5)"),
  output_weight: z.coerce.number().describe("Taylor rule weight on output gap (e.g. 0.5)"),
  historical_inflation: z.array(z.coerce.number()).describe("Historical inflation rates for trend analysis"),
  historical_unemployment: z.array(z.coerce.number()).describe("Historical unemployment rates for Phillips curve estimation"),
});

export const InternationalSchema = z.object({
  domestic_country: z.string().describe("Domestic country name or code"),
  foreign_country: z.string().describe("Foreign country name or code"),
  spot_exchange_rate: z.coerce.number().describe("Current spot exchange rate (domestic per foreign)"),
  domestic_inflation: z.coerce.number().describe("Domestic inflation rate (decimal)"),
  foreign_inflation: z.coerce.number().describe("Foreign inflation rate (decimal)"),
  domestic_interest_rate: z.coerce.number().describe("Domestic nominal interest rate (decimal)"),
  foreign_interest_rate: z.coerce.number().describe("Foreign nominal interest rate (decimal)"),
  domestic_gdp_growth: z.coerce.number().describe("Domestic real GDP growth rate (decimal)"),
  foreign_gdp_growth: z.coerce.number().describe("Foreign real GDP growth rate (decimal)"),
  forward_exchange_rate: z.coerce.number().optional().describe("Observed forward exchange rate (domestic per foreign)"),
  ppp_base_rate: z.coerce.number().optional().describe("PPP-implied base exchange rate for comparison"),
  current_account_pct_gdp: z.coerce.number().describe("Current account balance as percentage of GDP (decimal)"),
  years_forward: z.coerce.number().int().min(1).describe("Number of years forward for projections (integer, min 1)"),
});
