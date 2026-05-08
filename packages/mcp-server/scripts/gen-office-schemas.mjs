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
import { readdirSync, mkdirSync } from "node:fs";
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
