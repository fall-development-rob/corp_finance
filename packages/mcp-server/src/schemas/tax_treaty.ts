import { z } from "zod";

const IncomeFlowSchema = z.object({
  income_type: z.string().describe("Type of income (e.g. Dividends, Interest, Royalties, CapitalGains, Services)"),
  amount: z.coerce.number().describe("Income amount"),
  domestic_wht_rate: z.coerce.number().describe("Domestic withholding tax rate without treaty (decimal)"),
});

const TreatyRateSchema = z.object({
  income_type: z.string().describe("Type of income for the treaty rate"),
  treaty_rate: z.coerce.number().describe("Treaty withholding tax rate (decimal)"),
  qualifying_conditions: z.array(z.string()).describe("Conditions required to qualify for the treaty rate"),
});

export const TreatyNetworkSchema = z.object({
  source_jurisdiction: z.string().describe("Jurisdiction where income originates"),
  recipient_jurisdiction: z.string().describe("Jurisdiction of the income recipient"),
  income_types: z.array(IncomeFlowSchema).describe("List of income flows to analyze"),
  treaty_rates: z.array(TreatyRateSchema).optional().describe("Optional treaty rates (if known; otherwise defaults apply)"),
  intermediary_jurisdictions: z.array(z.string()).describe("List of potential intermediary jurisdictions for treaty shopping analysis"),
  recipient_entity_type: z.string().describe("Type of recipient entity (e.g. Corporation, Partnership, Trust, Individual)"),
  beneficial_owner: z.coerce.boolean().describe("Whether the recipient is the beneficial owner"),
  lob_qualified: z.coerce.boolean().describe("Whether the recipient meets Limitation on Benefits (LOB) tests"),
  ppt_met: z.coerce.boolean().describe("Whether the Principal Purpose Test (PPT) is satisfied"),
});

const OperatingEntitySchema = z.object({
  name: z.string().describe("Operating entity name"),
  jurisdiction: z.string().describe("Entity jurisdiction"),
  annual_profit: z.coerce.number().describe("Annual operating profit"),
  annual_dividends_up: z.coerce.number().describe("Annual dividends paid upstream to holding company"),
  annual_royalties_out: z.coerce.number().describe("Annual royalties paid out"),
  annual_interest_out: z.coerce.number().describe("Annual interest paid out"),
  annual_management_fees_out: z.coerce.number().describe("Annual management fees paid out"),
  corporate_tax_rate: z.coerce.number().describe("Local corporate tax rate (decimal)"),
});

const HoldingCandidateSchema = z.object({
  jurisdiction: z.string().describe("Holding jurisdiction"),
  corporate_tax_rate: z.coerce.number().describe("Corporate tax rate (decimal)"),
  participation_exemption: z.coerce.boolean().describe("Whether participation exemption on dividends is available"),
  participation_threshold_pct: z.coerce.number().describe("Minimum ownership percentage for participation exemption (decimal)"),
  ip_box_rate: z.coerce.number().optional().describe("IP box reduced tax rate if available (decimal)"),
  cfc_rules_risk: z.string().describe("CFC rules risk level (Low, Medium, High)"),
  substance_cost_annual: z.coerce.number().describe("Annual cost of maintaining substance in jurisdiction"),
  treaty_network_size: z.coerce.number().int().describe("Number of tax treaties in the jurisdiction's network"),
});

const ParentEntitySchema = z.object({
  jurisdiction: z.string().describe("Ultimate parent jurisdiction"),
  corporate_tax_rate: z.coerce.number().describe("Parent jurisdiction corporate tax rate (decimal)"),
});

const PeRiskFactorSchema = z.object({
  jurisdiction: z.string().describe("Jurisdiction being assessed for PE risk"),
  has_fixed_place: z.coerce.boolean().describe("Whether a fixed place of business exists"),
  has_dependent_agent: z.coerce.boolean().describe("Whether a dependent agent operates in the jurisdiction"),
  employees_in_jurisdiction: z.coerce.number().int().describe("Number of employees in the jurisdiction"),
  contracts_concluded_locally: z.coerce.boolean().describe("Whether contracts are concluded locally"),
  server_or_warehouse: z.coerce.boolean().describe("Whether servers or warehouses are maintained"),
  duration_months: z.coerce.number().int().describe("Duration of activities in months"),
});

export const TreatyOptSchema = z.object({
  group_name: z.string().describe("Name of the multinational group"),
  operating_jurisdictions: z.array(OperatingEntitySchema).describe("List of operating entities across jurisdictions"),
  holding_jurisdiction_candidates: z.array(HoldingCandidateSchema).describe("Candidate holding jurisdictions to evaluate"),
  ultimate_parent: ParentEntitySchema.describe("Ultimate parent entity details"),
  pe_risk_factors: z.array(PeRiskFactorSchema).describe("Permanent establishment risk factors by jurisdiction"),
});
