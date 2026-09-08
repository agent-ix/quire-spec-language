---
id: IT-002
title: "Run the supported native state workflow"
type: IT
relationships:
  - target: "ix://agent-ix/quire-spec-language/FR-005"
    type: verifies
  - target: "ix://agent-ix/quire-spec-language/FR-006"
    type: verifies
  - target: "ix://agent-ix/quire-spec-language/FR-007"
    type: verifies
  - target: "ix://agent-ix/quire-spec-language/FR-008"
    type: verifies
  - target: "ix://agent-ix/quire-spec-language/FR-009"
    type: verifies
---
# IT-002: Run the supported native state workflow

## Objective

The workflow has real healthy, violating and refused/incomplete observations with exact source/model identity. Any missing stage remains an unmet setup condition; authored expected data is insufficient.

## Target Integration

Native parse/link/reference-evaluate pipeline and the existing qualified executable binder/backend.

## Preconditions

FS02/FS03/FS05 and model/source/frame adapters must be reviewed. Actual native linking, evaluation and qualified backend entry points must be available. Current parser-only functionality does not satisfy this setup.

## Inputs

Real compiled ConfigVersion model, exact authored parent-order clause, complete healthy/violating populations, and refused/incomplete mutation cases.

## Test Procedure

1. Build every selected pipeline component (timeout 180 seconds).
   - IT-002-SC-01: Recorded component revisions match the requested execution plan.
2. Run the healthy case through the full real pipeline (timeout 30 seconds).
   - IT-002-SC-02: The completed reference result is true.
3. Run the violating case through the same pipeline (timeout 30 seconds).
   - IT-002-SC-03: The completed reference result is false.
4. Run missing-model, dangling-reference and exhausted-budget controls (timeout 30 seconds).
   - IT-002-SC-04: Every control has its specified refused or incomplete outcome.
5. Compare the qualified backend observations (timeout 30 seconds).
   - IT-002-SC-05: The backend preserves the selected logical and source-activation semantics.

## Expected Results

The workflow has real healthy, violating and refused/incomplete observations with exact source/model identity. Any missing stage remains an unmet setup condition; authored expected data is insufficient.

## Metadata

Priority: High. Automation: real command/file/API execution. Status: draft integration specification; each prerequisite and result requires observed evidence.

## Dependencies

- [FR-005](../functional/FR-005-link-shared-model.md)
- [FR-006](../functional/FR-006-check-defined-expressions.md)
- [FR-007](../functional/FR-007-validate-runtime-inputs.md)
- [FR-008](../functional/FR-008-evaluate-state-reference.md)
- [FR-009](../functional/FR-009-lower-qualified-projections.md)
