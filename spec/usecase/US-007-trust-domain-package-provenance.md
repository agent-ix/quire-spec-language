---
id: US-007
title: "Trust a compiled artifact's domain-package provenance"
type: US
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: exercises
  - target: "ix://agent-ix/quire-spec-language/StR-001"
    type: traces_to
---
# US-007: Trust a compiled artifact's domain-package provenance

## Story

**As a** downstream consumer of a compiled-protocol package (a proof or
assessment backend, an auditor, or a tool that re-links a package against a
newer domain package)
**I want** the package's `Model` to name the domain package it was linked
against directly, by identity, version and digest
**So that** I can verify which domain package a model came from by reading
one record, without decoding a separate producer/relation object that names
the same fact through an indirection with its own failure modes.

## Context

Before this migration, a compiled-protocol `Model` named its domain package
only through an optional `Correspondence`, itself naming a `ProducerObject`:
an interface-indexed producer identity, checked against a separately
supplied relation artifact. A `Model` with no `Correspondence` named no
domain package at all. Since FR-056 landed real domain-package intake,
admitting a domain package's own identity, version and Semantic IR digest
directly, ahead of any compiled artifact, the compiled-protocol wire carries
that same fact twice, through two different shapes, with the older one still
able to go absent.

## Acceptance Examples (Illustrative)

### US-007-EX-1: One domain package, one name

- **Given** a native package linked against one domain package admitted
  under FR-056.
- **When** the compiler emits the compiled-protocol package.
- **Then** the emitted `Model`'s domain package identity, version and digest,
  read directly from the `Model` record, equal the `DomainPackageRef` FR-056
  admitted at linking.

### US-007-EX-2: No second producer object to reconcile

- **Given** the same emitted package.
- **When** a consumer reads the `Model` record.
- **Then** there is no `Correspondence` or `ProducerObject` value to decode,
  check against a relation artifact, or find absent.

### US-007-EX-3: An old-shaped payload does not pass silently

- **Given** a `quire.compiled-protocol/1` payload whose `Model` still carries
  a `correspondence`, `producer` or `interface` member from before this
  migration.
- **When** it is offered to the reader built under this requirement.
- **Then** the reader refuses it as an unrecognized field rather than
  reading part of it into a domain package identity or ignoring the rest.

## Priority and Risk (Informative)

Priority: High. A consumer that cannot tell which domain package a model
came from, or that must reconcile two differently-shaped provenance records
to find out, cannot safely attribute a compiled artifact's meaning to the
domain package a later change may have altered.

## Traceability (Informative)

- [FR-042](../functional/FR-042-publish-compiled-protocol-artifacts.md)
  defines the compiled-protocol `Model` and its domain-package naming.
- [FR-056](../functional/FR-056-admit-domain-package-model-declarations.md)
  admits the domain package this story's `Model` names.
