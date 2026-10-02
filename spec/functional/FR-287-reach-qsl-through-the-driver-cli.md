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
QSL builds no user binary. The request handling FR-027 and FR-100 specify is
a QSL library operation, `command`, which the driver reaches.

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

- The QSL workspace shall build no executable target.
- QSL `command` shall be a library operation that takes a typed request and
  returns a typed outcome whose exit code is FR-285's code of its category.
- QSL `command` shall write nothing to the process streams.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-287-AC-1 | `cargo build --workspace` in the QSL repository produces no executable file in its target directory. | Test (TC-771) |
| FR-287-AC-2 | `command` called in process with FR-027's compile request over `tests/fixtures/spine-compile.native` returns a typed success outcome whose FR-285 exit code is 0, and with FR-100-AC-6's `work_units` 0 run request a typed incomplete outcome whose exit code is 22; neither call writes a byte to stdout or stderr. | Test (TC-771) |

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
(exit 30, a closed pipe ending quietly with the outcome's code).

## References

- QSL-390 (ARCH-50): owner ruling 1 (the user CLI is the driver),
  recorded on the ticket.
- QSpec FR-301 (STD-141): the QSpec half.
