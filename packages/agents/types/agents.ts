// BC2: Specialist Analysts - AnalystAgent aggregate
// Domain-specific agents with curated MCP tool subsets

export type AgentType =
  | 'chief-analyst'
  | 'equity-analyst'
  | 'credit-analyst'
  | 'fixed-income-analyst'
  | 'derivatives-analyst'
  | 'quant-risk-analyst'
  | 'macro-analyst'
  | 'esg-regulatory-analyst'
  | 'private-markets-analyst';

export interface AgentCapability {
  readonly agentType: AgentType;
  readonly toolDomains: string[];     // MCP tool module names this agent can use
  readonly analysisTypes: string[];   // types of analysis this agent can perform
  readonly description: string;
}

export interface ToolInvocation {
  invocationId: string;
  agentId: string;
  toolName: string;
  params: Record<string, unknown>;
  result?: unknown;
  error?: string;
  duration?: number;     // ms
  timestamp: Date;
}

export interface Finding {
  readonly statement: string;
  readonly supportingData: Record<string, unknown>;
  readonly confidence: number;   // 0-1
  readonly methodology: string;
  readonly citations: Citation[];
}

export interface Citation {
  readonly invocationId: string;
  readonly toolName: string;
  readonly relevantOutput: string;
}

export interface AnalysisResult {
  resultId: string;
  agentId: string;
  agentType: AgentType;
  assignmentId: string;
  findings: Finding[];
  summary: string;
  confidence: number;
  toolInvocations: ToolInvocation[];
  completedAt: Date;
}
