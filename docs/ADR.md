# ADR-001: Multi-Agent CFA Analyst System on corp-finance-mcp

| Field       | Value                        |
|-------------|------------------------------|
| **Status**  | Proposed                     |
| **Date**    | 2026-02-12                   |
| **Authors** | Robert Fall                  |
| **Deciders**| Engineering / Architecture   |

---

## Context

The **corp-finance-mcp** project (`@robotixai/corp-finance-mcp` on npm, `corp-finance-core` on
crates.io) provides 215 MCP tools spanning 71 domain modules and 71 CLI subcommands for
institutional-grade corporate finance calculations. The Rust core uses 128-bit decimal precision
(`rust_decimal`) and covers the full CFA curriculum: valuation, credit analysis, fixed income,
derivatives, portfolio optimization, quant risk, ESG, M&A, restructuring, securitization,
regulatory compliance, and 50+ additional specialty domains including volatility surfaces,
interest rate models, mortgage analytics, CLO analytics, carbon markets, and financial forensics.

Today these tools are consumed by a single LLM agent that calls them one at a time through the
Model Context Protocol. This works for isolated calculations but falls short for real-world
analyst workflows that require:

- **Cross-domain synthesis**: An M&A fairness opinion requires DCF valuation, comparable
  company analysis, credit assessment, regulatory review, and ESG scoring -- simultaneously.
- **Iterative refinement**: Equity research involves cycles of hypothesis, calculation, review,
  and revision that benefit from scratchpad memory and structured reasoning loops.
- **Persistent institutional knowledge**: Past analyses, valuation assumptions, and market
  regime observations should accumulate and improve future work.
- **Parallel specialist work**: A portfolio review touching 50 positions across asset classes
  needs concurrent analysis by domain specialists, not sequential single-agent processing.

We need an architecture that decomposes complex financial analysis into coordinated specialist
agents, each with deep domain expertise, that collaborate to produce institutional-quality
research output.

### Prior Art

Three open-source projects inform this design:

1. **virattt/dexter** -- A financial research agent with iterative tool-calling loops,
   scratchpad memory, and streaming events. Proves that structured reasoning cycles
   (observe -> think -> act -> reflect) outperform single-pass generation for financial
   analysis. We adopt this pattern for each specialist agent's internal loop.
2. **ruvnet/agentic-flow** -- Orchestration framework with 66+ agent types, hierarchical/mesh/
   ring/star/adaptive topologies, SONA reinforcement learning, and ReasoningBank for
   cross-session pattern storage.
3. **ruvector** -- Vector database with HNSW indexing, GNN self-learning retrieval, local
   all-MiniLM-L6-v2 embeddings, and Cypher graph queries. Already integrated in this project;
   we extend it to financial analysis memory.

---

## Decision

We will build a **multi-agent CFA analyst system** as an orchestration layer on top of the
existing corp-finance-mcp tool server. The system uses agentic-flow for agent lifecycle and
coordination, RuVector for persistent semantic memory, and the MCP protocol as the exclusive
interface between agents and financial calculations.

### 1. Agent Topology: Hierarchical with Specialist Analysts

```
                    +-------------------------+
                    |   CFA Chief Analyst     |
                    |   (Coordinator Agent)   |
                    +-----+------+------+-----+
                          |      |      |
            +-------------+      |      +--------------+
            |                    |                     |
     +------+------+    +-------+-------+    +--------+-------+
     | Equity      |    | Credit        |    | Quant / Risk   |
     | Analyst     |    | Analyst       |    | Analyst        |
     +-------------+    +---------------+    +----------------+
     | DCF, comps, |    | Spreads,      |    | VaR, Greeks,   |
     | multiples,  |    | ratings,      |    | Monte Carlo,   |
     | earnings    |    | covenants,    |    | vol surfaces,  |
     | quality     |    | default prob  |    | optimization   |
     +-------------+    +---------------+    +----------------+

     +-------------+    +---------------+    +----------------+
     | Macro       |    | ESG / Climate |    | Risk / Reg     |
     | Analyst     |    | Analyst       |    | Analyst        |
     +-------------+    +---------------+    +----------------+
     | Rates, FX,  |    | ESG scores,   |    | Basel, AML,    |
     | sovereign,  |    | carbon mkts,  |    | FATCA/CRS,     |
     | inflation   |    | impact metrics|    | substance      |
     +-------------+    +---------------+    +----------------+
```

**CFA Chief Analyst** (coordinator):
- Receives high-level research requests (e.g., "Produce a buy-side analysis of Company X")
- Decomposes requests into specialist tasks using agentic-flow's hierarchical topology
- Aggregates specialist outputs through attention-based consensus
- Resolves contradictions (e.g., strong equity valuation vs. deteriorating credit metrics)
- Produces the final synthesized research output

**Specialist Agents** (6 core analysts, extensible):
- **Equity Analyst**: Calls valuation, equity_research, earnings_quality, dividend_policy,
  pe, and comparable analysis tools
- **Credit Analyst**: Calls credit, credit_scoring, credit_portfolio, credit_derivatives,
  private_credit, and restructuring tools
- **Quant/Risk Analyst**: Calls quant_risk, derivatives, volatility_surface, monte_carlo,
  portfolio_optimization, risk_budgeting, and quant_strategies tools
- **Macro Analyst**: Calls macro_economics, sovereign, fx_commodities, interest_rate_models,
  inflation_linked, and emerging_markets tools
- **ESG/Climate Analyst**: Calls esg, carbon_markets, infrastructure, and behavioral tools
- **Regulatory/Compliance Analyst**: Calls regulatory, compliance, aml_compliance, fatca_crs,
  regulatory_reporting, substance_requirements, and tax_treaty tools

Each specialist runs a Dexter-style iterative loop internally:
1. **Observe**: Receive task + relevant context from coordinator
2. **Think**: Plan which MCP tools to call, review scratchpad from prior iterations
3. **Act**: Call corp-finance-mcp tools via MCP protocol, capture structured results
4. **Reflect**: Evaluate results, decide whether to iterate or finalize
5. **Report**: Return structured findings to coordinator

### 2. Tool Layer: MCP Protocol as the Single Calculation Interface

All 215 MCP tools remain the exclusive computational layer. Agents NEVER perform financial
calculations directly -- they formulate tool calls and interpret results. This guarantees:

- **128-bit decimal precision** on every calculation (Rust `rust_decimal` backend)
- **Auditability**: Every number in every report traces to a specific tool invocation with
  logged inputs and outputs
- **Determinism**: Same inputs always produce the same outputs, unlike LLM-generated arithmetic
- **Separation of concerns**: The Rust core is tested independently; agents focus on reasoning

The MCP server runs as a hosted remote service (SSE/streamable-HTTP transport) so that all
agents in the swarm connect to a single server instance rather than each spawning a local
stdio process. This enables:

- Centralized logging and audit trails for regulatory compliance
- Rate limiting and access control per agent
- Horizontal scaling of the tool server independently of the agent swarm
- Remote deployment -- agents and tools can run in different environments

### 3. Memory: RuVector for Semantic Financial Knowledge

RuVector serves as the persistent memory layer with three collections:

**Analysis Archive**: Past research reports, valuations, and recommendations stored as
embeddings generated by the local all-MiniLM-L6-v2 model. Agents query this to find
comparable prior analyses ("show me how we valued similar SaaS companies at 15x revenue").

**Market Context**: Financial data snapshots, earnings transcripts, and macro regime
observations. GNN-enhanced retrieval learns which data points were most useful for which
analysis types, improving relevance ranking over time.

**Methodology Patterns**: Successful analysis workflows stored as graph structures via
Cypher queries. For example, a graph linking "leveraged buyout analysis" to the specific
sequence of tool calls, assumptions, and sensitivity ranges that produced high-quality
outputs in past sessions.

Key RuVector capabilities leveraged:
- **HNSW indexing**: Fast ANN search across millions of stored financial data points
- **Local embeddings**: all-MiniLM-L6-v2 runs locally -- no external API calls, critical for
  confidential financial data
- **GNN self-learning**: Learns which stored analyses are most relevant for new queries
- **Cypher graph queries**: Structured traversal of relationships between analyses, companies,
  sectors, and methodologies

### 4. Learning: SONA + ReasoningBank

**SONA (Self-Organizing Neural Architecture)** provides reinforcement learning at the swarm
level. After each completed analysis:

- The coordinator scores output quality (automated metrics + optional human feedback)
- SONA adjusts agent selection, tool-calling patterns, and coordination strategies
- Over time, the system learns which specialist combinations and tool sequences produce
  the best results for different analysis types

**ReasoningBank** stores successful reasoning chains as reusable patterns:

- When an equity analyst discovers that combining earnings_quality + dividend_policy tools
  with a specific sequence of valuation tools produces superior fundamental analysis, that
  pattern is stored and can be retrieved by future agent instances
- Patterns are indexed by analysis type, sector, asset class, and market regime
- The `search_learning_patterns` API allows agents to consult proven strategies before
  beginning new analyses
- The `train_from_feedback` API records quality scores that improve pattern rankings

### 5. Hosted MCP Deployment

The corp-finance-mcp server will be deployed as a hosted MCP service:

- **Transport**: SSE (Server-Sent Events) and streamable-HTTP, replacing local stdio for
  multi-agent access
- **Authentication**: OAuth 2.1 / API key per agent identity
- **Scaling**: Stateless tool handlers behind a load balancer; the Rust/NAPI computation
  layer scales horizontally
- **Monitoring**: Structured logging of every tool call with inputs, outputs, latency, and
  calling agent identity -- critical for financial audit requirements

### 6. Agent Orchestration via agentic-flow

agentic-flow manages the full agent lifecycle:

- **Spawning**: Coordinator dynamically spawns specialists based on the request; a simple
  equity valuation needs only the Equity Analyst, while a full acquisition spawns all six
  plus ad-hoc agents for deal-specific domains (e.g., real_assets for a REIT target)
- **Topology**: Default is hierarchical (coordinator -> specialists) but the system can switch
  to mesh topology for peer-review scenarios where specialists critique each other's work
- **Consensus**: Attention-based consensus resolves conflicting outputs; e.g., if Credit flags
  covenant breach risk while Equity projects strong upside, the coordinator weighs both with
  learned attention weights
- **Lifecycle**: Agents are ephemeral by default (spawned per request, terminated after) but
  can be made persistent for ongoing coverage of specific companies or portfolios
- **Streaming**: All agent reasoning steps stream as events, enabling real-time monitoring
  dashboards and human-in-the-loop intervention

---

## Consequences

### Positive

- **Institutional-quality output**: Specialist agents with domain-specific tool access produce
  deeper analysis than a generalist single agent attempting to use all 215 tools
- **Parallel execution**: A full-spectrum company analysis that would take 15-20 sequential
  tool calls completes faster with 6 specialists working concurrently
- **Improving over time**: SONA learning and ReasoningBank mean the system gets better at
  financial analysis with every completed request
- **Audit trail**: MCP protocol + centralized hosted server provides complete provenance for
  every calculation in every report
- **Data sovereignty**: Local RuVector embeddings and on-premise deployment options mean
  confidential financial data never leaves controlled infrastructure
- **Extensibility**: Adding new specialist agents (e.g., a Structured Products Analyst) requires
  only defining which subset of the 215 tools it accesses and what its reasoning loop looks like

### Negative

- **Operational complexity**: Running a multi-agent swarm with vector database, learning
  infrastructure, and hosted MCP server is significantly more complex than a single-agent setup
- **Latency overhead**: Agent coordination, consensus, and memory retrieval add latency on top
  of raw tool-call time; simple calculations that need one tool call will be slower
- **Cost**: Multiple concurrent LLM agents multiply token costs; a 6-specialist analysis uses
  roughly 6x the tokens of a single-agent approach (partially offset by more focused prompts)
- **Debugging difficulty**: Distributed multi-agent failures are harder to diagnose than
  single-agent errors; requires investment in observability tooling
- **Learning cold start**: SONA and ReasoningBank provide no benefit until sufficient analysis
  history accumulates; initial outputs may not exceed single-agent quality

### Neutral

- The existing corp-finance-mcp server and Rust core require no modifications; they remain a
  stable, independently versioned calculation layer
- The multi-agent system is an additive layer; the single-agent MCP usage path remains
  available for simple queries and backward compatibility

---

## Alternatives Considered

### A. Enhanced Single Agent with All 215 Tools

Keep one agent with access to all tools, improve its reasoning with chain-of-thought prompting
and structured output formats.

**Rejected because**: Context window limitations make it impractical for one agent to maintain
expertise across 71 domains. Tool selection degrades as tool count grows. No path to parallel
execution.

### B. Static Pipeline (No Agent Orchestration)

Define fixed analysis pipelines (e.g., "equity valuation pipeline" = specific sequence of
tool calls) without LLM-based agents.

**Rejected because**: Financial analysis is inherently adaptive -- discovering negative free
cash flow pivots from DCF to distressed valuation. Static pipelines cannot handle this
branching logic without encoding every possible path.

### C. virattt/dexter Fork (Single-Agent with Scratchpad)

Fork Dexter directly, replacing its tools with corp-finance-mcp tools. Single agent with
iterative reasoning loops and scratchpad memory.

**Rejected because**: While Dexter's iterative loop is excellent (we adopt it within each
specialist), a single agent cannot parallelize cross-domain work. We take the pattern but
not the single-agent constraint.

### D. LangGraph / CrewAI for Orchestration

Use an established Python-based multi-agent framework instead of agentic-flow.

**Rejected because**: agentic-flow provides SONA learning and ReasoningBank out of the box,
which would need to be built from scratch on LangGraph/CrewAI. Its topology flexibility is
more mature, RuVector integration is native, and staying within the Rust/TypeScript ecosystem
avoids introducing Python as a third runtime.

### E. Direct LLM Arithmetic (No MCP Tool Layer)

Let agents compute financial metrics directly via LLM generation instead of calling tools.

**Rejected because**: LLMs cannot reliably perform financial arithmetic. A 1 basis point
discount rate error cascades into material valuation errors. The entire premise of
corp-finance-core is deterministic, precision-guaranteed computation -- fundamentally
incompatible with LLM-generated arithmetic.
