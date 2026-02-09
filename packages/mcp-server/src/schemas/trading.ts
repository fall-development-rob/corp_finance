import { z } from 'zod';

export const TradeEntrySchema = z.object({
  trade_number: z.number().int().positive(),
  contract: z.string().describe('Contract symbol (e.g., ES, NQ, CL)'),
  time_open: z.string().describe('Time trade opened (HH:MM:SS)'),
  time_close: z.string().optional().describe('Time trade closed'),
  open_price: z.number().describe('Entry price'),
  profit_target: z.number().optional().describe('Target exit price'),
  stop_loss: z.number().optional().describe('Stop loss price'),
  actual_exit: z.number().optional().describe('Actual exit price'),
  profit_loss: z.number().optional().describe('P+L on trade'),
  comment: z.string().optional(),
});

export const CancelledTradeSchema = z.object({
  description: z.string(),
  reason: z.string(),
});

export const TradingDaySchema = z.object({
  date: z.string().describe('Trading date (YYYY-MM-DD)'),
  confidence_level: z.number().int().min(0).max(10).describe('Trader confidence 0-10'),
  support_levels: z.array(z.number()).describe('Price support levels'),
  resistance_levels: z.array(z.number()).describe('Price resistance levels'),
  trades: z.array(TradeEntrySchema).min(1),
  cancelled_trades: z.array(CancelledTradeSchema).optional().default([]),
  trader_comments: z.string().optional(),
  currency: z.string().optional(),
});

export const DaySummarySchema = z.object({
  date: z.string().describe('Trading date (YYYY-MM-DD)'),
  daily_pnl: z.number().describe('Net daily P+L'),
  num_trades: z.number().int(),
  num_winners: z.number().int(),
  num_losers: z.number().int(),
  total_profit: z.number().describe('Sum of winning trades'),
  total_losses: z.number().describe('Sum of losing trades (positive number)'),
  confidence_level: z.number().int().min(0).max(10),
});

export const TradingAnalyticsSchema = z.object({
  day_summaries: z.array(DaySummarySchema).min(1),
  starting_capital: z.number().positive().describe('Starting account capital'),
  risk_free_rate: z.number().optional().describe('Annual risk-free rate for Sharpe calculation'),
  currency: z.string().optional(),
});
