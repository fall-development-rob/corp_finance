use chrono::NaiveDate;
use corp_finance_core::pe::{debt_schedule, returns, sources_uses};
use corp_finance_core::types::CashFlow;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

// ===========================================================================
// PE Returns tests — IRR / XIRR / MOIC
// ===========================================================================

#[test]
fn test_moic_known_answer() {
    // Invest 100, receive 50+50+200 => MOIC = 300/100 = 3.0x
    let input = returns::ReturnsInput {
        cash_flows: vec![dec!(-100), dec!(50), dec!(50), dec!(200)],
        dated_cash_flows: None,
        entry_equity: dec!(100),
        exit_equity: dec!(200),
        holding_period_years: Some(dec!(3)),
        dates: None,
    };
    let result = returns::calculate_returns(&input).unwrap();
    assert_eq!(result.result.moic, dec!(3));
}

#[test]
fn test_moic_2x_typical_pe() {
    // Typical 2x MOIC over 5 years
    let input = returns::ReturnsInput {
        cash_flows: vec![dec!(-500), dec!(0), dec!(0), dec!(0), dec!(0), dec!(1000)],
        dated_cash_flows: None,
        entry_equity: dec!(500),
        exit_equity: dec!(1000),
        holding_period_years: Some(dec!(5)),
        dates: None,
    };
    let result = returns::calculate_returns(&input).unwrap();
    assert_eq!(result.result.moic, dec!(2));
    assert_eq!(result.result.cash_on_cash, dec!(2));
}

#[test]
fn test_irr_known_answer_even_cashflows() {
    // -1000, +400, +400, +400 => IRR ~9.7%
    let input = returns::ReturnsInput {
        cash_flows: vec![dec!(-1000), dec!(400), dec!(400), dec!(400)],
        dated_cash_flows: None,
        entry_equity: dec!(1000),
        exit_equity: dec!(400),
        holding_period_years: Some(dec!(3)),
        dates: None,
    };
    let result = returns::calculate_returns(&input).unwrap();
    let irr = result.result.irr.unwrap();
    assert!(
        (irr - dec!(0.097)).abs() < dec!(0.01),
        "Expected IRR ~9.7%, got {}",
        irr
    );
}

#[test]
fn test_irr_high_return_lbo() {
    // -100 invest, +300 exit in 3 years => IRR ~44%
    let input = returns::ReturnsInput {
        cash_flows: vec![dec!(-100), dec!(0), dec!(0), dec!(300)],
        dated_cash_flows: None,
        entry_equity: dec!(100),
        exit_equity: dec!(300),
        holding_period_years: Some(dec!(3)),
        dates: None,
    };
    let result = returns::calculate_returns(&input).unwrap();
    let irr = result.result.irr.unwrap();
    assert!(
        irr > dec!(0.40) && irr < dec!(0.50),
        "Expected IRR ~44%, got {}",
        irr
    );
    assert_eq!(result.result.moic, dec!(3));
}

#[test]
fn test_xirr_with_dated_cashflows() {
    let d0 = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
    let d1 = NaiveDate::from_ymd_opt(2021, 6, 15).unwrap();
    let d2 = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();

    let input = returns::ReturnsInput {
        cash_flows: vec![],
        dated_cash_flows: Some(vec![
            CashFlow {
                date: d0,
                amount: dec!(-1000),
                label: None,
            },
            CashFlow {
                date: d1,
                amount: dec!(200),
                label: None,
            },
            CashFlow {
                date: d2,
                amount: dec!(1200),
                label: None,
            },
        ]),
        entry_equity: dec!(1000),
        exit_equity: dec!(1200),
        holding_period_years: None,
        dates: Some((d0, d2)),
    };
    let result = returns::calculate_returns(&input).unwrap();
    assert!(result.result.xirr.is_some());
    let xirr = result.result.xirr.unwrap();
    assert!(
        xirr > dec!(0.05) && xirr < dec!(0.50),
        "XIRR should be reasonable, got {}",
        xirr
    );
}

#[test]
fn test_holding_period_from_dates() {
    let d0 = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
    let d1 = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let input = returns::ReturnsInput {
        cash_flows: vec![],
        dated_cash_flows: None,
        entry_equity: dec!(100),
        exit_equity: dec!(200),
        holding_period_years: None,
        dates: Some((d0, d1)),
    };
    let result = returns::calculate_returns(&input).unwrap();
    assert!(
        (result.result.holding_period - dec!(5)).abs() < dec!(0.1),
        "Expected ~5 year holding period, got {}",
        result.result.holding_period
    );
}

#[test]
fn test_zero_entry_equity_error() {
    let input = returns::ReturnsInput {
        cash_flows: vec![],
        dated_cash_flows: None,
        entry_equity: Decimal::ZERO,
        exit_equity: dec!(100),
        holding_period_years: Some(dec!(1)),
        dates: None,
    };
    assert!(returns::calculate_returns(&input).is_err());
}

#[test]
fn test_negative_entry_equity_error() {
    let input = returns::ReturnsInput {
        cash_flows: vec![],
        dated_cash_flows: None,
        entry_equity: dec!(-100),
        exit_equity: dec!(200),
        holding_period_years: Some(dec!(3)),
        dates: None,
    };
    assert!(returns::calculate_returns(&input).is_err());
}

// ===========================================================================
// Debt Schedule tests
// ===========================================================================

fn bullet_tranche() -> debt_schedule::DebtTrancheInput {
    debt_schedule::DebtTrancheInput {
        name: "Senior Term Loan".into(),
        amount: dec!(5_000_000),
        interest_rate: dec!(0.06),
        is_floating: false,
        base_rate: None,
        spread: None,
        amortisation: debt_schedule::AmortisationType::Bullet,
        maturity_years: 7,
        pik_rate: None,
        seniority: 1,
        commitment_fee: None,
        is_revolver: false,
    }
}

#[test]
fn test_bullet_schedule_no_amort_until_maturity() {
    let result = debt_schedule::build_debt_schedule(&bullet_tranche()).unwrap();
    let sched = &result.result;

    assert_eq!(sched.periods.len(), 7);

    // No repayment in years 1-6
    for p in &sched.periods[..6] {
        assert_eq!(p.scheduled_repayment, Decimal::ZERO);
        assert_eq!(p.closing_balance, dec!(5_000_000));
    }

    // Full repayment in year 7
    let last = &sched.periods[6];
    assert_eq!(last.scheduled_repayment, dec!(5_000_000));
    assert_eq!(last.closing_balance, Decimal::ZERO);

    // Total interest = 5M * 0.06 * 7 = 2_100_000
    assert_eq!(sched.total_interest_paid, dec!(2_100_000));
    assert_eq!(sched.total_principal_paid, dec!(5_000_000));
}

#[test]
fn test_straight_line_amortisation() {
    let mut input = bullet_tranche();
    input.amortisation = debt_schedule::AmortisationType::StraightLine(dec!(0.10)); // 10% per year
    input.maturity_years = 5;
    input.amount = dec!(1_000_000);

    let result = debt_schedule::build_debt_schedule(&input).unwrap();
    let sched = &result.result;

    // Year 1: repay 100k (10% of 1M)
    assert_eq!(sched.periods[0].scheduled_repayment, dec!(100_000));
    assert_eq!(sched.periods[0].closing_balance, dec!(900_000));

    // All principal repaid
    assert_eq!(sched.total_principal_paid, dec!(1_000_000));
    assert_eq!(sched.periods.last().unwrap().closing_balance, Decimal::ZERO);
}

#[test]
fn test_pik_interest_capitalisation() {
    let mut input = bullet_tranche();
    input.pik_rate = Some(dec!(0.03));
    input.maturity_years = 2;
    input.amount = dec!(1_000_000);

    let result = debt_schedule::build_debt_schedule(&input).unwrap();
    let sched = &result.result;

    // Year 1: PIK = 1M * 0.03 = 30k, balance becomes 1.03M
    assert_eq!(sched.periods[0].pik_interest, dec!(30_000));
    assert_eq!(sched.periods[0].closing_balance, dec!(1_030_000));

    // Year 2: PIK = 1.03M * 0.03 = 30_900
    assert_eq!(sched.periods[1].opening_balance, dec!(1_030_000));
    assert_eq!(sched.periods[1].pik_interest, dec!(30_900));
    // Bullet: entire balance repaid at maturity
    assert_eq!(sched.periods[1].closing_balance, Decimal::ZERO);
}

#[test]
fn test_floating_rate_tranche() {
    let mut input = bullet_tranche();
    input.is_floating = true;
    input.base_rate = Some(dec!(0.04));
    input.spread = Some(dec!(0.025));
    input.maturity_years = 1;
    input.amount = dec!(1_000_000);

    let result = debt_schedule::build_debt_schedule(&input).unwrap();
    let sched = &result.result;

    // Effective rate = 0.04 + 0.025 = 0.065
    // Interest = 1M * 0.065 = 65_000
    assert_eq!(sched.periods[0].interest, dec!(65_000));
}

#[test]
fn test_zero_amount_error() {
    let mut input = bullet_tranche();
    input.amount = Decimal::ZERO;
    assert!(debt_schedule::build_debt_schedule(&input).is_err());
}

#[test]
fn test_zero_maturity_error() {
    let mut input = bullet_tranche();
    input.maturity_years = 0;
    assert!(debt_schedule::build_debt_schedule(&input).is_err());
}

// ===========================================================================
// Sources & Uses tests
// ===========================================================================

#[test]
fn test_sources_uses_balanced() {
    // EV 1000 + fees 50 = 1050 uses
    // Equity 400 + Senior 500 + Mezz 150 = 1050 sources
    let input = sources_uses::SourcesUsesInput {
        enterprise_value: dec!(10_000_000),
        equity_contribution: dec!(4_000_000),
        debt_tranches: vec![
            ("Senior Debt".into(), dec!(5_000_000)),
            ("Mezzanine".into(), dec!(1_500_000)),
        ],
        transaction_fees: Some(dec!(300_000)),
        financing_fees: Some(dec!(200_000)),
        management_rollover: None,
    };
    let result = sources_uses::build_sources_uses(&input).unwrap();
    let out = &result.result;

    assert_eq!(out.total_sources, dec!(10_500_000));
    assert_eq!(out.total_uses, dec!(10_500_000));
    assert!(out.balanced);
}

#[test]
fn test_sources_uses_unbalanced() {
    let input = sources_uses::SourcesUsesInput {
        enterprise_value: dec!(1000),
        equity_contribution: dec!(300),
        debt_tranches: vec![("Senior".into(), dec!(500))],
        transaction_fees: None,
        financing_fees: None,
        management_rollover: None,
    };
    let result = sources_uses::build_sources_uses(&input).unwrap();
    assert!(!result.result.balanced);
    assert_eq!(result.result.total_sources, dec!(800));
    assert_eq!(result.result.total_uses, dec!(1000));
}

#[test]
fn test_sources_uses_with_management_rollover() {
    let input = sources_uses::SourcesUsesInput {
        enterprise_value: dec!(1000),
        equity_contribution: dec!(350),
        debt_tranches: vec![("Term Loan".into(), dec!(600))],
        transaction_fees: None,
        financing_fees: None,
        management_rollover: Some(dec!(50)),
    };
    let result = sources_uses::build_sources_uses(&input).unwrap();
    assert_eq!(result.result.total_sources, dec!(1000));
    assert!(result.result.balanced);
    assert!(result
        .result
        .sources
        .iter()
        .any(|(n, _)| n == "Management Rollover"));
}

#[test]
fn test_sources_uses_zero_ev_error() {
    let input = sources_uses::SourcesUsesInput {
        enterprise_value: Decimal::ZERO,
        equity_contribution: dec!(100),
        debt_tranches: vec![],
        transaction_fees: None,
        financing_fees: None,
        management_rollover: None,
    };
    assert!(sources_uses::build_sources_uses(&input).is_err());
}

#[test]
fn test_sources_uses_negative_equity_error() {
    let input = sources_uses::SourcesUsesInput {
        enterprise_value: dec!(1000),
        equity_contribution: dec!(-100),
        debt_tranches: vec![],
        transaction_fees: None,
        financing_fees: None,
        management_rollover: None,
    };
    assert!(sources_uses::build_sources_uses(&input).is_err());
}

#[test]
fn test_sources_uses_negative_tranche_error() {
    let input = sources_uses::SourcesUsesInput {
        enterprise_value: dec!(1000),
        equity_contribution: dec!(500),
        debt_tranches: vec![("Bad Tranche".into(), dec!(-200))],
        transaction_fees: None,
        financing_fees: None,
        management_rollover: None,
    };
    assert!(sources_uses::build_sources_uses(&input).is_err());
}
