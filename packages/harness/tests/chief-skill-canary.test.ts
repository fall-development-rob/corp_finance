/**
 * Chief-analyst skill canary — Phase 33 Wave 1.
 *
 * Proves that loading chief-analyst via the DirectSkillLoader produces an
 * AgentDef that is byte-equivalent (after normalization) to the TypeScript-
 * defined chiefAnalyst exported from src/agents/chief-analyst.ts.
 *
 * Tests:
 *   1. Loader returns a valid AgentDef (shape + key field values).
 *   2. systemPrompt byte-equivalence after whitespace normalization (THE GATE).
 *   3. tools / model / maxTokens / maxRecursionDepth parity.
 *   4. description contains key phrases (substring tolerance for YAML wrapping).
 *   5. Live smoke test — skipped unless API key + built plugin dist files present.
 *
 * Normalization contract (normalize()):
 *   • Trim leading/trailing whitespace.
 *   • Normalize CRLF → LF.
 *   • Collapse trailing whitespace on every line (tabs/spaces before \n).
 *   • Collapse 3+ consecutive blank lines to 2 blank lines (\n\n).
 *
 *   Word content is NEVER elided. If Test 2 fails, vitest prints a unified
 *   diff showing the first divergence; locate the diff at the character
 *   offset reported and fix the SKILL.md body in the chief skill file.
 */

import { describe, expect, it } from "vitest";
import { existsSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { dirname } from "node:path";

// Agent A's loader — will fail-fast at import if index.ts is not yet written.
import { createDirectSkillLoader } from "../src/skills/index.js";

// TypeScript-defined chief (reference / ground truth).
import { chiefAnalyst as tsChief } from "../src/agents/chief-analyst.js";

// Registry — for defaultMCPServers used in the live skip gate.
import { defaultMCPServers } from "../src/agents/registry.js";

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
// Live test skip gate
// ---------------------------------------------------------------------------

const skipLive =
  !process.env["ANTHROPIC_API_KEY"] ||
  defaultMCPServers.some((s) => !existsSync(s.args[0]!));

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("chief-analyst skill canary (Phase 33 Wave 1)", () => {
  // -------------------------------------------------------------------------
  // Test 1 — Loader returns a valid AgentDef
  // -------------------------------------------------------------------------
  it("loads chief-analyst as a valid AgentDef", async () => {
    const def = await loader.loadAgent("chief-analyst");
    expect(def.id).toBe("chief-analyst");
    expect(def.tools).toBe("*");
    expect(def.model).toBe("claude-opus-4-5");
    expect(def.maxTokens).toBe(16384);
    expect(def.maxRecursionDepth).toBe(1);
    expect(def.systemPrompt.length).toBeGreaterThan(5000);
  });

  // -------------------------------------------------------------------------
  // Test 2 — Byte-equivalence (THE GATE for Wave 2 switch-over)
  //
  // If this test fails, vitest produces a full string diff. The divergence
  // point is the first character where the two normalized strings differ.
  // Open the SKILL.md at:
  //   .claude/skills/cfa/corp-finance-analyst-chief/SKILL.md
  // and reconcile the prose with chief-analyst.ts systemPrompt.
  // -------------------------------------------------------------------------
  it("loaded chief systemPrompt matches the TypeScript-defined chief (normalized)", async () => {
    const def = await loader.loadAgent("chief-analyst");
    const loaded = normalize(def.systemPrompt);
    const original = normalize(tsChief.systemPrompt);
    expect(loaded).toBe(original);
  });

  // -------------------------------------------------------------------------
  // Test 3 — Tools and model parity
  // -------------------------------------------------------------------------
  it("loaded chief AgentDef has identical tools / model / maxTokens / maxRecursionDepth as the TS chief", async () => {
    const def = await loader.loadAgent("chief-analyst");
    expect(def.tools).toEqual(tsChief.tools);
    expect(def.model).toBe(tsChief.model);
    expect(def.maxTokens).toBe(tsChief.maxTokens);
    expect(def.maxRecursionDepth).toBe(tsChief.maxRecursionDepth);
  });

  // -------------------------------------------------------------------------
  // Test 4 — Description parity (substring tolerance)
  //
  // The TypeScript description is a single concatenated string; the SKILL.md
  // description is a YAML scalar that may wrap at column boundaries or be
  // a single-sentence summary. We assert that key institutional-context
  // phrases survive rather than byte-comparing the full multi-sentence value.
  //
  // Tolerance: the primary check (/chief|coordinator|delegate/i) matches even
  // a minimal one-sentence description. The secondary check (/financial/i)
  // ensures domain context is preserved regardless of phrasing or wrapping.
  // If Agent B author writes the full TS description verbatim, all checks pass;
  // if they write a shorter summary, the primary and secondary still pass.
  // -------------------------------------------------------------------------
  it("loaded chief description matches (substring tolerance for YAML wrapping)", async () => {
    const def = await loader.loadAgent("chief-analyst");
    // Primary: at least one of the three coordinator-role terms must appear.
    expect(def.description).toMatch(/chief|coordinator|delegate/i);
    // Secondary: domain context phrase — survives both short and long forms.
    expect(def.description).toMatch(/financial/i);
  });

  // -------------------------------------------------------------------------
  // Test 5 — Live integration smoke test (skipped without API key / built MCPs)
  //
  // Verifies that the loader-built AgentDef survives a real Anthropic dispatch.
  // Uses a minimal Chaco-style prompt; asserts toolUses >= 1.
  // This is a smoke test only; the full Wave 2 routing grid is tested elsewhere.
  // -------------------------------------------------------------------------
  it.skipIf(skipLive)(
    "loaded chief produces a substantive memo on the Chaco prompt (live)",
    async () => {
      const { createAnthropicProvider } = await import(
        "../src/core/providers/anthropic.js"
      );
      const { createStdioMCPClient } = await import(
        "../src/mcp-client/stdio.js"
      );
      const { dispatch } = await import("../src/core/agent-loop.js");

      const def = await loader.loadAgent("chief-analyst");

      const mcp = createStdioMCPClient(defaultMCPServers);
      await mcp.initialize();

      try {
        const provider = createAnthropicProvider({
          apiKey: process.env["ANTHROPIC_API_KEY"]!,
        });

        const result = await dispatch({
          agent: def,
          prompt:
            "Run country_risk_premium for Argentina (country_risk_premium_rate: 0.14, " +
            "risk_free_rate: 0.035) and report the result in one sentence.",
          provider,
          mcp,
          maxTurns: 5,
        });

        expect(result.toolUses).toBeGreaterThanOrEqual(1);
        expect(result.finalText.length).toBeGreaterThan(50);
      } finally {
        await mcp.close();
      }
    },
    300_000,
  );
});
