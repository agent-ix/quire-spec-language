---
id: FR-027
title: "Compile a 1-draft program source into a checked package"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-002
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-026
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-278
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-285
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-287
    type: references
  - target: "ix://agent-ix/quire-specification/FR-301"
    type: depends_on
---
## Description

When a caller hands QSL's library operation `command` (FR-287) a native-compile/1 request file, `command` shall compile the selected program source by the edition its header declares. A `1-draft` source compiles through the spine (S1 to S4, ADR-011 §7.3 M-6a) into `quire.checked-package/v2` bytes. Every other declared edition, `0-draft` included, is one no compiler reads, and `command` refuses it with `unknown_edition`. A source that does not open with a `language "ix:native" edition` header goes to the spine, whose S1 refuses it.

A `1-draft` request's `program` is one source selection or a non-empty array of source selections: the package unit list in request order (FR-480). Every unit of the list compiles into the one checked package.

Each selected source carries the two source labels of
[FR-001](FR-001-read-exact-source.md) under the request identity
[FR-026](FR-026-run-standalone-native-workflow.md) defines.

The driver's `quire compile` reaches `command` (FR-287); `command` returns a
typed outcome and writes nothing to the process streams. A `1-draft` native-compile/1
request is the command encoding of the `parse`, `select`, `check` and
`package` operations composed (FR-278). Every exit status this requirement
states is FR-285's exit code of the outcome's ADR-013 O-16 category.

## Inputs

A closed format/request JSON envelope. The request contains only models,
program and, for a `1-draft` program, `libraries`, using the same source selections and complete authored clause bindings
as native-run/1. It contains no runtime artifacts or execution selection.
The command shares FR-026's `request_bytes` and `dependent_bytes` intake
limits, relative-path resolution, source selection checks and existing
model/compiler stage defaults. A refusal at an intake limit names the
limit's setting (`request_bytes` or `dependent_bytes`) and its configured
value.

## Outputs

For a `1-draft` program, the successful outcome's package artifact is exactly the
`quire.checked-package/v2` bytes the FR-278 composition writes for that
source, with no command wrapper or extra newline; exit code 0. A `1-draft`
program's `program` carries no `clauses` key at all -- any presence,
including an empty array, refuses with `invalid-request` (FR-027-AC-7),
consistent with FR-100's `1-draft` request. A compile request whose
`program` carries an `extraction` member refuses `extraction-requires-run`
at stage `request`, exit 20, before it reads a dependent file, in every
build (FR-031). Each model a `1-draft` request
selects is a domain package
document in format `semantic-ir/2.0.0`, read and handed
to spine `compile` as FR-056's package input; the program's `model`
declarations select it by identity, and the v2 lock's
`model_selections` names each selected package by identity. A `1-draft` request may carry `libraries`: one
`{identity, source}` object per supplied library, where `source` is
the same source selection a model or the program uses (file and the FR-001
labels), and each library object may carry `refinement_internals`, a JSON
value. A `1-draft` request may also carry `refinement_internals` at its top
level for the program's own refinements. The command hands the libraries
to spine `compile` as its dependency input and the values as its refinement
internals input, unchanged (FR-099, ADR-015 D-1). A
request with no `libraries` supplies none, and one with no
`refinement_internals` supplies the empty list. Each library source counts toward
FR-026's `dependent_bytes` limit. A `1-draft` request
with a library of an empty identity, or with a library object that has any
member besides `identity`, `source` and `refinement_internals` (a `version`, for one), refuses with `invalid-request`. A `1-draft` compile validates the
program selection's `document` and `formal_revision` but does not record them:
the `quire.checked-package/v2` wire has no member for them, so two requests that
differ only in those fields write identical bytes. A spine refusal uses the same
command-error envelope, with the refusing spine stage (`source`, `forms`,
`intake`, `assembly`, `check` or `emit`) as its stage and that stage's cause code as its
code.
Intake and static failures carry no package bytes, and their outcome carries
the artifact-command refusal envelope: `format` is `native-run-result/2`, `status` is
`refused` or `incomplete`, alongside the stage and code. This artifact envelope
is outside FR-267's closed `run` command-error reader. On FR-301's six-code
contract, the outcome's exit code is 20 for refusal, malformed
requests and I/O failures; a selected model
naming a real, profile-gated capability this build does not admit — a later
native model profile or a rational scalar/value declared outside its admitting
profile — has exit code 21; incomplete (exhausted) failures have exit code 22.
Writing the package bytes to a stream, and a stream that fails or closes,
are the driver's (FR-287, Overlap).

## Behavior

The compiler command shall select native-compile/1 before decoding request fields.
The compiler command shall read each program unit's declared edition once, in request order, after its source selection check and before any model source is read. Only a non-empty program list whose units all declare `1-draft` reaches the compiler; the first other edition refuses with `unknown_edition` at that unit's edition literal, before header agreement is checked. An empty list refuses with `invalid-request`. If the header declares
an edition other than `1-draft`, `0-draft` included, then the compiler command shall
refuse with `unknown_edition` at stage `profile` at the edition literal, naming the file and the
edition, whatever members the request carries. If the source does not open
with a `language "ix:native" edition "<edition>"` header, then the compiler
command shall hand it to the spine, and S1's refusal of the header is the
command's refusal, at stage `source`.
Each request type shall own its selector from the shared native format catalog.
The compiler command shall use the same source frontend and static compiler path
as the run command, without reading or constructing runtime artifacts.
File intake shall own bounded reading.
The command's compilation module shall own model admission and parse/link/check/package orchestration.
The compiler command shall finish package construction before returning an outcome that carries its bytes.
If a static stage fails, then the compiler command shall return its existing
typed failure without a successful artifact.

Each library source file SHALL count toward `dependent_bytes` by its size,
with the model and program sources; every unit in an array `program` counts
as a program source. No limit counts library files.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-027-AC-2 | Through `command` in process, a directory containing only the selected `1-draft` sources and their compile request compiles, exit code 0. A native-run/1 request given to `compile`, and a native-compile/1 request carrying `snapshots`, `invocations`, `selection` or `package`, each refuse `invalid-request` at the request stage, exit code 20, with no package bytes, without reading a model or runtime file. | Test (TC-105) |
| FR-027-AC-3 | Through `command` in process, for a `1-draft` request, stale source and malformed syntax return their original codes in an outcome with no package bytes; a program source without `identity`, and one without `authority`, refuses `invalid-request`, exit code 20; a request past `dependent_bytes` refuses naming the setting `dependent_bytes` and its configured value, and the same request with the limit raised to its size compiles; no call writes a byte to stdout or stderr. | Test (TC-105) |
| FR-027-AC-5 | A native-compile/1 request whose program source declares `edition "1-draft"` and holds a record, an Integer function and a function with parameters writes exactly the bytes the FR-278 composition returns for that source, and QSL's I2 reader reads those bytes back Verified with no node omitted. Two such requests that differ only in the program's `document` and `formal_revision` write identical bytes. | Test (TC-435) |
| FR-027-AC-7 | A program source declaring `edition "0-draft"`, the same source in a request carrying a library and `refinement_internals`, and a source declaring `edition "7-draft"` each refuse with `unknown_edition` at stage `profile`, exit code 20, no package bytes, and a message naming the file and the edition. A source with no header refuses `invalid_syntax` and one declaring `language "ix:other"` refuses `unknown_language`, each at stage `source`, exit code 20, no package bytes. A `1-draft` request selecting native rule models or clause bindings refuses with `invalid-request`, exit code 20, whatever state the model files are in. | Test (TC-435) |
| FR-027-AC-8 | A `1-draft` source each spine stage refuses (`source`, `forms`, `assembly`, `check`, `emit`) returns that stage's cause code and reports the stage, with no package bytes. | Test (TC-435) |
| FR-027-AC-9 | A `1-draft` request selecting a `semantic-ir/2.0.0` domain package document whose program declares `model M` by that document's identity writes exactly the bytes the FR-278 composition returns over the same source and package input. Their lock `model_selections` hold that package's identity, and QSL's I2 reader, given that package as domain package evidence, reads them back Verified. `M::Nope` refuses at stage `assembly` (`missing_declaration`) at `M::Nope`; a missing document or an identity that names no supplied document refuses at stage `intake` at the `model` declaration. | Test (TC-442) |
| FR-027-AC-10 | A `1-draft` request whose program imports `test/geometry` and whose `libraries` supplies it by file and labels writes exactly the bytes the FR-278 composition returns over the same source and dependency input, exit code 0. A `1-draft` request with a library of an empty identity refuses with `invalid-request`, exit code 20, no package bytes; so does one with a library object that has any member besides `identity`, `source` and `refinement_internals`, a `version` for one. | Test (TC-446) |
| FR-027-AC-12 | A `1-draft` request carrying `refinement_internals` at its top level and on a library object hands those values to spine `compile` unchanged, and writes exactly the bytes the FR-278 composition returns over the same source, dependency input and refinement internals input, exit code 0; a request with the member absent supplies the empty list. | Test (TC-446) |
| FR-027-AC-11 | For FR-027-AC-5's, AC-9's and AC-10's requests, the bytes the command writes equal the `package` output of `parse`, `select`, `check` and `package` composed over the same source, domain packages and libraries; for FR-027-AC-8's sources, the refusing operation's stage and cause code equal the command's, and its FR-285 exit code equals the command's exit status. | Test (TC-785) |
| FR-027-AC-13 | A `1-draft` request whose program source and selected models total less than `dependent_bytes`, and whose one `libraries` source makes the total of all dependent files one byte over it, refuses at intake naming `dependent_bytes` and its configured value, with no package bytes; the same request with `dependent_bytes` raised to that total returns the package bytes FR-027-AC-10 states, exit 0. A `1-draft` request whose program imports 65 libraries, each supplied as its own small `libraries` source file, with every dependent file totalling less than `dependent_bytes`, compiles at the default limits, exit 0: no intake limit counts its files. | Test (TC-105) |

| FR-027-AC-14 | Through `command`, a `1-draft` request whose `program` is an array of FR-480-AC-1's units `U1` and `U2` writes exactly the bytes the FR-278 composition returns over that unit list, exit 0, and the same request with the array reversed writes the same bytes. An array whose second unit declares `edition "0-draft"` refuses `unknown_edition` at that unit's edition literal, naming its file, with no package bytes and before FR-480's header agreement is checked; an empty array refuses `invalid-request`; and an array whose two units exceed `dependent_bytes` together, though each fits alone, refuses at intake naming `dependent_bytes`. | Test (TC-1210) |

## Dependencies

- [FR-026](FR-026-run-standalone-native-workflow.md): bounded local command intake.
- [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md) §7.3 M-6a: the spine route for `1-draft` sources.
- [FR-056](FR-056-admit-domain-package-model-declarations.md): domain package admission, which spine intake runs.
- [FR-099](FR-099-compile-against-supplied-libraries.md): the dependency input `libraries` supplies.
- [FR-278](FR-278-parse-select-check-and-package-as-library-operations.md): the library operations the command encodes.
- [FR-285](FR-285-map-every-outcome-category-to-one-exit-code.md): the exit function.
- [FR-287](FR-287-reach-qsl-through-the-driver-cli.md): the driver CLI.
