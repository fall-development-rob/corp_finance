When a tool call or subagent dispatch returns empty content, the agent must
acknowledge the data gap explicitly rather than fabricating a response.

Steps when an empty result is received:

1. **Classify the emptiness**: distinguish between "no data exists for this
   query" (legitimate empty) and "the tool call failed or timed out" (error).
2. **Do not fill in values**: never substitute zero, N/A, or a prior period's
   value for a missing current value without flagging the substitution.
3. **Return a structured gap acknowledgement**:
   ```
   {"data_available": false, "reason": "<one-line explanation>",
    "suggested_follow_up": "<alternative query or data source>"}
   ```
4. **Request a follow-up**: in the structured response include a
   `suggested_follow_up` field naming the alternative tool or data source
   the caller should try (e.g. a different date range, a fallback vendor).
5. **Do not retry silently**: if the same tool has already returned empty
   for this query in this turn, do not call it a second time. Surface the
   gap and stop.

Empty results from authoritative sources (EDGAR, FRED, FMP) are real signals.
Treat them as data, not as errors to suppress.
