import { z } from 'zod';

// ---------------------------------------------------------------------------
// Option Pricing — matches OptionInput in options.rs
// ---------------------------------------------------------------------------
export const OptionPriceSchema = z.object({
  spot_price: z.number().positive().describe('Underlying spot price'),
  strike_price: z.number().positive().describe('Strike price'),
  time_to_expiry: z.number().positive().describe('Time to expiration in years'),
  risk_free_rate: z.number().describe('Risk-free rate (annualised decimal)'),
  volatility: z.number().positive().describe('Annualised volatility (decimal)'),
  dividend_yield: z.number().optional().default(0).describe('Continuous dividend yield'),
  option_type: z.enum(['Call', 'Put']).describe('Option type'),
  exercise_style: z.enum(['European', 'American']).describe('Exercise style'),
  binomial_steps: z.number().int().optional().describe('Number of binomial tree steps (default 100)'),
});

// ---------------------------------------------------------------------------
// Implied Volatility — matches ImpliedVolInput in options.rs
// ---------------------------------------------------------------------------
export const ImpliedVolSchema = z.object({
  spot_price: z.number().positive().describe('Underlying spot price'),
  strike_price: z.number().positive().describe('Strike price'),
  time_to_expiry: z.number().positive().describe('Time to expiration in years'),
  risk_free_rate: z.number().describe('Risk-free rate (annualised decimal)'),
  dividend_yield: z.number().optional().default(0).describe('Continuous dividend yield'),
  option_type: z.enum(['Call', 'Put']).describe('Option type'),
  market_price: z.number().positive().describe('Market price of the option'),
});

// ---------------------------------------------------------------------------
// Forward Pricing — matches ForwardInput in forwards.rs
// ---------------------------------------------------------------------------
export const ForwardPriceSchema = z.object({
  spot_price: z.number().positive().describe('Current spot price of the underlying asset'),
  risk_free_rate: z.number().describe('Annualised risk-free interest rate (decimal)'),
  time_to_expiry: z.number().positive().describe('Time to expiry in years'),
  storage_cost_rate: z.number().optional().describe('Annualised storage cost as percentage of spot (commodities)'),
  convenience_yield: z.number().optional().describe('Annualised convenience yield (commodities)'),
  dividend_yield: z.number().optional().describe('Annualised continuous dividend yield (equity/index)'),
  foreign_rate: z.number().optional().describe('Foreign risk-free rate for currency forwards'),
  underlying_type: z.enum(['Equity', 'Commodity', 'Currency', 'Index', 'Bond']).describe('Type of underlying asset'),
});

// ---------------------------------------------------------------------------
// Forward Position Valuation — matches ForwardPositionInput in forwards.rs
// ---------------------------------------------------------------------------
export const ForwardPositionSchema = z.object({
  original_forward_price: z.number().positive().describe('Original locked-in forward price'),
  current_spot: z.number().positive().describe('Current spot price'),
  risk_free_rate: z.number().describe('Current annualised risk-free rate'),
  remaining_time: z.number().positive().describe('Remaining time to expiry in years'),
  is_long: z.boolean().describe('True if long the forward, false if short'),
  contract_size: z.number().positive().describe('Number of units in the contract'),
  dividend_yield: z.number().optional().describe('Continuous dividend yield (if applicable)'),
});

// ---------------------------------------------------------------------------
// Futures Basis Analysis — matches BasisAnalysisInput / FuturesContract in forwards.rs
// ---------------------------------------------------------------------------
export const FuturesContractSchema = z.object({
  expiry_months: z.number().positive().describe('Months until expiry'),
  price: z.number().positive().describe('Observed futures price'),
  label: z.string().describe('Descriptive label (e.g. "Mar-25", "Jun-25")'),
});

export const BasisAnalysisSchema = z.object({
  spot_price: z.number().positive().describe('Current spot price'),
  futures_prices: z.array(FuturesContractSchema).min(1).describe('Futures contracts across the term structure'),
  risk_free_rate: z.number().describe('Annualised risk-free rate'),
});

// ---------------------------------------------------------------------------
// Interest Rate Swap — matches IrsInput / DiscountPoint / ForwardRatePoint in swaps.rs
// ---------------------------------------------------------------------------
export const DiscountPointSchema = z.object({
  maturity: z.number().positive().describe('Maturity in year fractions'),
  rate: z.number().describe('Spot rate as decimal'),
});

export const ForwardRatePointSchema = z.object({
  maturity: z.number().positive().describe('Maturity in year fractions'),
  rate: z.number().describe('Forward rate as decimal'),
});

export const IrsSchema = z.object({
  notional: z.number().positive().describe('Notional principal amount'),
  fixed_rate: z.number().describe('Fixed rate (annualised decimal)'),
  payment_frequency: z.number().int().describe('Payments per year: 1, 2, or 4'),
  remaining_years: z.number().positive().describe('Remaining swap tenor in years'),
  discount_curve: z.array(DiscountPointSchema).min(1).describe('Discount / spot rate curve points'),
  forward_rates: z.array(ForwardRatePointSchema).optional().describe('Forward rate curve points (derived from discount curve if omitted)'),
  is_pay_fixed: z.boolean().describe('True if valuing from pay-fixed perspective'),
  last_floating_reset: z.number().optional().describe('Last observed floating reset rate for the current period'),
});

// ---------------------------------------------------------------------------
// Currency Swap — matches CurrencySwapInput in swaps.rs
// ---------------------------------------------------------------------------
export const CurrencySwapSchema = z.object({
  notional_domestic: z.number().positive().describe('Domestic notional amount'),
  notional_foreign: z.number().positive().describe('Foreign notional amount'),
  domestic_fixed_rate: z.number().describe('Domestic fixed coupon rate'),
  foreign_fixed_rate: z.number().describe('Foreign fixed coupon rate'),
  payment_frequency: z.number().int().describe('Payments per year: 1, 2, or 4'),
  remaining_years: z.number().positive().describe('Remaining swap tenor in years'),
  domestic_discount_curve: z.array(DiscountPointSchema).min(1).describe('Domestic discount curve points'),
  foreign_discount_curve: z.array(DiscountPointSchema).min(1).describe('Foreign discount curve points'),
  spot_fx_rate: z.number().positive().describe('Spot FX rate (domestic per foreign)'),
  is_pay_domestic: z.boolean().describe('True if paying domestic leg'),
});

// ---------------------------------------------------------------------------
// Option Strategy Analysis — matches StrategyInput / StrategyLeg in strategies.rs
// ---------------------------------------------------------------------------
export const StrategyLegSchema = z.object({
  leg_type: z.enum(['Call', 'Put', 'Stock']).describe('Type of leg'),
  position: z.enum(['Long', 'Short']).describe('Long or short position'),
  strike: z.number().positive().optional().describe('Strike price (required for Call/Put legs)'),
  premium: z.number().describe('Premium per unit (purchase price for Stock legs)'),
  quantity: z.number().positive().describe('Number of contracts / shares'),
});

export const StrategySchema = z.object({
  strategy_type: z.enum([
    'LongCall', 'LongPut', 'CoveredCall', 'ProtectivePut',
    'BullCallSpread', 'BearPutSpread', 'LongStraddle', 'LongStrangle',
    'IronCondor', 'ButterflySpread', 'Collar', 'Custom',
  ]).describe('Strategy type'),
  underlying_price: z.number().positive().describe('Current underlying price'),
  legs: z.array(StrategyLegSchema).min(1).describe('Strategy legs'),
  price_range: z.tuple([z.number(), z.number()]).optional().describe('Price range [low, high] for payoff analysis'),
  price_steps: z.number().int().optional().describe('Number of price steps in payoff table (default 21)'),
});
