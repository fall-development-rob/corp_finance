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
    exposure_amount: z.number().positive().describe("Exposure at default (EAD)"),
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
    conservation_buffer: z.number().min(0).max(0.1).describe("Capital conservation buffer (e.g. 0.025)"),
    countercyclical_buffer: z.number().min(0).max(0.1).describe("Countercyclical buffer"),
    systemic_buffer: z.number().min(0).max(0.1).describe("G-SIB systemic buffer"),
  }).optional().describe("Capital buffer requirements"),
});

export const LcrSchema = z.object({
  institution_name: z.string().describe("Institution name"),
  hqla: z.array(z.object({
    name: z.string().describe("HQLA item name"),
    market_value: z.number().positive().describe("Market value"),
    level: z.enum(["Level1", "Level2A", "Level2B"]).describe("HQLA classification level"),
  })).describe("High-Quality Liquid Assets"),
  outflows: z.array(z.object({
    name: z.string().describe("Outflow item name"),
    amount: z.number().positive().describe("Outflow amount"),
    outflow_type: z.enum([
      "RetailStable", "RetailLessStable", "UnsecuredWholesaleOperational",
      "UnsecuredWholesaleNonOperational", "SecuredFunding", "DerivativeOutflows",
      "CreditCommitmentDrawdown", "Other",
    ]).describe("Outflow type for run-off factor"),
    run_off_rate: z.number().min(0).max(1).optional().describe("Override run-off rate"),
  })).describe("Cash outflows"),
  inflows: z.array(z.object({
    name: z.string().describe("Inflow item name"),
    amount: z.number().positive().describe("Inflow amount"),
    inflow_rate: z.number().min(0).max(1).describe("Inflow rate"),
  })).describe("Cash inflows"),
  cap_inflows: z.boolean().optional().describe("Cap inflows at 75% of outflows (default true)"),
});

export const NsfrSchema = z.object({
  institution_name: z.string().describe("Institution name"),
  available_stable_funding: z.array(z.object({
    name: z.string().describe("ASF item name"),
    amount: z.number().positive().describe("Amount"),
    category: z.enum([
      "Tier1Capital", "StableRetailDeposits", "LessStableRetailDeposits",
      "WholesaleFundingLongTerm", "WholesaleFundingMedTerm", "OtherShortTerm",
    ]).describe("ASF category"),
  })).describe("Available Stable Funding items"),
  required_stable_funding: z.array(z.object({
    name: z.string().describe("RSF item name"),
    amount: z.number().positive().describe("Amount"),
    category: z.enum([
      "Cash", "Level1Hqla", "Level2AHqla", "CorporateLoansShort",
      "CorporateLoansLong", "RetailMortgages", "Other",
    ]).describe("RSF category"),
  })).describe("Required Stable Funding items"),
});

export const AlmSchema = z.object({
  institution_name: z.string().describe("Institution name"),
  asset_buckets: z.array(z.object({
    bucket: z.string().describe("Bucket label (e.g. '0-3m', '1-5yr')"),
    midpoint_years: z.number().positive().describe("Midpoint tenor in years"),
    amount: z.number().min(0).describe("Amount maturing in this bucket"),
  })).describe("Assets by maturity bucket"),
  liability_buckets: z.array(z.object({
    bucket: z.string().describe("Bucket label"),
    midpoint_years: z.number().positive().describe("Midpoint tenor in years"),
    amount: z.number().min(0).describe("Amount maturing in this bucket"),
  })).describe("Liabilities by maturity bucket"),
  asset_duration: z.number().min(0).describe("Weighted average duration of assets (years)"),
  liability_duration: z.number().min(0).describe("Weighted average duration of liabilities (years)"),
  total_assets: z.number().positive().describe("Total assets"),
  total_equity: z.number().positive().describe("Total equity"),
  rate_shock_bps: z.number().describe("Parallel rate shock in basis points (e.g. 200)"),
  current_nii: z.number().describe("Current annual Net Interest Income"),
  asset_repricing_pct: z.number().min(0).max(1).describe("Assets repricing within 1yr (0-1)"),
  liability_repricing_pct: z.number().min(0).max(1).describe("Liabilities repricing within 1yr (0-1)"),
});
