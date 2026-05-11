String fields in the output_schema that represent identifiers, dates, or numeric
values MUST carry explicit `pattern:` regex constraints. Unconstrained string
fields allow the agent to return values that downstream validators will reject.

Canonical patterns to apply by field class:

- **Identifier fields** (`*_id`, ticker, CUSIP, ISIN, slug):
  `pattern: "^[A-Za-z0-9._-]+$"` + `maxLength: 64`
- **Date fields** (`*_date`, `as_of`, `period_end`):
  `pattern: "^[0-9-]{1,10}$"` + `maxLength: 10`
- **Numeric-as-string fields** (price, ratio, percentage, yield):
  `pattern: "^-?[0-9]+(\\.[0-9]+)?$"` + `maxLength: 32`
- **Rating fields** (credit_rating, grade): use the canonical `enum-rating-pattern`
  from `docs/skill-editor-templates/tighten-output-schema/enum-rating-pattern.yaml`.

Fields that are genuinely free-text (notes, commentary, executive_summary) are
exempt from pattern constraints but must carry `maxLength: 2000`.

When adding constraints, do not alter existing passing constraints — append only.
