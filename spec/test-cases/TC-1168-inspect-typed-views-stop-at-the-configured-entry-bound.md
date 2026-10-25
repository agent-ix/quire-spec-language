---
id: TC-1168
title: "Inspect typed views within the configured entry bound"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-297
    type: verifies
---
# TC-1168: Inspect typed views within the configured entry bound

## Description

Verify FR-297-AC-10's determinism, cancellation and typed-view entry limit.
Use inputs that produce at least two entries in each applicable collection:
package dependencies, model selections, exports, requirements and capability
report records; node occurrences and source-map regions; counterexample
assignments; trace positions; and state-graph states and transitions.

## Test Procedure

1. For each view type, inspect a complete fixture with at least two collection
   entries. For a package view, include entries in more than one collection
   category. Set `InspectLimits.max_view_entries` to the total entry count
   across all collections in that selected view.
2. Repeat each request with the bound one less than that total count.
3. Inspect a valid empty view with the bound set to zero.
4. Inspect the complete fixtures twice with equal limits and independent
   uncancelled handles; then submit a request through a precancelled handle.
5. Compare lifecycle-stage work reported by inspection with the inputs'
   previously recorded producer outcomes.

Tag tests `#[trace("TC-1168", "FR-297-AC-10")]`.

## Expected Results

- Step 1 returns the complete view for every collection, with no limit
  failure.
- Step 2 returns no view and reports `StageFailure::Limit` identifying
  `max_view_entries`, the configured bound, an attempted entry count equal to
  the bound plus one, and `inspect.max_view_entries`.
- Step 3 returns the empty view.
- Step 4's equal requests return equal views; the precancelled request returns
  cancelled with no view.
- Step 5 reports no work in the stages that produced each input.
