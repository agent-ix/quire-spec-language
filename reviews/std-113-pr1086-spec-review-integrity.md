---
id: SR-13423
title: 'Spec review of PR #1086: QSL cites the Quantity<u> type reference, integrity'
type: SpecReview
analysis: integrity
scope: 'agent-ix/quire-spec-language@626b08692; git diff refs/remotes/origin/main...626b08692 (PR #1086, base main 812c95430): spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md, spec/functional/FR-098-execute-a-replay-request.md, spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md, spec/test-cases/TC-620-s3-admits-random-parameters-workloads-and-rewards.md, spec/test-cases/TC-906-composite-arguments-replay-and-refuse-by-kind-domain-and-limit.md; read against FR-056, ADR-024 7.2 and the TC-620 test fixture on the head, QSpec PR #296 (agent-ix/quire-specification, head f789815): spec/functional/type-model/FR-142-evaluate-quantities-and-units.md, spec/objects/interfaces/FR-322-checked-package-artifact.md, spec/test-cases/TC-187-quantity-and-unit-conversion.md, proposals/quire-v1/shared-grammar.md, QSpec FR-405; ticket STD-113'
review_set: subset
date: '2026-10-20'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-185
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-620
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-091
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-098
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-906
  type: reviews
---

## Summary

Ticket: STD-113. PR: quire-spec-language#1086, head 626b08692, scoped to `git diff refs/remotes/origin/main...626b08692` (five files, +19/-16). Spec-only. It cites QSpec PR #296's `Quantity<u>` type form: FR-185's 'Quantity-typed parameters' section types a quantity operation parameter as `Quantity<u>` with quantity-literal supports, keeps integer supports for the `Millis = Int[1, 12]` fixture, and adds FR-185-AC-8 and TC-620 step 8 (waiting on QSL intake); FR-091's type-form lookup paragraph and OQ-11(d) now name `Quantity<u>`; FR-098 and TC-906's dependency notes cite QSpec FR-142. Integrity review: traceability (TC-620 scope and steps against FR-185's ACs), relationships, and the consistency of the dependency notes and the OQ-11 ruling row with the new text. TC-620's Scope now runs to FR-185-AC-8 and step 8 and its expected result trace AC-8; TC-620 `verifies` FR-185. Gates on the head: make check-spec-validation exits 0 with 17 warnings, none in the changed files. tools/check-index-completeness.sh passes. quoin 0.28.3, quire 0.36.2 (engine 0.50.2). Methods: spec-review/base (SR-13421), spec-review/ears (SR-13422), spec-review/integrity (SR-13423). code-review and gap-analysis were not run, because no code or test file changed.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| TC-620-scope | spec/test-cases/TC-620-s3-admits-random-parameters-workloads-and-rewards.md | examined |
| TC-620-step-8 | spec/test-cases/TC-620-s3-admits-random-parameters-workloads-and-rewards.md | examined |
| FR-185-relationships | spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md | examined |
| FR-091-OQ-11 | spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md | examined |
| FR-091-dependency-STD-113 | spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md | examined |
| FR-098-dependency-note | spec/functional/FR-098-execute-a-replay-request.md | examined |
| TC-906-dependency-note | spec/test-cases/TC-906-composite-arguments-replay-and-refuse-by-kind-domain-and-limit.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-091 still lists 'STD-113 (a quantity type reference in complete-V1 source)' as an open dependency, and OQ-11's Reopen cell still reads 'QSpec publishes ... STD-113's quantity type reference', the fact the new ruling (d) now relies on. Fix: cite QSpec FR-142 (Quantity type reference) in the Dependencies bullet, and give OQ-11(d) a reopen condition such as 'QSpec changes or withdraws `Quantity<u>`'. | spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md:1001 |
| FND-002 | low | FR-185, FR-091, FR-098 and TC-906 cite QSpec FR-142 for the behaviour they now depend on, but none has a relationship to ix://agent-ix/quire-specification/FR-142 (FR-185 depends_on only QSpec FR-272). Fix: add `depends_on ix://agent-ix/quire-specification/FR-142` to FR-185 and FR-091 (and FR-098 if its relationships list QSpec dependencies), so the link graph shows the dependency the prose states. | spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md:1-13 |

## Verdict

Mergeable from the integrity side once the two low findings are fixed; TC-620's traceability to FR-185-AC-8 is complete. The PR as a whole waits on SR-13421.
