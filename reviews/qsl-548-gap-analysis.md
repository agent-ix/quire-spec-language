---
id: SR-12279
title: "Gap analysis of QSL-548 step simulation occurrence keys (PR #1164)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@f9e64bcfa (qsl-548-impl, diff origin/main...HEAD); FR-146 (Record identity and occurrence, AC-3), ADR-013 O-07 and O-09, TC-551, against qsl-semantics/tests/it/refinement_declarations.rs"
review_set: subset
---

## Summary

Ticket: QSL-548 (PR quire-spec-language#1164). Planless gap analysis of the
diff: the behaviour the PR adds (each per-step record carries its O-07
occurrence key) and the FR-146-AC-3 second half, compared with the tests
added in the diff and the strength of their oracles. Plan completion: not
assessed.

The PR tags its tests `TC-551` only and claims no FR-146 AC. That is
deliberate and correct. No AC is fully built yet: AC-1 and AC-3 speak of the
records in the request, and writing them into `CheckedGraph::requirements()`
is listed remaining work. So the matrix does not report an unbuilt AC as
backed.

## Verdict

PASS. No high or medium finding; one low.

`each_record_has_its_generated_occurrence_key` has a strong oracle. For all
eight `CasRefinesCounter` entries it asserts the occurrence node (built
independently from the fixture's digest), the role spelling `generated`, the
ordinal equal to the entry's index, and the obligation at that ordinal
(`Initial` at 0, `Step { row: ordinal - 1 }` after it). An off-by-one
ordinal, a wrong role, a wrong node or a desynchronised obligation would each
fail it. `a_hypothesis_is_its_own_operation_contract_record` looks up
`CasInv`'s requirement record by its exact `claim` occurrence key and asserts
the `OperationContract` capability, so it would fail if the invariant wrote
no record or wrote one of another kind. Its second loop repeats an assertion
that `a_concrete_invariant_is_a_hypothesis_of_every_record` already makes.
That is redundant but harmless.

Context, not a finding of this diff: FR-146-AC-5's sortedness clause (the row
at position i has a node-key digest not greater than the row at i + 1's) is
not asserted by `reordering_step_rows_does_not_change_the_row_a_reference_names`,
which checks only permutation and source-order invariance. That test is
unchanged here. The code under it (`step_row_refs`) sorts correctly; see
SR-12278.

## Examined Scope

```yaml
scope:
  - {id: FR-146-AC-3, path: spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md, role: examined, excerpt: "With an invariant CasInv (self.busyA or self.tmpA <= self.value) added to Impl::Counter, every StepSimulationRecord of CasRefinesCounter lists CasInv in hypotheses, the request also holds CasInv's own operation-contract record."}
  - {id: FR-146-AC-1, path: spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md, role: context_only, excerpt: "ADR-020 §8's CasRefinesCounter writes eight operation-contract records beside its refinement record: one Initial and one Step for each of its seven rows"}
  - {id: FR-146-AC-5, path: spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md, role: context_only, excerpt: "Over a model subject, StepRowRef is the zero-based position in the steps order QSpec FR-375 fixes (node-key digest of the first named declaration, then member name)."}
  - {id: FR-146, path: spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md, role: examined, excerpt: "QSpec FR-322-AC-88 through AC-91 define the generated-owner occurrence role, ordinal, region and proof-eligibility rule that ADR-013 O-07 follows."}
  - {id: ADR-013 O-07, path: spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md, role: examined, excerpt: "The initial record has ordinal 0. A step record for zero-based row i has ordinal i + 1, where i is the row's position in the refinement's FR-375 steps total order (FR-375-AC-19), not its source order."}
  - {id: TC-551, path: spec/test-cases/TC-551-per-step-simulation-records-are-written-and-settled.md, role: examined, excerpt: "Scope: FR-146-AC-1 to FR-146-AC-7. ... Tag the tests #[trace(\"TC-551\", \"FR-146-AC-n\")]."}
bindings:
  - {test_id: each_record_has_its_generated_occurrence_key, ac_id: TC-551, trace: correct}
  - {test_id: a_hypothesis_is_its_own_operation_contract_record, ac_id: TC-551, trace: correct}
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The occurrence key behaviour has no acceptance criterion. FR-146 states it in prose ("Record identity and occurrence", deferring to ADR-013 O-07 and QSpec FR-322-AC-88 to AC-91), but AC-1 to AC-8 never mention the key, its role or its ordinal. TC-551 has no step for it either. `each_record_has_its_generated_occurrence_key` can therefore carry only the `TC-551` tag, and the matrix cannot show the O-07 mapping as covered. Fix: add an FR-146 AC (CasRefinesCounter's eight records carry keys (refinement node, `generated`, 0..7) with `Step { row: i }` at ordinal i + 1, unchanged under reversed source rows), plus a matching TC-551 step. | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md:136-146, 193-204; qsl-semantics/tests/it/refinement_declarations.rs:1525-1548 |

## Coverage

Planless review over the PR diff; tags read from the diff's `#[trace]`
attributes. Plan completion: not assessed. A `quoin matrix` run was not
needed to judge the diff's own tags.
