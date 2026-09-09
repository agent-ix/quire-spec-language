---
id: TC-072
title: "Compare native text records and identities"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-008
    type: verifies
---
# TC-072: Compare native text records and identities

## Description

Integration, priority P1. Verifies FR-008-AC-15. Planned until actual Rust
execution. Setup uses qualified public model/checker APIs; unrelated setup
failure cannot stand in for the intended phase's outcome.

## Test Procedure

Execute eligible equality/order over equal and differing Unicode scalar strings, composed/decomposed text, prefixes and supplementary scalars; compare nested eligible records with reordered input fields, enums and same/distinct object/reference identities across pre/post. Include source Option/Seq equality and cross-universe controls at checking.

## Expected Results

Text uses exact scalar lexicographic order without normalization. Record equality uses model field names/types. Same identity can compare equal across observations, while coincident labels in different universes cannot bypass checker eligibility. Distinct equal-valued objects remain unequal.
