#!/usr/bin/env python3
"""Generate crates/corp-finance-wasm/src/lib.rs from packages/bindings/src/lib.rs.

The NAPI bindings file is the source of truth for which corp-finance-core
functions are exposed at the language boundary. Each NAPI binding has the
shape:

    #[napi]
    pub fn <name>(input_json: String) -> NapiResult<String> {
        let input: <InputType> =
            serde_json::from_str(&input_json).map_err(to_napi_error)?;
        let output = <function>(&input).map_err(to_napi_error)?;
        serde_json::to_string(&output).map_err(to_napi_error)
    }

We transform each into the equivalent wasm_tool! macro invocation:

    wasm_tool!(<name>, <InputType>, <function>);

Section headers (// ----- ... ----- comment blocks) are preserved verbatim so
the WASM file's structure mirrors the NAPI file.
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
NAPI = REPO / "packages" / "bindings" / "src" / "lib.rs"
WASM = REPO / "crates" / "corp-finance-wasm" / "src" / "lib.rs"

HEADER = """//! WebAssembly bindings for corp-finance-core.
//!
//! GENERATED FROM packages/bindings/src/lib.rs by scripts/gen-wasm-bindings.py.
//! Do not edit by hand — re-run the generator if NAPI bindings change.
//!
//! Each exported function takes a JSON string (matching the corresponding
//! `*Input` struct in corp-finance-core) and returns a JSON string with the
//! computed `*Output`. The `wasm_tool!` macro keeps each binding to one line.

use wasm_bindgen::prelude::*;

fn err_to_js(e: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&e.to_string())
}

macro_rules! wasm_tool {
    ($name:ident, $input_path:path, $fn_path:path) => {
        #[wasm_bindgen]
        pub fn $name(input_json: &str) -> Result<String, JsValue> {
            let input: $input_path = serde_json::from_str(input_json).map_err(err_to_js)?;
            let output = $fn_path(&input).map_err(err_to_js)?;
            serde_json::to_string(&output).map_err(err_to_js)
        }
    };
}

"""

FOOTER = """
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
"""

# Regex: capture "pub fn NAME(" then "let input: TYPE =" then "let output = FN(&input)".
# Whitespace between tokens is permissive — NAPI bindings vary in line wrapping
# (single-line, broken before `serde_json`, broken before `.map_err`, etc.)
# but the token sequence is identical.
NAPI_RE = re.compile(
    r"#\[napi\]\s+"
    r"pub\s+fn\s+(\w+)\s*\(\s*input_json\s*:\s*String\s*\)\s*->\s*NapiResult<String>\s*\{\s*"
    r"let\s+input\s*:\s*([\w:]+)\s*=\s*"
    r"serde_json::from_str\(\s*&input_json\s*\)\s*\.map_err\(\s*to_napi_error\s*\)\s*\?\s*;\s*"
    r"let\s+output\s*=\s*"
    # Function path may wrap onto next line; (&input) may also be (\n &input,\n).
    r"([\w:]+)\s*\(\s*&input\s*,?\s*\)\s*\.map_err\(\s*to_napi_error\s*\)\s*\?\s*;\s*"
    r"serde_json::to_string\(\s*&output\s*\)\s*\.map_err\(\s*to_napi_error\s*\)\s*"
    r"\}",
    re.DOTALL,
)

SECTION_RE = re.compile(
    r"// -+\s*\n// ([^\n]+)\s*\n// -+",
    re.MULTILINE,
)


def main() -> int:
    src = NAPI.read_text()

    # Find every binding and every section header in document order.
    matches: list[tuple[int, str, tuple]] = []
    for m in NAPI_RE.finditer(src):
        matches.append((m.start(), "tool", (m.group(1), m.group(2), m.group(3))))
    for m in SECTION_RE.finditer(src):
        matches.append((m.start(), "section", (m.group(1).strip(),)))
    matches.sort(key=lambda t: t[0])

    out: list[str] = [HEADER]
    tool_count = 0
    section_count = 0

    for _, kind, payload in matches:
        if kind == "section":
            (title,) = payload
            out.append(f"\n// {'-' * 75}\n// {title}\n// {'-' * 75}\n\n")
            section_count += 1
        else:
            name, input_type, fn_path = payload
            out.append(f"wasm_tool!({name}, {input_type}, {fn_path});\n")
            tool_count += 1

    out.append(FOOTER)
    WASM.write_text("".join(out))

    print(f"Wrote {WASM.relative_to(REPO)}")
    print(f"  Tools  : {tool_count}")
    print(f"  Sections: {section_count}")

    expected = sum(1 for _ in NAPI_RE.finditer(src))
    if tool_count != expected:
        print(
            f"WARNING: regex matched {expected} bindings but emitted {tool_count}",
            file=sys.stderr,
        )
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
