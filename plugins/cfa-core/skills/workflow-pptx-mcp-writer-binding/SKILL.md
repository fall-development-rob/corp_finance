---
name: "workflow-pptx-mcp-writer-binding"
description: |
  WHAT: Binding layer that maps a finalised markdown deck (authored per `workflow-pptx-deck-structure`) to a `SlideDeckSpec` JSON object and calls `office_pptx_write` to produce a binary `.pptx` terminal deliverable; covers the four supported slide kinds (title, section, content, table), v1 scope limitations, and result-struct handling.
  WHEN: Invoke after a deck has been fully authored and reviewed in markdown format and must be converted to a binary `.pptx` file for client delivery, audit logging, or downstream file use.
---

# Slide Deck — MCP Writer Binding (office_pptx_write)

## What this skill covers

The mechanical binding from a finalised markdown deck to a `SlideDeckSpec` JSON object and the `office_pptx_write` MCP tool call. Covers the four supported slide kinds, the mapping rules from markdown patterns to Slide objects, v1 scope limitations, and correct handling of the `WriteDeckResult` return value.

## The office_pptx_write tool

`office_pptx_write` accepts a `SlideDeckSpec` and returns:

```typescript
WriteDeckResult {
  output_path: string,   // absolute path to the written .pptx file
  bytes_written: number, // file size in bytes
  sha256: string,        // deterministic hash — identical spec → same hash on every run
  slide_count: number    // number of slides written
}
```

The `WriteDeckResult` is the system-of-record handle. Do not read the file back into the agent after writing — the result struct is sufficient for audit logging and downstream reference.

## Supported slide kinds (v1)

| Kind | When to use | Required fields |
|------|------------|----------------|
| `title` | Deck title slide (first slide only) | `title`, `subtitle` |
| `section` | Section divider with no body content | `heading` |
| `content` | H2 title + bullet list | `title`, `bullets` (array of strings) |
| `table` | H2 title + tabular data | `title`, `headers` (array), `rows` (array of arrays) |

v1 does NOT support: images, charts, embedded figures, transitions, animations, speaker notes, custom themes, hyperlinks, merged table cells. Mark any slide requiring these features for manual post-processing in PowerPoint.

## Markdown-to-SlideDeckSpec mapping rules

| Markdown pattern | Slide kind | Mapping |
|-----------------|-----------|---------|
| H1 at top of slide + bold subtitle, no bullets | `title` | H1 → `title`; bold text immediately below → `subtitle` |
| H2 with no content body (section divider slide) | `section` | H2 → `heading` |
| H2 title + bullet list (`-` or `*`) | `content` | H2 → `title`; bullet items → `bullets` array |
| H2 title + markdown table | `table` | H2 → `title`; header row → `headers`; data rows → `rows` |

Slides with mixed content (bullets + table, or chart placeholder) cannot be auto-mapped and must be rendered as `content` with a note flagging the unsupported element for manual addition.

## SlideDeckSpec JSON structure

```json
{
  "spec": {
    "slides": [
      { "kind": "title", "title": "...", "subtitle": "..." },
      { "kind": "section", "heading": "..." },
      { "kind": "content", "title": "...", "bullets": ["...", "...", "..."] },
      { "kind": "table", "title": "...", "headers": ["Col1", "Col2", "Col3"], "rows": [["A", "B", "C"], ["D", "E", "F"]] }
    ],
    "properties": {
      "title": "[Deck title for metadata]",
      "author": "CFA Agent"
    }
  },
  "output_path": "/tmp/[filename].pptx"
}
```

## Mapping workflow

### Step 1 — Parse the markdown deck

Split the markdown file on `---` slide boundaries. Each segment is one slide. Collect them in order.

### Step 2 — Classify each slide segment

For each segment, identify the pattern:
- First line H1 + bold line → `title`
- H2 only (no body text) → `section`
- H2 + bullet list → `content`
- H2 + markdown table → `table`
- H2 + mixed or chart placeholder → `content` (with manual-flag note)

### Step 3 — Extract slide fields

For each slide, extract the required fields per the kind:
- `title`/`section`: extract the heading text (strip `#` and `##` prefixes).
- `content`: extract the heading text and each bullet item (strip `-` or `*` prefix and leading space).
- `table`: extract the heading text, the header row as an array of strings, and each data row as an array of strings. Strip markdown formatting (`**bold**`, `*italic*`) from table cells before passing to the spec.

### Step 4 — Build the SlideDeckSpec

Assemble the slides array in order. Set `properties.title` to the title slide's `title` field. Set `properties.author` to "CFA Agent".

Set `output_path` to the target file path (e.g., `/tmp/[project]_deck_[YYYYMMDD].pptx`).

### Step 5 — Call office_pptx_write

Pass the complete SlideDeckSpec to `office_pptx_write`. The tool returns a `WriteDeckResult`.

### Step 6 — Handle the result

Record in the agent output:
- `output_path` — the path to the written file.
- `sha256` — for audit log.
- `slide_count` — verify this matches the expected number of slides.
- `bytes_written` — sanity check (a 20-slide deck should be >50KB).

Do not read the file back. The WriteDeckResult is sufficient. If `slide_count` does not match expected: re-inspect the mapping for missed `---` boundaries or mis-classified slides.

## Complete example

```json
{
  "spec": {
    "slides": [
      {
        "kind": "title",
        "title": "Project Falcon",
        "subtitle": "IC Presentation — May 2026"
      },
      {
        "kind": "section",
        "heading": "Investment Thesis"
      },
      {
        "kind": "content",
        "title": "Key Highlights",
        "bullets": [
          "$1.2B revenue, 22% EBITDA margin",
          "Market leader with 35% share of $4B addressable market",
          "Three near-term catalysts: regulatory approval, product launch, M&A"
        ]
      },
      {
        "kind": "table",
        "title": "Trading Comparables",
        "headers": ["Company", "EV ($M)", "EV/EBITDA"],
        "rows": [
          ["ACME Corp", "2,850", "9.2x"],
          ["Beta Inc", "1,380", "7.8x"],
          ["Median", "2,115", "8.5x"],
          ["Target Co", "1,950", "8.0x"]
        ]
      }
    ],
    "properties": {
      "title": "Project Falcon IC Deck",
      "author": "CFA Agent"
    }
  },
  "output_path": "/tmp/falcon_ic_20260510.pptx"
}
```

## v1 limitations and workarounds

| Unsupported feature | Workaround |
|--------------------|-----------|
| Charts and graphs | Include the underlying data table as a `table` slide; note "chart to be added manually" |
| Images and logos | Leave a placeholder text in the `content` bullets; note "image to be inserted manually" |
| Speaker notes | Append notes as a final `content` slide titled "Presenter Notes — [Slide Name]" |
| Custom themes | The writer uses a default theme; apply branding manually in PowerPoint |
| Merged table cells | Split the merged concept into separate rows or use a `content` slide with structured bullets |

## Quality gates

- `slide_count` in WriteDeckResult matches the expected slide count from the markdown deck — mismatch triggers re-inspection.
- `output_path` is an absolute path and the directory exists before calling the tool.
- Markdown table cell values are stripped of `**bold**` and `*italic*` markers before inclusion in `rows`.
- The `title` slide must be the first element in the `slides` array.
- Every unsupported feature is flagged in the agent output with a specific manual-action note.
- The WriteDeckResult `sha256` is recorded for audit purposes — do not discard it.

## Related skills

- `workflow-pptx-deck-structure` — authors the markdown deck that this skill converts
- `workflow-pptx-valuation-layout` — valuation slides map to `content` and `table` kinds
- `workflow-pptx-comps-table` — comps and sensitivity slides map to `table` kind

## Routing

**Primary agent:** `cfa-private-markets-analyst` (pitch and IC decks)
**Secondary agent:** `cfa-equity-analyst` (research decks)
