---
id: SR-13422
title: 'Spec review of PR #1086: QSL cites the Quantity<u> type reference, EARS'
type: SpecReview
analysis: ears-conformance
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

Ticket: STD-113. PR: quire-spec-language#1086, head 626b08692, scoped to `git diff refs/remotes/origin/main...626b08692` (five files, +19/-16). Spec-only. It cites QSpec PR #296's `Quantity<u>` type form: FR-185's 'Quantity-typed parameters' section types a quantity operation parameter as `Quantity<u>` with quantity-literal supports, keeps integer supports for the `Millis = Int[1, 12]` fixture, and adds FR-185-AC-8 and TC-620 step 8 (waiting on QSL intake); FR-091's type-form lookup paragraph and OQ-11(d) now name `Quantity<u>`; FR-098 and TC-906's dependency notes cite QSpec FR-142. EARS review of the changed statements: FR-185's 'Quantity-typed parameters' section, FR-185-AC-8, TC-620 step 8, FR-091's type-form lookup paragraph and OQ-11(d). The FR-185 section and the FR-091 paragraph are descriptive prose under rules stated with SHALL elsewhere (FR-185 Random parameters, FR-091's refusal list), as they were on main; the refusal they name is the existing `If a value is not of the parameter's type, S3 SHALL refuse ...` rule. AC-8 is a direct assertion of the outcome. The grammar lint raises no warning in the changed files. The build-log wording is recorded under SR-13421 FND-006. Gates on the head: make check-spec-validation exits 0 with 17 warnings, none in the changed files. tools/check-index-completeness.sh passes. quoin 0.28.3, quire 0.36.2 (engine 0.50.2). Methods: spec-review/base (SR-13421), spec-review/ears (SR-13422), spec-review/integrity (SR-13423). code-review and gap-analysis were not run, because no code or test file changed.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-185-quantity-section | spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md | examined |
| FR-185-AC-8 | spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md | examined |
| TC-620-step-8 | spec/test-cases/TC-620-s3-admits-random-parameters-workloads-and-rewards.md | examined |
| FR-091-type-form-lookup | spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md | examined |
| FR-091-OQ-11 | spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md | examined |
| FR-185-random-parameters-value-rule | spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Clean for EARS. No defect in the changed statements' requirement grammar; the substantive findings are in SR-13421 and SR-13423.
