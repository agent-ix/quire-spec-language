---
id: FR-214
title: "Require an explicit refinement row for every protocol step class"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-024
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-027
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-206
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-215
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-218
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-177
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-432
    type: depends_on
---
# FR-214: Require an explicit refinement row for every protocol step class

## Description

When the concrete subject of a refinement declaration is a protocol
subject, S3 SHALL check its `step` rows by the refinement record's row check
(FR-136: `StepRow`, the protocol step classes, coverage and its refusals),
as QSpec FR-432 states (ADR-027 PR-1). This FR adds what a protocol on the
abstract side reads: the observation of each concrete step, computed from
its `StepRow` target, and the pairing of initial protocol states (ADR-027
PR-2, PR-3).

## Use case

A verification operator shows that a concurrent protocol implements a
sequential specification. Every step of the protocol, including each fork,
join and finish, is mapped in the declaration, so a step they forgot to
think about is a compile error and never passes the check unexamined.

## Inputs

- A parsed refinement declaration whose concrete subject names a protocol,
  with its `step` rows and, for an abstract protocol, the refinement
  profile's explicit list of internal abstract steps.
- The concrete protocol's checked clause (FR-218) and its memory model's
  internal step kinds (FR-215).

## Outputs

- `observation(row: &StepRow) -> Observation`, with `Observation` one of
  `Visible{label}`, `Internal` and `Either`, computed from the row's
  `StepTarget` and stored nowhere else.

## Behavior

- S3 SHALL check the rows of a refinement over a concrete protocol subject
  with FR-136's coverage and refusals, which already cover every protocol
  step class.
- `observation` SHALL return `Visible{label}` for a row whose target is an
  abstract operation or node, with the label that abstract step carries
  (its kind, its node or operation, and the binder record the row's
  arguments give); `Internal` for a `stutter` target; and `Either` for an
  `any` target.
- For an abstract protocol, an abstract step SHALL be internal exactly when
  the refinement profile's explicit list names it.
- Each concrete initial protocol state SHALL pair with the abstract initial
  model state its mapping gives, with the abstract root settled from its
  `run`.
- The refinement check over the rows is ADR-020's; this FR gives it the
  observation of each row.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-214-AC-1 | A refinement of an abstract model with operation `add()` by ADR-027 §7's `Fill`, with rows `step inc -> add()`, `step setTwo -> add()`, `step Fill::Both.fork -> stutter`, `step Fill::Both.join -> stutter` and `step Fill::Done -> stutter`, checks; every concrete step class has exactly one row, the attempts are `Visible` with `add`'s label and the control steps `Internal`. | Test (TC-659) |
| FR-214-AC-2 | Removing the `join` row refuses `missing_declaration`/`missing-name` naming `Both`'s join; removing the `finish` row refuses naming `Done`. Adding a second `step inc -> …` row refuses `invalid_model_binding`/`conflicting-binding`. | Test (TC-659) |
| FR-214-AC-3 | Adding `step Fill::Main::A -> stutter` beside `step inc -> add()` maps `A`'s steps by the node row (`Internal`) and leaves `inc`'s row unused by `A`. Over ADR-027 §7.1, omitting the `Refund` template's `cend` row refuses naming the template; over `Chatter` (FR-212), omitting the channel row refuses naming `ch`. | Test (TC-659) |
| FR-214-AC-4 | A row `-> any` gives `Either`. An abstract protocol whose profile lists `fork` as internal gives that abstract step internal, and its other abstract steps visible. | Test (TC-659) |

## Dependencies

- ADR-027 §6 PR-1 to PR-3; ADR-020's refinement check (References).
- FR-206 (step kinds), FR-215 (memory step kinds), FR-218 (checked
  clause).
- QSpec owns protocol refinement and the explicit step rows (QSpec FR-432,
  over FR-177) and the `step` row syntax.

## References

- Owning ticket: Linear QSL-396. QSpec half: QSpec FR-432 (Linear STD-140). Refinement
  record: ADR-020, Linear QSL-367.
