---
id: SR-14095
title: "gap analysis of STD-248 changed ACs against TC rows (PR #1196, QSpec PR #335)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@060cff6e3fa4f10da34432d2e2c49f9ad3c3bc03; spec/functional/FR-221-explore-the-x86-tso-memory-model.md, spec/functional/FR-222-explore-the-release-acquire-memory-model.md, spec/test-cases/TC-666-x86-tso-memory-model.md, spec/test-cases/TC-667-release-acquire-memory-model.md; agent-ix/quire-specification@f605fded924fd505168e61ed772de5f609c2b11b; spec/functional/memory/FR-436-explore-total-store-order.md, spec/test-cases/TC-390-store-buffering-under-each-model.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-221
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-222
    type: reviews
---
# SR-14095: gap analysis of STD-248 (PR #1196, QSpec PR #335)

## Summary

Ticket: STD-248. Plan completion: not assessed. This is a manual mapping.

- FR-221-AC-1 (changed) maps to TC-666 step 1 ("§10's four-step
  counterexample"), which still holds.
- FR-222-AC-1 (changed) maps to TC-667 step 1, which refers to section 10's
  table and still holds.
- QSpec FR-436-AC-1 (changed) maps to TC-390 SB-02, which is updated. TC-390
  SB-03 is also updated.

The existing tests check less than the ACs claim. The `tso` test
(`weak_memory.rs`) compares the steps as a set, and the `ra` test
(`release_acquire.rs`) checks depths and walks the old order as a path. The
PR's Remaining work lists the order-asserting test for `ra`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Clean. Every changed AC has a TC row.
