---
id: FR-223
title: "Order seq_cst accesses and fences under ra by the RC11 partial SC order"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-025
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-025
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-222
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-229
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-437
    type: depends_on
---
# FR-223: Order seq_cst accesses and fences under ra by the RC11 partial SC order

## Description

Under `ra`, `Ra` SHALL implement QSpec FR-437's RC11 partial SC order: the
live SC events, the acyclic graph `P` over them, the per-location summary
`F(ℓ)`, the happens-before frontiers carried by views, the pruning of a
`seq_cst` access or fence whose edges would close a cycle in `P`, and the
removal of collected events (ADR-025 PSC-1 to PSC-6). `P`, `F` and the
frontiers SHALL be part of the state key.

## Use case

A verification operator's Rust code uses `SeqCst` accesses, as much Rust
code does. Reading them as acquire and release would report outcomes the
code cannot have; reading them as fenced accesses would prove too much. The
check gives exactly RC11's outcomes, so store buffering with `SeqCst`
accesses is proved and independent readers of `SeqCst` writes agree.

## Inputs

- The `Ra` component (FR-222) and each step's SC events.

## Outputs

- `P`, `F(ℓ)` and the frontiers as members of the `Ra` component, encoded
  with events named by thread and message position.
- The pruning of every message or slot choice whose SC event would close a
  cycle in `P`.

## Behavior

- `Ra` SHALL keep `P`, `F(ℓ)` and the frontiers as members of its
  component, with each event named by its thread and message position.
- When a step occurs, `Ra` SHALL update `P`, `F(ℓ)` and the frontiers by
  QSpec FR-437's new-event, frontier and collection rules.
- `Ra` SHALL enable a `seq_cst` access or fence with a given message or
  slot choice only when QSpec FR-437's acyclicity check on the resulting
  `P` passes, and SHALL give no step for a pruned choice.
- `Ra` SHALL serialize `P`, `F` and the frontiers in the canonical state key
  (FR-229), so that states with equal futures coalesce.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-223-AC-1 | `SB` with every store and load `seq_cst` and no fence settles `proved` under `ra`. At the state after `Sx`, `Ly` (reading `y0`) and `Sy`, the load `Lx` is enabled reading `x1` and not reading `x0`, because the edges `Sx → Ly → Sy → Lx → Sx` would close a cycle. | Test (TC-668) |
| FR-223-AC-2 | IRIW with every access `seq_cst` settles with its weak outcome unreachable under `ra`, and IRIW with `release` stores and `acquire` loads with it reachable. `SB` with `seq_cst` fences and `relaxed` accesses settles `proved` under `ra`. | Test (TC-668) |
| FR-223-AC-3 | Two threads each running a `repeat` of maximum 3 over a `seq_cst` store to `x` and a `seq_cst` load of `y`: at every reachable state, the number of live SC events in `P` is at most the live messages times the threads, and every event on a removed message is absent from `P`. | Test (TC-668) |
| FR-223-AC-4 | Running AC-1 and AC-3 twice gives equal state counts and byte-equal keys; two behaviours that differ only in SC events since removed reach one state. | Test (TC-668) |

## Dependencies

- ADR-025 §15 PSC-1 to PSC-6, RA-5, RA-7.
- FR-222 (`Ra`), FR-229 (state key).
- QSpec FR-437 owns RC11's `seq_cst` semantics under `ra` and its litmus
  vectors.

## References

- Owning ticket: Linear QSL-372. QSpec half: QSpec FR-437 (Linear STD-138).
