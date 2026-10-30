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

`qsl-analyze`'s `model_check` SHALL explore a model subject's state graph
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
  `ModelCheckLimits` and the request's `Cancel` handle (FR-276).
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
    pub key: StateKey,                      // quire.simulation.state-key/v1: canonical RFC 8785 bytes of the state
    pub depth: u64,
    pub labels: Vec<PredicateLabel>,        // per item, instance and predicate
}

pub struct GraphEdge {
    pub source: NodeIndex,
    pub transition: TransitionIdentity,
    pub target: NodeIndex,
    pub permutation: Permutation, // identity when symmetry is not applied
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
  and retain every explored edge. With no reduction, a closed node's outgoing
  edges SHALL be exactly its FR-120 successors. With a selected reduction
  admitted for the state-graph form by ADR-022 GR-1, the engine SHALL retain the quotient or ample-set
  edges specified by ADR-021; a node is closed when that reduced expansion is
  complete.
- When symmetry is selected and admitted for the item's form, the engine
  SHALL canonicalise each initial and successor state with the declaration's
  group or the instance binding's stabiliser (ADR-021 SYM-6, SYM-7), coalesce
  equal canonical states, and retain the permutation that maps each concrete
  successor to its canonical target on the edge. For an item whose `over`
  bindings form one or more symmetry orbits, the engine SHALL decide one
  instance per orbit, bound to its least key, and use that binding's
  stabiliser. Because the graph is shared, it SHALL use the intersection of
  the stabilisers of all retained instances; this group fixes every binding
  whose predicate labels are stored in the graph. Predicate labels SHALL be
  evaluated for each retained instance under its binding, using the symmetric
  predicates admitted by ADR-021 SYM-3. The graph SHALL retain enough edge
  information for FR-168 witnesses and FR-170 replay to recover concrete
  transitions.
- When partial-order reduction is selected and admitted for the item's form,
  the engine SHALL expand the ample set and apply the breadth-first revisit
  proviso as ADR-021 POR-6 to POR-8 specify.
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
  An undefined guard, a precondition clause whose `evaluation` is
  `Undefined`, leaves its node open `UndecidedSuccessor` like any undecided
  expansion, and is also refuting evidence of every state-graph item over
  the subject (FR-168 "Undefined evaluation").
- When `max_states`, `max_transitions`, a clause meter or a cancelled
  `Cancel` handle stops the run, every discovered node not yet expanded SHALL be open,
  `NotExpanded`, and `end` SHALL be `Stopped` with that limit.
- Every other node SHALL be closed: in an unreduced run its retained outgoing
  edges are exactly its FR-120 successors; in a reduced run its selected
  quotient or ample-set expansion is complete.

### Placement

- The EN-1 provider manifest SHALL advertise the three state-graph forms
  beside its temporal forms, through FR-075's registration.
- The trace monitor (S6a) SHALL advertise no state-graph form.

### Determinism

- The explored graph, its labels and its open set SHALL be functions of the
  subject, the items, the limits and the selected reductions.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-167-AC-1 | ADR-022 §7.1's subject explores to 9 nodes and 18 edges, every node closed; §7.2's game to 4 nodes and 3 edges in discovery order `Start`, `Mid`, `Lost`, `Won`; §7.3's job to 5 nodes and 5 edges. A request with `ReachesTwo`, `ReachesThree` and the deadlock-freedom item over §7.1 explores the subject once, and its statistics count 9 nodes. | Test (TC-592) |
| FR-167-AC-2 | §7.1 with `max_depth` 1: `(1, 0)` and `(0, 1)` are open, `MaxDepth`, `(0, 0)` is closed, and `end` is `Completed`. With `max_states` 3 the run ends `Stopped(ResourceExhausted, MaxStates)` and each discovered unexpanded node is open, `NotExpanded`. A subject with an undecided contract conjunction at one reachable node marks that node open, `UndecidedSuccessor`, and still expands every other reachable node. | Test (TC-592) |
| FR-167-AC-3 | §7.2's game with no `terminal` declaration: the deadlock-freedom item's violation is `Lost` with the path `play, lose`. A subject with no initial state ends `NoInitialState`. An unbounded population root returns `RequiresBound` with no node explored. | Test (TC-592) |
| FR-167-AC-4 | With partial-order reduction selected, an `always possible` item and a `unique path` item settle V-6 `ReductionNotPreserving` before any expansion while a `possible` item over the same subject is explored under it; with symmetry selected, a `unique path` item settles `ReductionNotPreserving`. The EN-1 manifest's `state_graph_forms` lists the three exact FR-390 forms, the trace monitor's manifest has no such member, and candidate computation filters a state-graph item by its exact form. | Test (TC-592, TC-193) |
| FR-167-AC-5 | Running AC-1's §7.2 request twice gives byte-equal explored graphs, labels and open sets. | Test (TC-592) |
| FR-167-AC-6 | Over ADR-021 §7.1 with the admitted `config_history` symmetry declaration and `ReachesTwo` instance `c = a`, whose binding stabilizer is `[[a], [b, c]]` (SYM-6), EN-1 stores the 18 states of that quotient rather than 27; every retained edge targets a canonical state and carries the permutation mapping its concrete successor to that target, and predicate labels remain keyed to `c = a`. The same request without symmetry stores 27 states. | Test (TC-592) |

## Dependencies

- ADR-022 §2 GM-1, GM-2, GM-6, GM-8; §3 GE-1 and "Placement in
  negotiation"; §6 GR-1 and GR-4; ADR-011 §1 and §6.1 (S6c, E10, layer A
  `model_check`) as amended by ADR-018.
- [FR-101](FR-101-explore-finite-models-with-canonical-order-and-the-qspec-sampler.md),
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

- ADR-022. QSpec half: QSpec FR-390, FR-391 and FR-392 (Linear STD-135;
  ADR-022 QS-3, QS-6, QS-7, QS-9).
