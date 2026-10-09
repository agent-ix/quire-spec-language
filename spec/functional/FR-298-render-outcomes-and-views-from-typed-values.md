---
id: FR-298
title: "Render outcomes and views as JSON, text and DOT from typed values only"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-029
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-029
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-001
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-073
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-286
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-297
    type: depends_on
  - target: "ix://agent-ix/quire-specification/FR-300"
    type: depends_on
---
# FR-298: Render outcomes and views as JSON, text and DOT from typed values only

## Description

`render` turns an outcome or a view into bytes (ADR-029 IN-2, IN-3). It lives
in layer P, crate `qsl-inspect`. JSON is the authoritative form: each outcome
serializes as FR-286's document and each view with its own `format`
identity. Text and Graphviz DOT are rendered from the typed value only, so a
change to a text or DOT renderer leaves the JSON bytes equal. Renderers feed
nothing back into logic (ADR-011 FB-02, FB-09).

Text forms:

- A diagnostic reads `path:line:column: code: message`. Line and column are
  computed at render time from the source bytes the caller provides
  (FR-001).
- A counterexample renders as a table of parameter names and values.
- A trace renders as numbered positions with the loop marked.
- A `proved` record renders its certification, `certified`, `uncertified`
  or `trusted`, beside the verdict.
- Bulk content renders as a bounded descriptor (FR-073).

DOT is available for the state-graph and trace views.

## Inputs

An outcome or `View`; the output form (`json`, `text` or `dot`); the source
byte provision; limits and `&Cancel`.

## Outputs

`Staged<Rendered>`: the rendered bytes.

## Behavior

- The `render` operation shall produce JSON output by FR-286's serialization
  or the view's own serialization.
- The `render` operation shall produce text and DOT output from the typed
  outcome or view only.
- The `render` operation shall compute a diagnostic's line and column from
  the caller's source bytes at render time.
- If the caller provides no source bytes for a diagnostic's region, then
  `render` shall print the region's source digest and byte offsets in place
  of path, line and column.
- No logic stage shall take a rendered form as input.
- If `dot` is requested for a view other than state graph or trace, then
  `render` shall refuse naming the view and the form.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-298-AC-1 | The text rendering of FR-286-AC-2's refusal is one line `<path>:<line>:<column>: ill_typed: <message>`, whose line and column are those of the region's first byte in the provided source; without the source bytes it shows the source digest and byte offsets. | Test (TC-783) |
| FR-298-AC-2 | The text rendering of FR-281-AC-2's counterexample is a table with one row per parameter name and value; FR-283-AC-2's lasso renders as numbered positions with the loop start marked; FR-290-AC-1's record shows `proved` with `trusted` beside it; a 1 MiB text value renders as FR-073's bounded descriptor. | Test (TC-783) |
| FR-298-AC-3 | The DOT rendering of an FR-120 state-graph view has one node per retained state and one edge per retained transition, and `dot` for the package view refuses naming the view and the form. | Test (TC-783) |
| FR-298-AC-4 | With the text renderer swapped for a test renderer that uppercases every line, the JSON bytes of every AC-1 to AC-3 outcome and view are unchanged. | Test (TC-783) |

## Dependencies

- ADR-029 IN-2, IN-3: formats, diagnostics and counterexamples.
- ADR-011 FB-02, FB-09: renderers feed nothing back.
- [FR-001](FR-001-read-exact-source.md): line and column at render time.
- [FR-073](FR-073-implement-redacted-safe-diagnostic-rendering.md): bounded descriptors.
- [FR-286](FR-286-serialize-every-outcome-as-one-json-outcome-document.md): the outcome document.
- [FR-297](FR-297-inspect-packages-outcomes-and-traces-as-typed-views.md): the views.
- QSpec FR-300: the `render` operation.

## References

- QSL-390 (ARCH-50), QSL-393 (V1-A06): rendering.
- QSpec FR-300 (STD-141): the QSpec half.
