---
id: TC-070
title: "Preserve sequence order and multiplicity"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-008
    type: verifies
---
# TC-070: Preserve sequence order and multiplicity

## Description

Property, priority P1. Verifies FR-008-AC-13. Planned until actual Rust
execution. Setup uses qualified public model/checker APIs; unrelated setup
failure cannot stand in for the intended phase's outcome.

## Test Procedure

Generate short sequences over three Version scalar values, including every empty/singleton/repeated/permuted sequence up to length three. Evaluate size, forall and exists with an implication predicate whose events reveal occurrence order and short circuiting. Independently enumerate expected events/truth from the authored sequence. Additional reference-element cases first admit a separately source-derived Seq of NodeRef model through the existing Rust producer; they do not assume the baseline scalar items field has reference elements.

## Expected Results

Size counts occurrences, duplicates remain observable and iteration follows input order. Forall stops at the first false, exists at the first true; empty values are true/false respectively. Missing data is incomplete during validation, never an empty sequence.
