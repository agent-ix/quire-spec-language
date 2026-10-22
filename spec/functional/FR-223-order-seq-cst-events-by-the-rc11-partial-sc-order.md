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
removal of collected events (ADR-025 PSC-1 to PSC-6), less the RC11 edges
that PSC-7 omits. `P`, `F` and the
frontiers SHALL be part of the state key.

## Use case

A verification operator's Rust code uses `SeqCst` accesses, as much Rust
code does. Reading them as acquire and release would report outcomes the
code cannot have; reading them as fenced accesses would prove too much. The
check gives RC11's outcomes except for the shapes of ADR-025 PSC-7, where it
admits more and never fewer, so store buffering with `SeqCst` accesses is
proved and independent readers of `SeqCst` writes agree, and a `proved`
verdict holds under RC11.

## Inputs

- The `Ra` component (FR-222) and each step's SC events.

## Outputs

- `P`, `F(ℓ)`, the frontiers and the `owed` sets (ADR-025 PSC-4) as members of
  the `Ra` component, encoded with events named by thread and message
  position.
- The pruning of every message or slot choice whose SC event would close a
  cycle in `P`.

## Behavior

- `Ra` SHALL keep `P`, `F(ℓ)` and the frontiers as members of its
  component, with each event named by its thread and message position.
- When a step occurs, `Ra` SHALL update `P`, `F(ℓ)` and the frontiers by
  QSpec FR-437's new-event, frontier and collection rules.
- A new SC event `x` of thread `b` SHALL take an edge from each live SC event
  of `b`; from each live SC event on `x`'s location that happens before `b`'s
  current view; and from each live `seq_cst` fence of another thread that
  happens before `b`'s current view, decided from the frontier. A fence's
  `eco` clause SHALL build no edge, since the join of `S` at a `seq_cst` fence
  already excludes the choices that would complete it.
- `Ra` SHALL NOT build the edges ADR-025 PSC-7 omits: the
  `po|≠loc ; hb ; po|≠loc` edge from an SC access, and an edge between an SC
  fence and an SC access derived through a non-SC event adjacent to the fence.
  The omission SHALL only enlarge the set of explored behaviours.
- `Ra` SHALL enable a `seq_cst` access or fence with a given message or
  slot choice only when QSpec FR-437's acyclicity check on the resulting
  `P` passes, and SHALL give no step for a pruned choice.
- When garbage collection removes an SC event, `Ra` SHALL keep its live
  predecessors as predecessors of every later SC event of its thread, of every
  later SC write to its location (or, for a read, every later write to it), and
  of every later SC event on its location by a thread whose view covers it, by
  QSpec FR-437's collection rule.
- `Ra` SHALL serialize `P`, `F`, the frontiers and the `owed` sets in the
  canonical state key (FR-229), so that states with equal futures coalesce.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-223-AC-1 | `SB` with every store and load `seq_cst` and no fence settles `proved` under `ra`. At the state after `Sx`, `Ly` (reading `y0`) and `Sy`, the load `Lx` is enabled reading `x1` and not reading `x0`, because the edges `Sx → Ly → Sy → Lx → Sx` would close a cycle. | Test (TC-668) |
| FR-223-AC-2 | IRIW with every access `seq_cst` settles with its weak outcome unreachable under `ra`, and IRIW with `release` stores and `acquire` loads with it reachable. `SB` with `seq_cst` fences and `relaxed` accesses settles `proved` under `ra`. | Test (TC-668) |
| FR-223-AC-3 | Two threads each running a `repeat` of maximum 3 over a `seq_cst` store to `x` and a `seq_cst` load of `y`: at every reachable state, the number of live SC events in `P` is at most the live messages times the threads, and every event on a removed message is absent from `P`. | Test (TC-668) |
| FR-223-AC-4 | Running AC-1 and AC-3 twice gives equal state counts and byte-equal keys; two behaviours that differ only in SC events since removed reach one state. | Test (TC-668) |
| FR-223-AC-5 | ADR-025 §15's LT-12 (a release flag between an SC write and an SC read through a third thread) and LT-13 (an SC access pair against a `relaxed` store, an SC fence and a `relaxed` load) each reach their weak outcome under `ra`, the shapes PSC-7 names, and neither is reachable under `sc` or `tso`. Every other litmus outcome of FR-222-AC-2 and FR-223-AC-2 is unchanged. | Test (TC-668) |
| FR-223-AC-6 | ADR-025 §15's LT-14, in which thread `u` stores `X := 1` with `seq_cst`, fences with `seq_cst` and releases `Y`, thread `v` acquires `Y` and stores `P := 1` with `seq_cst`, and thread `w` stores `P := 2` and loads `X` with `seq_cst`, does not reach `a = 1`, `b = 0` and final `P = 2`: at the state where `u` has run, `v` has read `Y = 1` and stored `P := 1`, and `w` has stored `P := 2` after it in modification order, `w`'s load of `X` is enabled for `X = 1` only. | Test (TC-668) |
| FR-223-AC-7 | ADR-025 §15's LT-15 does not reach a = 2, b = 3 and final `X = 1` under `ra`: once `b`'s load has read `L := 3`, every current view of `L` is at that message, the SC events on `L := 1` and `L := 2` are collected, and the slot for `t`'s `X := 2` before `u`'s `X := 1` in modification order is not offered, because `u`'s `X := 1` still precedes `t`'s `X := 2` in the SC event graph through `owed(t)`. | Test (TC-668) |
| FR-223-AC-8 | ADR-025 §15's LT-16 does not reach a = 2, b = 3, c = 3, d = 3 and final `X = 1` under `ra`: after `L := 2` is collected, `w`'s `seq_cst` load of `L` after the acquire fence still takes `L := 2`'s predecessors, so the slot for `w`'s `X := 2` before `X := 1` in modification order is not offered. | Test (TC-668) |

## Dependencies

- ADR-025 §15 PSC-1 to PSC-7, RA-5, RA-7.
- FR-222 (`Ra`), FR-229 (state key).
- QSpec FR-437 owns RC11's `seq_cst` semantics under `ra` and its litmus
  vectors.

## References

- Owning ticket: Linear QSL-372. QSpec half: QSpec FR-437 (Linear STD-138).
