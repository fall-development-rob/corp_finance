// Morningstar MCP Bridge — connects agents to the morningstar-mcp-server for fund & research data
// Mirrors FredBridge but routes to the Morningstar server

import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { StdioClientTransport } from '@modelcontextprotocol/sdk/client/stdio.js';

export interface MorningstarBridgeConfig {
  /** Path to the Morningstar MCP server entry point (default: packages/morningstar-mcp-server/dist/index.js) */
  serverPath?: string;
  /** Command to launch the server (default: 'node') */
  command?: string;
}

export class MorningstarBridge {
  private client: Client;
  private transport: StdioClientTransport | null = null;
  private connected = false;

  constructor() {
    this.client = new Client(
      { name: 'cfa-agents-morningstar', version: '1.0.0' },
      { capabilities: {} },
    );
  }

  async connect(config: MorningstarBridgeConfig = {}): Promise<void> {
    if (this.connected) return;

    const serverPath = config.serverPath ?? new URL('../../morningstar-mcp-server/dist/index.js', import.meta.url).pathname;
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

  /** Call a Morningstar tool by name */
  async callTool(toolName: string, params: Record<string, unknown>): Promise<unknown> {
    if (!this.connected) throw new Error('Morningstar bridge not connected');

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
 * Create a callMorningstarTool function for agent contexts.
 */
export async function createMorningstarToolCaller(config?: MorningstarBridgeConfig): Promise<{
  callMorningstarTool: (toolName: string, params: Record<string, unknown>) => Promise<unknown>;
  bridge: MorningstarBridge;
}> {
  const bridge = new MorningstarBridge();
  await bridge.connect(config);
  return {
    callMorningstarTool: (name, params) => bridge.callTool(name, params),
    bridge,
  };
}
