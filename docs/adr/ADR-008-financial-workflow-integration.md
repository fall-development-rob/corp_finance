# ADR-008: Financial Services Workflow Integration

## Status
Accepted

## Date
2026-03-01

## Context
The Anthropic financial-services-plugins repository (https://github.com/anthropics/financial-services-plugins) provides 41 institutional workflow skills across equity research, investment banking, private equity, and wealth management. These define professional document generation pipelines (CIM drafting, IC memos, initiating coverage reports, earnings analyses) that guide agents through multi-step institutional processes.

Our CFA agent platform has the computational backbone (195 MCP tools, 67 Rust modules, 5,417 tests) but lacks workflow orchestration for producing institutional deliverables. The Anthropic plugins provide complementary workflow definitions without computation -- they guide WHAT to produce and HOW to structure it, while our tools provide the calculations.

## Decision
Integrate Anthropic's workflow patterns as 6 consolidated workflow skills mapped to existing agents, rather than importing all 41 skills individually. This avoids context window bloat while providing full coverage.

### Consolidation Strategy
| New Skill | Consolidates | Agent |
|-----------|-------------|-------|
| `workflow-equity-research` | 9 ER skills | `cfa-equity-analyst` |
| `workflow-investment-banking` | 9 IB skills | `cfa-private-markets-analyst` |
| `workflow-private-equity` | 9 PE skills | `cfa-private-markets-analyst` |
| `workflow-wealth-management` | 6 WM skills | `cfa-quant-risk-analyst` |
| `workflow-financial-analysis` | 4 FA QC skills | `cfa-chief-analyst` |
| `workflow-deal-documents` | Cross-cutting standards | `cfa-chief-analyst` |

### What We Skip
- LSEG partner plugin (8 skills) -- proprietary data source; FMP provides equivalent data
- S&P Global partner plugin (3 skills) -- same reason
- skill-creator meta-skill -- not applicable
- ppt-template-creator -- overly specific to PowerPoint XML
- MCP connector configs (.mcp.json) -- we use FMP, not Daloopa/FactSet/Morningstar

### Integration Points
- Skills: `.claude/skills/workflow-*/SKILL.md` (6 new files)
- Commands: `.claude/commands/cfa/*.md` (16 new slash commands)
- Pipeline: `packages/agents/src/pipeline.ts` AGENT_SKILLS + CFA_INTENTS
- Agents: 4 agent definitions updated with new capabilities

## Consequences
### Positive
- Agents can produce institutional-grade deliverables (CIMs, IC memos, coverage reports, pitch decks)
- Professional workflow standards (formatting, quality checklists, document structure) codified
- 16 new slash commands provide direct access to specific workflows
- No new MCP tools or Rust code needed -- pure skill layer addition
- HNSW routing updated to direct workflow requests to correct agents

### Negative
- Context window cost: each workflow skill adds ~150-250 lines to agent prompts
- Private markets analyst now carries 2 workflow skills (IB + PE) in addition to existing skills
- Wealth management mapping to quant-risk agent may feel unintuitive to some users

### Risks
- Context window pressure on private-markets-analyst (mitigated: total ~500 lines, comparable to regulatory analyst's single 944-line skill)
- DRIFT-001 contract: new skills are NOT MCP tools/modules -- no drift-check impact
