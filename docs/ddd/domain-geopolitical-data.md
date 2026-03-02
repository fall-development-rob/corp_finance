# Domain Model: Geopolitical & Alternative Data

## Bounded Context: Geopolitical Intelligence

This bounded context provides real-time and historical geopolitical, environmental, trade, and alternative data through the `packages/geopolitical-mcp-server/` MCP server, with extended World Bank governance and development indicators added to `packages/data-mcp-server/`. These data streams feed directly into existing CFA agent computation modules (sovereign risk, scenario analysis, ESG, commodities, macro economics, behavioral sentiment, insurance, and crypto) to replace static inputs with live intelligence.

### Domain Language (Ubiquitous Language)

| Term | Definition |
|------|-----------|
| **Conflict Event** | A discrete incident of political violence (battle, protest, explosion, or civilian targeting) with geographic coordinates, actors, and fatality count |
| **Conflict Classification** | UCDP standard categorization of organized violence by intensity: war (>1000 battle deaths/year), minor conflict (25-999), or non-conflict |
| **Tension Pair** | A directional relationship between two countries with a computed tone score derived from GDELT event data, indicating cooperative or hostile interactions |
| **Country Instability Index** | A composite 0-100 score blending conflict, unrest, security, and information pillars to quantify near-term sovereign instability risk |
| **Risk Pillar** | One of four weighted dimensions (unrest, conflict, security, information) that compose the Country Instability Index |
| **Travel Advisory** | A government-issued risk level (1-4 scale) indicating safety conditions for a country |
| **Disaster Alert** | A multi-hazard notification from GDACS with severity classification (green, orange, red) covering earthquakes, floods, cyclones, and other natural events |
| **Fire Detection** | A satellite-observed thermal anomaly from NASA FIRMS with brightness temperature, fire radiative power, and confidence score |
| **Environmental Event** | A NASA EONET-catalogued natural event (wildfire, volcanic eruption, severe storm, iceberg) with open or closed status |
| **Trade Barrier** | A WTO-registered tariff, SPS measure, TBT regulation, or anti-dumping duty that restricts cross-border goods flow |
| **Federal Award** | A US government contract, grant, or loan disbursement tracked by USASpending.gov |
| **Prediction Market** | A binary outcome contract with yes/no prices (0-1) reflecting crowd-sourced event probability |
| **Displacement Flow** | A UNHCR-tracked population movement from origin to destination country with headcount and classification (refugee, IDP, asylum seeker) |
| **Climate Anomaly** | A deviation from ERA5 climatological baseline for temperature or precipitation in a geographic zone, scored by severity |
| **Governance Score** | A World Bank WGI dimension score ranging from -2.5 (worst) to +2.5 (best) across voice/accountability, stability, effectiveness, regulatory quality, rule of law, and corruption control |
| **ACL (Anti-Corruption Layer)** | A translation boundary that normalizes raw API responses into domain value objects, shielding computation modules from external schema changes |

### Aggregates

#### Conflict Events Aggregate
- Root: `ConflictEventRepository`
- Entities: ACLED Event, UCDP Conflict Episode, GDELT Event Record
- Data Sources: ACLED API (battles, protests, explosions/remote violence, violence against civilians), UCDP API (conflict classification), GDELT Project (geo-events, tone analysis)
- Value Objects:
  - `ConflictEvent { lat: Decimal, lon: Decimal, date: Date, event_type: ConflictType, fatalities: u32, actor1: String, actor2: String, country_iso: String, source: DataSource }`
  - `ConflictClassification { intensity: Intensity, conflict_type: ConflictCategory, parties: Vec<String>, battle_deaths_year: u32 }`
  - `TensionPair { country_a: CountryCode, country_b: CountryCode, tone_score: Decimal, event_count: u32, period: DateRange }`
- Invariants:
  - Events must reference valid ISO 3166-1 alpha-2 or alpha-3 country codes
  - Fatality counts must be >= 0
  - Conflict classification must follow UCDP standard categories (state-based, non-state, one-sided)
  - ACLED event types restricted to the canonical set (battles, protests, riots, explosions/remote violence, violence against civilians, strategic developments)
  - Tone scores bounded to GDELT range (-100 to +100)

#### Country Instability Aggregate
- Root: `CountryInstabilityAssessment`
- Entities: Country Profile, Pillar Assessment, Historical Trend
- Data Sources: Composite of conflict, unrest, security, and information indicators derived from Conflict Events, World Bank WGI, and advisory feeds
- Value Objects:
  - `InstabilityScore { value: Decimal, classification: RiskTier, updated_at: DateTime }`
  - `RiskPillar { dimension: PillarDimension, score: Decimal, weight: Decimal, sub_indicators: Vec<SubIndicator> }`
  - `TravelAdvisory { country: CountryCode, level: u8, issuer: String, effective_date: Date }`
- Invariants:
  - Composite score must be 0-100
  - Active war classification (UCDP intensity >= war threshold) forces minimum score of 70
  - Pillar weights must sum to 1.0
  - Individual pillar scores must be 0-100
  - Travel advisory levels must be 1-4

#### Natural Disasters Aggregate
- Root: `DisasterEventRepository`
- Entities: GDACS Alert, USGS Earthquake, NASA FIRMS Detection, NASA EONET Event
- Data Sources: GDACS API (multi-hazard alerts), USGS Earthquake Hazards API, NASA FIRMS (satellite fire detection), NASA EONET (environmental events)
- Value Objects:
  - `DisasterAlert { alert_id: String, hazard_type: HazardType, severity: AlertSeverity, lat: Decimal, lon: Decimal, date: DateTime, population_exposed: Option<u64>, country_iso: CountryCode }`
  - `Earthquake { event_id: String, magnitude: Decimal, depth_km: Decimal, lat: Decimal, lon: Decimal, timestamp: DateTime, tsunami_flag: bool }`
  - `FireDetection { lat: Decimal, lon: Decimal, brightness_k: Decimal, frp_mw: Decimal, confidence: u8, satellite: String, acq_date: Date }`
  - `EnvironmentalEvent { event_id: String, category: EventCategory, title: String, status: EventStatus, geometries: Vec<GeoPoint>, sources: Vec<String> }`
- Invariants:
  - Earthquake magnitude filter threshold at 4.5+ (below this is noise for financial impact)
  - Fire detection confidence must be 0-100
  - GDACS alerts filtered to orange and red severity only (green alerts excluded as not financially material)
  - EONET event status restricted to { open, closed }
  - Hazard types restricted to GDACS canonical set (earthquake, flood, cyclone, volcano, drought, wildfire)

#### Energy Supply Aggregate
- Root: `EnergyDataRepository`
- Entities: EIA Time Series, Production Report, Inventory Level
- Data Sources: EIA (U.S. Energy Information Administration) API v2
- Value Objects:
  - `EnergyDataPoint { series_id: String, value: Decimal, unit: String, period: String, frequency: Frequency }`
- Invariants:
  - Series IDs must be valid EIA API series identifiers
  - Values must be non-negative for production and inventory series
  - Frequency restricted to { weekly, monthly, quarterly, annual }

#### Trade Policy Aggregate
- Root: `TradePolicyRepository`
- Entities: WTO Tariff Schedule, SPS Measure, TBT Notification, Trade Dispute
- Data Sources: WTO API (tariff data, trade barriers, SPS/TBT notifications)
- Value Objects:
  - `TradeBarrier { barrier_type: BarrierType, reporting_country: CountryCode, affected_country: CountryCode, product_hs_code: String, measure_description: String, effective_date: Date, ad_valorem_rate: Option<Decimal> }`
  - `TariffRate { hs_code: String, mfn_rate: Decimal, bound_rate: Decimal, preferential_rates: Vec<PreferentialRate> }`
- Invariants:
  - Tariff rates must be >= 0
  - HS codes must follow standard 2/4/6/8 digit format
  - Barrier types restricted to { tariff, sps, tbt, anti_dumping, countervailing, safeguard, quota }

#### Government Spending Aggregate
- Root: `FederalAwardRepository`
- Entities: Federal Contract, Grant, Loan, Direct Payment
- Data Sources: USASpending.gov API
- Value Objects:
  - `FederalAward { award_id: String, recipient: String, amount: Decimal, agency: String, award_type: AwardType, naics_code: String, period_of_performance: DateRange, place_of_performance: GeoPoint }`
- Invariants:
  - Award amounts must be non-negative
  - Award types restricted to { contract, grant, loan, direct_payment, other }
  - NAICS codes must be valid 2-6 digit codes

#### Prediction Markets Aggregate
- Root: `PredictionMarketRepository`
- Entities: Polymarket Contract, Market Resolution
- Data Sources: Polymarket API (event contracts, order book data)
- Value Objects:
  - `PredictionMarket { question: String, yes_price: Decimal, no_price: Decimal, volume_24h: Decimal, liquidity: Decimal, end_date: DateTime, category: MarketCategory }`
- Invariants:
  - Yes and no prices must be in range [0, 1]
  - yes_price + no_price must approximate 1.0 (within spread tolerance)
  - Volume and liquidity must be >= 0

#### Sentiment Indicators Aggregate
- Root: `SentimentIndexRepository`
- Entities: CoinGecko Fear & Greed Index, Stablecoin Peg Monitor
- Data Sources: CoinGecko API (crypto fear & greed), on-chain stablecoin price feeds
- Value Objects:
  - `SentimentIndex { value: Decimal, classification: SentimentClassification, timestamp: DateTime }`
  - `StablecoinPegHealth { coin: String, current_price: Decimal, peg_target: Decimal, deviation_pct: Decimal, is_depegged: bool }`
- Invariants:
  - Fear & Greed index must be 0-100
  - Classification must follow canonical set { extreme_fear, fear, neutral, greed, extreme_greed }
  - Stablecoin depeg threshold at 0.5% deviation from target

#### Humanitarian Aggregate
- Root: `DisplacementRepository`
- Entities: UNHCR Population Statistics, Displacement Event
- Data Sources: UNHCR API (refugee statistics, displacement flows)
- Value Objects:
  - `DisplacementFlow { origin_country: CountryCode, destination_country: CountryCode, population: u64, classification: DisplacementType, year: u16 }`
- Invariants:
  - Population counts must be > 0
  - Classification restricted to { refugee, idp, asylum_seeker, stateless, returned }
  - Country codes must be valid ISO 3166-1

#### Climate Anomalies Aggregate
- Root: `ClimateAnomalyRepository`
- Entities: ERA5 Reanalysis Record, Zone Anomaly Assessment
- Data Sources: Open-Meteo ERA5 API (historical climate reanalysis)
- Value Objects:
  - `ClimateAnomaly { zone: GeoZone, temp_delta_c: Decimal, precip_delta_pct: Decimal, severity: AnomalySeverity, baseline_period: String, observation_period: String }`
- Invariants:
  - Severity restricted to { normal, moderate, severe, extreme }
  - Baseline period must be a valid WMO standard period (e.g., 1991-2020)
  - Temperature delta expressed in Celsius, precipitation delta as percentage change

#### Governance Indicators Aggregate (World Bank Extended)
- Root: `GovernanceIndicatorRepository`
- Entities: WGI Country Score, Dimension Time Series
- Data Sources: World Bank WGI (Worldwide Governance Indicators) API
- Value Objects:
  - `GovernanceScore { country: CountryCode, dimension: GovernanceDimension, estimate: Decimal, percentile_rank: Decimal, std_error: Decimal, year: u16 }`
- Dimensions: voice_and_accountability, political_stability, government_effectiveness, regulatory_quality, rule_of_law, control_of_corruption
- Invariants:
  - Estimate scores must range from -2.5 to +2.5 per WGI standard
  - Percentile ranks must be 0-100
  - Country codes must be valid ISO 3166-1 alpha-2 or alpha-3

#### Development Indicators Aggregate (World Bank Extended)
- Root: `DevelopmentIndicatorRepository`
- Entities: Climate Indicator, Poverty Indicator, Health Indicator, Education Indicator, Infrastructure Indicator, Trade Indicator
- Data Sources: World Bank Open Data API (extended indicator set)
- Value Objects:
  - `DevelopmentIndicator { indicator_code: String, country: CountryCode, year: u16, value: Decimal, unit: String, source: String }`
- Invariants:
  - Indicator codes must be valid World Bank indicator identifiers
  - Country codes must be valid ISO 3166-1

### Context Map

```
+-----------------------------------------------------------------------+
|                  Geopolitical Intelligence Context                     |
|                  (packages/geopolitical-mcp-server/)                   |
|                                                                       |
|  +-------------------+  +-------------------+  +-------------------+  |
|  |    Conflict        |  |   Geopolitical    |  |   Environmental   |  |
|  |    Events          |  |   Risk            |  |   Events          |  |
|  |    Aggregate       |  |   Aggregate       |  |   Aggregate       |  |
|  | (ACLED/UCDP/GDELT) |  | (Instability/CII) |  | (GDACS/USGS/NASA) |  |
|  +--------+-----------+  +--------+----------+  +--------+----------+  |
|           |                       |                       |            |
|  +--------+-----------+  +-------+----------+  +---------+---------+  |
|  |   Trade & Supply   |  |   Alternative    |  |      WB Extended  |  |
|  |   Chain            |  |   Data           |  |    (Governance +  |  |
|  |   Aggregate        |  |   Aggregate      |  |    Development)   |  |
|  | (EIA/WTO/USASpend) |  | (Poly/CG/UNHCR/  |  |                   |  |
|  |                    |  |  Open-Meteo)      |  |                   |  |
|  +--------+-----------+  +--------+----------+  +--------+----------+  |
|           |                       |                       |            |
+-----------|----------All-data-through-ACL--------|--------|-----------+
            |                       |                       |
            |   +-------------------+-------------------+   |
            |   |                                       |   |
            v   v                                       v   v
+-----------+---+---------------------------------------+---+-----------+
|                         CFA Agent Core                                |
|                   (crates/corp-finance-core/)                         |
|                                                                       |
|  +-------------------+  +-------------------+  +-------------------+  |
|  | sovereign/         |  | scenarios/         |  | esg/              |  |
|  |  country_risk.rs   |  |  sensitivity.rs    |  |  scoring.rs       |  |
|  |  (political_       |  |  (geopolitical     |  |  (governance/     |  |
|  |   stability_score, |  |   probability      |  |   social pillar   |  |
|  |   rule_of_law_     |  |   weighting)       |  |   enrichment)     |  |
|  |   score)           |  |                    |  |                   |  |
|  +-------------------+  +-------------------+  |  climate.rs        |  |
|                                                 |  (physical risk    |  |
|  +-------------------+  +-------------------+  |   from disasters)  |  |
|  | commodity_trading/ |  | macro_economics/  |  +-------------------+  |
|  |  storage.rs        |  |  international.rs |                        |
|  |  (supply           |  |  (trade balance,  |  +-------------------+  |
|  |   disruption       |  |   FX, tariff      |  | insurance/         |  |
|  |   risk)            |  |   impact)         |  |  reserving.rs      |  |
|  +-------------------+  +-------------------+  |  (catastrophe      |  |
|                                                 |   reserve impact)  |  |
|  +-------------------+  +-------------------+  +-------------------+  |
|  | behavioral/        |  | crypto/            |                       |  |
|  |  sentiment.rs      |  |  valuation.rs      |                       |  |
|  |  (fear_greed,      |  |  (stablecoin peg,  |                       |  |
|  |   risk_appetite)   |  |   crypto sentiment)|                       |  |
|  +-------------------+  +-------------------+                        |
|                                                                       |
+-----------------------------------------------------------------------+
            |                       |                       |
            v                       v                       v
+-----------+---+  +----------------+--+  +-----------------+-+
|  MCP Server   |  |  Data MCP Server |  |  Partner MCP      |
|  (195 tools)  |  |  (75 + WB ext)   |  |  Server (87 tools)|
+---------------+  +-------------------+  +-------------------+
```

### Anti-Corruption Layer

Each external data source is accessed through a dedicated ACL client that translates raw API responses into domain value objects. The ACL shields the geopolitical bounded context from:

1. **Schema volatility** -- External APIs change response formats without notice. The ACL absorbs these changes in a single translation point.
2. **Data quality variance** -- Raw feeds contain duplicates, incomplete records, and inconsistent identifiers. The ACL validates and normalizes before domain entry.
3. **Rate limiting and caching** -- Each ACL client manages its own rate limiter and cache (following the pattern established in `packages/data-mcp-server/`).
4. **Identifier normalization** -- Country names, ISO codes, coordinate systems, and date formats are normalized to domain-standard representations at the ACL boundary.

| External API | ACL Client | Key Responsibilities | Domain Value Object |
|-------------|------------|---------------------|---------------------|
| ACLED | AcledClient | Validate ISO codes, normalize event types | ConflictEvent |
| UCDP | UcdpClient | Map UCDP categories, validate death counts | ConflictClassification |
| GDELT | GdeltClient | Parse tone scores, filter relevance | TensionPair |
| GDACS | GdacsClient | Filter severity (orange/red), parse GeoRSS/CAP | DisasterAlert |
| USGS | UsgsClient | Filter by magnitude (>=4.5), validate coordinates | Earthquake |
| NASA FIRMS | FirmsClient | Validate confidence, parse CSV/JSON | FireDetection |
| NASA EONET | EonetClient | Map event categories, track open/closed status | EnvironmentalEvent |
| EIA | EiaClient | Validate series IDs, normalize units | EnergyDataPoint |
| WTO | WtoClient | Validate HS codes, normalize tariff rates | TradeBarrier |
| USASpending | UsaSpendingClient | Validate NAICS codes, normalize amounts | FederalAward |
| Polymarket | PolymarketClient | Validate price bounds (0-1), filter by volume | PredictionMarket |
| CoinGecko | CoinGeckoClient | Validate 0-100 range, classify sentiment level | SentimentIndex |
| UNHCR | UnhcrClient | Validate ISO codes, classify displacement type | DisplacementFlow |
| Open-Meteo | OpenMeteoClient | Compute anomaly deltas, classify severity | ClimateAnomaly |
| WB WGI | WgiClient | Validate -2.5/+2.5 range, normalize dimensions | GovernanceScore |
| WB Open Data | WbExtendedClient | Validate indicator codes, normalize units | DevelopmentIndicator |

Each ACL client follows the established pattern from `packages/data-mcp-server/`:
- Independent `client.ts` with typed fetch wrapper
- Per-source rate limiter (token bucket)
- Per-source cache with configurable TTL (SHORT for real-time feeds like ACLED/GDACS, LONG for reference data like WGI/WTO)
- Zod schemas for input validation at the MCP tool boundary
- Error normalization to a common `DataSourceError` type

### Domain Services

#### CountryRiskEnricher
Feeds real-time conflict and instability data into `sovereign/country_risk.rs`, replacing the currently static `political_stability_score` and `rule_of_law_score` inputs with live values derived from ACLED event counts, UCDP classification, GDELT tone analysis, and WGI governance scores.

**Input mapping:**
| CountryRiskInput field | Geopolitical source |
|------------------------|---------------------|
| `political_stability_score` | Composite of: WGI political_stability estimate (rescaled 0-100), ACLED fatality-weighted event density, UCDP conflict intensity |
| `rule_of_law_score` | WGI rule_of_law estimate (rescaled 0-100), blended with corruption control |
| `sovereign_default_history` | Existing (unchanged) |
| `gdp_growth_rate`, `inflation_rate`, etc. | Existing macro inputs (unchanged, sourced from FRED/WB) |

#### ScenarioProbabilityWeighter
Feeds geopolitical risk assessments into `scenarios/sensitivity.rs` to provide probability weights for scenario analysis. Conflict escalation data, prediction market odds, and instability scores inform the likelihood assigned to bull/base/bear scenarios.

**Integration pattern:**
- Country Instability Index > 70 automatically triggers a "geopolitical stress" scenario
- Prediction market prices for relevant geopolitical events provide empirical probability anchors
- Trade barrier changes trigger supply chain disruption scenarios for affected commodity models

#### EnvironmentalImpactFeeder
Feeds disaster and climate data into `insurance/reserving.rs` (catastrophe reserve adjustments) and `esg/climate.rs` (physical risk scoring).

**Integration pattern:**
- GDACS red alerts within a company's operational geography trigger reserve reassessment
- Cumulative fire detection density feeds into physical climate risk scores
- EONET volcanic/cyclone events feed supply chain disruption models
- Climate anomaly severity maps to TCFD physical risk categories

#### TradeFlowAnalyzer
Feeds tariff and energy supply data into `commodity_trading/storage.rs` (supply disruption risk) and `macro_economics/international.rs` (trade balance and tariff impact on FX).

**Integration pattern:**
- New WTO SPS/TBT measures on agricultural commodities trigger storage cost repricing
- EIA inventory drawdowns below 5-year average flag supply tightness for energy commodity models
- USASpending defense contract surges feed fiscal analysis inputs
- Tariff rate changes feed into CIP/UIP forward rate adjustments

#### AlternativeDataSynthesizer
Feeds alternative signals into `behavioral/sentiment.rs` (fear & greed enrichment), `scenarios/` (probability calibration), and `crypto/` (stablecoin monitoring).

**Integration pattern:**
- CoinGecko Fear & Greed index maps to `risk_appetite_indicators` in `SentimentInput`
- Stablecoin depeg events (>0.5% deviation) trigger crypto stress scenarios
- UNHCR displacement surges correlate to sovereign risk premium adjustments for affected regions
- Prediction market odds for rate decisions feed monetary policy scenario weights

#### GovernanceEnricher
Feeds WGI governance scores into `esg/scoring.rs` (governance and social pillar enrichment) and `macro_economics/` (institutional quality inputs).

**Integration pattern:**
- WGI control_of_corruption maps to ESG governance pillar inputs
- WGI voice_and_accountability feeds social pillar scoring
- WGI regulatory_quality and government_effectiveness feed macro institutional risk models
- Development indicators (poverty, health, education) enrich ESG social metrics for sovereign and corporate analysis

### Context Mapping Patterns

| Pattern | Relationship | Description |
|---------|-------------|-------------|
| **Anti-Corruption Layer** | External APIs --> Geopolitical Context | Every external data source enters through a dedicated ACL client that validates, normalizes, and translates raw responses into domain value objects. Schema changes in external APIs are absorbed at the ACL boundary. |
| **Customer-Supplier** | Geopolitical Context --> sovereign/country_risk.rs | Country risk module defines what inputs it needs (political_stability_score, rule_of_law_score). Geopolitical context supplies enriched values conforming to the existing interface. |
| **Customer-Supplier** | Geopolitical Context --> scenarios/sensitivity.rs | Scenario module defines scenario structures. Geopolitical context supplies probability weights and trigger conditions. |
| **Customer-Supplier** | Geopolitical Context --> esg/scoring.rs, esg/climate.rs | ESG modules define metric interfaces. Geopolitical context supplies governance scores and physical risk data. |
| **Customer-Supplier** | Geopolitical Context --> behavioral/sentiment.rs | Sentiment module defines RiskIndicator inputs. Geopolitical context supplies alternative sentiment indicators. |
| **Published Language** | Geopolitical Context --> All consumers | Domain events (see below) provide a stable, versioned interface for cross-context communication. Consumers subscribe to events rather than polling tools. |
| **Conformist** | WB Extended --> World Bank API | The extended World Bank integration conforms to the existing WB API patterns established in `packages/data-mcp-server/src/wb/`, reusing the same client infrastructure, cache strategy, and tool registration patterns. |
| **Partnership** | Conflict Events <--> Country Instability | Tight collaboration: conflict event data is a primary input to instability scoring. Changes in either aggregate require coordinated updates. |
| **Open Host Service** | Geopolitical MCP Server | Exposes all geopolitical data through standard MCP tool protocol, consumable by any CFA agent. |

### Domain Events

| Event | Trigger | Consumers |
|-------|---------|-----------|
| `ConflictEscalated { country, intensity, fatalities_30d }` | UCDP classification change or ACLED fatality threshold breach | CountryRiskEnricher, ScenarioProbabilityWeighter |
| `DisasterAlerted { alert_id, hazard_type, severity, country }` | GDACS orange or red alert issued | EnvironmentalImpactFeeder, InsuranceReserveAdjuster |
| `TensionShifted { country_a, country_b, tone_delta }` | GDELT tone score shifts beyond threshold (>10 points in 7 days) | CountryRiskEnricher, ScenarioProbabilityWeighter |
| `InstabilityThresholdBreached { country, score, previous_score }` | Country Instability Index crosses tier boundary (30/50/70) | All downstream risk models |
| `TradeBarrierImposed { country, hs_code, barrier_type, rate }` | New WTO SPS/TBT/tariff notification | TradeFlowAnalyzer, CommodityStorageModel |
| `EnergySupplyShock { series, value, deviation_from_mean }` | EIA inventory or production deviates >2 sigma from 5-year average | TradeFlowAnalyzer, CommoditySpreadModel |
| `StablecoinDepegged { coin, deviation_pct }` | Stablecoin price deviates >0.5% from peg target | AlternativeDataSynthesizer, CryptoStressScenario |
| `DisplacementSurge { origin, destination, population, delta_pct }` | UNHCR displacement flow increases >25% year-over-year | CountryRiskEnricher, SovereignRiskPremium |
| `GovernanceScoreChanged { country, dimension, new_score, delta }` | WGI annual update with score change >0.25 | GovernanceEnricher, ESGScoringModel |
| `ClimateAnomalyDetected { zone, severity, temp_delta, precip_delta }` | ERA5 anomaly exceeds "severe" threshold | EnvironmentalImpactFeeder, ClimateRiskModel |
| `PredictionMarketShift { question, old_price, new_price, delta }` | Polymarket contract price moves >10% in 24 hours | ScenarioProbabilityWeighter |

### Event Flow

1. User invokes geopolitical data tool (e.g., `acled_events`, `country_instability`, `gdacs_alerts`)
2. ACL client fetches from external API, validates, and normalizes to domain value objects
3. MCP tool returns structured data to the calling agent
4. Agent (or domain service) detects threshold breaches and raises domain events
5. Domain events trigger enrichment of existing computation module inputs
6. Enriched inputs flow through standard MCP computation tools (e.g., `country_risk_assessment`, `sensitivity_analysis`, `esg_score`)
7. Computation results incorporate live geopolitical intelligence instead of static assumptions

### MCP Tool Inventory

#### Conflict Intelligence (8 tools)
| Tool | Description |
|------|-------------|
| `acled_events` | Query ACLED conflict events by country, date range, event type |
| `acled_fatality_summary` | Aggregate fatality statistics by country and period |
| `ucdp_conflicts` | Query UCDP conflict classifications and intensity levels |
| `ucdp_battle_deaths` | Historical battle death counts by conflict and year |
| `gdelt_events` | Query GDELT geo-events by country pair and date range |
| `gdelt_tone` | Get GDELT tone analysis for country pairs (tension scores) |
| `gdelt_trending` | Trending geopolitical themes and event clusters |
| `conflict_timeline` | Composite timeline blending ACLED, UCDP, and GDELT for a country |

#### Geopolitical Risk (4 tools)
| Tool | Description |
|------|-------------|
| `country_instability` | Compute Country Instability Index (composite 0-100) |
| `risk_pillar_breakdown` | Detailed pillar-level scores with sub-indicators |
| `instability_trend` | Historical instability score trend for a country |
| `travel_advisory` | Current travel advisory levels by issuing government |

#### Environmental Events (8 tools)
| Tool | Description |
|------|-------------|
| `gdacs_alerts` | Current GDACS disaster alerts (orange/red severity) |
| `gdacs_history` | Historical GDACS alerts by hazard type and region |
| `usgs_earthquakes` | Recent earthquakes above magnitude threshold |
| `usgs_earthquake_detail` | Detailed seismic data for a specific earthquake event |
| `nasa_fires` | Active fire detections from NASA FIRMS by region |
| `nasa_fire_density` | Fire detection density analysis for geographic zones |
| `eonet_events` | Current NASA EONET environmental events |
| `eonet_by_category` | EONET events filtered by category (wildfire, volcano, storm, iceberg) |

#### Trade & Supply Chain (8 tools)
| Tool | Description |
|------|-------------|
| `eia_series` | Query EIA energy data series (oil, gas, electricity) |
| `eia_petroleum_summary` | Summary of US petroleum supply and inventory |
| `wto_tariffs` | Query WTO tariff rates by country and HS code |
| `wto_barriers` | Active SPS/TBT trade barrier notifications |
| `wto_disputes` | WTO trade dispute status and rulings |
| `usaspending_awards` | Federal contract and grant awards by agency and recipient |
| `usaspending_by_sector` | Federal spending aggregated by NAICS sector |
| `usaspending_trends` | Federal spending trends over time |

#### Alternative Data (8 tools)
| Tool | Description |
|------|-------------|
| `polymarket_events` | Prediction market contracts and current prices |
| `polymarket_by_category` | Prediction markets filtered by category (politics, economics, geopolitics) |
| `coingecko_fear_greed` | Current crypto Fear & Greed index and history |
| `stablecoin_peg` | Stablecoin peg health monitoring (USDT, USDC, DAI) |
| `unhcr_displacement` | UNHCR displacement flow statistics by origin/destination |
| `unhcr_country_profile` | Refugee and IDP population for a country |
| `climate_anomaly` | ERA5 temperature and precipitation anomalies by zone |
| `climate_anomaly_trend` | Historical climate anomaly trends for a region |

#### World Bank Extended (6 tools, added to packages/data-mcp-server/)
| Tool | Description |
|------|-------------|
| `wb_wgi_scores` | WGI governance scores for a country (6 dimensions) |
| `wb_wgi_comparison` | Compare WGI scores across multiple countries |
| `wb_wgi_trend` | Historical WGI score trends for a country |
| `wb_development_climate` | Climate-related development indicators |
| `wb_development_poverty` | Poverty and inequality indicators |
| `wb_development_health` | Health and education indicators |

**Total: 42 new tools** (36 in geopolitical-mcp-server + 6 extended in data-mcp-server)

### Package Structure

`packages/geopolitical-mcp-server/src/` follows the same per-source directory pattern as `packages/data-mcp-server/`. Each source gets a subdirectory with three standard files:

| Directory | ACL Client | Tools |
|-----------|-----------|-------|
| `acled/` | ACLED conflict events | `acled_events`, `acled_fatality_summary` |
| `ucdp/` | UCDP conflict classification | `ucdp_conflicts`, `ucdp_battle_deaths` |
| `gdelt/` | GDELT geo-events and tone | `gdelt_events`, `gdelt_tone`, `gdelt_trending` |
| `gdacs/` | GDACS multi-hazard alerts | `gdacs_alerts`, `gdacs_history` |
| `usgs/` | USGS earthquake hazards | `usgs_earthquakes`, `usgs_earthquake_detail` |
| `nasa/` | NASA FIRMS + EONET | `nasa_fires`, `nasa_fire_density`, `eonet_events`, `eonet_by_category` |
| `eia/` | EIA energy data | `eia_series`, `eia_petroleum_summary` |
| `wto/` | WTO trade policy | `wto_tariffs`, `wto_barriers`, `wto_disputes` |
| `usaspending/` | USASpending.gov | `usaspending_awards`, `usaspending_by_sector`, `usaspending_trends` |
| `polymarket/` | Polymarket predictions | `polymarket_events`, `polymarket_by_category` |
| `coingecko/` | CoinGecko sentiment | `coingecko_fear_greed`, `stablecoin_peg` |
| `unhcr/` | UNHCR displacement | `unhcr_displacement`, `unhcr_country_profile` |
| `openmeteo/` | Open-Meteo ERA5 climate | `climate_anomaly`, `climate_anomaly_trend` |
| `risk/` | Composite instability scoring | `country_instability`, `risk_pillar_breakdown`, `instability_trend`, `travel_advisory` |

Each subdirectory contains: `client.ts` (ACL fetch + cache + rate limiter), `schemas/` (Zod validation), `tools/` (MCP tool registration). Entry point: `index.ts`.

World Bank extensions added to `packages/data-mcp-server/src/wb/`: `tools/governance.ts` (3 WGI tools), `tools/development.ts` (3 development tools), with corresponding `schemas/` files.

### Authentication & Cache Strategy

API keys are loaded from environment variables at runtime. They are never hardcoded or committed. Public APIs degrade gracefully to rate-limited access when no key is provided.

| Source | Auth | Env Variable | Cache TTL |
|--------|------|-------------|-----------|
| ACLED | API key (header) | `ACLED_API_KEY` + `ACLED_EMAIL` | 1 hour |
| UCDP | Public | -- | 24 hours |
| GDELT | Public | -- | 15 minutes |
| GDACS | Public (RSS) | -- | 15 minutes |
| USGS | Public | -- | 5 minutes |
| NASA FIRMS | API key (query) | `NASA_FIRMS_API_KEY` | 1 hour |
| NASA EONET | Public | -- | 1 hour |
| EIA | API key (query) | `EIA_API_KEY` | 6 hours |
| WTO | Public | -- | 24 hours |
| USASpending | Public | -- | 6 hours |
| Polymarket | Public | -- | 5 minutes |
| CoinGecko | API key (optional) | `COINGECKO_API_KEY` | 15 minutes |
| UNHCR | Public | -- | 24 hours |
| Open-Meteo | Public | -- | 6 hours |
| WB WGI | Public | -- | 7 days |
| WB Development | Public | -- | 24 hours |
