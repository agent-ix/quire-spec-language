---
id: FR-304
title: "Check an authored model-to-implementation abstraction relation at S3"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-031
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-017
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-089
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-303
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-353
    type: depends_on
  - target: ix://agent-ix/quire-specification/AD-006
    type: depends_on
---
# FR-304: Check an authored model-to-implementation abstraction relation at S3

## Description

When a unit declares one or more abstraction relations, the compiler SHALL
parse each at S2 as QSpec FR-450's `abstraction-decl` and check them at S3,
as `Relation` family declarations, into one `CheckedAbstractionRelation`
holding at most one binding per model key across all of the unit's
declarations. It SHALL refuse each binding QSpec FR-450 refuses, with the
code, cause and order QSpec FR-450 gives.

## Inputs

- The unit's `abstraction-decl` declarations, in QSpec FR-450's surface
  spelling.
- The admitted domain packages of the unit's `model` declarations
  ([FR-056](FR-056-admit-domain-package-model-declarations.md)), with their
  object types, fields, populations and operations.

## Outputs

A `CheckedAbstractionRelation` in the `check` core, or the located refusals
QSpec FR-450 names.

## Behavior

### Placement

The relation is authored in QSL source as a `Relation` family declaration
(ADR-012 §3, ADR-017 AR-1), parsed at S2, checked at S3 and carried in the
checked package. It is a declaration of its own, distinct from a model
refinement declaration, with its own checked type.

- S2 SHALL parse each `abstraction-decl` into its relation form, refusing as
  QSpec FR-450-AC-3 states: `unsupported_construct` at the declaration when
  the package does not select `quire.model.complete/v1`, and
  `invalid_syntax` at the offending token for a declaration with no binding
  or a binding whose Rust spelling is not quoted.
- S3 SHALL check every `abstraction-decl` of the unit into the one
  `CheckedAbstractionRelation`. Key uniqueness spans all of them (QSpec
  FR-450 rule 5).

### Keys and binding values

Each binding SHALL be keyed and valued as QSpec FR-450 "Binding forms"
states, in the `check` core types ADR-017 AR-2 names:

| Model element | Key | Binding value |
| --- | --- | --- |
| Object type | the object type's `DeclarationKey { package, node }` | `ObjectBinding { rust_type: RustPath, fields: field Identifier → RustField }` |
| Population | the population declaration's `DeclarationKey`, the `population_key` [FR-089](FR-089-carry-population-identity-across-the-kernel-boundary.md) mints `PopulationId` from | `PopulationBinding { collection: RustPath }` |
| Operation frame | `OperationKey { declaring: DeclarationKey, operation: Identifier }` | `FrameBinding { function: RustPath, receiver: RustReceiver, parameters: operation parameter Identifier → Rust parameter identifier }` |

The `DeclarationKey` is QSpec FR-353's AD-006 effective-declaration key,
ADR-013 O-03. `RustPath`, `RustField`, `RustReceiver` and the Rust
parameter identifier hold QSpec FR-450 "Rust spellings" as lexical keys
that compare by bytes. The checker checks their syntax by QSpec FR-450 and
resolves none of them against Rust code.

### Refusals

The checker SHALL apply QSpec FR-450 "Refusals" rules 1 to 5, in their
order and in source order of the bindings, with the codes, causes and named
members QSpec FR-450 gives. It SHALL report every refused binding of every
declaration and produce no checked package when any binding refuses.

### Partial relations

A relation that binds a subset of the model's elements SHALL check. The
unbound-element refusal is scoped to the item that references the element
([FR-307](FR-307-export-the-bindings-each-item-references.md)).

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-304-AC-1 | A relation binding the ConfigVersion object type with two fields, its population and its `attemptUpdate` operation checks into a `CheckedAbstractionRelation` with exactly three bindings, keyed by the object type's and the population's `DeclarationKey`s and by `OperationKey { ConfigVersion, attemptUpdate }`. | Test (TC-797) |
| FR-304-AC-2 | A relation that binds only the ConfigVersion object type, leaving its population and operations unbound, checks. | Test (TC-797) |
| FR-304-AC-3 | A binding whose key names an object type absent from the domain package, and a binding whose `OperationKey` names an operation its type does not declare, each refuse `missing_declaration`/`missing-name`, naming the key and its owning `DomainPackageRef` identity. | Test (TC-798) |
| FR-304-AC-4 | Two equal bindings for one key, and two different bindings for one key, each refuse `invalid_model_binding`/`conflicting-binding`, naming both bindings and the key. | Test (TC-799) |
| FR-304-AC-5 | A `FrameBinding` whose `parameters` map omits a declared parameter, one that names an undeclared parameter, one that maps two parameters to one Rust parameter, and an `ObjectBinding` naming an undeclared field each refuse `invalid_model_binding`/`malformed-declaration`, naming the entry; an `ObjectBinding` for a subtype naming a field its supertype declares checks. | Test (TC-800) |
| FR-304-AC-6 | The `RustPath` segments `type` and `9lives` and the tuple-field index `01` each refuse `invalid_model_binding`/`malformed-declaration`, naming the segment and its span; the segments `r#type` and `config_store`, the index `0` and the receivers `self` and `store` are admitted; the receiver `0` refuses `invalid_model_binding`/`malformed-declaration`. QSpec TC-410's and TC-411's cases each give their verdict. | Test (TC-801) |
| FR-304-AC-7 | A unit with two `abstraction-decl`s, one binding the ConfigVersion object type and the other its population, checks into one `CheckedAbstractionRelation` with both bindings; when the second declaration also binds the ConfigVersion object type, with an equal or a different value, the unit refuses `invalid_model_binding`/`conflicting-binding`, naming both bindings and the key. | Test (TC-810) |
| FR-304-AC-8 | Compiling source text holding FR-304-AC-1's relation as an `abstraction-decl` yields the same `CheckedAbstractionRelation` as AC-1; the same declaration in a package that does not select `quire.model.complete/v1` refuses `unsupported_construct` at the declaration's span; a declaration with no binding, and a binding with an unquoted Rust path, refuse `invalid_syntax` at the offending token. | Test (TC-811) |

## Dependencies

- **Upstream:** QSpec FR-353 owns the relation's semantics; QSpec FR-450
  owns the surface spelling, the Rust spellings and the refusals; ADR-017
  AR-1 to AR-3 fix its placement and its `check` core types; ADR-013 O-03
  owns the declaration key;
  [FR-303](FR-303-keep-the-model-correspondence-one-to-one.md) keeps the
  key resolution one-to-one.
- **Downstream:** [FR-305](FR-305-relate-a-frame-binding-to-its-operation-s-frame.md),
  [FR-306](FR-306-carry-the-abstraction-relation-in-the-checked-package.md)
  and [FR-307](FR-307-export-the-bindings-each-item-references.md).
- ADR-020 keeps model-to-model refinement a separate declaration with its own
  checked type; this relation shares no key or check with it.

## References

- ADR-017 §3 AR-1 to AR-3; ADR-020 §7 MC-2 and MC-3.
- QSpec FR-450, FR-451 (specification ticket STD-116, Q-1, Q-2).
- Linear QSL-388 (specification), QSL-36 (implementation).
