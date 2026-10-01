---
id: SR-908
title: "QSL-336 base spec review of PR 541's FR-122 and TC-517 edits"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@ec9476fb2a8970754aaf6974647841a995991f32; spec/functional/FR-122-replay-a-state-clause-counterexample.md; spec/test-cases/TC-517-replay-a-state-clause-counterexample.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-122
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-517
    type: reviews
---
## Summary

Ticket: QSL-336 (code half). PR: quire-spec-language#541 at ec9476fb. This
is a new review set for PR #541, separate from PR #539's SR-824 to SR-828.

Examined: FR-122 Outputs (the rewritten second bullet), FR-122-AC-2 and
TC-517 Expected Results step 2. These apply the leader's ruling that FR-072
wins: an `inconclusive`, `Verdicts` result carries no FR-351 record because
its settlement basis is not decisive.

Checks:

- **Against FR-072.** FR-072 carries the nested FR-351 record "when the
  settlement basis is decisive". The new Outputs text gives the record only
  to a reproducing `Witness`-arm result and states that `Verdicts` carries
  the value with no record. They agree.
- **Inside FR-122 and TC-517.** AC-1 (record on reproduce), AC-2 (no record,
  evaluated `true`) and TC-517 step 2 now say the same thing. A spec-wide
  grep finds no other statement that an inconclusive result carries a
  deciding element of `true`.
- **Against FR-098-AC-2.** FR-098-AC-2 gives the record only to an agreeing
  `Witness` replay, which is consistent.
- **Form rule.** FR-122 Behavior refuses `Invocation` for a precondition
  while FR-106 check 2 admits it. This is the leader's ruling, and the two
  requirements state different scopes (replay versus admission), so it is
  not a contradiction. Not a finding.
- **Statement quality.** The edited bullet is in EARS-compatible
  declarative form and the AC is testable. The tests assert
  `record().is_none()` and `value() == Some(true)`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Clean. The edits are minimal and match FR-072 and FR-098. FR-122 and
TC-517 are internally consistent after the change.
