# Skill-Editor Templates

Templates used by the `cfa-harness skill-editor` to inject structured content
into agent SKILL.md files and YAML manifests.

## Directory layout

```
docs/skill-editor-templates/
  add-skill-section/          # Markdown bodies inserted into SKILL.md files
    anti-injection-reminder.md
    tool-discipline-checklist.md
  tighten-output-schema/      # YAML fragments for schema constraint remediations
    id-field-pattern.yaml
  README.md                   # This file
```

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

## Stability guarantee

Template files must be byte-stable across commits. The emitter guarantees that
the same `OutlierReport` produces byte-identical remediation files, which in turn
reference these templates by ID. Changing a template content is a breaking change
to the idempotency contract — create a new template ID instead.
