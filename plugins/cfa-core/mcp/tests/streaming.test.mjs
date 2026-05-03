#!/usr/bin/env node
/**
 * Integration test for cfa-core MCP streaming.
 *
 * Spawns `dist/server.js` over stdio, completes the MCP handshake, then
 * issues a `tools/call` for `run_monte_carlo_streaming` with a
 * `_meta.progressToken`. Asserts that:
 *
 * 1. At least one `notifications/progress` message arrives carrying the
 *    same token before the final result.
 * 2. The progress values are non-decreasing and the last is 1.0.
 * 3. The `tools/call` reply arrives with a non-error JSON result whose
 *    `num_simulations` matches what we requested.
 *
 * Why a hand-rolled test instead of a framework: cfa-core/mcp has no test
 * harness yet (`package.json` has only `build` / `start` / `typecheck`),
 * and we don't want to introduce a Vitest/Jest dependency for a single
 * integration test. Plain Node + node:assert keeps the dependency surface
 * unchanged. Run with:
 *
 *     node plugins/cfa-core/mcp/tests/streaming.test.mjs
 *
 * (After `npx tsc` so dist/server.js exists.)
 */
import { strict as assert } from "node:assert";
import { spawn } from "node:child_process";
import { once } from "node:events";
import { fileURLToPath } from "node:url";
import path from "node:path";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const SERVER_JS = path.resolve(__dirname, "..", "dist", "server.js");

/**
 * Tiny stdio JSON-RPC client. Buffers stdout, splits on newlines, dispatches
 * either to a per-id reply waiter or to the catch-all notification listener.
 * The MCP SDK uses Content-Length-framed messages by default — but the
 * StdioServerTransport in @modelcontextprotocol/sdk just emits one JSON
 * object per line on stdout, no framing.
 */
class StdioClient {
  constructor(child) {
    this.child = child;
    this.nextId = 1;
    this.pending = new Map(); // id → {resolve, reject}
    this.notifications = []; // queue of received notifications

    let buf = "";
    child.stdout.on("data", (chunk) => {
      buf += chunk.toString("utf8");
      let nl;
      while ((nl = buf.indexOf("\n")) >= 0) {
        const line = buf.slice(0, nl).trim();
        buf = buf.slice(nl + 1);
        if (!line) continue;
        let msg;
        try {
          msg = JSON.parse(line);
        } catch (e) {
          // Non-JSON output (e.g. accidental console.log) — surface for
          // debugging but don't fail the test setup.
          process.stderr.write(`[client] non-JSON stdout: ${line}\n`);
          continue;
        }
        if (msg.id !== undefined && this.pending.has(msg.id)) {
          const waiter = this.pending.get(msg.id);
          this.pending.delete(msg.id);
          if (msg.error) waiter.reject(new Error(JSON.stringify(msg.error)));
          else waiter.resolve(msg.result);
        } else if (msg.method) {
          // Notifications and requests-from-server land here. We treat both
          // as notifications for assertion purposes (the SDK's ping &
          // related fanout don't matter for this test).
          this.notifications.push(msg);
        }
      }
    });

    child.stderr.on("data", (chunk) => {
      // Server diagnostics (the listening banner, warnings) — print so a
      // failing test gives the operator a clue.
      process.stderr.write(`[server] ${chunk}`);
    });
  }

  request(method, params, meta) {
    const id = this.nextId++;
    const msg = {
      jsonrpc: "2.0",
      id,
      method,
      params: meta ? { ...params, _meta: meta } : params,
    };
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      this.child.stdin.write(JSON.stringify(msg) + "\n");
    });
  }

  notify(method, params) {
    this.child.stdin.write(
      JSON.stringify({ jsonrpc: "2.0", method, params }) + "\n",
    );
  }

  /** Wait until at least one notification matching `pred` is queued, with timeout. */
  async waitForNotification(pred, timeoutMs = 5000) {
    const start = Date.now();
    while (Date.now() - start < timeoutMs) {
      if (this.notifications.some(pred)) return;
      await new Promise((r) => setTimeout(r, 25));
    }
    throw new Error(
      `timed out waiting for notification (queued: ${this.notifications.map((n) => n.method).join(", ")})`,
    );
  }
}

async function main() {
  console.log(`[test] spawning ${SERVER_JS}`);
  const child = spawn(process.execPath, [SERVER_JS], {
    stdio: ["pipe", "pipe", "pipe"],
  });
  const client = new StdioClient(child);

  // Surface unclean exits early.
  child.on("exit", (code, signal) => {
    if (code !== 0 && code !== null) {
      process.stderr.write(`[server] exited code=${code} signal=${signal}\n`);
    }
  });

  // 1. MCP handshake. The SDK handles the heavy lifting on the server side
  // — we just need to send `initialize` and the `notifications/initialized`
  // follow-up so the server marks the session live.
  const initResult = await client.request("initialize", {
    protocolVersion: "2024-11-05",
    capabilities: {},
    clientInfo: { name: "streaming-integration-test", version: "0.0.1" },
  });
  assert.equal(
    typeof initResult.serverInfo.name,
    "string",
    "initialize must return serverInfo.name",
  );
  console.log(
    `[test] connected to ${initResult.serverInfo.name} v${initResult.serverInfo.version}`,
  );
  client.notify("notifications/initialized", {});

  // 2. Sanity check: streaming tool is in tools/list.
  const list = await client.request("tools/list", {});
  const streamingTool = list.tools.find(
    (t) => t.name === "run_monte_carlo_streaming",
  );
  assert.ok(
    streamingTool,
    `run_monte_carlo_streaming missing from tools/list (got ${list.tools.length} tools)`,
  );
  console.log(`[test] tools/list returned ${list.tools.length} tools, streaming tool present`);

  // 3. Call run_monte_carlo_streaming with a progressToken. We pick a
  // non-trivial path count so the per-iteration sink fires several times.
  const progressToken = `mc-stream-${Date.now()}`;
  const callPromise = client.request(
    "tools/call",
    {
      name: "run_monte_carlo_streaming",
      arguments: {
        input: {
          num_simulations: 50_000,
          seed: 42,
          variables: [
            {
              name: "revenue_growth",
              distribution: { type: "Normal", mean: 0.05, std_dev: 0.02 },
            },
            {
              name: "ebitda_margin",
              distribution: { type: "Normal", mean: 0.20, std_dev: 0.03 },
            },
          ],
        },
      },
    },
    { progressToken },
  );

  // 4. Await the final response. Progress notifications are buffered and
  // flushed by the server right before the response (see streaming.ts for
  // why). After the response arrives, all notifications should be in queue.
  const callResult = await callPromise;
  assert.ok(callResult.content, "tools/call result missing content");
  assert.equal(callResult.content[0].type, "text");
  const payload = JSON.parse(callResult.content[0].text);
  assert.equal(
    payload.result.num_simulations,
    50_000,
    "result.num_simulations should round-trip",
  );
  assert.equal(
    payload.result.variables.length,
    2,
    "result.variables should have 2 entries",
  );

  // 5. Verify progress notifications arrived and are well-formed.
  const progressNotifs = client.notifications.filter(
    (n) =>
      n.method === "notifications/progress" &&
      n.params?.progressToken === progressToken,
  );
  assert.ok(
    progressNotifs.length >= 2,
    `expected ≥2 progress notifications for token=${progressToken}, got ${progressNotifs.length}`,
  );
  console.log(
    `[test] received ${progressNotifs.length} progress notifications`,
  );

  // First should be 0.0 / "starting…", last should be 1.0 / "complete".
  const first = progressNotifs[0];
  const last = progressNotifs[progressNotifs.length - 1];
  assert.equal(
    first.params.progress,
    0,
    `first progress event should be 0.0, got ${first.params.progress}`,
  );
  assert.match(first.params.message, /starting/i);
  assert.equal(
    last.params.progress,
    1,
    `last progress event should be 1.0, got ${last.params.progress}`,
  );
  assert.match(last.params.message, /complete/i);

  // Monotonicity: non-decreasing fractions.
  for (let i = 1; i < progressNotifs.length; i++) {
    assert.ok(
      progressNotifs[i].params.progress >= progressNotifs[i - 1].params.progress,
      `progress went backwards: ${progressNotifs[i - 1].params.progress} → ${progressNotifs[i].params.progress}`,
    );
  }

  // Multivariable coverage: at least one event > 0.5 (i.e. somewhere in
  // variable 2). This proves the fraction calculation spans the full
  // [0, 1] range, not just [0, 1/var_count].
  assert.ok(
    progressNotifs.some((n) => n.params.progress > 0.5),
    "expected at least one progress event > 0.5 across two variables",
  );

  // 6. Verify a call WITHOUT progressToken still succeeds (no regression
  // for clients that don't opt into streaming).
  const noTokenResult = await client.request("tools/call", {
    name: "run_monte_carlo_streaming",
    arguments: {
      input: {
        num_simulations: 1_000,
        seed: 99,
        variables: [
          {
            name: "x",
            distribution: { type: "Uniform", min: 0.0, max: 1.0 },
          },
        ],
      },
    },
  });
  const noTokenPayload = JSON.parse(noTokenResult.content[0].text);
  assert.equal(noTokenPayload.result.num_simulations, 1_000);

  // 7. Sanity: print first/last event so a developer can eyeball.
  console.log("[test] sample progress events:");
  console.log(`  [0]  ${first.params.progress.toFixed(2)} ${first.params.message}`);
  if (progressNotifs.length > 2) {
    const mid = progressNotifs[Math.floor(progressNotifs.length / 2)];
    console.log(
      `  [${Math.floor(progressNotifs.length / 2)}] ${mid.params.progress.toFixed(2)} ${mid.params.message}`,
    );
  }
  console.log(
    `  [${progressNotifs.length - 1}] ${last.params.progress.toFixed(2)} ${last.params.message}`,
  );

  console.log("[test] all assertions passed");

  // Clean shutdown.
  child.kill("SIGTERM");
  try {
    await once(child, "exit");
  } catch {
    /* already exited */
  }
}

main().catch((err) => {
  console.error("[test] FAILED:", err);
  process.exit(1);
});
