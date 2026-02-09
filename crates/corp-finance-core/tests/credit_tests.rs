use corp_finance_core::credit::{capacity, metrics};
use corp_finance_core::CorpFinanceError;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

// ===========================================================================
// Credit Metrics tests
// ===========================================================================

fn sample_bbb_company() -> metrics::CreditMetricsInput {
    // A typical BBB-rated company: leverage ~3x, coverage ~5x
    metrics::CreditMetricsInput {
        revenue: dec!(5_000_000),
        ebitda: dec!(1_000_000),
        ebit: dec!(800_000),
        interest_expense: dec!(200_000),
        depreciation_amortisation: dec!(200_000),
        total_debt: dec!(3_000_000),
        cash: dec!(300_000),
        total_assets: dec!(8_000_000),
        current_assets: dec!(1_500_000),
        current_liabilities: dec!(1_000_000),
        total_equity: dec!(3_500_000),
        retained_earnings: dec!(2_000_000),
        working_capital: dec!(500_000),
        operating_cash_flow: dec!(900_000),
        capex: dec!(250_000),
        funds_from_operations: Some(dec!(850_000)),
        lease_payments: Some(dec!(50_000)),
        preferred_dividends: None,
        market_cap: Some(dec!(5_000_000)),
    }
}

#[test]
fn test_credit_metrics_bbb_company() {
    let input = sample_bbb_company();
    let result = metrics::calculate_credit_metrics(&input).unwrap();
    let m = &result.result;

    // Net debt = 3M - 300k = 2.7M
    assert_eq!(m.net_debt, dec!(2_700_000));

    // Net debt / EBITDA = 2.7M / 1M = 2.7
    assert_eq!(m.net_debt_to_ebitda, dec!(2.7));

    // Total debt / EBITDA = 3M / 1M = 3.0
    assert_eq!(m.total_debt_to_ebitda, dec!(3));

    // Debt / equity = 3M / 3.5M ≈ 0.857
    let expected_de = dec!(3_000_000) / dec!(3_500_000);
    assert_eq!(m.debt_to_equity, expected_de);

    // Interest coverage = EBITDA / interest = 1M / 200k = 5.0
    assert_eq!(m.interest_coverage, dec!(5));

    // EBIT coverage = 800k / 200k = 4.0
    assert_eq!(m.ebit_coverage, dec!(4));

    // FCF = 900k - 250k = 650k
    assert_eq!(m.fcf, dec!(650_000));

    // Current ratio = 1.5M / 1M = 1.5
    assert_eq!(m.current_ratio, dec!(1.5));
}

#[test]
fn test_credit_metrics_implied_rating_bbb_area() {
    let input = sample_bbb_company();
    let result = metrics::calculate_credit_metrics(&input).unwrap();
    let m = &result.result;

    // Coverage=5x, leverage=2.7x => A zone per the rating grid
    // (c > 5.0 && l < 2.5 => A, but l=2.7 so it falls to c > 4.0 && l < 3.5 => BBB)
    assert!(
        m.implied_rating == metrics::CreditRating::BBB
            || m.implied_rating == metrics::CreditRating::A,
        "Expected BBB or A area rating, got {:?}",
        m.implied_rating,
    );
}

#[test]
fn test_credit_metrics_dscr() {
    let input = sample_bbb_company();
    let result = metrics::calculate_credit_metrics(&input).unwrap();
    // DSCR = (EBITDA - capex) / interest = (1M - 250k) / 200k = 750k / 200k = 3.75
    assert_eq!(result.result.dscr, dec!(3.75));
}

#[test]
fn test_credit_metrics_ffo_to_debt() {
    let input = sample_bbb_company();
    let result = metrics::calculate_credit_metrics(&input).unwrap();
    // FFO/debt = 850k / 3M ≈ 0.2833
    let expected = dec!(850_000) / dec!(3_000_000);
    assert_eq!(result.result.ffo_to_debt, Some(expected));
}

#[test]
fn test_credit_metrics_fixed_charge_coverage() {
    let input = sample_bbb_company();
    let result = metrics::calculate_credit_metrics(&input).unwrap();
    // total_charges = interest(200k) + lease(50k) = 250k
    // numerator (EBITDAR proxy) = EBITDA(1M) + lease(50k) = 1.05M
    // FCC = 1.05M / 250k = 4.2
    assert_eq!(result.result.fixed_charge_coverage, Some(dec!(4.2)));
}

#[test]
fn test_credit_metrics_net_debt_to_ev() {
    let input = sample_bbb_company();
    let result = metrics::calculate_credit_metrics(&input).unwrap();
    // EV = market_cap(5M) + net_debt(2.7M) = 7.7M
    // net_debt/EV = 2.7M / 7.7M
    let nd_ev = result.result.net_debt_to_ev.unwrap();
    let expected = dec!(2_700_000) / dec!(7_700_000);
    assert_eq!(nd_ev, expected);
}

#[test]
fn test_credit_metrics_strong_company_aaa() {
    // High coverage, low leverage => AAA zone
    let input = metrics::CreditMetricsInput {
        revenue: dec!(10_000_000),
        ebitda: dec!(3_000_000),
        ebit: dec!(2_800_000),
        interest_expense: dec!(100_000),
        depreciation_amortisation: dec!(200_000),
        total_debt: dec!(500_000),
        cash: dec!(1_000_000),
        total_assets: dec!(12_000_000),
        current_assets: dec!(3_000_000),
        current_liabilities: dec!(1_500_000),
        total_equity: dec!(8_000_000),
        retained_earnings: dec!(5_000_000),
        working_capital: dec!(1_500_000),
        operating_cash_flow: dec!(2_500_000),
        capex: dec!(500_000),
        funds_from_operations: None,
        lease_payments: None,
        preferred_dividends: None,
        market_cap: None,
    };
    let result = metrics::calculate_credit_metrics(&input).unwrap();
    // coverage=30x, leverage < 0 (net_debt negative) => AAA
    assert_eq!(result.result.implied_rating, metrics::CreditRating::AAA);
}

#[test]
fn test_credit_metrics_zero_interest_coverage_capped() {
    let mut input = sample_bbb_company();
    input.interest_expense = Decimal::ZERO;
    let result = metrics::calculate_credit_metrics(&input).unwrap();
    assert_eq!(result.result.interest_coverage, dec!(999));
    assert!(result.warnings.iter().any(|w| w.contains("zero")));
}

#[test]
fn test_credit_metrics_negative_revenue_rejected() {
    let mut input = sample_bbb_company();
    input.revenue = dec!(-100);
    let err = metrics::calculate_credit_metrics(&input).unwrap_err();
    match err {
        CorpFinanceError::InvalidInput { field, .. } => assert_eq!(field, "revenue"),
        other => panic!("Expected InvalidInput for revenue, got {other:?}"),
    }
}

#[test]
fn test_credit_metrics_zero_total_debt_division_error() {
    let mut input = sample_bbb_company();
    input.total_debt = Decimal::ZERO;
    assert!(metrics::calculate_credit_metrics(&input).is_err());
}

#[test]
fn test_credit_metrics_zero_current_liabilities_rejected() {
    let mut input = sample_bbb_company();
    input.current_liabilities = Decimal::ZERO;
    assert!(metrics::calculate_credit_metrics(&input).is_err());
}

// ===========================================================================
// Debt Capacity tests
// ===========================================================================

fn sample_capacity_input() -> capacity::DebtCapacityInput {
    capacity::DebtCapacityInput {
        ebitda: dec!(500_000),
        interest_rate: dec!(0.06),
        max_leverage: Some(dec!(4.0)),
        min_interest_coverage: Some(dec!(3.0)),
        min_dscr: Some(dec!(1.5)),
        min_ffo_to_debt: Some(dec!(0.20)),
        existing_debt: Some(dec!(1_000_000)),
        annual_amortisation: Some(dec!(50_000)),
        ffo: Some(dec!(400_000)),
    }
}

#[test]
fn test_debt_capacity_leverage_constraint() {
    let input = sample_capacity_input();
    let result = capacity::calculate_debt_capacity(&input).unwrap();
    // max_by_leverage = EBITDA * max_leverage = 500k * 4 = 2M
    assert_eq!(result.result.max_debt_by_leverage, Some(dec!(2_000_000)));
}

#[test]
fn test_debt_capacity_coverage_constraint() {
    let input = sample_capacity_input();
    let result = capacity::calculate_debt_capacity(&input).unwrap();
    // max_by_coverage = EBITDA / (min_coverage * rate) = 500k / (3.0 * 0.06) = 500k / 0.18
    let expected = dec!(500_000) / dec!(0.18);
    assert_eq!(result.result.max_debt_by_coverage, Some(expected));
}

#[test]
fn test_debt_capacity_dscr_constraint() {
    let input = sample_capacity_input();
    let result = capacity::calculate_debt_capacity(&input).unwrap();
    // debt <= (EBITDA/min_dscr - amort) / rate = (500k/1.5 - 50k) / 0.06
    let ebitda_over_dscr = dec!(500_000) / dec!(1.5);
    let numerator = ebitda_over_dscr - dec!(50_000);
    let expected = numerator / dec!(0.06);
    assert_eq!(result.result.max_debt_by_dscr, Some(expected));
}

#[test]
fn test_debt_capacity_ffo_constraint() {
    let input = sample_capacity_input();
    let result = capacity::calculate_debt_capacity(&input).unwrap();
    // max_by_ffo = FFO / min_ffo_to_debt = 400k / 0.20 = 2M
    assert_eq!(result.result.max_debt_by_ffo, Some(dec!(2_000_000)));
}

#[test]
fn test_debt_capacity_binding_is_minimum() {
    let input = sample_capacity_input();
    let result = capacity::calculate_debt_capacity(&input).unwrap();
    let out = &result.result;

    let all_caps: Vec<Decimal> = [
        out.max_debt_by_leverage,
        out.max_debt_by_coverage,
        out.max_debt_by_dscr,
        out.max_debt_by_ffo,
    ]
    .iter()
    .filter_map(|x| *x)
    .collect();

    let min_cap = all_caps.iter().copied().min().unwrap();
    let expected_incremental = (min_cap - dec!(1_000_000)).max(Decimal::ZERO);
    assert_eq!(out.max_incremental_debt, expected_incremental);
}

#[test]
fn test_debt_capacity_no_constraints_fails() {
    let input = capacity::DebtCapacityInput {
        ebitda: dec!(500_000),
        interest_rate: dec!(0.05),
        max_leverage: None,
        min_interest_coverage: None,
        min_dscr: None,
        min_ffo_to_debt: None,
        existing_debt: None,
        annual_amortisation: None,
        ffo: None,
    };
    assert!(capacity::calculate_debt_capacity(&input).is_err());
}

#[test]
fn test_debt_capacity_negative_ebitda_rejected() {
    let mut input = sample_capacity_input();
    input.ebitda = dec!(-100_000);
    assert!(capacity::calculate_debt_capacity(&input).is_err());
}

#[test]
fn test_debt_capacity_zero_rate_coverage_unconstrained() {
    let input = capacity::DebtCapacityInput {
        ebitda: dec!(500_000),
        interest_rate: Decimal::ZERO,
        max_leverage: Some(dec!(3.0)),
        min_interest_coverage: Some(dec!(2.0)),
        min_dscr: None,
        min_ffo_to_debt: None,
        existing_debt: None,
        annual_amortisation: None,
        ffo: None,
    };
    let result = capacity::calculate_debt_capacity(&input).unwrap();
    // With zero rate, coverage is unconstrained (None)
    assert_eq!(result.result.max_debt_by_coverage, None);
    assert_eq!(result.result.binding_constraint, "max_leverage");
}
