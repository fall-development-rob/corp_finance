/**
 * Gemini provider — wraps @google/generative-ai to implement the Provider
 * interface defined in types.ts.
 *
 * Translation rules follow ADR-033: Anthropic-flavored canonical shapes are
 * converted to Gemini GenerateContent format at the wire boundary.
 */
import { GoogleGenerativeAI } from "@google/generative-ai";
import type {
  Content,
  FunctionDeclaration,
  FunctionDeclarationsTool,
  Part,
} from "@google/generative-ai";
import type {
  CanonicalTool,
  ContentBlock,
  Message,
  Provider,
  ProviderTurnRequest,
  ProviderTurnResponse,
} from "../../types.js";

const DEFAULT_MODEL = "gemini-2.0-flash";
const DEFAULT_MAX_TOKENS = 4096;

export interface GeminiProviderOptions {
  apiKey?: string;
  defaultModel?: string;
}

// ---------------------------------------------------------------------------
// Exported helpers — pure functions, testable without the SDK
// ---------------------------------------------------------------------------

/**
 * Walk messages backward to find the tool_use block matching `toolUseId`.
 * Returns the tool name, or undefined if not found.
 */
export function findToolUseName(
  messages: Message[],
  toolUseId: string,
): string | undefined {
  for (let i = messages.length - 1; i >= 0; i--) {
    const msg = messages[i];
    if (!msg) continue;
    for (const block of msg.content) {
      if (block.type === "tool_use" && block.id === toolUseId) {
        return block.name;
      }
    }
  }
  return undefined;
}

export function toGeminiTools(tools: CanonicalTool[]): FunctionDeclarationsTool[] {
  const declarations: FunctionDeclaration[] = tools.map((t) => ({
    name: t.name,
    description: t.description,
    // FunctionDeclarationSchema needs SchemaType; input_schema is structurally
    // compatible as an object schema.
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    parameters: t.input_schema as any,
  }));
  return [{ functionDeclarations: declarations }];
}

export function toGeminiContents(messages: Message[]): Content[] {
  const contents: Content[] = [];
  for (const msg of messages) {
    const role = msg.role === "assistant" ? "model" : "user";
    const parts: Part[] = [];

    for (const block of msg.content) {
      if (block.type === "text") {
        parts.push({ text: block.text });
      } else if (block.type === "tool_use") {
        parts.push({
          functionCall: {
            name: block.name,
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
            args: block.input as any,
          },
        });
      } else if (block.type === "tool_result") {
        const name =
          findToolUseName(messages, block.tool_use_id) ?? block.tool_use_id;
        let response: object;
        try {
          const parsed: unknown = JSON.parse(block.content);
          response =
            typeof parsed === "object" && parsed !== null
              ? (parsed as object)
              : { output: block.content };
        } catch {
          response = { output: block.content };
        }
        parts.push({ functionResponse: { name, response } });
      }
    }

    if (parts.length > 0) {
      contents.push({ role, parts });
    }
  }
  return contents;
}

export function toCanonicalContent(parts: Part[]): ContentBlock[] {
  return parts.map((p): ContentBlock => {
    if ("text" in p && typeof p.text === "string") {
      return { type: "text", text: p.text };
    }
    if ("functionCall" in p && p.functionCall) {
      return {
        type: "tool_use",
        id: crypto.randomUUID(),
        name: p.functionCall.name,
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        input: (p.functionCall.args as Record<string, unknown>) ?? {},
      };
    }
    // Fallback: unknown part types become empty text blocks
    return { type: "text", text: "" };
  });
}

export function mapFinishReason(
  reason: string | undefined,
  hasToolUse: boolean,
): ProviderTurnResponse["stopReason"] {
  if (reason === "MAX_TOKENS") return "max_tokens";
  if (reason === "SAFETY") return "stop_sequence";
  if (hasToolUse) return "tool_use";
  return "end_turn";
}

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

export function createGeminiProvider(opts: GeminiProviderOptions = {}): Provider {
  const apiKey =
    opts.apiKey ??
    process.env["GEMINI_API_KEY"] ??
    process.env["GOOGLE_API_KEY"] ??
    "";
  const defaultModel = opts.defaultModel ?? DEFAULT_MODEL;
  const client = new GoogleGenerativeAI(apiKey);

  return {
    name: "gemini",

    async turn(request: ProviderTurnRequest): Promise<ProviderTurnResponse> {
      const modelName = request.model ?? defaultModel;
      const maxTokens = request.maxTokens ?? DEFAULT_MAX_TOKENS;

      const model = client.getGenerativeModel({
        model: modelName,
        systemInstruction: {
          role: "user",
          parts: [{ text: request.systemPrompt }],
        },
        ...(request.tools.length > 0
          ? { tools: toGeminiTools(request.tools) }
          : {}),
        generationConfig: { maxOutputTokens: maxTokens },
      });

      const contents = toGeminiContents(request.messages);
      const result = await model.generateContent({ contents });
      const response = result.response;

      const candidate = response.candidates?.[0];
      const rawParts: Part[] = candidate?.content?.parts ?? [];
      const content = toCanonicalContent(rawParts);

      const hasToolUse = content.some((b) => b.type === "tool_use");
      const finishReason = candidate?.finishReason as string | undefined;
      const stopReason = mapFinishReason(finishReason, hasToolUse);

      const message: Message = { role: "assistant", content };

      return {
        message,
        stopReason,
        usage: {
          inputTokens: response.usageMetadata?.promptTokenCount ?? 0,
          outputTokens: response.usageMetadata?.candidatesTokenCount ?? 0,
        },
      };
    },
  };
}
