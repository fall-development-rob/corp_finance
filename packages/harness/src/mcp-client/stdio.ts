/**
 * Stdio MCP client — spawns MCP server processes and speaks JSON-RPC over stdio.
 *
 * Each server process is managed independently. Tool names are translated
 * between bare names (agent-facing) and wire-prefixed names (JSON-RPC).
 */
import { spawn, type ChildProcessWithoutNullStreams } from "node:child_process";
import { randomUUID } from "node:crypto";
import type { MCPClient, MCPServerConfig, CanonicalTool } from "../types.js";
import {
  buildNameMap,
  resolveBareToWire,
  extractBareFromWire,
  type ResolvedTool,
} from "./name-resolver.js";

const TOOL_CALL_TIMEOUT_MS = 30_000;
const MAX_STDERR_BYTES = 1024 * 1024; // 1 MB

interface PendingRequest {
  resolve: (value: JsonRpcResponse) => void;
  reject: (reason: Error) => void;
  timer: ReturnType<typeof setTimeout>;
}

interface JsonRpcResponse {
  jsonrpc: "2.0";
  id: number | string;
  result?: unknown;
  error?: { code: number; message: string; data?: unknown };
}

interface WireToolDef {
  name: string;
  description?: string;
  inputSchema?: {
    type: "object";
    properties?: Record<string, unknown>;
    required?: string[];
  };
}

interface ServerState {
  config: MCPServerConfig;
  process: ChildProcessWithoutNullStreams;
  nextId: number;
  pending: Map<number, PendingRequest>;
  buffer: string;
  stderrSize: number;
}

function sendRequest(
  state: ServerState,
  method: string,
  params: unknown,
): Promise<JsonRpcResponse> {
  const id = state.nextId++;
  const message = JSON.stringify({ jsonrpc: "2.0", id, method, params }) + "\n";

  return new Promise<JsonRpcResponse>((resolve, reject) => {
    const timer = setTimeout(() => {
      state.pending.delete(id);
      reject(new Error(`MCP request "${method}" timed out after ${TOOL_CALL_TIMEOUT_MS}ms`));
    }, TOOL_CALL_TIMEOUT_MS);

    state.pending.set(id, { resolve, reject, timer });
    state.process.stdin.write(message);
  });
}

function handleLine(state: ServerState, line: string): void {
  if (!line) return;
  let msg: JsonRpcResponse;
  try {
    msg = JSON.parse(line) as JsonRpcResponse;
  } catch {
    // Non-JSON banner line (e.g., "cfa-core MCP server v1.0.0 listening on stdio") — skip
    return;
  }
  if (msg.id == null) return; // notification — ignore
  const pending = state.pending.get(msg.id as number);
  if (!pending) return;
  clearTimeout(pending.timer);
  state.pending.delete(msg.id as number);
  pending.resolve(msg);
}

function attachStdout(state: ServerState): void {
  state.process.stdout.on("data", (chunk: Buffer) => {
    state.buffer += chunk.toString("utf8");
    let idx: number;
    while ((idx = state.buffer.indexOf("\n")) >= 0) {
      const line = state.buffer.slice(0, idx).trim();
      state.buffer = state.buffer.slice(idx + 1);
      handleLine(state, line);
    }
  });
}

function attachStderr(state: ServerState): void {
  state.process.stderr.on("data", (chunk: Buffer) => {
    if (state.stderrSize < MAX_STDERR_BYTES) {
      state.stderrSize += chunk.length;
    }
  });
}

async function spawnServer(config: MCPServerConfig): Promise<ServerState> {
  const proc = spawn(config.command, config.args, {
    stdio: ["pipe", "pipe", "pipe"],
    env: { ...process.env, ...(config.env ?? {}) },
  }) as ChildProcessWithoutNullStreams;

  const state: ServerState = {
    config,
    process: proc,
    nextId: 1,
    pending: new Map(),
    buffer: "",
    stderrSize: 0,
  };

  attachStdout(state);
  attachStderr(state);
  return state;
}

async function initializeServer(state: ServerState): Promise<void> {
  const resp = await sendRequest(state, "initialize", {
    protocolVersion: "2025-03-26",
    capabilities: {},
    clientInfo: { name: "cfa-harness", version: "0.1.0" },
  });
  if (resp.error) {
    throw new Error(`MCP initialize failed for "${state.config.name}": ${resp.error.message}`);
  }
}

async function listServerTools(state: ServerState): Promise<WireToolDef[]> {
  const resp = await sendRequest(state, "tools/list", {});
  if (resp.error) {
    throw new Error(`tools/list failed for "${state.config.name}": ${resp.error.message}`);
  }
  const result = resp.result as { tools?: WireToolDef[] };
  return result?.tools ?? [];
}

function wireToolToCanonical(tool: WireToolDef, prefix: string): CanonicalTool {
  const bareName = extractBareFromWire(tool.name, prefix);
  return {
    name: bareName,
    description: tool.description ?? "",
    input_schema: {
      type: "object",
      properties: tool.inputSchema?.properties ?? {},
      required: tool.inputSchema?.required,
    },
  };
}

/**
 * Creates and returns an MCPClient that connects to all specified MCP servers
 * via stdio JSON-RPC transport.
 */
export function createStdioMCPClient(servers: MCPServerConfig[]): MCPClient {
  const serverStates = new Map<string, ServerState>();
  let toolCatalog: CanonicalTool[] = [];
  let nameMap = new Map<string, ResolvedTool>();

  return {
    async initialize(): Promise<void> {
      const wireToolsByServer = new Map<string, string[]>();

      for (const config of servers) {
        const state = await spawnServer(config);
        serverStates.set(config.name, state);
        await initializeServer(state);
        const wireTools = await listServerTools(state);
        wireToolsByServer.set(
          config.name,
          wireTools.map((t) => t.name),
        );

        // Build canonical tools for this server
        for (const wt of wireTools) {
          toolCatalog.push(wireToolToCanonical(wt, config.prefix));
        }
      }

      nameMap = buildNameMap(servers, wireToolsByServer);
    },

    async listTools(): Promise<CanonicalTool[]> {
      return toolCatalog;
    },

    async callTool(bareName: string, input: Record<string, unknown>): Promise<unknown> {
      const resolved = resolveBareToWire(nameMap, bareName);
      if (!resolved) {
        throw new Error(`Unknown tool: "${bareName}". Not found in any connected MCP server.`);
      }

      const state = serverStates.get(resolved.serverName);
      if (!state) {
        throw new Error(`Server state missing for "${resolved.serverName}"`);
      }

      const resp = await sendRequest(state, "tools/call", {
        name: resolved.wireName,
        arguments: { input },
      });

      if (resp.error) {
        throw new Error(
          `Tool "${bareName}" returned error: ${resp.error.message}` +
            (resp.error.data ? ` — ${JSON.stringify(resp.error.data)}` : ""),
        );
      }

      // Plugin returns: { content: [{ type: "text", text: "<json>" }] }
      const result = resp.result as { content?: Array<{ type: string; text: string }> };
      const firstContent = result?.content?.[0];
      if (firstContent?.type === "text" && typeof firstContent.text === "string") {
        try {
          return JSON.parse(firstContent.text) as unknown;
        } catch {
          return firstContent.text;
        }
      }
      return resp.result;
    },

    async close(): Promise<void> {
      for (const state of serverStates.values()) {
        // Reject all pending requests
        for (const pending of state.pending.values()) {
          clearTimeout(pending.timer);
          pending.reject(new Error("MCP client closed"));
        }
        state.pending.clear();
        state.process.kill();
      }
      serverStates.clear();
      toolCatalog = [];
      nameMap = new Map();
    },
  };
}

// Re-export for convenience
export { randomUUID };
