---
id: TC-034
title: "Retain ambiguity provenance without partial linkage"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-013
    type: verifies
---

## Description

Retain ambiguity provenance without partial linkage. Integration/property controls, priority P1. Traces: FR-013-AC-6.
Authored before implementation; now executed with the public link API and actual IR constructors.

## Test Procedure

Construct two independently valid environments with distinct RequirementRef owners and declaration source revisions, both exporting BoundedCounter. Supply exact imports under the same native alias. Permute import/environment order and the position of otherwise valid neighboring clauses, with a successful earlier call. Also duplicate an exact catalog candidate.

## Expected Results

Ambiguity returns the authored occurrence and all conflicting typed declaration identities/IR source spans, in deterministic identity order. No candidate wins and no partial package escapes, regardless of order or prior calls.
