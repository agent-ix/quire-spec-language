---
id: SR-2272001
title: "spec-review of quire-spec-language PR 2272 (QSL-1632 ruling, QSL-1608 fixes)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; FR-283; FR-286; FR-515; FR-516; TC-765; TC-770; TC-1310; TC-1311"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-286
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-515
    type: references
---

# Spec review of quire-spec-language PR 2272

## Summary

Tickets: QSL-1632 (QSL-1608 pointer). Head 451d551b4b4fe29e59efae8ce393f9d2832abefd. Verified: FR-515-AC-22 re-derived by hand: atoms [capture, over() only, no binder, capture and over()] give instance-bound {0, 3}, and swapping the first two gives {1, 3}; FR-337 states S3 refuses a read of the trigger binder in a `holds` condition as `missing-name`, so only capture binders make an atom instance-bound; FR-516-AC-37 now matches FR-504 Consistency (fixed-sample and event-position only); `No other cause exists` stays true (clause-root-unresolved is in the table, clause-unreadable extended); FR-515-AC-24 and FR-516-AC-45 and AC-46 are unused on main and on the open PR heads; Delivery lines, TC scope lines and trace tags updated; no ticket number in any AC. quire validate exit 0 on all eight changed files and on spec/** plus plan/**.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | `decisive` is specified as a member of the `counterexample` record in the outcome item (FR-286 Violated row, AC-10, TC-770 step 9), but QSpec FR-364 says a TemporalCounterexample is one JSON object with exactly the members of its table (AC-364-2 lists them), and FR-267's closed `temporal` row for `violated` lists only `position` and `counterexample`, writes the counterexample as the report holds it and derives none of its members, and its reader refuses a counterexample FR-364's replayer refuses and any member the row does not list. FR-283 calls `decisive` a QSL member of the in-memory record outside the FR-364 wire form, so it cannot be written inside the counterexample object of any machine output (item or run-result/2) without breaking FR-364 or FR-267, and omitting it loses the data the item asserts; FR-283 also sources it 'through FR-267's witness channel' while FR-266/FR-267 say a temporal run has no witness. As written, FR-286-AC-10 and TC-770 step 9 cannot pass on the machine output. Resolution: carry `decisive` as a sibling member of `position` and `counterexample` on the MonitorRecord and the item (a QSL member of the record, not nested in the FR-364 object), state that FR-267's temporal member and FR-364's wire are unchanged by it, and drop the 'witness channel' wording; or add the member to QSpec FR-364's table through a QSpec change first. | spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md | wrong-requirement |
| FND-002 | medium | The cross-check says an in-memory clause whose formula differs from the package's `formula_graph()` makes `evaluate_temporal_evidence` refuse `clause-unreadable`, but the entry takes `(&self, clause: &CheckedNodeId, request, meter)` and no in-memory clause, the read-back route (`from_read`) has none, and the spec does not say where the in-memory formula lives (the `Emission`?) or at which call the comparison runs (construction or each evaluation). AC-24 and AC-46 then cannot be written as a test. State what holds the in-memory formula and where the comparison runs, and what `from_read` does. AC-24 (TC-1310) and AC-46 (TC-1311) also assert the same behaviour twice. | spec/functional/FR-515-read-the-checked-predicate-and-temporal-subject-from-a-checked-package.md | wrong-requirement |
| FND-003 | medium | AC-19, AC-22, AC-24, FR-516-AC-30 and AC-46 are stated on the `config-version` fixture (examples/config-version), which holds model.semantic-ir.json and Rust test files and no temporal clause, capture, `on each` activation or guard. The ACs do not say the test authors those clauses over the fixture, so an implementer cannot tell whether the fixture is to be extended or the clause supplied elsewhere. Say which clauses are written over the fixture's model (and where), or name the fixture file that holds them. | spec/functional/FR-515-read-the-checked-predicate-and-temporal-subject-from-a-checked-package.md | wrong-requirement |
| FND-004 | low | FR-515-AC-18 and FR-516-AC-18 say 'R's three clauses (the FR-517 packages)', but FR-517 says R is three packages, one per clock kind, holding four temporal clauses. Say 'each of R's three packages and its temporal clauses'. | spec/functional/FR-515-read-the-checked-predicate-and-temporal-subject-from-a-checked-package.md | wrong-requirement |
| FND-005 | low | Wording: the FR-286 member table and clause-run table still say `cause` 'none' while AC-8, AC-10 and TC-770 now say `cause` null; the Violated row keeps a `witness` clause although FR-283 now says a temporal Violated or Undefined record never has one; FR-516 says the cause table is 'owned here' and also that `clause-root-unresolved` is FR-337's cause, which FR-337 defines as a lookup result with no catalog code, so the call's catalog code for it is unstated; FR-283 Dependencies has a stray double blank line. | spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md | wrong-requirement |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-006 | medium | FR-283 and FR-286 read `decisive` from `ClauseRunReport::decisive`, but no report requirement defines that member: FR-330's Outputs and FR-266 (which adds `basis` and `witness`) do not list it, and FR-267 requires writing a temporal report and reading it back to restore an equal typed value (listing the two rows that restore less) while its closed `violated` row has no `decisive`, so a written-and-read report would lose the member and break the round trip. Add `decisive` to the report requirement (FR-330 Outputs or an FR-266 addition, with its presence rule: only a temporal violation) and say in FR-267 whether it is outside the written form (a third row that restores less) or written. | spec/functional/FR-283-monitor-a-supplied-trace-offline.md:175-195 | missing-requirement |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-007 | low | TC-840 gained step 5 for FR-330-AC-12 but its Scope line still reads FR-330-AC-1 to FR-330-AC-4; add FR-330-AC-12. Separately, the `clause-root-unresolved` row now says the catalog code is FR-337's, but FR-337 defines the name as a lookup result and states no catalog code, so the call has none stated; say 'FR-337 states none' or name one. Non-blocking. | spec/test-cases/TC-840-run-a-temporal-clause-through-the-spine.md:17 | wrong-requirement |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Verified in fix commit b38348989 against the current text. |
| FND-002 | fixed | Verified in fix commit b38348989 against the current text. |
| FND-003 | fixed | Verified in fix commit b38348989 against the current text. |
| FND-004 | fixed | Verified in fix commit b38348989 against the current text. |
| FND-005 | still-open | The null cause, the conditional Violated witness and the blank line are fixed, but the new `clause-root-unresolved` row says the call reports it 'with the catalog code `invalid_runtime_input`/`clause-root-unresolved`, as it reports the other request-shape causes of this table'. FR-516's table states no catalog code for any other cause (trigger-required, occurrence-required, occurrence-unknown, activation-list-mismatch, activation-mapping-*, progress-inconsistent, clause-unreadable all list a condition only), and line 125 sends the code to the owning requirement, so that comparison is unsupported. FR-337 defines the name as a lookup result with no catalog code, and `invalid_runtime_input` pairs elsewhere with causes such as `invalid-value`, `missing-member`, `wrong-role-mapping` (FR-522, FR-331), so `invalid_runtime_input`/`clause-root-unresolved` would be a new catalog pair no requirement or catalog entry defines. FR-516-AC-45 gives only the code and TC-1311 row 45 the code, while the row gives the pair: the three disagree. Fix: either state one catalog code/cause pair for request-shape refusals in one place (and check the catalog holds it; FR-337's own `missing_declaration`/`missing-name` is the nearest existing pair for an unresolved name), or say `CallFailure::Admission` carries the cause name only and the code is the catalog's; then make the row, AC-45 and TC row 45 agree. |
| FND-006 | still-open | new this round; see New findings. |
| FND-005 | fixed | Round 2: row states condition and cause name only, AC-45 and TC row 45 aligned, no invented code. |
| FND-006 | fixed | Round 2: FR-330 Outputs defines ClauseRunReport::decisive, FR-330-AC-12, TC-840 step 5, FR-267 three rows restore less, FR-267-AC-19, TC-742 step 4. |
| FND-007 | still-open | low, non-blocking. |

## Verdict

Not mergeable: FND-001 (high) makes FR-286-AC-10 and TC-770 step 9 unsatisfiable on the machine output. FND-002 and FND-003 are medium.

Round 1: FND-001 to FND-004 fixed; FND-005 (catalog code claim) still open; FND-006 new (ClauseRunReport::decisive undefined). Not mergeable.

Round 2: FND-005 and FND-006 fixed; FND-007 is a new low, non-blocking finding. Mergeable.
