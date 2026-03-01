// Valuation param builders: DCF, WACC, Comps, SOTP

import type { ParamBuilder } from './types.js';

export const dcf_model: ParamBuilder = (m) => {
  const revenue = m.revenue ?? (m.ebitda ? (m.ebitda / (m.ebitda_margin ?? 0.15)) : undefined);
  return {
    base_revenue: revenue ?? 1e9,
    revenue_growth_rates: m.growth_rate
      ? [m.growth_rate, m.growth_rate * 0.9, m.growth_rate * 0.8, m.growth_rate * 0.7, m.growth_rate * 0.6]
      : [0.08, 0.07, 0.06, 0.05, 0.04],
    ebitda_margin: m.ebitda_margin ?? (m.ebitda && m.revenue ? m.ebitda / m.revenue : 0.20),
    capex_as_pct_revenue: m.capex && m.revenue ? m.capex / m.revenue : 0.05,
    nwc_as_pct_revenue: 0.02,
    tax_rate: m.tax_rate ?? 0.21,
    wacc: m.wacc ?? 0.10,
    terminal_method: 'GordonGrowth',
    terminal_growth_rate: m.terminal_growth ?? 0.025,
    currency: 'USD',
    net_debt: m.net_debt,
    shares_outstanding: m.shares_outstanding,
  };
};

export const wacc_calculator: ParamBuilder = (m) => ({
  risk_free_rate: m.risk_free_rate ?? 0.04,
  equity_risk_premium: 0.055,
  beta: m.beta ?? 1.0,
  cost_of_debt: m.cost_of_debt ?? 0.05,
  tax_rate: m.tax_rate ?? 0.21,
  debt_weight: m.debt_to_equity ? m.debt_to_equity / (1 + m.debt_to_equity) : 0.30,
  equity_weight: m.debt_to_equity ? 1 / (1 + m.debt_to_equity) : 0.70,
});

export const comps_analysis: ParamBuilder = (m) => ({
  target_name: m._company ?? 'Target Company',
  target_metrics: {
    revenue: m.revenue,
    ebitda: m.ebitda,
    net_income: m.net_income,
    share_price: m.share_price,
    market_cap: m.market_cap,
    enterprise_value: m.enterprise_value,
  },
  comparables: [],
  multiples: ['EvEbitda', 'EvRevenue', 'PriceEarnings'],
  currency: 'USD',
});

export const sotp_valuation: ParamBuilder = (m) => ({
  company_name: m._company ?? 'Target Company',
  segments: [
    { name: 'Core Business', revenue: m.revenue, ebitda: m.ebitda, ebit: m.ebit, method: 'EvEbitda', multiple: 10 },
  ],
  net_debt: m.net_debt,
  shares_outstanding: m.shares_outstanding,
});
