---
id: TC-060
title: "Validate every supplied typed value"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-007
    type: verifies
---
# TC-060: Validate every supplied typed value

## Description

Property, priority P1. Verifies FR-007-AC-7, FR-007-AC-8. Planned until actual Rust
execution. Setup uses qualified public model/checker APIs; unrelated setup
failure cannot stand in for the intended phase's outcome.

## Test Procedure

Generate each supported model type at object fields, State roots, parameters and results, including nested structural records, Option and Seq wrappers. Apply independent missing/extra/duplicate field, wrong primitive/nominal/enum, absent/present, signed bound, Unicode scalar bound, sequence maximum and reference-kind mutations. Reuse one arena node at different model sites and observations; retain skipped-field controls.

## Expected Results

Every required/supplied selected root is checked under its exact type and capture. No type inference comes from first runtime use. Invalid mutations refuse at validation, while structurally admitted missing observations retain their own incomplete diagnosis. Empty/duplicate-preserving sequences and exact scalar endpoints remain valid.
