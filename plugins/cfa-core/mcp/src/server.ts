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

  const transport = new StdioServerTransport();
  await server.connect(transport);
  console.error(`cfa-core MCP server v${wasm.version()} listening on stdio`);
}

main().catch((err) => {
  console.error("cfa-core MCP fatal error:", err);
  process.exit(1);
});
