---
id: SR-14052
title: "gap analysis of STD-239 new and changed ACs against TC rows (PR #1192, QSpec PR #331)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@39f1130926a6da9993c49a56fef0365d3a7eecb4; spec/functional/FR-223-order-seq-cst-events-by-the-rc11-partial-sc-order.md, spec/test-cases/TC-668-rc11-partial-sc-order.md; agent-ix/quire-specification@4010594880b59e0154cdc9f7e0a86d0aca18339b; spec/functional/memory/FR-437-explore-release-acquire-memory.md, spec/test-cases/TC-391-litmus-conformance-vectors.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-223
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-668
    type: reviews
---
# SR-14052: gap analysis of STD-239 (PR #1192, QSpec PR #331)

## Summary

Ticket: STD-239. Plan completion: not assessed. This is a manual mapping of
each new or changed AC to its TC rows. It covers spec only: the code for
these ACs is in the code lane's remaining work, so no tagged test exists
yet. Mapping:

- FR-223-AC-5 maps to TC-668. Scope is now AC-1 to AC-6. Step 5 runs LT-12
  and LT-13 under `sc`, `tso` and `ra`, and its expected result is "both
  outcomes reachable under `ra` only".
- FR-223-AC-6 maps to TC-668 step 6, with the load enabled for X = 1 only.
- FR-437-AC-6 (changed) maps to TC-391. The procedure and the expected
  results both name the LT-12 and LT-13 exception.
- FR-437-AC-8 maps to the TC-391 vector rows LT-12 and LT-13.
- FR-437-AC-9 maps to the TC-391 vector row LT-14.

The expected cells were recomputed by script (see SR-14050) and match.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-223-AC-5's second sentence, "Every other litmus outcome of FR-222-AC-2 and FR-223-AC-2 is unchanged", has no TC-668 step or expected result. Step 5 runs only LT-12 and LT-13. Either point step 5's expected result at the TC-667 and TC-668 step 2 vectors still holding, or drop the sentence, since those ACs already pin the outcomes. | spec/functional/FR-223-order-seq-cst-events-by-the-rc11-partial-sc-order.md:80; spec/test-cases/TC-668-rc11-partial-sc-order.md:25,39 |
| FND-002 | low | FR-437-AC-9's state-level clause, that the third thread's `seq_cst` load of the fenced location is enabled for the new message only, is not checked by TC-391. TC-391 records only whether each outcome is reachable, through the `always holds(not outcome)` invariant. Add a TC-391 step that lists the load's enabled choices at the state in question, as QSL TC-668 step 6 does, or reduce the AC to the outcome. | spec/functional/memory/FR-437-explore-release-acquire-memory.md:179; spec/test-cases/TC-391-litmus-conformance-vectors.md:52 |

## Verdict

Every new or changed AC has a TC row. Two low coverage gaps remain on
secondary clauses. The test bindings come with the code lane, so there are
none to check here.

## Dispositions

Disposition pass 1, against QSL PR #1192 head `985acb35d010c8b1c20f0e24cf2b9ff17412c80d` and QSpec PR #331 head `7e7a8e09af7a696cc0df776047fb8c7ba16648ec`. Both heads contain `origin/main`. Litmus cells and the soundness claims were recomputed by script (scratchpad rv-std239-d1/litmus/: lt2.py, fuzz2.py, econ.py, eco3n.py), with `ra` run by RA-1 to RA-6 and PSC-1 to PSC-3 as now worded.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | The sentence now names a checkable property, and TC-668 step 5 runs the RC11 checker with a matching expected result. After: Every outcome that an independent RC11 execution-graph checker allows for the two programs is reachable under `ra`. - Step 5: both outcomes reachable under `ra` only; every outcome the checker allows is reachable under `ra`. |
| FND-002 | fixed | The AC is reduced to the outcome, which TC-391 checks. After: \| FR-437-AC-9 \| TC-391 LT-14 and LT-18 do not reach their weak outcomes under `ra`. \| |
