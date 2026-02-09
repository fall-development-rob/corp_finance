use corp_finance_core::valuation::{wacc, dcf};
use corp_finance_core::types::Currency;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

// ===========================================================================
// WACC tests
// ===========================================================================

#[test]
fn test_wacc_apple_damodaran_reference() {
    // Reference: Damodaran Apple WACC ~10%
    // Rf = 4.25%, ERP = 4.72%, Beta = 1.24, Kd = 3.4%, t = 16.23%
    // D/V = 6.37%, E/V = 93.63%
    let input = wacc::WaccInput {
        risk_free_rate: dec!(0.0425),
        equity_risk_premium: dec!(0.0472),
        beta: dec!(1.24),
        cost_of_debt: dec!(0.034),
        tax_rate: dec!(0.1623),
        debt_weight: dec!(0.0637),
        equity_weight: dec!(0.9363),
        size_premium: None,
        country_risk_premium: None,
        specific_risk_premium: None,
        unlevered_beta: None,
        target_debt_equity: None,
    };
    let result = wacc::calculate_wacc(&input).unwrap();
    // Ke = 0.0425 + 1.24 * 0.0472 = 0.0425 + 0.058528 = 0.101028
    // Kd_at = 0.034 * (1 - 0.1623) = 0.034 * 0.8377 = 0.0284818
    // WACC = 0.101028 * 0.9363 + 0.0284818 * 0.0637 ≈ 0.09477 + 0.00181 ≈ 0.09658
    // Close to 10%
    assert!(
        (result.result.wacc - dec!(0.10)).abs() < dec!(0.01),
        "Expected WACC ~10%, got {}",
        result.result.wacc
    );
}

#[test]
fn test_wacc_us_industrial_damodaran() {
    // Typical US industrial: Rf=4.2%, ERP=5.5%, Beta=1.10, Kd=5.5%,
    // t=21%, D/V=30%, E/V=70%
    // Expected WACC ~8.5%
    let input = wacc::WaccInput {
        risk_free_rate: dec!(0.042),
        equity_risk_premium: dec!(0.055),
        beta: dec!(1.10),
        cost_of_debt: dec!(0.055),
        tax_rate: dec!(0.21),
        debt_weight: dec!(0.30),
        equity_weight: dec!(0.70),
        size_premium: None,
        country_risk_premium: None,
        specific_risk_premium: None,
        unlevered_beta: None,
        target_debt_equity: None,
    };
    let result = wacc::calculate_wacc(&input).unwrap();
    assert!(
        result.result.wacc > dec!(0.07) && result.result.wacc < dec!(0.10),
        "US industrial WACC should be ~8.5%, got {}",
        result.result.wacc
    );
}

#[test]
fn test_wacc_emerging_market_with_premiums() {
    // Emerging-market company with all premiums
    let input = wacc::WaccInput {
        risk_free_rate: dec!(0.042),
        equity_risk_premium: dec!(0.055),
        beta: dec!(1.30),
        cost_of_debt: dec!(0.08),
        tax_rate: dec!(0.30),
        debt_weight: dec!(0.20),
        equity_weight: dec!(0.80),
        size_premium: Some(dec!(0.02)),
        country_risk_premium: Some(dec!(0.035)),
        specific_risk_premium: Some(dec!(0.015)),
        unlevered_beta: None,
        target_debt_equity: None,
    };
    let result = wacc::calculate_wacc(&input).unwrap();
    // Ke = 0.042 + 1.30*0.055 + 0.02 + 0.035 + 0.015 = 0.042 + 0.0715 + 0.07 = 0.1835
    // Kd_at = 0.08 * 0.70 = 0.056
    // WACC = 0.1835 * 0.80 + 0.056 * 0.20 = 0.1468 + 0.0112 = 0.158
    let expected_wacc = dec!(0.158);
    assert!(
        (result.result.wacc - expected_wacc).abs() < dec!(0.001),
        "Expected WACC ~{expected_wacc}, got {}",
        result.result.wacc
    );
}

#[test]
fn test_wacc_hamada_relever() {
    // Re-lever from unlevered beta
    let input = wacc::WaccInput {
        risk_free_rate: dec!(0.04),
        equity_risk_premium: dec!(0.06),
        beta: dec!(1.0), // will be overridden
        cost_of_debt: dec!(0.05),
        tax_rate: dec!(0.25),
        debt_weight: dec!(0.40),
        equity_weight: dec!(0.60),
        size_premium: None,
        country_risk_premium: None,
        specific_risk_premium: None,
        unlevered_beta: Some(dec!(0.85)),
        target_debt_equity: Some(dec!(0.667)),
    };
    let result = wacc::calculate_wacc(&input).unwrap();
    // Beta_L = 0.85 * (1 + (1-0.25) * 0.667) = 0.85 * (1 + 0.50025) = 0.85 * 1.50025 = 1.2752
    let expected_beta_l = dec!(0.85) * (Decimal::ONE + dec!(0.75) * dec!(0.667));
    assert!(
        (result.result.levered_beta - expected_beta_l).abs() < dec!(0.001),
        "Expected levered beta ~{expected_beta_l}, got {}",
        result.result.levered_beta
    );
    assert_eq!(result.result.unlevered_beta, Some(dec!(0.85)));
}

#[test]
fn test_wacc_weights_must_sum_to_one() {
    let input = wacc::WaccInput {
        risk_free_rate: dec!(0.04),
        equity_risk_premium: dec!(0.05),
        beta: dec!(1.0),
        cost_of_debt: dec!(0.04),
        tax_rate: dec!(0.20),
        debt_weight: dec!(0.50),
        equity_weight: dec!(0.60), // sum = 1.10
        size_premium: None,
        country_risk_premium: None,
        specific_risk_premium: None,
        unlevered_beta: None,
        target_debt_equity: None,
    };
    assert!(wacc::calculate_wacc(&input).is_err());
}

#[test]
fn test_wacc_negative_risk_free_rate_rejected() {
    let input = wacc::WaccInput {
        risk_free_rate: dec!(-0.01),
        equity_risk_premium: dec!(0.05),
        beta: dec!(1.0),
        cost_of_debt: dec!(0.04),
        tax_rate: dec!(0.20),
        debt_weight: dec!(0.30),
        equity_weight: dec!(0.70),
        size_premium: None,
        country_risk_premium: None,
        specific_risk_premium: None,
        unlevered_beta: None,
        target_debt_equity: None,
    };
    assert!(wacc::calculate_wacc(&input).is_err());
}

#[test]
fn test_wacc_zero_beta_rejected() {
    let input = wacc::WaccInput {
        risk_free_rate: dec!(0.04),
        equity_risk_premium: dec!(0.05),
        beta: Decimal::ZERO,
        cost_of_debt: dec!(0.04),
        tax_rate: dec!(0.20),
        debt_weight: dec!(0.30),
        equity_weight: dec!(0.70),
        size_premium: None,
        country_risk_premium: None,
        specific_risk_premium: None,
        unlevered_beta: None,
        target_debt_equity: None,
    };
    assert!(wacc::calculate_wacc(&input).is_err());
}

#[test]
fn test_wacc_tax_rate_out_of_range_rejected() {
    let input = wacc::WaccInput {
        risk_free_rate: dec!(0.04),
        equity_risk_premium: dec!(0.05),
        beta: dec!(1.0),
        cost_of_debt: dec!(0.04),
        tax_rate: dec!(1.5), // > 1.0
        debt_weight: dec!(0.30),
        equity_weight: dec!(0.70),
        size_premium: None,
        country_risk_premium: None,
        specific_risk_premium: None,
        unlevered_beta: None,
        target_debt_equity: None,
    };
    assert!(wacc::calculate_wacc(&input).is_err());
}

#[test]
fn test_unlever_relever_roundtrip() {
    let beta_l = dec!(1.50);
    let tax = dec!(0.25);
    let de = dec!(0.80);
    let beta_u = wacc::unlever_beta(beta_l, tax, de).unwrap();
    let beta_l_back = wacc::relever_beta(beta_u, tax, de);
    assert!(
        (beta_l - beta_l_back).abs() < dec!(0.00001),
        "Round-trip failed: {beta_l} -> {beta_u} -> {beta_l_back}"
    );
}

// ===========================================================================
// DCF tests
// ===========================================================================

fn sample_dcf_input() -> dcf::DcfInput {
    dcf::DcfInput {
        base_revenue: dec!(1_000_000),
        revenue_growth_rates: vec![
            dec!(0.10), dec!(0.08), dec!(0.07), dec!(0.06), dec!(0.05),
        ],
        ebitda_margin: dec!(0.25),
        ebit_margin: None,
        da_as_pct_revenue: Some(dec!(0.03)),
        capex_as_pct_revenue: dec!(0.05),
        nwc_as_pct_revenue: dec!(0.10),
        tax_rate: dec!(0.25),
        wacc: dec!(0.10),
        wacc_input: None,
        terminal_method: dcf::TerminalMethod::GordonGrowth,
        terminal_growth_rate: Some(dec!(0.025)),
        terminal_exit_multiple: None,
        currency: Currency::USD,
        forecast_years: None,
        mid_year_convention: Some(true),
        net_debt: Some(dec!(500_000)),
        minority_interest: None,
        shares_outstanding: Some(dec!(1000)),
    }
}

#[test]
fn test_dcf_basic_gordon_growth() {
    let input = sample_dcf_input();
    let result = dcf::calculate_dcf(&input).unwrap();
    let out = &result.result;

    assert_eq!(out.projections.len(), 5);
    assert!(out.enterprise_value > Decimal::ZERO);
    assert!(out.terminal_value_gordon.is_some());
    assert!(out.terminal_value_exit.is_none());
    assert_eq!(out.wacc_used, dec!(0.10));
    assert!(out.terminal_value_pct >= Decimal::ZERO);
    assert!(out.terminal_value_pct <= Decimal::ONE);
}

#[test]
fn test_dcf_year1_projection_values() {
    let input = sample_dcf_input();
    let result = dcf::calculate_dcf(&input).unwrap();
    let y1 = &result.result.projections[0];

    // Revenue = 1M * 1.10 = 1.1M
    assert_eq!(y1.revenue, dec!(1_100_000));
    // EBITDA = 1.1M * 0.25 = 275_000
    assert_eq!(y1.ebitda, dec!(275_000));
    // D&A = 1.1M * 0.03 = 33_000
    assert_eq!(y1.plus_da, dec!(33_000));
    // EBIT = 275_000 - 33_000 = 242_000
    assert_eq!(y1.ebit, dec!(242_000));
}

#[test]
fn test_dcf_exit_multiple_method() {
    let mut input = sample_dcf_input();
    input.terminal_method = dcf::TerminalMethod::ExitMultiple;
    input.terminal_growth_rate = None;
    input.terminal_exit_multiple = Some(dec!(10));

    let result = dcf::calculate_dcf(&input).unwrap();
    let out = &result.result;

    assert!(out.terminal_value_exit.is_some());
    assert!(out.terminal_value_gordon.is_none());
    let last_ebitda = out.projections.last().unwrap().ebitda;
    assert_eq!(out.terminal_value_exit.unwrap(), last_ebitda * dec!(10));
}

#[test]
fn test_dcf_equity_bridge() {
    let input = sample_dcf_input();
    let result = dcf::calculate_dcf(&input).unwrap();
    let out = &result.result;

    assert!(out.equity_value.is_some());
    let eq = out.equity_value.unwrap();
    assert_eq!(eq, out.enterprise_value - dec!(500_000));
    assert!(out.equity_value_per_share.is_some());
    let eps = out.equity_value_per_share.unwrap();
    assert_eq!(eps, eq / dec!(1000));
}

#[test]
fn test_dcf_terminal_growth_exceeds_wacc_rejected() {
    let mut input = sample_dcf_input();
    input.terminal_growth_rate = Some(dec!(0.12)); // > WACC of 10%
    assert!(dcf::calculate_dcf(&input).is_err());
}

#[test]
fn test_dcf_zero_wacc_rejected() {
    let mut input = sample_dcf_input();
    input.wacc = Decimal::ZERO;
    assert!(dcf::calculate_dcf(&input).is_err());
}

#[test]
fn test_dcf_negative_revenue_rejected() {
    let mut input = sample_dcf_input();
    input.base_revenue = dec!(-100);
    assert!(dcf::calculate_dcf(&input).is_err());
}

#[test]
fn test_dcf_missing_terminal_growth_for_gordon() {
    let mut input = sample_dcf_input();
    input.terminal_method = dcf::TerminalMethod::GordonGrowth;
    input.terminal_growth_rate = None;
    assert!(dcf::calculate_dcf(&input).is_err());
}

#[test]
fn test_dcf_mid_year_vs_end_year_convention() {
    let mut input = sample_dcf_input();
    input.mid_year_convention = Some(false);
    let result_end = dcf::calculate_dcf(&input).unwrap();

    input.mid_year_convention = Some(true);
    let result_mid = dcf::calculate_dcf(&input).unwrap();

    // Mid-year convention should produce higher EV (less discounting)
    assert!(
        result_mid.result.enterprise_value > result_end.result.enterprise_value,
        "Mid-year EV ({}) should exceed end-of-year EV ({})",
        result_mid.result.enterprise_value,
        result_end.result.enterprise_value,
    );
}
