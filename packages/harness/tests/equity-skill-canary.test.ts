/**
 * Equity-analyst skill canary — Phase 33 Wave 2.
 *
 * Proves that loading equity-analyst via the DirectSkillLoader produces an
 * AgentDef that is byte-equivalent (after normalization) to the TypeScript-
 * defined equityAnalyst exported from src/agents/specialists/equity.ts.
 *
 * Tests:
 *   1. Loader returns a valid AgentDef (shape + key field values).
 *   2. systemPrompt byte-equivalence after whitespace normalization (THE GATE).
 *   3. tools / model / maxTokens / maxRecursionDepth parity.
 *   4. description contains key equity-domain phrases (substring tolerance).
 *
 * The live smoke test is intentionally omitted: specialists are not dispatched
 * directly from a top-level prompt — the chief delegates to them. Live testing
 * for specialist routing is Wave 4 work.
 *
 * Normalization contract (normalize()):
 *   • Trim leading/trailing whitespace.
 *   • Normalize CRLF → LF.
 *   • Collapse trailing whitespace on every line (tabs/spaces before \n).
 *   • Collapse 3+ consecutive blank lines to 2 blank lines (\n\n).
 *
 *   Word content is NEVER elided. If Test 2 fails, vitest prints a unified
 *   diff showing the first divergence; locate the diff at the character
 *   offset reported and fix the SKILL.md body in the equity skill file.
 */

import { describe, expect, it } from "vitest";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { dirname } from "node:path";

// Agent A's loader.
import { createDirectSkillLoader } from "../src/skills/index.js";

// TypeScript-defined equity (reference / ground truth).
import { equityAnalyst as tsEquity } from "../src/agents/specialists/equity.js";

// ---------------------------------------------------------------------------
// Path resolution
// ---------------------------------------------------------------------------

const _thisDir = dirname(fileURLToPath(import.meta.url));
// packages/harness/tests -> packages/harness -> packages -> repo root
const repoRoot = resolve(_thisDir, "..", "..", "..");

const loader = createDirectSkillLoader({
  skillsRoot: resolve(repoRoot, ".claude", "skills", "cfa"),
  agentsRoot: resolve(repoRoot, ".claude", "agents", "cfa"),
});

// ---------------------------------------------------------------------------
// Normalization
//
// Normalization rules (must stay in sync with any equivalent in the loader):
//   1. Trim leading/trailing whitespace from the whole string.
//   2. Normalize CRLF → LF (Windows-safe).
//   3. Strip trailing whitespace from every line (tabs/spaces before newline).
//   4. Collapse 3+ consecutive blank lines to exactly 2 (\n\n).
//
// Intentionally NOT normalizing: indentation, inline spacing, or any prose.
// ---------------------------------------------------------------------------

function normalize(s: string): string {
  return s
    .trim()
    .replace(/\r\n/g, "\n")
    .replace(/[ \t]+\n/g, "\n")
    .replace(/\n{3,}/g, "\n\n");
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("equity-analyst skill canary (Phase 33 Wave 2)", () => {
  // -------------------------------------------------------------------------
  // Test 1 — Loader returns a valid AgentDef
  // -------------------------------------------------------------------------
  it("loads equity-analyst as a valid AgentDef", async () => {
    const def = await loader.loadAgent("equity-analyst");
    expect(def.id).toBe("equity-analyst");
    expect(Array.isArray(def.tools) && (def.tools as string[]).length === 72).toBe(true);
    expect(def.model).toBe("claude-opus-4-5");
    expect(def.maxTokens).toBe(8192);
    expect(def.maxRecursionDepth).toBe(0);
    expect(def.systemPrompt.length).toBeGreaterThan(5000);
  });

  // -------------------------------------------------------------------------
  // Test 2 — Byte-equivalence (THE GATE)
  //
  // If this test fails, vitest produces a full string diff. The divergence
  // point is the first character where the two normalized strings differ.
  // Open the SKILL.md at:
  //   .claude/skills/cfa/corp-finance-analyst-equity/SKILL.md
  // and reconcile the prose with equity.ts systemPrompt.
  // -------------------------------------------------------------------------
  it("loaded equity systemPrompt matches the TypeScript-defined equity (normalized)", async () => {
    const def = await loader.loadAgent("equity-analyst");
    const loaded = normalize(def.systemPrompt);
    const original = normalize(tsEquity.systemPrompt);
    expect(loaded).toBe(original);
  });

  // -------------------------------------------------------------------------
  // Test 3 — Tools / model / maxTokens / maxRecursionDepth parity
  //
  // Order-sensitive deep array equality on tools.
  // -------------------------------------------------------------------------
  it("loaded equity AgentDef has identical tools / model / maxTokens / maxRecursionDepth as the TS equity", async () => {
    const def = await loader.loadAgent("equity-analyst");
    expect(def.tools).toEqual(tsEquity.tools);
    expect(def.model).toBe(tsEquity.model);
    expect(def.maxTokens).toBe(tsEquity.maxTokens);
    expect(def.maxRecursionDepth).toBe(tsEquity.maxRecursionDepth);
  });

  // -------------------------------------------------------------------------
  // Test 4 — Description parity (substring tolerance)
  //
  // The TypeScript description is a single concatenated string; the agent
  // manifest description is a YAML scalar that may be a one-sentence summary.
  // Assert that key equity-domain and specialist-role phrases survive rather
  // than byte-comparing the full multi-sentence value.
  // -------------------------------------------------------------------------
  it("loaded equity description matches (substring tolerance for YAML wrapping)", async () => {
    const def = await loader.loadAgent("equity-analyst");
    // Primary: equity-domain context phrase.
    expect(def.description).toMatch(/equity|valuation|fundamental/i);
    // Secondary: CFA / specialist role phrase.
    expect(def.description).toMatch(/cfa|specialist/i);
  });
});
