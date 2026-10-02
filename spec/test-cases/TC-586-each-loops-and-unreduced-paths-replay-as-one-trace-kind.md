---
id: TC-586
title: "Each-fairness loops take every identity, and every reduced counterexample is one ordinary trace kind"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-161
    type: verifies
---
# TC-586: Each-fairness loops take every identity, and every reduced counterexample is one ordinary trace kind

## Description

Verify the AQ-7 loop for an `each` refutation, that partial-order and constrained paths are emitted unchanged, that no counterexample carries reduction data, and determinism.

Scope: FR-161-AC-3 to FR-161-AC-4.

## Test Procedure

Fixtures: ADR-021 §7.1's `each` refutation; the counterexamples of TC-577 and TC-580.

1. Concretise the `each` refutation and replay it with the fairness re-check.
2. Emit the TC-577 and TC-580 counterexamples; decode every counterexample of TC-585 and this case as `TemporalCounterexample` and scan for canonical states, permutations and reduction names; concretise TC-585 step 1 twice.

Tag the tests `#[trace("TC-586", "FR-161-AC-n")]`.

## Expected Results

- Step 1: a loop from and to `(0,0,0)` taking `upd(a)`, `upd(b)`, `upd(c)` and passing `a.versionNumber = 2`; replay confirms each identity taken and settles `reproduced-with-evaluated-witness`.
- Step 2: unchanged paths that replay; every decode succeeds and the scan finds nothing; byte-equal results.
