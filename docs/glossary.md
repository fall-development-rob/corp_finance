# CFA Agent Unified Glossary

## Preface

This document consolidates the ubiquitous language from across the CFA Agent platform: the 10 DDD bounded-context docs, 25 ADRs, 17 specflow contracts, and 12 workflow skills. When a term is defined or refined across multiple sources, the most-recent or most-specific definition is canonical; related definitions are linked. This glossary is maintained manually as source docs evolve; if a term lands in two or more docs with conflicting definitions, it appears under "Open ambiguities" below for review.

**Audience**: Platform engineers, specialist CFA agents, integration partners, and compliance auditors.

**Maintenance**: As of Phase 29 Wave 12c, this is a read-only extracted snapshot. Future waves may extend it; edits should flow through the originating doc (DDD context, ADR, or skill manifest) and this glossary should be regenerated via `npx @claude-flow/cli@latest hooks worker dispatch --trigger document`.

---

## Core Domain Concepts

### **Surface**

Enum: `Cli`, `Mcp`, `Skill`, `Plugin`. The four runtime entry points through which the CFA agent system exposes its capabilities.

- **CLI**: Command-line invocations via `cfa <subcommand>` in the corp-finance-cli binary.
- **MCP**: Tool handler calls registered via `server.tool()` in the TypeScript MCP server at `packages/mcp-server/src/`.
- **Skill**: Slash commands and `.claude/skills/*` invoked by the LLM via the Skill tool, recorded via the MCP wrapper.
- **Plugin**: PreToolUse / PostToolUse / Write / Edit hooks configured in `plugins/cfa-core/hooks/hooks.json`.

**Source**: Shared kernel across [domain-orchestration.md](ddd/domain-orchestration.md#surface), [domain-memory.md](ddd/domain-memory.md#surface), and [domain-audit-observability.md](ddd/domain-audit-observability.md#surface).

**Related**: Surface Invocation, Surface Event.

---

### **Surface Invocation**

A single CLI subcommand call, OR a single MCP tool handler execution, OR a plugin hook fire. The unit of indexing, retrieval, auditing, and multi-agent runtime activity.

Skills are recorded via the MCP wrapper through which they execute. A Surface Invocation produces one RunSummary in the Memory context and one AuditManifest if file outputs are involved.

**Source**: [domain-orchestration.md](ddd/domain-orchestration.md#surface-invocation), [domain-memory.md](ddd/domain-memory.md#surface-invocation), [domain-audit-observability.md](ddd/domain-audit-observability.md#surface-invocation).

**Related**: Surface, Surface Event, RunSummary, AuditManifest.

---

### **Surface Event**

A single CLI subcommand call, OR a single MCP tool handler execution, OR a skill invocation (recorded via its MCP wrapper), OR a plugin hook fire. Synonymous with Surface Invocation in most contexts; used in the Self-Learning domain to emphasize the trajectory-construction perspective.

**Source**: [domain-self-learning.md](ddd/domain-self-learning.md#domain-language-ubiquitous-language).

**Related**: Surface Invocation, Trajectory.

---

### **Workflow**

A structured multi-step process that produces a professional financial document. Workflows compose existing computation and data retrieval capabilities into institutional-grade deliverables such as equity research reports, CIMs, IC memos, or due-diligence checklists.

The 12 workflow skills define the orchestration patterns; the MCP tools provide the computation boundary.

**Source**: [domain-financial-workflows.md](ddd/domain-financial-workflows.md#domain-language-ubiquitous-language).

**Related**: Skill, MCP Tool, Specialist Agent.

---

### **Skill**

A slash command (e.g., `/cfa/initiate-coverage`) or a `.claude/skills/*` invocation that routes to a specialist CFA agent. The 12 workflow skills are:

1. **workflow-clean-data-xls** — Data hygiene (outlier detection, unit/currency/frequency reconciliation, period stubs).
2. **workflow-deal-documents** — Cross-cutting document standards (formatting, citation, quality checklists).
3. **workflow-equity-research** — Equity research (initiating coverage, earnings updates, model updates, thesis tracking).
4. **workflow-financial-analysis** — Model audit, deck review, competitive analysis, document QA.
5. **workflow-fund-admin** — Fund accounting (GL reconciliation, NAV tie-out, accrual schedules, variance commentary).
6. **workflow-investment-banking** — Deal execution (CIM, teaser, process letter, buyer list, merger model).
7. **workflow-model-audit** — Model QA (link tracing, formula consistency, balance-sheet integrity).
8. **workflow-operations-kyc** — KYC/AML (customer intake, beneficial ownership, sanctions/PEP screening, source-of-funds).
9. **workflow-pptx-author** — Slide deck authoring (markdown-with-slide-breaks to SlideDeckSpec via `office_pptx_write`).
10. **workflow-private-equity** — PE lifecycle (deal screening, DD, IC memo, returns analysis, unit economics, VCP).
11. **workflow-wealth-management** — Wealth advisory (financial planning, rebalancing, tax-loss harvesting, client reports).
12. **workflow-xlsx-author** — Tabular output (markdown-tabular and CSV conventions; no reader surface).

**Source**: Individual SKILL.md frontmatter in `.claude/skills/workflow-*`.

**Related**: Surface, MCP Tool, Specialist Agent.

---

### **MCP Tool**

A registered function exposed via the TypeScript MCP server at `packages/mcp-server/src/tools/`. The platform exposes 195+ MCP tools from 67 Rust domain modules. MCP tools are the boundary between agent orchestration and computation; they enforce runtime validation via Zod schemas and translate NAPI calls from the Rust computation layer.

**Source**: [DDD.md](ddd/DDD.md) context map; [ADR-025-rust-to-ts-schema-auto-gen.md](adr/ADR-025-rust-to-ts-schema-auto-gen.md).

**Related**: Tool Invocation, NAPI Binding, MCP Server.

---

### **Tool Invocation**

A single call to one of the 195 MCP tools, recording tool name, input params, raw output, duration, and success/failure status. Tracked in the AnalysisResult and recorded in the audit ledger.

**Source**: [DDD.md](ddd/DDD.md) (Specialist Analysts context, ToolInvocation entity).

**Related**: MCP Tool, Audit Manifest, Tool Call Ledger.

---

### **Agent**

An autonomous entity capable of invoking MCP tools and accessing a curated skill set. The 9 CFA agents are:

- **cfa-chief-analyst** — Orchestrator: decomposes queries into research plans, assigns sub-tasks to specialists, aggregates results, and emits goals to the planner.
- **cfa-equity-analyst** — Domain: equity research (valuation, earnings quality, behavioral finance, performance attribution).
- **cfa-credit-analyst** — Domain: credit (credit scoring, credit portfolio, credit derivatives, restructuring, forensics).
- **cfa-fixed-income-analyst** — Domain: fixed income (interest-rate models, inflation-linked, mortgage analytics, repo financing, municipal, sovereign).
- **cfa-derivatives-analyst** — Domain: derivatives (volatility surface, convertibles, structured products, real options, Monte Carlo).
- **cfa-quant-risk-analyst** — Domain: quant and risk (portfolio optimization, risk budgeting, market microstructure, index construction, scenarios).
- **cfa-macro-strategist** — Domain: macro economics (FX/commodities, commodity trading, emerging markets, trade finance, carbon markets).
- **cfa-esg-regulatory-analyst** — Domain: ESG and regulatory (ESG, regulatory reporting, AML/KYC, FATCA/CRS, substance, transfer pricing).
- **cfa-private-markets-analyst** — Domain: private markets (PE, venture, private credit, infrastructure, real assets, CLO analytics, securitization).

**Source**: [DDD.md](ddd/DDD.md) context map (Specialist Analysts bounded context).

**Related**: Chief Analyst, Specialist Agent, Specialist Output, Agent Invocation.

---

### **Specialist Agent**

One of the eight domain-focused CFA agents (all except the Chief Analyst) with access to a curated subset of the 195 MCP tools and a designated domain-expertise skill set. Specialists execute sub-tasks on behalf of the Chief Analyst and return structured analysis results.

**Source**: [DDD.md](ddd/DDD.md) (Analysis Orchestration context, ResearchPlan/AnalystAssignment).

**Related**: Agent, Chief Analyst, Agent Invocation, Specialist Output.

---

### **Chief Analyst**

The cfa-chief-analyst agent that orchestrates the multi-agent system: receives research queries, decomposes them into a structured research plan (GOAP Plan), assigns sub-tasks to specialist agents, aggregates their results, and feeds goals into the Self-Learning planner for cross-domain optimization.

**Source**: [DDD.md](ddd/DDD.md) (Analysis Orchestration context).

**Related**: Agent, Specialist Agent, GOAP Plan, Research Plan.

---

### **Specialist Output**

The structured result returned by a specialist agent invocation: text plus any artefacts written to the working directory. Entity references and domain signals are extracted from specialist output and registered in the entity graph.

**Source**: [domain-orchestration.md](ddd/domain-orchestration.md#specialist-output).

**Related**: Agent Invocation, Entity Reference, Pattern.

---

### **Agent Invocation**

A single Claude Code Agent tool call dispatched by the Chief Analyst (or any other agent) to a specialist. The aggregate root for runtime coordination in the Multi-Agent Coordination context. Carries `invocation_id`, `caller_agent`, `target_agent`, `goal`, `surface`, and on completion, `entity_refs` and `output_hash`.

**Source**: [domain-orchestration.md](ddd/domain-orchestration.md#agentinvocation-aggregate-root).

**Related**: Agent, Specialist Agent, Entity Reference, Surface.

---

### **Tenant**

A distinct legal entity (LP, family, fund, deal team) whose data, audit trail, and outputs must be isolated from other tenants on the same installation. Every CLI subcommand, MCP tool call, and plugin hook fire receives a TenantContext (defaulting to the reserved tenant `local` when no tenant is selected).

**Source**: [domain-federation.md](ddd/domain-federation.md#tenant) (Multi-Tenant Federation context).

**Related**: TenantContext, Federation, Trust Tier, Trust Score.

---

### **TenantContext**

The runtime value object carrying tenant identity, output root (`<repo>/var/tenants/<tenant_id>/out/`), env namespace (`TENANT_<UPPER_KEBAB>_`), audit namespace (`tenants.<tenant_id>`), and PII redaction policy. Supplied to every surface invocation; enforced across filesystem, environment, and audit boundaries.

**Source**: [domain-federation.md](ddd/domain-federation.md#tenantcontext).

**Related**: Tenant, Federation, PII Redaction Policy.

---

### **Trust Tier**

Federation posture indicator: `Open`, `Verified`, or `Trusted`. Maps from the existing `CookbookTier` enum (CoreOnly, Freemium, PaidVendor) at deployment time only; it is not a runtime tenancy concept. Controls whether a federated peer is permitted to access cross-tenant data.

**Source**: [domain-federation.md](ddd/domain-federation.md#trust-tier) and Trust Tier Mapping section.

**Related**: Tenant, Federation Identity, Trust Score, Federated Session.

---

### **Federation**

The multi-tenant, multi-installation collaboration infrastructure. Owns tenant identity, federation session state, PII redaction policy, and behavioral trust scoring. v1 provides within-installation tenant isolation; v2 adds cross-installation handshake and federated sessions.

**Source**: [domain-federation.md](ddd/domain-federation.md) (Multi-Tenant Federation context).

**Related**: Tenant, TenantContext, Federated Session, Trust Attestation.

---

---

## Memory & Retrieval

### **Run Summary**

The canonical structured record of one Surface Invocation; the unit indexed in the trajectory store. Contains `run_id`, `ts`, `surface`, `surface_event_id`, `surface_audit_hash`, `model`, `ticker`, `asset_class`, `recommendation`, `skills_invoked`, `mcp_tools_invoked`, `sub_agents`, `status`, `duration_ms`, `token_usage`. The aggregate root for the Memory bounded context.

**Source**: [domain-memory.md](ddd/domain-memory.md#runsummary-aggregate-root).

**Related**: Surface Invocation, Trajectory, CFA Session, Similar Run.

---

### **Trajectory**

The complete record of one user-facing session: an ordered sequence of Surface Events plus a final eval-grade (A/B/C/D/F or numeric 0-100). Immutable once captured. Used by Self-Learning for clustering and pattern extraction; indexed by HNSW for similarity retrieval.

**Source**: [domain-self-learning.md](ddd/domain-self-learning.md#trajectory-aggregate) and [domain-memory.md](ddd/domain-memory.md).

**Related**: RunSummary, Surface Event, Trajectory Cluster, Eval Grade.

---

### **Entity Reference** (or **EntityRef**)

A typed identifier extracted from specialist output and registered in the entity graph. Kinds: `issuer`, `ticker`, `fund`, `property`, `counterparty`. Shared kernel between Memory and Multi-Agent Coordination contexts; used to detect cross-domain patterns.

**Source**: [domain-orchestration.md](ddd/domain-orchestration.md#entityref) (shared kernel with Memory).

**Related**: Entity Graph, Pattern, Entity Relation.

---

### **Entity Kind**

One of five enumerated identifier types: `issuer`, `ticker`, `fund`, `property`, `counterparty`. Used to type-check entity references during extraction and graph registration.

**Source**: [domain-orchestration.md](ddd/domain-orchestration.md#entityref).

**Related**: Entity Reference, Entity Graph.

---

### **Entity Graph**

The petgraph-backed in-process store that accumulates entity references and relations across specialist outputs within a single user-facing session. Commands: `register_entity()`, `register_relation()`, `query_pattern()`. Supports pattern detection over a time window.

**Source**: [domain-orchestration.md](ddd/domain-orchestration.md#entitygraph-aggregate) (Multi-Agent Coordination context).

**Related**: Entity Reference, Pattern, Pattern Match, Tenant.

---

### **BM25**

Okapi BM25 full-text search ranking algorithm. Implemented via the `tantivy` crate in the Memory context's hybrid retriever. Combined with HNSW vector search and petgraph graph traversal for ranking similar runs.

**Source**: [domain-memory.md](ddd/domain-memory.md) (SimilarRunQuery aggregate).

**Related**: HNSW, Hybrid Retrieval, Similar Run.

---

### **HNSW**

Hierarchical Navigable Small World (HNSW) approximate nearest-neighbor search algorithm. Implemented via the `hnsw_rs` crate to index trajectory embeddings in the Memory context. Enables fast semantic similarity search without exhaustive comparison.

**Source**: [domain-memory.md](ddd/domain-memory.md#trajectoryindex-aggregate) and [ADR-016](adr/ADR-016-memory-architecture.md).

**Related**: BM25, Trajectory Index, Similar Run, Hybrid Retrieval.

---

### **Hybrid Retrieval**

A multi-modal retrieval strategy combining BM25 (keyword), HNSW (vector), and petgraph (graph-hop) rankings, re-ranked by maximal-marginal-relevance (MMR) diversity and composite similarity score. Used to fetch similar past invocations for planning bias and knowledge seeding.

**Source**: [domain-memory.md](ddd/domain-memory.md#similarrunquery-aggregate).

**Related**: BM25, HNSW, Similar Run, MMR Diversity.

---

### **Similar Run**

A past RunSummary retrieved via hybrid (BM25 + vector) search and ranked by composite similarity to a query. Returned set is re-ranked by MMR to penalise near-duplicates and cover different aspects of the query.

**Source**: [domain-memory.md](ddd/domain-memory.md#ubiquitous-language).

**Related**: RunSummary, Hybrid Retrieval, MMR Diversity, Trajectory.

---

### **MMR Diversity** (Maximal Marginal Relevance)

A re-ranking strategy that penalises near-duplicates in retrieval results so the returned set covers different aspects of the query. Weighted by a user-configurable `mmr_lambda` parameter in [0, 1]; default 0.5.

**Source**: [domain-memory.md](ddd/domain-memory.md#ubiquitous-language).

**Related**: Hybrid Retrieval, Similar Run.

---

### **Aggregate Root**

In Domain-Driven Design, the root entity of an aggregate that enforces invariants on the aggregate as a whole. All changes to the aggregate flow through the root. Key roots in the CFA platform: `AnalysisRequest` (Analysis Orchestration), `AnalystAgent` (Specialist Analysts), `AnalysisArchive` (Financial Memory), `LearningPattern` (Learning & Adaptation), `RunSummary` (Memory), `AgentInvocation` (Multi-Agent Coordination), `Trajectory` (Self-Learning).

**Source**: Standard DDD terminology; applied throughout the bounded contexts.

**Related**: Entity, Value Object, Invariant.

---

### **CFA Session**

A persisted, restorable workspace — typically a multi-day diligence session with intermediate state. Backed by a portable session archive (JSON + flate2 gzip) at `~/.cfa-session/<session_id>.cfa-session`. Optionally swappable for RVF format via `rvf_backend` cargo feature.

**Source**: [domain-memory.md](ddd/domain-memory.md#cfasession-aggregate).

**Related**: RunSummary, Portable Session Archive, CFA Session.

---

### **Portable Session Archive**

The native `.cfa-session` archive format: JSON content plus flate2 gzip compression. An optional `rvf_backend` cargo feature can swap this for the upstream `rvf-cli` crate's RVF format. Stores working notes and intermediate artefacts; never stores credentials or PII (enforced by pre-memory-write hook per ADR-017).

**Source**: [domain-memory.md](ddd/domain-memory.md#ubiquitous-language).

**Related**: CFA Session, RunSummary, Audit Pipeline.

---

---

## Multi-Agent Coordination

### **GOAP Plan**

An A* plan tree over the action space (MCP tool actions and slash-command actions) emitted by the chief-analyst's goal decomposition. Each step is either an MCP tool call or a slash-command invocation. Backed by the `pathfinding` crate's A* implementation. Deterministic hash given the same goal and action-space registry version.

**Source**: [domain-orchestration.md](ddd/domain-orchestration.md#goapplan-aggregate) (Multi-Agent Coordination context) and [domain-self-learning.md](ddd/domain-self-learning.md#goapplan-aggregate).

**Related**: Plan Step, Goal, Plan Hash, Replanning.

---

### **Plan Step**

One node in a GOAP Plan: either an MCP tool call or a slash-command invocation. Carries `step_id` (1-indexed), `action_kind`, `action_name`, `preconditions`, and `postconditions`. Depends on prior steps via a DAG of dependencies.

**Source**: [domain-orchestration.md](ddd/domain-orchestration.md#planstep).

**Related**: GOAP Plan, Goal, Action, Dependency.

---

### **Pattern** (or **Pattern Match**)

A signal raised when N specialist outputs touch the same entity within a time window. Detected by the entity graph via the `query_pattern(entity_ref, window)` command. Consumed by Self-Learning to assemble domain signals and fed back to Multi-Agent Coordination for cross-domain routing.

**Source**: [domain-orchestration.md](ddd/domain-orchestration.md#pattern).

**Related**: Entity Reference, Entity Graph, Domain Signal.

---

### **Topology**

The structure of connections between CFA agents. Defined at the agent registry (`.claude/agents/cfa/`) and used by the planner to route goals. Not to be confused with network topology; this is an agent-collaboration topology.

**Source**: [domain-orchestration.md](ddd/domain-orchestration.md) (implicit in Agent Invocation validation).

**Related**: Agent, Chief Analyst, Specialist Agent, Entity Graph.

---

### **Intent**

A natural-language goal submitted by the user or generated by the chief-analyst during goal decomposition. Parsed into a QueryIntent value object (classification: valuation, credit assessment, portfolio construction, etc.) and used as the input to the planner.

**Source**: [DDD.md](ddd/DDD.md) (Analysis Orchestration, QueryIntent value object).

**Related**: QueryIntent, Goal, GOAP Plan, Research Plan.

---

### **AGENT_SKILLS**

An expansion enumeration in Phase 24+ that lists all skills accessible to a given specialist agent. Before Phase 24, agents had access to core corp-finance skills only. Phase 24 onwards adds data retrieval (data-*), geopolitical awareness (geopolitical-*), and vendor intelligence (vendor-*) skills to each agent's capability set.

**Source**: [domain-gap-remediation.md](ddd/domain-gap-remediation.md#1-specialist-analysts-agent-routing).

**Related**: Specialist Agent, Skill, AGENT_INTENTS.

---

### **CFA_INTENTS**

An enumeration of user-facing intents that map to one of the nine CFA agents via the SemanticRouter. Examples: `initiate-coverage` → cfa-equity-analyst, `credit-analysis` → cfa-credit-analyst, `ic-memo` → cfa-private-markets-analyst.

**Source**: [domain-gap-remediation.md](ddd/domain-gap-remediation.md) (3 new slash commands added in Phase 25).

**Related**: Intent, Skill, SemanticRouter, Specialist Agent.

---

### **Goal**

A natural-language predicate that a GOAP plan satisfies. Examples: "compute DCF valuation for AAPL", "produce IC memo for Acme acquisition", "screen deal flow against return criteria". Goals are decomposed from user queries and may cascade through multiple agent invocations.

**Source**: [domain-orchestration.md](ddd/domain-orchestration.md#goapplan-aggregate) and [domain-self-learning.md](ddd/domain-self-learning.md#domain-language-ubiquitous-language).

**Related**: GOAP Plan, Intent, Replanning.

---

### **Action**

One step in the action space: an MCP tool call OR a slash-command invocation. Annotated with preconditions (required entities/inputs) and postconditions (entities/outputs produced) for planning.

**Source**: [domain-self-learning.md](ddd/domain-self-learning.md#domain-language-ubiquitous-language).

**Related**: Plan Step, GOAP Plan, MCP Tool, Slash Command.

---

---

## Federation & Trust

### **Trust Attestation** (or **Federation Identity**)

An ed25519 keypair (via `ed25519-dalek`) plus mTLS certificate (via `rcgen` for self-signed, `rustls` for TLS stack) that uniquely identifies a tenant or peer installation. Stored under `<repo>/var/tenants/<id>/identity/`.

**Source**: [domain-federation.md](ddd/domain-federation.md#federation-identity).

**Related**: Tenant, Trust Score, Federated Session.

---

### **Capability** (or **Trust Capability**)

In the federation context, the set of operations a peer installation is permitted to perform based on its trust score and trust tier. Derived from TrustScore and PIIRedactionPolicy.

**Source**: [domain-federation.md](ddd/domain-federation.md#trust-score-aggregate) (thresholds for inbound, REDACT/HASH/PASS, cross-firm handshake).

**Related**: Trust Score, Trust Tier, Federated Session, PII Redaction Policy.

---

### **Issuer** (or **Issuer Identifier**)

One of the five canonical entity kinds in the entity graph. Represents a corporation, government, or other issuer of securities. Used to link specialist outputs that reference the same issuer across analyses.

**Source**: [domain-orchestration.md](ddd/domain-orchestration.md#entityref).

**Related**: Entity Reference, Entity Kind.

---

### **Subject** (in Audit/Trust context)

The entity that a trust score or PII redaction policy applies to. In federation, the Subject is a peer installation identified by its Federation Identity.

**Source**: [domain-federation.md](ddd/domain-federation.md#federatedsession-aggregate-v2-only).

**Related**: Trust Score, Federation Identity, Audit Namespace.

---

### **Revocation** (PII/Trust context)

The act of withdrawing a certificate, trust attestation, or session capability. Triggered when trust score crosses a downgrade threshold or when a peer's mTLS cert is revoked.

**Source**: [domain-federation.md](ddd/domain-federation.md) (Trust Tier section).

**Related**: Trust Score, Trust Downgrade, Federated Session.

---

### **Federated Session** (v2 only)

A live cross-installation collaboration with a peer; carries handshake state, redaction policy, and trust score. State machine: `Initiated -> Authenticated -> Authorised -> Active` or `Initiated -> Failed`. Enters `Hold` on `trust_downgrade`.

**Source**: [domain-federation.md](ddd/domain-federation.md#federatedsession-aggregate-v2-only).

**Related**: Federation, Trust Score, PII Redaction Policy, Cross-Firm Handshake.

---

### **Cross-Firm Handshake**

The signed agreement required for `PaidVendor`-tagged surface targets in v2 federation; both peers cryptographically authorise vendor data redistribution. Threshold: trust score >= 0.95.

**Source**: [domain-federation.md](ddd/domain-federation.md#cross-firm-handshake).

**Related**: Federated Session, Trust Tier, Trust Score.

---

### **Trust Score**

A native behavioral score in [0, 1] computed as `0.4 * success + 0.2 * uptime + 0.2 * threat + 0.2 * integrity`, persisted in the cost ledger SQLite store. Thresholds: >= 0.70 for inbound cross-installation, >= 0.80 for REDACT/HASH PII, >= 0.90 for PASS PII, >= 0.95 for cross-firm handshake.

**Source**: [domain-federation.md](ddd/domain-federation.md#trust-score-aggregate).

**Related**: Trust Tier, Capability, Trust Downgrade.

---

### **Trust Downgrade**

An event raised when a peer's trust score crosses a threshold downward; pending actions enter hold state and the federated session transitions to Hold.

**Source**: [domain-federation.md](ddd/domain-federation.md#trust-downgrade).

**Related**: Trust Score, Federated Session.

---

### **PII Redaction Policy**

Per-tenant configuration of one of four modes (BLOCK, REDACT, HASH, PASS) for each of the 14 PII types (names+address, emails, phones, addresses, SSN, passport, drivers' licence, bank account, credit card, IBAN, IP, MAC, DoB, government ID/EIN). Default for new tenants: BLOCK for all types.

**Source**: [domain-federation.md](ddd/domain-federation.md#piiredactionpolicy-aggregate) and [domain-audit-observability.md](ddd/domain-audit-observability.md#pii-category).

**Related**: Tenant, TenantContext, PII Category, Security Scan.

---

---

## Audit & Observability

### **Audit Manifest**

The structured `.audit.json` companion file written when a Write or Edit plugin hook fires or when a CLI / MCP invocation produces a numeric output. Contains `run_id`, `surface`, `surface_event_id`, `surface_audit_hash`, `model`, `tool_call_ledger`, `skills_in_scope`, `sub_agents_in_scope`. The aggregate root for the audit context.

**Source**: [domain-audit-observability.md](ddd/domain-audit-observability.md#auditmanifest-aggregate-root).

**Related**: Surface Invocation, Tool Call Ledger, sha256, Surface Audit Hash.

---

### **Surface Audit Hash**

A djb2 fingerprint over the surface configuration that produced the invocation (CLI subcommand version, MCP tool registration, plugin hook configuration). Computed deterministically and used to detect drift across runs and validate audit manifest integrity.

**Source**: [domain-audit-observability.md](ddd/domain-audit-observability.md#surface-audit-hash) and [ADR-009](adr/ADR-009-workflow-rust-auditability.md).

**Related**: Audit Manifest, Workflow Audit, djb2.

---

### **Call ID**

A UUID v4 issued at the moment an MCP tool invocation begins. Used as the unique key in the audit log (`call_id`) and to link the audit record to the parent RunSummary and AuditManifest.

**Source**: [ADR-024](adr/ADR-024-mcp-tool-call-audit.md) (MCP Tool-Call Audit Middleware).

**Related**: Audit Manifest, MCP Tool, Audit Pipeline.

---

### **Timestamp**

An ISO 8601 UTC datetime recording when a surface invocation began. Carried in RunSummary (`ts`), AuditManifest (`ts`), and MCP audit logs. Used for temporal filtering and event ordering.

**Source**: All DDD contexts (RunSummary, AuditManifest, TraceSpan, Audit Manifest, Audit Pipeline).

**Related**: RunSummary, Audit Manifest, Observability.

---

### **sha256**

SHA-256 cryptographic hash. Used for stable audit primitives: `surface_audit_hash` (djb2 over surface config), `input_hash` (djb2 over MCP input), `output_hash` (djb2 over MCP output in audit log), and `WriteWorkbookResult.sha256`, `WriteDocResult.sha256`, `WriteDeckResult.sha256` (deterministic hashes of generated office documents).

**Source**: Multiple contexts — [ADR-009](adr/ADR-009-workflow-rust-auditability.md) (djb2), [ADR-021](adr/ADR-021-office-ooxml-serialization.md), [ADR-022](adr/ADR-022-office-docx-serialization.md), [ADR-023](adr/ADR-023-office-pptx-serialization.md), [ADR-024](adr/ADR-024-mcp-tool-call-audit.md).

**Related**: Audit Manifest, Tool Call Ledger, Drift Detection.

---

### **Tool Call Ledger**

The deterministic sequence of MCP tool calls behind a single output, captured in the audit manifest. Each entry records `step` (1-indexed), `tool` (name), `input_hash` (djb2), `output_hash` (djb2).

**Source**: [domain-audit-observability.md](ddd/domain-audit-observability.md#toolcall).

**Related**: Audit Manifest, MCP Tool, Tool Invocation.

---

### **Run ID**

A UUID v7 (sortable by time) issued when a surface invocation begins. Unique identifier for the runthrough of a single CLI subcommand, MCP tool, or plugin hook. Appears in RunSummary, AuditManifest, TraceSpan, and audit logs as the stable correlation key.

**Source**: [domain-memory.md](ddd/domain-memory.md#runsummary-aggregate-root) and all audit/observability contexts.

**Related**: Audit Manifest, RunSummary, Call ID, Timestamp.

---

### **Trace Span**

A structured unit of telemetry recording one operation with attributes, start, end, and parent reference, emitted via the `tracing` crate. The aggregate for structured observability, supporting optional OTLP export. Spans cover every CLI subcommand, every MCP tool handler, and every plugin hook fire.

**Source**: [domain-audit-observability.md](ddd/domain-audit-observability.md#tracespan-aggregate).

**Related**: Observability, Tracing, Span ID, TraceID.

---

### **Audit Pipeline**

The unified event stream from surface invocations into the audit layer. Emits `cli_invocation_started/completed`, `mcp_tool_started/completed`, `plugin_hook_fired`, `audit_failure`, `budget_threshold_crossed`, `pii_detected`, `prompt_injection_blocked`. Backed by native Rust modules in `corp_finance_core::audit`.

**Source**: [domain-audit-observability.md](ddd/domain-audit-observability.md#domain-events).

**Related**: Audit Manifest, Memory, Cost Ledger, Security Scan.

---

### **Cost Budget**

The configured budget for a surface target and its current state. Persisted in SQLite at `<repo>/var/observability/cost-ledger.sqlite`. Keys on `(surface, surface_event_id)` tuple. Tracks monthly USD limit, warn thresholds (%), hard stop, and current usage.

**Source**: [domain-audit-observability.md](ddd/domain-audit-observability.md#costbudget-aggregate).

**Related**: Audit Pipeline, Cost Ledger, Budget Threshold Crossed.

---

### **Cost Ledger**

The SQLite database backing cost tracking. Located at `<repo>/var/observability/cost-ledger.sqlite`. Stores CostBudget rows keyed by `(surface, surface_event_id)` and TrustScore samples per peer (federation context).

**Source**: [domain-federation.md](ddd/domain-federation.md) and [domain-audit-observability.md](ddd/domain-audit-observability.md).

**Related**: Cost Budget, Trust Score, Audit Pipeline.

---

### **Security Scan**

A pass over content (input, memory write, final output) that detects PII categories and prompt-injection patterns via the native scanner in `corp_finance_core::observability::security_scan`. Scans fire at hook points: `pre-tool-call`, `pre-memory-write`, `post-invocation`.

**Source**: [domain-audit-observability.md](ddd/domain-audit-observability.md#securityscan-aggregate).

**Related**: PII Category, PII Finding, Injection Finding, Audit Pipeline.

---

### **PII Category**

One of 14 types defined and owned by the platform: names+address tuples, emails, phones, addresses, SSN, passport, drivers' licence, bank account, credit card, IBAN, IP address, MAC address, date of birth, government ID / EIN. Detected by the native security scanner; actions (BLOCK/REDACT/HASH/PASS) are configured per-tenant in PIIRedactionPolicy.

**Source**: [domain-audit-observability.md](ddd/domain-audit-observability.md#pii-category).

**Related**: PII Redaction Policy, Security Scan, PII Finding.

---

### **Prompt Injection**

A pattern in user input or retrieved memory designed to manipulate the model's instructions. Detected by pattern-library scanning in the native security scanner. Actions: `pre-tool-call` detections block the call; `post-invocation` detections alert only.

**Source**: [domain-audit-observability.md](ddd/domain-audit-observability.md#prompt-injection).

**Related**: Security Scan, Injection Finding, Audit Pipeline.

---

### **Allowlist** (CFA Allowlist)

The CFA-specific set of identifiers that suppress PII false positives: CUSIP, ISIN, SEDOL, FIGI, LEI, ticker. Consulted by the security scanner before adding a match to `pii_findings`.

**Source**: [domain-audit-observability.md](ddd/domain-audit-observability.md#allowlist).

**Related**: PII Finding, Security Scan, Allowlist.

---

---

## Office / OOXML

### **WorkbookSpec**

The input specification for .xlsx generation. Consists of sheets, each containing cells with values (text, number, decimal, bool, datetime, empty, formula, frozen panes). Decimal cells carry a string representation and are parsed + rounded before OOXML emission (the canonical Decimal→f64 boundary exception).

**Source**: [ADR-021](adr/ADR-021-office-ooxml-serialization.md) (Office OOXML Serialization — .xlsx).

**Related**: CellValue, WriteWorkbookResult, Office Module.

---

### **WordDocSpec**

The input specification for .docx generation. Consists of sections, each containing DocBlocks (Heading, Paragraph, Table, BulletList, NumberedList, PageBreak). Tagged enum with serde discriminant `"type"`.

**Source**: [ADR-022](adr/ADR-022-office-docx-serialization.md) (Office OOXML Serialization — .docx).

**Related**: DocBlock, WriteDocResult, Office Module.

---

### **SlideDeckSpec**

The input specification for .pptx generation. Consists of slides, each one of four kinds (Title, Section, Content, Table). Tagged enum with serde discriminant `"kind"`. v1 scope: Title, Section, Content, Table only (no images, charts, animations, speaker notes, custom themes, hyperlinks).

**Source**: [ADR-023](adr/ADR-023-office-pptx-serialization.md) (Office OOXML Serialization — .pptx).

**Related**: Slide, WriteDeckResult, Office Module.

---

### **CellValue**

Tagged enum for XLSX cell content: `Text | Number | Decimal | Bool | DateTime | Empty`. The Decimal case is parsed as string, converted to f64, and emitted as IEEE 754 double.

**Source**: [ADR-021](adr/ADR-021-office-ooxml-serialization.md).

**Related**: WorkbookSpec, Decimal→f64 Exception.

---

### **DocBlock**

Tagged enum for DOCX content: `Heading | Paragraph | Table | BulletList | NumberedList | PageBreak`. Serde discriminant: `"type"`. Heading level must be in 1..=3.

**Source**: [ADR-022](adr/ADR-022-office-docx-serialization.md).

**Related**: WordDocSpec, Slide (pptx equivalent).

---

### **Slide** (or **SlideDeck Slide**)

Tagged enum for PPTX content (v1): `Title | Section | Content | Table`. Serde discriminant: `"kind"`. A Title slide has title and optional subtitle. A Section slide is a divider. A Content slide has title and bullet list. A Table slide has title and data grid.

**Source**: [ADR-023](adr/ADR-023-office-pptx-serialization.md).

**Related**: SlideDeckSpec, DocBlock (docx equivalent).

---

### **Template**

In the office context, a pre-designed structure (sheet layout, section format, slide master) that is populated with data. The office module produces terminal deliverables (write-only); there is no reader surface and no template database. Templates are conceptual (e.g., "IC memo template" exists as a SlideDeckSpec or WordDocSpec, not as a persistent artefact).

**Source**: [ADR-021](adr/ADR-021-office-ooxml-serialization.md), [ADR-022](adr/ADR-022-office-docx-serialization.md), [ADR-023](adr/ADR-023-office-pptx-serialization.md).

**Related**: WorkbookSpec, WordDocSpec, SlideDeckSpec, Terminal Deliverable.

---

### **Tearsheet**

A compact one-page or two-page financial summary (or pitch sheet) typically used for deal marketing or sector overview. Represented as either a WorkbookSpec (single sheet) or SlideDeckSpec (single Content slide), depending on distribution channel.

**Source**: Implied in [domain-financial-workflows.md](ddd/domain-financial-workflows.md) (Strip Profile entity).

**Related**: Strip Profile, Datapack, WorkbookSpec, SlideDeckSpec.

---

### **Terminal Deliverable**

An output file produced by a single surface invocation and consumed by a human or external system. The office module enforces write-only invariants: no reader surface, no round-trip path back into the agent stack. `WriteWorkbookResult`, `WriteDocResult`, and `WriteDeckResult` are the sole system-of-record handles. Invariant: if a file is terminal, you may write it but never read it back.

**Source**: [ADR-021](adr/ADR-021-office-ooxml-serialization.md#terminal-deliverable-invariant), [ADR-022](adr/ADR-022-office-docx-serialization.md#terminal-deliverable-invariant), [ADR-023](adr/ADR-023-office-pptx-serialization.md#terminal-deliverable-invariant).

**Related**: WriteWorkbookResult, WriteDocResult, WriteDeckResult, Office Module.

---

### **WriteWorkbookResult**

Return value from `write_workbook()`: `{ output_path, bytes_written, sha256, sheet_count }`. The sole system-of-record handle post-write. The `sha256` field is stable across identical inputs and used for audit primitives.

**Source**: [ADR-021](adr/ADR-021-office-ooxml-serialization.md).

**Related**: WorkbookSpec, Terminal Deliverable, Audit Manifest.

---

### **WriteDocResult**

Return value from `write_word_doc()`: `{ output_path, bytes_written, sha256, section_count }`. The sole system-of-record handle post-write. The `sha256` field is stable across identical inputs.

**Source**: [ADR-022](adr/ADR-022-office-docx-serialization.md).

**Related**: WordDocSpec, Terminal Deliverable, Audit Manifest.

---

### **WriteDeckResult**

Return value from `write_slide_deck()`: `{ output_path, bytes_written, sha256, slide_count }`. The sole system-of-record handle post-write. The `sha256` field is stable across identical inputs.

**Source**: [ADR-023](adr/ADR-023-office-pptx-serialization.md).

**Related**: SlideDeckSpec, Terminal Deliverable, Audit Manifest.

---

### **Office Module** (or **Cargo Feature `office`**)

A unified namespace in `corp_finance_core::office` gating .xlsx, .docx, and .pptx write surfaces. Feature-gated at `crates/corp-finance-core/Cargo.toml` behind the `office` cargo feature. Without the feature, the platform binary is unchanged in size and behavior.

**Source**: [ADR-021](adr/ADR-021-office-ooxml-serialization.md#engine-and-feature-flag), [ADR-022](adr/ADR-022-office-docx-serialization.md#engine-and-feature-flag), [ADR-023](adr/ADR-023-office-pptx-serialization.md#engine-and-feature-flag).

**Related**: WorkbookSpec, WordDocSpec, SlideDeckSpec, Cargo Feature.

---

---

## Schema Generation

### **schemars**

A Rust crate providing derive macros that emit JSON Schema from Rust types. Used to generate `#[derive(JsonSchema)]` annotations on Rust domain types in `corp_finance_core`.

**Source**: [ADR-025](adr/ADR-025-rust-to-ts-schema-auto-gen.md) (Rust-to-TypeScript Schema Auto-Generation).

**Related**: JsonSchema, Generated Schema, schema_gen Feature.

---

### **JsonSchema** (JSON Schema)

A lingua franca for describing JSON document structure, validation, and constraints. Produced by `schemars` from Rust types; consumed by `json-schema-to-zod` to produce TypeScript Zod validators.

**Source**: [ADR-025](adr/ADR-025-rust-to-ts-schema-auto-gen.md).

**Related**: schemars, Zod, json-schema-to-zod.

---

### **Generated Schema**

A hand-forbidden schema file produced by the pipeline `schemars` → JSON Schema → `json-schema-to-zod` → TypeScript Zod. Canonical source: the Rust struct. Landing directory: `packages/mcp-server/src/schemas/generated/<domain>/`. Invariant SCHEMA-INV-001: hand-editing generated files is forbidden.

**Source**: [ADR-025](adr/ADR-025-rust-to-ts-schema-auto-gen.md).

**Related**: Hand-maintained Schema, schema_gen Feature, Zod.

---

### **Hand-maintained Schema**

A TypeScript Zod validator file authored manually and maintained alongside a Rust domain type. As of Phase 29 Wave 11, 83 hand-maintained schema files remain (non-office domains). Office domain (25 types) transitioned to Generated Schema during Wave 11 pilot.

**Source**: [ADR-025](adr/ADR-025-rust-to-ts-schema-auto-gen.md#pilot-scope).

**Related**: Generated Schema, Zod, Drift Detection.

---

### **schema_gen Feature** (Cargo Feature)

A new cargo feature in `crates/corp-finance-core/Cargo.toml` that gates schema generation. NOT included in the `full` feature set. The `schemars` derive and integration test are compiled only with `--features schema_gen`. Zero runtime overhead: the NAPI bindings and MCP server process load no code from this feature.

**Source**: [ADR-025](adr/ADR-025-rust-to-ts-schema-auto-gen.md#cargo-feature-gate).

**Related**: Generated Schema, Cargo Feature, Office Module.

---

### **json-schema-to-zod**

An npm utility that converts JSON Schema documents to TypeScript Zod validators. Part of the schema generation pipeline. Preserves validation constraints and discriminator hints from JSON Schema.

**Source**: [ADR-025](adr/ADR-025-rust-to-ts-schema-auto-gen.md#pipeline).

**Related**: JsonSchema, Zod, schemars, schema_gen Feature.

---

---

## Cargo Features

### **full**

The default feature set. Includes all computation modules, data integration, audit/observability, federation, and self-learning. Does NOT include `office` or `schema_gen` (which must be explicitly requested).

**Source**: `crates/corp-finance-core/Cargo.toml`.

**Related**: office, federation, schema_gen, multi_agent.

---

### **office**

Optional feature gating .xlsx, .docx, and .pptx write surfaces. Depends on `rust_xlsxwriter`, `docx-rs`, optional pptx engine, and `sha2`. Without it, the platform binary is unchanged; the CLI subcommand `corp-finance-cli office *` is not available.

**Source**: [ADR-021](adr/ADR-021-office-ooxml-serialization.md), [ADR-022](adr/ADR-022-office-docx-serialization.md), [ADR-023](adr/ADR-023-office-pptx-serialization.md).

**Related**: WorkbookSpec, WordDocSpec, SlideDeckSpec, schema_gen.

---

### **federation**

Optional feature enabling multi-tenant isolation, federation session handshakes, and trust score computation. Depends on `rustls`, `ed25519-dalek`, `rcgen`, `rusqlite`. When enabled, every surface invocation receives a TenantContext and PII redaction is enforced.

**Source**: [domain-federation.md](ddd/domain-federation.md).

**Related**: Tenant, TenantContext, Trust Tier, PII Redaction Policy.

---

### **multi_agent**

Optional feature enabling multi-agent coordination, GOAP planning, and entity graph operations. Depends on `petgraph` and `pathfinding`. When disabled, only single-agent (Chief Analyst) execution is supported.

**Source**: [domain-orchestration.md](ddd/domain-orchestration.md) (Multi-Agent Coordination context).

**Related**: GOAP Plan, Entity Graph, Agent, Specialist Agent.

---

### **self_learning**

Optional feature enabling trajectory capture, trajectory clustering, pattern extraction, and drift detection. Depends on cluster-learning infrastructure (k-means worker), ed25519 signing for golden-set manifests, and the native memory store. Without it, planning bias and replay detection are unavailable.

**Source**: [domain-self-learning.md](ddd/domain-self-learning.md).

**Related**: Trajectory, Trajectory Cluster, Golden Set, Drift Detection.

---

### **schema_gen**

Optional feature gating Rust→JSON Schema→Zod schema generation. NOT included in `full`. Depends on `schemars` crate. Compiled only when explicitly requested via `--features schema_gen`; zero runtime overhead.

**Source**: [ADR-025](adr/ADR-025-rust-to-ts-schema-auto-gen.md).

**Related**: schemars, JsonSchema, generated Schema.

---

### **audit**

Optional feature enabling comprehensive audit manifest generation, tool-call ledger recording, and event-stream emission to the audit pipeline. When disabled, only minimal observability hooks are present.

**Source**: [ADR-017](adr/ADR-017-audit-cost-observability.md) and [domain-audit-observability.md](ddd/domain-audit-observability.md).

**Related**: Audit Manifest, Audit Pipeline, Tool Call Ledger.

---

### **observability**

Optional feature enabling structured tracing via `tracing` + `tracing-subscriber`, with optional OTLP export. Provides span lifecycle, attribute collection, and telemetry export. Depends on `tracing` and optional `tracing-opentelemetry`.

**Source**: [domain-audit-observability.md](ddd/domain-audit-observability.md#tracespan-aggregate).

**Related**: Trace Span, Audit Pipeline.

---

### **security**

Optional feature enabling the native PII and prompt-injection security scanner. Depends on `regex` and the hand-rolled pattern library. When disabled, security scans are no-op.

**Source**: [domain-audit-observability.md](ddd/domain-audit-observability.md#securityscan-aggregate).

**Related**: Security Scan, PII Category, Prompt Injection.

---

### **cost**

Optional feature enabling cost budget tracking, ledger persistence to SQLite, and threshold-crossing alerts. Depends on `rusqlite`. When disabled, cost tracking is not available.

**Source**: [domain-audit-observability.md](ddd/domain-audit-observability.md#costbudget-aggregate).

**Related**: Cost Budget, Cost Ledger, Audit Pipeline.

---

---

## Skills (Workflow Orchestration)

### **workflow-clean-data-xls**

Data Hygiene Workflows — outlier detection, unit and sign reconciliation, frequency alignment (monthly/quarterly/annual), unit conversion (000s/millions/billions), currency normalisation, period-stub handling, and data lineage tracking before any number enters a financial model. Routes to cfa-chief-analyst for data-quality sign-off.

**Source**: `.claude/skills/workflow-clean-data-xls/SKILL.md`.

**Related**: Skill, Workflow, Data Quality.

---

### **workflow-deal-documents**

Deal Document Standards — cross-cutting document production standards for institutional financial deliverables: confidentiality disclaimers, professional formatting conventions, output specifications, quality checklists, citation standards, number formatting rules. Shared reference for all deal-related document workflows across IB, PE, ER, WM.

**Source**: `.claude/skills/workflow-deal-documents/SKILL.md`.

**Related**: Skill, Workflow, Confidentiality, Citation.

---

### **workflow-equity-research**

Equity Research Workflows — professional equity research document workflows: initiating coverage reports, earnings updates, morning notes, model updates, thesis tracking, catalyst calendars, idea generation, sector overviews. Orchestrates corp-finance-mcp computation tools and FMP market data tools. Routes to cfa-equity-analyst.

**Source**: `.claude/skills/workflow-equity-research/SKILL.md`.

**Related**: Skill, Workflow, Specialist Agent (cfa-equity-analyst).

---

### **workflow-financial-analysis**

Financial Analysis Workflows — quality assurance and competitive analysis workflows: model checking and auditing, presentation/deck review, competitive analysis frameworks, document formatting standards. Routes to cfa-chief-analyst for model QA.

**Source**: `.claude/skills/workflow-financial-analysis/SKILL.md`.

**Related**: Skill, Workflow, Model Audit.

---

### **workflow-fund-admin**

Fund Administration Workflows — professional fund administration and accounting operations: GL break reconciliation, NAV tie-out for LP statements, period-end accrual schedules, period-over-period roll-forward, FP&A variance commentary, break-tracing across GL and sub-ledger systems. Routes to cfa-quant-risk-analyst.

**Source**: `.claude/skills/workflow-fund-admin/SKILL.md`.

**Related**: Skill, Workflow, Fund Accounting.

---

### **workflow-investment-banking**

Investment Banking Workflows — professional IB deal execution document workflows: CIM drafting, teasers, process letters, buyer lists, merger models, pitch decks, strip profiles, deal tracking, datapack assembly. Sell-side and buy-side advisory using corp-finance-mcp tools and FMP data. Routes to cfa-private-markets-analyst.

**Source**: `.claude/skills/workflow-investment-banking/SKILL.md`.

**Related**: Skill, Workflow, Specialist Agent (cfa-private-markets-analyst).

---

### **workflow-model-audit**

Model Audit Workflows — financial-model audit QA: link tracing, formula consistency, hardcode detection, circular reference assessment, sensitivity stress testing, balance-sheet integrity, three-statement tie-out, re-derivation against corp-finance-mcp tools. Routes to cfa-chief-analyst for model QA.

**Source**: `.claude/skills/workflow-model-audit/SKILL.md`.

**Related**: Skill, Workflow, Model Audit.

---

### **workflow-operations-kyc**

KYC Operations Workflows — KYC/AML operational workflows: customer intake, beneficial-ownership verification, sanctions screening (OFAC, EU, HMT, UN, FATF), PEP screening, country risk classification, source-of-funds documentation (SDD/CDD/EDD), ongoing monitoring, periodic review. Routes to cfa-esg-regulatory-analyst.

**Source**: `.claude/skills/workflow-operations-kyc/SKILL.md`.

**Related**: Skill, Workflow, Compliance, KYC, AML.

---

### **workflow-pptx-author**

Slide Deck Authoring Workflows — markdown-with-slide-breaks deck authoring conventions for headless pitch and research decks. Phase 29 Wave 8 adds native PPTX writer via `office_pptx_write` MCP tool; maps markdown deck to SlideDeckSpec and calls the writer to produce binary .pptx terminal deliverable. Routes to cfa-private-markets-analyst (IB/PE) or cfa-equity-analyst (research).

**Source**: `.claude/skills/workflow-pptx-author/SKILL.md`.

**Related**: Skill, Workflow, SlideDeckSpec, WriteDeckResult, Terminal Deliverable.

---

### **workflow-private-equity**

Private Equity Workflows — professional PE deal lifecycle workflows: deal sourcing and screening, due diligence checklists, DD meeting prep, IC memos, returns analysis, unit economics, value creation plans, portfolio monitoring. Institutional PE document production pipelines using corp-finance-mcp tools. Routes to cfa-private-markets-analyst.

**Source**: `.claude/skills/workflow-private-equity/SKILL.md`.

**Related**: Skill, Workflow, Specialist Agent (cfa-private-markets-analyst).

---

### **workflow-wealth-management**

Wealth Management Workflows — professional wealth management client workflows: client meeting prep, financial planning, portfolio rebalancing, tax-loss harvesting, client reports, investment proposals. Advisory document production using corp-finance-mcp portfolio, retirement, and tax tools. Routes to cfa-quant-risk-analyst.

**Source**: `.claude/skills/workflow-wealth-management/SKILL.md`.

**Related**: Skill, Workflow, Specialist Agent (cfa-quant-risk-analyst).

---

### **workflow-xlsx-author**

Tabular Output Authoring Workflows — markdown-tabular and CSV authoring conventions for headless Excel-equivalent deliverables: header row standards, units row, source-of-truth columns, formula columns rendered as =CELL text, cross-references as ->Sheet:Cell, named-range substitutes via heading anchors. No Excel writer in headless environment; markdown/CSV structure so recipients paste into Excel without rework. Routes to cfa-chief-analyst.

**Source**: `.claude/skills/workflow-xlsx-author/SKILL.md`.

**Related**: Skill, Workflow, WorkbookSpec (Phase 29 Wave 6 adds native .xlsx writer).

---

---

## Open Ambiguities

(None detected in this extraction. All terms with multiple definitions across docs have been harmonized to the most-recent or most-specific version with related-term cross-links.)

---

## Index by Source

### Bounded Contexts (DDD)
- **domain-orchestration.md**: Surface Invocation, GOAP Plan, Plan Step, Pattern, EntityRef, Entity Kind, Entity Graph, Entity Relation, AgentInvocation, Specialist Output, Surface.
- **domain-memory.md**: RunSummary, Trajectory, Similar Run, BM25, HNSW, CFA Session, Portable Session Archive, Hybrid Retrieval.
- **domain-self-learning.md**: Trajectory, Trajectory Cluster, Action, Eval Grade, Golden Set, Drift Detection, Domain Signal.
- **domain-federation.md**: Tenant, TenantContext, Trust Tier, Federation, Trust Attestation, PII Redaction Policy, Trust Score, Federated Session.
- **domain-audit-observability.md**: Audit Manifest, Surface Audit Hash, Cost Budget, Cost Ledger, Trace Span, Security Scan, PII Category, Prompt Injection, Tool Call Ledger.
- **domain-financial-workflows.md**: Workflow, Initiating Coverage, CIM, IC Memo, Teaser, Process Letter, Datapack, VCP, TLH, Morning Note.

### ADRs (Architecture Decision Records)
- **ADR-021 (Office OOXML Serialization — .xlsx)**: WorkbookSpec, CellValue, WriteWorkbookResult, Office Module, Terminal Deliverable.
- **ADR-022 (Office OOXML Serialization — .docx)**: WordDocSpec, DocBlock, WriteDocResult, Terminal Deliverable.
- **ADR-023 (Office OOXML Serialization — .pptx)**: SlideDeckSpec, Slide, WriteDeckResult, Terminal Deliverable.
- **ADR-024 (MCP Tool-Call Audit)**: Call ID, Audit Record Schema, Append Strategy, Audit Failure Isolation.
- **ADR-025 (Rust-to-TypeScript Schema Auto-Gen)**: schemars, JsonSchema, Generated Schema, Hand-maintained Schema, schema_gen Feature, json-schema-to-zod.

### Skills (Workflow Orchestration)
- **workflow-clean-data-xls**: Data Hygiene.
- **workflow-deal-documents**: Document Standards.
- **workflow-equity-research**: Equity Research.
- **workflow-financial-analysis**: Financial Analysis, Model Audit.
- **workflow-fund-admin**: Fund Administration.
- **workflow-investment-banking**: Investment Banking.
- **workflow-model-audit**: Model Audit.
- **workflow-operations-kyc**: KYC Operations.
- **workflow-pptx-author**: Slide Deck Authoring.
- **workflow-private-equity**: Private Equity.
- **workflow-wealth-management**: Wealth Management.
- **workflow-xlsx-author**: Tabular Output Authoring.

### Contracts (Specflow)
- Invariant prefix enumeration: ROUTE-INV, ARCH-INV, RUF-AUD-INV, RUF-COST-INV, RUF-OBS-INV, RUF-SEC-INV, RUF-FED-INV.

---

Last regenerated: 2026-05-08 — manual extraction; consider automating once docs are stable.
