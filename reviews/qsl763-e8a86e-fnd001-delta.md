---
id: SR-7835-D1
title: "Exact-head delta review — QSL-719 pre-charge subtype closure"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@e8a86e77a3b2ed7b018e8ba1afc662734e13d7cf; PR #763; delta from prior SR-7835/SR-7836 at c1b557cdcf1ef66d0513903925899a308f30a4bc"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-084
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-090
    type: reviews
---

## Summary

Bounded static exact-head re-review of the FND-001 fix and the prior review's material concerns. No build, test, gate, implementation edit, commit, push, or merge was performed.

## Verdict

**PASS — FND-001 resolved at the reviewed head.** This is a review disposition only; it does not claim a gate result.

## Delta evidence

- `qsl-semantics/tests/it/model_population.rs:580-598` repeats the incomplete-population query with `ScalarLimits { work_units: 0, ..SCALAR_UNLIMITED }`, requires `IncompleteSubtypeClosure` with queried type `A` and missing witness `[A]`, then asserts consumed work is zero and the admitted charge-point list is empty. The fixture has one admitted member, so charging its first `PopulationVisit` would be denied and change the result to scalar incompleteness; the assertions discriminate that regression.
- `qsl-semantics/src/model/population.rs:2154-2175` establishes coverage before entering the member loop. `PopulationVisit` charging begins at lines 2177-2182, only after successful coverage.
- `qsl-semantics/src/model/accounting.rs:513-548` computes candidate counters and returns on a denied size/work limit before mutating consumed counters, admission count, or test-support charge list. Thus a zero-work first charge cannot leave a partial accounting mutation.
- `spec/test-cases/TC-227-allinstances-requires-both-closures.md` names the zero-work pre-charge assertion under the existing semantic binder.

## Prior concerns rechecked

The exact PR summary and full diff remain consistent with the earlier accepted concerns: subtype coverage uses `ModelIndex::conforms` against retained declared member types; no retained descendants relation, quadratic materialization, or whole-catalog walk is added; incomplete closure returns the singleton witness `[T]` and no collection; CLI rendering, query propagation, canonical order, and the four closure direction vectors remain in scope. The QSpec #233 no-new-charge-row alignment remains stated in the PR body and FR-084.

No new finding was identified in this bounded delta. The prior quire-matrix timeout remains unresolved and is not represented as a pass. No build/test/gate result is inferred.
