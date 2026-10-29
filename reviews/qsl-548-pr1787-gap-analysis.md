---
id: SR-1787002
title: "gap analysis of PR #1787 (QSL-548 slice A): FR-146 step simulation records"
type: SpecReview
analysis: gap-analysis
date: 2026-10-28
scope: "agent-ix/quire-spec-language; PR #1787 at 39c8b047f against FR-146-AC-1..3, 5, 6, 9, 10 (STD-556)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: reviews
---
# SR-1787002: gap analysis of PR #1787 (QSL-548 slice A)

## Summary

These are covered (TC-551):
- AC-1: eight records; none with `any`.
- AC-2: hidden field; history in qsl-replay.
- AC-3: `CasInv` hypothesis.
- AC-5: model and protocol row order, reversal-invariant.
- AC-6: `AbsInv` is no hypothesis.
- AC-9: occurrence keys.
- AC-10: the `Impl::counters` domain, a bound gives `Bounded`, `audits` adds none, `peek` parameter and result domains, an ambiguous `spare` refuses every row.

Gaps:
- **FND-001.** No test that a population reaches a `Step` record through a contract clause (a `reaches`, or an FR-391-named population, in `pre`/`post`), or through a `reaches` in the state mapping. These are the extent rules beyond the context population, and the ones that could under-approximate. AC-10 tests only that an unnamed population adds nothing.
- **FND-002.** No test of the ambiguity refusal for a mapped concrete type with no operation row, which refuses at the refinement name.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No test of a contract-clause population (reaches or FR-391-named) or a state-mapping `reaches` population entering a `Step` record | FR-146 Claim extent, FR-146-AC-10 |
| FND-002 | low | No test of the ambiguous mapped-type refusal at the refinement name | FR-146 Claim extent |

## Verdict

Mergeable. Both findings are low.
