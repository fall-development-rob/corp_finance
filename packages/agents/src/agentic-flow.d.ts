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
