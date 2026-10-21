---
id: FR-398
title: "Expose the admitted unit table through the replay facade"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-394
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-396
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-060
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-087
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-094
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-015
    type: depends_on
---
# FR-398: Expose the admitted unit table through the replay facade

## Description

ADR-011 T12 and FR-087 forbid a consumer of QSL to build a `NodeKey`. A
contract generator that lowers a quantity operand builds `ValueType::Quantity`
for the operand's unit and a unit conversion for each explicit quantity
conversion. `ValueType::Quantity` and a kernel `Quantity` take the unit's
kernel `UnitId`: for a declared unit, `UnitId::declared` of the unit node's
key, the id QSL's checked `Quantity<u>` types and replayed quantity values
carry. A conversion (`convert_quantity`) takes the source quantity resolved
against a unit table and the target unit's `QuantityUnit`. A consumer holding
a `compile_admitted` result (FR-394) can get neither the id nor the unit
graph. This requirement extends the `AdmittedContext` with the dimension and
unit tables and the unit table, so the consumer looks a declared unit up by
its `unit` node's wire id, reads its kernel id and unit, and never decodes,
hashes or builds a key.

## Inputs

- A `compile_admitted` result (FR-394).
- For a lookup, the wire node id of a declared unit's or dimension's node, as
  the emitted package or a contract IR document carries it.

## Outputs

- The context lists every dimension an admitted unit names, and the base
  dimensions those derive from, each with the id of its node, its qualified
  name and whether it is a base dimension, ascending by node id
  (`dimensions`).
- The context lists every declared unit the check admitted, each with the id
  of its node, its kernel `UnitId`, its qualified name, its dimension's node
  id and its `QuantityUnit`, ascending by node id (`units`).
- `unit_declaration` and `dimension_declaration` return the admitted entry or
  a typed refusal.
- `unit_table` returns the package's kernel unit table: each admitted unit by
  its kernel id.
- A unit's `converts_to` answers whether a quantity of it converts to another
  admitted unit, the kernel relation (same normalized dimension).

## Behavior

- The kernel id of a unit is the key of the unit table the package's checked
  type environment holds: the table the checker resolved `Quantity<u>`
  against. It is `UnitId::declared` of the unit node's key: the same 32 bytes
  as the node id, wrapped in the declared-unit domain
  (`UnitDomain::Declared`). The two are different types, `UnitId` and
  `NodeKey`, so the node id cannot stand in for the kernel id in
  `ValueType::Quantity` or a `Quantity`. A lookup takes the node id; the entry
  returns both. A consumer builds `ValueType::Quantity` and `Quantity` from
  the returned id and never wraps the node id itself.
- A unit enters the package's checked graph, and so the emitted package, only
  when the package uses it. The context lists every declared unit, used or
  not, so a consumer holding the node id of a used unit finds it, and a
  declared unit the package never uses is listed with the node id its
  declaration determines.
- The tables hold only the ids the check admitted. A lookup finds an entry or
  refuses; it creates none and returns no partial value.
- Two units, including two of one dimension, have different ids; the same
  unit looked up again, or recompiled from the same input, has the same id.
- A unit's `QuantityUnit` carries its normalized dimension, canonical root and
  exact path to it, so `convert_quantity` with the unit as the target
  converts a quantity resolved from `unit_table` (FR-094, QSV unit graph).
- The lookups refuse in this order: an id a package of the dependency
  closure, not this package, admitted as a unit (for `unit_declaration`) or a
  dimension (for `dimension_declaration`) is `Foreign`, naming that
  dependency's `package_id`; an id that names another node of this package's
  checked graph (a dimension for a unit lookup, a unit for a dimension
  lookup, an enum, a record, a function or any other node, or a declared
  unit or dimension the package does not use) is `WrongKind`;
  any other id is `Unknown`.
- The lookups of FR-394 and FR-396 keep their refusals, except that a unit or
  dimension id looked up as a record or tuple is `WrongKind`, as every other
  node of the package is. To tell `WrongKind` from `Unknown` the context keeps
  the id of every node of this package's checked graph: its size grows with
  the graph, one 32-byte id per node.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-398-AC-1 | A consumer that names only the `qsl_replay` root and the kernel (`quire-exact`, `quire-semantic-value`), compiling a unit that declares `dimension Length;`, `unit m : Length = rational(1, 1);` and `unit km : Length = rational(1000, 1) * m;` through `compile_admitted`, finds both units through `unit_declaration`, and from what the context returns builds `ValueType::Quantity` of each, a quantity of `m` that a type environment over `unit_table` admits at `Quantity<m>` and refuses at `Quantity<km>`, and the exact conversion of that quantity to `km`, without decoding, hashing or constructing a node key or a unit id. | Test (TC-1092) |
| FR-398-AC-2 | Over a unit declaring two dimensions and three units (two of one dimension) and a function whose parameters are `Quantity<m>`, `Quantity<km>` and `Quantity<s>`, the context's tables equal, per unit and dimension, the spine's own checked package read independently of the unit table: the node id and name of each `unit` and `dimension` semantic node, and the `UnitId` the checked function signature carries for each `Quantity<u>` parameter, which carries the declared-unit domain and the node id's bytes. The unit ids differ; each unit names its dimension's node id; `m` and `km` convert to each other and neither converts to `s`; `unit_table` holds every admitted unit by its id; and a repeated lookup and a recompile return the same entries. | Test (TC-1092) |
| FR-398-AC-3 | `unit_declaration` and `dimension_declaration` refuse `Unknown` for an id no package admitted; `WrongKind` for the unit's dimension, enum and record (unit lookup) and for a unit (dimension lookup); and `Foreign` naming the library's `package_id` for a library unit and dimension imported directly or through an intermediate library, while every own unit is found. `composite_declaration` refuses a unit id `WrongKind`. | Test (TC-1092) |
| FR-398-AC-4 | FR-394-AC-1 to FR-394-AC-5 and FR-396-AC-1 to FR-396-AC-5 still hold. | Test (TC-1089, TC-1090) |

## Dependencies

- FR-394 (the admitted-context facade), FR-396 (the composite table and the
  refusals this mirrors), FR-060 (the T12 boundary), FR-087 (the cross-package
  node key), FR-091 and FR-094 (the unit declarations and their nodes),
  ADR-015 (the dependency closure).
