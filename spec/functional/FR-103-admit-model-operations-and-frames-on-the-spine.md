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
---
# FR-103: Admit a domain package's operations and their frames on the spine

## Description

When a `1-draft` unit's `model M` selects a domain package whose object types
declare operations, spine intake (ADR-011 I1) SHALL read each operation's
`frame` into the operation's `OperationEffect`, and the E3 assembler SHALL
declare each operation on its object type, with its parameters, its result and
its effect, so that a `pre` or `post` clause can name it as `M::T::op`
(FR-104).

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
  the `DeclarationKey`s its frame names, in each member's declared order.
- In the assembled `PackageDeclarations`: each object type `M::T` with its
  operations, each operation with its name, its parameters (name and value
  type, in declared order), its result value type or none, and its effect.
- Each population record of the package, with its member types and its
  declared extent, available to the checker and to observation admission
  (FR-106) by its declaration key.

## Behavior

- Intake SHALL resolve each `modifies` entry to a field declaration of the
  package, and each `creates` and `deletes` entry to an object type
  declaration, under QSpec `model-complete.md`'s Frames row, checking the
  entries of `modifies`, then `creates`, then `deletes`, each in document
  order, and reporting the first that fails.
- If a frame entry names no declaration of the package, then intake SHALL
  refuse `missing_declaration`/`missing-name` at the entry, retaining the
  operation's IR node identity, the entry, the artifact id and the span.
- If a frame entry names a relationship (`modifies`) or a process (`creates`,
  `deletes`), then intake SHALL refuse `unknown_required_feature`/
  `unsupported-feature` at the entry. The Frames row admits both, but no
  FR-106 observation carries relationship links or process instances, so a
  grant over them could never be enforced.
- If a frame entry names a declaration of any other meaning, then intake SHALL
  refuse `invalid_model_binding`/`malformed-declaration` at the entry, with
  the same retained data.
- Intake SHALL read an absent frame, or one whose three members are all
  empty, as the empty effect, as it does today.
- The assembler SHALL type each parameter and the result by FR-056's field
  rule: a native `Boolean` or `Integer`, a scalar type as its `Int[lower,
  upper]`, an object type as `Reference<M::T>`, with presence and
  multiplicity applied the same way. A parameter or result type the type
  environment cannot hold SHALL refuse
  `unknown_required_feature`/`unsupported-feature` at the `model`
  declaration, as a field of that type does.
- The assembler SHALL declare an operation on the object type that owns it.
  The operation is visible on a subtype through FR-081's effective view, as a
  field is, and keeps its declaring type as its owner (FR-105 anchors it
  there).
- The assembler SHALL still refuse record value types, systems parts, ports
  and allocations as `UnsupportedModelMember`. This requirement admits
  operations only.
- Intake SHALL still shape-check the operation's `pre` and `post` expression
  texts in the domain package and SHALL NOT keep them. State clauses in the
  `1-draft` unit are the clauses this lane checks and runs.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-103-AC-1 | The ConfigVersion domain package (TC-458 fixture: object type `ConfigVersion` with `versionNumber: VersionNumber` bound to `Int[0, 1000]` and optional `parent: ConfigVersion`, population `config_history` over `ConfigVersion` with extent `closed`, and operation `attemptUpdate` with no parameters, result `Boolean` and frame `modifies [ConfigVersion/versionNumber]`) admits. The assembled `Config::ConfigVersion` has fields `versionNumber: Int[0, 1000]` and `parent: Option<Reference<Config::ConfigVersion>>`, and operation `attemptUpdate` with no parameters, result `Boolean` and an effect whose `modifies` is exactly the `versionNumber` field key. | Test (TC-458) |
| FR-103-AC-2 | The same package with `modifies [ConfigVersion/missing]` refuses `missing_declaration`/`missing-name` at that entry; with `modifies [ConfigVersion]` (an object type) or `creates [ConfigVersion/versionNumber]` (a field) it refuses `invalid_model_binding`/`malformed-declaration` at that entry; with a relationship `Supersedes` declared and `modifies [Supersedes]` it refuses `unknown_required_feature`/`unsupported-feature` at that entry; with both `modifies [ConfigVersion/missing]` and `creates [ConfigVersion/versionNumber]` it reports only the `modifies` refusal. No declaration is admitted in any case. | Test (TC-458) |
| FR-103-AC-3 | An operation with a parameter `note: Text` refuses `unknown_required_feature`/`unsupported-feature` at the `model` declaration; an operation with parameter `delta: VersionNumber` admits it typed `Int[0, 1000]`. | Test (TC-458) |
| FR-103-AC-4 | A package holding a record value type still refuses `UnsupportedModelMember` naming that node, with the operation admitted beside it not changing the refusal. | Test (TC-458) |
| FR-103-AC-5 | Admission is deterministic: admitting the ConfigVersion package twice, and admitting it with its operation and population records reordered in the document, gives equal `PackageDeclarations` and equal effect key lists. | Test (TC-458) |

## Dependencies

- FR-056 (intake and assembler), FR-081 (effective view), FR-084
  (population records).
- QSpec FR-154, FR-340 and `model-complete.md`'s Frames row.
- filament-core-data's Semantic IR reader already validates an operation's
  `frame` shape at the pinned revision 033e228
  (`crates/semantic-ir/src/schema.rs:1656-1663`).
