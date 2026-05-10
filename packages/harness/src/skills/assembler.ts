/**
 * System-prompt assembler for the Phase 33 skill loader.
 *
 * Composes a final system prompt from an ordered list of parsed skills
 * plus an optional agent-specific body. The composition is deterministic:
 * skill bodies are concatenated in the order they were resolved, separated
 * by a blank line. The agent body (if any) is appended last.
 */

import type { ParsedSkill } from "./types.js";

export interface AssembleOptions {
  skills: ParsedSkill[];
  agentBody?: string;
}

/**
 * Compose a final system prompt from an ordered list of parsed skills
 * (each contributing its body) plus an optional agent-specific body.
 *
 * Trims trailing whitespace; ensures exactly one blank line between
 * sections. Empty sections are skipped.
 */
export function assembleSystemPrompt(opts: AssembleOptions): string {
  const sections: string[] = [];

  for (const skill of opts.skills) {
    const trimmed = skill.body.trim();
    if (trimmed.length > 0) {
      sections.push(trimmed);
    }
  }

  if (opts.agentBody !== undefined) {
    const trimmed = opts.agentBody.trim();
    if (trimmed.length > 0) {
      sections.push(trimmed);
    }
  }

  return sections.join("\n\n");
}
