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

// REC-3: manifest static linter
export {
  checkManifests,
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
