---
id: SR-1086
title: "Integrity review of the QSL-352 function-contract obligation identity"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@7b1ba3a3c; ADR-013 O-09 and O-26, ADR-011 E7, FR-121, TC-516, spec/spec.md, spec/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-121
    type: reviews
---

## Summary

Ticket: QSL-352. Base checklist and integrity review of the diff against
FR-092 (function node shape and key), FR-098 (function-path replay),
QSpec FR-322 (`declaration` member and occurrence roles) and the
`qsl-replay` `FunctionSite` on main.

The function-contract subject (function node id, its `declaration`
occurrence at ordinal 0) is sound: FR-322 lets a declaring node carry one
`declaration` occurrence and refuses two nodes with one name, and FR-092
keys a function node over `node_tag` `function` and its `declaration`, so
it cannot equal a clause or application node id. The granularity (one
obligation per function per CG kind) matches FR-098, which selects one
whole `Boolean` function and settles one verdict. `FunctionSite` on main
holds only `parameters`, so CG had no QSL-supplied subject members; FR-121
now returns them and AC-15 covers them. AC-15 has a TC (TC-516 step 15),
and the index rows record it as planned.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Fixed. O-09 said an edit leaving the function's name, parameters, result type and body unchanged leaves its id unchanged; the id also depends on the owner and on every node the body references (a changed callee changes it). The sentence now names the owner and referenced nodes, and says a comment elsewhere changes nothing. | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md O-09 |
| FND-002 | low | Fixed. O-09 stated the contract over "the parameters' declared domains"; the obligation holds over the declared per-argument domains of `arguments`, which can be a harness subset (AD-016 harness domain). Reworded. | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md O-09 |
| FND-003 | low | Fixed. O-09's implementing-ticket row did not say where the function subject members come from; it now names FR-121's `FunctionSite` (QSL-352). | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md O-09 |
