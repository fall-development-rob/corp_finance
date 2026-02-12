import { z } from "zod";

export const FactorRiskBudgetSchema = z.object({
  asset_names: z.array(z.string()).min(1).describe("Asset identifiers"),
  weights: z.array(z.coerce.number()).describe("Portfolio weights (should sum to 1)"),
  factor_names: z.array(z.string()).min(1).describe("Factor identifiers"),
  factor_loadings: z.array(z.array(z.coerce.number())).describe("N x K matrix of factor loadings (assets x factors)"),
  factor_covariance: z.array(z.array(z.coerce.number())).describe("K x K factor covariance matrix"),
  specific_variances: z.array(z.coerce.number()).describe("N-vector of idiosyncratic variances"),
  risk_budgets: z.array(z.coerce.number()).optional().describe("Target risk budget per factor (should sum to 1, optional)"),
  rebalance: z.boolean().describe("Whether to solve for optimal weights matching target budgets"),
});

const StressScenarioSchema = z.object({
  name: z.string().describe("Scenario name"),
  asset_shocks: z.array(z.coerce.number()).describe("Return shock per asset"),
});

export const TailRiskSchema = z.object({
  asset_names: z.array(z.string()).min(1).describe("Asset identifiers"),
  weights: z.array(z.coerce.number()).describe("Portfolio weights"),
  expected_returns: z.array(z.coerce.number()).describe("Expected returns per asset"),
  covariance_matrix: z.array(z.array(z.coerce.number())).describe("N x N covariance matrix"),
  confidence_level: z.coerce.number().min(0.5).max(0.9999).describe("VaR/CVaR confidence level (e.g. 0.95 or 0.99)"),
  time_horizon: z.coerce.number().positive().describe("Time horizon in years (e.g. 1/252 for daily)"),
  distribution: z.enum([
    "Normal",
    "CornishFisher",
    "Historical",
  ]).describe("Assumed return distribution"),
  historical_returns: z.array(z.array(z.coerce.number())).optional().describe("T x N historical return matrix (required for Historical/CornishFisher)"),
  portfolio_value: z.coerce.number().positive().describe("Portfolio net asset value for dollar risk measures"),
  stress_scenarios: z.array(StressScenarioSchema).optional().describe("Optional stress test scenarios"),
});
