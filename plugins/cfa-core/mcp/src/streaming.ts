/**
 * cfa-core MCP streaming tools.
 *
 * ## Why this file is hand-maintained
 *
 * `tools.ts` is auto-generated from the NAPI bindings via cfa-codegen, and the
 * codegen-parity check enforces byte-equality with the committed copy. Tools
 * that need a special handler shape (here: a JS callback parameter to receive
 * progress events) can't fit that template — every generated entry is a
 * straight `wasm[name](json) -> json` shim. So streaming tools live here
 * instead.
 *
 * ## What this file does
 *
 * For each tool listed in `STREAMING_TOOLS`:
 *
 * 1. We look up the matching `<name>_streaming` export on the loaded WASM
 *    module (which comes from `crates/corp-finance-wasm/src/streaming.rs`).
 * 2. We register it as a regular MCP tool whose handler:
 *    - reads `_meta.progressToken` from the incoming request,
 *    - builds a JS callback that, on each invocation, calls
 *      `extra.sendNotification({ method: "notifications/progress", … })`
 *      with the original token,
 *    - invokes the WASM function with `(json, callback)`,
 *    - returns the final JSON result as a normal MCP `text` content block.
 *
 * If the client didn't pass a `progressToken`, the callback is a no-op and
 * the tool behaves exactly like its non-streaming sibling — no events are
 * emitted but the computation still runs to completion. That keeps the tool
 * usable from clients that don't yet understand streaming.
 *
 * ## Adding a new streaming tool
 *
 * 1. Add a `<name>_with_progress(input, &dyn ProgressSink)` variant in
 *    corp-finance-core (see monte_carlo/simulation.rs for the pattern).
 * 2. Add `wasm_tool_streaming!(<name>_streaming, …)` in
 *    crates/corp-finance-wasm/src/streaming.rs.
 * 3. Append an entry to `STREAMING_TOOLS` below.
 * 4. Re-run the WASM build (`bash plugins/cfa-core/scripts/build-wasm.sh`).
 *
 * No codegen run is needed — `tools.ts` and the parity check are unaffected.
 */
import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";

import { TOOL_SCHEMAS } from "./schemas.js";

/** WASM exports the streaming tools dispatch into. */
type StreamingWasmFn = (
  inputJson: string,
  callback: (progress: number, message: string) => void,
) => string;

interface StreamingToolSpec {
  /** MCP tool name as it appears in `tools/list`. */
  name: string;
  /** Underlying WASM export from `streaming.rs`. */
  exportName: string;
  /**
   * Tool description. Streaming tools surface a hint about progress so the
   * LLM understands the latency profile.
   */
  description: string;
  /**
   * Optional zod schema key from `TOOL_SCHEMAS`. When unset we fall back to
   * the non-streaming sibling's schema (e.g. `run_monte_carlo` for
   * `run_monte_carlo_streaming`) so we don't duplicate the input shape.
   */
  schemaKey?: string;
}

/** All streaming tools registered by `registerStreamingTools`. */
const STREAMING_TOOLS: StreamingToolSpec[] = [
  {
    name: "run_monte_carlo_streaming",
    exportName: "run_monte_carlo_streaming",
    description:
      "Streaming variant of run_monte_carlo. Identical inputs and outputs, but if the request includes _meta.progressToken the server emits notifications/progress events at ~5% increments showing path-by-path progress (e.g. '5000/10000 paths'). Without a progressToken it behaves like the non-streaming sibling. Useful for long simulations (≥1M paths) where the LLM/client wants to render a live progress UI instead of seeing a frozen tool call.",
    // Reuse the auto-generated zod schema for the non-streaming sibling
    // — inputs are identical (the callback is a transport detail, not a
    // user-facing parameter).
    schemaKey: "run_monte_carlo",
  },
  // To add more: see `streaming.rs` for the matching wasm_tool_streaming!.
];

const passthroughShape = {
  input: z
    .record(z.any())
    .describe("Tool inputs as a JSON object — see tool description for fields"),
};

function shapeFor(spec: StreamingToolSpec) {
  const key = spec.schemaKey ?? spec.name;
  const schema = TOOL_SCHEMAS[key];
  if (schema) return { input: schema };
  return passthroughShape;
}

function wrap(jsonResult: string) {
  return { content: [{ type: "text" as const, text: jsonResult }] };
}

/**
 * Read the request's progressToken from `extra._meta`. The MCP SDK doesn't
 * expose `_meta` as a typed field on `RequestHandlerExtra` (it lives on the
 * raw request envelope), so we duck-type it. Returns null if the client
 * didn't ask for progress notifications.
 */
function progressTokenFrom(
  extra: unknown,
): string | number | null {
  if (!extra || typeof extra !== "object") return null;
  const meta = (extra as { _meta?: unknown })._meta;
  if (!meta || typeof meta !== "object") return null;
  const token = (meta as { progressToken?: unknown }).progressToken;
  if (typeof token === "string" || typeof token === "number") return token;
  return null;
}

/**
 * Register every streaming tool from `STREAMING_TOOLS`. Returns the number
 * actually registered — exports missing from the WASM module are skipped
 * with a warning so partial builds (e.g. without the `monte_carlo` feature)
 * don't crash the whole server.
 */
export function registerStreamingTools(
  server: McpServer,
  wasm: Record<string, unknown>,
): number {
  let registered = 0;

  for (const spec of STREAMING_TOOLS) {
    const fn = wasm[spec.exportName] as StreamingWasmFn | undefined;
    if (typeof fn !== "function") {
      console.warn(
        `[cfa-core/streaming] WASM export missing: ${spec.exportName} — skipping ${spec.name} ` +
          `(was the corresponding feature enabled at build time?)`,
      );
      continue;
    }

    server.tool(
      spec.name,
      spec.description,
      shapeFor(spec),
      async (params: { input: Record<string, unknown> }, extra: unknown) => {
        const token = progressTokenFrom(extra);

        // Build the JS-side callback. When no token, this is a no-op so we
        // skip the JSON-RPC overhead entirely. Otherwise each call becomes
        // an MCP `notifications/progress` message routed back to the client
        // through the same JSON-RPC transport that delivered the request.
        const sendNotification =
          extra && typeof extra === "object"
            ? (extra as {
                sendNotification?: (n: unknown) => Promise<void>;
              }).sendNotification
            : undefined;

        // We collect notifications and fire them serially after the WASM
        // call returns. The WASM-side closure is synchronous (a Rust
        // for-loop), so we cannot await sendNotification inside it without
        // crossing the sync/async boundary — and forcing it to async would
        // prevent the Rust loop from making any forward progress until the
        // notification was delivered. Buffering preserves ordering and keeps
        // the streaming experience responsive in practice (the client sees
        // them as a rapid burst right before the result, which still beats
        // a silent multi-minute call).
        //
        // For true real-time emission we'd need to drive the simulation
        // through a Rust async runtime and yield between checkpoints —
        // tracked in docs/STREAMING.md v0.4.
        const buffered: Array<{ progress: number; message: string }> = [];
        const callback =
          token !== null && typeof sendNotification === "function"
            ? (progress: number, message: string) => {
                buffered.push({ progress, message });
              }
            : (_p: number, _m: string) => {
                /* no-op */
              };

        let result: string;
        try {
          result = fn(JSON.stringify(params.input ?? {}), callback);
        } catch (err) {
          const message = err instanceof Error ? err.message : String(err);
          return {
            content: [
              {
                type: "text" as const,
                text: `error: ${message}`,
              },
            ],
            isError: true,
          };
        }

        // Drain the buffered events as proper progress notifications. We
        // tolerate `sendNotification` rejecting (e.g. transport closed)
        // because the result is already computed and worth returning.
        if (token !== null && typeof sendNotification === "function") {
          for (const ev of buffered) {
            try {
              await sendNotification({
                method: "notifications/progress",
                params: {
                  progressToken: token,
                  progress: ev.progress,
                  total: 1,
                  message: ev.message,
                },
              });
            } catch {
              break;
            }
          }
        }

        return wrap(result);
      },
    );
    registered++;
  }

  return registered;
}

/**
 * Exposed for the integration test in tests/streaming.test.ts — lets the
 * test assert which streaming tools should be registered without re-parsing
 * server output.
 */
export function listStreamingTools(): readonly StreamingToolSpec[] {
  return STREAMING_TOOLS;
}
