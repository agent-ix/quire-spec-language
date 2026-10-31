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

Disposition pass 2 confirms the deadline tests are traceable to FR-281-AC-12, including both DeadlineGuard unit tests. A separate AC-12 evidence gap remains: the typed engine-limit precedence case is covered, but the accounting-limit precedence clause has no focused test.

## Verdict

**CONDITIONAL** — FND-003 is fixed, but a new medium evidence gap remains for accounting-limit precedence in FR-281-AC-12.

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

Rechecked fix commits `a8858796c`, `9c17d3d66`, and `255746a05` at head `255746a05f86bf63c9c80c9200111057d1b016cb`. Manual acceptance-to-test review confirms FND-001 and FND-002 are fixed. The deadline tests now name TC-763 and FR-281-AC-12, which exists on current main. The new deadline integration cases cover settled records, multiple open records, caller cancellation, and engine-limit precedence. The accounting-limit precedence clause remains without a corresponding test. The whole-repository matrix/reverse-gap/stub audit remains unavailable because `quoin gap-analysis` is absent and `quoin matrix --repo . --json` produced no output before interruption. Optional semantic review was skipped.

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
