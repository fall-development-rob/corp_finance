import { z } from "zod";

export const RiskAdjustedSchema = z.object({
  returns: z
    .array(z.number())
    .min(2)
    .describe("Periodic returns as decimals (e.g. 0.02 = 2%)"),
  frequency: z
    .enum(["Daily", "Weekly", "Monthly", "Quarterly", "Annual"])
    .describe("Return observation frequency"),
  risk_free_rate: z
    .number()
    .min(0)
    .max(0.2)
    .optional()
    .describe("Annualised risk-free rate (default 0)"),
  benchmark_returns: z
    .array(z.number())
    .optional()
    .describe("Benchmark returns for relative metrics (same frequency)"),
  target_return: z
    .number()
    .optional()
    .describe("Target return for Sortino ratio calculation"),
});

export const RiskMetricsSchema = z.object({
  returns: z
    .array(z.number())
    .min(2)
    .describe("Periodic returns as decimals (e.g. 0.02 = 2%)"),
  frequency: z
    .enum(["Daily", "Weekly", "Monthly", "Quarterly", "Annual"])
    .describe("Return observation frequency"),
  confidence_level: z
    .number()
    .min(0.9)
    .max(0.999)
    .describe("Confidence level for VaR/CVaR (e.g. 0.95 or 0.99)"),
  benchmark_returns: z
    .array(z.number())
    .optional()
    .describe("Benchmark returns for relative risk metrics"),
  risk_free_rate: z
    .number()
    .min(0)
    .max(0.2)
    .optional()
    .describe("Annualised risk-free rate"),
  portfolio_value: z
    .number()
    .positive()
    .optional()
    .describe("Current portfolio value for absolute VaR"),
});

export const KellySchema = z.object({
  win_probability: z
    .number()
    .min(0.01)
    .max(0.99)
    .describe("Probability of a winning trade"),
  win_loss_ratio: z
    .number()
    .positive()
    .describe("Average win size / average loss size"),
  kelly_fraction: z
    .number()
    .min(0.01)
    .max(1)
    .optional()
    .describe("Fractional Kelly multiplier (e.g. 0.25 for quarter-Kelly)"),
  portfolio_value: z
    .number()
    .positive()
    .optional()
    .describe("Total portfolio value for position sizing"),
  max_position_pct: z
    .number()
    .min(0.01)
    .max(1)
    .optional()
    .describe("Hard cap on position size as % of portfolio"),
});
