import { z } from "zod";

export const SovereignBondSchema = z.object({
  face_value: z.coerce.number(),
  coupon_rate: z.coerce.number(),
  maturity_years: z.coerce.number(),
  payment_frequency: z.coerce.number(),
  risk_free_rate: z.coerce.number(),
  sovereign_spread: z.coerce.number(),
  currency: z.string(),
  country: z.string(),
  is_local_currency: z.boolean(),
  inflation_rate: z.coerce.number().optional(),
  fx_volatility: z.coerce.number().optional(),
});

export const CountryRiskSchema = z.object({
  country: z.string(),
  gdp_growth_rate: z.coerce.number(),
  inflation_rate: z.coerce.number(),
  fiscal_balance_pct_gdp: z.coerce.number(),
  debt_to_gdp: z.coerce.number(),
  current_account_pct_gdp: z.coerce.number(),
  fx_reserves_months_imports: z.coerce.number(),
  political_stability_score: z.coerce.number(),
  rule_of_law_score: z.coerce.number(),
  external_debt_to_gdp: z.coerce.number(),
  short_term_debt_to_reserves: z.coerce.number(),
  sovereign_default_history: z.boolean(),
  dollarization_pct: z.coerce.number().optional(),
});
