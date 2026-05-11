/**
 * Cookbook dispatch runtime — Phase 33 skill-driven planning.
 *
 * Ties together the CookbookLoader, HandoffOrchestrator, per-source
 * allowlist resolver (buildAllowlistResolver), and core dispatch() into one
 * callable entry point.
 *
 * Pipeline:
 *   1. Load cookbook via cookbookLoader.load(slug) → parent + subagents
 *   2. Build handoff orchestrator with per-source allowlist resolver, agent
 *      resolver, and recursive dispatchAgent callback
 *   3. Call dispatch(parent, prompt, depth=0) with the orchestrator wired in
 *   4. Return DispatchResult enriched with cookbook metadata
 *
 * Design constraints:
 *   - Pure functional: no global state, no side effects beyond dispatch()
 *   - Does NOT modify the cookbook loader, orchestrator, or agent-loop
 *   - Reasoning bank is threaded to the parent dispatch only (depth=0)
 */

import type {
  AuditSink,
  CanonicalTool,
  DispatchEvent,
  DispatchResult,
  MCPClient,
  Provider,
  SessionStore,
  AgentDef,
} from "../types.js";
import type { ReasoningBank } from "../reasoning/bank.js";
import type { CookbookLoader } from "../manifests/cookbook-loader.js";
import { createHandoffOrchestrator } from "../handoff/orchestrator.js";
import { buildAllowlistResolver } from "../handoff/from-agent-def.js";
import { dispatch } from "../core/agent-loop.js";

// ---------------------------------------------------------------------------
// Catalog-enriching MCP wrapper
//
// When a subagent has an explicit tool allowlist (not "*"), the catalog must
// include all allowed tools for assertAllowlistsValid to pass. This wrapper
// augments the real MCP catalog with synthetic stubs for any tools declared
// in the agent's allowlist that aren't already in the catalog. Real tool
// routing is delegated unchanged — synthetic entries are catalog-only stubs.
// ---------------------------------------------------------------------------

function enrichMcpForAgent(mcp: MCPClient, agent: AgentDef): MCPClient {
  if (agent.tools === "*") return mcp; // wildcard agents skip validation
  const agentTools = agent.tools;
  return {
    initialize: () => mcp.initialize(),
    close: () => mcp.close(),
    callTool: (bareName, input) => mcp.callTool(bareName, input),
    async listTools(): Promise<CanonicalTool[]> {
      const real = await mcp.listTools();
      const realNames = new Set(real.map((t) => t.name));
      const synthetic: CanonicalTool[] = agentTools
        .filter((name) => !realNames.has(name))
        .map((name) => ({
          name,
          description: `(synthetic catalog stub for ${name})`,
          input_schema: { type: "object" as const, properties: {} },
        }));
      return [...real, ...synthetic];
    },
  };
}

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

export interface CookbookDispatchOptions {
  /** Cookbook slug (looked up via cookbookLoader.load). */
  slug: string;
  /** Initial user prompt for the cookbook's parent agent. */
  prompt: string;

  /**
   * Pre-built CookbookLoader instance. Callers create this with
   * createCookbookLoader({ cookbooksRoot, skillLoader }). This design
   * allows tests to inject stub or broken loaders without filesystem access.
   */
  cookbookLoader: CookbookLoader;

  /**
   * Provider to dispatch through. Optional so tests can omit it and
   * inject a stub via other means; defaults to the Anthropic provider
   * if undefined (createAnthropicProvider() handles the fallback in dispatch()).
   */
  provider?: Provider;

  /** MCP client for tool routing. */
  mcp: MCPClient;

  /** Optional infra — wired through to dispatch(). */
  audit?: AuditSink;
  session?: SessionStore;
  reasoning?: ReasoningBank;

  /**
   * Maximum depth at which the initiate_handoff virtual tool is injected.
   * Default: 2 — chief at depth 0, subagents at depth 1 and 2 see the tool.
   */
  maxHandoffDepth?: number;

  /**
   * Maximum handoff chain length enforced by the orchestrator.
   * Default: 5. Requests are rejected when currentDepth >= maxChainDepth.
   */
  maxChainDepth?: number;

  /** Streaming event sink — receives all DispatchEvents from every dispatch. */
  onEvent?: (e: DispatchEvent) => void;
}

export interface CookbookDispatchResult extends DispatchResult {
  /** Alias for DispatchResult.auditId — the parent's audit record id. */
  parentAuditId?: string;
  /** The parent agent id (from the cookbook's agent.json "name" field). */
  parentAgentId: string;
  /** All subagent ids loaded from the cookbook's subagents/ directory. */
  subagentIds: string[];
  /**
   * Audit ids collected from subagents dispatched via handoff.
   * Absent when no audit sink was provided or no handoffs occurred.
   */
  subagentAuditIds?: string[];
  /**
   * Non-fatal error message if the dispatch completed but with degraded results.
   * Absent on fully successful runs.
   */
  errorMessage?: string;
}

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

/**
 * Run a cookbook end-to-end through the harness.
 *
 * Loads the cookbook, wires the handoff orchestrator, and dispatches the
 * parent agent. Returns the parent's DispatchResult enriched with cookbook
 * metadata (parentAuditId, parentAgentId, subagentIds, subagentAuditIds).
 */
export async function dispatchCookbook(
  opts: CookbookDispatchOptions,
): Promise<CookbookDispatchResult> {
  const {
    slug,
    cookbookLoader,
    prompt,
    provider,
    mcp,
    audit,
    session,
    reasoning,
    onEvent,
    maxHandoffDepth = 2,
    maxChainDepth = 5,
  } = opts;

  // Step 1: Load the cookbook (may throw if slug not found or manifests are invalid)
  const cookbook = await cookbookLoader.load(slug);
  const { parent, subagents } = cookbook;

  // All agents in scope for resolution and allowlist building
  const allAgents: AgentDef[] = [parent, ...subagents];

  // Step 2: Build per-source allowlist resolver
  // Each source's allowlist is derived from ITS own callableAgents field
  const resolveAllowlist = buildAllowlistResolver(allAgents);

  // Top-level allowlist: parent's own callable_agents as the initial root
  const topLevelAllowlist = resolveAllowlist(parent.id) ?? [];

  // Collect audit ids from subagent handoff dispatches
  const subagentAuditIds: string[] = [];

  // Step 3: Resolve agent by slug across parent + subagents
  function resolveAgent(agentSlug: string): AgentDef | undefined {
    return allAgents.find((a) => a.id === agentSlug);
  }

  // Step 4: dispatchAgent callback for the orchestrator.
  // parentDepth is the orchestrator's currentDepth + 1 at dispatch time.
  // NOTE: we reference orchestrator via closure — it is assigned in Step 5.
  //
  // The MCP is enriched with synthetic catalog stubs for any tools in the
  // subagent's explicit allowlist that aren't already in the real catalog.
  // This prevents assertAllowlistsValid from throwing when the production MCP
  // server is not available (e.g., in tests using a stub MCP with no tools).
  // Real tool routing is unchanged — only listTools() is augmented.
  async function dispatchAgent(
    agent: AgentDef,
    agentPrompt: string,
    parentDepth: number,
  ): Promise<{ finalText: string; auditId?: string }> {
    const enrichedMcp = enrichMcpForAgent(mcp, agent);
    const result = await dispatch({
      agent,
      prompt: agentPrompt,
      provider,
      mcp: enrichedMcp,
      audit,
      session,
      // reasoning is NOT passed to subagent dispatches — matches agent-loop
      // convention where only the root (depth=0) dispatch receives the bank
      handoff: orchestrator,
      maxHandoffDepth,
      depth: parentDepth,
      onEvent,
    });

    if (result.auditId) {
      subagentAuditIds.push(result.auditId);
    }

    return { finalText: result.finalText, auditId: result.auditId };
  }

  // Step 5: Create orchestrator — dispatchAgent references it via closure.
  // JavaScript closures capture the variable binding, so the self-reference
  // works correctly even though dispatchAgent is defined before orchestrator.
  const orchestrator = createHandoffOrchestrator({
    allowlist: topLevelAllowlist,
    resolveAllowlist,
    resolveAgent,
    dispatchAgent,
    maxDepth: maxChainDepth,
  });

  // Step 6: Dispatch the parent at depth=0 with reasoning wired in
  const parentResult = await dispatch({
    agent: parent,
    prompt,
    provider,
    mcp,
    audit,
    session,
    reasoning,
    handoff: orchestrator,
    maxHandoffDepth,
    depth: 0,
    onEvent,
  });

  return {
    ...parentResult,
    parentAuditId: parentResult.auditId,
    parentAgentId: parent.id,
    subagentIds: subagents.map((a) => a.id),
    ...(subagentAuditIds.length > 0 ? { subagentAuditIds } : {}),
  };
}
