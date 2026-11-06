---
id: SR-20261105101031
title: "Code review — QSL-1750 compound conversion frame"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-eval/src/value/expression/, qsl-eval/tests/it/compound_conversion_frame.rs, qsl-semantics/src/check/, qsl-replay/src/execute/composite_site.rs; initial review head 42e0353e0f54139c22a2cffca37f9b6d175a7bab; re-review through 41b19b02bba4210255bafcc7b3257237512d3b67"
review_set: subset
---

## Summary

Reviewed PR #2419 at `42e0353e0f54139c22a2cffca37f9b6d175a7bab`, including the retained-unit propagation, public evaluator entry, replay caller, and focused test against QSpec FR-142-AC-10 / TC-187 U25. The core frame API is exercised with bound values, but the reviewed test does not cover the required compound pivot target/two-step conversion; formatting also fails.

## Verdict

**CONDITIONAL** — the targeted API and charge assertions are sound for the single source-compound-to-named-target path, but AC1/U25 coverage is incomplete and formatting needs correction. Aggregate gates remain with the code lead.

## Scope

- QSpec FR-142-AC-10 and TC-187 U25: examined against the changed evaluator and test.
- `evaluate_closed_with_frame`, retained formed-unit propagation through checked graph/expression/function state, and replay caller: examined.
- Unknown compound and dimension-mismatch refusal assertions: examined.
- Other package behavior and full workspace gates: context only; not run by this reviewer.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The focused test converts a compound-valued source directly to named `J`; it does not exercise U25's first conversion from `N_m` to the compound pivot target followed by a second conversion to `J`. The U25 acceptance path therefore remains unverified by this PR at the reviewed head. | qsl-eval/tests/it/compound_conversion_frame.rs:119 |
| FND-002 | low | `cargo fmt --all -- --check` fails because the `quantity_literals` import is not in rustfmt order. | qsl-eval/tests/it/compound_conversion_frame.rs:16 |

## Review Evidence

- The coder reported the focused test passed; no independent test/build was run during this review.
- `cargo fmt --all -- --check` failed on the import ordering in FND-002.
- `cargo deny check` exited 0; it emitted existing `source-not-allowed` warnings for git dependencies.
- `quoin matrix --repo . --json` ran for about four minutes at high CPU without output and was interrupted. Matrix evidence is unavailable; the ticket/QSpec criterion was checked manually.
- The optional intent-to-test-to-code semantic review was not run.
- No CI workflow files changed.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | low | The second U25 leg asserts magnitude 1 but, at reviewed commit 8aa2a5857, does not assert that the result unit equals the checked `J` target; an incorrect result unit could pass. | qsl-eval/tests/it/compound_conversion_frame.rs:250-258 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Commit 8aa2a5857 evaluates the compound quantity literal, checks a compound-target conversion and a subsequent conversion to J, and asserts each leg's three conversion-only charge points. |
| FND-002 | fixed | Commit 8aa2a5857 orders the import; `cargo fmt --all -- --check` now passes. |
| FND-003 | still-open | The requested reviewed commit 8aa2a5857 lacks an assertion that the second result's unit equals its checked J target. A matching assertion exists only in an uncommitted worktree edit, so it is not counted as fixed on this head. |
| FND-003 | fixed | Commit 96695ebe9 adds the checked-target unit assertion for the second conversion result. |


## Follow-up head review

Reviewed commit `41b19b02bba4210255bafcc7b3257237512d3b67` against `96695ebe948247b663b36498a436d368c624b084`. The only delta is a private enum that makes the two existing checker-helper modes explicit (`Expected` versus `Conversion`) and updates their callers to pass the matching variant. The `match` arms preserve the prior behavior exactly: expected-type checking still validates then checks the expression; conversion still infers then converts and finishes the body. No reviewed behavior or disposition changes. No build or test was run.
