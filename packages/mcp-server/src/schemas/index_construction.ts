import { z } from "zod";

export const WeightingSchema = z.object({
  constituents: z.array(z.object({
    ticker: z.string().describe("Stock ticker symbol"),
    market_cap: z.coerce.number().describe("Market capitalization"),
    price: z.coerce.number().describe("Current stock price"),
    shares: z.coerce.number().describe("Shares outstanding"),
    free_float_pct: z.coerce.number().describe("Free float percentage as decimal"),
    revenue: z.coerce.number().describe("Annual revenue"),
    book_value: z.coerce.number().describe("Book value"),
    dividends: z.coerce.number().describe("Annual dividends paid"),
    earnings: z.coerce.number().describe("Annual earnings"),
    sector: z.string().describe("Sector classification"),
  })).describe("Array of index constituents"),
  weighting_method: z.string().describe("Weighting method (market_cap, equal, fundamental, free_float)"),
  cap_weight: z.coerce.number().describe("Maximum weight cap per constituent as decimal"),
  fundamental_weights: z.object({
    revenue_w: z.coerce.number().describe("Revenue factor weight"),
    book_w: z.coerce.number().describe("Book value factor weight"),
    dividend_w: z.coerce.number().describe("Dividend factor weight"),
    earnings_w: z.coerce.number().describe("Earnings factor weight"),
  }).optional().describe("Fundamental weighting factors (required for fundamental method)"),
});

export const RebalancingSchema = z.object({
  current_weights: z.array(z.object({
    ticker: z.string().describe("Stock ticker symbol"),
    current_weight: z.coerce.number().describe("Current portfolio weight as decimal"),
    target_weight: z.coerce.number().describe("Target portfolio weight as decimal"),
    price: z.coerce.number().describe("Current stock price"),
    avg_daily_volume: z.coerce.number().describe("Average daily trading volume in shares"),
  })).describe("Array of position weights"),
  portfolio_value: z.coerce.number().describe("Total portfolio value"),
  transaction_cost_bps: z.coerce.number().describe("Transaction cost in basis points"),
  rebalance_threshold: z.coerce.number().describe("Drift threshold triggering rebalance as decimal"),
  rebalance_frequency: z.string().describe("Rebalance frequency (daily, monthly, quarterly, annual)"),
});

export const TrackingErrorSchema = z.object({
  portfolio_returns: z.array(z.coerce.number()).describe("Array of portfolio period returns"),
  benchmark_returns: z.array(z.coerce.number()).describe("Array of benchmark period returns"),
  portfolio_weights: z.array(z.object({
    ticker: z.string().describe("Stock ticker symbol"),
    weight: z.coerce.number().describe("Portfolio weight as decimal"),
  })).describe("Portfolio constituent weights"),
  benchmark_weights: z.array(z.object({
    ticker: z.string().describe("Stock ticker symbol"),
    weight: z.coerce.number().describe("Benchmark weight as decimal"),
  })).describe("Benchmark constituent weights"),
  covariance_diagonal: z.array(z.coerce.number()).describe("Diagonal of the covariance matrix (variances)"),
});

export const SmartBetaSchema = z.object({
  constituents: z.array(z.object({
    ticker: z.string().describe("Stock ticker symbol"),
    market_cap: z.coerce.number().describe("Market capitalization"),
    price: z.coerce.number().describe("Current stock price"),
    beta: z.coerce.number().describe("Stock beta relative to market"),
    momentum_score: z.coerce.number().describe("Momentum score (e.g., 12-1 month return)"),
    value_score: z.coerce.number().describe("Value score (e.g., composite of B/P, E/P, D/P)"),
    quality_score: z.coerce.number().describe("Quality score (e.g., ROE, stability, leverage)"),
    volatility: z.coerce.number().describe("Annualized volatility as decimal"),
    dividend_yield: z.coerce.number().describe("Dividend yield as decimal"),
  })).describe("Array of smart beta constituents"),
  factor_tilts: z.object({
    value_tilt: z.coerce.number().describe("Value factor tilt strength"),
    momentum_tilt: z.coerce.number().describe("Momentum factor tilt strength"),
    quality_tilt: z.coerce.number().describe("Quality factor tilt strength"),
    low_vol_tilt: z.coerce.number().describe("Low volatility factor tilt strength"),
    dividend_tilt: z.coerce.number().describe("Dividend factor tilt strength"),
  }).describe("Factor tilt configuration"),
  max_weight: z.coerce.number().describe("Maximum constituent weight as decimal"),
  min_weight: z.coerce.number().describe("Minimum constituent weight as decimal"),
});

export const ReconstitutionSchema = z.object({
  current_members: z.array(z.object({
    ticker: z.string().describe("Stock ticker symbol"),
    market_cap: z.coerce.number().describe("Market capitalization"),
    meets_criteria: z.boolean().describe("Whether member currently meets inclusion criteria"),
    float_pct: z.coerce.number().describe("Free float percentage as decimal"),
    avg_volume: z.coerce.number().describe("Average daily trading volume"),
  })).describe("Array of current index members"),
  candidates: z.array(z.object({
    ticker: z.string().describe("Stock ticker symbol"),
    market_cap: z.coerce.number().describe("Market capitalization"),
    meets_criteria: z.boolean().describe("Whether candidate meets inclusion criteria"),
    float_pct: z.coerce.number().describe("Free float percentage as decimal"),
    avg_volume: z.coerce.number().describe("Average daily trading volume"),
  })).describe("Array of candidate stocks for inclusion"),
  min_market_cap: z.coerce.number().describe("Minimum market cap for index membership"),
  min_float_pct: z.coerce.number().describe("Minimum free float percentage as decimal"),
  min_volume: z.coerce.number().describe("Minimum average daily trading volume"),
  max_members: z.coerce.number().int().describe("Maximum number of index members"),
  buffer_zone_pct: z.coerce.number().describe("Buffer zone percentage to reduce unnecessary turnover"),
});
