---
name: "SSCMFI Bond Math (Native)"
description: "Use the corp-finance-mcp server's native SSCMFI-compatible bond math tools for institutional-grade fixed income calculations. Invoke when performing bond price/yield calculations, accrued interest, yield-to-worst, callable bond analytics, duration, convexity, PV01, YV32, cash flow schedules, and risk metrics for Treasury, Agency, Corporate, Municipal, and CD securities. Supports Periodic, Discount, IAM, Stepped, Multi-Step, PIK, and Part-PIK payment types with multiple day-count conventions (SSCM 30/360, Actual/Actual, Actual/360, Actual/365). All computation uses native Rust 128-bit decimal precision — no external API calls required."
---

# SSCMFI Bond Math (Native)

You have access to 4 native SSCMFI bond math MCP tools implemented in Rust with 128-bit decimal precision. These tools replace external SSCMFI API calls with identical semantics, computed locally. All tools return structured JSON with `result`, `methodology`, `assumptions`, `warnings`, and `metadata` fields.

## Tool Reference

### SSCMFI Bond Calculator

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `sscmfi_bond` | Full SSCMFI bond math — price/yield, accrued interest, analytics, cashflows | security_type, payment_type, maturity_date, coupon_rate, given_type, given_value |
| `sscmfi_bond_batch` | Batch calculation — up to 100 bonds per call | bonds (array of sscmfi_bond inputs) |
| `sscmfi_price_to_yield` | Quick price-to-yield shortcut | security_type, maturity_date, coupon_rate, price |
| `sscmfi_yield_to_price` | Quick yield-to-price shortcut | security_type, maturity_date, coupon_rate, yield_value |

### CLI Commands

| CLI Command | Purpose |
|-------------|---------|
| `cfa sscmfi-bond --input bond.json` | Full SSCMFI bond calculation from JSON |

## Security Types & Default Conventions

| Security Type | Day Count | Frequency | EOM Rule |
|---------------|-----------|-----------|----------|
| Treasury | Actual/Actual | Semiannual | Adjust |
| Agency | SSCM 30/360 | Semiannual | Adjust |
| Corporate | SSCM 30/360 | Semiannual | Adjust |
| Municipal | SSCM 30/360 | Semiannual | Adjust |
| CD | Actual/360 | Monthly | Adjust |

## Payment Types

| Type | Description |
|------|-------------|
| **Periodic** | Standard periodic coupon bond (default) |
| **Discount** | Zero-coupon / discount security — no periodic coupons |
| **IAM** | Interest At Maturity — all interest paid in one lump sum at maturity |
| **Stepped** | Single step coupon change — rate changes once at step date |
| **Multistep** | Multiple coupon rate changes at scheduled dates |
| **PIK** | Payment-In-Kind — interest accrues to principal instead of cash payment |
| **PartPIK** | Part cash / part PIK — split between cash_rate and pik_rate |

## Day Count Conventions

| Convention | Description |
|------------|-------------|
| `SSCM30_360` | 30/360 (SSCM variant — assumes 30-day months, 360-day year) |
| `ActualActual` | ACT/ACT ICMA — actual days in period / actual days in year |
| `Actual360` | ACT/360 — actual days / 360 |
| `Actual365` | ACT/365 Fixed — actual days / 365 |

## Analytics Output

When `calc_analytics: true` (default), the response includes:

| Metric | Description |
|--------|-------------|
| `macaulay_duration` | Weighted average time to receipt of cash flows |
| `modified_duration` | Price sensitivity to yield changes (% change per 100bp) |
| `convexity` | Second-order price sensitivity (curvature) |
| `pv01` | Price value of a basis point (dollar duration for 1bp move) |
| `yv32` | Yield value of a 32nd (yield change for 1/32 price change) |

## Callable Bond Analytics

When `call_schedule` is provided, the tool computes:
- **Yield to each call date** — yield assuming bond is called at each scheduled date/price
- **Yield-to-worst** — minimum of yield-to-maturity and all yield-to-call values
- **Redemption info** — whether worst yield is to maturity or a specific call date

## Input Conventions

**IMPORTANT** — SSCMFI uses these conventions (different from other fixed income tools):
- **Rates as percentages**: coupon_rate=5.0 means 5%, NOT 0.05
- **Prices per 100 par**: price=98.5 means $98.50 per $100 face
- **Dates in MM/DD/YYYY**: maturity_date="06/15/2030"
- **given_type/given_value**: specify whether you're providing "Price" or "Yield"

## Response Envelope

All responses follow the standard ComputationOutput envelope:

```json
{
  "result": { ... },
  "methodology": "SSCMFI Bond Math (native Rust, 128-bit decimal)",
  "assumptions": { ... },
  "warnings": [],
  "metadata": {
    "precision": "rust_decimal_128bit",
    "computation_time_us": 42
  }
}
```

## Example Workflows

### Price a Corporate Bond from Yield

```json
{
  "security_type": "Corporate",
  "payment_type": "Periodic",
  "maturity_date": "06/15/2030",
  "coupon_rate": 5.25,
  "given_type": "Yield",
  "given_value": 5.50,
  "calc_analytics": true
}
```

### Compute Yield-to-Worst for Callable Municipal

```json
{
  "security_type": "Municipal",
  "maturity_date": "01/01/2035",
  "coupon_rate": 4.0,
  "given_type": "Price",
  "given_value": 102.5,
  "call_schedule": [
    { "date": "01/01/2028", "price": 102 },
    { "date": "01/01/2030", "price": 101 },
    { "date": "01/01/2032", "price": 100 }
  ],
  "calc_analytics": true
}
```

### Zero-Coupon Treasury

```json
{
  "security_type": "Treasury",
  "payment_type": "Discount",
  "maturity_date": "03/15/2026",
  "coupon_rate": 0,
  "given_type": "Yield",
  "given_value": 4.75
}
```

### PIK Bond Pricing

```json
{
  "security_type": "Corporate",
  "payment_type": "PIK",
  "maturity_date": "12/01/2029",
  "coupon_rate": 8.0,
  "pik_rate": 8.0,
  "given_type": "Yield",
  "given_value": 9.5
}
```

### Part-PIK (Cash + PIK Split)

```json
{
  "security_type": "Corporate",
  "payment_type": "PartPIK",
  "maturity_date": "06/15/2028",
  "coupon_rate": 10.0,
  "cash_rate": 6.0,
  "pik_rate": 4.0,
  "given_type": "Price",
  "given_value": 95.0
}
```

### Stepped Coupon Bond

```json
{
  "security_type": "Corporate",
  "payment_type": "Stepped",
  "maturity_date": "09/01/2030",
  "coupon_rate": 3.5,
  "given_type": "Yield",
  "given_value": 5.0,
  "step_schedule": [
    { "date": "09/01/2027", "coupon_rate": 5.0 }
  ]
}
```

### Batch: Multiple Bonds at Once

```json
{
  "bonds": [
    { "security_type": "Treasury", "maturity_date": "05/15/2027", "coupon_rate": 4.5, "given_type": "Yield", "given_value": 4.25 },
    { "security_type": "Corporate", "maturity_date": "03/01/2030", "coupon_rate": 6.0, "given_type": "Price", "given_value": 101.5 },
    { "security_type": "Municipal", "maturity_date": "07/01/2035", "coupon_rate": 3.75, "given_type": "Price", "given_value": 98.0 }
  ]
}
```

## Tool Chaining Workflows

1. **Relative value**: Use `sscmfi_bond_batch` to price a portfolio, then `credit_spreads` for spread analysis
2. **Curve construction**: Use `bootstrap_spot_curve` for the benchmark curve, then `sscmfi_bond` to compute Z-spread vs that curve
3. **Callable analysis**: Use `sscmfi_bond` with call_schedule to get yield-to-worst, then `bond_duration` for key rate durations
4. **Risk metrics**: Use `sscmfi_bond` for PV01/YV32, then `risk_metrics` for portfolio-level VaR
