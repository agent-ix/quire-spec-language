---
id: FR-103
title: "Admit a domain package's operations and their frames on the spine"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-003
    type: implements
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-084
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-154
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-340
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-322
    type: depends_on
---
# FR-103: Admit a domain package's operations and their frames on the spine

## Description

When a `1-draft` unit's `model M` selects a domain package whose object types
or processes declare operations, spine intake (ADR-011 I1) SHALL read each
operation's `frame` into the operation's `OperationEffect`, and the E3
assembler SHALL declare each operation on its object type, with its
parameters, its result and its effect, so that a `pre` or `post` clause can
name it as `M::T::op` (FR-104).

This amends FR-056's Complete-V1 paragraph, which this PR edits to match.
Today intake refuses any operation whose frame declares an entry
(`qsl-semantics/src/model/intake.rs:1263-1289`,
`unsupported_at(.., "operation.frame")`), and the assembler refuses every
`OperationMember` record as `AssemblyCause::UnsupportedModelMember` (the match
arm at `qsl-semantics/src/check/assemble.rs:405-409`), so no unit can use a
model that has an operation. This is the `StateModel` share of ADR-012 §15.3.

## Inputs

- One admitted domain package (FR-056, QSpec FR-154), Semantic IR 2.0.0.
- `ModelNormalizationLimitsV1`.

## Outputs

- For each operation member: an `OperationMemberRecord` whose `effect` holds
  the `DeclarationKey`s its frame names, in each member's declared order. In
  the compiled unit's selected model view, the assembler SHALL set its
  `has_body` value true when a retained unit `OperationBodyForm` (FR-102)
  resolves to that operation's `DeclarationKey`, and false when no such
  declaration exists.
- In the assembled `PackageDeclarations`: each object type or process `M::T`
  with its operations, each operation with its name, its parameters (name and
  value type, in declared order), its result value type or none, and its
  effect.
- Each population record of the package, with its member types and its
  declared extent, available to the checker and to observation admission
  (FR-106) by its declaration key.

## Behavior

- Intake SHALL resolve each `modifies` entry to a field declaration of the
  package, and each `creates` and `deletes` entry to an object type or process
  declaration, under QSpec `model-complete.md`'s Frames row, checking the
  entries of `modifies`, then `creates`, then `deletes`, each in document
  order, and reporting the first that fails.
- If a frame entry names no declaration of the package, then intake SHALL
  refuse `missing_declaration`/`missing-name` at the entry, retaining the
  operation's IR node identity, the entry, the artifact id and the span.
- If a `modifies` frame entry names a relationship, then intake SHALL refuse
  `unknown_required_feature`/`unsupported-feature` at the entry.
- If a frame entry names a declaration of any other meaning, then intake SHALL
  refuse `invalid_model_binding`/`malformed-declaration` at the entry, with
  the same retained data.
- Intake SHALL read an absent frame, or one whose three members are all
  empty, as the empty effect, as it does today.
- The assembler SHALL type each parameter and the result by FR-056's field
  rule: a native `Boolean` or `Integer`, a scalar type as its `Int[lower,
  upper]`, an object type as `Reference<M::T>`, or a record value type as
  `M::R`, with presence and multiplicity applied the same way. A parameter or result type the type
  environment cannot hold SHALL refuse
  `unknown_required_feature`/`unsupported-feature` at the `model`
  declaration, as a field of that type does.
- The assembler SHALL declare an operation on the object type or process that
  owns it. The operation is visible on a subtype through FR-081's effective
  view, as a field is, and keeps its declaring type as its owner (FR-105
  anchors it there).
- The S3 assembler SHALL preserve each resolved unit `operation-body`
  expression and its source provenance, and each unit `pre` and `post`
  `StateClauseForm` as a contract clause. It SHALL associate the body and
  clauses with the resolved operation's `DeclarationKey` and keep them
  available to checking and dispatch linking. It SHALL bind the body's
  receiver-less parameter names positionally to the operation's parameters in
  signature order, as required by the QSpec Complete-V1 grammar, and type its
  body against the operation's declared result. At most one unit
  `operation-body` SHALL resolve to a given `DeclarationKey`; FR-302 defines
  the duplicate refusal. This multiplicity rule applies to executable bodies,
  not to the distinct unit `pre`/`post` `StateClauseForm`s.
- The assembler SHALL declare each admitted record value type beside object
  types and operations, with its effective fields, invariant clauses and
  supertypes, and SHALL allow object and record fields to use that type by
  value. It SHALL carry a record-typed field through checking and checked
  package emission as QSpec FR-322's self-typed `model`/
  `record_value_type` node, including a record field named by a frame's
  `modifies` entry under QSpec FR-340. The record node SHALL keep its
  `ModelOwner`; it is never an object-reference target.
- The assembler SHALL still refuse systems parts, ports and allocations as
  `UnsupportedModelMember`.
- Intake SHALL still shape-check the operation's `pre` and `post` expression
  texts in the domain package and SHALL NOT keep or use them as unit clauses
  or operation bodies. This discard rule applies only to those domain-package
  texts. The separately authored `operation-body` declaration in the
  `1-draft` unit supplies the candidate body; that unit's `pre` and `post`
  `StateClauseForm`s supply operation contract clauses. S2 retains their
  expression trees and source provenance under FR-102, and the S3 assembler
  associates them with the resolved operation identity. An operation with no
  matching unit body has `has_body = false` and is not a dispatch candidate.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-103-AC-1 | The ConfigVersion domain package (TC-458 fixture: object type `ConfigVersion` with `versionNumber: VersionNumber` bound to `Int[0, 1000]` and optional `parent: ConfigVersion`, population `config_history` over `ConfigVersion` with extent `closed`, and operation `attemptUpdate` with no parameters, result `Boolean` and frame `modifies [ConfigVersion/versionNumber]`) admits. The assembled `Config::ConfigVersion` has fields `versionNumber: Int[0, 1000]` and `parent: Option<Reference<Config::ConfigVersion>>`, and operation `attemptUpdate` with no parameters, result `Boolean` and an effect whose `modifies` is exactly the `versionNumber` field key. FR-056's `value-type/v1` scalar reader is verified end to end through `admit_unit`/`PackageDeclarations::assemble` by a dedicated bound-scalar fixture (`qsl-semantics/tests/it/model_operations.rs::bound_integer_value_type_admits_and_assembles`), and the shared `ConfigVersion` fixture this row's own tests use declares `versionNumber` as that same bound `VersionNumber` scalar. | Test |
| FR-103-AC-2 | The same package with `modifies [ConfigVersion/missing]` refuses `missing_declaration`/`missing-name` at that entry; with `modifies [ConfigVersion]` (an object type) or `creates [ConfigVersion/versionNumber]` (a field) it refuses `invalid_model_binding`/`malformed-declaration` at that entry; with a relationship `Supersedes` declared and `modifies [Supersedes]` it refuses `unknown_required_feature`/`unsupported-feature` at that entry; with both `modifies [ConfigVersion/missing]` and `creates [ConfigVersion/versionNumber]` it reports only the `modifies` refusal. No declaration is admitted in any case. | Test |
| FR-103-AC-3 | An operation with a parameter `note: Decimal` refuses `unsupported_construct`/`declaration-form` at the parameter, at I1 (intake) -- the same stage and cause a field of that type refuses at, per this FR's own Behavior ("as a field of that type does"); an operation with parameter `delta: VersionNumber` admits it typed `Int[0, 1000]`. FR-056's `value-type/v1` scalar reader is verified against both halves over the shared `ConfigVersion` fixture. The parameter type is `Decimal`, not this row's earlier `Text`: the FCD schema validator's native-scalar vocabulary has no `Text` entry (it spells the concept `String`), so `Text` cannot reach even I1 through the real pipeline, and QSL's own `NativeValueType` has no `String` variant either -- that cross-repo vocabulary gap is QSL-290, not fixed here. | Test |
| FR-103-AC-4 | A package holding `Address { street: Text[0, 64; nfc] }` and object `Account { address: Address }` assembles both declarations beside an object operation. A clause that reads `deref(a).address.street` checks against the record's effective fields, and checked-package emission types `Account.address` by the self-typed `model`/`record_value_type` node with its `ModelOwner`. No `UnsupportedModelMember` names `Address`, and no `Reference<Address>` is emitted. | Test |
| FR-103-AC-5 | Admission is deterministic: admitting the ConfigVersion package twice, and admitting it with its operation and population records reordered in the document, gives equal `PackageDeclarations` and equal effect key lists. | Test |
| FR-103-AC-6 | A frame modifying `Account.address` and a frame entry naming `Address.street` bind distinct declaration-and-field pairs; a record subtype's inherited field resolves through its effective fields. An operation parameter and result typed `Address` retain record value typing through check and emission; a record node supplied as an object-reference target refuses the QSpec FR-322 derived-shape check. | Test |

## Dependencies

- FR-056 (intake and assembler), FR-081 (effective view), FR-084
  (population records).
- QSpec FR-154, FR-340 and `model-complete.md`'s Frames row.
- filament-core-data's Semantic IR reader already validates an operation's
  `frame` shape
  (`crates/semantic-ir/src/schema.rs:1656-1663`).
