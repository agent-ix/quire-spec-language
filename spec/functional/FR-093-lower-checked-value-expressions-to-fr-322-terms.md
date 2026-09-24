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

### Nesting depth

Checking a function body walks its expressions to type them, to prove their
static definedness and to lower them. Each walk keeps the expressions it has
still to finish on a heap stack, so the host stack it uses does not grow
with the body's nesting. The guard facts the definedness walk derives from
one condition are derived over that condition's own nesting. The check
stage's depth limit (`CheckingLimits`, at most `MAX_CHECKING_DEPTH`) bounds
all of that nesting. Typing counts each expression it enters as one level
and refuses a body nested past the limit with
`resource_exhausted`/`insufficient-next-charge` naming the depth limit
(`CheckCause::ResourceExhausted`), before any node of it is lowered. The
walk that measures a declaration before typing (FR-062's input bytes, node
count and work) and the syntactic checks of a `pre(…)` operand walk the
parsed expression on a heap stack too, so a body nested past every limit
refuses on a limit.

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
is the text-leaf list (Text leaves, below) of the compared type
(`operand:0`, `inner:0`), or for `result_inner` of the element type of a
`set`, `bag` or `ordered_set` result. `laws` is `[]`, `mode` is
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
| `Equality(operator, schedule)` (`=`, `!=`) | `binary` | `quire.op.<f>.eq` or `.ne` for an operand family `<f>` of `boolean`, `integer`, `rational`, `decimal`, `text`, `enum`, `quantity` or `reference`; `quire.op.structural.eq` or `.ne` for an `option`, record, tuple or collection operand | for `text`: law `text_profile`, mode `text_profile`; for `structural`: leaves `operand:0`, the compared type's text-leaf list | `[ref(l), ref(r)]` |
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

The `Pre` row is the node of a checked `Pre`. `pre(...)` is legal only in
an operation postcondition (`ClauseKind::Postcondition`), and ADR-012 §4.3
moves `Pre` with the `ProtocolClause` lowering (#218, designed in #223).
`check` checks a postcondition through `check_postcondition_expression` and
keys no node for it, and `DeclaredClauseKind`, the clause kind of the
declarations this lowering keys (function bodies and measures, and FR-094's
invariant, precondition and body clause functions), has no postcondition
arm. `ProtocolClause` lowers an operation's postconditions with its checked
operation header and gives each its node, owner and clause binding. It
builds each `Pre` read inside a postcondition by this row, so a `Pre` node
keys to one id whichever family lowers the clause that holds it.

### Text leaves

`check` SHALL build the **text-leaf list** of a type by walking it from an
empty path and appending leaves in the order the walk reaches them. The walk
keeps the **open composites**: each declared record or tuple whose fields or
positions it is walking, with the number of segments the path had when the
walk entered it. At a type `U` and a path `p`:

1. `Text[min, max; profile]`: append the text leaf
   `{path: p, laws: [the text_profile law], mode: {kind: "text_profile", value: profile}}`.
2. `Option<V>`, or `K<V>` or `K<V>[min, max]` for a collection kind `K`:
   walk `V` at `p` + `inner`.
3. A declared record or tuple `C` that is open, entered at `d` segments:
   append the **recursion leaf**
   `{path: p + "recursion:d", laws: [], mode: null}` when a `Text` type is
   reachable from `C`, and nothing otherwise. The walk does not enter `C`
   again.
4. A declared record `C` that is not open: open `C` at the length of `p`;
   for each field in declaration order, walk a required field `f: V`'s `V`
   at `p` + `field:f` and an optional field `f?: V`'s `V` at `p` + `field:f`
   + `inner`; then close `C`. A record or tuple `C` from which no `Text`
   type is reachable is not entered, here or in step 5, and appends nothing.
5. A declared tuple `C` that is not open: open `C` at the length of `p`;
   walk each position `n`'s type, in position order, at `p` + `position:n`;
   then close `C`.
6. Any other type: nothing.

A `Text` type is **reachable** from `C` when some sequence of steps 2, 4
and 5, applied without regard to open composites, leads from `C` to a
`Text` type. A type alias walks as its resolved type (FR-091). `d` is
written as its canonical decimal string, as FR-322 writes `position:n`.

An optional field's path passes through `inner` because the field's slot
holds a value of its `Option<V>` node: FR-092 types the field by that node,
and an omitted or `null` slot is a literal of it (Non-application nodes).
So `f?: V` and `f: Option<V>`, which hold one set of values, have one leaf
path.

A recursion leaf stands for every text leaf below its path. For a
recursion leaf whose path is `q` + `recursion:d`, let `r` be the first `d`
segments of `q`. The value at `q` has the type of the value at `r`: the
composite `C`, entered at `r` and reentered at `q`. Each text leaf at a path
`r` + `s` therefore has a counterpart at `q` + `s` with the same laws and
mode. The **expanded leaf set** of a list is the least set that holds each
of the list's text leaves and, for each of its recursion leaves and each
text leaf in the set at a path `r` + `s`, that leaf re-rooted at `q` + `s`.
Every text leaf of the type's unfolding is in the set: a path through a
reentry at `q` is `q` + `s`, and `r` + `s` is a shorter path to a leaf of the
same type, so induction on path length ends at a text leaf of the list.

The list has these properties:

- **Finite.** A composite is open at most once at a time, so a path enters
  each declared composite of the package at most once, passes between two
  entries only the finitely many `inner`, `field` and `position` segments
  of one field or position type, and ends at the first reentry. The walk
  refuses past the check stage's depth limit as every FR-092 walk does.
  Because rule 3 ends each path at its first reentry, a recursive type's
  walk is as deep as its composites' nesting, not as deep as the limit.
- **Lossless.** The expanded leaf set is computed from the list alone, so
  the list determines every text leaf of the type's unfolding, with its path
  and profile. Two types whose text leaves differ in any path or profile
  therefore have different lists.
- **Without recursion.** A walk that reaches no open composite appends no
  recursion leaf. Its leaves are FR-322's text leaves, except that an
  optional field's path passes through `inner` (a QSL proposal, ADR-013
  QC-24). A recursive composite that reaches no `Text` type, such as
  FR-092's `List`, adds no leaf.
- **Independent of keys and group order.** `d` counts path segments. It is
  not a FR-092 group ordinal, and the list names no node, so an application
  node's leaves do not depend on its operand types' recursion group order or
  keys, and they name no member of that group.

A recursion leaf carries no law: each text leaf it stands for carries its
own. A type from which a `Text` type is reachable has at least one text leaf
in its list, because the shortest walk from the type to a `Text` type
enters no composite twice. The `missing-selection` refusal of a node whose
leaf needs the text-profile law therefore holds for recursive types too.

Each appended leaf, text or recursion, costs one unit of the check stage's
node limit (`CheckingLimits`), because the list is bounded per path but not
in width: `n` records that each hold a text field and an optional field of
every other record give on the order of `(n - 1)!` leaves. A walk that would
pass the limit refuses with `resource_exhausted`/`insufficient-next-charge`
naming the node limit, and yields no node. `check` completes the walk, or
refuses on a limit, before it reads any leaf's law, so a resource refusal
comes before `missing-selection`.

For `record Node { label: Text[0, 8; binary-utf8]; next?: Node; }`, the list
is `field:label`, a text leaf with mode `binary-utf8`, then
`field:next`, `inner`, `recursion:0`. The expanded leaf set holds
`field:label`, `field:next`/`inner`/`field:label`, and so on down every
chain of `next` fields.

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

### Node dependencies

A node's `dependencies` list the nodes its definition is built from. A v2
reader follows them, beside `semantic_type` and the body's own references,
when it builds a node's closure and checks graph cycles, and it joins them
exactly against the body of an application node, the nominal preimage of an
enum, dimension or unit node, and the entries of a frame (QSpec FR-322,
FR-340). They enter `package_id` through the `identity_projection`.

The v2 emission arm SHALL write each node's `dependencies` as the node ids,
unique and ascending by digest, that these rules name:

1. each `reference` target in the node's wire body, at any depth: an
   aggregate member, a binding's value or an application argument;
2. each declaration an application's `operation.member` names (every FR-322
   member kind that names a declaring node; QSL's rows use `field`,
   `operation` and `type_argument`);
3. for a `bounded_domain` node, its `semantic_type`, the type it bounds;
4. for a node that carries a `nominal_identity_preimage`, the nodes FR-322's
   nominal joins name: an `enum_value` node's enum declaration, a
   dimension's term dimensions, and a unit's dimension and target unit;
5. for a `state`/`frame` node, each entry of its `modifies`, `creates` and
   `deletes` (QSpec FR-340).

The five rules give the whole list. A literal's `type`, an application's
`result_type` and the `semantic_type` of a node other than a
`bounded_domain` are type annotations, which a reader reads through those
members; one is in the list only when a rule names the same node, as rule 4
does for an `enum_value` or a unit. A law's `definition` is a
`DefinitionRef`, not a node. Each dependency names a node of the package's
own graph: a `dependency_reference` (FR-322, ADR-013 QC-10) names a node of a
dependency package, adds no entry, and a reader reaches it through
`dependency_selections`.

On the wire, a reference to a member of the node's own recursion group is a
`reference` naming that member's `node_id`. `{term: "group_reference",
ordinal}` is its spelling inside a key preimage only (FR-092, FR-322
`application_node_preimage`). A group member is therefore a dependency by
rule 1, and by rule 3 when it is a `bounded_domain` node's `semantic_type`.
In FR-092's vectors, G6 `f(x - 1)` lists G4 and E13, and G9
`Sequence<Tree>[0, 3]` lists G8.

Rules 1 and 2 are FR-322's rule for an application node: "exactly the unique
digest-ascending reference targets and member declarations of its body".
Rule 4 is FR-322's nominal join and rule 5 FR-340's frame join. For every
other node QSL applies rules 1 and 3, which QSpec's
`positive-operation-identities.json` follows at each of its 58 nodes (ADR-013
QC-27). No v2 reader joins the list of such a node, and the list enters
`package_id`, so for these nodes `package_id` depends on the writer's rule
until QSpec states one.

| Node | `dependencies` |
|---|---|
| builtin `scalar_type` (`boolean`, `integer`, `rational`, `decimal`, `text`, `float32`, `float64`) | `[]`; the body is `aggregate{[]}` |
| `bounded_domain` (FR-092's ranges, `text_bounds`, `float_rounding`, `collection_bounds`; FR-094's `model_population`) | its `semantic_type` |
| `composite_type` `option`, a collection kind, `reference` | the node the body references |
| declared `composite_type` `record` or `tuple` | each field or position type node; an optional field's `Option` node |
| `value` `literal` or `parameter` | `[]`; the body holds literals only |
| `value` `record_value` or `tuple_value` | each node a present slot references |
| `scalar_type` `enum` (an enum declaration) | `[]`; its nominal preimage names no node |
| `value` `enum_value` | its enum declaration node |
| `expression` holding an application | rules 1 and 2 over the application |
| `function`, a clause function included | its parameter nodes, its body's root node and its measure's node |
| `model` or `relation` declaration node (FR-094) | `[]`; the body is `aggregate{[]}` |
| declared unit or dimension node (FR-094) | rule 4 |
| `scalar_type` `compound_unit` (FR-094) | each unit its terms reference |

### Comparison with QSpec's v2 positive fixtures

QSpec's v2 positive fixtures carry placeholders: the all-`1` `quire-edition`
edition (ADR-011 §2.4), the diagnostics `catalog` reference, and a
placeholder key for every node whose body holds no application. Each
fixture application key hashes placeholder operand keys. The emission's
`edition`, `definition_selections`, law `definition`s, `node_id`s, node keys
in bodies and `dependencies`, `semantic_type`s, `identity_preimage`,
`package_id`, `source_map` and `diagnostics` are therefore outside the
comparison.

The fixture test compares, for each application node of
`positive-operation-identities.json` and `positive-control-operations.json`
whose `operation.identity` a row of the application table lowers, the node
QSL emits for a function whose body holds that operation: its `node_tag`,
`semantic_form`, `operator`, `operation.identity`, law roles in order,
`mode`, member `kind` and `name`, each leaf's `path` and `mode`, and each
argument's term kind and binding name. IR's v2 reader admits each emitted
package. The test reads the fixtures from the `QSPEC_DIR` checkout under
`make conformance`.

### Recursive text-leaf vectors

Each vector below is the exact preimage bytes and the key `check` SHALL mint
for the named node, under owner (`a`, `u`), from these declarations:

- `record Node { label: Text[0, 8; binary-utf8]; next?: Node; }`, a group
  of two (G16, G17);
- `record A { name: Text[0, 8; binary-utf8]; b?: B; }` and
  `record B { tag: Text[0, 4; nfc]; a?: A; }`, a mutually recursive pair, a
  group of four (G18 to G21);
- `function eq using v(a: Node, b: Node): Boolean pure { a = b }` (E14);
- `function has using v(s: Sequence<Node>[0, 3], b: Node): Boolean pure { contains(s, b) }`
  (E15), whose `b` is E14's `b` (P11);
- `function eqa using v(x: A, y: A): Boolean pure { x = y }` (E16);
- `function eqo using v(a: Option<Node>, b: Option<Node>): Boolean pure { a = b }`
  (E17), whose recursion leaf's prefix is `inner`, one segment.

The lock evidence selects QSpec's text definition
`quire.value.text.unicode-17.0.0/v1`, revision `quire-draft`/`1-draft.1`,
the `DefinitionRef` of QSpec's `structural-eq-record` operation vector at
`e72756f`; every text leaf carries it as its `text_profile` law. The leaves
are:

| Vector | Compared type | Leaves, in order |
|---|---|---|
| E14, E15 | `Node` | `field:label` (`binary-utf8`); `field:next`, `inner`, `recursion:0` |
| E16 | `A` | `field:name` (`binary-utf8`); `field:b`, `inner`, `field:tag` (`nfc`); `field:b`, `inner`, `field:a`, `inner`, `recursion:0` |
| E17 | `Option<Node>` | `inner`, `field:label` (`binary-utf8`); `inner`, `field:next`, `inner`, `recursion:1` |

T1 to T3 are FR-092's. The keys were computed by a script that also
recomputes FR-092's T1 to T4, T7, T8, P1, P4, E11, E13, G2 to G15 and the
QSpec `structural-eq-record` and `collection-contains` operation vectors, which
fix the leaf spelling.

| Vector | Node | Key |
|---|---|---|
| T13 | `Text[0, 8; binary-utf8]` | `90156d6406244cce35d880a73c3280afa7ca4d3b912251cb3217d74126ff39d7` |
| T14 | `Text[0, 4; nfc]` | `41382183c79ab43356015828827799c0bb66da4eb126f4e43a8a6822f348dbef` |
| G16 | `record Node`, ordinal 0 | `12993602f5710f82bbf852543383c01b3ce75fae2dd269a556175352dc766af4` |
| G17 | `Option<Node>`, ordinal 1 | `57d82ab7977af236dc5f898b98693d88499665ac53e19a25df7919ee867a6f13` |
| G18 | `Option<A>`, ordinal 0 | `c50e48e0850cfad1b96eb970873a2f40bb9a76bc7570b4cb0fd5ecbcd8aeef04` |
| G19 | `record B`, ordinal 1 | `193c4ff199f62645b8410fee31906c24407a0b8f44c9531546d2feb26afa7e6f` |
| G20 | `record A`, ordinal 2 | `871ecfc1333281aeeba9465655892aeebc4f55d920add6eaa2883d3ba5eaf999` |
| G21 | `Option<B>`, ordinal 3 | `70d8b14aa580fd43e3b74db225dd15e921dd086f02e9ccb60ff1f880940dab65` |
| S4 | `Sequence<Node>` | `31a87726ba061f8a52d37f2b1fdc5cd37639e451398e41ad194441e0c6065166` |
| S5 | `Sequence<Node>[0, 3]` | `0d46108bcd90e1c86a9b292c11fe2d6368a041c9b117df067059ad793868c343` |
| P10 | parameter `a: Node`, level 0 | `28de329c02d58a0d8950b48f884f7bfe9b8c349ef1401d3570cdd93413aa98ce` |
| P11 | parameter `b: Node`, level 1 | `bd3d0f259771dbd6aa6bdf0fc135e60d16cecd1e4474b677a3e695cfaa9386b7` |
| P12 | parameter `s: Sequence<Node>[0, 3]`, level 0 | `179e6f8dbe0f7943bc07142cb3426073a72ed652c8a1a97d265dea817da0f24f` |
| P13 | parameter `x: A`, level 0 | `5c4dbf18699020a60de695300b93f610c4a9900b0c975c5106095bce1f601376` |
| P14 | parameter `y: A`, level 1 | `518c4adf51b804f39682ed5590a0480540dbe3f44b0560b6c30ce841b92f5dda` |
| P15 | parameter `a: Option<Node>`, level 0 | `5edb6e8d2054522ce9131bb1129c12cbef280ee76cba8ed93b8cc8cfaf5bcec1` |
| P16 | parameter `b: Option<Node>`, level 1 | `af2b0e4ae044dd50d47cd580242dc4b41cde13fdd0e0f562a97f155050a6fce8` |
| E14 | `a = b` over `Node` | `b7b375bfadf766a83df340eca04ee098a257fa14d74de525666ac69a1ff4212e` |
| E15 | `contains(s, b)` over `Sequence<Node>[0, 3]` | `711360736369b8de6c907d1c961b8cff04cd2c79b6fc768fb9f4efb05b42710e` |
| E16 | `x = y` over `A` | `a75b4fe5ff706e7100dc5b177565ac3b4a5383f70d3eab4f1320c665ccc11ce4` |
| E17 | `a = b` over `Option<Node>` | `c51fd9a2d78d5682b27d7dc90020825e9e26a112e7b186386946fc28e18d623a` |

**T13**: `Text[0, 8; binary-utf8]`

```json
{"body":{"members":[{"name":"min","term":"binding","value":{"term":"literal","type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"value":"0","value_kind":"integer"}},{"name":"max","term":"binding","value":{"term":"literal","type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"value":"8","value_kind":"integer"}},{"name":"text_profile","term":"binding","value":{"term":"literal","type":{"digest":"0062ac9eee212061689152b5b0d62c5b3a6ca69ce83c1897b8c0624837ab5659","domain":"quire.checked-semantic-node/v1"},"value":"binary-utf8","value_kind":"text"}}],"term":"aggregate"},"declaration":null,"node_tag":"bounded_domain","recursion":null,"semantic_form":"text_bounds","semantic_type":{"digest":"0062ac9eee212061689152b5b0d62c5b3a6ca69ce83c1897b8c0624837ab5659","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: `90156d6406244cce35d880a73c3280afa7ca4d3b912251cb3217d74126ff39d7`

**T14**: `Text[0, 4; nfc]`

```json
{"body":{"members":[{"name":"min","term":"binding","value":{"term":"literal","type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"value":"0","value_kind":"integer"}},{"name":"max","term":"binding","value":{"term":"literal","type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"value":"4","value_kind":"integer"}},{"name":"text_profile","term":"binding","value":{"term":"literal","type":{"digest":"0062ac9eee212061689152b5b0d62c5b3a6ca69ce83c1897b8c0624837ab5659","domain":"quire.checked-semantic-node/v1"},"value":"nfc","value_kind":"text"}}],"term":"aggregate"},"declaration":null,"node_tag":"bounded_domain","recursion":null,"semantic_form":"text_bounds","semantic_type":{"digest":"0062ac9eee212061689152b5b0d62c5b3a6ca69ce83c1897b8c0624837ab5659","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: `41382183c79ab43356015828827799c0bb66da4eb126f4e43a8a6822f348dbef`

**G16**: `record Node`, ordinal 0

```json
{"body":{"members":[{"name":"label","term":"binding","value":{"target":{"digest":"90156d6406244cce35d880a73c3280afa7ca4d3b912251cb3217d74126ff39d7","domain":"quire.checked-semantic-node/v1"},"term":"reference"}},{"name":"next","term":"binding","value":{"name":"optional","term":"binding","value":{"ordinal":1,"term":"group_reference"}}}],"term":"aggregate"},"declaration":{"qualified_name":["Node"]},"node_tag":"composite_type","owner":{"authority":"a","identity":"u","kind":"source"},"recursion":{"group":"5479ad5c83c36a2788eafecaa0a608d82573b59b3bc46c44c917bc082ace8254","ordinal":0,"size":2},"semantic_form":"record","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: `12993602f5710f82bbf852543383c01b3ce75fae2dd269a556175352dc766af4`

**G17**: `Option<Node>`, ordinal 1

```json
{"body":{"members":[{"ordinal":0,"term":"group_reference"}],"term":"aggregate"},"declaration":null,"node_tag":"composite_type","recursion":{"group":"5479ad5c83c36a2788eafecaa0a608d82573b59b3bc46c44c917bc082ace8254","ordinal":1,"size":2},"semantic_form":"option","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: `57d82ab7977af236dc5f898b98693d88499665ac53e19a25df7919ee867a6f13`

**G18**: `Option<A>`, ordinal 0

```json
{"body":{"members":[{"ordinal":2,"term":"group_reference"}],"term":"aggregate"},"declaration":null,"node_tag":"composite_type","recursion":{"group":"3416bf755bd330e4277e231f64f2a0ac5bc9d69530f62f16f71a9f8a633919f0","ordinal":0,"size":4},"semantic_form":"option","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: `c50e48e0850cfad1b96eb970873a2f40bb9a76bc7570b4cb0fd5ecbcd8aeef04`

**G19**: `record B`, ordinal 1

```json
{"body":{"members":[{"name":"tag","term":"binding","value":{"target":{"digest":"41382183c79ab43356015828827799c0bb66da4eb126f4e43a8a6822f348dbef","domain":"quire.checked-semantic-node/v1"},"term":"reference"}},{"name":"a","term":"binding","value":{"name":"optional","term":"binding","value":{"ordinal":0,"term":"group_reference"}}}],"term":"aggregate"},"declaration":{"qualified_name":["B"]},"node_tag":"composite_type","owner":{"authority":"a","identity":"u","kind":"source"},"recursion":{"group":"3416bf755bd330e4277e231f64f2a0ac5bc9d69530f62f16f71a9f8a633919f0","ordinal":1,"size":4},"semantic_form":"record","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: `193c4ff199f62645b8410fee31906c24407a0b8f44c9531546d2feb26afa7e6f`

**G20**: `record A`, ordinal 2

```json
{"body":{"members":[{"name":"name","term":"binding","value":{"target":{"digest":"90156d6406244cce35d880a73c3280afa7ca4d3b912251cb3217d74126ff39d7","domain":"quire.checked-semantic-node/v1"},"term":"reference"}},{"name":"b","term":"binding","value":{"name":"optional","term":"binding","value":{"ordinal":3,"term":"group_reference"}}}],"term":"aggregate"},"declaration":{"qualified_name":["A"]},"node_tag":"composite_type","owner":{"authority":"a","identity":"u","kind":"source"},"recursion":{"group":"3416bf755bd330e4277e231f64f2a0ac5bc9d69530f62f16f71a9f8a633919f0","ordinal":2,"size":4},"semantic_form":"record","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: `871ecfc1333281aeeba9465655892aeebc4f55d920add6eaa2883d3ba5eaf999`

**G21**: `Option<B>`, ordinal 3

```json
{"body":{"members":[{"ordinal":1,"term":"group_reference"}],"term":"aggregate"},"declaration":null,"node_tag":"composite_type","recursion":{"group":"3416bf755bd330e4277e231f64f2a0ac5bc9d69530f62f16f71a9f8a633919f0","ordinal":3,"size":4},"semantic_form":"option","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: `70d8b14aa580fd43e3b74db225dd15e921dd086f02e9ccb60ff1f880940dab65`

**S4**: `Sequence<Node>`

```json
{"body":{"members":[{"target":{"digest":"12993602f5710f82bbf852543383c01b3ce75fae2dd269a556175352dc766af4","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"term":"aggregate"},"declaration":null,"node_tag":"composite_type","recursion":null,"semantic_form":"sequence","semantic_type":null,"version":"quire.structural-node/v1"}
```

Key: `31a87726ba061f8a52d37f2b1fdc5cd37639e451398e41ad194441e0c6065166`

**S5**: `Sequence<Node>[0, 3]`

```json
{"body":{"members":[{"name":"min","term":"binding","value":{"term":"literal","type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"value":"0","value_kind":"integer"}},{"name":"max","term":"binding","value":{"term":"literal","type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"value":"3","value_kind":"integer"}}],"term":"aggregate"},"declaration":null,"node_tag":"bounded_domain","recursion":null,"semantic_form":"collection_bounds","semantic_type":{"digest":"31a87726ba061f8a52d37f2b1fdc5cd37639e451398e41ad194441e0c6065166","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: `0d46108bcd90e1c86a9b292c11fe2d6368a041c9b117df067059ad793868c343`

**P10**: parameter `a: Node`, level 0

```json
{"body":{"members":[{"name":"name","term":"binding","value":{"term":"literal","type":{"digest":"0062ac9eee212061689152b5b0d62c5b3a6ca69ce83c1897b8c0624837ab5659","domain":"quire.checked-semantic-node/v1"},"value":"a","value_kind":"text"}},{"name":"level","term":"binding","value":{"term":"literal","type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"value":"0","value_kind":"integer"}}],"term":"aggregate"},"declaration":null,"node_tag":"value","recursion":null,"semantic_form":"parameter","semantic_type":{"digest":"12993602f5710f82bbf852543383c01b3ce75fae2dd269a556175352dc766af4","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: `28de329c02d58a0d8950b48f884f7bfe9b8c349ef1401d3570cdd93413aa98ce`

**P11**: parameter `b: Node`, level 1

```json
{"body":{"members":[{"name":"name","term":"binding","value":{"term":"literal","type":{"digest":"0062ac9eee212061689152b5b0d62c5b3a6ca69ce83c1897b8c0624837ab5659","domain":"quire.checked-semantic-node/v1"},"value":"b","value_kind":"text"}},{"name":"level","term":"binding","value":{"term":"literal","type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"value":"1","value_kind":"integer"}}],"term":"aggregate"},"declaration":null,"node_tag":"value","recursion":null,"semantic_form":"parameter","semantic_type":{"digest":"12993602f5710f82bbf852543383c01b3ce75fae2dd269a556175352dc766af4","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: `bd3d0f259771dbd6aa6bdf0fc135e60d16cecd1e4474b677a3e695cfaa9386b7`

**P12**: parameter `s: Sequence<Node>[0, 3]`, level 0

```json
{"body":{"members":[{"name":"name","term":"binding","value":{"term":"literal","type":{"digest":"0062ac9eee212061689152b5b0d62c5b3a6ca69ce83c1897b8c0624837ab5659","domain":"quire.checked-semantic-node/v1"},"value":"s","value_kind":"text"}},{"name":"level","term":"binding","value":{"term":"literal","type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"value":"0","value_kind":"integer"}}],"term":"aggregate"},"declaration":null,"node_tag":"value","recursion":null,"semantic_form":"parameter","semantic_type":{"digest":"0d46108bcd90e1c86a9b292c11fe2d6368a041c9b117df067059ad793868c343","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: `179e6f8dbe0f7943bc07142cb3426073a72ed652c8a1a97d265dea817da0f24f`

**P13**: parameter `x: A`, level 0

```json
{"body":{"members":[{"name":"name","term":"binding","value":{"term":"literal","type":{"digest":"0062ac9eee212061689152b5b0d62c5b3a6ca69ce83c1897b8c0624837ab5659","domain":"quire.checked-semantic-node/v1"},"value":"x","value_kind":"text"}},{"name":"level","term":"binding","value":{"term":"literal","type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"value":"0","value_kind":"integer"}}],"term":"aggregate"},"declaration":null,"node_tag":"value","recursion":null,"semantic_form":"parameter","semantic_type":{"digest":"871ecfc1333281aeeba9465655892aeebc4f55d920add6eaa2883d3ba5eaf999","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: `5c4dbf18699020a60de695300b93f610c4a9900b0c975c5106095bce1f601376`

**P14**: parameter `y: A`, level 1

```json
{"body":{"members":[{"name":"name","term":"binding","value":{"term":"literal","type":{"digest":"0062ac9eee212061689152b5b0d62c5b3a6ca69ce83c1897b8c0624837ab5659","domain":"quire.checked-semantic-node/v1"},"value":"y","value_kind":"text"}},{"name":"level","term":"binding","value":{"term":"literal","type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"value":"1","value_kind":"integer"}}],"term":"aggregate"},"declaration":null,"node_tag":"value","recursion":null,"semantic_form":"parameter","semantic_type":{"digest":"871ecfc1333281aeeba9465655892aeebc4f55d920add6eaa2883d3ba5eaf999","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: `518c4adf51b804f39682ed5590a0480540dbe3f44b0560b6c30ce841b92f5dda`

**P15**: parameter `a: Option<Node>`, level 0

```json
{"body":{"members":[{"name":"name","term":"binding","value":{"term":"literal","type":{"digest":"0062ac9eee212061689152b5b0d62c5b3a6ca69ce83c1897b8c0624837ab5659","domain":"quire.checked-semantic-node/v1"},"value":"a","value_kind":"text"}},{"name":"level","term":"binding","value":{"term":"literal","type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"value":"0","value_kind":"integer"}}],"term":"aggregate"},"declaration":null,"node_tag":"value","recursion":null,"semantic_form":"parameter","semantic_type":{"digest":"57d82ab7977af236dc5f898b98693d88499665ac53e19a25df7919ee867a6f13","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: `5edb6e8d2054522ce9131bb1129c12cbef280ee76cba8ed93b8cc8cfaf5bcec1`

**P16**: parameter `b: Option<Node>`, level 1

```json
{"body":{"members":[{"name":"name","term":"binding","value":{"term":"literal","type":{"digest":"0062ac9eee212061689152b5b0d62c5b3a6ca69ce83c1897b8c0624837ab5659","domain":"quire.checked-semantic-node/v1"},"value":"b","value_kind":"text"}},{"name":"level","term":"binding","value":{"term":"literal","type":{"digest":"07f6dca966d22bde13d3bb198f12610e57d8e1e04d0476bbab03f405d2b04e32","domain":"quire.checked-semantic-node/v1"},"value":"1","value_kind":"integer"}}],"term":"aggregate"},"declaration":null,"node_tag":"value","recursion":null,"semantic_form":"parameter","semantic_type":{"digest":"57d82ab7977af236dc5f898b98693d88499665ac53e19a25df7919ee867a6f13","domain":"quire.checked-semantic-node/v1"},"version":"quire.structural-node/v1"}
```

Key: `af2b0e4ae044dd50d47cd580242dc4b41cde13fdd0e0f562a97f155050a6fce8`

**E14**: `a = b` over `Node`

```json
{"body":{"arguments":[{"target":{"digest":"28de329c02d58a0d8950b48f884f7bfe9b8c349ef1401d3570cdd93413aa98ce","domain":"quire.checked-semantic-node/v1"},"term":"reference"},{"target":{"digest":"bd3d0f259771dbd6aa6bdf0fc135e60d16cecd1e4474b677a3e695cfaa9386b7","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"operation":{"identity":"quire.op.structural.eq","laws":[],"leaves":[{"laws":[{"definition":{"authority":"agent-ix","digest":"cd4a985a0d7d2f2b3d3625caee3787832c00c5244e805fb49e1c2c7075b9de5e","digest_domain":"quire.definition.bytes/v1","identity":"quire.value.text.unicode-17.0.0/v1","revision":{"namespace":"quire-draft","value":"1-draft.1"}},"role":"text_profile"}],"mode":{"kind":"text_profile","value":"binary-utf8"},"path":["field:label"]},{"laws":[],"mode":null,"path":["field:next","inner","recursion:0"]}],"member":null,"mode":null},"operator":"binary","result_type":{"digest":"9964390677844ad66b781babdbfa95933bc2b16ef1e86f67005966b77e6db3aa","domain":"quire.checked-semantic-node/v1"},"term":"application"},"declaration":null,"node_tag":"expression","recursion":null,"semantic_form":"binary","semantic_type":{"digest":"9964390677844ad66b781babdbfa95933bc2b16ef1e86f67005966b77e6db3aa","domain":"quire.checked-semantic-node/v1"},"version":"quire.application-node/v1"}
```

Key: `b7b375bfadf766a83df340eca04ee098a257fa14d74de525666ac69a1ff4212e`

**E15**: `contains(s, b)` over `Sequence<Node>[0, 3]`

```json
{"body":{"arguments":[{"target":{"digest":"179e6f8dbe0f7943bc07142cb3426073a72ed652c8a1a97d265dea817da0f24f","domain":"quire.checked-semantic-node/v1"},"term":"reference"},{"target":{"digest":"bd3d0f259771dbd6aa6bdf0fc135e60d16cecd1e4474b677a3e695cfaa9386b7","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"operation":{"identity":"quire.op.collection.contains","laws":[],"leaves":[{"laws":[{"definition":{"authority":"agent-ix","digest":"cd4a985a0d7d2f2b3d3625caee3787832c00c5244e805fb49e1c2c7075b9de5e","digest_domain":"quire.definition.bytes/v1","identity":"quire.value.text.unicode-17.0.0/v1","revision":{"namespace":"quire-draft","value":"1-draft.1"}},"role":"text_profile"}],"mode":{"kind":"text_profile","value":"binary-utf8"},"path":["field:label"]},{"laws":[],"mode":null,"path":["field:next","inner","recursion:0"]}],"member":null,"mode":null},"operator":"collection","result_type":{"digest":"9964390677844ad66b781babdbfa95933bc2b16ef1e86f67005966b77e6db3aa","domain":"quire.checked-semantic-node/v1"},"term":"application"},"declaration":null,"node_tag":"expression","recursion":null,"semantic_form":"collection","semantic_type":{"digest":"9964390677844ad66b781babdbfa95933bc2b16ef1e86f67005966b77e6db3aa","domain":"quire.checked-semantic-node/v1"},"version":"quire.application-node/v1"}
```

Key: `711360736369b8de6c907d1c961b8cff04cd2c79b6fc768fb9f4efb05b42710e`

**E16**: `x = y` over `A`

```json
{"body":{"arguments":[{"target":{"digest":"5c4dbf18699020a60de695300b93f610c4a9900b0c975c5106095bce1f601376","domain":"quire.checked-semantic-node/v1"},"term":"reference"},{"target":{"digest":"518c4adf51b804f39682ed5590a0480540dbe3f44b0560b6c30ce841b92f5dda","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"operation":{"identity":"quire.op.structural.eq","laws":[],"leaves":[{"laws":[{"definition":{"authority":"agent-ix","digest":"cd4a985a0d7d2f2b3d3625caee3787832c00c5244e805fb49e1c2c7075b9de5e","digest_domain":"quire.definition.bytes/v1","identity":"quire.value.text.unicode-17.0.0/v1","revision":{"namespace":"quire-draft","value":"1-draft.1"}},"role":"text_profile"}],"mode":{"kind":"text_profile","value":"binary-utf8"},"path":["field:name"]},{"laws":[{"definition":{"authority":"agent-ix","digest":"cd4a985a0d7d2f2b3d3625caee3787832c00c5244e805fb49e1c2c7075b9de5e","digest_domain":"quire.definition.bytes/v1","identity":"quire.value.text.unicode-17.0.0/v1","revision":{"namespace":"quire-draft","value":"1-draft.1"}},"role":"text_profile"}],"mode":{"kind":"text_profile","value":"nfc"},"path":["field:b","inner","field:tag"]},{"laws":[],"mode":null,"path":["field:b","inner","field:a","inner","recursion:0"]}],"member":null,"mode":null},"operator":"binary","result_type":{"digest":"9964390677844ad66b781babdbfa95933bc2b16ef1e86f67005966b77e6db3aa","domain":"quire.checked-semantic-node/v1"},"term":"application"},"declaration":null,"node_tag":"expression","recursion":null,"semantic_form":"binary","semantic_type":{"digest":"9964390677844ad66b781babdbfa95933bc2b16ef1e86f67005966b77e6db3aa","domain":"quire.checked-semantic-node/v1"},"version":"quire.application-node/v1"}
```

Key: `a75b4fe5ff706e7100dc5b177565ac3b4a5383f70d3eab4f1320c665ccc11ce4`

**E17**: `a = b` over `Option<Node>`

```json
{"body":{"arguments":[{"target":{"digest":"5edb6e8d2054522ce9131bb1129c12cbef280ee76cba8ed93b8cc8cfaf5bcec1","domain":"quire.checked-semantic-node/v1"},"term":"reference"},{"target":{"digest":"af2b0e4ae044dd50d47cd580242dc4b41cde13fdd0e0f562a97f155050a6fce8","domain":"quire.checked-semantic-node/v1"},"term":"reference"}],"operation":{"identity":"quire.op.structural.eq","laws":[],"leaves":[{"laws":[{"definition":{"authority":"agent-ix","digest":"cd4a985a0d7d2f2b3d3625caee3787832c00c5244e805fb49e1c2c7075b9de5e","digest_domain":"quire.definition.bytes/v1","identity":"quire.value.text.unicode-17.0.0/v1","revision":{"namespace":"quire-draft","value":"1-draft.1"}},"role":"text_profile"}],"mode":{"kind":"text_profile","value":"binary-utf8"},"path":["inner","field:label"]},{"laws":[],"mode":null,"path":["inner","field:next","inner","recursion:1"]}],"member":null,"mode":null},"operator":"binary","result_type":{"digest":"9964390677844ad66b781babdbfa95933bc2b16ef1e86f67005966b77e6db3aa","domain":"quire.checked-semantic-node/v1"},"term":"application"},"declaration":null,"node_tag":"expression","recursion":null,"semantic_form":"binary","semantic_type":{"digest":"9964390677844ad66b781babdbfa95933bc2b16ef1e86f67005966b77e6db3aa","domain":"quire.checked-semantic-node/v1"},"version":"quire.application-node/v1"}
```

Key: `c51fd9a2d78d5682b27d7dc90020825e9e26a112e7b186386946fc28e18d623a`

G16-G17, group digest `5479ad5c83c36a2788eafecaa0a608d82573b59b3bc46c44c917bc082ace8254`:

| Ordinal | Member | Anonymous signature | Full signature |
|---|---|---|---|
| 0 | G16 `Node` | `48941cd392fb2016446dcf8d97800d86c49aa8245cf24059b1898720f7f48c22` | `7a49f3f01641ff7adf2eea5f8c494c0a82d536535da4123194f9c56c4f23166e` |
| 1 | G17 `Option<Node>` | `581d248695dafdd3eaa1e684a6ff9085b68d9f192adfa77cf9d5162ca8048266` | `c2ed1d827853bc7815578d453189cff97a1cf9751934e6d0a591a3d8312f5c69` |

G18-G21, group digest `3416bf755bd330e4277e231f64f2a0ac5bc9d69530f62f16f71a9f8a633919f0`:

| Ordinal | Member | Anonymous signature | Full signature |
|---|---|---|---|
| 0 | G18 `Option<A>` | `12b7c54627ccf76f4aea35baad2db8f0fb96198bcf4b2a7c2175344d2b24b87d` | `9fda411f5813f0a323845e7683ac88c9cda6046391b4d56b262426937ce03326` |
| 1 | G19 `B` | `184d0868899c7e726ab53ad66f8e4c3d052cbd74719a4217885a169ea42d0ecb` | `6e0ba537121f0427ff289ba7cefd2f922a80662047290af8a557f4a1e3ccb731` |
| 2 | G20 `A` | `36833168fd92bea4e7c82f9f6f5159d8a8f2eec3b2d6522ba6be5d1be5c7ff36` | `ca73f785d623459b510ed8e012d359387abaa9fdbb48e1020a3724e8a12eab04` |
| 3 | G21 `Option<B>` | `5049b247603f321a7904b9548e7e679a988fea550900a0afadb405afec1ab97c` | `225dd48ffda2cef36053d63e818df1b3d1725776b1ebb96580b3cf218f43bfd2` |

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
| FR-093-AC-8 | In functions over a `Population<M::Order>[3]` parameter `p` and a `Reference<M::Order>` parameter `r` (FR-094 vectors E4 to E9), and in a clause function (FR-094) that holds a dispatched call (QSpec FR-151, QSpec TC-196 D06), a checked `Attribute`, `AllInstances`, `Lookup` and `Dispatch` node each lowers to the operation, member, mode and arguments its row gives, and a `Lookup` with `absent empty` carries mode `absence` = `empty`. | Test (TC-415) |
| FR-093-AC-9 | Every node of the checked package of AC-7 has at least one occurrence: `a`'s parameter node has an `anchor` occurrence over `a: Boolean` and one `expression` occurrence per read, and the `Integer` and text scalar nodes that type P1's body literals have a `generated` occurrence. | Test (TC-416) |
| FR-093-AC-10 | With lock evidence that selects the text definition the Recursive text-leaf vectors name, `eq`, `has`, `eqa` and `eqo` of those vectors check with no refusal. Their `a = b`, `contains(s, b)`, `x = y` and `a = b` nodes key to E14, E15, E16 and E17 with those vectors' preimage bytes, whose leaves are the lists the vectors give, and the type, group and parameter nodes they name key to T13, T14, G16 to G21, S4, S5 and P10 to P16. Declaring `B` before `A` gives the same keys. | Test (TC-415) |
| FR-093-AC-11 | Structural equality over `record R { t?: Text[0, 64; nfc]; }` and over `record S { t: Option<Text[0, 64; nfc]>; }` each carries one leaf, path `field:t`, `inner`; over `record W { t: Text[0, 64; nfc]; }` one leaf, path `field:t`. Structural equality and `contains` over FR-092's recursive `List`, which reaches no `Text` type, carry no leaves. Structural equality over `record Tree2 { label: Text[0, 8; binary-utf8]; kids: Sequence<Tree2>[0, 3]; }` carries `field:label`, then `field:kids`, `inner`, `recursion:0`; over `record Two { x: Node; y: Node; }` it carries `field:x`, `field:label`; `field:x`, `field:next`, `inner`, `recursion:1`; `field:y`, `field:label`; `field:y`, `field:next`, `inner`, `recursion:1`. `eq` of the Recursive text-leaf vectors, lowered with lock evidence that supplies no text-profile definition, refuses with `missing_declaration`/`missing-selection` naming role `text_profile`, and yields no node; lowered with a node limit (`CheckingLimits`) that admits every node of its package but not also its two leaves, it refuses with `resource_exhausted`/`insufficient-next-charge` naming the node limit, and yields no node. | Test (TC-415) |
| FR-093-AC-12 | For every node of the checked package of AC-7, of the package of the recursive `f` and of a package holding `record Tree { kids: Sequence<Tree>[0, 3]; }`, the emitted `dependencies` equal the list that rules 1 to 5 of Node dependencies rebuild from the node as written. In ascending digest order, E1 lists P2 and P1; F2 `both` lists P2, P1 and E1; E2 lists L1, F2 and P1; T1, L1 and P1 list none. In the package of the recursive `f`, T4 `Int[0, 9]` lists T2, G4 lists G5 and P4, G5 lists L1, E11 and G6, and G6 lists G4 and E13. In the `Tree` package, G7 lists G9, G8 lists G7 and G9 lists G8. | Test (TC-416) |
| FR-093-AC-13 | For each application node of QSpec's `positive-operation-identities.json` and `positive-control-operations.json` whose `operation.identity` a row of the application table lowers, the node emitted for a function whose body holds that operation equals the fixture node in `node_tag`, `semantic_form`, `operator`, `operation.identity`, law roles in order, `mode`, member `kind` and `name`, leaf `path`s and modes, and argument term kinds and binding names, and IR's v2 reader admits the emitted package. The comparison reads none of the members Comparison with QSpec's v2 positive fixtures places outside it. | Test (TC-416) |
| FR-093-AC-14 | On a thread with a 2 MiB stack, at the default limits, a function body of 127 nested `a and (…)` checks and one of 128 refuses with `resource_exhausted`/`insufficient-next-charge` naming the depth limit. Each nested form TC-415 step 10 lists checks at the deepest nesting the default limits admit, or refuses there on its own unproved obligation, and refuses naming the depth limit one level deeper. Each of those forms nested 1,000 deep refuses naming the depth limit at the default limits, at the maximum depth with node, input-byte and work limits unlimited, and at a depth limit of 16. A postcondition `pre(…)` over 1,000 nested levels refuses. | Test (TC-415) |

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
- QSpec FR-322 `dependencies`: FR-322 gives the rule for an application
  node, a nominal node and (FR-340) a frame node. Rules 1 and 3 for every
  other node are QSL's proposal (ADR-013 QC-27). The published
  `positive-all-families.json` writes `dependencies: []` for the
  `expression`/`reference` node `eeee…` and the `correspondence` node
  `7070…`, whose bodies reference `dddd…`, and for the `bounded_domain`
  node `cccc…`; QSL asks QSpec to correct them.
- IR-242: the IR reader's recursion preimage. It derives an in-group
  ordinal from graph order, which the emission sets to FR-092's group
  order.
- The `Attribute` row's `record.project` over a `deref` result: FR-322 gives a
  model entity type the family `reference`, and no QSpec fixture projects an
  attribute of a dereferenced object. The row is QSL's proposal (ADR-013
  QC-24).
- [ADR-012](../decisions/ADR-012-semantic-family-extension-contracts.md)
  §5.1: no catch-all arm; §3 and §4.3: a postcondition, and the `Pre` read
  inside it, belong to the `ProtocolClause` family, which lowers them with
  #218 (designed in #223).
- QSpec FR-143: the recursive records whose text-leaf walk rule 3 ends.
- The `recursion:d` leaf segment and an optional field's `inner` segment:
  FR-322's `LeafSegment` admits `field:<name>`, `position:<n>` and `inner`,
  and names no leaf form for a recursive composite. Both are QSL proposals
  (ADR-013 QC-24). Until QSpec adopts QC-24, a v2 reader that validates
  against the published `LeafSegment` pattern refuses a package holding a
  recursion leaf, and a reader that derives an optional field's leaf
  without `inner` refuses its leaves.

## Status

Specified under QSL-208; the text-leaf walk and its recursion leaf
specified under QSL-212. Implemented on the QSL-156 slice
A4b branch, pending merge: `check` lowers each checked node in
`qsl-semantics/src/check/lowering.rs` and keys it by FR-092, and TC-415
backs AC-1 to AC-6, AC-8, AC-10, AC-11, AC-14 and CON-1 there. The
text-leaf walk (`text_leaves`) follows the Text leaves rules, charges each
leaf to the node limit before any leaf's law is read, and keys the Recursive
text-leaf vectors. A4b spells an integer literal as a decimal string and gives
`quire.op.quantity.convert` mode `rounding` = `exact`. The emission half is QSL-6 S1b: `qsl-package/src/emit.rs`
refuses every non-empty graph (`ProjectionNotYetImplemented`), so AC-7, AC-9,
AC-12, AC-13 and CON-2 (TC-416) are unbacked. AC-13's IR admission also
waits on IR reading the ADR-013 QC-24 and QC-25 node shapes. The `dependencies` rule and
the fixture comparison are specified under QSL-225. Ownership, decided here: QSL-156 A4b builds
the lowering and the keys in `check`; QSL-6 S1b serializes the lowered nodes
and does not lower. No FR-093 AC backs the `Pre` row; the `ProtocolClause`
postcondition lowering backs it. Remaining work: #218.
