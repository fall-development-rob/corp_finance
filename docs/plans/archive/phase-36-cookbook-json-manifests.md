# Phase 36 — Cookbook-Style JSON Manifests

**Branch**: `phase-33-skill-driven-planning`
**Status**: Design spec — do not implement until reviewed
**Author**: Architect agent (swarm)
**Date**: 2026-05-10

---

## 0. Overview

Phase 36 adopts the cookbook agent JSON format as the canonical authoring surface for the nine CFA harness agent manifests. The YAML thin manifests at `plugins/cfa-core/agents/cfa/*.md` become auto-generated derivatives; the JSON files at the same path become the single source of truth for tool allowlists, model overrides, skill composition, and (in Phase 37) subagent pipeline topology.

Three waves:

| Wave | Scope |
|------|-------|
| W1 | JSON manifest types + JSON loader (`createDirectJsonManifestLoader`) |
| W2 | Migrate 9 agent manifests from `.md` to `.json` + auto-generate `.md` |
| W3 | Pre-commit hook to regenerate `.md` from `.json`; ADR-043; +25-30 tests |

The `AgentDef` runtime interface (`packages/harness/src/types.ts`) is unchanged. JSON manifests are the authoring surface; `AgentDef` remains the runtime surface.

---

## 1. JSON Manifest Schema

### 1a. Public types — `packages/harness/src/manifests/types.ts`

```typescript
/**
 * Top-level agent manifest — the cookbook JSON shape.
 * Canonical source: plugins/cfa-core/agents/cfa/<id>.json
 */
export interface AgentManifest {
  /** Human-readable logical name, e.g. "cfa-equity-analyst". Required. */
  name: string;
  /** Optional description forwarded to AgentDef.description. */
  description?: string;
  /** Model override; maps to AgentDef.model. */
  model?: string;
  /** Max output tokens; maps to AgentDef.maxTokens. */
  max_tokens?: number;
  /** Delegation depth cap; maps to AgentDef.maxRecursionDepth. */
  max_recursion_depth?: number;
  /** System prompt source. Exactly one of file/text should be set. */
  system?: SystemPromptConfig;
  /** Tool sets granted to this agent. Union of all blocks → AgentDef.tools. */
  tools?: ToolsetConfig[];
  /** MCP server references — used for doc/validation; not yet wired at runtime. */
  mcp_servers?: McpServerRef[];
  /** Skill references whose bodies are assembled into the system prompt. */
  skills?: SkillRef[];
  /** Subagent pipeline entries — parsed in Phase 36, wired in Phase 37. */
  callable_agents?: CallableAgentRef[];
  /** Structured output schema (optional, Phase 37 enforcement). */
  output_schema?: OutputSchema;
}

export interface SystemPromptConfig {
  /** Path to an external file; resolved relative to the manifest's directory. */
  file?: string;
  /** Inline text used directly. Mutually exclusive with file. */
  text?: string;
  /** Appended verbatim after the resolved file/text content. */
  append?: string;
}

/**
 * One toolset block. Three shapes share this interface via the `type` discriminant.
 *
 *   "agent_toolset"  — built-in Claude Code tools (read, write, glob, …)
 *   "mcp_toolset"   — all or selected tools from one named MCP server
 *   "tools_array"   — explicit bare-name array (back-compat with YAML allowlists)
 */
export type ToolsetConfig =
  | AgentToolsetConfig
  | McpToolsetConfig
  | ToolsArrayConfig;

export interface AgentToolsetConfig {
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
  /** Bare tool names — direct pass-through, back-compat with YAML `tools:` list. */
  tools: string[];
}

export interface EnabledConfig {
  enabled: boolean;
}

export interface ToolOverride {
  /** Bare tool name (without MCP prefix). */
  name: string;
  enabled: boolean;
}

export interface McpServerRef {
  type: "url" | "stdio";
  name: string;
  url?: string;    // for type "url"
  command?: string; // for type "stdio"
  args?: string[];  // for type "stdio"
}

export interface SkillRef {
  /** Basename of a directory under plugins/cfa-core/skills/cfa/. */
  from_skill: string;
}

export interface CallableAgentRef {
  /**
   * Relative path from this manifest's directory to the subagent manifest.
   * E.g. "./subagents/analyst.json"
   */
  manifest: string;
  /** Optional alias override for the delegate_to_<id> virtual tool name. */
  alias?: string;
}

export interface OutputSchema {
  type: "object";
  required?: string[];
  additionalProperties?: boolean;
  properties?: Record<string, unknown>;
}
```

### 1b. Mapping to AgentDef

| Manifest field | AgentDef field | Notes |
|---|---|---|
| `name` | `id` | Identity |
| `description` | `description` | Pass-through |
| `model` | `model` | Optional |
| `max_tokens` | `maxTokens` | Optional |
| `max_recursion_depth` | `maxRecursionDepth` | Optional |
| system prompt (assembled) | `systemPrompt` | See §2 |
| tools (projected) | `tools` | See §4 |
| `callable_agents` | `callableAgents` (new field) | See §6 |

`AgentDef` gains one new optional field in `packages/harness/src/types.ts`:

```typescript
/** Phase 36: subagent pipeline entries loaded from callable_agents[]. */
callableAgents?: AgentDef[];
```

This field is populated during loading but not consumed by the dispatch loop until Phase 37.

---

## 2. JSON Loader

### 2a. Location

New file: `packages/harness/src/manifests/json-loader.ts`

Exports one factory: `createDirectJsonManifestLoader(opts: JsonManifestLoaderOptions): JsonManifestLoader`

```typescript
export interface JsonManifestLoaderOptions {
  /** e.g. "<repo>/plugins/cfa-core/agents/cfa" */
  agentsRoot: string;
  /** For resolving skills[*].from_skill. Reuses existing SkillLoader. */
  skillLoader: SkillLoader;
}

export interface JsonManifestLoader {
  loadAgent(agentId: string): Promise<AgentDef>;
  clearCache(): void;
}
```

### 2b. Resolution rules

**`system.file`** — Resolved relative to the manifest's own directory via `resolve(dirname(manifestPath), system.file)`. The file is read as UTF-8 text. If `system.append` is also set, its content is concatenated after a `\n`.

**`system.text`** — Used verbatim. If `system.append` is also set, its content is concatenated after a `\n`.

**`skills[*].from_skill`** — Delegates to the injected `skillLoader.loadSkill(ref.from_skill)`. The resulting skill bodies are concatenated in declaration order (same semantics as the YAML `extends:` chain), then prepended before any `system.file`/`system.text` content. This matches how the existing assembler works for `extends:`.

If both `skills` and `system.file`/`system.text` are set, the order is: skill bodies first, then the system file/text, then `system.append`. This mirrors the YAML path where assembled skills precede the agent-body.

**`callable_agents[*].manifest`** — Resolved relative to the manifest's own directory. Each subagent manifest is loaded recursively via the same JSON loader. Cycle detection uses a `Set<string>` of absolute resolved paths. If a path appears twice in the call stack, an error is thrown: `Cycle detected in callable_agents: <path> → <path>`.

### 2c. Caching

In-memory `Map<string, AgentDef>` keyed by the absolute manifest path. Cache is populated on first load; `clearCache()` empties it. Tests call `clearCache()` between cases. The cache is per-loader-instance (not module-level), so multiple loaders (e.g., one per test file) do not share state.

### 2d. Error handling

- File not found: `Error("Agent manifest \"<id>\" not found. Expected file at: <path>")` — same format as the YAML loader.
- JSON parse failure: `Error("Failed to parse manifest at <path>: <message>")`.
- Missing required `name` field: `Error("Manifest at <path> is missing required field \"name\"")`.
- Cycle in `callable_agents`: `Error("Cycle detected in callable_agents: <a> → <b> → <a>")`.
- Unknown `skills[*].from_skill` reference: propagated from the skill loader unchanged.

---

## 3. Migration Strategy

### Recommendation: Option C — JSON canonical, `.md` auto-generated

**Rationale (6 lines):**
Claude Code reads `.md` agent files from `plugins/cfa-core/agents/cfa/` at session boot via the plugin install mechanism. Deleting the `.md` files (Option B) silently drops the agent from the Claude Code session without any harness error, which is the worst failure mode. A sidecar approach (Option A) creates two parallel sources of truth that must be kept in sync by hand, which is exactly the harm we reversed in Phase 33. Option C keeps the `.md` present and correct by generating it from the JSON in a pre-commit hook; the JSON is the authoritative representation and the `.md` is a build artifact. This is the same pattern used by auto-generated protobuf stubs, OpenAPI clients, and GraphQL type files: one source, one derivation, enforced at commit time.

### Generation rule

The `.md` is a thin YAML frontmatter wrapper with no body. Fields extracted from the JSON:

```markdown
---
name: <manifest.name minus "cfa-" prefix>
description: <manifest.description or "">
extends: <manifest.skills[*].from_skill joined as YAML sequence>
tools: <projected bare tool list — see §4>
model: <manifest.model or omit>
max_tokens: <manifest.max_tokens or omit>
max_recursion_depth: <manifest.max_recursion_depth or omit>
---
```

The `.md` body remains empty (the system prompt is fully assembled from `skills` and `system` fields at load time). This is the same shape as the current YAML thin manifests.

### Migration steps per agent (9 agents)

1. Create `plugins/cfa-core/agents/cfa/<id>.json` with the full cookbook format.
2. Run the generation script (shipped as W3): `scripts/gen-agent-md.ts --agent <id>` to produce the `.md`.
3. Confirm `createSkillRegistry()` still resolves all 9 agents (existing test suite).
4. Confirm `createDirectJsonManifestLoader` produces a structurally equivalent `AgentDef`.

---

## 4. Per-Tool Explicit-Enable Projection

### Algorithm

For each `McpToolsetConfig` or `AgentToolsetConfig` block in `tools[]`:

1. Obtain the full tool list for the server from the MCP client (`MCPClient.listTools()`, filtered by `mcp_server_name`). **Note**: at manifest load time the MCP client is not connected; the loader therefore produces a *static projection* using only the `configs` list, not a live server query. The live filtering happens in the agent loop (`assertAllowlistsValid`), unchanged.

2. Static projection at load time:
   - If `default_config.enabled === true` and `configs` is empty or absent: all tools from this block are included (equivalent to `"*"` for that server).
   - If `default_config.enabled === false` and `configs` is absent or empty: no tools from this block are included.
   - If `configs` is present: it is an override list. Start from `default_config.enabled`:
     - For each `{ name, enabled }` in `configs`, set the named tool's enabled flag to `enabled`, overriding the default.
     - Collect all tools where the final enabled flag is `true`.

3. **Denylist semantics** (`default_config.enabled === true` + explicit `enabled: false` entries): those named tools are excluded; all other tools from the server remain included. This is the complement of the allowlist case.

4. **Union across blocks**: the projected tool names from every toolset block are collected into one `Set<string>`, deduplicated, and sorted lexicographically. The result becomes `AgentDef.tools: string[]`.

5. **Special case — `tools_array` blocks**: bare names are added directly to the union set. This is the back-compat path for agents whose JSON was generated from a YAML `tools:` list.

6. **Special case — `agent_toolset` blocks**: the tool names (read, write, glob, bash, etc.) are treated as bare names from the built-in toolset. They project into the same union set without any server prefix.

### Example projection

```json
[
  { "type": "mcp_toolset", "mcp_server_name": "cfa-core",
    "default_config": { "enabled": false },
    "configs": [
      { "name": "wacc_calculator", "enabled": true },
      { "name": "dcf_model",       "enabled": true }
    ]
  },
  { "type": "mcp_toolset", "mcp_server_name": "fmp",
    "default_config": { "enabled": true },
    "configs": [
      { "name": "fmp_insider_latest", "enabled": false }
    ]
  }
]
```

Result at load time (static projection):
- `cfa-core` block: `["dcf_model", "wacc_calculator"]`
- `fmp` block: `["*"]` minus `fmp_insider_latest` (resolved at dispatch time when tool catalog is live)
- `AgentDef.tools`: `["dcf_model", "wacc_calculator", "*fmp*minus:fmp_insider_latest"]`

**Revised approach for mixed blocks**: To avoid a complex deferred-exclude representation, the loader uses a two-field strategy on `AgentDef`:

```typescript
// Addition to AgentDef in types.ts
toolsetsRaw?: ToolsetConfig[];  // Phase 36: preserved for live filtering at dispatch
```

The static `tools` field remains `string[] | "*"` for back-compat. When `toolsetsRaw` is set, the agent loop performs live projection against the connected tool catalog after `MCPClient.listTools()` returns, replacing the static allowlist check. This defers the "fmp all minus fmp_insider_latest" resolution to dispatch time, where the catalog is available.

For agents with only `tools_array` or `mcp_toolset` blocks where `default_config.enabled === false` (pure allowlist), the static projection is complete and `toolsetsRaw` is not needed. For agents with `default_config.enabled === true` (denylist or wildcard), `toolsetsRaw` is preserved and live-filtered at dispatch.

---

## 5. Per-Role Models

The JSON manifest's `model` field maps directly to `AgentDef.model`, which is already optional. No changes to `AgentDef` or the dispatch loop are needed: `DispatchOptions.agent.model` is already used to override `provider.turn()` per-dispatch.

For `callable_agents`, each subagent manifest has its own `model`. After recursive loading, each subagent's `AgentDef.model` carries the per-subagent model. In Phase 37, when the pipeline runner constructs `DispatchOptions` for each subagent step, it uses `subagentDef.model` (not the parent's model). This is a natural consequence of loading each subagent as a full `AgentDef`; no harness changes beyond the Phase 37 pipeline runner are needed.

For Phase 36, the only guarantee needed is that `createDirectJsonManifestLoader` correctly sets `AgentDef.model` on every loaded manifest including recursively loaded subagents.

---

## 6. Callable Agents — Phase 36 Scope

### What Phase 36 does

1. Parse `callable_agents[]` from the JSON manifest.
2. Recursively load each referenced subagent manifest via the same JSON loader.
3. Store the resulting `AgentDef[]` on the parent's new `callableAgents?: AgentDef[]` field.
4. In the dispatch loop: if `callableAgents` is populated, inject `delegate_to_<subagent.id>` virtual tools for each entry, identical to how `DispatchOptions.delegates` are handled today. This makes Phase 36 purely additive at runtime — subagents declared in `callable_agents` behave identically to delegates declared in `DispatchOptions.delegates`.

### What Phase 37 does (out of scope here)

Phase 37 introduces a sequential pipeline runner: when the chief calls `pipeline_run`, the runner invokes each subagent in `callable_agents` order, passing structured output from step N as the prompt for step N+1. The `callableAgents` field set in Phase 36 is the input to the Phase 37 runner.

### Why not wire pipeline semantics in Phase 36

The runtime semantics of "sequential pipeline with structured handoff" require changes to the agent loop (a new `pipeline` execution mode), structured output enforcement (the `output_schema` field), and a pipeline-level audit record. Shipping all three in Phase 36 would exceed its scope. The Phase 36 goal is manifest format adoption; Phase 37's goal is execution semantics.

---

## 7. ADR-043 Outline

**Title**: ADR-043: Cookbook JSON as Canonical Agent Manifest Format

**Status**: Proposed

**Date**: 2026-05-10

**Tags**: `agents`, `manifests`, `json`, `cookbook`, `phase-36`

### Context

Phase 33 (ADR-031) moved agent prose to SKILL.md files and introduced thin YAML frontmatter manifests at `plugins/cfa-core/agents/cfa/`. The YAML format carries `name`, `extends`, `tools`, `model`, `max_tokens`, `max_recursion_depth`. It has no representation for:

- Multi-skill composition (current YAML supports `extends:` but each manifest uses exactly one skill)
- Per-tool explicit-enable / denylist (YAML `tools:` is a flat allowlist)
- Per-role model override for subagents (all nine agents hard-code a single model)
- Declarative subagent pipeline topology (`callable_agents`)
- Structured output schema

The `managed-agent-cookbooks/equity-analyst/` directory already contains a working example of a richer format (agent.json + subagents/) that addresses all five gaps. The user has explicitly requested adoption of this format.

### Decision

Adopt the cookbook JSON format as the canonical authoring surface. The `.md` thin manifests are auto-generated build artifacts, regenerated by a pre-commit hook from the JSON source. Claude Code reads `.md` files at session boot; the hook ensures the `.md` stays in sync.

### Consequences (positive)

- Per-tool explicit-enable tightens the tool surface for each specialist (reduces attack surface for tool misuse).
- Per-role model allows Haiku for data-reader subagents, Sonnet for computation, Opus for chief — cost optimization without changing the harness dispatch loop.
- Multi-skill composition (`skills: [{ from_skill: X }, { from_skill: Y }]`) enables cross-domain specialists without deep `extends:` chains.
- `callable_agents` establishes the Phase 37 pipeline topology declaratively in the manifest, not in code.

### Consequences (negative)

- Pre-commit hook adds a generation step; hook failure blocks commit.
- JSON is less readable than YAML for prose-heavy manifests (mitigated: prose stays in SKILL.md; JSON only carries metadata).
- Recursive subagent loading adds loader complexity and a cycle-detection requirement.

### Alternatives considered

- **Option A (sidecar)**: Keep `.md`, add sibling `.json`. Two sources of truth; rejected.
- **Option B (replace)**: Delete `.md`, write only `.json`. Claude Code loses agents at session boot; rejected.
- **Option C (canonical JSON + auto-generated `.md`)**: Adopted. Single source of truth; derivative artifact enforced by hook.
- **Extend YAML format**: Adding `callable_agents` and per-tool configs to YAML is possible but produces a non-standard schema with no upstream tooling. The cookbook JSON format has an existing implementation in the cookbook repo; reusing it is lower risk.

---

## 8. Test Strategy

### 8a. Unit tests for `createDirectJsonManifestLoader` (~12 tests)

File: `packages/harness/src/manifests/__tests__/json-loader.test.ts`

| # | Test |
|---|------|
| 1 | Loads a minimal manifest (name + system.text only) → AgentDef with correct id, systemPrompt, tools="*" |
| 2 | Loads system.file — reads external file, assembles prompt |
| 3 | system.file + system.append — append is concatenated after file content |
| 4 | skills[] resolved via injected skill loader — bodies prepended before system.text |
| 5 | tools_array block → AgentDef.tools = that array |
| 6 | mcp_toolset with default_config.enabled=false + configs allowlist → correct projection |
| 7 | mcp_toolset with default_config.enabled=true + configs denylist → toolsetsRaw preserved |
| 8 | Multiple toolset blocks (cfa-core + fmp) → union of projected tools |
| 9 | callable_agents resolved recursively → AgentDef.callableAgents populated |
| 10 | Cycle detection in callable_agents → throws with path list |
| 11 | Missing name field → throws descriptive error |
| 12 | JSON parse failure → throws with path in message |

### 8b. Migration smoke tests — 9 agents

File: `packages/harness/src/manifests/__tests__/migration-parity.test.ts`

For each of the 9 agent ids, load from JSON and compare structural shape against the YAML loader output:

- Same `id`
- Same `model`
- Same `maxTokens`
- Same `maxRecursionDepth`
- JSON `tools` projection is a subset or equal to YAML `tools` array (JSON may be more restrictive due to explicit-enable)
- System prompt contains the same skill bodies (loaded by the same skill loader)

Note: The JSON manifests are authored to be deliberately more restrictive (per-tool explicit-enable), so byte-equality of `tools` is not asserted — structural containment is.

### 8c. Per-tool explicit-enable tests (~6 tests)

File: `packages/harness/src/manifests/__tests__/toolset-projection.test.ts`

| # | Test |
|---|------|
| 1 | default_config.enabled=false, no configs → empty tool list |
| 2 | default_config.enabled=false, configs with enabled=true entries → only those tools |
| 3 | default_config.enabled=true, no configs → toolsetsRaw preserved, tools="*" for that block |
| 4 | default_config.enabled=true, configs with enabled=false entries → toolsetsRaw preserved |
| 5 | tools_array block → plain list, no toolsetsRaw |
| 6 | Mixed blocks: allowlist block + tools_array → union |

### 8d. Schema validation tests (~3 tests)

File: `packages/harness/src/manifests/__tests__/schema.test.ts`

Validate that the 9 produced `.json` manifests and the 3 cookbook subagent manifests all pass a minimal structural check (required fields present, type discriminants valid).

**Total expected delta**: +28 to +32 tests (+12 loader, +9 migration, +6 projection, +3 schema, +2 integration).

---

## 9. Migration Plan — File Changes for W2

### New files

| File | Description |
|---|---|
| `packages/harness/src/manifests/types.ts` | AgentManifest schema types (~100 LOC) |
| `packages/harness/src/manifests/json-loader.ts` | Factory + resolution logic (~180 LOC) |
| `packages/harness/src/manifests/index.ts` | Re-export barrel (~10 LOC) |
| `packages/harness/src/manifests/__tests__/json-loader.test.ts` | 12 unit tests (~200 LOC) |
| `packages/harness/src/manifests/__tests__/migration-parity.test.ts` | 9 smoke tests (~120 LOC) |
| `packages/harness/src/manifests/__tests__/toolset-projection.test.ts` | 6 projection tests (~100 LOC) |
| `packages/harness/src/manifests/__tests__/schema.test.ts` | 3 schema tests (~60 LOC) |
| `plugins/cfa-core/agents/cfa/chief-analyst.json` | JSON manifest |
| `plugins/cfa-core/agents/cfa/equity-analyst.json` | JSON manifest |
| `plugins/cfa-core/agents/cfa/credit-analyst.json` | JSON manifest |
| `plugins/cfa-core/agents/cfa/fixed-income-analyst.json` | JSON manifest |
| `plugins/cfa-core/agents/cfa/derivatives-analyst.json` | JSON manifest |
| `plugins/cfa-core/agents/cfa/quant-risk-analyst.json` | JSON manifest |
| `plugins/cfa-core/agents/cfa/macro-analyst.json` | JSON manifest |
| `plugins/cfa-core/agents/cfa/private-markets-analyst.json` | JSON manifest |
| `plugins/cfa-core/agents/cfa/esg-regulatory-analyst.json` | JSON manifest |
| `scripts/gen-agent-md.ts` | Generator script: JSON → .md (~80 LOC) |

### Files modified

| File | Change |
|---|---|
| `packages/harness/src/types.ts` | Add `callableAgents?: AgentDef[]` and `toolsetsRaw?: ToolsetConfig[]` to `AgentDef` (~8 LOC) |
| `packages/harness/src/agents/skill-registry.ts` | Add `createJsonManifestRegistry()` factory parallel to `createSkillRegistry()` (~50 LOC) |
| `packages/harness/src/skills/index.ts` | Export `SkillLoader` (already exported; verify) |
| `plugins/cfa-core/agents/cfa/*.md` (9 files) | Regenerated by `gen-agent-md.ts` — YAML frontmatter only, tool list updated to reflect explicit-enable projection |

### Files unchanged

- `packages/harness/src/skills/loader.ts` — YAML loader unchanged
- `packages/harness/src/skills/types.ts` — unchanged
- `packages/harness/src/core/agent-loop.ts` — dispatch loop unchanged in Phase 36
- All SKILL.md skill files — unchanged

### Estimated LOC delta

| Category | +LOC | -LOC |
|---|---|---|
| New types + loader + barrel | +290 | 0 |
| New tests (4 files) | +480 | 0 |
| `types.ts` additions | +8 | 0 |
| `skill-registry.ts` addition | +50 | 0 |
| `gen-agent-md.ts` script | +80 | 0 |
| 9 JSON manifests (~60 LOC each) | +540 | 0 |
| 9 .md files regenerated (net) | ~0 | 0 |
| **Total** | **+1,448** | **0** |

The 9 YAML `.md` files are replaced by regenerated versions of similar length; no net reduction. The implementation is purely additive — no existing files are deleted or significantly reduced in Phase 36.

---

## 10. Open Questions

**Q1: Tool name prefixing in `configs[*].name`**
The cookbook subagent `analyst.json` uses full prefixed names in configs (e.g., `"mcp__cfa-core__wacc_calculator"`). The YAML manifests use bare names (e.g., `"wacc_calculator"`). The loader must normalise one or the other. Recommendation: accept both; strip the `mcp__<server>__` prefix on load so `AgentDef.tools` always carries bare names, consistent with the current harness. This aligns with `CanonicalTool.name` semantics in `types.ts`.

**Q2: `agent_toolset_20260401` vs `agent_toolset`**
The cookbook uses `"agent_toolset_20260401"` as the type discriminant (a versioned form). The schema above accepts both strings as the `AgentToolsetConfig.type`. Should Phase 36 normalise to the versioned form or accept both indefinitely? Recommendation: accept both; normalise to `"agent_toolset"` internally.

**Q3: `mcp_servers` field at runtime**
The cookbook `mcp_servers` array carries URL references with env var placeholders (e.g., `${CFA_CORE_MCP_URL}`). The harness currently wires MCP servers via `defaultMCPServers` in `registry.ts`, not from agent manifests. Phase 36 should parse and store `mcp_servers` on a new `AgentDef.mcpServers` field for future use (Phase 38 dynamic server wiring), but not wire it at runtime. Confirm scope with the user.

**Q4: `output_schema` enforcement**
The `data-reader.json` subagent carries an `output_schema` (JSON Schema). Phase 37 is the right place to enforce this (structured output via the Anthropic API's `tool_choice: required` + single-tool schema trick). Phase 36 should parse and store the schema on `AgentDef.outputSchema?: unknown` without enforcement. Confirm.

**Q5: Skill body ordering when both `skills[]` and `system.file` are set**
The recommendation in §2b is: skill bodies first, then system.file, then system.append. This matches the YAML assembler's convention (extended skill bodies precede agent body). Confirm this is the intended order for the equity-analyst manifest, where `skills` contains `corp-finance-analyst-core` + `workflow-equity-research` and `system.file` points to the existing `.claude/agents/cfa/equity-analyst.md`. If the intent is for system.file to be the primary prompt and skills to be reference material, the order should be reversed.

**Q6: Nine agents vs. subagent manifests**
The `SKILL_REGISTRY_AGENT_IDS` list covers the nine top-level analysts. The cookbook introduces a fourth tier: subagent manifests in `subagents/`. Phase 36's `createJsonManifestRegistry` should load the same 9 ids. Subagent manifests are only loaded transitively via `callable_agents`. Should the registry expose a `loadSubagent(path: string): Promise<AgentDef>` method for direct access in Phase 37? Recommendation: yes, add it in Phase 37, not Phase 36.
