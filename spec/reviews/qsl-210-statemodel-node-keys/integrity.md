---
id: SR-607
title: "Integrity review of the QSL-210 model-owned, StateModel and quantity node-key requirements"
type: SpecReview
analysis: integrity
scope: "Commit ac4f974f: FR-094 against QSpec FR-322 and proposals/checked-package-v2 at e72756f, the model, check and value code it cites, and the FR-092, FR-093 and ADR-013 edits"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-094
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-092
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: reviews
  - target: ix://agent-ix/quire-specification/FR-322
    type: references
---

## Summary

These parts match QSpec at `e72756f`, the code, or both:

- **Application rows.** E4 to E9 conform to their `operation-catalog.json`
  entries:
  - `all_instances`: operator `query`, member `type_argument`, mode `null`,
    result `set`.
  - `lookup`: `query`, `type_argument`, mode `absence`. The result is
    `reference` for `undefined` (R1) and `option` for `empty` (R3).
  - `deref`: operator `deref`, no member, result `inner:0` (M1).
  - `record.project`: `query`, member `field`.
  - `dispatch_call`: operator `call`, member `operation`, result `member`.
- **Application preimage shape.** The application preimages have no `owner`,
  as the published `ApplicationNode` requires.
- **Node forms.** `object_type`, `systems_interface`, `relationship`,
  `reference` and `model_population` are in the closed `ModelNode`,
  `RelationNode`, `CompositeTypeNode` and `DomainNode` enums.
- **`declaration` rules.** Model and relation nodes carry no `declaration`,
  which satisfies `DeclarationTagRules` and `DeclarationOccurrenceRule`.
- **Recorded QSpec conflicts.** Each conflict with a published rule has a QC
  item:
  - `compound_unit` is outside the closed `ScalarTypeNode` enum (QC-26).
  - The base-chain rule would give the `set` family, not `population`
    (QC-25).
  - No v2 member carries `ModelOwner.node` (QC-25).
  - A model-owned node has no `declaration` occurrence (QC-25).
- **ModelOwner.** Its shape matches the published `ModelOwner` exactly. The
  version comes from the `DomainPackageRef` selection, because a
  `DeclarationKey` carries no version (`model/key.rs:75-85`).
- **Reference mapping.** An `EffectiveId`'s preimage holds its original
  `DeclarationKey` (`model/key.rs:193-229`). So `type_identities` is
  injective, and mapping a `Reference<T>` back to one `DeclarationKey` is
  well defined.
- **Unit identity.** Declared and compound `UnitId`s are distinct domains
  (`quire-exact/src/identity.rs:143-197`).
- **Cross-document consistency.** FR-092's amended `owner` row, its
  type-node paragraph, FR-093's model rows, ADR-013 O-04, QC-3, QC-25 and
  QC-26 agree with FR-094.

Three statements contradict the code:

- where compound units are held;
- how many record kinds the exhaustive match covers;
- which refusal admits a `Reference` target.

One reader check has no data to run on, the `member` result over an empty
model body.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-094 says `check` reads a compound unit's terms from "the package's unit table". It also says "the unit table admits every compound unit a checked quantity type names", and treats a missing unit as an internal fault. The code does not work that way. Checking forms each product and quotient unit in the checker's own `UnitScope::formed` table, not in the package `UnitTable`. That table is built per checker and dropped after checking. `UnitTable::declared` holds only declared units. So `a * a` over two `metre` parameters is typed `Quantity(metre^2)`, and that id is in no package table. As written, every product or quotient quantity type refuses as an internal fault. Fix: read terms from the check stage's unit scope (package table, then formed units). Require `check` to keep the formed units until lowering. Restate the refusal as a `UnitId` held by neither table, and correct the admission sentence. | FR-094:209-212, :223, :227-228, :519; `qsl-semantics/src/check/check.rs:544`, `:1640-1648`; `value/quantity.rs:96-103`, `:135-170` |
| FND-002 | medium | FR-094-CON-1 requires the `match` that builds a model node from a domain package record kind to have no `_` arm. `DomainPackageRecord` has nine variants, and the table gives node forms for two: `ObjectType` and `Relationship`. `FieldMember`, `OperationMember`, `ScalarType`, `Component`, `Endpoint`, `Allocation` and `Population` have no stated arm, so an implementer has nothing to write for seven arms. QSpec has forms that may fit some of them: `systems_part`, `systems_port`, `systems_connection`, `systems_allocation` and `relation`/`population`. A `RelationshipRecord` is also an FR-152 Connection candidate, which is QSpec's `systems_connection`. "With `interfaceFeatures`" also leaves `Some([])` undecided. Fix: give every variant an arm. For a record that no checked node can reference (the members, `Component`, `Endpoint`, `Allocation`, `Population`, and `ScalarType` unless a value can be typed by it), the arm is an internal fault. State whether a Connection-candidate relationship is `relationship` or `systems_connection`. Define `Some([])`. | FR-094:97-101, :506; `qsl-semantics/src/model/domain_package.rs:28-37`, `:467-487`; QSpec `schema.json` `ModelNode`, `RelationNode` |
| FND-003 | medium | A model node's body is `aggregate{[]}`. FR-322 checks the `member` result form and the `member_of` constraint against "the type the member names", and the graph holds no member types. The `member` result form is used by `record.project`, `dispatch_call` and `navigate`. A reader therefore cannot check E8's `result_type` (T4, the type of `total`) or E9's (T2, the result of `size`) against M1, and may refuse them as `operator-ineligible`. QSpec publishes `field_declaration` and `operation_declaration` model forms. QC-25 asks about the member's declaring node but not about how a reader finds the member's type. Fix: add to QC-25 how a reader derives a model member's type, either from the locked domain package bytes or from member declaration nodes. Alternatively, give the model body one binding per member, name to type node reference, and add a vector. | FR-094:97-107, :266-268; QSpec FR-322 result forms and `member_of` (FR-322.md:256-262, :277); `schema.json` `ModelNode` |
| FND-004 | medium | FR-094 says the model correspondence "holds exactly the pairs `check` keyed". `PackageDeclarations::model_correspondence` is still a caller-supplied input, which `check` records verbatim. FR-094 does not say whether that field goes, or what happens to a supplied entry that `check` did not key. The field's documented consumer is FR-340 frame subjects. A frame needs an entry for each declaration it names, even when no value node references that declaration. FR-094 builds model nodes only for declarations a checked node references. Fix: make `check` the only writer and remove the caller-supplied field. Build a model node for every declaration that a checked node or a frame names. Alternatively, define how supplied entries merge, with a refusal when an entry conflicts. | FR-094:109-114, :137-142; `qsl-semantics/src/check/check.rs:283-297`; `check/mod.rs:665-676`; `check/refusal.rs:258-275` |
| FND-005 | medium | The clause-kind spelling does not match the code and leaves the invariant owner undefined. `FunctionDeclaration` stores `ClauseKind`, which has four variants including `Postcondition`, not `DeclaredClauseKind`. So the CON-1 exhaustive match has an arm with no spelling. FR-094 spells `"invariant"`, but its owner rule names "the operation member whose clause it is", and an invariant belongs to an object type. `checked_dispatch` never builds an invariant clause function. `FunctionDeclaration::clause` also carries no `DeclarationKey` or `DomainPackageRef`, so the owner can be recovered only from the synthesized label, which FR-094 excludes from the preimage. Fix: match on `DeclaredClauseKind` and keep it on the declaration. Either give an invariant clause function the object type's `ModelOwner` or remove the `"invariant"` spelling. State in Status that A4b adds the owning `DeclarationKey` and the selection to the synthesized declaration. | FR-094:62-63, :173-180, :506; `qsl-forms/src/syntax.rs:205-230`, `:577-594`; `checked_dispatch.rs:887-962` |
| FND-006 | low | The Refusals paragraph credits "`TypeEnvironment::new`'s `UnknownObjectType`" with refusing an unresolved `Reference` target. Type admission refuses an unknown `Reference<T>` target with `DeclarationCause::Type(IllTypedCause::TypeMismatch)` in `type_refusal`. `UnknownObjectType` is the supertype refusal. The environment's `object_types` are also caller-supplied `EffectiveId`s, and no stated rule makes each one a `type_identities` value of an admitted view, so the claim that "reaching one is a fault in `check`" rests on an unstated invariant. Fix: cite `type_refusal`'s `type-mismatch`. State that the type environment's object types are exactly the admitted views' `type_identities` values, or make a mismatch an input refusal. | FR-094:225-228; `qsl-semantics/src/value/declaration.rs:600-613`, `:794-806` |
| FND-007 | low | E7 and E8 depend on QC-24's open `Attribute` item. FR-322 gives a model entity type the family `reference`, and `record.project`'s operand family is `record`. FR-094's Dependencies cite only QC-25 and QC-26 as the QSL proposals behind these vectors. Fix: cite QC-24 beside E7 and E8, and in Dependencies. | FR-094:266-267, :540-545; ADR-013 QC-24; FR-322.md:232-239 |
| FND-008 | low | ADR-013 O-06's Equality row keys a member by "(declaring node id, identifier …)". FR-094 names the receiver's static object type even when a supertype declares the member. So `Sub`'s inherited `total` and `Order`'s `total` are two members for one declared attribute, and O-06's wording says they are one. Fix: amend the O-06 Equality row to "(the node of the receiver's static object type, identifier)", citing FR-094 and QC-25. | ADR-013 O-06 Equality; FR-094:119-122, :516 |

## Resolution

All findings are fixed. FND-001: terms come from the check stage's unit scope (the unit table or the units `UnitScope` formed), which keeps formed units until lowering; the refusal and AC-7 name the unit scope, and Status records where formed units live today. FND-002: the record-kind `match` has one arm per `DomainPackageRecord` variant, unnamed kinds are internal faults (AC-7, TC-417 step 7), a present `interfaceFeatures` (empty included) is `systems_interface`, and a relationship is `relationship` whatever FR-152 role it plays (QC-25). FND-003: QC-25 asks how a reader derives a model member's type, and FR-094 states the gap. FND-004: `check` is the correspondence's only writer, and model nodes are also built for frame subjects (O-08). FND-005: the spelling `match` is over `DeclaredClauseKind`, the invariant owner is the object type, and Status records that A4b supplies the owner and kind. FND-006: the refusal cites `type_refusal`/`TypeMismatch`, and FR-094 states that the type environment's object types equal the views' `type_identities`. FND-007: FR-094 cites QC-24 for the `Attribute` projection. FND-008: ADR-013 O-06's Equality row names the receiver's static object type for model `field` and `operation` members.
