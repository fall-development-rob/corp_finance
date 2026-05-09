# ADR-034: Reuse Existing Plugin MCP Servers Without Modification

## Status: Accepted

## Date: 2026-05-09

## Deciders

- Robert Fall
- CFA Agent platform engineering

## Tags

`mcp`, `plugins`, `stdio`, `json-rpc`, `cfa-core`, `cfa-data`, `cfa-pro`, `phase-31`

## Context

Phase 29 and Phase 30 together produced four production-grade plugin MCP servers that expose the complete CFA analyst tool surface:

| Plugin | Tools | Technology |
|---|---|---|
| `plugin:cfa-core:cfa-core` | 227 | TypeScript + WASM (`corp-finance-core` 1.1.0) |
| `plugin:cfa-data:data` | 129 | TypeScript (FRED, EDGAR, FIGI, YF, WB, geopolitical) |
| `plugin:cfa-pro:fmp-market-data` | 180 | TypeScript (FMP REST API) |
| `plugin:cfa-pro:vendor` | 87 | TypeScript (LSEG, S&P, FactSet, Morningstar, Moody's, PitchBook) |
| **Total** | **623** | |

These servers are installed as Claude Code plugins (`claude mcp list` shows all four `✓ Connected`) and have been proven correct via two independent paths:

1. **Claude Code general-purpose subagent**: `tool_uses: 22` on the Chaco Minerals memo, producing rust_decimal-precision warrant prices. The general-purpose agent invokes the plugin servers via Claude Code's internal MCP dispatch.

2. **Custom MCP JSON-RPC client** (`/tmp/mcp-chaco-demo.mjs`): 21 real `tools/call` invocations completed against the plugin servers via stdio, producing numerically matching results (per-warrant C$0.0208 at C$0.10 strike, 80% vol). This script demonstrates that the plugin servers accept standard JSON-RPC 2.0 over stdio from any client, not only from Claude Code.

The harness (ADR-031) needs exactly these 623 tools. Phase 29/30 invested approximately 800 LOC in schema auto-generation, WASM port, surface parity gates, and MCP registration to produce the current tool surface. There is no deficit in the tool layer. The deficit is in the dispatch layer (the broken cfa-* Claude Code subagent), which is the concern of ADR-031.

Creating new MCP servers for Phase 31 would duplicate an investment that is already complete, introduce a second canonical tool definition surface that could drift from the plugin servers, and delay the Wave 1 acceptance test.

## Decision

The harness connects to the four existing plugin MCP servers as JSON-RPC 2.0 clients over stdio. No new MCP servers are created in Phase 31.

### Connection pattern

`packages/mcp-client/stdio.ts` spawns each plugin server as a child process and communicates via its stdin/stdout:

```typescript
// packages/mcp-client/stdio.ts (illustrative shape)

import { spawn } from "node:child_process";
import { createInterface } from "node:readline";

export interface McpClient {
  listTools(): Promise<McpTool[]>;
  callTool(name: string, input: Record<string, unknown>): Promise<unknown>;
}

export async function connectStdio(
  command: string,
  args: string[]
): Promise<McpClient> {
  const proc = spawn(command, args, { stdio: ["pipe", "pipe", "inherit"] });
  // MCP handshake: initialize → initialized
  // then expose listTools() and callTool()
  return buildClient(proc);
}
```

The four servers are started once at harness initialisation and kept alive for the duration of the session. `listTools()` is called once per server at startup; results are merged into the full 623-tool list. Tool routing (ADR-031) dispatches each `tool_use` block to the server that owns that tool name.

### Tool name routing

Each plugin server owns a disjoint set of tool names. The harness maintains a routing table built at startup from `tools/list` responses:

```
cfa-core:    option_pricer, implied_volatility, wacc_calculator, ...  (227 tools)
cfa-data:    fred_series, edgar_filings, yf_quote, ...                (129 tools)
fmp:         fmp_quote, fmp_income_statement, ...                      (180 tools)
vendor:      factset_prices, lseg_bond_pricing, ...                    (87 tools)
```

When the model returns a `tool_use` block with `name: "option_pricer"`, the tool router looks up `"option_pricer"` in the routing table, finds it belongs to the `cfa-core` client, and issues a `tools/call` to that client.

### Plugin server command resolution

The plugin servers are Claude Code plugins installed at the user scope. Their entry points are discoverable from `~/.claude.json` (the `mcpServers` registry). The harness reads this file at startup to resolve the `command` and `args` for each plugin server, avoiding hardcoded paths.

### No new MCP servers in Phase 31

All 623 tools are already implemented and tested. The harness does not add new tools in Phase 31. If new tools are needed in a future phase, they are added to the appropriate existing plugin server (following the Phase 29/30 patterns), not by creating a new server.

## Consequences

### Positive

- Zero new MCP server code in Phase 31: the tool surface is complete. Wave 1 can focus entirely on the dispatch loop, which is the actual gap.
- The plugin servers are already tested via the Claude Code general-purpose agent and the `/tmp/mcp-chaco-demo.mjs` proof-of-concept. There are no unknowns in the tool layer.
- The stdio transport is the simplest MCP transport: no network, no authentication, no port conflicts. It is reliable in a single-machine context and matches how Claude Code itself connects to plugin servers.
- Tool definitions (via `tools/list`) flow from the servers to the harness at runtime. There is no hand-maintained tool schema in the harness codebase — the servers remain the single source of truth.
- Phase 32+ can add remote MCP transports (SSE, HTTP) without changing the plugin servers; only `packages/mcp-client` gains new transport implementations.

### Negative

- Spawning four child processes at harness startup adds ~200ms initialisation latency. This is acceptable for a batch analysis tool (the Chaco memo takes seconds, not milliseconds) but would be a concern for a low-latency API.
- The harness depends on the plugin servers being installed (`claude mcp list` shows them connected). A developer without the plugins installed cannot run the harness. Installation instructions must be documented.
- If a plugin server crashes mid-session, the harness must detect the dead child process and either restart it or return a tool error. Error handling for crashed MCP server processes is a Wave 1 deliverable.
- `~/.claude.json` is an undocumented Claude Code internal format; reading it to resolve plugin entry points couples the harness to a Claude Code implementation detail. If this file moves or changes format, the path resolution breaks. A fallback configuration (`packages/cli/plugin-paths.ts`) should allow manual override.

### Neutral

- The 623-tool count will grow in future phases as new tools are added to the plugin servers. The harness discovers tools dynamically via `tools/list` and does not need to be updated when new tools are added.
- Wave 3 adds SSE and HTTP transports to `packages/mcp-client` for remote plugin server scenarios. These are additive; the stdio transport remains in place for local use.

## Alternatives Considered

**Build a new unified MCP server for Phase 31** — Rejected. Creating a new server would duplicate the 227 + 129 + 180 + 87 tools already implemented, tested, and surface-parity-gated in Phases 29/30. It would introduce a second source of truth for tool definitions and create a maintenance burden without adding capability. The existing servers are the correct target.

**Embed the WASM plugin directly (call `corp-finance-core` WASM from the harness without MCP)** — Rejected for `cfa-core`. The WASM plugin provides the financial compute, but the three data/market-data servers (`cfa-data`, `fmp-market-data`, `vendor`) have no WASM equivalent — they are REST API clients. A hybrid approach (WASM for compute, MCP for data) would require maintaining two different tool-invocation code paths and would not exercise the proven MCP client pattern from `/tmp/mcp-chaco-demo.mjs`. Uniformity via MCP is preferred.

**HTTP-over-MCP from the start** — SSE/HTTP transports support remote plugin servers (useful for multi-machine deployments). Deferred to Wave 3. Stdio is sufficient for Wave 1 (all four plugin servers run locally), simpler to implement correctly, and eliminates network-related test flakiness in the acceptance test.

**Import plugin server packages directly as npm modules (no child process)** — The plugin servers expose standard `@modelcontextprotocol/sdk` Server instances. In principle they could be imported and called in-process. Rejected: the plugin servers may have incompatible singleton state (e.g., the cfa-core WASM runtime is initialised once per process). Running them in separate processes is the standard deployment model and avoids in-process coupling.

## References

- Master plan: `docs/plans/phase-31-harness.md`
- ADR-031: Custom dispatch harness (establishes that the MCP servers are the compute layer)
- ADR-032: TypeScript + Node.js (the `spawn` and `readline` interfaces used in stdio.ts)
- ADR-033: Multi-provider abstraction (tool definitions from `tools/list` become `CanonicalTool[]`)
- Phase 29: WASM port and plugin extraction (origin of the 227-tool cfa-core plugin)
- Phase 30: Script-to-tool migration (origin of the Phase 30 tool additions)
- Proof-of-concept: `/tmp/mcp-chaco-demo.mjs` (21 `tools/call` invocations via stdio)
- ADR-026: Plugin/Packages dual-mode and surface parity (the gate that validates the 227-tool surface)
- ADR-027: WASM port strategy (background on the cfa-core WASM plugin architecture)
- MCP specification: https://spec.modelcontextprotocol.io
