# Domain-Driven Design: CFA Multi-Agent Analyst System

## System Overview

The CFA Multi-Agent Analyst System is an institutional-grade corporate finance platform built on three pillars:

- **corp-finance-mcp** -- 71 Rust domain modules exposed as 215 MCP tools via a TypeScript server
- **agentic-flow** -- Multi-agent orchestration coordinating specialist CFA analyst agents
- **ruvector** -- Semantic vector database for financial memory and retrieval-augmented analysis

This document defines the bounded contexts, aggregates, entities, value objects, domain events, and the context map governing their relationships.

---

## Context Map

```
+-----------------------------------------------------------------------+
|                                                                       |
|  +-------------------------+       +----------------------------+     |
|  | Analysis Orchestration  |<----->| Specialist Analysts        |     |
|  | (Chief Analyst Agent)   | C/S   | (8 domain-specific agents) |     |
|  +----------+--------------+       +------+---------------------+     |
|             |                             |                           |
|             | Pub/Sub                      | Pub/Sub                   |
|             v                             v                           |
|  +-------------------------+       +----------------------------+     |
|  | Learning & Adaptation   |       | Financial Memory           |     |
|  | (SONA / ReasoningBank)  |<------| (RuVector)                 |     |
|  +-------------------------+ Query +------+---------------------+     |
|                                           |                           |
|             Conformist                    | Conformist                |
|             +-----------------------------+                           |
|             v                                                         |
|  +------------------------------------------------------------------+ |
|  | Hosted MCP Gateway                                                | |
|  | (HTTP/SSE transport -- 215 tools from 71 Rust domain modules)     | |
|  +------------------------------------------------------------------+ |
+-----------------------------------------------------------------------+
```

### Relationship Types

| Upstream               | Downstream             | Relationship       | Description                                                     |
|------------------------|------------------------|--------------------|-----------------------------------------------------------------|
| Analysis Orchestration | Specialist Analysts    | Customer/Supplier  | Chief Analyst creates plans; Specialists fulfill assignments    |
| Specialist Analysts    | Financial Memory       | Publisher/Subscriber| Completed analyses are stored; past analyses are retrieved      |
| Financial Memory       | Learning & Adaptation  | Query/Feed         | Memory provides data for pattern extraction and reward signals  |
| Analysis Orchestration | Learning & Adaptation  | Publisher/Subscriber| Quality feedback flows to SONA for strategy adaptation          |
| Hosted MCP Gateway     | Specialist Analysts    | Conformist         | Analysts conform to the MCP tool schemas; gateway is upstream   |
| Hosted MCP Gateway     | Financial Memory       | Conformist         | Memory context conforms to gateway session and rate policies    |

### Anti-Corruption Layers

- **Specialist Analysts -> MCP Gateway**: Each analyst agent wraps raw MCP tool responses in domain-typed `AnalysisResult` objects, preventing MCP transport concerns from leaking into the analysis domain.
- **Learning & Adaptation -> Financial Memory**: SONA patterns reference memory entries by ID only; they never hold raw embeddings, preserving the learning domain's independence from the storage schema.

---

## Bounded Context 1: Analysis Orchestration

**Responsibility**: The CFA Chief Analyst agent receives research queries, decomposes them into a structured research plan, assigns sub-tasks to specialist agents, and aggregates results into a unified analysis.

### Aggregate: AnalysisRequest

The root aggregate representing a user's research query through its entire lifecycle.

**Entities**:

| Entity             | Description                                                         |
|--------------------|---------------------------------------------------------------------|
| AnalysisRequest    | Root entity. Holds the original query, status, timestamps, and the final aggregated result. Identity: `requestId` (UUID). |
| ResearchPlan       | Ordered decomposition of the request into sub-tasks with dependencies. Created by the Chief Analyst's planning step. |
| AnalystAssignment  | Maps a sub-task to a specific specialist agent, tracking status and the returned result reference. |

**Value Objects**:

| Value Object       | Description                                                         |
|--------------------|---------------------------------------------------------------------|
| QueryIntent        | Parsed intent classification (e.g., valuation, credit assessment, portfolio construction). Immutable. |
| PlanStep           | A single step within the ResearchPlan: description, required tool domains, dependencies on other steps. |
| AggregationStrategy| Strategy enum for combining results: synthesis, comparison, weighted-consensus, majority-vote. |
| ConfidenceScore    | Normalized 0.0--1.0 score representing the Chief Analyst's confidence in the aggregated output. |
| Priority           | Enum: CRITICAL, HIGH, STANDARD, LOW. Determines routing urgency and resource allocation. |

**Domain Events**:

| Event              | Trigger                                           | Payload                                      |
|--------------------|---------------------------------------------------|----------------------------------------------|
| AnalysisRequested  | User submits a new research query                 | requestId, query, priority, timestamp        |
| PlanCreated        | Chief Analyst completes query decomposition       | requestId, planId, steps[], estimatedDuration |
| AnalystAssigned    | A sub-task is routed to a specialist agent        | requestId, assignmentId, agentType, stepRef  |
| ResultAggregated   | All assignments complete; final synthesis ready   | requestId, aggregatedResult, confidence      |
| AnalysisEscalated  | Confidence below threshold; human review needed   | requestId, reason, partialResult             |

**Invariants**:
- A ResearchPlan must have at least one PlanStep before any AnalystAssignment can be created.
- An AnalysisRequest cannot transition to COMPLETED until all AnalystAssignments are RESOLVED or SKIPPED.
- PlanStep dependency cycles are rejected at creation time.

---

## Bounded Context 2: Specialist Analysts

**Responsibility**: Eight domain-specific analyst agents, each with access to a curated subset of the 215 MCP tools. They execute sub-tasks, invoke tools, and return structured analysis results.

### Specialist Agent Roster and Tool Mappings

| Agent               | Domain Modules (from 71 Rust modules)                                                                                    |
|---------------------|--------------------------------------------------------------------------------------------------------------------------|
| Equity Analyst      | equity_research, valuation, earnings_quality, dividend_policy, behavioral, performance_attribution                       |
| Credit Analyst      | credit, credit_scoring, credit_portfolio, credit_derivatives, restructuring, financial_forensics                         |
| Fixed Income        | fixed_income, interest_rate_models, inflation_linked, mortgage_analytics, repo_financing, municipal, sovereign           |
| Derivatives & Vol   | derivatives, volatility_surface, convertibles, structured_products, real_options, monte_carlo                            |
| Quant & Risk        | quant_risk, quant_strategies, portfolio_optimization, risk_budgeting, market_microstructure, index_construction, scenarios|
| Macro Strategist    | macro_economics, fx_commodities, commodity_trading, emerging_markets, trade_finance, carbon_markets                      |
| ESG & Regulatory    | esg, regulatory, compliance, aml_compliance, regulatory_reporting, fatca_crs, substance_requirements, tax_treaty, transfer_pricing |
| Private Markets     | pe, venture, private_credit, private_wealth, infrastructure, real_assets, fund_of_funds, clo_analytics, securitization   |

Cross-cutting modules used by multiple agents: three_statement, fpa, portfolio, treasury, ma, capital_allocation, insurance, pension, wealth, bank_analytics, lease_accounting, onshore_structures, offshore_structures, jurisdiction, crypto.

### Aggregate: AnalystAgent

**Entities**:

| Entity           | Description                                                              |
|------------------|--------------------------------------------------------------------------|
| AnalystAgent     | Root entity. Represents a specialist agent instance. Identity: `agentId`. Holds agent type, active assignment count, and capability manifest. |
| ToolInvocation   | A single call to an MCP tool: tool name, input params, raw output, duration, success/failure. |
| AnalysisResult   | Structured output from one assignment: findings, data tables, confidence, citations to tool invocations. |

**Value Objects**:

| Value Object       | Description                                                          |
|--------------------|----------------------------------------------------------------------|
| ToolName           | Qualified tool identifier matching the MCP registry (e.g., `credit_score_corporate`). |
| InvocationParams   | Immutable map of input parameters passed to an MCP tool.             |
| Finding            | A single analytical conclusion: statement, supporting data, confidence, methodology. |
| Citation           | Reference linking a Finding back to a specific ToolInvocation for auditability. |
| AgentCapability    | Describes which tool domains and analysis types an agent can perform. |

**Domain Events**:

| Event              | Trigger                                          | Payload                                        |
|--------------------|--------------------------------------------------|-------------------------------------------------|
| ToolCalled         | Agent invokes an MCP tool                        | agentId, toolName, params, invocationId         |
| ToolSucceeded      | MCP tool returns a valid result                  | invocationId, duration, resultSummary           |
| ToolFailed         | MCP tool returns an error or times out           | invocationId, errorType, retryable              |
| AnalysisCompleted  | Agent finishes processing its assignment         | agentId, assignmentId, resultId, confidence     |
| InsightGenerated   | Agent identifies a notable finding worth surfacing| agentId, finding, severity                      |

**Invariants**:
- An AnalystAgent must only invoke tools within its declared AgentCapability manifest.
- A ToolInvocation must be recorded before its output can be cited in any Finding.
- An AnalysisResult requires at least one Finding to be considered valid.

---

## Bounded Context 3: Financial Memory

**Responsibility**: RuVector-backed semantic storage providing persistent memory across analysis sessions. Stores past analyses, market data embeddings, and financial report embeddings. Enables retrieval-augmented analysis through vector similarity search.

### Aggregate: AnalysisArchive

**Entities**:

| Entity           | Description                                                              |
|------------------|--------------------------------------------------------------------------|
| AnalysisArchive  | Root entity. Collection-level container grouping related memory entries by topic, sector, or time period. Identity: `archiveId`. |
| MemoryEntry      | An individual stored record: the source text, its embedding vector, metadata (source type, date, entity references), and retrieval statistics. |
| EmbeddingIndex   | Configuration and state of a vector index: model name (e.g., all-MiniLM-L6-v2, gte-small), dimension, distance metric, last rebuild timestamp. |

**Value Objects**:

| Value Object       | Description                                                          |
|--------------------|----------------------------------------------------------------------|
| EmbeddingVector    | Fixed-dimension float array (384d for MiniLM, 384d for gte-small). Immutable once generated. |
| SimilarityScore    | Cosine similarity result 0.0--1.0 from a vector search.             |
| MemoryMetadata     | Tags: source_type (analysis, filing, market_data), sector, date_range, entity_tickers. |
| RetentionPolicy    | TTL and archival rules: hot (30d, fast index), warm (1y, compressed), cold (indefinite, archive). |
| RetrievalContext   | A ranked list of MemoryEntries with scores, returned to a requesting agent. |

**Domain Events**:

| Event              | Trigger                                          | Payload                                        |
|--------------------|--------------------------------------------------|-------------------------------------------------|
| MemoryStored       | A new analysis or data embedding is persisted    | entryId, archiveId, embeddingModel, metadata   |
| MemoryRetrieved    | A vector search returns relevant past analyses   | queryEmbedding, resultCount, topScore          |
| PatternDiscovered  | Clustering or anomaly detection finds a pattern  | patternId, relatedEntryIds, description        |
| IndexRebuilt       | A vector index is rebuilt or rebalanced          | indexId, entryCount, buildDuration             |
| MemoryExpired      | A record exceeds its retention policy TTL        | entryId, retentionTier, age                    |

**Invariants**:
- Every MemoryEntry must have a valid EmbeddingVector before it can be indexed for retrieval.
- An EmbeddingIndex must specify its model and dimension at creation; these are immutable.
- MemoryEntries referenced by active LearningPatterns (BC4) cannot be expired.

---

## Bounded Context 4: Learning & Adaptation

**Responsibility**: Continuous improvement through SONA self-optimizing neural architecture patterns, cross-session reasoning persistence via ReasoningBank, and quality feedback loops that refine agent strategies over time.

### Aggregate: LearningPattern

**Entities**:

| Entity           | Description                                                              |
|------------------|--------------------------------------------------------------------------|
| LearningPattern  | Root entity. A discovered strategy pattern: which tools in which sequence produce high-quality results for a given query type. Identity: `patternId`. |
| ReasoningTrace   | Step-by-step record of an agent's reasoning chain during an analysis, linked to the pattern it followed or created. |
| QualityFeedback  | Human or automated quality assessment of an AnalysisResult, providing the reward signal for SONA reinforcement learning. |

**Value Objects**:

| Value Object       | Description                                                          |
|--------------------|----------------------------------------------------------------------|
| RewardScore        | SONA reward signal: 0.0--1.0 derived from QualityFeedback.          |
| StrategyVector     | Embedding of a reasoning strategy in pattern space for similarity matching. |
| TaskType           | Classification of the task a pattern applies to (e.g., website_discovery, content_scraping, summarization, valuation, credit_assessment). |
| PatternFingerprint | Content hash of a LearningPattern for deduplication.                 |
| AdaptationDelta    | Description of what changed when a strategy was adapted: old vs. new tool sequence, parameter adjustments. |

**Domain Events**:

| Event              | Trigger                                          | Payload                                        |
|--------------------|--------------------------------------------------|-------------------------------------------------|
| PatternLearned     | A new successful strategy is extracted from a reasoning trace | patternId, taskType, rewardScore    |
| StrategyAdapted    | An existing pattern is updated based on new feedback | patternId, adaptationDelta, newReward        |
| ModelUpdated       | SONA model weights are updated from batch feedback | modelVersion, trainingExamples, avgReward     |
| FeedbackReceived   | Quality feedback is submitted for an analysis    | requestId, feedbackId, score, comments         |
| PatternDeprecated  | A pattern's reward drops below viability threshold| patternId, lastReward, replacementId           |

**Invariants**:
- A LearningPattern must have at least one associated ReasoningTrace as evidence.
- QualityFeedback must reference a valid, completed AnalysisRequest from BC1.
- PatternFingerprint must be unique across the ReasoningBank; duplicates are merged rather than created.

---

## Bounded Context 5: Hosted MCP Gateway

**Responsibility**: The remote MCP server that exposes all 215 tools from the 71 Rust domain modules over HTTP/SSE transport. Manages sessions, tool registration, rate limiting, and transport-level concerns.

### Aggregate: McpSession

**Entities**:

| Entity           | Description                                                              |
|------------------|--------------------------------------------------------------------------|
| McpSession       | Root entity. Represents an active connection from an agent to the MCP server. Identity: `sessionId`. Tracks connection state, authenticated identity, and invocation count. |
| ToolRegistry     | Singleton catalog of all 215 registered tools with their schemas, grouped by the 71 domain modules. Serves tool discovery requests. |
| RateLimitPolicy  | Per-session or per-agent rate limiting rules: max RPM, burst allowance, cooldown period. |

**Value Objects**:

| Value Object       | Description                                                          |
|--------------------|----------------------------------------------------------------------|
| TransportType      | Enum: STDIO, HTTP_SSE, STREAMABLE_HTTP. Determines the wire protocol. |
| ToolSchema         | Zod-validated input/output schema for a single MCP tool. Immutable per version. |
| SessionToken       | Opaque authentication token binding a session to an agent identity.  |
| RateLimitWindow    | Sliding window parameters: windowSizeMs, maxRequests, burstSize.     |
| ToolDomain         | Grouping label mapping a tool to its Rust source module (e.g., `credit`, `derivatives`). |

**Domain Events**:

| Event              | Trigger                                          | Payload                                        |
|--------------------|--------------------------------------------------|-------------------------------------------------|
| SessionCreated     | An agent connects and authenticates              | sessionId, agentId, transportType, timestamp   |
| SessionTerminated  | Connection closed or timed out                   | sessionId, reason, totalInvocations            |
| ToolExecuted       | A tool call completes (success or failure)       | sessionId, toolName, duration, status          |
| RateLimitExceeded  | An agent exceeds its rate limit policy           | sessionId, policyId, currentRate, limit        |
| ToolRegistered     | A new tool is added to the registry (deploy-time)| toolName, domain, schemaVersion                |

**Invariants**:
- A ToolInvocation (BC2) can only be executed within an active McpSession.
- RateLimitPolicy must be evaluated before every tool execution; blocked calls are rejected, not queued.
- ToolRegistry entries require a valid ToolSchema; tools without schemas cannot be registered.

---

## Cross-Cutting Concerns

### Ubiquitous Language

| Term                  | Definition                                                                    |
|-----------------------|-------------------------------------------------------------------------------|
| Chief Analyst         | The orchestrating agent that decomposes queries and aggregates results        |
| Specialist Agent      | A domain-focused agent with a curated tool subset                            |
| Tool Invocation       | A single call to one of the 215 MCP tools                                    |
| Finding               | An atomic analytical conclusion supported by data                            |
| Memory Entry          | A vector-embedded record in the RuVector financial memory store              |
| Learning Pattern      | A reusable strategy discovered through SONA reinforcement learning           |
| Reasoning Trace       | A recorded chain of agent reasoning steps                                    |
| Research Plan         | A dependency graph of sub-tasks decomposed from a user query                 |
| Confidence Score      | A 0.0--1.0 measure of analytical certainty                                   |
| Anti-Corruption Layer | Adapter that translates between bounded context languages                    |

### Domain Event Flow (End-to-End)

```
User Query
  |
  v
[AnalysisRequested] --> BC1: Chief Analyst creates ResearchPlan
  |
  v
[PlanCreated] --> BC1: Chief Analyst creates AnalystAssignments
  |
  v
[AnalystAssigned] --> BC2: Specialist receives assignment
  |                        |
  |                        v
  |                   [MemoryRetrieved] <-- BC3: Agent retrieves relevant past analyses
  |                        |
  |                        v
  |                   [SessionCreated] --> BC5: Agent opens MCP session
  |                        |
  |                        v
  |                   [ToolCalled] --> BC5: [ToolExecuted]
  |                        |
  |                        v
  |                   [AnalysisCompleted] --> BC3: [MemoryStored]
  |                        |
  |                        v
  |                   [InsightGenerated] --> BC1: Chief Analyst receives result
  |
  v
[ResultAggregated] --> BC4: [FeedbackReceived] --> [PatternLearned]
```

### Deployment Boundary Alignment

| Bounded Context        | Runtime                     | Data Store                  |
|------------------------|-----------------------------|-----------------------------|
| Analysis Orchestration | agentic-flow coordinator    | SQLite (agentdb.db)         |
| Specialist Analysts    | agentic-flow worker agents  | In-memory (per-session)     |
| Financial Memory       | RuVector service            | PostgreSQL + pgvector       |
| Learning & Adaptation  | SONA service                | ReasoningBank (persistent)  |
| Hosted MCP Gateway     | Node.js MCP server          | ToolRegistry (in-memory)    |
