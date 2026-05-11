# Skill-Editor Templates

Templates used by the `cfa-harness skill-editor` to inject structured content
into agent SKILL.md files and YAML manifests.

## Directory layout

```
docs/skill-editor-templates/
  add-skill-section/          # Markdown bodies inserted into SKILL.md files
  tighten-output-schema/      # YAML fragments for schema constraint remediations
  adjust-tool-allowlist/      # YAML fragments for tool allowlist remediations
  README.md                   # This file
```

## Template index

### add-skill-section/

| Template ID                        | Use-case                                                          |
| ---------------------------------- | ----------------------------------------------------------------- |
| `anti-injection-reminder`          | Prompt-injection guard — sanitise tool outputs before re-use      |
| `tool-discipline-checklist`        | Tool-call hygiene — necessity, dedup, batching, exit condition    |
| `output-schema-regex-warning`      | Missing regex constraints on ID/date/numeric string fields        |
| `validation-failure-recovery`      | JSON output rules — no code fences, required fields, error envelope |
| `delegation-isolation-reminder`    | Chief-to-subagent contract — match asks to declared output_schema |
| `empty-result-handling`            | Data-gap handling — acknowledge empty results, no fabrication     |
| `pii-redaction-reminder`           | Redact SSN, account numbers, IBAN, email before returning output  |
| `compliance-citation-reminder`     | Cite specific regulation articles, not generic phrases            |
| `source-attribution-reminder`      | Attribute every numeric claim to a named data source + vendor     |
| `tool-call-traceability-reminder`  | Inline tool_trace table mapping output fields to tool invocations |

### tighten-output-schema/

| Template ID                  | Use-case                                                        |
| ---------------------------- | --------------------------------------------------------------- |
| `id-field-pattern`           | Identifier fields: `^[A-Z0-9._\-]{1,20}$`, maxLength 20        |
| `date-field-pattern`         | Date fields: `^[0-9-]{1,10}$`, maxLength 10                     |
| `numeric-string-pattern`     | Decimal-as-string fields: `^-?[0-9]+(\.[0-9]+)?$`, maxLength 32 |
| `free-text-cap`              | Free-text fields: maxLength 2000, no pattern                    |
| `array-bounds`               | Array fields: maxItems 500                                      |
| `enum-rating-pattern`        | Credit rating enum: full 23-value S&P/Fitch scale + NR          |
| `enum-recommendation-pattern`| Analyst recommendation enum: 6-value strong_buy…no_recommendation |

### adjust-tool-allowlist/

| Template ID                  | Use-case                                                        |
| ---------------------------- | --------------------------------------------------------------- |
| `block-write-tools`          | Block xlsx/pptx/docx write tools for reader subagents           |
| `block-recursive-handoff`    | Block initiate_handoff for depth-1 specialists                  |
| `block-recall-from-depth-1`  | Block recall_similar/recall_by_graph for depth-1 specialists    |

## How templates are selected

The emitter (`packages/harness/src/reasoning/remediation.ts`) maps each
`OutlierCluster.cluster_type` to a template via a static lookup table:

| cluster_type         | recommended_action      | template_id                  |
| -------------------- | ----------------------- | ---------------------------- |
| validation-failure   | tighten-output-schema   | (constraint inline)          |
| delegation-mismatch  | add-skill-section       | anti-injection-reminder      |
| tool-thrashing       | adjust-tool-allowlist   | (no template, direct action) |
| novel                | add-skill-section       | anti-injection-reminder      |

## Authoring new templates

### add-skill-section templates

- File name must match `section_body_template_id` in the remediation YAML.
- Content is plain Markdown without a heading (the heading is injected by the apply step).
- Keep templates under 30 lines and byte-stable (no timestamps or dynamic content).
- Idempotency: the apply step checks whether the exact template body already
  appears in the SKILL.md before inserting.

### tighten-output-schema templates

- YAML files with an `add_constraint` key.
- Reference from a remediation's `change.add_constraint` by copying the block.
- These are reference examples, not directly consumed by the apply step.

### adjust-tool-allowlist templates

- YAML files with a `block_tools` key containing a list of tool names.
- Merge the list into the target agent manifest's `block_tools` array.

## Stability guarantee

Template files must be byte-stable across commits. The emitter guarantees that
the same `OutlierReport` produces byte-identical remediation files, which in turn
reference these templates by ID. Changing a template content is a breaking change
to the idempotency contract — create a new template ID instead.
