# Product Requirements Document: CFA Analyst Platform

**Product**: Autonomous CFA Analyst Platform
**Package**: @robotixai/corp-finance-mcp
**Version**: 1.0 Draft
**Date**: 2026-02-12
**Author**: RobotixAI Engineering

---

## 1. Vision

An autonomous CFA analyst platform where specialized AI agents collaborate to produce
institutional-quality financial research, powered by 215 precision calculation tools
built in Rust with 128-bit decimal arithmetic. The platform transforms a library of
71 domain modules and 5,879 validated tests into a living analytical workforce --
agents that reason like senior analysts, compute like terminals, and learn from
every engagement.

The end state: a portfolio manager asks "Walk me through the risk-adjusted return
profile of this LBO under three rate scenarios with covenant stress testing" and
receives a structured, auditable, terminal-grade response from cooperating specialists.

---

## 2. User Personas

### 2.1 Portfolio Manager (PM)
- **Role**: Manages $500M+ AUM across equity and credit.
- **Need**: Automated equity and credit research that matches sell-side depth.
- **Pain**: Spends 6+ hours per name on manual model building and scenario analysis.
- **Success**: Full equity research note with DCF, comps, and risk factors in under 60 seconds.

### 2.2 Investment Analyst
- **Role**: Buy-side analyst covering 30-50 names across sectors.
- **Need**: Rapid multi-asset analysis with consistent methodology.
- **Pain**: Context-switching between Bloomberg, Excel, and internal tools.
- **Success**: Single interface for valuation, credit analysis, and relative value.

### 2.3 Risk Manager
- **Role**: Oversees portfolio-level risk for a fund or institution.
- **Need**: Portfolio risk decomposition, stress testing, and scenario analysis.
- **Pain**: Aggregating risk across asset classes with different models.
- **Success**: Real-time portfolio VaR, Greeks, factor exposures, and tail risk metrics.

### 2.4 Corporate Finance Professional
- **Role**: M&A advisor, restructuring consultant, or internal corp-dev team.
- **Need**: Deal analysis -- LBO models, merger accretion/dilution, restructuring waterfalls.
- **Pain**: Building complex deal models from scratch for each engagement.
- **Success**: Parameterized deal models with sensitivity tables generated on demand.

### 2.5 Wealth Advisor
- **Role**: Manages client portfolios across risk profiles.
- **Need**: Client portfolio construction, rebalancing, and reporting.
- **Pain**: Translating investment committee views into client-specific allocations.
- **Success**: Optimized portfolio proposals with risk attribution and fee analysis.

---

## 3. Core Features

### P0 -- MVP (Weeks 1-6)

#### 3.0.1 Hosted MCP Server

Remote access to all 215 tools via HTTP/SSE, removing the need for local Rust builds.

- **Transport**: MCP protocol over SSE (Server-Sent Events) and STDIO.
- **Authentication**: API key-based access with rate limiting.
- **Discovery**: Full tool manifest with typed schemas, descriptions, and examples.
- **Deployment**: Single container image, deployable to any cloud or bare metal.

| Metric               | Target          |
|-----------------------|-----------------|
| Tool invocation p50   | < 50ms          |
| Tool invocation p99   | < 200ms         |
| Uptime                | 99.9%           |
| Concurrent sessions   | 100+            |

#### 3.0.2 Single CFA Analyst Agent

General-purpose CFA analyst that decomposes queries into tool call sequences
and synthesizes results into coherent analysis.

- **Query Decomposition**: Breaks complex questions into sub-tasks mapped to specific tools.
- **Tool Selection**: Matches analytical needs to the correct tool from 215 available.
- **Result Synthesis**: Combines numerical outputs with narrative explanation.
- **Citation**: Every number in the output traces back to a specific tool call with inputs.

#### 3.0.3 AgentDB Working Memory

Persistent structured memory via agentic-flow's built-in agentdb.

- **Session State**: Key-value workspace for intermediate calculations and assumptions.
- **Assumption Tracking**: Explicit registry of all assumptions used in an analysis.
- **Dependency Graph**: Tracks which calculations depend on which inputs.
- **Cross-Session Persistence**: State survives across sessions, enabling iterative refinement over time.

#### 3.0.4 CLI and API Access

- **CLI**: Interactive terminal for analysts. Supports streaming output, tool call
  visibility, and session management. Ships as `npx @robotixai/cfa-agent`.
- **API**: RESTful + WebSocket API for integration into existing workflows and dashboards.
- **Output Formats**: Markdown, JSON, and structured report objects.

---

### P1 -- Multi-Agent System (Weeks 7-14)

#### 3.1.1 Specialist Analyst Agents

Six domain-specific agents with tailored system prompts, tool subsets, and frameworks.

| Agent            | Domain Coverage                                  | Key Tools (subset)                              |
|------------------|--------------------------------------------------|-------------------------------------------------|
| Equity Analyst   | DCF, comps, sum-of-parts, earnings quality       | dcf_*, wacc_*, multiples_*, growth_*            |
| Credit Analyst   | Credit metrics, spreads, recovery, covenants     | credit_*, spread_*, recovery_*, covenant_*      |
| Quant/Risk       | VaR, Greeks, factor models, portfolio analytics  | var_*, greeks_*, factor_*, portfolio_*           |
| Macro Analyst    | Rates, FX, yield curves, economic indicators     | rates_*, fx_*, curve_*, macro_*                 |
| ESG Analyst      | ESG scoring, carbon metrics, governance          | esg_*, carbon_*, governance_*                   |
| Private Markets  | LBO, M&A, venture, restructuring, waterfalls     | lbo_*, merger_*, restructuring_*, waterfall_*   |

Each agent has a CFA-level system prompt, curated tool access, and typed output schemas.

#### 3.1.2 CFA Chief Analyst Orchestrator

Meta-agent coordinating specialists via agentic-flow's hierarchical topology.

- **Query Routing**: Classifies incoming queries and delegates to appropriate specialists.
- **Multi-Agent Coordination**: Spawns multiple specialists for cross-domain questions.
- **Conflict Resolution**: Flags tension when specialists disagree and synthesizes balance.
- **Quality Gate**: Reviews all specialist outputs before final delivery.
- **Topology**: Hierarchical (Chief -> Specialists), with peer messaging for
  cross-domain data sharing.

#### 3.1.3 RuVector-Backed Financial Memory

Persistent memory using RuVector's HNSW index with GNN-enhanced retrieval.

- **Analysis Archive**: Every completed analysis is embedded and stored for retrieval.
- **Embedding Model**: all-MiniLM-L6-v2 via local RuVector (no external API calls).
- **Semantic Search**: "Find analyses similar to this credit deterioration pattern."
- **Context Injection**: Relevant past analyses surfaced to agents as context.
- **Entity Memory**: Per-company memory of prior analyses, assumptions, and conclusions.

#### 3.1.4 Streaming Analysis Events

Real-time visibility into the analytical process.

- **Event Types**: `thinking`, `tool_call`, `tool_result`, `agent_delegation`,
  `synthesis`, `final_output`.
- **SSE Stream**: Clients subscribe to analysis events for live progress.
- **Audit Trail**: Complete event log for compliance and reproducibility.

---

### P2 -- Learning System (Weeks 15-24)

#### 3.2.1 SONA Self-Optimizing Patterns

Reinforcement learning loop improving analysis quality over time.

- **Pattern Recording**: Successful tool call sequences recorded as patterns.
- **Reward Signal**: User feedback, output quality scores, calculation consistency checks.
- **Pattern Retrieval**: Agents query the ReasoningBank for high-reward patterns before analysis.
- **Continuous Improvement**: Patterns versioned and scored; low-reward patterns decay.

#### 3.2.2 ReasoningBank for Cross-Session Learning

Persistent reasoning store accumulating institutional knowledge.

- **Reasoning Traces**: Full chain-of-thought for every analysis, stored with embeddings.
- **Cross-Session**: Insights from one analysis available to all future analyses.
- **Category Indexing**: Patterns indexed by analysis_type (equity, credit, M&A, etc.).
- **Feedback Loop**: `train_from_feedback` mechanism for financial analysis quality.

#### 3.2.3 Custom Analysis Templates and Workflows

- **Template Builder**: Multi-step workflows with fixed tool sequences and parameterized inputs.
- **Workflow Library**: Pre-built templates (quarterly earnings review, new position
  analysis, credit watchlist scan).
- **Scheduling**: Run templated analyses on a schedule (e.g., daily risk report).
- **Parameterization**: Templates accept runtime parameters (ticker, date range, scenarios).

#### 3.2.4 Multi-User Workspace with Shared Memory

- **Workspaces**: Isolated environments per team with shared analysis history.
- **Shared Memory**: Team-level RuVector index so all analysts benefit from team knowledge.
- **Permissions**: Role-based access (admin, analyst, viewer).
- **Audit**: Full trail of who ran what analysis and when.

---

## 4. Technical Architecture

### 4.1 Stack Overview

```
+------------------------------------------------------------+
|                      Client Layer                          |
|   CLI (npx)  |  REST API  |  WebSocket  |  MCP Client     |
+------------------------------------------------------------+
|                   Orchestration Layer                       |
|   agentic-flow (TypeScript, Claude Agent SDK)              |
|   - Hierarchical topology (Chief -> Specialists)           |
|   - AgentDB working memory                                 |
|   - Streaming event bus                                    |
+------------------------------------------------------------+
|                    Memory Layer                             |
|   RuVector (HNSW + GNN, local embeddings)                  |
|   - Analysis archive     - Entity memory                   |
|   - ReasoningBank        - SONA patterns                   |
+------------------------------------------------------------+
|                  Calculation Layer                          |
|   corp-finance-mcp (Rust, 128-bit decimal)                 |
|   - 71 domain modules    - 215 MCP tools                   |
|   - 5,879 tests          - SSE/STDIO transport             |
+------------------------------------------------------------+
|                      LLM Layer                             |
|   Claude Opus (primary reasoning)                          |
|   - agentic-flow routing for model selection               |
|   - Tool-use optimized prompts                             |
+------------------------------------------------------------+
```

### 4.2 Key Design Decisions

| Decision                        | Rationale                                                    |
|---------------------------------|--------------------------------------------------------------|
| Rust for calculations           | 128-bit decimal precision, zero-cost abstractions, safety    |
| TypeScript for orchestration    | Claude Agent SDK native, rapid iteration, agentic-flow compat|
| Local embeddings (RuVector)     | No external API calls for embeddings, lower latency, privacy |
| MCP protocol                    | Standard tool protocol, interop with any MCP client          |
| Hierarchical agent topology     | Clear chain of command, quality gating, conflict resolution  |
| AgentDB over chat history       | Structured persistent memory beats unstructured conversation context |

### 4.3 Data Flow: Equity Analysis Example

```
User: "Analyze AAPL with a 3-stage DCF, peer comps, and risk assessment"
  |
  v
Chief Analyst: classifies as multi-domain, delegates to:
  |
  +---> Equity Analyst: DCF model, comparable analysis
  |       - Calls: dcf_3stage, wacc_build, terminal_value, multiples_ev_ebitda
  |       - Writes to agentdb: fair_value, implied_upside, comp_table
  |
  +---> Quant/Risk Analyst: risk decomposition
  |       - Calls: var_parametric, beta_regression, factor_exposure
  |       - Writes to agentdb: risk_metrics, factor_loadings
  |
  v
Chief Analyst: synthesizes specialist outputs
  - Reads agentdb, checks consistency, produces final report with citations
  |
  v
Output: Markdown report + JSON data + audit trail
```

---

## 5. Success Metrics

### 5.1 Calculation Accuracy

| Metric                            | Target       | Measurement                              |
|-----------------------------------|--------------|------------------------------------------|
| Numerical precision               | 128-bit exact| Matches Bloomberg/FactSet terminal output|
| Tool correctness                  | 100%         | All 5,879 tests pass on every release    |
| Formula fidelity                  | CFA-standard | Validated against CFA Institute formulas |

### 5.2 Analysis Quality

| Metric                            | Target       | Measurement                              |
|-----------------------------------|--------------|------------------------------------------|
| Tool selection accuracy           | > 90%        | Agent picks correct tool for the task    |
| Assumption transparency           | 100%         | Every assumption explicitly stated       |
| Citation completeness             | 100%         | Every number traces to a tool call       |
| Analytical coherence              | > 85%        | Cross-check between specialist outputs   |

### 5.3 Performance

| Metric                            | Target       | Measurement                              |
|-----------------------------------|--------------|------------------------------------------|
| Full equity report latency        | < 60s        | End-to-end from query to final output    |
| Single tool invocation            | < 200ms p99  | MCP tool call round-trip                 |
| Streaming first token             | < 2s         | Time to first event after query          |
| Concurrent analyses               | 20+          | Simultaneous multi-agent sessions        |

### 5.4 Learning Effectiveness (P2)

| Metric                            | Target       | Measurement                              |
|-----------------------------------|--------------|------------------------------------------|
| Pattern reuse rate                | > 40%        | Analyses leveraging prior patterns       |
| Quality improvement               | +15%/quarter | SONA reward scores trending upward       |
| Knowledge transfer                | Measurable   | ReasoningBank retrievals per analysis    |

---

## 6. Milestones

| Phase | Milestone                          | Deliverable                                     | Target     |
|-------|------------------------------------|-------------------------------------------------|------------|
| P0.1  | Hosted MCP Server                  | All 215 tools accessible via SSE                | Week 2     |
| P0.2  | Single Agent MVP                   | One CFA agent with agentdb memory, CLI access   | Week 4     |
| P0.3  | API + Polish                       | REST API, output formats, error handling        | Week 6     |
| P1.1  | Specialist Agents                  | 6 domain agents with curated tool access        | Week 9     |
| P1.2  | Chief Analyst Orchestrator         | Hierarchical coordination, conflict resolution  | Week 11    |
| P1.3  | Financial Memory                   | RuVector integration, analysis archive          | Week 13    |
| P1.4  | Streaming + Audit                  | Live events, compliance trail                   | Week 14    |
| P2.1  | SONA Learning                      | Pattern recording, reward signals, retrieval    | Week 18    |
| P2.2  | Templates + Workflows              | Template builder, scheduling, library           | Week 21    |
| P2.3  | Multi-User Workspaces              | Teams, shared memory, permissions               | Week 24    |

---

## 7. Risks and Mitigations

| Risk                                      | Impact | Mitigation                                        |
|-------------------------------------------|--------|---------------------------------------------------|
| LLM hallucination in financial context    | High   | All numbers from tools, not LLM generation        |
| Tool selection errors compound            | High   | Chief Analyst quality gate, consistency checks     |
| Latency budget exceeded with multi-agent  | Medium | Parallel specialist execution, result caching      |
| Embedding drift in financial memory       | Medium | Periodic re-embedding, version-tagged indices      |
| Regulatory/compliance requirements        | Medium | Full audit trails, assumption transparency         |
| MCP protocol evolution                    | Low    | Abstraction layer between agents and transport     |

---

## 8. Out of Scope (v1)

- Real-time market data feeds (agents work with user-provided or cached data).
- Trade execution or order management.
- Regulatory filing generation (10-K, 10-Q authoring).
- Multi-language support (English only for v1).
- On-premise deployment tooling (cloud-first, self-host via container).

---

## 9. Open Questions

1. **Data sourcing**: Market data API integration (Alpha Vantage, Polygon, Bloomberg B-PIPE) deferred past v1.
2. **Model routing**: Route tool-heavy tasks to faster models, reserve Claude Opus for synthesis?
3. **Compliance**: Audit/explainability requirements for regulated personas may need additional controls.
4. **Pricing model**: Per-analysis, per-seat, or usage-based (tool calls + LLM tokens)?

---

*This document is a living artifact. Updated as decisions are made and scope evolves.*
