---
id: FR-093
title: "Lower checked Value expressions to FR-322 nodes with catalogued operations"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-005
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-092
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-065
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
occurrence entry, keyed (node id, `expression`, ordinal) in source order
(ADR-013 O-07). A parameter node's declaration site is its `declaration`
occurrence, and each read of it is an `expression` occurrence.

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
| `Record{declaration, slots}` (`R { f: e, g: null }`) | `value` / `record_value`; `semantic_type` the record's node; body `aggregate` of one binding per slot in declaration order: `f` = `reference(e)` for a present slot, `f` = `literal{type: the field's Option node, value_kind: "none", value: null}` for a `null` slot; an omitted optional slot has no binding |
| `Tuple{declaration, arguments}` (`Q(a, b)`) | `value` / `tuple_value`; `semantic_type` the tuple's node; body `aggregate{[reference(a), reference(b)]}` |

### Application nodes

Every application's `operation.laws`, `mode`, `member` and `leaves` are
exactly those the catalog entry names. `laws` is `[]`, `mode` is `null`,
`member` is `null` and `leaves` is `[]` unless the row says otherwise.
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
| `Coerce(e, interval)` | `convert` | `quire.op.numeric.convert` when FR-149 classifies `T(e)` into `Int[interval]` as exact, `quire.op.numeric.narrow` when it classifies it as range narrowing | member `type_argument{declaration: T(Int[interval])}` | `[ref(e)]` |
| `ConvertScalar(target, e)` | `convert` | `quire.op.numeric.convert` | member `type_argument` of the target type node | `[ref(e)]` |
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
| `Flatten` (`flatten`, and the outer node of `flatMap`) | `collection` | `quire.op.collection.flatten` | leaves `result_inner` | `[ref(c)]` |
| `Fold{identity: Some(i)}` (`fold<A>`) | `collection` | `quire.op.collection.fold` | member `type_argument` of `T(node)` | `[ref(c), binding{name: acc, value: binding{name: x, value: ref(step)}}, ref(i)]` |
| `Fold{identity: None}` (`reduce<A>`) | `collection` | `quire.op.collection.reduce` | member `type_argument` of `T(node)` | `[ref(c), binding{name: acc, value: binding{name: x, value: ref(step)}}]` |
| `Size` | `collection` | `quire.op.collection.size` | member `type_argument` of `T(node)` | `[ref(c)]` |
| `Contains` | `collection` | `quire.op.collection.contains` | leaves `inner:0` | `[ref(c), ref(v)]` |
| `Attribute{reference, name}` (`deref(r).f`) | `deref`, then `query` | the node `quire.op.model.deref` over `[ref(r)]`, and over it `quire.op.record.project` | the projection's member `field{declaration: the object type's node, name: f}` | the projection's `[ref(deref node)]` |
| `AllInstances` | `query` | `quire.op.model.all_instances` | member `type_argument` of the queried type's node | `[ref(p)]` |
| `Lookup{absence}` | `query` | `quire.op.model.lookup` | mode `absence` = the authored mode; member `type_argument` of the queried type's node | `[ref(p), ref(r)]` |
| `Dispatch{receiver, arguments}` | `call` | `quire.op.model.dispatch_call` | member `operation{declaration: the declaring model node, name: member}` | `[ref(receiver), ref(a0), ...]` |
| `Pre` | `pre` | `quire.op.state.pre` | | `[ref(e)]` |

A `binding` names its binder with the source name, and the binder's parameter
node (FR-092) has that name and the binder's level. For `fold` and `reduce`,
the outer binding names the accumulator and the inner binding the element.

A law's `definition` is the `DefinitionRef` the package's lock evidence
selects for its role (ADR-011 §2.4): `text_profile` and `ieee_profile` read
from QSpec's `complete-value-lock.json` accessor. A node whose operation
needs a law that the lock evidence does not supply refuses with a typed
cause and yields no node. `check` writes no law from a constant.

### Who builds the lowering

QSL-156 slice A4b builds this lowering and FR-092's keys, in the layer-3
`check` core, because E3 mints node keys and each key hashes the lowered
body (ADR-011 SG-1, FB-13). The checked semantic graph holds each lowered
node: its tag, form, semantic type, declaration, owner, body and key. The
QSL-6 slice S1b v2 emission arm (layer-4 `package`) writes those nodes to the
wire: it adds each node's `node_id`, `dependencies`, `occurrences` and
`recursion_group` and the graph order, and it builds no body term and mints
no key of its own.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-093-CON-1 | The lowering `match` over `NodeKind` has one arm per variant and no `_` or catch-all arm (ADR-012 §5.1), so a new checked node kind does not compile until it has a lowering. | Design | Test (TC-415) |
| FR-093-CON-2 | Non-test code in the layer-4 `package` crate builds no `SemanticTerm` body and calls no preimage or key function; it serializes the nodes `check` lowered. | Design | Test (TC-416) |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-093-AC-1 | For `function t using v(): Boolean pure { if true then true else true }`, the checked graph holds exactly one `value`/`literal` node for `true` and one `expression`/`conditional` node whose arguments are three `reference`s to that literal node, whose operation is `quire.op.control.if` and whose `result_type` and `semantic_type` are T1. The literal node has three `expression` occurrences, ordinals 0 to 2 in source order. | Test (TC-415) |
| FR-093-AC-2 | For `both`, `nb` (`function nb using v(a: Boolean): Boolean pure { both(a, true) }`) and `h` of FR-092-AC-4, the node of `a and b` keys to E1, the node of `both(a, true)` keys to E2 and the node of `let y = a in y` keys to E3, each with the preimage bytes FR-092 lists. No node is built for a read of `a`, `b` or `y`: each is a `reference` to that parameter's node. | Test (TC-415) |
| FR-093-AC-3 | For each row of the application table, a fixture function whose body holds that checked node lowers to one `expression` node with the row's `operator`, `semantic_form`, `operation.identity`, member, mode, laws, leaves and argument shape, and its `result_type` is the type node of the checked node's `value_type`. | Test (TC-415) |
| FR-093-AC-4 | `function c1 using v(x: Int[0, 9]): Int[0, 10] pure { x }` lowers its body to a `convert` node with `quire.op.numeric.convert` and member `type_argument` naming `Int[0, 10]`'s node. `function c2 using v(): Int[0, 9] pure { 3 }` lowers its body to a `convert` node with `quire.op.numeric.narrow` over the literal `3`. | Test (TC-415) |
| FR-093-AC-5 | In `function q using v(s: Sequence<Int[0, 9]>[0, 5]): Boolean pure { forall(x in s: exists(y in s: x = y)) and exists(z in s: true) }`, `x`'s parameter node has level 1, `y`'s level 2 and `z`'s level 1. The `forall` argument list is `[reference(s's node), binding{name: "x", value: reference(the exists node)}]`. | Test (TC-415) |
| FR-093-AC-6 | A text equality whose package lock evidence supplies the text-profile definition carries law `text_profile` with that `DefinitionRef` and mode `text_profile` equal to the operands' profile. The same source, lowered with no text-profile definition in the lock evidence, refuses with a typed cause naming the missing law role, and yields no node. | Test (TC-415) |
| FR-093-AC-7 | For every node of a checked package holding `both`, `nb`, `h`, `f` and `t`, the key recomputed from the node as the v2 emission arm writes it (the FR-322 application-node rule, or FR-092's structural-node rule) equals the node's `node_id`. The `package` crate's non-test code names no `SemanticTerm` constructor and no key function. | Test (TC-416) |

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
- QSpec: the `complete-value-lock.json` accessor (ADR-011 §2.4). Until it is
  published, a node whose operation needs a law refuses (AC-6); nodes
  without laws are keyed. The two-name `fold`/`reduce` binder and the
  `bound` of a nested binding are QSL proposals (ADR-013 QC-24).
- IR-242: the IR reader's recursion preimage, for nodes in a recursion group.

## Status

Specified under QSL-208. Not implemented. `value::application_key`
(`qsl-semantics/src/value/application_key.rs`, QSL-156 A4a) builds the
`quire.application-node/v1` preimage from a given body and has no production
caller. It spells an integer literal as a JSON number
(`LiteralValue::Integer(i64)`); FR-092 spells it as a decimal string, and
A4b changes it. `qsl-package/src/emit.rs` refuses every non-empty graph
(`ProjectionNotYetImplemented`). Ownership, decided here: QSL-156 A4b builds
the lowering and the keys in `check`; QSL-6 S1b serializes the lowered nodes
and does not lower. TC-415 and TC-416 are planned.
