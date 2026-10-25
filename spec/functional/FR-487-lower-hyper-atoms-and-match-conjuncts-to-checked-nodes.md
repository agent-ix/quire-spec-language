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
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-128
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-337
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-175
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-176
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-177
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-178
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-183
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-315
    type: traces_to
---
# FR-487: Lower a hyper clause's holds atoms and match conjuncts to checked nodes

## Description

The S3 hyper check SHALL lower each `holds` atom of a checked hyper clause
over `behaviours`, and each conjunct of its `match` block (`μ_U` and `μ_E`),
to a checked Boolean node. FR-107's one clause evaluator runs that node and
FR-315's `encode_hyper_query` reads it, so no caller supplies an evaluator
seam for these atoms and conjuncts.

Today a body atom is `FormulaNode::Holds { span }` only, and a conjunct is a
`CheckedStepPredicate` that holds a span and its step-label reads
(`qsl-semantics/src/check/hyper_clause.rs`). The product reads both through
the caller seams `HyperLetters` and `StepMatch`
(`qsl-eval/src/hyper/seam.rs`), which only tests implement, by reading
source text. A temporal clause's atoms already have this form:
`TemporalClauseNode::condition` (FR-337) holds each atom's checked condition
over its `over` parameter, and `ClauseLetters` runs that condition through
FR-107's evaluator.

This requirement covers the lowering and its evaluation. It also delivers
FR-172-AC-5's label constants, which `check_match` refuses today, and
narrows FR-172's `match` check for function calls. FR-315's
`encode_hyper_query` (Linear QSL-1010) reads the result. Emitting these
nodes in the clause's `hyperproperty` node is Linear QSL-1071 and is not
part of this requirement.

## Inputs

- A `HyperBehavioursForm` (FR-171) that FR-172 checks to a
  `CheckedHyperClause`: its object parameters, its quantifier prefix, its
  body's `holds` atoms (each `holds(c)` with its condition expression `c`)
  and its `match` conjuncts as FR-172 splits them.
- The unit's type scope, declared functions, admitted models and checking
  limits, against which FR-337's temporal conditions are typed.
- To evaluate: a `CheckedPackageEvaluation` (FR-107) over the package that
  holds the clause, a `Meter`, and either the component states of one joint
  position (for an atom) or the step labels of one joint step (for a
  conjunct).

## Outputs

```rust
// qsl-semantics, on the checked clause (FR-172 "Outputs").
impl CheckedHyperClause {
    /// One entry per atom slot, in slot order.
    pub fn atom_slots(&self) -> &[AtomSlot];
    /// The checked condition of the `holds` atom `atom` and its slot
    /// count; `None` for a node that is not a `holds` atom of the body.
    pub fn condition(&self, atom: FormulaNodeId) -> Option<(CheckedNode<'_>, usize)>;
    /// The component (trace variable index, quantifier order) that each model
    /// read of the atom `atom` reads, keyed by the read's location.
    pub fn component_reads(&self, atom: FormulaNodeId) -> Option<&BTreeMap<Location, usize>>;
}

pub struct AtomSlot {
    pub trace: usize,     // the trace variable's index in quantifier order
    pub parameter: usize, // the object parameter's index in source order
}

impl CheckedStepPredicate {
    /// The conjunct's distinct step-label reads; slot `k` is entry `k`.
    pub fn slots(&self) -> &[StepRead];
    /// The conjunct's checked condition and its slot count.
    pub fn condition(&self) -> (CheckedNode<'_>, usize);
}

// qsl-eval: one atom slot's binding.
pub enum HyperSlot {
    /// The parameter's object in the slot's component.
    Bound(Value),
    /// The instance key names no object of the slot's component's state
    /// (FR-125 Instancing): `key` is its reference and `population` the
    /// population it is drawn from.
    Absent { key: ObjectReference, population: String },
}

// qsl-eval, beside `evaluate_holds` on `CheckedPackageEvaluation` (FR-107).
fn evaluate_hyper_holds(
    &self,
    clause: &CheckedHyperClause,
    atom: FormulaNodeId,
    slots: Vec<HyperSlot>,             // one per `clause.atom_slots()` entry
    components: &[&ObjectEnvironment], // one per trace variable, quantifier order
    meter: &mut Meter,
) -> Result<Evaluation, CallFailure>;

fn evaluate_step_predicate(
    &self,
    clause: &CheckedHyperClause,
    part: MatchPart,           // `Universal` (μ_U) or `Existential` (μ_E)
    conjunct: usize,           // its index in `mu_universal()` or `mu_existential()`
    labels: &[StepLabel<'_>],  // the label vector `StepMatch` receives for `part`
    meter: &mut Meter,
) -> Result<Evaluation, CallFailure>;
```

- On refusal, a typed `CheckRefusal` with a span and no checked clause, when
  an atom or conjunct does not type or its definedness check refuses.

## Behavior

### Atom slots

- The clause's atom slots SHALL be the pairs (trace variable `t`, object
  parameter `p`) whose alias is the same for `t` and `p` (FR-172 "Object
  parameters"), ordered by `t` in quantifier order and then by `p` in source
  order. Slot `k` is the `k`-th pair. Every atom of the clause has the same
  slot list, and the locals an atom binds take the slots after it.
- Each atom slot SHALL be typed `Reference<T>`, `T` being the object type
  that `p` declares (`v: V::T`).

### Lowering a holds atom

- The S3 hyper check SHALL type each atom's condition `c` once with the value
  typer, as FR-337's `check_conditions` types a temporal atom's condition
  (FR-104's invariant state-clause typing, no `self`), over the clause's
  atom slots in place of the one `over` slot. FR-172 "Body expressions" sets
  which expressions refuse and where, and its value-typer refusals
  (FR-172-AC-6) are kept. The typed result SHALL be the atom's checked
  condition.
- Inside an indexed read `e @ t`, the typer SHALL read each name of an object
  parameter `p` as the atom slot (`t`, `p`). The `@ t` node lowers to no node
  of its own: `v.l @ a` lowers to the field read `l` of slot (`a`, `v`).
- Outside every indexed read, the typer SHALL read a bare object-parameter
  name `p` that is not the operand of a member read as the slot (`t`, `p`)
  of the first trace variable `t` of `p`'s alias in quantifier order. Within
  an instance, `p` denotes one key in every trace of its alias (ADR-023
  HS-3), and FR-107 compares references ignoring the observation, so the
  choice of trace does not change the value.
- **Components of reads.** The S3 hyper check SHALL record, for each model
  read of the condition (each field selection and each `deref`), the
  component it reads: the trace variable of the innermost indexed read that
  encloses it. FR-172 admits `(e) @ t` with `@ t` scoping over `e` and
  refuses `ill_typed` a member read that no `@` encloses (FR-172-AC-8), so
  every model read of a checked condition lies inside some `@ t`. The record is static, made
  at S3, and keyed by the read's location, as FR-104 records the observation
  of each read. A reference stored in a `let` or chosen by an `if` inside
  `@ t` is therefore read in component `t` wherever it is used.
- The S3 hyper check SHALL run FR-104's definedness check over each condition
  under the clause's check mode, as `check_conditions` does for a temporal
  atom. The check SHALL treat each component as its own observation: a fact
  established by a read of component `t` discharges only an obligation of a
  read of component `t`.
- The checked conditions SHALL count toward the package's node ceiling and
  checking work, as a temporal clause's conditions do (FR-337, NFR-011).
- The S3 hyper check SHALL locate each condition at the site
  `hyper <c> holds <n>`, an `Origin::TypeDeclaration` name of the form
  FR-337's `temporal_site` gives a temporal condition. Here `c` is the
  clause's index among the unit's hyper clauses over `behaviours`, and `n` is
  the atom's formula-node index. The body arena and its `FormulaNode::Holds`
  nodes are unchanged, and the conditions are keyed by that index.

### Lowering a match conjunct

- **Typing at S3.** The S3 hyper check SHALL type each conjunct once, with
  the value typer, under FR-172's step-label types and `match` rules:
  - `t.step.op` is the operation-or-`stutter` label of `t`'s model. A label
    is compared only by `=` or `!=`, and only with a label of a trace
    variable over the same model, with a label constant of that model or with
    `stutter`. Any other use of a label is refused `ill_typed`/`type-mismatch`
    at it, as FR-172 refuses a label constant used elsewhere. Such uses
    include an ordering, `size`, a call argument, a comparison with a text
    literal, and a comparison with a label of another model.
  - `t.step.receiver` is `Option<Reference<T>>`, `T` being the one object
    type that declares every operation of `t`'s model that has a receiver
    (FR-172, QSpec FR-395 "Step labels").
  - `t.step.O.p` is `Option<T>` for parameter `p: T` of `O` (FR-172).
- **Label constants.** The S3 hyper check SHALL check a qualified name that
  resolves to an operation `O` of the alias bound to `t`, written as an
  operand of `=` or `!=` beside `t.step.op`, as the label constant `O`
  (FR-172 "Match"), with FR-172-AC-5's refusals. Today `check_match` refuses
  every `ExprNode::Name` as a state read, so this requirement delivers
  FR-172-AC-5.
- **Calls.** The S3 hyper check SHALL check a call of a declared function in a
  conjunct as FR-104 checks a call, and SHALL check its arguments under the
  same `match` rules as the rest of the conjunct. Each argument reads step
  labels, literals and other calls only. An argument that reads state SHALL
  be refused `unsupported_construct`/`expression-form` at that read, as
  FR-172 refuses a state read in `match`. FR-315-AC-10's conjunct is such a
  call.
- **Slots.** A conjunct's slots SHALL be its distinct step-label reads, in
  order of first read. `stutter` is not a read: it is a literal.
- **Lowered form.** In the checked condition, a `t.step.op` slot and every
  label operand SHALL be represented as a `String`. A slot carries the
  operation text that `ModelTransition::operation()` carries for the step,
  or the text `stutter`. `stutter` lowers to the `String` literal `stutter`,
  and a label constant `O` lowers to the `String` literal equal to the
  operation text of an `O` step. An operation text is a canonical RFC 8785
  document and `stutter` is not one, so no operation's label equals
  `stutter`. This representation is internal to the checked node. S3's
  label typing above decides which conjuncts check, so a `String` operation
  of the value typer never reaches a label.
- **Receivers over several object types.** If the operations of `t`'s model
  that have a receiver are declared by more than one object type, then the
  S3 hyper check SHALL refuse a conjunct that reads `t.step.receiver`,
  `unsupported_construct`/`expression-form` at the read. Such a read has no
  single type `T`, and QSpec FR-395 gives a receiver no union type. FR-172
  states the same narrowing.
- The S3 hyper check SHALL run FR-104's definedness check over each conjunct
  under the clause's check mode, as it does for an atom.
- The S3 hyper check SHALL locate each conjunct of `μ_U` at the site
  `hyper <c> match <k>` and each conjunct of `μ_E` at
  `hyper <c> match exists <k>`, `k` being the conjunct's index in
  `mu_universal()` or `mu_existential()`.

### Identity

- The lowering SHALL add no member to `HyperIdentity` (FR-172 "Identity",
  ADR-023 HV-7), because the body and `μ` already bind every atom and
  conjunct. Checking the same clause twice SHALL give equal conditions, equal
  slot lists, equal component records and equal identities.

### Evaluating an atom

- `evaluate_hyper_holds` SHALL run the atom's condition through FR-107's
  evaluator (the `Machine` that `evaluate_holds` runs), with slot `k` bound
  from `slots[k]`.
- `evaluate_hyper_holds` SHALL read each model read of the condition in
  `components[r]`, `r` being the component that `component_reads(atom)`
  records for that read. References compare by (universe, object type,
  key), ignoring the component (FR-107).
- For a slot bound `HyperSlot::Absent { key, population }`, the evaluator
  SHALL read the slot as `evaluate_state_predicate_at_absent_key` reads its
  slot 0. It binds `Value::Reference(key)` and marks the slot as
  `Machine::with_absent_parameter` marks it, naming `population`, so a read
  through it gives FR-125's absent-key reading and a `lookup` guard over it
  is false. The other slots, bound or absent, are read independently. The
  machine SHALL accept any number of absent slots.
- If `slots` does not hold one binding per atom slot, `components` does not
  hold one environment per trace variable, or `atom` is not a `holds` atom
  of the clause, then `evaluate_hyper_holds` SHALL return
  `CallFailure::Fault(InternalFault)` and charge nothing.
- The evaluation SHALL charge `meter` exactly as FR-107 charges the same
  condition's nodes, and binding the slots SHALL charge nothing. When the
  meter denies a charge, the result SHALL be `Outcome::Incomplete` with
  `resource_exhausted`/`insufficient-next-charge`.

### Evaluating a match conjunct

- `labels` SHALL be the label vector `StepMatch` receives for `part`: for
  `Universal`, one label per universal trace variable in quantifier order;
  for `Existential`, one label per trace variable of the clause, the
  universal ones first.
- `evaluate_step_predicate` SHALL run the conjunct's condition through FR-107's
  evaluator, with slot `k` bound from the label at the quantifier index of
  the trace variable that read `k` names:
  - `t.step.op`: the step's `ModelTransition::operation()` text, or `stutter`
    for `StepLabel::Stutter`;
  - `t.step.receiver`: none at `StepLabel::Stutter` and at a step with no
    receiver, and otherwise the reference that the step's receiver text names;
  - `t.step.O.p`: present exactly when the step is an `O` step, holding the
    argument at `p`'s parameter index, decoded from FR-120's typed canonical
    form as FR-128's replay decodes it.
- A step label is the step's transition identity (ADR-023 HS-5), so the
  conjunct reads no state and `evaluate_step_predicate` takes no object
  environment and no instance.
- `evaluate_step_predicate` SHALL return `CallFailure::Fault(InternalFault)`
  and charge nothing in each of these cases:
  - `conjunct` is out of range for `part`;
  - `labels` is not the length this section gives for `part`;
  - a label's receiver or argument text does not decode at its declared type.
- The conjunct SHALL charge `meter` exactly as FR-107 charges its nodes, and
  binding its slots SHALL charge nothing.

### The clause seams

`qsl-eval` SHALL implement `HyperLetters` and `StepMatch` over the lowered
nodes as `CheckedHyperLetters` and `CheckedStepMatch`, in its `hyper`
module beside the traits. They are the production seams through which
FR-176's product, FR-177's witness product, FR-178's projected product and
FR-183's replay evaluate the body's atoms and `μ` with FR-107's one
evaluator.

- `HyperLetters::atom` SHALL return `Result<AtomReading, AtomStop>`, the
  error channel of `ModelLetters::atom` (`AtomStop::{Incomplete, Refused,
  Unobservable, Fault}`), in place of `Result<AtomReading, Incomplete>`.
- `CheckedHyperLetters::atom` SHALL observe each component state through its
  own component's `ModelSystem`. It binds slot (`t`, `p`) to `HyperSlot::Bound`
  holding the reference of `p`'s instance key in component `t`, or, when
  that key names no object of component `t`'s state, to `HyperSlot::Absent`
  naming the key and its population, and it evaluates through
  `evaluate_hyper_holds`. Its result SHALL map as `ClauseLetters` maps an
  evaluation: a completed Boolean or an undefined cause to `AtomReading`, an
  exhausted meter to `AtomStop::Incomplete`, a refusal or a non-Boolean
  result to `AtomStop::Refused`, a state that cannot be observed to
  `AtomStop::Unobservable`, and an evaluator fault to `AtomStop::Fault`.
- The hyper products SHALL read `AtomStop::Incomplete` as they read
  `Err(Incomplete)` today. They SHALL record any other `AtomStop` in their
  `Halt`. `qsl-analyze`'s hyper model check SHALL map a recorded stop as its
  temporal check maps a letter stop: `Refused` to
  `ModelCheckRefusal::AtomRefused`, `Unobservable` to
  `ModelCheckRefusal::StateUnobservable` and `Fault` to
  `ModelCheckRefusal::Fault`.
- `CheckedStepMatch::universal` and `existential` SHALL evaluate the
  conjuncts of their part in index order, each once, through
  `evaluate_step_predicate`, sharing the one `meter`, and stop at the first
  conjunct that is not `Completed(true)`. A conjunct after it is neither
  evaluated nor charged, as FR-107 skips the right operand of a decided
  `and`.
- `StepMatch::universal` and `StepMatch::existential` SHALL return
  `Result<MatchReading, MatchIncomplete>` with these shapes, `conjunct` being
  the index in `mu_universal()` for `universal` and in `mu_existential()`
  for `existential`:

  ```rust
  pub enum MatchReading {
      Holds,                                                // every conjunct completed `true`
      Fails,                                                // the first conjunct not `true` completed `false`
      Undefined { conjunct: usize, cause: UndefinedCause }, // the first such conjunct had no value
      Refused { conjunct: usize, outcome: ClauseOutcome },  // the first such conjunct was refused
  }
  pub struct MatchIncomplete { pub conjunct: usize, pub incomplete: Incomplete }
  ```

  For a clause with no conjunct in a part, the reading of that part SHALL be
  `Holds`, with nothing charged.
- The products SHALL read `Refused { .. }` and `Err(MatchIncomplete { .. })`
  as they read `Refused` and `Err(Incomplete)` today: FR-176
  `MatchUndetermined`, and the matching findings of FR-177 and FR-178. This
  requirement adds no conjunct field to those outcomes. The conjunct is
  observable through `StepMatch` and through FR-315's `match-conjunct`
  refusal.

### Reading the lowered nodes in an SMT query

FR-315's `encode_hyper_query` SHALL read these nodes as stated below. No
evaluator seam is added to `HyperQueryRequest`.

- **Atoms.** At joint position `i`, `encode_hyper_query` SHALL read an atom's
  condition through FR-315's expression reader (`Reader::boolean`), with atom
  slot (`t`, `p`) bound to the layout object of `p`'s key in
  `request.instance`. Each model read is read in copy `s<i>.<r>`, `r` being
  the component that `component_reads(atom)` records for it. The atom gives
  its value term and its definedness term, as a temporal atom does.
- **The property of an `always` body.** For a body `always φ`, where `φ` is a
  Boolean combination of `holds` atoms with no temporal operator (FR-315
  "Property"), `encode_hyper_query` SHALL take the property at position `i`
  to be `φ` over the atoms' terms at `i`. Its definedness conditions are
  those of the atoms that `φ`'s short-circuit reads at `i`, as FR-315's
  reader builds a temporal atom's condition under an `and`.
- **The property of any other body.** For any other HP-2 body in the safety
  fragment, `encode_hyper_query` SHALL use the monitor that FR-315's monitor
  tabulation gives a temporal clause with the same formula and profile and an
  `on origin` activation:
  - The automaton is `translate_body(body, profile, limit)` under
    `Monitor::bad_prefix`. It is tabulated as `MonitorTable::of` tabulates
    (breadth-first numbering, initial state 0, letter bit `j` being the
    table's atom `j`), under the request's `max_monitor_atoms` and
    `max_monitor_states`.
  - `MonitorTable` SHALL gain a constructor over a `CheckedFormula` and a
    profile, and `MonitorTable::of` delegates to it for an `on origin`
    clause, so the two tabulations cannot differ.
  - Letter bit `j` at position `i` is atom `j`'s term at `i`.
  - The query has one instance, so its monitor state is `m<i>` (FR-315
    "Copies").
- **`μ_U`.** For each tuple of component disjuncts, in FR-315's joint-step
  order, `encode_hyper_query` SHALL evaluate `μ_U` through
  `CheckedStepMatch::universal`, charging `request.meter`. It passes one
  label per trace variable: the label of an operation disjunct is the
  `ModelTransition` that FR-120 gives the same application (operation,
  receiver and argument vector), and the label of the proof-query stutter
  is `StepLabel::Stutter`. The readings act on the tuple as follows:
  - `Holds` keeps the tuple.
  - `Fails` drops it.
  - `Undefined` keeps it, with FR-315's undefined-match definedness failure.
  - `Refused { conjunct, .. }` and `MatchIncomplete { conjunct, .. }` refuse
    `match-conjunct` at the conjunct (its index in `mu_universal()` and its
    site `hyper <c> match <k>`), naming the tuple by its component disjuncts
    in quantifier order.
- `HyperProductIdentity::of` SHALL evaluate no conjunct and charge nothing
  (FR-315 "Product identity").

### Out of scope

- The body of a `relation` over model executions (`CheckedModelRelation`,
  HP-1). FR-172 notes that its lowering remains.
- Writing these nodes into the clause's checked-package v2 `hyperproperty`
  node (QSpec FR-402) and exposing its occurrence on `CheckedHyperClause`.
  That is Linear QSL-1071. The emitter lowers each condition as
  [FR-093](FR-093-lower-checked-value-expressions-to-fr-322-terms.md)
  lowers a Boolean expression, with the slots becoming its parameter nodes.
- SMT encoding of `μ_E`: FR-315 encodes HP-2 only.

## Acceptance Criteria

The model `G` used below has object type `Node`, with fields
`next: Option<Reference<Node>>`, `p: Option<Int[0, 1]>` and `l: Int[0, 1]`,
population `nodes` with universe `{n1, n2}`, and one operation `link` with
frame `modifies [next]`. `Linked` is `hyper Linked using inf over
behaviours (v: G::Node) forall trace a of G forall trace b of G { always
holds(B) }`, its body atom `B` varying by criterion, read at the instance `n1`.

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-487-AC-1 | ADR-023 §1's `NonInterference` checks with `atom_slots()` `[(a, v), (b, v)]` (traces 0 and 1, parameter 0). The condition of its one atom, `v.l @ a = v.l @ b`, is the equality of the field read `l` of slot 0 and the field read `l` of slot 1, both `Int[0, 1]`, with slot count 2 and no node for `@`. `component_reads` records component 0 for the first read and component 1 for the second. `SavesPower` checks with `atom_slots()` `[(s, d), (n, e)]`, and its atom `d.used @ s <= e.used @ n` reads `used` of slot 0 and of slot 1. Checking either clause twice gives equal conditions, equal slot lists, equal component records and equal `HyperIdentity` values. | Test (TC-1264) |
| FR-487-AC-2 | `NonInterference`'s conjunct 0, `a.step.op = b.step.op`, has `slots()` `[Operation a, Operation b]`, each lowered as a `String`. Its conjunct 1, `a.step.Vault::step.i = b.step.Vault::step.i`, has two `Option<Int[0, 1]>` slots. In `match { a.step.op = V::Vault::step and b.step.op = stutter }`, conjunct 0 compares its one slot with the literal operation text of `step`, and conjunct 1 compares its one slot with the literal `stutter`. `match { a.step.receiver = b.step.receiver }` over the vault has two `Option<Reference<Vault>>` slots. Each refuses `ill_typed`/`type-mismatch` at the label: `match { a.step.op = "x" }`; `match { size(a.step.op) > 0 }`; `SavesPower` with `match { s.step.op = n.step.op }`, a comparison of labels of two models. | Test (TC-1264) |
| FR-487-AC-3 | Over ADR-023 §8's secure vault at instance `v`, `evaluate_hyper_holds` for `NonInterference`'s atom gives `Completed(true)` with components `(0, 0)` and `(1, 0)`, and `Completed(false)` with `(0, 1)` and `(0, 0)`. For FR-176-AC-5's atom `1 mod (1 - v.h @ a) = 1 mod (1 - v.h @ b)` with components `(1, 0)` and `(0, 0)`, it gives `Undefined` with cause `division-by-zero`. A meter with a zero budget gives `Incomplete` with `resource_exhausted`/`insufficient-next-charge`. A `slots` vector of length 1 gives `CallFailure::Fault` and charges nothing. Two equal calls give equal evaluations and equal charge logs. | Test (TC-1264) |
| FR-487-AC-4 | For `NonInterference`'s `match`, `CheckedStepMatch::universal` gives `Holds` over `(step(0), step(0))`, and over `(stutter, stutter)`, where both arguments are absent. It gives `Fails` over `(step(0), step(1))`, with a charge log holding the charges of conjuncts 0 and 1. It gives `Fails` over `(step(0), stutter)`, with a charge log holding conjunct 0's charges only. | Test (TC-1264) |
| FR-487-AC-5 | `NonInterference` with FR-176-AC-4's conjunct `(if present(a.step.Vault::step.i) then 1 mod value(a.step.Vault::step.i) else 0) = 0` added as conjunct 2 gives `Undefined { conjunct: 2, cause: division-by-zero }` over `(step(0), step(0))`. With conjunct 2 instead calling a declared function that exhausts the meter, it gives `Err(MatchIncomplete { conjunct: 2, .. })`. With conjunct 2 instead comparing `quire::value::ieee::sqrt` under `Float64[exact]` of a constant that is not a perfect square with a constant (FR-397-AC-20), it gives `Refused { conjunct: 2, outcome }`, `outcome` being the kernel `Refused` `ieee_not_exact` outcome. Over `(step(0), step(1))`, each of the three gives `Fails`, with a charge log holding no charge of conjunct 2. | Test (TC-1264) |
| FR-487-AC-6 | EN-1 (FR-176), run with `CheckedHyperLetters` and `CheckedStepMatch` over ADR-023 §8, gives FR-176-AC-1's outcomes: for the leaky vault, 10 reachable states and `Violated` with the §8.1 `Lockstep` counterexample; for the secure vault, 8 states and `Holds{Exhaustive}`. FR-176-AC-4's added conjunct gives the `Undefined` counterexample that criterion states. | Test (TC-1264) |
| FR-487-AC-7 | `encode_hyper_query` for `NonInterference` at instance `v` with `InductionStep{depth: 1}` asserts the property at position 0 as `(= s0.a.v.l s0.b.v.l)`, and its violation at position 1 over `s1.a.v.l` and `s1.b.v.l`, both read from the atom's checked condition with slot 0 in copy `a` and slot 1 in copy `b`. `InductionBase{depth: 1}` keeps exactly FR-315-AC-7's three joint disjuncts, each chosen by `CheckedStepMatch::universal` over the disjuncts' labels. `HyperQueryRequest` has no `StepMatch` or `HyperLetters` member. | Test (TC-1264) |
| FR-487-AC-8 | FR-315-AC-10's request, whose `match` conjunct calls a function that exhausts the meter, refuses `match-conjunct`, naming the conjunct's index in `mu_universal()`, its site `hyper <c> match <k>` and the tuple `(step(0), step(0))`. Placing the conjunct `present(a.step.Vault::step.i) implies value(a.step.Vault::step.i) = 1` directly before the calling conjunct makes the refusal name the calling conjunct and the tuple `(step(1), step(1))`. The new conjunct is false on every tuple whose `a` step is `step(0)`, and `NonInterference`'s equal-input conjunct fails on `(step(1), step(0))`, so `(step(1), step(1))` is the first tuple in joint-step order on which every conjunct before the calling one holds. | Test (TC-1264) |
| FR-487-AC-9 | `Linked` with `B` = `(let r = v.next in present(r) and deref(value(r)).l = 1) @ a`: S3 records component 0 for the read of `next` and for the read of `l` through `r`. Its condition gives `Completed(true)` with component `a` at `n1.next = n2`, `n2.l = 1` and component `b` at `n1.next` absent, `n2.l = 0`. The same atom with `@ b` gives `Completed(false)` over those components. With component `b`'s state holding no object `n1`, the atom with `@ a` still gives `Completed(true)`. There, with slot (`b`, `v`) bound `HyperSlot::Absent`, `v.l @ b = 0` gives the absent-key reading that `ClauseLetters` gives the temporal atom `v.l = 0` over a state with no `n1`, and `v.l @ a = 0` reads component `a` as before. | Test (TC-1264) |
| FR-487-AC-10 | `Linked` with `B` = `present(v.p @ a) and value(v.p @ b) = 1` is refused `undefined_expression`/`unproved-presence` at `value(v.p @ b)`: the presence fact is about component `a` only. With `B` = `present(v.p @ a) and value(v.p @ a) = 1`, `Linked` checks. | Test (TC-1264) |
| FR-487-AC-11 | With FR-172-AC-7's conjunct `f(a.step.Vault::step.i)`, for a declared `f(x: Option<Int[0, 1]>): Boolean` whose body is `present(x)`, the conjunct's `slots()` are `[Argument(a, step, i)]`, and its condition is a call of `f` over that slot. `evaluate_step_predicate` gives `Completed(true)` over `(step(1), step(1))`, where `f` receives the present value 1, and `Completed(false)` over `(stutter, stutter)`, where it receives none. The refusals of a receiver read over two object types and of a call argument that reads state are FR-172-AC-7's. | Test (TC-1264) |
| FR-487-AC-12 | `NonInterference` with the body `always (holds(v.l @ a = v.l @ b) and holds(S))`, `S` comparing `quire::value::ieee::sqrt` under `Float64[exact]` of a constant that is not a perfect square with a constant (FR-397-AC-20): `CheckedHyperLetters::atom` for `holds(S)` returns `Err(AtomStop::Refused(outcome))`, `outcome` being the kernel `Refused` `ieee_not_exact` outcome, and EN-1 over the secure vault returns `ModelCheckRefusal::AtomRefused` with that outcome, as `qsl-analyze`'s temporal check returns for a refused temporal atom. `CheckedHyperLetters::atom` for `NonInterference`'s own atom returns `Err(AtomStop::Unobservable(..))` for a component state that its component's system cannot observe. | Test (TC-1264) |

## Dependencies

- [FR-172](FR-172-check-hyper-and-relation-clauses-over-model-subjects.md)
  provides the checked clause, its indexed atoms, its step labels and its
  `match` rules, including the receiver narrowing and the call rule that
  FR-172 now states, and the "Body expressions" value-typer rule. This
  lowering replaces the S3 hyper typing roots: main's `hyper_typing`
  `type_condition` pass, and the QSL-867 `type_body` route if that route
  lands again. #1521 merged that route and #1525 reverted it.
  - On main, `hyper_typing::type_condition` is a separate forward `Ty` pass
    with real label types. It has no `HyperType` markers and builds no
    checked node, so no typed node is kept.
  - The reverted QSL-867 route had one root per object parameter, `Boolean`
    placeholders with `HyperType` markers for step labels, default checking
    limits, a private node counter, and a typed node that is discarded.
  - The lowering replaces either with per-(trace, parameter) slots, real label
    types, kept conditions, the package's node ceiling, and definedness by
    component. It keeps the value-typer refusals of FR-172-AC-6.
  - Sequencing: QSL-1072 is coded on main, or on QSL-867 if QSL-867 lands
    first, and QSL-1071 follows it.
- [FR-104](FR-104-check-state-clauses.md) provides the typing, definedness
  and read observations of a state predicate.
  [FR-337](FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md)
  provides the temporal condition that this lowering mirrors, and its site
  names.
- [FR-107](FR-107-evaluate-state-clauses-at-s6a.md) provides the one clause
  evaluator, its charges and its reference comparison.
  [FR-120](FR-120-simulate-a-checked-package-s-state-family.md) provides
  transition identities and their argument encoding.
  [FR-128](FR-128-replay-a-model-counterexample.md) provides the argument
  decoding, and FR-125 Instancing provides the absent instance key.
- [FR-175](FR-175-evaluate-a-body-over-a-tuple-of-behaviours.md),
  [FR-176](FR-176-check-a-universal-hyperproperty-by-self-composition.md),
  [FR-177](FR-177-check-a-forall-exists-safety-hyperproperty-by-witness-sets.md),
  [FR-178](FR-178-check-a-projection-aligned-hyperproperty.md) and
  [FR-183](FR-183-replay-a-hyper-counterexample.md) read the seams.
  [FR-315](FR-315-encode-the-smt-lib-transition-relation.md) defines the
  hyper queries.
- ADR-023 HS-3, HS-4, HS-5, HM-3, HM-4, HV-7 and HV-8; QSpec FR-395 ("Step
  labels"); QSpec FR-402 (the `hyperproperty` node that QSL-1071 emits).

## References

- Owning ticket: Linear QSL-1072. Consumer: Linear QSL-1010
  (`encode_hyper_query`, FR-315-AC-7 to AC-10), and through it Linear
  IR-974. The interim seam contract on `HyperQueryRequest` was drafted and
  then withdrawn (quire-spec-language #1484, SR-17048 FND-001 and FND-002).
- Spec review of this requirement: SR-17180 to SR-17186.
