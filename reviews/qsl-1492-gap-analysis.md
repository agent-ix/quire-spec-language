---
id: SR-2271002
title: "Gap analysis: QSL-1492 FR-146-AC-9/AC-15 and FR-149-AC-13/AC-27 (PR #2271)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@95585e8a8; PR #2271 diff only (qsl-semantics/src/check/facts.rs, termination.rs, type_form.rs, assemble/tests.rs; qsl-eval/tests/it/removal_filter_decrease.rs, float_rounding.rs, float_callables.rs, equality_matrix.rs)"
review_set: subset
---

## Summary

Ticket: QSL-1492. A planless gap analysis, scoped to the PR diff, that compares each touched QSpec AC (cited by ID) with its tagged tests and code. Plan completion was not assessed.

- FR-146-AC-15 is tagged on `a_removal_filter_under_a_membership_fact_decreases_a_cardinality_measure`. The test covers four collection kinds, the three equality shapes, a same-kind convert, `not contains(d, x)`, the lexicographic earlier-equal rule, five decrease refusals and two measure-kind refusals. The `not bitIdentical(x, e)` float clause is untested because the kernel refuses float collection elements (IR-1320).
- FR-146-AC-9 is left untagged on purpose. The new test is tagged TC-191 and covers the literal-divisor clauses and their refusals. The negation clauses wait on IR-1318.
- FR-149-AC-13: the new rounding-mode ordering test is added, and x05 is now tagged. Together they cover unlike widths, float against exact, unlike rounding modes and the IEEE comparison vectors.
- FR-149-AC-27: the new type_form test checks the exact membership values and the four malformed bound pairs, both bare and as a quantity carrier.

## Verdict

**MERGEABLE.** No code is left without an owning requirement and no test is a stub. One low trace inconsistency is recorded.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-146-AC-15 is tagged in full although its `not bitIdentical(x, e)` float clause is neither implemented nor tested (IR-1320). The PR leaves FR-146-AC-9 untagged for the same partial-coverage reason. The matrix will show AC-15 as covered while one clause is open. Fix: keep the tag and make sure QSL-1492/IR-1320 tracks the float clause, or drop the AC tag until IR-1320 lands, to match AC-9. | qsl-eval/tests/it/removal_filter_decrease.rs:100 |
