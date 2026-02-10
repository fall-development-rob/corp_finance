import { z } from "zod";

const McDistributionSchema = z.discriminatedUnion("type", [
  z.object({
    type: z.literal("Normal"),
    mean: z.coerce.number().describe("Mean of the normal distribution"),
    std_dev: z.coerce.number().positive().describe("Standard deviation"),
  }),
  z.object({
    type: z.literal("LogNormal"),
    mu: z.coerce.number().describe("Log-mean parameter"),
    sigma: z.coerce.number().positive().describe("Log-standard deviation"),
  }),
  z.object({
    type: z.literal("Triangular"),
    min: z.coerce.number().describe("Minimum value"),
    mode: z.coerce.number().describe("Most likely value"),
    max: z.coerce.number().describe("Maximum value"),
  }),
  z.object({
    type: z.literal("Uniform"),
    min: z.coerce.number().describe("Minimum value"),
    max: z.coerce.number().describe("Maximum value"),
  }),
]);

export const MonteCarloSchema = z.object({
  num_simulations: z.coerce.number().int().min(100).max(1000000).optional().describe("Number of simulation paths (default 10,000)"),
  seed: z.coerce.number().int().optional().describe("Optional seed for reproducibility"),
  variables: z.array(z.object({
    name: z.string().describe("Variable name"),
    distribution: McDistributionSchema.describe("Probability distribution for this variable"),
  })).describe("Variables to simulate"),
});

export const McDcfSchema = z.object({
  base_fcf: z.coerce.number().describe("Base year free cash flow"),
  projection_years: z.coerce.number().int().min(1).max(30).describe("Number of projection years"),
  revenue_growth: McDistributionSchema.describe("Distribution for annual revenue growth rate"),
  ebitda_margin: McDistributionSchema.describe("Distribution for EBITDA margin"),
  wacc: McDistributionSchema.describe("Distribution for the discount rate (WACC)"),
  terminal_growth: McDistributionSchema.describe("Distribution for the terminal growth rate"),
  capex_pct: z.coerce.number().min(0).max(1).describe("Capex as a percentage of revenue (fixed)"),
  tax_rate: z.coerce.number().min(0).max(0.5).describe("Corporate tax rate"),
  num_simulations: z.coerce.number().int().min(100).max(1000000).optional().describe("Number of simulation paths (default 10,000)"),
  seed: z.coerce.number().int().optional().describe("Optional seed for reproducibility"),
});
