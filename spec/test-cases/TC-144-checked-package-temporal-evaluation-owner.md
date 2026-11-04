---
id: TC-144
title: "Preserve temporal evaluation ownership on the checked-package route"
type: TC
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-052, type: verifies }
---
# TC-144: Preserve temporal evaluation ownership on the checked-package route

## Description

Verify FR-052-AC-9 to FR-052-AC-14 on the Layer-4 checked-package route, which
has two entry points, and each row is verified on the one that carries its
input:

- the clause route, `qsl_replay::spine::run_clause` (FR-330) with a
  `ClauseRunRequest`, for rows 1, 4a to 4f, 4h, 5, 6, 7 and 8, which need only a
  clause, a trace, progress and a ledger;
- the evidence route, `qsl_eval::temporal::evidence` (FR-516): `EvidenceRequest`,
  `evaluate_temporal_evidence` and `activate`, over a package compiled through S1
  to S4 and its FR-449 activation map, for rows 2, 3, 4g and 9. Trigger identity,
  the activation map, capture removal and the correction relation (FR-524,
  FR-526) exist only there; `run_clause` has no trigger or relation input.

The test is a behavior test over those routes and does not call the retired native
request/result API or `src/temporal`.

Catches a route that returns a leaf valuation as the formula result, accepts
caller-supplied truth, turns a missing fact into false, closed or complete,
erases a settled sibling when one obligation exhausts its budget, accepts a
non-direct correction predecessor, or refuses the finite-window profile.

Executable controls for the clause-route rows live in the module
`checked_route_owner` beside the spine clause tests in `qsl-replay`; those of the
evidence-route rows (2, 3, 4g, 9) live in `qsl-eval/tests/it`.
Expected values come from the authored unit and the stated traces, and are computed by the FR-043/FR-501 reference rules
written in the table, never from the route's own output.

## Test Procedure

Fixture **T**: a unit with model `M` (object `Widget`, field `level`),
temporal clauses `Reach` = `eventually[0,2] holds(level = 2)` under
event-position false-extension, `on origin`, and `Window` = `eventually[0,5]
holds(level = 2)` under finite-window, and a protocol obligation `Due` that
activates `Reach` through the FR-449 map. Trace **A** has `level` 0, 1, 2 at
positions 0, 1, 2 (stamps 0, 2, 5 for `Window`; `W` 5; `Complete {through 5}`).

Fixture **T-E** (evidence route): fixture **T** with `Reach` activated `on origin`.
Rows 2, 3 and 9 supply one execution origin with a non-empty identity `O1` and a
`trigger` naming `O1`, and one correspondence held fixed across every request of a
row; the relation lookup is keyed by the package, the clause and that trigger
identity (FR-526), and a changed correspondence is refused `correspondence-changed`.
Row 9's first step keeps the correspondence unchanged and changes only the evidence.

| # | Input | Expected |
| --- | --- | --- |
| 1 | `Reach` over trace **A** | `Completed(true)`; the leaf `holds(level = 2)` at position 0 is false, so the result differs from the leaf valuation (FR-052-AC-9) |
| 2 | evidence route (fixture **T-E**, below): `Reach` over **A** as an `EvidenceRequest`; then a `Superseding` request (FR-526) naming row 1's result, same correspondence and trigger, whose evidence is changed so that the formula is violated | a test destructures `EvidenceRequest` exhaustively, so a member carrying truth, settlement or support breaks the build; the superseding result's truth, settlement and support are derived from its own evidence (`Completed(false)`, not row 1's `true`) and row 1's result is unchanged (FR-526 Behavior) |
| 3 | evidence route: `Due` activation of `Reach` from the checked package over **A**, `control` naming the `Due` clause | the same truth as row 1; the `EvidenceResult` holds the authenticated binding (profile and clock), the trigger identity, the instances with their activation states, the four progress axes, the completeness record and the truth as separate members |
| 4a | `Window` over **A**, stamps 0, 2, 5, `level` 0, 1, 2, `W` 5, `Complete {through 5}`, scope `Closed` | `Completed(true)`, basis `closed-scope`, support {(position 2, level)} |
| 4b | 4a with the valuation of `level` removed at position 1 (stamp 2) | `Completed(true)` unchanged (the missing fact is outside the support); the report lists the missing valuation in its premises |
| 4c | 4a with the valuation of `level` removed at position 2 (stamp 5) | `Indeterminate {MissingValuation, position 2, level}`, no truth |
| 4d | trace **B**: `level` 0, 1, 0 at stamps 0, 2, 5, `W` 5, `Complete {through 5}`; then the same with `Incomplete` | `Completed(false)`; then `Indeterminate {IncompleteInput}` |
| 4e | trace **B** with `W` 3, `Complete {through 3}` | `Pending {deadline 5}` |
| 4f | 4a with scope `Open` | `Completed(true)`, basis `decisive-witness` (a verdict, not a non-truth outcome) |
| 4g | evidence route: a request whose `control` names a protocol clause with no row in the package's FR-449 activation map; then one whose row names another temporal clause; then the `Due` delivery supplying one capture fact fewer than the clause declares | `Err(CallFailure::Admission)` with cause `activation-mapping-missing` (FR-516-AC-16); then cause `activation-mapping-mismatch`; each with an empty charge log; then an `Unavailable` instance whose `ActivationFault` names the missing capture with kind `Missing` (FR-524), no truth, and no temporal position evaluated |
| 4h | `Reach` supplied a `Timestamped` trace; `Window` supplied a `Finite` trace | each refused at stage `admit`, `invalid_runtime_input`/`invalid-value` |
| 5 | clause route: a `Lasso` trace for `Reach`; a `Timestamped` trace for `Window` listed out of stamp order (0, 5, 2) (`level` 0, 2, 1, so the canonical order is trace **A**'s) | the `Lasso` is refused at stage `admit`, `invalid_runtime_input`/`invalid-value`, with no partial truth; the out-of-order `Timestamped` listing is not refused and is evaluated in canonical order, positions being indices in that order, so the same observations in another order give an identical report, `result_identity` included (FR-501 "Canonical order", FR-501-AC-18, FR-501-AC-19): the 0, 5, 2 listing gives `Completed(true)` with the same outcome, basis, support, premises and `result_identity` as row 4a |
| 6 | `Reach` (settled over **A**) and `Reach2` (same clause, work budget zero) in one request | `Reach` `Completed(true)` unchanged; `Reach2` `Incomplete` at `temporal.visit`; no `Reach2` truth |
| 7 | `Window` over **A** | `Completed(true)` (the position stamped 5 is eligible for `[0,5]` and `level = 2` there); the profile is not refused (FR-052-AC-14) |
| 8 | progress ledger for the `Window` binding: W 100 then W 60; `Complete {through 100}` then `Complete {through 60}`; the same progress under a foreign package, then a foreign clause | the contradictions refuse as FR-502-AC-1 and AC-2; the foreign progress settles nothing (FR-052-AC-13) |
| 9 | evidence route: with a ledger holding row 1's result `r1`, a `Superseding(r1)` request with changed evidence; then, once that result `r2` is held, `Superseding(r1)` again; then a `Superseding(r1)` for another clause of the package; then a `Superseding(r1)` whose trigger identity is another trigger's, with no ledger entry under that request's own trigger identity; then a `Superseding(r2)` with `r2`'s evidence unchanged; then a `Superseding(r1)` with no ledger | `lineage` 1, relation `Superseding` naming `r1`'s `result_identity`, and `r1` unchanged (FR-526-AC-2, AC-3); then `Err(CallFailure::Admission)` with cause `predecessor-not-latest`; then `predecessor-unknown`; then `predecessor-unknown` (the identity-mismatched predecessor of FR-052-AC-12, FR-053-AC-9: the ledger holds no entry under the request's own trigger identity); then `unchanged-input`; then `no-ledger`; each refusal with an empty charge log, the held results unchanged (FR-052-AC-12) |


## Expected Results

- Truth, settlement and support equal the reference rule for every row and no
  input permits caller-authored truth.
- Row 4 asserts, for each removed fact, the outcome variant and that every other
  fact is still reported.
- Row 6 asserts the settled sibling's result is byte for byte the one it has
  alone.
- Test code carries the tags FR-052-AC-9 to AC-14, FR-501-AC-19 (row 5) and TC-144.
  The clause-route rows are tagged in `checked_route_owner`; the rows 2, 3, 4g and
  9 in `qsl-eval/tests/it`.
