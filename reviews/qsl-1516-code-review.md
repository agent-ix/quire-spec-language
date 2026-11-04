---
id: SR-20261104025137
title: "QSL-1516 code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; QSL-1516; qsl-analyze/tests/it/lasso_replay.rs; FR-499 coverage table; FR-329-AC-3, FR-128-AC-3, TC-839"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-329
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-128
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-499
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-839
    type: references
---

## Summary

Reviewed the frozen diff for QSL-1516. The existing integration test runs a model lasso through replay and checks the loop-open cause, but the new FR-329-AC-3 path discards the returned catalog code and the doc comment still claims an unrelated post-state-key mutation.

## Verdict

NOT MERGEABLE until the test asserts the required code for the model-loop case and the inaccurate coverage comment is corrected.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The FR-329 model-loop assertion discards the returned code and cause tag, so it does not prove the required `invalid_runtime_input`/`invalid-value` result. | qsl-analyze/tests/it/lasso_replay.rs:328-329 |
| FND-002 | low | The test doc comment claims this function mutates a post-state key and checks `stale_dependency`/`content-mismatch`, but this function only changes lasso/prefix metadata and checks loop/stutter refusals. | qsl-analyze/tests/it/lasso_replay.rs:306-307 |

## Coverage

- FR-329-AC-3 (examined), spec/functional/FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md: "A lasso with an empty loop refuses `invalid_runtime_input`/`invalid-value`. A `Lasso::Model` whose last loop post-state differs from its entry state refuses the same way."
- TC-839 procedure step 3 and expected result (examined), spec/test-cases/TC-839-s6a-evaluates-an-infinite-trace-clause-over-a-lasso.md: the empty-loop and nonclosing model-loop cases each refuse `invalid_runtime_input`/`invalid-value`.
- FR-499 lasso-family row (examined), spec/functional/FR-499-delete-each-retired-family-only-behind-its-named-successor.md: the new row describes the FR-329-AC-3 ModelSystem case, while retaining the `PARTIAL` disposition.
- FR-128-AC-3 (context only), spec/functional/FR-128-replay-a-model-counterexample.md: includes the no-close and nonterminal-stutter refusal cases; the changed comment also claims post-state-key coverage.

Bindings checked: `a_lasso_that_does_not_close_or_does_not_stutter_at_a_terminal_state_refuses` is relevant to TC-523, TC-839 and FR-329-AC-3. Its lasso behavior assertions are correctly traced, but the FR-329 catalog code is missing from the model-loop assertion.

Rust review: test uses the real fixture, model-checker output and replay API; no test double or production code change was introduced. Cargo gates could not run: the clean-main workspace check fails in `qsl-foundation/src/diagnostic.rs` for newly added `Category::Pending` and `Category::Indeterminate` variants, tracked by QSL-1621 / PR #2248. `cargo fmt --all -- --check` and `git diff --check` passed. No applicable AssuranceProfile was found. Gap analysis was not applicable because the PR changes no production code.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | The fix removes the TC-523 trace tag even though this test still asserts TC-523's loop-truncation and nonterminal-stutter refusal cases, leaving those assertions without their test-case binding. | qsl-analyze/tests/it/lasso_replay.rs:308-339 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The model-loop branch now binds `cause`, `code` and `tag`, and asserts both `ModelTraceRefusal::LoopOpen { entry: 0, steps: 3 }` and `(Code::InvalidRuntimeInput, "invalid-value"). |
| FND-002 | fixed | The stale post-state-key claim is removed; the test doc now describes only the loop-open and terminal-stutter checks it performs. |
| FND-003 | still-open | Restore the TC-523 trace tag while keeping the inaccurate post-state-key claim removed; this function still checks TC-523's loop-truncation and nonterminal terminal-stutter cases. |

Disposition scope: QSL-1516 fix commit 127d51d518c5ccbbc6989be93352ce2e172eb2e1 inspected. Cargo gates remain unrun because clean main cannot compile under QSL-1621 / PR #2248. The fix introduces a new trace-coverage regression; the PR remains not mergeable until it is corrected.

Round 2 disposition at fix commit `bdb720ceb547d4dba4d558e4fb591a0922688b5e`: the `TC-523` binding is restored and the inaccurate stale-dependency claim remains removed. The model-loop code assertion from round 1 is unchanged. Cargo gates remain blocked by QSL-1621 / #2248.

| FND | outcome | reason |
| --- | --- | --- |
| FND-003 | fixed | The test again carries the TC-523 trace tag alongside TC-839 and FR-329-AC-3; the inaccurate post-state-key comment remains removed. |
