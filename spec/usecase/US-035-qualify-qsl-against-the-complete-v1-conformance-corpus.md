---
id: US-035
title: "Qualify QSL against the complete-V1 conformance corpus"
type: US
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-350
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-351
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-352
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-353
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-354
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-003
    type: exercises
  - target: ix://agent-ix/quire-spec-language/StR-001
    type: traces_to
---
# US-035: Qualify QSL against the complete-V1 conformance corpus

## Story

**As a** QSL maintainer, or a team that adopts QSL as its complete-V1
compiler and reference runtime
**I want** one repository gate that runs the QSpec complete-V1 conformance corpus
against QSL and reports, for each capability, whether QSL passed its
vectors, with a coverage figure and a qualified or not-qualified verdict
**So that** "QSL is complete V1" is a statement the product measures on
every run, and each capability that is missing, failing or unsupported is
named with the vectors that show it.

## Context

QSpec FR-311 defines complete-V1 qualification as executing positive,
boundary and refusal vectors for every inventory capability, and QSpec
FR-336 defines the corpus those vectors come from. QSL provides the
compiler, the reference runtime, replay and the formatter. This story is
the QSL side: running the vectors that apply to those subjects through
QSL's own public entry points and reporting what happened.

Extensions are part of the language surface the corpus exercises (QSpec
FR-133). QSL admits a declaration-form extension from its declarative
grammar and typed-node schemas, so an extension is qualified by the same
vectors as the core language.

## Acceptance Examples (Illustrative)

### US-035-EX-1: A clean run reports complete V1 qualified

- **Given** a corpus in which every vector that applies to QSL passes.
- **When** the maintainer runs the `qualify` gate.
- **Then** every capability reports `passed`, coverage is 100 per cent,
  the verdict is qualified and the gate exits 0.

### US-035-EX-2: One failing vector blocks only its capability

- **Given** a corpus in which one refusal vector expects
  `ill_typed`/`unit-mismatch` and QSL returns `ill_typed`/`type-mismatch`.
- **When** the maintainer runs the `qualify` gate.
- **Then** that vector's capability reports `failed`, naming the vector,
  the expected and the actual code and cause; every other capability keeps
  its own outcome; the verdict is not qualified and names that capability.

### US-035-EX-3: A capability with no vector is uncovered, not passed

- **Given** a corpus that lists a capability for the `language` consumer type and
  holds no vector for it.
- **When** the maintainer runs the `qualify` gate.
- **Then** the capability reports `uncovered` and counts against coverage.

### US-035-EX-4: An extension declared by schema parses and checks

- **Given** a definition catalog holding an extension whose grammar schema
  adds a declaration form and whose typed-node schema gives its node.
- **When** a unit that selects the extension declares that form.
- **Then** QSL parses it, checks it against the typed-node schema and
  carries the typed node in the checked package; a unit that does not
  select the extension is refused at that form.

## Priority and Risk (Informative)

Priority: High. Without a run that measures every capability, a
complete-V1 claim rests on reading tickets and test names.

## Traceability (Informative)

- [FR-350](../functional/FR-350-run-the-conformance-corpus-against-qsl.md)
- [FR-351](../functional/FR-351-settle-each-capability-from-its-executed-vectors.md)
- [FR-352](../functional/FR-352-report-capability-coverage.md)
- [FR-353](../functional/FR-353-settle-the-complete-v1-qualification-verdict.md)
- [FR-354](../functional/FR-354-admit-extensions-from-declarative-grammar-and-typed-node-schemas.md)
- [FR-003](../functional/FR-003-format-native-source.md) (formatting keeps
  the checked package identity, AC-9)
