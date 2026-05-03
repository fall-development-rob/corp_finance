# cfa-core MCP tool examples

243 of 244 tools have a sample input here. They live in this directory as `<tool_name>.json` and pair with the per-tool zod schemas in `mcp/src/schemas.ts`.

## Two flavours

- **8 rich examples** — hand-written realistic financial scenarios (WACC for a typical mid-cap, 5-year DCF with 8→4% growth taper, 5x EBITDA LBO, ATM Black-Scholes call, etc.). These run end-to-end through the MCP tool and produce sensible outputs. Use them as starting points for your own analyses. See `crates/cfa-codegen/src/data/examples.rs:RICH_EXAMPLES`.
- **235 auto-generated shape examples** — minimum-valid inputs derived from the Rust struct definitions. Optional fields are omitted; required fields get type-appropriate placeholders (`"0.05"` for decimals, `"EXAMPLE"` for strings, `1` for ints). These pass the zod type check but **most will fail Rust validation** — domain enums (TerminalMethod, OptionType) need real values and business invariants (settlement before maturity, weights summing to 1) need realistic numbers. Treat them as field-name discovery aids, not turnkey inputs.
- **1 tools without examples** — input struct couldn't be auto-extracted (enum-shaped variants without a single inner type).

## Usage

```bash
# In any Claude Code conversation:
#   "Run calculate_wacc with the example from plugins/cfa-core/examples/calculate_wacc.json"
# Claude reads the JSON and calls the MCP tool with it.

# Or pipe directly via the CLI test harness:
cat plugins/cfa-core/examples/build_lbo.json \
  | jq '{input: .}' \
  | scripts/mcp-call cfa-core build_lbo
```

## Regenerating

```bash
cargo run -p cfa-codegen -- examples
```

Re-run after any change to `crates/corp-finance-core/src/` Input structs or after editing `RICH_EXAMPLES` in `crates/cfa-codegen/src/data/examples.rs`.
