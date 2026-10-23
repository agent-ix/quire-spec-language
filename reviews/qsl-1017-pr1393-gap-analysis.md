---
id: SR-16327
title: "Gap analysis of PR 1393: hybrid trace replay"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@867f449be3c57e4356bf90351c1783d133e8fe75; QSL-1017; FR-450, TC-1221, qsl-replay/src/hybrid_replay/{exact,interval,tests}.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-450
    type: reviews
---
# Gap analysis of PR 1393

## Summary

Manually compared FR-450 and TC-1221 with the PR's trace tags, replay code, and seven tests. The six `FR-450-AC-n` tags exist and the tests call the checked replay API, but the fairness and termination branches below have no failing test. FR-450-AC-1 through AC-5's verdict settlement clauses are assigned to QSL-900 and are outside this PR's replay slice.

## Verdict

**CONDITIONAL**: the replay slice has two missing behavioral tests. The code-review artifact SR-16326 records defects in the corresponding implementation paths.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-450 requires a lasso's periodic behaviour to satisfy the claim's fairness set, but the only fair-lasso assertion uses `fair Up;` while the Up transition is taken. No test makes a fair transition continuously enabled and untaken, especially alongside an undefined atom, so an unfair lasso can incorrectly return `Reproduced` without a test failing. | qsl-replay/src/hybrid_replay/tests.rs:494 |
| FND-002 | medium | FR-450 says static recursion always terminates; the test exercises only an undecided measure and a point measure in `Int[0, 9]`. No test exercises a deep, definite measure or a large future window over a short lasso period, the two resource paths in SR-16326. Add bounded-outcome assertions for both rather than wall-clock thresholds. | qsl-replay/src/hybrid_replay/tests.rs:402; qsl-replay/src/hybrid_replay/tests.rs:494 |

## Coverage

Plan completion: not assessed

The computed matrix was attempted with `quoin matrix --repo . --json` and `quire matrix --scope . --format tsv`; neither completed within the review's 2-minute and 50-second limits, respectively. The manual trace index found one `#[trace("TC-1221", "FR-450-AC-n")]` test for each AC-1 through AC-6, plus one TC-only exact-root test. This artifact makes no repository-wide matrix-completeness claim. Optional semantic review was not requested; the scope above is a focused code/test/criterion comparison for this PR.

## Dispositions

Disposition round 1 at PR head `6784de4aca345b573b8c03a778810f655dcf6e36`:

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The new unfair-lasso test defines a continuously enabled `Staying::Exiting` transition, takes only `Staying::Staying`, evaluates an undefined claim atom, and asserts `NotReproduced` at `Claim`. |
| FND-002 | fixed | The new deep-recursion tests assert resource refusal at low fuel and successful 20,000-edge evaluation with ample fuel; the new large-window test asserts deterministic resource refusal at 100,000 units. |
