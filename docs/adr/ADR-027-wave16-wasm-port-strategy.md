# ADR-027: Wave-16 WASM Port Strategy

## Status: Accepted

## Date: 2026-05-08

## Deciders

- CFA Agent platform engineering
- Plugin (WASM) surface owner
- MCP tool surface owner
- CFA specialist agent system owner

## Tags

wasm, napi, wave-16, port-strategy, surface-parity, batch-migration, bundle-size, wasm-bindgen

## Context

Wave 15 (commit 8dda27b) shipped the dual-mode architecture gate defined in ADR-026. The surface-parity invariant is now enforced at CI via `packages/mcp-server/scripts/check-surface-parity.mjs`. The allowlist documents which tools are NAPI-only and why.

The current state of the two surfaces following the Wave-15 gate:

- **Plugin surface** (`plugins/cfa-core/mcp/`): 6 tools registered — `wacc_calculator`, `dcf_model`, `comps_analysis`, `credit_metrics`, `debt_capacity`, `covenant_compliance` — all backed by `crates/corp-finance-wasm`.
- **Packages surface** (`packages/mcp-server/`): approximately 277 tools registered.
- **Wave-16 backlog**: 237 tools identified as pure-math tools that are portable to WASM in principle. These appear in `.surface-allowlist.json` with a temporary `napi_only` reason indicating they have not been ported yet, not that they cannot be.
- **Permanent carve-outs**: 34 tools require native facilities (federation keypairs via ed25519-dalek with OS entropy, SQLite trajectory storage via rusqlite, HNSW mmap indexing, office OOXML native compression, audit middleware, cost tracking, self-learning loop). These will remain NAPI-only indefinitely.

Wave 16a is a pilot sub-wave: one Rust module is ported end-to-end by a parallel agent to validate the mechanic and surface structural blockers. Subsequent sub-waves (16c, 16d, ...) will port the remaining modules in batches until the allowlist is reduced to only the 34 permanent carve-outs plus any modules that hit structural blockers.

The motivations for proceeding now rather than deferring further:

1. CFA subagents using the plugin surface get fewer than 3% of available tools (6 of ~277). The practical utility of the plugin deployment is severely limited until parity is approached.
2. Cross-platform deployment — the primary rationale for the WASM surface — is blocked at scale by the port backlog.
3. Wave 15 established all the enforcement infrastructure (allowlist, audit script, agent-frontmatter validation). The porting work can now be mechanical rather than architectural.

## Decision

### 1. Port in batches by Rust module, not individually

A Rust module (`mod`) is the smallest unit of port work because it has consistent feature gating, shared internal types, and a coherent set of `wasm_bindgen` exports. Porting module-by-module keeps each sub-wave diff reviewable and keeps the allowlist diff clean. Individual tool-by-tool porting is rejected because it produces hundreds of micro-commits with no structural coherence and makes blockers harder to attribute.

Each sub-wave (16c, 16d, ...) targets exactly one module. Sub-waves run sequentially to avoid merge conflicts on `server.ts` and the allowlist.

### 2. Commit message format

Every port sub-wave commit follows this exact format:

```
feat(phase-29-wave-16x): port <module> to WASM (N tools)

Tools ported:
- tool_name_1
- tool_name_2
...
```

The parenthetical wave tag (16c, 16d, ...) increments with each sub-wave. The tool list in the body is the audit trail linking commit to allowlist removals.

### 3. Bundle size tracking

The `.wasm` bundle size is recorded before and after each port sub-wave using `ls -la plugins/cfa-core/wasm/*.wasm`. The delta is captured in the sub-wave commit body or an accompanying note.

If total bundle size grows past **5 MB**, the team evaluates splitting into per-module `.wasm` files with lazy loading in the plugin server. The evaluation must produce a written decision (a brief addendum to this ADR or a new ADR superseding it) before the next sub-wave proceeds. The 5 MB threshold is chosen because it is well above the estimated maximum single-module size (200 KB–2 MB) but below a range that causes measurable plugin startup latency in Claude Code.

### 4. Stop conditions per port attempt

If a module fails to compile to `wasm32-unknown-unknown` after **3 distinct compile attempts** with different mitigation strategies, the module is declared a structural blocker:

- The port attempt is abandoned for that sub-wave.
- A `wasm_blocker` reason is written to the allowlist entry for every tool in the module, including the specific crate or symbol that fails (e.g., `"wasm_blocker: getrandom 0.2 requires js feature gate not exposed by corp-finance-wasm Cargo.toml"`).
- A structural-blocker memo is appended to this ADR's References section.
- The sub-wave commit is skipped; the wave counter advances for the next module.

Three attempts is the threshold because: the first attempt surfaces the error, the second attempt applies the canonical mitigation (feature gate, crate swap, or thin shim), and the third attempt validates the mitigation or confirms the blocker is structural.

### 5. Done condition

The Wave-16 port effort is complete when `.surface-allowlist.json` contains only:

- The 34 permanent native-required carve-outs (federation, memory, multi_agent, audit, cost, self_learning, and their transitive-dependency tools).
- Any `wasm_blocker` carve-outs identified during porting, each with a non-empty `wasm_blocker` reason.

At that point `npm run check:surface-parity` passes with zero unaccounted tools, and the plugin surface reaches approximately 243 tools (277 - 34, adjusted for any structural blockers).

### Alternatives Considered

**Keep NAPI-only permanently, document as acceptable** — Rejected. This defeats the cross-platform deployment goal that motivated the WASM surface in ADR-026. The dual-mode architecture is only valuable if the plugin surface approaches tool-count parity with the NAPI surface for pure-math tools.

**Switch the plugin entirely to NAPI per-platform binaries** — Rejected in ADR-026. Ships per-platform `.node` files, losing the single-artefact portability that is the plugin's primary value. That decision stands.

**Build a thin RPC adapter where the plugin calls a local NAPI process** — Rejected. Introduces a daemon and IPC channel, which is not portable and adds a runtime dependency that breaks the single-artefact deployment model.

**Codegen plugin server registrations from `packages/mcp-server`** — Deferred. This would auto-generate the `server.ts` registrations from the NAPI tool definitions, which is architecturally attractive and was also deferred in ADR-026 (Decision 6). It does not solve the underlying `wasm_bindgen` export problem — the Rust symbols must still be exported from `crates/corp-finance-wasm` manually. Revisit after 5–6 modules are ported and the `wasm_bindgen` export patterns are stable enough to codegen from.

**Port all 237 tools in a single wave** — Rejected on operational grounds. A single large wave produces a multi-thousand-line diff that is unreviable, blocks main for an extended period, and makes structural blockers impossible to attribute to a specific module.

## Consequences

### Positive

- The plugin surface grows from 6 tools to approximately 243 tools upon completion, enabling CFA subagents that use the plugin to access near-full computational coverage.
- Cross-platform deployment of the cfa-core plugin is unblocked for all pure-math tools.
- The batch-by-module pattern keeps each sub-wave diff small and reviewable. Structural blockers are surfaced early and isolated to a module, not buried in a large wave.
- The stop-condition and blocker-memo protocol ensures no tool silently remains unported without a written explanation.

### Negative

- The `.wasm` bundle will grow significantly. Estimated 200 KB–2 MB per module; with 237 tools across an estimated 20–40 modules, total bundle size could reach 4–80 MB before any splitting is applied.
- Plugin startup time in Claude Code may grow measurably as the WASM binary grows, because `require`-time instantiation is proportional to bundle size.
- The `wasm-bindgen` and `wasm-pack` toolchain must remain stable across all sub-waves. A breaking change in either tool would require a coordinated update across all in-flight sub-waves.
- Sequential sub-waves mean the port effort is serialised. With 20–40 modules, completion could span many weeks of wave cadence.

### Neutral

- The Wave 16a pilot (parallel agent, single-module end-to-end) validates the recipe before the sequential sub-waves begin. Its findings may cause minor revisions to the recipe doc but do not alter the strategic decisions above.
- The 34 permanent carve-outs remain in the allowlist indefinitely. Their `reason` entries must be maintained accurately as the underlying crates evolve (e.g., if rusqlite eventually ships a WASM target, the carve-out for SQLite-dependent tools should be revisited).
- The bundle-size split evaluation (triggered at 5 MB) may produce a follow-on ADR. That ADR would supersede the bundle-size section of this one, not the porting strategy as a whole.

## Links

- Supersedes: none
- Depends on: ADR-026 (Plugin/Packages Dual-Mode Architecture and Surface Parity Invariant)
- Related: ADR-015 (Ruflo Orchestration Substrate), ADR-018 (Multi-Agent Coordination), ADR-024 (MCP Tool-Call Audit Middleware), ADR-025 (Rust-to-TypeScript Schema Auto-Generation)

## References

- `crates/corp-finance-wasm/` — WASM target crate; `wasm_bindgen` exports live here
- `plugins/cfa-core/mcp/src/server.ts` — plugin tool registrations (6 tools at time of this ADR; lines 77–117)
- `packages/mcp-server/.surface-allowlist.json` — allowlist with Wave-16 backlog entries
- `packages/mcp-server/scripts/check-surface-parity.mjs` — surface-parity enforcement script
- Wave 16a pilot report: to be appended here once the parallel agent completes
- Structural-blocker memos: to be appended here as sub-waves encounter them
