---
id: TC-671
title: "EN-1 warns of load-buffering shapes and candidate sets follow the resolved model"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-226
    type: verifies
---
# TC-671: EN-1 warns of load-buffering shapes and candidate sets follow the resolved model

## Description

Verify EN-1's load-buffering warning, EN-1's advertised memory models, and candidate sets by resolved model.

Scope: FR-226-AC-1 to FR-226-AC-3.

## Test Procedure

1. Check the package of the branch `r1 := x (relaxed); y := 1 (relaxed)` at S3
   and run EN-1's pre-check on a request over it, then its `acquire` and
   fenced variants, a same-location load and store, and a variant whose
   locations no other branch accesses.
2. Read EN-1's registered provider manifest.
3. Compute the candidate set of a `tso` item over `SB` with EN-1 and an
   `sc`-only test candidate registered, then with the test candidate alone;
   compute the `sc` request's set with it.

Tag the tests `#[trace("TC-671", "FR-226-AC-n")]`.

## Expected Results

- Step 1: S3 checks with no warning; EN-1's pre-check puts one
  `memory.load-buffering-shape` warning naming both attempts in the terminal
  record `warnings`, the verdict unchanged; no warning in the other variants.
- Step 2: `memory_models` `sc`, `tso` and `ra`.
- Step 3: `{EN-1}`; the empty set; `{test candidate}` for the `sc`
  request.
