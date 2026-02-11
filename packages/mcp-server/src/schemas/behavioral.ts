import { z } from "zod";

const OutcomeSchema = z.object({
  description: z.string().describe("Outcome description"),
  value: z.coerce.number().describe("Outcome value"),
  probability: z.coerce.number().describe("Outcome probability (0 to 1)"),
});

export const ProspectTheorySchema = z.object({
  outcomes: z.array(OutcomeSchema).describe("List of possible outcomes with values and probabilities"),
  reference_point: z.coerce.number().describe("Reference point for gain/loss framing"),
  current_value: z.coerce.number().describe("Current value of the investment"),
  loss_aversion_lambda: z.coerce.number().positive().describe("Loss aversion coefficient (e.g. 2.25)"),
  alpha: z.coerce.number().describe("Value function exponent for gains (0 < alpha <= 1, e.g. 0.88)"),
  beta_param: z.coerce.number().describe("Value function exponent for losses (0 < beta <= 1, e.g. 0.88)"),
  gamma: z.coerce.number().describe("Probability weighting parameter for gains (e.g. 0.61)"),
  delta_param: z.coerce.number().describe("Probability weighting parameter for losses (e.g. 0.69)"),
  holding_period_months: z.coerce.number().int().min(0).describe("Holding period in months"),
  annual_return_history: z.array(z.coerce.number()).optional().describe("Historical annual returns"),
});

const RiskIndicatorSchema = z.object({
  name: z.string().describe("Indicator name"),
  value: z.coerce.number().describe("Current indicator value"),
  bullish_threshold: z.coerce.number().describe("Bullish threshold value"),
  bearish_threshold: z.coerce.number().describe("Bearish threshold value"),
  weight: z.coerce.number().describe("Indicator weight"),
});

export const SentimentSchema = z.object({
  market_name: z.string().describe("Market name or index"),
  vix_current: z.coerce.number().min(0).describe("Current VIX level"),
  vix_sma_50: z.coerce.number().min(0).describe("50-day simple moving average of VIX"),
  put_call_ratio: z.coerce.number().min(0).describe("Current put/call ratio"),
  put_call_sma_20: z.coerce.number().describe("20-day SMA of put/call ratio"),
  advance_decline_ratio: z.coerce.number().describe("Advance/decline ratio"),
  new_highs_lows_ratio: z.coerce.number().describe("New highs to new lows ratio"),
  margin_debt_change_pct: z.coerce.number().describe("Month-over-month change in margin debt (decimal)"),
  fund_flows: z.coerce.number().describe("Net fund flows (positive = inflow)"),
  short_interest_ratio: z.coerce.number().describe("Short interest ratio (days to cover)"),
  insider_buy_sell_ratio: z.coerce.number().describe("Insider buy/sell ratio"),
  consumer_confidence: z.coerce.number().describe("Consumer confidence index (0-100)"),
  risk_appetite_indicators: z.array(RiskIndicatorSchema).optional().describe("Custom risk appetite indicators"),
  contrarian_mode: z.coerce.boolean().optional().describe("Enable contrarian signal generation"),
});
