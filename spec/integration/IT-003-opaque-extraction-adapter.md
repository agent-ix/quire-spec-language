---
id: IT-003
title: "Consume a native clause from existing extraction"
type: IT
relationships:
  - target: "ix://agent-ix/quire-spec-language/FR-004"
    type: verifies
  - target: "ix://agent-ix/quire-spec-language/FR-011"
    type: verifies
---
# IT-003: Consume a native clause from existing extraction

## Objective

Actual extraction and parsing preserve correspondence without adding an expression compiler to the Markdown extractor.

## Target Integration

Existing Quire Rust extract_clauses connected to A's optional native consumer.
C retains changes to the producer and existing-repository CLI/wire adoption.

## Preconditions

Use FR-030 with the pinned Quire producer and accepted source-map contract.
The selected original document and compiler revisions must be available.
No manually extracted string substitutes for actual Quire extraction.

## Inputs

A Markdown document containing an indented native fence, non-ASCII original text, CRLF layout and a separately selected authored obligation.

## Test Procedure

1. Run actual extraction and native parsing (timeout 30 seconds).
   - IT-003-SC-01: The extracted clause produces a source-addressable native outcome.
2. Inspect the original location mapping (timeout 10 seconds).
   - IT-003-SC-02: Mapped bytes correspond to the selected authored document region.
3. Mutate the original document under a stale digest (timeout 30 seconds).
   - IT-003-SC-03: The adapter refuses source correspondence.
4. Run an unsupported native expression (timeout 30 seconds).
   - IT-003-SC-04: The original obligation remains present in the consumer outcome.

## Expected Results

Actual extraction and native compilation preserve original correspondence and
retain the producer's unchanged availability, diagnostics and clause population.

## Metadata

Priority: High. Automation: real command/file/API execution. Status: draft integration specification; each prerequisite and result requires observed evidence.

## Dependencies

- [FR-004](../functional/FR-004-verify-source-maps.md)
- [FR-011](../functional/FR-011-integrate-opaque-extraction.md)
