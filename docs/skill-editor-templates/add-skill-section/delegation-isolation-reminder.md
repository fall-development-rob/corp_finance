Each subagent is contracted to return exactly the fields declared in its own
output_schema — nothing more. The chief analyst must never delegate a task when
the expected output could not possibly fit that schema.

Before issuing a delegation:

1. **Match the ask to the schema**: confirm the subagent's declared output_schema
   contains every field the chief needs. If a required field is absent, handle the
   computation locally or decompose into a different subagent pair.
2. **No schema bridging at the chief**: the chief must not post-process a subagent
   response to derive fields the subagent was never contracted to produce. That
   derivation is silent fabrication.
3. **Single responsibility**: each delegation call targets exactly one subagent.
   The chief must not send a single request to two subagents and merge the results
   under the assumption both will conform to the same schema.
4. **Escalate schema gaps**: if no available subagent has a schema that covers the
   required output, surface a structured gap report rather than approximate with
   a mismatched subagent.

A delegation mismatch means the chief issued a request the subagent's schema
cannot satisfy. Fix the delegation target, not the subagent schema.
