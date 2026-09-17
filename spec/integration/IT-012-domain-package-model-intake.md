---
id: IT-012
title: "Admit a spec artifact bundle through quire-rs and the FCD semantic IR crates"
type: IT
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-056, type: verifies }
  - { target: ix://agent-ix/quire-spec-language/FR-036, type: verifies }
  - { target: ix://agent-ix/filament-core-data/FR-142, type: depends_on }
  - { target: ix://agent-ix/filament-core-data/FR-143, type: depends_on }
---
# IT-012: Admit a spec artifact bundle through quire-rs and the FCD semantic IR crates

## Objective

Verify the real integration boundary of model intake: the compiler turns a spec
artifact bundle into Quire model declarations by calling quire-rs extraction,
the FCD extraction-frontend lift and the FCD semantic IR reader, and links
native source against the result. Without this test, a hand-built semantic
context or a model-shaped fixture could stand in for the real lowering.

## Target Integration

The compiler under test is this repository's public Rust intake and linking API.
Its external dependencies are:

- `quire_rs::semantic::extract_semantic`, driven by the real module manifests'
  `body_extraction` and `semantic` blocks;
- `agent-ix-extraction-frontend`, which lifts extraction output to Semantic IR
  2.0.0;
- `agent-ix-semantic-ir`, which reads and checks the IR document.

All three are Rust crate calls at the exact git revisions selected in
`Cargo.lock`. The integration type is an in-process library call chain.

## Preconditions

- agent-ix/filament-core-data#172 (Semantic IR 2.0.0 with embedded `constructs`
  table) is merged and its revision is selected in `Cargo.lock`.
- The module manifests that type the bundle declare a `construct` for each
  object type the bundle uses.
- A domain-package fixture bundle whose constructs cover object, record, enum,
  reference, operation, relationship and population exports is committed with
  its IR golden. Which module supplies it is decided under
  agent-ix/quire-spec-language#131.

## Inputs

- The fixture bundle's artifact paths and bytes, and the exact module manifests.
- The IR 2.0.0 golden for that bundle from its owning repository.
- A native package whose declarations reference the bundle's objects, fields,
  references, operations, relationships and populations.

## Test Procedure

1. Run extraction and lift over the bundle through the crate APIs.
   - IT-012-SC-01: the lifted IR document is byte-identical to the owning
     repository's golden and reports contract version `2.0.0`.
2. Read the document through `agent-ix-semantic-ir` and run compiler intake.
   - IT-012-SC-02: the reader returns no diagnostics, and intake yields one model
     declaration per type definition, each bound through its construct's
     `meaning` id.
3. Link the native package against the admitted domain package.
   - IT-012-SC-03: every model reference binds to a declaration key that carries
     the domain package identity and `sha256-jcs` digest.
4. Inspect the resolved dependency graph and compiler source.
   - IT-012-SC-04: `Cargo.lock` selects the pinned revisions, the FCD crates stay
     `publish = false`, and no compiler source contains a module object-type name
     used for dispatch.

## Expected Results

All four success criteria hold. Model declarations come from the real
extraction, lift and reader, not from copied types or hand-built JSON. Linking
establishes static model binding only; evaluation, observation and protocol
assessment are verified elsewhere.

## Metadata

- Priority: P0
- Target Integration: quire-rs extraction, FCD extraction-frontend and semantic IR crates
- Automation: Automated Rust integration test

## Dependencies

**Upstream:** [FR-056](../functional/FR-056-admit-domain-package-model-declarations.md),
agent-ix/filament-core-data#172 and the fixture bundle's owning repository.
**Downstream:** [TC-148](../test-cases/TC-148-domain-package-end-to-end.md) and
the compiled-protocol `Model` identity under agent-ix/quire-spec-language#132.
