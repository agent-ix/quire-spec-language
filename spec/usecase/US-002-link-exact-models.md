---
id: US-002
title: "Link clauses to the intended domain model"
type: US
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-036
    type: exercises
  - target: "ix://agent-ix/quire-spec-language/StR-001"
    type: traces_to
  - target: "ix://agent-ix/quire-spec-language/FR-004"
    type: exercises
  - target: "ix://agent-ix/quire-spec-language/FR-005"
    type: exercises
  - target: "ix://agent-ix/quire-spec-language/FR-006"
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-019
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-020
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-021
    type: exercises
---
# US-002: Link clauses to the intended domain model

## Story

**As a** model author
**I want** to check clauses against the exact domain declarations I selected
**So that** incorrect or stale model assumptions are caught before execution.

## Context

This story covers exact model selection for historical clauses and the proposed
composed package. Cross-system and related-instance templates must retain their
model/definition/binding requirements before concrete assessment inputs exist.
Historical linkage evidence does not establish the composed path.

## Acceptance Examples (Illustrative)

### US-002-EX-1

- **Given** an exact qualified model.
- **When** I link my clause.
- **Then** its names resolve to that model.

### US-002-EX-2

- **Given** a stale model binding.
- **When** I request linking.
- **Then** the mismatch is reported.

## Priority and Risk (Informative)

Priority: High. Confusing a source/model identity or incomplete execution with a verified result would invalidate the intended assessment.

## Traceability (Informative)

- [FR-036](../functional/FR-036-link-composed-native-packages.md)
- [FR-004](../functional/FR-004-verify-source-maps.md)
- [FR-005](../functional/FR-005-link-shared-model.md)
- [FR-006](../functional/FR-006-check-defined-expressions.md)
- [FR-019](../functional/FR-019-package-checked-native-clauses.md)
- [FR-020](../functional/FR-020-read-and-rebind-native-packages.md)
- [FR-021](../functional/FR-021-derive-native-package-identity.md)
