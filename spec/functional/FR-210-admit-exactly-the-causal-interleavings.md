---
id: FR-210
title: "Admit exactly the causal interleavings of a protocol"
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
  - target: ix://agent-ix/quire-specification/FR-052
    type: depends_on
---
# FR-210: Admit exactly the causal interleavings of a protocol

## Description

The protocol system SHALL enable a step of node `m` only when every causal
predecessor of `m` in the compiled edge relation has occurred, and SHALL
order a step after a step of another thread only through a join, a queue, a
binder, a memory gate or the model state its own precondition or constraint
reads (ADR-027 CI-1, CI-2). The projections of its behaviours onto their
event steps SHALL be exactly the linear extensions of the causal edge
relation, unfolded along the choices and iterations taken, in which each
event step is enabled when reached (ADR-027 CI-3).

## Use case

A verification operator splits a shipment into parcels sent by independent
branches. They need every order in which the branches can run to be
checked, including orders that differ from how the branches are written,
and no order that a send-before-receive or a join forbids, so that a proof
covers the concurrent schedule and a counterexample shows one that really
can happen.

## Inputs

- The checked protocol clause's compiled edge relation (structural,
  `branch`, `join`, send.out → receive.in, attempt.out → effect.in and await
  anchor edges) and the protocol system's steps (FR-206).

## Outputs

- The protocol system's successor relation (FR-215), whose behaviours
  realise exactly the causal interleavings.

## Behavior

- The protocol system SHALL enforce each causal edge of the compiled edge
  relation through the mechanism QSpec FR-052's causal-interleavings rules
  name: the owning thread's rest point, the fork, the `join` step, the
  queue, the effect's binder condition and the await's anchor step.
- The protocol system SHALL add no ordering from source order, array order,
  timestamps or FIFO order on another channel.
- Canonical transition order (QSpec FR-426) SHALL order the search and the
  choice of counterexample, and SHALL leave the set of behaviours unchanged.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-210-AC-1 | A `parallel` of three branches, each one attempt with a true precondition on its own object: the event projections of the behaviours are exactly the six orders of the three attempts. Writing the branches in another source order gives the same set of projections. | Test (TC-655) |
| FR-210-AC-2 | A `parallel` of branch `a`, a `sequence` of attempts `X` then `Y`, and branch `b`, one attempt `Z`, all with true preconditions on distinct objects: the projections are exactly `X Y Z`, `X Z Y` and `Z X Y`; no behaviour takes `Y` before `X`. | Test (TC-655) |
| FR-210-AC-3 | Branch `s` sends `Ping` on a channel and branch `r` receives it, each followed by one attempt: no behaviour takes `receive` before `send`, and every other linear extension of the edge relation is reached. With a second channel whose FIFO order runs opposite, the set of projections is unchanged. | Test (TC-655) |
| FR-210-AC-4 | QSpec FR-052-AC-1's and FR-052-AC-6's split shipments: branches `S1` and `S2`, each a `sequence` of a `pack` attempt then a `ship` attempt on its own shipment, joined `all`, then `finish`. The event projections are exactly the six interleavings of `pack1 ship1` with `pack2 ship2` that keep each branch's order, each followed by `finish`; `finish` follows both `ship` steps in every behaviour. | Test (TC-655) |

## Dependencies

- ADR-027 §2.4 CI-1 to CI-3.
- FR-206 (step kinds), FR-215 (`ProtocolSystem`).
- QSpec owns CI-3 as the meaning of FR-052's "every interleaving that
  preserves each branch's causal edges": QSpec FR-052's causal-interleavings
  rules, tested by QSpec FR-052-AC-6 (ADR-027 QS-5).

## References

- Owning ticket: Linear QSL-396. QSpec half: the causal interleavings of QSpec
  FR-052, tested by QSpec FR-052-AC-6 (Linear STD-140).
