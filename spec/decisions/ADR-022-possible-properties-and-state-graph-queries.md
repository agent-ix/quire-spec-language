---
id: ADR-022
title: "Possible properties and state-graph queries over a model"
type: ADR
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-016
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: relates_to
  - target: ix://agent-ix/quire-specification/FR-161
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-181
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-243
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-290
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-331
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-341
    type: depends_on
---
# ADR-022: Possible properties and state-graph queries over a model

## Status

Proposed, 2026-10-01. Draft for plan-lead review of the key decisions; the
QSL compiler requirements that implement it follow review. It builds on
ADR-018, itself a draft, and follows the owner's rulings recorded there
(ADR-018 RU-1 to RU-4): the explicit-state engine comes first; the unmarked
fairness granularity is `whole`; reachable deadlocks are reported by a
derived deadlock-freedom item; and the infinite-trace profile admits interval
operators nested under unbounded ones. It reads the strong-fairness record
(ADR-019), the refinement record (ADR-020) and the state-space reduction
record (ADR-021), all drafts, for their interactions (§6). §10 records the
owner's rulings on the draft's four questions. The owning ticket and related
work are listed under References.

"QSpec FR-nnn" names a quire-specification requirement; a bare FR id is a QSL
requirement. "QSpec FR-341 (infinite-trace)" is the infinite-trace result
disposition vocabulary, as in ADR-018. Item ids `SG-`, `GM-`, `GE-`, `GV-`,
`GX-`, `GR-`, `DS-`, `QS-` and `RU-` are local to this record. Other artifacts cite
them as `ADR-022 GM-3`. Items of ADR-018 are cited as `ADR-018 SM-3`.

## Context

ADR-018 gives a temporal claim over a model subject a verdict over every
behaviour. Its semantics is the trace evaluator (ADR-018 SM-1): a formula is
true or false on one behaviour, and the model satisfies it when every
admitted behaviour does (SM-5). That is linear time.

Three kinds of statement an author wants to make about a model are not
statements about each behaviour:

- **P is possible.** Some behaviour from the start reaches a state where `P`
  holds. Example: the game can be won. In CTL this is `EF P`.
- **P is always still possible.** From every reachable state, some
  continuation reaches `P`. Example: the replicas can always still converge.
  In CTL this is `AG EF P`.
- **Questions about the whole state graph.** Example: from every start state
  there is exactly one way to finish.

Each needs a quantifier over paths that branch from a state, which SM-1 has
no place for: the truth of `EF P` at a state depends on the paths leaving it,
and one trace holds one of them. TLA+ states none of these natively; TLC
checks them only through tricks such as negating an invariant to obtain a
trace.

What QSL already has that bears on them:

- **The explored graph.** QSpec FR-181's exhaustive mode explores a model's
  successor relation breadth-first in canonical order, coalescing equal state
  keys. FR-101 implements it; FR-120's `ModelSystem` supplies the successor
  relation of a checked state model. ADR-018 EN-1 runs on it and retains every
  explored edge (ADR-018 §3), so the whole reachable state graph is in memory
  after its first phase. EN-1 runs under `ModelCheckLimits`, whose members
  are ADR-014 B-5 budgets of QSL's own provider that the caller sets (ADR-018
  IV-6).
- **Sampling.** FR-101 `sample` draws seeded random walks from the same
  successor relation, and a sampled trace replays through `ModelSystem`. A
  sampled trace is identified by `SampleProvenance{seed, trace, sampler}`
  (ADR-014 TR-1).
- **Replay.** A model trace replays through FR-101 `replay` with post-state
  digest checks (ADR-018 CX-3).
- **Object-graph reachability.** QSpec V1-TYPE-024's `reaches` is traversal
  of typed relationships inside one state. It reads one state's object graph,
  never the model's state graph, and the names this record adds are distinct
  from it.

## Decision

### 1. State-graph claims: a property family beside temporal claims

| ID | Rule |
| --- | --- |
| SG-1 | **`possible`.** `possible Name using v over (x: T) { P }` states that from every initial state of the subject some finite path reaches a state where `P` holds. `P` is a state predicate (GM-3). This is CTL `EF P` at every initial state. |
| SG-2 | **`always possible`.** `always possible Name using v over (x: T) from (Q) { P }` states that from every reachable state where `Q` holds some finite path reaches a state where `P` holds. `from (Q)` is optional; without it `Q` is `true`. This is CTL `AG (Q implies EF P)`. |
| SG-3 | **`unique path`.** `unique path Name using v over (x: T) from (X) to (Y)` states that from every reachable state where `X` holds there is exactly one path to `Y` (GM-5). It is the first state-graph query: a property of the graph's path structure with no CTL counterpart. |
| SG-4 | **A family of its own.** SG-1 to SG-3 are **state-graph claims**, a property family separate from temporal claims. A state-graph claim has no temporal profile, no clock, no activation and no temporal operator in its predicates. Its form (SG-1, SG-2 or SG-3) is computed by S3 and recorded beside its requirement record, as ADR-018 §1 records a temporal form. S3 records it as `Requirements{kind: temporal-satisfaction, extent: Unbounded{domains}}`, with `domains` naming the claim, so it routes on the existing kind (ADR-014 §6), and each candidate's arm reads the form. |
| SG-5 | **Spelling.** The keywords `possible`, `always possible`, `unique path`, `from` and `to` are illustrative; QSpec's shared grammar owns them (QS-1). The `over` binding is ADR-018 QS-5's: one claim instance per object of the universe. |

`P`, `Q`, `X` and `Y` are Boolean state clauses under the unit's value
profile, as an invariant body is. A claim with no `over` parameter has one
instance.

**Why a family and not CTL operators in the temporal grammar.** Every
temporal formula has a truth value on one trace, which replay, monitors and
every engine share (ADR-018 SM-1, SM-7). A path quantifier inside a temporal
formula would end that: the formula would have no value on a trace, and a
mixed formula is CTL*, which needs its own evaluator and costs more to check.
A separate family keeps SM-1 exact for every temporal claim and gives each
state-graph form a fixed graph algorithm that is linear in the graph's size
(§3). A further form joins the family as a new SG row with its own algorithm
and evidence shape.

### 2. Semantics over the explored state graph

| ID | Rule |
| --- | --- |
| GM-1 | **The state graph.** The state graph of a model subject (ADR-018 SM-2) has a node for each state reachable from an initial state and an edge `s -t-> s'` for each FR-120 successor: transition identity `t`, post-state `s'`. It has no terminal stutter step (ADR-018 SM-4): a terminal state is a node with no outgoing edge. Universes and `ProofBound`s define the subject and so the graph; a verdict holds for exactly its subject (ADR-018 §1 "Scope"). |
| GM-2 | **Every reachable state.** "Reachable" means reachable from an initial state of the subject by edges of GM-1. A state outside a universe, or outside a `ProofBound`, is outside the subject. A state constraint (ADR-021 SC-1) and the run limits are method parameters: they decide which states EN-1 expands, never which states are reachable (GM-6). |
| GM-3 | **State predicates.** A predicate of a state-graph claim is evaluated at a node through the one clause evaluator (ADR-016 FE-3) over the node's synthesized state observation (ADR-016 ID-10) with the `over` parameter bound. It reads the state alone. S3 refuses a predicate that reads an operation anchor or a pre-state, `unsupported_construct`/`expression-form`, located at the read, so a predicate's truth at a node does not depend on the edge that reached it. Where the bound object does not exist at a node, the predicate reads as ADR-018 QS-5 states for temporal atoms. |
| GM-4 | **Truth of SG-1 and SG-2.** `E(P)` is the set of nodes from which some path of zero or more edges reaches a node where `P` holds: the least fixpoint `Z = P ∪ pre(Z)`, with `pre(Z)` the nodes that have an edge into `Z`. SG-1 holds when every initial state is in `E(P)`. SG-2 holds when every reachable node where `Q` holds is in `E(P)`. A node where `P` holds is in `E(P)` by the empty path, so a terminal state satisfies SG-2 at itself exactly when `P` or `not Q` holds there. |
| GM-5 | **Paths and SG-3.** A path from a node `x` to `Y` is a finite edge sequence from `x` whose last node satisfies `Y` and no earlier node does. When `x` satisfies `Y` the empty path is its only path. Two paths are distinct when their edge sequences differ, an edge being (source, transition identity, post-state). SG-3 holds when every reachable node where `X` holds has exactly one path to `Y`. A cycle on a path from `x` to `Y` gives `x` unboundedly many paths. |
| GM-6 | **Open nodes.** A node EN-1 has not fully expanded is **open**: a node at `max_depth` (ADR-018 §3), a boundary state of a state constraint (ADR-021 SC-2), a node whose expansion evaluated a contract conjunction undecided (ADR-018 V-6 `UndecidedSuccessor`), and every discovered node not expanded when a run limit stopped the run. Its outgoing edges are not known. Every other node is **closed**: its outgoing edges are exactly its FR-120 successors. Decisive evidence (GV-2) reads only facts that open nodes cannot change. |
| GM-7 | **Fairness plays no part.** S3 refuses a fairness constraint on a state-graph claim, `unsupported_construct`/`expression-form`. Weak and strong fairness of operations are machine-closed (ADR-018 §4, ADR-019 SF-7): every finite path from an initial state extends to a fair behaviour. So every reachable node lies on a fair behaviour, and every finite path to `P` is a prefix of one. Fairness therefore changes no state-graph verdict, and admitting it would only add a premise with no effect. |
| GM-8 | **Terminal states and deadlocks.** A terminal state, intended or not (ADR-018 DL-1, DL-2), is a node with no outgoing edge and reads by GM-4 and GM-5 like any node. A reachable terminal state where `Q` holds and `P` does not refutes SG-2. The model's `terminal` member changes no state-graph verdict, as it changes no temporal verdict (ADR-018 DL-6). The derived deadlock-freedom item (ADR-018 DL-3) is added for each distinct model subject among a request's temporal and state-graph items, so a request whose only claims are state-graph claims still reports its subject's deadlocks. |

### 3. Engines and algorithms

| ID | Engine | Forms and verdicts |
| --- | --- | --- |
| GE-1 | **EN-1, explicit-state** (ADR-018 §3), in layer 5 `model_check`. A state-graph claim needs no property automaton: EN-1 explores the subject's `TransitionSystem` directly, retains every edge, and labels each node with the truth of the claim's predicates under each instance's binding. One exploration serves every state-graph item, every instance and the deadlock-freedom item of one subject under one method. | SG-1, SG-2, SG-3. GV-1, GV-2, GV-4 to GV-6 |
| GE-2 | **EN-1 phase 0, witness sampling, on by default.** Before exploration, for an SG-1 item, EN-1 draws `witness_samples` random walks per initial state with FR-101 `sample`, each at most `max_depth` steps. Walk `r` from initial state `i` is the FR-101 sampled trace with the request's seed and trace index `i × witness_samples + r`, so each walk has its own `SampleProvenance`. A walk that reaches a node where `P` holds is a witness for its initial state, and it stops there. When every initial state has a witness, the item settles from them (GV-1) after replay, and phase 1 does not run for it. Otherwise phase 1 runs, keeps the sampled witnesses, and finds the rest. Zero walks reaching `P` decides nothing. `witness_samples` is a member of `ModelCheckLimits`, a B-5 budget the caller sets, with a published default of 64; 0 turns phase 0 off. The seed is a request member with a published default; the run's seed is recorded in its terminal record either way. | SG-1. GV-1 |
| GE-3 | **EN-2, SMT unrolling** (ADR-018 §3), when the SMT backend exists. For SG-1 it searches for a path of at most `k` steps from each initial state to `P`. A path found is a witness (GV-1); none for some initial state settles V-5. | SG-1 only. An SG-2 or SG-3 item routed to it settles `unsupported` (ADR-014 §6 step 4) |

**Phase 1 algorithms.** After exploration, each runs in time linear in the
retained graph's nodes and edges.

- **SG-1 and SG-2: backward reachability.** A breadth-first search backwards
  over retained edges from every node where `P` holds and every open node
  computes `E⁺`, the nodes that reach `P` or an open node, with each node's
  distance `d` to the nearest such node. The closed nodes outside `E⁺` are
  **traps**: every node reachable from a trap is closed and none satisfies
  `P`. With no open node, `E⁺` is GM-4's `E(P)`.
- **SG-3: path counting.** Let `H` be the retained graph restricted to the
  nodes that reach a `Y` node or an open node, with every edge out of a `Y`
  node removed. Decompose `H` into SCCs and count paths to `Y` per node in
  reverse topological order of the condensation, saturating at 2: a `Y` node
  counts 1; a node in a non-trivial SCC of `H`, or with a self-loop, counts 2;
  any other node counts the saturated sum over its edges in `H`. A node is
  **known** when no open node is reachable from it in `H`. A node with no path
  to `Y` or to an open node counts 0 and is a trap for `Y`.

**Determinism.** EN-1's verdict and evidence are a function of the subject,
the claim, the limits and the seed (ADR-018 §3 "Determinism"). A request that
names no seed runs under the published default seed, so phase 0 on by default
keeps every run reproducible: the walks are FR-101 counter-based samples
named by their `SampleProvenance`, and phase 1 is canonical. Phase 0 changes
how an SG-1 item is settled, never which verdict it reaches: it only adds
witnesses, a witness proves SG-1 whichever phase found it (GV-1), and on a
run that phase 1 completes, `proved` from a sampled witness is the verdict
phase 1 alone would give. A sampled witness can settle an item that phase 1
would leave V-5 or V-7 under the same limits; it never settles one that
phase 1 refutes, since a witness and a trap at the same initial state cannot
both exist.

**Canonical evidence.** Edges out of a
node are ordered by their transition identity's canonical bytes (QSpec
FR-181), then by the post-state's state-key bytes.

- An SG-1 witness from initial state `i` found in phase 1 is the shortest path
  from `i` to `P`: at each node, the first edge in canonical order whose
  target has distance one less.
- A trap is the first trap node in FR-181 canonical breadth-first discovery
  order (where `Q` holds, for SG-2; where `X` holds, for SG-3), and its stem
  is the canonical breadth-first path to it.
- An SG-3 path pair from node `x` is the shortest path to `Y`, ties broken by
  canonical edge order, and the shortest path that first deviates from it:
  over every position of the first path and every other edge out of that
  position's node into `H`, the prefix, that edge, then the shortest path on;
  the least of these by length, then canonical edge order. Shortest-then-
  canonical order is a well-order on paths, so both exist whenever `x` counts
  2.

**Placement in negotiation.** EN-1's provider manifest advertises the
state-graph forms it decides beside its temporal forms. CG `negotiate_*`
routes a state-graph item to a candidate whose arm discharges its form, as
ADR-018 §3 states for temporal forms. S6a, the trace monitor, evaluates no
state-graph claim: a state-graph claim has no truth on one trace.

### 4. Verdicts

Every verdict is one QSpec FR-341 label with one QSpec FR-243 basis, carried
by an existing `TerminalValue` variant. Two verdict kinds join ADR-018's
V-1 to V-8.

| ID | Rule |
| --- | --- |
| GV-1 | **V-9 Witnessed.** Every initial state has a witness path to `P`, and each replays (GX-2). SG-1 settles `proved`, basis `decisive-witness`, `TerminalValue::Proved{basis: Witness{sources}}`, O-16 success. A witness decides SG-1 whatever the rest of the graph holds, so it settles the item even when other nodes are open or a limit later stopped the run. `ProofBasis` gains `Witness{sources}`, which `TerminalValue::category` maps to success. `sources` names, per initial state, how its witness was found: `Sampled(SampleProvenance)`, the phase 0 walk with its seed and trace index (GE-2); `Explored`, the canonical shortest path of phase 1; or `Unrolled{depth}`, an EN-2 path (GE-3). |
| GV-2 | **V-10 Refuted by trap.** A trap node is reachable where the form requires reaching `P` (or `Y`): an initial state for SG-1, a node where `Q` holds for SG-2, a node where `X` holds for SG-3. The forward closure of the trap is closed and holds no target node, so its decision scope is complete. The item settles `refuted`, basis `closed-scope`, `TerminalValue::Refuted`, O-16 violation, with a trap counterexample (GX-1) that replays (GX-3). A trap found before a run stops settles the item. |
| GV-3 | **V-4 for SG-3 with too many paths.** A known node where `X` holds counts 2. SG-3 settles `refuted`, basis `decisive-counterexample`, `TerminalValue::Refuted` (ADR-018 V-4), with a path-pair counterexample (GX-1): two distinct paths are enough, whatever the rest of the graph holds. |
| GV-4 | **V-1 for SG-2 and SG-3.** Exploration completed with no open node, and every reachable node where `Q` holds is in `E(P)` (SG-2), or every reachable node where `X` holds counts exactly 1 (SG-3). The item settles `proved`, basis `closed-scope`, `Proved{basis: Exhaustive}` (ADR-018 V-1), or `Proved{basis: Reduced{…}}` under a reduction GR-1 admits (ADR-021 RV-1). SG-1 never needs V-1: with no open node, every initial state in `E(P)` has a witness (GV-1). |
| GV-5 | **No decisive evidence.** When the run ends with no GV-1 to GV-4 result, ADR-018's map applies: a run a limit stopped settles V-7 `Incomplete`; a completed run with open nodes settles V-6 with `UndecidedSuccessor` when an undecided expansion is among them, else V-6 `ConstraintReached` (ADR-021 RV-4) when a boundary state is, else V-5 `BoundReached{depth: max_depth}`, or V-7 under POR (ADR-021 RV-5). A subject with no initial state settles V-6 `NoInitialState`. A selected reduction GR-1 does not admit settles V-6 `ReductionNotPreserving` (ADR-021 RV-2). |
| GV-6 | **Replay disagreement.** Evidence that fails its replay check settles V-6 `ReplayParity`, as ADR-018 SM-7 states for counterexamples. A witness proof also answers to replay: no `proved` from a witness reaches an item before GX-2 reproduces it. |

**Settlement method.** Every state-graph result states how it was settled,
in its `ProofBasis` and in its FR-331 terminal record's method: V-9 by
`Witness{sources}`, per initial state sampled (with seed and trace index),
explored or unrolled; V-1 by exhaustive exploration (`Exhaustive`, or
`Reduced` with its reductions); V-10 and V-4 by exploration, with the evidence
(GX-1) that replays; V-5 to V-7 with the limits the run used. The verdict
does not depend on the method: a sampled and an explored witness give the
same `proved`.

V-5 for a state-graph claim means: the search completed to depth `k` and
found neither a witness for every initial state nor a trap. It is
`inconclusive`, so a depth-limited search never reads as proof or as
refutation. No new `InconclusiveCause` is added.

A state-graph claim has no infinite-trace profile, and QSpec FR-341 names its
vocabulary for infinite-trace results. The five labels and their FR-241,
FR-242 and FR-243 combinations fit state-graph results unchanged, so QSpec
extends that vocabulary's scope to state-graph claims (QS-4).

### 5. Evidence and replay

| ID | Rule |
| --- | --- |
| GX-1 | **Shapes.** A `ModelPath` is an initial-state index and a sequence of steps, each step its FR-181 transition identity and its post-state's `quire.simulation.state-key/v1` digest, as ADR-018 CX-2 writes a step. `GraphEvidence` has three arms: `Witness{paths}`, one `ModelPath` per initial state, each ending at a node where `P` holds and carrying its source (`Sampled(SampleProvenance)`, `Explored` or `Unrolled{depth}`, GV-1); `Trap{stem}`, a `ModelPath` ending at the trap node; `PathPair{stem, first, second}`, a stem ending at a node where `X` holds and two distinct step sequences from it, each ending at its first `Y` node. Every arm carries the `over` binding of its instance. It travels in `WitnessEnvelope<GraphEvidence>` with a new `ReplaySource::ModelGraph` arm, and the envelope's obligation identity binds the subject and the claim. |
| GX-2 | **Witness replay.** E9 replay recompiles the package (FR-098), re-admits the subject's initial states and universes from the byte provision, and re-executes each path through `ModelSystem` with FR-101 `replay`, refusing as ADR-018 CX-3 refuses on a digest mismatch or a transition that is not enabled. It then evaluates `P` at each path's last node. `P` true at every last node settles `reproduced-with-evaluated-witness` and the item `proved`; `P` false at one is a disagreement (GV-6). |
| GX-3 | **Trap replay.** Replay re-executes the stem as GX-2 does and evaluates `Q` (SG-2) or `X` (SG-3) at its last node. It then explores the forward closure of that node with FR-101 `explore`, the node as the only initial state, unreduced, under the request's limits, evaluating the target predicate at each node. Exploration that completes with no target node settles `reproduced-with-evaluated-witness` and the item `refuted`. A target node found, or a stem predicate false, is a disagreement (GV-6). Exploration a limit stops settles the item V-7. Replay re-establishes the unreachability half by exploring it again, so it trusts no engine claim about the closure. |
| GX-4 | **Path-pair replay.** Replay re-executes the stem, evaluates `X` at its last node, re-executes both step sequences from that node, and checks that each ends at a node where `Y` holds, that `Y` is false at every earlier node of each, and that the two sequences differ. Agreement settles the item `refuted`; any failed check is a disagreement (GV-6). |
| GX-5 | **Engine independence.** Witnesses from phase 0, phase 1 and EN-2 are the same `ModelPath` and replay the same way; the source changes no replay step. A sampled witness replays from its recorded steps, without re-running the sampler. A reduced search concretises its evidence as ADR-021 EI-6 concretises a counterexample, so no evidence carries a canonical state, a permutation or a reduction. |

### 6. Interactions with the sibling records

| ID | Rule |
| --- | --- |
| GR-1 | **Reductions (ADR-021).** A state-graph claim admits each reduction exactly where its row below reads "yes", which extends ADR-021 PT-2. "No" settles the item before expansion, V-6 `ReductionNotPreserving`. |
| GR-2 | **Symmetry.** The predicates `P`, `Q`, `X` and `Y` join the clauses ADR-021 SYM-3 requires to be identity-transparent. Under an admitted declaration (ADR-021 SYM-5) the quotient graph is bisimilar to the subject on symmetric predicates, and bisimulation preserves `EF` and `AG EF` (Emerson and Sistla; Clarke, Emerson, Jha and Sistla). An instance's binding is handled as ADR-021 SYM-6 states. A quotient node stands for many nodes and a quotient edge for many edges, so path counts are not preserved. |
| GR-3 | **Partial-order reduction.** With every transition that writes a location `P` reads made visible (ADR-021 POR-5), the reduced graph holds, for every behaviour, a stutter-equivalent behaviour on the visible atoms; a path that reaches a `P` node has a representative that reaches one, and every reduced path is a path of the subject, so SG-1 is preserved. `AG EF` is a branching property: a node the reduction omits can be a trap, and a reduced node can reach `P` only through omitted nodes. Branching-time reduction needs a further ample-set condition that ADR-021 does not build, so SG-2 is not preserved. Pruned interleavings are distinct paths, so SG-3 is not preserved. |
| GR-4 | **State constraints.** Boundary states are open (GM-6). A witness through expanded states, or ending at a boundary state where `P` holds, is a path of the subject. A trap's closure holds no open node, so it lies wholly inside the expanded graph and is a trap of the subject. A proof needs no open node. |
| GR-5 | **Strong fairness (ADR-019).** GM-7 refuses every fairness constraint on a state-graph claim, so the fairness kind plays no part. |
| GR-6 | **Refinement (ADR-020).** Refinement is inclusion of behaviours. A state-graph claim of the abstract model reads which paths exist, and the concrete model may have fewer, so a state-graph claim is outside ADR-020 CO-3's transfer fragment and is checked on the concrete subject directly. |

| Form | Symmetry (admitted, ADR-021 SYM-5) | POR (ADR-021 POR-6) | State constraint |
| --- | --- | --- | --- |
| SG-1 possible | yes | yes, with `P`'s reads visible | witness real; trap real (its closure holds no boundary state); otherwise `ConstraintReached` |
| SG-2 always possible | yes | no: branching property | trap real; proof only when no boundary state is reached |
| SG-3 unique path | no: the quotient merges paths | no: prunes distinct paths | path pair real; trap real; proof only when no boundary state is reached |

Under POR, an SG-1 trap replays by exploring the unreduced closure (GX-3), so
an SG-1 refutation costs replay the subject's full reachable graph from that
initial state. Under symmetry the same holds for the concrete closure. A
witness found by a reduced search is concrete (GX-5), so its proof basis is
`Witness`, never `Reduced`, as ADR-021 RV-7 states for refutations.

### 7. Worked examples

All state counts below are hand enumeration; no QSL engine produced them.

#### 7.1 `possible`: ConfigVersion

The subject is ADR-018 §6's modified ConfigVersion unit, whose
`attemptUpdate` advances `versionNumber` modulo 3, with universe
`config_history = {a, b}` and one initial snapshot with both at version 0.
States are `(va, vb)`; `upd(a)` is `attemptUpdate` with receiver `a`.

```text
possible ReachesTwo using v over (c: Config::ConfigVersion) {
  c.versionNumber = 2
}
possible ReachesThree using v over (c: Config::ConfigVersion) {
  c.versionNumber = 3
}
```

**Graph.** 9 nodes, 18 edges, every node closed. Each node has the two
successors `upd(a)` and `upd(b)`.

**`ReachesTwo`.** Instance `c = a`: the nodes with `va = 2` have `d = 0`,
`va = 1` have `d = 1`, `va = 0` have `d = 2`. The canonical witness from
`(0, 0)` takes `upd(a)` (first in canonical order, to `d = 1`) twice:
`(0, 0) -upd(a)-> (1, 0) -upd(a)-> (2, 0)`. Instance `c = b` is the same with
`upd(b)`. Each instance settles `proved`, `decisive-witness`,
`Proved{basis: Witness{sources}}` (V-9), after GX-2 replays two steps and evaluates
`c.versionNumber = 2` true at the last node. With phase 0 on, as by
default, a walk from `(0, 0)` that reaches `va = 2` settles the instance
first, with source `Sampled` and that walk's seed and trace index, and may be
longer than the explored witness; with `witness_samples` 0 the source is
`Explored`. The verdict is `proved` either way.

**`ReachesThree`.** `versionNumber` ranges over `Int[0, 1000]`, but the post
clause keeps it in `{0, 1, 2}`, so no node satisfies `P`. `E⁺` is empty and
the initial state `(0, 0)` is a trap. Each instance settles `refuted`,
`closed-scope` (V-10), with `Trap{stem}` whose stem is the initial state and
no step. GX-3 explores the closure of `(0, 0)`, all 9 nodes, and finds no
node with `versionNumber = 3`.

The derived deadlock-freedom item settles `proved` (V-1), as in ADR-018 §6.

#### 7.2 `always possible`: a game that can be lost

Object type `Game` with `phase: Phase`, an enum `{Start, Mid, Won, Lost}`;
population `games`, universe `{g}`, initial phase `Start`. Operations, each
with frame `modifies [phase]`, no parameters and no result: `play` (pre
`phase = Start`, post `phase = Mid`), `win` (pre `phase = Mid`, post
`phase = Won`) and `lose` (pre `phase = Mid`, post `phase = Lost`).

```text
always possible CanStillWin using v over (x: G::Game) {
  x.phase = Won
}
```

**Graph.** 4 nodes, 3 edges: `Start -play-> Mid`, `Mid -lose-> Lost`,
`Mid -win-> Won`. Canonical discovery order is `Start`, `Mid`, `Lost`, `Won`,
since `G::Game::lose` precedes `G::Game::win` in transition-identity order.

**Verdict.** `E(P) = {Won, Mid, Start}`. `Lost` is outside it, closed, and
terminal, so it is a trap. The item settles `refuted`, `closed-scope`
(V-10), with `Trap{stem}`: the initial state, `play`, `lose`. GX-3 replays
the two steps, then explores the closure of `Lost`, which is `{Lost}`, and
finds `phase = Won` false there.

The model declares no `terminal` member, so the deadlock-freedom item is
refuted at `Lost`, the first terminal state in discovery order, with the same
two-step prefix. A `terminal when` predicate that holds where the phase is
`Won` or `Lost` settles the deadlock-freedom item `proved` and leaves
`CanStillWin` refuted (GM-8).

**With `from`.** `always possible CanStillWin using v over (x: G::Game) from
(x.phase != Lost) { x.phase = Won }`: the nodes where `Q` holds are `Start`,
`Mid` and `Won`, all in `E(P)`, and every node is closed. The item settles
`proved`, `closed-scope`, `Proved{basis: Exhaustive}` (V-1), over 4 nodes.

**With a restart.** Adding `restart` (pre `phase = Lost`, post
`phase = Start`) adds the edge `Lost -restart-> Start`; `E(P)` is all 4
nodes and the claim without `from` settles `proved` (V-1).

#### 7.3 `unique path`: two steps in either order

Object type `Job` with `a`, `b` and `done`, all `Bool`; population `jobs`,
universe `{j}`, initial state all false. Operations: `stepA` (pre `not a`,
post `a`, frame `[a]`), `stepB` (pre `not b`, post `b`, frame `[b]`) and
`finish` (pre `a and b and not done`, post `done`, frame `[done]`). States
are `(a, b, done)` written with `t` and `f`.

```text
unique path InOneWay using v over (j: M::Job)
  from (not j.a and not j.b) to (j.done)
```

**Graph.** 5 nodes, 5 edges: `fff -stepA-> tff`, `fff -stepB-> ftf`,
`tff -stepB-> ttf`, `ftf -stepA-> ttf`, `ttf -finish-> ttt`. The only node
where `X` holds is `fff`.

**Counts.** `H` is the whole graph and has no cycle. `ttt` counts 1, `ttf` 1,
`tff` 1, `ftf` 1, and `fff` the saturated sum 2. The item settles `refuted`,
`decisive-counterexample` (V-4), with `PathPair`: empty stem; first path
`stepA, stepB, finish`, the shortest with `stepA` first in canonical order;
second path `stepB, stepA, finish`, the deviation at position 0. GX-4 replays
both and checks `done` is false before each last node.

**Sequenced.** With `stepB`'s precondition `a and not b`, the graph is 4 nodes
and 3 edges, `fff`, `tff`, `ttf`, `ttt`, and `fff` counts 1. The item settles
`proved`, `closed-scope`, `Proved{basis: Exhaustive}` (V-1).

In both models `ttt` is terminal, so the deadlock-freedom item is refuted at
`ttt` unless the model declares `terminal when` a predicate that holds there.

### 8. Downstream impact and sequencing

**Stage DAG (ADR-011 §1).** No stage or edge is added. State-graph items run
in ADR-018's S6c. A V-1 proof, a bound reached, an undecided run or a stopped
run settles the item's terminal record there. Every result that carries
evidence (V-9, V-10, V-4) leaves S6c over ADR-018's E11 as a typed witness,
reaches S8 through E9, and settles its terminal record only after replay,
as an EN-1 counterexample does.

| ID | Repository | Change |
| --- | --- | --- |
| DS-1 | QSL | S3: the three claim forms (SG-1 to SG-3) and their form record, the state-only predicate check (GM-3), the fairness refusal (GM-7). The request writer: the deadlock-freedom item for subjects of state-graph items (GM-8). Layer 5 `model_check`: node labelling, phase 0 sampling (GE-2), backward reachability, path counting, open-node tracking (GM-6), canonical evidence (§3), the GR-1 rows in the pre-check. `qsl-replay`: `ProofBasis::Witness`, `GraphEvidence`, `ReplaySource::ModelGraph` and its result arm with GX-2 to GX-4. The EN-1 manifest's state-graph forms. |
| DS-2 | CG | The EN-1 `negotiate_*` arm reads the state-graph form; an SMT arm discharges SG-1 only. `Proved{basis: Witness{sources}}` in the outcome map. |
| DS-3 | IR | SG-1 path search in the SMT encoding (GE-3), after IR admits state nodes (ADR-018 DS-3). |
| DS-4 | Driver | Runs a state-graph item routed to EN-1 in process and runs E9 for its evidence. |
| DS-5 | QSpec | §9. |

**Sequencing.**

1. **Specification.** This record, then QSpec (§9), then the QSL compiler
   requirements.
2. **Prerequisite.** ADR-018 EN-1 with edge retention (its steps 2 and 3).
3. **SG-1 and SG-2 on EN-1.** Backward reachability, traps, witnesses and
   their replay. §7.1 and §7.2 are the first conformance vectors.
4. **SG-3 on EN-1.** Path counting and the path pair. §7.3 is its vector.
5. **Phase 0 sampling.** Independent of steps 3 and 4 once SG-1 replay
   exists.
6. **SG-1 on EN-2.** When the SMT backend exists and IR admits state nodes (ADR-018 DS-3).

### 9. What QSpec must specify

| ID | Item | Where |
| --- | --- | --- |
| QS-1 | The state-graph claim family in the shared grammar: `possible`, `always possible … from`, `unique path … from … to`, with keywords distinct from `reaches` (V1-TYPE-024); refusal of a temporal operator, an anchor or pre-state read, or a fairness constraint inside one | shared grammar; a new FR beside QSpec FR-161 |
| QS-2 | Semantics over a model subject: the state graph with no stutter step (GM-1), reachability (GM-2), state-only predicates (GM-3), the fixpoint `E(P)` and the truth of SG-1 and SG-2 (GM-4), paths and SG-3 (GM-5), the `over` binding (ADR-018 QS-5), and why fairness is not admitted (GM-7) | the new FR |
| QS-3 | Open and closed nodes (GM-6) and which evidence is decisive in a partial exploration: a witness, a trap with a closed closure, a path pair through known nodes | the new FR |
| QS-4 | Verdicts: V-9 (`proved`, `decisive-witness`) and V-10 (`refuted`, `closed-scope`), V-4 for a path pair, V-1 for SG-2 and SG-3, and V-5 to V-8 as GV-5 states; extend QSpec FR-341's scope from infinite-trace results to state-graph results, with its five labels unchanged | QSpec FR-341 (infinite-trace), FR-331, the new FR |
| QS-5 | The `GraphEvidence` wire (GX-1) and its replay rules (GX-2 to GX-4), including trap replay as an exhaustive exploration from a recorded state | the counterexample contract; QSpec FR-331 |
| QS-6 | QSpec FR-181: exhaustive exploration from a given state (the trap closure, GX-3) and the explored graph's edges as the subject of state-graph claims; FR-181-AC-5's replay covers witness paths | QSpec FR-181 |
| QS-7 | Request members: `witness_samples` in `ModelCheckLimits` with its published default 64, the seed with its published default, and the settlement method in the terminal record (GE-2, GV-1); the state-graph forms in the provider manifest and a negotiation that routes a form only to a candidate that discharges it | QSpec FR-331, FR-290 |
| QS-8 | The deadlock-freedom item is added for subjects of state-graph items too (GM-8) | ADR-018 QS-12's FR |
| QS-9 | The GR-1 rows in the preservation table of ADR-021 QS-5 | ADR-021 QS-5's FR |
| QS-10 | Conformance vectors, each with expected verdict kind and evidence that must replay (validity is checked, not the exact evidence): (a) §7.1, `ReachesTwo` witnessed and `ReachesThree` refuted by a trap at the initial state; (b) §7.2, the trap at `Lost`, the `from` variant proved, the restart variant proved, and the deadlock-freedom item with and without `terminal when`; (c) §7.3, the path pair and the sequenced variant; (d) a cycle on a path to `Y`, refuted with a path pair; (e) a partial run: a trap found with `max_depth` reached elsewhere settles V-10, and a run with no trap and open nodes settles V-5; (f) replay disagreements: a witness whose last node fails `P`, a trap whose closure reaches the target, a path pair whose paths are equal | new TCs |

### 10. Rulings on the draft's questions

The owner ruled on the four questions the draft left open, on 2026-10-01.

| ID | Question | Ruling | Rationale | Where it lands |
| --- | --- | --- | --- | --- |
| RU-1 | Initial-state quantifier for `possible` | **Every initial state**, the CTL reading | Every other verdict in ADR-018 and here quantifies over every initial state, and CTL reads a model that way. An author who means one start lists that one initial snapshot in the subject | SG-1, GM-4; a witness per initial state (GV-1, GX-1) |
| RU-2 | `from (Q)` on `always possible` | **Kept** | It states "recovery is possible from every non-lost state", and it costs nothing in the algorithm: the trap search reads `Q` at the candidate node | SG-2, GM-4, GV-2; §7.2 |
| RU-3 | How `unique path` counts | **Per `X` node** | Each reachable `X` node has exactly one path to `Y`; a refutation names the one node that fails, with a path pair or a trap from it | SG-3, GM-5, GV-2, GV-3 |
| RU-4 | Phase 0 default | **On by default**, `witness_samples` with a published default of 64, settable to 0. The result states how it was settled: a sampled witness with its seed and trace index, or exploration | Sampling is cheap and settles many `possible` items early. Naming the method in the result keeps a sampled proof visible as one, and the verdict is the same either way. The seed has a published default, so a run with phase 0 on is reproducible | GE-2, §3 "Determinism", GV-1 "Settlement method", GX-1 |

## Consequences

- An author states that something is possible, always still possible, or
  reachable in exactly one way, and gets `proved` or `refuted` from QSL's own
  explicit-state engine over a finite subject.
- Temporal claims keep one semantics, the trace evaluator; state-graph claims
  have a graph semantics of their own and never enter a temporal formula.
- Each form costs one pass over the retained graph after exploration, and one
  exploration serves every state-graph item of a subject.
- Every `proved` from a witness and every `refuted` replays without the engine
  that found it. A trap's unreachability is re-established by a fresh
  exploration of its closure, which costs replay the size of that closure.
- A partial exploration still settles an item when its evidence is decisive:
  a witness proves `possible`, and a trap with a closed closure refutes
  `possible` and `always possible`, whatever else remains open.

## Amendments made with this record

- ADR-018 §1: `ProofBasis` gains `Witness`; the verdict kinds V-9 and V-10
  are this record's (GV-1, GV-2). §10 DL-3: the deadlock-freedom item is added for subjects of
  state-graph items too (GM-8).
- ADR-014 §6 item 4: a candidate's arm also decides whether it discharges a
  state-graph form (SG-4).

## Amendments to make on acceptance

- **ADR-021** (when accepted): PT-2 gains the three rows of §6; SYM-3's list
  of evaluated clauses gains the state-graph predicates (GR-2).
- **ADR-020** (when accepted): CO-3 notes that state-graph claims do not
  transfer and are checked on the concrete subject (GR-6).
- **ADR-013** O-16 and the `TerminalValue` row: `ProofBasis::Witness` maps to
  success.
- **ADR-011** §6.1: layer 5 `model_check` gains state-graph checking; layer 6
  `replay` gains the model-graph arm.
- **QSpec FR-181** (through the paired QSpec ticket): QS-6.
- `spec/spec.md`: index row.

## Alternatives Considered

- **CTL operators inside temporal formulas.** Rejected (§1). A formula would
  lose its truth value on one trace, and every replay, monitor and engine
  that answers to ADR-018 SM-1 would need a second semantics. Mixed CTL and
  LTL is CTL*, whose checking costs more.
- **A general CTL fragment as its own family** (`EX`, `EU`, `AF`, `AU` and
  nesting). Not chosen. `EF` and `AG EF` cover the stated needs; each further
  operator adds an evidence shape and a replay rule. A later form joins the
  family as its own SG row.
- **`possible` by negating an invariant**, as TLC users do: check
  `always holds(not P)` and read its counterexample as the witness. Rejected.
  The claim would read as the opposite of what the author means, its
  `refuted` would mean success, and it cannot express `AG EF`.
- **Witness sampling as the only engine**, as in Quint's witnesses mode.
  Rejected. Sampling proves `possible` and decides nothing else; GE-2 keeps
  it as a fast first phase.
- **Witness sampling off by default.** Rejected (RU-4). Sampling is cheap,
  settles many `possible` items before any exhaustive exploration, and states
  in the result that it did.
- **`possible` from some initial state.** Not chosen. Every other verdict in
  ADR-018 and here quantifies over every initial state, and CTL reads a model
  that way. An author who means one start lists that one initial snapshot in
  the subject.
- **Fairness on state-graph claims.** Rejected (GM-7). Machine closure makes
  it change no verdict.
- **The trap's closure in the evidence**, as a list of state digests.
  Rejected. Replay must explore the closure to check it, and exploring it
  needs no list.
- **`decisive-counterexample` for a trap.** Not chosen. A trap's verdict rests
  on its whole closure being explored, which is QSpec FR-243's `closed-scope`.
- **Path counts beyond one** (`at most n`, `exactly n`). Not chosen for the
  first query. A refutation of "exactly `n`" by too few paths when `n > 1`
  has no finite counterexample short of the whole closure.

## References

- Owning ticket: Linear QSL-369. Built on ADR-018 (Linear QSL-366).
  Interacts with ADR-019 (Linear QSL-365), ADR-020 (Linear QSL-367) and
  ADR-021 (Linear QSL-368). A paired QSpec STD ticket carries §9.
- Later research, outside this record: Linear RES-40 (a planner whose plan
  is a `possible` witness, and a runtime monitor that asks whether recovery
  is still possible from the live state).
- H. Wayne, on properties TLA+ cannot express natively: `EF` and `AG EF`.
- Quint's `--witnesses` option: sampled witnesses for reachability.
- E. M. Clarke, O. Grumberg and D. Peled, *Model Checking*, MIT Press, 1999:
  CTL fixpoint checking, linear in the state graph.
- E. A. Emerson and A. P. Sistla, "Symmetry and model checking", Formal
  Methods in System Design, 1996; E. M. Clarke, E. A. Emerson, S. Jha and
  A. P. Sistla, "Symmetry reductions in model checking", CAV 1998: symmetry
  preserves CTL* over symmetric atoms.
- R. Gerth, R. Kuiper, D. Peled and W. Penczek, "A partial order approach to
  branching time logic model checking", Information and Computation, 1999:
  the further ample-set condition branching properties need.
