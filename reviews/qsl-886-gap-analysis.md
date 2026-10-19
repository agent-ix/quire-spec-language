---
id: SR-12182
title: "Gap analysis of QSL-886 FR-125 instancing (PR #1019)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@7ff160b37351dece5d249aa5852ccf80c7568e7d; FR-125-AC-4, AC-8, AC-10; FR-125 Instancing and Remaining work; TC-520; QSpec-FR-361-AC-8 bindings; qsl-analyze/tests/it/model_instances.rs, qsl-analyze/tests/it/model_check.rs, qsl-analyze/tests/it/closure_certificate.rs, and the production files they test"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-520
    type: reviews
---
# SR-12182: Gap analysis of QSL-886 FR-125 instancing (PR #1019)

## Summary

Ticket: QSL-886. PR quire-spec-language#1019 at head 7ff160b37. This is a
planless gap analysis scoped to the diff. Plan completion: not assessed. I
traced criteria to tagged tests by reading them. No tests were run (static
brief). `make check-spec-validation` passed on the reviewed head, with no
warning on FR-125 or TC-520.

I judged each trace binding in the diff on whether the test can fail on the
behaviour its AC states:

- FR-125-AC-8 on
  `instances_are_each_key_of_each_admitting_population_in_canonical_order`:
  correct. `P1` declares both `Lamp` and `Switch`, and the test asserts
  `P1/d` and `P1/c` exactly once for `over (x: Thing)`, so it fails if
  instances are emitted per member type. `Q` (`Other`) is excluded for
  `Thing` and `Lamp`, so it fails if admission ignores conformance. `P2`
  (`Thing`) is admitted for `Lamp`, so it fails if supertype admission is
  dropped.
- FR-125-AC-10 on the same test: correct. The universes are passed in the
  order `Q`, `P2`, `P1`, and the keys `[d, c]` and `[b, a]` are descending.
  Sorting keys ascending, or ordering populations by request order, fails
  the test. The assertion is made directly on `ModelSystem::instances`, as
  the AC names it.
- FR-125-AC-8 on
  `an_unbounded_admitting_population_requires_a_bound_and_no_key_is_no_instance`:
  correct. `P2` admits `Lamp`, has no universe and is held by no initial
  state, so the pre-check passes. The RequiresBound root `P2` can only come
  from instancing, and the test asserts that root exactly. A non-admitting
  population with no universe (`Q` for `Switch`) is not refused. Empty
  admitting universes give `NoInstance` with zero product states, and so
  does a type no population admits.
- FR-125-AC-4 on `a_clause_over_a_subject_with_no_initial_state_explores_nothing`:
  correct. It uses the Counter fixture instead of ADR-018 §6's
  `Config::ConfigVersion`, but the behaviour is the same: one instance per
  key of a two-key universe (`[b, a]` gives `b` then `a`, `[a]` gives `a`),
  and NoInitialState. The test fails if instancing drops or duplicates a key.
- QSpec-FR-361-AC-8 on
  `zero_instances_report_no_instance_after_the_requires_bound_pre_check` and
  `a_clause_with_no_instance_settles_inconclusive_never_proved_or_refuted`:
  correct. The zero instances now come from an empty universe, not from
  clearing a test-supplied list. Before this PR they did not exercise the
  production path.

Spec edits: FR-125 and TC-520 rename `ModelLetters::bindings` to
`ModelSystem::instances`. That matches the code. The "Code follow-up
(QSL-886)" bullet is replaced by the key-context bullet (FND-002).

Declared and not raised: AC-7, AC-9, AC-11 and AC-12 are untagged. Claims
over a subtype of a mixed population are refuted at non-conforming keys
until STD-196. The FR-338 typed key context is FR-338's call.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No test reaches the instancing refusals that the closure checker newly maps. `ItemRefusal::Unbounded` should become `ClosureRefusal::NotSimulated(RequiresBound)` for an admitting population with no universe, and `ItemRefusal::Fault` comes from `bound_reference`. FR-125's RequiresBound rule is tested through `check_model` only, so `check_closure` could map it wrongly and no test would fail. | qsl-replay/src/certificate/closure.rs:192-198 |
| FND-002 | low | The new FR-125 Remaining-work bullet on the FR-338 key context ends "(Remaining work)" although it already sits under Remaining work. It names no owner or ticket, against the repo rule `Remaining work: #N`. It also does not say that it departs from the Instancing SHALL ("That pair is the instance in the obligation identity ... neither carries an object type"), while the bound value is a reference that carries the `over` type and no population. A reader of the Instancing section cannot tell that the SHALL is not met. | spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md:246-249 |

## Verdict

Mergeable. No high finding. The three new tag sites (AC-8 twice, AC-10,
AC-4) can each fail on their AC's behaviour, so nothing is over-tagged.
The two QSpec-FR-361-AC-8 tests now exercise the real derivation. The
empty-universe-key misclassification is recorded in SR-12181 FND-001. It
has no test either.
