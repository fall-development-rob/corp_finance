/**
 * Phase 30 Wave 1 — Zod schema transform pipeline as an agent-callable MCP tool.
 *
 * Ports packages/mcp-server/scripts/lib/transform-zod.mjs and the six thin
 * gen-<domain>-schemas.mjs wrappers into a single typed TypeScript module.
 *
 * COREREPO resolution order:
 *   1. process.env.COREREPO
 *   2. path.resolve(__dirname, "../../../../../../corp-finance-core")
 *      (sibling checkout relative to this file's compiled location in dist/)
 *
 * The full default schema_src for domain "office" is:
 *   ${schema_src_root}/crates/corp-finance-core/target/schemas/office
 * The full default schema_out for domain "office" is:
 *   ${schema_out_root}/office
 */

import { execFileSync } from "node:child_process";
import { readdirSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join, basename, resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

// ---------------------------------------------------------------------------
// Constants — preserved byte-for-byte from transform-zod.mjs
// ---------------------------------------------------------------------------

const SENTINEL = "z.any().superRefine((x, ctx) => {\n    const schemas = ";
const END_MARKER = "\n  })";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

/** Root of the mcp-server package (two dirs up from src/tools/agent_infrastructure). */
const MCP_SERVER_ROOT = resolve(__dirname, "../../..");

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

export type DomainKey =
  | "office"
  | "attest"
  | "memory"
  | "audit"
  | "cost"
  | "observability";

export interface DomainConfig {
  domain: string;
  schema_src: string;
  schema_out: string;
}

export interface DomainReport {
  domain: string;
  files_found: number;
  files_generated: number;
  files_post_processed: number;
  errors: string[];
}

export interface SchemasRefreshInput {
  domain: DomainKey | "all";
  schema_src_root?: string;
  schema_out_root?: string;
}

export interface SchemasRefreshReport {
  domain: string;
  reports: DomainReport[];
}

// ---------------------------------------------------------------------------
// Internal helpers — post-processing logic (exported for unit testing)
// ---------------------------------------------------------------------------

interface LiteralKey {
  key: string;
  value: string;
}

/** Extract the first z.literal field from a z.object({...}) source string. */
export function extractLiteralKey(objSrc: string): LiteralKey | null {
  const m = objSrc.match(/"(\w+)":\s*z\.literal\("([^"]*)"\)/);
  return m ? { key: m[1]!, value: m[2]! } : null;
}

/**
 * Auto-detect the discriminator field by intersecting literal keys across all variants.
 * Returns the field name if exactly one key appears in every variant, else null.
 */
export function detectDiscriminator(variants: string[]): string | null {
  const counts = new Map<string, number>();
  for (const v of variants) {
    const hit = extractLiteralKey(v);
    if (hit) {
      counts.set(hit.key, (counts.get(hit.key) ?? 0) + 1);
    }
  }
  const common = [...counts.entries()]
    .filter(([, c]) => c === variants.length)
    .map(([k]) => k);
  return common.length === 1 ? common[0]! : null;
}

/**
 * Rewrite a single superRefine block into the appropriate Zod union form.
 *
 * Pattern 1 — string enum:   all members are z.literal, no objects
 *             → z.enum([...])
 * Pattern 2/3 — object ADT: auto-detect discriminator
 *             → z.discriminatedUnion("<field>", [...])
 * Pattern 4 — fallback:     keep original superRefine, record error
 *             → original block returned unchanged (caller adds to errors)
 *
 * Returns { result, warn } where warn is truthy when the block was kept as-is.
 */
export function rewriteUnion(
  schemasArr: string,
  describeStr: string
): { result: string; warn: boolean } {
  const hasObject = schemasArr.includes("z.object(");
  const hasLiteral = schemasArr.includes("z.literal(");

  // Pattern 1: string enum — only z.literal members, no objects
  if (!hasObject && hasLiteral) {
    const allLiteral = [...schemasArr.matchAll(/z\.(\w+)\(/g)].every(
      (m) => m[1] === "literal"
    );
    if (allLiteral) {
      const values = [...schemasArr.matchAll(/z\.literal\("([^"]*)"\)/g)].map(
        (m) => `"${m[1]}"`
      );
      return { result: `z.enum([${values.join(", ")}])${describeStr}`, warn: false };
    }
  }

  // Patterns 2 & 3: object-variant ADT — auto-detect discriminator
  if (hasObject) {
    const variants = [...schemasArr.matchAll(/z\.object\(\{[^}]*\}\)/g)].map(
      (m) => m[0]
    );
    if (variants.length > 0) {
      const disc = detectDiscriminator(variants);
      if (disc) {
        return {
          result: `z.discriminatedUnion("${disc}", ${schemasArr})${describeStr}`,
          warn: false,
        };
      }
      // Detection failed — preserve superRefine so the maintainer notices
      return {
        result: `${SENTINEL}${schemasArr}${END_MARKER}${describeStr}`,
        warn: true,
      };
    }
  }

  // Fallback: mixed/literal union where allLiteral check failed → z.union
  return { result: `z.union(${schemasArr})${describeStr}`, warn: false };
}

/**
 * Post-process all .ts files in outDir, rewriting superRefine oneOf blocks.
 * Returns { count, errors } where errors lists per-file warnings.
 */
function postProcess(
  outDir: string
): { count: number; errors: string[] } {
  const tsFiles = readdirSync(outDir).filter(
    (f) => f.endsWith(".ts") && f !== "index.ts"
  );
  let count = 0;
  const errors: string[] = [];

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
      const { result, warn } = rewriteUnion(schemasArr, describeStr);
      if (warn) {
        errors.push(
          `[transform-zod] ${file}: Could not auto-detect discriminator for union: ${schemasArr.slice(0, 120)}...`
        );
      }
      next = next.slice(0, startIdx) + result + next.slice(totalEnd);
    }

    if (next !== src) {
      writeFileSync(filePath, next, "utf8");
      count++;
    }
  }

  return { count, errors };
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Full pipeline for one domain: generate Zod schemas from JSON Schema files,
 * then post-process. Returns a structured DomainReport — never throws on
 * per-file errors; collects them in the errors array.
 */
export function generateDomain(config: DomainConfig): DomainReport {
  const { domain, schema_src, schema_out } = config;
  const errors: string[] = [];

  mkdirSync(schema_out, { recursive: true });

  let files: string[];
  try {
    files = readdirSync(schema_src).filter((f) => f.endsWith(".json"));
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    return {
      domain,
      files_found: 0,
      files_generated: 0,
      files_post_processed: 0,
      errors: [`[${domain}] Cannot read schema_src ${schema_src}: ${msg}`],
    };
  }

  let generated = 0;
  for (const file of files) {
    const typeName = basename(file, ".json");
    const schemaName = `${typeName}Schema`;
    const inputPath = join(schema_src, file);
    const outputPath = join(schema_out, `${typeName}.ts`);
    try {
      execFileSync(
        "npx",
        [
          "json-schema-to-zod",
          "--zodVersion",
          "3",
          "-n",
          schemaName,
          "-i",
          inputPath,
          "-o",
          outputPath,
        ],
        { cwd: MCP_SERVER_ROOT, stdio: "pipe" }
      );
      generated++;
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      errors.push(`[${domain}] Failed to generate ${typeName}: ${msg}`);
    }
  }

  const { count: filesPostProcessed, errors: ppErrors } = postProcess(schema_out);
  errors.push(...ppErrors);

  return {
    domain,
    files_found: files.length,
    files_generated: generated,
    files_post_processed: filesPostProcessed,
    errors,
  };
}

// ---------------------------------------------------------------------------
// Convenience: schemasRefresh (multi-domain)
// ---------------------------------------------------------------------------

const ALL_DOMAINS: DomainKey[] = [
  "office",
  "attest",
  "memory",
  "audit",
  "cost",
  "observability",
];

function resolveCorerepo(): string {
  return (
    process.env["COREREPO"] ??
    resolve(__dirname, "../../../../../../corp-finance-core")
  );
}

function buildConfig(
  domain: DomainKey,
  schemaSrcRoot: string,
  schemaOutRoot: string
): DomainConfig {
  return {
    domain,
    schema_src: join(
      schemaSrcRoot,
      "crates",
      "corp-finance-core",
      "target",
      "schemas",
      domain
    ),
    schema_out: join(schemaOutRoot, domain),
  };
}

/**
 * Convenience entry point: resolve standard paths and generate one or all domains.
 */
export function schemasRefresh(input: SchemasRefreshInput): SchemasRefreshReport {
  const coreRepo = resolveCorerepo();
  const schemaSrcRoot = input.schema_src_root ?? coreRepo;
  const schemaOutRoot =
    input.schema_out_root ??
    resolve(MCP_SERVER_ROOT, "src", "schemas", "generated");

  const domains =
    input.domain === "all" ? ALL_DOMAINS : [input.domain as DomainKey];

  const reports: DomainReport[] = domains.map((d) =>
    generateDomain(buildConfig(d, schemaSrcRoot, schemaOutRoot))
  );

  return { domain: input.domain, reports };
}

/**
 * MCP tool wrapper: accepts JSON string input, returns JSON string output.
 * Used by the MCP layer to expose schemasRefresh as an agent-callable tool.
 */
export async function schemasRefreshMcp(inputJson: string): Promise<string> {
  const input = JSON.parse(inputJson) as SchemasRefreshInput;
  const report = schemasRefresh(input);
  return JSON.stringify(report);
}

// CLI invocation: `tsx zod_transform.ts <domain>` — same code path as the MCP
// tool, exposed for npm scripts and CI without introducing a separate script file.
if (import.meta.url === `file://${process.argv[1]}`) {
  const domain = (process.argv[2] ?? "all") as SchemasRefreshInput["domain"];
  const report = schemasRefresh({ domain });
  process.stdout.write(JSON.stringify(report, null, 2) + "\n");
  const hasErrors = report.reports.some((r) => r.errors.length > 0);
  process.exit(hasErrors ? 1 : 0);
}
