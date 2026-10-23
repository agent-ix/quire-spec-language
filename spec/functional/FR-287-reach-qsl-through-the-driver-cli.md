---
id: FR-287
title: "Reach QSL through the driver's thin CLI frontend"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-029
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-029
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-275
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-285
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-286
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-027
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: references
  - target: "ix://agent-ix/quire-specification/FR-301"
    type: depends_on
---
# FR-287: Reach QSL through the driver's thin CLI frontend

## Description

The user binary is `quire`, built from the driver repository (ADR-029 CB-1).
QSL builds no user binary. Its only executable targets are the developer
tools, the packages that run against source trees and measurements rather
than serve a user and that no QSL library's build depends on: `xtask`, the
packages whose manifest directory is under `tools/`, `qsl-string-edge` (the
standalone string-edge scanner FR-064 requires as a binary) and `qsl-bench`
(whose `qsl-bench-probe` binary MP-006's probe and the Makefile's
`BENCH_PROBE` run). The request
handling FR-027 and FR-100 specify is a QSL library operation, `command`,
which the driver reaches.

The CLI is a thin frontend: it parses arguments, reads the files the
arguments name or stdin, builds a typed request, calls one library
operation, renders the outcome and maps it to an exit code through FR-285.
It makes no semantic decision and reads no diagnostic text (ADR-011 FB-09).
QSpec FR-301 owns the CLI's lifecycle, argument refusal and version output.
QSL's part is the library surface the CLI calls: the operations (FR-275),
`command`, the outcome document (FR-286) and the exit function (FR-285).

The verbs are `parse`, `format`, `check`, `compile`, `run`, `analyze`,
`monitor`, `prove`, `replay`, `generate`, `inspect` and `version` (ADR-029
CB-5). Every verb takes `--format json|text`, and `inspect` also takes
`dot`.

## Inputs

For `command`, a typed request decoded from a request document (FR-027,
FR-100).

## Outputs

`command`'s typed outcome, from which the CLI renders its output (FR-286,
FR-298) and takes FR-285's exit code.

## Behavior

- The QSL workspace shall build no executable target outside the
  developer-tool packages: `xtask`, the packages whose manifest directory is
  under `tools/`, `qsl-string-edge` and `qsl-bench`.
- The QSL root package shall declare no `[[bin]]` target and no
  `default-run`.
- No QSL workspace member other than a developer-tool package shall take a
  normal or build dependency on a developer-tool package; a dev dependency,
  which reaches only that member's tests, is allowed.
- QSL `command` shall be a library operation that takes a typed request and
  returns a typed outcome whose exit code is FR-285's code of its category.
- QSL `command` shall write nothing to the process streams.
- The `quire` frontend's argument, encoding, stream and exit-code behavior for
  the `parse`, `format`, `run` and `compile` verbs shall be as FR-287-AC-3 to
  FR-287-AC-9 state; the driver implements them and TC-1262 specifies the
  tests that bind them.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-287-AC-1 | `cargo metadata --no-deps --format-version 1` in the QSL repository lists a `bin` target only in the developer-tool packages (`xtask`, the packages whose manifest directory is under `tools/`, `qsl-string-edge` and `qsl-bench`); the root package `quire-spec-language` has no `bin` target and no `default_run`; and no workspace member outside those packages lists one of them as a normal or build dependency. | Test (TC-771) |
| FR-287-AC-2 | `command` called in process with FR-027's compile request over `tests/fixtures/spine-compile.native` returns a typed success outcome whose FR-285 exit code is 0, and with FR-100-AC-6's `call.accounting` `{"work_units": 0}` run request a typed incomplete outcome whose exit code is 22; neither call writes a byte to stdout or stderr. | Test (TC-771) |
| FR-287-AC-3 | `quire run` and `quire compile` with a missing request-file operand, and with an extra operand, exit 20 and write the selected verb's usage to stderr before any file is read: a request path that does not exist is never opened, and no stdout byte is written. `quire parse` and `quire format` with the two labels and no file, and with one extra operand, exit 20 the same way. A `parse` or `format` call with a blank label and a missing file exits 20 with `invalid_source_identity`, cause `blank-label`, the blank label's name in `label`, not a file error. | Test (TC-1262) |
| FR-287-AC-4 | `--request-bytes` and `--dependent-bytes` set `command`'s `request_bytes` and `dependent_bytes` settings (defaults 1 MiB and 8 MiB per FR-026), have no ceiling, and a limit refusal printed by `quire run` or `quire compile` names the limit, its configured value and the option that raises it. A request one byte over each limit refuses naming it, and the same request with the option set to its size runs. | Test (TC-1262) |
| FR-287-AC-5 | An invalid invocation of `parse`, `format`, `run` or `compile` (an unknown option, a missing or extra operand, a malformed option value) exits 20 with no panic, and the same verb run over a valid invocation after it runs normally. | Test (TC-1262) |
| FR-287-AC-6 | A command word or `parse`/`format` label that is not valid UTF-8 (Unix bytes, Windows unpaired surrogates) refuses before the file operand is opened, exit 20, with no panic; a valid source at a file path that is not valid UTF-8 parses. | Test (TC-1262) |
| FR-287-AC-7 | When writing the outcome document, the formatted source, the package bytes or the refusal envelope to the process stream fails (stdout bound to `/dev/full`), `quire parse`, `format`, `run` and `compile` exit 30, FR-301's code for tool failure; a partial stdout prefix may remain and is not a complete artifact. | Test (TC-1262) |
| FR-287-AC-8 | When the reader of stdout closes the pipe before the output is written, `quire parse`, `format`, `run` and `compile` end quietly, write nothing to stderr and exit with the outcome's FR-285 code, not 30. | Test (TC-1262) |
| FR-287-AC-9 | `quire compile` writes exactly `command`'s package bytes to stdout, with no wrapper or extra newline, only after package construction finishes, and a failed compile writes no package byte; `quire run` writes exactly `command`'s native-run-result document to stdout; a refusal writes `command`'s refusal envelope to stderr and no result bytes to stdout; one request path resolved from two working directories, as a relative and as an absolute operand, gives equal output bytes and exit code. | Test (TC-1262) |

## Dependencies

- ADR-029 CB-1, CB-5: the binary and its verbs.
- ADR-011 FB-09: no diagnostic text is read back.
- [FR-275](FR-275-take-a-typed-request-limits-and-cancel-on-every-lifecycle-operation.md): the library operations.
- [FR-285](FR-285-map-every-outcome-category-to-one-exit-code.md), [FR-286](FR-286-serialize-every-outcome-as-one-json-outcome-document.md): exit code and outcome document.
- [FR-027](FR-027-export-compiled-native-package.md), [FR-100](FR-100-run-a-named-function-through-the-spine.md): QSL `command`.
- QSpec FR-301: the CLI lifecycle, argument refusal, and version output, as the
  QSpec half in References aligns it.

## Overlap

The driver repository specifies and tests the `quire` binary against QSpec
FR-301: its verbs and arguments, refusal of an unknown verb, option,
profile or provider before any stage (exit 20), `version`, reading no
configuration file its arguments do not name, and stream-write failures
(exit 30, a closed pipe ending quietly with the outcome's code). For the
`run` and `compile` verbs over FR-026 and FR-027 requests it also owns the
request-file operand, the option spelling that sets the `request_bytes` and
`dependent_bytes` settings, usage and arity errors before any file is read
(exit 20), writing `command`'s package bytes or result document to stdout
unchanged and its refusal envelope to stderr. For `parse` and `format` it
owns the command and label operands, their OS encoding (a non-UTF-8 command
or label refuses, exit 20), the operand count, writing the outcome document
or formatted source to stdout, and a write failure (exit 30).
FR-287-AC-3 to FR-287-AC-9 state these criteria and TC-1262 tests them.

## References

- QSL-390 (ARCH-50): owner ruling 1 (the user CLI is the driver),
  recorded on the ticket.
- QSpec FR-301 (STD-141): the QSpec half.
