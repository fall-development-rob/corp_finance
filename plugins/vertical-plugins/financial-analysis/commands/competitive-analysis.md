# Competitive Analysis

Run a competitive landscape and positioning study using the Competitive Analysis section of the `workflow-financial-analysis` skill.

## What It Does
Produces a structured competitive analysis: (1) Porter's Five Forces with low/moderate/high rating and supporting evidence per force, (2) market sizing (TAM / SAM / SOM with explicit methodology), (3) competitive positioning matrix (2-axis map of 6-10 players), (4) financial benchmarking against peers (margins, growth, multiples, capital efficiency), (5) moat assessment (brand, IP, switching costs, network effects, cost advantages) with durability rating.

## Agent
Routes to `cfa-chief-analyst` with `workflow-financial-analysis` skill.

## Key Tools
`comps_analysis`, `fmp_profile`, `fmp_key_metrics`, `fmp_income_statement`, `fmp_balance_sheet`

## Usage
Provide the subject company plus the relevant industry / sub-sector definition. Optionally specify a custom peer list, the positioning-matrix axes (default: price vs quality), and depth of moat analysis. The agent will pull peer financials, score each force, build the positioning map, and produce a moat-durability assessment.

## Output
Markdown report with: Five Forces ratings table (force, rating, evidence), TAM/SAM/SOM table with sources, positioning matrix description (axes, plotted players, white space callouts), peer benchmarking table (margins, growth, multiples, ROIC), moat assessment (each moat dimension scored, overall durability narrow / wide / none with horizon), and key competitive risks summary.
