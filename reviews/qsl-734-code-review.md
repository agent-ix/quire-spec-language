---
id: SR-7876
title: "QSL-734 PR 799 code review: variant types as enum and variant exports"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@437648288cf5f990a76adbf5e8d0c5336014a532; PR #799 diff against impl/business-model-meanings (merge base e4f9b5ae8): qsl-semantics/src/model/intake.rs, qsl-semantics/src/model/variant_type.rs, qsl-semantics/src/model/domain_package.rs, qsl-semantics/src/model/normalize.rs, qsl-semantics/src/model/index.rs, qsl-semantics/src/model/admitted.rs, qsl-semantics/src/model/meaning_dispatch.rs, qsl-semantics/src/model/systems.rs, qsl-semantics/src/check/assemble.rs, qsl-semantics/src/check/lowering/model.rs, qsl-semantics/tests/it/variant_type.rs, src/checking/types.rs, src/linking/composed/models.rs, src/protocol_artifact/domain.rs, qsl-replay/src/spine.rs"
review_set: subset
---

## Summary

Ticket: QSL-734. PR quire-spec-language#799 has one commit (43764828) on base
`impl/business-model-meanings`. The rust-review lane is folded into this file.
This is a reviewer-only pass. I read the diff and the source at the reviewed
sha. I did not run a build or tests.

The PR reads a `quire.meaning.model.variant-type/v1` type into one
`EnumRecord` (members sorted by UTF-8 bytes) and one `VariantRecord` per case.
`read_variant_type` refuses a construct that is not shape `enumeration`, an
empty `variants`, a repeated case name (`conflicting-binding`), a case with no
span, a case whose identity is not the minted one, and a payload. The variant
case's original declaration is kept as the export locus. A new
`AssemblyCause::UnsupportedVariantType` refuses source type forms and
state-clause contexts that name a variant type. Normalize refuses a field whose
`typeRef` names a variant type.

## Verdict

PASS WITH FINDINGS. The reader rules match FR-056 "Variant types" for every
case the tests cover. Members sort by bytes, so `Gold` and `gold` are distinct
cases. The export key is (owner, case) and never the minted node. Clause
refusals use `unsupported_construct`/`declaration-form`, and a field typed by a
variant type refuses `malformed-declaration` at the field node. The
exhaustive-match updates in `src/checking/types.rs`,
`src/linking/composed/models.rs` and `qsl-replay` refuse or return `None`, so
nothing passes silently. The exception is `protocol_artifact/domain.rs`
(FND-004).

Defects:

- Admission makes a variant type nameable from an operation signature. There
  it gets the wrong cause (FND-001).
- Only the first defect of a variant type is reported (FND-002).
- The re-pointed `known_but_unsupported` test pins a cause that FR-056-AC-3
  contradicts (FND-003).

Landing hazard (not a defect of this diff): PR #796 now gives
`IntakeMalformedDeclaration`, `UnsupportedDeclarationForm` and
`ConflictingBinding` a `member` field. Whichever PR lands second will not
compile until it is adapted. The adaptation covers the normalize field refusal,
every `ctx.malformed`/`unsupported_at`
site in `read_variant_type`, the `ConflictingBinding { key: owner }`
construction, and the test helper `node_of` and the
`ConflictingBinding { key: key("Tier") }` equality. #796's derived path for a
plain `ctx.malformed` at `$.types[i]` stops at `TypeDefinition`, so these sites
need explicit paths. A textual conflict with #796 is in
`intake.rs` (the re-pointed test's `what`/`member`). One with #792 is in the
`intake.rs` import list. Both are trivial.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | An operation parameter or result whose `typeRef` names an admitted variant type falls to `signature_type_refusal`. Its `forbidden_meaning` list has no `Enum` arm, so the operation refuses `dangling_reference`/`UnknownValueType` "is not a declared type", although the type is declared. FR-056 says a `typeRef` naming another business-model meaning is rejected under FR-154 with its named refusal, and that a member `typeRef` naming a variant type refuses `invalid_model_binding`/`malformed-declaration`. The path was unreachable before this PR, because the variant type itself refused. No test covers it. Add `DomainPackageRecord::Enum(_)` (and `Variant(_)`) to `forbidden_meaning`, and add a vector. | qsl-semantics/src/model/normalize.rs:1777-1790; qsl-semantics/src/model/normalize.rs:2136-2160; spec/functional/FR-056-admit-domain-package-model-declarations.md:406-407; spec/functional/FR-056-admit-domain-package-model-declarations.md:504-511 |
| FND-002 | medium | `read_variant_type` returns on the first failing check ("the first failure refuses"). A variant type with two payload cases, or one spanless case and one mis-minted case, reports one refusal. QSpec FR-154 says intake "reports every declaration refusal in node order" and, within one node, every failing check in table order and member-path order. FR-056 "Declarations" repeats "every declaration refusal". Each case is its own IR node (the `variant/` slot). Other readers already return one `Result` per type node. This new reader writes the first-failure rule into its own doc comment. | qsl-semantics/src/model/intake.rs:3589-3597; qsl-semantics/src/model/intake.rs:3648-3671; QSpec spec/functional/type-model/FR-154-admit-domain-package-model.md:107-109 |
| FND-003 | medium | `refuses_a_known_but_unsupported_fr208_meaning_as_unsupported` is re-pointed from `VARIANT_TYPE` to `POPULATION`. It now asserts that a `types[]` node bound to the population meaning refuses `unsupported_construct`/`declaration-form` ("has no reader yet"). It is tagged FR-056-AC-3, which says "a node not valid for its construct's meaning under FR-154 refuses `invalid_model_binding`/`malformed-declaration`". A population read from `types[]` is such a node, and populations do have a reader. After this PR, POPULATION is the only meaning the `other if meaning::ALL.contains(&other)` arm can reach. QSL-734 AC3 says to replace this test with positive and negative vectors, not re-point it. | qsl-semantics/src/model/intake.rs:5985-6027; qsl-semantics/src/model/intake.rs:3824-3832; spec/functional/FR-056-admit-domain-package-model-declarations.md:375-377 |
| FND-004 | medium | `protocol_artifact::domain::exports` skips `Enum` and `Variant` with `continue`, so a compiled artifact over a model with a variant type carries no `enum` or `variant` export. FR-056 says "the compiler SHALL derive these export records", and the wire already has `ExportKind::Enum`/`Variant`, which the native-model path emits. The domain path also drops `ScalarType` (pre-existing, the same gap). Nothing tests the artifact export. | src/protocol_artifact/domain.rs:150-163; src/protocol_artifact/models.rs:707-726; spec/functional/FR-056-admit-domain-package-model-declarations.md:439-449 |
| FND-005 | low | `check_names` refuses a bare `Config::Tier` type form as `UnsupportedVariantType`. Inside `Reference<Config::Tier>` the argument goes to the object-type check and refuses `missing_declaration` (`UnresolvedTypeName`) instead. FR-056 gives every source naming of a variant type `unsupported_construct`/`declaration-form`. | qsl-semantics/src/check/assemble.rs:1221-1236 |
| FND-006 | low | The record loop re-reads each case identity with `.unwrap_or(node)`. If that were ever reached, it would key a case by its owner type's node. The first loop already validated the identity with `str_field("identity")?`. Keep that validated `&str` in `cases` and drop the fallback. | qsl-semantics/src/model/intake.rs:3655; qsl-semantics/src/model/intake.rs:3681-3687 |
