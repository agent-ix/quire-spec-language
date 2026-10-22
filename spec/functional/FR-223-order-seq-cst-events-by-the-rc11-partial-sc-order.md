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
`F(ℓ)`, the happens-before frontiers carried by views, the `owed` sets that keep a
collected event's predecessors, the pruning of a
`seq_cst` access or fence whose edges would close a cycle in `P`, and the
removal of collected events (ADR-025 PSC-1 to PSC-6), less the RC11 edges
that PSC-7 omits. `P`, `F`, the frontiers and the `owed` sets SHALL be part of
the state key.

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

- `Ra` SHALL keep `P`, `F(ℓ)`, the frontiers and the `owed` sets as members of its
  component, with each event named by its thread and message position.
- When a step occurs, `Ra` SHALL update `P`, `F(ℓ)`, the frontiers and the
  `owed` sets by
  QSpec FR-437's new-event, frontier and collection rules.
- When a `seq_cst` access or fence `x` of thread `b` executes, `Ra` SHALL add
  to `P` an edge to `x` from each live SC event of `b`; from each live SC event
  on `x`'s location that happens before `b`'s current view; from each live
  `seq_cst` fence of another thread that happens before `b`'s current view;
  and, when `x` is a fence, from each live SC access of another thread that
  happens before `b`'s current view. `Ra` SHALL decide these edges from the
  frontier of `b`'s current view at `x`, after the joins `x` itself makes: the
  message a `seq_cst` load reads, and the acquire part and `S` of a `seq_cst`
  fence.
- `Ra` SHALL record a `seq_cst` fence in the frontier of its thread's current
  view, of `S` and of its release views as the fence becomes an SC event.
- `Ra` SHALL build no edge for the `eco` part of RC11's fence clause.
- `Ra` SHALL NOT build the edges ADR-025 PSC-7 (a) and (b) omit: the
  `po|≠loc ; hb ; po|≠loc` edge from an SC access, and the `psc_base` edges
  with an SC fence at one end whose `scb` step is an `mo` or `rb` step with a
  non-SC access at its fence-side end.
- `Ra` SHALL enable a `seq_cst` access or fence with a given message or
  slot choice only when QSpec FR-437's acyclicity check on the resulting
  `P` passes, and SHALL give no step for a pruned choice.
- When garbage collection removes an SC event, `Ra` SHALL keep its live
  predecessors as predecessors of every later SC event of its thread; of every
  later SC write to its location; and of every later SC event on its location
  by a thread whose current view's frontier covers the removed event, by QSpec
  FR-437's collection rule.
- While some view, at rest or in flight on a channel, has a `seq_cst` fence as
  the latest live fence of its thread that the view's frontier covers,
  `Ra` SHALL keep the fence in `P`, and no longer.
- `Ra` SHALL serialize `P`, `F`, the frontiers and the `owed` sets in the
  canonical state key (FR-229), so that states with equal futures coalesce.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-223-AC-1 | `SB` with every store and load `seq_cst` and no fence settles `proved` under `ra`. At the state after `Sx`, `Ly` (reading `y0`) and `Sy`, the load `Lx` is enabled reading `x1` and not reading `x0`, because the edges `Sx → Ly → Sy → Lx → Sx` would close a cycle. | Test (TC-668) |
| FR-223-AC-2 | IRIW with every access `seq_cst` settles with its weak outcome unreachable under `ra`, and IRIW with `release` stores and `acquire` loads with it reachable. `SB` with `seq_cst` fences and `relaxed` accesses settles `proved` under `ra`. | Test (TC-668) |
| FR-223-AC-3 | Two threads each running a `repeat` of maximum 3 over a `seq_cst` store to `x` and a `seq_cst` load of `y`: at every reachable state, the number of live SC events in `P` is at most the live messages times the threads, and every event on a removed message is absent from `P`. | Test (TC-668) |
| FR-223-AC-4 | Running AC-1 and AC-3 twice gives equal state counts and byte-equal keys; two behaviours that differ only in SC events since removed reach one state. | Test (TC-668) |
| FR-223-AC-5 | ADR-025 §15's LT-12 (a release flag between an SC write and an SC read through a third thread) and LT-13 (an SC access pair against a `relaxed` store, an SC fence and a `relaxed` load) each reach their weak outcome under `ra`, the shapes PSC-7 names, and neither is reachable under `sc` or `tso`. Every outcome that an independent RC11 execution-graph checker allows for the two programs is reachable under `ra`. | Test (TC-668) |
| FR-223-AC-6 | ADR-025 §15's LT-14, in which thread `u` stores `X := 1` with `seq_cst`, fences with `seq_cst` and stores `Y := 1` with `relaxed`, thread `v` acquires `Y` and stores `P := 1` with `seq_cst`, and thread `w` stores `P := 2` and loads `X` with `seq_cst`, does not reach `a = 1`, `b = 0` and final `P = 2`: at the state where `u` has run, `v` has read `Y = 1` and stored `P := 1`, and `w` has stored `P := 2` after it in modification order, `w`'s load of `X` is enabled for `X = 1` only. LT-18, which is LT-12 with a `seq_cst` fence between `v`'s two loads, does not reach its weak outcome. | Test (TC-668) |
| FR-223-AC-7 | ADR-025 §15's LT-15 does not reach a = 2, b = 3 and final `X = 1` under `ra`, whether or not garbage collection has removed the SC event on `L := 2`, the earlier message `t` wrote: when it has, its live predecessors, among them `X := 1`, are kept in `owed(t)`, `u`'s `X := 1` still precedes `t`'s `X := 2` in the SC event graph, and the slot for `X := 2` before `X := 1` in modification order is not offered. | Test (TC-668) |
| FR-223-AC-8 | ADR-025 §15's LT-16 does not reach a = 2, b = 3, c = 3, d = 4 and final `X = 1` under `ra`: `w`'s view covers `L := 2` through the view of `L := 3` and the acquire fence, though the message `d` reads (`L := 4`) does not; when `L := 2` has been collected, `w`'s view holds its predecessors, `w`'s `seq_cst` load takes them, and the slot for `w`'s `X := 2` before `X := 1` in modification order is not offered. | Test (TC-668) |
| FR-223-AC-9 | ADR-025 §15's LT-17 does not reach a = 1, b = 0 and final `P = 2` under `ra` in any interleaving, including those in which `u`'s second fence runs before `v` loads `Y`: the first fence is still in `P` when `v` stores `P := 1`, because `Y`'s message view covers it and not the second fence. | Test (TC-668) |
| FR-223-AC-10 | A thread running two `seq_cst` fences with no store between them and no other thread has, after its second fence, the first fence absent from `P`; the same thread storing a `release` flag between the fences keeps the first fence in `P` after its second fence, for as long as the flag's message is live and its view covers the first fence and not the second. | Test (TC-668) |

## Dependencies

- ADR-025 §15 PSC-1 to PSC-7, RA-5, RA-7.
- FR-222 (`Ra`), FR-229 (state key).
- QSpec FR-437 owns RC11's `seq_cst` semantics under `ra` and its litmus
  vectors.

## References

- Owning ticket: Linear QSL-372. QSpec half: QSpec FR-437 (Linear STD-138).
