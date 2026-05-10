/**
 * Macro-analyst skill canary — Phase 33 Wave 2.
 *
 * Proves that loading macro-analyst via the DirectSkillLoader produces an
 * AgentDef that is byte-equivalent (after normalization) to the TypeScript-
 * defined macroAnalyst exported from src/agents/specialists/macro.ts.
 *
 * Tests:
 *   1. Loader returns a valid AgentDef (shape + key field values).
 *   2. systemPrompt byte-equivalence after whitespace normalization (THE GATE).
 *   3. tools / model / maxTokens / maxRecursionDepth parity.
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
 *   offset reported and fix the SKILL.md body in the macro skill file.
 */

import { describe, expect, it } from "vitest";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { dirname } from "node:path";

// Agent A's loader — will fail-fast at import if index.ts is not yet written.
import { createDirectSkillLoader } from "../src/skills/index.js";

// TypeScript-defined macro (reference / ground truth).
import { macroAnalyst as tsMacro } from "../src/agents/specialists/macro.js";

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

describe("macro-analyst skill canary (Phase 33 Wave 2)", () => {
  // -------------------------------------------------------------------------
  // Test 1 — Loader returns a valid AgentDef
  // -------------------------------------------------------------------------
  it("loads macro-analyst as a valid AgentDef", async () => {
    const def = await loader.loadAgent("macro-analyst");
    expect(def.id).toBe("macro-analyst");
    expect(Array.isArray(def.tools)).toBe(true);
    expect((def.tools as string[]).length).toBe(tsMacro.tools.length);
    expect(def.model).toBe("claude-opus-4-5");
    expect(def.maxTokens).toBe(8192);
    expect(def.maxRecursionDepth).toBe(0);
  });

  // -------------------------------------------------------------------------
  // Test 2 — Byte-equivalence (THE GATE for Wave 2 switch-over)
  //
  // If this test fails, vitest produces a full string diff. The divergence
  // point is the first character where the two normalized strings differ.
  // Open the SKILL.md at:
  //   .claude/skills/cfa/corp-finance-analyst-macro/SKILL.md
  // and reconcile the prose with macro.ts systemPrompt.
  // -------------------------------------------------------------------------
  it("loaded macro systemPrompt matches the TypeScript-defined macro (normalized)", async () => {
    const def = await loader.loadAgent("macro-analyst");
    const loaded = normalize(def.systemPrompt);
    const original = normalize(tsMacro.systemPrompt);
    expect(loaded).toBe(original);
  });

  // -------------------------------------------------------------------------
  // Test 3 — Tools and model parity
  // -------------------------------------------------------------------------
  it("loaded macro AgentDef has identical tools / model / maxTokens / maxRecursionDepth as the TS macro", async () => {
    const def = await loader.loadAgent("macro-analyst");
    expect(def.tools).toEqual(tsMacro.tools);
    expect(def.model).toBe(tsMacro.model);
    expect(def.maxTokens).toBe(tsMacro.maxTokens);
    expect(def.maxRecursionDepth).toBe(tsMacro.maxRecursionDepth);
  });

  // -------------------------------------------------------------------------
  // Test 4 — Description parity (substring tolerance)
  //
  // The TypeScript description is a single concatenated string; the SKILL.md
  // description is a YAML scalar that may wrap at column boundaries or be
  // a single-sentence summary. We assert that key macro-domain phrases
  // survive rather than byte-comparing the full multi-sentence value.
  // -------------------------------------------------------------------------
  it("loaded macro description matches (substring tolerance for YAML wrapping)", async () => {
    const def = await loader.loadAgent("macro-analyst");
    // Primary: at least one macro-domain term must appear.
    expect(def.description).toMatch(/macro|fx|commodit|sovereign/i);
    // Secondary: institutional-role context phrase.
    expect(def.description).toMatch(/cfa|specialist|analyst|strategist/i);
  });
});
