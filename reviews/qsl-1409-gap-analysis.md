---
id: SR-1988004
title: "Gap analysis — QSL-1409 acceptance coverage"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; qsl-eval/src/simulation/protocol/ra.rs, qsl-analyze/tests/it/rc11_oracle.rs, spec/functional/FR-222-explore-the-release-acquire-memory-model.md, spec/functional/FR-223-order-seq-cst-events-by-the-rc11-partial-sc-order.md, spec/test-cases/TC-668-rc11-partial-sc-order.md"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-spec-language/FR-223", type: references }
---

## Summary

Performed a planless, PR-scoped manual acceptance-criteria-to-tests check for QSL-1409. The changed RA state-key path is owned by the existing release-acquire and partial-SC-order requirements, and its LT-16 and LT-12 tests carry the relevant trace tags and behavioral assertions; repository-wide matrix and reverse-gap claims were not made.

## Verdict

**CONDITIONAL** — the examined PR acceptance mapping has no gap, but the repository-wide Quoin matrix did not complete in this runtime, so this is not a repository-wide gap-analysis pass.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No gaps found in the PR-scoped manual acceptance check | FR-223-AC-8, FR-223-AC-5 |

## Coverage

- Reconciliation: manual acceptance criteria-to-tests check — `quoin matrix --repo . --json` and `quire matrix --scope . --format json` did not produce a matrix after module diagnostics and were cancelled; the full repository matrix was unavailable.
- Plan completion: not assessed
- Criteria examined: `FR-223-AC-8` (`lt16_collected_sc_predecessors_still_constrain_the_sc_load`) and `FR-223-AC-5` (`lt12_reaches_its_weak_outcome_under_ra_only`); both tests are non-ignored and carry `TC-668` plus the applicable criterion id.
- The LT-16 oracle checks its state and edge counts and the forbidden weak outcome; the LT-12 comparison checks the expected RA-only weak outcome against RC11, SC, and TSO.
- Reverse-gap inventory, stubs, and repository-wide criterion counts: not assessed because the matrix tool did not complete and this review was scoped to the changed behavior.
- Semantic review: skipped.

## Bindings

- `lt16_collected_sc_predecessors_still_constrain_the_sc_load` → `FR-223-AC-8`: correct.
- `lt12_reaches_its_weak_outcome_under_ra_only` → `FR-223-AC-5`: correct.
