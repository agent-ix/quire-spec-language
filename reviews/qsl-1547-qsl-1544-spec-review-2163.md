---
id: SR-2163001
title: "spec-review of batch 5 lane gaps FR-265, FR-509, FR-515, FR-519 (quire-spec-language PR 2163)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-265-derive-a-state-clause-separating-witness.md; spec/decisions/ADR-031-state-forall-separating-witness.md; spec/test-cases/TC-740-derive-a-state-clause-separating-witness.md; spec/functional/FR-509-declare-a-channel-s-delivery-count-interval.md; spec/test-cases/TC-1304-declare-a-channel-s-delivery-count-interval.md; spec/functional/FR-515-read-the-checked-predicate-and-temporal-subject-from-a-checked-package.md; spec/test-cases/TC-1310-read-the-checked-predicate-and-temporal-subject-from-a-checked-package.md; spec/functional/FR-519-publish-the-protocol-read-surface-the-quire-protocol-link-layer-imports.md; spec/test-cases/TC-1314-publish-the-protocol-read-surface-the-quire-protocol-link-layer-imports.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-265
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-515
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-519
    type: references
---

# Spec review of batch 5 lane gaps (quire-spec-language PR 2163)

## Summary

Tickets: QSL-1547 (FR-519, FR-265) and QSL-1544 (FR-515). One spec-review pass over
`git diff origin/main...HEAD` at head f302bfd63 (EARS, integrity and cross-reference checks folded
into this one file; no Jev client used, criterion-strength analysis skipped).

Checked against the code: QSL PR #2088 head 01b65d4f6 (`qsl-replay/src/witness/derivation.rs`,
`qsl-eval` evaluate.rs `ConnectiveOrigin`), PR #2158 head 45ec3ccd5 (`protocol_view.rs`,
`qsl-semantics` `protocol_clause/types.rs`), PR #2156 head c886b935f (`checked_views`), merged
FR-516/517/518 and the Linear tickets IR-1278 and IR-1279. TRUE of the code: the Authored and
Synthesized connective rule and the closed-scope end (FR-265 AC-8..10); the Port, PortRef,
CausalEdge and ten EdgeKind vocabulary and the from, to, kind ordering; the fourteen view function
names; `requirements()` and `source_span()` (role `claim`); the guard-then-holds-then-clause kind
order. Spec ahead of code, with an owning ticket: `LeafHasNoSource` (AC-12), the span-inside-formula-root
rule (AC-4; the code says "inside the clause's span") and the view-removal refusals (AC-8). Gated
ACs with a ticket: AC-13 on IR-1279, AC-14 on IR-1278. `quire validate` over the nine changed
files exits 0 with only the pre-existing archetype warnings. No id collision with open PRs; no
contradiction with merged FR-516/517/518 or FR-322 (AC-310..341) found.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | FR-265 loses three existing requirements: the walk visits each node at most once, the walk ending at a stopped quantifier settles `decisive-counterexample` (false) or `decisive-witness` (true), and the record is built from that stop report. The diff replaces them with the connective bullets and no remaining statement covers them. ADR-031 SW-2 and SW-3 and AC-1 to AC-7 still depend on them. | spec/functional/FR-265-derive-a-state-clause-separating-witness.md:68-85 | wrong-requirement |
| FND-002 | medium | TC-1304 row 11 has four cells (row number, AC number, clause, expected) in a table whose header is Row, Channel clause, Expected; the AC cell shifts the clause and expected columns. quire validate does not catch it. | spec/test-cases/TC-1304-declare-a-channel-s-delivery-count-interval.md:33 | wrong-requirement |
| FND-003 | medium | FR-515 still says the shared-node clock case has no admitted package "until the Contract IR decision named in FR-515-AC-7 lands". AC-7 no longer names it (it is infinite-trace only); the gate is AC-13 and IR-1279. | spec/functional/FR-515-read-the-checked-predicate-and-temporal-subject-from-a-checked-package.md:154 | wrong-requirement |
| FND-004 | medium | `requirements()` is specified (node ids of protocol clauses whose requirements name the clause, in row order) and FR-515-AC-1 lists the accessors it checks without it; no AC and no TC-1310 row asserts `requirements()`. | spec/functional/FR-515-read-the-checked-predicate-and-temporal-subject-from-a-checked-package.md:101 | missing-requirement |
| FND-005 | medium | FR-519 says the map "ends in one of two dispositions" (read by a QSL view function, or consumed by name), but the rows `Package`, `Handle`, `Declaration`/`Body::Protocol`, `ActivationMapping` and `BindingKind::Clock` keep the bare disposition `Successor`. AC-5 covers only rows marked read by a view function, so those rows have no AC. | spec/functional/FR-519-publish-the-protocol-read-surface-the-quire-protocol-link-layer-imports.md:99-127 | missing-requirement |
| FND-006 | low | The `source()` row for the predicate view says the leaf's entry is "the one whose regions lie inside the source regions of the clause's formula root node". An `ActivationGuard` leaf is read from the activation argument, not the formula, so that rule cannot hold for it. AC-4 covers only `HoldsLeaf`. | spec/functional/FR-515-read-the-checked-predicate-and-temporal-subject-from-a-checked-package.md:126 | wrong-requirement |
| FND-007 | low | FR-515-AC-6 asks for a second package "whose clause has the same name", but the package carries no clause name until IR-1278 (AC-14); AC-9 states "QSL-303 retired them", a history clause rather than a checkable property (the TC row tests only the grep). | spec/functional/FR-515-read-the-checked-predicate-and-temporal-subject-from-a-checked-package.md:165 | wrong-requirement |
| FND-008 | low | TC-740 step 10 expects the Authored tree to record "`true` or `false`" for the left operand while AC-10 asserts only that a decision is recorded; the TC row should name the value the fixed observation produces so it can fail. | spec/test-cases/TC-740-derive-a-state-clause-separating-witness.md:75 | missing-requirement |

## Verdict

Not mergeable until FND-001 is fixed: it silently removes the requirement that settles the decisive
basis and builds the record, which the unchanged ADR and AC-1..7 rely on. FND-002 to FND-005 are
real defects to fix in the same PR. The remaining checks passed: AC-8..10 and TC-740 steps 8..10 match
the code; FR-519 vocabulary, ordering, EdgeKinds and view function list match #2158; AC-7 of FR-519
and TC row 7 (QSL-1546/1547/1548) agree; renumbering of FR-515 ACs 1..14 and FR-519 ACs 1..9 is
consistent between FR, TC scope lines and trace tags; FR-509-AC-7 and TC-1304 trace tag agree.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-009 | low | FR-515 `source_span` is specified per leaf kind, but only `HoldsLeaf` (AC-4) and `ActivationGuard` (AC-16) have a criterion; the `ClauseLeaf` rule (entry inside the formula root's regions) is asserted by none. | spec/functional/FR-515-read-the-checked-predicate-and-temporal-subject-from-a-checked-package.md:126 | missing-requirement |
| FND-010 | low | FR-519 map rows consumed by name (`Type`/`Value`/`Number`, `BindingKind::Population`) and the `Window` row (no QSL view) have no criterion; AC-5 covers the view-read rows and AC-10 only `Package` and `Handle`. | spec/functional/FR-519-publish-the-protocol-read-surface-the-quire-protocol-link-layer-imports.md:125-131 | missing-requirement |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | The three FR-265 bullets are back (visit each node at most once; settle decisive-counterexample or decisive-witness; build the record from the stop report) beside the connective bullets; the diff against main now deletes no FR-265 line. |
| FND-002 | fixed | TC-1304 row 11 is three cells: parse a channel clause with `delivery [0,3]` (CST only, no check); the AC is FR-509-AC-7, expected parses with no syntax error. |
| FND-003 | fixed | FR-515 now reads "until the Contract IR decision (IR-1279, FR-515-AC-13) lands". |
| FND-004 | fixed | `requirements` is in AC-1's accessor list, and new FR-515-AC-15 and TC-1310 row 15 assert it (empty for an unnamed clause, the protocol clause id for a named one). |
| FND-005 | fixed | Every map row has a disposition: Package and Handle consumed by name (AC-10, TC-1314 row 10), Declaration/Body read by protocol_view and declaration, ActivationMapping by protocol_view, BindingKind::Clock by temporal_binding (AC-5), Window no QSL view. |
| FND-006 | fixed | The predicate source() row states the rule per kind: HoldsLeaf and ClauseLeaf inside the formula root's regions, ActivationGuard an entry of the guard node (AC-16, TC-1310 row 16). |
| FND-007 | fixed | AC-6 uses a second package emitted from the same source; AC-9 says neither module path resolves, with a compile-fail use of each (TC row 9 matches). |
| FND-008 | fixed | TC-740 step 10 uses an `and` whose left operand is `true`; the Authored tree records decision `true`, the Synthesized tree none. |
