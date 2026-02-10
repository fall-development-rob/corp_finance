import { z } from 'zod';

export const BondPricingSchema = z.object({
  face_value: z.number().positive().describe('Par / face value (e.g. 1000)'),
  coupon_rate: z.number().min(0).describe('Annual coupon rate as decimal (0.05 = 5%)'),
  coupon_frequency: z.number().int().describe('Coupons per year: 1, 2, 4, or 12'),
  ytm: z.number().describe('Yield to maturity as decimal'),
  settlement_date: z.string().describe('Settlement date (YYYY-MM-DD)'),
  maturity_date: z.string().describe('Maturity date (YYYY-MM-DD)'),
  day_count: z.enum(['Thirty360', 'Actual360', 'Actual365', 'ActualActual']).describe('Day count convention'),
  call_price: z.number().optional().describe('Call price for callable bonds'),
  call_date: z.string().optional().describe('Call date for callable bonds (YYYY-MM-DD)'),
});

export const BondYieldSchema = z.object({
  face_value: z.number().positive().describe('Par / face value'),
  coupon_rate: z.number().min(0).describe('Annual coupon rate as decimal'),
  coupon_frequency: z.number().int().describe('Coupons per year: 1, 2, 4, or 12'),
  market_price: z.number().positive().describe('Market (dirty) price of the bond'),
  years_to_maturity: z.number().positive().describe('Years remaining until maturity'),
  current_yield_only: z.boolean().optional().default(false).describe('Skip Newton-Raphson YTM solve'),
});

export const ParInstrumentSchema = z.object({
  maturity_years: z.number().positive().describe('Maturity in years'),
  par_rate: z.number().describe('Par (coupon) rate as decimal'),
  coupon_frequency: z.number().int().describe('Coupons per year'),
});

export const BootstrapSchema = z.object({
  par_instruments: z.array(ParInstrumentSchema).min(2).describe('Par instruments sorted by maturity'),
});

export const ObservedRateSchema = z.object({
  maturity: z.number().positive().describe('Maturity in years'),
  rate: z.number().describe('Observed yield as decimal'),
});

export const NelsonSiegelSchema = z.object({
  observed_rates: z.array(ObservedRateSchema).min(3).describe('Observed market rates'),
  initial_lambda: z.number().optional().describe('Decay parameter lambda (default 1.0)'),
});

export const DurationSchema = z.object({
  face_value: z.number().positive().describe('Par / face value'),
  coupon_rate: z.number().min(0).describe('Annual coupon rate as decimal'),
  coupon_frequency: z.number().int().describe('Coupons per year: 1, 2, 4, or 12'),
  ytm: z.number().describe('Yield to maturity as decimal'),
  years_to_maturity: z.number().positive().describe('Years remaining'),
  yield_shift_bps: z.number().optional().describe('Yield shift in bps for effective duration (default 10)'),
  key_rate_tenors: z.array(z.number()).optional().describe('Tenors for key rate duration (e.g. [1, 2, 5, 10, 30])'),
});

export const BenchmarkPointSchema = z.object({
  maturity: z.number().positive().describe('Time to maturity in years'),
  rate: z.number().describe('Spot rate as decimal'),
});

export const CreditSpreadSchema = z.object({
  face_value: z.number().positive().describe('Par / face value'),
  coupon_rate: z.number().min(0).describe('Annual coupon rate as decimal'),
  coupon_frequency: z.number().int().describe('Coupons per year: 1, 2, 4, or 12'),
  market_price: z.number().positive().describe('Dirty market price'),
  years_to_maturity: z.number().positive().describe('Years remaining'),
  benchmark_curve: z.array(BenchmarkPointSchema).min(2).describe('Risk-free benchmark spot curve'),
  recovery_rate: z.number().min(0).max(1).optional().describe('Recovery rate for CDS (default 0.40)'),
  default_probability: z.number().min(0).max(1).optional().describe('Annual default probability'),
});
