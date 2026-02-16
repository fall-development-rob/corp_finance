// Tool domain mappings for specialist agents
// Each agent gets a curated subset of the 71 MCP tool modules

export const TOOL_MAPPINGS: Record<string, string[]> = {
  'equity-analyst': [
    'equity_research', 'valuation', 'earnings_quality', 'dividend_policy',
    'behavioral', 'performance_attribution', 'three_statement', 'fpa',
    'fmp',
  ],
  'credit-analyst': [
    'credit', 'credit_scoring', 'credit_portfolio', 'credit_derivatives',
    'restructuring', 'financial_forensics', 'three_statement', 'bank_analytics',
    'fmp',
  ],
  'fixed-income-analyst': [
    'fixed_income', 'interest_rate_models', 'inflation_linked', 'mortgage_analytics',
    'repo_financing', 'municipal', 'sovereign', 'three_statement',
    'fmp',
  ],
  'derivatives-analyst': [
    'derivatives', 'volatility_surface', 'convertibles', 'structured_products',
    'real_options', 'monte_carlo', 'credit_derivatives',
    'fmp',
  ],
  'quant-risk-analyst': [
    'quant_risk', 'quant_strategies', 'portfolio_optimization', 'risk_budgeting',
    'market_microstructure', 'index_construction', 'scenarios', 'monte_carlo',
    'portfolio', 'performance_attribution',
    'fmp',
  ],
  'macro-analyst': [
    'macro_economics', 'fx_commodities', 'commodity_trading', 'emerging_markets',
    'trade_finance', 'carbon_markets', 'sovereign', 'inflation_linked',
    'fmp',
  ],
  'esg-regulatory-analyst': [
    'esg', 'regulatory', 'compliance', 'aml_compliance', 'regulatory_reporting',
    'fatca_crs', 'substance_requirements', 'tax_treaty', 'transfer_pricing',
    'carbon_markets',
    'fmp',
  ],
  'private-markets-analyst': [
    'pe', 'venture', 'private_credit', 'private_wealth', 'infrastructure',
    'real_assets', 'fund_of_funds', 'clo_analytics', 'securitization',
    'ma', 'capital_allocation', 'lease_accounting',
    'fmp',
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


/** Domain keyword patterns — shared between static classifier and semantic router */
export const DOMAIN_PATTERNS: Record<string, string[]> = {
  valuation: ['dcf', 'valuation', 'fair value', 'intrinsic value', 'comps', 'multiples', 'sum of parts'],
  equity_research: ['equity', 'stock', 'earnings', 'eps', 'revenue growth', 'margin'],
  credit: ['credit', 'default', 'spread', 'covenant', 'rating', 'leverage'],
  fixed_income: ['bond', 'yield', 'duration', 'convexity', 'coupon', 'fixed income'],
  derivatives: ['option', 'derivative', 'swap', 'futures', 'greeks', 'volatility'],
  quant_risk: ['var', 'risk', 'sharpe', 'drawdown', 'factor', 'beta'],
  portfolio_optimization: ['portfolio', 'allocation', 'rebalance', 'efficient frontier'],
  macro_economics: ['macro', 'gdp', 'inflation', 'rates', 'central bank'],
  esg: ['esg', 'sustainability', 'carbon', 'governance', 'social'],
  regulatory: ['regulatory', 'compliance', 'aml', 'fatca', 'basel'],
  pe: ['lbo', 'buyout', 'private equity', 'leverage'],
  ma: ['m&a', 'merger', 'acquisition', 'accretion', 'dilution'],
  restructuring: ['restructuring', 'distressed', 'bankruptcy', 'workout'],
  insurance: ['insurance', 'reserv', 'loss triangle', 'premium', 'combined ratio', 'loss ratio', 'solvency', 'scr'],
  pension: ['pension', 'defined benefit', 'ldi', 'liability driven'],
  wealth: ['retirement', 'withdrawal', 'tax loss', 'harvesting', 'estate', 'trust', 'inheritance'],
  crypto: ['crypto', 'token', 'defi', 'yield farm', 'staking'],
  jurisdiction: ['fee', 'management fee', 'gaap', 'ifrs', 'reconcil', 'withholding', 'wht',
                  'nav', 'net asset value', 'gp economics', 'carry', 'investor return', 'net return',
                  'ubti', 'eci', 'tax-exempt investor'],
  treasury: ['cash management', 'liquidity', 'hedge effective', 'ias 39'],
  three_statement: ['three statement', 'financial model', '3-statement'],
  monte_carlo: ['monte carlo', 'simulation', 'stochastic dcf', 'monte carlo dcf'],
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
