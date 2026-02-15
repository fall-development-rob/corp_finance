// Company name resolution via semantic similarity
// Uses agentic-flow's local embedding model (all-MiniLM-L6-v2, 384-dim)
// No API calls — runs entirely locally in <10ms after warmup

import {
  computeEmbedding,
  computeEmbeddingBatch,
} from 'agentic-flow/embeddings';

export interface CompanyMatch {
  name: string;
  ticker: string;
  similarity: number;
}

// Top companies by market cap — covers most queries
const COMPANIES: Array<[string, string]> = [
  ['Apple', 'AAPL'], ['Microsoft', 'MSFT'], ['Google', 'GOOGL'],
  ['Alphabet', 'GOOGL'], ['Amazon', 'AMZN'], ['Nvidia', 'NVDA'],
  ['Meta', 'META'], ['Facebook', 'META'], ['Tesla', 'TSLA'],
  ['Berkshire Hathaway', 'BRK.B'], ['JPMorgan', 'JPM'],
  ['Johnson & Johnson', 'JNJ'], ['Visa', 'V'], ['Walmart', 'WMT'],
  ['Procter & Gamble', 'PG'], ['Mastercard', 'MA'],
  ['UnitedHealth', 'UNH'], ['Home Depot', 'HD'],
  ['Chevron', 'CVX'], ['ExxonMobil', 'XOM'], ['Pfizer', 'PFE'],
  ['Coca-Cola', 'KO'], ['PepsiCo', 'PEP'], ['Costco', 'COST'],
  ['Disney', 'DIS'], ['Walt Disney', 'DIS'],
  ['Netflix', 'NFLX'], ['Adobe', 'ADBE'], ['Salesforce', 'CRM'],
  ['Intel', 'INTC'], ['AMD', 'AMD'], ['Cisco', 'CSCO'],
  ['Oracle', 'ORCL'], ['IBM', 'IBM'], ['Qualcomm', 'QCOM'],
  ['Goldman Sachs', 'GS'], ['Morgan Stanley', 'MS'],
  ['Bank of America', 'BAC'], ['Wells Fargo', 'WFC'],
  ['Citigroup', 'C'], ['BlackRock', 'BLK'],
  ['Boeing', 'BA'], ['Lockheed Martin', 'LMT'],
  ['Caterpillar', 'CAT'], ['3M', 'MMM'], ['General Electric', 'GE'],
  ['Ford', 'F'], ['General Motors', 'GM'], ['Toyota', 'TM'],
  ['Nike', 'NKE'], ['Starbucks', 'SBUX'], ['McDonald\'s', 'MCD'],
  ['AT&T', 'T'], ['Verizon', 'VZ'], ['T-Mobile', 'TMUS'],
  ['PayPal', 'PYPL'], ['Uber', 'UBER'], ['Airbnb', 'ABNB'],
  ['Spotify', 'SPOT'], ['Snap', 'SNAP'], ['Pinterest', 'PINS'],
  ['Palantir', 'PLTR'], ['Snowflake', 'SNOW'], ['Shopify', 'SHOP'],
  ['CrowdStrike', 'CRWD'], ['Datadog', 'DDOG'], ['Twilio', 'TWLO'],
  ['Moderna', 'MRNA'], ['Eli Lilly', 'LLY'], ['AbbVie', 'ABBV'],
  ['Merck', 'MRK'], ['Bristol-Myers Squibb', 'BMY'],
  ['Broadcom', 'AVGO'], ['Texas Instruments', 'TXN'],
  ['Applied Materials', 'AMAT'], ['Lam Research', 'LRCX'],
  ['ASML', 'ASML'], ['Taiwan Semiconductor', 'TSM'],
  // UK / International
  ['Shell', 'SHEL'], ['BP', 'BP'], ['HSBC', 'HSBC'],
  ['Unilever', 'UL'], ['AstraZeneca', 'AZN'], ['Rio Tinto', 'RIO'],
  ['BHP', 'BHP'], ['SAP', 'SAP'], ['Siemens', 'SIEGY'],
  ['Nestlé', 'NSRGY'], ['Roche', 'RHHBY'], ['Novartis', 'NVS'],
  ['LVMH', 'LVMUY'], ['Samsung', 'SSNLF'], ['Sony', 'SONY'],
  ['Alibaba', 'BABA'], ['Tencent', 'TCEHY'],
];

let corpusEmbeddings: Float32Array[] | null = null;
let warmupPromise: Promise<void> | null = null;

/**
 * Pre-compute embeddings for company corpus.
 * Called lazily on first resolve — subsequent calls are instant.
 */
async function ensureCorpus(): Promise<Float32Array[]> {
  if (corpusEmbeddings) return corpusEmbeddings;

  if (!warmupPromise) {
    warmupPromise = computeEmbeddingBatch(COMPANIES.map(([name]) => name))
      .then((embeddings) => { corpusEmbeddings = embeddings; });
  }
  await warmupPromise;
  return corpusEmbeddings!;
}

function cosineSim(a: Float32Array, b: Float32Array): number {
  let dot = 0, normA = 0, normB = 0;
  for (let i = 0; i < a.length; i++) {
    dot += a[i] * b[i];
    normA += a[i] * a[i];
    normB += b[i] * b[i];
  }
  const denom = Math.sqrt(normA) * Math.sqrt(normB);
  return denom === 0 ? 0 : dot / denom;
}

/**
 * Resolve a company name from a query using semantic similarity.
 * Returns the best match above the threshold, or null.
 *
 * Runs locally via agentic-flow's ONNX embedding model.
 * First call ~200ms (model load), subsequent calls <10ms.
 */
export async function resolveCompany(
  query: string,
  threshold = 0.45,
): Promise<CompanyMatch | null> {
  try {
    const corpus = await ensureCorpus();
    const queryEmbedding = await computeEmbedding(query);

    let bestIdx = -1;
    let bestSim = -1;

    for (let i = 0; i < corpus.length; i++) {
      const sim = cosineSim(queryEmbedding, corpus[i]);
      if (sim > bestSim) {
        bestSim = sim;
        bestIdx = i;
      }
    }

    if (bestIdx < 0 || bestSim < threshold) return null;

    const [name, ticker] = COMPANIES[bestIdx];
    return { name, ticker, similarity: bestSim };
  } catch {
    return null;
  }
}

/** Expose corpus for testing */
export function getCorpusSize(): number {
  return COMPANIES.length;
}
