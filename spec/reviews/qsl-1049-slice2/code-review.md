---
id: SR-17014
title: "Code review of QSL-1064 state-local V-8 refusals"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; QSL-1064 worktree diff from main 9b186cab: qsl-analyze/src/model_check/check.rs, qsl-analyze/tests/it/{weak_memory.rs,release_acquire.rs}, qsl-eval/src/simulation/{model.rs,protocol.rs,protocol/access.rs,protocol/selection.rs,protocol/steps.rs}, qsl-replay/src/{lib.rs,model_check/settle.rs,outcome.rs,proof_result.rs}"
review_set: subset
---

## Summary

The direct Rust review covered the listed diff, FR-220 and ADR-025 MA-1/MA-4, the result settlement path, and the affected weak-memory and release/acquire tests. No applicable AssuranceProfile was found; the scoped matrix reports FR-219-AC-4, FR-220-AC-5/14/15, and FR-229-AC-3 as tagged. The review uses the direct code-review path because the installed Quoin release has no `gap-analysis` command and the full-root matrix did not emit output; a temporary scope containing only the touched test files and their source requirements produced the scoped matrix.

## Verdict

**CONDITIONAL.** The reachable-state refusal plumbing and failure-array settlement are covered, but this slice still reads the EN-1 access set computed from the initial model state. The next ordered QSL-1065 implementation must replace that input with per-thread, state-varying enabled applications and protocol-transition ordering before QSL-1049 is complete.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `state_access_shape` reuses `ProtocolSystem.enabled`, which is computed once from the initial model state, and the current attempt failure key is `ModelTransition`. A later guard or attempt constraint can change which application is enabled, while node and attempt failures also need combined ordering by typed protocol-transition identity. The current slice can therefore report a failure at the wrong reachable state or return the wrong order for mixed failures. QSL-1065 is the next ordered child under QSL-1049 and owns dynamic per-thread EN-1 integration; this remains in QSL-1049 scope. | qsl-eval/src/simulation/protocol/steps.rs:242; qsl-eval/src/simulation/protocol/access.rs:212 |

## Coverage

The scoped computed matrix reported `tagged` for FR-219-AC-4, FR-220-AC-5, FR-220-AC-14, FR-220-AC-15, and FR-229-AC-3. Targeted validation passed: 34 weak-memory integration tests, 18 release/acquire integration tests, `cargo check -p qsl-eval -p qsl-replay -p qsl-analyze`, and `cargo clippy -p qsl-analyze -p qsl-eval -p qsl-replay --all-targets -- -D warnings`. `cargo test -p qsl-replay --lib` passed 847 tests. `cargo fmt --all -- --check` and `git diff --check` passed. Plan completion: not assessed.

## Dispositions

- FND-001: fixed in QSL-1065. Runtime expansion and transition-footprint calculation now derive the enabled applications and failures per reachable state and thread path; attempt failures use the thread observation and node/attempt failures enter the same typed-transition ordering.
