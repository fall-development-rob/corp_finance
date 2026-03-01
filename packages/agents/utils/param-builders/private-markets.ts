// Private Markets param builders: LBO, PE returns, VC, M&A, infrastructure, real estate,
// CLO, securitization, recovery, FoF, capital allocation, wealth, payout, accrual, three-statement

import type { ParamBuilder } from './types.js';

export const lbo_model: ParamBuilder = (m) => ({
  purchase_ev: m.enterprise_value,
  ebitda: m.ebitda,
  entry_multiple: m.enterprise_value && m.ebitda ? m.enterprise_value / m.ebitda : undefined,
  exit_multiple: m.enterprise_value && m.ebitda ? (m.enterprise_value / m.ebitda) + 1 : undefined,
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
  capex_pct_revenue: m.capex && m.revenue ? m.capex / m.revenue : 0.03,
  nwc_pct_revenue: 0.02,
  debt_paydown_pct: 0.10,
});

export const returns_calculator: ParamBuilder = (_m) => ({
  entry_equity: 2e8,
  exit_equity: 5e8,
  holding_period: 5,
  distributions: [0, 0, 1e7, 2e7, 5e8],
  management_fee_pct: 0.02,
  carry_pct: 0.20,
  preferred_return: 0.08,
  hurdle_type: 'European',
});

export const funding_round: ParamBuilder = (_m) => ({
  pre_money_valuation: 5e7,
  raise_amount: 1e7,
  round_type: 'SeriesA',
  existing_shares: 1e7,
  option_pool_pct: 0.10,
  liquidation_preference: 1.0,
  participation: true,
  participation_cap: 3.0,
});

export const dilution_analysis: ParamBuilder = (_m) => ({
  current_shares: 1e7,
  rounds: [
    { name: 'Series A', shares_issued: 2e6, price_per_share: 5 },
    { name: 'Series B', shares_issued: 1.5e6, price_per_share: 15 },
  ],
  option_pool_shares: 1e6,
  founders_shares: 6e6,
});

export const merger_model: ParamBuilder = (m) => ({
  acquirer: {
    name: undefined,
    share_price: undefined,
    shares_outstanding: undefined,
    eps: undefined,
    pe_ratio: undefined,
    net_income: undefined,
    tax_rate: m.tax_rate ?? 0.21,
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
  synergies: undefined,
  integration_costs: undefined,
  cost_of_debt: m.cost_of_debt ?? 0.05,
});

export const ppp_model: ParamBuilder = (_m) => ({
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

export const concession_valuation: ParamBuilder = (_m) => ({
  concession_years_remaining: 20,
  annual_revenue: 8e7,
  revenue_growth: 0.03,
  opex_margin: 0.35,
  capex_annual: 1e7,
  discount_rate: 0.09,
  terminal_value: 0,
});

export const property_valuation: ParamBuilder = (_m) => ({
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

export const clo_tranche_analytics: ParamBuilder = (_m) => ({
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

export const tranching_analysis: ParamBuilder = (_m) => ({
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

export const recovery_analysis: ParamBuilder = (m) => ({
  enterprise_value: m.enterprise_value,
  liquidation_value: undefined,
  valuation_type: 'Both',
  claims: [],
  administrative_costs: undefined,
  cash_on_hand: m.cash,
});

export const fof_portfolio: ParamBuilder = (_m) => ({
  funds: [
    { name: 'Buyout Fund I', strategy: 'Buyout', vintage: 2020, commitment: 5e7, called_pct: 0.80, nav: 6e7, distributions: 1e7 },
    { name: 'Growth Fund II', strategy: 'Growth', vintage: 2021, commitment: 3e7, called_pct: 0.60, nav: 2.5e7, distributions: 0 },
    { name: 'Venture Fund III', strategy: 'Venture', vintage: 2022, commitment: 2e7, called_pct: 0.40, nav: 1.2e7, distributions: 0 },
  ],
  total_commitment: 1e8,
});

export const euler_allocation: ParamBuilder = (_m) => ({
  portfolio_risk: 0.15,
  positions: [
    { name: 'Position A', weight: 0.40, marginal_risk: 0.08 },
    { name: 'Position B', weight: 0.35, marginal_risk: 0.05 },
    { name: 'Position C', weight: 0.25, marginal_risk: 0.02 },
  ],
});

export const wealth_transfer: ParamBuilder = (_m) => ({
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

export const payout_sustainability: ParamBuilder = (m) => ({
  net_income: m.net_income,
  dividends_paid: undefined,
  fcf: m.operating_cash_flow && m.capex ? m.operating_cash_flow - m.capex : undefined,
  total_debt: m.total_debt,
  ebitda: m.ebitda,
  historical_payout_ratios: [],
  growth_rate: m.growth_rate,
});

export const accrual_quality: ParamBuilder = (m) => ({
  net_income: m.net_income,
  cfo: m.operating_cash_flow,
  total_assets: m.total_assets, prior_total_assets: undefined,
  current_assets: m.current_assets, prior_current_assets: undefined,
  current_liabilities: m.current_liabilities, prior_current_liabilities: undefined,
  depreciation: m.depreciation,
  revenue: m.revenue, prior_revenue: undefined,
  ppe: m.ppe, prior_ppe: undefined,
});

export const three_statement_model: ParamBuilder = (m) => ({
  base_revenue: m.revenue,
  revenue_growth_rates: m.growth_rate
    ? [m.growth_rate, m.growth_rate * 0.9, m.growth_rate * 0.8, m.growth_rate * 0.7, m.growth_rate * 0.6]
    : [0.08, 0.07, 0.06, 0.05, 0.04],
  cogs_pct: m.cogs && m.revenue ? m.cogs / m.revenue : 0.60,
  sga_pct: m.sga && m.revenue ? m.sga / m.revenue : 0.15,
  rnd_pct: 0.05,
  da_pct: m.depreciation && m.revenue ? m.depreciation / m.revenue : 0.10,
  interest_rate: m.cost_of_debt ?? 0.05,
  tax_rate: m.tax_rate ?? 0.21,
  base_cash: m.cash,
  base_receivables: m.receivables,
  base_inventory: m.inventory,
  base_payables: m.payables,
  base_ppe: m.ppe,
  base_debt: m.total_debt,
  base_equity: m.total_equity,
  dso_days: m.receivables && m.revenue ? (m.receivables / m.revenue) * 365 : 45,
  dio_days: m.inventory && m.cogs ? (m.inventory / m.cogs) * 365 : 30,
  dpo_days: m.payables && m.cogs ? (m.payables / m.cogs) * 365 : 35,
  capex_pct: m.capex && m.revenue ? m.capex / m.revenue : 0.05,
  debt_repayment_pct: 0.05,
  dividend_payout_ratio: 0.30,
  min_cash_balance: undefined,
});
