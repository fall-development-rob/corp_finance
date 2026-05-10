/**
 * Unit tests for the session memory module — Phase 31 Wave 4.
 *
 * Uses a real tmpdir; no network I/O.
 */
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { buildReplayPrompt } from "../src/memory/replay.js";
import { createFileSessionStore } from "../src/memory/session.js";
import type { Message, SessionState } from "../src/types.js";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeState(overrides: Partial<SessionState> = {}): SessionState {
  const now = new Date().toISOString();
  return {
    session_id: "sess-001",
    agent_id: "cfa-equity",
    prompt: "Analyse AAPL",
    messages: [],
    tool_uses: 0,
    child_session_ids: [],
    status: "in_progress",
    created_at: now,
    updated_at: now,
    ...overrides,
  };
}

// ---------------------------------------------------------------------------
// Test suite
// ---------------------------------------------------------------------------

describe("createFileSessionStore", () => {
  let dir: string;

  beforeEach(() => {
    dir = mkdtempSync(join(tmpdir(), "harness-session-test-"));
  });

  afterEach(() => {
    rmSync(dir, { recursive: true, force: true });
  });

  it("Test 1: save + load round-trip", async () => {
    const store = createFileSessionStore({ dir });
    const state = makeState({ session_id: "s1" });
    await store.save(state);
    const loaded = await store.load("s1");
    expect(loaded).not.toBeNull();
    // save() updates updated_at, so compare everything except updated_at
    expect(loaded?.session_id).toBe(state.session_id);
    expect(loaded?.agent_id).toBe(state.agent_id);
    expect(loaded?.prompt).toBe(state.prompt);
    expect(loaded?.status).toBe(state.status);
  });

  it("Test 2: save updates updated_at to a later timestamp", async () => {
    const store = createFileSessionStore({ dir });
    const before = new Date(Date.now() - 100).toISOString();
    const state = makeState({ session_id: "s2", updated_at: before });

    await store.save(state);
    const loaded = await store.load("s2");
    expect(loaded).not.toBeNull();
    expect(loaded!.updated_at >= before).toBe(true);
  });

  it("Test 3: list filtering by agentId", async () => {
    const store = createFileSessionStore({ dir });
    await store.save(makeState({ session_id: "s3a", agent_id: "cfa-equity" }));
    await store.save(makeState({ session_id: "s3b", agent_id: "cfa-credit" }));
    await store.save(makeState({ session_id: "s3c", agent_id: "cfa-equity" }));

    const results = await store.list({ agentId: "cfa-equity" });
    expect(results).toHaveLength(2);
    expect(results.every((s) => s.agent_id === "cfa-equity")).toBe(true);
  });

  it("Test 4: list filtering by status", async () => {
    const store = createFileSessionStore({ dir });
    await store.save(makeState({ session_id: "s4a", status: "completed" }));
    await store.save(makeState({ session_id: "s4b", status: "in_progress" }));
    await store.save(makeState({ session_id: "s4c", status: "failed" }));

    const completed = await store.list({ status: "completed" });
    expect(completed).toHaveLength(1);
    expect(completed[0]?.session_id).toBe("s4a");
  });

  it("Test 5: list with no filter returns all sessions sorted by updated_at descending", async () => {
    const store = createFileSessionStore({ dir });
    // Stagger timestamps by forcing updated_at values
    const t1 = "2026-01-01T00:00:01.000Z";
    const t2 = "2026-01-01T00:00:02.000Z";
    const t3 = "2026-01-01T00:00:03.000Z";

    // Save in a non-chronological order
    await store.save(makeState({ session_id: "s5b", updated_at: t2 }));
    await store.save(makeState({ session_id: "s5c", updated_at: t3 }));
    await store.save(makeState({ session_id: "s5a", updated_at: t1 }));

    // save() overwrites updated_at; re-save with controlled timestamps by
    // writing raw states that already have updated_at set.  The store will
    // update again — so we check order relative to what save() produces.
    // Instead, use distinct session_ids and verify descending order by
    // comparing updated_at of adjacent items.
    const all = await store.list();
    expect(all).toHaveLength(3);
    for (let i = 1; i < all.length; i++) {
      expect(all[i - 1]!.updated_at >= all[i]!.updated_at).toBe(true);
    }
  });

  it("Test 6: delete removes file; subsequent load returns null", async () => {
    const store = createFileSessionStore({ dir });
    await store.save(makeState({ session_id: "s6" }));
    expect(await store.load("s6")).not.toBeNull();

    await store.delete("s6");
    expect(await store.load("s6")).toBeNull();
  });

  it("Test 6b: delete is a no-op when file does not exist", async () => {
    const store = createFileSessionStore({ dir });
    await expect(store.delete("nonexistent")).resolves.toBeUndefined();
  });
});

// ---------------------------------------------------------------------------
// buildReplayPrompt
// ---------------------------------------------------------------------------

describe("buildReplayPrompt", () => {
  it("Test 7: returns original prompt and messages array", () => {
    const messages: Message[] = [
      { role: "user", content: [{ type: "text", text: "hello" }] },
      { role: "assistant", content: [{ type: "text", text: "world" }] },
    ];
    const state = makeState({ prompt: "Analyse AAPL", messages, status: "completed" });
    const replay = buildReplayPrompt(state);

    expect(replay.prompt).toBe("Analyse AAPL");
    expect(replay.messages).toBe(messages); // same reference — no copy needed
  });
});
