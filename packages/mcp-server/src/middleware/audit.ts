/**
 * MCP tool-call audit middleware (Phase 29 Wave 10).
 *
 * PRIVACY NOTE: Only sha256 hashes of input/output are persisted, never the
 * raw contents. Tool parameters may contain PII or trade-sensitive data; hashing
 * ensures the audit log is forensically useful without retaining sensitive payload.
 *
 * PERFORMANCE NOTE: Synchronous appendFileSync is the v1 design choice. If audit
 * write latency becomes a bottleneck under high call volume, replace with a
 * buffered async writer backed by a drain queue. Do not pre-optimize.
 */

import { createHash, randomUUID } from "node:crypto";
import { appendFileSync, mkdirSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, join } from "node:path";
import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface AuditRecord {
  call_id: string;
  timestamp: string;
  tool: string;
  input_sha256: string;
  output_sha256: string | null;
  latency_ms: number;
  ok: boolean;
  error?: string;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function sha256(value: string): string {
  return createHash("sha256").update(value, "utf8").digest("hex");
}

function resolveAuditPath(): string {
  const envPath = process.env["CFA_AUDIT_LOG_PATH"];
  if (envPath) return envPath;
  const dateStr = new Date().toISOString().slice(0, 10); // YYYY-MM-DD UTC
  return join(homedir(), ".cfa-agent", "audit", `mcp-${dateStr}.jsonl`);
}

function appendAuditRecord(filePath: string, record: AuditRecord): void {
  try {
    mkdirSync(dirname(filePath), { recursive: true });
    appendFileSync(filePath, JSON.stringify(record) + "\n", "utf8");
  } catch (writeErr) {
    console.error("[audit] Failed to write audit record:", writeErr);
    // Deliberately swallowed — audit failure must not fail the tool call.
  }
}

// ---------------------------------------------------------------------------
// Middleware
// ---------------------------------------------------------------------------

/**
 * Wraps every `server.tool()` registration with audit-emit logic.
 * Returns the same McpServer instance (fluent).
 *
 * Call_id uses crypto.randomUUID() (UUID v4). UUID v7 (time-ordered) would
 * require the `uuid` package which is not a declared dependency of this
 * package. v4 is noted as a deviation; callers may upgrade to v7 later by
 * adding `uuid` to package.json and swapping the call below.
 */
export function withAudit(server: McpServer): McpServer {
  const auditPath = resolveAuditPath();

  // Save the original .tool method so we can delegate to it.
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const originalTool = server.tool.bind(server) as (...args: any[]) => unknown;

  // Patch .tool — all overloads funnel here.
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  (server as any).tool = function (...args: any[]): unknown {
    // The last argument is always the handler callback.
    const handlerIndex = args.length - 1;
    const toolName: string = args[0];
    const originalHandler = args[handlerIndex] as (...handlerArgs: unknown[]) => Promise<unknown>;

    const auditedHandler = async (...handlerArgs: unknown[]): Promise<unknown> => {
      const call_id = randomUUID();
      const timestamp = new Date().toISOString();
      // handlerArgs[0] is the parsed params object (may be undefined for zero-arg tools).
      const paramsJson = JSON.stringify(handlerArgs[0] ?? null);
      const input_sha256 = sha256(paramsJson);
      const start = Date.now();

      try {
        const result = await originalHandler(...handlerArgs);
        const latency_ms = Date.now() - start;

        // Compute output hash from the JSON-serialised response payload.
        const resultJson = JSON.stringify(result);
        const output_sha256 = sha256(resultJson);

        appendAuditRecord(auditPath, {
          call_id,
          timestamp,
          tool: toolName,
          input_sha256,
          output_sha256,
          latency_ms,
          ok: true,
        });

        return result;
      } catch (e: unknown) {
        const latency_ms = Date.now() - start;

        appendAuditRecord(auditPath, {
          call_id,
          timestamp,
          tool: toolName,
          input_sha256,
          output_sha256: null,
          latency_ms,
          ok: false,
          error: String(e),
        });

        throw e;
      }
    };

    const patchedArgs = [...args];
    patchedArgs[handlerIndex] = auditedHandler;
    return originalTool(...patchedArgs);
  };

  return server;
}
