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

When FR-167's exploration ends, `qsl-analyze`'s `model_check` SHALL decide each
state-graph item instance by one pass over the explored graph, linear in its
nodes and edges (ADR-022 §3): backward reachability for `possible` and
`always possible`, and path counting for `unique path`. It SHALL return
evidence only where open nodes cannot change it (ADR-022 GM-6): a trap whose
forward closure is closed, a pair of paths through known nodes, or an
undefined evaluation. A witness for every initial state decides `possible`
only when the exploration reached every reachable node and found no
undefined evaluation of the target (ADR-022 GV-1, RU-5). Evidence is canonical, so the same request gives the same evidence.

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
  or `Explored`, on a run that completed with no open node;
- `WitnessedUnchecked { paths, end, open }`: the same witnesses on a run
  that a limit stopped or that ended with open nodes, so a reachable
  undefined evaluation of the target is not ruled out, with the run's end
  and its open causes;
- `Trapped { stem }`: a `ModelPath` ending at a trap node;
- `PathPair { stem, first, second }`: a stem ending at a node where `from`
  holds and two distinct step sequences from it, each ending at its first
  `to` node;
- `Holds { basis }`: `Exhaustive`, or `Reduced{reductions}` under an
  admitted reduction;
- `NoDecision { end, open }`: the run's end and the causes of its open
  nodes, which FR-169 maps;
- `Undefined { stem, undefined }`: a `ModelPath` ending at the first node, in
  discovery order, where a predicate of the claim evaluates undefined, and
  its `UndefinedEvaluation{where, cause}`;
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
- A node where the target evaluates `Undefined` SHALL NOT be a target node;
  it is refuting evidence ("Undefined evaluation" below).
- For `Possible`, the engine SHALL also compute, by a breadth-first search
  backwards from the target nodes alone, each node's distance `d_P` to the
  nearest target node.
- For `Possible`: an initial state with no sampled witness and a path of
  closed or target nodes to a target node SHALL get the **explored witness**:
  from the initial state, at each node, the first edge in canonical order
  whose target has `d_P` one less, so the witness ends at a target node
  whether or not the run has open nodes.
- When every initial state has a sampled or explored witness, `Possible`
  SHALL return `Witnessed` if the run completed with no open node, and
  `WitnessedUnchecked` otherwise. Otherwise, if an initial state is a trap,
  it SHALL return `Trapped` with that initial state and an empty stem.
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

### Undefined evaluation

- The engine SHALL evaluate the claim's predicates through the one clause
  evaluator (FR-107) at every explored node.
- If a predicate evaluates `Undefined` at an explored node, then the engine
  SHALL return `Undefined` with the canonical breadth-first path to the
  first such node in discovery order, with `where` and `cause` as QSpec
  FR-391 states them: the predicate, the node's state-key digest and the
  locus of the expression with no value, and the catalogued reason
  (ADR-022 GV-7, ADR-018 UE-1).
- `Undefined` SHALL take the place of `Witnessed`, `WitnessedUnchecked`
  and `Holds`, whether the witnesses were sampled or explored. When a trap
  or a path pair is found at a node earlier in discovery order, the engine
  SHALL return that evidence instead. A node that is undefined and also a
  trap or a count-2 node SHALL return `Undefined` (QSpec FR-391).

### Partial runs

- A trap, a path pair and an undefined evaluation SHALL be returned whether
  `end` is `Completed` or `Stopped`, and whatever other nodes are open.
- `Witnessed` and `Holds` SHALL be returned only when the run completed with
  no open node. Witnesses for every initial state on any other run SHALL be
  returned as `WitnessedUnchecked`.
- Every instance with none of the above SHALL return `NoDecision`, carrying
  the run's end and the set of open causes.

### Determinism

- The outcome and its evidence SHALL be functions of the subject, the
  item, the limits and the seed. Phase 0 changes which source a witness
  has, never which outcome kind an instance reaches on a run that completes
  with no open node.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-168-AC-1 | ADR-022 §7.1 with `witness_samples` 0: `ReachesTwo` for `c = a` returns `Witnessed` with the explored path `(0, 0) -upd(a)-> (1, 0) -upd(a)-> (2, 0)`, and for `c = b` the same with `upd(b)`; `ReachesThree` returns `Trapped` with initial state `(0, 0)` and an empty stem for each instance. | Test (TC-593) |
| FR-168-AC-2 | §7.2: `CanStillWin` returns `Trapped` at `Lost` with stem `play, lose`; with `from (x.phase != Lost)` it returns `Holds{Exhaustive}`; with the `restart` operation and no `from` it returns `Holds{Exhaustive}`. | Test (TC-593) |
| FR-168-AC-3 | §7.3: `InOneWay` returns `PathPair` with an empty stem, first `stepA, stepB, finish` and second `stepB, stepA, finish`; the sequenced variant returns `Holds{Exhaustive}`. The sequenced variant with `reset` (precondition `a and not b`, postcondition `not a`, frame `[a]`) returns `PathPair` with first `stepA, stepB, finish` and second `stepA, reset, stepA, stepB, finish`. | Test (TC-593) |
| FR-168-AC-4 | The §7.2 game with field `n: Int[0, 50]`, initially 0, and operation `celebrate` (precondition `phase = Won and n < 50`, postcondition `n = n + 1`, frame `[n]`), under the search horizon `max_depth` 3: `CanStillWin` returns `Trapped` at `Lost` although the node `Won` with `n = 1` is open; `possible x.phase = Won and x.n = 50` returns `NoDecision` with `end` `Completed` and open cause `MaxDepth`. | Test (TC-593) |
| FR-168-AC-5 | Over §7.1's subject, `possible 2 / (2 - c.versionNumber) = 2` for `c = a` returns `Undefined` with the canonical path `(0, 0) -upd(a)-> (1, 0) -upd(a)-> (2, 0)`, `where` `(2, 0)` and cause `division-by-zero`, although `(1, 0)` satisfies the target. | Test (TC-612) |
| FR-168-AC-6 | `ReachesTwo` with `witness_samples` 64 and with 0 both return `Witnessed`, with sources `Sampled` and `Explored` respectively. Running AC-2's three requests twice gives byte-equal outcomes. | Test (TC-593) |
| FR-168-AC-7 | FR-168-AC-5's item with default `witness_samples` holds a sampled witness from `(0, 0)` ending at `(1, 0)` and still returns `Undefined` at `(2, 0)`, not `Witnessed`; `ReachesTwo` with default limits returns `Witnessed` with `Sampled` sources from a run that completed with no open node. | Test (TC-613) |
| FR-168-AC-8 | `ReachesTwo` with default `witness_samples` and `max_states` 2 returns `WitnessedUnchecked` with `Sampled` sources and `end` `Stopped(ResourceExhausted, MaxStates)`; with `witness_samples` 0 and `max_states` 2 it returns `NoDecision` with the same `end`. | Test (TC-614) |
| FR-168-AC-9 | Fixture `Fork`: one object with `pos: Int[0, 4]`, initially 0; operations `toA()` (pre `pos = 0`, post `pos = 1`), `toB()` (pre `pos = 0`, post `pos = 2`), `fromA()` (pre `pos = 1`, post `pos = 3`), `fromB()` (pre `pos = 2`, post `pos = 4`) and `back()` (pre `pos = 3`, post `pos = 0`), each framed `modifies self.pos`, with `toA` before `toB` in canonical transition order. `possible x.pos = 4` with `witness_samples` 0 under horizon 2: the node with `pos = 3` is open, `pos = 1` and `pos = 2` both have `d` 1, and the explored witness is `toB, fromB`, ending at the target `pos = 4`, not `toA, fromA`; the outcome is `WitnessedUnchecked`. | Test (TC-619) |

## Dependencies

- ADR-022 §2 GM-4 to GM-6; §3 "Phase 1 algorithms", "Determinism" and
  "Canonical evidence"; §4 GV-1 to GV-5 and GV-7 (which outcomes are decisive); §10 RU-5; §7
  worked examples.
- [FR-167](FR-167-explore-a-state-graph-subject-and-track-open-nodes.md)
  (the explored graph and open nodes),
  [FR-166](FR-166-sample-witnesses-for-possible-claims.md) (sampled
  witnesses), [FR-165](FR-165-check-state-graph-claims-at-s3.md) (forms),
  [FR-101](FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md)
  (canonical discovery order).
- FR-169 settles the outcome; FR-170 replays its evidence.
- QSpec FR-390 and FR-391 own the graph semantics and which evidence is
  decisive on a partial run (ADR-022 QS-2, QS-3).

## References

- ADR-022. QSpec half: QSpec FR-390 and FR-391 (Linear STD-135; ADR-022
  QS-2, QS-3).
