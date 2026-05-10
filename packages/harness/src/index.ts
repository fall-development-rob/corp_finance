/**
 * Public entry point for @robotixai/cfa-harness.
 *
 * Phase 31 surface (Waves 1-3):
 *   - dispatch(opts): run an agent through the agent loop with optional
 *     specialist delegation (Wave 2)
 *   - Providers: Anthropic (Wave 1), OpenAI + Gemini (Wave 3)
 *   - MCP transports: stdio (Wave 1), SSE + StreamableHTTP (Wave 3)
 *   - registry: chief-analyst + 8 specialists (Wave 2)
 */

export * from "./types.js";
export { dispatch } from "./core/agent-loop.js";
export { createAnthropicProvider } from "./core/providers/anthropic.js";
export { createOpenAIProvider } from "./core/providers/openai.js";
export { createGeminiProvider } from "./core/providers/gemini.js";
export {
  createStdioMCPClient,
  createSSEMCPClient,
  type SSEMCPServerConfig,
  createHTTPMCPClient,
  type HTTPMCPServerConfig,
} from "./mcp-client/index.js";

// Wave 4 — audit, memory, security
export {
  createFileAuditSink,
  verifyAuditChain,
} from "./audit/index.js";
export {
  createFileSessionStore,
  buildReplayPrompt,
} from "./memory/index.js";
export {
  defaultKeyScopes,
  createScopedEnv,
  matchesPattern,
  scopeEnvForAgent,
} from "./security/index.js";
export {
  registry,
  chiefAnalyst,
  defaultMCPServers,
  defaultDelegates,
  equityAnalyst,
  creditAnalyst,
  fixedIncomeAnalyst,
  derivativesAnalyst,
  quantRiskAnalyst,
  macroAnalyst,
  privateMarketsAnalyst,
  esgRegulatoryAnalyst,
} from "./agents/registry.js";
export {
  delegationToolName,
  isDelegationToolName,
  resolveDelegationTarget,
  createDelegationTools,
} from "./agents/delegate.js";
export {
  createSkillRegistry,
  SKILL_REGISTRY_AGENT_IDS,
  type SkillRegistry,
  type SkillRegistryOptions,
} from "./agents/skill-registry.js";

// Phase 32 Wave 1 — ToolCatalogValidator ACL
export {
  filterToolsForAgent,
  validateAllowlists,
  assertAllowlistsValid,
  type ToolCatalogValidationIssue,
  type ToolCatalogValidationResult,
} from "./core/tool-schema.js";

// Phase 34 Wave 1 — Reasoning bank
export {
  type EmbeddingFn,
  createOpenAIEmbedder,
  createDeterministicEmbedder,
  type ReasoningEntry,
  type ReasoningBank,
  type ReasoningBankOptions,
  type RecallOptions,
  createRuVectorBank,
} from "./reasoning/index.js";

// Phase 34 Wave 2 — Indexer (audit -> reasoning bank bridge)
export {
  indexAuditRecord,
  aggregateToolCalls,
  extractDelegations,
  summarizePrompt,
  summarizeResult,
  type IndexAuditRecordOptions,
} from "./reasoning/index.js";

// Phase 34 Wave 3 — recall_similar virtual tool
export {
  RECALL_TOOL_NAME,
  isRecallToolName,
  createRecallTool,
  formatRecallResult,
  executeRecallCall,
} from "./reasoning/index.js";

// Phase 34 Wave 4 — recall_by_graph virtual tool + graph query type
export {
  RECALL_GRAPH_TOOL_NAME,
  isRecallGraphToolName,
  createRecallGraphTool,
  formatRecallGraphResult,
  executeRecallGraphCall,
  GRAPH_RECALL_DEFAULT_LIMIT,
  GRAPH_RECALL_MAX_LIMIT,
  type GraphRecallQuery,
} from "./reasoning/index.js";
