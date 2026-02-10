import { z } from "zod";

export const RiskAdjustedSchema = z.object({
  returns: z
    .array(z.number())
    .min(2)
    .describe("Periodic returns as decimals (e.g. 0.02 = 2%)"),
  risk_free_rate: z
    .number()
    .min(0)
    .max(0.2)
    .describe("Annualised risk-free rate"),
  benchmark_returns: z
    .array(z.number())
    .optional()
    .describe("Benchmark returns for relative metrics (same frequency)"),
  frequency: z
    .enum(["Daily", "Weekly", "Monthly", "Quarterly", "Annual"])
    .describe("Return observation frequency"),
  target_return: z
    .number()
    .optional()
    .describe("Target return for Sortino ratio calculation (annualised); defaults to risk_free_rate"),
});

export const RiskMetricsSchema = z.object({
  returns: z
    .array(z.number())
    .min(3)
    .describe("Periodic returns as decimals (e.g. 0.02 = 2%)"),
  frequency: z
    .enum(["Daily", "Weekly", "Monthly", "Quarterly", "Annual"])
    .describe("Return observation frequency"),
  confidence_level: z
    .number()
    .min(0.9)
    .max(0.999)
    .describe("Confidence level for VaR/CVaR (e.g. 0.95 or 0.99)"),
  portfolio_value: z
    .number()
    .positive()
    .optional()
    .describe("Portfolio value for absolute VaR"),
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
    .describe("Fraction of full Kelly to use (e.g. 0.5 for half-Kelly)"),
  portfolio_value: z
    .number()
    .positive()
    .optional()
    .describe("Portfolio value for absolute position sizing"),
  max_position_pct: z
    .number()
    .min(0.01)
    .max(1)
    .optional()
    .describe("Maximum position as a percentage of portfolio"),
});
