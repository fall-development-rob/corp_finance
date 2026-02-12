import { z } from "zod";

const SectorConstraintSchema = z.object({
  name: z.string().describe("Sector or group name"),
  asset_indices: z.array(z.coerce.number().int().min(0)).describe("Indices of assets in this sector"),
  min_weight: z.coerce.number().describe("Minimum allocation to this sector"),
  max_weight: z.coerce.number().describe("Maximum allocation to this sector"),
});

const OptimizationConstraintsSchema = z.object({
  min_weights: z.array(z.coerce.number()).optional().describe("Per-asset minimum weights (optional)"),
  max_weights: z.array(z.coerce.number()).optional().describe("Per-asset maximum weights (optional)"),
  long_only: z.boolean().describe("Whether to enforce long-only (no short selling)"),
  max_total_short: z.coerce.number().min(0).optional().describe("Maximum total short exposure (optional)"),
  sector_constraints: z.array(SectorConstraintSchema).optional().describe("Sector-level weight constraints (optional)"),
});

export const MeanVarianceSchema = z.object({
  asset_names: z.array(z.string()).min(1).describe("Asset identifiers"),
  expected_returns: z.array(z.coerce.number()).describe("Annualized expected returns per asset"),
  covariance_matrix: z.array(z.array(z.coerce.number())).describe("N x N covariance matrix"),
  risk_free_rate: z.coerce.number().describe("Annual risk-free rate (decimal)"),
  constraints: OptimizationConstraintsSchema.describe("Portfolio constraints"),
  frontier_points: z.coerce.number().int().min(2).optional().describe("Number of efficient frontier points (default 20)"),
  target_return: z.coerce.number().optional().describe("Specific target return for optimization (optional)"),
  target_risk: z.coerce.number().optional().describe("Specific target risk for optimization (optional)"),
});

const ViewSchema = z.discriminatedUnion("type", [
  z.object({
    type: z.literal("Absolute"),
    asset_index: z.coerce.number().int().min(0).describe("Index of asset in asset_names"),
    expected_return: z.coerce.number().describe("Expected absolute return"),
  }),
  z.object({
    type: z.literal("Relative"),
    long_index: z.coerce.number().int().min(0).describe("Index of outperforming asset"),
    short_index: z.coerce.number().int().min(0).describe("Index of underperforming asset"),
    expected_return: z.coerce.number().describe("Expected relative return"),
  }),
]);

export const BlackLittermanPortfolioSchema = z.object({
  asset_names: z.array(z.string()).min(1).describe("Asset identifiers"),
  market_cap_weights: z.array(z.coerce.number()).describe("Market-capitalization equilibrium weights"),
  covariance_matrix: z.array(z.array(z.coerce.number())).describe("N x N covariance matrix"),
  risk_free_rate: z.coerce.number().describe("Annual risk-free rate (decimal)"),
  risk_aversion: z.coerce.number().positive().describe("Market risk-aversion coefficient lambda (typically 2.5)"),
  tau: z.coerce.number().positive().describe("Scaling factor for equilibrium return uncertainty (typically 0.05)"),
  views: z.array(ViewSchema).describe("Investor views (absolute or relative)"),
  view_confidences: z.array(z.coerce.number().min(0).max(1)).describe("Confidence in each view (0-1)"),
});
