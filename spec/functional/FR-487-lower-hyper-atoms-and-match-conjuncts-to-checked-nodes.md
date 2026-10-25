---
id: FR-487
title: "Lower a hyper clause's holds atoms and match conjuncts to checked nodes"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-021
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-023
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-107
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-172
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-337
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-175
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-176
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-315
    type: traces_to
---
# FR-487: Lower a hyper clause's holds atoms and match conjuncts to checked nodes

## Description

The S3 hyper check SHALL lower each `holds` atom of a checked hyper clause
over `behaviours` and each conjunct of its `match` block (`μ_U` and `μ_E`)
to a checked Boolean node that FR-107's one clause evaluator runs and that
FR-315's SMT reader reads, so no caller supplies an evaluator seam for
them. Today a body atom is `FormulaNode::Holds { span }` only, and a
conjunct is a `CheckedStepPredicate` with a span and its step-label reads
(`qsl-semantics/src/check/hyper_clause.rs`). The product reads both through
the caller seams `HyperLetters` and `StepMatch`
(`qsl-eval/src/hyper/seam.rs`), which only tests implement, by reading
source text. A temporal clause's atoms already have this form:
`TemporalClauseNode::condition` (FR-337) holds each atom's checked
condition over its `over` parameter, and `ClauseLetters` runs it through
FR-107's evaluator.

This requirement is the lowering and its evaluation. FR-315's
`encode_hyper_query` (Linear QSL-1010) reads the result. Emitting these
nodes in the clause's `hyperproperty` node is Linear QSL-1071, and is not
part of this requirement.

## Inputs

- A `HyperBehavioursForm` (FR-171) that FR-172 checks to a
  `CheckedHyperClause`: its object parameters, its quantifier prefix, its
  body's `holds` atoms (each `holds(c)` with its condition expression `c`)
  and its `match` conjuncts as FR-172 splits them.
- The unit's type scope, declared functions, admitted models and checking
  limits, as FR-337's temporal conditions are typed against them.
- To evaluate: a `CheckedPackageEvaluation` (FR-107) over the package that
  holds the clause, a `Meter`, and for an atom the component states of one
  joint position, or for a conjunct the step labels of one joint step.

## Outputs

```rust
// qsl-semantics, on the checked clause (FR-172 "Outputs").
impl CheckedHyperClause {
    /// One entry per atom slot, in slot order.
    pub fn atom_slots(&self) -> &[AtomSlot];
    /// The checked condition of the `holds` atom `atom` and its slot
    /// count; `None` for a node that is no `holds` atom of the body.
    pub fn condition(&self, atom: FormulaNodeId) -> Option<(CheckedNode<'_>, usize)>;
}

pub struct AtomSlot {
    pub trace: usize,     // the trace variable's index in quantifier order
    pub parameter: usize, // the object parameter's index in source order
}

impl CheckedStepPredicate {
    /// The conjunct's distinct step-label reads, slot `k` being entry `k`.
    pub fn slots(&self) -> &[StepRead];
    /// The conjunct's checked condition and its slot count.
    pub fn condition(&self) -> (CheckedNode<'_>, usize);
}

// qsl-eval, beside `evaluate_holds` on `CheckedPackageEvaluation` (FR-107).
fn evaluate_hyper_holds(
    &self,
    clause: &CheckedHyperClause,
    atom: FormulaNodeId,
    slots: Vec<Value>,                 // one per `clause.atom_slots()` entry
    components: &[&ObjectEnvironment], // one per trace variable, quantifier order
    meter: &mut Meter,
) -> Result<Evaluation, CallFailure>;

fn evaluate_step_predicate(
    &self,
    clause: &CheckedHyperClause,
    part: MatchPart,           // `Universal` (μ_U) or `Existential` (μ_E)
    conjunct: usize,           // its index in `mu_universal()` or `mu_existential()`
    labels: &[StepLabel<'_>],  // as `StepMatch` receives them for `part`
    meter: &mut Meter,
) -> Result<Evaluation, CallFailure>;
```

- A typed `CheckRefusal` with a span, and no checked clause, when an atom or
  conjunct does not type or its definedness check refuses.

## Behavior

### Atom slots

- The clause's atom slots SHALL be the pairs (trace variable `t`, object
  parameter `p`) whose alias is the same for `t` and `p` (FR-172 "Object
  parameters"), ordered by `t` in quantifier order and then by `p` in source
  order. Slot `k` is the `k`-th pair. Every atom of the clause has the same
  slot list, and the locals it binds take slots after it.
- Each atom slot SHALL be typed `Reference<T>`, `T` being the object type
  that `p` declares (`v: V::T`).

### Lowering a holds atom

- The S3 hyper check SHALL type each atom's condition `c` once, with the value
  typer, as FR-337's `check_conditions` types a temporal atom's condition
  (FR-104's invariant state-clause typing, no `self`, every read a model
  read of the current state) over the clause's atom slots in place of the
  one `over` slot. FR-172 "Body expressions" sets which expressions
  refuse and where. The typed result SHALL be the atom's checked condition.
- Inside an indexed read `e @ t`, the typer SHALL read each name of an object
  parameter `p` as the atom slot (`t`, `p`). The `@ t` node itself lowers to
  no node of its own: `v.l @ a` lowers to the field read `l` of slot
  (`a`, `v`).
- Outside every indexed read, a bare object-parameter name `p` that is not
  the operand of a member read SHALL read the slot (`t`, `p`) of the first
  trace variable `t` of `p`'s alias in quantifier order. Within an instance,
  `p` denotes one key in every trace of its alias (ADR-023 HS-3), and FR-107
  compares references ignoring the observation, so the choice of trace does
  not change the value.
- The S3 hyper check SHALL run FR-104's definedness check over each condition,
  with the clause's check mode, as `check_conditions` does for a temporal
  atom. Each slot SHALL be read at observation `current`.
- The checked conditions SHALL count toward the package's node ceiling and
  checking work, as a temporal clause's conditions do (FR-337, NFR-011).
- Each condition SHALL be located at the site `hyper <c> holds <n>`, an
  `Origin::TypeDeclaration` name in the form FR-337's `temporal_site` gives
  a temporal condition. Here `c` is the clause's index among the unit's
  hyper clauses over `behaviours` and `n` is the atom's formula-node index.
  The body arena and its `FormulaNode::Holds` nodes are unchanged, and the
  conditions are keyed by that index.

### Lowering a match conjunct

- A conjunct's slots SHALL be its distinct step-label reads, in order of
  first read. `stutter` is no read: it is a literal. Slot `k` SHALL be typed
  by its read:
  - `t.step.op`: `String`, the operation's label;
  - `t.step.receiver`: `Option<Reference<T>>`, `T` the one object type that
    declares every operation of `t`'s model that has a receiver (QSpec FR-395
    "Step labels");
  - `t.step.O.p`: `Option<T>` for parameter `p: T` of `O` (FR-172).
- If the operations of `t`'s model that have a receiver are declared by more
  than one object type, then a conjunct that reads `t.step.receiver` SHALL
  be refused `unsupported_construct`/`expression-form` at the read. No
  single receiver type exists for such a read, and this requirement gives
  that model's receivers no union type.
- The S3 hyper check SHALL type each conjunct once, with the value typer over
  its slots, under FR-172's `match` rules. `stutter` SHALL lower to the
  `String` literal `stutter`. A label constant `O`, an operand of `=` or `!=`
  beside `t.step.op` (FR-172 "Match"), SHALL lower to the `String` literal
  equal to the operation text `ModelTransition::operation()` carries for an
  `O` step. That text is a canonical RFC 8785 document and `stutter` is not
  one, so no operation's label equals `stutter`.
- A call of a declared function in a conjunct SHALL be typed as FR-104 types
  a call. A call reads no state, so FR-172's state-read refusal does not
  apply to it. FR-315-AC-10's conjunct is such a call.
- The S3 hyper check SHALL run FR-104's definedness check over each conjunct
  with the clause's check mode, as for an atom.
- Each conjunct of `μ_U` SHALL be located at the site `hyper <c> match <k>`,
  and each conjunct of `μ_E` at `hyper <c> match exists <k>`, `k` being the
  conjunct's index in `mu_universal()` or `mu_existential()`.

### Identity

- The lowering SHALL add no member to `HyperIdentity` (FR-172 "Identity",
  ADR-023 HV-7): the body and `μ` already bind every atom and conjunct. The
  same clause checked twice SHALL give equal conditions, equal slot lists
  and equal identities.

### Evaluating an atom

- `evaluate_hyper_holds` SHALL run the atom's condition through FR-107's
  evaluator (the `Machine` that `evaluate_holds` runs), with slot `k` bound
  to `slots[k]`.
- Slot `k`'s value SHALL be read in `components[atom_slots()[k].trace]`. A
  reference read through a slot keeps the component it was read in, as an
  FR-107 reference keeps its observation. A field read through it, or a
  `deref` of it, SHALL read that same component's environment. References
  compare by (universe, object type, key), ignoring the component (FR-107).
- If `slots` does not hold one value per atom slot, `components` does not
  hold one environment per trace variable, or `atom` is no `holds` atom of
  the clause, then `evaluate_hyper_holds` SHALL return
  `CallFailure::Fault(InternalFault)` and charge nothing.
- The evaluation SHALL charge `meter` exactly as FR-107 charges the same
  condition's nodes, and binding the slots SHALL charge nothing. When the
  meter denies a charge, the result SHALL be `Outcome::Incomplete` with
  `resource_exhausted`/`insufficient-next-charge`.

### Evaluating a match conjunct

- `evaluate_step_predicate` SHALL run the conjunct's condition through FR-107's
  evaluator, with slot `k` bound from the label of the trace variable that
  read `k` reads, in `labels`:
  - `t.step.op`: the step's `ModelTransition::operation()` text, or `stutter`
    for `StepLabel::Stutter`;
  - `t.step.receiver`: none at `StepLabel::Stutter` and at a step with no
    receiver, otherwise the reference that the step's receiver text names;
  - `t.step.O.p`: present exactly when the step is an `O` step, with the
    argument at `p`'s parameter index, decoded from FR-120's typed canonical
    form as FR-128's replay decodes it.
- A step label is the step's transition identity (ADR-023 HS-5), so the
  conjunct reads no state and `evaluate_step_predicate` takes no object
  environment and no instance.
- If `conjunct` is out of range for `part`, `labels` does not hold one label
  per trace variable that `part` reads, or a label's receiver or argument
  text does not decode at its declared type, then `evaluate_step_predicate`
  SHALL return `CallFailure::Fault(InternalFault)` and charge nothing.
- The conjunct SHALL charge `meter` exactly as FR-107 charges its nodes, and
  binding its slots SHALL charge nothing.

### The clause seams

`qsl-eval` SHALL implement `HyperLetters` and `StepMatch` over the lowered
nodes as `CheckedHyperLetters` and `CheckedStepMatch`, in its `hyper`
module beside the traits. These are the production seams through which
FR-176's product, FR-177's witness product, FR-178's projected product and
FR-183's replay evaluate the body's atoms and `μ` through FR-107's one
evaluator.

- `CheckedHyperLetters::atom` SHALL observe each component state through its
  own component's `ModelSystem`, bind slot (`t`, `p`) to the reference of
  `p`'s instance key in component `t`, and evaluate through
  `evaluate_hyper_holds`. A key that names no object of component `t`'s
  state SHALL be bound the way `ClauseLetters` binds an absent instance key,
  in that component only. Its outcome maps to `AtomReading` as
  `ClauseLetters` maps it.
- `CheckedStepMatch::universal` and `existential` SHALL evaluate the
  conjuncts of their part in index order, each once, through
  `evaluate_step_predicate`, sharing the one `meter`, and stop at the first
  conjunct that is not `Completed(true)`. A conjunct after it is neither
  evaluated nor charged, as FR-107 skips the right operand of a decided
  `and`.
- `MatchReading` and the error of `StepMatch::universal` and `existential`
  SHALL name the conjunct that decides a reading other than `Holds`.
  `Refused` gains `conjunct` and the refusal's `ClauseOutcome`, and an
  exhausted meter is `Err(MatchIncomplete { conjunct, incomplete })` in
  place of `Err(Incomplete)`:

  ```rust
  pub enum MatchReading {
      Holds,                                                // every conjunct completed `true`
      Fails,                                                // the first conjunct not `true` completed `false`
      Undefined { conjunct: usize, cause: UndefinedCause }, // the first such conjunct had no value
      Refused { conjunct: usize, outcome: ClauseOutcome },  // the first such conjunct was refused
  }
  pub struct MatchIncomplete { pub conjunct: usize, pub incomplete: Incomplete }
  ```

  `conjunct` is the index in `mu_universal()` for `universal` and in
  `mu_existential()` for `existential`. A clause with no conjunct in a part
  reads `Holds` for that part and charges nothing.
- Every other implementation of `StepMatch`, test seams included, SHALL
  return the same shapes. The products read `Refused` and `MatchIncomplete`
  as they read `Refused` and `Err(Incomplete)` today (FR-176
  `MatchUndetermined`, FR-177, FR-178), and keep the conjunct for a refusal
  locus.

### Reading the lowered nodes in an SMT query

FR-315's hyper self-composition queries read these nodes as follows. No
evaluator seam is added to `HyperQueryRequest`.

- **Atoms.** At joint position `i`, the reader SHALL read an atom's
  condition with atom slot (`t`, `p`) bound to the layout object of `p`'s
  key in `request.instance`, in trace variable `t`'s copy `s<i>.<t>`. A field
  read through a reference read from copy `t` SHALL read copy `t`. The atom
  gives its value term and its definedness term, as a temporal atom does
  through `Reader::boolean`.
- **The property of an `always` body.** For a body `always φ`, `φ` being a
  Boolean combination of `holds` atoms with no temporal operator (FR-315
  "Property"), the property at position `i` SHALL be `φ` over the atoms'
  terms at `i`. Its definedness conditions SHALL be those of the atoms
  `φ`'s short-circuit reads at `i`, as FR-315's reader builds a temporal
  atom's condition under an `and`.
- **The property of any other body.** For any other HP-2 body in the safety
  fragment, the monitor SHALL be the one FR-315's monitor tabulation gives a
  temporal clause with the same formula, profile and an `on origin`
  activation. The automaton is `translate_body(body, profile, limit)` under
  `Monitor::bad_prefix`. It is tabulated as `MonitorTable::of` tabulates
  (breadth-first numbering, initial state 0, letter bit `j` the table's
  atom `j`), under the request's `max_monitor_atoms` and
  `max_monitor_states`. `MonitorTable` SHALL gain a constructor over a
  `CheckedFormula` and profile that `MonitorTable::of` delegates to for an
  `on origin` clause, so the two tabulations cannot differ. Letter bit `j`
  at position `i` is atom `j`'s term at `i`. The query has one instance, so
  its monitor state is `m<i>` (FR-315 "Copies").
- **`μ_U`.** For each tuple of component disjuncts, in FR-315's joint-step
  order, the encoder SHALL evaluate `μ_U` through `CheckedStepMatch::universal`
  with one label per trace variable: the label of an operation disjunct is
  the `ModelTransition` that FR-120 gives the same application (operation,
  receiver and argument vector), and the label of the proof-query stutter
  is `StepLabel::Stutter`. It SHALL charge `request.meter`. `Holds` keeps
  the tuple, `Fails` drops it, and `Undefined` keeps it with FR-315's
  undefined-match definedness failure. `Refused { conjunct, .. }` and
  `MatchIncomplete { conjunct, .. }` refuse `match-conjunct`, at the
  conjunct (its index in `mu_universal()` and its site `hyper <c> match
  <k>`), naming the tuple by its component disjuncts in quantifier order.
- `HyperProductIdentity::of` SHALL evaluate no conjunct and charge nothing
  (FR-315 "Product identity").

### Out of scope

- The body of a `relation` over model executions (`CheckedModelRelation`,
  HP-1): FR-172 notes that its lowering remains.
- Writing these nodes into the clause's checked-package v2 `hyperproperty`
  node (QSpec FR-402) and exposing its occurrence on `CheckedHyperClause`:
  Linear QSL-1071. The emitter lowers each condition as FR-093 lowers a
  Boolean expression, the slots becoming its parameter nodes.
- SMT encoding of `μ_E`: FR-315 encodes HP-2 only.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-487-AC-1 | ADR-023 §1's `NonInterference` checks with `atom_slots()` `[(a, v), (b, v)]` (traces 0 and 1, parameter 0). The condition of its one atom, `v.l @ a = v.l @ b`, is the equality of the field read `l` of slot 0 and the field read `l` of slot 1, both `Int[0, 1]`, with slot count 2 and no node for `@`. `SavesPower` checks with `atom_slots()` `[(s, d), (n, e)]`, and its atom `d.used @ s <= e.used @ n` reads `used` of slot 0 and of slot 1. Checking either clause twice gives equal conditions, equal slot lists and equal `HyperIdentity` values. | Test (TC-1264) |
| FR-487-AC-2 | `NonInterference`'s conjunct 0, `a.step.op = b.step.op`, has `slots()` `[Operation a, Operation b]`, both `String`. Its conjunct 1, `a.step.Vault::step.i = b.step.Vault::step.i`, has two `Option<Int[0, 1]>` slots. In `match { a.step.op = V::Vault::step and b.step.op = stutter }`, conjunct 0 compares its one `String` slot with the literal operation text of `step`, and conjunct 1 compares its one slot with the literal `stutter`. `match { a.step.receiver = b.step.receiver }` over the vault has two `Option<Reference<Vault>>` slots. Over a model whose operations with a receiver are declared by two object types, a conjunct reading `a.step.receiver` refuses `unsupported_construct`/`expression-form` at that read. | Test (TC-1264) |
| FR-487-AC-3 | Over ADR-023 §8's secure vault at instance `v`, `evaluate_hyper_holds` for `NonInterference`'s atom gives `Completed(true)` with components `(0, 0)` and `(1, 0)`, and `Completed(false)` with `(0, 1)` and `(0, 0)`. For FR-176-AC-5's atom `1 mod (1 - v.h @ a) = 1 mod (1 - v.h @ b)` with components `(1, 0)` and `(0, 0)` it gives `Undefined` with cause `division-by-zero`. A meter with a zero budget gives `Incomplete` with `resource_exhausted`/`insufficient-next-charge`. A `slots` vector of length 1 gives `CallFailure::Fault` and charges nothing. Two equal calls give equal evaluations and equal charge logs. | Test (TC-1264) |
| FR-487-AC-4 | For `NonInterference`'s `match`, `CheckedStepMatch::universal` gives `Holds` over `(step(0), step(0))` and over `(stutter, stutter)`, both arguments being absent there. Over `(step(0), step(1))` it gives `Fails`, from conjunct 1. Over `(step(0), stutter)` it gives `Fails` from conjunct 0 and evaluates no node of conjunct 1: the meter's charge log holds conjunct 0's charges only. | Test (TC-1264) |
| FR-487-AC-5 | `NonInterference` with FR-176-AC-4's conjunct `(if present(a.step.Vault::step.i) then 1 mod value(a.step.Vault::step.i) else 0) = 0` added as conjunct 2 gives `Undefined { conjunct: 2, cause: division-by-zero }` over `(step(0), step(0))`. With conjunct 2 instead calling a declared function that exhausts the meter, it gives `Err(MatchIncomplete { conjunct: 2, .. })`. With conjunct 2 instead refused by its evaluation, it gives `Refused { conjunct: 2, .. }`. Over `(step(0), step(1))` each of the three gives `Fails` from conjunct 1 and does not evaluate conjunct 2. | Test (TC-1264) |
| FR-487-AC-6 | EN-1 (FR-176) run with `CheckedHyperLetters` and `CheckedStepMatch` over ADR-023 §8 gives FR-176-AC-1's outcomes: for the leaky vault 10 reachable states and `Violated` with the §8.1 `Lockstep` counterexample, and for the secure vault 8 states and `Holds{Exhaustive}`. FR-176-AC-4's added conjunct gives the `Undefined` counterexample that criterion states. | Test (TC-1264) |
| FR-487-AC-7 | `encode_hyper_query` for `NonInterference` at instance `v` with `InductionStep{depth: 1}` asserts the property at position 0 as `(= s0.a.v.l s0.b.v.l)` and its violation at position 1 over `s1.a.v.l` and `s1.b.v.l`, read from the atom's checked condition with slot 0 in copy `a` and slot 1 in copy `b`. `InductionBase{depth: 1}` keeps exactly FR-315-AC-7's three joint disjuncts, each chosen by `CheckedStepMatch::universal` over the disjuncts' labels. `HyperQueryRequest` has no `StepMatch` or `HyperLetters` member. | Test (TC-1264) |
| FR-487-AC-8 | FR-315-AC-10's request, whose `match` conjunct calls a function that exhausts the meter, refuses `match-conjunct` naming the conjunct's index in `mu_universal()`, its site `hyper <c> match <k>` and the tuple `(step(0), step(0))`. With the conjunct `present(a.step.Vault::step.i) implies value(a.step.Vault::step.i) = 1` placed directly before it, which is false on every tuple whose `a` step is `step(0)`, the refusal names the calling conjunct and the tuple `(step(1), step(1))`: the first tuple in joint-step order on which every conjunct before it holds, since `NonInterference`'s equal-input conjunct fails on `(step(1), step(0))`. | Test (TC-1264) |

## Dependencies

- [FR-172](FR-172-check-hyper-and-relation-clauses-over-model-subjects.md)
  (the checked clause, its indexed atoms, step labels, `match` rules and
  "Body expressions" value-typer rule; Linear QSL-867 moves its typing onto
  the value typer, which this lowering runs),
  [FR-104](FR-104-check-state-clauses.md) (typing and definedness of a state
  predicate), [FR-337](FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md)
  (the temporal condition this lowering mirrors and its site names).
- [FR-107](FR-107-evaluate-state-clauses-at-s6a.md) (the one clause
  evaluator, its charges and its reference comparison),
  [FR-120](FR-120-simulate-a-checked-package-s-state-family.md) (transition
  identities and their argument encoding),
  [FR-128](FR-128-replay-a-model-counterexample.md) (argument decoding).
- [FR-175](FR-175-evaluate-a-body-over-a-tuple-of-behaviours.md),
  [FR-176](FR-176-check-a-universal-hyperproperty-by-self-composition.md),
  [FR-177](FR-177-check-a-forall-exists-safety-hyperproperty-by-witness-sets.md),
  [FR-178](FR-178-check-a-projection-aligned-hyperproperty.md) and
  [FR-183](FR-183-replay-a-hyper-counterexample.md) (the readers of the
  seams), [FR-315](FR-315-encode-the-smt-lib-transition-relation.md) (the
  hyper queries).
- ADR-023 HS-3, HS-4, HS-5, HM-3, HM-4, HV-7, HV-8; QSpec FR-395 ("Step
  labels"), QSpec FR-402 (the `hyperproperty` node QSL-1071 emits).

## References

- Owning ticket: Linear QSL-1072. Consumer: Linear QSL-1010
  (`encode_hyper_query`, FR-315-AC-7 to AC-10), and through it Linear
  IR-974. The interim seam contract on `HyperQueryRequest` was drafted and
  withdrawn (quire-spec-language #1484, SR-17048 FND-001 and FND-002).
