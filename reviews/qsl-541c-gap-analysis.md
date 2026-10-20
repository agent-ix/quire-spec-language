---
id: SR-12228
title: "Gap analysis of quire-spec-language PR #1104: FR-153-AC-4 agreement and FR-161 shared budget"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@f8702a5d971aa6e797ee8d9649b0a1f9ddae295c; FR-153-AC-4, TC-571, TC-586, FR-161 \"Length and limits\" against qsl-analyze/tests/it/quotient.rs and qsl-analyze/src/model_check/{liveness.rs, concretise.rs, quotient.rs (unit tests)}"
review_set: subset
---
# Gap analysis of quire-spec-language PR #1104

## Summary

Ticket: QSL-541. A planless gap analysis: plan completion was not assessed.
It is scoped to the PR diff. Each new or changed `#[trace]` was read against
its AC and its TC on `origin/main`.

Examined:

- **FR-153-AC-4** — "Every AC-1 to AC-3 run with symmetry selected gives the
  same verdict as the unreduced run of the same claim; repeating each run
  gives equal generators and equal verdicts. With the run's meter set below
  the cost of AC-2's orbit closure, the run stops `Stopped(ResourceExhausted,
  …)` naming the meter." `every_reduced_verdict_agrees_with_the_unreduced_run`
  covers all four runs (AC-1 proof, AC-2 refutation, AC-3 weak and strong),
  reduced and unreduced, and repeats each reduced run. The meter test, now
  also tagged AC-4, covers the meter clause. Equal generators on repeat are
  covered for AC-1 only (see FND-001).
- **TC-571** — Step 1 (Toggle weak and strong, reduced and unreduced) and
  step 2 (AC-1 and AC-2 unreduced, every reduced request twice, meter stop)
  are all exercised.
- **TC-586 / FR-161 "Length and limits"** — "Building it SHALL count against
  the run's `max_transitions` and meter." The new budget test fails on the
  old per-binding budget (measured by reverting `src/` only). It stops on
  the concretisation budget, not on exploration.
- No stub or tautology was found in the diff. Both tests assert exact
  outcomes. `no_fair_cycle` matches one refusal that only the no-lasso,
  unreduced end of `decide` returns.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-153-AC-4 is tagged as covered, but its clause "repeating each run gives equal generators" is asserted only for AC-1. That check is `quotient.rs`'s unit test, which is tagged FR-153-AC-1, not AC-4. The new test repeats AC-2 and AC-3 and compares outcomes, but `ModelCheckOutcome` carries no generators, so a nondeterministic cycle group that still gives the same verdict would pass. Add the repeat-and-compare of `spanning(...).group` to the AC-2 and AC-3 refinement unit tests and tag them FR-153-AC-4, or drop the AC-4 tag until it is covered. | qsl-analyze/tests/it/quotient.rs:297-351, qsl-analyze/src/model_check/quotient.rs:1283-1340 |
| FND-002 | low | "The same verdict as the unreduced run" is read as `HoldsReduced` agreeing with the unreduced refusal `ComponentCertificateNotBuilt`. That refusal is returned after a full search finds no fair accepting cycle, and the unreduced engine cannot return `Holds` until FR-339's certificate exists. The reading is documented in the module doc, and it was already used for the weak case. But the AC text says "verdict", and a refusal is not a verdict. A spec note in FR-153-AC-4 naming this reading would remove the ambiguity. | qsl-analyze/tests/it/quotient.rs:14-20, qsl-analyze/tests/it/quotient.rs:286-295 |
| FND-003 | low | `every_binding_concretises_within_the_one_max_transitions_budget` is tagged `TC-586`, but TC-586's stated scope is FR-161-AC-3 to AC-4, and FR-161 "Length and limits" has no AC. The existing `concretising_the_loop_counts_against_max_transitions` does the same thing. The binding is to the right FR but not to a TC step. | qsl-analyze/tests/it/quotient.rs:411-422 |

## Verdict

Mergeable. No high findings. The agreement test is a real oracle: it would
fail on a wrong reduced verdict. The budget test fails on the old code.
FND-001 is a medium coverage gap in one sub-clause of FR-153-AC-4 (generator
determinism for AC-2 and AC-3). It does not block, but the AC-4 tag
overclaims until it is fixed. FND-002 and FND-003 are low.

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 44b0e3035: the AC-2 and AC-3 refinement unit tests rerun `analyze` and compare every component's states and cycle group, including refined sub-components, which sit in the flat `analyses` list. The AC-1, AC-2 and AC-3 unit tests are tagged FR-153-AC-4. |
| FND-002 | accepted-no-change | Low and a judgement call. The reading is documented in the test module, and the unreduced engine cannot return `Holds` until FR-339's certificate exists. A clarifying note belongs to a spec change, not to this code PR. |
| FND-003 | fixed | 44b0e3035: the `TC-586` tag is removed from the budget test, and its doc says FR-161 "Length and limits" has no criterion. |
