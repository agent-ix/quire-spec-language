---
id: TC-144
title: "Preserve temporal evaluation ownership on the checked-package route"
type: TC
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-052, type: verifies }
---
# TC-144: Preserve temporal evaluation ownership on the checked-package route

## Description

Verify FR-052-AC-9 to FR-052-AC-14 on the Layer-4 checked-package route:
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
| 2 | `Reach` over **A** with the request's result fields (truth, settlement, support) set to `Completed(false)` / other values wherever the request type has such fields; also supplied through every serialized request form the route decodes | the result equals row 1's; a request that carries those fields is refused at decode with `invalid_runtime_input`/`invalid-value` naming the field, or the type has no such field (inspect and record which) |
| 3 | `Due` activation from the checked package | the same truth as row 1 for the activated clause; the result retains the clause identity, profile and clock binding, activation state, anchor and progress as separate fields (FR-052-AC-10) |
| 4 | remove, one run each: the activation record; a capture value; the valuation of `level` at position 1; the clock binding name; the completeness assertion (set `Incomplete`); the watermark; the scope closure (set `Open`) | each run returns a typed non-truth outcome (`Pending`, `Indeterminate`, `Incomplete`, or a refusal with its record), never `Completed`, and the other facts of the result stay present and unchanged; none reads as false, closed or complete |
| 5 | an unsupported input: a `Lasso` for `Reach`; a `Finite` trace for `Window`; an inconsistent input: a stamp decreasing along the order for `Window` | `invalid_runtime_input`/`invalid-value` at stage `admit` each; no partial truth |
| 6 | two obligations `Reach` (settled over **A**) and `Reach2` (same clause over a trace with a work budget of zero) in one request | `Reach` `Completed(true)` unchanged; `Reach2` `Incomplete` at `temporal.visit`; no `Reach2` truth |
| 7 | `Window` over **A** | `Completed(true)` (the position stamped 5 is eligible for `[0,5]` and `level = 2` there); the profile is not refused (FR-052-AC-14) |
| 8 | progress ledger for the binding: W 100 then W 60; `Complete {through 100}` then `Complete {through 60}`; a foreign clock then a foreign package under the same request | the contradictions refuse as FR-502-AC-1 and AC-2; the foreign progress settles nothing (FR-052-AC-13) |
| 9 | correction relation: if the route's request type carries a predecessor reference, supply the exact direct predecessor with a changed input, then a self predecessor, a non-direct predecessor, a predecessor of another obligation and a predecessor with a mismatched result identity | the first admits and the predecessor's bytes stay unchanged; each of the other four refuses with its typed record. If the request type carries no predecessor reference, the step records `not applicable: no correction relation on the route` and inspects that a supplied predecessor field is refused at decode (FR-052-AC-12) |

## Expected Results

- Truth, settlement and support equal the reference rule for every row and no
  input permits caller-authored truth.
- Row 4 asserts, for each removed fact, the outcome variant and that every other
  fact is still reported.
- Row 6 asserts the settled sibling's result is byte for byte the one it has
  alone.
- Test code carries the tags FR-052-AC-9 to AC-14 and TC-144.
