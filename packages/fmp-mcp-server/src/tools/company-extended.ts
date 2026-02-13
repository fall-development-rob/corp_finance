import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { fmpFetch, CacheTTL } from '../client.js';
import {
  CikSearchSchema,
  CusipSearchSchema,
  IsinSearchSchema,
  ExchangeVariantsSchema,
  SymbolOnlySchema,
  BatchSymbolsSchema,
  PageLimitSchema,
  DelistedSchema,
  MaSearchSchema,
  EmptySchema,
  ExchangeQuoteSchema,
  HistoricalPriceLightSchema,
} from '../schemas/company-extended.js';

function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

export function registerCompanyExtendedTools(server: McpServer) {
  // ─────────────────────────────────────────────────────────────────────────────
  // Extended Search
  // ─────────────────────────────────────────────────────────────────────────────

  server.tool(
    'fmp_search_cik',
    'Search for a company by CIK number. Returns company info matching the SEC CIK identifier.',
    CikSearchSchema.shape,
    async (params) => {
      const { cik } = CikSearchSchema.parse(params);
      const data = await fmpFetch('search-cik', { cik }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_search_cusip',
    'Search for a security by CUSIP identifier. Returns matching company and security details.',
    CusipSearchSchema.shape,
    async (params) => {
      const { cusip } = CusipSearchSchema.parse(params);
      const data = await fmpFetch('search-cusip', { cusip }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_search_isin',
    'Search for a security by ISIN identifier. Returns matching company and security details.',
    IsinSearchSchema.shape,
    async (params) => {
      const { isin } = IsinSearchSchema.parse(params);
      const data = await fmpFetch('search-isin', { isin }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_exchange_variants',
    'Find all exchange listings for a given symbol. Shows where a stock is listed across different exchanges.',
    ExchangeVariantsSchema.shape,
    async (params) => {
      const { symbol } = ExchangeVariantsSchema.parse(params);
      const data = await fmpFetch('search-exchange-variants', { symbol }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  // ─────────────────────────────────────────────────────────────────────────────
  // Stock Directory
  // ─────────────────────────────────────────────────────────────────────────────

  server.tool(
    'fmp_stock_list',
    'Get the complete list of all available stock symbols with name, exchange, and price.',
    EmptySchema.shape,
    async (params) => {
      EmptySchema.parse(params);
      const data = await fmpFetch('stock-list', {}, { cacheTtl: CacheTTL.STATIC });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_financial_statement_symbols',
    'Get list of all symbols that have available financial statements. Useful for filtering before pulling financials.',
    EmptySchema.shape,
    async (params) => {
      EmptySchema.parse(params);
      const data = await fmpFetch('financial-statement-symbol-list', {}, { cacheTtl: CacheTTL.STATIC });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_cik_list',
    'Get CIK number directory mapping CIK numbers to company names and tickers.',
    PageLimitSchema.shape,
    async (params) => {
      const { page, limit } = PageLimitSchema.parse(params);
      const data = await fmpFetch('cik-list', { page, limit }, { cacheTtl: CacheTTL.STATIC });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_symbol_changes',
    'Get recent ticker symbol changes. Tracks when companies change their trading symbol.',
    EmptySchema.shape,
    async (params) => {
      EmptySchema.parse(params);
      const data = await fmpFetch('symbol-change', {}, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_etf_list',
    'Get the complete list of all available ETF symbols with name and exchange.',
    EmptySchema.shape,
    async (params) => {
      EmptySchema.parse(params);
      const data = await fmpFetch('etf-list', {}, { cacheTtl: CacheTTL.STATIC });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_actively_trading',
    'Get list of all currently actively trading symbols. Useful for filtering active instruments.',
    EmptySchema.shape,
    async (params) => {
      EmptySchema.parse(params);
      const data = await fmpFetch('actively-trading-list', {}, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_available_exchanges',
    'Get list of all available stock exchanges supported by FMP.',
    EmptySchema.shape,
    async (params) => {
      EmptySchema.parse(params);
      const data = await fmpFetch('available-exchanges', {}, { cacheTtl: CacheTTL.STATIC });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_available_sectors',
    'Get list of all available market sectors for filtering and screening.',
    EmptySchema.shape,
    async (params) => {
      EmptySchema.parse(params);
      const data = await fmpFetch('available-sectors', {}, { cacheTtl: CacheTTL.STATIC });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_available_industries',
    'Get list of all available industries for filtering and screening.',
    EmptySchema.shape,
    async (params) => {
      EmptySchema.parse(params);
      const data = await fmpFetch('available-industries', {}, { cacheTtl: CacheTTL.STATIC });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_available_countries',
    'Get list of all available countries for filtering and screening.',
    EmptySchema.shape,
    async (params) => {
      EmptySchema.parse(params);
      const data = await fmpFetch('available-countries', {}, { cacheTtl: CacheTTL.STATIC });
      return wrapResponse(data);
    },
  );

  // ─────────────────────────────────────────────────────────────────────────────
  // Extended Company Info
  // ─────────────────────────────────────────────────────────────────────────────

  server.tool(
    'fmp_profile_by_cik',
    'Get company profile by CIK number. Same data as standard profile but using SEC CIK identifier.',
    CikSearchSchema.shape,
    async (params) => {
      const { cik } = CikSearchSchema.parse(params);
      const data = await fmpFetch('profile-cik', { cik }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_company_notes',
    'Get company notes and filing annotations. Useful for tracking SEC filings and corporate disclosures.',
    SymbolOnlySchema.shape,
    async (params) => {
      const { symbol } = SymbolOnlySchema.parse(params);
      const data = await fmpFetch('company-notes', { symbol }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_delisted_companies',
    'Get list of delisted companies with delisting date and reason. Useful for survivorship bias analysis.',
    DelistedSchema.shape,
    async (params) => {
      const { page, limit } = DelistedSchema.parse(params);
      const data = await fmpFetch('delisted-companies', { page, limit }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_employee_count',
    'Get current employee count for a company. Useful for productivity and efficiency analysis.',
    SymbolOnlySchema.shape,
    async (params) => {
      const { symbol } = SymbolOnlySchema.parse(params);
      const data = await fmpFetch('employee-count', { symbol }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_historical_employee_count',
    'Get historical employee count data over time. Useful for growth trend and workforce analysis.',
    SymbolOnlySchema.shape,
    async (params) => {
      const { symbol } = SymbolOnlySchema.parse(params);
      const data = await fmpFetch('historical-employee-count', { symbol }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_historical_market_cap',
    'Get historical market capitalization data over time. Useful for tracking valuation trends.',
    SymbolOnlySchema.shape,
    async (params) => {
      const { symbol } = SymbolOnlySchema.parse(params);
      const data = await fmpFetch('historical-market-capitalization', { symbol }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_batch_market_cap',
    'Get market capitalization for multiple symbols in a single request. Efficient for comparing company sizes.',
    BatchSymbolsSchema.shape,
    async (params) => {
      const { symbols } = BatchSymbolsSchema.parse(params);
      const data = await fmpFetch('market-capitalization-batch', { symbols }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_shares_float',
    'Get share float and liquidity data for a company. Shows shares outstanding, float, and insider ownership.',
    SymbolOnlySchema.shape,
    async (params) => {
      const { symbol } = SymbolOnlySchema.parse(params);
      const data = await fmpFetch('shares-float', { symbol }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_shares_float_all',
    'Get share float data for all companies. Paginated list of float and ownership data across the market.',
    PageLimitSchema.shape,
    async (params) => {
      const { page, limit } = PageLimitSchema.parse(params);
      const data = await fmpFetch('shares-float-all', { page, limit }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_ma_latest',
    'Get latest mergers and acquisitions transactions. Tracks recent M&A activity across the market.',
    PageLimitSchema.shape,
    async (params) => {
      const { page, limit } = PageLimitSchema.parse(params);
      const data = await fmpFetch('mergers-acquisitions-latest', { page, limit }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_ma_search',
    'Search mergers and acquisitions by company name. Find M&A deals involving a specific company.',
    MaSearchSchema.shape,
    async (params) => {
      const { name } = MaSearchSchema.parse(params);
      const data = await fmpFetch('mergers-acquisitions-search', { name }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_executive_compensation',
    'Get detailed executive compensation data including salary, bonus, stock awards, and total pay.',
    SymbolOnlySchema.shape,
    async (params) => {
      const { symbol } = SymbolOnlySchema.parse(params);
      const data = await fmpFetch('governance-executive-compensation', { symbol }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_compensation_benchmark',
    'Get executive compensation benchmark data across companies. Useful for governance and pay analysis.',
    EmptySchema.shape,
    async (params) => {
      EmptySchema.parse(params);
      const data = await fmpFetch('executive-compensation-benchmark', {}, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  // ─────────────────────────────────────────────────────────────────────────────
  // Extended Quotes
  // ─────────────────────────────────────────────────────────────────────────────

  server.tool(
    'fmp_aftermarket_trade',
    'Get after-hours trade data for a symbol. Shows extended-hours trading activity and prices.',
    SymbolOnlySchema.shape,
    async (params) => {
      const { symbol } = SymbolOnlySchema.parse(params);
      const data = await fmpFetch('aftermarket-trade', { symbol }, { cacheTtl: CacheTTL.REALTIME });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_aftermarket_quote',
    'Get after-hours quote for a symbol. Shows extended-hours bid/ask and last trade price.',
    SymbolOnlySchema.shape,
    async (params) => {
      const { symbol } = SymbolOnlySchema.parse(params);
      const data = await fmpFetch('aftermarket-quote', { symbol }, { cacheTtl: CacheTTL.REALTIME });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_price_change',
    'Get stock price change percentages across multiple time periods (1D, 5D, 1M, 3M, 6M, YTD, 1Y, 3Y, 5Y, 10Y, max).',
    SymbolOnlySchema.shape,
    async (params) => {
      const { symbol } = SymbolOnlySchema.parse(params);
      const data = await fmpFetch('stock-price-change', { symbol }, { cacheTtl: CacheTTL.REALTIME });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_batch_quote_short',
    'Get abbreviated quotes for multiple symbols in a single request. Returns price and volume only.',
    BatchSymbolsSchema.shape,
    async (params) => {
      const { symbols } = BatchSymbolsSchema.parse(params);
      const data = await fmpFetch('batch-quote-short', { symbols }, { cacheTtl: CacheTTL.REALTIME });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_batch_aftermarket_trade',
    'Get after-hours trade data for multiple symbols in a single request.',
    BatchSymbolsSchema.shape,
    async (params) => {
      const { symbols } = BatchSymbolsSchema.parse(params);
      const data = await fmpFetch('batch-aftermarket-trade', { symbols }, { cacheTtl: CacheTTL.REALTIME });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_batch_aftermarket_quote',
    'Get after-hours quotes for multiple symbols in a single request.',
    BatchSymbolsSchema.shape,
    async (params) => {
      const { symbols } = BatchSymbolsSchema.parse(params);
      const data = await fmpFetch('batch-aftermarket-quote', { symbols }, { cacheTtl: CacheTTL.REALTIME });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_exchange_quotes',
    'Get all real-time quotes for every symbol on a given exchange. Useful for broad market screening.',
    ExchangeQuoteSchema.shape,
    async (params) => {
      const { exchange } = ExchangeQuoteSchema.parse(params);
      const data = await fmpFetch('batch-exchange-quote', { exchange }, { cacheTtl: CacheTTL.REALTIME });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_batch_mutualfund_quotes',
    'Get real-time quotes for all mutual funds. Comprehensive mutual fund market snapshot.',
    EmptySchema.shape,
    async (params) => {
      EmptySchema.parse(params);
      const data = await fmpFetch('batch-mutualfund-quotes', {}, { cacheTtl: CacheTTL.REALTIME });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_batch_etf_quotes',
    'Get real-time quotes for all ETFs. Comprehensive ETF market snapshot.',
    EmptySchema.shape,
    async (params) => {
      EmptySchema.parse(params);
      const data = await fmpFetch('batch-etf-quotes', {}, { cacheTtl: CacheTTL.REALTIME });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_batch_crypto_quotes',
    'Get real-time quotes for all cryptocurrencies. Comprehensive crypto market snapshot.',
    EmptySchema.shape,
    async (params) => {
      EmptySchema.parse(params);
      const data = await fmpFetch('batch-crypto-quotes', {}, { cacheTtl: CacheTTL.REALTIME });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_batch_forex_quotes',
    'Get real-time quotes for all forex pairs. Comprehensive FX market snapshot.',
    EmptySchema.shape,
    async (params) => {
      EmptySchema.parse(params);
      const data = await fmpFetch('batch-forex-quotes', {}, { cacheTtl: CacheTTL.REALTIME });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_batch_index_quotes',
    'Get real-time quotes for all market indices. Comprehensive index market snapshot.',
    EmptySchema.shape,
    async (params) => {
      EmptySchema.parse(params);
      const data = await fmpFetch('batch-index-quotes', {}, { cacheTtl: CacheTTL.REALTIME });
      return wrapResponse(data);
    },
  );

  // ─────────────────────────────────────────────────────────────────────────────
  // Extended Charts
  // ─────────────────────────────────────────────────────────────────────────────

  server.tool(
    'fmp_historical_price_light',
    'Get lightweight historical EOD price data (date, close, volume only). Faster than full endpoint for simple analysis.',
    HistoricalPriceLightSchema.shape,
    async (params) => {
      const { symbol, from, to } = HistoricalPriceLightSchema.parse(params);
      const data = await fmpFetch('historical-price-eod/light', { symbol, from, to }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_historical_price_unadjusted',
    'Get unadjusted (non-split-adjusted) historical EOD prices. Raw prices without stock split adjustments.',
    HistoricalPriceLightSchema.shape,
    async (params) => {
      const { symbol, from, to } = HistoricalPriceLightSchema.parse(params);
      const data = await fmpFetch('historical-price-eod/non-split-adjusted', { symbol, from, to }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_historical_price_div_adjusted',
    'Get dividend-adjusted historical EOD prices. Prices adjusted for both splits and dividend reinvestment.',
    HistoricalPriceLightSchema.shape,
    async (params) => {
      const { symbol, from, to } = HistoricalPriceLightSchema.parse(params);
      const data = await fmpFetch('historical-price-eod/dividend-adjusted', { symbol, from, to }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  // ─────────────────────────────────────────────────────────────────────────────
  // Extended Analyst
  // ─────────────────────────────────────────────────────────────────────────────

  server.tool(
    'fmp_ratings_snapshot',
    'Get current analyst ratings snapshot with overall rating score and recommendation breakdown.',
    SymbolOnlySchema.shape,
    async (params) => {
      const { symbol } = SymbolOnlySchema.parse(params);
      const data = await fmpFetch('ratings-snapshot', { symbol }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_ratings_historical',
    'Get historical analyst ratings data over time. Track how ratings have changed for trend analysis.',
    SymbolOnlySchema.shape,
    async (params) => {
      const { symbol } = SymbolOnlySchema.parse(params);
      const data = await fmpFetch('ratings-historical', { symbol }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_price_target_consensus',
    'Get consensus price target from analyst estimates. Shows average, median, high, and low price targets.',
    SymbolOnlySchema.shape,
    async (params) => {
      const { symbol } = SymbolOnlySchema.parse(params);
      const data = await fmpFetch('price-target-consensus', { symbol }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_grades_historical',
    'Get historical analyst grade changes (upgrades, downgrades, initiations). Track sell-side sentiment shifts.',
    SymbolOnlySchema.shape,
    async (params) => {
      const { symbol } = SymbolOnlySchema.parse(params);
      const data = await fmpFetch('grades-historical', { symbol }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_grades_consensus',
    'Get consensus analyst grade with buy/hold/sell breakdown. Summary of current sell-side sentiment.',
    SymbolOnlySchema.shape,
    async (params) => {
      const { symbol } = SymbolOnlySchema.parse(params);
      const data = await fmpFetch('grades-consensus', { symbol }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );
}
