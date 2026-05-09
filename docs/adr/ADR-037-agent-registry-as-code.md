# ADR-037: Agent Definitions as TypeScript Code in packages/agents/registry.ts

## Status: Accepted

## Date: 2026-05-09

## Deciders

- Robert Fall
- CFA Agent platform engineering

## Tags

`agent-registry`, `code-defined-agents`, `system-prompts`, `frontmatter`, `typescript`, `phase-31`

## Context

The nine CFA analyst agents (`cfa-chief-analyst`, `cfa-equity-analyst`, `cfa-credit-analyst`, `cfa-fixed-income-analyst`, `cfa-derivatives-analyst`, `cfa-quant-risk-analyst`, `cfa-macro-analyst`, `cfa-private-markets-analyst`, `cfa-esg-regulatory-analyst`) are currently defined as `.md` files in `.claude/agents/cfa/`. Each file carries YAML frontmatter with `name`, `description`, `model`, `tools`, and `type` fields, followed by a markdown body that is the agent's system prompt.

This format was chosen to match Claude Code's subagent registration convention. Phase 29 updated the `tools:` allowlists to plugin-namespaced wildcards (commit `6c4bd78`). Phase 30 rewrote all nine agent bodies with 308 explicit plugin-prefixed tool references and invocation conventions (commit `d548509`). Despite this investment, the `.md` frontmatter format proved unable to deliver its central promise: cfa-* subagents cannot invoke MCP tools (ADR-031).

The rewrite also exposed a structural weakness of the format: when tool prefix conventions changed (from `mcp__cfa-tools_*` to `mcp__plugin_cfa-core_cfa-core__*`), all 9 files required a simultaneous update to 308 references. A rename is a single-point change in code; in a distributed frontmatter format it is a 308-site update that cannot be enforced by a compiler. The `6c4bd78` and `d548509` commits exist precisely because there was no compile-time check that the allowlists and body references were consistent.

The harness (ADR-031) needs agent definitions — system prompts, tool subsets, and delegation rules — at runtime. These definitions must be type-checked, version-controlled alongside the dispatch code that consumes them, and refactorable with standard TypeScript tooling. The `.md` frontmatter format satisfies none of these requirements.

A code-defined agent registry is the established pattern in the broader agentic systems landscape: LangChain agent configurations, AutoGen agent specs, and the ruvnet/ruflo plugin-as-agent pattern all define agent metadata in code (Python dataclasses, TypeScript objects, or Rust structs), not markdown frontmatter. The harness adopts this pattern.

## Decision

Agent definitions live as TypeScript code in `packages/agents/registry.ts`. The `AgentDef` interface captures everything the harness dispatch loop needs:

```typescript
// packages/agents/registry.ts

export interface AgentDef {
  /** Unique identifier. Used in audit records, CLI --agent flag, delegation tool names. */
  id: string;

  /** Display name for logging and CLI output. */
  displayName: string;

  /** System prompt passed to the provider on every turn. */
  systemPrompt: string;

  /**
   * Tool name allowlist for this agent.
   * If null, the agent has access to all 623 MCP tools.
   * Delegation pseudo-tool names (e.g., "delegate_to_derivatives_analyst") are
   * automatically added by the dispatch layer; do not include them here.
   */
  toolAllowlist: string[] | null;

  /**
   * Agents this agent is permitted to delegate to.
   * Empty array = no delegation (leaf specialist).
   * ["*"] = delegate to any registered agent (chief only).
   */
  canDelegateTo: string[];

  /**
   * Maximum token budget for a single agent loop execution.
   * Prevents runaway specialist loops from exhausting the API quota.
   */
  maxTokens: number;
}

// Registry: map from agent id to definition
export const AGENT_REGISTRY: Record<string, AgentDef> = {
  "chief-analyst": {
    id: "chief-analyst",
    displayName: "CFA Chief Analyst",
    systemPrompt: CHIEF_SYSTEM_PROMPT,  // imported from ./prompts/chief.ts
    toolAllowlist: null,                // access to all 623 tools
    canDelegateTo: ["*"],
    maxTokens: 16384,
  },
  "derivatives-analyst": {
    id: "derivatives-analyst",
    displayName: "CFA Derivatives Analyst",
    systemPrompt: DERIVATIVES_SYSTEM_PROMPT,
    toolAllowlist: DERIVATIVES_TOOL_SUBSET,  // imported from ./subsets/derivatives.ts
    canDelegateTo: [],
    maxTokens: 8192,
  },
  // ... 7 more specialists
};

export function getAgent(id: string): AgentDef {
  const agent = AGENT_REGISTRY[id];
  if (!agent) throw new Error(`Unknown agent: ${id}`);
  return agent;
}
```

### System prompts as typed constants

System prompts live in `packages/agents/prompts/` as TypeScript template literals:

```typescript
// packages/agents/prompts/chief.ts

export const CHIEF_SYSTEM_PROMPT = `
You are the CFA Chief Analyst, the lead financial analyst on this platform.
Your role is to:
1. Decompose complex financial analysis requests into domain sub-problems.
2. Delegate sub-problems to domain specialists using the delegation tools provided.
3. Invoke financial computation tools directly for single-domain tasks.
4. Synthesise specialist outputs and your own analysis into a coherent, institutionally formatted deliverable.

You have access to all 623 financial tools across four MCP servers:
- corp-finance-core: 227 financial computation tools (options, fixed income, credit, portfolio, etc.)
- cfa-data: 129 market and economic data tools (FRED, EDGAR, Yahoo Finance, World Bank, etc.)
- fmp-market-data: 180 Financial Modelling Prep tools (quotes, statements, ratios, etc.)
- vendor: 87 premium vendor tools (FactSet, LSEG, Moody's, Morningstar, PitchBook, S&P)

All numerical results must use the precision provided by the computation tools.
Do not approximate or estimate values that can be computed exactly.
` as const;
```

Using TypeScript `const` assertions on the system prompt strings provides:
- Compile-time detection of unterminated template literals.
- IDE hover showing the full prompt text.
- Refactoring tools that can find and rename string references.

### Tool subsets as typed arrays

```typescript
// packages/agents/subsets/derivatives.ts

export const DERIVATIVES_TOOL_SUBSET: string[] = [
  "option_pricer",
  "implied_volatility",
  "implied_vol_surface",
  "sabr_calibration",
  "convertible_bond_pricing",
  "convertible_bond_analysis",
  "cds_pricing",
  "interest_rate_swap",
  "currency_swap",
  "option_strategy",
  "exotic_product_pricing",
  "hedge_effectiveness",
  "forward_pricer",
  "futures_basis_analysis",
  "structured_note_pricing",
] as const;
```

The dispatch layer validates tool subsets at startup: it calls `tools/list` on all four MCP servers, builds the full 623-name set, and asserts that every name in every tool allowlist is in the full set. Unknown tool names fail startup with an error that names the misspelled tool. This is a compile-analogue check at runtime that the `.md` frontmatter format never provided.

### The .md files remain but the harness does not consume them

The 9 files in `.claude/agents/cfa/` remain in the repository. They continue to serve as the Claude Code agent definitions for users who invoke CFA analysts via the Claude Code IDE (even though tool invocation does not work in that path). They are not deleted because:
1. They document the agent system prompts in a format that Claude Code may eventually support correctly.
2. They are the historical record of the Phase 29/30 configuration work.
3. Removing them would break the Claude Code agent picker for users who have not switched to `cfa-harness`.

The harness reads from `AGENT_REGISTRY` only. If the `.md` files and the registry diverge (e.g., a system prompt is updated in one but not the other), the harness is the authoritative source for harness dispatches. A `scripts/check-agent-sync.ts` script is planned (Phase 32) to warn when significant drift is detected.

### CLI integration

The `--agent` flag in `cfa-harness run` accepts any key in `AGENT_REGISTRY`:

```bash
cfa-harness run --agent chief-analyst --prompt docs/examples/chaco-minerals-deal.md
cfa-harness run --agent derivatives-analyst --prompt docs/examples/warrant-grid.md
cfa-harness list-agents  # prints all keys and displayNames from AGENT_REGISTRY
```

## Consequences

### Positive

- Tool allowlist changes are single-point edits in `packages/agents/subsets/<specialist>.ts`. No more 308-site updates across 9 markdown files.
- TypeScript compilation catches invalid `AgentDef` shapes (missing required fields, wrong types) before runtime.
- Startup validation catches misspelled tool names in allowlists before any Messages-API call is made.
- System prompts are first-class TypeScript strings: they can be unit-tested (e.g., assert that the chief prompt includes a delegation instruction), interpolated with runtime values (e.g., today's date), and syntax-highlighted in IDEs.
- The registry is the single source of truth for the harness. Agent definitions, tool subsets, delegation rules, and token budgets are co-located in one place rather than scattered across 9 markdown files.
- Refactoring tools (TypeScript rename, find-all-references) work on agent ids, tool names, and system prompt constants.

### Negative

- The 9 `.md` agent files and the `AGENT_REGISTRY` are now two partially-overlapping definitions of the same agents. If a system prompt is updated in one location, it must be updated in the other manually until the `check-agent-sync.ts` script is shipped in Phase 32.
- System prompts in TypeScript template literals lose the markdown formatting conveniences of the `.md` format (e.g., GitHub rendering, easy preview). Developers must use `console.log` or a helper script to inspect the rendered system prompt.
- The `AgentDef.toolAllowlist` is a list of bare tool names (e.g., `"option_pricer"`), not plugin-namespaced names (e.g., `"mcp__plugin_cfa-core_cfa-core__option_pricer"`). The dispatch layer maps bare names to plugin-namespaced names at the MCP wire boundary. This mapping must be maintained correctly; a mismatch produces a silent routing failure.

### Neutral

- The `canDelegateTo` field anticipates Phase 32+ multi-level delegation (e.g., a macro analyst delegating a sub-calculation). In Phase 31, the chief uses `["*"]` and all specialists use `[]`. The field is present but not fully exercised in Phase 31.
- The `maxTokens` field per agent allows the chief and complex specialists to have larger token budgets than simple lookup specialists. This is a finer-grained control than the single `max_tokens` passed to `messages.create` today.

## Alternatives Considered

**Keep the .md frontmatter format, parse it in the harness** — The harness could read `.claude/agents/cfa/*.md`, parse the YAML frontmatter, and use the `tools:` field as the allowlist. This preserves the single source of truth. Rejected because: (1) the YAML frontmatter `tools:` field uses glob patterns (`mcp__plugin_cfa-core_cfa-core__*`), not bare tool names — the harness would need to expand globs against the live `tools/list` response, adding complexity; (2) the `.md` body is the system prompt, but it includes markdown formatting (headers, bold, bullet lists) that is not appropriate as a raw system prompt string; (3) the phase 29/30 experience showed that the `.md` format is a maintenance burden when tool names change.

**JSON or YAML configuration files** — `packages/agents/registry.json` or `registry.yaml`. More portable than TypeScript. Rejected because: (1) no type checking without a JSON Schema validator; (2) no IDE support for autocomplete on field names; (3) system prompts as JSON strings lose all readability; (4) the existing workspace is TypeScript — there is no motivation to introduce a second configuration language.

**Database-backed registry (AgentDB or SQLite)** — Agent definitions stored in AgentDB (the ruflo memory system). Enables runtime updates without code deploys. Rejected for Phase 31: the agent registry changes infrequently (new agent types are a Phase-level event, not a session-level event); the complexity of a database-backed registry is disproportionate to the use case in Phase 31. Phase 32+ may add a runtime registry API for dynamic agent configuration.

**One file per agent (not a unified registry)** — `packages/agents/chief-analyst.ts`, `packages/agents/derivatives-analyst.ts` etc., each exporting an `AgentDef`. More modular. Rejected for Phase 31 because: (1) a unified `AGENT_REGISTRY` map is the natural lookup structure for the CLI `--agent` flag and the delegation dispatch; (2) importing from 9 separate files at the top of `delegate.ts` is noisier than a single import from `registry.ts`. Phase 32 can split if the registry file grows large.

## References

- Master plan: `docs/plans/phase-31-harness.md`
- ADR-031: Custom dispatch harness (motivates the code-defined registry as the harness needs typed agent definitions)
- ADR-032: TypeScript + Anthropic SDK (the TypeScript type system is the primary benefit of this approach)
- ADR-035: Hierarchical chief-specialist dispatch (the `canDelegateTo` and `toolAllowlist` fields implement the dispatch pattern)
- ADR-036: Audit chain (the `agent_id` in audit records corresponds to `AgentDef.id`)
- Phase 30 commit `6c4bd78`: allowlist updates (the 308-reference maintenance problem this ADR resolves)
- Phase 30 commit `d548509`: agent body rewrites (the system prompt content that migrates to `packages/agents/prompts/`)
- `.claude/agents/cfa/` — existing 9 agent .md files (remain in repo; harness does not consume them)
