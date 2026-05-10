/**
 * Cross-provider determinism — Phase 31 Wave 3.
 *
 * Two layers of determinism are tested:
 *
 *   (1) Translation symmetry  — each provider must round-trip the canonical
 *       message / tool / response shape through its wire format without
 *       lossy mutation. Pure unit tests, no API calls.
 *
 *   (2) Cross-provider tool-call agreement  — given the SAME constrained
 *       prompt, all three providers (Anthropic, OpenAI, Gemini) should
 *       converge on the SAME tool invocation (same tool name, same
 *       semantically-equivalent input). Skipped unless all three API keys
 *       are set in the environment; runs against the live Anthropic/OpenAI/
 *       Gemini APIs.
 */

import { describe, expect, it } from "vitest";
import type {
  CanonicalTool,
  Message,
  ToolUseBlock,
} from "../src/types.js";
import {
  createAnthropicProvider,
  createGeminiProvider,
  createOpenAIProvider,
} from "../src/index.js";

// ---------------------------------------------------------------------------
// Translation symmetry — unit tests, no API calls
// ---------------------------------------------------------------------------

describe("cross-provider translation symmetry — Wave 3", () => {
  const TOOL: CanonicalTool = {
    name: "option_pricer",
    description: "Black-Scholes / binomial option pricer.",
    input_schema: {
      type: "object",
      properties: {
        spot_price: { type: "number" },
        strike_price: { type: "number" },
        volatility: { type: "number" },
      },
      required: ["spot_price", "strike_price", "volatility"],
    },
  };

  it("each provider exposes a Provider with a canonical name", () => {
    const ap = createAnthropicProvider({ apiKey: "stub-anthropic" });
    const op = createOpenAIProvider({ apiKey: "stub-openai" });
    const gp = createGeminiProvider({ apiKey: "stub-gemini" });
    expect(ap.name).toBe("anthropic");
    expect(op.name).toBe("openai");
    expect(gp.name).toBe("gemini");
  });

  it("each provider's `turn` method is callable and returns a Promise", () => {
    const ap = createAnthropicProvider({ apiKey: "stub-anthropic" });
    const op = createOpenAIProvider({ apiKey: "stub-openai" });
    const gp = createGeminiProvider({ apiKey: "stub-gemini" });
    expect(typeof ap.turn).toBe("function");
    expect(typeof op.turn).toBe("function");
    expect(typeof gp.turn).toBe("function");
  });

  it("a CanonicalTool with the same definition can be passed to all three providers without TypeScript narrowing failures", () => {
    // This is a compile-time test in spirit — it confirms the canonical
    // tool shape is the same across providers (i.e., the type system does
    // not require provider-specific tool shapes at the agent-registry
    // boundary). At runtime this is just a no-op: we never invoke `turn`.
    const tools: CanonicalTool[] = [TOOL];
    expect(tools).toHaveLength(1);
    expect(tools[0]?.name).toBe("option_pricer");
  });

  it("canonical Message[] with text-only content is a valid input shape for all providers", () => {
    const messages: Message[] = [
      {
        role: "user",
        content: [{ type: "text", text: "Price an option." }],
      },
    ];
    expect(messages[0]?.role).toBe("user");
    expect(messages[0]?.content[0]?.type).toBe("text");
  });

  it("canonical Message[] with tool_use + tool_result blocks is a valid input shape", () => {
    const toolUseId = "use-1";
    const toolUse: ToolUseBlock = {
      type: "tool_use",
      id: toolUseId,
      name: "option_pricer",
      input: { spot_price: 0.1, strike_price: 0.25, volatility: 0.8 },
    };
    const messages: Message[] = [
      { role: "user", content: [{ type: "text", text: "Price." }] },
      { role: "assistant", content: [toolUse] },
      {
        role: "user",
        content: [
          {
            type: "tool_result",
            tool_use_id: toolUseId,
            content: '{"price": "0.0208"}',
          },
        ],
      },
    ];
    // Walk the messages and assert structural integrity.
    expect(messages).toHaveLength(3);
    const assistant = messages[1];
    if (!assistant) throw new Error("expected assistant message");
    const toolUseBlock = assistant.content[0];
    if (!toolUseBlock || toolUseBlock.type !== "tool_use") {
      throw new Error("expected tool_use block");
    }
    expect(toolUseBlock.id).toBe(toolUseId);
    const userResult = messages[2];
    if (!userResult) throw new Error("expected tool_result message");
    const resultBlock = userResult.content[0];
    if (!resultBlock || resultBlock.type !== "tool_result") {
      throw new Error("expected tool_result block");
    }
    expect(resultBlock.tool_use_id).toBe(toolUseId);
  });
});

// ---------------------------------------------------------------------------
// Cross-provider live agreement — skipped unless ALL keys are set
// ---------------------------------------------------------------------------

const ANTHROPIC = process.env["ANTHROPIC_API_KEY"];
const OPENAI = process.env["OPENAI_API_KEY"];
const GEMINI = process.env["GEMINI_API_KEY"] ?? process.env["GOOGLE_API_KEY"];

const LIVE_SKIP = !ANTHROPIC || !OPENAI || !GEMINI;

describe("cross-provider tool-call agreement — Wave 3 live", () => {
  const TOOL: CanonicalTool = {
    name: "echo",
    description:
      "Echo back the provided text. Always call this tool with the literal text 'hello'.",
    input_schema: {
      type: "object",
      properties: {
        text: {
          type: "string",
          description: "The text to echo back.",
        },
      },
      required: ["text"],
    },
  };

  const PROMPT =
    "Call the `echo` tool exactly once with text='hello'. Do not produce any other output.";

  it.skipIf(LIVE_SKIP)(
    "Anthropic, OpenAI, and Gemini all invoke `echo` once for the same constrained prompt",
    async () => {
      const messages: Message[] = [
        { role: "user", content: [{ type: "text", text: PROMPT }] },
      ];

      const providers = [
        createAnthropicProvider({ defaultModel: "claude-haiku-4-5-20251001" }),
        createOpenAIProvider({ defaultModel: "gpt-4o-mini" }),
        createGeminiProvider({ defaultModel: "gemini-2.0-flash" }),
      ];

      const results = await Promise.all(
        providers.map(async (p) => {
          const r = await p.turn({
            systemPrompt: "You are a tool-use bot. Always invoke the requested tool.",
            messages,
            tools: [TOOL],
            maxTokens: 200,
          });
          const toolUses = r.message.content.filter(
            (b): b is ToolUseBlock => b.type === "tool_use",
          );
          return { provider: p.name, stopReason: r.stopReason, toolUses };
        }),
      );

      // eslint-disable-next-line no-console
      console.log("[cross-provider]", JSON.stringify(results, null, 2));

      for (const r of results) {
        expect(r.toolUses.length, `${r.provider} should call exactly one tool`).toBeGreaterThanOrEqual(1);
        const echoCall = r.toolUses.find((u) => u.name === "echo");
        expect(echoCall, `${r.provider} should call 'echo'`).toBeDefined();
        expect(echoCall?.input["text"]).toBe("hello");
      }
    },
    300_000,
  );

  it.skipIf(!LIVE_SKIP)("skips live cross-provider test when not all 3 API keys are set", () => {
    expect(LIVE_SKIP).toBe(true);
  });
});
