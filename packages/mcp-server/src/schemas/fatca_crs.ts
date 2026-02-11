import { z } from "zod";

const ControllingPersonSchema = z.object({
  name: z.string().describe("Controlling person name"),
  tax_residence: z.string().describe("Tax residence jurisdiction"),
  ownership_pct: z.coerce.number().describe("Ownership percentage (decimal, e.g. 0.25 = 25%)"),
});

export const FatcaCrsReportingSchema = z.object({
  institution_name: z.string().describe("Financial institution name"),
  jurisdiction: z.string().describe("Institution jurisdiction"),
  iga_model: z.enum(["Model1", "Model2", "NonIGA"]).describe("FATCA IGA model type"),
  account_count: z.coerce.number().int().min(0).describe("Total number of reportable accounts"),
  aggregate_balance_usd: z.coerce.number().min(0).describe("Aggregate balance of reportable accounts in USD"),
  account_types: z.array(z.enum([
    "Depository",
    "Custodial",
    "EquityInterest",
    "DebtInterest",
    "CashValueInsurance",
  ])).describe("Types of accounts held by the institution"),
  us_indicia_found: z.coerce.number().int().min(0).describe("Number of accounts with US indicia found"),
  has_giin: z.boolean().describe("Whether the institution has a Global Intermediary Identification Number"),
  crs_participating: z.boolean().describe("Whether the institution participates in CRS reporting"),
  crs_jurisdictions: z.array(z.string()).describe("List of CRS participating jurisdictions"),
  reporting_year: z.coerce.number().int().describe("Tax reporting year"),
});

export const EntityClassificationSchema = z.object({
  entity_name: z.string().describe("Name of the entity being classified"),
  entity_type: z.string().describe("Legal entity type (e.g. Corporation, Trust, Partnership)"),
  jurisdiction_of_incorporation: z.string().describe("Jurisdiction where entity is incorporated"),
  jurisdiction_of_tax_residence: z.string().describe("Jurisdiction of tax residence"),
  gross_income: z.coerce.number().min(0).describe("Total gross income"),
  passive_income: z.coerce.number().min(0).describe("Passive income (dividends, interest, rents, royalties)"),
  total_assets: z.coerce.number().min(0).describe("Total assets"),
  passive_assets: z.coerce.number().min(0).describe("Passive assets (assets producing passive income)"),
  is_publicly_traded: z.boolean().describe("Whether the entity is publicly traded on a recognized exchange"),
  is_government_entity: z.boolean().describe("Whether the entity is a government entity or instrumentality"),
  is_international_org: z.boolean().describe("Whether the entity is an international organization"),
  is_pension_fund: z.boolean().describe("Whether the entity is a broad-participation or narrow-participation pension fund"),
  controlling_persons: z.array(ControllingPersonSchema).describe("Controlling persons with ownership or control"),
  has_us_controlling_persons: z.boolean().describe("Whether any controlling persons are US persons"),
  is_sponsored: z.boolean().describe("Whether the entity is a sponsored FFI"),
  sponsor_giin: z.string().optional().describe("Sponsoring entity GIIN (if applicable)"),
});
