/**
 * Phase 25 Tier C2 — semver validator unit tests.
 *
 * Covers the strict subset of semver-2.0 used by managed-agent cookbook
 * version fields. Each `it()` exercises one rule from the spec.
 */

import { describe, it, expect } from "vitest";
import { isValidSemver, parseSemver } from "../src/manifests/semver.js";

describe("isValidSemver — accepts", () => {
  const ok = [
    "0.0.0",
    "1.0.0",
    "10.20.30",
    "1.2.3-beta",
    "1.2.3-beta.1",
    "1.0.0-rc.1",
    "1.0.0-alpha.0.0.0",
    "1.0.0+build.42",
    "1.0.0+exp.sha.5114f85",
    "1.0.0-rc.1+build.42",
    "0.0.1",
    "1.0.0-0",
    "999999999.999999999.999999999",
  ];
  for (const s of ok) {
    it(`accepts "${s}"`, () => {
      expect(isValidSemver(s)).toBe(true);
    });
  }
});

describe("isValidSemver — rejects", () => {
  const bad: Array<[string, string]> = [
    ["", "empty"],
    ["1", "single segment"],
    ["1.0", "missing patch"],
    ["1.0.0.0", "extra segment"],
    ["01.0.0", "leading zero in major"],
    ["1.02.0", "leading zero in minor"],
    ["1.0.03", "leading zero in patch"],
    ["v1.0.0", "v-prefix not allowed"],
    ["1.0.0-", "empty prerelease"],
    ["1.0.0+", "empty build meta"],
    ["1.0.0-..", "empty prerelease identifier"],
    ["1.0.0+..", "empty build identifier"],
    ["a.b.c", "non-numeric"],
    ["1.0.0 ", "trailing whitespace"],
    [" 1.0.0", "leading whitespace"],
    ["-1.0.0", "negative major"],
  ];
  for (const [s, why] of bad) {
    it(`rejects "${s}" (${why})`, () => {
      expect(isValidSemver(s)).toBe(false);
    });
  }
});

describe("isValidSemver — non-string inputs", () => {
  it("rejects null", () => {
    expect(isValidSemver(null)).toBe(false);
  });
  it("rejects undefined", () => {
    expect(isValidSemver(undefined)).toBe(false);
  });
  it("rejects number", () => {
    expect(isValidSemver(1)).toBe(false);
  });
  it("rejects object", () => {
    expect(isValidSemver({ major: 1 })).toBe(false);
  });
});

describe("parseSemver", () => {
  it("parses a simple MAJOR.MINOR.PATCH", () => {
    expect(parseSemver("1.2.3")).toEqual({ major: 1, minor: 2, patch: 3 });
  });

  it("parses prerelease tag", () => {
    expect(parseSemver("1.0.0-beta.1")).toEqual({
      major: 1,
      minor: 0,
      patch: 0,
      prerelease: "beta.1",
    });
  });

  it("parses build metadata", () => {
    expect(parseSemver("1.0.0+exp.42")).toEqual({
      major: 1,
      minor: 0,
      patch: 0,
      build: "exp.42",
    });
  });

  it("parses both prerelease and build", () => {
    expect(parseSemver("1.0.0-rc.1+build.42")).toEqual({
      major: 1,
      minor: 0,
      patch: 0,
      prerelease: "rc.1",
      build: "build.42",
    });
  });

  it("returns null for invalid input", () => {
    expect(parseSemver("not.a.version")).toBeNull();
    expect(parseSemver("1.0")).toBeNull();
  });

  it("returns null for the empty string", () => {
    expect(parseSemver("")).toBeNull();
  });
});
