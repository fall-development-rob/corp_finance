import { z } from "zod";

export const ConcentratedStockSchema = z.object({
  position_value: z.coerce.number().describe("Current market value of the concentrated stock position"),
  cost_basis: z.coerce.number().describe("Original cost basis of the position"),
  annual_dividend_yield: z.coerce.number().describe("Annual dividend yield as decimal"),
  stock_volatility: z.coerce.number().describe("Annualized stock volatility as decimal"),
  risk_free_rate: z.coerce.number().describe("Risk-free rate as decimal"),
  investment_horizon: z.coerce.number().int().describe("Investment horizon in years"),
  tax_rate_ltcg: z.coerce.number().describe("Long-term capital gains tax rate as decimal"),
  tax_rate_stcg: z.coerce.number().describe("Short-term capital gains tax rate as decimal"),
  collar_put_strike_pct: z.coerce.number().describe("Collar put strike as percentage of current price"),
  collar_call_strike_pct: z.coerce.number().describe("Collar call strike as percentage of current price"),
  exchange_fund_diversification_pct: z.coerce.number().describe("Exchange fund diversification percentage"),
  prepaid_forward_advance_pct: z.coerce.number().describe("Prepaid variable forward advance rate percentage"),
});

export const PhilanthropicVehiclesSchema = z.object({
  donation_amount: z.coerce.number().describe("Total donation amount"),
  donor_income: z.coerce.number().describe("Donor annual income"),
  donor_tax_rate: z.coerce.number().describe("Donor marginal tax rate as decimal"),
  appreciated_asset_fmv: z.coerce.number().describe("Fair market value of appreciated asset"),
  appreciated_asset_basis: z.coerce.number().describe("Cost basis of appreciated asset"),
  payout_rate: z.coerce.number().describe("Trust payout rate as decimal (for CRT/CLT)"),
  trust_term_years: z.coerce.number().int().describe("Trust term in years"),
  discount_rate: z.coerce.number().describe("Discount rate for present value calculations"),
  donor_age: z.coerce.number().int().describe("Donor age in years"),
});

export const WealthTransferSchema = z.object({
  estate_value: z.coerce.number().describe("Total estate value"),
  annual_exclusion: z.coerce.number().describe("Annual gift tax exclusion amount per donee"),
  lifetime_exemption: z.coerce.number().describe("Lifetime estate/gift tax exemption amount"),
  estate_tax_rate: z.coerce.number().describe("Estate tax rate as decimal"),
  gst_tax_rate: z.coerce.number().describe("Generation-skipping transfer tax rate as decimal"),
  num_beneficiaries: z.coerce.number().int().describe("Number of beneficiaries"),
  transfer_years: z.coerce.number().int().describe("Number of years for transfer planning"),
  asset_growth_rate: z.coerce.number().describe("Expected annual asset growth rate as decimal"),
  grantor_trust_assets: z.coerce.number().describe("Assets in grantor trusts"),
  grat_annuity_rate: z.coerce.number().describe("GRAT annuity rate as decimal"),
  section_7520_rate: z.coerce.number().describe("IRS Section 7520 interest rate"),
});

export const DirectIndexingSchema = z.object({
  portfolio_value: z.coerce.number().describe("Total portfolio market value"),
  holdings: z.array(z.object({
    ticker: z.string().describe("Stock ticker symbol"),
    weight: z.coerce.number().describe("Current portfolio weight as decimal"),
    cost_basis: z.coerce.number().describe("Cost basis of the holding"),
    current_value: z.coerce.number().describe("Current market value of the holding"),
    holding_period_days: z.coerce.number().int().describe("Holding period in days"),
  })).describe("Array of individual stock holdings"),
  benchmark_return: z.coerce.number().describe("Benchmark return for tracking error calculation"),
  tax_rate_ltcg: z.coerce.number().describe("Long-term capital gains tax rate as decimal"),
  tax_rate_stcg: z.coerce.number().describe("Short-term capital gains tax rate as decimal"),
  wash_sale_window: z.coerce.number().int().describe("Wash sale window in days (typically 30)"),
  tracking_error_budget: z.coerce.number().describe("Maximum tolerable tracking error as decimal"),
});

export const FamilyGovernanceSchema = z.object({
  family_members: z.coerce.number().int().describe("Total number of family members involved"),
  generations_active: z.coerce.number().int().describe("Number of active generations"),
  has_family_constitution: z.boolean().describe("Whether a family constitution/charter exists"),
  has_investment_committee: z.boolean().describe("Whether a formal investment committee exists"),
  has_succession_plan: z.boolean().describe("Whether a succession plan is in place"),
  has_conflict_resolution: z.boolean().describe("Whether conflict resolution mechanisms exist"),
  has_next_gen_education: z.boolean().describe("Whether next-generation education programs exist"),
  has_external_advisors: z.boolean().describe("Whether external advisors are engaged"),
  has_regular_meetings: z.boolean().describe("Whether regular family meetings are held"),
  has_philanthropy_program: z.boolean().describe("Whether a philanthropy program exists"),
  total_aum: z.coerce.number().describe("Total assets under management"),
  num_investment_vehicles: z.coerce.number().int().describe("Number of investment vehicles/entities"),
  reporting_frequency: z.string().describe("Reporting frequency (monthly/quarterly/annually)"),
});
