// CFA Multi-Agent Analyst System
// Orchestrates specialist financial analysts powered by corp-finance-mcp tools

export { Orchestrator } from './orchestrator/index.js';
export type { OrchestratorConfig } from './orchestrator/index.js';
export { createSpecialist } from './orchestrator/index.js';

export { ChiefAnalyst } from './agents/chief-analyst.js';
export { BaseAnalyst } from './agents/base-analyst.js';
export { EquityAnalyst } from './agents/equity-analyst.js';
export { CreditAnalyst } from './agents/credit-analyst.js';
export { FixedIncomeAnalyst } from './agents/fixed-income-analyst.js';
export { DerivativesAnalyst } from './agents/derivatives-analyst.js';
export { QuantRiskAnalyst } from './agents/quant-risk-analyst.js';
export { MacroAnalyst } from './agents/macro-analyst.js';
export { EsgRegulatoryAnalyst } from './agents/esg-regulatory-analyst.js';
export { PrivateMarketsAnalyst } from './agents/private-markets-analyst.js';

export { AgentDbFinancialMemory, LocalFinancialMemory, PgFinancialMemory } from './memory/index.js';
export type { FinancialMemory } from './memory/index.js';
export { LocalReasoningBank, SonaReasoningBank, PgReasoningBank } from './learning/index.js';
export type { ReasoningBank } from './learning/index.js';

// Database factory — auto-selects backend from CFA_MEMORY_BACKEND env var
export { createFinancialMemory, createReasoningBank } from './config/database.js';
export type { MemoryBackend } from './config/database.js';

export * from './types/index.js';

// Bridge — connects agents to corp-finance-mcp tools via MCP protocol or CLI
export { McpBridge, createToolCaller } from './bridge/index.js';
export type { McpBridgeConfig } from './bridge/index.js';
export { CliBridge, createCliToolCaller } from './bridge/index.js';
export type { CliBridgeConfig } from './bridge/index.js';
