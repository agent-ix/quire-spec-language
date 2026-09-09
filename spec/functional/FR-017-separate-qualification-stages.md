---
id: FR-017
title: "Preserve identity across focused qualification and linkage stages"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-002
    type: implements
  - target: ix://agent-ix/quire-spec-language/StR-001
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-015
    type: traces_to
---
# FR-017: Preserve identity across focused qualification and linkage stages

## Description

When qualification or linkage inputs are processed, the native compiler and its Rust helpers shall preserve source and outcome identity across explicit decoding, admission and construction stages.

## Inputs

The existing source-bound native rule-model JSON, formal-environment linker
inputs, and historical fixture/role packets. Existing profiles, source limits,
public signatures and selected producer pins remain the input contracts.

## Outputs

The existing typed declarations, linked packages and audit outcomes. The
source-aware qualification decoder additionally retains each selected JSON
value's exact original byte span; it is private Rust test support.

## Behavior

The native rule-model producer shall obtain declaration locations from the original parsed occurrence before constructing IR declarations.

If a JSON occurrence does not belong to the exact bound source, then the qualification decoder shall refuse that occurrence.

The native rule-model producer shall store each scalar representation and its native role together under one validated scalar identity.

The native linker shall separate inventory admission, exact import selection and clause resolution while retaining its current refusal ordering and public output.

The historical fixture auditors shall separate artifact loading, composition checks and result rendering while retaining their current acceptance and refusal behavior.

The JSON grammar remains owned by pinned serde_json. Borrowed RawValue preserves
the original occurrence; checked offsets and FormalSource map it to IR loci.
No text search, reconstructed JSON spelling, new language grammar or unsafe
pointer dereference supplies provenance. Enable raw_value only in the existing
serde_json development dependency; no new package or non-Rust execution is needed.

Use focused typed conversion functions and one cohesive scalar table. Enum
matches remain exhaustive. Setup errors propagate to a single test-harness
expect boundary; a setup failure cannot count as a native checker refusal.
Existing bounded JSON audit admission and native lexer/Pratt parser remain their
grammar authorities. This refactoring introduces no extra validation policy for
historical packets and no change to reported logical truth or resource outcomes.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-017-AC-1 | Repeated names, a type reference before its declaration, escaped JSON strings and changed whitespace/key order retain the exact selected occurrence; foreign buffers, malformed JSON and duplicate/unknown typed fields refuse. | Test (TC-054) |
| FR-017-AC-2 | The producer has separate source intake, decoding and typed lowering responsibilities, one scalar representation/role table, propagated setup errors and bounded source/type processing. | Inspection |
| FR-017-AC-3 | Existing exact imports, ambiguity provenance, legacy unsupported constructs and lowered/hard linker limits retain the TC-030–034 outcomes after stage separation. | Test (TC-030, TC-031, TC-032, TC-033, TC-034) |
| FR-017-AC-4 | Historical audit commands retain their positive, corruption and incomplete observations after stage separation, including selected private fixture cases. | Test (TC-001, TC-002, TC-003, TC-004, TC-006) |

## Dependencies

FR-012 owns historical Rust audit semantics; FR-013 owns existing linkage;
FR-014 owns exact native/IR coordinates; FR-015 owns admitted native models.
NFR-005 applies to all new qualification execution. Hosted CI stays manual-only.

## Status

Specified repair of architecture findings SR-074. Existing native model/checker
tests remain unfinished; refactoring the producer does not establish FR-015/016.
