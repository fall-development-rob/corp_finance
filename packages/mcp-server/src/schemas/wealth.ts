import { z } from "zod";

export const RetirementSchema = z.object({
  current_age: z.coerce.number().int().positive().describe("Current age"),
  retirement_age: z.coerce.number().int().positive().describe("Target retirement age"),
  life_expectancy: z.coerce.number().int().positive().describe("Life expectancy"),
  current_savings: z.coerce.number().min(0).describe("Current retirement savings"),
  annual_income: z.coerce.number().positive().describe("Current annual income"),
  annual_savings: z.coerce.number().min(0).describe("Annual savings contribution"),
  savings_growth_rate: z.coerce.number().describe("Annual growth rate of savings contributions"),
  pre_retirement_return: z.coerce.number().describe("Expected annual return pre-retirement"),
  post_retirement_return: z.coerce.number().describe("Expected annual return post-retirement"),
  inflation_rate: z.coerce.number().min(0).describe("Expected annual inflation rate"),
  desired_replacement_ratio: z.coerce.number().min(0).max(1).describe("Target income replacement ratio (e.g. 0.80)"),
  social_security_annual: z.coerce.number().min(0).describe("Expected annual Social Security benefit"),
  withdrawal_strategy: z.union([
    z.literal("ConstantDollar"),
    z.object({ ConstantPercentage: z.coerce.number().min(0).max(1) }),
    z.object({
      GuardrailsPercent: z.object({
        initial_pct: z.coerce.number().min(0).max(1),
        floor_pct: z.coerce.number().min(0).max(1),
        ceiling_pct: z.coerce.number().min(0).max(1),
      }),
    }),
    z.literal("Rmd"),
  ]).describe("Withdrawal strategy"),
  tax_rate_retirement: z.coerce.number().min(0).max(1).describe("Effective tax rate in retirement"),
});

export const TlhSchema = z.object({
  portfolio_value: z.coerce.number().positive().describe("Total portfolio value"),
  positions: z.array(z.object({
    ticker: z.string().describe("Ticker symbol"),
    market_value: z.coerce.number().positive().describe("Current market value"),
    cost_basis: z.coerce.number().positive().describe("Cost basis"),
    holding_period_days: z.coerce.number().int().min(0).describe("Days held"),
    unrealized_gain_loss: z.coerce.number().describe("Unrealized gain/loss (negative = loss)"),
  })).describe("Portfolio positions"),
  short_term_tax_rate: z.coerce.number().min(0).max(1).describe("Short-term capital gains tax rate"),
  long_term_tax_rate: z.coerce.number().min(0).max(1).describe("Long-term capital gains tax rate"),
  annual_capital_gains: z.coerce.number().min(0).describe("Annual capital gains to offset"),
  harvest_threshold_pct: z.coerce.number().min(0).max(1).describe("Minimum loss % to consider harvesting"),
  wash_sale_days: z.coerce.number().int().min(0).describe("Wash sale rule days (typically 30)"),
});

export const EstatePlanSchema = z.object({
  total_estate_value: z.coerce.number().positive().describe("Total estate value"),
  annual_gifting: z.array(z.object({
    recipient_name: z.string().describe("Recipient name"),
    annual_amount: z.coerce.number().positive().describe("Annual gift amount"),
    is_skip_person: z.coerce.boolean().describe("Whether recipient is a skip person (GST)"),
    years_of_gifting: z.coerce.number().int().positive().describe("Years of planned gifting"),
  })).describe("Gifting plan"),
  estate_tax_exemption: z.coerce.number().min(0).describe("Federal estate tax exemption"),
  gift_tax_annual_exclusion: z.coerce.number().min(0).describe("Annual gift tax exclusion per recipient"),
  estate_tax_rate: z.coerce.number().min(0).max(1).describe("Federal estate tax rate"),
  state_estate_tax_rate: z.coerce.number().min(0).max(1).optional().describe("State estate tax rate"),
  state_exemption: z.coerce.number().min(0).optional().describe("State estate tax exemption"),
  gst_tax_rate: z.coerce.number().min(0).max(1).describe("Generation-skipping transfer tax rate"),
  gst_exemption: z.coerce.number().min(0).describe("GST tax exemption"),
  trust_structures: z.array(z.object({
    name: z.string().describe("Trust name"),
    trust_type: z.enum(["Revocable", "Irrevocable", "Grat", "Ilit", "Qprt", "CrummeyTrust", "CharitableRemainder"]).describe("Trust type"),
    funded_amount: z.coerce.number().min(0).describe("Amount funded into trust"),
    annual_distribution: z.coerce.number().min(0).describe("Annual distribution from trust"),
    expected_return: z.coerce.number().describe("Expected annual return on trust assets"),
  })).describe("Trust structures"),
  charitable_bequests: z.coerce.number().min(0).describe("Charitable bequests amount"),
  marital_deduction: z.coerce.number().min(0).describe("Marital deduction amount"),
  life_insurance_proceeds: z.coerce.number().min(0).describe("Life insurance proceeds"),
  planning_horizon_years: z.coerce.number().int().positive().describe("Planning horizon in years"),
});
