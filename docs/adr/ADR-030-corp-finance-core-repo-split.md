# ADR-030: corp-finance-core Repository Split

## Status: Accepted

## Date: 2026-05-08

## Deciders

- Robert Fall
- CFA Agent platform engineering

## Tags

`repo-split`, `cargo`, `workspace`, `wasm`, `schema-gen`, `lexius`, `regfin`, `cross-repo`

## Context

Phase 29 shipped 24 commits across Waves 6–17 on 2026-05-08. Over that session, `corp-finance-core` evolved from 277 NAPI tools backed by a hand-maintained plugin into a 227-WASM-tool plugin with surface-parity gates (ADR-026), a CLI schema-discovery flag (ADR-029), a Rust-to-TypeScript schema auto-gen pipeline (ADR-025), and a WASM bundle size budget enforced at CI (ADR-028). The compute library is now a coherent, independently testable artefact with its own feature matrix and CI gates.

Simultaneously, a broader regfin agentic vision is taking shape: a compute library (`corp-finance-core`), a regulatory intelligence library (`lexius`), and an agent integration layer (`cfa_agent`). Independent, crate-published libraries are the natural architecture for this stack. Downstream consumers — including a future `lexius` service — should be able to depend on `corp-finance-core` without pulling in MCP servers, agent persona definitions, skill YAML files, or audit middleware.

The crates `corp-finance-core`, `corp-finance-wasm`, and `corp-finance-cli` are the stable compute surface. Everything else in `cfa_agent` — MCP servers, plugins, skills, slash commands, hooks, audit middleware, schema-gen outputs, office writer integrations — is the agent integration layer. These two concerns have different release cadences, different audiences, and different CI requirements.

## Decision

### 1. Extraction to a new git repository

`corp-finance-core`, `corp-finance-wasm`, and `corp-finance-cli` are extracted to a new git repository at `/home/robert/corp-finance-core` (initial commit `69c4c79`). The repository will later be pushed to GitHub as a separate org/repo. The extraction is a clean copy; the full commit history of the three crates remains accessible in the `cfa_agent` git log via `git log -- crates/corp-finance-core`.

### 2. Cargo dependency across the boundary

`cfa_agent` depends on the new repo via a cargo path dependency initially:

```toml
[dependencies]
corp-finance-core = { path = "/home/robert/corp-finance-core/crates/corp-finance-core" }
```

The path form is the Phase-29 bootstrap. The migration path is:

1. **Phase 29 (now)** — absolute path dep (`path = "/home/robert/..."`)
2. **Near-term** — git dep with SHA pinning (`git = "https://github.com/..."`, `rev = "<sha>"`)
3. **First stable release** — crates.io publish with semver constraint

### 3. Boundary definition

**corp-finance-core owns:**
- All decimal-precision financial math (277+ functions across 70+ modules)
- WASM-compatible compute (zero `f64` in financial calculations; Monte Carlo is the sole exception)
- `corp-finance-wasm` and `corp-finance-cli` as sibling crates in the new repo
- Its own CI: feature matrix (`default`, `schema_gen`, `cli_schema`, `wasm`), WASM bundle size check, cargo clippy

**cfa_agent owns:**
- MCP server registrations (`packages/mcp-server`, `packages/fmp-mcp-server`, etc.)
- Plugin manifest and WASM plugin (`plugins/cfa-core`)
- Agent persona definitions (`.claude/agents/cfa/`)
- Skills, slash commands, hooks (`.claude/skills/`, `.claude/commands/`)
- Audit middleware and audit-JSON companions
- Schema-gen pipeline outputs (`packages/mcp-server/src/schemas/`)
- Office writer integrations (`packages/mcp-server/src/tools/office_*`)
- Surface-parity gate and allowlist

### 4. WASM artefact flow

The WASM artefact (`plugins/cfa-core/mcp/wasm/corp_finance_wasm_bg.wasm`) is built in `corp-finance-core` via `scripts/build-wasm.sh` and copied into `cfa_agent`. The build-wasm script in `cfa_agent` at `plugins/cfa-core/scripts/build-wasm.sh` delegates to the new repo and copies the resulting `.wasm` file. `cfa_agent` does not rebuild WASM independently; the WASM is treated as a versioned binary artefact crossing the boundary.

### 5. Schema-gen flow

The Wave-11+ Rust→TypeScript schema auto-gen pipeline (ADR-025) crosses the boundary. The npm scripts in `cfa_agent/packages/mcp-server` invoke `cargo test` in the `corp-finance-core` repo to emit JSON Schema files, then convert to Zod and write output into `cfa_agent`. The pipeline is explicitly documented as cross-repo; the `schema-gen` npm script gains a `--repo-path` or a shell `cd` to the new repo root before invoking cargo.

### 6. Surface-parity scope is cfa_agent-only

The Wave-15 surface-parity audit (PARITY-INV-001..005, ADR-026) compares `packages/mcp-server/src/tools/` against `plugins/cfa-core/mcp/src/server.ts`. This comparison is an integration concern internal to `cfa_agent`. `corp-finance-core` has its own CI gates (feature matrix, WASM bundle size) but is not subject to the surface-parity check; the plugin-vs-packages comparison is meaningless at the compute library layer.

## Alternatives Considered

**Leave as a monorepo** — Rejected. A single repo locks the compute library into `cfa_agent`'s release cadence. `lexius` and future regfin consumers cannot depend on `corp-finance-core` without pulling in the full agent integration tree. The compute library would never acquire independent semver or crates.io presence.

**Cargo workspace with sibling path deps (`path = "../corp-finance-core/..."`)** — This IS the chosen approach for the Phase-29 bootstrap. The absolute path (`/home/robert/...`) will be replaced with a git dep as soon as the new repo has a remote. The sibling-path form is a transitional artefact.

**Extract via `git filter-repo` to carry history** — Deferred. A clean extraction is faster and sufficient for Phase 29. The three crates' history lives in `cfa_agent`'s git log and is retrievable with `git log -- crates/corp-finance-core`. History replay can be done at any point before the first public crates.io release if provenance is required.

**Publish to crates.io immediately** — Deferred. Requires a version bump to `1.0.0` or a pre-1.0 semver convention, a crates.io account under the org, and README polish. Deferred to the first stable release milestone.

**Git submodule** — Rejected. Submodule UX is hostile to the fast-iteration workflow of this project. Cargo's git dep with SHA pinning achieves the same version pinning without submodule footguns.

## Consequences

### Positive

- `corp-finance-core` becomes a standalone, publishable Rust library with its own semver, CI, and contributor surface.
- `lexius` and any future regfin compute consumer can depend on `corp-finance-core` without importing MCP servers, agent personas, or skills.
- CI separation: compute tests run independently; a failing integration test in `cfa_agent` does not block a compute-only fix, and vice versa.
- The feature matrix (`default`, `schema_gen`, `cli_schema`, `wasm`) is enforced in one place; downstream consumers opt in to only what they need.
- New contributors to `corp-finance-core` see a focused Rust library without MCP, plugin, or skill scaffolding.

### Negative

- Cross-repo refactors require coordinated changes: adding a new compute tool and its MCP registration now spans two repos and at minimum two PRs.
- The absolute path dep (`path = "/home/robert/corp-finance-core/..."`) is not portable; another developer's machine requires the same path alias or a local override in `.cargo/config.toml`.
- The schema-gen pipeline now requires a `cd` or `--manifest-path` to the new repo root; a script that previously assumed a single workspace tree now has an explicit cross-repo step.
- The WASM build introduces a copy step; a stale `.wasm` file in `cfa_agent` that was not rebuilt after a compute change is a silent correctness failure until CI runs the full pipeline.
- CI cannot run the full feature matrix on a single `cargo build --workspace`; two separate build trees must be wired together.

### Neutral

- The `cfa_agent` git log retains the full commit history of the three extracted crates up to the extraction commit (`69c4c79`). No history is lost; it is simply no longer in the live workspace.
- The split does not change the public MCP API surface or any agent behavior. Consumers of the MCP server observe no change.
- The WASM binary is already treated as a versioned artefact (ADR-028 bundle budget); the copy-across-repo step formalises what was already an implicit versioning discipline.

## Future Work

- Replace the absolute path dep with a git dep and SHA pinning to eliminate developer-machine portability issues.
- Publish `corp-finance-core 1.0.0` to crates.io with a curated README and feature-flag documentation.
- Mirror `corp-finance-core` on the user's GitHub org and wire CI to run the feature matrix on every push.
- Consider a `regfin-meta` umbrella repo (or Cargo virtual workspace) that aliases both `corp-finance-core` and `lexius` for downstream consumers that want both.
- Evaluate `git filter-repo` to carry the pre-extraction commit history into the new repo for crates.io provenance.

## Links

- Depends on: ADR-025 (Rust-to-TypeScript Schema Auto-Gen) — schema-gen pipeline that now crosses the repo boundary
- Depends on: ADR-026 (Plugin/Packages Dual-Mode and Surface Parity) — surface-parity gate that remains cfa_agent-internal
- Depends on: ADR-027 (Wave-16 WASM Port Strategy) — WASM artefact flow that this split formalises
- Depends on: ADR-028 (WASM Bundle Size Optimization) — bundle budget that corp-finance-core CI now owns independently
- Depends on: ADR-029 (CLI Schema Discovery Flag) — cli_schema feature that lives in the new repo
- `docs/contracts/feature_repo_split.yml` — invariants REPO-INV-001..005
- New repo root: `/home/robert/corp-finance-core` (initial commit `69c4c79`)
