/**
 * Deploy payload assembler — Phase 28 D1.
 *
 * Reads a YAML cookbook (agent.yaml + subagents/*.yaml + referenced skills
 * + system prompt files) and returns the structured payload an external
 * deploy CLI POSTs to Anthropic's Managed Agents API.
 *
 * Replaces two legacy paths:
 *   - scripts/deploy-managed-agent.sh (reads agent.json — legacy shape)
 *   - corp-finance-core deploy.rs (also reads agent.json)
 *
 * Both predate Phase 36's YAML migration and cannot deploy our current
 * cookbooks. This module produces the same payload shape they emitted but
 * sources its inputs from agent.yaml + subagents/*.yaml.
 *
 * Pure function — no network I/O. The CLI in scripts/deploy-cookbook.ts
 * is responsible for actually POSTing the resulting payload.
 *
 * Determinism: given the same disk state + the same env_vars map, two
 * invocations produce byte-identical output (modulo ordering inside JSON
 * objects, which we don't constrain at this layer).
 */

import { readFileSync, existsSync } from "node:fs";
import { resolve, dirname, basename, join } from "node:path";
import { parse as parseYaml } from "yaml";

import type {
  AgentManifest,
  CallableAgentRef,
  McpServerRef,
  SkillRef,
  ToolsetConfig,
} from "../manifests/types.js";

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/**
 * Skill content extracted from a referenced plugin directory. Mirrors the
 * shape the Managed Agents API expects when uploaded via /v1/skills.
 */
export interface SkillUpload {
  /**
   * Stable slug used to refer to this skill from the orchestrator manifest
   * after upload (after POSTing, the API returns a skill_id that the
   * caller patches into the manifest).
   */
  name: string;
  /** Source path that contributed this skill content (informational). */
  source_path: string;
  /** Full SKILL.md body (post system.append assembly is for the agent, not the skill). */
  content: string;
}

/**
 * A single subagent payload, ready to POST to /v1/agents.
 * The orchestrator's callable_agents[].agent_id is patched after each
 * subagent's POST returns its server-assigned id.
 */
export interface SubagentPayload {
  /** Local subagent slug — used to thread agent_ids back into the orchestrator. */
  name: string;
  /** Path the YAML was read from (informational). */
  source_manifest: string;
  /** Full POST body for /v1/agents. */
  body: Record<string, unknown>;
}

/**
 * Orchestrator (parent) payload. callable_agents[] carries placeholders
 * like `{subagent_<i>_id}` that the apply CLI substitutes once each
 * subagent has been POSTed.
 */
export interface OrchestratorPayload {
  name: string;
  source_manifest: string;
  body: Record<string, unknown>;
}

export interface DeployPayload {
  slug: string;
  /** Cookbook version from agent.yaml (Phase 25 Tier C2). */
  version: string;
  /** Audit hash echoed from caller-supplied input (informational, not gating). */
  audit_hash?: string;
  /**
   * Mirror of process.env names referenced via ${VAR} in the cookbook,
   * with the values they were substituted to. Surfacing this lets the
   * CLI confirm there are no remaining placeholders before --apply.
   */
  env_substitutions: Record<string, string>;
  /** ${VAR} placeholders that were referenced but not provided. */
  env_unresolved: string[];
  skills: SkillUpload[];
  subagents: SubagentPayload[];
  orchestrator: OrchestratorPayload;
}

export interface BuildPayloadInput {
  /** Absolute path to managed-agent-cookbooks/<slug>/. */
  cookbookDir: string;
  /** Slug override; defaults to basename(cookbookDir). */
  slug?: string;
  /**
   * Environment-variable map for ${VAR} substitution. Supply
   * process.env or a curated subset (e.g. only deploy-relevant vars).
   * Unset vars leave the placeholder in place and are recorded in
   * env_unresolved.
   */
  envVars: Record<string, string>;
  /** Optional audit hash for traceability. */
  auditHash?: string;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const ENV_PLACEHOLDER_RE = /\$\{([A-Z_][A-Z0-9_]*)\}/g;

function substituteEnv(
  raw: string,
  envVars: Record<string, string>,
  resolved: Map<string, string>,
  unresolved: Set<string>,
): string {
  return raw.replace(ENV_PLACEHOLDER_RE, (match, key: string) => {
    const v = envVars[key];
    if (v === undefined) {
      unresolved.add(key);
      return match;
    }
    resolved.set(key, v);
    return v;
  });
}

function readYamlManifest(path: string): AgentManifest {
  if (!existsSync(path)) {
    throw new Error(`manifest not found: ${path}`);
  }
  const parsed = parseYaml(readFileSync(path, "utf8")) as unknown;
  if (
    typeof parsed !== "object" ||
    parsed === null ||
    !("name" in parsed) ||
    typeof (parsed as Record<string, unknown>).name !== "string"
  ) {
    throw new Error(`manifest at ${path} missing required field "name"`);
  }
  return parsed as AgentManifest;
}

function assembleSystemPrompt(
  manifest: AgentManifest,
  manifestDir: string,
): string {
  const sections: string[] = [];
  if (manifest.system?.file) {
    const filePath = resolve(manifestDir, manifest.system.file);
    if (!existsSync(filePath)) {
      throw new Error(
        `system.file '${manifest.system.file}' resolves to missing path ${filePath}`,
      );
    }
    const body = readFileSync(filePath, "utf8").trim();
    if (body.length > 0) sections.push(body);
  }
  if (manifest.system?.text?.trim()) sections.push(manifest.system.text.trim());
  if (manifest.system?.append?.trim()) sections.push(manifest.system.append.trim());
  return sections.join("\n\n");
}

function collectSkills(
  refs: SkillRef[] | undefined,
  manifestDir: string,
): SkillUpload[] {
  if (!refs || refs.length === 0) return [];
  const out: SkillUpload[] = [];
  for (const ref of refs) {
    if (!ref.from_plugin) {
      // from_skill (legacy) is not supported by this deployer — the
      // resolved path is too dependent on a SkillLoader's slug→dir map.
      // Cookbooks use from_plugin exclusively since Phase 36.
      continue;
    }
    const skillDir = resolve(manifestDir, ref.from_plugin);
    const skillPath = join(skillDir, "SKILL.md");
    if (!existsSync(skillPath)) {
      throw new Error(
        `skill SKILL.md not found at ${skillPath} (from_plugin: ${ref.from_plugin})`,
      );
    }
    const content = readFileSync(skillPath, "utf8");
    out.push({
      name: basename(skillDir),
      source_path: skillPath,
      content,
    });
  }
  return out;
}

function toolsetForPayload(
  tools: ToolsetConfig[] | undefined,
): unknown[] | undefined {
  if (!tools) return undefined;
  // Pass through verbatim — the Managed Agents API accepts this shape.
  return tools.map((t) => ({ ...t }));
}

function mcpServersForPayload(
  refs: McpServerRef[] | undefined,
): unknown[] | undefined {
  if (!refs || refs.length === 0) return undefined;
  return refs.map((r) => ({ ...r }));
}

function buildAgentBody(
  manifest: AgentManifest,
  manifestDir: string,
  opts: { isOrchestrator: boolean },
): Record<string, unknown> {
  const body: Record<string, unknown> = {
    name: manifest.name,
    system: { text: assembleSystemPrompt(manifest, manifestDir) },
  };
  if (manifest.model) body.model = manifest.model;
  if (manifest.max_tokens !== undefined) body.max_tokens = manifest.max_tokens;
  if (manifest.max_recursion_depth !== undefined) {
    body.max_recursion_depth = manifest.max_recursion_depth;
  }

  const toolsOut = toolsetForPayload(manifest.tools);
  if (toolsOut) body.tools = toolsOut;

  const mcpOut = mcpServersForPayload(manifest.mcp_servers);
  if (mcpOut) body.mcp_servers = mcpOut;

  if (manifest.output_schema) body.output_schema = manifest.output_schema;
  if (manifest.input_schema) body.input_schema = manifest.input_schema;

  if (manifest.block_tools && manifest.block_tools.length > 0) {
    body.block_tools = manifest.block_tools;
  }

  // Skills are uploaded separately and attached after the response returns
  // a skill_id. Both the orchestrator and subagents can declare their own
  // skill set (the cfa cookbook pattern: parent has workflow-* skills,
  // subagents have corp-finance-tools-* skills). The caller patches the
  // skill_id field after each upload.
  if (manifest.skills && manifest.skills.length > 0) {
    body.skills = manifest.skills.map((s) => ({
      ...(s.from_plugin !== undefined ? { from_plugin: s.from_plugin } : {}),
      ...(s.from_skill !== undefined ? { from_skill: s.from_skill } : {}),
    }));
  }

  // callable_agents — orchestrator only; subagents have empty list per our shape.
  if (opts.isOrchestrator && manifest.callable_agents && manifest.callable_agents.length > 0) {
    body.callable_agents = manifest.callable_agents.map((ca, i) => ({
      manifest: ca.manifest,
      agent_id: `{subagent_${i}_id}`,
    }));
  }

  return body;
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Build the full deploy payload for one cookbook. Throws on missing files
 * or schema-shape errors. Two calls with the same inputs return deeply
 * equal output.
 */
export function buildDeployPayload(input: BuildPayloadInput): DeployPayload {
  const slug =
    input.slug ?? input.cookbookDir.split(/[/\\]/).filter(Boolean).pop() ?? "";

  const resolved = new Map<string, string>();
  const unresolved = new Set<string>();

  // Read agent.yaml with env-var substitution applied to the raw text first.
  const agentPath = join(input.cookbookDir, "agent.yaml");
  if (!existsSync(agentPath)) {
    throw new Error(`agent.yaml not found at ${agentPath}`);
  }
  const agentRaw = substituteEnv(
    readFileSync(agentPath, "utf8"),
    input.envVars,
    resolved,
    unresolved,
  );
  const orchestratorManifest = parseYaml(agentRaw) as AgentManifest;

  const orchestratorBody = buildAgentBody(
    orchestratorManifest,
    input.cookbookDir,
    { isOrchestrator: true },
  );

  // Skills referenced by orchestrator + every subagent. Deduped by name —
  // multiple agents in the same cookbook commonly reference the same skill
  // (e.g. corp-finance-tools-core), and the API expects each skill uploaded
  // once.
  const skillsByName = new Map<string, SkillUpload>();
  for (const s of collectSkills(orchestratorManifest.skills, input.cookbookDir)) {
    skillsByName.set(s.name, s);
  }

  // Subagents — read each callable_agents[].manifest with env-substitution.
  const subagents: SubagentPayload[] = [];
  if (orchestratorManifest.callable_agents) {
    for (const ref of orchestratorManifest.callable_agents) {
      const subPath = resolve(input.cookbookDir, ref.manifest);
      const subRaw = substituteEnv(
        readFileSync(subPath, "utf8"),
        input.envVars,
        resolved,
        unresolved,
      );
      const subManifest = parseYaml(subRaw) as AgentManifest;
      const subDir = dirname(subPath);
      const subBody = buildAgentBody(subManifest, subDir, {
        isOrchestrator: false,
      });
      subagents.push({
        name: subManifest.name,
        source_manifest: subPath,
        body: subBody,
      });
      for (const s of collectSkills(subManifest.skills, subDir)) {
        if (!skillsByName.has(s.name)) skillsByName.set(s.name, s);
      }
    }
  }
  const skills = [...skillsByName.values()].sort((a, b) =>
    a.name.localeCompare(b.name),
  );

  return {
    slug,
    version: typeof orchestratorManifest.version === "string"
      ? orchestratorManifest.version
      : "",
    ...(input.auditHash !== undefined ? { audit_hash: input.auditHash } : {}),
    env_substitutions: Object.fromEntries(resolved.entries()),
    env_unresolved: [...unresolved].sort(),
    skills,
    subagents,
    orchestrator: {
      name: orchestratorManifest.name,
      source_manifest: agentPath,
      body: orchestratorBody,
    },
  };
}

/**
 * Serialise the assembled payload as deterministic JSON. Sorted keys at
 * every object level. Trailing newline. Two calls against the same
 * payload produce byte-identical strings — used by the snapshot CI gate.
 */
export function serialiseDeployPayload(payload: DeployPayload): string {
  return (
    JSON.stringify(payload, sortedReplacer(), 2) + "\n"
  );
}

function sortedReplacer(): (key: string, value: unknown) => unknown {
  return (_key, value) => {
    if (value && typeof value === "object" && !Array.isArray(value)) {
      const obj = value as Record<string, unknown>;
      const sorted: Record<string, unknown> = {};
      for (const k of Object.keys(obj).sort()) sorted[k] = obj[k];
      return sorted;
    }
    return value;
  };
}

export function parseDeployPayload(json: string): DeployPayload {
  const parsed = JSON.parse(json) as unknown;
  if (
    typeof parsed !== "object" ||
    parsed === null ||
    !("orchestrator" in parsed) ||
    !("subagents" in parsed)
  ) {
    throw new Error("deploy payload missing required fields orchestrator/subagents");
  }
  return parsed as DeployPayload;
}
