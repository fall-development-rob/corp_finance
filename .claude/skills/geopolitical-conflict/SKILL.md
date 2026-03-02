---
name: geopolitical-conflict
description: "Conflict and political instability data -- ACLED armed conflict events, UCDP battle deaths, GDELT news tone and bilateral tension analysis"
---

# Conflict & Political Instability Data

You have access to 9 conflict MCP tools across 3 sources for tracking armed conflict, political violence, protests, and bilateral tensions. Essential for sovereign risk, country risk, ESG scoring, and supply chain disruption analysis.

No authentication required for UCDP and GDELT. ACLED requires `ACLED_ACCESS_TOKEN` and `ACLED_EMAIL` (free academic access).

## Tool Reference

### ACLED (3 tools)

| MCP Tool | Description |
|----------|-------------|
| `acled_events` | Query conflict events by country, date range, and event type (battles, protests, riots, explosions, violence_against_civilians, strategic_developments). Returns location, actors, fatalities, descriptions. |
| `acled_fatalities` | Aggregate fatality counts by country and date range. Returns total fatalities, event count, and breakdown by event type. |
| `acled_country_summary` | Composite conflict profile: event counts by type, total fatalities, top actors, geographic hotspots over configurable lookback. |

### UCDP (3 tools)

| MCP Tool | Description |
|----------|-------------|
| `ucdp_conflicts` | Geo-referenced armed conflicts by country and date range. Returns conflict ID, violence type (state-based/non-state/one-sided), death estimates, region. |
| `ucdp_battle_deaths` | Time series of battle-related deaths by conflict ID or country. Returns yearly best/low/high death estimates. |
| `ucdp_country_profile` | Country conflict history: years with conflict, total battle deaths, active conflict types, intensity classification (war/minor/none). |

### GDELT (3 tools)

| MCP Tool | Description |
|----------|-------------|
| `gdelt_events` | Search recent news articles by query and timespan. Returns title, URL, source, date, language, tone score (negative=hostile, positive=cooperative). |
| `gdelt_tone` | Daily tone analysis for a query over a timespan. Track sentiment trends -- negative = hostile coverage, positive = cooperative. |
| `gdelt_country_tension` | Bilateral tension analysis between two countries. Returns tone trend, volume trend, and combined tension score (higher = more hostile coverage). |

## Usage Notes

- Use `acled_country_summary` for a quick conflict risk snapshot of any country.
- Use `gdelt_country_tension` to assess bilateral relations (e.g., US-China, Russia-Ukraine).
- GDELT tone scores are noisy -- use multi-day averages rather than single-day readings.
- UCDP classifies conflicts by death threshold: war (>1000/year), minor (<1000/year), none.
- ACLED data is near-real-time (1-2 week lag). UCDP is updated annually.
- Country codes: ACLED uses country names, UCDP uses ISO alpha-3 (e.g., `USA`, `GBR`), GDELT uses free-text queries.
