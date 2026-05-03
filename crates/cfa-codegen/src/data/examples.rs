//! Hand-written rich examples for the priority tools.
//!
//! Mirrors `RICH_EXAMPLES` in `scripts/gen-examples.py`. Order of fields in
//! each example matches the Python dict insertion order so the resulting
//! JSON is byte-identical to what the Python script emits.

use serde_json::{json, Value};

/// Returns the rich example for a tool, or `None` if it should fall through
/// to the auto-generated placeholder.
pub fn lookup(tool: &str) -> Option<Value> {
    Some(match tool {
        "calculate_wacc" => json!({
            "risk_free_rate": "0.045",
            "equity_risk_premium": "0.055",
            "beta": "1.20",
            "cost_of_debt": "0.045",
            "tax_rate": "0.21",
            "debt_weight": "0.30",
            "equity_weight": "0.70"
        }),
        "build_dcf" => json!({
            "base_revenue": "1000000",
            "currency": "USD",
            "revenue_growth_rates": ["0.08", "0.07", "0.06", "0.05", "0.04"],
            "ebitda_margin": "0.25",
            "capex_as_pct_revenue": "0.05",
            "nwc_as_pct_revenue": "0.02",
            "tax_rate": "0.21",
            "wacc": "0.10",
            "terminal_method": "GordonGrowth",
            "terminal_growth_rate": "0.025",
            "shares_outstanding": "100000",
            "net_debt": "200000"
        }),
        "comps_analysis" => json!({
            "target": {
                "ticker": "TGT",
                "ev": "5000000",
                "revenue": "1200000",
                "ebitda": "300000",
                "earnings": "150000",
                "book_value": "800000"
            },
            "peers": [
                {
                    "ticker": "PEER1",
                    "ev": "4500000",
                    "revenue": "1100000",
                    "ebitda": "275000",
                    "earnings": "140000",
                    "book_value": "750000"
                },
                {
                    "ticker": "PEER2",
                    "ev": "5500000",
                    "revenue": "1300000",
                    "ebitda": "325000",
                    "earnings": "160000",
                    "book_value": "850000"
                }
            ]
        }),
        "credit_metrics" => json!({
            "revenue": "2000000",
            "ebitda": "500000",
            "ebit": "400000",
            "interest_expense": "60000",
            "depreciation_amortisation": "100000",
            "total_debt": "1500000",
            "cash": "200000",
            "total_assets": "3000000",
            "current_assets": "400000",
            "current_liabilities": "250000",
            "total_equity": "1000000",
            "retained_earnings": "500000",
            "working_capital": "150000",
            "operating_cash_flow": "350000",
            "capex": "100000"
        }),
        "altman_zscore" => json!({
            "working_capital": "150000",
            "total_assets": "3000000",
            "retained_earnings": "500000",
            "ebit": "400000",
            "revenue": "2000000",
            "total_liabilities": "1500000",
            "market_cap": "2500000",
            "is_public": true,
            "is_manufacturing": true
        }),
        "build_lbo" => json!({
            "entry_ev": "10000000",
            "entry_ebitda": "1500000",
            "base_revenue": "6000000",
            "revenue_growth": ["0.08", "0.07", "0.06", "0.05", "0.04"],
            "ebitda_margin": ["0.25", "0.26", "0.27", "0.27", "0.28"],
            "da_as_pct_revenue": "0.04",
            "capex_as_pct_revenue": "0.05",
            "nwc_as_pct_revenue": "0.02",
            "tax_rate": "0.21",
            "tranches": [
                {
                    "name": "Term Loan B",
                    "amount": "5000000",
                    "interest_rate": "0.07",
                    "is_floating": false,
                    "amortisation": "Bullet",
                    "maturity_years": 7,
                    "seniority": 1,
                    "is_revolver": false
                }
            ],
            "equity_contribution": "5000000",
            "exit_year": 5,
            "exit_multiple": "9.0",
            "transaction_fees": "200000",
            "financing_fees": "150000"
        }),
        "price_bond" => json!({
            "face_value": "1000",
            "coupon_rate": "0.05",
            "coupon_frequency": 2,
            "ytm": "0.045",
            "settlement_date": "2026-05-03",
            "maturity_date": "2036-05-03",
            "day_count": "Thirty360"
        }),
        "price_option" => json!({
            "spot_price": "100",
            "strike_price": "100",
            "time_to_expiry": "0.5",
            "risk_free_rate": "0.05",
            "volatility": "0.25",
            "dividend_yield": "0.02",
            "option_type": "Call",
            "exercise_style": "European"
        }),
        _ => return None,
    })
}

/// All tool names with rich examples (used for counting / reporting).
pub const RICH_TOOLS: &[&str] = &[
    "calculate_wacc",
    "build_dcf",
    "comps_analysis",
    "credit_metrics",
    "altman_zscore",
    "build_lbo",
    "price_bond",
    "price_option",
];
