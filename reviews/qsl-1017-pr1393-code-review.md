---
id: SR-16326
title: "Code review of PR 1393: hybrid trace replay"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@867f449be3c57e4356bf90351c1783d133e8fe75; QSL-1017; PR #1393 against qsl-1017-pr3; qsl-replay/src/hybrid_replay.rs, qsl-replay/src/hybrid_replay/{eval,exact,expand,flow,interval,poly,real,signal,static_replay,tests,trace}.rs, qsl-replay/src/lib.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-450
    type: reviews
---
# Code review of PR 1393

## Summary

Reviewed the replay implementation and tests against FR-450, TC-1221, and the checked-model interfaces. No CI workflow changed. The review covers FR-450's trace admission, static replay, exact replay, fairness, decisive instant and all six acceptance criteria. The PR's own tests cover one path per AC, but three reachable paths below can return a wrong result or exhaust the process.

## Verdict

**FAIL**: an unfair lasso can be reported as a reproduced undefined evaluation; two unbounded or insufficiently bounded paths can exhaust the process.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | On a lasso, an undefined atom or model expression returns `Reproduced(UndefinedEvaluation)` before `fair(...)` runs. FR-450 admits a periodic behaviour only when its fairness set holds. A lasso that never takes a continuously enabled fair transition and also divides by zero is therefore incorrectly accepted as a counterexample. Check fairness before either reproduced outcome. | qsl-replay/src/hybrid_replay/exact.rs:1351 |
| FND-002 | high | `claim_segments` unrolls a lasso until the authored future-window horizon, cloning every segment each turn without a bound or refusal. A valid claim such as `eventually [1000000000, 1000000001] holds(...)` over a one-second loop attempts roughly one billion copies and can hang or exhaust memory before replay returns. Use periodic evaluation or a bounded typed outcome. | qsl-replay/src/hybrid_replay/exact.rs:735 |
| FND-003 | high | Static recursion counts up to 16,384 evaluations but uses the Rust call stack for every step. A checked recursive function with an `Int[0, 100000]` measure and a point argument of 10,000 can overflow the process stack long before the evaluation budget returns its result range, violating FR-450's termination guarantee. Bound stack depth or evaluate iteratively. | qsl-replay/src/hybrid_replay/interval.rs:453 |

## Coverage

Examined FR-450-AC-1 through AC-6 and TC-1221 against the tests in `qsl-replay/src/hybrid_replay/tests.rs`. In particular, AC-6's only fair lasso takes the named fair transition, so it does not exercise an unfair lasso. AC-5's recursive-call test evaluates a point argument after an undecided one, but the checked function's `Int` measure is only `[0, 9]`. Focused tests were not rerun because the lead reserved the shared build lane for final gates. This is a source review, not a gate result.
