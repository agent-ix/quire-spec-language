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

When an author invokes quire-spec compile with a native-compile/1 request file, the command shall compile the selected program source by the edition its header declares. A `1-draft` source compiles through the spine (S1 to S4, ADR-011 §7.3 M-6a) into `quire.checked-package/v2` bytes. A `0-draft` source compiles into the exact native linked package bytes produced from the selected model and program sources. Native compile is deleted with M-6c and QSL-5 (ADR-011, owner ruling 2026-09-24).

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

For a `0-draft` program, successful stdout is exactly NativePackage::bytes in
native-linked-package/1 format, with no command wrapper or extra newline; exit 0.
For a `1-draft` program, successful stdout is exactly the
`quire.checked-package/v2` bytes `command::spine::compile` writes for that
source, with no command wrapper or extra newline; exit 0. A `1-draft` request
selects no clause bindings, and each model it selects is a domain package
document in format `semantic-ir/2.0.0`, read under its source digest and handed
to spine `compile` as FR-056's package input; the program's `model`
declarations select from it by `sha256-jcs` digest, and the v2 lock's
`model_selections` names each selected package by identity, version and that
digest. A `1-draft` compile validates the
program selection's `document` and `formal_revision` but does not record them:
the `quire.checked-package/v2` wire has no member for them, so two requests that
differ only in those fields write identical bytes. A spine refusal uses the same
command-error envelope, with the refusing spine stage (`source`, `forms`,
`intake`, `assembly`, `check` or `emit`) as its stage and that stage's cause code as its
code. A consumer selects
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
The compiler command shall read the program source's declared edition once,
from its header, after the source's digest check and before any model source
is read or either compiler runs. Each program source goes to exactly one compiler. If the header declares
an edition other than `0-draft` or `1-draft`, then the compiler command shall
refuse with `unknown_edition` at the edition literal, naming the file and the
edition.
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
| FR-027-AC-5 | A native-compile/1 request whose program source declares `edition "1-draft"` and holds a record, an Integer function and a function with parameters writes exactly the bytes `command::spine::compile` returns for that source, and QSL's I2 reader reads those bytes back Verified with no node omitted. Two such requests that differ only in the program's `document` and `formal_revision` write identical bytes. | Test (TC-435) |
| FR-027-AC-6 | A program source declaring `edition "0-draft"` compiles through native compile, and its bytes equal the native static pipeline's (FR-027-AC-1). | Test (TC-435) |
| FR-027-AC-7 | A program source declaring any other edition refuses with `unknown_edition`, exit 20, empty stdout, and a message naming the file and the edition. A `1-draft` request selecting native rule models or clause bindings refuses with `invalid-request`, exit 20, whatever state the model files are in. | Test (TC-435) |
| FR-027-AC-8 | A `1-draft` source each spine stage refuses (`source`, `forms`, `assembly`, `check`, `emit`) exits with that stage's cause code and reports the stage, with empty stdout. | Test (TC-435) |
| FR-027-AC-9 | A `1-draft` request selecting a `semantic-ir/2.0.0` domain package document whose program declares `model M` by that document's `sha256-jcs` digest writes exactly the bytes `command::spine::compile` returns over the same source and package input. Their lock and identity preimage `model_selections` hold that package's identity, version, `sha256-jcs` and digest, and QSL's I2 reader, given that digest as domain package evidence, reads them back Verified. `M::Nope` refuses at stage `assembly` (`missing_declaration`) at `M::Nope`; a missing or different document and a `sha256:` digest refuse at stage `intake` at the `model` declaration. | Test (TC-442) |

## Dependencies

- [FR-026](FR-026-run-standalone-native-workflow.md): bounded local command intake.
- [FR-019](FR-019-package-checked-native-clauses.md): native package construction.
- [FR-020](FR-020-read-and-rebind-native-packages.md): verified consumer intake.
- [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md) §7.3 M-6a: the spine route for `1-draft` sources.
- [FR-056](FR-056-admit-domain-package-model-declarations.md): domain package admission, which spine intake runs.

## Status

FR-027-AC-4 is implemented under QSL-233 (ADR-013 §7 slice S-4b) and backed by TC-430.
FR-027-AC-5 to FR-027-AC-8 are implemented under QSL-8 (ADR-011 §7.3 M-6a) and backed by TC-435.
