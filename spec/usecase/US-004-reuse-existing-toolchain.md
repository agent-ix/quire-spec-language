---
id: US-004
title: "Use native clauses through the existing toolchain"
type: US
relationships:
  - target: "ix://agent-ix/quire-spec-language/FR-042"
    type: exercises
  - target: "ix://agent-ix/quire-spec-language/FR-038"
    type: exercises
  - target: "ix://agent-ix/quire-spec-language/StR-001"
    type: traces_to
  - target: "ix://agent-ix/quire-spec-language/FR-009"
    type: exercises
  - target: "ix://agent-ix/quire-spec-language/FR-011"
    type: exercises
  - target: "ix://agent-ix/quire-spec-language/FR-012"
    type: exercises
  - target: "ix://agent-ix/quire-spec-language/FR-045"
    type: exercises
---
# US-004: Use native clauses through the existing toolchain

## Story

**As a** tool integrator
**I want** to connect native clauses to the existing extraction and execution interfaces
**So that** source and model meaning survive the integration.

## Context

This story drives the native finite-state work assigned to Agent A. The requirements below remain draft until the requested specification review is completed. Existing implementation observations do not establish the unimplemented pipeline stages.

Compiler #40 extends this consumer need to compiled choreography: downstream
tools must retain exact numeric values, typed declarations and their original
source/model authorities without interpreting native source.

## Acceptance Examples (Illustrative)

### US-004-EX-1

- **Given** a qualified adapter and projection.
- **When** I execute a native clause.
- **Then** the authored obligation remains identifiable.

### US-004-EX-2

- **Given** an unqualified feature.
- **When** I request projection.
- **Then** the limitation remains visible.

## Priority and Risk (Informative)

Priority: High. Confusing a source/model identity or incomplete execution with a verified result would invalidate the intended assessment.

## Traceability (Informative)

- [FR-009](../functional/FR-009-lower-qualified-projections.md)
- [FR-011](../functional/FR-011-integrate-opaque-extraction.md)

- [FR-012](../functional/FR-012-audit-fixtures-in-rust.md) covers Rust verification of selected fixtures.
- [FR-038](../functional/FR-038-encode-exact-protocol-numbers.md) preserves exact integer/rational wire values for protocol consumers.
- [FR-042](../functional/FR-042-publish-compiled-protocol-artifacts.md) defines the full compiled artifact and real Rust consumer handoff.
- [FR-045](../functional/FR-045-classify-temporal-mapping-support.md)
