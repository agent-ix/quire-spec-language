---
id: SR-20261104091646
title: "Gap analysis — QSL-1507 spine successor coverage"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; FR-036-AC-8, FR-040-AC-11, FR-048-AC-4, FR-513-AC-12; TC-114 row 7, TC-119 row 11, TC-1308 row 22; qsl-replay/src/spine/lifecycle/tests.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-036
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-040
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-048
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-513
    type: references
---

## Summary

Manually checked the changed AC-to-test-to-code paths and trace tags. Closure tests cover the new direct spine API; proof tests omit a substituted Handler-name case; and the TC-1308 row-22 test covers all eight choice-guard cases but not the repeat-guard behavior included in FR-048-AC-4.

## Verdict

**CONDITIONAL** — two tagged criteria are only partially exercised by the new spine successor tests.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new tagged test covers choice guards only, while FR-048-AC-4 also requires repeat guards. | qsl-replay/src/spine/lifecycle/tests.rs:682-687 |
| FND-002 | medium | The proof test rejects a non-Handler point but does not test a substituted Handler name, which the implementation accepts. | qsl-replay/src/spine/lifecycle/tests.rs:1214-1277 |

## Coverage

- FR-036-AC-8: selected ObservationBinding closure, direct finite-global dependency, wrong-kind separation, and omitted dependency refusal are tagged and exercised; the shared resolver is reused by profile closure.
- FR-040-AC-11: positive result plus missing, duplicate, foreign QSL source, foreign clause, foreign formal owner/source, and non-Handler execution point are exercised; the substituted Handler-name case is not.
- FR-513-AC-12 / TC-1308 row 22: four invisible choice-guard atoms are each tested directly and only in a skipped operand (eight refusals).
- Quire's whole-repository matrix was stopped after the orchestrator limited this review to changed scope (the repository contains 2,110 spec files; the scan was CPU-bound without output). The changed tags and criteria above were checked manually. No complete repository matrix verdict is claimed.
- Plan completion: not assessed.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | The added repeat test asserts `PostTestRepeat` construct refusal for a body-produced binder, which is not a visibility refusal. FR-218-AC-33 allows the same-owner completed-body binder; the test does not put an invisible atom in the repeat guard and assert the FR-218 visible-entry cause. |
| FND-002 | fixed | The proof test now substitutes `Handler { name: different_handler }` while retaining the expected `validate` handler and asserts typed `ExecutionPoint` refusal. |
