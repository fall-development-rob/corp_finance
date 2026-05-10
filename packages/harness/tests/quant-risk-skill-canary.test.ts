/**
 * Quant-risk-analyst skill canary — Phase 33 Wave 2.
 *
 * Proves that loading quant-risk-analyst via the DirectSkillLoader produces
 * an AgentDef that is byte-equivalent (after normalization) to the
 * TypeScript-defined quantRiskAnalyst exported from
 * src/agents/specialists/quant-risk.ts.
 *
 * Tests:
 *   1. Loader returns a valid AgentDef (shape + key field values).
 *   2. systemPrompt byte-equivalence after whitespace normalization (THE GATE).
 *   3. tools array deep-equality with the TS specialist (order preserved).
 *   4. description contains key phrases (substring tolerance for YAML wrapping).
 *
 * Normalization contract (normalize()):
 *   • Trim leading/trailing whitespace.
 *   • Normalize CRLF → LF.
 *   • Collapse trailing whitespace on every line (tabs/spaces before \n).
 *   • Collapse 3+ consecutive blank lines to 2 blank lines (\n\n).
 *
 *   Word content is NEVER elided. If Test 2 fails, vitest prints a unified
 *   diff showing the first divergence; locate the diff at the character
 *   offset reported and fix the SKILL.md body in the quant-risk skill file.
 */

import { describe, expect, it } from "vitest";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { dirname } from "node:path";

// Agent A's loader.
import { createDirectSkillLoader } from "../src/skills/index.js";

// TypeScript-defined quant-risk specialist (reference / ground truth).
import { quantRiskAnalyst as tsQR } from "../src/agents/specialists/quant-risk.js";

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

describe("quant-risk-analyst skill canary (Phase 33 Wave 2)", () => {
  // -------------------------------------------------------------------------
  // Test 1 — Loader returns a valid AgentDef
  // -------------------------------------------------------------------------
  it("loads quant-risk-analyst as a valid AgentDef", async () => {
    const def = await loader.loadAgent("quant-risk-analyst");
    expect(def.id).toBe("quant-risk-analyst");
    expect(Array.isArray(def.tools)).toBe(true);
    expect((def.tools as string[]).length).toBe(tsQR.tools.length);
    expect(def.model).toBe("claude-opus-4-5");
    expect(def.maxTokens).toBe(8192);
    expect(def.maxRecursionDepth).toBe(0);
    expect(def.systemPrompt.length).toBeGreaterThan(5000);
  });

  // -------------------------------------------------------------------------
  // Test 2 — Byte-equivalence (THE GATE for Wave 2 switch-over)
  //
  // If this test fails, vitest produces a full string diff. The divergence
  // point is the first character where the two normalized strings differ.
  // Open the SKILL.md at:
  //   .claude/skills/cfa/corp-finance-analyst-quant-risk/SKILL.md
  // and reconcile the prose with quant-risk.ts systemPrompt.
  // -------------------------------------------------------------------------
  it("loaded quant-risk systemPrompt matches the TypeScript-defined specialist (normalized)", async () => {
    const def = await loader.loadAgent("quant-risk-analyst");
    const loaded = normalize(def.systemPrompt);
    const original = normalize(tsQR.systemPrompt);
    expect(loaded).toBe(original);
  });

  // -------------------------------------------------------------------------
  // Test 3 — Tools array deep-equality (order preserved)
  // -------------------------------------------------------------------------
  it("loaded quant-risk AgentDef tools array matches the TS specialist exactly", async () => {
    const def = await loader.loadAgent("quant-risk-analyst");
    expect(def.tools).toEqual(tsQR.tools);
  });

  // -------------------------------------------------------------------------
  // Test 4 — Description parity (substring tolerance)
  //
  // The TypeScript description is a single concatenated string; the manifest
  // description is a YAML scalar that may be a shorter summary. We assert
  // that key domain phrases survive rather than byte-comparing the full
  // multi-sentence value.
  //
  // Tolerance: primary check (/risk|factor|portfolio|quant/i) matches the
  // quant-risk domain; secondary check (/cfa|specialist|analyst/i) ensures
  // institutional context is preserved regardless of phrasing or wrapping.
  // -------------------------------------------------------------------------
  it("loaded quant-risk description matches (substring tolerance for YAML wrapping)", async () => {
    const def = await loader.loadAgent("quant-risk-analyst");
    expect(def.description).toMatch(/risk|factor|portfolio|quant/i);
    expect(def.description).toMatch(/cfa|specialist|analyst/i);
  });
});
