import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  managedAgentList,
  managedAgentValidate,
  managedAgentCheckAll,
  managedAgentSync,
  managedAgentDeploy,
  managedAgentOrchestrate,
} from "../bindings.js";
import {
  ManagedAgentListSchema,
  ManagedAgentValidateSchema,
  ManagedAgentCheckAllSchema,
  ManagedAgentSyncSchema,
  ManagedAgentDeploySchema,
  ManagedAgentOrchestrateSchema,
} from "../schemas/managed_agent.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerManagedAgentTools(server: McpServer) {
  server.tool(
    "managed_agent_list",
    "Enumerate CFA managed-agent cookbooks grouped by cost tier. Returns slug, tier (core_only / freemium / paid_vendor), and dependency tag for each. Lets users discover which cookbooks they can run without paying for a vendor subscription. No I/O - registry-only.",
    ManagedAgentListSchema.shape,
    async (params) => {
      const validated = ManagedAgentListSchema.parse(coerceNumbers(params));
      const result = managedAgentList(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "managed_agent_validate",
    "Schema-validate a single CFA managed-agent cookbook (no network). Checks: slug allowlist, agent.json parse, required fields, system-prompt path resolution, skill path resolution, subagent manifest existence, steering-examples.json existence. Returns per-check pass/fail with detail.",
    ManagedAgentValidateSchema.shape,
    async (params) => {
      const validated = ManagedAgentValidateSchema.parse(coerceNumbers(params));
      const result = managedAgentValidate(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "managed_agent_check_all",
    "Walk every cookbook found under cookbooks_root and run validate against each. Returns aggregate summary with passed/failed counts and per-cookbook outcome. No network. Replaces the upstream Python check.py.",
    ManagedAgentCheckAllSchema.shape,
    async (params) => {
      const validated = ManagedAgentCheckAllSchema.parse(coerceNumbers(params));
      const result = managedAgentCheckAll(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "managed_agent_sync",
    "Coverage and freshness audit between cookbooks and skills. In our pattern, skills are resolved at deploy time (no bundled copies), so this does NOT copy files - it audits which skills are referenced by which cookbooks, flags references that don't resolve to a SKILL.md (missing_skills), and flags skills on disk no cookbook references (orphan_skills).",
    ManagedAgentSyncSchema.shape,
    async (params) => {
      const validated = ManagedAgentSyncSchema.parse(coerceNumbers(params));
      const result = managedAgentSync(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "managed_agent_deploy",
    "Assemble the deployment JSON payload for a cookbook (no network). Performs ${VAR} substitution from env_vars, inlines system prompts and skill content, and returns orchestrator + subagent + skill payloads ready for the consumer to POST to /v1/agents.",
    ManagedAgentDeploySchema.shape,
    async (params) => {
      const validated = ManagedAgentDeploySchema.parse(coerceNumbers(params));
      const result = managedAgentDeploy(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "managed_agent_orchestrate",
    "Route a single handoff_request event to a target cookbook. Validates event_type and target slug against the allowlist; returns dispatch decision with full payload for accepted events.",
    ManagedAgentOrchestrateSchema.shape,
    async (params) => {
      const validated = ManagedAgentOrchestrateSchema.parse(coerceNumbers(params));
      const result = managedAgentOrchestrate(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
