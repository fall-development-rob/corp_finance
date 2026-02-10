import { z } from 'zod';

export const OptionPriceSchema = z.object({
  spot: z.number().positive().describe('Underlying spot price'),
  strike: z.number().positive().describe('Strike price'),
  risk_free_rate: z.number().describe('Risk-free rate (annualised decimal)'),
  volatility: z.number().min(0).describe('Annualised volatility (decimal)'),
  time_to_expiry: z.number().min(0).describe('Time to expiration in years'),
  option_type: z.enum(['Call', 'Put']).describe('Option type'),
  option_style: z.enum(['European', 'American']).optional().default('European'),
  pricing_model: z.enum(['BlackScholes', 'Binomial']).optional().default('BlackScholes'),
  dividend_yield: z.number().optional().default(0).describe('Continuous dividend yield'),
  binomial_steps: z.number().int().optional().describe('Number of binomial tree steps'),
});

export const ImpliedVolSchema = z.object({
  spot: z.number().positive().describe('Underlying spot price'),
  strike: z.number().positive().describe('Strike price'),
  risk_free_rate: z.number().describe('Risk-free rate'),
  time_to_expiry: z.number().positive().describe('Time to expiration in years'),
  market_price: z.number().positive().describe('Market price of the option'),
  option_type: z.enum(['Call', 'Put']).describe('Option type'),
  dividend_yield: z.number().optional().default(0),
});

export const ForwardPriceSchema = z.object({
  spot: z.number().positive().describe('Current spot price'),
  risk_free_rate: z.number().describe('Domestic risk-free rate'),
  time_to_expiry: z.number().positive().describe('Time to delivery in years'),
  underlying_type: z.enum(['Financial', 'Commodity', 'Currency']).describe('Type of underlying'),
  dividend_yield: z.number().optional().default(0),
  storage_cost_rate: z.number().optional().default(0),
  convenience_yield: z.number().optional().default(0),
  foreign_rate: z.number().optional().default(0),
});

export const ForwardPositionSchema = z.object({
  spot: z.number().positive().describe('Current spot price'),
  delivery_price: z.number().positive().describe('Original delivery price'),
  risk_free_rate: z.number().describe('Risk-free rate'),
  time_remaining: z.number().min(0).describe('Remaining time to delivery in years'),
  dividend_yield: z.number().optional().default(0),
  position: z.number().int().optional().default(1).describe('1 for long, -1 for short'),
});

export const FuturesContractSchema = z.object({
  expiry_label: z.string().describe('Contract expiry label (e.g. "Mar25")'),
  price: z.number().positive().describe('Futures price'),
  time_to_expiry: z.number().positive().describe('Time to expiry in years'),
});

export const BasisAnalysisSchema = z.object({
  spot: z.number().positive().describe('Current spot price'),
  risk_free_rate: z.number().describe('Risk-free rate'),
  dividend_yield: z.number().optional().default(0),
  storage_cost_rate: z.number().optional().default(0),
  convenience_yield: z.number().optional().default(0),
  contracts: z.array(FuturesContractSchema).min(1).describe('Futures contracts to analyse'),
});

export const DiscountPointSchema = z.object({
  maturity: z.number().positive().describe('Maturity in years'),
  rate: z.number().describe('Spot rate as decimal'),
});

export const IrsSchema = z.object({
  notional: z.number().positive().describe('Notional principal'),
  fixed_rate: z.number().describe('Fixed rate (annualised decimal)'),
  payment_frequency: z.number().int().describe('Payments per year: 1, 2, or 4'),
  remaining_years: z.number().positive().describe('Remaining swap tenor in years'),
  discount_curve: z.array(DiscountPointSchema).min(1).describe('Discount curve points'),
  forward_curve: z.array(DiscountPointSchema).min(1).describe('Forward rate curve points'),
  perspective: z.enum(['payer', 'receiver']).optional().default('payer'),
});

export const CurrencySwapSchema = z.object({
  domestic_notional: z.number().positive().describe('Domestic notional'),
  foreign_notional: z.number().positive().describe('Foreign notional'),
  domestic_rate: z.number().describe('Domestic fixed rate'),
  foreign_rate: z.number().describe('Foreign fixed rate'),
  spot_fx_rate: z.number().positive().describe('Spot FX rate (domestic per foreign)'),
  payment_frequency: z.number().int().describe('Payments per year: 1, 2, or 4'),
  remaining_years: z.number().positive().describe('Remaining years'),
  domestic_discount_rate: z.number().describe('Domestic discount rate'),
  foreign_discount_rate: z.number().describe('Foreign discount rate'),
});

export const StrategyLegSchema = z.object({
  strike: z.number().positive().describe('Strike price'),
  option_type: z.enum(['Call', 'Put']).describe('Option type'),
  quantity: z.number().int().describe('Contracts: positive = long, negative = short'),
  premium: z.number().optional().describe('Premium per contract (calculated if omitted)'),
});

export const StrategySchema = z.object({
  spot: z.number().positive().describe('Underlying spot price'),
  risk_free_rate: z.number().describe('Risk-free rate'),
  volatility: z.number().min(0).describe('Annualised volatility'),
  time_to_expiry: z.number().min(0).describe('Time to expiration in years'),
  dividend_yield: z.number().optional().default(0),
  legs: z.array(StrategyLegSchema).min(1).describe('Strategy legs'),
  strategy_name: z.string().optional().default('custom').describe('Strategy name'),
});
