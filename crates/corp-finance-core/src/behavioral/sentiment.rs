use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::{CorpFinanceError, CorpFinanceResult};

// ---------------------------------------------------------------------------
// Input types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskIndicator {
    pub name: String,
    pub value: Decimal,
    pub bullish_threshold: Decimal,
    pub bearish_threshold: Decimal,
    pub weight: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentimentInput {
    pub market_name: String,
    pub vix_current: Decimal,
    pub vix_sma_50: Decimal,
    pub put_call_ratio: Decimal,
    pub put_call_sma_20: Decimal,
    pub advance_decline_ratio: Decimal,
    pub new_highs_lows_ratio: Decimal,
    pub margin_debt_change_pct: Decimal,
    pub fund_flows: Decimal,
    pub short_interest_ratio: Decimal,
    pub insider_buy_sell_ratio: Decimal,
    pub consumer_confidence: Decimal,
    #[serde(default)]
    pub risk_appetite_indicators: Vec<RiskIndicator>,
    #[serde(default)]
    pub contrarian_mode: bool,
}

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndicatorScore {
    pub name: String,
    pub raw_value: Decimal,
    pub normalized_score: Decimal,
    pub signal: String,
    pub weight: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FearGreedBreakdown {
    pub volatility_component: Decimal,
    pub options_component: Decimal,
    pub breadth_component: Decimal,
    pub momentum_component: Decimal,
    pub flow_component: Decimal,
    pub leverage_component: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentimentOutput {
    pub composite_score: Decimal,
    pub sentiment_label: String,
    pub contrarian_signal: String,
    pub indicator_scores: Vec<IndicatorScore>,
    pub fear_greed_decomposition: FearGreedBreakdown,
    pub volatility_regime: String,
    pub market_breadth_signal: String,
    pub smart_money_signal: String,
    pub flow_momentum: String,
    pub risk_on_off: String,
    pub historical_context: String,
}

// ---------------------------------------------------------------------------
// Normalization helpers
// ---------------------------------------------------------------------------

fn clamp(val: Decimal, lo: Decimal, hi: Decimal) -> Decimal {
    if val < lo {
        lo
    } else if val > hi {
        hi
    } else {
        val
    }
}

fn normalize_vix(vix_current: Decimal, vix_sma_50: Decimal) -> Decimal {
    if vix_sma_50 == Decimal::ZERO {
        return dec!(50);
    }
    let ratio = vix_current / vix_sma_50;
    let raw = dec!(100) - (ratio - dec!(0.8)) * dec!(250);
    clamp(raw, Decimal::ZERO, dec!(100))
}

fn normalize_put_call(put_call_ratio: Decimal, put_call_sma_20: Decimal) -> Decimal {
    if put_call_sma_20 == Decimal::ZERO {
        return dec!(50);
    }
    let ratio = put_call_ratio / put_call_sma_20;
    let raw = dec!(100) - (ratio - dec!(0.7)) * dec!(166);
    clamp(raw, Decimal::ZERO, dec!(100))
}

fn normalize_ad_ratio(ad_ratio: Decimal) -> Decimal {
    clamp(ad_ratio * dec!(50), Decimal::ZERO, dec!(100))
}

fn normalize_highs_lows(hl_ratio: Decimal) -> Decimal {
    clamp(hl_ratio * dec!(50), Decimal::ZERO, dec!(100))
}

fn normalize_margin_debt(change_pct: Decimal) -> Decimal {
    clamp(dec!(50) + change_pct * dec!(200), Decimal::ZERO, dec!(100))
}

fn normalize_fund_flows(fund_flows: Decimal) -> Decimal {
    let signum = if fund_flows > Decimal::ZERO {
        Decimal::ONE
    } else if fund_flows < Decimal::ZERO {
        -Decimal::ONE
    } else {
        Decimal::ZERO
    };
    let magnitude = fund_flows.abs() / dec!(1000000) * dec!(50);
    let capped = if magnitude > dec!(50) {
        dec!(50)
    } else {
        magnitude
    };
    clamp(dec!(50) + signum * capped, Decimal::ZERO, dec!(100))
}

fn normalize_short_interest(sir: Decimal) -> Decimal {
    clamp(dec!(100) - sir * dec!(15), Decimal::ZERO, dec!(100))
}

fn normalize_insider(buy_sell_ratio: Decimal) -> Decimal {
    clamp(buy_sell_ratio * dec!(50), Decimal::ZERO, dec!(100))
}

fn normalize_confidence(confidence: Decimal) -> Decimal {
    clamp(confidence, Decimal::ZERO, dec!(100))
}

fn signal_from_score(score: Decimal) -> String {
    if score >= dec!(80) {
        "Extreme Greed".to_string()
    } else if score >= dec!(60) {
        "Greed".to_string()
    } else if score >= dec!(40) {
        "Neutral".to_string()
    } else if score >= dec!(20) {
        "Fear".to_string()
    } else {
        "Extreme Fear".to_string()
    }
}

fn normalize_custom_indicator(ind: &RiskIndicator) -> Decimal {
    // Linear interpolation between bearish and bullish thresholds
    let range = ind.bullish_threshold - ind.bearish_threshold;
    if range == Decimal::ZERO {
        return dec!(50);
    }
    let raw = (ind.value - ind.bearish_threshold) / range * dec!(100);
    clamp(raw, Decimal::ZERO, dec!(100))
}

// ---------------------------------------------------------------------------
// Classification helpers
// ---------------------------------------------------------------------------

fn classify_volatility_regime(vix: Decimal) -> String {
    if vix < dec!(15) {
        "Low Vol".to_string()
    } else if vix <= dec!(25) {
        "Normal".to_string()
    } else if vix <= dec!(35) {
        "Elevated".to_string()
    } else {
        "Crisis".to_string()
    }
}

fn classify_market_breadth(ad_score: Decimal, hl_score: Decimal) -> String {
    let avg = (ad_score + hl_score) / dec!(2);
    if avg >= dec!(75) {
        "Strong".to_string()
    } else if avg >= dec!(50) {
        "Healthy".to_string()
    } else if avg >= dec!(25) {
        "Weakening".to_string()
    } else {
        "Deteriorating".to_string()
    }
}

fn classify_smart_money(insider_score: Decimal, short_score: Decimal) -> String {
    let avg = (insider_score + short_score) / dec!(2);
    if avg >= dec!(60) {
        "Bullish".to_string()
    } else if avg >= dec!(40) {
        "Neutral".to_string()
    } else {
        "Bearish".to_string()
    }
}

fn classify_flow_momentum(flow_score: Decimal) -> String {
    if flow_score >= dec!(80) {
        "Strong Inflow".to_string()
    } else if flow_score >= dec!(60) {
        "Inflow".to_string()
    } else if flow_score >= dec!(40) {
        "Neutral".to_string()
    } else if flow_score >= dec!(20) {
        "Outflow".to_string()
    } else {
        "Strong Outflow".to_string()
    }
}

fn classify_risk_on_off(composite: Decimal) -> String {
    if composite >= dec!(50) {
        "Risk On".to_string()
    } else {
        "Risk Off".to_string()
    }
}

fn contrarian_signal(composite: Decimal) -> String {
    if composite < dec!(20) {
        "Strong Buy".to_string()
    } else if composite < dec!(35) {
        "Buy".to_string()
    } else if composite <= dec!(65) {
        "Neutral".to_string()
    } else if composite <= dec!(80) {
        "Sell".to_string()
    } else {
        "Strong Sell".to_string()
    }
}

fn historical_context(composite: Decimal) -> String {
    if composite < dec!(15) {
        "Readings below 15 have historically preceded significant market rebounds, \
         similar to March 2009 and March 2020 lows."
            .to_string()
    } else if composite < dec!(25) {
        "Fear levels in the 15-25 range have often marked intermediate bottoms \
         and buying opportunities over 6-12 month horizons."
            .to_string()
    } else if composite < dec!(40) {
        "Moderate fear readings suggest cautious positioning but not extreme \
         dislocation. Markets may consolidate before establishing direction."
            .to_string()
    } else if composite <= dec!(60) {
        "Neutral sentiment readings suggest balanced market conditions. \
         Historical returns from these levels are close to long-term averages."
            .to_string()
    } else if composite <= dec!(75) {
        "Moderate greed levels suggest elevated optimism. Markets can continue \
         higher but corrections become more likely."
            .to_string()
    } else if composite <= dec!(85) {
        "Readings in the 75-85 range have preceded increased volatility and \
         potential pullbacks, similar to late-cycle behavior."
            .to_string()
    } else {
        "Extreme greed above 85 has historically preceded significant market corrections. \
         Similar readings were observed before the 2000 and 2007 peaks."
            .to_string()
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub fn analyze_sentiment(input: &SentimentInput) -> CorpFinanceResult<SentimentOutput> {
    // Validation
    if input.market_name.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "market_name".to_string(),
            reason: "Market name cannot be empty".to_string(),
        });
    }

    if input.vix_current < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "vix_current".to_string(),
            reason: "VIX cannot be negative".to_string(),
        });
    }

    if input.vix_sma_50 < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "vix_sma_50".to_string(),
            reason: "VIX SMA cannot be negative".to_string(),
        });
    }

    if input.put_call_ratio < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "put_call_ratio".to_string(),
            reason: "Put/call ratio cannot be negative".to_string(),
        });
    }

    // Normalize each indicator
    let vix_score = normalize_vix(input.vix_current, input.vix_sma_50);
    let pc_score = normalize_put_call(input.put_call_ratio, input.put_call_sma_20);
    let ad_score = normalize_ad_ratio(input.advance_decline_ratio);
    let hl_score = normalize_highs_lows(input.new_highs_lows_ratio);
    let margin_score = normalize_margin_debt(input.margin_debt_change_pct);
    let flow_score = normalize_fund_flows(input.fund_flows);
    let short_score = normalize_short_interest(input.short_interest_ratio);
    let insider_score = normalize_insider(input.insider_buy_sell_ratio);
    let confidence_score = normalize_confidence(input.consumer_confidence);

    // Default weights: vol 20%, options 15%, breadth(A/D) 15%, momentum(H/L) 10%,
    // flows 15%, leverage 10%, insider 10%, confidence 5%
    let vol_w = dec!(0.20);
    let opt_w = dec!(0.15);
    let breadth_w = dec!(0.15);
    let momentum_w = dec!(0.10);
    let flow_w = dec!(0.15);
    let leverage_w = dec!(0.10);
    let insider_w = dec!(0.10);
    let confidence_w = dec!(0.05);

    let mut total_weight =
        vol_w + opt_w + breadth_w + momentum_w + flow_w + leverage_w + insider_w + confidence_w;
    let mut weighted_sum = vix_score * vol_w
        + pc_score * opt_w
        + ad_score * breadth_w
        + hl_score * momentum_w
        + flow_score * flow_w
        + margin_score * leverage_w
        + insider_score * insider_w
        + confidence_score * confidence_w;

    // Build indicator scores
    let mut indicator_scores = vec![
        IndicatorScore {
            name: "VIX".to_string(),
            raw_value: input.vix_current,
            normalized_score: vix_score,
            signal: signal_from_score(vix_score),
            weight: vol_w,
        },
        IndicatorScore {
            name: "Put/Call Ratio".to_string(),
            raw_value: input.put_call_ratio,
            normalized_score: pc_score,
            signal: signal_from_score(pc_score),
            weight: opt_w,
        },
        IndicatorScore {
            name: "Advance/Decline".to_string(),
            raw_value: input.advance_decline_ratio,
            normalized_score: ad_score,
            signal: signal_from_score(ad_score),
            weight: breadth_w,
        },
        IndicatorScore {
            name: "New Highs/Lows".to_string(),
            raw_value: input.new_highs_lows_ratio,
            normalized_score: hl_score,
            signal: signal_from_score(hl_score),
            weight: momentum_w,
        },
        IndicatorScore {
            name: "Fund Flows".to_string(),
            raw_value: input.fund_flows,
            normalized_score: flow_score,
            signal: signal_from_score(flow_score),
            weight: flow_w,
        },
        IndicatorScore {
            name: "Margin Debt".to_string(),
            raw_value: input.margin_debt_change_pct,
            normalized_score: margin_score,
            signal: signal_from_score(margin_score),
            weight: leverage_w,
        },
        IndicatorScore {
            name: "Short Interest".to_string(),
            raw_value: input.short_interest_ratio,
            normalized_score: short_score,
            signal: signal_from_score(short_score),
            weight: insider_w,
        },
        IndicatorScore {
            name: "Insider Buy/Sell".to_string(),
            raw_value: input.insider_buy_sell_ratio,
            normalized_score: insider_score,
            signal: signal_from_score(insider_score),
            weight: insider_w,
        },
        IndicatorScore {
            name: "Consumer Confidence".to_string(),
            raw_value: input.consumer_confidence,
            normalized_score: confidence_score,
            signal: signal_from_score(confidence_score),
            weight: confidence_w,
        },
    ];

    // Add custom risk appetite indicators
    for ind in &input.risk_appetite_indicators {
        let norm = normalize_custom_indicator(ind);
        weighted_sum += norm * ind.weight;
        total_weight += ind.weight;
        indicator_scores.push(IndicatorScore {
            name: ind.name.clone(),
            raw_value: ind.value,
            normalized_score: norm,
            signal: signal_from_score(norm),
            weight: ind.weight,
        });
    }

    let composite = if total_weight > Decimal::ZERO {
        clamp(weighted_sum / total_weight, Decimal::ZERO, dec!(100))
    } else {
        dec!(50)
    };

    let sentiment_label = signal_from_score(composite);
    let contrarian = if input.contrarian_mode {
        contrarian_signal(composite)
    } else {
        "N/A (contrarian mode disabled)".to_string()
    };

    let fear_greed_decomposition = FearGreedBreakdown {
        volatility_component: vix_score,
        options_component: pc_score,
        breadth_component: ad_score,
        momentum_component: hl_score,
        flow_component: flow_score,
        leverage_component: margin_score,
    };

    let volatility_regime = classify_volatility_regime(input.vix_current);
    let market_breadth_signal = classify_market_breadth(ad_score, hl_score);
    let smart_money_signal = classify_smart_money(insider_score, short_score);
    let flow_momentum = classify_flow_momentum(flow_score);
    let risk_on_off = classify_risk_on_off(composite);
    let hist_context = historical_context(composite);

    Ok(SentimentOutput {
        composite_score: composite,
        sentiment_label,
        contrarian_signal: contrarian,
        indicator_scores,
        fear_greed_decomposition,
        volatility_regime,
        market_breadth_signal,
        smart_money_signal,
        flow_momentum,
        risk_on_off,
        historical_context: hist_context,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn default_input() -> SentimentInput {
        SentimentInput {
            market_name: "S&P 500".to_string(),
            vix_current: dec!(20),
            vix_sma_50: dec!(18),
            put_call_ratio: dec!(0.85),
            put_call_sma_20: dec!(0.80),
            advance_decline_ratio: dec!(1.2),
            new_highs_lows_ratio: dec!(1.5),
            margin_debt_change_pct: dec!(0.02),
            fund_flows: dec!(500000),
            short_interest_ratio: dec!(3.0),
            insider_buy_sell_ratio: dec!(1.1),
            consumer_confidence: dec!(65),
            risk_appetite_indicators: vec![],
            contrarian_mode: false,
        }
    }

    #[test]
    fn test_basic_sentiment() {
        let input = default_input();
        let result = analyze_sentiment(&input).unwrap();

        assert!(result.composite_score >= Decimal::ZERO);
        assert!(result.composite_score <= dec!(100));
        assert!(!result.sentiment_label.is_empty());
    }

    #[test]
    fn test_composite_in_range() {
        let input = default_input();
        let result = analyze_sentiment(&input).unwrap();

        assert!(result.composite_score >= Decimal::ZERO);
        assert!(result.composite_score <= dec!(100));
    }

    #[test]
    fn test_extreme_fear_scenario() {
        let input = SentimentInput {
            market_name: "S&P 500".to_string(),
            vix_current: dec!(45),
            vix_sma_50: dec!(20),
            put_call_ratio: dec!(1.5),
            put_call_sma_20: dec!(0.80),
            advance_decline_ratio: dec!(0.3),
            new_highs_lows_ratio: dec!(0.1),
            margin_debt_change_pct: dec!(-0.15),
            fund_flows: dec!(-5000000),
            short_interest_ratio: dec!(8.0),
            insider_buy_sell_ratio: dec!(0.3),
            consumer_confidence: dec!(20),
            risk_appetite_indicators: vec![],
            contrarian_mode: true,
        };
        let result = analyze_sentiment(&input).unwrap();

        assert!(result.composite_score < dec!(25));
        assert!(result.sentiment_label == "Fear" || result.sentiment_label == "Extreme Fear");
        // Contrarian should say buy
        assert!(result.contrarian_signal == "Strong Buy" || result.contrarian_signal == "Buy");
    }

    #[test]
    fn test_extreme_greed_scenario() {
        let input = SentimentInput {
            market_name: "NASDAQ".to_string(),
            vix_current: dec!(10),
            vix_sma_50: dec!(15),
            put_call_ratio: dec!(0.50),
            put_call_sma_20: dec!(0.80),
            advance_decline_ratio: dec!(2.5),
            new_highs_lows_ratio: dec!(3.0),
            margin_debt_change_pct: dec!(0.15),
            fund_flows: dec!(10000000),
            short_interest_ratio: dec!(1.0),
            insider_buy_sell_ratio: dec!(2.5),
            consumer_confidence: dec!(90),
            risk_appetite_indicators: vec![],
            contrarian_mode: true,
        };
        let result = analyze_sentiment(&input).unwrap();

        assert!(result.composite_score > dec!(70));
        assert!(result.sentiment_label == "Greed" || result.sentiment_label == "Extreme Greed");
        // Contrarian should say sell
        assert!(result.contrarian_signal == "Strong Sell" || result.contrarian_signal == "Sell");
    }

    #[test]
    fn test_neutral_scenario() {
        let input = SentimentInput {
            market_name: "S&P 500".to_string(),
            vix_current: dec!(18),
            vix_sma_50: dec!(18),
            put_call_ratio: dec!(0.80),
            put_call_sma_20: dec!(0.80),
            advance_decline_ratio: dec!(1.0),
            new_highs_lows_ratio: dec!(1.0),
            margin_debt_change_pct: dec!(0.0),
            fund_flows: dec!(0),
            short_interest_ratio: dec!(3.0),
            insider_buy_sell_ratio: dec!(1.0),
            consumer_confidence: dec!(50),
            risk_appetite_indicators: vec![],
            contrarian_mode: false,
        };
        let result = analyze_sentiment(&input).unwrap();

        // Should be somewhere around neutral
        assert!(result.composite_score > dec!(30));
        assert!(result.composite_score < dec!(70));
    }

    #[test]
    fn test_volatility_regime_low() {
        let mut input = default_input();
        input.vix_current = dec!(12);
        let result = analyze_sentiment(&input).unwrap();
        assert_eq!(result.volatility_regime, "Low Vol");
    }

    #[test]
    fn test_volatility_regime_normal() {
        let mut input = default_input();
        input.vix_current = dec!(20);
        let result = analyze_sentiment(&input).unwrap();
        assert_eq!(result.volatility_regime, "Normal");
    }

    #[test]
    fn test_volatility_regime_elevated() {
        let mut input = default_input();
        input.vix_current = dec!(30);
        let result = analyze_sentiment(&input).unwrap();
        assert_eq!(result.volatility_regime, "Elevated");
    }

    #[test]
    fn test_volatility_regime_crisis() {
        let mut input = default_input();
        input.vix_current = dec!(40);
        let result = analyze_sentiment(&input).unwrap();
        assert_eq!(result.volatility_regime, "Crisis");
    }

    #[test]
    fn test_market_breadth_strong() {
        let mut input = default_input();
        input.advance_decline_ratio = dec!(2.0);
        input.new_highs_lows_ratio = dec!(2.5);
        let result = analyze_sentiment(&input).unwrap();
        assert_eq!(result.market_breadth_signal, "Strong");
    }

    #[test]
    fn test_market_breadth_deteriorating() {
        let mut input = default_input();
        input.advance_decline_ratio = dec!(0.2);
        input.new_highs_lows_ratio = dec!(0.1);
        let result = analyze_sentiment(&input).unwrap();
        assert_eq!(result.market_breadth_signal, "Deteriorating");
    }

    #[test]
    fn test_smart_money_bullish() {
        let mut input = default_input();
        input.insider_buy_sell_ratio = dec!(2.0);
        input.short_interest_ratio = dec!(1.0);
        let result = analyze_sentiment(&input).unwrap();
        assert_eq!(result.smart_money_signal, "Bullish");
    }

    #[test]
    fn test_smart_money_bearish() {
        let mut input = default_input();
        input.insider_buy_sell_ratio = dec!(0.3);
        input.short_interest_ratio = dec!(8.0);
        let result = analyze_sentiment(&input).unwrap();
        assert_eq!(result.smart_money_signal, "Bearish");
    }

    #[test]
    fn test_flow_momentum_strong_inflow() {
        let mut input = default_input();
        input.fund_flows = dec!(10000000);
        let result = analyze_sentiment(&input).unwrap();
        assert!(result.flow_momentum == "Strong Inflow" || result.flow_momentum == "Inflow");
    }

    #[test]
    fn test_flow_momentum_strong_outflow() {
        let mut input = default_input();
        input.fund_flows = dec!(-10000000);
        let result = analyze_sentiment(&input).unwrap();
        assert!(result.flow_momentum == "Strong Outflow" || result.flow_momentum == "Outflow");
    }

    #[test]
    fn test_risk_on_off() {
        let input = default_input();
        let result = analyze_sentiment(&input).unwrap();

        if result.composite_score >= dec!(50) {
            assert_eq!(result.risk_on_off, "Risk On");
        } else {
            assert_eq!(result.risk_on_off, "Risk Off");
        }
    }

    #[test]
    fn test_contrarian_mode_disabled() {
        let mut input = default_input();
        input.contrarian_mode = false;
        let result = analyze_sentiment(&input).unwrap();
        assert!(result.contrarian_signal.contains("disabled"));
    }

    #[test]
    fn test_contrarian_mode_enabled() {
        let mut input = default_input();
        input.contrarian_mode = true;
        let result = analyze_sentiment(&input).unwrap();
        assert!(!result.contrarian_signal.contains("disabled"));
    }

    #[test]
    fn test_historical_context_not_empty() {
        let input = default_input();
        let result = analyze_sentiment(&input).unwrap();
        assert!(!result.historical_context.is_empty());
    }

    #[test]
    fn test_indicator_scores_count() {
        let input = default_input();
        let result = analyze_sentiment(&input).unwrap();

        // 9 base indicators (VIX, P/C, A/D, H/L, Flows, Margin, Short, Insider, Confidence)
        assert_eq!(result.indicator_scores.len(), 9);
    }

    #[test]
    fn test_custom_risk_indicators() {
        let mut input = default_input();
        input.risk_appetite_indicators = vec![
            RiskIndicator {
                name: "Credit Spreads".to_string(),
                value: dec!(1.5),
                bullish_threshold: dec!(3.0),
                bearish_threshold: dec!(1.0),
                weight: dec!(0.05),
            },
            RiskIndicator {
                name: "Copper/Gold".to_string(),
                value: dec!(0.8),
                bullish_threshold: dec!(1.0),
                bearish_threshold: dec!(0.5),
                weight: dec!(0.05),
            },
        ];
        let result = analyze_sentiment(&input).unwrap();

        // 9 base + 2 custom
        assert_eq!(result.indicator_scores.len(), 11);
    }

    #[test]
    fn test_fear_greed_decomposition() {
        let input = default_input();
        let result = analyze_sentiment(&input).unwrap();

        let fgb = &result.fear_greed_decomposition;
        assert!(fgb.volatility_component >= Decimal::ZERO);
        assert!(fgb.volatility_component <= dec!(100));
        assert!(fgb.options_component >= Decimal::ZERO);
        assert!(fgb.options_component <= dec!(100));
        assert!(fgb.breadth_component >= Decimal::ZERO);
        assert!(fgb.breadth_component <= dec!(100));
        assert!(fgb.momentum_component >= Decimal::ZERO);
        assert!(fgb.momentum_component <= dec!(100));
        assert!(fgb.flow_component >= Decimal::ZERO);
        assert!(fgb.flow_component <= dec!(100));
        assert!(fgb.leverage_component >= Decimal::ZERO);
        assert!(fgb.leverage_component <= dec!(100));
    }

    #[test]
    fn test_invalid_empty_market_name() {
        let mut input = default_input();
        input.market_name = String::new();
        assert!(analyze_sentiment(&input).is_err());
    }

    #[test]
    fn test_invalid_negative_vix() {
        let mut input = default_input();
        input.vix_current = dec!(-5);
        assert!(analyze_sentiment(&input).is_err());
    }

    #[test]
    fn test_invalid_negative_vix_sma() {
        let mut input = default_input();
        input.vix_sma_50 = dec!(-1);
        assert!(analyze_sentiment(&input).is_err());
    }

    #[test]
    fn test_invalid_negative_put_call() {
        let mut input = default_input();
        input.put_call_ratio = dec!(-0.5);
        assert!(analyze_sentiment(&input).is_err());
    }

    #[test]
    fn test_normalize_vix_high() {
        // VIX much higher than SMA = fear = low score
        let score = normalize_vix(dec!(40), dec!(18));
        assert!(score < dec!(30));
    }

    #[test]
    fn test_normalize_vix_low() {
        // VIX much lower than SMA = greed = high score
        let score = normalize_vix(dec!(12), dec!(18));
        assert!(score > dec!(60));
    }

    #[test]
    fn test_normalize_vix_zero_sma() {
        let score = normalize_vix(dec!(20), Decimal::ZERO);
        assert_eq!(score, dec!(50));
    }

    #[test]
    fn test_normalize_put_call_high() {
        // High P/C ratio = fear = low score
        let score = normalize_put_call(dec!(1.3), dec!(0.80));
        assert!(score < dec!(30));
    }

    #[test]
    fn test_normalize_fund_flows_positive() {
        let score = normalize_fund_flows(dec!(2000000));
        assert!(score > dec!(50));
    }

    #[test]
    fn test_normalize_fund_flows_negative() {
        let score = normalize_fund_flows(dec!(-2000000));
        assert!(score < dec!(50));
    }

    #[test]
    fn test_normalize_fund_flows_zero() {
        let score = normalize_fund_flows(Decimal::ZERO);
        assert_eq!(score, dec!(50));
    }

    #[test]
    fn test_sentiment_label_extreme_fear() {
        let label = signal_from_score(dec!(10));
        assert_eq!(label, "Extreme Fear");
    }

    #[test]
    fn test_sentiment_label_extreme_greed() {
        let label = signal_from_score(dec!(90));
        assert_eq!(label, "Extreme Greed");
    }

    #[test]
    fn test_contrarian_strong_buy() {
        let signal = contrarian_signal(dec!(10));
        assert_eq!(signal, "Strong Buy");
    }

    #[test]
    fn test_contrarian_strong_sell() {
        let signal = contrarian_signal(dec!(90));
        assert_eq!(signal, "Strong Sell");
    }
}
