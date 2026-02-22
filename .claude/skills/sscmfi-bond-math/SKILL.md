---
name: "SSCMFI Bond Math"
description: "Use the SSCMFI MCP server and REST API for industry-standard fixed income bond calculations. Invoke when performing bond price/yield calculations, accrued interest, yield-to-worst, callable bond analytics, duration, convexity, PV01, cash flow schedules, and risk metrics for Treasury, Agency, Corporate, Municipal, and CD securities. Supports Periodic, Discount, IAM, Stepped, Multi-Step, PIK, and Part-PIK payment types with multiple day-count conventions (30/360, Actual/Actual, Actual/360, Actual/365)."
---

# SSCMFI Bond Math

You have access to the SSCMFI (Standard Securities Calculation Methods Fixed Income) engine for industry-standard bond calculations. This engine implements the definitive formulas from SSCM Volumes 1 & 2 with institutional precision. **Never attempt to manually calculate bond price, yield, accrued interest, or duration** -- always use the SSCMFI engine.

## MCP Tool: `calculate_bond_periodic`

The primary MCP tool for bond calculations. Handles all periodic coupon bond types with intelligent defaults.

### Required Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `securityType` | string | `Treasury`, `Agency`, `Corporate`, `Municipal`, or `CD` -- sets institutional defaults for day count, frequency, and EOM rule |
| `maturityDate` | string | Final maturity date (MM/DD/YYYY format) |
| `couponRate` | number | Annual coupon rate as percentage (e.g., 5.0 for 5%). Use 0 for zero-coupon bonds |
| `givenType` | string | `Price` or `Yield` -- which value you are providing |
| `givenValue` | number | The clean price (e.g., 98.75) or yield as percent (e.g., 4.25) |

### Optional Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `settlementDate` | string | Settlement date (MM/DD/YYYY). Defaults to next business day |
| `callSchedule` | array | List of `{date, price}` objects for callable bonds. Engine auto-calculates yield-to-worst |

### Intelligent Defaults by Security Type

| Security Type | Day Count | Frequency | EOM Rule |
|---------------|-----------|-----------|----------|
| Treasury | Actual/Actual | Semiannual | Adjust |
| Corporate | SSCM30/360 | Semiannual | Adjust |
| Agency | SSCM30/360 | Semiannual | Adjust |
| Municipal | SSCM30/360 | Semiannual | Adjust |
| CD | Actual/360 | Monthly | Adjust |

### Response Fields

**Price/Yield (PY):**
- `price` -- Clean price per 100 par
- `yield` -- Yield to worst as percentage
- `ai` -- Accrued interest per 100 par
- `tradingPrice` -- Dirty price (clean + accrued)

**Analytics (PYAnalytics):**
- `approxDuration` -- Macaulay Duration
- `approxModDuration` -- Modified Duration
- `approxConvexity` -- Convexity (price-yield curve curvature)
- `pv1b` -- Present Value of a Basis Point (dollar value of 0.01% move)
- `yv32` -- Yield Value of a 32nd

**Cash Flow Analytics:**
- Periodic, semiannual, and annual yield calculations
- Macaulay and Modified Duration
- Convexity in periods and years
- Average life
- After-tax yield and taxable equivalent yield
- Total interest/principal flows, interest on interest
- Total and dollar returns, capital gains/losses

**Redemption Info:**
- `redemptionType` -- `Maturity`, `Call`, or `Put`
- `redemptionDate` -- The date used for yield calculation
- `worstIndicator` -- Indicates if this is the yield-to-worst result

**Metadata:**
- `industryConventionAssumptions` -- Shows all defaults applied (tradeDate, settlementDate, dayCountBasis, periodsPerYear, eomRule, redemptionValue)

---

## REST API Endpoints

For direct API access (e.g., batch processing or non-MCP workflows), the following endpoints are available at `https://api.sscmfi.com`:

### Authenticated Endpoints (require `X-API-KEY` header)

| Endpoint | Purpose |
|----------|---------|
| `POST /api/sscmfiAPICalculate` | Primary calculation engine -- all payment types |
| `POST /api/sscmfiAPIBatchCalculate` | Batch processing (up to 1000 securities per request) |
| `POST /api/sscmfiAPICalculatePeriodic` | Periodic bond calculations |
| `POST /api/sscmfiAPICalculateDiscount` | Discount security calculations |
| `POST /api/sscmfiAPICalculateIAM` | Interest At Maturity calculations |
| `POST /api/sscmfiAPICalculateStepped` | Stepped coupon bond calculations |
| `POST /api/sscmfiAPICalculateMultistep` | Multi-step coupon bond calculations |
| `POST /api/sscmfiAPICalculatePIK` | Payment-in-Kind bond calculations |
| `POST /api/sscmfiAPICalculatePartPIK` | Part-PIK bond calculations |

### Public Endpoint (no API key, rate-limited to 10 req/min)

| Endpoint | Purpose |
|----------|---------|
| `POST /api/sscmfiPublicAPI` | Same functionality, rate-limited public access |

### Payment Types

| Type | Description |
|------|-------------|
| Periodic | Fixed periodic coupon payments (Treasuries, Corporates, Agencies, Munis) |
| Discount | Zero-coupon / discount securities |
| IAM | Interest At Maturity -- all interest paid at maturity |
| Stepped | Single coupon rate step change |
| Multistep | Multiple coupon rate changes over bond life |
| PIK | Payment-In-Kind -- interest paid in additional bond principal |
| Part-PIK | Combination cash and PIK coupon payments |

### API Request Structure

```json
{
  "metadata": {
    "calculationID": "optional-user-id"
  },
  "securityDefinition": {
    "paymentType": "Periodic",
    "securityType": "Corporate",
    "maturityDate": "12/15/2034",
    "couponRate": 5.5,
    "redemptionValue": 100.0,
    "dayCountBasis": "SSCM30/360",
    "eomRule": "Adjust",
    "periodsPerYear": "Semiannual",
    "callRedemptions": {
      "redemptionScheduleType": "Discrete no notification",
      "redemptionList": [
        {"date": "12/15/2029", "value": 102.0},
        {"date": "12/15/2031", "value": 101.0}
      ]
    }
  },
  "tradeDefinition": {
    "settlementDate": "02/22/2026",
    "givenType": "Yield",
    "givenValue": 5.25
  },
  "calculationSelection": {
    "calcsToReturn": {
      "calcPY": "Yes",
      "calcPYAnalytics": "Yes",
      "calcCFS": "No",
      "calcCFSAnalytics": "No",
      "calcCouponPeriod": "Yes"
    },
    "calculationsFor": "Worst and maturity"
  }
}
```

### Calculation Selection Options

| Field | Values | Description |
|-------|--------|-------------|
| `calcPY` | Yes/No | Clean price, yield, accrued interest |
| `calcPYAnalytics` | Yes/No | Duration, convexity, PV01 |
| `calcCFS` | Yes/No | Detailed cash flow schedules |
| `calcCFSAnalytics` | Yes/No | Total returns, after-tax yield |
| `calcCouponPeriod` | Yes/No | Previous/next coupon dates |
| `calculationsFor` | Maturity only, Worst only, Worst and maturity, All redemptions | Which redemption scenarios to compute |

### Call Redemption Schedule Types

| Type | Description |
|------|-------------|
| Discrete no notification | Call dates with no notice period |
| Discrete with notification | Call dates requiring notice (specify `notifyMinDays`, `notifyMaxDays`) |
| Continuous with notification | Callable any time after first call date with notice |

### Day Count Conventions

| Convention | Typical Use |
|------------|-------------|
| SSCM30/360 | Corporate, Agency, Municipal bonds |
| Actual/Actual | US Treasury bonds |
| Actual/360 | CDs, money market instruments |
| Actual/365 | Some international bonds |

---

## CLI Usage

The SSCMFI public API can be called directly via curl for CLI workflows:

```bash
# Price-to-Yield calculation for a Corporate bond
curl -s -X POST https://api.sscmfi.com/api/sscmfiPublicAPI \
  -H "Content-Type: application/json" \
  -d '{
    "securityDefinition": {
      "paymentType": "Periodic",
      "securityType": "Corporate",
      "maturityDate": "12/15/2034",
      "couponRate": 5.5,
      "redemptionValue": 100.0,
      "dayCountBasis": "SSCM30/360",
      "eomRule": "Adjust",
      "periodsPerYear": "Semiannual"
    },
    "tradeDefinition": {
      "settlementDate": "02/22/2026",
      "givenType": "Price",
      "givenValue": 101.5
    },
    "calculationSelection": {
      "calcsToReturn": {
        "calcPY": "Yes",
        "calcPYAnalytics": "Yes",
        "calcCFS": "No",
        "calcCFSAnalytics": "No",
        "calcCouponPeriod": "Yes"
      },
      "calculationsFor": "Maturity only"
    }
  }'

# Yield-to-Worst on a callable Municipal bond
curl -s -X POST https://api.sscmfi.com/api/sscmfiPublicAPI \
  -H "Content-Type: application/json" \
  -d '{
    "securityDefinition": {
      "paymentType": "Periodic",
      "securityType": "Municipal",
      "maturityDate": "06/01/2040",
      "couponRate": 4.0,
      "redemptionValue": 100.0,
      "dayCountBasis": "SSCM30/360",
      "eomRule": "Adjust",
      "periodsPerYear": "Semiannual",
      "callRedemptions": {
        "redemptionScheduleType": "Discrete no notification",
        "redemptionList": [
          {"date": "06/01/2030", "value": 100.0},
          {"date": "06/01/2032", "value": 100.0}
        ]
      }
    },
    "tradeDefinition": {
      "settlementDate": "02/22/2026",
      "givenType": "Price",
      "givenValue": 105.0
    },
    "calculationSelection": {
      "calcsToReturn": {
        "calcPY": "Yes",
        "calcPYAnalytics": "Yes",
        "calcCFS": "No",
        "calcCFSAnalytics": "No",
        "calcCouponPeriod": "No"
      },
      "calculationsFor": "Worst and maturity"
    }
  }'

# Batch calculation (authenticated, requires API key)
curl -s -X POST https://api.sscmfi.com/api/sscmfiAPIBatchCalculate \
  -H "Content-Type: application/json" \
  -H "X-API-KEY: $SSCMFI_API_KEY" \
  -d '[
    { "securityDefinition": {...}, "tradeDefinition": {...}, "calculationSelection": {...} },
    { "securityDefinition": {...}, "tradeDefinition": {...}, "calculationSelection": {...} }
  ]'
```

---

## Usage Patterns & CFA Workflows

### Bond Pricing (Given Yield, Find Price)

1. Use `calculate_bond_periodic` with `givenType: "Yield"` and the target yield
2. Engine returns clean price, accrued interest, and dirty price
3. Check `metadata.industryConventionAssumptions` for the defaults applied

### Yield Calculation (Given Price, Find Yield)

1. Use `calculate_bond_periodic` with `givenType: "Price"` and the clean price
2. Engine returns yield-to-maturity (or yield-to-worst if call schedule provided)
3. Check `redemptionInfo.worstIndicator` to see if yield is to a call date

### Callable Bond Analysis (Yield-to-Worst)

1. Include `callSchedule` with all call dates and prices
2. Engine computes yield to every call date plus maturity
3. Returns the worst yield scenario automatically
4. Use `calculationsFor: "All redemptions"` to see every scenario

### Duration & Risk Analysis

1. Request `calcPYAnalytics: "Yes"` in the calculation selection
2. Returns Macaulay Duration, Modified Duration, Convexity, PV01, YV32
3. Use for portfolio immunisation, hedging, and risk management

### Cash Flow Schedule

1. Request `calcCFS: "Yes"` and `calcCFSAnalytics: "Yes"`
2. Returns full cash flow schedule with dates and amounts
3. Returns total returns, after-tax yields, interest on interest
4. Use for ALM, liability matching, and detailed portfolio analysis

### Integration with Corp Finance Tools

1. **Bond pricing** via SSCMFI -> feed yields into `wacc_calculator` cost of debt
2. **Duration/convexity** from SSCMFI -> use with corp-finance `bond_pricing` for portfolio analytics
3. **Yield curves** from SSCMFI -> compare with FMP `fmp_historical_price` data
4. **Credit spreads** from SSCMFI yields -> feed into `credit_metrics` analysis

---

## Error Handling

Errors return structured responses:

```json
{
  "success": false,
  "errorInfo": {
    "error": true,
    "errorNumber": 3000,
    "errorMessage": "Settlement Date is greater than Maturity Date"
  }
}
```

Common error codes:
- **3000**: Date validation errors (settlement > maturity, invalid dates)
- **5014**: Invalid date (e.g., February 30th)
- **6001**: Missing required field
- **6008**: No calculation selections specified
- **10001**: Wrong HTTP method (use POST)

---

## Important Notes

- **Dates**: Always use MM/DD/YYYY format
- **Rates as percentages**: 5% = `5.0`, not `0.05` (unlike corp-finance-mcp which uses decimals)
- **Prices per 100 par**: A bond at $1,015 per $1,000 face = price of 101.5
- **Public API rate limit**: 10 requests per minute (no auth required)
- **Authenticated API**: Requires `SSCMFI_API_KEY` environment variable and `X-API-KEY` header
- **Precision**: All calculations follow SSCM Volumes 1 & 2 institutional standards
- **Callable bonds**: Always check `redemptionInfo.redemptionType` and `worstIndicator` to distinguish yield-to-call from yield-to-maturity
