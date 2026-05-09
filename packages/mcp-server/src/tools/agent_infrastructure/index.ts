/**
 * Phase 30 Wave 1 — Agent-infrastructure MCP tools.
 *
 * Registers four tools that replace the prior imperative .mjs scripts with
 * agent-callable, structured-I/O MCP tools:
 *
 *   - surface_parity_check    (replaces check-surface-parity.mjs + lib/tool-extract.mjs)
 *   - schemas_refresh         (replaces gen-*-schemas.mjs ×6 + lib/transform-zod.mjs)
 *   - cookbook_validate_all   (replaces scripts/test-cookbooks.sh)
 *   - wasm_build              (replaces plugins/cfa-core/scripts/build-wasm.sh)
 *
 * Each tool exposes a JSON-in / JSON-out boundary; the underlying TypeScript
 * implementations are pure and unit-tested in their respective `*.test.ts`
 * files.
 */

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { wrapResponse } from "../../formatters/response.js";

import { surfaceParityCheckMcp } from "./surface_parity.js";
import { schemasRefreshMcp } from "./zod_transform.js";
import { cookbookValidateAllMcp } from "./cookbook.js";
import { wasmBuildMcp } from "./wasm_build.js";

// ---------------------------------------------------------------------------
// Input Zod schemas (hand-written; small enough that schema_gen is overkill)
// ---------------------------------------------------------------------------

const SurfaceParityCheckSchema = z.object({
  workspace_root: z.string().optional(),
  allowlist_path: z.string().optional(),
});

const SchemasRefreshSchema = z.object({
  domain: z.enum([
    "office",
    "attest",
    "memory",
    "audit",
    "cost",
    "observability",
    "all",
  ]),
  schema_src_root: z.string().optional(),
  schema_out_root: z.string().optional(),
});

const CookbookValidateAllSchema = z.object({
  workspace_root: z.string().optional(),
  cookbooks_root: z.string().optional(),
  skills_root: z.string().optional(),
  agents_root: z.string().optional(),
  cfa_binary: z.string().optional(),
  slugs: z.array(z.string()).optional(),
});

const WasmBuildSchema = z.object({
  corerepo: z.string().optional(),
  plugin_wasm_dir: z.string().optional(),
  workspace_root: z.string().optional(),
  skip_build: z.boolean().optional(),
});

// ---------------------------------------------------------------------------
// Tool registration
// ---------------------------------------------------------------------------

export function registerAgentInfrastructureTools(server: McpServer) {
  server.tool(
    "surface_parity_check",
    "Audit MCP tool drift between packages/mcp-server (NAPI) and plugins/cfa-core/mcp (WASM). Returns structured categorisation: shared, allowlisted, plugin_only, packages_only. drift_detected=true iff any tool exists in NAPI but neither WASM nor the allowlist. Replaces check-surface-parity.mjs.",
    SurfaceParityCheckSchema.shape,
    async (params) => {
      const result = await surfaceParityCheckMcp(JSON.stringify(params));
      return wrapResponse(result);
    },
  );

  server.tool(
    "schemas_refresh",
    "Regenerate Zod TypeScript schemas from JSON Schema sources emitted by corp-finance-core's schema_gen tests. Domain ∈ {office, attest, memory, audit, cost, observability, all}. Reads JSON Schema from $COREREPO/crates/corp-finance-core/target/schemas/<domain>/, writes Zod TS to packages/mcp-server/src/schemas/generated/<domain>/. Post-processes z.any().superRefine blocks into z.discriminatedUnion / z.enum. Replaces gen-*-schemas.mjs and lib/transform-zod.mjs.",
    SchemasRefreshSchema.shape,
    async (params) => {
      const result = await schemasRefreshMcp(JSON.stringify(params));
      return wrapResponse(result);
    },
  );

  server.tool(
    "cookbook_validate_all",
    "Validate every managed-agent cookbook under managed-agent-cookbooks/ via the cfa CLI: schema/reference validation + dry-run deploy payload assembly. Returns per-cookbook results with all_ok aggregate. Requires cfa binary in PATH (install via `cargo install --git https://github.com/rob-otix-ai/corp-finance-core --tag v1.1.0 corp-finance-cli --locked`). Replaces scripts/test-cookbooks.sh.",
    CookbookValidateAllSchema.shape,
    async (params) => {
      const result = await cookbookValidateAllMcp(JSON.stringify(params));
      return wrapResponse(result);
    },
  );

  server.tool(
    "wasm_build",
    "Orchestrate WASM build in the extracted corp-finance-core repo and copy artefacts to plugins/cfa-core/mcp/wasm/. Returns artefact paths, byte sizes, and sha256 hashes. skip_build=true only copies pre-built artefacts. Replaces plugins/cfa-core/scripts/build-wasm.sh.",
    WasmBuildSchema.shape,
    async (params) => {
      const result = await wasmBuildMcp(JSON.stringify(params));
      return wrapResponse(result);
    },
  );
}
