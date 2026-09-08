---
id: IT-001
title: "Reproduce the bounded model with the existing producer"
type: IT
relationships:
  - target: "ix://agent-ix/quire-spec-language/FR-005"
    type: verifies
---
# IT-001: Reproduce the bounded model with the existing producer

## Objective

The existing producer emits the selected bounded declaration package. This is real package production; native typed-model binding remains a separate unmet requirement.

## Target Integration

Existing Filament TypeSpec frontend invoked as a real process over original model source, manifest and lock files.

## Preconditions

Select the clean Filament checkout at 3b75e01c652ba00bb07c352ff5467419401e792b and its installed TypeSpec 1.15.0 toolchain. The native fixture source/output files must be present. File I/O and compiler execution are real.

## Inputs

tests/fixtures/model-source and model-output; the stale-lock control modifies a temporary copy of the lock fingerprint.

## Test Procedure

1. Run tools/check_model_fixture.py with the pinned Filament checkout (timeout 180 seconds).
   - IT-001-SC-01: Fresh compilation reproduces the selected IR bytes.
2. Inspect the helper's selected-lock comparison (timeout 10 seconds).
   - IT-001-SC-02: Selected-lock output matches the fresh output.
3. Inspect the stale-lock control (timeout 10 seconds).
   - IT-001-SC-03: The compiler emits STALE_LOCK without an IR output file.

## Expected Results

The existing producer emits the selected bounded declaration package. This is real package production; native typed-model binding remains a separate unmet requirement.

## Metadata

Priority: High. Automation: real command/file/API execution. Status: draft integration specification; each prerequisite and result requires observed evidence.

## Dependencies

- [FR-005](../functional/FR-005-link-shared-model.md)
