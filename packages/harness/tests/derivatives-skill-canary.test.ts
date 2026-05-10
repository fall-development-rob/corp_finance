/**
 * Derivatives-analyst skill canary — Phase 33 Wave 2.
 *
 * Proves that loading derivatives-analyst via the DirectSkillLoader produces an
 * AgentDef that is byte-equivalent (after normalization) to the TypeScript-
 * defined derivativesAnalyst exported from
 * src/agents/specialists/derivatives.ts.
 *
 * Tests:
 *   1. Loader returns a valid AgentDef (shape + key field values).
 *   2. systemPrompt byte-equivalence after whitespace normalization (THE GATE).
 *   3. tools / model / maxTokens / maxRecursionDepth parity.
 *   4. description contains key derivatives-domain phrases.
 *
 * Normalization contract (normalize()):
 *   • Trim leading/trailing whitespace.
 *   • Normalize CRLF → LF.
 *   • Collapse trailing whitespace on every line (tabs/spaces before \n).
 *   • Collapse 3+ consecutive blank lines to 2 blank lines (\n\n).
 *
 *   Word content is NEVER elided. If Test 2 fails, vitest prints a unified
 *   diff showing the first divergence; locate the diff at the character
 *   offset reported and fix the SKILL.md body in the derivatives skill file.
 */

import { describe, expect, it } from "vitest";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { dirname } from "node:path";

// Agent A's loader.
import { createDirectSkillLoader } from "../src/skills/index.js";

// TypeScript-defined derivatives specialist (reference / ground truth).
import { derivativesAnalyst as tsDeriv } from "../src/agents/specialists/derivatives.js";

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

describe("derivatives-analyst skill canary (Phase 33 Wave 2)", () => {
  // -------------------------------------------------------------------------
  // Test 1 — Loader returns a valid AgentDef
  // -------------------------------------------------------------------------
  it("loads derivatives-analyst as a valid AgentDef", async () => {
    const def = await loader.loadAgent("derivatives-analyst");
    expect(def.id).toBe("derivatives-analyst");
    expect(Array.isArray(def.tools)).toBe(true);
    expect((def.tools as string[]).length).toBe(tsDeriv.tools.length);
    expect(def.model).toBe("claude-opus-4-5");
    expect(def.maxTokens).toBe(8192);
    expect(def.maxRecursionDepth).toBe(0);
  });

  // -------------------------------------------------------------------------
  // Test 2 — Byte-equivalence (THE GATE for Wave 2 switch-over)
  // -------------------------------------------------------------------------
  it("loaded derivatives systemPrompt matches the TypeScript-defined derivatives (normalized)", async () => {
    const def = await loader.loadAgent("derivatives-analyst");
    const loaded = normalize(def.systemPrompt);
    const original = normalize(tsDeriv.systemPrompt);
    expect(loaded).toBe(original);
  });

  // -------------------------------------------------------------------------
  // Test 3 — Tools / model / maxTokens / maxRecursionDepth parity
  // -------------------------------------------------------------------------
  it("loaded derivatives AgentDef has identical tools / model / maxTokens / maxRecursionDepth as the TS derivatives", async () => {
    const def = await loader.loadAgent("derivatives-analyst");
    expect(def.tools).toEqual(tsDeriv.tools);
    expect(def.model).toBe(tsDeriv.model);
    expect(def.maxTokens).toBe(tsDeriv.maxTokens);
    expect(def.maxRecursionDepth).toBe(tsDeriv.maxRecursionDepth);
  });

  // -------------------------------------------------------------------------
  // Test 4 — Description parity (substring tolerance)
  // -------------------------------------------------------------------------
  it("loaded derivatives description matches (substring tolerance for YAML wrapping)", async () => {
    const def = await loader.loadAgent("derivatives-analyst");
    // Domain coverage: derivatives terminology must appear.
    expect(def.description).toMatch(/derivat|option|volatility|swap/i);
    // Role/specialist context.
    expect(def.description).toMatch(/cfa|specialist|analyst/i);
  });
});
