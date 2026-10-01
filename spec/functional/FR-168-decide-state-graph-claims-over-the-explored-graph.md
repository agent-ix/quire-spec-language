---
id: FR-168
title: "Decide state-graph claims over the explored graph"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-020
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-022
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-165
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-166
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-167
    type: depends_on
---
# FR-168: Decide state-graph claims over the explored graph

## Description

When FR-167's exploration ends, QSL's layer-5 `model_check` SHALL decide each
state-graph item instance by one pass over the explored graph, linear in its
nodes and edges (ADR-022 §3): backward reachability for `possible` and
`always possible`, and path counting for `unique path`. It SHALL return
evidence only where open nodes cannot change it (ADR-022 GM-6): a witness, a
trap whose forward closure is closed, or a pair of paths through known
nodes. Evidence is canonical, so the same request gives the same evidence.

## Use case

A verification operator asks whether a game can always still be won. The
engine finds the state `Lost`, from which no path reaches `Won`, and returns
it with the shortest path to it. When the exploration stopped at a depth
limit elsewhere in the graph, the trap still counts, because nothing past
the limit can change what `Lost` reaches. A claim with no such evidence is
left undecided, never guessed.

## Inputs

- FR-167's `ExploredStateGraph` for the subject.
- FR-166's sampled witnesses for each `Possible` instance.
- Each state-graph item (FR-165) and its instances.

## Outputs

`StateGraphOutcome`, per item instance, one of:

- `Witnessed { paths }`: one `ModelPath` per initial state, each ending at a
  node where the target holds, each with its source, `Sampled(…)` (FR-166)
  or `Explored`;
- `Trapped { stem }`: a `ModelPath` ending at a trap node;
- `PathPair { stem, first, second }`: a stem ending at a node where `from`
  holds and two distinct step sequences from it, each ending at its first
  `to` node;
- `Holds { basis }`: `Exhaustive`, or `Reduced{reductions}` under an
  admitted reduction;
- `NoDecision { end, open }`: the run's end and the causes of its open
  nodes, which FR-169 maps;
- `NoInitialState`.

Each carries its instance's `over` binding.

## Behavior

### Canonical order

- Edges out of a node SHALL be ordered by their transition identity's
  canonical bytes (QSpec FR-181), then by the post-state's state-key bytes.

### `possible` and `always possible`: backward reachability

- The engine SHALL compute `E⁺`, the nodes that reach a node where the
  target holds or an open node, by a breadth-first search backwards over the
  retained edges from every such node, recording each node's distance `d` to
  the nearest one.
- A closed node outside `E⁺` SHALL be a **trap**.
- For `Possible`: an initial state with no sampled witness and a path of
  closed or target nodes to a target node SHALL get the **explored witness**:
  from the initial state, at each node, the first edge in canonical order
  whose target is a node at distance one less, ending at a target node.
- `Possible` SHALL return `Witnessed` when every initial state has a sampled
  or explored witness. Otherwise, if an initial state is a trap, it SHALL
  return `Trapped` with that initial state and an empty stem.
- `AlwaysPossible` SHALL return `Trapped` when a trap where `from` holds is
  reachable: the first such trap in FR-101 canonical breadth-first discovery
  order, with the canonical breadth-first path to it as its stem.
- `AlwaysPossible` SHALL return `Holds` when no node is open and every
  reachable node where `from` holds is in `E⁺`.

### `unique path`: path counting

- The engine SHALL build `H`, the retained graph restricted to the nodes
  that reach a `to` node or an open node, with every edge out of a `to` node
  removed.
- It SHALL decompose `H` into strongly connected components and count paths
  to `to` per node in reverse topological order of the condensation,
  saturating at 2: a `to` node counts 1; a node in a non-trivial component
  of `H`, or with a self-loop, counts 2; any other node counts the saturated
  sum over its edges in `H`. A node with no path to a `to` node or an open
  node counts 0.
- A node SHALL be **known** when no open node is reachable from it in `H`.
- `UniquePath` SHALL return `PathPair` for the first known node, in
  discovery order, where `from` holds and that counts 2. The pair SHALL be
  the shortest path from it to `to`, ties broken by canonical edge order,
  and the least, by length and then canonical edge order, of the paths that
  first deviate from it: over every position of the first path and every
  other edge out of that position's node into `H`, the prefix, that edge,
  then the shortest path on. The stem is the canonical breadth-first path to
  the node.
- Otherwise `UniquePath` SHALL return `Trapped` for the first closed node in
  discovery order where `from` holds and that counts 0, with its canonical
  breadth-first stem.
- `UniquePath` SHALL return `Holds` when no node is open and every reachable
  node where `from` holds counts exactly 1.

### Partial runs

- A witness, a trap and a path pair SHALL be returned whether `end` is
  `Completed` or `Stopped`, and whatever other nodes are open.
- `Holds` SHALL be returned only when the run completed with no open node.
- Every instance with none of the above SHALL return `NoDecision`, carrying
  the run's end and the set of open causes.

### Determinism

- The outcome and its evidence SHALL be functions of the subject, the
  item, the limits and the seed. Phase 0 changes which source a witness
  has, never which outcome kind an instance reaches on a run that phase 1
  completes.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-168-AC-1 | ADR-022 §7.1 with `witness_samples` 0: `ReachesTwo` for `c = a` returns `Witnessed` with the explored path `(0, 0) -upd(a)-> (1, 0) -upd(a)-> (2, 0)`, and for `c = b` the same with `upd(b)`; `ReachesThree` returns `Trapped` with initial state `(0, 0)` and an empty stem for each instance. | Test (TC-593) |
| FR-168-AC-2 | §7.2: `CanStillWin` returns `Trapped` at `Lost` with stem `play, lose`; with `from (x.phase != Lost)` it returns `Holds{Exhaustive}`; with the `restart` operation and no `from` it returns `Holds{Exhaustive}`. | Test (TC-593) |
| FR-168-AC-3 | §7.3: `InOneWay` returns `PathPair` with an empty stem, first `stepA, stepB, finish` and second `stepB, stepA, finish`; the sequenced variant returns `Holds{Exhaustive}`. The sequenced variant with `reset` (precondition `a and not b`, postcondition `not a`, frame `[a]`) returns `PathPair` with first `stepA, stepB, finish` and second `stepA, reset, stepA, stepB, finish`. | Test (TC-593) |
| FR-168-AC-4 | The §7.2 game with field `n: Int[0, 50]` and operation `celebrate` (precondition `phase = Won and n < 50`, postcondition `n = n + 1`, frame `[n]`), under `max_depth` 3: `CanStillWin` returns `Trapped` at `Lost` although the node `Won` with `n = 1` is open; `possible x.phase = Won and x.n = 50` returns `NoDecision` with `end` `Completed` and open cause `MaxDepth`. | Test (TC-593) |
| FR-168-AC-5 | `ReachesTwo` with `witness_samples` 64 and with 0 both return `Witnessed`, with sources `Sampled` and `Explored` respectively. Running AC-2's three requests twice gives byte-equal outcomes. | Test (TC-593) |

## Dependencies

- ADR-022 §2 GM-4 to GM-6; §3 "Phase 1 algorithms", "Determinism" and
  "Canonical evidence"; §4 GV-1 to GV-5 (which outcomes are decisive); §7
  worked examples.
- [FR-167](FR-167-explore-a-state-graph-subject-and-track-open-nodes.md)
  (the explored graph and open nodes),
  [FR-166](FR-166-sample-witnesses-for-possible-claims.md) (sampled
  witnesses), [FR-165](FR-165-check-state-graph-claims-at-s3.md) (forms),
  [FR-101](FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md)
  (canonical discovery order).
- FR-169 settles the outcome; FR-170 replays its evidence.
- QSpec owns the graph semantics and which evidence is decisive on a partial
  run (ADR-022 QS-2, QS-3).

## References

- ADR-022. QSpec half: Linear STD-135 (ADR-022 QS-2, QS-3).
