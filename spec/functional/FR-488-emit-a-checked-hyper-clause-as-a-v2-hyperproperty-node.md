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
- `quantifiers`: one `binding(variable, aggregate)` per quantifier in
  source order. The aggregate holds, in order, a `text` literal `forall` or
  `exists`, a `text` literal naming the alias, and an aggregate of the
  variable's resolved fairness constraints, in fairness-set order, empty for
  the empty set. A constraint is written as the aggregate of the bindings
  `kind` (`weak` or `strong`), `granularity` (`whole` or `each`),
  `declaration` (a `reference` to the declaring `model`/`object_type` node
  of the operation) and `name` (the operation's text), with the resolved
  `fairness_kind` and `granularity` as FR-337 resolves them, so an unmarked
  constraint and its written `weak whole` form emit identical bytes
  (ADR-018 FA-6). This inline form replaces FR-402's "references to its
  fairness constraints" for the reason in Q-1.
- `match`: one term per conjunct of `mu_universal()` and `mu_existential()`
  together, in source order, the order of the conjuncts' span starts. Each
  term is the conjunct's lowered condition (§ "Match conjuncts"). The
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

- The emitter SHALL mint, for each atom slot (`t`, `p`) of
  `atom_slots()` (FR-487 "Atom slots"), one `value`/`parameter` node named
  `<p>@<t>` (parameter name, `@`, trace variable name; neither name can
  contain `@`), at level `k`, the slot's index, typed by the type node of
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
  equal to the lock's `profile_selections` row of that role for the
  clause's profile and the six arguments of QSpec FR-370, in order:
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
- The I2 reader SHALL NOT return the wrapper as a temporal clause: it SHALL
  skip a `temporal`/`temporal_clause` node that is the `body` target of a
  `hyperproperty` node when it lists FR-337's clause roots, and a looked-up
  clause root that names the wrapper SHALL return `clause-root-unresolved`.

### Match conjuncts

- Each conjunct is its checked condition lowered as FR-093 lowers a Boolean
  expression. The aggregate member is the term the lowering returns: a
  `reference` to the condition's root `expression` node, or a `literal`
  term for a constant conjunct such as `true` (Q-4).
- The condition's slot `k` (FR-487 "Slots") is bound to a
  `value`/`parameter` node, at level `k`, named by the step-label read it
  stands for, written `<t>.step.op`, `<t>.step.receiver` or
  `<t>.step.<O>.<p>`, `O` being the operation as `<T>::<name>` with `T` the
  declaring object type, typed by the type node of `StepRead::read_type()`,
  which is a `String` for `op` (FR-487 "Lowered form") and an `Option` for a
  receiver or argument. A conjunct reading `a.step.op` and `a.step.op` again
  has one slot.
- The emitter SHALL record each slot node's `expression` occurrence and its
  type's `type` occurrence at `hyper <c> match <k> slot <j>` for a conjunct
  of `μ_U`, or `hyper <c> match exists <k> slot <j>` for `μ_E`, and each
  conjunct root's `expression` occurrence at the conjunct's site
  (`hyper <c> match <k>`, `hyper <c> match exists <k>`, FR-487).
- A label constant lowers to the `String` literal FR-487 gives it and
  `stutter` to the `String` literal `stutter`, so `a.step.op = V::Vault::step`
  carries the operation text of `step`.

### Clauses S4 does not emit

- A hyper clause whose profile is the timed profile (FR-234) SHALL be refused
  `unsupported_construct`/`not-yet-implemented` at the clause, as FR-337
  refuses a timed temporal clause.
- A hyper clause with an alias that binds no object parameter SHALL be refused
  the same way (Q-2).
- A hyper clause with an atom that has a model read whose recorded
  component (FR-487 "Components of reads") differs from the component of the
  slots its object operand is rooted in SHALL be refused the same way. The
  roots of a read's object operand are the slots of its lowered subtree, with
  a `let`-bound name replaced by its initializer's subtree. A package term
  carries no component of its own beside the slot (Q-3), so a read whose
  component is not that of its roots has no encoding. A read of `v.l @ a`,
  and a read `deref(value(r)).l` of a reference `r` that `let r = v.next`
  binds inside the same `@ a`, are encodable; a `let r = (v.next @ a) in
  deref(value(r)).l` inside `@ b` is not.
- Every refusal here is a new site of an existing code and tag; this
  requirement adds no wire-visible refusal.

### Node accounting

The nodes this requirement mints count toward the package's node ceiling
and checking work (NFR-011), the conditions as FR-487 states.

## Acceptance Criteria

The vault, `NonInterference` and `SavesPower` are ADR-023 §1's and §8's, as
in FR-487.

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-488-AC-1 | Compiling `NonInterference` emits a `claim`/`hyperproperty` node of `semantic_type` Boolean whose body is the aggregate of seven bindings `domain`, `subjects`, `parameters`, `quantifiers`, `match`, `align_skip`, `body` in that order: `domain` the text `behaviours`; `subjects` one binding `V` to the `model` node of `Vault`; `parameters` one binding `v` to the same node; `quantifiers` `a` and `b`, each `forall`, `V` and an empty aggregate; `match` two terms in source order; `align_skip` empty; `body` a reference to a `temporal`/`temporal_clause` node. QSL's I2 reader admits the package and returns the clause's `claim` triple and the body wrapper's NodeId. | Test (TC-1266) |
| FR-488-AC-2 | The body wrapper of `NonInterference` is a `quire.op.temporal.clause` application with one `temporal_profile` law equal to the lock's row and the arguments `over` a reference to the `value`/`parameter` node named `v@a` at level 0, clock `model-steps`, activation binding `kind` to `origin` alone, empty captures, empty fairness, and `formula` a reference to a `quire.op.temporal.always` node with a `null` interval whose operand is a `holds` node. The `holds` operand is the equality of the field read `l` of `v@a` (level 0) and of `v@b` (level 1), no term for `@`; both slot nodes have an `expression` occurrence at `hyper 0 slot 0` and `hyper 0 slot 1` and a `type` occurrence of their type node. | Test (TC-1266) |
| FR-488-AC-3 | `SavesPower` emits `subjects` with two bindings `S` and `N` in order of first naming, each to the `model` node of its alias's first object parameter's type, `parameters` `d` and `e` in declaration order, and the slot nodes `d@s` and `e@n`; its `holds` operand reads `used` of `d@s` and of `e@n`. A clause whose alias binds no object parameter is refused `unsupported_construct`/`not-yet-implemented` at the clause and gives no package. | Test (TC-1266) |
| FR-488-AC-4 | `NonInterference`'s `match` aggregate holds, in source order, a reference to the equality of the `parameter` nodes named `a.step.op` and `b.step.op` (level 0 and 1, `String`), and a reference to the equality of the nodes named `a.step.Vault::step.i` and `b.step.Vault::step.i` (`Option<Int[0, 1]>`). In `match { a.step.op = V::Vault::step and b.step.op = stutter }` the second term compares its one slot with the `String` literal `stutter`, and the first with the operation text of `step`. For a clause whose conjuncts are `c0` (universal), `c1` (reads an existential variable) and `c2` (universal), written in that order, the aggregate holds `c0`, `c1`, `c2`, not `μ_U` then `μ_E`. A constant conjunct `true` is a `literal` term. Each conjunct root has an `expression` occurrence at `hyper 0 match <k>` or `hyper 0 match exists <k>`. | Test (TC-1266) |
| FR-488-AC-5 | `NonInterference` with `align skip { V::Vault::mix }` binds `align_skip` to one reference, to the `state`/`operation_anchor` node of `mix`, which equals the anchor node that a state clause naming `mix` references; two listed operations give two references in written order. | Test (TC-1266) |
| FR-488-AC-6 | Each of these gives a different `hyperproperty` NodeId from `NonInterference`: the two quantifiers reversed in kind or in alias; `fair weak V::Vault::step` on `a`; the same constraint on `b` (and different from the one on `a`); an added `match` conjunct; an `align skip` list; a changed body atom; a renamed trace variable. Reformatting and a changed clause name give the same NodeId. Two structurally equal clause declarations share one node and give two `claim` occurrences with ordinals 0 and 1. | Test (TC-1266) |
| FR-488-AC-7 | With `fair weak V::Vault::step` on `a`, `quantifiers` binds `a` to an aggregate whose third member is one aggregate binding `kind` `weak`, `granularity` `each`, `declaration` a reference to `Vault`'s `model` node and `name` `step`; `fair step` and `fair weak whole step` emit identical bytes to the resolved forms they resolve to (FR-123), and `b`'s fairness aggregate is empty. The package is admitted by the I2 reader. | Test (TC-1266) |
| FR-488-AC-8 | After assembly, `CheckedHyperClause::occurrence()` of `NonInterference` equals the triple the I2 reader returns for the clause, and FR-315-AC-12's `HyperProductIdentity::clause()` equals it. A looked-up triple with an unknown NodeId, a non-`hyperproperty` NodeId, role `invariant` or another ordinal returns `hyper-root-unresolved` at the triple. The body wrapper is not in the I2 reader's temporal clause roots, and looking it up as a clause root returns `clause-root-unresolved`. | Test (TC-1266) |
| FR-488-AC-9 | `Linked` (FR-487) with `B` = `(let r = v.next in present(r) and deref(value(r)).l = 1) @ a` emits, every read in the component of slot `v@a`. With `B` = `(let r = (v.next @ a) in present(r) and deref(value(r)).l = 1) @ b` the clause is refused `unsupported_construct`/`not-yet-implemented` at the clause and gives no package. A clause under the timed profile is refused the same way. | Test (TC-1266) |
| FR-488-AC-10 | The package of AC-1 to AC-7 passes QSpec FR-370's reader in the order FR-370 states: the body wrapper's placement, profile, `over` reference and intervals raise no refusal, and no `temporal`/`formula` or `temporal`/`fairness` node is referenced from the `hyperproperty` node. | Test (TC-1266) |

## Open QSpec questions

- **Q-1 (contradiction).** FR-402 binds each quantifier's fairness as "an
  aggregate of references to its fairness constraints". FR-370 "Placement and
  profile fit" allows a reference to a `temporal`/`fairness` node only from a
  `temporal_clause` node's fairness argument and refuses any other with
  `ill_typed`/`operator-ineligible` at the referencing node. A reference
  from a `hyperproperty` quantifier is therefore refused. QSL writes the
  constraint inline (the `kind`, `granularity`, `declaration`, `name`
  aggregate) so the package is admitted today. QSpec should either admit the
  reference from a `hyperproperty` quantifier aggregate or state the inline
  aggregate; QSL then changes this one encoding.
- **Q-2.** FR-402 says "the alias's `model` node" but an alias names a
  model package with many `model` nodes. QSL references the object type of
  the alias's first object parameter. QSpec should state which node.
- **Q-3.** A field read in a package carries no observation or component
  member, so a hyper atom's per-trace read has no carrier but the slot
  parameter node it is rooted in. QSL names the slot `<p>@<t>`, refuses a
  read it cannot root, and asks QSpec to state a slot or component
  encoding (a `hyper_slot` member or a component on the read).
- **Q-4.** FR-402 says `match` is "references to the match's conjuncts"; a
  constant conjunct lowers to a literal, not a node. QSpec should state
  whether it is a literal term or a node.
- **Q-5.** FR-402's `body` is "a reference to the body's `temporal` node"
  and FR-370's formula nodes may be referenced only from a clause or a
  formula. QSL uses a `temporal_clause` wrapper with fixed `over` (the
  first slot), clock, activation, captures and fairness. QSpec may replace it
  with a hyper body form that names the slots directly.

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
