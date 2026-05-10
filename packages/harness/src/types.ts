/**
 * Shared types for the CFA Harness — Phase 31 Wave 1.
 *
 * These interfaces define the contracts between the four bounded contexts
 * (Agent Runtime, MCP Transport, Provider Abstraction, Agent Registry).
 * Implementations live in their respective directories under src/.
 */
import type { ReasoningBank } from "./reasoning/bank.js";

// ---------------------------------------------------------------------------
// Tool model — Anthropic-flavored canonical shape
// ---------------------------------------------------------------------------

/**
 * Canonical tool definition. Anthropic-shaped: provider adapters translate
 * to OpenAI / Gemini formats at the wire boundary (Wave 3).
 *
 * `name` is the **bare** name (e.g., "option_pricer"). Wire-prefix translation
 * (e.g., "mcp__plugin_cfa-core_cfa-core__option_pricer") happens inside the
 * MCP client, never in the agent registry or provider layer.
 */
export interface CanonicalTool {
  name: string;
  description: string;
  input_schema: {
    type: "object";
    properties: Record<string, unknown>;
    required?: string[];
  };
}

/**
 * A single tool invocation request from the model.
 */
export interface ToolCall {
  id: string;
  name: string; // bare name
  input: Record<string, unknown>;
}

/**
 * Result of a tool invocation, returned to the model in the next turn.
 */
export interface ToolResult {
  call_id: string;
  content: unknown; // structured (object, array, primitive) or string
  is_error: boolean;
}

// ---------------------------------------------------------------------------
// MCP transport
// ---------------------------------------------------------------------------

/**
 * Configuration for one MCP server connection.
 *
 * `name` is the logical/local identifier (e.g., "cfa-core"). `prefix` is the
 * wire-format tool-name prefix the server uses (e.g.,
 * "mcp__plugin_cfa-core_cfa-core__"). The MCP client uses `prefix` to
 * translate bare tool names to wire names at the JSON-RPC boundary.
 */
export interface MCPServerConfig {
  name: string;
  prefix: string;
  command: string;
  args: string[];
  env?: Record<string, string>;
}

/**
 * MCP client interface. The harness connects to N servers and presents a
 * unified bare-name tool surface to the agent loop.
 */
export interface MCPClient {
  /** Spawn server processes, perform handshakes, populate tool catalog. */
  initialize(): Promise<void>;
  /** Return all tools across all connected servers, keyed by bare name. */
  listTools(): Promise<CanonicalTool[]>;
  /** Invoke a tool by bare name; client resolves to wire name internally. */
  callTool(bareName: string, input: Record<string, unknown>): Promise<unknown>;
  /** Shut down server processes. */
  close(): Promise<void>;
}

// ---------------------------------------------------------------------------
// Agent registry
// ---------------------------------------------------------------------------

/**
 * Definition of an agent — system prompt + tool subset + recursion policy.
 * Defined as code in src/agents/registry.ts (per ADR-037).
 */
export interface AgentDef {
  id: string;
  description: string;
  systemPrompt: string;
  /**
   * Allowlist of bare tool names this agent can invoke. The string `"*"`
   * grants access to every tool returned by `MCPClient.listTools()`.
   */
  tools: string[] | "*";
  /**
   * Maximum recursion depth for chief→specialist delegation. 0 (default)
   * means the agent cannot delegate. Phase 31 caps depth at 1.
   */
  maxRecursionDepth?: number;
  /** Override the default model for this agent. */
  model?: string;
  /** Override the default max output tokens. */
  maxTokens?: number;
}

// ---------------------------------------------------------------------------
// Provider abstraction
// ---------------------------------------------------------------------------

export type Role = "user" | "assistant";

export interface TextBlock {
  type: "text";
  text: string;
}

export interface ToolUseBlock {
  type: "tool_use";
  id: string;
  name: string; // bare name
  input: Record<string, unknown>;
}

export interface ToolResultBlock {
  type: "tool_result";
  tool_use_id: string;
  content: string;
  is_error?: boolean;
}

export type ContentBlock = TextBlock | ToolUseBlock | ToolResultBlock;

export interface Message {
  role: Role;
  content: ContentBlock[];
}

export interface ProviderTurnRequest {
  systemPrompt: string;
  messages: Message[];
  tools: CanonicalTool[];
  model?: string;
  maxTokens?: number;
}

export interface ProviderTurnResponse {
  /** The assistant message produced this turn. */
  message: Message;
  /**
   * Reason the turn ended. `tool_use` means there are tool_use blocks the
   * caller must execute and feed back; `end_turn` means the dispatch loop
   * is complete.
   */
  stopReason: "end_turn" | "tool_use" | "max_tokens" | "stop_sequence";
  usage?: {
    inputTokens: number;
    outputTokens: number;
  };
}

export interface Provider {
  readonly name: "anthropic" | "openai" | "gemini" | "bedrock";
  /** Run one Messages-API turn. */
  turn(request: ProviderTurnRequest): Promise<ProviderTurnResponse>;
}

// ---------------------------------------------------------------------------
// Agent loop / dispatch
// ---------------------------------------------------------------------------

export interface DispatchOptions {
  agent: AgentDef;
  prompt: string;
  /** Provider to dispatch through (default: Anthropic). */
  provider?: Provider;
  /** MCP client for tool routing. */
  mcp: MCPClient;
  /** Maximum number of model turns before forcing end_turn (safety). */
  maxTurns?: number;
  /** Current recursion depth; chief→specialist increments this. */
  depth?: number;
  /** Optional handler for streaming events (for CLI output, audit logs). */
  onEvent?: (event: DispatchEvent) => void;
  /**
   * Specialist agents this dispatch can delegate to. The agent loop injects
   * a `delegate_to_<id>` virtual tool for each entry; when the model calls
   * one, the loop runs a nested dispatch on the specialist with the
   * sub-prompt as input. Phase 31 caps recursion at depth 1 (chief →
   * specialist; specialists do not delegate further).
   */
  delegates?: AgentDef[];
  /** Wave 4: audit sink. If provided, every dispatch produces an AuditRecord. */
  audit?: AuditSink;
  /** Wave 4: session store. If provided, dispatch state is persisted on completion. */
  session?: SessionStore;
  /** Wave 4: parent audit id, set by the agent loop on nested dispatches. */
  parentAuditId?: string;
  /**
   * Phase 34 Wave 2: reasoning bank. If provided alongside `audit`, every
   * successful dispatch fire-and-forgets a `ReasoningEntry` derived from the
   * audit record + prompt + finalText. Index failures emit a
   * `reasoning_index_failed` DispatchEvent but never fail the dispatch.
   */
  reasoning?: ReasoningBank;
}

export type DispatchEvent =
  | { type: "turn_started"; turn: number; agentId: string }
  | { type: "tool_call"; call: ToolCall; turn: number }
  | { type: "tool_result"; result: ToolResult; turn: number }
  | { type: "delegation"; targetAgent: string; subPrompt: string }
  | { type: "turn_completed"; turn: number; stopReason: string }
  | { type: "dispatch_completed"; toolUses: number }
  | { type: "reasoning_index_failed"; reason: string; audit_id: string };

export interface DispatchResult {
  finalText: string;
  toolUses: number;
  messages: Message[];
  /** Per-agent dispatches recorded if delegation occurred. */
  childDispatches?: DispatchResult[];
  /** Audit record id if an audit sink was provided. */
  auditId?: string;
  /** Session id if a session store was provided. */
  sessionId?: string;
}

// ---------------------------------------------------------------------------
// Audit chain (Wave 4)
// ---------------------------------------------------------------------------

export interface AuditToolCall {
  id: string;
  name: string;             // bare tool name
  input_hash: string;       // sha256 of JSON-stringified input
  result_hash: string;      // sha256 of result content
  is_error: boolean;
  duration_ms: number;
}

export interface AuditRecord {
  audit_id: string;
  parent_audit_id?: string;
  agent_id: string;
  prompt_hash: string;      // sha256 of prompt
  tool_calls: AuditToolCall[];
  result_hash: string;      // sha256 of finalText
  duration_ms: number;
  total_tool_uses: number;
  child_audit_ids: string[]; // delegation children
  timestamp: string;        // ISO 8601
  model?: string;
  usage?: { inputTokens: number; outputTokens: number };
}

export interface AuditSink {
  /** Persist a completed audit record. */
  write(record: AuditRecord): Promise<void>;
  /** Retrieve a previously-written record by id. */
  read(auditId: string): Promise<AuditRecord | null>;
  /** List records, optionally filtered. */
  list(filter?: { agentId?: string; since?: Date }): Promise<AuditRecord[]>;
}

// ---------------------------------------------------------------------------
// Session memory (Wave 4)
// ---------------------------------------------------------------------------

export interface SessionState {
  session_id: string;
  agent_id: string;
  prompt: string;
  messages: Message[];
  tool_uses: number;
  child_session_ids: string[];
  final_text?: string;
  status: "in_progress" | "completed" | "failed";
  audit_id?: string;
  created_at: string;
  updated_at: string;
  /** Optional structured metadata for the caller to attach. */
  metadata?: Record<string, unknown>;
}

export interface SessionStore {
  save(state: SessionState): Promise<void>;
  load(sessionId: string): Promise<SessionState | null>;
  list(filter?: { agentId?: string; status?: SessionState["status"] }): Promise<SessionState[]>;
  delete(sessionId: string): Promise<void>;
}

// ---------------------------------------------------------------------------
// API-key scoping (Wave 4)
// ---------------------------------------------------------------------------

export interface KeyScope {
  /** Env var name (e.g. "FMP_API_KEY"). */
  env_var: string;
  /**
   * Bare tool-name patterns this key applies to. Patterns are simple prefix
   * matches against the tool's bare name; e.g. "fmp_*" matches every FMP
   * tool. The literal "*" matches all tools.
   */
  tools: string[];
}

export interface ScopedEnv {
  /**
   * Compute the env vars to expose for an agent given its allowlist. Keys
   * whose `tools` patterns don't intersect the allowlist are dropped.
   * Returns a Record suitable for passing to MCPServerConfig.env.
   */
  scopeForAgent(agentTools: string[] | "*"): Record<string, string>;
}
