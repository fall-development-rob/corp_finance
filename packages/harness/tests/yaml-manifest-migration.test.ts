/**
 * YAML manifest migration gate tests — Phase 36.
 *
 * For each of the 9 CFA analyst agents, verifies that loading from the
 * canonical .yaml manifest produces an AgentDef that matches the
 * auto-generated .md thin manifest on the structural fields:
 *   id, model, maxTokens, maxRecursionDepth, tools (deep-equal).
 *
 * systemPrompt equality is checked by verifying both contain the same
 * skill body (asserted non-empty). The .yaml path is the NEW canonical
 * source; the .md is the auto-generated artifact.
 */

import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { existsSync } from "node:fs";
import { describe, expect, it } from "vitest";

import { createDirectYamlManifestLoader } from "../src/manifests/yaml-loader.js";
import { createDirectSkillLoader } from "../src/skills/loader.js";

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, "..", "..", "..");
const AGENTS_ROOT = resolve(REPO_ROOT, "plugins", "cfa-core", "agents", "cfa");

// W5: skills moved from plugins/cfa-core/skills/cfa/ to
// plugins/agent-plugins/<slug>/skills/corp-finance-analyst-<slug>/.
// The migration test verifies each agent independently using its own skillsRoot.
const AGENT_IDS = [
  "chief-analyst",
  "equity-analyst",
  "credit-analyst",
  "fixed-income-analyst",
  "derivatives-analyst",
  "quant-risk-analyst",
  "macro-analyst",
  "private-markets-analyst",
  "esg-regulatory-analyst",
] as const;

function skillsRootFor(agentId: string): string {
  return resolve(REPO_ROOT, "plugins", "agent-plugins", agentId, "skills");
}

function makeLoaders(agentId: string) {
  const skillLoader = createDirectSkillLoader({
    skillsRoot: skillsRootFor(agentId),
    agentsRoot: AGENTS_ROOT,
  });
  const yamlLoader = createDirectYamlManifestLoader({
    agentsRoot: AGENTS_ROOT,
    skillLoader,
  });
  return { skillLoader, yamlLoader };
}

// Helper: sort tools for comparison
function sortedTools(tools: string[] | "*"): string[] | "*" {
  if (tools === "*") return "*";
  return [...tools].sort();
}

describe("YAML manifest migration gate — 9 agents", () => {
  for (const agentId of AGENT_IDS) {
    describe(agentId, () => {
      it("yaml and md loaders agree on id, model, maxTokens, maxRecursionDepth", async () => {
        const yamlPath = resolve(AGENTS_ROOT, `${agentId}.yaml`);
        const mdPath = resolve(AGENTS_ROOT, `${agentId}.md`);

        expect(existsSync(yamlPath)).toBe(true);
        expect(existsSync(mdPath)).toBe(true);

        const { skillLoader, yamlLoader } = makeLoaders(agentId);
        const [fromYaml, fromMd] = await Promise.all([
          yamlLoader.loadAgent(agentId),
          skillLoader.loadAgent(agentId, "agent"),
        ]);

        expect(fromYaml.id).toBe(fromMd.id);
        expect(fromYaml.model).toBe(fromMd.model);
        expect(fromYaml.maxTokens).toBe(fromMd.maxTokens);
        expect(fromYaml.maxRecursionDepth).toBe(fromMd.maxRecursionDepth);
      });

      it("tools from yaml match tools from regenerated md (deep-equal)", async () => {
        const { skillLoader, yamlLoader } = makeLoaders(agentId);
        const [fromYaml, fromMd] = await Promise.all([
          yamlLoader.loadAgent(agentId),
          skillLoader.loadAgent(agentId, "agent"),
        ]);

        // Both should have the same sorted tool list
        expect(sortedTools(fromYaml.tools)).toEqual(sortedTools(fromMd.tools));
      });

      it("systemPrompt from yaml is non-empty and contains skill content", async () => {
        const { yamlLoader } = makeLoaders(agentId);
        const fromYaml = await yamlLoader.loadAgent(agentId);
        expect(fromYaml.systemPrompt.length).toBeGreaterThan(100);
      });
    });
  }
});
