#!/usr/bin/env node
/**
 * Phase 29 Wave 13 — Generate zod schemas from exported JSON Schemas (cost domain).
 *
 * Reads every *.json from the cargo test output directory and emits a
 * corresponding .ts file into src/schemas/generated/cost/.
 *
 * Usage (via npm run schemas:gen:cost):
 *   node scripts/gen-cost-schemas.mjs
 */
import { execSync } from "node:child_process";
import { readdirSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join, basename } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = fileURLToPath(new URL(".", import.meta.url));
const WORKSPACE_ROOT = join(__dirname, "..", "..", "..");
const SCHEMA_SRC = join(WORKSPACE_ROOT, "crates", "corp-finance-core", "target", "schemas", "cost");
const SCHEMA_OUT = join(__dirname, "..", "src", "schemas", "generated", "cost");

mkdirSync(SCHEMA_OUT, { recursive: true });

const files = readdirSync(SCHEMA_SRC).filter((f) => f.endsWith(".json"));
console.log(`Found ${files.length} JSON Schema files in ${SCHEMA_SRC}`);

let generated = 0;
for (const file of files) {
  const typeName = basename(file, ".json");
  const schemaName = `${typeName}Schema`;
  const inputPath = join(SCHEMA_SRC, file);
  const outputPath = join(SCHEMA_OUT, `${typeName}.ts`);

  try {
    execSync(
      `npx json-schema-to-zod --zodVersion 3 -n ${schemaName} -i "${inputPath}" -o "${outputPath}"`,
      { cwd: join(__dirname, ".."), stdio: "pipe" }
    );
    generated++;
  } catch (err) {
    console.error(`Failed to generate ${typeName}: ${err.message}`);
  }
}

console.log(`Generated ${generated}/${files.length} zod schema files in ${SCHEMA_OUT}`);

// Post-processing transform: rewrite z.any().superRefine(oneOf[...]) to either:
//   - z.discriminatedUnion("status", [...]) when every variant is a z.object (tagged-object union)
//   - z.union([...]) when variants are z.literal (plain string enum)
// This handles both BudgetStatus (serde tag = "status") and string-const enums (Surface, TierTag, etc.)
let rewritten = 0;
const tsFiles = readdirSync(SCHEMA_OUT).filter((f) => f.endsWith(".ts") && f !== "index.ts");
for (const file of tsFiles) {
  const filePath = join(SCHEMA_OUT, file);
  let src = readFileSync(filePath, "utf8");
  const SENTINEL = "z.any().superRefine((x, ctx) => {\n    const schemas = ";
  const END_MARKER = "\n  })";
  if (!src.includes(SENTINEL)) {
    continue;
  }
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
    // Determine whether variants are objects (tagged union) or literals (string enum)
    const isTaggedObjectUnion = schemasArr.includes("z.object(");
    const replacement = isTaggedObjectUnion
      ? `z.discriminatedUnion("status", ${schemasArr})${describeStr}`
      : `z.union(${schemasArr})${describeStr}`;
    next = next.slice(0, startIdx) + replacement + next.slice(totalEnd);
  }
  if (next !== src) {
    writeFileSync(filePath, next, "utf8");
    rewritten++;
  }
}
console.log(`Rewrote ${rewritten} file(s): z.any().superRefine → z.discriminatedUnion("status",[...]) or z.union([...])`);
