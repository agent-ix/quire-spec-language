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
role, the profile, the premises and the bounds (ADR-036 PA-5, PA-6), and the
claim's assessed-subject root: the observation and the FR-204 declared object
identity the claim is assessed over (FR-574). The root is a request input, not
a key member (FR-572). The protocol is read only through the FR-519 read
surface, and the premises (channel members and `choice_visibility`) are
populated from it.

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
- A `parallel ... join all` shall stay a local partial order, and the
  engine shall enumerate no interleaving (PA-9).
- The engine shall keep FIFO order per exact channel key and evaluated key
  value as the local receive order (PA-9).
- At a `choice` the engine shall classify the role as internal choice when it is
  the owner (PA-10).
- The engine shall classify a non-owner role as observing the outcome only when
  every earlier message whose payload binder the choice's decision facts read
  is a message the role sent or received (FR-053-AC-1). The decision facts are
  the choice's `visible` expressions followed through the value nodes they
  reference; a checked package's `visible` holds expression nodes, not roles.
  A decision that also reads a message the role took no part in does not
  inform the role.
- The engine shall classify a non-owner role that is not told the outcome as
  external choice when, in every branch, the first events include a receive
  from one peer and the message types of those receives differ across
  branches (PA-10).
- Otherwise the engine shall merge branches only when their first events pair
  off by direction and peer with equal message types, merged pairs recursing
  on their continuations (PA-10).
- A failure to merge shall return the counterexample of PA-11 and no graph:
  the choice, its owner, the role, two global paths to the first divergent
  action (path A being the path to an action when the other branch has ended, else
  the one whose node id sequence is canonically first, so the pair is ordered;
  the path to an ended branch is empty) and the two actions, of smallest length (the sum of the two paths'
  event counts), ties broken by the lexicographic order of the choice node
  id, then the node ids of path A, then those of path B.
- A choice event, a send, a receive and the entry and exit of a `parallel`
  are the global events; the entry and exit of a `parallel` are local events
  of the role when at least two of its branches hold events of the role. Two
  receives of one `fifo` channel whose key expression reads nothing of the
  message (every message carries an equal key) follow the order of their
  sends; a key that reads the message adds no order.
- Run limits `max_states` (events) and `max_transitions` (covering edges)
  default to 4096 and 262144; a run one of them stops is `failed`,
  `resource-incomplete`, and states the limit and its value.
- An event deeper than `max_depth` (the longest chain of events ending at it)
  is beyond the horizon. The merge of two branches compares layer by layer and
  stops at the first layer whose first events include an event beyond the
  horizon; a divergence at an earlier layer is a counterexample, and a search
  that finds no counterexample and stops at such a layer, or meets any event
  beyond the horizon, is `inconclusive`.
- When a branch holds a choice, the engine shall treat the branch as one run
  per alternative of each nested choice and shall require every run of one
  branch to merge with every run of the other: each nested alternative is
  compared against the other branch on its own, never as an action concurrent
  with its sibling alternatives. A branch that splits into more than 256 runs
  is reported as a horizon cut: `inconclusive` with the horizon cause, because
  the comparison stops short of the whole branch. Nested runs merge by plain
  V1 merging (equal direction, peer and message type); the engine does not
  distinguish nested runs by the types of their first receives as it does at a
  top-level choice, so a protocol that relies on that is refused with a
  counterexample. Each comparison of two runs counts as a transition against
  `max_transitions`, beyond the covering edges of the structure.
- A merged choice's paired events are recorded in the local graph as
  alternatives (one behaviour of the role in two branches), not as concurrent
  events.
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
| FR-570-AC-4 | A choice branch merges for the role only by the PA-10 rule: owner, an earlier message the role took part in that carries the whole decision, or distinct first-receive message types, or pairwise equal (direction, peer, message type) first events recursing on their continuations; a send against a receive, an empty first set against a non-empty one, and differing peers or send message types do not merge (implements FR-174-AC-5). | Test (TC-1410) |
| FR-570-AC-5 | A choice the role cannot distinguish returns a counterexample naming the choice, owner, role, the two paths and the two actions, of smallest length with the PA-11 tie-break, and returns no `local_graph` and no substitute graph; for two failing choices the counterexample names the one the tie-break orders first (implements FR-174-AC-2). | Test (TC-1410) |
| FR-570-AC-6 | For `Seller` owning `ship \| cancel` with `ship` sending `pick` to `Warehouse`, projection onto `Warehouse` returns the counterexample (`receive pick` against end); with `cancel` sending `cancelled` to `Warehouse`, or with an earlier message that `Warehouse` sends to `Seller` and `Seller` receives, whose payload binder the choice's `visible` fact reads (and no other message is read), it returns the witness; when the decision also reads a message `Warehouse` took no part in it returns the counterexample; an earlier `decision` message of one type in both branches that no decision fact reads returns the counterexample (ADR-036 worked example). | Test (TC-1410) |
| FR-570-AC-7 | A request the engine cannot decide (FR-575) or whose search reaches `max_depth` returns a non-conclusive result with no graph, and the result carries no substitute for the missing graph (implements FR-174-AC-3). | Test (TC-1410) |
| FR-570-AC-8 | A choice nested in a branch contributes one run per alternative, and each run merges with the other branch on its own: with `cancel` sending `pick3` to `Warehouse` and `ship` holding a nested choice of `pick` or `pick2` to `Warehouse`, projection onto `Warehouse` returns the witness; with `cancel` empty and the same `ship`, it returns the counterexample (implements FR-174-AC-5). | Test (TC-1410) |

## Dependencies

- ADR-036 PA-3, PA-5, PA-6, PA-9 to PA-12, PA-22, PA-25.
- [FR-519](FR-519-publish-the-protocol-read-surface-the-quire-protocol-link-layer-imports.md): the only graph reader.
- [FR-572](FR-572-key-protocol-analysis-results-structurally.md): the key.
- [FR-573](FR-573-settle-protocol-analysis-results-with-the-fr-360-labels.md): settlement.
- [FR-575](FR-575-answer-unsupported-for-constructs-outside-the-bounded-static-profile.md): the profile and unsupported causes.
- QSpec FR-174, FR-053, FR-054, FR-172.

## References

- ADR-036; QSL-1889, QSL-1893.
