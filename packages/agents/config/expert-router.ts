// MoE Expert Router — semantic specialist selection via @ruvector/router
// Replaces static keyword matching with embedding-based intent classification
// Uses agentic-flow's local ONNX model (all-MiniLM-L6-v2, 384-dim)
// No API calls — runs entirely locally in <10ms after warmup

import { AGENT_DESCRIPTIONS, TOOL_MAPPINGS, DOMAIN_PATTERNS } from './tool-mappings.js';

export interface RoutingResult {
  agentType: string;
  score: number;
}

export interface ExpertRouterConfig {
  /** Cosine similarity threshold. Default: 0.35 */
  threshold?: number;
  /** Maximum agents to return. Default: 6 */
  maxResults?: number;
}

export class ExpertRouter {
  private router: any = null; // SemanticRouter (typed as any to avoid import-time dep)
  private initPromise: Promise<void> | null = null;
  private initFailed = false;
  private config: Required<ExpertRouterConfig>;

  constructor(config?: ExpertRouterConfig) {
    this.config = {
      threshold: config?.threshold ?? 0.35,
      maxResults: config?.maxResults ?? 6,
    };
  }

  private async initialize(): Promise<void> {
    if (this.router) return;
    if (this.initFailed) return;
    if (this.initPromise) { await this.initPromise; return; }

    this.initPromise = (async () => {
      try {
        // Force ONNX transformer model — bypasses NPX environment detection
        // that otherwise falls back to non-semantic hash-based embeddings
        if (!process.env.FORCE_TRANSFORMERS) {
          process.env.FORCE_TRANSFORMERS = '1';
        }

        const ruvector = await import('@ruvector/router');
        const SemanticRouter = (ruvector as any).default?.SemanticRouter ?? (ruvector as any).SemanticRouter;
        const { computeEmbedding } = await import('agentic-flow/embeddings');

        const sr = new SemanticRouter({
          dimension: 384,
          metric: 'cosine',
          threshold: this.config.threshold,
        });

        sr.setEmbedder(computeEmbedding);

        // Register each specialist agent as an intent
        for (const [agentType, description] of Object.entries(AGENT_DESCRIPTIONS)) {
          const toolDomains = TOOL_MAPPINGS[agentType] ?? [];

          // Build utterances: description + domain keywords
          const utterances = [description];
          for (const domain of toolDomains) {
            const keywords = DOMAIN_PATTERNS[domain];
            if (keywords) {
              // Join short keywords into meaningful phrases
              utterances.push(keywords.join(' '));
            }
          }

          await sr.addIntentAsync({
            name: agentType,
            utterances,
            metadata: { toolDomains, description },
          });
        }

        this.router = sr;
      } catch {
        this.initFailed = true;
      }
    })();

    await this.initPromise;
  }

  /**
   * Route a user query to top-k matching specialist agents.
   * Returns empty array if semantic routing unavailable.
   */
  async route(query: string, k?: number): Promise<RoutingResult[]> {
    await this.initialize();
    if (!this.router) return [];

    try {
      const maxK = k ?? this.config.maxResults;
      const results = await this.router.route(query, maxK);

      // SemanticRouter already filters by constructor threshold;
      // map to our RoutingResult interface
      return results.map((r: any) => ({
        agentType: r.intent as string,
        score: r.score as number,
      }));
    } catch {
      return [];
    }
  }

  /**
   * Route a plan step description to the single best agent.
   * Returns null if semantic routing unavailable.
   */
  async routeStep(description: string): Promise<RoutingResult | null> {
    const results = await this.route(description, 1);
    return results[0] ?? null;
  }

  /** True after successful initialization */
  get isAvailable(): boolean {
    return this.router !== null;
  }

  /** Force re-initialization */
  reset(): void {
    this.router = null;
    this.initPromise = null;
    this.initFailed = false;
  }
}
