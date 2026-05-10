/**
 * MCP client barrel — public surface for stdio (local), SSE (remote), and
 * StreamableHTTP (remote) transports plus the bare↔wire name resolver.
 */
export { createStdioMCPClient } from "./stdio.js";
export {
  createSSEMCPClient,
  type SSEMCPServerConfig,
} from "./sse.js";
export {
  createHTTPMCPClient,
  type HTTPMCPServerConfig,
} from "./http.js";
export {
  buildNameMap,
  resolveBareToWire,
  extractBareFromWire,
  type ResolvedTool,
} from "./name-resolver.js";
