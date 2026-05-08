#!/usr/bin/env node
// Phase 29 Wave 14a — Generate zod schemas for the memory domain.
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { generateDomain } from "./lib/transform-zod.mjs";

const __dirname = fileURLToPath(new URL(".", import.meta.url));
// Phase 29 Wave 18: corp-finance-core extracted to /home/robert/corp-finance-core.
const COREREPO = "/home/robert/corp-finance-core";
generateDomain({
  domain: "memory",
  schemaSrc: join(COREREPO, "crates", "corp-finance-core", "target", "schemas", "memory"),
  schemaOut: join(__dirname, "..", "src", "schemas", "generated", "memory"),
});
