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
  - target: ix://agent-ix/quire-specification/FR-360
    type: depends_on
---
# ADR-018: Temporal properties over every behaviour of a model

## Status

Proposed, 2026-10-01. §9 records the owner's rulings on the four questions
this record first left open, on undefined evaluation (RU-5), on uncertified
proofs (RU-6) and on undefined model-side expressions (RU-7); §10 and §11
carry the two that add design. The
QSL compiler requirements that implement it are FR-123 to FR-128 (use case
US-015). The owning ticket and related work are listed under References.

"QSpec FR-nnn" names a quire-specification requirement; a bare FR id is a QSL
requirement. QSpec FR-360 is the infinite-trace result disposition
vocabulary. Item ids `TP-`, `V-`, `SM-`, `EN-`, `FA-`, `CX-`, `DS-`,
`QS-`, `RU-`, `DL-`, `IV-` and `UE-` are local to this record. Other artifacts cite
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
QS-13 change. QSpec FR-360 maps `proved`, `refuted`, `inconclusive`, `unsupported` and
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

**Verdict kinds.** Each verdict is one QSpec FR-360 label
with one QSpec FR-243 basis, carried by an existing `TerminalValue` variant.
The strength of a proof is the basis plus a `ProofBasis` member that this
record adds to `TerminalValue::Proved`. No new result axis, category or label
is added.

| ID | Verdict kind | Meaning | QSpec FR-360 label | QSpec FR-243 basis | `TerminalValue` | O-16 category |
| --- | --- | --- | --- | --- | --- | --- |
| V-1 | Exhaustive | Every admitted behaviour of the subject was examined | `proved` | `closed-scope` | `Proved{basis: Exhaustive, certification}`: `Certified` once the PC-3 or PC-4 check accepts EN-1's certificate | success |
| V-2 | Bounded-complete to `k` | Unrolled to depth `k` at or past the formula's horizon (TP-2 `on origin` only), so every behaviour is decided | `proved` | `closed-scope` | `Proved{basis: BoundedComplete{depth: k}, certification: Uncertified}` (PC-1) | success |
| V-3 | Proved by induction at `k` | The property is `k`-inductive: it holds in the first `k` steps of every behaviour, and any `k` consecutive states of the declared types that satisfy it have only satisfying successors | `proved` | `decisive-witness` | `Proved{basis: Inductive{depth: k}, certification: Uncertified}` (PC-1) | success |
| V-4 | Refuted | A counterexample (§5) reproduced by replay, including one that ends where the claim evaluated undefined (UE-1) | `refuted` | `decisive-counterexample` | `Refuted` | violation |
| V-5 | Bounded-to-`k` | The engine completed its analysis to depth `k` and found no counterexample of length at most `k` | `inconclusive` | `unsettled` | `Inconclusive(BoundReached{depth: k})` | inconclusive |
| V-6 | Inconclusive, other cause | `InductionNotClosed{depth: k}`: base case holds to `k`, the step case has a counterexample-to-induction. `UndecidedSuccessor`: a contract conjunction was refused during expansion, with no clause false and none undefined, so the successor relation is not exactly known (FR-120 `ContractUndetermined` with a `Refused` evaluation); an undefined one refutes (UE-6). `CertificateRejected{rule, state}`: the core checker rejected EN-1's proof certificate (PC-2). `NoInitialState`: the subject has no initial state (ADR-016 EX-10). `ReplayParity` and `ReplayRefused`: as ADR-013 C-09 | `inconclusive` | `unsettled` | `Inconclusive(cause)` | inconclusive |
| V-7 | Stopped | A run limit stopped the analysis before it completed its method: state or transition limit, automaton-state limit (IV-6), meter, time, cancellation | `failed`, execution `resource-incomplete` | `unavailable` | `Incomplete(cause)` | incomplete |
| V-8 | Unsupported | No candidate settles the form; or a profile the model subject does not admit | `unsupported` | `unavailable` | `Unsupported(cause)` | unsupported |

`ProofBasis` is `Checks{success_checks}` (Kani, unchanged in meaning: zero
is the vacuous proof), `Exhaustive`, `BoundedComplete{depth}` and
`Inductive{depth}`. `TerminalValue::category` maps `Checks{0}` to
inconclusive with `KaniVacuousProof`, as today, and every other basis to
success. `InconclusiveCause` gains `BoundReached{depth}`,
`InductionNotClosed{depth}`, `UndecidedSuccessor`, `NoInitialState` and
`CertificateRejected{rule, state}`. `TerminalValue::Proved` carries
`certification: Certification::{Certified, Uncertified}` beside its basis
(PC-1).

**Proof certificates.** A `proved` from EN-1 counts only once a checker in
the qualified core accepts the certificate EN-1 returns with it (LA-3). A
checker recompiles the package, re-admits the subject and recomputes with
its own code; it reads nothing from the engine's search but the
certificate, so the engine's soundness does not decide the verdict.

| ID | Rule |
| --- | --- |
| PC-1 | **Certification label.** `Certified`: a core checker accepted the proof's certificate. `Uncertified`: the proof's engine has no core certificate checker; today that is every `Checks` (Kani), `BoundedComplete` and `Inductive` (SMT) proof. An `Uncertified` proof is `proved`, with its basis and O-16 category success; it is never `inconclusive`. The label is a member of the FR-331 terminal record (QS-6). |
| PC-2 | **Rejected certificate.** A certificate the checker rejects settles the item `inconclusive`, basis `unsettled`, `Inconclusive(CertificateRejected{rule, state})`, naming the check rule that failed and the product state it failed at. It never settles `proved`. |
| PC-3 | **Closure certificate (safety).** For a TP-1, TP-2 or TP-3 item and the deadlock-freedom item, EN-1 returns `ClosureCertificate{states}`: the product states it explored, each as its model state's `quire.simulation.state-key/v1` digest and its automaton state's canonical key (the translation's canonical encoding of the automaton state, independent of the order states were materialized). The check: (a) every initial product state is in `states`; (b) from the initial product states, the checker expands each reached state through `ModelSystem`, the property automaton and the terminal rules (SM-4), and every successor product state is in `states`, so the reached set is closed under the transitions and contains every reachable product state; (c) no reached state is bad: its monitor state rejects (a TP-2 closure at a terminal state included), it is deadlocked (DL-2), its letter is undefined (UE-1), or its expansion evaluates a contract conjunction undefined (UE-6) or refused (V-6). The checker expands only states in the certificate, so its work is bounded by the certificate's size. |
| PC-4 | **Component certificate (liveness).** For a TP-4 item, EN-1 returns `ComponentCertificate{closure, components}`: a PC-3 closure and a partition of its states into components listed in topological order, each with a witness: `Trivial` (one state with no edge to itself), `MissingAcceptance{set}` (no state of the component lies in that acceptance set of the generalized Büchi automaton), or `UnfairWeak{constraint}` (a weak constraint of the clause's fairness set enabled at every state of the component and taken by no edge with both ends in it). The check: (a) PC-3 (a) and (b), and PC-3 (c) without the monitor rule; (b) the components partition the reached states; (c) every edge the checker computes goes from a component to itself or to a later one; (d) every component's witness holds. By (c) every cycle lies inside one component, so no fair accepting cycle exists. EN-1 builds the certificate from its second-phase SCC decomposition, listing each SCC with the first witness that holds for it. |
| PC-5 | **Where the checkers run.** The checkers are layer-6 entries in `qsl-replay` beside `replay_model_trace`, inside the qualified core (LA-3). They use `qsl-eval`'s `ModelSystem` and property-automaton translation (LA-2) and no `qsl-analyze` code. |

**Undefined evaluation.** A claim that evaluates undefined on an admitted
behaviour is refuted. This is the well-definedness reading of Dafny and
TLA+: a claim that is not defined on some behaviour does not hold for the
subject.

| ID | Rule |
| --- | --- |
| UE-1 | **Rule.** At each position a claim reads, it reads a *letter*: the value of every atom of the claim there, a `holds` atom through the one clause evaluator (ADR-016 FE-3), and for the deadlock-freedom item DL-1's `P` at a terminal state. A TP-2 `on origin` claim reads positions 0 to `h`; every other form reads every position of the behaviour. A position that a bounded profile's closed-boundary rule supplies past a terminal state is read by that rule and evaluates no atom. When an atom evaluates `Undefined` (the kernel `Outcome::Undefined`, or a family's `FamilyResult::Undefined`) at a position the claim reads on an admitted behaviour, the claim evaluates undefined there, and the item settles `refuted`, basis `decisive-counterexample`, `TerminalValue::Refuted` (V-4), with cause `UndefinedEvaluation{where, cause}`. |
| UE-2 | **Cause.** `where` is QSpec FR-363's site of the undefined evaluation; QSL records it as FR-363 states and keeps no field list of its own. Its position is the counterexample's `trace_position` (ADR-014 TR-2). `cause` is the evaluator's undefined cause as its `UndefinedRecord` (ADR-013 O-16), taken from the first atom in clause-node order that evaluated undefined at `where`. `UndefinedEvaluation{where, cause}` is a `TemporalCounterexample` `kind` (DL-4), so the refuted record carries it with its counterexample. No label, basis, `TerminalValue` variant or O-16 category is added: ADR-013 O-16 and QSpec FR-360 are unchanged. |
| UE-3 | **Counterexample.** The counterexample is a finite prefix from an initial state to the position `where` (CX-1), for every form, safety and liveness alike. Weak fairness is machine-closed (§4), so the prefix extends to a fair behaviour, and the claim is undefined on an admitted behaviour. |
| UE-4 | **First refuting evidence.** An engine reports the first refuting evidence in its canonical order, whether that is an undefined evaluation or a violation. EN-1 reads a position's letter when it creates the product state for that position, since the automaton steps on the letter. Its first phase ends at the first product state, in FR-181 canonical breadth-first order, whose letter is undefined or whose monitor state rejects: an undefined position ranks like any other violation, the shortest prefix first and canonical transition order after that. A TP-4 item's second phase runs only when the first phase found no undefined evaluation. With `max_depth = k`, EN-1 creates product states up to depth `k`, so V-5 still means no counterexample of length at most `k`. EN-2 and EN-3 conjoin the definedness of every atom the claim reads to the property they encode, so an undefined evaluation is a violation to them as well. |
| UE-5 | **Replay.** CX-3 replays the prefix unchanged, then evaluates the letters of the replayed positions in order. When the first position whose letter is undefined is `where`, with an undefined cause equal to the payload's `cause`, replay settles `reproduced-with-evaluated-witness` and the item `refuted`: reproducing the undefined value at `where` is what reproducing this counterexample means. A defined letter at `where`, an undefined letter before it, a `where` past the replayed positions, or a different cause is a disagreement and settles `inconclusive`, `ReplayParity`. No fairness check applies (UE-3). |
| UE-6 | **Model-side expressions.** An undefined model-side expression at a reachable state refutes the item as an undefined claim does: a contract conjunction that evaluates undefined during expansion (FR-120 `ContractUndetermined` with an `Undefined` evaluation, an undefined precondition guard included) and DL-1's `terminal when P` that evaluates undefined at a terminal state settle `refuted`, cause `UndefinedEvaluation{where, cause}`, with a prefix to that state; it is never read as a disabled step. Replay re-executes the prefix and reproduces it when it evaluates the same expression undefined at `where` with an equal cause. A contract conjunction that is refused, with no clause false and none undefined, leaves the successor relation unknown and settles V-6 `UndecidedSuccessor`. Every engine that settles a claim over a model subject, a tuple of behaviours or a sample applies UE-1 to UE-5 with its own site, which its record names. |

**Length.** The length of a counterexample is its number of transitions,
prefix and loop together. "No counterexample of length at most `k`" is the
one definition of V-5 for every engine.

**Depth is a method parameter.** The unrolling or exploration depth `k` is
the search horizon of a bounded search, not a nesting depth or a modelling
limit; EN-1 takes it as `max_depth`, and every verdict that depends on it
states the `k` it used. It travels with EN-1's run limits, which are B-5
budgets of QSL's own provider with published defaults (IV-6). A run that completes every depth up to `k` has
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
| SM-5 | **Model verdict.** A formula holds for a subject exactly when SM-1 returns true on every admitted behaviour: every behaviour under a bounded profile; every fair behaviour (§4) under infinite-trace. A refutation is one admitted behaviour on which SM-1 returns false, or on which the claim evaluates undefined (UE-1). |
| SM-6 | **Bounded and unbounded share the definition, not the representation.** ADR-014 TR-3 keeps bounded-profile and infinite-trace operators in distinct representations, and this record keeps that. An interval operator inside an infinite-trace formula is an infinite-trace operator (IV-1), and the translation tracks it by IV-3. Every form reaches the engines through one interface, a property automaton over positions (§3), built by one translation per form. A bounded formula (TP-2) gets a deterministic finite monitor. An infinite-trace safety formula (TP-1, TP-3) and the deadlock-freedom item get a deterministic bad-prefix monitor: the automaton of the formula itself, trimmed to the states from which some infinite run is accepting, then determinized by subset construction, so its one rejecting state, the empty subset, is reached exactly when the prefix read so far is a bad prefix (ADR-014 A-4). A liveness formula (TP-4) gets a generalized Büchi automaton for its negation. Past operators are tracked as automaton state (each past subformula's truth, or its open window, at the previous position), so the automaton stays finite and the product needs no unrolling. |
| SM-7 | **Engine encodings answer to the evaluator.** Every refutation is replayed through SM-1 before it settles `refuted` (§5), as ADR-013 C-09 requires of a Kani counterexample. Every translation is tested against SM-1 on the QSpec conformance lassos (§8). An engine whose verdict disagrees with SM-1 on a replayed counterexample settles `inconclusive`, `ReplayParity`. |
| SM-8 | **Lasso evaluation with past operators.** A lasso has a prefix of `n` positions and a loop of `L` positions, so position `p >= n + L` holds the same model state as position `p - L`. State reads (atoms, the observation and its anchor) wrap: position `p >= n` reads the state of position `n + ((p - n) mod L)`. A subformula's truth wraps only after its past reach is used up. The past reach `R(ψ)` is 0 for an atom; the largest reach of the operands for a Boolean or future operator; `R(ψ) + 1` for a previous-position operator; `R(ψ) + b` for a past interval operator with upper bound `b`, taking the larger operand reach for `since[a,b]` and `triggered[a,b]`; and the largest operand reach plus `L` for an unbounded `once`, `historically`, `since` or `triggered`. Every subformula `ψ` is periodic from `n + R(ψ)`: its truth at `p + L` equals its truth at `p` for every `p >= n + R(ψ)`. The evaluator unrolls the loop `m = ceil(R(φ) / L) + 1` times. It evaluates every subformula over the `n + m × L` unrolled positions, with past operators reading back to position 0 and atomic predicates false before it, and reads a subformula at a position past the unrolled positions at the position of the last unrolled loop copy that is congruent to it modulo `L`. Example: on §6's loop (`n = 0`, `L = 3`), `always eventually (holds(a.versionNumber = 0) and once[1,1] holds(a.versionNumber = 2))` has `R = 1` and `m = 2`. The conjunction is false at position 0, which has no position before it, and true at position 3, whose previous position holds version 2; read past position 5 by congruence, it is true at positions 3 and 6 and every third position after, so the formula is true. EN-1 needs no unrolling, because its automaton state carries the past subformulas (SM-6); SM-8 is how the evaluator, and so replay, read a lasso. |

### 3. Engines

| ID | Engine | Where it lives | Forms and verdicts |
| --- | --- | --- | --- |
| EN-1 | **Explicit-state product.** The product of the subject's state graph with the property automaton, explored by FR-101's canonical breadth-first engine. The product is itself a `TransitionSystem`: its state is (model state, automaton state), its key is (model state key, automaton state index). The engine retains every product edge it explores. A second phase decomposes the retained graph into strongly connected components and finds a fair accepting cycle (§4). A safety form (TP-1, TP-2, TP-3, the deadlock-freedom item) needs only the first phase: reaching the monitor's rejecting state is a violation (SM-6). At a terminal model state a TP-2 monitor is closed by the profile's closed-boundary rule: it reads the remaining positions of its horizon with every atomic predicate false and every constant unchanged (QSpec FR-091), and the item is violated when that closure reaches the rejecting state. Under infinite-trace a terminal model state continues along its stutter edge (SM-4). | QSL layer A, crate `qsl-analyze`, module `model_check` (LA-1) | TP-1 to TP-4. V-1, V-4, V-5 (with `max_depth`), V-6, V-7 |
| EN-2 | **SMT unrolling.** The transition relation encoded over `k` steps, for bounded violations and for lasso-shaped violations with a loop back-edge inside `k`; fairness constraints are constraints on the loop segment. | the SMT backend in the contract IR lane, reached by negotiation | TP-1 to TP-4 refutation (V-4); V-2 for TP-2 `on origin` at `k >= h`; V-5 otherwise |
| EN-3 | **`k`-induction.** Base case: no violation in the first `k` steps. Step case: `k` consecutive states that satisfy the property, with pairwise distinct states, have no violating successor. | the same SMT backend | TP-1, TP-3 and TP-2 `on each`, through the safety monitor product. V-3, V-6 `InductionNotClosed` |

**Acceptance-cycle detection is SCC-based.** EN-1 uses one algorithm:
decompose the retained product graph into SCCs, then test each non-trivial
SCC that holds a state of every acceptance set of the generalized Büchi
automaton against the fairness filter (§4). This handles generalized Büchi
acceptance and weak fairness in one pass over each SCC, and it is the seam
where a new fairness kind adds SCC refinement (FA-5).

**Determinism.** EN-1's verdict is a function of the subject, the clause and
the limits. Its counterexample is canonical: the stem is the first path in
FR-181 canonical breadth-first order to the first accepting SCC that passes
the fairness filter, and the loop is the greedy walk of CX-5 inside that
SCC. SCC enumeration order follows discovery order and
never hash-map iteration (ADR-016 ND-4).

**EN-1 pre-check.** Before any expansion, EN-1 classifies every root of the
subject (FR-120 `domains()` under the request's universes). An unbounded root
returns `requires-bound`, which the caller answers with a universe for a
population root or a `bounded_domain` in the model for a parameter, result or
field root, as ADR-016 §2 states for simulation.

**Explicit-state limits.** EN-1 runs under FR-101's `Limits`
(`max_states`, `max_transitions`) and takes `max_depth`, its horizon, as a
request member beside them, never a member of `Limits`. With
`max_depth = k`, EN-1 expands every product state at depth below `k` and
retains its edges. When it reaches depth `k` with no first-phase violation, a
TP-4 item runs the second phase over the retained graph: a passing SCC there
gives its lasso (V-4), and only when none passes does the run settle V-5 with
`k = max_depth`. Every state of a lasso of length at most `k`, except the
re-entered loop state, sits at a position below `k` and so at a depth below
`k`, so its edges are retained and V-5 means no counterexample of length at
most `k`. Reaching `max_states` or `max_transitions`, the automaton-state limit
`max_automaton_states` (IV-6), a clause meter, or cancellation settles V-7.
This map is `model_check`'s own. Simulation reads `max_depth` the same
way: `explore::Outcome::BoundReached` is a completed horizon, inconclusive,
and only `max_states` and `max_transitions` stop it as `Bounded` (FR-101,
ADR-014 §7).

**Placement in negotiation.** Both engines are backends. QSL publishes a
provider manifest for EN-1 advertising (`temporal-satisfaction`, `bounded`)
and (`temporal-satisfaction`, `unbounded`); the SMT backend's manifest is its
lane's. CG `negotiate_*` settles each item, with an arm per backend that
reads the recorded property form (§1) and the subject classification. The
driver runs an item routed to EN-1 in process, through `qsl-analyze`, and an item
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
| CX-1 | **Shapes.** A safety refutation (TP-1, TP-2, TP-3) is a finite prefix. Under a bounded profile the prefix covers the formula's horizon from every activation it reports, or ends at a terminal state, so the evaluator decides it exactly. Under infinite-trace it is a bad prefix in ADR-014 A-4's sense. A liveness refutation (TP-4) is a lasso: a prefix, possibly empty, and a non-empty loop that re-enters at its first position. A behaviour that ends at a terminal state is the lasso whose loop is the one terminal stutter step (SM-4). A deadlock refutation is a finite prefix ending at the deadlocked state (DL-4). An undefined-evaluation refutation, of any form, is a finite prefix ending at the position where the claim evaluated undefined (UE-3). |
| CX-2 | **Content.** A counterexample over a model subject is QSpec FR-364's counterexample: its members `kind`, `initial_state`, `prefix`, `loop`, `over_binding`, `fairness` and `trace_position`, with the `terminal-stutter` step, as FR-364 states them. QSL's `TemporalCounterexample` (`qsl-replay`) reads and writes exactly that wire and keeps no member list of its own. The clause node and occurrence key travel in the FR-070 envelope, `WitnessEnvelope<TemporalCounterexample>`, with a new `ReplaySource::ModelTrace` source arm; the envelope's obligation identity binds the subject. |
| CX-3 | **Replay.** E9 replay of a model counterexample goes through the layer-6 facade entry `qsl_replay::replay_model_trace`, beside `replay`, as `replay_state_clause` and `replay_frame` do for their families; its envelope's source is `ReplaySource::ModelTrace` and its result is that arm's. It recompiles the package (FR-098), re-admits the subject's initial state and universes from the byte provision, and re-executes each step through `ModelSystem` with FR-101 `replay`. A step's post-state digest selects which successor of its transition identity the step took, since one identity can have several post-states (FR-120). Replay refuses `stale_dependency`/`content-mismatch` when no successor of a step's identity has the recorded digest (FR-101's key mismatch), and `invalid_runtime_input`/`invalid-value` when the loop's last post-state is not its entry state or a transition is not enabled. It then checks the lasso against the fairness set (an unfair lasso refuses, ADR-014 A-4) and evaluates the formula over it with SM-1, reading the lasso by SM-8. Agreement settles `reproduced-with-evaluated-witness` and the item `refuted`; disagreement settles `inconclusive`, `ReplayParity`. A deadlock counterexample replays by DL-5 in place of the formula evaluation, and an undefined-evaluation counterexample by UE-5. |
| CX-4 | **Engine independence.** Both engines produce the same counterexample type. EN-2 reads the transition identity of each step from selector variables in its encoding and names the initial state by index, so its counterexample replays through CX-3 with no solver present. |
| CX-5 | **Canonical loop: a greedy walk.** Given the passing SCC `C` and its entry state `e` (the stem's last state), EN-1 builds the loop by a deterministic walk whose cost is polynomial in the size of `C`. The obligations of `C` are: one state of each acceptance set of the generalized Büchi automaton; and each fairness obligation of the component, weak or strong. A weak constraint's obligation is discharged by an edge of `C` that takes it or by a state of `C` where it is disabled. A constraint whose kind requires it to be taken whenever it is enabled in the component (strong fairness) and that is enabled at some state of `C` has an obligation discharged only by an edge of `C` that takes it; one enabled nowhere in `C` has none. Starting at `e` with the obligations `e` itself discharges removed, the walk repeats: breadth-first search inside `C` from the current state for the nearest state or edge that discharges an open obligation, nearest by edge count, ties broken by canonical transition order and then by obligation order; append that path, ending with the discharging edge when the obligation is an edge; remove every obligation the appended path discharges. When none is open, it appends the shortest path inside `C` back to `e`, ties broken by canonical transition order; when that leaves the loop empty, it appends the shortest cycle through `e`. The walk runs at most one search per obligation plus one, each linear in the edges of `C`. The loop discharges every obligation, so the lasso is fair under every constraint by construction. |

**What exists and what is new.** `qsl_replay::replay`, `WitnessEnvelope`,
`TracePosition`, `ProfileSelection`, the FR-072 settlement and FR-101
`replay` exist. ADR-014 names `TemporalCounterexample` and its replay
refusals, which the temporal spine work is to build. New in this
record: the model-subject step content (CX-2), the facade entry
`replay_model_trace`, `ReplaySource::ModelTrace` and its `ReplayResult` arm, subject inputs in the replay request (initial-state
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
and outputs one `ModelCheckOutcome` per routed item. A proof, a completed
search horizon, an undecided run or a stopped run settles the item's FR-331 terminal
record there. A refutation leaves S6c over a new edge **E11 S6c → S7** as a
typed witness, reaches S8 through E9 as an S6b counterexample does, and
settles its terminal record only after replay (SM-7). SMT temporal checks run on the existing S5 → E7 → S6b
path; S6b's role widens from Kani to any IR-lowered negotiated backend.
ADR-011 §6.1 gains layer A, crate `qsl-analyze`, above the qualified core;
layer 6 `replay` gains the facade entry `replay_model_trace`, the certificate
checkers and the settlement map, and already depends on layer 5.

**Crate layout.**

| ID | Crate | Holds |
| --- | --- | --- |
| LA-1 | `qsl-analyze`, layer A, above the qualified core; it depends on layers 6, 5, 4, 3, F, SV and K, and nothing in layers 1 to 6 depends on it | the engines: EN-1's `model_check`, with its product, phases, canonical counterexample and certificate emission |
| LA-2 | `qsl-eval`, layer 5, in the core | FR-120's `ModelSystem` and the model's other transition systems and memory models, the model subject, FR-101's exploration engine and sampler, the trace evaluator, and the property-automaton translation that the engine and the checkers both use |
| LA-3 | `qsl-replay`, layer 6, in the core | the verdict types (`ModelCheckOutcome`, `TerminalValue`, `ProofBasis`, `Certification`, `InconclusiveCause`, `TemporalCounterexample`, the certificate types), the `replay_model_trace` arm, the certificate checkers (PC-3, PC-4) and the settlement map (FR-127) |
| LA-4 | CG's reads | every type CG reads is in `qsl-replay`, `quire-semantic-value` or `quire-exact`; CG reads no `qsl-analyze` type |

**Consequences for each repository.**

| ID | Repository | Change |
| --- | --- | --- |
| DS-1 | QSL | `qsl-analyze` `model_check` (LA-1): the product `TransitionSystem`, edge retention in the FR-101 engine, the bounded monitor and Büchi translations with interval unrolling (IV-3), the automaton-state limit (IV-6), SCC decomposition and the fairness filter, deadlock detection (DL-7), the canonical counterexample. S3: property form classification, the fairness constraint check with its unmarked granularity (FA-6), interval operators under infinite-trace (IV-1), and the `terminal` member (DL-1). The request writer: the deadlock-freedom item (DL-3). Layer-5 evaluator: interval operators over a lasso (IV-2). `qsl-replay`: `ProofBasis`, the new `InconclusiveCause` variants, `TemporalCounterexample` over a model subject with its `kind` (DL-4, UE-2), `ReplaySource::ModelTrace` and its result arm with the deadlock check (DL-5), the certificate checkers (PC-3, PC-4) and the map from EN-1's outcome into `TerminalValue` (FR-127). The EN-1 provider manifest. |
| DS-2 | CG | A `negotiate_*` arm for EN-1 and one for the SMT temporal arm, each reading the property form and the subject classification; the deadlock-freedom item goes through the TP-1 path of each. The map from the SMT backend's outcome into `TerminalValue`, extending C-09 for `ProofBasis`. Obligation identity binds the subject. |
| DS-3 | IR | Admission of `state` nodes (ADR-016 PI-5), a transition-relation form for operations with frames and contracts, temporal intake for bounded and infinite-trace forms (ADR-014 §13 item 1) with interval operators inside infinite-trace forms (IV-1), and the SMT encodings of EN-2 and EN-3, including the `deadlocked` predicate (DL-7). |
| DS-4 | Driver | Runs an item routed to EN-1 in process and writes its terminal record; runs E9 for its counterexample. |
| DS-5 | QSpec | §8. |

**Sequencing in QSL.** The explicit-state path depends only on QSL work.

1. **Specification.** This record and the QSL compiler requirements
   (FR-123 to FR-128).
2. **Prerequisites already planned in QSL.** FR-101 findings and stopped
   expansions (ADR-016 G-1); FR-120 `ModelSystem` (ADR-016 G-4); the temporal
   spine migration with infinite-trace admission, the layer-5 evaluator and
   lasso evaluation (ADR-014 §11); temporal atoms over state types. With this
   record these are on the critical path for model checking, and the temporal
   spine migration gains an evaluator interface the product can drive.
3. **EN-1.** `model_check`, the EN-1 provider, the model-trace replay entry,
   and the worked example as its first conformance vector.

### 8. What QSpec must specify

| ID | Item | Where |
| --- | --- | --- |
| QS-1 | The model subject of a temporal claim (SM-2, SM-3): initial states, universes, behaviours as maximal paths, position 0 as the initialization observation, position `i` as the post-state of step `i` with its operation anchor | an amendment to QSpec FR-161's "transition/model subject" and a new temporal FR |
| QS-2 | Terminal states (SM-4): closed finite execution under bounded profiles, terminal stutter extension under infinite-trace, the stutter step's identity and its exclusion from fairness; intended terminal states and deadlocks are QS-12 | the same FR; QSpec FR-181 |
| QS-3 | Profiles admitted over a model subject: event-position false-extension and infinite-trace; the model step sequence as their sequence authority and its clock binding | QSpec FR-090, FR-250 |
| QS-4 | Fairness: surface syntax in the shared grammar, `whole` and `each` granularity with `whole` unmarked (FA-6), enabledness (FA-2), weak fairness (FA-3), an extensible fairness kind, admission on infinite-trace clauses only; and what QSpec FR-161 and FR-360 mean by a "missing fairness premise" | shared grammar; QSpec FR-161 |
| QS-5 | Binding of a temporal clause's `over` parameter to a model population: one instance per object of the universe, and the reading at positions where the object does not exist | QSpec FR-161 or the new FR |
| QS-6 | Verdict strength: the V-1 to V-8 table onto QSpec FR-360 and FR-243, the method and depth in the FR-331 terminal record, the certification label `certified`/`uncertified` of a `proved` record (PC-1), the new inconclusive causes including `certificate-rejected` (PC-2), the definition of counterexample length and of V-5 | QSpec FR-161, FR-360, FR-331 |
| QS-7 | Bounded MLTL over a model (TP-2): `on origin` truth depends only on the first `h + 1` positions; `on each` holds at every activating position | QSpec FR-091, FR-092 or the new FR |
| QS-8 | Counterexample wire and its replay rules (CX-2, CX-3, UE-5) | QSpec FR-364 |
| QS-9 | The FR-331 request's subject members (initial-state snapshot references, universes) and the subject's place in the obligation identity; what an advertisement of (`temporal-satisfaction`, `unbounded`) commits to, given that V-5 is a valid result; reconciling QSpec FR-161-AC-7's "liveness backend" wording with the FR-290 mode table | QSpec FR-290, FR-331, FR-161 |
| QS-10 | Conformance vectors, each with expected verdict kind and, for a refutation, a counterexample that must replay (validity is checked; the exact counterexample is not, since engines may return different valid ones): (a) the QSpec TC-200 operator lassos with fair and unfair variants; (b) the §6 worked example, its three verdicts and the bounded variant; (c) one model with an intended terminal state (DL-1) read under event-position false-extension and under infinite-trace, with different verdicts; (d) a bound-reached case (V-5) and an undecided-successor case (V-6); (e) replay refusals: a loop that does not close, an unfair lasso, a post-state digest mismatch, a transition not enabled; (f) deadlocks: a model whose deadlock-freedom item settles `refuted` with a prefix that replays, the same model with `terminal when` covering that state, settling `proved`, and with `terminal any`, which adds no item; (g) interval operators under infinite-trace: the recovery-stability formula of §11 over lassos that satisfy and violate it, `always (req implies eventually[0,5] ack)` classified TP-3, and a case that reaches `max_automaton_states` and settles V-7 | QSpec TC-200 and new TCs |
| QS-11 | v2 temporal operation identities for bounded and infinite-trace forms, needed by the SMT path only | the open item in ADR-014 §13 |
| QS-12 | Deadlocks (§10): the `terminal` member's syntax and meaning (DL-1), the deadlock definition (DL-2), the deadlock-freedom item, its identity and its addition to every request with a model subject unless the model opts out (DL-3), the counterexample `kind` member (DL-4) and its replay (DL-5) | QSpec FR-181, FR-331, the new temporal FR, the shared grammar |
| QS-13 | Interval operators under infinite-trace (§11): closed intervals on every interval-capable operator under the profile (IV-1), their meaning over an infinite trace (IV-2), the safety fragment with interval operators (IV-4), and the automaton-state limit with its V-7 settlement (IV-6). This replaces the rule that the profile admits no interval bound | QSpec FR-090, FR-250-AC-6, FR-255, FR-091 and FR-092 (their infinite-trace paragraphs), FR-161, the shared grammar note on `[a,*]` |

### 9. Rulings on the draft's questions

The owner ruled on the four questions the draft left open, and on undefined
evaluation (RU-5), on 2026-10-01.

| ID | Question | Ruling | Rationale | Where it lands |
| --- | --- | --- | --- | --- |
| RU-1 | Which engine comes first | **Explicit-state first.** | EN-1 depends only on QSL work already planned. QSL has no SMT code; the planned SMT backend is in the contract IR lane, and its temporal use also needs IR to admit state nodes. EN-1 alone gives `proved` and `refuted` for safety and liveness over a finite subject; EN-2 and EN-3 add V-2, V-3 and faster refutation and change no EN-1 verdict | §3; §7 sequencing step 3 |
| RU-2 | The unmarked fairness granularity | **`whole`.** `each` is always written | `whole` is the weaker premise: it excludes fewer behaviours, so an unmarked constraint assumes the least, and a claim proved under it also holds under `each`. It is TLA+'s `WF` over the existentially quantified action, the reading TLA+ authors expect. A clause that relies on per-identity fairness says so in its text, its obligation identity and its counterexample | FA-6; §6 writes `each` on the variant it proves with `each` |
| RU-3 | Whether EN-1 reports reachable terminal states | **Deadlocks are reported by default, as TLC does, with a per-model opt-out.** | A successor-free state that the author did not intend is usually a missing operation or a precondition that is too strong. Stutter extension alone absorbs it silently: every safety claim holds on the stuck tail, and only a claim about progress notices. Reporting it once per subject, as its own item, leaves every claim's verdict equal to its SM-1 truth. A model that halts on purpose in some states and can get stuck in others marks its intended terminal states, so the report survives for the others | §10, DL-1 to DL-7 |
| RU-4 | Whether interval operators may appear inside infinite-trace formulas | **Mixed formulas are admitted**, for example `always (fail implies eventually always[0,10] healthy)` | That formula states recovery stability: after every failure the system eventually stays healthy for ten consecutive steps. A bounded `on each` clause states a bounded response at each activation and cannot put a bounded operator under an unbounded `eventually`; its false-extension also reads a terminal state differently from stutter extension. Mixed formulas cost automaton size that grows with interval length, which §11 states and limits | §11, IV-1 to IV-7; ADR-014 TR-3, A-2 and A-4, amended with this record |
| RU-6 | How a proof settles when its engine has no core certificate checker | **`proved`, labelled `uncertified`.** It is never `inconclusive`. EN-1's proofs carry certificates that core checkers verify | The certificate makes the engine's soundness irrelevant where a checker exists; where none exists the proof still stands, and the label says so | PC-1 to PC-5, LA-3 |
| RU-7 | How an undefined guard or `terminal when` settles | **Refuted**, cause `UndefinedEvaluation{where, cause}`, never a disabled step | A model whose own expressions are undefined on a reachable state does not satisfy the claim, as RU-5 reads an undefined claim | UE-6, DL-5, DL-7 |
| RU-5 | How an engine settles a claim that evaluates undefined in some state | **Refuted**, cause `UndefinedEvaluation{where, cause}`, in every proof engine. Replay reproduces the undefined value at `where`. No new result kind | The Dafny and TLA+ well-definedness reading: an undefined claim does not hold. The refutation is decisive and replays without the engine, and the existing vocabulary carries it, so ADR-013 O-16 and QSpec FR-360 stay as they are | §1 UE-1 to UE-6; SM-5, CX-1, CX-3, DL-4 |

### 10. Deadlocks

| ID | Rule |
| --- | --- |
| DL-1 | **Intended terminal states.** A state model declares its intended terminal states with at most one `terminal` member, in one of two forms. `terminal when P`, where `P` is a state predicate evaluated through the one clause evaluator (ADR-016 FE-3) over the state's observation: a terminal state (SM-4) that satisfies `P` is intended. `terminal any`: every terminal state is intended; this is the per-model opt-out. A model with no `terminal` member intends no terminal state. `P` classifies terminal states only; a state that satisfies it and has a successor is not terminal. The spelling is illustrative; QSpec's shared grammar owns it (QS-12). The member is part of the checked package, so it is part of the subject and its obligation identity (§1 Scope). |
| DL-2 | **Deadlock.** A deadlock is a reachable state of the subject that is terminal (SM-4) and not intended (DL-1). Enabledness is FA-2's, so a state is terminal exactly when no transition identity is enabled at it. The definition reads the state graph alone; fairness plays no part in it. |
| DL-3 | **Reported by default.** For each distinct model subject among a request's temporal items, the request writer (ADR-013 O-20) adds one **deadlock-freedom item**, unless the subject's state model declares `terminal any`. The item is a TP-1 invariant `always holds(not deadlocked)`, where `deadlocked` is DL-2's predicate, derived and not authored. Its obligation identity is the subject and the fixed item kind `deadlock-freedom` (ADR-013 O-09). It is recorded, negotiated and settled as a TP-1 item, with the verdict kinds of §1: V-1 when exploration completes with no deadlock, V-3 by EN-3, V-4 with a deadlock counterexample, and V-5, V-6 and V-7 as for any TP-1 item. |
| DL-4 | **Verdict and counterexample.** A reachable deadlock settles the deadlock-freedom item `refuted`, basis `decisive-counterexample`, `TerminalValue::Refuted` (V-4). Its counterexample is a CX-1 finite prefix from an initial state to the deadlocked state. `TemporalCounterexample` carries a `kind` member, `Formula` for every counterexample of §5, `Deadlock` for this one, and `UndefinedEvaluation{where, cause}` for UE-2's. The item kind and the counterexample kind state the cause; no label, basis or `TerminalValue` variant is added. EN-1's canonical deadlock counterexample is the canonical path to the first deadlocked state in FR-181 canonical breadth-first order. |
| DL-5 | **Replay.** CX-3 replays the prefix unchanged. At its last state it then enumerates the transition identities through `ModelSystem` and evaluates `P` when the model declares `terminal when P`. No enabled identity and `P` false settles `reproduced-with-evaluated-witness` and the item `refuted`. `P` undefined settles the item `refuted` with cause `UndefinedEvaluation{where, cause}` (UE-6). An enabled identity, or `P` true, is a disagreement with the engine and settles `inconclusive`, `ReplayParity`. |
| DL-6 | **Claims read every terminal state the same way.** A deadlock changes no authored claim's verdict: every terminal state, intended or not, reads by SM-4, closed under a bounded profile and stutter-extended under infinite-trace. Under infinite-trace, a behaviour that deadlocks before `p` refutes `eventually holds(p)` with the stutter lasso (CX-1), and the deadlock-freedom item reports the same state as a deadlock; in a model that opts out, only the claim reports it. `terminal when P` changes only the deadlock-freedom item. |
| DL-7 | **Engines.** EN-1 decides the deadlock-freedom item in its first phase: a model state whose FR-120 expansion yields no successor and whose observation does not satisfy `P` is a deadlock. An expansion with an undefined contract conjunction, or a terminal state whose `P` evaluates undefined, refutes the item (UE-6); one with a refused contract conjunction settles V-6 `UndecidedSuccessor`, as for any item. EN-2 and EN-3 encode `deadlocked` as the negation of FA-2 enabledness over every transition identity, conjoined with the negation of `P`. |

### 11. Interval operators under infinite-trace

| ID | Rule |
| --- | --- |
| IV-1 | **Admission.** Under `quire.temporal.infinite-trace/v1`, every temporal operator that carries an interval under a bounded profile (`eventually`, `always`, `until`, `release`, `once`, `historically`, `since`, `triggered`) admits a closed interval `[a,b]` with `a <= b`, or none. With an interval it is an **interval operator**; without one it is an unbounded operator. The two nest in any order, so `always (fail implies eventually always[0,10] healthy)` is admitted. `[a,*]` and `a > b` refuse as ADR-014 TR-3 states for bounded profiles. The checked operator holds `Some(TemporalInterval)` for an interval operator and `None` for an unbounded one. The interval's key carries the infinite-trace profile identity (QSpec FR-255), so it never equals a bounded-profile interval. |
| IV-2 | **Meaning.** An interval operator has QSpec FR-091's offset meaning (future operators) or FR-092's (past operators), read over the infinite trace. Every future position exists, so no closed-boundary rule applies. Position 0 is the behaviour's authoritative origin (SM-3), so a past interval that reaches before position 0 reads atomic predicates there as false, FR-092's complete-history rule. Over a lasso, SM-8 states how positions past the represented trace read, past operators included. Evaluation charges TR-5 work per (node, position) visit. |
| IV-3 | **Translation.** An interval operator's meaning is its expansion into nested next-position steps, with `X` the next-position step and `X^d` `d` of them: `eventually[a,b] p` is `X^a (p or X p or … or X^(b-a) p)`; `always[a,b] p` is `X^a (p and X p and … and X^(b-a) p)`; `p until[a,b] q` is `X^a` applied to the disjunction over `d` in `[0, b-a]` of `X^d q` conjoined with `X^e p` for every `e < d`, which is FR-091's lower-bound convention; `release[a,b]` is its dual; a past interval operator expands into previous-position steps. `X` exists only in this definition; the authored grammar has no next-position operator. The automaton construction (SM-6) does not keep one state per pending `X^d` obligation. It is a **counter construction**: for each occurrence of an interval operator it keeps that occurrence's open obligations and merges each into one that subsumes it. For lower bound 0 over an operand with no interval operator, the earliest open deadline subsumes the later ones for `eventually` and `until`, and the furthest required position subsumes the nearer ones for `always` and `release`, so the occurrence keeps one counter with `b + 1` values. A past interval operator with lower bound 0 keeps one counter of positions since its operand last held, saturating at `b + 1`. For a nonzero lower bound, or an operand that holds another interval operator, the occurrence keeps its open obligations as a set of offsets in `[0, b]`. The translation answers to SM-1 (SM-7). |
| IV-4 | **Safety fragment.** ADR-014 A-4's safety fragment admits every interval operator, in negation normal form: the truth of an interval operator at a position is decided within `b` positions of it. A formula in the fragment is TP-3, so `always (req implies eventually[0,5] ack)` is TP-3. A formula with an interval operator under an unbounded `eventually` or `until` is TP-4, so the recovery-stability formula is TP-4. |
| IV-5 | **Cost.** *Size.* An interval operator with upper bound `b` unrolls to `b + 1` positions: an `a`-step shift and `b - a + 1` copies of its operand. Nested interval operators multiply, so a chain with upper bounds `b1 … bn` unrolls to `O(n × (b1 + 1) × … × (bn + 1))` for a formula of `n` nodes. *States, under IV-3's counter construction.* An occurrence with lower bound 0 whose operand has no interval operator keeps one counter, factor `f = b + 1`, linear in the bound. An occurrence that keeps a set of offsets has factor at most `f = 2^(b+1)`; that is also the bound for a construction that keeps each pending `X^d` obligation separately, which IV-3 does not use. Nested interval operators multiply their factors. The automaton has at most `c × f1 × … × fn` states, with `c` the size the unbounded operators alone give, and the EN-1 product has at most the reachable model states times that. EN-1 explores reachable product pairs only and materializes automaton states as it reaches them, so the cost paid is the reachable part. *Example.* The negation of the recovery-stability formula is `eventually (fail and always eventually[0,10] not healthy)`. Under the counter construction its automaton has 12 states: one before the failure and a counter of positions since the last unhealthy state, 0 to 10. With bound `b` it has `b + 2`. EN-2 encodes an interval operator over its `b + 1` positions at each unrolled step, so its encoding grows linearly in `b` per operator. |
| IV-6 | **Limit.** No syntactic cap on interval length applies beyond TR-3's `u64` bounds. EN-1's limits are FR-101's `Limits` plus `max_automaton_states`, an ADR-014 B-5 budget of QSL's own provider, like the rest of `ModelCheckLimits`, with a published default of 2^20 (1,048,576) automaton states, which a request sets like the other limits. The other published defaults are `max_states` 10,000,000 and `max_transitions` 100,000,000 (FR-101, FR-126). EN-1 counts distinct automaton states as the translation materializes them during product exploration, with checked arithmetic. Reaching the limit stops the run and settles V-7: QSpec FR-360 `failed`, execution `resource-incomplete`, basis `unavailable`, `Incomplete(ResourceExhausted)` naming `max_automaton_states`. V-7 is the stopped-run verdict of §1; `inconclusive` (V-5, V-6) is kept for a run that completed its method. Either way the result is non-Boolean, and the caller raises the limit or narrows the interval and reruns. |
| IV-7 | **Position counting.** An interval operator counts positions, so a formula that has one is not stutter-invariant: inserting a step that repeats a state can change its truth. The state-space reduction record (References) specifies how reductions treat such a formula. A transfer of a claim through a refinement mapping that relies on stutter invariance holds only for formulas with no interval operator. |

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
  `ModelSystem` the critical path for model checking in QSL.
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

Each amended cell carries an "Amended by ADR-018" note.

- ADR-011 §1: the S6c stage row, and the E10 and E11 edges in the stage
  diagram; the S6b output role covers any negotiated IR-lowered backend; the
  S8 row compares a model counterexample's replay with the S6c refutation.
  §2.1: the E10 and E11 rows, and the E9 row's `replay_model_trace` entry.
  §4: S6c admits only S4 outputs. §6.1: layer A `qsl-analyze` with
  `model_check`, serving S6c; layer 5 with the model subject and the
  property-automaton translation; layer 6 `replay` with the verdict types,
  certificate checkers and settlement map (LA-1 to LA-3). §6.2: the
  `model_check` module row. T-13: the
  driver runs S6c, E11 and the E9 replay of a model counterexample.
- ADR-013 O-16: the proof column takes the model-check verdicts, with
  `ProofBasis` and `certification` on `Proved`, the new `InconclusiveCause` variants and
  their categories, and the replay settlements of a model counterexample.
  O-24 public type: `Proved` carries a `ProofBasis`, and the typed
  inconclusive causes include the four new ones.
- ADR-014 §1 B-5 "When reached": a run that completes its search horizon
  `k` (EN-1's `max_depth`) settles `inconclusive` `BoundReached{depth: k}`;
  a run stopped before it completes the horizon settles `incomplete`; EN-1's
  budget type is `ModelCheckLimits`. §2 B-1
  table, row "Temporal interval under `quire.temporal.infinite-trace/v1`":
  optional, refused only as `[a,*]` or `a > b`. §3 TR-3: under infinite-trace
  an operator holds a closed interval or none (IV-1). §3 TR-8 and §10
  scenario 1: EN-1 is a registered liveness backend. §5 A-2: the
  infinite-trace admission admits closed intervals. §5 A-4: the safety
  fragment admits interval operators (IV-4), and lasso evaluation is the
  semantics every engine answers to (SM-1).
- ADR-016 §6: its rules hold for `explore` and `sample`; a model-check result
  reaches proof accounting through negotiation (§3), and a model
  counterexample enters E9 (§5).
- `spec/spec.md`: the index row for this record and for the compiler
  requirements that implement it.

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
  bounded, inductive as new labels). Rejected. QSpec FR-360 and FR-243
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

- Owning ticket: Linear QSL-366. Its QSpec half, QS-1 to QS-13, is QSpec
  FR-360 to FR-370 (Linear STD-131). The language-feature tickets that build on
  this record: QSL-365 (strong fairness), QSL-367 (refinement mappings),
  QSL-368 (state-space reduction, whose record is ADR-021 on
  quire-spec-language#568), QSL-369 (EF and AG EF), QSL-370
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
