---
id: SR-20261103150649
title: "gap-analysis of QSL-1524 FR-507 acceptance criteria and TC-1302"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; PR #2201 at 8a2223ad2; FR-507-AC-1..AC-10, FR-057-AC-11, spec/test-cases/TC-1302-apply-each-capability-kind-to-its-declaration-family-only.md, TC-971, TC-115"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-507
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1302
    type: references
---

## Summary

Ticket: QSL-1524. Planless gap analysis of a spec-only PR. Plan completion: not assessed. No production code changes; the question is whether FR-507-AC-1..AC-10 and TC-1302 rows 1-10 are sufficient, traced and able to fail. Trace: TC-1302's tag line names FR-507-AC-1..AC-10 and each AC has its own row (1-10); FR-057-AC-11 traces TC-115, TC-971, TC-1302. Fail-ability: row 1 fails on any mis-tabled pair (14/70 recounted); row 8 fails if the family is read from the anchor instead of `item_kind` (temporal-satisfaction on a protocol node would be refused); row 7 fails if `operation-contract` is withheld from `relation-claim` or `global-conformance` admitted there; row 10 fails if the caller's disposition wins.

## Examined scope

- FR-507-AC-1 .. FR-507-AC-10 (examined) and TC-1302 rows 1-10 and fixture `C0` (examined).
- FR-057-AC-11 and TC-971/TC-115 expected results (examined).
- QSpec FR-290-AC-17 (context only): "For every claim-form row whose family is not none, the request item of that row's kind on such a claim is never refused this way."
- QSL grep for `verification_claim`, `analysis_claim`, `synthesis_request` emission (context only): only TC-1302 row 9 mentions one.

## Verdict

CONDITIONAL. The criteria are traced and mostly discriminating. One medium gap: no criterion exercises each node-to-family row, so a mis-mapped frame-obligation, Boolean value-clause, state-graph or timed/hybrid node would refuse its own kind with no TC-1302 row failing. Two low gaps on the row-9 fixture and on AC-10's precedence cases.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-507 has no criterion matching QSpec FR-290-AC-17's "never refused" clause per claim-form row. TC-1302's `C0` has one claim per family, so only one node row per family is exercised: `V` is a value function (scalar-operation `expression` node) and no Boolean value clause (`expression`, role `claim`/`declaration`) is tested; no frame obligation (`state`/`operation_anchor`, role `anchor`), state-graph (`claim`/`state_graph`), timed or hybrid temporal claim is tested. An implementation mapping `operation_anchor` to no family would refuse every frame obligation's `operation-contract` and pass AC-1..AC-10. Add an AC/row: for each row of FR-507's claim-form table, the row's kind on such a claim is not refused. | spec/functional/FR-507-apply-each-capability-kind-to-its-declaration-family-only.md:169-178; spec/test-cases/TC-1302-apply-each-capability-kind-to-its-declaration-family-only.md:19-28 |
| FND-002 | low | TC-1302 row 9 needs `C0` to hold a `claim`/`verification_claim` node, but no QSL requirement emits one (only TC-1302 names it), and the case does not say how the node enters `C0` (hand-built package, or an I2-read QSpec vector the reader admits). The rule itself matches QSpec (a node of no row has no family). Name the fixture source, or use an emitted `expression` node with an occurrence role outside the node table. | spec/test-cases/TC-1302-apply-each-capability-kind-to-its-declaration-family-only.md:40 |
| FND-003 | low | FR-507-AC-10 / TC-1302 row 10 cover handed `Supported`, `RequiresBound`, `Unsupported` and `invalid-request`/`absent-extent`, but not the precedence cases that collide with FR-281: `Supported` on a candidate `engines` does not hold (FR-281 `unknown-backend`), `Supported` on an engine not advertising the (kind, mode) (`inconsistent-candidates`), and a handed `invalid-request` with a candidate-table cause (`ambiguous-backend`). Add them once FR-281 states the order (spec-review FND-001). | spec/functional/FR-507-apply-each-capability-kind-to-its-declaration-family-only.md:178; spec/test-cases/TC-1302-apply-each-capability-kind-to-its-declaration-family-only.md:41 |
