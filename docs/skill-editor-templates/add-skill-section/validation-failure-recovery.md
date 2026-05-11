The agent MUST return valid, parseable JSON that conforms to its declared
output_schema. When a validation failure is detected the consumer receives an
opaque error; the original content is lost. Follow these rules to prevent that:

- **No code fences**: do not wrap JSON output in triple-backtick blocks.
  Return the raw JSON object as the entire response body.
- **No markdown wrapper**: do not preface the JSON with prose such as
  "Here is the result:" — the response must start with `{`.
- **Strip before returning**: if the computation pipeline yields a string
  that includes a code fence or backtick delimiter, strip those characters
  before inserting the value into the output object.
- **Validate required fields**: before returning, confirm every `required`
  field listed in the output_schema is present and non-null.
- **Type coercion is not assumed**: return `"0.05"` not `0.05` for string
  fields; return `42` not `"42"` for integer fields.

If the response cannot be made to conform (e.g. a tool returned an error),
return a well-formed error envelope:
`{"error": true, "message": "<reason>", "cluster_id": "<id>"}`.
