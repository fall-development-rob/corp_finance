/**
 * Public types for Phase 36 YAML agent manifest infrastructure.
 *
 * AgentManifest is the canonical authoring surface (YAML files at
 * plugins/cfa-core/agents/cfa/<id>.yaml). AgentDef remains the runtime
 * surface; the loader bridges the two.
 *
 * from_plugin is the PRIMARY skill reference form (Anthropic canonical).
 * from_skill  is the LEGACY slug-based form (kept for backwards compat).
 */

// ---------------------------------------------------------------------------
// Top-level manifest
// ---------------------------------------------------------------------------

export interface AgentManifest {
  /** Human-readable logical name. Required. */
  name: string;
  /** Optional description forwarded to AgentDef.description. */
  description?: string;
  /** Model override; maps to AgentDef.model. */
  model?: string;
  /** Max output tokens; maps to AgentDef.maxTokens. */
  max_tokens?: number;
  /** Delegation depth cap; maps to AgentDef.maxRecursionDepth. */
  max_recursion_depth?: number;
  /** System prompt sources assembled in order. */
  system?: AgentSystem;
  /** Tool sets granted to this agent. Union of all blocks → AgentDef.tools. */
  tools?: ToolsetConfig[];
  /** MCP server references — stored on AgentDef.mcpServerRefs; not wired at runtime. */
  mcp_servers?: McpServerRef[];
  /** Skill references whose bodies are assembled into the system prompt. */
  skills?: SkillRef[];
  /** Callable subagent manifests — loaded recursively; runtime wiring is Phase 37. */
  callable_agents?: CallableAgentRef[];
  /** Structured output schema — stored on AgentDef.outputSchema; not enforced (Phase 37). */
  output_schema?: Record<string, unknown>;
}

// ---------------------------------------------------------------------------
// System prompt
// ---------------------------------------------------------------------------

export interface AgentSystem {
  /** Path to an external file; resolved relative to the manifest's directory. */
  file?: string;
  /** Inline text used directly. */
  text?: string;
  /**
   * Appended verbatim after the resolved file/text content.
   * Anthropic "headless mode" addendum slot.
   */
  append?: string;
}

// ---------------------------------------------------------------------------
// Toolset blocks
// ---------------------------------------------------------------------------

export type ToolsetConfig =
  | AgentToolsetConfig
  | McpToolsetConfig
  | ToolsArrayConfig;

export interface AgentToolsetConfig {
  /** "agent_toolset_20260401" is the versioned Anthropic form; both are accepted. */
  type: "agent_toolset" | "agent_toolset_20260401";
  default_config?: EnabledConfig;
  configs?: ToolOverride[];
}

export interface McpToolsetConfig {
  type: "mcp_toolset";
  /** Must match a name in mcp_servers[]. */
  mcp_server_name: string;
  default_config?: EnabledConfig;
  configs?: ToolOverride[];
}

export interface ToolsArrayConfig {
  type: "tools_array";
  /** Bare tool names — direct pass-through, back-compat with YAML `tools:` lists. */
  tools: string[];
}

export interface EnabledConfig {
  enabled: boolean;
}

export interface ToolOverride {
  /** Bare tool name (without MCP wire prefix). */
  name: string;
  enabled: boolean;
}

// ---------------------------------------------------------------------------
// MCP server references
// ---------------------------------------------------------------------------

export interface McpServerRef {
  type: "url" | "stdio";
  name: string;
  url?: string;
  command?: string;
  args?: string[];
}

// ---------------------------------------------------------------------------
// Skill references
// ---------------------------------------------------------------------------

export interface SkillRef {
  /**
   * PRIMARY FORM (Anthropic canonical). Path relative to the manifest file.
   * Loader resolves the path, then reads <path>/SKILL.md for the prose body.
   * Example: "../../skills/cfa/corp-finance-analyst-equity"
   */
  from_plugin?: string;
  /**
   * LEGACY FORM (slug-based). Resolves via the injected SkillLoader's loadSkill().
   * Kept for backwards compat with local cookbook JSON files.
   */
  from_skill?: string;
}

// ---------------------------------------------------------------------------
// Callable agents
// ---------------------------------------------------------------------------

export interface CallableAgentRef {
  /** Relative path from this manifest's directory to the subagent .yaml manifest. */
  manifest: string;
}
