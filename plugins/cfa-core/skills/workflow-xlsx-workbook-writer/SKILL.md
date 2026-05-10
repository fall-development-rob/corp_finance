---
name: workflow-xlsx-workbook-writer
description: |
  WHAT: Binary `.xlsx` file generation via the `office_xlsx_write` MCP tool — mapping markdown-tabular sections to a WorkbookSpec JSON, cell value types, formula injection, defined names, frozen panes, and column widths. The output is a terminal deliverable; do not read the .xlsx file back into the system.
  WHEN: Invoke when a recipient requests an actual `.xlsx` file (not markdown); when producing a multi-sheet workbook as a binary download; when named ranges, frozen panes, or formula injection are required in the final file.
---

# Excel Workbook Writer

## What this skill covers

How to map a markdown-tabular deliverable (produced per `workflow-xlsx-tabular-conventions`) to a `WorkbookSpec` JSON and invoke `office_xlsx_write` to produce a binary `.xlsx` file. This is the terminal step — it converts a markdown-structured output into a real Excel file.

**Prerequisites**: the markdown deliverable must already conform to `workflow-xlsx-tabular-conventions`, `workflow-xlsx-formula-cells`, and `workflow-xlsx-named-ranges` before this tool is invoked.

## When to use `office_xlsx_write`

Use the MCP tool when:
- The recipient explicitly requests a `.xlsx` file (not a markdown table)
- The deliverable has multiple sheets and cross-sheet defined names
- Formula injection (`=SUM(...)`, `=B5*B6`) is required in the final file
- Frozen panes or specific column widths are needed for usability

Default to markdown-tabular when the recipient has not requested a binary file — markdown is universal, requires no tool invocation, and can be reviewed in the conversation.

## Mapping markdown to WorkbookSpec

### One markdown H2 section → one sheet

Each `## Sheet Name` section in the markdown becomes one entry in `spec.sheets`. The sheet `name` field matches the H2 text exactly.

### Column headers → `headers` array

The markdown header row (first row of each table) maps to the `headers` array. Include the units row as the first data row if units are needed (they function as a data row, not a sheet header, in Excel).

### Data rows → `rows` array

Each markdown table row maps to one array of cell objects in `rows`. Use these cell types:

| Markdown content | Cell kind | Example |
|-----------------|----------|---------|
| Text / label | `text` | `{"kind": "text", "value": "Revenue"}` |
| Financial number ($M) | `decimal` | `{"kind": "decimal", "value": "125.4"}` |
| Percentage | `decimal` | `{"kind": "decimal", "value": "0.254"}` (store as decimal; format in Excel) |
| Multiple (x) | `decimal` | `{"kind": "decimal", "value": "12.5"}` |
| Blank cell | `empty` | `{"kind": "empty"}` |
| Boolean | `bool` | `{"kind": "bool", "value": true}` |
| Date | `datetime` | `{"kind": "datetime", "value": "2026-03-31T00:00:00Z"}` |

**Use `decimal` for all financial values** to preserve the canonical Decimal precision from corp-finance-mcp tools.

### Formula injection → `formulas` array

To inject actual Excel formulas at specific cell addresses (after the data is laid out):

```json
"formulas": [
  {"row": 5, "col": 2, "formula": "=SUM(C3:C4)", "cached_result": 750.0},
  {"row": 8, "col": 2, "formula": "=C5/C6", "cached_result": 0.254}
]
```

Row and column are 0-indexed from the top-left of the sheet (not the table body). Account for header rows and units rows when computing row indices.

### Named ranges → `defined_names` array

Each heading-anchor substitute from `workflow-xlsx-named-ranges` maps to a `defined_names` entry:

```json
"defined_names": [
  {"name": "WACC", "range": "Assumptions!$B$5"},
  {"name": "TerminalGrowthRate", "range": "Assumptions!$B$6"}
]
```

### Frozen panes and column widths

```json
"frozen_panes": {"row": 2, "col": 0},
"column_widths": [20, 12, 12, 12, 18, 20]
```

- `frozen_panes`: freeze rows above and columns left of the specified 0-indexed cell (typically `row: 2` to freeze header + units rows)
- `column_widths`: per-column width in Excel units; 12-20 is typical; label columns (20-25), data columns (12-15)

## Tool invocation

```
Tool: office_xlsx_write
Input: {
  "spec": {
    "sheets": [ ... ],
    "defined_names": [ ... ],
    "properties": {
      "title": "Deliverable Title",
      "author": "cfa-chief-analyst",
      "company": "CFA Agent",
      "subject": "Analysis type"
    }
  },
  "output_path": "/tmp/deliverable_name.xlsx"
}
```

`output_path` must be a temp or artifact directory (not root or repo). Use `/tmp/` or a configured output directory.

## Tool output

```json
{
  "output_path": "/tmp/deliverable_name.xlsx",
  "bytes_written": 45678,
  "sha256": "a1b2c3...",
  "sheet_count": 4
}
```

The `sha256` and `bytes_written` are the system-of-record receipt. **Do not read the `.xlsx` file back into the system.** Surface the output path and hash as the deliverable receipt to the user.

## Quality checklist

- [ ] All numeric columns use `decimal` cell type (not `number`)
- [ ] Headers row matches the markdown H2 section exactly
- [ ] Units row present as the first data row in the `rows` array
- [ ] Source column populated on derived rows
- [ ] Negatives stored as negative decimal values (Excel renders as negative; use conditional formatting for parentheses if needed)
- [ ] `column_widths` set to appropriate values (label columns 20-25, data columns 12-15)
- [ ] `frozen_panes` set to freeze header + units rows (typically `row: 2`)
- [ ] `defined_names` populated for every heading anchor in the markdown delivery
- [ ] `properties.title`, `properties.author`, and `properties.subject` set
- [ ] Output path is `/tmp/` or a configured artifact directory — never root or repo

## Relationship to markdown-tabular form

The markdown-tabular form and the `.xlsx` binary are equivalent deliverables. The markdown form is always produced first (it is the human-reviewable form). The `.xlsx` form is produced on request by mapping the markdown to a `WorkbookSpec`. They must contain the same numeric values — if they differ, the corp-finance-mcp tool output is canonical.

## Related skills

- `workflow-xlsx-tabular-conventions` — prerequisite: layout standard that must be applied before invoking this tool
- `workflow-xlsx-formula-cells` — formula column text that informs the `formulas` array entries
- `workflow-xlsx-named-ranges` — heading anchors that map to `defined_names` entries
