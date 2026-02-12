import { z } from "zod";

const TipsPricingInputSchema = z.object({
  face_value: z.coerce.number().describe("Face/par value"),
  real_coupon_rate: z.coerce.number().describe("Real coupon rate as decimal"),
  coupon_frequency: z.coerce.number().int().describe("Coupons per year (1, 2, 4)"),
  real_yield: z.coerce.number().describe("Real yield to maturity"),
  settlement_date: z.string().describe("Settlement date (YYYY-MM-DD)"),
  maturity_date: z.string().describe("Maturity date (YYYY-MM-DD)"),
  cpi_base: z.coerce.number().describe("Base CPI at issue"),
  cpi_current: z.coerce.number().describe("Current CPI"),
  cpi_projected_annual_rate: z.coerce.number().describe("Projected annual inflation rate"),
  remaining_periods: z.coerce.number().int().describe("Remaining coupon periods"),
});

const YieldPointSchema = z.object({
  maturity: z.coerce.number().describe("Maturity in years"),
  rate: z.coerce.number().describe("Yield as decimal"),
});

const BreakevenInputSchema = z.object({
  nominal_yield: z.coerce.number().describe("Nominal Treasury yield"),
  real_yield: z.coerce.number().describe("TIPS real yield"),
  nominal_yield_curve: z.array(YieldPointSchema).describe("Nominal yield curve"),
  real_yield_curve: z.array(YieldPointSchema).describe("Real yield curve"),
});

const TipsSecuritySchema = z.object({
  maturity: z.coerce.number().describe("Maturity in years"),
  real_yield: z.coerce.number().describe("Real yield"),
  nominal_yield: z.coerce.number().describe("Nominal yield"),
  cpi_ratio: z.coerce.number().describe("CPI index ratio"),
});

const RealYieldInputSchema = z.object({
  tips_securities: z.array(TipsSecuritySchema).describe("TIPS securities data"),
});

export const TipsAnalyticsSchema = z.object({
  model: z.discriminatedUnion("type", [
    z.object({ type: z.literal("Pricing"), ...TipsPricingInputSchema.shape }),
    z.object({ type: z.literal("Breakeven"), ...BreakevenInputSchema.shape }),
    z.object({ type: z.literal("RealYield"), ...RealYieldInputSchema.shape }),
  ]).describe("TIPS analytics model selection"),
});

const ZcisInputSchema = z.object({
  notional: z.coerce.number().describe("Notional amount"),
  maturity_years: z.coerce.number().describe("Swap maturity in years"),
  cpi_base: z.coerce.number().describe("Base CPI"),
  cpi_current: z.coerce.number().describe("Current CPI"),
  expected_inflation: z.coerce.number().describe("Expected annual inflation rate"),
  real_discount_rate: z.coerce.number().describe("Real discount rate"),
  nominal_discount_rate: z.coerce.number().describe("Nominal discount rate"),
});

const YyisInputSchema = z.object({
  notional: z.coerce.number().describe("Notional amount"),
  num_periods: z.coerce.number().int().describe("Number of periods"),
  payment_frequency: z.coerce.number().int().describe("Payments per year"),
  cpi_base: z.coerce.number().describe("Base CPI"),
  expected_inflation_curve: z.array(z.coerce.number()).describe("Expected inflation rates per period"),
  real_discount_curve: z.array(z.coerce.number()).describe("Real discount rates per period"),
  nominal_discount_curve: z.array(z.coerce.number()).describe("Nominal discount rates per period"),
});

const InflationCapFloorInputSchema = z.object({
  notional: z.coerce.number().describe("Notional amount"),
  strike_rate: z.coerce.number().describe("Strike inflation rate"),
  option_type: z.enum(["Cap", "Floor"]).describe("Cap or Floor"),
  num_periods: z.coerce.number().int().describe("Number of periods"),
  expected_inflation_curve: z.array(z.coerce.number()).describe("Expected inflation per period"),
  inflation_vol: z.coerce.number().describe("Inflation volatility"),
  discount_curve: z.array(z.coerce.number()).describe("Discount rates per period"),
});

export const InflationDerivativeSchema = z.object({
  model: z.discriminatedUnion("type", [
    z.object({ type: z.literal("Zcis"), ...ZcisInputSchema.shape }),
    z.object({ type: z.literal("Yyis"), ...YyisInputSchema.shape }),
    z.object({ type: z.literal("CapFloor"), ...InflationCapFloorInputSchema.shape }),
  ]).describe("Inflation derivative model selection"),
});
