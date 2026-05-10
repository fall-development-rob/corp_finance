/**
 * OpenAI Chat Completions provider — wraps the `openai` SDK to implement the
 * Provider interface defined in types.ts.
 *
 * Translation rules (ADR-033):
 *   canonical systemPrompt  → first message with role "system"
 *   canonical TextBlocks    → assistant content string / content-parts
 *   canonical ToolUseBlocks → assistant tool_calls (arguments as JSON string)
 *   canonical ToolResult    → role "tool" message per block (one-to-one)
 *   finish_reason mapping   → see toStopReason()
 */
import OpenAI from "openai";
import type {
  CanonicalTool,
  ContentBlock,
  Message,
  Provider,
  ProviderTurnRequest,
  ProviderTurnResponse,
} from "../../types.js";

const DEFAULT_MODEL = "gpt-4-turbo";
const DEFAULT_MAX_TOKENS = 4096;

export interface OpenAIProviderOptions {
  apiKey?: string;
  defaultModel?: string;
  baseURL?: string;
  /**
   * Inject a pre-built OpenAI client. Used only in unit tests so we avoid
   * real HTTP calls. Not exposed in the public signature docs because callers
   * should use apiKey/baseURL instead; the underscore prefix signals "internal".
   */
  _client?: OpenAI;
}

// ---------------------------------------------------------------------------
// Outbound translation: canonical → OpenAI wire format
// ---------------------------------------------------------------------------

type OAIMessage =
  | OpenAI.Chat.ChatCompletionSystemMessageParam
  | OpenAI.Chat.ChatCompletionUserMessageParam
  | OpenAI.Chat.ChatCompletionAssistantMessageParam
  | OpenAI.Chat.ChatCompletionToolMessageParam;

/**
 * Build the OpenAI messages array from the canonical request.
 * System prompt becomes the first message; tool_result blocks fan out to
 * individual role="tool" messages (one per block, required by OpenAI).
 */
function toOAIMessages(systemPrompt: string, messages: Message[]): OAIMessage[] {
  const result: OAIMessage[] = [{ role: "system", content: systemPrompt }];

  for (const msg of messages) {
    if (msg.role === "user") {
      // Check whether this user message carries tool_result blocks.
      const toolResults = msg.content.filter((b): b is Extract<ContentBlock, { type: "tool_result" }> =>
        b.type === "tool_result",
      );
      const textBlocks = msg.content.filter((b) => b.type === "text");

      if (toolResults.length > 0) {
        // Fan out: one role="tool" message per tool_result block.
        for (const tr of toolResults) {
          result.push({
            role: "tool",
            tool_call_id: tr.tool_use_id,
            content: tr.content,
          });
        }
        // Any remaining text in the same user turn becomes a separate user msg.
        if (textBlocks.length > 0) {
          result.push({ role: "user", content: textBlocks.map((b) => (b as { text: string }).text).join("\n") });
        }
      } else {
        const text = msg.content
          .filter((b) => b.type === "text")
          .map((b) => (b as { text: string }).text)
          .join("\n");
        result.push({ role: "user", content: text });
      }
    } else {
      // assistant message: may contain TextBlocks and/or ToolUseBlocks.
      const textBlocks = msg.content.filter((b) => b.type === "text");
      const toolUseBlocks = msg.content.filter(
        (b): b is Extract<ContentBlock, { type: "tool_use" }> => b.type === "tool_use",
      );

      const contentText = textBlocks.length > 0
        ? textBlocks.map((b) => (b as { text: string }).text).join("\n")
        : null;

      const toolCalls: OpenAI.Chat.ChatCompletionMessageToolCall[] = toolUseBlocks.map((b) => ({
        id: b.id,
        type: "function" as const,
        function: { name: b.name, arguments: JSON.stringify(b.input) },
      }));

      const assistantMsg: OpenAI.Chat.ChatCompletionAssistantMessageParam = {
        role: "assistant",
        ...(contentText !== null ? { content: contentText } : {}),
        ...(toolCalls.length > 0 ? { tool_calls: toolCalls } : {}),
      };
      result.push(assistantMsg);
    }
  }

  return result;
}

function toOAITools(tools: CanonicalTool[]): OpenAI.Chat.ChatCompletionTool[] {
  return tools.map((t) => ({
    type: "function" as const,
    function: { name: t.name, description: t.description, parameters: t.input_schema },
  }));
}

// ---------------------------------------------------------------------------
// Inbound translation: OpenAI wire format → canonical
// ---------------------------------------------------------------------------

function toStopReason(
  finishReason: string | null,
): "end_turn" | "tool_use" | "max_tokens" | "stop_sequence" {
  switch (finishReason) {
    case "tool_calls":    return "tool_use";
    case "length":        return "max_tokens";
    case "content_filter": return "stop_sequence";
    default:              return "end_turn"; // "stop" and null
  }
}

/**
 * Convert an OpenAI assistant response message to canonical ContentBlock[].
 *
 * We cast `message` as `unknown` first then use type guards because the OpenAI
 * SDK's `ChatCompletionMessage` discriminated union is not narrowed to the
 * `tool_calls` variant until TypeScript can see the finish_reason — we handle
 * both text and tool_calls regardless of finish_reason to support mixed responses.
 */
function toCanonicalContent(message: OpenAI.Chat.ChatCompletionMessage): ContentBlock[] {
  const blocks: ContentBlock[] = [];

  if (typeof message.content === "string" && message.content.length > 0) {
    blocks.push({ type: "text", text: message.content });
  }

  // `tool_calls` is present when the model returns function calls.
  // The OpenAI SDK types tool_calls as optional on ChatCompletionMessage.
  const toolCalls = (message as { tool_calls?: OpenAI.Chat.ChatCompletionMessageToolCall[] }).tool_calls;
  if (Array.isArray(toolCalls)) {
    for (const tc of toolCalls) {
      blocks.push({
        type: "tool_use",
        id: tc.id,
        name: tc.function.name,
        // JSON.parse is safe: OpenAI guarantees `arguments` is a valid JSON string.
        input: JSON.parse(tc.function.arguments) as Record<string, unknown>,
      });
    }
  }

  return blocks;
}

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

export function createOpenAIProvider(opts: OpenAIProviderOptions = {}): Provider {
  // Allow injecting a pre-built client for unit tests; otherwise construct one.
  const client: OpenAI = opts._client ?? new OpenAI({
    apiKey: opts.apiKey,   // undefined → OpenAI constructor reads OPENAI_API_KEY
    ...(opts.baseURL ? { baseURL: opts.baseURL } : {}),
  });
  const defaultModel = opts.defaultModel ?? DEFAULT_MODEL;

  return {
    name: "openai",

    async turn(request: ProviderTurnRequest): Promise<ProviderTurnResponse> {
      const model = request.model ?? defaultModel;
      const maxTokens = request.maxTokens ?? DEFAULT_MAX_TOKENS;
      const oaiMessages = toOAIMessages(request.systemPrompt, request.messages);
      const tools = toOAITools(request.tools);

      const params: OpenAI.Chat.ChatCompletionCreateParamsNonStreaming = {
        model,
        max_tokens: maxTokens,
        messages: oaiMessages,
        // Omit tools entirely when empty — some endpoints reject empty arrays.
        ...(tools.length > 0 ? { tools } : {}),
      };

      const response = await client.chat.completions.create(params);

      // We always use the first choice (n=1 default).
      const choice = response.choices[0];
      if (!choice) throw new Error("OpenAI returned no choices");

      const content = toCanonicalContent(choice.message);
      const message: Message = { role: "assistant", content };
      const stopReason = toStopReason(choice.finish_reason ?? null);

      return {
        message,
        stopReason,
        usage: response.usage
          ? { inputTokens: response.usage.prompt_tokens, outputTokens: response.usage.completion_tokens }
          : undefined,
      };
    },
  };
}
