# PRD: Geopolitical, Environmental & Alternative Data Integration

## Overview
Add geopolitical intelligence, environmental event monitoring, trade/supply-chain fundamentals, alternative sentiment data, and expanded World Bank indicators to the CFA agent platform via a new `packages/geopolitical-mcp-server/` MCP server and extended `packages/data-mcp-server/` World Bank tools.

## Problem Statement
Our platform has 357 MCP tools covering financial computation, market data, and institutional workflows, but the inputs to several critical Rust modules -- sovereign risk scoring, scenario probability weighting, catastrophe reserving, ESG climate/social scoring, macro trade analysis, commodity fundamentals, and behavioral sentiment -- require manual data entry. Real-world geopolitical events, natural disasters, trade policy shifts, and alternative sentiment signals are not programmatically available to the agent. The existing 14 World Bank tools cover basic macro indicators (GDP, population, inflation) but miss governance, climate, poverty, health, education, infrastructure, and trade datasets that feed directly into country risk and ESG models.

## User Stories

1. As a sovereign credit analyst, I want real-time conflict data (ACLED/UCDP) for a country so that my `sovereign/country_risk.rs` political stability and rule-of-law inputs are evidence-based rather than manually estimated.
2. As a scenario analyst, I want prediction market odds from Polymarket for geopolitical events so that I can calibrate probability weights in `scenarios/sensitivity.rs` against market-implied expectations.
3. As a commodity trading analyst, I want EIA oil production and inventory data so that I can feed supply fundamentals into `commodity_trading/storage.rs` instead of relying on stale snapshots.
4. As an insurance analyst, I want GDACS disaster alerts and USGS earthquake feeds so that I can parameterize catastrophe models in `insurance/reserving.rs` with current event data.
5. As an ESG analyst, I want NASA FIRMS wildfire satellite data and World Bank climate indicators (CO2 emissions, renewable energy share) so that I can score physical climate risk in `esg/climate.rs` with observed data.
6. As an ESG analyst, I want UNHCR displacement statistics and World Bank governance indicators (WGI) so that the social and governance pillars in `esg/scoring.rs` reflect current conditions.
7. As a macro economist, I want WTO trade restriction data and World Bank trade/FDI indicators so that `macro_economics/international.rs` trade policy analysis uses structured inputs rather than manual assumptions.
8. As a behavioral finance analyst, I want GDELT tone/sentiment data and CoinGecko Fear & Greed index so that `behavioral/sentiment.rs` can incorporate real-time alternative sentiment signals.
9. As a PE associate screening emerging market deals, I want World Bank poverty, health, education, and infrastructure indicators so that I can assess market development and demographic risk alongside financial metrics.
10. As a portfolio manager, I want NASA EONET environmental event data cross-referenced with portfolio holdings' geographic exposure so that I can identify concentration risk from natural disasters.

## Features

### Tier 1 -- Conflict & Geopolitical Intelligence (4 sources, ~12 tools)

| Source | API Base | Auth | Tools |
|--------|----------|------|-------|
| ACLED | `api.acleddata.com` | Free token (env `ACLED_API_KEY` + `ACLED_EMAIL`) | `acled_events`, `acled_fatalities`, `acled_country_summary` |
| UCDP | `ucdpapi.pcr.uu.se` | None | `ucdp_conflicts`, `ucdp_battle_deaths`, `ucdp_country_profile` |
| GDELT | `api.gdeltproject.org` | None | `gdelt_events`, `gdelt_tone`, `gdelt_country_tension` |
| Polymarket | `gamma-api.polymarket.com` | None | `polymarket_events`, `polymarket_odds`, `polymarket_geopolitical` |

#### Tool Specifications

- **acled_events** -- Query armed conflict events by country, date range, event type (battles, protests, riots, violence against civilians, explosions/remote violence, strategic developments). Returns location, actors, fatalities, notes.
- **acled_fatalities** -- Aggregate fatality counts by country, time period, and event type. Supports rolling windows for trend analysis.
- **acled_country_summary** -- Composite conflict profile: event counts by type, fatality trends, top actors, geographic hotspots. Feeds `sovereign/country_risk.rs` political stability score.
- **ucdp_conflicts** -- Active and historical armed conflicts by country with UCDP classification (state-based, non-state, one-sided). Returns intensity level, best/low/high death estimates.
- **ucdp_battle_deaths** -- Time series of battle-related deaths with confidence intervals. Supports comparison across conflicts.
- **ucdp_country_profile** -- Aggregated conflict history for a country: years active, conflict types, cumulative casualties. Direct input to rule-of-law scoring.
- **gdelt_events** -- Query GDELT Global Knowledge Graph events by country, date, and CAMEO event codes. Returns tone, Goldstein scale, mention counts.
- **gdelt_tone** -- Average tone analysis for a country or entity over time. Negative tone trends correlate with political instability.
- **gdelt_country_tension** -- Bilateral tension index between two countries based on GDELT event tone and volume. Feeds scenario modeling.
- **polymarket_events** -- Search active prediction markets by keyword. Returns market question, current odds, volume, liquidity.
- **polymarket_odds** -- Time series of prediction market odds for a specific event. Useful for implied probability curves.
- **polymarket_geopolitical** -- Filtered view of geopolitical prediction markets (elections, conflicts, sanctions, trade). Pre-curated for financial relevance.

### Tier 2 -- Environmental Events (4 sources, ~10 tools)

| Source | API Base | Auth | Tools |
|--------|----------|------|-------|
| GDACS | `gdacs.org/gdacsapi` | None | `gdacs_alerts`, `gdacs_events`, `gdacs_country_exposure` |
| USGS | `earthquake.usgs.gov` | None | `usgs_earthquakes`, `usgs_significant` |
| NASA FIRMS | `firms.modaps.eosdis.nasa.gov` | Free API key (env `NASA_FIRMS_KEY`) | `firms_fires`, `firms_country_fires` |
| NASA EONET | `eonet.gsfc.nasa.gov` | None | `eonet_events`, `eonet_categories` |

#### Tool Specifications

- **gdacs_alerts** -- Current global disaster alerts with alert level (Green/Orange/Red), type (earthquake, flood, cyclone, volcano, drought), severity, and affected population estimate.
- **gdacs_events** -- Historical disaster events by type, country, date range. Returns coordinates, magnitude/intensity, alert score, episode duration.
- **gdacs_country_exposure** -- Aggregate disaster exposure for a country: event frequency by type, maximum severity, population affected. Direct input to `insurance/reserving.rs` catastrophe parameters.
- **usgs_earthquakes** -- Query earthquakes by magnitude threshold (default 4.5+), date range, and bounding box. Returns magnitude, depth, location, tsunami flag.
- **usgs_significant** -- List significant recent earthquakes (USGS curated). Includes PAGER alert level (economic/fatality impact estimate).
- **firms_fires** -- Active fire detections from VIIRS satellite within a bounding box or country. Returns confidence, brightness temperature, fire radiative power (FRP). Feeds `esg/climate.rs` physical risk.
- **firms_country_fires** -- Aggregate fire detection statistics by country and date range. Trend analysis for deforestation and wildfire seasons.
- **eonet_events** -- NASA Earth Observatory natural events: wildfires, severe storms, sea/lake ice, volcanoes, floods. Returns event geometry and source references.
- **eonet_categories** -- List EONET event categories with descriptions and recent event counts.

### Tier 3 -- Trade & Supply Chain (3 sources, ~8 tools)

| Source | API Base | Auth | Tools |
|--------|----------|------|-------|
| EIA | `api.eia.gov/v2/` | Free API key (env `EIA_API_KEY`) | `eia_petroleum`, `eia_electricity`, `eia_capacity` |
| WTO | `apiportal.wto.org` | None | `wto_tariffs`, `wto_barriers`, `wto_trade_stats` |
| USASpending | `api.usaspending.gov` | None | `usaspending_contracts`, `usaspending_agencies` |

#### Tool Specifications

- **eia_petroleum** -- U.S. and global petroleum data: crude production, refinery throughput, inventory levels (SPR, commercial), imports/exports. Weekly and monthly series. Primary input to `commodity_trading/storage.rs`.
- **eia_electricity** -- Electricity generation by source, capacity factors, consumption by sector. Supports energy transition analysis for `esg/climate.rs`.
- **eia_capacity** -- Power plant capacity by fuel type, additions/retirements, planned builds. Feeds infrastructure and energy models.
- **wto_tariffs** -- Applied and bound tariff rates by HS code, reporter, and partner country. Supports trade policy impact analysis in `macro_economics/international.rs`.
- **wto_barriers** -- SPS (Sanitary and Phytosanitary) and TBT (Technical Barriers to Trade) notifications. Signals emerging trade friction.
- **wto_trade_stats** -- Bilateral merchandise trade flows by product group and country pair. Feeds trade openness and dependency analysis.
- **usaspending_contracts** -- Federal government contract awards by agency, recipient, NAICS code, and amount. Supports defense/infrastructure sector analysis.
- **usaspending_agencies** -- Agency-level spending summaries by fiscal year. Useful for government expenditure trend analysis.

### Tier 4 -- Alternative Data (3 sources, ~5 tools)

| Source | API Base | Auth | Tools |
|--------|----------|------|-------|
| CoinGecko | `api.coingecko.com/api/v3/` | None | `coingecko_fear_greed`, `coingecko_stablecoin_health` |
| UNHCR | `api.unhcr.org` | None (CC BY 4.0) | `unhcr_displacement`, `unhcr_country` |
| Open-Meteo | `archive-api.open-meteo.com` | None | `openmeteo_climate_anomaly` |

#### Tool Specifications

- **coingecko_fear_greed** -- Crypto Fear & Greed Index with historical time series. Alternative sentiment indicator for `behavioral/sentiment.rs`.
- **coingecko_stablecoin_health** -- Stablecoin market cap, dominance, and peg deviation monitoring. Early warning for crypto-correlated risk events.
- **unhcr_displacement** -- Global displacement statistics: refugees, asylum seekers, IDPs, stateless persons by country of origin and asylum. Direct input to `esg/scoring.rs` social pillar.
- **unhcr_country** -- Country-level displacement profile: hosted population, origin population, trend analysis. Feeds sovereign social risk scoring.
- **openmeteo_climate_anomaly** -- ERA5 reanalysis temperature and precipitation anomalies for a location/region. Supports physical climate risk assessment in `esg/climate.rs`.

### Tier 5 -- World Bank Expansion (extend existing data-mcp-server, ~15 tools)

These tools extend the existing `packages/data-mcp-server/src/wb/` module, following the established `wbFetch` client pattern.

| Category | Indicator Codes | Tools |
|----------|----------------|-------|
| Governance (WGI) | CC.EST, GE.EST, PV.EST, RQ.EST, RL.EST, VA.EST | `wb_governance`, `wb_governance_compare` |
| Climate | EN.ATM.CO2E.PC, AG.LND.FRST.ZS, EG.FEC.RNEW.ZS | `wb_climate`, `wb_climate_vulnerability` |
| Poverty | SI.POV.DDAY, SI.POV.GINI, SI.DST.FRST.20 | `wb_poverty`, `wb_inequality` |
| Health | SP.DYN.LE00.IN, SH.DYN.MORT, SH.XPD.CHEX.GD.ZS | `wb_health` |
| Education | SE.PRM.ENRR, SE.ADT.LITR.ZS, GB.XPD.RSDV.GD.ZS | `wb_education` |
| Infrastructure | EG.ELC.ACCS.ZS, IT.NET.USER.ZS, LP.LPI.OVRL.XQ | `wb_infrastructure`, `wb_logistics` |
| Trade | TG.VAL.TOTL.GD.ZS, BX.KLT.DINV.WD.GD.ZS, NE.TRD.GNFS.ZS | `wb_trade`, `wb_fdi` |

#### Tool Specifications

- **wb_governance** -- Worldwide Governance Indicators (WGI) for a country: Voice & Accountability, Political Stability, Government Effectiveness, Regulatory Quality, Rule of Law, Control of Corruption. Returns percentile rank and estimate. Direct input to `sovereign/country_risk.rs`.
- **wb_governance_compare** -- Compare WGI scores across multiple countries for a single governance dimension. Supports peer group analysis for sovereign rating models.
- **wb_climate** -- Climate indicators for a country: CO2 emissions per capita, forest area percentage, renewable energy share, methane emissions. Time series with trend calculation. Feeds `esg/climate.rs`.
- **wb_climate_vulnerability** -- Composite climate vulnerability view: CO2 trajectory, deforestation rate, renewable transition progress, agricultural land change. Multi-indicator dashboard for physical risk.
- **wb_poverty** -- Poverty indicators: headcount ratio at $2.15/day, Gini coefficient, income share of bottom 20%. Feeds sovereign social risk and ESG scoring.
- **wb_inequality** -- Extended inequality metrics: income distribution quintiles, shared prosperity premium, poverty gap index. Supports social stability modeling.
- **wb_health** -- Health development indicators: life expectancy, under-5 mortality, health expenditure as % of GDP, immunization rates. Feeds ESG social pillar and demographic risk.
- **wb_education** -- Education indicators: primary/secondary enrollment, adult literacy rate, R&D expenditure as % of GDP. Human capital assessment for sovereign and ESG models.
- **wb_infrastructure** -- Infrastructure development: electricity access %, internet users %, mobile subscriptions per 100 people. Market development scoring for EM analysis.
- **wb_logistics** -- Logistics Performance Index (LPI) overall and sub-components (customs, infrastructure, shipments, logistics competence, tracking, timeliness). Trade friction indicator for `macro_economics/international.rs`.
- **wb_trade** -- Trade indicators: merchandise trade as % of GDP, trade openness, current account balance. Supports trade dependency and vulnerability analysis.
- **wb_fdi** -- Foreign direct investment: net inflows/outflows as % of GDP, FDI stock, greenfield investment. Cross-border capital flow analysis.

## Architecture

### New Package: `packages/geopolitical-mcp-server/`

```
packages/geopolitical-mcp-server/
  src/
    index.ts              # Server entry point, tool registration
    acled/
      client.ts           # ACLED API client (token auth, cache, circuit breaker)
      schemas/common.ts   # Zod schemas
      tools/events.ts     # acled_events, acled_fatalities, acled_country_summary
    ucdp/
      client.ts           # UCDP API client (no auth, cache)
      schemas/common.ts
      tools/conflicts.ts  # ucdp_conflicts, ucdp_battle_deaths, ucdp_country_profile
    gdelt/
      client.ts           # GDELT API client (no auth, cache)
      schemas/common.ts
      tools/events.ts     # gdelt_events, gdelt_tone, gdelt_country_tension
    polymarket/
      client.ts           # Polymarket Gamma API client (no auth, cache)
      schemas/common.ts
      tools/markets.ts    # polymarket_events, polymarket_odds, polymarket_geopolitical
    gdacs/
      client.ts           # GDACS API client (no auth, GeoJSON cache)
      schemas/common.ts
      tools/alerts.ts     # gdacs_alerts, gdacs_events, gdacs_country_exposure
    usgs/
      client.ts           # USGS Earthquake API client (no auth, cache)
      schemas/common.ts
      tools/earthquakes.ts # usgs_earthquakes, usgs_significant
    nasa/
      firms-client.ts     # FIRMS API client (API key auth, cache)
      eonet-client.ts     # EONET API client (no auth, cache)
      schemas/common.ts
      tools/fires.ts      # firms_fires, firms_country_fires
      tools/events.ts     # eonet_events, eonet_categories
    eia/
      client.ts           # EIA v2 API client (API key auth, cache)
      schemas/common.ts
      tools/energy.ts     # eia_petroleum, eia_electricity, eia_capacity
    wto/
      client.ts           # WTO API client (no auth, cache)
      schemas/common.ts
      tools/trade.ts      # wto_tariffs, wto_barriers, wto_trade_stats
    usaspending/
      client.ts           # USASpending API client (no auth, cache)
      schemas/common.ts
      tools/spending.ts   # usaspending_contracts, usaspending_agencies
    coingecko/
      client.ts           # CoinGecko API client (no auth, rate limited)
      schemas/common.ts
      tools/sentiment.ts  # coingecko_fear_greed, coingecko_stablecoin_health
    unhcr/
      client.ts           # UNHCR API client (no auth, CC BY 4.0)
      schemas/common.ts
      tools/displacement.ts # unhcr_displacement, unhcr_country
    openmeteo/
      client.ts           # Open-Meteo Archive API client (no auth, cache)
      schemas/common.ts
      tools/climate.ts    # openmeteo_climate_anomaly
  package.json
  tsconfig.json
```

### Extended: `packages/data-mcp-server/src/wb/`

```
packages/data-mcp-server/src/wb/
  tools/
    countries.ts          # (existing)
    data.ts               # (existing)
    indicators.ts         # (existing)
    sources.ts            # (existing)
    governance.ts         # NEW: wb_governance, wb_governance_compare
    climate.ts            # NEW: wb_climate, wb_climate_vulnerability
    poverty.ts            # NEW: wb_poverty, wb_inequality
    health.ts             # NEW: wb_health
    education.ts          # NEW: wb_education
    infrastructure.ts     # NEW: wb_infrastructure, wb_logistics
    trade.ts              # NEW: wb_trade, wb_fdi
  schemas/
    common.ts             # (existing, extend with new schemas)
    governance.ts         # NEW
    development.ts        # NEW (poverty, health, education, infrastructure)
    trade.ts              # NEW
```

### Client Pattern

Every API client follows the established pattern from `packages/data-mcp-server/src/wb/client.ts`:

- **Cache**: In-memory `Map<string, CacheEntry>` with configurable TTL per data type. Eviction on size limit (1000 entries).
- **Rate limiting**: Polite delay between requests (`POLITE_DELAY_MS`). Per-source configuration via environment variables.
- **Circuit breaker**: Consecutive failure counter (threshold: 5). Half-open retry after 60 seconds. Prevents cascade failures when a source is down.
- **Timeout**: `AbortController` with 15-second default. Configurable per source.
- **Error handling**: HTTP status codes surfaced with truncated body. 429 rate-limit errors distinguished for retry logic.

### Auth Patterns

| Auth Type | Sources | Configuration |
|-----------|---------|---------------|
| None | UCDP, GDELT, Polymarket, GDACS, USGS, NASA EONET, WTO, USASpending, CoinGecko, UNHCR, Open-Meteo, World Bank | No env vars required |
| Free API key | ACLED, NASA FIRMS, EIA | `ACLED_API_KEY` + `ACLED_EMAIL`, `NASA_FIRMS_KEY`, `EIA_API_KEY` |

### Cache TTL Strategy

| Data Freshness | TTL | Sources |
|----------------|-----|---------|
| Real-time alerts | 5 minutes | GDACS alerts, USGS significant, NASA FIRMS active fires |
| Near-real-time | 15 minutes | ACLED events, GDELT events, Polymarket odds |
| Hourly | 1 hour | EIA petroleum, CoinGecko Fear & Greed, EONET events |
| Daily | 24 hours | UCDP conflicts, WTO barriers, USASpending, UNHCR |
| Static/annual | 7 days | World Bank indicators (governance, climate, poverty, health, education, infrastructure, trade), Open-Meteo ERA5 |

### Pipeline Integration

#### New Agent Skills (3 files)

- `.claude/skills/geopolitical-data/SKILL.md` -- Conflict, prediction markets, and geopolitical event tools. Assigned to `cfa-macro-analyst`, `cfa-credit-analyst`.
- `.claude/skills/environmental-data/SKILL.md` -- Disaster alerts, seismic, fire, and environmental event tools. Assigned to `cfa-esg-analyst`, `cfa-insurance-analyst`.
- `.claude/skills/trade-alternative-data/SKILL.md` -- Energy, trade, government spending, sentiment, and displacement tools. Assigned to `cfa-commodity-analyst`, `cfa-equity-analyst`.

#### AGENT_SKILLS Updates

| Agent | Added Skills |
|-------|-------------|
| `cfa-credit-analyst` | `+geopolitical-data` |
| `cfa-macro-analyst` | `+geopolitical-data`, `+trade-alternative-data` |
| `cfa-esg-analyst` | `+environmental-data`, `+trade-alternative-data` |
| `cfa-equity-analyst` | `+trade-alternative-data` |
| `cfa-chief-analyst` | `+geopolitical-data`, `+environmental-data` |

#### CFA_INTENTS Updates (4 new entries)

| Intent Pattern | Agent | Confidence |
|----------------|-------|------------|
| "conflict risk", "political stability", "geopolitical" | `cfa-credit-analyst` | 0.85 |
| "disaster", "earthquake", "wildfire", "catastrophe" | `cfa-esg-analyst` | 0.85 |
| "oil production", "energy supply", "trade barriers" | `cfa-macro-analyst` | 0.80 |
| "prediction market", "sentiment index", "fear greed" | `cfa-equity-analyst` | 0.75 |

### Downstream Rust Module Integration Points

These tools provide data that feeds into existing Rust computation modules. The data flows through agent orchestration -- agents call geopolitical/environmental MCP tools, extract structured values, then pass them as inputs to computation MCP tools.

| Rust Module | Input Field(s) | Data Source(s) |
|-------------|----------------|----------------|
| `sovereign/country_risk.rs` | `political_stability_score`, `rule_of_law_score`, `corruption_control_score` | ACLED country summary, UCDP country profile, WB Governance (WGI) |
| `scenarios/sensitivity.rs` | `scenario_probability` | Polymarket odds, GDELT tension index |
| `commodity_trading/storage.rs` | `crude_inventory_level`, `production_volume` | EIA petroleum |
| `insurance/reserving.rs` | `catastrophe_frequency`, `severity_distribution` | GDACS events, USGS earthquakes, NASA FIRMS fires |
| `esg/climate.rs` | `co2_per_capita`, `renewable_share`, `fire_radiative_power` | WB Climate, NASA FIRMS, Open-Meteo anomaly |
| `esg/scoring.rs` | `governance_percentile`, `displacement_per_capita` | WB Governance, UNHCR displacement |
| `macro_economics/international.rs` | `tariff_rate`, `trade_openness`, `fdi_net_inflows` | WTO tariffs, WB Trade, WB FDI |
| `behavioral/sentiment.rs` | `fear_greed_index`, `media_tone` | CoinGecko Fear & Greed, GDELT tone |

## Success Metrics

- ~35 new MCP tools in `packages/geopolitical-mcp-server/` (12 conflict/geopolitical + 10 environmental + 8 trade/supply-chain + 5 alternative)
- ~15 new World Bank tools in `packages/data-mcp-server/` (2 governance + 2 climate + 2 poverty + 1 health + 1 education + 2 infrastructure + 2 trade + 3 composite views)
- All 14 API clients have circuit breaker, cache, and timeout patterns
- Contract tests for all new tools (minimum 1 schema validation test + 1 mock response test per tool)
- 3 new agent skill files with YAML frontmatter and workflow selection tables
- Pipeline AGENT_SKILLS updated for 5 agents, CFA_INTENTS updated with 4 new entries
- No regression in existing 5841 tests or 357 tools
- All free-tier sources functional without paid subscriptions
- API key sources (ACLED, NASA FIRMS, EIA) gracefully degrade with clear error messages when keys are missing

## Phasing

### Phase A: Geopolitical MCP Server Foundation
- Package scaffolding (`packages/geopolitical-mcp-server/`)
- Shared client utilities (cache, circuit breaker, rate limiter)
- Tier 1 tools: ACLED, UCDP, GDELT, Polymarket (12 tools)
- Contract tests for Tier 1

### Phase B: Environmental & Energy
- Tier 2 tools: GDACS, USGS, NASA FIRMS, NASA EONET (10 tools)
- Tier 3 tools: EIA, WTO, USASpending (8 tools)
- Contract tests for Tiers 2 and 3

### Phase C: Alternative Data & World Bank Expansion
- Tier 4 tools: CoinGecko, UNHCR, Open-Meteo (5 tools)
- Tier 5 tools: World Bank governance, climate, poverty, health, education, infrastructure, trade (15 tools)
- Contract tests for Tiers 4 and 5

### Phase D: Agent Integration
- 3 agent skill files
- Pipeline routing updates (AGENT_SKILLS, CFA_INTENTS)
- HNSW embedding updates for new intent patterns
- End-to-end integration testing with existing Rust computation tools

## Out of Scope

- New Rust computation modules (tools provide data inputs to existing modules)
- Paid/premium data sources (all sources are free-tier or open access)
- Real-time streaming or WebSocket connections (all tools are request/response)
- Historical backfill or bulk data ingestion (tools query on-demand per request)
- Custom geopolitical risk scoring models (scoring remains in Rust; these tools provide raw data)
- Modifications to existing World Bank client.ts (new tools reuse the existing `wbFetch` and `CacheTTL` infrastructure)
