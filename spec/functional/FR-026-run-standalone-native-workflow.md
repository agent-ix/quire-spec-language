---
id: FR-026
title: "Run a selected native workflow from files"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-002
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-025
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-024
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-023
    type: references
  - target: "ix://agent-ix/quire-specification/FR-301"
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-285
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-287
    type: references
---
## Description

When a caller hands QSL's library operation `command` (FR-287) a native-run/1 request file whose program source declares `0-draft` or no edition, `command` shall compile the selected model and native sources and execute the selected clause against verified runtime artifacts.

The run command routes each program source by the edition its header
declares, with FR-027's edition reader. This requirement owns the `0-draft`
route, which also takes a source declaring no edition. A `1-draft` program runs a named function through the spine under
[FR-100](FR-100-run-a-named-function-through-the-spine.md), which owns that
route's request members (`call`, `libraries`), outcome document
(`spine-run-result/1`; FR-286 owns the driver's `quire-outcome/1`)
and exit statuses. The members, outcome and exit
contract below apply to `0-draft` programs.

The code applies fixed 1 MiB, 64-file and
8 MiB ceilings; they become the configurable `request_bytes` and
`dependent_bytes` limits in Behavior (FR-026-AC-4).

The `0-draft` request and result identities carry `revision_namespace` and
`revision` (QSpec STD-150).

## Inputs

A JSON object with format and request fields. The closed request names model
sources with explicit native-rule-model/1 format, program source with complete
authored clause bindings, snapshot/invocation file selections and an execution
selection. Each source selects file, the two source labels of
[FR-001](FR-001-read-exact-source.md) (`authority` and `identity`), SHA-256
digest and formal document/revision. Both labels are required members. A
request whose source identity lacks one, such as a request written with only
`identity`, refuses at the request stage with the native code
`invalid-request`, exit code 20; the catalog keeps that code as a retained
host code with its broad meaning, malformed command
(`quire.native.diagnostics/v1`, "Other retained host/source codes").
Runtime selections use the existing typed references.
Relative paths resolve against the request's directory; absolute paths remain
valid local file operands. Source display paths retain the authored file string.
Parent-relative components are also admitted local operands; the command does
not establish a tenant filesystem boundary.
Optional limits lower validation work and evaluation expression steps; omitted
limits use the existing defaults. Null individual limit values also select those
defaults; a supplied limits record must be an object. Other stages use their
current default limits.
The optional package selection under [FR-028](FR-028-run-selected-native-package.md)
uses the existing verified reader; omission retains source compilation.

## Outputs

`command` returns a typed outcome, writing nothing to the process streams
(FR-287); the driver renders it and exits with its FR-285 exit code. The
outcome of a run carries one native-run-result/1 JSON result with request digest, compiled
package byte/static identities, original source/model/runtime identities
(each source and runtime artifact rendered with both labels and its digest),
selected authored clause, actual stage status, diagnostics, work counters and
ordered implication events. Only completed execution has Boolean truth.
On FR-301's six-code contract, for a `0-draft` program, the outcome's exit
code is 0 for completed true; 10 for completed false (a logical violation);
20 for refused, request syntax, identifier or I/O failure; 21 for a construct the parser recognizes
but the admitted profile does not support, or a native package naming an
unknown or unavailable required feature; 22 for incomplete. This result's diagnostics field can carry several diagnostics of
different classifications at once; when it does, the exit code is the
highest-severity classification present, on FR-301's ordering (tool failure,
invalid, unsupported, incomplete, violation, success) — an invalid diagnostic
always outranks an unsupported one, which always outranks an incomplete one,
regardless of how many of each are present or their order in the list.
An intake failure's outcome carries a JSON error with request digest when
read, typed stage/code and relevant original source/reference details.
Writing the outcome to a stream, and a stream that fails or closes, are the
driver's, specified by FR-287-AC-3 to FR-287-AC-9 (FR-287, Overlap). This output is a local native result, not a
portable verification envelope or assurance claim.
Serialized results are constructed from typed status/stage payloads; incomplete
or refused payloads cannot carry truth. Command error spellings belong to the
native code catalog. Resource causes distinguish file-byte and file-count limits.

## Behavior

The command shall decode the envelope and select its format before request fields.
The command shall admit, at decode, the members FR-100 defines for a `1-draft`
program, and shall apply the member rules of the program's edition after it
reads the program source's declared edition.
The command shall reject unknown/duplicate fields and positional record arrays.
The command shall open each file once and cap reads before parsing or hashing.
The command shall read the request under the caller's `request_bytes` limit
(default 1 MiB) and the dependent files it names under the caller's
`dependent_bytes` limit on their total size (default 8 MiB). The caller sets
both in the limits it gives `command`; neither has a ceiling. A refusal at
either names its setting (`request_bytes` or `dependent_bytes`) and its
configured value.
The command shall verify selected source and runtime digests before consumption.
The command shall use the existing source frontend, admission, parser, linker,
checker, package constructor and runtime execute APIs.
The command shall preserve refused and incomplete stage outcomes without truth.
The command shall begin each invocation with fresh budgets.
The command shall derive incomplete status consistently from its typed cause,
including incomplete native, model, package and runtime input failures.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-026-AC-1 | A file-driven aggregate example returns exact true/false outcomes, package/source/input identities and measured work using the real compiler/runtime. | Test (TC-103) |
| FR-026-AC-2 | Recorded pre/post operation files execute with immutable captures; illegal frame changes refuse without Boolean truth. | Test (TC-103) |
| FR-026-AC-3 | Stale source/input selections, malformed requests, unknown formats and unavailable files report their actual stage and catalogued code with available provenance; emitted outcomes conform to the native result schema. | Test (TC-104) |
| FR-026-AC-4 | Through `command` in process, a request one byte over `request_bytes`, and dependent files one byte over `dependent_bytes`, each refuse naming the setting and its configured value; the same requests with each limit raised to their size run; caller-lowered runtime limits stop with incomplete; a fresh default request succeeds. | Test (TC-104) |
| FR-026-AC-5 | `command` given one request file by its path from two different process working directories returns equal outcomes, the request's relative and absolute file paths resolving against the request's directory and as written; no call writes a byte to stdout or stderr. | Test (TC-103) |
| FR-026-AC-6 | A native-run/1 request whose program source, model source and snapshot selection each name `authority` `agent-ix` and their own `identity` runs, and its result renders each with those two labels and its digest, and no other identity member. The same request without the program source's `authority`, and again without the snapshot selection's `identity`, refuses at the request stage with `invalid-request`, exit code 20. A present but blank `authority` refuses with `invalid_source_identity`, cause `blank-label`, field `label` `authority` (FR-001). | Test (TC-430) |

## Dependencies

- [FR-025](FR-025-compile-rule-model-source.md): public model source frontend.
- [FR-024](FR-024-read-native-runtime-artifacts.md): selected runtime artifact intake.
- [FR-023](FR-023-run-native-packages.md): native execution and retained outcomes.
