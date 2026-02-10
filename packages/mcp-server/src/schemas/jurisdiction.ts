import { z } from "zod";
import { CurrencySchema } from "./common.js";

// ---------------------------------------------------------------------------
// Jurisdiction enum (matches Rust Jurisdiction enum in withholding_tax.rs)
// ---------------------------------------------------------------------------
const JurisdictionSchema = z.enum([
  "US", "UK", "Cayman", "Ireland", "Luxembourg", "Jersey", "Guernsey",
  "BVI", "Germany", "France", "Netherlands", "Switzerland", "Singapore",
  "HongKong", "Japan", "Australia", "Canada",
]).describe("Jurisdiction code");

// ---------------------------------------------------------------------------
// FundFeeInput (fund_fees.rs)
// ---------------------------------------------------------------------------
export const FundFeeSchema = z.object({
  fund_size: z.coerce.number().positive().describe("Total fund size / commitments"),
  management_fee_rate: z.coerce.number().min(0).max(1).describe("Annual management fee rate (e.g. 0.02 for 2%)"),
  management_fee_basis: z.enum(["CommittedCapital", "InvestedCapital", "NetAssetValue"]).describe("Basis for management fee calculation"),
  performance_fee_rate: z.coerce.number().min(0).max(1).describe("Carried interest rate (e.g. 0.20 for 20%)"),
  hurdle_rate: z.coerce.number().min(0).max(1).describe("Preferred return / hurdle rate"),
  catch_up_rate: z.coerce.number().min(0).max(1).describe("GP catch-up rate (1.0 = 100% catch-up)"),
  waterfall_type: z.enum(["European", "American"]).describe("Waterfall type for carry calculation"),
  gp_commitment_pct: z.coerce.number().min(0).max(1).describe("GP co-investment as % of fund"),
  clawback: z.coerce.boolean().describe("Whether GP clawback provision exists"),
  fund_life_years: z.coerce.number().int().min(1).describe("Total fund life in years"),
  investment_period_years: z.coerce.number().int().min(0).describe("Investment period in years"),
  gross_irr_assumption: z.coerce.number().describe("Assumed gross IRR for projections"),
  gross_moic_assumption: z.coerce.number().min(0).describe("Assumed gross MOIC for projections"),
  annual_fund_expenses: z.coerce.number().min(0).describe("Annual fund operating expenses"),
  currency: CurrencySchema.optional(),
});

// ---------------------------------------------------------------------------
// ReconciliationInput (reconciliation.rs)
// ---------------------------------------------------------------------------
export const ReconciliationSchema = z.object({
  source_standard: z.enum(["UsGaap", "Ifrs"]).describe("Source accounting standard"),
  target_standard: z.enum(["UsGaap", "Ifrs"]).describe("Target accounting standard to reconcile to"),
  revenue: z.coerce.number().describe("Total revenue under source standard"),
  ebitda: z.coerce.number().describe("EBITDA under source standard"),
  ebit: z.coerce.number().describe("EBIT under source standard"),
  net_income: z.coerce.number().describe("Net income under source standard"),
  total_assets: z.coerce.number().describe("Total assets under source standard"),
  total_debt: z.coerce.number().describe("Total debt under source standard"),
  total_equity: z.coerce.number().describe("Total equity under source standard"),
  inventory: z.coerce.number().describe("Inventory value under source standard"),
  ppe_net: z.coerce.number().describe("Net PP&E under source standard"),
  operating_lease_payments: z.coerce.number().optional().describe("Annual operating lease payments"),
  operating_lease_remaining_years: z.coerce.number().int().optional().describe("Remaining operating lease term in years"),
  lifo_reserve: z.coerce.number().optional().describe("LIFO reserve (GAAP only, for LIFO to FIFO conversion)"),
  capitalised_dev_costs: z.coerce.number().optional().describe("Capitalised development costs eligible for IAS 38 treatment"),
  dev_cost_amortisation: z.coerce.number().optional().describe("Annual amortisation of capitalised development costs"),
  revaluation_surplus: z.coerce.number().optional().describe("Asset revaluation surplus (IFRS allows, GAAP does not)"),
  discount_rate_for_leases: z.coerce.number().optional().describe("Discount rate for lease capitalisation PV calculation"),
  currency: CurrencySchema.optional(),
});

// ---------------------------------------------------------------------------
// WhtInput (withholding_tax.rs)
// ---------------------------------------------------------------------------
export const WhtSchema = z.object({
  source_jurisdiction: JurisdictionSchema.describe("Jurisdiction where income is sourced"),
  investor_jurisdiction: JurisdictionSchema.describe("Jurisdiction of the investor"),
  fund_jurisdiction: JurisdictionSchema.optional().describe("Jurisdiction of the fund vehicle (if applicable)"),
  income_type: z.enum(["Dividend", "Interest", "Royalty", "RentalIncome", "CapitalGain"]).describe("Type of income subject to withholding"),
  gross_income: z.coerce.number().positive().describe("Gross income amount before withholding"),
  is_tax_exempt_investor: z.coerce.boolean().describe("Whether the investor is a tax-exempt entity"),
  currency: CurrencySchema.optional(),
});

// ---------------------------------------------------------------------------
// PortfolioWhtInput (withholding_tax.rs)
// ---------------------------------------------------------------------------
export const PortfolioWhtSchema = z.object({
  holdings: z.array(WhtSchema).describe("Array of individual WHT holdings to analyse"),
});

// ---------------------------------------------------------------------------
// ShareClassInput (nav.rs) - sub-schema for NavInput
// ---------------------------------------------------------------------------
const SubscriptionSchema = z.object({
  investor_id: z.string().describe("Investor identifier"),
  amount: z.coerce.number().describe("Subscription amount"),
  nav_per_share_at_entry: z.coerce.number().describe("NAV per share at time of subscription"),
  shares_issued: z.coerce.number().describe("Number of shares issued for the subscription"),
});

const RedemptionSchema = z.object({
  investor_id: z.string().describe("Investor identifier"),
  shares_redeemed: z.coerce.number().describe("Number of shares redeemed"),
  nav_per_share_at_exit: z.coerce.number().describe("NAV per share at time of redemption"),
});

const ShareClassInputSchema = z.object({
  class_name: z.string().describe("Share class name (e.g. Class A, Class B)"),
  currency: CurrencySchema.describe("Currency for this share class"),
  shares_outstanding: z.coerce.number().positive().describe("Number of shares outstanding"),
  nav_per_share_opening: z.coerce.number().positive().describe("Opening NAV per share"),
  high_water_mark: z.coerce.number().positive().describe("High water mark NAV per share"),
  management_fee_rate: z.coerce.number().min(0).describe("Annual management fee rate for this class"),
  performance_fee_rate: z.coerce.number().min(0).describe("Performance fee rate for this class"),
  hurdle_rate: z.coerce.number().min(0).optional().describe("Hurdle rate for performance fee"),
  crystallisation_frequency: z.enum(["Monthly", "Quarterly", "SemiAnnually", "Annually", "OnRedemption"]).describe("Performance fee crystallisation frequency"),
  fx_rate_to_base: z.coerce.number().optional().describe("FX rate to convert class currency to base currency"),
  fx_hedging_cost: z.coerce.number().optional().describe("Annual FX hedging cost as a rate"),
  subscriptions: z.array(SubscriptionSchema).describe("Subscriptions during the period"),
  redemptions: z.array(RedemptionSchema).describe("Redemptions during the period"),
});

// ---------------------------------------------------------------------------
// NavInput (nav.rs)
// ---------------------------------------------------------------------------
export const NavSchema = z.object({
  share_classes: z.array(ShareClassInputSchema).describe("Array of share classes in the fund"),
  gross_portfolio_return: z.coerce.number().describe("Gross portfolio return for the period (e.g. 0.10 for 10%)"),
  period_label: z.string().describe("Label for the calculation period (e.g. Q4 2025)"),
  equalisation_method: z.enum(["EqualisationShares", "SeriesAccounting", "DepreciationDeposit", "None"]).describe("NAV equalisation method"),
  base_currency: CurrencySchema.describe("Base currency for total fund NAV calculation"),
});

// ---------------------------------------------------------------------------
// GpEconomicsInput (gp_economics.rs)
// ---------------------------------------------------------------------------
export const GpEconomicsSchema = z.object({
  fund_size: z.coerce.number().positive().describe("Total fund size / commitments"),
  management_fee_rate: z.coerce.number().min(0).max(0.05).describe("Annual management fee rate"),
  carried_interest_rate: z.coerce.number().min(0).max(0.50).describe("Carried interest rate"),
  hurdle_rate: z.coerce.number().min(0).describe("Preferred return / hurdle rate"),
  gp_commitment_pct: z.coerce.number().min(0).describe("GP co-investment as % of fund"),
  fund_life_years: z.coerce.number().int().min(1).describe("Total fund life in years"),
  investment_period_years: z.coerce.number().int().min(0).describe("Investment period in years"),
  num_investment_professionals: z.coerce.number().int().min(1).describe("Number of investment professionals"),
  annual_gp_overhead: z.coerce.number().min(0).describe("Annual GP overhead costs (rent, systems, travel, etc.)"),
  gross_irr_assumption: z.coerce.number().describe("Assumed gross IRR for projections"),
  gross_moic_assumption: z.coerce.number().min(0).describe("Assumed gross MOIC for projections"),
  fee_holiday_years: z.coerce.number().int().min(0).optional().describe("Years with reduced/no management fee at fund inception"),
  fee_discount_rate: z.coerce.number().min(0).max(1).optional().describe("Discount on management fee (e.g. for anchor LPs)"),
  successor_fund_offset: z.coerce.number().int().min(1).optional().describe("Year when successor fund starts charging fees"),
  currency: CurrencySchema.optional(),
});

// ---------------------------------------------------------------------------
// InvestorNetReturnsInput (investor_returns.rs)
// ---------------------------------------------------------------------------
export const InvestorNetReturnsSchema = z.object({
  gross_return: z.coerce.number().describe("Annualised gross return (e.g. 0.15 for 15%)"),
  investment_amount: z.coerce.number().positive().describe("Total investment amount"),
  holding_period_years: z.coerce.number().positive().describe("Holding period in years (can be fractional)"),
  management_fee: z.coerce.number().min(0).describe("Annual management fee as a rate"),
  performance_fee: z.coerce.number().min(0).describe("Performance fee rate (applied to gain above hurdle)"),
  hurdle_rate: z.coerce.number().min(0).optional().describe("Hurdle rate for the performance fee"),
  fund_expenses_pct: z.coerce.number().min(0).describe("Annual fund operating expenses as % of NAV"),
  fof_management_fee: z.coerce.number().min(0).optional().describe("Fund-of-funds additional management fee layer"),
  fof_performance_fee: z.coerce.number().min(0).optional().describe("Fund-of-funds performance fee layer"),
  wht_drag: z.coerce.number().min(0).describe("Annual withholding tax drag on returns"),
  blocker_cost: z.coerce.number().min(0).optional().describe("Annual blocker entity maintenance cost as a rate"),
  investor_tax_rate: z.coerce.number().min(0).optional().describe("Personal/institutional tax rate on gains"),
  currency: CurrencySchema.optional(),
});

// ---------------------------------------------------------------------------
// InvestmentDetail sub-schema (ubti.rs)
// ---------------------------------------------------------------------------
const InvestmentDetailSchema = z.object({
  name: z.string().describe("Investment name or identifier"),
  investment_type: z.enum([
    "DirectEquity",
    "DirectDebt",
    "LeveragedRealEstate",
    "OperatingBusiness",
    "MLP",
    "DebtFinancedProperty",
    "HedgeFund",
    "PrivateEquityFund",
    "VentureCapitalFund",
    "RealEstateFund",
    "FundOfFunds",
    "PublicEquity",
    "PublicFixedIncome",
    "Commodities",
    "Derivatives",
  ]).describe("Type of investment"),
  amount: z.coerce.number().positive().describe("Investment amount"),
  is_leveraged: z.coerce.boolean().describe("Whether the investment uses leverage"),
  leverage_ratio: z.coerce.number().min(0).optional().describe("Debt/equity ratio at the investment level"),
  has_operating_income: z.coerce.boolean().describe("Whether the investment generates operating business income"),
  us_source_income_pct: z.coerce.number().min(0).max(1).optional().describe("Percentage of income that is US-source (0-1)"),
});

// ---------------------------------------------------------------------------
// UbtiScreeningInput (ubti.rs)
// ---------------------------------------------------------------------------
export const UbtiScreeningSchema = z.object({
  investor_type: z.enum([
    "TaxExemptUS",
    "ForeignInvestor",
    "TaxableUS",
    "SovereignWealth",
    "InsuranceCompany",
  ]).describe("Type of investor for UBTI/ECI screening"),
  investments: z.array(InvestmentDetailSchema).describe("Array of investments to screen for UBTI/ECI"),
  use_blocker: z.coerce.boolean().optional().describe("Whether a blocker entity is being considered"),
  currency: CurrencySchema.optional(),
});
