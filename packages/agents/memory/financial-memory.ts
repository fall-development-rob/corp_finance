// Financial Memory — persistent analysis storage
// Interface for RuVector integration with in-memory fallback

import { randomUUID } from 'node:crypto';
import type { MemoryEntry, MemoryMetadata, RetrievalContext, RetentionTier } from '../types/memory.js';

export interface FinancialMemory {
  store(content: string, metadata: MemoryMetadata): Promise<MemoryEntry>;
  search(query: string, limit?: number): Promise<RetrievalContext>;
  retrieve(entryId: string): Promise<MemoryEntry | null>;
  getByTicker(ticker: string, limit?: number): Promise<MemoryEntry[]>;
}

// In-memory implementation — swap for RuVector in production
export class LocalFinancialMemory implements FinancialMemory {
  private entries = new Map<string, MemoryEntry>();

  async store(content: string, metadata: MemoryMetadata): Promise<MemoryEntry> {
    const entry: MemoryEntry = {
      entryId: randomUUID(),
      archiveId: 'default',
      content,
      metadata,
      embeddingModel: 'local-placeholder',
      retentionTier: 'hot',
      createdAt: new Date(),
      lastAccessedAt: new Date(),
      accessCount: 0,
    };

    this.entries.set(entry.entryId, entry);
    return entry;
  }

  async search(query: string, limit = 10): Promise<RetrievalContext> {
    // Simple keyword matching — RuVector provides real vector similarity
    const q = query.toLowerCase();
    const scored = [...this.entries.values()]
      .map(entry => {
        const content = entry.content.toLowerCase();
        const words = q.split(/\s+/);
        const matchCount = words.filter(w => content.includes(w)).length;
        return { entry, similarityScore: matchCount / Math.max(words.length, 1) };
      })
      .filter(r => r.similarityScore > 0)
      .sort((a, b) => b.similarityScore - a.similarityScore)
      .slice(0, limit);

    // Update access stats
    for (const { entry } of scored) {
      entry.lastAccessedAt = new Date();
      entry.accessCount++;
    }

    return {
      entries: scored,
      query,
      totalSearched: this.entries.size,
    };
  }

  async retrieve(entryId: string): Promise<MemoryEntry | null> {
    const entry = this.entries.get(entryId);
    if (entry) {
      entry.lastAccessedAt = new Date();
      entry.accessCount++;
    }
    return entry ?? null;
  }

  async getByTicker(ticker: string, limit = 10): Promise<MemoryEntry[]> {
    return [...this.entries.values()]
      .filter(e => e.metadata.tickers?.includes(ticker))
      .sort((a, b) => b.createdAt.getTime() - a.createdAt.getTime())
      .slice(0, limit);
  }
}
