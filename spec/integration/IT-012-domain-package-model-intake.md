---
id: IT-012
title: "Admit a spec artifact bundle through quire-rs and the FCD semantic IR crates"
type: IT
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-056, type: verifies }
  - { target: ix://agent-ix/quire-spec-language/FR-036, type: verifies }
  - { target: ix://agent-ix/quire-specification/FR-152, type: references }
  - { target: ix://agent-ix/quire-specification/FR-154, type: references }
  - { target: ix://agent-ix/filament-core-data/FR-142, type: depends_on }
  - { target: ix://agent-ix/filament-core-data/FR-143, type: depends_on }
---
# IT-012: Admit a spec artifact bundle through quire-rs and the FCD semantic IR crates

## Objective

Verify the real integration boundary of model intake: the compiler turns a spec
artifact bundle into Quire model declarations by calling quire-rs extraction,
the FCD extraction-frontend lift and the FCD semantic IR reader, and links
native source against the result. Without this test, a hand-built semantic
context or model-shaped fixture could stand in for the real lowering.

## Target Integration

The compiler under test is this repository's public Rust bundle entry point,
intake seam and model linker. Its external dependencies are:

- `quire_rs::semantic::extract_semantic`, driven by the real module manifests'
  `body_extraction` and `semantic` blocks;
- `agent-ix-extraction-frontend`, which lifts extraction output to Semantic IR
  2.0.0 bytes;
- `agent-ix-semantic-ir`, which reads and checks those bytes.

All three are Rust crate calls at the exact git revisions selected in
`Cargo.lock`. The integration type is an in-process library call chain.

## Preconditions

- agent-ix/filament-core-data#172 (Semantic IR 2.0.0 with embedded `constructs`
  table) is merged and its revision is selected in `Cargo.lock`.
- The architecture fixture of agent-ix/filament-core-data#173, with its IR
  golden, is available at that revision. It declares parts, interfaces, ports
  and at least one connection between two ports.
- Quire specification FR-208 and FR-154 (agent-ix/quire-specification PR #86,
  pending merge) fix the meaning ids the fixture's constructs bind to and
  artifact-id identity.

## Inputs

- The fixture bundle's artifact paths and bytes, and the exact module manifests.
- The IR golden for that bundle from filament-core-data.
- A ModelSelection naming the fixture's domain package identity, version and
  `sha256-jcs` digest.
- A native package whose model references name a fixture object field, a Port
  and a Connection.

## Test Procedure

1. Run the bundle entry point: extraction and lift over the fixture bundle.
   - IT-012-SC-01: the lifted bytes equal the filament-core-data golden and their
     independently computed `sha256-jcs` digest equals the selection's digest.
2. Run the intake seam over the lifted bytes with the ModelSelection.
   - IT-012-SC-02: intake admits one declaration per IR node with no refusal,
     each type keyed by its artifact id and bound through its construct's
     `meaning` id; the result is byte-identical to the bundle entry point's.
3. Inspect the admitted systems declarations.
   - IT-012-SC-03: every Port retains its owning part, direction, interface type
     and typed multiplicity, and the Connection is admitted under FR-152's
     connection rule.
4. Link the native package against the admitted domain package.
   - IT-012-SC-04: every model reference binds to its declaration key and kind
     under the selected domain package identity and digest.
5. Inspect the resolved dependency graph and compiler source.
   - IT-012-SC-05: `Cargo.lock` selects the pinned revisions, the FCD crates stay
     `publish = false`, and no compiler source contains a module object-type name
     used for dispatch.

## Expected Results

All five success criteria hold. Model declarations come from the real
extraction, lift and reader, not from copied types or hand-built JSON. Linking
establishes static model binding only; evaluation, observation and protocol
assessment are verified elsewhere.

## Metadata

- Priority: P0
- Target Integration: quire-rs extraction, FCD extraction-frontend and semantic IR crates
- Automation: Automated Rust integration test

## Dependencies

**Upstream:** [FR-056](../functional/FR-056-admit-domain-package-model-declarations.md),
agent-ix/filament-core-data#172 and #173, and Quire specification FR-208 and
FR-154 (agent-ix/quire-specification PR #86, pending merge).
**Downstream:** [TC-148](../test-cases/TC-148-domain-package-end-to-end.md) and
the compiled-protocol `Model` identity under agent-ix/quire-spec-language#132.
