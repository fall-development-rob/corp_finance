# ADR-032: TypeScript (Strict) on Node.js ≥ 20 with @anthropic-ai/sdk ≥ 0.30

## Status: Accepted

## Date: 2026-05-09

## Deciders

- Robert Fall
- CFA Agent platform engineering

## Tags

`typescript`, `node`, `anthropic-sdk`, `language`, `runtime`, `phase-31`

## Context

The custom harness (ADR-031) requires a language and runtime for the dispatch loop, CLI, and MCP client packages. The choice determines:

- How the harness interoperates with the four existing TypeScript MCP servers (`packages/mcp-server`, `packages/fmp-mcp-server`, `packages/data-mcp-server`, `packages/vendor-mcp-server`) and the `@modelcontextprotocol/sdk` already used in those servers.
- How the harness calls the Anthropic Messages API to implement the agent loop.
- How the harness is distributed to users: npm package, compiled binary, or script.
- Whether the financial compute logic (all in `corp-finance-core` Rust, exposed via WASM plugin) introduces a language impedance mismatch.

The existing codebase is a TypeScript-primary workspace. `packages/` contains four MCP servers, all written in TypeScript with `tsc` builds. The WASM plugin (`plugins/cfa-core`) is TypeScript. The `@modelcontextprotocol/sdk` npm package is the established MCP client transport. The Anthropic TypeScript SDK (`@anthropic-ai/sdk`) is the first-party client for the Messages API, actively maintained by Anthropic, and generates fully typed request and response interfaces.

The dispatch loop is IO-bound orchestration: it awaits model responses, routes tool calls to MCP servers over stdio, and aggregates results. It does not perform financial computation (that is delegated to `corp-finance-core` via MCP). There is no compute-intensive inner loop that would benefit from a systems language.

## Decision

The harness is written in **TypeScript strict mode** targeting **Node.js ≥ 20 LTS**, using **`@anthropic-ai/sdk` ≥ 0.30** as the Anthropic Messages API client.

### TypeScript strict mode

All `packages/` are compiled with `"strict": true` in `tsconfig.json`. This includes `noImplicitAny`, `strictNullChecks`, `strictFunctionTypes`, and `strictPropertyInitialization`. The agent loop, tool router, MCP client, and provider abstraction are all fully typed. Tool input and output schemas are typed via the Anthropic SDK's `Tool` and `ToolResultBlockParam` interfaces.

### Node.js ≥ 20 LTS

Node 20 LTS (2023-10-24 → April 2026) and Node 22 LTS (2025-10-28 → April 2028) are both targets. `engines` in `package.json`:

```json
{
  "engines": { "node": ">=20.0.0" }
}
```

Node 20 provides native `fetch`, `AbortController`, `AsyncLocalStorage` (for audit context propagation), and the stable `--experimental-vm-modules` flag that vitest requires. Node 22 adds `require(esm)` which simplifies the `@modelcontextprotocol/sdk` ESM import in a CJS context.

### @anthropic-ai/sdk ≥ 0.30

Pinned to `^0.30.0` in `package.json`. Version 0.30 introduced stable streaming helpers, the `messages.stream()` convenience method, and the `MessageStreamEvent` typed union used in the async generator loop. The SDK generates typed `Message`, `ContentBlock`, `ToolUseBlock`, and `ToolResultBlockParam` interfaces that the tool router consumes directly.

Example agent loop call site (illustrative shape, not final code):

```typescript
import Anthropic from "@anthropic-ai/sdk";

const client = new Anthropic();

async function* agentLoop(
  systemPrompt: string,
  tools: Anthropic.Tool[],
  messages: Anthropic.MessageParam[]
): AsyncGenerator<Anthropic.ContentBlock> {
  while (true) {
    const response = await client.messages.create({
      model: "claude-sonnet-4-6",
      max_tokens: 8192,
      system: systemPrompt,
      tools,
      messages,
    });
    for (const block of response.content) {
      yield block;
    }
    if (response.stop_reason === "end_turn") break;
    // append tool_result blocks and continue
    messages = appendToolResults(messages, response.content);
  }
}
```

### npm workspace integration

The harness packages share the existing npm workspace root at `cfa_agent/`. New packages (`packages/core`, `packages/mcp-client`, `packages/agents`, `packages/cli`, `packages/audit`, `packages/memory`) are added to the workspace `packages` array. The build script is `tsc -b` from the workspace root, consistent with the existing `npm run build` convention.

### Test runner: vitest

Vitest is the project-wide test runner (used by existing MCP servers). All harness tests use `vitest`. The acceptance test (`tests/chaco-acceptance.test.ts`) is an integration test that spawns the four plugin MCP servers via `stdio`, runs a full agent loop, and asserts `tool_uses ≥ 21`.

## Consequences

### Positive

- Zero language impedance mismatch: harness, MCP servers, and MCP SDK are all TypeScript; imports work without FFI.
- Anthropic's first-party SDK provides typed `Message`, `Tool`, and `ToolUseBlock` interfaces; no hand-written API client is needed.
- The existing `@modelcontextprotocol/sdk` in `packages/mcp-server` is reused in `packages/mcp-client` without new dependencies.
- TypeScript strict mode catches missing null checks and incorrect tool input shapes at compile time, not at runtime during a live analyst session.
- The harness is distributable as an npm package; users install with `npm install -g cfa-harness` and run `cfa-harness run`.

### Negative

- TypeScript adds a compile step; developers must run `tsc` or `tsc --watch` before testing. The existing `npm run build` convention absorbs this but adds ~10s to the cold-start CI cycle.
- `@anthropic-ai/sdk` is a first-party dependency that must be tracked for breaking changes. Pinning to `^0.30.0` gives patch and minor updates but requires a deliberate bump for major versions.
- The SDK does not abstract provider differences: `@anthropic-ai/sdk` is Anthropic-specific. Multi-provider support (ADR-033) requires a hand-written abstraction layer in `packages/core/providers/`, which the SDK does not provide.

### Neutral

- The financial compute layer (Rust, `corp-finance-core`) is not affected by this choice. The harness calls compute via MCP tool calls over stdio; the language boundary is the JSON-RPC protocol, not an FFI.
- Deno or Bun were not evaluated: the existing workspace uses Node.js and there is no motivation to introduce a second runtime.

## Alternatives Considered

**Rust for the dispatch loop** — Rust is consistent with `corp-finance-core` and would produce a single-binary CLI without a Node.js runtime dependency. Rejected because: (1) the dispatch loop is IO-bound async orchestration, not compute; (2) the `@modelcontextprotocol/sdk` is TypeScript-first and there is no stable Rust MCP client; (3) the existing MCP servers are TypeScript — sharing types and utilities across the language boundary would require generated bindings, adding substantial complexity for no compute benefit. The compute stays in Rust (`corp-finance-core`); the orchestration is TypeScript.

**Python with the Anthropic Python SDK** — The Anthropic Python SDK is feature-equivalent to the TypeScript SDK. Rejected because: (1) the entire existing codebase is TypeScript; (2) `@modelcontextprotocol/sdk` has no Python equivalent at the same maturity level; (3) introducing a second language in `packages/` would fragment the workspace.

**Go** — Go's stdlib concurrency model maps cleanly to the async generator pattern. Rejected: no Go code in the existing workspace, no MCP SDK, no Anthropic Go SDK at parity with the TypeScript SDK.

**Plain JavaScript (no TypeScript)** — Removes the compile step. Rejected: the existing MCP servers are TypeScript; tool schema types from the Anthropic SDK are the primary safety net for the tool-call dispatch loop; losing `strict` type checking on `ToolUseBlock.input` shapes would introduce silent runtime errors in a financial analysis context.

## References

- Master plan: `docs/plans/phase-31-harness.md`
- ADR-031: Custom dispatch harness (motivates this language choice)
- ADR-033: Multi-provider abstraction (identifies where the SDK's Anthropic-specific shape is factored out)
- ADR-034: MCP plugin reuse (requires `@modelcontextprotocol/sdk` interop)
- Anthropic SDK: https://github.com/anthropics/anthropic-sdk-typescript
- MCP SDK: https://github.com/modelcontextprotocol/typescript-sdk
- Existing MCP server builds: `packages/mcp-server/tsconfig.json`, `packages/fmp-mcp-server/tsconfig.json`
