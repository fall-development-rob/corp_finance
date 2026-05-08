# ADR-026: Plugin/Packages Dual-Mode Architecture and Surface Parity Invariant

## Status: Accepted

## Date: 2026-05-08

## Deciders

- CFA Agent platform engineering
- MCP tool surface owner
- Plugin (WASM) surface owner
- CFA specialist agent system owner

## Tags

dual-mode, wasm, napi, surface-parity, mcp, plugin, deployment, cfa-agents, drift-prevention

## Context

The CFA agent stack runs two MCP server implementations that cover the same conceptual tool domain but are shaped by fundamentally different deployment constraints:

**`plugins/cfa-core/mcp/`** is a WASM-backed Claude Code plugin server. It currently registers 6 tools (wacc_calculator, dcf_model, comps_analysis, credit_metrics, debt_capacity, covenant_compliance). WASM is portable across architectures (Linux, macOS, Windows; x64, arm64) and requires no per-platform binary distribution. Its constraint is that any Rust crate relying on native-only facilities — memory-mapped files, platform entropy sources (ed25519-dalek with OS random), rusqlite, or non-WASI network sockets — cannot compile to wasm32-unknown-unknown and is therefore excluded from this surface.

**`packages/mcp-server/`** is a NAPI-backed local-development server with approximately 90 tools accumulated over 14 waves. It has unrestricted access to the full Rust feature set (federation keypairs, SQLite trajectory storage, HNSW indexing, office OOXML writers, audit middleware, template dispatchers). Its constraint is that it ships per-platform `.node` binaries and is unsuitable as a portable deployable artefact.

These are not bugs to be unified — they are deliberate deployment-shape tradeoffs with different cost/benefit profiles. However, on 2026-05-08 the absence of any enforcement mechanism allowed the two surfaces to diverge by approximately 84 tools. CFA specialist subagents declared `tools: cfa-tools` in their frontmatter, which had been renamed to `cfa-core` without a corresponding agent-frontmatter update. The ghost name resolved to nothing at runtime, silently stripping all 84 tools from subagent sessions. This incident demonstrates that dual-mode correctness requires explicit invariants and automated enforcement, not convention alone.

The following distinctions are now formally acknowledged:

| Property | Plugin (`plugins/cfa-core/mcp/`) | Packages (`packages/mcp-server/`) |
|---|---|---|
| Runtime | WASM (wasm32-unknown-unknown) | NAPI `.node` binaries |
| Portability | Cross-platform, single artefact | Per-platform build required |
| Tool count | Curated subset (~6 today) | Full surface (~90 today) |
| Native crates | Excluded | Permitted |
| Deployment target | End-user Claude Code plugin | Local developer environment |

## Decision

### 1. Embrace the dual-mode split — do not merge

The two surfaces remain separate implementations. No attempt is made to unify them into a single server or to compile all 90 NAPI tools to WASM. The WASM surface is a deliberately curated subset of the NAPI surface, not a degraded copy.

### 2. Allowlist documents the NAPI-only carve-outs

Every tool registered in `packages/mcp-server/src/tools/*.ts` that is not present in `plugins/cfa-core/mcp/src/server.ts` must appear in `packages/mcp-server/.surface-allowlist.json` with a non-empty `reason` field explaining why it is NAPI-only (e.g., "uses rusqlite — cannot compile to wasm32" or "requires ed25519-dalek OS random source"). The allowlist is the boundary's explicit documentation.

### 3. Surface-parity audit script enforces drift at CI

`packages/mcp-server/scripts/check-surface-parity.mjs` compares the set of registered `server.tool()` calls between both surfaces and fails with a diff of unaccounted tools if any tool is absent from the plugin surface without an allowlist entry. This script is the primary enforcement mechanism for PARITY-INV-001.

### 4. Agent frontmatter tool declarations are validated

CFA specialist agent files at `.claude/agents/cfa/*.md` declare an MCP server in their `tools:` frontmatter. Every such declaration must reference a server name that exists as a registered plugin (or built-in Claude Code tool set). The current canonical plugin name is `cfa-core` (corrected from the ghost name `cfa-tools` on 2026-05-08). A grep-based validation check asserts PARITY-INV-002 and PARITY-INV-005.

### 5. Wave commits touching tools must document surface coverage

A wave commit message that adds a public `server.tool()` registration must either (a) update both `packages/mcp-server` and `plugins/cfa-core/mcp/`, or (b) include the token `NAPI-only because:` followed by a one-line reason in the commit body. This is a review-time convention enforced by PARITY-INV-003 and the commit-message format check.

### 6. Codegen from a shared manifest is deferred

The idea of deriving both surfaces from a single tool manifest (with per-surface capability flags) is architecturally attractive but premature. The manifest would need to capture WASM-compat status, input schemas, and routing metadata for ~90 tools before it becomes useful. This is deferred to a future wave once the allowlist has stabilised and both surfaces have grown to a comparable count.

### Alternatives Considered

**Merge plugin into packages (Rejected)** — Loses the cross-platform portability that is the plugin's primary value. End users without a Rust toolchain or the correct platform binary cannot use the server.

**Compile all packages tools to WASM (Rejected)** — Physically impossible for tools that depend on rusqlite, ed25519-dalek with platform entropy, HNSW mmap, or OS-level file/network operations. The constraint is the Rust crate ecosystem, not an implementation choice.

**Codegen from a shared manifest (Deferred)** — Correct direction architecturally but requires the manifest to grow alongside the tool surface before it provides a net benefit. Deferred to a future wave.

## Consequences

### Positive

- Silent drift between the two surfaces is impossible once the CI gate is in place. The allowlist makes every asymmetry a deliberate, documented choice rather than an accident.
- The architecture is honest about its tradeoffs. New contributors can read the allowlist to understand which tools are NAPI-only and why, rather than inferring it from build failures.
- Agent frontmatter validation eliminates the ghost-name class of bug. An agent that declares a non-existent server name will fail a deterministic grep check before deployment.
- The dual-mode boundary is now the single place to reason about portability: adding a new tool prompts a conscious decision (can this compile to WASM?) rather than defaulting to one surface.

### Negative

- Every new corp-finance tool that can compile to WASM now requires implementation in both surfaces. For tools that are straightforward wrappers around pure Rust math, this is low-cost but still additional work.
- Some tools are intrinsically NAPI-only (federation memory, SQLite trajectory, HNSW indexing, office OOXML writers with native compression). These acknowledged carve-outs live in the allowlist permanently, which must be maintained as the tool set grows.
- The audit script adds a CI dependency on both surfaces being built before the check can run. Build order matters.

### Neutral

- The allowlist (`packages/mcp-server/.surface-allowlist.json`) is a new file that must be introduced and schema-validated as part of the infrastructure landing for this ADR.
- The plugin server name `cfa-core` is now the canonical identifier. All documentation, agent frontmatter, and CI scripts must use this name. The former name `cfa-tools` is retired.
- Codegen unification is explicitly deferred; this ADR does not preclude it in a future wave.

## Related Decisions

- ADR-015: Ruflo Orchestration Substrate — multi-agent coordination context in which the agent frontmatter `tools:` declaration operates
- ADR-018: Multi-Agent Coordination — agent topology and subagent tool-set resolution
- ADR-024: MCP Tool-Call Audit Middleware — audit middleware wraps all tool registrations on the NAPI surface; the plugin surface has no audit middleware today
- ADR-025: Rust-to-TypeScript Schema Auto-Generation — schema drift between Rust types and TypeScript tool inputs; a complementary drift-prevention mechanism on the same `packages/mcp-server` surface

## References

- `plugins/cfa-core/mcp/src/server.ts` — WASM plugin tool registrations (6 tools at time of this ADR)
- `packages/mcp-server/src/tools/` — NAPI tool registrations (~90 tools)
- `packages/mcp-server/.surface-allowlist.json` — NAPI-only carve-out documentation (to be created)
- `packages/mcp-server/scripts/check-surface-parity.mjs` — drift audit script (to be created)
- `.claude/agents/cfa/` — CFA specialist agent frontmatter with `tools:` declarations
- `plugins/cfa-core/.claude-plugin/plugin.json` — canonical plugin name (`cfa-core`)
- Specflow contracts: `docs/contracts/feature_surface_parity.yml`
