---
id: SR-20261104013117
title: "Code review — QSL-1505 bounded Rational model admission"
type: SpecReview
analysis: code-review
scope: "qsl-semantics/src/model/, qsl-semantics/src/check/, qsl-semantics/tests/it/model_intake.rs, FR-499"
review_set: subset
---

## Summary

Reviewed QSL-1505's exact Rational value-type admission, cross-node refusal paths, type projection and lowering, focused tests, and FR-499 dispositions. The implementation follows FR-505's bounded and unbounded forms and uses the shared exact Rational domain and carrier types.

## Verdict

**CONDITIONAL** — no implementation defects were found in the reviewed diff; the repository-wide traceability analysis could not complete with the installed tooling.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Gap analysis was unavailable: the installed `quoin` has no `gap-analysis` command, and `quire matrix --scope . --format json --strict` produced no result after approximately four minutes and was stopped. | quoin CLI; quire matrix |

## Coverage

- Reviewed all implementation, test, lockfile and FR-499 disposition changes in this PR.
- `cargo clippy --locked -p qsl-semantics --all-targets -- -D warnings` passed on the rebased head.
- `cargo clippy --locked -p qsl-semantics --all-targets --all-features -- -D warnings` passed on the rebased head.
- `cargo test --locked -p qsl-semantics --all-features --lib rational` passed: 10 tests.
- `cargo test --locked -p qsl-semantics --all-features --test it model_intake::` passed: 25 tests.
- `cargo deny check bans licenses sources` passed. The advisory check was unavailable because the configured advisory database is not present in the writable Cargo home; `--disable-fetch` reported the missing database.
- Optional intent↔test↔code semantic review was skipped.
