---
id: FR-563
title: "Run the parity gate over every non-interpreter backend and fail on any difference from the interpreter"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-030
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-029
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-294
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-295
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-296
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-560
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-350
    type: references
  - target: "ix://agent-ix/quire-specification/FR-306"
    type: depends_on
---
# FR-563: Run the parity gate over every non-interpreter backend and fail on any difference from the interpreter

## Description

ADR-029 EB-5 and FR-295 require each non-interpreter backend to run the
conformance corpus and generated programs against the interpreter, with any
difference failing the gate. This requirement specifies the runner that does
it and its failure policy.

### The runner

The runner is a library operation in `qsl-conformance`, a QSL crate outside
the qualified core (ADR-029 CB-3 item 3, as amended):

```rust
pub fn run_parity(
    backends: &[(ExecutionBackendChoice, &dyn SuppliedBackend, Option<&BackendCoverage>)],
    inputs: &ParityInputs,
    limits: &ParityLimits,
    cancel: &Cancel,
) -> Result<ParityReport, ParityRefusal>;
```

`ParityInputs` names the conformance corpus location (read in place, as
FR-350 reads it), the generator seed (default FR-295's
`0x51534C2D4A495431`) and count (default 10,000). The reference is always
the interpreter: a test may substitute a reference only through
`run_parity_with_reference`, which exists only under `qsl-conformance`'s
`test-support` feature, and `make parity` never enables it. `BackendCoverage`
is the seam type FR-560 defines in `qsl-eval`, and `qsl-conformance` does not
depend on `qsl-jit`. The driver runs the same operation over `quire-aot`
(Overlap). `make parity` runs `cargo xtask parity`, which links every
non-interpreter backend the QSL workspace has and calls `run_parity` with the
defaults.

For each backend and each corpus or generated unit, the runner prepares the
unit's entries on the backend and on the reference, then calls each entry
with each argument tuple on both, twice on the backend.

### What fails the gate

Each of these is a finding in the report, and any finding fails the gate:

| Finding | When |
| --- | --- |
| `difference` | The backend's result differs from the reference's in any of: `Ok` against `Err`; the `Evaluation` outcome kind; the completed value; the typed cause; the `Location`; the loss records; an `Incomplete`'s limit or charge point; a `CallFailure`'s variant or payload. A backend `Fault` where the reference returned anything else is a `difference`. |
| `nondeterministic` | The backend's two calls with the same inputs return different results. |
| `coverage-gap` | The backend supplied a coverage (FR-560) and listed unsupported an entry whose constructs all lie in it. |
| `prepare-failed` | The backend's `prepare` returned a `PrepareFailure` for a unit the reference prepared. |
| `empty-comparison` | No entry was compared for a backend: every entry was unsupported. |
| `reference-fault` | The interpreter returned `Fault` for an entry: the oracle itself has a defect. The entry is not compared on any backend. |

An entry the backend lists unsupported outside its coverage is reported with
its `UnsupportedEntry` and not compared (FR-295). Every finding carries the
FR-295 report fields: the seed, the unit's index or corpus vector id, the
unit's source bytes, the entry, the arguments and both results.

### Failure policy

- No allow-list, tolerance, expected-difference record, retry or skip
  exists. The runner has no option, environment variable or setting that
  excuses a finding.
- The runner runs every unit after a finding. The report holds every finding
  up to `ParityLimits.max_reported` (default 100) and the total count.
- `ParityRefusal` stops the run before any unit runs, with no verdict:
  `empty-scope` when no non-interpreter backend is supplied; an unreadable
  corpus; an unknown option of `cargo xtask parity`. Cancellation stops the
  run with no verdict.
- `make parity` exits 0 when the report has no finding, 1 when it has any,
  and 2 on a `ParityRefusal`, the exit convention of FR-353.
- `make ci` lists `parity` as a prerequisite in the change that adds
  `qsl-jit` to the workspace.

## Inputs

The backends with their optional coverages; the corpus location, seed and
count; `ParityLimits`; `&Cancel`.

## Outputs

`ParityReport`: per backend, the compared count, the unsupported entries and
the findings; or `ParityRefusal`.

## Behavior

- The runner shall compare every entry each backend prepares with the
  reference's result for the same entry, arguments and limits, over the
  corpus and the generated units.
- When a backend result differs from the reference's in any compared member,
  the runner shall record a `difference` naming the member.
- When a backend's two calls with equal inputs return different results, the
  runner shall record `nondeterministic`.
- If a backend lists unsupported an entry whose constructs all lie in the
  coverage it supplied, then the runner shall record `coverage-gap`.
- If a backend's `prepare` fails for a unit the reference prepared, then the
  runner shall record `prepare-failed`.
- If a backend compares no entry, then the runner shall record
  `empty-comparison`.
- If the interpreter returns `Fault` for an entry, then the runner shall
  record `reference-fault` for that entry.
- If no non-interpreter backend is supplied, then the runner shall refuse
  with `empty-scope` before running any unit.
- The runner shall offer no setting that allows, skips, retries or tolerates
  a finding.
- `make parity` shall exit 0 with no finding, 1 with any finding and 2 on a
  refusal.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-563-AC-1 | A test backend that delegates to the interpreter passes the gate over the corpus and the default generated units: the report has no finding and a compared count above zero, and the report's exit code is 0. | Test (TC-1386) |
| FR-563-AC-2 | Each of eight test backends that differs from the interpreter in exactly one member on one entry (`Ok` against `Err`, outcome kind, completed value, typed cause, `Location`, loss records, `Incomplete` charge point, `CallFailure` payload) fails with one `difference` naming that member and carrying the seed, unit index, source bytes, entry, arguments and both results; a backend returning `Fault` on one entry fails with a `difference`. | Test (TC-1386) |
| FR-563-AC-3 | A test backend whose result alternates between two values on repeated calls fails with `nondeterministic` naming the entry and arguments. | Test (TC-1386) |
| FR-563-AC-4 | A test backend whose coverage lists `let` but which lists every entry using `let` unsupported fails with one `coverage-gap` per such entry; the same backend with `let` absent from its coverage passes, listing those entries with their `UnsupportedEntry`. | Test (TC-1387) |
| FR-563-AC-5 | A test backend whose `prepare` returns `PrepareFailure::Limit` for one generated unit fails with `prepare-failed` naming that unit; a backend that lists every entry unsupported and supplies no coverage fails with `empty-comparison`. | Test (TC-1387) |
| FR-563-AC-6 | Under `test-support`, a reference that returns `Fault` for one entry gives one `reference-fault` finding naming the entry, exit 1; a run with no non-interpreter backend refuses `empty-scope` before any unit runs, exit 2; a corpus location that does not exist refuses, exit 2. | Test (TC-1387) |
| FR-563-AC-7 | A backend with 150 `difference` findings yields a report holding 100 findings and the total 150 at the default `max_reported`, and the run reaches every unit. | Test (TC-1386) |
| FR-563-AC-8 | `run_parity`'s inputs and `cargo xtask parity`'s options hold no member that allows, skips, retries or tolerates a finding, and none selects a reference; `run_parity_with_reference` does not exist without the `test-support` feature (a `compile_fail` doctest with a compiling control under the feature); `cargo xtask parity --no-such-option` refuses, exit 2, naming the option. | Test (TC-1387) and Inspection |

## Dependencies

- ADR-029 EB-3, EB-5: the reference and the parity evidence.
- [FR-294](FR-294-run-every-execution-backend-behind-one-seam.md), [FR-296](FR-296-select-the-execution-backend-in-the-request.md): the seam and the supplied backends.
- [FR-295](FR-295-return-interpreter-outcomes-from-jit-and-aot.md): the compared members, the corpus and the generator.
- [FR-560](FR-560-translate-the-jit-coverage-set-and-report-every-other-construct-untranslatable.md): the JIT coverage.
- [FR-350](FR-350-run-the-conformance-corpus-against-qsl.md): reading the corpus in place.
- QSpec FR-306-AC-1: parity over the conformance corpus and generated
  programs.

## Overlap

The driver repository runs `run_parity` over `quire-aot`, depending on
`qsl-conformance`, and gates the AOT backend on it.

## References

- QSL-393 (V1-A06a).
