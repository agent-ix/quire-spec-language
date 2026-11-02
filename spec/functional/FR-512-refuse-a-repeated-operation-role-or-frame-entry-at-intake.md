---
id: FR-512
title: "Refuse a repeated operation role or frame entry at intake"
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
# FR-512: Refuse a repeated operation role or frame entry at intake

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
parameter's `name` equals the last segment of its identity, so equal parameter
names under one operation are equal identities (QSpec `model-complete.md`,
Operation parameters). The `returns` object holds only `typeRef`, `decimal`,
`multiplicity` and `nullable`; it has no name. A frame entry written twice
with one scope is one entry (QSpec Frames; FR-154).

When the IR reader or FR-056's identity rule refuses a case, that refusal
governs: FR-056 ends intake on a reader refusal and refuses every
same-identity collision as `invalid_model_binding`/`conflicting-binding`, so
this requirement adds a check only for what the reader admits.

## Inputs

- A domain package operation: its place in its owner type's `operations[]`,
  its `name`, `params`, `returns` and its frame (`modifies`, `modifies_receiver`,
  `creates`, `deletes`), as FR-103 and FR-154 read them.

## Outputs

- An admitted operation, or the first refusal below.

## Behavior

- **Context is a mapped object.** The owner of an operation is the type whose
  `operations[]` holds it. A population has no `operations` member, so an
  operation cannot be owned by a population in a document the reader admits:
  the semantic-IR reader refuses the member (`operations` is not a member of a
  population) and, under FR-056-AC-19, retains that reader diagnostic and
  admits no declaration; intake adds no refusal and `config_history` is never
  named as an owner. An operation held by a type of a meaning that declares no
  such operation is refused by the meaning's own rule, which this requirement
  does not restate: an event type or namespace as
  `invalid_model_binding`/`malformed-declaration` at the operation, a record
  value type as `unsupported_construct`/`declaration-form` (QSpec FR-208).
  The owner of an admitted operation is therefore an object type, a process, a
  state machine or a persistence interface, and FR-103 assigns it there.
- **Unique by (owner, name).** The operation's name is its `name` member; the
  identity suffix does not define it. Two operations of one owner that share one
  identity are the same-identity collision and are refused by the reader and
  FR-056 as `invalid_model_binding`/`conflicting-binding` at the shared
  identity with path `TypeDefinition.operations[k]` for the second; the checks
  below do not run for it. Two operations of one owner with equal `name`
  and distinct identities, which the reader admits, SHALL refuse
  `ambiguous_declaration`/`ambiguous-name` at the second operation's `name`,
  naming both. An operation of a subtype with the name of an inherited one is
  not a repeat; it is a redefinition (FR-081, FR-103-AC-10).
- **Distinct parameters.** Two parameters of one operation with equal names
  share one identity, so the reader and FR-056 refuse them as
  `invalid_model_binding`/`conflicting-binding` at the shared identity, path
  `TypeDefinition.operations[k].params[s]` for the second (the reader's
  `DUPLICATE_PARAM` is subsumed by that one refusal). Intake therefore never
  sees two equal parameter names; a parameter whose `name` differs from the
  last segment of its identity is `invalid_model_binding`/
  `malformed-declaration` under QSpec's Operation parameters row, not a repeat.
- **Result name.** The wire gives the result no name, and a `post` clause binds
  it by the reserved word `result` (FR-104). A parameter of an operation that
  declares `returns` and is named `result` SHALL refuse
  `ambiguous_declaration`/`ambiguous-name` at that parameter's `name`, naming
  the parameter and the operation's result, because the word would name both.
  A parameter named `result` of an operation with no `returns` is admitted.
  This is the successor of the legacy rule that a parameter must not take the
  result binder's name; no wire addition is needed.
- **Every Input has an operation.** The record shape holds a parameter only
  inside its operation's `params`, so no parameter can be without an operation
  role; this rule is satisfied by construction and needs no check (the legacy
  check guarded a separate role inventory, which the spine does not have).
- **Frame entries unique.** The scope of a `modifies` entry is `every-object`
  for an entry in `modifies` and `receiver` for one in `modifies_receiver`
  (FR-154); a `creates` or `deletes` entry has no field scope. After FR-103
  resolves the entries, a resolved entry written twice in one list (the same
  declaration-and-field pair in `modifies`, in `modifies_receiver`, or the same
  declaration in `creates`, in `deletes`) is one entry and SHALL NOT be refused,
  as the model contract and the semantic-IR schema state. The operation's
  `OperationEffect` holds the entry once, at the position of its first
  occurrence, so the frame's declared fields and created and deleted objects
  are unique by construction. The same declaration in `creates` and in
  `deletes` is not a repeat. A field in `modifies` and in `modifies_receiver`
  is not a repeat either: it is FR-154's `conflicting-binding`, which this
  requirement does not restate.
- **Order.** For one operation the first failing check SHALL be the one
  reported, in this order: a reader refusal (which includes the
  same-identity parameter and operation collisions above); the owner rule; the
  operation name; the result name; then the frame, in the order FR-103 and
  FR-154 give.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-512-AC-1 | In the ConfigVersion package, an `operations` member on the population `config_history` is refused by the semantic-IR reader, whose diagnostic is retained (FR-056-AC-19), and no declaration is admitted; an operation held by an event type refuses `invalid_model_binding`/`malformed-declaration` at the operation, and one held by an object type is admitted. | Test (TC-1307) |
| FR-512-AC-2 | Two `Counter` operations that share the identity `.../Counter/bump` refuse `invalid_model_binding`/`conflicting-binding` at that identity, path `TypeDefinition.operations[1]`; two `Counter` operations named `bump` with the distinct identities `.../Counter/bump` and `.../Counter/bump2` refuse `ambiguous_declaration`/`ambiguous-name` at the second operation's `name`, naming both; `Sub.bump` redefining `Counter.bump` through `redefines` admits. | Test (TC-1307) |
| FR-512-AC-3 | An operation with parameters `a` and `a` refuses `invalid_model_binding`/`conflicting-binding` at the shared identity, path `TypeDefinition.operations[k].params[1]`, and no other refusal; parameters named `a` with the identities `.../a` and `.../b` refuse `invalid_model_binding`/`malformed-declaration`; a parameter `result` on an operation that declares `returns` refuses `ambiguous_declaration`/`ambiguous-name` at its `name`, and the same parameter on an operation without `returns` admits. | Test (TC-1307) |
| FR-512-AC-4 | `modifies [Gauge/g, Gauge/g]` is admitted and its effect equals that of `modifies [Gauge/g]`; so are `creates [A, A]` and `deletes [B, B]` against `creates [A]` and `deletes [B]`; `modifies [Gauge/g, Gauge/h]` holds both in declared order; `creates [A]` with `deletes [A]` admits; `modifies [Gauge/g]` with `modifies_receiver [Gauge/g]` refuses `invalid_model_binding`/`conflicting-binding` (FR-154-AC-4). | Test (TC-1307) |
| FR-512-AC-5 | An operation with a repeated parameter `a` and an unresolved `modifies` entry reports the `conflicting-binding` refusal only; one with a `result` parameter and `returns` and an unresolved `modifies` entry reports the `ambiguous-name` refusal only; one held by an event type whose second operation repeats a name reports the owner refusal only. | Test (TC-1307) |
| FR-512-AC-6 | Admitting the ConfigVersion package with its operations and frame entries reordered in the document still gives equal `PackageDeclarations` (FR-103-AC-5), and the refusals of AC-1 to AC-3 and AC-5 name the same entries under the reordering of unrelated records. | Test (TC-1307) |

## Dependencies

- **Upstream:** FR-103 (frame resolution), FR-056 (intake order and
  same-identity collisions), FR-154 (QSL receiver scope; QSpec operation and
  frame vocabulary), QSpec FR-208 (meaning rules for an operation's owner),
  FR-104 (the `result` binder), FR-015-AC-2 (the legacy behavior).
