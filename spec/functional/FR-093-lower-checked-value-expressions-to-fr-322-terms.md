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
  - target: ix://agent-ix/quire-specification/FR-440
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-322
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/NFR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-255
    type: traces_to
  - target: ix://agent-ix/quire-specification/FR-140
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-146
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-142
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-145
    type: depends_on
---
# FR-093: Lower checked Value expressions to FR-322 nodes with catalogued operations

## Description

QSpec FR-322 defines the checked-package/v2 node contract and application
identity rules (FR-322-AC-10, AC-17 and AC-18). `check` assigns node ids at E3
(ADR-011 §2.2, FB-13), so `check` lowers each checked expression into its
FR-322 node before it can identify it. This requirement fixes QSL's lowering:
which nodes a checked expression becomes, and the operator, operation
identity, laws, mode, member and arguments of each application.
[FR-092](FR-092-key-type-parameter-and-declared-nodes.md) fixes the type,
parameter and function nodes the lowering references and the structural
id of every node.

The source is the checked tree (`check::ir::Node`), not the parsed
`Expression`: each checked node carries its checked type, and FR-322 takes
every `result_type` and literal `type` from the checked type, never by
inference. Each `Expression` form checks to the `NodeKind` the table below
names.

QSpec references (`ix://agent-ix/quire-specification`, by reference): FR-322 "Operation families, groups and constraints", the closed
`proposals/checked-package-v2/operation-catalog.json` and the operator-class
semantic forms its qualification tests fix.

## Inputs

- A checked function declaration: its parameters, its checked body and
  measure (`check::ir::Node` trees), and its declared result type.
- The type, parameter and function nodes of
  [FR-092](FR-092-key-type-parameter-and-declared-nodes.md).
- The layer table (FR-110): each law's `DefinitionRef`, `{authority, identity}`,
  the QSpec definition prose's definition of its role.

## Outputs

- The checked semantic graph's `Value`-family nodes, each with its FR-322
  body and its key, and for each node the source occurrences it was lowered
  from.
- Or a typed refusal and no node for the declaration.

## Behavior

### One node per checked expression

When lowering source dimension declarations, the emitted semantic graph
preserves the checked base-versus-derived discriminator in each dimension
payload independently of its normalized terms. In particular, lowering
`dimension N = L / L;` retains a derived dimension payload (`base=false`)
with no terms; the corresponding base `L` payload remains `base=true`.

`check` SHALL lower each checked node to one FR-322 node, except a read of a
local slot (`NodeKind::Local`). An argument of an application is a
`reference` to its operand's node, or a `binding` where the operation's
operand is a binder. A `Local` read is a `reference` to the parameter node of
the binder that bound the slot (FR-092), and has no node of its own.

Nodes are content-addressed: two checked expressions that lower to the same
body, type and form are one node. Each source occurrence of a node is its own
occurrence entry, keyed (node id, role, ordinal) (ADR-013 O-07). Ordinals per
(node id, role) follow insertion order: the order `check` records them in, not
a sort by region. In a package of several source units the units' occurrences
follow FR-480's canonical unit order, an earlier unit's first, and insertion
order applies within each unit. Each binder's own site is recorded before that binder's
reads, even when its region starts at the same byte as its first read.
Content-addressing means two binders can share one parameter node: when they
do, the node's occurrences follow one sequence in lowering order, so a later
binder's own site is not necessarily ordinal 0 -- only the first-recorded
occurrence in the whole shared sequence is.

- an expression's region is an `expression` occurrence;
- a binder's site (a parameter's `name: T`, a `let` name, a query binder) is
  an `expression` occurrence of its parameter node (QSpec FR-341-AC-10),
  which carries no `declaration` (FR-092), and each read of it is also an
  `expression` occurrence, in the same ordinal sequence;
- a type form's region is a `type` occurrence of its type node;
- a declared record, tuple or enum has one `declaration` occurrence, at its
  declared name, so its node carries its `declaration` (FR-322);
- a node that no region denotes, such as the `Integer` and text scalar nodes
  that type the literals inside a type or parameter node's body, has one
  `generated` occurrence, so every node has at least one (FR-322). It is
  placed at the root of the least function body, measure, state clause or
  protocol attempt that reaches the node through the nodes it names, and an
  enum declaration reaches each of its member nodes; a node none of those
  reaches is placed at the least declared type name that reaches it. "Least"
  orders roots by kind (function body, measure, declared type name, state
  clause, protocol attempt), then by declared name in UTF-8 byte order, then
  by declaration index (a protocol attempt by its protocol's index, then its
  own); source order plays no part.

QSL lowers each checked application to the QSpec FR-322 catalog operation
selected by the application table below. QSpec FR-322 owns the emitted node
fields and reader rules; QSL selects `result_type` and literal type from the
checked type, using the type nodes of FR-092.

Every checked operation node `check` lowers SHALL expose its `NodeKey`, the
structural id of the FR-322 node it lowers to, through a public accessor of
the checked unit beside its `Location` accessor. A backend that names an
operation it does not translate (FR-560 `Operation(k)`) reads `k` from that
accessor; no other stage constructs the key (ADR-013 O-04). Two structurally
equal operations lower to one node and so share one key, and the `Location`
tells their sites apart.

### Nesting depth

Checking a function body walks its expressions to type them, to prove their
static definedness and to lower them. Every such walk, including the
definedness helpers `outcomes` (the facts a condition establishes),
`interval` (an integer expression's proved range), `stable_path` (a guarded
read's path) and `shape` (an argument's shape), keeps the expressions it has
still to finish on an explicit heap stack, so a body of any depth is checked
within the check stage's node, input-byte and work limits
([FR-258](FR-258-check-and-lower-expressions-at-any-depth.md), ADR-030
D-4.3). The walk that measures a declaration before typing (FR-062's input
bytes, node count and work) and the syntactic checks of a `pre(…)` operand
walk the parsed expression on a heap stack too. The dispatch bridge copies
each clause, and renames an inherited precondition into a redefinition's
parameters, on a heap stack as well.

Each walk closes a binder's scope when the binder's body is done, and walks
every operand. A `let`, `count`, `sum`, query or `fold` binder is out of scope
after its body, so a later sibling may bind the same name, and lowering
gives that sibling its enclosing binder's level. The definedness walk walks
both branches of an `if`.

### Non-application nodes

| Checked node | Node |
|---|---|
| `Literal` of a Boolean, integer or rational value (`true`, `false`, an integer literal, `rational(n, d)`) | `value` / `literal`; `semantic_type` its type node; body `literal{type, value_kind, value}` spelled as FR-092 fixes. An integer literal that is an operand of `quire.op.integer.add`, `sub`, `mul` or `negate` is no node: it is the same `literal{type, value_kind, value}` term, typed at the builtin `Integer` node, in that application's `arguments` (FR-357, FR-358, ADR-013 O-09), as is an integer literal operand of an integer `quire.op.integer.eq` or `ne` (FR-358 step 6); every other literal operand, including the integer literal of an ordering, the literal that an equality over a bounded integer peer wraps in a `numeric.narrow` application (the operand is that application, identified `graph_child` by FR-358 step 6), and the operands of a composite or non-integer equality, is a reference to its node |
| `Literal` of a String, UUID or Timestamp value (`"abc"`, `UUID("...")`, `Timestamp(n)`, as check resolves the two calls) | `value` / `literal`; `semantic_type` the node of the native `string`, `uuid` or `timestamp` scalar type; body `literal{type, value_kind, value}` with `value_kind` `string`, `uuid` or `timestamp` and `value` spelled as FR-092 fixes |
| `Literal` of a Text value (`Text("...")`, as check resolves the call) | `value` / `literal`; `semantic_type` the position's `Text[min, max; profile]` `text_bounds` type node; body `literal{type, value_kind, value}` with `value_kind` `text` and `value` the retained scalar sequence as a JSON string (a `text` literal at a type of family `text`, FR-322's literal typing rule) |
| `Literal` of a quantity value (`3 ms`) | `value` / `literal`; `semantic_type` its unit's `scalar_type`/`unit` node; body `literal{type, value_kind, value}` with `value_kind` `quantity` and `value` the canonical magnitude ("Quantity literals") |
| `Literal` of an enum member (`E::m`, or a bare `m` that FR-317 resolves to `E::m`) | the enum member's QSpec `value` / `enum_value` node (FR-092) |
| `Record{declaration, slots}` (`R { f: e, g: null }`) | `value` / `record_value`; `semantic_type` the record's node; body `aggregate` of one binding per slot in declaration order: `f` = `reference(e)` for a present slot, and `f` = `literal{type: the field's Option node, value_kind: "none", value: null}` for a `null` slot and for an omitted optional slot, which denote one value |
| `Tuple{declaration, arguments}` (`Q(a, b)`) | `value` / `tuple_value`; `semantic_type` the tuple's node; body `aggregate{[reference(a), reference(b)]}` |

### Quantity literals

For a bounded quantity operand with both finite endpoints over a nominal
unit, `check` SHALL lower `+`, `convert` and `<` using the existing
`quire.op.quantity.add`, `quire.op.quantity.convert` and
`quire.op.quantity.lt` operations and dimensional rules, preserving the
operand's FR-092 bounded type (QSpec FR-322-AC-384). Addition SHALL return
ordinary `Quantity<u>`: the input interval does not bound its sum. Conversion
to a bounded result SHALL check the exact converted magnitude against both
inclusive endpoints and refuse `rational_out_of_domain` at the result type
when outside. Comparison SHALL compare exact converted values without
erasing the operands' bounds; incompatible dimensions retain their existing
typed refusal. Carried-quantity restrictions are
unchanged. A bare nominal or compound unit at a position requiring a bound
continues to refuse `RequiresBound` with its exact underlying structural id.

`check` SHALL type a quantity literal `n u` in a value expression, such as the
argument of `f(3 ms)`, as `Quantity<u>` and lower it to one `value`/`literal`
node of `value_kind` `quantity`.

**Typing.** The literal's magnitude `n` is an `exact-value` (a `uint`, a
`decimal-literal`, `rational(n, d)` or `decimal(n, s)`; FR-393), so it always
denotes an exact rational. `check` measures and reduces it by FR-185's
measure, under the rational-bits ceiling NFR-011 applies to each quantity
literal `check` types (NFR-011 Scope). The unit name `u` resolves in the
unit namespace, as a `Quantity<u>` type form's name does (FR-091-AC-44): it
names a declared unit, never a dimension and never a compound unit. The
literal's checked type is `ValueType::Quantity(UnitId)` of that unit, the
type `Quantity<u>`, and its checked value is the reduced magnitude in that
unit.

A magnitude carries a sign only inside `rational(n, d)`, so a negative
quantity literal is written `rational(-3, 1) ms`, of value `"-3"`. A unary `-`
is never part of a literal: the literal `n u` (a magnitude followed by its unit)
is a primary expression that binds tighter than the prefix `-`, so `-3 ms`
parses as `-(3 ms)`, the one negation of the literal `3 ms`.

**Negation.** `check` SHALL admit the unary `-` over a quantity operand of a
non-affine unit `u` (QSpec FR-142 "Quantity negation", FR-142-AC-25). Its
checked type is `Quantity<u>`, the operand's own type, and its checked value is
the exact negation of the operand's value, so `-(3 ms)` is `-3` in `ms`, still
in `ms` and not in the root unit, and `-(0 ms)` is `0 ms`. The negation of a
negation is admitted to any depth. The operand is any quantity expression: a
literal, a parameter, a sum, a product or a quotient, and a compound-unit
operand is negated in its compound unit. It lowers by the `QuantityNegate` row
to one `unary` `quire.op.quantity.negate` node (QSpec FR-322-AC-119) whose
argument is `[ref(e)]`, whose `result_type` is the operand's unit node, and
which carries no law, mode, member or leaves. A negation rooted directly in a
nonzero quantity literal is a nonzero fact, as is any further unary-negation
chain rooted in that same literal (QSpec FR-146-AC-9; FR-094). Thus `d / -3
ms` and `d / -(-(3 ms))` have nonzero facts, while `d / -(0 ms)` does not. A
negation rooted in a parameter, call, `let` name, conversion, or other
nonliteral quantity expression is not a nonzero fact, even when a guard or
the expression's value happens to be nonzero.

A bounded quantity (FR-154) and a carried quantity (FR-497) are no operand of
the unary `-`: a bounded one refuses `ill_typed`/`operator-ineligible` and a
carried one `ill_typed`/`type-mismatch` (FR-497-AC-2), each with no charge and
no node.

A literal never changes type to fit its slot (FR-056-AC-41). A quantity
literal is admitted where its operand position takes `Quantity<u>` of the
same unit: a parameter of type `Quantity<u>`, a comparison or `+`/`-` whose
other operand has type `Quantity<u>`, and a product or quotient, whose result
unit `check` forms as for any quantity operands (FR-094). Every unit and
dimension rule of a quantity operand applies to it unchanged. Addition and
comparison require compatible dimensions and an explicit common-unit
conversion, and conversion never occurs implicitly (QSpec FR-142 Behavior),
so a quantity in another unit, of the same dimension or another, stands
where `Quantity<u>` is required only through `convert<Quantity<u>>(…)`
(QSpec FR-142), which lowers by the `ConvertScalar` rows to
`quire.op.quantity.convert` with mode `rounding` = `exact`. A quantity
divisor other than a nonzero quantity literal or a unary-negation chain rooted
directly in one is never proved nonzero (FR-094).

**Division by a literal or its negation.** `check` SHALL admit a quotient
whose divisor is a quantity literal of nonzero reduced magnitude or one or
more unary quantity negations applied directly to such a literal, and SHALL
record that literal and its exact negation chain as a nonzero fact that
discharges the division's nonzero obligation (QSpec FR-146-AC-9), whatever the
dividend. No other expression form proves a quantity divisor nonzero. The quotient follows QSpec
FR-142's division. Each operand is taken in the canonical root of its unit,
where a literal of magnitude `n` in unit `u` is `n` times the composed scale
of `u` to its root, so a magnitude that is a rational, a decimal or a
negative `rational(-3, 1)` composes its scale exactly. The quotient's
dimension is the dividend's dimension with the divisor's exponents
subtracted, zero exponents removed. Its unit is the compound unit over the
canonical root units with those exponents (FR-094), which is the dimensionless
unit when every exponent cancels, and its value is the exact quotient of the
two root values. So with `d: Quantity<ms>` and `a: Quantity<m>`, `d / 3 ms`
is dimensionless, with the value of `d` divided by three, and `a / 3 ms` has
the unit `m` per `s` and `1000/3` times the value of `a`. A negative literal
divisor negates the quotient. A divisor in another unit of the same
dimension composes its own scale: `d / 3 s` is `d`, taken in seconds,
divided by three, and is dimensionless as well. The division lowers by the
`Quantity(operator)` row to one `binary` `quire.op.quantity.div` node whose
arguments are `[ref(l), ref(r)]`, `r` the literal's node, and whose
`result_type` is the node of the quotient's compound unit (FR-094).

**Integer power.** `check` SHALL admit `e ^ n` (FR-091 "Integer power
spelling") over a quantity operand `e` of a non-affine unit and an exponent
`n` written as a signed integer literal (QSpec FR-142 "integer power",
FR-142-AC-6; TC-187 U12, U13, U21). The exponent is a literal because the
result unit depends on it. The result's dimension is the operand's normalized
base-dimension map with every exponent multiplied by `n`, zero exponents
removed, and its unit is the compound unit over the canonical root units with
those exponents (FR-094), the dimensionless unit when `n` is `0`. Each
exponent is an exact integer, whatever its size. The checked value is the
operand's value taken in its canonical root, as a product takes it, raised to
`n`: `x ^ 0` is exactly one for every `x`, zero included, and a zero base under
a negative exponent is undefined. So `2 m ^ 2 = 2 m * 2 m` is true, `d ^ 1`
over `d: Quantity<ms>` has the compound unit `s` to the power 1 and the value
of `d` in seconds, and `d ^ -1` has the unit `s` to the power -1. The power
lowers by the `QuantityPower` row to one `binary` `quire.op.quantity.pow` node
whose arguments are `[ref(e), ref(n)]`, `n` the `value`/`literal` node of the
exponent (`value_kind` `integer`, typed by the `Integer` scalar node), whose
`result_type` is the node of the result's compound unit, and which carries no
law, mode, member or leaves: QSpec FR-322's operand contract for the operation
(QSpec FR-322-AC-383, TC-187 QP-01 to QP-10), which IR's v2 reader applies.

- A negative exponent divides by the base, so it carries the nonzero
  obligation of a quantity divisor on `e` (QSpec FR-146): only a base that is
  a quantity literal of nonzero reduced magnitude discharges it (FR-094).
  Otherwise, a zero-magnitude literal base included, `check` SHALL refuse the
  power `undefined_expression`/`unproved-nonzero` at the `^`. An exponent of
  zero or above carries no obligation. Under `CheckMode::Kernel` (FR-495) the
  checker records no nonzero obligation for a power, and a zero base under a
  negative exponent evaluates to `Undefined::DivisionByZero`
  (`division-by-zero`) at the charge QSpec states (TC-187 U21).
- If `e` is a quantity of an affine unit, then `check` SHALL refuse the power
  `ill_typed`/`operator-ineligible` at the `^`, before any charge and emitting
  no node (QSpec FR-142 allows no arithmetic on an affine unit). If `e` is a
  carried quantity `Quantity<u, C>`, `check` SHALL refuse it
  `ill_typed`/`type-mismatch` at the expression (QSpec FR-142-AC-27, FR-497).
  If `e` is not a quantity, `check` SHALL refuse it `ill_typed`/`type-mismatch`
  at the operand: QSpec defines integer power over quantities only.
- The evaluator SHALL evaluate a `quire.op.quantity.pow` node through the
  owned quantity core's integer power (`quire_semantic_value`'s
  `QuantityOperation::Power`) with the call's meter, and keeps no QSL copy of
  it. Its charges are therefore QSpec FR-142's unit schedule of
  `quire.value.accounting/v1`, in that order: the operand identity reads, the
  undefined condition, the rational events, whose `integer_bits` amount is
  `abs(n)` times the operand's retained parts and is charged before the power
  is computed (U13), then the target domain and result retention. A denied
  charge ends the node `incomplete` with that charge's record and no value.

A quantity divisor is proved nonzero only when it is a quantity literal of
nonzero reduced magnitude, or is one or more unary quantity negations applied
directly to such a literal (QSpec FR-146-AC-9). The latter includes the direct
form `-3 ms` (parsed as `-(3 ms)`) and nested forms such as `-(-(3 ms))`;
negation preserves nonzero-ness exactly. Negating zero, including
`-(0 ms)`, provides no proof. A parameter or a `let` name remains unproved
whether negated or guarded; so do calls, conversions, products and quotients,
even when their value happens to be a nonzero literal. These forms refuse
`undefined_expression`/`unproved-nonzero` at the division. A quantity literal
of zero magnitude remains a legitimate value where it is not a divisor, such
as `d = 0 ms`.

**Dimension equivalence.** A dimension equivalence the unit declares
(`dimension A ~ B by x U = y V;`, FR-091) puts the units of the two dimensions in one
equivalence class and states the ratio of their canonical roots, and `check` applies
QSpec FR-142 "Dimension equivalence" "Effect" to every quantity operand. The identification
edge from one root to another carries that ratio, so a conversion across a class whose roots
differ in size (`1 beat` is `rational(1, 4) s`) scales the value by it, and a conversion
across equal roots leaves it unchanged. `convert<Quantity<u>>(e)` is admitted when
the dimension nodes of `e`'s unit and of `u` are equal or in one class, lowers
as any conversion does, and charges the class graph's edges as FR-142 states;
its value is the exact one. Addition, subtraction, equality and ordering still
require the identical unit, so a quantity of one unit of a class and a quantity
of another unit of that class refuse `ill_typed`/`type-mismatch` at the
operation or the argument, as `1 m` and `100 cm` do, and meet only through
`convert`. A product, quotient or integer power takes each operand in the
canonical root of its class, so the unit of `d / e` over `d: Quantity<s>` and
`e: Quantity<sec>` of two equivalent dimensions is the dimensionless unit with
the value of `d` in the class root over the value of `e` in the class root, and
the product of `s` and `sec` has the compound unit of the class root squared,
the one the product of `s` and `s` has. A conversion between units of two
dimension nodes that no equivalence relates, directly or through a class,
refuses `ill_typed` as before.

**Lowering.** The literal's node has as `semantic_type` the
`scalar_type`/`unit` node of its unit (FR-094: a declared unit's quantity
type is its unit node), and its body is `literal{type, value_kind, value}`
with `type` that same unit node, `value_kind` `quantity` and `value` the
reduced magnitude in QSpec FR-322's canonical spelling ("Scalar literal
values", FR-322-AC-79): the decimal integer when the denominator is 1,
otherwise `numerator/denominator` in lowest terms with a sign only on the
numerator. So `3 ms` has value `"3"`, `0.5 ms` and `rational(2, 4) ms` each
have value `"1/2"`, and `rational(1, 1000) s` has value `"1/1000"` at the `s`
node. The unit node's family is `quantity`, the family `value_kind`
`quantity` admits, so the literal satisfies the literal/type-family rule. A
`rational` literal typed at a unit node violates that rule, so `check` SHALL
NOT substitute a rational scalar type to emit a quantity literal. As for any
literal outside an integer application's arguments, an operand position
holds a reference to the literal's node. Literal nodes are
content-addressed, so two quantity literals with one unit and one reduced
magnitude are one node, and `3 ms` and `3 s` are two.

**Domain.** A quantity literal contributes no domain to a claim's extent,
because a literal is one value and not a domain (ADR-014 §4 extent rule). A
quantity domain comes only from a quantity-typed position, such as a
`Quantity<u>` parameter, which ADR-014 §4 and FR-097-AC-2 classify as one
unbounded domain that no `FiniteBound` ranges over. The Contract IR reader
returns `requires_bound` under a bounds-required profile only for a position
typed at a quantity; a package whose closure reaches a unit only as a
`literal.type` lowers (Contract IR FR-038-AC-73).

**Positions outside a value expression.** A quantity literal outside a value
expression is read by the requirement that owns its position, and lowers to
no FR-322 node here: a distribution's support value (FR-185), an
expected-reward threshold (FR-195), a quantile threshold or horizon
(FR-186), and a time declaration's period or bound and a timed interval's
bounds (FR-393, TC-1078). The probabilistic positions are checked by the same
magnitude and unit reading and recorded in their form. A support value or
reward is refused at S4 as FR-185 "Package emission" states; a claim's
threshold or horizon is in no emitted body (FR-498).

**Refusals.** `check` reads a quantity literal in this order: it measures
the magnitude against the ceiling, reduces it, resolves the unit name, then
types the literal's operand position. The first refusal reached is the one
reported, so `rational(1, 0) ghost` refuses for its zero denominator, not
for its unit name. In each case `check` emits no literal node, and a
ceiling, zero-denominator or unit-name refusal is located at the whole literal.

- If a quantity literal's written magnitude exceeds the rational-bits
  ceiling, then `check` SHALL refuse it
  `stage_limit_exceeded`/`rational-bits-exceeded`, naming `s3.rational_bits`
  and the configured ceiling (NFR-011).
- If a quantity literal's magnitude has a zero denominator
  (`rational(1, 0) ms`), then `check` SHALL refuse it
  `undefined_expression`/`unproved-nonzero`, FR-091's code for a zero
  denominator.
- If a quantity literal's unit name resolves to no declared unit, a
  dimension name included, then `check` SHALL refuse it
  `missing_declaration`/`missing-name`, naming the name.
- If a quantity literal stands where its operand position takes another unit
  or no quantity, then `check` SHALL refuse it `ill_typed`/`type-mismatch`,
  at the argument of a call and at the comparison or operation otherwise
  (QSpec FR-142).
- If the divisor of a quantity division is a quantity literal of zero
  magnitude, or a unary-negation chain rooted directly in one (`d / 0 ms`,
  `d / -(0 ms)`), then `check` SHALL refuse the division
  `undefined_expression`/`unproved-nonzero` at the division. A literal
  refusal above is reached first and is the one reported: `d / rational(1, 0)
  ms` refuses at the literal for its zero denominator.
- If the operand of a unary `-` is a quantity of an affine unit (a unit whose
  composed offset to its root is not zero), then `check` SHALL refuse the
  negation `ill_typed`/`operator-ineligible` at the negation, before any
  charge and emitting no node, because QSpec FR-142 allows no arithmetic on an
  affine unit (QSpec TC-187 U39).

A magnitude is always an exact rational, so no quantity literal is refused
for its magnitude's kind.

### Decimal constructors

A value-position `decimal(c, s)` (FR-091 "Decimal constructor") is one exact
decimal of coefficient `c` and scale `s`, kept as written: `decimal(100, 2)`
and `decimal(1, 0)` are the values 1.00 and 1, and neither is reduced.

`check` SHALL type the constructor by the decimal type its position gives it:
the declared result of a function gives one, and a literal operand of `+`, `-`,
`*` or `/` beside an operand of one named decimal type takes that type
(FR-104 "Infix decimal arithmetic"). A negated literal takes its operand's
decimal type from the position and is typed by it. A `let` binding gives no
type.

`check` SHALL decide a constructor in this order, and the first failing step
is the refusal:

1. A position that gives no type SHALL refuse `ill_typed`/`ambiguous-literal`
   (FR-104-AC-16), and a position that expects a type that is not a
   `Decimal[..]` SHALL refuse `ill_typed`/`type-mismatch`.
2. **Literal scale limit.** A written scale `s` above the S3 setting
   `s3.decimal_literal_scale`, default 65535 digits, SHALL refuse
   `stage_limit_exceeded`/`work-budget-exceeded` (limit kind work budget) at the
   literal, naming the bound, the actual `s` and the setting, before any charge
   and with no node.
3. **Literal scale total.** The written scales of the `decimal(c, s)`
   literals of one checked package, each occurrence counted and each counted when
   it passes step 2, SHALL total at most the S3 setting
   `s3.decimal_literal_scale_total`, default 1048576 digits. The literal
   whose scale takes the running total above it SHALL refuse
   `stage_limit_exceeded`/`work-budget-exceeded` at that literal, naming the
   bound, the running total including that literal and the setting, before any
   charge and with no node.
4. **Membership by value.** The literal SHALL be a member of its type by value
   (QSpec FR-140): its value normalized to scale `max(s', smin)`, where `s'` is
   the lowest scale at which it is exact, has no more than `smax` fractional
   digits and a coefficient within the bounds. A non-member SHALL refuse
   `ill_typed`/`type-mismatch` at the literal at type checking, before any
   charge (QSpec FR-140-AC-8). A constructor can be a member although its
   written coefficient or scale is outside the type's: `decimal(100, 2)` is a
   member of a scale-0 type, being the value 1. Membership applies to the source operand
   (QSpec FR-140): a negated literal's operand is the literal, so
   `-decimal(5, 0)` at `Decimal[-9, 0; 0, 0; toward-zero]` refuses
   `ill_typed`/`type-mismatch` at check because 5 is not a member, and
   `-decimal(-5, 0)` checks because -5 is, and then evaluates the negation in
   the one named type: a negated value outside the domain (5 here) is refused
   at evaluation `decimal_out_of_domain`/`outside-domain`.

The two limits exist because the literal's wire spelling has exactly `s`
fractional digits and a zero coefficient is a member of every `Decimal[..]`
type whatever its scale (step 4), so without them `decimal(0, 4294967295)`
would spell four billion digits and a package of many such literals would spell
far more than its source bytes. A limit is decided before membership, so
`decimal(1, 65536)` and `decimal(0, 65536)` refuse the limit and not
`type-mismatch`. Neither is FR-091's decimal-scale limit (`s3.decimal_scale`,
4096), which bounds the `10^s` an assembler forms for a unit declaration's
exact number; a value-position constructor forms no power of ten. The stop's
locus is in FR-096.

A checked literal SHALL keep its written scale as its retained
representation, and SHALL lower to one `value`/`literal` node of `value_kind`
`decimal` whose `type` and `semantic_type` are the type's node and whose
`value` is the coefficient with the decimal point placed `s` digits from the
right: exactly `s` fractional digits, no digit for `s` = 0, a leading `0` before
the point where the coefficient has no more digits than `s`, and a `-` before
a negative value (`decimal(125, 2)` is `"1.25"`, `decimal(5, 2)` `"0.05"`,
`decimal(10, 0)` `"10"`, `decimal(0, 2)` `"0.00"`, `decimal(-5, 1)` `"-0.5"`).
Two constructors of one value, type and scale SHALL be one node, and two
scales of one magnitude (`decimal(125, 2)`, `decimal(1250, 3)`) SHALL be two. A
negated literal SHALL lower to one `quire.op.decimal.negate` node over the
literal's node. The evaluator reads the retained coefficient and scale, so the
operand charges of QSpec FR-140 are those of the written representation, not
of the normalized value.

A divisor that is a `decimal(c, s)` of nonzero coefficient SHALL be a nonzero
fact that discharges the division's nonzero obligation at every scale
(`a / decimal(400, 2)`; QSpec FR-146). A zero coefficient SHALL NOT discharge
it (`decimal(0, 2)`), and no other divisor SHALL: a parameter, a `let` name, a
conditional of two nonzero literals, or a negated literal is not proved
nonzero, and each refuses `undefined_expression`/`unproved-nonzero` at the
division.

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
| `QuantityNegate` | `unary` | `quire.op.quantity.negate` | | `[ref(e)]` |
| `QuantityPower` (`e ^ n`) | `binary` | `quire.op.quantity.pow` | | `[ref(e), ref(n)]`, `n` the exponent's `Integer` literal node |
| `Ieee(operator)` of width `w` | `binary` | `quire.op.ieee.float32.*` or `quire.op.ieee.float64.*` | law `ieee_profile`; mode `rounding` = the operand types' mode | `[ref(l), ref(r)]` |
| The checked fused multiply-add `fma(a, b, c)` of width `w` (FR-397) | `call` | `quire.op.ieee.float32.fma` or `quire.op.ieee.float64.fma` | law `ieee_profile`; mode `rounding` = the operand types' mode | `[ref(a), ref(b), ref(c)]` |
| `IeeeSquareRoot(m, e)` | `call` | `quire.op.ieee.float32.sqrt` or `quire.op.ieee.float64.sqrt` | law `ieee_profile`; mode `rounding` = `m` | `[ref(e)]` |
| `IeeeNumericEquality`, `IeeeTotalOrder`, `IeeeBitIdentical` (`=` over floats, and the `numericEqual`, `totalOrder` and `bitIdentical` intrinsic calls) | `call`, as the operation catalog lists them | `quire.op.ieee.numeric_equal`, `quire.op.ieee.total_order`, `quire.op.ieee.bit_identical` | law `ieee_profile`; no mode; none is a requirement claim site | `[ref(l), ref(r)]` |
| `Order(operator, kind)` (`<`, `<=`, `>`, `>=`) | `binary` | `quire.op.<f>.lt`, `.le`, `.gt`, `.ge`, where `<f>` is `integer`, `rational`, `decimal`, `enum`, `text` or `quantity` for `kind` `Integers`, `Rationals`, `Decimals`, `Enums`, `Texts`, `Quantities` | for `text`: law `text_profile`, mode `text_profile` = the operands' profile | `[ref(l), ref(r)]` |
| `Equality(operator, schedule)` (`=`, `!=`) | `binary` | `quire.op.<f>.eq` or `.ne` for an operand family `<f>` of `boolean`, `integer`, `rational`, `decimal`, `text`, `enum`, `quantity`, `reference`, `string`, `uuid` or `timestamp`; `quire.op.structural.eq` or `.ne` for an `option`, record, tuple, union or collection operand | for `text`: law `text_profile`, mode `text_profile`; for `string`, `uuid` and `timestamp`: no law, no mode, no member and no leaves; for `structural`: leaves `operand:0`, the compared type's text-leaf list | `[ref(l), ref(r)]` |
| `Coerce(e, interval)` (an integer admitted into an `Int[..]` that does not contain its type) | `convert` | `quire.op.numeric.narrow` | member `type_argument{declaration: T(Int[interval])}` | `[ref(e)]` |
| `ConvertScalar(target, e)` whose operand type is an exact numeric type other than the target | `convert` | `quire.op.numeric.convert` | member `type_argument` of the target type node | `[ref(e)]` |
| `ConvertScalar(target, e)` whose operand is a quantity | `convert` | `quire.op.quantity.convert` | member `type_argument` of the target type node; mode `rounding` = `exact` for a `Quantity<u>` target, and the carried node's `rounding` for a `Quantity<u, C>` target (FR-497) | `[ref(e)]` |
| `ConvertText(target, e)` (`convert<T>(e)` whose operand has a `Text[..]` or `String` type, QSpec FR-478) | `convert` | `quire.op.text.to_<k>` for a `Text[..]` operand, `quire.op.string.to_<k>` for a `String` operand, where `<k>` is `integer` (`Integer`, `Int[..]`), `decimal` (`Decimal`, `Decimal[..]`), `rational` (`Rational`, `Rational[..]`), `float32`, `float64` or `quantity` (`Quantity<u>`) for the target | law `text_profile` for a `Text[..]` operand, then law `ieee_profile` for a `float32` or `float64` target; mode `rounding` = the target's mode for a decimal or IEEE target (`exact` when the target omits it) and `exact` for a quantity target, no mode for an integer or rational target; member `type_argument` of the target type node | `[ref(e)]` |
| `ConvertDecimal(e, target)` (`convert<T>(e)` to a decimal) | `convert` | `quire.op.numeric.convert` when exact, `quire.op.numeric.convert_rounding` when FR-149 classifies it as scale reduction | member `type_argument` of `target`'s node; for `convert_rounding`, mode `rounding` = `target`'s mode | `[ref(e)]` |
| `IeeeToRational(e, domain)` | `convert` | `quire.op.ieee.to_rational` | law `ieee_profile`; member `type_argument` of `domain`'s node | `[ref(e)]` |
| `ConvertCollection{target, operand}` | `convert` | `quire.op.collection.convert` | member `type_argument` of `target`'s node; leaves `result_inner` | `[ref(e)]` |
| `Field{declaration, name}` (`e.f`; the checked read of a record field carries the field's identity, FR-359 item 2, and a tuple read keeps its position) | `query` | `quire.op.record.project` | member `field{declaration: the record's node, name: f}` | `[ref(e)]` |
| `Present` | `present` | `quire.op.option.present` | | `[ref(e)]` |
| `Value` | `value` | `quire.op.option.value` | | `[ref(e)]` |
| `Call{function, arguments}` (`g(a, b)`) | `call` | `quire.op.function.call` | | `[ref(g's function node), ref(a), ref(b)]` |
| `Collection{collection_type, elements}` (`sequence[..]`, `set[..]`, `bag[..]`, `orderedSet[..]`) | `collection` | `quire.op.collection.sequence`, `.set`, `.bag`, `.ordered_set` | leaves `result_inner` for `set`, `bag`, `ordered_set` | `[ref(e0), ...]` in source order |
| `Query{visit: Map}` (`map`, `collect`) | `collection` | `quire.op.collection.map` | leaves `result_inner` | `[ref(c), binding{name: x, value: ref(body)}]` |
| `Query{visit: Filter}` | `collection` | `quire.op.collection.filter` | | `[ref(c), binding{name: x, value: ref(body)}]` |
| `Query{visit: Forall}`, `Query{visit: Exists}` | `quantify` | `quire.op.collection.forall`, `.exists` | | `[ref(c), binding{name: x, value: ref(body)}]` |
| `Query{visit: Count}` (`count<N>`) | `collection` | `quire.op.collection.count` | member `type_argument` of `T(node)` | `[ref(c), binding{name: x, value: ref(predicate)}]` |
| `Query{visit: Sum}` (`sum<N>`) | `collection` | `quire.op.collection.sum.<family>` for the numeric family of `N`: `integer` for `Integer` and `Int[..]`, `rational`, `decimal`, `float32`, `float64`, `quantity` for `Quantity<u>`, `carried_quantity` for a carried `Quantity<u, C>` | member `type_argument` of `T(node)`; mode `rounding` = `N`'s mode for `decimal`, `float32` and `float64`, `exact` for `quantity` and the carrier's mode for `carried_quantity` (QSpec FR-322 "Carried-quantity sum"), no mode otherwise; law `ieee_profile` for `float32` and `float64` | `[ref(c), binding{name: x, value: ref(summand)}]` |
| `Flatten` whose operand is not a `Query{visit: Map}` (`flatten(c)`) | `collection` | `quire.op.collection.flatten` | leaves `result_inner` | `[ref(c)]` |
| `Flatten` over a `Query{visit: Map}` (`flatMap(x in c: e)`, and `flatten(map(x in c: e))`) | `collection` | `quire.op.collection.flat_map`; no node is built for the inner map | leaves `result_inner` | `[ref(c), binding{name: x, value: ref(e)}]` |
| `Fold{identity: Some(i)}` (`fold<A>`) | `collection` | `quire.op.collection.fold` | member `type_argument` of `T(node)` | `[ref(c), binding{name: acc, value: aggregate{binding{name: x, value: ref(step)}}}, ref(i)]` |
| `Fold{identity: None}` (`reduce<A>`) | `collection` | `quire.op.collection.reduce` | member `type_argument` of `T(node)` | `[ref(c), binding{name: acc, value: aggregate{binding{name: x, value: ref(step)}}}]` |
| `Size` | `collection` | `quire.op.collection.size` | member `type_argument` of `T(node)` | `[ref(c)]` |
| `Contains` | `collection` | `quire.op.collection.contains` | leaves `inner:0` | `[ref(c), ref(v)]` |
| `DropFirst` (`dropFirst(c)`) over a sequence or an ordered set | `collection` | `quire.op.collection.sequence.drop_first` or `quire.op.collection.ordered_set.drop_first`, by `c`'s kind | | `[ref(c)]` |
| `RemoveOne` (`removeOne(c, v)`) | `collection` | `quire.op.collection.remove_one` | leaves `inner:0`, as `Contains` lists them | `[ref(c), ref(v)]` |
| `Attribute{reference, name}` (`deref(r).f`) | `deref`, then `query` | the node `quire.op.model.deref` over `[ref(r)]`, whose `result_type` is the model node of `r`'s object type `T`, and over it `quire.op.record.project` | the projection's member `field{declaration: T's model node, name: f}` | the projection's `[ref(deref node)]` |
| `Navigate{reference, relationship, role}` (`r.name` over a relationship end, FR-363) | `query` | `quire.op.model.navigate` | member `relationship_end{declaration: the relationship's model node, name: role}`; `result_type` is the type node of the checked `Reference<U>`, `Option<Reference<U>>`, `Set<Reference<U>>[l,u]` or `Bag<Reference<U>>[l,u]` | `[ref(r)]`, with no `deref` node |
| `AllInstances` | `query` | `quire.op.model.all_instances` | member `type_argument` of the queried type's model node | `[ref(p)]`, `p` a population parameter or, for a named population `M::P`, the population reference of FR-391 |
| `Lookup{absence}` | `query` | `quire.op.model.lookup` | mode `absence` = the authored mode; member `type_argument` of the queried type's model node | `[ref(p), ref(r)]`, `p` as for `AllInstances` |
| `Dispatch{receiver, arguments}` | `call` | `quire.op.model.dispatch_call` | member `operation{declaration: the model node of the receiver's static object type, name: member}` | `[ref(receiver), ref(a0), ...]` |
| `Pre` | `pre` | `quire.op.state.pre` | | `[ref(e)]` |

**One-occurrence collection forms.** `dropFirst(c)` and `removeOne(c, v)`
shrink a collection by exactly one occurrence (QSpec FR-145, FR-322-AC-117).
QSL types, lowers and evaluates them as QSpec states, with no QSL rule of its
own:

- `check` SHALL type `dropFirst(c)` over a `Sequence<T>[a, b]` or an
  `OrderedSet<T>[a, b]` as the same kind `K<T>[max(a-1,0), max(b-1,0)]`, an
  unbounded `b` staying unbounded, and SHALL refuse it over a `Set` or a `Bag`
  as `ill_typed`/`operator-ineligible` at the call, since neither has a first
  occurrence.
- `check` SHALL type `removeOne(c, v)` over any collection kind `K<T>[a, b]`
  as `K<T>[max(a-1,0), b]`. Its `v` is typed as the second operand of
  `contains` is: the element type `T` exactly, or a record conforming to a
  record `T`. Any other `v` SHALL be refused `ill_typed`/`type-mismatch` at
  `v`.
- An operand `c` that is not a collection SHALL be refused
  `ill_typed`/`type-mismatch` at `c`. A call with another argument count is
  refused by S1, whose production fixes the arity, as
  `invalid_syntax`/`unexpected-token`.
- Each lowers by its row above. The `result_type` is the node of the derived
  type, and a `drop_first` node carries no law, mode, member or leaves.
- The evaluator SHALL evaluate each node through the owned evaluation
  kernel's one-occurrence operation, with the call's meter, and keeps no QSL
  copy. `dropFirst` drops the first occurrence in order and is the empty
  collection for an empty `c`. `removeOne` drops exactly one occurrence equal
  to `v` under QSpec FR-144's membership relation (bit identity for a float
  leaf), the same relation `contains` uses: the first equal occurrence of a
  sequence, the equal member of a set or ordered set, and one multiplicity of
  a bag's equal member. It returns `c` unchanged when no member is equal. The
  charges are QSpec FR-145's: for `dropFirst`, the formation of the survivors
  only; for `removeOne`, the `contains(c, v)` scan up to and including the
  first equal member (over all members when none is equal), without that
  scan's scalar retain, then the formation of the result.

**Present options.** A checked `OptionSome(e)` SHALL lower as the
`value`/`option_value` form QSpec FR-322 defines. It SHALL NOT lower as an
application or function call. S4 SHALL use the checked option type and the
lowered node of `e`; QSpec FR-322 owns the carrier's body, identity,
dependencies and reader rules.

**Native scalar equality.** An `Equality` whose operands are both
`ValueType::String`, both `ValueType::Uuid` or both `ValueType::Timestamp`
lowers to `quire.op.string.eq`/`.ne`, `quire.op.uuid.eq`/`.ne` or
`quire.op.timestamp.eq`/`.ne` (QSpec FR-322 and its FR-322-AC-67): a `binary`
expression with result type `boolean`, arguments `[ref(l), ref(r)]`, and no
law, mode, member or leaves. A `string` operand never takes a `text_profile`
law or mode, and lowering never selects a `quire.op.text.*` identity for it.
`check` refuses a comparison of unlike operand types (`ill_typed`/`type-mismatch`)
or an ordering over these types (`ill_typed`/`operator-ineligible`) before
lowering (FR-056), so lowering sees only same-type pairs; a pair that
nevertheless reaches it refuses `ill_typed`/`operator-ineligible` at the
comparison, and no node is built.
These identities reference no text-profile definition.

A checked quantity type, `ValueType::Quantity(UnitId)`, has an exact
rational magnitude and no numeric domain, so the rounding mode it fixes is
`exact`. FR-322 takes a `rounding` mode from the types
(`type_pinned_modes` lists `quantity`), and QSpec FR-142 converts a quantity
into an exact target with no loss. `quire.op.quantity.convert` therefore
carries mode `rounding` = `exact` for a `Quantity<u>` target. A `Quantity<u, C>` target is the carried quantity of FR-497, whose conversion carries the carrier's rounding.

**Text to number.** `check` SHALL build a `convert<T>(e)` whose `e` has a
`Text[..]` or `String` type as a `ConvertText(T, e)` node of type `T` when `T`
is an integer, decimal, rational, `Float32`, `Float64` or `Quantity<u>` type
(QSpec FR-478). A target equal to `e`'s own type builds no node, as for
every `convert` (below). For any other target there is no conversion from
text, and `check` refuses it with `ill_typed`/`operator-ineligible` at the `convert`,
building no node. A `ConvertText` is never a `ConvertScalar`,
`ConvertDecimal`, `Coerce` or `IeeeToRational`: it lowers by its own row,
inside or outside an `Equality`.

It is a `convert`, so its `rounding` mode is fixed by the target type alone
(QSpec FR-322 `type_pinned_modes`): a decimal or IEEE target's declared mode,
`exact` for a quantity, and no mode for an integer or rational target. A
`Text[..]` operand gives its `text_profile` law, with the layer table's text-profile
definition as for every text identity, and its profile is read from the
operand's type node, not from a `text_profile` mode. A `String` operand takes
no `text_profile` law, as for `quire.op.string.eq`, and lowering selects a
`quire.op.string.to_*` identity for it, never a `quire.op.text.*` one. An IEEE
target adds the `ieee_profile` law, whose definition is FR-397's.
The operand's text-leaf list is unchanged. In a pure function body a
`ConvertText` carries no obligation (QSpec FR-146). Its undefined outcome for
text that is not a number, and its refusals for a number outside `T` or a
strict `exact` rounding, are FR-478's at evaluation.

A `Coerce` exists only for a narrowing: an integer whose type an `Int[..]`
contains is admitted with no node, and a `ConvertScalar` whose operand type
equals its target builds no node; its parent references the operand's node.

**Converted equality operand.** An `Equality` operand written `convert<T>(e)`,
whose `e` has an exact numeric or quantity type other than `T`, is checked as
a `ConvertScalar(T, e)` node of type `T`. It lowers by the `ConvertScalar`
rows: `quire.op.numeric.convert`, or `quire.op.quantity.convert` for a
quantity. An `Equality` operand written `convert<T>(e)` whose `e` has a
`Text[..]` or `String` type is checked as a `ConvertText(T, e)` node, by the
"Text to number" rule above, and lowers by the `ConvertText` row. Either way,
the equality's operand family `<f>` is that of the compared type `T`, and its
argument at that operand's position references the convert node, never `e`'s
node. A `convert<T>(e)` whose `e` already has type `T` builds no node. An
exact numeric conversion keeps the value, so a guard (`=`, `!=` or an
ordering) comparing an integer-typed chain of such conversions with an integer
literal bounds the chain's innermost integer operand as the comparison of that
operand itself does. A `ConvertText` reads a number out of text, so a guard over
it bounds no operand.

The model nodes, the `Reference<T>` and `Population<T>[N]` type nodes and
the model rows of AC-8 are
[FR-094](FR-094-key-model-owned-reference-population-and-quantity-nodes.md)'s.

A selected relationship declaration is an unconditional model-graph root,
including when no checked value, function, frame or relationship-end names it.
`check` records one `generated` occurrence for that relation node at the
selected model alias's declaration-site/source span. This rule is independent
of function presence and ordering. The fallback `generated_location` is never
used for this root; a relation node never reaches emission as
`UnlocatedOccurrence`.

A `binding` names its binder with the source name, and the binder's parameter
node (FR-092) has that name and the binder's level. For `fold` and `reduce`,
the outer binding names the accumulator and the inner binding the element.

A law's `definition` is the `DefinitionRef` `{authority, identity}` of the QSpec
definition prose for that role (FR-110's layer table; ADR-011 §2.4), with no
revision and no selection rows: for `text_profile` and
`ieee_profile`, the layer table's definition of that role. The `ieee_profile`
law is
[FR-397](FR-397-select-the-ieee-profile-of-a-package-that-declares-or-evaluates-a-float.md)'s.
`check` writes no law from a constant.

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
keeps the **open composites**: each declared record, tuple or union whose
fields, positions or member payloads it is walking, with the number of
segments the path had when the walk entered it. At a type `U` and a path `p`:

1. `Text[min, max; profile]`: append the text leaf
   `{path: p, laws: [the text_profile law], mode: {kind: "text_profile", value: profile}}`.
2. `Option<V>`, or `K<V>` or `K<V>[min, max]` for a collection kind `K`:
   walk `V` at `p` + `inner`.
3. A declared record, tuple or union `C` that is open, entered at `d` segments:
   append the **recursion leaf**
   `{path: p + "recursion:d", laws: [], mode: null}` when a `Text` type is
   reachable from `C`, and nothing otherwise. The walk does not enter `C`
   again.
4. A declared record `C` that is not open: open `C` at the length of `p`;
   for each field in declaration order, walk a required field `f: V`'s `V`
   at `p` + `field:f` and an optional field `f: V?`'s `V` at `p` + `field:f`
   + `inner`; then close `C`. A record, tuple or union `C` from which no
   `Text` type is reachable is not entered, here or in steps 5 and 6, and
   appends nothing.
5. A declared tuple `C` that is not open: open `C` at the length of `p`;
   walk each position `n`'s type, in position order, at `p` + `position:n`;
   then close `C`.
6. A declared union `C` that is not open: open `C` at the length of `p`;
   for each member `m` in member declaration order, walk each payload
   position `n`'s type in position order at `p` + `member:m` + `position:n`;
   then close `C`. A nullary member contributes nothing. This walks every
   declared member, independently of any value's active member.
7. Any other type: nothing.

A `Text` type is **reachable** from `C` when some finite sequence of steps
2, 4, 5 and 6, applied without regard to open composites, leads from `C` to a
`Text` type. A type alias walks as its resolved type (FR-091). `d` is
written as its canonical decimal string, as FR-322 writes `position:n`.

`member:m` contains the exact declared member identifier, with the same
identifier spelling as FR-440's union type binding. The payload index `n`
starts at zero and uses canonical decimal spelling. These are two distinct
path segments; the member is neither a record field nor a tuple position.
The path identifies a member relative to its enclosing union type; it
carries no union node key, `VariantId`, member ordinal or active-value tag.
Member identity remains (union identity, member identifier), as FR-319
requires. The type and its identity remain bound by the application's
operand/result types; the leaf list does not identify the whole type.
Two union types can therefore have the same text-leaf list.

An optional field's path passes through `inner` because the field's slot
holds a value of its `Option<V>` node: FR-092 types the field by that node,
and an omitted or `null` slot is a literal of it (Non-application nodes).
So `f: V?` and `f: Option<V>`, which hold one set of values, have one leaf
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
  entries only the finitely many `inner`, `field`, `member` and `position`
  segments of one field, position or member payload type, and ends at the
  first reentry. The walk runs over an explicit heap stack, one frame per
  entered type (FR-258).
  Because rule 3 ends each path at its first reentry, a recursive type's
  walk is as deep as its composites' nesting.
- **Lossless.** The expanded leaf set is computed from the list alone, so
  the list determines every text leaf of the type's unfolding, with its path
  and profile. Two types whose text leaves differ in any path or profile
  therefore have different lists.
- **Without recursion.** A walk that reaches no open composite appends no
  recursion leaf. Its text leaves use `inner` for an optional field and
  `member:<identifier>` for a union payload; these paths await a QSpec
  FR-322 extension (ADR-013 QC-24). A recursive composite that reaches no
  `Text` type, such as FR-092's `List`, adds no leaf.
- **Independent of keys and group order.** `d` counts path segments. It is
  not a FR-092 group ordinal, and the list names no node, so an application
  node's leaves do not depend on its operand types' recursion group order or
  keys, and they name no member of that group.

A recursion leaf carries no law: each text leaf it stands for carries its
own. A type from which a `Text` type is reachable has at least one text leaf
in its list, because the shortest walk from the type to a `Text` type
enters no composite twice.

Each appended leaf, text or recursion, costs one unit of the check stage's
node limit (`CheckingLimits`), because the list is bounded per path but not
in width: `n` records that each hold a text field and an optional field of
every other record give on the order of `(n - 1)!` leaves. The node limit
is finite by default, and each leaf's key bytes are charged to the work
budget ([NFR-011](../non-functional/NFR-011-bound-value-checking-work.md)). A walk that would
pass the limit stops with a stage limit of kind node count, reported as
`stage_limit_exceeded`/`node-count-exceeded` (FR-096), and yields no node. `check` completes the walk, or
stops on a limit.

These rules apply equally to union payloads reached from `operand:0`,
`inner:0` and `result_inner`: the selected type starts at the empty path,
so selecting a collection's element type adds no leading `inner`; walking
the collection type itself does. `result_inner` still yields no leaves for
a `sequence` result. A member's declared text profile governs its leaves,
including members not active in a compared value. The walk and reachability
computation retain FR-258's explicit heap stack and NFR-011's checking
ceilings, with no fixed depth cap. Each entered union is a composite work
charge, and leaf path work includes `member:m` bytes. At every stage-limit
or work charge, the lifecycle operation polls cancellation under
[FR-276](FR-276-cancel-a-lifecycle-operation.md); cancellation returns its
cause and no partial artifact.

For `record Node { label: Text[0, 8; binary-utf8]; next: Node?; }`, the list
is `field:label`, a text leaf with mode `binary-utf8`, then
`field:next`, `inner`, `recursion:0`. The expanded leaf set holds
`field:label`, `field:next`/`inner`/`field:label`, and so on down every
chain of `next` fields.

### Union text-leaf vectors

These vectors fix the leaf list independently of node identity.
The text definition is the Recursive text-leaf
vectors': `{authority: "agent-ix", identity: "quire.value.text.unicode-17.0.0/v1"}`.
In the table, `N(p)` is exactly
`{"path":p,"laws":[{"role":"text_profile","definition":{"authority":"agent-ix","identity":"quire.value.text.unicode-17.0.0/v1"}}],"mode":{"kind":"text_profile","value":"nfc"}}`;
`B(p)` is the same object with mode value `"binary-utf8"`; `R(p)` is exactly
`{"path":p,"laws":[],"mode":null}`. Every displayed path is an ordered JSON
array of literal UTF-8 segment strings, with no normalization, escaping of
the colon, inserted separator or omitted segment. Lists are in walk order.

| Vector | Declarations and walked type | Complete leaves, in order |
| --- | --- | --- |
| U1 | `union Empty { None, Done }`; `Empty` | `[]` |
| U2 | `union Choice { Right(Text[0, 8; binary-utf8], Text[0, 8; nfc]), Left(Text[0, 8; nfc]), Empty }`; `Choice` | `B(["member:Right","position:0"])`, `N(["member:Right","position:1"])`, `N(["member:Left","position:0"])` |
| U3 | `union Wrapped { Item(Option<Sequence<Text[0, 8; nfc]>[0, 2]>), Empty }`; `Option<Wrapped>` | `N(["inner","member:Item","position:0","inner","inner"])` |
| U4 | `union Chain { End(Text[0, 8; nfc]), Next(Option<Chain>) }`; `Chain` | `N(["member:End","position:0"])`, `R(["member:Next","position:0","inner","recursion:0"])` |
| U5 | U4's `Chain`; `Option<Chain>` | `N(["inner","member:End","position:0"])`, `R(["inner","member:Next","position:0","inner","recursion:1"])` |
| U6 | `union Silent { End, Next(Option<Silent>) }`; `Silent` | `[]` |
| U7 | `union Link { End(Text[0, 8; binary-utf8]), More(Cell) }`; `record Cell { tag: Text[0, 8; nfc]; next: Link?; }`; `Link` | `B(["member:End","position:0"])`, `N(["member:More","position:0","field:tag"])`, `R(["member:More","position:0","field:next","inner","recursion:0"])` |

For U2, renaming `Left` to `Other` changes only the corresponding path's
`member:Left` segment to `member:Other`. Declaring `Left` before `Right`
moves its leaf before both `Right` leaves; no lexicographic member sort is
performed. Swapping `Right`'s payload types exchanges the profiles at
`position:0` and `position:1`. Source `case` arm order changes none of
these lists. U4 expands to `End`'s text at every finite chain of `Next`
payloads with its `nfc` profile; U7 expands both profiles through every
finite `More`/`next` chain. Existing record/tuple, optional-field and E14
to E17 recursive vectors retain their paths, profiles and order.

### Who builds the lowering

`check` builds this lowering and FR-092's keys (#384), in the layer-3
`check` core, because E3 assigns node ids (ADR-011 SG-1, FB-13). The checked
semantic graph holds each lowered node: its tag, form, semantic type,
declaration, owner, body and structural id. The
M-4 v2 emission arm (layer-4 `package`) writes those nodes to the
wire: it adds each node's `dependencies`, `occurrences` and
`recursion_group` and the graph order, and it builds no body term and assigns
no node id of its own.

The emission SHALL write a node's `owner` member (QSpec FR-322 `owner`)
exactly when the node is a declared structural node, a model declaration
node or a clause function (QSpec FR-322 owner table): a
declared structural node carries its unit's `SourceOwner`
(`{kind: "source", authority, identity}`, FR-092), and a model declaration
node or clause function carries its `ModelOwner`
(`{kind: "model", identity, node}`, FR-094). The value is the checked
node's owner, copied and not rebuilt, and no part of the node's id. A
nominal node, an application node and every
undeclared node that is not model-owned carry no `owner`. The node's
`owner` is not carried in a separate projection; the wire carries no `identity_projection`. A reader derives no node id, and reads
no owner from the source map. The package's `sources` holds each emitted
`SourceOwner`'s (`authority`, `identity`), and each emitted `ModelOwner`'s
`identity` joins to a `model_root` node, the joins FR-322 checks.

A recursion group's members carry one `recursion_group`
label, the group label (FR-092), and the emission
writes them in the member order FR-322's `recursion` `{size, ordinal}` gives;
each member's `node_id` ordinal is the producer-assigned one (FR-322
"Structural node identity").

The emission writes `diagnostics.catalog` as QSpec's native diagnostics
catalog, a QSpec `DefinitionRef` naming QSpec's `native-diagnostics.md`:
authority `agent-ix` and identity `quire.native.diagnostics/v1`.

### Node dependencies

A node's `dependencies` list the nodes its definition is built from. A v2
reader follows them, beside `semantic_type` and the body's own references,
when it builds a node's closure and checks graph cycles, and it joins them
exactly against the body of an application node, the nominal joins of an
enum, dimension or unit node, and the entries of a frame (QSpec FR-322,
FR-340). The wire carries no `identity_projection` for them.

The v2 emission arm SHALL write each node's `dependencies` as the node ids,
unique and ascending by node id, that these rules name:

1. each `reference` target in the node's wire body, at any depth: an
   aggregate member, a binding's value or an application argument;
2. each declaration an application's `operation.member` names (every FR-322
   member kind that names a declaring node; QSL's rows use `field`,
   `operation` and `type_argument`);
3. for a `bounded_domain` node, its `semantic_type`, the type it bounds;
4. for an enum, dimension or unit node, the nodes FR-322's
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
`reference` naming that member's `node_id`. A group member is therefore a
dependency by rule 1, and by rule 3 when it is a `bounded_domain` node's
`semantic_type`. For the recursive `f` of FR-092, the application `f(x - 1)`
lists the group's member nodes it references, and `Sequence<Tree>[0, 3]` lists
its `Tree` group member.

Rules 1 and 2 are FR-322's rule for an application node: "the unique
reference targets and member declarations of its body, ascending by node id.
Rule 4 is FR-322's nominal join and rule 5 FR-340's frame join. For every
other node QSL applies rules 1 and 3, which QSpec's
`positive-operation-identities.json` follows at each of its 58 nodes (ADR-013
QC-27). No v2 reader joins the list of such a node, so for these nodes the list depends on
the writer's rule until QSpec states one.

| Node | `dependencies` |
|---|---|
| builtin `scalar_type` (`boolean`, `integer`, `rational`, `decimal`, `text`, `float32`, `float64`) | `[]`; the body is `aggregate{[]}` |
| `bounded_domain` (FR-092's ranges, `text_bounds`, `float_rounding`, `collection_bounds`; FR-094's `model_population`) | its `semantic_type` |
| `composite_type` `option`, a collection kind, `reference` | the node the body references |
| declared `composite_type` `record` or `tuple` | each field or position type node; an optional field's `Option` node |
| `value` `literal` or `parameter` | `[]`; the body holds literals only |
| `value` `record_value` or `tuple_value` | each node a present slot references |
| `scalar_type` `enum` (an enum declaration) | `[]`; it names no node |
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
fixture application key holds placeholder operand keys. The emission's
`edition`, law `definition`s, `node_id`s
in bodies and `dependencies`, `semantic_type`s,
`source_map` and `diagnostics` are therefore outside the
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

The vectors below name the nodes `check` mints for these declarations under
owner (`a`, `u`); the table gives each comparison's leaves:

- `record Node { label: Text[0, 8; binary-utf8]; next: Node?; }`, a group
  of two (G16, G17);
- `record A { name: Text[0, 8; binary-utf8]; b: B?; }` and
  `record B { tag: Text[0, 4; nfc]; a: A?; }`, a mutually recursive pair, a
  group of four (G18 to G21);
- `function eq using v(a: Node, b: Node): Boolean pure { a = b }` (E14);
- `function has using v(s: Sequence<Node>[0, 3], b: Node): Boolean pure { contains(s, b) }`
  (E15), whose `b` is E14's `b` (P11);
- `function eqa using v(x: A, y: A): Boolean pure { x = y }` (E16);
- `function eqo using v(a: Option<Node>, b: Option<Node>): Boolean pure { a = b }`
  (E17), whose recursion leaf's prefix is `inner`, one segment.

The text definition is QSpec's
`quire.value.text.unicode-17.0.0/v1`, the `DefinitionRef`
`{agent-ix, quire.value.text.unicode-17.0.0/v1}`; every text leaf carries it as its `text_profile` law. The leaves
are:

| Vector | Compared type | Leaves, in order |
|---|---|---|
| E14, E15 | `Node` | `field:label` (`binary-utf8`); `field:next`, `inner`, `recursion:0` |
| E16 | `A` | `field:name` (`binary-utf8`); `field:b`, `inner`, `field:tag` (`nfc`); `field:b`, `inner`, `field:a`, `inner`, `recursion:0` |
| E17 | `Option<Node>` | `inner`, `field:label` (`binary-utf8`); `inner`, `field:next`, `inner`, `recursion:1` |

T1 to T3 are FR-092's. The QSpec `structural-eq-record` and `collection-contains` operation vectors
fix the leaf spelling.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-093-CON-1 | The lowering `match` over `NodeKind` has one arm per variant and no `_` or catch-all arm (ADR-012 §5.1), so a new checked node kind does not compile until it has a lowering. | Design | Test (TC-415) |
| FR-093-CON-2 | Non-test code in the layer-4 `package` crate builds no node body term, assigns no node id and calls no node id constructor; it serializes the nodes `check` lowered with the ids `check` assigned. | Design | Test (TC-416) |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-093-AC-1 | For `function t using v(): Boolean pure { if true then true else true }`, the checked graph holds exactly one `value`/`literal` node for `true` and one `expression`/`conditional` node whose arguments are three `reference`s to that literal node, whose operation is `quire.op.control.if` and whose `result_type` and `semantic_type` are T1. The literal node has three `expression` occurrences, ordinals 0 to 2 in source order. | Test (TC-415) |
| FR-093-AC-2 | For `both`, `nb` (`function nb using v(a: Boolean): Boolean pure { both(a, true) }`) and `h` of FR-092-AC-4, the nodes of `a and b`, `both(a, true)` and `let y = a in y` are three distinct `expression` nodes, each with the operation, members and arguments its row gives. No node is built for a read of `a`, `b` or `y`: each is a `reference` to that parameter's node. | Test (TC-415) |
| FR-093-AC-3 | For each row of the application table whose checked node the `Value` family builds in a function body (every row except `Attribute`, `Navigate`, `AllInstances`, `Lookup`, `Dispatch` and `Pre`), a fixture function whose body holds that checked node lowers to one `expression` node with the row's `operator`, `semantic_form`, `operation.identity`, member, mode, laws, leaves and argument shape, and its `result_type` is the type node of the checked node's `value_type`. | Test (TC-415) |
| FR-093-AC-4 | `function c1 using v(x: Int[0, 9]): Int[0, 10] pure { x }` builds no convert node: `c1`'s `body` binding references `x`'s parameter node. `function c2 using v(): Int[0, 9] pure { 3 }` lowers its body to a `convert` node with `quire.op.numeric.narrow` and member `type_argument` naming `Int[0, 9]`'s node, over the literal `3`. `function c3 using v(x: Int[0, 9]): Rational[0, 9; 1, 1] pure { convert<Rational[0, 9; 1, 1]>(x) }` lowers to `quire.op.numeric.convert`. `function fm using v(s: Sequence<Sequence<Int[0, 9]>[0, 2]>[0, 3]): Sequence<Int[0, 9]>[0, 6] pure { flatMap(x in s: x) }` lowers its body to one `quire.op.collection.flat_map` node and no map node. | Test (TC-415) |
| FR-093-AC-5 | In `function q using v(s: Sequence<Int[0, 9]>[0, 5]): Boolean pure { forall(x in s: exists(y in s: x = y)) and exists(z in s: true) }`, `x`'s parameter node has level 1, `y`'s level 2 and `z`'s level 1. The `forall` argument list is `[reference(s's node), binding{name: "x", value: reference(the exists node)}]`. | Test (TC-415) |
| FR-093-AC-6 | A text equality carries law `text_profile` with the layer table's text-profile `DefinitionRef` and mode `text_profile` equal to the operands' profile. | Test (TC-415) |
| FR-093-AC-7 | **Withdrawn.** The rebuilt-preimage-equals-key oracle and the FR-092 vector keys G4 to G6 were superseded by QSpec FR-322 structural node identity, under which a reader derives no node id; the package crate's non-test code names no `SemanticTerm` constructor and no node id function (FR-093-CON-2), and for the recursive `f` of FR-092 the three members carry one `recursion_group` label and distinct producer-assigned `node_id`s. | Withdrawn |
| FR-093-AC-8 | In functions over a `Population<M::Order>[3]` parameter `p` and a `Reference<M::Order>` parameter `r` (FR-094-AC-3 and AC-4), and in a clause function (FR-094) that holds a dispatched call (QSpec FR-151, QSpec TC-196 D06), a checked `Attribute`, `AllInstances`, `Lookup` and `Dispatch` node each lowers to the operation, member, mode and arguments its row gives, and a `Lookup` with `absent empty` carries mode `absence` = `empty`. | Test (TC-415) |
| FR-093-AC-9 | Every node of the checked package of AC-7 has at least one occurrence: `a`'s parameter node has an `expression` occurrence over `a: Boolean` (QSpec FR-341-AC-10) and one `expression` occurrence per read, and the `Integer` and text scalar nodes that type P1's body literals have a `generated` occurrence. | Test (TC-416) |
| FR-093-AC-10 | Withdrawn: the AC asserted node keys that were digests of preimages; those keys are removed with the digest ceremony. The leaf lists it gave are covered by FR-093-AC-11. | Withdrawn |
| FR-093-AC-11 | Structural equality over `record R { t: Text[0, 64; nfc]?; }` and over `record S { t: Option<Text[0, 64; nfc]>; }` each carries one leaf, path `field:t`, `inner`; over `record W { t: Text[0, 64; nfc]; }` one leaf, path `field:t`. Structural equality and `contains` over FR-092's recursive `List`, which reaches no `Text` type, carry no leaves. Structural equality over `record Tree2 { label: Text[0, 8; binary-utf8]; kids: Sequence<Tree2>[0, 3]; }` carries `field:label`, then `field:kids`, `inner`, `recursion:0`; over `record Two { x: Node; y: Node; }` it carries `field:x`, `field:label`; `field:x`, `field:next`, `inner`, `recursion:1`; `field:y`, `field:label`; `field:y`, `field:next`, `inner`, `recursion:1`. `eq` of the Recursive text-leaf vectors, lowered with a node limit (`CheckingLimits`) that admits every node of its package but not also its two leaves, it stops with `stage_limit_exceeded`/`node-count-exceeded` at the node limit's bound, and yields no node. | Test (TC-415) |
| FR-093-AC-12 | For every node of the checked package of AC-7, of the package of the recursive `f` and of a package holding `record Tree { kids: Sequence<Tree>[0, 3]; }`, the emitted `dependencies` equal the list that rules 1 to 5 of Node dependencies rebuild from the node as written. In ascending node id order, E1 lists P2 and P1; F2 `both` lists P2, P1 and E1; E2 lists L1, F2 and P1; T1, L1 and P1 list none. In the package of the recursive `f`, T4 `Int[0, 9]` lists T2, G4 lists G5 and P4, G5 lists L1, G6 and E11, and G6 lists E13 and G4. In the `Tree` package, G7 lists G9, G8 lists G7 and G9 lists G8. | Test (TC-416) |
| FR-093-AC-13 | For each application node of QSpec's `positive-operation-identities.json` and `positive-control-operations.json` whose `operation.identity` a row of the application table lowers, the node emitted for a function whose body holds that operation equals the fixture node in `node_tag`, `semantic_form`, `operator`, `operation.identity`, law roles in order, `mode`, member `kind` and `name`, leaf `path`s and modes, and argument term kinds and binding names, and IR's v2 reader admits the emitted package. The comparison reads none of the members Comparison with QSpec's v2 positive fixtures places outside it. The rows AC-3 excludes (`Attribute`, `Navigate`, `AllInstances`, `Lookup`, `Dispatch`, `Pre`) are outside the comparison. | Test (TC-416) |
| FR-093-AC-14 | On a thread with a 512 KiB stack, at the default limits, a function body of the longest nested `a and (…)` chain the default S1 limits admit checks with no outcome naming a depth. Each nested form TC-415 step 10 lists, nested 1,000 deep with S1 and S3 limits raised to fit it, checks or refuses on its own unproved obligation, the same way it does nested 2 deep. A postcondition `pre(…)` over 1,000 nested `a and (…)` refuses as a forbidden pre-read. | Test (TC-415) |
| FR-093-AC-15 | With the alias `Total = Integer` and parameters `x: Int[0, 9]` and `s: Sequence<Int[0, 9]>[0, 5]`, reading `v` after `let v = x in v`, `count<Total>(v in s: v < 5)`, `sum<Total>(v in s: v)`, `forall(v in s: v < 5)` or `fold<Total>(acc, v in s: acc + v, identity: 0)` refuses with `missing_declaration`/`missing-name` naming `v`, and reading `acc` after that `fold` refuses naming `acc`. Each of those forms beside a copy of itself that binds the same name checks. After `f`'s four parameters, `let v = x in ((let w = x in w) + (let u = x in v + u))` lowers `v` at level 4 and both `w` and `u` at level 5, and `v + u` reads `v`'s and `u`'s parameter nodes. `if a then value(o) else 0` and `if a then 0 else value(o)`, over `o: Option<Int[0, 9]>`, each refuse with `undefined_expression`/`unproved-presence` alone. `if present(o) then value(o) else 0` checks. | Test (TC-415) |
| FR-093-AC-16 | The v2 emission arm writes only direct imports from `CheckedPackage::dependency_selections` as FR-322 `dependency_selections`: one `{identity}` entry per directly imported library identity (no `version`: a selection binds by identity, QSpec FR-322-AC-35), in ascending UTF-8 byte order of `identity` (`test/\u{FF61}` before `test/\u{1F600}`, the reverse of UTF-16 order). A package linked with two imports reads back Verified through QSL's I2 read with exactly those entries in both members. QSL's I2 read over QSpec's `dependency-selection-vectors.json` refuses each authored entry mutation at the mutated entry, and gives each order vector its recorded outcome at its last recorded locus, since IR's refusal carries one path (`make conformance`). | Test (TC-416) |
| FR-093-AC-17 | The v2 emission of a checked package holding `t` writes `diagnostics.catalog` with authority `agent-ix` and identity `quire.native.diagnostics/v1`, and no other member, and IR's v2 reader admits the package. | Test (TC-416) |
| FR-093-AC-18 | For `ordered enum Status { READY, DONE }` read from source text, with no member literal written: when the enum's only uses are the parameter types of `before using v(a: Status, b: Status): Boolean pure { a < b }`, or of the same function with `a = b`, each member node has exactly one occurrence, a `generated` one whose source-map region is that function's body text, and the package emits with nothing omitted. When no function names the enum, each member's region is the declared name `Status`, and for `record P { x: Int[0, 9]; }` no function names, the `Int[0, 9]` node's region is `P`, and stays `P` with `record Q { y: Int[0, 9]; }` declared before it. Each of those packages reads back Verified through QSL's I2 read. With `record R { x: Int[0, 9]; }` beside the post clause `VersionUnchanged` over `1 < 2`, which a protocol attempt names, the `Integer` node's only occurrence is `generated`, with region `1 < 2`: a type name never displaces a state clause or attempt placement. | Test (TC-416) |
| FR-093-AC-19 | Every IR checked-package/v2 node kind (`node_tag`, `semantic_form`) is classified exactly once: as a family QSL's lowering writes, or as a kind QSL's lowering never writes, with the reason. For each family QSL writes, a package holding a node of it is admitted by IR's checked-package/v2 reader, through QSL's I2 read, as emitted. | Test (TC-416) |
| FR-093-AC-20 | The v2 emission of a checked package holding `t` writes the package's `edition` as the layer table's definition of that role, exactly `{authority, identity}` with no revision. The emission of a `Float64` addition writes the `definition` of the operation's `ieee_profile` law as that same exactly-two-member row. IR's v2 reader admits both packages. | Test (TC-416) |
| FR-093-AC-21 | The v2 emission writes `owner` on exactly the declared structural nodes (`SourceOwner`) and the model declaration nodes and clause functions (`ModelOwner`), equal to the checked node's owner, and the wire carries no `identity_projection`. Checked under owner (`a`, `u`): `record P { x: Int[0, 9]; }`, the recursive `record Tree { kids: Sequence<Tree>[0, 3]; }` and the declared function over a `Reference<M::Order>` parameter (FR-094-AC-1) carry the `SourceOwner` (`a`, `u`); the model declaration nodes of `Order` and `Sub` (FR-094-AC-1) carry their `ModelOwner`s (`ix://acme/orders/Order`, `ix://acme/orders/Sub`); the clause functions of FR-094-AC-5, built by `checked_dispatch_operation` with a test-built clause table, carry their `ModelOwner`s; and the `Int[0, 9]`, `Sequence<Tree>`, `collection_bounds`, the `Reference<Order>` type node, parameter and expression nodes, and the declaration and member nodes of `ordered enum Status { READY, DONE }`, carry none. IR's v2 reader admits each package. | Test (TC-416) |
| FR-093-AC-22 | The same source, `record Point { x: Integer; }` and `record List { next?: List; }`, checked and emitted under the owners (`agent-ix`, `example-a`) and (`agent-ix`, `example-b`), gives two packages whose `Point` node id is `{package, path: ["Point"], ordinal: 0}` in both, whose `List` group labels and `List` group member ids are equal and whose `Integer` node id is equal; the owner enters only the declared nodes' `owner` member (AC-21), not a node id. IR's v2 reader admits both. QSpec's `positive-two-owners-a.json` and `positive-two-owners-b.json` (QSpec FR-322-AC-53) differ in `package` as well as owner, which is why their `Point` and `List` ids differ. | Test (TC-416) |
| FR-093-AC-23 | Structural equality and inequality over each Union text-leaf vector U1 to U7 carry exactly that vector's complete ordered leaf list, with the selected text definition and stated profiles. U2's rename, member reorder, payload swap and source arm reorder give exactly the changes the vectors state. | Test |
| FR-093-AC-24 | For each `K` of `Set`, `Bag` and `OrderedSet`, structural equality over `K<Choice>` (U2) carries U2's leaves with one leading `inner` on every path; `contains` over that collection carries U2's leaves without that prefix (`inner:0`). A catalogued `result_inner` operation yielding `K<Choice>` carries U2's leaves without that prefix, while one yielding `Sequence<Choice>` carries `[]`. Structural equality over `Sequence<Choice>` still carries U2's leaves with one leading `inner`. | Test |
| FR-093-AC-25 | When a selected node or work ceiling is exhausted by these walks, checking stops with `stage_limit_exceeded` and the corresponding `node-count-exceeded` or `work-budget-exceeded` cause before resolving leaf laws, yielding no node; every appended text or recursion leaf costs one node, every entered union costs composite work, and every leaf's full path bytes count toward work. | Test |
| FR-093-AC-26 | Cancelling a lifecycle `check` during a Union text-leaf walk stops at the next charge with `StageFailure::Cancelled` retaining the requested cause and no output artifact (FR-276). Existing AC-10 and AC-11 Record/Tuple and optional-inner vectors remain byte-identical. | Test |
| FR-093-AC-27 | For a selected model alias whose package declares an unreferenced relationship and has no function or frame naming it, `check` emits the `relation`/`relationship` node and records exactly one `generated` occurrence at that alias's declaration-site/source span. It never uses the fallback `generated_location` or emits an `UnlocatedOccurrence`. | Test (TC-417) |
| FR-093-AC-28 | `function e1 using v(a: String, b: String): Boolean pure { a = b }`, the same over `UUID` and over `Timestamp`, and each with `!=`, lower to one `expression` node whose operation is `quire.op.string.eq`, `.ne`, `quire.op.uuid.eq`, `.ne`, `quire.op.timestamp.eq` or `.ne`, result type `boolean`, arguments `[ref(a's node), ref(b's node)]`, and no law, no mode, no member and no leaves. The same functions with a `Text[0, 8; nfc]` operand against a `String`, or `String` against `UUID`, refuse `ill_typed`/`type-mismatch` and build no node for the declaration. | Test (TC-1028) |
| FR-093-AC-29 | `function s1 using v(): String pure { "a\u00e9" }`, `function u1 using v(): UUID pure { UUID("00112233-4455-6677-8899-aabbccddeeff") }` and `function t1 using v(): Timestamp pure { Timestamp(-5) }` each lower the body to one `value`/`literal` node whose `semantic_type` and body `type` are the `string`, `uuid` or `timestamp` type node, with `value_kind` `string`, `uuid` or `timestamp` and `value` `"a\u00e9"` as JSON, `"00112233-4455-6677-8899-aabbccddeeff"` and `"-5"`. `Timestamp(0)` and `Timestamp(-0)` lower to one shared node of value `"0"`; two `"abc"` in one unit lower to one shared node. In `function e2 using v(a: String): Boolean pure { a = "abc" }` the comparison is one `quire.op.string.eq` node with arguments `[ref(a's node), ref(the literal's node)]`, no law, mode, member or leaves; the `uuid` and `timestamp` counterparts use `quire.op.uuid.eq` and `quire.op.timestamp.eq`. | Test (TC-1052) |
| FR-093-AC-30 | Over `x: Int[0, 9]`, the bodies `x + 1`, `1 + x`, `x - 1`, `x * 2` and `-5`, and over `x: Integer` the bodies `x == 1` and `1 != x`, each lower to one application whose integer literal operand is a `literal{type, value_kind: integer, value}` term in the application's `arguments`, typed at the builtin `Integer` node, and the checked graph holds no node for that literal. In `2 * (x + 1)` each literal is inline in its own application and the inner application is a reference in the outer. Over `x: Int[0, 9]`, `x == 1` compares `x` with a `numeric.narrow` application of the literal's node, which FR-358 step 6 identifies as `graph_child`. `x > 0` and `a == true` (with `a: Boolean`) keep a reference to their literal node, and a function whose body is the literal `1` holds the `value`/`literal` node L6. | Test (TC-415) |
| FR-093-AC-31 | `function f using v(a: Int[-100,100], b: Decimal[-100,100;0,0;nearest-even]): Boolean pure { convert<Decimal[-100,100;0,0;nearest-even]>(a) = b }` lowers to one `quire.op.numeric.convert` node over `a`'s parameter node, whose `result_type` is `b`'s type node, and one `quire.op.decimal.eq` whose arguments are `[ref(convert), ref(b)]`. `!=` gives `quire.op.decimal.ne`, and `b = convert<..>(a)` puts the convert node second. `convert<Decimal[..]>(a) = convert<Decimal[..]>(c)` builds one convert per operand. A conversion between Rational, Decimal and Int types in either direction, and a Quantity conversion `convert<Km>(q) = r` (`quire.op.quantity.convert` with mode `rounding` = `exact`, then `quire.op.quantity.eq`), each lowers a convert node and the equality of the converted type. `a = b` over two `Int[-100,100]` parameters builds no convert node and gives `quire.op.integer.eq` over the two parameters. IR's v2 reader admits each package, and evaluating each equality gives the result of comparing the converted values. | Test (TC-415) |
| FR-093-AC-32 | `function g using v(x: Int[-5,5]): Int[0,3] pure { if convert<Integer>(x) = 2 then x else 0 }`, the same with `convert<Int[-10,10]>(x)`, `if convert<Integer>(x) != 2 then 0 else x`, `if convert<Int[-5,5]>(convert<Decimal[-5,5;0,0;nearest-even]>(x)) = 2 then x else 0` and `if convert<Integer>(x) < 4 then (if convert<Integer>(x) > -1 then x else 0) else 0` each check, as the same guards over `x` itself do: the guarded branch reads `x` within the interval the comparison states, `=`, `!=` and the orderings alike. | Test (TC-415) |
| FR-093-AC-33 | With FR-091-AC-44's units (`dimension Time;`, `unit s : Time = rational(1, 1);`, `unit ms : Time = rational(1, 1000) * s;`) and `function f using v(d: Quantity<ms>): Boolean pure { true }`, the body of `function h using v(): Boolean pure { f(3 ms) }` lowers the argument `3 ms` to one `value`/`literal` node whose `semantic_type` and body `type` are the `ms` unit node, with `value_kind` `quantity` and `value` `"3"`, and the call's argument references that node. In one unit beside `h`, `function h2 using v(): Boolean pure { f(0.5 ms) }` and `function h3 using v(): Boolean pure { f(rational(2, 4) ms) }` lower their arguments to one shared literal node of value `"1/2"`, and `function h4 using v(): Boolean pure { f(3 ms) }` references `h`'s `3 ms` node; `function n using v(): Boolean pure { f(rational(-3, 1) ms) }` lowers a literal of value `"-3"`; `function k using v(d: Quantity<s>): Boolean pure { d = rational(1, 1000) s }` lowers `quire.op.quantity.eq` with arguments `[ref(d's node), ref(the literal's node)]`, the literal of value `"1/1000"` at the `s` node; and `function c using v(d: Quantity<ms>): Boolean pure { convert<Quantity<s>>(d) = 3 s }` lowers `quire.op.quantity.convert` (mode `rounding` = `exact`) over `d`, then `quire.op.quantity.eq` over the convert node and the `3 s` literal. No literal of `value_kind` `rational` is emitted for any of them, and IR's v2 reader admits each package. | Test (TC-483) |
| FR-093-AC-34 | Under AC-33's units and `f`, each of these refuses and emits no literal node: `f(3 s)`, `ill_typed`/`type-mismatch` at the argument; `function e using v(d: Quantity<ms>): Boolean pure { d < 3 s }`, `ill_typed`/`type-mismatch` at the comparison; `function g2 using v(x: Integer): Boolean pure { x = 3 ms }`, `ill_typed`/`type-mismatch` at the comparison; `f(rational(1, 0) ghost)`, `undefined_expression`/`unproved-nonzero`, not `missing-name`; `f(3 ghost)` and `f(3 Time)`, `missing_declaration`/`missing-name` naming `ghost` and `Time`; `f(rational(1, 0) ms)`, `undefined_expression`/`unproved-nonzero`; `function q using v(d: Quantity<ms>, e: Quantity<ms>): Boolean pure { d / e = e / d }`, `undefined_expression`/`unproved-nonzero` at a division because its divisor `e` is a quantity parameter, not a nonzero literal (FR-094); and, with the rational-bits ceiling set to `16`, `f(rational(1, 100000) ms)`, `stage_limit_exceeded`/`rational-bits-exceeded` naming `s3.rational_bits` and `16`. | Test (TC-483) |
| FR-093-AC-35 | Over `t: Text[0, 8; nfc]`, each of `convert<Int[0, 99]>(t)`, `convert<Decimal[0, 1000; 0, 2; nearest-even]>(t)`, `convert<Rational[-9, 9; 1, 9]>(t)`, `convert<Float32[toward-zero]>(t)`, `convert<Float64>(t)` and, with FR-091-AC-44's units, `convert<Quantity<ms>>(t)` as a pure function body lowers to one `convert` node over `[ref(t)]` with member `type_argument` naming the target type node. The identities are `quire.op.text.to_integer`, `to_decimal`, `to_rational`, `to_float32`, `to_float64` and `to_quantity`. Each carries law `text_profile` with the layer table's `quire.value.text.unicode-17.0.0/v1` definition; `to_float32` and `to_float64` also carry law `ieee_profile`. The modes are: none for `to_integer` and `to_rational`, `{"kind":"rounding","value":"nearest-even"}` for `to_decimal`, `toward-zero` for `to_float32`, `exact` for `to_float64` and `exact` for `to_quantity`. No node carries a `text_profile` mode. Over `s: String` the same six bodies lower to `quire.op.string.to_integer` to `quire.op.string.to_quantity` with the same members and modes, the `ieee_profile` law for the two IEEE targets and no `text_profile` law. No function carries a body obligation. | Test (TC-1196) |
| FR-093-AC-36 | With FR-091-AC-44's units and `dimension Length;`, `unit m : Length = rational(1, 1);`, `dimension Velocity = Length / Time;` and `unit mps : Velocity = rational(1, 1);`, each of these checks and lowers: `function p1 using v(d: Quantity<ms>, e: Quantity<ms>): Boolean pure { d / 3 ms = e / 3 ms }`, two `quire.op.quantity.div` nodes with arguments `[ref(d), ref(L)]` and `[ref(e), ref(L)]`, `L` the one shared `3 ms` literal node, each with `result_type` the dimensionless compound-unit node, under one `quire.op.quantity.eq`; `function p2 using v(a: Quantity<m>, b: Quantity<mps>): Boolean pure { convert<Quantity<mps>>(a / 3 ms) = b }`, a quotient whose compound unit has the terms `m` to the power 1 and `s` to the power -1, which `convert` admits to `mps`; `function p3 using v(a: Quantity<m>, b: Quantity<mps>): Boolean pure { convert<Quantity<mps>>(a / rational(-3, 1) ms) = b }`, divisor literal of value `"-3"`; `function p4 using v(a: Quantity<m>, b: Quantity<mps>): Boolean pure { convert<Quantity<mps>>(a / 0.5 s) = b }`, divisor literal of value `"1/2"` at the `s` node; `function p5 using v(d: Quantity<ms>, e: Quantity<ms>): Boolean pure { d / 3 s = e / 3 s }`; and `function p6 using v(d: Quantity<ms>): Boolean pure { d = 0 ms }`. Evaluating the lowered bodies gives: `p1` true for `d` = `6 ms`, `e` = `6 ms` and false for `d` = `6 ms`, `e` = `3 ms`; `p2` true for `a` = `6 m`, `b` = `2000 mps`; `p3` true for `a` = `6 m`, `b` = `-2000 mps`; `p4` true for `a` = `6 m`, `b` = `12 mps`; `p5` true for `d` = `6000 ms`, `e` = `6000 ms` and false for `d` = `6000 ms`, `e` = `3000 ms`; `p6` true for `d` = `0 ms`. IR's v2 reader admits each package. | Test (TC-483) |
| FR-093-AC-37 | Under AC-36's units, `function z9 using v(d: Quantity<ms>): Boolean pure { d / -3 ms = -(d / 3 ms) }` and `function z10 using v(d: Quantity<ms>): Boolean pure { d / -(-(3 ms)) = d / 3 ms }` check and lower with a dimensionless compound-unit result; at `d = 6 ms`, both comparisons evaluate `true` (`-2 = -2` for z9 and `2 = 2` for z10). A divisor that is the direct unary negation of a nonzero quantity literal, or nested unary negations rooted directly in such a literal, is a nonzero fact (QSpec FR-146-AC-9). Each of `z1` through `z3` (zero-magnitude divisors in `ms`), `z4` (whose rational denominator is zero), `z5` with `e = 3 ms`, `z6` with guarded `e = 3 ms`, `z7` with `k = 3 ms` and `z8` with `convert<Quantity<ms>>(3 s)` (value `3000 ms`), `function z11 using v(d: Quantity<ms>): Boolean pure { d / -(0 ms) = d / 3 ms }`, `function z12 using v(d: Quantity<ms>, e: Quantity<ms>): Boolean pure { d / -e = d / 3 ms }`, `function z13 using v(d: Quantity<ms>): Boolean pure { let k = 3 ms in d / -k = d / 3 ms }` and, with `function divisor using v(): Quantity<ms> pure { 3 ms }`, `function z14 using v(d: Quantity<ms>): Boolean pure { d / -divisor() = d / 3 ms }` refuses `undefined_expression`/`unproved-nonzero` at the first division, except `z4` refuses at its zero-denominator literal. Negated zero remains unproved at value `0` in `ms`; negated parameter `e`, local `k` and call result `divisor()` each have value `3` in `ms` in this fixture but remain unproved. Every refused function emits no quotient node, result unit or result value. | Test (TC-483) |
| FR-093-AC-38 | With `type Z = Decimal[-99, 99; 0, 2; nearest-even];`, `function z using v(): Z pure { decimal(0, N) }` checks for `N` = 65534 and 65535 and lowers one `decimal` literal node whose `value` is `0.` followed by `N` zeros (length `N` + 2). For `N` = 65536 and 4294967295, and for `decimal(1, 65536)` and `decimal(0, 65536)` at the same position, it refuses `stage_limit_exceeded`/`work-budget-exceeded` at the literal, naming bound 65535, the actual scale and the setting `s3.decimal_literal_scale`, before any charge and with no node, and the refusal is not `ill_typed`. | Test (TC-1232) |
| FR-093-AC-45 | With `s3.decimal_literal_scale` set to 4 and AC-38's `Z`, `decimal(0, 4)` checks and `decimal(0, 5)` refuses `stage_limit_exceeded`/`work-budget-exceeded` naming bound 4 and actual 5. With no expected type, `let v = decimal(0, 65536) in v` refuses `ill_typed`/`ambiguous-literal`, and `decimal(0, 65536)` at an `Int[0, 9]` position refuses `ill_typed`/`type-mismatch`, each before the limit. | Test (TC-1232) |
| FR-093-AC-52 | With AC-38's `Z` and the default settings, a package of 16 functions each of body `decimal(0, 65535)` checks (the total is 1048560 digits) and a package of 17 such functions refuses `stage_limit_exceeded`/`work-budget-exceeded` at the 17th literal, naming bound 1048576, actual 1114095 and the setting `s3.decimal_literal_scale_total`, before any charge and with no node for that literal; with `s3.decimal_literal_scale_total` set to 10, a package of `decimal(0, 5)` twice checks and with a third `decimal(0, 1)` it refuses naming bound 10 and actual 11. | Test (TC-1232) |
| FR-093-AC-53 | `decimal(1, 4294967295)` builds at S2 (FR-091-AC-57) and, at a `Decimal[..]` position, is refused by `s3.decimal_literal_scale` as in AC-38 and not at S2; `decimal(1, 4294967296)` is refused `invalid_syntax`/`invalid-token` at S1 (FR-091-AC-58); and a unit declaration's `unit c : Length = decimal(1, 5000) * m;` is refused by `s3.decimal_scale` with bound 4096 (FR-091-AC-67), not by either literal setting. | Test (TC-1232) |
| FR-093-AC-39 | With `type Q = Decimal[-99999, 99999; 0, 2; nearest-even];` and `function c1 using v(): Q pure { decimal(125, 2) }`, `c2` over `decimal(5, 2)`, `c3` over `decimal(10, 0)`, `c4` over `decimal(0, 2)`, `c5` over `decimal(-5, 1)` and `c6` over `decimal(1250, 3)`, each checks and lowers one `value`/`literal` node of `value_kind` `decimal` at `Q`'s type node, with `value` `"1.25"`, `"0.05"`, `"10"`, `"0.00"`, `"-0.5"` and `"1.250"`. Two functions over `decimal(125, 2)` share one literal node, and `c1` and `c6` are two. `function n using v(): Q pure { -decimal(5, 0) }` lowers one `quire.op.decimal.negate` node over the literal `"5"` (a lowering-only assertion on the node structure; the evaluated value is AC-57's). `function add using v(p: Q): Q pure { p + decimal(125, 2) }` lowers the literal typed `Q` and one `quire.op.decimal.add` node over `[ref(p), ref(the literal)]`; evaluated at `p` = coefficient 100, scale 2 it returns coefficient 225, scale 2, and `c1` returns coefficient 125, scale 2. IR's v2 reader admits each package. | Test (TC-1232) |
| FR-093-AC-40 | Under AC-39's `Q`: `function a2 using v(): Int[0, 9] pure { decimal(1, 0) }` and `function a3 using v(): Boolean pure { decimal(1, 0) }` refuse `ill_typed`/`type-mismatch`; and, at the literal, with no charge and no node, `function a4 using v(): Decimal[-9, 9; 0, 0; toward-zero] pure { decimal(150, 2) }` (1.50 has two fractional digits), `function a5 using v(): Decimal[-1000, 1000; 0, 1; nearest-even] pure { decimal(1234, 3) }`, `function a6 using v(): Decimal[-9, 9; 0, 0; toward-zero] pure { decimal(10, 0) }` (above the bound) and `function a7 using v(): Q pure { decimal(100000000000000000000000000000000000000000, 2) }` (a coefficient far above any type's bounds) refuse `ill_typed`/`type-mismatch`. Each of `decimal(100, 2)` at `Decimal[-9, 9; 0, 0; toward-zero]` (the value 1), `decimal(1200, 3)` at `Decimal[-1000, 1000; 0, 1; nearest-even]` (the value 1.2) and `decimal(0, 7)` at that scale-0 type checks, and its literal keeps the scale as written. | Test (TC-1232) |
| FR-093-AC-41 | With `type W = Decimal[-99999, 99999; 2, 2; nearest-even];`, each of `function x1 using v(a: W): W pure { a / decimal(4, 0) }`, `function x2 using v(a: W): W pure { a / decimal(400, 2) }` and `function x3 using v(a: W): Boolean pure { a / decimal(4, 0) = a }` checks, with one `quire.op.decimal.div` node over `[ref(a), ref(the literal)]` typed `W`, and `x1` evaluated at `a` = coefficient 600, scale 2 returns coefficient 150, scale 2. | Test (TC-1232) |
| FR-093-AC-54 | Under AC-41's `W`, `function y1 using v(a: W): W pure { a / decimal(0, 2) }` and `function y2 using v(a: W): W pure { a / decimal(0, 0) }` each refuse `undefined_expression`/`unproved-nonzero` at the division, with no node: a zero coefficient discharges nothing at any scale. | Test (TC-1232) |
| FR-093-AC-55 | Under AC-41's `W`, each of `function y3 using v(a: W, b: W): W pure { a / b }`, `function y4 using v(a: W, b: W): Boolean pure { a / b = a }`, `function y5 using v(a: W, c: Boolean): W pure { a / (if c then decimal(5, 2) else decimal(6, 2)) }` and `function y6 using v(a: W): W pure { a / -decimal(4, 0) }` refuses `undefined_expression`/`unproved-nonzero` at the division, with no node: a parameter, a conditional of nonzero literals and a negated literal are no nonzero fact. | Test (TC-1232) |
| FR-093-AC-42 | With `type P = Decimal[0, 1000; 2, 2; exact];` and `type R = Decimal[0, 1000; 1, 1; exact];`, `function f using v(r: R): P pure { convert<P>(decimal(15, 1) + r) }` checks, the literal typed `R` by its peer `r` (its `decimal` literal node `"1.5"` at `R`'s type node) and the sum typed `R` before the conversion, and lowers one `convert` node over the sum's node whose `result_type` is `P`'s type node. `convert<Decimal[0, 100; 0, 3; nearest-even]>(a)` over `a: Float64[nearest-even]` is refused `ill_typed`/`type-mismatch` as FR-397-AC-18 states. | Test (TC-1232) |
| FR-093-AC-56 | With `function g using v(q: Rational[0, 2; 1, 3]): Decimal[0, 100; 2, 2; nearest-even] pure { convert<Decimal[0, 100; 2, 2; nearest-even]>(q) }`, evaluating `g` at `q` = 2/3 returns coefficient 67, scale 2 with the charge points of QSpec TC-185 D24 plus the call's `FunctionCall`. The same body with the mode `exact` is refused `inexact_decimal`/`nonzero-discarded-digit` after the `decimal.arithmetic` charge, with no `decimal.rounding` charge and no value. | Test (TC-1232, QSpec TC-185) |
| FR-093-AC-57 | With `type N = Decimal[-9, 0; 0, 0; toward-zero];`, `function m1 using v(): N pure { -decimal(5, 0) }` and `function m2 using v(): N pure { decimal(5, 0) }` each refuse `ill_typed`/`type-mismatch` at the literal (5 is not a member of `N`), at check and before any charge; `function m3 using v(): N pure { -decimal(-5, 0) }` checks (the operand -5 is a member) and, evaluated, is refused `decimal_out_of_domain`/`outside-domain` (the negated value 5 is outside `N`); `function m4 using v(): N pure { -decimal(0, 0) }` checks and evaluates to coefficient 0, scale 0; and `-decimal(5, 0)` at AC-39's `Q` checks and evaluates to coefficient -500, scale 2, because a computed result is retained at the type's target scale `smax` (QSpec FR-140). | Test (TC-1232) |
| FR-093-AC-43 | With FR-091-AC-44's units (`Time`, `s`, `ms`), `dimension Period;`, `unit sec : Period = rational(1, 1);`, `unit msec : Period = rational(1, 1000) * sec;` and `dimension Period ~ Time by 1 sec = 1 s;` (root ratio 1), each of these checks and lowers: `function c1 using v(d: Quantity<s>): Boolean pure { convert<Quantity<sec>>(d) = 3 sec }`, a `quire.op.quantity.convert` node with mode `rounding` = `exact` over `d` then `quire.op.quantity.eq` over it and the `3 sec` literal at the `sec` node; `function c2 using v(d: Quantity<s>): Boolean pure { convert<Quantity<msec>>(d) = 3000 msec }`; and `function c3 using v(e: Quantity<msec>): Boolean pure { convert<Quantity<s>>(e) = rational(3, 2) s }`. IR's v2 reader admits each package, and evaluating the lowered bodies gives: `c1` true for `d` = `3 s` and false for `d` = `4 s`; `c2` true for `d` = `3 s`; `c3` true for `e` = `1500 msec`. With `dimension Length;` and `unit m : Length = rational(1, 1);` beside them, `function c4 using v(d: Quantity<s>): Boolean pure { convert<Quantity<m>>(d) = 3 m }` refuses `ill_typed` at the conversion, and in the same units without `dimension Period ~ Time by 1 sec = 1 s;` `c1` refuses `ill_typed` at the conversion. | Test (TC-483) |
| FR-093-AC-44 | Under AC-43's units, each of these refuses `ill_typed`/`type-mismatch` at the operation and emits no node: `function a1 using v(d: Quantity<s>, p: Quantity<sec>): Boolean pure { d + p = d + p }`, `function a2 using v(d: Quantity<s>, p: Quantity<sec>): Boolean pure { d = p }`, `function a3 using v(d: Quantity<s>, p: Quantity<sec>): Boolean pure { d < p }` and, with `function f using v(d: Quantity<s>): Boolean pure { true }`, `function a4 using v(): Boolean pure { f(3 sec) }`. Each of these checks and lowers: `function a5 using v(d: Quantity<s>, p: Quantity<sec>): Boolean pure { convert<Quantity<s>>(p) + d = 6 s }`, true for `d` = `3 s` and `p` = `3 sec`; `function a6 using v(): Boolean pure { f(convert<Quantity<s>>(3 sec)) }`; `function a7 using v(d: Quantity<s>): Boolean pure { d / 3 sec = d / 3 s }`, two `quire.op.quantity.div` nodes whose `result_type` is the one dimensionless compound-unit node, true for `d` = `6 s`; and `function a8 using v(d: Quantity<s>, p: Quantity<sec>): Boolean pure { d * p = d * d }`, whose two products have one compound-unit node over the class root unit squared, true for `d` = `2 s` and `p` = `2 sec`. | Test (TC-483) |
| FR-093-AC-46 | Unequal roots (QSpec FR-142-AC-26). With FR-091-AC-44's units, `dimension Beat;`, `unit beat : Beat = rational(1, 1);`, `unit bar : Beat = rational(4, 1) * beat;` and `dimension Beat ~ Time by 1 beat = rational(1, 4) s;` (root ratio 1/4: a beat is a quarter of a second), each of these checks and lowers: `function u1 using v(d: Quantity<beat>): Boolean pure { convert<Quantity<s>>(d) = rational(1, 4) s }`, true for `d` = `1 beat` and false for `d` = `2 beat`; `function u2 using v(d: Quantity<s>): Boolean pure { convert<Quantity<beat>>(d) = 12 beat }`, true for `d` = `3 s`; `function u3 using v(d: Quantity<bar>): Boolean pure { convert<Quantity<ms>>(d) = 1000 ms }`, true for `d` = `1 bar`; and `function u4 using v(d: Quantity<s>): Boolean pure { d / 3 beat = d / rational(3, 4) s }`, two `quire.op.quantity.div` nodes with the one dimensionless compound-unit node, true for `d` = `6 s` (both quotients are `8`). In the same units with `dimension Beat ~ Time by 1 beat = 1 s;` (root ratio 1) `u1` is false for `d` = `1 beat` and true for `d` = `rational(1, 4) beat`, and `u2` is true for `d` = `12 s`. `function u5 using v(d: Quantity<s>, p: Quantity<beat>): Boolean pure { d + p = d + p }` refuses `ill_typed`/`type-mismatch` at the operation under either declaration. | Test (TC-483) |
| FR-093-AC-47 | `function g using v(t: Text[0, 8; nfc]): Boolean pure { convert<Integer>(t) = 3 }` lowers the comparison to `quire.op.integer.eq` with arguments `[ref(the convert node), ref(the literal 3)]`, where the convert node is `quire.op.text.to_integer` over `[ref(t)]` and no `ConvertScalar` or `quire.op.numeric.convert` node is built. Over the same `t`, `convert<Boolean>(t)`, `convert<Text[0, 4; nfc]>(t)`, `convert<String>(t)` and, for `s: String`, `convert<Text[0, 8; nfc]>(s)` each refuse `ill_typed`/`operator-ineligible` at the `convert` and build no node; `convert<Text[0, 8; nfc]>(t)` builds no node. | Test (TC-1196) |
| FR-093-AC-48 | A checked present-option value is lowered under the carrier contract in QSpec FR-322; QSL's I2 reader (FR-264) returns `Verified`, and the test asserts that the emitted package id equals the package id the reader computes from that package's checked contents. S3 refuses a `Some(e)` whose checked type differs from the option result's element type as `ill_typed`/`type-mismatch` at the `Some(e)` expression; S4 emits no package for it. | Test (TC-1273) |
| FR-093-AC-49 | Quantity negation (QSL-1389; QSpec TC-187 U39, FR-142-AC-25). With FR-091-AC-44's units, `dimension Length;`, `unit m : Length = rational(1, 1);`, `dimension Temperature;`, `unit kelvin : Temperature = rational(1, 1);` and `unit degC : Temperature = rational(1, 1) * kelvin + decimal(27315, 2);`, each of these checks, lowers and evaluates: `function a using v(): Quantity<m> pure { -(2 m) }` to `-2 m` and `function b using v(): Quantity<m> pure { -(2 m) + 1 m }` to `-1 m` (dimension `Length`); `function c using v(): Quantity<ms> pure { -(3 ms) }` and `function c2 using v(): Quantity<ms> pure { -3 ms }` to `-3 ms` in `ms`, the same two nodes; `function z using v(): Quantity<m> pure { -(0 m) }` to `0 m`; `function q using v(): Boolean pure { -(-(3 ms)) = 3 ms }` to `true` through two nested negate nodes; `function r using v(d: Quantity<ms>): Boolean pure { d = -3 ms }` with a `quire.op.quantity.eq` over `d` and the negate node of the `3 ms` literal; and the compound `function v2 using v(): Boolean pure { -(3 m / 1 s) = -(3 m / 1 s) }` to `true`, the two equal sides sharing one content-addressed negate node, whose `result_type` is the quotient's compound-unit node (terms `m` to the power 1 and `s` to the power -1) and which has no law, mode, member or leaves. Every non-affine row lowers to `quire.op.quantity.negate` and traverses no unit edge, and IR's v2 reader admits each package. `function n using v(t: Quantity<degC>): Quantity<degC> pure { -t }` and `function n2 using v(): Quantity<degC> pure { -(5 degC) }` each refuse `ill_typed`/`operator-ineligible` at the negation with no charge and no node; `function k using v(d: Quantity<ms>): Boolean pure { d / -3 ms = -(d / 3 ms) }` checks and evaluates to `true` at `d = 6 ms`, with both sides dimensionless and equal to `-2`; `function k2 using v(d: Quantity<ms>): Boolean pure { d / -(-(3 ms)) = d / 3 ms }` checks and evaluates to `true` at `d = 6 ms`, with both sides dimensionless and equal to `2`; `function k3 using v(d: Quantity<ms>): Boolean pure { d / -(0 ms) = d / 3 ms }` and `function k4 using v(d: Quantity<ms>, e: Quantity<ms>): Boolean pure { d / -e = d / 3 ms }` each refuse `undefined_expression`/`unproved-nonzero` at the division; `function b2 using v(b: Boolean): Boolean pure { -b }` refuses `ill_typed`/`type-mismatch` at the operand. | Test (TC-1285) |
| FR-093-AC-50 | `function t1 using v(): Text[0, 8; nfc] pure { Text("e\u0301") }` lowers the body to one `value`/`literal` node whose `semantic_type` and body `type` are the `Text[0, 8; nfc]` type node, with `value_kind` `text` and `value` `"\u00e9"` as JSON; `Text("\u00e9")` in the same slot lowers to the same node. In `function e3 using v(x: Text[0, 8; nfc]): Boolean pure { x = Text("\u00e9") }` the comparison is one text equality with law `text_profile` and mode `text_profile` `nfc` (FR-093-AC-6) and arguments `[ref(x's node), ref(the literal's node)]`. | Test (TC-1052) |
| FR-093-AC-51 | Fused multiply-add (STD-646; FR-397-AC-15). Over a unit that declares `x: Float64[nearest-even]`, `y: Float64[nearest-even]` and `z: Float64[nearest-even]` parameters and a `w: Float32[nearest-even]` parameter, `quire::value::ieee::fma(x, y, z)` lowers to one node with `operator` `call` as a generic application call (there is no dedicated fma node kind, and the node does not take the `quire.op.function.call` identity or a function-node argument), `operation.identity` `quire.op.ieee.float64.fma`, law `ieee_profile`, mode `rounding` `nearest-even`, `result_type` the `Float64[nearest-even]` node and exactly the three arguments `[ref(x), ref(y), ref(z)]` in call order; the same over `Float32[nearest-even]` gives `quire.op.ieee.float32.fma` and a `Float32` result. `fma(x, y)` and `fma(x, y, z, x)` refuse `ill_typed`/`type-mismatch` at the call, and `fma(x, y, w)` refuses `ill_typed`/`type-mismatch` at the argument `w`, each emitting no node (FR-397-AC-16). | Test (TC-415) |
| FR-093-AC-58 | Quantity integer power (QSL-1656; QSpec TC-187 U12, U13 and U21, FR-142-AC-6). With FR-091-AC-44's units, `dimension Length;`, `unit m : Length = rational(1, 1);`, `dimension Area = Length ^ 2;`, `unit m2 : Area = rational(1, 1);`, `dimension Temperature;`, `unit kelvin : Temperature = rational(1, 1);` and `unit degC : Temperature = rational(1, 1) * kelvin + decimal(27315, 2);`, each of these checks, lowers and evaluates: `function w1 using v(): Boolean pure { 2 m ^ 2 = 2 m * 2 m }` to `true`, its power one `quire.op.quantity.pow` node with arguments `[ref(L), ref(N)]`, `L` the `2 m` literal node and `N` the `Integer` literal node of value `"2"`, whose `result_type` is the one compound-unit node (`m` to the power 2) the product also has, with no law, mode, member or leaves (U12's square); `function w2 using v(): Boolean pure { convert<Quantity<m2>>(3 m ^ 2) = 9 m2 }` to `true`; `function w3 using v(d: Quantity<ms>): Boolean pure { d ^ 0 = 1 m / 1 m }` to `true` for `d` = `0 ms` and for `d` = `5 ms`, the power's `result_type` the dimensionless compound-unit node; `function w4 using v(): Boolean pure { 0 m ^ 0 = 1 m / 1 m }` to `true` (U21); `function w5 using v(d: Quantity<ms>): Boolean pure { d ^ 1 = convert<Quantity<s>>(d) * (1 s / 1 s) }` to `true` for `d` = `1500 ms`, the power's unit being `s` to the power 1; `function w6 using v(): Boolean pure { 2 m ^ -1 = (1 m / 2 m) / 1 m }` to `true`, the literal base `2 m` discharging the nonzero obligation; and `function w7 using v(): Boolean pure { -1 m ^ 2 = -(1 m * 1 m) }` to `true`, the power binding tighter than the prefix `-`. `function w8 using v(): Boolean pure { 2 m ^ 18446744073709551616 = 2 m ^ 18446744073709551616 }` checks, the exponent an exact integer above `u64::MAX`, and evaluated with U13's tuple as the meter's remaining budget at the first power node it is `incomplete { limit_kind: integer_bits, limit: 64, consumed: 2, next_charge: 36893488147419103232, charge_point: unit.rational-arithmetic }` before the power is computed. IR's v2 reader admits each package. `function r1 using v(d: Quantity<m>): Boolean pure { d ^ -1 = d ^ -1 }` and `function r2 using v(): Boolean pure { 0 m ^ -1 = 0 m ^ -1 }` each refuse `undefined_expression`/`unproved-nonzero` at the first `^` under `CheckMode::Linked`; under `CheckMode::Kernel` `r2` checks and its first power evaluates to `Undefined::DivisionByZero` (`division-by-zero`) after one `unit.identity-read` with U21's `L(1,0,0,1,1,0)` as the remaining budget, and is `incomplete { limit_kind: work_units, limit: 0, consumed: 0, next_charge: 1, charge_point: unit.identity-read }` with `L(1,0,0,1,0,0)`. `function r3 using v(t: Quantity<degC>): Boolean pure { t ^ 2 = t ^ 2 }` refuses `ill_typed`/`operator-ineligible` at the first `^` with no charge and no node; `function r4 using v(i: Int[0, 9]): Boolean pure { i ^ 2 = i }` refuses `ill_typed`/`type-mismatch` at `i`; `function r5 using v(c: Quantity<m, Int[0, 9]>): Boolean pure { c ^ 2 = c ^ 2 }` refuses `ill_typed`/`type-mismatch` at the first power. The bodies `d ^ k`, `d ^ (2)`, `d ^ 2 ^ 3` and `d ^ rational(1, 2)` over `d: Quantity<m>` and `k: Integer` refuse `invalid_syntax`/`unexpected-token` at the token after the first `^` that is not a signed integer literal (at the second `^` for `d ^ 2 ^ 3`), before S2. | Test (TC-1336) |
| FR-093-AC-59 | One-occurrence collection forms (QSL-1488; QSpec FR-145-AC-10 and AC-11, TC-190). Each of these checks, lowers and evaluates: `function f1 using v(s: Sequence<Integer>[0, 9]): Sequence<Integer>[0, 8] pure { dropFirst(s) }`, one `quire.op.collection.sequence.drop_first` node over `[ref(s)]` whose `result_type` is the `Sequence<Integer>[0, 8]` node, with no law, mode, member or leaves, giving `sequence[1, 3]` for `sequence[3, 1, 3]` and `sequence[]` for `sequence[]`; `function f2 using v(o: OrderedSet<Integer>[1, 5]): OrderedSet<Integer>[0, 4] pure { dropFirst(o) }`, one `quire.op.collection.ordered_set.drop_first` node, giving `orderedSet[7]` for `orderedSet[2, 7]`; `function g1 using v(s: Sequence<Integer>[0, 9]): Sequence<Integer>[0, 9] pure { removeOne(s, 3) }`, one `quire.op.collection.remove_one` node over `[ref(s), ref(L)]`, `L` the literal `3`, with no leaves, giving `sequence[1, 3]` for `sequence[3, 1, 3]` and `sequence[1]` for `sequence[1]`; the same body over `Bag<Integer>[0, 9]` giving `bag[2, 5]` for `bag[2, 2, 5]` with `removeOne(b, 2)`, over `Set<Integer>[0, 9]` giving `set[5]` for `set[2, 5]`, and over `OrderedSet<Integer>[0, 9]` giving `orderedSet[5, 7]` for `orderedSet[5, 2, 7]`; `removeOne(t, x)` over `t: Bag<Text[0, 8; nfc]>[0, 9]` and `x: Text[0, 8; nfc]`, whose node lists the empty-path leaf with the `nfc` law and mode as `contains` lists it; and `removeOne(u, q)` over `u: Set<Float64[nearest-even]>[0, 9]` and `q: Float64[nearest-even]`, whose node lists the empty-path float leaf with the law `ieee_profile` and mode `null`, giving `set[]` for `u` = `set[0x7ff8000000000000]` and `q` of the same bits, and `u` unchanged for `u` = `set[0x0000000000000000]` and `q` = `0x8000000000000000` (bit identity). IR's v2 reader admits each package. The meter's charge log of `dropFirst(sequence[1, 2, 3])` holds only the formation of the two survivors, with no visit and no membership comparison, and that of `removeOne(sequence[1, 2, 3], 2)` holds the `contains` scan charges of the members `1` and `2` with no scalar retain, then the formation of the two survivors. `dropFirst(t)` over `t: Set<Integer>[0, 9]` and over `t: Bag<Integer>[0, 9]` refuse `ill_typed`/`operator-ineligible` at the call and build no node; `removeOne(s, true)` over `s: Sequence<Integer>[0, 9]` refuses `ill_typed`/`type-mismatch` at `true`; `dropFirst(i)` over `i: Int[0, 9]` refuses `ill_typed`/`type-mismatch` at `i`; `dropFirst(s, s)` and `removeOne(s)` refuse `invalid_syntax`/`unexpected-token`. | Test (TC-1335) |
| FR-093-AC-60 | Lowering source dimensions preserves base-versus-derived identity independently of normalized terms (QSL-1738; FR-091-AC-73, FR-092-AC-25). For `dimension L; dimension N = L / L; unit m : L = rational(1, 1); unit one : N = rational(1, 1);`, the checked semantic graph has distinct `dimension` nodes for `L` and `N`, with `L` carrying a base payload and `N` a derived payload (`base=false`) with no terms. The `m` and `one` declarations resolve to their respective nodes; `one` is dimensionless. | Test (TC-483) |
| FR-093-AC-61 | Bounded nominal quantities lower addition, conversion and less-than to their existing quantity operation identities. Adding `1 m` and `2 m` in `[0,2]` gives ordinary `Quantity<m>` of value `3 m`; converting `150 cm` to `Quantity<m>[0,2]` gives `3/2 m`; converting `200 cm` gives the inclusive endpoint `2 m`, while `201 cm` refuses `rational_out_of_domain` at the result type. Less-than of `1 m` and `convert<Quantity<m>>(150 cm)` gives true under the existing identical-unit ordering rule; dimensionally incompatible operands retain their existing typed refusal. Existing bare-unit exact-id refusal controls remain unchanged (QSpec BQ-07..BQ-10). | Test (TC-413) |
| FR-093-AC-62 | For `function w using v(a: Integer, b: Integer): Integer pure { (a + b) * (a + b) }`, the checked unit's accessor returns for the multiplication the `NodeKey` of its `expression` node in the checked graph, and for both `a + b` operations the one `NodeKey` of their shared `expression` node, with two different `Location`s; a function whose body holds `a * b` returns a key different from all three. No stage other than `check` constructs a `NodeKey` for it. | Test (TC-415) |

## Dependencies

- [FR-092](FR-092-key-type-parameter-and-declared-nodes.md): type, parameter
  and function nodes, the structural node ids and the literal spellings.
- [FR-319](FR-319-key-union-member-and-case-nodes.md) and
  [FR-323](FR-323-key-union-values-in-collections.md): union identity,
  equality and canonical collection keys; QSpec
  [FR-440](ix://agent-ix/quire-specification/FR-440) and
  [FR-322](ix://agent-ix/quire-specification/FR-322): union nodes, the
  catalog's leaf sources and the shared leaf path vocabulary.
- [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)
  §2.2 E3 (keys minted at E3), §2.4 (package selections and law `DefinitionRef`s),
  §3 FB-13, §6.1 (a family's emission arm under `package`).
- [ADR-013](../decisions/ADR-013-canonical-type-package-conversion-ownership.md)
  O-04, O-07 (occurrences), QC-24.
- [FR-065](FR-065-migrate-function-application-to-checked-family.md): the
  function declaration and call identities this lowering keys.
- [FR-094](FR-094-key-model-owned-reference-population-and-quantity-nodes.md):
  the model declaration nodes the model rows' members name, and the
  `Reference`, `Population` and quantity type nodes.
- QSpec: the definitions the QSpec definition prose names (ADR-011 §2.4),
  which the layer table names by identity. A node whose
  operation needs no law is keyed with no law;
  every law's `definition` is the layer table's. The two-name `fold`/`reduce` binder and the
  `bound` of a nested binding are QSL proposals (ADR-013 QC-24).
- QSpec FR-322 `dependencies`: FR-322 gives the rule for an application
  node, a nominal node and (FR-340) a frame node. Rules 1 and 3 for every
  other node are QSL's proposal (ADR-013 QC-27). The published
  `positive-all-families.json` writes `dependencies: []` for the
  `expression`/`reference` node `eeee…` and the `correspondence` node
  `7070…`, whose bodies reference `dddd…`, and for the `bounded_domain`
  node `cccc…`; QSL asks QSpec to correct them.
- QSpec FR-322 `owner` (QSL-638): the node member the emission writes
  (the reader's joins over it, FR-322-AC-51 to AC-53). IR's
  v2 node type carries it once IR-646 adds `owner` to
  `CheckedSemanticNodeV2`; AC-21 and AC-22 ("IR's v2 reader admits") cannot
  pass before that lands.
- The `Attribute` row's `record.project` over a `deref` result: FR-322 gives a
  model entity type the family `reference`, and no QSpec fixture projects an
  attribute of a dereferenced object. The row is QSL's proposal (ADR-013
  QC-24).
- [ADR-012](../decisions/ADR-012-semantic-family-extension-contracts.md)
  §5.1: no catch-all arm; §3 and §4.3: a postcondition, and the `Pre` read
  inside it, belong to the `ProtocolClause` family, which lowers them with
  #218 (designed in #223).
- QSpec FR-143: the recursive composites whose text-leaf walk rule 3 ends.
- The `recursion:d` leaf segment and an optional field's `inner` segment:
  FR-322's `LeafSegment` admits `field:<name>`, `position:<n>` and `inner`,
  and names no leaf form for a recursive composite. Both are QSL proposals
  (ADR-013 QC-24). Until QSpec adopts QC-24, a v2 reader that validates
  against the published `LeafSegment` pattern refuses a package holding a
  recursion leaf, and a reader that derives an optional field's leaf
  without `inner` refuses its leaves.
