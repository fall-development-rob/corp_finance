use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Decimal math helpers (pure Decimal, no f64)
// ---------------------------------------------------------------------------

/// Newton's method square root (20 iterations).
fn sqrt_decimal(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    if x == Decimal::ONE {
        return Decimal::ONE;
    }
    let two = Decimal::from(2);
    let mut guess = x / two;
    for _ in 0..20 {
        if guess.is_zero() {
            break;
        }
        guess = (guess + x / guess) / two;
    }
    guess
}

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// Type of commodity spread.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpreadType {
    /// Oil refining margin (crude -> gasoline + distillates).
    Crack,
    /// Soybean processing margin (soybeans -> meal + oil).
    Crush,
    /// Power generation margin (gas -> electricity).
    Spark,
    /// Same commodity, different delivery months.
    Calendar,
    /// Same commodity, different delivery locations.
    Location,
    /// Different grades of same commodity.
    Quality,
}

// ---------------------------------------------------------------------------
// Input / Output structs
// ---------------------------------------------------------------------------

/// A single commodity price observation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommodityPrice {
    /// Name of the commodity or product (e.g. "WTI Crude", "RBOB Gasoline").
    pub name: String,
    /// Price per unit.
    pub price: Decimal,
    /// Unit of measure (e.g. "barrel", "bushel", "MWh").
    pub unit: String,
    /// Volume in the spread ratio.
    pub volume: Decimal,
}

/// Input for commodity spread analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommoditySpreadInput {
    /// Type of spread to analyse.
    pub spread_type: SpreadType,
    /// Input commodity prices (e.g. crude oil, soybeans, natural gas).
    pub input_prices: Vec<CommodityPrice>,
    /// Output / product prices (e.g. gasoline, soybean meal, electricity).
    pub output_prices: Vec<CommodityPrice>,
    /// Conversion ratios: units of each output per unit of input.
    pub conversion_ratios: Vec<Decimal>,
    /// Variable processing cost per unit of input (optional).
    pub processing_cost: Option<Decimal>,
    /// Fixed costs per period (optional).
    pub fixed_costs: Option<Decimal>,
    /// Capacity utilisation rate 0-1 (optional).
    pub capacity_utilization: Option<Decimal>,
    /// Heat rate for spark spread: MMBtu per MWh (optional).
    pub heat_rate: Option<Decimal>,
    /// Carbon price per ton of CO2 (optional).
    pub carbon_price: Option<Decimal>,
    /// Emission factor: tons CO2 per unit of output (optional).
    pub emission_factor: Option<Decimal>,
    /// Historical spread values for percentile / VaR analysis (optional).
    pub historical_spreads: Option<Vec<Decimal>>,
}

/// A component in the spread breakdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpreadComponent {
    /// Label for this component.
    pub name: String,
    /// Dollar value of this component.
    pub value: Decimal,
    /// Percentage of gross spread.
    pub pct_of_gross: Decimal,
}

/// Risk metrics derived from historical spread data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpreadRiskMetrics {
    /// 95% Value at Risk (5th percentile of historical spreads).
    pub var_95: Option<Decimal>,
    /// Expected shortfall (average of spreads below VaR).
    pub expected_shortfall: Option<Decimal>,
    /// Maximum historical loss.
    pub max_historical_loss: Option<Decimal>,
}

/// Output from commodity spread analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommoditySpreadOutput {
    /// Gross spread: total output value minus total input cost.
    pub gross_spread: Decimal,
    /// Net spread after processing and fixed costs.
    pub net_spread: Decimal,
    /// Net spread per unit of input commodity.
    pub spread_per_unit: Decimal,
    /// Net margin as a percentage of input cost.
    pub margin_pct: Decimal,
    /// Input price at which the net spread equals zero.
    pub breakeven_input_price: Decimal,
    /// Net spread after carbon costs (if carbon_price and emission_factor given).
    pub carbon_adjusted_spread: Option<Decimal>,
    /// Breakdown of spread components.
    pub spread_components: Vec<SpreadComponent>,
    /// Percentile rank of current spread in historical distribution.
    pub historical_percentile: Option<Decimal>,
    /// Standard deviation of historical spreads.
    pub spread_volatility: Option<Decimal>,
    /// Risk metrics from historical data.
    pub risk_metrics: SpreadRiskMetrics,
}

// ---------------------------------------------------------------------------
// Core function
// ---------------------------------------------------------------------------

/// Analyse a commodity spread (crack, crush, spark, calendar, location, or quality).
///
/// # Spread Calculations
///
/// - **Crack** (oil refining): output_value - input_value, where
///   output_value = sum(product_price * conversion_ratio * volume).
///   Classic 3:2:1 = (2*gasoline + 1*heating_oil - 3*crude) / 3
///
/// - **Crush** (soybeans): meal_value + oil_value - soybean_cost.
///   Standard: 44 lbs meal + 11 lbs oil per bushel.
///
/// - **Spark**: power_price - gas_price * heat_rate - carbon_cost * emission_factor.
///   Clean spark = spark spread minus carbon cost.
///
/// - **Calendar**: near_month_price - far_month_price.
///   Positive = backwardation, negative = contango.
///
/// - **Location**: same commodity at different delivery points.
///
/// - **Quality**: different grades of same commodity.
pub fn analyze_commodity_spread(
    input: &CommoditySpreadInput,
) -> CorpFinanceResult<CommoditySpreadOutput> {
    // -- Validation --
    if input.input_prices.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one input price is required".into(),
        ));
    }
    if input.output_prices.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one output price is required".into(),
        ));
    }
    if input.conversion_ratios.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one conversion ratio is required".into(),
        ));
    }
    if input.output_prices.len() != input.conversion_ratios.len() {
        return Err(CorpFinanceError::InvalidInput {
            field: "conversion_ratios".into(),
            reason: format!(
                "Number of conversion ratios ({}) must match number of output prices ({})",
                input.conversion_ratios.len(),
                input.output_prices.len()
            ),
        });
    }
    if let Some(util) = input.capacity_utilization {
        if util < Decimal::ZERO || util > Decimal::ONE {
            return Err(CorpFinanceError::InvalidInput {
                field: "capacity_utilization".into(),
                reason: "Capacity utilization must be between 0 and 1".into(),
            });
        }
    }
    if let Some(hr) = input.heat_rate {
        if hr <= Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "heat_rate".into(),
                reason: "Heat rate must be positive".into(),
            });
        }
    }

    for ip in &input.input_prices {
        if ip.volume <= Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "input_prices.volume".into(),
                reason: format!("Volume for '{}' must be positive", ip.name),
            });
        }
    }
    for op in &input.output_prices {
        if op.volume <= Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "output_prices.volume".into(),
                reason: format!("Volume for '{}' must be positive", op.name),
            });
        }
    }

    // -- Compute input cost --
    let mut input_cost = Decimal::ZERO;
    let mut total_input_volume = Decimal::ZERO;
    for ip in &input.input_prices {
        input_cost += ip.price * ip.volume;
        total_input_volume += ip.volume;
    }

    // -- Compute output value --
    let mut output_value = Decimal::ZERO;
    let mut components: Vec<SpreadComponent> = Vec::new();

    match input.spread_type {
        SpreadType::Spark => {
            // Spark spread: power_price - gas_price * heat_rate
            let heat_rate = input.heat_rate.unwrap_or(Decimal::ONE);
            if let Some(power) = input.output_prices.first() {
                output_value = power.price * power.volume;
            }
            // Gas cost is input cost adjusted by heat rate
            // For spark: gas_cost = gas_price * heat_rate * volume
            // We recalculate input_cost with heat_rate applied
            let gas_cost = if let Some(gas) = input.input_prices.first() {
                gas.price * heat_rate * gas.volume
            } else {
                Decimal::ZERO
            };
            // Override input_cost for spark spread calculation
            input_cost = gas_cost;

            components.push(SpreadComponent {
                name: "Power Revenue".into(),
                value: output_value,
                pct_of_gross: Decimal::ZERO, // filled later
            });
            components.push(SpreadComponent {
                name: "Fuel Cost".into(),
                value: -gas_cost,
                pct_of_gross: Decimal::ZERO,
            });
        }
        _ => {
            // Standard spread: output_value = sum(price * ratio * input_volume)
            for (i, op) in input.output_prices.iter().enumerate() {
                let ratio = input.conversion_ratios[i];
                let val = op.price * ratio * total_input_volume;
                output_value += val;
                components.push(SpreadComponent {
                    name: op.name.clone(),
                    value: val,
                    pct_of_gross: Decimal::ZERO,
                });
            }
            components.push(SpreadComponent {
                name: "Input Cost".into(),
                value: -input_cost,
                pct_of_gross: Decimal::ZERO,
            });
        }
    }

    // -- Gross spread --
    let gross_spread = output_value - input_cost;

    // -- Processing costs --
    let proc_cost = input.processing_cost.unwrap_or(Decimal::ZERO) * total_input_volume;
    let fixed = input.fixed_costs.unwrap_or(Decimal::ZERO);

    // -- Capacity utilization adjustment --
    let util = input.capacity_utilization.unwrap_or(Decimal::ONE);
    let adjusted_fixed = if util > Decimal::ZERO {
        fixed / util
    } else {
        fixed
    };

    let total_costs = proc_cost + adjusted_fixed;

    if total_costs > Decimal::ZERO {
        components.push(SpreadComponent {
            name: "Processing Costs".into(),
            value: -proc_cost,
            pct_of_gross: Decimal::ZERO,
        });
        if adjusted_fixed > Decimal::ZERO {
            components.push(SpreadComponent {
                name: "Fixed Costs".into(),
                value: -adjusted_fixed,
                pct_of_gross: Decimal::ZERO,
            });
        }
    }

    // -- Net spread --
    let net_spread = gross_spread - total_costs;

    // -- Carbon adjustment --
    let carbon_adjusted_spread = match (input.carbon_price, input.emission_factor) {
        (Some(cp), Some(ef)) => {
            let carbon_cost = cp * ef * total_input_volume;
            components.push(SpreadComponent {
                name: "Carbon Cost".into(),
                value: -carbon_cost,
                pct_of_gross: Decimal::ZERO,
            });
            Some(net_spread - carbon_cost)
        }
        _ => None,
    };

    // -- Fill pct_of_gross for components --
    if gross_spread != Decimal::ZERO {
        for comp in &mut components {
            comp.pct_of_gross = comp.value / gross_spread;
        }
    }

    // -- Spread per unit --
    let spread_per_unit = if total_input_volume > Decimal::ZERO {
        net_spread / total_input_volume
    } else {
        Decimal::ZERO
    };

    // -- Margin percentage --
    let margin_pct = if input_cost > Decimal::ZERO {
        net_spread / input_cost
    } else {
        Decimal::ZERO
    };

    // -- Breakeven input price --
    // At breakeven: output_value - (breakeven_price * total_input_volume) - total_costs = 0
    // breakeven_price = (output_value - total_costs) / total_input_volume
    let breakeven_input_price = if total_input_volume > Decimal::ZERO {
        match input.spread_type {
            SpreadType::Spark => {
                // For spark: output - gas_price_be * heat_rate * volume - costs = 0
                // gas_price_be = (output - costs) / (heat_rate * volume)
                let heat_rate = input.heat_rate.unwrap_or(Decimal::ONE);
                let gas_vol = input
                    .input_prices
                    .first()
                    .map(|g| g.volume)
                    .unwrap_or(Decimal::ONE);
                let denom = heat_rate * gas_vol;
                if denom > Decimal::ZERO {
                    (output_value - total_costs) / denom
                } else {
                    Decimal::ZERO
                }
            }
            _ => (output_value - total_costs) / total_input_volume,
        }
    } else {
        Decimal::ZERO
    };

    // -- Historical analysis --
    let (historical_percentile, spread_volatility, risk_metrics) =
        compute_historical_metrics(input, net_spread);

    Ok(CommoditySpreadOutput {
        gross_spread,
        net_spread,
        spread_per_unit,
        margin_pct,
        breakeven_input_price,
        carbon_adjusted_spread,
        spread_components: components,
        historical_percentile,
        spread_volatility,
        risk_metrics,
    })
}

/// Compute historical percentile, volatility, and risk metrics from historical spreads.
fn compute_historical_metrics(
    input: &CommoditySpreadInput,
    current_spread: Decimal,
) -> (Option<Decimal>, Option<Decimal>, SpreadRiskMetrics) {
    let empty_metrics = SpreadRiskMetrics {
        var_95: None,
        expected_shortfall: None,
        max_historical_loss: None,
    };

    let hist = match &input.historical_spreads {
        Some(h) if !h.is_empty() => h,
        _ => return (None, None, empty_metrics),
    };

    let n = Decimal::from(hist.len() as u64);

    // -- Percentile: fraction of historical spreads <= current --
    let count_below = hist.iter().filter(|&&s| s <= current_spread).count();
    let percentile = Decimal::from(count_below as u64) / n * Decimal::from(100);

    // -- Volatility (standard deviation) --
    let mean = hist.iter().copied().sum::<Decimal>() / n;
    let variance = hist
        .iter()
        .map(|s| {
            let diff = *s - mean;
            diff * diff
        })
        .sum::<Decimal>()
        / n;
    let vol = sqrt_decimal(variance);

    // -- Risk metrics: sort ascending for VaR --
    let mut sorted = hist.clone();
    sorted.sort();

    // VaR at 95%: 5th percentile (worst 5%)
    let var_idx = (hist.len() as f64 * 0.05).floor() as usize;
    let var_95 = if var_idx < sorted.len() {
        Some(sorted[var_idx])
    } else {
        sorted.first().copied()
    };

    // Expected shortfall: mean of values at or below VaR
    let es = if let Some(var_val) = var_95 {
        let tail: Vec<Decimal> = sorted.iter().copied().filter(|&s| s <= var_val).collect();
        if !tail.is_empty() {
            let tail_n = Decimal::from(tail.len() as u64);
            Some(tail.iter().copied().sum::<Decimal>() / tail_n)
        } else {
            Some(var_val)
        }
    } else {
        None
    };

    // Max historical loss
    let max_loss = sorted.first().copied();

    let metrics = SpreadRiskMetrics {
        var_95,
        expected_shortfall: es,
        max_historical_loss: max_loss,
    };

    (Some(percentile), Some(vol), metrics)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn tol() -> Decimal {
        dec!(0.01)
    }

    fn assert_approx(actual: Decimal, expected: Decimal, tolerance: Decimal, label: &str) {
        let diff = (actual - expected).abs();
        assert!(
            diff <= tolerance,
            "{label}: expected ~{expected}, got {actual} (diff={diff}, tol={tolerance})"
        );
    }

    // Helper: build a basic crack spread input (3:2:1)
    fn crack_321_input() -> CommoditySpreadInput {
        CommoditySpreadInput {
            spread_type: SpreadType::Crack,
            input_prices: vec![CommodityPrice {
                name: "WTI Crude".into(),
                price: dec!(80),
                unit: "barrel".into(),
                volume: dec!(3),
            }],
            output_prices: vec![
                CommodityPrice {
                    name: "RBOB Gasoline".into(),
                    price: dec!(100),
                    unit: "barrel".into(),
                    volume: dec!(2),
                },
                CommodityPrice {
                    name: "Heating Oil".into(),
                    price: dec!(95),
                    unit: "barrel".into(),
                    volume: dec!(1),
                },
            ],
            conversion_ratios: vec![dec!(1), dec!(1)],
            processing_cost: None,
            fixed_costs: None,
            capacity_utilization: None,
            heat_rate: None,
            carbon_price: None,
            emission_factor: None,
            historical_spreads: None,
        }
    }

    // -----------------------------------------------------------------------
    // 1. 3:2:1 crack spread basic
    // -----------------------------------------------------------------------
    #[test]
    fn test_crack_spread_321_basic() {
        // Input: 3 bbl crude @ $80 = $240
        // Output: 2 bbl gasoline @ $100 * 1 * 3 = $600, 1 bbl heating oil @ $95 * 1 * 3 = $285
        // Actually: output_value = sum(price * ratio * total_input_volume)
        // = 100*1*3 + 95*1*3 = 300 + 285 = 585
        // gross = 585 - 240 = 345
        // spread per unit = 345 / 3 = 115
        let input = crack_321_input();
        let result = analyze_commodity_spread(&input).unwrap();

        assert_approx(result.gross_spread, dec!(345), tol(), "crack gross");
        assert_approx(result.net_spread, dec!(345), tol(), "crack net (no costs)");
        assert_approx(result.spread_per_unit, dec!(115), tol(), "crack per unit");
    }

    // -----------------------------------------------------------------------
    // 2. Crack spread with processing cost
    // -----------------------------------------------------------------------
    #[test]
    fn test_crack_spread_with_processing_cost() {
        let mut input = crack_321_input();
        input.processing_cost = Some(dec!(5)); // $5 per barrel input
        let result = analyze_commodity_spread(&input).unwrap();

        // proc cost = 5 * 3 = 15
        // net = 345 - 15 = 330
        assert_approx(result.gross_spread, dec!(345), tol(), "crack gross");
        assert_approx(result.net_spread, dec!(330), tol(), "crack net w/ proc");
    }

    // -----------------------------------------------------------------------
    // 3. Crack spread with fixed costs
    // -----------------------------------------------------------------------
    #[test]
    fn test_crack_spread_with_fixed_costs() {
        let mut input = crack_321_input();
        input.fixed_costs = Some(dec!(30));
        let result = analyze_commodity_spread(&input).unwrap();

        // net = 345 - 30 = 315
        assert_approx(result.net_spread, dec!(315), tol(), "crack net w/ fixed");
    }

    // -----------------------------------------------------------------------
    // 4. Crack spread with capacity utilization
    // -----------------------------------------------------------------------
    #[test]
    fn test_crack_spread_capacity_utilization() {
        let mut input = crack_321_input();
        input.fixed_costs = Some(dec!(30));
        input.capacity_utilization = Some(dec!(0.75));
        let result = analyze_commodity_spread(&input).unwrap();

        // adjusted_fixed = 30 / 0.75 = 40
        // net = 345 - 40 = 305
        assert_approx(result.net_spread, dec!(305), tol(), "crack w/ util");
    }

    // -----------------------------------------------------------------------
    // 5. Soybean crush spread
    // -----------------------------------------------------------------------
    #[test]
    fn test_crush_spread_basic() {
        // Standard crush: 1 bushel soybeans -> 44 lbs meal + 11 lbs oil
        // Soybean: $14/bushel, Meal: $0.35/lb, Oil: $0.55/lb
        // conversion_ratios: 44 lbs meal per bushel, 11 lbs oil per bushel
        // output_value = 0.35 * 44 * 1 + 0.55 * 11 * 1 = 15.40 + 6.05 = 21.45
        // input_cost = 14 * 1 = 14
        // gross = 21.45 - 14 = 7.45
        let input = CommoditySpreadInput {
            spread_type: SpreadType::Crush,
            input_prices: vec![CommodityPrice {
                name: "Soybeans".into(),
                price: dec!(14),
                unit: "bushel".into(),
                volume: dec!(1),
            }],
            output_prices: vec![
                CommodityPrice {
                    name: "Soybean Meal".into(),
                    price: dec!(0.35),
                    unit: "lb".into(),
                    volume: dec!(44),
                },
                CommodityPrice {
                    name: "Soybean Oil".into(),
                    price: dec!(0.55),
                    unit: "lb".into(),
                    volume: dec!(11),
                },
            ],
            conversion_ratios: vec![dec!(44), dec!(11)],
            processing_cost: None,
            fixed_costs: None,
            capacity_utilization: None,
            heat_rate: None,
            carbon_price: None,
            emission_factor: None,
            historical_spreads: None,
        };
        let result = analyze_commodity_spread(&input).unwrap();

        // output = 0.35*44*1 + 0.55*11*1 = 15.40 + 6.05 = 21.45
        // gross = 21.45 - 14 = 7.45
        assert_approx(result.gross_spread, dec!(7.45), tol(), "crush gross");
        assert_approx(
            result.margin_pct,
            dec!(7.45) / dec!(14),
            tol(),
            "crush margin",
        );
    }

    // -----------------------------------------------------------------------
    // 6. Spark spread basic (no carbon)
    // -----------------------------------------------------------------------
    #[test]
    fn test_spark_spread_basic() {
        // Power: $50/MWh, Gas: $4/MMBtu, Heat rate: 7 MMBtu/MWh
        // Spark = 50 - 4*7 = 50 - 28 = 22
        let input = CommoditySpreadInput {
            spread_type: SpreadType::Spark,
            input_prices: vec![CommodityPrice {
                name: "Natural Gas".into(),
                price: dec!(4),
                unit: "MMBtu".into(),
                volume: dec!(1),
            }],
            output_prices: vec![CommodityPrice {
                name: "Electricity".into(),
                price: dec!(50),
                unit: "MWh".into(),
                volume: dec!(1),
            }],
            conversion_ratios: vec![dec!(1)],
            heat_rate: Some(dec!(7)),
            processing_cost: None,
            fixed_costs: None,
            capacity_utilization: None,
            carbon_price: None,
            emission_factor: None,
            historical_spreads: None,
        };
        let result = analyze_commodity_spread(&input).unwrap();

        // output_value = 50*1 = 50
        // input_cost = 4*7*1 = 28
        // gross = 50 - 28 = 22
        assert_approx(result.gross_spread, dec!(22), tol(), "spark gross");
        assert_approx(result.net_spread, dec!(22), tol(), "spark net");
    }

    // -----------------------------------------------------------------------
    // 7. Spark spread with carbon cost (clean spark)
    // -----------------------------------------------------------------------
    #[test]
    fn test_spark_spread_with_carbon() {
        // Spark = 22 (same as above)
        // Carbon: $30/ton CO2, 0.5 tons/MWh
        // Carbon cost = 30 * 0.5 * 1 = 15
        // Clean spark = 22 - 15 = 7
        let input = CommoditySpreadInput {
            spread_type: SpreadType::Spark,
            input_prices: vec![CommodityPrice {
                name: "Natural Gas".into(),
                price: dec!(4),
                unit: "MMBtu".into(),
                volume: dec!(1),
            }],
            output_prices: vec![CommodityPrice {
                name: "Electricity".into(),
                price: dec!(50),
                unit: "MWh".into(),
                volume: dec!(1),
            }],
            conversion_ratios: vec![dec!(1)],
            heat_rate: Some(dec!(7)),
            processing_cost: None,
            fixed_costs: None,
            capacity_utilization: None,
            carbon_price: Some(dec!(30)),
            emission_factor: Some(dec!(0.5)),
            historical_spreads: None,
        };
        let result = analyze_commodity_spread(&input).unwrap();

        assert_approx(result.gross_spread, dec!(22), tol(), "spark gross");
        assert!(result.carbon_adjusted_spread.is_some());
        assert_approx(
            result.carbon_adjusted_spread.unwrap(),
            dec!(7),
            tol(),
            "clean spark",
        );
    }

    // -----------------------------------------------------------------------
    // 8. Calendar spread: contango
    // -----------------------------------------------------------------------
    #[test]
    fn test_calendar_spread_contango() {
        // Near month $80, far month $84
        // Calendar spread = near - far output: use near as input, far as output
        // For calendar: input = near month, output = far month
        // gross_spread = output_value - input_cost
        // Here: near is what we sell (input to the spread model as "input"),
        //   far is what we buy (modeled as "output")
        // Actually for calendar: we buy near, sell far. Input = near price, output = far price.
        // gross = far * ratio * vol - near * vol
        let input = CommoditySpreadInput {
            spread_type: SpreadType::Calendar,
            input_prices: vec![CommodityPrice {
                name: "WTI Near Month".into(),
                price: dec!(80),
                unit: "barrel".into(),
                volume: dec!(1),
            }],
            output_prices: vec![CommodityPrice {
                name: "WTI Far Month".into(),
                price: dec!(84),
                unit: "barrel".into(),
                volume: dec!(1),
            }],
            conversion_ratios: vec![dec!(1)],
            processing_cost: None,
            fixed_costs: None,
            capacity_utilization: None,
            heat_rate: None,
            carbon_price: None,
            emission_factor: None,
            historical_spreads: None,
        };
        let result = analyze_commodity_spread(&input).unwrap();

        // output = 84*1*1 = 84, input = 80*1 = 80
        // gross = 84 - 80 = 4
        assert_approx(result.gross_spread, dec!(4), tol(), "calendar contango");
        assert!(result.gross_spread > Decimal::ZERO);
    }

    // -----------------------------------------------------------------------
    // 9. Calendar spread: backwardation
    // -----------------------------------------------------------------------
    #[test]
    fn test_calendar_spread_backwardation() {
        let input = CommoditySpreadInput {
            spread_type: SpreadType::Calendar,
            input_prices: vec![CommodityPrice {
                name: "WTI Near Month".into(),
                price: dec!(84),
                unit: "barrel".into(),
                volume: dec!(1),
            }],
            output_prices: vec![CommodityPrice {
                name: "WTI Far Month".into(),
                price: dec!(80),
                unit: "barrel".into(),
                volume: dec!(1),
            }],
            conversion_ratios: vec![dec!(1)],
            processing_cost: None,
            fixed_costs: None,
            capacity_utilization: None,
            heat_rate: None,
            carbon_price: None,
            emission_factor: None,
            historical_spreads: None,
        };
        let result = analyze_commodity_spread(&input).unwrap();

        // gross = 80 - 84 = -4
        assert_approx(
            result.gross_spread,
            dec!(-4),
            tol(),
            "calendar backwardation",
        );
        assert!(result.gross_spread < Decimal::ZERO);
    }

    // -----------------------------------------------------------------------
    // 10. Location spread
    // -----------------------------------------------------------------------
    #[test]
    fn test_location_spread() {
        // WTI Cushing $78, Brent $82
        let input = CommoditySpreadInput {
            spread_type: SpreadType::Location,
            input_prices: vec![CommodityPrice {
                name: "WTI Cushing".into(),
                price: dec!(78),
                unit: "barrel".into(),
                volume: dec!(1),
            }],
            output_prices: vec![CommodityPrice {
                name: "Brent".into(),
                price: dec!(82),
                unit: "barrel".into(),
                volume: dec!(1),
            }],
            conversion_ratios: vec![dec!(1)],
            processing_cost: None,
            fixed_costs: None,
            capacity_utilization: None,
            heat_rate: None,
            carbon_price: None,
            emission_factor: None,
            historical_spreads: None,
        };
        let result = analyze_commodity_spread(&input).unwrap();

        assert_approx(result.gross_spread, dec!(4), tol(), "location spread");
    }

    // -----------------------------------------------------------------------
    // 11. Quality spread
    // -----------------------------------------------------------------------
    #[test]
    fn test_quality_spread() {
        // Light sweet vs heavy sour
        let input = CommoditySpreadInput {
            spread_type: SpreadType::Quality,
            input_prices: vec![CommodityPrice {
                name: "Heavy Sour".into(),
                price: dec!(72),
                unit: "barrel".into(),
                volume: dec!(1),
            }],
            output_prices: vec![CommodityPrice {
                name: "Light Sweet".into(),
                price: dec!(80),
                unit: "barrel".into(),
                volume: dec!(1),
            }],
            conversion_ratios: vec![dec!(1)],
            processing_cost: None,
            fixed_costs: None,
            capacity_utilization: None,
            heat_rate: None,
            carbon_price: None,
            emission_factor: None,
            historical_spreads: None,
        };
        let result = analyze_commodity_spread(&input).unwrap();

        assert_approx(result.gross_spread, dec!(8), tol(), "quality spread");
    }

    // -----------------------------------------------------------------------
    // 12. Historical percentile: current spread at 50th percentile
    // -----------------------------------------------------------------------
    #[test]
    fn test_historical_percentile_median() {
        let mut input = crack_321_input();
        // Current net spread is 345 (from crack_321_input).
        // Put 345 right in the middle of historical data.
        input.historical_spreads = Some(vec![
            dec!(100),
            dec!(200),
            dec!(300),
            dec!(345),
            dec!(400),
            dec!(500),
            dec!(600),
            dec!(700),
            dec!(800),
            dec!(900),
        ]);
        let result = analyze_commodity_spread(&input).unwrap();

        assert!(result.historical_percentile.is_some());
        // 4 values <= 345 out of 10 = 40%
        assert_approx(
            result.historical_percentile.unwrap(),
            dec!(40),
            tol(),
            "percentile",
        );
    }

    // -----------------------------------------------------------------------
    // 13. Historical percentile: at bottom
    // -----------------------------------------------------------------------
    #[test]
    fn test_historical_percentile_bottom() {
        let mut input = crack_321_input();
        input.historical_spreads =
            Some(vec![dec!(350), dec!(400), dec!(500), dec!(600), dec!(700)]);
        // Current net = 345, below all historical => 0%
        let result = analyze_commodity_spread(&input).unwrap();
        assert_approx(
            result.historical_percentile.unwrap(),
            dec!(0),
            tol(),
            "percentile bottom",
        );
    }

    // -----------------------------------------------------------------------
    // 14. Historical percentile: at top
    // -----------------------------------------------------------------------
    #[test]
    fn test_historical_percentile_top() {
        let mut input = crack_321_input();
        input.historical_spreads =
            Some(vec![dec!(100), dec!(200), dec!(300), dec!(340), dec!(345)]);
        // Current net = 345, all values <= 345, so 100%
        let result = analyze_commodity_spread(&input).unwrap();
        assert_approx(
            result.historical_percentile.unwrap(),
            dec!(100),
            tol(),
            "percentile top",
        );
    }

    // -----------------------------------------------------------------------
    // 15. Spread volatility
    // -----------------------------------------------------------------------
    #[test]
    fn test_spread_volatility() {
        let mut input = crack_321_input();
        // Known data: [10, 20, 30, 40, 50]
        // Mean = 30, variance = (400+100+0+100+400)/5 = 200, std = sqrt(200) ~ 14.14
        input.historical_spreads = Some(vec![dec!(10), dec!(20), dec!(30), dec!(40), dec!(50)]);
        let result = analyze_commodity_spread(&input).unwrap();

        assert!(result.spread_volatility.is_some());
        assert_approx(
            result.spread_volatility.unwrap(),
            dec!(14.14),
            dec!(0.1),
            "volatility",
        );
    }

    // -----------------------------------------------------------------------
    // 16. VaR 95 with small sample
    // -----------------------------------------------------------------------
    #[test]
    fn test_var_95_small_sample() {
        let mut input = crack_321_input();
        input.historical_spreads = Some(vec![
            dec!(-50),
            dec!(-20),
            dec!(10),
            dec!(30),
            dec!(50),
            dec!(100),
            dec!(150),
            dec!(200),
            dec!(250),
            dec!(300),
            dec!(350),
            dec!(400),
            dec!(450),
            dec!(500),
            dec!(550),
            dec!(600),
            dec!(650),
            dec!(700),
            dec!(750),
            dec!(800),
        ]);
        let result = analyze_commodity_spread(&input).unwrap();

        // 20 samples, 5th percentile index = floor(20*0.05) = 1
        // sorted: -50, -20, 10, 30, ...
        // var_95 = sorted[1] = -20
        assert!(result.risk_metrics.var_95.is_some());
        assert_approx(
            result.risk_metrics.var_95.unwrap(),
            dec!(-20),
            tol(),
            "VaR 95",
        );
    }

    // -----------------------------------------------------------------------
    // 17. Expected shortfall
    // -----------------------------------------------------------------------
    #[test]
    fn test_expected_shortfall() {
        let mut input = crack_321_input();
        input.historical_spreads = Some(vec![
            dec!(-50),
            dec!(-20),
            dec!(10),
            dec!(30),
            dec!(50),
            dec!(100),
            dec!(150),
            dec!(200),
            dec!(250),
            dec!(300),
            dec!(350),
            dec!(400),
            dec!(450),
            dec!(500),
            dec!(550),
            dec!(600),
            dec!(650),
            dec!(700),
            dec!(750),
            dec!(800),
        ]);
        let result = analyze_commodity_spread(&input).unwrap();

        // VaR = -20, values <= -20 are: -50, -20 => ES = (-50 + -20)/2 = -35
        assert!(result.risk_metrics.expected_shortfall.is_some());
        assert_approx(
            result.risk_metrics.expected_shortfall.unwrap(),
            dec!(-35),
            tol(),
            "ES",
        );
    }

    // -----------------------------------------------------------------------
    // 18. Max historical loss
    // -----------------------------------------------------------------------
    #[test]
    fn test_max_historical_loss() {
        let mut input = crack_321_input();
        input.historical_spreads = Some(vec![dec!(-100), dec!(-50), dec!(0), dec!(50), dec!(100)]);
        let result = analyze_commodity_spread(&input).unwrap();

        assert!(result.risk_metrics.max_historical_loss.is_some());
        assert_approx(
            result.risk_metrics.max_historical_loss.unwrap(),
            dec!(-100),
            tol(),
            "max loss",
        );
    }

    // -----------------------------------------------------------------------
    // 19. Breakeven input price
    // -----------------------------------------------------------------------
    #[test]
    fn test_breakeven_input_price() {
        // From crack_321: output_value = 585, input_volume = 3
        // breakeven = 585 / 3 = 195
        let input = crack_321_input();
        let result = analyze_commodity_spread(&input).unwrap();
        assert_approx(result.breakeven_input_price, dec!(195), tol(), "breakeven");
    }

    // -----------------------------------------------------------------------
    // 20. Breakeven with processing costs
    // -----------------------------------------------------------------------
    #[test]
    fn test_breakeven_with_costs() {
        let mut input = crack_321_input();
        input.processing_cost = Some(dec!(10)); // $10/bbl
        let result = analyze_commodity_spread(&input).unwrap();

        // proc_cost = 10 * 3 = 30, total_costs = 30
        // breakeven = (585 - 30) / 3 = 555 / 3 = 185
        assert_approx(
            result.breakeven_input_price,
            dec!(185),
            tol(),
            "breakeven w/ costs",
        );
    }

    // -----------------------------------------------------------------------
    // 21. Zero processing cost
    // -----------------------------------------------------------------------
    #[test]
    fn test_zero_processing_cost() {
        let mut input = crack_321_input();
        input.processing_cost = Some(Decimal::ZERO);
        let result = analyze_commodity_spread(&input).unwrap();

        // Same as no processing cost
        assert_approx(result.net_spread, dec!(345), tol(), "zero proc cost");
    }

    // -----------------------------------------------------------------------
    // 22. Margin percentage calculation
    // -----------------------------------------------------------------------
    #[test]
    fn test_margin_percentage() {
        let input = crack_321_input();
        let result = analyze_commodity_spread(&input).unwrap();

        // input_cost = 240, net_spread = 345
        // margin = 345 / 240 = 1.4375
        let expected_margin = dec!(345) / dec!(240);
        assert_approx(result.margin_pct, expected_margin, tol(), "margin pct");
    }

    // -----------------------------------------------------------------------
    // 23. Spread components present
    // -----------------------------------------------------------------------
    #[test]
    fn test_spread_components_present() {
        let input = crack_321_input();
        let result = analyze_commodity_spread(&input).unwrap();

        // Should have output components + input cost
        assert!(result.spread_components.len() >= 3);
        assert!(result
            .spread_components
            .iter()
            .any(|c| c.name == "Input Cost"));
    }

    // -----------------------------------------------------------------------
    // 24. Spark spread breakeven gas price
    // -----------------------------------------------------------------------
    #[test]
    fn test_spark_breakeven_gas_price() {
        let input = CommoditySpreadInput {
            spread_type: SpreadType::Spark,
            input_prices: vec![CommodityPrice {
                name: "Natural Gas".into(),
                price: dec!(4),
                unit: "MMBtu".into(),
                volume: dec!(1),
            }],
            output_prices: vec![CommodityPrice {
                name: "Electricity".into(),
                price: dec!(50),
                unit: "MWh".into(),
                volume: dec!(1),
            }],
            conversion_ratios: vec![dec!(1)],
            heat_rate: Some(dec!(7)),
            processing_cost: None,
            fixed_costs: None,
            capacity_utilization: None,
            carbon_price: None,
            emission_factor: None,
            historical_spreads: None,
        };
        let result = analyze_commodity_spread(&input).unwrap();

        // breakeven gas = output / (heat_rate * vol) = 50 / 7 ~ 7.14
        assert_approx(
            result.breakeven_input_price,
            dec!(50) / dec!(7),
            tol(),
            "spark breakeven gas",
        );
    }

    // -----------------------------------------------------------------------
    // 25. Validation: empty input prices
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_empty_inputs() {
        let input = CommoditySpreadInput {
            spread_type: SpreadType::Crack,
            input_prices: vec![],
            output_prices: vec![CommodityPrice {
                name: "Gas".into(),
                price: dec!(100),
                unit: "barrel".into(),
                volume: dec!(1),
            }],
            conversion_ratios: vec![dec!(1)],
            processing_cost: None,
            fixed_costs: None,
            capacity_utilization: None,
            heat_rate: None,
            carbon_price: None,
            emission_factor: None,
            historical_spreads: None,
        };
        let err = analyze_commodity_spread(&input).unwrap_err();
        match err {
            CorpFinanceError::InsufficientData(_) => {}
            e => panic!("Expected InsufficientData, got {e:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // 26. Validation: empty output prices
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_empty_outputs() {
        let input = CommoditySpreadInput {
            spread_type: SpreadType::Crack,
            input_prices: vec![CommodityPrice {
                name: "Crude".into(),
                price: dec!(80),
                unit: "barrel".into(),
                volume: dec!(1),
            }],
            output_prices: vec![],
            conversion_ratios: vec![],
            processing_cost: None,
            fixed_costs: None,
            capacity_utilization: None,
            heat_rate: None,
            carbon_price: None,
            emission_factor: None,
            historical_spreads: None,
        };
        let err = analyze_commodity_spread(&input).unwrap_err();
        match err {
            CorpFinanceError::InsufficientData(_) => {}
            e => panic!("Expected InsufficientData, got {e:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // 27. Validation: mismatched conversion ratios
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_mismatched_ratios() {
        let input = CommoditySpreadInput {
            spread_type: SpreadType::Crack,
            input_prices: vec![CommodityPrice {
                name: "Crude".into(),
                price: dec!(80),
                unit: "barrel".into(),
                volume: dec!(1),
            }],
            output_prices: vec![
                CommodityPrice {
                    name: "Gas".into(),
                    price: dec!(100),
                    unit: "barrel".into(),
                    volume: dec!(1),
                },
                CommodityPrice {
                    name: "Heating Oil".into(),
                    price: dec!(95),
                    unit: "barrel".into(),
                    volume: dec!(1),
                },
            ],
            conversion_ratios: vec![dec!(1)], // only 1 ratio for 2 outputs
            processing_cost: None,
            fixed_costs: None,
            capacity_utilization: None,
            heat_rate: None,
            carbon_price: None,
            emission_factor: None,
            historical_spreads: None,
        };
        let err = analyze_commodity_spread(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "conversion_ratios");
            }
            e => panic!("Expected InvalidInput, got {e:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // 28. Validation: capacity utilization out of range
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_capacity_utilization_range() {
        let mut input = crack_321_input();
        input.capacity_utilization = Some(dec!(1.5));
        let err = analyze_commodity_spread(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "capacity_utilization");
            }
            e => panic!("Expected InvalidInput, got {e:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // 29. Validation: negative heat rate
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_negative_heat_rate() {
        let mut input = crack_321_input();
        input.spread_type = SpreadType::Spark;
        input.heat_rate = Some(dec!(-7));
        let err = analyze_commodity_spread(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "heat_rate");
            }
            e => panic!("Expected InvalidInput, got {e:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // 30. No historical data: metrics are None
    // -----------------------------------------------------------------------
    #[test]
    fn test_no_historical_data() {
        let input = crack_321_input();
        let result = analyze_commodity_spread(&input).unwrap();

        assert!(result.historical_percentile.is_none());
        assert!(result.spread_volatility.is_none());
        assert!(result.risk_metrics.var_95.is_none());
        assert!(result.risk_metrics.expected_shortfall.is_none());
        assert!(result.risk_metrics.max_historical_loss.is_none());
    }

    // -----------------------------------------------------------------------
    // 31. Carbon adjusted spread is None without carbon data
    // -----------------------------------------------------------------------
    #[test]
    fn test_no_carbon_data() {
        let input = crack_321_input();
        let result = analyze_commodity_spread(&input).unwrap();
        assert!(result.carbon_adjusted_spread.is_none());
    }

    // -----------------------------------------------------------------------
    // 32. 100% capacity utilization is same as no utilization parameter
    // -----------------------------------------------------------------------
    #[test]
    fn test_full_utilization() {
        let mut input1 = crack_321_input();
        input1.fixed_costs = Some(dec!(30));

        let mut input2 = input1.clone();
        input2.capacity_utilization = Some(Decimal::ONE);

        let r1 = analyze_commodity_spread(&input1).unwrap();
        let r2 = analyze_commodity_spread(&input2).unwrap();

        assert_approx(r1.net_spread, r2.net_spread, tol(), "100% util = default");
    }

    // -----------------------------------------------------------------------
    // 33. VaR with large sample
    // -----------------------------------------------------------------------
    #[test]
    fn test_var_large_sample() {
        let mut input = crack_321_input();
        // Generate 100 historical values from -50 to 49
        let hist: Vec<Decimal> = (-50i64..50i64).map(Decimal::from).collect();
        input.historical_spreads = Some(hist);
        let result = analyze_commodity_spread(&input).unwrap();

        // 100 values, 5th percentile index = floor(100*0.05) = 5
        // sorted: -50, -49, ..., sorted[5] = -45
        assert!(result.risk_metrics.var_95.is_some());
        assert_approx(
            result.risk_metrics.var_95.unwrap(),
            dec!(-45),
            tol(),
            "VaR large sample",
        );
    }
}
