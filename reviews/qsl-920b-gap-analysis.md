---
id: SR-12222
title: "Gap analysis of QSL-920 part 2 against FR-187-AC-1..4, FR-188-AC-1..6, FR-185-AC-5..7, the QSpec sampler definition and TC-354 (PR #1099)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@9bf8726508e8f533545be292b640bef73fb85d4c; spec/functional/FR-187-*.md (Behavior, AC-1..AC-4), spec/functional/FR-188-*.md (Behavior, AC-1..AC-6), spec/functional/FR-185-*.md (Package emission, AC-5..AC-7), spec/test-cases/TC-622-*.md, spec/test-cases/TC-623-*.md, spec/test-cases/TC-620-*.md (context); QSpec sampler definition and TC-354 SM-01..SM-09 (read by id); qsl-eval/src/simulation/model/probabilistic/tests.rs, qsl-eval/src/simulation/sampled/tests.rs, qsl-eval/src/simulation/choice.rs (tests), qsl-eval/tests/it/weighted_sampler.rs, qsl-semantics/tests/it/probabilistic_declarations.rs, qsl-package/src/emit/tests/probabilistic.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-187
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-188
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-185
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-622
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-623
    type: reviews
---
# SR-12222: Gap analysis of QSL-920 part 2 against FR-187-AC-1..4, FR-188-AC-1..6, FR-185-AC-5..7, the QSpec sampler definition and TC-354 (PR #1099)

## Summary

Ticket: QSL-920. Planless. Plan completion: not assessed. The traceability below comes from the trace tags in the diff. No `quoin matrix` run was made, because the brief says no gates.

How the tests bind to the criteria:

- **FR-187-AC-1: untagged, correctly.** The test is `service_steps_carry_the_products_of_their_draws` (`TC-622`). It asserts all six probabilities, their sum of 1, `duration = d`, and `request` at Idle. The fixture stands in `Int[0,2]`, `Int[1,12]` and Boolean for enum `Phase`, `1 ms` and enum `Outcome`, because intake admits no `Quantity` parameter (STD-113) and FR-120 encodes no enum.
- **FR-187-AC-2: `link_steps_weigh_operations_then_draws`.** The binding is correct. It asserts all four probabilities, two actions in canonical order with draws summing to 1, and the stutter step.
- **FR-187-AC-3: `health_variants_are_not_markov`.** The binding is correct. It asserts both variants: the state key, `tick`, the drawn value and the post-state counts from `weighted_steps` and from `actions`.
- **FR-187-AC-4: `a_negative_reward_refuses_admission` and `the_support_graph_is_the_state_graph`.** Both are correct. The second runs over the substituted `Service` (see FND-002).
- **FR-188-AC-1: SM-01, SM-02, SM-03 and TC-210's single uniform vector.** These are `tc_210_uniform_vector_reproduces_as_unit_weights_at_choice_zero` and `tc_354_weighted_vectors_reproduce`. The binding is correct. GeneratorMismatch is checked with a digest count of 0 (`another_generator_is_refused_before_any_draw`). SM-04 (trace-index assignment) and SM-06 (delays) belong to FR-189 and FR-420, not to this PR.
- **FR-188-AC-2: the 60,000-point reference and two-choice independence tests.** Correct (part 1, now driven through the slice API).
- **FR-188-AC-3: split.** The reproduction half is untagged, correctly, for the same substitution as AC-1. The `NotMarkov` half is `a_non_unique_post_state_stops_the_sample`, tagged correctly.
- **FR-188-AC-4: SM-05 (`one_item_choices_draw_nothing`), SM-07 (`a_random_parameter_takes_the_first_choice_index`) and one weight (`one_weight_selects_index_zero_without_a_digest`).** The binding is correct. The Coin test asserts the exact preimage bytes, `v mod 4 = 2`, `choices = [0]`, `random = [false]` and one digest.
- **FR-188-AC-5: the total-of-2^256 test, which is SM-08 at trace 0 among traces 0..49; `(2^256, 1)` with 0 digests (SM-09); and `Wide`.** Correct.
- **FR-188-AC-6: `the_sample_types_are_qsl_eval_simulation_items`.** Correct. Its second half holds by construction, because quire-semantic-value does not depend on qsl-eval.
- **FR-185-AC-5: `a_workload_name_is_package_wide`.** Correct for the refusal, the subject and `related` in source order (see FND-004).
- **FR-185-AC-6: `a_distribution_total_is_bounded_after_each_addition` and `a_check_cancelled_before_it_starts_ends_cancelled`.** The first is correct. The second is weak (FND-003).
- **FR-185-AC-7: `service_checks_and_refuses_emission_naming_its_first_random_parameter`.** Correct. The AC itself specifies `d` as `Millis = Int[1, 12]`, so this fixture is the AC's.

Behavior bullets of FR-187 and FR-188 that have no test: the stop on an undecided contract, and an enabled identity with no post-state for every draw. Both are CR defects (SR-12221 FND-001, FND-002), and FND-005 records the missing tests.

## Verdict

**Coverage is adequate for every criterion tagged.** Two criteria are untagged by design: FR-187-AC-1, and the reproduction half of FR-188-AC-3. They wait on STD-113 and on enum encoding, and they are the only ACs this PR does not fully cover. No tag claims more than its test asserts, except the FR-185-AC-6 cancellation half. The blocking issue for this PR is SR-12221 FND-001, which the missing test in FND-005 would have caught. This file has no finding that blocks.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-187-AC-1 and the reproduction half of FR-188-AC-3 are not verified as written. Their tests run a `Service` that stands in integers and Booleans for the enum `Phase`, the `Quantity<Time>` `d` and the enum `Outcome`. The tests are honestly untagged. The ACs stay open until intake admits quantity parameters (STD-113) and FR-120 encodes enum values. Record the dependency on QSL-920 so the ACs are not counted as done. | qsl-eval/src/simulation/model/probabilistic/tests.rs:85-137; qsl-eval/src/simulation/sampled/tests.rs:164-218; qsl-eval/src/simulation/sampled/world.rs:1-12 |
| FND-002 | low | The tagging is inconsistent across substituted fixtures. `the_support_graph_is_the_state_graph` is tagged `FR-187-AC-4` over the same substituted `Service` that kept AC-1 untagged. PM-6 does not depend on the value types, so the tag is defensible. Say so in the test doc, so the two decisions do not read as contradictory. | qsl-eval/src/simulation/model/probabilistic/tests.rs:290-341 |
| FND-003 | medium | The FR-185-AC-6 cancellation test cannot fail on what the AC states. `assert_eq!(cancel.cause(), Some(Requested))` reads back the handle the test itself tripped. `!refusals.is_empty()` passes for any refusal. The loop only rejects a 65,536 limit. Nothing asserts that the check *ended cancelled*. A check that ignored the handle and refused for any other reason would still pass. Assert that a refusal carries the cancelled cause (FR-276), or that it is the only refusal. | qsl-semantics/tests/it/probabilistic_declarations.rs:1268-1285 |
| FND-004 | low | FR-185-AC-5's clause "the first remains the package's `Steady`" is checked on a package with no duplicate (`service(None, "")`), not on one where the duplicate is refused. The duplicate case returns refusals and no graph, so the clause can only be observed through the refusal's `related[0]`. Either assert that `related[0]` is the first declaration, which the test already does through the start offsets, and drop the separate graph check, or reword the AC. | qsl-semantics/tests/it/probabilistic_declarations.rs:1180-1240 |
| FND-005 | medium | No test covers two FR-187 Behavior bullets. One is "An undecided contract conjunction during expansion SHALL stop with FR-120's `ContractUndetermined`". The other is an enabled scheduled identity with no post-state for every draw (`NotMarkov` with 0 post-states, not the stutter step). Both are defects in the code (SR-12221 FND-001, FND-002). Add a fixture whose postcondition is undecided for one draw, and one whose only enabled identity is unsatisfiable for every draw, and assert both through `weighted_steps` and `sample_probabilistic`. | qsl-eval/src/simulation/model/probabilistic/tests.rs; qsl-eval/src/simulation/sampled/tests.rs |
| FND-006 | low | The terminal stutter's "every reward 0" is asserted only on `Link`, which declares no reward (`stutter.rewards.is_empty()`). No test shows a stutter step carrying `(name, 0)` for a model that has rewards, so `zero_rewards` and `zero_of` are untested. Add a terminal state to a rewarded fixture. | qsl-eval/src/simulation/model/probabilistic/tests.rs:198-205; qsl-eval/src/simulation/sampled/tests.rs:234-252 |
