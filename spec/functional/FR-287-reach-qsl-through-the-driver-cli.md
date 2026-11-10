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

The request-file owner is the non-core crate `qsl-command`, whose public
modules are `qsl_command::command` and `qsl_command::formal_source`.
These replace the existing root modules, with no compatibility re-export.
The driver's `quire-cli` package calls this owner directly for bounded
request preparation and compilation, without depending on the QSL root
package. No core crate depends on `qsl-command`, and its filesystem intake
stays outside the qualified core
([FR-284](FR-284-keep-the-qualified-core-separable-by-crate.md)).

`RunError::outcome_document(operation: qsl_replay::Operation)` returns a
`qsl_replay::OutcomeDocument` through the command owner's pure projection
of its existing typed failure. It SHALL retain the original category,
catalog code, typed cause and available location. It SHALL retain the
spine's reached stage for a stage refusal and `last_stage` null for a
pre-stage intake failure, cancellation or fault as FR-286 specifies.
Source identity causes retain FR-001's typed `label` and optional region.
The projection uses the core outcome types and their one serializer;
neither it nor the driver reads back native JSON or branches on a human
diagnostic message.

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
| FR-287-AC-4 | `--request-bytes` and `--dependent-bytes` set `command`'s `request_bytes` and `dependent_bytes` settings for `quire run` (defaults 1 MiB and 8 MiB per FR-026), have no ceiling, and a run limit refusal names the limit, its configured value and the option that raises it. A run request one byte over each setting refuses naming it, and the same request with the option set to its size runs. `quire compile` has fixed 1 MiB request-file and 8 MiB dependent-file ceilings, accepts no limit options, and refuses an over-limit input naming its setting and maximum without an option hint. | Test (TC-1262) |
| FR-287-AC-5 | An invalid invocation of `parse`, `format`, `run` or `compile` (an unknown option, a missing or extra operand, a malformed option value) exits 20 with no panic, and the same verb run over a valid invocation after it runs normally. | Test (TC-1262) |
| FR-287-AC-6 | A command word or `parse`/`format` label that is not valid UTF-8 (Unix bytes, Windows unpaired surrogates) refuses before the file operand is opened, exit 20, with no panic; a valid source at a file path that is not valid UTF-8 parses. | Test (TC-1262) |
| FR-287-AC-7 | When writing the machine outcome document or formatted text source to stdout fails (stdout bound to `/dev/full`), `quire parse`, `format`, `run` and `compile` exit 30, FR-301's code for tool failure; a partial stdout prefix may remain and is not a complete artifact. | Test (TC-1262) |
| FR-287-AC-8 | When the reader of stdout closes the pipe before an attempted machine-outcome or formatted-source write, `quire parse`, `format`, `run` and `compile` end quietly, write nothing to stderr and exit with the outcome's FR-285 code, not 30. A text rendering that is unsupported and makes no primary stdout write is outside this pipe-write criterion; it retains the driver's declared unsupported-rendering behavior. | Test (TC-1262) |
| FR-287-AC-9 | In machine mode, `quire compile` and `quire run` write exactly their producer-owned FR-286 `quire-outcome/1` document to stdout, for success and refusal alike, with no CLI-added or removed member; no native package bytes, native-run-result document or refusal envelope replace it on a process stream. A compile's document is built only after package construction finishes or refuses. Resolving the same valid request as a relative and an absolute operand from two working directories gives equal outcome bytes and exit code. | Test (TC-1262) |
| FR-287-AC-10 | An external Rust consumer with no dependency on `quire-spec-language` calls `qsl_command::command::compile_outcome` over AC-2's compile request and `prepare_run` over its bounded run request. The compile bytes equal the existing `compile` entry's bytes and the document equals FR-286's projection of the actual package result. Executing the prepared run through the existing spine yields the same result and incomplete category as the existing command entry; preparation retains authored labels, package name, packages, libraries, call arguments and accounting. The QSL root defines and re-exports neither `command` nor `formal_source`, and no core crate depends on `qsl-command`. | Test |
| FR-287-AC-11 | Missing request files and malformed request JSON produce the command owner's outcome document with the original catalog code and category and `last_stage` null; source, check and emit refusals retain their owning typed cause, category, reached stage and available region. A source identity refusal retains its exact typed label with no fabricated locus. The command's existing native library result and refusal behavior remain unchanged. | Test |

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
request-file operand, usage and arity errors before any file is read (exit 20),
and writing the producer-owned FR-286 outcome document to stdout unchanged
in machine mode. FR-286 explicitly replaces the earlier native result
stream convention when the driver's verbs land; native library artifacts
remain in-process command products. `run` accepts the `request_bytes` and
`dependent_bytes` options; `compile` uses fixed default ceilings and accepts no
limit options. For `parse` and `format` it
owns the command and label operands, their OS encoding (a non-UTF-8 command
or label refuses, exit 20), the operand count, writing the outcome document
or formatted source to stdout, and a write failure (exit 30).
FR-287-AC-3 to FR-287-AC-9 state these criteria and TC-1262 tests them.

## References

- QSL-390 (ARCH-50): owner ruling 1 (the user CLI is the driver),
  recorded on the ticket.
- QSpec FR-301 (STD-141): the QSpec half.
