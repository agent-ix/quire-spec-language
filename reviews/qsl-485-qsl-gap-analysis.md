---
id: SR-1337
title: "Gap analysis of quire-spec-language PR #641: FR-082, FR-083, FR-096, FR-099-AC-7, FR-111, NFR-012 against B4's tests"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@fc90c46a13308f63d21d28549dd9b30e9d745087; FR-062, FR-082-AC-3/AC-6, FR-083-AC-4, FR-096-AC-2, FR-099-AC-7, FR-111-AC-6, NFR-012; TC-220, TC-225, TC-427, TC-446, TC-491, TC-758; tests in qsl-replay/src/spine/dependency_tests.rs, qsl-semantics/src/check/checked_dispatch.rs, qsl-semantics/src/library/bundle_tests.rs, qsl-semantics/tests/it/{model_conformance,model_dispatch,model_population,type_environment_model}.rs, qsl-foundation/src/diagnostic/stage.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-082
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-083
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-099
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-111
    type: reviews
  - target: ix://agent-ix/quire-spec-language/NFR-012
    type: reviews
---
# Gap analysis of quire-spec-language PR #641

## Summary

Ticket: QSL-485 (B4). Manual check of acceptance criteria against tests (no plan
bundle for B4). Reviewed local head fc90c46a1.

Examined, with the test that backs each:

- FR-082-AC-3: a walk of `n` edges stops at `n - 1` and gives a verdict at `n`,
  and a 10,000-long chain completes on 512 KiB at the default.
  `divergence_chain_past_the_ceiling_refuses_at_check_and_at_evaluation` and
  `a_ten_thousand_long_chain_conforms_on_a_small_stack_at_the_default_limits`.
  Backed.
- FR-082-AC-6: check and evaluation share one edge count and one default.
  `check_and_evaluation_share_one_default_and_one_edge_count` (a diamond, 4
  edges) and the chain divergence rows. Backed, with FND-002.
- FR-083-AC-4: `family_steps` is an edge count over every walk together,
  stops at `n - 1`, links at `n`, and a 10,000-long chain links on 512 KiB.
  `family_steps_counts_the_edges_of_every_walk_together` (a wide family, the
  oracle that discriminates a shared count),
  `a_family_at_the_configured_bound_links_and_one_step_more_refuses` and
  `a_ten_thousand_long_redefines_chain_links_on_a_small_stack`. Backed.
- FR-096-AC-2: seven `LimitKind`s with their causes.
  `limit_exceeded_reports_stage_limit_exceeded_per_kind`. Backed.
- FR-099-AC-7: the libraries, import_edges and source_bytes refusals, each
  with kind, bound, actual and setting, the admit one above, and a long chain
  at the defaults. The three `the_*_limit_refuses_*` tests and
  `a_dependency_chain_of_any_length_compiles_on_a_small_stack` (1,000, above
  the AC's 200). Backed, with FND-003.
- FR-111-AC-6: dependency edges admit exactly and refuse one below, and no
  depth ceiling applies. `dependency_edge_limits_admit_exactly_and_refuse_one_below`
  and `a_dependency_chain_of_any_length_closes_within_the_edge_limit` (2,000
  links, above the old default depth of 256). Backed.
- NFR-012: `ancestor_steps` and `family_steps` default to 16777216 edges.
  The defaults tests in normalize.rs and population.rs. Backed.
- FR-082 Behavior, "each distinct edge once" in normalization's
  ancestor-path walk. Not backed; see FND-001.

## Verdict

Changes requested on test coverage. Every B4 AC has a passing-shaped test. One
behaviour, normalization's distinct-edge dedup, has no test that would fail if
it broke (FND-001). The diamond fixtures do not tell an edge count apart from a
node count (FND-002), and the setting string is never asserted (FND-003).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `normalize::ancestor_paths` charges each distinct `(specific, ancestor)` edge once, however many paths reach it, through the `followed` set. No test fails if that dedup is removed. The only multi-path fixture is the 4-type diamond in `qsl204_a_diamond_over_ancestor_steps_refuses_admission_naming_the_limit`. Its two paths `D-B-A` and `D-C-A` share no edge, so counting per path also gives 4. Fix: add an edge above the join (`A -> Z`). That gives 5 distinct edges against 6 counted per path; assert that `ancestor_steps = 5` admits and 4 refuses. | qsl-semantics/src/model/normalize.rs:959-990; qsl-semantics/tests/it/model_population.rs:3198-3260 |
| FND-002 | low | The diamond fixtures behind FR-082-AC-6 (`diamond_package` in type_environment_model.rs, and `fixture_diamond`) have 4 types and 4 edges, so the old node count gives the same admit and stop verdicts. They rule out a chain depth but not a node count. The chain divergence rows do discriminate (5 edges, 6 types), so the AC is still backed. Fix: add a direct `D -> A` edge to the diamond (5 edges, 4 types). | qsl-semantics/tests/it/type_environment_model.rs:404-472 |
| FND-003 | low | FR-099-AC-7 says the refusal names "setting `dependency.libraries`" and so on. The tests assert the `LimitsField` enum value but never the spelled string (`LimitsField::as_str`) a user sees in the message. No test runs a dependency limit through lifecycle `check` to show it returns `StageFailure::Limit` (`limit_of`'s new `Import { Limit }` arm). Fix: assert `limit.limits_field().map(LimitsField::as_str) == Some("dependency.libraries")`, and add a dependency limit row to `check_names_*_limit_field_it_reached`. | qsl-replay/src/spine/dependency_tests.rs:546-627; qsl-replay/src/spine/lifecycle.rs:304-307; qsl-foundation/src/diagnostic/stage.rs:139-141 |
