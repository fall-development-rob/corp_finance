/**
 * Skill-editor bank factory — Phase 41 Wave 3.
 *
 * Provides a ReasoningBank suitable for the skill-editor CLI.
 * When --bucket is supplied (or CFA_BANK_BACKEND env is set), routes to the
 * S3-backed bank. Otherwise returns an empty in-memory bank so the CLI can
 * run offline without error.
 *
 * Parallel agent D wires the full S3Bank implementation. This module is the
 * seam that will be updated then — currently the S3 path returns empty results
 * so that `analyse` produces zero clusters (correct for an empty/offline bank).
 */
import type { ReasoningBank, ReasoningEntry, GraphRecallQuery, RecallOptions } from "./bank.js";

export interface SkillEditorBankOptions {
  bucket?: string;
  repoRoot: string;
}

/** An always-empty ReasoningBank that satisfies the interface. */
function createEmptyBank(): ReasoningBank {
  return {
    async index(): Promise<void> {},
    async recallSimilar(_query: string, _opts?: RecallOptions): Promise<ReasoningEntry[]> {
      return [];
    },
    async recallByGraph(_query: GraphRecallQuery): Promise<ReasoningEntry[]> {
      return [];
    },
    async close(): Promise<void> {},
  };
}

/**
 * Open the appropriate bank for the skill-editor CLI.
 * S3 path is a stub until the parallel D agent wires it in.
 */
export async function createInMemoryBank(
  opts: SkillEditorBankOptions,
): Promise<ReasoningBank> {
  const backend = opts.bucket ?? process.env["CFA_BANK_BACKEND"];

  if (backend) {
    // S3 bank wiring point — parallel D agent provides the real implementation.
    // For now, log and fall through to empty bank so the CLI doesn't crash.
    process.stderr.write(
      `[skill-editor] bank backend "${backend}" requested but S3 wiring not yet active — using empty bank\n`,
    );
  }

  return createEmptyBank();
}
