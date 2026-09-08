---
id: FR-009
title: "Lower through the existing executable binder"
type: FR
relationships:
  - target: "ix://agent-ix/quire-spec-language/US-004"
    type: implements
---
# FR-009: Lower through the existing executable binder

## Description

When a backend projection is requested, the compiler shall lower only features qualified for the selected existing binder and backend.

## Inputs

Linked checked clauses, explicit source/model derivation and selected backend capability.

## Outputs

Existing bound executable projection or an obligation-preserving unsupported result.

## Behavior

Binding and lowering remain modules. The compiler creates no competing model or executable binder. Existing IR cannot encode identity-bearing references as recursive records. Lowering preserves source implication activation and complete executable clause population; unsupported obligations remain addressable.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-009-AC-1 | A qualified scalar projection is accepted by the existing strict binder. | Test |
| FR-009-AC-2 | An unqualified reference feature is refused as unsupported. | Test |
| FR-009-AC-3 | A missing executable clause binding refuses the projection. | Test |
| FR-009-AC-4 | Changed declaration semantics changes the bound identity. | Test |

## Dependencies

- [US-004](../usecase/US-004-reuse-existing-toolchain.md) supplies the user need.
- [Detailed contract or implementation evidence](../../README.md) supplies the scoped context.

## Status

Draft. Specification review and prerequisite acceptance remain distinct from existing code/tests. No acceptance criterion is claimed satisfied solely because this artifact has been authored.
