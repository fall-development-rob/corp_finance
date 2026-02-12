import { z } from "zod";

export const CountryRiskPremiumSchema = z.object({
  sovereign_spread_bps: z.coerce.number().describe("Sovereign CDS spread in basis points"),
  equity_vol_local: z.coerce.number().describe("Local equity market annualized volatility"),
  bond_vol_local: z.coerce.number().describe("Local bond market annualized volatility"),
  us_equity_risk_premium: z.coerce.number().describe("US equity risk premium as decimal"),
  country_rating: z.string().describe("Sovereign credit rating (e.g., BBB+, BB-, etc.)"),
  gdp_growth: z.coerce.number().describe("Expected GDP growth rate as decimal"),
  inflation_rate: z.coerce.number().describe("Current inflation rate as decimal"),
  fx_volatility: z.coerce.number().describe("FX volatility against USD as decimal"),
  governance_score: z.coerce.number().describe("Governance quality score (0-1 scale)"),
  risk_free_rate: z.coerce.number().describe("Risk-free rate as decimal"),
});

export const PoliticalRiskSchema = z.object({
  country: z.string().describe("Country name or ISO code"),
  political_stability: z.coerce.number().describe("Political stability score (-2.5 to 2.5, WGI scale)"),
  regulatory_quality: z.coerce.number().describe("Regulatory quality score (-2.5 to 2.5)"),
  rule_of_law: z.coerce.number().describe("Rule of law score (-2.5 to 2.5)"),
  control_of_corruption: z.coerce.number().describe("Control of corruption score (-2.5 to 2.5)"),
  voice_accountability: z.coerce.number().describe("Voice and accountability score (-2.5 to 2.5)"),
  government_effectiveness: z.coerce.number().describe("Government effectiveness score (-2.5 to 2.5)"),
  expropriation_history: z.boolean().describe("Whether country has history of expropriation"),
  sanctions_risk: z.boolean().describe("Whether country is subject to sanctions risk"),
  conflict_zone: z.boolean().describe("Whether country is in or near a conflict zone"),
  investment_amount: z.coerce.number().describe("Amount of planned investment"),
  insurance_premium_rate: z.coerce.number().describe("Political risk insurance premium rate as decimal"),
});

export const CapitalControlsSchema = z.object({
  country: z.string().describe("Country name or ISO code"),
  control_type: z.string().describe("Type of capital control (e.g., repatriation, conversion, transfer)"),
  repatriation_delay_days: z.coerce.number().int().describe("Expected repatriation delay in days"),
  withholding_tax_dividends: z.coerce.number().describe("Withholding tax rate on dividends as decimal"),
  withholding_tax_interest: z.coerce.number().describe("Withholding tax rate on interest as decimal"),
  withholding_tax_royalties: z.coerce.number().describe("Withholding tax rate on royalties as decimal"),
  fx_conversion_spread: z.coerce.number().describe("FX conversion spread/cost as decimal"),
  investment_amount: z.coerce.number().describe("Total investment amount"),
  expected_annual_income: z.coerce.number().describe("Expected annual income from investment"),
  holding_period_years: z.coerce.number().int().describe("Planned holding period in years"),
  risk_free_rate: z.coerce.number().describe("Risk-free rate as decimal"),
});

export const EmBondAnalysisSchema = z.object({
  local_currency_yield: z.coerce.number().describe("Local currency bond yield as decimal"),
  hard_currency_yield: z.coerce.number().describe("Hard currency (USD) bond yield as decimal"),
  spot_fx_rate: z.coerce.number().describe("Current spot FX rate (local per USD)"),
  forward_fx_rate: z.coerce.number().describe("Forward FX rate (local per USD)"),
  local_inflation: z.coerce.number().describe("Local inflation rate as decimal"),
  us_inflation: z.coerce.number().describe("US inflation rate as decimal"),
  sovereign_spread: z.coerce.number().describe("Sovereign credit spread as decimal"),
  local_bond_duration: z.coerce.number().describe("Duration of local currency bond in years"),
  hard_bond_duration: z.coerce.number().describe("Duration of hard currency bond in years"),
  fx_volatility: z.coerce.number().describe("FX volatility as decimal"),
  investment_amount: z.coerce.number().describe("Investment amount"),
  hedging_cost: z.coerce.number().describe("FX hedging cost as decimal"),
});

export const EmEquityPremiumSchema = z.object({
  local_market_return: z.coerce.number().describe("Local equity market expected return as decimal"),
  us_market_return: z.coerce.number().describe("US equity market expected return as decimal"),
  risk_free_rate: z.coerce.number().describe("Risk-free rate as decimal"),
  sovereign_spread: z.coerce.number().describe("Sovereign credit spread as decimal"),
  equity_vol_local: z.coerce.number().describe("Local equity market volatility as decimal"),
  equity_vol_us: z.coerce.number().describe("US equity market volatility as decimal"),
  bond_vol_local: z.coerce.number().describe("Local bond market volatility as decimal"),
  market_cap_to_gdp: z.coerce.number().describe("Market capitalization to GDP ratio"),
  pe_ratio: z.coerce.number().describe("Local market price-to-earnings ratio"),
  dividend_yield: z.coerce.number().describe("Local market dividend yield as decimal"),
  gdp_growth: z.coerce.number().describe("Expected GDP growth rate as decimal"),
  earnings_growth: z.coerce.number().describe("Expected earnings growth rate as decimal"),
  fx_volatility: z.coerce.number().describe("FX volatility against USD as decimal"),
});
