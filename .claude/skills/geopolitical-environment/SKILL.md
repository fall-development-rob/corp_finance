---
name: geopolitical-environment
description: "Environmental disaster and hazard data -- GDACS disaster alerts, USGS earthquakes, NASA FIRMS fire detection, NASA EONET natural events"
---

# Environmental & Disaster Data

You have access to 9 environment MCP tools across 4 sources for monitoring natural disasters, earthquakes, wildfires, and environmental events. Essential for ESG scoring, supply chain disruption, insurance risk, and infrastructure analysis.

No authentication required for GDACS, USGS, and EONET. NASA FIRMS requires `NASA_FIRMS_API_KEY` (free).

## Tool Reference

### GDACS (3 tools)

| MCP Tool | Description |
|----------|-------------|
| `gdacs_alerts` | Current global disaster alerts (Orange and Red severity only). Returns alertid, eventtype, severity, country, coordinates, population affected, alertscore. |
| `gdacs_events` | Historical events filtered by hazard type (earthquake/flood/cyclone/volcano/drought/wildfire) and optional country. |
| `gdacs_country_exposure` | Aggregate disaster exposure for a country: events by type, max severity, total population affected, most recent event. |

### USGS (2 tools)

| MCP Tool | Description |
|----------|-------------|
| `usgs_earthquakes` | Query earthquakes by minimum magnitude, date range, and limit. Returns magnitude, place, depth, coordinates, tsunami flag, PAGER alert level. |
| `usgs_significant` | Curated significant recent earthquakes (M6.0+). Returns full details including felt reports and intensity estimates. |

### NASA FIRMS (2 tools)

| MCP Tool | Description |
|----------|-------------|
| `firms_fires` | Active fire detections by country (ISO3 code) and days (1-10). Returns lat, lon, brightness, fire radiative power, confidence, date. |
| `firms_country_fires` | Aggregate fire statistics for a country: total detections, high-confidence count, average/max FRP, detections by day. |

### NASA EONET (2 tools)

| MCP Tool | Description |
|----------|-------------|
| `eonet_events` | Current natural events from NASA EONET. Filter by days and status (open/closed). Returns id, title, category, sources, geometries. |
| `eonet_categories` | List EONET event categories with descriptions and active event counts. |

## Usage Notes

- Use `gdacs_alerts` for a real-time global disaster dashboard -- only Orange/Red alerts are returned.
- Use `gdacs_country_exposure` for aggregate natural disaster risk assessment of a country.
- Use `usgs_significant` for a quick view of major seismic events globally.
- NASA FIRMS country codes use ISO 3166-1 alpha-3 (e.g., `USA`, `BRA`, `AUS`).
- FIRMS data is near-real-time (2-4 hour lag via VIIRS satellite).
- GDACS alertscore ranges from 0-3.5 where higher = more severe impact.
