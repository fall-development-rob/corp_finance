/**
 * Canonical wrapResponse for MCP tool handlers.
 *
 * Serialises the resolved result of an MCP tool call into the MCP content
 * envelope.  All fmp-mcp-server, data-mcp-server, and vendor-mcp-server tool
 * files use this shared implementation instead of maintaining per-file copies.
 *
 * Behaviour: JSON.stringify with 2-space indent so responses are human-readable
 * in the Claude tool-result panel.  Errors should be caught by the caller and
 * converted to a structured object before passing here; this function does not
 * special-case Error instances (unlike the mcp-server/formatters/response.ts
 * variant which uses String() and sets isError).
 */
export function wrapResponse(data: unknown): {
  content: { type: "text"; text: string }[];
} {
  return {
    content: [{ type: "text" as const, text: JSON.stringify(data, null, 2) }],
  };
}
