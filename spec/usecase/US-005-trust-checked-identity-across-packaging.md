---
id: US-005
title: "Trust checked identity through the packaging boundary"
type: US
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-065
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-066
    type: exercises
  - target: "ix://agent-ix/quire-spec-language/StR-001"
    type: traces_to
---
# US-005: Trust checked identity through the packaging boundary

## Story

**As a** developer of a QSL packaging, lowering or proof-pipeline consumer
**I want** the function I am packaging or lowering to carry the same stable
identity and source provenance on both sides of the checked-package boundary
**So that** I can trust a function's identity and its reported source location
without reconstructing them from the CST, tokens, display strings or
diagnostics I would otherwise have to re-parse or re-derive.

## Context

Before this migration, packaging and lowering code for function application
reads meaning back out of a native-v1 representation that was already
checked once; two separate paths (native-v1 and the checked-package spine)
can each hand a consumer a different notion of "this function" with no
shared identity between them. A consumer that wants to know "is this the
function my caller checked" or "where in the source did this call occur"
has no single, trustworthy answer.

## Acceptance Examples (Illustrative)

### US-005-EX-1: Same function, same identity, across the boundary

- **Given** a function declared once and called once in a source file.
- **When** the declaration and call are checked, linked into a package, and
  the package is emitted as `quire.checked-package/v2` bytes and read back.
- **Then** the function's identity, read from the checked node, the linked
  package and the decoded v2 bytes, is the same value every time.

### US-005-EX-2: Source location survives the boundary

- **Given** the same call site.
- **When** its occurrence is resolved before and after packaging.
- **Then** it resolves to the same byte span in the original source both
  times.

### US-005-EX-3: One producer, not two

- **Given** a request to package or lower a function application.
- **When** the request is made after this migration lands.
- **Then** exactly one code path can have produced the resulting package or
  artifact, and no caller can select a second, older path.

## Priority and Risk (Informative)

Priority: High. Without a shared, stable identity and provenance, a proof or
lowering consumer cannot reliably attribute a checked result back to the
source function it came from, which undermines every later stage that
depends on that attribution (replay, diagnostics, proof obligations).

## Traceability (Informative)

- [FR-062](../functional/FR-062-implement-checked-family-contract.md)
- [FR-065](../functional/FR-065-migrate-function-application-to-checked-family.md)
- [FR-066](../functional/FR-066-document-family-migration-recipe.md)
