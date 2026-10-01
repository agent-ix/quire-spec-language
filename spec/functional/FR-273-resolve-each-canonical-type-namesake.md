---
id: FR-273
title: "Resolve each namesake of a canonical type by deleting or renaming it"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-028
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-032
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-272
    type: depends_on
---
# FR-273: Resolve each namesake of a canonical type by deleting or renaming it

## Description

Each workspace item that shares a canonical type's identifier SHALL be
resolved by its meaning, as ADR-032 DT-7 and ruling R-2 decide. A namesake
with the same meaning as the canonical type is a copy, which the
implementing change deletes in favour of the canonical type. A namesake
with a different meaning is one the implementing change renames to say what
it is. The verdict table below gives
the verdict for each namesake measured on main on 2026-10-01; after it is
carried out, `cargo xtask canonical-types` (FR-272) reports nothing over the
QSL workspace.

## Inputs

- The canonical types: the ADR-013 §3 owner-row public types that FR-271
  tags.
- Each module-level item of the same identifier in the workspace's shipped
  code.

## Outputs

- The workspace with each namesake deleted or renamed by the table.

## Behavior

- The implementing change SHALL delete each `delete` row's item.
- The implementing change SHALL point each deleted item's uses at the
  canonical type.
- The implementing change SHALL rename each `rename` row's item to the name
  the row gives, in its definition and every use.
- A new name SHALL be one that no canonical type carries.

## Verdict table

"Root" is the root `quire-spec-language` crate (`src/`). Each canonical
type is named by its owner crate and module.

| Canonical type | Namesake | Namesake's meaning | Verdict |
| --- | --- | --- | --- |
| `quire_exact::accounting::LimitKind` (a `ScalarLimits` counter) | `qsl_semantics::model::accounting::LimitKind` | a `ModelNormalizationLimits` counter, an independent counter set (ADR-013 O-21) | rename `NormalizationLimitKind` |
| same | `qsl_foundation::diagnostic::stage::LimitKind` | a `stage_limit_exceeded` cause (ADR-013 T-4) | rename `StageLimitKind` |
| same | root `command::LimitKind` | a command intake ceiling on selected files | rename `IntakeLimitKind` |
| `quire_exact::accounting::ChargePoint` | `qsl_semantics::model::accounting::ChargePoint` | a model-normalization charge point | rename `NormalizationChargePoint` |
| `quire_exact::accounting::Incomplete` | `qsl_semantics::model::accounting::Incomplete` | the model-normalization incomplete record | rename `NormalizationIncomplete` |
| same | root `temporal::result::Incomplete` | a required temporal input that was absent | rename `MissingTemporalInput` |
| `quire_exact::accounting::Meter` | `qsl_semantics::model::accounting::Meter` | the model-normalization meter | rename `NormalizationMeter` |
| same | root `package::intake::Meter` | the package-intake pass budget | rename `PackageIntakeMeter` |
| same | root `package::encoding::Meter` | the package-encoding pass budget | rename `PackageEncodingMeter` |
| same | root `protocol_artifact::encoding::Meter` | the protocol-artifact encoding work budget | rename `ArtifactEncodingMeter` |
| same | root `checking::proof::Meter` | the composed proof-check work budget | rename `ProofCheckMeter` |
| `quire_exact::value::Value` (the kernel value, ADR-013 O-13) | root `state::input::Value` | an exact semantic value (lane-private, ADR-013 §6) | delete; use `quire_exact::Value` |
| same | root `runtime::evaluation::value::Value` | the native runtime's evaluated value | delete; use `quire_exact::Value` |
| same | root `model_source::wire::Value` | a value-declaration wire record (name, kind, type) | rename `ValueDeclarationWire` |
| same | root `checking::proof::Value` | a proof-graph value node | rename `ProofValueNode` |
| same | root `checking::composed::proofs::engine::Value` | a proof-graph value node | rename `ProofValueNode` |
| `quire_exact::value::ValueType` (O-14) | root `lowering::wire::ValueType` | the serialization form of an IR value type | rename `ValueTypeWire` |
| `quire_exact::integer::Integer` (O-13) | root `protocol_artifact::wire::Integer` | an integer-only wire position | rename `IntegerWire` |
| `quire_exact::outcome::Outcome` (O-16) | `qsl_eval::simulation::explore::Outcome` | the result of one exploration run | rename `ExplorationOutcome` |
| same | `qsl_cst::parser::Outcome` | the result of matching one grammar rule | rename `MatchOutcome` |
| same | root `command::output::extraction::Outcome` | the serialized view of a clauses outcome | rename `ClausesOutcomeView` |
| same | root `command::output::types::Outcome` | the stage member of a run-result document | rename `RunStageOutcome` |
| same | root `temporal::activation::Outcome` | what activation produced | rename `ActivationOutcome` |
| `quire_exact::outcome::Refusal` (a kernel value refusal, O-16) | `qsl_cst::parser::Refusal` | a resource ceiling hit while matching | rename `MatchLimitHit` |
| same | root `Refusal` in `main.rs` | one syntax-command refusal line | rename `RefusalLine` |
| same | root `protocol_artifact::v2::refusal::Refusal` | a version-2 artifact refusal | rename `ArtifactV2Refusal` |
| same | root `protocol_artifact::v3::refusal::Refusal` | a version-3 artifact refusal | rename `ArtifactV3Refusal` |
| same | root `state::input::Refusal` | a state-input admission refusal | rename `StateInputRefusal` |
| same | root `linking::composed::binding::Refusal` | the evidence location of a binding failure | rename `BindingEvidence` |
| same | root `temporal::result::Refusal` | a located temporal refusal | rename `TemporalRefusal` |
| `qsl_eval::value::expression::evaluate::Evaluation` (the S6a evaluation, O-16) | root `command::output::types::Evaluation` | the evaluation status member of a run-result document | rename `EvaluationStatus` |
| `quire_exact::location::Location` (O-12) | `quire_semantic_value::location::Location` | a declaration and a child-index path to an expression | rename `ExpressionLocation` |
| same | root `package::view::Location` | the serialized view of a declaration location | rename `DeclarationLocationView` |
| `quire_exact::location::Origin` (an occurrence key within a node, O-07) | `quire_semantic_value::location::Origin` | the declaration a location belongs to | rename `ExpressionOwner` |
| `qsl_foundation::source::provenance::OccurrenceKey` (O-07) | root `protocol_artifact::occurrence::OccurrenceKey` | a validated runtime workflow key | rename `RuntimeOccurrenceKey` |
| `qsl_foundation::source::provenance::RawSourceRef` (O-07, digest domain `quire.source.bytes/v1`) | `qsl_replay::identity::RawSourceRef` | a definition-document reference whose digest is in any domain | rename `DefinitionSourceRef` |
| `qsl_semantics::model::key::DeclarationKey` (O-03) | root `linking::DeclarationKey` | a declaration path within one requirement owner (lane-private, ADR-013 §6) | rename `LinkedDeclarationPath` |
| `qsl_cst::ParsedSource` (O-15) | root `linking::composed::ParsedSource` | parsed evidence kept when namespace admission fails | rename `UnadmittedParsedUnit` |
| `qsl_package::checked::CheckedPackage` (O-15) | root `checking::CheckedPackage` | a checked package (lane-private, ADR-013 §6) | delete; use `qsl_package::CheckedPackage` |
| `qsl_package::checked::EmittedPackage` (O-15) | root `protocol_artifact::native::EmittedPackage` | the bytes of a native protocol artifact | rename `NativeArtifactBytes` |
| `QualifiedName` (O-11), defined twice: `qsl_eval::value::expression::family::QualifiedName` and `qsl_replay::identity::QualifiedName` | each of the two | a non-empty sequence of identifiers naming a declaration, the same meaning | delete both; one tagged `QualifiedName` in `quire-semantic-value`, which both crates and the backends reach |
| same | root `runtime::input::QualifiedName` | a model-owned declaration selector (model and name) | rename `ModelDeclarationSelector` |
| same | root `runtime::input::wire::QualifiedName` | the wire form of that selector | rename `ModelDeclarationSelectorWire` |
| same | root `syntax::composed::QualifiedName` | a model alias and a name, with their spans | rename `ModelQualifiedReference` |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-273-AC-1 | With the verdict table's canonical types tagged and every row carried out, `cargo xtask canonical-types` over the QSL workspace reports nothing and exits 0, and `make ci`, with the gate among its prerequisites, passes. | Test (TC-748) |

## Dependencies

- [ADR-032](../decisions/ADR-032-checked-input-and-duplicate-canonical-type-gates.md)
  DT-7 and rulings R-1 and R-2.
- ADR-013 §3 (owner rows) and §6 (lane-private types).
- [FR-271](FR-271-tag-canonical-types-at-their-definition.md) and
  [FR-272](FR-272-fail-on-a-second-definition-of-a-canonical-type.md).

## References

- Owning ticket: Linear QSL-391. Implementation: Linear QSL-13.
