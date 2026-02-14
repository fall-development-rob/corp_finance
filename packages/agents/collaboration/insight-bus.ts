// ADR-006: Cross-Specialist InsightBus
// Allows specialist agents to broadcast findings mid-execution
// so peer agents can incorporate them in subsequent reasoning iterations.

import { randomUUID } from 'node:crypto';
import type { AgentType } from '../types/agents.js';
import type { AgentInsight, InsightFilter, InsightType } from '../types/collaboration.js';

export type InsightHandler = (insight: AgentInsight) => void;

export class InsightBus {
  private insights: AgentInsight[] = [];
  private subscribers = new Map<string, InsightHandler>();

  /**
   * Broadcast a finding to all subscribed agents.
   */
  broadcast(insight: Omit<AgentInsight, 'id' | 'timestamp'>): AgentInsight {
    const full: AgentInsight = {
      ...insight,
      id: randomUUID(),
      timestamp: new Date(),
    };
    this.insights.push(full);

    for (const [subId, handler] of this.subscribers) {
      // Don't send insights back to the source agent
      if (subId === insight.sourceAgentId) continue;
      try {
        handler(full);
      } catch {
        // Best-effort delivery — don't crash on handler errors
      }
    }

    return full;
  }

  /**
   * Subscribe an agent to receive peer insights.
   */
  subscribe(agentId: string, handler: InsightHandler): void {
    this.subscribers.set(agentId, handler);
  }

  /**
   * Unsubscribe an agent.
   */
  unsubscribe(agentId: string): void {
    this.subscribers.delete(agentId);
  }

  /**
   * Query accumulated insights with optional filtering.
   */
  getInsights(filter?: InsightFilter): AgentInsight[] {
    let results = [...this.insights];

    if (filter?.sourceAgent) {
      results = results.filter(i => i.sourceAgent === filter.sourceAgent);
    }
    if (filter?.insightType) {
      results = results.filter(i => i.insightType === filter.insightType);
    }
    if (filter?.minConfidence !== undefined) {
      results = results.filter(i => i.confidence >= filter.minConfidence!);
    }
    if (filter?.since) {
      results = results.filter(i => i.timestamp >= filter.since!);
    }

    return results;
  }

  /**
   * Get insights from agents other than the specified one.
   * Used by agents to check what peers have discovered.
   */
  getPeerInsights(excludeAgentId: string, minConfidence = 0.5): AgentInsight[] {
    return this.insights.filter(
      i => i.sourceAgentId !== excludeAgentId && i.confidence >= minConfidence,
    );
  }

  /**
   * Format peer insights as context text for injection into reasoning.
   */
  formatPeerContext(excludeAgentId: string, minConfidence = 0.5): string {
    const peers = this.getPeerInsights(excludeAgentId, minConfidence);
    if (peers.length === 0) return '';

    return peers
      .map(i => `[${i.sourceAgent}] (${i.insightType}, confidence: ${i.confidence.toFixed(2)}): ${i.content}`)
      .join('\n');
  }

  /**
   * Get total insight count.
   */
  get size(): number {
    return this.insights.length;
  }

  /**
   * Get subscriber count.
   */
  get subscriberCount(): number {
    return this.subscribers.size;
  }

  /**
   * Clear all insights (for testing or between requests).
   */
  clear(): void {
    this.insights = [];
  }
}
