import { z } from "zod";

export const RegulatoryCapitalSchema = z.object({
  institution_name: z.string().describe("Institution name"),
  capital: z.object({
    cet1: z.number().min(0).describe("Common Equity Tier 1"),
    additional_tier1: z.number().min(0).describe("Additional Tier 1 (CoCos, preferred)"),
    tier2: z.number().min(0).describe("Tier 2 (subordinated debt, general provisions)"),
    deductions: z.number().min(0).describe("Regulatory deductions"),
  }).describe("Capital structure"),
  credit_exposures: z.array(z.object({
    name: z.string().describe("Exposure name"),
    exposure_amount: z.number().min(0).describe("Exposure at default (EAD)"),
    asset_class: z.enum(["Sovereign", "Bank", "Corporate", "Retail", "Mortgage", "Equity", "Other"]).describe("Basel III asset class"),
    risk_weight: z.number().min(0).max(1.5).optional().describe("Override risk weight"),
    external_rating: z.string().optional().describe("External credit rating"),
    collateral_value: z.number().min(0).optional().describe("Collateral value for CRM"),
    collateral_type: z.enum(["Cash", "GovernmentBond", "CorporateBond", "Equity", "RealEstate"]).optional().describe("Collateral type"),
  })).describe("Credit exposures for RWA calculation"),
  market_risk_charge: z.number().min(0).optional().describe("Pre-calculated market risk RWA"),
  operational_risk: z.object({
    approach: z.enum(["BasicIndicator", "Standardised"]).describe("OpRisk approach"),
    gross_income_3yr: z.array(z.number()).describe("Three years of gross income"),
    business_lines: z.array(z.object({
      line: z.string().describe("Business line name"),
      gross_income: z.number().describe("Gross income for this line"),
    })).optional().describe("Business-line breakdown (for Standardised)"),
  }).describe("Operational risk inputs"),
  buffers: z.object({
    conservation_buffer: z.number().min(0).describe("Capital conservation buffer (e.g. 0.025)"),
    countercyclical_buffer: z.number().min(0).describe("Countercyclical buffer"),
    systemic_buffer: z.number().min(0).describe("G-SIB systemic buffer"),
  }).optional().describe("Capital buffer requirements"),
});

const HqlaAssetSchema = z.object({
  name: z.string().describe("HQLA asset name"),
  market_value: z.number().min(0).describe("Market value"),
  haircut: z.number().min(0).max(1).optional().describe("Override haircut (0-1)"),
});

const OutflowCategoryEnum = z.enum([
  "RetailStableDeposits",
  "RetailLessStable",
  "UnsecuredWholesaleOperational",
  "UnsecuredWholesaleNonOperational",
  "UnsecuredWholesaleFinancial",
  "SecuredFundingCentral",
  "SecuredFundingLevel1",
  "SecuredFundingLevel2A",
  "SecuredFundingOther",
  "CreditFacilities",
  "LiquidityFacilities",
  "Other",
]);

const InflowCategoryEnum = z.enum([
  "RetailLoans",
  "WholesaleNonFinancial",
  "WholesaleFinancial",
  "SecuredLendingLevel1",
  "SecuredLendingLevel2A",
  "SecuredLendingOther",
  "Other",
]);

export const LcrSchema = z.object({
  institution_name: z.string().describe("Institution name"),
  hqla: z.object({
    level1_assets: z.array(HqlaAssetSchema).describe("Level 1: cash, central bank reserves, govt bonds"),
    level2a_assets: z.array(HqlaAssetSchema).describe("Level 2A: GSE bonds, 20% RW corporate bonds"),
    level2b_assets: z.array(HqlaAssetSchema).describe("Level 2B: lower quality (RMBS, corporate bonds, equities)"),
  }).describe("HQLA portfolio"),
  cash_outflows: z.array(z.object({
    category: OutflowCategoryEnum.describe("Outflow category"),
    amount: z.number().min(0).describe("Outflow amount"),
    run_off_rate: z.number().min(0).max(1).optional().describe("Override run-off rate"),
  })).describe("Cash outflows"),
  cash_inflows: z.array(z.object({
    category: InflowCategoryEnum.describe("Inflow category"),
    amount: z.number().min(0).describe("Inflow amount"),
    inflow_rate: z.number().min(0).max(1).optional().describe("Override inflow rate"),
  })).describe("Cash inflows"),
});

const AsfCategoryEnum = z.enum([
  "RegulatoryCapital",
  "StableRetailDeposits",
  "LessStableRetailDeposits",
  "WholesaleFundingGt1Y",
  "WholesaleFunding6mTo1Y",
  "WholesaleFundingLt6M",
  "Other",
]);

const RsfCategoryEnum = z.enum([
  "Cash",
  "CentralBankReserves",
  "Level1Hqla",
  "Level2aHqla",
  "Level2bHqla",
  "LoansToFILt6M",
  "LoansToFI6mTo1Y",
  "ResidentialMortgages",
  "RetailLoans",
  "CorporateLoansGt1Y",
  "NonPerformingLoans",
  "FixedAssets",
  "Other",
]);

export const NsfrSchema = z.object({
  institution_name: z.string().describe("Institution name"),
  available_funding: z.array(z.object({
    category: AsfCategoryEnum.describe("ASF category"),
    amount: z.number().min(0).describe("Amount"),
    asf_factor: z.number().min(0).max(1).optional().describe("Override ASF factor"),
  })).describe("Available Stable Funding sources"),
  required_funding: z.array(z.object({
    category: RsfCategoryEnum.describe("RSF category"),
    amount: z.number().min(0).describe("Amount"),
    rsf_factor: z.number().min(0).max(1).optional().describe("Override RSF factor"),
  })).describe("Required Stable Funding items"),
});

const RepricingBucketEnum = z.enum([
  "Overnight",
  "UpTo1M",
  "M1to3",
  "M3to6",
  "M6to12",
  "Y1to2",
  "Y2to3",
  "Y3to5",
  "Y5to10",
  "Over10Y",
  "NonSensitive",
]);

const MaturityBucketEnum = z.enum([
  "Overnight",
  "UpTo1M",
  "M1to3",
  "M3to6",
  "M6to12",
  "Y1to2",
  "Y2to3",
  "Y3to5",
  "Y5to10",
  "Over10Y",
  "NonSensitive",
]);

const RateTypeEnum = z.enum(["Fixed", "Floating"]);

const AlmPositionSchema = z.object({
  name: z.string().describe("Position name"),
  balance: z.number().min(0).describe("Position balance"),
  rate: z.number().describe("Interest rate"),
  repricing_bucket: RepricingBucketEnum.describe("Repricing bucket"),
  maturity_bucket: MaturityBucketEnum.describe("Maturity bucket"),
  rate_type: RateTypeEnum.describe("Fixed or Floating"),
  rate_sensitivity: z.number().describe("Rate pass-through sensitivity (0-1)"),
});

const BucketShiftSchema = z.object({
  bucket: RepricingBucketEnum.describe("Repricing bucket"),
  shift_bps: z.number().int().describe("Rate shift in basis points"),
});

const RateScenarioSchema = z.object({
  name: z.string().describe("Scenario name"),
  shifts: z.array(BucketShiftSchema).describe("Rate shifts by bucket"),
});

export const AlmSchema = z.object({
  institution_name: z.string().describe("Institution name"),
  assets: z.array(AlmPositionSchema).describe("Asset positions"),
  liabilities: z.array(AlmPositionSchema).describe("Liability positions"),
  off_balance_sheet: z.array(AlmPositionSchema).optional().default([]).describe("Off-balance sheet positions"),
  rate_scenarios: z.array(RateScenarioSchema).describe("Interest rate scenarios"),
  current_nii: z.number().describe("Current annual Net Interest Income"),
});
