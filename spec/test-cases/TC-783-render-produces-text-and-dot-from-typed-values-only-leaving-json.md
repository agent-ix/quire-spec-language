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

Scope: FR-298-AC-1 to FR-298-AC-4.

## Test Procedure

1. Render FR-286-AC-2's refusal as text with the source bytes, then without them.
2. Render FR-281-AC-2's counterexample, FR-283-AC-2's lasso, FR-290-AC-1's record and a 1 MiB text value as text.
3. Render an FR-120 state-graph view as DOT; request DOT for the package view.
4. Swap the text renderer for a test renderer that uppercases every line, and serialize the JSON of every outcome and view from steps 1 to 3.

Tag the tests `#[trace("TC-783", "<AC id>")]`.

## Expected Results

- Step 1: one line `<path>:<line>:<column>: ill_typed: <message>` at the region's first byte; without the source, the source digest and byte offsets.
- Step 2: a table of parameter names and values; numbered positions with the loop start marked; `proved` with `trusted` beside it; FR-073's bounded descriptor.
- Step 3: one node per retained state and one edge per retained transition; the package-view request refuses naming the view and the form.
- Step 4: every JSON byte string equals the one produced with the standard renderer.
