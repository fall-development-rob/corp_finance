/**
 * shared-prompt.ts governance tests — Phase 32 Wave 2.
 *
 * Verifies that:
 * 1. Each shared constant is non-empty.
 * 2. No shared constant contains a literal `${` (template-literal escaping bug).
 * 3. Every specialist systemPrompt contains the canonical no-LLM-arithmetic sentence.
 * 4. Every specialist systemPrompt mentions the { "input": { ...params... } } envelope.
 * 5. Every specialist systemPrompt contains the traceability table mandate.
 */

import { describe, expect, it } from "vitest";
import {
  QUALITY_GATE_NO_LLM_ARITHMETIC,
  SPECIALIST_OPERATING_MODE,
  SPECIALIST_PREAMBLE,
  TOOL_CALLING_CONVENTION,
  TRACEABILITY_TABLE_FOOTER,
} from "../src/agents/specialists/shared-prompt.js";
import { creditAnalyst } from "../src/agents/specialists/credit.js";
import { derivativesAnalyst } from "../src/agents/specialists/derivatives.js";
import { equityAnalyst } from "../src/agents/specialists/equity.js";
import { esgRegulatoryAnalyst } from "../src/agents/specialists/esg-regulatory.js";
import { fixedIncomeAnalyst } from "../src/agents/specialists/fixed-income.js";
import { macroAnalyst } from "../src/agents/specialists/macro.js";
import { privateMarketsAnalyst } from "../src/agents/specialists/private-markets.js";
import { quantRiskAnalyst } from "../src/agents/specialists/quant-risk.js";

const ALL_SPECIALISTS = [
  creditAnalyst,
  derivativesAnalyst,
  equityAnalyst,
  esgRegulatoryAnalyst,
  fixedIncomeAnalyst,
  macroAnalyst,
  privateMarketsAnalyst,
  quantRiskAnalyst,
];

describe("shared-prompt constants", () => {
  it("SPECIALIST_OPERATING_MODE is non-empty and has no literal ${", () => {
    expect(SPECIALIST_OPERATING_MODE.length).toBeGreaterThan(0);
    expect(SPECIALIST_OPERATING_MODE).not.toContain("${");
  });

  it("TOOL_CALLING_CONVENTION is non-empty and has no literal ${", () => {
    expect(TOOL_CALLING_CONVENTION.length).toBeGreaterThan(0);
    expect(TOOL_CALLING_CONVENTION).not.toContain("${");
  });

  it("QUALITY_GATE_NO_LLM_ARITHMETIC is non-empty and has no literal ${", () => {
    expect(QUALITY_GATE_NO_LLM_ARITHMETIC.length).toBeGreaterThan(0);
    expect(QUALITY_GATE_NO_LLM_ARITHMETIC).not.toContain("${");
  });

  it("TRACEABILITY_TABLE_FOOTER is non-empty and has no literal ${", () => {
    expect(TRACEABILITY_TABLE_FOOTER.length).toBeGreaterThan(0);
    expect(TRACEABILITY_TABLE_FOOTER).not.toContain("${");
  });

  it("SPECIALIST_PREAMBLE is non-empty and has no literal ${", () => {
    expect(SPECIALIST_PREAMBLE.length).toBeGreaterThan(0);
    expect(SPECIALIST_PREAMBLE).not.toContain("${");
  });
});

describe("specialist systemPrompts carry all governance prose", () => {
  it.each(ALL_SPECIALISTS)(
    "$id: contains canonical no-LLM-arithmetic sentence",
    (specialist) => {
      expect(specialist.systemPrompt).toContain(
        "LLM-generated arithmetic is prohibited"
      );
    }
  );

  it.each(ALL_SPECIALISTS)(
    '$id: contains { "input": { ...params... } } envelope mention',
    (specialist) => {
      expect(specialist.systemPrompt).toContain(
        '{ "input": { ...params... } }'
      );
    }
  );

  it.each(ALL_SPECIALISTS)(
    "$id: contains traceability table mandate",
    (specialist) => {
      expect(specialist.systemPrompt).toContain(
        "| # | Tool | Key Inputs | Output |"
      );
    }
  );
});
