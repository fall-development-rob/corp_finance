# Wave-16 WASM Port Recipe

Mechanical checklist for porting one Rust module from NAPI-only to WASM-also in the cfa-core stack.
One module = one sub-wave commit. Complete all steps in order; do not skip ahead.

See ADR-027 for the strategic rationale and stop conditions.

---

## Prerequisites

- Rust toolchain with the `wasm32-unknown-unknown` target installed:
  ```
  rustup target add wasm32-unknown-unknown
  ```
- `wasm-pack` installed (0.12+):
  ```
  cargo install wasm-pack
  ```
- Node 20+ and the workspace `node_modules` populated (`npm install` at repo root).
- Identify the module you are porting: a single `mod` in `crates/corp-finance-core/src/` whose tools are listed in `.surface-allowlist.json` without a `wasm_blocker` reason.

---

## Step 1: Compile-check

Verify the module compiles to `wasm32-unknown-unknown` before writing any export code.

```bash
cargo build \
  --target wasm32-unknown-unknown \
  --release \
  -p corp-finance-wasm \
  --features <module-feature-flag>
```

Replace `<module-feature-flag>` with the Cargo feature that gates the module (check `crates/corp-finance-wasm/Cargo.toml`).

**Common blockers and mitigations:**

- `chrono` date/time functions: add the `wasmbind` feature to the `chrono` dependency in `crates/corp-finance-wasm/Cargo.toml`:
  ```toml
  chrono = { version = "...", features = ["wasmbind"] }
  ```
- `getrandom` (pulled in by UUID, rand, etc.): add the `js` feature:
  ```toml
  getrandom = { version = "...", features = ["js"] }
  ```
- `rusqlite` or any crate linking against `libsqlite3`: **hard stop**. SQLite cannot compile to `wasm32-unknown-unknown`. Declare a `wasm_blocker` in the allowlist and stop here.
- `std::fs`, `std::net`, or `mmap` usage inside the module: **hard stop** unless the call site can be feature-gated behind `#[cfg(not(target_arch = "wasm32"))]`.

If the build fails: apply a mitigation and retry. After **3 distinct compile attempts** with different mitigations, declare a structural blocker — see ADR-027 stop-condition protocol.

---

## Step 2: Add `wasm_bindgen` exports

Open `crates/corp-finance-wasm/src/lib.rs` (or the per-module subfile if one exists).

For each tool function in the module, add a `#[wasm_bindgen]` export. The canonical pattern mirrors the existing 6 tools:

```rust
use wasm_bindgen::prelude::*;
use corp_finance_core::<module>::<ToolInputType>;

#[wasm_bindgen]
pub fn <tool_name>(input: JsValue) -> Result<JsValue, JsValue> {
    let params: <ToolInputType> = serde_wasm_bindgen::from_value(input)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let result = corp_finance_core::<module>::<tool_fn>(params)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    serde_wasm_bindgen::to_value(&result)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}
```

Key rules:
- Input and output types must implement `serde::Serialize` + `serde::Deserialize`.
- Use `serde_wasm_bindgen` (already a dependency), not `serde_json`, for the JsValue boundary.
- Do not re-export the function under a different name from what the allowlist records — the allowlist entry must match the final `server.ts` tool name exactly.

---

## Step 3: Build the bundle

From the repo root:

```bash
wasm-pack build \
  crates/corp-finance-wasm \
  --target nodejs \
  --out-dir ../../plugins/cfa-core/wasm \
  --release
```

This overwrites `plugins/cfa-core/wasm/corp_finance_wasm.js`, `corp_finance_wasm_bg.wasm`, and the TypeScript typedefs. The `.wasm` bytes must be re-committed (they are checked in — see Common Pitfalls).

---

## Step 4: Register in the plugin server

Open `plugins/cfa-core/mcp/src/server.ts`.

For each newly exported function, add a `tool()` call inside `main()`, following the pattern of the existing 6 tools at lines 77–117:

```typescript
tool(
  server,
  "<tool_name>",
  "<One-sentence description matching the allowlist description field>",
  wasm.<tool_name>,
);
```

- The tool name string must match the key in `.surface-allowlist.json` exactly.
- The `wasm.<tool_name>` reference must match the `#[wasm_bindgen]` export name from Step 2.
- Insert in alphabetical order by tool name to keep diffs clean.

---

## Step 5: Update the WASM typedef

At the top of `plugins/cfa-core/mcp/src/server.ts`, the WASM module is imported and cast to a typed interface:

```typescript
import * as wasmRaw from "../wasm/corp_finance_wasm.js";
const wasm = wasmRaw as typeof wasmRaw & {
  calculate_wacc: (input: unknown) => unknown;
  build_dcf: (input: unknown) => unknown;
  // ... existing entries ...
};
```

Add each new export to this `as` cast:

```typescript
  <tool_name>: (input: unknown) => unknown;
```

Without this, TypeScript will error on `wasm.<tool_name>` in Step 4.

---

## Step 6: Remove from allowlist

Open `packages/mcp-server/.surface-allowlist.json`.

Delete the entry (or entries) for each tool you have just ported. The file must remain valid JSON and must stay sorted alphabetically by tool name key after your deletion (so that diff reviews are clean — see Common Pitfalls).

Do not delete the 34 permanent carve-outs (federation, memory, multi_agent, audit, cost, self_learning families) or any `wasm_blocker` entries from earlier sub-waves.

---

## Step 7: Verify

Run both checks from the repo root:

```bash
# Surface-parity audit must exit 0
npm run check:surface-parity

# Plugin server must boot without crashing
node plugins/cfa-core/mcp/dist/server.js < /dev/null
```

If `check:surface-parity` fails: a tool was deleted from the allowlist without being registered in `server.ts`, or a tool was registered in `server.ts` without a matching allowlist removal. Fix the discrepancy.

If the server crashes on boot: a `wasm.<fn>` reference in `server.ts` does not exist in the built bundle. Rebuild the bundle (Step 3) and verify the export name matches exactly.

Also run the full workspace build and test to confirm nothing regressed:

```bash
cargo test --workspace --all-features
npm test
```

---

## Step 8: Bundle-size check

```bash
ls -la plugins/cfa-core/wasm/*.wasm
```

Record the size before and after the port. If the `.wasm` file grew by more than **1 MB** in this sub-wave, note it in the commit body.

If total bundle size exceeds **5 MB**, stop before the next sub-wave and follow the ADR-027 bundle-size split evaluation protocol.

---

## Step 9: Commit

Use this exact message format:

```
feat(phase-29-wave-16x): port <module> to WASM (N tools)

Tools ported:
- tool_name_1
- tool_name_2
- ...

WASM bundle delta: +NNN KB (total: NNN KB)
```

- Replace `16x` with the actual sub-wave letter (16c, 16d, ...).
- Replace `<module>` with the Rust module name as it appears in `corp_finance_core`.
- List every tool name that was removed from the allowlist.
- Include the bundle delta line even if size is unchanged.

Do not combine multiple modules in a single commit. One module, one commit, one sub-wave tag.

---

## Common Pitfalls

**Checked-in `.wasm` bytes.** The built WASM binary at `plugins/cfa-core/wasm/corp_finance_wasm_bg.wasm` is committed to the repo. After `wasm-pack build`, stage the updated `.wasm` file along with the JS glue and TypeScript typedef files. It is easy to forget and commit only the source changes.

**Plugin manifest version bump.** `plugins/cfa-core/.claude-plugin/plugin.json` contains a `version` field. Bump the patch version for each sub-wave that changes the tool surface. Claude Code uses this version to detect plugin updates and reload the server.

**Allowlist must stay sorted.** `.surface-allowlist.json` is sorted alphabetically by tool name key. If your deletion leaves the file out of order, the next diff will look noisy and CI may flag it depending on the schema validator. Re-sort after any deletion.

**`serde_wasm_bindgen` vs `serde_json`.** The WASM boundary uses `JsValue`, not `String`. Do not use `serde_json::from_str` at the WASM export boundary — use `serde_wasm_bindgen::from_value`. Mixing the two causes silent type errors at runtime in Node.

**Feature gate completeness.** If the module is gated behind a Cargo feature in `corp-finance-core`, ensure that feature is also listed in `crates/corp-finance-wasm/Cargo.toml` under `[features]`. A missing feature gate causes the exported functions to compile but silently call stub implementations.

**TypeScript `as` cast drift.** After `wasm-pack build`, the generated `.d.ts` typedef in `plugins/cfa-core/wasm/` will include the new exports. The manual `as` cast in `server.ts` (Step 5) must include every export you call — but the cast does not fail if it is incomplete, it just makes TypeScript think the function exists when it may not. Keep the cast in sync with the actual `tool()` registrations.
