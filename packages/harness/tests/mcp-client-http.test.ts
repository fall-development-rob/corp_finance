/**
 * Tests for the HTTP MCP client (StreamableHTTP transport).
 *
 * Unit tests mock the SDK Client + StreamableHTTPClientTransport so no real
 * network is required. The integration test is skipped unless HTTP_MCP_URL
 * is set in the environment.
 */
import { describe, it, expect, vi, beforeEach } from "vitest";
import type { MCPClient } from "../src/types.js";

// ---------------------------------------------------------------------------
// Mock the MCP SDK before importing the module under test
// ---------------------------------------------------------------------------

const mockConnect = vi.fn<() => Promise<void>>().mockResolvedValue(undefined);
const mockListTools = vi.fn();
const mockCallTool = vi.fn();
const mockClose = vi.fn<() => Promise<void>>().mockResolvedValue(undefined);

vi.mock("@modelcontextprotocol/sdk/client/index.js", () => ({
  Client: vi.fn().mockImplementation(() => ({
    connect: mockConnect,
    listTools: mockListTools,
    callTool: mockCallTool,
    close: mockClose,
  })),
}));

vi.mock("@modelcontextprotocol/sdk/client/streamableHttp.js", () => ({
  StreamableHTTPClientTransport: vi.fn().mockImplementation(() => ({})),
}));

// Import after mocks are registered
const { createHTTPMCPClient } = await import("../src/mcp-client/http.js");

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeServerConfig(overrides?: Partial<{ name: string; prefix: string; url: string }>) {
  return {
    name: overrides?.name ?? "cfa-core",
    prefix: overrides?.prefix ?? "mcp__plugin_cfa-core_cfa-core__",
    url: overrides?.url ?? "https://mcp.example.com/mcp",
  };
}

function makeWireTool(bareName: string, prefix: string) {
  return {
    name: `${prefix}${bareName}`,
    description: `${bareName} tool`,
    inputSchema: { type: "object" as const, properties: {}, required: [] },
  };
}

// ---------------------------------------------------------------------------
// Test 1: initialize() connects all transports and populates the tool catalog
// ---------------------------------------------------------------------------

describe("createHTTPMCPClient — initialize", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("connects each server and populates the tool catalog", async () => {
    const cfaCoreTools = [
      makeWireTool("option_pricer", "mcp__plugin_cfa-core_cfa-core__"),
      makeWireTool("bond_pricer", "mcp__plugin_cfa-core_cfa-core__"),
    ];
    const cfaDataTools = [makeWireTool("yf_quote", "mcp__plugin_cfa-data_data__")];

    mockListTools
      .mockResolvedValueOnce({ tools: cfaCoreTools })
      .mockResolvedValueOnce({ tools: cfaDataTools });

    const client: MCPClient = createHTTPMCPClient([
      makeServerConfig({ name: "cfa-core", prefix: "mcp__plugin_cfa-core_cfa-core__" }),
      makeServerConfig({ name: "cfa-data", prefix: "mcp__plugin_cfa-data_data__", url: "https://data.example.com/mcp" }),
    ]);

    await client.initialize();

    // Two SDK Client instances should have called connect
    expect(mockConnect).toHaveBeenCalledTimes(2);

    const tools = await client.listTools();
    expect(tools).toHaveLength(3);
    expect(tools.map((t) => t.name)).toContain("option_pricer");
    expect(tools.map((t) => t.name)).toContain("bond_pricer");
    expect(tools.map((t) => t.name)).toContain("yf_quote");

    // Bare names — no prefix in catalog
    for (const tool of tools) {
      expect(tool.name).not.toContain("mcp__");
    }
  });

  it("propagates connection errors with a descriptive message", async () => {
    mockConnect.mockRejectedValueOnce(new Error("ECONNREFUSED"));

    const client = createHTTPMCPClient([makeServerConfig()]);
    await expect(client.initialize()).rejects.toThrow(/HTTP MCP connect failed.*cfa-core/);
  });
});

// ---------------------------------------------------------------------------
// Test 2: callTool routes to the correct SDK client
// ---------------------------------------------------------------------------

describe("createHTTPMCPClient — callTool", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("routes to the correct server and unwraps text content", async () => {
    const wireTool = makeWireTool("option_pricer", "mcp__plugin_cfa-core_cfa-core__");
    mockListTools.mockResolvedValue({ tools: [wireTool] });
    mockCallTool.mockResolvedValue({
      content: [{ type: "text", text: '{"price":"0.0208"}' }],
    });

    const client = createHTTPMCPClient([makeServerConfig()]);
    await client.initialize();

    const result = await client.callTool("option_pricer", { spot_price: 0.10 });
    expect(mockCallTool).toHaveBeenCalledWith(
      {
        name: "mcp__plugin_cfa-core_cfa-core__option_pricer",
        arguments: { spot_price: 0.10 },
      },
      undefined,
      { timeout: 30_000 },
    );
    expect(result).toEqual({ price: "0.0208" });
  });

  it("returns plain string when text content is not valid JSON", async () => {
    const wireTool = makeWireTool("some_tool", "mcp__plugin_cfa-core_cfa-core__");
    mockListTools.mockResolvedValue({ tools: [wireTool] });
    mockCallTool.mockResolvedValue({
      content: [{ type: "text", text: "plain text result" }],
    });

    const client = createHTTPMCPClient([makeServerConfig()]);
    await client.initialize();

    const result = await client.callTool("some_tool", {});
    expect(result).toBe("plain text result");
  });

  it("throws for an unknown bare name", async () => {
    mockListTools.mockResolvedValue({ tools: [] });

    const client = createHTTPMCPClient([makeServerConfig()]);
    await client.initialize();

    await expect(client.callTool("nonexistent_tool", {})).rejects.toThrow(
      /Unknown tool.*nonexistent_tool/,
    );
  });
});

// ---------------------------------------------------------------------------
// Test 3: Bare-name collision detection
// ---------------------------------------------------------------------------

describe("createHTTPMCPClient — collision detection", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("throws when two servers expose the same bare name", async () => {
    // Both servers expose a tool that strips to "duplicate_tool"
    const cfaCoreTools = [makeWireTool("duplicate_tool", "mcp__plugin_cfa-core_cfa-core__")];
    const cfaDataTools = [makeWireTool("duplicate_tool", "mcp__plugin_cfa-data_data__")];

    mockListTools
      .mockResolvedValueOnce({ tools: cfaCoreTools })
      .mockResolvedValueOnce({ tools: cfaDataTools });

    const client = createHTTPMCPClient([
      makeServerConfig({ name: "cfa-core", prefix: "mcp__plugin_cfa-core_cfa-core__" }),
      makeServerConfig({ name: "cfa-data", prefix: "mcp__plugin_cfa-data_data__", url: "https://data.example.com/mcp" }),
    ]);

    await expect(client.initialize()).rejects.toThrow(/collision/i);
  });
});

// ---------------------------------------------------------------------------
// Test 4: close() cleans up
// ---------------------------------------------------------------------------

describe("createHTTPMCPClient — close", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("calls close on each SDK client and empties the catalog", async () => {
    mockListTools.mockResolvedValue({ tools: [makeWireTool("option_pricer", "mcp__plugin_cfa-core_cfa-core__")] });

    const client = createHTTPMCPClient([makeServerConfig()]);
    await client.initialize();

    expect((await client.listTools()).length).toBe(1);

    await client.close();

    expect(mockClose).toHaveBeenCalledTimes(1);
    expect(await client.listTools()).toHaveLength(0);
  });

  it("throws on callTool after close", async () => {
    mockListTools.mockResolvedValue({ tools: [makeWireTool("option_pricer", "mcp__plugin_cfa-core_cfa-core__")] });

    const client = createHTTPMCPClient([makeServerConfig()]);
    await client.initialize();
    await client.close();

    await expect(client.callTool("option_pricer", {})).rejects.toThrow(/Unknown tool/);
  });
});

// ---------------------------------------------------------------------------
// Integration test (skipped unless HTTP_MCP_URL is set)
// ---------------------------------------------------------------------------

const HTTP_MCP_URL = process.env["HTTP_MCP_URL"];

describe.skipIf(!HTTP_MCP_URL)("http.integration", () => {
  it("lists tools from a real HTTP MCP endpoint", async () => {
    const client = createHTTPMCPClient([
      {
        name: "remote",
        prefix: "",
        url: HTTP_MCP_URL!,
        headers: process.env["HTTP_MCP_TOKEN"]
          ? { Authorization: `Bearer ${process.env["HTTP_MCP_TOKEN"]}` }
          : undefined,
      },
    ]);

    await client.initialize();
    const tools = await client.listTools();
    expect(tools.length).toBeGreaterThan(0);
    await client.close();
  }, 60_000);
});
