---
id: FR-041
title: "Admit exact rational roles through an explicit native model profile"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-002
    type: implements
  - target: ix://agent-ix/quire-spec-language/FR-015
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-025
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-040
    type: traces_to
---
# FR-041: Admit exact rational roles through an explicit native model profile

## Description

When a caller explicitly selects native-state-model/2, the native model adapter SHALL admit exact rational scalar roles over the supplied IR declarations while retaining the selected profile in its immutable artifact.

## Inputs

The existing exact `FormalSource`, validated IR `DeclarationEnvironment`,
`NativeRoles` and caller-lowered `ModelLimits`, with an explicit model-profile
selection. The source-derived path selects `native-rule-model/2` through the
existing `model_source::read` frontend and retains that selection through draft
admission into `native-state-model/2`.

`NativeModel::new` remains the `native-state-model/1` entry point;
`model_source::FORMAT` remains `native-rule-model/1`. New selection is explicit,
never inferred from the presence of a rational, a caller's backend or current
implementation defaults. Unknown profile/format selections refuse.

## Outputs

An immutable admitted `NativeModel` with inspectable actual profile, original
source/declaration/role loci, canonical IR declaration content and deterministic
native artifact bytes/digest. Failure retains the existing typed frontend,
upstream IR or native-admission cause with original source correspondence and
no partially admitted model. A source draft remains a draft until admission.

Model admission supplies a producer prerequisite for
[FR-040](FR-040-check-composed-values.md). It establishes neither rational
expression typing/definedness nor runtime evaluation or a complete compiled
package/consumer artifact.

## Behavior

The native model adapter SHALL preserve the native-state-model/1 admission rules and exact artifact encoding for existing inputs.

If a rational declaration or role is supplied under native-state-model/1, then the native model adapter SHALL refuse that model without upgrading its profile.

When native-state-model/2 is selected, the native model adapter SHALL retain the existing integer, text, enum, record, optional, sequence, object and operation rules while admitting explicitly mapped rational scalar sites.

The native model adapter SHALL bind each rational primitive leaf to exactly one nominal scalar role whose exact unit and IR rational representation agree across all its declared sites.

If a scalar role is missing, duplicated, foreign, without declared sites or inconsistent with any site's representation, then the native model adapter SHALL refuse the complete model.

Rational `ScalarKind` adds its exact `Unit`; numerator minimum, numerator
maximum and maximum denominator come from the actual `ir::RationalType` at the
IR sites. Equal representations cannot merge different nominal scalar names or
model owners. Optional/sequence wrappers preserve the same leaf role with their
existing authored bounds and ordering. Rational bounds remain IR-owned;
`NativeType::Scalar` can retain the actual IR value type without a parallel
type, range or arithmetic engine.

The `/2` source format adds one closed scalar variant to the existing located
JSON model grammar. Existing model sections and scalar-reference/type shapes
retain their meanings; no second model schema or Markdown reader is introduced.

| Field | Required interpretation |
| --- | --- |
| `kind` | Required exact tag `rational`. |
| `name` | Required string admitted by the existing symbol constructor. |
| `numerator_minimum` | Required JSON integer decoded exactly as i64. |
| `numerator_maximum` | Required JSON integer decoded exactly as i64. |
| `maximum_denominator` | Required JSON integer decoded exactly as u64, then checked by the actual IR constructor. |
| `unit` | Existing optional string convention: absent or null means dimensionless; a supplied name is the exact admitted named unit. No unit is inferred from a field or sample. |

When decoding native-rule-model/2 rational bounds, the source frontend SHALL construct the representation through `ir::RationalType::new` with the exact supplied integers.

The IR constructor requires numerator minimum no greater than maximum and
maximum denominator in 1..i64::MAX inclusive. Float/exponent JSON numbers,
out-of-width integers, unknown or duplicate fields, missing required fields,
wrong tags and wrong value kinds refuse through the existing strict decoder
or IR constructor. No float conversion, rounding, inferred bound, widening or
handwritten replacement constructor repairs them. `native-rule-model/1` refuses
the rational variant. Original occurrence locations come from the existing
located JSON decoder and exact `FormalSource`, not normalized/reserialized text.

The native model adapter SHALL encode the actual selected model profile in the artifact alongside the existing source, roles, declaration loci and canonical IR content.

The existing bounded IR canonicalization and Serde artifact writer remain the
identity machinery. The actual profile is fixed by admission and cannot be
retagged afterward. Otherwise identical `/1` and `/2` models have distinct
native bytes/digests even when no rational is used. Raw native artifact integrity
remains distinct from IR canonical identity and original model source integrity.

If a historical link_native import selects a native-state-model/2 artifact, then the historical linker SHALL refuse the unit before exposing it to the historical checker or runtime.

When composed model binding selects an admitted native-state-model/2 artifact, the composed binder SHALL retain its actual profile, exact import bytes/digest, model ownership and existing permitted-export checks.

Adding an unused `/2` input does not grant historical semantics; the selected
import is the boundary requiring refusal. Composed model binding grants no
permission to skip later value/profile checking. A `/1` source format cannot
silently emit `/2`, and a `/2` source draft cannot silently admit as `/1`.

The source frontend and model adapter SHALL apply their existing independently caller-lowered hard ceilings and atomic refusal paths to the new variant.

Existing ceilings remain 1 MiB source/artifact content, 10,000 frontend entries,
10,000 admission roles/entries/type nodes and depth 64 at their respective
boundaries. Nested rational sites count under the existing wrapper/node rules;
artifact bytes include the profile and rational content. Zero never disables a
limit, above-hard requests are clamped and the next unaffordable step refuses
before its work. No rational fast path bypasses source, locus, role, type-depth,
canonicalization or output-content checks.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-041-AC-1 | Existing constructors/FORMAT select `/1`, preserve exact historical artifact bytes and rational refusal, and never infer `/2`. Explicit `/2` survives source-read/draft/admission with the actual inspectable model profile; unknown selections refuse. | Test (TC-120) |
| FR-041-AC-2 | A rational scalar with identical IR numerator/denominator bounds at all scalar sites admits under `/2`; missing/duplicate/empty site mappings, kind mismatch or a change to any one site's minimum, maximum or denominator refuses without a model. | Test (TC-120) |
| FR-041-AC-3 | The closed rational JSON variant preserves i64 numerator extrema, integers adjacent to 2^53 and valid maximum-denominator boundaries exactly. Zero/negative/too-large denominator, reversed numerator bounds, missing/duplicate/unknown fields, wrong tags, floating values and out-of-width integers refuse through the actual decoder/IR constructor. | Test (TC-120) |
| FR-041-AC-4 | Source-derived rational values and record fields under options/sequences retain exact nominal identity, unit and original scalar/site loci. Missing/null unit is dimensionless; named units remain exact. Existing integer/text/enum/object/operation meanings and wrappers are preserved under both compatible profiles. | Test (TC-120) |
| FR-041-AC-5 | Historical link_native refuses a selected `/2` model, including an otherwise unchanged integer-only model, without a linked package. Composed bind_models accepts actual admitted `/2` exports and refuses stale digests, foreign ownership or wrong exports through its existing typed boundary. | Test (TC-120) |
| FR-041-AC-6 | `/1` and `/2` over otherwise identical source/declarations/roles yield different profile-bearing native artifacts. Rational bounds, unit and source mutations alter the selected artifact; set-like inventory reorderings remain deterministic, and IR canonical/source digests cannot substitute for native artifact digest. | Test (TC-120) |
| FR-041-AC-7 | Controlled rational/wrapped inputs obey existing source-entry, role/site/node/depth and artifact/canonicalization limits at zero, exact and one-step-insufficient capacities; above-hard requests cannot raise ceilings. Exhaustion yields typed resource incompleteness without partial model or historical fallback. | Test (TC-120) |

## Dependencies

[FR-015](FR-015-project-native-model-semantics.md) owns `/1` native model
admission and [FR-025](FR-025-compile-rule-model-source.md) owns the existing
located JSON producer. This explicit extension supplies
[FR-040](FR-040-check-composed-values.md) under compiler
[#35](https://github.com/agent-ix/quire-spec-language/issues/35) and
[#36](https://github.com/agent-ix/quire-spec-language/issues/36).
[TC-120](../test-cases/TC-120-admit-rational-native-model-profile.md) has implemented
public controls with passing local runs; PR review corrections are underway.
The pinned IR already owns rational declarations, constructors, canonicalization
and arithmetic; this requirement changes native producer selection/admission.
It does not implement expression checks, execution, D's unrelated producer
correspondence or the full artifact delivery tracked by compiler #40.
