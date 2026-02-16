#!/usr/bin/env tsx
/**
 * FMP Data Fetcher for Apple Inc (AAPL)
 *
 * This script demonstrates parallel FMP tool calls to gather comprehensive
 * financial data for Apple Inc. It calls all 5 major FMP endpoints in parallel:
 * 1. fmp_quote - Current stock price and market cap
 * 2. fmp_income_statement - Latest annual income statement
 * 3. fmp_balance_sheet - Latest annual balance sheet
 * 4. fmp_cash_flow - Latest annual cash flow statement
 * 5. fmp_key_metrics - Key financial metrics (P/E, EV/EBITDA, P/B, etc.)
 *
 * Usage:
 *   FMP_API_KEY=your_key npm run ts -- scripts/fetch-aapl-data.ts
 */

import 'dotenv/config';
import { join } from 'node:path';
import { dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createFmpToolCaller } from '../bridge/fmp-bridge.js';

const __cliDir = dirname(fileURLToPath(import.meta.url));
const __pkgDir = join(__cliDir, '..');
const fmpServerPath = join(__pkgDir, '..', '..', 'fmp-mcp-server', 'dist', 'index.js');

interface FmpQuote {
  symbol: string;
  price: number;
  sharesOutstanding: number;
  [key: string]: unknown;
}

interface FmpIncomeStatement {
  symbol: string;
  revenue: number;
  operatingIncome: number;
  ebitda: number;
  netIncome: number;
  eps: number;
  [key: string]: unknown;
}

interface FmpBalanceSheet {
  symbol: string;
  totalAssets: number;
  totalStockholdersEquity: number;
  totalDebt: number;
  cash: number;
  cashAndCashEquivalents: number;
  [key: string]: unknown;
}

interface FmpCashFlow {
  symbol: string;
  operatingCashFlow: number;
  capitalExpenditure: number;
  [key: string]: unknown;
}

interface FmpKeyMetrics {
  symbol: string;
  peRatio: number;
  pbRatio: number;
  enterpriseValue: number;
  enterpriseValueOverEbitda: number;
  [key: string]: unknown;
}

interface ComprehensiveFinancialData {
  quote: FmpQuote | null;
  incomeStatement: FmpIncomeStatement | null;
  balanceSheet: FmpBalanceSheet | null;
  cashFlow: FmpCashFlow | null;
  keyMetrics: FmpKeyMetrics | null;
  timestamp: string;
}

// Helper to format large numbers
function formatNumber(n: number | undefined): string {
  if (n === undefined || n === null) return 'N/A';
  if (Math.abs(n) >= 1e12) return (n / 1e12).toFixed(2) + 'T';
  if (Math.abs(n) >= 1e9) return (n / 1e9).toFixed(2) + 'B';
  if (Math.abs(n) >= 1e6) return (n / 1e6).toFixed(2) + 'M';
  return n.toFixed(2);
}

// Helper to calculate derived metrics
function calculateDerivedMetrics(data: ComprehensiveFinancialData) {
  const result: Record<string, string> = {};

  if (data.quote && data.quote.sharesOutstanding && data.quote.price) {
    const marketCap = data.quote.price * data.quote.sharesOutstanding;
    result['Market Cap'] = formatNumber(marketCap);
  }

  if (data.incomeStatement) {
    const { revenue, operatingIncome, netIncome, ebitda } = data.incomeStatement;
    if (revenue) {
      if (operatingIncome) result['Operating Margin'] = `${((operatingIncome / revenue) * 100).toFixed(2)}%`;
      if (netIncome) result['Net Margin'] = `${((netIncome / revenue) * 100).toFixed(2)}%`;
      if (ebitda) result['EBITDA Margin'] = `${((ebitda / revenue) * 100).toFixed(2)}%`;
    }
  }

  if (data.balanceSheet && data.incomeStatement) {
    const { totalAssets, totalStockholdersEquity } = data.balanceSheet;
    const { netIncome } = data.incomeStatement;
    if (totalAssets && netIncome) result['ROA'] = `${((netIncome / totalAssets) * 100).toFixed(2)}%`;
    if (totalStockholdersEquity && netIncome) result['ROE'] = `${((netIncome / totalStockholdersEquity) * 100).toFixed(2)}%`;
  }

  if (data.cashFlow && data.incomeStatement) {
    const { operatingCashFlow, capitalExpenditure } = data.cashFlow;
    if (operatingCashFlow && capitalExpenditure) {
      const freeCashFlow = operatingCashFlow + capitalExpenditure; // capex is usually negative
      result['Free Cash Flow'] = formatNumber(freeCashFlow);
    }
  }

  return result;
}

async function main() {
  // Validate API key
  if (!process.env.FMP_API_KEY) {
    console.error('Error: FMP_API_KEY environment variable is required');
    console.error('Usage: FMP_API_KEY=your_key npm run ts -- scripts/fetch-aapl-data.ts');
    process.exit(1);
  }

  console.log('\n╔════════════════════════════════════════════════════════════════╗');
  console.log('║         FMP Comprehensive Financial Data Fetcher for AAPL        ║');
  console.log('╚════════════════════════════════════════════════════════════════╝\n');

  try {
    // Connect to FMP bridge
    console.log('Connecting to FMP MCP server...');
    const { callFmpTool } = await createFmpToolCaller({ serverPath: fmpServerPath });
    console.log('Connected.\n');

    // Prepare parallel calls
    const symbol = 'AAPL';
    console.log(`Fetching comprehensive financial data for ${symbol} in parallel...\n`);

    const startTime = Date.now();

    // Execute all 5 calls in parallel
    const [quoteResult, incomeResult, balanceResult, cashFlowResult, metricsResult] = await Promise.allSettled([
      callFmpTool('fmp_quote', { symbol }),
      callFmpTool('fmp_income_statement', { symbol, period: 'annual', limit: 1 }),
      callFmpTool('fmp_balance_sheet', { symbol, period: 'annual', limit: 1 }),
      callFmpTool('fmp_cash_flow', { symbol, period: 'annual', limit: 1 }),
      callFmpTool('fmp_key_metrics', { symbol, period: 'annual', limit: 1 }),
    ]);

    const duration = Date.now() - startTime;

    // Extract results
    const data: ComprehensiveFinancialData = {
      quote: quoteResult.status === 'fulfilled' ? (Array.isArray(quoteResult.value) ? quoteResult.value[0] : quoteResult.value) : null,
      incomeStatement: incomeResult.status === 'fulfilled' ? (Array.isArray(incomeResult.value) ? incomeResult.value[0] : incomeResult.value) : null,
      balanceSheet: balanceResult.status === 'fulfilled' ? (Array.isArray(balanceResult.value) ? balanceResult.value[0] : balanceResult.value) : null,
      cashFlow: cashFlowResult.status === 'fulfilled' ? (Array.isArray(cashFlowResult.value) ? cashFlowResult.value[0] : cashFlowResult.value) : null,
      keyMetrics: metricsResult.status === 'fulfilled' ? (Array.isArray(metricsResult.value) ? metricsResult.value[0] : metricsResult.value) : null,
      timestamp: new Date().toISOString(),
    };

    // Report fetch status
    console.log(`Fetch Status (${duration}ms):`);
    console.log(`  ✓ Quote: ${quoteResult.status === 'fulfilled' ? 'SUCCESS' : 'FAILED'}`);
    console.log(`  ✓ Income Statement: ${incomeResult.status === 'fulfilled' ? 'SUCCESS' : 'FAILED'}`);
    console.log(`  ✓ Balance Sheet: ${balanceResult.status === 'fulfilled' ? 'SUCCESS' : 'FAILED'}`);
    console.log(`  ✓ Cash Flow: ${cashFlowResult.status === 'fulfilled' ? 'SUCCESS' : 'FAILED'}`);
    console.log(`  ✓ Key Metrics: ${metricsResult.status === 'fulfilled' ? 'SUCCESS' : 'FAILED'}\n`);

    // Display Results
    console.log('═══════════════════════════════════════════════════════════════════\n');
    console.log('QUOTE DATA (Current Price & Market Cap)\n');
    console.log('─────────────────────────────────────────────────────────────────');

    if (data.quote) {
      console.log(`Symbol:               ${data.quote.symbol}`);
      console.log(`Share Price:          $${data.quote.price?.toFixed(2)}`);
      console.log(`Shares Outstanding:   ${formatNumber(data.quote.sharesOutstanding)}`);
      if (data.quote.price && data.quote.sharesOutstanding) {
        console.log(`Market Cap:           $${formatNumber(data.quote.price * data.quote.sharesOutstanding)}`);
      }
    } else {
      console.log('No quote data available');
    }

    console.log('\n═══════════════════════════════════════════════════════════════════\n');
    console.log('INCOME STATEMENT DATA (Revenue, Operating Income, Net Income)\n');
    console.log('─────────────────────────────────────────────────────────────────');

    if (data.incomeStatement) {
      console.log(`Revenue:              $${formatNumber(data.incomeStatement.revenue)}`);
      console.log(`Cost of Revenue:      $${formatNumber((data.incomeStatement as any).costOfRevenue)}`);
      console.log(`Operating Income:     $${formatNumber(data.incomeStatement.operatingIncome)}`);
      console.log(`EBIT:                 $${formatNumber(data.incomeStatement.operatingIncome)}`);
      console.log(`EBITDA:               $${formatNumber(data.incomeStatement.ebitda)}`);
      console.log(`Interest Expense:     $${formatNumber((data.incomeStatement as any).interestExpense)}`);
      console.log(`Net Income:           $${formatNumber(data.incomeStatement.netIncome)}`);
      console.log(`EPS:                  $${data.incomeStatement.eps?.toFixed(2)}`);
    } else {
      console.log('No income statement data available');
    }

    console.log('\n═══════════════════════════════════════════════════════════════════\n');
    console.log('BALANCE SHEET DATA (Assets, Equity, Debt, Cash)\n');
    console.log('─────────────────────────────────────────────────────────────────');

    if (data.balanceSheet) {
      console.log(`Total Assets:         $${formatNumber(data.balanceSheet.totalAssets)}`);
      console.log(`Current Assets:       $${formatNumber((data.balanceSheet as any).totalCurrentAssets)}`);
      console.log(`Receivables:          $${formatNumber((data.balanceSheet as any).netReceivables)}`);
      console.log(`Inventory:            $${formatNumber((data.balanceSheet as any).inventory)}`);
      console.log(`PP&E (net):           $${formatNumber((data.balanceSheet as any).propertyPlantEquipmentNet)}`);
      console.log(`\nTotal Liabilities:    $${formatNumber((data.balanceSheet as any).totalLiabilities)}`);
      console.log(`Current Liabilities:  $${formatNumber((data.balanceSheet as any).totalCurrentLiabilities)}`);
      console.log(`Total Debt:           $${formatNumber(data.balanceSheet.totalDebt)}`);
      console.log(`\nStockholders Equity:  $${formatNumber(data.balanceSheet.totalStockholdersEquity)}`);
      console.log(`Cash & Equivalents:   $${formatNumber(data.balanceSheet.cashAndCashEquivalents)}`);
      console.log(`Net Debt:             $${formatNumber((data.balanceSheet as any).netDebt)}`);
    } else {
      console.log('No balance sheet data available');
    }

    console.log('\n═══════════════════════════════════════════════════════════════════\n');
    console.log('CASH FLOW DATA (Operating CF, CapEx, Free CF)\n');
    console.log('─────────────────────────────────────────────────────────────────');

    if (data.cashFlow) {
      console.log(`Operating Cash Flow: $${formatNumber(data.cashFlow.operatingCashFlow)}`);
      console.log(`Capital Expenditure:  $${formatNumber(Math.abs(data.cashFlow.capitalExpenditure))}`);
      if (data.cashFlow.operatingCashFlow && data.cashFlow.capitalExpenditure) {
        const freeCF = data.cashFlow.operatingCashFlow + data.cashFlow.capitalExpenditure;
        console.log(`Free Cash Flow:       $${formatNumber(freeCF)}`);
      }
      console.log(`Dividend Paid:        $${formatNumber((data.cashFlow as any).dividendsPaid)}`);
    } else {
      console.log('No cash flow data available');
    }

    console.log('\n═══════════════════════════════════════════════════════════════════\n');
    console.log('KEY METRICS (P/E, EV/EBITDA, P/B, EV, ROE, ROA)\n');
    console.log('─────────────────────────────────────────────────────────────────');

    if (data.keyMetrics) {
      console.log(`P/E Ratio:            ${data.keyMetrics.peRatio?.toFixed(2)}`);
      console.log(`P/B Ratio:            ${data.keyMetrics.pbRatio?.toFixed(2)}`);
      console.log(`EV/EBITDA:            ${data.keyMetrics.enterpriseValueOverEbitda?.toFixed(2)}`);
      console.log(`Enterprise Value:     $${formatNumber(data.keyMetrics.enterpriseValue)}`);
      console.log(`Debt/Equity:          ${((data.keyMetrics as any).debtToEquity)?.toFixed(2)}`);
      console.log(`Current Ratio:        ${((data.keyMetrics as any).currentRatio)?.toFixed(2)}`);
      console.log(`Interest Coverage:    ${((data.keyMetrics as any).interestCoverage)?.toFixed(2)}x`);
    } else {
      console.log('No key metrics data available');
    }

    // Derived metrics
    const derived = calculateDerivedMetrics(data);
    if (Object.keys(derived).length > 0) {
      console.log('\n═══════════════════════════════════════════════════════════════════\n');
      console.log('DERIVED METRICS (Calculated)\n');
      console.log('─────────────────────────────────────────────────────────────────');
      for (const [key, value] of Object.entries(derived)) {
        console.log(`${key.padEnd(20)}: ${value}`);
      }
    }

    // Summary
    console.log('\n═══════════════════════════════════════════════════════════════════\n');
    console.log('SUMMARY\n');
    console.log('─────────────────────────────────────────────────────────────────');
    console.log(`Data Fetched:         ${[
      data.quote ? '1 (quote)' : null,
      data.incomeStatement ? '1 (income)' : null,
      data.balanceSheet ? '1 (balance)' : null,
      data.cashFlow ? '1 (cashflow)' : null,
      data.keyMetrics ? '1 (metrics)' : null,
    ]
      .filter(Boolean)
      .join(' + ')} = 5 endpoints`);
    console.log(`Total Fetch Time:     ${duration}ms`);
    console.log(`Timestamp:            ${data.timestamp}`);
    console.log();

    // Raw JSON output option
    console.log('\n═══════════════════════════════════════════════════════════════════\n');
    console.log('RAW JSON DATA\n');
    console.log('─────────────────────────────────────────────────────────────────');
    console.log(JSON.stringify(data, null, 2));
  } catch (error) {
    console.error('Error:', error instanceof Error ? error.message : String(error));
    process.exit(1);
  }
}

main().catch((err) => {
  console.error('Fatal error:', err);
  process.exit(1);
});
