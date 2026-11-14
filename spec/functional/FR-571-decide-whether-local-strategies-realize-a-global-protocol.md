---
id: FR-571
title: "Decide whether local strategies realize a checked global protocol"
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
  - target: ix://agent-ix/quire-spec-language/FR-218
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-175
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-170
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-054
    type: depends_on
---
# FR-571: Decide whether local strategies realize a checked global protocol

## Description

QSL SHALL decide the `realizability` capability kind for a checked global
protocol in the `bounded-static` profile (FR-575) as a game in which the
environment chooses admitted external events and channel outcomes and each
role chooses from its local history only, and return a replayable strategy
witness, a counterexample, or a non-conclusive result. It implements ADR-036
PA-7, PA-8, PA-16 to PA-19 and PA-25 (realizability artifacts) and QSpec
FR-175-AC-1 to AC-5. The key is FR-572's; settlement is FR-573's.

## Inputs

A realizability request: the FR-288 definition key, a strategy profile,
premises and bounds (ADR-036 PA-5), and the claim's assessed-subject root: the observation and
the FR-204 declared object identity the claim is assessed over (FR-574). The
root is a request input, not a key member (FR-572). The premises are the projection premises
(channel members, `choice_visibility`) plus the claim-authored role
knowledge (shared facts) and environment (admitted external events and
channel outcomes). The last two are optional values: absent is not
declared-empty (PA-8).

## Outputs

`RealizabilityWitness { key, strategies, closure }`, or a
`RealizabilityCounterexample`: `IndistinguishableHistories { key, role,
history_a, history_b, action_a, action_b, obligation, separating_witness }` or
`EnvironmentDefeat { key, defeats_per_branch, obligation,
separating_witness }`; or a non-conclusive result. `strategies` is, per
role, a table from observation class to the committed action (in
`bounded-static` an owner's branch is fixed, so the table holds sends,
receives and ends only), over every class reached; `closure` records
plays explored, the horizon and whether exploration closed (PA-19).

## Behavior

- The arena shall be the global protocol's state graph under the one
  `qsl-eval` protocol transition system (QSpec FR-170); the engine shall
  build no transition system of its own and conclude from no sampled or
  replayed execution (PA-16, PA-22).
- A role's observation shall be the records it sent and received, each
  with its control, and the protocol captures of its instance, as bound
  when the instance starts; its own choice outcomes and
  those visible to it are functions of these. Two histories of one
  role are indistinguishable exactly when their observation sequences are
  equal, computed by the single function `observation(role, history)`
  (PA-16, PA-19).
- Every step of the `bounded-static` arena shall be an environment move:
  the order in which threads step, the record a send carries, and each
  delivery alternative. A `choice`'s decision reads only what its owner
  received, the protocol captures and constants (FR-218), and `qsl-eval`
  steps the one case whose guard holds, so the branch is fixed by earlier
  environment moves or the initial state, through the captures: an owner
  node has one branch, no earlier owner choice can avoid a history, and an
  environment defeat carries one defeating play, for that branch. The
  profile activates a protocol `on origin` only, one instance started with
  the initial state; `activation on each` is outside it and answers
  `unsupported` (PA-21, FR-575 `activation-on-each`).
- A role's action at a point where it can step shall be a `send` at its
  control or a receive from a peer, and at a terminal state the end of its
  part; a
  receive commits to the peer only, and the message that arrives, with its
  record, enters the observation. An observation records each record the
  role sent or received with its control, so a choice outcome is visible to
  a role exactly through the records it exchanged and the protocol
  captures.
- V1 defines no string a role-knowledge or environment premise may hold:
  a non-empty set of either shall be refused `invalid_runtime_input`/
  `invalid-value` until the strings these groups hold are specified;
  declared-empty decides and absent is `inconclusive` (PA-8). The
  `bounded-static` profile admits no domain `event`, so the admitted
  external events add no move beyond the channel outcomes the delivery
  premises admit.
- Each send shall have one alternative per delivery count in its channel's
  policy range (`0` loss, greater than `1` duplication); an open range shall
  be cut at the channel's literal capacity and `max_depth`, and a result that
  depended on a cut count shall be non-conclusive (PA-7). A strategy that
  passes the walk under an open range depends on the cut, since a count
  above it could defeat it, and shall be non-conclusive; a refutation found
  among the explored counts stands.
- Environment nodes shall win only if every environment move wins, and owner
  nodes if some branch wins. Per role and observation class the class shall
  commit to an action in the intersection of the winning actions at its
  histories (PA-17). In `bounded-static` an owner node has one branch.
- The engine shall claim a refutation only when forced (PA-18): for
  indistinguishable histories whose winning-action sets are disjoint and
  neither avoidable by an earlier owner choice, or when the environment
  defeats every owner choice. An empty common intersection with pairwise
  non-empty sets, an avoidable history, or a defeat that depends on a cut
  choice shall be non-conclusive. In `bounded-static` no owner makes a
  branch choice, so no history is avoidable by one and no owner choice is
  cut; a disjointness that rests on a cut action is still non-conclusive.
- A role's table is not an owner's branch choice: a role chooses among its
  own enabled actions, and waits while others move when its committed
  action is not enabled (QSpec FR-175: the environment chooses only
  admitted events, channel outcomes and thread order, and cannot make a
  role act). A table can so delay a step or reorder around it. The engine
  shall refute only on forced plays: at every state on each history, or on
  the defeating play, every move is the step taken (one role's one action,
  its records and delivery the environment's, or the environment's steps
  alone), and, for indistinguishable histories, every move at either end is
  a step of the role. Any other refutation is non-conclusive
  (`unforced-refutation`). Every profile is then stuck on a forced play or
  follows it, and at the two ends its one action for the class is not
  enabled at one of them while nothing else can move.
- The engine shall return realizable only after the witness table passes the in-engine
  replay walk of PA-19 over every environment play of the premises,
  recomputing each class; a table that fails it, or whose exploration did not
  close, shall be non-conclusive (PA-19).

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-571-AC-1 | For a realizable protocol the result is a `RealizabilityWitness` whose in-engine replay walk selects each action from the table by recomputed class over every environment play of the premises, finds every action enabled and every play winning; a table edited to fail the walk makes the result non-conclusive, never realizable (implements FR-175-AC-1). | Test (TC-1411) |
| FR-571-AC-2 | For the unrealizable hidden-choice protocol of ADR-036's worked example the result is `IndistinguishableHistories` naming the role, the two histories, the two required actions and the violated obligation; a defeat by the environment of every owner choice is `EnvironmentDefeat` with a defeating play per owner branch of the first lost owner node, one play in `bounded-static` (implements FR-175-AC-2). | Test (TC-1411) |
| FR-571-AC-3 | An absent (`None`) role-knowledge or environment premise returns an `inconclusive` result, while the same premise declared empty (`Some` of an empty set) is decided under; a search that completes its method up to `max_depth` or `max_history`, or an exploration that did not close, returns `inconclusive`, never realizable. A run stopped by a run limit (`max_states`, `max_transitions`, the meter) is not this case and settles `failed` (FR-573-AC-5) (implements FR-175-AC-3). | Test (TC-1411) |
| FR-571-AC-4 | Recomputing `observation(role, history)` for every explored history, equal observation sequences get equal committed actions in the witness, and a protocol whose class has no common winning action returns refuted or `inconclusive`, never a witness (implements FR-175-AC-4). | Test (TC-1411) |
| FR-571-AC-5 | The result quantifies over every admitted environment event and channel outcome: a protocol that is realizable on every sampled run but defeated by one admitted loss or event outcome is not realizable, and an outcome not admitted by the premises never appears in a play; under an open delivery range a result that depended on a cut count is `inconclusive` (implements FR-175-AC-5). | Test (TC-1411) |
| FR-571-AC-6 | In the `bounded-static` profile no arena step is an owner's branch selection: a `choice` decision reads only its owner's received records, protocol captures and constants (FR-218), and `qsl-eval` enters the one case whose guard holds, so no history is avoidable by an owner choice, no owner choice is cut, and an `EnvironmentDefeat` has exactly one entry in `defeats_per_branch`. A role's table may still delay or reorder around a step, so the engine refutes only on forced plays with the role the sole mover at both ends: a protocol in which a role can wait for its committed action, enabled after another role's step in the same class, is not refuted (PA-16, PA-18). | Test (TC-1411) |

## Dependencies

- ADR-036 PA-7, PA-8, PA-16 to PA-19, PA-22, PA-25.
- [FR-519](FR-519-publish-the-protocol-read-surface-the-quire-protocol-link-layer-imports.md): the graph reader.
- [FR-572](FR-572-key-protocol-analysis-results-structurally.md), [FR-573](FR-573-settle-protocol-analysis-results-with-the-fr-360-labels.md), [FR-575](FR-575-answer-unsupported-for-constructs-outside-the-bounded-static-profile.md).
- QSpec FR-175, FR-170, FR-054.

## References

- ADR-036; QSL-1889, QSL-1893.
