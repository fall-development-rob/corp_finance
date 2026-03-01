// Pipeline types, intent definitions, and skill injection
// Extracted from pipeline.ts for ARCH-003 compliance (500-line limit)

import { join } from 'node:path';
import { existsSync, readFileSync } from 'node:fs';

import type { AgentDefinition } from 'agentic-flow/dist/utils/agentLoader.js';
import type { AgentIntent } from 'agentic-flow/dist/routing/SemanticRouter.js';
import type { SwarmTopology } from 'agentic-flow/dist/coordination/attention-coordinator.js';
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

// ── Default config ──────────────────────────────────────────────────

export const DEFAULT_CONFIG: PipelineConfig = {
  topology: 'hierarchical',
  confidenceThreshold: 0.3,
  maxAgents: 6,
  attentionMechanism: 'flash',
  enableLearning: true,
};

export const AGENT_TIMEOUT_MS = 180_000;
export const AGENT_MAX_TURNS = 12;
export const DEFAULT_AGENT = 'cfa-chief-analyst';

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
    'workflow-financial-analysis',
    'workflow-deal-documents',
  ],
  'cfa-equity-analyst': [
    'corp-finance-tools-core',
    'fmp-market-data',
    'fmp-technicals',
    'fmp-news-intelligence',
    'workflow-equity-research',
  ],
  'cfa-credit-analyst':          ['corp-finance-tools-core', 'fmp-market-data', 'fmp-sec-compliance'],
  'cfa-private-markets-analyst': [
    'corp-finance-tools-core',
    'fmp-market-data',
    'fmp-sec-compliance',
    'workflow-investment-banking',
    'workflow-private-equity',
  ],
  'cfa-fixed-income-analyst':    ['corp-finance-tools-markets', 'fmp-market-data'],
  'cfa-derivatives-analyst':     ['corp-finance-tools-markets', 'fmp-market-data', 'fmp-technicals'],
  'cfa-macro-analyst':           ['corp-finance-tools-markets', 'fmp-research', 'fmp-news-intelligence'],
  'cfa-quant-risk-analyst': [
    'corp-finance-tools-risk',
    'fmp-market-data',
    'fmp-technicals',
    'fmp-etf-funds',
    'workflow-wealth-management',
  ],
  'cfa-esg-regulatory-analyst':  ['corp-finance-tools-regulatory', 'fmp-research', 'fmp-sec-compliance'],
};

const skillCache = new Map<string, string>();

function readSkillBody(skillName: string, skillsDir: string): string {
  const cacheKey = `${skillsDir}/${skillName}`;
  if (skillCache.has(cacheKey)) return skillCache.get(cacheKey)!;

  const skillPath = join(skillsDir, skillName, 'SKILL.md');
  if (!existsSync(skillPath)) return '';

  const raw = readFileSync(skillPath, 'utf-8');
  const body = raw.replace(/^---\n[\s\S]*?\n---\n/, '').trim();
  skillCache.set(cacheKey, body);
  return body;
}

export function injectSkills(agent: AgentDefinition, skillsDir: string): AgentDefinition {
  const skills = AGENT_SKILLS[agent.name];
  if (!skills || skills.length === 0) return agent;

  const skillContent = skills
    .map(s => readSkillBody(s, skillsDir))
    .filter(Boolean)
    .join('\n\n---\n\n');

  if (!skillContent) return agent;

  return {
    ...agent,
    systemPrompt: agent.systemPrompt + '\n\n---\n\n# MCP Tool Reference\n\n' + skillContent,
  };
}

// ── CFA agent intent definitions for HNSW routing ──────────────────

export const CFA_INTENTS: AgentIntent[] = [
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
  {
    agentType: 'cfa-equity-analyst',
    description: 'Initiate coverage reports, earnings analysis, morning notes, thesis tracking, equity screening, sector overviews',
    examples: [
      'Initiate coverage on Apple with a buy rating',
      'Write a morning note on tech sector earnings',
    ],
    tags: ['coverage', 'earnings', 'morning-note', 'thesis', 'screening', 'sector', 'research-workflow'],
  },
  {
    agentType: 'cfa-private-markets-analyst',
    description: 'CIM drafting, deal teasers, buyer lists, pitch decks, deal screening, IC memos, DD checklists, value creation plans',
    examples: [
      'Draft a CIM for a $200M SaaS company',
      'Prepare a buyer list for this healthcare target',
      'Write an IC memo for this acquisition opportunity',
    ],
    tags: ['cim', 'teaser', 'buyer-list', 'pitch-deck', 'ic-memo', 'dd-checklist', 'deal-documents', 'value-creation'],
  },
  {
    agentType: 'cfa-quant-risk-analyst',
    description: 'Client portfolio reviews, financial planning, rebalancing, tax-loss harvesting, wealth management proposals',
    examples: [
      'Prepare a quarterly client review for a $5M portfolio',
      'Create a retirement financial plan for a 45-year-old',
    ],
    tags: ['client-review', 'financial-plan', 'rebalance', 'tax-loss-harvesting', 'wealth', 'retirement'],
  },
  {
    agentType: 'cfa-chief-analyst',
    description: 'Model audit and quality checks, deck reviews, competitive analysis, document quality standards',
    examples: [
      'Audit this financial model for errors and inconsistencies',
      'Review this pitch deck for institutional quality',
    ],
    tags: ['model-audit', 'deck-review', 'competitive-analysis', 'quality', 'document-standards'],
  },
];

// ── Agent name normalization ────────────────────────────────────────

export function agentNameFromType(agentType: string): string {
  return agentType;
}

export function inferTaskType(agentType: string): TaskType {
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

export function createRouterEmbedder(realEmbedder: any): any {
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
