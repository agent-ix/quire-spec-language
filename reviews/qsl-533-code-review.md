---
id: SR-12149
title: "Code review of quire-spec-language PR #984: FR-132 missing fairness premise tests"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@53365a40b4bae9489330394feed9fc2da462e448; qsl-eval/tests/it/model_simulation.rs, qsl-eval/tests/it/temporal_trace.rs"
review_set: subset
---
# Code review of quire-spec-language PR #984

## Summary

Ticket: QSL-533. Code review with the rust-review lane, scoped to
`git diff origin/main...HEAD` (2 files, tests only). Static review only: no
cargo or make was run, per the dispatch brief.

Examined:
- `temporal_trace.rs`: `modelled_with`, `shape`, `mutex_outcome`,
  `a_fairness_constraint_over_a_supplied_lasso_names_the_first_constraint`
  and `an_empty_fairness_set_over_a_supplied_lasso_is_evaluated`.
- `model_simulation.rs`: the new `acquire()` and `release()` operations.
- Context only: `qsl-eval/src/temporal/engine.rs` `run` (the fairness
  short-circuit), `qsl-eval/src/temporal/outcome.rs` `TemporalOutcome` and
  `category`, `qsl-semantics/src/check/temporal_clause.rs` (fairness under a
  bounded profile refuses at S3), and `qsl-replay/src/proof_result.rs`
  `UnavailabilityCause` and `TerminalValue::category`.

The PR's claim that the evaluator behaviour already existed holds for
`TemporalOutcome`: `engine.rs` is not in the diff, and on `origin/main` `run`
returns `TemporalOutcome::MissingFairnessPremise` with
`clause.fairness().first()` before building a plan, for every unbounded
profile under `Goal::Clause`. The claim does not hold for FR-132's
`TerminalValue::Unsupported(UnavailabilityCause::MissingFairnessPremise)`
output, which does not exist. That gap is recorded in SR-12150 FND-001, not
repeated here.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The AC-1 test runs on an unlimited meter and does not record reads, so it would still pass if the engine evaluated the formula before settling `MissingFairnessPremise`. FR-132 says the evaluator "SHALL evaluate no formula". The TC-848 tests check this on the same engine branch with `limits(0)` and an empty read list; the AC-1 test could do the same with `run(..., limits(0))`. | qsl-eval/tests/it/temporal_trace.rs:983-989 |
| FND-002 | low | `assert_eq!(Some(constraint), fixture.clause.fairness().first())` restates the engine's own expression (`clause.fairness().first()`), so it cannot fail independently. The `shape` assertion before it is the real oracle. | qsl-eval/tests/it/temporal_trace.rs:1023 |

## Verdict

PASS for the test code. Neither finding is a defect in behaviour. The oracles
are strong:

- A missing `MissingFairnessPremise` fails the `let ... else` panic.
- A wrong constraint fails the `(kind, operation name, granularity)`
  comparison. Case 3 (`strong each acquire` then `weak whole release`) fails
  if the engine returned the last constraint, or one in any other order.
- Case 2 (`fair strong`) checks that an unmarked constraint defaults to
  `Whole`.
- The AC-2 case runs the same modelled unit, lasso and clause with no
  constraint, and fails if the clause settled as a missing premise or held.
  No observation has `c.level = 1`, so `Violated` at position 0 is the right
  answer.
- Rust lane: no production code changed. There is no new panic surface beyond
  test assertions, and no `unsafe` or integer conversion. The `shape` borrow
  lifetime is sound. The `#[trace]` style matches the file.
