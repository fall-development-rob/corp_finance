#!/usr/bin/env python3
# DEPRECATED: use crates/cfa-codegen instead. Will be removed in v0.3.
# Equivalent: `cargo run -p cfa-codegen -- zod-schemas`
"""Generate plugins/cfa-core/mcp/src/schemas.ts from corp-finance-core source.

For each Input struct used by a NAPI binding, walks the Rust source, extracts
field name + type + Optional + doc-comment, and emits a zod schema. Inputs
without a parseable struct fall back to the v0.2 passthrough schema (so this
strictly improves the existing surface — never breaks it).

The Rust → zod type map covers the 25 types that account for >99% of fields
(per `grep` survey of the source). Unknown types fall through to z.any() with
a description noting the original Rust type.
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
NAPI = REPO / "packages" / "bindings" / "src" / "lib.rs"
CORE_SRC = REPO / "crates" / "corp-finance-core" / "src"
OUT = REPO / "plugins" / "cfa-core" / "mcp" / "src" / "schemas.ts"
MANIFEST = REPO / "plugins" / "cfa-core" / "mcp" / "src" / "schemas.json"

# Map (tool_name → fully::qualified::InputType) from NAPI source.
NAPI_RE = re.compile(
    r"#\[napi\]\s+"
    r"pub\s+fn\s+(\w+)\s*\(\s*input_json\s*:\s*String\s*\)\s*->\s*NapiResult<String>\s*\{\s*"
    r"let\s+input\s*:\s*([\w:]+)\s*=",
    re.DOTALL,
)


def load_napi_inputs() -> dict[str, str]:
    src = NAPI.read_text()
    return {m.group(1): m.group(2) for m in NAPI_RE.finditer(src)}


# ---------------------------------------------------------------------------
# Rust struct parser
# ---------------------------------------------------------------------------

# Match `pub struct <Name>` and capture the brace-delimited body.
STRUCT_RE = re.compile(
    r"pub\s+struct\s+(\w+)\s*\{",
)

# Doc comment / attribute / `pub name:` prefix. We capture up to the colon and
# then walk the type by hand because Rust generic types can contain commas
# (e.g. Vec<(String, Decimal)>) that regex can't bracket-balance.
FIELD_PREFIX_RE = re.compile(
    r"""
    (?P<docs>(?:^[ \t]*///[^\n]*\n)*)                  # leading doc comments
    (?:^[ \t]*\#\[(?P<attr>[^\]]+)\][ \t]*\n)?         # optional serde attribute
    [ \t]*pub[ \t]+(?P<name>\w+)[ \t]*:[ \t]*          # `pub name:`
    """,
    re.MULTILINE | re.VERBOSE,
)


def parse_type_until_comma(body: str, start: int) -> tuple[str, int]:
    """Walk forward from `start`, returning the type expression up to the next
    top-level comma (one that's not inside angle brackets or parens) and the
    index just past that comma. If no comma is found, returns up to the end."""
    depth = 0
    i = start
    while i < len(body):
        ch = body[i]
        if ch in "<(":
            depth += 1
        elif ch in ">)":
            depth -= 1
        elif ch == "," and depth == 0:
            return body[start:i].strip(), i + 1
        i += 1
    return body[start:i].strip(), i


def find_brace_end(src: str, start: int) -> int:
    """Return index of matching `}` for the `{` at `start`. -1 if not found."""
    depth = 0
    i = start
    in_str = False
    while i < len(src):
        ch = src[i]
        if ch == '"' and (i == 0 or src[i - 1] != "\\"):
            in_str = not in_str
        elif not in_str:
            if ch == "{":
                depth += 1
            elif ch == "}":
                depth -= 1
                if depth == 0:
                    return i
        i += 1
    return -1


def parse_struct(src: str, start: int) -> tuple[str, list[dict], int] | None:
    """Parse one `pub struct ... { ... }` starting near `start`.

    Returns (struct_name, fields, end_index) or None.
    """
    m = STRUCT_RE.search(src, start)
    if not m:
        return None
    name = m.group(1)
    brace_open = src.find("{", m.end() - 1)
    if brace_open == -1:
        return None
    brace_close = find_brace_end(src, brace_open)
    if brace_close == -1:
        return None
    body = src[brace_open + 1 : brace_close]
    fields = []
    for fm in FIELD_PREFIX_RE.finditer(body):
        docs = " ".join(
            line.strip().lstrip("/").strip()
            for line in fm.group("docs").splitlines()
            if line.strip()
        )
        attr = fm.group("attr") or ""
        rust_type, _ = parse_type_until_comma(body, fm.end())
        fields.append(
            {
                "name": fm.group("name"),
                "rust_type": rust_type,
                "doc": docs,
                "is_serde_skip_none": "skip_serializing_if" in attr
                and "Option::is_none" in attr,
            }
        )
    return name, fields, brace_close


def index_structs() -> dict[str, dict]:
    """Return { struct_name: { module_path, fields } } for every struct in the
    corp-finance-core src tree."""
    result: dict[str, dict] = {}
    for path in CORE_SRC.rglob("*.rs"):
        text = path.read_text()
        # Strip module prefix to align with NAPI-style paths.
        rel = path.relative_to(CORE_SRC).with_suffix("")
        module_parts = list(rel.parts)
        if module_parts and module_parts[-1] == "mod":
            module_parts.pop()
        module_path = "::".join(["corp_finance_core", *module_parts])

        i = 0
        while True:
            parsed = parse_struct(text, i)
            if parsed is None:
                break
            name, fields, end = parsed
            # Record the *first* definition we see (subsequent duplicates from
            # other crates are extremely unlikely for `*Input` structs).
            result.setdefault(
                name,
                {
                    "module_path": module_path,
                    "fields": fields,
                },
            )
            i = end + 1
    return result


# ---------------------------------------------------------------------------
# Rust type → zod expression
# ---------------------------------------------------------------------------

DECIMAL_LIKE = {"Decimal", "Rate", "Money", "Multiple", "f64", "f32"}
INT_LIKE = {"u32", "u64", "u16", "u8", "i32", "i64", "i16", "i8", "usize", "isize"}
STRING_LIKE = {"String", "&str", "str"}
BOOL_LIKE = {"bool"}
DATE_LIKE = {"NaiveDate", "chrono::NaiveDate", "DateTime", "chrono::DateTime"}


def rust_to_zod(rust_type: str) -> str:
    """Map a Rust type expression to a zod schema expression."""
    t = rust_type.strip()

    # Strip qualifying paths e.g. `chrono::NaiveDate` → `NaiveDate` for the lookup,
    # but keep the original for error messages.
    bare = t.split("::")[-1]

    if bare in DECIMAL_LIKE:
        return "decimalLike"
    if bare in INT_LIKE:
        return "z.number().int()"
    if bare in STRING_LIKE:
        return "z.string()"
    if bare in BOOL_LIKE:
        return "z.boolean()"
    if bare in DATE_LIKE:
        return 'z.string().describe("ISO-8601 date")'

    if t.startswith("Option<") and t.endswith(">"):
        inner = t[len("Option<") : -1]
        return f"{rust_to_zod(inner)}.optional()"

    if t.startswith("Vec<") and t.endswith(">"):
        inner = t[len("Vec<") : -1]
        return f"z.array({rust_to_zod(inner)})"

    if t.startswith("HashMap<") and t.endswith(">"):
        # HashMap<String, V> → z.record(rust_to_zod(V))
        inner = t[len("HashMap<") : -1]
        parts = split_top_level_comma(inner)
        if len(parts) == 2 and parts[0].strip() in STRING_LIKE:
            return f"z.record({rust_to_zod(parts[1].strip())})"
        return f'z.record(z.any()).describe("HashMap<{inner}>")'

    if t.startswith("BTreeMap<") and t.endswith(">"):
        inner = t[len("BTreeMap<") : -1]
        parts = split_top_level_comma(inner)
        if len(parts) == 2 and parts[0].strip() in STRING_LIKE:
            return f"z.record({rust_to_zod(parts[1].strip())})"
        return f'z.record(z.any()).describe("BTreeMap<{inner}>")'

    if t.startswith("(") and t.endswith(")"):
        # Tuple type
        inner = t[1:-1]
        parts = split_top_level_comma(inner)
        if parts:
            tuple_items = ", ".join(rust_to_zod(p.strip()) for p in parts)
            return f"z.tuple([{tuple_items}])"

    # Unknown / domain type → escape hatch with original type in description so
    # Claude can still see the field name and the original Rust type.
    safe = t.replace("`", "'")
    return f'z.any().describe("Rust type: {safe}")'


def split_top_level_comma(s: str) -> list[str]:
    """Split on commas not inside angle brackets/parens."""
    parts = []
    depth = 0
    cur = []
    for ch in s:
        if ch in "<(":
            depth += 1
        elif ch in ">)":
            depth -= 1
        if ch == "," and depth == 0:
            parts.append("".join(cur))
            cur = []
        else:
            cur.append(ch)
    if cur:
        parts.append("".join(cur))
    return parts


# ---------------------------------------------------------------------------
# Emit
# ---------------------------------------------------------------------------

HEADER = """/**
 * cfa-core MCP tool input schemas.
 *
 * GENERATED FROM crates/corp-finance-core/src/ via scripts/gen-zod-schemas.py.
 * Do not edit by hand — re-run the generator if Input structs change.
 *
 * Each entry is a zod schema for one MCP tool's input. Tools without an
 * extractable schema fall back to the passthrough record schema in tools.ts.
 */
import { z } from "zod";

// Decimal-like financial values: rust_decimal::Decimal serialises to a string
// via the `serde-with-str` feature, but we accept numbers for ergonomics.
const decimalLike = z
  .union([z.string(), z.number()])
  .describe("Decimal value (string preferred for full precision; number accepted)");

export type ToolSchema = z.ZodObject<Record<string, z.ZodTypeAny>>;

export const TOOL_SCHEMAS: Record<string, ToolSchema> = {
"""

FOOTER = "};\n"


def emit_schema(tool: str, struct_name: str, fields: list[dict]) -> str:
    """Emit a single `<tool>: z.object({ ... }),` entry."""
    lines = [f"  {tool}: z.object({{"]
    for f in fields:
        zod = rust_to_zod(f["rust_type"])
        # If field is Optional<T> with serde skip-if-none, the .optional() is
        # already in the zod expression. If serde says skip_if_none but the type
        # isn't Option<T>, still mark it optional — the binding tolerates absence.
        if f["is_serde_skip_none"] and ".optional()" not in zod:
            zod = f"{zod}.optional()"
        # Doc comment becomes .describe(...). Collapse newlines so the resulting
        # string literal stays single-line for the TS parser.
        if f["doc"]:
            doc = (
                f["doc"]
                .replace("\\", "\\\\")
                .replace("`", "'")
                .replace('"', '\\"')
                .replace("\n", " ")
                .replace("\r", " ")
            )
            # Strip duplicate "Rust type: ..." describe added by the unknown-type
            # fallback — the doc comment is more useful.
            if zod.endswith(".optional()"):
                base = zod[: -len(".optional()")]
                if '.describe("Rust type:' in base:
                    base = re.sub(r'\.describe\("Rust type: [^"]+"\)', "", base)
                zod = f'{base}.describe("{doc}").optional()'
            else:
                if '.describe("Rust type:' in zod:
                    zod = re.sub(r'\.describe\("Rust type: [^"]+"\)', "", zod)
                zod = f'{zod}.describe("{doc}")'
        lines.append(f"    {f['name']}: {zod},")
    lines.append("  }),")
    return "\n".join(lines)


def main() -> int:
    napi_inputs = load_napi_inputs()
    print(f"NAPI bindings    : {len(napi_inputs)}")

    structs = index_structs()
    print(f"Input structs    : {sum(1 for n in structs if n.endswith('Input'))}")

    # Build a map struct_name → fully-qualified module path
    schemas: dict[str, dict] = {}
    missing = []
    for tool, type_path in napi_inputs.items():
        # type_path looks like corp_finance_core::credit::metrics::CreditMetricsInput
        struct_name = type_path.split("::")[-1]
        if struct_name not in structs:
            missing.append((tool, struct_name))
            continue
        fields = structs[struct_name]["fields"]
        if not fields:
            missing.append((tool, struct_name))
            continue
        schemas[tool] = {
            "struct": struct_name,
            "module_path": structs[struct_name]["module_path"],
            "fields": fields,
        }

    print(f"Schemas extracted: {len(schemas)} of {len(napi_inputs)}")
    print(f"Missing/empty    : {len(missing)}")
    for tool, struct_name in missing[:10]:
        print(f"  - {tool} (struct {struct_name})")
    if len(missing) > 10:
        print(f"  ... and {len(missing) - 10} more")

    out: list[str] = [HEADER]
    for tool in sorted(schemas):
        s = schemas[tool]
        out.append(emit_schema(tool, s["struct"], s["fields"]))
        out.append("\n")
    out.append(FOOTER)
    OUT.write_text("".join(out))

    # Also dump a JSON manifest so other tooling can introspect what's covered.
    # See plugins/cfa-core/docs/SCHEMA-VERSIONING.md for the bump policy —
    # this version is updated by hand when wire-format-affecting changes land.
    MANIFEST.write_text(
        json.dumps(
            {
                "schema_version": "1.0.0",
                "generated_from": str(NAPI.relative_to(REPO)),
                "tool_count": len(napi_inputs),
                "schema_count": len(schemas),
                "tools": sorted(schemas.keys()),
                "missing": [{"tool": t, "struct": s} for t, s in missing],
            },
            indent=2,
        )
        + "\n"
    )

    print(f"Wrote {OUT.relative_to(REPO)}")
    print(f"Wrote {MANIFEST.relative_to(REPO)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
