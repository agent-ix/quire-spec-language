---
id: FR-221
title: "Explore the x86-TSO memory model"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-025
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-025
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-027
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-220
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-229
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-436
    type: depends_on
---
# FR-221: Explore the x86-TSO memory model

## Description

Crate `qsl-eval`'s `simulation` module SHALL provide `Tso: MemoryModel` (ADR-027 SE-1), which
implements QSpec FR-436's x86-TSO rules through the memory-model seam: its
component is one FIFO store buffer per thread of the active weak
`parallel`, and its internal step is `flush(b)` (ADR-025 TSO-1 to TSO-8).

## Use case

A verification operator's code runs on x86. They check the store-buffering
protocol under `tso` and see the counterexample in which each thread's
store is still in its buffer when the other thread loads, then add
`seq_cst` fences and see the claim proved.

## Inputs

- The protocol state (ADR-027 PS-8) with the `Tso` component, and each
  access's classification and ordering (FR-220).

## Outputs

- `Tso`'s component: for each thread, a sequence of (location, value) pairs
  in program order, as the state key's memory member (FR-229).
- `flush(b)` memory steps with a typed canonical identity naming the
  thread.

## Behavior

- `Tso` SHALL implement QSpec FR-436's store, load, flush, locked-access,
  fence, fork, join and send rules, and SHALL apply them only through the
  ADR-027 SE-1 members: the buffers as the component (a), a thread's load
  as observe (b), a store's delta as write (c), `flush(b)` as the one
  internal step (d), the empty-buffer conditions as the gate (e), and the
  empty branch buffers as split and merge (f).
- `Tso` SHALL give each `flush(b)` a typed canonical transition identity
  naming the thread, and SHALL serialize its component as the state key's
  memory member (FR-229).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-221-AC-1 | ADR-025 §10's `SB` under `tso` reaches 34 states in the `parallel` and 4 joined states, 38 in all, and the claim settles `refuted` with the counterexample `Sx`, `Ly`, `Sy`, `Lx` of §10's table: at position 4 memory holds `x = 0`, `y = 0`, `left`'s buffer `[x := 1]`, `right`'s `[y := 1]`, and `c1.r = c2.r = 0`. | Test (TC-666) |
| FR-221-AC-2 | `SB` with `fence F ordering seq_cst;` between each store and load settles `proved` under `tso`; so does `SB` with every store and load `seq_cst` and no fence. | Test (TC-666) |
| FR-221-AC-3 | One branch storing `x := 1` then `y := 1`, beside a branch that loads `x` and then `y`, so that both locations are shared: after both stores the storing thread's buffer is `[x := 1, y := 1]`, and the first `flush` writes `x` only. A load of `x` by the same thread before any flush reads 1; a load by another thread reads 0. A `fetch_add` by that thread is not enabled until both flushes. | Test (TC-666) |
| FR-221-AC-4 | In `SB`, `join(Both)` is not enabled at a state where both threads are done and either buffer is non-empty, and every joined state has no buffer. A `send` by a thread with a non-empty buffer is not enabled. Under `tso`, the message-passing, load-buffering, IRIW, 2+2W and CoRR litmus tests of ADR-025 §3.3's table each settle with their `tso` column: the weak outcome is unreachable. | Test (TC-666) |

## Dependencies

- ADR-025 §3.2 TSO-1 to TSO-8 (as amended by ADR-027); ADR-027 SE-1.
- FR-220 (classification and orderings), FR-229 (the component in the
  state and the seam).
- QSpec FR-436 owns the `tso` semantics and the litmus vectors.

## References

- Owning ticket: Linear QSL-372. QSpec half: QSpec FR-436 (Linear STD-138).
