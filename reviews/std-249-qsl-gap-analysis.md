---
id: SR-14035
title: 'Gap analysis of PR #1191: FR-147 node-row argument count and types (STD-249), changed ACs against TC rows'
type: SpecReview
analysis: gap-analysis
scope: 'agent-ix/quire-spec-language@53cee90bd; git diff origin/main...53cee90bd (PR #1191, branch std-protocol-249, based on main, not stacked): spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md, spec/test-cases/TC-552-s3-checks-a-refinement-whose-abstract-side-is-a-protocol.md; ticket STD-249'
review_set: subset
date: '2026-10-22'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-147
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-552
  type: reviews
---

## Summary

Ticket: STD-249. PR: quire-spec-language#1191, head 53cee90bd. Method: gap analysis of the new AC against its TC rows, done by hand from the spec. Spec-only PR, so no matrix run and no cargo tests. Plan completion: not assessed. Tooling: quoin 0.28.3, quire 0.36.2 (engine 0.50.2), spec-artifacts-process@b82e42e.

FR-147-AC-14 (Test, TC-552) is in TC-552's scope (now AC-1 to AC-14), run against built by the build sentence, and covered by step 14 and its expected result: every row of the AC is checked in place of `commitA`'s row. The `a1()` row shares the defect recorded as SR-14033 FND-001. No test is tagged with AC-14 yet; the code work is QSL-887, so that is not a finding.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-147-AC-14 | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md:289 | examined |
| TC-552 scope | spec/test-cases/TC-552-s3-checks-a-refinement-whose-abstract-side-is-a-protocol.md:17-21 | examined |
| TC-552 step 14 | spec/test-cases/TC-552-s3-checks-a-refinement-whose-abstract-side-is-a-protocol.md:58-59 | examined |
| TC-552 expected step 14 | spec/test-cases/TC-552-s3-checks-a-refinement-whose-abstract-side-is-a-protocol.md:108-110 | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | TC-552's expected result for step 14 says the long lists refuse `operator-ineligible` and `a1(1)` refuses `type-mismatch`, but drops what FR-147-AC-14 requires them to retain: `expected` 1 and `actual` 3 for the count refusal, and `expected` and `actual` for the type refusal. A test written from TC-552 would not check the retained fields. Fix: add them to the expected result. | spec/test-cases/TC-552-s3-checks-a-refinement-whose-abstract-side-is-a-protocol.md:108-110 |

## Verdict

FR-147-AC-14 has its TC rows. One low finding: the expected result drops the retained fields the AC requires.
