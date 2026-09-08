---
id: FR-005
title: "Link native names through the shared model adapter"
type: FR
relationships:
  - target: "ix://agent-ix/quire-spec-language/US-002"
    type: traces_to
---
# FR-005: Link native names through the shared model adapter

## Description

When linking is requested, the compiler shall resolve native name occurrences against the selected shared model environment.

## Inputs

ParsedUnit, exact import/source bindings, qualified shared declaration handles and accepted contracts.

## Outputs

A separately constructed LinkedPackage or located linkage diagnostics.

## Behavior

The existing Filament model authority and contract-IR typed-model view own domain types. The compiler does not infer declarations from populations or source examples. It checks stable declaration IDs and closure. The actual synthetic producer fixture remains unqualified until those bindings exist.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-005-AC-1 | An exact qualified import resolves to the shared model's exported type identifier. | Test |
| FR-005-AC-2 | A missing import receives missing_import. | Test |
| FR-005-AC-3 | Ambiguous exports receive ambiguous_declaration. | Test |
| FR-005-AC-4 | A stale package closure receives stale_dependency. | Test |
| FR-005-AC-5 | A failed link yields no LinkedPackage. | Test |

## Dependencies

- [US-002](../usecase/US-002-link-exact-models.md) supplies the user need.
- [Detailed contract or implementation evidence](../../README.md) supplies the scoped context.

## Status

Draft. Specification review and prerequisite acceptance remain distinct from existing code/tests. No acceptance criterion is claimed satisfied solely because this artifact has been authored.
