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
  - target: ix://agent-ix/quire-spec-language/FR-095
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-255
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-276
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-277
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-285
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-286
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-297
    type: depends_on
  - target: "ix://agent-ix/quire-specification/FR-300"
    type: depends_on
  - target: "ix://agent-ix/quire-specification/FR-272"
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

Text and DOT are display forms. Their exact bytes are not normative: the
structure this requirement states is, and equal inputs render equal bytes.
JSON bytes are normative, through FR-286 and each view's serialization.

### The source provision

`RenderSources` maps a source digest to the admitted
[FR-001](FR-001-read-exact-source.md) `Source` that holds those bytes. A
`SourceRegion` names its source by digest (FR-095), so `render` looks up the
region's digest in the map. The text diagnostic's `path` is that `Source`'s
path, and its line and column are FR-001's line and scalar-column
coordinates of the region's first byte. An admitted `Source` carries its own
digest, so a provision whose bytes disagree with its key cannot be built.
`RenderSources` may be empty; an empty map gives every diagnostic the
fallback form.

### The trace DOT shape

The DOT rendering of a trace view is one directed graph with one node per
position, in position order, labelled with the position's zero-based index
and its state. Each position has one edge to the next position. A lasso has
one more edge, from its last position back to its loop start; a finite
trace has no edge out of its last position. The deciding position's node,
when the view has one, is marked as deciding. The node and edge sets are
normative; attribute spelling, layout and whitespace are not.

### Limits and cancellation

`RenderLimits` has one field, `output_bytes`, the FR-255 setting
`render.output_bytes` (QSpec FR-461 rule 2's output-byte budget), default
16777216 bytes. `render` writes its output as units: one line of text, or
one DOT statement. Before it writes a unit, `render` charges the unit's
UTF-8 byte length against `output_bytes` and polls `&Cancel` (FR-276). The
budget counts every byte `render` would return, newlines included. A JSON
rendering is one unit.

### The unsupported-form refusal

DOT requested for the package, node, outcome or counterexample view refuses
`StageFailure::Refused(RenderCause::UnsupportedForm { view, form })`,
naming the view kind and the form `dot`. Its catalog code is QSpec FR-271's
`invalid_quire_context` (a lifecycle request that supplied an incompatible
context) with QSpec FR-272's cause `unsupported-render-form`, whose payload
keys `view` and `form` carry the view kind (`package`, `node`, `outcome`,
`counterexample`, `trace` or `state-graph`) and the form. Its O-16 category
is refusal and it exits 20 (FR-285). A
form the view has is never refused this way.

## Inputs

An outcome or `View`; the output form (`json`, `text` or `dot`);
`RenderSources`; `RenderLimits` and `&Cancel`.

## Outputs

`Result<Staged<Rendered>, StageFailure<RenderCause>>`, following
[FR-275](FR-275-take-a-typed-request-limits-and-cancel-on-every-lifecycle-operation.md).
`Rendered` holds the rendered bytes. A refused, limited or cancelled render
returns no bytes and no partial bytes.

## Behavior

- The `render` operation shall produce JSON output by FR-286's serialization
  or the view's own serialization.
- The `render` operation shall produce text and DOT output from the typed
  outcome or view only.
- The `render` operation shall compute a diagnostic's line and column from
  the caller's source bytes at render time.
- No logic stage shall take a rendered form as input.
- If `dot` is requested for a view other than state graph or trace, then
  `render` shall refuse `RenderCause::UnsupportedForm` naming the view and
  the form, with catalog code `invalid_quire_context`, cause
  `unsupported-render-form` and category refusal.
- The `render` operation shall look up each diagnostic region's source
  digest in `RenderSources` and take the path, line and column from the
  `Source` it finds.
- If `RenderSources` holds no `Source` for a region's digest, or the
  region's end lies past that `Source`'s bytes, then `render` shall print the
  region's source digest and byte offsets in place of path, line and column.
- The `render` operation shall render a trace view as DOT with one node per
  position, one edge from each position to the next, and, for a lasso, one
  edge from the last position to the loop start.
- When `render` has an output unit to write, `render` shall charge the
  unit's UTF-8 byte length against `RenderLimits.output_bytes` and poll
  `&Cancel` before the unit is written.
- When a charge would exceed `output_bytes`, `render` shall return
  `StageFailure::Limit(LimitExceeded)` naming `render.output_bytes`, the
  bound and the count the charge would have reached, and no bytes.
- When the `Cancel` handle is cancelled, `render` shall stop at its next
  charge and return `StageFailure::Cancelled` with the handle's cause and no
  bytes.
- The `render` operation shall return byte-equal output for equal inputs,
  forms, source provisions and limits whenever its `Cancel` handle is not
  cancelled.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-298-AC-1 | The text rendering of FR-286-AC-2's refusal is one line `<path>:<line>:<column>: ill_typed: <message>`, whose line and column are those of the region's first byte in the provided source; without the source bytes it shows the source digest and byte offsets. | Test (TC-783) |
| FR-298-AC-2 | The text rendering of FR-281-AC-2's counterexample is a table with one row per parameter name and value; FR-283-AC-2's lasso renders as numbered positions with the loop start marked; FR-290-AC-1's plugin, `analyze` and Kani records show `proved` with `trusted`, `certified` and `certified` beside it, and FR-281-AC-4's record with `uncertified`; a 1 MiB text value renders as FR-073's bounded descriptor. | Test (TC-783) |
| FR-298-AC-3 | The DOT rendering of an FR-120 state-graph view has one node per retained state and one edge per retained transition, and `dot` for the package view refuses naming the view and the form. | Test (TC-783) |
| FR-298-AC-4 | With the text renderer swapped for a test renderer that uppercases every line, the JSON bytes of every AC-1 to AC-3 outcome and view are unchanged. | Test (TC-783) |
| FR-298-AC-5 | `dot` requested for the package view, the node view of `seven`, FR-286-AC-2's outcome view and FR-281-AC-2's counterexample view each returns `StageFailure::Refused(RenderCause::UnsupportedForm { view, form: dot })` naming that view kind, no bytes, catalog code `invalid_quire_context` with cause `unsupported-render-form`, `view` that view kind and `form` `dot`, category refusal and exit 20; `text` and `json` for the same four views and `dot` for the state-graph and trace views are not refused. | Test (TC-783) |
| FR-298-AC-6 | FR-286-AC-2's refusal rendered as text with a `RenderSources` holding its `Source` admitted under path `src/inv.qsl` prints `src/inv.qsl:` followed by FR-001's line and scalar column of the region's first byte. An `ill_typed` refusal over a source whose region's line holds `é` (two UTF-8 bytes) before the region prints column `b`, where `b` is the region's zero-based byte offset within its line: FR-001's one-based scalar column, not the byte-based `b + 1`. FR-286-AC-2's refusal with an empty `RenderSources`, with a `RenderSources` holding only another source, and with its region's end moved one byte past the provided source's length, prints the region's source digest and byte offsets each time, and no path, line or column. | Test (TC-783) |
| FR-298-AC-7 | The DOT rendering of FR-283-AC-2's lasso (prefix positions 0 and 1, loop positions 2 and 3), parsed as DOT, has exactly four nodes labelled 0 to 3, the edges 0→1, 1→2, 2→3 and 3→2 and no other, and marks the deciding position's node; FR-283-AC-1's three-position finite trace has exactly the edges 0→1 and 1→2 and no edge out of position 2. | Test (TC-783) |
| FR-298-AC-8 | Rendering AC-7's lasso as DOT with `render.output_bytes` set to one below the byte length of its default-limit DOT output returns `StageFailure::Limit(LimitExceeded)` with kind `input-bytes-exceeded`, that bound, the count the refused charge would have reached and setting `render.output_bytes`, no bytes, and exit 22; at exactly that byte length it returns the default-limit bytes. | Test (TC-783) |
| FR-298-AC-9 | DOT rendering of a retained state graph of 100,000 states, cancelled with `Deadline` from a second thread once the test's charge observer has seen 1,000 output charges, returns `StageFailure::Cancelled(Deadline)`, no bytes, and the observer records at most one charge after the cancellation; with a handle already cancelled with `Requested`, `render` returns `StageFailure::Cancelled(Requested)` before its first charge. Both exit 22. | Test (TC-783) |
| FR-298-AC-10 | Each AC-2, AC-3 and AC-7 rendering called twice with equal inputs, forms, `RenderSources` and limits returns byte-equal output. The text counterexample rows follow the function's parameter declaration order; the text trace lists positions in increasing index with the loop-start line and the deciding-position line each marked; no structural element AC-2 or AC-7 names is absent. | Test (TC-783) |

## Dependencies

- ADR-029 IN-2, IN-3: formats, diagnostics and counterexamples.
- ADR-011 FB-02, FB-09: renderers feed nothing back.
- [FR-001](FR-001-read-exact-source.md): the admitted `Source`, its path and its line and column coordinates.
- [FR-095](FR-095-occurrence-keyed-source-map-and-locus.md): a region names its source by digest.
- [FR-255](FR-255-name-the-setting-that-raises-a-reached-limit.md), [FR-276](FR-276-cancel-a-lifecycle-operation.md) and [FR-277](FR-277-bound-every-lifecycle-operation-by-caller-limits.md): `render.output_bytes` and cancellation.
- [FR-285](FR-285-map-every-outcome-category-to-one-exit-code.md): exit codes.
- QSpec FR-271 and FR-272: `invalid_quire_context` and its cause `unsupported-render-form`; QSpec FR-461: the output-byte setting name.
- [FR-073](FR-073-implement-redacted-safe-diagnostic-rendering.md): bounded descriptors.
- [FR-286](FR-286-serialize-every-outcome-as-one-json-outcome-document.md): the outcome document.
- [FR-297](FR-297-inspect-packages-outcomes-and-traces-as-typed-views.md): the views.
- QSpec FR-300: the `render` operation.

## References

- QSL-390 (ARCH-50), QSL-393 (V1-A06): rendering.
- QSpec FR-300 (STD-141): the QSpec half.
