---
id: US-003
title: "Evaluate a bounded state with honest outcomes"
type: US
relationships:
  - target: "ix://agent-ix/quire-spec-language/StR-001"
    type: traces_to
  - target: "ix://agent-ix/quire-spec-language/FR-007"
    type: exercises
  - target: "ix://agent-ix/quire-spec-language/FR-008"
    type: exercises
  - target: "ix://agent-ix/quire-spec-language/FR-018"
    type: exercises
  - target: "ix://agent-ix/quire-spec-language/FR-043"
    type: exercises
  - target: "ix://agent-ix/quire-spec-language/FR-044"
    type: exercises
  - target: "ix://agent-ix/quire-spec-language/FR-101"
    type: exercises
---
# US-003: Evaluate a bounded state with honest outcomes

## Story

**As a** verification operator
**I want** to run a condition on a declared finite population
**So that** I can distinguish a violation from an inability to evaluate.

## Context

This story drives the native finite-state work assigned to Agent A. The requirements below remain draft until the requested specification review is completed. Existing implementation observations do not establish the unimplemented pipeline stages.

## Acceptance Examples (Illustrative)

### US-003-EX-1

- **Given** a complete valid state.
- **When** I evaluate its clause.
- **Then** a logical result is produced.

### US-003-EX-2

- **Given** an exhausted work budget.
- **When** I evaluate its clause.
- **Then** incompleteness is reported.

## Priority and Risk (Informative)

Priority: High. Confusing a source/model identity or incomplete execution with a verified result would invalidate the intended assessment.

## Traceability (Informative)

- [FR-007](../functional/FR-007-validate-runtime-inputs.md)
- [FR-008](../functional/FR-008-evaluate-state-reference.md)
- [FR-018](../functional/FR-018-construct-native-runtime-inputs.md)
- [FR-043](../functional/FR-043-evaluate-bounded-native-temporal.md)
- [FR-044](../functional/FR-044-activate-temporal-obligations.md)
