---
id: FR-029
title: "Export the existing Boolean executable projection"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-009
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-027
    type: references
---
## Description

When an author invokes quire-spec lower with a native-compile/1 request file, the command shall compile the selected sources and export the existing boolean-oracle/v1 projection through the strict IR binder.

## Inputs

The same closed source-only request as compile, with the existing model/program
selections, authored bindings and intake/static defaults. The target is fixed to
boolean-oracle/v1 and uses the existing LoweringLimits defaults.

## Outputs

Success is exactly NativeProjection::bytes, the existing executable-projection/v1
wire accepted by BoundPackage::from_json_bytes; exit 0, no wrapper or newline.
This is a derived executable artifact, not the native package or proof evidence.
Compile remains the command for retaining native model/runtime obligations.
Lowering refusal emits no projection bytes and retains the exact request digest,
native package reference, original formal/program source and typed LoweringError.
The command error JSON uses stage lower and codes unsupported_projection,
resource_exhausted, projection_binding or invalid_projection_correspondence,
with authored clause, located native span and original upstream diagnostics.
Existing intake errors and stdout I/O behavior follow FR-027.

## Behavior

The command shall share compile's source-only intake and native package construction.
The command shall invoke the existing lower function without reimplementing expressions or widening its admitted domain.
If any clause is unsupported or lowering fails, then the command shall return the original failure without partial projection or fallback.
The command shall finish strict binding before writing output.
The command shall return exit 3 for exhausted budgets and exit 1 for lowering refusals.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-029-AC-1 | Actual lower-command bytes match library lowering, pass both pinned IR readers and generate the complete existing backend oracle population without runtime files. | Test |
| FR-029-AC-2 | A later unsupported clause refuses the complete projection with empty stdout and original request/package/source/clause identity; a fresh valid retry succeeds. | Test |
| FR-029-AC-3 | Wrong request format, runtime fields, stale source and intake limits retain existing error/exit behavior; compile/run/parse/format remain usable. | Test |

## Dependencies

- [FR-009](FR-009-lower-qualified-projections.md): admitted target and strict binding.
- [FR-027](FR-027-export-compiled-native-package.md): source-only command contract.
