---
id: FR-167
title: "Explore a state-graph subject once and track its open nodes (EN-1 phase 1)"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-020
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-022
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-124
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-165
    type: depends_on
---
# FR-167: Explore a state-graph subject once and track its open nodes (EN-1 phase 1)

## Description

QSL's layer-5 `model_check` SHALL explore a model subject's state graph
once for all of its state-graph items (ADR-022 GE-1, GM-1). It runs at stage
S6c over edge E10, as FR-126 does. The exploration retains every edge, labels
every node with the truth of each item's predicates under each instance's
binding, and marks every node it did not fully expand as **open**, with the
reason (ADR-022 GM-6). FR-168 decides each item over the result.

## Use case

A verification operator requests three state-graph claims and the derived
deadlock-freedom item over one model. The engine explores the model's
reachable states once, with no property automaton, and records which states
it could not expand and why, so a later decision never treats an unexplored
state as a dead end.

## Inputs

- A `ModelCheckRequest` (FR-126, FR-166) whose items are the subject's
  state-graph items (FR-165) and its deadlock-freedom item (FR-124), with
  `ModelCheckLimits` and a cancellation poll.
- Any reduction or state constraint the request selects (ADR-021, as its
  requirements state).

## Outputs

```rust
pub struct ExploredStateGraph {
    pub nodes: Vec<GraphNode>,              // FR-101 canonical discovery order
    pub edges: Vec<GraphEdge>,              // (source, transition identity, post-state)
    pub open: BTreeMap<NodeIndex, OpenCause>,
    pub end: RunEnd,
    pub stats: GraphStats,                  // nodes, edges, depth reached
}

pub struct GraphNode {
    pub key: DigestRecord,                  // quire.simulation.state-key/v1
    pub depth: u64,
    pub labels: Vec<PredicateLabel>,        // per item, instance and predicate
}

pub enum OpenCause {
    MaxDepth,
    ConstraintBoundary,
    UndecidedSuccessor,
    NotExpanded,                            // discovered when a run limit stopped the run
}

pub enum RunEnd {
    Completed,
    Stopped(IncompleteCause, ModelCheckLimit),
}
```

## Behavior

### Pre-check

- Before any expansion, the engine SHALL classify every root of the subject
  as FR-126's pre-check does, and return `ModelCheckRefusal::RequiresBound`
  for an unbounded root with no node explored.
- When the request selects a reduction, the engine SHALL admit it for a
  state-graph item exactly where ADR-022 §6's table reads "yes" for the
  item's form, and SHALL settle every other state-graph item under that
  reduction V-6 `ReductionNotPreserving` before any expansion (ADR-022
  GR-1).

### Exploration

- The engine SHALL explore the subject's `ModelSystem` (FR-120) directly,
  with no property automaton, by FR-101's canonical breadth-first engine,
  and retain every explored edge.
- One exploration SHALL serve every state-graph item of the subject, every
  instance of each, and the subject's deadlock-freedom item, under one
  method.
- The engine SHALL evaluate each predicate of each item, under each
  instance's binding, at each node through the one clause evaluator
  (FR-107) over the node's state observation, and record the result as the
  node's label.
- The deadlock-freedom item SHALL be decided on the same exploration: the
  first deadlocked node (FR-124) in discovery order is its violation, with
  the canonical breadth-first path to it, as FR-126 states.
- A subject with no initial state SHALL end with every item undecided,
  `NoInitialState`.

### Open nodes

- A node at `max_depth` that the engine does not expand SHALL be open,
  `MaxDepth`.
- A boundary state of a selected state constraint (ADR-021 SC-2) SHALL be
  open, `ConstraintBoundary`.
- If a node's expansion evaluates a contract conjunction undecided (FR-120
  `ContractUndetermined`), then the node SHALL be open,
  `UndecidedSuccessor`, and exploration SHALL continue with the other nodes.
- When `max_states`, `max_transitions`, a clause meter or a `true` poll stops
  the run, every discovered node not yet expanded SHALL be open,
  `NotExpanded`, and `end` SHALL be `Stopped` with that limit.
- Every other node SHALL be closed: its retained outgoing edges are exactly
  its FR-120 successors.

### Placement

- The EN-1 provider manifest SHALL advertise the three state-graph forms
  beside its temporal forms, through FR-075's registration.
- The trace monitor (S6a) SHALL advertise no state-graph form.

### Determinism

- The explored graph, its labels and its open set SHALL be functions of the
  subject, the items and the limits.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-167-AC-1 | ADR-022 §7.1's subject explores to 9 nodes and 18 edges, every node closed; §7.2's game to 4 nodes and 3 edges in discovery order `Start`, `Mid`, `Lost`, `Won`; §7.3's job to 5 nodes and 5 edges. A request with `ReachesTwo`, `ReachesThree` and the deadlock-freedom item over §7.1 explores the subject once, and its statistics count 9 nodes. | Test (TC-592) |
| FR-167-AC-2 | §7.1 with `max_depth` 1: `(1, 0)` and `(0, 1)` are open, `MaxDepth`, `(0, 0)` is closed, and `end` is `Completed`. With `max_states` 3 the run ends `Stopped(ResourceExhausted, MaxStates)` and each discovered unexpanded node is open, `NotExpanded`. A subject with an undecided contract conjunction at one reachable node marks that node open, `UndecidedSuccessor`, and still expands every other reachable node. | Test (TC-592) |
| FR-167-AC-3 | §7.2's game with no `terminal` member: the deadlock-freedom item's violation is `Lost` with the path `play, lose`. A subject with no initial state ends `NoInitialState`. An unbounded population root returns `RequiresBound` with no node explored. | Test (TC-592) |
| FR-167-AC-4 | With partial-order reduction selected, an `always possible` item and a `unique path` item settle V-6 `ReductionNotPreserving` before any expansion while a `possible` item over the same subject is explored under it; with symmetry selected, a `unique path` item settles `ReductionNotPreserving`. The EN-1 manifest lists the three state-graph forms and the trace monitor's manifest lists none. | Test (TC-592) |
| FR-167-AC-5 | Running AC-1's §7.2 request twice gives byte-equal explored graphs, labels and open sets. | Test (TC-592) |

## Dependencies

- ADR-022 §2 GM-1, GM-2, GM-6, GM-8; §3 GE-1 and "Placement in
  negotiation"; §6 GR-1 and GR-4; ADR-011 §1 and §6.1 (S6c, E10, layer 5
  `model_check`) as amended by ADR-018.
- [FR-101](FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md),
  [FR-120](FR-120-simulate-a-checked-package-s-state-family.md),
  [FR-124](FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md),
  [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md)
  (request, limits, pre-check),
  [FR-165](FR-165-check-state-graph-claims-at-s3.md),
  [FR-075](FR-075-compute-candidates-from-registered-backends.md)
  (manifest registration).
- The reductions and state constraints themselves are ADR-021's.
- QSpec owns open and closed nodes and the state-graph forms in the
  provider manifest (ADR-022 QS-3, QS-7) and the explored graph as the
  subject of state-graph claims (QS-6).

## References

- ADR-022. QSpec half: Linear STD-135 (ADR-022 QS-3, QS-6, QS-7, QS-9).
