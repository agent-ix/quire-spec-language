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

Independently reviewed the current PR diff, its prior review artifact, the FR-280/FR-281 contracts and the changed Rust tests. The provider path still accepts `AnalyzeLimits.deadline_ms` without enforcing it, so a finite request deadline can elapse while the analysis continues.

## Verdict

**CONDITIONAL** — one medium finding remains. Formatting, Clippy and the two focused provider tests pass; `cargo deny check` fails on an unchanged AGPL dependency allowance, and the repository-wide matrix did not complete.

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

## Coverage

Plan completion: not assessed.

Scope examined: FR-280-AC-1 through AC-7, FR-281's `AnalyzeLimits` input and cancellation/limit behavior, the complete PR diff, the new provider path, settlement conversion, touched test modules, and repository Rust guidance. Optional semantic review was skipped. `quoin gap-analysis --help` failed with `command gap-analysis not found` under quoin 0.29.1. `quoin matrix --repo . --json` produced no output and was interrupted after more than 90 seconds, so repository-wide matrix, reverse-gap and stub conclusions are unavailable; the targeted manual acceptance check is recorded separately.

Focused evidence at the reviewed head:

- `cargo fmt --all -- --check` — passed.
- `cargo clippy -p qsl-analyze -p qsl-replay --all-targets -- -D warnings` — passed.
- `cargo test -p qsl-analyze --test it analyze_and_the_provider_entry_settle_one_record_for_one_en4_item` — passed.
- `cargo test -p qsl-analyze --test it provider_entry_settles_an_incomplete_binding_per_item` — passed.
- `cargo deny check` — failed on `AGPL-3.0-or-later` not being explicitly allowed for unchanged `agent-ix-semantic-ir` / `agent-ix-extraction-frontend` dependencies; this PR changes no manifests or lockfile.
- No CI workflow files changed.

The documented `codex exec review --base origin/main` method could not initialize its in-process app-server client (`Operation not permitted`); this review was completed manually against the code-review and Rust-review checklists.
