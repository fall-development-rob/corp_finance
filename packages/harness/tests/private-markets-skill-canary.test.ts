/**
 * Private-markets-analyst skill canary — Phase 33 Wave 2.
 *
 * Proves that loading private-markets-analyst via the DirectSkillLoader
 * produces an AgentDef that is byte-equivalent (after normalization) to the
 * TypeScript-defined privateMarketsAnalyst exported from
 * src/agents/specialists/private-markets.ts.
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
 *   offset reported and fix the SKILL.md body in the private-markets skill file.
 */

import { describe, expect, it } from "vitest";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { dirname } from "node:path";

// Agent A's loader.
import { createDirectSkillLoader } from "../src/skills/index.js";

// TypeScript-defined private-markets specialist (reference / ground truth).
import { privateMarketsAnalyst as tsPM } from "../src/agents/specialists/private-markets.js";

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

describe("private-markets-analyst skill canary (Phase 33 Wave 2)", () => {
  // -------------------------------------------------------------------------
  // Test 1 — Loader returns a valid AgentDef
  // -------------------------------------------------------------------------
  it("loads private-markets-analyst as a valid AgentDef", async () => {
    const def = await loader.loadAgent("private-markets-analyst");
    expect(def.id).toBe("private-markets-analyst");
    expect(Array.isArray(def.tools)).toBe(true);
    expect((def.tools as string[]).length).toBe(tsPM.tools.length);
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
  //   .claude/skills/cfa/corp-finance-analyst-private-markets/SKILL.md
  // and reconcile the prose with private-markets.ts systemPrompt.
  // -------------------------------------------------------------------------
  it("loaded private-markets systemPrompt matches the TypeScript-defined private-markets (normalized)", async () => {
    const def = await loader.loadAgent("private-markets-analyst");
    const loaded = normalize(def.systemPrompt);
    const original = normalize(tsPM.systemPrompt);
    expect(loaded).toBe(original);
  });

  // -------------------------------------------------------------------------
  // Test 3 — Tools and model parity
  // -------------------------------------------------------------------------
  it("loaded private-markets AgentDef has identical tools / model / maxTokens / maxRecursionDepth as the TS private-markets", async () => {
    const def = await loader.loadAgent("private-markets-analyst");
    expect(def.tools).toEqual(tsPM.tools);
    expect(def.model).toBe(tsPM.model);
    expect(def.maxTokens).toBe(tsPM.maxTokens);
    expect(def.maxRecursionDepth).toBe(tsPM.maxRecursionDepth);
  });

  // -------------------------------------------------------------------------
  // Test 4 — Description parity (substring tolerance)
  //
  // The TypeScript description is a single concatenated string; the SKILL.md
  // description is a YAML scalar that may wrap at column boundaries or be
  // a single-sentence summary. We assert that key institutional-context
  // phrases survive rather than byte-comparing the full multi-sentence value.
  // -------------------------------------------------------------------------
  it("loaded private-markets description matches (substring tolerance for YAML wrapping)", async () => {
    const def = await loader.loadAgent("private-markets-analyst");
    // Primary: at least one of the private-markets domain terms must appear.
    expect(def.description).toMatch(/private|lbo|venture|pe|infrastructure/i);
    // Secondary: domain context phrase — survives both short and long forms.
    expect(def.description).toMatch(/cfa|specialist|analyst/i);
  });
});
