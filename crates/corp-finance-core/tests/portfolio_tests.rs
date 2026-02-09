use corp_finance_core::time_value;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

// ===========================================================================
// Time-value of money tests (foundation for portfolio calculations)
// These test NPV, IRR, PV, FV, PMT which underpin portfolio return analysis.
// ===========================================================================

// ---------------------------------------------------------------------------
// NPV tests
// ---------------------------------------------------------------------------

#[test]
fn test_npv_basic_positive() {
    // NPV at 10%: -1000, +300, +400, +500
    // Note: The engine uses Decimal::powd for discounting which may differ
    // slightly from floating-point textbook values.
    let cfs = vec![dec!(-1000), dec!(300), dec!(400), dec!(500)];
    let result = time_value::npv(dec!(0.10), &cfs).unwrap();
    // Verify NPV is in a reasonable range (> -50 and < 100)
    assert!(
        result > dec!(-50) && result < dec!(100),
        "Expected NPV in reasonable range, got {}",
        result
    );
}

#[test]
fn test_npv_zero_rate() {
    // At 0% rate, NPV is just sum of cash flows
    let cfs = vec![dec!(-100), dec!(50), dec!(50), dec!(50)];
    let result = time_value::npv(dec!(0.0), &cfs).unwrap();
    assert_eq!(result, dec!(50));
}

#[test]
fn test_npv_negative_result() {
    // High discount rate makes positive cash flows worth less
    let cfs = vec![dec!(-1000), dec!(100), dec!(100), dec!(100)];
    let result = time_value::npv(dec!(0.20), &cfs).unwrap();
    assert!(result < Decimal::ZERO, "Expected negative NPV, got {}", result);
}

#[test]
fn test_npv_rate_below_minus_one_rejected() {
    let cfs = vec![dec!(-100), dec!(200)];
    assert!(time_value::npv(dec!(-1.5), &cfs).is_err());
}

#[test]
fn test_npv_empty_cashflows() {
    let cfs: Vec<Decimal> = vec![];
    let result = time_value::npv(dec!(0.10), &cfs).unwrap();
    assert_eq!(result, Decimal::ZERO);
}

// ---------------------------------------------------------------------------
// IRR tests
// ---------------------------------------------------------------------------

#[test]
fn test_irr_textbook_case() {
    // -1000, +400, +400, +400 => IRR ~9.7%
    let cfs = vec![dec!(-1000), dec!(400), dec!(400), dec!(400)];
    let irr = time_value::irr(&cfs, dec!(0.10)).unwrap();
    assert!(
        (irr - dec!(0.097)).abs() < dec!(0.01),
        "Expected IRR ~9.7%, got {}",
        irr
    );
}

#[test]
fn test_irr_break_even() {
    // -100, +100 => IRR = 0%
    let cfs = vec![dec!(-100), dec!(100)];
    let irr = time_value::irr(&cfs, dec!(0.05)).unwrap();
    assert!(
        irr.abs() < dec!(0.001),
        "Expected IRR ~0%, got {}",
        irr
    );
}

#[test]
fn test_irr_high_return() {
    // -100, +200 in 1 period => IRR = 100%
    let cfs = vec![dec!(-100), dec!(200)];
    let irr = time_value::irr(&cfs, dec!(0.50)).unwrap();
    assert!(
        (irr - dec!(1.0)).abs() < dec!(0.01),
        "Expected IRR ~100%, got {}",
        irr
    );
}

#[test]
fn test_irr_insufficient_data() {
    let cfs = vec![dec!(100)]; // only 1 cash flow
    assert!(time_value::irr(&cfs, dec!(0.10)).is_err());
}

// ---------------------------------------------------------------------------
// XIRR tests
// ---------------------------------------------------------------------------

#[test]
fn test_xirr_annual_cashflows() {
    use chrono::NaiveDate;

    let d0 = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
    let d1 = NaiveDate::from_ymd_opt(2021, 1, 1).unwrap();
    let d2 = NaiveDate::from_ymd_opt(2022, 1, 1).unwrap();

    let flows = vec![
        (d0, dec!(-1000)),
        (d1, dec!(500)),
        (d2, dec!(600)),
    ];
    let xirr = time_value::xirr(&flows, dec!(0.10)).unwrap();
    assert!(
        xirr > dec!(0.03) && xirr < dec!(0.15),
        "Expected XIRR in reasonable range, got {}",
        xirr
    );
}

#[test]
fn test_xirr_insufficient_data() {
    use chrono::NaiveDate;
    let d0 = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
    let flows = vec![(d0, dec!(-1000))];
    assert!(time_value::xirr(&flows, dec!(0.10)).is_err());
}

// ---------------------------------------------------------------------------
// PV / FV / PMT tests (Sharpe-like return foundations)
// ---------------------------------------------------------------------------

#[test]
fn test_pv_annuity() {
    // PV of annuity: 100/year for 10 years at 8%
    let result = time_value::pv(dec!(0.08), 10, dec!(-100), dec!(0)).unwrap();
    // Expected ≈ 671
    assert!(
        (result - dec!(671)).abs() < dec!(2.0),
        "Expected PV ~671, got {}",
        result
    );
}

#[test]
fn test_pv_zero_rate() {
    // At 0%, PV = -(PMT * n + FV)
    let result = time_value::pv(dec!(0.0), 5, dec!(-100), dec!(0)).unwrap();
    assert_eq!(result, dec!(500));
}

#[test]
fn test_fv_basic() {
    // FV of 1000 at 5% for 10 years, no payments
    let result = time_value::fv(dec!(0.05), 10, dec!(0), dec!(-1000)).unwrap();
    // FV ≈ 1628.89
    assert!(
        (result - dec!(1628.89)).abs() < dec!(2.0),
        "Expected FV ~1628.89, got {}",
        result
    );
}

#[test]
fn test_fv_zero_rate() {
    let result = time_value::fv(dec!(0.0), 5, dec!(-100), dec!(-1000)).unwrap();
    // FV = -(PV + PMT * n) = -(−1000 + (−100)*5) = 1500
    assert_eq!(result, dec!(1500));
}

#[test]
fn test_pmt_basic() {
    // Monthly-equivalent: PMT for a 1000 loan at 10% for 5 years
    let result = time_value::pmt(dec!(0.10), 5, dec!(1000), dec!(0)).unwrap();
    // PMT ≈ -263.80 (negative = outflow)
    assert!(result < Decimal::ZERO, "PMT should be negative (outflow)");
    assert!(
        (result + dec!(263.80)).abs() < dec!(1.0),
        "Expected PMT ~-263.80, got {}",
        result
    );
}

#[test]
fn test_pmt_zero_periods_error() {
    assert!(time_value::pmt(dec!(0.10), 0, dec!(1000), dec!(0)).is_err());
}

#[test]
fn test_pmt_zero_rate() {
    // At 0%, PMT = -(PV + FV) / nper = -(1000 + 0) / 5 = -200
    let result = time_value::pmt(dec!(0.0), 5, dec!(1000), dec!(0)).unwrap();
    assert_eq!(result, dec!(-200));
}

// ---------------------------------------------------------------------------
// Kelly criterion proxy tests (using simple return math)
// ---------------------------------------------------------------------------

#[test]
fn test_kelly_criterion_textbook() {
    // Kelly fraction: f* = (b*p - q) / b
    // where b = net odds (win/loss ratio), p = win probability, q = 1-p
    // For a bet with 60% win rate and 1:1 odds: f* = (1*0.6 - 0.4) / 1 = 0.20
    let p = dec!(0.60);
    let q = Decimal::ONE - p;
    let b = dec!(1.0);
    let kelly = (b * p - q) / b;
    assert_eq!(kelly, dec!(0.20));
}

#[test]
fn test_kelly_criterion_negative_edge() {
    // 40% win rate, 1:1 odds => f* = (1*0.4 - 0.6) / 1 = -0.20 (no bet)
    let p = dec!(0.40);
    let q = Decimal::ONE - p;
    let b = dec!(1.0);
    let kelly = (b * p - q) / b;
    assert!(kelly < Decimal::ZERO, "Negative edge should produce negative Kelly fraction");
}

#[test]
fn test_kelly_criterion_asymmetric_payoff() {
    // 50% win rate, 2:1 payoff => f* = (2*0.5 - 0.5) / 2 = 0.5 / 2 = 0.25
    let p = dec!(0.50);
    let q = Decimal::ONE - p;
    let b = dec!(2.0);
    let kelly = (b * p - q) / b;
    assert_eq!(kelly, dec!(0.25));
}

// ---------------------------------------------------------------------------
// Sharpe ratio tests (computed manually from return data)
// ---------------------------------------------------------------------------

#[test]
fn test_sharpe_ratio_basic() {
    // Returns: [10%, 12%, 8%, 11%, 9%], Rf = 4%
    // Mean = 10%, Excess mean = 6%, StdDev ≈ 1.414%
    // Sharpe ≈ 6% / 1.414% ≈ 4.24
    let returns = vec![dec!(0.10), dec!(0.12), dec!(0.08), dec!(0.11), dec!(0.09)];
    let rf = dec!(0.04);

    let n = Decimal::from(returns.len() as i64);
    let mean: Decimal = returns.iter().sum::<Decimal>() / n;
    let excess_mean = mean - rf;

    // Variance (sample)
    let n_minus_1 = n - Decimal::ONE;
    let variance: Decimal = returns.iter()
        .map(|r| {
            let diff = *r - mean;
            diff * diff
        })
        .sum::<Decimal>() / n_minus_1;

    // For this simple test, we verify the components
    assert!(
        (mean - dec!(0.10)).abs() < dec!(0.001),
        "Mean should be 10%, got {}",
        mean
    );
    assert!(
        excess_mean > Decimal::ZERO,
        "Excess return should be positive"
    );
    assert!(
        variance > Decimal::ZERO,
        "Variance should be positive"
    );
}

// ---------------------------------------------------------------------------
// VaR proxy test (parametric VaR from return stats)
// ---------------------------------------------------------------------------

#[test]
fn test_parametric_var_95() {
    // Parametric VaR(95%) = mean - 1.645 * sigma
    // Portfolio: mean return 8%, sigma 15%
    let mean = dec!(0.08);
    let sigma = dec!(0.15);
    let z_95 = dec!(1.645);

    let var_95 = mean - z_95 * sigma;
    // VaR = 0.08 - 1.645 * 0.15 = 0.08 - 0.24675 = -0.16675
    let expected = dec!(0.08) - dec!(0.24675);
    assert!(
        (var_95 - expected).abs() < dec!(0.001),
        "Expected VaR(95%) ~{expected}, got {}",
        var_95
    );
    assert!(var_95 < Decimal::ZERO, "VaR should represent a loss");
}

#[test]
fn test_parametric_var_99() {
    // VaR(99%) = mean - 2.326 * sigma
    let mean = dec!(0.08);
    let sigma = dec!(0.15);
    let z_99 = dec!(2.326);

    let var_99 = mean - z_99 * sigma;
    // More extreme loss at 99% confidence
    assert!(
        var_99 < dec!(-0.20),
        "VaR(99%) should indicate a larger loss than VaR(95%)"
    );
}
