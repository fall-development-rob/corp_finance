/**
 * StreamableHTTP MCP client — connects to remote MCP servers via HTTP.
 *
 * Uses the MCP SDK's StreamableHTTPClientTransport which supports both
 * request/response and SSE streaming via a single HTTP endpoint.
 *
 * Tool names are translated between bare names (agent-facing) and wire-prefixed
 * names (JSON-RPC) using the shared name-resolver utilities.
 */
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StreamableHTTPClientTransport } from "@modelcontextprotocol/sdk/client/streamableHttp.js";
import type { MCPClient, CanonicalTool } from "../types.js";
import {
  extractBareFromWire,
  type ResolvedTool,
} from "./name-resolver.js";

const TOOL_CALL_TIMEOUT_MS = 30_000;

/**
 * Configuration for one HTTP MCP server connection.
 * Defined here to keep types.ts transport-agnostic.
 */
export interface HTTPMCPServerConfig {
  name: string;
  prefix: string;
  /** Base URL of the StreamableHTTP endpoint (e.g., https://mcp.example.com/mcp). */
  url: string;
  /** Optional HTTP headers (e.g., Authorization) for requests. */
  headers?: Record<string, string>;
}

interface SDKToolDef {
  name: string;
  description?: string;
  inputSchema: {
    type: "object";
    properties?: Record<string, unknown>;
    required?: string[];
  };
}

interface ServerEntry {
  config: HTTPMCPServerConfig;
  client: Client;
}

function sdkToolToCanonical(tool: SDKToolDef, prefix: string): CanonicalTool {
  const bareName = extractBareFromWire(tool.name, prefix);
  return {
    name: bareName,
    description: tool.description ?? "",
    input_schema: {
      type: "object",
      properties: (tool.inputSchema.properties ?? {}) as Record<string, unknown>,
      required: tool.inputSchema.required,
    },
  };
}

/**
 * Builds a bare→ResolvedTool map from HTTP server configs and their reported
 * wire tool names. Throws on duplicate bare names across servers.
 */
function buildHTTPNameMap(
  entries: ServerEntry[],
  wireToolsByServer: Map<string, string[]>,
): Map<string, ResolvedTool> {
  const map = new Map<string, ResolvedTool>();

  for (const { config } of entries) {
    const wireNames = wireToolsByServer.get(config.name) ?? [];
    for (const wireName of wireNames) {
      const bareName = extractBareFromWire(wireName, config.prefix);
      if (map.has(bareName)) {
        const existing = map.get(bareName)!;
        throw new Error(
          `Tool name collision: bare name "${bareName}" is provided by both ` +
            `"${existing.serverName}" and "${config.name}"`,
        );
      }
      map.set(bareName, { serverName: config.name, prefix: config.prefix, wireName });
    }
  }

  return map;
}

/**
 * Creates and returns an MCPClient that connects to all specified MCP servers
 * via StreamableHTTP transport.
 */
export function createHTTPMCPClient(servers: HTTPMCPServerConfig[]): MCPClient {
  const entries: ServerEntry[] = servers.map((config) => ({
    config,
    client: new Client({ name: "cfa-harness", version: "0.1.0" }, { capabilities: {} }),
  }));

  let toolCatalog: CanonicalTool[] = [];
  let nameMap = new Map<string, ResolvedTool>();
  const clientByServer = new Map<string, Client>();

  return {
    async initialize(): Promise<void> {
      toolCatalog = [];
      const wireToolsByServer = new Map<string, string[]>();

      for (const entry of entries) {
        const { config, client } = entry;
        const transport = new StreamableHTTPClientTransport(new URL(config.url), {
          requestInit: config.headers
            ? { headers: config.headers as Record<string, string> }
            : undefined,
        });

        try {
          await client.connect(transport);
        } catch (err) {
          const msg = err instanceof Error ? err.message : String(err);
          throw new Error(`HTTP MCP connect failed for "${config.name}" (${config.url}): ${msg}`);
        }

        const response = await client.listTools();
        const wireNames = (response.tools as SDKToolDef[]).map((t) => t.name);
        wireToolsByServer.set(config.name, wireNames);
        clientByServer.set(config.name, client);

        for (const tool of response.tools as SDKToolDef[]) {
          toolCatalog.push(sdkToolToCanonical(tool, config.prefix));
        }
      }

      nameMap = buildHTTPNameMap(entries, wireToolsByServer);
    },

    async listTools(): Promise<CanonicalTool[]> {
      return toolCatalog;
    },

    async callTool(bareName: string, input: Record<string, unknown>): Promise<unknown> {
      const resolved = nameMap.get(bareName);
      if (!resolved) {
        throw new Error(`Unknown tool: "${bareName}". Not found in any connected HTTP MCP server.`);
      }

      const sdkClient = clientByServer.get(resolved.serverName);
      if (!sdkClient) {
        throw new Error(`Client missing for server "${resolved.serverName}"`);
      }

      // The SDK's callTool takes bare name + arguments; the transport sends the
      // wire name. We pass the wire name directly so the server schema is authoritative.
      // Input wrapping follows the same convention as the stdio client: pass arguments
      // as-is; the server validates against its own schema.
      const result = await sdkClient.callTool(
        { name: resolved.wireName, arguments: input },
        undefined,
        { timeout: TOOL_CALL_TIMEOUT_MS },
      );

      // Unwrap text content from the canonical MCP response shape.
      const content = result.content as Array<{ type: string; text?: string }>;
      const firstText = content?.find((c) => c.type === "text" && typeof c.text === "string");
      if (firstText?.text != null) {
        try {
          return JSON.parse(firstText.text) as unknown;
        } catch {
          return firstText.text;
        }
      }
      return result;
    },

    async close(): Promise<void> {
      for (const { client } of entries) {
        await client.close();
      }
      toolCatalog = [];
      nameMap = new Map();
      clientByServer.clear();
    },
  };
}
