---
id: TC-617
title: "Complete enabling footprints and membership keep fair violations under partial-order reduction"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-157
    type: verifies
---
# TC-617: Complete enabling footprints and membership keep fair violations under partial-order reduction

## Description

Verify that fairness visibility reads a class member's whole read footprint,
on ADR-021 §7.6's `Gate` with `go`'s guard in its postcondition, and that a
`creates` adding a member to a `whole` class is fairness-visible.

Scope: FR-157-AC-5, FR-157-AC-6.

## Test Procedure

Fixtures: ADR-021 §7.6's `Gate` with `go()`'s pre `not self.done` and post
`self.done and pre(self.c) and pre(self.d)`, claim `Opens`, `fair weak go`;
the `Gate` unit with `spawn()` framed `creates Gate`, `fair weak whole go`.

1. Compute `go(g)`'s enabling footprint and the visible set, then run the
   first subject with partial-order reduction and without it.
2. Compute `spawn`'s visibility under the second subject's fairness set.

Tag the tests `#[trace("TC-617", "<AC id>")]`.

## Expected Results

- Step 1: the enabling footprint holds `Field{g, c}` and `Field{g, d}`;
  `arm` and `arm2` are visible; the ample set at `(F,F,F,F)` is `{toggle}`;
  both runs return `Violated` with the loop `toggle, toggle`, the reduced
  run over 6 states.
- Step 2: `spawn` writes `AnyMembership{gates}`, which meets
  `Membership{gates, g}`, and is visible.
