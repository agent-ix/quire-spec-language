---
id: SR-60020
title: "Gap analysis of PR #1592: release declarations at S1-S3 (QSL-1004, FR-249-AC-3, TC-704)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@8b97d4816473c92a229589e91342c12b547c15eb; PR #1592; spec/functional/FR-249-lower-a-task-automaton-to-a-timed-subject.md (Behavior, AC-1..AC-3); spec/functional/FR-253-check-delay-distributions-and-define-the-race.md; spec/test-cases/TC-704-*.md; spec/decisions/ADR-026-dense-time.md RT-8; tests tagged TC-704 and TC-708"
review_set: subset
date: '2026-10-25'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-249
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-253
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-704
    type: reviews
---
## Summary

Ticket: QSL-1004. Plan completion: not assessed.

Tag honesty: the seven tests in `qsl-semantics/tests/it/task_set_releases.rs` carry
`#[trace("TC-704", "FR-249-AC-3")]`; the two qsl-cst tests and the one qsl-forms
test carry `#[trace("TC-704")]` only. FR-249-AC-1 and AC-2 (lowering,
`StopwatchRequired`) are claimed by no test in the PR and no new production code
lowers anything: `release.rs` says so in its header. Correct.

Rows AC-3 asserts, and which test covers each: both tasks of one operation in source
order, another operation recording none (`a_release_is_recorded...`, `an_operation_that_releases_nothing...`);
duplicate at the second declaration (`a_second_release_of_one_task...`); the three
unresolved names (`an_unresolved_task_set_task_or_operation...`); no `release` records
none (`..._records_no_release`, last assertion); "`release` as a temporal operator is
unaffected" (only the TC-704-tagged qsl-cst test, not an AC-3 test). Extra tests
beyond AC-3: task of another set, every unresolved name reported, facet gate.

Edge cases:

- Operation named in another model: the operation resolves through the shared
  `ProbabilisticInput::operation` in every selected model, like `delay`; a task set is
  unit-level and has no model, so there is nothing to cross-check. Correct.
- Same task name in two task sets: covered (`Ctl.z` refused, `Other.z` admitted).
- Release on an untimed model: admitted silently. The fixtures declare no `time`
  source, so every test in the file is in fact on an untimed model. QSpec
  shared-grammar.md and FR-419 give `release-decl` no refusal, FR-249 defines
  the model as a task automaton only "for a timed model", and the only disposition it
  names is the lowering's `StopwatchRequired`. Recommendation: keep S3 admitting
  it (a refusal here would be a new code neither FR names), say so in one sentence
  in FR-249 Behavior, and make QSL-585's lowering refuse a release on a model
  that is not `time dense` with a typed cause (compare `delay-on-untimed-model`);
  pin the decision with one test that names the case. See FND-003.
- The second-`delay` refusal (FR-253): see FND-001.

## Findings

| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-001 | medium | FR-253 gains a SHALL ("If an operation has a second `delay` declaration, the checker SHALL refuse `invalid_model_binding`/`malformed-declaration` at the second declaration") with no acceptance criterion and no test. FR-253-AC-3 (the refusal row) still lists four refusals and not this one; no test in `timed_declarations.rs` declares a delay twice. The production branch exists (`DelayFault::RepeatedDeclaration`, model.rs:1100) and works: a throwaway test with two `delay ConfigVersion::attemptUpdate ~ uniform;` on a dense model refuses `InvalidModelBinding`/`malformed-declaration` at the second declaration (reverted). So the sentence is true but unverified: deleting the branch leaves every test green. Fix in this PR: add the vector to FR-253-AC-3 and TC-708, and one `#[trace("TC-708","FR-253-AC-3")]` test. | FR-253 Behavior; model.rs:1100 |
| FND-002 | medium | The ambiguous-operation refusal (`ambiguous_declaration`/`ambiguous-name`) is in FR-249 Behavior and release.rs but in no AC and no test (see code-review SR-60019 FND-003). Fix: add a vector to AC-3 and TC-704 step 3 (two models that expose the same object type, `release Ctl.a on Type::op;` with no alias, refuses `ambiguous_declaration`/`ambiguous-name` at the operation name) and a test; or delete the promise. | FR-249 Behavior; release.rs:105 |
| FND-003 | low | Release on a model with no `time dense` source is admitted and recorded, undocumented and untested as a decision. Recommendation above. | release.rs; FR-249 |
| FND-004 | low | AC-3 ends "`release` as a temporal operator is unaffected", but TC-704 step 3 does not mention it and no AC-3-tagged test asserts it (the qsl-cst test is TC-704-tagged, so the matrix counts it for the test case but not the criterion). Move the sentence out of AC-3 into TC-704 step 3 and keep the qsl-cst test, or tag the test with AC-3. | FR-249-AC-3; taskset.rs |

## Verdict

PASS with findings. FND-001 and FND-002 should be closed in this PR; FND-003 and
FND-004 are wording or a follow-up note on QSL-585.
