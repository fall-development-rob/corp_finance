/**
 * SSE MCP client — connects to remote MCP servers via HTTP Server-Sent Events.
 *
 * Transport model (per MCP spec):
 *   - The server pushes JSON-RPC responses on an SSE stream (GET /sse).
 *   - The client sends JSON-RPC requests via HTTP POST to a separate endpoint.
 *
 * Name disambiguation:
 *   - `SSEMCPServerConfig.prefix` is used ONLY for catalog disambiguation
 *     (stripping a common prefix from tool names for the bare-name surface).
 *   - SSE servers typically expose bare tool names directly (no wire prefix).
 *     If the server sends prefixed names the prefix is stripped; if it sends
 *     bare names the prefix field should be set to "" and names pass through.
 *
 * Tool argument passing:
 *   - Stdio plugin servers wrap arguments as `{ input: { ...fields } }` due to
 *     the NAPI boundary. SSE servers are assumed to follow the MCP spec directly:
 *     `arguments` is the flat input object as declared in the tool's inputSchema.
 *     Do NOT wrap in `{ input: ... }` for SSE transports.
 */

import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { SSEClientTransport } from "@modelcontextprotocol/sdk/client/sse.js";
import type { MCPClient, CanonicalTool } from "../types.js";

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

export interface SSEMCPServerConfig {
  name: string;
  prefix: string;
  /** Base URL of the SSE endpoint (e.g., https://mcp.example.com/sse). */
  url: string;
  /** Optional HTTP headers (e.g., Authorization) for the SSE connection. */
  headers?: Record<string, string>;
}

// ---------------------------------------------------------------------------
// Internal types
// ---------------------------------------------------------------------------

interface SdkTool {
  name: string;
  description?: string;
  inputSchema?: {
    type?: string;
    properties?: Record<string, unknown>;
    required?: string[];
  };
}

interface ServerEntry {
  config: SSEMCPServerConfig;
  client: Client;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const TOOL_CALL_TIMEOUT_MS = 30_000;

function extractBare(wireName: string, prefix: string): string {
  if (prefix && wireName.startsWith(prefix)) {
    return wireName.slice(prefix.length);
  }
  return wireName;
}

function sdkToolToCanonical(tool: SdkTool, prefix: string): CanonicalTool {
  return {
    name: extractBare(tool.name, prefix),
    description: tool.description ?? "",
    input_schema: {
      type: "object",
      properties: tool.inputSchema?.properties ?? {},
      required: tool.inputSchema?.required,
    },
  };
}

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

/**
 * Creates and returns an MCPClient that connects to all specified MCP servers
 * via SSE transport. No server processes are spawned; connections are HTTP-based.
 */
export function createSSEMCPClient(servers: SSEMCPServerConfig[]): MCPClient {
  const entries = new Map<string, ServerEntry>();
  let toolCatalog: CanonicalTool[] = [];
  // bare name → server name
  const bareToServer = new Map<string, string>();

  return {
    async initialize(): Promise<void> {
      toolCatalog = [];
      bareToServer.clear();

      for (const config of servers) {
        const transport = new SSEClientTransport(new URL(config.url), {
          requestInit: config.headers
            ? { headers: config.headers }
            : undefined,
        });

        const sdkClient = new Client(
          { name: "cfa-harness-sse", version: "0.1.0" },
          { capabilities: {} },
        );

        try {
          await sdkClient.connect(transport);
        } catch (err) {
          const msg = err instanceof Error ? err.message : String(err);
          throw new Error(
            `SSE MCP connection failed for server "${config.name}" at ${config.url}: ${msg}`,
          );
        }

        entries.set(config.name, { config, client: sdkClient });

        const listResult = await sdkClient.listTools();
        const sdkTools = (listResult.tools ?? []) as SdkTool[];

        for (const sdkTool of sdkTools) {
          const canonical = sdkToolToCanonical(sdkTool, config.prefix);
          if (bareToServer.has(canonical.name)) {
            const existing = bareToServer.get(canonical.name)!;
            throw new Error(
              `Tool name collision: bare name "${canonical.name}" is exposed by ` +
                `both "${existing}" and "${config.name}"`,
            );
          }
          bareToServer.set(canonical.name, config.name);
          toolCatalog.push(canonical);
        }
      }
    },

    async listTools(): Promise<CanonicalTool[]> {
      return toolCatalog;
    },

    async callTool(
      bareName: string,
      input: Record<string, unknown>,
    ): Promise<unknown> {
      const serverName = bareToServer.get(bareName);
      if (!serverName) {
        throw new Error(
          `Unknown tool: "${bareName}". Not found in any connected SSE MCP server.`,
        );
      }

      const entry = entries.get(serverName);
      if (!entry) {
        throw new Error(`Server entry missing for "${serverName}"`);
      }

      // SSE servers follow MCP spec directly: pass arguments as flat input.
      // Do NOT wrap in { input: ... } — that is a stdio/NAPI-only convention.
      const resultPromise = entry.client.callTool({
        name: bareName,
        arguments: input,
      });

      const timeoutPromise = new Promise<never>((_, reject) =>
        setTimeout(
          () =>
            reject(
              new Error(
                `Tool call "${bareName}" timed out after ${TOOL_CALL_TIMEOUT_MS}ms`,
              ),
            ),
          TOOL_CALL_TIMEOUT_MS,
        ),
      );

      const result = await Promise.race([resultPromise, timeoutPromise]);

      // MCP spec: result.content is an array of content blocks.
      const content = (result as { content?: Array<{ type: string; text?: string }> })
        .content;
      const first = content?.[0];
      if (first?.type === "text" && typeof first.text === "string") {
        try {
          return JSON.parse(first.text) as unknown;
        } catch {
          return first.text;
        }
      }
      return result;
    },

    async close(): Promise<void> {
      for (const entry of entries.values()) {
        try {
          await entry.client.close();
        } catch {
          // Best-effort teardown; ignore close errors.
        }
      }
      entries.clear();
      toolCatalog = [];
      bareToServer.clear();
    },
  };
}
