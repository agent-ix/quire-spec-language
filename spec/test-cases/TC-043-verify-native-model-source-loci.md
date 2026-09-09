---
id: TC-043
title: "Verify model loci and inventory identity consistency"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-015
    type: verifies
---
# TC-043: Verify model loci and inventory identity consistency

## Description

Integration, priority P1. Verifies FR-015-AC-4. Qualified at 0cd679c; SR-085
records the actual public-API controls, local gates and bounded claim. Tests use
the source-derived Rust producer and successfully constructed actual IR inputs.

## Test Procedure

Give a valid source-bound IR model a false line, column, split-scalar or foreign source/revision locus, including an unused field/variant and a native role. Then supply two individually valid models whose formal source identity denotes different native labels/revision/bytes, or whose same formal RequirementRef denotes different native artifacts. Include two identical copies and two independent valid owners as controls.

## Expected Results

False/foreign loci refuse model construction. Conflicting identity bindings refuse the linked inventory as invalid_model_binding. Identical candidate copies retain the existing ambiguous_declaration outcome for an exact import, and independent valid owners are accepted. No first/last-wins reconciliation or partial LinkedPackage is exposed.
