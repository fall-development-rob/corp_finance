# Conflict Risk Assessment

Geopolitical conflict risk analysis using ACLED, UCDP, and GDELT data from `geopolitical-conflict`.

## What It Does
Builds a composite conflict risk profile for a country or region: active conflict events, fatality trends, bilateral tensions, and media sentiment. Outputs a risk score with supporting evidence.

## Agent
Routes to `cfa-macro-analyst`.

## Key Tools
`acled_country_summary`, `ucdp_country_profile`, `gdelt_country_tension`, `gdelt_tone`, `wb_governance`

## Usage
Specify a country (e.g., "Ukraine", "Ethiopia") or a bilateral pair (e.g., "US-China tensions"). Optionally specify a lookback period.
