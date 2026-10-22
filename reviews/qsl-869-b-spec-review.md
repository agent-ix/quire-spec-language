---
id: SR-12320
title: 'Spec review of the QSL PR #1203 fix-round edits: FR-094 population-reference owner, FR-391-AC-6 unit wording (QSL-869)'
type: SpecReview
analysis: base
scope: 'agent-ix/quire-spec-language@b8d4723ef; fix commit 50605a374 (PR #1203, branch qsl-869b-impl): spec/functional/FR-094-key-model-owned-reference-population-and-quantity-nodes.md, spec/functional/FR-391-name-a-population-in-a-state-clause.md; ticket QSL-869'
review_set: subset
date: '2026-10-22'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-094
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-391
  type: reviews
---

## Summary

Ticket: QSL-869. PR quire-spec-language#1203, fix commit 50605a374, head b8d4723ef. The fix round changed two spec units, and both are reviewed here. TC-1076 was not changed and was read for consistency only.

- **FR-094 population-reference paragraph.** It now reads "keyed by the population's declaration key alone, with no `N`, whose `owner` is the model owner of that key (QSpec FR-322)". This agrees with QSpec FR-322 "Node" and "Declaration key" at quire-specification main bb6d08ae: a `ModelOwner {kind: model, identity, node}` that is the declaration key, no maximum, and `malformed_wire` without an owner. It also agrees with the code: `population_node_content` keys P7/P8 exactly, as re-measured this round. The contradiction of SR-12316 FND-003 is gone.
- **FR-391-AC-6.** The FR-155-AC-2 precondition clause changed from "in the same unit and emitted package" to "in a unit without `archive` (with it, a clause on `M::Counter` is ambiguous, AC-4)". The old wording could not be satisfied. FR-155-AC-2's precondition is anchored on `M::Counter::bump`, and FR-391-AC-4 (and FR-104-AC-5) refuse a clause on `M::Counter` as `ambiguous_declaration`/`ambiguous-name` once `archive`, a second population of `Counter`, is present. The new wording is consistent with AC-4. It matches the existing test `a_named_population_is_the_one_membership_read`, which checks `ROOM` over `Unit::new(false, ...)`. TC-1076 row 8 never named the FR-155-AC-2 clause, so it needs no change.
- `quire validate --scope .` on both files exits 0. Its only output is module-catalog duplicate warnings, the same as for every file.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-094 population-reference paragraph | spec/functional/FR-094-key-model-owned-reference-population-and-quantity-nodes.md:212-217 | examined |
| FR-391-AC-6 | spec/functional/FR-391-name-a-population-in-a-state-clause.md:214 | examined |
| FR-391-AC-4 | spec/functional/FR-391-name-a-population-in-a-state-clause.md:212 | context_only |
| FR-155-AC-2 | spec/functional/FR-155-derive-and-enforce-read-and-write-footprints.md:160 | context_only |
| TC-1076 row 8 | spec/test-cases/TC-1076-a-state-clause-names-a-population.md:30 | context_only |
| QSpec FR-322 "Population references" Node / Declaration key | quire-specification spec/objects/interfaces/FR-322-checked-package-artifact.md:1394-1406 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | AC-6 now spans two units. The `A`/`B`/`C` unit has `archive`, and the FR-155-AC-2 precondition's unit has no `archive`. The next sentence, "The emitted package holds one population reference per declared population read ... the package is admitted by the full reader", does not say which unit's package it means. The tests read it as the `A`/`B`/`C` unit (`the_ac_6_unit_is_admitted_by_the_full_reader`). Fix: "The emitted package of the `A`, `B`, `C` unit holds ...". | spec/functional/FR-391-name-a-population-in-a-state-clause.md:214 |

## Verdict

**Clean apart from one low wording finding. Nothing blocks the merge.** Both edits are correct and consistent with QSpec FR-322, FR-391-AC-4 and the tests.

## Dispositions

| FND | Outcome | Sha |
| --- | --- | --- |
| FND-001 | fixed | 0f21c85a1 |
