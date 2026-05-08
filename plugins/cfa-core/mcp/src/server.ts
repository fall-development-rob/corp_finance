#!/usr/bin/env node
/**
 * cfa-core MCP server.
 *
 * Loads the WASM-compiled corp-finance-wasm module and exposes its functions
 * as MCP tools. v0.1 ships 6 tools (WACC, DCF, comps, credit metrics, debt
 * capacity, covenant compliance) — the smallest set needed for the AAPL-style
 * equity workflows + credit screens.
 *
 * Schemas are deliberately permissive: the Rust side validates each input
 * struct via serde and returns descriptive errors. Adding strict zod schemas
 * is a v0.2 enhancement.
 */
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { z } from "zod";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
// Resolve relative to this file so the server works whether invoked from dist/ or src/.
const wasm = require("../wasm/corp_finance_wasm.js") as {
  calculate_wacc: (json: string) => string;
  build_dcf: (json: string) => string;
  comps_analysis: (json: string) => string;
  credit_metrics: (json: string) => string;
  debt_capacity: (json: string) => string;
  covenant_compliance: (json: string) => string;
  // FP&A — Wave 16a pilot port
  variance_analysis: (json: string) => string;
  breakeven_analysis: (json: string) => string;
  working_capital: (json: string) => string;
  rolling_forecast: (json: string) => string;
  // Behavioral — Wave 16x
  prospect_theory: (json: string) => string;
  market_sentiment: (json: string) => string;
  // Performance attribution — Wave 16x
  brinson_attribution: (json: string) => string;
  factor_attribution: (json: string) => string;
  // Quant strategies — Wave 16x
  pairs_trading: (json: string) => string;
  momentum_analysis: (json: string) => string;
  // Equity research — Wave 16x
  sotp_valuation: (json: string) => string;
  target_price: (json: string) => string;
  // Commodity trading — Wave 16x
  commodity_spread: (json: string) => string;
  storage_economics: (json: string) => string;
  // Dividend policy — Wave 16x
  h_model_ddm: (json: string) => string;
  multistage_ddm: (json: string) => string;
  buyback_analysis: (json: string) => string;
  payout_sustainability: (json: string) => string;
  total_shareholder_return: (json: string) => string;
  // Offshore structures — Wave 16y
  cayman_fund_structure: (json: string) => string;
  lux_ireland_fund_structure: (json: string) => string;
  channel_islands_fund_structure: (json: string) => string;
  singapore_vcc_structure: (json: string) => string;
  hong_kong_fund_structure: (json: string) => string;
  middle_east_fund_structure: (json: string) => string;
  jurisdiction_comparison: (json: string) => string;
  fund_migration_analysis: (json: string) => string;
  // Derivatives — Wave 16y
  option_pricer: (json: string) => string;
  implied_volatility: (json: string) => string;
  forward_pricer: (json: string) => string;
  forward_position_value: (json: string) => string;
  futures_basis_analysis: (json: string) => string;
  interest_rate_swap: (json: string) => string;
  currency_swap: (json: string) => string;
  option_strategy: (json: string) => string;
  // Jurisdiction — Wave 16y
  fund_fee_calculator: (json: string) => string;
  gaap_ifrs_reconcile: (json: string) => string;
  withholding_tax: (json: string) => string;
  nav_calculator: (json: string) => string;
  gp_economics: (json: string) => string;
  investor_net_returns: (json: string) => string;
  ubti_screening: (json: string) => string;
  // PE — Wave 16y
  returns_calculator: (json: string) => string;
  debt_schedule: (json: string) => string;
  sources_uses: (json: string) => string;
  lbo_model: (json: string) => string;
  waterfall_calculator: (json: string) => string;
  altman_zscore: (json: string) => string;
  // Fixed income — Wave 16y
  bond_pricer: (json: string) => string;
  bond_yield: (json: string) => string;
  bootstrap_spot_curve: (json: string) => string;
  nelson_siegel_fit: (json: string) => string;
  bond_duration: (json: string) => string;
  credit_spreads: (json: string) => string;
  // Institutional real estate — Wave 16y
  institutional_rent_roll: (json: string) => string;
  institutional_comparable_sales: (json: string) => string;
  institutional_hbu_analysis: (json: string) => string;
  institutional_replacement_cost: (json: string) => string;
  institutional_benchmark: (json: string) => string;
  institutional_acquisition: (json: string) => string;
  // Earnings quality — Wave 16y
  beneish_mscore: (json: string) => string;
  piotroski_fscore: (json: string) => string;
  accrual_quality: (json: string) => string;
  revenue_quality: (json: string) => string;
  earnings_quality_composite: (json: string) => string;
  // Financial forensics — Wave 16y
  benfords_law: (json: string) => string;
  dupont_analysis: (json: string) => string;
  zscore_models: (json: string) => string;
  peer_benchmarking: (json: string) => string;
  red_flag_scoring: (json: string) => string;
  // Bank analytics — Wave 16y
  nim_analysis: (json: string) => string;
  camels_rating: (json: string) => string;
  cecl_provisioning: (json: string) => string;
  deposit_beta: (json: string) => string;
  loan_book_analysis: (json: string) => string;
  // Emerging markets — Wave 16y
  country_risk_premium: (json: string) => string;
  political_risk: (json: string) => string;
  capital_controls: (json: string) => string;
  em_bond_analysis: (json: string) => string;
  em_equity_premium: (json: string) => string;
  // Carbon markets — Wave 16y
  carbon_credit_pricing: (json: string) => string;
  ets_compliance: (json: string) => string;
  cbam_analysis: (json: string) => string;
  offset_valuation: (json: string) => string;
  shadow_carbon_price: (json: string) => string;
  // CLO analytics — Wave 16y
  clo_waterfall: (json: string) => string;
  clo_coverage_tests: (json: string) => string;
  clo_reinvestment: (json: string) => string;
  clo_tranche_analytics: (json: string) => string;
  clo_scenario: (json: string) => string;
  // Fund of funds — Wave 16z
  j_curve_model: (json: string) => string;
  commitment_pacing: (json: string) => string;
  manager_selection: (json: string) => string;
  secondaries_pricing: (json: string) => string;
  fof_portfolio: (json: string) => string;
  // Private wealth — Wave 16z
  concentrated_stock: (json: string) => string;
  philanthropic_vehicles: (json: string) => string;
  wealth_transfer: (json: string) => string;
  direct_indexing: (json: string) => string;
  family_governance: (json: string) => string;
  // Venture — Wave 16z
  funding_round: (json: string) => string;
  dilution_analysis: (json: string) => string;
  convertible_note: (json: string) => string;
  safe_conversion: (json: string) => string;
  venture_fund_model: (json: string) => string;
  // Credit scoring — Wave 16z
  credit_scorecard: (json: string) => string;
  merton_pd: (json: string) => string;
  intensity_model: (json: string) => string;
  pd_calibration: (json: string) => string;
  scoring_validation: (json: string) => string;
  // Capital allocation — Wave 16z
  economic_capital: (json: string) => string;
  raroc_calculation: (json: string) => string;
  euler_allocation: (json: string) => string;
  shapley_allocation: (json: string) => string;
  limit_management: (json: string) => string;
  // Index construction — Wave 16z
  index_weighting: (json: string) => string;
  index_rebalancing: (json: string) => string;
  tracking_error: (json: string) => string;
  smart_beta: (json: string) => string;
  index_reconstitution: (json: string) => string;
  // Regulatory — Wave 16z
  regulatory_capital: (json: string) => string;
  lcr: (json: string) => string;
  nsfr: (json: string) => string;
  alm_analysis: (json: string) => string;
  // Quant risk — Wave 16z
  factor_model: (json: string) => string;
  black_litterman: (json: string) => string;
  risk_parity: (json: string) => string;
  stress_test: (json: string) => string;
  // ESG — Wave 16z
  esg_score: (json: string) => string;
  carbon_footprint: (json: string) => string;
  green_bond: (json: string) => string;
  sll_covenants: (json: string) => string;
  // Insurance — Wave 16z
  loss_reserving: (json: string) => string;
  premium_pricing: (json: string) => string;
  combined_ratio: (json: string) => string;
  solvency_scr: (json: string) => string;
  // FX & commodities — Wave 16z
  fx_forward: (json: string) => string;
  cross_rate: (json: string) => string;
  commodity_forward: (json: string) => string;
  commodity_curve: (json: string) => string;
  // Private credit — Wave 16z
  unitranche_pricing: (json: string) => string;
  direct_loan: (json: string) => string;
  syndication_analysis: (json: string) => string;
  // Wave 16α — wealth/portfolio/macro/etc. (72 tools)
  retirement_planning: (json: string) => string;
  tax_loss_harvesting: (json: string) => string;
  estate_planning: (json: string) => string;
  risk_adjusted_returns: (json: string) => string;
  risk_metrics: (json: string) => string;
  kelly_sizing: (json: string) => string;
  mean_variance_optimization: (json: string) => string;
  black_litterman_portfolio: (json: string) => string;
  monetary_policy: (json: string) => string;
  international_economics: (json: string) => string;
  sensitivity_matrix: (json: string) => string;
  implied_vol_surface: (json: string) => string;
  sabr_calibration: (json: string) => string;
  short_rate_model: (json: string) => string;
  term_structure_fit: (json: string) => string;
  prepayment_analysis: (json: string) => string;
  mbs_analytics: (json: string) => string;
  tips_analytics: (json: string) => string;
  inflation_derivatives: (json: string) => string;
  repo_analytics: (json: string) => string;
  collateral_analytics: (json: string) => string;
  factor_risk_budget: (json: string) => string;
  tail_risk_analysis: (json: string) => string;
  convertible_bond_pricing: (json: string) => string;
  convertible_bond_analysis: (json: string) => string;
  cds_pricing: (json: string) => string;
  cva_calculation: (json: string) => string;
  portfolio_credit_risk: (json: string) => string;
  credit_migration: (json: string) => string;
  pension_funding: (json: string) => string;
  ldi_strategy: (json: string) => string;
  real_option_valuation: (json: string) => string;
  decision_tree_analysis: (json: string) => string;
  structured_note_pricing: (json: string) => string;
  exotic_product_pricing: (json: string) => string;
  lease_classification: (json: string) => string;
  sale_leaseback_analysis: (json: string) => string;
  letter_of_credit: (json: string) => string;
  supply_chain_finance: (json: string) => string;
  token_valuation: (json: string) => string;
  defi_analysis: (json: string) => string;
  beps_compliance: (json: string) => string;
  intercompany_pricing: (json: string) => string;
  treaty_network: (json: string) => string;
  treaty_structure_optimization: (json: string) => string;
  fatca_crs_reporting: (json: string) => string;
  entity_classification: (json: string) => string;
  economic_substance: (json: string) => string;
  jurisdiction_substance_test: (json: string) => string;
  best_execution: (json: string) => string;
  gips_report: (json: string) => string;
  kyc_risk_assessment: (json: string) => string;
  sanctions_screening: (json: string) => string;
  aifmd_reporting: (json: string) => string;
  sec_cftc_reporting: (json: string) => string;
  abs_cashflow_model: (json: string) => string;
  tranching_analysis: (json: string) => string;
  cash_management: (json: string) => string;
  hedge_effectiveness: (json: string) => string;
  ppp_model: (json: string) => string;
  concession_valuation: (json: string) => string;
  sovereign_bond_analysis: (json: string) => string;
  country_risk_assessment: (json: string) => string;
  recovery_analysis: (json: string) => string;
  distressed_debt_analysis: (json: string) => string;
  muni_bond_pricing: (json: string) => string;
  municipal_analysis: (json: string) => string;
  spread_analysis: (json: string) => string;
  optimal_execution: (json: string) => string;
  property_valuation: (json: string) => string;
  project_finance_model: (json: string) => string;
  us_fund_structure: (json: string) => string;
  uk_eu_fund_structure: (json: string) => string;
  // Wave 17b — partial residuals + previously-blocked
  merger_model: (json: string) => string;
  three_statement_model: (json: string) => string;
  monte_carlo_simulation: (json: string) => string;
  monte_carlo_dcf: (json: string) => string;
  scenario_analysis: (json: string) => string;
  version: () => string;
};

function wrap(jsonResult: string) {
  return {
    content: [{ type: "text" as const, text: jsonResult }],
  };
}

// v0.1: pass-through schema. The Rust side validates each input via serde and
// returns a descriptive error if a field is missing or wrongly typed. v0.2
// will add per-tool zod schemas for richer Claude Code tool hints.
const passthroughShape = {
  input: z
    .record(z.any())
    .describe("Tool inputs as a JSON object — see tool description for fields"),
};

function tool<T extends (json: string) => string>(
  server: McpServer,
  name: string,
  description: string,
  fn: T,
) {
  server.tool(
    name,
    description,
    passthroughShape,
    async (params: { input: Record<string, unknown> }) => {
      try {
        const result = fn(JSON.stringify(params.input ?? {}));
        return wrap(result);
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        return {
          content: [{ type: "text" as const, text: `error: ${message}` }],
          isError: true,
        };
      }
    },
  );
}

async function main() {
  const server = new McpServer({
    name: "cfa-core",
    version: wasm.version(),
  });

  tool(
    server,
    "wacc_calculator",
    "Weighted Average Cost of Capital via CAPM. Inputs (all decimals as strings or numbers): risk_free_rate, equity_risk_premium, beta, cost_of_debt, tax_rate, debt_weight, equity_weight. Optional: size_premium, country_risk_premium, specific_risk_premium, unlevered_beta, target_debt_equity (Hamada re-levering). Returns wacc, cost_of_equity, after_tax_cost_of_debt, levered_beta.",
    wasm.calculate_wacc,
  );

  tool(
    server,
    "dcf_model",
    "Discounted Cash Flow valuation with multi-stage projection and Gordon Growth or exit-multiple terminal value. Returns enterprise value, equity value, per-share value, and year-by-year projections.",
    wasm.build_dcf,
  );

  tool(
    server,
    "comps_analysis",
    "Trading comparables analysis across peer set. Computes EV/EBITDA, EV/Revenue, P/E, P/B, PEG mean/median/high/low and derives implied target valuations.",
    wasm.comps_analysis,
  );

  tool(
    server,
    "credit_metrics",
    "Standard credit metrics from financial statements: leverage ratios (Debt/EBITDA, Net Debt/EBITDA), coverage ratios (EBITDA/Interest, FCF/Debt), and liquidity ratios. Returns synthetic rating estimate.",
    wasm.credit_metrics,
  );

  tool(
    server,
    "debt_capacity",
    "Maximum sustainable debt capacity given target leverage, interest coverage, and cash flow projections. Solves backwards from covenant headroom.",
    wasm.debt_capacity,
  );

  tool(
    server,
    "covenant_compliance",
    "Test debt covenant compliance: maintenance covenants (leverage, coverage, fixed-charge), incurrence covenants, and headroom analysis with breach scenarios.",
    wasm.covenant_compliance,
  );

  // FP&A — Wave 16a pilot port
  tool(
    server,
    "variance_analysis",
    "Perform budget-vs-actual variance analysis with price/volume/mix revenue decomposition. Computes revenue variance (price, volume, and mix components), cost variance, profit variance with margin analysis, per-line detail, and optional year-over-year comparison.",
    wasm.variance_analysis,
  );

  tool(
    server,
    "breakeven_analysis",
    "Perform break-even and operating leverage analysis. Computes contribution margin, break-even units and revenue, margin of safety, operating leverage (DOL), target volume for profit goals, and what-if scenario analysis with multiple overrides.",
    wasm.breakeven_analysis,
  );

  tool(
    server,
    "working_capital",
    "Analyse working capital efficiency across multiple periods. Computes DSO, DIO, DPO, Cash Conversion Cycle, net working capital, current/quick ratios, trend analysis, optimization opportunities (cash freed by reducing DSO/DIO), financing savings, and optional industry benchmark comparison.",
    wasm.working_capital,
  );

  tool(
    server,
    "rolling_forecast",
    "Build a rolling financial forecast from historical data. Derives driver assumptions (COGS/OpEx/CapEx as % of revenue) from historical averages or overrides, projects revenue with growth rate, computes EBIT, EBITDA, net income, free cash flow, and summary statistics across forecast periods.",
    wasm.rolling_forecast,
  );

  // Behavioral — Wave 16x
  tool(
    server,
    "prospect_theory",
    "Prospect theory analysis: loss aversion, probability weighting, reference dependence, disposition effect, framing bias.",
    wasm.prospect_theory,
  );
  tool(
    server,
    "market_sentiment",
    "Market sentiment analysis: fear/greed index, put-call ratio, VIX term structure, fund flows, crowding indicators.",
    wasm.market_sentiment,
  );

  // Performance attribution — Wave 16x
  tool(
    server,
    "brinson_attribution",
    "Brinson-Fachler performance attribution: allocation, selection, interaction effects by sector with multi-period linking via Carino method.",
    wasm.brinson_attribution,
  );
  tool(
    server,
    "factor_attribution",
    "Factor-based return attribution: active exposure decomposition, R-squared, tracking error breakdown by systematic factors.",
    wasm.factor_attribution,
  );

  // Quant strategies — Wave 16x
  tool(
    server,
    "pairs_trading",
    "Statistical pairs trading analysis: cointegration, z-scores, half-life, backtested trades, Sharpe ratio.",
    wasm.pairs_trading,
  );
  tool(
    server,
    "momentum_analysis",
    "Momentum factor scoring: risk-adjusted rankings, portfolio construction, backtest, crash risk.",
    wasm.momentum_analysis,
  );

  // Equity research — Wave 16x
  tool(
    server,
    "sotp_valuation",
    "Sum-of-the-parts valuation: segment-level multiples, conglomerate discount, football field analysis.",
    wasm.sotp_valuation,
  );
  tool(
    server,
    "target_price",
    "Multi-method target price: PE, PEG, PB, PS, DDM, analyst consensus with football field and recommendation.",
    wasm.target_price,
  );

  // Commodity trading — Wave 16x
  tool(
    server,
    "commodity_spread",
    "Commodity spread analysis: crack, crush, spark, calendar, location, quality spreads with risk metrics.",
    wasm.commodity_spread,
  );
  tool(
    server,
    "storage_economics",
    "Commodity storage economics: contango/backwardation, convenience yields, cash-and-carry arbitrage, seasonal analysis.",
    wasm.storage_economics,
  );

  // Dividend policy — Wave 16x
  tool(
    server,
    "h_model_ddm",
    "H-Model DDM: Fuller & Hsia dividend valuation with linearly declining growth from short-term to long-term rate over half-life.",
    wasm.h_model_ddm,
  );
  tool(
    server,
    "multistage_ddm",
    "Multi-stage DDM: N-stage dividend discount model with explicit growth periods and terminal Gordon Growth value.",
    wasm.multistage_ddm,
  );
  tool(
    server,
    "buyback_analysis",
    "Share buyback analysis: EPS accretion/dilution, P/E breakeven, tax efficiency vs dividends, debt-funded vs cash-funded comparison.",
    wasm.buyback_analysis,
  );
  tool(
    server,
    "payout_sustainability",
    "Payout sustainability analysis: payout ratio, FCF coverage, debt capacity, dividend safety score, Lintner smoothing model.",
    wasm.payout_sustainability,
  );
  tool(
    server,
    "total_shareholder_return",
    "Total shareholder return: price appreciation, dividend yield, buyback yield, annualized TSR, component attribution.",
    wasm.total_shareholder_return,
  );

  // Offshore structures — Wave 16y
  tool(server, "cayman_fund_structure", "Cayman/BVI offshore fund structure: Exempted LP, SPC, Unit Trust, BVI BCA with master-feeder economics, CIMA registration, economic substance.", wasm.cayman_fund_structure);
  tool(server, "lux_ireland_fund_structure", "Luxembourg/Ireland fund structure: SICAV-SIF, RAIF, SCSp, ICAV, QIAIF, Section 110 with subscription tax, AIFMD passport, UCITS analysis.", wasm.lux_ireland_fund_structure);
  tool(server, "channel_islands_fund_structure", "Channel Islands fund structures: Jersey JPF/Expert/QIF, Guernsey PIF/QIF/RQIF, Protected/Incorporated Cell Companies.", wasm.channel_islands_fund_structure);
  tool(server, "singapore_vcc_structure", "Singapore Variable Capital Company (VCC): standalone/umbrella, sub-fund allocation, RFMC/LRFMC/A-LFMC licensing, S13O/S13U/S13D incentives.", wasm.singapore_vcc_structure);
  tool(server, "hong_kong_fund_structure", "Hong Kong Open-ended Fund Company (OFC): public/private, umbrella sub-funds, SFC Type 9 licensing, OFC grant scheme.", wasm.hong_kong_fund_structure);
  tool(server, "middle_east_fund_structure", "Middle East fund structures: DIFC QIF/Exempt/Domestic, Sharia compliance, DFSA regulatory framework.", wasm.middle_east_fund_structure);
  tool(server, "jurisdiction_comparison", "Multi-jurisdiction comparison: weighted scoring across setup cost, annual cost, tax, regulatory speed, distribution reach, substance for 10 offshore domiciles.", wasm.jurisdiction_comparison);
  tool(server, "fund_migration_analysis", "Fund migration/redomiciliation feasibility: statutory continuation, scheme of arrangement, investor consent, timeline, cost across offshore corridors.", wasm.fund_migration_analysis);

  // Derivatives — Wave 16y
  tool(server, "option_pricer", "Price an option using Black-Scholes or binomial model — price, Greeks (delta, gamma, theta, vega, rho), intrinsic/time value.", wasm.option_pricer);
  tool(server, "implied_volatility", "Solve for implied volatility from a market option price using Newton-Raphson on Black-Scholes.", wasm.implied_volatility);
  tool(server, "forward_pricer", "Price a forward/futures contract using cost-of-carry — financial, commodity, or currency underlying.", wasm.forward_pricer);
  tool(server, "forward_position_value", "Value an existing forward position — current value, unrealised P&L, updated forward price.", wasm.forward_position_value);
  tool(server, "futures_basis_analysis", "Analyse futures basis — raw basis, fair value, mispricing, implied repo rate, contango/backwardation.", wasm.futures_basis_analysis);
  tool(server, "interest_rate_swap", "Value an interest rate swap — NPV, fixed/floating leg PV, cashflow schedule, par swap rate, DV01.", wasm.interest_rate_swap);
  tool(server, "currency_swap", "Value a currency swap — NPV in domestic currency, domestic/foreign leg PVs.", wasm.currency_swap);
  tool(server, "option_strategy", "Multi-leg option strategy analysis — payoff diagram, breakevens, max profit/loss, aggregated Greeks.", wasm.option_strategy);

  // Jurisdiction — Wave 16y
  tool(server, "fund_fee_calculator", "Fund fee economics: management fee + carry + LP/GP splits across multi-tranche capital structures.", wasm.fund_fee_calculator);
  tool(server, "gaap_ifrs_reconcile", "GAAP/IFRS accounting standards reconciliation across revenue, leases, financial instruments, intangibles, deferred tax.", wasm.gaap_ifrs_reconcile);
  tool(server, "withholding_tax", "Cross-border withholding tax: per-jurisdiction rates, treaty relief, gross-up calc, post-tax yield.", wasm.withholding_tax);
  tool(server, "nav_calculator", "Fund NAV calculation with equalisation, side-pocket carve-out, performance-fee allocation, share-class hedging.", wasm.nav_calculator);
  tool(server, "gp_economics", "GP economics: management-fee revenue, GP carry waterfall, GP commitment, claw-back, key-person provisions.", wasm.gp_economics);
  tool(server, "investor_net_returns", "Investor net returns after fees, taxes, FX — gross-to-net waterfall, IRR/MOIC, hurdle/catch-up impact.", wasm.investor_net_returns);
  tool(server, "ubti_screening", "UBTI/ECI screening for US tax-exempt investors: blocker analysis, debt-financed property exposure, leverage tests.", wasm.ubti_screening);

  // PE — Wave 16y
  tool(server, "returns_calculator", "Investment returns: IRR, XIRR (irregular dates), MOIC, cash-on-cash from cash flow series.", wasm.returns_calculator);
  tool(server, "debt_schedule", "Year-by-year debt amortisation schedule for a tranche: bullet/straight-line/cash-sweep, floating rates, PIK, commitment fees.", wasm.debt_schedule);
  tool(server, "sources_uses", "Sources & uses of funds for a transaction (LBO, M&A, recap). Validates total sources = total uses.", wasm.sources_uses);
  tool(server, "lbo_model", "Full leveraged buyout model: multi-tranche debt, cash sweep, year-by-year projections, exit IRR/MOIC, sources & uses, credit metrics.", wasm.lbo_model);
  tool(server, "waterfall_calculator", "GP/LP distribution waterfall: return of capital, preferred return, GP catch-up, carried interest. European and American.", wasm.waterfall_calculator);
  tool(server, "altman_zscore", "Altman Z-Score for bankruptcy prediction. Original Z (public manufacturing), Z-prime (private), Z-double-prime (non-manufacturing/EM).", wasm.altman_zscore);

  // Fixed income — Wave 16y
  tool(server, "bond_pricer", "Price a bond — clean/dirty prices, accrued interest, current yield, cashflow schedule, YTC/YTW for callable.", wasm.bond_pricer);
  tool(server, "bond_yield", "Bond yield metrics — YTM (Newton-Raphson), current yield, BEY, effective annual yield.", wasm.bond_yield);
  tool(server, "bootstrap_spot_curve", "Bootstrap a zero-coupon spot rate curve from par instruments — spot rates, forward rates, discount factors.", wasm.bootstrap_spot_curve);
  tool(server, "nelson_siegel_fit", "Fit a Nelson-Siegel yield curve to observed yields — beta parameters, fitted rates, RMSE.", wasm.nelson_siegel_fit);
  tool(server, "bond_duration", "Bond duration, convexity, DV01, key rate durations — Macaulay, modified, effective.", wasm.bond_duration);
  tool(server, "credit_spreads", "Credit spreads — I-spread, G-spread, Z-spread, spread duration, CDS spread estimate, credit quality indicator.", wasm.credit_spreads);

  // Institutional real estate — Wave 16y
  tool(server, "institutional_rent_roll", "Argus-style tenant-by-tenant cash flow projection with escalation schedules, WALT, mark-to-market gap.", wasm.institutional_rent_roll);
  tool(server, "institutional_comparable_sales", "Sales comparison approach with structured adjustment grid (equal weight, quality score, inverse distance).", wasm.institutional_comparable_sales);
  tool(server, "institutional_hbu_analysis", "Highest & best use analysis: legal permissibility, physical possibility, financial feasibility, maximum productivity.", wasm.institutional_hbu_analysis);
  tool(server, "institutional_replacement_cost", "Cost approach: RCN from Marshall & Swift base costs minus accrued depreciation plus land value.", wasm.institutional_replacement_cost);
  tool(server, "institutional_benchmark", "NCREIF-style return attribution: income return, appreciation return, alpha, tracking error, information ratio.", wasm.institutional_benchmark);
  tool(server, "institutional_acquisition", "Full acquisition underwriting: cap rate, NOI projection, exit valuation, multi-tranche debt, levered/unlevered IRR, DSCR.", wasm.institutional_acquisition);

  // Earnings quality — Wave 16y
  tool(server, "beneish_mscore", "Beneish M-Score: 8-variable earnings manipulation model (DSRI, GMI, AQI, SGI, DEPI, SGAI, LVGI, TATA) with manipulation flag.", wasm.beneish_mscore);
  tool(server, "piotroski_fscore", "Piotroski F-Score: 9-signal fundamental strength (profitability, leverage, efficiency) with component breakdown.", wasm.piotroski_fscore);
  tool(server, "accrual_quality", "Accrual quality: Sloan ratio, Dechow-Dichev, Jones / modified-Jones models, cash conversion metrics.", wasm.accrual_quality);
  tool(server, "revenue_quality", "Revenue quality: receivables DSO trend, deferred revenue growth, HHI concentration, allowance-to-receivables.", wasm.revenue_quality);
  tool(server, "earnings_quality_composite", "Composite earnings quality: weighted blend of Beneish, Piotroski, accrual, revenue quality with traffic-light rating.", wasm.earnings_quality_composite);

  // Financial forensics — Wave 16y
  tool(server, "benfords_law", "Benford's Law analysis: first/second/first-two digit distribution test, chi-squared and MAD statistics, conformity assessment.", wasm.benfords_law);
  tool(server, "dupont_analysis", "DuPont decomposition: 3-step and 5-step ROE breakdown with trend.", wasm.dupont_analysis);
  tool(server, "zscore_models", "Z-Score models: Altman, Ohlson, Zmijewski, Springate with distress classification.", wasm.zscore_models);
  tool(server, "peer_benchmarking", "Peer benchmarking: percentile ranking, z-score normalization, composite scoring across multiple metrics.", wasm.peer_benchmarking);
  tool(server, "red_flag_scoring", "Red flag scoring: composite fraud/distress risk from Beneish, Altman, Piotroski, ratios, qualitative indicators.", wasm.red_flag_scoring);

  // Bank analytics — Wave 16y
  tool(server, "nim_analysis", "Net interest margin: NIM calc, rate/volume decomposition, asset/liability mix contribution, IR gap.", wasm.nim_analysis);
  tool(server, "camels_rating", "CAMELS bank rating: Capital, Asset quality, Management, Earnings, Liquidity, Sensitivity composite (1-5).", wasm.camels_rating);
  tool(server, "cecl_provisioning", "CECL/IFRS 9 expected credit loss: multi-scenario weighted ECL by segment, stage classification, lifetime vs 12-month provision.", wasm.cecl_provisioning);
  tool(server, "deposit_beta", "Deposit beta: pass-through rate estimation, cumulative beta, asymmetry (up vs down cycles), repricing lag.", wasm.deposit_beta);
  tool(server, "loan_book_analysis", "Loan book analysis: sector/geography concentration (HHI), NPL analysis, provision adequacy, weighted average rate and maturity.", wasm.loan_book_analysis);

  // Emerging markets — Wave 16y
  tool(server, "country_risk_premium", "Country risk premium: Damodaran sovereign spread, relative volatility, composite premium with governance and macro adjustments.", wasm.country_risk_premium);
  tool(server, "political_risk", "Political risk assessment: WGI composite, MIGA insurance valuation, expropriation/sanctions/conflict quantification.", wasm.political_risk);
  tool(server, "capital_controls", "Capital controls: repatriation delay cost, withholding drag, FX conversion cost, effective yield impact.", wasm.capital_controls);
  tool(server, "em_bond_analysis", "EM bond analysis: local vs hard currency, FX-adjusted yield, carry trade decomposition, hedged/unhedged scenarios.", wasm.em_bond_analysis);
  tool(server, "em_equity_premium", "EM equity risk premium: sovereign spread, relative volatility, composite ERP with valuation/growth adjustments.", wasm.em_equity_premium);

  // Carbon markets — Wave 16y
  tool(server, "carbon_credit_pricing", "Carbon credit pricing: forward price (cost-of-carry), vintage discount, registry premium, credit type adjustment.", wasm.carbon_credit_pricing);
  tool(server, "ets_compliance", "ETS compliance: allowance surplus/deficit, compliance cost, price volatility, carbon intensity vs benchmark.", wasm.ets_compliance);
  tool(server, "cbam_analysis", "EU CBAM: certificate cost per good, net liability after origin carbon-price credit, total exposure.", wasm.cbam_analysis);
  tool(server, "offset_valuation", "Carbon offset valuation: quality-adjusted price, permanence/additionality/vintage/certification adjustments, co-benefit premium.", wasm.offset_valuation);
  tool(server, "shadow_carbon_price", "Shadow carbon price: carbon-adjusted NPV, abatement cost, project ranking with/without carbon pricing, breakeven price.", wasm.shadow_carbon_price);

  // CLO analytics — Wave 16y
  tool(server, "clo_waterfall", "CLO interest/principal waterfall: per-tranche payment, OC/IC test outcomes, equity residual.", wasm.clo_waterfall);
  tool(server, "clo_coverage_tests", "CLO O/C and I/C coverage tests: par-haircut adjustments, breach scenarios, pass/fail vs trigger.", wasm.clo_coverage_tests);
  tool(server, "clo_reinvestment", "CLO reinvestment period analytics: vintage diversification, par build-up, prepayment recycling.", wasm.clo_reinvestment);
  tool(server, "clo_tranche_analytics", "CLO tranche analytics: WAL, weighted-avg coupon, attachment/detachment points, par subordination.", wasm.clo_tranche_analytics);
  tool(server, "clo_scenario", "CLO scenario analysis: default/loss/prepayment stress, equity returns, rating-migration impact across waterfall.", wasm.clo_scenario);

  // Fund of funds — Wave 16z
  tool(server, "j_curve_model", "J-curve fund lifecycle: cash flow projection, TVPI/DPI/RVPI, PME (Kaplan-Schoar), net/gross IRR, trough analysis.", wasm.j_curve_model);
  tool(server, "commitment_pacing", "Commitment pacing: vintage year allocation, drawdown modeling, NAV projection, over-commitment ratio.", wasm.commitment_pacing);
  tool(server, "manager_selection", "Manager due diligence: performance scoring, persistence analysis, alpha estimation, qualitative rating.", wasm.manager_selection);
  tool(server, "secondaries_pricing", "Secondaries pricing: NAV discount, unfunded PV, IRR sensitivity at multiple exit multiples, breakeven.", wasm.secondaries_pricing);
  tool(server, "fof_portfolio", "Fund of funds portfolio: diversification by strategy/vintage/geography, HHI, constraint monitoring.", wasm.fof_portfolio);

  // Private wealth — Wave 16z
  tool(server, "concentrated_stock", "Concentrated stock: collar, exchange fund, prepaid forward, charitable strategies with tax-adjusted after-tax comparison.", wasm.concentrated_stock);
  tool(server, "philanthropic_vehicles", "Philanthropic vehicles: CRT, CLT, DAF, private foundation with tax deduction, income stream, remainder analysis.", wasm.philanthropic_vehicles);
  tool(server, "wealth_transfer", "Wealth transfer: estate tax, GST, annual exclusion, GRAT, grantor trust, dynasty trust, ILIT analysis with tax savings.", wasm.wealth_transfer);
  tool(server, "direct_indexing", "Direct indexing: tax-loss harvesting opportunities, wash sale compliance, tracking error, after-tax alpha.", wasm.direct_indexing);
  tool(server, "family_governance", "Family governance: governance score, complexity assessment, structure recommendations, risk identification.", wasm.family_governance);

  // Venture — Wave 16z
  tool(server, "funding_round", "Model a single VC funding round with option-pool shuffle, pre/post-money valuations, dilution.", wasm.funding_round);
  tool(server, "dilution_analysis", "Dilution analysis across multiple funding rounds, ownership trajectories, anti-dilution effects.", wasm.dilution_analysis);
  tool(server, "convertible_note", "Convert a convertible note into equity at the most favourable price for the note-holder (cap, discount, MFN).", wasm.convertible_note);
  tool(server, "safe_conversion", "Convert a SAFE into shares at a qualifying financing event (post-money/pre-money, cap, discount).", wasm.safe_conversion);
  tool(server, "venture_fund_model", "VC fund returns over full lifecycle: TVPI, DPI, IRR, vintage diversification, distribution waterfall.", wasm.venture_fund_model);

  // Credit scoring — Wave 16z
  tool(server, "credit_scorecard", "Logistic regression scorecard: WoE binning, IV calculation, scorecard points, Gini, KS statistic.", wasm.credit_scorecard);
  tool(server, "merton_pd", "Merton structural model: asset value/volatility, distance to default, PD, KMV EDF.", wasm.merton_pd);
  tool(server, "intensity_model", "Reduced-form intensity model: hazard rates from CDS spreads, survival probability, term structure.", wasm.intensity_model);
  tool(server, "pd_calibration", "PIT/TTC PD calibration: Vasicek single-factor, Basel IRB correlation, central tendency.", wasm.pd_calibration);
  tool(server, "scoring_validation", "Credit model validation: AUC-ROC, accuracy ratio, Gini, Brier score, Hosmer-Lemeshow.", wasm.scoring_validation);

  // Capital allocation — Wave 16z
  tool(server, "economic_capital", "Economic capital: VaR/ES-based capital, IRB capital requirement (Basel), stress capital buffer, adequacy ratio.", wasm.economic_capital);
  tool(server, "raroc_calculation", "RAROC: risk-adjusted return on capital, RORAC, EVA, SVA, spread to hurdle, risk-adjusted pricing.", wasm.raroc_calculation);
  tool(server, "euler_allocation", "Euler risk contribution: marginal capital allocation, diversification benefit, HHI concentration.", wasm.euler_allocation);
  tool(server, "shapley_allocation", "Shapley value capital allocation: game-theoretic fair allocation (exact N≤8, sampled N>8).", wasm.shapley_allocation);
  tool(server, "limit_management", "Risk limit management: notional/VaR/concentration limits, utilization tracking, breach detection.", wasm.limit_management);

  // Index construction — Wave 16z
  tool(server, "index_weighting", "Index weighting: market-cap, equal, fundamental, free-float with cap constraints, HHI concentration, sector breakdown.", wasm.index_weighting);
  tool(server, "index_rebalancing", "Index rebalancing: drift analysis, optimal trade list, transaction cost estimation, turnover, liquidity-adjusted scheduling.", wasm.index_rebalancing);
  tool(server, "tracking_error", "Tracking error: ex-post (returns) and ex-ante (weights/covariance), active share, information ratio decomposition.", wasm.tracking_error);
  tool(server, "smart_beta", "Smart beta: multi-factor tilted weights (value, momentum, quality, low-vol, dividend) with factor exposure and risk analysis.", wasm.smart_beta);
  tool(server, "index_reconstitution", "Index reconstitution: member eligibility screening, additions/deletions, buffer zone, turnover, impact analysis.", wasm.index_reconstitution);

  // Regulatory — Wave 16z
  tool(server, "regulatory_capital", "Basel III/IV regulatory capital and RWA: credit SA risk weights with CRM, op risk (BIA/SA), market risk charge, CET1/Tier1/total ratios, buffers.", wasm.regulatory_capital);
  tool(server, "lcr", "Basel III LCR: HQLA L1/L2A/L2B with haircuts and composition caps, run-off factors, 75% inflow cap, pass/fail vs 100% minimum.", wasm.lcr);
  tool(server, "nsfr", "Basel III NSFR: ASF/RSF factors per category, pass/fail vs 100% minimum.", wasm.nsfr);
  tool(server, "alm_analysis", "ALM/IRRBB: repricing and maturity gap, NII sensitivity, EVE sensitivity, duration gap across rate scenarios.", wasm.alm_analysis);

  // Quant risk — Wave 16z
  tool(server, "factor_model", "OLS factor regression: CAPM, Fama-French 3F, Carhart 4F, or custom. Returns alpha, factor betas + t-stats, R², adj R², DW, IR.", wasm.factor_model);
  tool(server, "black_litterman", "Black-Litterman portfolio optimisation: combines market equilibrium with investor views. Returns posterior returns, optimal weights, return/vol/Sharpe.", wasm.black_litterman);
  tool(server, "risk_parity", "Risk-parity portfolio construction: inverse-vol, ERC, or min-variance. Returns weights, per-asset risk contributions, diversification ratio, effective N.", wasm.risk_parity);
  tool(server, "stress_test", "Portfolio stress testing: equity/rate/credit/FX/commodity/vol shocks mapped to positions. Returns per-scenario P&L, VaR breach flags, worst-case.", wasm.stress_test);

  // ESG — Wave 16z
  tool(server, "esg_score", "ESG score with E/S/G pillar weighting, sector materiality mapping, peer benchmarking, red/amber/green flags. 0-100 score, AAA-CCC rating.", wasm.esg_score);
  tool(server, "carbon_footprint", "Carbon footprint: Scope 1/2/3 emissions, intensity (tCO2e/$M revenue), carbon cost exposure, SBTi target gap, implied temperature.", wasm.carbon_footprint);
  tool(server, "green_bond", "Green bond greenium: bps premium, PV of coupon savings, total CO2 impact, cost per tonne avoided, ICMA/CBI/EU Taxonomy alignment.", wasm.green_bond);
  tool(server, "sll_covenants", "Sustainability-linked loan covenants: per-KPI progress vs SPT, met/unmet flag, adjusted margin, annual savings.", wasm.sll_covenants);

  // Insurance — Wave 16z
  tool(server, "loss_reserving", "Insurance loss reserves: Chain-Ladder + Bornhuetter-Ferguson — age-to-age factors, ultimates, IBNR, tail factors, optional PV discount.", wasm.loss_reserving);
  tool(server, "premium_pricing", "Insurance premium pricing: frequency × severity pure premium, trended gross premium with expense/profit/RI loadings, rate breakdown.", wasm.premium_pricing);
  tool(server, "combined_ratio", "Insurance combined ratio: loss, LAE, expense, dividend ratios per period; summary stats, trend, best/worst, profitable-year count.", wasm.combined_ratio);
  tool(server, "solvency_scr", "Solvency II Standard Formula SCR: non-life UW SCR, cat SCR, market/credit/op SCR, diversification, MCR, solvency ratio, surplus.", wasm.solvency_scr);

  // FX & commodities — Wave 16z
  tool(server, "fx_forward", "FX forward pricing: covered interest parity, forward points, swap basis, all-in rate.", wasm.fx_forward);
  tool(server, "cross_rate", "FX cross rate: triangular arbitrage, bid/ask cross via vehicle currency, transaction-cost-adjusted rate.", wasm.cross_rate);
  tool(server, "commodity_forward", "Commodity forward: cost-of-carry, convenience yield, storage cost, forward curve, contango/backwardation flag.", wasm.commodity_forward);
  tool(server, "commodity_curve", "Commodity term structure analysis: roll yield, term-structure slope, calendar spread, seasonality.", wasm.commodity_curve);

  // Private credit — Wave 16z
  tool(server, "unitranche_pricing", "Unitranche facility: FO/LO split, blended yield, call protection, leverage metrics.", wasm.unitranche_pricing);
  tool(server, "direct_loan", "Direct lending loan: cash/PIK toggle, delayed draw, amortization, credit VaR, default-adjusted yield.", wasm.direct_loan);
  tool(server, "syndication_analysis", "Loan syndication economics: arranger vs participant fees, oversubscription, pro-rata allocations, per-participant economics.", wasm.syndication_analysis);

// Wave 16α — bulk port (72 tools across 36 modules)
  tool(server, "retirement_planning", "Retirement plan: contribution-to-replacement-ratio, glide-path, Monte-Carlo-free deterministic projection across multiple income/expense scenarios.", wasm.retirement_planning);
  tool(server, "tax_loss_harvesting", "Tax-loss harvesting: simulate per-lot harvesting with wash-sale compliance, after-tax alpha, replacement-asset tracking error.", wasm.tax_loss_harvesting);
  tool(server, "estate_planning", "Estate planning: federal/state estate tax, GST, annual exclusion, bypass trusts, gift strategies with after-tax wealth comparison.", wasm.estate_planning);
  tool(server, "risk_adjusted_returns", "Portfolio risk-adjusted return metrics: Sharpe, Sortino, Treynor, Information Ratio, Calmar with rolling-window analysis.", wasm.risk_adjusted_returns);
  tool(server, "risk_metrics", "Portfolio risk metrics: VaR, CVaR (parametric/historical/Monte Carlo deterministic), max drawdown, downside deviation, beta, tracking error.", wasm.risk_metrics);
  tool(server, "kelly_sizing", "Kelly criterion position sizing: full Kelly, fractional Kelly (0.25/0.5), max-drawdown constraint, growth-rate optimisation.", wasm.kelly_sizing);
  tool(server, "mean_variance_optimization", "Markowitz mean-variance optimisation: efficient frontier, max-Sharpe portfolio, constrained optimisation with min/max weights.", wasm.mean_variance_optimization);
  tool(server, "black_litterman_portfolio", "Black-Litterman portfolio: posterior returns from prior + views, optimal weights vs implied-equilibrium baseline.", wasm.black_litterman_portfolio);
  tool(server, "monetary_policy", "Monetary policy: Taylor rule, Phillips curve, Okun's law, recession risk indicators.", wasm.monetary_policy);
  tool(server, "international_economics", "International economics: PPP, interest rate parity, real exchange rate, balance of payments analysis.", wasm.international_economics);
  tool(server, "sensitivity_matrix", "Sensitivity matrix: 2D parameter sweep with output grid, optionally with breakeven contour.", wasm.sensitivity_matrix);
  tool(server, "implied_vol_surface", "Implied volatility surface: cubic-spline interpolation across strikes/maturities, ATM term structure, smile dynamics.", wasm.implied_vol_surface);
  tool(server, "sabr_calibration", "SABR model calibration: alpha, beta, rho, nu fit to market vol smile via least-squares.", wasm.sabr_calibration);
  tool(server, "short_rate_model", "Short-rate models: Vasicek, CIR, Hull-White zero-coupon bond pricing, calibration to term structure.", wasm.short_rate_model);
  tool(server, "term_structure_fit", "Term structure fitting: Nelson-Siegel-Svensson with constrained optimisation, RMSE diagnostic.", wasm.term_structure_fit);
  tool(server, "prepayment_analysis", "Prepayment analysis: PSA / CPR / SMM models, refi-burnout, prepayment-adjusted cash flows.", wasm.prepayment_analysis);
  tool(server, "mbs_analytics", "MBS analytics: WAL, OAS estimate, prepayment-adjusted yield, duration with prepayment risk.", wasm.mbs_analytics);
  tool(server, "tips_analytics", "TIPS analytics: real yield, breakeven inflation, inflation-adjusted cash flows, real duration.", wasm.tips_analytics);
  tool(server, "inflation_derivatives", "Inflation derivatives: zero-coupon inflation swap, inflation cap/floor, inflation-linked annuity pricing.", wasm.inflation_derivatives);
  tool(server, "repo_analytics", "Repo analytics: special vs general collateral, implied repo, term repo, haircut economics.", wasm.repo_analytics);
  tool(server, "collateral_analytics", "Collateral analytics: eligibility checking, haircut application, concentration limits, optimisation.", wasm.collateral_analytics);
  tool(server, "factor_risk_budget", "Factor risk budget: per-factor risk contribution, marginal/total risk, ex-ante tracking error decomposition.", wasm.factor_risk_budget);
  tool(server, "tail_risk_analysis", "Tail risk: VaR/CVaR (multiple methods), expected shortfall, tail-correlation, conditional drawdown.", wasm.tail_risk_analysis);
  tool(server, "convertible_bond_pricing", "Convertible bond pricing: bond-floor, conversion value, parity, premium decomposition; binomial tree with credit-adjusted discount.", wasm.convertible_bond_pricing);
  tool(server, "convertible_bond_analysis", "Convertible bond analysis: delta, gamma, vega, rho, breakeven recovery, in-the-money probability.", wasm.convertible_bond_analysis);
  tool(server, "cds_pricing", "CDS pricing: par spread, hazard rate calibration, mark-to-market, upfront-with-fixed-coupon.", wasm.cds_pricing);
  tool(server, "cva_calculation", "Counterparty CVA: expected exposure, unilateral CVA, bilateral CVA + DVA, wrong-way risk uplift.", wasm.cva_calculation);
  tool(server, "portfolio_credit_risk", "Portfolio credit risk: aggregate EL, UL, RAROC, expected shortfall, correlation effect on tail loss.", wasm.portfolio_credit_risk);
  tool(server, "credit_migration", "Credit migration: rating transition matrix, multi-period migration, MTM impact across rating moves.", wasm.credit_migration);
  tool(server, "pension_funding", "Pension funding: PBO/ABO, funded ratio, NPPC, contribution requirements, cohort liabilities.", wasm.pension_funding);
  tool(server, "ldi_strategy", "Liability-driven investing: duration-matching, hedging portfolio, glide-path schedule with funding-ratio triggers.", wasm.ldi_strategy);
  tool(server, "real_option_valuation", "Real options: CRR binomial — Expand/Abandon/Defer/Switch/Contract/Compound with Greeks and exercise boundary.", wasm.real_option_valuation);
  tool(server, "decision_tree_analysis", "Decision tree: EMV rollback, EVPI, sensitivity, optimal path, risk-adjusted value.", wasm.decision_tree_analysis);
  tool(server, "structured_note_pricing", "Structured note pricing: capital-protected, reverse convertible, participation, credit-linked notes via Black-Scholes decomposition.", wasm.structured_note_pricing);
  tool(server, "exotic_product_pricing", "Exotic options: autocallables (observation schedule), barrier options (Rubinstein-Reiner), digitals (cash/asset-or-nothing) with Greeks.", wasm.exotic_product_pricing);
  tool(server, "lease_classification", "Lease classification: ASC 842 / IFRS 16 five-test, ROU asset, lease liability, monthly amortisation schedule.", wasm.lease_classification);
  tool(server, "sale_leaseback_analysis", "Sale-leaseback: ASC 606 sale-qualification, gain/loss recognition, deferred gain, failed-sale financing obligation.", wasm.sale_leaseback_analysis);
  tool(server, "letter_of_credit", "Letter of credit pricing: fee, all-in cost, risk score, banker's-acceptance discounting across 5 LC subtypes.", wasm.letter_of_credit);
  tool(server, "supply_chain_finance", "Supply chain finance: reverse factoring, dynamic discounting, forfaiting, ECA-backed export finance.", wasm.supply_chain_finance);
  tool(server, "token_valuation", "Token valuation: NVT, P/S, FDV vs circulating market cap, DCF of protocol revenue, relative-valuation peer analysis.", wasm.token_valuation);
  tool(server, "defi_analysis", "DeFi yield: gas-adjusted APR/APY, impermanent loss, staking effective yield, liquidity-pool fee income, pool share.", wasm.defi_analysis);
  tool(server, "beps_compliance", "OECD BEPS / CbCR / Pillar Two analysis: GloBE income, ETR, top-up tax, transitional safe harbours.", wasm.beps_compliance);
  tool(server, "intercompany_pricing", "Transfer pricing: CUP/RPM/CPLM/TNMM/Profit-Split, GILTI, ATAD, arm's-length range and median.", wasm.intercompany_pricing);
  tool(server, "treaty_network", "Tax treaty network: WHT optimisation, conduit routing, LOB / PPT scoring across multi-jurisdiction structures.", wasm.treaty_network);
  tool(server, "treaty_structure_optimization", "Treaty structure optimisation: multi-jurisdiction holding, IP box, PE risk, after-tax effective rate.", wasm.treaty_structure_optimization);
  tool(server, "fatca_crs_reporting", "FATCA/CRS reporting: IGA classification, reportable accounts, GIIN validation, CRS jurisdiction mapping.", wasm.fatca_crs_reporting);
  tool(server, "entity_classification", "FATCA/CRS entity classification: FFI/NFFE determination, passive/active NFFE, sponsored FFI, reporting model.", wasm.entity_classification);
  tool(server, "economic_substance", "Economic substance: 0-100 score, CIGA breakdown, penalty exposure, jurisdiction-by-jurisdiction comparison.", wasm.economic_substance);
  tool(server, "jurisdiction_substance_test", "Jurisdiction substance tests: Directed & Managed / CM&C / POEM / ATAD / Tax Incentive evaluation.", wasm.jurisdiction_substance_test);
  tool(server, "best_execution", "MiFID II best execution: Perold implementation shortfall, RTS 27/28 compliance scoring.", wasm.best_execution);
  tool(server, "gips_report", "GIPS reporting: Modified Dietz returns, geometric linking, composite dispersion, Sharpe/IR with verification flags.", wasm.gips_report);
  tool(server, "kyc_risk_assessment", "KYC risk: composite scoring, SDD/CDD/EDD tier determination, country/PEP/sector overlay.", wasm.kyc_risk_assessment);
  tool(server, "sanctions_screening", "Sanctions screening: fuzzy Levenshtein name matching, multi-list screening (OFAC/EU/HMT/UN/FATF), country risk.", wasm.sanctions_screening);
  tool(server, "aifmd_reporting", "AIFMD Annex IV: AUM-based reporting frequency, leverage (gross/net), liquidity profile, market exposure.", wasm.aifmd_reporting);
  tool(server, "sec_cftc_reporting", "Form PF / CPO-PQR: adviser classification, qualifying hedge fund, threshold + borrowing analyses.", wasm.sec_cftc_reporting);
  tool(server, "abs_cashflow_model", "ABS/MBS cash flow: CPR/PSA/SMM prepayment, CDR/SDA defaults, WAL, principal/interest waterfall.", wasm.abs_cashflow_model);
  tool(server, "tranching_analysis", "Tranching: CDO/CLO waterfall, OC/IC tests, per-tranche IRR/WAL, equity residual.", wasm.tranching_analysis);
  tool(server, "cash_management", "Cash management: optimal cash holdings, sweep economics, working-capital efficiency, payment-cycle analysis.", wasm.cash_management);
  tool(server, "hedge_effectiveness", "Hedge effectiveness: dollar-offset, regression, hypothetical-derivative, hedge-ratio range testing.", wasm.hedge_effectiveness);
  tool(server, "ppp_model", "PPP / project finance: revenue model, debt-service-cover ratio, equity IRR, government subsidy IRR.", wasm.ppp_model);
  tool(server, "concession_valuation", "Concession valuation: NPV, IRR, MIRR, sensitivity to traffic/utility/tariff assumptions.", wasm.concession_valuation);
  tool(server, "sovereign_bond_analysis", "Sovereign bond: YTM, Z-spread, duration, sovereign-CDS basis, real-yield decomposition.", wasm.sovereign_bond_analysis);
  tool(server, "country_risk_assessment", "Country risk: composite score across political, economic, financial, governance dimensions; quintile ranking.", wasm.country_risk_assessment);
  tool(server, "recovery_analysis", "Distressed recovery: enterprise valuation under restructuring, claim ranking, recovery percentage by class.", wasm.recovery_analysis);
  tool(server, "distressed_debt_analysis", "Distressed debt: par-yield-distressed, fulcrum security identification, exchange offer NPV.", wasm.distressed_debt_analysis);
  tool(server, "muni_bond_pricing", "Municipal bond pricing: tax-equivalent yield, AMT-adjusted yield, callable bond YTC, bank-qualified discount.", wasm.muni_bond_pricing);
  tool(server, "municipal_analysis", "Municipal credit: revenue-coverage, debt-service-cover, fund-balance ratios, peer comparison.", wasm.municipal_analysis);
  tool(server, "spread_analysis", "Bid-ask spread analysis: effective vs quoted spread, price impact decomposition, MiFID II RTS-27 quality.", wasm.spread_analysis);
  tool(server, "optimal_execution", "Optimal execution: Almgren-Chriss closed-form, market-impact + risk trade-off, schedule output.", wasm.optimal_execution);
  tool(server, "property_valuation", "Property valuation: DCF with NOI projections, cap-rate, sales-comparison, residual-land value, sensitivity.", wasm.property_valuation);
  tool(server, "project_finance_model", "Project finance: revenue/opex/capex projections, sculpted debt, DSCR, sponsor IRR, breakeven analysis.", wasm.project_finance_model);
  tool(server, "us_fund_structure", "US fund structures: Delaware LP, REIT, RIC, BDC structure analysis with tax-adjusted returns.", wasm.us_fund_structure);
  tool(server, "uk_eu_fund_structure", "UK/EU fund structures: UCITS, AIF (RAIF/SIF/SCSp), ICAV, ETC across compliance and tax dimensions.", wasm.uk_eu_fund_structure);

  // Wave 17b — partial residuals + previously-blocked
  tool(server, "merger_model", "M&A merger model: EPS accretion/dilution, all-cash/all-stock/mixed consideration, premium analysis, exchange ratios, breakeven synergies.", wasm.merger_model);
  tool(server, "three_statement_model", "Three-statement model: linked income statement, balance sheet, cash flow with circular-reference resolution for interest expense.", wasm.three_statement_model);
  tool(server, "monte_carlo_simulation", "Monte Carlo simulation: parametric distributions, deterministic-seed mode, percentile output, scenario sensitivity.", wasm.monte_carlo_simulation);
  tool(server, "monte_carlo_dcf", "Monte Carlo DCF: stochastic WACC + growth + terminal-multiple inputs; percentile EV/equity output, sensitivity decomposition.", wasm.monte_carlo_dcf);
  tool(server, "scenario_analysis", "Scenario analysis: probability-weighted scenarios with output values + base case; expected value, deviations, dispersion.", wasm.scenario_analysis);

  const transport = new StdioServerTransport();
  await server.connect(transport);
  console.error(`cfa-core MCP server v${wasm.version()} listening on stdio`);
}

main().catch((err) => {
  console.error("cfa-core MCP fatal error:", err);
  process.exit(1);
});
