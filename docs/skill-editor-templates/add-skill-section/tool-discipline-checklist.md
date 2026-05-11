Before invoking any tool, verify:

1. **Necessity**: Is this tool call required to answer the user's request, or can
   the answer be derived from information already in context?
2. **Deduplication**: Has this exact tool been called with the same arguments in
   this turn? If so, use the cached result rather than calling again.
3. **Batch first**: Can multiple data-retrieval needs be satisfied by one batched
   call (e.g. `fmp_batch_quote`) rather than N sequential single-ticker calls?
4. **Scope**: Is the tool fetching only the fields needed, or is it over-fetching
   a large payload that will be mostly discarded?
5. **Exit condition**: After each tool result, re-evaluate whether the original
   question can now be answered. Stop calling tools as soon as sufficient
   information is available.

Exceeding the p95 tool-call count is a signal that one or more of the above
checks failed. Review the dispatch trajectory and apply the checklist.
