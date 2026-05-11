Regulatory and ESG claims must cite the specific legislative or regulatory source
that supports the statement. Generic phrases like "as required by regulation" or
"under applicable law" are not acceptable in structured output.

Citation format requirements:

- **EU regulations**: cite by level-1 instrument and article.
  Example: `"AIFMD Art. 23(4)"` not `"AIFMD disclosure rules"`.
- **US regulations**: cite by CFR title, part, and section.
  Example: `"17 CFR § 275.206(4)-7"` not `"SEC adviser requirements"`.
- **IFRS / GAAP standards**: cite by standard number and paragraph.
  Example: `"IFRS 9 para. 5.5.3"` not `"expected credit loss rules"`.
- **ESG frameworks**: cite by framework version and disclosure number.
  Example: `"TCFD Recommendations 2017 — Metrics C2"` not `"TCFD metrics"`.
- **Basel / BCBS**: cite by document number and paragraph.
  Example: `"Basel III CRE 20.7"` not `"Pillar 1 capital requirements"`.

Place citations in a `regulatory_basis` array field in the output object.
Each element must be a string of the form `"<instrument> <article/section>"`.
If no authoritative citation can be provided, set `regulatory_basis: []` and
flag `citation_available: false` rather than fabricating a reference.
