---
id: SR-12914
title: "Integrity review of STD-189 (PR #1002): FR-381 control-tree dominance against AC-11, TC-1058 and QSpec"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@2e8dd2460; spec/functional/FR-381-check-protocol-captures-activation-and-compensation-recovery-at-s3.md Compensation templates commit bullet and AC-11; spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md Choice and repeat decisions and Admission for emission; spec/test-cases/TC-1058-s3-checks-compensation-registration-retry-commit-and-recovery.md rows 19-20b; spec/test-cases/TC-1056-s3-checks-channels-relationships-requirements-and-choice-visibility.md row 45; agent-ix/quire-specification@ad2b37e proposals/quire-v1/shared-grammar.md Precedence and bindings, join-policy"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-381
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1058
    type: reviews
---
# SR-12914: integrity review of STD-189 (PR #1002)

## Summary

Ticket: STD-189. This review checks that the new FR-381 text is internally consistent and that it agrees with AC-11, with TC-1058, and with the QSpec grammar and join semantics.

No sentence still defines dominance over executions. The old "executions can reach the effect" admitted-list wording is replaced by the sibling-branch clause, and AC-11 now says "on the control tree". AC-11's refuse and check lists match the FR clause by clause: `join all`, `quorum(2) [a, b]` and `join any [a]` refuse; a sibling branch (with the send/receive parenthetical), `join any [a, b]`, `quorum(1) [a, b]`, a `repeat` body and one `case` of a `choice` check. The join classification agrees with FR-218's admission rules: `quorum(n)` counts distinct labels, and a label listed twice is one label. The `every set of completed branches` test agrees with QSpec FR-426, whose join fires over `completed` dispositions, so the recursion is sound. FR-218's `not (got.count > 0)` matches shared-grammar 'Precedence and bindings': `not a = b` means `(not a) = b`. The FR-218 atom-identity rule still holds, since parentheses do not change the checked term.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | 'a commit in the body of a `repeat` before the effect' and 'a commit in one `case` of a `choice` before the effect' (AC-11, and the FR's closing admitted list) can be read as covering an effect later in the same body or case. The control-tree rule refuses that case, because the body's or case's own `sequence` orders the commit before the effect. Rows 19e and 20a put the effect after the `repeat` or `choice`, so the tests are right. Only the prose is ambiguous. Write 'with the effect after the `repeat`' and 'with the effect after the `choice`', as the `join any [a, b]` clause already does. | spec/functional/FR-381-check-protocol-captures-activation-and-compensation-recovery-at-s3.md:282 |

## Verdict

Consistent apart from the low wording ambiguity in FND-001, which should land in the same fix round as SR-12913 FND-001.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | 4c8265a4a: AC-11 now reads 'a commit in the body of a `repeat` with the effect after the `repeat`, and a commit in one `case` of a `choice` with the effect after the `choice`'. The FR's admitted list reads 'one `case` of a `choice` with the effect after the `choice`', and row 20a states the effect position. (FR-381:254-255, 284; TC-1058:62) |
