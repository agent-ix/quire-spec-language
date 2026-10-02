---
id: TC-886
title: "Reduced and unreduced runs agree over the model-check corpus"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-160
    type: verifies
---
# TC-886: Reduced and unreduced runs agree over the model-check corpus

## Description

Verify that no reduction changes a verdict: every model-check case of the
corpus runs unreduced and under each reduction selection its PT-2 rows
admit, and each pair settles the same verdict.

Scope: FR-160-AC-6.

## Test Procedure

Fixtures: every model-check case of QSL's model-check test corpus, which
includes the worked examples of ADR-018, ADR-019, ADR-021 §7.1 to §7.6 and
ADR-027, and the QSpec conformance vectors QSL runs.

1. For each case, list the selections among symmetry, partial-order
   reduction, and both, whose PT-2 rows admit the case's claim form and
   fairness set. List symmetry only where the case declares a `symmetric`
   population, and partial-order reduction only where its system supplies
   footprints. Skip a case that carries a state constraint.
2. Run the case unreduced, then once under each listed selection.
3. Drop a pair in which either run ends at its horizon or at a limit.
4. Compare the ADR-018 verdicts of each remaining pair, and replay every
   counterexample from either run (FR-128), with fairness re-established for
   a lasso.

Tag the tests `#[trace("TC-886", "FR-160-AC-6")]`. The test runs under `make ci`
(`cargo test --locked --workspace`) over the whole corpus and is not
`#[ignore]`, so the differential is part of the gate.

## Expected Results

- Every compared pair settles the same verdict, and every counterexample
  replays.
- ADR-021 §7.1: `fair weak each` `Holds` and `fair weak whole` `Violated`,
  both under symmetry and unreduced.
- ADR-021 §7.4: `Holds` under the default scheduler constraints and
  `Violated` under `scheduling adversarial`, both under partial-order
  reduction and unreduced.
- ADR-021 §7.5 and §7.6: every vector `Violated` under partial-order
  reduction and unreduced.
