#!/usr/bin/env node
// Phase 29 Wave 14a — Generate zod schemas for the cost domain.
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { generateDomain } from "./lib/transform-zod.mjs";

const __dirname = fileURLToPath(new URL(".", import.meta.url));
const WORKSPACE_ROOT = join(__dirname, "..", "..", "..");
generateDomain({
  domain: "cost",
  schemaSrc: join(WORKSPACE_ROOT, "crates", "corp-finance-core", "target", "schemas", "cost"),
  schemaOut: join(__dirname, "..", "src", "schemas", "generated", "cost"),
});
