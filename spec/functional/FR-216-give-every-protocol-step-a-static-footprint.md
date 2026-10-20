---
id: FR-216
title: "Give every protocol step a static footprint, enabling footprint and visibility"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-024
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-027
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-206
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-207
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-208
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-209
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-215
    type: depends_on
---
# FR-216: Give every protocol step a static footprint, enabling footprint and visibility

## Description

`ProtocolSystem` SHALL give each step a static read footprint `R(s)` and
write footprint `W(s)` over model locations and the protocol locations
`ctl(t)`, `inst(i)`, `bind(n, t)`, `queue(h)`, `reg(c, ·)`, `role(r)`,
`pinst` and the memory model's locations (ADR-027 §5, FT-1), enforced by
construction for protocol locations (ADR-027 FT-2). It SHALL give each step
an enabling footprint (ADR-027 FT-3), and SHALL report independence and
visibility over these footprints as ADR-021 POR-4 and POR-5 read them
(ADR-027 FT-4, FT-5).

## Use case

A verification operator checks a safety claim over a protocol whose two
branches update different objects. State-space reduction needs to know that
the two branches' attempts commute and that forks and joins are invisible
to a claim over model state, so that it can explore one order where every
order gives the same answer.

## Inputs

- A protocol step (FR-206 to FR-209), its checked node, and the model read
  and write footprints ADR-021 POR-1 and POR-3 give its application.

## Outputs

- `footprint(step) -> Footprint { reads, writes }` and
  `enabling(step) -> LocationSet` on `ProtocolSystem`, over `Location`, the
  union of ADR-021's model locations, the protocol locations and the memory
  model's locations.

## Behavior

### Per-step footprints

- `attempt(n)` by thread `t`: `R` SHALL be the model read footprint of its
  application with the clauses `n` selects, the binders `n`'s constraint
  reads, and `ctl(t)`; `W` SHALL be the frame's model locations, `bind(n,
  t)`, `ctl(t)`, and `inst(i)` when the step ends `t`'s branch body.
- `event(n)` and `finish`: `R` the binders read and `ctl(t)`; `W`
  `bind(n, t)`, `ctl(t)`, and `inst(i)` when the step ends a branch. A step
  that ends a branch SHALL write `inst(i)`, changing only that branch's
  disposition, and the step that ends the last running thread of an instance
  whose join has settled SHALL also write the rest of `inst(i)`, since it
  removes the instance. `finish` SHALL also read the `ctl` of every thread of its
  protocol instance and write every `queue(h)` of it. An `event`, `send` or
  `finish` whose record type holds a reference SHALL read the membership of
  each population the reference ranges over.
- `send(n)` and `receive(n)`: as `event(n)`, with `queue(h)` in both `R` and
  `W`. `duplicate(h, e)` and `lose(h, e)`: `queue(h)` in both.
- `fork(p)` by `t`: `R` SHALL be `ctl(t)` and, for every ordinal `o` from 0
  up to the ordinal the fork takes (the smallest ordinal no live sibling
  holds), `inst(i_o)` of the parallel instance of `p` forked by `t` with
  ordinal `o`, the `ctl` of every thread at or below each of its branch
  threads and, because a live thread whose binder environment names a binder
  under that ordinal holds it, the `ctl` of every live thread at or below
  `t`; the binder environments (ADR-027 PS-5) are part of `ctl`, so the
  fork's write of each new thread's `ctl`, and `join`'s write of `ctl(t)`,
  carry them; `W` SHALL be `ctl(t)`, the new instance and its threads' `ctl`, and the
  `bind` locations of every path it reuses. An
  ordinal is held while any live thread's path extends the instance's
  branch paths, so those threads' `ctl` are read as stated.
  The fork reads the earlier ordinals because the ordinal it takes depends
  on which of them are live; without them, a non-commuting pair (a `repeat`
  around a `parallel` whose `outstanding continue` join leaves a branch
  running, with the step that ends that branch) would be reported
  independent. `join(p)` by `t`: `R` `inst(i)` and `ctl(t)`; `W`
  `ctl(t)`, `inst(i)` and, under `outstanding cancel`, the `ctl` of every
  branch thread of `i`, and for a branch that forked, the `ctl` of every
  thread at or below it and the inner parallel instances. `timeout(a)`: `ctl(t)` in both.
- A forward-effect step SHALL also write `reg(c, ·)`; a step that binds a
  trigger-type record SHALL read and write `reg(c, ·)`, write the new
  compensation thread's `ctl`, and read the `ctl` of each compensation
  thread of that template in the instance with an ordinal from 0 up to
  the ordinal it takes; a commit-node step SHALL write `reg(c, ·)`.
  `cattempt` SHALL be as `attempt` with its thread's `ctl`, its count and
  the previous attempt's binder; `cend` SHALL be its thread's `ctl`, `reg(c, r)` and the
  model locations its recovery predicate reads.
- `activate(T)`: `W` `pinst` and the new instance's `ctl`. `spawn` and
  `retire`: `role(r)` in both, and for `retire` the model and binder
  locations `p` reads. A step `by r` SHALL read `role(r)`.
- A memory step SHALL have its model's footprint; a gate SHALL add its
  memory locations to `R`.

### Enabling footprint, independence and visibility

- `ProtocolSystem::footprint` SHALL return the state-space reduction
  record's `Footprint{reads, writes, enabling}` (ADR-021 EI-4), with each
  protocol location as `Location::Protocol(ProtocolLocation)`,
  `ProtocolLocation` one of `ctl(t)`, `inst(i)`, `bind(n, t)`, `queue(h)`,
  `reg(c, ·)`, `role(r)` and `pinst`.
- A step's enabling footprint SHALL hold the model locations its
  precondition reads; the pre-state locations its postconditions read; the
  membership of its receiver and of each reference argument; the binders
  its constraint, guard or retry relation reads; its own `ctl(t)` and the
  instance locations it reads (`inst(i)` for a `join`, `reg(c, ·)` and the
  commit's occurrence for a `cattempt`, `pinst` for `activate`); `queue(h)`
  for a `receive`, a `send` under `block`, and `duplicate` and `lose` on
  `h`; `role(r)` for a step `by r` and for `spawn`; and its gate's memory
  locations.
- Two steps SHALL be independent exactly when ADR-021 POR-4 holds over
  their footprints. Two steps of one thread SHALL be dependent.
- A step SHALL be visible to a claim when its write footprint meets a model
  location an atom of the claim reads, or an atom reads its position's
  anchor and names its node, operation, channel, template or role.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-216-AC-1 | In ADR-027 §7, `attempt(A)` and `attempt(B)` at s1 both write (`c`, `v`) and are dependent. In a variant where `B` applies `setTwo` to another object `d` under a receiver-scoped frame, the two attempts are independent, and for a claim over `d.v` only, `attempt(A)` is invisible and `attempt(B)` visible. | Test (TC-661) |
| FR-216-AC-2 | In §7, `fork(Both)` writes `ctl` of `(0)`, `(0, Both, left, 0)` and `(0, Both, right, 0)` and `inst` of `Both`; `join(Both)` reads `inst` of `Both`. `fork`, `join` and `finish` are invisible to `always holds(k.v <= 3)`. Two steps of `left` are dependent. `attempt(A)`, which ends branch `left`, writes `inst(Both)`, changing only `left`'s disposition; `finish` reads the `ctl` of every thread of its instance and, in `Chatter`, writes `queue(ch)`. In FR-205-AC-3's `repeat` protocol, the second `fork(P)` reads `inst` of ordinal 0 of `P` and the `ctl` of its branch threads, and is dependent with the attempt that ends instance 0's last running branch; the two orders of that pair reach states with different ordinals; the fork also writes the `bind` locations of the paths it reuses. | Test (TC-661) |
| FR-216-AC-3 | Under `join any … outstanding cancel` (FR-206-AC-3), `join`'s write footprint holds the `ctl` of every branch thread; under `outstanding continue` it holds none of theirs. A `receive`'s enabling footprint holds its channel's `queue`, and in `Chatter` (FR-212) the `send` and `receive` on `ch` are dependent while `attempt(Done)` and `duplicate(ch, Tx)` are independent. When a branch of an `outstanding cancel` join holds a `parallel` that has forked, `join`'s write footprint also holds the `ctl` of every thread at or below that branch and the inner parallel instances. | Test (TC-661) |
| FR-216-AC-4 | In ADR-027 §7.1, `event(Paid)` writes `reg(Refund, ·)`; `event(Fail)` reads and writes it and writes the compensation thread's `ctl`. For each step of §7.1, recomputing its post-state while changing a location outside its `R` leaves the post-state's change confined to its `W`. Where template `Refund` has two registrations, `cend(Refund, r1)` of thread `(0, Refund, 0)` writes `reg(Refund, r1)` and `ctl` of that thread, and the step that activates the second registration reads that `ctl`; the two are dependent, and their two orders give the new thread ordinal 1 and 0. | Test (TC-661) |
| FR-216-AC-5 | An attempt of `bump(by)` on receiver `c` with reference argument `d`, whose precondition reads `c.v`, whose postcondition reads `pre(d.w)` and whose binder constraint reads binder `a`, has an enabling footprint holding (`c`, `v`), (`d`, `w`), the membership of `c` and of `d`, `bind` of `a`, its own `ctl(t)`, and nothing else; a `duplicate(ch, Tx)` step's enabling footprint holds `queue(ch)` of its instance. An `event` whose record type holds a `Reference<Peer>` reads the membership of the `Peer` population. In `Chatter`, `send` on `ch` is visible to an atom that reads its position's anchor and names `ch` and invisible to one naming another channel; the same holds for a template (`cattempt` of a compensation template) and for a role (a step `by` it). Each footprint is a `Footprint{reads, writes, enabling}` with protocol locations as `Location::Protocol`. | Test (TC-661) |

## Dependencies

- ADR-027 §5 FT-1 to FT-5; ADR-021 POR-1 to POR-5 (References).
- FR-206 to FR-209 (step kinds), FR-215 (`ProtocolSystem`).

## References

- Owning ticket: Linear QSL-396. State-space reduction record: ADR-021,
  Linear QSL-368.
