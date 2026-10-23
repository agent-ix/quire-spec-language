---
id: FR-225
title: "Bound the memory component by request method bounds, default 4, stated in the result"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-025
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-025
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-127
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-158
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-221
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-222
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-331
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-439
    type: depends_on
---
# FR-225: Bound the memory component by request method bounds, default 4, stated in the result

## Description

`ModelCheckRequest` SHALL carry `max_store_buffer` and `max_messages` (FR-126)
beside `max_depth`
as method bounds, like the `max_depth` search horizon: bounds on the
explored state space that every result states, not resource limits. The
request sets them with no ceiling, and their published default is 4
(ADR-025 MB-2). The model checker SHALL apply them and settle under them as
QSpec FR-439 states (ADR-025 MB-3, MB-4), and every terminal record of an
item over a subject with a weak `parallel` SHALL state the bound used and
whether it was reached.

## Use case

A verification operator checks a protocol whose loop stores repeatedly, so
its store buffer could grow without end. The check runs with a buffer of 4
by default and says so; if the bound was reached and nothing was found, the
result is `inconclusive`, and the operator raises the bound and runs
again.

## Inputs

- `ModelCheckRequest.max_store_buffer: Option<u64>` and
  `ModelCheckRequest.max_messages: Option<u64>` (FR-126); `None` means 4.
- The `Tso` or `Ra` component (FR-221, FR-222).

## Outputs

- `ModelCheckOutcome::Undecided(MemoryBoundReached { bound, states })`,
  settled by FR-127 as V-6, with `bound` naming the request member that
  raises it and its value.
- In every FR-331 terminal record of an item over a subject with a weak
  `parallel`: the method's memory bound value and whether it was reached.

## Behavior

- When a request leaves a memory bound unset, the model checker SHALL run
  with 4. The model checker SHALL treat the bounds as method parameters outside
  the model, the subject and its obligation identity.
- The model checker SHALL leave unexpanded each store that QSpec FR-439's
  bound rule cuts, expand every other enabled step, record the state as
  bound-limited, and read a state whose only enabled steps are cut stores as
  a boundary state (ADR-025 MB-3).
- The model checker SHALL return the outcome QSpec FR-439's verdict table
  gives a run under a bound, which FR-127 settles: V-4 with a counterexample, V-1 with the bound stated as not
  reached when no state was bound-limited, and otherwise V-6,
  `MemoryBoundReached{bound, states}`, with `bound` naming the request
  member that raises it (`max_store_buffer` or `max_messages`) and its
  value, and `states` the count of bound-limited states.
- The model checker SHALL return the first applicable cause, in the order
  `ConstraintReached` (ADR-021), `MemoryBoundReached`,
  `InstanceBoundReached` (ADR-027), then `BoundReached` (the completed
  `max_depth` search horizon).
- Under a state constraint (FR-158), the model checker SHALL evaluate the
  constraint at a reached state before expanding it, on the memory values
  FR-158 "Protocol subjects" states. A state where the constraint is false is
  a constraint boundary state: it is counted once, in `ConstraintReached`'s
  `boundary_states`, and is never expanded, so no store is cut there and it
  is never bound-limited, also when its only enabled steps are stores the
  bound would cut. A state where the constraint holds is expanded under the
  bound rule above, and when the bound cuts a store there it is counted once,
  in `MemoryBoundReached`'s `states`, also when its only enabled steps are
  cut stores (ADR-025 MB-3); it is not counted in `boundary_states`. The
  terminal record states the memory bound as reached exactly when at least
  one state was bound-limited, whichever cause the run settles.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-225-AC-1 | ADR-025 §10's three requests leave both bounds unset: the `tso` and `ra` records state `max_store_buffer` 4 and `max_messages` 4 respectively, not reached; the `sc` record states no memory bound. Two `tso` requests that differ only in `max_store_buffer` carry equal obligation identities. | Test (TC-670) |
| FR-225-AC-2 | A branch that stores to `x`, `y` and `z` in sequence with no fence, beside a branch that loads `x`, `y` and `z`, so that all three are shared, under `tso` with the claim `always holds(x.v <= 1)`: with `max_store_buffer` 2 the run settles V-6 `MemoryBoundReached{bound: max_store_buffer 2, states}` with `states` at least 1; with 3 it settles V-1, bound not reached. | Test (TC-670) |
| FR-225-AC-3 | `SB` under `tso` with `max_store_buffer` 1 still settles `refuted` (V-4). `SB` under `ra` with `max_messages` 1 settles V-6 `MemoryBoundReached` with `bound: max_messages 1`, since no store can add a second message. | Test (TC-670) |
| FR-225-AC-4 | In AC-2 with bound 2, a state whose only enabled step is the third store is not reported by the deadlock-freedom item and has no stutter successor. AC-2's protocol written in ADR-025 §10's vocabulary (`Cell` with `v: Int[0, 1]`, universe `{x, y, z}` at 0, `set()` storing 1 with `ordering release`; `Core` with `r: Int[0, 2]` at 2; branch `left` on `c1` runs `set` on `x`, `y`, then `z`; branch `right` on `c2` runs `read(x)`, `read(y)`, then `read(z)` with `ordering acquire`), under `tso` with `max_store_buffer` 2, the claim `always holds(x.v <= 1)` and the request naming `constraint Zero using v on M::Cell at current { self.v <= 0 }`, stores 20 states: 12 expanded states with no store flushed, 4 of them bound-limited (`left`'s third store cut), and 8 boundary states reached by `left`'s first flush. It settles `ConstraintReached{boundary_states: 8}` with no counterexample, and its terminal record states `max_store_buffer` 2 as reached; the same request without `Zero` settles `MemoryBoundReached` (AC-2). | Test (TC-670) |
| FR-225-AC-5 | ADR-025 §10's `SB` under `ra` with `max_messages` 1 (AC-3) and the request naming `constraint One using v on M::Cell at current { self.v >= 1 }`, false at the initial state, stores one state and settles `ConstraintReached{boundary_states: 1}`; the initial state, whose only enabled steps are stores the bound would cut, is not bound-limited, and the terminal record states `max_messages` 1 as not reached. | Test (TC-670) |

## Dependencies

- ADR-025 §6 MB-2 to MB-4, MS-4; ADR-014 B-5; ADR-018 V-1, V-4, V-6.
- [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md)
  (`ModelCheckRequest`, outcomes),
  [FR-127](FR-127-settle-a-model-check-verdict-as-a-terminal-record.md)
  (the V-6 row and the terminal record), FR-221, FR-222.
- QSpec owns the memory bounds as request method bounds, their default, the bound
  semantics and verdicts, and their statement in the terminal record (QSpec
  FR-439, FR-331).

## References

- Owning ticket: Linear QSL-372. QSpec half: QSpec FR-439 (Linear STD-138).
