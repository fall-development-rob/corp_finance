// S&P Global MCP Bridge — connects agents to the sp-global-mcp-server for ratings & analytics
// Mirrors FredBridge but routes to the S&P Global server

import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { StdioClientTransport } from '@modelcontextprotocol/sdk/client/stdio.js';

export interface SpGlobalBridgeConfig {
  /** Path to the S&P Global MCP server entry point (default: packages/sp-global-mcp-server/dist/index.js) */
  serverPath?: string;
  /** Command to launch the server (default: 'node') */
  command?: string;
}

export class SpGlobalBridge {
  private client: Client;
  private transport: StdioClientTransport | null = null;
  private connected = false;

  constructor() {
    this.client = new Client(
      { name: 'cfa-agents-sp-global', version: '1.0.0' },
      { capabilities: {} },
    );
  }

  async connect(config: SpGlobalBridgeConfig = {}): Promise<void> {
    if (this.connected) return;

    const serverPath = config.serverPath ?? new URL('../../sp-global-mcp-server/dist/index.js', import.meta.url).pathname;
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

  /** Call an S&P Global tool by name */
  async callTool(toolName: string, params: Record<string, unknown>): Promise<unknown> {
    if (!this.connected) throw new Error('S&P Global bridge not connected');

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
 * Create a callSpGlobalTool function for agent contexts.
 */
export async function createSpGlobalToolCaller(config?: SpGlobalBridgeConfig): Promise<{
  callSpGlobalTool: (toolName: string, params: Record<string, unknown>) => Promise<unknown>;
  bridge: SpGlobalBridge;
}> {
  const bridge = new SpGlobalBridge();
  await bridge.connect(config);
  return {
    callSpGlobalTool: (name, params) => bridge.callTool(name, params),
    bridge,
  };
}
