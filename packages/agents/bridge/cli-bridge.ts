// CLI Bridge — calls the `cfa` Rust CLI binary for tool execution
// Alternative to the MCP client bridge, useful for local/offline use

import { execFile } from 'node:child_process';
import { promisify } from 'node:util';

const execFileAsync = promisify(execFile);

export interface CliBridgeConfig {
  /** Path to the cfa binary (default: searches PATH) */
  binaryPath?: string;
  /** Output format: json (default), yaml, csv, table */
  format?: 'json' | 'yaml' | 'csv' | 'table';
  /** Timeout in ms (default: 30000) */
  timeout?: number;
}

// Maps MCP tool names to CLI subcommand + argument structure
// The CLI uses `cfa <subcommand> --json '<params>'` pattern
const TOOL_TO_CLI: Record<string, string> = {
  // Valuation
  wacc_calculator: 'wacc',
  dcf_model: 'dcf',
  comps_analysis: 'comps',
  // Credit
  credit_metrics: 'credit',
  debt_capacity: 'debt-capacity',
  covenant_compliance: 'covenant',
  altman_zscore: 'altman',
  // PE / Returns
  returns_calculator: 'returns',
  debt_schedule: 'debt-schedule',
  sources_uses: 'sources-uses',
  lbo_model: 'lbo',
  waterfall_calculator: 'waterfall',
  // M&A
  merger_model: 'merger',
  // Portfolio
  risk_adjusted_returns: 'risk-adjusted-returns',
  risk_metrics: 'risk-metrics',
  kelly_sizing: 'kelly',
  sensitivity_matrix: 'sensitivity',
  // Fund
  fund_fee_calculator: 'fund-fees',
  gaap_ifrs_reconciliation: 'gaap-ifrs',
  withholding_tax_calculator: 'wht',
  portfolio_wht_calculator: 'portfolio-wht',
  nav_calculator: 'nav',
  gp_economics_model: 'gp-economics',
  investor_net_returns: 'investor-returns',
  ubti_eci_screening: 'ubti',
  // Financial modelling
  three_statement_model: 'three-statement',
  // Monte Carlo
  monte_carlo_simulation: 'monte-carlo',
  monte_carlo_dcf: 'mc-dcf',
  // Scenario
  scenario_analysis: 'scenario',
  // Fixed income
  bond_pricing: 'bond-price',
  bond_yield: 'bond-yield',
  bootstrap_curve: 'bootstrap',
  nelson_siegel: 'nelson-siegel',
  duration_calculator: 'duration',
  credit_spread: 'credit-spread',
  // Derivatives
  option_pricing: 'option-price',
  implied_vol: 'implied-vol',
  forward_price: 'forward-price',
  irs_valuation: 'irs',
  strategy_analysis: 'strategy',
  // Quant risk
  factor_model: 'factor-model',
  black_litterman: 'black-litterman',
  risk_parity: 'risk-parity',
  stress_test: 'stress-test',
  // ESG
  esg_score: 'esg-score',
  carbon_footprint: 'carbon-footprint',
  green_bond: 'green-bond',
  // Regulatory
  regulatory_capital: 'reg-capital',
  lcr_calculator: 'lcr',
  nsfr_calculator: 'nsfr',
  // Restructuring
  recovery_analysis: 'recovery',
  distressed_debt: 'distressed',
  // Earnings quality
  beneish_mscore: 'beneish',
  piotroski_fscore: 'piotroski',
  accrual_quality: 'accrual-quality',
  revenue_quality: 'revenue-quality',
  earnings_quality_composite: 'earnings-composite',
  // Dividend policy
  h_model_ddm: 'h-model-ddm',
  multistage_ddm: 'multistage-ddm',
  buyback_analysis: 'buyback',
  payout_sustainability: 'payout',
  total_shareholder_return: 'tsr',
  // Financial forensics
  benfords_law: 'benfords',
  dupont_analysis: 'dupont',
  zscore_models: 'zscore-models',
  peer_benchmarking: 'peer-benchmark',
  red_flag_scoring: 'red-flags',
};

export class CliBridge {
  private config: Required<CliBridgeConfig>;

  constructor(config: CliBridgeConfig = {}) {
    this.config = {
      binaryPath: config.binaryPath ?? 'cfa',
      format: config.format ?? 'json',
      timeout: config.timeout ?? 30000,
    };
  }

  async callTool(toolName: string, params: Record<string, unknown>): Promise<unknown> {
    const subcommand = TOOL_TO_CLI[toolName] ?? toolName.replace(/_/g, '-');
    const jsonInput = JSON.stringify(params);

    try {
      const { stdout } = await execFileAsync(
        this.config.binaryPath,
        [subcommand, '--format', this.config.format, '--json', jsonInput],
        { timeout: this.config.timeout },
      );

      try {
        return JSON.parse(stdout);
      } catch {
        return stdout.trim();
      }
    } catch (err: unknown) {
      const error = err as { stderr?: string; message?: string };
      throw new Error(`CLI tool ${subcommand} failed: ${error.stderr ?? error.message}`);
    }
  }
}

export function createCliToolCaller(config?: CliBridgeConfig): {
  callTool: (toolName: string, params: Record<string, unknown>) => Promise<unknown>;
  bridge: CliBridge;
} {
  const bridge = new CliBridge(config);
  return {
    callTool: (name, params) => bridge.callTool(name, params),
    bridge,
  };
}
