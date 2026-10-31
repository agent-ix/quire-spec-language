---
id: SR-1932004
title: "Gap analysis — QSL-965 FR-280 EN-4 provider path"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; FR-280-AC-5, FR-280-AC-6, FR-281 AnalyzeLimits; qsl-analyze/src/analyze.rs, qsl-analyze/src/provider.rs, qsl-analyze/tests/it/analyze.rs"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-280
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-281
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-761
    type: reviews
review_set: subset
---

## Summary

Disposition pass 3 confirms the accounting-limit precedence case is now covered: a finite-deadline analyze request settles with the typed WorkUnits limit result and does not cancel. The prior traceability finding is also fixed; no new spec-coverage gaps were found on the current head.

## Verdict

**PASS** — FND-003 and FND-004 are fixed, and no new spec-coverage gaps were found.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | A finite AnalyzeLimits.deadline_ms has no enforcement and no test; the field is never read, so the operation may outlive the caller's deadline. | qsl-analyze/src/analyze.rs:211 | implementation-bug-despite-evidence |
| FND-002 | medium | FR-280-AC-6 has only a one-item incomplete-snapshot test; no test covers the refused undeclared-population binding, reversed result order, or no-engine-run assertions. | qsl-analyze/tests/it/analyze.rs:564 | correct-requirement-no-evidence |

## Dispositions

| Finding | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | `analyze` now starts a deadline guard from `AnalyzeLimits.deadline_ms`; the guard cancels with the deadline cause, and integration coverage verifies the zero-deadline incomplete outcome. |
| FND-002 | fixed | `provider_entry_refuses_undeclared_population_before_running_items_in_input_order` checks refused admission, two records in input order, backend-free declined results, the typed diagnostic, and zero engine charges. |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-003 | medium | Deadline tests are not traceable to an acceptance criterion: the integration test uses `#[trace("FR-281")]`, a root requirement id, the guard unit tests have no trace tags, and FR-281 has no deadline-specific AC. | qsl-analyze/tests/it/analyze.rs:681; qsl-analyze/src/analyze.rs:1007 | correct-requirement-no-evidence |

## Coverage

Plan completion: not assessed.

Rechecked fix commits `a8858796c`, `9c17d3d66`, and `255746a05` at head `255746a05f86bf63c9c80c9200111057d1b016cb`. Manual acceptance-to-test review confirms FND-001 and FND-002 are fixed. The deadline tests now name TC-763 and FR-281-AC-12, which exists on current main. The deadline integration cases cover settled records, multiple open records, caller cancellation, engine-limit precedence, and accounting-limit precedence. The accounting-limit test asserts a typed WorkUnits result and that cancellation was not triggered. The whole-repository matrix/reverse-gap/stub audit remains unavailable because `quoin gap-analysis` is absent and `quoin matrix --repo . --json` produced no output before interruption. Optional semantic review was skipped.

Disposition checks on the updated head:

- `cargo fmt --all -- --check` — passed.
- `cargo clippy -p qsl-analyze -p qsl-replay --all-targets -- -D warnings` — passed.
- Deadline guard unit tests — 2 passed.
- Undeclared-population admission/order test — passed.
- Zero-millisecond deadline integration test — passed.


## New findings (disposition pass 2)

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-004 | medium | FR-281-AC-12 requires preserving an item's typed result when an accounting limit stops it before deadline expiry, but the new precedence test covers only an engine max-draws limit; no analyze test exercises an accounting limit with a finite deadline. | qsl-analyze/tests/it/analyze.rs:876 | correct-requirement-no-evidence |

Disposition pass 2 rechecked fix commit `c62868b76c0bc86fbdb60224ad3f2d3bc5f7265d` against current main `5ec1d91c8a31ed3e3a123a9df00f5a63e0baa060`.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-003 | fixed | The zero-deadline and finite-deadline integration tests and both DeadlineGuard unit tests are tagged with TC-763 and FR-281-AC-12, now present on main. |

Focused evidence: `cargo fmt --all -- --check` passed; `cargo test -p qsl-analyze --lib deadline_guard` passed (2 tests); finite-deadline, caller-cancellation, and engine-limit precedence integration tests each passed (1 test each).


Disposition pass 3 rechecked the full PR head `dc5663b4d23c624c216df3b4d505063988b60952` against current main `5ec1d91c8a31ed3e3a123a9df00f5a63e0baa060`.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-004 | fixed | The finite-deadline integration test sets accounting work_units to 1, asserts the typed `EvaluationMeter{WorkUnits}` limit result, and asserts `cancel.cause()` remains `None`. |

Round-3 focused evidence: `cargo test -p qsl-analyze --test it accounting_limit_before_deadline_keeps_its_typed_result` passed (1 test); `cargo fmt --all -- --check` passed; `git diff --check origin/main...HEAD` passed. No new coverage gaps were identified.
