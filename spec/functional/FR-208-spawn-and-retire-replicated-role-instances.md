---
id: FR-208
title: "Spawn and retire replicated role instances"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-024
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-027
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-205
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-206
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-218
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-050
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-055
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-171
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-428
    type: depends_on
---
# FR-208: Spawn and retire replicated role instances

## Description

The protocol system SHALL run replicated roles as QSpec FR-428 states: one
role instance per spawned object of the role's population universe, at most
the authored `max n` live at once, any live instance acting for an event
step `by r`, and retirement by lifetime (ADR-027 RR-1 to RR-4, ST-14,
ST-15). The state key SHALL identify an instance by its object reference.

## Use case

A verification operator models workers drawn from a pool, at most two at a
time, each working until a condition holds. They set the pool's universe in
the subject, so they decide how many workers can ever exist, and the
protocol's own `max` decides how many run at once. A worker that has
retired is never spawned again, and spawning the same workers in a
different order does not count as a different state.

## Inputs

- A protocol state (FR-205), the checked replicated role declarations
  (FR-218) with their population, authored `max`, lifetime and the S3
  `scope` region for a `scope` lifetime, and the subject's universe for each
  role's population.

## Outputs

- `spawn(r, o)` and `retire(r, o)` steps; the state key's `roles` member
  mapping each (protocol instance, replicated role) to its instances' object references with
  status `live` or `retired`; the acting instance on each step `by r`.

## Behavior

- The protocol system SHALL give `spawn(r, o)` and `retire(r, o)` steps and
  retire instances by lifetime as QSpec FR-428 states, folding `workflow`
  and `scope` retirement into the step that ends the lifetime.
- The protocol system SHALL give an event step `by r` once per live
  instance of `r`, with its binder and transition identity naming the
  acting instance.
- The `roles` member SHALL key instances by protocol instance ordinal and
  object reference, keep a retired instance with status `retired` while its
  protocol instance is live, and treat QSpec FR-171's spawn ordinal as trace
  data that replay recomputes.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-208-AC-1 | A role `r each Worker from workers max 1 lifetime until(p)`, universe `{w1, w2}`, a body `event Work by r …` and `p` true once the instance has worked: `spawn(r, w1)` and `spawn(r, w2)` are both enabled initially; after `spawn(r, w1)` neither spawn is enabled until `retire(r, w1)`; after it `spawn(r, w2)` is enabled and `spawn(r, w1)` never is again, and `w1` stays in `roles` as `retired`. | Test (TC-653) |
| FR-208-AC-2 | With `max 2`, spawning `w1` then `w2` and spawning `w2` then `w1` reach states with equal keys. With two live instances, `event(Work)` gives two steps whose identities name `w1` and `w2`. | Test (TC-653) |
| FR-208-AC-3 | With lifetime `workflow`, every live instance retires in the `finish` step, which removes the protocol instance with its role instances, so the state after `finish` holds none. With lifetime `scope`, an instance retires in the step after which the last `by r` node of its region is unreachable from every live thread, and no `retire` step exists. | Test (TC-653) |

## Dependencies

- ADR-027 §2.2 ST-14 and ST-15, §2.2a RR-1 to RR-4.
- FR-205 (`roles` member), FR-206 (event steps), FR-218 (checked roles and
  the `scope` region).
- QSpec owns replicated roles' meaning (ADR-027 QS-11): QSpec FR-050,
  FR-055, FR-171.

## References

- Owning ticket: Linear QSL-396. QSpec half: QSpec FR-428 (Linear STD-140).
