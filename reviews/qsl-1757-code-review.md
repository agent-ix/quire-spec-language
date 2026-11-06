---
id: SR-20261105101048
title: "Code review — QSL-1757 engine retry bound test"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-eval/tests/it/protocol_retry_bound.rs, qsl-eval/tests/it/main.rs, FR-520-AC-4, TC-1315 row 5"
review_set: subset
---

## Summary

Reviewed the QSL-1757 Rust integration test against FR-520-AC-4 and TC-1315 row 5. The test exercises compensation attempts through protocol simulation for bounds 1, 3, and 5, reads the emitted maximum through the protocol view, and its off-by-one mutation check fails as intended; no defects found.

## Verdict

**PASS** — the focused engine test exercises the bound at the public protocol simulation path, checks the authored count through the package reader, and fails when the engine admits attempt n+1.

## Review Scope

- `FR-520-AC-4`: examined; engine attempt bound and strict protocol-view count requirement.
- `TC-1315` row 5: examined; retry relation over attempts 1 and 2, protocol-view maximum, and expected Boolean/authored count.
- `qsl-eval/tests/it/protocol_retry_bound.rs`: examined; fixture, reachable graph traversal, read-back bound assertion, exact attempt-count assertion, and trace tags.
- `qsl-eval/tests/it/main.rs`: examined; integration module registration.
- No applicable `AssuranceProfile` is present under `spec/`; `spec/assurance/` contains measurement plans and an architecture description.

## Review Methods

- Ran code review with the Rust lane. The test reuses the established `tc1070` fixture through its existing test support path; it does not vendor or duplicate that fixture. The trace attribute binds both `FR-520-AC-4` and `TC-1315`.
- Compared the test with the exact AC and row text. The engine side is tested with a real `ProtocolSystem` reachable graph; the strict package reader supplies the maximum assertion. The graph maximum equals each bound, and the mutation from `count >= maximum` to `count > maximum` fails at bound 1 with 2 observed attempts.
- The diff contains tests only, so production gap analysis was not applicable. No spec, plan, dependency, or CI workflow files changed.
- Verification: focused test passed; combined workspace check and both Clippy configurations passed; `string-edge` passed; `cargo deny check` passed with existing git-source allowlist warnings. `rustfmt --check` passed for both changed Rust files. Workspace `cargo fmt --all -- --check` reports an unrelated existing extra blank line in `qsl-package/src/emit.rs` (not changed by this PR).

## Findings

| ID      | Severity | Summary                    | Refs |
| ------- | -------- | -------------------------- | ---- |
| FND-001 | low      | No findings (placeholder) | -    |
