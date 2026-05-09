/**
 * Unit + integration tests for zod_transform.ts (Phase 30 Wave 1).
 *
 * Unit tests cover the four post-processing patterns by feeding known
 * superRefine strings to rewriteUnion directly.
 *
 * Integration test runs the full generateDomain pipeline against fixture
 * JSON Schema files in tests/fixtures/schemas/ and verifies the output.
 */

import { describe, it, expect, afterEach } from "vitest";
import {
  mkdtempSync,
  readdirSync,
  readFileSync,
  rmSync,
  copyFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

import {
  rewriteUnion,
  detectDiscriminator,
  extractLiteralKey,
  generateDomain,
} from "./zod_transform.js";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Build the superRefine preamble + array + closing that transform-zod.mjs produces. */
function makeSuperRefineBlock(schemasArr: string, describe_suffix = ""): string {
  return (
    `z.any().superRefine((x, ctx) => {\n    const schemas = ${schemasArr}\n  })${describe_suffix}`
  );
}

// ---------------------------------------------------------------------------
// Unit: extractLiteralKey
// ---------------------------------------------------------------------------

describe("extractLiteralKey", () => {
  it("returns key+value for a z.literal field", () => {
    const src = `z.object({ "kind": z.literal("text"), "value": z.string() })`;
    expect(extractLiteralKey(src)).toEqual({ key: "kind", value: "text" });
  });

  it("returns null when no z.literal field", () => {
    const src = `z.object({ "value": z.string() })`;
    expect(extractLiteralKey(src)).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// Unit: detectDiscriminator
// ---------------------------------------------------------------------------

describe("detectDiscriminator", () => {
  it("detects 'kind' when every variant has a kind literal", () => {
    const variants = [
      `z.object({ "kind": z.literal("a"), "x": z.string() })`,
      `z.object({ "kind": z.literal("b"), "y": z.number() })`,
    ];
    expect(detectDiscriminator(variants)).toBe("kind");
  });

  it("detects 'type' as custom discriminator", () => {
    const variants = [
      `z.object({ "type": z.literal("cash"), "amount": z.string() })`,
      `z.object({ "type": z.literal("equity"), "shares": z.number() })`,
    ];
    expect(detectDiscriminator(variants)).toBe("type");
  });

  it("returns null when literal fields differ across variants", () => {
    const variants = [
      `z.object({ "kind": z.literal("a") })`,
      `z.object({ "tag": z.literal("b") })`,
    ];
    expect(detectDiscriminator(variants)).toBeNull();
  });

  it("returns null for empty array", () => {
    expect(detectDiscriminator([])).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// Unit: rewriteUnion — Pattern 1: string enum
// ---------------------------------------------------------------------------

describe("rewriteUnion — Pattern 1: string enum", () => {
  it("rewrites all-literal array to z.enum", () => {
    const schemasArr = `[z.literal("a"), z.literal("b"), z.literal("c")]`;
    const { result, warn } = rewriteUnion(schemasArr, "");
    expect(warn).toBe(false);
    expect(result).toBe(`z.enum(["a", "b", "c"])`);
  });

  it("preserves .describe() suffix", () => {
    const schemasArr = `[z.literal("x"), z.literal("y")]`;
    const { result, warn } = rewriteUnion(schemasArr, `.describe("test")`);
    expect(warn).toBe(false);
    expect(result).toBe(`z.enum(["x", "y"]).describe("test")`);
  });
});

// ---------------------------------------------------------------------------
// Unit: rewriteUnion — Pattern 2: kind-tagged ADT
// ---------------------------------------------------------------------------

describe("rewriteUnion — Pattern 2: kind-tagged ADT", () => {
  it("rewrites to z.discriminatedUnion with 'kind'", () => {
    const schemasArr =
      `[z.object({ "kind": z.literal("text"), "value": z.string() }), ` +
      `z.object({ "kind": z.literal("number"), "value": z.number() })]`;
    const { result, warn } = rewriteUnion(schemasArr, "");
    expect(warn).toBe(false);
    expect(result).toBe(`z.discriminatedUnion("kind", ${schemasArr})`);
  });

  it("preserves .describe() suffix after discriminatedUnion", () => {
    const schemasArr =
      `[z.object({ "kind": z.literal("bool"), "value": z.boolean() }), ` +
      `z.object({ "kind": z.literal("empty") })]`;
    const { result } = rewriteUnion(schemasArr, `.describe("CellValue")`);
    expect(result).toContain(`.describe("CellValue")`);
    expect(result).toContain(`z.discriminatedUnion("kind"`);
  });
});

// ---------------------------------------------------------------------------
// Unit: rewriteUnion — Pattern 3: custom discriminator ADT
// ---------------------------------------------------------------------------

describe("rewriteUnion — Pattern 3: custom discriminator ADT", () => {
  it("rewrites to z.discriminatedUnion with 'type'", () => {
    const schemasArr =
      `[z.object({ "type": z.literal("cash"), "amount": z.string() }), ` +
      `z.object({ "type": z.literal("equity"), "shares": z.number() })]`;
    const { result, warn } = rewriteUnion(schemasArr, "");
    expect(warn).toBe(false);
    expect(result).toBe(`z.discriminatedUnion("type", ${schemasArr})`);
  });
});

// ---------------------------------------------------------------------------
// Unit: rewriteUnion — Pattern 4: fallback (undetectable discriminator)
// ---------------------------------------------------------------------------

describe("rewriteUnion — Pattern 4: mixed fallback", () => {
  it("keeps superRefine block and sets warn=true when discriminator undetectable", () => {
    // Objects with different literal field names → can't detect discriminator
    const schemasArr =
      `[z.object({ "kind": z.literal("a") }), z.object({ "tag": z.literal("b") })]`;
    const { result, warn } = rewriteUnion(schemasArr, "");
    expect(warn).toBe(true);
    // Must contain the SENTINEL prefix and END_MARKER
    expect(result).toContain("z.any().superRefine");
    expect(result).toContain(schemasArr);
  });

  it("preserves describe suffix even in fallback case", () => {
    const schemasArr =
      `[z.object({ "kind": z.literal("a") }), z.object({ "tag": z.literal("b") })]`;
    const { result } = rewriteUnion(schemasArr, `.describe("Foo")`);
    expect(result).toContain(`.describe("Foo")`);
  });
});

// ---------------------------------------------------------------------------
// Unit: rewriteUnion — mixed/fallback literal+object → z.union
// ---------------------------------------------------------------------------

describe("rewriteUnion — mixed literal+object fallback", () => {
  it("emits z.union for mixed literal+object with undetectable discriminator", () => {
    // hasObject=false, hasLiteral=true, but allLiteral=false (has z.string too)
    const schemasArr = `[z.literal("x"), z.string()]`;
    const { result, warn } = rewriteUnion(schemasArr, "");
    expect(warn).toBe(false);
    expect(result).toBe(`z.union(${schemasArr})`);
  });
});

// ---------------------------------------------------------------------------
// Integration: generateDomain against fixture files
// ---------------------------------------------------------------------------

describe("generateDomain — integration", () => {
  const tmpDirs: string[] = [];

  afterEach(() => {
    for (const d of tmpDirs) {
      rmSync(d, { recursive: true, force: true });
    }
    tmpDirs.length = 0;
  });

  it("generates .ts files for each fixture JSON Schema", () => {
    const outDir = mkdtempSync(join(tmpdir(), "zod-transform-test-"));
    tmpDirs.push(outDir);

    const fixtureDir = join(__dirname, "tests", "fixtures", "schemas");

    const report = generateDomain({
      domain: "test-fixture",
      schema_src: fixtureDir,
      schema_out: outDir,
    });

    expect(report.domain).toBe("test-fixture");
    expect(report.files_found).toBe(3);
    expect(report.files_generated).toBe(3);
    // Fixture schemas are simple enums/objects — no superRefine blocks expected
    expect(report.files_post_processed).toBe(0);
    expect(report.errors).toHaveLength(0);

    const generated = readdirSync(outDir).filter((f) => f.endsWith(".ts"));
    expect(generated).toContain("Color.ts");
    expect(generated).toContain("Amount.ts");
    expect(generated).toContain("Status.ts");
  });

  it("generated .ts files import zod and export a schema", () => {
    const outDir = mkdtempSync(join(tmpdir(), "zod-transform-test-"));
    tmpDirs.push(outDir);

    const fixtureDir = join(__dirname, "tests", "fixtures", "schemas");
    generateDomain({
      domain: "test-fixture",
      schema_src: fixtureDir,
      schema_out: outDir,
    });

    const colorTs = readFileSync(join(outDir, "Color.ts"), "utf8");
    expect(colorTs).toContain(`import { z } from "zod"`);
    expect(colorTs).toContain("ColorSchema");
  });

  it("reports error and continues when schema_src does not exist", () => {
    const outDir = mkdtempSync(join(tmpdir(), "zod-transform-test-"));
    tmpDirs.push(outDir);

    const report = generateDomain({
      domain: "bad-domain",
      schema_src: "/nonexistent/path/schemas",
      schema_out: outDir,
    });

    expect(report.files_found).toBe(0);
    expect(report.errors.length).toBeGreaterThan(0);
    expect(report.errors[0]).toContain("bad-domain");
  });
});
