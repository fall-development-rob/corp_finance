# Domain Model: Multi-Tenant Federation

## Bounded Context: Multi-Tenant Federation

This bounded context owns tenant identity, federation session state, PII redaction policy, and behavioral trust scoring. It supplies `TenantContext` to every CFA runtime invocation regardless of surface, and brokers cross-installation collaboration through native Rust modules (`rustls` for mTLS, `ed25519-dalek` for signatures, `regex` for outbound PII policy, `rusqlite` for persisted trust scores). The federation design draws inspiration from `ruflo-federation` concepts (zero-trust mTLS plus ed25519, 14-type PII policy, behavioral trust scoring composition) without runtime coupling.

Tenant scoping applies to every output and state boundary on every CFA surface:

- **CLI** output paths under `<repo>/var/tenants/<tenant_id>/out/`
- **MCP** per-tool persistent state keyed on `tenant_id`
- **Skill** outputs (inheriting CLI / MCP placement via the wrapper used)
- **Plugin** hook output files (`plugins/cfa-core/hooks/hooks.json`-driven Write / Edit outputs) routed under the tenant's output root

Cookbooks (`managed-agent-cookbooks/`) are deployment artefacts and outside this bounded context's runtime scope; the only place `CookbookTier` enters this context is at deployment-time tagging (see Trust Tier Mapping below).

The context is the safety substrate beneath the Multi-Agent Coordination context: agent invocations cannot escape their tenant; cross-firm collaboration requires explicit handshake; PII redaction defaults to BLOCK. v1 ships with within-installation tenant isolation only; v2 adds cross-installation handshake.

### Domain Language (Ubiquitous Language)

| Term | Definition |
|------|-----------|
| **Tenant** | A distinct legal entity (LP, family, fund, deal team) whose data, audit trail, and outputs must be isolated from other tenants on the same installation |
| **TenantContext** | The runtime value object carrying tenant identity, output root, env namespace, audit namespace, and PII policy; supplied to every CLI subcommand, every MCP tool call, and every plugin hook fire |
| **Trust Tier** | Federation posture: `Open`, `Verified`, `Trusted` |
| **Federation Identity** | An ed25519 keypair (via `ed25519-dalek`) plus mTLS certificate (via `rcgen` for self-signed and `rustls` for the TLS stack) that uniquely identifies a tenant or peer installation |
| **Federated Session** | A live cross-installation collaboration with a peer; carries handshake state, redaction policy, and trust score (v2 only) |
| **PII Redaction Policy** | Per-tenant configuration of one of four modes (BLOCK, REDACT, HASH, PASS) for each of the 14 PII types defined and owned by us in `corp_finance_core::observability::security_scan` |
| **Trust Score** | A native behavioral score in [0, 1] computed as `0.4 * success + 0.2 * uptime + 0.2 * threat + 0.2 * integrity`, persisted in the cost ledger SQLite store |
| **Trust Downgrade** | An event raised when a peer's trust score crosses a threshold downward; pending actions enter hold state |
| **Cross-Firm Handshake** | The signed agreement required for `PaidVendor`-tagged surface targets in v2 federation; both peers cryptographically authorise vendor data redistribution |
| **Tenant Registry** | The TOML file at `<repo>/var/tenants/registry.toml` listing configured tenants and their identities |

### Aggregates

#### Tenant Aggregate

- **Root**: `Tenant`
- **Entities**: `TenantContext`, `EnvNamespace`, `AuditNamespace`, `OutDirRoot`
- **Invariants**:
  - `tenant_id` must be a non-empty, lowercase, kebab-case string
  - `tenant_id` must be unique within the registry
  - `out_dir_root` must be an absolute path under `<repo>/var/tenants/<tenant_id>/out`
  - `out_dir_root` permissions must be 0700 (POSIX) on first use
  - `env_namespace` must be `TENANT_<UPPER_KEBAB>_` derived from `tenant_id`
  - `audit_namespace` must be `tenants.<tenant_id>` derived from `tenant_id`
  - Default tenant `local` is reserved; cannot be re-registered
  - Every CLI subcommand, every MCP tool call, and every plugin hook fire receives a `TenantContext` (defaulting to `local` when no tenant is selected)

#### TrustScore Aggregate

- **Root**: `TrustScore`
- **Entities**: `ScoreSample`, `Threshold`
- **Invariants**:
  - Score is in `[0.0, 1.0]`
  - Score composition is `0.4 * success + 0.2 * uptime + 0.2 * threat + 0.2 * integrity` (native implementation, formula adopted from the ruflo-federation design)
  - Threshold for inbound cross-installation request: `>= 0.70` (v2)
  - Threshold for outbound REDACT/HASH PII: `>= 0.80`
  - Threshold for outbound PASS PII: `>= 0.90`
  - Threshold for cross-firm handshake on `PaidVendor`-tagged surface targets: `>= 0.95` (v2)
  - Score downgrade is instantaneous on threat detection (no smoothing)

#### FederatedSession Aggregate (v2 only)

- **Root**: `FederatedSession`
- **Entities**: `HandshakeState`, `MessageLog`, `RedactionLedger`
- **Invariants**:
  - Session id is unique across all federated peers
  - `handshake_state` must transition through: `Initiated -> Authenticated -> Authorised -> Active` or `Initiated -> Failed`
  - Session enters `Hold` on `trust_downgrade`; no new actions accepted while held
  - Failed cert validation always results in `Failed` (deny by default)
  - Messages exceeding the session's PII redaction policy are blocked at the boundary
  - Inbound messages are logged with their claimed redaction mode for audit reconstruction

#### PIIRedactionPolicy Aggregate

- **Root**: `PIIRedactionPolicy`
- **Entities**: `PolicyRow` (one per of 14 PII types)
- **Invariants**:
  - Every PII type must have a mode (BLOCK, REDACT, HASH, PASS)
  - Default policy for new tenants: every type set to BLOCK
  - Policy changes emit `pii_redacted_outbound` event on next outbound use
  - Policy is per-tenant; CLI invocations, MCP tool calls, and plugin hook fires inherit the originating tenant's policy
  - Policy mutations are append-only (new version supersedes old; old version retained for audit)

### Domain Events

| Event | Raised By | Payload | Consumers |
|-------|-----------|---------|-----------|
| `tenant_provisioned` | Tenant aggregate | `tenant_id`, `display_name`, `trust_tier`, `created_at` | Audit pipeline, Operator UI |
| `federation_handshake_started` | FederatedSession (v2) | `session_id`, `peer_url`, `peer_identity` | Audit pipeline |
| `federation_handshake_completed` | FederatedSession (v2) | `session_id`, `peer_identity`, `outcome` | Audit pipeline, Multi-Agent Coordination |
| `trust_downgrade` | TrustScore | `peer_identity`, `previous_score`, `current_score`, `threshold_crossed` | Audit pipeline, FederatedSession (places in Hold), Operator alert |
| `pii_redacted` | PIIRedactionPolicy / RedactionLedger | `surface`, `surface_event_id`, `pii_type`, `mode_applied`, `count` | Audit pipeline |
| `pii_blocked_outbound` | RedactionLedger (v2) | `session_id`, `pii_type`, `reason` | Audit pipeline, Operator alert |
| `tenant_isolation_enforced` | Tenant aggregate | `tenant_id`, `surface`, `boundary` (`fs` / `env` / `audit`), `attempt` | Audit pipeline, Operator alert (if attempt was cross-tenant) |
| `cross_firm_handshake_required` | FederatedSession (v2) | `session_id`, `surface_event_id`, `cookbook_tier` | Operator UI |

There are no aggregates or events tied to cookbook deploy lifecycle: cookbook deploys run outside this context's runtime scope.

### Anti-Corruption Layer

The federation context is implemented as native Rust modules. The ACL wraps the surface event format so domain types (Tenant, TenantContext, PIIRedactionPolicy, TrustScore) stay clean of any external-MCP-server-shape leakage. v1 builds depend only on `rustls`, `ed25519-dalek`, `rcgen`, and `rusqlite`.

- `federation/pii_redaction.rs` — defines the 14 PII type set (shared with `corp_finance_core::observability::security_scan`) and the four redaction modes BLOCK / REDACT / HASH / PASS; exposes `PIIRedactionPolicy` as the domain type
- `federation/trust_score.rs` — implements the native trust score formula and persistence in the cost ledger SQLite store; exposes `TrustScore` as the domain type and `trust_threshold_for(action) -> Decimal` as the policy lookup
- `federation/session.rs` — implements the handshake state machine using `rustls` (TLS 1.3) and `ed25519-dalek` (signatures); exposes `FederatedSession` and the `HandshakeState` enum as domain types
- `federation/tenant.rs` — Tenant aggregate plus the registry parser; translates surface event payloads (CLI / MCP / plugin) into `TenantContext` injection at the boundary; specialists never see raw MCP envelope shape

The rest of the codebase consumes only domain types. Tenancy lookups (`tenant_lookup(id) -> Option<Tenant>`) are pure registry operations.

### Trust Tier Mapping

The existing `CookbookTier` enum at `crates/corp-finance-core/src/managed_agent/types.rs` maps to federation `TrustTier` only at deployment time. The mapping informs the question "if the user is publishing this surface target as a cookbook of tier T, what trust posture is required to federate it?"; it is not a runtime tenancy concept.

| CookbookTier | TrustTier | Federation Behaviour (deployment-time consideration) |
|--------------|-----------|------------------------------------------------------|
| `CoreOnly` | `Open` | If published, always permitted to federate; no client data implications |
| `Freemium` | `Verified` | If published, federation requires receiving installation present a valid signed federation identity |
| `PaidVendor` | `Trusted` | If published, federation requires explicit cross-firm handshake; both peers sign session-scoped data redistribution agreement |

The mapping is encoded in `federation::cookbook_tier_to_trust_tier(CookbookTier) -> TrustTier`. This function is the single source of truth for tier-to-trust translation. Runtime tenancy does not consult `CookbookTier`; it operates on `Tenant` and `TrustScore` directly.

### Context Map

```
+---------------------------------------------------------------+
|              MULTI-TENANT FEDERATION CONTEXT                  |
|                                                               |
|  +-------------+   +---------------+   +------------------+   |
|  |   Tenant    |-->| TenantContext |-->| CLI / MCP /      |   |
|  | (registry)  |   | (per-         |   | Skill / Plugin   |   |
|  |             |   |  invocation)  |   | invocation       |   |
|  +------+------+   +-------+-------+   +------------------+   |
|         |                  |                                  |
|         v                  v                                  |
|  +------+------+   +-------+--------+                         |
|  | TrustScore  |<->|FederatedSession|                         |
|  | (per peer)  |   | (per session,  |                         |
|  |             |   |  v2 only)      |                         |
|  +------+------+   +-------+--------+                         |
|         |                  |                                  |
|         v                  v                                  |
|  +------+------------------+--------+                         |
|  |   PIIRedactionPolicy             |                         |
|  |   (14 types x 4 modes)           |                         |
|  +----------------------------------+                         |
+----+-----------------+--------------+-------------------------+
     |                 |              |
     v                 v              v
+----+-------+   +-----+------+   +---+-----------+
| Multi-     |   |   Audit /  |   |  Memory       |
| Agent      |   | Observ.    |   |  (Knowledge   |
| Coordin.   |   |            |   |   graph,      |
|            |   |            |   |   trajectory) |
+------------+   +------------+   +---------------+
```

### Context Relationships

| Upstream | Downstream | Relationship | Detail |
|----------|------------|--------------|--------|
| CLI / MCP / Skill / Plugin surfaces | Federation | Customer / Supplier | Every surface invocation receives a `TenantContext`. Federation supplies it; surfaces enforce it across the file system, env, and audit boundaries. |
| **Multi-Agent Coordination (`domain-orchestration.md`)** | Federation | Customer / Supplier | Coordination consumes `TenantContext` per agent dispatch and per entity registration. Federation supplies the tenant identity; Coordination enforces tenant scoping in `EntityGraph`. |
| **Audit / Observability** | Federation | Conformist | Federation emits all domain events through the audit pipeline; we conform to the audit envelope shape defined in `domain-audit-observability.md`. |
| **Memory** | Federation | Customer / Supplier | Memory partitions stored RunSummaries by `tenant_id`. Federation supplies the tenant identity; Memory is responsible for partitioning. |
| **Cost** | Federation | Customer / Supplier | Cost ledger keys include `tenant_id`. Federation supplies the identity; cost owns the accounting. |
| **Existing Managed Agent (`managed_agent::types::CookbookTier`)** | Federation | Shared Kernel (deployment-time only) | `CookbookTier` is consumed at deployment-time by `cookbook_tier_to_trust_tier`. Cookbooks themselves are deployment artefacts and outside this bounded context's runtime scope. |
| **Optional external federation backend** | Federation | Anti-Corruption Layer | The ACL boundary at `federation/pii_redaction.rs` / `federation/trust_score.rs` / `federation/session.rs` / `federation/tenant.rs` exists as future-proofing. v1 builds use the native `rustls` + `ed25519-dalek` implementation with no external dependency. |

### Invariants Summary

| ID | Invariant | Enforced By |
|----|-----------|-------------|
| FED-INV-001 | Every CFA surface invocation must carry `TenantContext` (when feature is enabled) | CLI / MCP / Skill / Plugin boundary |
| FED-INV-002 | `tenant_id = "local"` is reserved; cannot be re-registered | Tenant registry |
| FED-INV-003 | `out_dir_root` permissions are 0700 on POSIX | Tenant aggregate |
| FED-INV-004 | Default PII policy is BLOCK for all 14 types | PIIRedactionPolicy default ctor |
| FED-INV-005 | Trust score below threshold places session in Hold (v2) | FederatedSession |
| FED-INV-006 | Failed cert validation results in handshake `Failed` (deny by default) | FederatedSession |
| FED-INV-007 | `cookbook_tier_to_trust_tier` is the single source of truth for tier-to-trust mapping | federation module |
| FED-INV-008 | Cross-tenant entity merge is forbidden unless explicit federation session bridges | Tenant + Multi-Agent Coordination boundary |
| FED-INV-009 | PaidVendor federation requires both-peer signed handshake (v2) | FederatedSession state machine |
| FED-INV-010 | Audit and env namespaces are derived from `tenant_id` (no manual override) | Tenant aggregate |

### Sub-Domain Breakdown

1. **Tenant Registry**: TOML file at `<repo>/var/tenants/registry.toml`. Loaded at startup; SIGHUP or restart to reload. Contains tenant id, display name, federation identity reference, and PII policy version.
2. **Federation Identity**: ed25519 keypair plus mTLS certificate. Generated via `cfa tenant init` and stored under `<repo>/var/tenants/<id>/identity/`.
3. **Trust Score Computation**: native implementation of the behavioral score `0.4 * success + 0.2 * uptime + 0.2 * threat + 0.2 * integrity`; samples persisted in the cost ledger SQLite store.
4. **Session Lifecycle (v2)**: State machine `Initiated -> Authenticated -> Authorised -> Active`; or any state to `Failed` on validation error; `Active -> Hold` on trust downgrade.
5. **PII Policy Application**: Outbound message scanning against the policy at every CFA surface boundary; BLOCK aborts the message; REDACT replaces with placeholder; HASH replaces with `sha256(value)[:8]`; PASS sends as-is.

### MCP Tool Mapping

| Sub-Domain | Tool Name | Description |
|------------|-----------|-------------|
| Tenant management | `tenant_list` | List all configured tenants |
| Tenant management | `tenant_show` | Show full tenant context including PII policy |
| Federation status (v2) | `federation_status` | List active federated sessions and trust scores |
| Federation status (v2) | `federation_audit_query` | Query federation audit records by session, peer, or time window |
| Trust score | `trust_score_get` | Get current trust score for a peer |
| PII policy | `pii_policy_get` | Get the PII policy for a tenant |
| Session management (v2) | `federation_handshake_initiate` | Initiate cross-firm handshake (interactive confirmation required) |
