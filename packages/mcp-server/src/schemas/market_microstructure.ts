import { z } from "zod";

const TradeRecordSchema = z.object({
  timestamp: z.coerce.number().int().min(0).describe("Epoch milliseconds"),
  price: z.coerce.number().positive().describe("Trade price"),
  volume: z.coerce.number().positive().describe("Trade volume"),
  side: z.enum(["Buy", "Sell", "Unknown"]).describe("Trade direction"),
});

const QuoteRecordSchema = z.object({
  timestamp: z.coerce.number().int().min(0).describe("Epoch milliseconds"),
  bid_price: z.coerce.number().positive().describe("Bid price"),
  ask_price: z.coerce.number().positive().describe("Ask price"),
  bid_size: z.coerce.number().positive().describe("Bid size"),
  ask_size: z.coerce.number().positive().describe("Ask size"),
});

export const SpreadAnalysisSchema = z.object({
  security_name: z.string().describe("Security identifier"),
  trade_data: z.array(TradeRecordSchema).min(1).describe("Time-sequenced trade data"),
  quote_data: z.array(QuoteRecordSchema).min(1).describe("Time-sequenced quote data (NBBO)"),
  analysis_method: z.enum([
    "Quoted",
    "Effective",
    "Realized",
    "RollModel",
    "KyleModel",
  ]).describe("Spread decomposition method"),
  benchmark_spread: z.coerce.number().optional().describe("Benchmark spread for comparison (optional)"),
  daily_volume: z.coerce.number().positive().describe("Average daily volume"),
  market_cap: z.coerce.number().positive().optional().describe("Market capitalisation for liquidity scoring (optional)"),
});

const MarketParametersSchema = z.object({
  current_price: z.coerce.number().positive().describe("Current market price"),
  daily_volume: z.coerce.number().positive().describe("Average daily volume"),
  daily_volatility: z.coerce.number().positive().describe("Annualized volatility (decimal)"),
  bid_ask_spread: z.coerce.number().min(0).describe("Bid-ask spread in price units"),
  volume_profile: z.array(z.coerce.number()).optional().describe("Intraday volume distribution (sums to 1, optional)"),
  temporary_impact: z.coerce.number().describe("Temporary price impact coefficient (eta)"),
  permanent_impact: z.coerce.number().describe("Permanent price impact coefficient (gamma)"),
});

const ExecutionConstraintsSchema = z.object({
  max_participation_rate: z.coerce.number().min(0).max(1).describe("Maximum % of market volume per slice"),
  min_slice_size: z.coerce.number().min(0).optional().describe("Minimum quantity per slice (optional)"),
  max_slice_size: z.coerce.number().min(0).optional().describe("Maximum quantity per slice (optional)"),
  no_trade_periods: z.array(z.tuple([z.coerce.number().int(), z.coerce.number().int()])).optional().describe("Slice-index ranges where trading is not allowed (optional)"),
});

export const OptimalExecutionSchema = z.object({
  security_name: z.string().describe("Security identifier"),
  order_size: z.coerce.number().positive().describe("Total shares/units to execute"),
  side: z.enum(["Buy", "Sell"]).describe("Order direction"),
  execution_strategy: z.enum([
    "TWAP",
    "VWAP",
    "IS",
    "POV",
  ]).describe("Execution strategy (TWAP/VWAP/IS/POV)"),
  market_params: MarketParametersSchema.describe("Market parameters"),
  time_horizon: z.coerce.number().positive().describe("Total execution window in hours"),
  num_slices: z.coerce.number().int().min(1).describe("Number of time slices"),
  urgency: z.coerce.number().min(0).max(1).describe("Urgency parameter: 0 (patient) to 1 (urgent)"),
  constraints: ExecutionConstraintsSchema.describe("Execution constraints"),
});
