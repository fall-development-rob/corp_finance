# ADR-033: Multi-Provider Abstraction Layer

## Status: Accepted

## Date: 2026-05-09

## Deciders

- Robert Fall
- CFA Agent platform engineering

## Tags

`provider-abstraction`, `openai`, `gemini`, `bedrock`, `tool-schema`, `phase-31`

## Context

The harness (ADR-031) is built first against the Anthropic Messages API (ADR-032). However, one of the stated goals in the Phase 31 master plan is a provider-neutral dispatch loop: `"Anthropic now, OpenAI/Gemini/Bedrock as additive provider adapters."` The motivation is not mere optionality — it is risk management and commercial flexibility:

- The CFA platform compute layer (`corp-finance-core`, the four plugin MCP servers) is entirely provider-neutral: tools are registered as JSON-RPC endpoints that accept plain JSON inputs and return plain JSON outputs. No part of the compute stack cares which model is orchestrating it.
- LLM provider pricing, rate limits, and capability gaps shift on a monthly cadence in this market. An institutional platform that hard-codes a single model provider accepts vendor lock-in that a financial services firm may not be able to justify to its compliance or procurement functions.
- The Phase 31 master plan references `ruvnet/open-claude-code` as the source of the multi-provider pattern: `"Provider abstraction for Anthropic/OpenAI/Google/Bedrock/Vertex."` The architectural pattern is established and understood.

The central challenge in a multi-provider abstraction for tool-calling agents is **tool-schema translation**. The four providers use structurally different wire formats for tool definitions and tool-call results:

- **Anthropic** — `{"name": ..., "description": ..., "input_schema": {"type": "object", "properties": {...}}}` in the `tools` array; model returns `{"type": "tool_use", "id": ..., "name": ..., "input": {...}}` in `content`; caller returns `{"type": "tool_result", "tool_use_id": ..., "content": [...]}`.
- **OpenAI** — `{"type": "function", "function": {"name": ..., "description": ..., "parameters": {...}}}` in `tools`; model returns `{"role": "assistant", "tool_calls": [{"id": ..., "type": "function", "function": {"name": ..., "arguments": "..."}}]}`; caller returns `{"role": "tool", "tool_call_id": ..., "content": "..."}`.
- **Gemini** — `{"function_declarations": [{"name": ..., "description": ..., "parameters": {...}}]}` in `tools`; model returns `{"functionCall": {"name": ..., "args": {...}}}` in parts; caller returns `{"functionResponse": {"name": ..., "response": {...}}}`.
- **Bedrock (Anthropic Converse API)** — structurally similar to Anthropic Messages API but with a different endpoint and auth mechanism; tool schema compatible when using Claude models via Bedrock.

The `input_schema` / `parameters` fields are all JSON Schema objects, but nesting and required fields differ subtly. The `arguments` field in OpenAI is a serialized JSON string, not an object. Gemini `args` is an object but uses `camelCase` field names in some SDK versions.

## Decision

Implement a provider abstraction layer in **`packages/core/providers/`** with a canonical internal tool schema defined in **`packages/core/tool-schema.ts`**. Translation to provider wire formats happens at the boundary of each provider adapter.

### Canonical tool schema (Anthropic-flavored)

The internal tool representation is Anthropic-flavored because Anthropic is the Wave 1 provider and the cfa-core MCP servers expose their tool definitions via the Anthropic `Tool` type from `@anthropic-ai/sdk`. The canonical shape:

```typescript
// packages/core/tool-schema.ts

export interface CanonicalTool {
  name: string;
  description: string;
  input_schema: {
    type: "object";
    properties: Record<string, JSONSchema7>;
    required?: string[];
  };
}

export interface CanonicalToolCall {
  id: string;
  name: string;
  input: Record<string, unknown>;
}

export interface CanonicalToolResult {
  tool_call_id: string;
  content: string; // JSON-serialized result
  is_error?: boolean;
}
```

These types are the internal contract. Provider adapters translate to/from these shapes at the wire boundary.

### Provider interface

```typescript
// packages/core/providers/types.ts

export interface ProviderMessage {
  role: "user" | "assistant";
  content: string | ProviderContentBlock[];
}

export interface ProviderResponse {
  stop_reason: "end_turn" | "tool_use" | "max_tokens";
  tool_calls: CanonicalToolCall[];
  text: string;
  usage: { input_tokens: number; output_tokens: number };
}

export interface Provider {
  name: string;
  chat(
    system: string,
    tools: CanonicalTool[],
    messages: ProviderMessage[]
  ): Promise<ProviderResponse>;
}
```

The `agent-loop.ts` in `packages/core` depends only on `Provider`, never on `@anthropic-ai/sdk` directly. The Anthropic adapter (`packages/core/providers/anthropic.ts`) wraps the SDK and translates between `CanonicalTool[]` and `Anthropic.Tool[]`.

### Wave 1: Anthropic adapter (ships with Wave 1 of Phase 31)

`packages/core/providers/anthropic.ts` translates `CanonicalTool` → `Anthropic.Tool` (trivial: the shapes are identical). Returns `CanonicalToolCall` from `ToolUseBlock`. Returns `CanonicalToolResult` from tool result content blocks.

### Wave 3: OpenAI adapter

`packages/core/providers/openai.ts` translates:
- `CanonicalTool` → OpenAI `ChatCompletionTool` (`input_schema` → `parameters`, wrap in `{"type": "function", "function": {...}}`).
- `tool_calls[].function.arguments` (string) → `JSON.parse()` → `CanonicalToolCall.input`.
- `CanonicalToolResult` → `{role: "tool", tool_call_id: ..., content: ...}` message.

### Wave 3: Gemini adapter

`packages/core/providers/gemini.ts` translates:
- `CanonicalTool` → `FunctionDeclaration` (`input_schema` → `parameters`; `properties` keys left as-is).
- `Part.functionCall` → `CanonicalToolCall`.
- `CanonicalToolResult` → `Part.functionResponse`.

### Wave 4: Bedrock adapter (optional)

`packages/core/providers/bedrock.ts` wraps AWS Bedrock's Converse API. When using Claude models via Bedrock, the tool schema is compatible with the Anthropic adapter; the adapter handles only auth and endpoint differences.

### Tool definition source of truth

Tool definitions flow from MCP servers → `packages/mcp-client/stdio.ts` (via `tools/list`) → `CanonicalTool[]`. The MCP `tools/list` response uses a JSON Schema-compatible `inputSchema` field that maps directly to `CanonicalTool.input_schema`. No hand-written tool definitions are maintained in the harness; they are always sourced from the live MCP servers.

## Consequences

### Positive

- The agent loop (`packages/core/agent-loop.ts`) is provider-neutral from Wave 1. Adding a new provider requires implementing the `Provider` interface in a new file; no changes to the loop, router, or agent registry.
- Tool definitions flow from MCP servers once, are translated once at the provider boundary, and are not maintained in multiple places.
- The Anthropic-flavored canonical shape means zero translation cost for the Wave 1 Anthropic adapter.
- Cross-provider acceptance testing (`tests/cross-provider-determinism.test.ts`) validates that the same prompt produces the same tool calls regardless of provider, which guards against translation bugs.

### Negative

- The provider interface imposes a synchronous call/response abstraction. Streaming responses (Anthropic's `messages.stream()`) are flattened to a single `ProviderResponse`. This loses streaming for the CLI's live output display; a streaming-capable provider interface is a Wave 3+ enhancement.
- Gemini's `functionResponse` does not support multi-part content blocks in the same way Anthropic does; complex tool results (e.g., an MCP tool returning a list of objects) require serialisation to a string, losing structured content in the Gemini path.
- Maintaining translations for three provider wire formats introduces ongoing maintenance: each provider SDK update may require adapter updates. Provider API drift must be tracked.

### Neutral

- OpenAI and Gemini providers are Wave 3 deliverables. Wave 1 ships only the Anthropic adapter. The abstraction is designed before it is needed, but the additional LOC in Wave 1 is minimal (the `Provider` interface + Anthropic adapter is ~100 lines).
- Bedrock is Wave 4 and conditional on demand. If no institutional customer requires Bedrock, it is never shipped.

## Alternatives Considered

**Single-provider (Anthropic only), no abstraction** — Simpler Wave 1. Rejected because retrofitting a provider interface after 2000 LOC of Anthropic-specific code is written is substantially harder than designing the interface upfront. The `Provider` interface costs ~50 lines in Wave 1 and eliminates a 500-line refactor in Wave 3.

**Use an existing multi-provider library (LangChain, Vercel AI SDK)** — Both abstract provider differences. Rejected because: (1) LangChain adds ~300 transitive dependencies and its tool-calling abstraction does not align cleanly with MCP's `tools/list` schema shape; (2) Vercel AI SDK is optimised for streaming UI use cases, not CLI batch dispatch; (3) neither library is a declared dependency in the existing workspace; adding a 300-dep library for an abstraction that is ~200 lines of hand-written code is disproportionate.

**Canonical schema based on OpenAI (most widely supported)** — OpenAI's `function.parameters` shape is the de-facto standard for many providers. Rejected because the MCP `tools/list` response and the Anthropic `Tool` type both use `input_schema`, and the Wave 1 and primary provider is Anthropic. Using OpenAI's schema as canonical would add a non-trivial translation layer in Wave 1 for no benefit.

**One harness binary per provider** — Separate `cfa-harness-anthropic`, `cfa-harness-openai` etc. Rejected: duplicates the agent loop, tool router, and MCP client for each provider. The abstraction exists precisely to avoid this.

## References

- Master plan: `docs/plans/phase-31-harness.md`
- ADR-031: Custom dispatch harness (motivates provider flexibility as a design goal)
- ADR-032: TypeScript + Anthropic SDK (Wave 1 provider; the canonical schema is Anthropic-flavored)
- ADR-034: MCP plugin reuse (MCP `tools/list` is the source of `CanonicalTool[]`)
- Inspiration: `ruvnet/open-claude-code` provider abstraction pattern (referenced in Phase 31 plan)
- Anthropic Messages API tool format: https://docs.anthropic.com/en/api/messages
- OpenAI function calling: https://platform.openai.com/docs/guides/function-calling
- Gemini function calling: https://ai.google.dev/gemini-api/docs/function-calling
