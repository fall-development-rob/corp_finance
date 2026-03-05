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

export const JerseyFundSchema = z.object({
  fund_name: z.string().describe("Name of the Jersey/Guernsey fund"),
  structure_type: z.string().describe("Fund structure type (JPF, ExpertFund, ListedFund, QIF)"),
  fund_strategy: z.string().describe("Fund strategy (Hedge, PE, VC, RealEstate, Credit)"),
  fund_size: z.coerce.number().describe("Total fund size in base currency"),
  management_fee_rate: z.coerce.number().describe("Annual management fee rate (decimal)"),
  performance_fee_rate: z.coerce.number().describe("Performance fee rate (decimal)"),
  investor_count: z.coerce.number().int().describe("Number of investors"),
  jersey_directors_count: z.coerce.number().int().describe("Number of Jersey-resident directors"),
  local_admin: z.coerce.boolean().describe("Whether a local administrator is used"),
  aif_designation: z.coerce.boolean().describe("Whether the fund has AIF designation"),
  target_investors: z.array(z.string()).describe("Target investor types (e.g. Institutional, Professional, HNWI)"),
});

const SubFundInfoSchema = z.object({
  name: z.string().describe("Sub-fund name"),
  strategy: z.string().describe("Sub-fund investment strategy"),
  target_aum: z.coerce.number().describe("Target AUM for this sub-fund"),
  currency: z.string().describe("Base currency of the sub-fund"),
});

export const VccFundSchema = z.object({
  fund_name: z.string().describe("Name of the Singapore VCC fund"),
  vcc_type: z.string().describe("VCC type (Standalone, Umbrella)"),
  sub_funds: z.array(SubFundInfoSchema).describe("List of sub-funds (for umbrella VCC)"),
  manager_license: z.string().describe("Manager license type (RFMC, LRFMC, A_LFMC)"),
  fund_size: z.coerce.number().describe("Total fund size in base currency"),
  management_fee_rate: z.coerce.number().describe("Annual management fee rate (decimal)"),
  performance_fee_rate: z.coerce.number().describe("Performance fee rate (decimal)"),
  tax_incentive_scheme: z.string().optional().describe("Tax incentive scheme (S13O, S13U, S13D) or null"),
  investment_professionals_sg: z.coerce.number().int().describe("Number of investment professionals in Singapore"),
  local_business_spending: z.coerce.number().describe("Annual local business spending in SGD"),
  target_investors: z.array(z.string()).describe("Target investor types"),
});

export const HkOfcSchema = z.object({
  fund_name: z.string().describe("Name of the Hong Kong OFC fund"),
  ofc_type: z.string().describe("OFC type (Public or Private)"),
  umbrella: z.coerce.boolean().describe("Whether this is an umbrella fund"),
  sub_fund_count: z.coerce.number().int().describe("Number of sub-funds"),
  fund_strategy: z.string().describe("Fund strategy"),
  fund_size: z.coerce.number().describe("Total fund size in base currency"),
  management_fee_rate: z.coerce.number().describe("Annual management fee rate (decimal)"),
  performance_fee_rate: z.coerce.number().describe("Performance fee rate (decimal)"),
  type9_licensed_manager: z.coerce.boolean().describe("Whether manager holds SFC Type 9 license"),
  grant_scheme_eligible: z.coerce.boolean().describe("Whether eligible for OFC grant scheme (up to HKD 1M, 70% of eligible expenses)"),
  target_investors: z.array(z.string()).describe("Target investor types"),
});

export const DifcFundSchema = z.object({
  fund_name: z.string().describe("Name of the DIFC fund"),
  fund_type: z.string().describe("DIFC fund type (QIF, ExemptFund, DomesticFund)"),
  fund_strategy: z.string().describe("Fund strategy"),
  fund_size: z.coerce.number().describe("Total fund size in base currency"),
  management_fee_rate: z.coerce.number().describe("Annual management fee rate (decimal)"),
  performance_fee_rate: z.coerce.number().describe("Performance fee rate (decimal)"),
  minimum_subscription: z.coerce.number().describe("Minimum subscription amount"),
  investor_count: z.coerce.number().int().describe("Number of investors"),
  sharia_compliant: z.coerce.boolean().describe("Whether the fund is Sharia-compliant"),
  sharia_board_members: z.coerce.number().int().describe("Number of Sharia supervisory board members"),
  target_investors: z.array(z.string()).describe("Target investor types"),
});

const ComparisonWeightsSchema = z.object({
  setup_cost: z.coerce.number().describe("Weight for setup cost (decimal, weights should sum to 1)"),
  annual_cost: z.coerce.number().describe("Weight for annual ongoing cost"),
  tax: z.coerce.number().describe("Weight for tax efficiency"),
  regulatory_speed: z.coerce.number().describe("Weight for regulatory approval speed"),
  distribution_reach: z.coerce.number().describe("Weight for distribution reach"),
  substance: z.coerce.number().describe("Weight for economic substance requirements"),
});

export const JurisdictionComparisonSchema = z.object({
  jurisdictions: z.array(z.string()).describe("List of jurisdiction codes to compare (e.g. Cayman, BVI, Luxembourg, Ireland, Jersey, Guernsey, Singapore, HongKong, DIFC, ADGM)"),
  fund_strategy: z.string().describe("Fund strategy (Hedge, PE, VC, RealEstate, Credit)"),
  fund_size: z.coerce.number().describe("Total fund size in base currency"),
  fund_type: z.string().describe("Fund type (OpenEnded or ClosedEnded)"),
  weights: ComparisonWeightsSchema.describe("Comparison dimension weights (should sum to 1)"),
});

export const MigrationFeasibilitySchema = z.object({
  source_jurisdiction: z.string().describe("Current fund jurisdiction code"),
  source_vehicle_type: z.string().describe("Current fund vehicle type"),
  target_jurisdiction: z.string().describe("Target jurisdiction code for migration"),
  target_vehicle_type: z.string().describe("Target fund vehicle type"),
  fund_size: z.coerce.number().describe("Total fund size in base currency"),
  investor_count: z.coerce.number().int().describe("Number of investors"),
  fund_remaining_life_years: z.coerce.number().int().optional().describe("Remaining fund life in years (optional for open-ended)"),
  migration_driver: z.string().describe("Primary driver for migration (e.g. Regulatory, Tax, Distribution, Substance, Investor)"),
});
