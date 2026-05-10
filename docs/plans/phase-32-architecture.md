# Phase 32 — Monorepo Architecture Refactor

**Status:** Planning (awaiting approval)
**Date:** 2026-05-10
**Owner:** Robert Fall
**Predecessors:** Phase 31 (CFA Harness Waves 1–4), Phase 30 (script-to-tool consolidation), Phase 29 (corp-finance-core extraction).

## Executive summary

Four parallel review swarms (duplication audit, hex architecture, DDD bounded contexts, monorepo structure) converge on a consistent diagnosis:

1. **Real duplication is contained** (~470 LOC across 5 patterns) and concentrated in MCP-server boilerplate (`wrapResponse` × 52), MCP-client transport adapters (`toCanonicalTool` × 3, `unwrapMcpContent` × 3), and specialist prompt preambles (~200 LOC × 8 agents).
2. **Layering is broken in 5 specific places** — most importantly, `agent-loop.ts` instantiates `createAnthropicProvider` directly, breaking the Provider port abstraction; `cli/run.ts` is both inbound adapter and composition root; `Provider.name` encodes vendor identity in the domain type.
3. **Bounded contexts have drifted** — the Phase 31 DDD doc and `domain-orchestration.md` describe overlapping models (BC1 Analysis Orchestration vs Multi-Agent Coordination). Six additional contexts exist outside the harness (Compute Core, Tool Catalog, Agent Persona, Workflow/Deal, Distribution, Schema Generation). The hardest boundary is Tool Catalog ↔ Harness — there's no ACL validating that `AgentDef.tools` allowlists reference real registered tool names. This is the failure class behind the Phase 31 `tool_uses=0` incidents.
4. **Monorepo physical structure has 4 concrete pain points**: tripled MCP boilerplate (`packages/*` source + `plugins/*/mcp/` dist copies + separate `node_modules`), orphaned lockfile in `packages/harness/`, `plugins/cfa-core/mcp/` outside the workspace, mixed npm scope (`@robotixai/*` and unscoped `cfa-core-mcp`).

The proposal below combines all four reviews into a 7-wave refactor over ~4 weeks. Wave 1 (lockfile + ACL) is zero-risk and ships value immediately; later waves are progressively higher risk and can be re-scoped independently.

## Target architecture

### Target package layout

```
packages/
├── cfa-domain/                 @cfa/domain         private  ZERO deps
│   src/
│   ├── tool.ts                 CanonicalTool, ToolCall, ToolResult
│   ├── agent.ts                AgentDef
│   ├── message.ts              Message, ContentBlock, Role
│   ├── audit.ts                AuditRecord, AuditToolCall (data only)
│   ├── session.ts              SessionState (data only)
│   └── security.ts             KeyScope, matchesPattern (pure)
│
├── cfa-application/            @cfa/application    private  depends: cfa-domain
│   src/
│   ├── ports/                  Provider, MCPClient, AuditSink, SessionStore (port interfaces)
│   ├── dispatch.ts             agent-loop use case — NO concrete imports
│   ├── tool-router.ts
│   ├── tool-schema.ts          filterToolsForAgent + ToolCatalogValidator (NEW ACL)
│   └── delegate.ts             pure delegation policy
│
├── cfa-agents/                 @cfa/agents         private  depends: cfa-domain
│   src/
│   ├── chief-analyst.ts
│   ├── specialists/            8 specialists
│   │   ├── shared-prompt.ts    NEW: SPECIALIST_PREAMBLE + TRACEABILITY_TABLE_FOOTER
│   │   ├── equity.ts ... esg-regulatory.ts
│   └── registry.ts             Pure Map<id, AgentDef> — no IO
│
├── cfa-adapters-llm/           @cfa/adapters-llm   private  depends: cfa-application + SDKs
│   src/{anthropic,openai,gemini}.ts
│
├── cfa-adapters-mcp/           @cfa/adapters-mcp   private  depends: cfa-application
│   src/
│   ├── stdio.ts
│   ├── sse.ts
│   ├── http.ts
│   ├── name-resolver.ts        NEW central exports: toCanonicalTool, unwrapMcpContent
│   └── shared/                 NEW: connection scaffold extracted from 3 transports
│
├── cfa-adapters-fs/            @cfa/adapters-fs    private  depends: cfa-application
│   src/
│   ├── file-json-store.ts      NEW: shared atomic-JSON-write primitive
│   ├── file-audit.ts           AuditSink built on file-json-store
│   └── file-session.ts         SessionStore built on file-json-store
│
├── cfa-composition/            @cfa/composition    private  depends: all adapters + agents
│   src/
│   ├── default-mcp-servers.ts  Path detection (moved from registry.ts)
│   ├── env-scope.ts            scopeForAgent ↔ process.env (moved from security/)
│   └── container.ts            buildContainer(opts) → DispatchOptions
│
├── cfa-cli/                    @cfa/cli            public   depends: cfa-composition
│   src/run.ts                  argv parsing + buildContainer(args) + dispatch()
│
├── cfa-mcp-base/               @cfa/mcp-base       private  shared MCP server scaffold
│   src/
│   ├── server.ts               StdioServerTransport bootstrap
│   ├── response.ts             wrapResponse — single canonical export (replaces 52 copies)
│   └── error.ts
│
├── cfa-mcp-core/               @cfa/mcp-core       private  227 cfa-core compute tools
│   (packages/mcp-server/ + plugins/cfa-core/mcp/src/ merged)
│
├── cfa-mcp-data/               @cfa/mcp-data       private  129 free-data tools
├── cfa-mcp-fmp/                @cfa/mcp-fmp        private  180 FMP tools
└── cfa-mcp-vendor/             @cfa/mcp-vendor     private  87 paid-vendor tools

plugins/                        Manifests + symlinks ONLY
├── cfa-core/.claude-plugin/plugin.json     → @cfa/mcp-core/dist/server.js
├── cfa-data/.claude-plugin/plugin.json     → @cfa/mcp-data/dist/index.js
└── cfa-pro/.claude-plugin/plugin.json      → @cfa/mcp-fmp + @cfa/mcp-vendor

crates/
└── corp-finance-bindings/      Unchanged — NAPI cdylib for the external corp-finance-core crate
```

11 packages total. Naming convention: `@cfa/<role>` (single scope, no `@robotixai/` mixed in). All packages private except `@cfa/cli` and `@cfa/agents` (the public surface).

### Hex architecture rules

- **Domain core (`cfa-domain`)**: pure types and pure pattern predicates. Zero npm dependencies. Compile-enforced via tsconfig path constraints.
- **Application (`cfa-application`)**: use cases + port interfaces. Depends on `cfa-domain` only. May NOT import any SDK or any `cfa-adapters-*` package.
- **Agents (`cfa-agents`)**: pure data definitions of system prompts and tool allowlists. Depends on `cfa-domain` only.
- **Adapters (`cfa-adapters-*`)**: concrete implementations of application ports. Each adapter package depends on `cfa-application` (for the port) plus its specific SDK or Node primitive.
- **Composition (`cfa-composition`)**: the only place that knows which adapter implements which port. Wires everything for a given runtime configuration.
- **Inbound adapters (`cfa-cli`, future HTTP server, etc.)**: depend on `cfa-composition` and call `dispatch()`. Carry no business logic.

### Bounded context discipline

8 bounded contexts, each formally mapped to a package or module group:

| Context | Package(s) | Notes |
|---|---|---|
| Compute Core | `crates/corp-finance-bindings` + external `corp-finance-core` | NAPI boundary is the ACL |
| Tool Catalog | `cfa-mcp-{base,core,data,fmp,vendor}` | Single Open Host, four tier-specific tool-set servers |
| Analysis Orchestration | `cfa-application` + `cfa-agents` | The harness's dispatch use case |
| Persistence (audit + memory) | `cfa-adapters-fs` | **MERGED** — Phase 31 separated audit+memory but the sink+store implementations are structurally identical |
| Provider Abstraction | `cfa-adapters-llm` | One adapter per provider |
| MCP Transport | `cfa-adapters-mcp` | One adapter per transport |
| Distribution | `plugins/*` (manifests only) | Marketplace + Claude Code plugin format |
| Workflow / Deal Documents | `.claude/commands/`, `managed-agent-cookbooks/` | Stay declarative; not packaged |

Agent Persona (the `.claude/agents/cfa/*.md` files) is **retired as a runtime artefact** in Phase 32 — `cfa-agents` is the source of truth. The `.md` files become read-only documentation pointers.

Learning & Adaptation (Phase 31 BC4) stays unimplemented and is removed from the DDD doc until there's code to back it.

### The new ACL — `ToolCatalogValidator`

The hardest boundary today is Tool Catalog ↔ Harness. `AgentDef.tools` allowlists are arbitrary strings; if a tool is renamed in an MCP server, the agent silently gets an empty toolset and `tool_uses=0` is the runtime symptom.

**Phase 32 ships a startup-time ACL** in `cfa-application/src/tool-schema.ts`:

```typescript
export interface ToolCatalogValidationIssue {
  agent_id: string;
  unknown_tool: string;
}

export function validateAllowlists(
  agentDefs: AgentDef[],
  catalog: Set<string>,
): { ok: boolean; issues: ToolCatalogValidationIssue[] };
```

`buildContainer()` calls this after `mcp.initialize()` and before the first dispatch. Result: a renamed tool produces a loud startup failure (`Unknown tool "fmp_quotee" in agent equity-analyst allowlist`) instead of a silent runtime failure 90 seconds into the dispatch loop.

This is a 50-LOC fix that closes the failure class documented in `feedback_mcp_registration_check.md` and `feedback_cfa_agents_cannot_invoke_tools.md`.

## Wave plan

### Wave 1 — Zero-risk fixes (1 day)

1. **Delete `packages/harness/package-lock.json`**, run `npm install` from root, commit a single root lockfile. Add CI assertion: no workspace member may carry a `package-lock.json`.
2. **Add `ToolCatalogValidator`** in the existing harness `core/tool-schema.ts`. Wire into `dispatch()` after `mcp.listTools()`. Add a unit test that fails when `AgentDef.tools` contains an unknown name.
3. **Promote `wrapResponse`** to a single shared module (`packages/mcp-server/src/formatters/response.ts` is already canonical — re-export it from a small `@cfa/mcp-utils` placeholder package, update the 52 importers via codemod). LOC saved: ~150.

Net: ~250 LOC removed, one whole class of silent failure converted to a startup error, CI lockfile failure permanently fixed.

### Wave 2 — Adapter-side consolidation (2 days)

4. **Centralize MCP transport helpers** in `packages/harness/src/mcp-client/name-resolver.ts`: add `toCanonicalTool` + `unwrapMcpContent`. Update `stdio.ts`, `sse.ts`, `http.ts` to import from `name-resolver.ts`. LOC saved: ~54 + closes `.find()` vs `[0]` divergence bug.
5. **Extract `FileJsonStore<T>` primitive** in `packages/harness/src/audit/` and consume from `audit/chain.ts` and `memory/session.ts`. Both stores now share atomic-write/list/filter/delete logic. LOC saved: ~80.
6. **Extract `SPECIALIST_PREAMBLE` + `TRACEABILITY_TABLE_FOOTER`** templates in `packages/harness/src/agents/specialists/shared-prompt.ts`. Each specialist interpolates rather than copying. LOC saved: ~200. Future-strengthening edits to the governance prose become 1-file changes.

Net: ~334 LOC removed, three well-known maintenance hotspots de-risked.

### Wave 3 — Hex layer extraction (1 week)

7. Create `cfa-domain/` package — move pure types from `harness/src/types.ts`. Rename `Provider.name: "anthropic" | "openai" | ...` → `Provider.name: string` (Open/Closed fix).
8. Create `cfa-application/` package — move `dispatch.ts`, `tool-router.ts`, `tool-schema.ts`, `delegate.ts`, port interfaces. Remove the `createAnthropicProvider` import from `dispatch()`.
9. Create `cfa-composition/` package — move `defaultMCPServers` + `existsSync` plumbing out of `registry.ts`; move `process.env` reads out of `security/key-scoping.ts`. Add `buildContainer(opts)`.
10. Create `cfa-cli/` package — `run.ts` shrinks to argv parsing + `buildContainer(args)` + `dispatch()`.

`cfa-agents/`, `cfa-adapters-llm/`, `cfa-adapters-mcp/`, `cfa-adapters-fs/` packages emerge from relocations, not rewrites — pure file moves with import-path updates.

Net: 4 violations from the Review-B hex audit closed. All existing tests move unchanged (the pure-function unit tests don't care about packaging).

### Wave 4 — MCP server unification (3 days)

11. Create `@cfa/mcp-base` — extract the shared MCP server scaffold (StdioServerTransport bootstrap, Zod wiring, error handling) currently duplicated across the four MCP servers. Each `@cfa/mcp-{core,data,fmp,vendor}` becomes a thin tool-list package on top of `mcp-base`.
12. Merge `plugins/cfa-core/mcp/src/server.ts` into `@cfa/mcp-core` (currently the only plugin server with its own source).

### Wave 5 — Plugin packaging (2 days)

13. Replace `plugins/cfa-data/mcp/`, `plugins/cfa-pro/mcp/`, `plugins/cfa-pro/mcp/vendors/` (currently dist-copies with separate `node_modules` and `package-lock.json`) with **symlinks** into `packages/cfa-mcp-*/dist/`. Plugin manifests reference the symlink paths.
14. Delete plugin-local `node_modules` and `package-lock.json`. Plugins become pure manifest + symlink artefacts.

### Wave 6 — Naming consistency + Turbo graph (1 day)

15. Rename `@robotixai/*` → `@cfa/*` across all packages. Update all internal imports.
16. Add explicit `@cfa/*` deps in each `package.json` so Turbo's `dependsOn: ["^build"]` sees the full build graph.
17. Replace hardcoded `/home/robert/corp-finance-core` paths in `mcp-server/package.json schemas:gen:*` with `${COREREPO}` env var.

### Wave 7 — DDD docs + ADRs (1 day)

18. Update `docs/ddd/` to reflect Phase 32 truth: 8 contexts, no Phase 31 BC4 (Learning), audit+memory merged, validateAllowlists ACL documented.
19. Write ADR-038 through ADR-044 for the structural decisions (one ADR per major architectural choice).
20. Retire vestigial `.claude/agents/cfa/*.md` runtime artefacts; replace with pointers to `@cfa/agents/src/{chief,specialists/}`.

## Success metrics

| Metric | Pre-Phase-32 | Target |
|---|---|---|
| `wrapResponse` declarations | 52 | 1 |
| Provider.name domain coupling | 4 vendor literals | string |
| Application-layer SDK imports | 1 (createAnthropicProvider in dispatch) | 0 |
| Lockfiles | 2 (root + harness) | 1 |
| Plugin-local `node_modules`/`package-lock.json` | 4 | 0 |
| Tool-name silent-failure surface | unbounded | 0 (validated at startup) |
| Specialist prompt boilerplate | 200 LOC × 8 = 1600 | 25 LOC × 1 + 8 imports |
| Layering violations from Review B | 5 | 0 |
| LOC eliminated | — | ~580 |

## Risk register

| Risk | Mitigation |
|---|---|
| Wave 3 import-path explosion breaks all tests | Land Waves 1–2 first; they buy time and confidence. Wave 3 lands behind a feature branch with full CI green before merge. |
| Plugin symlinks break Claude Code plugin loader on Windows | Test on macOS + Linux; if Windows is needed, switch to a build step that copies to `dist/` instead of symlinking. |
| `@cfa/*` rename breaks downstream consumers (regfin, lexius future integration) | Coordinate the rename via a deprecation window; keep `@robotixai/*` as a re-export for one minor version. |
| Wave 4 `mcp-base` extraction destabilises live MCP servers | Each MCP server keeps its current behavior; `mcp-base` is a non-breaking opt-in until all four migrate. |
| Wave 7 ADR drift | Write ADRs as a single PR after Waves 1–6 land, so they describe shipped reality, not aspiration. |

## Open questions

1. **`@cfa/*` vs `@robotixai/*`** — which scope wins? Both are in use today. Pick one and commit.
2. **Where do `.claude/agents/cfa/*.md` go?** Retire entirely, or keep as documentation under `docs/personas/`?
3. **Should the workflow + cookbook context (`managed-agent-cookbooks/`, slash commands) become a package or stay as raw declarative artefacts?** Phase 32 leaves it unpackaged; revisit if a programmatic consumer emerges.
4. **Anthropic/OpenAI/Gemini SDK pinning** — current pins are loose (`^0.30.1` etc). Tighten on package extraction?

## Cross-references

- Review reports (this plan synthesises): code-duplication audit, hex architecture mapping, DDD bounded-context review, monorepo structure review.
- Existing planning artefacts: `docs/plans/phase-31-harness.md`, `docs/ddd/phase-31-harness.md`, `docs/prd/phase-31-harness.md`, `docs/contracts/feature_harness_*.yml`.
- Related ADRs: ADR-031 through ADR-037 (Phase 31). Phase 32 ADRs (-038…-044) ship in Wave 7.
- Memory notes that Phase 32 closes: `feedback_mcp_registration_check.md`, `feedback_cfa_agents_cannot_invoke_tools.md` (the validateAllowlists ACL is the structural fix).
