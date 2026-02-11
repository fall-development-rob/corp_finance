import { z } from "zod";

const FeederInfoSchema = z.object({
  jurisdiction: z.string().describe("Feeder fund jurisdiction"),
  feeder_type: z.string().describe("Feeder type (Cayman, Delaware, BVI, Ireland)"),
  allocation_pct: z.coerce.number().describe("Allocation percentage to this feeder (decimal)"),
  investor_profile: z.string().describe("Investor profile (USTaxExempt, USTaxable, NonUS)"),
});

const ServiceProvidersSchema = z.object({
  administrator: z.string().describe("Fund administrator name"),
  auditor: z.string().describe("Fund auditor name"),
  legal_counsel: z.string().describe("Legal counsel name"),
  prime_broker: z.string().describe("Prime broker name"),
});

export const CaymanFundSchema = z.object({
  fund_name: z.string().describe("Name of the Cayman fund"),
  structure_type: z.string().describe("Fund structure type (ExemptedLP, SPC, UnitTrust, LLC, BVI_BCA, BVI_LP)"),
  fund_strategy: z.string().describe("Fund strategy (Hedge, PE, VC, RealEstate, Credit, FundOfFunds)"),
  fund_size: z.coerce.number().describe("Total fund size in base currency"),
  management_fee_rate: z.coerce.number().describe("Annual management fee rate (decimal)"),
  performance_fee_rate: z.coerce.number().describe("Performance fee rate (decimal, e.g. 0.20 for 20%)"),
  hurdle_rate: z.coerce.number().describe("Performance fee hurdle rate (decimal)"),
  high_water_mark: z.coerce.boolean().describe("Whether a high water mark applies to performance fees"),
  fund_term_years: z.coerce.number().int().optional().describe("Fund term in years (optional for open-ended funds)"),
  master_feeder: z.coerce.boolean().describe("Whether this is a master-feeder structure"),
  feeder_jurisdictions: z.array(FeederInfoSchema).describe("List of feeder fund jurisdictions and allocations"),
  service_providers: ServiceProvidersSchema.describe("Service provider details"),
  cima_registered: z.coerce.boolean().describe("Whether the fund is registered with CIMA"),
  annual_government_fees: z.coerce.number().optional().describe("Annual government fees (optional)"),
});

export const LuxFundSchema = z.object({
  fund_name: z.string().describe("Name of the Luxembourg/Ireland fund"),
  structure_type: z.string().describe("Fund structure type (SICAV_SIF, SICAV_RAIF, SCSp, ICAV, QIAIF, Section110)"),
  domicile: z.string().describe("Fund domicile (Luxembourg, Ireland)"),
  fund_size: z.coerce.number().describe("Total fund size in base currency"),
  management_fee_rate: z.coerce.number().describe("Annual management fee rate (decimal)"),
  carried_interest_rate: z.coerce.number().describe("Carried interest rate (decimal)"),
  fund_term_years: z.coerce.number().int().optional().describe("Fund term in years (optional for open-ended)"),
  target_investor_base: z.array(z.string()).describe("Target investor categories (e.g. Institutional, HNWI, Retail)"),
  aifmd_full_scope: z.coerce.boolean().describe("Whether subject to full AIFMD scope"),
  ucits_compliant: z.coerce.boolean().describe("Whether the fund is UCITS compliant"),
  subscription_tax_exempt: z.coerce.boolean().describe("Whether subscription tax exemption applies"),
  management_company_location: z.string().describe("Location of the management company"),
});
