---
id: ADR-020
title: "Refinement mappings between QSL models"
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
  - target: ix://agent-ix/quire-spec-language/ADR-015
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-016
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-017
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-057
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: relates_to
  - target: ix://agent-ix/quire-specification/FR-013
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-151
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-161
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-177
    type: relates_to
  - target: ix://agent-ix/quire-specification/FR-181
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-243
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-290
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-331
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-348
    type: relates_to
  - target: ix://agent-ix/quire-specification/FR-353
    type: relates_to
---
# ADR-020: Refinement mappings between QSL models

## Status

Proposed, 2026-10-01. Draft for plan-lead review of the key decisions; the
QSL compiler requirements that implement it follow review. It builds on
ADR-018, itself a draft. The owning ticket and related work are listed under
References.

"QSpec FR-nnn" names a quire-specification requirement; a bare FR id is a QSL
requirement. Item ids `RM-`, `RS-`, `CO-`, `AX-`, `RE-`, `RC-`, `MC-`, `DS-`
and `QS-` are local to this record. Other artifacts cite them as
`ADR-020 RS-2`. Items of ADR-018 are cited as `ADR-018 SM-3`.

## Context

A layered design proves properties of an abstract model once, then shows
that each more detailed model implements the abstract one. The detailed
model then inherits the abstract model's properties through the mapping.
TLA+ does this with a refinement mapping: a function from concrete states to
abstract states, under which every concrete behaviour is an abstract
behaviour up to stuttering, with the abstract fairness holding on the mapped
behaviour. TLC checks it as a temporal property; deductive tools check it as
per-step simulation obligations.

QSL relates models in three other ways today, none of which relates the
behaviours of two QSL state models:

- **Protocol refinement** is QSpec FR-177: an implementation protocol model
  against a specification under a trace, failure and observation relation,
  with permitted internal steps, refusal sets, divergence and environment
  assumptions. It is QSpec FR-290's `refinement` kind, a `protocol`-family
  member. QSL's S4 emits no protocol node, and no QSL engine explores a
  protocol.
- **The abstraction relation** is QSpec FR-353 and ADR-017 §3: an authored
  binding from model objects, populations and operation frames to Rust types,
  collections and functions. QSL checks its keys and the syntax of its Rust
  paths and carries it to CG in the checked package. Kani and Verus read it
  as a premise of `operation-contract` claims. QSL never resolves or runs the
  code side (ADR-017 AR-2).
- **Spec-versioning and profile-layering gates** (ADR-017 §2) are corpus
  tests that compare spine outcomes of two revisions of one specification.
  They record no claim.

ADR-018 gives QSL every-behaviour verdicts over a **model subject** (ADR-018
SM-2): a checked package with a state model, its initial states and
universes, with FR-120's `ModelSystem` as its successor relation. Its
explicit-state engine EN-1 builds a product of the subject's state graph with
a property automaton on FR-101's canonical breadth-first engine, retains the
product edges, and finds fair accepting cycles by SCC decomposition under
weak fairness (ADR-018 FA-1 to FA-5). Behaviours are maximal paths, each
position carries the transition that reached it, and a terminal state is
extended by a terminal stutter step (ADR-018 SM-3, SM-4). Counterexamples are
model traces replayed through `ModelSystem` (ADR-018 CX-1 to CX-4). ADR-018
§7 lists refinement mappings as a feature built on EN-1.

QSpec FR-290 already names the claim form for step-wise refinement of state
models: "Refinement between two operation contracts or state models",
required kind `operation-contract`, "each implication between the two
contracts' clauses is one `operation-contract` claim". ADR-017 RF-1 records
that no route owns it. QSpec FR-161 gives infinite-trace operators their
meaning; its until is non-strict. Under the infinite-trace profile QSpec
FR-092 admits the unbounded past operators previous, historically, once and
since, with previous false at position zero, and ADR-018 §11 admits interval
operators nested under unbounded ones. ADR-018 IV-7 records that an interval
operator counts positions.

## Decision

### 1. The refinement declaration

A refinement is one declaration in a QSL unit that selects both models. The
spelling below is illustrative; QSpec's shared grammar owns it (QS-1).

```text
refinement <Name> using <infinite-trace profile> abstract <A> concrete <C> {
  population <A>::<pop> from <C>::<pop>;
  object <A>::<Type> from <C>::<Type> {
    <abstract field> = <expression over the concrete object>;
    …
  }
  step <C>::<Type>::<op> -> <A>::<Type>::<op>(<receiver>, <argument>, …);
  step <C>::<Type>::<op> -> stutter;
  step <C>::<Type>::<op> -> any;
  history <C>::<Type>.<field>: <bounded type> = <literal> { on <C>::<Type>::<op>: <expression>; … }
  assume fair weak [each | whole] <C>::<Type>::<op>;
  ensure fair weak [each | whole] <A>::<Type>::<op>;
}
```

| ID | Rule |
| --- | --- |
| RM-1 | **Placement.** The declaration is a `Relation` family declaration (ADR-012 §3, ADR-017 AR-1), parsed at S2 and checked at S3. `<A>` and `<C>` are model aliases of the unit's `model` declarations, or `<A>` is a model alias of a dependency package compiled against supplied libraries (ADR-015). The **abstract subject** is the checked package that holds the abstract model's contract clauses: the declaring package itself, or that dependency's checked package from the S4 closure. The **concrete subject** is the declaring package. Both models' domain packages are admitted at I1. |
| RM-2 | **Population map.** Each abstract population is the key-preserving image of one concrete population: the abstract object with key `k` exists in a state exactly when the concrete object with key `k` exists in the source population. The abstract universe of a population is the concrete universe of its source. S3 refuses an abstract population with no `from` row, or with two, naming it. A concrete population with no abstract image is concrete-only state. |
| RM-3 | **Object map.** Each abstract object type that an abstract population admits is mapped from one concrete object type of its source population. Its most-specific type in the mapped state is the abstract type whose `from` names the concrete object's most-specific type. Each row `f = e` defines abstract field `f` by expression `e`, whose `self` is the concrete object. `e` reads the concrete state through the forms a state-clause body admits (ADR-016 FE-4): field reads, `deref`, `reaches`, history fields (AX-1) and pure function application. A concrete reference to an object of a mapped type denotes the abstract object with the same key, so a field of type `Reference<T_A>` is mapped by an expression of type `Reference<T_C>` where `T_A` is mapped from `T_C`. |
| RM-4 | **Typing.** S3 checks each `e` against the abstract field's declared type with `TypeEnvironment::conforms` (ADR-016 SC-2) and refuses `ill_typed`/`type-mismatch` at `e`. A field named twice refuses `invalid_model_binding`/`conflicting-binding` naming both rows. A field with no row is a **hidden field** (AX-2). |
| RM-5 | **Step map.** Each concrete operation has exactly one `step` row; S3 refuses an operation with none (`missing_declaration`/`missing-name`, naming the operation) or with two (`invalid_model_binding`/`conflicting-binding`). The right side is one of: an abstract operation applied to a receiver and one expression per declared parameter, each over the concrete receiver as `self`, the concrete arguments by parameter name and the concrete pre-state, typed against the abstract parameter types, with `_` for an argument the concrete step leaves open; `stutter`; or `any`. |
| RM-6 | **Fairness rows.** `assume` rows are the concrete fairness set `F_C` and `ensure` rows the abstract fairness set `F_A`, each an ADR-018 FA-1 constraint over its own model's operations. A row with no granularity is `whole`, as ADR-018 FA-6 reads an unmarked constraint; a row that means `each` spells it. Either set may be empty. |
| RM-7 | **Mapping functions.** For a concrete state `s` with history values `h`, `map(s, h)` is the abstract state built by RM-2 and RM-3 from the visible rows. A row whose evaluation is undefined, refused or incomplete at a state makes `map` undetermined there (RE-4). Each `e` is evaluated through the one clause evaluator (ADR-016 FE-3) over `s`'s synthesized observation (ADR-016 ID-10) with a fresh meter, as a value-typed body. |

The refinement declaration enters the checked package as a node, so its
identity enters the `package_id` and every obligation identity that cites it
(ADR-013 O-02, O-09). Its v2 node is QS-4.

### 2. Mapped behaviours and stuttering

| ID | Rule |
| --- | --- |
| RS-1 | **The step check is the semantics.** One layer-5 function decides each concrete step: `check_step(refinement, abstract subject, (s, h), t, (s', h')) -> StepVerdict`. EN-1, the SMT encodings and replay answer to it, as every temporal engine answers to the trace evaluator (ADR-018 SM-1, SM-7). |
| RS-2 | **Initial states.** For each initial state `c0` of the concrete subject, with history initial values `h0`, `map(c0, h0)` equals, by state key, one initial state of the abstract subject. The abstract subject's initial states are its own FR-106 snapshots. |
| RS-3 | **A `stutter` step** passes when `map(s', h') = map(s, h)` by state key. |
| RS-4 | **An abstract-operation step** passes when the abstract transition identity it names (the abstract operation; the receiver; the arguments evaluated at `(s, h)`, each `_` ranging over its parameter domain) is a transition from `a = map(s, h)` to `a' = map(s', h')` under FR-120: the effective precondition holds at `a`, `StateModel::check_frame` over `(a, a')` returns a delta, and some value of the result domain makes the effective postcondition true over `(a, a', delta)`. These are the same calls `ModelSystem` makes when it expands `a`, applied to one candidate. |
| RS-5 | **An `any` step** passes when it passes RS-3 or passes RS-4 for some abstract transition identity enabled at `a`. |
| RS-6 | **Terminal stutter.** The concrete terminal stutter step (ADR-018 SM-4) is a `stutter` step. |
| RS-7 | **Abstract progress.** Abstract behaviours are maximal paths (ADR-018 SM-3), so a mapped behaviour stutters forever only at an abstract terminal state. A concrete behaviour violates abstract progress when, from some position onward, every step is a `stutter` step and some abstract transition is enabled at the (constant) mapped state. Infinite stuttering at an abstract terminal state is the abstract terminal stutter. A concrete terminal state is the case where the stuttering is the concrete terminal stutter (RS-6): it violates RS-7 when its mapped state is not abstract-terminal. This is the divergence condition of QSpec FR-177's relation, with divergence observable. |
| RS-8 | **Refinement.** The concrete subject refines the abstract subject under the declaration exactly when RS-2 holds and every behaviour of the concrete subject that is fair under `F_C` passes RS-3 to RS-5 at every step, satisfies RS-7, and satisfies `F_A` on its mapped behaviour (CO-2). Each concrete step maps to one abstract step or to a stutter. |

Under this definition the mapped behaviour, with its stutter positions
removed, is a behaviour of the abstract subject. Removing stutter positions
removes no abstract state change, and RS-7 keeps the result maximal.

**Deadlocks.** ADR-018 DL-1 to DL-7 define them per model. A state model
declares its intended terminal states with `terminal when P`, or declares
every terminal state intended with `terminal any`, the opt-out. A deadlock is
a reachable terminal state not marked intended (DL-2), and the request writer
adds one derived deadlock-freedom item per model subject unless that model
declares `terminal any` (DL-3). Deadlock-freedom and RS-7 answer different
questions and stay separate:

| ID | Rule |
| --- | --- |
| RS-9 | **Separate items.** The refinement item's subject is the concrete subject, so a request with a refinement item gets the concrete model's deadlock-freedom item (DL-3). The abstract subject gets its own deadlock-freedom item when the request has a temporal item over it. The refinement item reads neither model's `terminal` member, and its verdict is the same under every combination of them. |
| RS-10 | **A concrete terminal state in the refinement.** The refinement classifies a concrete terminal state by its mapped state. If the mapped state is abstract-terminal, the concrete terminal stutter matches the abstract terminal stutter and the refinement holds there, whether or not the abstract model marks that state intended. If not, the item settles `refuted` with failure `Divergence` (RS-7, RC-1): the concrete model stops where the abstract model continues, which is a refinement failure whatever either `terminal` member says. |
| RS-11 | **The abstract `terminal` member maps by authoring.** The concrete deadlock-freedom item reads only the concrete model's own `terminal` member, which is part of the concrete subject (DL-1). An author can write the concrete member as the abstract predicate read through the mapping: `terminal when P_A'`, where `P_A'` is `P_A` with each abstract field replaced by its RM-3 row. This is possible when `P_A` reads only fields mapped by rows over concrete fields, since hidden fields and history fields have no expression in the concrete package. With that member, the refinement proved and the abstract deadlock-freedom item proved, the concrete deadlock-freedom item follows: RS-10 puts every concrete terminal state on an abstract-terminal state, and abstract deadlock-freedom makes `P_A` true there. EN-1 still checks the concrete item directly. Recording it from the two premises is CO-5's question. |
| RS-12 | **Combinations.** Neither model declares `terminal any`: a concrete halt at an abstract-terminal state is reported by the concrete deadlock-freedom item unless the concrete `terminal when` covers it, and by the abstract item (when present) unless `P_A` covers the mapped state; the refinement holds. Abstract `terminal any`, concrete not: the refinement holds, and the concrete item reports the halt until the concrete model covers it with `terminal when` or opts out. Concrete `terminal any`, abstract not: no concrete item; the refinement holds, and the abstract item reports the mapped state if `P_A` does not cover it. Both `terminal any`: no deadlock-freedom items. In every case RS-10 refutes a concrete halt where the abstract model can move. |
| RS-13 | **Stuttering is not a deadlock.** A concrete state where only `stutter`-mapped operations are enabled (such as `peek` in §8) is not terminal, so it is no deadlock. RS-7 refutes it when a loop there is fair under `F_C` and the mapped state is not abstract-terminal. |

### 3. What carries over

| ID | Rule |
| --- | --- |
| CO-1 | **Safety.** RS-2 to RS-5 are a safety property of the concrete subject: every violation is a finite prefix ending at the first failing step. Over a functional mapping (no hidden field), EN-1 decides it in its first phase by running `check_step` on every product edge it explores, the way it detects a rejecting monitor state (ADR-018 EN-1). |
| CO-2 | **Liveness.** `F_A` holds on a mapped behaviour when each constraint satisfies ADR-018 FA-3 read through the mapping: an abstract transition identity is **enabled** at a position when it is enabled at that position's mapped state (ADR-018 FA-2, evaluated by the abstract subject's `ModelSystem`), and it is **taken** at a step when that step is an abstract-operation step for it (RS-4) or an `any` step that RS-5 matches to a transition of that constraint. RS-7 and `F_A` are liveness properties of the concrete subject under `F_C`. EN-1 checks them in its second phase, with the concrete fairness filter `F_C` on each SCC (ADR-018 FA-4). |
| CO-3 | **Transfer of abstract claims.** When the abstract subject satisfies an infinite-trace clause `P` under fairness set `F_P` where `F_A` implies every constraint of `F_P` (the constraint is in `F_A`, or it is the `whole` form of an `each` constraint in `F_A`, since `each` implies `whole` for one operation), and the concrete subject refines it, the concrete subject satisfies `P` read through `map` under `F_C`. `P`'s atoms read abstract state only, and `P`'s `over` parameter ranges over the images of the concrete objects (RM-2). Transfer covers the **stutter-invariant fragment**: infinite-trace clauses built from `always`, `eventually`, `until`, `release`, `historically`, `once`, `since` and `triggered` over state-predicate atoms, with no interval operator and no previous operator. On that fragment, inserting a step that repeats a state changes no truth value (QSpec FR-161's until and since are non-strict), and that is what makes transfer sound. A clause outside the fragment carries over only by being checked on the concrete subject directly (CO-4). |
| CO-4 | **Clauses with an interval or previous operator are checked on each model.** Transfer is per clause, and a clause carries over exactly when it is in CO-3's stutter-invariant fragment. **Previous** reads the position before, so at a concrete step mapped to `stutter` it reads the same abstract state where the abstract behaviour would read the state before the last abstract step: `previous p` can be true on the abstract behaviour and false on the concrete one at the corresponding position. Strong previous is `once[1,1]` (QSpec FR-092), an interval operator. An interval operator counts event positions (ADR-018 SM-3, IV-7), and a concrete behaviour has more positions than its mapped abstract behaviour, so a stutter step changes which states an interval covers. This covers bounded-profile clauses and **mixed** infinite-trace clauses, whose interval operators are nested under unbounded ones (ADR-018 IV-1 admits them under the infinite-trace profile). In a mixed clause the part that does not carry is every interval or previous subformula, and with it every operator whose scope contains one; in `always (fail implies eventually always[0,10] healthy)` that is `always[0,10] healthy`, and so the whole clause. Such a clause about the concrete model is checked on the concrete subject directly (ADR-018 TP-2 for a bounded clause, TP-3 or TP-4 by its form for an infinite-trace one). An author who wants a part inside the fragment transferred writes it as its own clause. |
| CO-5 | **Recording a transferred verdict.** A transferred claim is a claim over the concrete subject that names its two premises: the abstract claim and the refinement. It settles `proved` exactly when both premises settled `proved` and the abstract claim's subject is the abstract subject this refinement names. Its basis is the weaker of the two premise bases. Any other premise combination leaves it to be checked directly on the concrete subject. QS-7 asks QSpec where this vertical composition lives. |

**Step labels.** QSL behaviours carry each step's transition identity
(ADR-018 SM-3), and the step map reads it. So a refinement can require that a
particular concrete operation is a particular abstract operation, which a
purely state-based mapping states only through auxiliary variables. §8
shows a lost update that the step map refutes and a state-only mapping
(`any`) accepts.

### 4. Auxiliary state

A refinement mapping can be missing for two reasons. The abstract state can
depend on the concrete past, which the concrete state no longer holds; TLA+
adds a history variable. Or the abstract model resolves a choice earlier
than the concrete model does; TLA+ adds a prophecy variable. Both change the
concrete specification, and each needs a side argument that the change
removes no concrete behaviour. QSL keeps that argument inside the checker.

| ID | Rule |
| --- | --- |
| AX-1 | **History fields.** A `history` row adds a field to every object of a concrete type, for this refinement only. It has a bounded declared type, an initial value literal (also the value of an object created by a step), and at most one update per concrete operation. An update is an expression over the step's receiver as `self` (post-state fields), `pre(...)` of pre-state fields and of the history field, and the step's arguments. A step whose operation has no update keeps every history value; an update writes only the receiver's history field. S3 type-checks updates as RM-4 does. History fields never appear in the concrete package, its clauses or its successor relation. |
| AX-2 | **Hidden fields.** An abstract field with no RM-3 row is hidden. EN-1 tracks, per product state, the set of abstract states whose visible fields equal the mapped ones and that are consistent with the behaviour so far: initially the abstract initial states matching `map(c0)`; after a `stutter` step the same set, provided the visible part is unchanged; after an abstract-operation or `any` step every RS-4 or RS-5 successor of a member whose visible part equals `map(s', h')`. A step whose set becomes empty fails. |
| AX-3 | **Soundness of history.** History values are a function of the concrete prefix, and updates gate no step. So the product of the concrete subject with its history fields has exactly the concrete subject's behaviours, each with one history annotation. An update that leaves its declared type or evaluates undefined, refused or incomplete settles the item `inconclusive`, `MappingUndetermined` (RE-4), and never removes the step. |
| AX-4 | **Hidden fields replace prophecy for safety.** The set in AX-2 keeps every abstract choice alive until a later concrete step rules it out, so a safety refinement that would need a prophecy variable is checked exactly, with no change to the concrete model. The cost is the set size, bounded by the hidden fields' finite domains and by FR-101's `Limits`. |
| AX-5 | **Liveness needs a functional mapping.** A refinement with a hidden field and a non-empty `F_A` checks its safety half as AX-2 states and settles its liveness half `unsupported` (V-8), cause `unsupported-requested-capability`, since fair-behaviour inclusion against a nondeterministic abstract behaviour set needs Büchi complementation. RS-7 is checked: a loop of `stutter` steps violates it when every member of the (constant) set has an enabled abstract transition. The author's routes are to map the hidden field (with history fields where the past determines it), or to check the abstract liveness claim on the concrete subject directly through its visible fields (CO-4's direct route). |

**Example of each.** An abstract `Counter` with a field `total` counting
increments, refined by a concrete counter that keeps only `value`, maps
`total = self.commits` with `history Impl::Counter.commits: Int[0, 4] = 0 {
on Impl::Counter::commitA: pre(self.commits) + 1; … }`. An abstract coin whose
`toss` fixes a field `side`, refined by a concrete coin whose `reveal`
chooses the shown side, leaves `side` hidden: after the concrete `toss` the
set holds both sides, and the concrete `reveal` prunes it to the one shown.

### 5. Engines and verdicts

| ID | Engine | Checks | Verdicts |
| --- | --- | --- | --- |
| RE-1 | **EN-1, refinement product.** The product `TransitionSystem` (ADR-018 EN-1) whose state is (concrete state, history values, monitor state), plus the AX-2 set when hidden fields exist; its key adds the history values and the set's sorted abstract state keys. Phase 1 runs `check_step` on each retained edge (CO-1). Phase 2 runs SCC decomposition on the retained graph with the Büchi automaton for the negation of RS-7 and `F_A` (CO-2) and the `F_C` filter. | the whole refinement, RS-8 | V-1, V-4, V-5, V-6, V-7; V-8 for AX-5 |
| RE-2 | **SMT per-step simulation.** One `operation-contract` obligation per `step` row and one for RS-2, as QSpec FR-290's state-model refinement row states. For a step row of operation `o`: the concrete invariants, `o`'s effective precondition, its frame and its postcondition over `(s, s')` imply RS-3 or RS-4 over `(map(s), map(s'))`. The concrete package's invariant clauses are the induction hypothesis; each is its own `operation-contract` record. | safety half, functional mappings, step rows other than `any` | V-3 with `Inductive{depth: 1}` and basis `decisive-witness`; V-6 `InductionNotClosed{depth: 1}` when a step obligation fails from a pre-state that satisfies the invariants |
| RE-3 | **SMT unrolling (ADR-018 EN-2)** with the RS-1 edge monitor | safety half refutation, liveness half lasso refutation | V-4, V-5 |
| RE-4 | **Verdict causes.** `InconclusiveCause` gains `MappingUndetermined`: a mapping row, an argument expression or a history update was undefined, refused or incomplete, or a history update left its declared type, at a reachable state. `UndecidedSuccessor` (ADR-018 V-6) covers an undecided contract conjunction of either subject during expansion or `check_step`. | | V-6 |

**Claim kinds.** The whole refinement is one requirement record of kind
`temporal-satisfaction`: it is a temporal property of the concrete subject,
`init ∧ □[step map]_map ∧ progress ∧ F_A` read through the mapping, and
EN-1's provider manifest already advertises that kind (ADR-018 §3). RE-2's
per-step obligations are `operation-contract` records, which is QSpec
FR-290's existing row. QSpec FR-290's `refinement` kind stays the
`protocol`-family member of QSpec FR-177. No capability kind is added.

**Strength.** RE-1's `proved` holds for the concrete subject (its initial
states and universes) against the abstract subject with the derived
universes (RM-2), as ADR-018 §1 "Scope" states. RE-2's `proved` holds for
every state of the declared types, so it is independent of universes. An
RE-2 failure from an unreachable pre-state is not a refutation, so RE-2 never
settles `refuted`. A refutation comes from RE-1 or RE-3 and is replayed
(RC-2).

**Negotiation.** CG's EN-1 arm (ADR-018 DS-2) reads the property form; a
refinement record carries form `TP-5 Refinement` with two halves: safety
(RS-2 to RS-5, phase 1) and liveness (RS-7 always, plus `F_A`, phase 2). The
SMT arm routes RE-2 and RE-3 records after IR admits state nodes and transition relations (ADR-018 DS-3).

### 6. Counterexamples and replay

| ID | Rule |
| --- | --- |
| RC-1 | **Content.** A refinement counterexample is ADR-018's `TemporalCounterexample` over the concrete subject (ADR-018 CX-2) with a `RefinementFailure` member: `InitialNotAbstract{initial}` (RS-2); `StutterChanged{position}` (RS-3); `AbstractStepRejected{position, transition, cause}` with cause `Precondition{clause}`, `Frame{code}` or `Postcondition` (RS-4); `NoAbstractMatch{position}` (RS-5, or an empty AX-2 set); `Divergence` (RS-7, a lasso whose loop has only `stutter` steps); `AbstractUnfair{constraint}` (`F_A`, a lasso). A safety failure is a finite prefix ending at its failing step; a liveness failure is a lasso fair under `F_C`. The counterexample carries concrete steps only; replay recomputes mapped states and history values. |
| RC-2 | **Replay.** E9 replay recompiles the concrete package and the abstract subject's package (FR-098, ADR-015), re-executes the concrete steps through the concrete `ModelSystem` as ADR-018 CX-3 does, recomputes history values, `map` and the AX-2 sets along the trace, and reruns `check_step` at the failing position or, for a lasso, checks it fair under `F_C` and evaluates RS-7 or the `F_A` constraint over its loop. Agreement settles `reproduced-with-evaluated-witness` and the item `refuted`. A recomputed failure of a different kind or at a different position settles `inconclusive`, `ReplayParity`. ADR-018 CX-3's refusals apply unchanged. |
| RC-3 | **Source arm.** The packet uses ADR-018's `ReplaySource::ModelTrace` with the refinement node's identity, and the arm result names the `RefinementFailure` it reproduced. |

### 7. One mechanism, with the abstraction relation as its code-side premise

**Decision.** Model-to-model refinement and QSpec FR-177 protocol refinement
are one mechanism: a behaviour-inclusion check over two explored
`TransitionSystem`s through an authored mapping, decided by `check_step` on
the EN-1 product. The model-to-code abstraction relation is a separate
mechanism: a representation binding that supplies the abstraction function
for code-side `operation-contract` claims. It shares only the per-step
obligation shape of RE-2, and it is discharged by CG's backends.

| | Model-to-model (this record) | Protocol refinement (QSpec FR-177) | Abstraction relation (QSpec FR-353) |
| --- | --- | --- | --- |
| Both sides | two checked packages, each a model subject with a `ModelSystem` (FR-120) | two protocol models with an exact correspondence | a model element and a Rust item named by `RustPath` (ADR-017 AR-2) |
| Who explores the sides | QSL layer 5, EN-1 over both | QSL layer 5 once a protocol `TransitionSystem` exists | neither side; Kani and Verus check the code |
| Mapping | `map` from concrete to abstract state, plus the step map | correspondence relation, visible-step matching, internal steps | model key to Rust path; the abstraction function is reading the bound representation |
| QSL check | S3 typing (RM-4, RM-5); `check_step` at S6c | the same `check_step`, with visible observations as step labels and internal actions as `stutter`, plus refusal-set, terminal-success and assumption checks on the same product | S3 key and syntax checks (ADR-017 AR-3); per-item unbound refusal at export (AR-4) |
| Claim kind | `temporal-satisfaction` (RE-1); `operation-contract` per step (RE-2) | `refinement` | none; a premise of the `operation-contract` claim it binds |
| Stage | S3, S4, E10 to S6c; SMT through S5, E7, S6b | S3, S4, then S6c | S3, S4, layer-4 export, then CG at S6b |

| ID | Reason |
| --- | --- |
| MC-1 | **FR-177 is the same check with more observations.** FR-177's "permitted internal steps" are `stutter` rows, its visible-step matching is the step map with the observation as label, its divergence rule is RS-7, and its paired initial states are RS-2. Its refusal sets, terminal-success matching and assumption weakening are further checks over the same product edges and states. One `check_step`, one product and one counterexample type serve both. A protocol engine needs a protocol `TransitionSystem`, which waits on protocol nodes at S4 (ADR-017 PF-7). |
| MC-2 | **The abstraction relation has no explored side.** QSL never resolves a `RustPath` (ADR-017 AR-2), so it cannot run `check_step` on code. Its claims are `operation-contract` claims over code, and Kani and Verus establish each implementation function against its operation's contract through the bound representation. That is RE-2's per-step obligation with the code as the concrete side, discharged at CG. |
| MC-3 | **They compose vertically.** An abstract model refined by a concrete model whose operations are bound to code gives three layers: the abstract claims hold for the concrete model by CO-3, and the concrete model's `operation-contract` claims hold for the code through the abstraction relation. The two mappings stay separate declarations with separate keys. |

### 8. Worked example: a counter refined by a compare-and-set implementation

**Abstract model.** Domain package `example/counter-spec`: object type
`Counter` with `value: Int[0, 2]`, population `counters`, operation `inc()`
with frame `modifies [value]`.

**Concrete model.** Domain package `example/counter-cas`: object type
`Counter` with `value: Int[0, 2]`, `tmpA: Int[0, 2]`, `busyA: Boolean`,
`tmpB: Int[0, 2]`, `busyB: Boolean`, population `counters`. Two workers, A
and B, each read `value` into a scratch field, then write it back with a
compare-and-set. Operations: `beginA()` with frame `modifies [tmpA, busyA]`,
`commitA()` with `modifies [value, busyA]`, `retryA()` with `modifies
[busyA]`, the same three for B, and `peek()` with an empty frame.

```text
profile v   = "quire.value.complete/v1" …;
profile inf = "quire.temporal.infinite-trace/v1" …;
model Spec = "example/counter-spec" …;
model Impl = "example/counter-cas" …;

pre  CanInc     using v on Spec::Counter::inc     { self.value < 2 }
post Inc        using v on Spec::Counter::inc     { self.value = pre(self.value) + 1 }

pre  BeginAPre  using v on Impl::Counter::beginA  { not self.busyA }
post BeginAPost using v on Impl::Counter::beginA  { self.busyA and self.tmpA = pre(self.value) }
pre  CommitAPre using v on Impl::Counter::commitA { self.busyA and self.tmpA = self.value and self.value < 2 }
post CommitAPost using v on Impl::Counter::commitA { self.value = pre(self.tmpA) + 1 and not self.busyA }
pre  RetryAPre  using v on Impl::Counter::retryA  { self.busyA and self.tmpA != self.value }
post RetryAPost using v on Impl::Counter::retryA  { not self.busyA }
// the same six clauses for B over tmpB and busyB

refinement CasRefinesCounter using inf abstract Spec concrete Impl {   // illustrative spelling (QS-1)
  population Spec::counters from Impl::counters;
  object Spec::Counter from Impl::Counter {
    value = self.value;
  }
  step Impl::Counter::commitA -> Spec::Counter::inc(self);
  step Impl::Counter::commitB -> Spec::Counter::inc(self);
  step Impl::Counter::beginA  -> stutter;
  step Impl::Counter::beginB  -> stutter;
  step Impl::Counter::retryA  -> stutter;
  step Impl::Counter::retryB  -> stutter;
  step Impl::Counter::peek    -> stutter;
  assume fair weak each Impl::Counter::beginA;   // and beginB, commitA, commitB, retryA, retryB, each spelled
  ensure fair weak each Spec::Counter::inc;
}
```

Every fairness row spells `each`, the granularity meant. With one receiver
and no parameters each operation has one transition identity, so `whole`
would give the same verdicts here; the spelling stays explicit because an
unmarked row is `whole`.

**Subjects.** Universe `counters = {c}`. Abstract initial snapshot: `c` with
`value` 0. Concrete initial snapshot: `c` with every integer field 0 and both
`busy` fields false. Write a concrete state as `(value, tmpA, busyA, tmpB,
busyB)` with `t` and `f` for the Booleans. The mapped state is `value`. RS-2
holds: the concrete initial state maps to `value` 0, the abstract initial
state.

**A stuttering step.** `beginA` from `(0, 0, f, 0, f)` reaches `(0, 0, t, 0,
f)`. Both map to `value` 0, so the `stutter` row passes RS-3. The concrete
model took a step; the abstract model did not move.

**The refinement holds.** The concrete subject has 28 reachable states. Every
edge passes `check_step`: each `begin`, `retry` and `peek` leaves `value`
unchanged, and each `commit` is enabled only when its scratch value equals
`value` and `value < 2`, so it writes `value + 1`, which `CanInc` and `Inc`
admit at the mapped state. Phase 2 finds no SCC that passes the `F_C` filter
while staying at a mapped `value` below 2 with only `stutter` steps: in each
such SCC some `begin`, `commit` or `retry` constraint is enabled at every
state and taken in none. At `value` 2, `inc` is disabled at the mapped state,
so RS-7 admits stuttering there. The
item settles `proved`, basis `closed-scope`, `TerminalValue::Proved{basis:
Exhaustive}` (ADR-018 V-1). By CO-3, an abstract claim such as `always
eventually holds(c.value = 2)`, proved on the abstract subject under `fair weak each Spec::Counter::inc`, holds for the
concrete subject under `F_C` read through `map`.

**A broken refinement: lost update.** Change both commit preconditions to
`self.busyA and self.tmpA < 2` (and the B analogue), dropping the
compare-and-set guard, and drop the `retry` operations and rows. The commit
now writes its stale scratch value plus one. The concrete subject has 36
reachable states. EN-1's phase 1 settles `refuted` (ADR-018 V-4) with this
prefix, the first failing edge in canonical breadth-first order:

| Position | Concrete state | Mapped `value` | Step into this position | `check_step` |
| --- | --- | --- | --- | --- |
| 0 | `(0, 0, f, 0, f)` | 0 | initial | RS-2 holds |
| 1 | `(0, 0, t, 0, f)` | 0 | `beginA` → `stutter` | passes RS-3 |
| 2 | `(0, 0, t, 0, t)` | 0 | `beginB` → `stutter` | passes RS-3 |
| 3 | `(1, 0, f, 0, t)` | 1 | `commitA` → `inc(c)` | passes RS-4: `CanInc` at 0, frame, `Inc` from 0 to 1 |
| 4 | `(1, 0, f, 0, f)` | 1 | `commitB` → `inc(c)` | fails RS-4: `CanInc` holds at 1 and the frame admits the pair, but `Inc` needs `value` 2 |

The failure is `AbstractStepRejected{position: 4, transition: inc(c),
cause: Postcondition}`. Replay (RC-2) re-executes the four steps, recomputes
the mapped values 0, 0, 0, 1, 1, reruns `check_step` at position 4 and gets
the same failure.

Had the author written `step Impl::Counter::commitB -> any`, position 4
would pass RS-5 as a stutter, and the lost update would be invisible to the
refinement: the mapped behaviour 0, 0, 0, 1, 1 is an abstract behaviour up to
stuttering. The step map is what makes "every commit is an increment" part of
the claim.

**A broken refinement: divergence.** Keep the compare-and-set model and
replace the `assume` rows by `assume fair weak each Impl::Counter::commitA;
assume fair weak each Impl::Counter::commitB;`. Phase 2 finds the SCC `{(0,
0, f, 0, f)}` with its `peek` self-loop: both commits are disabled there, so
the loop is fair under `F_C`, every step is `stutter`, and `inc` is enabled
at the mapped state 0. The item settles `refuted` with the lasso whose stem is
empty and whose loop is one `peek` step, failure `Divergence`.

**SMT per-step (RE-2), when the SMT path lands.** For the compare-and-set
model each step obligation holds with no invariant, and each `step` record
settles `proved` with `Inductive{depth: 1}`. For the lost-update model the
`commitB` obligation fails from `(1, 0, f, 0, t)`, which satisfies every
concrete invariant (there are none), and settles `inconclusive`,
`InductionNotClosed{depth: 1}`; EN-1's refutation above is the replayable
counterexample.

### 9. Downstream impact and sequencing

**Stage DAG (ADR-011 §1).** No stage or edge is added. The refinement
declaration is an S2 form and an S3 checked node, emitted at S4. Its
`temporal-satisfaction` record routes to EN-1 at ADR-018's S6c over E10;
S6c receives the abstract subject's checked package from the S4 dependency
closure or from the declaring package itself (RM-1). RE-2 and RE-3 records run
on S5 → E7 → S6b. A refutation leaves S6c as an S7 typed witness and replays
through E9 at S8.

**Module DAG (ADR-011 §6.1).** Layer 3 `check` gains the refinement
declaration's checker and `CheckedRefinement`. Layer 5 `model_check` gains
`check_step`, the history and AX-2 product extensions, and the RS-7 and
`F_A` automata. Layer 6 `replay` gains RC-2 inside the `ModelTrace` arm. The
mapping-row evaluation is a value-typed use of the FE-3 clause evaluator.

| ID | Repository | Change |
| --- | --- | --- |
| DS-1 | QSL | S2 form and S3 checks (RM-1 to RM-6, AX-1 typing); the S4 node; layer-5 `check_step`, the refinement product (RE-1), history fields, the AX-2 set, the RS-7 and `F_A` automata, `MappingUndetermined`; `RefinementFailure` in `TemporalCounterexample`; RC-2 in replay. |
| DS-2 | CG | The EN-1 arm reads form TP-5 and its halves; the SMT arm routes RE-2 per-step `operation-contract` records and RE-3. |
| DS-3 | IR | The SMT encoding of RE-2's step obligation (two models' transition relations and the mapping as terms), with ADR-018 DS-3's state-node admission. |
| DS-4 | Driver | Supplies the abstract subject's package to S6c and to E9 replay. |
| DS-5 | QSpec | §10. |

**Sequencing.**

1. **Specification.** This record, then QSpec (§10), then the QSL compiler
   requirements.
2. **Prerequisite.** ADR-018 steps 2 and 3: FR-120 `ModelSystem`, the
   temporal spine and EN-1 with its SCC phase. Refinement adds nothing to
   that critical path.
3. **Safety half on EN-1.** RM-1 to RM-7, RS-1 to RS-6, phase-1 `check_step`,
   RC-1 and RC-2 for prefix failures, and the §8 holding and lost-update
   cases as the first conformance vectors. It can start as soon as EN-1's
   phase 1 exists.
4. **Liveness half on EN-1.** RS-7 and `F_A` on phase 2, the divergence
   vector. Strong fairness in `F_C` or `F_A` comes with the strong-fairness
   feature (ADR-018 FA-5) with no change here.
5. **Auxiliary state.** History fields (AX-1, AX-3) and hidden fields (AX-2,
   AX-4, AX-5). Independent of step 4.
6. **Transfer** (CO-3, CO-5), once QSpec answers QS-7.
7. **SMT per-step simulation** (RE-2, RE-3), after IR admits state nodes and
   transition relations and the SMT backend exists (ADR-018 step 5).
8. **Protocol refinement** (QSpec FR-177) on the same product, once protocol
   nodes reach S4 and a protocol `TransitionSystem` exists (MC-1).

State-space reduction (ADR-018 §7) applies to the refinement product with
one restriction: symmetry over universe keys is sound when the mapping and
the step map treat keys uniformly, which every RM-2 key-preserving map does.

### 10. What QSpec must specify

| ID | Item | Where |
| --- | --- | --- |
| QS-1 | Surface syntax of the refinement declaration in the shared grammar: the header with abstract and concrete model aliases and profile; population, object and field rows; step rows with abstract operation application, `_` arguments, `stutter` and `any`; history rows; `assume` and `ensure` fairness rows, an unmarked granularity read as `whole` | shared grammar |
| QS-2 | Refinement semantics: RS-2 to RS-8 over model subjects, the key-preserving population map, reference lifting through object maps, hidden fields as existentially chosen abstract state, the rule that a refinement verdict is independent of either model's deadlock opt-out with a concrete terminal state classified by its mapped state (RS-9 to RS-13), and how an abstract `terminal when` predicate is written for the concrete model (RS-11), and the abstract progress rule | a new QSpec FR, citing QSpec FR-181 and FR-161 |
| QS-3 | History fields: initial value, per-operation update, receiver-only writes, and the rule that an update never removes a step (AX-1, AX-3) | the same FR |
| QS-4 | The `quire.checked-package/v2` node for a refinement declaration and its place in the obligation identity | QSpec v2 contract, FR-331 |
| QS-5 | Claim kinds: the whole refinement as `temporal-satisfaction`; the per-step obligations as the existing `operation-contract` row, with the step obligation of RE-2 stated as that row's "implication between the two contracts' clauses"; the `refinement` kind's scope as the `protocol` family only | QSpec FR-290 |
| QS-6 | Counterexample wire: the `RefinementFailure` member and its six kinds; replay rules of RC-2; the `MappingUndetermined` inconclusive cause | QSpec FR-331 and the counterexample contract; QSpec FR-341 (infinite-trace) |
| QS-7 | Vertical composition: a claim over a concrete subject discharged from an abstract claim and a refinement (CO-3, CO-5); the per-clause transfer rule over the stutter-invariant fragment, under which a clause with an interval operator (bounded or mixed) or a previous operator does not carry (CO-3, CO-4); fairness implication between `F_A` and the claim's fairness set; its premises, its basis, and whether QSpec FR-348 covers it or a new FR does | QSpec FR-348 or a new FR |
| QS-8 | QSpec FR-177's relation stated over the same step rules: internal actions as `stutter`, visible steps as step-map rows with observations, divergence as RS-7, with refusal sets, terminal success and assumption weakening as the protocol-only additions (MC-1) | QSpec FR-177 |
| QS-9 | Conformance vectors, each with expected verdict and, for a refutation, a counterexample that must replay: (a) §8's holding case, lost-update case and divergence case; (b) a `step … -> any` variant of the lost-update model that holds; (c) an initial-state mismatch; (d) a `stutter` step that changes a mapped field; (e) a history-field refinement that holds, and the same refinement with the row for the history-backed abstract field removed, so that field is hidden and, with an `ensure` row, the liveness half settles `unsupported`; (f) a hidden-field safety refinement that holds where every functional mapping fails (the coin of §4); (g) a mapping row that evaluates undefined (`MappingUndetermined`); (h) replay refusals and a `ReplayParity` case; (i) a concrete halt at an abstract-terminal state, with the same refinement verdict under every combination of the two models' `terminal` members, and with the concrete deadlock-freedom item proved under `terminal when` written from the abstract predicate (RS-11); (j) a concrete halt at a state where the abstract model can move, refuted `Divergence` under every combination of `terminal` members; (k) a mixed clause and a clause using previous, each proved on the abstract subject, that do not transfer, and are checked directly on the concrete subject, with a model where the previous clause's concrete verdict differs | QSpec TC-200 neighbourhood and new TCs |

## Consequences

- An author can verify a design in layers: prove claims on a small abstract
  model, then show each concrete model refines it and inherit the
  infinite-trace claims through the mapping.
- Refinement is checked by the same engine, product and counterexample type
  as every other temporal claim over a model, and its counterexamples replay
  as ordinary model traces.
- History values and hidden abstract fields live in the refinement
  declaration and the checker. The concrete model, its clauses and its other
  verdicts are unaffected by them, and the checker guarantees they remove no
  behaviour.
- Step labels make operation correspondence part of the claim, so bugs that a
  state-only mapping would accept as stuttering are refuted.
- Liveness refinement with hidden abstract state settles `unsupported`; the
  author maps the field or checks the claim directly.
- QSpec FR-177's protocol refinement has a defined implementation path that
  reuses this record's check.

## Amendments to make on acceptance

- **ADR-011** §1 S6c row (as ADR-018 adds it): S6c also reads the abstract
  subject's checked package for a refinement record. §6.1: layer 5
  `model_check` holds `check_step` and the refinement product; layer 3
  `check` holds the refinement checker.
- **ADR-017** RF-1: QSpec FR-290's "Refinement between two operation
  contracts or state models" row is owned by ADR-020 RE-2; the spec-versioning
  and profile-layering gates stay corpus tests. §3 AR-1: the refinement
  declaration is a second `Relation` family declaration, distinct from the
  abstraction relation, with its own checked type `CheckedRefinement`. §5
  PF-7 change scenarios: protocol refinement lands on ADR-020's product once
  protocol nodes reach S4 (MC-1).
- **ADR-018** §1: property form TP-5 Refinement, settled by EN-1 (both
  halves), the SMT backend per-step (RE-2) and SMT unrolling (RE-3). V-6:
  `MappingUndetermined`. §5 CX-2: the optional `RefinementFailure` member.
  §7 item 4 "Refinement mappings": replaced by a reference to this record;
  the stuttering it anticipates is RS-3, RS-6 and RS-7.
- **ADR-012** §3 `Relation` row: the refinement declaration and its S3
  checks.
- `spec/spec.md`: index row.

## Alternatives Considered

- **State-only mappings, as in TLA+.** Rejected as the only form. QSL
  behaviours carry transition identities, so the step map is available at no
  cost and refutes bugs that a state-only mapping reads as stuttering (§8).
  `any` keeps the state-only reading for a step where the author wants it.
- **History and prophecy variables as ordinary fields of the concrete
  model.** Rejected. They would enter the concrete package, its successor
  relation and every other verdict about it, and the side argument that they
  remove no behaviour would be the author's. AX-1 and AX-3 keep history out of
  the model and make the argument a checker rule; AX-2 replaces prophecy for
  safety.
- **A relational mapping written as a Boolean predicate over concrete and
  abstract states.** Rejected. Evaluating it needs enumeration of abstract
  candidates at every state. Hidden fields give the same expressiveness
  where it matters, with the visible part still a typed function.
- **One concrete step covering several abstract steps.** Rejected for this
  record. Each concrete step is one abstract step or a stutter, as in TLA+;
  a concrete model that batches abstract steps is refined through an
  intermediate model.
- **A new capability kind for model refinement.** Rejected. The whole
  refinement is a temporal property of the concrete subject, and the
  per-step form is QSpec FR-290's existing `operation-contract` row.
- **One mechanism covering the abstraction relation as well.** Rejected. QSL
  explores neither side of a code binding (MC-2); forcing it into
  `check_step` would need QSL to run Rust.
- **Transferring a mixed clause with its intervals counted in abstract
  steps.** Rejected. That reading counts only the concrete steps that are not
  stutter steps, which is not a formula the trace evaluator states over the
  concrete behaviour (ADR-018 SM-1), so its verdict could not be re-checked or
  replayed on the concrete subject. CO-4 checks such a clause directly.
- **Abstract initial condition as a predicate.** Rejected. Model subjects
  have initial snapshots (ADR-018 SM-2), and RS-2 compares by state key.

## Open questions

1. **Totality of the step map.** RM-5 requires a row for every concrete
   operation. An unmapped operation could instead default to `any`, TLA+'s
   reading. Explicit rows make the claim visible; confirm.
2. **Abstract progress as a default.** RS-7 follows ADR-018 SM-3's maximal
   behaviours, so infinite concrete stuttering at an abstract state that can
   move is a refutation even with an empty `F_A`. TLA+ admits it unless the
   abstract specification has fairness. Confirm that QSL keeps maximality.
3. **Transfer recording.** CO-5 records a transferred claim from two proved
   premises. Whether this waits for QSpec (QS-7) or QSL records it locally
   first.
4. **Liveness with hidden fields.** AX-5 settles it `unsupported`. A later
   engine could complement the abstract Büchi automaton over finite hidden
   domains. Confirm `unsupported` is acceptable for the first version.
5. **Aggregating mappings.** RM-3 reads one concrete object per abstract
   object. An abstract object derived from a whole concrete population (a
   queue from ring-buffer slots) needs a population-valued expression in a
   clause body, which ADR-016 FE-4 does not admit. Whether to ask QSpec for
   one now.

## References

- Owning ticket: Linear QSL-367. Built on ADR-018 (Linear QSL-366). Sibling
  features on EN-1: QSL-365 (strong fairness), QSL-368 (state-space
  reduction), QSL-369 (EF and AG EF), QSL-370 (hyperproperties).
- ADR-017's refinement gates and abstraction relation: Linear QSL-40,
  QSL-39 and QSL-36; CG Verus and Kani consumers IR-32 and IR-93; the planned
  SMT backend and its parity corpus IR-33.
- The paired QSpec STD ticket is filed with this record's spec phase.
