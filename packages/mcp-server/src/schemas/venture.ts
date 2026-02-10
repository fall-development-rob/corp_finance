import { z } from "zod";

export const FundingRoundSchema = z.object({
  pre_money_valuation: z.coerce.number().positive().describe("Pre-money valuation of the company"),
  investment_amount: z.coerce.number().positive().describe("Amount the new investor is investing"),
  existing_shares: z.coerce.number().int().positive().describe("Total shares outstanding before this round"),
  existing_shareholders: z.array(z.object({
    name: z.string().describe("Shareholder name"),
    shares: z.coerce.number().int().min(0).describe("Number of shares held"),
    share_class: z.string().describe("Share class (e.g. 'Common', 'Series A')"),
  })).describe("List of current shareholders"),
  option_pool_pct: z.coerce.number().min(0).max(1).optional().describe("Target option pool as % of post-money (e.g. 0.10 = 10%)"),
  option_pool_shares_existing: z.coerce.number().int().min(0).describe("Shares already allocated to option pool"),
  round_name: z.string().describe("Round label (e.g. 'Series A')"),
  liquidation_preference: z.enum(["NonParticipating", "Participating", "CappedParticipating"]).describe("Liquidation preference type"),
  participation_cap: z.coerce.number().positive().optional().describe("Participation cap as multiple (e.g. 3.0x)"),
});

export const DilutionSchema = z.object({
  rounds: z.array(z.object({
    name: z.string().describe("Round name (e.g. 'Seed', 'Series A')"),
    pre_money_valuation: z.coerce.number().positive().describe("Pre-money valuation"),
    investment_amount: z.coerce.number().positive().describe("Investment amount"),
    option_pool_pct: z.coerce.number().min(0).max(1).describe("Option pool as % of post-money"),
  })).describe("Rounds in chronological order"),
  initial_shares: z.coerce.number().int().positive().describe("Total founder shares at incorporation"),
  founders: z.array(z.object({
    name: z.string().describe("Founder name"),
    initial_shares: z.coerce.number().int().positive().describe("Initial shares held"),
  })).describe("Founder breakdown"),
});

export const ConvertibleNoteSchema = z.object({
  principal: z.coerce.number().positive().describe("Note face value"),
  interest_rate: z.coerce.number().min(0).max(0.3).describe("Annual interest rate (e.g. 0.05 = 5%)"),
  term_months: z.coerce.number().int().positive().describe("Note term in months"),
  elapsed_months: z.coerce.number().int().min(0).describe("Months elapsed since issuance"),
  discount_rate: z.coerce.number().min(0).max(1).describe("Conversion discount (e.g. 0.20 = 20%)"),
  valuation_cap: z.coerce.number().positive().optional().describe("Maximum pre-money valuation for conversion"),
  qualified_financing_amount: z.coerce.number().positive().describe("Size of the qualifying equity round"),
  qualified_financing_pre_money: z.coerce.number().positive().describe("Pre-money valuation of the qualifying round"),
  pre_money_shares: z.coerce.number().int().positive().describe("Shares outstanding before the round"),
  conversion_trigger: z.enum(["QualifiedFinancing", "Maturity", "ChangeOfControl"]).describe("What triggers conversion"),
});

export const SafeSchema = z.object({
  investment_amount: z.coerce.number().positive().describe("Amount invested via the SAFE"),
  valuation_cap: z.coerce.number().positive().optional().describe("Valuation cap"),
  discount_rate: z.coerce.number().min(0).max(1).optional().describe("Discount rate (e.g. 0.20 = 20%)"),
  safe_type: z.enum(["PreMoney", "PostMoney"]).describe("Type of SAFE"),
  qualified_financing_pre_money: z.coerce.number().positive().describe("Pre-money valuation of qualifying round"),
  qualified_financing_amount: z.coerce.number().positive().describe("Amount of the qualifying round"),
  pre_money_shares: z.coerce.number().int().positive().describe("Shares outstanding before conversion"),
  mfn: z.coerce.boolean().describe("Most Favoured Nation provision"),
});

export const VentureFundSchema = z.object({
  fund_size: z.coerce.number().positive().describe("Total LP commitments"),
  management_fee_rate: z.coerce.number().min(0).max(0.1).describe("Annual management fee as decimal (e.g. 0.02 = 2%)"),
  carry_rate: z.coerce.number().min(0).max(0.5).describe("Carried interest rate (e.g. 0.20 = 20%)"),
  hurdle_rate: z.coerce.number().min(0).max(0.3).describe("Preferred return hurdle (e.g. 0.08 = 8%)"),
  fund_life_years: z.coerce.number().int().positive().describe("Total fund life in years"),
  investment_period_years: z.coerce.number().int().positive().describe("Investment period in years"),
  investments: z.array(z.object({
    company_name: z.string().describe("Portfolio company name"),
    investment_amount: z.coerce.number().positive().describe("Capital deployed"),
    investment_year: z.coerce.number().int().positive().describe("Year deployed (1-indexed)"),
    exit_year: z.coerce.number().int().positive().optional().describe("Year of exit"),
    exit_multiple: z.coerce.number().min(0).optional().describe("Return multiple (e.g. 10.0 = 10x)"),
    exit_type: z.enum(["Ipo", "Acquisition", "Secondary", "WriteOff", "Unrealised"]).describe("Exit type"),
  })).describe("Portfolio company investments"),
  recycling_rate: z.coerce.number().min(0).max(1).describe("Fraction of early returns that can be reinvested"),
});
