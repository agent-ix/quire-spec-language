---
id: SR-13650
title: 'Spec review of PR #1138: QSpec consumer alignment, EARS conformance'
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@fd5cd6d5e; git diff origin/main...fd5cd6d5e (PR #1138, branch qsl-spec-std-consumers, base main 5c4a96692): FR-057, FR-125, FR-134, FR-146, FR-153, FR-161, FR-218, FR-314, FR-337, TC-535, TC-893, TC-1056, TC-1079; read against origin/main code (qsl-route/src/lib.rs, qsl-route/tests/it/fairness_kinds.rs, qsl-semantics/src/check/protocol_requirement.rs, protocol_relationship.rs, refusal.rs, qsl-cst/src/grammar.rs, qsl-replay/src/certificate.rs), QSL FR-136, FR-315, FR-338, and quire-specification main fd7a694 (FR-468, FR-375) and open PRs #313 (9f0d0c8) and #314 (7722857); tickets STD-197, STD-193, STD-194, STD-186, STD-217, QSL-877"
review_set: subset
date: '2026-10-21'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-057
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-125
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-134
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-146
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-153
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-161
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-218
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-314
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-337
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-535
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-893
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-1056
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-1079
  type: reviews
---

## Summary

Ticket: STD-197 (also STD-193, STD-194, STD-186, STD-217, QSL-877). PR quire-spec-language#1138, head fd5cd6d5e. EARS lens: one SHALL per rule, a named subject, and no status wording in the new and edited Behavior statements.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-134 admission rule | spec/functional/FR-134-advertise-the-fairness-kinds-en-1-decides.md | examined |
| FR-134 precedence | spec/functional/FR-134-advertise-the-fairness-kinds-en-1-decides.md | examined |
| FR-134 empty list | spec/functional/FR-134-advertise-the-fairness-kinds-en-1-decides.md | examined |
| FR-134 non-temporal | spec/functional/FR-134-advertise-the-fairness-kinds-en-1-decides.md | examined |
| FR-146 safety half | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-125 one instance | spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md | examined |
| FR-218 requirement binding | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md | examined |
| FR-218 not emitted | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md | examined |
| FR-218 wrong-export | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md | examined |
| FR-314 refinement shape | spec/functional/FR-314-check-an-smt-proof-certificate.md | examined |
| FR-314 encode | spec/functional/FR-314-check-an-smt-proof-certificate.md | examined |
| FR-057 registration causes | spec/functional/FR-057-admit-shared-capability-kinds.md | examined |
| FR-337 over node | spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md | examined |
| FR-337 reader over | spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-146's safety-half bullet has no SHALL. 'The refinement's safety half combines its records' settlements as QSpec FR-468 states: ...' is a Behavior rule with no SHALL and no subject that does the combining. Write it as 'QSL SHALL settle the refinement's safety half ...' (see integrity FND-001 for the content), or move it to Dependencies as a pointer to QSpec FR-468. | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md:167-172 |
| FND-002 | low | FR-218's not-emitted sentence has no SHALL and carries status wording. 'A clause of a kind S4 does not emit yet refuses the unit at that clause (unsupported_construct/not-yet-implemented) and produces no package, so no package holds a requirement bound to a clause with no node.' has no SHALL and 'does not emit yet' is a statement of current build status. State the rule: 'S3 SHALL refuse a temporal clause of a kind FR-337 does not emit with unsupported_construct/not-yet-implemented at that clause', naming the kinds by FR-337's emitted set rather than by 'yet'. | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:265-269 |
| FND-003 | low | FR-125's temporal-clause sentence has no SHALL. 'A temporal clause always names an over parameter (QSpec FR-370-AC-13); written without one it refuses invalid_syntax/unexpected-token ... and QSL produces no node.' states a refusal without a SHALL or a subject (S1). Either 'S1 SHALL refuse ...' or reduce it to a pointer to FR-337, which already states the rule. | spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md:162-165 |
| FND-004 | low | FR-134's empty-list rule lost its SHALL. 'A present empty list is admitted and advertises no fairness kind, so the backend is a candidate only for an item whose fairness set is empty.' was a clause of a SHALL bullet before the PR and is now a standalone bullet with no SHALL. Write 'The registry SHALL admit a present empty list ...'. | spec/functional/FR-134-advertise-the-fairness-kinds-en-1-decides.md:102-103 |
| FND-005 | low | FR-314's base-position and QueryPart sentences have no SHALL. In the new RefinementInductive bullet, 'A step part is never accepted in the base position.' and '`QueryPart::Step` (FR-338) carries the row index for this variant and none for `Inductive`.' are normative statements with no SHALL; the first restates the ShapeMismatch-at-Base rule and the second is a type definition that belongs in the Inputs code block (see integrity FND-003). Drop the first and move the second. | spec/functional/FR-314-check-an-smt-proof-certificate.md:100-102 |

## Verdict

Mergeable on this lens once the five sentences carry a SHALL or become pointers; each is low. The other new and edited Behavior bullets (FR-134 rule and precedence, FR-218 binding, FR-314 encoding) have one SHALL per rule and name their subject.
