/**
 * Unit tests for the MCP audit middleware (Phase 29 Wave 10).
 *
 * We use a minimal mock of McpServer rather than instantiating the real server,
 * which requires NAPI bindings. The mock only needs a `.tool` method because
 * withAudit patches exactly that property.
 */

import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createHash } from "node:crypto";
import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { withAudit } from "./audit.js";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function sha256(value: string): string {
  return createHash("sha256").update(value, "utf8").digest("hex");
}

// Minimal McpServer mock — only .tool is needed by withAudit.
type ToolHandler = (...args: unknown[]) => Promise<unknown>;
interface CapturedTool {
  name: string;
  handler: ToolHandler;
}

function makeMockServer(): { server: McpServer; captured: CapturedTool[] } {
  const captured: CapturedTool[] = [];
  const server = {
    tool(...args: unknown[]) {
      const name = args[0] as string;
      const handler = args[args.length - 1] as ToolHandler;
      captured.push({ name, handler });
      return {};
    },
  } as unknown as McpServer;
  return { server, captured };
}

// UUID v4 regex (crypto.randomUUID output).
const UUID_V4_RE =
  /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;

// ---------------------------------------------------------------------------
// Test suite
// ---------------------------------------------------------------------------

describe("withAudit middleware", () => {
  let tmpDir: string;
  let auditFile: string;
  let originalEnv: string | undefined;

  beforeEach(() => {
    tmpDir = mkdtempSync(join(tmpdir(), "audit-test-"));
    auditFile = join(tmpDir, "test-audit.jsonl");
    originalEnv = process.env["CFA_AUDIT_LOG_PATH"];
    process.env["CFA_AUDIT_LOG_PATH"] = auditFile;
  });

  afterEach(() => {
    if (originalEnv === undefined) {
      delete process.env["CFA_AUDIT_LOG_PATH"];
    } else {
      process.env["CFA_AUDIT_LOG_PATH"] = originalEnv;
    }
    rmSync(tmpDir, { recursive: true, force: true });
  });

  // -------------------------------------------------------------------------

  it("returns the same server instance (fluent)", () => {
    const { server } = makeMockServer();
    const result = withAudit(server);
    expect(result).toBe(server);
  });

  // -------------------------------------------------------------------------

  it("emits one audit record for a successful tool call", async () => {
    const { server, captured } = makeMockServer();
    withAudit(server);

    const params = { ticker: "AAPL", period: 3 };
    const response = { content: [{ type: "text", text: "42" }] };

    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (server as any).tool("test_tool", "A test tool", {}, async (_p: unknown) => response);

    expect(captured).toHaveLength(1);
    await captured[0]!.handler(params);

    const lines = readFileSync(auditFile, "utf8").trim().split("\n");
    expect(lines).toHaveLength(1);

    const record = JSON.parse(lines[0]!) as Record<string, unknown>;

    expect(record["tool"]).toBe("test_tool");
    expect(record["ok"]).toBe(true);
    expect(record["error"]).toBeUndefined();
    expect(typeof record["latency_ms"]).toBe("number");
    expect((record["latency_ms"] as number) >= 0).toBe(true);

    // Validate sha256 fields — must be 64-hex strings.
    const hexRe = /^[0-9a-f]{64}$/;
    expect(hexRe.test(record["input_sha256"] as string)).toBe(true);
    expect(hexRe.test(record["output_sha256"] as string)).toBe(true);

    // Validate call_id UUID v4 format.
    expect(UUID_V4_RE.test(record["call_id"] as string)).toBe(true);

    // Validate timestamp is ISO 8601.
    expect(() => new Date(record["timestamp"] as string)).not.toThrow();
    expect(isNaN(new Date(record["timestamp"] as string).getTime())).toBe(false);
  });

  // -------------------------------------------------------------------------

  it("emits ok:false and error string when handler throws", async () => {
    const { server, captured } = makeMockServer();
    withAudit(server);

    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (server as any).tool("boom_tool", "Throws", {}, async () => {
      throw new Error("intentional failure");
    });

    await expect(captured[0]!.handler({})).rejects.toThrow("intentional failure");

    const lines = readFileSync(auditFile, "utf8").trim().split("\n");
    expect(lines).toHaveLength(1);

    const record = JSON.parse(lines[0]!) as Record<string, unknown>;
    expect(record["tool"]).toBe("boom_tool");
    expect(record["ok"]).toBe(false);
    expect(typeof record["error"]).toBe("string");
    expect((record["error"] as string).length > 0).toBe(true);
    expect(record["output_sha256"]).toBeNull();

    // call_id still present and valid.
    expect(UUID_V4_RE.test(record["call_id"] as string)).toBe(true);
  });

  // -------------------------------------------------------------------------

  it("input_sha256 is deterministic for identical params", async () => {
    const { server, captured } = makeMockServer();
    withAudit(server);

    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (server as any).tool("deterministic_tool", "Deterministic", {}, async () => ({ content: [] }));

    const params = { x: 1, y: "hello" };
    await captured[0]!.handler(params);
    await captured[0]!.handler(params);

    const lines = readFileSync(auditFile, "utf8").trim().split("\n");
    expect(lines).toHaveLength(2);

    const r1 = JSON.parse(lines[0]!) as Record<string, unknown>;
    const r2 = JSON.parse(lines[1]!) as Record<string, unknown>;

    expect(r1["input_sha256"]).toBe(r2["input_sha256"]);

    // Verify the hash matches what we would compute ourselves.
    const expected = sha256(JSON.stringify(params));
    expect(r1["input_sha256"]).toBe(expected);
  });

  // -------------------------------------------------------------------------

  it("records multiple tools independently", async () => {
    const { server, captured } = makeMockServer();
    withAudit(server);

    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (server as any).tool("tool_alpha", "Alpha", {}, async () => ({ content: [{ type: "text" as const, text: "a" }] }));
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (server as any).tool("tool_beta", "Beta", {}, async () => ({ content: [{ type: "text" as const, text: "b" }] }));

    await captured[0]!.handler({ n: 1 });
    await captured[1]!.handler({ n: 2 });

    const lines = readFileSync(auditFile, "utf8").trim().split("\n");
    expect(lines).toHaveLength(2);

    const names = lines.map((l) => (JSON.parse(l) as Record<string, unknown>)["tool"]);
    expect(names).toContain("tool_alpha");
    expect(names).toContain("tool_beta");
  });
});
