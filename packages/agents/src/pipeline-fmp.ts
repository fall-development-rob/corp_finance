// FMP command routing and ticker resolution for the CFA pipeline
// Extracted from pipeline.ts for ARCH-003 compliance (500-line limit)

import { join } from 'node:path';

// ── Intelligent FMP tool selection per agent type ────────────────────

export const FMP_COMMANDS: Record<string, { args: string; desc: string }> = {
  quote:               { args: 'SYMBOL',                          desc: 'Price, market cap, PE, volume' },
  financials:          { args: 'SYMBOL --period annual --limit 3', desc: 'Income statement (revenue, EBITDA, net income)' },
  'balance-sheet':     { args: 'SYMBOL --period annual --limit 3', desc: 'Balance sheet (assets, liabilities, equity)' },
  'cash-flow':         { args: 'SYMBOL --period annual --limit 3', desc: 'Cash flow (operating CF, capex, FCF)' },
  'key-metrics':       { args: 'SYMBOL --limit 1',                desc: 'EV/EBITDA, P/E, P/B, ROE, etc.' },
  ratios:              { args: 'SYMBOL --limit 1',                desc: 'Ratios (margins, turnover, coverage)' },
  earnings:            { args: 'SYMBOL',                          desc: 'Historical earnings surprises' },
  'analyst-estimates': { args: 'SYMBOL --limit 1',                desc: 'Consensus analyst estimates' },
  dividends:           { args: 'SYMBOL',                          desc: 'Dividend history' },
  profile:             { args: 'SYMBOL',                          desc: 'Company profile, sector, employees' },
  insider:             { args: 'SYMBOL',                          desc: 'Insider trading activity' },
  institutional:       { args: 'SYMBOL',                          desc: '13F institutional ownership' },
  sec:                 { args: 'SYMBOL',                          desc: 'SEC filings' },
  macro:               { args: 'GDP',                             desc: 'Economic indicators (GDP, CPI, etc.)' },
  treasury:            { args: '',                                desc: 'US Treasury rates' },
  search:              { args: '"QUERY" --json',                  desc: 'Search by company name or ticker' },
};

// Base FMP commands per agent type
export const AGENT_FMP_COMMANDS: Record<string, string[]> = {
  'cfa-equity-analyst':          ['quote', 'financials', 'cash-flow', 'earnings'],
  'cfa-credit-analyst':          ['quote', 'financials', 'balance-sheet', 'ratios'],
  'cfa-fixed-income-analyst':    ['quote', 'key-metrics', 'treasury'],
  'cfa-derivatives-analyst':     ['quote', 'key-metrics'],
  'cfa-quant-risk-analyst':      ['quote', 'key-metrics'],
  'cfa-macro-analyst':           ['quote', 'macro', 'treasury'],
  'cfa-esg-regulatory-analyst':  ['quote', 'profile', 'sec'],
  'cfa-private-markets-analyst': ['quote', 'financials', 'balance-sheet', 'cash-flow'],
  'cfa-chief-analyst':           ['quote', 'financials', 'cash-flow', 'key-metrics'],
};

// Query keywords that trigger additional FMP commands
export const QUERY_TRIGGERS: [RegExp, string[]][] = [
  [/balance.?sheet|assets|liabilities|leverage|debt.to/i,     ['balance-sheet']],
  [/cash.?flow|fcf|free cash|capex|operating cash/i,          ['cash-flow']],
  [/dividend|payout|yield|buyback/i,                          ['dividends', 'cash-flow']],
  [/earnings|eps|surprise|beat|miss|guidance/i,               ['earnings']],
  [/estimate|forecast|consensus|forward/i,                    ['analyst-estimates']],
  [/valuation|dcf|wacc|multiple|pe.ratio/i,                   ['financials', 'key-metrics']],
  [/margin|profitability|revenue|income|ebitda/i,             ['financials']],
  [/ratio|roe|roa|current.ratio|coverage/i,                   ['ratios']],
  [/macro|gdp|cpi|inflation|interest.rate|fed/i,              ['macro', 'treasury']],
  [/insider|director.deal/i,                                  ['insider']],
  [/institutional|13f|ownership/i,                            ['institutional']],
  [/sec|filing|10-k|10-q|proxy/i,                             ['sec']],
  [/profile|sector|industry|employee/i,                       ['profile']],
];

// ── Pre-flight ticker resolution via FMP search-name ────────────────

const PREFERRED_EXCHANGES = new Set([
  'NASDAQ', 'NYSE', 'AMEX', 'TSX', 'TSXV', 'LSE', 'ASX', 'HKSE',
  'EURONEXT', 'XETRA', 'SIX', 'SGX', 'JSE', 'NSE', 'BSE',
]);

export function extractExplicitTicker(query: string): string | null {
  const parenMatch = query.match(/\(([A-Z][A-Z0-9]{0,5}(?:[.\-][A-Z0-9]{1,3})?)\)/);
  if (parenMatch) return parenMatch[1];

  const labelMatch = query.match(/(?:ticker|symbol)[:\s]+([A-Z][A-Z0-9]{0,5}(?:[.\-][A-Z0-9]{1,3})?)/i);
  if (labelMatch) return labelMatch[1].toUpperCase();

  return null;
}

export function extractCompanyName(query: string): string | null {
  const cleaned = query.replace(/\([A-Z][A-Z0-9.]{0,7}\)/g, '').trim();

  const verbMatch = cleaned.match(
    /(?:analyze|analyse|assess|evaluate|review|research|rate|value|cover)\s+(.+?)(?:\s*[-–—,;:|]|\s+(?:for|with|using|and|focusing|including|on the))/i,
  );
  if (verbMatch) {
    const name = verbMatch[1].trim();
    if (name.length > 2 && !/^(the|this|that|these|those|its|my|our|their)$/i.test(name)) {
      return name;
    }
  }

  const forMatch = cleaned.match(/for\s+(.+?)$/i);
  if (forMatch) {
    const name = forMatch[1].replace(/\s*[-–—,;:|].*/g, '').trim();
    if (name.length > 2) return name;
  }

  const capMatch = cleaned.match(/\b([A-Z][a-z]+(?:\s+(?:[A-Z][a-z]+|Inc\.?|Corp\.?|Ltd\.?|PLC|SA|AG|NV|SE))+)/);
  if (capMatch) return capMatch[1].trim();

  return null;
}

export async function resolveTickerViaFmp(companyName: string, pipelineDir: string): Promise<{ symbol: string; name: string; exchange: string } | null> {
  const cliPath = join(pipelineDir, '..', '..', 'fmp-mcp-server', 'src', 'fmp-cli.ts');

  try {
    const { execFile } = await import('node:child_process');
    const { promisify } = await import('node:util');
    const execFileAsync = promisify(execFile);

    const { stdout } = await execFileAsync(
      'npx',
      ['tsx', cliPath, 'search', companyName, '--json', '--limit', '10'],
      { timeout: 10_000, env: { ...process.env } },
    );

    const results = JSON.parse(stdout.trim()) as Array<{
      symbol: string; name: string; exchange: string; currency: string;
    }>;

    if (!Array.isArray(results) || results.length === 0) return null;

    const queryLower = companyName.toLowerCase();

    const scored = results.map(r => {
      let score = 0;
      const nameLower = (r.name || '').toLowerCase();

      if (nameLower === queryLower) score += 100;
      else if (nameLower.startsWith(queryLower)) score += 80;
      else if (nameLower.includes(queryLower)) score += 60;
      else if (queryLower.includes(nameLower)) score += 40;
      else if (nameLower.split(' ')[0] === queryLower.split(' ')[0]) score += 20;

      if (PREFERRED_EXCHANGES.has(r.exchange)) score += 30;
      if (r.exchange === 'OTC' || r.exchange === 'Other OTC') score -= 20;

      return { ...r, score };
    });

    scored.sort((a, b) => b.score - a.score);
    const best = scored[0];

    if (best.score > 0) {
      return { symbol: best.symbol, name: best.name, exchange: best.exchange };
    }

    const preferred = results.find(r => PREFERRED_EXCHANGES.has(r.exchange));
    const fallback = preferred || results[0];
    return { symbol: fallback.symbol, name: fallback.name, exchange: fallback.exchange };
  } catch {
    return null;
  }
}

export async function resolveTickerFromQuery(query: string, pipelineDir: string): Promise<string | null> {
  const explicit = extractExplicitTicker(query);
  if (explicit) return explicit;

  const companyName = extractCompanyName(query);
  if (!companyName) return null;

  const match = await resolveTickerViaFmp(companyName, pipelineDir);
  return match?.symbol ?? null;
}

export function buildAgentPreamble(agentType: string, query: string, pipelineDir: string, resolvedTicker?: string | null): string {
  const cmds = new Set(AGENT_FMP_COMMANDS[agentType] ?? AGENT_FMP_COMMANDS['cfa-chief-analyst']);

  for (const [pattern, extraCmds] of QUERY_TRIGGERS) {
    if (pattern.test(query)) {
      for (const c of extraCmds) cmds.add(c);
    }
  }

  const apiKey = process.env.FMP_API_KEY;
  if (!apiKey) throw new Error('FMP_API_KEY environment variable is required');
  const cliPath = join(pipelineDir, '..', '..', 'fmp-mcp-server', 'src', 'fmp-cli.ts');
  const cliBase = `FMP_API_KEY=${apiKey} npx tsx ${cliPath}`;
  const ticker = resolvedTicker ?? 'SYMBOL';
  const cmdLines = [...cmds].map(key => {
    const c = FMP_COMMANDS[key];
    if (!c) return '';
    const args = c.args.replace(/SYMBOL/g, ticker);
    return `# ${c.desc}\n${cliBase} ${key} ${args}`;
  }).filter(Boolean).join('\n\n');

  const cmdCount = cmds.size;
  const maxTurns = Math.min(cmdCount + 4, 12);

  const tickerNote = resolvedTicker
    ? `The ticker **${resolvedTicker}** has been pre-resolved. Use it exactly as shown above.`
    : 'Replace SYMBOL with the actual ticker symbol (e.g., AAPL).';

  return `
## CRITICAL INSTRUCTIONS — Read Before Starting

1. **OUTPUT FORMAT**: Return your complete analysis as TEXT in your final message. Do NOT write files. Your text output IS the deliverable.

2. **DATA SOURCE**: Get financial data by running these FMP CLI commands via Bash. Run them IN PARALLEL:

\`\`\`bash
${cmdLines}
\`\`\`

${tickerNote}

3. **TURN BUDGET**: You have ${maxTurns} tool calls max. Run all ${cmdCount} data commands in parallel, then analyze and write your report.
   - Do NOT explore the codebase, read source files, or search for code
   - Do NOT use Read, Write, Edit, Glob, Grep, or WebSearch

4. **ACCURACY**: Every number must come from tool output. Do NOT use numbers from memory. If data is missing, say so.

---

`;
}
