---
id: FR-488
title: "Emit a checked hyper clause as a v2 hyperproperty claim node"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-021
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-023
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-172
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-487
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-337
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-315
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-338
    type: references
---
# FR-488: Emit a checked hyper clause as a v2 hyperproperty claim node

## Description

The S4 package emitter (ADR-011 layer 4 `package`, edge E4) SHALL emit each
checked hyper clause over `behaviours` (FR-172) as one
`quire.checked-semantic-graph/v2` node with `node_tag: "claim"` and
`semantic_form: "hyperproperty"`, whose body is the seven-binding aggregate
QSpec FR-402 states, together with the nodes that aggregate references: the
clause's object-parameter and subject model nodes, its `match` conjunct
nodes, its `align skip` operation anchors and its body's formula nodes.
QSL's I2 reader reads the node back, and the checked clause exposes its
`claim` occurrence (§ "Outputs"). This is the compiler side of QSpec FR-402
(ADR-023 QS-7) and the hyper analog of FR-337: QSpec owns the aggregate and
the obligation identity it binds; QSL emits them from its checked clause.

FR-487 lowers each atom's condition and each conjunct to a checked Boolean
node. This requirement writes those conditions into the package.

## Use case

A verification operator compiles ADR-023 §1's `NonInterference`. The
package carries the clause as a `hyperproperty` node, so FR-315's
`HyperProductIdentity` names the clause by a `claim` occurrence of the
package, the SMT backend reads the clause's atoms and `match` conjuncts from
the package, and two clauses that differ in a quantifier order, a fairness
set, a `match` conjunct, an `align skip` list or a body have different
node ids.

## Semantic authority and boundary

QSpec FR-402 owns the `hyperproperty` body's seven bindings and their order,
and the obligation identity. QSpec FR-370 owns the `temporal` node forms,
the `quire.op.temporal.*` identities and the placement rules this
requirement relies on. QSpec states no node form for a `match` conjunct, no
encoding of a hyper body's indexed atoms and no reader refusal for the
`hyperproperty` body (it is a term body of FR-322's grammar). Where this
requirement chooses a form for those, the choice is QSL's and stays within
what FR-322, FR-370 and FR-402 permit; the choices that QSpec may replace
are listed in "Open QSpec questions".

## Inputs

- A `CheckedHyperClause` over `behaviours` (FR-172, FR-487): aliases,
  object parameters, quantifier prefix with each variable's resolved
  fairness set, the `mu_universal()` and `mu_existential()` conjuncts and
  their checked conditions, `align_skip()`, the body `CheckedFormula`, the
  atom slots with their checked conditions and component records, and the
  profile.
- The package's lock evidence (`profile_selections`) and the node keys
  FR-092 mints.
- `CheckedHyperClause::conjuncts() -> Vec<(MatchPart, usize)>`, which this
  requirement adds beside FR-487's accessors: both parts' conjuncts in
  source order (ordered by `CheckedStepPredicate::span()`).

A hyper clause over a finite trace domain (FR-191) and a `relation` over
model executions (HP-1) are outside this requirement and are emitted as
they are today.

## Outputs

- One `claim`/`hyperproperty` node per distinct checked clause, with
  `semantic_type` the package's `scalar_type`/`boolean` node and every
  occurrence of role `claim`. Node ids are content-addressed, so two
  structurally equal clause declarations share one node and differ only by
  occurrence ordinal (FR-088-AC-3). The clause's name enters no key.
- One `claim` occurrence per declaration, located at the
  `Origin::TypeDeclaration` site `hyper <c>`, `c` being the clause's index
  among the unit's hyper clauses over `behaviours` (FR-487's site scheme).
- On `CheckedHyperClause`, once the package is assembled:

  ```rust
  /// The `claim` occurrence [node, "claim", ordinal] (ADR-013 O-07) of the
  /// clause's `hyperproperty` node: the `clause` member of FR-315's
  /// `HyperProductIdentity`.
  pub fn occurrence(&self) -> &OccurrenceKey;
  ```

  The occurrence is assigned by the emitter, not by S3, so a clause checked
  and not assembled has none; the accessor belongs to the clause as the
  assembled package carries it, as a claim's `occurrence()` does
  (`qsl-semantics/src/check/claims.rs`).
- From the I2 reader, for each `claim` occurrence of a
  `claim`/`hyperproperty` node: its identity triple and the NodeId of its
  body wrapper node (the `body` binding's target, § "The body"), or FR-402's
  refusal. A looked-up triple that names a NodeId the package does not hold,
  a node that is not a `claim`/`hyperproperty` node, or an occurrence whose
  role or ordinal differs SHALL return the lookup refusal
  `hyper-root-unresolved` at the triple. It is a QSL lookup result, not a
  package refusal, as `clause-root-unresolved` is (FR-337).

## Behavior

### The node and its aggregate

The emitter SHALL write the node's body as the aggregate of QSpec FR-402
with its seven bindings in that order:

- `domain`: the `text` literal `behaviours`.
- `subjects`: one `binding(alias, reference)` per alias of `aliases()`, in
  order of first naming. The reference is to the alias's subject model node
  (QSL's choice, see Q-2): the `model`/`object_type` node (or
  `model`/`systems_interface` node, whichever the declaration is) of the
  object type of the **first object parameter bound to that alias**, in
  parameter declaration order, minted as FR-337 mints the declaring node of
  a fairness member.
- `parameters`: one `binding(name, reference)` per object parameter in
  declaration order, the reference being to the `model` node of the object
  type `T` that `v: V::T` declares.
- `quantifiers`: one `binding("quantifiers", tuple)`, `tuple` being an
  `aggregate` whose members are all Groups (QSpec FR-322 "Body grammar"; Q-7),
  in this order: for each quantifier in source order, one **quantifier
  Group**, then one **fairness Group** per constraint of the variable's
  resolved fairness set, in fairness-set order (none for the empty set).
  - A quantifier Group is an `aggregate` of four `binding`s with `text`
    literal values, in order: `kind` = `quantifier`, `variable` = the trace
    variable's name, `quantifier` = `forall` or `exists`, and `alias` = the
    alias's name.
  - A fairness Group is an `aggregate` of six `binding`s with Leaf values, in
    order: `kind` = the text `fairness`, `variable` = the text naming the
    variable the constraint belongs to, `fairness_kind` (`weak` or `strong`),
    `granularity` (`whole` or `each`), `declaration` (a `reference` to the
    declaring `model`/`object_type` node) and `name` (the operation's text).
    Its `kind`, `fairness_kind`, `granularity`, `declaration` and `name` are
    the names and values of QSpec FR-370's `fairness` member, resolved as
    FR-337 resolves them, so an unmarked constraint and its written
    `weak whole` form emit identical bytes (ADR-018 FA-6).
  - A fairness Group belongs to the variable it names, and stands after that
    variable's quantifier Group; moving a constraint to another variable
    changes `variable` and its position.
  - The emitter takes `declaration` and `name` from the resolved constraint
    (FR-123, checked at S3 against the quantifier's model): the most derived
    effective operation and its declaring type, as FR-370 "Fairness
    resolution" does. No reader validates them in a `hyperproperty` body
    (Q-1). The emitter SHALL refuse a constraint whose declaring node is a
    `model`/`systems_interface` node `unsupported_construct`/
    `not-yet-implemented` at the constraint, as FR-370-AC-11 refuses such a
    fairness member of a clause (Remaining work: QSL-1094).
  - This flat form replaces FR-402's `binding(variable, aggregate)` per
    quantifier with nested fairness references, which FR-322's body grammar
    cannot express (Q-7), and FR-402's references to fairness constraint
    nodes, for the reason in Q-1.

**Grammar fit.** Every shape the aggregate uses is legal in FR-322 "Body
grammar", and the pinned reader's flat-wire rules (`flat_wire.rs`, `Flat::enter`
and `aggregate_stratum`) place it as follows. The node body is a Body
aggregate, so each of the seven bindings stands as a Member `binding`
(rule: Body aggregate members are Members; `Place::Member` takes a
`binding`), whose value is read as `BindingValue`:
`domain` a Leaf literal; `subjects`, `parameters` and `align_skip` an
`aggregate` whose first non-Leaf member is a `binding` (or none), hence a
Group whose members are `binding(name, Leaf)` or Leaves; `match` an `aggregate`
of Leaves (references, or a literal for a constant conjunct), hence a Group;
`quantifiers` an `aggregate` whose first non-Leaf member is an `aggregate`,
hence a Tuple whose members are Groups, each Group's members being
`binding(name, Leaf)` (`Place::GroupMember` takes a `binding` whose value is
`LeafOnly`); `body` a `binding` of a Leaf reference. The depth chain is
Member binding, Tuple, Group, binding, Leaf, the longest the grammar allows.
An `aggregate` in a `GroupMember` or `LeafOnly` position, or a `binding`
anywhere but a Member or Group member, is `malformed_wire`; this encoding has
none. The `aggregate_stratum` rule reads a binding value by its first
non-Leaf member, so the `quantifiers` Tuple SHALL hold only Groups and a
Group only Leaves and bindings of Leaves.

- `match`: one term per conjunct of `mu_universal()` and `mu_existential()`
  together, in source order. The order comes from
  `CheckedHyperClause::conjuncts()`, which this requirement adds to FR-487's
  accessors: the conjuncts of both parts as `(MatchPart, index)` pairs
  ordered by `CheckedStepPredicate::span()` start. The package keeps no part
  or index, so a package-only reader cannot recover the split between `μ_U`
  and `μ_E`; the part is recomputed from the checked clause, which FR-315's
  `match-conjunct` refusal reads. Each term is the conjunct's lowered
  condition (§ "Match conjuncts"). The
  aggregate is empty with no `match` block.
- `align_skip`: one `reference` per listed operation, in the order written,
  to the `state`/`operation_anchor` node of that operation, minted by the
  same routine that mints a state clause's anchor for the operation, so an
  `align skip` of `V::Vault::mix` and a state clause naming `mix` share one
  anchor node, and an operation FR-342 refuses an anchor for is refused as
  FR-342 refuses it. The aggregate is empty with no list.
- `body`: a `reference` to the body wrapper node (§ "The body").

The emitter SHALL key the node by FR-322's rule over this body, so equal
checked clauses emit equal keys, and reversing two quantifiers, moving a
fairness constraint from one variable to another, renaming the alias a
variable ranges over, or changing the `match` conjuncts, the `align skip`
list or the body changes the key (QSpec FR-402-AC-2). It adds no member for
the instance, which FR-315's `HyperProductIdentity` binds beside the
occurrence, or for the copy-swap symmetry, a function of the clause.

### Atom slots as parameter nodes

- **One level rule.** A slot's level SHALL be its index in the slot list of
  the term that reads it: `k` in `atom_slots()` for an atom's condition,
  `j` in `slots()` for a conjunct. The wrapper binds only atom slot 0
  (through `over`); every other slot of an atom or a conjunct is a free
  parameter of its term, as an `on each` trigger is of its guard. The same
  read therefore mints different nodes where its position differs: `b.step.op`
  is level 1 in a conjunct that reads `a.step.op` first and level 0 in a
  conjunct that reads only `b`. Parameter nodes are keyed by name, level and
  type (FR-322), so equal reads at equal levels share one node across atoms,
  conjuncts and clauses. QSpec states no binder scope for them (Q-3).
- The emitter SHALL mint, for each atom slot (`t`, `p`) of
  `atom_slots()` (FR-487 "Atom slots"), one `value`/`parameter` node named
  `<p>@<t>` (parameter name, `@`, trace variable name; neither name can
  contain `@`), at the level the rule above gives, typed by the type node of
  the slot's checked type `Reference<T>`.
- It SHALL record the node's one `expression` occurrence at the site
  `hyper <c> slot <k>` and a `type` occurrence of its type node there, as
  FR-337 records the `over` parameter node's.
- Every atom of the clause SHALL read the same slot nodes, whatever the atom.

The slot is the component: a read of slot `v@a` is a read in the state of
the component that trace variable `a` names (QSpec states no other carrier,
Q-3).

### The body

- The emitter SHALL write the body of the clause as one
  `temporal`/`temporal_clause` node (the **body wrapper**) whose body is a
  `quire.op.temporal.clause` application, with one `temporal_profile` law
  equal to the lock's `profile_selections` row of role
  `temporal_profile` for the clause's profile and the six arguments of QSpec FR-370, in order:
  1. `over`: a reference to the slot node `<p>@<t>` of slot 0;
  2. `clock`: the `text` literal `model-steps`;
  3. `activation`: the aggregate that binds `kind` to the text `origin`
     alone;
  4. `captures`: the empty aggregate;
  5. `fairness`: the empty aggregate;
  6. `formula`: a reference to the `temporal`/`formula` node of the body's
     root.
  FR-402's body binding is "a reference to the body's `temporal` node";
  this wrapper is that node. The hyper body carries no `over` parameter, no
  clock and no activation of its own; the wrapper fills the argument
  positions FR-370 requires with these fixed values, and the slots are
  where a consumer reads the body's parameters, never `over`.
- The emitter SHALL write the wrapper's formula as FR-337 writes a temporal
  formula: each node of `body()` as a `temporal`/`formula` application with
  its own `quire.op.temporal.*` identity and its operands as references to
  their formula nodes, each interval and unbounded operator as FR-337
  writes it. A `holds` atom's operand is the atom's checked condition
  lowered as FR-093 lowers a Boolean expression, with slot `k` of the
  condition bound to the slot node of atom slot `k`, and the locals the
  atom binds taking the binder levels after the slots. `@ t` lowers to no
  term (FR-487).
- The emitter SHALL record one `claim` occurrence of the wrapper at the site
  `hyper <c> body`, one `expression` occurrence of each formula node at
  `hyper <c> node <i>`, `i` its arena index, and the condition's
  occurrences at `hyper <c> holds <n>` (FR-487). The wrapper's name
  `<p>@<t>` for `over` contains `@`, which no source identifier does, so the
  wrapper equals no temporal clause a unit declares.
- **Consequence.** FR-370 gives a `temporal_clause` node `claim`
  occurrences only, so the wrapper is, to a consumer that does not know it, a
  single-trace temporal claim over slot `v@a` with a free level-1 parameter.
  Mitigation inside QSL: the I2 reader SHALL skip a
  `temporal`/`temporal_clause` node that is the `body` target of a
  `hyperproperty` node when it lists FR-337's clause roots; a looked-up
  clause root that names the wrapper SHALL return `clause-root-unresolved`;
  and every QSL consumer that enumerates or resolves temporal clause roots
  (FR-338's resolution, FR-315's lookups, FR-337's listing) SHALL go through
  that reader and so skip it. A consumer outside QSL (QSpec's readers, CG's
  item enumeration) cannot be bound here and may list it (Q-5).

### Match conjuncts

- Each conjunct is its checked condition lowered as FR-093 lowers a Boolean
  expression. The aggregate member is the term the lowering returns: a
  `reference` to the condition's root `expression` node, or a `literal`
  term for a constant conjunct such as `true` (Q-4).
- The condition's slot `j` (FR-487 "Slots") is bound to a
  `value`/`parameter` node, at level `j` (the level rule above), named by the step-label read it
  stands for, written `<t>.step.op`, `<t>.step.receiver` or
  `<t>.step.<O>.<p>`, `O` being the operation as `<T>::<name>` with `T` the
  declaring object type, typed by the type node of `StepRead::read_type()`,
  which is a `String` for `op` (FR-487 "Lowered form") and an `Option` for a
  receiver or argument. A conjunct reading `a.step.op` twice has one slot.
- The emitter SHALL record each slot node's `expression` occurrence and its
  type's `type` occurrence at `hyper <c> match <k> slot <j>` for a conjunct
  of `μ_U`, or `hyper <c> match exists <k> slot <j>` for `μ_E`, and each
  conjunct root's `expression` occurrence at the conjunct's site
  (`hyper <c> match <k>`, `hyper <c> match exists <k>`, FR-487).
- A label constant lowers to the `String` literal FR-487 gives it and
  `stutter` to the `String` literal `stutter`, so `a.step.op = V::Vault::step`
  carries the operation text of `step`.

### Clauses S4 does not emit

A clause below is refused `unsupported_construct`/`not-yet-implemented`
(a new site of the existing cause) and is not compiled into a package, as
FR-337 refuses a clause it does not emit. The checks run per clause in unit
order, and a clause stops at its first refusal, in this order:

1. a clause under the timed profile (FR-234), at the clause;
2. a clause with an alias that binds no object parameter, at the clause: the
   subject node of such an alias is undefined (Q-2);
3. a clause whose quantifier has a non-empty fairness set under a profile
   that QSpec FR-370's profile fit gives an empty fairness argument (the
   three bounded profiles), at the constraint, when S3 admitted it;
4. a clause with a constraint whose declaring node is a `systems_interface`
   (§ "The node and its aggregate"), at the constraint;
5. a clause with a model read that has no encodable component, at the read
   (the first in atom arena order, then source order).

A read's **roots** are the slots in its lowered object operand's subtree,
with a `let`-bound name replaced by its initializer's subtree. The read is
encodable exactly when its roots are non-empty and all have the component
that `component_reads(atom)` records for the read. A read with no root (the
operand is a call result or a constant) and a read rooted in slots of two
components are therefore refused, as is a read whose recorded component
differs from its roots'. A read of `v.l @ a`, and a read `deref(value(r)).l`
of a reference `r` that `let r = v.next` binds inside the same `@ a`, are
encodable; `let r = (v.next @ a) in deref(value(r)).l` inside `@ b` is not.

A package term carries no component of its own beside the slot (Q-3), so a
read outside these cases has no encoding. Remaining work: QSL-1094 lifts
refusals 1, 4 and 5 and, with QSpec's answer to Q-2, refusal 2. No code or
tag is added.

### Profiles

A clause under any profile FR-172 admits, other than the timed profile, is
emitted with that profile's `temporal_profile` law. Its intervals are
written as FR-337 writes them (integer `[a,b]` under a bounded profile; `null`,
`[a,*]` and `[a,b]` under infinite-trace), and `until`, `release` and the
other binary operators as their own identities.

### Node accounting

The nodes this requirement mints count toward the package's node ceiling
and checking work (NFR-011), the conditions as FR-487 states.

## Acceptance Criteria

The vault, `NonInterference` and `SavesPower` are ADR-023 §1's and §8's, as
in FR-487. "Admitted" means QSL's I2 admission of the emitted package
(`qsl-package`), which runs QSpec FR-370's reader.

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-488-AC-1 | Compiling `NonInterference` emits one `claim`/`hyperproperty` node of Boolean `semantic_type` whose body is an aggregate of exactly the seven bindings `domain`, `subjects`, `parameters`, `quantifiers`, `match`, `align_skip`, `body`, in that order. | Test (TC-1266) |
| FR-488-AC-2 | In AC-1's node, `domain` is the text `behaviours`; `subjects` is one binding `V` to the `model` node of `Vault`; `parameters` is one binding `v` to the same node; `quantifiers` is a Tuple of exactly two quantifier Groups, `a` then `b`, each binding `kind` `quantifier`, `variable`, `quantifier` `forall` and `alias` `V`, and no fairness Group; `align_skip` is empty; `body` references a `temporal`/`temporal_clause` node. | Test (TC-1266) |
| FR-488-AC-3 | The I2 reader admits AC-1's package and returns the clause's `claim` triple and the body wrapper's NodeId. | Test (TC-1266) |
| FR-488-AC-4 | The body wrapper of `NonInterference` is a `quire.op.temporal.clause` application with one `temporal_profile` law equal to the lock's row and the arguments `over` a reference to the `value`/`parameter` node `v@a` at level 0, clock `model-steps`, activation binding `kind` to `origin` alone, empty captures, empty fairness, and `formula` a reference to a `quire.op.temporal.always` node with a `null` interval whose operand is a `holds` node. | Test (TC-1266) |
| FR-488-AC-5 | The `holds` operand of AC-4 is the equality of the field read `l` of `v@a` (level 0) and of `v@b` (level 1), with no term for `@`; the slot nodes have `expression` occurrences at `hyper 0 slot 0` and `hyper 0 slot 1` and `type` occurrences of their type node. | Test (TC-1266) |
| FR-488-AC-6 | `SavesPower` emits `subjects` with two bindings `S` and `N` in order of first naming, each to the `model` node of its alias's first object parameter's type, `parameters` `d` and `e` in declaration order, slot nodes `d@s` and `e@n`, and a `holds` operand reading `used` of `d@s` and of `e@n`. | Test (TC-1266) |
| FR-488-AC-7 | `NonInterference`'s `match` aggregate holds, in source order, a reference to the equality of the `parameter` nodes `a.step.op` and `b.step.op` (levels 0 and 1, `String`), then a reference to the equality of `a.step.Vault::step.i` and `b.step.Vault::step.i` (`Option<Int[0, 1]>`), with each conjunct root's `expression` occurrence at `hyper 0 match <k>`. | Test (TC-1266) |
| FR-488-AC-8 | In `match { a.step.op = V::Vault::step and b.step.op = stutter }` the first term compares its one slot with the `String` literal of `step`'s operation text and the second compares its one slot with the `String` literal `stutter`. A constant conjunct `true` is a `literal` term. A conjunct that reads only `b.step.op` has that slot at level 0, and `b.step.op` in `a.step.op = b.step.op` is a different node at level 1. | Test (TC-1266) |
| FR-488-AC-9 | For a clause whose conjuncts `c0` (universal), `c1` (reads an existential variable) and `c2` (universal) are written in that order, `match` holds `c0`, `c1`, `c2`, as `CheckedHyperClause::conjuncts()` returns them, with `c1` at `hyper 0 match exists 0`. | Test (TC-1266) |
| FR-488-AC-10 | `NonInterference` with `align skip { V::Vault::mix }` binds `align_skip` to one reference, to the `state`/`operation_anchor` node of `mix`, equal to the anchor node a state clause naming `mix` references; two listed operations give two references in written order. | Test (TC-1266) |
| FR-488-AC-11 | Each of these gives a `hyperproperty` NodeId different from `NonInterference`'s: the two quantifiers' aliases changed; one quantifier `exists`; the quantifier order reversed; `fair weak V::Vault::step` on `a`; the same constraint on `b` (a fairness Group naming `b`, after `b`'s quantifier Group, and different from the one on `a`); an added `match` conjunct; an `align skip` list; a changed body atom. | Test (TC-1266) |
| FR-488-AC-12 | Reformatting and a changed clause name give `NonInterference` the same NodeId; two structurally equal clause declarations share one node and have two `claim` occurrences with ordinals 0 and 1. A renamed trace variable gives a different NodeId (Q-6). | Test (TC-1266) |
| FR-488-AC-13 | With `fair weak V::Vault::step` on `a`, the `quantifiers` Tuple holds the Groups for `a`, a fairness Group, and `b`, in that order; the fairness Group binds `kind` `fairness`, `variable` `a`, `fairness_kind` `weak`, `granularity` `each`, `declaration` a reference to `Vault`'s `model` node and `name` `step`. `fair step` and `fair weak whole step`, in the resolved forms, emit identical bytes and the package is admitted. A constraint whose declaring node is a `systems_interface` is refused `unsupported_construct`/`not-yet-implemented` at the constraint. | Test (TC-1266) |
| FR-488-AC-22 | Every emitted `hyperproperty` body of AC-1 to AC-13 passes strict wire validation of the flat grammar (QSpec FR-322 "Body grammar"): no term is refused `malformed_wire`, and the `quantifiers` binding is a Tuple of Groups whose members are literals, references and bindings of Leaves. | Test (TC-1266) |
| FR-488-AC-14 | After assembly, `CheckedHyperClause::occurrence()` of `NonInterference` equals the triple the I2 reader returns and FR-315-AC-12's `HyperProductIdentity::clause()`. A looked-up triple with an unknown NodeId, a non-`hyperproperty` NodeId, role `invariant` or another ordinal returns `hyper-root-unresolved` at the triple. | Test (TC-1266) |
| FR-488-AC-15 | The body wrapper is not among the I2 reader's temporal clause roots, looking it up as a clause root returns `clause-root-unresolved`, and FR-338's resolution of the wrapper's triple does the same. | Test (TC-1266) |
| FR-488-AC-16 | `Linked` with `B` = `(let r = v.next in present(r) and deref(value(r)).l = 1) @ a` emits. | Test (TC-1266) |
| FR-488-AC-17 | `Linked` with `B` = `(let r = (v.next @ a) in present(r) and deref(value(r)).l = 1) @ b` is refused `unsupported_construct`/`not-yet-implemented` at the read of `l`, and so is an atom whose read has no slot root (the object operand a call result) and one rooted in slots of two components. No package is produced. | Test (TC-1266) |
| FR-488-AC-18 | A clause under the timed profile is refused `unsupported_construct`/`not-yet-implemented` at the clause. A timed clause with an alias that binds no object parameter is refused at the clause, as the timed profile is checked first; the alias case alone is refused at the clause too. A clause with two refusals reports the first in the order of "Clauses S4 does not emit". | Test (TC-1266) |
| FR-488-AC-19 | An event-position false-extension clause with body `always[0,3] holds(...)` emits a wrapper whose law is that profile's row and an `always` node with the interval `{lower: "0", upper: "3"}`; an infinite-trace `v.l @ a = 0 until v.l @ b = 1` clause emits `quire.op.temporal.until` with a `null` interval and `eventually[2,*]` emits `{lower: "2", upper: null}`. All are admitted. | Test (TC-1266) |
| FR-488-AC-20 | A bounded-profile clause with `fair weak V::Vault::step` on `a`, which S3 admits, is refused `unsupported_construct`/`not-yet-implemented` at the constraint. | Test (TC-1266) |
| FR-488-AC-21 | The nodes this requirement mints count toward the package node ceiling: with a ceiling one node below `NonInterference`'s emission, compiling is refused with the ceiling's existing refusal, and at the ceiling it compiles. | Test (TC-1266) |

## Open QSpec questions

- **Q-1 (contradiction).** FR-402 binds each quantifier's fairness as "an
  aggregate of references to its fairness constraints". FR-370 "Placement and
  profile fit" allows a reference to a `temporal`/`fairness` node only from a
  `temporal_clause` node's fairness argument and refuses any other with
  `ill_typed`/`operator-ineligible` at the referencing node. A reference
  from a `hyperproperty` quantifier is therefore refused. QSL writes the
  constraint inline (a fairness Group of `kind`, `variable`,
  `fairness_kind`, `granularity`, `declaration`, `name`) so the package is admitted
  today. Nothing in the package validates `declaration` and `name`: FR-370's
  fairness resolution runs on a clause's fairness argument only, and QSpec
  states no reader refusal for a `hyperproperty` body. QSpec should either admit the
  reference from a `hyperproperty` quantifier aggregate or state the inline
  aggregate; QSL then changes this one encoding.
- **Q-2.** FR-402 says "the alias's `model` node" but an alias names a
  model package with many `model` nodes. QSL references the object type of
  the alias's first object parameter. QSpec should state which node.
- **Q-3.** A field read in a package carries no observation or component
  member, so a hyper atom's per-trace read has no carrier but the slot
  parameter node it is rooted in. QSL names the slot `<p>@<t>`, refuses a
  read it cannot root, and asks QSpec to state a slot or component
  encoding (a `hyper_slot` member or a component on the read). QSpec also
  states no binder scope for an atom slot beyond slot 0 or for a `match`
  conjunct's slots, so QSL's one level rule (the slot's index in its term's
  slot list) leaves them free parameters, and the same read can have a node
  at two levels; QSpec should state the scope.
- **Q-4.** FR-402 says `match` is "references to the match's conjuncts"; a
  constant conjunct lowers to a literal, not a node. QSpec should state
  whether it is a literal term or a node.
- **Q-5.** FR-402's `body` is "a reference to the body's `temporal` node"
  and FR-370's formula nodes may be referenced only from a clause or a
  formula. QSL uses a `temporal_clause` wrapper with fixed `over` (the
  first slot), clock, activation, captures and fairness. FR-370 gives such a
  node `claim` occurrences only, so a consumer outside QSL that lists
  temporal claims sees a spurious single-trace claim with a free level-1
  parameter; QSL's own consumers skip it (§ "The body"). QSpec should give
  the hyper body a non-claim role, or a hyper body form that names the slots
  directly.
- **Q-6.** FR-402-AC-3 says clauses that differ only in source spelling share
  one identity. QSL's trace-variable names are bound in `quantifiers` and
  appear in slot names (`<p>@<t>`), so renaming a trace variable changes the
  NodeId (FR-488-AC-12). QSpec should say whether bound-variable renaming is
  spelling.

- **Q-7.** FR-402's `quantifiers` shape, `binding(variable, aggregate)` with
  the `forall`/`exists` literal, the alias literal and a nested aggregate of
  fairness constraints, is not expressible in FR-322 "Body grammar": a
  `binding` in a Group has a Leaf value only, and a constraint of several
  members needs a further aggregate, a stratum below the Group. The pinned
  reader refuses it `malformed_wire`. QSL writes a Tuple of Groups (one
  quantifier Group per quantifier, one fairness Group per constraint, naming
  its variable) and asks QSpec to state a legal shape; QSL then changes this
  one encoding. QSL-1093 owns the QSpec change.

## Dependencies

- ADR-011 §6.1 layer 4 `package` and §2.1 E4; ADR-023 QS-7, HV-7.
- [FR-172](FR-172-check-hyper-and-relation-clauses-over-model-subjects.md)
  (the checked clause), [FR-487](FR-487-lower-hyper-atoms-and-match-conjuncts-to-checked-nodes.md)
  (the atoms, slots, conjuncts, components and sites),
  [FR-337](FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md)
  (the formula emission this reuses),
  [FR-093](FR-093-lower-checked-value-expressions-to-fr-322-terms.md),
  FR-092, FR-342 (the operation anchor).
- [FR-315](FR-315-encode-the-smt-lib-transition-relation.md) ("Product
  identity", the consumer of the occurrence) and
  [FR-338](FR-338-check-an-en-1-closure-certificate.md).
- QSpec FR-402, FR-370, FR-322.

## References

- QSpec FR-402 (hyperproperty claim body and obligation identity), FR-370
  (temporal clause body): Linear STD-136 and STD-99.
- Owning ticket: Linear QSL-1071.
