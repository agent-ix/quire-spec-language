---
id: SR-12931
title: "Code review (Rust lane) of quire-spec-language PR #1015: QSL-873 NoInstance outcome tests"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@f6c5a7467c31321d1c91065a5151c783e3e25d28; git diff origin/main...HEAD (3 files, +116/-1): qsl-analyze/tests/it/closure_certificate.rs, qsl-analyze/tests/it/model_check.rs, qsl-analyze/tests/it/model_settle.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-127
    type: reviews
---
# SR-12931: Code review (Rust lane) of quire-spec-language PR #1015: QSL-873 NoInstance outcome tests

## Summary

Ticket: QSL-873. PR quire-spec-language#1015 at head f6c5a7467, reviewed as `git diff origin/main...HEAD`. The rust-review lane is folded in here. The diff is test-only: two new tests and one trace tag added to an existing test. There is no production change: all three paths are under `qsl-analyze/tests/it/`. I could not run the tests: `cargo test -p qsl-analyze --test it` fails to resolve the locked git dependency `quire-contract-ir#fdef3c59` (fetch refused with publickey; `--offline` has no checkout), so the green/red state at this head was not checked here.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Mostly duplicates existing tests. Its run block is the same setup and the same `Undecided(NoInstance)`, `product_states == 0` assertion as `a_clause_with_no_instance_is_never_certified` at lines 636-661. The new model_check test repeats it a third time (model_check.rs:716-734). The `value`, `label` and `basis` checks repeat the NoInstance row of `each_outcome_settles_its_row_of_the_map` (model_settle.rs:559-565). `InconclusiveCause::NoInstance.as_str() == "no-instance"` checks a constant, not the record, and repeats qsl-replay/src/proof_result.rs:1113. The new assertions are `execution == Completed`, `counterexample.is_none()`, and settling a real run's outcome instead of a hand-built one. Better: add those assertions and the trace tag to the existing TC-525 test or the model_settle row, not a third copy. | qsl-analyze/tests/it/closure_certificate.rs:663-700 |
| FND-002 | low | The RequiresBound check matches `ModelCheckRefusal::RequiresBound(_)` with a wildcard. FR-125 (Instancing) says RequiresBound must name the population with no universe, and the test never checks that it does. The ordering check is real: check.rs:237-241 runs the bounds pre-check before the `instances.is_empty()` branch at 260, so swapping them would fail this test. But a `RequiresBound` naming the wrong root would still pass. Assert that the refusal's `domains` name the `c` population. | qsl-analyze/tests/it/model_check.rs:740-755 |
| FND-003 | low | The final block (no initial state with zero bindings gives `NoInitialState`) fixes an order FR-125 never states. FR-125 only says each of NoInitialState and NoInstance is reported 'before it explores any state'. It is the second half of FR-125-AC-4, not AC-7, and the doc comment says so, but the test carries only the AC-7 tag. NoInitialState before exploration is already covered by `an_empty_or_unbounded_subject_and_an_undecided_form_explore_nothing` (model_check.rs:648). Either drop the block, or get FR-125 to state that NoInitialState takes precedence over NoInstance and then tag it. | qsl-analyze/tests/it/model_check.rs:757-773 |

## Verdict

The tests can fail on what they claim. The NoInstance outcome is checked by exact `matches!` on the cause with zero product states. The settled record is checked by exact `TerminalValue::Inconclusive(NoInstance)`, `ResultLabel::Inconclusive` and `SettlementBasis::Unsettled`, so `proved` and `refuted` are excluded by equality, not by a negative check. RequiresBound before NoInstance is a real ordering check against check.rs:237-273. The `|_| panic!` replay closure correctly shows that no replay runs. No tautologies, no `unwrap` on production paths, no new helpers. All three findings are low: redundancy, one wildcard and one unspecified ordering. Trace accuracy is covered in the gap analysis (SR-12932).
