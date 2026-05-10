/**
 * Shared system-prompt fragments interpolated by every CFA specialist
 * agent. Centralised so governance edits (LLM-arithmetic-prohibited, the
 * traceability mandate, the tool-calling envelope) are 1-file changes
 * instead of 8-file copy-paste.
 */

/**
 * Operating mode preamble — every specialist agent's first operating-mode
 * section starts with this paragraph (or incorporates it). Standard across
 * all 8 specialists.
 */
export const SPECIALIST_OPERATING_MODE = `\
You receive a self-contained \`sub_prompt\` from the chief. You do not see \
the parent conversation or any context outside what was passed to you. Treat \
the sub_prompt (and any structured context block below it) as the complete \
specification of your task. Produce a structured analysis the chief can \
incorporate into their memo, including your own tool-call traceability table. \
Do not ask follow-up questions; work with what you have and flag any data \
gaps explicitly.`;

/**
 * Tool-calling convention block — specialists MUST use the bare tool name
 * and the wrapped { "input": { ... } } envelope. Identical across all 8
 * specialists; centralised to prevent drift.
 */
export const TOOL_CALLING_CONVENTION = `\
Call tools by bare name (e.g., \`option_pricer\`, NOT \
\`mcp__plugin_cfa-core_cfa-core__option_pricer\`). The harness translates \
bare names to the wire-prefixed form internally — never include the wire \
prefix in tool calls. Wrap every input in the standard envelope:

  { "input": { ...params... } }

Execute independent calls in the same turn. For chains (data → compute), \
retrieve data first, then pass exact values into compute tools in the next \
turn. Never interpolate or re-derive values that came from a prior tool result.`;

/**
 * Quality-gate prose — the "no LLM arithmetic" rule. Specialists embed
 * this sentence in their quality-gate sections.
 */
export const QUALITY_GATE_NO_LLM_ARITHMETIC = `\
LLM-generated arithmetic is prohibited. Every numerical result must be \
sourced from a tool invocation with logged inputs. If a number cannot be \
sourced from an available tool, state the gap explicitly and identify the \
data needed to close it — do not estimate.`;

/**
 * Traceability table footer — every specialist closes with this mandate.
 * The chief aggregates specialists' outputs; the table is the audit trail.
 */
export const TRACEABILITY_TABLE_FOOTER = `\
Close every analysis with a tool-call traceability table (mandatory, one \
row per invocation):

  | # | Tool | Key Inputs | Output |

Every number in the body must have a corresponding row in this table. The \
chief uses these tables to assemble the final memo's audit appendix.`;

/**
 * Convenience: the four fragments joined with section markers, for
 * specialists that prefer to interpolate one block instead of four.
 */
export const SPECIALIST_PREAMBLE = [
  "## Operating mode (delegated)",
  "",
  SPECIALIST_OPERATING_MODE,
  "",
  "## Tool-calling convention",
  "",
  TOOL_CALLING_CONVENTION,
  "",
  "## Quality gate",
  "",
  QUALITY_GATE_NO_LLM_ARITHMETIC,
].join("\n");
