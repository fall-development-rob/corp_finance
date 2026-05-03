#!/usr/bin/env node
/**
 * cfa-core MCP server.
 *
 * Loads the WASM-compiled corp-finance-core module and exposes its 244 exports
 * as MCP tools. Tool registration is delegated to the generated tools.ts so
 * adding a new export requires only re-running scripts/gen-mcp-tools.py.
 *
 * Schemas are deliberately permissive in v0.2: the Rust serde layer validates
 * each input struct and returns descriptive errors. Per-tool zod schemas with
 * autocomplete-quality field hints are tracked for v0.3 (T1.2).
 */
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { createRequire } from "node:module";

import { registerStreamingTools } from "./streaming.js";
import { registerAllTools } from "./tools.js";

const require = createRequire(import.meta.url);
const wasm = require("../wasm/corp_finance_wasm.js") as Record<
  string,
  (json: string) => string
> & { version: () => string };

async function main() {
  const server = new McpServer({
    name: "cfa-core",
    version: wasm.version(),
  });

  const count = registerAllTools(server, wasm);
  // Streaming tools live in a separate hand-maintained module so
  // tools.ts (auto-generated) stays untouched. See streaming.ts for the
  // progressToken → notifications/progress wiring.
  const streamingCount = registerStreamingTools(
    server,
    wasm as Record<string, unknown>,
  );

  const transport = new StdioServerTransport();
  await server.connect(transport);
  console.error(
    `cfa-core MCP server v${wasm.version()} listening — ${count} tools registered ` +
      `(+${streamingCount} streaming)`,
  );
}

main().catch((err) => {
  console.error("cfa-core MCP fatal error:", err);
  process.exit(1);
});
