// Build properly-typed tool parameters from extracted financial metrics
// Each builder returns params that match the MCP server's Zod schema

import type { ExtractedMetrics } from './financial-parser.js';
import { resolveToolName } from '../config/tool-name-resolver.js';

type ParamBuilder = (m: ExtractedMetrics) => Record<string, unknown>;

/** Track which param values are real data vs estimates */
export interface ParamQuality {
  realFields: string[];
  estimatedFields: string[];
  missingCriticalFields: string[];
}

export function trackQuality(m: ExtractedMetrics, required: string[], optional: string[]): ParamQuality {
  const quality: ParamQuality = { realFields: [], estimatedFields: [], missingCriticalFields: [] };

  const mRec = m as unknown as Record<string, unknown>;

  for (const field of required) {
    const val = mRec[field];
    if (val !== undefined && val !== null) {
      quality.realFields.push(field);
    } else {
      quality.missingCriticalFields.push(field);
    }
  }

  for (const field of optional) {
    const val = mRec[field];
    if (val !== undefined && val !== null) {
      quality.realFields.push(field);
    } else {
      quality.estimatedFields.push(field);
    }
  }

  return quality;
}

// ─── Valuation ───────────────────────────────────────────────────────────────

const dcf_model: ParamBuilder = (m) => {
  const revenue = m.revenue ?? (m.ebitda ? (m.ebitda / (m.ebitda_margin ?? 0.15)) : undefined);
  return {
    base_revenue: revenue ?? 1e9, // estimate: no FMP data
    revenue_growth_rates: m.growth_rate
      ? [m.growth_rate, m.growth_rate * 0.9, m.growth_rate * 0.8, m.growth_rate * 0.7, m.growth_rate * 0.6]
      : [0.08, 0.07, 0.06, 0.05, 0.04], // estimate: no FMP data
    ebitda_margin: m.ebitda_margin ?? (m.ebitda && m.revenue ? m.ebitda / m.revenue : 0.20), // estimate: no FMP data
    capex_as_pct_revenue: m.capex && m.revenue ? m.capex / m.revenue : 0.05, // estimate: no FMP data
    nwc_as_pct_revenue: 0.02,
    tax_rate: m.tax_rate ?? 0.21, // estimate: no FMP data
    wacc: m.wacc ?? 0.10, // estimate: no FMP data
    terminal_method: 'GordonGrowth',
    terminal_growth_rate: m.terminal_growth ?? 0.025, // estimate: no FMP data
    currency: 'USD',
    net_debt: m.net_debt,
    shares_outstanding: m.shares_outstanding,
  };
};

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
    revenue: m.revenue,
    ebitda: m.ebitda,
    net_income: m.net_income,
    share_price: m.share_price,
    market_cap: m.market_cap,
    enterprise_value: m.enterprise_value,
  },
  // No fake peers — the MCP tool will handle peer discovery
  // or the orchestrator should provide real peer data
  comparables: [],
  multiples: ['EvEbitda', 'EvRevenue', 'PriceEarnings'],
  currency: 'USD',
});

const sotp_valuation: ParamBuilder = (m) => ({
  company_name: m._company ?? 'Target Company',
  segments: [
    { name: 'Core Business', revenue: m.revenue, ebitda: m.ebitda, ebit: m.ebit, method: 'EvEbitda', multiple: 10 },
  ],
  net_debt: m.net_debt,
  shares_outstanding: m.shares_outstanding,
});

// ─── Credit ──────────────────────────────────────────────────────────────────

const credit_metrics: ParamBuilder = (m) => ({
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
  retained_earnings: undefined, // requires historical data
  working_capital: m.current_assets && m.current_liabilities ? m.current_assets - m.current_liabilities : undefined,
  operating_cash_flow: m.operating_cash_flow,
  capex: m.capex,
});

const credit_scorecard: ParamBuilder = (_m) => ({
  // These bins require real historical data — pass empty to signal the tool
  // should use its internal defaults rather than fabricated data
  bins: [],
  target_score: 600,
  target_odds: 50,
  pdo: 20,
});

const credit_spreads: ParamBuilder = (m) => ({
  face_value: m.face_value ?? 1000, // estimate: no FMP data
  coupon_rate: m.coupon_rate ?? 0.05, // estimate: no FMP data
  coupon_frequency: 2,
  market_price: undefined, // requires real market data
  years_to_maturity: m.maturity_years ?? 5, // estimate: no FMP data
  // Benchmark curve requires real market data — pass empty to signal the tool
  // should fetch current rates
  benchmark_curve: [],
  recovery_rate: m.recovery_rate ?? 0.4, // estimate: no FMP data
});

const merton_pd: ParamBuilder = (m) => ({
  equity_value: m.market_cap,
  equity_vol: m.volatility ?? 0.30, // estimate: no FMP data
  debt_face: m.total_debt,
  risk_free_rate: m.risk_free_rate ?? 0.04, // estimate: no FMP data
  maturity: m.maturity_years ?? 1, // estimate: no FMP data
  growth_rate: m.growth_rate,
});

const portfolio_credit_risk: ParamBuilder = (_m) => ({
  exposures: [],
  default_correlation: 0.20,
  confidence_level: 0.99,
  num_simulations: 10000,
});

const cds_pricing: ParamBuilder = (m) => ({
  notional: m.total_debt,
  spread_bps: undefined, // requires real market data
  recovery_rate: m.recovery_rate ?? 0.4, // estimate: no FMP data
  maturity_years: m.maturity_years ?? 5, // estimate: no FMP data
  risk_free_rate: m.risk_free_rate ?? 0.04, // estimate: no FMP data
  payment_frequency: 4,
});

const distressed_debt_analysis: ParamBuilder = (m) => ({
  enterprise_value: m.enterprise_value,
  exit_enterprise_value: undefined, // requires projection assumptions
  exit_timeline_years: 2,
  // Capital structure requires real deal data — pass empty
  capital_structure: [],
  proposed_treatment: [],
  operating_assumptions: {
    annual_ebitda: m.ebitda,
    maintenance_capex: m.capex,
    working_capital_change: undefined, // requires historical data
    restructuring_costs: undefined, // requires deal-specific data
  },
});

const beneish_mscore: ParamBuilder = (m) => {
  // Beneish M-Score requires two years of data
  // Only populate fields we actually have; pass undefined for missing
  return {
    current_receivables: m.receivables,
    prior_receivables: undefined, // requires historical data
    current_revenue: m.revenue,
    prior_revenue: undefined, // requires historical data
    current_cogs: m.cogs,
    prior_cogs: undefined, // requires historical data
    current_total_assets: m.total_assets,
    prior_total_assets: undefined, // requires historical data
    current_ppe: m.ppe,
    prior_ppe: undefined, // requires historical data
    current_depreciation: m.depreciation,
    prior_depreciation: undefined, // requires historical data
    current_sga: m.sga,
    prior_sga: undefined, // requires historical data
    current_total_debt: m.total_debt,
    prior_total_debt: undefined, // requires historical data
    current_net_income: m.net_income,
    current_cfo: m.operating_cash_flow,
  };
};

const camels_rating: ParamBuilder = (m) => ({
  tier1_capital: m.total_equity, // estimate: using total_equity as proxy
  total_capital: m.total_equity,
  risk_weighted_assets: m.total_assets, // estimate: using total_assets as proxy
  npl_ratio: undefined, // requires bank-specific data
  provision_coverage: undefined, // requires bank-specific data
  management_score: undefined, // requires qualitative assessment
  roa: m.net_income && m.total_assets ? m.net_income / m.total_assets : undefined,
  roe: m.net_income && m.total_equity ? m.net_income / m.total_equity : undefined,
  nim: undefined, // requires bank-specific data
  liquid_assets: m.cash,
  total_deposits: undefined, // requires bank-specific data
  rate_sensitivity_gap: undefined, // requires bank-specific data
});

const covenant_compliance: ParamBuilder = (m) => {
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
      quick_ratio: undefined, // requires detailed current asset breakdown
      cash_to_debt: m.cash && debt ? m.cash / debt : undefined,
      implied_rating: undefined, // requires credit model output
      rating_rationale: [],
    },
  };
};

// ─── Fixed Income ────────────────────────────────────────────────────────────

const bond_pricer: ParamBuilder = (m) => ({
  face_value: m.face_value ?? 1000, // estimate: no FMP data
  coupon_rate: m.coupon_rate ?? 0.05, // estimate: no FMP data
  coupon_frequency: 2,
  ytm: m.ytm ?? m.yield,
  settlement_date: undefined, // requires real settlement date
  maturity_date: undefined, // requires real maturity date
  day_count: 'Thirty360',
});

const bootstrap_spot_curve: ParamBuilder = (_m) => ({
  // Par instruments require real market data — pass empty to signal the tool
  // should fetch current rates
  par_instruments: [],
});

const short_rate_model: ParamBuilder = (m) => ({
  model_type: 'Vasicek',
  current_rate: m.risk_free_rate ?? 0.04, // estimate: no FMP data
  mean_reversion_speed: 0.3,
  long_term_mean: 0.045,
  volatility: 0.015,
  time_horizon: 5,
  num_steps: 252,
  num_paths: 1000,
});

const tips_analytics: ParamBuilder = (_m) => ({
  face_value: 1000,
  real_coupon_rate: 0.0125,
  coupon_frequency: 2,
  years_to_maturity: 10,
  current_cpi: 310,
  base_cpi: 280,
  nominal_yield: 0.045,
  real_yield: 0.015,
});

const repo_analytics: ParamBuilder = (_m) => ({
  collateral_value: 1e7,
  repo_rate: 0.04,
  haircut: 0.02,
  term_days: 30,
  collateral_type: 'Treasury',
});

const prepayment_analysis: ParamBuilder = (_m) => ({
  original_balance: 1e6,
  coupon_rate: 0.055,
  wac: 0.055,
  wam_months: 300,
  current_rate: 0.045,
  seasoning_months: 36,
  model_type: 'PSA',
  psa_speed: 150,
});

const municipal_analysis: ParamBuilder = (_m) => ({
  par_value: 1e6,
  coupon_rate: 0.04,
  yield: 0.035,
  maturity_years: 20,
  tax_bracket: 0.37,
  state_tax_rate: 0.05,
  credit_rating: 'AA',
  insurance_premium_bps: 10,
});

const sovereign_bond_analysis: ParamBuilder = (_m) => ({
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
  spot_price: m.share_price,
  strike_price: m.share_price ? m.share_price * 1.05 : undefined,
  time_to_expiry: 0.5,
  risk_free_rate: m.risk_free_rate ?? 0.04, // estimate: no FMP data
  volatility: m.volatility ?? 0.25, // estimate: no FMP data
  option_type: 'Call',
  model: 'BlackScholes',
  dividend_yield: undefined, // requires real dividend data
});

const implied_vol_surface: ParamBuilder = (m) => ({
  spot_price: m.share_price,
  risk_free_rate: m.risk_free_rate ?? 0.04, // estimate: no FMP data
  // Option data requires real market quotes — pass empty to signal the tool
  // should fetch or the orchestrator should provide real option chain data
  option_data: [],
});

const sabr_calibration: ParamBuilder = (m) => ({
  forward: m.share_price,
  expiry: 0.5,
  // Market vols require real option market data — pass empty
  market_vols: [],
  beta: 0.5,
});

const monte_carlo_simulation: ParamBuilder = (m) => ({
  initial_value: m.share_price ?? m.enterprise_value,
  drift: m.growth_rate ?? 0.05, // estimate: no FMP data
  volatility: m.volatility ?? 0.25, // estimate: no FMP data
  time_horizon: 1.0,
  num_paths: 10000,
  num_steps: 252,
});

const convertible_bond_pricing: ParamBuilder = (m) => ({
  face_value: m.face_value ?? 1000, // estimate: no FMP data
  coupon_rate: m.coupon_rate,
  maturity_years: m.maturity_years,
  conversion_ratio: undefined, // requires bond-specific data
  stock_price: m.share_price,
  stock_volatility: m.volatility ?? 0.30, // estimate: no FMP data
  risk_free_rate: m.risk_free_rate ?? 0.04, // estimate: no FMP data
  credit_spread: undefined, // requires real market data
  dividend_yield: undefined, // requires real dividend data
});

const structured_note_pricing: ParamBuilder = (m) => ({
  face_value: m.face_value ?? 1000, // estimate: no FMP data
  maturity_years: m.maturity_years,
  coupon_rate: m.coupon_rate,
  underlying_price: m.share_price,
  underlying_volatility: m.volatility ?? 0.25, // estimate: no FMP data
  risk_free_rate: m.risk_free_rate ?? 0.04, // estimate: no FMP data
  barrier_level: 0.7,
  participation_rate: 1.5,
  product_type: 'ReverseConvertible',
});

const real_option_valuation: ParamBuilder = (m) => ({
  underlying_value: m.enterprise_value,
  strike_price: m.enterprise_value ? m.enterprise_value * 0.9 : undefined,
  time_to_expiry: 3,
  risk_free_rate: m.risk_free_rate ?? 0.04, // estimate: no FMP data
  volatility: m.volatility ?? 0.30, // estimate: no FMP data
  option_type: 'DeferralOption',
  steps: 100,
});

// ─── Quant Risk ──────────────────────────────────────────────────────────────

const tail_risk_analysis: ParamBuilder = (_m) => ({
  // Portfolio data requires real holdings — pass empty to signal the tool
  // should use provided portfolio or fetch data
  asset_names: [],
  weights: [],
  expected_returns: [],
  covariance_matrix: [],
  confidence_level: 0.95,
  time_horizon: 1 / 252,
  distribution: 'Normal',
  portfolio_value: undefined, // requires real portfolio value
});

const mean_variance_optimization: ParamBuilder = (m) => ({
  // Portfolio optimization requires real asset data — pass empty
  asset_names: [],
  expected_returns: [],
  covariance_matrix: [],
  risk_free_rate: m.risk_free_rate ?? 0.04, // estimate: no FMP data
  target_return: undefined, // requires investor-specified target
  constraints: { long_only: true },
});

const risk_parity: ParamBuilder = (m) => ({
  // Asset data requires real portfolio holdings
  assets: [],
  covariance_matrix: [],
  method: 'EqualRiskContribution',
  risk_free_rate: m.risk_free_rate ?? 0.04, // estimate: no FMP data
});

const factor_model: ParamBuilder = (m) => ({
  // Real factor analysis requires historical returns data
  // Pass empty arrays — the tool should fetch or signal missing data
  asset_returns: [],
  factor_returns: [],
  model_type: 'FamaFrench3',
  risk_free_rate: m.risk_free_rate ?? 0.04, // estimate: no FMP data
});

const stress_test: ParamBuilder = (_m) => ({
  // Portfolio requires real holdings data
  portfolio: [],
  // Keep standard stress scenarios as structural config
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

const factor_attribution: ParamBuilder = (m) => ({
  portfolio_name: m._company ?? 'Portfolio',
  portfolio_return: undefined, // requires real portfolio return data
  benchmark_return: undefined, // requires real benchmark data
  // Factor exposures require real regression analysis
  factors: [],
  risk_free_rate: m.risk_free_rate ?? 0.04, // estimate: no FMP data
});

const brinson_attribution: ParamBuilder = (m) => ({
  portfolio_name: m._company ?? 'Portfolio',
  benchmark_name: undefined, // requires real benchmark specification
  // Sector data requires real portfolio and benchmark holdings
  sectors: [],
  risk_free_rate: m.risk_free_rate ?? 0.04, // estimate: no FMP data
});

const momentum_analysis: ParamBuilder = (_m) => ({
  asset_returns: [],
  lookback_months: 12,
  holding_months: 1,
  skip_months: 1,
});

const index_weighting: ParamBuilder = (_m) => ({
  // Constituents require real index/portfolio data
  constituents: [],
  methodology: 'MarketCapWeighted',
  max_weight: 0.30,
  min_weight: 0.01,
});

const spread_analysis: ParamBuilder = (_m) => ({
  // Spread analysis requires real market microstructure data
  bid_prices: [],
  ask_prices: [],
  trade_volumes: [],
  timestamps: [],
});

// ─── Macro ───────────────────────────────────────────────────────────────────

const monetary_policy: ParamBuilder = (_m) => ({
  policy_rate: 0.0525,
  inflation_rate: 0.032,
  inflation_target: 0.02,
  output_gap: -0.01,
  unemployment_rate: 0.039,
  natural_rate: 0.04,
  gdp_growth: 0.025,
});

const international_economics: ParamBuilder = (_m) => ({
  country: 'US',
  gdp_growth: 0.025,
  inflation: 0.032,
  trade_balance_pct_gdp: -0.03,
  current_account_pct_gdp: -0.035,
  fiscal_balance_pct_gdp: -0.06,
  debt_to_gdp: 1.2,
  policy_rate: 0.0525,
});

const fx_forward: ParamBuilder = (_m) => ({
  spot_rate: 1.08,
  domestic_rate: 0.0525,
  foreign_rate: 0.04,
  maturity_days: 90,
  notional: 1e6,
  currency_pair: 'EURUSD',
});

const cross_rate: ParamBuilder = (_m) => ({
  rate1: 1.08,
  rate2: 149.5,
  pair1: 'EURUSD',
  pair2: 'USDJPY',
  target_pair: 'EURJPY',
});

const commodity_spread: ParamBuilder = (_m) => ({
  front_price: 75.50,
  back_price: 77.20,
  front_expiry_months: 1,
  back_expiry_months: 3,
  commodity: 'WTI Crude',
  contract_size: 1000,
});

const commodity_forward: ParamBuilder = (_m) => ({
  spot_price: 75.50,
  risk_free_rate: 0.04,
  storage_cost_rate: 0.02,
  convenience_yield: 0.01,
  maturity_years: 0.5,
});

const country_risk_premium: ParamBuilder = (_m) => ({
  country: 'Brazil',
  sovereign_spread_bps: 250,
  equity_market_vol: 0.25,
  bond_market_vol: 0.10,
  default_probability: 0.02,
  country_credit_rating: 'BB+',
});

const em_bond_analysis: ParamBuilder = (_m) => ({
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

const country_risk_assessment: ParamBuilder = (_m) => ({
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

const letter_of_credit: ParamBuilder = (_m) => ({
  amount: 1e6,
  currency: 'USD',
  tenor_days: 90,
  issuing_bank_rating: 'A',
  type: 'Irrevocable',
  margin_pct: 0.015,
});

const carbon_credit_pricing: ParamBuilder = (_m) => ({
  credit_type: 'EUA',
  volume_tonnes: 10000,
  vintage_year: 2024,
  spot_price: 80,
  delivery_date: '2025-12-15',
  risk_free_rate: 0.04,
  volatility: 0.30,
});

// ─── ESG & Regulatory ────────────────────────────────────────────────────────

const esg_score: ParamBuilder = (_m) => ({
  environmental: { carbon_intensity: 150, renewable_pct: 0.3, waste_recycled_pct: 0.6, water_intensity: 50 },
  social: { diversity_pct: 0.4, turnover_rate: 0.12, injury_rate: 1.5, community_investment: 5e6 },
  governance: { board_independence: 0.75, women_on_board: 0.35, ceo_pay_ratio: 150, audit_committee_meetings: 8 },
  weights: { environmental: 0.33, social: 0.33, governance: 0.34 },
});

const carbon_footprint: ParamBuilder = (m) => ({
  scope1_emissions: undefined, // requires ESG-specific data
  scope2_emissions: undefined, // requires ESG-specific data
  scope3_emissions: undefined, // requires ESG-specific data
  revenue: m.revenue,
  portfolio_weight: undefined, // requires portfolio context
  carbon_price: 80,
});

const offset_valuation: ParamBuilder = (_m) => ({
  project_type: 'ForestConservation',
  volume_tonnes: 5000,
  vintage_year: 2024,
  registry: 'Verra',
  permanence_years: 25,
  additionality_score: 0.8,
  co_benefits: ['Biodiversity', 'Community'],
});

const best_execution: ParamBuilder = (_m) => ({
  // Trade data requires real execution records
  trades: [],
  market_impact_model: 'SquareRoot',
});

const regulatory_capital: ParamBuilder = (m) => ({
  risk_weighted_assets: m.total_assets, // estimate: using total_assets as proxy
  tier1_capital: m.total_equity, // estimate: using total_equity as proxy
  tier2_capital: undefined, // requires regulatory-specific data
  total_capital: m.total_equity, // estimate: using total_equity as proxy
  minimum_cet1_ratio: 0.045,
  capital_conservation_buffer: 0.025,
  countercyclical_buffer: 0.01,
  systemic_buffer: 0.0,
});

const kyc_risk_assessment: ParamBuilder = (m) => ({
  entity_type: 'Corporation',
  jurisdiction: 'US',
  industry: m._industry ?? m._sector,
  annual_revenue: m.revenue,
  pep_status: false,
  sanctions_match: false,
  adverse_media: false,
  risk_factors: [],
});

const sanctions_screening: ParamBuilder = (m) => ({
  entity_name: m._company ?? 'Unknown Entity',
  aliases: [],
  jurisdiction: 'US',
  screening_lists: ['OFAC_SDN', 'EU_SANCTIONS', 'UN_CONSOLIDATED'],
  fuzzy_threshold: 0.85,
});

const entity_classification: ParamBuilder = (_m) => ({
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

const treaty_network: ParamBuilder = (_m) => ({
  source_country: 'US',
  recipient_country: 'UK',
  income_type: 'Dividend',
  recipient_type: 'Corporation',
  holding_pct: 0.25,
});

const intercompany_pricing: ParamBuilder = (_m) => ({
  transaction_type: 'ServiceFee',
  transfer_price: 1e6,
  comparable_prices: [8e5, 9e5, 1.1e6, 1.2e6],
  method: 'ComparableUncontrolledPrice',
  entity_a_jurisdiction: 'US',
  entity_b_jurisdiction: 'Ireland',
});

const economic_substance: ParamBuilder = (_m) => ({
  entity_jurisdiction: 'Cayman Islands',
  relevant_activities: ['Holding', 'FinanceLeasing'],
  employees: 5,
  office_space_sqft: 500,
  annual_expenditure: 5e5,
  board_meetings_per_year: 4,
  strategic_decisions_local: true,
});

const aifmd_reporting: ParamBuilder = (_m) => ({
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
  purchase_ev: m.enterprise_value,
  ebitda: m.ebitda,
  entry_multiple: m.enterprise_value && m.ebitda ? m.enterprise_value / m.ebitda : undefined,
  exit_multiple: m.enterprise_value && m.ebitda ? (m.enterprise_value / m.ebitda) + 1 : undefined,
  holding_period: 5,
  revenue_growth: m.growth_rate ?? 0.05, // estimate: no FMP data
  margin_expansion: 0.01,
  debt_to_ebitda: 4.0,
  senior_debt_pct: 0.60,
  sub_debt_pct: 0.20,
  equity_pct: 0.20,
  senior_rate: 0.05,
  sub_rate: 0.08,
  tax_rate: m.tax_rate ?? 0.21, // estimate: no FMP data
  capex_pct_revenue: m.capex && m.revenue ? m.capex / m.revenue : 0.03, // estimate: no FMP data
  nwc_pct_revenue: 0.02,
  debt_paydown_pct: 0.10,
});

const returns_calculator: ParamBuilder = (_m) => ({
  entry_equity: 2e8,
  exit_equity: 5e8,
  holding_period: 5,
  distributions: [0, 0, 1e7, 2e7, 5e8],
  management_fee_pct: 0.02,
  carry_pct: 0.20,
  preferred_return: 0.08,
  hurdle_type: 'European',
});

const funding_round: ParamBuilder = (_m) => ({
  pre_money_valuation: 5e7,
  raise_amount: 1e7,
  round_type: 'SeriesA',
  existing_shares: 1e7,
  option_pool_pct: 0.10,
  liquidation_preference: 1.0,
  participation: true,
  participation_cap: 3.0,
});

const dilution_analysis: ParamBuilder = (_m) => ({
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
    // Acquirer data must be provided by the orchestrator — not available from target metrics
    name: undefined,
    share_price: undefined,
    shares_outstanding: undefined,
    eps: undefined,
    pe_ratio: undefined,
    net_income: undefined,
    tax_rate: m.tax_rate ?? 0.21, // estimate: no FMP data
  },
  target: {
    name: m._company ?? 'Target Co',
    share_price: m.share_price,
    shares_outstanding: m.shares_outstanding,
    eps: m.eps,
    pe_ratio: m.share_price && m.eps ? m.share_price / m.eps : undefined,
    net_income: m.net_income,
  },
  offer_price_per_share: m.share_price ? m.share_price * 1.25 : undefined,
  pct_cash: 0.5,
  pct_stock: 0.5,
  synergies: undefined, // requires deal-specific estimate
  integration_costs: undefined, // requires deal-specific estimate
  cost_of_debt: m.cost_of_debt ?? 0.05, // estimate: no FMP data
});

const ppp_model: ParamBuilder = (_m) => ({
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

const concession_valuation: ParamBuilder = (_m) => ({
  concession_years_remaining: 20,
  annual_revenue: 8e7,
  revenue_growth: 0.03,
  opex_margin: 0.35,
  capex_annual: 1e7,
  discount_rate: 0.09,
  terminal_value: 0,
});

const property_valuation: ParamBuilder = (_m) => ({
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

const clo_tranche_analytics: ParamBuilder = (_m) => ({
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

const tranching_analysis: ParamBuilder = (_m) => ({
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
  enterprise_value: m.enterprise_value,
  liquidation_value: undefined, // requires appraisal data
  valuation_type: 'Both',
  // Claims require real capital structure data
  claims: [],
  administrative_costs: undefined, // requires deal-specific estimate
  cash_on_hand: m.cash,
});

const fof_portfolio: ParamBuilder = (_m) => ({
  funds: [
    { name: 'Buyout Fund I', strategy: 'Buyout', vintage: 2020, commitment: 5e7, called_pct: 0.80, nav: 6e7, distributions: 1e7 },
    { name: 'Growth Fund II', strategy: 'Growth', vintage: 2021, commitment: 3e7, called_pct: 0.60, nav: 2.5e7, distributions: 0 },
    { name: 'Venture Fund III', strategy: 'Venture', vintage: 2022, commitment: 2e7, called_pct: 0.40, nav: 1.2e7, distributions: 0 },
  ],
  total_commitment: 1e8,
});

const euler_allocation: ParamBuilder = (_m) => ({
  portfolio_risk: 0.15,
  positions: [
    { name: 'Position A', weight: 0.40, marginal_risk: 0.08 },
    { name: 'Position B', weight: 0.35, marginal_risk: 0.05 },
    { name: 'Position C', weight: 0.25, marginal_risk: 0.02 },
  ],
});

const wealth_transfer: ParamBuilder = (_m) => ({
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
  net_income: m.net_income,
  dividends_paid: undefined, // requires real dividend data
  fcf: m.operating_cash_flow && m.capex ? m.operating_cash_flow - m.capex : undefined,
  total_debt: m.total_debt,
  ebitda: m.ebitda,
  historical_payout_ratios: [], // requires historical data
  growth_rate: m.growth_rate,
});

const accrual_quality: ParamBuilder = (m) => {
  // Accrual quality requires two years of data
  // Only populate fields we actually have; pass undefined for missing
  return {
    net_income: m.net_income,
    cfo: m.operating_cash_flow,
    total_assets: m.total_assets, prior_total_assets: undefined, // requires historical data
    current_assets: m.current_assets, prior_current_assets: undefined, // requires historical data
    current_liabilities: m.current_liabilities, prior_current_liabilities: undefined, // requires historical data
    depreciation: m.depreciation,
    revenue: m.revenue, prior_revenue: undefined, // requires historical data
    ppe: m.ppe, prior_ppe: undefined, // requires historical data
  };
};

// ─── Three Statement ─────────────────────────────────────────────────────────

const three_statement_model: ParamBuilder = (m) => ({
  base_revenue: m.revenue,
  revenue_growth_rates: m.growth_rate
    ? [m.growth_rate, m.growth_rate * 0.9, m.growth_rate * 0.8, m.growth_rate * 0.7, m.growth_rate * 0.6]
    : [0.08, 0.07, 0.06, 0.05, 0.04], // estimate: no FMP data
  cogs_pct: m.cogs && m.revenue ? m.cogs / m.revenue : 0.60, // estimate: no FMP data
  sga_pct: m.sga && m.revenue ? m.sga / m.revenue : 0.15, // estimate: no FMP data
  rnd_pct: 0.05,
  da_pct: m.depreciation && m.revenue ? m.depreciation / m.revenue : 0.10, // estimate: no FMP data
  interest_rate: m.cost_of_debt ?? 0.05, // estimate: no FMP data
  tax_rate: m.tax_rate ?? 0.21, // estimate: no FMP data
  base_cash: m.cash,
  base_receivables: m.receivables,
  base_inventory: m.inventory,
  base_payables: m.payables,
  base_ppe: m.ppe,
  base_debt: m.total_debt,
  base_equity: m.total_equity,
  dso_days: m.receivables && m.revenue ? (m.receivables / m.revenue) * 365 : 45, // estimate: no FMP data
  dio_days: m.inventory && m.cogs ? (m.inventory / m.cogs) * 365 : 30, // estimate: no FMP data
  dpo_days: m.payables && m.cogs ? (m.payables / m.cogs) * 365 : 35, // estimate: no FMP data
  capex_pct: m.capex && m.revenue ? m.capex / m.revenue : 0.05, // estimate: no FMP data
  debt_repayment_pct: 0.05,
  dividend_payout_ratio: 0.30,
  min_cash_balance: undefined, // requires company-specific policy
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
  if (builder) {
    const params = builder(metrics);
    // Strip undefined values to avoid sending nulls to MCP tools
    return Object.fromEntries(
      Object.entries(params).filter(([_, v]) => v !== undefined)
    );
  }

  // Fallback: return metrics as-is (will likely fail validation but at least has data)
  const { _raw, _company, ...rest } = metrics;
  return rest as Record<string, unknown>;
}
