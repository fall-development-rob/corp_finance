// Tool domain mappings for specialist agents
// Each agent gets a curated subset of the 71 MCP tool modules

export const TOOL_MAPPINGS: Record<string, string[]> = {
  'equity-analyst': [
    'equity_research', 'valuation', 'earnings_quality', 'dividend_policy',
    'behavioral', 'performance_attribution', 'three_statement', 'fpa',
  ],
  'credit-analyst': [
    'credit', 'credit_scoring', 'credit_portfolio', 'credit_derivatives',
    'restructuring', 'financial_forensics', 'three_statement', 'bank_analytics',
  ],
  'fixed-income-analyst': [
    'fixed_income', 'interest_rate_models', 'inflation_linked', 'mortgage_analytics',
    'repo_financing', 'municipal', 'sovereign', 'three_statement',
  ],
  'derivatives-analyst': [
    'derivatives', 'volatility_surface', 'convertibles', 'structured_products',
    'real_options', 'monte_carlo', 'credit_derivatives',
  ],
  'quant-risk-analyst': [
    'quant_risk', 'quant_strategies', 'portfolio_optimization', 'risk_budgeting',
    'market_microstructure', 'index_construction', 'scenarios', 'monte_carlo',
    'portfolio', 'performance_attribution',
  ],
  'macro-analyst': [
    'macro_economics', 'fx_commodities', 'commodity_trading', 'emerging_markets',
    'trade_finance', 'carbon_markets', 'sovereign', 'inflation_linked',
  ],
  'esg-regulatory-analyst': [
    'esg', 'regulatory', 'compliance', 'aml_compliance', 'regulatory_reporting',
    'fatca_crs', 'substance_requirements', 'tax_treaty', 'transfer_pricing',
    'carbon_markets',
  ],
  'private-markets-analyst': [
    'pe', 'venture', 'private_credit', 'private_wealth', 'infrastructure',
    'real_assets', 'fund_of_funds', 'clo_analytics', 'securitization',
    'ma', 'capital_allocation', 'lease_accounting',
  ],
};

// Cross-cutting modules available to all specialists on request
export const CROSS_CUTTING_MODULES = [
  'three_statement', 'fpa', 'portfolio', 'treasury', 'ma',
  'capital_allocation', 'insurance', 'pension', 'wealth',
  'bank_analytics', 'lease_accounting', 'onshore_structures',
  'offshore_structures', 'jurisdiction', 'crypto', 'scenarios',
];

// Agent type to human-readable description
export const AGENT_DESCRIPTIONS: Record<string, string> = {
  'equity-analyst': 'Equity research specialist: DCF, comps, multiples, earnings quality, dividend policy',
  'credit-analyst': 'Credit analysis specialist: ratings, spreads, default probability, covenants, restructuring',
  'fixed-income-analyst': 'Fixed income specialist: rates, yield curves, mortgage analytics, municipal, sovereign debt',
  'derivatives-analyst': 'Derivatives & volatility specialist: options, vol surfaces, convertibles, structured products, Monte Carlo',
  'quant-risk-analyst': 'Quantitative risk specialist: VaR, Greeks, factor models, portfolio optimization, risk budgeting',
  'macro-analyst': 'Macro strategist: rates, FX, commodities, emerging markets, trade finance, sovereign analysis',
  'esg-regulatory-analyst': 'ESG & regulatory specialist: ESG scores, carbon markets, compliance, AML, FATCA/CRS, tax treaties',
  'private-markets-analyst': 'Private markets specialist: PE, venture, LBO, M&A, infrastructure, real assets, CLO/securitization',
};

// Given a query, suggest which agent types are needed
export function suggestAgents(queryDomains: string[]): string[] {
  const agents = new Set<string>();
  for (const domain of queryDomains) {
    for (const [agentType, tools] of Object.entries(TOOL_MAPPINGS)) {
      if (tools.includes(domain)) {
        agents.add(agentType);
      }
    }
  }
  return [...agents];
}
