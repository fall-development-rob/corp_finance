/**
 * callable_agents parsing and runtime depth tests — Phase 36.
 *
 * Verifies that the YAML loader correctly parses and stores callable_agents
 * without wiring pipeline semantics (that is Phase 37).
 */

import {
  mkdtempSync,
  mkdirSync,
  writeFileSync,
  rmSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { createDirectYamlManifestLoader } from "../src/manifests/yaml-loader.js";
import type { SkillLoader } from "../src/skills/types.js";
import type { ParsedSkill } from "../src/skills/types.js";

let tmpDir: string;

beforeEach(() => {
  tmpDir = mkdtempSync(join(tmpdir(), "callable-agents-test-"));
});

afterEach(() => {
  rmSync(tmpDir, { recursive: true, force: true });
});

function writeFile(relPath: string, content: string): void {
  const full = join(tmpDir, relPath);
  mkdirSync(full.replace(/\/[^/]+$/, ""), { recursive: true });
  writeFileSync(full, content, "utf-8");
}

const emptySkillLoader: SkillLoader = {
  async loadSkill(id: string): Promise<ParsedSkill> {
    throw new Error(`Skill "${id}" not found`);
  },
  async loadAgent() {
    throw new Error("not used");
  },
  clearCache() {},
};

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("callable_agents — Phase 36 scope", () => {
  it("parses and stores callable_agents from YAML manifest", async () => {
    writeFile(
      "agents/sub.yaml",
      ["name: sub-agent", "system:", "  text: Sub."].join("\n"),
    );
    writeFile(
      "agents/parent.yaml",
      [
        "name: parent-agent",
        "system:",
        "  text: Parent.",
        "callable_agents:",
        "  - manifest: ./sub.yaml",
      ].join("\n"),
    );
    const loader = createDirectYamlManifestLoader({
      agentsRoot: join(tmpDir, "agents"),
      skillLoader: emptySkillLoader,
    });
    const parent = await loader.loadAgent("parent");
    const ca = (parent as any).callableAgents as { id: string }[];
    expect(Array.isArray(ca)).toBe(true);
    expect(ca).toHaveLength(1);
    expect(ca[0]!.id).toBe("sub-agent");
  });

  it("depth=0 check: callable_agents stored but not dispatched by loader", async () => {
    // The loader only stores; dispatching is Phase 37. We verify the field exists.
    writeFile(
      "agents/leaf.yaml",
      ["name: leaf-agent", "system:", "  text: Leaf."].join("\n"),
    );
    writeFile(
      "agents/root.yaml",
      [
        "name: root-agent",
        "max_recursion_depth: 0",
        "system:",
        "  text: Root.",
        "callable_agents:",
        "  - manifest: ./leaf.yaml",
      ].join("\n"),
    );
    const loader = createDirectYamlManifestLoader({
      agentsRoot: join(tmpDir, "agents"),
      skillLoader: emptySkillLoader,
    });
    const root = await loader.loadAgent("root");
    // maxRecursionDepth is 0 on this agent
    expect(root.maxRecursionDepth).toBe(0);
    // callable_agents still populated (runtime decision is separate)
    const ca = (root as any).callableAgents;
    expect(ca).toBeDefined();
    expect(ca).toHaveLength(1);
  });

  it("loads multiple callable_agents in order", async () => {
    for (const sub of ["alpha", "beta", "gamma"]) {
      writeFile(
        `agents/${sub}.yaml`,
        [`name: ${sub}-agent`, "system:", `  text: ${sub}.`].join("\n"),
      );
    }
    writeFile(
      "agents/chief.yaml",
      [
        "name: chief-agent",
        "system:",
        "  text: Chief.",
        "callable_agents:",
        "  - manifest: ./alpha.yaml",
        "  - manifest: ./beta.yaml",
        "  - manifest: ./gamma.yaml",
      ].join("\n"),
    );
    const loader = createDirectYamlManifestLoader({
      agentsRoot: join(tmpDir, "agents"),
      skillLoader: emptySkillLoader,
    });
    const chief = await loader.loadAgent("chief");
    const ca = (chief as any).callableAgents as { id: string }[];
    expect(ca.map((a) => a.id)).toEqual(["alpha-agent", "beta-agent", "gamma-agent"]);
  });

  it("cycle detection: A → B → A throws with path info", async () => {
    writeFile(
      "agents/alpha.yaml",
      [
        "name: alpha",
        "system:",
        "  text: A.",
        "callable_agents:",
        "  - manifest: ./beta.yaml",
      ].join("\n"),
    );
    writeFile(
      "agents/beta.yaml",
      [
        "name: beta",
        "system:",
        "  text: B.",
        "callable_agents:",
        "  - manifest: ./alpha.yaml",
      ].join("\n"),
    );
    const loader = createDirectYamlManifestLoader({
      agentsRoot: join(tmpDir, "agents"),
      skillLoader: emptySkillLoader,
    });
    await expect(loader.loadAgent("alpha")).rejects.toThrow(/Cycle detected/);
  });

  it("in-memory cache: second load returns same object reference", async () => {
    writeFile(
      "agents/cached.yaml",
      ["name: cached-agent", "system:", "  text: Cached."].join("\n"),
    );
    const loader = createDirectYamlManifestLoader({
      agentsRoot: join(tmpDir, "agents"),
      skillLoader: emptySkillLoader,
    });
    const first = await loader.loadAgent("cached");
    const second = await loader.loadAgent("cached");
    expect(first).toBe(second);
  });
});
