import { z } from "zod";

/**
 * Security bounded-context schemas. Mirrors
 * `crates/corp-finance-core/src/security/types.rs` (Finding aggregate).
 * Per ADR-017 §5 / RUF-SEC-001..003.
 */

export const SurfacePiiScanSchema = z.object({
  text: z
    .string()
    .describe(
      "Free-form text scanned for the 14 PII categories (SSN, EIN, credit card, IBAN, email, phone, IP, MAC, passport, driver's licence, bank account, routing, DOB, name+address)."
    ),
});

export const SurfaceInjectionDetectSchema = z.object({
  text: z
    .string()
    .describe(
      "Free-form text scanned for prompt-injection patterns (ignore_previous_instructions, role_switch, jailbreak_attempt, system_prompt_leak)."
    ),
});
