# ADR-011: Geopolitical and Alternative Data Integration

## Status: Proposed

## Context

The CFA agent has strong coverage of financial computation (200 MCP tools), market data (FMP), macro indicators (FRED, World Bank), and filings (EDGAR). A gap analysis against institutional analyst workflows identified four significant blind spots:

| Gap | Severity (1-10) | Description |
|-----|-----------------|-------------|
| Geopolitical risk | 8/10 | No conflict, sanctions, or political instability data |
| Alternative data | 9/10 | No prediction markets, sentiment proxies, or displacement signals |
| Commodity supply-side | 5/10 | No energy production/inventory data (EIA), no trade policy (WTO) |
| Climate/environmental events | 5/10 | No disaster alerts, fire detection, or climate anomaly feeds |

These gaps directly affect sovereign risk (ADR: credit/sovereign modules), ESG scoring (ADR: esg module), macro analysis (ADR: macro_economics module), and commodity trading (ADR: commodity_trading module). Analysts must currently exit the agent to manually gather this data, breaking workflow continuity.

The existing World Bank integration (14 tools) covers basic sovereign/EM indicators but omits the World Governance Indicators (WGI) and broader development metrics that underpin country risk models.

## Decision

Create a new `packages/geopolitical-mcp-server/` TypeScript MCP server package with ~30 tools across 4 modules, and expand World Bank coverage in the existing `packages/data-mcp-server/` with ~15 additional tools.

### New Package: packages/geopolitical-mcp-server/

```
packages/geopolitical-mcp-server/
  src/
    index.ts                    -- MCP server entry, tool registration
    conflict/
      acled-client.ts           -- ACLED API client (battles, protests, fatalities)
      ucdp-client.ts            -- UCDP API client (conflict classification)
      gdelt-client.ts           -- GDELT API client (geo-events, tone analysis)
      tools.ts                  -- 8 MCP tools
    environment/
      gdacs-client.ts           -- GDACS disasters
      usgs-client.ts            -- USGS earthquakes
      firms-client.ts           -- NASA FIRMS fire detection
      eonet-client.ts           -- NASA EONET environmental events
      tools.ts                  -- 6 MCP tools
    trade/
      eia-client.ts             -- EIA energy supply data
      wto-client.ts             -- WTO trade policy
      usaspending-client.ts     -- Federal spending
      tools.ts                  -- 8 MCP tools
    alternative/
      polymarket-client.ts      -- Prediction markets
      coingecko-sentiment.ts    -- Fear & Greed, stablecoin peg
      unhcr-client.ts           -- Displacement data
      openmeteo-client.ts       -- Climate anomalies (ERA5)
      tools.ts                  -- 8 MCP tools
    shared/
      circuit-breaker.ts        -- Circuit breaker with cache TTL
      rate-limiter.ts           -- Per-source rate limiting
      types.ts                  -- Shared TypeScript interfaces
```

### Existing Package Extension: packages/data-mcp-server/

Add new World Bank tool modules within the existing `wb/` source directory:

| Module | Tools | Indicators |
|--------|-------|------------|
| `wb-governance.ts` | 6 | Voice & Accountability, Political Stability, Government Effectiveness, Regulatory Quality, Rule of Law, Control of Corruption (WGI) |
| `wb-development.ts` | 9 | Climate (CO2, forest loss, renewable %), poverty (headcount, Gini), health (life expectancy, mortality), education (enrollment), infrastructure (internet, electricity access), trade openness |

Total: ~15 new tools added to the existing World Bank source in `packages/data-mcp-server/`.

### Source Details

| Source | Base URL | Auth | Rate Limit | Cache TTL | Module |
|--------|----------|------|------------|-----------|--------|
| ACLED | api.acleddata.com | Token + email (free) | None stated | 15 min | conflict |
| UCDP | ucdpapi.pcr.uu.se | None | None stated | 1 hour | conflict |
| GDELT | api.gdeltproject.org | None | None stated | 5 min | conflict |
| GDACS | gdacs.org/gdacsapi | None | None stated | 10 min | environment |
| USGS | earthquake.usgs.gov | None | None stated | 5 min | environment |
| NASA FIRMS | firms.modaps.eosdis.nasa.gov | Free API key | None stated | 15 min | environment |
| NASA EONET | eonet.gsfc.nasa.gov | None | None stated | 30 min | environment |
| EIA | api.eia.gov/v2/ | Free API key | None stated | 1 hour | trade |
| WTO | Public API | None | None stated | 1 hour | trade |
| USASpending | api.usaspending.gov | None | None stated | 15 min | trade |
| Polymarket | gamma-api.polymarket.com | None | Unknown | 2 min | alternative |
| CoinGecko | api.coingecko.com | None | 10-30 req/min | 5 min | alternative |
| UNHCR | Population Statistics API | None | None stated | 1 hour | alternative |
| Open-Meteo | Open-Meteo API | None | None stated | 1 hour | alternative |
| World Bank | api.worldbank.org/v2 | None | None stated | 1 hour | data-mcp-server (existing) |

Auth summary: 11 of 14 new sources require no authentication. 3 sources require free API keys (ACLED token, NASA FIRMS key, EIA key). All sources are free-tier or fully open.

### Integration Pattern

Follows the established pattern from ADR-010 (Multi-Source Financial Data Integration):

- Each source has its own `client.ts` with independent cache and rate-limiter (no cross-source interference)
- Circuit breaker wraps all external HTTP calls with configurable failure thresholds
- `GeopoliticalBridge` class for agent pipeline integration (same pattern as `DataBridge`, `PartnerBridge`)
- Bridge exported from `packages/geopolitical-mcp-server/src/index.ts` and registered in the bridge index
- Turbo build orchestration (ADR: monorepo turborepo) handles dependency graph

### Skill Files

4 new skill files for the geopolitical package:

| Skill | File | Agent Targets |
|-------|------|---------------|
| `geopolitical-conflict` | `.claude/skills/geopolitical-conflict/SKILL.md` | chief, macro, credit, equity |
| `geopolitical-environment` | `.claude/skills/geopolitical-environment/SKILL.md` | chief, esg, macro |
| `geopolitical-trade` | `.claude/skills/geopolitical-trade/SKILL.md` | chief, macro, equity, commodity |
| `geopolitical-alternative` | `.claude/skills/geopolitical-alternative/SKILL.md` | chief, macro, quant-risk |

1 updated skill file: `.claude/skills/data-wb/SKILL.md` expanded with governance and development tool documentation.

### Pipeline Routing

`AGENT_SKILLS` updated in `packages/agents/src/pipeline.ts`:

| Agent | Added Skills |
|-------|-------------|
| `cfa-chief-analyst` | +conflict, +environment, +trade, +alternative |
| `cfa-macro-analyst` | +conflict, +trade, +alternative, +wb-governance |
| `cfa-equity-analyst` | +conflict, +trade |
| `cfa-credit-analyst` | +conflict, +wb-governance |
| `cfa-esg-analyst` | +environment, +wb-development |
| `cfa-quant-risk-analyst` | +alternative |

## Consequences

### Positive
- Fills the 4 largest data gaps identified in analyst workflow gap analysis
- Total MCP tool count: 357 (current) + ~30 (geopolitical) + ~15 (World Bank expansion) = ~402 tools
- 11 of 14 new sources are fully public with no authentication required
- All sources are free -- no paid API subscriptions needed
- Follows established monorepo pattern with turbo build orchestration
- Each source independently deployable -- users enable only what they need
- World Bank WGI data directly feeds existing Rust sovereign risk and country risk modules
- Prediction market data (Polymarket) provides forward-looking probability signals unavailable from traditional sources
- Disaster/conflict data enables real-time ESG and supply chain disruption scoring

### Negative
- New package adds a 5th MCP server to manage (was 4 after ADR-010 consolidation)
- 3 new environment variables required: `ACLED_ACCESS_TOKEN`, `NASA_FIRMS_API_KEY`, `EIA_API_KEY`
- GDELT returns large payloads -- needs aggressive response truncation and pagination limits
- Polymarket is an unofficial API with no stability guarantees (similar to Yahoo Finance risk noted in ADR-010)
- CoinGecko free tier has tight rate limits (10-30 req/min) -- may need request queuing under heavy use
- Context window pressure: 4 new skills add ~100-200 lines each to agent prompts

### Risks
- ACLED may restrict free academic access in the future (mitigated: UCDP provides fallback conflict data)
- GDELT tone analysis scores are noisy and require careful interpretation guidance in skill docs
- Open-Meteo ERA5 reanalysis data has a 5-day lag -- not suitable for real-time weather trading signals
- USASpending API has known slow response times (>5s) -- circuit breaker timeout must be generous

## Options Considered

### Option 1: Embed in data-mcp-server (Rejected)
- **Pros**: No new package, simpler dependency graph
- **Cons**: data-mcp-server grows from 75 to 105+ tools, violating single-responsibility; geopolitical data is a distinct domain from financial market data; mixed concerns complicate per-source cache/rate-limit tuning

### Option 2: Multiple small packages, one per source (Rejected)
- **Pros**: Maximum isolation, independent versioning per source
- **Cons**: ADR-010 already consolidated from 13 packages to 4; adding 14 new packages reverses that consolidation decision; turbo build graph becomes unwieldy; bridge index bloat

### Option 3: Rust-native integration (Rejected)
- **Pros**: Consistent with computation modules; type-safe at compile time
- **Cons**: These are HTTP API data clients, not computation modules; TypeScript MCP server is the established pattern for data ingestion (ADR-010); Rust HTTP client boilerplate is heavier than TypeScript fetch for simple REST APIs; no Decimal precision needed for raw data retrieval

## Related Decisions

- ADR-010: Multi-Source Financial Data Integration (establishes the multi-package data integration pattern)
- ADR-008: Financial Services Workflow Integration (workflow skills that consume this data)
- ADR-009: Workflow Rust Auditability (audit hashing pattern for workflow tools)

## References

- [ACLED API Documentation](https://apidocs.acleddata.com/)
- [UCDP API](https://ucdp.uu.se/apidocs/)
- [GDELT Project](https://www.gdeltproject.org/)
- [GDACS API](https://www.gdacs.org/Knowledge/models.aspx)
- [USGS Earthquake API](https://earthquake.usgs.gov/fdsnws/event/1/)
- [NASA FIRMS](https://firms.modaps.eosdis.nasa.gov/api/)
- [NASA EONET](https://eonet.gsfc.nasa.gov/docs/v3)
- [EIA API v2](https://www.eia.gov/opendata/)
- [WTO Data Portal](https://apiportal.wto.org/)
- [USASpending API](https://api.usaspending.gov/)
- [Polymarket Gamma API](https://gamma-api.polymarket.com/)
- [CoinGecko API](https://www.coingecko.com/en/api/documentation)
- [UNHCR Population Statistics](https://www.unhcr.org/refugee-statistics/)
- [Open-Meteo API](https://open-meteo.com/en/docs)
- [World Bank Governance Indicators](https://info.worldbank.org/governance/wgi/)
