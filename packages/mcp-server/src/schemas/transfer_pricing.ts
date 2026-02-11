import { z } from "zod";

const GroupEntitySchema = z.object({
  name: z.string().describe("Entity name"),
  jurisdiction: z.string().describe("Entity jurisdiction"),
  function: z.string().describe("Entity function (e.g. Manufacturing, Distribution, IP_Holder, Services)"),
  revenue: z.coerce.number().describe("Total revenue"),
  operating_profit: z.coerce.number().describe("Operating profit"),
  employees: z.coerce.number().int().describe("Number of employees"),
  tangible_assets: z.coerce.number().describe("Value of tangible assets"),
  intangible_assets: z.coerce.number().describe("Value of intangible assets"),
  related_party_revenue: z.coerce.number().describe("Revenue from related party transactions"),
});

const IntercompanyTxSchema = z.object({
  from_entity: z.string().describe("Name of the entity paying/transferring"),
  to_entity: z.string().describe("Name of the entity receiving"),
  transaction_type: z.string().describe("Transaction type (e.g. Goods, Services, Royalty, Interest, ManagementFee)"),
  amount: z.coerce.number().describe("Transaction amount"),
  arm_length_range_low: z.coerce.number().describe("Low end of arm's length range"),
  arm_length_range_high: z.coerce.number().describe("High end of arm's length range"),
});

export const BepsSchema = z.object({
  entity_name: z.string().describe("Name of the reporting entity or multinational group"),
  parent_jurisdiction: z.string().describe("Jurisdiction of the ultimate parent entity"),
  entities: z.array(GroupEntitySchema).describe("List of group entities across jurisdictions"),
  intercompany_transactions: z.array(IntercompanyTxSchema).describe("List of intercompany transactions"),
  group_consolidated_revenue: z.coerce.number().describe("Group consolidated revenue"),
  group_consolidated_profit: z.coerce.number().describe("Group consolidated profit"),
  cbcr_threshold: z.coerce.number().describe("Country-by-country reporting threshold (e.g. 750000000 for EUR 750M)"),
  pillar_two_applicable: z.coerce.boolean().describe("Whether Pillar Two global minimum tax applies"),
});

const TestedPartySchema = z.object({
  name: z.string().describe("Name of the tested party"),
  jurisdiction: z.string().describe("Tested party jurisdiction"),
  function: z.string().describe("Function performed (e.g. LimitedRiskDistributor, ContractManufacturer)"),
  operating_revenue: z.coerce.number().describe("Operating revenue"),
  operating_costs: z.coerce.number().describe("Operating costs"),
  operating_profit: z.coerce.number().describe("Operating profit"),
  assets: z.coerce.number().describe("Total assets employed"),
});

const ComparableSchema = z.object({
  name: z.string().describe("Comparable company name"),
  net_margin: z.coerce.number().describe("Net profit margin (decimal)"),
  berry_ratio: z.coerce.number().optional().describe("Berry ratio (gross profit / operating expenses)"),
  return_on_assets: z.coerce.number().optional().describe("Return on assets (decimal)"),
  gross_margin: z.coerce.number().optional().describe("Gross margin (decimal)"),
});

const CfcParamsSchema = z.object({
  parent_jurisdiction: z.string().describe("Parent entity jurisdiction for CFC analysis"),
  subsidiary_jurisdiction: z.string().describe("Subsidiary jurisdiction being tested"),
  subsidiary_income: z.coerce.number().describe("Total subsidiary income"),
  subsidiary_tax_paid: z.coerce.number().describe("Tax paid by subsidiary"),
  passive_income_pct: z.coerce.number().describe("Percentage of passive income (decimal)"),
  ownership_pct: z.coerce.number().describe("Parent ownership percentage (decimal)"),
  de_minimis_threshold: z.coerce.number().describe("De minimis threshold for CFC rules"),
});

export const IntercompanySchema = z.object({
  transaction_name: z.string().describe("Name or description of the intercompany transaction"),
  pricing_method: z.string().describe("Transfer pricing method (CUP, RPM, CPLM, TNMM, ProfitSplit)"),
  tested_party: TestedPartySchema.describe("The tested party in the transfer pricing analysis"),
  comparables: z.array(ComparableSchema).describe("List of comparable companies for benchmarking"),
  transaction_value: z.coerce.number().describe("Total value of the intercompany transaction"),
  cfc_analysis: CfcParamsSchema.optional().describe("Optional CFC (Controlled Foreign Corporation) analysis parameters"),
  gaar_jurisdiction: z.string().optional().describe("Jurisdiction for GAAR (General Anti-Avoidance Rule) analysis"),
});
