// Credit param builders: metrics, scorecard, spreads, Merton, portfolio, CDS, distressed, Beneish, CAMELS, covenants

import type { ParamBuilder } from './types.js';

export const credit_metrics: ParamBuilder = (m) => ({
  revenue: m.revenue,
  ebitda: m.ebitda,
  ebit: m.ebit ?? (m.ebitda && m.depreciation ? m.ebitda - m.depreciation : undefined),
  interest_expense: m.interest_expense,
  depreciation_amortisation: m.depreciation,
  total_debt: m.total_debt,
  cash: m.cash,
  total_assets: m.total_assets,
  current_assets: m.current_assets,
  current_liabilities: m.current_liabilities ?? (m.current_ratio && m.current_assets ? m.current_assets / m.current_ratio : undefined),
  total_equity: m.total_equity,
  retained_earnings: undefined,
  working_capital: m.current_assets && m.current_liabilities ? m.current_assets - m.current_liabilities : undefined,
  operating_cash_flow: m.operating_cash_flow,
  capex: m.capex,
});

export const credit_scorecard: ParamBuilder = (_m) => ({
  bins: [],
  target_score: 600,
  target_odds: 50,
  pdo: 20,
});

export const credit_spreads: ParamBuilder = (m) => ({
  face_value: m.face_value ?? 1000,
  coupon_rate: m.coupon_rate ?? 0.05,
  coupon_frequency: 2,
  market_price: undefined,
  years_to_maturity: m.maturity_years ?? 5,
  benchmark_curve: [],
  recovery_rate: m.recovery_rate ?? 0.4,
});

export const merton_pd: ParamBuilder = (m) => ({
  equity_value: m.market_cap,
  equity_vol: m.volatility ?? 0.30,
  debt_face: m.total_debt,
  risk_free_rate: m.risk_free_rate ?? 0.04,
  maturity: m.maturity_years ?? 1,
  growth_rate: m.growth_rate,
});

export const portfolio_credit_risk: ParamBuilder = (_m) => ({
  exposures: [],
  default_correlation: 0.20,
  confidence_level: 0.99,
  num_simulations: 10000,
});

export const cds_pricing: ParamBuilder = (m) => ({
  notional: m.total_debt,
  spread_bps: undefined,
  recovery_rate: m.recovery_rate ?? 0.4,
  maturity_years: m.maturity_years ?? 5,
  risk_free_rate: m.risk_free_rate ?? 0.04,
  payment_frequency: 4,
});

export const distressed_debt_analysis: ParamBuilder = (m) => ({
  enterprise_value: m.enterprise_value,
  exit_enterprise_value: undefined,
  exit_timeline_years: 2,
  capital_structure: [],
  proposed_treatment: [],
  operating_assumptions: {
    annual_ebitda: m.ebitda,
    maintenance_capex: m.capex,
    working_capital_change: undefined,
    restructuring_costs: undefined,
  },
});

export const beneish_mscore: ParamBuilder = (m) => ({
  current_receivables: m.receivables,
  prior_receivables: undefined,
  current_revenue: m.revenue,
  prior_revenue: undefined,
  current_cogs: m.cogs,
  prior_cogs: undefined,
  current_total_assets: m.total_assets,
  prior_total_assets: undefined,
  current_ppe: m.ppe,
  prior_ppe: undefined,
  current_depreciation: m.depreciation,
  prior_depreciation: undefined,
  current_sga: m.sga,
  prior_sga: undefined,
  current_total_debt: m.total_debt,
  prior_total_debt: undefined,
  current_net_income: m.net_income,
  current_cfo: m.operating_cash_flow,
});

export const camels_rating: ParamBuilder = (m) => ({
  tier1_capital: m.total_equity,
  total_capital: m.total_equity,
  risk_weighted_assets: m.total_assets,
  npl_ratio: undefined,
  provision_coverage: undefined,
  management_score: undefined,
  roa: m.net_income && m.total_assets ? m.net_income / m.total_assets : undefined,
  roe: m.net_income && m.total_equity ? m.net_income / m.total_equity : undefined,
  nim: undefined,
  liquid_assets: m.cash,
  total_deposits: undefined,
  rate_sensitivity_gap: undefined,
});

export const covenant_compliance: ParamBuilder = (m) => {
  const ebitda = m.ebitda;
  const debt = m.total_debt;
  const interest = m.interest_expense;
  const ocf = m.operating_cash_flow;
  const capex = m.capex;
  return {
    covenants: [
      { name: 'Max Leverage', metric: 'NetDebtToEbitda', threshold: 4.0, direction: 'MaxOf' },
      { name: 'Min Interest Coverage', metric: 'InterestCoverage', threshold: 3.0, direction: 'MinOf' },
      { name: 'Min DSCR', metric: 'Dscr', threshold: 1.2, direction: 'MinOf' },
    ],
    actuals: {
      net_debt: m.net_debt,
      net_debt_to_ebitda: m.net_debt && ebitda ? m.net_debt / ebitda : undefined,
      total_debt_to_ebitda: debt && ebitda ? debt / ebitda : undefined,
      debt_to_equity: m.debt_to_equity,
      debt_to_assets: debt && m.total_assets ? debt / m.total_assets : undefined,
      interest_coverage: m.interest_coverage ?? (ebitda && interest ? ebitda / interest : undefined),
      ebit_coverage: m.ebit && interest ? m.ebit / interest : undefined,
      dscr: ebitda && interest && debt ? ebitda / (interest + debt * 0.05) : undefined,
      ocf_to_debt: ocf && debt ? ocf / debt : undefined,
      fcf_to_debt: ocf && capex && debt ? (ocf - capex) / debt : undefined,
      fcf: ocf && capex ? ocf - capex : undefined,
      cash_conversion: ocf && ebitda ? ocf / ebitda : undefined,
      current_ratio: m.current_ratio,
      quick_ratio: undefined,
      cash_to_debt: m.cash && debt ? m.cash / debt : undefined,
      implied_rating: undefined,
      rating_rationale: [],
    },
  };
};
