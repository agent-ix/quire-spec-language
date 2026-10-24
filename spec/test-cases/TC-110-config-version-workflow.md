---
id: TC-110
title: "Generate concrete ConfigVersion cases"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-032
    type: verifies
---
## Description

Integration, P1: generate explicit ConfigVersion model and input artifacts,
admit the model with the actual frontend, and verify repeatable fixture files.
TC-469 owns the independent clause-run judgments and FR-267 result round trips.

## Test Procedure

Inspect the admitted model's source-corresponded roles. Generate each catalog
case with its distinct input identity, check generator output failure through
the actual example binary, and compare repeated healthy artifact bytes.

## Expected Results

The actual frontend admits the named model's integer, reference and frame
roles. The generator emits repeatable case files and reports filesystem
failures. TC-469 verifies the case dispositions through the clause spine.
