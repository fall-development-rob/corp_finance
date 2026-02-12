import { z } from "zod";

export const CarbonCreditPricingSchema = z.object({
  spot_price: z.coerce.number().describe("Current spot price per tonne CO2e"),
  risk_free_rate: z.coerce.number().describe("Risk-free rate as decimal"),
  storage_cost: z.coerce.number().describe("Storage/holding cost as decimal"),
  convenience_yield: z.coerce.number().describe("Convenience yield as decimal"),
  time_to_delivery: z.coerce.number().describe("Time to delivery in years"),
  vintage_year: z.coerce.number().int().describe("Credit vintage year"),
  current_year: z.coerce.number().int().describe("Current year"),
  registry: z.string().describe("Registry name (e.g. 'VCS', 'Gold Standard', 'ACR')"),
  credit_type: z.string().describe("Credit type (e.g. 'removal', 'avoidance', 'reduction')"),
});

export const EtsComplianceSchema = z.object({
  verified_emissions: z.coerce.number().describe("Verified emissions in tonnes CO2e"),
  free_allowances: z.coerce.number().describe("Free allowances received"),
  purchased_allowances: z.coerce.number().describe("Purchased allowances"),
  allowance_price: z.coerce.number().describe("Current allowance price per tonne"),
  historical_prices: z.array(z.coerce.number()).describe("Historical allowance prices for volatility calculation"),
  compliance_deadline_days: z.coerce.number().int().describe("Days until compliance deadline"),
  benchmark_emission_factor: z.coerce.number().describe("Industry benchmark emission factor"),
  actual_emission_factor: z.coerce.number().describe("Company actual emission factor"),
});

export const CbamAnalysisSchema = z.object({
  imported_goods: z.array(z.object({
    product: z.string().describe("Product description"),
    quantity_tonnes: z.coerce.number().describe("Quantity in tonnes"),
    embedded_emissions: z.coerce.number().describe("Embedded emissions per tonne CO2e"),
    origin_country: z.string().describe("Country of origin"),
    origin_carbon_price: z.coerce.number().describe("Carbon price paid in origin country per tonne"),
  })).describe("Imported goods subject to CBAM"),
  eu_ets_price: z.coerce.number().describe("Current EU ETS allowance price per tonne"),
  eu_free_allocation_pct: z.coerce.number().describe("EU free allocation percentage as decimal"),
});

export const OffsetValuationSchema = z.object({
  base_price: z.coerce.number().describe("Base market price per tonne CO2e"),
  credit_type: z.string().describe("Offset credit type (e.g. 'forestry', 'renewable', 'methane')"),
  permanence_years: z.coerce.number().describe("Expected permanence duration in years"),
  additionality_score: z.coerce.number().describe("Additionality score (0-1)"),
  vintage_year: z.coerce.number().int().describe("Credit vintage year"),
  current_year: z.coerce.number().int().describe("Current year"),
  certification: z.string().describe("Certification standard (e.g. 'VCS', 'Gold Standard', 'CAR')"),
  co_benefits: z.array(z.string()).describe("Co-benefits (e.g. 'biodiversity', 'community', 'water')"),
  reversal_risk: z.coerce.number().describe("Reversal/permanence risk as decimal (0-1)"),
});

export const ShadowCarbonPriceSchema = z.object({
  projects: z.array(z.object({
    name: z.string().describe("Project name"),
    capex: z.coerce.number().describe("Capital expenditure"),
    annual_cash_flows: z.array(z.coerce.number()).describe("Annual cash flows over project life"),
    annual_emissions: z.array(z.coerce.number()).describe("Annual emissions (positive=emitting, negative=reducing)"),
    project_life: z.coerce.number().int().describe("Project life in years"),
  })).describe("Projects to evaluate under shadow carbon pricing"),
  carbon_price: z.coerce.number().describe("Shadow carbon price per tonne CO2e"),
  discount_rate: z.coerce.number().describe("Discount rate as decimal"),
  carbon_price_escalation: z.coerce.number().describe("Annual carbon price escalation rate as decimal"),
});
