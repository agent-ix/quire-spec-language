---
id: FR-396
title: "Expose the admitted record and tuple keys through the replay facade"
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
# FR-396: Expose the admitted record and tuple keys through the replay facade

## Description

ADR-011 T12 and FR-087 forbid a consumer of QSL to build a `NodeKey`. A
contract generator that lowers a record or tuple equality builds a kernel
`CompositeDeclaration` and `ValueType::Composite` for each record and tuple
of the operand's type closure. Both take the declaration's kernel key: the
key the package's checked `TypeEnvironment` gives the declaration, which
every `ValueType::Composite` of it in QSL's own checked signatures and every
replayed composite value carries. That key is not the id of the
declaration's `composite_type` node (FR-092-AC-12), which is what the
emitted package and a contract IR document name. This requirement extends
the `AdmittedContext` that `compile_admitted` returns (FR-394) with the
record and tuple table, so the consumer looks a declaration up by its
`composite_type` node's wire id and reads its kernel key, and never decodes,
hashes or builds one.

## Inputs

- A `compile_admitted` result (FR-394).
- For a lookup, the wire node id of a record or tuple declaration's
  `composite_type` node, as the emitted package or a contract IR document
  carries it.

## Outputs

- The context lists every record and tuple declaration the check admitted,
  each with the id of its `composite_type` node, its kernel key, its
  qualified name and its form (`Record` or `Tuple`), ascending by node id.
- `composite_declaration` returns the admitted declaration, or a typed
  refusal.

## Behavior

- The table holds only the keys the check admitted for the package's record
  and tuple declarations. A lookup finds a key or refuses; it creates
  none and returns no partial value.
- Two declarations of the same shape under different names have different
  keys; the same declaration looked up again, or recompiled from the same
  input, has the same key.
- The lookup refuses in this order: an id a package of the dependency
  closure, not this package, admitted as a record or tuple is `Foreign`,
  naming that dependency's `package_id`; an id that names another node of
  this package's checked graph (an enum, a union, a scalar type, a function
  or any other node) is `WrongKind`; any other id is `Unknown`.
- The enum lookups of FR-394 keep their refusals: they never refuse
  `WrongKind`.
- To tell `WrongKind` from `Unknown`, the context keeps the id of every node
  of this package's checked graph: its size grows with the graph, one 32-byte
  id per node.
- A unit cannot name an imported record or tuple as a type (the assembler
  refuses `ImportedTypeName`), so an own declaration never has a field of a
  dependency's composite type.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-396-AC-1 | A consumer that names only the `qsl_replay` root, the kernel (`quire-exact`) and the QSV declaration API, compiling a unit that declares a record and a tuple through `compile_admitted`, finds both through `composite_declaration` with forms `Record` and `Tuple`, and builds from their keys a closed `TypeEnvironment` with a record field of the tuple's `ValueType::Composite` type and a record value it admits, without decoding, hashing or constructing a key. | Test (TC-1090) |
| FR-396-AC-2 | Over a unit with two same-shaped records and two same-shaped tuples, the context's table equals, per declaration, the spine's own checked package read independently of its checked type nodes: the `composite_type` semantic node's id, the key its checked `TypeEnvironment` gives the declaration, its name and form. A function parameter of a record's type is declared `ValueType::Composite` of that record's admitted key, which differs from its node id. The four keys differ, and a repeated lookup and a recompile return the same declarations. | Test (TC-1090) |
| FR-396-AC-3 | `composite_declaration` refuses `Unknown` for an id no package admitted, `WrongKind` for the unit's enum declaration, union and function, and `Foreign` naming the library's `package_id` for a library record imported directly or through an intermediate library, while every own record and tuple is found. | Test (TC-1090) |
| FR-396-AC-4 | FR-394-AC-1 to FR-394-AC-5 still hold: the enum table, its lookups and refusals, and the `Package` / `Resource` / `Fault` compile refusals are unchanged. | Test (TC-1090) |
| FR-396-AC-5 | A replay witness whose record and tuple values name the admitted kernel keys replays on both arms, and the same witness naming the `composite_type` node ids refuses. | Test (TC-1090) |

## Dependencies

- FR-394 (the admitted-context facade), FR-060 (the T12 boundary), FR-087
  (the cross-package node key), FR-092 (the composite node key), ADR-015 (the
  dependency closure).
