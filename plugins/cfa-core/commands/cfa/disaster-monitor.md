# Disaster Monitor

Environmental disaster monitoring using GDACS, USGS, NASA FIRMS, and EONET data from `geopolitical-environment`.

## What It Does
Scans current global disaster alerts, recent earthquakes, active wildfires, and natural events. Produces a risk dashboard with severity ratings, affected populations, and supply chain disruption potential.

## Agent
Routes to `cfa-esg-regulatory-analyst`.

## Key Tools
`gdacs_alerts`, `gdacs_country_exposure`, `usgs_significant`, `firms_country_fires`, `eonet_events`

## Usage
Run without arguments for a global dashboard. Specify a country for a focused exposure report (e.g., "Japan earthquake risk", "Australia fire season").
