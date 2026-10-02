---
id: TC-578
title: "Fairness visibility keeps a fair violation that C0 to C3 alone would discard"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-157
    type: verifies
---
# TC-578: Fairness visibility keeps a fair violation that C0 to C3 alone would discard

## Description

Verify POR-10 on the `Gate` unit, where `go`'s enabledness reads two locations written by other operations: the visible set, the ample sets, the verdict, and the ample set C0 to C3 alone would choose.

Scope: FR-157-AC-1 to FR-157-AC-2.

## Test Procedure

Fixtures: The `Gate` unit of FR-157, claim `Opens`, `fair weak go`.

1. Run `Opens` with partial-order reduction, recording visibility and ample sets; run it unreduced.
2. At `(F,F,F,F)`, compute the ample-set choice through the visibility function with the fairness set and with the empty fairness set.

Tag the tests `#[trace("TC-578", "FR-157-AC-n")]`.

## Expected Results

- Step 1: `arm`, `arm2`, `go` visible and `toggle` invisible; `{toggle}` at `(F,F,F,F)` and full expansion at `(T,F,F,F)`; `Violated` with an empty stem and loop `toggle, toggle`, equal to the unreduced verdict and lasso.
- Step 2: `{toggle}` with the fairness set; `{arm}` with the empty set.
