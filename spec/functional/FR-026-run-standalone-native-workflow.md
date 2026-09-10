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
---
## Description

When an author invokes quire-spec run with a native-run/1 request file, the command shall compile the selected model and native sources and execute the selected clause against verified runtime artifacts.

## Inputs

A JSON object with format and request fields. The closed request names model
sources with explicit native-rule-model/1 format, program source with complete
authored clause bindings, snapshot/invocation file selections and an execution
selection. Each source selects file, native identity/revision, SHA-256 digest and
formal document/revision. Runtime selections use the existing typed references.
Relative paths resolve against the request's directory; absolute paths remain
valid local file operands. Source display paths retain the authored file string.
Parent-relative components are also admitted local operands; the command does
not establish a tenant filesystem boundary.
Optional limits lower validation work and evaluation expression steps; omitted
limits use the existing defaults. Null individual limit values also select those
defaults; a supplied limits record must be an object. Other stages use their
current default limits.

## Outputs

One native-run-result/1 JSON result on stdout with request digest, compiled
package byte/static identities, original source/model/runtime identities,
selected authored clause, actual stage status, diagnostics, work counters and
ordered implication events. Only completed execution has Boolean truth.
Exit 0 means completed true; 1 means completed false or refused; 3 means
incomplete; 2 means command/request syntax, identifier, I/O or output failure.
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

## Dependencies

- [FR-025](FR-025-compile-rule-model-source.md): public model source frontend.
- [FR-024](FR-024-read-native-runtime-artifacts.md): selected runtime artifact intake.
- [FR-023](FR-023-run-native-packages.md): native execution and retained outcomes.
