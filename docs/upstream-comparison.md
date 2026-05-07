# CFA-agent vs anthropics/financial-services

Side-by-side functionality comparison after Phases 26 → 29 Wave 2.

## Engineering capabilities

| Capability area | This repo | anthropics/financial-services |
|---|---|---|
| Deterministic computation core | Yes (Rust, decimal precision) | No (LLM prompts) |
| Specialist analyst agents | Yes | Yes |
| Skills + slash commands + cookbooks | Yes | Yes |
| CLI / MCP / plugin / skill surfaces | All four | Skill + plugin only |
| Free public data MCPs | Yes | No |
| Paid vendor MCPs (free/paid tier split) | Yes | Partial (LSEG + S&P) |
| Memory & retrieval (HNSW + BM25 + entity graph) | Yes | No |
| Self-learning (trajectory + replay + drift + golden sets) | Yes | No |
| Multi-agent orchestration | Yes | No |
| Audit trail (hashes + per-output companions) | Yes | No |
| Multi-tenancy & federation | Yes | No |
| Cost ledger & budget enforcement | Yes | No |
| Observability (tracing) | Yes | No |
| PII / prompt-injection security | Yes | No |
| Compliance reporting | Yes | Partial (KYC) |
| Specflow contracts + ADRs / DDDs / PRDs | Yes | No |
| MS Office / M365 integration | Planned | Yes |
| Closed-source vendor MCPs (Chronograph / Egnyte) | Planned | Yes |

## Domain coverage

| Domain | Techniques | This repo | upstream |
|---|---|---|---|
| Equity research | DCF, trading comps, DDM, earnings quality, financial forensics, three-statement modelling | Yes (deterministic) | Yes (LLM) |
| Credit analysis | Synthetic ratings, debt capacity, covenants, Altman Z, CDS, CVA, credit scoring, distress | Yes | Partial |
| Fixed income | Bond pricing, yield-curve construction, duration / convexity, TIPS, repo, MBS, munis, sovereigns, interest-rate models | Yes | No |
| Derivatives | Option pricing, IV, SABR vol surface, forwards / futures, swaps, structured products, convertibles, real options, Monte Carlo | Yes | No |
| Quant & risk | Factor models, Black-Litterman, risk parity, VaR / CVaR, stress testing, portfolio optimisation, perf attribution, microstructure, capital allocation | Yes | No |
| Macro / FX / commodities | FX forwards, cross rates, commodity term structure, EM, monetary policy, sovereign risk, trade finance, inflation-linked | Yes | No |
| Private markets | LBO, sources & uses, debt schedules, waterfalls (MOIC / IRR), M&A merger model, VC, fund-of-funds, CLOs, securitisation, infrastructure, institutional real estate | Yes | Yes (LLM) |
| Investment banking workflows | Pitch decks, process letters, deal trackers, one-pagers, merger models, datapacks | Yes | Yes |
| Wealth / FPA | Wealth planning, behavioural finance, dividend policy, private wealth, FP&A, rebalancing, TLH | Yes | Yes (LLM) |
| ESG & regulatory | ESG scoring, carbon markets, Basel III, MiFID II / GIPS, AIFMD, Form PF, AML / KYC, FATCA / CRS, transfer pricing, tax treaty, onshore / offshore structures | Yes | Partial (KYC only) |
| Insurance & pension | Insurance liabilities, pension accounting, lease accounting | Yes | No |
| Treasury & banking | Treasury management, bank analytics, repo financing | Yes | No |
| Restructuring / distress | Debt restructuring, bankruptcy modelling | Yes | No |
| Crypto & digital assets | Crypto asset modelling | Yes | No |
