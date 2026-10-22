---
id: SR-14075
title: "gap analysis of STD-247 new ACs against TC rows (PR #1194, QSpec PR #333)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@a3c2673b34bb35ab546454473df4188732a87c3d; spec/functional/FR-223-order-seq-cst-events-by-the-rc11-partial-sc-order.md, spec/test-cases/TC-668-rc11-partial-sc-order.md; agent-ix/quire-specification@e69cade8882527e8f53844c9fc14e5b70116ff90; spec/functional/memory/FR-437-explore-release-acquire-memory.md, spec/test-cases/TC-391-litmus-conformance-vectors.md, spec/test-cases/TC-393-memory-budgets-and-verdicts.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-223
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-668
    type: reviews
---
# SR-14075: gap analysis of STD-247 new ACs against TC rows

## Summary

Ticket: STD-247. Plan completion: not assessed. This is a manual mapping, spec only. FR-223-AC-9 maps to TC-668 step 8 (LT-17 in every interleaving). FR-223-AC-10 has no TC-668 step of its own (FND-001). FR-437-AC-12 maps to TC-391 LT-17, and FR-437-AC-13 maps to TC-393 MB-12. TC-668's Scope reads AC-1 to AC-10. No test tagged FR-223-AC-9 or AC-10 exists yet.

Units examined (role, id, path):

- examined: FR-223-AC-9 (spec/functional/FR-223-order-seq-cst-events-by-the-rc11-partial-sc-order.md)
- examined: FR-223-AC-10 (spec/functional/FR-223-order-seq-cst-events-by-the-rc11-partial-sc-order.md)
- examined: TC-668 step 8 (spec/test-cases/TC-668-rc11-partial-sc-order.md)
- examined: FR-437-AC-12 (spec/functional/memory/FR-437-explore-release-acquire-memory.md)
- examined: FR-437-AC-13 (spec/functional/memory/FR-437-explore-release-acquire-memory.md)
- examined: TC-391 LT-17 (spec/test-cases/TC-391-litmus-conformance-vectors.md)
- examined: TC-393 MB-12 (spec/test-cases/TC-393-memory-budgets-and-verdicts.md)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-223-AC-10 has two fixtures: a lone thread running two `seq_cst` fences with no store, and the same thread with a `release` flag between the fences. TC-668 has a step for neither. Step 8 runs LT-17, a three-thread program whose first fence is also covered by `v`'s views, and its expected result says nothing about the lone-thread case. QSpec has MB-12 for the same AC (FR-437-AC-13). Add a TC-668 step with MB-12's two fixtures and their expected presence and absence. | spec/test-cases/TC-668-rc11-partial-sc-order.md:31-32 |
| FND-002 | low | MB-12's second fixture adds 'a second thread that has loaded `Y` and is still running'. That thread's acquire view (and its current view, for an acquire load) covers the first fence and not the second, so it keeps the fence live on its own. The fixture therefore cannot show that `Y`'s message view keeps the fence, which is what FR-437-AC-13 asserts. Drop the second thread (`u`'s own current view keeps `Y`'s message live), or assert presence before any load of `Y`. | spec/test-cases/TC-393-memory-budgets-and-verdicts.md:43 |

## Verdict

Every new AC has a TC row except FR-223-AC-10 in TC-668 (FND-001). LT-17 recomputes as stated.
