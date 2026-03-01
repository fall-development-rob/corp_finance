# Domain Model: Financial Workflows

## Bounded Context: Workflow Orchestration

This bounded context handles the orchestration of multi-step professional document workflows that compose existing computation and data retrieval capabilities into institutional-grade deliverables.

### Domain Language (Ubiquitous Language)

| Term | Definition |
|------|-----------|
| **Workflow** | A structured multi-step process that produces a professional financial document |
| **Initiating Coverage** | First-time equity research coverage report (30-50 pages) |
| **CIM** | Confidential Information Memorandum -- sell-side marketing document (40-60 pages) |
| **IC Memo** | Investment Committee memorandum -- PE deal approval document (10-15 pages) |
| **Teaser** | Anonymous or named 1-2 page deal summary for initial buyer outreach |
| **Process Letter** | Formal bid process instructions for sell-side M&A |
| **Strip Profile** | Compact financial metrics summary for quick deal comparison |
| **Datapack** | Comprehensive data compilation for buyer due diligence |
| **VCP** | Value Creation Plan -- post-acquisition strategic roadmap |
| **TLH** | Tax-Loss Harvesting -- systematic realization of tax losses |
| **Morning Note** | Daily market intelligence brief for investment teams |
| **Deal Screening** | Quick pass/fail assessment of inbound deal flow |

### Aggregates

#### Equity Research Aggregate
- Root: `workflow-equity-research` skill
- Entities: Initiating Coverage, Earnings Analysis, Earnings Preview, Model Update, Morning Note, Thesis Tracker, Catalyst Calendar, Idea Generation, Sector Overview
- Agent: `cfa-equity-analyst`
- Invariants:
  - Every price target requires at least 2 valuation methods
  - Terminal value must be 50-75% of enterprise value
  - Bull/base/bear scenarios are mandatory for all recommendations

#### Deal Execution Aggregate
- Root: `workflow-investment-banking` + `workflow-private-equity` skills
- Entities: CIM, Teaser, Buyer List, Merger Model, Pitch Deck, Strip Profile, Process Letter, Deal Tracker, Datapack, Deal Screening, IC Memo, DD Checklist, DD Meeting Prep, Returns Analysis, Unit Economics, VCP, Portfolio Monitoring
- Agent: `cfa-private-markets-analyst`
- Invariants:
  - Sources must equal Uses in any S&U table
  - LBO target returns: 20-25% IRR / 2.5-3.0x MOIC
  - IC memo financials must be internally consistent across all sections
  - Altman Z-Score < 1.81 at entry = mandatory red flag

#### Wealth Advisory Aggregate
- Root: `workflow-wealth-management` skill
- Entities: Client Review, Financial Plan, Portfolio Rebalance, Tax-Loss Harvesting, Client Report, Investment Proposal
- Agent: `cfa-quant-risk-analyst`
- Invariants:
  - Monte Carlo simulation requires 1000+ iterations
  - Report median AND 10th/90th percentiles for all projections
  - TLH must check wash sale rules across ALL household accounts
  - Rebalancing must minimize tax impact and transaction costs

#### Quality Assurance Aggregate
- Root: `workflow-financial-analysis` + `workflow-deal-documents` skills
- Entities: Model Audit, Deck Review, Competitive Analysis, Document Standards
- Agent: `cfa-chief-analyst`
- Invariants:
  - Balance sheet must balance (Assets = Liabilities + Equity) every period
  - No circular references unresolved in financial models
  - All numbers traceable to tool output or stated assumption
  - Confidentiality notice on every page of deal documents

### Context Map

```
+-----------------------------------------------------------+
|              Workflow Orchestration Context                |
|                                                           |
|  +--------------+  +--------------+  +--------------+     |
|  |   Equity     |  |    Deal      |  |   Wealth     |     |
|  |  Research    |  |  Execution   |  |  Advisory    |     |
|  |  Aggregate   |  |  Aggregate   |  |  Aggregate   |     |
|  +------+-------+  +------+-------+  +------+-------+     |
|         |                 |                 |              |
|  +------+-----------------+-----------------+--------+     |
|  |            Quality Assurance Aggregate            |     |
|  +---------------------------+-----------------------+     |
+------------------------------+----------------------------+
                               |
               +---------------+---------------+
               |               |               |
               v               v               v
     +---------+---+  +-------+------+  +------+-------+
     | Computation |  | Market Data  |  |   Semantic   |
     |   Context   |  |   Context    |  |   Routing    |
     | (195 tools) |  | (180 FMP)    |  |  (HNSW)      |
     +-------------+  +--------------+  +--------------+
```

### Anti-Corruption Layer
Workflow skills reference MCP tools by their registered names (e.g., `dcf_model`, `lbo_model`). They never import or call Rust functions directly. The MCP server provides the anti-corruption boundary between workflow orchestration and computation.

### Event Flow
1. User invokes slash command (e.g., `/cfa/ic-memo ACME Corp`)
2. HNSW SemanticRouter matches to `cfa-private-markets-analyst`
3. Pipeline injects `workflow-private-equity` skill into agent prompt
4. Agent follows IC Memo workflow steps, calling MCP tools as directed
5. Agent produces structured document output
6. Chief analyst coordinates if multi-agent delegation needed
