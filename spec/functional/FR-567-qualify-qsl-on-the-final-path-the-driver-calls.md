---
id: FR-567
title: "Qualify QSL on the final path the driver calls"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-029
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/US-035
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-350
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-353
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-352
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-563
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-565
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-287
    type: references
  - target: "ix://agent-ix/quire-specification/FR-306"
    type: references
---
# FR-567: Qualify QSL on the final path the driver calls

## Description

The final path is the path a user's run takes: a `quire` verb in the driver
calls a QSL library operation (FR-287), which runs on the interpreter or on
a non-interpreter backend the build links (FR-296). The `qualify` gate's
verdict SHALL cover that path, not only the entry points the corpus runner
calls: the gate (FR-353) runs three parts in every run and reports
`qualified` only when all three pass.

| Part | Passes when | Name of a failure |
| --- | --- | --- |
| `corpus` | FR-353's verdict over the FR-350 run is `qualified` | each failing capability, as FR-353 names it |
| `lifecycle-matrix` | every `Case` of FR-565's `lifecycle_matrix()` gives a document or item whose category and exit code equal the case's expected ones | `lifecycle-matrix/<verb>/<kind>` for each failing case |
| `parity` | FR-563's runner over every non-interpreter backend the QSL workspace links reports no finding | `parity/<backend>` with the backend's findings |

The `lifecycle-matrix` part passes when every `Case` passes.

When the workspace links no non-interpreter backend, the `parity` part is
reported `no-backend` and passes: the final path is then the interpreter
alone, which the other two parts cover. Every parity finding, a
`reference-fault` included, fails the part. A `ParityRefusal` other than
`empty-scope` (an unreadable corpus) is a preflight refusal of `qualify`
(exit 2, before any vector executes), as FR-353 already refuses an
unreadable corpus.

The FR-352 report gains one `parts` member naming each part and its result;
its capability coverage is unchanged. The QSL version the report names is the
build's own: `env!("CARGO_PKG_VERSION")` of the `quire-spec-language`
package the gate ran against, never a version written into the gate's
source.

The gate's library entry is `xtask::qualify::run(inputs: &QualifyInputs)`,
which `cargo run --package xtask -- qualify` calls with the defaults:
FR-565's `lifecycle_matrix()` and the non-interpreter backends the workspace
links. Under xtask's `test-support` feature, `QualifyInputs` also accepts a
replacement matrix cell set and replacement backends, so a test can make
each part fail; the command line offers neither. The exit contract stays
FR-353's: 0 when `qualified`, 1 when any part fails, 2 on a refusal before a
verdict. The `complete-v1: qualified` label keeps FR-353's meaning and also
requires the `lifecycle-matrix` and `parity` parts to pass.

A non-interpreter backend's results are on the final path only through the
`parity` part: a backend the build links and the parity part has not passed
leaves the run `not-qualified`.

## Inputs

FR-353's inputs; the FR-565 cases; the non-interpreter backends the
workspace links.

## Outputs

The FR-352 report with the three parts and the verdict; the exit code.

## Behavior

- The `qualify` gate shall run the `corpus`, `lifecycle-matrix` and `parity`
  parts in every run.
- When all three parts pass, the gate shall report `qualified` and exit 0.
- If a `lifecycle-matrix` case's category or exit code differs from the
  case's expected one, then the gate shall report `not-qualified`, naming
  `lifecycle-matrix/<verb>/<kind>`, and exit 1.
- If the parity runner reports a finding for a linked backend, then the
  `qualify` gate shall report `not-qualified`, naming `parity/<backend>`, and exit 1.
- When the workspace links no non-interpreter backend, the gate shall report
  the `parity` part `no-backend` and treat it as passed.
- The FR-352 report shall name each part and its result.
- The FR-352 report shall name as the QSL version the
  `quire-spec-language` package's `CARGO_PKG_VERSION` from the build that
  ran.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-567-AC-1 | Over FR-353-AC-1's passing fixture corpus, with every matrix case passing and a linked test backend that delegates to the interpreter, `qualify` reports `complete-v1: qualified`, names the three parts as passed, and exits 0. | Test (TC-1396) |
| FR-567-AC-2 | Through `xtask::qualify::run` under `test-support`, the AC-1 run with the matrix case set replaced by one whose `check`/`refusal` case expects exit 21 reports `not-qualified`, naming `lifecycle-matrix/check/refusal`, and exits 1, although the `corpus` part passes. | Test (TC-1396) |
| FR-567-AC-3 | Through `xtask::qualify::run` under `test-support`, the AC-1 run with FR-295-AC-1's rounding backend supplied in place of the delegating one reports `not-qualified`, naming `parity/jit` and the finding, and exits 1. | Test (TC-1396) |
| FR-567-AC-4 | The AC-1 run with no non-interpreter backend linked reports the `parity` part `no-backend` and `complete-v1: qualified`, exit 0; the AC-1 run whose parity reference (substituted under `test-support`) returns `Fault` reports `not-qualified`, naming `parity/jit` with a `reference-fault` finding, exit 1. | Test (TC-1396) |
| FR-567-AC-5 | The AC-1 report's QSL version is the quire-spec-language crate version (the root package's version, not the `xtask` crate's) and equals `env!("CARGO_PKG_VERSION")` evaluated in the `quire-spec-language` crate of the same build; built from a scratch copy whose `quire-spec-language` package version is changed to `9.9.9`, the report names `9.9.9`. | Test (TC-1396) |
| FR-567-AC-6 | Through `xtask::qualify::run` under `test-support`, the AC-1 run with the matrix case set replaced by one whose `analyze`/`violation` case expects exit 0 reports `not-qualified`, naming `lifecycle-matrix/analyze/violation`, and exits 1, although the `corpus` part passes. | Test (TC-1396) |

## Dependencies

- [FR-350](FR-350-run-the-conformance-corpus-against-qsl.md), [FR-352](FR-352-report-capability-coverage.md), [FR-353](FR-353-settle-the-complete-v1-qualification-verdict.md): the corpus run, the report and the verdict, whose "if and only if" names the two parts this requirement adds.
- [FR-563](FR-563-run-the-parity-gate-and-fail-on-any-difference-from-the-interpreter.md): the parity runner.
- [FR-565](FR-565-publish-the-lifecycle-matrix-cases-the-driver-cli-half-runs.md): the matrix cases.
- [FR-287](FR-287-reach-qsl-through-the-driver-cli.md): the driver CLI that calls these operations.
- QSpec FR-306: backend parity.

## Overlap

The driver repository qualifies its own half of the path: its CLI half of
the matrix (FR-565) and the parity of `quire-aot` (FR-563).

## References

- QSL-390 (ARCH-50).
