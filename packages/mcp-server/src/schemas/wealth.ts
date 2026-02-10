import { z } from "zod";

export const RetirementSchema = z.object({
  current_age: z.number().int().positive().describe("Current age"),
  retirement_age: z.number().int().positive().describe("Target retirement age"),
  life_expectancy: z.number().int().positive().describe("Life expectancy"),
  current_savings: z.number().min(0).describe("Current retirement savings"),
  annual_income: z.number().positive().describe("Current annual income"),
  annual_savings: z.number().min(0).describe("Annual savings contribution"),
  savings_growth_rate: z.number().describe("Annual growth rate of savings contributions"),
  pre_retirement_return: z.number().describe("Expected annual return pre-retirement"),
  post_retirement_return: z.number().describe("Expected annual return post-retirement"),
  inflation_rate: z.number().min(0).describe("Expected annual inflation rate"),
  desired_replacement_ratio: z.number().min(0).max(1).describe("Target income replacement ratio (e.g. 0.80)"),
  social_security_annual: z.number().min(0).describe("Expected annual Social Security benefit"),
  withdrawal_strategy: z.union([
    z.literal("ConstantDollar"),
    z.object({ ConstantPercentage: z.number().min(0).max(1) }),
    z.object({
      GuardrailsPercent: z.object({
        initial_pct: z.number().min(0).max(1),
        floor_pct: z.number().min(0).max(1),
        ceiling_pct: z.number().min(0).max(1),
      }),
    }),
    z.literal("Rmd"),
  ]).describe("Withdrawal strategy"),
  tax_rate_retirement: z.number().min(0).max(1).describe("Effective tax rate in retirement"),
});

export const TlhSchema = z.object({
  portfolio_value: z.number().positive().describe("Total portfolio value"),
  positions: z.array(z.object({
    ticker: z.string().describe("Ticker symbol"),
    market_value: z.number().positive().describe("Current market value"),
    cost_basis: z.number().positive().describe("Cost basis"),
    holding_period_days: z.number().int().min(0).describe("Days held"),
    unrealized_gain_loss: z.number().describe("Unrealized gain/loss (negative = loss)"),
  })).describe("Portfolio positions"),
  short_term_tax_rate: z.number().min(0).max(1).describe("Short-term capital gains tax rate"),
  long_term_tax_rate: z.number().min(0).max(1).describe("Long-term capital gains tax rate"),
  annual_capital_gains: z.number().min(0).describe("Annual capital gains to offset"),
  harvest_threshold_pct: z.number().min(0).max(1).describe("Minimum loss % to consider harvesting"),
  wash_sale_days: z.number().int().min(0).describe("Wash sale rule days (typically 30)"),
});

export const EstatePlanSchema = z.object({
  total_estate_value: z.number().positive().describe("Total estate value"),
  annual_gifting: z.array(z.object({
    recipient_name: z.string().describe("Recipient name"),
    annual_amount: z.number().positive().describe("Annual gift amount"),
    is_skip_person: z.boolean().describe("Whether recipient is a skip person (GST)"),
    years_of_gifting: z.number().int().positive().describe("Years of planned gifting"),
  })).describe("Gifting plan"),
  estate_tax_exemption: z.number().min(0).describe("Federal estate tax exemption"),
  gift_tax_annual_exclusion: z.number().min(0).describe("Annual gift tax exclusion per recipient"),
  estate_tax_rate: z.number().min(0).max(1).describe("Federal estate tax rate"),
  state_estate_tax_rate: z.number().min(0).max(1).optional().describe("State estate tax rate"),
  state_exemption: z.number().min(0).optional().describe("State estate tax exemption"),
  gst_tax_rate: z.number().min(0).max(1).describe("Generation-skipping transfer tax rate"),
  gst_exemption: z.number().min(0).describe("GST tax exemption"),
  trust_structures: z.array(z.object({
    name: z.string().describe("Trust name"),
    trust_type: z.enum(["Revocable", "Irrevocable", "Grat", "Ilit", "Qprt", "CrummeyTrust", "CharitableRemainder"]).describe("Trust type"),
    funded_amount: z.number().min(0).describe("Amount funded into trust"),
    annual_distribution: z.number().min(0).describe("Annual distribution from trust"),
    expected_return: z.number().describe("Expected annual return on trust assets"),
  })).describe("Trust structures"),
  charitable_bequests: z.number().min(0).describe("Charitable bequests amount"),
  marital_deduction: z.number().min(0).describe("Marital deduction amount"),
  life_insurance_proceeds: z.number().min(0).describe("Life insurance proceeds"),
  planning_horizon_years: z.number().int().positive().describe("Planning horizon in years"),
});
