---
id: SR-20261104025136
title: "Gap analysis — QSL-1580 TC-1308 carrier gaps (PR #2243)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@c11158986; PR #2243 diff; FR-513-AC-4, FR-513-AC-7, FR-046-AC-2, FR-046-AC-8"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-513
    type: reviews
---

## Summary

Ticket: QSL-1580. Plan completion: not assessed. Quoin 0.29.1. The trace was checked by reading the `#[trace]` tags on the new tests.

FR-513-AC-4 has the following tagged, asserting spine tests:

- measured self and mutual recursion admits;
- a non-decreasing self measure refuses with obligation `decrease`;
- an unguarded rational divisor refuses at `D` (pre-existing);
- a guarded rational divisor admits through two callers;
- the adverse guards `x = 0r and`, `x != 1r and` refuse;
- the `else` of `x = 0r` admits.

FR-513-AC-7 / FR-046-AC-8 has a test for both stage labels and both reader refusals.

There are no stubs and no dead code. The new checker arm is reached by these tests. FR-513-AC-3 and AC-5 remain open and are owned elsewhere (STD-651, STD-652 → IR-1313), as the PR states. Those are accurate deferrals, not gaps in this diff.

## Verdict

**MERGEABLE** for coverage of what the diff claims. The test-strength nits are in the code-review SR, and the AC-4 integer-divisor narrowing is in the spec-review SR.

## Scope

- FR-513-AC-4: examined.
- FR-513-AC-7: examined.
- FR-046-AC-2, FR-046-AC-8: examined via trace tags.
- FR-513-AC-3, FR-513-AC-5: context only (deferred to other owners).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
