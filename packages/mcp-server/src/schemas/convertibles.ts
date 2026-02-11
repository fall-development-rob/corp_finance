import { z } from "zod";

export const ConvertiblePricingSchema = z.object({
  bond_name: z.string().describe("Bond name / identifier"),
  face_value: z.coerce.number().positive().describe("Face value of the bond"),
  coupon_rate: z.coerce.number().min(0).describe("Annual coupon rate"),
  coupon_frequency: z.coerce.number().int().positive().describe("Coupon payments per year"),
  maturity_years: z.coerce.number().positive().describe("Years to maturity"),
  credit_spread: z.coerce.number().min(0).describe("Credit spread over risk-free"),
  risk_free_rate: z.coerce.number().describe("Risk-free rate"),
  stock_price: z.coerce.number().positive().describe("Current stock price"),
  conversion_ratio: z.coerce.number().positive().describe("Shares per bond on conversion"),
  stock_volatility: z.coerce.number().positive().describe("Stock return volatility"),
  dividend_yield: z.coerce.number().min(0).optional().describe("Stock dividend yield"),
  call_price: z.coerce.number().positive().optional().describe("Issuer call price"),
  call_protection_years: z.coerce.number().min(0).optional().describe("Call protection period in years"),
  put_price: z.coerce.number().positive().optional().describe("Investor put price"),
  put_date_years: z.coerce.number().min(0).optional().describe("Put exercise date in years"),
  tree_steps: z.coerce.number().int().positive().optional().describe("Binomial tree steps (default 100)"),
});

export const ConvertibleAnalysisSchema = z.object({
  bond_name: z.string().describe("Bond name / identifier"),
  face_value: z.coerce.number().positive().describe("Face value"),
  coupon_rate: z.coerce.number().min(0).describe("Annual coupon rate"),
  maturity_years: z.coerce.number().positive().describe("Years to maturity"),
  credit_spread: z.coerce.number().min(0).describe("Credit spread"),
  risk_free_rate: z.coerce.number().describe("Risk-free rate"),
  stock_price: z.coerce.number().positive().describe("Current stock price"),
  conversion_ratio: z.coerce.number().positive().describe("Conversion ratio"),
  stock_volatility: z.coerce.number().positive().describe("Stock volatility"),
  dividend_yield: z.coerce.number().min(0).optional().describe("Dividend yield"),
  call_price: z.coerce.number().positive().optional().describe("Call price"),
  stock_scenarios: z.array(z.coerce.number().positive()).min(1).describe("Stock price scenarios"),
  vol_scenarios: z.array(z.coerce.number().positive()).optional().describe("Volatility scenarios"),
  spread_scenarios: z.array(z.coerce.number()).optional().describe("Credit spread scenarios"),
});
