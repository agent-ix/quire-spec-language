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

1. Check the package of branch `t` (`r1 := x (relaxed); y := 1 (relaxed)`)
   with branch `s` (`y := 0 (relaxed); x := 1 (relaxed)`), which makes `x` and
   `y` shared, and run EN-1's pre-check on a request resolved to `ra`. Repeat
   with the load `acquire`, with an `acquire` fence between the two, with a
   same-location load and store, with the load of a location only `t`
   accesses, and with the store to a location only `t` accesses. Run the
   first request resolved to `sc` and to `tso`.
2. Read EN-1's registered provider manifest.
3. Compute the candidate set of a `tso` item over `SB` with EN-1 and an
   `sc`-only test candidate registered, then with the test candidate alone;
   compute the `sc` request's set with it; then repeat with a test candidate
   whose descriptor has no `memory_models`.

Tag the tests `#[trace("TC-671", "FR-226-AC-n")]`.

## Expected Results

- Step 1: under `ra` one `memory.load-buffering-shape` warning naming both
  attempts in the terminal record `warnings`, the record equal to the
  same request settled without the warning; no warning in the other `ra`
  variants; none under `sc` or `tso`.
- Step 2: `memory_models` `sc`, `tso` and `ra`.
- Step 3: `{EN-1}`; the empty set; `{test candidate}` for the `sc`
  request; for the candidate without `memory_models`, `{candidate}` for the
  `sc` request and the empty set for the `tso` request.
