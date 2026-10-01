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

When a unit declares an abstraction relation, the S3 checker SHALL check it
as a `Relation` family declaration into one `CheckedAbstractionRelation`
holding at most one binding per model key, and SHALL refuse each binding
whose key, value or syntax is not admissible, naming the key.

## Inputs

- The unit's abstraction relation declaration, in the surface spelling
  QSpec FR-450 gives the QSpec FR-353 relation, parsed at S2 into its
  relation form.
- The admitted domain packages of the unit's `model` declarations
  ([FR-056](FR-056-admit-domain-package-model-declarations.md)), with their
  object types, fields, populations and operations.

## Outputs

A `CheckedAbstractionRelation` in the `check` core, or a located refusal per
inadmissible binding.

## Behavior

### Placement

The relation is authored in QSL source as a `Relation` family declaration
(ADR-012 §3, ADR-017 AR-1), parsed at S2, checked at S3 and carried in the
checked package. It is a declaration of its own, distinct from a model
refinement declaration, with its own checked type.

### Keys and binding values

Each binding SHALL be keyed and valued as ADR-017 AR-2 states:

| Model element | Key | Binding value |
| --- | --- | --- |
| Object type | the object type's `DeclarationKey { package, node }` | `ObjectBinding { rust_type: RustPath, fields: field Identifier → RustField }`, where a field is any effective field member of the object type, own or inherited |
| Population | the population declaration's `DeclarationKey`, the `population_key` [FR-089](FR-089-carry-population-identity-across-the-kernel-boundary.md) mints `PopulationId` from | `PopulationBinding { collection: RustPath }`, the path from the implementation state root to the collection |
| Operation frame | `OperationKey { declaring: DeclarationKey, operation: Identifier }` | `FrameBinding { function: RustPath, receiver: RustReceiver, parameters: operation parameter Identifier → Rust parameter identifier }` |

The `DeclarationKey` is QSpec FR-353's AD-006 effective-declaration key
(domain package identity, IR node identity, `sha256-jcs`), ADR-013 O-03. A
`RustPath` is a non-empty sequence of Rust identifiers. A `RustField` is a
Rust identifier or a tuple-field index. A `RustReceiver` is exactly `self`,
the bound function's method receiver, or one Rust identifier naming one of
the function's parameters (QSpec FR-450). A Rust identifier is `IDENTIFIER` of
the Rust Reference for edition 2021: a non-keyword identifier or a raw
identifier such as `r#type`. A tuple-field index is a decimal integer with no
leading zero. These are lexical keys that compare by bytes. The checker checks
their syntax and resolves none of them against Rust code.

### Refusals

The checker SHALL refuse, naming the key:

- a model key that resolves to no admitted declaration of the selected
  domain package, or an `OperationKey` whose declaring type declares no such
  operation: `missing_declaration`/`missing-name`, naming the key and its
  owning `DomainPackageRef` identity;
- two bindings for one key, equal or not: `invalid_model_binding`/
  `conflicting-binding`, naming both bindings and the key. Two bindings are
  equal when their values are equal member by member, by bytes;
- a `FrameBinding` whose `parameters` map names a parameter the operation
  does not declare, omits a parameter it declares, or maps two parameters to
  one Rust parameter, or an `ObjectBinding` whose `fields` map names a name that
  is not an effective field member of the object type, own or inherited: `invalid_model_binding`/`malformed-declaration`,
  naming the entry;
- a `RustPath`, `RustField` or `RustReceiver` segment that is not of its
  position's syntax as defined above: `invalid_model_binding`/
  `malformed-declaration`, naming the segment and its span.

The checker reports every refused binding of the relation and produces no
checked package when any binding refuses.

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
| FR-304-AC-6 | The `RustPath` segments `type` and `9lives` and the tuple-field index `01` each refuse `invalid_model_binding`/`malformed-declaration`, naming the segment and its span; the segments `r#type` and `config_store`, the index `0` and the receivers `self` and `store` are admitted; the receiver `0` refuses `invalid_model_binding`/`malformed-declaration`. | Test (TC-801) |

## Dependencies

- **Upstream:** QSpec FR-353 owns the relation's semantics and refusal codes;
  QSpec FR-450 gives the surface spelling; ADR-017 AR-1 to AR-3 fix its
  placement, keys, values and S3 refusals; ADR-013 O-03 owns the declaration
  key; [FR-303](FR-303-keep-the-model-correspondence-one-to-one.md) keeps the
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
