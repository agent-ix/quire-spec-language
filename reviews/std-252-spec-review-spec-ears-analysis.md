---
id: SR-13951
title: 'Spec review of PR #1186: quantile field and a buildable symmetry case (STD-252), EARS analysis'
type: SpecReview
analysis: ears-conformance
scope: 'agent-ix/quire-spec-language@91e502f0e; git diff origin/main...91e502f0e (PR #1186, branch std-qsl570-252): spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md, spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md; ticket STD-252'
review_set: subset
date: '2026-10-22'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-189
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-192
  type: reviews
---

## Summary

Ticket: STD-252. PR: quire-spec-language#1186, head 91e502f0e, based on main. Method: spec-review with the EARS sub-analysis, applied to the requirement statements the PR adds or changes. Tooling: quoin 0.28.3, quire 0.36.2 (engine 0.50.2), spec-artifacts-process@b82e42e. The `make check-spec-validation` EARS lints report no warning on either touched FR.

Statements examined: the FR-189 symmetry bullet with its added sentences; the FR-189 quantile bullet; the FR-189 least-favourable-test bullet (one clause changed); the FR-192 `estimate` bullet (one clause removed); and the FR-192 `quantile` bullet. The FR-192 `quantile` bullet and both changed clauses are single ubiquitous statements with a subject and a testable result.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-189 Symmetry bullet | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md:111-121 | examined |
| FR-189 quantile bullet | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md:153-159 | examined |
| FR-189 least favourable test bullet | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md:199-202 | examined |
| FR-192 `estimate` bullet | spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md:119-124 | examined |
| FR-192 `quantile` bullet | spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md:125-128 | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The added symmetry sentences end with a normative-sounding claim with no SHALL: "both conditions hold for every declaration EN-4 admits, and it runs one test per orbit". This is a derived consequence placed inside a conditional SHALL bullet, so the bullet reads as both "when invariant, SHALL reduce" and "always reduces". Split it: keep the SHALL as the requirement, and move the rationale into a Note that cites the rule that makes the condition always true (see SR-13950 FND-001). | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md:117-121 |
| FND-002 | low | The quantile bullet holds three SHALLs (set `quantile`, be `Censored`, have `quantile` absent) and an unmarked normative sentence, "The quantile never decides the test." These are three requirements in one bullet, and the last one has no SHALL. Split it into the computation, the Censored case and the absent case, and phrase the last as "The decision SHALL NOT read `quantile`" if it is meant to be normative. | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md:153-159 |

## Verdict

Mergeable on the EARS lens. Both findings are low and about wording and structure. The new statements have a clear subject, trigger and result, and the changed clauses are well formed.
