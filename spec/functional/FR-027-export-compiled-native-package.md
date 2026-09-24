---
id: FR-027
title: "Export a compiled native package from selected source files"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-002
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-026
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-019
    type: references
  - target: "ix://agent-ix/quire-specification/FR-301"
    type: depends_on
---
## Description

When an author invokes quire-spec compile with a native-compile/1 request file, the command shall emit the exact native linked package bytes produced from the selected model and program sources.

Each selected source carries the four source labels of
[FR-001](FR-001-read-exact-source.md) under the request identity
[FR-026](FR-026-run-standalone-native-workflow.md) defines, and the
native-linked-package/1 `source` renders all four. The package bytes, the
schema and the golden vectors carry them.

## Inputs

A closed format/request JSON envelope. The request contains only models and
program, using the same source selections and complete authored clause bindings
as native-run/1. It contains no runtime artifacts or execution selection.
The command shares FR-026's request/file/count/aggregate limits, relative-path
resolution, source digest checks and existing model/compiler stage defaults.
Preflight receives explicit program/model/snapshot/invocation counts and retains
the exceeded category, requested count, remaining slots and total ceiling.

## Outputs

Successful stdout is exactly NativePackage::bytes in native-linked-package/1
format, with no command wrapper or extra newline; exit 0. A consumer selects
those bytes by their SHA-256 digest and may reread them using the existing
NativePackage::read_verified API with explicit source/model bindings.
Intake and static failures emit no package bytes and use FR-026's existing native-run-result/1
command-error envelope on stderr, including the original request digest and
stage/code when available. On FR-301's six-code contract, refusal, malformed
requests, I/O failures and invalid command usage all exit 20; a selected model
naming a real, profile-gated capability this build does not admit — a later
native model profile or a rational scalar/value declared outside its admitting
profile — exits 21; incomplete (exhausted) failures exit 22. Broken pipes end
quietly; other write and output-serialization failures exit 30 and may leave a
partial stdout prefix, which is not a complete selected artifact.

## Behavior

The compiler command shall select native-compile/1 before decoding request fields.
Each request type shall own its selector from the shared native format catalog.
The compiler command shall use the same source frontend and static compiler path
as the run command, without reading or constructing runtime artifacts.
File intake shall own bounded reading.
The command's compilation module shall own model admission and parse/link/check/package orchestration.
The compiler command shall finish package construction before writing its bytes.
If a static stage fails, then the compiler command shall return its existing
typed failure without a successful artifact.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-027-AC-1 | CLI output matches the existing public static pipeline byte-for-byte and is accepted by the existing verified package reader with explicit bindings. | Test |
| FR-027-AC-2 | A directory containing only selected sources and its compile request produces the package; native-run/1 and unexpected runtime fields refuse at the command boundary. | Test |
| FR-027-AC-3 | Stale source and malformed syntax return original codes with empty stdout; file-count exhaustion identifies its category; compile arity errors precede I/O and exit 20; output failures exit 30; existing run and parse/format tests still pass. | Test |
| FR-027-AC-4 | A native-compile/1 request whose program source names `authority` `agent-ix`, `identity` `p`, `revision_namespace` `git` and `revision` `1` emits package bytes whose `source` names those four labels, and the package validates against the native-linked-package/1 schema. The same request without `revision_namespace` refuses with `invalid-request` and exits 20. | Test (TC-430) |

## Dependencies

- [FR-026](FR-026-run-standalone-native-workflow.md): bounded local command intake.
- [FR-019](FR-019-package-checked-native-clauses.md): native package construction.
- [FR-020](FR-020-read-and-rebind-native-packages.md): verified consumer intake.

## Status

FR-027-AC-4 is specified under QSL-233 and not implemented; ADR-013 §7 slice S-4b builds it.
