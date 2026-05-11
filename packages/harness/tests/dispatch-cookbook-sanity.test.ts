/**
 * dispatchCookbook API sanity test — Phase 33 skill-driven planning.
 *
 * Verifies the public API exists, is async, and returns a Promise.
 * Deep integration tests live in cookbook-runtime-integration.test.ts (agent B).
 */

import { describe, expect, it } from "vitest";
import { dispatchCookbook } from "../src/runtime/dispatch-cookbook.js";
import type { CookbookLoader } from "../src/manifests/cookbook-loader.js";

// Stub loader that rejects immediately — tests Promise return type
function makeRejectingLoader(): CookbookLoader {
  return {
    async list() { return []; },
    async load(slug: string) {
      throw new Error(`Cookbook not found: ${slug}`);
    },
    async loadAll() { return []; },
  };
}

describe("dispatchCookbook API surface", () => {
  it("is exported as a function", () => {
    expect(typeof dispatchCookbook).toBe("function");
  });

  it("returns a Promise when called with a rejecting loader", () => {
    const promise = dispatchCookbook({
      slug: "__nonexistent__",
      cookbookLoader: makeRejectingLoader(),
      prompt: "test",
      provider: {
        name: "anthropic" as const,
        turn: async () => ({ message: { role: "assistant" as const, content: [] }, stopReason: "end_turn" as const }),
      },
      mcp: {
        initialize: async () => {},
        listTools: async () => [],
        callTool: async () => ({}),
        close: async () => {},
      },
    });

    expect(promise).toBeInstanceOf(Promise);
    // Swallow the expected rejection (cookbook not found)
    return promise.catch(() => undefined);
  });

  it("is re-exported from the package root index", async () => {
    const mod = await import("../src/index.js");
    expect(typeof mod.dispatchCookbook).toBe("function");
  });
});
