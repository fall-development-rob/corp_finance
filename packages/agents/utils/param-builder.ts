// Build properly-typed tool parameters from extracted financial metrics
// Each builder returns params that match the MCP server's Zod schema

import type { ExtractedMetrics } from './financial-parser.js';
import { resolveToolName } from '../config/tool-name-resolver.js';

type ParamBuilder = (m: ExtractedMetrics) => Record<string, unknown>;

// ─── Valuation ───────────────────────────────────────────────────────────────

const dcf_model: ParamBuilder = (m) => ({
  base_revenue: m.revenue ?? 1e9,
  revenue_growth_rates: [0.08, 0.07, 0.06, 0.05, 0.04],
  ebitda_margin: m.ebitda_margin ?? 0.20,
  capex_as_pct_revenue: 0.05,
  nwc_as_pct_revenue: 0.02,
  tax_rate: m.tax_rate ?? 0.21,
  wacc: m.wacc ?? 0.10,
  terminal_method: 'GordonGrowth',
  terminal_growth_rate: m.terminal_growth ?? 0.025,
  currency: 'USD',
  net_debt: m.net_debt,
  shares_outstanding: m.shares_outstanding,
});

const wacc_calculator: ParamBuilder = (m) => ({
  risk_free_rate: m.risk_free_rate ?? 0.04,
  equity_risk_premium: 0.055,
  beta: m.beta ?? 1.0,
  cost_of_debt: m.cost_of_debt ?? 0.05,
  tax_rate: m.tax_rate ?? 0.21,
  debt_weight: m.debt_to_equity ? m.debt_to_equity / (1 + m.debt_to_equity) : 0.30,
  equity_weight: m.debt_to_equity ? 1 / (1 + m.debt_to_equity) : 0.70,
});

const comps_analysis: ParamBuilder = (m) => ({
  target_name: m._company ?? 'Target Company',
  target_metrics: {
    revenue: m.revenue ?? 1e9,
    ebitda: m.ebitda ?? 2e8,
    net_income: m.net_income ?? 1e8,
    share_price: m.share_price,
    market_cap: m.market_cap,
    enterprise_value: m.enterprise_value,
  },
  comparables: [
    { name: 'Peer A', metrics: { revenue: (m.revenue ?? 1e9) * 1.1, ebitda: (m.ebitda ?? 2e8) * 1.2, net_income: (m.net_income ?? 1e8) * 1.1, market_cap: (m.market_cap ?? 2e9) * 1.1, enterprise_value: (m.enterprise_value ?? 2.5e9) * 1.1, share_price: 50 }, include: true },
    { name: 'Peer B', metrics: { revenue: (m.revenue ?? 1e9) * 0.9, ebitda: (m.ebitda ?? 2e8) * 0.85, net_income: (m.net_income ?? 1e8) * 0.9, market_cap: (m.market_cap ?? 2e9) * 0.9, enterprise_value: (m.enterprise_value ?? 2.5e9) * 0.9, share_price: 35 }, include: true },
    { name: 'Peer C', metrics: { revenue: (m.revenue ?? 1e9) * 1.3, ebitda: (m.ebitda ?? 2e8) * 1.4, net_income: (m.net_income ?? 1e8) * 1.2, market_cap: (m.market_cap ?? 2e9) * 1.3, enterprise_value: (m.enterprise_value ?? 2.5e9) * 1.3, share_price: 65 }, include: true },
  ],
  multiples: ['EvEbitda', 'EvRevenue', 'PriceEarnings'],
  currency: 'USD',
});

const sotp_valuation: ParamBuilder = (m) => ({
  company_name: m._company ?? 'Target Company',
  segments: [
    { name: 'Core Business', revenue: m.revenue ?? 1e9, ebitda: m.ebitda ?? 2e8, ebit: m.ebit ?? 1.5e8, method: 'EvEbitda', multiple: 10 },
  ],
  net_debt: m.net_debt ?? 0,
  shares_outstanding: m.shares_outstanding ?? 1e8,
});

// ─── Credit ──────────────────────────────────────────────────────────────────

const credit_metrics: ParamBuilder = (m) => ({
  revenue: m.revenue ?? 1e9,
  ebitda: m.ebitda ?? 2e8,
  ebit: m.ebit ?? m.ebitda ? (m.ebitda ?? 2e8) * 0.8 : 1.6e8,
  interest_expense: m.interest_expense ?? 3e7,
  depreciation_amortisation: m.depreciation ?? 3e7,
  total_debt: m.total_debt ?? 5e8,
  cash: m.cash ?? 1e8,
  total_assets: m.total_assets ?? 2e9,
  current_assets: m.current_assets ?? 4e8,
  current_liabilities: m.current_liabilities ?? (m.current_ratio ? (m.current_assets ?? 4e8) / m.current_ratio : 2.2e8),
  total_equity: m.total_equity ?? 8e8,
  retained_earnings: 5e8,
  working_capital: (m.current_assets ?? 4e8) - (m.current_liabilities ?? 2.2e8),
  operating_cash_flow: m.operating_cash_flow ?? (m.ebitda ?? 2e8) * 0.7,
  capex: m.capex ?? 5e7,
});

const credit_scorecard: ParamBuilder = () => ({
  bins: [
    { lower: 0, upper: 300, good_count: 50, bad_count: 200 },
    { lower: 300, upper: 500, good_count: 200, bad_count: 150 },
    { lower: 500, upper: 700, good_count: 500, bad_count: 50 },
    { lower: 700, upper: 850, good_count: 800, bad_count: 10 },
  ],
  target_score: 600,
  target_odds: 50,
  pdo: 20,
});

const credit_spreads: ParamBuilder = (m) => ({
  face_value: m.face_value ?? 1000,
  coupon_rate: m.coupon_rate ?? 0.05,
  coupon_frequency: 2,
  market_price: 980,
  years_to_maturity: m.maturity_years ?? 5,
  benchmark_curve: [
    { maturity: 1, rate: 0.04 },
    { maturity: 2, rate: 0.042 },
    { maturity: 5, rate: 0.045 },
    { maturity: 10, rate: 0.048 },
  ],
  recovery_rate: m.recovery_rate ?? 0.4,
});

const merton_pd: ParamBuilder = (m) => ({
  equity_value: m.market_cap ?? 1e9,
  equity_vol: m.volatility ?? 0.30,
  debt_face: m.total_debt ?? 5e8,
  risk_free_rate: m.risk_free_rate ?? 0.04,
  maturity: m.maturity_years ?? 1,
  growth_rate: m.growth_rate ?? 0.03,
});

const portfolio_credit_risk: ParamBuilder = () => ({
  exposures: [
    { name: 'Corp A', exposure: 1e7, pd: 0.02, lgd: 0.45, rating: 'BBB' },
    { name: 'Corp B', exposure: 5e6, pd: 0.01, lgd: 0.40, rating: 'A' },
    { name: 'Corp C', exposure: 8e6, pd: 0.05, lgd: 0.60, rating: 'BB' },
  ],
  default_correlation: 0.20,
  confidence_level: 0.99,
  num_simulations: 10000,
});

const cds_pricing: ParamBuilder = (m) => ({
  notional: m.total_debt ?? 1e7,
  spread_bps: 200,
  recovery_rate: m.recovery_rate ?? 0.4,
  maturity_years: m.maturity_years ?? 5,
  risk_free_rate: m.risk_free_rate ?? 0.04,
  payment_frequency: 4,
});

const distressed_debt_analysis: ParamBuilder = (m) => ({
  enterprise_value: m.enterprise_value ?? 1e9,
  exit_enterprise_value: (m.enterprise_value ?? 1e9) * 1.2,
  exit_timeline_years: 2,
  capital_structure: [
    { name: 'First Lien', face_value: 3e8, market_price: 0.85, coupon_rate: 0.06, maturity_years: 3, seniority: 'FirstLien', is_secured: true },
    { name: 'Second Lien', face_value: 2e8, market_price: 0.50, coupon_rate: 0.09, maturity_years: 5, seniority: 'SecondLien', is_secured: true },
    { name: 'Unsecured', face_value: 1.5e8, market_price: 0.25, coupon_rate: 0.08, maturity_years: 7, seniority: 'Senior', is_secured: false },
  ],
  proposed_treatment: [
    { tranche_name: 'First Lien', treatment_type: 'Reinstate' },
    { tranche_name: 'Second Lien', treatment_type: 'Exchange', new_face_value: 1.5e8, new_coupon: 0.07 },
    { tranche_name: 'Unsecured', treatment_type: 'EquityConversion', equity_conversion_pct: 0.60 },
  ],
  operating_assumptions: {
    annual_ebitda: m.ebitda ?? 1.5e8,
    maintenance_capex: 2e7,
    working_capital_change: 5e6,
    restructuring_costs: 1e7,
  },
});

const beneish_mscore: ParamBuilder = (m) => {
  const rev = m.revenue ?? 1e9;
  return {
    current_receivables: rev * 0.12, prior_receivables: rev * 0.11,
    current_revenue: rev, prior_revenue: rev * 0.95,
    current_cogs: rev * 0.6, prior_cogs: rev * 0.58,
    current_total_assets: m.total_assets ?? 2e9, prior_total_assets: (m.total_assets ?? 2e9) * 0.95,
    current_ppe: (m.total_assets ?? 2e9) * 0.3, prior_ppe: (m.total_assets ?? 2e9) * 0.29,
    current_depreciation: m.depreciation ?? 3e7, prior_depreciation: (m.depreciation ?? 3e7) * 0.95,
    current_sga: rev * 0.15, prior_sga: rev * 0.14,
    current_total_debt: m.total_debt ?? 5e8, prior_total_debt: (m.total_debt ?? 5e8) * 0.98,
    current_net_income: m.net_income ?? rev * 0.1,
    current_cfo: m.operating_cash_flow ?? rev * 0.12,
  };
};

const camels_rating: ParamBuilder = (m) => ({
  tier1_capital: (m.total_equity ?? 8e8) * 0.8,
  total_capital: m.total_equity ?? 8e8,
  risk_weighted_assets: (m.total_assets ?? 2e9) * 0.7,
  npl_ratio: 0.025,
  provision_coverage: 1.2,
  management_score: 3,
  roa: (m.net_income ?? 1e8) / (m.total_assets ?? 2e9),
  roe: (m.net_income ?? 1e8) / (m.total_equity ?? 8e8),
  nim: 0.03,
  liquid_assets: m.cash ?? 1e8,
  total_deposits: (m.total_assets ?? 2e9) * 0.6,
  rate_sensitivity_gap: 0.05,
});

const covenant_compliance: ParamBuilder = (m) => {
  const ebitda = m.ebitda ?? 2e8;
  const debt = m.total_debt ?? 5e8;
  const interest = m.interest_expense ?? 3e7;
  return {
    covenants: [
      { name: 'Max Leverage', metric: 'NetDebtToEbitda', threshold: 4.0, direction: 'MaxOf' },
      { name: 'Min Interest Coverage', metric: 'InterestCoverage', threshold: 3.0, direction: 'MinOf' },
      { name: 'Min DSCR', metric: 'Dscr', threshold: 1.2, direction: 'MinOf' },
    ],
    actuals: {
      net_debt: m.net_debt ?? debt * 0.9,
      net_debt_to_ebitda: (m.net_debt ?? debt * 0.9) / ebitda,
      total_debt_to_ebitda: debt / ebitda,
      debt_to_equity: m.debt_to_equity ?? 0.6,
      debt_to_assets: debt / (m.total_assets ?? 2e9),
      interest_coverage: m.interest_coverage ?? ebitda / interest,
      ebit_coverage: (ebitda * 0.8) / interest,
      dscr: ebitda / (interest + debt * 0.05),
      ocf_to_debt: (m.operating_cash_flow ?? ebitda * 0.7) / debt,
      fcf_to_debt: ((m.operating_cash_flow ?? ebitda * 0.7) - (m.capex ?? 5e7)) / debt,
      fcf: (m.operating_cash_flow ?? ebitda * 0.7) - (m.capex ?? 5e7),
      cash_conversion: (m.operating_cash_flow ?? ebitda * 0.7) / ebitda,
      current_ratio: m.current_ratio ?? 1.5,
      quick_ratio: (m.current_ratio ?? 1.5) * 0.8,
      cash_to_debt: (m.cash ?? 1e8) / debt,
      implied_rating: 'BBB',
      rating_rationale: ['Adequate coverage', 'Moderate leverage'],
    },
  };
};

// ─── Fixed Income ────────────────────────────────────────────────────────────

const bond_pricer: ParamBuilder = (m) => ({
  face_value: m.face_value ?? 1000,
  coupon_rate: m.coupon_rate ?? 0.05,
  coupon_frequency: 2,
  ytm: m.ytm ?? m.yield ?? 0.045,
  settlement_date: '2025-01-15',
  maturity_date: '2030-01-15',
  day_count: 'Thirty360',
});

const bootstrap_spot_curve: ParamBuilder = () => ({
  par_instruments: [
    { maturity_years: 0.5, par_rate: 0.04, coupon_frequency: 2 },
    { maturity_years: 1, par_rate: 0.042, coupon_frequency: 2 },
    { maturity_years: 2, par_rate: 0.044, coupon_frequency: 2 },
    { maturity_years: 5, par_rate: 0.046, coupon_frequency: 2 },
    { maturity_years: 10, par_rate: 0.048, coupon_frequency: 2 },
    { maturity_years: 30, par_rate: 0.05, coupon_frequency: 2 },
  ],
});

const short_rate_model: ParamBuilder = (m) => ({
  model_type: 'Vasicek',
  current_rate: m.risk_free_rate ?? 0.04,
  mean_reversion_speed: 0.3,
  long_term_mean: 0.045,
  volatility: 0.015,
  time_horizon: 5,
  num_steps: 252,
  num_paths: 1000,
});

const tips_analytics: ParamBuilder = () => ({
  face_value: 1000,
  real_coupon_rate: 0.0125,
  coupon_frequency: 2,
  years_to_maturity: 10,
  current_cpi: 310,
  base_cpi: 280,
  nominal_yield: 0.045,
  real_yield: 0.015,
});

const repo_analytics: ParamBuilder = () => ({
  collateral_value: 1e7,
  repo_rate: 0.04,
  haircut: 0.02,
  term_days: 30,
  collateral_type: 'Treasury',
});

const prepayment_analysis: ParamBuilder = () => ({
  original_balance: 1e6,
  coupon_rate: 0.055,
  wac: 0.055,
  wam_months: 300,
  current_rate: 0.045,
  seasoning_months: 36,
  model_type: 'PSA',
  psa_speed: 150,
});

const municipal_analysis: ParamBuilder = () => ({
  par_value: 1e6,
  coupon_rate: 0.04,
  yield: 0.035,
  maturity_years: 20,
  tax_bracket: 0.37,
  state_tax_rate: 0.05,
  credit_rating: 'AA',
  insurance_premium_bps: 10,
});

const sovereign_bond_analysis: ParamBuilder = () => ({
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

// ─── Derivatives ─────────────────────────────────────────────────────────────

const option_pricer: ParamBuilder = (m) => ({
  spot_price: m.share_price ?? 100,
  strike_price: (m.share_price ?? 100) * 1.05,
  time_to_expiry: 0.5,
  risk_free_rate: m.risk_free_rate ?? 0.04,
  volatility: m.volatility ?? 0.25,
  option_type: 'Call',
  model: 'BlackScholes',
  dividend_yield: 0.015,
});

const implied_vol_surface: ParamBuilder = (m) => ({
  spot_price: m.share_price ?? 100,
  risk_free_rate: m.risk_free_rate ?? 0.04,
  option_data: [
    { strike: 90, expiry: 0.25, market_price: 12.5, option_type: 'Call' },
    { strike: 100, expiry: 0.25, market_price: 5.8, option_type: 'Call' },
    { strike: 110, expiry: 0.25, market_price: 2.1, option_type: 'Call' },
    { strike: 90, expiry: 0.5, market_price: 14.2, option_type: 'Call' },
    { strike: 100, expiry: 0.5, market_price: 7.9, option_type: 'Call' },
    { strike: 110, expiry: 0.5, market_price: 3.8, option_type: 'Call' },
  ],
});

const sabr_calibration: ParamBuilder = (m) => ({
  forward: m.share_price ?? 100,
  expiry: 0.5,
  market_vols: [
    { strike: 85, vol: 0.30 },
    { strike: 90, vol: 0.27 },
    { strike: 95, vol: 0.25 },
    { strike: 100, vol: 0.24 },
    { strike: 105, vol: 0.245 },
    { strike: 110, vol: 0.255 },
  ],
  beta: 0.5,
});

const monte_carlo_simulation: ParamBuilder = (m) => ({
  initial_value: m.share_price ?? m.enterprise_value ?? 100,
  drift: m.growth_rate ?? 0.05,
  volatility: m.volatility ?? 0.25,
  time_horizon: 1.0,
  num_paths: 10000,
  num_steps: 252,
});

const convertible_bond_pricing: ParamBuilder = (m) => ({
  face_value: 1000,
  coupon_rate: 0.02,
  maturity_years: 5,
  conversion_ratio: 20,
  stock_price: m.share_price ?? 45,
  stock_volatility: m.volatility ?? 0.30,
  risk_free_rate: m.risk_free_rate ?? 0.04,
  credit_spread: 0.02,
  dividend_yield: 0.01,
});

const structured_note_pricing: ParamBuilder = () => ({
  face_value: 1000,
  maturity_years: 3,
  coupon_rate: 0.06,
  underlying_price: 100,
  underlying_volatility: 0.25,
  risk_free_rate: 0.04,
  barrier_level: 0.7,
  participation_rate: 1.5,
  product_type: 'ReverseConvertible',
});

const real_option_valuation: ParamBuilder = (m) => ({
  underlying_value: m.enterprise_value ?? 1e8,
  strike_price: (m.enterprise_value ?? 1e8) * 0.9,
  time_to_expiry: 3,
  risk_free_rate: m.risk_free_rate ?? 0.04,
  volatility: m.volatility ?? 0.30,
  option_type: 'DeferralOption',
  steps: 100,
});

// ─── Quant Risk ──────────────────────────────────────────────────────────────

const tail_risk_analysis: ParamBuilder = () => ({
  asset_names: ['Equities', 'Bonds', 'Alternatives'],
  weights: [0.6, 0.3, 0.1],
  expected_returns: [0.08, 0.04, 0.06],
  covariance_matrix: [
    [0.04, 0.01, 0.015],
    [0.01, 0.0064, 0.005],
    [0.015, 0.005, 0.025],
  ],
  confidence_level: 0.95,
  time_horizon: 1 / 252,
  distribution: 'Normal',
  portfolio_value: 1e7,
});

const mean_variance_optimization: ParamBuilder = () => ({
  asset_names: ['Equities', 'Bonds', 'Alternatives'],
  expected_returns: [0.08, 0.04, 0.06],
  covariance_matrix: [
    [0.04, 0.01, 0.015],
    [0.01, 0.0064, 0.005],
    [0.015, 0.005, 0.025],
  ],
  risk_free_rate: 0.04,
  target_return: 0.07,
  constraints: { long_only: true },
});

const risk_parity: ParamBuilder = () => ({
  assets: [
    { name: 'Equities', expected_return: 0.08, volatility: 0.20 },
    { name: 'Bonds', expected_return: 0.04, volatility: 0.08 },
    { name: 'Commodities', expected_return: 0.05, volatility: 0.18 },
  ],
  covariance_matrix: [
    [0.04, 0.005, 0.01],
    [0.005, 0.0064, 0.002],
    [0.01, 0.002, 0.0324],
  ],
  method: 'EqualRiskContribution',
  risk_free_rate: 0.04,
});

const factor_model: ParamBuilder = () => ({
  asset_returns: Array.from({ length: 60 }, () => (Math.random() - 0.48) * 0.08),
  factor_returns: [
    { name: 'MKT', returns: Array.from({ length: 60 }, () => (Math.random() - 0.48) * 0.06) },
    { name: 'SMB', returns: Array.from({ length: 60 }, () => (Math.random() - 0.5) * 0.03) },
    { name: 'HML', returns: Array.from({ length: 60 }, () => (Math.random() - 0.5) * 0.03) },
  ],
  model_type: 'FamaFrench3',
  risk_free_rate: 0.04,
});

const stress_test: ParamBuilder = () => ({
  portfolio: [
    { name: 'US Equity', weight: 0.40, asset_class: 'Equity', beta: 1.0 },
    { name: 'Int Equity', weight: 0.15, asset_class: 'Equity', beta: 1.1, fx_exposure: 'EUR' },
    { name: 'Corp Bonds', weight: 0.25, asset_class: 'Credit', duration: 5.0 },
    { name: 'Treasuries', weight: 0.15, asset_class: 'FixedIncome', duration: 7.0 },
    { name: 'REITs', weight: 0.05, asset_class: 'RealEstate', beta: 0.8 },
  ],
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

const factor_attribution: ParamBuilder = () => ({
  portfolio_name: 'Portfolio',
  portfolio_return: 0.12,
  benchmark_return: 0.10,
  factors: [
    { factor_name: 'Market', portfolio_exposure: 1.05, benchmark_exposure: 1.0, factor_return: 0.10 },
    { factor_name: 'Size', portfolio_exposure: 0.3, benchmark_exposure: 0.0, factor_return: 0.02 },
    { factor_name: 'Value', portfolio_exposure: -0.2, benchmark_exposure: 0.0, factor_return: 0.03 },
  ],
  risk_free_rate: 0.04,
});

const brinson_attribution: ParamBuilder = () => ({
  portfolio_name: 'Portfolio',
  benchmark_name: 'S&P 500',
  sectors: [
    { sector: 'Technology', portfolio_weight: 0.35, benchmark_weight: 0.30, portfolio_return: 0.15, benchmark_return: 0.12 },
    { sector: 'Healthcare', portfolio_weight: 0.20, benchmark_weight: 0.15, portfolio_return: 0.08, benchmark_return: 0.10 },
    { sector: 'Financials', portfolio_weight: 0.15, benchmark_weight: 0.20, portfolio_return: 0.11, benchmark_return: 0.09 },
    { sector: 'Other', portfolio_weight: 0.30, benchmark_weight: 0.35, portfolio_return: 0.07, benchmark_return: 0.08 },
  ],
  risk_free_rate: 0.04,
});

const momentum_analysis: ParamBuilder = () => ({
  asset_returns: [
    { name: 'Asset A', returns: Array.from({ length: 12 }, () => (Math.random() - 0.45) * 0.08) },
    { name: 'Asset B', returns: Array.from({ length: 12 }, () => (Math.random() - 0.50) * 0.06) },
    { name: 'Asset C', returns: Array.from({ length: 12 }, () => (Math.random() - 0.55) * 0.10) },
  ],
  lookback_months: 12,
  holding_months: 1,
  skip_months: 1,
});

const index_weighting: ParamBuilder = () => ({
  constituents: [
    { name: 'Stock A', market_cap: 1e11, price: 150, free_float: 0.85, sector: 'Technology' },
    { name: 'Stock B', market_cap: 8e10, price: 200, free_float: 0.90, sector: 'Healthcare' },
    { name: 'Stock C', market_cap: 5e10, price: 75, free_float: 0.95, sector: 'Financials' },
  ],
  methodology: 'MarketCapWeighted',
  max_weight: 0.30,
  min_weight: 0.01,
});

const spread_analysis: ParamBuilder = () => ({
  bid_prices: [99.5, 99.6, 99.4, 99.7, 99.3],
  ask_prices: [100.5, 100.4, 100.6, 100.3, 100.7],
  trade_volumes: [1000, 1500, 800, 2000, 1200],
  timestamps: ['09:30', '10:00', '10:30', '11:00', '11:30'],
});

// ─── Macro ───────────────────────────────────────────────────────────────────

const monetary_policy: ParamBuilder = () => ({
  policy_rate: 0.0525,
  inflation_rate: 0.032,
  inflation_target: 0.02,
  output_gap: -0.01,
  unemployment_rate: 0.039,
  natural_rate: 0.04,
  gdp_growth: 0.025,
});

const international_economics: ParamBuilder = () => ({
  country: 'US',
  gdp_growth: 0.025,
  inflation: 0.032,
  trade_balance_pct_gdp: -0.03,
  current_account_pct_gdp: -0.035,
  fiscal_balance_pct_gdp: -0.06,
  debt_to_gdp: 1.2,
  policy_rate: 0.0525,
});

const fx_forward: ParamBuilder = () => ({
  spot_rate: 1.08,
  domestic_rate: 0.0525,
  foreign_rate: 0.04,
  maturity_days: 90,
  notional: 1e6,
  currency_pair: 'EURUSD',
});

const cross_rate: ParamBuilder = () => ({
  rate1: 1.08,
  rate2: 149.5,
  pair1: 'EURUSD',
  pair2: 'USDJPY',
  target_pair: 'EURJPY',
});

const commodity_spread: ParamBuilder = () => ({
  front_price: 75.50,
  back_price: 77.20,
  front_expiry_months: 1,
  back_expiry_months: 3,
  commodity: 'WTI Crude',
  contract_size: 1000,
});

const commodity_forward: ParamBuilder = () => ({
  spot_price: 75.50,
  risk_free_rate: 0.04,
  storage_cost_rate: 0.02,
  convenience_yield: 0.01,
  maturity_years: 0.5,
});

const country_risk_premium: ParamBuilder = () => ({
  country: 'Brazil',
  sovereign_spread_bps: 250,
  equity_market_vol: 0.25,
  bond_market_vol: 0.10,
  default_probability: 0.02,
  country_credit_rating: 'BB+',
});

const em_bond_analysis: ParamBuilder = () => ({
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

const country_risk_assessment: ParamBuilder = () => ({
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

const letter_of_credit: ParamBuilder = () => ({
  amount: 1e6,
  currency: 'USD',
  tenor_days: 90,
  issuing_bank_rating: 'A',
  type: 'Irrevocable',
  margin_pct: 0.015,
});

const carbon_credit_pricing: ParamBuilder = () => ({
  credit_type: 'EUA',
  volume_tonnes: 10000,
  vintage_year: 2024,
  spot_price: 80,
  delivery_date: '2025-12-15',
  risk_free_rate: 0.04,
  volatility: 0.30,
});

// ─── ESG & Regulatory ────────────────────────────────────────────────────────

const esg_score: ParamBuilder = () => ({
  environmental: { carbon_intensity: 150, renewable_pct: 0.3, waste_recycled_pct: 0.6, water_intensity: 50 },
  social: { diversity_pct: 0.4, turnover_rate: 0.12, injury_rate: 1.5, community_investment: 5e6 },
  governance: { board_independence: 0.75, women_on_board: 0.35, ceo_pay_ratio: 150, audit_committee_meetings: 8 },
  weights: { environmental: 0.33, social: 0.33, governance: 0.34 },
});

const carbon_footprint: ParamBuilder = () => ({
  scope1_emissions: 50000,
  scope2_emissions: 30000,
  scope3_emissions: 200000,
  revenue: 1e9,
  portfolio_weight: 0.05,
  carbon_price: 80,
});

const offset_valuation: ParamBuilder = () => ({
  project_type: 'ForestConservation',
  volume_tonnes: 5000,
  vintage_year: 2024,
  registry: 'Verra',
  permanence_years: 25,
  additionality_score: 0.8,
  co_benefits: ['Biodiversity', 'Community'],
});

const best_execution: ParamBuilder = () => ({
  trades: [
    { symbol: 'AAPL', side: 'Buy', quantity: 10000, benchmark_price: 180, execution_price: 180.05, arrival_price: 179.95, vwap: 180.02 },
    { symbol: 'MSFT', side: 'Sell', quantity: 5000, benchmark_price: 400, execution_price: 399.90, arrival_price: 400.10, vwap: 399.95 },
  ],
  market_impact_model: 'SquareRoot',
});

const regulatory_capital: ParamBuilder = (m) => ({
  risk_weighted_assets: (m.total_assets ?? 2e9) * 0.7,
  tier1_capital: (m.total_equity ?? 8e8) * 0.8,
  tier2_capital: (m.total_equity ?? 8e8) * 0.1,
  total_capital: (m.total_equity ?? 8e8) * 0.9,
  minimum_cet1_ratio: 0.045,
  capital_conservation_buffer: 0.025,
  countercyclical_buffer: 0.01,
  systemic_buffer: 0.0,
});

const kyc_risk_assessment: ParamBuilder = () => ({
  entity_type: 'Corporation',
  jurisdiction: 'US',
  industry: 'Technology',
  annual_revenue: 1e9,
  pep_status: false,
  sanctions_match: false,
  adverse_media: false,
  risk_factors: ['complex_structure', 'high_value_transactions'],
});

const sanctions_screening: ParamBuilder = () => ({
  entity_name: 'Acme Corp',
  aliases: ['Acme Corporation', 'ACME LLC'],
  jurisdiction: 'US',
  screening_lists: ['OFAC_SDN', 'EU_SANCTIONS', 'UN_CONSOLIDATED'],
  fuzzy_threshold: 0.85,
});

const entity_classification: ParamBuilder = () => ({
  entity_name: 'Investment Fund LP',
  entity_type: 'LimitedPartnership',
  jurisdiction: 'Cayman Islands',
  passive_income_pct: 0.6,
  passive_assets_pct: 0.7,
  publicly_traded: false,
  regulated_entity: true,
  controlling_persons: [
    { name: 'Person A', nationality: 'US', ownership_pct: 0.30, us_indicia: true },
  ],
});

const treaty_network: ParamBuilder = () => ({
  source_country: 'US',
  recipient_country: 'UK',
  income_type: 'Dividend',
  recipient_type: 'Corporation',
  holding_pct: 0.25,
});

const intercompany_pricing: ParamBuilder = () => ({
  transaction_type: 'ServiceFee',
  transfer_price: 1e6,
  comparable_prices: [8e5, 9e5, 1.1e6, 1.2e6],
  method: 'ComparableUncontrolledPrice',
  entity_a_jurisdiction: 'US',
  entity_b_jurisdiction: 'Ireland',
});

const economic_substance: ParamBuilder = () => ({
  entity_jurisdiction: 'Cayman Islands',
  relevant_activities: ['Holding', 'FinanceLeasing'],
  employees: 5,
  office_space_sqft: 500,
  annual_expenditure: 5e5,
  board_meetings_per_year: 4,
  strategic_decisions_local: true,
});

const aifmd_reporting: ParamBuilder = () => ({
  fund_name: 'CFA Fund',
  fund_type: 'Hedge',
  aum: 5e8,
  nav: 4.8e8,
  leverage_gross: 2.5,
  leverage_commitment: 1.8,
  strategy: 'LongShort',
  jurisdiction: 'Luxembourg',
  reporting_period: 'Q4-2024',
});

// ─── Private Markets ─────────────────────────────────────────────────────────

const lbo_model: ParamBuilder = (m) => ({
  purchase_ev: m.enterprise_value ?? 1e9,
  ebitda: m.ebitda ?? 1.5e8,
  entry_multiple: (m.enterprise_value ?? 1e9) / (m.ebitda ?? 1.5e8),
  exit_multiple: ((m.enterprise_value ?? 1e9) / (m.ebitda ?? 1.5e8)) + 1,
  holding_period: 5,
  revenue_growth: m.growth_rate ?? 0.05,
  margin_expansion: 0.01,
  debt_to_ebitda: 4.0,
  senior_debt_pct: 0.60,
  sub_debt_pct: 0.20,
  equity_pct: 0.20,
  senior_rate: 0.05,
  sub_rate: 0.08,
  tax_rate: m.tax_rate ?? 0.21,
  capex_pct_revenue: 0.03,
  nwc_pct_revenue: 0.02,
  debt_paydown_pct: 0.10,
});

const returns_calculator: ParamBuilder = () => ({
  entry_equity: 2e8,
  exit_equity: 5e8,
  holding_period: 5,
  distributions: [0, 0, 1e7, 2e7, 5e8],
  management_fee_pct: 0.02,
  carry_pct: 0.20,
  preferred_return: 0.08,
  hurdle_type: 'European',
});

const funding_round: ParamBuilder = () => ({
  pre_money_valuation: 5e7,
  raise_amount: 1e7,
  round_type: 'SeriesA',
  existing_shares: 1e7,
  option_pool_pct: 0.10,
  liquidation_preference: 1.0,
  participation: true,
  participation_cap: 3.0,
});

const dilution_analysis: ParamBuilder = () => ({
  current_shares: 1e7,
  rounds: [
    { name: 'Series A', shares_issued: 2e6, price_per_share: 5 },
    { name: 'Series B', shares_issued: 1.5e6, price_per_share: 15 },
  ],
  option_pool_shares: 1e6,
  founders_shares: 6e6,
});

const merger_model: ParamBuilder = (m) => ({
  acquirer: {
    name: 'Acquirer Co',
    share_price: 50,
    shares_outstanding: 1e8,
    eps: 3.0,
    pe_ratio: 16.7,
    net_income: 3e8,
    tax_rate: m.tax_rate ?? 0.21,
  },
  target: {
    name: m._company ?? 'Target Co',
    share_price: m.share_price ?? 30,
    shares_outstanding: m.shares_outstanding ?? 5e7,
    eps: 2.0,
    pe_ratio: 15,
    net_income: m.net_income ?? 1e8,
  },
  offer_price_per_share: (m.share_price ?? 30) * 1.25,
  pct_cash: 0.5,
  pct_stock: 0.5,
  synergies: 5e7,
  integration_costs: 2e7,
  cost_of_debt: m.cost_of_debt ?? 0.05,
});

const ppp_model: ParamBuilder = () => ({
  project_cost: 5e8,
  concession_years: 25,
  construction_years: 3,
  annual_revenue: 8e7,
  opex_pct_revenue: 0.30,
  debt_pct: 0.70,
  equity_pct: 0.30,
  debt_rate: 0.05,
  debt_tenor: 20,
  tax_rate: 0.21,
  inflation_rate: 0.025,
  discount_rate: 0.08,
});

const concession_valuation: ParamBuilder = () => ({
  concession_years_remaining: 20,
  annual_revenue: 8e7,
  revenue_growth: 0.03,
  opex_margin: 0.35,
  capex_annual: 1e7,
  discount_rate: 0.09,
  terminal_value: 0,
});

const property_valuation: ParamBuilder = () => ({
  gross_rental_income: 5e6,
  vacancy_rate: 0.05,
  operating_expenses: 1.5e6,
  cap_rate: 0.055,
  comparable_prices_psf: [500, 520, 480, 510],
  square_footage: 50000,
  discount_rate: 0.08,
  growth_rate: 0.02,
  holding_period: 10,
});

const clo_tranche_analytics: ParamBuilder = () => ({
  collateral_par: 5e8,
  collateral_spread_bps: 350,
  collateral_default_rate: 0.03,
  recovery_rate: 0.40,
  reinvestment_period: 4,
  tranche: {
    name: 'Class A',
    par: 3e8,
    spread_bps: 130,
    attachment: 0.0,
    detachment: 0.60,
    rating: 'AAA',
  },
});

const tranching_analysis: ParamBuilder = () => ({
  collateral_pool: 1e9,
  weighted_avg_coupon: 0.055,
  weighted_avg_maturity: 5,
  default_rate: 0.03,
  recovery_rate: 0.40,
  prepayment_rate: 0.10,
  tranches: [
    { name: 'Senior', pct: 0.70, coupon: 0.04, rating: 'AAA' },
    { name: 'Mezzanine', pct: 0.20, coupon: 0.06, rating: 'BBB' },
    { name: 'Equity', pct: 0.10, coupon: 0.0, rating: 'NR' },
  ],
});

const recovery_analysis: ParamBuilder = (m) => ({
  enterprise_value: m.enterprise_value ?? 8e8,
  liquidation_value: (m.enterprise_value ?? 8e8) * 0.5,
  valuation_type: 'Both',
  claims: [
    { name: 'DIP', amount: 5e7, priority: 'SuperPriority', is_secured: true },
    { name: 'First Lien', amount: 3e8, priority: 'SecuredFirst', is_secured: true, collateral_value: 3.5e8 },
    { name: 'Senior Unsecured', amount: 2e8, priority: 'Senior', is_secured: false },
    { name: 'Sub Debt', amount: 1e8, priority: 'Subordinated', is_secured: false },
    { name: 'Equity', amount: 2e8, priority: 'Equity', is_secured: false },
  ],
  administrative_costs: 1e7,
  cash_on_hand: 5e7,
});

const fof_portfolio: ParamBuilder = () => ({
  funds: [
    { name: 'Buyout Fund I', strategy: 'Buyout', vintage: 2020, commitment: 5e7, called_pct: 0.80, nav: 6e7, distributions: 1e7 },
    { name: 'Growth Fund II', strategy: 'Growth', vintage: 2021, commitment: 3e7, called_pct: 0.60, nav: 2.5e7, distributions: 0 },
    { name: 'Venture Fund III', strategy: 'Venture', vintage: 2022, commitment: 2e7, called_pct: 0.40, nav: 1.2e7, distributions: 0 },
  ],
  total_commitment: 1e8,
});

const euler_allocation: ParamBuilder = () => ({
  portfolio_risk: 0.15,
  positions: [
    { name: 'Position A', weight: 0.40, marginal_risk: 0.08 },
    { name: 'Position B', weight: 0.35, marginal_risk: 0.05 },
    { name: 'Position C', weight: 0.25, marginal_risk: 0.02 },
  ],
});

const wealth_transfer: ParamBuilder = () => ({
  estate_value: 5e7,
  annual_income: 2e6,
  tax_bracket: 0.37,
  estate_tax_rate: 0.40,
  gift_tax_exemption: 1.292e7,
  annual_gift_exclusion: 18000,
  beneficiaries: 3,
  trust_types: ['GRAT', 'ILIT', 'FLP'],
  time_horizon: 20,
  growth_rate: 0.07,
  discount_rate: 0.04,
});

const payout_sustainability: ParamBuilder = (m) => ({
  net_income: m.net_income ?? 1e8,
  dividends_paid: (m.net_income ?? 1e8) * 0.4,
  fcf: (m.operating_cash_flow ?? 1.5e8) - (m.capex ?? 5e7),
  total_debt: m.total_debt ?? 5e8,
  ebitda: m.ebitda ?? 2e8,
  historical_payout_ratios: [0.35, 0.38, 0.40, 0.42],
  growth_rate: m.growth_rate ?? 0.05,
});

const accrual_quality: ParamBuilder = (m) => {
  const rev = m.revenue ?? 1e9;
  const ta = m.total_assets ?? 2e9;
  return {
    net_income: m.net_income ?? rev * 0.1,
    cfo: m.operating_cash_flow ?? rev * 0.12,
    total_assets: ta, prior_total_assets: ta * 0.95,
    current_assets: ta * 0.25, prior_current_assets: ta * 0.24,
    current_liabilities: ta * 0.15, prior_current_liabilities: ta * 0.145,
    depreciation: m.depreciation ?? rev * 0.03,
    revenue: rev, prior_revenue: rev * 0.95,
    ppe: ta * 0.3, prior_ppe: ta * 0.29,
  };
};

// ─── Three Statement ─────────────────────────────────────────────────────────

const three_statement_model: ParamBuilder = (m) => ({
  base_revenue: m.revenue ?? 1e9,
  revenue_growth_rates: [0.08, 0.07, 0.06, 0.05, 0.04],
  cogs_pct: m.cogs && m.revenue ? m.cogs / m.revenue : 0.60,
  sga_pct: 0.15,
  rnd_pct: 0.05,
  da_pct: 0.10,
  interest_rate: m.cost_of_debt ?? 0.05,
  tax_rate: m.tax_rate ?? 0.21,
  base_cash: m.cash ?? 1e8,
  base_receivables: m.receivables ?? (m.revenue ?? 1e9) * 0.12,
  base_inventory: m.inventory ?? (m.revenue ?? 1e9) * 0.08,
  base_payables: m.payables ?? (m.revenue ?? 1e9) * 0.07,
  base_ppe: m.ppe ?? (m.total_assets ?? 2e9) * 0.3,
  base_debt: m.total_debt ?? 5e8,
  base_equity: m.total_equity ?? 8e8,
  dso_days: 45,
  dio_days: 30,
  dpo_days: 35,
  capex_pct: 0.05,
  debt_repayment_pct: 0.05,
  dividend_payout_ratio: 0.30,
  min_cash_balance: 5e7,
});

// ─── Registry ────────────────────────────────────────────────────────────────

const BUILDERS: Record<string, ParamBuilder> = {
  // Valuation
  dcf_model, wacc_calculator, comps_analysis, sotp_valuation,
  // Credit
  credit_metrics, credit_scorecard, credit_spreads, merton_pd,
  portfolio_credit_risk, cds_pricing, distressed_debt_analysis,
  beneish_mscore, camels_rating, covenant_compliance,
  // Fixed Income
  bond_pricer, bootstrap_spot_curve, short_rate_model, tips_analytics,
  repo_analytics, prepayment_analysis, municipal_analysis, sovereign_bond_analysis,
  // Derivatives
  option_pricer, implied_vol_surface, sabr_calibration, monte_carlo_simulation,
  convertible_bond_pricing, structured_note_pricing, real_option_valuation,
  // Quant Risk
  tail_risk_analysis, mean_variance_optimization, risk_parity,
  factor_model, stress_test, factor_attribution, brinson_attribution,
  momentum_analysis, index_weighting, spread_analysis,
  // Macro
  monetary_policy, international_economics, fx_forward, cross_rate,
  commodity_spread, commodity_forward, country_risk_premium, em_bond_analysis,
  country_risk_assessment, letter_of_credit, carbon_credit_pricing,
  // ESG & Regulatory
  esg_score, carbon_footprint, offset_valuation, best_execution,
  regulatory_capital, kyc_risk_assessment, sanctions_screening,
  entity_classification, treaty_network, intercompany_pricing,
  economic_substance, aifmd_reporting,
  // Private Markets
  lbo_model, returns_calculator, funding_round, dilution_analysis,
  merger_model, ppp_model, concession_valuation, property_valuation,
  clo_tranche_analytics, tranching_analysis, recovery_analysis,
  fof_portfolio, euler_allocation, wealth_transfer,
  // Cross-cutting
  three_statement_model, payout_sustainability, accrual_quality,
};

/**
 * Build valid tool parameters for a given tool name using extracted metrics.
 * Accepts both agent-constructed names and MCP names.
 */
export function buildToolParams(
  toolName: string,
  metrics: ExtractedMetrics,
): Record<string, unknown> {
  // Resolve agent name → MCP name
  const mcpName = resolveToolName(toolName);
  const builder = BUILDERS[mcpName];
  if (builder) return builder(metrics);

  // Fallback: return metrics as-is (will likely fail validation but at least has data)
  const { _raw, _company, ...rest } = metrics;
  return rest as Record<string, unknown>;
}
