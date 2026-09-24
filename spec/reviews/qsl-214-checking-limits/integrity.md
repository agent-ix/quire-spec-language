---
id: SR-615
title: "Integrity and EARS review of the QSL-214 default checking ceilings"
type: SpecReview
analysis: integrity
scope: "QSL-214/QSL-215 branch: new NFR-011 and TC-423, the NFR-011 references added to FR-062 and FR-093, and their spec.md and tests.md index rows"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/NFR-011
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-423
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: reviews
---

## Summary

The base checklist, the integrity analysis and the EARS analysis were run over
the change. NFR-011's statement uses the EARS unwanted-behaviour pattern
("If ..., then the checker shall ..."), with one trigger and one response. The
four metric rows are measurable and each has a default and a method. Every
metric row traces to TC-423, and each ceiling's refusal at a caller-selected
bound traces to an existing TC or AC. FR-062 and FR-093 now cite NFR-011
instead of restating the numbers, apart from FR-093's one example value. The
index rows resolve. Four integrity gaps were found against the code.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Scope said a standalone expression check runs "under one `CheckingLimits`" like a package check, implying all four ceilings apply. `check_clause_expression` builds only a `Typer`: it applies nodes and depth, encodes no declaration and charges no work. Fix: say which ceilings each entry applies. | NFR-011 Scope; `qsl-semantics/src/check/mod.rs` `check_clause_expression` |
| FND-002 | low | The preimage-bytes derivation claimed a source within NFR-001's defaults "fits this ceiling with headroom", but no measurement backs it. Fix: state the linear growth in source bytes and nodes that the derivation rests on, and drop the unmeasured claim. | NFR-011 Default derivation |
| FND-003 | low | "The node ceiling ends the walk before memory grows with `n`" is false as worded. The measured peak RSS at refusal grows from 73 MB at n = 9 to 97 MB at n = 12, because leaf paths get longer. Fix: say that the ceiling, not `(n - 1)!`, bounds the walk, and that its memory grows at most linearly with `n`. | NFR-011 Cost at the defaults; `qsl-bench/BASELINE.md` |
| FND-004 | low | Verification cited "TC-381 and FR-062-AC-5 and AC-7" without saying which covers which ceiling. Fix: map each ceiling to its TC or AC. | NFR-011 Verification |

## Resolution

All four findings are fixed in NFR-011: Scope names the ceilings each entry
applies (FND-001), the preimage derivation states only the linear growth
(FND-002), the cost section states the linear memory bound (FND-003), and
Verification maps nodes to TC-381, depth to FR-062-AC-7, and preimage bytes
and work to FR-062-AC-5 (FND-004).
