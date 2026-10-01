---
id: ADR-018
title: "Temporal properties over every behaviour of a model"
type: ADR
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-016
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-017
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-057
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-069
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-070
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-072
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-097
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: relates_to
  - target: ix://agent-ix/quire-specification/FR-090
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-161
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-181
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-243
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-250
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-290
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-331
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-341
    type: depends_on
---
# ADR-018: Temporal properties over every behaviour of a model

## Status

Proposed, 2026-10-01. Draft for plan-lead review of the key decisions; the
QSL compiler requirements that implement it follow review. §9 records the
owner's rulings on the draft's four questions; §10 and §11 carry the two that
add design. The owning ticket and related work are listed under References.

"QSpec FR-nnn" names a quire-specification requirement; a bare FR id is a QSL
requirement. QSpec has two artifacts numbered FR-341; this record cites only
the infinite-trace result disposition vocabulary, written "QSpec FR-341
(infinite-trace)". Item ids `TP-`, `V-`, `SM-`, `EN-`, `FA-`, `CX-`, `DS-`,
`QS-`, `RU-`, `DL-` and `IV-` are local to this record. Other artifacts cite
them as `ADR-018 SM-2`.

## Context

QSL has three ways to look at the behaviour of a model today, and none of
them gives a verdict over every behaviour for a temporal property.

- **Finite exploration.** FR-101 implements QSpec FR-181: canonical
  breadth-first exploration over any `TransitionSystem`, with state keys,
  frontiers, `Limits{max_states, max_depth, max_transitions}` and
  `Outcome{Exhaustive, Bounded, Cancelled}`. FR-120's `ModelSystem` supplies
  the successor relation of a checked state model: an operation applied to an
  argument vector from finite parameter domains, enabled when its effective
  precondition holds, times every post-state in its frame that satisfies its
  postcondition. Exploration checks state invariants only, as findings.
  ADR-016 §6 keeps every exploration result out of proof accounting.
- **Trace evaluation.** ADR-014 A-4 evaluates a temporal clause over one
  given trace: bounded profiles over a finite trace, the infinite-trace
  profile three-valued over a finite prefix and exactly over a fair lasso.
  This is replay and monitoring of a supplied trace.
- **Negotiated proof.** ADR-014 A-5 sends proof of an infinite-trace claim
  through negotiation only, and QSpec FR-161-AC-7 settles it `unsupported`
  with a warning while no liveness backend is registered. No liveness backend
  exists. The SMT backend is planned in the contract IR lane and is not
  built; QSL contains no SMT code.

QSpec supplies the semantics this record builds on. QSpec FR-161 defines
infinite-trace meaning for `always`, `eventually`, `until`, `release` and the
past operators, states that fairness filters traces before evaluation, that a
counterexample is a finite prefix plus a loop, and that finite prefixes or
absence of a counterexample never prove infinite satisfaction. QSpec FR-090
fixes four profiles; three are bounded (interval on every operator) and
`quire.temporal.infinite-trace/v1` admits no interval today, which §11 and
QS-13 change. QSpec FR-341
(infinite-trace) maps `proved`, `refuted`, `inconclusive`, `unsupported` and
`failed` onto the QSpec FR-241 execution, FR-242 truth and FR-243 settlement
basis axes. QSpec FR-243's bases are `closed-scope`, `decisive-witness`,
`decisive-counterexample`, `unsettled` and `unavailable`.

QSL's result vocabulary is ADR-013 O-16's eight categories and
`qsl_replay::TerminalValue` (`Proved{success_checks}`, `Tested`, `Refuted`,
`Declined`, `Unsupported`, `Incomplete`, `Failed`, and the `Inconclusive`
variant ADR-013 C-09 adds), with `InconclusiveCause`
(`KaniVacuousProof`, `ReplayParity`, `ReplayRefused`) and `IncompleteCause`
(`TimedOut`, `Cancelled`, `ResourceExhausted`).

## Decision

### 1. Property forms with an every-behaviour verdict

A temporal claim over a **model subject** (§2) gets an every-behaviour
verdict in four forms. The form is computed by the S3 TemporalTrace `check`
from the checked formula and recorded beside the claim's requirement record.

| ID | Form | Profile | Shape | Engines that settle it (§3) |
| --- | --- | --- | --- | --- |
| TP-1 | Reachable-state invariant | infinite-trace | `always holds(I)` where `I` is a state predicate, including an `invariant` clause named by the request | explicit-state (exhaustive); SMT k-induction (inductive); SMT unrolling (refutation, bounded-to-k) |
| TP-2 | Bounded MLTL | event-position false-extension | any bounded-profile clause. With activation `on origin` its truth on a behaviour depends only on the first `h + 1` positions, `h` its ADR-014 TR-4 horizon. With activation `on each` it holds at every activating position, which makes it an unbounded safety property | `on origin`: explicit-state (exhaustive) and SMT unrolling to `h` (bounded-complete). `on each`: as TP-3 |
| TP-3 | LTL safety | infinite-trace | a formula in ADR-014 A-4's safety fragment, which admits interval operators (IV-4) | explicit-state (exhaustive); SMT k-induction (inductive); SMT unrolling (refutation, bounded-to-k) |
| TP-4 | LTL liveness | infinite-trace | any other admitted infinite-trace formula, including one with interval operators nested under unbounded ones (IV-1), with its fairness constraints (§4) | explicit-state (exhaustive); SMT lasso unrolling (refutation, bounded-to-k) |

The model subject admits two profiles: event-position false-extension and
infinite-trace. Both read one admitted semantic-event position per step, and
a model's behaviour is a sequence of steps. The fixed-sample and
timestamped-event profiles need a clock that a model behaviour carries no
value for, so a claim under either over a model subject settles
`unsupported`, `unsupported-requested-capability`, at negotiation.

Each model subject also gets one derived **deadlock-freedom** item, a TP-1
invariant, unless its state model opts out (§10, DL-3).

**Verdict kinds.** Each verdict is one QSpec FR-341 (infinite-trace) label
with one QSpec FR-243 basis, carried by an existing `TerminalValue` variant.
The strength of a proof is the basis plus a `ProofBasis` member that this
record adds to `TerminalValue::Proved`. No new result axis, category or label
is added.

| ID | Verdict kind | Meaning | QSpec FR-341 label | QSpec FR-243 basis | `TerminalValue` | O-16 category |
| --- | --- | --- | --- | --- | --- | --- |
| V-1 | Exhaustive | Every admitted behaviour of the subject was examined | `proved` | `closed-scope` | `Proved{basis: Exhaustive}` | success |
| V-2 | Bounded-complete to `k` | Unrolled to depth `k` at or past the formula's horizon (TP-2 `on origin` only), so every behaviour is decided | `proved` | `closed-scope` | `Proved{basis: BoundedComplete{depth: k}}` | success |
| V-3 | Proved by induction at `k` | The property is `k`-inductive: it holds in the first `k` steps of every behaviour, and any `k` consecutive states of the declared types that satisfy it have only satisfying successors | `proved` | `decisive-witness` | `Proved{basis: Inductive{depth: k}}` | success |
| V-4 | Refuted | A counterexample (§5) reproduced by replay | `refuted` | `decisive-counterexample` | `Refuted` | violation |
| V-5 | Bounded-to-`k` | The engine completed its analysis to depth `k` and found no counterexample of length at most `k` | `inconclusive` | `unsettled` | `Inconclusive(BoundReached{depth: k})` | inconclusive |
| V-6 | Inconclusive, other cause | `InductionNotClosed{depth: k}`: base case holds to `k`, the step case has a counterexample-to-induction. `UndecidedSuccessor`: a contract conjunction evaluated undecided during expansion, so the successor relation is not exactly known (FR-120 `ContractUndetermined`). `NoInitialState`: the subject has no initial state (ADR-016 EX-10). `ReplayParity` and `ReplayRefused`: as ADR-013 C-09 | `inconclusive` | `unsettled` | `Inconclusive(cause)` | inconclusive |
| V-7 | Stopped | A run limit stopped the analysis before it completed its method: state or transition limit, automaton-state limit (IV-6), meter, time, cancellation | `failed`, execution `resource-incomplete` | `unavailable` | `Incomplete(cause)` | incomplete |
| V-8 | Unsupported | No candidate settles the form; or a profile the model subject does not admit | `unsupported` | `unavailable` | `Unsupported(cause)` | unsupported |

`ProofBasis` is `Checks{success_checks}` (Kani, unchanged in meaning: zero
is the vacuous proof), `Exhaustive`, `BoundedComplete{depth}` and
`Inductive{depth}`. `TerminalValue::category` maps `Checks{0}` to
inconclusive with `KaniVacuousProof`, as today, and every other basis to
success. `InconclusiveCause` gains `BoundReached{depth}`,
`InductionNotClosed{depth}`, `UndecidedSuccessor` and `NoInitialState`.

**Length.** The length of a counterexample is its number of transitions,
prefix and loop together. "No counterexample of length at most `k`" is the
one definition of V-5 for every engine.

**Depth is a method parameter.** The unrolling or exploration depth `k` is
an ADR-014 B-5 value. A run that completes every depth up to `k` has
execution `completed` and truth `pending`, so it settles V-5. A run that
stops before completing `k` settles V-7. The other ADR-014 B-5 budgets (time,
memory, solver resources) settle V-7, as ADR-014 §7 states.

**Scope.** A verdict holds for exactly its subject (§2): the checked package,
the initial states, the universes and any substituted `ProofBound`s. The
subject is part of the obligation identity (ADR-013 O-09), so a verdict over
one universe is joined only to its own request (ADR-014 §8).

### 2. One semantics for traces and models

| ID | Rule |
| --- | --- |
| SM-1 | **The trace evaluator is the semantics.** The truth of a formula on one trace is the value the layer-5 TemporalTrace evaluator returns (ADR-014 A-4): the bounded profile's own evaluation over a finite trace, exact evaluation over a lasso under infinite-trace, three-valued evaluation over a finite prefix under infinite-trace. Replay, monitors and every model-checking engine use this one definition. |
| SM-2 | **Model subject.** A model subject is a checked package with a state model, a non-empty list of initial states (FR-106 snapshots, admitted as FR-120 states them), the universes of its populations, and any `ProofBound`s. Its successor relation is FR-120's. |
| SM-3 | **Positions.** A behaviour is a maximal path of the successor relation from an initial state. Position 0 is the initial state, observed with an `initialization` anchor. Position `i > 0` is the post-state of the behaviour's `i`-th transition, observed with that transition's operation anchor. This is FR-120's synthesized state observation (ADR-016 ID-10), anchored by the transition the behaviour took. A `holds` atom evaluates through the one clause evaluator (ADR-016 FE-3) over the observation at its position. The step sequence is the event-position sequence authority of both admitted profiles. |
| SM-4 | **Terminal states.** A state with no enabled successor is terminal. Every terminal state reads by this rule, whether the model intends it or it is a deadlock (§10, DL-6). Under a bounded profile, a behaviour that reaches a terminal state is a closed finite execution, read with that profile's closed-boundary rule, as replay reads a closed trace. Under infinite-trace, it is extended by a terminal stutter step that repeats the terminal state forever. The stutter step has its own transition identity, enables no operation and is outside every fairness constraint. |
| SM-5 | **Model verdict.** A formula holds for a subject exactly when SM-1 returns true on every admitted behaviour: every behaviour under a bounded profile; every fair behaviour (§4) under infinite-trace. A refutation is one admitted behaviour on which SM-1 returns false. |
| SM-6 | **Bounded and unbounded share the definition, not the representation.** ADR-014 TR-3 keeps bounded-profile and infinite-trace operators in distinct representations, and this record keeps that. An interval operator inside an infinite-trace formula is an infinite-trace operator (IV-1), and the infinite-trace translation unrolls it (IV-3). Both reach the engines through one interface, a property automaton over positions (§3), built by one translation per profile: a deterministic finite monitor for a bounded formula, a generalized Büchi automaton for the negation of an infinite-trace formula. Past operators are tracked as monitor state (each past subformula's truth at the previous position), so the automaton stays finite. |
| SM-7 | **Engine encodings answer to the evaluator.** Every refutation is replayed through SM-1 before it settles `refuted` (§5), as ADR-013 C-09 requires of a Kani counterexample. Every translation is tested against SM-1 on the QSpec conformance lassos (§8). An engine whose verdict disagrees with SM-1 on a replayed counterexample settles `inconclusive`, `ReplayParity`. |

### 3. Engines

| ID | Engine | Where it lives | Forms and verdicts |
| --- | --- | --- | --- |
| EN-1 | **Explicit-state product.** The product of the subject's state graph with the property automaton, explored by FR-101's canonical breadth-first engine. The product is itself a `TransitionSystem`: its state is (model state, automaton state), its key is (model state key, automaton state index). The engine retains every product edge it explores. A second phase decomposes the retained graph into strongly connected components and finds a fair accepting cycle (§4). A safety form (TP-1, TP-2, TP-3) needs only the first phase: reaching a rejecting monitor state is a violation. | QSL layer 5, module `model_check`, after `simulation` | TP-1 to TP-4. V-1, V-4, V-5 (with `max_depth`), V-6, V-7 |
| EN-2 | **SMT unrolling.** The transition relation encoded over `k` steps, for bounded violations and for lasso-shaped violations with a loop back-edge inside `k`; fairness constraints are constraints on the loop segment. | the SMT backend in the contract IR lane, reached by negotiation | TP-1 to TP-4 refutation (V-4); V-2 for TP-2 `on origin` at `k >= h`; V-5 otherwise |
| EN-3 | **`k`-induction.** Base case: no violation in the first `k` steps. Step case: `k` consecutive states that satisfy the property, with pairwise distinct states, have no violating successor. | the same SMT backend | TP-1, TP-3 and TP-2 `on each`, through the safety monitor product. V-3, V-6 `InductionNotClosed` |

**Acceptance-cycle detection is SCC-based.** EN-1 uses one algorithm:
decompose the retained product graph into SCCs, then test each non-trivial
SCC that contains an accepting state against the fairness filter (§4). This
handles generalized Büchi acceptance and weak fairness in one pass over each
SCC, and it is the seam where strong fairness adds SCC refinement. The
retained graph also serves the other graph queries that follow this record
(§7).

**Determinism.** EN-1's verdict is a function of the subject, the clause and
the limits. Its counterexample is canonical: the stem is the first path in
FR-181 canonical breadth-first order to the first accepting SCC that passes
the fairness filter, and the loop is the shortest cycle inside that SCC that
visits an accepting state and every fairness obligation, ties broken by
canonical transition order. SCC enumeration order follows discovery order and
never hash-map iteration (ADR-016 ND-4).

**EN-1 pre-check.** Before any expansion, EN-1 classifies every root of the
subject (FR-120 `domains()` under the request's universes). An unbounded root
returns `requires-bound`, which the caller answers with a universe for a
population root or a `bounded_domain` in the model for a parameter, result or
field root, as ADR-016 §2 states for simulation.

**Explicit-state limits.** EN-1 runs under FR-101's `Limits`. Reaching
`max_depth` with no counterexample settles V-5 with `k = max_depth`;
reaching `max_states` or `max_transitions`, the automaton-state limit
`max_automaton_states` (IV-6), a clause meter, or cancellation settles V-7.
This map is `model_check`'s own; `explore::Outcome::category()` keeps
ADR-014 §7's map for simulation.

**Placement in negotiation.** Both engines are backends. QSL publishes a
provider manifest for EN-1 advertising (`temporal-satisfaction`, `bounded`)
and (`temporal-satisfaction`, `unbounded`); the SMT backend's manifest is its
lane's. CG `negotiate_*` settles each item, with an arm per backend that
reads the recorded property form (§1) and the subject classification. The
driver runs an item routed to EN-1 in process, through layer 5, and an item
routed to the SMT backend through S5 and E7 as for Kani. A `proved` reaches
an item only through this path (ADR-014 A-5).

### 4. Fairness

| ID | Rule |
| --- | --- |
| FA-1 | A fairness constraint is part of an infinite-trace clause (QSpec FR-161 "fairness assumptions"). It names one operation of the model and a granularity: **whole**, one constraint over every transition of the operation; or **each**, one constraint per transition identity (operation, receiver and argument vector). The clause's fairness set enters its obligation identity and every counterexample (QSpec FR-161-AC-5). The empty set is a valid premise: every behaviour is admitted. |
| FA-2 | **Enabled.** A transition identity is enabled at a state when the operation's effective precondition holds for that receiver and argument vector and at least one post-state satisfies its postcondition (FR-120's successor rule). A whole-operation constraint is enabled when any of its transition identities is. |
| FA-3 | **Weak fairness.** A behaviour satisfies weak fairness for a constraint when, from no position onward, the constraint stays enabled at every position while none of its transitions is taken. On a lasso this reduces to the loop: the constraint is taken somewhere in the loop or disabled somewhere in the loop. |
| FA-4 | **Fairness filter.** An accepting SCC passes the filter when, for every constraint, the SCC contains an edge of that constraint or a state where it is disabled. A passing SCC contains a fair accepting cycle, and EN-1 builds the loop from it (§3). |
| FA-5 | **Strong fairness seam.** The constraint type is `FairnessConstraint{kind: FairnessKind, operation, granularity}` with `FairnessKind::Weak`. The filter is one function over an SCC and the constraint set. A new kind adds a variant and its filter rule, which may refine an SCC into smaller ones and re-test them; the product, the SCC decomposition, the counterexample shape and the verdict vocabulary stay as they are. |
| FA-6 | **Unmarked granularity.** A constraint written without a granularity is `whole`: `fair weak Op` means `fair weak whole Op`, TLA+'s `WF` over the action that is the existential over the operation's receivers and argument vectors. `each` is always written. |

Weak fairness of operations is machine-closed: every finite prefix of a
behaviour extends to a fair behaviour. Two consequences follow. A finite
subject always has a fair behaviour, so fairness never makes a proof vacuous.
A bounded verdict (TP-2 `on origin`) depends only on a finite prefix, so
fairness cannot change it, and a fairness constraint is admitted on an
infinite-trace clause only.

### 5. Counterexamples as QSL traces

| ID | Rule |
| --- | --- |
| CX-1 | **Shapes.** A safety refutation (TP-1, TP-2, TP-3) is a finite prefix. Under a bounded profile the prefix covers the formula's horizon from every activation it reports, or ends at a terminal state, so the evaluator decides it exactly. Under infinite-trace it is a bad prefix in ADR-014 A-4's sense. A liveness refutation (TP-4) is a lasso: a prefix, possibly empty, and a non-empty loop that re-enters at its first position. A behaviour that ends at a terminal state is the lasso whose loop is the one terminal stutter step (SM-4). A deadlock refutation is a finite prefix ending at the deadlocked state (DL-4). |
| CX-2 | **Content.** A counterexample is ADR-014's `TemporalCounterexample{prefix, loop, fairness, interval}` with its steps over a model subject: the index of the initial state in the subject, then each step as its FR-181 transition identity and its post-state's `quire.simulation.state-key/v1` digest, and the binding of the clause's `over` parameter (the object the refutation is about). It travels in `WitnessEnvelope<TemporalCounterexample>` with `trace_position` naming the failing position (ADR-014 TR-2) and a new `ReplaySource::ModelTrace` source arm. The envelope's obligation identity binds the subject. |
| CX-3 | **Replay.** E9 replay of a model counterexample recompiles the package (FR-098), re-admits the subject's initial state and universes from the byte provision, re-executes each transition through `ModelSystem` with FR-101 `replay`, and refuses with `stale_dependency`/`revision-mismatch` when a recomputed post-state digest differs, or `invalid_runtime_input`/`invalid-value` when the loop's last post-state is not its entry state or a transition is not enabled. It then checks the lasso against the fairness set (an unfair lasso refuses, ADR-014 A-4) and evaluates the formula over it with SM-1. Agreement settles `reproduced-with-evaluated-witness` and the item `refuted`; disagreement settles `inconclusive`, `ReplayParity`. A deadlock counterexample replays by DL-5 in place of the formula evaluation. |
| CX-4 | **Engine independence.** Both engines produce the same counterexample type. EN-2 reads the transition identity of each step from selector variables in its encoding and names the initial state by index, so its counterexample replays through CX-3 with no solver present. |

**What exists and what is new.** `qsl_replay::replay`, `WitnessEnvelope`,
`TracePosition`, `ProfileSelection`, the FR-072 settlement and FR-101
`replay` exist. ADR-014 names `TemporalCounterexample` and its replay
refusals, which the temporal spine work is to build. New in this
record: the model-subject step content (CX-2), `ReplaySource::ModelTrace` and
its `ReplayResult` arm, subject inputs in the replay request (initial-state
snapshots in the byte provision, universes), the loop-closure and
enabledness refusals, and the terminal stutter step. A model counterexample is
a backend result and enters E9; a simulation finding stays FR-101 replay
only (ADR-016 §6).

### 6. Worked example: liveness under fairness on ConfigVersion

The subject uses the ConfigVersion domain package from
`examples/config-version` (FR-108): object type `ConfigVersion` with
`versionNumber: Int[0, 1000]` and optional `parent`, population
`config_history`, and operation `attemptUpdate` with frame `modifies
[versionNumber]`, no parameters and a Boolean result.

The shipped unit's post clause `VersionUnchanged` keeps `versionNumber`
fixed, so every transition is a self-loop. The example unit keeps the two
invariants and replaces that post clause with one that advances the version
modulo 3. The fairness clause spelling is illustrative; QSpec's shared
grammar owns it (QS-4).

```text
profile v   = "quire.value.complete/v1" …;
profile inf = "quire.temporal.infinite-trace/v1" …;
model Config = "example/config-version" …;

invariant ParentOrder using v on Config::ConfigVersion at current { … }   // unchanged
invariant NoCycle     using v on Config::ConfigVersion at current { … }   // unchanged
post Cycles using v on Config::ConfigVersion::attemptUpdate {
  self.versionNumber = (pre(self.versionNumber) + 1) mod 3
}

temporal ReachesTwo using inf over (c: Config::ConfigVersion) clock "model-steps" on origin {
  fair weak each Config::ConfigVersion::attemptUpdate;      // illustrative spelling
  always eventually holds(c.versionNumber = 2)
}
```

**Subject.** Universe `config_history = {a, b}`. One initial snapshot: `a`
and `b` both at version 0 with `parent` absent. Write a state as
`(va, vb)` and the transition `attemptUpdate` with receiver `a` as `upd(a)`.
The result value is not in the state key, so both result choices reach the
same successor. The model has 9 reachable states, each with two successors,
`upd(a)` and `upd(b)`, both always enabled.

**Product.** The form is TP-4. For `c = b`, the negation is `eventually
always (b.versionNumber != 2)`, a two-state Büchi automaton: `q0` loops on
any state and moves to accepting `q1` on a state with `vb != 2`; `q1` loops
on such states. The reachable product has 15 states: 9 with `q0` and 6 with
`q1` (`vb` in `{0, 1}`). It has two accepting SCCs, `{(*, 0, q1)}` and
`{(*, 1, q1)}`, three states each, whose internal edges are all `upd(a)`.

**Verdict under `fair weak each`.** `upd(b)` is enabled at every state of
both SCCs and taken in neither, so FA-4 rejects both. No fair accepting cycle
exists for `c = b`; `c = a` is symmetric. The item settles `proved`, basis
`closed-scope`, `TerminalValue::Proved{basis: Exhaustive}` (V-1), over this
subject.

**Verdict under `fair weak whole`.** The constraint is `attemptUpdate` as a
whole, and `upd(a)` inside the SCC satisfies it. The unmarked spelling
`fair weak Config::ConfigVersion::attemptUpdate` is this variant (FA-6); the
clause above writes `each` because its verdict relies on it. The first
accepting SCC in canonical order contains the initial product state
`(0, 0, q1)`, so the stem is empty. The item settles `refuted` (V-4) with this lasso, bound to `c = b`:

| Position | State | Step into the next position |
| --- | --- | --- |
| 0 (loop entry) | `(0, 0)` | `upd(a)` |
| 1 | `(1, 0)` | `upd(a)` |
| 2 | `(2, 0)` | `upd(a)`, back to position 0 |

Replay (CX-3) re-executes the three `upd(a)` steps from the initial snapshot,
checks the last post-state digest equals position 0's, checks the loop takes
`attemptUpdate` (fair under `whole`), and evaluates `always eventually holds(
b.versionNumber = 2)` over the lasso: `vb` is 0 at every position, so the
formula is false at position 0, `trace_position` `0`. Under `each` the same
lasso is unfair (`upd(b)` is enabled throughout and never taken) and replay
refuses it.

**The shipped unit.** With `VersionUnchanged` the only reachable state is
`(0, 0)`. Under `fair weak each` the item settles `refuted` with the lasso
whose loop is `(0, 0) -upd(a)-> (0, 0) -upd(b)-> (0, 0)`. Fairness makes
every operation instance run, and the post clause still prevents progress.

**Deadlock freedom.** The unit declares no `terminal` member, so the request
adds the subject's deadlock-freedom item (DL-3). Each of the 9 reachable
states has two successors, so EN-1 settles it `proved`, V-1. The shipped
unit's one state has its two self-loops and settles the same way.

**A bounded variant.** `eventually[0,5] holds(c.versionNumber = 2)` under
event-position false-extension, `on origin`, is TP-2 with `h = 5`. It settles
`refuted` under EN-1 with a six-position finite prefix for `c = b`, such as
`(0,0) (1,0) (2,0) (0,0) (1,0) (2,0)` reached by five `upd(a)` steps, on
which `vb` is never 2. EN-2 unrolled to 5 finds a counterexample of the same
length; with no counterexample at depth 5 it would settle V-2. Fairness is not admitted on it (§4).

### 7. Downstream impact and sequencing

**Stage DAG (ADR-011 §1).** A new stage **S6c Temporal model check** reads
the in-process S4 package and the subject over a new edge **E10 S4 → S6c**,
and outputs one FR-331 terminal record per routed item. A refutation leaves
S6c as an S7 typed witness and reaches S8 through E9, as an S6b
counterexample does. SMT temporal checks run on the existing S5 → E7 → S6b
path; S6b's role widens from Kani to any IR-lowered negotiated backend.
ADR-011 §6.1 layer 5 gains `model_check` after `simulation`; layer 6
`replay` gains the model-trace arm and already depends on layer 5.

**Consequences for each repository.**

| ID | Repository | Change |
| --- | --- | --- |
| DS-1 | QSL | Layer 5 `model_check`: the product `TransitionSystem`, edge retention in the FR-101 engine, the bounded monitor and Büchi translations with interval unrolling (IV-3), the automaton-state limit (IV-6), SCC decomposition and the fairness filter, deadlock detection (DL-7), the canonical counterexample. S3: property form classification, the fairness constraint check with its unmarked granularity (FA-6), interval operators under infinite-trace (IV-1), and the `terminal` member (DL-1). The request writer: the deadlock-freedom item (DL-3). Layer-5 evaluator: interval operators over a lasso (IV-2). `qsl-replay`: `ProofBasis`, the new `InconclusiveCause` variants, `TemporalCounterexample` over a model subject with its `kind` (DL-4), `ReplaySource::ModelTrace` and its result arm with the deadlock check (DL-5). The EN-1 provider manifest. |
| DS-2 | CG | A `negotiate_*` arm for EN-1 and one for the SMT temporal arm, each reading the property form and the subject classification; the deadlock-freedom item goes through the TP-1 path of each. The map from each engine's outcome into `TerminalValue`, extending C-09 for `ProofBasis`. Obligation identity binds the subject. |
| DS-3 | IR | Admission of `state` nodes (ADR-016 PI-5), a transition-relation form for operations with frames and contracts, temporal intake for bounded and infinite-trace forms (ADR-014 §13 item 1) with interval operators inside infinite-trace forms (IV-1), and the SMT encodings of EN-2 and EN-3, including the `deadlocked` predicate (DL-7). |
| DS-4 | Driver | Runs an item routed to EN-1 in process and writes its terminal record; runs E9 for its counterexample. |
| DS-5 | QSpec | §8. |

**Sequencing.** The explicit-state path depends only on QSL work that is
already planned. The SMT path depends on IR and CG work that has not started.

1. **Specification.** This record, then QSpec (§8), then the QSL compiler
   requirements.
2. **Prerequisites already planned in QSL.** FR-101 findings and stopped
   expansions (ADR-016 G-1); FR-120 `ModelSystem` (ADR-016 G-4); the temporal
   spine migration with infinite-trace admission, the layer-5 evaluator and
   lasso evaluation (ADR-014 §11); temporal atoms over state types. With this
   record these are on the critical path for model checking, and the temporal
   spine migration gains an evaluator interface the product can drive.
3. **EN-1.** `model_check`, the EN-1 provider and CG arm, the model-trace
   replay arm, and the worked example as its first conformance vector.
4. **Language features built on EN-1**, in parallel once step 3 lands:
   - *Strong fairness* adds a `FairnessKind` and its SCC refinement (FA-5).
     It needs nothing else from this record.
   - *EF and AG EF* are queries over the retained graph (backward
     reachability from the target states). Their refutation of AG EF is a
     prefix to a state from which the target is unreachable. The prefix
     replays through CX-3; the unreachability half is a `closed-scope` fact
     over the retained graph, and its witness shape is that ticket's
     decision.
   - *State-space reduction* changes which product states EN-1 stores.
     Symmetry over universe keys keeps the verdict. Partial-order reduction
     keeps it for formulas with no interval operator; a formula with one
     counts positions (IV-7) and is checked on the unreduced graph. The
     reduction applied enters the result's method, and the
     canonical counterexample is defined over the reduced graph.
   - *Refinement mappings* check an abstract model's temporal formula,
     with its fairness, over the concrete subject's behaviours through a state
     mapping (ADR-017 §3's relation). It reuses SM-5, EN-1 and the lasso, and
     it needs stuttering steps in the abstract behaviour, which SM-4's stutter
     step anticipates for terminal states only.
   - *Hyperproperties* check a formula over pairs of behaviours by
     self-composition: the subject is the product of the model with a copy of
     itself, and the counterexample carries two traces, which extends CX-2.
5. **EN-2 and EN-3.** After IR admits state nodes and temporal forms and the
   SMT backend exists. They add V-2, V-3 and faster refutation; they change no
   verdict EN-1 gives.

Lowering a QSL model and property to TLA+ or Quint, so that TLC or Apalache
can check them, is later research and outside this record.

### 8. What QSpec must specify

| ID | Item | Where |
| --- | --- | --- |
| QS-1 | The model subject of a temporal claim (SM-2, SM-3): initial states, universes, behaviours as maximal paths, position 0 as the initialization observation, position `i` as the post-state of step `i` with its operation anchor | an amendment to QSpec FR-161's "transition/model subject" and a new temporal FR |
| QS-2 | Terminal states (SM-4): closed finite execution under bounded profiles, terminal stutter extension under infinite-trace, the stutter step's identity and its exclusion from fairness; intended terminal states and deadlocks are QS-12 | the same FR; QSpec FR-181 |
| QS-3 | Profiles admitted over a model subject: event-position false-extension and infinite-trace; the model step sequence as their sequence authority and its clock binding | QSpec FR-090, FR-250 |
| QS-4 | Fairness: surface syntax in the shared grammar, `whole` and `each` granularity with `whole` unmarked (FA-6), enabledness (FA-2), weak fairness (FA-3), an extensible fairness kind, admission on infinite-trace clauses only; and what QSpec FR-161 and FR-341 mean by a "missing fairness premise" | shared grammar; QSpec FR-161 |
| QS-5 | Binding of a temporal clause's `over` parameter to a model population: one instance per object of the universe, and the reading at positions where the object does not exist | QSpec FR-161 or the new FR |
| QS-6 | Verdict strength: the V-1 to V-8 table onto QSpec FR-341 (infinite-trace) and FR-243, the method and depth in the FR-331 terminal record, the four new inconclusive causes, the definition of counterexample length and of V-5 | QSpec FR-161, FR-341 (infinite-trace), FR-331 |
| QS-7 | Bounded MLTL over a model (TP-2): `on origin` truth depends only on the first `h + 1` positions; `on each` holds at every activating position | QSpec FR-091, FR-092 or the new FR |
| QS-8 | Counterexample wire: initial-state index, transition identities, post-state digests, loop entry, terminal stutter marker, `over` binding, fairness set; the replay rules of CX-3 | QSpec FR-331 and the counterexample contract |
| QS-9 | The FR-331 request's subject members (initial-state snapshot references, universes) and the subject's place in the obligation identity; what an advertisement of (`temporal-satisfaction`, `unbounded`) commits to, given that V-5 is a valid result; reconciling QSpec FR-161-AC-7's "liveness backend" wording with the FR-290 mode table | QSpec FR-290, FR-331, FR-161 |
| QS-10 | Conformance vectors, each with expected verdict kind and, for a refutation, a counterexample that must replay (validity is checked; the exact counterexample is not, since engines may return different valid ones): (a) the QSpec TC-200 operator lassos with fair and unfair variants; (b) the §6 worked example, its three verdicts and the bounded variant; (c) one model with an intended terminal state (DL-1) read under event-position false-extension and under infinite-trace, with different verdicts; (d) a bound-reached case (V-5) and an undecided-successor case (V-6); (e) replay refusals: a loop that does not close, an unfair lasso, a post-state digest mismatch, a transition not enabled; (f) deadlocks: a model whose deadlock-freedom item settles `refuted` with a prefix that replays, the same model with `terminal when` covering that state, settling `proved`, and with `terminal any`, which adds no item; (g) interval operators under infinite-trace: the recovery-stability formula of §11 over lassos that satisfy and violate it, `always (req implies eventually[0,5] ack)` classified TP-3, and a case that reaches `max_automaton_states` and settles V-7 | QSpec TC-200 and new TCs |
| QS-11 | v2 temporal operation identities for bounded and infinite-trace forms, needed by the SMT path only | the open item in ADR-014 §13 |
| QS-12 | Deadlocks (§10): the `terminal` member's syntax and meaning (DL-1), the deadlock definition (DL-2), the deadlock-freedom item, its identity and its addition to every request with a model subject unless the model opts out (DL-3), the counterexample `kind` member (DL-4) and its replay (DL-5) | QSpec FR-181, FR-331, the new temporal FR, the shared grammar |
| QS-13 | Interval operators under infinite-trace (§11): closed intervals on every interval-capable operator under the profile (IV-1), their meaning over an infinite trace (IV-2), the safety fragment with interval operators (IV-4), and the automaton-state limit with its V-7 settlement (IV-6). This replaces the rule that the profile admits no interval bound | QSpec FR-090, FR-250-AC-6, FR-255, FR-091 and FR-092 (their infinite-trace paragraphs), FR-161, the shared grammar note on `[a,*]` |

### 9. Rulings on the draft's questions

The owner ruled on the four questions the draft left open, on 2026-10-01.

| ID | Question | Ruling | Rationale | Where it lands |
| --- | --- | --- | --- | --- |
| RU-1 | Which engine comes first | **Explicit-state first.** EN-1 is built first; EN-2 and EN-3 follow IR state-node admission and the SMT backend | EN-1 depends only on QSL work already planned. QSL has no SMT code; the planned SMT backend is in the contract IR lane, and its temporal use also needs IR to admit state nodes. EN-1 alone gives `proved` and `refuted` for safety and liveness over a finite subject; EN-2 and EN-3 add V-2, V-3 and faster refutation and change no EN-1 verdict | §3, §7 sequencing steps 3 and 5 |
| RU-2 | The unmarked fairness granularity | **`whole`.** `each` is always written | `whole` is the weaker premise: it excludes fewer behaviours, so an unmarked constraint assumes the least, and a claim proved under it also holds under `each`. It is TLA+'s `WF` over the existentially quantified action, the reading TLA+ authors expect. A clause that relies on per-identity fairness says so in its text, its obligation identity and its counterexample | FA-6; §6 writes `each` on the variant it proves with `each` |
| RU-3 | Whether EN-1 reports reachable terminal states | **Deadlocks are reported by default, as TLC does, with a per-model opt-out.** A model marks its intended terminal states | A successor-free state that the author did not intend is usually a missing operation or a precondition that is too strong. Stutter extension alone absorbs it silently: every safety claim holds on the stuck tail, and only a claim about progress notices. Reporting it once per subject, as its own item, leaves every claim's verdict equal to its SM-1 truth | §10, DL-1 to DL-7 |
| RU-4 | Whether interval operators may appear inside infinite-trace formulas | **Mixed formulas are admitted**, for example `always (fail implies eventually always[0,10] healthy)` | That formula states recovery stability: after every failure the system eventually stays healthy for ten consecutive steps. A bounded `on each` clause states a bounded response at each activation and cannot put a bounded operator under an unbounded `eventually`; its false-extension also reads a terminal state differently from stutter extension. Mixed formulas cost automaton size that grows with interval length, which §11 states and limits | §11, IV-1 to IV-7; ADR-014 TR-3, A-2 and A-4, amended with this record |

### 10. Deadlocks

| ID | Rule |
| --- | --- |
| DL-1 | **Intended terminal states.** A state model declares its intended terminal states with at most one `terminal` member, in one of two forms. `terminal when P`, where `P` is a state predicate evaluated through the one clause evaluator (ADR-016 FE-3) over the state's observation: a terminal state (SM-4) that satisfies `P` is intended. `terminal any`: every terminal state is intended; this is the per-model opt-out. A model with no `terminal` member intends no terminal state. `P` classifies terminal states only; a state that satisfies it and has a successor is not terminal. The spelling is illustrative; QSpec's shared grammar owns it (QS-12). The member is part of the checked package, so it is part of the subject and its obligation identity (§1 Scope). |
| DL-2 | **Deadlock.** A deadlock is a reachable state of the subject that is terminal (SM-4) and not intended (DL-1). Enabledness is FA-2's, so a state is terminal exactly when no transition identity is enabled at it. The definition reads the state graph alone; fairness plays no part in it. |
| DL-3 | **Reported by default.** For each distinct model subject among a request's temporal items, the request writer (ADR-013 O-20) adds one **deadlock-freedom item**, unless the subject's state model declares `terminal any`. The item is a TP-1 invariant `always holds(not deadlocked)`, where `deadlocked` is DL-2's predicate, derived and not authored. Its obligation identity is the subject and the fixed item kind `deadlock-freedom` (ADR-013 O-09). It is recorded, negotiated and settled as a TP-1 item, with the verdict kinds of §1: V-1 when exploration completes with no deadlock, V-3 by EN-3, V-4 with a deadlock counterexample, and V-5, V-6 and V-7 as for any TP-1 item. |
| DL-4 | **Verdict and counterexample.** A reachable deadlock settles the deadlock-freedom item `refuted`, basis `decisive-counterexample`, `TerminalValue::Refuted` (V-4). Its counterexample is a CX-1 finite prefix from an initial state to the deadlocked state. `TemporalCounterexample` carries a `kind` member, `Formula` for every counterexample of §5 and `Deadlock` for this one. The item kind and the counterexample kind state the cause; no label, basis or `TerminalValue` variant is added. EN-1's canonical deadlock counterexample is the canonical path to the first deadlocked state in FR-181 canonical breadth-first order. |
| DL-5 | **Replay.** CX-3 replays the prefix unchanged. At its last state it then enumerates the transition identities through `ModelSystem` and evaluates `P` when the model declares `terminal when P`. No enabled identity and `P` false settles `reproduced-with-evaluated-witness` and the item `refuted`. An enabled identity, or `P` true, is a disagreement with the engine and settles `inconclusive`, `ReplayParity`. |
| DL-6 | **Claims read every terminal state the same way.** A deadlock changes no authored claim's verdict: every terminal state, intended or not, reads by SM-4, closed under a bounded profile and stutter-extended under infinite-trace. Under infinite-trace, a behaviour that deadlocks before `p` refutes `eventually holds(p)` with the stutter lasso (CX-1), and the deadlock-freedom item reports the same state as a deadlock; in a model that opts out, only the claim reports it. `terminal when P` changes only the deadlock-freedom item. |
| DL-7 | **Engines.** EN-1 decides the deadlock-freedom item in its first phase: a model state whose FR-120 expansion yields no successor and whose observation does not satisfy `P` is a deadlock. An expansion with an undecided contract conjunction settles V-6 `UndecidedSuccessor`, as for any item. EN-2 and EN-3 encode `deadlocked` as the negation of FA-2 enabledness over every transition identity, conjoined with the negation of `P`. |

### 11. Interval operators under infinite-trace

| ID | Rule |
| --- | --- |
| IV-1 | **Admission.** Under `quire.temporal.infinite-trace/v1`, every temporal operator that carries an interval under a bounded profile (`eventually`, `always`, `until`, `release`, `once`, `historically`, `since`, `triggered`) admits a closed interval `[a,b]` with `a <= b`, or none. With an interval it is an **interval operator**; without one it is an unbounded operator. The two nest in any order, so `always (fail implies eventually always[0,10] healthy)` is admitted. `[a,*]` and `a > b` refuse as ADR-014 TR-3 states for bounded profiles. The checked operator holds `Some(TemporalInterval)` for an interval operator and `None` for an unbounded one. The interval's key carries the infinite-trace profile identity (QSpec FR-255), so it never equals a bounded-profile interval. |
| IV-2 | **Meaning.** An interval operator has QSpec FR-091's offset meaning (future operators) or FR-092's (past operators), read over the infinite trace. Every future position exists, so no closed-boundary rule applies. Position 0 is the behaviour's authoritative origin (SM-3), so a past interval that reaches before position 0 reads atomic predicates there as false, FR-092's complete-history rule. Over a lasso, a position past the represented trace is the loop position it wraps to, under ADR-014 TR-2's prefix-then-loop indexing. Evaluation charges TR-5 work per (node, position) visit. |
| IV-3 | **Translation.** The infinite-trace translation (SM-6) expands each interval operator into nested next-position steps before it builds the generalized Büchi automaton. With `X` the next-position step and `X^d` `d` of them: `eventually[a,b] p` becomes `X^a (p or X p or … or X^(b-a) p)`; `always[a,b] p` becomes `X^a (p and X p and … and X^(b-a) p)`; `p until[a,b] q` becomes `X^a` applied to the disjunction over `d` in `[0, b-a]` of `X^d q` conjoined with `X^e p` for every `e < d`, which is FR-091's lower-bound convention; `release[a,b]` is its dual. A past interval operator expands into nested previous-position steps, tracked as monitor state as SM-6 tracks past subformulas. `X` exists only inside the translation; the authored grammar has no next-position operator. The translation answers to SM-1 (SM-7). |
| IV-4 | **Safety fragment.** ADR-014 A-4's safety fragment admits every interval operator, in negation normal form: the truth of an interval operator at a position is decided within `b` positions of it. A formula in the fragment is TP-3, so `always (req implies eventually[0,5] ack)` is TP-3. A formula with an interval operator under an unbounded `eventually` or `until` is TP-4, so the recovery-stability formula is TP-4. |
| IV-5 | **Cost.** *Size.* An interval operator with upper bound `b` unrolls to `b + 1` positions: an `a`-step shift and `b - a + 1` copies of its operand. Nested interval operators multiply, so a chain with upper bounds `b1 … bn` unrolls to `O(n × (b1 + 1) × … × (bn + 1))` for a formula of `n` nodes. *States.* The automaton records, for each interval operator, which of its obligations are still open over its window. For an operator with lower bound 0 whose operand has no interval operator, the open obligations reduce to one counter with `b + 1` values (the earliest open deadline for `eventually` and `until`, the furthest required position for `always` and `release`), so its factor is `f = b + 1`, linear in the bound. A nonzero lower bound keeps the activations of up to `b + 1` positions as a set, factor at most `f = 2^(b+1)`. Nested interval operators multiply their factors. The automaton has at most `c × f1 × … × fn` states, with `c` the size the unbounded operators alone give, and the EN-1 product has at most the reachable model states times that. EN-1 explores reachable product pairs only and materializes automaton states as it reaches them, so the cost paid is the reachable part. *Example.* The negation of the recovery-stability formula is `eventually (fail and always eventually[0,10] not healthy)`. Its automaton has 12 states: one before the failure and a counter of positions since the last unhealthy state, 0 to 10. With bound `b` it has `b + 2`. EN-2 encodes an interval operator over its `b + 1` positions at each unrolled step, so its encoding grows linearly in `b` per operator. |
| IV-6 | **Limit.** No syntactic cap on interval length applies beyond TR-3's `u64` bounds. EN-1's limits are FR-101's `Limits` plus `max_automaton_states`, an ADR-014 B-2 run limit with a published default of 2^20 (1,048,576) automaton states, which a request sets like the other limits. EN-1 counts distinct automaton states as the translation materializes them during product exploration, with checked arithmetic. Reaching the limit stops the run and settles V-7: QSpec FR-341 (infinite-trace) `failed`, execution `resource-incomplete`, basis `unavailable`, `Incomplete(ResourceExhausted)` naming `max_automaton_states`. V-7 is the stopped-run verdict of §1; `inconclusive` (V-5, V-6) is kept for a run that completed its method. Either way the result is non-Boolean, and the caller raises the limit or narrows the interval and reruns. |
| IV-7 | **Position counting.** An interval operator counts positions, so a formula that has one is not stutter-invariant: inserting a step that repeats a state can change its truth. A reduction or a transfer that relies on stutter invariance applies only to formulas with no interval operator (§7 item 4). |

## Consequences

- A temporal claim over a finite model subject gets a `proved` or `refuted`
  verdict from QSL's own explicit-state engine, safety and liveness, with
  weak fairness. No IR or CG engine work is needed for that, only CG's
  negotiation arm.
- Every verdict states its strength through QSpec's existing labels and
  settlement bases plus `ProofBasis`. Bounded-to-`k` is `inconclusive`, so a
  depth-limited search never reads as proof.
- Replay, monitors and the engines share one semantics, the trace evaluator,
  and every refutation is reproduced by it before it counts.
- Counterexamples are ordinary QSL traces over the model and replay without
  the engine that found them.
- The explicit-state path makes the temporal spine migration and FR-120
  `ModelSystem` the critical path for the five language-feature tickets that
  follow, which are then independent of each other and of the SMT path.
- The retained product graph costs memory proportional to the reachable
  product. State-space reduction is the place that cost is addressed.
- Every request with a model subject reports that subject's reachable
  deadlocks unless the model opts out, and every claim's verdict stays its
  SM-1 truth over stutter-extended terminal states.
- Infinite-trace formulas can nest bounded operators under unbounded ones.
  Their automaton grows with interval length, linearly per operator in the
  common lower-bound-0 shapes, and `max_automaton_states` turns an oversized
  translation into a stopped run rather than a refusal.

## Amendments made with this record

- ADR-014 §3 TR-3: under infinite-trace an operator holds a closed interval
  or none (IV-1). §2 B-1 table, row "Temporal interval under
  `quire.temporal.infinite-trace/v1`": optional, refused only as `[a,*]` or
  `a > b`. §5 A-2: the infinite-trace admission admits closed intervals.
  §5 A-4: the safety fragment admits interval operators (IV-4).

## Amendments to make on acceptance

- ADR-011 §1 and §2.1: S6c and E10; S6b's output role covers any negotiated
  IR-lowered backend. §6.1: layer 5 `model_check` after `simulation`.
- ADR-013 O-16 proof column and the `TerminalValue` row: `ProofBasis` on
  `Proved`, the four new `InconclusiveCause` variants and their categories.
- ADR-014 §1 B-5 "When reached": a completed depth settles `inconclusive`
  `BoundReached`; a run stopped before its depth settles `incomplete`. §3 TR-8
  and §10 scenario 1: EN-1 is a registered liveness backend. §5 A-4: lasso
  evaluation is the semantics every engine answers to (SM-1).
- ADR-016 §6: its rules hold for `explore` and `sample`; a model-check result
  reaches proof accounting through negotiation as §3 of this record states,
  and a model counterexample enters E9 (§5).
- `spec/spec.md`: index row.

## Alternatives Considered

- **Nested depth-first search for acceptance cycles.** Rejected. It handles
  one acceptance set, so weak fairness needs a degeneralization step, and it
  gives strong fairness no place to refine. SCC decomposition over the
  retained graph handles both and serves EF and AG EF.
- **On-the-fly emptiness during exploration.** Deferred to state-space
  reduction. Two phases keep the canonical counterexample simple to define
  and keep FR-101's engine unchanged apart from edge retention.
- **Bounded-to-`k` as a `proved` result qualified by `k`, through a new
  `FiniteBound` on the formula domain.** Rejected. QSpec FR-161 forbids proof
  from finite prefixes, and V-5 states the same strength as `inconclusive`
  with its depth, with no change to ADR-014 §4.
- **A separate verdict vocabulary for model checking** (exhaustive,
  bounded, inductive as new labels). Rejected. QSpec FR-341 and FR-243
  already carry the distinction; `ProofBasis` adds only the method and depth.
- **Universes as `ProofBound`s for EN-1.** Rejected, as in ADR-016 §2. A
  universe is part of the subject and enters the obligation identity as such.
- **Deadlock as a separate refusal instead of a stutter extension.**
  Rejected. The stutter extension keeps every behaviour infinite under
  infinite-trace, so "the system gets stuck before `p`" is an ordinary
  liveness refutation.
- **Deadlock as a conjunct of every claim over the subject.** Rejected. A
  claim would settle `refuted` on a behaviour where SM-1 says its formula is
  true, which breaks SM-7, and a TP-2 `on origin` claim could no longer settle
  V-2 at its horizon, because deadlock freedom is unbounded. One derived item
  per subject reports the deadlock once (DL-3).
- **Deadlock reporting as an exploration finding.** Rejected. A finding
  never reaches proof accounting (ADR-016 §6), so a model checked as
  deadlock-free would carry no verdict saying so.
- **A per-model opt-out with no terminal predicate.** Rejected. A model that
  halts on purpose in some states and can still get stuck in others would
  lose the report entirely; `terminal when P` keeps it.
- **`each` as the unmarked granularity.** Rejected. It is the stronger
  premise, so an unmarked constraint would assume more than TLA+'s `WF` and
  could prove a claim that holds only because every receiver and argument
  vector is served (RU-2).
- **Bounded `on each` clauses as the only form of bounded response under
  infinite-trace.** Rejected. They cannot put a bounded operator under an
  unbounded one (RU-4).
- **A syntactic cap on interval length.** Rejected. Automaton size depends
  on the formula's shape as well as its bounds (IV-5); a ceiling on the
  states actually materialized stops only the runs that are too large.
- **Model checking inside the simulation outcome.** Rejected. ADR-016 §6
  keeps exploration out of proof; the model checker is a negotiated backend
  with its own identity and result map.

## References

- Owning ticket: Linear QSL-366. Its QSpec half, QS-1 to QS-13, is Linear
  STD-131. The language-feature tickets that build on
  this record: QSL-365 (strong fairness), QSL-367 (refinement mappings),
  QSL-368 (state-space reduction), QSL-369 (EF and AG EF), QSL-370
  (hyperproperties).
- TL-256: the tl-mltl infinite-trace acceptance criteria. QSpec FR-251 gives
  infinite-trace requests no TL target, so TL is not an engine here; its
  conformance lassos are candidates for QS-10 (a).
- QSpec V1-TEMP-021 to V1-TEMP-026: infinite-trace capability inventory rows.
  V1-TEMP-026's `unsupported` settlement holds until EN-1's provider
  registers.
- The planned SMT backend and its SMT/runtime parity corpus are in the
  contract IR lane (Linear IR-33 and its predecessor).
- Later research on lowering to TLA+ and Quint: RES-40, RES-42, RES-49,
  RES-50.
