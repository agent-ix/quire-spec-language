---
id: SR-20261104180256
title: "QSL-1638/QSL-1650 PR 2313 gap analysis: remaining model-alias joins per unit"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; PR #2313 diff only (head 852525fff); FR-480 Selections and imports, FR-480-AC-12..18 (context), FR-230 The time member; qsl-semantics/tests/it/{timed_model.rs,model_intake.rs,hyper_clauses.rs,refinement_declarations.rs,dimension_equivalence.rs,probabilistic_declarations.rs,model_operations.rs,timed_intervals.rs}; qsl-analyze/tests/it/monitor_plan.rs; qsl-eval/tests/it/timed_trace.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-480
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: references
---

## Summary

Ticket: QSL-1638 (with QSL-1650). Plan completion: not assessed (planless).

The PR closes both findings of the earlier gap analysis:

- Its FND-001: `a_port_role_on_a_shared_alias_is_its_own_units_packages_port` assembles two units, each with a Port role on a shared alias, and asserts each role's package.
- Its FND-002: `a_clock_reading_parameter_names_its_own_units_package` now asserts which package unit b's `time Config` times. `each_units_time_source_times_its_own_units_package` covers two time sources, one per package.

There is one new test per fixed join: refinement, hyper clause, step relation, dimension equivalence and claim subject. `two_units_timing_one_package_under_one_alias_is_a_duplicate` keeps the common layout refusing.

Mutation checks on the trial merge:

- `ModelSpellings::model` made alias-only fails all six per-join tests.
- `ModelTimeSource::times` made alias-only fails the two time-source tests.

The tests are not tautological. FR-480-AC-14..18 are the unit-scoped type-name ACs, and AC-18 is the common layout. The PR does not change their behaviour. The new tests are tagged at the FR level (`#[trace("FR-480")]`) because no AC states the per-join rule.

## Verdict

**CONDITIONAL.** Every changed join that has a fixture is pinned by a test that fails on revert. Three gaps remain:

- Four fixed joins have no test.
- The newly admitted two-time-sources-per-alias layout is untested past S3.
- The behaviour change to the duplicate-time-source rule has no acceptance criterion.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Two `time M` declarations under one alias were refused before this PR. They are now accepted when two units bind `M` to two packages, but no test follows that layout past S3. The S4 emitter's alias-keyed joins (code-review SR-20261104180255 FND-001) cross-bind the two timed models with no failing test. Fix: add a qsl-package emit test for that layout, asserting each emitted timed model's clock places and urgencies belong to its own package. | qsl-semantics/tests/it/timed_model.rs:1269-1304; qsl-package/src/emit.rs:5532-5536 |
| FND-002 | medium | Four joins the PR fixes have no test in the one-alias-two-packages layout, as the PR body states: the protocol relationship export, the tick-anchor join, the probabilistic delay time source and the claim time source. Reverting any of them to alias-only passes every test. The tick anchor (`time M tick ... on M::T::op`) and the delay need only the `rpc_document` fixture that the time-source tests already use, so "no fixture" applies to the relationship export only. Fix: add the tick-anchor and delay/claim time-source tests now. Add the relationship-export test with a Port/relationship fixture, or ticket it. | qsl-semantics/src/check/mod.rs:5060-5063; qsl-semantics/src/check/probabilistic/model.rs:1101-1108; qsl-semantics/src/check/probabilistic/claim.rs:1112-1116; qsl-semantics/src/check/protocol_relationship.rs:121-124 |
| FND-003 | low | No acceptance criterion states the rule the new tests pin. Two units binding one alias to two packages may each write `time M`, and the duplicate rule is per package. The FR-230 "at most one `time` member per state model" bullet and FR-480 "Selections and imports" cover it only by implication. `each_units_time_source_times_its_own_units_package` therefore traces FR-480 at the FR level and backs no AC. Fix: add an FR-230 (or FR-480) AC for per-package duplicate time sources, including the two-aliases-one-package case (code-review FND-003), and tag the tests with it. | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md:145-147; qsl-semantics/tests/it/timed_model.rs:1269 |

## Dispositions

| ID | Disposition |
| --- | --- |
| FND-001 | fixed: covered by the code-review FND-001 test, on the checked graph (S4 cannot write the layout yet, see the code-review dispositions). |
| FND-002 | partly fixed: `each_units_tick_anchor_is_its_own_packages_operation` pins the tick-anchor join (fails when `ModelTimeSource::times` is alias-only). The delay time-source join is not testable here: delay operation resolution is alias-only (code-review FND-002, QSL-1663), and a delay test passed under the alias-only mutation, so it was not kept. The claim time-source join is not testable with the service fixture: `time Service` is refused `AmbiguousTimedModel` (several object types). Protocol relationship export stays untested (no Port fixture). Remaining joins tracked in QSL-1663. |
| FND-003 | accepted-no-change: tests trace FR-480/FR-230 at the FR level; adding an AC is spec work outside this PR. |
