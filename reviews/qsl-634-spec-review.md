---
id: SR-1341
title: "Spec review of quire-spec-language PR #640: FR-106 and FR-122 post-state range rule (QSL-634)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@6a09ff93786ec1607cd08af6ccc79e54cf991c81; spec/functional/FR-106-admit-snapshots-and-invocations.md (check 6.5, AC-12), spec/functional/FR-122-replay-a-state-clause-counterexample.md (behaviour, AC-7), spec/test-cases/TC-465-*.md, spec/test-cases/TC-517-*.md, spec/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-122
    type: reviews
---
# Spec review of quire-spec-language PR #640

## Summary

Ticket: QSL-634. Reviewed head 6a09ff93, against the ruling on QSL-634.
FR-106 check 6.5 states the inputs/outputs rule clearly. It keeps pre-state
and arguments refused, and keeps wrong-kind and non-canonical spellings
refused. FR-122-AC-7 is concrete and testable. The TC-465/TC-517 scope lines
and spec/tests.md rows are consistent with the new ACs.

## Verdict

Changes requested: FND-001 is a contradiction between two FR-122 SHALLs.
FND-002 writes a deviation from the ruling into the spec. FND-003 is a claim
the code does not meet in every case.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Two FR-122 SHALLs cover the same case and settle it differently. The new bullet says a range violation with a completed value reproduces "whatever the clause evaluated", including `true`. The unchanged next bullet says "If the evaluation completes `true`, then the executor SHALL settle `inconclusive` with cause `Verdicts`", with no exception for a range violation. Fix: qualify the `true` and `false` bullets with "and the post snapshot holds no range violation", or state that the range bullet takes precedence. | spec/functional/FR-122-replay-a-state-clause-counterexample.md:168-181 |
| FND-002 | medium | FR-122 settles the range violation only "and the evaluation completes a value". The no-value case therefore stays `inconclusive`/`NoValue`, or becomes an `InternalFault` refusal when a typed op faults on the exact value. The QSL-634 ruling makes the out-of-range value itself the witness and settles reproduced/violated, with no condition on the clause's evaluation. This narrows the ruling in the spec without a ruling of its own (see SR-1339 FND-002). | spec/functional/FR-122-replay-a-state-clause-counterexample.md:168-179 |
| FND-003 | low | FR-106 check 6.5 ends "A clause that reads the field evaluates over the exact integer." That holds for ordering, arithmetic and equality. A clause that passes the field to a typed op that checks its static range (a coercion, or anything still raising `CheckedInvariant`) does not evaluate: it refuses or faults. Either make the statement true or narrow it to what the evaluator actually does. | spec/functional/FR-106-admit-snapshots-and-invocations.md:285 |

## Dispositions

Round 1, reviewed at b2c7911504123cbd65bb139209b9736f9cddf675 (fix commits 9869ac332, 78edbf4b3, b2c791150, rebased onto origin/main; compared by content).

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 78edbf4b3 |
| FND-002 | fixed | 78edbf4b3 |
| FND-003 | fixed | 78edbf4b3 |
