---
id: ADR-021
title: "State-space reduction: symmetry, partial-order reduction and state constraints"
type: ADR
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
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
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-115
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: relates_to
  - target: ix://agent-ix/quire-specification/FR-013
    type: depends_on
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
---
# ADR-021: State-space reduction: symmetry, partial-order reduction and state constraints

## Context

ADR-018's explicit-state engine EN-1 explores the product of a model
subject's state graph with a property automaton on FR-101's canonical
breadth-first engine, retains every product edge, and finds fair accepting
cycles by SCC decomposition. Its memory is proportional to the reachable
product. The subject's size grows as the product of its universes' sizes and
its field domains, so a model with three interchangeable objects of a dozen
states each already has 1,728 model states before any automaton. QSL bounds
the subject with universes, `bounded_domain` ranges and `ProofBound`s
(ADR-014 B-1, B-4), and it decomposes claims with induction and composition.
It has no reduction of the explored graph.

Three standard reductions apply to explicit-state checking.

- **Symmetry.** When permuting the identities of interchangeable objects maps
  the model onto itself, states that differ only by such a permutation have
  the same future up to the permutation, and the search stores one state per
  orbit (Ip and Dill; Emerson and Sistla 1996).
  TLC offers it as a `SYMMETRY` set, which it does not check, and states
  that temporal checking under it may miss errors (Lamport, §14.3.4).
- **Partial-order reduction (POR).** When two transitions are independent
  (neither disables the other and they commute), many interleavings of them
  reach the same state, and the search expands a subset of the enabled
  transitions at a state (ample sets: Peled 1993, Clarke, Grumberg and
  Peled; persistent sets: Godefroid; stubborn sets: Valmari). It preserves
  properties invariant under stuttering. SPIN implements it (Holzmann and
  Peled).
- **State constraints.** A predicate that cuts the search at states outside
  it. TLC's `CONSTRAINT` makes a pass a pass over the constrained graph only.

What QSL has that bears on each:

- **References carry no order.** The S3 checker refuses `<`, `<=`, `>` and
  `>=` on `Reference` operands (`qsl-semantics/src/check/check.rs`, the
  ordering arm). A state clause reads model state through field reads,
  `deref` and `reaches` over its observation, and no state-clause source
  produces a population value (ADR-016 FE-4). A reference is observed only by
  equality and dereference.
- **Universes are keys.** FR-120's `PopulationUniverse` gives each simulated
  population a finite key set; the state key encodes a reference as FR-181's
  triple with the key's UTF-8 bytes as identity, and `Set` and `Bag` contents
  are sorted by their JCS bytes, so the key order of the encoding follows
  identity bytes.
- **Frames are type-wide.** QSpec FR-013 grants a `modifies` field "on every
  object of the invocation's pre and post populations whose effective type
  has the field". FR-120's `check_frame` enforces that frame on every
  successor; the checked-package v2 frame entry IR FR-040 admits names a
  declaration and a field, with no object scope. Nothing computes the fields a
  clause reads.
- **`parallel` is a protocol construct.** ADR-027's `ProtocolSystem` explores
  protocols, `parallel` included, with static footprints per step (ADR-027
  FT-1 to FT-5). FR-120's `ModelSystem` is the model-level system (ADR-016
  FP-4); its successor relation already interleaves every operation on every
  receiver, which is where independent transitions occur in a state model.
- **QSpec FR-181-AC-4** requires exhaustive exploration to coalesce "only
  equal complete state keys", which a symmetry quotient does not do.
- **The infinite-trace grammar has no next or previous operator** and its
  until is non-strict (QSpec FR-161), so an infinite-trace formula with no
  interval operator is invariant under stuttering. ADR-018 IV-1 admits closed
  intervals under the profile, IV-3 unrolls them into next-position steps,
  and IV-7 records that a formula with one counts positions. A bounded-profile
  formula counts positions too.
- **Deadlocks are an item.** For each model subject, the request writer adds a
  derived deadlock-freedom item, a TP-1 invariant `always holds(not
  deadlocked)`, unless the model declares `terminal any`; `terminal when P`
  marks intended terminal states (ADR-018 DL-1 to DL-3).

"QSpec FR-nnn" names a quire-specification requirement; a bare FR id is a QSL
requirement; "CG", "IR" and "RT" prefix the codegen, contract IR and contract
runtime repositories' own artifacts. "QSpec FR-360" is the infinite-trace
result disposition vocabulary, as in ADR-018. Item ids `RD-`,
`SYM-`, `AQ-`, `POR-`, `SC-`, `PT-`, `EI-`, `RV-`, `TX-`, `DS-`, `QS-` and `RU-` are local
to this record. Other artifacts cite them as `ADR-021 SYM-3`. Items of
ADR-018 are cited as `ADR-018 SM-3`.

## Decision

### 1. The reductions and how each is declared

| ID | Rule |
| --- | --- |
| RD-1 | **Three reductions.** EN-1 applies three reductions to a model-check run: symmetry over population universes (§1.1), partial-order reduction over independent transitions (§1.2), and a state constraint (§1.3). Each is selected by the request that routes the item to EN-1, beside the subject's universes and initial states (ADR-018 SM-2). A request may select any combination. |
| RD-2 | **Reductions apply to model checking.** FR-101 `explore` and `sample` and FR-120 `explore_model` keep their unreduced semantics: a simulation reports a finding for each reachable state, as FR-120 states. A reduction changes which product states EN-1 stores, never the truth of a claim on a behaviour (ADR-018 SM-1). |
| RD-3 | **Checked, never trusted.** Every selected reduction is admitted by EN-1's pre-check (EI-1) against the checked package, the subject and the claim before any expansion. A selection the claim's form does not admit, or that the model breaks, settles the item with a named cause (§5). EN-1 never runs a different method than the one requested and never drops a requested reduction. |

#### 1.1 Symmetry over population universes

| ID | Rule |
| --- | --- |
| SYM-1 | **Declaration.** A request opts in to symmetry per universe of a population declared `symmetric` (SYM-8): `SymmetryDeclaration{population, classes}`, where `classes` partitions some or all of that universe's keys into classes of interchangeable keys. One class holding every key is full symmetry. A key in no class, or in a class of one, is fixed. The **symmetry group** `G` is the product, over every declaration, of the symmetric group of each class. A permutation `π` in `G` acts on a model state by renaming every reference whose universe and key it moves, in every field, collection and record, and re-encoding the state key; on a transition identity by renaming its receiver and its reference arguments. |
| SYM-2 | **Well-formed declaration.** A declaration whose population is not declared `symmetric` in the checked package, is not a universe of the request, names a key not in that universe, or puts a key in two classes refuses the request, `invalid_runtime_input`/`invalid-value`, naming the declaration, as FR-120 refuses a malformed universe. |
| SYM-3 | **Identity-transparent forms.** A population is identity-transparent in a subject when no clause the run evaluates observes the identity of its objects other than by equality, dereference (field read, `deref`, `reaches`), membership and passing as a value. The clauses the run evaluates are every precondition and postcondition of every operation, the invariants, the claim's atoms, the model's `terminal when P` predicate (ADR-018 DL-1), the state constraint (§1.3), and, for a refinement item, ADR-020's mapping rows, step-map arguments and history updates. A form that breaks transparency is: any order over references (already ill-typed); a `fold`, `reduce` or other order-sensitive traversal of a `Set` or `Bag` whose element type contains a reference into the population, since the traversal follows the identity-sorted encoding; any conversion of a reference to a value of another type; and a reference literal naming a key. S3 refuses a breaking form that reads a population declared `symmetric` (SYM-8), so a checked package whose symmetric populations admit a request has none. |
| SYM-4 | **Closed initial states.** For every generator of `G` (per class of `m` keys, the transposition of its first two keys, and for `m >= 3` the cycle through all `m` in key order), applying it to each initial state yields a state whose key equals some initial state's key. A finite set closed under the generators is closed under `G`. |
| SYM-5 | **Admission.** A declaration is admitted when SYM-4 holds for `G`; SYM-3 holds already, by the authoring-time refusal of SYM-8. Under admission every `π` in `G` is an automorphism of the subject: `s -t-> s'` is a transition exactly when `π(s) -π(t)-> π(s')` is, the initial states are fixed as a set, and every closed atom has the same truth at `s` and `π(s)`. This is the scalarset argument of Ip and Dill; QSL's typing supplies its premise for references. A declaration whose initial states break SYM-4 settles the item `inconclusive`, `SymmetryBroken` (RV-3), naming the initial state and generator. |
| SYM-6 | **Bindings.** A claim with an `over` parameter has one instance per object of the universe (ADR-018 QS-5). Under `G`, instances whose bound keys lie in one orbit have the same verdict. EN-1 checks one instance per orbit, the one with the least key, and reduces that instance's product by the stabiliser of its binding: the subgroup of `G` that fixes the bound key. The instance's verdict is the verdict of every instance in its orbit. |
| SYM-7 | **Canonical representative.** The canonicaliser sorts (RU-2). For each symmetry class of the run's group (`G`, or the binding's stabiliser), the **sort key** of a key `k` is the JCS bytes of the key-free encoding of the object `k` names: its FR-120 record with every reference into a symmetric class replaced by that class's placeholder, so the encoding names no permuted identity; a key with no object in the state sorts as the empty encoding. The class's objects are sorted ascending unsigned-lexicographic by sort key, equal sort keys keeping their keys' order, and the class's keys are reassigned to them in key order. The result is a permutation `π` in the group, and the canonical form is `π(s)`; the canonicaliser returns both. Two states coalesce exactly when their canonical forms have equal state-key bytes. **Soundness:** `π(s)` is a member of `s`'s orbit, and every member of an orbit has the same behaviours up to the permutation (SYM-5), so storing `π(s)` in place of `s` loses no reachable behaviour; two states coalesce only when they share an orbit. **Cost:** per class of `n` keys, `O(n log n)` comparisons of object encodings, plus one re-encoding of the state. **Duplicates:** objects with equal sort keys that differ in where their references point are ordered by their keys, so two members of one orbit can sort to different canonical forms. The reduced graph then holds more than one representative of that orbit. It never holds fewer states than there are reachable orbits, nor more than the unreduced graph, and a verdict is unaffected. The canonicaliser is a function of the state, so EN-1 stays deterministic (ADR-018 §3). |
| SYM-8 | **The `symmetric` annotation.** A population declaration may carry `symmetric` (illustrative spelling: `population config_history: ConfigVersion symmetric`; the shared grammar and the model contract own it). It states that the population's identities are interchangeable in every clause that reads it. S3 refuses, at authoring time, every SYM-3 breaking form in any clause of any unit that reads an annotated population: operation preconditions and postconditions, invariants, temporal clause atoms, `terminal when` predicates, state constraints and refinement rows. The refusal names the breaking node and the population. The annotation enters the checked package, so every verdict that used symmetry binds to a model that carries it (ADR-013 O-09). It does not apply the reduction: a request still opts in with SYM-1, and chooses the key classes. |

Symmetry has two halves. The model's half, that no clause observes identity,
is a property of source, so it is annotated there and checked at authoring
time (SYM-8, RU-5). The subject's half, which keys are interchangeable and
whether the initial states respect that, depends on the universe and the
initial snapshot, which are request members (ADR-018 SM-2), so the request
declares the classes and EN-1 checks SYM-4.

#### 1.1a The annotated quotient: `each` fairness under symmetry

A `whole` constraint's enabledness and taking are invariant under every `π`,
so the fairness filter reads them off the quotient directly (§2). An `each`
constraint names one transition identity, and `π` maps it to another, so the
filter needs to know which concrete identities a lifted cycle takes and
where they are disabled. The quotient EN-1 retains already carries that: each
edge keeps the permutation its target was canonicalised by (EI-3). This
record decides `each` fairness on that annotated quotient, after Emerson and
Sistla 1997's annotated quotient structure.

| ID | Rule |
| --- | --- |
| AQ-1 | **Annotated quotient.** The retained reduced product, whose edges are (source, transition identity, canonical target, `π`) with target `= π(t(source))`. Along a walk, the concrete state is `σ(canonical state)` with `σ` updated to `σ ∘ π⁻¹` at each edge (EI-6), so a walk's **word**, the composition of the `π⁻¹` along it, says which concrete identity each step takes: at a node reached with word `u` from a concrete entry `σ₀`, edge label `t` is the concrete identity `σ₀(u(t))`. |
| AQ-2 | **Cycle group.** For an SCC `C` of the annotated quotient with entry `c₀` (its first state in discovery order), fix a spanning tree of `C` from `c₀` in canonical edge order, and let `u_c` be the tree word to node `c`. Every edge (`c`, `t`, `c'`, `π`) of `C` gives the closed-walk word `u_c ∘ π⁻¹ ∘ u_c'⁻¹`. The **cycle group** `H_C` is the group these words generate (tree edges give the identity). It is a subgroup of the run's group. |
| AQ-3 | **The concrete component.** For a concrete entry `σ₀(c₀)`, the concrete states over `C` reachable from it are exactly `σ₀ h u_c (c)` for `h` in `H_C` and `c` in `C`, and they are mutually reachable: any closed walk with word `h` repeated the order of `h` times returns to the entry. This concrete component is strongly connected, it contains the infinitely visited part of every concrete lasso whose projection settles in `C`, and the components over `C` for different `σ₀` are images of one another under the run's group. Testing it once per quotient SCC is therefore exact, duplicates of the sort canonicaliser (SYM-7) included: the argument uses only that each edge's `π` is exact. |
| AQ-4 | **Witnesses.** For an `each` constraint on operation `O`, its witness set in `C` is `W_taken = { u_c(t) : edge (c, t, …) of C with t of O }` and `W_disabled = { u_c(d) : d an identity of O disabled at c, c in C }`, read from enabled sets (ADR-019 SR-1). `W_enabled` is the same with enabled identities. In the concrete component, identity `e` is taken somewhere exactly when `σ₀⁻¹(e)` lies in the `H_C`-orbit of `W_taken`, and likewise for disabled and enabled. The identities of an `each` constraint form a set closed under the run's group, so `σ₀` drops out and the tests read orbits of `H_C` alone. |
| AQ-5 | **Weak `each` fairness.** `C` passes ADR-018 FA-4 for a weak `each` constraint on `O` when the `H_C`-orbits of `W_taken ∪ W_disabled` cover every identity of `O`. |
| AQ-6 | **Strong `each` fairness.** `C` meets ADR-019 SR-2 (c) for a strong `each` constraint on `O` when every identity of `O` in the `H_C`-orbit of `W_enabled` is in the `H_C`-orbit of `W_taken`. The failing identities `B` form a union of `H_C`-orbits, so whether a concrete state over `c` enables a member of `B` depends on `c` alone: either every lift of `c` does or none does. ADR-019's refinement therefore removes whole quotient states, `R = { c : B meets u_c(E(c)) }`, and recurses on the SCCs of `C \ R`, each with its own cycle group. ADR-019 SR-3 and SR-4 hold unchanged. |
| AQ-7 | **Concrete counterexample.** For a passing component, EN-1 builds the loop from the concrete entry by visiting obligations in canonical order (acceptance sets, then `whole` constraints, then each `each` identity). For an `each` identity `e`, it finds a witness `x` at node `c` and `h` in `H_C` with `σ₀ h u_c(x) = e`, writes `h` as a word in the generators by breadth-first search over the generators in canonical edge order, walks the closed walk of each generator in that word, then the tree path to `c`, then takes `x` (or stays at `c`, where `x` is disabled). It returns to `c₀` by the tree, and repeats the whole loop until the concrete state equals the concrete entry state, as EI-6 does. Replay re-checks every `each` identity against the concrete loop (ADR-019 SR-8). |
| AQ-8 | **Cost.** Per SCC with `n` states and `m` edges: the spanning tree and generator words, `O(n + m)`; at most `m - n + 1` generators, which a Schreier–Sims reduction may shrink; orbit closure of the witness sets under the generators, `O(I × g)` permutation applications for `I` identities of `each`-constrained operations and `g` generators. Strong `each` multiplies this by ADR-019's refinement depth. The concretised loop grows with the number of `each` identities times the generator-word length and the SCC's diameter. Every one of these runs under the caller's ADR-018 limits, time budget and meter, and reaching one settles V-7. No structural cap applies. |

#### 1.2 Partial-order reduction over independent transitions

| ID | Rule |
| --- | --- |
| POR-1 | **Footprints.** Every transition identity `t` of a `TransitionSystem` that offers POR has a static **write footprint** `W(t)` and **read footprint** `R(t)`, sets of locations. A location is (object, field) for a resolved object, (object type, field, any object) for a field read or written on an unresolved object, (population, membership of `k`) for whether the object with key `k` is a member of the population, or (population, membership of any object) when the object is not resolved. A protocol system adds ADR-027's protocol locations: `ctl(t)` (thread `t`'s rest point and disposition), `inst(i)` (parallel instance `i`), `bind(n, t)` (a binder), `queue(h)` (a channel's queue), `reg(c, ·)` (a compensation template's registrations), `role(r)` (a replicated role's instances), `pinst` (the live protocol instances), and the locations a memory model names; ADR-027 FT-1 gives each protocol step's `W` and `R` over them. Model locations, their meets, `W(t)` and `R(t)` are QSpec FR-383's "Locations", "Write footprint" and "Read footprint", which this record cites and does not restate. In summary: `W(t)` is the frame at each entry's scope, with a scoped `deletes self` or `deletes p` (QSpec FR-013) writing that object's membership and a type-wide `deletes T` or any `creates` writing (population, membership of any object); `R(t)` is every location a precondition reads, every pre-state location and every post-state location outside `W(t)` a postcondition reads, the receiver's and reference arguments' membership, for a `deletes` every reference-valued field that could name the deleted object (FR-120 admits no post-state with a dangling reference, so a delete is enabled only while no such field names it), and for a `creates` or a reference-field write the target populations' membership. A navigation `self.parent.f` reads (receiver, `parent`) and (the target type, `f`, any object); a field read on a quantified variable, binder or collection element reads (type, field, any object); a quantification over a population reads its any-object membership; a field read through `result`, which ranges over the post-state (FR-120), reads (the result's type, field, any object) and the any-object membership of the result's population. S3 derives `R` and `W` per operation from its checked clause bodies and frame with the receiver and parameters symbolic; EN-1 instantiates both footprints per transition identity. |
| POR-2 | **Enforced footprints.** POR is sound only when every transition writes inside `W(t)` and reads inside `R(t)`. On the model both are enforced. Writes: FR-120's `check_frame` admits a successor only when its delta lies inside the operation's frame (QSpec FR-013-AC-1), at the scope of each entry (POR-3). Reads: the clause evaluator (ADR-016 FE-3) evaluates a clause body for EN-1 over an observation restricted to `R(t)`, and a read outside it is an internal fault, `TerminalValue::Failed`, never a value. A footprint is therefore a checked fact about the model, not an author's claim. |
| POR-3 | **Receiver-scoped frame entries.** A `modifies` entry may be scoped to the receiver: it grants writes to that field on the receiver object only. A write to the field on another object is `frame_violation`. A type-wide entry keeps QSpec FR-013's meaning. Independence needs object-granular write footprints: with type-wide entries, two transitions of one operation on different receivers both write (type, field, any object) and are always dependent. The form is RU-1's. |
| POR-4 | **Independence.** Transition identities `t` and `u` are independent when `W(t) ∩ (R(u) ∪ W(u))` and `W(u) ∩ R(t)` are empty, where locations meet as follows: (type, field, any object) meets every location of that type and field; (population, membership of `k`) meets itself, (population, membership of any object), and every field location of the object `k` and of its type; (population, membership of any object) meets every membership location of the population and every field location of its member type. So deleting an object is dependent on every transition whose receiver or reference argument is that object or that reads its fields, and a `creates`, or a `deletes` whose object is not resolved, is dependent on every transition over the population. Independent transitions neither enable nor disable each other, and executing them in either order from a state reaches the same set of states, because each one's enabledness and post-states depend only on locations the other leaves unchanged. |
| POR-5 | **Visibility.** A transition is visible for a run when its write footprint meets a location some atom of the claim reads, a location the state constraint reads (§1.3), or when an atom reads the operation anchor of its position (ADR-018 SM-3) and names its operation. Every other transition is invisible: it leaves every atom's value unchanged. The deadlock-freedom item's `deadlocked` predicate makes no transition visible: a persistent-set search reaches every reachable terminal state (§2), and EN-1 evaluates `deadlocked` and `P` at each one it reaches. Over a protocol subject, `fork`, `join`, `timeout`, channel steps, `event` steps and `finish` write no model location, so they are invisible to every claim whose atoms read model state only (ADR-027 FT-5). Under a non-empty fairness set POR-10 adds fairness visibility. |
| POR-6 | **Ample set.** At a product state with model state `s` and enabled set `E(s)` (computed in full, ADR-019 SR-1), EN-1 expands an ample set `A(s) ⊆ E(s)` that satisfies the ample-set conditions of Clarke, Grumberg and Peled: **C0** `A(s)` is empty only when `E(s)` is; **C1** no transition dependent on a member of `A(s)` can run from `s` before a member of `A(s)` runs (ensured by the stubborn-set closure of POR-7); **C2** when `A(s) ≠ E(s)`, every member of `A(s)` is invisible; **C3** when `A(s) ≠ E(s)`, every successor through `A(s)` is a product state not yet discovered, otherwise `A(s) = E(s)` (POR-8). |
| POR-7 | **Closure and choice.** For each enabled `t` in canonical transition order (FR-181), the candidate seeded by `t` is the least set containing `t` that holds, for each enabled member, every transition identity dependent on it, and for each disabled member, every transition identity whose write footprint meets that member's **enabling footprint**, every location its enabledness reads. This is a stubborn-set closure (Valmari; Godefroid's persistent sets): the transitions dependent on an enabled member, and the transitions that can enable a disabled member. A model transition is enabled when its receiver and reference arguments are members of their populations, its precondition holds, and some post-state within its frame satisfies its postcondition (ADR-018 FA-2), so its enabling footprint is all of `R(t)` (QSpec FR-383 "Enabling footprint"): the precondition's reads, the postcondition's pre-state reads and its post-state reads outside `W(t)`, the receiver's and reference arguments' membership, and the referrer and target-membership reads of a `deletes` or `creates`. A protocol step's enabling footprint is ADR-027 FT-3's set, and that set must contain at least: the reads of the step's operation precondition and of its postcondition in the pre-state; the membership of its receiver and reference arguments; every binder its arguments read (ADR-027 FT-3); its own thread's control and instance locations; and, for a `duplicate` or `lose` step on channel `h`, `queue(h)`. A smaller enabling footprint is unsound: §7.6 gives a false `proved` for a guard written in a postcondition, a missing membership location, a delete blocked by a reference and a navigation through a rewritten reference. `A(s)` is the enabled part of the candidate with the fewest enabled members that satisfies C2 and C3, ties broken by the seed's canonical order, or `E(s)` when none does. The choice is a function of the subject and the run, so EN-1 stays deterministic (ADR-018 §3). |
| POR-8 | **The cycle proviso for breadth-first search.** FR-181 explores breadth-first, so the depth-first stack proviso does not apply. C3's revisit rule refers to the discovered set, as the provisos of Bošnački, Leue and Lluch Lafuente refer to the open and closed sets: a state with an ample-set successor already discovered is fully expanded. *Every cycle of the reduced graph has a fully expanded state.* Proof: let `c` be the cycle's state discovered first and `p` its predecessor on the cycle (`p = c` for a self-loop). When `p` is expanded, its successor `c` is already discovered, so C3 makes `A(p) = E(p)`. This is the strong proviso liveness needs (no enabled transition is ignored forever around a cycle) and also removes the ignoring problem for safety. It is checked on product states, so a cycle of the product, not of the model alone, carries a fully expanded state. |
| POR-9 | **Systems that offer POR.** `ModelSystem` offers POR, with footprints from its operations' frames and clauses: its successor relation interleaves every operation on every receiver. ADR-027's `ProtocolSystem` offers POR, with its steps' footprints (ADR-027 FT-1) and enabling footprints (FT-3); steps of distinct threads, such as sibling `parallel` branches, are its source of independence (FT-4). A system that supplies no footprints offers no POR, and a request selecting it settles V-8 (RV-6). |
| POR-10 | **Fairness visibility.** A **fairness class** is the set of transition identities one fairness constraint of the claim's fairness set covers, authored or derived: an operation's or attempt node's steps (`whole`), one transition identity or one thread's steps (`each`), or a thread target's steps, which are what ADR-027's default scheduler constraints cover (one `fair weak whole` per `parallel` branch, the root and each compensation template). Under a non-empty fairness set, a transition is also visible when its write footprint meets the enabling footprint (POR-7) of a member of some fairness class it does not itself belong to. A transition that adds a member to a class or removes one writes that member's membership, which is in its enabling footprint, so a `creates` that adds a member to a `whole` class, and a `deletes` that removes one, is fairness-visible. A step that writes only its own thread's control, its own binders and model or channel locations that no other class's enabledness reads stays invisible. C0, C1, C3 and the proviso of POR-8 are unchanged; C2 reads visibility with this addition. POR-11 proves that the reduced product then keeps a fair accepting behaviour whenever the full product has one. |
| POR-11 | **Why POR-10 keeps fairness verdicts exact.** *Soundness.* The reduced product is a subgraph of the full product, and the fairness filter reads full enabled sets (POR-6, ADR-019 SR-1), so a fair accepting cycle it finds is a fair accepting cycle of the subject. *Completeness* is proved in eight steps. A state maps each **cell**, an (object, field) pair or a (population, key) membership, to a value; a footprint location stands for the cells it meets (POR-4). A **step** `(t, d)` is a transition identity `t` with one admitted assignment `d` to the cells of `W(t)`; it changes those cells to `d` and no other cell (POR-2). Steps 1 to 3 and 5 to 8 are proved here; step 4 names its sources. **(1) A step depends only on its read footprint.** Whether `(t, d)` is enabled at `s` is a function of `s` on the cells of `R(t)`: the receiver's and arguments' membership, the precondition, and the postcondition's pre-state reads and post-state reads outside `W(t)` are all in `R(t)` (POR-1, POR-7), the post-state inside `W(t)` is `d`, and the evaluator reads nothing else (POR-2). Whether `t` is enabled, some `(t, d)` being enabled, is a function of the same cells. **(2) Independent steps commute.** Let `t` and `u` be independent (POR-4). `(t, d)` writes no cell of `R(u)`, so by (1) `(u, e)` is enabled at `s` exactly when it is enabled after `(t, d)`. When both are enabled, both orders reach the same state, because `W(t)` and `W(u)` share no cell and each step sets its cells to a fixed assignment. So two step sequences that differ by swapping adjacent independent steps are enabled together and reach the same state. **(3) Last writer.** After a step sequence, a cell holds the value its last writer assigned, or its initial value. Two steps that write a common cell are dependent (POR-4). **(4) The representative.** Let `σ = s₀ a₀ s₁ a₁ …` be a fair accepting behaviour of the full product. Build `σ'` in the reduced product with a queue `η` of the steps of `σ` not yet taken, initially all of them in `σ`'s order. At a reduced state `v`: if the identity of some step of `η` is in `A(v)`, take the first such step and remove it from `η`; otherwise take a step of some member of `A(v)`, an **extra**, and leave `η` unchanged. This is the construction of the correctness proof of Clarke, Grumberg and Peled, and Peled 1996 proves it carries over to the product with the property automaton when C3 is checked on product states, as POR-8 does. By C1, the steps of `η` before the step taken run from `v` without a member of `A(v)`, so none is dependent on the step taken; for an extra, all of `η` runs from `v` without a member of `A(v)`, so the extra is independent of every step of `η`. By (2), `η` stays enabled after each step. If `E(v)` is empty, `v` is terminal and the construction takes no step: `σ'` stays at `v` with its terminal stutter (ADR-018 SM-4). Then `η` is empty too, since a remaining step of `η` would be enabled at `v` by (2). So `σ` has also reached its terminal state, and `σ'` ends at that same terminal state, stutter-extended as `σ` is. **(5) What `σ'` keeps.** (a) Every step of `σ` is taken. If the head `α` of `η` were never taken, from some point every later step would be independent of `α`, so `α` stays enabled by (2), and no state from that point has `α`'s identity in its ample set. `σ'` runs in the finite reduced graph, so it closes a cycle among those states, and POR-8 gives that cycle a fully expanded state, whose ample set holds `α`'s identity. (b) Steps of `σ` that are dependent keep their order, so by (3) each cell has the same sequence of writers from `σ` in `σ` and in `σ'`. (c) An extra, and every step taken from an ample set smaller than `E(v)`, is invisible (C2). A visible step is taken only at a fully expanded state, where the head of `η` is enabled and in the ample set, so it is the head of `η`: every step `σ` takes before it is already taken. (d) `σ'` is stuttering equivalent to `σ` on the atoms (Clarke, Grumberg and Peled), and the infinite-trace forms PT-2 admits under POR are invariant under stuttering, because a formula without next is (Lamport 1983 for the future operators; QSpec FR-161 and TC-328 for `since` and `triggered` without intervals), so `σ'` is accepting. **(6) No extra is in a class `σ` takes finitely often.** Fix a fairness class `K` whose members `σ` takes finitely often, none at or after position `p`. Suppose an extra of `σ'` is in `K`, and let `β` be the first one, taken at `v`, with `X` the steps of `σ` already taken and `η` the rest. Take any `j` beyond `p` and beyond every position of `X` in `σ`, and compare the cells of `β`'s enabling footprint, all of `R(β)` (POR-7), at `v` and at `s_j`. At `v` their writers are steps of `X` and earlier extras. Each earlier extra is outside `K`, by the choice of `β`, and invisible by (5c), so by POR-10 it writes no cell of the enabling footprint of any member of `K`. Each cell's last writer at `v` is therefore its last writer in `X`, the same in `σ'`'s order as in `σ`'s by (5b). At `s_j` the writers are the steps of `σ` up to `j`, which are those of `X` and some of `η`. `β` is independent of every step of `η` (4), so none of those writes `R(β)`, and each cell's last writer is again its last writer in `X`. By (3) the two states agree on `R(β)`, so by (1) `β` is enabled at every `s_j`. A member of `K` is then enabled at every state of `σ` from some point on, and `σ` takes none, so `σ` violates weak fairness for `K`, and strong fairness with it. This contradicts the fairness of `σ`, so no extra is in `K`, and `σ'` takes members of `K` finitely often. **(7) `K`'s enabledness along `σ'` is a stuttering of its enabledness along `σ`.** Call a step a **`K`-writer** when it writes a cell of the enabling footprint of a member of `K`. Let `q` be a point of `σ'` after which every step of `σ` before `p` is taken (5a). No step after `q` is in `K`, by the choice of `p` and by (6), so by POR-10 every `K`-writer after `q` is visible. By (5c) no extra is a `K`-writer, and the `K`-writers of `σ` at or after `p` are taken in `σ`'s order, each as the head of `η`. At a state `v` of `σ'` after `q`, let `aₙ` be the first `K`-writer of `σ` at or after position `p` that `σ'` has not taken (`n = ∞` when there is none). The `K`-writers among the steps taken are then exactly the `K`-writers of `σ` before `aₙ`, so by (3) and (5b) every cell of `K`'s enabling footprints has at `v` the value it has at every state of `σ` after the last `K`-writer before `aₙ` and up to `sₙ`, the state where `σ` takes `aₙ` (for `n = ∞`, at every state of `σ` after its last `K`-writer). That value changes along `σ'` only when `σ'` takes `aₙ`. So the sequence of `K`'s enabledness along `σ'` after `q` is the sequence along `σ` after `p`, with each maximal constant block repeated one or more times. `K` is enabled infinitely often along `σ'` exactly when along `σ`, and enabled at every state from some point on along `σ'` exactly when along `σ`. **(8) Conclusion.** For each class `K`: if `σ` takes `K` infinitely often, so does `σ'` (5a), and `K`'s weak and strong constraints hold on `σ'`. Otherwise, by (6) and (7), `σ'` takes `K` finitely often, and `K`'s weak (strong) constraint holds on `σ'` exactly when it holds on `σ`, which it does. With (5d), `σ'` is a fair accepting behaviour of the reduced product, with fairness read from full enabled sets. ADR-019's search finds a fair accepting SCC of the graph it is given whenever that graph has a fair accepting behaviour, so the reduced product has a fair accepting SCC whenever the full product does. |

#### 1.3 State constraints

| ID | Rule |
| --- | --- |
| SC-1 | **Form.** A state constraint is a Boolean state clause over the model, declared in the unit and checked at S3 like an invariant body; the request names it. The spelling is QSpec's (QS-1). |
| SC-2 | **Meaning.** EN-1 expands a state only when the constraint holds there. A reached state where it is false is a **boundary state**: it is discovered, keyed, read by the property automaton and by the safety monitor, and never expanded. A boundary state is not terminal: it is never stutter-extended (ADR-018 SM-4) and never a deadlock, since ADR-018 DL-7 reads a deadlock from an expansion that yields no successor and a boundary state is not expanded. Enabledness at expanded states is computed from the model and is unaffected by the cut (ADR-019 SR-1). A constraint that evaluates `Undefined` at a reached state does not make it a boundary state: the item settles `refuted` with cause `UndefinedEvaluation` at that state, by the undefined ruling, with a counterexample ending there that replays. A refused constraint evaluation stops the expansion with its refusal. Constraint evaluation is charged to the run's meter, and an incomplete evaluation is that meter running out, a reached limit (RV-5). |
| SC-3 | **Relation to bounds and universes.** A universe and a `ProofBound` define the subject and enter the obligation identity (ADR-018 §1 "Scope", ADR-014 B-4). A state constraint does neither: it is a method parameter of EN-1, an ADR-014 B-5 value like `max_depth`, and it is recorded in the terminal record's method. A run that reaches no boundary state explored the whole subject, and a run that reaches one settles `inconclusive` (RV-4), so a constraint never changes what a verdict means, except that a constraint that evaluates `Undefined` at a reachable state refutes the item (SC-2). It cannot replace a bound: EN-1's pre-check still requires every root bounded (ADR-018 §3). |

### 2. Which property kinds each reduction preserves

| ID | Rule |
| --- | --- |
| PT-1 | **The table is normative.** A reduction applies to a run exactly when its row in PT-2 reads "yes" for the claim's form and fairness set. "No" settles the item before expansion, `inconclusive`, `ReductionNotPreserving` (RV-2). |
| PT-2 | **Preservation.** The table below. "Whole" and "each" are ADR-018 FA-1 granularities; "weak" and "strong" are ADR-019's kinds. |

| Form | Symmetry (admitted, SYM-5) | POR (C0–C3, POR-6) | State constraint |
| --- | --- | --- | --- |
| TP-1 reachable-state invariant | yes | yes | refutation real; pass only when unreached (RV-4) |
| TP-2 bounded MLTL, `on origin` or `on each` | yes | no: counts positions | as TP-1 |
| TP-3 LTL safety, no interval operator | yes | yes | as TP-1 |
| TP-3 LTL safety with an interval operator (ADR-018 IV-4) | yes | no: counts positions (ADR-018 IV-7) | as TP-1 |
| TP-4 liveness, empty fairness set, no interval operator | yes | yes | lasso real (only expanded states form cycles); pass only when unreached |
| TP-4 liveness with an interval operator under an unbounded one (ADR-018 IV-1) | yes, subject to the fairness rows | no: counts positions | as above |
| TP-4 liveness, weak or strong, every constraint `whole`, including a protocol's default scheduler constraints (ADR-027 PA-3) | yes | yes, with fairness visibility (POR-10) | as above |
| TP-4 liveness, weak or strong, any constraint `each` | yes, on the annotated quotient (AQ-5, AQ-6) | yes, with fairness visibility (POR-10) | as above |
| Deadlock-freedom item (ADR-018 DL-3) | yes | yes (C0, C1) | a deadlock found is real; boundary states are never deadlocks; pass only when unreached |
| TP-5 refinement (ADR-020), safety half | yes | no: reads step labels | as TP-1 |
| TP-5 refinement, liveness half | yes; `each` rows on the annotated quotient | no | as TP-4 |

**Protocol subjects.** The rows read the same over a protocol subject
(ADR-027) as over a model subject. A fairness constraint on a branch, the
root or a compensation template (ADR-027 PA-1) reads as `whole` or `each`
exactly as an operation constraint does. The default scheduler constraints
(ADR-027 PA-3) make every liveness clause's fairness set non-empty, so the
`whole` row with POR-10 applies; a protocol with `scheduling adversarial` and
no authored constraint has an empty fairness set and takes the first
liveness row.

Why each entry holds:

- **Symmetry, safety and bounded forms.** Under SYM-5 the quotient by `G` is
  bisimilar to the subject on symmetric atoms (Emerson and Sistla 1996). The property automaton's moves depend only on
  atom values, which `π` preserves, so the product quotients by `G` acting on
  its model component. Paths of the quotient have the lengths of the paths
  they stand for, so step-counting forms (TP-2, formulas with interval
  operators) are preserved. `deadlocked` and `P` are identity-transparent
  (SYM-3), so the deadlock-freedom item is a symmetric invariant.
- **Symmetry and liveness.** A fair accepting lasso of the subject maps to a
  fair accepting cycle of the quotient, since acceptance and every `whole`
  constraint's enabledness and taking are invariant under `π`: a whole
  operation is enabled at `s` exactly when it is at `π(s)`, and `π(t)` is a
  transition of the same operation. Conversely a quotient cycle lifts to a
  subject lasso by repeating it until the concrete state closes (EI-6), and
  every repetition visits the same accepting states, operations and
  enabledness. Neither direction loses a violation, so no liveness violation
  is hidden. This is the case where TLC's temporal checking may miss errors
  (Lamport, §14.3.4): TLC checks neither the symmetry nor the property's
  symmetry, and QSL checks both (SYM-3 to SYM-6) and
  replays every lifted lasso (EI-6).
- **Symmetry and `each`.** An `each` constraint names one transition
  identity, and `π` maps it to another, so the quotient's edge labels alone
  do not say which identity a lifted cycle takes. The annotated quotient
  (AQ-1 to AQ-6) recovers it from the edge permutations and the SCC's cycle
  group, and decides weak and strong `each` fairness exactly, as Emerson and
  Sistla 1997's annotated quotient structure does for fairness under
  symmetry.
- **POR and stutter-invariant forms.** With C0–C3 the reduced graph contains,
  for every behaviour of the subject, a behaviour that is stuttering
  equivalent on the visible atoms (Clarke, Grumberg and Peled; Peled
  1996 with the property automaton), so it preserves every property
  invariant under stuttering. The direction used is that every formula
  without next is invariant under stuttering: for the future operators,
  LTL without next (Lamport 1983); for `since` and `triggered` without
  intervals, QSpec FR-161 (QS-8, tested by QSpec TC-328). Peled and Wilke
  prove the converse, that every stutter-invariant property is expressible
  without next, so the fragment loses no stutter-invariant property. The infinite-trace grammar without interval operators is
  that fragment (Context).
  TP-1 and TP-3 are its safety members, TP-4 with no fairness its liveness
  members; the strong proviso (POR-8) is what makes the liveness case sound.
- **POR and position counting.** A bounded-profile operator and an interval
  operator under infinite-trace count positions (ADR-018 IV-7), and a
  stuttering-equivalent behaviour has different positions, so neither TP-2
  nor any formula with an interval operator is preserved.
- **POR and fairness.** Fairness reads which transitions are taken and
  where a class is enabled. C0–C3 alone keep every transition of a behaviour
  but may move the points where another thread changes a class's
  enabledness, so a fair behaviour could have only unfair representatives.
  POR-10 makes exactly those writes visible, and POR-11 shows the reduced
  product then keeps a fair representative of every fair behaviour, for weak
  and strong constraints of either granularity. The cost is the extra
  visible steps: in a protocol, a step that ends a branch (it writes the
  parallel instance the parent's join reads), a fork, a send to a channel
  another thread receives from, and a write to a model location another
  thread's enabledness reads. Steps local to their thread stay invisible.
- **POR and deadlocks.** C0 and C1 make every ample set a persistent set, and
  a persistent-set search reaches every reachable terminal state (Godefroid,
  the persistent-set and safety chapters).
  The deadlock-freedom item is decided at terminal states (ADR-018 DL-7), so
  it is preserved without making any transition visible (POR-5).
- **POR and refinement.** ADR-020's step check reads each step's transition
  identity, so every transition is visible to it and the reduction removes
  nothing.
- **State constraint.** Every explored path is a path of the subject, so a
  violation found is real; a lasso is formed only by expanded states, whose
  enabledness the cut does not change, so it is a fair lasso of the subject.
  A pass says nothing about paths through boundary states.

**Combining reductions.** Symmetry and POR combine (Emerson, Jha and Peled 1997):
EN-1 computes `A(s)` on the canonical representative, canonicalises each
successor, and applies C3 to canonical product states. A combination applies
only when every member's row reads "yes". A constraint combined with symmetry
must itself be identity-transparent (SYM-3); combined with POR, its read
locations are visible (POR-5).

### 3. Interactions with the strong-fairness and refinement records

- **Strong fairness under POR.** Preserved under fairness visibility
  (POR-10, POR-11): the argument keeps a class's enabledness stuttering
  equivalent, which covers "enabled infinitely often" as well as
  "continuously enabled". ADR-019's SCC refinement runs on the reduced
  product unchanged.
- **Strong fairness under symmetry.** Preserved for `whole` constraints, by
  the same invariance as weak `whole`: the Streett pair (enabled states,
  taking edges) of a whole operation is invariant under `π`. ADR-019's SCC
  refinement runs on the quotient product unchanged, reading enabled sets of
  canonical states. `each` constraints are decided on the annotated quotient
  (AQ-6), whose refinement removes whole quotient states.
- **Refinement under symmetry.** ADR-020's population map is key-preserving
  (ADR-020 RM-2), so `π` acts on the concrete and the abstract state alike and
  commutes with `map`. Admission (SYM-3) also covers the mapping rows,
  step-map arguments and history updates, and the AX-2 set of abstract states
  is renamed element by element and re-sorted. Both halves are preserved;
  `each` rows in `F_C` or `F_A` are decided on the annotated quotient (AQ-5,
  AQ-6).
- **Refinement under POR.** Not preserved (PT-2).

### 4. Integration with EN-1 and exhaustive mode

| ID | Rule |
| --- | --- |
| EI-1 | **Pre-check.** After ADR-018's root classification, EN-1 (a) looks up every selected reduction's PT-2 row for the claim's form and fairness set; (b) admits each symmetry declaration (SYM-2, SYM-4; SYM-3 holds by SYM-8) and computes the run's group, per binding orbit (SYM-6); (c) instantiates footprints (POR-1) for POR; (d) resolves the constraint clause. Any failure settles the item before the first expansion (§5). |
| EI-2 | **Initial states.** Each initial product state is canonicalised (SYM-7) and coalesced by canonical key. A constraint false at an initial state makes it a boundary state. |
| EI-3 | **Expansion.** For each product state in FR-181 breadth-first order: compute `E(s)` and its enabled set (ADR-019 SR-1); pick `A(s)` (POR-6, POR-7) or take `E(s)`; apply each member through the system; canonicalise each successor and keep the permutation; mark a successor where the constraint is false as a boundary state; apply C3 against the discovered set; retain each edge as (source, transition identity, canonical target, permutation). The retained graph is the reduced product; ADR-018's SCC phase and ADR-019's fairness filter run on it unchanged. |
| EI-4 | **Hooks.** FR-101's `TransitionSystem`, owned by `qsl-eval`, carries three optional hooks a system implements to offer a reduction: `canonical(state) -> (state, permutation)` for symmetry, `footprint(transition) -> Footprint` for POR, and `holds_constraint(state) -> ConstraintValue` for constraints, where `ConstraintValue` is `Holds`, `Boundary` or `Undefined{object, reason}` (SC-2); and `offered_reductions()`, which names the hooks it implements and which the pre-check reads (EI-1). `ModelSystem` implements all three. A product system delegates them to its model component, so the automaton component is never reduced. |
| EI-5 | **Limits.** `max_states` counts stored product states, boundary states included; the frontier holds canonical state keys' digests (ADR-014 TR-7). The limit map of ADR-018 §3 holds; `max_depth` is the horizon, not a limit (RV-5). |
| EI-6 | **Concrete counterexamples.** The canonical counterexample is chosen over the reduced graph as ADR-018 §3 chooses it. EN-1 then concretises it before it leaves `model_check`. Symmetry: keep `σ` with concrete state `= σ(canonical state)`; start at the first initial state in subject order whose canonical form is the stem's first state, with `σ = π⁻¹` for the permutation `π` the canonicaliser returned for it; each retained edge (source, `t`, target, `π`) becomes the concrete step `σ(t)`, and `σ` becomes `σ ∘ π⁻¹`. A lasso's quotient loop ends at its entry state's canonical form under some `ρ`; repeat the loop until the concrete state equals the concrete entry state, at most the order of `ρ` times. With an `each` constraint, the loop is built by AQ-7 and closed the same way. POR: a reduced path is a path of the subject, so it is already concrete. Constraint: every explored path is concrete. |
| EI-7 | **Replay.** The concretised counterexample is an ordinary ADR-018 CX-2 model trace and replays through `ModelSystem` by CX-3, with fairness re-established (ADR-019 SR-8). A counterexample for an undefined constraint (SC-2) also re-evaluates the constraint at the reported state, so the prefix reproduces the undefined value. No counterexample carries a canonical state, a permutation or a reduction: a reduced search produces the same counterexample type as an unreduced one, and QSpec FR-181-AC-5 holds for it. |

### 5. Verdicts and identity

Every verdict is one of ADR-018 V-1 to V-8.

| ID | Rule |
| --- | --- |
| RV-1 | **A reduced proof is its own technique.** A run that completes with symmetry or POR applied and finds no counterexample settles `proved`, basis `closed-scope`, proof basis `reduced` (QSpec FR-385), `TerminalValue::Proved{basis: ProofBasis::Reduced{reductions}, certification: Uncertified}`, O-16 success. It is labelled `uncertified` (ADR-018 PC-1, RU-6): FR-338's closure certificate and FR-339's component certificate do not cover a symmetry- or partial-order-reduced graph, and the core has no checker for such a proof. A copy-swap-reduced hyper proof is checked by ADR-023 HX-7's product-closure checker and is `Certified`. `reductions` lists each applied reduction with its parameters and its PT-2 row: `Symmetry{groups}`, each group a population with its key classes, and the binding stabiliser per orbit (SYM-6); `PartialOrder{proviso: BreadthFirstRevisit}`. `Reduced` is never `Exhaustive`, and a consumer that counts proofs counts it as its own technique, with the subject's universes, the groups and the footprint scopes as its bounds. A run with only a state constraint that reaches no boundary state settles `Proved{basis: Exhaustive, certification: Uncertified}`, with the constraint in the terminal record's method. |
| RV-2 | **`ReductionNotPreserving{reduction, form}`.** A new `InconclusiveCause`: a selected reduction's PT-2 row reads "no" for the claim's form or fairness set. Settled in the pre-check; no expansion runs. |
| RV-3 | **`SymmetryBroken{population, cause}`.** A new `InconclusiveCause`, with `cause` `InitialStatesNotClosed{initial, generator}` (SYM-4). Settled in the pre-check. An identity-observing form never reaches a run: S3 refuses it on an annotated population (SYM-8), and a request on an unannotated population refuses (SYM-2). |
| RV-4 | **`ConstraintReached{boundary_states}`.** A new `InconclusiveCause`: the run completed with no counterexample and reached at least one boundary state. It takes precedence over `BoundReached` when both apply. |
| RV-5 | **The search horizon under POR.** `max_depth` is the search horizon, a method parameter (ADR-014 B-5) and never a resource limit, so a run that reaches it completed its method. A reduced graph does not keep short counterexamples short, so "no counterexample of length at most `k`" (ADR-018 V-5) does not follow from a POR run to horizon `k`. A POR run that completes to `max_depth` with no counterexample settles V-6, `inconclusive`, with the new cause `ReductionHorizon{max_depth}` (QSpec FR-385 `reduction-horizon`), never `BoundReached` and never V-7. Symmetry keeps path lengths, so a symmetry-only run to `max_depth` settles V-5. `ConstraintReached` takes precedence over both. A run under any reduction that a limit stops before it completes its method (`max_states`, `max_transitions`, `max_automaton_states`, the evaluation meter) settles ADR-018 V-7, `incomplete`, `LimitReached{limit, value, setting}` (QSpec `limit-reached`), never `failed`. |
| RV-6 | **Capability.** EN-1's provider manifest advertises the reductions it applies. Negotiation routes an item that selects a reduction only to a candidate advertising it, and never drops or adds a reduction. With no such candidate, or a selected POR over a system that supplies no footprints (POR-9), the item settles V-8, `unsupported-requested-capability`. |
| RV-7 | **Refutations are unreduced.** A refutation settles `refuted` (V-4) with a concrete, replayed counterexample (EI-6, EI-7). Its basis is `decisive-counterexample` whatever reduction found it, and its record names no reduction. An undefined claim evaluation found on a reduced graph settles by ADR-018 UE-1 to UE-6, `refuted` with cause `UndefinedEvaluation{where, cause}`, its prefix concretised by EI-6 and replayed by EI-7. Whether an atom is defined depends only on the locations it reads, so POR-5 already makes visible every transition that can change it, and every reduction that preserves a form preserves its undefined evaluations. |
| RV-8 | **Identity.** The selected symmetry declarations and the POR selection enter the item's obligation identity (ADR-013 O-09) beside the subject. The PT-2 row that admits each is a function of the reduction, the claim's form and its fairness set, all of which the identity binds, so the identity carries the soundness fact; `ProofBasis::Reduced` states it in the result. A reduced and an unreduced request for one claim are different obligations, and a verdict on one never joins the other's request (ADR-014 §8). A state constraint is a method parameter (SC-3): it stays out of the obligation identity and is part of the request identity, like `limits`, so a result is never reused across requests with different constraints. |
| RV-9 | **No fallback.** EN-1 never answers a reduced request by running unreduced, or the reverse. A caller who wants the unreduced verdict after `ReductionNotPreserving` or `SymmetryBroken` submits a request without the reduction, which is a new obligation. |
| RV-10 | **Reduced-versus-unreduced agreement.** A reduction never changes a verdict, and the implementation shows it on every model it is tested with: every model-check case of QSL's model-check test corpus, the worked examples of ADR-018, ADR-019, ADR-021 and ADR-027 and the QSpec conformance vectors QSL runs, is checked unreduced and under every selection of symmetry, POR, or both, that the case's PT-2 rows admit. Each pair must settle the same ADR-018 verdict, and every counterexample from either run must replay (EI-7). A case with a state constraint, and a pair in which either run ends at its horizon or at a resource limit, is not compared, since neither settles what the subject satisfies. This is QSpec FR-383's agreement requirement. The proofs of POR-8, POR-11 and §2 say a reduction is sound; the differential test checks that the implementation of footprints, ample sets, canonicalisation and fairness visibility matches them. |

### 6. Transfer to code

A reduced verdict is a fact about the model subject. Using it as evidence
about the code that implements the model rests on the conditions below. QSL
states them as preconditions; it specifies no IR, CG or RT internals.

| ID | Rule |
| --- | --- |
| TX-1 | **POR transfers to code under a frame-enforcement precondition.** A POR-reduced proof equals the unreduced verdict on the model (PT-2) because the footprints are enforced on the model (POR-2). It transfers to code only when frame enforcement on code covers every field of the receiver that the operation's frame does not modify, and, for a receiver-scoped entry (POR-3), the granted field on every object other than the receiver. The precondition has three parts: (1) QSpec FR-013's frame semantics (AC-1, AC-5, AC-6) with the receiver-scope amendment (QS-4); (2) the frame entries IR FR-040 admits from the checked package, carrying each entry's scope; (3) a code-side frame-effect obligation that asserts every non-modified field of the receiver unchanged and, for a receiver-scoped entry, the granted field unchanged on every other object. CG FR-015's frame-effect obligation asserts unchanged only the fields its caller names (CG FR-015-AC-28), and its test case CG TC-025 is planned, so part (3) is a precondition the transfer depends on: the transfer holds once CG FR-015 covers all non-modified fields at the scope above and CG TC-025 passes. Read footprints are a model-side fact (POR-2) and need no code-side enforcement: an implementation that reads more still makes only steps its contract and frame admit. |
| TX-2 | **Symmetry transfer is a precondition.** A symmetry-reduced proof says the model's behaviours are closed under permuting interchangeable keys. Code-side evidence that relies on that closure, such as a harness that breaks symmetry by assuming an order on keys, or a measurement that counts one key assignment for its orbit, holds only when the code observes no identity order: either the runtime representation makes no identity order observable (no ordered-map iteration over identities, no identity-to-text reaching behaviour), or the evidence uses the bounded shadow path that CG ADR-003 Q1 selects per family. QSL records this precondition and requires no runtime change. |
| TX-3 | **Soundness per obligation kind lives in QSL's result.** The PT-2 row of every applied reduction is carried by the obligation identity and by `ProofBasis::Reduced` (RV-1, RV-8). A Kani harness is a bounded per-clause proof with no exploration, so EN-1's reductions never apply to one. A harness identity carries a reduction only when the harness applies one itself, as a tightened bound recorded in the identity (CG ADR-003 Q2). |
| TX-4 | **A reduced proof is never an unreduced proof.** A proof-coverage measurement counts `Reduced` as its own technique, covering only the property classes whose PT-2 row reads "yes", with the universes, symmetry groups and footprint scopes as its bounds. A `ReductionNotPreserving` or `SymmetryBroken` result is a visible gap, never coverage. |
| TX-5 | **No reduced trace reaches the evidence chain.** Counterexamples are concrete QSL model traces (EI-7), so no new trace kind enters replay or any downstream evidence (ADR-013 C-09). |
| TX-6 | **Lowering.** A later lowering of a QSL model to TLA+ may emit a TLC `SYMMETRY` set only for a declaration EN-1 admitted (SYM-5), and only for property kinds whose PT-2 row reads "yes". |

### 7. Worked examples

All state counts below are hand enumeration; no QSL engine produced them.

#### 7.1 Symmetry: three interchangeable configs

**Subject.** ADR-018 §6's modified ConfigVersion unit, whose `attemptUpdate`
advances `versionNumber` modulo 3, with universe `config_history = {a, b,
c}` and one initial snapshot with all three at version 0. Write a state as
`(va, vb, vc)`, and count states and transitions as ADR-018 §6 does: each
state has one successor per receiver, `upd(a)`, `upd(b)`, `upd(c)`. The
population is declared `symmetric` (SYM-8), and the request declares
`SymmetryDeclaration{config_history, [[a, b, c]]}`.

**Admission.** The post clause reads `self.versionNumber` only; the
invariants `ParentOrder` and `NoCycle` read `parent` by dereference and
equality; no clause folds over a set of references. S3 accepts the
annotation (SYM-8). The one
initial state is fixed by both generators, `(a b)` and `(a b c)`. SYM-4
holds.

**State counts.** Every object here holds only its version and an absent
`parent`, so objects with equal sort keys are identical and the sort
canonicaliser (SYM-7) keeps exactly one representative per orbit. The
symmetry counts below equal the orbit counts; a model whose objects hold
references could store a few more.

| | Unreduced | Symmetry |
| --- | --- | --- |
| Model states | 27 | 10 (multisets of three versions) |
| Model transitions | 81 | — |
| Instances of `ReachesTwo` checked | 3 (`c = a`, `b`, `c`) | 1 (`c = a`; one orbit) |
| Model states per instance | 27 | 18 (group: stabiliser of `a`, swapping `b` and `c`) |
| Product states per instance | 45 (27 with `q0`, 18 with `q1`) | 30 (18 with `q0`, 12 with `q1`) |
| Product states in total | 135 | 30 |

**Verdict under `fair weak whole attemptUpdate`.** The form is TP-4 with
one `whole` constraint; PT-2 reads "yes". The accepting SCC with `va = 0`
contains the initial product state, so the stem is empty, and the canonical
loop over the reduced graph is:

| Reduced edge | Permutation `π` | `σ` before | Concrete step | Concrete state after |
| --- | --- | --- | --- | --- |
| `(0,0,0)` —`upd(b)`→ `(0,0,1)` | `(b c)`: `upd(b)` reaches `(0,1,0)`; sorting `b` (version 1) and `c` (version 0) by sort key puts `c` first, giving `(0,0,1)` | identity | `upd(b)` | `(0,1,0)` |
| `(0,0,1)` —`upd(c)`→ `(0,0,2)` | identity | `(b c)` | `upd(b)` | `(0,2,0)` |
| `(0,0,2)` —`upd(c)`→ `(0,0,0)` | identity | `(b c)` | `upd(b)` | `(0,0,0)` |

The concrete state after one pass equals the concrete entry state, so the
loop closes with no repetition. The item settles `refuted` (V-4) with the
lasso `(0,0,0) -upd(b)-> (0,1,0) -upd(b)-> (0,2,0) -upd(b)-> (0,0,0)` bound to
`c = a`. Replay (ADR-018 CX-3) re-executes the three steps, checks
`attemptUpdate` is taken in the loop, and evaluates `always eventually
holds(a.versionNumber = 2)` false at position 0. The record names no
reduction (RV-7). The derived deadlock-freedom item has no terminal state to
find, as in ADR-018 §6, and settles `proved`, `Reduced{[Symmetry{…}]}`, over
the 10 canonical states.

**Verdict under `fair weak each attemptUpdate`.** PT-2 reads "yes", on the
annotated quotient. Take the accepting SCC with `va = 0`: six canonical
states `(0, y, z)` with `y <= z`, entry `(0,0,0)`, edges labelled `upd(b)`
and `upd(c)` (an `upd(a)` edge leaves for `va = 1`, where `q1` has no move
back).

- *Cycle group.* The spanning tree reaches `(0,0,1)` first by `upd(b)` with
  `π = (b c)`, so `u = (b c)` there. The non-tree edge `(0,0,0)` —`upd(c)`→
  `(0,0,1)` with `π` the identity gives the word `(b c)`. `H_C = {id, (b c)}`,
  the whole stabiliser of `a`.
- *Witnesses.* `W_taken` maps to `{upd(b), upd(c)}`, one `H_C`-orbit.
  `attemptUpdate` is enabled everywhere, so `W_disabled` is empty.
- *Test.* The orbits cover `upd(b)` and `upd(c)` but not `upd(a)`, which is
  enabled throughout and never taken. AQ-5 rejects the SCC. The SCC with
  `va = 1` is rejected the same way.

No fair accepting SCC exists, so the instance settles `proved`,
`Proved{basis: Reduced{[Symmetry{…}]}, certification: Uncertified}`, over 30 product states, the same
verdict ADR-018 §6 gives unreduced, where EN-1 explores 135.

**A refutation under `each`.** The claim `eventually always
holds(c.versionNumber != 2)` under `fair weak each attemptUpdate` has as its
negation `always eventually (c.versionNumber = 2)`. For `c = a`, its
accepting SCC spans all 18 canonical states of the stabiliser quotient, with
edges of all three labels, and its cycle group is again `{id, (b c)}`. The
orbits of `W_taken` cover `upd(a)`, `upd(b)` and `upd(c)`, so the SCC passes.
AQ-7 builds a loop that takes all three identities; one such loop is
`upd(a)` three times, then `upd(b)` three times, then `upd(c)` three times,
nine steps from `(0,0,0)` back to it, passing `a = 2` at its third
position. It replays under CX-3, and replay confirms each identity is taken
in the loop.

**Broken symmetry.** Add a field `peers: Set<Reference<ConfigVersion>>`
and a post clause that folds over `self.peers` to choose the new parent. Its
`fold` node traverses a set of references in identity order, so S3 refuses
the unit at that node, naming `config_history` (SYM-8). Without the
annotation the unit checks, and a request declaring symmetry on
`config_history` refuses (SYM-2). Separately, an initial snapshot with `a` at version
1 and the others at 0 is not fixed by the generator `(a b)`, and settles
`SymmetryBroken{config_history, InitialStatesNotClosed{0, (a b)}}`; the
declaration `[[a], [b, c]]` admits it.

#### 7.2 Partial-order reduction: independent bumps

**Subject.** Object type `Counter` with `v: Int[0, 2]`, population
`counters`, universe `{a, b, c}`, all at 0. Operation `bump()` with a
receiver-scoped frame on `v` (POR-3), precondition `self.v < 2` and
postcondition `self.v = pre(self.v) + 1`. Each `bump` reads and writes only
(receiver, `v`), so any two bumps on different receivers are independent.
Write a state as `(va, vb, vc)`.

```text
temporal ReachesTwo using inf over (x: M::Counter) clock "model-steps" on origin {
  eventually holds(x.v = 2)
}
```

**Claim.** The empty fairness set: TP-4, PT-2 "yes". Take the instance `x =
a`. Only `bump(a)` writes a location the atom reads, so `bump(b)` and
`bump(c)` are invisible.

**Reduced search.** Canonical transition order is `bump(a)`, `bump(b)`,
`bump(c)`. The singleton candidates `{bump(b)}` and `{bump(c)}` satisfy C1
(no dependent transition) and C2 (invisible); `{bump(a)}` fails C2.

| State | `E(s)` | `A(s)` | Successor |
| --- | --- | --- | --- |
| `(0,0,0)` | a, b, c | `{bump(b)}` | `(0,1,0)`, new |
| `(0,1,0)` | a, b, c | `{bump(b)}` | `(0,2,0)`, new |
| `(0,2,0)` | a, c | `{bump(c)}` | `(0,2,1)`, new |
| `(0,2,1)` | a, c | `{bump(c)}` | `(0,2,2)`, new |
| `(0,2,2)` | a | `E(s)` | `(1,2,2)` |
| `(1,2,2)` | a | `E(s)` | `(2,2,2)` |
| `(2,2,2)` | none | none (C0) | terminal |

**State counts**, for the instance `x = a`.

| | Unreduced | POR |
| --- | --- | --- |
| Model states | 27 | 7 |
| Model transitions | 54 | 6 |
| Product states for the negation `always (a.v != 2)` | 18 | 6 |

Every behaviour ends in the terminal state `(2,2,2)`, where `a.v = 2`, so no
accepting cycle exists. The instance settles `proved`,
`Proved{basis: Reduced{[PartialOrder{BreadthFirstRevisit}]}, certification: Uncertified}`; the instances
`x = b` and `x = c` reduce the same way with the roles exchanged. The
model declares no `terminal` member, so the request adds the deadlock-freedom
item (ADR-018 DL-3). Both searches reach `(2,2,2)`, which has no successor,
and settle it `refuted` with a six-step prefix: in the reduced search, the
path of the table (C0, C1). By ADR-018 DL-6 that refutation leaves the
verdict of `ReachesTwo` unchanged; a model that halts there on purpose
declares `terminal any` or `terminal when` a predicate that holds at
`(2,2,2)`.

**A form POR does not preserve.** `always[0,2] holds(x.v = 0)` under
event-position false-extension, `on origin`, is TP-2. For `x = a` the subject
refutes it with `bump(a)` at the first step, while the reduced graph's only
path keeps `a.v = 0` through position 4. PT-2 reads "no", so the item settles
`inconclusive`, `ReductionNotPreserving{PartialOrder, TP-2}`, before any
expansion, and the reduced graph never gives a false pass.

#### 7.3 State constraint: a per-object cut

**Subject.** The §7.2 counters with `v: Int[0, 3]` and precondition `self.v <
3`, no POR, and the constraint below, which holds at a state when it holds for
every counter, as an invariant over its context population does (spelling
illustrative, QS-1).

```text
constraint Low using v on M::Counter at current { self.v <= 1 }
```

**State counts.** The subject has 64 states. 8 satisfy the constraint (every
counter at 0 or 1). Their successors outside it are the 12 states with one
counter at 2 and the others at 0 or 1. EN-1 stores 20 states and expands 8.

| Claim, instance `x = a` | Constrained run | Unconstrained run |
| --- | --- | --- |
| `always holds(x.v <= 1)` (TP-1) | `refuted`: the first violation in breadth-first order is the boundary state `(2,0,0)`, prefix `bump(a), bump(a)`; boundary states are read by the monitor, and the prefix replays unchanged | `refuted`, same prefix |
| `always holds(x.v <= 2)` (TP-1) | `inconclusive`, `ConstraintReached{boundary_states: 12}`: every violating state has `a.v = 3`, beyond the boundary | `refuted` at `(3,0,0)` after three steps |
| `eventually holds(x.v = 3)` (TP-4, empty fairness) | `inconclusive`, `ConstraintReached{12}`: expanded states form no cycle, and boundary states are not stutter-extended, so no lasso exists | `proved`, `Exhaustive`: every behaviour ends at `(3,3,3)` |
| Deadlock-freedom item (ADR-018 DL-3) | `inconclusive`, `ConstraintReached{12}`: the terminal state `(3,3,3)` lies beyond the boundary, and boundary states are never deadlocks | `refuted` at `(3,3,3)` after nine steps |

#### 7.4 POR under default scheduler fairness: a branch the channel cannot starve

**Subject.** Object type `Cell` with `done: Boolean`, universe `{c}`, `c.done`
initially false. Operation `setDone()`, frame `modifies self.done`,
precondition `not self.done`, postcondition `self.done`. Channel `h` is
`unordered`, with capacity 2, a non-blocking overflow policy and the faults
`duplicate` and `lose` (ADR-027 ST-5), carrying a one-value message. The
spelling is illustrative.

```text
protocol Mark using p over (k: M::Cell) on origin {
  role w on M;
  run sequence Main {
    parallel Both {
      branch left  send S on h as (m) { true };
      branch right attempt D by w on M::Cell::setDone contracts [DonePre, DonePost] as (d) { true };
    } join all [left, right];
  }
  finish Done as (f) { true };
}

temporal Marked using inf over (k: M::Cell) clock "model-steps" on origin {
  always eventually holds(k.done)
}
```

The clause carries the default scheduler constraints `fair weak whole` over
`left`, `right` and the root (ADR-027 PA-3).

**States.** Write a state as (`c.done`, `left`, `right`, queue length) with
the root's rest point. Fifteen states are reachable: the start, with the root
at `fork`; eight with the root at `join` (`(F, S, D, 0)`, `(T, S, end, 0)`,
`(F, end, D, q)` and `(T, end, end, q)` for `q` in 0 to 2); three with the root
at `finish`; three ended, with `q` in 0 to 2. `duplicate` and `lose` stay
enabled after the protocol finishes, while the queue is non-empty; the ended
state with an empty queue is terminal and intended.

**Visibility.** The atom reads (`c`, `done`), so `attempt(D)` is visible
(POR-5). Under POR-10: `fork` writes the new threads' control, which their
steps' enabledness reads; `send(S)` and `attempt(D)` end their branches and
write the parallel instance that the root's `join` reads. All three are
visible. `duplicate` and `lose` write only `queue(h)`, which no constrained
step's enabledness reads (no thread receives on `h`, and the send does not
block), and they belong to no class (ADR-027 PA-2), so they are invisible.
`join` and `finish` write only the root's own control and its instance, which
no other class reads, so they are invisible.

**Reduced search.** The negation `eventually always holds(not k.done)` has a
two-state automaton, `q0` on every state and accepting `q1` on states with
`c.done` false. The unreduced product has 20 states: 15 with `q0`, 5 with
`q1`. Under POR-6 to POR-10 the ample sets are `{duplicate, lose}` at
`(F, end, D, 1)` with `q0`, `{join}` at the two join-pending states with
`c.done` true and a non-empty queue, and `{finish}` at the two
finish-pending states with a non-empty queue; every other state is fully
expanded, by C2 or by the proviso of POR-8. The reduced product keeps all 20
states, since each is reachable along a kept path, and drops 7 edges: the
`attempt(D)` edge from `(F, end, D, 1)` under `q0`, and the `duplicate` and
`lose` edges at the join-pending and finish-pending states listed. Counts are
hand enumeration.

**Verdict under the default.** The only non-trivial accepting SCC is `(F,
end, D, 1)` and `(F, end, D, 2)` under `q1`, joined by `duplicate` and `lose`.
Both states are fully expanded under `q1` (the proviso, since each reaches
the other). `right`'s class is enabled at both and never taken in the SCC, so
ADR-018 FA-4 rejects it. The item settles `proved`, `Proved{basis:
Reduced{[PartialOrder{BreadthFirstRevisit}]}, certification: Uncertified}`, the verdict the unreduced
product gives.

**Verdict with `scheduling adversarial`.** The fairness set is empty, PT-2's
first liveness row applies, and the same SCC passes. The item settles
`refuted` with the lasso `fork`, `send(S)`, then the loop `duplicate`,
`lose`: the channel's faults run forever and starve `right`.

**What POR-10 adds here.** In this subject the stubborn-set closure already
keeps both branch-ending steps together (the root's `join` is dependent on
each, and its necessary enabling set holds both), so the reduced product is
the same with or without POR-10. §7.5 gives a subject where it is not.

#### 7.5 Where fairness visibility changes the reduced product

The closure of POR-7 keeps together the writers of one member's enabling
footprint while that member is disabled, because each such writer is in the
candidate of the disabled member. For a class of one transition identity,
that is every location the class's enabledness reads. For a class with more
than one member, a `whole` class over several receivers or the steps of one
thread, it is not: writers of different members' enabling footprints are
independent of each other, the closure does not order them, and their order
decides whether the class is enabled at a point. With `fair weak whole go`
over gates `p` and `q`, `go` with pre `self.on` and `flip()` toggling
`self.on` under a receiver-scoped frame, `flip(p), flip(q)` from `(p.on,
q.on) = (T, F)` passes `(F, F)`, where the class is disabled, and the
swapped order passes `(T, T)`, where it is enabled. POR-10 makes each such
writer visible, so C2 keeps their order. POR-10 also covers what C0–C3 alone
do not prevent for any class: an ample set made of a transition the fair
behaviour never takes. The representative then takes it, and if it writes
the class's enabling footprint it can turn a behaviour on which the class is
disabled into one on which the class is enabled forever and never taken. The
filter then rejects the only representative of a real violation. POR-10
makes that transition visible, so C2 keeps it out of any reduced ample set.

**Subject.** Object type `Gate` with `a`, `c`, `d`, `done: Boolean`, one
object `g`, all false, and receiver-scoped frames: `arm()` sets `c` (pre `not
self.c`); `arm2()` sets `d` (pre `self.c and not self.d`); `go()` sets `done`
(pre `self.c and self.d and not self.done`); `toggle()` flips `a`. Canonical
order is `arm`, `arm2`, `go`, `toggle`. The claim is `eventually
holds(x.done)` under `fair weak go`. `go`'s enabledness reads `c` and `d`,
two locations written by other operations. Write a state (`a`, `c`, `d`,
`done`).

**The subject's verdict.** `toggle` forever from `(F,F,F,F)` never enables
`go`, so it is weakly fair, and it never reaches `done`: the claim is
refuted with the loop `toggle, toggle`.

**C0–C3 without POR-10.** `arm` and `arm2` are invisible (the atom reads
`done`). At `(F,F,F,F)`, `{arm}` satisfies C1 (`arm2` and `go` cannot run
before `arm`), C2 and C3, and wins the tie with `{toggle}` on canonical order;
`{arm2}` wins next. The reduced product's only accepting cycle is `(F,T,T,F)
⇄ (T,T,T,F)`, where `go` is enabled throughout and never taken, so FA-4
rejects it and the run would report `proved`.

**With POR-10.** `arm` and `arm2` write `go`'s enabling footprint and are not
in its class, so both are visible. The ample set at `(F,F,F,F)` is
`{toggle}`, `(T,F,F,F)` is fully expanded by the proviso, the cycle
`(F,F,F,F) ⇄ (T,F,F,F)` is kept, `go` is disabled on it, and the item
settles `refuted` with the subject's lasso. With the empty fairness set no
fairness visibility applies, `{arm}` is chosen, and the item is refuted
through the cycle that remains, which is a real violation there. FR-157
carries this subject as its acceptance vectors.

#### 7.6 Complete enabling footprints: seven vectors

Each vector below gives a false `proved` when the enabling footprint is the
precondition's reads alone, or when footprints carry no membership. With
POR-1's membership locations and POR-7's enabling footprint `R(t)`, the
reduced verdict equals the unreduced one. Each was run on an explicit-state
simulation of POR-4 to POR-10 with the breadth-first proviso.

**A guard in a postcondition (safety, C1).** Gate `g` with `armed` (true)
and `alarm` (false), Cell `a` with `v: Int[0, 1]` (0), `g.cell = a`.
`disarm()`: `modifies self.armed`, post `not self.armed`. `boom()`:
`modifies self.alarm`, pre `self.armed`, post `self.alarm and
pre(self.cell.v) = 1`. `set()` on `a`: `modifies self.v`, pre `self.v = 0`,
post `self.v = 1`. Claim `always holds(not g.alarm)`. The subject is
refuted by `set, boom`. With `boom`'s enabling footprint `{(g, armed)}`, the
candidate seeded by `disarm` is `{disarm, boom}`, its enabled part `{disarm}`
is invisible with a new successor, `boom` is disabled for good after it, and
the run reports `proved`. With `R(boom)`, which holds (Cell, `v`, any
object), `set` joins that candidate, the initial state is fully expanded,
and the item settles `refuted` with the subject's prefix `set, boom`.

**`Gate` with the guard moved into the postcondition (fairness, POR-10).**
§7.5's `Gate` with `go()`'s pre `not self.done` and post `self.done and
pre(self.c) and pre(self.d)`. The subject is refuted by `toggle, toggle`
under `fair weak go`. With `go`'s enabling footprint `{(g, done)}`, `arm` and
`arm2` are not fairness-visible, `{arm}` is chosen at `(F,F,F,F)`, and the
run reports `proved`. With `R(go)`, which holds `c` and `d`, both are
visible, the ample set at `(F,F,F,F)` is `{toggle}`, and the item settles
`refuted` with the lasso `toggle, toggle`, over 6 reduced states.

**Deleting the receiver (membership).** Job population `{j, k}` with
fields `armed`, `cancellable`, `bad` and `peer: Option<Reference<Job>>`;
`j.armed` and `j.cancellable` true, `k` neither, `j.peer = k`. `boom()`:
every-object `modifies bad`, pre `self.armed`, post `self.peer.bad`.
`cancel()`: `deletes self`, pre `self.cancellable`. Claim `always holds(not
k.bad)`. The subject is refuted by `boom(j)`. With `W(cancel(j))` a
population-wide membership location that `R(boom(j))` never reads, the two
are independent, `A = {cancel(j)}` at the initial state, nothing is enabled
after it, and the run reports `proved`. With POR-1, `cancel(j)` writes
(jobs, membership of `j`), which `boom(j)` reads, so they are dependent and
the item settles `refuted` by `boom(j)`, over 4 states.

**A type-wide delete (any-object membership).** The previous vector with a
field `target: Option<Reference<Job>>`, `j.cancellable` false, and a third
Job `m`, `cancellable`, `target = j`; `cancel()` has the frame `deletes Job`
and `modifies self.target`, pre `self.cancellable and present(self.target)`,
and post: `pre(self.target)` is no longer a member, every other job stays,
and `self.target` is absent. A type-wide `deletes` fixes no deleted key, so
if `cancel(m)`'s write is resolved to (jobs, membership of `m`), `cancel(m)`
and `boom(j)` are independent, `A = {cancel(m)}`, `j` is deleted, and the run
reports `proved` over 2 states. With POR-1, `cancel(m)` writes (jobs,
membership of any object), which meets `boom(j)`'s receiver membership and
its fields, and the item settles `refuted` by `boom(j)`, over 4 states.

**A delete blocked by a reference.** Jobs `{j, k}` with `cancellable`,
`x: Int[0, 1]` and `peer: Option<Reference<Job>>`; `j.peer = k`,
`k.cancellable` true. `lock()`: `modifies self.cancellable`, pre
`self.cancellable`, post `not self.cancellable`. `unlink()`: `modifies
self.peer`, pre `present(self.peer)`, post `not present(self.peer)`.
`cancel()`: `deletes self`, pre `self.cancellable`. Claim `always holds(k.x =
0)`, which evaluates undefined once `k` is deleted. FR-120 admits no
post-state with a dangling reference, so `cancel(k)` is enabled only after
`unlink(j)`; the subject is refuted by `unlink, cancel` with
`UndefinedEvaluation`. With `R(cancel(k))` lacking the referrer field,
`unlink(j)` writes nothing in `cancel(k)`'s enabling footprint, `A =
{lock(k)}`, and the run reports `proved` over 3 states. With POR-1's read of
(Job, `peer`, any object), `unlink(j)` joins the candidate, the initial state
is fully expanded, and the item settles `refuted`, over 5 states.

**A navigation through a rewritten reference.** Gate `g` with `armed` (true),
`alarm` (false), `cell` set to Cell `a` and `alt` set to Cell `b`; `a.v = 0`,
`b.v = 1`. `disarm()`: `modifies self.armed`, post `not self.armed`.
`point()`: `modifies self.cell`, pre `self.cell != self.alt`, post `self.cell
= pre(self.alt)`. `boom()`: `modifies self.alarm`, pre `self.armed and
self.cell.v = 1`, post `self.alarm`. Claim `always holds(not g.alarm)`. The
subject is refuted by `point, boom`. With `R(boom)` holding only (g,
`armed`) and (Cell, `v`, any object), `A = {disarm}` and the run reports
`proved` over 3 states. With POR-1's read of the intermediate field (g,
`cell`), `point` joins the candidate seeded by `disarm` through the disabled
`boom`, and the item settles `refuted`, over 6 states.

**A postcondition reading through `result`.** Gate `g` with `armed` (true)
and `alarm` (false); Cell `a` with `v = 0` in population `cells`.
`disarm()`: `modifies self.armed`, post `not self.armed`. `set()` on `a`:
`modifies self.v`, pre `self.v = 0`, post `self.v = 1`. `pick():
Reference<Cell>`: `modifies self.alarm`, pre `self.armed`, post `self.alarm
and result.v = 1`. Claim `always holds(not g.alarm)`. FR-120 ranges
`result` over the post-state, so `pick` is enabled only when some Cell has
`v = 1`; the subject is refuted by `set, pick`. With no read derived for
`result`, `A = {disarm}` and the run reports `proved` over 3 states. With
POR-1's read of (Cell, `v`, any object) and the any-object membership of
`cells`, `set` joins the candidate, and the item settles `refuted`, over 6
states.

### 8. Downstream impact and sequencing

**Stage DAG (ADR-011 §1).** No stage or edge is added. Reductions run inside
ADR-018's S6c. S3 gains the `symmetric` annotation and its refusal of
identity-observing forms (SYM-8), and the read footprint of each operation
(POR-1), which EN-1 reads.

| ID | Repository | Change |
| --- | --- | --- |
| DS-1 | QSL | S3: the `symmetric` annotation and its refusal of SYM-3 breaking forms (SYM-8), POR-1 read footprints, receiver-scoped frame entries (POR-3) and the constraint clause (SC-1). `qsl-eval`: FR-120's `check_frame` at entry scope, the restricted observation (POR-2), FR-101's `TransitionSystem` hooks (EI-4), `Footprint` and `ConstraintValue`. `quire-semantic-value`: `Permutation`. `qsl-analyze` `model_check` (layer A, ADR-018 LA-1): the pre-check (EI-1), canonicaliser (SYM-7), the annotated quotient's cycle groups and `each` tests (AQ-2 to AQ-7), ample sets and proviso (POR-6 to POR-8), constraint cut (SC-2), concretisation (EI-6). `qsl-replay`: `ProofBasis::Reduced` and the three `InconclusiveCause` variants. The EN-1 manifest's reductions (RV-6). |
| DS-2 | CG | The negotiation arm routes and never drops a reduction (RV-6); obligation identity binds the selection (RV-8); the frame-effect obligation covers every non-modified receiver field and, for a receiver-scoped entry, every other object (TX-1 precondition, part 3). |
| DS-3 | IR | The v2 frame entry carries its scope (TX-1). Measurement of `Reduced` as its own technique (TX-4). |
| DS-4 | QSpec | §9. |

**Sequencing.**

1. **Specification.** This record, then QSpec (§9), then the QSL compiler
   requirements.
2. **Prerequisite.** ADR-018 EN-1 (its steps 2 and 3).
3. **Symmetry and state constraints.** Both need only the `symmetric`
   annotation and its S3 refusal, the canonicaliser, the annotated quotient,
   the constraint cut and concretisation on EN-1. They run
   in parallel, and §7.1 and §7.3 are their first conformance vectors.
4. **POR over `ModelSystem`.** After receiver-scoped frame entries reach
   QSpec, S3, FR-120 and the checked package, and S3 derives read
   footprints. §7.2 is its first conformance vector.
5. **POR over `ProtocolSystem`.** With ADR-027's protocol system (POR-9),
   its footprints (FT-1, FT-3) and fairness visibility (POR-10) for its
   default scheduler constraints. §7.4 is its first conformance vector, and
   §7.5 the vector where fairness visibility changes the result.

### 9. What QSpec must specify

| ID | Item | Where |
| --- | --- | --- |
| QS-1 | Request members: symmetry declarations (population, key classes), POR selection, state constraint clause reference; the constraint clause's surface syntax; symmetry and POR in the obligation identity, the constraint in the method | QSpec FR-382, FR-331, FR-290 |
| QS-2 | Reduced exhaustive exploration: the sort canonicaliser and its sort key (SYM-7), coalescing on canonical keys, ample sets C0–C3 with the breadth-first revisit proviso (POR-6 to POR-8), boundary states (SC-2); and the matching clause in QSpec FR-181-AC-4 that defers to it | QSpec FR-383 beside FR-181; FR-181-AC-4 |
| QS-3 | Symmetry: the `symmetric` population annotation and its authoring-time refusal of identity-observing forms (SYM-3, SYM-8), the request opt-in on annotated populations only (SYM-1, SYM-2), generator closure of initial states (SYM-4), binding orbits and stabilisers (SYM-6), and `each` fairness on the annotated quotient (AQ-1 to AQ-7) | QSpec FR-381, FR-382 and FR-384 |
| QS-4 | Receiver-scoped `modifies` entries: surface syntax, QSpec FR-013 semantics, the scope member of the checked-package v2 frame entry; read footprint derivation, membership locations and the enabling footprint `R(t)` (POR-1, POR-4, POR-7) | QSpec FR-013, FR-340 and FR-383 |
| QS-5 | The normative preservation table (PT-2) | QSpec FR-385 |
| QS-6 | Verdicts: `Reduced` in the FR-331 terminal record with its reductions and rows; `ReductionNotPreserving`, `SymmetryBroken`, `ConstraintReached` onto QSpec FR-360 and FR-243 (`inconclusive`, `unsettled`) as wire causes `reduction-not-preserving`, `symmetry-broken` and `constraint-reached`, and `Reduced` as proof basis `reduced` beside `exhaustive`; a POR run to its horizon as `reduction-horizon` and a limit reached under any reduction as `limit-reached` (RV-5); a symmetry- or partial-order-reduced proof labelled `uncertified` (RV-1); the reduced-versus-unreduced agreement requirement (RV-10) | QSpec FR-385, FR-331, FR-360 |
| QS-7 | Counterexamples from a reduced search are concrete model traces (EI-6), so QSpec FR-181-AC-5 and the counterexample contract need no new trace kind | QSpec FR-181, the counterexample contract |
| QS-8 | Stuttering invariance of the infinite-trace grammar: confirm `since` and `triggered` are non-strict, as `until` is | QSpec FR-161 |
| QS-9 | Provider advertisement of reductions; negotiation never drops or adds one | QSpec FR-290 and the provider manifest contract |
| QS-10 | Conformance vectors, each with expected verdict and, for a refutation, a counterexample that must replay: (a) §7.1, the 27-to-10 orbit count, the lifted lasso, the `each` proof and `each` refutation on the annotated quotient, the `fold` refusal at S3, the request refused on an unannotated population, the initial-state break and the `[[a], [b, c]]` admission; (b) §7.2, the 27-to-7 reduction, the deadlock-freedom refutation found by the reduced search and TP-2 not preserving; (c) §7.3, the three claims and the deadlock-freedom row, including a constraint-boundary lasso that is not reported; (d) a symmetry lasso whose loop needs more than one repetition to close; (e) §7.4, its proof under default scheduler fairness and its refutation under `scheduling adversarial`, both under POR; (f) §7.5, the refutation under `fair weak go` with fairness visibility, and the empty-fairness refutation; (g) §7.6's four vectors, each refuted under POR as unreduced; (h) every vector above, run reduced and unreduced with the same verdict (RV-10) | QSpec TC-210 neighbourhood and new TCs |

### 10. Rulings on the draft's questions

The owner ruled on the draft's five questions on 2026-10-01.

| ID | Question | Ruling | Rationale | Where it lands |
| --- | --- | --- | --- | --- |
| RU-1 | Add receiver-scoped frame entries, or defer POR | **Add them, as design.** A `modifies` entry is written `modifies self.f` to grant writes to field `f` on the receiver only, or `modifies f` to grant them on every object whose effective type has `f`, QSpec FR-013's existing meaning. The checked-package frame entry carries the scope, `receiver` or `every-object`. Under `receiver`, a change to `f` on any object other than the receiver is `frame_violation`, decided by FR-120's `check_frame` on the model and, for code, by the frame-effect precondition of TX-1. The spelling is illustrative; the shared grammar owns it when requirement work follows | POR's independence needs object-granular write footprints, and only a receiver-scoped entry gives them (POR-3). The entry also states more precisely what an operation may change, for every consumer of the frame | POR-1, POR-3, TX-1, QS-4 |
| RU-2 | Exact least orbit member, or a cheaper canonicaliser | **Sort-based canonicaliser.** Sort each class's objects by their key-free encoding and reassign keys in sorted order | `O(n log n)` per class in place of factorial. It is sound because every state maps to a member of its own orbit, so no reachable behaviour is lost; it may keep a few duplicate representatives of one orbit, which costs states and never changes a verdict | SYM-7, EI-6, §7.1 |
| RU-3 | Keep state constraints, or drop them | **Keep them.** SC-1 to SC-3 stand | A constraint is a cheap search cut for finding bugs in a subject too large to finish, and its verdict rule (RV-4) keeps a constrained pass from reading as a proof | §1.3, RV-4, §7.3 |
| RU-4 | Symmetry with `each` fairness: `inconclusive`, or an annotated quotient | **Specify the annotated quotient now.** Liveness under weak and strong `each` fairness is supported under symmetry | `whole` is the unmarked granularity, but `each` is what per-receiver progress claims use; leaving it `inconclusive` would make symmetry useless for them. The edge permutations EN-1 already retains are the annotation, so the cost is the cycle-group and orbit computation per SCC (AQ-8) | AQ-1 to AQ-8, PT-2, EI-6, §7.1 |
| RU-5 | Symmetry in source as well as on the request | **Add a `symmetric` population annotation, and require it for the reduction.** S3 refuses identity-observing forms on an annotated population at authoring time; a request opts in with key classes, and a request declaring symmetry on an unannotated population refuses | The model's half of symmetry is a property of source, and a located S3 refusal while authoring is better than an `inconclusive` after a request. One admission path keeps the run's pre-check to the initial-state check, and the annotation in the checked package binds every symmetry verdict to a model that promised it. Keeping request-time admission for unannotated populations was rejected: it keeps two paths for one fact, and lets a model lose its symmetry silently when a clause changes | SYM-1, SYM-2, SYM-3, SYM-5, SYM-8, RV-3 |

## Consequences

- Exhaustive model checking scales by orbit size for symmetric models and by
  interleaving count for models with independent operations, and a state
  constraint gives a cheap search for bugs in a subject too large to finish.
- Every reduction is checked before it runs: identity-observing forms are
  refused at authoring time on a `symmetric` population, and a selection the
  claim or the initial states do not admit settles a named `inconclusive`
  cause, so a declaration TLC would trust never gives a wrong verdict in QSL.
- Liveness under weak and strong `each` fairness keeps the symmetry
  reduction, decided on the annotated quotient at a per-SCC cost in cycle
  groups and orbits.
- Counterexamples stay concrete model traces and replay without the engine
  or the reduction; nothing downstream sees a reduced trace.
- A reduced proof is visibly its own technique with its own bounds, and its
  transfer to code names the frame and identity-order preconditions it rests
  on.
- POR needs receiver-scoped frame entries, a new authored form that also
  tightens what a frame promises about code.

## Amendments to make on acceptance

- **ADR-018** §1: `ProofBasis::Reduced{reductions}` beside `Exhaustive`; V-6
  gains `ReductionNotPreserving`, `SymmetryBroken`, `ConstraintReached` and
  `ReductionHorizon`; V-5 notes RV-5. §3 EN-1: the reductions of this record and the pre-check
  EI-1; the canonical counterexample is chosen over the reduced graph and
  concretised (EI-6). §10 DL-7: a boundary
  state (SC-2) is not expanded and is never a deadlock. Alternatives
  "On-the-fly emptiness during exploration": this record keeps two phases.
- **ADR-013** O-16 and the `TerminalValue` row: `ProofBasis::Reduced` maps to
  success; the three new `InconclusiveCause` variants map to inconclusive.
- **ADR-014** §1 B-5: a state constraint is a B-5 method parameter of EN-1,
  never a B-1 or B-4 value. §3 TR-7: under symmetry the frontier holds
  canonical state keys.
- **QSpec FR-181** (QSpec FR-383 states the reduced exploration): AC-4 coalesces only
  equal complete state keys unless the request selects a reduction under the
  new reduced-exploration FR (QS-2); AC-5 holds for concretised traces.
- **ADR-011** §6.1: layer 3 `check` gains the `symmetric` annotation check
  and read footprints; layer 5 `simulation` gains the `TransitionSystem`
  hooks; layer A `qsl-analyze` `model_check` gains the reduction machinery;
  `quire-semantic-value` gains `Permutation`. §1: no stage or edge change.
- **ADR-019** (when accepted): §4 notes strong fairness is preserved under
  POR with fairness visibility (POR-10) and under symmetry, `each`
  constraints through the annotated quotient (AQ-6).
- **ADR-027** (when accepted): its preservation row for scheduler fairness is
  PT-2's `whole` row with POR-10; FT-1 and FT-3 are POR-1's protocol
  locations and POR-7's enabling footprints, and FT-3's set for each step
  holds at least the locations POR-7 lists for a protocol step: the
  operation's precondition and postcondition pre-state reads, receiver and
  argument membership, the binders its arguments read, its own control and
  instance locations, and `queue(h)` for `duplicate` and `lose`.
- **ADR-020** (when accepted): §9's note on state-space reduction points to
  §3 of this record; `each` rows in `F_C` and `F_A` are supported under
  symmetry.
- `spec/spec.md`: index row.

## Alternatives Considered

- **Trust the symmetry declaration, as TLC does.** Rejected. A wrong
  declaration gives a wrong `proved`, and QSL's typing already supplies most
  of the check (SYM-3).
- **The least orbit member as canonical form.** Rejected (RU-2). It stores
  exactly one state per orbit, at a worst-case cost of the factorial of the
  largest class per state; computing it in general is the orbit problem,
  which is at least as hard as graph isomorphism (Clarke, Emerson, Jha and
  Sistla). The sort canonicaliser is `O(n log n)` per class,
  stays deterministic because it is a function of the state, and costs only
  occasional duplicate representatives.
- **Symmetry declared in source only.** Rejected (RU-5). The key classes and
  the initial-state check depend on the universe and the initial snapshot,
  which are request members; the source annotation carries the model's half.
- **Request-time admission of identity transparency.** Rejected (RU-5): a
  located authoring-time refusal on an annotated population replaces it.
- **Silent unreduced fallback when a reduction does not preserve a claim.**
  Rejected (RV-9). It would answer a request with a different technique than
  it named, under a different identity.
- **`unsupported` for a non-preserving reduction.** Not chosen. The candidate
  supports the form; the request's method cannot settle it, which is
  `inconclusive` with a named cause, as `BoundReached` is for a run to its
  horizon.
- **Dynamic, per-state footprints for POR.** Rejected for this record.
  Static footprints from frames and clause bodies are enforced facts
  (POR-2); per-state footprints would need their own enforcement.
- **POR from type-wide frames only.** Rejected. Every pair of transitions of
  one operation would be dependent, and the reduction would be nearly empty.
- **A state constraint that settles `proved` over the constrained graph.**
  Rejected. It would read as a proof about the subject. ADR-018 keeps a
  pass to a finite horizon `inconclusive`, and a constraint-limited pass is
  the same strength.
- **Counterexamples over the quotient with a permutation per step.**
  Rejected. Replay and every downstream consumer would need the reduction to
  read them; concretisation keeps one trace kind.

## References

- Owning ticket: Linear QSL-368. Built on ADR-018 (Linear QSL-366). Interacts with ADR-019 (Linear QSL-365, strong fairness) and
  ADR-020 (Linear QSL-367, refinement mappings). QSpec FR-381 to FR-385,
  with the amended QSpec FR-013, FR-181 and FR-340, carry §9 (Linear
  STD-134). QSpec FR-383 owns the footprint locations, meets and derivation
  POR-1 and POR-7 cite.
- Review: SR-1140 (reviews/qsl-368-spec-review.md).
- The IR proof-coverage measurement (Linear IR-361) and the IR Kani
  tractability harness identity (Linear IR-340) consume §6; the IR evidence
  chain is Linear IR-334. CG ADR-003 (Kani tractability) on CG `main` rules
  on the shadow path (Q1) and on tightened bounds in the harness identity
  (Q2).
- Later research on lowering to TLA+ and TLC: Linear RES-42.

Each source below names the step it supports. POR-8's proviso and POR-11's
steps 1 to 3 and 5 to 8 are proved in this record.

- D. Peled, "All from one, one for all: on model checking using
  representatives", Computer Aided Verification (CAV 1993), LNCS 697,
  Springer, 1993, pp. 409-423: model checking over one representative of
  each equivalence class of behaviours, the basis of ample sets (Context).
- D. Peled, "Combining partial order reductions with on-the-fly
  model-checking", Formal Methods in System Design 8, 1996, pp. 39-64
  (conference version CAV 1994, LNCS 818): the conditions under which
  partial-order reduction combines with the product with a property
  automaton, used in POR-11 step 4 and §2.
- E. M. Clarke, O. Grumberg and D. Peled, *Model Checking*, MIT Press,
  1999: the ample-set conditions C0 to C3 (POR-6) and the correctness
  proof whose construction and stuttering-equivalence result POR-11 steps
  4 and 5(d) use.
- P. Godefroid, *Partial-Order Methods for the Verification of Concurrent
  Systems: An Approach to the State-Explosion Problem*, LNCS 1032,
  Springer, 1996: persistent sets (POR-7) and the persistent-set search
  that reaches every reachable deadlock (§2, "POR and deadlocks").
- A. Valmari, "Stubborn sets for reduced state space generation", Advances
  in Petri Nets 1990, LNCS 483, Springer, 1991: stubborn sets, whose closure
  over enabled and disabled members POR-7 follows.
- G. J. Holzmann and D. Peled, "An improvement in formal verification",
  Formal Description Techniques VII (FORTE 1994), Chapman & Hall, 1994,
  pp. 197-211: partial-order reduction in SPIN (Context).
- D. Peled and T. Wilke, "Stutter-invariant temporal properties are
  expressible without the next-time operator", Information Processing
  Letters 63(5), 1997, pp. 243-246: the properties invariant under
  stuttering are those LTL without next expresses, the converse of the
  direction §2 uses (§2).
- L. Lamport, "What good is temporal logic?", Information Processing 83
  (IFIP 9th World Computer Congress), North-Holland, 1983, pp. 657-668:
  formulas without the next-time operator are invariant under stuttering
  (§2, POR-11 step 5(d)).
- D. Bošnački, S. Leue and A. Lluch Lafuente, "Partial-order reduction for
  general state exploring algorithms", Model Checking Software (SPIN 2006),
  LNCS 3925, Springer, 2006, pp. 271-287: cycle provisos stated over the
  open and closed sets of a non-depth-first search, the form of POR-8.
- C. N. Ip and D. L. Dill, "Better verification through symmetry", Formal
  Methods in System Design 9, 1996, pp. 41-75: scalarsets, the
  argument SYM-4 to SYM-6 rest on (Context, §1.1).
- E. A. Emerson and A. P. Sistla, "Symmetry and model checking", Formal
  Methods in System Design 9, 1996, pp. 105-131: the quotient by a
  symmetry group is bisimilar to the subject on symmetric properties (§2).
- E. M. Clarke, E. A. Emerson, S. Jha and A. P. Sistla, "Symmetry
  reductions in model checking", Computer Aided Verification (CAV 1998),
  LNCS 1427, Springer, 1998, pp. 147-158: the orbit problem, which makes
  the least orbit member costly to compute (RU-2).
- E. A. Emerson and A. P. Sistla, "Utilizing symmetry when model-checking
  under fairness assumptions: an automata-theoretic approach", ACM
  Transactions on Programming Languages and Systems 19(4), 1997,
  pp. 617-638: the annotated quotient structure for fairness under
  symmetry, which AQ-1 to AQ-7 follow.
- E. A. Emerson, S. Jha and D. Peled, "Combining partial order and symmetry
  reductions", Tools and Algorithms for the Construction and Analysis of
  Systems (TACAS 1997), LNCS 1217, Springer, 1997, pp. 19-34: symmetry and
  partial-order reduction applied together (§2, "Combining reductions").
- L. Lamport, *Specifying Systems: The TLA+ Language and Tools for Hardware
  and Software Engineers*, Addison-Wesley, 2002, chapter 14: TLC's
  `SYMMETRY` and `CONSTRAINT`, and §14.3.4's statement that TLC may check
  temporal properties incorrectly under a symmetry set (Context, §2).
