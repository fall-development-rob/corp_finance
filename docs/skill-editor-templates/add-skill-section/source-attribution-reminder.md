Every numeric claim in the output must be traceable to a named data source.
Fabricating or approximating financial figures without a sourced tool result
is a material accuracy failure.

Source attribution rules:

1. **Name the vendor**: for each key metric returned, include a `data_source`
   or `source` field indicating the originating system.
   Accepted values: `"FMP"`, `"EDGAR"`, `"FRED"`, `"FIGI"`, `"Yahoo Finance"`,
   `"World Bank"`, `"LSEG"`, `"FactSet"`, `"S&P Global"`, `"Moody's"`,
   `"PitchBook"`, `"Morningstar"`, `"manual"`.
2. **Include the retrieval timestamp** when the vendor provides one.
   Use the `as_of` field in ISO-8601 format (`YYYY-MM-DD`).
3. **Flag stale data**: if the source timestamp is more than 90 days before the
   analysis date, set `data_freshness: "stale"` on the affected field.
4. **No blended figures without disclosure**: if a metric is derived from
   multiple sources, list all sources in a `data_sources` array and note the
   derivation method in `derivation_note`.
5. **Reject tool errors silently**: if a tool returned an error or empty result,
   do not substitute an estimated value. Return `null` for the field and set
   `data_source: "unavailable"`.
