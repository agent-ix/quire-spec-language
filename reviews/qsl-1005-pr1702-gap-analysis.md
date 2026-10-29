---
id: SR-1702002
title: "Gap analysis of quire-spec-language PR #1702: timed refinement liveness on zones (QSL-1005d)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@f7bb1931c; PR #1702 against FR-430-AC-5 and its liveness-half rule, FR-143, FR-241, ADR-026 TS-4/EZ-6, TC-1170"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-430
    type: reviews
---
# Gap analysis of quire-spec-language PR #1702

## Summary

Ticket: QSL-1005, slice 1005d. Plan completion: not assessed.

FR-430-AC-5 is fully covered:
- `a_positive_delay_loop_that_never_takes_an_enabled_abstract_class_refutes`
  checks an empty stem, one `poll` with delay 1, `Divergence{send}` and
  replay `Reproduced`.
- `a_zero_delay_loop_does_not_refute` checks `Checked(HoldsReduced{[]})`
  beside safety `Holds`.
- `a_refinement_lasso_with_no_loop_delay_is_refused_at_replay` checks the
  `LoopZeroDelay` refusal.
- The extra TD-6 case is
  `a_subject_with_only_a_zero_delay_loop_admits_no_behaviour`.

## Verdict

**AC-5 is met.** The new engine's other branches are untested.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Every test uses the one `poll` loop with an empty stem, so most of `live.rs` runs only on that path. Untested branches: an `AbstractUnfair` lasso (a loop that takes abstract steps but never the class); a non-empty stem and the shortest-stem roll-back; an idle-tail lasso (`terminal_stutter: true`); `F_C` fairness removing or keeping a violation (my probes P1 and P2 show both work and could become the tests); `Unsupported(HiddenFieldLiveness)` and `Unsupported(StrongAbstractFairness)` on the timed path; `Undecided(LassoNotConcretized)`; and a tick-source product. Add one test for each. | qsl-analyze/tests/it/timed_refinement_liveness.rs; qsl-analyze/src/zone_check/refine/live.rs:717-842, refine.rs unsupported_liveness |
