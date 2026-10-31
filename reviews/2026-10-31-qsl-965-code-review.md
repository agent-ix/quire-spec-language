---
id: SR-1932003
title: "Code review — qsl-analyze FR-280 EN-4 provider entry"
type: SpecReview
analysis: code-review
scope: "quire-spec-language; qsl-analyze/src/, qsl-analyze/tests/it/, qsl-replay/src/"
review_set: subset
---

## Summary

Reviewed the FR-280 EN-4 provider entry, shared binding admission, settlement conversion, and the Service witness path against FR-280/FR-281 and the changed tests. Three review findings were fixed in this branch; the focused tests, formatting, check and Clippy pass.

## Verdict

**CONDITIONAL** — the changed paths are clean after the fix round, but the repository-wide gap audit could not be completed: `quoin gap-analysis` is not an installed CLI command, and `quire matrix --scope . --format json` emitted startup diagnostics then produced no result during a 90-second run.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Binding admission failure was followed by rerunning item work, allowing a backend-bearing record instead of settling directly from the shared admission failure. | qsl-analyze/src/provider.rs:69 |
| FND-002 | medium | FR-280-AC-5's test compared witnesses and checked their count but did not replay the provider-returned witnesses as TC-761 requires. | qsl-analyze/tests/it/analyze.rs:548 |
| FND-003 | medium | Analyze constructed an item from an independently supplied statistical subject, so the request binding did not determine the subject the engine ran. | qsl-analyze/src/analyze.rs:643 |

## Dispositions

| Finding | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | The provider maps `AdmissionFailure` directly to backend-free terminal records and preserves the typed catalog code. |
| FND-002 | fixed | The Service test now replays every retained witness and asserts `Reproduced`. |
| FND-003 | fixed | `AnalyzeRequest` now carries routed EN-4 claim data; `analyze` constructs the statistical request from its one admitted `SubjectBinding`. |

## Coverage

Plan completion: not assessed.

The requested review lanes were Rust review and gap analysis. `quoin --version` reports 0.29.1, whose command list does not include `gap-analysis`; the direct matrix fallback was attempted with `quire 0.37.1` (`engine 0.50.3`) and did not finish within 90 seconds. Repository-wide matrix, reverse-gap and stub conclusions are therefore unavailable. Optional semantic review was skipped.

Focused evidence after the fix round:

- `cargo check -p qsl-analyze --tests` — passed.
- `cargo test -p qsl-analyze --test it analyze_and_the_provider_entry_settle_one_record_for_one_en4_item` — passed; includes replay of all three returned witnesses.
- `cargo test -p qsl-analyze --test it provider_entry_settles_an_incomplete_binding_per_item` — passed.
- `cargo clippy -p qsl-analyze -p qsl-replay --all-targets -- -D warnings` — passed.
- `cargo fmt --all -- --check` — passed.
- No CI workflow files changed.
