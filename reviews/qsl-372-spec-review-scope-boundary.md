---
id: SR-1021
title: "QSL-372 scope-boundary review of FR-219 to FR-229 against QSpec FR-434 to FR-439"
type: SpecReview
analysis: scope-boundary
scope: "agent-ix/quire-spec-language@31d4826351932a91059e7e89a5288eb2d7899c7e; spec/functional/FR-219..FR-229; compared with agent-ix/quire-specification branch spec/wave-b-q5-memory-protocol FR-434..FR-439"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-221
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-222
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-223
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-224
    type: reviews
---

## Summary

Ticket: QSL-372 (PR #574). The Wave B rule is: "Do not re-specify QSpec-owned semantics normatively in QSL. Cite the QSpec FR." QSpec FR-434 to FR-439 now exist. They state the `tso` and `ra` rules, the SC event graph, data races and the race summary, memory fairness and the bound semantics.

**Clean:** FR-219, FR-220, FR-226, FR-227 and FR-229 specify QSL's own S3 checks, types, seam, routing and replay executor. They cite QSpec for the grammar, wire and catalog codes.

**Not clean:** each FR below says in its Dependencies that QSpec owns the semantics, yet restates that semantics rule by rule with SHALL. SR-1020 FND-007 and FND-008 show the copies have already drifted.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The Behavior restates QSpec FR-436's `tso` rules (store, load, flush, locked accesses, fences, gates) as normative SHALLs, while the Dependencies say "QSpec owns the `tso` semantics". Have `Tso` implement QSpec FR-436, and keep only the QSL type, seam and identity outputs. | spec/functional/FR-221-explore-the-x86-tso-memory-model.md:50-69 |
| FND-002 | medium | The Behavior restates QSpec FR-437's `ra` access rules, message views, fences, split and merge, and garbage collection. Cite FR-437. | spec/functional/FR-222-explore-the-release-acquire-memory-model.md:53-85 |
| FND-003 | medium | The whole Behavior restates QSpec FR-437's SC event graph: new events, pruning, frontier, collection and key. Cite FR-437 and keep only QSL's representation choices. | spec/functional/FR-223-order-seq-cst-events-by-the-rc11-partial-sc-order.md:51-74 |
| FND-004 | medium | The data-race definition, race summary and item rules restate QSpec FR-438. Keep the QSL request-writer, `RaceFreedom` type and EN-1 obligations, and cite FR-438 for the semantics. | spec/functional/FR-224-derive-the-race-freedom-item-for-non-atomic-locations.md:69-103 |
| FND-005 | low | The flush and visibility fairness definitions restate QSpec FR-436 and FR-437. Cite them, and keep the FR-126 filter and CX-5 obligations. | spec/functional/FR-228-derive-memory-fairness-constraints.md:59-77 |
| FND-006 | low | The bound semantics (bound-limited states, boundary states, the verdict table) restate QSpec FR-439. Keep the `ModelCheckLimits` fields and outcome type, and cite FR-439 for the rest. | spec/functional/FR-225-bound-the-memory-component-by-modelchecklimits-budgets.md:62-83 |

## Verdict

Six FRs duplicate QSpec semantics. The duplication is already causing drift (SR-1020 FND-007, FND-008). Replace the restated rules with citations to QSpec FR-436 to FR-439, and keep the QSL-specific types and engine obligations.

## Dispositions

Round 1, reviewed at 1e72750b1351d38cc2fc1a516fa951d1cda4583b.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 9807af2e: `Tso` implements QSpec FR-436 through the SE-1 members. |
| FND-002 | fixed | 9807af2e: `Ra` implements QSpec FR-437. |
| FND-003 | fixed | 9807af2e: FR-223 cites QSpec FR-437's new-event, frontier and collection rules. |
| FND-004 | fixed | 9807af2e: FR-224 carries QSpec FR-438's race summary by citation. |
| FND-005 | fixed | 9807af2e: FR-228 cites QSpec FR-436 and FR-437 for the fairness definitions. |
| FND-006 | fixed | 9807af2e: FR-225 cites QSpec FR-439's bound rule. |
