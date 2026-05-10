/**
 * Tests for the SSE MCP client.
 *
 * All tests mock the @modelcontextprotocol/sdk to avoid network calls.
 * The integration test is skipped (no real SSE server available locally).
 *
 * To run the integration test manually, set env vars:
 *   SSE_MCP_URL=https://your-server.example.com/sse
 *   SSE_MCP_AUTH=Bearer <token>
 * and change `it.skip` to `it` in the integration block below.
 */
import { describe, it, expect, vi, beforeEach } from "vitest";
import type { SSEMCPServerConfig } from "../src/mcp-client/sse.js";
import { createSSEMCPClient } from "../src/mcp-client/sse.js";

// ---------------------------------------------------------------------------
// SDK mock
// ---------------------------------------------------------------------------

const mockConnect = vi.fn().mockResolvedValue(undefined);
const mockClose = vi.fn().mockResolvedValue(undefined);
const mockListTools = vi.fn();
const mockCallTool = vi.fn();

vi.mock("@modelcontextprotocol/sdk/client/index.js", () => ({
  Client: vi.fn().mockImplementation(() => ({
    connect: mockConnect,
    close: mockClose,
    listTools: mockListTools,
    callTool: mockCallTool,
  })),
}));

vi.mock("@modelcontextprotocol/sdk/client/sse.js", () => ({
  SSEClientTransport: vi.fn().mockImplementation(() => ({})),
}));

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeConfig(overrides: Partial<SSEMCPServerConfig> = {}): SSEMCPServerConfig {
  return {
    name: "test-server",
    prefix: "",
    url: "https://mcp.example.com/sse",
    ...overrides,
  };
}

// ---------------------------------------------------------------------------
// Test suite
// ---------------------------------------------------------------------------

describe("SSE MCP client", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // -------------------------------------------------------------------------
  // Test 1: initialize populates tool catalog from listTools
  // -------------------------------------------------------------------------

  it("initialize() populates tool catalog from listTools()", async () => {
    mockListTools.mockResolvedValue({
      tools: [
        {
          name: "option_pricer",
          description: "Price an option",
          inputSchema: {
            type: "object",
            properties: { spot_price: { type: "number" } },
            required: ["spot_price"],
          },
        },
        {
          name: "bond_pricer",
          description: "Price a bond",
          inputSchema: {
            type: "object",
            properties: { face_value: { type: "number" } },
          },
        },
      ],
    });

    const client = createSSEMCPClient([makeConfig()]);
    await client.initialize();

    const tools = await client.listTools();
    expect(tools).toHaveLength(2);

    const optionTool = tools.find((t) => t.name === "option_pricer");
    expect(optionTool).toBeDefined();
    expect(optionTool!.description).toBe("Price an option");
    expect(optionTool!.input_schema.type).toBe("object");
    expect(optionTool!.input_schema.required).toEqual(["spot_price"]);

    expect(mockConnect).toHaveBeenCalledOnce();
    expect(mockListTools).toHaveBeenCalledOnce();
  });

  // -------------------------------------------------------------------------
  // Test 2: callTool routes to the correct server's client
  // -------------------------------------------------------------------------

  it("callTool() routes to the correct server and passes arguments as-is", async () => {
    mockListTools.mockResolvedValueOnce({
      tools: [{ name: "wacc_calculator", description: "Calc WACC", inputSchema: { type: "object", properties: {} } }],
    }).mockResolvedValueOnce({
      tools: [{ name: "yf_quote", description: "Get quote", inputSchema: { type: "object", properties: {} } }],
    });

    const resultPayload = { content: [{ type: "text", text: '{"wacc":0.1}' }] };
    mockCallTool.mockResolvedValue(resultPayload);

    const client = createSSEMCPClient([
      makeConfig({ name: "cfa-core", prefix: "", url: "https://core.example.com/sse" }),
      makeConfig({ name: "cfa-data", prefix: "", url: "https://data.example.com/sse" }),
    ]);
    await client.initialize();

    const result = await client.callTool("wacc_calculator", { equity_cost: 0.12 });

    expect(mockCallTool).toHaveBeenCalledOnce();
    expect(mockCallTool).toHaveBeenCalledWith({
      name: "wacc_calculator",
      arguments: { equity_cost: 0.12 },
    });

    // Result is parsed from text content
    expect(result).toEqual({ wacc: 0.1 });
  });

  it("callTool() throws for an unknown bare name", async () => {
    mockListTools.mockResolvedValue({ tools: [] });
    const client = createSSEMCPClient([makeConfig()]);
    await client.initialize();

    await expect(client.callTool("nonexistent_tool", {})).rejects.toThrow(
      /Unknown tool.*nonexistent_tool/,
    );
  });

  // -------------------------------------------------------------------------
  // Test 3: bare-name collision detection on initialize()
  // -------------------------------------------------------------------------

  it("initialize() throws on bare-name collision across servers", async () => {
    mockListTools
      .mockResolvedValueOnce({
        tools: [{ name: "duplicate_tool", description: "A", inputSchema: { type: "object", properties: {} } }],
      })
      .mockResolvedValueOnce({
        tools: [{ name: "duplicate_tool", description: "B", inputSchema: { type: "object", properties: {} } }],
      });

    const client = createSSEMCPClient([
      makeConfig({ name: "server-a", prefix: "", url: "https://a.example.com/sse" }),
      makeConfig({ name: "server-b", prefix: "", url: "https://b.example.com/sse" }),
    ]);

    await expect(client.initialize()).rejects.toThrow(
      /collision.*duplicate_tool/i,
    );
  });

  // -------------------------------------------------------------------------
  // Test 4: close() shuts down all transports cleanly
  // -------------------------------------------------------------------------

  it("close() calls close on all SDK clients and clears state", async () => {
    mockListTools
      .mockResolvedValueOnce({ tools: [{ name: "tool_a", inputSchema: { type: "object", properties: {} } }] })
      .mockResolvedValueOnce({ tools: [{ name: "tool_b", inputSchema: { type: "object", properties: {} } }] });

    const client = createSSEMCPClient([
      makeConfig({ name: "srv-1", prefix: "", url: "https://s1.example.com/sse" }),
      makeConfig({ name: "srv-2", prefix: "", url: "https://s2.example.com/sse" }),
    ]);
    await client.initialize();

    const toolsBefore = await client.listTools();
    expect(toolsBefore).toHaveLength(2);

    await client.close();

    // After close, mockClose was called for each client (2 servers)
    expect(mockClose).toHaveBeenCalledTimes(2);

    // Catalog is cleared
    const toolsAfter = await client.listTools();
    expect(toolsAfter).toHaveLength(0);

    // callTool after close should throw
    await expect(client.callTool("tool_a", {})).rejects.toThrow(/Unknown tool/);
  });

  // -------------------------------------------------------------------------
  // Integration test (skipped — no real SSE server locally)
  // -------------------------------------------------------------------------

  it.skip(
    "integration test against a real SSE MCP server",
    async () => {
      /**
       * To run this test manually:
       *   1. Start a real SSE MCP server (e.g., `npx @modelcontextprotocol/server-sse`)
       *   2. Set SSE_MCP_URL to its SSE endpoint URL
       *   3. Set SSE_MCP_AUTH to the Authorization header value (optional)
       *   4. Change `it.skip` to `it` and run:
       *        SSE_MCP_URL=https://... SSE_MCP_AUTH="Bearer ..." npm test -- mcp-client-sse
       */
      const url = process.env["SSE_MCP_URL"] ?? "http://localhost:3001/sse";
      const auth = process.env["SSE_MCP_AUTH"];

      const client = createSSEMCPClient([
        {
          name: "remote-mcp",
          prefix: "",
          url,
          headers: auth ? { Authorization: auth } : undefined,
        },
      ]);

      await client.initialize();
      const tools = await client.listTools();
      expect(tools.length).toBeGreaterThan(0);
      await client.close();
    },
    60_000,
  );
});
