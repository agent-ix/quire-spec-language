---
id: SR-14092
title: "gap analysis of STD-240 new and changed ACs against TC rows (PR #1195, QSpec PR #334)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@f53be97e54425d2f247a90925a66fb4b5e3bc0a5; spec/functional/FR-222-explore-the-release-acquire-memory-model.md, spec/functional/FR-229-carry-the-memory-component-through-the-protocol-system.md, spec/test-cases/TC-667-release-acquire-memory-model.md, spec/test-cases/TC-674-memory-component-through-the-seam.md; agent-ix/quire-specification@40cc2f8b08f639c790fba9791b155a3810d59b40; spec/functional/memory/FR-435-classify-memory-accesses-and-orderings.md, spec/functional/memory/FR-437-explore-release-acquire-memory.md, spec/test-cases/TC-389-memory-access-classification.md, spec/test-cases/TC-393-memory-budgets-and-verdicts.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-222
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-229
    type: reviews
---
# SR-14092: gap analysis of STD-240 (PR #1195, QSpec PR #334)

## Summary

Ticket: STD-240. Plan completion: not assessed. This is a manual mapping of
each new or changed AC to its TC rows. The change is spec only; the code
lane adds the tests.

- FR-222-AC-3 (changed) maps to TC-667 step 3, which is not updated
  (FND-001).
- FR-222-AC-5 (new) maps to TC-667 step 5 and its expected result. Scope is
  now AC-1 to AC-5.
- FR-229-AC-6 (new) maps to TC-674 step 6 and its expected result. Scope is
  now AC-1 to AC-6.
- QSpec FR-437-AC-4 (changed) maps to TC-393 MB-07, which is updated.
- QSpec FR-435-AC-7 (new) maps to TC-389 AC-12. The row exists, but its
  negative clause cannot be reached (SR-14090 FND-003).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-222-AC-3's new clause ("whatever its acquire, release and message views and `S` point at") has no TC step: TC-667 step 3 still checks only the `fetch_add` and the join, and sets up no message whose message view or `S` still points at a removed one, such as `SB`'s `x0` once both `cur` views pass it. | spec/test-cases/TC-667-release-acquire-memory-model.md:23 |

## Verdict

Every new AC has a TC row. Only one changed clause is left without a
procedure step.
