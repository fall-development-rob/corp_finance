/**
 * Name resolver — translates between bare tool names and wire-prefixed names.
 *
 * Bare name: "option_pricer"
 * Wire name: "mcp__plugin_cfa-core_cfa-core__option_pricer"
 *
 * Also exports shared MCP transport helpers:
 *   - toCanonicalTool  — wire-format → CanonicalTool
 *   - unwrapMcpContent — tools/call result → parsed/raw text
 */
import type { MCPServerConfig, CanonicalTool } from "../types.js";

export interface ResolvedTool {
  serverName: string;
  prefix: string;
  wireName: string;
}

/**
 * Builds a bare→ResolvedTool map from the server configs and their
 * reported wire tool names. Throws on duplicate bare names across servers.
 */
export function buildNameMap(
  servers: MCPServerConfig[],
  wireToolsByServer: Map<string, string[]>,
): Map<string, ResolvedTool> {
  const map = new Map<string, ResolvedTool>();

  for (const server of servers) {
    const wireNames = wireToolsByServer.get(server.name) ?? [];
    for (const wireName of wireNames) {
      const bareName = extractBareFromWire(wireName, server.prefix);
      if (map.has(bareName)) {
        throw new Error(
          `Tool name collision: bare name "${bareName}" is provided by both ` +
            `"${map.get(bareName)!.serverName}" and "${server.name}"`,
        );
      }
      map.set(bareName, { serverName: server.name, prefix: server.prefix, wireName });
    }
  }

  return map;
}

/**
 * Resolves a bare tool name to its server config and wire name.
 * Returns undefined if the bare name is not found.
 */
export function resolveBareToWire(
  nameMap: Map<string, ResolvedTool>,
  bareName: string,
): ResolvedTool | undefined {
  return nameMap.get(bareName);
}

/**
 * Extracts the bare name from a wire name by stripping the server prefix.
 * If the wire name does not start with the prefix, returns the wire name as-is.
 */
export function extractBareFromWire(wireName: string, prefix: string): string {
  if (wireName.startsWith(prefix)) {
    return wireName.slice(prefix.length);
  }
  return wireName;
}

// ---------------------------------------------------------------------------
// Shared MCP transport helpers
// ---------------------------------------------------------------------------

/**
 * Wire-format tool descriptor returned by every MCP server's tools/list.
 * Both the SDK Client (@modelcontextprotocol/sdk) and the stdio JSON-RPC
 * transport return tools in this shape.
 */
export interface WireTool {
  name: string;
  description?: string;
  inputSchema?: {
    type?: string;
    properties?: Record<string, unknown>;
    required?: string[];
  };
}

/**
 * Translate a single wire-format tool into the harness's CanonicalTool
 * shape, stripping the server's prefix from the bare name.
 *
 * Used by every MCP client transport (stdio, SSE, StreamableHTTP) so the
 * agent loop sees a uniform CanonicalTool[] regardless of how the catalog
 * was fetched.
 */
export function toCanonicalTool(tool: WireTool, prefix: string): CanonicalTool {
  const bareName = extractBareFromWire(tool.name, prefix);
  const schema: CanonicalTool["input_schema"] = {
    type: "object",
    properties: tool.inputSchema?.properties ?? {},
  };
  if (tool.inputSchema?.required !== undefined) {
    schema.required = tool.inputSchema.required;
  }
  return {
    name: bareName,
    description: tool.description ?? "",
    input_schema: schema,
  };
}

/**
 * Wire-format tool result returned by every MCP server's tools/call.
 * Contains an array of content parts. We extract the first text part
 * and try to JSON-parse its body; otherwise we return the raw string.
 *
 * Returns the parsed/raw text content, or undefined when no text part
 * exists (server returned only resource references, images, etc.).
 */
export function unwrapMcpContent(result: unknown): unknown {
  const r = result as { content?: Array<{ type: string; text?: string }> } | null | undefined;
  const content = r?.content;
  if (!Array.isArray(content)) return undefined;
  const textPart = content.find((c) => c.type === "text" && typeof c.text === "string");
  if (textPart?.text == null) return undefined;
  try {
    return JSON.parse(textPart.text) as unknown;
  } catch {
    return textPart.text;
  }
}
