/**
 * Tests for Phase 34 Wave 2 indexer helpers + indexAuditRecord round-trip.
 */
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import {
  aggregateToolCalls,
  createDeterministicEmbedder,
  createRuVectorBank,
  extractDelegations,
  indexAuditRecord,
  summarizePrompt,
  summarizeResult,
  type ReasoningBank,
} from "../src/reasoning/index.js";
import type { AuditRecord, AuditToolCall } from "../src/types.js";

function makeAuditRecord(
  partial: Partial<AuditRecord> & { audit_id: string; agent_id: string },
): AuditRecord {
  return {
    prompt_hash: "prompt-hash-" + partial.audit_id,
    tool_calls: [],
    result_hash: "result-hash-" + partial.audit_id,
    duration_ms: 100,
    total_tool_uses: 0,
    child_audit_ids: [],
    timestamp: new Date("2026-05-10T12:00:00Z").toISOString(),
    ...partial,
  };
}

function makeToolCall(name: string, idSuffix: string = ""): AuditToolCall {
  return {
    id: `call-${name}${idSuffix}`,
    name,
    input_hash: "i" + name + idSuffix,
    result_hash: "r" + name + idSuffix,
    is_error: false,
    duration_ms: 1,
  };
}

describe("summarizePrompt", () => {
  it("passes through inputs ≤ maxChars unchanged", () => {
    const short = "Compute the WACC for AAPL.";
    expect(summarizePrompt(short)).toBe(short);
  });

  it("truncates at last sentence boundary before maxChars", () => {
    // 3 sentences; first two together fit under 200 chars, third pushes over.
    const text =
      "First sentence. Second sentence. " +
      "Third sentence that is intentionally extremely long so the entire prompt " +
      "exceeds two hundred characters for this test, ensuring the truncator " +
      "must cut at the boundary between the second and third sentences.";
    const out = summarizePrompt(text);
    expect(out.length).toBeLessThanOrEqual(200);
    // Must end at a sentence boundary (period, '!', '?' or newline).
    expect(/[.!?\n]$/.test(out)).toBe(true);
    expect(out).toContain("Second sentence.");
  });

  it("hard-truncates with '...' suffix when no sentence boundary exists", () => {
    const noBoundary = "a".repeat(500);
    const out = summarizePrompt(noBoundary);
    expect(out.length).toBe(203); // 200 chars + '...'
    expect(out.endsWith("...")).toBe(true);
  });

  it("respects newline as a sentence boundary", () => {
    const withNewline =
      "Header line\n" + "x".repeat(250);
    const out = summarizePrompt(withNewline);
    expect(out.length).toBeLessThanOrEqual(200);
    expect(out).toBe("Header line\n");
  });

  it("accepts a custom maxChars", () => {
    const text = "abcdef. ghijkl. mnopqr. stuvwx.";
    expect(summarizePrompt(text, 10)).toBe("abcdef.");
  });
});

describe("summarizeResult", () => {
  it("passes through inputs ≤ 500 chars unchanged", () => {
    const r = "Result summary fits.";
    expect(summarizeResult(r)).toBe(r);
  });

  it("truncates at sentence boundary before 500 chars", () => {
    const sentence = "X".repeat(100) + ". ";
    const text = sentence.repeat(10); // ~1020 chars total
    const out = summarizeResult(text);
    expect(out.length).toBeLessThanOrEqual(500);
    expect(/[.!?\n]\s?$/.test(out) || out.endsWith(".")).toBe(true);
  });

  it("hard-truncates result with '...' when no boundary", () => {
    const noBoundary = "z".repeat(800);
    const out = summarizeResult(noBoundary);
    expect(out.length).toBe(503);
    expect(out.endsWith("...")).toBe(true);
  });
});

describe("aggregateToolCalls", () => {
  it("groups by name with descending counts then ascending name on ties", () => {
    const record = makeAuditRecord({
      audit_id: "a1",
      agent_id: "chief",
      tool_calls: [
        makeToolCall("dcf_model", "1"),
        makeToolCall("dcf_model", "2"),
        makeToolCall("dcf_model", "3"),
        makeToolCall("wacc_calculator", "1"),
        makeToolCall("comps_analysis", "1"),
        makeToolCall("comps_analysis", "2"),
      ],
    });
    expect(aggregateToolCalls(record)).toEqual([
      { name: "dcf_model", count: 3 },
      { name: "comps_analysis", count: 2 },
      { name: "wacc_calculator", count: 1 },
    ]);
  });

  it("breaks ties alphabetically", () => {
    const record = makeAuditRecord({
      audit_id: "a2",
      agent_id: "chief",
      tool_calls: [
        makeToolCall("zeta_tool"),
        makeToolCall("alpha_tool"),
        makeToolCall("mike_tool"),
      ],
    });
    expect(aggregateToolCalls(record)).toEqual([
      { name: "alpha_tool", count: 1 },
      { name: "mike_tool", count: 1 },
      { name: "zeta_tool", count: 1 },
    ]);
  });

  it("skips delegate_to_* entries", () => {
    const record = makeAuditRecord({
      audit_id: "a3",
      agent_id: "chief",
      tool_calls: [
        makeToolCall("dcf_model"),
        makeToolCall("delegate_to_equity_analyst"),
        makeToolCall("delegate_to_credit_analyst"),
      ],
    });
    expect(aggregateToolCalls(record)).toEqual([
      { name: "dcf_model", count: 1 },
    ]);
  });

  it("returns an empty array when there are no tool_calls", () => {
    const record = makeAuditRecord({ audit_id: "a4", agent_id: "chief" });
    expect(aggregateToolCalls(record)).toEqual([]);
  });
});

describe("extractDelegations", () => {
  it("returns sorted unique target slugs", () => {
    const record = makeAuditRecord({
      audit_id: "a5",
      agent_id: "chief",
      tool_calls: [
        makeToolCall("delegate_to_equity_analyst", "1"),
        makeToolCall("delegate_to_credit_analyst", "2"),
        makeToolCall("delegate_to_equity_analyst", "3"),
        makeToolCall("dcf_model"),
      ],
    });
    expect(extractDelegations(record)).toEqual([
      "credit_analyst",
      "equity_analyst",
    ]);
  });

  it("returns empty array when there are no delegations", () => {
    const record = makeAuditRecord({
      audit_id: "a6",
      agent_id: "chief",
      tool_calls: [makeToolCall("dcf_model"), makeToolCall("wacc_calculator")],
    });
    expect(extractDelegations(record)).toEqual([]);
  });
});

describe("indexAuditRecord round-trip", () => {
  let tmpDir: string;
  let bank: ReasoningBank | null = null;

  beforeEach(() => {
    tmpDir = mkdtempSync(join(tmpdir(), "indexer-test-"));
    bank = null;
  });

  afterEach(async () => {
    if (bank) {
      try {
        await bank.close();
      } catch {
        // best-effort
      }
    }
    rmSync(tmpDir, { recursive: true, force: true });
  });

  it("indexes a record and recallSimilar returns it", async () => {
    bank = await createRuVectorBank({
      dir: tmpDir,
      embed: createDeterministicEmbedder({ dim: 384 }),
      dimensions: 384,
    });

    const prompt = "Initiate coverage on AAPL with DCF and trading comps.";
    const record = makeAuditRecord({
      audit_id: "audit-aapl-1",
      agent_id: "chief-analyst",
      prompt_hash: "deadbeef",
      tool_calls: [
        makeToolCall("dcf_model", "1"),
        makeToolCall("dcf_model", "2"),
        makeToolCall("delegate_to_equity_analyst"),
        makeToolCall("comps_analysis"),
      ],
      total_tool_uses: 4,
    });

    await indexAuditRecord({
      bank,
      record,
      prompt,
      finalText: "Coverage initiated. Bull case implies 35% upside.",
      metadata: { ticker: "AAPL" },
    });

    const hits = await bank.recallSimilar(prompt, { k: 5 });
    expect(hits.length).toBe(1);
    const hit = hits[0]!;
    expect(hit.audit_id).toBe("audit-aapl-1");
    expect(hit.agent_id).toBe("chief-analyst");
    expect(hit.prompt_hash).toBe("deadbeef");
    expect(hit.prompt_summary).toBe(prompt); // short enough to pass through
    expect(hit.tool_calls).toEqual([
      { name: "dcf_model", count: 2 },
      { name: "comps_analysis", count: 1 },
    ]);
    expect(hit.delegations).toEqual(["equity_analyst"]);
    expect(hit.result_excerpt).toBe(
      "Coverage initiated. Bull case implies 35% upside.",
    );
    expect(hit.metadata).toEqual({ ticker: "AAPL" });
    expect(hit.timestamp).toBe(record.timestamp);
  });

  it("uses an empty metadata object when none is supplied", async () => {
    bank = await createRuVectorBank({
      dir: tmpDir,
      embed: createDeterministicEmbedder({ dim: 384 }),
      dimensions: 384,
    });

    const prompt = "Macro outlook for 2026.";
    const record = makeAuditRecord({
      audit_id: "audit-macro-1",
      agent_id: "macro-analyst",
    });

    await indexAuditRecord({
      bank,
      record,
      prompt,
      finalText: "Stable.",
    });

    const hits = await bank.recallSimilar(prompt);
    expect(hits.length).toBe(1);
    expect(hits[0]!.metadata).toEqual({});
  });
});
