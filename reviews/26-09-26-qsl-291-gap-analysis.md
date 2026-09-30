---
id: SR-731
title: "QSL-291 gap analysis of PR 488 (FR-100/FR-106 follow catalog 1-draft.8)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-100-run-a-named-function-through-the-spine.md; spec/functional/FR-106-admit-snapshots-and-invocations.md; spec/test-cases/TC-452-spine-run-entry-lives-in-qsl-replay.md; spec/test-cases/TC-465-admission-refuses-each-input-defect.md; qsl-replay/src/spine/call.rs; qsl-replay/src/spine/call/tests.rs; src/command/output.rs; qsl-eval/src/value/expression/evaluate.rs; quire-exact/src/outcome.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: reviews
---

## Summary

Ticket: QSL-291 (PR agent-ix/quire-spec-language#488). Spec-only diff, so
this analysis checks that each changed acceptance criterion has a test case
with a checkable oracle, and lists the code and tests that still assert the
old text. FR-100-AC-9 is covered by TC-452 step 4; FR-106-AC-3 and AC-7 by
TC-465 rows 8 and 42. FR-100's and TC-452's status sections correctly say
the new rows are specified and not implemented.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | TC-452 step 4 constructs the ten new kernel refusals without stating their payloads (target domain, IEEE width), and the expected result checks only "the field names" for them. A test can pass with wrong `expected`/`actual` values. Give each constructed refusal a concrete payload (for example `IntegerOutOfDomain` with `Int[0, 9]`, `IeeeNotExact` with `binary64`) and state each exact `fields` object, as the `CardinalityOutOfBound` and `ForeignReference` rows do. | spec/test-cases/TC-452-spine-run-entry-lives-in-qsl-replay.md:54-56, 102-107 |
| FND-002 | low | Code and tests still assert the old text; the spec now requires changes (deferred to a code follow-up, not blocking this spec PR). (a) qsl-replay/src/spine/call/tests.rs:433 asserts the ten refusals render bare `Kernel { location }`; must assert each record's code, cause and fields. (b) src/command/output.rs:678 lists four kernel undefined reasons; must add `sum-out-of-domain`. (c) src/command/output.rs:806 tests `CallRefusal::Kernel` rendering `{"kind":"refused"}`; the no-record kernel row is gone, so the test and the `CallRefusal::Kernel` variant (qsl-replay/src/spine/call.rs:129, :511) go. (d) qsl-eval/src/value/expression/evaluate.rs:1492 adds with `sum_domain`, refusing `IntegerOutOfDomain` for a running total, and does not check the seed; must return `Undefined::SumOutOfDomain` at the summand (seed) or `sum` node (addition). (e) evaluate.rs:1581 refuses `IntegerOutOfDomain` for the final total; must go (subject to SR-730 FND-001 for `n = 0`). Also required: `quire_exact::Undefined::SumOutOfDomain`, and payloads on the ten `Refusal` variants. | qsl-replay/src/spine/call/tests.rs:433; src/command/output.rs:678; src/command/output.rs:806; qsl-eval/src/value/expression/evaluate.rs:1492; qsl-eval/src/value/expression/evaluate.rs:1581 |

## Verdict

Approve with changes. Fix FND-001 in this PR. FND-002 is deferred to a code
follow-up ticket under QSL-291's parent.

## Dispositions

Round 2.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | TC-452 step 4 gives each of the ten refusals a concrete payload and step 4's expected results give each exact `fields` object |
| FND-002 | deferred | QSL-292, a code ticket blocked by QSL-291 |
