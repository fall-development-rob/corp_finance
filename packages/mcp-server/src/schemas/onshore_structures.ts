import { z } from "zod";

const InvestorTypeSchema = z.object({
  category: z.string().describe("Investor category (e.g. TaxExempt, Taxable, NonUS, ERISA)"),
  allocation_pct: z.coerce.number().describe("Allocation percentage to this investor category (decimal)"),
});

export const UsFundSchema = z.object({
  fund_name: z.string().describe("Name of the US fund"),
  structure_type: z.string().describe("Fund structure type (DelawareLP, LLC, REIT, MLP, BDC, QOZ)"),
  fund_size: z.coerce.number().describe("Total fund size in base currency"),
  management_fee_rate: z.coerce.number().describe("Annual management fee rate (decimal, e.g. 0.02 for 2%)"),
  carried_interest_rate: z.coerce.number().describe("Carried interest rate (decimal, e.g. 0.20 for 20%)"),
  preferred_return: z.coerce.number().describe("Preferred return hurdle rate (decimal, e.g. 0.08 for 8%)"),
  gp_commitment_pct: z.coerce.number().describe("GP commitment as percentage of fund size (decimal)"),
  fund_term_years: z.coerce.number().int().describe("Fund term in years"),
  state_of_formation: z.string().describe("US state of fund formation (e.g. Delaware, New York)"),
  investor_types: z.array(InvestorTypeSchema).describe("List of investor types and their allocations"),
  expected_annual_return: z.coerce.number().describe("Expected annual return (decimal)"),
  distribution_frequency: z.string().describe("Distribution frequency (Quarterly, Annual, AtRealization)"),
  tax_elections: z.array(z.string()).describe("Tax elections (e.g. Section754, QSBS, QOZDeferral)"),
});

export const UkEuFundSchema = z.object({
  fund_name: z.string().describe("Name of the UK/EU fund"),
  structure_type: z.string().describe("Fund structure type (UKLP, UKLLP, OEIC, ACS, SICAV, FCP, KG)"),
  domicile: z.string().describe("Fund domicile jurisdiction (UK, France, Germany, Netherlands)"),
  fund_size: z.coerce.number().describe("Total fund size in base currency"),
  management_fee_rate: z.coerce.number().describe("Annual management fee rate (decimal)"),
  carried_interest_rate: z.coerce.number().describe("Carried interest rate (decimal)"),
  preferred_return: z.coerce.number().describe("Preferred return hurdle rate (decimal)"),
  fund_term_years: z.coerce.number().int().describe("Fund term in years"),
  investor_types: z.array(InvestorTypeSchema).describe("List of investor types and their allocations"),
  expected_annual_return: z.coerce.number().describe("Expected annual return (decimal)"),
  aifmd_compliant: z.coerce.boolean().describe("Whether the fund is AIFMD compliant"),
  ucits_compliant: z.coerce.boolean().describe("Whether the fund is UCITS compliant"),
  vat_rate: z.coerce.number().describe("Applicable VAT rate on management fees (decimal)"),
});
