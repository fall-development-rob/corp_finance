/**
 * API-key scoping for the CFA Harness — Phase 31 Wave 4.
 *
 * Only injects API keys into MCP servers whose tools the agent actually uses,
 * limiting blast radius if a compromised agent prompt attempts credential leakage.
 */

import type { KeyScope, ScopedEnv } from "../types.js";

// ---------------------------------------------------------------------------
// Default scopes for standard CFA env vars
// ---------------------------------------------------------------------------

/**
 * Default scopes for the standard cfa env vars. Pattern semantics:
 *   - literal "*" matches every tool
 *   - prefix "<word>_*" matches tools whose bare name starts with "<word>_"
 *   - exact "<word>" matches that one tool
 */
export const defaultKeyScopes: KeyScope[] = [
  { env_var: "ANTHROPIC_API_KEY",   tools: ["*"] },
  { env_var: "FMP_API_KEY",         tools: ["fmp_*"] },
  { env_var: "FRED_API_KEY",        tools: ["fred_*"] },
  { env_var: "ACLED_API_KEY",       tools: ["acled_*"] },
  { env_var: "ACLED_EMAIL",         tools: ["acled_*"] },
  { env_var: "NASA_API_KEY",        tools: ["firms_*", "eonet_*"] },
  { env_var: "EIA_API_KEY",         tools: ["eia_*"] },
  { env_var: "LSEG_API_KEY",        tools: ["lseg_*"] },
  { env_var: "FACTSET_API_KEY",     tools: ["factset_*"] },
  { env_var: "MOODYS_API_KEY",      tools: ["moodys_*"] },
  { env_var: "MORNINGSTAR_API_KEY", tools: ["ms_*"] },
  { env_var: "SP_API_KEY",          tools: ["sp_*"] },
  { env_var: "PITCHBOOK_API_KEY",   tools: ["pb_*"] },
];

// ---------------------------------------------------------------------------
// Pattern matching
// ---------------------------------------------------------------------------

/**
 * Tool-name pattern matcher. Pure function for testability.
 * - "*" matches any name
 * - "prefix_*" matches names starting with "prefix_"
 * - exact match otherwise
 */
export function matchesPattern(toolName: string, pattern: string): boolean {
  if (pattern === "*") return true;
  if (pattern.endsWith("_*")) {
    const prefix = pattern.slice(0, -1); // strip the trailing "*", keep the "_"
    return toolName.startsWith(prefix);
  }
  return toolName === pattern;
}

// ---------------------------------------------------------------------------
// Scope resolution
// ---------------------------------------------------------------------------

/**
 * Returns true when any tool in `toolNames` matches any pattern in `patterns`.
 */
function anyToolMatchesAnyPattern(
  toolNames: string[],
  patterns: string[],
): boolean {
  return toolNames.some((tool) =>
    patterns.some((pattern) => matchesPattern(tool, pattern)),
  );
}

/**
 * Resolve the scoped env for an agent given its tool allowlist and the
 * full set of available tools (from MCPClient.listTools().map(t => t.name)).
 *
 * When agentTools is "*", every scope is granted (the agent can use any tool).
 */
export function scopeEnvForAgent(opts: {
  agentTools: string[] | "*";
  availableTools: string[];
  scopes?: KeyScope[];
  env?: NodeJS.ProcessEnv;
}): Record<string, string> {
  const { agentTools, availableTools, scopes = defaultKeyScopes, env = process.env } = opts;

  // Wildcard agents get every defined scope — they can invoke any tool, so all
  // keys that exist in the env are relevant.
  const isWildcard = agentTools === "*";
  const effectiveTools: string[] = isWildcard ? availableTools : agentTools;

  const result: Record<string, string> = {};

  for (const scope of scopes) {
    const matches =
      isWildcard || anyToolMatchesAnyPattern(effectiveTools, scope.tools);
    if (!matches) continue;
    const value = env[scope.env_var];
    if (value !== undefined) {
      result[scope.env_var] = value;
    }
  }

  return result;
}

// ---------------------------------------------------------------------------
// ScopedEnv factory
// ---------------------------------------------------------------------------

/**
 * Creates a ScopedEnv whose scopeForAgent() calls scopeEnvForAgent with the
 * stored scopes and env.
 */
export function createScopedEnv(opts: {
  scopes?: KeyScope[];
  env?: NodeJS.ProcessEnv;
}): ScopedEnv {
  const { scopes = defaultKeyScopes, env = process.env } = opts;

  return {
    scopeForAgent(agentTools: string[] | "*"): Record<string, string> {
      const availableTools =
        agentTools === "*" ? [] : (agentTools as string[]);
      return scopeEnvForAgent({ agentTools, availableTools, scopes, env });
    },
  };
}
