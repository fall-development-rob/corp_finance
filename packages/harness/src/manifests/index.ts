/**
 * Barrel export for Phase 36 YAML manifest infrastructure.
 */

export * from "./types.js";
export {
  createDirectYamlManifestLoader,
  type YamlManifestLoader,
  type YamlManifestLoaderOptions,
} from "./yaml-loader.js";

// REC-2: output_schema runtime validator
export {
  validateAgainstSchema,
  parseAndValidate,
  type SchemaValidationResult,
  type SchemaValidationError,
  type ValidatorOptions,
} from "./validator.js";

// REC-3: manifest static linter (Phase 40 Wave 4: + buildDefaultManifestPaths, buildAllTierManifestPaths)
export {
  checkManifests,
  buildDefaultManifestPaths,
  buildAllTierManifestPaths,
  type ManifestCheckIssue,
  type ManifestCheckReport,
  type CheckOptions,
} from "./checker.js";

// Phase 33 smoke gate: cookbook deployment loader
export {
  createCookbookLoader,
  type CookbookLoader,
  type LoadedCookbook,
  type CookbookLoaderOptions,
} from "./cookbook-loader.js";

// Phase 25 Tier A1: managed-agent tool-name catalog + cookbook lint
export {
  generateToolCatalog,
  serialiseCatalog,
  parseCatalog,
  lintCookbookToolNames,
  defaultServerSources,
  resolveRepoRoot,
  type ToolCatalog,
  type ServerSource,
  type GenerateInput,
  type LintInput,
  type CookbookLintIssue,
  type CookbookLintReport,
} from "./tool-catalog.js";

// Phase 25 Tier A2: managed-agent cookbook audit hashing
export {
  auditCookbook,
  auditAllCookbooks,
  serialiseAuditCatalog,
  parseAuditCatalog,
  diffAuditCatalogs,
  type CookbookAudit,
  type CookbookAuditCatalog,
  type FileEntry,
  type AuditCookbookInput,
  type AuditAllInput,
  type AuditDiff,
} from "./cookbook-audit.js";
