// Extract financial metrics from natural language task descriptions
// Handles patterns like "revenue $394B", "beta 1.2", "EBITDA margin 22%"

export interface ExtractedMetrics {
  // Income statement
  revenue?: number;
  ebitda?: number;
  ebit?: number;
  net_income?: number;
  cogs?: number;
  sga?: number;

  // Margins
  ebitda_margin?: number;
  ebit_margin?: number;
  net_margin?: number;
  gross_margin?: number;

  // Balance sheet
  total_assets?: number;
  total_equity?: number;
  total_debt?: number;
  net_debt?: number;
  cash?: number;
  current_assets?: number;
  current_liabilities?: number;
  ppe?: number;
  receivables?: number;
  inventory?: number;
  payables?: number;

  // Per share
  shares_outstanding?: number;
  share_price?: number;
  eps?: number;
  book_value_per_share?: number;
  dividend_per_share?: number;

  // Market
  market_cap?: number;
  enterprise_value?: number;

  // Rates & ratios
  beta?: number;
  risk_free_rate?: number;
  cost_of_equity?: number;
  cost_of_debt?: number;
  tax_rate?: number;
  wacc?: number;
  growth_rate?: number;
  terminal_growth?: number;

  // Credit
  current_ratio?: number;
  debt_to_equity?: number;
  interest_coverage?: number;
  interest_expense?: number;
  capex?: number;
  operating_cash_flow?: number;
  depreciation?: number;

  // FI
  coupon_rate?: number;
  ytm?: number;
  face_value?: number;
  maturity_years?: number;
  yield?: number;

  // Other
  volatility?: number;
  recovery_rate?: number;
  default_probability?: number;

  // Data source tracking
  _symbol?: string;
  _sector?: string;
  _industry?: string;
  _dataSource?: 'text-only' | 'fmp-enriched';

  // Raw text for context
  _raw: string;
  _company?: string;
}

const MULTIPLIERS: Record<string, number> = {
  T: 1e12, t: 1e12, trillion: 1e12,
  B: 1e9, b: 1e9, bn: 1e9, billion: 1e9,
  M: 1e6, m: 1e6, mm: 1e6, mn: 1e6, million: 1e6,
  K: 1e3, k: 1e3, thousand: 1e3,
};

function parseAmount(numStr: string, suffix?: string): number {
  const num = parseFloat(numStr.replace(/,/g, ''));
  if (isNaN(num)) return NaN;
  if (suffix) {
    const mult = MULTIPLIERS[suffix];
    if (mult) return num * mult;
  }
  return num;
}

// Match "$394B", "$50bn", "394 billion", "15.5B", "$1.2M"
const AMOUNT_RE = /\$?\s*([\d,.]+)\s*(trillion|billion|million|thousand|[TBMKtbmk](?:n|m)?)\b/gi;

// Match "22%", "5.5%"
const PCT_RE = /([\d.]+)\s*%/g;

// Match "5x", "1.8x"
const MULT_RE = /([\d.]+)\s*x\b/gi;

type PatternDef = [RegExp, string];

const METRIC_PATTERNS: PatternDef[] = [
  // Income statement
  [/\brevenue\s+\$?\s*([\d,.]+)\s*(trillion|billion|million|thousand|[TBMKtbmk](?:n|m)?)\b/i, 'revenue'],
  [/\bebitda\s+\$?\s*([\d,.]+)\s*(trillion|billion|million|thousand|[TBMKtbmk](?:n|m)?)\b/i, 'ebitda'],
  [/\bebit\s+\$?\s*([\d,.]+)\s*(trillion|billion|million|thousand|[TBMKtbmk](?:n|m)?)\b/i, 'ebit'],
  [/\bnet\s+income\s+\$?\s*([\d,.]+)\s*(trillion|billion|million|thousand|[TBMKtbmk](?:n|m)?)\b/i, 'net_income'],
  [/\bcogs\s+\$?\s*([\d,.]+)\s*(trillion|billion|million|thousand|[TBMKtbmk](?:n|m)?)\b/i, 'cogs'],

  // Balance sheet
  [/\btotal\s+assets?\s+\$?\s*([\d,.]+)\s*(trillion|billion|million|thousand|[TBMKtbmk](?:n|m)?)\b/i, 'total_assets'],
  [/\btotal\s+equity\s+\$?\s*([\d,.]+)\s*(trillion|billion|million|thousand|[TBMKtbmk](?:n|m)?)\b/i, 'total_equity'],
  [/\btotal\s+debt\s+\$?\s*([\d,.]+)\s*(trillion|billion|million|thousand|[TBMKtbmk](?:n|m)?)\b/i, 'total_debt'],
  [/\bnet\s+debt\s+\$?\s*([\d,.]+)\s*(trillion|billion|million|thousand|[TBMKtbmk](?:n|m)?)\b/i, 'net_debt'],
  [/\bcash\s+\$?\s*([\d,.]+)\s*(trillion|billion|million|thousand|[TBMKtbmk](?:n|m)?)\b/i, 'cash'],
  [/\bmarket\s+cap\s+\$?\s*([\d,.]+)\s*(trillion|billion|million|thousand|[TBMKtbmk](?:n|m)?)\b/i, 'market_cap'],
  [/\benterprise\s+value\s+\$?\s*([\d,.]+)\s*(trillion|billion|million|thousand|[TBMKtbmk](?:n|m)?)\b/i, 'enterprise_value'],
  [/\bcapex\s+\$?\s*([\d,.]+)\s*(trillion|billion|million|thousand|[TBMKtbmk](?:n|m)?)\b/i, 'capex'],
  [/\binterest\s+expense\s+\$?\s*([\d,.]+)\s*(trillion|billion|million|thousand|[TBMKtbmk](?:n|m)?)\b/i, 'interest_expense'],
  [/\boperating\s+cash\s+flow\s+\$?\s*([\d,.]+)\s*(trillion|billion|million|thousand|[TBMKtbmk](?:n|m)?)\b/i, 'operating_cash_flow'],
  [/\bdepreciation\s+\$?\s*([\d,.]+)\s*(trillion|billion|million|thousand|[TBMKtbmk](?:n|m)?)\b/i, 'depreciation'],

  // Per share
  [/\bshares?\s+outstanding\s+\$?\s*([\d,.]+)\s*(trillion|billion|million|thousand|[TBMKtbmk](?:n|m)?)\b/i, 'shares_outstanding'],
  [/\bshare\s+price\s+\$?\s*([\d,.]+)/i, 'share_price'],
  [/\beps\s+\$?\s*([\d,.]+)/i, 'eps'],

  // Ratios (no multiplier)
  [/\bbeta\s+([\d.]+)/i, 'beta'],
  [/\bcurrent\s+ratio\s+([\d.]+)/i, 'current_ratio'],
  [/\bdebt[- ]to[- ]equity\s+([\d.]+)/i, 'debt_to_equity'],
  [/\binterest\s+coverage\s+([\d.]+)/i, 'interest_coverage'],
  [/\bwacc\s+([\d.]+)/i, 'wacc'],
  [/\bcost\s+of\s+equity\s+([\d.]+)/i, 'cost_of_equity'],
  [/\bcost\s+of\s+debt\s+([\d.]+)/i, 'cost_of_debt'],
  [/\brisk[- ]free\s+rate\s+([\d.]+)/i, 'risk_free_rate'],
  [/\bvolatility\s+([\d.]+)/i, 'volatility'],
  [/\byield\s+([\d.]+)/i, 'yield'],
  [/\bcoupon\s+([\d.]+)/i, 'coupon_rate'],
  [/\bmaturity\s+([\d.]+)/i, 'maturity_years'],
  [/\brecovery\s+rate\s+([\d.]+)/i, 'recovery_rate'],
  [/\bgrowth\s+rate?\s+([\d.]+)/i, 'growth_rate'],

  // Percentage-based
  [/\bebitda\s+margin\s+([\d.]+)\s*%/i, 'ebitda_margin'],
  [/\bebit\s+margin\s+([\d.]+)\s*%/i, 'ebit_margin'],
  [/\bnet\s+margin\s+([\d.]+)\s*%/i, 'net_margin'],
  [/\bgross\s+margin\s+([\d.]+)\s*%/i, 'gross_margin'],
  [/\btax\s+rate\s+([\d.]+)\s*%/i, 'tax_rate'],
];

/**
 * Parse financial metrics from a natural-language task description.
 * Returns an ExtractedMetrics object with all values found.
 */
export function parseFinancialData(text: string): ExtractedMetrics {
  const metrics: ExtractedMetrics = { _raw: text };

  // Try to extract company name from multiple patterns
  let companyName: string | undefined;

  // Pattern 1: "Analyze Apple: ..." or "Assess Microsoft, ..." (verb + company + delimiter)
  const p1 = text.match(/^(?:analyze|assess|evaluate|value|review|compare)\s+([A-Z][A-Za-z\s.&']+?)(?:\s*[,:—-]|\s+(?:revenue|ebitda|with|has))/i);
  if (p1) companyName = p1[1].trim();

  // Pattern 2: "... for Apple" or "... for Apple Inc." (company after "for" at end)
  if (!companyName) {
    const p2 = text.match(/\bfor\s+([A-Z][A-Za-z.&']+(?:\s+(?:Inc|Corp|Ltd|LLC|Co|Group|Holdings|Technologies|Microsystems|Motors|Platforms|Entertainment)\.?)?)\s*$/i);
    if (p2) companyName = p2[1].trim();
  }

  // Pattern 3: "... for Apple's credit risk" (company after "for" mid-sentence)
  if (!companyName) {
    const p3 = text.match(/\bfor\s+([A-Z][A-Za-z.&']+(?:\s+(?:Inc|Corp|Ltd|LLC|Co)\.?)?)\s*(?:'s\b|\s+(?:credit|equity|risk|valuation|analysis|stock|bond|debt))/i);
    if (p3) companyName = p3[1].trim();
  }

  // Pattern 4: Known ticker symbol (1-5 uppercase letters that aren't common words)
  if (!companyName) {
    const EXCLUDED_TICKERS = new Set([
      'A', 'I', 'THE', 'FOR', 'AND', 'WITH', 'HAS', 'EPS', 'DCF', 'IRR',
      'WACC', 'EBITDA', 'EBIT', 'IPO', 'LBO', 'MBS', 'CLO', 'CDS', 'FMP',
      'VAR', 'CVAR', 'PE', 'PB', 'ROE', 'ROA', 'EV', 'ESG', 'AML', 'KYC',
      'FX', 'GDP', 'CPI', 'ETF', 'NAV', 'YTM', 'OAS', 'DPS', 'BPS',
    ]);
    const p4 = text.match(/\b([A-Z]{2,5})\b/g);
    if (p4) {
      const ticker = p4.find(t => !EXCLUDED_TICKERS.has(t));
      if (ticker) companyName = ticker;
    }
  }

  if (companyName) {
    metrics._company = companyName;
  }

  // Apply metric patterns
  for (const [re, key] of METRIC_PATTERNS) {
    const m = re.exec(text);
    if (!m) continue;

    const k = key as keyof ExtractedMetrics;
    if (metrics[k] !== undefined) continue; // don't overwrite

    if (m[2]) {
      // Has a multiplier suffix
      const val = parseAmount(m[1], m[2]);
      if (!isNaN(val)) (metrics as unknown as Record<string, unknown>)[k] = val;
    } else {
      // Plain number (ratio) or percentage
      const val = parseFloat(m[1]);
      if (!isNaN(val)) {
        // Convert percentages to decimals
        if (key.includes('margin') || key === 'tax_rate') {
          (metrics as unknown as Record<string, unknown>)[k] = val / 100;
        } else {
          (metrics as unknown as Record<string, unknown>)[k] = val;
        }
      }
    }
  }

  // Normalise rate fields: values > 1 are assumed to be percentages (e.g. 4 → 0.04)
  const RATE_KEYS: (keyof ExtractedMetrics)[] = [
    'risk_free_rate', 'cost_of_equity', 'cost_of_debt', 'wacc',
    'growth_rate', 'terminal_growth', 'ytm', 'yield', 'coupon_rate',
  ];
  for (const k of RATE_KEYS) {
    const v = metrics[k];
    if (typeof v === 'number' && v > 1) {
      (metrics as unknown as Record<string, unknown>)[k] = v / 100;
    }
  }

  // Derive common calculations
  if (metrics.revenue && metrics.ebitda && !metrics.ebitda_margin) {
    metrics.ebitda_margin = metrics.ebitda / metrics.revenue;
  }
  if (metrics.revenue && metrics.ebitda_margin && !metrics.ebitda) {
    metrics.ebitda = metrics.revenue * metrics.ebitda_margin;
  }
  if (metrics.revenue && metrics.net_income && !metrics.net_margin) {
    metrics.net_margin = metrics.net_income / metrics.revenue;
  }
  if (metrics.total_debt && metrics.cash && !metrics.net_debt) {
    metrics.net_debt = metrics.total_debt - metrics.cash;
  }
  if (metrics.ebitda && metrics.interest_expense && !metrics.interest_coverage) {
    metrics.interest_coverage = metrics.ebitda / metrics.interest_expense;
  }
  if (metrics.total_debt && metrics.total_equity && !metrics.debt_to_equity) {
    metrics.debt_to_equity = metrics.total_debt / metrics.total_equity;
  }

  return metrics;
}
