/**
 * MCP client barrel — public surface for the stdio transport and name resolver.
 */
export { createStdioMCPClient } from "./stdio.js";
export {
  buildNameMap,
  resolveBareToWire,
  extractBareFromWire,
  type ResolvedTool,
} from "./name-resolver.js";
