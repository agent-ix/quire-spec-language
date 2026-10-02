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

Each canonical type SHALL have exactly one module-level definition in the
QSL workspace's shipped code, and every other item that once shared its
identifier SHALL carry the name its meaning gives it, as ADR-032 DT-7 and
ruling R-2 decide. A namesake with the same meaning as the canonical type is
a copy and does not exist; its uses take the canonical type. A namesake with
a different meaning carries a name that says what it is. Every `pub use` of
a canonical type outside its owner crate names the owner's defining path.
The verdict table below gives the verdict for each namesake and re-export,
and `cargo xtask canonical-types` (FR-272) reports nothing over the QSL
workspace once the plan slices the `delete` rows cite have removed their
items.

## Inputs

- The canonical types: the ADR-013 §3 owner-row public types that FR-271
  tags.
- Each module-level item of the same identifier in the workspace's shipped
  code.

## Outputs

- The workspace in which each table row holds: a `delete` row's item does
  not exist, a `rename` row's item carries the row's name, and a `keep` row's
  `pub use` names the owner path.

## Behavior

- No item of a `delete` row SHALL exist in the workspace. A row that cites
  a plan slice is a root SEAM item that slice deletes with its module
  (ADR-011 §6, §7.3); any other `delete` row's former uses name the
  canonical type.
- Each `rename` row's item SHALL carry the name the row gives, in its
  definition and every use.
- No renamed item SHALL carry a canonical type's identifier.
- Each `keep` row's `pub use` SHALL name the canonical type by a path
  through its owner crate's public API, with no `as` rename.

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
| same | root `temporal::result::Incomplete` | a required temporal input that was absent | delete; removed with M-6c (ADR-011 §7.3) |
| `quire_exact::accounting::Meter` | `qsl_semantics::model::accounting::Meter` | the model-normalization meter | rename `NormalizationMeter` |
| same | root `package::intake::Meter` | the package-intake pass budget | delete; removed with M-6c (ADR-011 §7.3) |
| same | root `package::encoding::Meter` | the package-encoding pass budget | delete; removed with M-6c (ADR-011 §7.3) |
| same | root `protocol_artifact::encoding::Meter` | the protocol-artifact encoding work budget | delete; removed with M-6d, SEAM-3 (ADR-011 §7.3) |
| same | root `checking::proof::Meter` | the composed proof-check work budget | delete; removed with M-6e, SEAM-2, which still imports it (ADR-011 §7.3 and the SEAM-1 row) |
| `quire_exact::value::Value` (the kernel value, ADR-013 O-13) | root `state::input::Value` | a state-input value carrying a nominal wire type index (`value_type: u32`) beside its kind (lane-private, ADR-013 §6) | delete; removed with M-6c (ADR-011 §7.3) |
| same | root `runtime::evaluation::value::Value<'a>` | a borrowed arena view over IR value nodes, with `i64` integers, `&str` text and `quire_contract_model` references | delete; removed with M-6c (ADR-011 §7.3) |
| same | root `model_source::wire::Value` | a value-declaration wire record (name, kind, type) | delete; removed with M-6c (ADR-011 §7.3) |
| same | root `checking::proof::Value` | a proof-graph value node | delete; removed with M-6e, SEAM-2, which still imports it (ADR-011 §7.3 and the SEAM-1 row) |
| same | root `checking::composed::proofs::engine::Value` | a proof-graph value node | delete; removed with M-6e, SEAM-2 (ADR-011 §7.3) |
| `quire_exact::value::ValueType` (O-14) | root `lowering::wire::ValueType` | the serialization form of an IR value type | delete; removed with M-6c (ADR-011 §7.3) |
| `quire_exact::integer::Integer` (O-13) | root `protocol_artifact::wire::Integer` | an integer-only wire position | delete; removed with M-6d, SEAM-3 (ADR-011 §7.3) |
| `quire_exact::outcome::Outcome` (O-16) | `qsl_eval::simulation::explore::Outcome` | the result of one exploration run | rename `ExplorationOutcome` |
| same | `qsl_cst::parser::Outcome` | the result of matching one grammar rule | rename `MatchOutcome` |
| same | `tools/arch-lint` `canonical_encoder::Outcome` | the canonical-encoder check's result over one tree | rename `EncoderCheckOutcome` |
| same | root `command::output::extraction::Outcome` | the serialized view of a clauses outcome | delete; removed with M-6c (ADR-011 §7.3) |
| same | root `command::output::types::Outcome` | the stage member of a run-result document | delete; removed with M-6c (ADR-011 §7.3) |
| same | root `temporal::activation::Outcome` | what activation produced | delete; removed with M-6c (ADR-011 §7.3) |
| `quire_exact::outcome::Refusal` (a kernel value refusal, O-16) | `qsl_cst::parser::Refusal` | a resource ceiling hit while matching | rename `MatchLimitHit` |
| same | root `Refusal` in `main.rs` | one syntax-command refusal line | rename `RefusalLine` |
| same | root `protocol_artifact::v2::refusal::Refusal` | a version-2 artifact refusal | delete; removed with M-6d, SEAM-3 (ADR-011 §7.3) |
| same | root `protocol_artifact::v3::refusal::Refusal` | a version-3 artifact refusal | delete; removed with M-6d, SEAM-3 (ADR-011 §7.3) |
| same | root `state::input::Refusal` | a state-input admission refusal | delete; removed with M-6c (ADR-011 §7.3) |
| same | root `linking::composed::binding::Refusal` | the evidence location of a binding failure | delete; removed with M-6e, SEAM-2 (ADR-011 §7.3) |
| same | root `temporal::result::Refusal` | a located temporal refusal | delete; removed with M-6c (ADR-011 §7.3) |
| `qsl_eval::value::expression::evaluate::Evaluation` (the S6a evaluation, O-16) | root `command::output::types::Evaluation` | the evaluation status member of a run-result document | delete; removed with M-6c (ADR-011 §7.3) |
| `quire_exact::location::Location` (O-12) | `quire_semantic_value::location::Location` | a declaration and a child-index path to an expression | rename `ExpressionLocation` |
| same | root `package::view::Location` | the serialized view of a declaration location | delete; removed with M-6c (ADR-011 §7.3) |
| `quire_exact::location::Origin` (an occurrence key within a node, O-07) | `quire_semantic_value::location::Origin` | the declaration a location belongs to | rename `ExpressionOwner` |
| `qsl_foundation::source::provenance::OccurrenceKey` (O-07) | root `protocol_artifact::occurrence::OccurrenceKey` | a validated runtime workflow key | delete; removed with M-6d, SEAM-3 (ADR-011 §7.3) |
| `qsl_foundation::source::provenance::RawSourceRef` (O-07, digest domain `quire.source.bytes/v1`) | `qsl_replay::identity::RawSourceRef` | a definition-document reference whose digest is in any domain | rename `DefinitionSourceRef` |
| `qsl_semantics::model::key::DeclarationKey` (O-03) | root `linking::DeclarationKey` | a declaration path within one requirement owner (lane-private, ADR-013 §6) | delete; removed with M-6e, SEAM-2, which still imports it (ADR-011 §7.3 and the SEAM-1 row) |
| `qsl_cst::ParsedSource` (O-15) | root `linking::composed::ParsedSource` | parsed evidence kept when namespace admission fails | delete; removed with M-6e, SEAM-2 (ADR-011 §7.3) |
| `qsl_package::checked::CheckedPackage` (O-15) | root `checking::CheckedPackage<'a>` | native-v1 checked clauses that keep the linked AST (lane-private, ADR-013 §6) | delete; removed with M-6c (ADR-011 §7.3) |
| `qsl_package::checked::EmittedPackage` (O-15) | root `protocol_artifact::native::EmittedPackage` | the bytes of a native protocol artifact | delete; removed with M-6d, SEAM-3 (ADR-011 §7.3) |
| `QualifiedName` (O-11), defined twice: `qsl_eval::value::expression::family::QualifiedName` and `qsl_replay::identity::QualifiedName` | each of the two | a non-empty sequence of identifiers naming a declaration, the same meaning | delete both; one tagged `QualifiedName` in `quire-semantic-value`, which both crates and the backends reach |
| same | root `runtime::input::QualifiedName` | a model-owned declaration selector (model and name) | delete; removed with M-6c (ADR-011 §7.3) |
| same | root `runtime::input::wire::QualifiedName` | the wire form of that selector | delete; removed with M-6c (ADR-011 §7.3) |
| same | root `syntax::composed::QualifiedName` | a model alias and a name, with their spans | delete; removed with M-6e, SEAM-2 (ADR-011 §7.3) |
| `qsl_semantics::value::member::Member` (the O-06 member identity) | `qsl_semantics::check::termination::Member<'a>` | what termination checking needs of one checked function | rename `TerminationSubject` |
| `qsl_foundation::source::provenance::OccurrenceKey` (O-07) | `pub use` in `qsl-replay/src/lib.rs` | the facade path CG reaches QSL through (ADR-011 FB-05) | keep; the `pub use` names `qsl_foundation::source::provenance::OccurrenceKey` |
| `quire_exact::location::Origin` (O-07) | `pub use` in `qsl-replay/src/lib.rs` | the facade path CG reaches QSL through (ADR-011 FB-05) | keep; the `pub use` names `quire_exact::Origin`, the owner crate's public path |
| `quire_exact::integer::Integer` (O-13) | `pub use` in `qsl-replay/src/lib.rs` | the facade path CG reaches QSL through (ADR-011 FB-05) | keep; the `pub use` names `quire_exact::Integer`, the owner crate's public path |
| `qsl_semantics::model::key::DeclarationKey` (O-03) | `pub use` in `qsl-replay/src/lib.rs` | the facade path CG reaches QSL through (ADR-011 FB-05) | keep; the `pub use` names `qsl_semantics::model::key::DeclarationKey` |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-273-AC-1 | Until M-6d lands, `cargo xtask canonical-types` runs on demand over the QSL workspace and reports every remaining namesake, and no merge gate waits on it. With the verdict table's canonical types tagged, every row carried out, and the M-6c, M-6d and M-6e deletions the `delete` rows cite landed, the gate reports nothing and exits 0, and `make ci`, with the gate among its prerequisites from then on, passes. | Test (TC-748) |

## Dependencies

- [ADR-032](../decisions/ADR-032-checked-input-and-duplicate-canonical-type-gates.md)
  DT-7 and rulings R-1 and R-2.
- ADR-013 §3 (owner rows) and §6 (lane-private types).
- [FR-271](FR-271-tag-canonical-types-at-their-definition.md) and
  [FR-272](FR-272-fail-on-a-second-definition-of-a-canonical-type.md).

## References

- Owning ticket: Linear QSL-391. Implementation: Linear QSL-13.
