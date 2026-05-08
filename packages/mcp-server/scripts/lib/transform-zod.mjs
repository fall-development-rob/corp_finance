#!/usr/bin/env node
/**
 * Shared transform helper for gen-*-schemas.mjs scripts.
 * Generates Zod TypeScript schemas from JSON Schema files produced by cargo test.
 */
import { execSync } from "node:child_process";
import { readdirSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join, basename } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = fileURLToPath(new URL(".", import.meta.url));
const MCP_SERVER_ROOT = join(__dirname, "..", "..");

const SENTINEL = "z.any().superRefine((x, ctx) => {\n    const schemas = ";
const END_MARKER = "\n  })";

/**
 * Extract the literal key used as discriminator from a z.object block.
 * Returns null if the object has no z.literal field.
 * @param {string} objSrc - The z.object({...}) source text
 * @returns {{ key: string, value: string } | null}
 */
function extractLiteralKey(objSrc) {
  // Match "fieldName": z.literal("value") inside the object
  const m = objSrc.match(/"(\w+)":\s*z\.literal\("([^"]*)"\)/);
  return m ? { key: m[1], value: m[2] } : null;
}

/**
 * Auto-detect the discriminator field by intersecting literal keys across all variants.
 * Returns the field name string, or null if detection fails.
 * @param {string[]} variants - Array of z.object(...) source strings
 * @returns {string | null}
 */
function detectDiscriminator(variants) {
  /** @type {Map<string, number>} key -> count of variants that have a literal for it */
  const counts = new Map();
  for (const v of variants) {
    const hit = extractLiteralKey(v);
    if (hit) {
      counts.set(hit.key, (counts.get(hit.key) ?? 0) + 1);
    }
  }
  const common = [...counts.entries()].filter(([, c]) => c === variants.length).map(([k]) => k);
  return common.length === 1 ? common[0] : null;
}

/**
 * Rewrite a single superRefine block into the appropriate Zod union form.
 * Three patterns:
 *   1. String enum  → z.enum([...])
 *   2. Kind-tagged ADT → z.discriminatedUnion("kind", [...])
 *   3. Custom-discriminator ADT → z.discriminatedUnion("<field>", [...])
 *   4. Fallback → keep original, warn
 * @param {string} schemasArr - The [...] array source (including brackets)
 * @param {string} describeStr - Optional trailing .describe("...") text
 * @returns {string}
 */
function rewriteUnion(schemasArr, describeStr) {
  const hasObject = schemasArr.includes("z.object(");
  const hasLiteral = schemasArr.includes("z.literal(");

  // Pattern 1: string enum — only z.literal members, no objects
  if (!hasObject && hasLiteral) {
    const allLiteral = [...schemasArr.matchAll(/z\.(\w+)\(/g)].every((m) => m[1] === "literal");
    if (allLiteral) {
      const values = [...schemasArr.matchAll(/z\.literal\("([^"]*)"\)/g)].map((m) => `"${m[1]}"`);
      return `z.enum([${values.join(", ")}])${describeStr}`;
    }
  }

  // Patterns 2 & 3: object-variant ADT — auto-detect discriminator
  if (hasObject) {
    // Split into individual z.object(...) blocks
    const variants = [...schemasArr.matchAll(/z\.object\(\{[^}]*\}\)/g)].map((m) => m[0]);
    if (variants.length > 0) {
      const disc = detectDiscriminator(variants);
      if (disc) {
        return `z.discriminatedUnion("${disc}", ${schemasArr})${describeStr}`;
      }
      // Detection failed — warn and preserve superRefine so the maintainer notices
      console.warn(
        `[transform-zod] Could not auto-detect discriminator for union:\n  ${schemasArr.slice(0, 120)}...\n  Keeping superRefine pattern.`
      );
      return `${SENTINEL}${schemasArr}${END_MARKER}${describeStr}`;
    }
  }

  // Fallback: mixed or literal-only union where allLiteral check failed — emit z.union([...])
  return `z.union(${schemasArr})${describeStr}`;
}

/**
 * Post-process all .ts files in outDir, rewriting superRefine oneOf blocks.
 * @param {string} outDir
 * @returns {number} count of files rewritten
 */
function postProcess(outDir) {
  const tsFiles = readdirSync(outDir).filter((f) => f.endsWith(".ts") && f !== "index.ts");
  let rewritten = 0;
  for (const file of tsFiles) {
    const filePath = join(outDir, file);
    let src = readFileSync(filePath, "utf8");
    if (!src.includes(SENTINEL)) continue;
    let next = src;
    while (next.includes(SENTINEL)) {
      const startIdx = next.indexOf(SENTINEL);
      const arrStart = startIdx + SENTINEL.length;
      const arrEnd = next.indexOf("];", arrStart);
      if (arrEnd === -1) break;
      const schemasArr = next.slice(arrStart, arrEnd + 1);
      const blockEnd = next.indexOf(END_MARKER, arrEnd);
      if (blockEnd === -1) break;
      const afterBlock = blockEnd + END_MARKER.length;
      const describeMatch = next.slice(afterBlock).match(/^\.describe\("[^"]*"\)/);
      const describeStr = describeMatch ? describeMatch[0] : "";
      const totalEnd = afterBlock + describeStr.length;
      const replacement = rewriteUnion(schemasArr, describeStr);
      next = next.slice(0, startIdx) + replacement + next.slice(totalEnd);
    }
    if (next !== src) {
      writeFileSync(filePath, next, "utf8");
      rewritten++;
    }
  }
  return rewritten;
}

/**
 * Full pipeline: generate Zod schemas from JSON Schema files, then post-process.
 * @param {{ domain: string, schemaSrc: string, schemaOut: string }} opts
 */
export function generateDomain({ domain, schemaSrc, schemaOut }) {
  mkdirSync(schemaOut, { recursive: true });
  const files = readdirSync(schemaSrc).filter((f) => f.endsWith(".json"));
  console.log(`[${domain}] Found ${files.length} JSON Schema files in ${schemaSrc}`);
  let generated = 0;
  for (const file of files) {
    const typeName = basename(file, ".json");
    const schemaName = `${typeName}Schema`;
    const inputPath = join(schemaSrc, file);
    const outputPath = join(schemaOut, `${typeName}.ts`);
    try {
      execSync(
        `npx json-schema-to-zod --zodVersion 3 -n ${schemaName} -i "${inputPath}" -o "${outputPath}"`,
        { cwd: MCP_SERVER_ROOT, stdio: "pipe" }
      );
      generated++;
    } catch (err) {
      console.error(`[${domain}] Failed to generate ${typeName}: ${err.message}`);
    }
  }
  console.log(`[${domain}] Generated ${generated}/${files.length} zod schema files in ${schemaOut}`);
  const rewritten = postProcess(schemaOut);
  console.log(`[${domain}] Rewrote ${rewritten} file(s): z.any().superRefine → discriminatedUnion/enum`);
}
