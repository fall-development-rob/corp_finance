# Domain Model: Audit, Cost, Observability, Security

## Bounded Context: Audit / Cost / Observability / Security

This bounded context handles four production concerns at the runtime surface boundary: audit trails for outputs, cost tracking with budget alerts, structured observability for every CLI / MCP / plugin invocation, and security scanning for PII and prompt injection. The runtime activity covered is exclusively from the four CFA surfaces:

- **CLI** (`cfa <subcommand>`)
- **MCP** (every `server.tool(...)` registration in `packages/*-mcp-server/src/`)
- **Skills** (slash commands and `.claude/skills/*` invoked by the LLM via the Skill tool — observability inherits via the MCP wrapper)
- **Plugin** (PreToolUse / PostToolUse / Write / Edit hooks at `plugins/cfa-core/hooks/hooks.json`)

Cookbooks (`managed-agent-cookbooks/`) are deployment artefacts and outside this bounded context's runtime scope.

The context is implemented as native Rust modules using mature crates (`tracing` + `tracing-subscriber` for spans, `rusqlite` for the cost ledger, `regex` for PII / injection patterns). An anti-corruption layer wraps the surface event format so domain types stay clean of any external-MCP-server-shape leakage; for v1 the modules contain no external runtime dependency.

See ADR-015 for the umbrella decision and ADR-017 for the architectural detail. The Memory bounded context (see `docs/ddd/domain-memory.md`) is a sibling; both contexts are owned by the same team and cooperate via integration events.

### Domain Language (Ubiquitous Language)

| Term | Definition |
|------|-----------|
| **Surface Invocation** | A single CLI subcommand call, OR a single MCP tool handler execution, OR a plugin hook fire. The unit instrumented and audited. |
| **Audit Manifest** | The structured `.audit.json` companion file written when a Write or Edit plugin hook fires (file outputs) or when a CLI / MCP invocation produces a numeric output. |
| **Surface Audit Hash** | djb2 fingerprint over the surface configuration that produced the invocation (CLI subcommand version, MCP tool registration, plugin hook configuration). |
| **Tool Call Ledger** | The deterministic sequence of MCP tool calls behind a single output, captured in the audit manifest. |
| **CookbookTier** | `CoreOnly`, `Freemium`, or `PaidVendor`; classification used at cookbook publication time, not at runtime. |
| **McpServerTier** | Complementary classification of an MCP server by `crates/corp-finance-core/src/mcp_servers/types.rs::McpServerTier`. |
| **Cost Budget** | Monthly USD limit per surface target with thresholds for warning and hard stop, persisted in `rusqlite`. |
| **Trace Span** | A structured unit of telemetry recording one operation with attributes, start, end, and parent reference, emitted via the `tracing` crate. |
| **Security Scan** | A pass over content (input, memory write, final output) that detects PII categories and prompt-injection patterns via the native scanner. |
| **PII Category** | One of 14 categories defined and owned by us (inspired by aidefence's category list): names+address tuples, emails, phones, addresses, SSN, passport, drivers' licence, bank account, credit card, IBAN, IP address, MAC address, date of birth, government ID / EIN. |
| **Prompt Injection** | A pattern in user input or retrieved memory designed to manipulate the model's instructions. |
| **Allowlist** | The CFA-specific set of identifiers (CUSIP, ISIN, SEDOL, FIGI, LEI, ticker) that suppress PII false positives. |

### Aggregates

#### AuditManifest (Aggregate Root)

The structured record attached as a sibling file. The manifest is triggered at one of two points:

- **Plugin-hook level** — on a Write or Edit hook (configured in `plugins/cfa-core/hooks/hooks.json`), the manifest accompanies the file the hook produced.
- **CLI / MCP boundary** — for non-file outputs (numeric MCP results, CLI stdout artefacts), the manifest is written alongside the structured response.

| Field | Type | Description |
|-------|------|-------------|
| `schema_version` | `String` | "1.0" |
| `run_id` | `Uuid` (v7) | Same value as the `RunSummary.run_id` for this surface invocation |
| `surface` | `Surface` | `Cli`, `Mcp`, `Skill`, or `Plugin` |
| `surface_event_id` | `String` | CLI subcommand name, MCP tool name, slash command name, or plugin hook id |
| `surface_audit_hash` | `String` | `djb2:0x...` |
| `model` | `String` | Model identifier including 1M-context flag |
| `ts` | `DateTime<Utc>` | Output produced timestamp |
| `output_path` | `Option<String>` | Path of the output file this manifest accompanies (file case) |
| `output_sha256` | `Option<String>` | Content hash of the output file (file case) |
| `tool_call_ledger` | `Vec<ToolCall>` | Ordered MCP calls behind the output |
| `skills_in_scope` | `Vec<String>` | Sorted list |
| `sub_agents_in_scope` | `Vec<String>` | Sorted list |

**Invariants**:

- If `output_path` is present, it exists and `sha256(output_path) == output_sha256`.
- `tool_call_ledger` entries are ordered by `step` ascending; `step` values are 1-indexed and contiguous.
- `surface_audit_hash` matches the value computed by `compute_surface_hash()` over the surface configuration used.
- Where `output_path` is set, the manifest file lives at `<output_path>.audit.json`.
- `run_id` matches the `run_id` of the parent `RunSummary` for this surface invocation.

#### CostBudget (Aggregate)

The configured budget for a surface target and its current state. Persisted in `rusqlite` at `<repo>/var/observability/cost-ledger.sqlite`. A budget keys on the `(surface, surface_event_id)` tuple; CLI subcommands, MCP tools, and slash-command-via-MCP all key independently.

| Field | Type | Description |
|-------|------|-------------|
| `surface` | `Surface` | `Cli`, `Mcp`, `Skill`, or `Plugin` |
| `surface_event_id` | `String` | CLI subcommand / MCP tool / slash command / plugin hook id |
| `cookbook_tier` | `Option<CookbookTier>` | Set only if this surface target was published as a cookbook; classification applies at deploy time |
| `monthly_limit_usd` | `Option<Decimal>` | Null for free tier |
| `warn_thresholds_pct` | `Vec<u32>` | E.g., `[50, 80, 95]` for paid-vendor |
| `hard_stop_pct` | `Option<u32>` | Typically 100 |
| `current_usage_usd` | `Decimal` | Month-to-date |
| `last_threshold_crossed` | `Option<u32>` | Last warn threshold that fired this period |

**Commands**:

- `record_usage(token_usage, model)` — adds dollarised usage from a surface invocation.
- `check_thresholds()` — emits `budget_threshold_crossed` if a new threshold has been crossed since the last check.
- `reset_period()` — start of new month; current_usage_usd back to zero, last_threshold_crossed null.

**Invariants**:

- `current_usage_usd >= 0`.
- If `cookbook_tier == Some(CoreOnly)`, `monthly_limit_usd == None` and `warn_thresholds_pct.is_empty()`.
- `hard_stop_pct >= max(warn_thresholds_pct)` when both are set.
- A budget fires per surface invocation, never per cookbook deploy.

#### TraceSpan (Aggregate)

A single span in the structured trace for a surface invocation. Backed by the native `tracing` + `tracing-subscriber` crates; OTLP export available behind the `tracing-opentelemetry` optional feature. Spans cover every CLI subcommand, every MCP tool handler, and every plugin hook fire. Skills inherit via the MCP wrapper.

| Field | Type | Description |
|-------|------|-------------|
| `span_id` | `String` | Span identifier |
| `parent_span_id` | `Option<String>` | Parent in the trace tree |
| `trace_id` | `String` | Root trace id (matches the surface invocation's run id) |
| `name` | `String` | E.g., `cfa.cli.initiate_coverage`, `cfa.mcp.compute_dcf`, `cfa.plugin.write_hook` |
| `start_ts` | `DateTime<Utc>` | Open time |
| `end_ts` | `Option<DateTime<Utc>>` | Close time; null while open |
| `attributes` | `HashMap<String, AttributeValue>` | Span attributes |
| `status` | `SpanStatus` | `ok`, `error`, `cancelled` |

**Invariants**:

- `start_ts <= end_ts` when `end_ts` is set.
- `parent_span_id` references a span in the same trace (`trace_id` equality enforced).
- Span attributes containing user data are redacted per the policy in ADR-017 (`cfa.user` becomes first-initial-last-name).

#### SecurityScan (Aggregate)

A single pass over content. Implemented natively in `corp_finance_core::observability::security_scan` using a hand-rolled regex set covering 14 PII categories plus a prompt-injection pattern library. Scans fire at the surface boundary: pre-tool-call (MCP), pre-memory-write (Memory ACL), post-invocation (CLI / MCP output, plugin Write/Edit hook output).

| Field | Type | Description |
|-------|------|-------------|
| `scan_id` | `Uuid` | Identifier |
| `hook_point` | `HookPoint` | `pre-tool-call`, `pre-memory-write`, `post-invocation` |
| `content_kind` | `String` | `user_input`, `run_summary`, `session_archive`, `final_output` |
| `pii_findings` | `Vec<PiiFinding>` | Categories detected with offsets |
| `injection_findings` | `Vec<InjectionFinding>` | Prompt-injection patterns detected |
| `action` | `ScanAction` | `block`, `redact`, `alert_only` |
| `ts` | `DateTime<Utc>` | Scan time |

**Commands**:

- `scan(content, hook_point)` — runs the native scanner and populates findings.
- `apply_action()` — performs the configured action (block / redact / alert) per the hook policy in ADR-017.

**Invariants**:

- `hook_point == pre-tool-call` and `injection_findings.len() > 0` implies `action == block`.
- `hook_point == pre-memory-write` and `pii_findings.len() > 0` implies `action == redact`.
- `hook_point == post-invocation` always implies `action == alert_only` (final output not modified).
- The CFA allowlist (CUSIP, ISIN, SEDOL, FIGI, LEI, ticker symbols) is consulted before adding to `pii_findings`.

### Value Objects

#### Surface

Enum: `Cli`, `Mcp`, `Skill`, `Plugin`. Shared kernel with Memory and Multi-Agent Coordination contexts.

#### ToolCall

| Field | Type | Description |
|-------|------|-------------|
| `step` | `u32` | 1-indexed sequence position |
| `tool` | `String` | Registered MCP tool name |
| `input_hash` | `String` | `djb2:0x...` over canonical input |
| `output_hash` | `String` | `djb2:0x...` over canonical output |

#### CookbookTier (referenced; defined in `managed_agent::types`)

Enum: `CoreOnly`, `Freemium`, `PaidVendor`. The single source of truth lives at `crates/corp-finance-core/src/managed_agent/types.rs`. Used by this context only as a deployment-time tag persisted alongside CostBudget rows where the surface target was also published as a cookbook; no runtime semantic.

#### PiiFinding

| Field | Type | Description |
|-------|------|-------------|
| `category` | `String` | One of the 14 categories per ADR-017 |
| `offset` | `u32` | Character offset in scanned content |
| `length` | `u32` | Length of match |
| `confidence` | `Decimal` | 0..1 |

#### InjectionFinding

| Field | Type | Description |
|-------|------|-------------|
| `pattern` | `String` | Pattern label (e.g., `instruction_override`, `role_swap`) |
| `offset` | `u32` | Match offset |
| `length` | `u32` | Match length |
| `severity` | `String` | `low`, `medium`, `high` |

#### HookPoint

Enum: `PreToolCall`, `PreMemoryWrite`, `PostInvocation`.

#### ScanAction

Enum: `Block`, `Redact`, `AlertOnly`.

### Domain Events

| Event | Raised By | Payload | Consumers |
|-------|-----------|---------|-----------|
| `cli_invocation_started` | CLI surface | `run_id`, `surface_event_id`, `surface_audit_hash`, `ts`, `analyst` | Observability, Cost |
| `cli_invocation_completed` | CLI surface | `run_id`, `status`, `duration_ms`, `token_usage`, `output_dir` | Observability, Cost, Memory |
| `mcp_tool_started` | MCP surface | `run_id`, `surface_event_id`, `surface_audit_hash`, `ts` | Observability, Cost |
| `mcp_tool_completed` | MCP surface | `run_id`, `status`, `duration_ms`, `token_usage` | Observability, Cost, Memory |
| `plugin_hook_fired` | Plugin surface (Write / Edit / PreToolUse / PostToolUse) | `run_id`, `surface_event_id`, `surface_audit_hash`, `ts`, `status` | Observability, Cost, Memory |
| `audit_failure` | AuditManifest writer or Memory | `run_id`, `error_kind`, `phase` | Observability, alerting |
| `budget_threshold_crossed` | CostBudget | `surface`, `surface_event_id`, `threshold_pct`, `current_usage_usd`, `monthly_limit_usd` | Observability, alerting, analyst notification |
| `pii_detected` | SecurityScan | `scan_id`, `hook_point`, `categories`, `action`, `redacted_count` | Observability, compliance log |
| `prompt_injection_blocked` | SecurityScan | `scan_id`, `hook_point`, `pattern`, `severity` | Observability, alerting |

There are no `cookbook_deploy_*` events: cookbook deploys run outside this context's runtime scope.

### Anti-Corruption Layer

The `corp_finance_core::observability` Rust module (telemetry side) and `corp_finance_core::audit` (audit side) form the boundary. The ACL wraps the surface event format so domain types stay clean of any external-MCP-server-shape leakage: events arriving from MCP handlers, CLI subcommands, and plugin hooks are translated into `AuditManifest`, `TraceSpan`, `CostBudget` domain types at the module edge.

The default v1 implementation is fully native; the ACL exists as a future-proofing boundary should we later choose to integrate a ruflo MCP backend or another telemetry / scanner substrate. For v1 the modules are first-class Rust types with no external runtime dependency.

| Direction | ACL function | Wire format |
|-----------|--------------|-------------|
| MCP tool handler completion → AuditManifest | `from_mcp_completion(&Value) -> AuditManifest` | Validated against the MCP completion event JSON shape |
| CLI subcommand completion → AuditManifest | `from_cli_event(&CliEvent) -> AuditManifest` | Direct from the CLI event struct |
| Plugin Write/Edit hook fire → AuditManifest | `from_plugin_hook(&HookEvent) -> AuditManifest` | Direct from `plugins/cfa-core/hooks/hooks.json` payload |
| TraceSpan emit | `tracing::span!` macros + `tracing-subscriber` exporter | Optional OTLP envelope via `tracing-opentelemetry` feature |
| Cost ledger insert | `rusqlite` upsert into `cost_ledger.sqlite` | N/A in v1 |
| PII / injection scan | native regex set + injection pattern lib + CFA allowlist | Optional `aidefence@2.2.4` npm middleware (HTTP-only; not used by stdio-MCP) |

### Context Map

```
+-------------------------------------------------------------------+
|        Audit / Cost / Observability / Security Context             |
|                                                                   |
|  +------------------+        +------------------+                 |
|  |  AuditManifest   |        |   CostBudget     |                 |
|  |  (root for       |        |   (rusqlite      |                 |
|  |   audit slice    |        |    ledger keyed  |                 |
|  |   per surface)   |        |    by surface)   |                 |
|  +--------+---------+        +---------+--------+                 |
|           |                            |                          |
|           v                            v                          |
|  +--------+---------+        +---------+--------+                 |
|  |   TraceSpan      |        |  SecurityScan    |                 |
|  |   (tracing +     |        |  (native regex   |                 |
|  |    subscriber,   |        |   set + inject   |                 |
|  |    every CLI/    |        |   patterns,      |                 |
|  |    MCP/plugin    |        |   pre/post hook  |                 |
|  |    span)         |        |   points)        |                 |
|  +------------------+        +------------------+                 |
|                                                                   |
+--+----------+----------+-----------+----------+-------------------+
   |          |          |           |          |
   v          v          v           v          v
 cli_inv_  mcp_tool_  plugin_   budget_       pii_
 started/  started/   hook_     threshold_    detected
 completed completed  fired     crossed       prompt_
                                              injection_
                                              blocked
   |          |          |           |          |
   v          v          v           v          v
 +----------------------------+   +--------------------+
 |  Memory Context            |   |  Compliance Log    |
 |  (consumes surface         |   |  (PII / injection  |
 |   events, produces         |   |   audit trail)     |
 |   audit_failure)           |   +--------------------+
 +----------------------------+
```

### Relationship to Other Contexts

| Upstream | Downstream | Relationship | Detail |
|----------|------------|--------------|--------|
| CLI / MCP / Skill / Plugin surfaces | Audit / Cost / Observability | Customer/Supplier | Each surface emits start and completion events; this context consumes them for span lifecycle, cost recording, and audit manifest assembly. Skills are observed via the MCP wrapper. |
| Memory (sibling context) | Audit / Cost / Observability | Publisher/Subscriber | Memory publishes `run_indexed` and `audit_failure`; this context records both into the trace and the audit log. |
| Multi-Agent Coordination (`domain-orchestration.md`) | Audit / Cost / Observability | Publisher/Subscriber | Coordination publishes `agent_invocation_started/completed`, `plan_generated`, `plan_step_executed`; this context records spans for each. |
| Workflow Audit (ADR-009) | Audit / Cost / Observability | Conformist | This context conforms to the djb2 algorithm specified in ADR-009 for `surface_audit_hash` computation. |
| Audit / Cost / Observability | Compliance reporting (out-of-scope downstream) | Open Host Service | The audit log and PII / injection event stream are the API surface compliance teams query. |
| Optional ruflo MCP backend | Audit / Cost / Observability | Anti-Corruption Layer | When the optional `aidefence` npm middleware or another external backend is enabled, JSON wire formats stay behind the ACL; default v1 builds use the native modules with no external dependency. |
