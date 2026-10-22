---
id: SR-12330
title: "Spec review of the FR-357 claim-identity edit in quire-spec-language PR #1216 (QSL-973)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@1af39d30e; spec/functional/FR-357-replay-scalar-parity-claims.md"
review_set: subset
---
# Spec review of the FR-357 edit in PR #1216

## Summary

Ticket: QSL-973. The only spec edit in this PR is in FR-357 Behavior. The
function claim identity now carries "the `function` replay target as the
request sent it (absent when it carried another target, which the entry
refuses)". The code agrees: `ValueIdentity.function: Option<QualifiedName>`
takes the wire's `function` member. FR-357-AC-13, the `claim()` equality
statement, was also read as context.

## Verdict

The edit is sound and agrees with the code. There is one wording gap.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | "absent when it carried another target" does not cover a request carrying `function` and another member. The decode refuses it `invalid-value`, and the claim holds `Some(function)` because the value is copied from the wire's `function` member. A clearer wording is "the request's `function` member as sent, absent when the request carries none". | spec/functional/FR-357-replay-scalar-parity-claims.md:91-93 |

## Dispositions

| FND | outcome | sha |
| --- | --- | --- |
| FND-001 | fixed: FR-357 now reads the request's function member as sent, absent when the request carries no function member | b02604c70 |
