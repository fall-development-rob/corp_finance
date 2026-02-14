---
name: "FMP Technical Analysis"
description: "Use the fmp-mcp-server technical indicator tools for quantitative technical analysis. Invoke when calculating moving averages (SMA, EMA, WMA, DEMA, TEMA), momentum indicators (RSI, Williams %R, ADX), or volatility measures (standard deviation). Supports multiple timeframes from 1-minute to monthly for both intraday and swing trading analysis."
---

# FMP Technical Analysis Skill

## Overview

This skill covers the 9 technical indicator tools provided by the `fmp-mcp-server` MCP integration. These tools query Financial Modeling Prep (FMP) for real-time and historical technical indicator data, enabling quantitative analysis across multiple timeframes.

## Available Tools

### Moving Averages

#### `fmp_sma` — Simple Moving Average
- **Purpose:** Trend direction identification, dynamic support/resistance levels.
- **Key Inputs:** `symbol`, `periodLength`, `timeframe`
- **How it works:** Calculates the unweighted arithmetic mean of closing prices over the specified period. Equal weight is given to every data point in the window.
- **Typical periods:** 20 (short-term), 50 (medium-term), 200 (long-term).

#### `fmp_ema` — Exponential Moving Average
- **Purpose:** Trend detection with heavier weighting on recent prices, making it more responsive than SMA.
- **Key Inputs:** `symbol`, `periodLength`, `timeframe`
- **How it works:** Applies an exponential decay weighting so that the most recent prices contribute more to the average.
- **Typical periods:** 9, 12, 20, 26, 50.

#### `fmp_wma` — Weighted Moving Average
- **Purpose:** Linear-weighted trend following that prioritizes recent data.
- **Key Inputs:** `symbol`, `periodLength`, `timeframe`
- **How it works:** Assigns linearly increasing weights to each data point in the window (the most recent price gets the highest weight).
- **Typical periods:** 10, 20, 50.

#### `fmp_dema` — Double Exponential Moving Average
- **Purpose:** Reduced-lag trend following for faster signal generation.
- **Key Inputs:** `symbol`, `periodLength`, `timeframe`
- **How it works:** Combines a single EMA with an EMA-of-EMA to cancel out a portion of the inherent lag.
- **Typical periods:** 12, 20, 50.

#### `fmp_tema` — Triple Exponential Moving Average
- **Purpose:** Minimal-lag trend following for the most responsive moving average signals.
- **Key Inputs:** `symbol`, `periodLength`, `timeframe`
- **How it works:** Extends the DEMA concept with a third layer of exponential smoothing, further reducing lag.
- **Typical periods:** 12, 20, 50.

### Momentum Indicators

#### `fmp_rsi` — Relative Strength Index
- **Purpose:** Identify overbought and oversold conditions.
- **Key Inputs:** `symbol`, `periodLength`, `timeframe`
- **Interpretation:** Values above **70** suggest overbought conditions; values below **30** suggest oversold conditions. The 50 level acts as a bullish/bearish divider.
- **Typical period:** 14.

#### `fmp_williams` — Williams %R
- **Purpose:** Momentum oscillator measuring the current close relative to the high-low range over a lookback period.
- **Key Inputs:** `symbol`, `periodLength`, `timeframe`
- **Interpretation:** Readings above **-20** indicate overbought; readings below **-80** indicate oversold. The scale runs from 0 to -100.
- **Typical period:** 14.

#### `fmp_adx` — Average Directional Index
- **Purpose:** Measure trend strength regardless of direction.
- **Key Inputs:** `symbol`, `periodLength`, `timeframe`
- **Interpretation:** Values above **25** indicate a strong trend; values below **20** suggest a weak or range-bound market. ADX does not indicate direction, only strength.
- **Typical period:** 14.

### Volatility

#### `fmp_stddev` — Standard Deviation
- **Purpose:** Quantify price volatility over a given period.
- **Key Inputs:** `symbol`, `periodLength`, `timeframe`
- **How it works:** Calculates the statistical standard deviation of closing prices, providing a measure of dispersion around the mean.
- **Typical periods:** 20, 30.

## Supported Timeframes

All 9 tools accept the same set of timeframe values:

| Value | Description |
|-------|-------------|
| `1min` | 1-minute bars |
| `5min` | 5-minute bars |
| `15min` | 15-minute bars |
| `30min` | 30-minute bars |
| `1hour` | 1-hour bars |
| `4hour` | 4-hour bars |
| `1day` | Daily bars |
| `1week` | Weekly bars |
| `1month` | Monthly bars |

Use shorter timeframes (`1min` through `1hour`) for intraday and day-trading analysis. Use longer timeframes (`1day` through `1month`) for swing trading and position analysis.

## Usage Patterns

### 1. Trend Analysis — Golden/Death Cross Detection

Combine a short-period SMA with a long-period SMA to detect major trend reversals.

```
fmp_sma(symbol="AAPL", periodLength=50, timeframe="1day")
fmp_sma(symbol="AAPL", periodLength=200, timeframe="1day")
```

- **Golden Cross:** The 50-day SMA crosses above the 200-day SMA — bullish signal.
- **Death Cross:** The 50-day SMA crosses below the 200-day SMA — bearish signal.

Compare the most recent values from both responses. If `sma_50 > sma_200` and the previous period had `sma_50 < sma_200`, a golden cross has just occurred.

### 2. Mean Reversion — RSI + Standard Deviation

Use RSI to identify overbought/oversold extremes and standard deviation to add volatility context.

```
fmp_rsi(symbol="MSFT", periodLength=14, timeframe="1day")
fmp_stddev(symbol="MSFT", periodLength=20, timeframe="1day")
```

- **High-confidence oversold:** RSI < 30 AND stddev is elevated (price has deviated significantly from mean — reversion is likely).
- **High-confidence overbought:** RSI > 70 AND stddev is elevated.
- If stddev is low while RSI is extreme, the move may lack the volatility to snap back quickly.

### 3. Momentum Screening — ADX + EMA Filter

Filter for stocks in strong trends, then confirm direction with EMA.

```
fmp_adx(symbol="NVDA", periodLength=14, timeframe="1day")
fmp_ema(symbol="NVDA", periodLength=20, timeframe="1day")
```

- **Strong uptrend:** ADX > 25 AND current price is above the 20-period EMA.
- **Strong downtrend:** ADX > 25 AND current price is below the 20-period EMA.
- **Avoid:** ADX < 20 indicates a weak or non-trending market — momentum strategies are less reliable.

### 4. Multi-Timeframe Confirmation

Run the same indicator across multiple timeframes to confirm signals and avoid false entries.

```
fmp_rsi(symbol="TSLA", periodLength=14, timeframe="1day")
fmp_rsi(symbol="TSLA", periodLength=14, timeframe="1hour")
fmp_rsi(symbol="TSLA", periodLength=14, timeframe="15min")
```

- **Strong signal:** All three timeframes agree (e.g., all show oversold below 30).
- **Weak signal:** Only the shortest timeframe shows the condition — likely noise.
- **General rule:** Higher timeframes carry more weight. A daily oversold reading confirmed by hourly and 15-min is a higher-conviction setup than a 15-min reading alone.

## Best Practices

1. **Always specify `periodLength` and `timeframe` explicitly.** Do not rely on defaults; being explicit ensures reproducible analysis.
2. **Combine indicators from different categories.** A moving average (trend) paired with RSI (momentum) and stddev (volatility) gives a more complete picture than any single indicator alone.
3. **Use multi-timeframe analysis for entry timing.** Identify the trend on a higher timeframe, then time entries on a lower timeframe.
4. **Watch for divergences.** When price makes a new high but RSI or Williams %R does not, a reversal may be forming.
5. **ADX is non-directional.** A rising ADX means the trend is strengthening, but you must use price action or a moving average to determine whether the trend is up or down.
6. **Prefer EMA/DEMA/TEMA over SMA when reduced lag is important** — for example, in fast-moving intraday markets. Use SMA for widely followed levels like the 200-day moving average, where institutional alignment matters.
