// Quant Risk param builders: tail risk, mean-variance, risk parity, factor model, stress test,
// attribution, momentum, index weighting, spread analysis

import type { ParamBuilder } from './types.js';

export const tail_risk_analysis: ParamBuilder = (_m) => ({
  asset_names: [],
  weights: [],
  expected_returns: [],
  covariance_matrix: [],
  confidence_level: 0.95,
  time_horizon: 1 / 252,
  distribution: 'Normal',
  portfolio_value: undefined,
});

export const mean_variance_optimization: ParamBuilder = (m) => ({
  asset_names: [],
  expected_returns: [],
  covariance_matrix: [],
  risk_free_rate: m.risk_free_rate ?? 0.04,
  target_return: undefined,
  constraints: { long_only: true },
});

export const risk_parity: ParamBuilder = (m) => ({
  assets: [],
  covariance_matrix: [],
  method: 'EqualRiskContribution',
  risk_free_rate: m.risk_free_rate ?? 0.04,
});

export const factor_model: ParamBuilder = (m) => ({
  asset_returns: [],
  factor_returns: [],
  model_type: 'FamaFrench3',
  risk_free_rate: m.risk_free_rate ?? 0.04,
});

export const stress_test: ParamBuilder = (_m) => ({
  portfolio: [],
  scenarios: [
    { name: '2008 GFC', scenario_type: 'Historical', shocks: [
      { factor: 'equity_market', shock_pct: -0.40 },
      { factor: 'credit_spreads', shock_pct: 0.03 },
      { factor: 'interest_rates', shock_pct: -0.02 },
    ]},
    { name: 'Rate Shock', scenario_type: 'Hypothetical', shocks: [
      { factor: 'interest_rates', shock_pct: 0.02 },
      { factor: 'equity_market', shock_pct: -0.10 },
      { factor: 'credit_spreads', shock_pct: 0.01 },
    ]},
  ],
});

export const factor_attribution: ParamBuilder = (m) => ({
  portfolio_name: m._company ?? 'Portfolio',
  portfolio_return: undefined,
  benchmark_return: undefined,
  factors: [],
  risk_free_rate: m.risk_free_rate ?? 0.04,
});

export const brinson_attribution: ParamBuilder = (m) => ({
  portfolio_name: m._company ?? 'Portfolio',
  benchmark_name: undefined,
  sectors: [],
  risk_free_rate: m.risk_free_rate ?? 0.04,
});

export const momentum_analysis: ParamBuilder = (_m) => ({
  asset_returns: [],
  lookback_months: 12,
  holding_months: 1,
  skip_months: 1,
});

export const index_weighting: ParamBuilder = (_m) => ({
  constituents: [],
  methodology: 'MarketCapWeighted',
  max_weight: 0.30,
  min_weight: 0.01,
});

export const spread_analysis: ParamBuilder = (_m) => ({
  bid_prices: [],
  ask_prices: [],
  trade_volumes: [],
  timestamps: [],
});
