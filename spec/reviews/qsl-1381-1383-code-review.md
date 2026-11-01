---
id: SR-13627
title: "Code review — checked temporal successors and query proof"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-replay/src/spine.rs, qsl-replay/src/spine/query_proof.rs, qsl-replay/tests/it/main.rs, qsl-replay/tests/it/query_proof.rs, src/temporal.rs, src/temporal/activation.rs, src/temporal/formula.rs, src/temporal/mapping.rs, src/temporal/progress.rs, src/temporal/result.rs, src/temporal/trace.rs, tests/it/checked_temporal_successors.rs, tests/it/composed_temporal_limits.rs, tests/it/main.rs, tests/support/front_end.rs, tests/support/temporal/mod.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-043
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-044
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-045
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-046
    type: reviews
---

## Summary

Reviewed all 16 changed Rust source and test files in PR #1973, including the public checked-package temporal and query-proof APIs. The focused checked-temporal tests and Clippy pass, but a checked progress ledger can silently replace decision-scope closure, and the proof result omits the selected query node identity.

## Verdict

**FAIL** — one high-severity progress-contract defect and one medium-severity query-identity contract gap remain.

## Review coverage

- /code-review: duplication/vendoring, integrity, spec-code faithfulness, code-test alignment, and reverse-gap review.
- /rust-review: API shape, correctness, error handling, resource behavior, public boundaries, concurrency/safety, and test quality.
- All changed files were inspected; no AssuranceProfile applies to this Rust package scope.
- Focused checks: cargo test -p quire-spec-language --test it checked_temporal_successors -- --nocapture passed (20 tests); cargo clippy -p quire-spec-language -p qsl-replay --all-targets -- -D warnings passed; git diff --check origin/main...HEAD passed. The qsl-replay focused test command is still running at report time.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | [QSL-1381] record_checked accepts a changed decision_scope and overwrites the retained Progress after checking only watermark regression and complete-to-incomplete changes. A later open/closed assertion can therefore restamp the decision boundary under the same checked binding instead of returning a typed contradiction and preserving the earlier progress. | src/temporal/progress.rs:184 |
| FND-002 | medium | [QSL-1383] CheckedQueryProof retains only visit and result_type; it drops the checked node identity. When a clause contains multiple Map/Filter nodes with the same type, the typed Unsupported result cannot identify which checked representation was selected, despite the ticket contract requiring selected representation identity and type from the admitted package. | qsl-replay/src/spine/query_proof.rs:36 |
