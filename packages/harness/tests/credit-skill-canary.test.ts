/**
 * Credit-analyst skill canary — Phase 33 Wave 2.
 *
 * Proves that loading credit-analyst via the DirectSkillLoader produces an
 * AgentDef that is byte-equivalent (after normalization) to the TypeScript-
 * defined creditAnalyst exported from src/agents/specialists/credit.ts.
 *
 * Tests:
 *   1. Loader returns a valid AgentDef (shape + key field values).
 *   2. systemPrompt byte-equivalence after whitespace normalization (THE GATE).
 *   3. tools / model / maxTokens / maxRecursionDepth parity.
 *   4. description contains key credit-domain phrases (substring tolerance).
 *
 * Normalization contract (normalize()):
 *   • Trim leading/trailing whitespace.
 *   • Normalize CRLF → LF.
 *   • Collapse trailing whitespace on every line (tabs/spaces before \n).
 *   • Collapse 3+ consecutive blank lines to 2 blank lines (\n\n).
 *
 *   Word content is NEVER elided. If Test 2 fails, vitest prints a unified
 *   diff showing the first divergence; locate the diff at the character
 *   offset reported and fix the SKILL.md body in the credit skill file.
 */

import { describe, expect, it } from "vitest";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { dirname } from "node:path";

// Agent A's loader — will fail-fast at import if index.ts is not yet written.
import { createDirectSkillLoader } from "../src/skills/index.js";

// TypeScript-defined credit specialist (reference / ground truth).
import { creditAnalyst as tsCredit } from "../src/agents/specialists/credit.js";

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

describe("credit-analyst skill canary (Phase 33 Wave 2)", () => {
  // -------------------------------------------------------------------------
  // Test 1 — Loader returns a valid AgentDef
  // -------------------------------------------------------------------------
  it("loads credit-analyst as a valid AgentDef", async () => {
    const def = await loader.loadAgent("credit-analyst");
    expect(def.id).toBe("credit-analyst");
    expect(Array.isArray(def.tools)).toBe(true);
    expect((def.tools as string[]).length).toBe(tsCredit.tools.length);
    expect(def.model).toBe("claude-opus-4-5");
    expect(def.maxTokens).toBe(8192);
    expect(def.maxRecursionDepth).toBe(0);
    expect(def.systemPrompt.length).toBeGreaterThan(5000);
  });

  // -------------------------------------------------------------------------
  // Test 2 — Byte-equivalence (THE GATE for the skill switch-over)
  //
  // If this test fails, vitest produces a full string diff. The divergence
  // point is the first character where the two normalized strings differ.
  // Open the SKILL.md at:
  //   .claude/skills/cfa/corp-finance-analyst-credit/SKILL.md
  // and reconcile the prose with credit.ts systemPrompt. Never modify the
  // TypeScript source to make the test pass.
  // -------------------------------------------------------------------------
  it("loaded credit systemPrompt matches the TypeScript-defined credit (normalized)", async () => {
    const def = await loader.loadAgent("credit-analyst");
    const loaded = normalize(def.systemPrompt);
    const original = normalize(tsCredit.systemPrompt);
    expect(loaded).toBe(original);
  });

  // -------------------------------------------------------------------------
  // Test 3 — Tools parity (deep array, order-sensitive)
  // -------------------------------------------------------------------------
  it("loaded credit AgentDef has identical tools / model / maxTokens / maxRecursionDepth as the TS credit", async () => {
    const def = await loader.loadAgent("credit-analyst");
    expect(def.tools).toEqual(tsCredit.tools);
    expect(def.model).toBe(tsCredit.model);
    expect(def.maxTokens).toBe(tsCredit.maxTokens);
    expect(def.maxRecursionDepth).toBe(tsCredit.maxRecursionDepth);
  });

  // -------------------------------------------------------------------------
  // Test 4 — Description parity (substring tolerance for YAML wrapping)
  //
  // The TypeScript description is a single concatenated string; the manifest
  // description may be a one-sentence summary. We assert that key credit-
  // domain phrases survive rather than byte-comparing the full multi-sentence
  // value.
  // -------------------------------------------------------------------------
  it("loaded credit description matches (substring tolerance for YAML wrapping)", async () => {
    const def = await loader.loadAgent("credit-analyst");
    // Primary: at least one credit-domain term must appear.
    expect(def.description).toMatch(/credit|distress|covenant/i);
    // Secondary: institutional / specialist / analyst phrasing must survive.
    expect(def.description).toMatch(/cfa|specialist|analyst/i);
  });
});
