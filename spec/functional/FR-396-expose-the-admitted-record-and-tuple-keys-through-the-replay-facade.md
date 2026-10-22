---
id: FR-396
title: "Expose the admitted record, tuple and union keys through the replay facade"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-394
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-060
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-087
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-092
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-015
    type: depends_on
---
# FR-396: Expose the admitted record, tuple and union keys through the replay facade

## Description

ADR-011 T12 and FR-087 forbid a consumer of QSL to build a `NodeKey`. A
contract generator that lowers a record, tuple or union equality builds a
kernel `CompositeDeclaration` and `ValueType::Composite` for each record,
tuple and union of the operand's type closure. Both take the declaration's kernel key: the
key the package's checked `TypeEnvironment` gives the declaration, which
every `ValueType::Composite` of it in QSL's own checked signatures carries,
and which every replayed record or tuple value carries. A union is settled
under a second key: the environment's `union_key(key)` is the union's
`composite_type` node id, and the `UnionMember` of every union value, and the
`final_key` of `seal_union_verified`, are that node id. The kernel key of a record or tuple is not the id of the
declaration's `composite_type` node (FR-092-AC-12), which is what the
emitted package and a contract IR document name. This requirement extends
the `AdmittedContext` that `compile_admitted` returns (FR-394) with the
record, tuple and union table, so the consumer looks a declaration up by its
`composite_type` node's wire id and reads its kernel key, and never decodes,
hashes or builds one.

## Inputs

- A `compile_admitted` result (FR-394).
- For a lookup, the wire node id of a record, tuple or union declaration's
  `composite_type` node, as the emitted package or a contract IR document
  carries it.

## Outputs

- The context lists every record, tuple and union declaration the check
  admitted, each with the id of its `composite_type` node, its kernel key, its
  qualified name and its form (`Record`, `Tuple` or `Union`), ascending by
  node id.
- A union additionally lists its members in declared order, each with its
  identifier, its sealed `UnionMember` binding (declaration: the union's node
  id), its `VariantId` and its payload position types. The `VariantId` is the
  one the check bound to the member (QSpec FR-441), which the kernel
  `UnionMember` of a replayed union value carries. A consumer seals a union
  with `seal_union_verified(key, node, bindings)` over these bindings; an
  unsealed union admits no value. A payload position that is
  a record, tuple or union is `ValueType::Composite` of that declaration's
  kernel key.
- `composite_declaration` returns the admitted declaration, or a typed
  refusal.

## Behavior

- The table holds only the keys the check admitted for the package's record,
  tuple and union declarations. A lookup finds a key or refuses; it creates
  none and returns no partial value.
- Two declarations of the same shape under different names have different
  keys, and two same-shaped unions have different member ids; the same declaration looked up again, or recompiled from the same
  input, has the same key.
- The lookup refuses in this order: an id a package of the dependency
  closure, not this package, admitted as a record, tuple or union is `Foreign`,
  naming that dependency's `package_id`; an id that names another node of
  this package's checked graph (an enum, a scalar type, a function, a declared unit or dimension (FR-398) or any other
  node) is `WrongKind`; any other id is `Unknown`.
- The enum lookups of FR-394 keep their refusals: they never refuse
  `WrongKind`.
- To tell `WrongKind` from `Unknown`, the context keeps the id of every node
  of this package's checked graph: its size grows with the graph, one 32-byte
  id per node.
- A union value in a replay witness names its union by the declaration's
  `composite_type` node id and a member by its identifier; a record or tuple
  value names its kernel key.
- A unit cannot name an imported record or tuple as a type (the assembler
  refuses `ImportedTypeName`), so an own declaration never has a field of a
  dependency's composite type.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-396-AC-1 | A consumer that names only the `qsl_replay` root, the kernel (`quire-exact`) and the QSV declaration API, compiling a unit that declares a record and a tuple through `compile_admitted`, finds both through `composite_declaration` with forms `Record` and `Tuple`, and builds from their keys a closed `TypeEnvironment` with a record field of the tuple's `ValueType::Composite` type and a record value it admits, without decoding, hashing or constructing a key. | Test (TC-1090) |
| FR-396-AC-2 | Over a unit with two same-shaped records, tuples and unions, the context's table equals, per declaration, the spine's own checked package read independently of its checked type nodes: the `composite_type` semantic node's id, the key its checked `TypeEnvironment` gives the declaration, its name and form. A function parameter of a record's type is declared `ValueType::Composite` of that record's admitted key, which differs from its node id. Every declaration's key differs, and a repeated lookup and a recompile return the same declarations. | Test (TC-1090) |
| FR-396-AC-3 | `composite_declaration` refuses `Unknown` for an id no package admitted, `WrongKind` for the unit's enum declaration and function, and `Foreign` naming the library's `package_id` for a library record or union imported directly or through an intermediate library, while every own record, tuple and union is found. | Test (TC-1090) |
| FR-396-AC-4 | FR-394-AC-1 to FR-394-AC-5 still hold: the enum table, its lookups and refusals, and the `Package` / `Resource` / `Fault` compile refusals are unchanged. | Test (TC-1090) |
| FR-396-AC-5 | A replay witness whose record and tuple values name the admitted kernel keys replays on both arms, and the same witness naming the `composite_type` node ids refuses. | Test (TC-1090) |
| FR-396-AC-6 | Over a unit with a union with a record payload and a union with a union payload, each union is found by its `composite_type` node id with form `Union`, and its members equal, in declared order, the producer's own: the `VariantId` of the FR-441 member key over the union's settled node, the sealed `TypeEnvironment` binding of the member, and literal payload types (a record or union position is `ValueType::Composite` of that declaration's admitted key). | Test (TC-1090) |
| FR-396-AC-7 | A consumer naming only the `qsl_replay` root, the kernel (`quire-exact`) and the QSV declaration API declares a union's `CompositeDeclaration` from its admitted key and payloads, seals it with `seal_union_verified` over the admitted bindings (`AdmittedUnionMember::member`), and constructs and admits a value of every member, a member holding another union included, without decoding, hashing or constructing a key. | Test (TC-1090) |
| FR-396-AC-8 | Two same-shaped unions have distinct kernel keys, no `VariantId` is shared by two members of the unit's unions, and a repeated lookup and a recompile return the same declarations and members. | Test (TC-1090) |
| FR-396-AC-9 | A replay witness whose union value names the union's `composite_type` node id and an admitted member replays on both arms, and the same witness naming the union's kernel key refuses. | Test (TC-1090) |

## Dependencies

- FR-394 (the admitted-context facade), FR-060 (the T12 boundary), FR-087
  (the cross-package node key), FR-092 (the composite node key), ADR-015 (the
  dependency closure).
