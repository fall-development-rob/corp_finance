#!/usr/bin/env node
// Alpha Vantage CLI — lightweight command-line interface for Alpha Vantage data
// Usage: av-cli <command> [options]
//
// Examples:
//   av-cli quote AAPL
//   av-cli daily MSFT --outputsize full
//   av-cli overview TSLA
//   av-cli fx EUR USD
//   av-cli crypto BTC USD
//   av-cli gdp --interval quarterly
//   av-cli treasury --maturity 10year
//   av-cli sma AAPL --period 50 --interval daily
//   av-cli news --tickers AAPL,MSFT --limit 10
//   av-cli search "Apple"

import { avFetch } from './client.js';

const HELP = `
Alpha Vantage CLI

Usage: av-cli <command> [args] [options]

Commands:
  quote <symbol>              Real-time stock quote
  search <keywords>           Search for ticker symbols
  market-status               Global market status
  top-movers                  Top gainers, losers, most active

  intraday <symbol>           Intraday OHLCV time series
  daily <symbol>              Daily OHLCV time series
  weekly <symbol>             Weekly OHLCV time series
  monthly <symbol>            Monthly OHLCV time series

  overview <symbol>           Company overview and fundamentals
  income <symbol>             Income statement
  balance <symbol>            Balance sheet
  cashflow <symbol>           Cash flow statement
  earnings <symbol>           Earnings history and estimates

  fx <from> <to>              Forex exchange rate
  fx-daily <from> <to>        Forex daily time series
  crypto <symbol> [market]    Crypto exchange rate
  crypto-daily <symbol> [mkt] Crypto daily time series

  gdp                         US Real GDP
  cpi                         US Consumer Price Index
  inflation                   US Inflation rate
  fed-rate                    Federal Funds Rate
  treasury                    US Treasury yield
  unemployment                US Unemployment rate

  sma <symbol>                Simple Moving Average
  ema <symbol>                Exponential Moving Average
  rsi <symbol>                Relative Strength Index
  macd <symbol>               MACD indicator
  bbands <symbol>             Bollinger Bands

  news                        Market news with sentiment

Options:
  --interval <val>            Time interval (1min,5min,15min,30min,60min,daily,weekly,monthly)
  --outputsize <val>          compact (default) or full
  --period <n>                Time period for technicals (default: 20)
  --series-type <val>         close, open, high, low (default: close)
  --maturity <val>            Treasury maturity (3month,2year,5year,7year,10year,30year)
  --tickers <val>             Comma-separated tickers for news
  --topics <val>              News topics filter
  --limit <n>                 Max results
  --json                      Output raw JSON (default: formatted)
  --help                      Show this help

Environment:
  ALPHA_VANTAGE_API_KEY       Required. Get free key at https://www.alphavantage.co/support/#api-key
`.trim();

// ── Argument parsing ────────────────────────────────────────────────

function parseArgs(argv: string[]): { command: string; positional: string[]; flags: Record<string, string> } {
  const args = argv.slice(2);
  const command = args[0] || 'help';
  const positional: string[] = [];
  const flags: Record<string, string> = {};

  for (let i = 1; i < args.length; i++) {
    if (args[i].startsWith('--')) {
      const key = args[i].slice(2);
      if (key === 'help' || key === 'json') {
        flags[key] = 'true';
      } else {
        flags[key] = args[++i] || '';
      }
    } else {
      positional.push(args[i]);
    }
  }

  return { command, positional, flags };
}

function requireSymbol(positional: string[], label = 'symbol'): string {
  if (!positional[0]) {
    console.error(`Error: <${label}> argument is required`);
    process.exit(1);
  }
  return positional[0];
}

// ── Output formatting ───────────────────────────────────────────────

function output(data: unknown, raw: boolean): void {
  if (raw) {
    console.log(JSON.stringify(data, null, 2));
  } else {
    console.log(JSON.stringify(data, null, 2));
  }
}

// ── Command dispatch ────────────────────────────────────────────────

async function main(): Promise<void> {
  const { command, positional, flags } = parseArgs(process.argv);
  const raw = flags.json === 'true';

  if (command === 'help' || flags.help === 'true') {
    console.log(HELP);
    return;
  }

  try {
    let data: unknown;

    switch (command) {
      // ── Quotes & Search ──
      case 'quote':
        data = await avFetch({ function: 'GLOBAL_QUOTE', symbol: requireSymbol(positional) });
        break;
      case 'search':
        data = await avFetch({ function: 'SYMBOL_SEARCH', keywords: requireSymbol(positional, 'keywords') });
        break;
      case 'market-status':
        data = await avFetch({ function: 'MARKET_STATUS' });
        break;
      case 'top-movers':
        data = await avFetch({ function: 'TOP_GAINERS_LOSERS' });
        break;

      // ── Time Series ──
      case 'intraday':
        data = await avFetch({
          function: 'TIME_SERIES_INTRADAY',
          symbol: requireSymbol(positional),
          interval: flags.interval || '5min',
          outputsize: flags.outputsize || 'compact',
        });
        break;
      case 'daily':
        data = await avFetch({
          function: 'TIME_SERIES_DAILY',
          symbol: requireSymbol(positional),
          outputsize: flags.outputsize || 'compact',
        });
        break;
      case 'weekly':
        data = await avFetch({ function: 'TIME_SERIES_WEEKLY', symbol: requireSymbol(positional) });
        break;
      case 'monthly':
        data = await avFetch({ function: 'TIME_SERIES_MONTHLY', symbol: requireSymbol(positional) });
        break;

      // ── Fundamentals ──
      case 'overview':
        data = await avFetch({ function: 'COMPANY_OVERVIEW', symbol: requireSymbol(positional) });
        break;
      case 'income':
        data = await avFetch({ function: 'INCOME_STATEMENT', symbol: requireSymbol(positional) });
        break;
      case 'balance':
        data = await avFetch({ function: 'BALANCE_SHEET', symbol: requireSymbol(positional) });
        break;
      case 'cashflow':
        data = await avFetch({ function: 'CASH_FLOW', symbol: requireSymbol(positional) });
        break;
      case 'earnings':
        data = await avFetch({ function: 'EARNINGS', symbol: requireSymbol(positional) });
        break;

      // ── Forex ──
      case 'fx': {
        const from = requireSymbol(positional, 'from_currency');
        const to = positional[1] || 'USD';
        data = await avFetch({ function: 'CURRENCY_EXCHANGE_RATE', from_currency: from, to_currency: to });
        break;
      }
      case 'fx-daily': {
        const from = requireSymbol(positional, 'from_currency');
        const to = positional[1] || 'USD';
        data = await avFetch({ function: 'FX_DAILY', from_symbol: from, to_symbol: to, outputsize: 'compact' });
        break;
      }

      // ── Crypto ──
      case 'crypto': {
        const sym = requireSymbol(positional);
        const market = positional[1] || 'USD';
        data = await avFetch({ function: 'CURRENCY_EXCHANGE_RATE', from_currency: sym, to_currency: market });
        break;
      }
      case 'crypto-daily': {
        const sym = requireSymbol(positional);
        const market = positional[1] || 'USD';
        data = await avFetch({ function: 'DIGITAL_CURRENCY_DAILY', symbol: sym, market });
        break;
      }

      // ── Economics ──
      case 'gdp':
        data = await avFetch({ function: 'REAL_GDP', interval: flags.interval || 'annual' });
        break;
      case 'cpi':
        data = await avFetch({ function: 'CPI', interval: flags.interval || 'monthly' });
        break;
      case 'inflation':
        data = await avFetch({ function: 'INFLATION' });
        break;
      case 'fed-rate':
        data = await avFetch({ function: 'FEDERAL_FUNDS_RATE', interval: flags.interval || 'monthly' });
        break;
      case 'treasury':
        data = await avFetch({ function: 'TREASURY_YIELD', maturity: flags.maturity || '10year', interval: flags.interval || 'monthly' });
        break;
      case 'unemployment':
        data = await avFetch({ function: 'UNEMPLOYMENT' });
        break;

      // ── Technical Indicators ──
      case 'sma':
      case 'ema':
      case 'rsi':
      case 'adx':
      case 'bbands':
      case 'stoch':
      case 'obv':
      case 'vwap':
        data = await avFetch({
          function: command.toUpperCase(),
          symbol: requireSymbol(positional),
          interval: flags.interval || 'daily',
          time_period: flags.period || '20',
          series_type: flags['series-type'] || 'close',
        });
        break;
      case 'macd':
        data = await avFetch({
          function: 'MACD',
          symbol: requireSymbol(positional),
          interval: flags.interval || 'daily',
          series_type: flags['series-type'] || 'close',
        });
        break;

      // ── Intelligence ──
      case 'news': {
        const newsParams: Record<string, string | number> = {
          function: 'NEWS_SENTIMENT',
          sort: 'LATEST',
          limit: Number(flags.limit) || 50,
        };
        if (flags.tickers) newsParams.tickers = flags.tickers;
        if (flags.topics) newsParams.topics = flags.topics;
        data = await avFetch(newsParams);
        break;
      }

      default:
        console.error(`Unknown command: ${command}\nRun 'av-cli --help' for usage.`);
        process.exit(1);
    }

    output(data, raw);
  } catch (err) {
    console.error(`Error: ${err instanceof Error ? err.message : String(err)}`);
    process.exit(1);
  }
}

main();
