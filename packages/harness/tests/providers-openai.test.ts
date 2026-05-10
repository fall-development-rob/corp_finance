/**
 * Unit + live-skip tests for the OpenAI provider.
 *
 * All unit tests inject a stub client via `_client` so no HTTP calls are made.
 * The live test is gated on OPENAI_API_KEY and skipped in CI.
 */
import { describe, it, expect, vi } from "vitest";
import OpenAI from "openai";
import { createOpenAIProvider } from "../src/core/providers/openai.js";
import type {
  CanonicalTool,
  Message,
  ProviderTurnRequest,
} from "../src/types.js";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const TOOL_CALC: CanonicalTool = {
  name: "calc",
  description: "Simple calculator",
  input_schema: {
    type: "object",
    properties: { expression: { type: "string" } },
    required: ["expression"],
  },
};

function makeRequest(overrides: Partial<ProviderTurnRequest> = {}): ProviderTurnRequest {
  return {
    systemPrompt: "You are a helpful assistant.",
    messages: [{ role: "user", content: [{ type: "text", text: "Hello" }] }],
    tools: [],
    ...overrides,
  };
}

/**
 * Build a minimal stub for OpenAI.chat.completions.create.
 * The stub returns the provided completion choice(s).
 */
function makeStubClient(choice: OpenAI.Chat.ChatCompletion["choices"][number]): OpenAI {
  const create = vi.fn().mockResolvedValue({
    choices: [choice],
    usage: { prompt_tokens: 10, completion_tokens: 5, total_tokens: 15 },
  } satisfies Partial<OpenAI.Chat.ChatCompletion>);

  // We only need chat.completions.create; cast via unknown to satisfy the SDK type.
  return { chat: { completions: { create } } } as unknown as OpenAI;
}

function textChoice(text: string): OpenAI.Chat.ChatCompletion["choices"][number] {
  return {
    index: 0,
    finish_reason: "stop",
    message: { role: "assistant", content: text },
    logprobs: null,
  };
}

function toolCallChoice(
  id: string,
  name: string,
  args: Record<string, unknown>,
): OpenAI.Chat.ChatCompletion["choices"][number] {
  return {
    index: 0,
    finish_reason: "tool_calls",
    message: {
      role: "assistant",
      content: null,
      tool_calls: [
        { id, type: "function", function: { name, arguments: JSON.stringify(args) } },
      ],
    },
    logprobs: null,
  };
}

function mixedChoice(
  text: string,
  id: string,
  name: string,
  args: Record<string, unknown>,
): OpenAI.Chat.ChatCompletion["choices"][number] {
  return {
    index: 0,
    finish_reason: "tool_calls",
    message: {
      role: "assistant",
      content: text,
      tool_calls: [
        { id, type: "function", function: { name, arguments: JSON.stringify(args) } },
      ],
    },
    logprobs: null,
  };
}

// ---------------------------------------------------------------------------
// Test 1: text-only response maps to end_turn + TextBlock
// ---------------------------------------------------------------------------

describe("createOpenAIProvider", () => {
  it("Test 1: translates a simple text-only turn correctly", async () => {
    const stub = makeStubClient(textChoice("Hello there!"));
    const provider = createOpenAIProvider({ _client: stub });

    const response = await provider.turn(makeRequest());

    expect(response.stopReason).toBe("end_turn");
    expect(response.message.role).toBe("assistant");
    expect(response.message.content).toHaveLength(1);
    const block = response.message.content[0]!;
    expect(block.type).toBe("text");
    if (block.type === "text") expect(block.text).toBe("Hello there!");
    expect(response.usage).toEqual({ inputTokens: 10, outputTokens: 5 });
  });

  // -------------------------------------------------------------------------
  // Test 2: tool_use response maps to ToolUseBlocks + stopReason "tool_use"
  // -------------------------------------------------------------------------

  it("Test 2: translates a tool_use response correctly", async () => {
    const stub = makeStubClient(toolCallChoice("call_abc", "calc", { expression: "1+1" }));
    const provider = createOpenAIProvider({ _client: stub });

    const response = await provider.turn(makeRequest({ tools: [TOOL_CALC] }));

    expect(response.stopReason).toBe("tool_use");
    expect(response.message.content).toHaveLength(1);
    const block = response.message.content[0]!;
    expect(block.type).toBe("tool_use");
    if (block.type === "tool_use") {
      expect(block.id).toBe("call_abc");
      expect(block.name).toBe("calc");
      expect(block.input).toEqual({ expression: "1+1" });
    }
  });

  // -------------------------------------------------------------------------
  // Test 3: multi-turn with tool_results fans out to role="tool" messages
  // -------------------------------------------------------------------------

  it("Test 3: translates tool_result blocks to role=tool messages", async () => {
    const stub = makeStubClient(textChoice("The answer is 2."));
    const provider = createOpenAIProvider({ _client: stub });

    // A conversation that includes a tool result
    const messages: Message[] = [
      { role: "user", content: [{ type: "text", text: "What is 1+1?" }] },
      {
        role: "assistant",
        content: [{ type: "tool_use", id: "call_1", name: "calc", input: { expression: "1+1" } }],
      },
      {
        role: "user",
        content: [{ type: "tool_result", tool_use_id: "call_1", content: "2", is_error: false }],
      },
    ];

    await provider.turn(makeRequest({ messages }));

    // Inspect what was passed to the stub
    const createFn = (stub.chat.completions.create as ReturnType<typeof vi.fn>);
    const callArgs = createFn.mock.calls[0]![0] as OpenAI.Chat.ChatCompletionCreateParamsNonStreaming;
    const sentMessages = callArgs.messages;

    // system + user + assistant + tool (from tool_result)
    expect(sentMessages).toHaveLength(4);
    expect(sentMessages[0]!.role).toBe("system");
    expect(sentMessages[1]!.role).toBe("user");
    expect(sentMessages[2]!.role).toBe("assistant");
    expect(sentMessages[3]!.role).toBe("tool");

    const toolMsg = sentMessages[3] as OpenAI.Chat.ChatCompletionToolMessageParam;
    expect(toolMsg.tool_call_id).toBe("call_1");
    expect(toolMsg.content).toBe("2");
  });

  // -------------------------------------------------------------------------
  // Test 4: canonical tool schema maps to OpenAI function parameters
  // -------------------------------------------------------------------------

  it("Test 4: translates canonical tool schema to OpenAI function format", async () => {
    const stub = makeStubClient(textChoice("ok"));
    const provider = createOpenAIProvider({ _client: stub });

    await provider.turn(makeRequest({ tools: [TOOL_CALC] }));

    const createFn = (stub.chat.completions.create as ReturnType<typeof vi.fn>);
    const callArgs = createFn.mock.calls[0]![0] as OpenAI.Chat.ChatCompletionCreateParamsNonStreaming;

    expect(callArgs.tools).toHaveLength(1);
    const oaiTool = callArgs.tools![0]!;
    expect(oaiTool.type).toBe("function");
    expect(oaiTool.function.name).toBe("calc");
    expect(oaiTool.function.description).toBe("Simple calculator");
    // parameters must equal input_schema exactly
    expect(oaiTool.function.parameters).toEqual(TOOL_CALC.input_schema);
  });

  // -------------------------------------------------------------------------
  // Test 5: mixed text + tool_calls response preserves both block types
  // -------------------------------------------------------------------------

  it("Test 5: preserves both text and tool_use blocks from a mixed response", async () => {
    const stub = makeStubClient(mixedChoice("Let me calculate that.", "call_2", "calc", { expression: "2*3" }));
    const provider = createOpenAIProvider({ _client: stub });

    const response = await provider.turn(makeRequest({ tools: [TOOL_CALC] }));

    expect(response.message.content).toHaveLength(2);

    const [textBlock, toolBlock] = response.message.content;
    expect(textBlock!.type).toBe("text");
    if (textBlock!.type === "text") expect(textBlock.text).toBe("Let me calculate that.");

    expect(toolBlock!.type).toBe("tool_use");
    if (toolBlock!.type === "tool_use") {
      expect(toolBlock.id).toBe("call_2");
      expect(toolBlock.name).toBe("calc");
      expect(toolBlock.input).toEqual({ expression: "2*3" });
    }
  });

  // -------------------------------------------------------------------------
  // Test 6: empty tools array omits the tools key (some endpoints reject [])
  // -------------------------------------------------------------------------

  it("omits tools parameter when request.tools is empty", async () => {
    const stub = makeStubClient(textChoice("done"));
    const provider = createOpenAIProvider({ _client: stub });

    await provider.turn(makeRequest({ tools: [] }));

    const createFn = (stub.chat.completions.create as ReturnType<typeof vi.fn>);
    const callArgs = createFn.mock.calls[0]![0] as OpenAI.Chat.ChatCompletionCreateParamsNonStreaming;
    expect(callArgs.tools).toBeUndefined();
  });

  // -------------------------------------------------------------------------
  // Live test — skipped when OPENAI_API_KEY is absent
  // -------------------------------------------------------------------------

  it.skipIf(!process.env["OPENAI_API_KEY"])(
    "makes one real API turn and receives a tool_use response",
    async () => {
      const liveProvider = createOpenAIProvider({ defaultModel: "gpt-4o-mini" });

      const trivialTool: CanonicalTool = {
        name: "get_weather",
        description: "Get current weather for a city.",
        input_schema: {
          type: "object",
          properties: { city: { type: "string", description: "City name" } },
          required: ["city"],
        },
      };

      const response = await liveProvider.turn({
        systemPrompt: "You must always call the get_weather tool.",
        messages: [{ role: "user", content: [{ type: "text", text: "What is the weather in London?" }] }],
        tools: [trivialTool],
        model: "gpt-4o-mini",
      });

      expect(response.stopReason).toBe("tool_use");
      const toolBlock = response.message.content.find((b) => b.type === "tool_use");
      expect(toolBlock).toBeDefined();
      if (toolBlock?.type === "tool_use") {
        expect(toolBlock.name).toBe("get_weather");
      }
    },
    30_000, // 30 s timeout for live network call
  );
});
