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
`NativeRoles` and caller-configured `ModelLimits`, with an explicit model-profile
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
native artifact bytes. Failure retains the existing typed frontend,
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
native bytes even when no rational is used. Raw native artifact integrity
remains distinct from IR canonical identity and original model source integrity.

When composed model binding selects an admitted native-state-model/2 artifact, the composed binder SHALL retain its actual profile, exact import bytes, model ownership and existing permitted-export checks.

Adding an unused `/2` input does not grant historical semantics; the selected
import is the boundary requiring refusal. Composed model binding grants no
permission to skip later value/profile checking. A `/1` source format cannot
silently emit `/2`, and a `/2` source draft cannot silently admit as `/1`.

The source frontend and model adapter SHALL apply their existing independently caller-configured limits and atomic refusal paths to the new variant.

The published defaults remain 1 MiB source/artifact content, 10,000 frontend
entries and 10,000 admission roles/entries/type nodes at their respective
boundaries; a caller may raise or lower each, and it is used as given. Nesting
depth is not a limit (ADR-030 D-1). Nested rational sites count under the
existing wrapper/node rules; artifact bytes include the profile and rational
content. Zero never disables a limit, and the next unaffordable step refuses
before its work. No rational fast path bypasses source, locus, role, type-node,
canonicalization or output-content checks.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-041-AC-1 | **RETIRED in M-6d.** The historical `NativeModel::new`/`FORMAT` `/1` identity and the `/2` source-read/draft profile selection are no longer acceptance obligations of this feature. | Test |
| FR-041-AC-2 | A rational scalar with identical IR numerator/denominator bounds at all scalar sites admits under `/2`; missing/duplicate/empty site mappings, kind mismatch or a change to any one site's minimum, maximum or denominator refuses without a model. | Test |
| FR-041-AC-3 | **RETIRED in M-6d.** The former closed rational JSON decoding boundary is not a separately maintained native acceptance obligation; IR constructors remain authoritative for rational declarations. | Test |
| FR-041-AC-4 | **RETIRED in M-6d.** The former native source-derived rational value/site mapping is not a separately maintained acceptance obligation. | Test |
| FR-041-AC-5 | Composed `bind_models` accepts actual admitted `/2` exports and refuses stale models, foreign ownership or wrong exports through its existing typed boundary. The historical `link_native` refusal clause is retired in M-6d. | Test |
| FR-041-AC-6 | `/1` and `/2` over otherwise identical source/declarations/roles yield different profile-bearing native artifacts. Rational bounds, unit and source mutations alter the selected artifact; and set-like inventory reorderings remain deterministic. | Test |
| FR-041-AC-7 | Controlled rational/wrapped inputs obey existing source-entry, role/site/node and artifact/canonicalization limits at zero, exact and one-step-insufficient capacities, and an input above a default is admitted once that limit is raised to fit it. Exhaustion yields typed resource incompleteness without partial model or historical fallback. | Test |

## M-6d disposition

### M-6d disposition of the rational/native spine requirements

The following dispositions preserve each behavior unless a named spine test
already succeeds it. Retiring an old requirement or carrier does not by itself
retire the behavior. Any retained behavior without a named successor remains
in scope for [QSL-1380](https://linear.app/agent-ix/issue/QSL-1380/track-retained-temporal-and-choreography-behaviors-through-layer-4).

| Requirement | Disposition | Successor or retained behavior |
| --- | --- | --- |
| FR-042 | Retire in M-6d | Its checked-package publication contract is superseded by FR-027/FR-383 and TC-1061/TC-1062; no old wire artifact remains required. |
| FR-043 | Keep | Temporal settlement semantics have no complete named spine successor in this migration; retain and track Layer-4 follow-up. |
| FR-044 | Keep | Activation and progression behavior remains required; no complete named spine successor is identified here; retain and track Layer-4 follow-up. |
| FR-045 | Keep | Mapping-support classification remains required; no complete named spine successor is identified here; retain and track Layer-4 follow-up. |
| FR-046 | Keep | Predicate call identity/cycle behavior is exercised by AC-1/2, TC-126; query values and ordered semantics by AC-3..5, TC-127; missing input and accounting by AC-6/7, TC-128; emitted call records by AC-8, TC-126/127; union predicates/queries by AC-9, TC-833. The unsupported proof representations in AC-5 remain follow-up scope under QSL-1380. |
| FR-047 | Keep | TC-129 succeeds AC-2/3 and replaces AC-1's old identity chain with exact identity-axis mutations plus same-object pre/post observation rows; TC-130 succeeds AC-4..6; TC-131 succeeds AC-7/8. |
| FR-048 | Keep | Checked choreography evaluation succeeds to FR-390 and TC-1070..1073; retain any remaining native-only behavior not covered there and track it in the Layer-4 follow-up. |
| FR-049 | Keep | TC-136 succeeds AC-1..6 and AC-10/11; TC-137 succeeds AC-7/8; TC-142 and TC-1070..1073 succeed AC-9's protocol-slot selection, typed outcomes and shared-meter behavior. |
| FR-050 | Keep | Authenticated temporal artifact behavior has no complete named spine successor in this migration; retain and track Layer-4 follow-up. |
| FR-052 | Keep behavior; retire only the M-6d request/result carrier and its API-specific criteria | FR-052-AC-9–12 define the Layer-4 checked-package evaluation-owner successor; TC-144 is its named test obligation. TC-140 and `tests/it/native_temporal_owner.rs` are historical/retiring and provide no successor evidence. |
| FR-053 | Keep behavior; retire only the M-6d request/result carrier and its API-specific criteria | FR-053-AC-7–9 define the Layer-4 opaque trigger-identity successor; TC-149 is its named test obligation. TC-141 and `tests/it/native_temporal_owner.rs` are historical/retiring and provide no successor evidence. |
| TC-121 | Retire in M-6d | The case verifies the FR-042 artifact contract that is superseded by the spine package tests above. |

## Dependencies

[FR-015](FR-015-project-native-model-semantics.md) owns `/1` native model
admission and [FR-025](FR-025-compile-rule-model-source.md) owns the existing
located JSON producer. This explicit extension supplies
[FR-040](FR-040-check-composed-values.md) under compiler
[#35](https://github.com/agent-ix/quire-spec-language/issues/35) and
[#36](https://github.com/agent-ix/quire-spec-language/issues/36).
[TC-120](../test-cases/TC-120-admit-rational-native-model-profile.md) has implemented
public controls with passing local runs; PR review corrections are underway.
The IR already owns rational declarations, constructors, canonicalization
and arithmetic; this requirement changes native producer selection/admission.
It does not implement expression checks, execution, D's unrelated producer
correspondence or the full artifact delivery tracked by compiler #40.
