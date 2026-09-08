---
id: StR-001
title: "Trust native finite-state assessments"
type: StR
relationships:
  - target: "ix://agent-ix/quire-spec-language/FR-001"
    type: satisfied_by
  - target: "ix://agent-ix/quire-spec-language/FR-002"
    type: satisfied_by
  - target: "ix://agent-ix/quire-spec-language/FR-003"
    type: satisfied_by
  - target: "ix://agent-ix/quire-spec-language/FR-004"
    type: satisfied_by
  - target: "ix://agent-ix/quire-spec-language/FR-005"
    type: satisfied_by
  - target: "ix://agent-ix/quire-spec-language/FR-006"
    type: satisfied_by
  - target: "ix://agent-ix/quire-spec-language/FR-007"
    type: satisfied_by
  - target: "ix://agent-ix/quire-spec-language/FR-008"
    type: satisfied_by
  - target: "ix://agent-ix/quire-spec-language/FR-009"
    type: satisfied_by
  - target: "ix://agent-ix/quire-spec-language/FR-010"
    type: satisfied_by
  - target: "ix://agent-ix/quire-spec-language/FR-011"
    type: satisfied_by
---
# StR-001: Trust native finite-state assessments

## Stakeholder Need

Verification operators require that native finite-state assessments shall retain the meaning and identity of the selected authored obligation.

## Rationale

Operators need to distinguish an actual violation from stale data, unsupported semantics and incomplete observation. A syntax milestone alone does not answer that operational need.

## Validation Criteria

| ID | Criteria | Validation |
| --- | --- | --- |
| StR-001-VC-1 | The supported healthy and violating workflow identifies the exact authored source. | Demonstration |
| StR-001-VC-2 | An incomplete required observation is reported without a logical result. | Demonstration |

## Stakeholders

Specification authors, model producers, verification operators and existing toolchain integrators.

## Dependencies

- [US-001](../usecase/US-001-author-native-source.md)
- [US-002](../usecase/US-002-link-exact-models.md)
- [US-003](../usecase/US-003-evaluate-bounded-state.md)
- [US-004](../usecase/US-004-reuse-existing-toolchain.md)
