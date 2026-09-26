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
---
## Description

When an author invokes quire-spec run with a native-run/1 request file whose program source declares `0-draft` or no edition, the command shall compile the selected model and native sources and execute the selected clause against verified runtime artifacts.

The run command routes each program source by the edition its header
declares, with FR-027's edition reader. This requirement owns the `0-draft`
route, which also takes a source declaring no edition. A `1-draft` program runs a named function through the spine under
[FR-100](FR-100-run-a-named-function-through-the-spine.md), which owns that
route's request members (`call`, `libraries`), outcome document
(`spine-run-result/1`) and exit statuses. The members, outcome and exit
contract below apply to `0-draft` programs.

## Inputs

A JSON object with format and request fields. The closed request names model
sources with explicit native-rule-model/1 format, program source with complete
authored clause bindings, snapshot/invocation file selections and an execution
selection. Each source selects file, the four source labels of
[FR-001](FR-001-read-exact-source.md) (`authority`, `identity`,
`revision_namespace` and `revision`), SHA-256 digest and formal
document/revision. Every label is a required member. A request whose source
identity lacks one, such as a request written with only `identity` and
`revision`, refuses at the request stage with the native code
`invalid-request` and exits 20; the catalog keeps that code as a retained
host code with its broad meaning, malformed command
(`quire.native.diagnostics/v1`, "Other retained host/source codes"). The
wire is prerelease and carries no version change for this: a two-label
request is simply malformed. Runtime selections use the existing typed references.
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

One native-run-result/1 JSON result on stdout with request digest, compiled
package byte/static identities, original source/model/runtime identities
(each source and runtime artifact rendered with all four labels),
selected authored clause, actual stage status, diagnostics, work counters and
ordered implication events. Only completed execution has Boolean truth.
On FR-301's six-code contract, for a `0-draft` program, exit 0 means completed
true; 10 means completed false (a logical violation); 20 means refused, invalid command usage, request
syntax, identifier or I/O failure; 21 means a construct the parser recognizes
but the admitted profile does not support, or a native package naming an
unknown or unavailable required feature; 22 means incomplete. Output failure
exits 30. This result's diagnostics field can carry several diagnostics of
different classifications at once; when it does, the exit code is the
highest-severity classification present, on FR-301's ordering (tool failure,
invalid, unsupported, incomplete, violation, success) — an invalid diagnostic
always outranks an unsupported one, which always outranks an incomplete one,
regardless of how many of each are present or their order in the list.
Intake failures use a JSON error on stderr with request digest when read, typed
stage/code and relevant original source/reference details. Existing parse/format
commands preserve their behavior. Broken pipes end quietly with the computed
status. This output is a local native result, not a
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
The command shall limit the request to 1 MiB, dependent files to 64, and aggregate
dependent bytes to 8 MiB, also applying each stage's existing byte ceiling.
The command shall verify selected source and runtime digests before consumption.
The command shall use the existing source frontend, admission, parser, linker,
checker, package constructor and runtime execute APIs.
The command shall preserve refused and incomplete stage outcomes without truth.
The command shall begin each invocation with fresh budgets.
The CLI shall parse its command and operands once before I/O and report usage
for the selected command when operands are missing or excessive.
The command shall derive incomplete status consistently from its typed cause,
including incomplete native, model, package and runtime input failures.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-026-AC-1 | A file-driven aggregate example returns exact true/false outcomes, package/source/input identities and measured work using the real compiler/runtime. | Test |
| FR-026-AC-2 | Recorded pre/post operation files execute with immutable captures; illegal frame changes refuse without Boolean truth. | Test |
| FR-026-AC-3 | Stale source/input selections, malformed requests, unknown formats and unavailable files report their actual stage and catalogued code with available provenance; emitted outcomes conform to the native result schema. | Test |
| FR-026-AC-4 | Request/file/read and caller-lowered runtime limits stop with incomplete; a fresh default request succeeds. | Test |
| FR-026-AC-5 | Existing parse/format behavior remains, command-specific arity errors precede I/O, and relative and absolute file operands resolve independently of the working directory. | Test |
| FR-026-AC-6 | A native-run/1 request whose program source, model source and snapshot selection each name `authority` `agent-ix`, their own `identity`, `revision_namespace` `git` and `revision` `1` runs, and its result renders each with those four labels. The same request without the program source's `authority`, and again without the snapshot selection's `revision_namespace`, refuses at the request stage with `invalid-request` and exits 20. A present but blank `authority` refuses with `invalid_source_identity`. | Test (TC-430) |

## Dependencies

- [FR-025](FR-025-compile-rule-model-source.md): public model source frontend.
- [FR-024](FR-024-read-native-runtime-artifacts.md): selected runtime artifact intake.
- [FR-023](FR-023-run-native-packages.md): native execution and retained outcomes.

## Status

FR-026-AC-6 is implemented under QSL-233 (ADR-013 §7 slice S-4b) and backed by TC-430.
