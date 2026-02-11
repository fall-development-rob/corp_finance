import { z } from "zod";

const FundInfoSchema = z.object({
  name: z.string().describe("Fund name"),
  nav: z.coerce.number().min(0).describe("Net asset value"),
  strategy: z.enum([
    "Equity",
    "FixedIncome",
    "EventDriven",
    "Credit",
    "Macro",
    "RelativeValue",
    "ManagedFutures",
    "MultiStrategy",
    "Other",
  ]).describe("AIFMD strategy classification"),
  domicile: z.string().describe("Fund domicile jurisdiction"),
  leverage_gross: z.coerce.number().min(0).describe("Gross leverage ratio"),
  leverage_commitment: z.coerce.number().min(0).describe("Commitment leverage ratio"),
  investor_count: z.coerce.number().int().min(0).describe("Number of investors"),
  largest_investor_pct: z.coerce.number().min(0).max(1).describe("Largest investor concentration (decimal)"),
  redemption_frequency: z.string().describe("Redemption frequency (e.g. Daily, Monthly, Quarterly)"),
  notice_period_days: z.coerce.number().int().min(0).describe("Redemption notice period in days"),
  has_gates: z.boolean().describe("Whether fund has redemption gates"),
  has_lockup: z.boolean().describe("Whether fund has a lockup period"),
  lockup_months: z.coerce.number().int().min(0).describe("Lockup period in months"),
  side_pocket_pct: z.coerce.number().min(0).max(1).describe("Side pocket percentage (decimal)"),
});

const CounterpartyExposureSchema = z.object({
  name: z.string().describe("Counterparty name"),
  exposure_pct: z.coerce.number().min(0).max(1).describe("Exposure as percentage of NAV (decimal)"),
});

const LiquidityProfileSchema = z.object({
  pct_1d: z.coerce.number().min(0).max(1).describe("Percentage of NAV redeemable within 1 day (decimal)"),
  pct_2_7d: z.coerce.number().min(0).max(1).describe("Percentage redeemable within 2-7 days (decimal)"),
  pct_8_30d: z.coerce.number().min(0).max(1).describe("Percentage redeemable within 8-30 days (decimal)"),
  pct_31_90d: z.coerce.number().min(0).max(1).describe("Percentage redeemable within 31-90 days (decimal)"),
  pct_91_180d: z.coerce.number().min(0).max(1).describe("Percentage redeemable within 91-180 days (decimal)"),
  pct_181_365d: z.coerce.number().min(0).max(1).describe("Percentage redeemable within 181-365 days (decimal)"),
  pct_over_365d: z.coerce.number().min(0).max(1).describe("Percentage redeemable over 365 days (decimal)"),
});

const MarketExposureSchema = z.object({
  market: z.string().describe("Market name"),
  pct: z.coerce.number().describe("Exposure percentage (decimal)"),
});

export const AifmdReportingSchema = z.object({
  aifm_name: z.string().describe("Alternative Investment Fund Manager name"),
  aifm_jurisdiction: z.string().describe("AIFM home jurisdiction"),
  total_aum: z.coerce.number().min(0).describe("Total AUM across all funds"),
  funds: z.array(FundInfoSchema).describe("List of AIFs managed"),
  reporting_period_end: z.string().describe("Reporting period end date (ISO format, e.g. '2025-12-31')"),
  long_exposures: z.coerce.number().min(0).describe("Total long market exposures"),
  short_exposures: z.coerce.number().min(0).describe("Total short market exposures"),
  top_counterparties: z.array(CounterpartyExposureSchema).describe("Top counterparty exposures"),
  liquidity_profile: LiquidityProfileSchema.describe("Aggregate portfolio liquidity profile"),
  principal_markets: z.array(MarketExposureSchema).describe("Principal markets with exposure percentages"),
  stress_equity_impact: z.coerce.number().describe("Equity stress test impact (decimal, e.g. -0.20)"),
  stress_rates_impact: z.coerce.number().describe("Interest rate stress test impact (decimal)"),
  stress_fx_impact: z.coerce.number().describe("FX stress test impact (decimal)"),
  stress_credit_impact: z.coerce.number().describe("Credit spread stress test impact (decimal)"),
});

const FormPfFundSchema = z.object({
  name: z.string().describe("Fund name"),
  nav: z.coerce.number().min(0).describe("Fund NAV"),
  gross_assets: z.coerce.number().min(0).describe("Fund gross asset value"),
  strategy: z.enum([
    "EquityLongShort",
    "EventDriven",
    "Macro",
    "RelativeValue",
    "Credit",
    "MultiStrategy",
    "ManagedFutures",
    "Other",
  ]).describe("Form PF strategy classification"),
  is_hedge_fund: z.boolean().describe("Whether the fund is a hedge fund"),
  is_pe_fund: z.boolean().describe("Whether the fund is a private equity fund"),
  is_liquidity_fund: z.boolean().describe("Whether the fund is a liquidity fund"),
  total_borrowings: z.coerce.number().min(0).describe("Total borrowings"),
  secured_borrowings: z.coerce.number().min(0).describe("Secured borrowings"),
  management_fee_rate: z.coerce.number().min(0).max(1).describe("Management fee rate (decimal)"),
  incentive_fee_rate: z.coerce.number().min(0).max(1).describe("Incentive/performance fee rate (decimal)"),
  high_water_mark: z.boolean().describe("Whether a high water mark applies"),
  monthly_returns: z.array(z.coerce.number()).describe("Monthly returns for the reporting period (decimal)"),
  us_investor_pct: z.coerce.number().min(0).max(1).describe("Percentage of AUM from US investors (decimal)"),
  institutional_pct: z.coerce.number().min(0).max(1).describe("Percentage of AUM from institutional investors (decimal)"),
});

const CounterpartyInfoSchema = z.object({
  name: z.string().describe("Counterparty name"),
  exposure: z.coerce.number().min(0).describe("Exposure amount"),
  secured_pct: z.coerce.number().min(0).max(1).describe("Percentage of exposure that is secured (decimal)"),
});

export const SecCftcReportingSchema = z.object({
  adviser_name: z.string().describe("Registered investment adviser name"),
  sec_registered: z.boolean().describe("Whether the adviser is SEC-registered"),
  nfa_registered: z.boolean().describe("Whether the adviser is NFA-registered"),
  total_regulatory_aum: z.coerce.number().min(0).describe("Total regulatory AUM"),
  fund_count: z.coerce.number().int().min(0).describe("Number of funds advised"),
  funds: z.array(FormPfFundSchema).describe("Fund-level details for Form PF reporting"),
  fiscal_year_end: z.string().describe("Fiscal year end date (ISO format)"),
  reporting_date: z.string().describe("Reporting date (ISO format)"),
  counterparties: z.array(CounterpartyInfoSchema).describe("Top counterparty exposures"),
  otc_bilateral_pct: z.coerce.number().min(0).max(1).describe("OTC bilateral derivative percentage (decimal)"),
  otc_cleared_pct: z.coerce.number().min(0).max(1).describe("OTC cleared derivative percentage (decimal)"),
  exchange_traded_pct: z.coerce.number().min(0).max(1).describe("Exchange-traded derivative percentage (decimal)"),
  commodity_pool: z.boolean().describe("Whether any fund is a commodity pool under CFTC rules"),
});
