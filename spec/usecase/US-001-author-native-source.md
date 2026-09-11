---
id: US-001
title: "Author native source with precise feedback"
type: US
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-035
    type: exercises
  - target: "ix://agent-ix/quire-spec-language/StR-001"
    type: traces_to
  - target: "ix://agent-ix/quire-spec-language/FR-001"
    type: exercises
  - target: "ix://agent-ix/quire-spec-language/FR-002"
    type: exercises
  - target: "ix://agent-ix/quire-spec-language/FR-003"
    type: exercises
  - target: "ix://agent-ix/quire-spec-language/FR-010"
    type: exercises
---
# US-001: Author native source with precise feedback

## Story

**As a** specification author
**I want** to edit and read native clauses with precise feedback
**So that** I can correct the intended text without losing its identity.

## Context

This story covers historical state clauses and the proposed composed native
edition. Precise feedback applies equally to predicate, temporal and choreography
syntax. Existing parser evidence does not establish the composed grammar.

## Acceptance Examples (Illustrative)

### US-001-EX-1

- **Given** an admitted clause.
- **When** I parse or format it.
- **Then** the source text remains identifiable.

### US-001-EX-2

- **Given** a malformed clause.
- **When** I parse it.
- **Then** the relevant original location is reported.

## Priority and Risk (Informative)

Priority: High. Confusing a source/model identity or incomplete execution with a verified result would invalidate the intended assessment.

## Traceability (Informative)

- [FR-035](../functional/FR-035-parse-composed-native-units.md)
- [FR-001](../functional/FR-001-read-exact-source.md)
- [FR-002](../functional/FR-002-parse-native-units.md)
- [FR-003](../functional/FR-003-format-native-source.md)
- [FR-010](../functional/FR-010-report-native-outcomes.md)
