---
id: SR-14072
title: "gap analysis of STD-246 new ACs against TC rows (PR #1193, QSpec PR #332)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@851e3a6a155b7b7c17ee088688e83d49139b68ad; spec/functional/FR-223-order-seq-cst-events-by-the-rc11-partial-sc-order.md, spec/test-cases/TC-668-rc11-partial-sc-order.md; agent-ix/quire-specification@eba4eabe67c00ba55c36230429451fb06719183a; spec/functional/memory/FR-437-explore-release-acquire-memory.md, spec/test-cases/TC-391-litmus-conformance-vectors.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-223
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-668
    type: reviews
---
# SR-14072: gap analysis of STD-246 new ACs against TC rows

## Summary

Ticket: STD-246. Plan completion: not assessed. This is a manual mapping, spec only, because the code is the code lane's remaining work. FR-223-AC-7 and AC-8 map to TC-668 step 7 and its expected result, and TC-668's Scope now reads AC-1 to AC-10 at the STD-247 head (AC-1 to AC-8 here). FR-437-AC-10 and AC-11 map to TC-391 LT-15 and LT-16. No test tagged FR-223-AC-7 or AC-8 exists yet. The TC-668 step 7 dependency on RA-7's pin set is SR-14070 FND-001.

Units examined (role, id, path):

- examined: FR-223-AC-7 (spec/functional/FR-223-order-seq-cst-events-by-the-rc11-partial-sc-order.md)
- examined: FR-223-AC-8 (spec/functional/FR-223-order-seq-cst-events-by-the-rc11-partial-sc-order.md)
- examined: TC-668 step 7 (spec/test-cases/TC-668-rc11-partial-sc-order.md)
- examined: FR-437-AC-10 (spec/functional/memory/FR-437-explore-release-acquire-memory.md)
- examined: FR-437-AC-11 (spec/functional/memory/FR-437-explore-release-acquire-memory.md)
- examined: TC-391 LT-15 (spec/test-cases/TC-391-litmus-conformance-vectors.md)
- examined: TC-391 LT-16 (spec/test-cases/TC-391-litmus-conformance-vectors.md)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | LT-16's note says '`L := 2` happens before `d := L` through the acquire fence', and FR-437-AC-11 and FR-223-AC-8 describe it the same way. But `d := L` is a `seq_cst` (acquiring) load of `L := 3`, which is in `L := 2`'s release sequence, so `d` itself synchronises with `L := 2`. In the engine, `d` joins `L := 3`'s message view, which holds the pair, into its current view directly. The script shows the outcome RC11-forbidden with the fence removed as well. So the vector passes even if pairs never travel through `acq_b` and the acquire fence. To test that path, have `d` read a later message whose view does not cover `L := 2` (for example a `relaxed` `L := 4` by a fourth thread), or correct the notes. | spec/test-cases/TC-391-litmus-conformance-vectors.md:54 |

## Verdict

Every new AC has a TC row in its own repo, and both vectors recompute as stated. One low test-intent finding.
