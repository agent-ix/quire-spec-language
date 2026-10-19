---
id: SR-12907
title: "Integrity review of STD-184 (PR #998): FR-218 decision facts and partition against QSpec FR-466 and FR-053"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@bc002a60e; spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md Choice and repeat decisions, AC-2, AC-17, AC-19; spec/test-cases/TC-1056-s3-checks-channels-relationships-requirements-and-choice-visibility.md rows 28-49; agent-ix/quire-specification@ad2b37e spec/objects/interfaces/FR-466 Decision facts and Binder scope, FR-464 Reader order"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-218
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1056
    type: reviews
---
# SR-12907: integrity review of STD-184 (PR #998)

## Summary

Ticket: STD-184. This review checks that the FR-218 edits are consistent within FR-218 and with QSpec. The new decision-fact definition matches QSpec FR-466 (no Boolean restriction). Dropping the `finish` and compensation binders from the `never` row is consistent with QSpec FR-466, which keeps them in its `never` row but runs the scope check first, so both texts give the same refusal. The stated refusal order matches QSpec FR-464 'Reader order' (`over`/model-state, then binder scope, then decision facts) and FR-466 ('which precedes this check'). The one inconsistency is the partition rule.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The PR adds 'The partition of a `choice` treats each comparison or Boolean fact a guard reads as an independent atom' but leaves the later bullet 'The partition of a `choice` SHALL be proved over the visible facts as independent Booleans (QSpec FR-053)'. With an individual entry `got.count` the visible fact is not a Boolean, so the two sentences name different variables for one proof. Neither says when two comparisons are the same atom: row 45 relies on `not got.count > 0` negating `got.count > 0`, but `got.count <= 0`, `0 < got.count` or `got.count >= 1` would be separate atoms under a syntactic reading and the same proposition under a normalising one, so two implementers would accept and refuse different exhaustive choices. Restate the partition bullet over the guard atoms, say what makes two atoms equal (for example syntactic identity of the checked comparison term, with `not` the only negation recognised), and that the check is therefore conservative over arithmetic relations between atoms. | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:310-313,325-327 |

## Verdict

Fix FND-001 before merge. Everything else examined is consistent: the decision-fact definition, the two entry kinds, the reduced eligibility table and its new note, and the refusal order agree with QSpec FR-466 and FR-464, and TC-1056 rows 45 to 49 follow from the FR text.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | The partition bullet is restated over guard atoms, atom identity is syntactic with `not` the only negation, and the contradicting sentence in the earlier bullet is removed. AC-2, AC-19 and TC-1056 rows 37-40 and 45 agree with it. (a17a5c9d6, FR-218:323-330) |
