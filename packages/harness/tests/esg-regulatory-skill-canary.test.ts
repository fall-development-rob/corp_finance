/**
 * ESG/Regulatory-analyst skill canary — Phase 33 Wave 2.
 *
 * Proves that loading esg-regulatory-analyst via the DirectSkillLoader produces
 * an AgentDef that is byte-equivalent (after normalization) to the TypeScript-
 * defined esgRegulatoryAnalyst exported from
 * src/agents/specialists/esg-regulatory.ts.
 *
 * Tests:
 *   1. Loader returns a valid AgentDef (shape + key field values).
 *   2. systemPrompt byte-equivalence after whitespace normalization (THE GATE).
 *   3. tools / model / maxTokens / maxRecursionDepth parity.
 *   4. description contains ESG/regulatory/compliance + analyst phrases.
 *
 * Normalization contract (normalize()):
 *   • Trim leading/trailing whitespace.
 *   • Normalize CRLF → LF.
 *   • Collapse trailing whitespace on every line (tabs/spaces before \n).
 *   • Collapse 3+ consecutive blank lines to 2 blank lines (\n\n).
 *
 *   Word content is NEVER elided. If Test 2 fails, vitest prints a unified
 *   diff showing the first divergence; locate the diff at the character
 *   offset reported and fix the SKILL.md body in the esg-regulatory skill.
 */

import { describe, expect, it } from "vitest";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { dirname } from "node:path";

// Agent A's loader.
import { createDirectSkillLoader } from "../src/skills/index.js";

// TypeScript-defined ESG/regulatory specialist (reference / ground truth).
import { esgRegulatoryAnalyst as tsESG } from "../src/agents/specialists/esg-regulatory.js";

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

describe("esg-regulatory-analyst skill canary (Phase 33 Wave 2)", () => {
  // -------------------------------------------------------------------------
  // Test 1 — Loader returns a valid AgentDef
  // -------------------------------------------------------------------------
  it("loads esg-regulatory-analyst as a valid AgentDef", async () => {
    const def = await loader.loadAgent("esg-regulatory-analyst");
    expect(def.id).toBe("esg-regulatory-analyst");
    expect(Array.isArray(def.tools)).toBe(true);
    expect((def.tools as string[]).length).toBe(tsESG.tools.length);
    expect(def.model).toBe("claude-opus-4-5");
    expect(def.maxTokens).toBe(8192);
    expect(def.maxRecursionDepth).toBe(0);
    expect(def.systemPrompt.length).toBeGreaterThan(5000);
  });

  // -------------------------------------------------------------------------
  // Test 2 — Byte-equivalence (THE GATE for switch-over)
  //
  // If this test fails, vitest produces a full string diff. The divergence
  // point is the first character where the two normalized strings differ.
  // Open the SKILL.md at:
  //   .claude/skills/cfa/corp-finance-analyst-esg-regulatory/SKILL.md
  // and reconcile the prose with esg-regulatory.ts systemPrompt.
  // -------------------------------------------------------------------------
  it("loaded esg-regulatory systemPrompt matches the TypeScript-defined specialist (normalized)", async () => {
    const def = await loader.loadAgent("esg-regulatory-analyst");
    const loaded = normalize(def.systemPrompt);
    const original = normalize(tsESG.systemPrompt);
    expect(loaded).toBe(original);
  });

  // -------------------------------------------------------------------------
  // Test 3 — Tools and model parity
  // -------------------------------------------------------------------------
  it("loaded esg-regulatory AgentDef has identical tools / model / maxTokens / maxRecursionDepth as the TS specialist", async () => {
    const def = await loader.loadAgent("esg-regulatory-analyst");
    expect(def.tools).toEqual(tsESG.tools);
    expect(def.model).toBe(tsESG.model);
    expect(def.maxTokens).toBe(tsESG.maxTokens);
    expect(def.maxRecursionDepth).toBe(tsESG.maxRecursionDepth);
  });

  // -------------------------------------------------------------------------
  // Test 4 — Description parity (substring tolerance)
  //
  // The TypeScript description is a single concatenated string; the SKILL.md
  // description is a YAML scalar that may wrap at column boundaries or be a
  // single-sentence summary. We assert key ESG/regulatory phrases survive
  // rather than byte-comparing the full multi-sentence value.
  // -------------------------------------------------------------------------
  it("loaded esg-regulatory description matches (substring tolerance for YAML wrapping)", async () => {
    const def = await loader.loadAgent("esg-regulatory-analyst");
    // Primary: ESG/regulatory/compliance/carbon/Basel domain term must appear.
    expect(def.description).toMatch(/esg|regulator|compliance|carbon|basel/i);
    // Secondary: CFA / specialist / analyst role term must appear.
    expect(def.description).toMatch(/cfa|specialist|analyst/i);
  });
});
