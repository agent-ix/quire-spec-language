---
id: TC-671
title: "S3 warns of load-buffering shapes and candidate sets follow the resolved model"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-226
    type: verifies
---
# TC-671: S3 warns of load-buffering shapes and candidate sets follow the resolved model

## Description

Verify the load-buffering warning, EN-1's advertised memory models, and candidate sets by resolved model.

Scope: FR-226-AC-1 to FR-226-AC-3.

## Test Procedure

1. Check the branch `r1 := x (relaxed); y := 1 (relaxed)`, its `acquire`
   and fenced variants, and a same-location load and store.
2. Read EN-1's registered provider manifest.
3. Compute the candidate set of a `tso` item over `SB` with EN-1 and an
   `sc`-only test candidate registered, then with the test candidate alone;
   compute the `sc` request's set with it.

Tag the tests `#[trace("TC-671", "FR-226-AC-n")]`.

## Expected Results

- Step 1: one `memory.load-buffering-shape` warning naming both attempts
  and a compiled package; no warning in the other three.
- Step 2: `sc`, `tso` and `ra`.
- Step 3: `{EN-1}`; the empty set; `{test candidate}` for the `sc`
  request.
