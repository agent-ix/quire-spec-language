---
id: FR-512
title: "Keep operation names, parameters and frame entries unique at intake"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-015
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-103
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-154
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-154
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-208
    type: depends_on
---
# FR-512: Keep operation names, parameters and frame entries unique at intake

## Description

Native-model admission checked operation roles and frame inventories
(`admission.rs:502-598`, FR-015-AC-2): an operation's context is a mapped
object; an operation is unique by (context, name); its parameters and result
are distinct declarations and every Input has a role; the frame's fields are
unique actual object fields; created and deleted objects are unique mapped
objects. The deletion table lists these as UNKNOWN on the spine. FR-103
resolves each frame entry (missing name, wrong meaning) but states nothing
about repeats. This requirement states each legacy rule as the spine rule the
wire allows, and records which are satisfied by construction or by the IR
reader before QSL intake.

The wire fixes four facts that shape every rule below. An operation is a member
of its owner type's `operations[]`, never a record with an owner reference. A
parameter's `name` equals the last segment of its identity (QSpec
`model-complete.md`, Operation parameters), but the semantic-IR reader checks
parameter names by name (`DUPLICATE_PARAM`) and identities by identity
(`DUPLICATE_IDENTITY`), so a repeat can reach QSL as either. The `returns`
object holds only `typeRef`, `decimal`, `multiplicity` and `nullable`; it has
no name. A frame entry written twice is one entry: QSpec's Frames row says so
for `modifies`, `modifies_receiver` and scoped `deletes`, and the reader's frame
schema applies no uniqueness to any of the member lists.

Reader diagnostics: a document the reader refuses admits no declaration and
retains every reader diagnostic (FR-056-AC-19). Of those, a
`DUPLICATE_IDENTITY` diagnostic is reported as the QSL refusal
`invalid_model_binding`/`conflicting-binding` that FR-056 assigns to it; every
other reader diagnostic, such as `DUPLICATE_PARAM`, is retained beside it as
reader detail and is not a second QSL refusal. An intake check of this
requirement on a member that carries a retained reader diagnostic does not run.

## Inputs

- A domain package operation: its place in its owner type's `operations[]`,
  its `name`, `params`, `returns` and its frame (`modifies`, `modifies_receiver`,
  `creates`, `deletes`, `deletes_receiver`, `deletes_parameters`), as FR-103 and
  FR-154 read them.

## Outputs

- An admitted operation, or the first refusal below.

## Behavior

- **Context is a mapped object.** The owner of an operation is the type whose
  `operations[]` holds it. A population has no `operations` member, so an
  operation cannot be owned by a population in a document the reader admits:
  the reader refuses the member and intake adds no refusal. An operation held
  by a type whose meaning declares no such operation is refused by that
  meaning's rule (QSpec FR-208, "Other meanings"): an event type or namespace
  as `invalid_model_binding`/`malformed-declaration` at the operation, a record
  value type as `unsupported_construct`/`declaration-form`. The owner of an
  admitted operation is an object type, a process, a state machine or a
  persistence interface.
- **Unique by (owner, name).** The operation's name is its `name` member; the
  identity suffix does not define it. Two operations of one owner that share one
  identity are refused by the reader and reported as
  `invalid_model_binding`/`conflicting-binding` at the shared identity, path
  `TypeDefinition.operations[k]` for the second. Two operations of one owner with
  equal `name` and distinct identities, which the reader admits, SHALL refuse
  `ambiguous_declaration`/`ambiguous-name` at the second operation's `name`,
  naming both. An operation of a subtype with the name of an inherited one is
  not a repeat; it is a redefinition (FR-081, FR-103-AC-10).
- **Distinct parameters.** Two parameters of one operation with equal names are
  refused by the reader, as follows. If they share one identity, the QSL refusal
  is `invalid_model_binding`/`conflicting-binding` at the shared identity, path
  `TypeDefinition.operations[k].params[s]` for the second, with the reader's
  `DUPLICATE_PARAM` retained beside it. If their identities differ, the reader
  alone refuses at `/ir/types/i/operations/k/params/s/name` with
  `DUPLICATE_PARAM`; intake adds no refusal, admits no declaration, and the
  name/identity check below does not run for that member. A parameter whose
  `name` differs from the last segment of its identity, with no equal name
  among its operation's parameters, is `invalid_model_binding`/
  `malformed-declaration` at the parameter's `name` (QSpec Operation
  parameters row).
- **Result name.** The wire gives the result no name, and a `post` clause binds
  it by the word `result`, which FR-104 admits only in a `post` clause of an
  operation that declares `returns` and otherwise refuses as
  `wrong_snapshot`/`wrong-anchor`. So that the word names one thing in every
  operation, intake SHALL refuse a parameter named `result` as
  `ambiguous_declaration`/`ambiguous-name` at its `name`, whether or not the
  operation declares `returns`. This is the successor of the legacy rule that a
  parameter must not take the result binder's name; it needs no wire addition.
- **Every Input has an operation.** The record shape holds a parameter only
  inside its operation's `params`, so no parameter can be without an operation
  role; this rule is satisfied by construction and needs no check (the legacy
  check guarded a separate role inventory, which the spine does not have).
- **Frame entries unique by collapse.** A resolved entry written twice in one
  member list is one entry and SHALL NOT be refused. This covers every list:
  `modifies` and `modifies_receiver` (the scope of a `modifies` entry is
  `every-object` in `modifies` and `receiver` in `modifies_receiver`,
  FR-154), `deletes_parameters`, and also `creates` and type-wide `deletes`.
  QSpec's Frames row names only the first three; `creates` and `deletes` are
  collapsed for two stated reasons: the reader's frame schema applies the same
  no-uniqueness rule to all member lists, and the checked package requires
  `creates` and `deletes` duplicate-free (QSpec FR-340-AC-4), which collapse
  satisfies and a refusal would contradict the "written twice is one entry"
  rule the same schema states. The operation's `OperationEffect` holds the
  entry once, at the position of its first occurrence, so the frame's fields
  and created and deleted objects are unique by construction. The same
  declaration in `creates` and in `deletes` is not a repeat. A field in
  `modifies` and in `modifies_receiver` is FR-154's `conflicting-binding`, not
  a repeat.
- **Order.** For one operation the first failing check SHALL be the one
  reported, in this order: a reader refusal (including the same-identity and
  parameter equal-name collisions above); the owner rule; the operation name; the
  parameter name/identity check; the result name; then the frame, in the order
  FR-103 and FR-154 give.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-512-AC-1 | In the ConfigVersion package, an `operations` member on the population `config_history` is refused by the semantic-IR reader, whose diagnostic is retained, and no declaration is admitted. | Test (TC-1307) |
| FR-512-AC-2 | An operation held by an event type refuses `invalid_model_binding`/`malformed-declaration` at the operation; the same operation held by an object type is admitted. | Test (TC-1307) |
| FR-512-AC-3 | An operation held by a record value type refuses `unsupported_construct`/`declaration-form` at the operation. | Test (TC-1307) |
| FR-512-AC-4 | Two `Counter` operations that share the identity `.../Counter/bump` refuse `invalid_model_binding`/`conflicting-binding` at that identity, path `TypeDefinition.operations[1]`. | Test (TC-1307) |
| FR-512-AC-5 | Two `Counter` operations named `bump` with the distinct identities `.../Counter/bump` and `.../Counter/bump2` refuse `ambiguous_declaration`/`ambiguous-name` at the second operation's `name`, naming both. | Test (TC-1307) |
| FR-512-AC-6 | `Sub.bump` redefining `Counter.bump` through `redefines` is admitted. | Test (TC-1307) |
| FR-512-AC-7 | Parameters `a` and `a` with one identity refuse `invalid_model_binding`/`conflicting-binding` at the shared identity, path `TypeDefinition.operations[k].params[1]`, with the reader's `DUPLICATE_PARAM` retained and no other refusal. | Test (TC-1307) |
| FR-512-AC-8 | Parameters both named `a` with the identities `.../a` and `.../b` give the reader's `DUPLICATE_PARAM` at `/ir/types/i/operations/k/params/1/name` and no QSL intake refusal and no declaration. | Test (TC-1307) |
| FR-512-AC-9 | A parameter named `x` with identity `.../b`, and no equal name among the operation's parameters, refuses `invalid_model_binding`/`malformed-declaration` at its `name`. | Test (TC-1307) |
| FR-512-AC-10 | A parameter named `result` on an operation that declares `returns` refuses `ambiguous_declaration`/`ambiguous-name` at its `name`. | Test (TC-1307) |
| FR-512-AC-11 | A parameter named `result` on an operation with no `returns` refuses `ambiguous_declaration`/`ambiguous-name` at its `name`. | Test (TC-1307) |
| FR-512-AC-12 | `modifies [Gauge/g, Gauge/g]` and `modifies_receiver [Gauge/g, Gauge/g]` are each admitted with an effect equal to the one-entry list; `modifies [Gauge/g, Gauge/h]` holds both in declared order. | Test (TC-1307) |
| FR-512-AC-13 | `creates [A, A]`, `deletes [B, B]` and `deletes_parameters [p, p]` (for a `Reference` parameter `p`) are each admitted with an effect equal to the one-entry list. | Test (TC-1307) |
| FR-512-AC-14 | `creates [A]` with `deletes [A]` is admitted; `modifies [Gauge/g]` with `modifies_receiver [Gauge/g]` refuses `invalid_model_binding`/`conflicting-binding` (FR-154-AC-4). | Test (TC-1307) |
| FR-512-AC-15 | An operation with repeated parameters `a`, `a` and an unresolved `modifies` entry reports the `conflicting-binding` refusal only; one with a name/identity-mismatched parameter and an unresolved `modifies` entry reports the `malformed-declaration` refusal only; one with a `result` parameter and an unresolved `modifies` entry reports the `ambiguous-name` refusal only. | Test (TC-1307) |
| FR-512-AC-16 | A `Counter` with two operations named `bump` of distinct identities, the second also having a parameter `result`, reports the operation-name refusal only; an event type holding two operations of one name reports the owner refusal only; a record value type holding two operations of one name reports the owner refusal only. | Test (TC-1307) |
| FR-512-AC-17 | Admitting the ConfigVersion package with its operations and frame entries reordered in the document still gives equal `PackageDeclarations` (FR-103-AC-5), and the refusals of AC-4 to AC-11 and AC-15 to AC-16 name the same entries under the reordering of unrelated records. | Test (TC-1307) |

## Dependencies

- **Upstream:** FR-103 (frame resolution), FR-056 (intake order, same-identity
  collisions and retained reader diagnostics), FR-154 (QSL receiver scope; QSpec
  operation and frame vocabulary), QSpec FR-208 (meaning rules for an
  operation's owner), QSpec FR-340 (duplicate-free checked frame), FR-104 (the
  `result` word), FR-015-AC-2 (the legacy behavior).
