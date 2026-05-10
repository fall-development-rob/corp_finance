/**
 * Anthropic provider — wraps @anthropic-ai/sdk to implement the Provider
 * interface defined in types.ts.
 */
import Anthropic from "@anthropic-ai/sdk";
import type {
  CanonicalTool,
  ContentBlock,
  Message,
  Provider,
  ProviderTurnRequest,
  ProviderTurnResponse,
} from "../../types.js";

const DEFAULT_MODEL = "claude-opus-4-5";
const DEFAULT_MAX_TOKENS = 4096;

export interface AnthropicProviderOptions {
  apiKey?: string;
  defaultModel?: string;
}

/**
 * Maps our canonical ContentBlock[] to the Anthropic SDK's MessageParam
 * content format. The shapes are compatible but we must satisfy the SDK types.
 *
 * We use `as Parameters<...>[0]` casts here because the Anthropic SDK's
 * discriminated union types for content blocks are structurally identical to
 * our canonical types, but TypeScript cannot verify structural equality
 * across package boundaries without explicit casts.
 */
function toSdkMessages(
  messages: Message[],
): Anthropic.Messages.MessageParam[] {
  return messages.map((m) => ({
    role: m.role,
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    content: m.content as any,
  }));
}

function toSdkTools(tools: CanonicalTool[]): Anthropic.Messages.Tool[] {
  return tools.map((t) => ({
    name: t.name,
    description: t.description,
    input_schema: t.input_schema,
  }));
}

function toCanonicalContent(
  blocks: Anthropic.Messages.ContentBlock[],
): ContentBlock[] {
  return blocks.map((b): ContentBlock => {
    if (b.type === "text") return { type: "text", text: b.text };
    if (b.type === "tool_use") {
      return {
        type: "tool_use",
        id: b.id,
        name: b.name,
        input: b.input as Record<string, unknown>,
      };
    }
    // Exhaustive guard — the SDK may add block types in minor versions.
    throw new Error(`Unexpected content block type from Anthropic SDK: ${(b as { type: string }).type}`);
  });
}

export function createAnthropicProvider(
  opts: AnthropicProviderOptions = {},
): Provider {
  const client = new Anthropic({ apiKey: opts.apiKey ?? process.env["ANTHROPIC_API_KEY"] });
  const defaultModel = opts.defaultModel ?? DEFAULT_MODEL;

  return {
    name: "anthropic",

    async turn(request: ProviderTurnRequest): Promise<ProviderTurnResponse> {
      const model = request.model ?? defaultModel;
      const maxTokens = request.maxTokens ?? DEFAULT_MAX_TOKENS;

      const response = await client.messages.create({
        model,
        max_tokens: maxTokens,
        system: request.systemPrompt,
        messages: toSdkMessages(request.messages),
        tools: toSdkTools(request.tools),
      });

      const content = toCanonicalContent(response.content);
      const message: Message = { role: "assistant", content };

      const stopReason = (response.stop_reason ?? "end_turn") as
        | "end_turn"
        | "tool_use"
        | "max_tokens"
        | "stop_sequence";

      return {
        message,
        stopReason,
        usage: {
          inputTokens: response.usage.input_tokens,
          outputTokens: response.usage.output_tokens,
        },
      };
    },
  };
}
