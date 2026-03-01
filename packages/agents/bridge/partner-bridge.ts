// Partner MCP Bridge — connects agents to the unified partner-mcp-server
// Provides access to LSEG, S&P Global, FactSet, Morningstar, Moody's, PitchBook tools

import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { StdioClientTransport } from '@modelcontextprotocol/sdk/client/stdio.js';

export interface PartnerBridgeConfig {
  /** Path to the partner MCP server entry point (default: packages/partner-mcp-server/dist/index.js) */
  serverPath?: string;
  /** Command to launch the server (default: 'node') */
  command?: string;
}

export class PartnerBridge {
  private client: Client;
  private transport: StdioClientTransport | null = null;
  private connected = false;

  constructor() {
    this.client = new Client(
      { name: 'cfa-agents-partner', version: '1.0.0' },
      { capabilities: {} },
    );
  }

  async connect(config: PartnerBridgeConfig = {}): Promise<void> {
    if (this.connected) return;

    const serverPath = config.serverPath ?? new URL('../../partner-mcp-server/dist/index.js', import.meta.url).pathname;
    const command = config.command ?? 'node';

    this.transport = new StdioClientTransport({
      command,
      args: [serverPath],
    });

    await this.client.connect(this.transport);
    this.connected = true;
  }

  async disconnect(): Promise<void> {
    if (!this.connected) return;
    await this.client.close();
    this.connected = false;
  }

  /** Call a partner tool by name */
  async callTool(toolName: string, params: Record<string, unknown>): Promise<unknown> {
    if (!this.connected) throw new Error('Partner bridge not connected');

    const result = await this.client.callTool({ name: toolName, arguments: params });

    // MCP tool results come as content array; extract the text
    if (result.content && Array.isArray(result.content)) {
      const textContent = result.content.find((c: { type: string }) => c.type === 'text');
      if (textContent && 'text' in textContent) {
        try {
          return JSON.parse(textContent.text as string);
        } catch {
          return textContent.text;
        }
      }
    }

    return result;
  }

  get isConnected(): boolean {
    return this.connected;
  }
}

/**
 * Create a callPartnerTool function for agent contexts.
 */
export async function createPartnerToolCaller(config?: PartnerBridgeConfig): Promise<{
  callPartnerTool: (toolName: string, params: Record<string, unknown>) => Promise<unknown>;
  bridge: PartnerBridge;
}> {
  const bridge = new PartnerBridge();
  await bridge.connect(config);
  return {
    callPartnerTool: (name, params) => bridge.callTool(name, params),
    bridge,
  };
}
