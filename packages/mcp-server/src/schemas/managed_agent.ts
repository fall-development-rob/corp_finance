import { z } from "zod";

export const ManagedAgentListSchema = z.object({
  tier: z
    .enum(["core_only", "freemium", "paid_vendor"])
    .optional()
    .describe(
      "Filter by cost tier. core_only: cfa-core-only, no data feeds. freemium: cfa-core + free public data and/or FMP free tier. paid_vendor: requires LSEG/S&P/FactSet/etc. paid subscription."
    ),
});

export const ManagedAgentValidateSchema = z.object({
  slug: z
    .string()
    .describe("Cookbook slug (e.g. equity-analyst, lseg-rates-monitor)"),
  cookbooks_root: z
    .string()
    .describe("Absolute path to managed-agent-cookbooks/ directory"),
  skills_root: z
    .string()
    .describe("Absolute path to .claude/skills/ directory"),
  agents_root: z
    .string()
    .describe("Absolute path to .claude/agents/cfa/ directory"),
});

export const ManagedAgentCheckAllSchema = z.object({
  cookbooks_root: z
    .string()
    .describe("Absolute path to managed-agent-cookbooks/ directory"),
  skills_root: z
    .string()
    .describe("Absolute path to .claude/skills/ directory"),
  agents_root: z
    .string()
    .describe("Absolute path to .claude/agents/cfa/ directory"),
});

export const ManagedAgentSyncSchema = z.object({
  cookbooks_root: z
    .string()
    .describe("Absolute path to managed-agent-cookbooks/ directory"),
  skills_root: z
    .string()
    .describe("Absolute path to .claude/skills/ directory"),
});

export const ManagedAgentDeploySchema = z.object({
  slug: z
    .string()
    .describe("Cookbook slug to assemble payload for"),
  cookbooks_root: z.string(),
  skills_root: z.string(),
  agents_root: z.string(),
  env_vars: z
    .record(z.string())
    .describe("Environment-variable substitutions for ${VAR} placeholders"),
});

export const ManagedAgentOrchestrateSchema = z.object({
  event: z
    .object({
      event_type: z.string(),
      target: z.string(),
      payload: z.record(z.unknown()),
    })
    .describe("Single handoff_request event to route"),
  allowlist: z
    .array(z.string())
    .optional()
    .describe(
      "Allowed target slugs. Defaults to ALLOWED_SLUGS in managed_agent::types if omitted."
    ),
});
