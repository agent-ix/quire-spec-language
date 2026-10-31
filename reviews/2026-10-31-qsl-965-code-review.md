---
id: SR-1932003
title: "Code review — QSL-965 FR-280 EN-4 provider path"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-analyze/src/analyze.rs, qsl-analyze/src/provider.rs, qsl-analyze/src/lib.rs, qsl-replay/src/proof_result.rs, qsl-analyze/tests/it/analyze.rs, qsl-analyze/tests/it/statistical_witness_replay.rs, qsl-analyze/tests/it/{dense_route,derived_request,hyper_settle,operation_contract,prob_world,protocol_roles,race_freedom,timed_refinement,value_validity}.rs"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-280
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-281
    type: reviews
review_set: subset
---

## Summary

Disposition pass 1 confirms that all four original findings are fixed on the reviewed head. The deadline tests introduce a new low severity issue: they use `thread::sleep` as a synchronization oracle, contrary to the Rust testing guidance.

## Verdict

**CONDITIONAL** — all original findings are fixed, but one new test-quality finding remains open.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Binding admission failure was followed by rerunning item work, allowing a backend-bearing record instead of settling directly from the shared admission failure. | qsl-analyze/src/provider.rs:69 |
| FND-002 | medium | FR-280-AC-5's test compared witnesses and checked their count but did not replay the provider-returned witnesses as TC-761 requires. | qsl-analyze/tests/it/analyze.rs:548 |
| FND-003 | medium | Analyze constructed an item from an independently supplied statistical subject, so the request binding did not determine the subject the engine ran. | qsl-analyze/src/analyze.rs:643 |
| FND-004 | medium | AnalyzeLimits.deadline_ms is accepted but never read or enforced, so an analysis can continue past its finite caller deadline. | qsl-analyze/src/analyze.rs:211 |

## Dispositions

| Finding | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | The provider maps `AdmissionFailure` directly to backend-free terminal records and preserves the typed catalog code. |
| FND-002 | fixed | The Service test now replays every retained witness and asserts `Reproduced`. |
| FND-003 | fixed | `AnalyzeRequest` carries routed EN-4 claim data; `analyze` constructs the statistical request from its one admitted `SubjectBinding`. |
| FND-004 | fixed | `analyze` starts a `DeadlineGuard` from `limits.deadline_ms`; the guard cancels with `CancelCause::Deadline`, and a focused integration test verifies the deadline settlement. |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | low | The deadline-guard tests use `std::thread::sleep` as a synchronization oracle, making the cancellation assertions timing-sensitive and violating the Rust testing guidance. | qsl-analyze/src/analyze.rs:1013,1028 |

## Coverage

Plan completion: not assessed.

Rechecked findings against fix commits `a8858796c`, `9c17d3d66`, and `255746a05` at head `255746a05f86bf63c9c80c9200111057d1b016cb`. FND-001 through FND-004 are fixed. FND-005 is new and remains open. `codex exec review` remains unavailable because its in-process app-server client fails to initialize (`Operation not permitted`); static review used the installed code-review, rust-review, and rust-style checklists.

Disposition checks on the updated head:

- `cargo fmt --all -- --check` — passed.
- `cargo clippy -p qsl-analyze -p qsl-replay --all-targets -- -D warnings` — passed.
- `cargo test -p qsl-analyze --lib deadline_guard` — 2 passed.
- `cargo test -p qsl-analyze --test it provider_entry_refuses_undeclared_population_before_running_items_in_input_order` — passed.
- `cargo test -p qsl-analyze --test it provider_entry_settles_a_zero_millisecond_deadline_as_incomplete` — passed.

Earlier review limitations still apply: quoin 0.29.1 does not provide `gap-analysis`; the repository-wide matrix did not complete; `cargo deny check` fails on unchanged AGPL dependency allowances.
