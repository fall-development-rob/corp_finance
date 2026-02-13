// Type declarations for agentic-flow deep imports (not in package exports map)

declare module 'agentic-flow/dist/agents/claudeAgent.js' {
  import type { AgentDefinition } from 'agentic-flow/dist/utils/agentLoader.js';

  export function claudeAgent(
    agent: AgentDefinition,
    input: string,
    onStream?: (chunk: string) => void,
    modelOverride?: string,
  ): Promise<{ output: string; agent: string }>;
}

declare module 'agentic-flow/dist/utils/agentLoader.js' {
  export interface AgentDefinition {
    name: string;
    description: string;
    systemPrompt: string;
    color?: string;
    tools?: string[];
    filePath: string;
  }

  export function loadAgents(agentsDir?: string): Map<string, AgentDefinition>;
  export function getAgent(name: string, agentsDir?: string): AgentDefinition | undefined;
  export function listAgents(agentsDir?: string): AgentDefinition[];
}

declare module 'agentic-flow/dist/routing/SemanticRouter.js' {
  import type { EmbeddingService } from 'agentic-flow/dist/core/embedding-service.js';

  export interface AgentIntent {
    agentType: string;
    description: string;
    examples: string[];
    tags: string[];
  }

  export interface RoutingResult {
    primaryAgent: string;
    confidence: number;
    alternatives: Array<{ agentType: string; confidence: number }>;
    matchedIntents: string[];
    metrics: {
      routingTimeMs: number;
      embeddingTimeMs: number;
      searchTimeMs: number;
      candidatesEvaluated: number;
    };
  }

  export interface MultiIntentResult {
    intents: Array<{ agentType: string; confidence: number; matchedText: string }>;
    requiresMultiAgent: boolean;
    executionOrder: string[];
  }

  export class SemanticRouter {
    constructor(embedder: EmbeddingService);
    registerAgent(intent: AgentIntent): Promise<void>;
    registerAgents(intents: AgentIntent[]): Promise<void>;
    buildIndex(): void;
    route(taskDescription: string, k?: number): Promise<RoutingResult>;
    detectMultiIntent(taskDescription: string, threshold?: number): Promise<MultiIntentResult>;
    getStats(): { totalRoutes: number; avgLatency: number; avgConfidence: number };
    getRegisteredAgents(): AgentIntent[];
  }
}

declare module 'agentic-flow/dist/coordination/attention-coordinator.js' {
  export interface AgentOutput {
    agentId: string;
    agentType: string;
    embedding: Float32Array;
    value: any;
    confidence?: number;
    metadata?: Record<string, any>;
  }

  export type SwarmTopology = 'mesh' | 'hierarchical' | 'ring' | 'star';

  export interface CoordinationResult {
    consensus: any;
    attentionWeights: number[];
    mechanism: string;
    executionTimeMs: number;
    topAgents: string[];
  }

  export class AttentionCoordinator {
    constructor(attentionService: any);
    coordinateAgents(agentOutputs: AgentOutput[], mechanism?: string): Promise<CoordinationResult>;
    topologyAwareCoordination(agentOutputs: AgentOutput[], topology: SwarmTopology, graphStructure?: any): Promise<CoordinationResult>;
    hierarchicalCoordination(queenOutputs: AgentOutput[], workerOutputs: AgentOutput[], curvature?: number): Promise<CoordinationResult>;
  }

  export function createAttentionCoordinator(attentionService: any): AttentionCoordinator;
}

declare module 'agentic-flow/dist/core/embedding-service.js' {
  import { EventEmitter } from 'events';

  export interface EmbeddingConfig {
    provider: 'openai' | 'transformers' | 'onnx' | 'mock';
    model?: string;
    dimensions?: number;
    apiKey?: string;
    cacheSize?: number;
  }

  export interface EmbeddingResult {
    embedding: number[];
    usage?: { promptTokens: number; totalTokens: number };
    latency: number;
  }

  export abstract class EmbeddingService extends EventEmitter {
    protected config: EmbeddingConfig;
    constructor(config: EmbeddingConfig);
    abstract embed(text: string): Promise<EmbeddingResult>;
    abstract embedBatch(texts: string[]): Promise<EmbeddingResult[]>;
    clearCache(): void;
  }

  export class TransformersEmbeddingService extends EmbeddingService {
    constructor(config: Omit<EmbeddingConfig, 'provider'>);
    initialize(): Promise<void>;
    embed(text: string): Promise<EmbeddingResult>;
    embedBatch(texts: string[]): Promise<EmbeddingResult[]>;
  }

  export class MockEmbeddingService extends EmbeddingService {
    constructor(config?: Partial<EmbeddingConfig>);
    embed(text: string): Promise<EmbeddingResult>;
    embedBatch(texts: string[]): Promise<EmbeddingResult[]>;
  }

  export function createEmbeddingService(config: EmbeddingConfig): EmbeddingService;
}

declare module 'agentic-flow/dist/core/attention-fallbacks.js' {
  export interface AttentionConfig {
    hiddenDim: number;
    numHeads?: number;
    dropoutRate?: number;
    useFlash?: boolean;
  }

  export class FlashAttention {
    constructor(config: AttentionConfig);
    forward(query: number[][], key: number[][], value: number[][], numHeads?: number): {
      output: number[][];
      attentionScores: number[][];
    };
  }

  export class MultiHeadAttention {
    constructor(config: AttentionConfig);
    forward(query: number[], key: number[], value: number[], mask?: number[]): {
      output: number[];
      attentionWeights: number[][];
    };
  }

  export function createAttention(
    type: 'multi-head' | 'flash' | 'linear' | 'hyperbolic' | 'moe',
    config: AttentionConfig,
  ): any;

  export function isNativeAttentionAvailable(): boolean;
}
