---
id: FR-025
title: "Compile explicit rule-model source"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-002
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-015
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-017
    type: references
---
# FR-025: Compile explicit rule-model source

## Description

When a caller selects the native-rule-model/1 source profile and supplies an identified model document, the frontend shall derive located formal declarations and native roles through the existing IR constructors.

## Inputs

An immutable FormalSource, explicit source-format selector and caller-lowered
source-byte, declaration-entry and type-depth limits. The profile promotes the
existing source-aware rule-model fixture syntax: root license/package/requirement/
revision and scalar/record/value/object/operation arrays, with optional enums.
Declarations, nominal types, object references and operation frames retain the
existing fixture-to-native-model semantics under FR-015/017.

## Outputs

A ModelDraft containing the exact original source, declared license text, existing
DeclarationEnvironment and NativeRoles. A separate fallible admit operation uses
NativeModel::new with ModelLimits. Read/admission failures retain the original
FormalSource and typed JSON, IR, native admission or profile/limit cause.
Unknown profiles use UnknownWire; malformed declarations use InvalidModelBinding;
exhausted budgets use ResourceExhausted. Native admission/source errors preserve
their existing codes.
Duplicate and unknown scalar identities have distinct causes retaining the name.
Source-byte, entry-category and type-depth limits have distinct typed context,
including the effective ceiling; callers can inspect the clamped limits.

## Behavior

The frontend shall select native-rule-model/1 before interpreting its declarations.
The frontend shall apply inclusive caller-lowered ceilings of 1 MiB source,
10,000 declaration/field/variant/frame entries and 64 nested type levels.
The frontend shall decode JSON through Serde and require object-shaped records
with unique known fields.
The frontend shall retain exact raw JSON occurrences for all declaration loci.
Its located JSON utility shall require an explicit byte ceiling before reading.
Occurrence decoding requires a raw value borrowed directly from that source;
owned or foreign raw values shall refuse even when their bytes match.
The frontend shall use existing IR constructors for names, bounds, types and owners.
The frontend shall derive each scalar's sites from actual field/value uses.
The frontend shall refuse duplicate scalar declarations and unknown scalar uses.
The frontend shall retain ordered operation parameters and explicit effect frames.
The frontend shall return no draft on a decode/lowering failure and no model on
an admission failure.
The frontend shall report exhausted source/lowering/admission limits as incomplete.
The frontend shall charge each entry group after bounded JSON framing and before
decoding its values, including each operation's parameters and frame groups.

The existing syntax admits Boolean, scalar, record, enum, optional and bounded
sequence types; integer/text scalars; State/Input values; object/reference roles;
and operation frames. Omitted enums means none; omitted/null scalar unit means
dimensionless; omitted/null operation result means no result. The required license
string is retained metadata and does not select semantics or change source rights.
ModelDraft establishes source-derived declaration input; only successful admission
establishes NativeModel. The profile does not parse native-state-model/1 artifact
bytes or add another formal-model authority. No producer process or I/O is required.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-025-AC-1 | The existing rule-model source produces the same admitted model bytes/digest and declaration/role loci through the public frontend. | Test |
| FR-025-AC-2 | All admitted source types and role forms retain their existing meaning; repeated names in different declarations and Unicode/escaped text preserve their actual occurrences. | Test |
| FR-025-AC-3 | Unknown profiles, malformed/unknown/duplicate fields, duplicate scalar declarations, unknown scalar uses and native admission failures retain source and distinct typed causes without partial success; owned/foreign raw occurrences refuse. | Test |
| FR-025-AC-4 | Caller-lowered source, entry-category, type-depth and admission limits return incomplete with effective ceilings; exhausted entry groups stop before their values or later operations decode, and fresh requests can succeed. | Test |
| FR-025-AC-5 | A public source-derived model reaches native compilation and actual state/operation execution without test-only semantic lowering. | Test |

## Dependencies

- [FR-015](FR-015-project-native-model-semantics.md): native model admission.
- [FR-017](FR-017-separate-qualification-stages.md): existing source-aware fixture frontend.
