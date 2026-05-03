#!/usr/bin/env python3
# DEPRECATED: use crates/cfa-codegen instead. Will be removed in v0.3.
# Equivalent: `cargo run -p cfa-codegen -- examples`
"""Generate plugins/cfa-core/examples/<tool>.json for every cfa-core MCP tool.

Auto-generation walks the schemas.json manifest produced by gen-zod-schemas.py
and synthesises a minimum-valid input for each tool: required fields get
sensible defaults derived from their Rust type, optional fields are omitted.

The 11 priority tools (those with override descriptions in gen-mcp-tools.py)
get hand-written rich examples instead — see RICH_EXAMPLES below. These are
realistic financial scenarios pulled from textbook/CFA-curriculum problems
and are the inputs Claude should reach for when a user asks "what's a good
example of X?".

The auto-generated examples are placeholders — they parse and pass type
validation, but the numbers don't represent any meaningful financial scenario.
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
NAPI = REPO / "packages" / "bindings" / "src" / "lib.rs"
CORE_SRC = REPO / "crates" / "corp-finance-core" / "src"
EXAMPLES_DIR = REPO / "plugins" / "cfa-core" / "examples"
README = EXAMPLES_DIR / "README.md"

# Re-import the parser by running it inline. Rather than refactor schema
# extraction into a shared module, we just re-parse — this script runs in
# under a second.
sys.path.insert(0, str(REPO / "scripts"))
from importlib import import_module

gen_zod = import_module("gen-zod-schemas")  # type: ignore[import-not-found]


# ---------------------------------------------------------------------------
# Hand-written rich examples (priority 11 + a few institutional must-haves)
# ---------------------------------------------------------------------------

RICH_EXAMPLES: dict[str, dict] = {
    "calculate_wacc": {
        "risk_free_rate": "0.045",
        "equity_risk_premium": "0.055",
        "beta": "1.20",
        "cost_of_debt": "0.045",
        "tax_rate": "0.21",
        "debt_weight": "0.30",
        "equity_weight": "0.70",
    },
    "build_dcf": {
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
        "net_debt": "200000",
    },
    "comps_analysis": {
        "target": {
            "ticker": "TGT",
            "ev": "5000000",
            "revenue": "1200000",
            "ebitda": "300000",
            "earnings": "150000",
            "book_value": "800000",
        },
        "peers": [
            {
                "ticker": "PEER1",
                "ev": "4500000",
                "revenue": "1100000",
                "ebitda": "275000",
                "earnings": "140000",
                "book_value": "750000",
            },
            {
                "ticker": "PEER2",
                "ev": "5500000",
                "revenue": "1300000",
                "ebitda": "325000",
                "earnings": "160000",
                "book_value": "850000",
            },
        ],
    },
    "credit_metrics": {
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
        "capex": "100000",
    },
    "altman_zscore": {
        "working_capital": "150000",
        "total_assets": "3000000",
        "retained_earnings": "500000",
        "ebit": "400000",
        "revenue": "2000000",
        "total_liabilities": "1500000",
        "market_cap": "2500000",
        "is_public": True,
        "is_manufacturing": True,
    },
    "build_lbo": {
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
                "is_floating": False,
                "amortisation": "Bullet",
                "maturity_years": 7,
                "seniority": 1,
                "is_revolver": False,
            }
        ],
        "equity_contribution": "5000000",
        "exit_year": 5,
        "exit_multiple": "9.0",
        "transaction_fees": "200000",
        "financing_fees": "150000",
    },
    "price_bond": {
        "face_value": "1000",
        "coupon_rate": "0.05",
        "coupon_frequency": 2,
        "ytm": "0.045",
        "settlement_date": "2026-05-03",
        "maturity_date": "2036-05-03",
        "day_count": "Thirty360",
    },
    "price_option": {
        "spot_price": "100",
        "strike_price": "100",
        "time_to_expiry": "0.5",
        "risk_free_rate": "0.05",
        "volatility": "0.25",
        "dividend_yield": "0.02",
        "option_type": "Call",
        "exercise_style": "European",
    },
}


# ---------------------------------------------------------------------------
# Auto-generated placeholders by Rust type
# ---------------------------------------------------------------------------

DECIMAL_LIKE = {"Decimal", "Rate", "Money", "Multiple", "f64", "f32"}
INT_LIKE = {"u32", "u64", "u16", "u8", "i32", "i64", "i16", "i8", "usize", "isize"}
STRING_LIKE = {"String", "&str", "str"}
DATE_LIKE = {"NaiveDate", "chrono::NaiveDate"}


def placeholder(rust_type: str):
    """Return a sensible default value for a Rust type."""
    t = rust_type.strip()
    bare = t.split("::")[-1]

    if t.startswith("Option<"):
        # Optional fields are omitted entirely from examples.
        return None

    if bare in DECIMAL_LIKE:
        return "0.05"
    if bare in INT_LIKE:
        return 1
    if bare in STRING_LIKE:
        return "EXAMPLE"
    if bare == "bool":
        return False
    if bare in DATE_LIKE:
        return "2026-01-01"

    if t.startswith("Vec<"):
        # Single-element vec with the element's placeholder.
        inner = t[4:-1]
        elem = placeholder(inner)
        return [elem] if elem is not None else []

    if t.startswith("HashMap<") or t.startswith("BTreeMap<"):
        return {}

    if t.startswith("("):
        # Tuple — naive: pair of decimals.
        return ["EXAMPLE", "0.0"]

    # Unknown domain type — emit a placeholder string flagging the type
    # so users see they need to supply a real value.
    return f"<{bare}>"


def synth_example(fields: list[dict]) -> dict:
    """Build a placeholder input JSON from a list of parsed fields."""
    out: dict = {}
    for f in fields:
        # Skip optional fields (those with Option<T> or serde skip-if-none).
        if f["rust_type"].startswith("Option<") or f["is_serde_skip_none"]:
            continue
        val = placeholder(f["rust_type"])
        if val is not None:
            out[f["name"]] = val
    return out


def main() -> int:
    EXAMPLES_DIR.mkdir(parents=True, exist_ok=True)

    napi = gen_zod.load_napi_inputs()
    structs = gen_zod.index_structs()

    written = 0
    rich = 0
    skipped: list[str] = []

    for tool, type_path in napi.items():
        struct_name = type_path.split("::")[-1]
        if tool in RICH_EXAMPLES:
            example = RICH_EXAMPLES[tool]
            rich += 1
        elif struct_name in structs and structs[struct_name]["fields"]:
            example = synth_example(structs[struct_name]["fields"])
        else:
            skipped.append(tool)
            continue

        path = EXAMPLES_DIR / f"{tool}.json"
        path.write_text(json.dumps(example, indent=2) + "\n")
        written += 1

    # README pointing users at the examples.
    README.write_text(
        "# cfa-core MCP tool examples\n\n"
        f"{written} of {len(napi)} tools have a sample input here. They live in "
        "this directory as `<tool_name>.json` and pair with the per-tool zod "
        "schemas in `mcp/src/schemas.ts`.\n\n"
        "## Two flavours\n\n"
        f"- **{rich} rich examples** — hand-written realistic financial scenarios "
        "(WACC for a typical mid-cap, 5-year DCF with 8→4% growth taper, 5x EBITDA "
        "LBO, ATM Black-Scholes call, etc.). These run end-to-end through the MCP "
        "tool and produce sensible outputs. Use them as starting points for your "
        "own analyses. See `scripts/gen-examples.py:RICH_EXAMPLES`.\n"
        f"- **{written - rich} auto-generated shape examples** — minimum-valid "
        "inputs derived from the Rust struct definitions. Optional fields are "
        "omitted; required fields get type-appropriate placeholders (`\"0.05\"` "
        "for decimals, `\"EXAMPLE\"` for strings, `1` for ints). These pass the "
        "zod type check but **most will fail Rust validation** — domain enums "
        "(TerminalMethod, OptionType) need real values and business invariants "
        "(settlement before maturity, weights summing to 1) need realistic "
        "numbers. Treat them as field-name discovery aids, not turnkey inputs.\n"
        f"- **{len(skipped)} tools without examples** — input struct couldn't be "
        "auto-extracted (enum-shaped, e.g. `MbsAnalyticsInput`). v0.3 enum-aware "
        "parser pass.\n\n"
        "## Usage\n\n"
        "```bash\n"
        "# In any Claude Code conversation:\n"
        "#   \"Run calculate_wacc with the example from "
        "plugins/cfa-core/examples/calculate_wacc.json\"\n"
        "# Claude reads the JSON and calls the MCP tool with it.\n"
        "\n"
        "# Or pipe directly via the CLI test harness:\n"
        "cat plugins/cfa-core/examples/build_lbo.json \\\n"
        "  | jq '{input: .}' \\\n"
        "  | scripts/mcp-call cfa-core build_lbo\n"
        "```\n\n"
        "## Regenerating\n\n"
        "```bash\n"
        "python3 scripts/gen-examples.py\n"
        "```\n\n"
        "Re-run after any change to `crates/corp-finance-core/src/` Input structs "
        "or after editing `RICH_EXAMPLES` in the script.\n"
    )

    print(f"Wrote {written} examples to {EXAMPLES_DIR.relative_to(REPO)}")
    print(f"  Hand-written rich : {rich}")
    print(f"  Auto-generated    : {written - rich}")
    print(f"  Skipped (no input): {len(skipped)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
