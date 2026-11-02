---
id: FR-510
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
  - target: ix://agent-ix/quire-specification/FR-154
    type: depends_on
---
# FR-510: Refuse a repeated operation role or frame entry at intake

## Description

Native-model admission checked operation roles and frame inventories
(`admission.rs:502-598`, FR-015-AC-2): an operation's context is a mapped
object; an operation is unique by (context, name); its parameters and result
are distinct declarations and every Input has a role; the frame's fields are
unique actual object fields; created and deleted objects are unique mapped
objects. The deletion table lists these as UNKNOWN on the spine. FR-103
resolves each frame entry (missing name, wrong meaning) but states nothing
about repeats. This requirement states each legacy rule as a spine rule, and
records which one is satisfied by construction.

## Inputs

- A domain package operation record: its owner, name, `params`, `returns` and
  its frame (`modifies`, `creates`, `deletes`), as FR-103 reads them.

## Outputs

- An admitted operation, or the first refusal below.

## Behavior

- **Context is a mapped object.** An operation SHALL be declared on an object
  type or process of the package (FR-103); an operation record whose owner
  names any other declaration refuses `invalid_model_binding`/
  `malformed-declaration` at the owner reference. (FR-103 already assigns the
  operation to its owner; this states the refusal.)
- **Unique by (owner, name).** Two operations of one owner with equal names
  SHALL refuse `ambiguous_declaration`/`ambiguous-name` at the second,
  naming both. An operation of a subtype with the name of an inherited one is
  not a repeat; it is a redefinition (FR-081, FR-103-AC-10).
- **Distinct parameters.** Two parameters of one operation with equal names,
  or a parameter named as the operation's result binder, SHALL refuse
  `ambiguous_declaration`/`ambiguous-name` at the second.
- **Every Input has an operation.** The record shape holds a parameter only
  inside its operation's `params`, so no parameter can be without an operation
  role; this rule is satisfied by construction and needs no check (the legacy
  check guarded a separate role inventory, which the spine does not have).
- **Frame entries unique.** After FR-103 resolves the entries of `modifies`,
  then `creates`, then `deletes`, a repeated entry within one list (the same
  resolved declaration-and-field pair and scope in `modifies`; the same
  resolved declaration in `creates` or in `deletes`) SHALL refuse
  `invalid_model_binding`/`malformed-declaration` at the second entry, naming
  both positions, retaining the operation's IR node identity, the artifact id
  and the span. The same declaration in `creates` and in `deletes` is not a
  repeat.
- The first failing check in the order owner, name, parameters, `modifies`,
  `creates`, `deletes` SHALL be the one reported.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-510-AC-1 | In the ConfigVersion package, an operation record whose owner names the population `config_history` refuses `invalid_model_binding`/`malformed-declaration` at the owner reference. | Test (TC-1305) |
| FR-510-AC-2 | Two operations named `bump` on `Counter` refuse `ambiguous_declaration`/`ambiguous-name` at the second, naming both; `Sub.bump` redefining `Counter.bump` through `redefines` admits. | Test (TC-1305) |
| FR-510-AC-3 | An operation with parameters `a` and `a`, and one with a parameter `r` and a result binder `r`, each refuse `ambiguous_declaration`/`ambiguous-name` at the second. | Test (TC-1305) |
| FR-510-AC-4 | `modifies [Gauge/g, Gauge/g]` refuses `invalid_model_binding`/`malformed-declaration` at the second entry naming positions 0 and 1; `modifies [Gauge/g, Gauge/h]` admits; `creates [A, A]` and `deletes [B, B]` each refuse the same way; `creates [A]` with `deletes [A]` admits. | Test (TC-1305) |
| FR-510-AC-5 | An operation with a repeated parameter and a repeated `modifies` entry reports the parameter refusal only; one with a repeated `modifies` and a repeated `creates` reports the `modifies` one only. | Test (TC-1305) |
| FR-510-AC-6 | Admitting the ConfigVersion package with its operations and frame entries reordered in the document still gives equal `PackageDeclarations` (FR-103-AC-5), and the refusals of AC-2 to AC-4 name the same entries under the reordering of unrelated records. | Test (TC-1305) |

## Dependencies

- **Upstream:** FR-103 (frame resolution), FR-056 (intake order), FR-154
  (QSpec operation and frame vocabulary), FR-015-AC-2 (the legacy behavior).
