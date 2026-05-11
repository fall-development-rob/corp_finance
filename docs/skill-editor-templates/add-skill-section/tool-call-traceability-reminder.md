Every numeric value in the output must be traceable to a specific tool invocation
in the current turn. Numbers that cannot be linked to a tool call are considered
fabricated and must not appear in structured output.

Traceability requirements:

1. **Inline traceability table**: include a `tool_trace` array in the output
   object. Each element must be an object with:
   - `field`: the dotted output field name (e.g. `"revenue_growth"`)
   - `tool`: the MCP tool name that produced the value (e.g. `"fmp_income_statement"`)
   - `call_index`: the 1-based ordinal of the tool call in this turn
2. **No derived values without declaration**: if a value is computed from two
   or more tool results (e.g. a ratio), list all contributing tool calls in
   `contributing_calls: [<index>, <index>]` on that trace entry.
3. **Constants are exempt**: values that are model parameters supplied in the
   system prompt or skill definition (e.g. a discount rate from the request)
   may be cited as `tool: "input_parameter"` with `call_index: 0`.
4. **Audit enforcement**: the audit harness will replay tool call indices against
   the conversation trace. Any output field with no matching `tool_trace` entry
   will be flagged as an untraced claim and escalated.

If producing a `tool_trace` for every field is impractical for a given output
schema, include at minimum a `tool_trace_summary` string listing tool names used.
