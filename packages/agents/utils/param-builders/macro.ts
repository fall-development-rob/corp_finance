// Macro param builders: monetary policy, international econ, FX, commodities, country risk, EM bonds, trade finance, carbon

import type { ParamBuilder } from './types.js';

export const monetary_policy: ParamBuilder = (_m) => ({
  policy_rate: 0.0525,
  inflation_rate: 0.032,
  inflation_target: 0.02,
  output_gap: -0.01,
  unemployment_rate: 0.039,
  natural_rate: 0.04,
  gdp_growth: 0.025,
});

export const international_economics: ParamBuilder = (_m) => ({
  country: 'US',
  gdp_growth: 0.025,
  inflation: 0.032,
  trade_balance_pct_gdp: -0.03,
  current_account_pct_gdp: -0.035,
  fiscal_balance_pct_gdp: -0.06,
  debt_to_gdp: 1.2,
  policy_rate: 0.0525,
});

export const fx_forward: ParamBuilder = (_m) => ({
  spot_rate: 1.08,
  domestic_rate: 0.0525,
  foreign_rate: 0.04,
  maturity_days: 90,
  notional: 1e6,
  currency_pair: 'EURUSD',
});

export const cross_rate: ParamBuilder = (_m) => ({
  rate1: 1.08,
  rate2: 149.5,
  pair1: 'EURUSD',
  pair2: 'USDJPY',
  target_pair: 'EURJPY',
});

export const commodity_spread: ParamBuilder = (_m) => ({
  front_price: 75.50,
  back_price: 77.20,
  front_expiry_months: 1,
  back_expiry_months: 3,
  commodity: 'WTI Crude',
  contract_size: 1000,
});

export const commodity_forward: ParamBuilder = (_m) => ({
  spot_price: 75.50,
  risk_free_rate: 0.04,
  storage_cost_rate: 0.02,
  convenience_yield: 0.01,
  maturity_years: 0.5,
});

export const country_risk_premium: ParamBuilder = (_m) => ({
  country: 'Brazil',
  sovereign_spread_bps: 250,
  equity_market_vol: 0.25,
  bond_market_vol: 0.10,
  default_probability: 0.02,
  country_credit_rating: 'BB+',
});

export const em_bond_analysis: ParamBuilder = (_m) => ({
  country: 'Brazil',
  face_value: 1000,
  coupon_rate: 0.065,
  maturity_years: 10,
  yield: 0.075,
  us_treasury_yield: 0.045,
  fx_spot: 5.0,
  fx_forward_points: 0.15,
  local_inflation: 0.045,
  us_inflation: 0.03,
});

export const country_risk_assessment: ParamBuilder = (_m) => ({
  country: 'US',
  gdp_growth: 0.025,
  inflation: 0.032,
  debt_to_gdp: 1.2,
  fiscal_balance_pct: -0.06,
  current_account_pct: -0.035,
  fx_reserves_months: 2,
  political_stability_score: 0.8,
  governance_score: 0.85,
});

export const letter_of_credit: ParamBuilder = (_m) => ({
  amount: 1e6,
  currency: 'USD',
  tenor_days: 90,
  issuing_bank_rating: 'A',
  type: 'Irrevocable',
  margin_pct: 0.015,
});

export const carbon_credit_pricing: ParamBuilder = (_m) => ({
  credit_type: 'EUA',
  volume_tonnes: 10000,
  vintage_year: 2024,
  spot_price: 80,
  delivery_date: '2025-12-15',
  risk_free_rate: 0.04,
  volatility: 0.30,
});
