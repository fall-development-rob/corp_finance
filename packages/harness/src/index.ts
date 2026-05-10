/**
 * Public entry point for @robotixai/cfa-harness.
 *
 * Phase 31 Wave 1 surface:
 *   - dispatch(opts): run an agent through the agent loop
 *   - createAnthropicProvider(opts): default provider
 *   - createStdioMCPClient(servers): connect to N MCP servers via stdio
 *   - registry: built-in agent definitions (chief-analyst for Wave 1)
 */

export * from "./types.js";
export { dispatch } from "./core/agent-loop.js";
export { createAnthropicProvider } from "./core/providers/anthropic.js";
export { createStdioMCPClient } from "./mcp-client/stdio.js";
export {
  registry,
  chiefAnalyst,
  defaultMCPServers,
} from "./agents/registry.js";
