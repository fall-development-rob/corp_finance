// Fixed Income param builders: bonds, spot curve, rate models, TIPS, repo, MBS, munis, sovereign

import type { ParamBuilder } from './types.js';

export const bond_pricer: ParamBuilder = (m) => ({
  face_value: m.face_value ?? 1000,
  coupon_rate: m.coupon_rate ?? 0.05,
  coupon_frequency: 2,
  ytm: m.ytm ?? m.yield,
  settlement_date: undefined,
  maturity_date: undefined,
  day_count: 'Thirty360',
});

export const bootstrap_spot_curve: ParamBuilder = (_m) => ({
  par_instruments: [],
});

export const short_rate_model: ParamBuilder = (m) => ({
  model_type: 'Vasicek',
  current_rate: m.risk_free_rate ?? 0.04,
  mean_reversion_speed: 0.3,
  long_term_mean: 0.045,
  volatility: 0.015,
  time_horizon: 5,
  num_steps: 252,
  num_paths: 1000,
});

export const tips_analytics: ParamBuilder = (_m) => ({
  face_value: 1000,
  real_coupon_rate: 0.0125,
  coupon_frequency: 2,
  years_to_maturity: 10,
  current_cpi: 310,
  base_cpi: 280,
  nominal_yield: 0.045,
  real_yield: 0.015,
});

export const repo_analytics: ParamBuilder = (_m) => ({
  collateral_value: 1e7,
  repo_rate: 0.04,
  haircut: 0.02,
  term_days: 30,
  collateral_type: 'Treasury',
});

export const prepayment_analysis: ParamBuilder = (_m) => ({
  original_balance: 1e6,
  coupon_rate: 0.055,
  wac: 0.055,
  wam_months: 300,
  current_rate: 0.045,
  seasoning_months: 36,
  model_type: 'PSA',
  psa_speed: 150,
});

export const municipal_analysis: ParamBuilder = (_m) => ({
  par_value: 1e6,
  coupon_rate: 0.04,
  yield: 0.035,
  maturity_years: 20,
  tax_bracket: 0.37,
  state_tax_rate: 0.05,
  credit_rating: 'AA',
  insurance_premium_bps: 10,
});

export const sovereign_bond_analysis: ParamBuilder = (_m) => ({
  country: 'US',
  face_value: 1000,
  coupon_rate: 0.04,
  maturity_years: 10,
  yield: 0.042,
  cds_spread_bps: 15,
  debt_to_gdp: 1.2,
  fiscal_balance_pct: -0.05,
  current_account_pct: -0.03,
  inflation_rate: 0.03,
  fx_reserves_months: 3,
});
