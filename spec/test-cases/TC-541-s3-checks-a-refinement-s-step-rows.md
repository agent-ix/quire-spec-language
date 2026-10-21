---
id: TC-541
title: "S3 checks a refinement's step rows, over a model subject and a protocol subject"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: verifies
---
# TC-541: S3 checks a refinement's step rows, over a model subject and a protocol subject

## Description

Verify that every concrete operation, and over a protocol subject every
concrete step class, needs exactly one written-out step row, and that
receivers, arguments and results are typed against the abstract operation.

Scope: FR-136-AC-1 to FR-136-AC-10.

## Test Procedure

Fixtures: `CasRefinesCounter` and `RingIsQueue` (ADR-020 §8); FR-136-AC-4's
protocol subject over a model with operations `incA` and `incB`.

1. Check `CasRefinesCounter` and `RingIsQueue`.
2. Remove the `peek` row; write it twice; add a row for
   `Impl::Counter::nope`.
3. Check each right-side variant of FR-136-AC-3.
4. Check the protocol subject with no control-node rows, then with `fork`,
   `join` and `finish` rows, then with an extra attempt-node row, then with
   two `join` rows.
5. Check `CasRefinesCounter` with its rows in reverse source order and
   compare the emitted `steps` and each row's `StepRowRef`.
6. Check FR-136-AC-4's protocol (named `Run`) with rows naming `Run` only;
   with a row naming a second protocol clause `Other`; a row naming
   `Nope`; a row naming a clause that acts only on another model; a
   `step memory flush` row under a model subject and one naming a kind the
   memory model lacks; and an operation row for an operation no attempt uses.
7. Write the rows (`fork`, `join`, `finish`, a `duplicate` and a `lose` row
   of the `unknown`-delivery channel `ch`, `memory flush`) of FR-136-AC-7's
   protocol, which has `ch` and a `tso` outermost `parallel`, in source order
   and in reverse; compare the emitted `steps` and each `StepRowRef`.
8. Declare the protocol before and after the refinement in the source; give
   the protocol a defect of its own and leave one coverage row missing.
9. Name protocols `P`, `Q` and `R` in rows; name a clause of another package.
10. Check `memory flush` and `memory nope` rows over the `tso` protocol; a
    `memory flush` row over an `sc` protocol; admit a check request selecting
    `tso` for the `sc` protocol with no `flush` row, and one selecting `sc`
    for the `tso` protocol; check a protocol with an `sc` and a `tso`
    outermost `parallel` and one `flush` row.

Tag the tests `#[trace("TC-541", "FR-136-AC-n")]`.

## Expected Results

- Step 1: the step rows of FR-136-AC-1.
- Step 2: `missing_declaration`/`missing-name` naming `peek`;
  `invalid_model_binding`/`conflicting-binding` naming both rows;
  `missing_declaration`/`missing-name`.
- Step 3: `ill_typed`/`operator-ineligible`;
  `ill_typed`/`type-mismatch` at the argument; `arguments [None]`;
  `ill_typed`/`type-mismatch`.
- Step 4: `missing_declaration`/`missing-name` naming the `fork` node;
  checks; checks with the node row recorded;
  `invalid_model_binding`/`conflicting-binding`.
- Step 5: the same emitted `steps`, in ascending node-key digest order, and the same `StepRowRef`s.
- Step 6: one protocol, recorded as `concrete_protocol`, and `None` for a
  refinement that names none; `invalid_model_binding`/`conflicting-binding`
  at the second protocol's first row naming both rows;
  `missing_declaration`/`missing-name` at `Nope`;
  `invalid_model_binding`/`malformed-declaration` at the protocol name;
  `missing_declaration`/`missing-name` at the kind for both memory rows; the
  unused operation row checks.
- Step 7: equal `steps` arrays and `StepRowRef`s, ordered by node-key digest,
  then step keyword, then memory kind name.
- Step 8: the same coverage result whichever declares first; the protocol's
  own refusals and no coverage refusal for a class drawn from it, and no
  `CheckedRefinement`.
- Step 9: two `conflicting-binding` refusals, at `Q`'s and `R`'s first rows,
  each naming that row and `P`'s first row; `missing-name` for the foreign
  clause.
- Step 10: the `flush` row checks over `tso`; `nope` refuses `missing-name`;
  the `sc` protocol refuses at `flush`; the request selecting `tso` refuses
  `missing-name` naming `flush` when admitted; the request selecting `sc`
  admits with the row unused; one row covers both `parallel` nodes.
