/**
 * JSON Schema subset validator for output_schema enforcement — REC-2.
 *
 * Mirrors the role of upstream's scripts/validate.py (38 LOC Python) but
 * implemented in TypeScript with no external dependencies. Validates subagent
 * free-text output against an AgentManifest.output_schema before the result
 * crosses the trust boundary into the orchestrator.
 *
 * Supported schema keywords:
 *   type, required, additionalProperties, properties, items,
 *   maxItems, minItems, maxLength, minLength, pattern, enum, oneOf
 *
 * Unsupported keywords (allOf, anyOf, $ref, $defs, etc.) are silently
 * ignored — validation passes. With options.strict === true, unrecognised
 * keywords emit a console.warn.
 */

// ---------------------------------------------------------------------------
// Public surface
// ---------------------------------------------------------------------------

export interface SchemaValidationError {
  /** Dot / bracket notation path, e.g. "deliverable.risks[3].severity" */
  path: string;
  /** Human-readable error description. */
  message: string;
  /** The JSON Schema keyword that triggered this error, when applicable. */
  schemaKeyword?: string;
}

export interface SchemaValidationResult {
  ok: boolean;
  errors: SchemaValidationError[];
  /** The validated value, or undefined if validation failed. */
  value?: unknown;
}

export interface ValidatorOptions {
  /** When true, log console.warn for unsupported schema keywords. */
  strict?: boolean;
}

/**
 * Validates `value` against a JSON Schema subset covering the keywords our
 * cookbook output_schemas actually use. See module header for the full list.
 *
 * An empty schema `{}` always passes. Unknown keywords are ignored (or
 * warned about when strict === true).
 */
export function validateAgainstSchema(
  value: unknown,
  schema: Record<string, unknown>,
  options?: ValidatorOptions,
): SchemaValidationResult {
  const errors: SchemaValidationError[] = [];
  validateNode(value, schema, "", errors, options ?? {});
  return errors.length === 0
    ? { ok: true, errors: [], value }
    : { ok: false, errors };
}

/**
 * Convenience: JSON.parse the text first, then validate. Returns ok: false
 * with a descriptive error if the text is not valid JSON.
 */
export function parseAndValidate(
  text: string,
  schema: Record<string, unknown>,
  options?: ValidatorOptions,
): SchemaValidationResult {
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch (err) {
    return {
      ok: false,
      errors: [
        {
          path: "",
          message: `JSON parse error: ${err instanceof Error ? err.message : String(err)}`,
          schemaKeyword: undefined,
        },
      ],
    };
  }
  return validateAgainstSchema(parsed, schema, options);
}

// ---------------------------------------------------------------------------
// Regex cache — compile pattern strings once
// ---------------------------------------------------------------------------

const patternCache = new Map<string, RegExp>();

function getRegex(pattern: string): RegExp {
  let re = patternCache.get(pattern);
  if (!re) {
    re = new RegExp(pattern);
    patternCache.set(pattern, re);
  }
  return re;
}

// ---------------------------------------------------------------------------
// Known schema keywords (used for strict-mode unknown-keyword detection)
// ---------------------------------------------------------------------------

const KNOWN_KEYWORDS = new Set([
  "type",
  "required",
  "additionalProperties",
  "properties",
  "items",
  "maxItems",
  "minItems",
  "maxLength",
  "minLength",
  "pattern",
  "enum",
  "oneOf",
  // Meta-keywords we deliberately ignore but don't warn about
  "title",
  "description",
  "default",
  "examples",
  "$schema",
  "definitions",
]);

// ---------------------------------------------------------------------------
// Deep equality for enum comparison
// ---------------------------------------------------------------------------

function deepEqual(a: unknown, b: unknown): boolean {
  if (a === b) return true;
  if (typeof a !== typeof b) return false;
  if (a === null || b === null) return a === b;
  if (Array.isArray(a) && Array.isArray(b)) {
    if (a.length !== b.length) return false;
    return a.every((v, i) => deepEqual(v, b[i]));
  }
  if (typeof a === "object" && typeof b === "object") {
    const ao = a as Record<string, unknown>;
    const bo = b as Record<string, unknown>;
    const aKeys = Object.keys(ao);
    const bKeys = Object.keys(bo);
    if (aKeys.length !== bKeys.length) return false;
    return aKeys.every((k) => deepEqual(ao[k], bo[k]));
  }
  return false;
}

// ---------------------------------------------------------------------------
// Path helpers
// ---------------------------------------------------------------------------

function childPath(parent: string, key: string | number): string {
  if (typeof key === "number") {
    return `${parent}[${key}]`;
  }
  return parent === "" ? key : `${parent}.${key}`;
}

// ---------------------------------------------------------------------------
// Core recursive validator
// ---------------------------------------------------------------------------

function validateNode(
  value: unknown,
  schema: Record<string, unknown>,
  path: string,
  errors: SchemaValidationError[],
  options: ValidatorOptions,
): void {
  if (Object.keys(schema).length === 0) return;

  // Warn on unknown keywords in strict mode
  if (options.strict) {
    for (const key of Object.keys(schema)) {
      if (!KNOWN_KEYWORDS.has(key)) {
        console.warn(
          `[validator] Unsupported schema keyword "${key}" at path "${path || "."}" — ignored`,
        );
      }
    }
  }

  // type
  if ("type" in schema) {
    validateType(value, schema.type, path, errors);
  }

  // enum (short-circuit: if present, value must be one of the enum values)
  if ("enum" in schema) {
    const allowed = schema.enum as unknown[];
    if (!Array.isArray(allowed)) {
      errors.push({ path, message: 'schema "enum" must be an array', schemaKeyword: "enum" });
    } else if (!allowed.some((v) => deepEqual(value, v))) {
      const listed = allowed.map((v) => JSON.stringify(v)).join(", ");
      errors.push({
        path,
        message: `value ${JSON.stringify(value)} is not one of [${listed}]`,
        schemaKeyword: "enum",
      });
    }
    // enum is terminal — don't recurse into properties / items
    return;
  }

  // oneOf
  if ("oneOf" in schema) {
    const branches = schema.oneOf as unknown[];
    if (!Array.isArray(branches)) {
      errors.push({ path, message: 'schema "oneOf" must be an array', schemaKeyword: "oneOf" });
    } else {
      const matchCount = branches.reduce<number>((count, branch) => {
        const branchErrors: SchemaValidationError[] = [];
        validateNode(value, branch as Record<string, unknown>, path, branchErrors, options);
        return branchErrors.length === 0 ? count + 1 : count;
      }, 0);
      if (matchCount !== 1) {
        errors.push({
          path,
          message: `value must match exactly one of ${branches.length} oneOf schemas (matched ${matchCount})`,
          schemaKeyword: "oneOf",
        });
      }
    }
    return;
  }

  // String-specific keywords
  if (typeof value === "string") {
    validateString(value, schema, path, errors);
  }

  // Array-specific keywords
  if (Array.isArray(value)) {
    validateArray(value, schema, path, errors, options);
  }

  // Object-specific keywords
  if (isPlainObject(value)) {
    validateObject(value, schema, path, errors, options);
  }
}

// ---------------------------------------------------------------------------
// Type validation
// ---------------------------------------------------------------------------

function validateType(
  value: unknown,
  typeSpec: unknown,
  path: string,
  errors: SchemaValidationError[],
): void {
  const types = Array.isArray(typeSpec) ? typeSpec : [typeSpec];
  const matchesAny = types.some((t) => matchesType(value, t as string));
  if (!matchesAny) {
    const expected = types.join(" | ");
    errors.push({
      path,
      message: `expected type ${expected}, got ${jsTypeName(value)}`,
      schemaKeyword: "type",
    });
  }
}

function matchesType(value: unknown, type: string): boolean {
  switch (type) {
    case "object":   return isPlainObject(value);
    case "array":    return Array.isArray(value);
    case "string":   return typeof value === "string";
    case "number":   return typeof value === "number" && !Number.isNaN(value);
    case "integer":  return typeof value === "number" && Number.isInteger(value);
    case "boolean":  return typeof value === "boolean";
    case "null":     return value === null;
    default:         return false;
  }
}

function jsTypeName(value: unknown): string {
  if (value === null) return "null";
  if (Array.isArray(value)) return "array";
  return typeof value;
}

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

// ---------------------------------------------------------------------------
// String keyword validation
// ---------------------------------------------------------------------------

function validateString(
  value: string,
  schema: Record<string, unknown>,
  path: string,
  errors: SchemaValidationError[],
): void {
  if ("maxLength" in schema) {
    const max = schema.maxLength as number;
    if (value.length > max) {
      errors.push({
        path,
        message: `string length ${value.length} exceeds maxLength ${max}`,
        schemaKeyword: "maxLength",
      });
    }
  }

  if ("minLength" in schema) {
    const min = schema.minLength as number;
    if (value.length < min) {
      errors.push({
        path,
        message: `string length ${value.length} is below minLength ${min}`,
        schemaKeyword: "minLength",
      });
    }
  }

  if ("pattern" in schema) {
    const pat = schema.pattern as string;
    const re = getRegex(pat);
    if (!re.test(value)) {
      errors.push({
        path,
        message: `string does not match pattern ${pat}`,
        schemaKeyword: "pattern",
      });
    }
  }
}

// ---------------------------------------------------------------------------
// Array keyword validation
// ---------------------------------------------------------------------------

function validateArray(
  value: unknown[],
  schema: Record<string, unknown>,
  path: string,
  errors: SchemaValidationError[],
  options: ValidatorOptions,
): void {
  if ("maxItems" in schema) {
    const max = schema.maxItems as number;
    if (value.length > max) {
      errors.push({
        path,
        message: `array length ${value.length} exceeds maxItems ${max}`,
        schemaKeyword: "maxItems",
      });
    }
  }

  if ("minItems" in schema) {
    const min = schema.minItems as number;
    if (value.length < min) {
      errors.push({
        path,
        message: `array length ${value.length} is below minItems ${min}`,
        schemaKeyword: "minItems",
      });
    }
  }

  if ("items" in schema && isPlainObject(schema.items)) {
    const itemSchema = schema.items as Record<string, unknown>;
    for (let i = 0; i < value.length; i++) {
      validateNode(value[i], itemSchema, childPath(path, i), errors, options);
    }
  }
}

// ---------------------------------------------------------------------------
// Object keyword validation
// ---------------------------------------------------------------------------

function validateObject(
  value: Record<string, unknown>,
  schema: Record<string, unknown>,
  path: string,
  errors: SchemaValidationError[],
  options: ValidatorOptions,
): void {
  // required
  if ("required" in schema && Array.isArray(schema.required)) {
    for (const key of schema.required as string[]) {
      if (!(key in value)) {
        errors.push({
          path: childPath(path, key),
          message: `required property "${key}" is missing`,
          schemaKeyword: "required",
        });
      }
    }
  }

  // additionalProperties: false
  if (schema.additionalProperties === false && isPlainObject(schema.properties)) {
    const allowed = new Set(Object.keys(schema.properties as Record<string, unknown>));
    for (const key of Object.keys(value)) {
      if (!allowed.has(key)) {
        errors.push({
          path: childPath(path, key),
          message: `additional property "${key}" is not allowed`,
          schemaKeyword: "additionalProperties",
        });
      }
    }
  }

  // properties — recurse into each declared property
  if ("properties" in schema && isPlainObject(schema.properties)) {
    const props = schema.properties as Record<string, Record<string, unknown>>;
    for (const [key, propSchema] of Object.entries(props)) {
      if (key in value) {
        validateNode(value[key], propSchema, childPath(path, key), errors, options);
      }
    }
  }
}
