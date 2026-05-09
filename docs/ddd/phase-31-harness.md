# Domain Model: CFA Harness (Phase 31)

**Date:** 2026-05-09
**Status:** Draft
**Plan:** `docs/plans/phase-31-harness.md`
**PRD:** `docs/prd/phase-31-harness.md`

---

## Overview

Phase 31 introduces a custom TypeScript dispatch harness that owns the agent loop end-to-end. The harness replaces the broken Claude Code subagent path (which returns `tool_uses: 0`) with a direct `@anthropic-ai/sdk` Messages-API loop that correctly invokes MCP tools. Six bounded contexts divide the domain: **Agent Runtime** drives the turn-by-turn conversation loop; **MCP Transport** handles JSON-RPC framing to the four plugin servers; **Provider Abstraction** normalises provider-specific wire formats; **Agent Registry** holds agent specifications and the delegation topology; **Audit Chain** records every invocation in a sha2-linked log; and **Session Memory** persists per-conversation state for replay.

The six contexts share no direct coupling. Agent Runtime depends on MCP Transport and Provider Abstraction via published interfaces. Audit Chain observes Agent Runtime events as a conformist consumer. Session Memory is a customer of Agent Runtime events and provides read-only replay to Agent Runtime on session restore. Agent Registry is a shared kernel consulted by Agent Runtime and the CLI entry point.

---

## Context 1: Agent Runtime

### Overview

Agent Runtime is the core dispatch engine. It runs an async generator loop over `@anthropic-ai/sdk` Messages API calls, detects `tool_use` blocks in responses, routes each block to MCP Transport, injects `tool_result` blocks back into the message history, and iterates until the provider emits `stop_reason: end_turn` or a turn budget is exhausted. It also handles provider-level errors (rate limits, network timeouts) with bounded exponential back-off. The loop is the single path that replaces the broken Claude Code subagent dispatch.

### Domain Language

| Term | Definition |
|------|-----------|
| **Agent Turn** | One round-trip: a Messages-API request carrying the current history + one response containing text and/or `tool_use` blocks. |
| **Turn History** | The ordered list of `message` objects (user, assistant, tool_result roles) accumulated within one agent session. |
| **Tool Use Block** | An `assistant`-role content block with `type: "tool_use"`, carrying `id`, `name`, and `input`. Emitted by the provider; consumed by the tool router. |
| **Tool Result Block** | A `user`-role content block with `type: "tool_result"`, carrying `tool_use_id` and `content`. Injected back into Turn History after MCP execution. |
| **Dispatch Loop** | The async generator that yields completed turns. Terminates on `end_turn`, turn-budget exhaustion, or unrecoverable error. |
| **End Turn** | The provider `stop_reason` value `"end_turn"` signalling no further tool calls are pending. |
| **Turn Budget** | Maximum number of turns allowed for a single agent session (default: 40). Guards against runaway loops. |
| **Back-off Window** | Exponential retry interval applied when the provider returns `429` or a transient network error. Capped at 60 s. |
| **Agent Session** | One instantiation of the dispatch loop for a given agent spec and initial prompt. Has a unique `session_id`. |
| **Sub-dispatch** | A nested Agent Session spawned by the chief-analyst to delegate a sub-goal to a specialist. Bounded to depth 1 in Wave 2. |

### Aggregates

#### AgentSession (Aggregate Root)

Represents one running or completed execution of the dispatch loop for a single agent.

| Field | Type | Description |
|-------|------|-------------|
| `session_id` | `string` (UUID v7) | Unique, time-sortable identifier |
| `agent_id` | `string` | Agent spec identifier from the Registry (e.g., `chief-analyst`) |
| `parent_session_id` | `string \| null` | Non-null for specialist sub-dispatches |
| `turn_history` | `Message[]` | Ordered list of Messages-API message objects |
| `tool_use_count` | `number` | Running total of successful tool invocations |
| `turn_count` | `number` | Number of completed turns |
| `turn_budget` | `number` | Maximum turns before forced termination |
| `status` | `SessionStatus` | `running \| completed \| failed \| budget_exhausted` |
| `started_at` | `Date` | Wall-clock start time |
| `completed_at` | `Date \| null` | Wall-clock end time; null while running |
| `prompt_hash` | `string` | SHA-256 of the initial user prompt |
| `result_hash` | `string \| null` | SHA-256 of the final assistant text; null until completed |

**Invariants:**

- `turn_count <= turn_budget` at all times; violation transitions status to `budget_exhausted`.
- `parent_session_id` depth is at most 1; a sub-dispatch session may not itself spawn a sub-dispatch.
- `tool_use_count` increments only on successful MCP tool completions; failed tool calls are recorded but not counted.
- `result_hash` is set if and only if `status === "completed"`.
- `started_at <= completed_at` when `completed_at` is non-null.
- `turn_history` is append-only; no message object is mutated once added.

#### AgentTurn (Entity within AgentSession)

One turn within the dispatch loop — not an aggregate root; owned by AgentSession.

| Field | Type | Description |
|-------|------|-------------|
| `turn_id` | `number` | 1-indexed sequence within the session |
| `request_messages` | `Message[]` | Snapshot of Turn History sent to provider |
| `response_content` | `ContentBlock[]` | Raw content blocks from provider response |
| `stop_reason` | `string` | Provider stop_reason (`"tool_use"` or `"end_turn"`) |
| `tool_uses` | `ToolUseBlock[]` | Extracted tool_use blocks from response |
| `tool_results` | `ToolResultBlock[]` | MCP results injected as tool_result messages |
| `input_tokens` | `number` | Provider-reported input token count |
| `output_tokens` | `number` | Provider-reported output token count |

**Invariants:**

- `tool_results.length === tool_uses.length` when the turn completes without error.
- If `stop_reason === "end_turn"` then `tool_uses` is empty.

### Domain Events

| Event | Raised By | Payload | Consumers |
|-------|-----------|---------|-----------|
| `session_started` | AgentSession | `session_id`, `agent_id`, `parent_session_id`, `prompt_hash`, `started_at` | Audit Chain, Session Memory |
| `turn_completed` | AgentSession | `session_id`, `turn_id`, `tool_uses_count`, `input_tokens`, `output_tokens`, `stop_reason` | Audit Chain, Session Memory |
| `tool_invocation_succeeded` | AgentSession | `session_id`, `turn_id`, `tool_use_id`, `tool_name`, `duration_ms` | Audit Chain |
| `tool_invocation_failed` | AgentSession | `session_id`, `turn_id`, `tool_use_id`, `tool_name`, `error_code` | Audit Chain |
| `session_completed` | AgentSession | `session_id`, `tool_use_count`, `turn_count`, `result_hash`, `completed_at` | Audit Chain, Session Memory |
| `session_failed` | AgentSession | `session_id`, `error_kind`, `retries_exhausted` | Audit Chain, Session Memory |
| `back_off_triggered` | AgentSession | `session_id`, `turn_id`, `delay_ms`, `reason` | Audit Chain |
| `sub_dispatch_requested` | AgentSession | `parent_session_id`, `child_session_id`, `specialist_id`, `sub_goal` | Audit Chain, Session Memory |

### Anti-Corruption Layer

Agent Runtime is isolated from the provider wire format by the Provider Abstraction context. It sends and receives internal `Message` and `ContentBlock` types; translation to/from `@anthropic-ai/sdk` shapes is the Provider Abstraction's responsibility. Agent Runtime is similarly isolated from JSON-RPC details by MCP Transport: it calls `toolRouter.invoke(toolName, input)` and receives a typed `ToolResult`; it never sees raw JSON-RPC frames.

### Repository / Persistence

Agent Runtime holds `AgentSession` in memory for its duration. At session completion it emits `session_completed` which Session Memory persists to disk. Agent Runtime may call `SessionMemoryRepository.restore(session_id)` to re-hydrate a prior session for replay.

### Cross-Context Relationships

- **Depends on** MCP Transport — all tool invocations delegated via `ToolRouter` interface.
- **Depends on** Provider Abstraction — all Messages-API calls delegated via `ProviderClient` interface.
- **Consults** Agent Registry — resolves `AgentSpec` at session start.
- **Emits to** Audit Chain — all events above are observed by the Audit Chain.
- **Emits to** Session Memory — session and turn events are persisted.

---

## Context 2: MCP Transport

### Overview

MCP Transport manages connections to the four plugin MCP servers (cfa-core, cfa-data, fmp-market-data, vendor) and exposes a single `ToolRouter` interface to Agent Runtime. It handles the MCP handshake (`initialize` → `initialized` acknowledgement, `tools/list` discovery), JSON-RPC 2.0 framing over stdio (Wave 1), SSE and HTTP (Wave 3), and WebSocket (Wave 4). It caches the tool manifest from each server at startup and routes `tools/call` requests to the appropriate server by tool name prefix.

### Domain Language

| Term | Definition |
|------|-----------|
| **MCP Server** | An external process (or remote endpoint) that implements the Model Context Protocol and exposes a set of named tools. |
| **Transport** | The I/O mechanism connecting the harness to an MCP Server. One of: `stdio`, `sse`, `http`, `websocket`. |
| **MCP Handshake** | The three-message exchange: `initialize` (client → server), `initialized` (server → client), `tools/list` (client → server) producing the tool manifest. |
| **Tool Manifest** | The complete list of tool descriptors (`name`, `description`, `inputSchema`) returned by `tools/list` for one MCP server. |
| **JSON-RPC Frame** | A JSON-RPC 2.0 request or response object: `{jsonrpc, id, method, params}` for requests; `{jsonrpc, id, result}` or `{jsonrpc, id, error}` for responses. |
| **Tool Router** | The harness-internal component that accepts `(tool_name, input)` pairs and dispatches them to the correct MCP server via the correct transport. |
| **Tool Call** | A `tools/call` JSON-RPC request carrying `{name, arguments}` and expecting a `{content}` response. |
| **Tool Name Prefix** | The four-segment prefix (`plugin__cfa-core__cfa-core`, etc.) that identifies which MCP server owns a tool name. |
| **Server Handle** | An open connection to one MCP server: holds the transport stream, the pending-request map, and the cached tool manifest. |

### Aggregates

#### McpServerConnection (Aggregate Root)

Represents one active (or terminated) connection to a single MCP server.

| Field | Type | Description |
|-------|------|-------------|
| `connection_id` | `string` (UUID v7) | Unique identifier |
| `server_name` | `string` | Logical server name (e.g., `cfa-core`) |
| `transport_kind` | `TransportKind` | `stdio \| sse \| http \| websocket` |
| `endpoint` | `string` | Command string (stdio) or URL (remote transports) |
| `tool_manifest` | `ToolDescriptor[]` | Cached list of tool descriptors from `tools/list` |
| `pending_requests` | `Map<number, PendingRequest>` | In-flight JSON-RPC requests keyed by id |
| `status` | `ConnectionStatus` | `connecting \| ready \| degraded \| closed` |
| `handshake_completed_at` | `Date \| null` | Timestamp of successful `initialized` receipt |

**Invariants:**

- `tool_manifest` is non-empty only after `status === "ready"`.
- `pending_requests` is empty when `status === "closed"`.
- Each JSON-RPC request id is unique within the connection lifetime.
- No `tools/call` request is dispatched before the handshake completes.

#### ToolRouterTable (Aggregate)

Maps tool names to the `McpServerConnection` that owns them.

| Field | Type | Description |
|-------|------|-------------|
| `table_id` | `string` | Identifier (one per harness process) |
| `entries` | `Map<string, string>` | tool_name → connection_id |
| `server_connections` | `McpServerConnection[]` | All managed connections |
| `total_tools` | `number` | Sum of all manifest lengths |

**Invariants:**

- Tool names are globally unique across all connections; duplicate tool names across servers are rejected at startup.
- `total_tools` matches the sum of all `tool_manifest.length` values.
- A `tools/call` request is only dispatched to the connection that owns the tool name.

### Domain Events

| Event | Raised By | Payload | Consumers |
|-------|-----------|---------|-----------|
| `connection_established` | McpServerConnection | `connection_id`, `server_name`, `transport_kind`, `handshake_completed_at` | Audit Chain |
| `tool_manifest_loaded` | McpServerConnection | `connection_id`, `server_name`, `tool_count` | ToolRouterTable, Audit Chain |
| `tool_call_dispatched` | ToolRouterTable | `connection_id`, `tool_name`, `rpc_id`, `session_id` | Audit Chain |
| `tool_call_returned` | ToolRouterTable | `connection_id`, `tool_name`, `rpc_id`, `duration_ms`, `success` | Audit Chain |
| `connection_degraded` | McpServerConnection | `connection_id`, `server_name`, `error_detail` | Audit Chain |
| `connection_closed` | McpServerConnection | `connection_id`, `server_name` | Audit Chain |

### Anti-Corruption Layer

MCP Transport translates between the harness-internal `ToolInvocation` type and the MCP/JSON-RPC wire format. Agent Runtime calls `ToolRouter.invoke(name, input): Promise<ToolResult>` — it never sees JSON-RPC ids, method strings, or transport-specific framing. The ACL functions are:

| Direction | ACL function | Wire format |
|-----------|--------------|-------------|
| `ToolInvocation` → JSON-RPC request | `toRpcRequest(inv): JsonRpcRequest` | `{jsonrpc:"2.0", id, method:"tools/call", params:{name, arguments}}` |
| JSON-RPC response → `ToolResult` | `fromRpcResponse(resp): ToolResult` | Validates `result.content[]` shape; maps error codes to domain errors |
| `ToolDescriptor` → harness schema | `toHarnessSchema(desc): ToolSchema` | Strips MCP-specific fields not used by Provider Abstraction |

### Repository / Persistence

Connections are runtime-only. The tool manifest is fetched at startup and held in memory; it is not persisted. If a connection drops, the harness reconnects and re-fetches the manifest. The Audit Chain receives connection lifecycle events for durability.

### Cross-Context Relationships

- **Serves** Agent Runtime via `ToolRouter` interface.
- **Provides tool schemas to** Provider Abstraction (so the provider can include tool definitions in API requests).
- **Emits to** Audit Chain — connection and tool-call events.

---

## Context 3: Provider Abstraction

### Overview

Provider Abstraction normalises the differences between Anthropic Messages API, OpenAI Chat Completions, Google Gemini Generate Content, and Amazon Bedrock Converse into a single `ProviderClient` interface consumed by Agent Runtime. Wave 1 ships the Anthropic adapter; Waves 3 and 4 add OpenAI/Gemini and Bedrock. The key translation challenge is tool-schema format: Anthropic uses JSON Schema inline; OpenAI uses a `function` wrapper; Gemini uses `FunctionDeclaration`; Bedrock uses its own envelope. All translation occurs at this boundary; tool schemas are stored in provider-neutral form in `packages/core/tool-schema.ts`.

### Domain Language

| Term | Definition |
|------|-----------|
| **Provider** | An LLM API endpoint: one of Anthropic, OpenAI, Gemini, Bedrock. |
| **ProviderClient** | The harness-internal interface that Agent Runtime calls. Hides provider-specific request/response shapes. |
| **Provider-Neutral Message** | Internal message type shared across the harness. Role is one of `user \| assistant`. Content is `TextBlock \| ToolUseBlock \| ToolResultBlock`. |
| **Provider Request** | The provider-specific API request object built from Provider-Neutral Messages and tool schemas. |
| **Provider Response** | The provider-specific API response; translated back to Provider-Neutral Messages by the adapter. |
| **Tool Schema** | Provider-neutral JSON Schema descriptor for a single tool. Translated to provider format at the wire boundary. |
| **Adapter** | One concrete implementation of `ProviderClient` for a single provider. |
| **Stop Reason** | Normalised enumeration: `"tool_use"` (model wants to call tools) or `"end_turn"` (model is done). |
| **Token Usage** | Normalised `{input_tokens, output_tokens}` pair extracted from any provider response. |

### Aggregates

#### ProviderAdapter (Aggregate Root)

One configured adapter for a specific provider. Holds API credentials reference (never the credential value) and configuration.

| Field | Type | Description |
|-------|------|-------------|
| `adapter_id` | `string` | `anthropic \| openai \| gemini \| bedrock` |
| `model_id` | `string` | The model string passed to the API (e.g., `claude-sonnet-4-6`) |
| `api_key_env_var` | `string` | Name of the environment variable holding the API key (never the key value) |
| `max_tokens` | `number` | Per-turn output token cap passed to the provider |
| `temperature` | `number \| null` | Sampling temperature; null uses provider default |
| `tool_schemas` | `ToolSchema[]` | Provider-neutral schemas; translated at request build time |

**Invariants:**

- `api_key_env_var` is a non-empty string; the harness reads the env var at call time and never caches the value in any aggregate.
- `model_id` is validated against a known set at construction time; unknown model ids are rejected.
- `max_tokens >= 1024`; values below this are rejected.
- `tool_schemas` may be empty only when the session is intentionally running without tools (text-only mode).

### Value Objects

#### ProviderNeutralMessage

| Field | Type |
|-------|------|
| `role` | `"user" \| "assistant"` |
| `content` | `Array<TextBlock \| ToolUseBlock \| ToolResultBlock>` |

Equality is structural (same role + same content sequence).

#### TranslatedRequest

Opaque wrapper around the provider-specific request object. Carries the provider id so the transport layer routes to the correct SDK method.

#### NormalisedResponse

| Field | Type |
|-------|------|
| `content` | `Array<TextBlock \| ToolUseBlock>` |
| `stop_reason` | `"tool_use" \| "end_turn"` |
| `usage` | `{input_tokens: number, output_tokens: number}` |

### Domain Events

| Event | Raised By | Payload | Consumers |
|-------|-----------|---------|-----------|
| `provider_request_sent` | ProviderAdapter | `adapter_id`, `model_id`, `session_id`, `turn_id`, `tool_count` | Audit Chain |
| `provider_response_received` | ProviderAdapter | `adapter_id`, `session_id`, `turn_id`, `stop_reason`, `input_tokens`, `output_tokens` | Audit Chain |
| `provider_error` | ProviderAdapter | `adapter_id`, `session_id`, `turn_id`, `error_code`, `retryable` | Audit Chain, Agent Runtime (for back-off) |

### Anti-Corruption Layer

The adapter boundary is the single point where provider-specific types cross into the harness. ACL functions per adapter:

| Direction | Anthropic adapter ACL | OpenAI adapter ACL |
|-----------|-----------------------|--------------------|
| `ProviderNeutralMessage[]` → request | `toAnthropicMessages()` | `toOpenAiMessages()` |
| `ToolSchema[]` → tool definitions | `toAnthropicTools()` | `toOpenAiFunctions()` |
| SDK response → `NormalisedResponse` | `fromAnthropicResponse()` | `fromOpenAiResponse()` |
| SDK error → `ProviderError` | `fromAnthropicError()` | `fromOpenAiError()` |

Tool schema translation preserves all fields required by `corp_finance_core` tool contracts. Fields unknown to a provider are dropped silently; the Audit Chain records the adapter id so provenance is traceable.

### Repository / Persistence

ProviderAdapter is a stateless configuration object. It holds no conversation history (that lives in Agent Runtime's `AgentSession`). There is no persistence requirement for this context.

### Cross-Context Relationships

- **Serves** Agent Runtime via `ProviderClient` interface.
- **Receives tool schemas from** MCP Transport (via `ToolRouterTable`).
- **Emits to** Audit Chain.

---

## Context 4: Agent Registry

### Overview

Agent Registry is a shared kernel that holds code-defined agent specifications. Each specification (`AgentSpec`) declares the agent's identity, system prompt, the subset of MCP tools it is permitted to call, and its delegation rules (which specialists it may sub-dispatch to and under what conditions). The Registry is loaded once at harness startup from the `packages/agents/` directory and treated as read-only at runtime. The nine CFA specialists are first-class entries in the Registry alongside the chief-analyst.

### Domain Language

| Term | Definition |
|------|-----------|
| **AgentSpec** | A code-defined descriptor for one agent: identity, system prompt, tool allowlist, and delegation rules. |
| **Tool Allowlist** | The set of tool names (or glob patterns) the agent is permitted to invoke. Requests for tools outside the allowlist are rejected before dispatch. |
| **Delegation Rule** | A typed rule declaring that the agent may sub-dispatch to a named specialist under a stated precondition. |
| **Capability Map** | The index of which tools each agent can reach; derived from all tool allowlists at startup. |
| **Dispatch Table** | The map from agent id to AgentSpec; the lookup path used by Agent Runtime at session start. |
| **CFA Specialist** | One of the eight domain specialists: derivatives, equity, credit, fixed-income, quant-risk, macro, private-markets, esg-regulatory. |
| **Chief Analyst** | The orchestrating agent that decomposes user goals and delegates to specialists. Has the widest tool allowlist (all 623 tools in Wave 1). |
| **Tool Subset** | The filtered tool manifest exposed to a specialist, derived by intersecting the specialist's Tool Allowlist with the full ToolRouterTable manifest. |

### Aggregates

#### AgentSpec (Aggregate Root)

| Field | Type | Description |
|-------|------|-------------|
| `agent_id` | `string` | Stable identifier (e.g., `chief-analyst`, `derivatives`) |
| `display_name` | `string` | Human-readable name |
| `system_prompt` | `string` | Full system prompt text; treated as immutable once loaded |
| `tool_allowlist` | `string[]` | Glob patterns (e.g., `plugin__cfa-core__*`, `plugin__cfa-pro__fmp-market-data__option_*`) |
| `delegation_rules` | `DelegationRule[]` | Which specialists this agent may sub-dispatch to |
| `max_turn_budget` | `number` | Per-session turn cap for this agent (overrides harness default) |
| `provider_preference` | `string \| null` | Preferred provider id; null uses harness default |

**Invariants:**

- `agent_id` is unique within the Registry.
- `tool_allowlist` patterns are validated against the live `ToolRouterTable` at startup; a pattern that matches zero tools emits a warning but does not fail startup.
- `delegation_rules` may only reference `agent_id` values present in the Registry; forward references are rejected.
- `system_prompt` is non-empty.
- Chief-analyst `delegation_rules` references all eight specialists; specialists have empty `delegation_rules`.

#### AgentRegistry (Aggregate)

| Field | Type | Description |
|-------|------|-------------|
| `registry_id` | `string` | Harness process identifier |
| `agents` | `Map<string, AgentSpec>` | All registered agents |
| `capability_map` | `Map<string, string[]>` | agent_id → resolved tool names |
| `loaded_at` | `Date` | Timestamp of last registry load |

**Commands:**

- `resolve(agent_id): AgentSpec` — throws if not found.
- `toolSubset(agent_id, full_manifest): ToolDescriptor[]` — returns the filtered tool list for the agent.
- `delegationTargets(agent_id): string[]` — returns specialist ids the agent may sub-dispatch to.

**Invariants:**

- Registry is read-only after `loaded_at` is set; no runtime mutations.
- `capability_map` is computed once at load time and cached.

### Domain Events

| Event | Raised By | Payload | Consumers |
|-------|-----------|---------|-----------|
| `registry_loaded` | AgentRegistry | `registry_id`, `agent_count`, `total_tools_across_agents`, `loaded_at` | Audit Chain |
| `tool_allowlist_warning` | AgentRegistry | `agent_id`, `pattern`, `matched_count` | Operator log |

### Anti-Corruption Layer

Agent Registry reads agent spec files from `packages/agents/`. The file format is a TypeScript module exporting an `AgentSpec` object; no external-system schema leaks into the domain type. The spec loader validates each spec against the domain invariants before registering it.

### Repository / Persistence

The Registry is loaded from the filesystem at startup and held in memory. There is no runtime persistence; agent specs are source-controlled. If the Registry needs to be refreshed (e.g., after a deploy), the harness process restarts.

### Cross-Context Relationships

- **Shared Kernel with** Agent Runtime — `AgentSpec` is the canonical type consumed by both.
- **Consulted by** MCP Transport — tool allowlists determine which tool subsets are exposed.
- **Consulted by** CLI entry point — to validate `--agent` flag values before spawning a session.

---

## Context 5: Audit Chain

### Overview

Audit Chain records every significant harness event — session lifecycle, every tool invocation, provider calls, and registry loads — in a sha2-linked append-only log. Each entry hashes the previous entry's hash, the event payload, and a wall-clock timestamp, producing a chain that detects any post-hoc tampering. The chain mirrors the `corp_finance_core::audit` pattern shipped in Phase 26 and is exported to `.audit.json` alongside memo outputs. Numerical results are traceable to specific tool calls with logged inputs and outputs.

### Domain Language

| Term | Definition |
|------|-----------|
| **Audit Entry** | An immutable record of a single harness event: type, payload, timestamp, and chain hash. |
| **Chain Hash** | `SHA-256(prev_hash \|\| event_type \|\| canonical_json(payload) \|\| timestamp_iso)`. The first entry uses `prev_hash = "genesis"`. |
| **Audit Log** | The ordered sequence of Audit Entries for one harness run; written as a JSON Lines file to `<output>.audit.json`. |
| **Canonical JSON** | Deterministic JSON serialisation (keys sorted, no extra whitespace) used as the hash input; guarantees the same payload always produces the same hash. |
| **Invocation Record** | An Audit Entry of type `tool_invocation_succeeded` or `tool_invocation_failed`; captures tool name, input hash, output hash, and duration. |
| **Session Record** | An Audit Entry of type `session_completed` or `session_failed`; captures the final `result_hash` and `tool_use_count`. |

### Aggregates

#### AuditLog (Aggregate Root)

| Field | Type | Description |
|-------|------|-------------|
| `log_id` | `string` (UUID v7) | Unique identifier for this log file |
| `session_id` | `string` | The Agent Runtime session this log covers |
| `entries` | `AuditEntry[]` | Ordered, append-only list |
| `head_hash` | `string` | Current chain tip hash |
| `output_path` | `string` | Filesystem path where the log is written |
| `closed` | `boolean` | True once the session completes; no further writes allowed |

**Commands:**

- `append(event_type, payload): AuditEntry` — computes chain hash, appends, flushes to disk.
- `close(): void` — marks the log closed; subsequent `append` calls throw.
- `verify(): boolean` — re-walks the chain and confirms every hash links correctly.

**Invariants:**

- Entries are append-only; no deletion or mutation after insertion.
- `head_hash` after each `append` equals the chain hash of the last entry.
- `log_id` matches the `session_id` of the Agent Runtime session being audited.
- `closed === true` implies no further `append` calls succeed.
- The first entry's `prev_hash` field is always the literal string `"genesis"`.

#### AuditEntry (Value Object within AuditLog)

| Field | Type | Description |
|-------|------|-------------|
| `entry_id` | `number` | 1-indexed sequence number |
| `event_type` | `string` | The domain event type string (e.g., `session_started`) |
| `payload_hash` | `string` | SHA-256 of `canonical_json(payload)` |
| `payload` | `object` | The event payload (tool names, hashes, durations, token counts) |
| `prev_hash` | `string` | Hash of the previous entry (or `"genesis"`) |
| `chain_hash` | `string` | SHA-256 of `prev_hash || event_type || payload_hash || timestamp` |
| `timestamp` | `string` | ISO-8601 UTC timestamp |

### Domain Events

Audit Chain is a conformist consumer; it does not raise events itself. It subscribes to all events from Agent Runtime, MCP Transport, Provider Abstraction, and Agent Registry.

### Anti-Corruption Layer

Audit Chain receives raw domain events via an internal event bus. Each event type has a registered handler that extracts only the fields needed for the Audit Entry payload (no raw SDK objects are stored). The `toAuditPayload(event)` function maps domain event objects to plain JSON-serialisable objects, stripping any non-serialisable fields.

### Repository / Persistence

AuditLog writes to a JSON Lines file at `output_path` (e.g., `out/chaco-memo.audit.json`). Each `append` call flushes the new line synchronously to disk before returning, guaranteeing no entry is lost on crash. The log file is closed and sealed on `session_completed` or `session_failed`.

### Cross-Context Relationships

- **Observes** Agent Runtime — receives all session and turn events.
- **Observes** MCP Transport — receives connection and tool-call events.
- **Observes** Provider Abstraction — receives request/response events.
- **Observes** Agent Registry — receives registry load event.
- No context depends on Audit Chain at runtime (pure observer).

---

## Context 6: Session Memory

### Overview

Session Memory persists per-conversation state so that harness sessions can be replayed or resumed. It stores the full Turn History, all Tool Results, and the sub-dispatch tree (parent → child session links) in a portable JSON archive on disk. At session start the harness may optionally restore a prior session from disk, re-feeding the Turn History into the dispatch loop to continue from the last completed turn. Session Memory is the replay mechanism and the durable backing store for the Audit Chain's session-level data.

### Domain Language

| Term | Definition |
|------|-----------|
| **Session Archive** | A gzip-compressed JSON file on disk containing the complete state of one `AgentSession`, including Turn History and sub-dispatch tree. |
| **Session Snapshot** | An in-memory point-in-time copy of an `AgentSession`, written to the archive at key lifecycle events. |
| **Replay** | Restoring a Session Archive and feeding its Turn History back into a new dispatch loop invocation as the initial history. Produces a resumed session. |
| **Sub-Dispatch Tree** | The parent/child session relationship graph for one top-level chief-analyst session and all specialist sub-dispatches it spawned. |
| **Session Store** | The filesystem directory (default: `.cfa-sessions/`) where archives are written and read. |
| **Checkpoint** | A Session Snapshot written after each completed turn; allows partial replay if the harness crashes mid-session. |

### Aggregates

#### SessionArchive (Aggregate Root)

| Field | Type | Description |
|-------|------|-------------|
| `archive_id` | `string` | Matches the `session_id` of the source `AgentSession` |
| `agent_id` | `string` | Agent spec identifier |
| `prompt_hash` | `string` | SHA-256 of the initial prompt |
| `turn_history` | `Message[]` | Complete ordered Turn History at time of snapshot |
| `tool_results` | `Map<string, ToolResult>` | tool_use_id → result; for all successful tool calls |
| `sub_dispatch_tree` | `SubDispatchNode[]` | Ordered list of sub-dispatches with their own session ids |
| `final_result` | `string \| null` | The last assistant text response; null if incomplete |
| `result_hash` | `string \| null` | SHA-256 of `final_result`; null if incomplete |
| `checkpoint_turn` | `number` | Turn number at last checkpoint write |
| `status` | `SessionStatus` | Mirrors Agent Runtime status |
| `archived_at` | `Date` | Timestamp of last write |

**Invariants:**

- `archive_id` is immutable once set.
- `turn_history` only grows; turns are never removed from a persisted archive.
- `result_hash` matches SHA-256 of `final_result` when both are non-null.
- `checkpoint_turn <= turn_history.length`.

#### SubDispatchNode (Value Object within SessionArchive)

| Field | Type | Description |
|-------|------|-------------|
| `child_session_id` | `string` | Session id of the specialist sub-dispatch |
| `specialist_id` | `string` | Agent spec id of the specialist |
| `sub_goal` | `string` | The delegated sub-goal text |
| `spawned_at_turn` | `number` | Parent turn number when the sub-dispatch was requested |

### Domain Events

| Event | Raised By | Payload | Consumers |
|-------|-----------|---------|-----------|
| `archive_checkpoint_written` | SessionArchive | `archive_id`, `checkpoint_turn`, `archive_path` | Operator log |
| `archive_sealed` | SessionArchive | `archive_id`, `status`, `archived_at` | Operator log |
| `session_restored` | SessionMemoryRepository | `archive_id`, `restored_turn_count`, `resumed_session_id` | Audit Chain, Agent Runtime |

### Anti-Corruption Layer

Session Memory serialises and deserialises `Message` objects using the provider-neutral schema (not Anthropic SDK types). The `MessageSerializer` ACL function maps between the harness-internal `ProviderNeutralMessage` type and the JSON archive format. On restore, the `MessageDeserialiser` reconstructs the Turn History in provider-neutral form; the Provider Abstraction's adapter handles translation to the wire format when the resumed session makes its first API call.

### Repository / Persistence

`SessionMemoryRepository` writes archives to the Session Store directory as `<session_id>.session.json.gz`. It exposes:

- `checkpoint(session: AgentSession): Promise<void>` — writes a gzip snapshot after each turn.
- `seal(session: AgentSession): Promise<void>` — writes the final snapshot and marks the archive closed.
- `restore(session_id: string): Promise<SessionArchive>` — reads and decompresses the archive.
- `list(): Promise<SessionArchiveSummary[]>` — returns metadata for all archives in the Session Store.

### Cross-Context Relationships

- **Observes** Agent Runtime — receives `session_started`, `turn_completed`, `session_completed`, `sub_dispatch_requested` events.
- **Serves** Agent Runtime — provides `restore` for replay.
- **No dependency on** MCP Transport, Provider Abstraction, or Audit Chain.

---

## Context Map

```
+=========================================================================+
|                          CFA HARNESS DOMAIN                             |
|                                                                         |
|  +--------------------+           +----------------------------+        |
|  |  Agent Registry    |           |   Provider Abstraction     |        |
|  |  (Shared Kernel)   |           |   (Anti-Corruption Layer)  |        |
|  |                    |           |                            |        |
|  |  AgentRegistry     |           |  ProviderAdapter           |        |
|  |  AgentSpec (root)  |           |  (Anthropic / OAI /        |        |
|  |  CapabilityMap     |           |   Gemini / Bedrock)        |        |
|  +----+---------------+           +-------------+--------------+        |
|       |  resolves spec                          |  NormalisedResponse   |
|       |  at session start                       |  sent to Agent        |
|       v                                         v  Runtime              |
|  +----+-----------------------------------------+-----------------+   |
|  |                       Agent Runtime                            |   |
|  |                                                                |   |
|  |   AgentSession (root)                                          |   |
|  |   AgentTurn                                                    |   |
|  |   Dispatch Loop (async generator)                              |   |
|  |   Back-off / retry                                             |   |
|  |   Sub-dispatch (depth 1)                                       |   |
|  +----+------------------------------+---------+------------------+   |
|       |  ToolRouter.invoke()         |         |                       |
|       |                              |         | events                |
|       v                              v         v                       |
|  +----+------------------+   +-------+----+  ++-----------------+     |
|  |   MCP Transport       |   | Session    |  | Audit Chain      |     |
|  |                       |   | Memory     |  |                  |     |
|  |  McpServerConnection  |   |            |  | AuditLog (root)  |     |
|  |  (root)               |   | Session-   |  | AuditEntry (VO)  |     |
|  |  ToolRouterTable      |   | Archive    |  | SHA-256 chain    |     |
|  |  JSON-RPC framing     |   | (root)     |  | → .audit.json    |     |
|  |  stdio/SSE/HTTP/WS    |   | .session   |  |                  |     |
|  |                       |   | .json.gz   |  |                  |     |
|  +----+------------------+   +------------+  +------------------+     |
|       |                                                                |
|       v                                                                |
|  +----+------------------------------------------------------------------+
|  |  EXISTING — reused without modification                               |
|  |  plugin:cfa-core:cfa-core (227 tools)                                 |
|  |  plugin:cfa-data:data     (129 tools)                                 |
|  |  plugin:cfa-pro:fmp-market-data (180 tools)                           |
|  |  plugin:cfa-pro:vendor (87 tools)                                     |
|  +-----------------------------------------------------------------------+

Context relationships:

  Agent Runtime ──[depends-on]──> MCP Transport
  Agent Runtime ──[depends-on]──> Provider Abstraction
  Agent Runtime ──[consults]────> Agent Registry
  Agent Runtime ──[emits-to]────> Audit Chain          (conformist)
  Agent Runtime ──[emits-to]────> Session Memory       (customer/supplier)
  MCP Transport ──[serves]──────> Agent Runtime
  MCP Transport ──[provides schemas-to]─> Provider Abstraction
  MCP Transport ──[emits-to]────> Audit Chain
  Provider Abstraction ──[serves]──> Agent Runtime
  Provider Abstraction ──[emits-to]─> Audit Chain
  Agent Registry ──[shared kernel]──> Agent Runtime + CLI
  Session Memory ──[serves-replay-to]─> Agent Runtime
  Audit Chain ──[observes-all]──────> all five other contexts
```

---

## Cross-Context Event Flow (Chaco Acceptance Test Path)

```
CLI: cfa-harness run --agent chief-analyst --prompt chaco.md
        |
        v
  AgentRegistry.resolve("chief-analyst")          [Agent Registry]
        |
        v
  AgentSession.start(spec, prompt)                 [Agent Runtime]
    → session_started event
        |   ├──> AuditLog.append(session_started)  [Audit Chain]
        |   └──> SessionArchive.checkpoint()       [Session Memory]
        |
  [Turn 1] ProviderAdapter.complete(history, tools)[Provider Abstraction]
        |   └──> provider_request_sent event → Audit Chain
        |
  Response: tool_use blocks (option_pricer ×20, country_risk_premium ×1)
        |
  [Tool Router] ToolRouterTable.route("option_pricer")
        |   └──> McpServerConnection.call("tools/call")  [MCP Transport]
        |         └──> cfa-core MCP server (stdio)
        |         └──> tool_call_returned event → Audit Chain
        |
  tool_results injected into Turn History
        |
  [end_turn detected] session_completed event
        |   ├──> AuditLog.close()                  [Audit Chain]
        |   └──> SessionArchive.seal()             [Session Memory]
        |
  output written: chaco-memo.md + chaco-memo.audit.json
```

---

## Invariants Summary

| ID | Invariant | Bounded Context |
|----|-----------|----------------|
| AR-INV-001 | `turn_count <= turn_budget` at all times | Agent Runtime |
| AR-INV-002 | Sub-dispatch depth is at most 1 | Agent Runtime |
| AR-INV-003 | `result_hash` set iff `status === "completed"` | Agent Runtime |
| AR-INV-004 | Turn History is append-only | Agent Runtime |
| MT-INV-001 | Tool names are globally unique across all MCP server connections | MCP Transport |
| MT-INV-002 | No `tools/call` before handshake completes | MCP Transport |
| PA-INV-001 | API key is never cached in any aggregate; read from env var at call time | Provider Abstraction |
| PA-INV-002 | Unknown model ids are rejected at adapter construction | Provider Abstraction |
| RG-INV-001 | Agent Registry is read-only after load | Agent Registry |
| RG-INV-002 | Delegation rules may only reference registered agent ids | Agent Registry |
| RG-INV-003 | Chief-analyst delegation rules reference all eight specialists | Agent Registry |
| AC-INV-001 | Audit entries are append-only; no deletion or mutation | Audit Chain |
| AC-INV-002 | Chain hash links are contiguous; any gap is detectable | Audit Chain |
| AC-INV-003 | First entry `prev_hash` is always the literal string `"genesis"` | Audit Chain |
| SM-INV-001 | Turn History only grows in a persisted archive | Session Memory |
| SM-INV-002 | `result_hash` matches SHA-256 of `final_result` when both are non-null | Session Memory |
