/**
 * Semver validator — Phase 25 Tier C2.
 *
 * Tiny pure validator for cookbook version fields. Implements the strict
 * subset of semver-2.0 we use: MAJOR.MINOR.PATCH with optional prerelease
 * (e.g. "1.0.0-beta.1") and optional build metadata (e.g. "1.0.0+exp.42").
 *
 * No external dependency — we control the rule and don't need the full
 * compare/sort surface of npm:semver.
 */

// Reference: semver.org 2.0.0
// MAJOR.MINOR.PATCH where each is a non-negative integer with no leading zero.
// Optional `-PRERELEASE` and `+BUILDMETA` suffixes; each is a dot-separated
// series of identifiers matching [0-9A-Za-z-]+.
const SEMVER_RE =
  /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*))?(?:\+([0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?$/;

export interface ParsedSemver {
  major: number;
  minor: number;
  patch: number;
  prerelease?: string;
  build?: string;
}

/**
 * Returns true if `s` is a syntactically valid semver-2.0 version string.
 * Examples:
 *   "1.0.0"             → true
 *   "0.0.1"             → true
 *   "1.0.0-beta.1"      → true
 *   "1.0.0+build.42"    → true
 *   "1.0.0-rc.1+a.b"    → true
 *   "1.0"               → false (missing patch)
 *   "01.0.0"            → false (leading zero)
 *   "1.0.0-"            → false (empty prerelease)
 */
export function isValidSemver(s: unknown): s is string {
  return typeof s === "string" && SEMVER_RE.test(s);
}

/**
 * Parses a semver string. Returns null when the input is not a valid
 * semver-2.0 string. Does not throw.
 */
export function parseSemver(s: string): ParsedSemver | null {
  const m = SEMVER_RE.exec(s);
  if (!m) return null;
  const major = Number(m[1]);
  const minor = Number(m[2]);
  const patch = Number(m[3]);
  return {
    major,
    minor,
    patch,
    ...(m[4] !== undefined ? { prerelease: m[4] } : {}),
    ...(m[5] !== undefined ? { build: m[5] } : {}),
  };
}
