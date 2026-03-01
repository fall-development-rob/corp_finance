# PRD: Financial Services Workflow Integration

## Overview
Add institutional document workflow capabilities to the CFA agent platform by integrating workflow patterns from Anthropic's financial-services-plugins repository.

## Problem Statement
Our platform excels at financial computation (195 MCP tools with 128-bit precision) but lacks structured workflows for producing the professional deliverables that institutional finance requires -- CIMs, IC memos, coverage initiation reports, earnings analyses, pitch decks, and client reports.

## User Stories
1. As an equity research analyst, I want to produce an initiating coverage report for a company so that I can distribute institutional-quality first-time coverage to clients.
2. As an investment banker, I want to draft a CIM from financial data so that I can market a sell-side mandate.
3. As a PE associate, I want to generate an IC memo with LBO returns and risk analysis so that the investment committee can make an informed decision.
4. As a wealth advisor, I want to build a comprehensive financial plan with Monte Carlo simulations so that I can present retirement projections to clients.
5. As a deal team member, I want to screen inbound deals against our fund criteria so that I can quickly triage the pipeline.
6. As an analyst, I want to check a financial model for errors so that I can ensure accuracy before client delivery.

## Features

### Equity Research Workflows (6 commands)
- `/cfa/initiate-coverage [ticker]` -- 5-task initiating coverage pipeline
- `/cfa/earnings [ticker] [quarter]` -- Post-earnings update report
- `/cfa/morning-note` -- Daily morning research note
- `/cfa/thesis [ticker]` -- Investment thesis tracker
- `/cfa/screen [criteria]` -- Quantitative idea generation
- `/cfa/sector [sector]` -- Sector overview and landscape

### Investment Banking Workflows (4 commands)
- `/cfa/cim [company]` -- Confidential Information Memorandum
- `/cfa/teaser [company]` -- Deal teaser (anonymous/named)
- `/cfa/buyer-list [company]` -- Strategic + financial buyer identification
- `/cfa/pitch-deck [topic]` -- Pitch book structure

### Private Equity Workflows (4 commands)
- `/cfa/screen-deal` -- Inbound deal screening memo
- `/cfa/ic-memo [company]` -- Investment Committee memo
- `/cfa/dd-checklist [company]` -- Due diligence checklist
- `/cfa/value-creation [company]` -- Value creation plan

### Wealth Management Workflows (2 commands)
- `/cfa/financial-plan [client]` -- Comprehensive financial plan
- `/cfa/client-review [client]` -- Client meeting prep

## Success Metrics
- All 16 slash commands functional and routing to correct agents
- HNSW routing correctly directs workflow requests (>0.7 confidence)
- No regression in existing tool/agent functionality
- Contract tests continue to pass

## Out of Scope
- New MCP tools or Rust computation modules
- External data provider integrations (LSEG, S&P Global, FactSet)
- PowerPoint/Excel file generation (output is structured markdown/text)
- Automated multi-task pipeline execution (tasks run one at a time per user request)
