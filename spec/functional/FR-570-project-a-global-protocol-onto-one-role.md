---
id: FR-570
title: "Project a checked global protocol onto one role"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-036
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-519
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-572
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-573
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-575
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-174
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-053
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-054
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-172
    type: depends_on
---
# FR-570: Project a checked global protocol onto one role

## Description

QSL SHALL decide the `local-projection` capability kind for a checked global
protocol and one role, in the `bounded-static` profile (FR-575), and return
either the role's local graph with the evidence that it is sound or a minimal
global-path counterexample. It implements ADR-036 PA-9 to PA-12 and PA-25
(projection artifacts) and QSpec FR-174-AC-1 to AC-5. The key of the result
is FR-572's; its settlement is FR-573's.

## Inputs

A projection request: the FR-288 definition key of the checked protocol, a
role, the profile, the premises and the bounds (ADR-036 PA-5, PA-6). The
protocol is read only through the FR-519 read surface, and the premises
(channel members and `choice_visibility`) are populated from it.

## Outputs

`ProjectionWitness { key, local_graph, correspondence, merge_evidence }` or
`ProjectionCounterexample { key, choice, owner, role, path_a, path_b,
action_a, action_b, separating_witness }` (ADR-036 PA-25); or a
non-conclusive result with no graph (FR-573, FR-575). A `LocalGraph` holds
typed local events (send, receive, internal choice, external choice,
parallel region and join), each event's peer and message type, and causal
edges.

## Behavior

- The local graph shall keep exactly the events the role performs or owns,
  erase the rest, and keep the induced order: the role's event `a` precedes
  `b` when a causal path from `a` to `b` exists through any events (PA-9).
- Each global communication shall yield exactly one local send and one local
  receive, and a total correspondence table shall map each global event of
  the role to its local event (PA-9; FR-174-AC-4).
- A `parallel ... join all` shall stay a local partial order, and the engine shall enumerate no
  interleaving. FIFO order per exact channel key and
  evaluated key value shall be kept as the local receive order (PA-9).
- At a `choice` the projection shall treat the role as internal choice when it is the
  owner, as observing the outcome when it lies in `visible` or an earlier
  received message carries the outcome (FR-053-AC-1), as external choice when
  every branch's first events are receives from one peer with distinct
  message types, and otherwise branches shall merge only when their first
  events pair off by direction and peer with equal message types, merged
  pairs recursing on their continuations (PA-10).
- A failure to merge shall return the counterexample of PA-11 and no graph:
  the choice, its owner, the role, two global paths to the first divergent
  action and the two actions, of smallest length (the sum of the two paths'
  event counts), ties broken by the lexicographic order of the choice node
  id, then the node ids of path A, then those of path B.
- Soundness is by construction (PA-12): the correspondence table is total
  and order-preserving, each local event maps to one global event, and a
  branch merges only under the rule above. No composition check and no
  second transition system shall exist (PA-22).

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-570-AC-1 | For a projectable protocol the local graph's events and causal order equal the global event structure restricted to the role's events through the correspondence table, every local event maps to exactly one global event, and the local order is the induced order of any causal path (implements FR-174-AC-1). | Test (TC-1410) |
| FR-570-AC-2 | Every global communication projects to exactly one local send and one local receive, the correspondence table is total over the role's global events, and the send/receive pair is matched by message type and peer (implements FR-174-AC-4). | Test (TC-1410) |
| FR-570-AC-3 | A `parallel` region with `join all` projects to a local partial order with no enumerated interleaving, and two receives on one channel with equal evaluated FIFO key keep their global order as local receive order (PA-9). | Test (TC-1410) |
| FR-570-AC-4 | A choice branch merges for the role only by the PA-10 rule: owner, `visible`, an earlier outcome-carrying message, or distinct first-receive message types, or pairwise equal (direction, peer, message type) first events recursing on their continuations; a send against a receive, an empty first set against a non-empty one, and differing peers or send message types do not merge (implements FR-174-AC-5). | Test (TC-1410) |
| FR-570-AC-5 | A choice the role cannot distinguish returns a counterexample naming the choice, owner, role, the two paths and the two actions, of smallest length with the PA-11 tie-break, and returns no `local_graph` and no substitute graph; for two failing choices the counterexample names the one the tie-break orders first (implements FR-174-AC-2). | Test (TC-1410) |
| FR-570-AC-6 | For `Seller` owning `ship \| cancel` with `visible = {Buyer}` and `ship` sending `pick` to `Warehouse`, projection onto `Warehouse` returns the counterexample (`receive pick` against end); with `cancel` sending `cancelled` to `Warehouse`, or with an earlier `decision` message whose payload the guards test, it returns the witness; a `decision` of one type in both branches without that guard dependence returns the counterexample (ADR-036 worked example). | Test (TC-1410) |
| FR-570-AC-7 | A request the engine cannot decide (FR-575) or whose search reaches `max_depth` returns a non-conclusive result with no graph, and the result carries no substitute for the missing graph (implements FR-174-AC-3). | Test (TC-1410) |

## Dependencies

- ADR-036 PA-3, PA-5, PA-6, PA-9 to PA-12, PA-22, PA-25.
- [FR-519](FR-519-publish-the-protocol-read-surface-the-quire-protocol-link-layer-imports.md): the only graph reader.
- [FR-572](FR-572-key-protocol-analysis-results-structurally.md): the key.
- [FR-573](FR-573-settle-protocol-analysis-results-with-the-fr-360-labels.md): settlement.
- [FR-575](FR-575-answer-unsupported-for-constructs-outside-the-bounded-static-profile.md): the profile and unsupported causes.
- QSpec FR-174, FR-053, FR-054, FR-172.

## References

- ADR-036; QSL-1889, QSL-1893.
