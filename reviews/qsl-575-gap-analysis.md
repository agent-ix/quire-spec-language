---
id: SR-202610310140
title: "Gap analysis — QSL-575 PR #2014 FR-198/FR-199 trace coverage"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; QSL-575; PR #2014 at 16ad281d627c55cc5daf075234e0cc7469787633; spec/functional/FR-198-decide-unbounded-reachability-and-expected-rewards.md; spec/functional/FR-199-decide-long-run-fractions-by-bottom-components.md; spec/test-cases/TC-633-unbounded-reachability-and-expected-rewards.md; spec/test-cases/TC-634-long-run-fractions-exact.md; qsl-analyze/src/exact_probabilistic.rs; qsl-analyze/src/exact_probabilistic/unbounded.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-198
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-199
    type: reviews
---

## Summary

Manually traced FR-198-AC-1..5 and FR-199-AC-1..3 to the new tagged tests and solver code at PR #2014's frozen head. Every AC has at least one trace tag, but the trace for FR-198-AC-5 overstates evidence coverage: the test checks a solver value and action index, not settlement or replay of the required refutation witness. Matrix verification could not complete because both installed matrix commands exceeded their runtime window without producing a result.

## Verdict

**CONDITIONAL** — all eight AC IDs are tagged, but the FR-198-AC-5 witness replay obligation has no implementation test in this PR. FR-198's timed §13a/AC-2 and evidence-construction budget behavior remain explicitly tracked downstream in QSL-577 and the FR-201/202 slice; this report does not count those as permanent refusals or deleted scope.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The FR-198-AC-5 tagged test asserts only the interval value and selected action index; it does not settle `probability <= 0.25` as refuted or replay the evidence as required by the AC. | spec/functional/FR-198-decide-unbounded-reachability-and-expected-rewards.md:157; qsl-analyze/src/exact_probabilistic/unbounded.rs:2403-2440 |
| FND-002 | medium | The FR-198-AC-4 test covers the solver's policy-iteration stop, but not exhaustion of derived-component or randomized-evidence construction budgets returning the actual stop. | spec/functional/FR-198-decide-unbounded-reachability-and-expected-rewards.md:156; qsl-analyze/src/exact_probabilistic/unbounded.rs:2310-2400 |

## Coverage

- FR-198-AC-1: `scheduler_minimum_and_maximum_apply_qualitative_sets` is tagged `TC-633` and AC-1; its assertions check minimum 0, maximum 1, and exact 24/25.
- FR-198-AC-2: the expected-reward tests are tagged AC-2; they exercise +infinity, exact 11/10 and 6/5, but use hand-built `ProbProduct` graphs rather than building the `Link` fixture. The timed quiescent-observation behavior in AC-2/§13a is absent from this untimed product and is retained in downstream QSL-577.
- FR-198-AC-3: the self-looping Markov-chain test checks 1/2, the outward interval test checks both bounds around 1/2, and the equality test exercises exact fallback.
- FR-198-AC-4: the repeatability and policy-limit branches are tested; the derived-component/randomized-evidence construction budget clauses are not in this PR's solver slice and need follow-on evidence.
- FR-198-AC-5: the new test checks the MEC value and chosen exit action, but not the criterion's `<= 0.25` refutation settlement or evidence replay (FND-001).
- FR-199-AC-1: the workload test checks the 1800/1801 value and gain-bias pair.
- FR-199-AC-2: the MDP and workload test checks 1000/1001, 1800/1801, and 1400/1401.
- FR-199-AC-3: the workload test checks the duration-weighted 4/7 exact value and typed zero-weight error.

The changed tests use the expected `#[trace("TC-633", "FR-198-AC-n")]` and `#[trace("TC-634", "FR-199-AC-n")]` forms. Changed production symbols are within the FR-198/FR-199 scope; no new unowned behavior or test/source stubs were found. No plan was supplied. Plan completion: not assessed.

### Matrix verification limitations

- `quoin matrix --repo /Users/peter/dev/qsl-575-unbounded --json` (quoin 0.29.1): ran about 90 seconds without stdout/stderr or matrix output; the specific session was interrupted, exit 130.
- `quire matrix --scope /Users/peter/dev/qsl-575-unbounded --format json` (quire 0.37.1): emitted only startup warnings about an inline schema, duplicate archetypes/modules, and duplicate inverse edges; ran about 60 seconds with no matrix output; the specific session was interrupted, exit 130.
- Both commands targeted the verified checkout (`git rev-parse HEAD=16ad281` during the run); this was not a missing-root or unresolved-HEAD failure. `quoin gap-analysis --help` is not an installed CLI command (`command gap-analysis not found`). The installed gap-analysis skill was followed manually for changed requirements, tagged tests, code and stubs. No matrix status is claimed.

## Reverse Gap and Stub Check

The new public solver outputs and behavior correspond to FR-198 and FR-199. The changed module contains no `todo!`, `unimplemented!`, or unfinished TODO markers. The manual trace check found the two acceptance evidence gaps above; it did not compute repository-wide untagged criteria because matrix verification was unavailable.

## Dispositions

Reviewed round 1 at `162496324391ae0bcb7416863db35ee66e6a8f20`.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | QSL-574 owns FR-201/202's public probability checker and witness replay; that work is gated by spec PR #1987. QSL-577 owns the digital timed product and the remaining FR-198 timed observation semantics. These are explicitly unimplemented dependencies; PR #2014 does not claim full FR-198 completion and adds no permanent unsupported/refusal result. |
| FND-002 | deferred | QSL-574 owns FR-201/202 evidence-construction and replay budget behavior, gated by spec PR #1987; QSL-577 owns the timed continuation/product work. The remaining AC-4 budget behavior stays open under those dependencies and is not claimed complete by PR #2014. |

The matrix limitation was re-recorded against the verified checkout: `quoin matrix --repo /Users/peter/dev/qsl-575-unbounded --json` ran about 90 seconds without output and exited 130 when interrupted; `quire matrix --scope /Users/peter/dev/qsl-575-unbounded --format json` emitted startup warnings, then ran about 60 seconds without matrix output and exited 130 when interrupted. Both runs targeted `git rev-parse HEAD=16ad281`; neither was a missing-root or unresolved-HEAD failure. No matrix status is claimed.
