/**
 * Replay helper — Phase 31 Wave 4.
 *
 * Produces the canonical shape to resume a prior session. The caller passes
 * the returned `prompt` and `messages` into a new `dispatch()` call.
 *
 * NOTE: `dispatch()` does not yet accept a seed message array directly.
 * `buildReplayPrompt` returns the correct shape so callers can wire it up
 * once the dispatch signature is extended in a future wave.
 *
 * Future enhancement: extend `DispatchOptions` with an optional
 * `seedMessages?: Message[]` field so replay starts mid-conversation.
 */
import type { Message, SessionState } from "../types.js";

export interface ReplayShape {
  prompt: string;
  messages: Message[];
}

/**
 * Given a completed (or failed) `SessionState`, return the original prompt
 * and the full messages array so a caller can reconstruct the conversation
 * for replay or inspection.
 */
export function buildReplayPrompt(state: SessionState): ReplayShape {
  return {
    prompt: state.prompt,
    messages: state.messages,
  };
}
