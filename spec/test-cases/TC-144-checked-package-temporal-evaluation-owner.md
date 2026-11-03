---
id: TC-144
title: "Preserve temporal evaluation ownership on the checked-package route"
type: TC
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-052, type: verifies }
---
# TC-144: Preserve temporal evaluation ownership on the checked-package route

## Description

Verify FR-052-AC-9 to FR-052-AC-14 (FR-052-AC-12 through row 9's guard) on the Layer-4 checked-package route:
`qsl_replay::spine::run_clause` (FR-330) over a package compiled through S1 to
S4, with protocol obligations reached through the FR-449 activation map and
FR-390's slot evaluation. The test is a behavior test over that route and does
not call the retired native request/result API or `src/temporal`.

Catches a route that returns a leaf valuation as the formula result, accepts
caller-supplied truth, turns a missing fact into false, closed or complete,
erases a settled sibling when one obligation exhausts its budget, accepts a
non-direct correction predecessor, or refuses the finite-window profile.

Executable controls live in a new module `checked_route_owner` beside the spine
clause tests in `qsl-replay`. Expected values come from the authored unit and
the stated traces, and are computed by the FR-043/FR-501 reference rules
written in the table, never from the route's own output.

## Test Procedure

Fixture **T**: a unit with model `M` (object `Widget`, field `level`),
temporal clauses `Reach` = `eventually[0,2] holds(level = 2)` under
event-position false-extension, `on origin`, and `Window` = `eventually[0,5]
holds(level = 2)` under finite-window, and a protocol obligation `Due` that
activates `Reach` through the FR-449 map. Trace **A** has `level` 0, 1, 2 at
positions 0, 1, 2 (stamps 0, 2, 5 for `Window`; `W` 5; `Complete {through 5}`).

| # | Input | Expected |
| --- | --- | --- |
| 1 | `Reach` over trace **A** | `Completed(true)`; the leaf `holds(level = 2)` at position 0 is false, so the result differs from the leaf valuation (FR-052-AC-9) |
| 2 | `Reach` over **A**, the request serialized with the extra members `truth`, `settlement` and `support` set to values that contradict row 1 | refused at decode, stage `admit`, `invalid_runtime_input`/`invalid-value` naming the first extra member; no outcome. The unmodified request gives row 1's result |
| 3 | `Due` activation of `Reach` from the checked package over **A** | the same truth as row 1; the report holds clause identity, profile, activation state, anchor and progress as separate fields |
| 4a | `Window` over **A**, stamps 0, 2, 5, `level` 0, 1, 2, `W` 5, `Complete {through 5}`, scope `Closed` | `Completed(true)`, basis `closed-scope`, support {(position 2, level)} |
| 4b | 4a with the valuation of `level` removed at position 1 (stamp 2) | `Completed(true)` unchanged (the missing fact is outside the support); the report lists the missing valuation in its premises |
| 4c | 4a with the valuation of `level` removed at position 2 (stamp 5) | `Indeterminate {MissingValuation, position 2, level}`, no truth |
| 4d | trace **B**: `level` 0, 1, 0 at stamps 0, 2, 5, `W` 5, `Complete {through 5}`; then the same with `Incomplete` | `Completed(false)`; then `Indeterminate {IncompleteInput}` |
| 4e | trace **B** with `W` 3, `Complete {through 3}` | `Pending {deadline 5}` |
| 4f | 4a with scope `Open` | `Completed(true)`, basis `decisive-witness` (a verdict, not a non-truth outcome) |
| 4g | the `Due` activation with its activation record removed from the package map; then with a capture value removed | `missing_declaration`/`missing-name` at stage `select`; then an incomplete activation naming the capture, with no temporal position evaluated |
| 4h | `Reach` supplied a `Timestamped` trace; `Window` supplied a `Finite` trace | each refused at stage `admit`, `invalid_runtime_input`/`invalid-value` |
| 5 | a `Lasso` trace for `Reach`; a `Timestamped` trace for `Window` listed out of stamp order (0, 5, 2) (`level` 0, 2, 1, so the canonical order is trace **A**'s) | the `Lasso` is refused at stage `admit`, `invalid_runtime_input`/`invalid-value`, with no partial truth; the out-of-order `Timestamped` listing is not refused and is evaluated in canonical order (FR-501 "Canonical order", FR-501-AC-19): `Completed(true)` with the outcome and `result_identity` of row 4a |
| 6 | `Reach` (settled over **A**) and `Reach2` (same clause, work budget zero) in one request | `Reach` `Completed(true)` unchanged; `Reach2` `Incomplete` at `temporal.visit`; no `Reach2` truth |
| 7 | `Window` over **A** | `Completed(true)` (the position stamped 5 is eligible for `[0,5]` and `level = 2` there); the profile is not refused (FR-052-AC-14) |
| 8 | progress ledger for the `Window` binding: W 100 then W 60; `Complete {through 100}` then `Complete {through 60}`; the same progress under a foreign package, then a foreign clause | the contradictions refuse as FR-502-AC-1 and AC-2; the foreign progress settles nothing (FR-052-AC-13) |
| 9 | a request carrying a `predecessor` member naming a prior result, then a `supersedes` member | each refused at decode, stage `admit`, `invalid_runtime_input`/`invalid-value` naming the member: the spine route carries no correction relation, so FR-052-AC-12 binds only when a requirement adds one and this row is its guard |


## Expected Results

- Truth, settlement and support equal the reference rule for every row and no
  input permits caller-authored truth.
- Row 4 asserts, for each removed fact, the outcome variant and that every other
  fact is still reported.
- Row 6 asserts the settled sibling's result is byte for byte the one it has
  alone.
- Test code carries the tags FR-052-AC-9 to AC-14 and TC-144.
