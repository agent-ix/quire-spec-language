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
- The transition identity of `spawn(r, o)` and `retire(r, o)` SHALL hold the
  role, the object's reference and the ordinal of the protocol instance the
  step acts in, so the same object's `spawn` in two protocol instances has two
  identities (ADR-027 TS-2, QSpec FR-426). No thread owns a `spawn` or a
  `retire`, so its identity holds no `thread` member.
- A `send` node SHALL be owned by the `from` role of its channel and a
  `receive` node by the `to` role of its channel.
- While the role that owns a `send` or `receive` node is replicated, the
  protocol system SHALL give the node once per live instance, naming the
  acting instance in its transition identity.
- The `roles` member SHALL key instances by protocol instance ordinal and
  object reference, keep a retired instance with status `retired` while its
  protocol instance is live, and treat QSpec FR-171's spawn ordinal as trace
  data that replay recomputes.

### The `retire` predicate

The `retire` predicate `p` of `lifetime until(p)` holds for an instance over
the names in scope and the model observation (ADR-027 ST-15, QSpec FR-428).
Its names in scope are the role's instance binder, of type `Reference<T>`
for `role r each T`, the protocol captures and constants (QSpec FR-464,
stage `role-lifetime`).

- S3 SHALL check `p` through the clause checker over those names with the
  model observation in scope, and SHALL admit in it every model-state read
  a state clause admits (FR-104): a field read through the instance binder
  or another reference-typed value, an explicit dereference, a navigation or
  reachability over a relationship end, and a `lookup` or `allInstances`
  read of a population. No rule of FR-218 or FR-381 that bars model state
  applies to `p`.
- A population that `p` reads SHALL be named by its qualified name through
  the alias of the domain package that declares it, as in a state clause
  (`M::jobs`, QSpec FR-153), and S3 SHALL resolve it to that package's population. A name
  that resolves to no population of the package SHALL refuse with
  `missing_declaration`/`missing-name` at the name, retaining `role` and
  `name`.
- The protocol system SHALL evaluate `p` at each state for each live
  instance of the role, over that state's model observation, and its read
  footprint is FR-216's (ADR-027 FT-1): the fields it reads for each object
  of the universe, the membership of each population it reads, and
  `role(r)`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-208-AC-1 | A role `r each Worker from workers max 1 lifetime until(p)`, universe `{w1, w2}`, a body `event Work by r …` and `p` true once the instance has worked: `spawn(r, w1)` and `spawn(r, w2)` are both enabled initially; after `spawn(r, w1)` neither spawn is enabled until `retire(r, w1)`; after it `spawn(r, w2)` is enabled and `spawn(r, w1)` never is again, and `w1` stays in `roles` as `retired`. | Test (TC-653) |
| FR-208-AC-2 | With `max 2`, spawning `w1` then `w2` and spawning `w2` then `w1` reach states with equal keys. With two live instances, `event(Work)` gives two steps whose identities name `w1` and `w2`. | Test (TC-653) |
| FR-208-AC-3 | With lifetime `workflow`, every live instance retires in the `finish` step, which removes the protocol instance with its role instances, so the state after `finish` holds none. With lifetime `scope`, an instance retires in the step after which the last `by r` node of its region is unreachable from every live thread, and no `retire` step exists. | Test (TC-653) |
| FR-208-AC-4 | Under `activation on each` with protocol instances 0 and 1 live and `spawn` enabled for `w1` in each, the two `spawn(r, w1)` steps have different transition identities that differ in the instance ordinal and hold no `thread` member; the same holds for the two `retire(r, w1)` steps. | Test (TC-653) |
| FR-208-AC-5 | A channel `from r to c` with `r` replicated, `max 2` and two live instances gives two `send` steps whose identities name `w1` and `w2`; a channel `from c to r` gives two `receive` steps likewise; with an empty universe a thread at the `send` waits with `NoRoleInstance{r}`. | Test (TC-653) |
| FR-208-AC-6 | In FR-216-AC-6's protocol, `role w each M::Worker from workers max 2 lifetime until(w.done and present(lookup<M::Job>(M::jobs, j) absent empty))`, with `j` a protocol capture of type `Reference<M::Job>`, checks, and `retire(w, w1)` is enabled exactly at the states where `w1.done` holds and `j` is a member of `jobs`; the same predicate with `M::absent` in place of `M::jobs` refuses `missing_declaration`/`missing-name` at `absent`, retaining `role` and `name`. | Test (TC-653) |

## Dependencies

- ADR-027 §2.2 ST-14 and ST-15, §2.2a RR-1 to RR-4.
- FR-205 (`roles` member), FR-206 (event steps), FR-218 (checked roles and
  the `scope` region).
- QSpec owns replicated roles' meaning (ADR-027 QS-11): QSpec FR-050,
  FR-055, FR-171.

## References

- Owning ticket: Linear QSL-396. QSpec half: QSpec FR-428 (Linear STD-140).
