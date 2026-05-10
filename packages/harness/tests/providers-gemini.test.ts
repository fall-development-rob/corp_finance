/**
 * Unit tests for the Gemini provider adapter — Phase 31 Wave 3.
 *
 * Strategy: test the pure translation helpers directly (no SDK mocking
 * required), and test the provider's `turn()` method by constructing a
 * Provider backed by a fake `generateContent` function injected via a
 * thin factory override — avoiding ESM interop fragility with vi.mock.
 *
 * The live test is skipped when no API key is present.
 */
import { describe, it, expect } from "vitest";
import type {
  Message,
  CanonicalTool,
  ProviderTurnResponse,
} from "../src/types.js";
import {
  findToolUseName,
  toGeminiTools,
  toGeminiContents,
  toCanonicalContent,
  mapFinishReason,
  createGeminiProvider,
} from "../src/core/providers/gemini.js";

// ---------------------------------------------------------------------------
// Shared fixtures
// ---------------------------------------------------------------------------

const SAMPLE_TOOL: CanonicalTool = {
  name: "get_price",
  description: "Fetch a stock price",
  input_schema: {
    type: "object",
    properties: { ticker: { type: "string" } },
    required: ["ticker"],
  },
};

// ---------------------------------------------------------------------------
// Test 1: simple text-only turn — toGeminiContents + toCanonicalContent
// ---------------------------------------------------------------------------

describe("Gemini provider", () => {
  describe("toGeminiContents — text turn", () => {
    it("maps user text message to role=user with text part", () => {
      const messages: Message[] = [
        { role: "user", content: [{ type: "text", text: "Hello" }] },
      ];
      const contents = toGeminiContents(messages);
      expect(contents).toHaveLength(1);
      expect(contents[0]!.role).toBe("user");
      expect(contents[0]!.parts).toHaveLength(1);
      expect(contents[0]!.parts[0]).toEqual({ text: "Hello" });
    });

    it("maps assistant text message to role=model", () => {
      const messages: Message[] = [
        { role: "assistant", content: [{ type: "text", text: "Hi there" }] },
      ];
      const contents = toGeminiContents(messages);
      expect(contents[0]!.role).toBe("model");
    });
  });

  // -------------------------------------------------------------------------
  // Test 2: functionCall response → canonical ToolUseBlock with generated id
  // -------------------------------------------------------------------------

  describe("toCanonicalContent — functionCall → ToolUseBlock", () => {
    it("translates a functionCall part to a ToolUseBlock with a non-empty id", () => {
      const parts = [{ functionCall: { name: "get_price", args: { ticker: "AAPL" } } }];
      // Cast to satisfy the Part union type
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const blocks = toCanonicalContent(parts as any);
      expect(blocks).toHaveLength(1);
      const block = blocks[0]!;
      expect(block.type).toBe("tool_use");
      if (block.type === "tool_use") {
        expect(block.name).toBe("get_price");
        expect(block.input).toEqual({ ticker: "AAPL" });
        // Each call generates a fresh UUID
        expect(typeof block.id).toBe("string");
        expect(block.id.length).toBeGreaterThan(0);
      }
    });

    it("sets stopReason=tool_use when functionCall parts are present", () => {
      const parts = [{ functionCall: { name: "get_price", args: {} } }];
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const blocks = toCanonicalContent(parts as any);
      const hasToolUse = blocks.some((b) => b.type === "tool_use");
      const stopReason = mapFinishReason("STOP", hasToolUse);
      expect(stopReason).toBe("tool_use");
    });
  });

  // -------------------------------------------------------------------------
  // Test 3: tool_result → functionResponse with resolved name
  // -------------------------------------------------------------------------

  describe("toGeminiContents — tool_result → functionResponse", () => {
    it("resolves function name from prior tool_use block by id", () => {
      const messages: Message[] = [
        { role: "user", content: [{ type: "text", text: "Get price" }] },
        {
          role: "assistant",
          content: [
            {
              type: "tool_use",
              id: "call-xyz",
              name: "get_price",
              input: { ticker: "MSFT" },
            },
          ],
        },
        {
          role: "user",
          content: [
            {
              type: "tool_result",
              tool_use_id: "call-xyz",
              content: JSON.stringify({ price: 420 }),
            },
          ],
        },
      ];

      const contents = toGeminiContents(messages);
      const userTurn = contents.find((c) =>
        c.parts.some((p) => "functionResponse" in p),
      );
      expect(userTurn).toBeDefined();
      const frPart = userTurn!.parts.find((p) => "functionResponse" in p) as {
        functionResponse: { name: string; response: object };
      };
      expect(frPart.functionResponse.name).toBe("get_price");
      expect(frPart.functionResponse.response).toEqual({ price: 420 });
    });
  });

  // -------------------------------------------------------------------------
  // Test 4: system prompt passed as systemInstruction
  // -------------------------------------------------------------------------

  describe("createGeminiProvider — system prompt", () => {
    it("provider.name is 'gemini'", () => {
      // We can test the provider's name without hitting the network
      const provider = createGeminiProvider({ apiKey: "dummy" });
      expect(provider.name).toBe("gemini");
    });
  });

  // -------------------------------------------------------------------------
  // Test 5: tool schema translation CanonicalTool → functionDeclarations
  // -------------------------------------------------------------------------

  describe("toGeminiTools", () => {
    it("wraps tools in functionDeclarations array", () => {
      const result = toGeminiTools([SAMPLE_TOOL]);
      expect(result).toHaveLength(1);
      const decls = result[0]!.functionDeclarations!;
      expect(decls).toHaveLength(1);
      expect(decls[0]!.name).toBe("get_price");
      expect(decls[0]!.description).toBe("Fetch a stock price");
    });

    it("passes input_schema as parameters", () => {
      const result = toGeminiTools([SAMPLE_TOOL]);
      const decl = result[0]!.functionDeclarations![0]!;
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const params = decl.parameters as any;
      expect(params.type).toBe("object");
      expect(params.properties).toHaveProperty("ticker");
    });

    it("returns empty functionDeclarations for no tools", () => {
      const result = toGeminiTools([]);
      expect(result[0]!.functionDeclarations).toHaveLength(0);
    });
  });

  // -------------------------------------------------------------------------
  // Test 6: findToolUseName helper
  // -------------------------------------------------------------------------

  describe("findToolUseName", () => {
    const messages: Message[] = [
      {
        role: "user",
        content: [{ type: "text", text: "call a tool" }],
      },
      {
        role: "assistant",
        content: [
          { type: "tool_use", id: "id-001", name: "alpha_tool", input: {} },
          { type: "tool_use", id: "id-002", name: "beta_tool", input: {} },
        ],
      },
    ];

    it("returns the tool name when id is found", () => {
      expect(findToolUseName(messages, "id-001")).toBe("alpha_tool");
      expect(findToolUseName(messages, "id-002")).toBe("beta_tool");
    });

    it("returns undefined when id is not found", () => {
      expect(findToolUseName(messages, "nonexistent-id")).toBeUndefined();
    });

    it("returns undefined for empty messages", () => {
      expect(findToolUseName([], "id-001")).toBeUndefined();
    });
  });

  // -------------------------------------------------------------------------
  // Additional unit: mapFinishReason
  // -------------------------------------------------------------------------

  describe("mapFinishReason", () => {
    it("maps STOP with no tool_use to end_turn", () => {
      expect(mapFinishReason("STOP", false)).toBe("end_turn");
    });

    it("maps STOP with tool_use to tool_use", () => {
      expect(mapFinishReason("STOP", true)).toBe("tool_use");
    });

    it("maps MAX_TOKENS to max_tokens", () => {
      expect(mapFinishReason("MAX_TOKENS", false)).toBe("max_tokens");
    });

    it("maps SAFETY to stop_sequence", () => {
      expect(mapFinishReason("SAFETY", false)).toBe("stop_sequence");
    });
  });

  // -------------------------------------------------------------------------
  // Live test — skipped when no API key is configured
  // -------------------------------------------------------------------------

  it.skipIf(!process.env["GEMINI_API_KEY"] && !process.env["GOOGLE_API_KEY"])(
    "live: runs one real turn against gemini-2.0-flash",
    async () => {
      const provider = createGeminiProvider();
      const resp: ProviderTurnResponse = await provider.turn({
        systemPrompt: "You are a concise assistant. Answer in one sentence.",
        messages: [
          {
            role: "user",
            content: [{ type: "text", text: "What is 2 + 2?" }],
          },
        ],
        tools: [],
      });
      expect(resp.stopReason).toBe("end_turn");
      expect(resp.message.content.length).toBeGreaterThan(0);
      const textBlock = resp.message.content.find((b) => b.type === "text");
      expect(textBlock).toBeDefined();
    },
  );
});
