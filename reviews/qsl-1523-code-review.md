---
id: SR-2026110216378
title: "Code review — QSL-1523 model admission limits"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-foundation/, qsl-semantics/, qsl-replay/, qsl-eval/tests/"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-spec-language/FR-506", type: references }
  - { target: "ix://agent-ix/quire-spec-language/FR-255", type: references }
  - { target: "ix://agent-ix/quire-spec-language/FR-259", type: references }
---

## Summary

Reviewed PR #2181 at `1addfc6379b776a86ff7cc5b2dfd26c89cc213d3` against FR-506, FR-255 and FR-259, including the Rust implementation and tests. The identity-input-bytes subpart of FR-506-AC-5 is neither implemented nor testable under FR-259's retired structural preimage contract, so the change is blocked pending contract reconciliation.

## Verdict

**FAIL** — FR-506-AC-5 contains an unfulfilled identity-preimage requirement that conflicts with FR-259; no identity-input-bytes test or model admission path exists.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-506-AC-5 requires an identity preimage byte limit with no corresponding model intake behavior or test, while FR-259 retires structural node/package preimages | qsl-semantics/src/model/intake.rs:419 |
| FND-002 | low | New lifecycle test assertions are not rustfmt-formatted | qsl-replay/src/spine/lifecycle/tests.rs:1576 |

## Review notes

- AC-1 record counting and publication-on-refusal logic are directly covered; the test uses an 11-record package rather than TC-1301's 12-record L0 fixture.
- AC-2 through AC-4 and AC-6 through AC-8 have relevant boundary tests. The AC-2 test covers field, parameter, result, and an unused type's field, but does not construct the specified nested option-of-sequence maximum.
- AC-5's `intake.input_bytes` subpart has exact, one-short, and zero coverage. The `s3.nodes` test's bisection helper verifies the exact threshold and zero. No model admission test covers `identity.input_bytes` for any model identity preimage. FR-259-AC-2/4 explicitly retire node/package preimages, so this is a contract conflict and an open gap, not a pass.
- FR-255's `model.sequence_maximum` row was checked against the limits type and entry-point mapping.
- Rust review lane: public limits/settings mapping, typed refusal mapping, input-boundary handling, record counting, and test assertions were inspected. Focused `qsl-semantics` model-admission tests passed (5/5), the focused replay `s3.nodes` threshold test passed (1/1), and `cargo clippy -p qsl-foundation --all-targets --all-features -- -D warnings` passed. `cargo fmt --all -- --check` reports formatting diffs in changed lifecycle test lines 1576, 1584, and 1590; it also reports an unrelated diff in `qsl-semantics/src/check/mod.rs:1515`. `cargo deny check` reported advisories, bans, licenses and sources OK (with existing source-not-allowed warnings). Broader focused Clippy failed on untouched code: `qsl-cst/src/parser.rs:241` has `double_ended_iterator_last`, and unchanged `qsl-semantics/src/check/family.rs`, `check/hyper_clause.rs` and `check/temporal_clause*.rs` have non-exhaustive `StageFailure` matches and `result_large_err` errors. Both matrix commands timed out; no aggregate gate was run.
