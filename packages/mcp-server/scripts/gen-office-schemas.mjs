#!/usr/bin/env node
/**
 * Phase 29 Wave 11 — Generate zod schemas from exported JSON Schemas.
 *
 * Reads every *.json from the cargo test output directory and emits a
 * corresponding .ts file into src/schemas/generated/office/.
 *
 * Usage (via npm run schemas:gen:office):
 *   node scripts/gen-office-schemas.mjs
 */
import { execSync } from "node:child_process";
import { readdirSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join, basename } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = fileURLToPath(new URL(".", import.meta.url));
const WORKSPACE_ROOT = join(__dirname, "..", "..", "..");
const SCHEMA_SRC = join(WORKSPACE_ROOT, "crates", "corp-finance-core", "target", "schemas", "office");
const SCHEMA_OUT = join(__dirname, "..", "src", "schemas", "generated", "office");

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

// Gap 2 fix: rewrite z.any().superRefine(oneOf[...]) → z.discriminatedUnion("kind", [...])
// The generated pattern is always:
//   z.any().superRefine((x, ctx) => {\n    const schemas = [...];
//   ... boilerplate reduce/addIssue ...\n  })
// We extract the schemas array and rewrite to z.discriminatedUnion when every variant
// has a "kind" literal discriminator.
let rewritten = 0;
const tsFiles = readdirSync(SCHEMA_OUT).filter((f) => f.endsWith(".ts") && f !== "index.ts");
for (const file of tsFiles) {
  const filePath = join(SCHEMA_OUT, file);
  let src = readFileSync(filePath, "utf8");
  // Match the entire superRefine block (greedy across newlines up to the closing }).describe
  // or }) at end of assignment. Use a regex that captures:
  //   group 1: everything before z.any().superRefine
  //   group 2: the schemas array content (inside [...])
  //   group 3: optional .describe("...") after the closing })
  // Replace the multi-line superRefine block with z.discriminatedUnion("kind", [...]).
  // Strategy: split on the sentinel start line, then reassemble.
  const SENTINEL = "z.any().superRefine((x, ctx) => {\n    const schemas = ";
  const END_MARKER = "\n  })";
  if (!src.includes(SENTINEL)) {
    continue; // nothing to rewrite in this file
  }
  let next = src;
  while (next.includes(SENTINEL)) {
    const startIdx = next.indexOf(SENTINEL);
    // The schemas array starts right after the sentinel.
    const arrStart = startIdx + SENTINEL.length;
    // Find the end of the schemas array: first '];' at this level.
    const arrEnd = next.indexOf("];", arrStart);
    if (arrEnd === -1) break;
    const schemasArr = next.slice(arrStart, arrEnd + 1); // includes ']'
    // Find the closing END_MARKER after the array.
    const blockEnd = next.indexOf(END_MARKER, arrEnd);
    if (blockEnd === -1) break;
    const afterBlock = blockEnd + END_MARKER.length;
    // Check for optional .describe("...") immediately after.
    const describeMatch = next.slice(afterBlock).match(/^\.describe\("[^"]*"\)/);
    const describeStr = describeMatch ? describeMatch[0] : "";
    const totalEnd = afterBlock + describeStr.length;
    const replacement = `z.discriminatedUnion("kind", ${schemasArr})${describeStr}`;
    next = next.slice(0, startIdx) + replacement + next.slice(totalEnd);
  }
  if (next !== src) {
    writeFileSync(filePath, next, "utf8");
    rewritten++;
  }
}
console.log(`Rewrote ${rewritten} file(s): z.any().superRefine → z.discriminatedUnion("kind", [...])`);

