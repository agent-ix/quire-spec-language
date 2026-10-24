---
id: SR-11254
title: "Gap analysis — QSL-761 TC-235 M16 run 1 and M07 run 6 (PR 863)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@4a0703e794b64329ba64e9a6c1468d23f059be41; TC-235 M16, TC-235 M07, FR-207-AC-8; qsl-eval/src/value/expression/mod.rs, qsl-eval/src/value/expression/s6a/separation.rs, qsl-semantics/src/model/process.rs"
review_set: subset
---

## Summary

Ticket: QSL-761. Planless gap analysis of the rows PR 863 claims: TC-235 M16 run 1 (located violation path) and TC-235 M07 run 6 (member path of the missing-event refusal), against the QSpec rows and FR-207-AC-8.

## Verdict

**PASS.** M16 run 1: `tc235_m16_process_invariant_and_operation_clauses_evaluate_per_run` asserts the r2 violation at observation `s1`, subject `Object(universe, Fulfilment, r2)`, no steps, as the row states; the implementation in `ClauseSetup::of` produces exactly that and keeps the invocation-receiver root only for state-machine receivers, as FR-207-AC-8 requires. M07 run 6: `IntakeMissingDeclaration.member` is produced at qsl-semantics/src/model/process.rs and the test now asserts `TypeDefinition.steps[1].emits[0]`. No stub or coverage inflation in the changed path. The rows the PR leaves open (M21 run 2, M16 run 3, M22, UUID form, and ticket items 6-10) remain open under QSL-761; the PR references the ticket without closing it.

## Coverage

- Reconciliation: rows checked by reading TC-235 M07/M15/M16/M22 and FR-207-AC-8 and the tagged tests directly; `quoin matrix --repo . --json` (quoin 0.28.3) timed out after 180 s with no output, so no computed matrix backs this review.
- Plan completion: not assessed
- Changed criteria: TC-235 M16 run 1 covered; TC-235 M07 run 6 covered; FR-207-AC-8 (object-identity half) asserted by the M16 test, tagged elsewhere (evaluate.rs, qsl-replay result wire).
- Evidence: coder logs report red then green for the M16 test; not re-run (no builds in this review).
- Reverse gap: none in the changed path.
- Semantic review: skipped (not opted in).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
