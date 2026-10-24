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
  - target: ix://agent-ix/quire-spec-language/FR-401
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-405
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-071
    type: traces_to
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
- The table's namesakes SHALL be the items of the QSL workspace's members.
  `quire-exact`, `quire-semantic-value` and `quire-eval` are shared leaves in
  their own repositories, outside that workspace, and the `identifier` and
  `re-export` rules never apply inside them (FR-272). The verdict table's rows
  are checked by that gate; the shared-leaf table's rows are checked by a
  scan of the leaf's source, as that section states.
- `quire-semantic-value` SHALL name its `location::Location`, a declaration
  and a child-index path to an expression, `ExpressionLocation`, and its
  `location::Origin`, the declaration a location belongs to,
  `ExpressionOwner`: each shares its identifier with a canonical type of
  `quire_exact::location` (`Location` with members `node`, `occurrence`;
  `Origin` with members `role`, `ordinal`). The module stays
  `quire_semantic_value::location`, the path FR-401 and ADR-011 X-11 give,
  and the two renamed items are the evaluation interface's location
  vocabulary there: wherever FR-401 or ADR-011 names `Location` or `Origin`
  at that module, the item meant is `ExpressionLocation` or `ExpressionOwner`.
- `Evaluation`'s canonical definition SHALL be `quire_eval::Evaluation`,
  where FR-401 slice S0 and ADR-011 X-14 move it; FR-405 deletes the module
  `qsl-eval/src/value/expression/evaluate.rs` that defined it in QSL.
- `QualifiedName`'s one definition SHALL be `qsl-eval`'s
  `qsl_eval::value::expression::family::QualifiedName`, which FR-401 keeps
  out of the leaf. `qsl-replay` reaches it through its existing `qsl-eval`
  dependency.
- The replay request SHALL keep FR-071-AC-3 over that one type. Its
  function-selection entry points (`ReplayRequest::decode` and
  `ReplayRequestWire::replay_target`) take a `QualifiedName` value, never a
  `&str`, a `String` or an `impl Into<QualifiedName>`. `QualifiedName` SHALL
  implement no `From<&str>` and no `From<String>`, so no string converts into
  it through `Into`, and a string literal in the selection position fails to
  compile. Its `FromStr` implementation and its
  `unqualified(impl Into<String>)` constructor are explicit, fallible
  constructors of the name that refuse a non-identifier
  (`InvalidQualifiedName`). They are not constructors or decoders of the
  request and no implicit conversion, so FR-071-AC-3 holds with them in
  place.

## Verdict table

"Root" is the root `quire-spec-language` crate (`src/`). Each canonical
type is named by its owner crate and module. Row n is checked by
FR-273-AC-n.

| Row | Canonical type | Namesake | Namesake's meaning | Verdict |
| --- | --- | --- | --- | --- |
| 2 | `quire_exact::accounting::LimitKind` (a `ScalarLimits` counter) | `qsl_semantics::model::accounting::LimitKind` | a `ModelNormalizationLimits` counter, an independent counter set (ADR-013 O-21) | rename `NormalizationLimitKind` |
| 3 | same | `qsl_foundation::diagnostic::stage::LimitKind` | a `stage_limit_exceeded` cause (ADR-013 T-4) | rename `StageLimitKind` |
| 4 | same | root `command::LimitKind` | a command intake ceiling on selected files | rename `IntakeLimitKind` |
| 5 | `quire_exact::accounting::ChargePoint` | `qsl_semantics::model::accounting::ChargePoint` | a model-normalization charge point | rename `NormalizationChargePoint` |
| 6 | `quire_exact::accounting::Incomplete` | `qsl_semantics::model::accounting::Incomplete` | the model-normalization incomplete record | rename `NormalizationIncomplete` |
| 7 | same | root `temporal::result::Incomplete` | a required temporal input that was absent | delete; removed with M-6c (ADR-011 §7.3) |
| 8 | `quire_exact::accounting::Meter` | `qsl_semantics::model::accounting::Meter` | the model-normalization meter | rename `NormalizationMeter` |
| 9 | same | root `package::intake::Meter` | the package-intake pass budget | delete; removed with M-6c (ADR-011 §7.3) |
| 10 | same | root `package::encoding::Meter` | the package-encoding pass budget | delete; removed with M-6c (ADR-011 §7.3) |
| 11 | same | root `protocol_artifact::encoding::Meter` | the protocol-artifact encoding work budget | delete; removed with M-6d, SEAM-3 (ADR-011 §7.3) |
| 12 | same | root `checking::proof::Meter` | the composed proof-check work budget | delete; removed with M-6e, SEAM-2, which still imports it (ADR-011 §7.3 and the SEAM-1 row) |
| 13 | `quire_exact::value::Value` (the kernel value, ADR-013 O-13) | root `state::input::Value` | a state-input value carrying a nominal wire type index (`value_type: u32`) beside its kind (lane-private, ADR-013 §6) | delete; removed with M-6c (ADR-011 §7.3) |
| 14 | same | root `runtime::evaluation::value::Value<'a>` | a borrowed arena view over IR value nodes, with `i64` integers, `&str` text and `quire_contract_model` references | delete; removed with M-6c (ADR-011 §7.3) |
| 15 | same | root `model_source::wire::Value` | a value-declaration wire record (name, kind, type) | delete; removed with M-6c (ADR-011 §7.3) |
| 16 | same | root `checking::proof::Value` | a proof-graph value node | delete; removed with M-6e, SEAM-2, which still imports it (ADR-011 §7.3 and the SEAM-1 row) |
| 17 | same | root `checking::composed::proofs::engine::Value` | a proof-graph value node | delete; removed with M-6e, SEAM-2 (ADR-011 §7.3) |
| 18 | `quire_exact::integer::Integer` (O-13) | root `protocol_artifact::wire::Integer` | an integer-only wire position | delete; removed with M-6d, SEAM-3 (ADR-011 §7.3) |
| 19 | `quire_exact::outcome::Outcome` (O-16) | `qsl_eval::simulation::explore::Outcome` | the result of one exploration run | rename `ExplorationOutcome` |
| 20 | same | `qsl_cst::parser::Outcome` | the result of matching one grammar rule | rename `MatchOutcome` |
| 21 | same | `tools/arch-lint` `canonical_encoder::Outcome` | the canonical-encoder check's result over one tree | rename `EncoderCheckOutcome` |
| 22 | same | root `command::output::extraction::Outcome` | the serialized view of a clauses outcome | delete; removed with M-6c (ADR-011 §7.3) |
| 23 | same | root `command::output::types::Outcome` | the stage member of a run-result document | delete; removed with M-6c (ADR-011 §7.3) |
| 24 | same | root `temporal::activation::Outcome` | what activation produced | delete; removed with M-6c (ADR-011 §7.3) |
| 25 | `quire_exact::outcome::Refusal` (a kernel value refusal, O-16) | `qsl_cst::parser::Refusal` | a resource ceiling hit while matching | rename `MatchLimitHit` |
| 26 | same | root `Refusal` in `main.rs` | one syntax-command refusal line | rename `RefusalLine` |
| 27 | same | root `protocol_artifact::v2::refusal::Refusal` | a version-2 artifact refusal | delete; removed with M-6d, SEAM-3 (ADR-011 §7.3) |
| 28 | same | root `protocol_artifact::v3::refusal::Refusal` | a version-3 artifact refusal | delete; removed with M-6d, SEAM-3 (ADR-011 §7.3) |
| 29 | same | root `state::input::Refusal` | a state-input admission refusal | delete; removed with M-6c (ADR-011 §7.3) |
| 30 | same | root `linking::composed::binding::Refusal` | the evidence location of a binding failure | delete; removed with M-6e, SEAM-2 (ADR-011 §7.3) |
| 31 | same | root `temporal::result::Refusal` | a located temporal refusal | delete; removed with M-6c (ADR-011 §7.3) |
| 32 | `quire_eval::Evaluation` (the S6a evaluation, O-16; FR-401 and ADR-011 X-14 place it in `quire-eval`) | root `command::output::types::Evaluation` | the evaluation status member of a run-result document | delete; removed with M-6c (ADR-011 §7.3) |
| 33 | `quire_exact::location::Location` (O-12) | root `package::view::Location` | the serialized view of a declaration location | delete; removed with M-6c (ADR-011 §7.3) |
| 34 | `qsl_foundation::source::provenance::OccurrenceKey` (O-07) | root `protocol_artifact::occurrence::OccurrenceKey` | a validated runtime workflow key | delete; removed with M-6d, SEAM-3 (ADR-011 §7.3) |
| 35 | `qsl_foundation::source::provenance::RawSourceRef` (O-07, digest domain `quire.source.bytes/v1`) | `qsl_replay::identity::RawSourceRef` | a definition-document reference whose digest is in any domain | rename `DefinitionSourceRef` |
| 36 | `quire_checked::key::DeclarationKey` (O-03) | root `linking::DeclarationKey` | a declaration path within one requirement owner (lane-private, ADR-013 §6) | delete; removed with M-6e, SEAM-2, which still imports it (ADR-011 §7.3 and the SEAM-1 row) |
| 37 | `qsl_cst::ParsedSource` (O-15) | root `linking::composed::ParsedSource` | parsed evidence kept when namespace admission fails | delete; removed with M-6e, SEAM-2 (ADR-011 §7.3) |
| 38 | `qsl_package::checked::CheckedPackage` (O-15) | root `checking::CheckedPackage<'a>` | native-v1 checked clauses that keep the linked AST (lane-private, ADR-013 §6) | delete; removed with M-6c (ADR-011 §7.3) |
| 39 | `qsl_package::checked::EmittedPackage` (O-15) | root `protocol_artifact::native::EmittedPackage` | the bytes of a native protocol artifact | delete; removed with M-6d, SEAM-3 (ADR-011 §7.3) |
| 40 | `qsl_eval::value::expression::family::QualifiedName` (O-11; FR-401 keeps it in `qsl-eval`) | `qsl_replay::identity::QualifiedName` | a non-empty sequence of identifiers naming a declaration, the same meaning | delete; a copy, whose uses take the canonical type |
| 41 | same | root `runtime::input::QualifiedName` | a model-owned declaration selector (model and name) | delete; removed with M-6c (ADR-011 §7.3) |
| 42 | same | root `runtime::input::wire::QualifiedName` | the wire form of that selector | delete; removed with M-6c (ADR-011 §7.3) |
| 43 | same | root `syntax::composed::QualifiedName` | a model alias and a name, with their spans | delete; removed with M-6e, SEAM-2 (ADR-011 §7.3) |
| 44 | `qsl_semantics::value::member::Member` (the O-06 member identity) | `qsl_semantics::check::termination::Member<'a>` | what termination checking needs of one checked function | rename `TerminationSubject` |
| 45 | `qsl_foundation::source::provenance::OccurrenceKey` (O-07) | `pub use` in `qsl-replay/src/lib.rs` | the facade path CG reaches QSL through (ADR-011 FB-05) | keep; the `pub use` names `qsl_foundation::source::provenance::OccurrenceKey` |
| 46 | `quire_exact::location::Origin` (O-07) | `pub use` in `qsl-replay/src/lib.rs` | the facade path CG reaches QSL through (ADR-011 FB-05) | keep; the `pub use` names `quire_exact::Origin`, the owner crate's public path |
| 47 | `quire_exact::integer::Integer` (O-13) | `pub use` in `qsl-replay/src/lib.rs` | the facade path CG reaches QSL through (ADR-011 FB-05) | keep; the `pub use` names `quire_exact::Integer`, the owner crate's public path |
| 48 | `quire_checked::key::DeclarationKey` (O-03) | `pub use` in `qsl-replay/src/lib.rs` | the facade path CG reaches QSL through (ADR-011 FB-05) | keep; the `pub use` names `quire_checked::key::DeclarationKey` |

## Shared-leaf namesakes

The canonical-types gate (FR-272) scans the QSL workspace only, so it never
sees an item of the shared leaf `quire-semantic-value`. The two namesakes
below live in that leaf. Their verdicts hold by a scan of the leaf's own
source and by a compile-resolution test from the QSL workspace, not by the
gate. Row n is checked by FR-273-AC-n, and FR-273-AC-51 checks both.

| Row | Canonical type | Namesake | Namesake's meaning | Verdict |
| --- | --- | --- | --- | --- |
| 49 | `quire_exact::location::Location` (O-12) | `quire_semantic_value::location::Location` | a declaration and a child-index path to an expression | rename `ExpressionLocation`; checked by a scan of `quire-semantic-value` |
| 50 | `quire_exact::location::Origin` (an occurrence key within a node, O-07) | `quire_semantic_value::location::Origin` | the declaration a location belongs to | rename `ExpressionOwner`; checked by a scan of `quire-semantic-value` |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-273-AC-1 | Until M-6d lands, `cargo xtask canonical-types` runs on demand over the QSL workspace and reports every remaining namesake, and no merge gate waits on it. With the verdict table's canonical types tagged, every row carried out, and the M-6c, M-6d and M-6e deletions the `delete` rows cite landed, the gate reports nothing and exits 0, and `make ci`, with the gate among its prerequisites from then on, passes. | Test (TC-748) |
| FR-273-AC-2 | The item that was `qsl_semantics::model::accounting::LimitKind` is named `NormalizationLimitKind` in its definition and every use, and the canonical-types gate over the QSL workspace reports no `identifier` finding for `LimitKind` at it. | Test (TC-748) |
| FR-273-AC-3 | The item that was `qsl_foundation::diagnostic::stage::LimitKind` is named `StageLimitKind` in its definition and every use, and the canonical-types gate over the QSL workspace reports no `identifier` finding for `LimitKind` at it. | Test (TC-748) |
| FR-273-AC-4 | The item that was root `command::LimitKind` is named `IntakeLimitKind` in its definition and every use, and the canonical-types gate over the QSL workspace reports no `identifier` finding for `LimitKind` at it. | Test (TC-748) |
| FR-273-AC-5 | The item that was `qsl_semantics::model::accounting::ChargePoint` is named `NormalizationChargePoint` in its definition and every use, and the canonical-types gate over the QSL workspace reports no `identifier` finding for `ChargePoint` at it. | Test (TC-748) |
| FR-273-AC-6 | The item that was `qsl_semantics::model::accounting::Incomplete` is named `NormalizationIncomplete` in its definition and every use, and the canonical-types gate over the QSL workspace reports no `identifier` finding for `Incomplete` at it. | Test (TC-748) |
| FR-273-AC-7 | No module-level item at root `temporal::result::Incomplete` exists in the QSL workspace, and the canonical-types gate reports no `identifier` finding for `Incomplete` at its former path. | Test (TC-748) |
| FR-273-AC-8 | The item that was `qsl_semantics::model::accounting::Meter` is named `NormalizationMeter` in its definition and every use, and the canonical-types gate over the QSL workspace reports no `identifier` finding for `Meter` at it. | Test (TC-748) |
| FR-273-AC-9 | No module-level item at root `package::intake::Meter` exists in the QSL workspace, and the canonical-types gate reports no `identifier` finding for `Meter` at its former path. | Test (TC-748) |
| FR-273-AC-10 | No module-level item at root `package::encoding::Meter` exists in the QSL workspace, and the canonical-types gate reports no `identifier` finding for `Meter` at its former path. | Test (TC-748) |
| FR-273-AC-11 | No module-level item at root `protocol_artifact::encoding::Meter` exists in the QSL workspace, and the canonical-types gate reports no `identifier` finding for `Meter` at its former path. | Test (TC-748) |
| FR-273-AC-12 | No module-level item at root `checking::proof::Meter` exists in the QSL workspace, and the canonical-types gate reports no `identifier` finding for `Meter` at its former path. | Test (TC-748) |
| FR-273-AC-13 | No module-level item at root `state::input::Value` exists in the QSL workspace, and the canonical-types gate reports no `identifier` finding for `Value` at its former path. | Test (TC-748) |
| FR-273-AC-14 | No module-level item at root `runtime::evaluation::value::Value<'a>` exists in the QSL workspace, and the canonical-types gate reports no `identifier` finding for `Value` at its former path. | Test (TC-748) |
| FR-273-AC-15 | No module-level item at root `model_source::wire::Value` exists in the QSL workspace, and the canonical-types gate reports no `identifier` finding for `Value` at its former path. | Test (TC-748) |
| FR-273-AC-16 | No module-level item at root `checking::proof::Value` exists in the QSL workspace, and the canonical-types gate reports no `identifier` finding for `Value` at its former path. | Test (TC-748) |
| FR-273-AC-17 | No module-level item at root `checking::composed::proofs::engine::Value` exists in the QSL workspace, and the canonical-types gate reports no `identifier` finding for `Value` at its former path. | Test (TC-748) |
| FR-273-AC-18 | No module-level item at root `protocol_artifact::wire::Integer` exists in the QSL workspace, and the canonical-types gate reports no `identifier` finding for `Integer` at its former path. | Test (TC-748) |
| FR-273-AC-19 | The item that was `qsl_eval::simulation::explore::Outcome` is named `ExplorationOutcome` in its definition and every use, and the canonical-types gate over the QSL workspace reports no `identifier` finding for `Outcome` at it. | Test (TC-748) |
| FR-273-AC-20 | The item that was `qsl_cst::parser::Outcome` is named `MatchOutcome` in its definition and every use, and the canonical-types gate over the QSL workspace reports no `identifier` finding for `Outcome` at it. | Test (TC-748) |
| FR-273-AC-21 | The item that was `tools/arch-lint` `canonical_encoder::Outcome` is named `EncoderCheckOutcome` in its definition and every use, and the canonical-types gate over the QSL workspace reports no `identifier` finding for `Outcome` at it. | Test (TC-748) |
| FR-273-AC-22 | No module-level item at root `command::output::extraction::Outcome` exists in the QSL workspace, and the canonical-types gate reports no `identifier` finding for `Outcome` at its former path. | Test (TC-748) |
| FR-273-AC-23 | No module-level item at root `command::output::types::Outcome` exists in the QSL workspace, and the canonical-types gate reports no `identifier` finding for `Outcome` at its former path. | Test (TC-748) |
| FR-273-AC-24 | No module-level item at root `temporal::activation::Outcome` exists in the QSL workspace, and the canonical-types gate reports no `identifier` finding for `Outcome` at its former path. | Test (TC-748) |
| FR-273-AC-25 | The item that was `qsl_cst::parser::Refusal` is named `MatchLimitHit` in its definition and every use, and the canonical-types gate over the QSL workspace reports no `identifier` finding for `Refusal` at it. | Test (TC-748) |
| FR-273-AC-26 | The item that was root `Refusal` in `main.rs` is named `RefusalLine` in its definition and every use, and the canonical-types gate over the QSL workspace reports no `identifier` finding for `Refusal` at it. | Test (TC-748) |
| FR-273-AC-27 | No module-level item at root `protocol_artifact::v2::refusal::Refusal` exists in the QSL workspace, and the canonical-types gate reports no `identifier` finding for `Refusal` at its former path. | Test (TC-748) |
| FR-273-AC-28 | No module-level item at root `protocol_artifact::v3::refusal::Refusal` exists in the QSL workspace, and the canonical-types gate reports no `identifier` finding for `Refusal` at its former path. | Test (TC-748) |
| FR-273-AC-29 | No module-level item at root `state::input::Refusal` exists in the QSL workspace, and the canonical-types gate reports no `identifier` finding for `Refusal` at its former path. | Test (TC-748) |
| FR-273-AC-30 | No module-level item at root `linking::composed::binding::Refusal` exists in the QSL workspace, and the canonical-types gate reports no `identifier` finding for `Refusal` at its former path. | Test (TC-748) |
| FR-273-AC-31 | No module-level item at root `temporal::result::Refusal` exists in the QSL workspace, and the canonical-types gate reports no `identifier` finding for `Refusal` at its former path. | Test (TC-748) |
| FR-273-AC-32 | No module-level item at root `command::output::types::Evaluation` exists in the QSL workspace, and the canonical-types gate reports no `identifier` finding for `Evaluation` at its former path. | Test (TC-748) |
| FR-273-AC-33 | No module-level item at root `package::view::Location` exists in the QSL workspace, and the canonical-types gate reports no `identifier` finding for `Location` at its former path. | Test (TC-748) |
| FR-273-AC-34 | No module-level item at root `protocol_artifact::occurrence::OccurrenceKey` exists in the QSL workspace, and the canonical-types gate reports no `identifier` finding for `OccurrenceKey` at its former path. | Test (TC-748) |
| FR-273-AC-35 | The item that was `qsl_replay::identity::RawSourceRef` is named `DefinitionSourceRef` in its definition and every use, and the canonical-types gate over the QSL workspace reports no `identifier` finding for `RawSourceRef` at it. | Test (TC-748) |
| FR-273-AC-36 | No module-level item at root `linking::DeclarationKey` exists in the QSL workspace, and the canonical-types gate reports no `identifier` finding for `DeclarationKey` at its former path. | Test (TC-748) |
| FR-273-AC-37 | No module-level item at root `linking::composed::ParsedSource` exists in the QSL workspace, and the canonical-types gate reports no `identifier` finding for `ParsedSource` at its former path. | Test (TC-748) |
| FR-273-AC-38 | No module-level item at root `checking::CheckedPackage<'a>` exists in the QSL workspace, and the canonical-types gate reports no `identifier` finding for `CheckedPackage` at its former path. | Test (TC-748) |
| FR-273-AC-39 | No module-level item at root `protocol_artifact::native::EmittedPackage` exists in the QSL workspace, and the canonical-types gate reports no `identifier` finding for `EmittedPackage` at its former path. | Test (TC-748) |
| FR-273-AC-40 | No module-level item `qsl_replay::identity::QualifiedName` exists; `qsl-replay`'s former uses name `qsl_eval::value::expression::family::QualifiedName`, which implements no `From<&str>` or `From<String>`; `ReplayRequest::decode` and `ReplayRequestWire::replay_target` take a `QualifiedName` value, and a string literal passed in the selection position fails to compile (FR-071-AC-3); and the canonical-types gate reports no `identifier` finding for `QualifiedName` in `qsl-replay`. | Test (TC-748) |
| FR-273-AC-41 | No module-level item at root `runtime::input::QualifiedName` exists in the QSL workspace, and the canonical-types gate reports no `identifier` finding for `QualifiedName` at its former path. | Test (TC-748) |
| FR-273-AC-42 | No module-level item at root `runtime::input::wire::QualifiedName` exists in the QSL workspace, and the canonical-types gate reports no `identifier` finding for `QualifiedName` at its former path. | Test (TC-748) |
| FR-273-AC-43 | No module-level item at root `syntax::composed::QualifiedName` exists in the QSL workspace, and the canonical-types gate reports no `identifier` finding for `QualifiedName` at its former path. | Test (TC-748) |
| FR-273-AC-44 | The item that was `qsl_semantics::check::termination::Member<'a>` is named `TerminationSubject` in its definition and every use, and the canonical-types gate over the QSL workspace reports no `identifier` finding for `Member` at it. | Test (TC-748) |
| FR-273-AC-45 | The `pub use` of `OccurrenceKey` in `qsl-replay/src/lib.rs` names `qsl_foundation::source::provenance::OccurrenceKey` with no `as` rename, and the canonical-types gate reports no `re-export` finding for it. | Test (TC-748) |
| FR-273-AC-46 | The `pub use` of `Origin` in `qsl-replay/src/lib.rs` names `quire_exact::Origin` with no `as` rename, and the canonical-types gate reports no `re-export` finding for it. | Test (TC-748) |
| FR-273-AC-47 | The `pub use` of `Integer` in `qsl-replay/src/lib.rs` names `quire_exact::Integer` with no `as` rename, and the canonical-types gate reports no `re-export` finding for it. | Test (TC-748) |
| FR-273-AC-48 | The `pub use` of `DeclarationKey` in `qsl-replay/src/lib.rs` names `quire_checked::key::DeclarationKey` with no `as` rename, and the canonical-types gate reports no `re-export` finding for it. | Test (TC-748) |
| FR-273-AC-49 | A scan of the module-level items, type aliases and `pub use` items of `quire-semantic-value`'s `location` module finds `ExpressionLocation`, a declaration and a child-index path to an expression, and no item, alias or re-export whose identifier is `Location`; the same scan over a copy of the module with `pub type Location = ExpressionLocation;` added reports that alias. | Test (TC-748) |
| FR-273-AC-50 | A scan of the module-level items, type aliases and `pub use` items of `quire-semantic-value`'s `location` module finds `ExpressionOwner`, the declaration a location belongs to, and no item, alias or re-export whose identifier is `Origin`; the same scan over a copy of the module with `pub use self::ExpressionOwner as Origin;` added reports that re-export. | Test (TC-748) |
| FR-273-AC-51 | FR-401's interface type that locates an expression resolves at `quire_semantic_value::location::ExpressionLocation`, the owner of an `ExpressionLocation` at `quire_semantic_value::location::ExpressionOwner`, and neither `quire_semantic_value::location::Location` nor `quire_semantic_value::location::Origin` resolves. | Test (TC-748) |

## Dependencies

- [ADR-032](../decisions/ADR-032-checked-input-and-duplicate-canonical-type-gates.md)
  DT-7 and rulings R-1 and R-2.
- ADR-013 §3 (owner rows) and §6 (lane-private types).
- [FR-271](FR-271-tag-canonical-types-at-their-definition.md) and
  [FR-272](FR-272-fail-on-a-second-definition-of-a-canonical-type.md).
- [FR-401](FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md)
  (the owners of `Evaluation`, `Location` and `QualifiedName`),
  [FR-405](FR-405-retire-the-std-reference-evaluator-for-quire-eval.md)
  (the deleted evaluator module) and ADR-011 X-11 and X-14.
- [FR-071](FR-071-implement-typed-replay-request.md)-AC-3 (no string
  conversion into the replay request's `QualifiedName`).

## References

- Owning ticket: Linear QSL-391. Implementation: Linear QSL-13.
