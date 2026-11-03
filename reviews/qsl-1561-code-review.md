---
id: SR-2026110216378
title: "QSL-1561 PR #2179 code review"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; PR #2179; src/population_fixtures.rs retarget encoder sidecars"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-416
    type: references
---
# SR-2026110216378: QSL-1561 PR #2179 code review

## Summary

Reviewed the one-file change that passes the checked package's sidecar rows to IR's v2 encoder while constructing the retargeted population fixture. The retargeted fixture's admission and assertions are covered by the existing population fixture tests. No defect was found in the PR diff.

## Verdict

**PASS** — no code findings in the reviewed diff. The focused root all-features tests, root-only Clippy, package formatting and deny checks pass. Separate workspace-wide failures are listed below and are outside the changed file/scope.

## Coverage

- Code-review with the Rust lane; reviewed `src/population_fixtures.rs` and its test callers, and checked the repository instructions and Cargo/lint conventions.
- No applicable AssuranceProfile was found for the fixture helper.
- `cargo test -p quire-spec-language --all-features population_fixtures::tests` via `locked-build.sh`: 10 passed, 0 failed.
- `cargo clippy -p quire-spec-language --all-targets --all-features --no-deps -- -D warnings` via `locked-build.sh`: passed.
- `cargo fmt -p quire-spec-language -- --check`: passed; `git diff --check`: passed.
- `cargo deny check`: exit 0; existing git-source warnings were reported, and advisories, bans, licenses and sources were OK.
- `cargo fmt --all --check` is red in unchanged `qsl-semantics/src/check/mod.rs:1515` due to rustfmt layout. The changed file is formatted.
- `cargo clippy -p quire-spec-language --all-targets --all-features -- -D warnings` is red in unchanged `qsl-cst/src/parser.rs:241` (`clippy::double_ended_iterator_last`). Root-only Clippy with `--no-deps` passes.
- `cargo check --workspace --all-targets --all-features` is red in unchanged `qsl-semantics/src/check/family.rs:3362` and `:3422`: non-exhaustive matches omit `StageFailure::LimitWithPayload` and `StageFailure::RefusedWithPayload`. This failure is separate from QSL-1561's root fixture fix.
- No workflow files changed. GitHub CLA checks pass; merge state is `BLOCKED` / `REVIEW_REQUIRED` despite `MERGEABLE`.
- The Quoin and Quire matrix commands each timed out after 90 seconds without JSON. The focused manual acceptance-to-test check is in the separate gap-analysis artifact; no matrix rollup is claimed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No code findings in the scoped PR diff. | - |

## Scope

- `src/population_fixtures.rs`: changed `retarget` encoder call; test callers in the same module.
- `FR-093-AC-19`: classify every checked-package/v2 node kind and admit one fixture for each QSL-written family.
- `TC-416`, step 11: emit and read each QSL-written family fixture through QSL's I2/IR reader.
