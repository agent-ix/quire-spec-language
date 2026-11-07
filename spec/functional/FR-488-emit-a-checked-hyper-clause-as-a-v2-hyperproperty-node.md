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
- The package's clause profile selections (`profile_selections`) and the node keys
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
  order of first naming. The reference is to the `model`/`model_root` node
  of the domain package the alias selects (QSpec FR-402), the node QSpec
  FR-322 derives once per selected package identity, whatever object types
  the alias's parameters name.
- `parameters`: one `binding(name, reference)` per object parameter in
  declaration order, the reference being to the `model` node of the object
  type `T` that `v: V::T` declares.
- `quantifiers`: one `binding("quantifiers", tuple)`, `tuple` being an
  `aggregate` of one **quantifier Group** per quantifier, in source order
  (QSpec FR-402; a Tuple of QSpec FR-322 "Body grammar").
  - A quantifier Group is an `aggregate` of three `binding`s with `text`
    literal values, in order: `variable` = the trace variable's name,
    `quantifier` = `forall` or `exists`, and `alias` = the alias's name;
    then one `reference` per constraint of the variable's resolved fairness
    set, in fairness-set order (none for the empty set).
  - Each reference targets the `temporal`/`fairness` node whose body root
    is the constraint's `quire.op.temporal.fair` application, minted by the
    routine FR-337 uses for a clause's fairness argument, with QSpec
    FR-370's `fairness` member resolved as FR-337 resolves it, so an
    unmarked constraint and its written `weak whole` form emit one node
    (ADR-018 FA-6), and a constraint shared with a temporal clause shares
    that clause's fairness node.
  - The member's `operation` target takes `declaration` and `name` from the
    resolved constraint (FR-123, checked at S3 against the quantifier's
    model): the most derived effective operation and its declaring type, as
    QSpec FR-370 "Fairness resolution" does, which the QSpec reader re-checks
    for a quantifier's references (QSpec FR-402 "Reader order"). The emitter
    SHALL refuse a constraint whose declaring node is a
    `model`/`systems_interface` node `unsupported_construct`/
    `not-yet-implemented` at the constraint, as FR-370-AC-11 refuses such a
    fairness member of a clause (Remaining work: QSL-1094).
  - A constraint belongs to the Group of its variable; moving it to another
    variable moves its reference to that variable's Group.

**Grammar fit.** Every shape the aggregate uses is legal in FR-322 "Body
grammar", and the QSpec reader's flat-wire rules (`flat_wire.rs`, `Flat::enter`
and `aggregate_stratum`) place it as follows. The node body is a Body
aggregate, so each of the seven bindings stands as a Member `binding`
(rule: Body aggregate members are Members; `Place::Member` takes a
`binding`), whose value is read as `BindingValue`:
`domain` a Leaf literal; `subjects`, `parameters` and `align_skip` an
`aggregate` whose first non-Leaf member is a `binding` (or none), hence a
Group whose members are `binding(name, Leaf)` or Leaves; `match` an `aggregate`
of Leaves (references; a constant conjunct is a reference to its
`value`/`literal` node), hence a Group;
`quantifiers` an `aggregate` whose first non-Leaf member is an `aggregate`,
hence a Tuple whose members are Groups, each Group's members being
`binding(name, Leaf)` (`Place::GroupMember` takes a `binding` whose value is
`LeafOnly`) and then Leaf `reference`s; `body` a `binding` of a Leaf
reference. The depth chain is
Member binding, Tuple, Group, binding, Leaf, the longest the grammar allows.
An `aggregate` in a `GroupMember` or `LeafOnly` position, or a `binding`
anywhere but a Member or Group member, is `malformed_wire`; this encoding has
none. The `aggregate_stratum` rule reads a binding value by its first
non-Leaf member, so the `quantifiers` Tuple SHALL hold only Groups and a
Group only Leaves and bindings of Leaves.

**Structural stage.** After the flat wire, the QSpec reader's structural
stage (`structural.rs`, `validate_structural_nodes`) checks per node kind. For
the kinds this requirement emits:

- `value`/`parameter` (`parameter_defect`): the `semantic_type` is a binder
  type, a node of tag `scalar_type`, `composite_type` or `bounded_domain` (a
  `Reference<T>`, `Option<…>` or `String` slot type qualifies) or a record or
  hybrid model type; the body is the aggregate of `binding("name", text
  literal)` and `binding("level", integer literal)` with the name an ASCII
  identifier (§ "Slot node names") and the level a non-negative integer
  string; `dependencies` is empty; every occurrence has role `expression`.
- `state`/`operation_anchor`: `typed_anchor_body`, written by the existing
  anchor routine.
- `claim`/`hyperproperty`, `temporal`/`temporal_clause`, `temporal`/`formula`
  and `expression` nodes have no structural form; an application node's
  `dependencies` equal the references of its body (`application_dependencies`),
  as the existing emitters write them. The `hyperproperty` body is an
  aggregate and has no application join; its `dependencies` are the nodes it
  references (QSpec FR-402).
- The `model` nodes the body references are the existing declaration nodes
  of the parameters' object types and the `model`/`model_root` nodes of the
  subjects' packages.

No other structural rule applies to a node of these kinds, so the slot name is
the one rule this requirement's new nodes can break there.

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
  read therefore mints different nodes where its position differs: the node
  `op_sb` (the `b.step.op` read) is level 1 in a conjunct that reads
  `a.step.op` first and level 0 in a conjunct that reads only `b`. Parameter nodes are keyed by name, level and
  type (FR-322), so equal reads at equal levels share one node across atoms,
  conjuncts and clauses. QSpec states no binder scope for them (Q-3).
- The emitter SHALL mint, for each atom slot (`t`, `p`) of
  `atom_slots()` (FR-487 "Atom slots"), one `value`/`parameter` node named
  `slot_s<esc(p)>_s<esc(t)>` (§ "Slot node names"), at the level the rule above gives, typed by the type node of
  the slot's checked type `Reference<T>`.
- It SHALL record the node's one `expression` occurrence at the site
  `hyper <c> slot <k>` and a `type` occurrence of its type node there, as
  FR-337 records the `over` parameter node's.
- Every atom of the clause SHALL read the same slot nodes, whatever the atom.

The slot is the component: a read of slot `slot_sv_sa` (`v@a`) is a read in the state of
the component that trace variable `a` names (QSpec states no other carrier,
Q-3).

### Slot node names

The QSpec reader requires a `value`/`parameter` node's `name` to be an ASCII
identifier, `[A-Za-z_][A-Za-z0-9_]*` (`typed_parameter_binding` with
`is_identifier`, structural stage), so a name cannot carry `@`, `.` or `:`.
The emitter SHALL write every slot node name by QSpec FR-402's "Slot
parameters" encoding, which a consumer decodes by the same rule. It is
restated here for the emitter and is deterministic and injective:

- `esc(x)` maps each byte of the UTF-8 text `x` as follows: an ASCII letter
  or digit is itself; `_` is `__`; any other byte is `_x` followed by two
  uppercase hexadecimal digits.
- A name is a tag, then one `_s` and `esc(field)` for each field, in order:
  - atom slot (`p`, `t`): `slot_s<esc(p)>_s<esc(t)>` (object parameter name,
    trace variable name), e.g. `slot_sv_sa` for `v@a`;
  - conjunct `op` read of `t`: `op_s<esc(t)>`, e.g. `op_sa`;
  - conjunct `receiver` read of `t`: `rcv_s<esc(t)>`;
  - conjunct argument read of `t`, operation `O` = `T::name`, parameter `p`:
    `arg_s<esc(t)>_s<esc(T)>_s<esc(name)>_s<esc(p)>`, e.g.
    `arg_sa_sVault_sstep_si`.
- **Injective.** The tags `slot`, `op`, `rcv` and `arg` contain no `_` and
  each name begins with a tag followed by `_s`, so the tag, and with it the
  number of fields, is determined by the name. Within the remainder every `_`
  starts exactly one of the codes `__` (a literal `_`), `_x` plus two hex
  digits (one byte) or `_s` (a field separator), and no code is a prefix of
  another, so reading left to right splits the name into its fields uniquely
  and each field decodes uniquely. Two distinct (tag, fields) therefore give
  distinct names, whatever underscores or non-ASCII characters the source
  names hold, and a name begins with a letter, as an identifier must.
- The name is unique per (tag, fields); the node's identity still comes from
  its (name, level, type) key (FR-322), and the level rule above is
  unchanged.

### The body

- The emitter SHALL write the body of the clause as one
  `temporal`/`temporal_clause` node (the **body wrapper**) whose body is a
  `quire.op.temporal.clause` application, with one `temporal_profile` law
  equal to the package's `profile_selections` row of role
  `temporal_profile` for the clause's profile and the six arguments of QSpec FR-370, in order:
  1. `over`: a reference to the slot node `<p>@<t>` of slot 0;
  2. `clock`: the `text` literal `model-steps`;
  3. `activation`: the aggregate that binds `kind` to the text `origin`
     alone;
  4. `captures`: the empty aggregate;
  5. `fairness`: the empty aggregate;
  6. `formula`: a reference to the `temporal`/`formula` node of the body's
     root.
  This wrapper is QSpec FR-402's "Body wrapper", the target of its `body`
  binding. The hyper body carries no `over` parameter, no
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
  occurrences at `hyper <c> holds <n>` (FR-487). The wrapper's `over`
  parameter is named `slot_s<esc(p)>_s<esc(t)>`, a name a unit's temporal
  clause could in principle also declare; a standalone clause that equalled
  the wrapper in every node would share it.
- **Consequence.** FR-370 gives a `temporal_clause` node `claim`
  occurrences only, and QSpec FR-402 "Body wrapper" makes the wrapper's
  occurrences and name rows the hyper clause's, so every consumer, inside
  QSL or not, SHALL exclude it from the single-trace temporal claims. Inside
  QSL: the I2 reader SHALL skip a
  `temporal`/`temporal_clause` node that is the `body` target of a
  `hyperproperty` node when it lists FR-337's clause roots; a looked-up
  clause root that names the wrapper SHALL return `clause-root-unresolved`;
  and every QSL consumer that enumerates or resolves temporal clause roots
  (FR-338's resolution, FR-315's lookups, FR-337's listing) SHALL go through
  that reader and so skip it. A consumer outside QSL (QSpec's readers, CG's
  item enumeration) is bound by QSpec FR-402 to skip it too.

### Match conjuncts

- Each conjunct is its checked condition lowered as FR-093 lowers a Boolean
  expression. The aggregate member is the term the lowering returns: a
  `reference` to the condition's root `expression` node; a constant conjunct
  such as `true` lowers to a boolean `literal` `value` node and the
  term is a `reference` to it, as QSpec FR-402 states.
- The condition's slot `j` (FR-487 "Slots") is bound to a
  `value`/`parameter` node, at level `j` (the level rule above), named by the
  step-label read it stands for as § "Slot node names" gives (`op_s<t>`,
  `rcv_s<t>` or `arg_s<t>_s<T>_s<name>_s<p>`, each field escaped), typed by the type node of `StepRead::read_type()`,
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
2. a clause with an alias that binds no object parameter, at the clause:
   QSpec FR-402 gives such an alias its package's `model_root` as subject,
   and the refusal stays only until the emitter writes the `model_root`
   subject this requirement now states in place of the earlier subject, the
   object type of the alias's first object parameter, which such an alias
   lacks (Remaining work: QSL-1094);
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
refusals 1, 2, 4 and 5. No code or tag is added.

### Profiles

A clause under any profile FR-172 admits, other than the timed profile, is
emitted with that profile's `temporal_profile` law. Its intervals are
written as FR-337 writes them (integer `[a,b]` under a bounded profile; `null`,
`[a,*]` and `[a,b]` under infinite-trace), and `until`, `release` and the
other binary operators as their own identities.

### Node accounting

The nodes this requirement mints count toward the package's node ceiling
and checking work (NFR-011), the conditions as FR-487 states.

### Clause names and clock bindings

A hyper clause is named and clocked by its body wrapper, the node a consumer
resolves as the clause's claim, not by the `hyperproperty` node, so the rows
of QSpec FR-322-AC-342 to FR-322-AC-370 key on the wrapper.

- The emitter SHALL write one `temporal_clause_names` row
  `{declaration, occurrence, name}` for every hyper clause, whatever its
  profile: `declaration` is the wrapper's emitted NodeId, `occurrence` is the
  ordinal of the clause's assembled `claim` occurrence of the wrapper (the
  occurrence of AC-14) and `name` is the declared clause name. Two
  structurally equal hyper clauses share a wrapper and carry two rows at their
  own ordinals; the name enters no key (AC-12).
- The emitter SHALL write one `temporal_bindings` row
  `{declaration, occurrence, clock_name, clock}` at the same key for a hyper
  clause whose profile is clock-configured. `clock_name` is the text of the
  wrapper's clock argument and `clock` is the declaration's own
  `ClockConfiguration`; the emitter refuses a missing, unused or
  profile-mismatched clock selection as FR-337 states, and a clause under an
  unclocked profile writes a name row and no binding.
- These rows share the arrays, the (NodeId, occurrence) order and the
  omission rule of FR-337 "Clause names and clock bindings per occurrence"
  with the temporal clauses of the same unit.

## Acceptance Criteria

The vault, `NonInterference` and `SavesPower` are ADR-023 §1's and §8's, as
in FR-487. "Admitted" means QSL's I2 admission of the emitted package
(`qsl-package`), which runs QSpec FR-370's reader.

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-488-AC-1 | Compiling `NonInterference` emits one `claim`/`hyperproperty` node of Boolean `semantic_type` whose body is an aggregate of exactly the seven bindings `domain`, `subjects`, `parameters`, `quantifiers`, `match`, `align_skip`, `body`, in that order. | Test (TC-1266) |
| FR-488-AC-2 | In AC-1's node, `domain` is the text `behaviours`; `subjects` is one binding `V` to the `model`/`model_root` node of `V`'s package; `parameters` is one binding `v` to the `model` node of `Vault`; `quantifiers` is a Tuple of exactly two quantifier Groups, `a` then `b`, each binding `variable`, `quantifier` `forall` and `alias` `V` and holding no fairness reference; `align_skip` is empty; `body` references a `temporal`/`temporal_clause` node. | Test (TC-1266) |
| FR-488-AC-3 | The I2 reader admits AC-1's package and returns the clause's `claim` triple and the body wrapper's NodeId. | Test (TC-1266) |
| FR-488-AC-4 | The body wrapper of `NonInterference` is a `quire.op.temporal.clause` application with one `temporal_profile` law equal to the package's row and the arguments `over` a reference to the `value`/`parameter` node `slot_sv_sa` at level 0, clock `model-steps`, activation binding `kind` to `origin` alone, empty captures, empty fairness, and `formula` a reference to a `quire.op.temporal.always` node with a `null` interval whose operand is a `holds` node. | Test (TC-1266) |
| FR-488-AC-5 | The `holds` operand of AC-4 is the equality of the field read `l` of `slot_sv_sa` (level 0) and of `slot_sv_sb` (level 1), with no term for `@`; the slot nodes have `expression` occurrences at `hyper 0 slot 0` and `hyper 0 slot 1` and `type` occurrences of their type node. | Test (TC-1266) |
| FR-488-AC-6 | `SavesPower` emits `subjects` with two bindings `S` and `N` in order of first naming, each to the `model`/`model_root` node of its alias's package, `parameters` `d` and `e` in declaration order, slot nodes `slot_sd_ss` and `slot_se_sn`, and a `holds` operand reading `used` of `slot_sd_ss` and of `slot_se_sn`. | Test (TC-1266) |
| FR-488-AC-7 | `NonInterference`'s `match` aggregate holds, in source order, a reference to the equality of the `parameter` nodes `op_sa` and `op_sb` (levels 0 and 1, `String`), then a reference to the equality of `arg_sa_sVault_sstep_si` and `arg_sb_sVault_sstep_si` (`Option<Int[0, 1]>`), with each conjunct root's `expression` occurrence at `hyper 0 match <k>`. | Test (TC-1266) |
| FR-488-AC-8 | In `match { a.step.op = V::Vault::step and b.step.op = stutter }` the first term compares its one slot with the `String` literal of `step`'s operation text and the second compares its one slot with the `String` literal `stutter`. A constant conjunct `true` is a `reference` to a boolean `literal` `value` node. A conjunct that reads only `b.step.op` has `op_sb` at level 0, and `op_sb` in `a.step.op = b.step.op` is a different node at level 1. | Test (TC-1266) |
| FR-488-AC-9 | For a clause whose conjuncts `c0` (universal), `c1` (reads an existential variable) and `c2` (universal) are written in that order, `match` holds `c0`, `c1`, `c2`, as `CheckedHyperClause::conjuncts()` returns them, with `c1` at `hyper 0 match exists 0`. | Test (TC-1266) |
| FR-488-AC-10 | `NonInterference` with `align skip { V::Vault::mix }` binds `align_skip` to one reference, to the `state`/`operation_anchor` node of `mix`, equal to the anchor node a state clause naming `mix` references; two listed operations give two references in written order. | Test (TC-1266) |
| FR-488-AC-11 | Each of these gives a `hyperproperty` NodeId different from `NonInterference`'s: the two quantifiers' aliases changed; one quantifier `exists`; the quantifier order reversed; `fair weak V::Vault::step` on `a`; the same constraint on `b` (its reference in `b`'s Group, not `a`'s); an added `match` conjunct; an `align skip` list; a changed body atom. | Test (TC-1266) |
| FR-488-AC-12 | Reformatting and a changed clause name give `NonInterference` the same NodeId; two structurally equal clause declarations share one node and have two `claim` occurrences with ordinals 0 and 1. A renamed trace variable gives a different NodeId (QSpec FR-402-AC-3). | Test (TC-1266) |
| FR-488-AC-13 | With `fair weak V::Vault::step` on `a`, the `quantifiers` Tuple holds `a`'s Group and then `b`'s; `a`'s Group binds `variable` `a`, `quantifier` `forall` and `alias` `V` and then holds one `reference` to a `temporal`/`fairness` node whose `fairness` member is `fairness_kind` `weak`, `granularity` `whole` (an unmarked granularity is `whole`, ADR-018 FA-6, FR-123) and an `operation` target naming `Vault`'s `model` node and `step`; `b`'s Group holds no reference. `fair weak whole V::Vault::step` emits identical bytes and `fair weak each V::Vault::step` a different fairness node and a different NodeId. The package is admitted. A constraint whose declaring node is a `systems_interface` is refused `unsupported_construct`/`not-yet-implemented` at the constraint. | Test (TC-1266) |
| FR-488-AC-23 | Every slot node name of AC-5 to AC-8 matches `[A-Za-z_][A-Za-z0-9_]*`, and the encoding is injective: `slot_s` names for object parameters `a_b` with trace variable `c` and `a` with trace variable `b_c` differ (`slot_sa__b_sc` and `slot_sa_sb__c`), a non-ASCII or punctuation byte is `_x` plus two uppercase hex digits, and names built from every pair of distinct (tag, fields) over a set including `_`, `s`, `x` and digit-leading names are pairwise distinct. The I2 admission raises no structural refusal (`invalid_package`/`InvalidSemanticGraph`) at any parameter node. | Test (TC-1266) |
| FR-488-AC-22 | Every emitted `hyperproperty` body of AC-1 to AC-13 passes strict wire validation of the flat grammar (QSpec FR-322 "Body grammar"): no term is refused `malformed_wire`, and the `quantifiers` binding is a Tuple of Groups whose members are bindings of text literals followed by references to `temporal`/`fairness` nodes, and the QSpec reader's temporal-step placement admits those references (QSpec FR-370-AC-21). | Test (TC-1266) |
| FR-488-AC-14 | After assembly, `CheckedHyperClause::occurrence()` of `NonInterference` equals the triple the I2 reader returns and FR-315-AC-12's `HyperProductIdentity::clause()`. A looked-up triple with an unknown NodeId, a non-`hyperproperty` NodeId, role `invariant` or another ordinal returns `hyper-root-unresolved` at the triple. | Test (TC-1266) |
| FR-488-AC-15 | The body wrapper is not among the I2 reader's temporal clause roots, looking it up as a clause root returns `clause-root-unresolved`, and FR-338's resolution of the wrapper's triple does the same. | Test (TC-1266) |
| FR-488-AC-16 | `Linked` with `B` = `(let r = v.next in present(r) and deref(value(r)).l = 1) @ a` emits. | Test (TC-1266) |
| FR-488-AC-17 | `Linked` with `B` = `(let r = (v.next @ a) in present(r) and deref(value(r)).l = 1) @ b` is refused `unsupported_construct`/`not-yet-implemented` at the read of `l`, and so is an atom whose read has no slot root (the object operand a call result) and one rooted in slots of two components. No package is produced. | Test (TC-1266) |
| FR-488-AC-18 | A clause under the timed profile is refused `unsupported_construct`/`not-yet-implemented` at the clause. A timed clause with an alias that binds no object parameter is refused at the clause, as the timed profile is checked first; the alias case alone is refused at the clause too. A clause with two refusals reports the first in the order of "Clauses S4 does not emit". | Test (TC-1266) |
| FR-488-AC-19 | An event-position false-extension clause with body `always[0,3] holds(...)` emits a wrapper whose law is that profile's row and an `always` node with the interval `{lower: "0", upper: "3"}`; an infinite-trace `v.l @ a = 0 until v.l @ b = 1` clause emits `quire.op.temporal.until` with a `null` interval and `eventually[2,*]` emits `{lower: "2", upper: null}`. All are admitted. | Test (TC-1266) |
| FR-488-AC-20 | A bounded-profile clause with `fair weak V::Vault::step` on `a`, which S3 admits, is refused `unsupported_construct`/`not-yet-implemented` at the constraint. | Test (TC-1266) |
| FR-488-AC-21 | The nodes this requirement mints count toward the package node ceiling: with a ceiling one node below `NonInterference`'s emission, compiling is refused with the ceiling's existing refusal, and at the ceiling it compiles. | Test (TC-1266) |
| FR-488-AC-24 | `NonInterference` emits one `temporal_clause_names` row keyed by its body wrapper's NodeId, at the ordinal of its assembled `claim` occurrence, with the name `NonInterference`; the `hyperproperty` node has no row; two structurally equal hyper clauses with different names share one wrapper and carry two rows at distinct ordinals; renaming one changes no NodeId; and the rows sort with the unit's temporal-clause rows by (NodeId, occurrence). | Test (TC-1327) |
| FR-488-AC-25 | Under event-position false-extension with a declared clock, the clause emits one `temporal_bindings` row at the wrapper's NodeId and the same occurrence, whose `clock_name` is the wrapper's clock text and whose `clock` is the declared configuration; an absent, unused or profile-mismatched clock selection refuses the compile with no package; under infinite-trace the clause has a name row and no binding. | Test (TC-1327) |

## Open QSpec questions

QSpec FR-402 answers Q-1, Q-2, Q-4, Q-5, Q-6, Q-7 and Q-8 (QSL-1093), and this
requirement follows it: each quantifier is one Group whose fairness is a list
of references to `temporal`/`fairness` nodes, which QSpec FR-370 now places
and QSpec FR-402's reader resolves (Q-1, Q-7); a subject is its package's
`model_root` (Q-2); a constant conjunct is a reference to its literal node
(Q-4); the body wrapper is QSpec's, excluded by every consumer from the
single-trace claims (Q-5); the slot names are QSpec's encoding (Q-8); and renaming a trace variable is
not spelling, so it changes the identity (Q-6). One question remains open:

- **Q-3.** A field read in a package carries no observation or component
  member, so a hyper atom's per-trace read has no carrier but the slot
  parameter node it is rooted in. QSL names the slot `slot_s<p>_s<t>` (escaped), refuses a
  read it cannot root, and asks QSpec to state a slot or component
  encoding (a `hyper_slot` member or a component on the read). QSpec also
  states no binder scope for an atom slot beyond slot 0 or for a `match`
  conjunct's slots, so QSL's one level rule (the slot's index in its term's
  slot list) leaves them free parameters, and the same read can have a node
  at two levels; QSpec should state the scope.

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
