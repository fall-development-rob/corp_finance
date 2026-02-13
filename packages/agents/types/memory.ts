// BC3: Financial Memory - AnalysisArchive aggregate
// RuVector-backed semantic storage for past analyses

export type SourceType = 'analysis' | 'filing' | 'market_data' | 'earnings_transcript' | 'methodology';
export type RetentionTier = 'hot' | 'warm' | 'cold';

export interface MemoryMetadata {
  readonly sourceType: SourceType;
  readonly sector?: string;
  readonly tickers?: string[];
  readonly dateRange?: { start: Date; end: Date };
  readonly analysisType?: string;
  readonly tags: string[];
}

export interface MemoryEntry {
  entryId: string;
  archiveId: string;
  content: string;
  metadata: MemoryMetadata;
  embeddingModel: string;
  retentionTier: RetentionTier;
  createdAt: Date;
  lastAccessedAt: Date;
  accessCount: number;
}

export interface RetrievalContext {
  readonly entries: Array<{
    entry: MemoryEntry;
    similarityScore: number;
  }>;
  readonly query: string;
  readonly totalSearched: number;
}

export interface EmbeddingIndex {
  readonly indexId: string;
  readonly model: string;
  readonly dimension: number;
  readonly distanceMetric: 'cosine' | 'euclidean' | 'dot_product';
  readonly entryCount: number;
  readonly lastRebuilt: Date;
}
