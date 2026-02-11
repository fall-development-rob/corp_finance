import { z } from "zod";

const SectorWeightSchema = z.object({
  sector: z.string().describe("Sector or asset class name"),
  portfolio_weight: z.coerce.number().describe("Portfolio weight in this sector (decimal)"),
  benchmark_weight: z.coerce.number().describe("Benchmark weight in this sector (decimal)"),
  portfolio_return: z.coerce.number().describe("Portfolio return in this sector (decimal)"),
  benchmark_return: z.coerce.number().describe("Benchmark return in this sector (decimal)"),
});

const PeriodDataSchema = z.object({
  period_name: z.string().describe("Period identifier (e.g. 'Q1 2025')"),
  sectors: z.array(SectorWeightSchema).describe("Sector weights and returns for this period"),
});

export const BrinsonSchema = z.object({
  portfolio_name: z.string().describe("Name of the portfolio"),
  benchmark_name: z.string().describe("Name of the benchmark"),
  sectors: z.array(SectorWeightSchema).describe("Sector-level weights and returns for portfolio and benchmark"),
  risk_free_rate: z.coerce.number().describe("Risk-free rate (decimal, e.g. 0.02 for 2%)"),
  periods: z.array(PeriodDataSchema).optional().describe("Optional multi-period data for time-series attribution"),
});

const FactorExposureSchema = z.object({
  factor_name: z.string().describe("Factor name (e.g. 'Market', 'Size', 'Value', 'Momentum')"),
  portfolio_exposure: z.coerce.number().describe("Portfolio exposure (beta) to this factor"),
  benchmark_exposure: z.coerce.number().describe("Benchmark exposure (beta) to this factor"),
  factor_return: z.coerce.number().describe("Factor return over the period (decimal)"),
});

export const FactorAttributionSchema = z.object({
  portfolio_name: z.string().describe("Name of the portfolio"),
  portfolio_return: z.coerce.number().describe("Total portfolio return (decimal)"),
  benchmark_return: z.coerce.number().describe("Total benchmark return (decimal)"),
  factors: z.array(FactorExposureSchema).describe("Factor exposures and returns for attribution decomposition"),
  risk_free_rate: z.coerce.number().describe("Risk-free rate (decimal, e.g. 0.02 for 2%)"),
});
