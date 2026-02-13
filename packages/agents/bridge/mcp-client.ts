// MCP Client Bridge — connects agents to the corp-finance-mcp tool server
// Uses MCP protocol so agents access all 215 tools with full Zod validation

import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { StdioClientTransport } from '@modelcontextprotocol/sdk/client/stdio.js';

export interface McpBridgeConfig {
  /** Path to the MCP server entry point (default: packages/mcp-server/dist/index.js) */
  serverPath?: string;
  /** Command to launch the server (default: 'node') */
  command?: string;
}

export class McpBridge {
  private client: Client;
  private transport: StdioClientTransport | null = null;
  private connected = false;

  constructor() {
    this.client = new Client(
      { name: 'cfa-agents', version: '1.0.0' },
      { capabilities: {} },
    );
  }

  async connect(config: McpBridgeConfig = {}): Promise<void> {
    if (this.connected) return;

    const serverPath = config.serverPath ?? new URL('../../mcp-server/dist/index.js', import.meta.url).pathname;
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

  /** List all available tools from the MCP server */
  async listTools(): Promise<Array<{ name: string; description?: string }>> {
    if (!this.connected) throw new Error('MCP bridge not connected');
    const result = await this.client.listTools();
    return result.tools.map(t => ({ name: t.name, description: t.description }));
  }

  /** Call a tool by name — this is the function agents use via callTool */
  async callTool(toolName: string, params: Record<string, unknown>): Promise<unknown> {
    if (!this.connected) throw new Error('MCP bridge not connected');

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
 * Create a fully wired callTool function that agents can use.
 * Manages the MCP client lifecycle.
 */
export async function createToolCaller(config?: McpBridgeConfig): Promise<{
  callTool: (toolName: string, params: Record<string, unknown>) => Promise<unknown>;
  bridge: McpBridge;
}> {
  const bridge = new McpBridge();
  await bridge.connect(config);
  return {
    callTool: (name, params) => bridge.callTool(name, params),
    bridge,
  };
}
