import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { surfacePiiScan, surfaceInjectionDetect } from "../bindings.js";
import {
  SurfacePiiScanSchema,
  SurfaceInjectionDetectSchema,
} from "../schemas/security.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerSecurityTools(server: McpServer) {
  server.tool(
    "surface_pii_scan",
    "Scan free-form text for the 14 PII categories (SSN, EIN, credit card, IBAN, email, phone, IP, MAC, passport, driver's licence, bank account, routing, DOB, name+address). Input: text. Output: ordered Finding list with span offsets, severity, and structure-preserving redaction proposals. RUF-SEC-001.",
    SurfacePiiScanSchema.shape,
    async (params) => {
      const validated = SurfacePiiScanSchema.parse(coerceNumbers(params));
      const result = surfacePiiScan(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "surface_injection_detect",
    "Scan free-form text for prompt-injection attack patterns (ignore_previous_instructions, role_switch, jailbreak_attempt, system_prompt_leak). Input: text. Output: ordered Finding list with span offsets and severity for hook-layer block routing. RUF-SEC-002.",
    SurfaceInjectionDetectSchema.shape,
    async (params) => {
      const validated = SurfaceInjectionDetectSchema.parse(coerceNumbers(params));
      const result = surfaceInjectionDetect(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
