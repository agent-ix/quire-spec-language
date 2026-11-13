---
id: SR-2694002
title: "ears-conformance review of FR-576 no-result-type body refusal (PR 2694)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language; PR #2694 diff origin/main...HEAD: spec/functional/FR-576-emit-the-operation-body-into-checked-packages-for-backend-consumers.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-576
    type: reviews
---

## Summary

Ticket: QSL-1899. This review checks EARS grammar for the one requirement
statement the PR adds: the FR-576 Behavior bullet at lines 92-99 ("While an
operation declares no result type, ..."). The bullet uses the state-driven
pattern, which suits the condition. Its second sentence ("The emitter SHALL
NOT withhold ...") has the right subject. `quire validate` reports no EARS
warning on the file.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The first SHALL has the wrong subject. "an `operation-body` resolved to it SHALL refuse the compile" makes the body the actor, where EARS needs the system (`check` / the S3 checker) as subject, as the sibling bullet at line 83 does. The sentence also carries its rationale after a colon inside the requirement. Suggested form: "While an operation declares no result type, `check` SHALL refuse an `operation-body` resolved to it as `ill_typed`/`type-mismatch` ...". Move the FR-094 rationale to a separate sentence. | spec/functional/FR-576-emit-the-operation-body-into-checked-packages-for-backend-consumers.md:92-97 |

## Verdict

PASS with one low finding. The statement is state-driven and singular, and
its outcomes can be tested. Only the subject of the first SHALL needs
rewording.
