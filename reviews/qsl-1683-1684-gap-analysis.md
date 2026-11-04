---
id: SR-1683002
title: "Gap analysis of temporal evidence successor tests (quire-spec-language PR 2337)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language PR 2337 @75ab30437; QSL-1683, QSL-1684; FR-327-AC-6..13; FR-329-AC-4/8/9/10; FR-502-AC-9..11; FR-522-AC-9, 13..20, 22, 23; FR-523-AC-11; FR-524-AC-23"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-327
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-329
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-522
    type: reviews
---

## Summary

Tickets: QSL-1683, QSL-1684. Planless gap analysis from `quire matrix --scope .`
(quire 0.38.3) on origin/main and on the PR head. The PR turns 29 untagged
criteria to tagged (FR-327-AC-8..13, FR-502-AC-9..11, FR-522-AC-9, 13..20, 22,
23, FR-523-AC-11, FR-524-AC-23); no other matrix row changes status (the other
diffs are line-number shifts of tests in the edited files). Plan completion: not
assessed.

Untagged after the PR, with the coder's reason checked against the code:

- FR-327-AC-6 (refused condition): the coder's reason (no refused-atom in
  `AtomReading`/`TemporalOutcome`) is the wrong layer. The refused condition is
  carried by `qsl_eval::temporal::observed::ObservedRun.stopped:
  Some(HoldsStop::Refused(Evaluation))` and by the replay disposition
  `ClauseDisposition::Evaluate` (qsl-replay `spine/clause/temporal.rs:853`,
  `:1029`), so neither the spec vocabulary nor the evaluator is missing
  anything. The test is writable now through `evaluate_observed` (a `holds`
  atom whose S6a result is `Outcome::Refused(IntegerOutOfDomain)`; the technique
  of `window_aggregates.rs:1290-1312` and the `CheckedInvariant` stub of
  `composite_values.rs:559` apply) and through `run_clause` for the prefix and
  lasso halves (TC-1160 step 4).
- FR-327-AC-7 (seven aggregate sub-cases): `TemporalOutcome::InsufficientData`
  and window aggregates exist; what is missing is a test fixture that runs
  `count ... min_count` aggregate atoms through the observed or replay path
  over the Counter trace. Work, not a spec or code question.
- FR-329-AC-4/8/9/10: `Lasso::Model` is not in qsl-eval or qsl-replay; the
  model lasso and the FR-217 protocol lasso live in qsl-analyze
  (`tests/it/lasso_replay.rs`, which already carries FR-329-AC-3 with TC-839;
  `tests/it/protocol_system.rs` and `scheduler_fairness.rs`, TC-662). The stop
  searched the wrong crates. The second half of FR-329-AC-4 (`fair weak each`
  refuses naming the constraint) is already asserted untagged by
  `an_unfair_lasso_refuses_naming_the_constraint` in lasso_replay.rs.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-327-AC-6 has no test although the refused-condition result exists in the evaluator (`HoldsStop::Refused`, replay `ClauseDisposition::Evaluate`); the coder's reason that the evaluator lacks the vocabulary is wrong. Write the test through `evaluate_observed` and `run_clause` (position 1 refusal with record `integer_out_of_domain`/`outside-domain`, `expected`, locus; false atom at 0 wins; `CheckedInvariant` carried as a refused condition; prefix and lasso). | spec/functional/FR-327-evaluate-a-temporal-clause-over-a-finite-trace.md; qsl-eval/src/temporal/observed.rs:20-42; qsl-replay/src/spine/clause/temporal.rs:853 | correct-requirement-no-evidence |
| FND-002 | medium | FR-327-AC-7 has no test; the seven sub-cases need aggregate atoms (`count where ... over past[0,0] min_count 1`) driven through the observed or replay path; no evaluator feature is missing. The precedence among false activation, `UndefinedEvaluation` and `InsufficientData` is the part to assert. | spec/functional/FR-327-evaluate-a-temporal-clause-over-a-finite-trace.md; qsl-eval/tests/it/window_aggregates.rs | correct-requirement-no-evidence |
| FND-003 | medium | FR-329-AC-4/8/9/10 have no tagged test; they belong in qsl-analyze (model lasso replay and FR-217 protocol lasso replay), not qsl-eval/qsl-replay. AC-4's `fair weak each` half is asserted untagged by `an_unfair_lasso_refuses_naming_the_constraint`; AC-4's `fair weak attemptUpdate` half, AC-8 (bounded-profile protocol lasso at distance 0), AC-9 and AC-10 (adversarial scheduling package) need new tests. | spec/functional/FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md; qsl-analyze/tests/it/lasso_replay.rs:348; qsl-analyze/tests/it/protocol_system.rs | correct-requirement-no-evidence |
| FND-004 | low | FR-329-AC-9 states that a `Lasso::Observed` under a bounded profile refuses `unsupported_construct`/`expression-form`; the replay entry refuses a `TraceInput::Lasso` under a non-infinite profile as `invalid-value` (`spine/clause/temporal.rs:603`). Not verified whether these are the same case; resolve when AC-9 is written. | qsl-replay/src/spine/clause/temporal.rs:603; spec/functional/FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md | wrong-requirement |
| FND-005 | low | QSL-1684's text describes FR-524-AC-23 as "captures count stays one across restoration (row B22)"; the spec AC on main is the forward capture reference refused at S3 (FR-337-AC-8 owns it). The PR tags per the spec; the ticket text is stale. | spec/functional/FR-524-create-one-temporal-instance-per-trigger-or-origin-with-immutable-captures-and-name-every-capture-fault.md | wrong-requirement |
