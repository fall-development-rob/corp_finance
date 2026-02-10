import { z } from "zod";

export const FactorModelSchema = z.object({
  asset_returns: z.array(z.coerce.number()).describe("Time series of asset excess returns"),
  factor_returns: z.array(z.object({
    name: z.string().describe("Factor name (e.g. MKT, SMB, HML, MOM)"),
    returns: z.array(z.coerce.number()).describe("Factor excess returns per period"),
  })).describe("Factor return time-series (one per factor)"),
  model_type: z.enum(["CAPM", "FamaFrench3", "Carhart4", "Custom"]).describe("Which factor model to run"),
  risk_free_rate: z.coerce.number().min(0).max(0.2).describe("Risk-free rate (for documentation)"),
  confidence_level: z.coerce.number().min(0.8).max(0.99).optional().describe("Confidence level for t-stat significance testing (default 0.95)"),
});

export const BlackLittermanSchema = z.object({
  market_cap_weights: z.array(z.object({
    name: z.string().describe("Asset name"),
    weight: z.coerce.number().min(0).max(1).describe("Market-cap weight"),
  })).describe("Market-cap weighted portfolio (weights must sum to ~1)"),
  covariance_matrix: z.array(z.array(z.coerce.number())).describe("NxN annualised covariance matrix (row-major)"),
  risk_aversion: z.coerce.number().positive().describe("Risk aversion coefficient (delta), typically around 2.5"),
  tau: z.coerce.number().positive().max(1).describe("Uncertainty scaling factor (tau), typically 0.025-0.05"),
  views: z.array(z.object({
    view_type: z.enum(["Absolute", "Relative"]).describe("Whether the view is absolute or relative"),
    assets: z.array(z.string()).describe("Asset names involved in this view"),
    asset_weights: z.array(z.coerce.number()).describe("Pick-matrix row weights for each asset"),
    expected_return: z.coerce.number().describe("Expected return expressed by this view"),
    confidence: z.coerce.number().min(0).max(1).describe("Confidence in the view (0 to 1)"),
  })).describe("Investor views"),
  risk_free_rate: z.coerce.number().min(0).max(0.2).describe("Risk-free rate (annualised)"),
});

export const RiskParitySchema = z.object({
  assets: z.array(z.object({
    name: z.string().describe("Asset name"),
    expected_return: z.coerce.number().describe("Expected annualised return"),
    volatility: z.coerce.number().positive().describe("Annualised volatility"),
  })).describe("Asset descriptions"),
  covariance_matrix: z.array(z.array(z.coerce.number())).describe("NxN covariance matrix (row-major)"),
  method: z.enum(["InverseVolatility", "EqualRiskContribution", "MinVariance"]).describe("Optimisation method"),
  target_volatility: z.coerce.number().positive().optional().describe("Optional target portfolio volatility for weight scaling"),
  risk_free_rate: z.coerce.number().min(0).optional().describe("Risk-free rate for Sharpe computation (defaults to 0)"),
});

export const StressTestSchema = z.object({
  portfolio: z.array(z.object({
    name: z.string().describe("Position name"),
    weight: z.coerce.number().min(0).max(1).describe("Portfolio weight"),
    asset_class: z.enum(["Equity", "FixedIncome", "Credit", "Commodity", "Currency", "RealEstate", "Alternative"]).describe("Asset class"),
    beta: z.coerce.number().optional().describe("Equity beta (default 1.0)"),
    duration: z.coerce.number().optional().describe("Fixed income / credit duration (default 5.0)"),
    fx_exposure: z.string().optional().describe("Currency code of FX exposure"),
  })).describe("Current portfolio positions"),
  scenarios: z.array(z.object({
    name: z.string().describe("Scenario name"),
    scenario_type: z.enum(["Historical", "Hypothetical"]).describe("Historical or hypothetical scenario"),
    shocks: z.array(z.object({
      factor: z.string().describe("Risk factor name (equity_market, interest_rates, credit_spreads, fx_usd, commodities, volatility)"),
      shock_pct: z.coerce.number().describe("Shock magnitude as decimal (e.g. -0.40 for 40% decline)"),
    })).describe("Market risk factor shocks"),
  })).describe("Scenarios to evaluate"),
  correlation_adjustments: z.coerce.boolean().optional().describe("Multiply historical impacts by 1.2 for crisis correlation spikes (default true)"),
});
