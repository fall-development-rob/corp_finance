// Derivatives param builders: options, vol surface, SABR, Monte Carlo, convertibles, structured notes, real options

import type { ParamBuilder } from './types.js';

export const option_pricer: ParamBuilder = (m) => ({
  spot_price: m.share_price,
  strike_price: m.share_price ? m.share_price * 1.05 : undefined,
  time_to_expiry: 0.5,
  risk_free_rate: m.risk_free_rate ?? 0.04,
  volatility: m.volatility ?? 0.25,
  option_type: 'Call',
  model: 'BlackScholes',
  dividend_yield: undefined,
});

export const implied_vol_surface: ParamBuilder = (m) => ({
  spot_price: m.share_price,
  risk_free_rate: m.risk_free_rate ?? 0.04,
  option_data: [],
});

export const sabr_calibration: ParamBuilder = (m) => ({
  forward: m.share_price,
  expiry: 0.5,
  market_vols: [],
  beta: 0.5,
});

export const monte_carlo_simulation: ParamBuilder = (m) => ({
  initial_value: m.share_price ?? m.enterprise_value,
  drift: m.growth_rate ?? 0.05,
  volatility: m.volatility ?? 0.25,
  time_horizon: 1.0,
  num_paths: 10000,
  num_steps: 252,
});

export const convertible_bond_pricing: ParamBuilder = (m) => ({
  face_value: m.face_value ?? 1000,
  coupon_rate: m.coupon_rate,
  maturity_years: m.maturity_years,
  conversion_ratio: undefined,
  stock_price: m.share_price,
  stock_volatility: m.volatility ?? 0.30,
  risk_free_rate: m.risk_free_rate ?? 0.04,
  credit_spread: undefined,
  dividend_yield: undefined,
});

export const structured_note_pricing: ParamBuilder = (m) => ({
  face_value: m.face_value ?? 1000,
  maturity_years: m.maturity_years,
  coupon_rate: m.coupon_rate,
  underlying_price: m.share_price,
  underlying_volatility: m.volatility ?? 0.25,
  risk_free_rate: m.risk_free_rate ?? 0.04,
  barrier_level: 0.7,
  participation_rate: 1.5,
  product_type: 'ReverseConvertible',
});

export const real_option_valuation: ParamBuilder = (m) => ({
  underlying_value: m.enterprise_value,
  strike_price: m.enterprise_value ? m.enterprise_value * 0.9 : undefined,
  time_to_expiry: 3,
  risk_free_rate: m.risk_free_rate ?? 0.04,
  volatility: m.volatility ?? 0.30,
  option_type: 'DeferralOption',
  steps: 100,
});
