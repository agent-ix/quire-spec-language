---
id: SR-1483002
title: "IR-1007 PR #1483 gap-analysis of root-location correction commit"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; IR-1007; PR #1483 head 6ee417f64 vs base e2e7883ca; correction commit 6ee417f64; spec/functional/FR-090-return-a-family-outcome-or-a-typed-family-refusal.md, spec/functional/FR-262-evaluate-values-and-calls-at-any-depth.md, spec/test-cases/TC-382-s6a-returns-kernel-outcomes-unchanged-in-evaluated.md, QSpec FR-146 / TC-191"
review_set: subset
---

## Summary

Ticket: IR-1007. Plan completion: not assessed. A narrow, manual, scoped trace check of the two tests changed by correction commit 6ee417f64. It checks their trace tags, what those criteria require, and how the new assertions relate to the spec text that governs `Evaluation.location`. No full-repository matrix was run. The scope is one commit.

## Verdict

**CONDITIONAL.** Both tests still back the criteria they are tagged with. `#[trace("QSpec-TC-191", "QSpec-FR-146-AC-5")]` still proves that an exhausted meter returns `Incomplete` at `FunctionCall`, and the function stays admitted. `#[trace("FR-090-AC-1", "TC-382")]` still checks all four outcomes, plus step 5's `FunctionCall` charge point. Neither AC nor TC-382 step 5 states a location for the denied call, so the new `Some(root)` expectation narrows nothing those criteria require. The expectation matches merged FR-262-AC-13 ("a work-limit denial at that charge has the same location", the checked body root `Location`).

One spec-consistency gap remains (FND-001). FR-090's own statement text still defines `e.location` as `None` when the evaluation "stopped before any node ran (a meter charge denied at call entry)". That is exactly the case these two tests now assert is `Some(root)`. FR-262-AC-13 and FR-090 now contradict each other. The code and tests follow FR-262-AC-13, and FR-090 is stale.

## Reviewed scope

- `QSpec-FR-146-AC-5` — examined: Exhausting execution fuel for an admitted function returns incomplete without changing the function's totality verdict.
- `FR-090-AC-1` — examined: ... For `x = 0.5` with a meter whose work limit is zero, the hook's `Ok(EvalOutcome::Kernel(Outcome::Incomplete(i)))` becomes `FamilyOutcome::Evaluated(Outcome::Incomplete(i))`, and `i` names `ChargePoint::FunctionCall`. None of the four is returned as `FamilyOutcome::FamilyEvaluated` or as `Err`.
- `TC-382` — examined: Step 5: `e.outcome` is `FamilyOutcome::Evaluated(Outcome::Incomplete(i))`, where `i`'s charge point is `ChargePoint::FunctionCall`.
- `FR-262-AC-13` — context_only: After successful Begin, cancelling the meter's installed handle before the first `function.call` charge returns a terminal incomplete evaluation whose `location` is exactly that graph's checked body root location ...; a work-limit denial at that charge has the same location.
- `FR-090` — examined: `location` is ... the node at which an evaluation that did not complete stopped, or `None` when the evaluation completed or stopped before any node ran (a meter charge denied at call entry).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-090 still specifies `location: None` for a meter charge denied at call entry, contradicting FR-262-AC-13 and the behaviour the PR's tests now assert | spec/functional/FR-090-return-a-family-outcome-or-a-typed-family-refusal.md:137-139, :211-215; qsl-eval/tests/it/checked_package_call.rs:358; qsl-eval/tests/it/total_functions.rs:1523 |

## Coverage

Bindings checked: `a_second_call_on_a_spent_meter_is_denied_at_its_function_call` -> QSpec-FR-146-AC-5 / QSpec-TC-191 (correct). `s6a_returns_kernel_outcomes_unchanged_in_evaluated` -> FR-090-AC-1 / TC-382 (correct). Neither binding changed in this commit. Focused tests were run on a detached worktree at 6ee417f64, with `RUST_TEST_THREADS=4` and `QSPEC_DIR` set to a read-only export of quire-specification origin/main 37a78bca. The run covered the 10 `qsl_replay` `application::tests` facade tests (`cargo test --locked -p qsl-replay --lib application::tests`: 10 passed, 0 failed) and the two modified qsl-eval tests (`cargo test --locked -p qsl-eval --test it -- a_second_call_on_a_spent_meter_is_denied_at_its_function_call s6a_returns_kernel_outcomes_unchanged_in_evaluated`: 2 passed, 0 failed). The aggregate gate and Kani were not run, by brief.

Suggested fix for FND-001: amend FR-090's two `location` definitions so that a `function.call` charge denied or cancelled after a successful Begin carries the checked body root `Location`, citing FR-262-AC-13. Keep `None` for completed evaluations only. This is a Markdown-only edit that can land in this PR.
