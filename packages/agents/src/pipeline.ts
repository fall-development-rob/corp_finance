// CFA Multi-Agent Pipeline — 6-stage orchestration
//
// User Request → Task Router → Agents (dynamic) → Coordination (attention)
//   → Vector Search (HNSW+GNN) → Synthesis (consensus) → Response

import { randomUUID } from 'node:crypto';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { dirname, join } from 'node:path';
import { existsSync, readFileSync } from 'node:fs';
import { createRequire } from 'node:module';

import type { AgentDefinition } from 'agentic-flow/dist/utils/agentLoader.js';
import type { AgentIntent, MultiIntentResult } from 'agentic-flow/dist/routing/SemanticRouter.js';
import type { AgentOutput, SwarmTopology, CoordinationResult } from 'agentic-flow/dist/coordination/attention-coordinator.js';
import type { ReasoningBank } from '../learning/reasoning-bank.js';
import type { FinancialMemory } from '../memory/financial-memory.js';
import type { TaskType } from '../types/learning.js';

// ── Re-export topology type ─────────────────────────────────────────

export type Topology = SwarmTopology;

// ── Pipeline types ──────────────────────────────────────────────────

export interface PipelineConfig {
  topology: Topology;
  confidenceThreshold: number;
  maxAgents: number;
  attentionMechanism: 'flash' | 'multi-head';
  enableLearning: boolean;
  onStatus?: (stage: string, message: string) => void;
}

export interface AgentResult {
  agentName: string;
  agentType: string;
  output: string;
  attentionWeight: number;
}

export interface PipelineResult {
  requestId: string;
  synthesis: string;
  agentResults: AgentResult[];
  routedAgents: string[];
  topology: Topology;
  coordination?: {
    mechanism: string;
    executionTimeMs: number;
    topAgents: string[];
    attentionWeights: number[];
  };
  timings: {
    routingMs: number;
    memoryMs: number;
    agentsMs: number;
    coordinationMs: number;
    synthesisMs: number;
    totalMs: number;
  };
}

export class PipelineError extends Error {
  constructor(
    message: string,
    public readonly stage: string,
    public readonly cause?: unknown,
  ) {
    super(message);
    this.name = 'PipelineError';
  }
}

// ── Directories ─────────────────────────────────────────────────────

const __pipelineDir = dirname(fileURLToPath(import.meta.url));
// From src/pipeline.ts: ../../../ = cfa_agent (packages/agents/src → packages/agents → packages → cfa_agent)
// From dist/pipeline.js: ../../../ = cfa_agent (packages/agents/dist → packages/agents → packages → cfa_agent)
const repoRoot = join(__pipelineDir, '..', '..', '..');
const cfaAgentsDir = join(repoRoot, '.claude', 'agents', 'cfa');
const skillsDir = join(repoRoot, '.claude', 'skills');

// ── Skill injection (shared with cli.ts) ────────────────────────────

export const AGENT_SKILLS: Record<string, string[]> = {
  'cfa-chief-analyst': [
    'corp-finance-tools-core',
    'corp-finance-tools-markets',
    'corp-finance-tools-risk',
    'corp-finance-tools-regulatory',
    'fmp-market-data',
    'fmp-research',
    'fmp-news-intelligence',
    'fmp-sec-compliance',
  ],
  'cfa-equity-analyst':          ['corp-finance-tools-core', 'fmp-market-data', 'fmp-technicals', 'fmp-news-intelligence'],
  'cfa-credit-analyst':          ['corp-finance-tools-core', 'fmp-market-data', 'fmp-sec-compliance'],
  'cfa-private-markets-analyst': ['corp-finance-tools-core', 'fmp-market-data', 'fmp-sec-compliance'],
  'cfa-fixed-income-analyst':    ['corp-finance-tools-markets', 'fmp-market-data'],
  'cfa-derivatives-analyst':     ['corp-finance-tools-markets', 'fmp-market-data', 'fmp-technicals'],
  'cfa-macro-analyst':           ['corp-finance-tools-markets', 'fmp-research', 'fmp-news-intelligence'],
  'cfa-quant-risk-analyst':      ['corp-finance-tools-risk', 'fmp-market-data', 'fmp-technicals', 'fmp-etf-funds'],
  'cfa-esg-regulatory-analyst':  ['corp-finance-tools-regulatory', 'fmp-research', 'fmp-sec-compliance'],
};

const skillCache = new Map<string, string>();

function readSkillBody(skillName: string): string {
  if (skillCache.has(skillName)) return skillCache.get(skillName)!;

  const skillPath = join(skillsDir, skillName, 'SKILL.md');
  if (!existsSync(skillPath)) return '';

  const raw = readFileSync(skillPath, 'utf-8');
  const body = raw.replace(/^---\n[\s\S]*?\n---\n/, '').trim();
  skillCache.set(skillName, body);
  return body;
}

export function injectSkills(agent: AgentDefinition): AgentDefinition {
  const skills = AGENT_SKILLS[agent.name];
  if (!skills || skills.length === 0) return agent;

  const skillContent = skills
    .map(readSkillBody)
    .filter(Boolean)
    .join('\n\n---\n\n');

  if (!skillContent) return agent;

  return {
    ...agent,
    systemPrompt: agent.systemPrompt + '\n\n---\n\n# MCP Tool Reference\n\n' + skillContent,
  };
}

// ── CFA agent intent definitions for HNSW routing ──────────────────

const CFA_INTENTS: AgentIntent[] = [
  {
    agentType: 'cfa-chief-analyst',
    description: 'Research coordination, query decomposition, specialist delegation, result aggregation, quality gating',
    examples: [
      'Give me a comprehensive analysis of this company',
      'Prepare an institutional research report',
      'What are the key risks and opportunities here?',
    ],
    tags: ['coordination', 'research', 'aggregation', 'report', 'comprehensive', 'delegation', 'quality'],
  },
  {
    agentType: 'cfa-equity-analyst',
    description: 'Equity valuation, DCF, trading comps, earnings quality, dividend policy, financial forensics, target price',
    examples: [
      'Calculate WACC for beta 1.2, risk-free rate 4%',
      'Run a DCF model for revenue $500M growing at 8%',
      'What is the Beneish M-Score for these financials?',
      'Derive a target price using PE and DDM methods',
    ],
    tags: ['equity', 'valuation', 'dcf', 'wacc', 'comps', 'earnings', 'dividend', 'target-price', 'forensics', 'pe-ratio'],
  },
  {
    agentType: 'cfa-credit-analyst',
    description: 'Credit ratings, spreads, default probability, covenants, restructuring, debt capacity, Altman Z-score',
    examples: [
      'Assess credit quality: D/E 0.6, interest coverage 5x',
      'Calculate Altman Z-score for these metrics',
      'Evaluate covenant compliance for this debt structure',
      'What is the probability of default given these financials?',
    ],
    tags: ['credit', 'ratings', 'spreads', 'default', 'covenants', 'restructuring', 'debt', 'z-score', 'leverage'],
  },
  {
    agentType: 'cfa-fixed-income-analyst',
    description: 'Bond pricing, yield curves, duration/convexity, MBS analytics, municipal bonds, sovereign debt, repo financing',
    examples: [
      'Price this 10-year bond with 5% coupon at par',
      'Bootstrap the yield curve from these swap rates',
      'Calculate duration and convexity for this portfolio',
      'Analyze prepayment risk for this MBS tranche',
    ],
    tags: ['bonds', 'yield', 'duration', 'convexity', 'mbs', 'municipal', 'sovereign', 'repo', 'fixed-income', 'rates'],
  },
  {
    agentType: 'cfa-derivatives-analyst',
    description: 'Options pricing, implied volatility, vol surfaces, convertibles, structured products, real options, Greeks',
    examples: [
      'Price this call option using Black-Scholes',
      'Build a volatility surface from these market quotes',
      'Value this convertible bond with credit spread 200bps',
      'Calculate Greeks for this options portfolio',
    ],
    tags: ['options', 'volatility', 'greeks', 'derivatives', 'convertibles', 'structured', 'black-scholes', 'swaps'],
  },
  {
    agentType: 'cfa-quant-risk-analyst',
    description: 'VaR, factor models, portfolio optimization, risk budgeting, stress testing, market microstructure',
    examples: [
      'Calculate 99% VaR for this portfolio',
      'Run Markowitz mean-variance optimization',
      'Decompose risk by factor using Barra model',
      'Stress test portfolio against 2008 scenario',
    ],
    tags: ['var', 'risk', 'portfolio', 'optimization', 'factors', 'stress-test', 'sharpe', 'quant', 'microstructure'],
  },
  {
    agentType: 'cfa-macro-analyst',
    description: 'Interest rates, FX, commodities, emerging markets, trade finance, sovereign analysis, inflation',
    examples: [
      'What is the macro outlook for emerging markets?',
      'Analyze FX carry trade for USD/JPY',
      'Forecast commodity prices given supply constraints',
      'Evaluate sovereign credit risk for Brazil',
    ],
    tags: ['macro', 'rates', 'fx', 'commodities', 'emerging-markets', 'inflation', 'sovereign', 'gdp', 'trade'],
  },
  {
    agentType: 'cfa-esg-regulatory-analyst',
    description: 'ESG scores, carbon markets, compliance, AML/KYC, FATCA/CRS, tax treaties, transfer pricing, regulatory reporting',
    examples: [
      'Calculate ESG composite score for this company',
      'Assess carbon exposure and transition risk',
      'Check FATCA compliance for this structure',
      'Evaluate transfer pricing for intercompany transactions',
    ],
    tags: ['esg', 'carbon', 'compliance', 'aml', 'fatca', 'regulatory', 'tax', 'transfer-pricing', 'sustainability'],
  },
  {
    agentType: 'cfa-private-markets-analyst',
    description: 'PE/LBO models, M&A, venture, private credit, CLO/securitization, infrastructure, fund-of-funds, waterfall',
    examples: [
      'Build an LBO model with 4x leverage and 5-year hold',
      'Calculate IRR and MOIC for this PE deal',
      'Analyze CLO waterfall with these tranche specs',
      'Value this venture deal at Series B',
    ],
    tags: ['pe', 'lbo', 'ma', 'venture', 'private-credit', 'clo', 'infrastructure', 'irr', 'waterfall', 'fund'],
  },
];

// ── Agent name normalization ────────────────────────────────────────

function agentNameFromType(agentType: string): string {
  // Agent types match the frontmatter 'name' field (e.g. 'cfa-equity-analyst')
  return agentType;
}

function inferTaskType(agentType: string): TaskType {
  if (agentType.includes('equity')) return 'valuation';
  if (agentType.includes('credit')) return 'credit_assessment';
  if (agentType.includes('risk') || agentType.includes('quant')) return 'risk_analysis';
  if (agentType.includes('macro')) return 'macro_research';
  if (agentType.includes('esg') || agentType.includes('regulatory')) return 'esg_review';
  if (agentType.includes('private') || agentType.includes('pe')) return 'deal_analysis';
  if (agentType.includes('fixed') || agentType.includes('income')) return 'portfolio_construction';
  if (agentType.includes('derivatives')) return 'risk_analysis';
  return 'valuation';
}

// ── Embedder adapter ────────────────────────────────────────────────
// SemanticRouter's cosineSimilarity iterates embed() results as number[],
// but EmbeddingService.embed() returns { embedding: number[], latency }.
// This adapter makes embed() return the raw array so HNSW similarity works.

function createRouterEmbedder(realEmbedder: any): any {
  return {
    async embed(text: string) {
      const result = await realEmbedder.embed(text);
      return result.embedding;
    },
    async embedBatch(texts: string[]) {
      const results = await realEmbedder.embedBatch(texts);
      return results.map((r: any) => r.embedding);
    },
  };
}

// ── Default config ──────────────────────────────────────────────────

const DEFAULT_CONFIG: PipelineConfig = {
  topology: 'hierarchical',
  confidenceThreshold: 0.3,
  maxAgents: 6,
  attentionMechanism: 'flash',
  enableLearning: true,
};

const AGENT_TIMEOUT_MS = 180_000;
const AGENT_MAX_TURNS = 12;
const DEFAULT_AGENT = 'cfa-chief-analyst';

// ── Intelligent FMP tool selection per agent type ────────────────────
// Instead of listing all 7+ FMP commands in every prompt, we select only
// the commands relevant to the agent's domain + query keywords.

const FMP_COMMANDS: Record<string, { args: string; desc: string }> = {
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
};

// Base FMP commands per agent type — the minimum data each specialist needs
const AGENT_FMP_COMMANDS: Record<string, string[]> = {
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

// Query keywords that trigger additional FMP commands beyond the base set
const QUERY_TRIGGERS: [RegExp, string[]][] = [
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
// Resolves company names to tickers BEFORE spawning agents so they get
// exact commands like `fmp-cli quote AIR.V` instead of `fmp-cli quote SYMBOL`.

/** Preferred exchanges in priority order (primary listings over OTC) */
const PREFERRED_EXCHANGES = new Set([
  'NASDAQ', 'NYSE', 'AMEX', 'TSX', 'TSXV', 'LSE', 'ASX', 'HKSE',
  'EURONEXT', 'XETRA', 'SIX', 'SGX', 'JSE', 'NSE', 'BSE',
]);

/**
 * Extract an explicit ticker from the query, e.g. "(AAPL)" or "(AIR.V)".
 * Returns null if none found.
 */
function extractExplicitTicker(query: string): string | null {
  // Match ticker in parens: (AAPL), (AIR.V), (BRK-B), (TSLA)
  const parenMatch = query.match(/\(([A-Z][A-Z0-9]{0,5}(?:[.\-][A-Z0-9]{1,3})?)\)/);
  if (parenMatch) return parenMatch[1];

  // Match "ticker: AAPL" or "ticker AAPL" or "symbol: AIR.V"
  const labelMatch = query.match(/(?:ticker|symbol)[:\s]+([A-Z][A-Z0-9]{0,5}(?:[.\-][A-Z0-9]{1,3})?)/i);
  if (labelMatch) return labelMatch[1].toUpperCase();

  return null;
}

/**
 * Extract a company name from the query for FMP search.
 * Looks for patterns like "Analyze Clean Air Metals Inc" or "for Clean Air Metals".
 */
function extractCompanyName(query: string): string | null {
  // Remove any explicit ticker in parens first
  const cleaned = query.replace(/\([A-Z][A-Z0-9.]{0,7}\)/g, '').trim();

  // Pattern: verb + company name (up to 6 words before a dash, comma, or end)
  const verbMatch = cleaned.match(
    /(?:analyze|analyse|assess|evaluate|review|research|rate|value|cover)\s+(.+?)(?:\s*[-–—,;:|]|\s+(?:for|with|using|and|focusing|including|on the))/i,
  );
  if (verbMatch) {
    const name = verbMatch[1].trim();
    // Filter out generic phrases that aren't company names
    if (name.length > 2 && !/^(the|this|that|these|those|its|my|our|their)$/i.test(name)) {
      return name;
    }
  }

  // Pattern: "for <Company>" at the end
  const forMatch = cleaned.match(/for\s+(.+?)$/i);
  if (forMatch) {
    const name = forMatch[1].replace(/\s*[-–—,;:|].*/g, '').trim();
    if (name.length > 2) return name;
  }

  // Fallback: Look for capitalised multi-word names (e.g. "Clean Air Metals Inc")
  const capMatch = cleaned.match(/\b([A-Z][a-z]+(?:\s+(?:[A-Z][a-z]+|Inc\.?|Corp\.?|Ltd\.?|PLC|SA|AG|NV|SE))+)/);
  if (capMatch) return capMatch[1].trim();

  return null;
}

/**
 * Resolve a company name to a ticker using the FMP CLI `search --json` command.
 * Uses the same FMP infrastructure (MCP client, caching, rate limiting) as agents.
 * Prefers primary exchange listings over OTC via fuzzy name matching.
 */
async function resolveTickerViaFmp(companyName: string): Promise<{ symbol: string; name: string; exchange: string } | null> {
  const cliPath = join(__pipelineDir, '..', '..', 'fmp-mcp-server', 'src', 'fmp-cli.ts');

  try {
    const { exec } = await import('node:child_process');
    const { promisify } = await import('node:util');
    const execAsync = promisify(exec);

    const escapedName = companyName.replace(/"/g, '\\"');
    const { stdout } = await execAsync(
      `npx tsx "${cliPath}" search "${escapedName}" --json --limit 10`,
      { timeout: 10_000, env: { ...process.env } },
    );

    const results = JSON.parse(stdout.trim()) as Array<{
      symbol: string; name: string; exchange: string; currency: string;
    }>;

    if (!Array.isArray(results) || results.length === 0) return null;

    const queryLower = companyName.toLowerCase();

    // Score each result: name similarity + exchange preference
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
    return null; // graceful fallback — agents will handle resolution
  }
}

/**
 * Resolve the ticker from the query. Tries explicit ticker first,
 * then FMP search-name API with fuzzy matching.
 */
async function resolveTickerFromQuery(query: string): Promise<string | null> {
  // 1. Check for explicit ticker in query: "(AAPL)", "(AIR.V)"
  const explicit = extractExplicitTicker(query);
  if (explicit) return explicit;

  // 2. Extract company name and search FMP
  const companyName = extractCompanyName(query);
  if (!companyName) return null;

  const match = await resolveTickerViaFmp(companyName);
  return match?.symbol ?? null;
}

function buildAgentPreamble(agentType: string, query: string, resolvedTicker?: string | null): string {
  // Start with agent's base commands
  const cmds = new Set(AGENT_FMP_COMMANDS[agentType] ?? AGENT_FMP_COMMANDS['cfa-chief-analyst']);

  // Add query-triggered commands
  for (const [pattern, extraCmds] of QUERY_TRIGGERS) {
    if (pattern.test(query)) {
      for (const c of extraCmds) cmds.add(c);
    }
  }

  // Build the CLI command block
  const apiKey = process.env.FMP_API_KEY;
  if (!apiKey) throw new Error('FMP_API_KEY environment variable is required');
  const cliPath = join(__pipelineDir, '..', '..', 'fmp-mcp-server', 'src', 'fmp-cli.ts');
  const cliBase = `FMP_API_KEY=${apiKey} npx tsx ${cliPath}`;
  // Replace SYMBOL with the resolved ticker if available
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


// ── Dedup helper ────────────────────────────────────────────────────
// The SDK sometimes emits the same analysis text across multiple assistant
// turns, causing the final output to contain the report duplicated. This
// detects and removes the duplicate by finding a repeated heading anchor.
function deduplicateOutput(text: string): string {
  if (text.length < 1500) return text;
  // Find the first markdown heading (e.g., "# APPLE INC (AAPL)")
  const headingMatch = text.match(/^(#{1,3}\s+.{5,})/m);
  if (!headingMatch) return text;
  const anchor = headingMatch[1];
  const firstIdx = text.indexOf(anchor);
  const secondIdx = text.indexOf(anchor, firstIdx + anchor.length);
  if (secondIdx > firstIdx) {
    // Anchor appears twice — keep whichever half is longer (more complete)
    const firstHalf = text.slice(firstIdx, secondIdx).trim();
    const secondHalf = text.slice(secondIdx).trim();
    return secondHalf.length >= firstHalf.length ? secondHalf : firstHalf;
  }
  // Fallback: check if the second half is a near-duplicate of the first half
  // by comparing the middle region of text
  const mid = Math.floor(text.length / 2);
  const probe = text.slice(mid, mid + 200);
  const probeIdx = text.indexOf(probe);
  if (probeIdx >= 0 && probeIdx < mid - 200) {
    // The probe from the middle also appears in the first half — likely a full dupe
    return text.slice(0, mid).trim();
  }
  return text;
}

// ── Agent runner with abort support ─────────────────────────────────
// Calls the Claude Agent SDK directly with maxTurns and AbortController
// so timed-out agents stop immediately instead of burning tokens.

async function runAgentWithAbort(
  agent: AgentDefinition,
  input: string,
  opts: { timeoutMs: number; maxTurns: number; onToolCall?: (name: string, count: number) => void },
): Promise<{ output: string; agent: string }> {
  const { query } = await import('@anthropic-ai/claude-agent-sdk');
  const ac = new AbortController();
  const timer = setTimeout(() => ac.abort(), opts.timeoutMs);

  // Load MCP servers from user config
  const mcpServers: Record<string, any> = {};
  try {
    const fs = await import('fs');
    const path = await import('path');
    const os = await import('os');
    const configPath = path.join(os.homedir(), '.agentic-flow', 'mcp-config.json');
    if (fs.existsSync(configPath)) {
      const config = JSON.parse(fs.readFileSync(configPath, 'utf-8'));
      for (const [name, server] of Object.entries(config.servers || {})) {
        const s = server as any;
        if (s.enabled) {
          mcpServers[name] = { type: 'stdio', command: s.command, args: s.args || [], env: { ...process.env, ...s.env } };
        }
      }
    }
  } catch { /* no config */ }

  const assistantChunks: string[] = [];
  let resultOutput = '';
  let toolCallCount = 0;
  const toolResults: string[] = [];

  try {
    const hasMcp = Object.keys(mcpServers).length > 0;
    const stream = query({
      prompt: input,
      options: {
        systemPrompt: agent.systemPrompt,
        model: process.env.CFA_MODEL || process.env.COMPLETION_MODEL || 'claude-haiku-4-5-20251001',
        permissionMode: 'bypassPermissions',
        allowDangerouslySkipPermissions: true,
        maxTurns: opts.maxTurns,
        abortController: ac,
        // Only allow Bash (for FMP CLI) — no file tools, no web search
        tools: ['Bash'],
        disallowedTools: ['WebSearch', 'WebFetch', 'Write', 'Edit', 'Read', 'Glob', 'Grep', 'NotebookEdit', 'Task'],
        mcpServers: hasMcp ? mcpServers : undefined,
      } as any,
    });
    for await (const msg of stream) {
      if (ac.signal.aborted) break;
      const msgType = (msg as any).type;
      if (msgType === 'assistant') {
        const chunk = (msg as any).message?.content?.map((c: any) => c.type === 'text' ? c.text : '').join('') || '';
        if (chunk.length > 0) assistantChunks.push(chunk);
        const toolBlocks = (msg as any).message?.content?.filter((c: any) => c.type === 'tool_use') || [];
        for (const tb of toolBlocks) {
          toolCallCount++;
          opts.onToolCall?.(tb.name || 'unknown', toolCallCount);
        }
      } else if (msgType === 'user') {
        // Capture tool results — these contain the actual data/analysis
        const toolResult = (msg as any).tool_use_result;
        if (toolResult && typeof toolResult === 'string' && toolResult.length > 50) {
          toolResults.push(toolResult.slice(0, 4000));
        } else if (toolResult && typeof toolResult === 'object') {
          const s = JSON.stringify(toolResult).slice(0, 4000);
          if (s.length > 50) toolResults.push(s);
        }
      } else if (msgType === 'result') {
        // SDK final result message — contains the authoritative output
        resultOutput = (msg as any).result || '';
      }
    }

    clearTimeout(timer);
    // Pick the longest assistant chunk — that's the actual analysis.
    // Short chunks are "thinking" narration before/between tool calls.
    const longestChunk = assistantChunks.reduce((a, b) => b.length > a.length ? b : a, '');
    // Prefer the longest chunk if substantial, else fall back to SDK result
    const raw = (longestChunk.length > 500 ? longestChunk : resultOutput) || assistantChunks.join('\n');
    return { output: deduplicateOutput(raw), agent: agent.name };
  } catch (err) {
    clearTimeout(timer);
    if (ac.signal.aborted) {
      // On timeout, return whatever we accumulated instead of throwing
      const longestPartial = assistantChunks.reduce((a, b) => b.length > a.length ? b : a, '');
      const partial = resultOutput || longestPartial || assistantChunks.join('\n');
      if (partial.length > 100) {
        return { output: `[Partial — timed out after ${opts.timeoutMs / 1000}s]\n\n${partial}`, agent: agent.name };
      }
      // If no text output but we have tool results, synthesize from those
      if (toolResults.length > 0) {
        const synthesized = toolResults.slice(-5).join('\n\n---\n\n');
        return { output: `[Partial — timed out after ${opts.timeoutMs / 1000}s, ${toolCallCount} tool calls]\n\nTool results:\n${synthesized}`, agent: agent.name };
      }
      throw new Error(`Agent ${agent.name} timed out after ${opts.timeoutMs / 1000}s`);
    }
    throw err;
  }
}

// ── Pipeline class ──────────────────────────────────────────────────

export class CfaPipeline {
  private config: PipelineConfig;
  private initialized = false;

  // agentic-flow modules (loaded lazily)
  private claudeAgent!: typeof import('agentic-flow/dist/agents/claudeAgent.js')['claudeAgent'];
  private getAgent!: typeof import('agentic-flow/dist/utils/agentLoader.js')['getAgent'];

  // Routing & coordination (may be null if init fails gracefully)
  private semanticRouter: InstanceType<typeof import('agentic-flow/dist/routing/SemanticRouter.js')['SemanticRouter']> | null = null;
  private coordinator: InstanceType<typeof import('agentic-flow/dist/coordination/attention-coordinator.js')['AttentionCoordinator']> | null = null;
  private embedder: InstanceType<typeof import('agentic-flow/dist/core/embedding-service.js')['EmbeddingService']> | null = null;

  // Learning & memory (best-effort)
  private reasoningBank: ReasoningBank | null = null;
  private financialMemory: FinancialMemory | null = null;

  constructor(config?: Partial<PipelineConfig>) {
    this.config = { ...DEFAULT_CONFIG, ...config };
  }

  // ── Lazy initialization ───────────────────────────────────────────

  private async init(): Promise<void> {
    if (this.initialized) return;

    const _require = createRequire(import.meta.url);
    const _afDir = dirname(_require.resolve('agentic-flow/package.json'));
    const afImport = (subpath: string) =>
      import(pathToFileURL(join(_afDir, 'dist', ...subpath.split('/'))).href);

    // Core agent modules (required)
    const [agentMod, loaderMod] = await Promise.all([
      afImport('agents/claudeAgent.js') as Promise<typeof import('agentic-flow/dist/agents/claudeAgent.js')>,
      afImport('utils/agentLoader.js') as Promise<typeof import('agentic-flow/dist/utils/agentLoader.js')>,
    ]);
    this.claudeAgent = agentMod.claudeAgent;
    this.getAgent = loaderMod.getAgent;

    // Embedding service (Transformers.js local model — no API calls)
    try {
      const embMod = await afImport('core/embedding-service.js') as typeof import('agentic-flow/dist/core/embedding-service.js');
      const transformers = new embMod.TransformersEmbeddingService({
        model: 'Xenova/all-MiniLM-L6-v2',
        dimensions: 384,
      });
      await transformers.initialize();
      this.embedder = transformers;
      this.status('init', 'Embedding: TransformersEmbeddingService (384-dim)');
    } catch (err) {
      this.status('init', `Embedding: failed to initialize — ${err instanceof Error ? err.message : String(err)}`);
    }

    // Semantic router (best-effort)
    if (this.embedder) {
      try {
        const routerMod = await afImport('routing/SemanticRouter.js') as typeof import('agentic-flow/dist/routing/SemanticRouter.js');
        this.semanticRouter = new routerMod.SemanticRouter(createRouterEmbedder(this.embedder) as any);
        await this.semanticRouter.registerAgents(CFA_INTENTS);
        this.semanticRouter.buildIndex();
        this.status('init', `SemanticRouter: ${CFA_INTENTS.length} intents indexed`);
      } catch {
        this.semanticRouter = null;
        this.status('init', 'SemanticRouter: unavailable — will use chief analyst fallback');
      }
    }

    // Attention coordinator (best-effort)
    if (this.embedder) {
      try {
        const attMod = await afImport('core/attention-fallbacks.js') as typeof import('agentic-flow/dist/core/attention-fallbacks.js');
        const coordMod = await afImport('coordination/attention-coordinator.js') as typeof import('agentic-flow/dist/coordination/attention-coordinator.js');
        const attention = attMod.createAttention(this.config.attentionMechanism, {
          hiddenDim: 384,
          numHeads: 8,
        });
        this.coordinator = coordMod.createAttentionCoordinator(attention);
        this.status('init', `AttentionCoordinator: ${this.config.attentionMechanism} (384-dim, 8 heads)`);
      } catch {
        this.coordinator = null;
        this.status('init', 'AttentionCoordinator: unavailable — will use equal weights');
      }
    }

    // Learning & memory modules
    // CFA_MEMORY_BACKEND: postgres | sqlite (default) | local
    const memoryBackend = process.env.CFA_MEMORY_BACKEND ?? 'sqlite';

    if (memoryBackend === 'postgres') {
      // Postgres-backed (ruvector-postgres with pgvector search)
      try {
        const { PgReasoningBank } = await import('../learning/pg-reasoning-bank.js');
        const { PgFinancialMemory } = await import('../memory/pg-financial-memory.js');
        const { healthCheck, runMigrations } = await import('../db/pg-client.js');

        const healthy = await healthCheck();
        if (!healthy) throw new Error('Postgres health check failed');

        const migrations = await runMigrations();
        if (migrations.length > 0) {
          this.status('init', `Postgres: ran ${migrations.length} migration(s)`);
        }

        this.reasoningBank = new PgReasoningBank();
        this.financialMemory = new PgFinancialMemory();
        this.status('init', `ReasoningBank: PgReasoningBank (${process.env.PG_HOST ?? 'localhost'}:${process.env.PG_PORT ?? '5433'})`);
        this.status('init', `FinancialMemory: PgFinancialMemory`);
      } catch (err) {
        this.status('init', `Postgres init failed: ${err instanceof Error ? err.message : String(err)} — falling back to sqlite`);
        // Fall through to sqlite/agentdb below
        await this.initSqliteMemory();
      }
    } else if (memoryBackend === 'local') {
      // Pure in-memory (no persistence)
      try {
        const { LocalReasoningBank } = await import('../learning/reasoning-bank.js');
        const { LocalFinancialMemory } = await import('../memory/financial-memory.js');
        this.reasoningBank = new LocalReasoningBank();
        this.financialMemory = new LocalFinancialMemory();
        this.status('init', 'ReasoningBank: LocalReasoningBank (in-memory)');
        this.status('init', 'FinancialMemory: LocalFinancialMemory (in-memory)');
      } catch {
        this.status('init', 'Memory: local backends unavailable');
      }
    } else {
      // Default: sqlite via agentic-flow's ReasoningBank
      await this.initSqliteMemory();
    }

    this.initialized = true;
  }

  private async initSqliteMemory(): Promise<void> {
    try {
      const { SonaReasoningBank } = await import('../learning/reasoning-bank.js');
      this.reasoningBank = new SonaReasoningBank();
      this.status('init', 'ReasoningBank: SonaReasoningBank (sqlite)');
    } catch {
      try {
        const { LocalReasoningBank } = await import('../learning/reasoning-bank.js');
        this.reasoningBank = new LocalReasoningBank();
        this.status('init', 'ReasoningBank: LocalReasoningBank fallback');
      } catch {
        this.status('init', 'ReasoningBank: unavailable');
      }
    }

    try {
      const { AgentDbFinancialMemory } = await import('../memory/financial-memory.js');
      this.financialMemory = new AgentDbFinancialMemory();
      this.status('init', 'FinancialMemory: AgentDbFinancialMemory (sqlite)');
    } catch {
      try {
        const { LocalFinancialMemory } = await import('../memory/financial-memory.js');
        this.financialMemory = new LocalFinancialMemory();
        this.status('init', 'FinancialMemory: LocalFinancialMemory fallback');
      } catch {
        this.status('init', 'FinancialMemory: unavailable');
      }
    }
  }

  // ── Execute: 6-stage pipeline ─────────────────────────────────────

  async execute(
    query: string,
    onStream?: (chunk: string) => void,
  ): Promise<PipelineResult> {
    await this.init();

    const requestId = randomUUID();
    const totalStart = Date.now();
    const timings = { routingMs: 0, memoryMs: 0, agentsMs: 0, coordinationMs: 0, synthesisMs: 0, totalMs: 0 };

    // ── Stage 1: Task Router ──────────────────────────────────────
    this.status('routing', 'Analyzing query intent...');
    const routingStart = Date.now();

    let routedAgentTypes: string[];
    let multiIntent: MultiIntentResult | null = null;

    if (this.semanticRouter) {
      try {
        multiIntent = await this.semanticRouter.detectMultiIntent(query, this.config.confidenceThreshold);

        if (multiIntent.requiresMultiAgent && multiIntent.intents.length > 1) {
          routedAgentTypes = multiIntent.intents
            .slice(0, this.config.maxAgents)
            .map(i => i.agentType);
          this.status('routing', `Multi-intent: ${routedAgentTypes.length} agents [${routedAgentTypes.join(', ')}]`);
        } else if (multiIntent.intents.length > 0) {
          // Single intent — skip to synthesis with one agent
          routedAgentTypes = [multiIntent.intents[0].agentType];
          this.status('routing', `Single intent: ${routedAgentTypes[0]} (confidence: ${multiIntent.intents[0].confidence.toFixed(2)})`);
        } else {
          // detectMultiIntent found nothing above threshold — fall back to route()
          const singleRoute = await this.semanticRouter!.route(query);
          routedAgentTypes = [singleRoute.primaryAgent];
          this.status('routing', `Routed: ${singleRoute.primaryAgent} (confidence: ${singleRoute.confidence.toFixed(2)})`);
        }
      } catch {
        routedAgentTypes = [DEFAULT_AGENT];
        this.status('routing', 'Router error — defaulting to chief analyst');
      }
    } else {
      routedAgentTypes = [DEFAULT_AGENT];
      this.status('routing', 'No router — using chief analyst');
    }

    timings.routingMs = Date.now() - routingStart;

    // ── Stage 2: Vector Search (prior patterns) ───────────────────
    this.status('memory', 'Searching prior patterns and analyses...');
    const memoryStart = Date.now();

    let priorPatterns = '';
    let priorAnalyses = '';

    const primaryTaskType = inferTaskType(routedAgentTypes[0]);

    const [patternsResult, memoryResult] = await Promise.all([
      this.reasoningBank
        ? this.reasoningBank.searchPatterns(primaryTaskType, 5).catch(() => [])
        : Promise.resolve([]),
      this.financialMemory
        ? this.financialMemory.search(query, 3).catch(() => ({ entries: [] }))
        : Promise.resolve({ entries: [] }),
    ]);

    if (patternsResult.length > 0) {
      priorPatterns = patternsResult
        .map(p => `- ${p.taskType}: ${p.toolSequence.join(' → ')} (reward: ${p.rewardScore.toFixed(2)})`)
        .join('\n');
      this.status('memory', `Found ${patternsResult.length} prior patterns`);
    }

    if (memoryResult.entries.length > 0) {
      priorAnalyses = memoryResult.entries
        .map(e => `- [score ${e.similarityScore.toFixed(2)}] ${e.entry.content.slice(0, 200)}...`)
        .join('\n');
      this.status('memory', `Found ${memoryResult.entries.length} prior analyses`);
    }

    timings.memoryMs = Date.now() - memoryStart;

    // ── Stage 2b: Pre-flight ticker resolution ─────────────────────
    const resolvedTicker = await resolveTickerFromQuery(query);
    if (resolvedTicker) {
      this.status('routing', `Ticker resolved: ${resolvedTicker}`);
    }

    // ── Stage 3: Spawn Agents ─────────────────────────────────────
    this.status('agents', `Spawning ${routedAgentTypes.length} agent(s)...`);
    const agentsStart = Date.now();

    // Build augmented prompt with context
    const contextBlock = [
      priorPatterns ? `## Prior Successful Tool Sequences\n${priorPatterns}` : '',
      priorAnalyses ? `## Prior Related Analyses\n${priorAnalyses}` : '',
    ].filter(Boolean).join('\n\n');

    const augmentedQuery = contextBlock
      ? `${query}\n\n---\n\n# Context from Prior Analyses\n\n${contextBlock}`
      : query;

    type AgentRunResult = { agentName: string; agentType: string; output: string };

    const agentPromises = routedAgentTypes.map(async (agentType): Promise<AgentRunResult | null> => {
      const agentName = agentNameFromType(agentType);

      try {
        const agentDef = this.getAgent(agentName, cfaAgentsDir) ?? this.getAgent(agentName);
        if (!agentDef) {
          this.status('agents', `Agent ${agentName} not found — skipping`);
          return null;
        }

        const agent = injectSkills(agentDef);

        // Prepend runtime instructions to the query
        const agentPrompt = buildAgentPreamble(agentType, query, resolvedTicker) + augmentedQuery;

        // Run with AbortController — on timeout the SDK subprocess is killed
        // immediately, preventing orphaned agents from burning tokens.
        const result = await runAgentWithAbort(agent, agentPrompt, {
          timeoutMs: AGENT_TIMEOUT_MS,
          maxTurns: AGENT_MAX_TURNS,
          onToolCall: (name, count) => {
            const ts = new Date().toISOString().split('T')[1].split('.')[0];
            process.stderr.write(`\n[${ts}] 🔍 Tool call #${count}: ${name}\n`);
          },
        });

        this.status('agents', `${agentName} complete (${result.output.length} chars)`);
        return { agentName, agentType, output: result.output };
      } catch (err) {
        this.status('agents', `${agentName} failed: ${err instanceof Error ? err.message : String(err)}`);
        return null;
      }
    });

    const rawResults = await Promise.all(agentPromises);
    const agentResults = rawResults.filter((r): r is AgentRunResult => r !== null);

    if (agentResults.length === 0) {
      throw new PipelineError('All agents failed — cannot produce analysis', 'agents');
    }

    this.status('agents', `${agentResults.length}/${routedAgentTypes.length} agents returned results`);
    timings.agentsMs = Date.now() - agentsStart;

    // ── Stage 4: Coordination Layer ───────────────────────────────
    this.status('coordination', 'Running attention-based coordination...');
    const coordStart = Date.now();

    let coordResult: CoordinationResult | null = null;
    let attentionWeights: number[] = agentResults.map(() => 1 / agentResults.length);

    if (this.coordinator && this.embedder && agentResults.length > 1) {
      try {
        // Embed each agent's output
        const embedResults = await Promise.all(
          agentResults.map(r => this.embedder!.embed(r.output.slice(0, 2000))),
        );

        const agentOutputs: AgentOutput[] = agentResults.map((r, i) => ({
          agentId: r.agentName,
          agentType: r.agentType,
          embedding: new Float32Array(embedResults[i].embedding),
          value: new Float32Array(embedResults[i].embedding),
          confidence: multiIntent?.intents.find(intent => intent.agentType === r.agentType)?.confidence ?? 0.7,
        }));

        coordResult = await this.coordinator.topologyAwareCoordination(
          agentOutputs,
          this.config.topology,
        );

        attentionWeights = coordResult.attentionWeights.slice(0, agentResults.length);

        // Normalize weights
        const weightSum = attentionWeights.reduce((s, w) => s + w, 0);
        if (weightSum > 0) {
          attentionWeights = attentionWeights.map(w => w / weightSum);
        }

        this.status('coordination', `Topology: ${this.config.topology} | Top agents: ${coordResult.topAgents.join(', ')}`);
      } catch (err) {
        this.status('coordination', `Coordination failed: ${err instanceof Error ? err.message : String(err)} — using equal weights`);
        attentionWeights = agentResults.map(() => 1 / agentResults.length);
      }
    } else if (agentResults.length === 1) {
      attentionWeights = [1.0];
      this.status('coordination', 'Single agent — skipping coordination');
    }

    // Sort by attention weight (highest first)
    const indexed = agentResults.map((r, i) => ({ ...r, attentionWeight: attentionWeights[i] }));
    indexed.sort((a, b) => b.attentionWeight - a.attentionWeight);

    timings.coordinationMs = Date.now() - coordStart;

    // ── Stage 5: Result Synthesis ─────────────────────────────────
    this.status('synthesis', 'Synthesizing final analysis...');
    const synthStart = Date.now();

    let synthesis: string;

    if (indexed.length === 1) {
      // Single agent — stream directly
      synthesis = indexed[0].output;
      if (onStream) onStream(synthesis);
    } else {
      // Multi-agent — use chief analyst to synthesize
      try {
        const chiefDef = this.getAgent(DEFAULT_AGENT, cfaAgentsDir) ?? this.getAgent(DEFAULT_AGENT);
        if (!chiefDef) throw new Error('Chief analyst agent not found');

        // Inject all 4 skill domains
        const chiefAgent = injectSkills({
          ...chiefDef,
          name: DEFAULT_AGENT,
        });

        const synthesisPrompt = this.buildSynthesisPrompt(query, indexed, coordResult);

        const result = await runAgentWithAbort(chiefAgent, synthesisPrompt, {
          timeoutMs: AGENT_TIMEOUT_MS,
          maxTurns: AGENT_MAX_TURNS,
        });
        synthesis = result.output;
        if (onStream) onStream(synthesis);
      } catch (err) {
        // Fallback: concatenate raw outputs
        this.status('synthesis', `Synthesis agent failed: ${err instanceof Error ? err.message : String(err)} — concatenating raw outputs`);
        synthesis = indexed
          .map(r => `## ${r.agentName} (weight: ${r.attentionWeight.toFixed(2)})\n\n${r.output}`)
          .join('\n\n---\n\n');
        if (onStream) onStream(synthesis);
      }
    }

    timings.synthesisMs = Date.now() - synthStart;
    timings.totalMs = Date.now() - totalStart;

    // ── Stage 6: Learning ─────────────────────────────────────────
    if (this.config.enableLearning) {
      this.recordLearning(requestId, query, indexed, synthesis).catch(() => {
        // Silently ignore learning failures
      });
    }

    const pipelineResult: PipelineResult = {
      requestId,
      synthesis,
      agentResults: indexed,
      routedAgents: routedAgentTypes,
      topology: this.config.topology,
      coordination: coordResult ? {
        mechanism: coordResult.mechanism,
        executionTimeMs: coordResult.executionTimeMs,
        topAgents: coordResult.topAgents,
        attentionWeights,
      } : undefined,
      timings,
    };

    return pipelineResult;
  }

  // ── Synthesis prompt builder ───────────────────────────────────

  private buildSynthesisPrompt(
    query: string,
    rankedResults: AgentResult[],
    coordResult: CoordinationResult | null,
  ): string {
    const agentSections = rankedResults
      .map((r, i) => {
        const rank = i + 1;
        return `### Agent ${rank}: ${r.agentName} (attention weight: ${r.attentionWeight.toFixed(3)})\n\n${r.output}`;
      })
      .join('\n\n---\n\n');

    const coordMeta = coordResult
      ? `\n\n## Coordination Metadata\n- Mechanism: ${coordResult.mechanism}\n- Topology: ${this.config.topology}\n- Top agents: ${coordResult.topAgents.join(', ')}\n- Execution: ${coordResult.executionTimeMs}ms`
      : '';

    return `You are synthesizing results from ${rankedResults.length} specialist financial analysts into a single, coherent institutional-grade response.

## Original User Query

${query}

## Agent Results (ranked by attention weight)

${agentSections}
${coordMeta}

## Synthesis Instructions

1. Integrate findings from all agents, weighting more heavily those with higher attention scores.
2. Resolve any contradictions by noting the disagreement and the reasoning on each side.
3. Present a unified analysis with clear sections, numerical precision, and base/bull/bear scenarios where applicable.
4. Every number must trace to a specific agent's tool output — do not generate new calculations.
5. Include a brief "Methodology" section listing which agents contributed and their key tools.
6. End with "Key Risks" and "Confidence Assessment".`;
  }

  // ── Learning recorder ──────────────────────────────────────────

  private async recordLearning(
    requestId: string,
    query: string,
    results: AgentResult[],
    synthesis: string,
  ): Promise<void> {
    // Record reasoning trace
    if (this.reasoningBank) {
      try {
        await this.reasoningBank.recordTrace({
          traceId: randomUUID(),
          agentType: 'cfa-pipeline',
          requestId,
          steps: [
            { phase: 'observe', content: query, timestamp: new Date() },
            ...results.map(r => ({
              phase: 'act' as const,
              content: `${r.agentName}: ${r.output.slice(0, 200)}`,
              toolCalls: [r.agentType],
              timestamp: new Date(),
            })),
            { phase: 'reflect', content: synthesis.slice(0, 500), timestamp: new Date() },
          ],
          outcome: 'success',
          createdAt: new Date(),
        });
      } catch { /* best-effort */ }
    }

    // Store synthesis in financial memory
    if (this.financialMemory) {
      try {
        await this.financialMemory.store(synthesis.slice(0, 5000), {
          sourceType: 'analysis',
          analysisType: 'pipeline-synthesis',
          tags: results.map(r => r.agentType),
        });
      } catch { /* best-effort */ }
    }
  }

  // ── Status emitter ─────────────────────────────────────────────

  private status(stage: string, message: string): void {
    this.config.onStatus?.(stage, message);
  }
}
