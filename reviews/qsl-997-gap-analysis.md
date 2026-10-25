---
id: SR-17171
title: "QSL-997 gap analysis of FR-220-AC-14 and TC-665 step 13 (quire-spec-language PR 1522)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@70cd9be9a070346cabaad698bf85d947527f7d46; spec/functional/FR-220-check-access-orderings-fences-and-access-classification.md; spec/test-cases/TC-665-access-orderings-and-classification.md; qsl-eval/src/simulation/protocol/access.rs; qsl-eval/src/simulation/protocol/steps.rs; qsl-analyze/tests/it/weak_memory.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-220
    type: references
---

## Summary

Ticket: QSL-997 (PR quire-spec-language#1522). This is a planless gap analysis
scoped to FR-220-AC-14 and TC-665 step 13. FR-220-AC-5, FR-220-AC-15 and
FR-220's Behavior and Reporting rules were read for context only.

The PR claims that main already implements the behavior and that QSL-997's
ticket text is stale. The production code was read against every clause of
the AC. Three tests are tagged FR-220-AC-14, all in
`qsl-analyze/tests/it/weak_memory.rs`:

- `ordering_mismatches_are_accumulated_at_the_first_state`, already on main
- `each_ordering_mismatch_settles_its_shape_under_tso_and_ra`, added by this PR
- `a_multi_location_attempt_is_not_tested_for_ordering_and_the_pre_check_continues`, added by this PR

A full `quire matrix --scope .` run did not finish within the 300 s probe, so
the tags were enumerated with grep. Each clause of FR-220-AC-14 has an
implementation and a killing test, checked against `access.rs` and
`steps.rs`. Mutant ids refer to SR-17170.

| Clause | Implementation | Test |
|---|---|---|
| Load with `release` or `acq_rel` | `ordering_mismatch` Load arm | exact wire shape under tso and ra; killed M7 |
| Store with `acquire` or `acq_rel` | Store arm | exact wire shape; killed M8 |
| RMW with no ordering, written `"ordering":null` | `ordering.is_none()`; `map_or(Enc::Null, …)` | exact shape; killed M4 and M6 |
| Local access with an ordering | `Local => ordering.is_some()`, which FR-220's Behavior extends to any ordering, `relaxed` included | the AC's `release` case, exact `local` shape; killed M5 and M9 |
| Admitted under `sc` | the pre-check runs only for a weak governing model | `sc` check for each case |
| An `acq_rel` RMW is not flagged | the Rmw arm | killed M6 |
| A multi-location attempt has its `MultiLocation` entry and no ordering test | `InstanceAccess::Shape` pushes only `multi_location` | only partly asserted: FND-001 in SR-17170 |
| The pre-check does not stop at that entry | `first_shape` loops over every attempt | killed M2 |
| Settles at the initial state, before any outgoing transition | `state_access_shape` runs before expansion | `product_states == 1` in the pre-existing test |
| Failures in canonical order | `from_failures` sorts by `(transition.is_none(), JCS bytes, site)` | **no test**: FND-001 |

The implementation satisfies every clause, and the wire form matches FR-220's
Reporting rule exactly: `{"type":"ordering-mismatch","access":…,"ordering":…}`.
No production change is needed, so the ticket's request to implement is
indeed stale.

## Verdict

PARTIAL. The implementation is complete for FR-220-AC-14 and for TC-665
step 13. Test coverage has one untested clause, canonical order (FND-001).
The weak multi-location assertion is FND-001 of SR-17170.

QSL-997 can be closed when this PR merges, provided the fix round adds an
exact ordered list assertion. That one change covers both findings.

## Findings

| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-001 | medium | FR-220-AC-14's "all other applicable failures are collected in canonical order" has no killing test. Mutant M10 reverses the transition-byte sort in `from_failures`, and all 785 `qsl-analyze` `it` tests still pass. The SB cases produce two identical shapes, and the multi-location test checks only counts and `contains`. Fix: assert the exact ordered `[{site, transition, shape}]` list in the multi-location test | qsl-analyze/tests/it/weak_memory.rs:972-988; qsl-eval/src/simulation/protocol/access.rs:249-254 |

## Coverage

Examined: every clause of FR-220-AC-14 and TC-665 step 13. The
implementation was read in `ordering_mismatch`,
`WeakAccessShape::ordering_mismatch`, `StateWeakAccesses::first_shape`,
`WeakAccessShape::from_failures` and `state_access_shape`.

A reviewer probe confirmed that every case is `Holds` under `sc` and that the
acq_rel RMW is `Holds` under `tso` and `ra`.

Out of scope, noted for context only: `from_failures` orders entries without a
transition (node readers) after every attempt entry. FR-220's Reporting rule
orders a node reader by its event transition identity. That is FR-220-AC-15
territory and was not judged here.
