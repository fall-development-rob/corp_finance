# ADR-035: Hierarchical Chief-Specialist Dispatch Pattern

## Status: Accepted

## Date: 2026-05-09

## Deciders

- Robert Fall
- CFA Agent platform engineering

## Tags

`dispatch`, `hierarchical`, `chief-analyst`, `specialist`, `delegation`, `queen-led`, `phase-31`

## Context

The CFA analyst stack models a real institutional research desk: a chief analyst who decomposes a complex client question into domain sub-problems, routes each sub-problem to the relevant specialist (equity, credit, fixed-income, derivatives, quant-risk, macro, private-markets, esg-regulatory), receives their structured outputs, and synthesises a final deliverable. This is not an aesthetic choice — it reflects the real constraint that no single model context window should hold all 623 tool definitions simultaneously while also maintaining a coherent long narrative (an IC memo is 3,000–8,000 words), and that tool subsets constrained to a specialist domain reduce hallucinated tool calls.

Phase 30 attempted to implement this pattern via Claude Code's cfa-* subagent class. The subagent class cannot invoke tools (ADR-031, `feedback_cfa_agents_cannot_invoke_tools.md`). But the hierarchical pattern itself — chief decomposes, specialists execute, chief aggregates — is the correct architecture. The harness (ADR-031) re-implements it using real Messages-API loops at each level.

The pattern is inspired by ruflo's Queen-led hierarchical dispatch, where a coordinator agent decomposes a task into subtasks, spawns specialist agents, and aggregates their outputs. The CFA harness is a TypeScript implementation of this pattern bounded to depth 1 in Phase 31 (chief delegates to specialists; specialists do not delegate further).

A key design question is whether specialist dispatch should be implemented as: (a) parallel Messages-API calls to multiple specialists simultaneously, (b) sequential specialist calls driven by the chief's tool-use decisions, or (c) chief-determines-then-dispatches in a single coordination turn. The chosen approach is (b): the chief's agent loop yields delegation tool calls (e.g., `delegate_to_derivatives_analyst`) which the tool router resolves into nested Messages-API calls to the specialist agent. This keeps the chief's context window focused on coordination while each specialist runs a full, independent agent loop with access only to its domain tool subset.

## Decision

Implement a two-level hierarchical dispatch in `packages/agents/`:

### Level 1: Chief analyst

`packages/agents/chief-analyst.ts` defines a chief agent with:
- A system prompt that frames the chief's role: decompose complex financial analysis requests, delegate to domain specialists, synthesise structured specialist outputs into a final deliverable.
- Access to all 623 MCP tools (for direct invocation when a task is clearly within a single domain) plus the delegation pseudo-tools.
- Delegation pseudo-tools: `delegate_to_derivatives_analyst`, `delegate_to_equity_analyst`, etc. — 8 pseudo-tools corresponding to the 8 specialists. These are not MCP tools; they are harness-internal tools handled by the tool router before any MCP call is made.

Pseudo-tool schema example:
```typescript
const delegationTools: CanonicalTool[] = [
  {
    name: "delegate_to_derivatives_analyst",
    description:
      "Delegate an options, futures, swaps, or structured product sub-task to " +
      "the derivatives specialist. Returns the specialist's structured analysis " +
      "including all tool invocations and numerical results.",
    input_schema: {
      type: "object",
      properties: {
        task: {
          type: "string",
          description: "The sub-task description, including all relevant inputs.",
        },
        context: {
          type: "string",
          description: "Any relevant context from the broader analysis.",
        },
      },
      required: ["task"],
    },
  },
  // ... 7 more delegation tools
];
```

### Level 2: Specialist agents

`packages/agents/specialists/` contains 8 specialist agent definitions. Each defines:
- A domain-specific system prompt (e.g., derivatives specialist: `"You are a CFA-level derivatives analyst. You have access to options pricing, volatility surface, and structured product tools..."`).
- A constrained tool subset: only the MCP tools relevant to the specialist's domain are passed to the model.
- No delegation tools: specialists do not call other specialists in Phase 31 (max recursion depth 1).

Example tool subset for the derivatives specialist:
```typescript
export const derivativesToolSubset = [
  "option_pricer",
  "implied_volatility",
  "implied_vol_surface",
  "sabr_calibration",
  "bond_pricer",
  "convertible_bond_pricing",
  "exotic_product_pricing",
  "cds_pricing",
  "interest_rate_swap",
  "currency_swap",
  "option_strategy",
  "hedge_effectiveness",
  "black_litterman",  // when derivatives context requires portfolio-level hedging
];
```

### Dispatch flow

`packages/agents/delegate.ts` implements the specialist dispatch:

1. Chief's agent loop yields a `delegate_to_<specialist>` pseudo-tool call.
2. `delegate.ts` intercepts this in the tool router (before MCP dispatch).
3. `delegate.ts` instantiates the specialist's agent loop with the task as user content and the specialist's tool subset.
4. Specialist loop runs to `end_turn`, accumulating tool calls and results.
5. Specialist returns a `SpecialistResult`: `{ summary: string, tool_calls: ToolInvocation[], key_metrics: Record<string, Decimal> }`.
6. `delegate.ts` serialises the result and returns it as a `tool_result` to the chief's loop.
7. Chief incorporates the specialist result and continues.

### Max recursion depth

Phase 31 enforces `maxDepth = 1`. The depth counter is an argument to `agentLoop()`:

```typescript
async function* agentLoop(
  agent: AgentDef,
  messages: ProviderMessage[],
  depth: number = 0
): AsyncGenerator<AgentEvent> {
  if (depth > MAX_DISPATCH_DEPTH) {
    throw new HarnessError("MAX_DISPATCH_DEPTH exceeded");
  }
  // ...
}
```

`MAX_DISPATCH_DEPTH = 1` in Phase 31. Specialists called at depth 1 see `maxDepth - depth = 0` and cannot delegate further. Phase 32+ may increase this limit if multi-level specialist chaining is required (e.g., a macro analyst delegating a PPP sub-calculation to a derivatives specialist).

### Tool subset enforcement

Specialists receive only their domain's tool definitions in the `tools` array passed to `provider.chat()`. The model cannot invoke out-of-domain tools because they are not visible to it. This reduces hallucinated tool calls and keeps specialist context windows small enough to hold the full specialist system prompt plus a meaningful analysis.

## Consequences

### Positive

- The chief's context is focused on orchestration: it sees delegation tools and a high-level task, not 623 raw MCP tool signatures that would crowd the context.
- Specialist context is focused on execution: each specialist sees only 10–25 tools relevant to its domain, reducing the probability of tool selection errors.
- The Chaco Minerals acceptance test maps cleanly to this architecture: chief delegates warrant grid → derivatives specialist (invokes `option_pricer` ×20), delegates country risk → macro specialist (invokes `country_risk_premium` ×1), aggregates results into the IC memo.
- Ruflo's proven Queen-led pattern is the architectural blueprint; the harness is a TypeScript implementation of an established design.
- Max depth 1 keeps the first implementation simple and testable; the recursive capability is built in from the start (depth counter) but not exposed until Phase 32.

### Negative

- Delegation pseudo-tools add latency: each specialist invocation is a full Messages-API round trip (chief → harness → specialist loop → MCP tools → specialist end_turn → chief). A complex memo with 4 specialist delegations has 5 Messages-API round trips minimum.
- The chief's tool roster (623 MCP tools + 8 delegation tools) is a large context payload for the system prompt. If the model context window is constrained by a very long prompt, the 623-tool list may need to be summarised or paginated.
- Specialist result serialisation (structured JSON returned as a tool result string) is a lossy boundary: rich tool invocation trees are flattened to a summary for the chief's consumption. Full specialist traces are written to the audit log but not re-fed to the chief.
- The delegation pseudo-tools do not map to any MCP server; they are harness-internal. If the provider abstraction (ADR-033) is extended, the pseudo-tool dispatch must be preserved in each provider adapter's tool result handling.

### Neutral

- The 8 specialist tool subsets are defined in code (`packages/agents/specialists/`) rather than in agent `.md` frontmatter. This is consistent with ADR-037 (agent registry as code) and means the subsets are typed and version-controlled alongside the dispatch logic.
- In Phase 31, the chief may also directly invoke MCP tools without delegation (for single-domain tasks). The delegation pattern is an option, not a requirement; the chief uses judgment about whether to delegate or directly invoke.

## Alternatives Considered

**Flat dispatch (chief invokes all tools directly, no specialists)** — The chief sees all 623 tools and invokes them directly. Simpler: no delegation pseudo-tools, no nested agent loops. Rejected because: (1) 623 tool definitions in a single Messages-API call is a large context payload that may degrade model attention on the most relevant tools; (2) the institutional research desk model that motivates the CFA platform uses specialist roles by design; (3) the Chaco test requires routing warrant pricing to derivatives and country risk to macro — flat dispatch blurs that boundary.

**Parallel specialist dispatch (all delegations fire simultaneously)** — Chief determines all delegations in one turn, fires all specialist loops in parallel (Promise.all), aggregates. Faster than sequential. Rejected for Phase 31 because: (1) parallel specialist loops complicate the audit chain (tool call ordering becomes non-deterministic); (2) the chief cannot use an earlier specialist's result to inform a later delegation in parallel mode; (3) sequential dispatch is easier to debug and reason about in Wave 2. Phase 32 can add parallel dispatch as an option.

**Single-agent with large context (no hierarchy)** — One agent, all 623 tools, full memo in one loop. This is how the general-purpose subagent proved tool invocation works. Rejected as the production architecture because it does not implement specialist role constraints, mixes domain concerns in a single context, and does not scale to multi-topic memos where specialist independence matters for accuracy.

**Ruflo swarm (spawn ruflo agents from the harness)** — Ruflo provides a Queen-led swarm runtime. Using ruflo as the dispatch substrate would avoid re-implementing the hierarchical pattern. Rejected: ruflo is a Claude Code plugin, not a TypeScript library; its agent instances are Claude Code subagents, which are exactly the broken dispatch class (ADR-031). The harness must own the dispatch loop to guarantee `tool_uses ≥ 21`.

## References

- Master plan: `docs/plans/phase-31-harness.md`
- ADR-031: Custom dispatch harness (establishes why the Claude Code subagent dispatch is replaced)
- ADR-032: TypeScript + Anthropic SDK (the `agentLoop` async generator lives in `packages/core`)
- ADR-033: Multi-provider abstraction (delegation pseudo-tools must be handled before provider dispatch)
- ADR-034: MCP plugin reuse (specialists invoke MCP tools from the existing plugin servers)
- ADR-037: Agent registry as code (specialist definitions, including tool subsets, live in `packages/agents/`)
- Ruflo Queen-led pattern: referenced in Phase 31 master plan under Inspirations
- Phase 30 commit `d548509`: agent body rewrites that documented the specialist structure this ADR now formalises in code
