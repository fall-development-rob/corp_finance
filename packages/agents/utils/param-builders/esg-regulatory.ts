// ESG & Regulatory param builders: ESG scores, carbon, offsets, best execution, regulatory capital,
// KYC/AML, sanctions, FATCA, treaty, transfer pricing, substance, AIFMD

import type { ParamBuilder } from './types.js';

export const esg_score: ParamBuilder = (_m) => ({
  environmental: { carbon_intensity: 150, renewable_pct: 0.3, waste_recycled_pct: 0.6, water_intensity: 50 },
  social: { diversity_pct: 0.4, turnover_rate: 0.12, injury_rate: 1.5, community_investment: 5e6 },
  governance: { board_independence: 0.75, women_on_board: 0.35, ceo_pay_ratio: 150, audit_committee_meetings: 8 },
  weights: { environmental: 0.33, social: 0.33, governance: 0.34 },
});

export const carbon_footprint: ParamBuilder = (m) => ({
  scope1_emissions: undefined,
  scope2_emissions: undefined,
  scope3_emissions: undefined,
  revenue: m.revenue,
  portfolio_weight: undefined,
  carbon_price: 80,
});

export const offset_valuation: ParamBuilder = (_m) => ({
  project_type: 'ForestConservation',
  volume_tonnes: 5000,
  vintage_year: 2024,
  registry: 'Verra',
  permanence_years: 25,
  additionality_score: 0.8,
  co_benefits: ['Biodiversity', 'Community'],
});

export const best_execution: ParamBuilder = (_m) => ({
  trades: [],
  market_impact_model: 'SquareRoot',
});

export const regulatory_capital: ParamBuilder = (m) => ({
  risk_weighted_assets: m.total_assets,
  tier1_capital: m.total_equity,
  tier2_capital: undefined,
  total_capital: m.total_equity,
  minimum_cet1_ratio: 0.045,
  capital_conservation_buffer: 0.025,
  countercyclical_buffer: 0.01,
  systemic_buffer: 0.0,
});

export const kyc_risk_assessment: ParamBuilder = (m) => ({
  entity_type: 'Corporation',
  jurisdiction: 'US',
  industry: m._industry ?? m._sector,
  annual_revenue: m.revenue,
  pep_status: false,
  sanctions_match: false,
  adverse_media: false,
  risk_factors: [],
});

export const sanctions_screening: ParamBuilder = (m) => ({
  entity_name: m._company ?? 'Unknown Entity',
  aliases: [],
  jurisdiction: 'US',
  screening_lists: ['OFAC_SDN', 'EU_SANCTIONS', 'UN_CONSOLIDATED'],
  fuzzy_threshold: 0.85,
});

export const entity_classification: ParamBuilder = (_m) => ({
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

export const treaty_network: ParamBuilder = (_m) => ({
  source_country: 'US',
  recipient_country: 'UK',
  income_type: 'Dividend',
  recipient_type: 'Corporation',
  holding_pct: 0.25,
});

export const intercompany_pricing: ParamBuilder = (_m) => ({
  transaction_type: 'ServiceFee',
  transfer_price: 1e6,
  comparable_prices: [8e5, 9e5, 1.1e6, 1.2e6],
  method: 'ComparableUncontrolledPrice',
  entity_a_jurisdiction: 'US',
  entity_b_jurisdiction: 'Ireland',
});

export const economic_substance: ParamBuilder = (_m) => ({
  entity_jurisdiction: 'Cayman Islands',
  relevant_activities: ['Holding', 'FinanceLeasing'],
  employees: 5,
  office_space_sqft: 500,
  annual_expenditure: 5e5,
  board_meetings_per_year: 4,
  strategic_decisions_local: true,
});

export const aifmd_reporting: ParamBuilder = (_m) => ({
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
