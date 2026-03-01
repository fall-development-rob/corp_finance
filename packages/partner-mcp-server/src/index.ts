#!/usr/bin/env node
import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';

// --- LSEG (15 tools) ---
import { registerPricingTools as registerLsegPricingTools } from './lseg/tools/pricing.js';
import { registerResearchTools as registerLsegResearchTools } from './lseg/tools/research.js';
import { registerFixedIncomeTools } from './lseg/tools/fixed-income.js';
import { registerReferenceTools } from './lseg/tools/reference.js';

// --- S&P Global (14 tools) ---
import { registerCompanyTools as registerSpCompanyTools } from './sp-global/tools/company.js';
import { registerFinancialTools } from './sp-global/tools/financials.js';
import { registerResearchTools as registerSpResearchTools } from './sp-global/tools/research.js';
import { registerDealTools as registerSpDealTools } from './sp-global/tools/deals.js';

// --- FactSet (16 tools) ---
import { registerFundamentalsTools } from './factset/tools/fundamentals.js';
import { registerPricingTools as registerFactsetPricingTools } from './factset/tools/pricing.js';
import { registerOwnershipTools } from './factset/tools/ownership.js';
import { registerAnalyticsTools } from './factset/tools/analytics.js';
import { registerResearchTools as registerFactsetResearchTools } from './factset/tools/research.js';
import { registerBatchTools } from './factset/tools/batch.js';

// --- Morningstar (14 tools) ---
import { registerFundTools } from './morningstar/tools/fund.js';
import { registerEtfTools } from './morningstar/tools/etf.js';
import { registerResearchTools as registerMsResearchTools } from './morningstar/tools/research.js';
import { registerPortfolioTools } from './morningstar/tools/portfolio.js';

// --- Moody's (14 tools) ---
import { registerRatingsTools } from './moodys/tools/ratings.js';
import { registerDefaultsTools } from './moodys/tools/defaults.js';
import { registerEconomicsTools } from './moodys/tools/economics.js';
import { registerEsgTools } from './moodys/tools/esg.js';
import { registerStructuredTools } from './moodys/tools/structured.js';

// --- PitchBook (14 tools) ---
import { registerCompanyTools as registerPbCompanyTools } from './pitchbook/tools/company.js';
import { registerDealTools as registerPbDealTools } from './pitchbook/tools/deals.js';
import { registerInvestorTools } from './pitchbook/tools/investors.js';
import { registerMarketTools } from './pitchbook/tools/market.js';

const server = new McpServer({
  name: 'partner-data',
  version: '1.0.0',
});

// ── LSEG (Refinitiv) — 15 tools ──
registerLsegPricingTools(server);      // 4: historical_prices, intraday_prices, bond_pricing, fx_rates
registerLsegResearchTools(server);     // 6: company_search, fundamentals, esg_scores, news, options_chain, economic_indicators
registerFixedIncomeTools(server);      // 2: yield_curve, credit_spreads
registerReferenceTools(server);        // 3: reference_data, corporate_actions, ownership

// ── S&P Global — 14 tools ──
registerSpCompanyTools(server);        // 4: company_search, company_tearsheet, capital_structure, ownership
registerFinancialTools(server);        // 3: financials, estimates, segment_data
registerSpResearchTools(server);       // 5: earnings_transcript, credit_rating, peer_analysis, key_developments, industry_benchmark
registerSpDealTools(server);           // 2: ma_deals, funding_digest

// ── FactSet — 16 tools ──
registerFundamentalsTools(server);     // 3: fundamentals, estimates, company_search
registerFactsetPricingTools(server);   // 2: prices, bond_pricing
registerOwnershipTools(server);        // 2: ownership, institutional
registerAnalyticsTools(server);        // 3: portfolio_analytics, risk_model, factor_exposure
registerFactsetResearchTools(server);  // 5: supply_chain, geo_revenue, events, people, ma_deals
registerBatchTools(server);            // 1: batch_request

// ── Morningstar — 14 tools ──
registerFundTools(server);             // 5: fund_rating, fund_holdings, fund_performance, historical_nav, expense_analysis
registerEtfTools(server);              // 1: etf_analytics
registerMsResearchTools(server);       // 5: fair_value, moat_rating, esg_risk, analyst_report, company_profile
registerPortfolioTools(server);        // 3: portfolio_xray, asset_allocation, peer_comparison

// ── Moody's — 14 tools ──
registerRatingsTools(server);          // 3: credit_rating, rating_history, issuer_profile
registerDefaultsTools(server);         // 3: default_rates, recovery_rates, transition_matrix
registerEconomicsTools(server);        // 3: economic_forecast, country_risk, industry_outlook
registerEsgTools(server);              // 2: esg_score, climate_risk
registerStructuredTools(server);       // 3: structured_finance, municipal_score, company_financials

// ── PitchBook — 14 tools ──
registerPbCompanyTools(server);        // 2: company_search, company_profile
registerPbDealTools(server);           // 3: deal_search, deal_details, comparable_deals
registerInvestorTools(server);         // 4: investor_profile, fund_search, fund_performance, lp_commitments
registerMarketTools(server);           // 5: vc_exits, fundraising, market_stats, people_search, service_providers

const transport = new StdioServerTransport();
await server.connect(transport);
