/**
 * Barrel export for the cross-agent handoff event loop — REC-4 Wave 1-2, Phase 38 W4.
 */
export * from "./types.js";
export { createHandoffOrchestrator } from "./orchestrator.js";

// REC-4 Wave 2 — virtual tool + AgentDef helpers
export {
  HANDOFF_TOOL_NAME,
  isHandoffToolName,
  createHandoffTool,
  formatHandoffResult,
  executeHandoffCall,
} from "./handoff-tool.js";
export {
  buildAllowlistFromAgent,
  buildResolverFromAgent,
  buildAllowlistResolver,
} from "./from-agent-def.js";
