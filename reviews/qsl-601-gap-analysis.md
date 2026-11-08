---
id: SR-20261108230651
title: "Gap analysis — qsl-inspect render (JSON, text, DOT) re-land"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; branch l5/p2-render e508a53e1 vs origin/l5/integration; FR-298-AC-1..11; TC-783; qsl-inspect/src/render.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-298
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-783
    type: references
---

## Summary

Ticket: QSL-601. Gap analysis of FR-298-AC-1 to AC-11 (TC-783) against the
eleven `#[trace("TC-783", ...)]` tests and the one `compile_fail` doctest in
`qsl-inspect/src/render.rs`. Every AC carries at least one tag, but four ACs
are only partly backed and one test cannot fail.

## Verdict

**Changes requested.** AC-4's test is vacuous; AC-3's package-view clause and
AC-5's package and node views have no test; AC-9's test exercises a private
helper over pre-rendered bytes, not `render`. AC-1, AC-2, AC-7 and AC-10 are
backed by tests that fail on a wrong result.

## Scope

- **FR-298-AC-1 — examined**: "The text rendering of FR-286-AC-2's refusal is one line `<path>:<line>:<column>: ill_typed: <message>` ... without the source bytes it shows the source reference and byte offsets."
- **FR-298-AC-2 — examined**: "counterexample is a table ...; lasso renders as numbered positions with the loop start marked; ... `proved` with `trusted`, `certified` and `certified` ..., and FR-281-AC-4's record with `uncertified`; a 1 MiB text value renders as FR-073's bounded descriptor."
- **FR-298-AC-3 — examined**: "DOT rendering of an FR-120 state-graph view has one node per retained state and one edge per retained transition, and `dot` for the package view refuses naming the view and the form."
- **FR-298-AC-4 — examined**: "With the text renderer swapped for a test renderer that uppercases every line, the JSON bytes of every AC-1 to AC-3 outcome and view are unchanged."
- **FR-298-AC-5 — examined**: "`dot` requested for the package view, the node view of `seven`, FR-286-AC-2's outcome view and FR-281-AC-2's counterexample view each returns `StageFailure::Refused(RenderCause::UnsupportedForm { view, form: dot })` ... category refusal and exit 20; `text` and `json` for the same four views ... are not refused."
- **FR-298-AC-6 — examined**: "... with its region's end moved one byte past the provided source's length, prints the region's source reference and byte offsets each time ..."
- **FR-298-AC-7 — examined**: "lasso ... exactly four nodes labelled 0 to 3, the edges 0→1, 1→2, 2→3 and 3→2 and no other, and marks the deciding position's node; ... finite trace has exactly the edges 0→1 and 1→2."
- **FR-298-AC-8 — examined**: "... returns `StageFailure::Limit(LimitExceeded)` with kind `input-bytes-exceeded`, that bound, the count ... and setting `render.output_bytes`, no bytes, and exit 22; at exactly that byte length it returns the default-limit bytes."
- **FR-298-AC-9 — examined**: "DOT rendering of a retained state graph of 100,000 states, cancelled with `Deadline` from a second thread once the test's charge observer has seen 1,000 output charges ... with a handle already cancelled with `Requested`, `render` returns `StageFailure::Cancelled(Requested)` before its first charge."
- **FR-298-AC-10 — examined**: "Each AC-2, AC-3 and AC-7 rendering called twice ... returns byte-equal output. The text counterexample rows follow the function's parameter declaration order ..."
- **FR-298-AC-11 — examined**: "`RenderSources::from_sources` over two admitted `Source` values holds each under its own source reference; ... renders with path `a.qsl`. A `compile_fail` doctest that inserts a `Source` under a caller-supplied key, or builds `RenderSources` from a map, fails to compile beside a compiling `from_sources` control."

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `json_bytes_do_not_depend_on_the_text_renderer` swaps no renderer: it upper-cases a local copy of the text, renders JSON twice and compares (a determinism check), and asserts the JSON lacks `[LOOP START]`, which it never could contain. It covers only the lasso, not every AC-1 to AC-3 outcome and view, and fails under no regression of JSON/text independence. | qsl-inspect/src/render.rs:1094-1107 |
| FND-002 | medium | AC-5's package view and node view of `seven`, and AC-3's "`dot` for the package view refuses", have no test; the AC-5 test covers only outcome and counterexample and asserts neither category refusal nor exit 20. `view_kind`'s `package` and `node` arms are unexercised. | qsl-inspect/src/render.rs:1109-1154, 270-279 |
| FND-003 | medium | The AC-9 test calls the private `finish_units` over bytes already rendered with an uncancelled handle, cancels synchronously from the observer instead of from a second thread, and runs the `Requested` case against `finish_units` too. `render_view`'s own cancellation (during formatting, or before its first charge) is never exercised, and "at most one charge after cancellation" is trivially zero. | qsl-inspect/src/render.rs:1329-1372 |
| FND-004 | low | AC-8's test asserts setting, bound and actual but not kind `input-bytes-exceeded` or exit 22. | qsl-inspect/src/render.rs:1321-1326 |
| FND-005 | low | AC-6's past-end case moves the end to 99 rather than one byte past the 6-byte source (7), so an off-by-one in the end check would pass. | qsl-inspect/src/render.rs:1066-1072 |
| FND-006 | low | AC-11's first clause (two distinct references each held under its own reference, both resolvable) is not asserted; the `compile_fail` doctest only builds a struct literal with a private field, not an insert under a caller key or a build from a map. | qsl-inspect/src/render.rs:96-100, 1036-1073 |

## Coverage

- Reconciliation: `quoin matrix --json` (quoin 0.29.1) was started and had not finished after several minutes, matching a prior timeout on this repo; a manual trace-tag index of `render.rs` was used and is not matrix output.
- Plan completion: not assessed
- AC-1: `diagnostic_text_resolves_path_and_scalar_column_or_falls_back_to_the_reference` — backed.
- AC-2: `trace_text_marks_loop_and_deciding_positions_in_order`, `counterexample_text_uses_parameter_declaration_order_and_bounds_bulk_values`, `proved_item_text_shows_its_certification_beside_the_verdict` — backed.
- AC-3: `state_graph_dot_has_one_node_per_state_and_one_edge_per_transition` — first clause backed; package-view clause unbacked (FND-002).
- AC-4: vacuous (FND-001); `outcome_json_is_the_existing_authoritative_document_bytes` backs outcome JSON = FR-286 bytes only.
- AC-5: partial (FND-002).
- AC-6: backed except the boundary (FND-005).
- AC-7: `trace_dot_has_exact_lasso_edges_and_marks_the_deciding_position` — backed (count + set by substring, not a DOT parse; adequate).
- AC-8: `output_bytes_bound_is_inclusive_and_refusal_returns_no_bytes` — backed except kind/exit (FND-004).
- AC-9: helper-level only (FND-003).
- AC-10: repeat-equality in the AC-2/3/7 tests and declaration-order test — backed.
- AC-11: partial (FND-006).
- Reverse gap: no production code in the diff lacks an owning FR-298 statement; the `qsl-replay` accessors serve AC-2. No stubs.
- Semantic review: not run.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `json_bytes_of_every_outcome_and_view_are_pinned_independent_of_text_and_dot` pins exact JSON for the AC-1 refusal, AC-2 proved records and counterexample, AC-2/AC-7 lasso and AC-3 graph, rendering text and DOT between two JSON renders (render.rs:1222-1228, 1506-1578). The renderer is not pluggable, so a golden pin is the realizable form; it fails if JSON starts to depend on text, DOT or `Debug`. The pin was taken from a first run, so it fixes current output rather than an independently derived expectation. |
| FND-002 | fixed | Package view and the node view of `seven` from the real spine fixture, plus outcome and counterexample, each refuse DOT with category refusal and exit 20, and text/JSON render (render.rs:1599-1704). |
| FND-003 | fixed | AC-9 runs through `render_view_observed` with a second thread cancelling while the observer blocks at charge 1,000; asserts `Deadline`, exit 22, 1,000 observed, at most one charge after cancel; `Requested` is checked through `render_view_observed` for all three forms and through `render_outcome` (render.rs:1890-1981). |
| FND-004 | fixed | Asserts `LimitKind::InputBytes`, `stage_limit_exceeded`/`input-bytes-exceeded`, `render.output_bytes`, exit 22 and the charge count (render.rs:1837-1888). |
| FND-005 | fixed | Past-end case uses source length + 1 and checks the at-length case still resolves (render.rs:1442-1457). |
| FND-006 | fixed | Distinct references resolve independently; same-reference keeps `a.qsl`; `compile_fail` doctests for a build from a map and an insert under a caller key (render.rs:102-115, 1460-1485). |
