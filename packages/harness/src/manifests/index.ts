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
