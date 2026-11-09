# Inspect render

`qsl-inspect` renders outcomes and typed inspection views as JSON, text or
Graphviz DOT (FR-298, TC-783). This note records the forms and how the render accounts for output bytes.

## Forms

- **JSON** is authoritative. An outcome is FR-286's document; a view carries
  `format: quire.inspect-view/v1`. Every field is an explicit serde field or a
  snake_case label. Nothing is a `Debug` string. A simulation graph names a
  state by its canonical key and a transition by its canonical encoding, both
  in hex. A package or node requirement is its occurrence, owner and extent
  roots. The node body is not part of the JSON. Known gap: the view JSON omits the requirement record's `requirements`, `result_bound`, `conversion_context`, `path_condition` and `references`, and no spec yet defines a view JSON schema.
- **Text** and **DOT** are display forms built from the typed value only. They
  never read rendered text back. DOT exists for trace and state-graph views;
  every other view refuses `RenderCause::UnsupportedForm { view, form }`.
  The `match` over `(form, view)` is exhaustive, so a new view kind cannot fall
  into an empty graph.

## Output-byte accounting

Output is produced as units: one line of text, one DOT statement, or the whole
JSON document. A single `Writer` is the only place bytes are appended. For each
unit it:

1. polls `Cancel`, before the unit is formatted;
2. formats the unit;
3. charges the unit's UTF-8 length against `render.output_bytes`, newline
   included, refusing with `LimitExceeded` when `used + n` exceeds the bound, before appending;
4. appends the unit.

A refused, limited or cancelled render returns no bytes. The output is held
once, as a `String`, and `Rendered` wraps it, so there is no second copy and no
UTF-8 re-check. `render.output_bytes` is separate from `inspect.max_view_entries`,
which bounds how many entries a view holds.
