---
id: FR-299
title: "Serve one Value-family source through every QSL library operation the CLI calls"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-029
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-029
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-275
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-276
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-278
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-279
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-285
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-286
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-287
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-289
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-292
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-099
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-294
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-280
    type: references
  - target: "ix://agent-ix/quire-specification/FR-300"
    type: depends_on
  - target: "ix://agent-ix/quire-specification/FR-301"
    type: depends_on
---
# FR-299: Serve one Value-family source through every QSL library operation the CLI calls

## Description

For one `1-draft` source with `Value`-family function claims, with or
without supplied libraries (FR-099), QSL's library operations SHALL return
the outcomes the driver's `quire check`, `compile`, `run` and `prove` render
(ADR-029 SL-1):

- `check` returns the `CheckedPackage` and its success outcome;
- `package` returns the v2 bytes `quire compile` writes;
- `execute`, on the interpreter through the execution seam (FR-294), returns
  the outcome `quire run` renders;
- `package`'s bytes are the input the driver's `prove` lowers and routes
  (FR-280).

Each call takes the FR-275 call shape and FR-276 cancellation, and each
outcome serializes to an FR-286 document with FR-285's exit code.

## Inputs

The source, optionally its supplied libraries, and each operation's request,
limits and `&Cancel`.

## Outputs

The `check`, `package` and `execute` outcomes, their FR-286 documents and
their FR-285 exit codes.

## Behavior

- When the caller runs `check` over the source, QSL shall return a success
  outcome holding the `CheckedPackage`.
- When the caller runs `package` over that `CheckedPackage`, QSL shall return
  the v2 bytes `qsl_replay::spine::compile` writes for the same source.
- When the caller runs `execute` naming a function of the source, QSL shall
  run it on the interpreter through the execution seam and return its
  outcome.
- When the source imports a library the caller supplies, QSL shall check,
  package and execute it against that library as FR-099 states.
- If the caller cancels an operation's `Cancel` handle before the operation
  ends, then the operation shall return its cancelled failure (FR-276).

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-299-AC-1 | Over a `1-draft` source declaring `p(x: Int[0, 9]): Boolean { x < 10 }` and `q(x: Int[0, 9]): Boolean { x < 5 }` with a claim on each, `check` returns success (FR-285 exit 0), `package` returns the bytes `qsl_replay::spine::compile` writes for that source, and `execute` of `p` with `x = 3` on the interpreter completes `true`, exit 0. | Test (TC-784) |
| FR-299-AC-2 | Each AC-1 outcome serializes to an FR-286 document whose `operation`, `category` and `artifacts` match the outcome, and serializing it twice gives equal bytes. | Test (TC-784) |
| FR-299-AC-3 | The AC-1 calls over a source importing `test/geometry`, with that library supplied, give the same dispositions and exit codes as AC-1. | Test (TC-784) |
| FR-299-AC-4 | A `check` over the AC-1 source with its `Cancel` handle cancelled before the call returns `StageFailure::Cancelled`, FR-285 exit 22. | Test (TC-784) |

## Dependencies

- ADR-029 SL-1: the operations the source exercises.
- [FR-275](FR-275-take-a-typed-request-limits-and-cancel-on-every-lifecycle-operation.md), [FR-276](FR-276-cancel-a-lifecycle-operation.md): call shape and cancellation.
- [FR-278](FR-278-parse-select-check-and-package-as-library-operations.md), [FR-279](FR-279-execute-a-checked-entry-as-a-library-operation.md): the library operations.
- [FR-285](FR-285-map-every-outcome-category-to-one-exit-code.md), [FR-286](FR-286-serialize-every-outcome-as-one-json-outcome-document.md): exit codes and documents.
- [FR-287](FR-287-reach-qsl-through-the-driver-cli.md): the CLI that calls these operations.
- [FR-289](FR-289-keep-results-unchanged-by-provider-installation.md), [FR-292](FR-292-key-cached-results-by-content-identity.md): provider and cache parts QSL supplies.
- [FR-099](FR-099-compile-against-supplied-libraries.md): supplied libraries.
- QSpec FR-300, FR-301: the lifecycle API and CLI.

## Overlap

The driver repository specifies and tests the same source through `quire
check`, `compile`, `run`, `prove` and `version`: each `--format json`
document equals the library outcome, `prove` routes each claim to the Kani
provider (CG) through the registry, settles `p` `proved` and `q` `refuted`
with a replayed counterexample, exits 10 and stores two cache entries, a
repeated `prove` reads both entries and runs Kani zero times, and a
cancelled `prove` writes no cache entry.

## References

- QSL-392 (ARCH-52): the bounded CLI, library, provider and cache slice.
- QSL-390 (ARCH-50).
- QSpec FR-300, FR-301, FR-306 (STD-141): the QSpec half.
