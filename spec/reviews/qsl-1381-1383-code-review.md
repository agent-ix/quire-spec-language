---
id: SR-13627
title: "Code review — checked temporal successors and query proof"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; PR #1973 full origin/main...HEAD review at 57497065073dade8d1715a7b698f5a377d8c8b71, all changed Rust source/tests and FR-043/044/045/046 plus TC-1277..1280"
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

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Added DecisionScope contradiction refusal before checked progress is replaced. |
| FND-002 | fixed | CheckedQueryProof now retains the containing clause NodeKey and selected query NodeId. |

## Disposition pass 1

Fix commit 57497065073dade8d1715a7b698f5a377d8c8b71 resolves both code-review findings. Focused checked temporal and proof tests, clippy, and diff-check pass.

Re-reviewed the full current `origin/main...HEAD` diff, including the Rust fix delta and the new FR/TC traceability documents. The query proof boundary does not alter ordinary Map/Filter/Count/Sum clause evaluation; it returns an independent unsupported proof-transfer disposition.

### Fix evidence

- FND-001: `record_checked` now rejects a changed `decision_scope` with `Refusal::Progress { dimension: Dimension::DecisionScope }` before the retained entry is overwritten; the checked-route regression asserts refusal and then proves the prior open scope can continue.
- FND-002: `CheckedQueryProof` contains `clause: NodeKey` and `node: NodeId`; TC-1280 asserts the selected clause and the exact NodeId among two same-typed Map candidates.
