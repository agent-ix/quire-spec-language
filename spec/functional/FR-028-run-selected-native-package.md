---
id: FR-028
title: "Run a verified selected native package"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-002
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-026
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-020
    type: references
---
## Description

When a native-run/1 request selects a compiled package, the command shall verify that artifact against the supplied source, authored bindings and admitted models before executing it.

## Inputs

An optional package member on the existing run request: a closed object with
file and canonical sha256-prefixed digest strings. Omission preserves source
compilation; a supplied null, positional array or malformed object refuses.
The original program source, complete clause bindings and model sources remain
required external authority. Native-compile/1 requests still reject this field.
All file operands follow FR-026: relative paths resolve from the request's
directory, including parent-relative paths; absolute paths remain absolute.
This local command does not confine selected files to a request-directory sandbox.

## Outputs

The existing native-run-result/1 outcome using the accepted package's exact raw
byte digest and reconstructed static identity. Equivalent JSON layouts retain
their own selected byte digest. Reader failures retain the original PackageError
and selected file/reference in the library error; command JSON exposes their
code, reader stage, path and nested cause under the distinct command stage
selected_package. Source-compilation package failures retain stage package.
File/I/O/intake failures retain the
existing FR-026 error behavior. Failed artifact verification performs no runtime
execution and emits no predicate truth.

## Behavior

The command shall count the selected package toward the existing 64-file and
8 MiB aggregate intake ceilings.
File-count preflight shall identify the exhausted named group, including packages.
The command shall call NativePackage::read_verified with the expected byte
reference, externally derived CheckBindings, admitted models and existing
default support/stage limits.
The command shall execute only the package accepted by that reader.
If selected bytes or reconstructed claims fail verification, then the command shall return the original refusal without falling back to source compilation.
The command shall preserve fresh budgets for subsequent requests.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-028-AC-1 | Selected exported state/operation packages produce the expected true, false and frame-refused outcomes through the real binary. | Test |
| FR-028-AC-2 | An equivalent alternate package layout retains its selected byte digest while its static identity and runtime observations agree with source compilation. | Test |
| FR-028-AC-3 | Stale bytes, forged claims with recomputed digest, incompatible source/bindings and malformed package selections refuse without fallback, preserving actual reader details where reached. | Test |
| FR-028-AC-4 | Selected-file/count/byte and runtime limits retain incomplete outcomes with fresh successful retry; omitted selection preserves run and compile behavior. | Test |

## Dependencies

- [FR-026](FR-026-run-standalone-native-workflow.md): local run orchestration.
- [FR-020](FR-020-read-and-rebind-native-packages.md): actual artifact reconstruction.
- [FR-027](FR-027-export-compiled-native-package.md): standalone package producer.

## Status

The selected package's sources carry the four source labels of FR-001 (FR-020). Implemented under QSL-233 (ADR-013 §7 S-4b) through FR-020's reader.
