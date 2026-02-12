import { z } from "zod";

export const NimAnalysisSchema = z.object({
  interest_income: z.coerce.number().describe("Total interest income"),
  interest_expense: z.coerce.number().describe("Total interest expense"),
  earning_assets: z.coerce.number().describe("Total earning assets"),
  asset_mix: z.array(z.object({
    name: z.string().describe("Asset category name"),
    balance: z.coerce.number().describe("Asset balance"),
    yield_rate: z.coerce.number().describe("Yield rate as decimal"),
  })).describe("Asset mix with yields for rate/volume decomposition"),
  liability_mix: z.array(z.object({
    name: z.string().describe("Liability category name"),
    balance: z.coerce.number().describe("Liability balance"),
    cost_rate: z.coerce.number().describe("Cost rate as decimal"),
  })).describe("Liability mix with costs for rate/volume decomposition"),
  prior_interest_income: z.coerce.number().describe("Prior period interest income"),
  prior_interest_expense: z.coerce.number().describe("Prior period interest expense"),
  prior_earning_assets: z.coerce.number().describe("Prior period earning assets"),
  rate_sensitive_assets: z.coerce.number().describe("Rate-sensitive assets for gap analysis"),
  rate_sensitive_liabilities: z.coerce.number().describe("Rate-sensitive liabilities for gap analysis"),
});

export const CamelsRatingSchema = z.object({
  tier1_capital: z.coerce.number().describe("Tier 1 capital"),
  total_capital: z.coerce.number().describe("Total regulatory capital"),
  risk_weighted_assets: z.coerce.number().describe("Risk-weighted assets"),
  leverage_ratio: z.coerce.number().describe("Leverage ratio as decimal"),
  npl_ratio: z.coerce.number().describe("Non-performing loan ratio as decimal"),
  provision_coverage: z.coerce.number().describe("Provision coverage ratio as decimal"),
  loan_loss_reserve_ratio: z.coerce.number().describe("Loan loss reserve ratio as decimal"),
  classified_assets_ratio: z.coerce.number().describe("Classified assets ratio as decimal"),
  efficiency_ratio: z.coerce.number().describe("Efficiency ratio as decimal"),
  compliance_score: z.coerce.number().describe("Compliance score (0-100)"),
  board_independence_pct: z.coerce.number().describe("Board independence percentage as decimal"),
  roa: z.coerce.number().describe("Return on assets as decimal"),
  roe: z.coerce.number().describe("Return on equity as decimal"),
  nim: z.coerce.number().describe("Net interest margin as decimal"),
  cost_income_ratio: z.coerce.number().describe("Cost-to-income ratio as decimal"),
  lcr: z.coerce.number().describe("Liquidity coverage ratio as decimal"),
  nsfr: z.coerce.number().describe("Net stable funding ratio as decimal"),
  loan_to_deposit: z.coerce.number().describe("Loan-to-deposit ratio as decimal"),
  interest_rate_risk_score: z.coerce.number().describe("Interest rate risk score (0-100)"),
  fx_exposure_pct: z.coerce.number().describe("FX exposure as percentage of assets"),
  duration_gap: z.coerce.number().describe("Duration gap in years"),
});

export const CeclProvisioningSchema = z.object({
  segments: z.array(z.object({
    name: z.string().describe("Loan segment name"),
    balance: z.coerce.number().describe("Segment balance"),
    pd_base: z.coerce.number().describe("Base scenario PD as decimal"),
    pd_adverse: z.coerce.number().describe("Adverse scenario PD as decimal"),
    pd_severe: z.coerce.number().describe("Severe scenario PD as decimal"),
    lgd: z.coerce.number().describe("Loss given default as decimal"),
    remaining_life: z.coerce.number().describe("Remaining life in years"),
    stage: z.coerce.number().int().describe("IFRS 9 stage (1, 2, or 3)"),
  })).describe("Loan segments for CECL expected credit loss calculation"),
  scenario_weights: z.object({
    base: z.coerce.number().describe("Base scenario weight as decimal"),
    adverse: z.coerce.number().describe("Adverse scenario weight as decimal"),
    severe: z.coerce.number().describe("Severe scenario weight as decimal"),
  }).describe("Probability weights for each macroeconomic scenario"),
  discount_rate: z.coerce.number().describe("Discount rate for present value calculation"),
});

export const DepositBetaSchema = z.object({
  rate_changes: z.array(z.object({
    period: z.string().describe("Period identifier (e.g. 'Q1 2024')"),
    benchmark_rate_change: z.coerce.number().describe("Change in benchmark rate (bps or decimal)"),
    deposit_rate_change: z.coerce.number().describe("Change in deposit rate (bps or decimal)"),
  })).describe("Historical rate change observations for beta estimation"),
  current_deposit_rate: z.coerce.number().describe("Current deposit rate as decimal"),
  current_benchmark_rate: z.coerce.number().describe("Current benchmark rate as decimal"),
});

export const LoanBookAnalysisSchema = z.object({
  loans: z.array(z.object({
    id: z.string().describe("Loan identifier"),
    balance: z.coerce.number().describe("Outstanding loan balance"),
    sector: z.string().describe("Industry sector"),
    geography: z.string().describe("Geographic region"),
    status: z.string().describe("Loan status (performing, watchlist, substandard, doubtful, loss)"),
    provision: z.coerce.number().describe("Provision amount"),
    interest_rate: z.coerce.number().describe("Loan interest rate as decimal"),
    maturity_years: z.coerce.number().describe("Remaining maturity in years"),
  })).describe("Individual loan details for portfolio analysis"),
});
