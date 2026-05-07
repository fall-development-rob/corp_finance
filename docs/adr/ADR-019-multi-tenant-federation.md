# ADR-019: Multi-Tenant Federation Across CLI / MCP / Plugin Surface Boundaries

## Status: Proposed

## Date: 2026-05-06

## Deciders: CFA platform engineering, managed-agent module owners, security

## Context

The CFA agent platform increasingly runs against data and workflows that belong to distinct legal entities: LP A's GL reconciliation must not leak into LP B's audit trail; a wealth advisor's client review for Family X must be invisible to Family Y; a sell-side bank's CIM drafting must not bleed across mandates. Today the platform has no tenancy primitive. A `cfa <subcommand>` invocation writes to `out/<slug>/<timestamp>/`, sources environment variables from the operator's shell, and emits audit records into a single shared sink. MCP tool calls return data without any tenant attribution. Plugin Write/Edit hooks fire on file events with no tenant scope. Two operators on the same workstation can clobber each other; two LPs sharing the same deployment cannot be cleanly isolated.

ADR-018 (revised) made clear that there is no daemon-mode runtime to scope; coordination happens through Claude Code's Agent tool inside a session, anchored at the four runtime surfaces from ADR-015 (CLI, MCP, skill, plugin). Tenancy must therefore be scoped at those same surfaces — every CLI invocation, every MCP tool call, every plugin hook fire, every output file emission must be tenant-attributed.

A second class of use case is appearing: cross-installation collaboration. A consortium of family offices wants to share research outputs without exposing the underlying client positions. A bank's M&A team wants to share an anonymised teaser with a counterparty's deal team. These are not multi-tenant scenarios within a single installation — they are cross-installation federation problems with strict PII redaction and trust requirements.

We build the federation capability natively, drawing inspiration from the `ruflo-federation` design (zero-trust mTLS plus ed25519, 14-type PII redaction policies BLOCK / REDACT / HASH / PASS, behavioral trust scoring). The May 2026 smoke test (`/tmp/ruflo-smoke-test.md`) showed `ruflo-federation` works end-to-end via `npx` shell-out (not MCP), but with no separate npm package and no Rust crate at the agent-federation scope. Building natively gives us a v1 simple-tenancy implementation (per-tenant output paths + per-tenant audit log namespaces) without runtime coupling to npx; v2 can add mTLS via `rustls` and ed25519 via `ed25519-dalek` if cross-installation handshake becomes a near-term need.

The existing `CookbookTier` enum in `crates/corp-finance-core/src/managed_agent/types.rs` (deploy-time tier classification for cookbook artefacts) and the `McpServerTier` enum at `crates/corp-finance-core/src/mcp_servers/types.rs` (runtime MCP cost tier) together encode trust boundaries that map cleanly to federation trust levels.

## Decision

Build a native multi-tenant federation module inspired by the `ruflo-federation` design. Define an explicit `TenantContext` value object that flows through every surface event (CLI invocation, MCP tool call, plugin hook fire). Map the existing `McpServerTier` and `CookbookTier` enums to native trust levels. Enforce tenant scoping at every output / state boundary across the four surfaces:

- **CLI output paths**: `./out/<tenant_id>/<subcommand>/<timestamp>/` rooted at `<repo>/var/tenants/<tenant_id>/out/`. Enforced at CLI binary entry in `crates/corp-finance-cli/src/main.rs`.
- **MCP per-tool-call state**: every MCP tool handler reads the active `TenantContext` from the call frame; tool outputs and any cached state are partitioned by tenant. Enforced at the MCP wrapper in `packages/*-mcp-server/src/`.
- **Plugin hook outputs**: `.audit.json` companions and PII scan logs are written under per-tenant paths; audit-log records carry `tenant_id`. Enforced at `plugins/cfa-core/hooks/hooks.json`.
- **Memory partitions**: HNSW + BM25 + petgraph stores (ADR-016) carry `tenant_id` on every record; queries default to filtering by the caller's tenant.
- **Cost ledgers**: per-tenant SQLite ledgers (ADR-017); cost summaries default to the caller's tenant.

Cross-installation handshake (mTLS + ed25519 + behavioral trust scoring) is implemented natively in v2; v1 ships with within-installation tenant isolation only.

### Module Structure

```
crates/corp-finance-core/src/federation/
  mod.rs                       (new -- TenantContext, TrustTier)
  tenant.rs                    (new -- Tenant aggregate + tenant registry parser; ed25519 signing of peer entries)
  pii_redaction.rs             (new -- 14 PII type definitions and redaction policy mapping; native scanner integration)
  trust_score.rs               (new -- behavioral trust scoring; native composition formula)
  session.rs                   (new -- federated session lifecycle and handshake state; rustls + ed25519-dalek in v2)
```

The federation module sits at `corp_finance_core::federation` (top-level, not under `managed_agent`) because it is not deploy-time tooling — it is runtime context flowing through every surface.

Feature flag: a new `federation` feature in `corp-finance-core/Cargo.toml`. Without the feature, the platform defaults to a single implicit tenant (`tenant_id = "local"`) and continues to behave as today. With the feature enabled, every surface event must carry a `TenantContext` or be rejected.

### TenantContext

Every surface event carries a `TenantContext`:

```text
TenantContext {
  tenant_id: String,                       // stable identifier; e.g. "lp-acme", "family-jones"
  display_name: String,                    // human-readable
  trust_tier: TrustTier,                   // Open | Verified | Trusted (mapped from tier enums)
  out_dir_root: PathBuf,                   // absolute, must be under <repo>/var/tenants/<tenant_id>/out
  env_namespace: String,                   // env var prefix; e.g. "TENANT_LPACME_"
  audit_namespace: String,                 // audit log namespace; e.g. "tenants.lp-acme"
  memory_namespace: String,                // memory store partition key; e.g. "tenants.lp-acme"
  cost_ledger_path: PathBuf,               // absolute path to per-tenant cost ledger SQLite
  federation_identity: Option<FederationIdentity>,  // ed25519 public key, mTLS cert ref
  pii_policy: PIIRedactionPolicy,
}
```

Tenant resolution at each surface:

- **CLI**: `--tenant <id>` flag on every CLI subcommand; if absent, falls back to `CFA_TENANT` env var; if absent, defaults to `local`. Resolved in the CLI binary's argument parser before subcommand dispatch.
- **MCP**: tenant attribution is read from the MCP request envelope's metadata (TODO: confirm metadata field name with the MCP transport team — proposal is `_meta.tenant_id`); falls back to `local` if absent.
- **Plugin**: hooks read the calling process's `CFA_TENANT` env var or the active `TenantContext` from a per-process file at `<repo>/var/tenants/<active>/.context`.
- **Skill**: not separately resolved; the LLM's tool call inherits the MCP request's tenant attribution.

The tenant registry lives at `<repo>/var/tenants/registry.toml`. An unknown `tenant_id` is rejected at the surface boundary with a structured error.

### Trust Tier Mapping

Trust tier flows from the runtime cost tier of MCP servers consumed and from the deploy-time cookbook tier (when the surface event is `cfa managed-agent deploy`):

| Source enum | Federation TrustTier | Cross-Firm Federation Behavior |
|-------------|----------------------|---------------------------------|
| `McpServerTier::Free` (and CLI invocations using only free MCP servers) | `Open` | Always permitted to federate. No vendor credentials required; no client-data inputs. Outputs are computation results from user-supplied inputs and can be shared freely subject to PII policy. |
| `McpServerTier::Freemium` (CLI invocations using freemium MCP servers) | `Verified` | Federation requires that the receiving installation present valid tenant credentials (signed federation identity). Outputs may include data from public sources and FMP; PII policy applies to any client identifiers in inputs. |
| `McpServerTier::PaidVendor` (CLI invocations using paid-vendor MCP servers) | `Trusted` | Federation requires an explicit cross-firm authorisation handshake: both installations sign a session-scoped agreement that vendor data may be redistributed under the receiving firm's existing vendor entitlement. Failure of either signature aborts the session closed (deny by default). |
| `CookbookTier` (when surface event is a cookbook deploy) | Same mapping by tier name | Cookbook deploy CLI invocations inherit the cookbook's deploy-time tier. |

The mapping is encoded in `federation::tier_to_trust_tier` and is the single source of truth for tier-to-trust translation. Surface events do not specify trust tiers manually — they flow from the consumed MCP server tiers and (for deploy-time) from the cookbook artefact's registry entry.

### PII Redaction Policy

The native scanner (per ADR-017) defines 14 PII types with four redaction modes (BLOCK, REDACT, HASH, PASS). The same 14 categories are reused here for federation policy; this is the load-bearing 14-count we own. The policy attached to each tenant configures which mode applies to each PII type for outbound federation messages. Default policy for new tenants is `BLOCK` for every type pending explicit operator review — federation fails closed.

Inbound federation messages are not redacted by us; the originating installation is responsible. We log every inbound message with its claimed redaction mode for audit reconstruction.

### Behavioral Trust Scoring

We compute a native behavioral trust score per peer with the composition `0.4 * success + 0.2 * uptime + 0.2 * threat + 0.2 * integrity`. The four input signals are:

- `success` — fraction of completed handshakes / messages that produced expected outcomes
- `uptime` — peer-reported heartbeat success ratio over the last rolling window
- `threat` — inverse of count(threat events) over the last window (each detected anomaly subtracts)
- `integrity` — fraction of inbound messages whose claimed PII redaction mode matched our scanner's verification

Each input is in [0, 1] and persisted in the cost ledger SQLite store under a `peer_trust_score` table. Trust thresholds for federation actions:

| Action | Minimum Score |
|--------|---------------|
| Inbound surface-event payload accepted | 0.70 |
| Outbound message with REDACT/HASH PII | 0.80 |
| Outbound message with PASS PII | 0.90 |
| Cross-firm handshake for PaidVendor surface event | 0.95 |

Score below the threshold triggers `trust_downgrade` and the federation session for that peer enters a hold state (no new actions accepted; pending actions allowed to drain or be cancelled by the operator).

### Tenant Scoping at the Surface Boundaries

1. **CLI binary** at `crates/corp-finance-cli/src/main.rs`: every subcommand reads `TenantContext`; output paths are composed under `<tenant.out_dir_root>/<subcommand>/<timestamp>/`. POSIX permissions on per-tenant directories are 0700 with ownership matching the deployment user.
2. **MCP wrapper** at `packages/*-mcp-server/src/`: every tool handler receives `TenantContext`; tool outputs are scoped by tenant; cached state (rate-limit counters, vendor-API tokens) is partitioned by `tenant_id`.
3. **Plugin hooks** at `plugins/cfa-core/hooks/hooks.json`: `.audit.json` companions are written under `<tenant.out_dir_root>/...`; audit log namespacing follows `tenant.audit_namespace`; PII scan logs are tenant-namespaced.
4. **Environment variables**: env-var substitution in cookbook manifests at deploy-time (`{{ENV_VAR}}` syntax) is namespaced by `tenant.env_namespace`. A manifest token `{{FMP_API_KEY}}` resolves to `TENANT_LPACME_FMP_API_KEY` for tenant `lp-acme`. The default tenant uses the bare names (no namespace) for backward compatibility.
5. **Memory partitions** (ADR-016): every `RunSummary` carries `tenant_id`; HNSW / BM25 / petgraph queries default to filtering by the caller's tenant; cross-tenant memory reads require an explicit operator role.
6. **Cost ledgers** (ADR-017): per-tenant SQLite ledgers at `<repo>/var/tenants/<tenant_id>/cost-ledger.sqlite`; budget thresholds are tenant-scoped.

### CLI Surface

| Command | Purpose |
|---------|---------|
| `cfa tenant list` | List configured tenants and their trust tiers |
| `cfa tenant scope <id>` | Display the tenant context (output root, env namespace, audit namespace, PII policy) |
| `cfa tenant show <id>` | Display full tenant record (alias of `cfa tenant scope`) |
| `cfa tenant init <id>` | Provision a new tenant (creates `out_dir_root`, generates ed25519 keypair if `federation` feature enabled) |
| `cfa federation status` | Status of active federated sessions and trust scores |
| `cfa federation handshake --peer <url>` | Initiate cross-firm handshake (interactive, prompts for confirmation) |
| `cfa <noun> <verb> --tenant <id>` | Run any CLI subcommand under a specific tenant (defaults to `local`) |

The slash commands consumed by `cfa-chief-analyst` and friends gain an optional `--tenant` flag with the same semantics. When omitted, the chief-analyst session inherits the operator's default tenant from the registry.

## Rationale

- mTLS, ed25519 identity, 14-type PII policies, and behavioral trust scoring are well-trafficked primitives with mature Rust crate support (`rustls`, `ed25519-dalek`, `regex`, `rusqlite`). Building natively gives us auditable code in our tree without runtime coupling to a partially-implemented upstream plugin.
- Tenancy and federation share the same identity primitive (ed25519 keypair, mTLS cert). Treating an internal tenant as a "self-federated peer" gives us one model rather than two.
- Mapping `McpServerTier` and `CookbookTier` to `TrustTier` makes the trust posture follow the surface event automatically: a CLI invocation that only consumes free MCP tools cannot accidentally land in a `Trusted` federation slot because its inferred trust tier is `Open`.
- Defaulting PII policy to all-`BLOCK` and trust thresholds high (0.70 minimum, 0.95 for PaidVendor handshake) makes the system fail closed. Operators must explicitly relax policy for any data movement.
- Tenant scoping at the file-system layer (POSIX 0700) provides a cheap, OS-enforced barrier even if higher-layer code has bugs. Defense in depth.
- v1 simple-tenancy ships in a few days (per-tenant output paths + audit namespaces + memory partitions + cost ledgers) without needing the cross-installation crypto stack; v2 adds mTLS + ed25519 only when cross-firm collaboration is a near-term need.
- Anchoring tenancy at the four runtime surfaces (rather than at a fictional cookbook-deploy event boundary) means tenant scoping is uniform across all 200+ CLI subcommands and ~594 MCP tools without per-feature wiring.

## Consequences

### Positive

- LP-A and LP-B isolation is enforced at file system, environment, audit, memory, and cost boundaries simultaneously, across all four runtime surfaces.
- Cross-firm collaboration becomes possible without ad-hoc data-sharing scripts; the federation module provides cryptographically auditable session state.
- The trust-tier mapping aligns federation posture with consumed MCP server tier and (for deploys) cookbook tier without operator decisions per surface event.
- Behavioral trust scoring penalises peers that fail (e.g., produce malformed outputs, miss heartbeats, present cert anomalies) automatically.
- Memory partitions and cost ledgers (Phase 26 ADRs 016/017) inherit tenant scoping with no additional wiring.

### Negative

- Adds approximately 900 lines of Rust across the federation files plus tests; compile-time impact estimated at 3-5 seconds when the feature is enabled.
- Operators must maintain a tenant registry and federation peer list. Mitigation: registry is a single TOML file under version control; CLI commands expose all state.
- All CLI subcommands must read `TenantContext` from the argument parser; all MCP tool handlers must read it from the request envelope. Mitigation: implicit `tenant_id = "local"` preserves backward compatibility for single-operator workstations; the wrapper from ADR-015 / ADR-017 is the single integration point.
- Federation sessions add network attack surface (mTLS endpoints). Mitigation: federation is opt-in via feature flag; default builds expose no listener.
- PII default-`BLOCK` will surprise operators who expect federation to "just work". Mitigation: documented in the federation onboarding flow and surfaced as a CLI warning on first handshake.

## Implementation Notes

- All federation state changes (tenant provisioned, handshake started, handshake completed, trust downgrade, PII redacted outbound) emit domain events consumed by the audit pipeline (Phase 26 RUF-AUDIT-*).
- The `federation` module follows the same JSON string boundary pattern as the rest of `corp-finance-core` for NAPI exposure.
- The tenant registry is loaded once per CLI binary start; updates require a new CLI invocation. This keeps the hot path free of disk reads.
- Native crate dependencies (added to `crates/corp-finance-core/Cargo.toml` behind the `federation` feature):
  - `rustls = "0.x"` — TLS 1.3 for cross-installation handshake (v2)
  - `ed25519-dalek = "2"` — keypair generation and signature verification
  - `rcgen = "0.x"` — self-signed mTLS cert generation
  - `serde_json` (already a dep) — message envelopes
  - `regex` (already used by the PII scanner) — PII pattern matching reused for outbound policy
  - `rusqlite` (already used by the cost ledger) — peer trust score persistence
- Surface-level instrumentation:
  - **CLI** at `crates/corp-finance-cli/src/main.rs`: argument parser reads `--tenant <id>` for every subcommand; resolves to `TenantContext` before subcommand dispatch; passes context through to the dispatched subcommand.
  - **MCP** at `packages/*-mcp-server/src/`: shared wrapper reads `_meta.tenant_id` from request envelope (TODO: confirm metadata convention with MCP team); resolves to `TenantContext`; passes to tool handler.
  - **Plugin** at `plugins/cfa-core/hooks/hooks.json`: hooks read active tenant from `<repo>/var/tenants/<active>/.context` or `CFA_TENANT` env var.
- POSIX 0700 directory creation is done by `cfa tenant init` at provisioning time. On Windows, equivalent ACLs are applied via standard library facilities; the platform supports Linux and macOS as primary targets.
- The federation peer list is signed by an operator-controlled root key; peers are added by appending a signed entry. We do not run an automatic peer-discovery service.
- The federation module is first-class native; the ACL boundary exists as future-proofing should we later choose to integrate the ruflo MCP backend or a different federation substrate. For v1 the modules contain no external runtime dependency.
- Specflow contracts: `docs/contracts/feature_federation.yml` (RUF-FED-001..010).

### Migration Path

The shipping order is incremental and preserves backward compatibility:

1. Land `TenantContext` plumbing through the CLI argument parser and the MCP wrapper with an implicit `tenant_id = "local"` default; no behavioural change for existing operators.
2. Land `federation::pii` and `federation::trust` modules with the default-BLOCK PII policy and trust-score consumption; no live federation yet.
3. Land `federation::session` and the handshake state machine; cross-firm sessions are operator-initiated only.
4. Wire `cfa tenant init` to generate the ed25519 keypair and mTLS cert under `<repo>/var/tenants/<id>/identity/`.
5. Enable the `federation` Cargo feature only after the tenant registry is populated and the audit pipeline (ADR-017) is verified to carry `tenant_id`.

Each step is feature-flagged. Single-operator workstations continue to run with `tenant_id = "local"` and the existing `out/` directory layout.

### Test Targets

| Module | Test Count | Key Scenarios |
|--------|-----------|---------------|
| federation (core) | ~25 | Tenant registry parse, reserved-id rejection, env-namespace derivation, audit-namespace derivation, McpServerTier/CookbookTier-to-TrustTier mapping coverage |
| federation::pii | ~30 | 14 PII type roundtrip, BLOCK / REDACT / HASH / PASS mode behaviour, default-BLOCK invariant, policy version transition |
| federation::trust | ~20 | Threshold-table coverage, score downgrade trigger, hold-state transition, threat-detection response time |
| federation::session | ~30 | Handshake state machine (Initiated -> Active and Initiated -> Failed), cert validation deny-by-default, PaidVendor both-peer-signed requirement, hold-on-downgrade |
| Tenant-scoped CLI invocation | ~15 | POSIX 0700 enforcement, env-var substitution per namespace, audit record tenant_id propagation, cross-tenant access denial |
| Tenant-scoped MCP tool call | ~10 | TenantContext propagation through wrapper, output partitioning, cached-state isolation |
| **Total** | **~130** | |

Combined with the multi-agent coordination test additions (ADR-018), Phase 27 brings the projected workspace test count from approximately 6,587 to approximately 6,787.

## Options Considered

### Option 1: Build tenancy without federation; layer federation on later (Rejected)

- Pros: Smaller initial surface area.
- Cons: Tenancy and federation share the identity primitive (ed25519 keypair). Building tenancy with a different identity model would require re-platforming when federation arrives. The combined design is no more complex than two phased designs and avoids a forced migration.

### Option 2: Adopt ruflo-federation as runtime substrate via npx shell-out (Rejected after smoke test)

- Pros: Delegates mTLS + ed25519 + PII redaction + trust scoring to the upstream plugin.
- Cons: May 2026 smoke test (`/tmp/ruflo-smoke-test.md`) showed the plugin works only via `npx -y -p @claude-flow/plugin-agent-federation@latest ruflo-federation <subcmd>` — not MCP. Every federation operation would shell out per call. Building natively with mature Rust crates (`rustls`, `ed25519-dalek`, `rcgen`) is comparable engineering cost and gives us in-process performance plus auditable code.

### Option 3: Defer federation entirely; ship simple tenancy only (Considered, partly adopted)

- Pros: Smallest v1 surface area.
- Cons: Cross-firm collaboration is on the roadmap. We resolve this by phasing: v1 ships simple tenancy (per-tenant paths + audit namespaces + memory partitions + cost ledgers); v2 adds the cross-installation crypto stack when needed. The native module structure is identical; v2 is a focused add-on.

### Option 4: Map every surface event to a single trust tier regardless of consumed MCP server tier (Considered)

- Pros: Simpler mental model.
- Cons: Surface events that consume PaidVendor MCP servers redistribute vendor-licensed data and require stronger handshake than CoreOnly. Collapsing the distinction either over-restricts CoreOnly federation or under-protects PaidVendor data. The three-tier mapping is the minimum that fits the data redistribution rights we already have.

### Option 5: Anchor tenancy at the cookbook-deploy boundary instead of the four runtime surfaces (Rejected)

- Pros: Smaller integration point if cookbooks were the runtime substrate.
- Cons: They are not. Cookbook deploys are CLI invocations like any other. Anchoring tenancy at the deploy event would leave 200+ non-deploy CLI subcommands and ~594 MCP tools un-scoped. Surface-level anchoring is the only complete answer.

## Related Decisions

- ADR-018 (revised): Multi-agent coordination via existing surfaces (consumes `TenantContext` from this ADR at every specialist routing call)
- ADR-015: Native Orchestration Umbrella (the four runtime surfaces; this ADR scopes tenancy to those surfaces)
- ADR-016: Memory Architecture (memory partitioned by `tenant_id` at the HNSW / BM25 / petgraph layer)
- ADR-017: Cost telemetry (cost ledgers keyed by `tenant_id`; per-tenant SQLite ledger files)
- ADR-001..ADR-007: Platform foundations consumed by tenancy (no breaking changes)

## References

- `crates/corp-finance-core/src/managed_agent/types.rs::CookbookTier` -- existing enum (deploy-time), mapped to trust tiers here
- `crates/corp-finance-core/src/mcp_servers/types.rs::McpServerTier` -- runtime enum, mapped to trust tiers here
- `crates/corp-finance-cli/src/main.rs` -- CLI binary; argument parser amended to read `--tenant <id>`
- `packages/*-mcp-server/src/` -- MCP servers; shared wrapper reads tenant from request metadata
- `plugins/cfa-core/hooks/hooks.json` -- plugin hooks; read active tenant for output path scoping
- `rustls`, `ed25519-dalek`, `rcgen`, `rusqlite` -- crate documentation on docs.rs
- Concept inspiration (not runtime dependency): https://github.com/ruvnet/ruflo
- Smoke test findings (May 2026): `/tmp/ruflo-smoke-test.md`
- ed25519 signature scheme: https://datatracker.ietf.org/doc/html/rfc8032
- mTLS reference: https://datatracker.ietf.org/doc/html/rfc8446 (TLS 1.3)
