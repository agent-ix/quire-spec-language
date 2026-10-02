---
id: FR-222
title: "Explore the release-acquire memory model with relaxed and seq_cst accesses"
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
  - target: ix://agent-ix/quire-spec-language/FR-223
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-229
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-437
    type: depends_on
---
# FR-222: Explore the release-acquire memory model with relaxed and seq_cst accesses

## Description

Crate `qsl-eval`'s `simulation` module SHALL provide `Ra: MemoryModel` (ADR-027 SE-1), which
implements QSpec FR-437's RC11 release-acquire rules through the
memory-model seam, with the SC event graph of FR-223 for `seq_cst`
accesses (ADR-025 RA-1 to RA-8).

## Use case

A verification operator's Rust code uses `Release` stores and `Acquire`
loads to pass a message, and `Relaxed` counters elsewhere. They check the
protocol under `ra` and get the outcomes RC11 gives: message passing holds
with release and acquire and fails with relaxed flags, and independent
readers may disagree on the order of independent writes.

## Inputs

- The protocol state with the `Ra` component, and each access's
  classification and ordering (FR-220).

## Outputs

- `Ra`'s component: for each shared location, its messages in mo, each with
  a value, a message view and an attached mark; each thread's three views;
  `S`; and FR-223's SC event graph and FR-224's race summary.
- Load and store steps whose transition identities name the message read,
  or the insertion slot, by **depth** from the mo-last message.

## Behavior

- `Ra` SHALL implement QSpec FR-437's entry, load, store, message-view,
  RMW, fence, fork, join, channel and garbage-collection rules, and SHALL
  apply them only through the ADR-027 SE-1 members: the messages and views
  as the component (a), a load's message choice as observe (b), a store's
  inserted message as write (c), the view split and join as split and merge
  (f), and the views a `send` carries as the channel effect.
- `Ra` SHALL name the message a load reads, or the slot a store inserts at,
  by its depth from the mo-last message in the step's transition identity.
- `Ra` SHALL serialize its component, FR-223's graph and FR-224's race
  summary as the state key's memory member (FR-229).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-222-AC-1 | ADR-025 §10's `SB` under `ra` reaches 14 states in the `parallel` and 4 joined states, 18 in all, and the claim settles `refuted` with the counterexample of §10's `ra` table: `Sx`, `Ly` at depth 0 (`y0`), `Sy`, `Lx` at depth 1 (`x0`), with the messages and views that table gives at each position. | Test (TC-667) |
| FR-222-AC-2 | Each litmus test of ADR-025 §3.3's table, run under `ra`, reaches its weak outcome exactly when its `ra` column says allowed: SB allowed; SB with `seq_cst` fences forbidden; MP with `release`/`acquire` flag forbidden; MP with `relaxed` flag allowed; LB forbidden; IRIW with `release`/`acquire` allowed; 2+2W with `release` stores allowed; CoRR forbidden. Each verdict equals the expected outcome of QSpec FR-437's litmus vectors. | Test (TC-667) |
| FR-222-AC-3 | Two threads each `fetch_add(1)` with `ordering relaxed` on a location at 0: every joined state holds 2, and no state has two RMWs reading one message. After `SB`'s `join all`, each location holds one message. A message earlier in mo than every view's message at its location is absent from the next state. | Test (TC-667) |
| FR-222-AC-4 | MP with `relaxed` flag accesses, a `release` fence before the flag store and an `acquire` fence after the flag load, settles `proved` under `ra`. MP whose flag is passed by a channel `send` and `receive` in place of the flag location settles `proved` under `ra`. | Test (TC-667) |

## Dependencies

- ADR-025 §3.3 RA-1 to RA-8 and the litmus table (as amended by ADR-027);
  ADR-027 SE-1.
- FR-220, FR-223 (SC event graph), FR-229 (the component in the state).
- QSpec FR-437 owns the `ra` semantics and the litmus vectors.

## References

- Owning ticket: Linear QSL-372. QSpec half: QSpec FR-437 (Linear STD-138).
