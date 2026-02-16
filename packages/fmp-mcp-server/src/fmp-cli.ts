#!/usr/bin/env node
import 'dotenv/config';
import { fmpFetch, CacheTTL } from './client.js';

// ── ANSI helpers (same pattern as cfa CLI) ──────────────────────────
const isTTY = process.stdout.isTTY ?? false;
const ansi = {
  reset: isTTY ? '\x1b[0m' : '', bold: isTTY ? '\x1b[1m' : '', dim: isTTY ? '\x1b[2m' : '',
  cyan: isTTY ? '\x1b[36m' : '', green: isTTY ? '\x1b[32m' : '', yellow: isTTY ? '\x1b[33m' : '',
  red: isTTY ? '\x1b[31m' : '', magenta: isTTY ? '\x1b[35m' : '', gray: isTTY ? '\x1b[90m' : '',
};
function c(color: keyof typeof ansi, text: string): string { return `${ansi[color]}${text}${ansi.reset}`; }

// ── Arg parsing (zero dependencies) ─────────────────────────────────
const args = process.argv.slice(2);
const command = args[0]?.toLowerCase();

function getFlag(flag: string): string | undefined {
  const idx = args.indexOf(flag);
  return idx >= 0 && idx + 1 < args.length ? args[idx + 1] : undefined;
}

function requireSymbol(): string {
  const sym = args[1];
  if (!sym || sym.startsWith('-')) { console.error(`${c('red', 'Error:')} <symbol> is required`); process.exit(1); }
  return sym.toUpperCase();
}

function fmt(n: number | null | undefined, d = 2): string {
  return n != null ? n.toLocaleString(undefined, { minimumFractionDigits: d, maximumFractionDigits: d }) : '-';
}
function fmtB(n: number | null | undefined): string {
  if (n == null) return '-';
  const a = Math.abs(n);
  if (a >= 1e12) return `$${(n / 1e12).toFixed(2)}T`;
  if (a >= 1e9) return `$${(n / 1e9).toFixed(2)}B`;
  if (a >= 1e6) return `$${(n / 1e6).toFixed(1)}M`;
  return `$${n.toLocaleString()}`;
}
function padR(s: string, n: number): string { return s.length >= n ? s : s + ' '.repeat(n - s.length); }
function padL(s: string, n: number): string { return s.length >= n ? s : ' '.repeat(n - s.length) + s; }

// ── Handlers ────────────────────────────────────────────────────────

async function handleQuote() {
  const symbol = requireSymbol();
  const data = await fmpFetch<any[]>('quote', { symbol }, { cacheTtl: CacheTTL.REALTIME });
  const q = Array.isArray(data) ? data[0] : data;
  if (!q) { console.log('No data found'); return; }
  const cc = (q.changesPercentage ?? 0) >= 0 ? 'green' : 'red';
  console.log(`\n  ${c('bold', q.symbol)} ${c('dim', q.name ?? '')}`);
  console.log(`  Price: ${c('bold', fmt(q.price))}  ${c(cc as any, `${q.change >= 0 ? '+' : ''}${fmt(q.change)} (${fmt(q.changesPercentage)}%)`)}`);
  console.log(`  ${c('dim', `Vol: ${q.volume?.toLocaleString() ?? '-'}  |  Mkt Cap: ${fmtB(q.marketCap)}  |  PE: ${fmt(q.pe)}`)}`);
  console.log(`  ${c('dim', `52W: ${fmt(q.yearLow)} - ${fmt(q.yearHigh)}  |  Avg Vol: ${q.avgVolume?.toLocaleString() ?? '-'}`)}\n`);
}

async function handleProfile() {
  const symbol = requireSymbol();
  const data = await fmpFetch<any[]>('profile', { symbol }, { cacheTtl: CacheTTL.LONG });
  const p = Array.isArray(data) ? data[0] : data;
  if (!p) { console.log('No data found'); return; }
  console.log(`\n  ${c('bold', p.symbol)} ${c('cyan', p.companyName ?? '')}`);
  console.log(`  ${c('dim', `${p.sector ?? ''} > ${p.industry ?? ''}  |  ${p.exchangeShortName ?? ''} (${p.country ?? ''})`)}`);
  console.log(`  ${c('dim', `CEO: ${p.ceo ?? '-'}  |  Employees: ${p.fullTimeEmployees?.toLocaleString() ?? '-'}  |  IPO: ${p.ipoDate ?? '-'}`)}`);
  console.log(`  Mkt Cap: ${c('bold', fmtB(p.mktCap))}  |  Price: ${c('bold', fmt(p.price))}  |  Beta: ${fmt(p.beta)}`);
  console.log(`  ${c('dim', `Range: ${p.range ?? '-'}  |  Avg Vol: ${p.volAvg?.toLocaleString() ?? '-'}`)}`);
  if (p.description) {
    const desc = p.description.length > 300 ? p.description.slice(0, 297) + '...' : p.description;
    console.log(`\n  ${c('dim', desc)}`);
  }
  if (p.website) console.log(`  ${c('cyan', p.website)}`);
  console.log();
}

async function handleFinancials() {
  const symbol = requireSymbol();
  const period = getFlag('--period') ?? 'annual';
  const limit = Number(getFlag('--limit') ?? 4);
  const [income, balance, cashflow] = await Promise.all([
    fmpFetch<any[]>('income-statement', { symbol, period, limit }, { cacheTtl: CacheTTL.MEDIUM }),
    fmpFetch<any[]>('balance-sheet-statement', { symbol, period, limit }, { cacheTtl: CacheTTL.MEDIUM }),
    fmpFetch<any[]>('cash-flow-statement', { symbol, period, limit }, { cacheTtl: CacheTTL.MEDIUM }),
  ]);
  console.log(`\n  ${c('bold', symbol)} Financial Statements ${c('dim', `(${period}, last ${limit})`)}\n`);
  if (income?.length) {
    console.log(`  ${c('cyan', 'Income Statement')}`);
    for (const r of income)
      console.log(`  ${c('dim', r.date ?? r.calendarYear)}  Rev: ${padL(fmtB(r.revenue), 10)}  GP: ${padL(fmtB(r.grossProfit), 10)}  NI: ${padL(fmtB(r.netIncome), 10)}  EPS: ${padL(fmt(r.eps), 6)}`);
  }
  if (balance?.length) {
    console.log(`\n  ${c('cyan', 'Balance Sheet')}`);
    for (const r of balance)
      console.log(`  ${c('dim', r.date ?? r.calendarYear)}  Assets: ${padL(fmtB(r.totalAssets), 10)}  Liab: ${padL(fmtB(r.totalLiabilities), 10)}  Equity: ${padL(fmtB(r.totalStockholdersEquity), 10)}`);
  }
  if (cashflow?.length) {
    console.log(`\n  ${c('cyan', 'Cash Flow')}`);
    for (const r of cashflow)
      console.log(`  ${c('dim', r.date ?? r.calendarYear)}  CFO: ${padL(fmtB(r.operatingCashFlow), 10)}  CapEx: ${padL(fmtB(r.capitalExpenditure), 10)}  FCF: ${padL(fmtB(r.freeCashFlow), 10)}`);
  }
  console.log();
}

async function handleEarnings() {
  const symbol = requireSymbol();
  const data = await fmpFetch<any[]>('earnings', { symbol, limit: 12 }, { cacheTtl: CacheTTL.MEDIUM });
  if (!data?.length) { console.log('No earnings data found'); return; }
  console.log(`\n  ${c('bold', symbol)} Earnings History\n`);
  console.log(`  ${c('dim', padR('Date', 12))}  ${padL('EPS Est', 9)}  ${padL('EPS Act', 9)}  ${padL('Surprise', 10)}  ${padL('Rev Est', 12)}  ${padL('Rev Act', 12)}`);
  console.log(`  ${c('dim', '-'.repeat(72))}`);
  for (const e of data) {
    const sur = e.epsSurprise ?? (e.epsActual != null && e.epsEstimated != null ? e.epsActual - e.epsEstimated : null);
    const sc = sur != null ? (sur >= 0 ? 'green' : 'red') : 'dim';
    console.log(`  ${c('dim', padR(e.date ?? '-', 12))}  ${padL(fmt(e.epsEstimated), 9)}  ${padL(fmt(e.epsActual), 9)}  ${c(sc as any, padL(sur != null ? `${sur >= 0 ? '+' : ''}${fmt(sur)}` : '-', 10))}  ${padL(fmtB(e.revenueEstimated), 12)}  ${padL(fmtB(e.revenue), 12)}`);
  }
  console.log();
}

async function handleScreen() {
  const params: Record<string, string | number | boolean | undefined> = {};
  if (getFlag('--sector')) params.sector = getFlag('--sector');
  if (getFlag('--industry')) params.industry = getFlag('--industry');
  if (getFlag('--market-cap-min')) params.marketCapMoreThan = Number(getFlag('--market-cap-min'));
  if (getFlag('--market-cap-max')) params.marketCapLessThan = Number(getFlag('--market-cap-max'));
  params.limit = Number(getFlag('--limit') ?? 20);
  const data = await fmpFetch<any[]>('company-screener', params, { cacheTtl: CacheTTL.MEDIUM });
  if (!data?.length) { console.log('No results found'); return; }
  console.log(`\n  ${c('bold', 'Stock Screener')} ${c('dim', `(${data.length} results)`)}\n`);
  console.log(`  ${c('dim', padR('Symbol', 8))} ${padR('Name', 30)} ${padL('Price', 10)} ${padL('Mkt Cap', 12)} ${padR('Sector', 25)}`);
  console.log(`  ${c('dim', '-'.repeat(90))}`);
  for (const s of data) {
    console.log(`  ${c('cyan', padR(s.symbol ?? '-', 8))} ${padR((s.companyName ?? '').slice(0, 28), 30)} ${padL(fmt(s.price), 10)} ${padL(fmtB(s.marketCap), 12)} ${padR((s.sector ?? '-').slice(0, 23), 25)}`);
  }
  console.log();
}

async function handleSearch() {
  const query = args[1];
  if (!query) { console.error(`${c('red', 'Error:')} <query> is required`); process.exit(1); }
  const limit = Number(getFlag('--limit') ?? 10);
  const jsonMode = args.includes('--json');
  const [bySymbol, byName] = await Promise.all([
    fmpFetch<any[]>('search-symbol', { query, limit }, { cacheTtl: CacheTTL.LONG }),
    fmpFetch<any[]>('search-name', { query, limit }, { cacheTtl: CacheTTL.LONG }),
  ]);
  const seen = new Set<string>();
  const results: any[] = [];
  for (const r of [...(bySymbol ?? []), ...(byName ?? [])]) {
    if (r.symbol && !seen.has(r.symbol)) { seen.add(r.symbol); results.push(r); }
  }
  if (!results.length) {
    if (jsonMode) { console.log(JSON.stringify([])); } else { console.log('No results found'); }
    return;
  }
  if (jsonMode) {
    console.log(JSON.stringify(results.slice(0, limit).map(r => ({
      symbol: r.symbol,
      name: r.name ?? r.companyName ?? '',
      exchange: r.exchangeShortName ?? r.exchange ?? r.stockExchange ?? '',
      currency: r.currency ?? '',
    }))));
    return;
  }
  console.log(`\n  ${c('bold', 'Search Results')} ${c('dim', `for "${query}"`)}\n`);
  for (const r of results.slice(0, limit))
    console.log(`  ${c('cyan', padR(r.symbol, 10))} ${r.name ?? r.companyName ?? '-'}  ${c('dim', r.exchangeShortName ?? r.stockExchange ?? '')}`);
  console.log();
}

async function handleNews() {
  const symbol = getFlag('--stock');
  const limit = Number(getFlag('--limit') ?? 10);
  const data: any[] = symbol
    ? await fmpFetch('news/stock', { symbols: symbol.toUpperCase(), limit }, { cacheTtl: CacheTTL.SHORT })
    : await fmpFetch('news/stock-latest', { limit }, { cacheTtl: CacheTTL.SHORT });
  if (!data?.length) { console.log('No news found'); return; }
  console.log(`\n  ${c('bold', 'Financial News')} ${symbol ? c('dim', `(${symbol.toUpperCase()})`) : ''}\n`);
  for (const n of data) {
    console.log(`  ${c('dim', (n.publishedDate ?? '').slice(0, 16))}  ${c('cyan', n.symbol ?? '')}  ${c('bold', n.title ?? '')}`);
    if (n.text) console.log(`  ${c('dim', (n.text as string).slice(0, 120) + '...')}`);
    console.log();
  }
}

async function handleTechnicals() {
  const indicator = args[1]?.toLowerCase();
  const symbol = args[2]?.toUpperCase();
  const valid = ['sma', 'ema', 'wma', 'dema', 'tema', 'rsi', 'adx', 'stddev', 'williams'];
  if (!indicator || !symbol) {
    console.error(`Usage: fmp technicals <indicator> <symbol> [--period N] [--timeframe 1day]`);
    console.error(`Indicators: ${valid.join(', ')}`); process.exit(1);
  }
  if (!valid.includes(indicator)) { console.error(`${c('red', 'Error:')} Unknown indicator. Valid: ${valid.join(', ')}`); process.exit(1); }
  const periodLength = Number(getFlag('--period') ?? 14);
  const timeframe = getFlag('--timeframe') ?? '1day';
  const data = await fmpFetch<any[]>(`technical-indicators/${indicator}`, { symbol, periodLength, timeframe }, { cacheTtl: CacheTTL.SHORT });
  if (!data?.length) { console.log('No data found'); return; }
  console.log(`\n  ${c('bold', `${indicator.toUpperCase()}(${periodLength})`)} ${c('cyan', symbol)} ${c('dim', `[${timeframe}]`)}\n`);
  for (const d of data.slice(0, 20)) {
    const val = d[indicator] ?? d.value ?? Object.values(d).find((v: any) => typeof v === 'number' && v !== d.volume);
    console.log(`  ${c('dim', padR(d.date ?? '-', 12))}  ${padL(fmt(val as number, 4), 12)}  ${c('dim', `O:${fmt(d.open)} H:${fmt(d.high)} L:${fmt(d.low)} C:${fmt(d.close)}`)}`);
  }
  console.log();
}

async function handleEtf() {
  const symbol = requireSymbol();
  const [info, holdings] = await Promise.all([
    fmpFetch<any[]>('etf/info', { symbol }, { cacheTtl: CacheTTL.MEDIUM }).catch(() => null),
    fmpFetch<any[]>('etf/holdings', { symbol }, { cacheTtl: CacheTTL.MEDIUM }).catch(() => null),
  ]);
  console.log(`\n  ${c('bold', symbol)} ETF Overview\n`);
  if (info?.length) {
    const e = info[0];
    console.log(`  ${c('cyan', e.name ?? '-')}`);
    console.log(`  ${c('dim', `Expense: ${e.expenseRatio ?? '-'}  |  AUM: ${fmtB(e.totalAssets ?? e.aum)}  |  Inception: ${e.inceptionDate ?? '-'}`)}\n`);
  }
  if (holdings?.length) {
    console.log(`  ${c('cyan', 'Top Holdings')} ${c('dim', `(${holdings.length} total)`)}\n`);
    console.log(`  ${c('dim', padR('Symbol', 10))} ${padR('Name', 35)} ${padL('Weight %', 10)}`);
    console.log(`  ${c('dim', '-'.repeat(58))}`);
    for (const h of holdings.slice(0, 20))
      console.log(`  ${c('cyan', padR(h.symbol ?? h.asset ?? '-', 10))} ${padR((h.name ?? '-').slice(0, 33), 35)} ${padL(h.weightPercentage != null ? fmt(h.weightPercentage) : (h.pctVal != null ? fmt(h.pctVal) : '-'), 10)}`);
  }
  console.log();
}

async function handleInsider() {
  const symbol = requireSymbol();
  const data = await fmpFetch<any[]>('insider-trading/search', { symbol, limit: 20 }, { cacheTtl: CacheTTL.SHORT });
  if (!data?.length) { console.log('No insider trading data found'); return; }
  console.log(`\n  ${c('bold', symbol)} Insider Trading\n`);
  console.log(`  ${c('dim', padR('Date', 12))} ${padR('Insider', 25)} ${padR('Type', 12)} ${padL('Shares', 12)} ${padL('Price', 10)} ${padL('Value', 12)}`);
  console.log(`  ${c('dim', '-'.repeat(88))}`);
  for (const t of data) {
    const tc = (t.transactionType ?? '').toLowerCase().includes('purchase') || (t.acquistionOrDisposition ?? '') === 'A' ? 'green' : 'red';
    const val = t.securitiesTransacted && t.price ? t.securitiesTransacted * t.price : null;
    console.log(`  ${c('dim', padR(t.filingDate ?? t.transactionDate ?? '-', 12))} ${padR((t.reportingName ?? t.insider ?? '-').slice(0, 23), 25)} ${c(tc as any, padR((t.transactionType ?? '-').slice(0, 10), 12))} ${padL(t.securitiesTransacted?.toLocaleString() ?? '-', 12)} ${padL(fmt(t.price), 10)} ${padL(val != null ? fmtB(val) : '-', 12)}`);
  }
  console.log();
}

async function handleSec() {
  const symbol = requireSymbol();
  const data = await fmpFetch<any[]>('sec-filings-search/symbol', { symbol, limit: 20 }, { cacheTtl: CacheTTL.SHORT });
  if (!data?.length) { console.log('No SEC filings found'); return; }
  console.log(`\n  ${c('bold', symbol)} SEC Filings\n`);
  console.log(`  ${c('dim', padR('Date', 12))} ${padR('Type', 10)} ${padR('Description', 60)}`);
  console.log(`  ${c('dim', '-'.repeat(85))}`);
  for (const f of data)
    console.log(`  ${c('dim', padR(f.fillingDate ?? f.date ?? '-', 12))} ${c('cyan', padR(f.type ?? f.formType ?? '-', 10))} ${(f.description ?? f.title ?? '-').slice(0, 58)}`);
  console.log();
}

async function handleInstitutional() {
  const symbol = requireSymbol();
  const data = await fmpFetch<any[]>('institutional-ownership/latest', { symbol, limit: 20 }, { cacheTtl: CacheTTL.SHORT });
  if (!data?.length) { console.log('No institutional ownership data found'); return; }
  console.log(`\n  ${c('bold', symbol)} Institutional Ownership\n`);
  console.log(`  ${c('dim', padR('Holder', 35))} ${padL('Shares', 14)} ${padL('Value', 14)} ${padL('Change', 12)} ${padR('Date', 12)}`);
  console.log(`  ${c('dim', '-'.repeat(90))}`);
  for (const h of data) {
    const chg = h.changeInShares != null ? `${h.changeInShares >= 0 ? '+' : ''}${h.changeInShares.toLocaleString()}` : '-';
    console.log(`  ${padR((h.investorName ?? h.holder ?? '-').slice(0, 33), 35)} ${padL(h.shares?.toLocaleString() ?? '-', 14)} ${padL(fmtB(h.value ?? h.totalInvested), 14)} ${c((h.changeInShares ?? 0) >= 0 ? 'green' : 'red' as any, padL(chg, 12))} ${c('dim', padR(h.filingDate ?? h.date ?? '-', 12))}`);
  }
  console.log();
}

async function handleDividends() {
  const symbol = requireSymbol();
  const data = await fmpFetch<any[]>('dividends', { symbol }, { cacheTtl: CacheTTL.MEDIUM });
  if (!data?.length) { console.log('No dividend history found'); return; }
  console.log(`\n  ${c('bold', symbol)} Dividend History\n`);
  console.log(`  ${c('dim', padR('Date', 12))} ${padL('Amount', 10)} ${padR('Record', 12)} ${padR('Payment', 12)} ${padR('Declaration', 12)}`);
  console.log(`  ${c('dim', '-'.repeat(62))}`);
  for (const d of data.slice(0, 20))
    console.log(`  ${c('dim', padR(d.date ?? '-', 12))} ${c('green', padL(fmt(d.dividend ?? d.adjDividend, 4), 10))} ${padR(d.recordDate ?? '-', 12)} ${padR(d.paymentDate ?? '-', 12)} ${padR(d.declarationDate ?? '-', 12)}`);
  console.log();
}

async function handleMacro() {
  const indicator = args[1];
  if (!indicator) {
    console.error(`Usage: fmp macro <indicator>`);
    console.error(`Examples: GDP, realGDP, CPI, inflationRate, unemployment, federalFundsRate`); process.exit(1);
  }
  const data = await fmpFetch<any[]>('economic-indicators', { name: indicator }, { cacheTtl: CacheTTL.MEDIUM });
  if (!data?.length) { console.log('No data found for indicator'); return; }
  console.log(`\n  ${c('bold', indicator)} Economic Data\n`);
  console.log(`  ${c('dim', padR('Date', 12))} ${padL('Value', 16)}`);
  console.log(`  ${c('dim', '-'.repeat(30))}`);
  for (const d of data.slice(0, 20))
    console.log(`  ${c('dim', padR(d.date ?? '-', 12))} ${padL(fmt(d.value, 4), 16)}`);
  console.log();
}

async function handleTreasury() {
  const data = await fmpFetch<any[]>('treasury-rates', {}, { cacheTtl: CacheTTL.SHORT });
  if (!data?.length) { console.log('No treasury data found'); return; }
  const latest = data[0];
  console.log(`\n  ${c('bold', 'US Treasury Rates')} ${c('dim', `(${latest.date ?? 'latest'})`)}\n`);
  const mats: [string, string][] = [
    ['month1','1M'],['month2','2M'],['month3','3M'],['month6','6M'],
    ['year1','1Y'],['year2','2Y'],['year3','3Y'],['year5','5Y'],
    ['year7','7Y'],['year10','10Y'],['year20','20Y'],['year30','30Y'],
  ];
  for (const [key, label] of mats) {
    const val = latest[key];
    if (val != null) console.log(`  ${c('dim', padR(label, 5))} ${c('cyan', padL(fmt(val, 3) + '%', 8))} ${c('dim', '#'.repeat(Math.round(val * 10)))}`);
  }
  console.log();
}

async function handleMovers(endpoint: string, title: string) {
  const data = await fmpFetch<any[]>(endpoint, {}, { cacheTtl: CacheTTL.REALTIME });
  if (!data?.length) { console.log('No data found'); return; }
  console.log(`\n  ${c('bold', title)}\n`);
  console.log(`  ${c('dim', padR('Symbol', 8))} ${padR('Name', 28)} ${padL('Price', 10)} ${padL('Change', 10)} ${padL('Change %', 10)} ${padL('Volume', 14)}`);
  console.log(`  ${c('dim', '-'.repeat(84))}`);
  for (const s of data.slice(0, 20)) {
    const pc = (s.changesPercentage ?? 0) >= 0 ? 'green' : 'red';
    console.log(`  ${c('cyan', padR(s.symbol ?? '-', 8))} ${padR((s.name ?? '-').slice(0, 26), 28)} ${padL(fmt(s.price), 10)} ${c(pc as any, padL(`${s.change >= 0 ? '+' : ''}${fmt(s.change)}`, 10))} ${c(pc as any, padL(`${s.changesPercentage >= 0 ? '+' : ''}${fmt(s.changesPercentage)}%`, 10))} ${padL(s.volume?.toLocaleString() ?? '-', 14)}`);
  }
  console.log();
}

async function handleTools() {
  const cats: Record<string, string[]> = {
    'Quotes (5)': ['fmp_quote','fmp_batch_quote','fmp_quote_short','fmp_historical_price','fmp_intraday_chart'],
    'Profiles (4)': ['fmp_company_profile','fmp_stock_peers','fmp_key_executives','fmp_market_cap'],
    'Financials (6)': ['fmp_income_statement','fmp_balance_sheet','fmp_cash_flow','fmp_income_ttm','fmp_key_metrics','fmp_financial_ratios'],
    'Financials Extended (20)': ['fmp_balance_sheet_ttm','fmp_cash_flow_ttm','fmp_key_metrics_ttm','fmp_ratios_ttm','fmp_financial_scores','fmp_owner_earnings','fmp_balance_as_reported','fmp_balance_sheet_growth','fmp_cash_flow_as_reported','fmp_cash_flow_growth','fmp_enterprise_values','fmp_financial_growth','fmp_financial_reports_dates','fmp_financial_reports_json','fmp_full_statement_as_reported','fmp_income_as_reported','fmp_income_growth','fmp_latest_financial_statements','fmp_revenue_geo_segments','fmp_revenue_product_segments'],
    'Earnings (6)': ['fmp_earnings','fmp_earnings_calendar','fmp_earnings_transcript','fmp_analyst_estimates','fmp_price_target','fmp_grades'],
    'Market (9)': ['fmp_search_symbol','fmp_search_name','fmp_stock_screener','fmp_sector_performance','fmp_industry_performance','fmp_index_constituents','fmp_economic_calendar','fmp_economic_indicators','fmp_treasury_rates'],
    'Market Extended (19)': ['fmp_biggest_gainers','fmp_biggest_losers','fmp_most_active','fmp_historical_sector_performance','fmp_historical_industry_performance','fmp_sector_pe','fmp_industry_pe','fmp_historical_sector_pe','fmp_historical_industry_pe','fmp_all_exchange_hours','fmp_exchange_hours','fmp_exchange_holidays','fmp_batch_commodity_quotes','fmp_commodities_list','fmp_index_list','fmp_market_risk_premium','fmp_historical_dowjones_constituent','fmp_historical_nasdaq_constituent'],
    'News (10)': ['fmp_fmp_articles','fmp_news_general','fmp_news_press_releases','fmp_news_stock','fmp_news_crypto','fmp_news_forex','fmp_search_press_releases','fmp_search_stock_news','fmp_search_crypto_news','fmp_search_forex_news'],
    'Technicals (9)': ['fmp_sma','fmp_ema','fmp_wma','fmp_dema','fmp_tema','fmp_rsi','fmp_stddev','fmp_williams','fmp_adx'],
    'ETF (9)': ['fmp_etf_holdings','fmp_etf_info','fmp_etf_country_weightings','fmp_etf_asset_exposure','fmp_etf_sector_weightings','fmp_fund_disclosure_holders','fmp_fund_disclosure','fmp_fund_disclosure_search','fmp_fund_disclosure_dates'],
    'Insider Trading (6)': ['fmp_insider_latest','fmp_insider_search','fmp_insider_by_name','fmp_insider_transaction_types','fmp_insider_stats','fmp_beneficial_ownership'],
    'Institutional (8)': ['fmp_institutional_latest','fmp_institutional_extract','fmp_institutional_dates','fmp_institutional_analytics_holder','fmp_holder_performance','fmp_holder_industry_breakdown','fmp_positions_summary','fmp_industry_ownership_summary'],
    'Dividends & Events (10)': ['fmp_dividends','fmp_dividends_calendar','fmp_splits','fmp_splits_calendar','fmp_ipo_calendar','fmp_ipo_disclosure','fmp_ipo_prospectus','fmp_earnings_transcript_dates','fmp_earnings_transcript_latest','fmp_earnings_transcript_list'],
    'SEC Filings (11)': ['fmp_sec_filings_financials','fmp_sec_filings_by_form','fmp_sec_filings_by_symbol','fmp_sec_filings_by_cik','fmp_sec_company_search_name','fmp_sec_company_search_symbol','fmp_sec_company_search_cik','fmp_sec_profile','fmp_all_sic','fmp_sic_list','fmp_sic_search'],
    'Company Extended (47)': ['fmp_search_cik','fmp_search_cusip','fmp_search_isin','fmp_exchange_variants','fmp_stock_list','fmp_financial_statement_symbols','fmp_etf_list','fmp_cik_list','fmp_delisted_companies','fmp_symbol_changes','fmp_available_exchanges','fmp_available_countries','fmp_available_sectors','fmp_available_industries','fmp_employee_count','fmp_historical_employee_count','fmp_executive_compensation','fmp_compensation_benchmark','fmp_company_notes','fmp_shares_float','fmp_shares_float_all','fmp_historical_market_cap','fmp_batch_market_cap','fmp_price_change','fmp_ratings_snapshot','fmp_ratings_historical','fmp_grades_consensus','fmp_grades_historical','fmp_price_target_consensus','fmp_profile_by_cik','fmp_ma_latest','fmp_ma_search','fmp_actively_trading','fmp_aftermarket_quote','fmp_aftermarket_trade','fmp_batch_aftermarket_quote','fmp_batch_aftermarket_trade','fmp_batch_quote_short','fmp_exchange_quotes','fmp_batch_crypto_quotes','fmp_batch_etf_quotes','fmp_batch_forex_quotes','fmp_batch_index_quotes','fmp_batch_mutualfund_quotes','fmp_historical_price_div_adjusted','fmp_historical_price_light','fmp_historical_price_unadjusted'],
  };
  let total = 0;
  console.log(`\n  ${c('bold', 'FMP MCP Tools')}\n`);
  for (const [cat, tools] of Object.entries(cats)) {
    total += tools.length;
    console.log(`  ${c('cyan', cat)}`);
    let line = '    ';
    for (const t of tools) {
      if (line.length + t.length + 2 > 100) { console.log(c('dim', line)); line = '    '; }
      line += `${t}  `;
    }
    if (line.trim()) console.log(c('dim', line));
    console.log();
  }
  console.log(`  ${c('bold', `Total: ${total} tools`)}\n`);
}

function printHelp() {
  console.log(`
  ${c('bold', 'fmp')} ${c('dim', '-- Financial Modeling Prep CLI')}

  ${c('cyan', 'Usage:')}  fmp <command> [args] [options]

  ${c('cyan', 'Commands:')}
    ${c('bold', 'quote')} <symbol>                    Real-time quote
    ${c('bold', 'profile')} <symbol>                  Company profile
    ${c('bold', 'financials')} <symbol>               Income + balance + cash flow
        ${c('dim', '[--period annual|quarter] [--limit N]')}
    ${c('bold', 'earnings')} <symbol>                 Earnings history
    ${c('bold', 'screen')}                            Stock screener
        ${c('dim', '[--sector X] [--industry X] [--market-cap-min N] [--market-cap-max N] [--limit N]')}
    ${c('bold', 'search')} <query>                    Search by name or ticker
    ${c('bold', 'news')} ${c('dim', '[--stock SYM] [--limit N]')}   Latest financial news
    ${c('bold', 'technicals')} <indicator> <symbol>   Technical indicators (sma,ema,rsi,...)
        ${c('dim', '[--period N] [--timeframe 1day]')}
    ${c('bold', 'etf')} <symbol>                      ETF holdings and info
    ${c('bold', 'insider')} <symbol>                  Insider trading activity
    ${c('bold', 'sec')} <symbol>                      SEC filings
    ${c('bold', 'institutional')} <symbol>            13F institutional ownership
    ${c('bold', 'dividends')} <symbol>                Dividend history
    ${c('bold', 'macro')} <indicator>                 Economic data (GDP, CPI, etc.)
    ${c('bold', 'treasury')}                          Current US Treasury rates
    ${c('bold', 'gainers')}                           Today's biggest gainers
    ${c('bold', 'losers')}                            Today's biggest losers
    ${c('bold', 'active')}                            Most actively traded
    ${c('bold', 'tools')}                             List all ${c('cyan', '180')} MCP tools
    ${c('bold', '--help')}                            Show this help

  ${c('cyan', 'Examples:')}
    ${c('dim', 'fmp quote AAPL')}
    ${c('dim', 'fmp financials MSFT --period quarter --limit 8')}
    ${c('dim', 'fmp screen --sector Technology --market-cap-min 1000000000')}
    ${c('dim', 'fmp technicals rsi TSLA --period 14')}
    ${c('dim', 'fmp news --stock NVDA --limit 5')}
    ${c('dim', 'fmp macro GDP')}

  ${c('dim', 'Requires FMP_API_KEY env var (or .env file).')}
`);
}

// ── Main ────────────────────────────────────────────────────────────
async function main() {
  if (!command || command === '--help' || command === '-h' || command === 'help') { printHelp(); return; }
  try {
    switch (command) {
      case 'quote':         await handleQuote(); break;
      case 'profile':       await handleProfile(); break;
      case 'financials':    await handleFinancials(); break;
      case 'earnings':      await handleEarnings(); break;
      case 'screen':        await handleScreen(); break;
      case 'search':        await handleSearch(); break;
      case 'news':          await handleNews(); break;
      case 'technicals':    await handleTechnicals(); break;
      case 'etf':           await handleEtf(); break;
      case 'insider':       await handleInsider(); break;
      case 'sec':           await handleSec(); break;
      case 'institutional': await handleInstitutional(); break;
      case 'dividends':     await handleDividends(); break;
      case 'macro':         await handleMacro(); break;
      case 'treasury':      await handleTreasury(); break;
      case 'gainers':       await handleMovers('biggest-gainers', 'Biggest Gainers'); break;
      case 'losers':        await handleMovers('biggest-losers', 'Biggest Losers'); break;
      case 'active':        await handleMovers('most-actives', 'Most Active'); break;
      case 'tools':         await handleTools(); break;
      default:
        console.error(`${c('red', 'Error:')} Unknown command "${command}"\n`);
        printHelp(); process.exit(1);
    }
  } catch (err) {
    console.error(`${c('red', 'Error:')} ${err instanceof Error ? err.message : String(err)}`);
    process.exit(1);
  }
}

main();
