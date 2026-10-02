---
id: ADR-019
title: "Strong fairness of operations"
type: ADR
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-016
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: relates_to
  - target: ix://agent-ix/quire-specification/FR-161
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-243
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-290
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-360
    type: depends_on
---
# ADR-019: Strong fairness of operations

## Status

Proposed, 2026-10-01. It builds on ADR-018. It follows ADR-018's rulings: unmarked fairness
granularity is `whole` (ADR-018 FA-6), deadlocks are reported by a derived
deadlock-freedom item (ADR-018 DL-1 to DL-7), and the infinite-trace profile
admits interval operators nested under unbounded ones (ADR-018 IV-1 to IV-7).
The owner's rulings on this record's own draft questions are in §9. The owning ticket and related
work are listed under References.

"QSpec FR-nnn" names a quire-specification requirement; a bare FR id is a QSL
requirement. QSpec FR-360 is the infinite-trace result disposition
vocabulary, as in ADR-018. Item ids `SY-`, `SF-`, `FS-`, `BE-`,
`SV-`, `QS-` and `AM-` are local to this record. Other artifacts cite them as
`ADR-019 FS-2`.

## Context

ADR-018 gives a temporal claim over a model subject an every-behaviour
verdict. Its explicit-state engine EN-1 builds the product of the subject's
state graph with a generalized Büchi automaton for the negated formula,
retains every product edge, decomposes the retained graph into strongly
connected components (SCCs), and accepts a cycle when an accepting SCC passes
the fairness filter (ADR-018 FA-4). Its fairness is weak fairness of
operations, per operation (`whole`) or per transition identity (`each`)
(ADR-018 FA-1 to FA-3). ADR-018 FA-5 leaves a seam: a new `FairnessKind`
adds a variant and a filter rule, and that rule may refine an SCC into
smaller ones and re-test them.

Weak fairness admits a behaviour in which an operation is enabled infinitely
often and also disabled infinitely often, and is never taken. Contended and
retrying systems produce exactly that shape: a lock acquirer that finds the
lock free only between another holder's release and re-acquire, or a node
that can rejoin an election only in the windows between other nodes leaving.
Their liveness needs strong fairness. TLA+ writes it `SF`, Quint
`strongFair`.

QSpec FR-161 states that fairness filters the admitted infinite traces before
evaluation and that the fairness set enters every proof and counterexample
identity (FR-161-AC-5). QSpec FR-362 settles a missing fairness premise
`unsupported`, cause `unsupported_projection`/`missing-fairness-premise`.

## Decision

### 1. Syntax

| ID | Rule |
| --- | --- |
| SY-1 | A fairness constraint names a **kind**, a **granularity** and one operation of the model. The kinds are `weak` and `strong`. The granularities are ADR-018 FA-1's `whole` and `each`, with the same meaning for both kinds. |
| SY-2 | The unmarked kind is `weak`: `fair Op` means `fair weak whole Op`. Strong fairness is always written with the `strong` keyword. `weak` may be written and means the same as the unmarked kind. The kind enters the clause's obligation identity and its counterexample in its resolved form, `weak` or `strong`, however it was spelled (SV-5). |
| SY-3 | The unmarked granularity is `whole`, as ADR-018 FA-6 rules for weak fairness, and it is the same for both kinds: `fair strong Op` means `fair strong whole Op`. The example below writes `each` wherever it relies on per-identity fairness. |
| SY-4 | A clause may carry weak and strong constraints together, including both kinds on the same operation. Strong implies weak (§2), so a weak constraint beside a strong one on the same operation and granularity adds nothing to the premise; both stay in the fairness set. |
| SY-5 | The constraint type is ADR-018 FA-5's `FairnessConstraint{kind, operation, granularity}` with `FairnessKind::Strong` added beside `FairnessKind::Weak`. Strong constraints are admitted on infinite-trace clauses only, as weak ones are. |

The spelling, illustrative as in ADR-018 §6 (QSpec's shared grammar owns it,
QS-1):

```text
temporal Served using inf over (m: M::Mutex) clock "model-steps" on origin {
  fair strong each M::Mutex::acquire;
  fair weak whole M::Mutex::release;
  always eventually holds(m.owner = 1)
}
```

### 2. Semantics

| ID | Rule |
| --- | --- |
| SF-1 | **One semantics.** Strong fairness is a filter on behaviours, applied before SM-1 evaluates the formula (ADR-018 SM-1, SM-5; QSpec FR-161). A strong constraint changes which behaviours are admitted; it never changes how a formula evaluates on an admitted behaviour. |
| SF-2 | **Enabled and taken.** A constraint is enabled at a position exactly as ADR-018 FA-2 defines it: a transition identity is enabled at a state when the operation's effective precondition holds for that receiver and argument vector and at least one post-state satisfies its postcondition; a `whole` constraint is enabled when any of its identities is. A constraint is taken at a step when the step's transition identity belongs to it. Enabledness is a property of the model state at the position and never of the automaton state. |
| SF-3 | **Strong fairness.** A behaviour satisfies a strong constraint when, if the constraint is enabled at infinitely many positions, it is taken at infinitely many steps. In automata terms it is a Streett pair: the states where the constraint is enabled, and the edges that take it. |
| SF-4 | **On a lasso.** The positions of the loop recur forever and the stem's do not, so on a lasso SF-3 reduces to the loop: if the constraint is enabled at some state of the loop, some step of the loop takes it. A lasso whose loop is the terminal stutter step (ADR-018 SM-4) satisfies every strong constraint, since no operation is enabled at a terminal state. Every terminal state, intended or not, reads this way under infinite-trace (ADR-018 DL-6). A deadlock is a reachable terminal state that the model does not mark intended (ADR-018 DL-2: `terminal when P`, or `terminal any` to opt out), and it is reported through the derived deadlock-freedom item (ADR-018 DL-3). DL-2 reads the state graph alone and that item is a TP-1 invariant with no fairness set, so strong fairness neither hides nor creates a deadlock. |
| SF-5 | **Strong implies weak.** A constraint continuously enabled from some position onward is enabled infinitely often, so every behaviour that satisfies the strong constraint satisfies the weak one with the same operation and granularity. The strongly fair behaviours are a subset of the weakly fair ones, so a claim proved under weak fairness holds under strong fairness, and a claim refuted under strong fairness is refuted under weak fairness. |
| SF-6 | **Granularity.** As for weak fairness, `each` is the stronger premise: every behaviour fair under `strong each` for an operation is fair under `strong whole` for it. Under `whole`, any transition of the operation discharges the obligation, so a different receiver or argument vector can starve one identity. |
| SF-7 | **Machine closure.** Strong fairness of operations of the model is machine-closed: every finite prefix of a behaviour extends to a behaviour fair under any finite set of weak and strong constraints. On a finite subject this is direct: from the last state of the prefix some bottom SCC of the state graph is reachable, every transition enabled at a state of a bottom SCC stays inside it, and a cycle through every edge of that SCC is fair for every constraint. A terminal state is a bottom SCC whose one edge is the stutter step, which enables no operation, so the argument covers prefixes that end in a terminal state, intended or not (ADR-018 DL-1, DL-2). Two consequences carry over from ADR-018 §4: strong fairness never makes a proof vacuous, and it never changes the verdict of a safety form (TP-1, TP-2, TP-3), whose truth depends on finite prefixes only. |

### 3. Checking on EN-1: SCC refinement for Streett emptiness

EN-1's first phase is unchanged. Its second phase decides whether the retained
product graph contains an accepting cycle that is fair for every constraint.
With strong constraints present this is emptiness of a generalized Büchi
automaton with Streett fairness, decided by recursive SCC refinement in the
style of Emerson and Lei, and of Latvala and Heljanko's treatment of strong
fairness in explicit-state LTL checking (References).

| ID | Rule |
| --- | --- |
| FS-1 | **Enabled sets.** For each model state, EN-1 records the set of transition identities enabled there. These are exactly the labels of the model state's outgoing FR-120 transitions, which the first phase computes when it expands the state, so the set costs no extra contract evaluation. The product reads enabledness from this set (SF-2), including at product states where the automaton has no move for some model transition. |
| FS-2 | **Filter.** `fair(C)` over a set of product states `C` and the constraint set: decompose the subgraph induced by `C` into SCCs. For each non-trivial SCC `S` (one that contains an edge): (a) `S` must contain a state of every acceptance set of the automaton; (b) every weak constraint must have an edge in `S` or be disabled at some state of `S` (ADR-018 FA-4); (c) for every strong constraint, `S` must contain an edge that takes it, or the constraint must be disabled at every state of `S`. An `S` that meets (a), (b) and (c) passes. An `S` that fails (a) or (b) is rejected. An `S` that meets (a) and (b) and fails (c) for a set of strong constraints `B` is refined: let `R` be the states of `S` at which some member of `B` is enabled, and recurse with `fair(S \ R)`. |
| FS-3 | **Why the refinement is exact.** No cycle inside `S` takes a member of `B`, since `S` has no edge that takes it. A fair cycle inside `S` therefore visits no state where a member of `B` is enabled, so every fair accepting cycle of `S` lies in `S \ R`. The recursion re-tests (a), (b) and (c) on the sub-SCCs, because removing states can remove the accepting state, the weak constraint's edge or its disabled state, or a strong constraint's edge. |
| FS-4 | **Termination and depth.** A strong constraint that triggers a refinement of `S` is disabled at every state of every sub-SCC of `S \ R`, so it never triggers again below `S`. Every refinement removes at least one state. The recursion depth is therefore at most `min(k, n) + 1`, where `k` is the number of strong constraints after `each` expansion and `n` the number of product states. |
| FS-5 | **Complexity.** The components at one depth are disjoint, so each depth costs one SCC decomposition and one pass of the tests over at most the whole retained graph: `O(n + m')`, with `m'` the product edges plus the enabled-set entries of FS-1. Phase two costs `O((min(k, n) + 1) · (n + m'))`. With weak constraints only, the depth is one and phase two is ADR-018's single pass. Under `each` granularity, `k` is the number of transition identities of each strongly fair operation (receivers times argument vectors), which is where the cost of strong fairness grows. Phase two runs under EN-1's `ModelCheckLimits`, ADR-014 B-5 budgets: the subject's evaluation meter and the cancellation poll settle V-7. Fairness constraints are not part of the formula, so they add no automaton states: `n` is the reachable product phase one built, whose automaton factor ADR-018 IV-5 bounds and whose `max_automaton_states` budget, one of the `ModelCheckLimits` (ADR-018 IV-6), settles V-7 in phase one, before refinement starts. |
| FS-6 | **Canonical result.** Sub-SCCs are enumerated in discovery order within their parent, and the first passing component in that order, depth-first through the refinement, is the one the counterexample uses. The stem is the first path in FR-181 canonical breadth-first order from an initial product state to that component. The loop is built by ADR-018 CX-5, the greedy walk, applied to that passing component and its entry state. |
| FS-7 | **The loop is fair by construction.** The loop lies inside the passing component `P` and is built by ADR-018 CX-5, whose obligations include each fairness obligation of `P`, weak or strong. It visits a state of every acceptance set and, for every weak constraint, an edge of it or a state where it is disabled. For every strong constraint enabled at some state of `P`, CX-5 discharges the obligation only by an edge that takes it, and `P` has one (FS-2 (c)). Every other strong constraint is disabled at every state of `P`, hence at every state of the loop, and CX-5 gives it no obligation. So every strong constraint enabled at a loop state is taken in the loop, which is SF-4. |
| FS-8 | **Replay re-establishes fairness.** The engine's claim of fairness is not trusted. ADR-018 CX-3 replay re-executes the lasso through `ModelSystem`, computes the enabled transition identities at every loop state from FR-120 itself, and checks every constraint of the fairness set against the loop: weak by FA-3, strong by SF-4. A lasso that fails a strong constraint is refused as an unfair lasso, with ADR-018 CX-3's existing refusal. Replay decides from the fairness set ADR-018 CX-2 already carries, with each constraint's kind. |
| FS-9 | **Mixed formulas.** The filter reads only model states, model transitions and enabled sets; the property automaton enters only through its acceptance sets (FS-2 (a)). An automaton that unrolls interval operators nested under unbounded ones into chains of next steps is one more automaton, so FS-1 to FS-8 hold for mixed formulas unchanged. Interval lengths enlarge `n` and `m'` as ADR-018 IV-5 states, and phase two's cost (FS-5) scales with that product. |

### 4. Back ends

| ID | Back end | Strong fairness |
| --- | --- | --- |
| BE-1 | **EN-1, explicit-state product** (ADR-018 §3) | Decides strong fairness exactly by §3. Proves (V-1) and refutes (V-4) TP-4 claims with strong constraints. Its provider manifest advertises the fairness kinds it decides, `weak` and `strong` (SV-1). |
| BE-2 | **EN-2, SMT lasso unrolling** (ADR-018 §3) | Encodes a strong constraint as a condition on the loop segment: for each loop position and each strong constraint, enabled at that position implies some loop step takes it. Enabledness is an existential over post-states, so each pair adds one existentially chosen successor. Refutation (V-4) only, otherwise V-5. It arrives with EN-2, after IR admits state nodes and temporal forms, as ADR-018 RU-1 orders it. |
| BE-3 | **EN-3, `k`-induction** (ADR-018 §3) | Settles safety forms. By SF-7 strong constraints leave a safety verdict unchanged, so EN-3 reads the claim without them. |
| BE-4 | **Lasso monitor** (ADR-014 A-4) | Checks strong fairness on the one lasso it is given, by SF-4, using enabledness at each loop position. A model-subject lasso has enabledness from `ModelSystem`, and a true result on a fair one is `tested`, evidence for that lasso only. A supplied trace carries no enabledness, so a clause with a fairness constraint over it settles `unsupported`, `MissingFairnessPremise` (SV-2). |
| BE-5 | **TLC** | Checks `SF`. QSL reaches it only through lowering a QSL model to TLA+, which is later research outside this record. The correspondence that lowering would use: `fair strong whole Op` is `SF_vars(\E args : Op(args))`; `fair strong each Op` is `\A args : SF_vars(Op(args))` over the finite receiver and argument domains. |

Negotiation never drops or weakens a fairness constraint to fit a back end.
An item whose fairness set contains a strong constraint is routed only to a
candidate that advertises `strong`.

### 5. Verdicts

Every verdict is one of ADR-018 V-1 to V-8, with ADR-018's `ProofBasis` and
`InconclusiveCause` members. Strong fairness adds one `Unsupported` cause
(SV-2), one manifest capability (SV-1) and one diagnostic (SV-6).

| ID | Rule |
| --- | --- |
| SV-1 | **Capability.** A temporal provider manifest advertises the fairness kinds its candidate decides. When no candidate for (`temporal-satisfaction`, `unbounded`) advertises `strong`, an item with a strong constraint settles V-8, `unsupported-requested-capability`, with a warning naming the strong-fairness capability, as QSpec FR-161-AC-7 does for a missing liveness backend. |
| SV-2 | **Missing fairness premise.** `MissingFairnessPremise{constraint}` is a new `Unsupported` cause, QSpec FR-362's "missing fairness premise": `unsupported`, basis `unavailable`, wire cause `unsupported_projection`/`missing-fairness-premise`. Enabledness comes from a model subject only, through FR-120. A supplied trace, replayed or monitored with no model subject behind it, carries states and steps and no enabledness. A clause with any fairness constraint, weak or strong, evaluated over a supplied trace settles `unsupported` with this cause, naming the first constraint of its checked fairness set, which holds the constraints in source order of first occurrence (FR-123). A clause with an empty fairness set over a supplied trace evaluates as ADR-014 A-4 states. |
| SV-3 | **Undecided enabledness.** A contract conjunction that evaluates undecided while EN-1 computes an enabled set leaves both the successor relation and enabledness unknown, and settles V-6 `UndecidedSuccessor`, as ADR-018 states. |
| SV-4 | **Unfair counterexample.** A lasso that fails a strong constraint on replay is refused by ADR-018 CX-3 and settles no `refuted`. An EN-1 counterexample is fair by FS-7; a refusal of one is an engine defect and settles V-6 `ReplayParity`. |
| SV-5 | **Identity.** Each constraint's kind enters the clause's obligation identity with its operation and granularity (QSpec FR-161-AC-5). A claim under `strong each` and the same claim under `weak each` are different obligations, and a verdict on one never joins the other's request. |
| SV-6 | **Strong-fairness hint.** A `refuted` TP-4 item whose counterexample lasso would be excluded by a strong constraint the clause does not carry names that constraint in a diagnostic. Replay computes it after CX-3 settles `reproduced-with-evaluated-witness`, from the enabled sets it already recomputes at each loop state (FS-8), so every engine's refutation gets it. The replay result carries the hints, and the writer of the item's terminal record copies them onto it: `model_check`'s settlement for an EN-1 refutation, and the orchestrating driver (ADR-011 T-13) for an SMT-backend refutation settled through CG's map (ADR-018 DS-2). For each operation of the model in canonical order: if the operation is enabled at some loop state and no loop step takes it, the hint names `fair strong whole Op`; otherwise, if some transition identity of it is enabled at a loop state and no loop step takes that identity, the hint names `fair strong each Op` with that identity. An operation the clause already constrains with the named kind and granularity gets no hint. Each hint is a warning-severity diagnostic `fairness.strong-would-exclude` on the item's FR-331 terminal record, beside the counterexample, with members: the suggested constraint (kind, operation, granularity), the identity for `each`, and the first loop position where it is enabled. It is outside the counterexample, its replay identity and the obligation identity, and it changes no verdict. |

### 6. Worked example: a contended mutex

The subject is a small model in the style of ADR-018 §6. The fairness clause
spelling is illustrative (QS-1).

**Model.** Object type `Mutex` with `owner: Int[0, 2]`, where 0 is free and 1
or 2 names the holding process; population `locks`. Operation `acquire(p:
Int[1, 2])` and operation `release()`, both with frame `modifies [owner]` and
no result.

```text
profile v   = "quire.value.complete/v1" …;
profile inf = "quire.temporal.infinite-trace/v1" …;
model M = "example/mutex" …;

pre  Free  using v on M::Mutex::acquire { self.owner = 0 }
post Takes using v on M::Mutex::acquire { self.owner = p }
pre  Held  using v on M::Mutex::release { self.owner != 0 }
post Frees using v on M::Mutex::release { self.owner = 0 }

temporal OneServed using inf over (m: M::Mutex) clock "model-steps" on origin {
  fair strong each M::Mutex::acquire;     // or: fair weak each, fair strong whole
  always eventually holds(m.owner = 1)
}
```

**Subject.** Universe `locks = {m}`. One initial snapshot: `m.owner = 0`.
Write a state by its `owner` value, `acq(1)` and `acq(2)` for `acquire` with
receiver `m` and argument 1 or 2, and `rel` for `release`. Three states are
reachable: at 0, `acq(1)` and `acq(2)` are enabled; at 1 and at 2, only `rel`
is. Edges: `0 -acq(1)-> 1`, `0 -acq(2)-> 2`, `1 -rel-> 0`, `2 -rel-> 0`. No
state is terminal, so the derived deadlock-freedom item (ADR-018 DL-3) settles V-1, and every behaviour returns to 0 after every
acquisition, so `release` needs no fairness here: at 1 and 2 it is the only
enabled transition, and a behaviour is a maximal path.

**Product.** The negation is `eventually always (m.owner != 1)`, ADR-018 §6's
two-state Büchi automaton: `q0` loops on every state and moves to accepting
`q1` on a state with `owner != 1`; `q1` loops on such states. The reachable
product has 5 states: `(0, q0)`, `(1, q0)`, `(2, q0)`, `(0, q1)`, `(2, q1)`.
It has one accepting SCC, `S = {(0, q1), (2, q1)}`, with edges `acq(2)` and
`rel`. The product has no `acq(1)` edge out of `(0, q1)`, since `q1` has no
move on `owner = 1`; the model state 0 still enables `acq(1)` (FS-1).

**Verdict under `fair weak each acquire`.** In `S`, `acq(1)` has no edge and
is disabled at `(2, q1)`, so FA-4 accepts it; `acq(2)` has an edge. `S`
passes. The initial product state `(0, q1)` is in `S`, so the stem is empty.
The item settles `refuted` (V-4) with this lasso, bound to the only `m`:

| Position | State | Step into the next position |
| --- | --- | --- |
| 0 (loop entry) | `0` | `acq(2)` |
| 1 | `2` | `rel`, back to position 0 |

Replay re-executes `acq(2)` and `rel` from the snapshot, checks that the last
post-state digest equals position 0's, checks weak fairness of `acq(1)` (it
is disabled at position 1) and of `acq(2)` (taken), and evaluates `always
eventually holds(m.owner = 1)` over the lasso: `owner` is never 1, so the
formula is false at position 0, `trace_position` `0`. `acquire` is taken in
the loop, so no `whole` hint applies; `acq(1)` is enabled at position 0 and
never taken, so the terminal record carries the SV-6 diagnostic `fair strong
each M::Mutex::acquire`, identity `acq(1)`, position 0.

**Verdict under `fair strong each acquire`.** In `S`, `acq(1)` has no edge
and is enabled at `(0, q1)`, so `S` fails FS-2 (c) with `B = {acq(1)}`. The
refinement removes `R = {(0, q1)}`. `fair({(2, q1)})` finds one trivial SCC,
with no edge, and rejects it. No fair accepting cycle exists. The item
settles `proved`, basis `closed-scope`, `TerminalValue::Proved{basis:
Exhaustive}` (V-1), over this subject. The lasso above is unfair under this
premise: `acq(1)` is enabled at position 0 and never taken, so replay refuses
it (FS-8).

**Verdict under `fair strong whole acquire`**, which is also the reading of
unmarked `fair strong acquire` (SY-3). The one constraint is taken by
`acq(2)` inside `S`, so `S` passes and the item settles `refuted` with the
same lasso. Process 2 re-acquiring satisfies fairness for `acquire` as a
whole while process 1 starves (SF-6).

### 7. Downstream impact and sequencing

| ID | Repository | Change |
| --- | --- | --- |
| DS-1 | QSL | S3: admit the `strong` kind and check the constraint (SY-1 to SY-5). Layer 5 `model_check`: enabled sets per model state (FS-1), the recursive filter (FS-2), the canonical component and loop (FS-6, FS-7). `qsl-replay`: the strong check in CX-3 replay and in the lasso monitor, `MissingFairnessPremise`, the SV-6 hint. The EN-1 manifest's fairness kinds, and the layer-R registry's candidate filter on fairness kinds (DS-2). |
| DS-2 | QSL and CG | QSL's layer-R registry filters candidates by fairness kind: a backend that does not advertise `strong` is not a candidate for an item with a strong constraint, and when no candidate is left the item settles V-8 `unsupported`, `unsupported-requested-capability`, naming the missing fairness capability (SV-1). The registry already computes candidates (FR-075), so the correctness decision stays where candidates are made. CG negotiates over the filtered set and carries the kind in the obligation identity (SV-5). |
| DS-3 | IR | The EN-2 loop-segment encoding of BE-2, as part of EN-2. |
| DS-4 | QSpec | §8. |

**Sequencing.** Strong fairness on EN-1 follows EN-1 (ADR-018 §7 step 3)
and depends on nothing else in ADR-018. Its EN-2 part lands with EN-2, which
follows IR state-node admission and the SMT backend (ADR-018 RU-1). It adds no stage, edge or layer to ADR-011.

### 8. What QSpec must specify

| ID | Item | Where |
| --- | --- | --- |
| QS-1 | Surface syntax: the `strong` kind beside `weak`, unmarked kind `weak` and `strong` always written (SY-2), the same `whole`/`each` granularity with unmarked `whole` (SY-3), so `fair Op` is `fair weak whole Op`; admission on infinite-trace clauses only | shared grammar |
| QS-2 | Strong fairness semantics on infinite traces (SF-3) and on lassos (SF-4), the shared enabled/taken definition (SF-2), strong implies weak (SF-5), granularity ordering (SF-6), machine closure and its two consequences (SF-7) | QSpec FR-161 |
| QS-3 | "Missing fairness premise": enabledness comes from a model subject only; a supplied trace carries none, so a clause with any fairness constraint, weak or strong, over a supplied trace settles `unsupported` (SV-2) | QSpec FR-161, FR-360, FR-362 |
| QS-4 | Provider advertisement of supported fairness kinds, and the rule that negotiation never drops or weakens a fairness constraint; the `unsupported` warning naming the strong-fairness capability | QSpec FR-290 and the provider manifest contract |
| QS-5 | The fairness set in the obligation identity and the counterexample wire carries each constraint's kind, operation and granularity | QSpec FR-161-AC-5, FR-331 and the counterexample contract |
| QS-6 | Replay checks every constraint of the fairness set against the loop with enabledness recomputed from the model, and refuses an unfair lasso | the counterexample contract |
| QS-7 | Conformance vectors, each with expected verdict kind and, for a refutation, a counterexample that must replay: (a) the §6 mutex under `strong each` (proved), `weak each` (refuted) and `strong whole` (refuted); (b) lassos with fairness verdicts: strongly fair; weakly fair and strongly unfair (refused under the strong premise); terminal stutter loop (fair); (c) a model whose accepting SCC fails a strong constraint and whose refined remainder still contains a fair accepting cycle, so the recursion is exercised past one level; (d) a supplied trace with a weak constraint and one with a strong constraint (both `unsupported`, missing fairness premise); (e) no candidate advertising `strong` (`unsupported`, with the warning); (f) the §6 `weak each` refutation carries the SV-6 hint `fair strong each M::Mutex::acquire`, and the `strong whole` refutation carries the same hint; (g) `fair Op` and `fair weak whole Op` produce the same obligation identity | QSpec TC-200 and new TCs |
| QS-8 | The strong-fairness hint (SV-6): when it is emitted, how the suggested constraint and granularity are chosen, its diagnostic code and members on the terminal record, and that it sits outside the counterexample, replay identity and obligation identity | QSpec FR-331 and the diagnostic contract |

### 9. Rulings on the draft's questions

The owner ruled on the draft's open questions on 2026-10-01.

| ID | Question | Ruling | Where it lands |
| --- | --- | --- | --- |
| RU-1 | The unmarked fairness kind | **`weak`.** `fair Op` means `fair weak whole Op`; strong fairness is always written `strong` | SY-2 |
| RU-2 | The unmarked granularity | **`whole`**, for both kinds, as ADR-018 FA-6 and RU-2 rule | SY-3 |
| RU-3 | The canonical loop | **A polynomial greedy walk**, settled in ADR-018 as CX-5, which ADR-019 cites | FS-6, FS-7 |
| RU-4 | A hint on weak-fairness refutations | **Added.** A refutation whose loop a strong constraint would exclude names that constraint; diagnostic only, no verdict change | SV-6 |
| RU-5 | Enabledness on supplied traces | **None.** Fairness of either kind over a supplied trace settles `unsupported`, `MissingFairnessPremise` | SV-2, BE-4, AM-6 |

## Consequences

- Liveness of contended and retrying systems is provable on a finite model
  subject by QSL's own explicit-state engine.
- One filter function covers weak and strong fairness; with weak constraints
  only it is ADR-018's single pass, so strong fairness costs nothing to
  claims that do not use it.
- The cost of strong fairness grows with the number of strong constraints
  after `each` expansion, bounded by FS-5.
- Counterexamples keep ADR-018's shape and wire; replay decides fairness
  without trusting the engine.
- A strong premise reaches only back ends that decide it, and a supplied
  trace without enabledness settles `unsupported` with a named cause.

## Amendments made with this record

Each amended cell or paragraph of ADR-018 and ADR-014 carries an "Amended by
ADR-019" note.

- **AM-1, ADR-018 FA-5:** `FairnessKind::Strong` is added; the fairness
  filter is FS-2, recursive when a strong constraint fails.
- **AM-2, ADR-018 FA-2 and FA-4:** enabledness at a product state is read
  from the model state's enabled set (FS-1), never from product edges. This
  matters for weak constraints as well: a GBA with no move on some model
  transition removes that edge from the product, and the constraint is still
  enabled.
- **AM-3, ADR-018 §4:** the machine-closure paragraph covers strong
  constraints (SF-7).
- **AM-4, ADR-018 V-8 and `TerminalValue::Unsupported`:** the
  `MissingFairnessPremise{constraint}` cause (SV-2), for both kinds; EN-1's
  provider manifest advertises fairness kinds (SV-1); the SV-6 diagnostic on
  a refuted TP-4 item's terminal record.
- **AM-5, ADR-018 §8 QS-4:** QS-4's "extensible fairness kind" is the `strong` kind of QS-1.
- **AM-6, ADR-014 A-4:** the lasso fairness check covers strong constraints
  by SF-4, reads enabledness at each loop position from the model subject,
  and settles a clause with any fairness constraint over a supplied trace
  `unsupported`, `MissingFairnessPremise`.
  ADR-014's `TemporalCounterexample.fairness` names each constraint with its
  kind.
- `spec/spec.md`: index row.

## Alternatives Considered

- **Strong fairness as the unmarked kind.** Rejected (RU-1): strong excludes
  more behaviours, so an unmarked constraint would assume more than it says.
- **Enabledness observations on supplied traces.** Rejected (RU-5): a trace
  would have to record, at every position, which transition identities a
  model it does not carry would enable.
- **Translating strong fairness into the formula** (`(G F enabled) implies
  (G F taken)` as an antecedent per constraint). Rejected. It needs
  `enabled` and `taken` atoms outside the formula grammar, it
  multiplies the Büchi automaton by the constraints, and it puts the premise
  inside the property, where the obligation identity and the counterexample
  would no longer show it as a premise.
- **Degeneralizing to a single Streett or Rabin condition and running a
  general Streett emptiness algorithm.** Rejected. The pairs here have a
  fixed shape (enabled states, taking edges), which FS-2's refinement uses
  directly on the graph ADR-018 already retains.
- **Approximating strong by weak fairness on back ends that decide only
  weak.** Rejected. It keeps proofs sound (SF-5) but yields refutations that
  replay refuses, and it hides from the verdict which premise was checked.

## References

- Owning ticket: Linear QSL-365. ADR-018's owning ticket: QSL-366. §8 is
  carried into QSpec by QSpec FR-362 (weak and strong fairness, the missing
  fairness premise), FR-364 (replay), FR-365 (the canonical counterexample),
  FR-368 (fairness kinds in negotiation) and FR-369 (the strong-fairness
  hint) (Linear STD-132).
- Later research on lowering to TLA+ and Quint, which BE-5 depends on:
  RES-42.
- QSpec V1-TEMP-021 to V1-TEMP-026: infinite-trace capability inventory rows.
- E. A. Emerson and C.-L. Lei, "Modalities for model checking: branching time
  logic strikes back", Science of Computer Programming, 1987: fair-SCC
  analysis under Streett-type fairness by recursive SCC decomposition.
- T. Latvala and K. Heljanko, "Coping with strong fairness", Fundamenta
  Informaticae, 2000: strong fairness in explicit-state LTL model checking
  through SCC refinement of the product.
- L. Lamport, "The temporal logic of actions", ACM TOPLAS, 1994, and
  *Specifying Systems*, 2002: `WF` and `SF`, and machine closure of fairness
  on subactions of the next-state relation.
