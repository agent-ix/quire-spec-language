---
id: FR-093
title: "Lower checked Value expressions to FR-322 nodes with catalogued operations"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-005
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-092
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-065
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-094
    type: traces_to
---
# FR-093: Lower checked Value expressions to FR-322 nodes with catalogued operations

## Description

A node's FR-322 key hashes its body: the `quire.application-node/v1` key hashes
the application's operator, catalogued operation, result type and arguments.
`check` mints node keys at E3 (ADR-011 §2.2, FB-13), so `check` lowers each
checked expression into its FR-322 node before it can key it. This
requirement fixes that lowering: which nodes a checked expression becomes,
and the operator, operation identity, laws, mode, member and arguments of
each application. [FR-092](FR-092-key-type-parameter-and-declared-nodes.md)
fixes the type, parameter and function nodes the lowering references and the
preimage of every node that holds no application.

The source is the checked tree (`check::ir::Node`), not the parsed
`Expression`: each checked node carries its checked type, and FR-322 takes
every `result_type` and literal `type` from the checked type, never by
inference. Each `Expression` form checks to the `NodeKind` the table below
names.

QSpec references (`ix://agent-ix/quire-specification`, by reference, at
`e72756f`): FR-322 "Operation families, groups and constraints", the closed
`proposals/checked-package-v2/operation-catalog.json` and the operator-class
semantic forms its qualification tests fix.

## Inputs

- A checked function declaration: its parameters, its checked body and
  measure (`check::ir::Node` trees), and its declared result type.
- The type, parameter and function nodes of
  [FR-092](FR-092-key-type-parameter-and-declared-nodes.md).
- The package's lock evidence (ADR-011 §2.4): each law's `DefinitionRef`,
  read from QSpec's `complete-value-lock.json` accessor.

## Outputs

- The checked semantic graph's `Value`-family nodes, each with its FR-322
  body and its key, and for each node the source occurrences it was lowered
  from.
- Or a typed refusal and no node for the declaration.

## Behavior

### One node per checked expression

`check` SHALL lower each checked node to one FR-322 node, except a read of a
local slot (`NodeKind::Local`). An argument of an application is a
`reference` to its operand's node, or a `binding` where the operation's
operand is a binder. A `Local` read is a `reference` to the parameter node of
the binder that bound the slot (FR-092), and has no node of its own.

Nodes are content-addressed: two checked expressions that lower to the same
body, type and form are one node. Each source occurrence of a node is its own
occurrence entry, keyed (node id, role, ordinal) (ADR-013 O-07), with
ordinals per (node id, role) in order of (source document identity, region
start, region end):

- an expression's region is an `expression` occurrence;
- a binder's site (a parameter's `name: T`, a `let` name, a query binder) is
  an `anchor` occurrence of its parameter node, which carries no
  `declaration` (FR-092), and each read of it is an `expression` occurrence;
- a type form's region is a `type` occurrence of its type node;
- a node that no region denotes, such as the `Integer` and text scalar nodes
  that type the literals inside a type or parameter node's body, has one
  `generated` occurrence, so every node has at least one (FR-322).

An application node has `node_tag` `expression`, `semantic_type` equal to its
`result_type`, and the `semantic_form` its operator class fixes: `call`,
`unary`, `binary`, `conditional`, `let`, `quantify`, `collection`, `query`,
`conversion` for `convert`, `pre_read` for `pre`, `presence_read` for
`present`, `value_read` for `value`, `deref`, and `reachability` for
`reaches`. Its `result_type` is the type node (FR-092) of the checked node's
`value_type`. A literal's `type` is the type node of the literal's checked
type.

### Non-application nodes

| Checked node | Node |
|---|---|
| `Literal` of a Boolean, integer or rational value (`true`, `false`, an integer literal, `rational(n, d)`) | `value` / `literal`; `semantic_type` its type node; body `literal{type, value_kind, value}` spelled as FR-092 fixes |
| `Literal` of an enum member (`E::m`) | the enum member's QSpec `value` / `enum_value` node (FR-092 rule 1) |
| `Record{declaration, slots}` (`R { f: e, g: null }`) | `value` / `record_value`; `semantic_type` the record's node; body `aggregate` of one binding per slot in declaration order: `f` = `reference(e)` for a present slot, and `f` = `literal{type: the field's Option node, value_kind: "none", value: null}` for a `null` slot and for an omitted optional slot, which denote one value |
| `Tuple{declaration, arguments}` (`Q(a, b)`) | `value` / `tuple_value`; `semantic_type` the tuple's node; body `aggregate{[reference(a), reference(b)]}` |

### Application nodes

Each application's `operation` is built by FR-322's rules from its catalog
entry: `laws` holds one law per role the entry names, `mode` is a value of
the entry's mode kind, `member` is of the entry's member kind, and `leaves`
is empty unless the entry names a leaf source. For a leaf source, `leaves`
lists one `{path, laws, mode}` per text leaf of the compared type
(`operand:0`, `inner:0`), or for `result_inner` of the element type of a
`set`, `bag` or `ordered_set` result, in declaration order, each with the
`text_profile` law and the profile its type pins. `laws` is `[]`, `mode` is
`null`, `member` is `null` and `leaves` is `[]` unless the row says
otherwise.
`T(x)` is the type node of `x`'s checked type, and `ref(x)` is the `reference`
to `x`'s node.

| Checked node (`Expression` form) | `operator` | `operation.identity` | Member, mode, laws, leaves | `arguments` |
|---|---|---|---|---|
| `Let{slot, value, body}` (`let x = v in b`) | `let` | `quire.op.control.let` | | `[binding{name: x, value: ref(v)}, ref(b)]` |
| `If` (`if c then t else o`) | `conditional` | `quire.op.control.if` | | `[ref(c), ref(t), ref(o)]` |
| `Connective(And, Or, Implies)` | `binary` | `quire.op.boolean.and`, `.or`, `.implies` | | `[ref(l), ref(r)]` |
| `Not` | `unary` | `quire.op.boolean.not` | | `[ref(e)]` |
| `Arithmetic(Add, Subtract, Multiply)` | `binary` | `quire.op.integer.add`, `.sub`, `.mul` | | `[ref(l), ref(r)]` |
| `Negate` (integer) | `unary` | `quire.op.integer.negate` | | `[ref(e)]` |
| `Divide{domain}` (integer `/`) | `binary` | `quire.op.rational.div` | | `[ref(l), ref(r)]` |
| `Rational{operator}` | `binary` | `quire.op.rational.add`, `.sub`, `.mul`, `.div` | | `[ref(l), ref(r)]` |
| `RationalNegate` | `unary` | `quire.op.rational.negate` | | `[ref(e)]` |
| `Decimal{operator, target}` | `binary` | `quire.op.decimal.add`, `.sub`, `.mul`, `.div` | mode `rounding` = `target`'s rounding mode | `[ref(l), ref(r)]` |
| `DecimalNegate` | `unary` | `quire.op.decimal.negate` | | `[ref(e)]` |
| `Quantity(operator)` | `binary` | `quire.op.quantity.add`, `.sub`, `.mul`, `.div` | | `[ref(l), ref(r)]` |
| `Ieee(operator)` of width `w` | `binary` | `quire.op.ieee.float32.*` or `quire.op.ieee.float64.*` | law `ieee_profile`; mode `rounding` = the operand types' mode | `[ref(l), ref(r)]` |
| `Order(operator, kind)` (`<`, `<=`, `>`, `>=`) | `binary` | `quire.op.<f>.lt`, `.le`, `.gt`, `.ge`, where `<f>` is `integer`, `rational`, `decimal`, `enum`, `text` or `quantity` for `kind` `Integers`, `Rationals`, `Decimals`, `Enums`, `Texts`, `Quantities` | for `text`: law `text_profile`, mode `text_profile` = the operands' profile | `[ref(l), ref(r)]` |
| `Equality(operator, schedule)` (`=`, `!=`) | `binary` | `quire.op.<f>.eq` or `.ne` for an operand family `<f>` of `boolean`, `integer`, `rational`, `decimal`, `text`, `enum`, `quantity` or `reference`; `quire.op.structural.eq` or `.ne` for an `option`, record, tuple or collection operand | for `text`: law `text_profile`, mode `text_profile`; for `structural`: one leaf per text leaf of the compared type (catalog `operand:0`) | `[ref(l), ref(r)]` |
| `Coerce(e, interval)` (an integer admitted into an `Int[..]` that does not contain its type) | `convert` | `quire.op.numeric.narrow` | member `type_argument{declaration: T(Int[interval])}` | `[ref(e)]` |
| `ConvertScalar(target, e)` whose operand type is an exact numeric type other than the target | `convert` | `quire.op.numeric.convert` | member `type_argument` of the target type node | `[ref(e)]` |
| `ConvertScalar(target, e)` whose operand is a quantity | `convert` | `quire.op.quantity.convert` | member `type_argument` of the target type node; mode `rounding` = `exact` (Quantity conversion) | `[ref(e)]` |
| `ConvertDecimal(e, target)` (`convert<T>(e)` to a decimal) | `convert` | `quire.op.numeric.convert` when exact, `quire.op.numeric.convert_rounding` when FR-149 classifies it as scale reduction | member `type_argument` of `target`'s node; for `convert_rounding`, mode `rounding` = `target`'s mode | `[ref(e)]` |
| `IeeeToRational(e, domain)` | `convert` | `quire.op.ieee.to_rational` | law `ieee_profile`; member `type_argument` of `domain`'s node | `[ref(e)]` |
| `ConvertCollection{target, operand}` | `convert` | `quire.op.collection.convert` | member `type_argument` of `target`'s node; leaves `result_inner` | `[ref(e)]` |
| `Field{index}` (`e.f`) | `query` | `quire.op.record.project` | member `field{declaration: the record's node, name: f}` | `[ref(e)]` |
| `Present` | `present` | `quire.op.option.present` | | `[ref(e)]` |
| `Value` | `value` | `quire.op.option.value` | | `[ref(e)]` |
| `Call{function, arguments}` (`g(a, b)`) | `call` | `quire.op.function.call` | | `[ref(g's function node), ref(a), ref(b)]` |
| `Collection{collection_type, elements}` (`sequence[..]`, `set[..]`, `bag[..]`, `orderedSet[..]`) | `collection` | `quire.op.collection.sequence`, `.set`, `.bag`, `.ordered_set` | leaves `result_inner` for `set`, `bag`, `ordered_set` | `[ref(e0), ...]` in source order |
| `Query{visit: Map}` (`map`, `collect`) | `collection` | `quire.op.collection.map` | leaves `result_inner` | `[ref(c), binding{name: x, value: ref(body)}]` |
| `Query{visit: Filter}` | `collection` | `quire.op.collection.filter` | | `[ref(c), binding{name: x, value: ref(body)}]` |
| `Query{visit: Forall}`, `Query{visit: Exists}` | `quantify` | `quire.op.collection.forall`, `.exists` | | `[ref(c), binding{name: x, value: ref(body)}]` |
| `Query{visit: Count}` (`count<N>`) | `collection` | `quire.op.collection.count` | member `type_argument` of `T(node)` | `[ref(c), binding{name: x, value: ref(predicate)}]` |
| `Query{visit: Sum}` (`sum<N>`) | `collection` | `quire.op.collection.sum.integer` | member `type_argument` of `T(node)` | `[ref(c), binding{name: x, value: ref(summand)}]` |
| `Flatten` whose operand is not a `Query{visit: Map}` (`flatten(c)`) | `collection` | `quire.op.collection.flatten` | leaves `result_inner` | `[ref(c)]` |
| `Flatten` over a `Query{visit: Map}` (`flatMap(x in c: e)`, and `flatten(map(x in c: e))`) | `collection` | `quire.op.collection.flat_map`; no node is built for the inner map | leaves `result_inner` | `[ref(c), binding{name: x, value: ref(e)}]` |
| `Fold{identity: Some(i)}` (`fold<A>`) | `collection` | `quire.op.collection.fold` | member `type_argument` of `T(node)` | `[ref(c), binding{name: acc, value: binding{name: x, value: ref(step)}}, ref(i)]` |
| `Fold{identity: None}` (`reduce<A>`) | `collection` | `quire.op.collection.reduce` | member `type_argument` of `T(node)` | `[ref(c), binding{name: acc, value: binding{name: x, value: ref(step)}}]` |
| `Size` | `collection` | `quire.op.collection.size` | member `type_argument` of `T(node)` | `[ref(c)]` |
| `Contains` | `collection` | `quire.op.collection.contains` | leaves `inner:0` | `[ref(c), ref(v)]` |
| `Attribute{reference, name}` (`deref(r).f`) | `deref`, then `query` | the node `quire.op.model.deref` over `[ref(r)]`, whose `result_type` is the model node of `r`'s object type `T`, and over it `quire.op.record.project` | the projection's member `field{declaration: T's model node, name: f}` | the projection's `[ref(deref node)]` |
| `AllInstances` | `query` | `quire.op.model.all_instances` | member `type_argument` of the queried type's model node | `[ref(p)]` |
| `Lookup{absence}` | `query` | `quire.op.model.lookup` | mode `absence` = the authored mode; member `type_argument` of the queried type's model node | `[ref(p), ref(r)]` |
| `Dispatch{receiver, arguments}` | `call` | `quire.op.model.dispatch_call` | member `operation{declaration: the model node of the receiver's static object type, name: member}` | `[ref(receiver), ref(a0), ...]` |
| `Pre` | `pre` | `quire.op.state.pre` | | `[ref(e)]` |

A checked quantity type, `ValueType::Quantity(UnitId)`, has an exact
rational magnitude and no numeric domain, so the rounding mode it pins is
`exact`. FR-322 takes a `rounding` mode from the types
(`type_pinned_modes` lists `quantity`), and QSpec FR-142 converts a quantity
into an exact target with no loss. `quire.op.quantity.convert` therefore
carries mode `rounding` = `exact`.

A `Coerce` exists only for a narrowing: an integer whose type an `Int[..]`
contains is admitted with no node, and a `ConvertScalar` whose operand type
equals its target builds no node; its parent references the operand's node.

The model nodes, the `Reference<T>` and `Population<T>[N]` type nodes and
the model rows' vectors E4 to E9 are
[FR-094](FR-094-key-model-owned-reference-population-and-quantity-nodes.md)'s.

A `binding` names its binder with the source name, and the binder's parameter
node (FR-092) has that name and the binder's level. For `fold` and `reduce`,
the outer binding names the accumulator and the inner binding the element.

A law's `definition` is the `DefinitionRef` the package's lock evidence
selects for its role (ADR-011 §2.4): `text_profile` and `ieee_profile` read
from QSpec's `complete-value-lock.json` accessor. A node whose operation or
leaf needs a law that the lock evidence does not supply refuses with
`missing_declaration`/`missing-selection` naming the law role and the node's
region, and yields no node. `check` writes no law from a constant.

### Who builds the lowering

QSL-156 slice A4b builds this lowering and FR-092's keys, in the layer-3
`check` core, because E3 mints node keys and each key hashes the lowered
body (ADR-011 SG-1, FB-13). The checked semantic graph holds each lowered
node: its tag, form, semantic type, declaration, owner, body and key. The
QSL-6 slice S1b v2 emission arm (layer-4 `package`) writes those nodes to the
wire: it adds each node's `node_id`, `dependencies`, `occurrences` and
`recursion_group` and the graph order, and it builds no body term and mints
no key of its own. A recursion group's members carry one `recursion_group`
label, the group digest as a lowercase hex string (FR-092), and the emission
writes them in ordinal order, so FR-322's ordinal, which a reader derives
from graph order, equals the ordinal each member's key hashes.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-093-CON-1 | The lowering `match` over `NodeKind` has one arm per variant and no `_` or catch-all arm (ADR-012 §5.1), so a new checked node kind does not compile until it has a lowering. | Design | Test (TC-415) |
| FR-093-CON-2 | Non-test code in the layer-4 `package` crate builds no node body term, builds no node-identity preimage (`quire.application-node/v1`, `quire.structural-node/v1` or a nominal preimage) and calls no `NodeKey` constructor; it serializes the nodes `check` lowered. Its `package_id` preimage and the I2 reader's recomputation are outside this constraint. | Design | Test (TC-416) |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-093-AC-1 | For `function t using v(): Boolean pure { if true then true else true }`, the checked graph holds exactly one `value`/`literal` node for `true` and one `expression`/`conditional` node whose arguments are three `reference`s to that literal node, whose operation is `quire.op.control.if` and whose `result_type` and `semantic_type` are T1. The literal node has three `expression` occurrences, ordinals 0 to 2 in source order. | Test (TC-415) |
| FR-093-AC-2 | For `both`, `nb` (`function nb using v(a: Boolean): Boolean pure { both(a, true) }`) and `h` of FR-092-AC-4, the node of `a and b` keys to E1, the node of `both(a, true)` keys to E2 and the node of `let y = a in y` keys to E3, each with the preimage bytes FR-092 lists. No node is built for a read of `a`, `b` or `y`: each is a `reference` to that parameter's node. | Test (TC-415) |
| FR-093-AC-3 | For each row of the application table whose checked node the `Value` family builds in a function body (every row except `Attribute`, `AllInstances`, `Lookup`, `Dispatch` and `Pre`), a fixture function whose body holds that checked node lowers to one `expression` node with the row's `operator`, `semantic_form`, `operation.identity`, member, mode, laws, leaves and argument shape, and its `result_type` is the type node of the checked node's `value_type`. | Test (TC-415) |
| FR-093-AC-4 | `function c1 using v(x: Int[0, 9]): Int[0, 10] pure { x }` builds no convert node: `c1`'s `body` binding references `x`'s parameter node. `function c2 using v(): Int[0, 9] pure { 3 }` lowers its body to a `convert` node with `quire.op.numeric.narrow` and member `type_argument` naming `Int[0, 9]`'s node, over the literal `3`. `function c3 using v(x: Int[0, 9]): Rational[0, 9; 1, 1] pure { convert<Rational[0, 9; 1, 1]>(x) }` lowers to `quire.op.numeric.convert`. `function fm using v(s: Sequence<Sequence<Int[0, 9]>[0, 2]>[0, 3]): Sequence<Int[0, 9]>[0, 6] pure { flatMap(x in s: x) }` lowers its body to one `quire.op.collection.flat_map` node and no map node. | Test (TC-415) |
| FR-093-AC-5 | In `function q using v(s: Sequence<Int[0, 9]>[0, 5]): Boolean pure { forall(x in s: exists(y in s: x = y)) and exists(z in s: true) }`, `x`'s parameter node has level 1, `y`'s level 2 and `z`'s level 1. The `forall` argument list is `[reference(s's node), binding{name: "x", value: reference(the exists node)}]`. | Test (TC-415) |
| FR-093-AC-6 | A text equality whose package lock evidence supplies the text-profile definition carries law `text_profile` with that `DefinitionRef` and mode `text_profile` equal to the operands' profile. The same source, lowered with no text-profile definition in the lock evidence, refuses with `missing_declaration`/`missing-selection` naming role `text_profile`, and yields no node. | Test (TC-415) |
| FR-093-AC-7 | For every node of a checked package holding `both`, `nb`, `h`, `f` and `t`, the key recomputed from the node as the v2 emission arm writes it (the FR-322 application-node rule, or FR-092's structural-node rule) equals the node's `node_id`. The `package` crate's non-test code names no `SemanticTerm` constructor and no key function. For a package holding the recursive `f` of FR-092 vectors G4 to G6, the three members carry the `recursion_group` label `0b9e8d18320d0ce587699e40ac33a25fd41c4a640226bda4b8b1521edc5e4c50`, their graph order is G5, G4, G6, and the keys recomputed from that graph order are G4 to G6. | Test (TC-416) |
| FR-093-AC-8 | In the postcondition, invariant and dispatch fixtures of FR-153 and FR-151 (TC-196), a checked `Attribute`, `AllInstances`, `Lookup`, `Dispatch` and `Pre` node each lowers to the operation, member, mode and arguments its row gives, and a `Lookup` with `absent empty` carries mode `absence` = `empty`. | Test (TC-415) |
| FR-093-AC-9 | Every node of the checked package of AC-7 has at least one occurrence: `a`'s parameter node has an `anchor` occurrence over `a: Boolean` and one `expression` occurrence per read, and the `Integer` and text scalar nodes that type P1's body literals have a `generated` occurrence. | Test (TC-416) |

## Dependencies

- [FR-092](FR-092-key-type-parameter-and-declared-nodes.md): type, parameter
  and function nodes, the structural-node preimage and the literal spellings.
- [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)
  §2.2 E3 (keys minted at E3), §2.4 (lock evidence and law `DefinitionRef`s),
  §3 FB-13, §6.1 (a family's emission arm under `package`).
- [ADR-013](../decisions/ADR-013-canonical-type-package-conversion-ownership.md)
  O-04, O-07 (occurrences), QC-24.
- [FR-065](FR-065-migrate-function-application-to-checked-family.md): the
  function declaration and call identities this lowering keys.
- [FR-094](FR-094-key-model-owned-reference-population-and-quantity-nodes.md):
  the model declaration nodes the model rows' members name, and the
  `Reference`, `Population` and quantity type nodes.
- QSpec: the `complete-value-lock.json` accessor (ADR-011 §2.4). Until it is
  published, a node whose operation needs a law refuses (AC-6); nodes
  without laws are keyed. The two-name `fold`/`reduce` binder and the
  `bound` of a nested binding are QSL proposals (ADR-013 QC-24).
- IR-242: the IR reader's recursion preimage. It derives an in-group
  ordinal from graph order, which the emission sets to FR-092's group
  order.
- The `Attribute` row's `record.project` over a `deref` result: FR-322 gives a
  model entity type the family `reference`, and no QSpec fixture projects an
  attribute of a dereferenced object. The row is QSL's proposal (ADR-013
  QC-24).
- [ADR-012](../decisions/ADR-012-semantic-family-extension-contracts.md)
  §5.1: no catch-all arm.

## Status

Specified under QSL-208. Implemented on the QSL-156 slice A4b branch, pending merge: `check` lowers each checked
node in `qsl-semantics/src/check/lowering.rs` and keys it by FR-092, and
TC-415 backs AC-1 to AC-3, AC-5, AC-6, AC-8 and CON-1 there, and AC-4 except its `fm` fixture: the A4b test flat-maps a flat sequence over itself. A4b spells an integer
literal as a decimal string and gives `quire.op.quantity.convert` mode
`rounding` = `exact`. The emission half is QSL-6 S1b: `qsl-package/src/emit.rs`
refuses every non-empty graph (`ProjectionNotYetImplemented`), so AC-7, AC-9
and CON-2 (TC-416) are unbacked. Ownership, decided here: QSL-156 A4b builds
the lowering and the keys in `check`; QSL-6 S1b serializes the lowered nodes
and does not lower.
