---
id: TC-783
title: "render produces text and DOT from typed values only, leaving JSON unchanged"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-298
    type: verifies
---
# TC-783: render produces text and DOT from typed values only, leaving JSON unchanged

## Description

Verify the renderers.

Scope: FR-298-AC-1 to FR-298-AC-11.

## Test Procedure

1. Render FR-286-AC-2's refusal as text with the source bytes, then without them.
2. Render FR-281-AC-2's counterexample, FR-283-AC-2's lasso, FR-290-AC-1's record and a 1 MiB text value as text.
3. Render an FR-120 state-graph view as DOT; request DOT for the package view.
4. Swap the text renderer for a test renderer that uppercases every line, and serialize the JSON of every outcome and view from steps 1 to 3.
5. Request DOT for the package view, the node view of `seven`, FR-286-AC-2's outcome view and FR-281-AC-2's counterexample view; request text and JSON for the same four views.
6. Render FR-286-AC-2's refusal as text with `RenderSources` holding its `Source` admitted under path `src/inv.qsl`; render an `ill_typed` refusal over a source with `é` before the region on its line; render FR-286-AC-2's refusal with an empty `RenderSources`, with one holding only another source, and with its region's end one byte past the source's length.
7. Render FR-283-AC-2's lasso and FR-283-AC-1's finite trace as DOT and parse each output as DOT.
8. Render step 7's lasso as DOT with `render.output_bytes` at one below its default-limit byte length, then at exactly that length.
9. Render a retained 100,000-state graph view as DOT and cancel with `Deadline` from a second thread once the charge observer has seen 1,000 output charges; render it again with a handle already cancelled with `Requested`.
10. Repeat each step 2, 3 and 7 rendering with equal inputs, forms, `RenderSources` and limits, and compare the bytes; inspect the text counterexample's row order and the text trace's position order and markers.
11. Build `RenderSources::from_sources` over two `Source` values, then over two with equal bytes under paths `a.qsl` and `b.qsl` and render FR-286-AC-2's refusal; compile the `compile_fail` doctests that insert a `Source` under a caller-supplied key or build `RenderSources` from a map, beside a `from_sources` control.

Tag the tests `#[trace("TC-783", "<AC id>")]`.

## Expected Results

- Step 1: one line `<path>:<line>:<column>: ill_typed: <message>` at the region's first byte; without the source, the source identity and byte offsets.
- Step 2: a table of parameter names and values; numbered positions with the loop start marked; `proved` with `trusted` beside it; FR-073's bounded descriptor.
- Step 3: one node per retained state and one edge per retained transition; the package-view request refuses naming the view and the form.
- Step 4: every JSON byte string equals the one produced with the standard renderer.
- Step 5: each DOT request returns `StageFailure::Refused(RenderCause::UnsupportedForm { view, form: dot })` naming its view kind, with no bytes, catalog code `invalid_quire_context`, cause `unsupported-render-form` with `view` and `form` `dot`, category refusal and exit 20; text and JSON succeed.
- Step 6: `src/inv.qsl:<line>:<column>` with FR-001's coordinates; for the `é` source the column is the zero-based byte offset within the line, not that offset plus one; the three fallback runs print the source identity and byte offsets with no path, line or column.
- Step 7: the lasso has exactly nodes 0 to 3 and edges 0→1, 1→2, 2→3, 3→2, with the deciding node marked; the finite trace has exactly edges 0→1 and 1→2.
- Step 8: one below gives `StageFailure::Limit(LimitExceeded)` with kind `input-bytes-exceeded`, the bound, the would-be count and setting `render.output_bytes`, no bytes, exit 22; exactly the length gives the default-limit bytes.
- Step 9: `StageFailure::Cancelled(Deadline)`, no bytes, at most one charge after cancellation; then `StageFailure::Cancelled(Requested)` before any charge; both exit 22.
- Step 10: every pair of repeated renderings is byte-equal; counterexample rows follow parameter declaration order; trace positions increase, with the loop-start and deciding lines marked.
- Step 11: each `Source` sits under its own identity; the refusal prints path `a.qsl`; the doctests fail to compile and the control compiles.
