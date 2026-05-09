# ADR-031: Custom Dispatch Harness on @anthropic-ai/sdk

## Status: Accepted

## Date: 2026-05-09

## Deciders

- Robert Fall
- CFA Agent platform engineering

## Tags

`harness`, `dispatch`, `anthropic-sdk`, `subagent`, `tool-use`, `phase-31`

## Context

Phase 29 and Phase 30 together completed what should have been the final plumbing step toward a fully capable CFA analyst stack. Phase 29 extracted `corp-finance-core` to crates.io 1.1.0 and ported 227 tools to a WASM plugin (`plugin:cfa-core:cfa-core`). Phase 30 migrated nine shell scripts to MCP tools, wired all four plugin servers (`plugin:cfa-core:cfa-core`, `plugin:cfa-data:data`, `plugin:cfa-pro:fmp-market-data`, `plugin:cfa-pro:vendor`), updated all 9 cfa-* agent frontmatter allowlists to plugin-namespaced wildcards (commit `6c4bd78`), and rewrote every agent body with 308 explicit plugin-prefixed tool references and invocation conventions (commit `d548509`).

Despite exhausting every configuration surface exposed by the Claude Code subagent format, cfa-* subagent dispatches return `tool_uses: 0` on minimal single-tool probes. The agent emits syntactically correct `tool_use` JSON as text but does not invoke the tool. This is not a configuration error in any individual file — it is an intrinsic property of the cfa-* subagent class in Claude Code's runtime.

The negative result is documented in `feedback_cfa_agents_cannot_invoke_tools.md` with controlled evidence: the same prompt (warrant grid and country risk calculation for Chaco Minerals) dispatched to a general-purpose subagent returns `tool_uses: 22` with real rust_decimal-precision results and a complete IC memo. The cfa-derivatives-analyst on a minimal single-tool probe returns `tool_uses: 0`. The control and treatment share identical tool registration, identical MCP plugin connection status (`✓ Connected`), and identical prompt structure. The only variable is the agent type.

Separately, `/tmp/mcp-chaco-demo.mjs` proves that the four plugin MCP servers accept JSON-RPC tool calls correctly: 21 real `tools/call` invocations were completed with numerically exact results (per-warrant price C$0.0208 at C$0.10 strike, 80% volatility) using a custom Node.js client that bypasses Claude Code entirely. The MCP server layer is sound. The Claude Code subagent dispatch layer is the broken link.

The pattern of exhausted fixes is the signal: plugin install, allowlist update, body rewrite, session restart, type change from coordinator to analyst, minimal-probe reduction — none unblocked the dispatch. Further iteration on cfa-* agent `.md` files will not produce a different result. The correct response is to own the dispatch loop.

## Decision

Build a custom dispatch harness in TypeScript on `@anthropic-ai/sdk`. The harness lives in `packages/` alongside the existing MCP servers and owns the full dispatch loop end-to-end:

1. **`packages/core/agent-loop.ts`** — async generator wrapping the Anthropic Messages API. Each turn: call `messages.create` with current context + tool definitions; yield `tool_use` blocks to the tool router; append `tool_result` blocks; loop until `stop_reason === "end_turn"`.

2. **`packages/core/tool-router.ts`** — translates `tool_use` blocks from the model into MCP `tools/call` JSON-RPC requests, dispatches to the appropriate plugin server via `packages/mcp-client/stdio.ts`, and returns the result as a `tool_result` content block.

3. **`packages/mcp-client/stdio.ts`** — JSON-RPC 2.0 client over stdio to each of the four plugin MCP servers. Performs the MCP handshake (`initialize` → `initialized`), then exposes `listTools()` and `callTool(name, input)`.

4. **`packages/agents/chief-analyst.ts`** — first agent definition: system prompt + full 623-tool allowlist + delegation rules.

5. **`packages/cli/run.ts`** — `cfa-harness run --agent <name> --prompt @<file> --output <file>` entry point.

The acceptance criterion is the Chaco Minerals memo: `tool_uses ≥ 21`, numerically matching canonical values, all 9 IC memo sections present, audit log written to `out/chaco-memo.audit.json`.

The decision to build rather than continue iterating on Claude Code subagents is final. The cfa-* `.md` agent files remain in the repository for historical Claude Code use but the harness does not read or parse them.

## Consequences

### Positive

- The dispatch loop is fully owned: tool invocation behavior is deterministic, testable, and not subject to undocumented Claude Code runtime constraints.
- The Chaco acceptance test (`tool_uses ≥ 21`) is a CI-runnable integration gate that makes regressions immediately visible.
- The harness is provider-neutral by construction (see ADR-033): swapping from Anthropic to OpenAI or Gemini requires only a provider adapter, not a reimplementation of the dispatch loop.
- MCP servers remain unmodified — the four plugin servers that Phase 29/30 built are reused directly (ADR-034).
- The `/tmp/mcp-chaco-demo.mjs` proof-of-concept becomes a first-class test rather than a throwaway script.

### Negative

- The harness is ~2100 LOC of new TypeScript across four waves (~10 working days). This is real investment that would have been unnecessary if the Claude Code subagent dispatch worked as expected.
- The harness introduces a second CLI entry point alongside `claude`. Users running the CFA analyst flow must choose between `claude` (IDE, no tool invocation) and `cfa-harness` (CLI/API, full tool invocation) until the harness exposes an IDE integration.
- The 9 cfa-* agent `.md` files are now partially obsolete: their system prompts and tool subsets will be maintained in `packages/agents/registry.ts` (ADR-037) instead, creating a potential sync drift if both surfaces are edited independently.

### Neutral

- The Claude Code IDE experience is unchanged. Users continue using `claude` for code editing; `cfa-harness` is the analyst execution runtime. These are complementary, not competing.
- The harness does not replace ruflo or AgentDB. Ruflo's hierarchical Queen-led dispatch pattern is the architectural model (see ADR-035); the harness is a TypeScript implementation of that pattern for the CFA domain.

## Alternatives Considered

**Continue iterating on cfa-* agent .md files** — Rejected. Six distinct fix attempts (plugin install, allowlist update, body rewrite, session restart, type change, minimal probe) produced zero improvement. The `feedback_cfa_agents_cannot_invoke_tools.md` note records these attempts as exhaustive. Further iteration produces delay without results.

**Use the general-purpose subagent with a CFA system prompt** — The general-purpose agent dispatches correctly but cannot be constrained to a specialist tool subset, cannot be given a persistent CFA-specific system prompt via frontmatter in the way the harness achieves via `registry.ts`, and cannot implement the hierarchical chief → specialist delegation pattern (ADR-035). It is a proof-of-concept medium, not a production analyst runtime.

**Fork Claude Code** — Rejected. The Claude Code runtime is not open-source at the relevant layer. Patching it would require reverse engineering proprietary code and maintaining the fork against upstream. The harness achieves the same outcome with standard, supported SDK interfaces.

**Wait for a Claude Code runtime fix** — Rejected. There is no known timeline for a fix, and the bug may be intentional (the cfa-* agent class may never be intended to support plugin-namespaced tool invocation). The harness provides a durable solution regardless of upstream changes.

## References

- Master plan: `docs/plans/phase-31-harness.md`
- Memory note (acceptance evidence): `~/.claude/projects/-home-robert-cfa-agent/memory/feedback_cfa_agents_cannot_invoke_tools.md`
- Proof-of-concept MCP client: `/tmp/mcp-chaco-demo.mjs` (21 real `tools/call` invocations)
- ADR-032: TypeScript + Anthropic SDK stack choice
- ADR-033: Multi-provider abstraction layer
- ADR-034: MCP plugin reuse strategy
- ADR-035: Hierarchical chief-specialist dispatch pattern
- ADR-036: Audit chain via corp-finance-core audit module
- ADR-037: Agent registry as TypeScript code
- Phase 30 commit `d548509`: agent body rewrites (308 plugin-prefix references)
- Phase 30 commit `6c4bd78`: allowlist updates to plugin-namespaced wildcards
