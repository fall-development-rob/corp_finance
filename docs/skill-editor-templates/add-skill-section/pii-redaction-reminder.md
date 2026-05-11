Structured output must never contain personally identifiable information (PII).
Redact PII fields before returning any response.

PII patterns that must be redacted:

- **SSN / TIN**: any string matching `\d{3}-\d{2}-\d{4}` or `\d{9}`.
  Replace with `"[REDACTED-SSN]"`.
- **Account numbers**: strings of 8-17 consecutive digits.
  Replace with `"[REDACTED-ACCT]"`.
- **IBAN**: strings matching `[A-Z]{2}\d{2}[A-Z0-9]{1,30}`.
  Replace with `"[REDACTED-IBAN]"`.
- **Full name + DOB combinations**: when both a personal name and a date of
  birth appear in the same output object, redact the DOB field.
- **Email addresses** in output fields not explicitly typed as email contact:
  Replace with `"[REDACTED-EMAIL]"`.

Redaction applies to all fields, including free-text commentary and notes.
Partial redaction (masking only middle digits) is not sufficient — replace
the entire value with the canonical placeholder.

If a PII token is discovered in a tool response being relayed to the output,
redact before insertion. Surface a `"pii_redacted": true` flag at the top
level of the output object so audit tooling can detect the redaction event.
